use super::*;

#[test]
fn add_entity_starts_minecart_and_bee_tickable_sound_instances() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(190, VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(191, VANILLA_ENTITY_TYPE_BEE_ID),
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 2);
    for (command, entity_id, event_id) in [
        (&audio.commands[0], 190, "minecraft:entity.minecart.riding"),
        (&audio.commands[1], 191, "minecraft:entity.bee.loop"),
    ] {
        let AudioCommand::PlayEntitySound(command) = command else {
            panic!("expected post-add entity sound, got {command:?}");
        };
        assert_eq!(command.entity_id, entity_id);
        assert_eq!(command.position, Some([1.0, 64.0, -2.0]));
        assert_eq!(command.packet_volume, 0.0);
        assert_eq!(command.packet_pitch, 0.0);
        assert_eq!(command.sound.event_id, event_id);
        assert_eq!(command.category, AudioCategory::Neutral);
        assert!(command.looping);
        assert!(command.linear_attenuation);
        assert!(command.tickable);
    }
    assert_eq!(world.tickable_entity_sounds().len(), 2);
}

#[test]
fn respawn_stops_music_only_when_the_dimension_changes() {
    let (tx, mut rx) = mpsc::channel(2);
    let login = protocol_play_login(9);
    tx.try_send(NetEvent::Play(PlayClientbound::Respawn(Respawn {
        common_spawn_info: login.common_spawn_info.clone(),
        data_to_keep: 0,
    })))
    .unwrap();
    let mut changed_spawn = login.common_spawn_info.clone();
    changed_spawn.dimension_type_id = 1;
    changed_spawn.dimension = "minecraft:the_nether".to_string();
    tx.try_send(NetEvent::Play(PlayClientbound::Respawn(Respawn {
        common_spawn_info: changed_spawn,
        data_to_keep: 0,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_login(&login);
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );
    assert_eq!(
        audio.commands,
        vec![AudioCommand::StopSound(bbb_audio::StopSoundCommand {
            category: Some(AudioCategory::Music),
            name: None,
        })]
    );
}

#[test]
fn damage_type_holder_drives_runtime_player_hurt_audio() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::DamageEvent(
        bbb_protocol::packets::DamageEvent {
            entity_id: 166,
            source_type_id: 0,
            source_cause_id: -1,
            source_direct_id: -1,
            source_position: None,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_login(&protocol_play_login(166));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        ..LocalPlayerPoseState::default()
    });
    world.record_registry_entries(
        bbb_world::DAMAGE_TYPE_REGISTRY,
        0,
        vec![RegistryPacketEntry::stub("minecraft:freeze")],
    );
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        1
    );
    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    assert_eq!(
        world.damage_type_effects_by_holder_id(0).unwrap(),
        Some(bbb_world::DamageTypeEffects::Freezing)
    );
    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Players);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert_eq!(command.seed, 0);
            assert_eq!(
                command.sound.event_id,
                "minecraft:entity.player.hurt_freeze"
            );
            assert_eq!(command.sound.sound_name, "minecraft:damage/freeze");
        }
        other => panic!("expected positioned sound command, got {other:?}"),
    }
}

