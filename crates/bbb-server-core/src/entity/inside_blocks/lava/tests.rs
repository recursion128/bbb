use super::*;
use crate::entity::tests::native_level::NativeLevel;
use crate::entity::{EntityIdCounter, EntityLifecycleEvent};
use bbb_protocol::packets::{SoundSource, Vec3d};

fn item_fixture(resource: &str) -> (Box<Entity>, EntityStore) {
    let mut store = EntityStore::new(29, EntityIdCounter::new());
    let id = crate::entity::item::spawn_item_entity(
        &mut store,
        Vec3d {
            x: 8.5,
            y: 80.0,
            z: 8.5,
        },
        crate::entity::item::plain_stack(resource, 1).unwrap(),
    )
    .unwrap();
    let entity = store.detach(id).unwrap();
    store.drain_lifecycle_events();
    (entity, store)
}

#[test]
fn lava_item_damage_and_burn_sound_respect_health_cadence_and_silent() {
    for (tick, health, silent, audible) in [
        (9, 5, false, false),
        (10, 5, false, true),
        (9, 4, false, true),
        (10, 5, true, false),
    ] {
        let (mut body, mut store) = item_fixture("minecraft:stone");
        body.tick_count = tick;
        body.set_silent(silent);
        let EntityData::Item(item) = &mut body.data else {
            unreachable!()
        };
        item.set_health(health);
        let mut native = NativeLevel::new();
        assert!(native.with_level(|level| apply_lava_hurt(
            &mut body,
            &mut store,
            level,
            4.0,
            &mut crate::entity::damage::hurt_detached_from_environment
        )));
        let EntityData::Item(item) = &body.data else {
            unreachable!()
        };
        assert_eq!(item.health(), health - 4);
        let events = store.drain_lifecycle_events();
        let sounds: Vec<_> = events
            .iter()
            .filter_map(|event| {
                if let EntityLifecycleEvent::Sound { sound } = event {
                    Some(sound)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(!sounds.is_empty(), audible);
        for sound in sounds {
            assert_eq!(sound.source, SoundSource::Ambient);
            assert_eq!(sound.volume, 0.4);
            assert!((2.0..=2.4).contains(&sound.pitch));
        }
    }
}

#[test]
fn lava_immune_body_keeps_health_and_has_no_burn_sound() {
    let (_, mut store) = item_fixture("minecraft:netherite_sword");
    let id = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BLAZE_ID,
            Vec3d {
                x: 8.5,
                y: 80.0,
                z: 8.5,
            },
        )
        .unwrap();
    let mut immune = store.detach(id).unwrap();
    let health = immune.living().unwrap().health();
    store.drain_lifecycle_events();
    let mut native = NativeLevel::new();
    assert!(native.with_level(|level| apply_lava_hurt(
        &mut immune,
        &mut store,
        level,
        4.0,
        &mut crate::entity::damage::hurt_detached_from_environment
    )));
    assert_eq!(immune.living().unwrap().health(), health);
    assert!(!store
        .drain_lifecycle_events()
        .iter()
        .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })));
}

#[test]
fn lava_orb_is_hurt_and_audible_between_item_sound_ticks() {
    let mut store = EntityStore::new(29, EntityIdCounter::new());
    let id = crate::entity::experience_orb::spawn_experience_orb(
        &mut store,
        Vec3d {
            x: 8.5,
            y: 80.0,
            z: 8.5,
        },
        1,
    )
    .unwrap();
    let mut orb = store.detach(id).unwrap();
    orb.tick_count = 9;
    store.drain_lifecycle_events();
    let mut native = NativeLevel::new();
    assert!(native.with_level(|level| apply_lava_hurt(
        &mut orb,
        &mut store,
        level,
        4.0,
        &mut crate::entity::damage::hurt_detached_from_environment
    )));
    let EntityData::ExperienceOrb(data) = orb.data() else {
        unreachable!()
    };
    assert_eq!(data.health(), 1);
    assert!(store.drain_lifecycle_events().iter().any(|event| matches!(event, EntityLifecycleEvent::Sound { sound } if sound.source == SoundSource::Ambient)));
}
