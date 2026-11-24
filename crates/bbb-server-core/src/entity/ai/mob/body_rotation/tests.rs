use bbb_protocol::entity_types::*;
use bbb_protocol::packets::Vec3d;
use uuid::Uuid;

use super::*;
use crate::entity::general_living::GeneralLivingData;
use crate::entity::living::LivingEntityState;
use crate::entity::store::{EntityIdCounter, EntityStore};
use crate::entity::tests::native_level::NativeLevel;
use crate::entity::{tick_entities, EntityData};

fn position(x: f64) -> Vec3d {
    Vec3d { x, y: 64.0, z: 0.5 }
}

#[test]
fn mob_body_rotation_uses_exact_max_head_yaw_table() {
    assert_eq!(
        base_body_rotation_max_yaw(VANILLA_ENTITY_TYPE_RAVAGER_ID),
        Some(45.0)
    );
    assert_eq!(
        base_body_rotation_max_yaw(VANILLA_ENTITY_TYPE_BREEZE_ID),
        Some(30.0)
    );
    assert_eq!(
        base_body_rotation_max_yaw(VANILLA_ENTITY_TYPE_GOAT_ID),
        Some(15.0)
    );
    assert_eq!(
        base_body_rotation_max_yaw(VANILLA_ENTITY_TYPE_DOLPHIN_ID),
        Some(1.0)
    );
    assert_eq!(
        base_body_rotation_max_yaw(VANILLA_ENTITY_TYPE_SNIFFER_ID),
        Some(50.0)
    );
    assert_eq!(
        base_body_rotation_max_yaw(VANILLA_ENTITY_TYPE_AXOLOTL_ID),
        Some(1.0)
    );
    assert_eq!(
        base_body_rotation_max_yaw(VANILLA_ENTITY_TYPE_FROG_ID),
        Some(5.0)
    );
    assert_eq!(
        base_body_rotation_max_yaw(VANILLA_ENTITY_TYPE_ZOMBIE_ID),
        Some(75.0)
    );
    assert_eq!(
        base_body_rotation_max_yaw(VANILLA_ENTITY_TYPE_CAMEL_ID),
        None
    );
    assert_eq!(
        base_body_rotation_max_yaw(VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID),
        None
    );
}

/// `Mob.getMaxHeadXRot` and every override 26.1 declares
/// of it: the five that answer and the Wolf, whose sitting pose this fact
/// cannot carry.
#[test]
fn mob_head_x_rot_answers_the_default_and_all_five_overrides() {
    assert_eq!(
        base_max_head_x_rot(VANILLA_ENTITY_TYPE_ZOMBIE_ID),
        Some(40.0)
    );
    assert_eq!(
        base_max_head_x_rot(VANILLA_ENTITY_TYPE_GUARDIAN_ID),
        Some(180.0)
    );
    assert_eq!(
        base_max_head_x_rot(VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID),
        Some(180.0)
    );
    assert_eq!(
        base_max_head_x_rot(VANILLA_ENTITY_TYPE_SHULKER_ID),
        Some(180.0)
    );
    assert_eq!(
        base_max_head_x_rot(VANILLA_ENTITY_TYPE_AXOLOTL_ID),
        Some(1.0)
    );
    assert_eq!(
        base_max_head_x_rot(VANILLA_ENTITY_TYPE_DOLPHIN_ID),
        Some(1.0)
    );
    assert_eq!(base_max_head_x_rot(VANILLA_ENTITY_TYPE_SLIME_ID), Some(0.0));
    assert_eq!(
        base_max_head_x_rot(VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID),
        Some(0.0)
    );
    assert_eq!(base_max_head_x_rot(VANILLA_ENTITY_TYPE_WOLF_ID), None);
}

