use super::*;
use bbb_protocol::entity_types::*;

const CHILD: Vec3d = Vec3d { x: 1.5, ..FEET };

fn tree(kind: i32) -> (EntityStore, NativeLevel, Box<Entity>, i32) {
    let (mut store, id) = walking_store(kind);
    let child = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, CHILD)
        .unwrap();
    assert!(store.link_existing_passenger_forced(id, child));
    let mut body = store.detach(id).unwrap();
    standard_fall_attributes(&mut body);
    store.drain_lifecycle_events();
    (store, gold_plate(), body, child)
}

#[test]
fn stored_passenger_fall_horse_hurts_itself_and_its_rider() {
    let (mut store, mut level, mut body, child) = tree(VANILLA_ENTITY_TYPE_HORSE_ID);
    let health = body.living().unwrap().health();
    provider_fall(
        &mut body,
        &mut store,
        &mut level,
        8.0,
        1.0,
        "minecraft:fall",
    );
    assert_eq!(body.living().unwrap().health(), health - 5.0);
    assert_eq!(store.get(child).unwrap().living().unwrap().health(), 15.0);
}

#[test]
fn stored_passenger_fall_short_llama_skips_hurt_and_children_even_for_positive_damage() {
    for distance in [5.0, 6.0] {
        let (mut store, mut level, mut body, child) = tree(VANILLA_ENTITY_TYPE_LLAMA_ID);
        let health = body.living().unwrap().health();
        provider_fall(
            &mut body,
            &mut store,
            &mut level,
            distance,
            1.0,
            "minecraft:fall",
        );
        let damage = if distance < 6.0 { 0.0 } else { 3.0 };
        assert_eq!(body.living().unwrap().health(), health - damage);
        assert_eq!(
            store.get(child).unwrap().living().unwrap().health(),
            20.0 - damage
        );
    }
}

#[test]
fn stored_passenger_fall_base_result_ignores_positive_child_result() {
    let mut store = EntityStore::new(17, EntityIdCounter::new());
    let root =
        crate::entity::spawn_boat(&mut store, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, FEET, 0.0).unwrap();
    let child = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, CHILD)
        .unwrap();
    assert!(store.link_existing_passenger_forced(root, child));
    let mut body = store.detach(root).unwrap();
    let mut level = gold_plate();
    provider_fall(
        &mut body,
        &mut store,
        &mut level,
        8.0,
        1.0,
        "minecraft:fall",
    );
    assert_eq!(store.get(child).unwrap().living().unwrap().health(), 15.0);
}

#[test]
fn stored_passenger_fall_real_living_tick_applies_child_and_parent_damage() {
    let (mut store, mut level, mut body, child) = tree(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let root = body.id();
    body.fall_distance = 8.0;
    body.set_delta_movement(Vec3d {
        x: 0.0,
        y: -0.01,
        z: 0.0,
    });
    store.attach(body);
    assert!(matches!(
        level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, root)),
        crate::entity::EntityTickOutcome::Complete
    ));

    assert_eq!(store.get(root).unwrap().living().unwrap().health(), 15.0);
    assert_eq!(store.get(child).unwrap().living().unwrap().health(), 15.0);
}

#[test]
fn stored_passenger_fall_health_zero_child_still_propagates_to_its_rider() {
    let (mut store, mut level, mut body, child) = tree(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    store
        .get_mut(child)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_health(0.0);
    let grandchild = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, Vec3d { x: 2.5, ..FEET })
        .unwrap();
    assert!(store.link_existing_passenger_forced(child, grandchild));
    provider_fall(
        &mut body,
        &mut store,
        &mut level,
        8.0,
        1.0,
        "minecraft:fall",
    );
    assert_eq!(store.get(child).unwrap().living().unwrap().health(), 0.0);
    assert_eq!(
        store.get(grandchild).unwrap().living().unwrap().health(),
        15.0
    );
}

#[test]
fn stored_passenger_fall_zero_parent_damage_still_hurts_child_without_changing_result() {
    let (mut store, mut level, mut body, child) = tree(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    body.living_mut()
        .unwrap()
        .attributes_mut()
        .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SAFE_FALL_DISTANCE_ID)
        .unwrap()
        .set_base_value(100.0);
    provider_fall(
        &mut body,
        &mut store,
        &mut level,
        8.0,
        1.0,
        "minecraft:fall",
    );
    assert_eq!(body.living().unwrap().health(), 20.0);
    assert_eq!(store.get(child).unwrap().living().unwrap().health(), 15.0);
}
