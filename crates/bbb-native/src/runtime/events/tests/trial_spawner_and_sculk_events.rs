use super::*;

fn advance_expected_trial_spawner_level_event_particle_randoms(
    event_type: i32,
    data: i32,
    random: &mut LevelEventSoundRandomState,
) {
    match event_type {
        3012 | 3021 => advance_expected_trial_spawner_spawn_particle_randoms(random),
        3013 | 3019 => advance_expected_trial_spawner_detect_player_particle_randoms(data, random),
        3014 => advance_expected_trial_spawner_eject_item_particle_randoms(random),
        3020 => {
            advance_expected_trial_spawner_detect_player_particle_randoms(0, random);
            advance_expected_trial_spawner_become_ominous_particle_randoms(random);
        }
        _ => unreachable!("unexpected trial spawner level event type {event_type}"),
    }
}

fn advance_expected_trial_spawner_spawn_particle_randoms(random: &mut LevelEventSoundRandomState) {
    for _ in 0..20 {
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
    }
}

fn advance_expected_trial_spawner_detect_player_particle_randoms(
    data: i32,
    random: &mut LevelEventSoundRandomState,
) {
    let count = 30_i64 + i64::from(data.min(10)) * 5;
    for _ in 0..count.max(0) {
        let _ = random.next_float();
        let _ = random.next_float();
        let _ = random.next_float();
    }
}

fn advance_expected_trial_spawner_eject_item_particle_randoms(
    random: &mut LevelEventSoundRandomState,
) {
    for _ in 0..20 {
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
    }
}

fn advance_expected_trial_spawner_become_ominous_particle_randoms(
    random: &mut LevelEventSoundRandomState,
) {
    for _ in 0..20 {
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
    }
}

#[test]
fn trial_spawner_level_events_emit_distance_delayed_vanilla_sounds() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 3012,
        pos: ProtocolBlockPos { x: 4, y: 65, z: -6 },
        data: 0,
        global: false,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 3020,
        pos: ProtocolBlockPos { x: -8, y: 70, z: 2 },
        data: 0,
        global: false,
    })))
    .unwrap();

    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_spawn_pitch =
        1.0 + (expected_random.next_float() - expected_random.next_float()) * 0.2;
    let expected_spawn_seed = expected_random.next_long();
    advance_expected_trial_spawner_level_event_particle_randoms(3012, 0, &mut expected_random);
    let expected_ominous_pitch =
        1.0 + (expected_random.next_float() - expected_random.next_float()) * 0.2;
    let expected_ominous_seed = expected_random.next_long();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 2);
    let AudioCommand::PlayPositionedSound(spawn) = &audio.commands[0] else {
        panic!(
            "expected positioned trial spawner sound, got {:?}",
            audio.commands[0]
        );
    };
    assert_eq!(
        spawn.sound.event_id,
        "minecraft:block.trial_spawner.spawn_mob"
    );
    assert_eq!(spawn.category, AudioCategory::Blocks);
    assert_eq!(spawn.position, [4.5, 65.5, -5.5]);
    assert_close(spawn.packet_volume, 1.0);
    assert_close(spawn.packet_pitch, expected_spawn_pitch);
    assert_eq!(spawn.seed, expected_spawn_seed);
    assert!(spawn.distance_delay);

    let AudioCommand::PlayPositionedSound(ominous) = &audio.commands[1] else {
        panic!(
            "expected positioned trial spawner ominous sound, got {:?}",
            audio.commands[1]
        );
    };
    assert_eq!(
        ominous.sound.event_id,
        "minecraft:block.trial_spawner.ominous_activate"
    );
    assert_eq!(ominous.category, AudioCategory::Blocks);
    assert_eq!(ominous.position, [-7.5, 70.5, 2.5]);
    assert_close(ominous.packet_volume, 0.3);
    assert_close(ominous.packet_pitch, expected_ominous_pitch);
    assert_eq!(ominous.seed, expected_ominous_seed);
    assert!(ominous.distance_delay);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:block.trial_spawner.ominous_activate")
    );
    assert!(world.last_sound().unwrap().distance_delay);
    assert_eq!(world.counters().sound_packets, 0);
    assert_eq!(world.counters().level_events_received, 2);
    assert_eq!(world.counters().level_events_tracked, 2);
}

