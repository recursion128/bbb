use super::*;

#[tokio::test]
async fn probe_applies_debug_game_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::DebugBlockValue(DebugBlockValue {
            pos: ProtocolBlockPos { x: 1, y: 64, z: -2 },
            update: bbb_protocol::packets::DebugSubscriptionUpdate {
                subscription_id: 6,
                value: Some(
                    bbb_protocol::packets::DebugSubscriptionValue::EntityBlockIntersection(
                        bbb_protocol::packets::DebugEntityBlockIntersection::InAir,
                    ),
                ),
            },
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::DebugChunkValue(DebugChunkValue {
            pos: ProtocolChunkPos { x: 3, z: -4 },
            update: bbb_protocol::packets::DebugSubscriptionUpdate {
                subscription_id: 11,
                value: None,
            },
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::DebugEntityValue(DebugEntityValue {
            entity_id: 123,
            update: bbb_protocol::packets::DebugSubscriptionUpdate {
                subscription_id: 3,
                value: None,
            },
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::DebugEvent(DebugEvent {
            event: bbb_protocol::packets::DebugSubscriptionEvent {
                subscription_id: 14,
                value: bbb_protocol::packets::DebugSubscriptionValue::NeighborUpdate(
                    ProtocolBlockPos { x: 4, y: 5, z: 6 },
                ),
            },
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::DebugSample(DebugSample {
            sample: vec![100, -50],
            sample_type: RemoteDebugSampleType::TickTime,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::GameRuleValues(GameRuleValues {
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
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::GameTestHighlightPos(
            GameTestHighlightPos {
                absolute_pos: ProtocolBlockPos {
                    x: -10,
                    y: 70,
                    z: 22,
                },
                relative_pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
            },
        ))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::TestInstanceBlockStatus(
            TestInstanceBlockStatus {
                status: bbb_protocol::RichTextComponent::plain("Ready"),
                size: Some(ProtocolVec3i { x: 3, y: 4, z: 5 }),
            },
        ))
        .await
        .unwrap();

    let report = probe.finish(8, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(report.world_counters.debug_block_value_packets, 1);
    assert_eq!(report.world_counters.debug_chunk_value_packets, 1);
    assert_eq!(report.world_counters.debug_entity_value_packets, 1);
    assert_eq!(report.world_counters.debug_event_packets, 1);
    assert_eq!(report.world_counters.debug_sample_packets, 1);
    assert_eq!(report.world_counters.game_rule_value_packets, 1);
    assert_eq!(report.world_counters.game_test_highlight_pos_packets, 1);
    assert_eq!(report.world_counters.test_instance_block_status_packets, 1);
    assert_eq!(
        report.world.last_debug_block_value(),
        Some(&bbb_client_world::DebugBlockValueState {
            pos: BlockPos { x: 1, y: 64, z: -2 },
            subscription_id: 6,
            value_present: true,
        })
    );
    assert_eq!(
        report
            .world
            .last_game_rule_values()
            .map(|state| state.len()),
        Some(2)
    );
    assert_eq!(
        report.world.last_test_instance_block_status(),
        Some(&bbb_client_world::TestInstanceBlockStatusState {
            status: bbb_protocol::RichTextComponent::plain("Ready"),
            size: Some(bbb_client_world::DebugVec3iState { x: 3, y: 4, z: 5 }),
        })
    );
}

#[tokio::test]
async fn probe_applies_award_stats_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::AwardStats(AwardStats {
            stats: vec![
                StatUpdate {
                    stat_type_id: VANILLA_STAT_TYPE_CUSTOM_ID,
                    value_id: 10,
                    amount: 3,
                },
                StatUpdate {
                    stat_type_id: VANILLA_STAT_TYPE_BLOCK_MINED_ID,
                    value_id: 4,
                    amount: 11,
                },
            ],
        }))
        .await
        .unwrap();

    let report = probe.finish(8, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(
        report.world.stat_value(VANILLA_STAT_TYPE_CUSTOM_ID, 10),
        Some(3)
    );
    assert_eq!(
        report.world.stat_value(VANILLA_STAT_TYPE_BLOCK_MINED_ID, 4),
        Some(11)
    );
    assert_eq!(report.world_counters.award_stats_packets, 1);
    assert_eq!(report.world_counters.award_stats_entries_received, 2);
    assert_eq!(report.world_counters.last_award_stats_entry_count, 2);
    assert_eq!(report.world_counters.stats_tracked, 2);
}

#[tokio::test]
async fn probe_applies_item_cooldown_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::Cooldown(Cooldown {
            cooldown_group: "minecraft:ender_pearl".to_string(),
            duration: 20,
        }))
        .await
        .unwrap();

    let cooldown = probe
        .world
        .cooldown("minecraft:ender_pearl")
        .expect("cooldown should be tracked after positive duration");
    assert_eq!(cooldown.duration, 20);
    assert_eq!(cooldown.remaining_ticks, 20);
    assert_eq!(
        probe
            .world
            .item_cooldown_percent("minecraft:ender_pearl", 0.5),
        0.975
    );

    probe
        .handle_play_packet(PlayClientbound::Cooldown(Cooldown {
            cooldown_group: "minecraft:ender_pearl".to_string(),
            duration: 0,
        }))
        .await
        .unwrap();

    let report = probe.finish(2, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert!(report.world.cooldown("minecraft:ender_pearl").is_none());
    assert_eq!(
        report
            .world
            .item_cooldown_percent("minecraft:ender_pearl", 0.0),
        0.0
    );
    assert_eq!(report.world_counters.cooldown_packets, 2);
    assert_eq!(report.world_counters.cooldowns_tracked, 0);
}

#[tokio::test]
async fn probe_applies_tag_query_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::TagQuery(TagQuery {
            transaction_id: 12,
            tag_present: true,
            raw_nbt: vec![10, 0],
        }))
        .await
        .unwrap();

    let report = probe.finish(1, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(
        report.world.last_tag_query(),
        Some(&bbb_client_world::TagQueryResponseState {
            transaction_id: 12,
            tag_present: true,
            raw_nbt: vec![10, 0],
        })
    );
    assert_eq!(report.world.last_tag_query().unwrap().raw_nbt_len(), 2);
    assert_eq!(report.world_counters.tag_query_packets, 1);
}

#[tokio::test]
async fn probe_applies_command_suggestion_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::CustomChatCompletions(
            CustomChatCompletions {
                action: CustomChatCompletionsAction::Set,
                entries: vec!["/spawn".to_string(), "/warp".to_string()],
            },
        ))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::CustomChatCompletions(
            CustomChatCompletions {
                action: CustomChatCompletionsAction::Remove,
                entries: vec!["/spawn".to_string()],
            },
        ))
        .await
        .unwrap();
    assert_eq!(probe.world.next_command_suggestion_request("/give").id, 0);
    probe
        .handle_play_packet(PlayClientbound::CommandSuggestions(CommandSuggestions {
            id: 0,
            start: 1,
            length: 4,
            suggestions: vec![
                CommandSuggestion {
                    text: "give".into(),
                    tooltip: Some("Run give".to_string()),
                    tooltip_styled: None,
                    tooltip_component: None,
                },
                CommandSuggestion {
                    text: "gamemode".into(),
                    tooltip: None,
                    tooltip_styled: None,
                    tooltip_component: None,
                },
            ],
        }))
        .await
        .unwrap();

    let report = probe.finish(3, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let completions = report.world.custom_chat_completions();
    assert_eq!(completions.len(), 1);
    assert!(completions.contains("/warp"));
    assert_eq!(
        report.world.last_custom_chat_completion_update(),
        Some(&bbb_client_world::CustomChatCompletionUpdateState {
            action: "remove".to_string(),
            entries: 1,
        })
    );

    let suggestions = report
        .world
        .command_suggestions_by_id(0)
        .expect("suggestions tracked by id");
    assert_eq!(suggestions.start, 1);
    assert_eq!(suggestions.length, 4);
    assert_eq!(suggestions.suggestions.len(), 2);
    assert_eq!(suggestions.suggestions[0].text, "give");
    assert_eq!(
        suggestions.suggestions[0].tooltip.as_deref(),
        Some("Run give")
    );
    assert_eq!(suggestions.suggestions[1].text, "gamemode");
    assert_eq!(suggestions.suggestions[1].tooltip, None);
    assert_eq!(report.world.last_command_suggestions(), Some(suggestions));
    assert_eq!(report.world_counters.custom_chat_completion_packets, 2);
    assert_eq!(report.world_counters.custom_chat_completions_tracked, 1);
    assert_eq!(report.world_counters.command_suggestion_packets, 1);
    assert_eq!(report.world_counters.command_suggestion_entries_tracked, 2);
}

#[tokio::test]
async fn probe_applies_command_tree_and_update_tags_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::Commands(command_tree_packet("say")))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::UpdateTags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:logs".to_string(),
                    entries: vec![5, 6, 7],
                }],
            }],
        }))
        .await
        .unwrap();

    let report = probe.finish(2, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let commands = report.world.commands();
    assert_eq!(commands.root_index, 0);
    assert_eq!(commands.nodes.len(), 3);
    assert_eq!(commands.nodes[1].name.as_deref(), Some("say"));
    assert_eq!(commands.nodes[2].name.as_deref(), Some("message"));
    assert_eq!(
        commands.nodes[2].parser.as_ref().unwrap().name,
        "minecraft:message"
    );
    assert_eq!(
        commands.nodes[2].suggestions.as_deref(),
        Some("minecraft:ask_server")
    );
    assert!(commands.command_requires_signed_arguments("say hello world"));
    assert_eq!(
        report.world.registry_tags("minecraft:item").unwrap().tags["minecraft:logs"],
        vec![5, 6, 7]
    );
    assert_eq!(report.world_counters.command_tree_packets, 1);
    assert_eq!(report.world_counters.command_nodes_tracked, 3);
    assert_eq!(report.world_counters.command_literal_nodes_tracked, 1);
    assert_eq!(report.world_counters.command_argument_nodes_tracked, 1);
    assert_eq!(report.world_counters.command_executable_nodes_tracked, 1);
    assert_eq!(report.world_counters.command_restricted_nodes_tracked, 1);
    assert_eq!(report.world_counters.update_tags_packets, 1);
    assert_eq!(report.world_counters.last_update_tags_registry_count, 1);
    assert_eq!(report.world_counters.last_update_tags_total_tag_count, 1);
    assert_eq!(report.world_counters.last_update_tags_total_value_count, 3);
    assert_eq!(report.world_counters.tag_registries_tracked, 1);
    assert_eq!(report.world_counters.tags_tracked, 1);
    assert_eq!(report.world_counters.tag_entries_tracked, 3);
}

