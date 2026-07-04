use super::*;

#[test]
fn client_audio_events_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::Sound(SoundEvent {
        sound: SoundEventHolder::Reference { registry_id: 41 },
        source: SoundSource::Blocks,
        position: ProtocolVec3d {
            x: 2.5,
            y: -1.0,
            z: 0.0,
        },
        volume: 0.75,
        pitch: 1.25,
        seed: 123456789,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SoundEntity(
        SoundEntityEvent {
            sound: SoundEventHolder::Direct {
                location: "minecraft:entity.cat.ambient".to_string(),
                fixed_range: Some(32.0),
            },
            source: SoundSource::Neutral,
            entity_id: 123,
            volume: 1.0,
            pitch: 0.5,
            seed: -9,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::StopSound(StopSound {
        source: Some(SoundSource::Music),
        name: Some("minecraft:music.menu".to_string()),
    })))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(123));
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );
    assert_eq!(world.counters().sound_packets, 1);
    assert_eq!(
        world.last_sound(),
        Some(&bbb_world::SoundEventState {
            sound: bbb_world::SoundHolderState {
                kind: "reference".to_string(),
                registry_id: Some(41),
                location: None,
                fixed_range: None,
            },
            source: "block".to_string(),
            position: ProtocolVec3d {
                x: 2.5,
                y: -1.0,
                z: 0.0,
            },
            volume: 0.75,
            pitch: 1.25,
            seed: 123456789,
            distance_delay: false,
        })
    );
    assert_eq!(world.counters().sound_entity_packets, 1);
    assert_eq!(world.counters().sound_entity_events_applied, 1);
    assert_eq!(world.counters().sound_entity_events_ignored, 0);
    assert_eq!(
        world.last_sound_entity(),
        Some(&bbb_world::SoundEntityEventState {
            sound: bbb_world::SoundHolderState {
                kind: "direct".to_string(),
                registry_id: None,
                location: Some("minecraft:entity.cat.ambient".to_string()),
                fixed_range: Some(32.0),
            },
            source: "neutral".to_string(),
            entity_id: 123,
            volume: 1.0,
            pitch: 0.5,
            seed: -9,
        })
    );
    assert_eq!(world.counters().stop_sound_packets, 1);
    assert_eq!(
        world.last_stop_sound(),
        Some(&bbb_world::StopSoundEventState {
            source: Some("music".to_string()),
            name: Some("minecraft:music.menu".to_string()),
        })
    );
}

#[test]
fn client_audio_events_emit_runtime_commands_for_applied_events() {
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::Sound(SoundEvent {
        sound: SoundEventHolder::Reference { registry_id: 0 },
        source: SoundSource::Ambient,
        position: ProtocolVec3d {
            x: 2.5,
            y: -1.0,
            z: 0.0,
        },
        volume: 0.75,
        pitch: 1.25,
        seed: 123456789,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SoundEntity(
        SoundEntityEvent {
            sound: SoundEventHolder::Direct {
                location: "minecraft:entity.cat.ambient".to_string(),
                fixed_range: Some(32.0),
            },
            source: SoundSource::Neutral,
            entity_id: 123,
            volume: 1.0,
            pitch: 0.5,
            seed: -9,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SoundEntity(
        SoundEntityEvent {
            sound: SoundEventHolder::Direct {
                location: "minecraft:entity.cat.ambient".to_string(),
                fixed_range: None,
            },
            source: SoundSource::Neutral,
            entity_id: 404,
            volume: 0.2,
            pitch: 1.8,
            seed: 7,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::StopSound(StopSound {
        source: Some(SoundSource::Music),
        name: Some("minecraft:music.menu".to_string()),
    })))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(123));
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(
        test_sound_catalog(),
        SoundEventRegistry::from_ids(["minecraft:ambient.cave"]),
    );

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        4
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 3);
    let world_counters = world.counters();
    assert_eq!(world_counters.sound_packets, 1);
    assert_eq!(world_counters.sound_entity_packets, 2);
    assert_eq!(world_counters.sound_entity_events_applied, 1);
    assert_eq!(world_counters.sound_entity_events_ignored, 1);
    assert_eq!(world_counters.stop_sound_packets, 1);

    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Ambient);
            assert_eq!(command.position, [2.5, -1.0, 0.0]);
            assert_eq!(command.packet_volume, 0.75);
            assert_eq!(command.packet_pitch, 1.25);
            assert_eq!(command.seed, 123456789);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:ambient.cave");
            assert_eq!(command.sound.sound_name, "minecraft:ambient/cave/cave1");
        }
        other => panic!("expected positioned sound command, got {other:?}"),
    }
    match &audio.commands[1] {
        AudioCommand::PlayEntitySound(command) => {
            assert_eq!(command.category, AudioCategory::Neutral);
            assert_eq!(command.entity_id, 123);
            assert_eq!(command.position, Some([1.0, 64.0, -2.0]));
            assert_eq!(command.packet_volume, 1.0);
            assert_eq!(command.packet_pitch, 0.5);
            assert_eq!(command.seed, -9);
            assert_eq!(command.fixed_range, Some(32.0));
            assert_eq!(command.sound.event_id, "minecraft:entity.cat.ambient");
            assert_eq!(command.sound.sound_name, "minecraft:mob/cat/meow1");
        }
        other => panic!("expected entity sound command, got {other:?}"),
    }
    match &audio.commands[2] {
        AudioCommand::StopSound(command) => {
            assert_eq!(command.category, Some(AudioCategory::Music));
            assert_eq!(command.name.as_deref(), Some("minecraft:music.menu"));
        }
        other => panic!("expected stop sound command, got {other:?}"),
    }
}

