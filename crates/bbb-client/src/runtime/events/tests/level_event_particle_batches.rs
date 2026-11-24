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
        explosion_particle: ParticlePayload {
            particle_type_id: VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
            raw_options: Vec::new(),
        },
        explosion_sound: SoundEventHolder::Reference { registry_id: 41 },
        block_particles: vec![bbb_protocol::packets::WeightedExplosionParticle {
            particle: ParticlePayload {
                particle_type_id: VANILLA_PARTICLE_TYPE_CLOUD_ID,
                raw_options: Vec::new(),
            },
            scaling: 0.5,
            speed: 1.25,
            weight: 3,
        }],
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
                particle_type_id: VANILLA_PARTICLE_TYPE_HEART_ID,
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
        Some(&bbb_client_world::ExplosionEventState {
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
            explosion_particle: ParticlePayload {
                particle_type_id: VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
                raw_options: Vec::new(),
            },
            explosion_sound: SoundEventHolder::Reference { registry_id: 41 },
            block_particles: vec![bbb_protocol::packets::WeightedExplosionParticle {
                particle: ParticlePayload {
                    particle_type_id: VANILLA_PARTICLE_TYPE_CLOUD_ID,
                    raw_options: Vec::new(),
                },
                scaling: 0.5,
                speed: 1.25,
                weight: 3,
            }],
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
        Some(&bbb_client_world::LevelParticlesEventState {
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
            particle_type_id: VANILLA_PARTICLE_TYPE_HEART_ID,
            raw_options_len: 2,
        })
    );
}

