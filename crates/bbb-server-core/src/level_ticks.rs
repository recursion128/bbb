//! One level's scheduled-tick run state of one kind (block or fluid). The
//! pending ticks live in the per-chunk [`ChunkTicks`] containers the
//! [`ChunkStore`] registers when a column joins the level scheduler.
//!
//! Each game tick collects every due tick of the block-ticking columns, runs
//! them in `(trigger_tick, priority, sub_tick_order)` order, and leaves any
//! tick scheduled meanwhile in its container, so it runs no earlier than the
//! next game tick even when it is due now.

use std::collections::VecDeque;

use bbb_protocol::packets::BlockPos;
use rustc_hash::FxHashSet;

use bbb_server_types::bounding_box::BoundingBox;
use bbb_server_types::chunk_data::ChunkColumn;
use bbb_server_types::chunk_ticks::{
    unique_tick_key, BlockTickType, ChunkTicks, FluidTickType, ScheduledTick, TickType,
    UniqueTickKey,
};

use crate::chunk::ChunkStore;

/// The per-tick budget `ServerLevel` gives the block queue and the fluid queue
/// each (`net/minecraft/server/level/ServerLevel.java:375`, `:377`).
pub const SERVER_LEVEL_MAX_TICKS_PER_TICK: usize = 65_536;

/// A chunk key, `(chunkX, chunkZ)`.
pub type ChunkKey = (i32, i32);

pub const fn chunk_of_block(pos: BlockPos) -> ChunkKey {
    (pos.x >> 4, pos.z >> 4)
}

/// A registered chunk's two scheduler containers.
#[derive(Debug, Default)]
pub struct TickContainers {
    block: ChunkTicks<BlockTickType>,
    fluid: ChunkTicks<FluidTickType>,
}

impl TickContainers {
    /// Takes the containers a column parks and gives their saved ticks trigger ticks.
    pub(crate) fn register(column: &mut ChunkColumn, now: i64) -> Self {
        let mut ticks = Self {
            block: std::mem::take(column.block_ticks_mut()),
            fluid: std::mem::take(column.fluid_ticks_mut()),
        };
        ticks.block.unpack(now);
        ticks.fluid.unpack(now);
        ticks
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.block.is_empty() && self.fluid.is_empty()
    }
}

/// Selects the block or the fluid half of a chunk's tick state.
pub trait LevelTickType: TickType + 'static {
    fn registered(ticks: &TickContainers) -> &ChunkTicks<Self>;
    fn registered_mut(ticks: &mut TickContainers) -> &mut ChunkTicks<Self>;
    fn parked(column: &ChunkColumn) -> &ChunkTicks<Self>;
}

impl LevelTickType for BlockTickType {
    fn registered(ticks: &TickContainers) -> &ChunkTicks<Self> {
        &ticks.block
    }

    fn registered_mut(ticks: &mut TickContainers) -> &mut ChunkTicks<Self> {
        &mut ticks.block
    }

    fn parked(column: &ChunkColumn) -> &ChunkTicks<Self> {
        column.block_ticks()
    }
}

impl LevelTickType for FluidTickType {
    fn registered(ticks: &TickContainers) -> &ChunkTicks<Self> {
        &ticks.fluid
    }

    fn registered_mut(ticks: &mut TickContainers) -> &mut ChunkTicks<Self> {
        &mut ticks.fluid
    }

