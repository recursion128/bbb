use super::*;
use crate::entity::store::EntityIdCounter;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};

fn origin() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}
fn destination() -> Vec3d {
    Vec3d {
        x: 64.5,
        y: 96.0,
        z: 64.5,
    }
}

fn nested() -> (EntityStore, i32, i32, i32) {
    let mut store = EntityStore::new(0x74656c65, EntityIdCounter::new());
    let root = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, origin())
        .unwrap();
    let child = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, origin())
        .unwrap();
    let grandchild = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, origin())
        .unwrap();
    assert!(store.link_existing_passenger_forced(root, child));
    assert!(store.link_existing_passenger_forced(child, grandchild));
    (store, root, child, grandchild)
}

#[test]
fn trial_and_fallback_snap_the_nested_tree_without_restoring_old_clones() {
    let (mut store, root, child, grandchild) = nested();
    let mut body = store.detach(root).unwrap();
    body.set_rot(37.0, 12.0);
    let mut tree = TeleportTree::capture(&body, &store).unwrap();
    assert_eq!(tree.overlay().len(), 3);
    tree.snap_to(destination(), 9).unwrap();
    assert_eq!(tree.body().position(), destination());
    for entity in tree.overlay().values() {
        assert_eq!(entity.old_position(), entity.position());
    }
    assert!(tree.overlay()[&grandchild].position().y > tree.overlay()[&child].position().y);
    assert_eq!(store.get(child).unwrap().position(), origin());
    tree.snap_to(origin(), 9).unwrap();
    assert_eq!(tree.body().position(), origin());
    assert_ne!(tree.overlay()[&child].position(), origin());
    let velocity = Vec3d {
        x: 0.25,
        y: -0.5,
        z: 0.75,
    };
    body.set_delta_movement(velocity);
    crate::entity::living::set_health(&mut body, 8.0);
    store.get_mut(child).unwrap().set_delta_movement(velocity);
    crate::entity::living::set_health(store.get_mut(child).unwrap(), 7.0);
    let expected: BTreeMap<_, _> = tree
        .overlay()
        .iter()
        .map(|(id, entity)| (*id, entity.position()))
        .collect();
    tree.apply_to(&mut body, &mut store).unwrap();
    assert_eq!(body.delta_movement(), velocity);
    assert_eq!(body.living().unwrap().health(), 8.0);
    assert_eq!((body.y_rot(), body.x_rot()), (37.0, 12.0));
    assert_eq!(body.old_position(), origin());
    assert_eq!(store.get(child).unwrap().delta_movement(), velocity);
    assert_eq!(store.get(child).unwrap().living().unwrap().health(), 7.0);
    for id in [child, grandchild] {
        assert_eq!(store.get(id).unwrap().position(), expected[&id]);
        assert_eq!(store.get(id).unwrap().old_position(), expected[&id]);
    }
    assert_eq!(store.vehicle(child), Some(root));
    assert_eq!(store.vehicle(grandchild), Some(child));
}

#[test]
fn boat_snap_reuses_seat_rotation_and_preserves_delta_movement() {
    let mut store = EntityStore::new(0x626f6174, EntityIdCounter::new());
    let root =
        crate::entity::spawn_boat(&mut store, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, origin(), 30.0)
            .unwrap();
    let child = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, origin())
        .unwrap();
    assert!(store.link_existing_passenger_forced(root, child));
    store.get_mut(child).unwrap().set_rot(180.0, 17.0);
    let velocity = Vec3d {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };
    store.get_mut(child).unwrap().set_delta_movement(velocity);
    let mut body = store.detach(root).unwrap();
    let mut tree = TeleportTree::capture(&body, &store).unwrap();
    tree.snap_to(destination(), 12).unwrap();
    let passenger = &tree.overlay()[&child];
    assert_eq!(passenger.y_rot(), 135.0);
    assert_eq!(passenger.x_rot(), 17.0);
    assert_eq!(passenger.living().unwrap().y_body_rot, 30.0);
    assert_eq!(passenger.living().unwrap().y_head_rot, 135.0);
    let position = passenger.position();
    tree.apply_to(&mut body, &mut store).unwrap();
    let passenger = store.get(child).unwrap();
    assert_eq!(passenger.old_position(), position);
    assert_eq!(passenger.y_rot(), 135.0);
    assert_eq!(passenger.delta_movement(), velocity);
}

#[test]
fn checked_same_vehicle_uses_the_full_chain() {
    let (mut store, root, child, grandchild) = nested();
    let unrelated = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, origin())
        .unwrap();
    let body = store.detach(child).unwrap();
    assert_eq!(store.checked_same_vehicle(&body, grandchild), Some(true));
    assert_eq!(store.checked_same_vehicle(&body, root), Some(true));
    assert_eq!(store.checked_same_vehicle(&body, unrelated), Some(false));
}
