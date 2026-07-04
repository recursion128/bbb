use super::*;

#[test]
fn world_effect_events_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::Explosion(Explosion {
        center: ProtocolVec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        radius: 4.5,
        block_count: 7,
        player_knockback: Some(ProtocolVec3d {
            x: 0.25,
            y: -0.5,
            z: 1.5,
        }),
        raw_effect_payload: vec![0x2d, 0x2a, 0x01, 0x00],
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelParticles(
        LevelParticles {
            override_limiter: true,
            always_show: false,
            position: ProtocolVec3d {
                x: 10.0,
                y: 64.5,
                z: -3.25,
            },
            offset: ProtocolVec3d {
                x: f64::from(0.1_f32),
                y: f64::from(0.2_f32),
                z: f64::from(0.3_f32),
            },
            max_speed: 1.5,
            count: 16,
            particle: ParticlePayload {
                particle_type_id: 45,
                raw_options: vec![0xaa, 0xbb],
            },
        },
    )))
    .unwrap();
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        delta_movement: ProtocolVec3d {
            x: 0.5,
            y: -0.25,
            z: 1.0,
        },
        ..LocalPlayerPoseState::default()
    });
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert_eq!(world.counters().explosion_packets, 1);
    assert_eq!(
        world.last_explosion(),
        Some(&bbb_world::ExplosionEventState {
            center: ProtocolVec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            radius: 4.5,
            block_count: 7,
            player_knockback: Some(ProtocolVec3d {
                x: 0.25,
                y: -0.5,
                z: 1.5,
            }),
            raw_effect_payload_len: 4,
        })
    );
    assert_eq!(
        world.local_player_pose().unwrap().delta_movement,
        ProtocolVec3d {
            x: 0.75,
            y: -0.75,
            z: 2.5,
        }
    );
    assert_eq!(world.counters().level_particles_packets, 1);
    assert_eq!(
        world.last_level_particles(),
        Some(&bbb_world::LevelParticlesEventState {
            override_limiter: true,
            always_show: false,
            position: ProtocolVec3d {
                x: 10.0,
                y: 64.5,
                z: -3.25,
            },
            offset: ProtocolVec3d {
                x: f64::from(0.1_f32),
                y: f64::from(0.2_f32),
                z: f64::from(0.3_f32),
            },
            max_speed: 1.5,
            count: 16,
            particle_type_id: 45,
            raw_options_len: 2,
        })
    );
}

#[test]
fn level_particles_emit_particle_runtime_batch_and_world_counters() {
    let packet = LevelParticles {
        override_limiter: false,
        always_show: true,
        position: ProtocolVec3d {
            x: 10.0,
            y: 64.5,
            z: -3.25,
        },
        offset: ProtocolVec3d {
            x: f64::from(0.1_f32),
            y: f64::from(0.2_f32),
            z: f64::from(0.3_f32),
        },
        max_speed: 1.5,
        count: 0,
        particle: ParticlePayload {
            particle_type_id: 4,
            raw_options: vec![0xcc],
        },
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelParticles(
        packet.clone(),
    )))
    .unwrap();
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        ..LocalPlayerPoseState::default()
    });
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert_eq!(particles.packets, vec![packet]);
    assert_eq!(
        particles.contexts,
        vec![LevelParticleSpawnContext {
            camera_position: Some([1.0, 3.62, 3.0]),
            vibration_entity_position: None,
        }]
    );
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().level_particles_packets, 1);
    assert_eq!(world.last_level_particles().unwrap().count, 0);
}

#[test]
fn level_particles_context_resolves_vibration_entity_position_source() {
    let mut raw_options = vec![1, 123];
    raw_options.extend_from_slice(&0.75_f32.to_be_bytes());
    raw_options.push(27);
    let packet = LevelParticles {
        override_limiter: false,
        always_show: true,
        position: ProtocolVec3d {
            x: 10.0,
            y: 64.5,
            z: -3.25,
        },
        offset: ProtocolVec3d::default(),
        max_speed: 0.0,
        count: 0,
        particle: ParticlePayload {
            particle_type_id: 48,
            raw_options,
        },
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelParticles(
        packet.clone(),
    )))
    .unwrap();
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(123));
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert_eq!(particles.packets, vec![packet]);
    assert_eq!(
        particles.contexts,
        vec![LevelParticleSpawnContext {
            camera_position: None,
            vibration_entity_position: Some(crate::particle_runtime::LevelParticleEntityPosition {
                entity_id: 123,
                position: [1.0, 64.0, -2.0],
            }),
        }]
    );
}

