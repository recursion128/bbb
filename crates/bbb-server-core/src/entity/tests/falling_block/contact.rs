use super::*;

fn body(
    store: &mut EntityStore,
    name: &str,
    time: i32,
    drop: bool,
    cancel: bool,
    position: Vec3d,
) -> i32 {
    store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        position,
        EntityData::FallingBlock(
            FallingBlockData::new(state_id(name), pos(0, 64, 0))
                .with_saved_fields(time, drop, cancel, false, 0.0, 40),
        ),
    )
}

#[test]
fn falling_contact_timeout_drops_without_invoking_anvil_broken_hook() {
    for drops in [false, true] {
        let mut level = NativeLevel::new();

        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::console_command(
            &sender,
            &receiver,
            &mut level.world,
            1,
            if drops {
                "gamerule minecraft:entity_drops true"
            } else {
                "gamerule minecraft:entity_drops false"
            },
        );
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = body(
            &mut store,
            "minecraft:anvil",
            600,
            true,
            false,
            Vec3d {
                x: 0.5,
                y: 100.0,
                z: 0.5,
            },
        );
        level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, id));
        assert!(store.get(id).is_none());
        assert_eq!(
            level.world.stored_items_for_test(DIMENSION).len(),
            usize::from(drops)
        );
        assert!(level
            .world
            .drain_level_events_for_test(DIMENSION)
            .is_empty());
    }
}

#[test]
fn falling_contact_cancel_drop_broken_hook_ignores_item_gates_and_honors_silent() {
    for name in [
        "minecraft:anvil",
        "minecraft:pointed_dripstone",
        "minecraft:suspicious_sand",
    ] {
        for silent in [false, true] {
            let mut level = NativeLevel::new();
            for x in -3..=3 {
                for z in -3..=3 {
                    level.world.set_block(
                        DIMENSION,
                        BlockPos { x, y: 0, z },
                        state_id("minecraft:stone"),
                        18,
                    );
                }
            }
            let (sender, receiver) = crate::tests::mailbox();
            crate::tests::console_command(
                &sender,
                &receiver,
                &mut level.world,
                1,
                "gamerule minecraft:entity_drops false",
            );
            let mut store = EntityStore::new(0, EntityIdCounter::new());
            let id = body(
                &mut store,
                name,
                0,
                false,
                true,
                Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5,
                },
            );
            store.get_mut(id).unwrap().set_silent(silent);
            level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, id));
            assert!(store.get(id).is_none());
            assert!(level.world.stored_items_for_test(DIMENSION).is_empty());
            let event = match name {
                "minecraft:anvil" => 1029,
                "minecraft:pointed_dripstone" => 1045,
                _ => 2001,
            };
            let events = level.world.drain_level_events_for_test(DIMENSION);
            assert_eq!(
                events
                    .iter()
                    .any(|record| record.event_type == event && record.pos == pos(0, 64, 0)),
                !silent || event == 2001
            );
            assert_eq!(
                level
                    .world
                    .positioned_game_events_for_test(DIMENSION)
                    .iter()
                    .any(|record| record.recorded.game_event
                        == crate::level_event::VanillaGameEvent::BLOCK_DESTROY
                        && record.recorded.source_entity_id == Some(id)
                        && record.recorded.pos == pos(0, 64, 0)),
                event == 2001
            );
        }
    }
}

#[test]
fn falling_contact_unchanged_set_block_failure_only_discards_when_both_drop_guards_hold() {
    for drops in [false, true] {
        for drop in [false, true] {
            let water = state_id("minecraft:water");
            let mut level = NativeLevel::new();
            for x in -3..=3 {
                for z in -3..=3 {
                    level.world.set_block(
                        DIMENSION,
                        BlockPos { x, y: 0, z },
                        state_id("minecraft:stone"),
                        18,
                    );
                }
            }
            level.world.set_block(DIMENSION, pos(0, 64, 0), water, 18);
            let (sender, receiver) = crate::tests::mailbox();
            crate::tests::console_command(
                &sender,
                &receiver,
                &mut level.world,
                1,
                if drops {
                    "gamerule minecraft:entity_drops true"
                } else {
                    "gamerule minecraft:entity_drops false"
                },
            );
            let mut store = EntityStore::new(0, EntityIdCounter::new());
            let id = body(
                &mut store,
                "minecraft:water",
                0,
                drop,
                false,
                Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5,
                },
            );
            level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, id));
            assert_eq!(store.get(id).is_none(), drop && drops);
            assert_eq!(
                level.world.block_state(DIMENSION, pos(0, 64, 0)).unwrap(),
                water
            );
            assert!(level
                .world
                .drain_level_events_for_test(DIMENSION)
                .is_empty());
            assert!(level.world.stored_items_for_test(DIMENSION).is_empty());
            if !(drop && drops) {
                assert!(store.get(id).unwrap().on_ground());
            }
        }
    }
}