#[test]
fn attack_entity_events_emit_positioned_audio_commands() {
    let (tx, mut rx) = mpsc::channel(7);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(140, VANILLA_ENTITY_TYPE_RAVAGER_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(141, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(142, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 140,
        event_id: 4,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 141,
        event_id: 4,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 142,
        event_id: 4,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 404,
        event_id: 4,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        7
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 2);
    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Hostile);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert_eq!(command.packet_pitch, 1.0);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:entity.ravager.attack");
        }
        other => panic!("expected ravager positioned sound command, got {other:?}"),
    }
    match &audio.commands[1] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Neutral);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert_eq!(command.packet_pitch, 1.0);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:entity.iron_golem.attack");
        }
        other => panic!("expected iron golem positioned sound command, got {other:?}"),
    }
    assert_eq!(world.counters().entity_events_applied, 3);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn evoker_fangs_attack_event_emits_positioned_audio_command() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(143, VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 143,
        event_id: 4,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_pitch = expected_random.next_float() * 0.2 + 0.85;

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Neutral);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert!((command.packet_pitch - expected_pitch).abs() < 1.0e-6);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(
                command.sound.event_id,
                "minecraft:entity.evoker_fangs.attack"
            );
        }
        other => panic!("expected evoker fangs positioned sound command, got {other:?}"),
    }
    assert_eq!(world.counters().entity_events_applied, 1);
    assert_eq!(world.counters().entity_events_ignored, 0);
}

#[test]
fn zombie_villager_cure_event_emits_eye_positioned_audio_command() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(144, VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 144,
        event_id: 16,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_volume = 1.0 + expected_random.next_float();
    let expected_pitch = expected_random.next_float() * 0.7 + 0.3;

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Hostile);
            assert_eq!(command.position[0], 1.0);
            assert!((command.position[1] - 65.74).abs() < 1.0e-6);
            assert_eq!(command.position[2], -2.0);
            assert!((command.packet_volume - expected_volume).abs() < 1.0e-6);
            assert!((command.packet_pitch - expected_pitch).abs() < 1.0e-6);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(
                command.sound.event_id,
                "minecraft:entity.zombie_villager.cure"
            );
        }
        other => panic!("expected zombie villager cure positioned sound command, got {other:?}"),
    }
    assert_eq!(world.counters().entity_events_applied, 1);
    assert_eq!(world.counters().entity_events_ignored, 0);
}

#[test]
fn armadillo_peek_event_emits_positioned_audio_command() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(145, VANILLA_ENTITY_TYPE_ARMADILLO_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 145,
        event_id: 64,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Neutral);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert_eq!(command.packet_pitch, 1.0);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:entity.armadillo.peek");
        }
        other => panic!("expected armadillo peek positioned sound command, got {other:?}"),
    }
    assert_eq!(world.counters().entity_events_applied, 1);
    assert_eq!(world.counters().entity_events_ignored, 0);
}