#[tokio::test]
async fn probe_applies_map_item_data_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::MapItemData(MapItemData {
            map_id: 12,
            scale: 1,
            locked: false,
            decorations: Some(vec![MapDecoration {
                type_id: 4,
                x: -8,
                y: 9,
                rot: 3,
                name: Some(bbb_protocol::RichTextComponent::plain("Camp")),
            }]),
            color_patch: Some(MapColorPatch {
                start_x: 5,
                start_y: 6,
                width: 2,
                height: 1,
                colors: vec![11, 12],
            }),
        }))
        .await
        .unwrap();

    let report = probe.finish(1, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let map = report.world.map_item(12).expect("map state is tracked");
    assert_eq!(map.decorations.len(), 1);
    assert_eq!(
        map.decorations[0]
            .name
            .as_ref()
            .map(bbb_protocol::RichTextComponent::text),
        Some("Camp")
    );
    assert_eq!(map.colors[5 + 6 * 128], 11);
    assert_eq!(map.colors[6 + 6 * 128], 12);
    assert_eq!(
        report.world.last_map_color_patch(),
        Some(&bbb_client_world::LastMapColorPatchState {
            map_id: 12,
            start_x: 5,
            start_y: 6,
            width: 2,
            height: 1,
        })
    );
    assert_eq!(report.world_counters.map_item_data_packets, 1);
    assert_eq!(report.world_counters.maps_tracked, 1);
    assert_eq!(report.world_counters.map_decorations_tracked, 1);
    assert_eq!(report.world_counters.map_color_patches_applied, 1);
}

