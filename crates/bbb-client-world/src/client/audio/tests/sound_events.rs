use super::*;

#[test]
fn elder_guardian_game_event_sound_uses_listener_rounded_param() {
    let mut store = WorldStore::new();
    store.set_local_player_pose(crate::LocalPlayerPoseState::default());

    for (param, expected) in [
        (0.499, false),
        (0.5, true),
        (1.499, true),
        (1.5, false),
        (f32::NAN, false),
    ] {
        let sound = store.game_event_positioned_sound(ProtocolGameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID,
            param,
        });

        assert_eq!(sound.is_some(), expected, "param={param:?}");
    }
}

#[test]
fn game_event_sound_mapping_covers_every_vanilla_event_type() {
    let mut store = WorldStore::new();
    store.set_local_player_pose(crate::LocalPlayerPoseState::default());

    for event in VANILLA_CLIENT_GAME_EVENT_TYPES {
        let sound = store.game_event_positioned_sound(ProtocolGameEvent {
            event_id: event.id,
            param: 1.0,
        });
        let expected = match event.constant_name {
            "PLAY_ARROW_HIT_SOUND" => Some(ARROW_HIT_PLAYER_SOUND_EVENT),
            "PUFFER_FISH_STING" => Some(PUFFER_FISH_STING_SOUND_EVENT),
            "GUARDIAN_ELDER_EFFECT" => Some(ELDER_GUARDIAN_CURSE_SOUND_EVENT),
            _ => None,
        };

        assert_eq!(
            sound
                .as_ref()
                .and_then(|state| state.sound.location.as_deref()),
            expected,
            "game event {} ({})",
            event.id,
            event.constant_name
        );
    }
}

#[test]
fn record_positioned_sound_updates_last_sound_without_counting_sound_packet() {
    let mut store = WorldStore::new();

    let sound = SoundEventState {
        sound: SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some("minecraft:block.dispenser.fail".to_string()),
            fixed_range: None,
        },
        source: "block".to_string(),
        position: ProtocolVec3d {
            x: 1.5,
            y: 2.5,
            z: -3.5,
        },
        volume: 1.0,
        pitch: 1.2,
        seed: 0,
        distance_delay: false,
    };

    let recorded = store.record_positioned_sound(sound);

    assert_eq!(store.last_sound(), Some(&recorded));
    assert_eq!(store.counters().sound_packets, 0);
}

#[test]
fn unknown_jukebox_holder_does_not_replace_an_existing_song_or_overlay() {
    let mut store = WorldStore::new();
    let pos = ProtocolBlockPos { x: 4, y: 65, z: -7 };
    store.apply_level_event(LevelEvent {
        event_type: 1010,
        pos,
        data: 1,
        global: false,
    });
    store.take_pending_chat_narrations();
    let action_bar = store.action_bar().cloned();
    let last_event = *store.last_jukebox_event().unwrap();

    assert_eq!(
        store.apply_level_event(LevelEvent {
            event_type: 1010,
            pos,
            data: 27,
            global: false,
        }),
        None
    );
    assert_eq!(
        store.playing_jukebox_songs(),
        &[JukeboxSongState {
            pos: crate::BlockPos { x: 4, y: 65, z: -7 },
            song_registry_id: 1,
        }]
    );
    assert_eq!(store.last_jukebox_event(), Some(&last_event));
    assert_eq!(store.action_bar(), action_bar.as_ref());
    assert!(store.take_pending_chat_narrations().is_empty());
}