#[test]
fn level_particles_spawn_one_particle_near_the_camera() {
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
            particle_type_id: VANILLA_PARTICLE_TYPE_CLOUD_ID,
            raw_options: vec![0xcc],
        },
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelParticles(packet)))
        .unwrap();
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 10.0,
            y: 63.0,
            z: -3.0,
        },
        ..LocalPlayerPoseState::default()
    });
    let mut counters = NetCounters::default();
    let mut particles = TestParticles::new();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            particles.target(),
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    // `count == 0`: one particle whose velocity is the offset times speed;
    // the camera is within `LevelRenderer.addParticleInternal`'s 32 blocks.
    assert_eq!(
        particles.live_kinds(&world),
        vec![bbb_client_world::particles::ParticleKind::CLOUD]
    );
    assert_eq!(world.counters().level_particles_packets, 1);
    assert_eq!(world.last_level_particles().unwrap().count, 0);
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
        Some(bbb_client_world::HurtingProjectileState {
            acceleration_power: 0.75,
        })
    );
    assert_eq!(world.hurting_projectile(456), None);
    assert_eq!(world.counters().projectile_power_packets, 3);
    assert_eq!(world.counters().projectile_power_updates_applied, 1);
    assert_eq!(world.counters().projectile_power_updates_ignored, 2);
    assert_eq!(
        world.last_projectile_power_update(),
        Some(&bbb_client_world::ProjectilePowerUpdateState {
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
            update: bbb_protocol::packets::DebugSubscriptionUpdate {
                subscription_id: 6,
                value: Some(
                    bbb_protocol::packets::DebugSubscriptionValue::EntityBlockIntersection(
                        bbb_protocol::packets::DebugEntityBlockIntersection::InAir,
                    ),
                ),
            },
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DebugChunkValue(
        DebugChunkValue {
            pos: ProtocolChunkPos { x: 3, z: -4 },
            update: bbb_protocol::packets::DebugSubscriptionUpdate {
                subscription_id: 11,
                value: None,
            },
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DebugEntityValue(
        DebugEntityValue {
            entity_id: 123,
            update: bbb_protocol::packets::DebugSubscriptionUpdate {
                subscription_id: 3,
                value: None,
            },
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DebugEvent(DebugEvent {
        event: bbb_protocol::packets::DebugSubscriptionEvent {
            subscription_id: 14,
            value: bbb_protocol::packets::DebugSubscriptionValue::NeighborUpdate(
                ProtocolBlockPos { x: 4, y: 5, z: 6 },
            ),
        },
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
            status: bbb_protocol::RichTextComponent::plain("Ready"),
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
    assert_eq!(world_counters.game_test_highlights_tracked, 1);
    assert_eq!(world_counters.test_instance_block_status_packets, 1);
    assert_eq!(
        world.last_debug_block_value(),
        Some(&bbb_client_world::DebugBlockValueState {
            pos: BlockPos { x: 1, y: 64, z: -2 },
            subscription_id: 6,
            value_present: true,
        })
    );
    assert_eq!(
        world.last_debug_chunk_value(),
        Some(&bbb_client_world::DebugChunkValueState {
            pos: ChunkPos { x: 3, z: -4 },
            subscription_id: 11,
            value_present: false,
        })
    );
    assert_eq!(
        world.last_debug_entity_value(),
        Some(&bbb_client_world::DebugEntityValueState {
            entity_id: 123,
            subscription_id: 3,
            value_present: false,
        })
    );
    assert_eq!(
        world.last_debug_event(),
        Some(&bbb_client_world::DebugEventState {
            subscription_id: 14
        })
    );
    assert_eq!(
        world.last_debug_sample(),
        Some(&bbb_client_world::DebugSampleState {
            sample_len: 2,
            sample_type: "tick_time".to_string(),
        })
    );
    assert_eq!(
        world.last_game_rule_values(),
        Some(&bbb_client_world::GameRuleValuesState {
            values: vec![
                bbb_client_world::GameRuleValueState {
                    rule: "minecraft:do_daylight_cycle".to_string(),
                    value: "false".to_string(),
                },
                bbb_client_world::GameRuleValueState {
                    rule: "minecraft:random_tick_speed".to_string(),
                    value: "3".to_string(),
                },
            ],
        })
    );
    assert_eq!(
        world.last_game_test_highlight_pos(),
        Some(&bbb_client_world::GameTestHighlightPosState {
            absolute_pos: BlockPos {
                x: -10,
                y: 70,
                z: 22,
            },
            relative_pos: BlockPos { x: 1, y: 2, z: 3 },
        })
    );
    assert_eq!(world.game_test_highlight_markers().len(), 1);
    let marker = &world.game_test_highlight_markers()[&BlockPos {
        x: -10,
        y: 70,
        z: 22,
    }];
    assert_eq!(marker.relative_pos, BlockPos { x: 1, y: 2, z: 3 });
    assert!(marker.remove_at_monotonic_millis >= 10_000);
    assert_eq!(
        world.last_test_instance_block_status(),
        Some(&bbb_client_world::TestInstanceBlockStatusState {
            status: bbb_protocol::RichTextComponent::plain("Ready"),
            size: Some(bbb_client_world::DebugVec3iState { x: 3, y: 4, z: 5 }),
        })
    );
}

#[test]
fn game_test_highlight_network_apply_uses_monotonic_deadline() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::GameTestHighlightPos(
        GameTestHighlightPos {
            absolute_pos: ProtocolBlockPos { x: 4, y: 5, z: 6 },
            relative_pos: ProtocolBlockPos { x: -7, y: 8, z: -9 },
        },
    )))
    .unwrap();
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    let marker = &world.game_test_highlight_markers()[&BlockPos { x: 4, y: 5, z: 6 }];
    assert_eq!(marker.relative_pos, BlockPos { x: -7, y: 8, z: -9 });
    assert!(marker.remove_at_monotonic_millis > 10_000);
}

#[test]
fn game_events_emit_local_player_sounds_and_elder_guardian_particles() {
    let (tx, mut rx) = mpsc::channel(5);
    use bbb_protocol::game_event_types::{
        VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID,
        VANILLA_CLIENT_GAME_EVENT_PLAY_ARROW_HIT_SOUND_ID,
        VANILLA_CLIENT_GAME_EVENT_PUFFER_FISH_STING_ID,
    };

    for (event_id, param) in [
        (VANILLA_CLIENT_GAME_EVENT_PLAY_ARROW_HIT_SOUND_ID, 0.0),
        (VANILLA_CLIENT_GAME_EVENT_PUFFER_FISH_STING_ID, 0.0),
        (VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID, 0.5),
        (VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID, 1.5),
        (VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID, 0.0),
    ] {
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
    let mut particles = TestParticles::new();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            particles.target(),
            None,
            &mut level_event_sound_random,
        ),
        5
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

    assert_eq!(
        particles.live_kinds(&world),
        vec![bbb_client_world::particles::ParticleKind::ELDER_GUARDIAN; 3]
    );
    assert_eq!(world.counters().game_event_packets, 5);
}
