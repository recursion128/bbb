use super::*;

#[test]
fn block_destruction_event_updates_world_and_counter() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::BlockDestruction(
        bbb_protocol::packets::BlockDestruction {
            id: 4,
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5,
            },
            progress: 6,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockDestruction(
        bbb_protocol::packets::BlockDestruction {
            id: 4,
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5,
            },
            progress: 10,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockDestruction(
        bbb_protocol::packets::BlockDestruction {
            id: 99,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            progress: 255,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );
    assert_eq!(world.counters().block_destructions_received, 3);
    assert_eq!(world.counters().block_destructions_tracked, 0);
    assert_eq!(world.counters().block_destructions_removed, 1);
    assert_eq!(world.counters().block_destructions_ignored, 1);
    assert!(world.block_destruction(4).is_none());
}

#[test]
fn block_and_level_events_update_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::BlockEvent(
        bbb_protocol::packets::BlockEvent {
            pos: ProtocolBlockPos {
                x: 12,
                y: 65,
                z: -5,
            },
            b0: 2,
            b1: 9,
            block_id: 54,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(
        bbb_protocol::packets::LevelEvent {
            event_type: 1001,
            pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
            data: 42,
            global: true,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    let world_counters = world.counters();
    assert_eq!(world_counters.block_events_received, 1);
    assert_eq!(world_counters.block_events_tracked, 1);
    assert_eq!(world_counters.level_events_received, 1);
    assert_eq!(world_counters.level_events_tracked, 1);

    let block_event = world.block_events().first().unwrap();
    assert_eq!(
        block_event.pos,
        BlockPos {
            x: 12,
            y: 65,
            z: -5
        }
    );
    assert_eq!(block_event.b0, 2);
    assert_eq!(block_event.b1, 9);
    assert_eq!(block_event.block_id, 54);

    let level_event = world.level_events().first().unwrap();
    assert_eq!(level_event.event_type, 1001);
    assert_eq!(level_event.pos, BlockPos { x: 3, y: 4, z: 5 });
    assert_eq!(level_event.data, 42);
    assert!(level_event.global);
}

#[test]
fn level_event_2001_emits_vanilla_block_break_sound() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 2001,
        pos: ProtocolBlockPos { x: 2, y: 3, z: -4 },
        data: 9,
        global: false,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    world.set_default_block_sound_profiles(BTreeMap::from([(
        "minecraft:grass_block".to_string(),
        WorldBlockSoundProfile {
            break_sound: "minecraft:block.grass.break".to_string(),
            hit_sound: "minecraft:block.grass.hit".to_string(),
            volume: 0.8,
            pitch: 1.2,
        },
    )]));
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
    assert_eq!(command.sound.event_id, "minecraft:block.grass.break");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [2.5, 3.5, -3.5]);
    assert_close(command.packet_volume, 0.9);
    assert_close(command.packet_pitch, 0.96);
    assert_eq!(command.seed, -4_962_768_465_676_381_896);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn level_event_3008_emits_brushable_completion_sound_and_particles() {
    let suspicious_sand = vanilla_block_state_id("minecraft:suspicious_sand", [("dusted", "2")]);
    let event = LevelEvent {
        event_type: 3008,
        pos: ProtocolBlockPos { x: 4, y: 65, z: -6 },
        data: suspicious_sand,
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
    assert_eq!(command.sound.event_id, "minecraft:item.brush.brushing.sand");
    assert_eq!(command.category, AudioCategory::Players);
    assert_eq!(command.position, [4.5, 65.5, -5.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, 1.0);
    assert_eq!(command.seed, -4_962_768_465_676_381_896);
    assert_eq!(particles.level_events, vec![event]);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:item.brush.brushing.sand")
    );
    assert_eq!(world.last_sound().unwrap().source, "player");
    assert_eq!(world.counters().sound_packets, 0);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn plant_growth_level_event_plays_bone_meal_sound_after_particles() {
    let short_grass_id = vanilla_block_state_id("minecraft:short_grass", []);
    let event = level_event_at_with_data(1505, 20, -63, -32, 4);
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut world = WorldStore::new();
    world
        .insert_level_chunk_with_light(synthetic_native_level_chunk_packet())
        .unwrap();
    set_test_block(&mut world, block_pos(20, -63, -32), short_grass_id);
    let context = level_event_particle_context(&world, &event);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    advance_growth_randoms_for_context(&event, &context, &mut expected_random);
    let expected_seed = expected_random.next_long();

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

    assert_eq!(particles.level_events, vec![event]);
    assert_eq!(particles.level_event_contexts, vec![context]);
    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:item.bone_meal.use");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [20.5, -62.5, -31.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, 1.0);
    assert_eq!(command.seed, expected_seed);
    let sound = world.last_sound().unwrap();
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:item.bone_meal.use")
    );
    assert_eq!(sound.source, "block");
    assert_eq!(sound.seed, expected_seed);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn plant_growth_level_event_audio_only_advances_particle_randoms_before_sound_seed() {
    let water_id = vanilla_block_state_id("minecraft:water", [("level", "0")]);
    let stone_id = vanilla_block_state_id("minecraft:stone", []);
    let event = level_event_at_with_data(1505, 16, -63, -32, 3);
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut world = WorldStore::new();
    world
        .insert_level_chunk_with_light(synthetic_native_level_chunk_packet())
        .unwrap();
    set_test_block(&mut world, block_pos(16, -63, -32), water_id);
    set_test_block(&mut world, block_pos(16, -64, -32), stone_id);
    let context = level_event_particle_context(&world, &event);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    advance_growth_randoms_for_context(&event, &context, &mut expected_random);
    let expected_seed = expected_random.next_long();

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
    assert_eq!(command.sound.event_id, "minecraft:item.bone_meal.use");
    assert_eq!(command.seed, expected_seed);
    assert_eq!(world.last_sound().unwrap().seed, expected_seed);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn fixed_level_event_emits_vanilla_positioned_sound() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1004,
        pos: ProtocolBlockPos { x: 8, y: 64, z: -2 },
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
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(
        command.sound.event_id,
        "minecraft:entity.firework_rocket.shoot"
    );
    assert_eq!(command.category, AudioCategory::Neutral);
    assert_eq!(command.position, [8.5, 64.5, -1.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, 1.2);
    assert_eq!(command.seed, -4_962_768_465_676_381_896);
    let recorded = world.last_sound().unwrap();
    assert_eq!(
        recorded.sound.location.as_deref(),
        Some("minecraft:entity.firework_rocket.shoot")
    );
    assert_eq!(recorded.source, "neutral");
    assert_eq!(
        recorded.position,
        ProtocolVec3d {
            x: 8.5,
            y: 64.5,
            z: -1.5,
        }
    );
    assert_close(recorded.volume, 1.0);
    assert_close(recorded.pitch, 1.2);
    assert_eq!(recorded.seed, -4_962_768_465_676_381_896);
    assert_eq!(world.counters().sound_packets, 0);
    assert_eq!(world.counters().level_events_received, 1);
}

#[test]
fn randomized_level_event_emits_vanilla_positioned_sound() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1015,
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
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:entity.ghast.warn");
    assert_eq!(command.category, AudioCategory::Hostile);
    assert_eq!(command.position, [-3.5, 70.5, 9.5]);
    assert_close(command.packet_volume, 10.0);
    assert_close(command.packet_pitch, 0.979_905_37);
    assert_eq!(command.seed, 4_437_113_781_045_784_766);
    let recorded = world.last_sound().unwrap();
    assert_eq!(
        recorded.sound.location.as_deref(),
        Some("minecraft:entity.ghast.warn")
    );
    assert_eq!(recorded.source, "hostile");
    assert_eq!(
        recorded.position,
        ProtocolVec3d {
            x: -3.5,
            y: 70.5,
            z: 9.5,
        }
    );
    assert_close(recorded.volume, 10.0);
    assert_close(recorded.pitch, 0.979_905_37);
    assert_eq!(recorded.seed, 4_437_113_781_045_784_766);
    assert_eq!(world.counters().sound_packets, 0);
    assert_eq!(world.counters().level_events_received, 1);
}

#[test]
fn potion_and_dragon_fireball_level_events_emit_vanilla_sounds() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 2002,
        pos: ProtocolBlockPos { x: 1, y: 64, z: -3 },
        data: 0x3366cc,
        global: false,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 2006,
        pos: ProtocolBlockPos { x: -2, y: 70, z: 4 },
        data: 1,
        global: false,
    })))
    .unwrap();

    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    advance_potion_break_level_event_particle_randoms(&mut expected_random);
    let expected_potion_pitch = 0.9 + expected_random.next_float().clamp(0.0, 1.0) * 0.1;
    let expected_potion_seed = expected_random.next_long();
    advance_dragon_fireball_explode_level_event_particle_randoms(&mut expected_random);
    let expected_dragon_pitch = 0.9 + expected_random.next_float().clamp(0.0, 1.0) * 0.1;
    let expected_dragon_seed = expected_random.next_long();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 2);
    let AudioCommand::PlayPositionedSound(potion) = &audio.commands[0] else {
        panic!(
            "expected positioned potion sound, got {:?}",
            audio.commands[0]
        );
    };
    assert_eq!(
        potion.sound.event_id,
        "minecraft:entity.splash_potion.break"
    );
    assert_eq!(potion.category, AudioCategory::Neutral);
    assert_eq!(potion.position, [1.5, 64.5, -2.5]);
    assert_close(potion.packet_volume, 1.0);
    assert_close(potion.packet_pitch, expected_potion_pitch);
    assert_eq!(potion.seed, expected_potion_seed);

    let AudioCommand::PlayPositionedSound(dragon) = &audio.commands[1] else {
        panic!(
            "expected positioned dragon sound, got {:?}",
            audio.commands[1]
        );
    };
    assert_eq!(
        dragon.sound.event_id,
        "minecraft:entity.dragon_fireball.explode"
    );
    assert_eq!(dragon.category, AudioCategory::Hostile);
    assert_eq!(dragon.position, [-1.5, 70.5, 4.5]);
    assert_close(dragon.packet_volume, 1.0);
    assert_close(dragon.packet_pitch, expected_dragon_pitch);
    assert_eq!(dragon.seed, expected_dragon_seed);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:entity.dragon_fireball.explode")
    );
    assert_eq!(world.counters().sound_packets, 0);
    assert_eq!(world.counters().level_events_received, 2);
    assert_eq!(world.counters().level_events_tracked, 2);
}

