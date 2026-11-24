use super::*;
use crate::entity::inside_blocks::apply_nonliving_inside_block_effects;
use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};
use crate::entity::{EntityIdCounter, EntityStore};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};

const SUPPORT: BlockPos = BlockPos { x: 0, y: 63, z: 0 };

fn body(kind: Option<i32>) -> (Box<Entity>, EntityStore) {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let pos = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let id = if let Some(kind) = kind {
        store.spawn_general_living(kind, pos).unwrap()
    } else {
        crate::entity::experience_orb::spawn_experience_orb(&mut store, pos, 1).unwrap()
    };
    let mut entity = store.detach(id).unwrap();
    entity.on_ground = true;
    (entity, store)
}

#[test]
fn step_on_stationary_magma_hurts_only_grounded_living_noncareful_bodies() {
    for (living, careful, grounded) in [
        (true, false, true),
        (false, false, true),
        (true, true, true),
        (true, false, false),
    ] {
        let (mut entity, mut store) = body(living.then_some(VANILLA_ENTITY_TYPE_PIG_ID));
        entity.on_ground = grounded;
        entity.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, careful);
        let mut native = NativeLevel::new();
        native.set(SUPPORT, state("minecraft:magma_block"));
        assert!(
            native.with_level(|level| apply_nonliving_inside_block_effects(
                &mut entity,
                level,
                Some(&mut store)
            ))
        );
        if living {
            assert_eq!(
                entity.living().unwrap().health(),
                if !careful && grounded { 9.0 } else { 10.0 }
            );
        } else {
            let crate::entity::EntityData::ExperienceOrb(data) = entity.data() else {
                unreachable!()
            };
            assert_eq!(data.health(), 5);
        }
    }
}

#[test]
fn step_on_stationary_slime_changes_horizontal_speed_unless_careful_or_fast_falling() {
    for (living, careful) in [(true, false), (false, true)] {
        for y in [0.05, -0.1] {
            let (mut entity, mut store) = body(living.then_some(VANILLA_ENTITY_TYPE_PIG_ID));
            entity.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, careful);
            entity.set_delta_movement(Vec3d { x: 2.0, y, z: -3.0 });
            let mut native = NativeLevel::new();
            native.set(SUPPORT, state("minecraft:slime_block"));
            assert!(
                native.with_level(|level| apply_nonliving_inside_block_effects(
                    &mut entity,
                    level,
                    Some(&mut store)
                ))
            );
            let expected = if !careful && y == 0.05 {
                Vec3d {
                    x: 0.82,
                    y,
                    z: -1.23,
                }
            } else {
                Vec3d { x: 2.0, y, z: -3.0 }
            };
            let actual = entity.delta_movement();
            assert!((actual.x - expected.x).abs() < 1e-12 && (actual.z - expected.z).abs() < 1e-12);
            assert_eq!(actual.y, expected.y);
        }
    }
}

#[test]
fn step_on_stationary_egg_breaks_for_eligible_mobs_and_respects_careful_and_griefing() {
    let egg = BlockPos { y: 64, ..SUPPORT };
    for (kind, careful, griefing) in [
        (Some(VANILLA_ENTITY_TYPE_PIG_ID), false, true),
        (Some(VANILLA_ENTITY_TYPE_PIG_ID), true, true),
        (Some(VANILLA_ENTITY_TYPE_PIG_ID), false, false),
        (Some(VANILLA_ENTITY_TYPE_TURTLE_ID), false, true),
        (Some(VANILLA_ENTITY_TYPE_BAT_ID), false, true),
        (None, false, true),
    ] {
        let (mut entity, mut store) = body(kind);
        entity.set_pos(Vec3d {
            y: 64.4375,
            ..entity.position()
        });
        entity.set_old_pos_and_rot();
        entity.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, careful);
        let mut native = NativeLevel::new();
        if !griefing {
            let (sender, receiver) = crate::tests::mailbox();
            crate::tests::console_command(
                &sender,
                &receiver,
                &mut native.world,
                1,
                "gamerule minecraft:mob_griefing false",
            );
        }
        native.set(SUPPORT, state("minecraft:sand"));
        native.set(egg, state("minecraft:turtle_egg"));
        for _ in 0..1024 {
            assert!(
                native.with_level(|level| apply_nonliving_inside_block_effects(
                    &mut entity,
                    level,
                    Some(&mut store)
                ))
            );
        }
        assert_eq!(
            native.world.block_state(DIMENSION, egg).unwrap(),
            if kind == Some(VANILLA_ENTITY_TYPE_PIG_ID) && !careful && griefing {
                state("minecraft:air")
            } else {
                state("minecraft:turtle_egg")
            }
        );
    }
}

#[test]
fn step_on_stationary_redstone_ore_lights_for_all_noncareful_bodies() {
    for living in [false, true] {
        for careful in [false, true] {
            let (mut entity, mut store) = body(living.then_some(VANILLA_ENTITY_TYPE_PIG_ID));
            entity.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, careful);
            let mut native = NativeLevel::new();
            native.set(SUPPORT, state("minecraft:redstone_ore"));
            assert!(
                native.with_level(|level| apply_nonliving_inside_block_effects(
                    &mut entity,
                    level,
                    Some(&mut store)
                ))
            );
            assert_eq!(
                bbb_protocol::block_state_types::vanilla_block_state_property(
                    native.world.block_state(DIMENSION, SUPPORT).unwrap(),
                    "lit"
                ),
                Some(if careful { "false" } else { "true" })
            );
        }
    }
}

/// `Cat.isSteppingCarefully` (`Cat.java:484-486`): a crouching cat walks
/// over magma unhurt, a standing one does not.
#[test]
fn a_crouching_cat_steps_carefully_over_magma() {
    for crouching in [false, true] {
        let (mut entity, mut store) =
            body(Some(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAT_ID));
        if crouching {
            entity.set_pose_id(bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID);
        }
        let before = entity.living().unwrap().health();
        let mut native = NativeLevel::new();
        native.set(SUPPORT, state("minecraft:magma_block"));
        native.with_level(|level| {
            apply_nonliving_inside_block_effects(&mut entity, level, Some(&mut store))
        });
        assert_eq!(
            entity.living().unwrap().health() < before,
            !crouching,
            "crouching: {crouching}"
        );
    }
}
