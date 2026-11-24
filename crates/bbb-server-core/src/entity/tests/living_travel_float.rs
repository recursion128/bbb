use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};

fn fixture() -> (EntityStore, i32, NativeLevel) {
    let mut store = EntityStore::new(91, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_STRIDER_ID,
            Vec3d {
                x: 8.5,
                y: 65.6,
                z: 8.5,
            },
        )
        .unwrap();
    let body = store.get_mut(id).unwrap();

    body.set_delta_movement(Vec3d {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    });
    let mut level = NativeLevel::new();

    level.world.set_block(
        DIMENSION,
        BlockPos { x: 8, y: 65, z: 8 },
        state_id("minecraft:lava"),
        18,
    );
    (store, id, level)
}

#[test]
fn living_travel_float_uses_entity_bottom_and_keeps_velocity_on_surface() {
    let (mut store, id, mut level) = fixture();
    let body = store.get_mut(id).unwrap();
    assert!(level.with_level(|ctx| crate::entity::strider::float_strider(body, ctx)));
    assert!(body.on_ground());
    assert_eq!(body.delta_movement().y, -1.0);
}

#[test]
fn living_travel_float_at_or_below_surface_adds_buoyancy() {
    let threshold = 65.5 - f64::from(1.0e-5_f32);
    for bottom in [threshold, f64::from_bits(threshold.to_bits() - 1)] {
        let (mut store, id, mut level) = fixture();
        level.world.set_block(
            DIMENSION,
            BlockPos { x: 8, y: 66, z: 8 },
            state_id("minecraft:air"),
            18,
        );
        let body = store.get_mut(id).unwrap();
        body.set_pos(Vec3d {
            y: bottom,
            ..body.position()
        });
        assert!(level.with_level(|ctx| crate::entity::strider::float_strider(body, ctx)));
        assert!(!body.on_ground());
        assert_eq!(body.delta_movement().y, -0.45);
    }
}