#[test]
fn trial_spawner_audio_only_events_advance_post_sound_particle_randoms() {
    let cases = [
        (3012, 1, "minecraft:block.trial_spawner.spawn_mob"),
        (3013, 2, "minecraft:block.trial_spawner.detect_player"),
        (3014, 0, "minecraft:block.trial_spawner.eject_item"),
        (3019, 10, "minecraft:block.trial_spawner.detect_player"),
        (3020, 3, "minecraft:block.trial_spawner.ominous_activate"),
        (3021, 1, "minecraft:block.trial_spawner.spawn_item"),
    ];

    for (event_type, data, expected_sound) in cases {
        let event = LevelEvent {
            event_type,
            pos: ProtocolBlockPos {
                x: event_type - 3000,
                y: 65,
                z: -6,
            },
            data,
            global: false,
        };
        let followup = LevelEvent {
            event_type: 1004,
            pos: ProtocolBlockPos { x: 8, y: 64, z: -2 },
            data: 0,
            global: false,
        };
        let (tx, mut rx) = mpsc::channel(2);
        tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
            .unwrap();
        tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(followup)))
            .unwrap();

        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        let expected_pitch =
            1.0 + (expected_random.next_float() - expected_random.next_float()) * 0.2;
        let expected_seed = expected_random.next_long();
        advance_expected_trial_spawner_level_event_particle_randoms(
            event_type,
            data,
            &mut expected_random,
        );
        let expected_followup_seed = expected_random.next_long();

        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();
        let mut audio =
            RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
        let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

        assert_eq!(
            drain_net_events_with_sinks(
                &mut rx,
                &mut world,
                &mut counters,
                &None,
                Some(&mut audio),
                None,
                None,
                None,
                &mut level_event_sound_random,
            ),
            2
        );

        assert!(audio.errors.is_empty(), "{:?}", audio.errors);
        assert_eq!(audio.commands.len(), 2);
        let AudioCommand::PlayPositionedSound(sound) = &audio.commands[0] else {
            panic!(
                "expected positioned trial spawner sound, got {:?}",
                audio.commands[0]
            );
        };
        assert_eq!(sound.sound.event_id, expected_sound);
        assert_close(sound.packet_pitch, expected_pitch);
        assert_eq!(sound.seed, expected_seed);
        assert!(sound.distance_delay);

        let AudioCommand::PlayPositionedSound(followup_sound) = &audio.commands[1] else {
            panic!(
                "expected positioned followup sound, got {:?}",
                audio.commands[1]
            );
        };
        assert_eq!(
            followup_sound.sound.event_id,
            "minecraft:entity.firework_rocket.shoot"
        );
        assert_eq!(followup_sound.seed, expected_followup_seed);
        assert_eq!(world.counters().level_events_received, 2);
        assert_eq!(world.counters().level_events_tracked, 2);
    }
}

