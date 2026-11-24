use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use crate::entity::{ProjectileState, ShulkerBulletData};
use bbb_protocol::packets::{SoundEventHolder, SoundSource};

fn detached_bullet() -> (EntityStore, Box<Entity>) {
    let mut store = EntityStore::new(37, EntityIdCounter::new());
    let id = store.spawn(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
        position(),
        EntityData::ShulkerBullet(ShulkerBulletData::from_saved(
            ProjectileState::new(None),
            None,
            None,
            0,
            Vec3d::default(),
        )),
    );
    store.drain_lifecycle_events();
    let entity = store.detach(id).unwrap();
    (store, entity)
}

#[test]
fn shulker_environment_hurt_discards_body_and_publishes_sound_particles_and_event() {
    let (mut store, mut body) = detached_bullet();
    assert!(supports_detached_environment_hurt(body.data()));
    body.set_invulnerable(true);
    let id = body.id();
    let mut level = NativeLevel::new();
    assert_eq!(
        level.with_level(|level| hurt_detached_from_environment(
            &mut body,
            &mut store,
            level,
            "minecraft:cactus",
            0.0
        )),
        Some(true)
    );
    assert_eq!(body.removal_reason(), Some(RemovalReason::Discarded));
    assert!(store.get(id).is_none());
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DAMAGE
                && event.recorded.pos == BlockPos { x: 1, y: 64, z: -3 }
                && event.recorded.source_entity_id == Some(id)
        ));
    let events = store.drain_lifecycle_events();
    let sound = events
        .iter()
        .find_map(|event| {
            if let EntityLifecycleEvent::Sound { sound } = event {
                Some(sound)
            } else {
                None
            }
        })
        .expect("hurt sound");
    let particles = events
        .iter()
        .find_map(|event| {
            if let EntityLifecycleEvent::Particles { particles } = event {
                Some(particles)
            } else {
                None
            }
        })
        .expect("hurt particles");
    assert_eq!(
        sound.sound,
        SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:entity.shulker_bullet.hurt"
            )
            .unwrap()
        }
    );
    assert_eq!(sound.source, SoundSource::Hostile);
    assert_eq!(sound.position, position());
    assert_eq!((sound.volume, sound.pitch), (1.0, 1.0));
    assert_eq!(particles.position, position());
    assert_eq!(
        particles.particle.particle_type_id,
        bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_CRIT_ID
    );
    assert!(particles.particle.raw_options.is_empty());
    assert_eq!(particles.count, 15);
    assert_eq!(
        particles.offset,
        Vec3d {
            x: 0.2,
            y: 0.2,
            z: 0.2
        }
    );
    assert_eq!(particles.max_speed, 0.0);
    assert!(!particles.override_limiter && !particles.always_show);
    store.attach(body);
    assert!(store.drain_lifecycle_events().iter().any(
        |event| matches!(event, EntityLifecycleEvent::Removed { entity, .. } if entity.id() == id)
    ));
}

#[test]
fn shulker_environment_silent_keeps_particles_discard_and_event() {
    let (mut store, mut body) = detached_bullet();
    body.set_silent(true);
    let mut level = NativeLevel::new();
    assert_eq!(
        level.with_level(|level| hurt_detached_from_environment(
            &mut body,
            &mut store,
            level,
            "minecraft:lava",
            4.0
        )),
        Some(true)
    );
    assert_eq!(body.removal_reason(), Some(RemovalReason::Discarded));
    let events = store.drain_lifecycle_events();
    assert!(events.iter().any(|event| matches!(event, EntityLifecycleEvent::Particles { particles } if particles.count == 15)));
    assert!(!events
        .iter()
        .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })));
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DAMAGE
                && event.recorded.source_entity_id == Some(body.id())
        ));
}
