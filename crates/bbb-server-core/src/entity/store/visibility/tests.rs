use super::*;
use crate::entity::{spawn_armor_stand, EntityIdCounter, LastHurtByMob, RemovalReason};
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Aabb;

fn at(x: f64, y: f64) -> Vec3d {
    Vec3d { x, y, z: 0.5 }
}

fn sorted(mut ids: Vec<i32>) -> Vec<i32> {
    ids.sort_unstable();
    ids
}

fn box_at(x: f64) -> Aabb {
    Aabb {
        min_x: x - 1.0,
        min_y: 63.0,
        min_z: -1.0,
        max_x: x + 1.0,
        max_y: 67.0,
        max_z: 2.0,
    }
}

#[test]
fn managed_visibility_hides_tracks_and_restarts_ticking() {
    let mut store = EntityStore::new_managed(
        crate::random::GameRandom::with_seed(12),
        EntityIdCounter::new(),
    );
    let upper = spawn_armor_stand(&mut store, at(0.5, 80.0)).unwrap();
    let lower = spawn_armor_stand(&mut store, at(0.5, 64.0)).unwrap();
    let peer = spawn_armor_stand(&mut store, at(1.5, 64.0)).unwrap();
    let before = store.get(lower).unwrap().clone();
    assert!(store.visible_ids().next().is_none());
    assert!(store.ticking_ids().is_empty());
    assert!(store.get_visible(lower).is_none());
    assert_eq!(store.entity_id_for_uuid(before.uuid()), Some(lower));
    assert_eq!(store.visible_entity_id_for_uuid(before.uuid()), None);
    assert_eq!(
        LastHurtByMob::Uuid(before.uuid()).stored_uuid_target(&store),
        None
    );
    assert!(store
        .drain_lifecycle_events()
        .iter()
        .all(|event| matches!(event, EntityLifecycleEvent::Added { tracked: false, .. })));
    store.apply_chunk_visibility((0, 0), EntityVisibility::Ticking);
    let all = sorted(vec![upper, lower, peer]);
    assert_eq!(sorted(store.visible_ids().collect()), all);
    assert_eq!(sorted(store.ticking_ids()), all);
    let started = store.drain_lifecycle_events();
    assert_eq!(
        sorted(
            started
                .iter()
                .map(|event| match event {
                    EntityLifecycleEvent::TrackingStarted { entity } => entity.id(),
                    other => panic!("unexpected {other:?}"),
                })
                .collect()
        ),
        all
    );
    assert_eq!(
        LastHurtByMob::Uuid(before.uuid()).stored_uuid_target(&store),
        Some(lower)
    );
    store.apply_chunk_visibility((0, 0), EntityVisibility::Tracked);
    assert!(store.ticking_ids().is_empty());
    assert!(store.drain_lifecycle_events().is_empty());
    assert_eq!(store.get_visible(lower), Some(&before));
    store.apply_chunk_visibility((0, 0), EntityVisibility::Hidden);
    assert_eq!(store.get(lower), Some(&before));
    assert!(store.visible_ids().next().is_none());
    assert!(store
        .drain_lifecycle_events()
        .iter()
        .all(|event| matches!(event, EntityLifecycleEvent::TrackingStopped { .. })));
    store.apply_chunk_visibility((0, 0), EntityVisibility::Ticking);
    assert_eq!(store.get_visible(lower), Some(&before));
    assert_eq!(sorted(store.ticking_ids()), all);
    store.drain_lifecycle_events();
    store.remove(lower, RemovalReason::Discarded).unwrap();
    assert!(
        matches!(store.drain_lifecycle_events().as_slice(), [EntityLifecycleEvent::Removed { entity, tracked: true }] if entity.id() == lower)
    );
    assert!(!store.contains_uuid(before.uuid()));
}

#[test]
fn visibility_detached_move_uses_actual_snapshot_and_accessible_sections() {
    let mut store = EntityStore::new_managed(
        crate::random::GameRandom::with_seed(13),
        EntityIdCounter::new(),
    );
    store.apply_chunk_visibility((0, 0), EntityVisibility::Ticking);
    store.apply_chunk_visibility((1, 0), EntityVisibility::Tracked);
    let id = spawn_armor_stand(&mut store, at(0.5, 64.0)).unwrap();
    assert!(matches!(
        store.drain_lifecycle_events().as_slice(),
        [EntityLifecycleEvent::Added { tracked: true, .. }]
    ));
    let mut body = store.detach(id).unwrap();
    body.set_pos(at(16.5, 64.0));
    store.refile_detached(&mut body);
    assert!(!store.is_ticking(id));
    assert_eq!(
        crate::entity::store::EntityBodyReadView::new(&store, Some(&body), None)
            .ids_in_box_sections(box_at(16.5)),
        [id]
    );
    assert_eq!(
        store.drain_lifecycle_events().as_slice(),
        [EntityLifecycleEvent::Moved {
            entity_id: id,
            from: SectionKey::new(0, 4, 0),
            to: SectionKey::new(1, 4, 0),
        }]
    );
    body.set_pos(at(32.5, 64.0));
    store.refile_detached(&mut body);
    assert!(
        crate::entity::store::EntityBodyReadView::new(&store, Some(&body), None)
            .ids_in_box_sections(box_at(32.5))
            .is_empty()
    );
    assert!(
        matches!(store.drain_lifecycle_events().as_slice(), [EntityLifecycleEvent::TrackingStopped { entity }] if entity.position().x == 32.5)
    );
    body.set_pos(at(0.5, 64.0));
    store.refile_detached(&mut body);
    assert!(store.is_ticking(id));
    assert!(
        matches!(store.drain_lifecycle_events().as_slice(), [EntityLifecycleEvent::TrackingStarted { entity }, EntityLifecycleEvent::Moved { .. }] if entity.position().x == 0.5)
    );
    store.attach(body);
    assert_eq!(store.visible_ids().collect::<Vec<_>>(), [id]);
}

#[test]
fn visibility_post_save_unload_removes_the_chunk_and_retains_player_carriers() {
    let mut store = EntityStore::new_managed(
        crate::random::GameRandom::with_seed(16),
        EntityIdCounter::new(),
    );
    let upper = spawn_armor_stand(&mut store, at(0.5, 80.0)).unwrap();
    let lower = spawn_armor_stand(&mut store, at(0.5, 64.0)).unwrap();
    let passenger = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID,
            at(0.5, 64.0),
        )
        .unwrap();
    assert!(store.link_existing_passenger(lower, passenger));
    store.get_mut(passenger).unwrap().set_pos(at(32.5, 64.0));
    store.refile(passenger);
    let carrier = spawn_armor_stand(&mut store, at(1.5, 64.0)).unwrap();
    assert!(store.attach_test_player(100, carrier, crate::entity::ExternalPlayerCanRide::Allowed));
    store.drain_lifecycle_events();
    store.unload_chunk_entities_after_save((0, 0));
    let removed = store
        .drain_lifecycle_events()
        .into_iter()
        .filter_map(|event| match event {
            EntityLifecycleEvent::Removed {
                entity,
                tracked: false,
            } => {
                assert_eq!(
                    entity.removal_reason(),
                    Some(RemovalReason::UnloadedToChunk)
                );
                Some(entity.id())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(sorted(removed), sorted(vec![passenger, lower, upper]));
    assert_eq!(store.ids(), [carrier]);
    assert_eq!(store.external_player_structural_vehicle(100), Some(carrier));
}