#[test]
fn trial_spawner_level_event_emits_sound_and_particle_side_effects() {
    let event = LevelEvent {
        event_type: 3012,
        pos: ProtocolBlockPos { x: 4, y: 65, z: -6 },
        data: 1,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_pitch = 1.0 + (expected_random.next_float() - expected_random.next_float()) * 0.2;
    let expected_seed = expected_random.next_long();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(sound) = &audio.commands[0] else {
        panic!(
            "expected positioned trial spawner sound, got {:?}",
            audio.commands[0]
        );
    };
    assert_eq!(
        sound.sound.event_id,
        "minecraft:block.trial_spawner.spawn_mob"
    );
    assert_eq!(sound.category, AudioCategory::Blocks);
    assert_close(sound.packet_pitch, expected_pitch);
    assert_eq!(sound.seed, expected_seed);
    assert!(sound.distance_delay);
    assert_eq!(particles.level_events, vec![event]);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

fn advance_expected_sculk_charge_level_event_particle_randoms(
    data: i32,
    full_block_pop: bool,
    random: &mut LevelEventSoundRandomState,
) {
    let count = data >> 6;
    if count <= 0 {
        let particle_count = if full_block_pop { 40 } else { 20 };
        for _ in 0..particle_count {
            let _ = random.next_float();
            let _ = random.next_float();
            let _ = random.next_float();
        }
        return;
    }

    let particle_data = data & 63;
    let face_count = if particle_data == 0 {
        6
    } else {
        particle_data.count_ones()
    };
    for _ in 0..face_count {
        let particle_count = random.next_int_bound(count + 1);
        for _ in 0..particle_count {
            let _ = random.next_double();
            let _ = random.next_double();
            let _ = random.next_double();
            let _ = random.next_double();
            let _ = random.next_double();
        }
    }
}

#[test]
fn sculk_charge_level_event_emits_vanilla_randomized_sound() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 3006,
        pos: ProtocolBlockPos { x: -2, y: 68, z: 3 },
        data: 5 << 6,
        global: false,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:block.sculk.charge");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [-1.5, 68.5, 3.5]);
    assert_close(command.packet_volume, 0.565_720_5);
    assert_close(command.packet_pitch, 0.760_804_6);
    assert_eq!(command.seed, -7_261_648_964_369_397_258);
    assert_eq!(world.counters().level_events_received, 1);
}

#[test]
fn sculk_charge_audio_only_charged_event_advances_particle_randoms() {
    let data = (5 << 6) | 0b001011;
    let event = LevelEvent {
        event_type: 3006,
        pos: ProtocolBlockPos { x: -2, y: 68, z: 3 },
        data,
        global: false,
    };
    let followup = LevelEvent {
        event_type: 1004,
        pos: ProtocolBlockPos { x: 8, y: 64, z: -2 },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(followup)))
        .unwrap();

    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let count = (data >> 6) as f32;
    assert!(expected_random.next_float() < 0.3 + count * 0.1);
    let expected_volume = 0.15 + 0.02 * count * count * expected_random.next_float();
    let expected_pitch = 0.4 + 0.3 * count * expected_random.next_float();
    let expected_seed = expected_random.next_long();
    advance_expected_sculk_charge_level_event_particle_randoms(data, false, &mut expected_random);
    let expected_followup_seed = expected_random.next_long();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            None,
            None,
            None,
            &mut level_event_sound_random,
        ),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 2);
    let AudioCommand::PlayPositionedSound(sculk) = &audio.commands[0] else {
        panic!(
            "expected positioned sculk sound, got {:?}",
            audio.commands[0]
        );
    };
    assert_eq!(sculk.sound.event_id, "minecraft:block.sculk.charge");
    assert_close(sculk.packet_volume, expected_volume);
    assert_close(sculk.packet_pitch, expected_pitch);
    assert_eq!(sculk.seed, expected_seed);

    let AudioCommand::PlayPositionedSound(followup_sound) = &audio.commands[1] else {
        panic!(
            "expected positioned followup sound, got {:?}",
            audio.commands[1]
        );
    };
    assert_eq!(
        followup_sound.sound.event_id,
        "minecraft:entity.firework_rocket.shoot"
    );
    assert_eq!(followup_sound.seed, expected_followup_seed);
    assert_eq!(world.counters().level_events_received, 2);
    assert_eq!(world.counters().level_events_tracked, 2);
}