#[tokio::test]
async fn probe_applies_audio_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(123)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::Sound(SoundEvent {
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
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SoundEntity(SoundEntityEvent {
            sound: SoundEventHolder::Direct {
                location: "minecraft:entity.cat.ambient".to_string(),
                fixed_range: Some(32.0),
            },
            source: SoundSource::Neutral,
            entity_id: 123,
            volume: 1.0,
            pitch: 0.5,
            seed: -9,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::StopSound(StopSound {
            source: Some(SoundSource::Music),
            name: Some("minecraft:music.menu".to_string()),
        }))
        .await
        .unwrap();

    let report = probe.finish(4, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(report.world_counters.sound_packets, 1);
    assert_eq!(
        report.world.last_sound(),
        Some(&bbb_client_world::SoundEventState {
            sound: bbb_client_world::SoundHolderState {
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
    assert_eq!(report.world_counters.sound_entity_packets, 1);
    assert_eq!(report.world_counters.sound_entity_events_applied, 1);
    assert_eq!(report.world_counters.sound_entity_events_ignored, 0);
    assert_eq!(
        report.world.last_sound_entity(),
        Some(&bbb_client_world::SoundEntityEventState {
            sound: bbb_client_world::SoundHolderState {
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
    assert_eq!(report.world_counters.stop_sound_packets, 1);
    assert_eq!(
        report.world.last_stop_sound(),
        Some(&bbb_client_world::StopSoundEventState {
            source: Some("music".to_string()),
            name: Some("minecraft:music.menu".to_string()),
        })
    );
}

#[tokio::test]
async fn probe_applies_visual_effect_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    probe.world.set_local_player_pose(LocalPlayerPoseState {
        delta_movement: ProtocolVec3d {
            x: 0.5,
            y: -0.25,
            z: 1.0,
        },
        ..LocalPlayerPoseState::default()
    });

    probe
        .handle_play_packet(PlayClientbound::Explosion(Explosion {
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
                particle_type_id: bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
                raw_options: Vec::new(),
            },
            explosion_sound: bbb_protocol::packets::SoundEventHolder::Reference { registry_id: 41 },
            block_particles: vec![bbb_protocol::packets::WeightedExplosionParticle {
                particle: ParticlePayload {
                    particle_type_id: bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_CLOUD_ID,
                    raw_options: Vec::new(),
                },
                scaling: 0.5,
                speed: 1.25,
                weight: 3,
            }],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::LevelParticles(LevelParticles {
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
        }))
        .await
        .unwrap();

    let report = probe.finish(2, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(report.world_counters.explosion_packets, 1);
    assert_eq!(
        report.world.last_explosion(),
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
                particle_type_id: bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
                raw_options: Vec::new(),
            },
            explosion_sound: bbb_protocol::packets::SoundEventHolder::Reference { registry_id: 41 },
            block_particles: vec![bbb_protocol::packets::WeightedExplosionParticle {
                particle: ParticlePayload {
                    particle_type_id: bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_CLOUD_ID,
                    raw_options: Vec::new(),
                },
                scaling: 0.5,
                speed: 1.25,
                weight: 3,
            }],
        })
    );
    assert_eq!(
        report.world.local_player_pose().unwrap().delta_movement,
        ProtocolVec3d {
            x: 0.75,
            y: -0.75,
            z: 2.5,
        }
    );
    assert_eq!(report.world_counters.level_particles_packets, 1);
    assert_eq!(
        report.world.last_level_particles(),
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

#[tokio::test]
async fn probe_chunk_batch_feedback_uses_vanilla_calculator() {
    let (client, mut server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::ChunkBatchStart)
        .await
        .unwrap();
    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "chunk batch start must only update the local calculator"
    );

    probe
        .handle_play_packet(PlayClientbound::ChunkBatchFinished { batch_size: 0 })
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("chunk batch received packet should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_CHUNK_BATCH_RECEIVED);
    let desired = Decoder::new(&payload).read_f32().unwrap();
    assert_eq!(desired, 3.5);
}

#[tokio::test]
async fn probe_play_keepalive_and_ping_send_common_responses() {
    let (client, mut server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::KeepAlive {
            id: 0x1122_3344_5566_7788,
        })
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::Ping { id: 0x0a0b_0c0d })
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("keepalive response should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_KEEP_ALIVE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i64().unwrap(), 0x1122_3344_5566_7788);
    assert!(decoder.is_empty());

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("pong response should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_PONG);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i32().unwrap(), 0x0a0b_0c0d);
    assert!(decoder.is_empty());
}

#[tokio::test]
async fn probe_player_chat_sends_chat_acknowledgement_after_threshold() {
    let (client, mut server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    for index in 0..65 {
        probe
            .handle_play_packet(PlayClientbound::PlayerChat(
                protocol_player_chat_with_signature(
                    index,
                    MessageSignature {
                        bytes: vec![index as u8; 256],
                    },
                ),
            ))
            .await
            .unwrap();
    }

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("chat acknowledgement packet should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_CHAT_ACK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 65);
    assert!(decoder.is_empty());
    assert_eq!(probe.world.counters().player_chat_packets, 65);
    assert_eq!(
        probe.world.counters().player_chat_acknowledgement_packets,
        1
    );
}

#[tokio::test]
async fn probe_applies_delete_and_disguised_chat_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    let signature = MessageSignature {
        bytes: vec![9; 256],
    };
    let expected_signature_checksum = signature.checksum();

    probe
        .handle_play_packet(PlayClientbound::PlayerChat(
            protocol_player_chat_with_signature(0, signature),
        ))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::DeleteChat(DeleteChat {
            message_signature: PackedMessageSignature {
                cache_id: Some(0),
                full_signature: None,
            },
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::DisguisedChat(DisguisedChat {
            message: "server notice".to_string(),
            message_styled: Vec::new(),
            message_component: None,
            chat_type: ChatTypeBound {
                chat_type: ChatTypeHolder::Registry { id: 0 },
                name: "Server".to_string(),
                name_styled: Vec::new(),
                name_component: None,
                target_name: None,
                target_name_styled: None,
                target_name_component: None,
            },
        }))
        .await
        .unwrap();

    let report = probe.finish(3, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let chat = report.world.client_chat();

    assert_eq!(chat.messages.len(), 2);
    assert_eq!(chat.deleted_messages.len(), 1);
    assert_eq!(
        chat.messages[0].kind,
        bbb_client_world::ChatMessageKind::Player
    );
    assert_eq!(chat.messages[0].content, "chat.validation_error");
    assert_eq!(
        chat.messages[0].validation_state,
        bbb_client_world::ChatValidationState::UnknownSender
    );
    assert_eq!(
        chat.messages[1].kind,
        bbb_client_world::ChatMessageKind::Disguised
    );
    assert_eq!(chat.messages[1].content, "server notice");
    assert_eq!(chat.messages[1].sender_name, "Server");

    let deleted = &chat.deleted_messages[0];
    assert_eq!(deleted.cache_id, Some(0));
    assert!(deleted.resolved);
    assert_eq!(
        deleted
            .signature
            .as_ref()
            .map(|signature| signature.checksum),
        Some(expected_signature_checksum)
    );

    assert_eq!(report.world_counters.player_chat_packets, 1);
    assert_eq!(report.world_counters.delete_chat_packets, 1);
    assert_eq!(report.world_counters.disguised_chat_packets, 1);
    assert_eq!(report.world_counters.chat_messages_tracked, 2);
    assert_eq!(report.world_counters.deleted_chat_messages_tracked, 1);
    assert_eq!(report.world_counters.chat_signature_cache_entries, 1);
    assert_eq!(report.world_counters.chat_unknown_packed_signatures, 0);
}

#[tokio::test]
async fn probe_start_configuration_acknowledges_and_resets_configuration_dedup_state() {
    let (client, mut server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    probe.state = ConnectionState::Play;
    probe.play_tick = Some(bbb_client_net::play_tick_interval());
    probe.seen_code_of_conduct = true;
    probe.world.apply_add_entity(protocol_add_entity(55));
    probe.world.set_local_using_item(true);

    probe
        .handle_play_packet(PlayClientbound::StartConfiguration)
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("configuration acknowledgement should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED);
    assert!(payload.is_empty());
    assert_eq!(probe.state, ConnectionState::Configuration);
    assert!(probe.play_tick.is_none());
    assert!(!probe.seen_code_of_conduct);
    assert_eq!(probe.world.entity_count(), 0);
    assert_eq!(
        probe.world.local_player(),
        &bbb_client_world::LocalPlayerState::default()
    );

    probe
        .handle_configuration_packet(packets::ConfigurationClientbound::CodeOfConduct {
            text: "Fresh configuration rules.".to_string(),
        })
        .await
        .unwrap();

    assert!(probe.seen_code_of_conduct);
    assert_eq!(
        probe.world.last_code_of_conduct().unwrap().text,
        "Fresh configuration rules."
    );
    assert_eq!(probe.world.counters().code_of_conduct_packets, 1);
}

#[tokio::test]
async fn probe_start_configuration_flushes_pending_chat_acknowledgement_first() {
    let (client, mut server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    probe.state = ConnectionState::Play;

    probe
        .handle_play_packet(PlayClientbound::PlayerChat(
            protocol_player_chat_with_signature(
                0,
                MessageSignature {
                    bytes: vec![9; 256],
                },
            ),
        ))
        .await
        .unwrap();

    probe
        .handle_play_packet(PlayClientbound::StartConfiguration)
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("chat acknowledgement should be sent first")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_CHAT_ACK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert!(decoder.is_empty());

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("configuration acknowledgement should be sent second")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED);
    assert!(payload.is_empty());
    assert_eq!(probe.state, ConnectionState::Configuration);
    assert_eq!(
        probe
            .world
            .counters()
            .player_chat_acknowledgement_pending_offset,
        0
    );
}

#[tokio::test]
async fn probe_records_nonfatal_chunk_update_errors() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::LevelChunkWithLight(
            synthetic_probe_level_chunk_packet(),
        ))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::BlockEntityData(BlockEntityData {
            pos: ProtocolBlockPos {
                x: 16,
                y: -64,
                z: -32,
            },
            block_entity_type_id:
                bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
            raw_nbt: vec![1],
        }))
        .await
        .unwrap();

    let counters = probe.world.counters();
    assert_eq!(counters.block_entity_updates_received, 1);
    assert_eq!(counters.block_entity_updates_applied, 0);
    let diagnostics = probe.world.apply_diagnostics();
    assert_eq!(diagnostics.apply_errors.len(), 1);
    assert_eq!(diagnostics.apply_errors[0].source, "block_entity_data");
    assert!(
        diagnostics.apply_errors[0].message.contains("nbt byte"),
        "{:?}",
        diagnostics.apply_errors
    );
    assert_eq!(counters.world_apply_errors, 1);

    let report = probe.finish(2, ChunkPos { x: 1, z: -2 }, Vec::new(), None, None);
    assert_eq!(report.world_apply_errors.len(), 1);
    assert!(report.world_apply_errors[0].contains("nbt byte"));
    assert_eq!(report.world_counters.world_apply_errors, 1);
    assert_eq!(report.world.apply_diagnostics().apply_errors.len(), 1);
}

#[tokio::test]
async fn probe_applies_block_entity_sign_text_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::LevelChunkWithLight(
            synthetic_probe_level_chunk_packet(),
        ))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::BlockEntityData(BlockEntityData {
            pos: ProtocolBlockPos {
                x: 16,
                y: -64,
                z: -32,
            },
            block_entity_type_id:
                bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
            raw_nbt: sign_text_nbt(
                ["Front A", "Front B", "Front C", "Front D"],
                ["Back A", "Back B", "Back C", "Back D"],
            ),
        }))
        .await
        .unwrap();

    let report = probe.finish(2, ChunkPos { x: 1, z: -2 }, Vec::new(), None, None);
    let pos = BlockPos {
        x: 16,
        y: -64,
        z: -32,
    };

    assert_eq!(
        report.world.sign_text_lines(pos, true),
        Some([
            "Front A".to_string(),
            "Front B".to_string(),
            "Front C".to_string(),
            "Front D".to_string(),
        ])
    );
    assert_eq!(
        report.world.sign_text_lines(pos, false),
        Some([
            "Back A".to_string(),
            "Back B".to_string(),
            "Back C".to_string(),
            "Back D".to_string(),
        ])
    );
    assert_eq!(report.world_counters.block_entity_updates_applied, 1);
}

#[tokio::test]
async fn probe_records_bad_chunk_and_continues_to_good_chunk() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    let mut bad_chunk = synthetic_probe_level_chunk_packet();
    bad_chunk.chunk_data.section_data = vec![0xff];

    let first_chunk = probe
        .handle_play_packet(PlayClientbound::LevelChunkWithLight(bad_chunk))
        .await
        .unwrap();

    assert_eq!(first_chunk, None);
    assert_eq!(probe.world.counters().world_apply_errors, 1);
    assert_eq!(
        probe.world.apply_diagnostics().apply_errors[0].source,
        "level_chunk_with_light"
    );

    let first_chunk = probe
        .handle_play_packet(PlayClientbound::LevelChunkWithLight(
            synthetic_probe_level_chunk_packet(),
        ))
        .await
        .unwrap();

    assert_eq!(first_chunk, Some(ChunkPos { x: 1, z: -2 }));
    assert_eq!(probe.world.counters().chunks_received, 1);
    assert_eq!(probe.world.counters().world_apply_errors, 1);
}

#[tokio::test]
async fn probe_applies_chunk_view_and_block_updates_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    let chunk_pos = ChunkPos { x: 1, z: -2 };

    probe
        .handle_play_packet(PlayClientbound::SetChunkCacheCenter(SetChunkCacheCenter {
            chunk_x: chunk_pos.x,
            chunk_z: chunk_pos.z,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetChunkCacheRadius(SetChunkCacheRadius {
            radius: 7,
        }))
        .await
        .unwrap();

    let first_chunk = probe
        .handle_play_packet(PlayClientbound::LevelChunkWithLight(
            synthetic_probe_level_chunk_packet(),
        ))
        .await
        .unwrap();

    assert_eq!(first_chunk, Some(chunk_pos));
    assert_eq!(probe.world.chunk_cache_center(), Some(chunk_pos));
    assert_eq!(probe.world.chunk_cache_radius(), Some(7));
    assert_eq!(probe.world.first_chunk(), Some(chunk_pos));
    assert_eq!(probe.world.chunk_positions(), vec![chunk_pos]);

    probe
        .handle_play_packet(PlayClientbound::BlockUpdate(BlockUpdate {
            pos: ProtocolBlockPos {
                x: 16,
                y: -64,
                z: -32,
            },
            block_state_id: 9,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SectionBlocksUpdate(SectionBlocksUpdate {
            section_x: 1,
            section_y: -4,
            section_z: -2,
            updates: vec![
                BlockUpdate {
                    pos: ProtocolBlockPos {
                        x: 17,
                        y: -64,
                        z: -32,
                    },
                    block_state_id: 9,
                },
                BlockUpdate {
                    pos: ProtocolBlockPos {
                        x: 18,
                        y: -64,
                        z: -32,
                    },
                    block_state_id: 9,
                },
            ],
        }))
        .await
        .unwrap();

    for x in 16..=18 {
        assert_eq!(
            probe
                .world
                .probe_block(BlockPos { x, y: -64, z: -32 })
                .unwrap()
                .block_state_id,
            9
        );
    }
    assert_eq!(probe.world.counters().chunks_received, 1);
    assert_eq!(probe.world.counters().chunks_decoded, 1);
    assert_eq!(probe.world.counters().block_updates_received, 3);
    assert_eq!(probe.world.counters().block_updates_applied, 3);
    assert_eq!(probe.world.counters().block_updates_ignored, 0);
    assert_eq!(
        probe.world.counters().chunk_cache_center_updates_received,
        1
    );
    assert_eq!(
        probe.world.counters().chunk_cache_radius_updates_received,
        1
    );

    probe
        .handle_play_packet(PlayClientbound::ForgetLevelChunk(ForgetLevelChunk {
            pos: ProtocolChunkPos {
                x: chunk_pos.x,
                z: chunk_pos.z,
            },
        }))
        .await
        .unwrap();

    let report = probe.finish(2, chunk_pos, Vec::new(), None, None);

    assert_eq!(report.first_chunk_summary.unwrap().pos, chunk_pos);
    assert_eq!(report.world.chunk_count(), 0);
    assert_eq!(report.world.first_chunk(), Some(chunk_pos));
    assert_eq!(report.world_counters.chunk_forgets_received, 1);
    assert_eq!(report.world_counters.chunks_forgotten, 1);
    assert_eq!(report.world_counters.chunk_forgets_ignored, 0);
}

#[tokio::test]
async fn probe_applies_light_and_biome_updates_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    let chunk_pos = ChunkPos { x: 1, z: -2 };
    let mut sky = vec![0; LIGHT_ARRAY_BYTES];
    let mut block = vec![0; LIGHT_ARRAY_BYTES];
    set_light_nibble(&mut sky, 0, 4);
    set_light_nibble(&mut block, 0, 13);

    probe
        .handle_play_packet(PlayClientbound::LevelChunkWithLight(
            synthetic_probe_level_chunk_packet(),
        ))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::LightUpdate(LightUpdate {
            chunk_x: chunk_pos.x,
            chunk_z: chunk_pos.z,
            light_data: LightUpdateData {
                sky_y_mask: vec![0b10],
                block_y_mask: vec![0b10],
                empty_sky_y_mask: Vec::new(),
                empty_block_y_mask: Vec::new(),
                sky_updates: vec![sky],
                block_updates: vec![block],
            },
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::ChunksBiomes(ChunksBiomes {
            chunks: vec![ChunkBiomeData {
                pos: ProtocolChunkPos {
                    x: chunk_pos.x,
                    z: chunk_pos.z,
                },
                raw_biomes: single_biome_payload(7),
            }],
        }))
        .await
        .unwrap();

    let chunk = probe.world.probe_chunk(chunk_pos).unwrap();
    assert_eq!(probe.world.queued_light_update_count(), 0);
    assert_eq!(chunk.light.sky_array_count(), 1);
    assert_eq!(chunk.light.block_array_count(), 1);
    assert_eq!(chunk.light.sky_packed(1).unwrap()[0] & 0x0f, 4);
    assert_eq!(chunk.light.block_packed(1).unwrap()[0] & 0x0f, 13);
    assert_eq!(
        probe
            .world
            .probe_block(BlockPos {
                x: 16,
                y: -64,
                z: -32,
            })
            .unwrap()
            .biome_id,
        Some(7)
    );

    let report = probe.finish(2, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(report.world_counters.chunks_received, 1);
    assert_eq!(report.world_counters.light_updates_received, 1);
    assert_eq!(report.world_counters.light_updates_applied, 1);
    assert_eq!(report.world_counters.light_updates_ignored, 0);
    assert_eq!(report.world_counters.biome_updates_received, 1);
    assert_eq!(report.world_counters.biome_updates_applied, 1);
    assert_eq!(report.world_counters.biome_updates_ignored, 0);
}

#[tokio::test]
async fn probe_applies_block_destruction_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::BlockDestruction(
            bbb_protocol::packets::BlockDestruction {
                id: 4,
                pos: ProtocolBlockPos {
                    x: 12,
                    y: 64,
                    z: -5,
                },
                progress: 6,
            },
        ))
        .await
        .unwrap();
    assert_eq!(
        probe
            .world
            .block_destruction(4)
            .map(|progress| progress.progress),
        Some(6)
    );

    probe
        .handle_play_packet(PlayClientbound::BlockDestruction(
            bbb_protocol::packets::BlockDestruction {
                id: 4,
                pos: ProtocolBlockPos {
                    x: 12,
                    y: 64,
                    z: -5,
                },
                progress: 10,
            },
        ))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::BlockDestruction(
            bbb_protocol::packets::BlockDestruction {
                id: 99,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                progress: 255,
            },
        ))
        .await
        .unwrap();

    let report = probe.finish(3, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(report.world_counters.block_destructions_received, 3);
    assert_eq!(report.world_counters.block_destructions_tracked, 0);
    assert_eq!(report.world_counters.block_destructions_removed, 1);
    assert_eq!(report.world_counters.block_destructions_ignored, 1);
    assert!(report.world.block_destruction(4).is_none());
}

#[tokio::test]
async fn probe_play_cookie_events_update_world_and_respond() {
    let (client, mut server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::StoreCookie(StoreCookie {
            key: "bbb:session".to_string(),
            payload: vec![4, 5, 6],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::CookieRequest(CookieRequest {
            key: "bbb:session".to_string(),
        }))
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("cookie response should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_COOKIE_RESPONSE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32767).unwrap(), "bbb:session");
    assert!(decoder.read_bool().unwrap());
    let len = decoder.read_len().unwrap();
    assert_eq!(
        decoder.read_exact(len, "cookie response").unwrap(),
        &[4, 5, 6]
    );
    assert!(decoder.is_empty());

    let report = probe.finish(2, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    assert_eq!(report.world.last_cookie_key(), Some("bbb:session"));
    assert_eq!(report.world.stored_cookie_count(), 1);
    assert_eq!(report.world_counters.store_cookie_packets, 1);
    assert_eq!(report.world_counters.stored_cookie_bytes, 3);
    assert_eq!(report.world_counters.cookie_request_packets, 1);
    assert_eq!(report.world_counters.cookie_response_hits, 1);
    assert_eq!(report.world_counters.cookie_response_misses, 0);
}

#[tokio::test]
async fn probe_play_server_presentation_packets_update_world_and_decline_resource_pack() {
    let pack_id = Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678);
    let missing_pack_id = Uuid::from_u128(0xaaaaaaaa_bbbb_cccc_dddd_eeeeeeeeeeee);
    let (client, mut server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    let mut server_icon = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    server_icon.extend_from_slice(&13i32.to_be_bytes());
    server_icon.extend_from_slice(b"IHDR");
    server_icon.extend_from_slice(&64i32.to_be_bytes());
    server_icon.extend_from_slice(&64i32.to_be_bytes());
    probe
        .handle_play_packet(PlayClientbound::ServerData(ServerData {
            motd: bbb_protocol::RichTextComponent::plain("Native test server"),
            icon_bytes: Some(server_icon.clone()),
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::ResourcePackPush(ResourcePackPush {
            id: pack_id,
            url: "https://example.invalid/pack.zip".to_string(),
            hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
            required: true,
            prompt: Some(bbb_protocol::RichTextComponent::plain("Install pack?")),
        }))
        .await
        .unwrap();

    let pack = probe
        .world
        .resource_pack(pack_id)
        .expect("resource pack should be tracked after push");
    assert_eq!(pack.url, "https://example.invalid/pack.zip");
    assert_eq!(pack.hash, "0123456789abcdef0123456789abcdef01234567");
    assert!(pack.required);
    assert_eq!(pack.prompt, None);
    assert_eq!(
        pack.last_response.as_ref().map(|response| response.action),
        Some(ResourcePackResponseAction::Declined)
    );

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("resource pack response should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_RESOURCE_PACK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_uuid().unwrap(), pack_id);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        ResourcePackResponseAction::Declined.ordinal()
    );
    assert!(decoder.is_empty());

    probe
        .handle_play_packet(PlayClientbound::ResourcePackPop(ResourcePackPop {
            id: Some(pack_id),
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::ResourcePackPop(ResourcePackPop {
            id: Some(missing_pack_id),
        }))
        .await
        .unwrap();

    let report = probe.finish(4, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let server_data = report
        .world
        .server_data()
        .expect("server data should be tracked");
    assert_eq!(server_data.motd.text(), "Native test server");
    assert_eq!(server_data.icon_byte_len(), Some(server_icon.len()));
    assert!(report.world.resource_packs().is_empty());
    assert_eq!(report.world_counters.server_data_packets, 1);
    assert_eq!(report.world_counters.resource_pack_push_packets, 1);
    assert_eq!(report.world_counters.resource_pack_response_packets, 1);
    assert_eq!(
        report.world_counters.resource_pack_response_updates_applied,
        1
    );
    assert_eq!(
        report.world_counters.resource_pack_response_updates_ignored,
        0
    );
    assert_eq!(report.world_counters.resource_pack_required_declines, 1);
    assert_eq!(report.world_counters.resource_pack_pop_packets, 2);
    assert_eq!(report.world_counters.resource_pack_pop_updates_applied, 1);
    assert_eq!(report.world_counters.resource_pack_pop_updates_ignored, 1);
    assert_eq!(report.world_counters.resource_packs_tracked, 0);
}

#[tokio::test]
async fn probe_play_common_server_presentation_packets_update_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    let details = BTreeMap::from([
        ("Region".to_string(), "local".to_string()),
        ("Server".to_string(), "probe-play".to_string()),
    ]);

    probe
        .handle_play_packet(PlayClientbound::CustomReportDetails(CustomReportDetails {
            details: details.clone(),
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::ServerLinks(ServerLinks {
            links: vec![
                ServerLinkEntry {
                    link_type: ServerLinkType::Known(ServerLinkKnownType::Support),
                    url: "https://example.invalid/support".to_string(),
                },
                ServerLinkEntry {
                    link_type: ServerLinkType::Custom {
                        label: "Rules".to_string(),
                    },
                    url: "http://example.invalid/rules".to_string(),
                },
                ServerLinkEntry {
                    link_type: ServerLinkType::Known(ServerLinkKnownType::Website),
                    url: "ftp://example.invalid/file".to_string(),
                },
            ],
        }))
        .await
        .unwrap();

    let report = probe.finish(2, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    assert_eq!(report.world.custom_report_details(), &details);
    assert_eq!(report.world.server_links().len(), 2);
    assert_eq!(
        report.world.server_links()[0].label,
        "known_server_link.support"
    );
    assert_eq!(
        report.world.server_links()[0].known_type.as_deref(),
        Some("support")
    );
    assert_eq!(
        report.world.server_links()[0].url,
        "https://example.invalid/support"
    );
    assert_eq!(report.world.server_links()[1].label, "Rules");
    assert_eq!(report.world.server_links()[1].known_type, None);
    assert_eq!(
        report.world.server_links()[1].url,
        "http://example.invalid/rules"
    );
    assert_eq!(report.world_counters.custom_report_detail_packets, 1);
    assert_eq!(report.world_counters.custom_report_details_tracked, 2);
    assert_eq!(report.world_counters.server_link_packets, 1);
    assert_eq!(report.world_counters.server_links_tracked, 2);
    assert_eq!(report.world_counters.server_link_invalid_entries, 1);
}

#[tokio::test]
async fn probe_applies_inventory_container_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::SetPlayerInventory(SetPlayerInventory {
            slot: 36,
            item: item_stack(42, 1),
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::OpenScreen(OpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
            title: bbb_protocol::RichTextComponent::plain("Merchant"),
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::ContainerSetContent(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![ItemStackSummary::empty(), item_stack(43, 2)],
            carried_item: item_stack(98, 1),
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::ContainerSetSlot(ContainerSetSlot {
            container_id: 7,
            state_id: 13,
            slot: 1,
            item: item_stack(44, 3),
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::ContainerSetData(ContainerSetData {
            container_id: 7,
            id: 2,
            value: 10,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::MerchantOffers(merchant_offers(7, 2)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetCursorItem(SetCursorItem {
            item: item_stack(99, 1),
        }))
        .await
        .unwrap();

    let inventory = probe.world.inventory();
    assert_eq!(inventory.player_slots.len(), 1);
    assert_eq!(inventory.player_slots[0].slot, 36);
    assert_eq!(inventory.player_slots[0].item, item_stack(42, 1));
    assert_eq!(inventory.cursor_item, item_stack(99, 1));

    let container = inventory.open_container.as_ref().unwrap();
    assert_eq!(container.container_id, 7);
    assert_eq!(container.menu_type_id, Some(VANILLA_MENU_TYPE_MERCHANT_ID));
    assert_eq!(
        container
            .title
            .as_ref()
            .map(bbb_protocol::RichTextComponent::text),
        Some("Merchant")
    );
    assert_eq!(container.state_id, 13);
    assert_eq!(inventory.container_slots(container).count(), 2);
    assert_eq!(
        inventory.container_slot(container, 0).unwrap().item,
        &ItemStackSummary::empty()
    );
    assert_eq!(
        inventory.container_slot(container, 1).unwrap().item,
        &item_stack(44, 3)
    );
    assert_eq!(container.data_values.len(), 1);
    assert_eq!(container.data_values[0].id, 2);
    assert_eq!(container.data_values[0].value, 10);
    let offers = container.merchant_offers.as_ref().unwrap();
    assert_eq!(offers.container_id, 7);
    assert_eq!(offers.offers.len(), 2);
    assert_eq!(offers.villager_level, 3);
    assert_eq!(offers.villager_xp, 120);
    assert!(offers.show_progress);
    assert!(!offers.can_restock);
    assert_eq!(offers.offers[0].buy_a, item_cost(42, 3));
    assert_eq!(offers.offers[0].sell, item_stack(99, 1));

    probe
        .handle_play_packet(PlayClientbound::ContainerClose(ContainerClose {
            container_id: 7,
        }))
        .await
        .unwrap();

    let report = probe.finish(3, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert!(report.world.inventory().open_container.is_none());
    assert_eq!(
        report.world.inventory().player_slots[0].item,
        item_stack(42, 1)
    );
    assert_eq!(
        report.world.inventory().cursor_item,
        ItemStackSummary::empty()
    );
    assert_eq!(report.world_counters.inventory_slot_updates_received, 1);
    assert_eq!(report.world_counters.inventory_slots_tracked, 1);
    assert_eq!(report.world_counters.cursor_item_updates_received, 1);
    assert_eq!(report.world_counters.container_open_updates_received, 1);
    assert_eq!(report.world_counters.container_content_updates_received, 1);
    assert_eq!(report.world_counters.container_slot_updates_received, 1);
    assert_eq!(report.world_counters.container_data_updates_received, 1);
    assert_eq!(report.world_counters.merchant_offer_packets_received, 1);
    assert_eq!(report.world_counters.merchant_offer_packets_applied, 1);
    assert_eq!(report.world_counters.merchant_offer_packets_ignored, 0);
    assert_eq!(report.world_counters.merchant_offers_tracked, 0);
    assert_eq!(report.world_counters.container_close_updates_received, 1);
    assert_eq!(report.world_counters.container_close_updates_applied, 1);
    assert_eq!(report.world_counters.container_close_updates_ignored, 0);
}

#[tokio::test]
async fn probe_play_custom_payload_updates_world_brand_and_unknown_payload() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::CustomPayload(CustomPayload {
            id: "minecraft:brand".to_string(),
            payload: CustomPayloadBody::Brand {
                brand: "vanilla".to_string(),
            },
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::CustomPayload(CustomPayload {
            id: "example:diagnostic".to_string(),
            payload: CustomPayloadBody::Unknown {
                raw_payload: vec![1, 2, 3, 4],
            },
        }))
        .await
        .unwrap();

    let report = probe.finish(2, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(report.world.server_brand(), Some("vanilla"));
    assert_eq!(
        report.world.last_custom_payload(),
        Some(&bbb_client_world::CustomPayloadState {
            id: "example:diagnostic".to_string(),
            kind: "unknown".to_string(),
            brand: None,
            raw_payload_len: 4,
        })
    );
    assert_eq!(report.world_counters.custom_payload_packets, 2);
    assert_eq!(report.world_counters.custom_payload_brand_packets, 1);
    assert_eq!(report.world_counters.custom_payload_unknown_packets, 1);
}

#[tokio::test]
async fn probe_play_resource_pack_invalid_url_updates_world_and_response() {
    let pack_id = Uuid::from_u128(0x77777777_3333_4444_5555_666666666666);
    let (client, mut server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::ResourcePackPush(ResourcePackPush {
            id: pack_id,
            url: "not a valid resource pack url".to_string(),
            hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
            required: true,
            prompt: Some(bbb_protocol::RichTextComponent::plain("Install pack?")),
        }))
        .await
        .unwrap();

    let pack = probe
        .world
        .resource_pack(pack_id)
        .expect("resource pack should be tracked after push");
    assert_eq!(
        pack.last_response.as_ref().map(|response| response.action),
        Some(ResourcePackResponseAction::InvalidUrl)
    );

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("resource pack response should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_RESOURCE_PACK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_uuid().unwrap(), pack_id);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        ResourcePackResponseAction::InvalidUrl.ordinal()
    );
    assert!(decoder.is_empty());
}

#[tokio::test]
async fn probe_play_transfer_updates_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::Transfer(Transfer {
            host: "next.example.invalid".into(),
            port: 25566,
        }))
        .await
        .unwrap();

    let report = probe.finish(1, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(report.world_counters.transfer_packets, 1);
    assert_eq!(
        report.world.last_transfer(),
        Some(&bbb_client_world::TransferTargetState {
            host: "next.example.invalid".to_string(),
            port: 25566,
        })
    );
}

#[tokio::test]
async fn probe_play_transfer_reconnects_with_transfer_intent_when_options_configured() {
    use bbb_client_net::{ConnectionOptions, RawConnection};
    use bbb_protocol::packets::{decode_client_intention, ClientIntent};
    use tokio::net::TcpListener;

    let (client, _server) = raw_connection_pair().await;
    let target_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target_port = target_listener.local_addr().unwrap().port();
    let options =
        ConnectionOptions::offline(format!("127.0.0.1:{target_port}"), "transfer-probe").unwrap();
    let mut probe = ProbeContext::with_connection_options(client, options);
    probe
        .session
        .store_cookie("bbb:session".to_string(), b"keep".to_vec());

    let accept_task = tokio::spawn(async move {
        let (stream, _) = target_listener.accept().await.unwrap();
        let mut peer = RawConnection::from_stream(stream, None);
        let (id, payload) = peer.read_packet().await.unwrap();
        decode_client_intention(id, &payload).unwrap()
    });

    probe
        .handle_play_packet(PlayClientbound::Transfer(Transfer {
            host: "127.0.0.1".into(),
            port: i32::from(target_port),
        }))
        .await
        .unwrap();
    probe.conn.flush().await.unwrap();

    let handshake = accept_task.await.unwrap();
    assert_eq!(handshake.intent, ClientIntent::Transfer);
    assert_eq!(handshake.port, target_port);
    assert_eq!(probe.state, ConnectionState::Login);
    assert_eq!(
        probe
            .session
            .server_cookies
            .get("bbb:session")
            .map(Vec::as_slice),
        Some(b"keep".as_slice())
    );
}

#[tokio::test]
async fn probe_move_vehicle_sends_vehicle_ack() {
    let (client, mut server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::Login(protocol_play_login(99)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity_with_type(
            10,
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        )))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetPassengers(SetPassengers {
            vehicle_id: 10,
            passenger_ids: vec![99],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::MoveVehicle(MoveVehicle {
            position: ProtocolVec3d {
                x: 5.0,
                y: 66.0,
                z: -7.0,
            },
            y_rot: 45.0,
            x_rot: -5.0,
        }))
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("move vehicle ack should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_VEHICLE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_f64().unwrap(), 5.0);
    assert_eq!(decoder.read_f64().unwrap(), 66.0);
    assert_eq!(decoder.read_f64().unwrap(), -7.0);
    assert_eq!(decoder.read_f32().unwrap(), 45.0);
    assert_eq!(decoder.read_f32().unwrap(), -5.0);
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());

    let report = probe.finish(4, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let vehicle = report.world.probe_entity(10).unwrap();
    assert_eq!(vehicle.position.x, 5.0);
    assert_eq!(vehicle.position.y, 66.0);
    assert_eq!(vehicle.position.z, -7.0);
    assert_eq!(report.world_counters.vehicle_moves_received, 1);
    assert_eq!(report.world_counters.vehicle_moves_applied, 1);
    assert_eq!(report.world_counters.vehicle_moves_acked, 1);
    assert_eq!(report.world_counters.vehicle_moves_snapped, 1);
}

#[tokio::test]
async fn probe_move_vehicle_without_mount_does_not_ack() {
    let (client, mut server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::Login(protocol_play_login(99)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(99)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::MoveVehicle(MoveVehicle {
            position: ProtocolVec3d {
                x: 5.0,
                y: 66.0,
                z: -7.0,
            },
            y_rot: 45.0,
            x_rot: -5.0,
        }))
        .await
        .unwrap();

    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "ignored vehicle move must not send a vehicle ack"
    );

    let report = probe.finish(4, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    assert_eq!(report.world_counters.vehicle_moves_received, 1);
    assert_eq!(report.world_counters.vehicle_moves_applied, 0);
    assert_eq!(report.world_counters.vehicle_moves_acked, 0);
    assert_eq!(report.world_counters.vehicle_moves_snapped, 0);
    assert_eq!(report.world_counters.vehicle_moves_ignored, 1);
}

#[tokio::test]
async fn probe_controlled_vehicle_teleport_sends_post_snap_ack() {
    let (client, mut server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::Login(protocol_play_login(99)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity_with_type(
            10,
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        )))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetPassengers(SetPassengers {
            vehicle_id: 10,
            passenger_ids: vec![99],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::TeleportEntity(TeleportEntity {
            id: 10,
            position: ProtocolVec3d {
                x: 5.0,
                y: 66.0,
                z: -7.0,
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot: 45.0,
            x_rot: -5.0,
            relatives_mask: 0,
            on_ground: true,
        }))
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("teleport vehicle ack should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_VEHICLE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_f64().unwrap(), 5.0);
    assert_eq!(decoder.read_f64().unwrap(), 66.0);
    assert_eq!(decoder.read_f64().unwrap(), -7.0);
    assert_eq!(decoder.read_f32().unwrap(), 45.0);
    assert_eq!(decoder.read_f32().unwrap(), -5.0);
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[tokio::test]
async fn probe_applies_minecart_along_track_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    probe
        .world
        .apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
            features: vec![
                "minecraft:vanilla".to_string(),
                "minecraft:minecart_improvements".to_string(),
            ],
        });

    probe
        .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity_with_type(
            10,
            VANILLA_ENTITY_TYPE_MINECART_ID,
        )))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(20)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::MoveMinecartAlongTrack(
            MoveMinecartAlongTrack {
                entity_id: 10,
                lerp_steps: vec![
                    minecart_step([1.25, 64.1, -2.25], [0.2, 0.0, -0.2], 45.0, -10.0, 0.5),
                    minecart_step([1.75, 64.2, -2.75], [0.4, 0.0, -0.4], 90.0, 5.0, 1.25),
                ],
            },
        ))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::MoveMinecartAlongTrack(
            MoveMinecartAlongTrack {
                entity_id: 999,
                lerp_steps: vec![minecart_step(
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                    0.0,
                    0.0,
                    1.0,
                )],
            },
        ))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::MoveMinecartAlongTrack(
            MoveMinecartAlongTrack {
                entity_id: 20,
                lerp_steps: vec![minecart_step(
                    [3.0, 64.0, -4.0],
                    [0.0, 0.0, 0.0],
                    0.0,
                    0.0,
                    1.0,
                )],
            },
        ))
        .await
        .unwrap();

    let mut report = probe.finish(5, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let minecart = report.world.probe_entity(10).unwrap();
    assert_eq!(minecart.position.x, 1.0);
    assert_eq!(minecart.position.y, 64.0);
    assert_eq!(minecart.position.z, -2.0);
    assert_eq!(minecart.minecart_lerp_pending_steps.len(), 2);
    assert!(minecart.minecart_lerp_steps.is_empty());

    report.world.advance_entity_client_animations(3);
    let minecart = report.world.probe_entity(10).unwrap();
    assert_eq!(minecart.position.x, 1.75);
    assert_eq!(minecart.position.y, 64.2);
    assert_eq!(minecart.position.z, -2.75);
    assert_eq!(minecart.delta_movement.x, 0.4);
    assert_eq!(minecart.delta_movement.y, 0.0);
    assert_eq!(minecart.delta_movement.z, -0.4);
    assert_eq!(minecart.y_rot, 90.0);
    assert_eq!(minecart.x_rot, 5.0);
    assert_eq!(minecart.minecart_lerp_steps.len(), 2);

    let non_minecart = report.world.probe_entity(20).unwrap();
    assert_eq!(non_minecart.position.x, 1.0);
    assert_eq!(non_minecart.position.y, 64.0);
    assert_eq!(non_minecart.position.z, -2.0);
    assert!(report.world.probe_entity(999).is_none());

    assert_eq!(report.world_counters.entities_received, 2);
    assert_eq!(report.world_counters.entities_tracked, 2);
    assert_eq!(report.world_counters.minecart_moves_received, 3);
    assert_eq!(report.world_counters.minecart_moves_applied, 1);
    assert_eq!(report.world_counters.minecart_moves_ignored, 2);
    assert_eq!(report.world_counters.minecart_lerp_steps_received, 4);
    assert_eq!(report.world_counters.minecart_lerp_steps_tracked, 2);
}

#[tokio::test]
async fn probe_applies_entity_motion_and_transient_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(123)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetEntityMotion(SetEntityMotion {
            id: 123,
            delta_movement: ProtocolVec3d {
                x: 0.1,
                y: 0.0,
                z: -0.1,
            },
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::RotateHead(RotateHead {
            id: 123,
            y_head_rot: 90.0,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::EntityAnimation(EntityAnimation {
            id: 123,
            action: 3,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 123,
            event_id: 35,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::HurtAnimation(HurtAnimation {
            id: 123,
            yaw: 45.5,
        }))
        .await
        .unwrap();

    let mut report = probe.finish(6, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let entity = report.world.probe_entity(123).unwrap();

    assert_eq!(entity.delta_movement.x, 0.1);
    assert_eq!(entity.delta_movement.y, 0.0);
    assert_eq!(entity.delta_movement.z, -0.1);
    assert_eq!(entity.y_head_rot, 30.0);
    assert_eq!(entity.last_animation_action, Some(3));
    assert_eq!(entity.last_event_id, Some(35));
    assert_eq!(entity.last_hurt_yaw, Some(45.5));
    report.world.advance_entity_client_animations(3);
    assert_eq!(report.world.probe_entity(123).unwrap().y_head_rot, 90.0);

    assert_eq!(report.world_counters.entities_received, 1);
    assert_eq!(report.world_counters.entities_tracked, 1);
    assert_eq!(report.world_counters.entity_motion_updates_received, 1);
    assert_eq!(report.world_counters.entity_motion_updates_applied, 1);
    assert_eq!(report.world_counters.entity_motion_updates_ignored, 0);
    assert_eq!(report.world_counters.entity_head_rotations_received, 1);
    assert_eq!(report.world_counters.entity_head_rotations_applied, 1);
    assert_eq!(report.world_counters.entity_head_rotations_ignored, 0);
    assert_eq!(report.world_counters.entity_animation_updates_received, 1);
    assert_eq!(report.world_counters.entity_animation_updates_applied, 1);
    assert_eq!(report.world_counters.entity_animation_updates_ignored, 0);
    assert_eq!(report.world_counters.entity_events_received, 1);
    assert_eq!(report.world_counters.entity_events_applied, 1);
    assert_eq!(report.world_counters.entity_events_ignored, 0);
    assert_eq!(report.world_counters.entity_hurt_animations_received, 1);
    assert_eq!(report.world_counters.entity_hurt_animations_applied, 1);
    assert_eq!(report.world_counters.entity_hurt_animations_ignored, 0);
}

#[tokio::test]
async fn probe_counts_missing_entity_updates_as_ignored() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::EntityPositionSync(EntityPositionSync {
            id: 999,
            position: ProtocolVec3d {
                x: 1.0,
                y: 65.0,
                z: -1.0,
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::MoveEntity(EntityMove {
            id: 999,
            has_position: true,
            delta_x: 4096,
            delta_y: 0,
            delta_z: -2048,
            y_rot: Some(90.0),
            x_rot: Some(10.0),
            on_ground: true,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::TeleportEntity(TeleportEntity {
            id: 999,
            position: ProtocolVec3d {
                x: 2.0,
                y: 70.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot: 180.0,
            x_rot: 15.0,
            relatives_mask: 0,
            on_ground: true,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::EntityAnimation(EntityAnimation {
            id: 999,
            action: 4,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 999,
            event_id: 21,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::HurtAnimation(HurtAnimation {
            id: 999,
            yaw: 90.0,
        }))
        .await
        .unwrap();

    let report = probe.finish(6, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert!(report.world.probe_entity(999).is_none());
    assert_eq!(report.world_counters.entities_received, 0);
    assert_eq!(report.world_counters.entities_tracked, 0);
    assert_eq!(report.world_counters.entity_position_syncs_received, 1);
    assert_eq!(report.world_counters.entity_position_syncs_applied, 0);
    assert_eq!(report.world_counters.entity_position_syncs_ignored, 1);
    assert_eq!(report.world_counters.entity_moves_received, 1);
    assert_eq!(report.world_counters.entity_moves_applied, 0);
    assert_eq!(report.world_counters.entity_moves_ignored, 1);
    assert_eq!(report.world_counters.entity_teleports_received, 1);
    assert_eq!(report.world_counters.entity_teleports_applied, 0);
    assert_eq!(report.world_counters.entity_teleports_ignored, 1);
    assert_eq!(report.world_counters.entity_animation_updates_received, 1);
    assert_eq!(report.world_counters.entity_animation_updates_applied, 0);
    assert_eq!(report.world_counters.entity_animation_updates_ignored, 1);
    assert_eq!(report.world_counters.entity_events_received, 1);
    assert_eq!(report.world_counters.entity_events_applied, 0);
    assert_eq!(report.world_counters.entity_events_ignored, 1);
    assert_eq!(report.world_counters.entity_hurt_animations_received, 1);
    assert_eq!(report.world_counters.entity_hurt_animations_applied, 0);
    assert_eq!(report.world_counters.entity_hurt_animations_ignored, 1);
}

#[tokio::test]
async fn probe_applies_entity_state_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(123)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(456)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity_with_type(
            300,
            VANILLA_ENTITY_TYPE_ITEM_ID,
        )))
        .await
        .unwrap();

    probe
        .handle_play_packet(PlayClientbound::EntityPositionSync(EntityPositionSync {
            id: 123,
            position: ProtocolVec3d {
                x: 2.0,
                y: 65.0,
                z: -3.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.25,
                z: 0.0,
            },
            y_rot: 180.0,
            x_rot: 30.0,
            on_ground: true,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::MoveEntity(EntityMove {
            id: 123,
            has_position: true,
            delta_x: 4096,
            delta_y: 0,
            delta_z: -2048,
            y_rot: Some(-90.0),
            x_rot: Some(45.0),
            on_ground: false,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::TeleportEntity(TeleportEntity {
            id: 123,
            position: ProtocolVec3d {
                x: 0.5,
                y: 70.0,
                z: -4.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.2,
                z: 0.0,
            },
            y_rot: 10.0,
            x_rot: -120.0,
            relatives_mask: 0,
            on_ground: true,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetEntityData(SetEntityData {
            id: 123,
            values: vec![entity_invisible_data(), entity_air_supply_data(301)],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetEquipment(SetEquipment {
            entity_id: 123,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: item_stack(42, 1),
            }],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::UpdateAttributes(UpdateAttributes {
            entity_id: 123,
            attributes: vec![AttributeSnapshot {
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
                base: 20.0,
                modifiers: vec![AttributeModifier {
                    id: "minecraft:health_bonus".to_string(),
                    amount: 4.0,
                    operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                }],
            }],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetEntityLink(SetEntityLink {
            source_id: 123,
            dest_id: 456,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetPassengers(SetPassengers {
            vehicle_id: 123,
            passenger_ids: vec![456],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetEntityData(SetEntityData {
            id: 300,
            values: vec![item_stack_entity_data(item_stack(99, 2))],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::TakeItemEntity(TakeItemEntity {
            item_id: 300,
            player_id: 123,
            amount: 1,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::UpdateMobEffect(UpdateMobEffect {
            entity_id: 123,
            effect_id: VANILLA_MOB_EFFECT_MINING_FATIGUE_ID,
            amplifier: 1,
            duration_ticks: 200,
            flags: MobEffectFlags {
                raw: 0b0110,
                ambient: false,
                visible: true,
                show_icon: true,
                blend: false,
            },
        }))
        .await
        .unwrap();
    assert_eq!(
        probe
            .world
            .entity_effect(123, VANILLA_MOB_EFFECT_MINING_FATIGUE_ID)
            .unwrap()
            .duration_ticks,
        200
    );
    probe
        .handle_play_packet(PlayClientbound::DamageEvent(DamageEvent {
            entity_id: 123,
            source_type_id: 5,
            source_cause_id: 456,
            source_direct_id: 300,
            source_position: Some(ProtocolVec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            }),
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::RemoveMobEffect(RemoveMobEffect {
            entity_id: 123,
            effect_id: VANILLA_MOB_EFFECT_MINING_FATIGUE_ID,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::RemoveEntities(RemoveEntities {
            entity_ids: vec![456, 404],
        }))
        .await
        .unwrap();

    let mut report = probe.finish(5, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let entity = report.world.probe_entity(123).unwrap();
    assert_eq!(entity.position.x, 2.0);
    assert_eq!(entity.position.y, 65.0);
    assert_eq!(entity.position.z, -3.0);
    assert_eq!(entity.delta_movement.y, 0.2);
    assert_eq!(entity.y_rot, 180.0);
    assert_eq!(entity.x_rot, 30.0);
    assert_eq!(
        entity.interpolation.as_ref().map(|state| (
            state.target_position,
            state.target_y_rot,
            state.target_x_rot,
        )),
        Some((
            bbb_client_world::EntityVec3 {
                x: 0.5,
                y: 70.0,
                z: -4.0,
            },
            10.0,
            -90.0,
        ))
    );
    assert_eq!(entity.on_ground, Some(true));
    assert_eq!(
        entity.data_values,
        vec![entity_invisible_data(), entity_air_supply_data(301)]
    );
    assert_eq!(
        entity.equipment,
        vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Head,
            item: item_stack(42, 1),
        }]
    );
    assert_eq!(
        entity.attributes,
        vec![AttributeSnapshot {
            attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
            base: 20.0,
            modifiers: vec![AttributeModifier {
                id: "minecraft:health_bonus".to_string(),
                amount: 4.0,
                operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            }],
        }]
    );
    assert_eq!(entity.leash_holder_id, None);
    assert!(entity.passengers.is_empty());
    assert!(report.world.probe_entity(456).is_none());
    assert_eq!(
        report.world.probe_entity(300).unwrap().data_values,
        vec![item_stack_entity_data(item_stack(99, 1))]
    );
    assert!(report
        .world
        .entity_effect(123, VANILLA_MOB_EFFECT_MINING_FATIGUE_ID)
        .is_none());
    let damage = report.world.entity_last_damage(123).unwrap();
    assert_eq!(damage.source_type_id, 5);
    assert_eq!(damage.source_cause_id, 456);
    assert_eq!(damage.source_direct_id, 300);
    assert_eq!(
        damage.source_position,
        Some(ProtocolVec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        })
    );

    assert_eq!(report.world_counters.entities_received, 3);
    assert_eq!(report.world_counters.entities_tracked, 2);
    assert_eq!(report.world_counters.entity_position_syncs_received, 1);
    assert_eq!(report.world_counters.entity_position_syncs_applied, 1);
    assert_eq!(report.world_counters.entity_position_syncs_ignored, 0);
    assert_eq!(report.world_counters.entity_moves_received, 1);
    assert_eq!(report.world_counters.entity_moves_applied, 1);
    assert_eq!(report.world_counters.entity_moves_ignored, 0);
    assert_eq!(report.world_counters.entity_teleports_received, 1);
    assert_eq!(report.world_counters.entity_teleports_applied, 1);
    assert_eq!(report.world_counters.entity_teleports_ignored, 0);
    assert_eq!(report.world_counters.entity_data_updates_received, 2);
    assert_eq!(report.world_counters.entity_data_values_received, 3);
    assert_eq!(report.world_counters.entity_data_updates_applied, 2);
    assert_eq!(report.world_counters.entity_data_updates_ignored, 0);
    assert_eq!(report.world_counters.entity_equipment_updates_received, 1);
    assert_eq!(report.world_counters.entity_equipment_slots_received, 1);
    assert_eq!(report.world_counters.entity_equipment_updates_applied, 1);
    assert_eq!(report.world_counters.entity_equipment_updates_ignored, 0);
    assert_eq!(report.world_counters.entity_attribute_updates_received, 1);
    assert_eq!(report.world_counters.entity_attributes_received, 1);
    assert_eq!(report.world_counters.entity_attribute_updates_applied, 1);
    assert_eq!(report.world_counters.entity_attribute_updates_ignored, 0);
    assert_eq!(report.world_counters.entity_link_updates_received, 1);
    assert_eq!(report.world_counters.entity_link_updates_applied, 1);
    assert_eq!(report.world_counters.entity_link_updates_ignored, 0);
    assert_eq!(report.world_counters.entity_passenger_updates_received, 1);
    assert_eq!(report.world_counters.entity_passenger_ids_received, 1);
    assert_eq!(report.world_counters.entity_passenger_updates_applied, 1);
    assert_eq!(report.world_counters.entity_passenger_updates_ignored, 0);
    assert_eq!(report.world_counters.take_item_entities_received, 1);
    assert_eq!(report.world_counters.take_item_entities_applied, 1);
    assert_eq!(report.world_counters.take_item_entities_ignored, 0);
    assert_eq!(report.world_counters.item_entity_stack_shrinks, 1);
    assert_eq!(report.world_counters.take_item_entities_removed, 0);
    assert_eq!(report.world_counters.update_mob_effect_packets, 1);
    assert_eq!(report.world_counters.update_mob_effects_ignored, 0);
    assert_eq!(report.world_counters.remove_mob_effect_packets, 1);
    assert_eq!(report.world_counters.remove_mob_effects_ignored, 0);
    assert_eq!(report.world_counters.active_mob_effects_tracked, 0);
    assert_eq!(report.world_counters.damage_event_packets, 1);
    assert_eq!(report.world_counters.damage_events_applied, 1);
    assert_eq!(report.world_counters.damage_events_ignored, 0);
    assert_eq!(report.world_counters.entity_removes_received, 2);
    assert_eq!(report.world_counters.entities_removed, 1);
    assert_eq!(report.world_counters.entity_removes_ignored, 1);

    report.world.advance_entity_client_animations(3);
    let entity = report.world.probe_entity(123).unwrap();
    assert_eq!(entity.position.x, 0.5);
    assert_eq!(entity.position.y, 70.0);
    assert_eq!(entity.position.z, -4.0);
    assert_eq!(entity.y_rot, 10.0);
    assert_eq!(entity.x_rot, -90.0);
    assert_eq!(entity.interpolation, None);
}

#[tokio::test]
async fn probe_projectile_power_updates_world_entity_state_and_counters() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity_with_type(
            123,
            VANILLA_ENTITY_TYPE_FIREBALL_ID,
        )))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity_with_type(
            456, 7,
        )))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::ProjectilePower(ProjectilePower {
            entity_id: 123,
            acceleration_power: 0.75,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::ProjectilePower(ProjectilePower {
            entity_id: 456,
            acceleration_power: 0.25,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::ProjectilePower(ProjectilePower {
            entity_id: 404,
            acceleration_power: 0.5,
        }))
        .await
        .unwrap();

    let report = probe.finish(5, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(
        report.world.hurting_projectile(123),
        Some(bbb_client_world::HurtingProjectileState {
            acceleration_power: 0.75,
        })
    );
    assert_eq!(report.world.hurting_projectile(456), None);
    assert_eq!(report.world_counters.projectile_power_packets, 3);
    assert_eq!(report.world_counters.projectile_power_updates_applied, 1);
    assert_eq!(report.world_counters.projectile_power_updates_ignored, 2);
    assert_eq!(
        report.world.last_projectile_power_update(),
        Some(&bbb_client_world::ProjectilePowerUpdateState {
            entity_id: 404,
            acceleration_power: 0.5,
            applied: false,
        })
    );
}

#[tokio::test]
async fn probe_applies_waypoint_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    let waypoint_id = Uuid::from_u128(0x00112233445566778899aabbccddeeff);

    probe
        .handle_play_packet(PlayClientbound::Waypoint(TrackedWaypointPacket {
            operation: WaypointOperation::Track,
            waypoint: TrackedWaypoint {
                identifier: WaypointIdentifier::Uuid(waypoint_id),
                icon: WaypointIcon {
                    style: "minecraft:default".to_string(),
                    color_rgb: Some(0x112233),
                },
                data: WaypointData::Position(WaypointVec3i {
                    x: 12,
                    y: 70,
                    z: -8,
                }),
            },
        }))
        .await
        .unwrap();

    let report = probe.finish(1, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let key = format!("uuid:{waypoint_id}");
    let waypoint = report
        .world
        .tracked_waypoints()
        .get(&key)
        .expect("waypoint should be tracked");

    assert_eq!(report.world_counters.waypoint_packets, 1);
    assert_eq!(report.world_counters.waypoints_tracked, 1);
    assert_eq!(waypoint.identifier_kind, "uuid");
    assert_eq!(waypoint.identifier, waypoint_id.to_string());
    assert_eq!(waypoint.icon_style, "minecraft:default");
    assert_eq!(waypoint.icon_color_rgb, Some(0x112233));
    assert_eq!(waypoint.data.kind, "position");
    assert_eq!(
        waypoint.data.position,
        Some(bbb_client_world::WaypointVec3iState {
            x: 12,
            y: 70,
            z: -8,
        })
    );
    assert_eq!(
        report
            .world
            .last_waypoint_event()
            .map(|event| event.applied),
        Some(true)
    );
}
