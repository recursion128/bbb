use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use bbb_protocol::entity_types::*;
use bbb_protocol::packets::LevelParticles;
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_BLOCK_ID;

fn falling_living(stand: bool, distance: f64, silent: bool) -> (EntityStore, NativeLevel, i32) {
    let mut store = EntityStore::new(31, EntityIdCounter::new());
    let position = Vec3d {
        x: 0.5,
        y: 64.25,
        z: 0.5,
    };
    let id = if stand {
        crate::entity::spawn_armor_stand(&mut store, position).unwrap()
    } else {
        store
            .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position)
            .unwrap()
    };
    let body = store.get_mut(id).unwrap();
    body.fall_distance = distance;
    body.set_silent(silent);
    body.set_delta_movement(Vec3d {
        x: 0.0,
        y: -0.5,
        z: 0.0,
    });
    store.drain_lifecycle_events();
    (
        store,
        {
            let mut native = NativeLevel::new();
            for x in -3..=3 {
                for z in -3..=3 {
                    native.world.set_block(
                        DIMENSION,
                        BlockPos { x, y: 63, z },
                        state_id("minecraft:stone"),
                        18,
                    );
                }
            }
            native
        },
        id,
    )
}

fn particles(events: &[EntityLifecycleEvent]) -> Vec<&LevelParticles> {
    events
        .iter()
        .filter_map(|event| match event {
            EntityLifecycleEvent::Particles { particles } => Some(particles),
            _ => None,
        })
        .collect()
}

#[test]
fn living_fall_particles_root_tick_uses_old_distance_and_ignores_silent() {
    for (stand, silent) in [(false, false), (true, true)] {
        {
            let (mut store, mut level, id) = falling_living(stand, 8.8, silent);
            let health = store.get(id).unwrap().living().unwrap().health();
            level.with_level(|ctx| tick_entities(&mut store, ctx, true));
            let body = store.get(id).unwrap();
            assert_eq!(
                body.position(),
                Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5
                }
            );
            assert!(body.on_ground());
            assert_eq!(body.fall_distance(), 0.0);
            // Old 8.8 produces power 5; the clipped descent produces 9.05 and damage 6.
            // ArmorStand rejects ordinary fall hurt after the shared Living prefix.
            assert_eq!(
                body.living().unwrap().health(),
                health - if stand { 0.0 } else { 6.0 }
            );
            let events = store.drain_lifecycle_events();
            let packets = particles(&events);
            assert_eq!(
                packets.len(),
                1,
                "stand={stand}, silent={silent}: {events:?}"
            );
            let packet = packets[0];
            assert!(!packet.override_limiter);
            assert!(!packet.always_show);
            assert_eq!(
                packet.position,
                Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5
                }
            );
            assert_eq!(packet.offset, Vec3d::default());
            assert_eq!(packet.count, 80);
            assert_eq!(packet.max_speed, 0.15_f32);
            assert_eq!(
                packet.particle.particle_type_id,
                VANILLA_PARTICLE_TYPE_BLOCK_ID
            );
            let mut decoder = bbb_protocol::codec::Decoder::new(&packet.particle.raw_options);
            assert_eq!(decoder.read_var_i32().unwrap(), state_id("minecraft:stone"));
            assert!(decoder.is_empty());
        }
    }
}

#[test]
fn living_fall_particles_root_tick_does_not_include_the_last_descent_in_particle_power() {
    for stand in [false, true] {
        let (mut store, mut level, id) = falling_living(stand, 3.8, false);
        let health = store.get(id).unwrap().living().unwrap().health();
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        let body = store.get(id).unwrap();
        assert!(body.on_ground());
        assert_eq!(body.position().y, 64.0);
        assert_eq!(
            body.living().unwrap().health(),
            health - if stand { 0.0 } else { 1.0 }
        );
        assert_eq!(body.fall_distance(), 0.0);
        assert!(particles(&store.drain_lifecycle_events()).is_empty());
    }
}