#[test]
fn potion_break_level_event_plays_sound_after_particles() {
    let event = LevelEvent {
        event_type: 2007,
        pos: ProtocolBlockPos { x: 1, y: 64, z: -3 },
        data: 0x3366cc,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    advance_potion_break_level_event_particle_randoms(&mut expected_random);
    let expected_pitch = 0.9 + expected_random.next_float().clamp(0.0, 1.0) * 0.1;
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

    assert_eq!(particles.level_events, vec![event]);
    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(
        command.sound.event_id,
        "minecraft:entity.splash_potion.break"
    );
    assert_eq!(command.category, AudioCategory::Neutral);
    assert_eq!(command.position, [1.5, 64.5, -2.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, expected_pitch);
    assert_eq!(command.seed, expected_seed);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:entity.splash_potion.break")
    );
    assert_eq!(world.last_sound().unwrap().seed, expected_seed);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn dragon_fireball_level_event_plays_sound_after_particles() {
    let event = LevelEvent {
        event_type: 2006,
        pos: ProtocolBlockPos { x: -2, y: 70, z: 4 },
        data: 1,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    advance_dragon_fireball_explode_level_event_particle_randoms(&mut expected_random);
    let expected_pitch = 0.9 + expected_random.next_float().clamp(0.0, 1.0) * 0.1;
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

    assert_eq!(particles.level_events, vec![event]);
    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(
        command.sound.event_id,
        "minecraft:entity.dragon_fireball.explode"
    );
    assert_eq!(command.category, AudioCategory::Hostile);
    assert_eq!(command.position, [-1.5, 70.5, 4.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, expected_pitch);
    assert_eq!(command.seed, expected_seed);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:entity.dragon_fireball.explode")
    );
    assert_eq!(world.last_sound().unwrap().seed, expected_seed);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}
