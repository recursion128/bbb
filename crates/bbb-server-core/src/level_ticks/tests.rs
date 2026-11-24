use bbb_protocol::block_types::VANILLA_BLOCK_REDSTONE_WIRE_ID;

use bbb_server_types::chunk_ticks::{SavedTick, TickPriority};

use super::*;

fn wire() -> BlockTickType {
    BlockTickType::of_block_type(VANILLA_BLOCK_REDSTONE_WIRE_ID).unwrap()
}

fn pos(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

fn normal(at: BlockPos, trigger_tick: i64, sub_tick_order: i64) -> ScheduledTick<BlockTickType> {
    ScheduledTick::new(wire(), at, trigger_tick, sub_tick_order)
}

struct Level {
    chunks: ChunkStore,
    ticks: LevelTicks<BlockTickType>,
    ticking: Vec<ChunkKey>,
}

impl Level {
    fn with_chunks(ticking: &[ChunkKey]) -> Self {
        let mut chunks = ChunkStore::new(
            crate::world_creation::default_overworld_level_source().unwrap(),
            0,
        );
        for &chunk in ticking {
            chunks.register_ticks(chunk, 0);
        }
        Self {
            chunks,
            ticks: LevelTicks::new(),
            ticking: ticking.to_vec(),
        }
    }

    fn schedule(&mut self, tick: ScheduledTick<BlockTickType>) -> ScheduleOutcome {
        self.chunks.schedule_tick(tick)
    }

    fn begin(&mut self, now: i64, max_ticks: usize) {
        self.ticks
            .begin_tick(&mut self.chunks, now, max_ticks, &self.ticking);
    }

    fn run(&mut self, now: i64, max_ticks: usize) -> Vec<i64> {
        self.begin(now, max_ticks);
        let mut ran = Vec::new();
        while let Some(tick) = self.ticks.next_collected_tick() {
            ran.push(tick.sub_tick_order);
        }
        self.ticks.finish_tick();
        ran
    }
}

#[test]
fn due_ticks_run_by_trigger_tick_then_priority_then_schedule_order() {
    let mut level = Level::with_chunks(&[(0, 0), (1, 0)]);
    level.schedule(normal(pos(0, 64, 0), 10, 3));
    level.schedule(ScheduledTick::with_priority(
        wire(),
        pos(16, 64, 0),
        10,
        TickPriority::VeryHigh,
        4,
    ));
    level.schedule(normal(pos(1, 64, 0), 9, 5));
    level.schedule(normal(pos(17, 64, 0), 10, 1));
    level.schedule(normal(pos(2, 64, 0), 11, 0));
    assert_eq!(level.run(10, SERVER_LEVEL_MAX_TICKS_PER_TICK), [5, 4, 1, 3]);
    assert_eq!(level.run(11, SERVER_LEVEL_MAX_TICKS_PER_TICK), [0]);
}

#[test]
fn the_budget_leaves_the_rest_for_the_next_tick() {
    let mut level = Level::with_chunks(&[(0, 0)]);
    for index in 0..5 {
        level.schedule(normal(pos(index, 64, 0), 10, i64::from(index)));
    }
    assert_eq!(level.run(10, 2), [0, 1]);
    assert_eq!(level.run(11, SERVER_LEVEL_MAX_TICKS_PER_TICK), [2, 3, 4]);
}

#[test]
fn a_tick_scheduled_while_ticks_run_waits_for_the_next_tick() {
    let mut level = Level::with_chunks(&[(0, 0)]);
    level.schedule(normal(pos(0, 64, 0), 10, 0));
    level.begin(10, SERVER_LEVEL_MAX_TICKS_PER_TICK);
    let tick = level.ticks.next_collected_tick().unwrap();
    // The running tick released its slot, so it can re-arm itself.
    assert_eq!(
        level.schedule(normal(tick.pos, 10, 1)),
        ScheduleOutcome::Scheduled
    );
    assert_eq!(level.ticks.next_collected_tick(), None);
    level.ticks.finish_tick();
    assert_eq!(level.run(10, SERVER_LEVEL_MAX_TICKS_PER_TICK), [1]);
}

#[test]
fn one_position_and_type_holds_one_pending_tick() {
    let mut level = Level::with_chunks(&[(0, 0)]);
    let at = pos(0, 64, 0);
    assert_eq!(
        level.schedule(normal(at, 100, 0)),
        ScheduleOutcome::Scheduled
    );
    assert_eq!(
        level.schedule(normal(at, 1, 1)),
        ScheduleOutcome::AlreadyScheduled
    );
    assert_eq!(
        level.schedule(normal(pos(16 * 9, 64, 0), 1, 2)),
        ScheduleOutcome::ChunkNotLoaded
    );
    assert!(level.run(99, SERVER_LEVEL_MAX_TICKS_PER_TICK).is_empty());
    assert_eq!(level.run(100, SERVER_LEVEL_MAX_TICKS_PER_TICK), [0]);
}

#[test]
fn will_tick_this_tick_covers_collected_ticks_that_have_not_run() {
    let mut level = Level::with_chunks(&[(0, 0)]);
    let (first, second, later) = (pos(0, 64, 0), pos(1, 64, 0), pos(2, 64, 0));
    level.schedule(normal(first, 10, 0));
    level.schedule(normal(second, 10, 1));
    level.schedule(normal(later, 20, 2));

    level.begin(10, SERVER_LEVEL_MAX_TICKS_PER_TICK);
    assert_eq!(level.ticks.next_collected_tick().unwrap().pos, first);
    assert!(!level.ticks.will_tick_this_tick(first, wire()));
    assert!(level.ticks.will_tick_this_tick(second, wire()));
    assert!(!level.chunks.has_scheduled_tick(second, wire()));
    assert!(level.chunks.has_scheduled_tick(later, wire()));
    level.ticks.finish_tick();
    assert!(!level.ticks.will_tick_this_tick(second, wire()));
}

#[test]
fn a_column_that_is_not_block_ticking_keeps_its_ticks() {
    let mut level = Level::with_chunks(&[(0, 0), (1, 0)]);
    level.schedule(normal(pos(0, 64, 0), 10, 0));
    level.schedule(normal(pos(16, 64, 0), 10, 1));
    level.ticking = vec![(0, 0)];
    level.begin(10, SERVER_LEVEL_MAX_TICKS_PER_TICK);
    assert_eq!(level.ticks.next_collected_tick().unwrap().sub_tick_order, 0);
    assert_eq!(level.ticks.next_collected_tick(), None);
    level.ticks.finish_tick();
    level.ticking = vec![(0, 0), (1, 0)];
    assert_eq!(level.run(11, SERVER_LEVEL_MAX_TICKS_PER_TICK), [1]);
}

#[test]
fn saved_ticks_are_due_relative_to_registration() {
    let saved = [5, 0].map(|delay| SavedTick {
        tick_type: wire(),
        pos: pos(delay, 64, 0),
        delay,
        priority: TickPriority::Normal,
    });
    let mut level = Level::with_chunks(&[]);
    *level.chunks.load_column_mut(0, 0).block_ticks_mut() =
        ChunkTicks::with_pending(saved.to_vec());
    level.chunks.register_ticks((0, 0), 1_000);
    level.ticking = vec![(0, 0)];
    assert_eq!(level.run(1_000, SERVER_LEVEL_MAX_TICKS_PER_TICK), [-1]);
    assert_eq!(level.run(1_005, SERVER_LEVEL_MAX_TICKS_PER_TICK), [-2]);
}

#[test]
fn copied_ticks_are_offset_after_the_source_order() {
    let mut level = Level::with_chunks(&[(0, 0), (6, 0)]);
    level.schedule(normal(pos(1, 5, 1), 10, 4));
    level.schedule(normal(pos(2, 5, 1), 20, 7));
    level.schedule(normal(pos(9, 5, 1), 20, 8));

    let ticks = level.ticks.ticks_in_area(
        &level.chunks,
        BoundingBox::from_corners(pos(0, 0, 0), pos(4, 9, 4)),
    );
    schedule_copied_ticks(&mut level.chunks, ticks, pos(100, 0, 0));
    let mut copied: Vec<_> = level
        .chunks
        .tick_container::<BlockTickType>((6, 0))
        .unwrap()
        .iter()
        .map(|tick| (tick.pos, tick.trigger_tick, tick.sub_tick_order))
        .collect();
    copied.sort_by_key(|entry| entry.2);
    assert_eq!(copied, [(pos(101, 5, 1), 10, 8), (pos(102, 5, 1), 20, 11)]);
}