#[test]
fn sculk_charge_audio_only_pop_event_uses_full_block_particle_context() {
    let event = LevelEvent {
        event_type: 3006,
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        data: 0,
        global: false,
    };
    let followup = LevelEvent {
        event_type: 1004,
        pos: ProtocolBlockPos { x: 8, y: 64, z: -2 },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockUpdate(BlockUpdate {
        pos: event.pos,
        block_state_id: 1,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(followup)))
        .unwrap();

    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_seed = expected_random.next_long();
    advance_expected_sculk_charge_level_event_particle_randoms(0, true, &mut expected_random);
    let expected_followup_seed = expected_random.next_long();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            None,
            None,
            None,
            &mut level_event_sound_random,
        ),
        4
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 2);
    let AudioCommand::PlayPositionedSound(sculk) = &audio.commands[0] else {
        panic!(
            "expected positioned sculk sound, got {:?}",
            audio.commands[0]
        );
    };
    assert_eq!(sculk.sound.event_id, "minecraft:block.sculk.charge");
    assert_close(sculk.packet_volume, 1.0);
    assert_close(sculk.packet_pitch, 1.0);
    assert_eq!(sculk.seed, expected_seed);

    let AudioCommand::PlayPositionedSound(followup_sound) = &audio.commands[1] else {
        panic!(
            "expected positioned followup sound, got {:?}",
            audio.commands[1]
        );
    };
    assert_eq!(
        followup_sound.sound.event_id,
        "minecraft:entity.firework_rocket.shoot"
    );
    assert_eq!(followup_sound.seed, expected_followup_seed);
    assert_eq!(world.counters().level_events_received, 2);
    assert_eq!(world.counters().level_events_tracked, 2);
}

#[test]
fn sculk_shrieker_level_event_emits_waterlogged_gated_sound_after_particles() {
    let dry_event = LevelEvent {
        event_type: 3007,
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        data: 0,
        global: false,
    };
    let wet_event = LevelEvent {
        event_type: 3007,
        pos: ProtocolBlockPos {
            x: 17,
            y: -64,
            z: -32,
        },
        data: 0,
        global: false,
    };
    let dry_shrieker = vanilla_block_state_id(
        "minecraft:sculk_shrieker",
        [
            ("can_summon", "false"),
            ("shrieking", "false"),
            ("waterlogged", "false"),
        ],
    );
    let wet_shrieker = vanilla_block_state_id(
        "minecraft:sculk_shrieker",
        [
            ("can_summon", "false"),
            ("shrieking", "false"),
            ("waterlogged", "true"),
        ],
    );
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockUpdate(BlockUpdate {
        pos: dry_event.pos,
        block_state_id: dry_shrieker,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockUpdate(BlockUpdate {
        pos: wet_event.pos,
        block_state_id: wet_shrieker,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(dry_event)))
        .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(wet_event)))
        .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_pitch = 0.6 + expected_random.next_float() * 0.4;
    let expected_seed = expected_random.next_long();

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        5
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(particles.level_events, vec![dry_event, wet_event]);
    assert_eq!(particles.batches.len(), 2);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(
        command.sound.event_id,
        "minecraft:block.sculk_shrieker.shriek"
    );
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [16.5, -63.5, -31.5]);
    assert_close(command.packet_volume, 2.0);
    assert_close(command.packet_pitch, expected_pitch);
    assert_eq!(command.seed, expected_seed);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:block.sculk_shrieker.shriek")
    );
    assert_eq!(world.last_sound().unwrap().seed, expected_seed);
    assert_eq!(world.counters().level_events_received, 2);
    assert_eq!(world.counters().level_events_tracked, 2);
}

