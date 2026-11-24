use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{
    VANILLA_BLOCK_LAVA_ID, VANILLA_BLOCK_REDSTONE_WIRE_ID, VANILLA_BLOCK_WATER_ID,
};
use bbb_protocol::registry_data_types::VanillaFluidIdentity;

use super::*;

fn wire() -> BlockTickType {
    BlockTickType::of_block_type(VANILLA_BLOCK_REDSTONE_WIRE_ID).unwrap()
}

fn water() -> BlockTickType {
    BlockTickType::of_block_type(VANILLA_BLOCK_WATER_ID).unwrap()
}

fn pos(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

fn tick(
    tick_type: BlockTickType,
    pos: BlockPos,
    trigger_tick: i64,
    priority: TickPriority,
    sub_tick_order: i64,
) -> ScheduledTick<BlockTickType> {
    ScheduledTick::with_priority(tick_type, pos, trigger_tick, priority, sub_tick_order)
}

fn drained(ticks: &mut ChunkTicks<BlockTickType>) -> Vec<(i64, TickPriority, i64)> {
    let mut out = Vec::new();
    while let Some(polled) = ticks.poll() {
        out.push((polled.trigger_tick, polled.priority, polled.sub_tick_order));
    }
    out
}

#[test]
fn tick_priority_orders_by_declaration_not_by_saved_value() {
    // TickPriority.java:6-12; DRAIN_ORDER compares with Enum.compareTo
    // (ScheduledTick.java:15), so declaration order is the ordering.
    assert!(TickPriority::ExtremelyHigh < TickPriority::VeryHigh);
    assert!(TickPriority::VeryHigh < TickPriority::High);
    assert!(TickPriority::High < TickPriority::Normal);
    assert!(TickPriority::Normal < TickPriority::Low);
    assert!(TickPriority::Low < TickPriority::VeryLow);
    assert!(TickPriority::VeryLow < TickPriority::ExtremelyLow);
    assert_eq!(TickPriority::default(), TickPriority::Normal);

    let values: Vec<i32> = TickPriority::ALL.iter().map(|p| p.value()).collect();
    assert_eq!(values, vec![-3, -2, -1, 0, 1, 2, 3]);
    for priority in TickPriority::ALL {
        assert_eq!(TickPriority::by_value(priority.value()), priority);
    }
    // TickPriority.java:28 clamps instead of failing.
    assert_eq!(TickPriority::by_value(-4), TickPriority::ExtremelyHigh);
    assert_eq!(
        TickPriority::by_value(i32::MIN),
        TickPriority::ExtremelyHigh
    );
    assert_eq!(TickPriority::by_value(4), TickPriority::ExtremelyLow);
    assert_eq!(TickPriority::by_value(i32::MAX), TickPriority::ExtremelyLow);
}

#[test]
fn drain_order_is_trigger_tick_then_priority_then_sub_tick_order() {
    let mut ticks = ChunkTicks::new();
    // Every tick sits at its own position so nothing is deduplicated, and the
    // insertion order is deliberately the reverse of the drain order.
    let scheduled = [
        // trigger tick decides first.
        tick(wire(), pos(0, 0, 9), 20, TickPriority::ExtremelyHigh, 0),
        // then priority, at an equal trigger tick.
        tick(wire(), pos(0, 0, 8), 10, TickPriority::ExtremelyLow, 0),
        tick(wire(), pos(0, 0, 7), 10, TickPriority::Normal, 7),
        tick(wire(), pos(0, 0, 6), 10, TickPriority::High, 99),
        // then sub-tick order, at an equal trigger tick and priority.
        tick(wire(), pos(0, 0, 5), 10, TickPriority::Normal, 5),
        tick(wire(), pos(0, 0, 4), 10, TickPriority::Normal, -1),
        tick(wire(), pos(0, 0, 3), 5, TickPriority::ExtremelyLow, 0),
    ];
    for entry in scheduled {
        assert!(ticks.schedule(entry));
    }
    assert_eq!(ticks.count(), 7);

    let expected = vec![
        (5, TickPriority::ExtremelyLow, 0),
        (10, TickPriority::High, 99),
        (10, TickPriority::Normal, -1),
        (10, TickPriority::Normal, 5),
        (10, TickPriority::Normal, 7),
        (10, TickPriority::ExtremelyLow, 0),
        (20, TickPriority::ExtremelyHigh, 0),
    ];
    // `drain_ordered` and repeated `poll` must agree.
    assert_eq!(
        ticks
            .drain_ordered()
            .into_iter()
            .map(|entry| (entry.trigger_tick, entry.priority, entry.sub_tick_order))
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(ticks.peek().unwrap().trigger_tick, 5);
    assert_eq!(drained(&mut ticks), expected);
    assert!(ticks.is_empty());
    assert_eq!(ticks.peek(), None);
    assert_eq!(ticks.poll(), None);
}

#[test]
fn one_pending_tick_per_position_and_type_with_the_first_winning() {
    let mut ticks = ChunkTicks::new();
    let at = pos(4, 70, -3);
    let first = tick(wire(), at, 100, TickPriority::Normal, 0);
    assert!(ticks.schedule(first));

    // A later schedule for the same (pos, type) is dropped whatever it asks
    // for -- sooner, higher priority, lower sub-tick order all lose
    // (LevelChunkTicks.java:51-55 gated on ScheduledTick.java:22-34).
    assert!(!ticks.schedule(tick(wire(), at, 1, TickPriority::ExtremelyHigh, -50)));
    assert!(!ticks.schedule(tick(wire(), at, 100, TickPriority::Normal, 0)));
    assert!(!ticks.schedule(tick(wire(), at, 999, TickPriority::ExtremelyLow, 9)));
    assert_eq!(ticks.count(), 1);
    assert_eq!(ticks.peek(), Some(&first));
    assert!(ticks.has_scheduled_tick(at, wire()));

    // Only the position and the type are compared, so the very same block at
    // a neighbouring position is a separate slot.
    assert!(!ticks.has_scheduled_tick(pos(4, 70, -2), wire()));
    assert!(ticks.schedule(tick(wire(), pos(4, 70, -2), 100, TickPriority::Normal, 1)));
    assert_eq!(ticks.count(), 2);
}

#[test]
fn different_types_at_one_position_do_not_deduplicate() {
    let mut block_ticks = ChunkTicks::new();
    let at = pos(0, 64, 0);
    assert!(block_ticks.schedule(tick(wire(), at, 10, TickPriority::Normal, 0)));
    assert!(block_ticks.schedule(tick(water(), at, 10, TickPriority::Normal, 1)));
    assert_eq!(block_ticks.count(), 2);
    assert!(block_ticks.has_scheduled_tick(at, wire()));
    assert!(block_ticks.has_scheduled_tick(at, water()));

    // The block and fluid containers are separate types entirely, so a fluid
    // tick never collides with a block tick.
    let mut fluid_ticks = ChunkTicks::new();
    let flowing = FluidTickType::of_fluid(VanillaFluidIdentity::FlowingWater);
    let source = FluidTickType::of_fluid(VanillaFluidIdentity::Water);
    assert!(fluid_ticks.schedule(ScheduledTick::new(flowing, at, 10, 0)));
    assert!(fluid_ticks.schedule(ScheduledTick::new(source, at, 10, 1)));
    assert!(!fluid_ticks.schedule(ScheduledTick::new(flowing, at, 3, 2)));
    assert_eq!(fluid_ticks.count(), 2);
}

#[test]
fn polling_frees_the_position_for_an_immediate_reschedule() {
    // This is what a self-scheduling block depends on: `poll` removes the
    // position from the dedup set (LevelChunkTicks.java:41-48) *before* the
    // tick body runs, so the block can arm its next tick from inside it.
    let mut ticks = ChunkTicks::new();
    let at = pos(1, 2, 3);
    assert!(ticks.schedule(tick(wire(), at, 10, TickPriority::Normal, 0)));
    assert!(!ticks.schedule(tick(wire(), at, 11, TickPriority::Normal, 1)));

    let polled = ticks.poll().unwrap();
    assert_eq!(polled.trigger_tick, 10);
    assert!(!ticks.has_scheduled_tick(at, wire()));
    assert!(ticks.schedule(tick(wire(), at, 12, TickPriority::Normal, 2)));
    assert_eq!(ticks.peek().unwrap().trigger_tick, 12);
    assert_eq!(ticks.count(), 1);
}

#[test]
fn pack_converts_trigger_ticks_into_delays_relative_to_now() {
    let mut ticks = ChunkTicks::new();
    assert!(ticks.schedule(tick(wire(), pos(0, 0, 0), 130, TickPriority::Normal, 0)));
    assert!(ticks.schedule(tick(wire(), pos(0, 0, 1), 100, TickPriority::VeryHigh, 1)));
    // A tick whose trigger already passed packs as a negative delay, exactly
    // as vanilla's `(int)(triggerTick - currentTick)` does
    // (ScheduledTick.java:48-50).
    assert!(ticks.schedule(tick(wire(), pos(0, 0, 2), 90, TickPriority::Normal, 2)));

    let packed = ticks.pack(100);
    assert_eq!(
        packed
            .iter()
            .map(|entry| (entry.pos.z, entry.delay, entry.priority))
            .collect::<Vec<_>>(),
        vec![
            (2, -10, TickPriority::Normal),
            (1, 0, TickPriority::VeryHigh),
            (0, 30, TickPriority::Normal),
        ]
    );

    // Round-tripping through the save schema keeps the delay, the position and
    // the priority, and resolves the type by registry name.
    let saved = ticks.pack_saved(100);
    assert_eq!(saved[0].id, "minecraft:redstone_wire");
    assert_eq!(saved[0].delay, -10);
    assert_eq!(saved[1].priority, -2);
    let reread: Vec<SavedTick<BlockTickType>> = saved
        .iter()
        .map(|entry| SavedTick::from_saved(entry).unwrap())
        .collect();
    assert_eq!(reread, packed);

    let unknown = Java26SavedTick {
        id: "minecraft:not_a_block".to_string(),
        x: 0,
        y: 0,
        z: 0,
        delay: 0,
        priority: 0,
    };
    let error = SavedTick::<BlockTickType>::from_saved(&unknown).unwrap_err();
    assert!(
        error.to_string().contains("minecraft:not_a_block"),
        "{error}"
    );
}

#[test]
fn unpack_restores_delays_as_trigger_ticks_and_negative_sub_tick_orders() {
    // Three saved ticks, deliberately not in delay order, so the restored
    // sub-tick order can be told apart from the restored trigger tick.
    let saved = vec![
        SavedTick {
            tick_type: wire(),
            pos: pos(0, 0, 0),
            delay: 5,
            priority: TickPriority::Normal,
        },
        SavedTick {
            tick_type: wire(),
            pos: pos(0, 0, 1),
            delay: 0,
            priority: TickPriority::Normal,
        },
        SavedTick {
            tick_type: wire(),
            pos: pos(0, 0, 2),
            delay: 5,
            priority: TickPriority::Normal,
        },
    ];
    let mut ticks = ChunkTicks::with_pending(saved.clone());

    // Before unpacking, the ticks count but never drain, and their positions
    // are already claimed (LevelChunkTicks.java:25-31).
    assert_eq!(ticks.count(), 3);
    assert_eq!(ticks.peek(), None);
    assert!(ticks.has_scheduled_tick(pos(0, 0, 1), wire()));
    assert!(!ticks.schedule(tick(wire(), pos(0, 0, 1), 1, TickPriority::Normal, 0)));
    // Packing a container that was never unpacked passes the saved ticks
    // through unchanged (LevelChunkTicks.java:91-94).
    assert_eq!(ticks.pack(1_000), saved);

    ticks.unpack(1_000);
    assert_eq!(ticks.count(), 3);
    let restored: Vec<(i32, i64, i64)> = ticks
        .drain_ordered()
        .into_iter()
        .map(|entry| (entry.pos.z, entry.trigger_tick, entry.sub_tick_order))
        .collect();
    assert_eq!(
        restored,
        vec![(1, 1_000, -2), (0, 1_005, -3), (2, 1_005, -1)]
    );

    // The restored sub-tick orders are the negative run -3..=-1
    // (LevelChunkTicks.java:104-109), so every restored tick sorts before a
    // tick scheduled during this run at the same trigger tick and priority.
    assert!(ticks.schedule(tick(wire(), pos(0, 0, 3), 1_005, TickPriority::Normal, 0)));
    assert_eq!(
        ticks
            .drain_ordered()
            .into_iter()
            .filter(|entry| entry.trigger_tick == 1_005)
            .map(|entry| entry.pos.z)
            .collect::<Vec<_>>(),
        vec![0, 2, 3]
    );

    // Unpacking twice is a no-op; the pending list is consumed.
    let before = ticks.clone();
    ticks.unpack(9_999);
    assert_eq!(ticks, before);
}

#[test]
fn generation_ticks_pack_with_zero_delay_in_insertion_order() {
    let mut ticks = ChunkTicks::new();
    let first = pos(0, 64, 0);
    let second = pos(0, 64, 1);
    assert!(ticks.schedule_for_generation(wire(), first, TickPriority::Low));
    assert!(ticks.schedule_for_generation(wire(), second, TickPriority::ExtremelyHigh));

    assert_eq!(ticks.count(), 2);
    assert!(!ticks.is_empty());
    assert!(ticks.has_scheduled_tick(first, wire()));
    assert_eq!(ticks.pack(-500), ticks.pack(9_000));
    assert_eq!(
        ticks
            .pack(9_000)
            .into_iter()
            .map(|entry| (entry.pos, entry.delay, entry.priority))
            .collect::<Vec<_>>(),
        vec![
            (first, 0, TickPriority::Low),
            (second, 0, TickPriority::ExtremelyHigh),
        ]
    );
}

#[test]
fn generation_ticks_deduplicate_by_type_and_position() {
    let mut ticks = ChunkTicks::new();
    let first = pos(3, 70, -4);
    assert!(ticks.schedule_for_generation(wire(), first, TickPriority::Low));
    assert!(!ticks.schedule_for_generation(wire(), first, TickPriority::ExtremelyHigh));
    assert!(ticks.schedule_for_generation(water(), first, TickPriority::Normal));
    assert!(ticks.schedule_for_generation(wire(), pos(3, 70, -3), TickPriority::Normal));

    let packed = ticks.pack(100);
    assert_eq!(packed.len(), 3);
    assert_eq!(packed[0].tick_type, wire());
    assert_eq!(packed[0].priority, TickPriority::Low);
    assert_eq!(packed[1].tick_type, water());
    assert_eq!(packed[2].pos, pos(3, 70, -3));
}

#[test]
fn generation_ticks_append_to_loaded_pending_ticks() {
    let loaded_pos = pos(1, 2, 3);
    let generated_pos = pos(4, 5, 6);
    let loaded = SavedTick {
        tick_type: wire(),
        pos: loaded_pos,
        delay: 12,
        priority: TickPriority::VeryHigh,
    };
    let mut ticks = ChunkTicks::with_pending(vec![loaded]);

    assert!(!ticks.schedule_for_generation(wire(), loaded_pos, TickPriority::Normal));
    assert!(ticks.schedule_for_generation(wire(), generated_pos, TickPriority::Low));
    assert_eq!(
        ticks.pack(70),
        vec![
            loaded,
            SavedTick {
                tick_type: wire(),
                pos: generated_pos,
                delay: 0,
                priority: TickPriority::Low,
            },
        ]
    );
}

#[test]
fn unpacked_generation_ticks_keep_order_and_share_runtime_deduplication() {
    let first = pos(0, 0, 1);
    let second = pos(0, 0, 2);
    let runtime = pos(0, 0, 3);
    let mut ticks = ChunkTicks::new();
    assert!(ticks.schedule_for_generation(wire(), first, TickPriority::Normal));
    assert!(ticks.schedule_for_generation(wire(), second, TickPriority::Normal));

    ticks.unpack(800);
    assert!(!ticks.schedule(tick(wire(), first, 800, TickPriority::Normal, 0)));
    assert!(ticks.schedule(tick(wire(), runtime, 800, TickPriority::Normal, 0)));
    assert_eq!(
        ticks
            .drain_ordered()
            .into_iter()
            .map(|entry| (entry.pos, entry.sub_tick_order))
            .collect::<Vec<_>>(),
        vec![(first, -2), (second, -1), (runtime, 0)]
    );
}

#[test]
#[should_panic(expected = "generation ticks cannot be scheduled after pending ticks were unpacked")]
fn generation_ticks_cannot_be_scheduled_after_unpack() {
    let mut ticks = ChunkTicks::new();
    ticks.unpack(0);
    let _ = ticks.schedule_for_generation(wire(), pos(0, 0, 0), TickPriority::Normal);
}

#[test]
fn saved_container_round_trips_through_the_save_schema() {
    let saved = vec![
        Java26SavedTick {
            id: "minecraft:lava".to_string(),
            x: -3,
            y: 71,
            z: 12,
            delay: 30,
            priority: 3,
        },
        Java26SavedTick {
            id: "minecraft:redstone_wire".to_string(),
            x: 0,
            y: 4,
            z: 0,
            delay: 1,
            priority: -3,
        },
    ];
    let mut ticks = ChunkTicks::<BlockTickType>::from_saved(&saved).unwrap();
    assert_eq!(ticks.count(), 2);
    assert_eq!(ticks.pack_saved(200), saved);

    ticks.unpack(200);
    let drained_order: Vec<(i32, i64, TickPriority)> = ticks
        .drain_ordered()
        .into_iter()
        .map(|entry| (entry.pos.y, entry.trigger_tick, entry.priority))
        .collect();
    assert_eq!(
        drained_order,
        vec![
            (4, 201, TickPriority::ExtremelyHigh),
            (71, 230, TickPriority::ExtremelyLow),
        ]
    );
    // Re-packing after the unpack reproduces the same rows, now emitted in
    // drain order rather than in the order the save happened to list them.
    assert_eq!(
        ticks.pack_saved(200),
        vec![saved[1].clone(), saved[0].clone()]
    );

    // `minecraft:lava` names both a block and a fluid, but
    // `minecraft:redstone_wire` names no fluid, so the block rows do not load
    // as fluid ticks.
    assert!(ChunkTicks::<FluidTickType>::from_saved(&saved).is_err());
    let fluid_saved = vec![Java26SavedTick {
        id: "minecraft:flowing_lava".to_string(),
        x: 1,
        y: 2,
        z: 3,
        delay: 10,
        priority: 0,
    }];
    let fluid_ticks = ChunkTicks::<FluidTickType>::from_saved(&fluid_saved).unwrap();
    assert_eq!(fluid_ticks.pack_saved(0), fluid_saved);
}

#[test]
fn block_tick_types_come_from_the_block_not_the_state() {
    // Every state of one block maps to one tick type; that is the whole point
    // of keying on the Block singleton (ScheduledTick.java:31).
    let lava_default = vanilla_block_default_state_id(VANILLA_BLOCK_LAVA_ID).unwrap();
    let lava_flowing = lava_default + 1;
    assert_eq!(
        BlockTickType::of_block_state(lava_default),
        BlockTickType::of_block_state(lava_flowing)
    );
    assert_eq!(
        BlockTickType::of_block_state(lava_default)
            .unwrap()
            .block_type_id(),
        VANILLA_BLOCK_LAVA_ID
    );
    assert_eq!(
        BlockTickType::of_block_state(lava_default)
            .unwrap()
            .resource_id(),
        "minecraft:lava"
    );
    assert_eq!(
        BlockTickType::from_resource_id("minecraft:lava"),
        BlockTickType::of_block_type(VANILLA_BLOCK_LAVA_ID)
    );
    assert_eq!(BlockTickType::of_block_state(-1), None);
    assert_eq!(BlockTickType::of_block_type(i32::MAX), None);
    assert_eq!(BlockTickType::from_resource_id("minecraft:nope"), None);

    let fluid = FluidTickType::of_fluid(VanillaFluidIdentity::FlowingLava);
    assert_eq!(fluid.identity(), VanillaFluidIdentity::FlowingLava);
    assert_eq!(fluid.resource_id(), "minecraft:flowing_lava");
    assert_eq!(
        FluidTickType::from_resource_id("minecraft:flowing_lava"),
        Some(fluid)
    );
}
