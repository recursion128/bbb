use super::*;

#[test]
fn display_spawn_and_canonical_reload_move_through_solid_blocks() {
    for data in [
        EntityData::BlockDisplay(crate::entity::BlockDisplayData::new()),
        EntityData::ItemDisplay(crate::entity::ItemDisplayData::new()),
        EntityData::TextDisplay(crate::entity::TextDisplayData::new()),
    ] {
        let mut spawned = EntityStore::new(78, EntityIdCounter::new());
        let id = spawn(&mut spawned, data, origin());
        let saved = write_entity_chunk(&spawned, (0, 0)).unwrap();
        let mut reloaded = EntityStore::new(79, EntityIdCounter::new());
        let restored_id = read_entity_chunk(&mut reloaded, &saved).unwrap()[0];
        for (mut store, id) in [(spawned, id), (reloaded, restored_id)] {
            let mut entity = store.detach(id).unwrap();
            let mut level = NativeLevel::new();
            let velocity = Vec3d {
                x: 0.1,
                y: -0.2,
                z: 0.3,
            };
            let delta = Vec3d {
                x: 0.5,
                y: -1.0,
                z: 0.25,
            };
            entity.set_delta_movement(velocity);
            let filed = entity.filed_section();
            for mover in [
                crate::entity::MoverType::Self_,
                crate::entity::MoverType::Piston,
            ] {
                let before = entity.position();
                level.with_level(|ctx| {
                    crate::entity::move_entity(&mut entity, &mut store, ctx, mover, delta)
                });
                assert_eq!(
                    entity.position(),
                    Vec3d {
                        x: before.x + delta.x,
                        y: before.y + delta.y,
                        z: before.z + delta.z
                    }
                );
                assert!(entity.no_physics());
                assert_eq!(entity.delta_movement(), velocity);
                assert!(!entity.horizontal_collision());
                assert!(!entity.vertical_collision());
            }
            assert_ne!(entity.filed_section(), filed);
            assert!(store.drain_lifecycle_events().iter().any(
                |event| matches!(event, EntityLifecycleEvent::Moved { entity_id, to, .. }
                    if *entity_id == entity.id() && *to == entity.filed_section())
            ));
            assert!(level
                .world
                .positioned_game_events_for_test(DIMENSION)
                .is_empty());
        }
    }
}
