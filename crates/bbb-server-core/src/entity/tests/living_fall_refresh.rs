use super::*;
use crate::entity::tests::native_level::NativeLevel;
use bbb_protocol::entity_types::*;

#[test]
fn flying_fall_check_preserves_health_on_the_real_floor() {
    let mut native = NativeLevel::new();
    for kind in [VANILLA_ENTITY_TYPE_PARROT_ID, VANILLA_ENTITY_TYPE_GHAST_ID] {
        let mut store = EntityStore::new(4, EntityIdCounter::new());
        let id = store
            .spawn_general_living(
                kind,
                Vec3d {
                    x: 8.5,
                    y: 64.0,
                    z: 8.5,
                },
            )
            .unwrap();
        let body = store.get_mut(id).unwrap();
        let health = body.living().unwrap().health();
        body.fall_distance = 5.0;
        body.on_ground = true;
        assert_eq!(
            native
                .with_level(|level| crate::entity::movement::check_fall_damage(body, level, -2.0)),
            None
        );
        assert_eq!(body.living().unwrap().health(), health);
        assert_eq!(body.fall_distance(), 5.0);
    }
}

#[test]
fn nonliving_descent_accumulates_saved_fall_distance() {
    let mut native = NativeLevel::new();
    let mut store = EntityStore::new(4, EntityIdCounter::new());
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: 8.5,
            y: 80.0,
            z: 8.5,
        },
        EntityData::Item(ItemEntityData::new(stack_of("minecraft:dirt", 1), 0.0)),
    );
    let body = store.get_mut(id).unwrap();
    body.fall_distance = 5.0;
    assert_eq!(
        native.with_level(|level| crate::entity::movement::check_fall_damage(body, level, -2.0)),
        None
    );
    assert_eq!(body.fall_distance(), 7.0);
}

#[test]
fn strider_lava_contact_resets_fall_distance_without_damage() {
    let mut native = NativeLevel::new();
    for y in 64..=66 {
        native.set(BlockPos { x: 8, y, z: 8 }, state_id("minecraft:lava"));
    }
    let mut store = EntityStore::new(4, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            Vec3d {
                x: 8.5,
                y: 64.0,
                z: 8.5,
            },
        )
        .unwrap();
    let body = store.get_mut(id).unwrap();
    body.first_tick = false;
    native.with_level(|level| {
        crate::entity::environment::update_fluid_interaction(body, level);
    });
    assert!(body.is_in_lava());
    let health = body.living().unwrap().health();
    body.fall_distance = 5.0;
    body.on_ground = true;
    assert_eq!(
        native.with_level(|level| crate::entity::movement::check_fall_damage(body, level, -2.0)),
        None
    );
    assert_eq!(body.fall_distance(), 0.0);
    assert_eq!(body.living().unwrap().health(), health);
}