    fn parked(column: &ChunkColumn) -> &ChunkTicks<Self> {
        column.fluid_ticks()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleOutcome {
    Scheduled,
    /// A tick is already pending for the same `(pos, type)`; first scheduler wins.
    AlreadyScheduled,
    /// The target column has no registered container.
    ChunkNotLoaded,
}

#[derive(Debug)]
pub struct LevelTicks<T: LevelTickType> {
    handling_tick: bool,
    /// Collected this tick and not yet run.
    to_run: VecDeque<ScheduledTick<T>>,
    to_run_keys: FxHashSet<UniqueTickKey<T>>,
    /// Run this tick; `/clone` from a command block copies these too.
    already_run: Vec<ScheduledTick<T>>,
    due_scratch: Vec<(ChunkKey, ScheduledTick<T>)>,
}

impl<T: LevelTickType> LevelTicks<T> {
    pub fn new() -> Self {
        Self {
            handling_tick: false,
            to_run: VecDeque::new(),
            to_run_keys: FxHashSet::default(),
            already_run: Vec::new(),
            due_scratch: Vec::new(),
        }
    }

    /// Whether this `(pos, type)` was collected for this game tick and has not
    /// run yet; a running tick asking about itself gets `false`, which is how
    /// diodes and torches tell that they came due on their own.
    pub fn will_tick_this_tick(&self, pos: BlockPos, tick_type: T) -> bool {
        self.to_run_keys.contains(&unique_tick_key(pos, tick_type))
    }

    pub fn is_handling_tick(&self) -> bool {
        self.handling_tick
    }

    /// Collects up to `max_ticks` due ticks of the `ticking` columns; the rest
    /// stay in their containers for a later tick.
    pub fn begin_tick(
        &mut self,
        chunks: &mut ChunkStore,
        now: i64,
        max_ticks: usize,
        ticking: &[ChunkKey],
    ) {
        self.handling_tick = true;
        self.due_scratch.clear();
        let due_now = |container: &ChunkTicks<T>| {
            container
                .peek()
                .is_some_and(|head| head.trigger_tick <= now)
        };
        for &chunk in ticking {
            let Some(container) = chunks.tick_container_mut::<T>(chunk) else {
                continue;
            };
            while due_now(container) {
                self.due_scratch
                    .push((chunk, container.poll().expect("the head was due")));
            }
        }
        if self.due_scratch.is_empty() {
            return;
        }
        self.due_scratch.sort_unstable_by_key(|(_, tick)| {
            (
                tick.trigger_tick,
                tick.priority,
                tick.sub_tick_order,
                tick.pos.x,
                tick.pos.z,
                tick.pos.y,
            )
        });
        if self.due_scratch.len() > max_ticks {
            for (chunk, tick) in self.due_scratch.drain(max_ticks..) {
                if let Some(container) = chunks.tick_container_mut::<T>(chunk) {
                    container.schedule(tick);
                }
            }
        }
        self.to_run_keys
            .extend(self.due_scratch.iter().map(|(_, tick)| tick.unique_key()));
        self.to_run
            .extend(self.due_scratch.drain(..).map(|(_, tick)| tick));
    }

    pub fn next_collected_tick(&mut self) -> Option<ScheduledTick<T>> {
        let tick = self.to_run.pop_front()?;
        self.to_run_keys.remove(&tick.unique_key());
        self.already_run.push(tick);
        Some(tick)
    }

    pub fn finish_tick(&mut self) {
        self.handling_tick = false;
        self.to_run.clear();
        self.to_run_keys.clear();
        self.already_run.clear();
    }

    /// The ticks `/clone` copies out of `area`: the ones that already ran or
    /// are still collected this tick, then the pending ones.
    pub fn ticks_in_area(&self, chunks: &ChunkStore, area: BoundingBox) -> Vec<ScheduledTick<T>> {
        let inside = |tick: &&ScheduledTick<T>| area.is_inside(tick.pos);
        let mut ticks: Vec<ScheduledTick<T>> = self
            .already_run
            .iter()
            .chain(&self.to_run)
            .filter(inside)
            .copied()
            .collect();
        let (min, max) = (chunk_of_block(area.min()), chunk_of_block(area.max()));
        for x in min.0..=max.0 {
            for z in min.1..=max.1 {
                if let Some(container) = chunks.tick_container::<T>((x, z)) {
                    ticks.extend(container.iter().filter(inside).copied());
                }
            }
        }
        ticks
    }
}

/// Schedules `/clone`'s copies; they keep their relative order and sort after
/// everything the source held.
pub fn schedule_copied_ticks<T: LevelTickType>(
    chunks: &mut ChunkStore,
    ticks: Vec<ScheduledTick<T>>,
    offset: BlockPos,
) {
    let Some(min_sub_tick) = ticks.iter().map(|tick| tick.sub_tick_order).min() else {
        return;
    };
    let max_sub_tick = ticks
        .iter()
        .map(|tick| tick.sub_tick_order)
        .max()
        .expect("a non-empty batch has a maximum");
    for tick in ticks {
        chunks.schedule_tick(ScheduledTick {
            pos: BlockPos {
                x: tick.pos.x + offset.x,
                y: tick.pos.y + offset.y,
                z: tick.pos.z + offset.z,
            },
            sub_tick_order: tick.sub_tick_order - min_sub_tick + max_sub_tick + 1,
            ..tick
        });
    }
}

#[cfg(test)]
mod tests;