#[test]
fn armor_stand_hit_event_emits_positioned_audio_command() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(146, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 146,
        event_id: 32,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Neutral);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 0.3);
            assert_eq!(command.packet_pitch, 1.0);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:entity.armor_stand.hit");
        }
        other => panic!("expected armor stand hit positioned sound command, got {other:?}"),
    }
    assert_eq!(world.counters().entity_events_applied, 1);
    assert_eq!(world.counters().entity_events_ignored, 0);
}

#[test]
fn armor_stand_death_event_emits_positioned_audio_command() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(147, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 147,
        event_id: 3,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_pitch = (expected_random.next_float() - expected_random.next_float()) * 0.2 + 1.0;

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Neutral);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert!((command.packet_pitch - expected_pitch).abs() < 1.0e-6);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:entity.armor_stand.break");
        }
        other => panic!("expected armor stand death positioned sound command, got {other:?}"),
    }
    assert_eq!(world.counters().entity_events_applied, 1);
    assert_eq!(world.counters().entity_events_ignored, 0);
}

#[test]
fn zombie_death_event_emits_positioned_audio_command() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(148, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 148,
        event_id: 3,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_pitch = (expected_random.next_float() - expected_random.next_float()) * 0.2 + 1.0;

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Hostile);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert!((command.packet_pitch - expected_pitch).abs() < 1.0e-6);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:entity.zombie.death");
        }
        other => panic!("expected zombie death positioned sound command, got {other:?}"),
    }
    assert_eq!(world.counters().entity_events_applied, 1);
    assert_eq!(world.counters().entity_events_ignored, 0);
}

#[test]
fn zombie_villager_death_event_emits_positioned_audio_command() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(149, VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 149,
        event_id: 3,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_pitch = (expected_random.next_float() - expected_random.next_float()) * 0.2 + 1.0;

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Hostile);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert!((command.packet_pitch - expected_pitch).abs() < 1.0e-6);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(
                command.sound.event_id,
                "minecraft:entity.zombie_villager.death"
            );
        }
        other => panic!("expected zombie villager death positioned sound command, got {other:?}"),
    }
    assert_eq!(world.counters().entity_events_applied, 1);
    assert_eq!(world.counters().entity_events_ignored, 0);
}

#[test]
fn attack_entity_death_events_emit_positioned_audio_commands() {
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(150, VANILLA_ENTITY_TYPE_RAVAGER_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 150,
        event_id: 3,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(151, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 151,
        event_id: 3,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_ravager_pitch =
        (expected_random.next_float() - expected_random.next_float()) * 0.2 + 1.0;
    let expected_golem_pitch =
        (expected_random.next_float() - expected_random.next_float()) * 0.2 + 1.0;

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        4
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 2);
    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Hostile);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert!((command.packet_pitch - expected_ravager_pitch).abs() < 1.0e-6);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:entity.ravager.death");
        }
        other => panic!("expected ravager death positioned sound command, got {other:?}"),
    }
    match &audio.commands[1] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Neutral);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert!((command.packet_pitch - expected_golem_pitch).abs() < 1.0e-6);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:entity.iron_golem.death");
        }
        other => panic!("expected iron golem death positioned sound command, got {other:?}"),
    }
    assert_eq!(world.counters().entity_events_applied, 2);
    assert_eq!(world.counters().entity_events_ignored, 0);
}

#[test]
fn villager_and_witch_death_events_emit_positioned_audio_commands() {
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(152, VANILLA_ENTITY_TYPE_WITCH_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 152,
        event_id: 3,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(153, VANILLA_ENTITY_TYPE_VILLAGER_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 153,
        event_id: 3,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_witch_pitch =
        (expected_random.next_float() - expected_random.next_float()) * 0.2 + 1.0;
    let expected_villager_pitch =
        (expected_random.next_float() - expected_random.next_float()) * 0.2 + 1.0;

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        4
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 2);
    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Hostile);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert!((command.packet_pitch - expected_witch_pitch).abs() < 1.0e-6);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:entity.witch.death");
        }
        other => panic!("expected witch death positioned sound command, got {other:?}"),
    }
    match &audio.commands[1] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Neutral);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert!((command.packet_pitch - expected_villager_pitch).abs() < 1.0e-6);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:entity.villager.death");
        }
        other => panic!("expected villager death positioned sound command, got {other:?}"),
    }
    assert_eq!(world.counters().entity_events_applied, 2);
    assert_eq!(world.counters().entity_events_ignored, 0);
}