#[test]
fn end_gateway_level_event_emits_vanilla_sound_and_particles() {
    let event = LevelEvent {
        event_type: 3000,
        pos: ProtocolBlockPos { x: 8, y: 64, z: -2 },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:block.end_gateway.spawn");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [8.5, 64.5, -1.5]);
    assert_close(command.packet_volume, 10.0);
    assert_close(command.packet_pitch, 0.685_933_77);
    assert_eq!(command.seed, 4_437_113_781_045_784_766);
    assert_eq!(particles.level_events, vec![event]);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn cobweb_place_level_event_emits_particles_before_distance_delayed_sound() {
    let event = LevelEvent {
        event_type: 3018,
        pos: ProtocolBlockPos { x: 2, y: 64, z: -5 },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(particles.level_events, vec![event]);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:block.cobweb.place");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [2.5, 64.5, -4.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, 1.013_698_2);
    assert_eq!(command.seed, 536_938_910_405_906_015);
    assert!(command.distance_delay);
    let sound = world.last_sound().unwrap();
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:block.cobweb.place")
    );
    assert_eq!(sound.seed, 536_938_910_405_906_015);
    assert!(sound.distance_delay);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn wax_on_level_event_emits_vanilla_sound_and_particles() {
    let event = LevelEvent {
        event_type: 3003,
        pos: ProtocolBlockPos { x: -3, y: 72, z: 5 },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    advance_wax_on_level_event_particle_randoms(&mut expected_random);
    let expected_seed = expected_random.next_long();

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:item.honeycomb.wax_on");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [-2.5, 72.5, 5.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, 1.0);
    assert_eq!(command.seed, expected_seed);
    assert_eq!(particles.level_events, vec![event]);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.last_sound().unwrap().seed, expected_seed);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:item.honeycomb.wax_on")
    );
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn wax_on_level_event_audio_only_advances_particles_before_sound_seed() {
    let event = LevelEvent {
        event_type: 3003,
        pos: ProtocolBlockPos { x: 1, y: 64, z: -2 },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    advance_wax_on_level_event_particle_randoms(&mut expected_random);
    let expected_seed = expected_random.next_long();
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            None,
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:item.honeycomb.wax_on");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [1.5, 64.5, -1.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, 1.0);
    assert_eq!(command.seed, expected_seed);
    assert_eq!(world.last_sound().unwrap().seed, expected_seed);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn global_level_event_emits_vanilla_camera_relative_sound() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1028,
        pos: ProtocolBlockPos { x: 10, y: 0, z: 0 },
        data: 0,
        global: true,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.5,
            y: -1.12,
            z: 0.5,
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
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(
        command.sound.event_id,
        "minecraft:entity.ender_dragon.death"
    );
    assert_eq!(command.category, AudioCategory::Hostile);
    assert!((command.position[0] - 2.5).abs() < 1.0e-6);
    assert!((command.position[1] - 0.5).abs() < 1.0e-6);
    assert!((command.position[2] - 0.5).abs() < 1.0e-6);
    assert_close(command.packet_volume, 5.0);
    assert_close(command.packet_pitch, 1.0);
    assert_eq!(command.seed, -4_962_768_465_676_381_896);
    let recorded = world.last_sound().unwrap();
    assert_eq!(
        recorded.sound.location.as_deref(),
        Some("minecraft:entity.ender_dragon.death")
    );
    assert_eq!(recorded.source, "hostile");
    assert!((recorded.position.x - 2.5).abs() < 1.0e-6);
    assert!((recorded.position.y - 0.5).abs() < 1.0e-6);
    assert!((recorded.position.z - 0.5).abs() < 1.0e-6);
    assert_close(recorded.volume, 5.0);
    assert_close(recorded.pitch, 1.0);
    assert_eq!(recorded.seed, -4_962_768_465_676_381_896);
    assert_eq!(world.counters().sound_packets, 0);
    assert_eq!(world.counters().level_events_received, 1);
}

#[test]
fn global_level_event_without_camera_does_not_emit_runtime_sound() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1023,
        pos: ProtocolBlockPos { x: 10, y: 0, z: 0 },
        data: 0,
        global: true,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert!(audio.commands.is_empty());
    assert_eq!(world.counters().level_events_received, 1);
}

#[test]
fn portal_travel_level_event_emits_vanilla_local_ambience() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1032,
        pos: ProtocolBlockPos { x: -4, y: 70, z: 9 },
        data: 0,
        global: false,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayLocalSound(command) = &audio.commands[0] else {
        panic!("expected local sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:block.portal.travel");
    assert_eq!(command.sound.sound_name, "minecraft:portal/travel");
    assert_eq!(command.category, AudioCategory::Ambient);
    assert_close(command.packet_volume, 0.25);
    assert_close(command.packet_pitch, 1.092_387_1);
    assert_eq!(command.seed, 0);
    assert_eq!(
        world.last_local_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:block.portal.travel")
    );
    assert_eq!(world.counters().level_events_received, 1);
}

#[test]
fn jukebox_level_events_update_world_audio_state() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1010,
        pos: ProtocolBlockPos { x: -4, y: 70, z: 9 },
        data: 27,
        global: false,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1011,
        pos: ProtocolBlockPos { x: -4, y: 70, z: 9 },
        data: 0,
        global: false,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );

    assert!(world.playing_jukebox_songs().is_empty());
    let event = world.last_jukebox_event().unwrap();
    assert_eq!(event.action, bbb_world::JukeboxLevelEventAction::Stop);
    assert_eq!(event.pos, BlockPos { x: -4, y: 70, z: 9 });
    assert_eq!(event.song_registry_id, None);
    assert!(event.stopped_existing);
    assert_eq!(world.counters().level_events_received, 2);
    assert_eq!(world.counters().level_events_tracked, 2);
}

#[test]
fn jukebox_level_events_emit_runtime_audio_commands() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1010,
        pos: ProtocolBlockPos { x: 4, y: 64, z: -7 },
        data: 1,
        global: false,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1011,
        pos: ProtocolBlockPos { x: 4, y: 64, z: -7 },
        data: 0,
        global: false,
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
    assert_eq!(audio.commands.len(), 2);
    let AudioCommand::PlayJukeboxSong(play) = &audio.commands[0] else {
        panic!("expected jukebox play command, got {:?}", audio.commands[0]);
    };
    assert_eq!(play.sound.event_id, "minecraft:music_disc.cat");
    assert_eq!(play.sound.sound_name, "minecraft:records/cat");
    assert_eq!(play.category, AudioCategory::Records);
    assert_eq!(play.position, [4.5, 64.5, -6.5]);
    assert_eq!(play.jukebox_pos, [4, 64, -7]);
    assert_close(play.packet_volume, 4.0);
    assert_close(play.packet_pitch, 1.0);

    let AudioCommand::StopJukeboxSong(stop) = &audio.commands[1] else {
        panic!("expected jukebox stop command, got {:?}", audio.commands[1]);
    };
    assert_eq!(stop.jukebox_pos, [4, 64, -7]);
    assert!(world.playing_jukebox_songs().is_empty());
    assert_eq!(world.counters().level_events_received, 2);
}

#[test]
fn jukebox_song_registry_data_updates_audio_resolution() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::RegistryData(RegistryData {
        registry: "minecraft:jukebox_song".to_string(),
        raw_payload_len: 48,
        entries: vec![RegistryDataEntry {
            id: "minecraft:tears".to_string(),
            raw_data: None,
        }],
    }))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1010,
        pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
        data: 0,
        global: false,
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
            .registry_content("minecraft:jukebox_song")
            .unwrap()
            .entries[0]
            .id,
        "minecraft:tears"
    );
    let AudioCommand::PlayJukeboxSong(play) = &audio.commands[0] else {
        panic!("expected jukebox play command, got {:?}", audio.commands[0]);
    };
    assert_eq!(play.sound.event_id, "minecraft:music_disc.tears");
    assert_eq!(play.sound.sound_name, "minecraft:records/tears");
    assert_eq!(play.jukebox_pos, [1, 2, 3]);
}