#[test]
fn remote_damage_and_server_sound_entity_emit_one_runtime_command() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(167, VANILLA_ENTITY_TYPE_COW_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DamageEvent(
        bbb_protocol::packets::DamageEvent {
            entity_id: 167,
            source_type_id: 0,
            source_cause_id: -1,
            source_direct_id: -1,
            source_position: None,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SoundEntity(
        SoundEntityEvent {
            sound: SoundEventHolder::Direct {
                location: "minecraft:entity.cow.hurt".to_string(),
                fixed_range: None,
            },
            source: SoundSource::Neutral,
            entity_id: 167,
            volume: 0.4,
            pitch: 1.25,
            seed: 91,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    world.record_registry_entries(
        bbb_world::DAMAGE_TYPE_REGISTRY,
        0,
        vec![RegistryPacketEntry::stub("minecraft:generic")],
    );
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        3
    );
    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    assert_eq!(
        world.entity_last_damage(167).unwrap().source_effects,
        Some(bbb_world::DamageTypeEffects::Hurt)
    );
    match &audio.commands[0] {
        AudioCommand::PlayEntitySound(command) => {
            assert_eq!(command.category, AudioCategory::Neutral);
            assert_eq!(command.packet_volume, 0.4);
            assert_eq!(command.packet_pitch, 1.25);
            assert_eq!(command.seed, 91);
            assert_eq!(command.sound.event_id, "minecraft:entity.cow.hurt");
            assert_eq!(command.sound.sound_name, "minecraft:mob/cow/hurt1");
        }
        other => panic!("expected entity sound command, got {other:?}"),
    }
}

#[test]
fn remote_damage_event_mutates_living_state_without_client_owned_audio() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(168, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DamageEvent(
        bbb_protocol::packets::DamageEvent {
            entity_id: 168,
            source_type_id: 77,
            source_cause_id: -1,
            source_direct_id: -1,
            source_position: None,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    let damage = world.entity_last_damage(168).unwrap();
    assert_eq!(damage.source_type_id, 77);
    assert_eq!(damage.source_effects, None);
    assert_eq!(world.entity_damage_invulnerable_time(168), Some(20));
    let animations = world.probe_entity(168).unwrap().client_animations;
    assert_eq!(animations.walk_animation.unwrap().speed, 1.5);
    assert!(animations.has_red_overlay());

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert!(audio.commands.is_empty());
}

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
            looping: false,
            linear_attenuation: true,
            tickable: false,
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
fn listener_special_entity_events_emit_tickable_audio_commands() {
    let (tx, mut rx) = mpsc::channel(4);
    for packet in [
        PlayClientbound::AddEntity(protocol_add_entity_with_type(
            170,
            VANILLA_ENTITY_TYPE_GUARDIAN_ID,
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 170,
            event_id: 21,
        }),
        PlayClientbound::AddEntity(protocol_add_entity_with_type(
            171,
            VANILLA_ENTITY_TYPE_SNIFFER_ID,
        )),
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 171,
            event_id: 63,
        }),
    ] {
        tx.try_send(NetEvent::Play(packet)).unwrap();
    }

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        4
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 2);
    let AudioCommand::PlayEntitySound(guardian) = &audio.commands[0] else {
        panic!("expected guardian entity sound command")
    };
    assert_eq!(guardian.sound.event_id, "minecraft:entity.guardian.attack");
    assert_eq!(guardian.category, AudioCategory::Hostile);
    assert_eq!(guardian.entity_id, 170);
    assert_eq!(guardian.position, Some([1.0, 64.0, -2.0]));
    assert!(guardian.looping);
    assert!(!guardian.linear_attenuation);
    assert!(guardian.tickable);
    assert_eq!(guardian.packet_volume, 0.0);
    assert_eq!(guardian.packet_pitch, 0.7);

    let AudioCommand::PlayEntitySound(sniffer) = &audio.commands[1] else {
        panic!("expected sniffer entity sound command")
    };
    assert_eq!(sniffer.sound.event_id, "minecraft:entity.sniffer.digging");
    assert_eq!(sniffer.category, AudioCategory::Neutral);
    assert_eq!(sniffer.entity_id, 171);
    assert!(!sniffer.looping);
    assert!(sniffer.linear_attenuation);
    assert!(sniffer.tickable);
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
fn remote_armor_stand_death_event_waits_for_server_owned_sound_packet() {
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

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert!(audio.commands.is_empty());
    assert!(world.probe_entity(147).unwrap().living_dead);
    assert_eq!(world.counters().entity_events_applied, 1);
    assert_eq!(world.counters().entity_events_ignored, 0);
}

#[test]
fn generated_remote_living_death_events_emit_no_positioned_audio_commands() {
    let mut probe = WorldStore::new();
    let mut event_ids = Vec::new();
    let mut catalog_entries = serde_json::Map::new();
    for entry in bbb_protocol::entity_types::VANILLA_ENTITY_TYPES {
        if matches!(
            bbb_world::vanilla_entity_metadata_type_death_sound_policy(entry.id),
            Some(bbb_world::VanillaEntityDeathSoundPolicy::NonLiving)
        ) {
            continue;
        }
        let entity_id = 40_000 + entry.id;
        probe.apply_add_entity(protocol_add_entity_with_type(entity_id, entry.id));
        let sound = probe
            .living_death_sound_for_entity(entity_id, || 0.5)
            .unwrap_or_else(|| panic!("{} death sound did not resolve", entry.resource_id));
        let event_id = sound
            .sound
            .location
            .unwrap_or_else(|| panic!("{} death sound was not direct", entry.resource_id));
        let position = [sound.position.x, sound.position.y, sound.position.z];
        let catalog_id = event_id
            .strip_prefix("minecraft:")
            .unwrap_or_else(|| panic!("unexpected non-vanilla death sound {event_id}"));
        catalog_entries.insert(
            catalog_id.to_string(),
            serde_json::json!({"sounds": ["test/generated_death"]}),
        );
        event_ids.push((entity_id, entry.id, event_id, position));
    }
    // Bee recreation starts a separate tickable loop before its death event.
    catalog_entries.insert(
        "entity.bee.loop".to_string(),
        serde_json::json!({"sounds": ["test/bee_loop"]}),
    );
    let catalog_json = serde_json::to_vec(&serde_json::Value::Object(catalog_entries)).unwrap();
    let assets_dir = std::env::temp_dir().join("bbb-native-generated-death-audio-test-assets");
    let catalog = SoundCatalog::from_json_bytes("minecraft", &assets_dir, &catalog_json).unwrap();

    let (tx, mut rx) = mpsc::channel(event_ids.len() * 2);
    let mut world = WorldStore::new();
    for (entity_id, entity_type_id, _, _) in &event_ids {
        let packet = protocol_add_entity_with_type(*entity_id, *entity_type_id);
        if *entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
            record_test_player_info(&mut world, packet.uuid);
        }
        tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(packet)))
            .unwrap();
        tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
            entity_id: *entity_id,
            event_id: 3,
        })))
        .unwrap();
    }

    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(catalog, SoundEventRegistry::default());
    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        event_ids.len() * 2
    );
    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    let positioned: Vec<_> = audio
        .commands
        .iter()
        .filter_map(|command| match command {
            AudioCommand::PlayPositionedSound(command) => Some(command),
            _ => None,
        })
        .collect();
    assert!(positioned.is_empty());
    assert_eq!(event_ids.len(), 92);
    for (entity_id, entity_type_id, _, _) in &event_ids {
        let entity = world.probe_entity(*entity_id).unwrap();
        assert_eq!(
            entity.living_dead,
            *entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID,
            "entity type {entity_type_id}"
        );
    }
}