#[test]
fn skeleton_family_death_events_emit_positioned_audio_commands() {
    let (tx, mut rx) = mpsc::channel(6);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(154, VANILLA_ENTITY_TYPE_SKELETON_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 154,
        event_id: 3,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(155, VANILLA_ENTITY_TYPE_STRAY_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 155,
        event_id: 3,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(156, VANILLA_ENTITY_TYPE_BOGGED_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 156,
        event_id: 3,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_skeleton_pitch =
        (expected_random.next_float() - expected_random.next_float()) * 0.2 + 1.0;
    let expected_stray_pitch =
        (expected_random.next_float() - expected_random.next_float()) * 0.2 + 1.0;
    let expected_bogged_pitch =
        (expected_random.next_float() - expected_random.next_float()) * 0.2 + 1.0;

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        6
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 3);
    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Hostile);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert!((command.packet_pitch - expected_skeleton_pitch).abs() < 1.0e-6);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:entity.skeleton.death");
        }
        other => panic!("expected skeleton death positioned sound command, got {other:?}"),
    }
    match &audio.commands[1] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Hostile);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert!((command.packet_pitch - expected_stray_pitch).abs() < 1.0e-6);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:entity.stray.death");
        }
        other => panic!("expected stray death positioned sound command, got {other:?}"),
    }
    match &audio.commands[2] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Hostile);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert!((command.packet_pitch - expected_bogged_pitch).abs() < 1.0e-6);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:entity.bogged.death");
        }
        other => panic!("expected bogged death positioned sound command, got {other:?}"),
    }
    assert_eq!(world.counters().entity_events_applied, 3);
    assert_eq!(world.counters().entity_events_ignored, 0);
}

#[test]
fn sound_event_registry_data_updates_audio_reference_resolution() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::RegistryData(RegistryData {
        registry: "minecraft:sound_event".to_string(),
        raw_payload_len: 64,
        entries: vec![RegistryDataEntry {
            id: "minecraft:ambient.cave".to_string(),
            raw_data: None,
        }],
    }))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::Sound(SoundEvent {
        sound: SoundEventHolder::Reference { registry_id: 0 },
        source: SoundSource::Ambient,
        position: ProtocolVec3d {
            x: 2.5,
            y: -1.0,
            z: 0.0,
        },
        volume: 0.75,
        pitch: 1.25,
        seed: 123456789,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(world.counters().registries_seen, 1);
    assert_eq!(
        world
            .registry_content("minecraft:sound_event")
            .unwrap()
            .entries[0]
            .id,
        "minecraft:ambient.cave"
    );
    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.sound.event_id, "minecraft:ambient.cave");
            assert_eq!(command.sound.sound_name, "minecraft:ambient/cave/cave1");
        }
        other => panic!("expected positioned sound command, got {other:?}"),
    }
}

#[test]
fn silent_entity_sound_events_do_not_emit_runtime_commands() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        SetEntityData {
            id: 123,
            values: vec![EntityDataValue {
                data_id: 4,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SoundEntity(
        SoundEntityEvent {
            sound: SoundEventHolder::Direct {
                location: "minecraft:entity.cat.ambient".to_string(),
                fixed_range: Some(32.0),
            },
            source: SoundSource::Neutral,
            entity_id: 123,
            volume: 1.0,
            pitch: 0.5,
            seed: -9,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(123));
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert!(audio.commands.is_empty(), "{:?}", audio.commands);
    assert_eq!(world.last_sound_entity(), None);
    assert_eq!(world.counters().sound_entity_packets, 1);
    assert_eq!(world.counters().sound_entity_events_applied, 0);
    assert_eq!(world.counters().sound_entity_events_ignored, 1);
}