#[test]
fn level_event_smoke_particles_emit_particle_runtime_batch_and_world_counters() {
    let event = LevelEvent {
        event_type: 1502,
        pos: ProtocolBlockPos { x: 4, y: 65, z: -7 },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert_eq!(particles.level_events, vec![event]);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
    assert_eq!(world.level_events().last().unwrap().event_type, 1502);
}

#[test]
fn post_sound_smoke_level_events_advance_particle_randoms_without_particle_sink() {
    fn advance_expected_particle_randoms(event_type: i32, random: &mut LevelEventSoundRandomState) {
        match event_type {
            1501 => {
                for _ in 0..8 {
                    let _ = random.next_double();
                    let _ = random.next_double();
                }
            }
            1502 => {
                for _ in 0..5 {
                    let _ = random.next_double();
                    let _ = random.next_double();
                    let _ = random.next_double();
                }
            }
            1503 => {
                for _ in 0..16 {
                    let _ = random.next_double();
                    let _ = random.next_double();
                }
            }
            _ => unreachable!("unexpected level event type {event_type}"),
        }
    }

    let cases = [
        (1501, "minecraft:block.lava.extinguish", true),
        (1502, "minecraft:block.redstone_torch.burnout", true),
        (1503, "minecraft:block.end_portal_frame.fill", false),
    ];

    for (event_type, expected_sound, randomized_pitch) in cases {
        let event = LevelEvent {
            event_type,
            pos: ProtocolBlockPos {
                x: event_type - 1500,
                y: 65,
                z: -7,
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
        let (tx, mut rx) = mpsc::channel(2);
        tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
            .unwrap();
        tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(followup)))
            .unwrap();

        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        let expected_pitch = if randomized_pitch {
            2.6 + (expected_random.next_float() - expected_random.next_float()) * 0.8
        } else {
            1.0
        };
        let expected_seed = expected_random.next_long();
        advance_expected_particle_randoms(event_type, &mut expected_random);
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
                "expected positioned smoke-side sound, got {:?}",
                audio.commands[0]
            );
        };
        assert_eq!(sound.sound.event_id, expected_sound);
        assert_close(sound.packet_pitch, expected_pitch);
        assert_eq!(sound.seed, expected_seed);

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

fn advance_expected_simple_particle_only_level_event_randoms(
    event_type: i32,
    random: &mut LevelEventSoundRandomState,
) {
    match event_type {
        2000 | 2010 => advance_expected_shoot_particles_randoms(random),
        2003 => advance_expected_item_break_particle_randoms(random),
        2004 => {
            for _ in 0..20 {
                let _ = random.next_double();
                let _ = random.next_double();
                let _ = random.next_double();
            }
        }
        2009 => {
            for _ in 0..8 {
                let _ = random.next_double();
                let _ = random.next_double();
            }
        }
        _ => unreachable!("unexpected simple particle-only event type {event_type}"),
    }
}

fn advance_expected_shoot_particles_randoms(random: &mut LevelEventSoundRandomState) {
    for _ in 0..10 {
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
    }
}

fn advance_expected_item_break_particle_randoms(random: &mut LevelEventSoundRandomState) {
    for _ in 0..8 {
        let _ = random.next_gaussian();
        let _ = random.next_double();
        let _ = random.next_gaussian();
    }
}

fn advance_expected_block_face_axis_level_event_randoms(
    event_type: i32,
    data: i32,
    random: &mut LevelEventSoundRandomState,
) {
    match event_type {
        3002 if matches!(data, 0..=2) => advance_expected_axis_particles_randoms(10, 19, random),
        3002 => advance_expected_block_face_particle_randoms(3, 5, random),
        3004 | 3005 => advance_expected_block_face_particle_randoms(3, 5, random),
        3009 => advance_expected_block_face_particle_randoms(3, 6, random),
        _ => unreachable!("unexpected block-face/axis event type {event_type}"),
    }
}

fn advance_expected_block_face_particle_randoms(
    min_particles_per_face: i32,
    max_particles_per_face: i32,
    random: &mut LevelEventSoundRandomState,
) {
    for _ in 0..6 {
        let particle_count = random
            .next_int_bound(max_particles_per_face - min_particles_per_face + 1)
            + min_particles_per_face;
        for _ in 0..particle_count {
            let _ = random.next_double();
            let _ = random.next_double();
            let _ = random.next_double();
            let _ = random.next_double();
            let _ = random.next_double();
        }
    }
}

fn advance_expected_axis_particles_randoms(
    min_particles: i32,
    max_particles: i32,
    random: &mut LevelEventSoundRandomState,
) {
    let particle_count = random.next_int_bound(max_particles - min_particles + 1) + min_particles;
    for _ in 0..particle_count {
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
    }
}

#[test]
fn particle_only_level_events_advance_randoms_without_particle_sink_before_followup_sound() {
    for event_type in [2000, 2003, 2004, 2009, 2010] {
        let event = LevelEvent {
            event_type,
            pos: ProtocolBlockPos {
                x: event_type - 1990,
                y: 64,
                z: -3,
            },
            data: 1,
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
        advance_expected_simple_particle_only_level_event_randoms(event_type, &mut expected_random);
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
        assert_eq!(audio.commands.len(), 1);
        let AudioCommand::PlayPositionedSound(sound) = &audio.commands[0] else {
            panic!(
                "expected positioned followup sound, got {:?}",
                audio.commands[0]
            );
        };
        assert_eq!(
            sound.sound.event_id,
            "minecraft:entity.firework_rocket.shoot"
        );
        assert_eq!(sound.seed, expected_followup_seed);
        assert_eq!(world.counters().level_events_received, 2);
        assert_eq!(world.counters().level_events_tracked, 2);
    }
}

#[test]
fn block_face_axis_level_events_advance_randoms_without_particle_sink_before_followup_sound() {
    for (event_type, data) in [(3002, 1), (3002, 99), (3004, 0), (3005, 0), (3009, 0)] {
        let event = LevelEvent {
            event_type,
            pos: ProtocolBlockPos {
                x: event_type - 3000,
                y: 64,
                z: -3,
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
        advance_expected_block_face_axis_level_event_randoms(
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
        assert_eq!(audio.commands.len(), 1);
        let AudioCommand::PlayPositionedSound(sound) = &audio.commands[0] else {
            panic!(
                "expected positioned followup sound, got {:?}",
                audio.commands[0]
            );
        };
        assert_eq!(
            sound.sound.event_id,
            "minecraft:entity.firework_rocket.shoot"
        );
        assert_eq!(sound.seed, expected_followup_seed);
        assert_eq!(world.counters().level_events_received, 2);
        assert_eq!(world.counters().level_events_tracked, 2);
    }
}

#[test]
fn sculk_charge_pop_level_event_threads_full_block_context_to_particles() {
    let full_block_event = LevelEvent {
        event_type: 3006,
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        data: 0,
        global: false,
    };
    let empty_block_event = LevelEvent {
        event_type: 3006,
        pos: ProtocolBlockPos {
            x: 17,
            y: -64,
            z: -32,
        },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ChunksBiomes(
        ChunksBiomes {
            chunks: vec![ChunkBiomeData {
                pos: ProtocolChunkPos { x: 1, z: -2 },
                raw_biomes: single_biome_payload(7),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockUpdate(BlockUpdate {
        pos: full_block_event.pos,
        block_state_id: 1,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(
        full_block_event,
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(
        empty_block_event,
    )))
    .unwrap();
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        5
    );

    assert_eq!(
        world
            .probe_block(bbb_world::BlockPos {
                x: 16,
                y: -64,
                z: -32,
            })
            .and_then(|probe| probe.block_name),
        Some("minecraft:stone".to_string())
    );
    assert_eq!(
        particles.level_events,
        vec![full_block_event, empty_block_event]
    );
    assert_eq!(
        particles.level_event_contexts,
        vec![
            LevelEventParticleContext {
                sculk_charge_pop_full_block: Some(true),
                block_state_id_at_event_pos: Some(1),
                biome_id_at_event_pos: Some(7),
                vault_block_entity_at_event_pos: false,
                vault_connection_particles: None,
                dripstone_drip_particle: None,
                growth_particles: None,
                in_block_particle_spread_height: None,
                composter_fill_center_shape_max_y: None,
            },
            LevelEventParticleContext {
                sculk_charge_pop_full_block: Some(false),
                block_state_id_at_event_pos: Some(0),
                biome_id_at_event_pos: Some(7),
                vault_block_entity_at_event_pos: false,
                vault_connection_particles: None,
                dripstone_drip_particle: None,
                growth_particles: None,
                in_block_particle_spread_height: None,
                composter_fill_center_shape_max_y: None,
            },
        ]
    );
}

#[test]
fn vault_activation_level_event_threads_block_entity_context_to_particles() {
    let vault_event = LevelEvent {
        event_type: 3015,
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        data: 0,
        global: false,
    };
    let non_vault_event = LevelEvent {
        event_type: 3015,
        pos: ProtocolBlockPos {
            x: 17,
            y: -64,
            z: -32,
        },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockEntityData(
        BlockEntityData {
            pos: vault_event.pos,
            block_entity_type_id: 45,
            raw_nbt: vec![0],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(vault_event)))
        .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(non_vault_event)))
        .unwrap();
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    bbb_world::advance_vault_activation_particle_randoms(&mut expected_random);
    let expected_pitch = (expected_random.next_float() - expected_random.next_float()) * 0.2 + 1.0;
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
        4
    );

    assert_eq!(
        world.block_entity_type_id_at(bbb_world::BlockPos {
            x: 16,
            y: -64,
            z: -32,
        }),
        Some(45)
    );
    assert_eq!(particles.level_events, vec![vault_event, non_vault_event]);
    assert_eq!(
        particles.level_event_contexts[0].vault_block_entity_at_event_pos,
        true
    );
    assert_eq!(
        particles.level_event_contexts[1].vault_block_entity_at_event_pos,
        false
    );
    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:block.vault.activate");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [16.5, -63.5, -31.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, expected_pitch);
    assert!(command.distance_delay);
    assert_eq!(command.seed, expected_seed);
    let sound = world.last_sound().unwrap();
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:block.vault.activate")
    );
    assert_eq!(sound.seed, expected_seed);
    assert!(sound.distance_delay);
    assert_eq!(world.counters().level_events_received, 2);
    assert_eq!(world.counters().level_events_tracked, 2);
}

#[test]
fn vault_activation_level_event_threads_connection_particle_context() {
    let event = LevelEvent {
        event_type: 3015,
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        data: 0,
        global: false,
    };
    let player_uuid = Uuid::from_u128(0x0011_2233_4455_6677_8899_aabb_ccdd_eeff);
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockUpdate(BlockUpdate {
        pos: event.pos,
        block_state_id: vault_block_state_id("east"),
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockEntityData(
        BlockEntityData {
            pos: event.pos,
            block_entity_type_id: 45,
            raw_nbt: vault_shared_data_nbt(&[player_uuid], Some(3.0)),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        vault_test_player(
            77,
            player_uuid,
            ProtocolVec3d {
                x: 18.25,
                y: -64.0,
                z: -32.0,
            },
        ),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        5
    );

    let context = particles.level_event_contexts[0].clone();
    assert!(context.vault_block_entity_at_event_pos);
    let connection = context.vault_connection_particles.unwrap();
    assert_eq!(connection.origin, [17.0, -62.25, -31.5]);
    assert_eq!(connection.targets.len(), 1);
    assert_eq!(connection.targets[0].entity_id, 77);
    assert_eq!(connection.targets[0].uuid, player_uuid);
    assert_close64(connection.targets[0].target_position[0], 18.25);
    assert_close64(connection.targets[0].target_position[1], -63.1);
    assert_close64(connection.targets[0].target_position[2], -32.0);
}

#[test]
fn vault_deactivation_level_event_emits_sound_after_particles() {
    let event = LevelEvent {
        event_type: 3016,
        pos: ProtocolBlockPos { x: -3, y: 72, z: 5 },
        data: 1,
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
    bbb_world::advance_vault_deactivation_particle_randoms(&mut expected_random);
    let expected_pitch = (expected_random.next_float() - expected_random.next_float()) * 0.2 + 1.0;
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

    assert_eq!(particles.level_events, vec![event]);
    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:block.vault.deactivate");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [-2.5, 72.5, 5.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, expected_pitch);
    assert!(command.distance_delay);
    assert_eq!(command.seed, expected_seed);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:block.vault.deactivate")
    );
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn dripstone_drip_level_event_context_uses_fluid_and_dimension_default() {
    let tip_id = vanilla_block_state_id(
        "minecraft:pointed_dripstone",
        [
            ("thickness", "tip"),
            ("vertical_direction", "down"),
            ("waterlogged", "false"),
        ],
    );
    let waterlogged_tip_id = vanilla_block_state_id(
        "minecraft:pointed_dripstone",
        [
            ("thickness", "tip"),
            ("vertical_direction", "down"),
            ("waterlogged", "true"),
        ],
    );
    let stone_id = vanilla_block_state_id("minecraft:stone", []);
    let water_id = vanilla_block_state_id("minecraft:water", [("level", "0")]);
    let lava_id = vanilla_block_state_id("minecraft:lava", [("level", "0")]);

    let mut world = WorldStore::new();
    world
        .insert_level_chunk_with_light(synthetic_native_level_chunk_packet())
        .unwrap();
    set_test_block(&mut world, block_pos(16, -64, -32), tip_id);
    set_test_block(&mut world, block_pos(16, -63, -32), stone_id);
    set_test_block(&mut world, block_pos(16, -62, -32), water_id);
    set_test_block(&mut world, block_pos(17, -64, -32), tip_id);
    set_test_block(&mut world, block_pos(17, -63, -32), stone_id);
    set_test_block(&mut world, block_pos(17, -62, -32), lava_id);
    set_test_block(&mut world, block_pos(18, -64, -32), waterlogged_tip_id);

    assert_eq!(
        level_event_particle_context(&world, &level_event_at(1504, 16, -64, -32))
            .dripstone_drip_particle,
        Some(LevelEventDripstoneDripParticle::Water)
    );
    assert_eq!(
        level_event_particle_context(&world, &level_event_at(1504, 17, -64, -32))
            .dripstone_drip_particle,
        Some(LevelEventDripstoneDripParticle::Lava)
    );
    assert_eq!(
        level_event_particle_context(&world, &level_event_at(1504, 18, -64, -32))
            .dripstone_drip_particle,
        None
    );

    let mut default_overworld = WorldStore::new();
    default_overworld
        .insert_level_chunk_with_light(synthetic_native_level_chunk_packet())
        .unwrap();
    set_test_block(&mut default_overworld, block_pos(19, -64, -32), tip_id);
    set_test_block(&mut default_overworld, block_pos(19, -63, -32), stone_id);
    assert_eq!(
        level_event_particle_context(&default_overworld, &level_event_at(1504, 19, -64, -32))
            .dripstone_drip_particle,
        Some(LevelEventDripstoneDripParticle::Water)
    );

    let mut nether = WorldStore::new();
    let mut login = protocol_play_login(1);
    login.levels = vec!["minecraft:the_nether".to_string()];
    login.common_spawn_info.dimension_type_id = 1;
    login.common_spawn_info.dimension = "minecraft:the_nether".to_string();
    nether.apply_login(&login);
    nether
        .insert_level_chunk_with_light(synthetic_native_level_chunk_packet())
        .unwrap();
    set_test_block(&mut nether, block_pos(16, 0, -32), tip_id);
    set_test_block(&mut nether, block_pos(16, 1, -32), stone_id);
    assert_eq!(
        level_event_particle_context(&nether, &level_event_at(1504, 16, 0, -32))
            .dripstone_drip_particle,
        Some(LevelEventDripstoneDripParticle::Lava)
    );
}

#[test]
fn plant_growth_level_event_context_uses_bonemeal_particle_branches() {
    let water_id = vanilla_block_state_id("minecraft:water", [("level", "0")]);
    let grass_id = vanilla_block_state_id("minecraft:grass_block", [("snowy", "false")]);
    let rooted_dirt_id = vanilla_block_state_id("minecraft:rooted_dirt", []);
    let short_grass_id = vanilla_block_state_id("minecraft:short_grass", []);
    let bottom_slab_id = vanilla_block_state_id(
        "minecraft:oak_slab",
        [("type", "bottom"), ("waterlogged", "false")],
    );
    let composter_6_id = vanilla_block_state_id("minecraft:composter", [("level", "6")]);
    let stone_id = vanilla_block_state_id("minecraft:stone", []);

    let mut world = WorldStore::new();
    world
        .insert_level_chunk_with_light(synthetic_native_level_chunk_packet())
        .unwrap();

    set_test_block(&mut world, block_pos(16, -63, -32), water_id);
    set_test_block(&mut world, block_pos(16, -64, -32), stone_id);
    let mut water_support = LevelEventGrowthParticleSupport::empty();
    water_support.insert(0, 0);
    assert_eq!(
        level_event_particle_context(&world, &level_event_at_with_data(1505, 16, -63, -32, 15))
            .growth_particles,
        Some(LevelEventGrowthParticleContext {
            pos: block_pos(16, -63, -32),
            mode: LevelEventGrowthParticleMode::WideNoFloating {
                support: water_support
            },
        })
    );

    set_test_block(&mut world, block_pos(17, -64, -32), grass_id);
    let mut grass_support = LevelEventGrowthParticleSupport::empty();
    grass_support.insert(-1, 0);
    grass_support.insert(0, 0);
    assert_eq!(
        level_event_particle_context(&world, &level_event_at_with_data(1505, 17, -64, -32, 15))
            .growth_particles,
        Some(LevelEventGrowthParticleContext {
            pos: block_pos(17, -63, -32),
            mode: LevelEventGrowthParticleMode::WideNoFloating {
                support: grass_support
            },
        })
    );

    set_test_block(&mut world, block_pos(18, -63, -32), rooted_dirt_id);
    assert_eq!(
        level_event_particle_context(&world, &level_event_at_with_data(1505, 18, -63, -32, 15))
            .growth_particles,
        Some(LevelEventGrowthParticleContext {
            pos: block_pos(18, -64, -32),
            mode: LevelEventGrowthParticleMode::InBlock { spread_height: 1.0 },
        })
    );

    set_test_block(&mut world, block_pos(19, -63, -32), stone_id);
    assert_eq!(
        level_event_particle_context(&world, &level_event_at_with_data(1505, 19, -63, -32, 15))
            .growth_particles,
        None
    );

    set_test_block(&mut world, block_pos(20, -63, -32), short_grass_id);
    assert_eq!(
        level_event_particle_context(&world, &level_event_at_with_data(1505, 20, -63, -32, 15))
            .growth_particles,
        Some(LevelEventGrowthParticleContext {
            pos: block_pos(20, -63, -32),
            mode: LevelEventGrowthParticleMode::InBlock {
                spread_height: 13.0 / 16.0
            },
        })
    );

    set_test_block(&mut world, block_pos(21, -63, -32), bottom_slab_id);
    assert_eq!(
        level_event_particle_context(&world, &level_event_at_with_data(2011, 21, -63, -32, 3))
            .in_block_particle_spread_height,
        Some(0.5)
    );

    set_test_block(&mut world, block_pos(22, -63, -32), composter_6_id);
    assert_eq!(
        level_event_particle_context(&world, &level_event_at_with_data(1500, 22, -63, -32, 1))
            .composter_fill_center_shape_max_y,
        Some(13.0 / 16.0)
    );
    assert_eq!(
        level_event_particle_context(&world, &level_event_at_with_data(1500, 23, -63, -32, 1))
            .composter_fill_center_shape_max_y,
        Some(1.0)
    );
}

#[test]
fn projectile_power_updates_world_entity_state_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::ProjectilePower(
        ProjectilePower {
            entity_id: 123,
            acceleration_power: 0.75,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ProjectilePower(
        ProjectilePower {
            entity_id: 456,
            acceleration_power: 0.25,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ProjectilePower(
        ProjectilePower {
            entity_id: 404,
            acceleration_power: 0.5,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_type(
        123,
        VANILLA_ENTITY_TYPE_FIREBALL_ID,
    ));
    world.apply_add_entity(protocol_add_entity_with_type(456, 7));
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );
    assert_eq!(
        world.hurting_projectile(123),
        Some(bbb_world::HurtingProjectileState {
            acceleration_power: 0.75,
        })
    );
    assert_eq!(world.hurting_projectile(456), None);
    assert_eq!(world.counters().projectile_power_packets, 3);
    assert_eq!(world.counters().projectile_power_updates_applied, 1);
    assert_eq!(world.counters().projectile_power_updates_ignored, 2);
    assert_eq!(
        world.last_projectile_power_update(),
        Some(&bbb_world::ProjectilePowerUpdateState {
            entity_id: 404,
            acceleration_power: 0.5,
            applied: false,
        })
    );
}

#[test]
fn debug_game_events_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(8);
    tx.try_send(NetEvent::Play(PlayClientbound::DebugBlockValue(
        DebugBlockValue {
            pos: ProtocolBlockPos { x: 1, y: 64, z: -2 },
            raw_update_payload: vec![5, 1, 0xaa],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DebugChunkValue(
        DebugChunkValue {
            pos: ProtocolChunkPos { x: 3, z: -4 },
            raw_update_payload: vec![7, 0],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DebugEntityValue(
        DebugEntityValue {
            entity_id: 123,
            raw_update_payload: vec![9, 1, 0xbb],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DebugEvent(DebugEvent {
        raw_event_payload: vec![4, 0xcc],
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DebugSample(DebugSample {
        sample: vec![100, -50],
        sample_type: RemoteDebugSampleType::TickTime,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::GameRuleValues(
        GameRuleValues {
            values: vec![
                GameRuleValue {
                    rule: "minecraft:do_daylight_cycle".to_string(),
                    value: "false".to_string(),
                },
                GameRuleValue {
                    rule: "minecraft:random_tick_speed".to_string(),
                    value: "3".to_string(),
                },
            ],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::GameTestHighlightPos(
        GameTestHighlightPos {
            absolute_pos: ProtocolBlockPos {
                x: -10,
                y: 70,
                z: 22,
            },
            relative_pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::TestInstanceBlockStatus(
        TestInstanceBlockStatus {
            status: "Ready".to_string(),
            size: Some(ProtocolVec3i { x: 3, y: 4, z: 5 }),
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        8
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.debug_block_value_packets, 1);
    assert_eq!(world_counters.debug_chunk_value_packets, 1);
    assert_eq!(world_counters.debug_entity_value_packets, 1);
    assert_eq!(world_counters.debug_event_packets, 1);
    assert_eq!(world_counters.debug_sample_packets, 1);
    assert_eq!(world_counters.game_rule_value_packets, 1);
    assert_eq!(world_counters.game_test_highlight_pos_packets, 1);
    assert_eq!(world_counters.test_instance_block_status_packets, 1);
    assert_eq!(
        world.last_debug_block_value(),
        Some(&bbb_world::DebugBlockValueState {
            pos: BlockPos { x: 1, y: 64, z: -2 },
            raw_update_payload_len: 3,
        })
    );
    assert_eq!(
        world.last_debug_chunk_value(),
        Some(&bbb_world::DebugChunkValueState {
            pos: ChunkPos { x: 3, z: -4 },
            raw_update_payload_len: 2,
        })
    );
    assert_eq!(
        world.last_debug_entity_value(),
        Some(&bbb_world::DebugEntityValueState {
            entity_id: 123,
            raw_update_payload_len: 3,
        })
    );
    assert_eq!(
        world.last_debug_event(),
        Some(&bbb_world::DebugEventState {
            raw_event_payload_len: 2,
        })
    );
    assert_eq!(
        world.last_debug_sample(),
        Some(&bbb_world::DebugSampleState {
            sample_len: 2,
            sample_type: "tick_time".to_string(),
        })
    );
    assert_eq!(
        world.last_game_rule_values(),
        Some(&bbb_world::GameRuleValuesState {
            values: vec![
                bbb_world::GameRuleValueState {
                    rule: "minecraft:do_daylight_cycle".to_string(),
                    value: "false".to_string(),
                },
                bbb_world::GameRuleValueState {
                    rule: "minecraft:random_tick_speed".to_string(),
                    value: "3".to_string(),
                },
            ],
        })
    );
    assert_eq!(
        world.last_game_test_highlight_pos(),
        Some(&bbb_world::GameTestHighlightPosState {
            absolute_pos: BlockPos {
                x: -10,
                y: 70,
                z: 22,
            },
            relative_pos: BlockPos { x: 1, y: 2, z: 3 },
        })
    );
    assert_eq!(
        world.last_test_instance_block_status(),
        Some(&bbb_world::TestInstanceBlockStatusState {
            status: "Ready".to_string(),
            size: Some(bbb_world::DebugVec3iState { x: 3, y: 4, z: 5 }),
        })
    );
}

#[test]
fn game_events_emit_local_player_sounds_and_elder_guardian_particles() {
    let (tx, mut rx) = mpsc::channel(4);
    for (event_id, param) in [(6, 0.0), (9, 0.0), (10, 1.75), (10, 0.0)] {
        tx.try_send(NetEvent::Play(PlayClientbound::GameEvent(
            bbb_protocol::packets::GameEvent { event_id, param },
        )))
        .unwrap();
    }

    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 4.0,
            y: 70.0,
            z: -1.0,
        },
        ..LocalPlayerPoseState::default()
    });
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
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
        4
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 3);
    let expected = [
        (
            "minecraft:entity.arrow.hit_player",
            AudioCategory::Players,
            [4.0, 71.62, -1.0],
            0.18,
            0.45,
        ),
        (
            "minecraft:entity.puffer_fish.sting",
            AudioCategory::Neutral,
            [4.0, 70.0, -1.0],
            1.0,
            1.0,
        ),
        (
            "minecraft:entity.elder_guardian.curse",
            AudioCategory::Hostile,
            [4.0, 70.0, -1.0],
            1.0,
            1.0,
        ),
    ];
    for (command, (event_id, category, position, volume, pitch)) in
        audio.commands.iter().zip(expected)
    {
        let AudioCommand::PlayPositionedSound(command) = command else {
            panic!("expected positioned sound, got {command:?}");
        };
        assert_eq!(command.sound.event_id, event_id);
        assert_eq!(command.category, category);
        assert_close(command.position[0] as f32, position[0] as f32);
        assert_close(command.position[1] as f32, position[1] as f32);
        assert_close(command.position[2] as f32, position[2] as f32);
        assert_close(command.packet_volume, volume);
        assert_close(command.packet_pitch, pitch);
        assert_eq!(command.seed, 0);
    }

    assert_eq!(particles.packets.len(), 2);
    assert_eq!(particles.contexts.len(), 2);
    for packet in &particles.packets {
        assert_eq!(
            packet.particle.particle_type_id,
            crate::particle_runtime::ELDER_GUARDIAN_PARTICLE_TYPE_ID
        );
        assert_eq!(
            packet.position,
            ProtocolVec3d {
                x: 4.0,
                y: 70.0,
                z: -1.0,
            }
        );
        assert_eq!(packet.offset, ProtocolVec3d::default());
        assert_eq!(packet.max_speed, 1.0);
        assert_eq!(packet.count, 0);
        assert!(!packet.override_limiter);
        assert!(!packet.always_show);
    }
    for context in &particles.contexts {
        let camera_position = context.camera_position.expect("camera position");
        assert_close(camera_position[0] as f32, 4.0);
        assert_close(camera_position[1] as f32, 71.62);
        assert_close(camera_position[2] as f32, -1.0);
    }
    assert_eq!(particles.batches.len(), 2);
    assert_eq!(world.counters().game_event_packets, 4);
}
