use super::*;

fn ambient_plays(store: &mut EntityStore, resource_id: &str) -> Vec<f32> {
    let registry_id =
        bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(resource_id).unwrap();
    store
        .drain_lifecycle_events()
        .into_iter()
        .filter_map(|event| match event {
            EntityLifecycleEvent::Sound { sound }
                if sound.sound == SoundEventHolder::Reference { registry_id } =>
            {
                Some(sound.volume)
            }
            _ => None,
        })
        .collect()
}

/// `Mob.baseTick` (Mob.java:282-285): every mob plays its ambient sound on
/// its own clock, at most once per `getAmbientSoundInterval`, and a silent
/// one never does.
#[test]
fn mobs_play_their_ambient_sound_periodically_unless_silent() {
    for (type_id, sound, volume, interval) in [
        (
            // A zombie would burn in this daylight, and each hurt restarts the clock.
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SPIDER_ID,
            "minecraft:entity.spider.ambient",
            1.0,
            80,
        ),
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
            "minecraft:entity.cow.ambient",
            0.4,
            120,
        ),
    ] {
        for silent in [false, true] {
            let mut store = EntityStore::new(0xA3B1_0001, EntityIdCounter::new());
            let id = store.spawn_general_living(type_id, position()).unwrap();
            let entity = store.get_mut(id).unwrap();
            entity.set_no_gravity(true);
            entity.set_silent(silent);
            store.drain_lifecycle_events();
            let mut native = NativeLevel::new();
            let mut played = Vec::new();
            for tick in 0..interval * 4 {
                native.with_level(|level| tick_entities(&mut store, level, true));
                let plays = ambient_plays(&mut store, sound);
                if !plays.is_empty() {
                    if let Some(previous) = played.last() {
                        assert!(
                            tick - previous > interval,
                            "{sound} at {previous} and {tick}"
                        );
                    }
                    played.push(tick);
                }
                assert!(plays.iter().all(|played| *played == volume), "{sound}");
            }
            if silent {
                assert!(played.is_empty(), "{sound} played while silent");
            } else {
                assert!(played.len() >= 2, "{sound} played {played:?}");
            }
        }
    }
}