#[test]
fn local_player_death_event_emits_player_death_audio_command() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 157,
        event_id: 3,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_login(&protocol_play_login(157));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        ..LocalPlayerPoseState::default()
    });
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        1
    );
    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Players);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:entity.player.death");
        }
        other => panic!("expected player death positioned sound command, got {other:?}"),
    }
}

#[test]
fn remote_zombie_death_and_server_sound_entity_emit_one_runtime_command() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(148, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 148,
        event_id: 3,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SoundEntity(
        SoundEntityEvent {
            sound: SoundEventHolder::Direct {
                location: "minecraft:entity.zombie.death".to_string(),
                fixed_range: None,
            },
            source: SoundSource::Hostile,
            entity_id: 148,
            volume: 1.0,
            pitch: 0.75,
            seed: 92,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        3
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    match &audio.commands[0] {
        AudioCommand::PlayEntitySound(command) => {
            assert_eq!(command.category, AudioCategory::Hostile);
            assert_eq!(command.entity_id, 148);
            assert_eq!(command.packet_volume, 1.0);
            assert_eq!(command.packet_pitch, 0.75);
            assert_eq!(command.seed, 92);
            assert_eq!(command.sound.event_id, "minecraft:entity.zombie.death");
        }
        other => panic!("expected entity sound command, got {other:?}"),
    }
    assert!(world.probe_entity(148).unwrap().living_dead);
    assert_eq!(world.counters().entity_events_applied, 1);
    assert_eq!(world.counters().entity_events_ignored, 0);
}

#[test]
fn remote_zombie_villager_death_event_waits_for_server_owned_sound_packet() {
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

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert!(audio.commands.is_empty());
    assert!(world.probe_entity(149).unwrap().living_dead);
    assert_eq!(world.counters().entity_events_applied, 1);
    assert_eq!(world.counters().entity_events_ignored, 0);
}

#[test]
fn remote_attack_entity_death_events_wait_for_server_owned_sound_packets() {
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

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        4
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert!(audio.commands.is_empty());
    assert!(world.probe_entity(150).unwrap().living_dead);
    assert!(world.probe_entity(151).unwrap().living_dead);
    assert_eq!(world.counters().entity_events_applied, 2);
    assert_eq!(world.counters().entity_events_ignored, 0);
}

#[test]
fn remote_villager_and_witch_death_events_wait_for_server_owned_sound_packets() {
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

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        4
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert!(audio.commands.is_empty());
    assert!(world.probe_entity(152).unwrap().living_dead);
    assert!(world.probe_entity(153).unwrap().living_dead);
    assert_eq!(world.counters().entity_events_applied, 2);
    assert_eq!(world.counters().entity_events_ignored, 0);
}

#[test]
fn remote_skeleton_family_death_events_wait_for_server_owned_sound_packets() {
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

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        6
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert!(audio.commands.is_empty());
    for entity_id in 154..=156 {
        assert!(world.probe_entity(entity_id).unwrap().living_dead);
    }
    assert_eq!(world.counters().entity_events_applied, 3);
    assert_eq!(world.counters().entity_events_ignored, 0);
}

#[test]
fn non_synchronized_sound_event_registry_data_keeps_static_audio_reference_resolution() {
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
        sound: SoundEventHolder::Reference { registry_id: 7 },
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
    assert!(world.registry_content("minecraft:sound_event").is_none());
    assert!(!world.registries().registries[0].configuration_synchronized);
    assert_eq!(
        audio.registry.event_id(0),
        Some("minecraft:entity.allay.ambient_with_item")
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
fn configuration_registry_and_tag_projections_commit_at_play_transition() {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:sound_event",
        0,
        vec![bbb_world::RegistryPacketEntry::stub(
            "minecraft:entity.cat.ambient",
        )],
    );
    world.record_registry_entries(
        "minecraft:jukebox_song",
        0,
        vec![bbb_world::RegistryPacketEntry::stub("minecraft:cat")],
    );
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: crate::entity_scene::PIGLIN_LOVED_ITEM_TAG.to_string(),
                entries: vec![bbb_protocol::item_types::VANILLA_ITEM_IRON_INGOT_ID],
            }],
        }],
    });
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(
        test_sound_catalog(),
        SoundEventRegistry::from_ids(["minecraft:entity.cat.ambient"]),
    );
    audio.jukebox_registry = JukeboxSongRegistry::from_registry_entry_ids(["minecraft:cat"]);

    let required_registries = minimal_non_empty_synchronized_registry_data();
    let expected_event_count = 4 + required_registries.len();
    let (tx, mut rx) = mpsc::channel(expected_event_count);
    tx.try_send(NetEvent::StateChanged {
        state: bbb_net::ConnectionState::Configuration,
    })
    .unwrap();
    tx.try_send(NetEvent::RegistryData(RegistryData {
        registry: "minecraft:sound_event".to_string(),
        raw_payload_len: 64,
        entries: vec![RegistryDataEntry {
            id: "minecraft:ambient.cave".to_string(),
            raw_data: None,
        }],
    }))
    .unwrap();
    tx.try_send(NetEvent::RegistryData(RegistryData {
        registry: "minecraft:jukebox_song".to_string(),
        raw_payload_len: 32,
        entries: vec![RegistryDataEntry {
            id: "minecraft:tears".to_string(),
            raw_data: None,
        }],
    }))
    .unwrap();
    tx.try_send(NetEvent::UpdateTags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: crate::entity_scene::PIGLIN_LOVED_ITEM_TAG.to_string(),
                entries: vec![bbb_protocol::item_types::VANILLA_ITEM_GOLD_INGOT_ID],
            }],
        }],
    }))
    .unwrap();
    for update in required_registries {
        tx.try_send(NetEvent::RegistryData(update)).unwrap();
    }
    drop(tx);

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        expected_event_count
    );
    assert_eq!(
        world.registry_resource_id_by_holder_id("minecraft:sound_event", 0),
        Some("minecraft:entity.cat.ambient")
    );
    assert_eq!(
        audio.registry.event_id(0),
        Some("minecraft:entity.cat.ambient")
    );
    assert_eq!(audio.jukebox_registry.song_id(0), Some("minecraft:cat"));

    world.apply_add_entity(protocol_add_entity_with_type(
        224,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
    ));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 224,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::OffHand,
            item: ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_GOLD_INGOT_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    }));
    let piglin_admiring = |world: &WorldStore| {
        crate::entity_scene::entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 224)
            .unwrap()
            .render_state
            .piglin_admiring
    };
    assert!(!piglin_admiring(&world));

    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::StateChanged {
        state: bbb_net::ConnectionState::Play,
    })
    .unwrap();
    drop(tx);
    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        1
    );
    assert_eq!(
        world.registry_resource_id_by_holder_id("minecraft:sound_event", 0),
        Some("minecraft:entity.allay.ambient_with_item")
    );
    assert_eq!(
        audio.registry.event_id(0),
        Some("minecraft:entity.allay.ambient_with_item")
    );
    assert_eq!(audio.registry.event_id(7), Some("minecraft:ambient.cave"));
    assert_eq!(audio.jukebox_registry.song_id(0), Some("minecraft:tears"));
    assert!(piglin_admiring(&world));
}

#[test]
fn silent_entity_sound_events_do_not_emit_runtime_commands() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        SetEntityData {
            id: 123,
            values: vec![entity_silent_data()],
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
