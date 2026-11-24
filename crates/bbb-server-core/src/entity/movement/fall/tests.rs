use crate::entity::tests::native_level::NativeLevel;
use crate::entity::{EntityIdCounter, EntityLifecycleEvent, EntityStore, MoverType};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
use bbb_protocol::packets::Vec3d;

#[test]
fn living_landing_publishes_dust_and_applies_actual_fall_damage() {
    for (distance, expected_health, particles) in
        [(0.0, 10.0, None), (2.0, 10.0, None), (8.0, 5.0, Some(80))]
    {
        let mut native = NativeLevel::new();
        let mut store = EntityStore::new(4, EntityIdCounter::new());
        let id = store
            .spawn_general_living(
                VANILLA_ENTITY_TYPE_COW_ID,
                Vec3d {
                    x: 8.5,
                    y: 64.0,
                    z: 8.5,
                },
            )
            .unwrap();
        store.drain_lifecycle_events();
        let mut body = store.detach(id).unwrap();
        body.fall_distance = distance;
        body.living_mut()
            .unwrap()
            .set_last_climbable_pos(Some(bbb_protocol::packets::BlockPos { x: 8, y: 70, z: 8 }));
        native
            .with_level(|level| {
                crate::entity::movement::move_living_entity(
                    &mut body,
                    &mut store,
                    level,
                    MoverType::Self_,
                    Vec3d {
                        x: 0.0,
                        y: -0.25,
                        z: 0.0,
                    },
                )
            })
            .unwrap();
        assert!(body.on_ground());
        assert_eq!(body.position().y, 64.0);
        assert_eq!(body.fall_distance(), 0.0);
        assert_eq!(body.living().unwrap().last_climbable_pos(), None);
        assert_eq!(body.living().unwrap().health(), expected_health);
        let dust: Vec<_> = store
            .drain_lifecycle_events()
            .into_iter()
            .filter_map(|event| match event {
                EntityLifecycleEvent::Particles { particles } => Some(particles),
                _ => None,
            })
            .collect();
        if let Some(count) = particles {
            assert!(dust.iter().any(|dust| dust.count == count));
        } else {
            assert!(dust.is_empty());
        }
    }
}