#[test]
fn living_fall_particles_root_tick_refreshes_water_before_reading_old_distance() {
    for stand in [false, true] {
        let (mut store, mut level, id) = falling_living(stand, 8.8, false);
        let body = store.get_mut(id).unwrap();
        body.set_pos(Vec3d {
            x: 0.5,
            y: 64.9,
            z: 0.5,
        });
        body.set_delta_movement(Vec3d {
            x: 0.0,
            y: -1.5,
            z: 0.0,
        });
        assert!(!body.is_in_water());
        level.world.set_block(
            DIMENSION,
            BlockPos { x: 0, y: 64, z: 0 },
            state_id("minecraft:water"),
            18,
        );
        let health = body.living().unwrap().health();
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        let body = store.get(id).unwrap();
        assert_eq!(body.position().y, 64.0);
        assert!(body.on_ground());
        assert!(body.is_in_water());
        assert_eq!(body.fall_distance(), 0.0);
        assert_eq!(body.living().unwrap().health(), health);
        assert!(particles(&store.drain_lifecycle_events()).is_empty());
    }
}

#[test]
fn living_fall_particles_root_tick_requires_ground_and_positive_old_distance() {
    for (distance, start_y, end_y, on_ground) in [(0.0, 1.25, 1.0, true), (8.8, 3.0, 2.5, false)] {
        let (mut store, mut level, id) = falling_living(false, distance, false);
        store.get_mut(id).unwrap().set_pos(Vec3d {
            x: 0.5,
            y: start_y + 63.0,
            z: 0.5,
        });
        let health = store.get(id).unwrap().living().unwrap().health();
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        let body = store.get(id).unwrap();
        assert_eq!(body.position().y, end_y + 63.0);
        assert_eq!(body.on_ground(), on_ground);
        assert_eq!(body.living().unwrap().health(), health);
        assert!(particles(&store.drain_lifecycle_events()).is_empty());
    }
}

#[test]
fn living_fall_particles_checked_mover_preserves_empty_flying_overrides() {
    for kind in [VANILLA_ENTITY_TYPE_PARROT_ID, VANILLA_ENTITY_TYPE_GHAST_ID] {
        let mut store = EntityStore::new(31, EntityIdCounter::new());
        let id = store
            .spawn_general_living(
                kind,
                Vec3d {
                    x: 0.5,
                    y: 64.25,
                    z: 0.5,
                },
            )
            .unwrap();
        let mut body = store.detach(id).unwrap();
        body.fall_distance = 8.8;
        let health = body.living().unwrap().health();
        let mut level = NativeLevel::new();
        for x in -3..=3 {
            for z in -3..=3 {
                level.world.set_block(
                    DIMENSION,
                    BlockPos { x, y: 63, z },
                    state_id("minecraft:stone"),
                    18,
                );
            }
        }
        store.drain_lifecycle_events();
        assert_eq!(
            level.with_level(|ctx| crate::entity::movement::move_living_entity(
                &mut body,
                &mut store,
                ctx,
                crate::entity::MoverType::Self_,
                Vec3d {
                    x: 0.0,
                    y: -0.5,
                    z: 0.0
                },
            )),
            Ok(())
        );
        assert!(body.on_ground(), "kind={kind}");
        assert_eq!(body.position().y, 64.0);
        assert_eq!(body.fall_distance(), 8.8);
        assert_eq!(body.living().unwrap().health(), health);
        assert!(particles(&store.drain_lifecycle_events()).is_empty());
    }
}

#[test]
fn living_fall_particles_nonliving_root_tick_emits_no_living_landing_particles() {
    let mut store = EntityStore::new(31, EntityIdCounter::new());
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: 0.5,
            y: 64.25,
            z: 0.5,
        },
        EntityData::Item(ItemEntityData::new(stack_of("minecraft:dirt", 1), 0.0)),
    );
    let body = store.get_mut(id).unwrap();
    body.fall_distance = 8.8;
    body.set_delta_movement(Vec3d {
        x: 0.0,
        y: -0.5,
        z: 0.0,
    });
    let mut level = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            level.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    store.drain_lifecycle_events();
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let body = store.get(id).unwrap();
    assert_eq!(body.position().y, 64.0);
    assert!(body.on_ground());
    assert_eq!(body.fall_distance(), 0.0);
    assert!(particles(&store.drain_lifecycle_events()).is_empty());
}
