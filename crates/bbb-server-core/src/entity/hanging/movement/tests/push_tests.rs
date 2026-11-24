use super::*;

fn stored_body(kind: usize) -> (EntityStore, i32) {
    let (mut store, entity) = body(kind);
    let id = entity.id();
    store.attach(entity);
    (store, id)
}

fn impulse() -> Vec3d {
    Vec3d {
        x: 0.25,
        y: 0.1,
        z: -0.5,
    }
}

#[test]
fn hanging_explosion_push_breaks_hanging_entities_with_drop_sound_and_event_rules() {
    for (kind, drops, silent) in CASES {
        let (mut store, id) = stored_body(kind);
        let entity = store.get_mut(id).unwrap();
        entity.set_invulnerable(true);
        entity.set_silent(silent);
        let before = (
            entity.position(),
            entity.delta_movement(),
            entity.fall_distance(),
        );
        let mut level = native(drops);
        assert_eq!(
            level.with_level(|level| store.push_hanging_entity(level, id, impulse())),
            Some(())
        );
        assert!(store.get(id).is_none());
        let lifecycle = store.drain_lifecycle_events();
        let removed = lifecycle
            .iter()
            .find_map(|event| match event {
                EntityLifecycleEvent::Removed { entity, .. } => Some(entity),
                _ => None,
            })
            .expect("the hanging entity is removed");
        assert_motion_unchanged(removed, before);
        assert_eq!(removed.removal_reason(), Some(RemovalReason::Killed));
        assert_eq!(count(&lifecycle, true), expected_drops(kind, drops));
        assert_eq!(
            count(&lifecycle, false),
            usize::from(!silent && (kind != 0 || drops))
        );
        assert_events(&level, kind, id);
    }
}

#[test]
fn hanging_explosion_push_zero_impulse_and_fixed_frame_are_noops() {
    let (mut store, id) = stored_body(1);
    let body = store.get(id).unwrap();
    let before = (body.position(), body.delta_movement(), body.fall_distance());
    let mut level = native(true);
    assert_eq!(
        level.with_level(|level| store.push_hanging_entity(level, id, Vec3d::default())),
        Some(())
    );
    assert_motion_unchanged(store.get(id).unwrap(), before);
    assert!(store.get(id).unwrap().removal_reason().is_none());
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .is_empty());
    assert!(store.drain_lifecycle_events().is_empty());

    let (mut store, id) = stored_body(1);
    frame_mut(store.get_mut(id).unwrap()).unwrap().fixed = true;
    let body = store.get(id).unwrap();
    let before = (body.position(), body.delta_movement(), body.fall_distance());
    assert_eq!(
        level.with_level(|level| store.push_hanging_entity(level, id, impulse())),
        Some(())
    );
    assert_motion_unchanged(store.get(id).unwrap(), before);
    assert!(store.get(id).unwrap().removal_reason().is_none());
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .is_empty());
    assert!(store.drain_lifecycle_events().is_empty());
}
