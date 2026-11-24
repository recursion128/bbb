use super::*;
use crate::entity::environment::update_fluid_interaction_checked;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};

const DRIFT: Vec3d = Vec3d {
    x: 0.03,
    y: -0.125,
    z: 0.0,
};

fn prior_wet_body() -> Box<crate::entity::Entity> {
    let mut store = EntityStore::new(17, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:cow", FEET);
    let mut entity = store.detach(id).unwrap();
    entity.first_tick = false;
    let mut mixed = sloping_pool("minecraft:water");
    mixed.world.set_block(
        DIMENSION,
        BlockPos { x: 0, y: 65, z: 0 },
        state_id("minecraft:lava"),
        18,
    );
    mixed.world.set_block(
        DIMENSION,
        BlockPos { x: 1, y: 65, z: 0 },
        state_with("minecraft:lava", &[("level", "1")]),
        18,
    );
    assert_eq!(
        mixed.with_level(|ctx| update_fluid_interaction_checked(&mut entity, ctx)),
        Some(())
    );
    assert!(entity.is_in_water());
    assert!(entity.is_in_lava());
    assert!(entity.was_touching_water());
    assert!(entity.is_eye_in_fluid(FluidTag::Lava));
    entity.fall_distance = 7.0;
    entity.set_delta_movement(DRIFT);
    entity
}

#[test]
fn fall_fluid_refresh_dry_clears_prior_trackers_without_resetting_distance_or_delta() {
    let mut entity = prior_wet_body();
    let mut level = NativeLevel::new();

    assert_eq!(
        level.with_level(|ctx| update_fluid_interaction_checked(&mut entity, ctx)),
        Some(())
    );
    assert!(!entity.is_in_water());
    assert!(!entity.is_in_lava());
    assert!(!entity.was_touching_water());
    assert!(!entity.is_eye_in_fluid(FluidTag::Lava));
    assert_eq!(entity.fall_distance(), 7.0);
    assert_eq!(entity.delta_movement(), DRIFT);
}

#[test]
fn fall_fluid_refresh_water_and_both_lava_scales_push_the_body() {
    for (family, fast, scale) in [
        ("minecraft:water", false, WATER_CURRENT_SCALE),
        ("minecraft:lava", false, LAVA_CURRENT_SCALE),
        ("minecraft:lava", true, FAST_LAVA_CURRENT_SCALE),
    ] {
        let mut entity = prior_wet_body();
        let mut level = fluid_pool(family, fast);
        assert_eq!(
            level.with_level(|ctx| update_fluid_interaction_checked(&mut entity, ctx)),
            Some(())
        );
        assert_eq!(entity.is_in_water(), family == "minecraft:water");
        assert_eq!(entity.is_in_lava(), family == "minecraft:lava");
        assert_eq!(entity.was_touching_water(), family == "minecraft:water");
        assert_eq!(
            entity.fall_distance(),
            if family == "minecraft:water" {
                0.0
            } else {
                7.0
            }
        );
        assert_eq!(
            entity.delta_movement(),
            Vec3d {
                x: DRIFT.x + scale,
                ..DRIFT
            }
        );
    }
}

#[test]
fn fall_fluid_refresh_mixed_water_and_lava_reset_before_applying_water_then_lava() {
    let mut entity = prior_wet_body();
    let mut level = sloping_pool("minecraft:water");
    level.world.set_block(
        DIMENSION,
        BlockPos { x: 0, y: 65, z: 0 },
        state_id("minecraft:lava"),
        18,
    );
    level.world.set_block(
        DIMENSION,
        BlockPos { x: 1, y: 65, z: 0 },
        state_with("minecraft:lava", &[("level", "1")]),
        18,
    );
    assert_eq!(
        level.with_level(|ctx| update_fluid_interaction_checked(&mut entity, ctx)),
        Some(())
    );
    assert_eq!(entity.fall_distance(), 0.0);
    assert_eq!(
        entity.delta_movement(),
        Vec3d {
            x: (DRIFT.x + WATER_CURRENT_SCALE) + LAVA_CURRENT_SCALE,
            ..DRIFT
        }
    );
    assert!(entity.was_touching_water());
    assert!(entity.is_in_lava());
}
