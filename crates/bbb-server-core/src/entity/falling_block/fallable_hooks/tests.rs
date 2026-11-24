use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use crate::fluid::with_property;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID;
use bbb_protocol::packets::Vec3d;
use uuid::Uuid;

use crate::entity::falling_block::FallingBlockData;

fn state(name: &str) -> i32 {
    vanilla_block_default_state_id(vanilla_block_type_for_resource_id(name).unwrap().id).unwrap()
}

const LANDING: BlockPos = BlockPos { x: 8, y: 70, z: 8 };

fn body(current_state: i32) -> Entity {
    Entity::new(
        37,
        Uuid::from_u128(37),
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 8.25,
            y: 70.7,
            z: 8.75,
        },
        EntityData::FallingBlock(FallingBlockData::new(current_state, LANDING)),
    )
}

#[test]
fn falling_fallable_hook_anvil_sounds_and_silent_gate() {
    for name in ["minecraft:anvil", "minecraft:damaged_anvil"] {
        for facing in ["north"] {
            let original = with_property(state(name), "facing", facing).unwrap();
            for silent in [false, true] {
                let mut entity = body(original);
                entity.set_silent(silent);
                let mut level = NativeLevel::new();
                assert_eq!(
                    level.with_level(|level| on_land(
                        &entity,
                        None,
                        level,
                        LANDING,
                        original,
                        state("minecraft:water")
                    )),
                    Some(())
                );
                assert_eq!(
                    level.with_level(|level| on_broken_after_fall(
                        &entity, level, LANDING, original
                    )),
                    Some(())
                );
                let events = level.world.drain_level_events_for_test(DIMENSION);
                for event_type in [1031, 1029] {
                    assert_eq!(
                        events.iter().any(|event| event.event_type == event_type
                            && event.pos == LANDING
                            && event.data == 0),
                        !silent
                    );
                }
            }
        }
    }
}

#[test]
fn falling_fallable_hook_pointed_dripstone_directions_and_silent_gate() {
    for direction in ["down"] {
        let original = with_property(
            state("minecraft:pointed_dripstone"),
            "vertical_direction",
            direction,
        )
        .unwrap();
        for silent in [false, true] {
            let mut entity = body(original);
            entity.set_silent(silent);
            let mut level = NativeLevel::new();
            assert_eq!(
                level.with_level(|level| on_land(
                    &entity,
                    None,
                    level,
                    LANDING,
                    original,
                    state("minecraft:air")
                )),
                Some(())
            );
            assert!(level
                .world
                .drain_level_events_for_test(DIMENSION)
                .is_empty());
            assert_eq!(
                level.with_level(|level| on_broken_after_fall(&entity, level, LANDING, original)),
                Some(())
            );
            let events = level.world.drain_level_events_for_test(DIMENSION);
            assert_eq!(
                events.iter().any(|event| event.event_type == 1045
                    && event.pos == LANDING
                    && event.data == 0),
                !silent
            );
        }
    }
}

#[test]
fn falling_fallable_hook_brushable_uses_bbox_center_current_state_and_ignores_silent() {
    for name in ["minecraft:suspicious_sand"] {
        let original = state(name);
        let current = original;
        for silent in [false, true] {
            let mut entity = body(current);
            entity.set_silent(silent);
            let center = entity.bounding_box().center();
            assert_ne!(center, entity.position());
            assert_eq!(center.y.floor(), 71.0);
            let mut level = NativeLevel::new();
            assert_eq!(
                level.with_level(|level| on_land(
                    &entity,
                    None,
                    level,
                    LANDING,
                    original,
                    state("minecraft:air")
                )),
                Some(())
            );
            assert!(level
                .world
                .drain_level_events_for_test(DIMENSION)
                .is_empty());
            assert_eq!(
                level.with_level(|level| on_broken_after_fall(&entity, level, LANDING, original)),
                Some(())
            );
            assert!(level
                .world
                .drain_level_events_for_test(DIMENSION)
                .iter()
                .any(|event| event.event_type == 2001
                    && event.pos == BlockPos { x: 8, y: 71, z: 8 }
                    && event.data == current));
            assert!(level
                .world
                .positioned_game_events_for_test(DIMENSION)
                .iter()
                .any(
                    |event| event.recorded.game_event == VanillaGameEvent::BLOCK_DESTROY
                        && event.position == center
                        && event.recorded.source_entity_id == Some(37)
                ));
        }
    }
}

#[test]
fn falling_fallable_hook_default_and_non_fallable_are_noops() {
    let entity = body(state("minecraft:anvil"));
    for name in ["minecraft:sand", "minecraft:stone"] {
        let mut level = NativeLevel::new();
        assert_eq!(
            level.with_level(|level| on_land(
                &entity,
                None,
                level,
                LANDING,
                state(name),
                state("minecraft:air")
            )),
            Some(())
        );
        assert_eq!(
            level.with_level(|level| on_broken_after_fall(&entity, level, LANDING, state(name))),
            Some(())
        );
        assert!(
            level
                .world
                .drain_level_events_for_test(DIMENSION)
                .is_empty(),
            "{name}"
        );
        assert!(
            level
                .world
                .positioned_game_events_for_test(DIMENSION)
                .is_empty(),
            "{name}"
        );
    }
}