#[test]
fn mob_body_rotation_moving_branch_faces_body_and_clamps_head() {
    let mut below_threshold =
        BodyRotationControl::for_entity(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap();
    let mut below_threshold_living = LivingEntityState::new("minecraft:zombie").unwrap();
    below_threshold_living.y_head_rot = 120.0;
    below_threshold.tick(
        position(0.5),
        position(0.5005),
        30.0,
        &mut below_threshold_living.y_body_rot,
        &mut below_threshold_living.y_head_rot,
        false,
    );
    assert_eq!(below_threshold_living.y_body_rot, 45.0);
    assert_eq!(below_threshold_living.y_head_rot, 120.0);

    let mut control = BodyRotationControl::for_entity(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap();
    let mut living = LivingEntityState::new("minecraft:zombie").unwrap();
    living.y_head_rot = 120.0;

    control.tick(
        position(0.5),
        position(0.501),
        30.0,
        &mut living.y_body_rot,
        &mut living.y_head_rot,
        false,
    );

    assert_eq!(living.y_body_rot, 30.0);
    assert_eq!(living.y_head_rot, 105.0);
}

#[test]
fn mob_body_rotation_stationary_branch_delays_then_converges() {
    let mut control = BodyRotationControl::for_entity(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap();
    let mut living = LivingEntityState::new("minecraft:zombie").unwrap();
    living.y_head_rot = 90.0;

    control.tick(
        position(0.5),
        position(0.5),
        0.0,
        &mut living.y_body_rot,
        &mut living.y_head_rot,
        false,
    );
    assert_eq!(living.y_body_rot, 15.0);

    for _ in 0..10 {
        control.tick(
            position(0.5),
            position(0.5),
            0.0,
            &mut living.y_body_rot,
            &mut living.y_head_rot,
            false,
        );
    }
    assert_eq!(living.y_body_rot, 15.0);

    control.tick(
        position(0.5),
        position(0.5),
        0.0,
        &mut living.y_body_rot,
        &mut living.y_head_rot,
        false,
    );
    assert_eq!(living.y_body_rot, 22.5);
    for _ in 0..9 {
        control.tick(
            position(0.5),
            position(0.5),
            0.0,
            &mut living.y_body_rot,
            &mut living.y_head_rot,
            false,
        );
    }
    assert_eq!(living.y_body_rot, 90.0);
}

#[test]
fn mob_body_rotation_normalizes_retained_wraps() {
    let mut living = LivingEntityState::new("minecraft:zombie").unwrap();
    living.y_body_rot = -190.0;
    living.y_body_rot_o = 190.0;
    living.y_head_rot = 190.0;
    living.y_head_rot_o = -190.0;

    normalize_retained_rotations(&mut living);

    assert_eq!(living.y_body_rot_o, -170.0);
    assert_eq!(living.y_head_rot_o, 170.0);
}

#[test]
fn no_ai_ground_and_flying_mobs_keep_their_position_with_velocity() {
    for entity_type in [
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        VANILLA_ENTITY_TYPE_PHANTOM_ID,
    ] {
        let mut data = GeneralLivingData::new(entity_type).unwrap();
        data.set_no_ai(true);
        let mut store = EntityStore::new(1, EntityIdCounter::new());
        let position = Vec3d {
            y: 72.0,
            ..position(0.5)
        };
        let id = store
            .restore(
                Uuid::from_u128(1),
                entity_type,
                position,
                EntityData::GeneralLiving(data),
            )
            .unwrap();
        let body = store.get_mut(id).unwrap();
        body.set_delta_movement(Vec3d {
            x: 0.1,
            y: -0.2,
            z: 0.05,
        });
        body.fall_distance = 8.0;
        let effect = if entity_type == VANILLA_ENTITY_TYPE_ZOMBIE_ID {
            "minecraft:slow_falling"
        } else {
            "minecraft:levitation"
        };
        assert!(body
            .living_mut()
            .unwrap()
            .add_active_effect(crate::entity::ActiveMobEffect::new(effect, 100, 0).unwrap()));
        let mut native = NativeLevel::new();
        for _ in 0..10 {
            native.with_level(|level| tick_entities(&mut store, level, true));
            assert_eq!(store.get(id).unwrap().position(), position);
            assert_eq!(store.get(id).unwrap().fall_distance(), 0.0);
        }
    }
}
