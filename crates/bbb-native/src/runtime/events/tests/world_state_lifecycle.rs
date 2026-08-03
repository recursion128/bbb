use super::*;
use crate::sign_scene::sign_scene_from_world;
use bbb_item_model::NativeItemRuntime;

#[test]
fn block_changed_ack_updates_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::BlockChangedAck(
        bbb_protocol::packets::BlockChangedAck { sequence: 17 },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(
        world.last_block_changed_ack(),
        Some(&bbb_world::BlockChangedAckState { sequence: 17 })
    );
    assert_eq!(world.counters().block_changed_ack_packets, 1);
}

#[test]
fn configuration_reentry_clears_online_client_level_state() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::StateChanged {
        state: bbb_net::ConnectionState::Configuration,
    })
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_login(&protocol_play_login(9));
    world.apply_add_entity(protocol_add_entity_with_type(55, 1));
    world.set_local_using_item(true);
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(counters.state.as_deref(), Some("Configuration"));
    assert_eq!(world.entity_count(), 0);
    assert_eq!(
        world.local_player(),
        &bbb_world::LocalPlayerState::default()
    );
}

#[test]
fn login_starts_fresh_registry_session_but_play_reconfiguration_keeps_access() {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:item",
        0,
        vec![RegistryPacketEntry::stub("example:first_server_item")],
    );
    world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec!["minecraft:trade_rebalance".to_string()],
    });
    world.apply_custom_payload(bbb_protocol::packets::CustomPayload {
        id: "minecraft:brand".to_string(),
        payload: bbb_protocol::packets::CustomPayloadBody::Brand {
            brand: "first-server".to_string(),
        },
    });
    world.apply_code_of_conduct("First server rules".to_string());

    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::StateChanged {
        state: bbb_net::ConnectionState::Configuration,
    })
    .unwrap();
    tx.try_send(NetEvent::StateChanged {
        state: bbb_net::ConnectionState::Play,
    })
    .unwrap();
    let mut counters = NetCounters::default();
    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert_eq!(
        world.registry_resource_id_by_holder_id("minecraft:item", 0),
        Some("example:first_server_item")
    );
    assert!(world.is_feature_enabled("minecraft:trade_rebalance"));
    assert_eq!(world.server_brand(), Some("first-server"));
    assert!(world.last_code_of_conduct().is_some());

    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Connected).unwrap();
    tx.try_send(NetEvent::StateChanged {
        state: bbb_net::ConnectionState::Login,
    })
    .unwrap();
    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert!(world
        .resolved_registry_entry_by_resource_id("minecraft:item", "example:first_server_item")
        .is_none());
    assert_eq!(
        world.registry_resource_id_by_holder_id(
            "minecraft:item",
            bbb_protocol::item_types::VANILLA_ITEM_SNOWBALL_ID,
        ),
        Some("minecraft:snowball")
    );
    assert_eq!(
        world.enabled_feature_list(),
        vec!["minecraft:vanilla".to_string()]
    );
    assert!(world.server_brand().is_none());
    assert!(world.last_code_of_conduct().is_none());
}

#[test]
fn invalid_configuration_registry_commit_requests_disconnect() {
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let (command_tx, mut command_rx) = mpsc::channel(1);

    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::StateChanged {
        state: bbb_net::ConnectionState::Configuration,
    })
    .unwrap();
    assert_eq!(
        drain_net_events(
            &mut rx,
            &mut world,
            &mut counters,
            &Some(command_tx.clone())
        ),
        1
    );
    world.record_registry_data(bbb_protocol::packets::RegistryData {
        registry: "minecraft:wolf_variant".to_string(),
        entries: Vec::new(),
        raw_payload_len: 0,
    });

    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::StateChanged {
        state: bbb_net::ConnectionState::Play,
    })
    .unwrap();
    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &Some(command_tx)),
        1
    );

    assert_eq!(command_rx.try_recv().unwrap(), NetCommand::Disconnect);
    assert_eq!(
        world
            .apply_diagnostics()
            .last_apply_error
            .as_ref()
            .map(|error| error.source.as_str()),
        Some("registry_configuration_validation")
    );
}

#[test]
fn chunk_cache_events_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::SetChunkCacheCenter(
        SetChunkCacheCenter {
            chunk_x: -4,
            chunk_z: 7,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetChunkCacheRadius(
        SetChunkCacheRadius { radius: 10 },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert_eq!(world.chunk_cache_center(), Some(ChunkPos { x: -4, z: 7 }));
    assert_eq!(world.chunk_cache_radius(), Some(10));
    let world_counters = world.counters();
    assert_eq!(world_counters.chunk_cache_center_updates_received, 1);
    assert_eq!(world_counters.chunk_cache_radius_updates_received, 1);
}

#[test]
fn terrain_chunk_events_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(7);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockUpdate(BlockUpdate {
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        block_state_id: vanilla_block_state_id(
            "minecraft:oak_sign",
            [("rotation", "0"), ("waterlogged", "false")],
        ),
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SectionBlocksUpdate(
        SectionBlocksUpdate {
            section_x: 1,
            section_y: -4,
            section_z: -2,
            updates: vec![BlockUpdate {
                pos: ProtocolBlockPos {
                    x: 17,
                    y: -64,
                    z: -31,
                },
                block_state_id: 6,
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockEntityData(
        BlockEntityData {
            pos: ProtocolBlockPos {
                x: 16,
                y: -64,
                z: -32,
            },
            block_entity_type_id:
                bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
            raw_nbt: vec![10, 0],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LightUpdate(LightUpdate {
        chunk_x: 1,
        chunk_z: -2,
        light_data: empty_light_update_data(),
    })))
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
    tx.try_send(NetEvent::Play(PlayClientbound::ForgetLevelChunk(
        ForgetLevelChunk {
            pos: ProtocolChunkPos { x: 1, z: -2 },
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        7
    );
    assert_eq!(world.first_chunk(), Some(ChunkPos { x: 1, z: -2 }));
    assert!(world.probe_chunk(ChunkPos { x: 1, z: -2 }).is_none());

    let world_counters = world.counters();
    macro_rules! assert_chunk_counter {
        ($field:ident, $value:expr) => {
            assert_eq!(world_counters.$field, $value);
        };
    }

    assert_chunk_counter!(chunks_received, 1);
    assert_chunk_counter!(chunks_decoded, 1);
    assert_chunk_counter!(sections_decoded, 24);
    assert_chunk_counter!(block_entities_seen, 1);
    assert_chunk_counter!(light_arrays_seen, 0);
    assert_chunk_counter!(block_updates_received, 2);
    assert_chunk_counter!(block_updates_applied, 2);
    assert_chunk_counter!(block_updates_ignored, 0);
    assert_chunk_counter!(block_entity_updates_received, 1);
    assert_chunk_counter!(block_entity_updates_applied, 1);
    assert_chunk_counter!(block_entity_updates_ignored, 0);
    assert_chunk_counter!(light_updates_received, 1);
    assert_chunk_counter!(light_updates_applied, 0);
    assert_chunk_counter!(light_updates_ignored, 1);
    assert_chunk_counter!(biome_updates_received, 1);
    assert_chunk_counter!(biome_updates_applied, 1);
    assert_chunk_counter!(biome_updates_ignored, 0);
    assert_chunk_counter!(chunk_forgets_received, 1);
    assert_chunk_counter!(chunks_forgotten, 1);
    assert_chunk_counter!(chunk_forgets_ignored, 0);
}

#[test]
fn network_drain_queues_light_until_the_runtime_poll_boundary() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LightUpdate(LightUpdate {
        chunk_x: 1,
        chunk_z: -2,
        light_data: LightUpdateData {
            sky_y_mask: vec![0b10],
            block_y_mask: Vec::new(),
            empty_sky_y_mask: Vec::new(),
            empty_block_y_mask: Vec::new(),
            sky_updates: vec![vec![7; 2_048]],
            block_updates: Vec::new(),
        },
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    let pos = ChunkPos { x: 1, z: -2 };
    assert_eq!(world.queued_light_update_count(), 2);
    assert!(world.probe_chunk(pos).unwrap().light.sky_updates.is_empty());
    assert_eq!(world.counters().light_updates_received, 1);
    assert_eq!(world.counters().light_updates_applied, 0);

    assert_eq!(world.poll_queued_light_updates(), 2);
    assert_eq!(world.queued_light_update_count(), 0);
    assert_eq!(world.probe_chunk(pos).unwrap().light.sky_updates[0][0], 7);
    assert_eq!(world.counters().light_updates_applied, 1);
}

#[test]
fn block_entity_data_sign_text_updates_world_through_event_dispatcher() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockUpdate(BlockUpdate {
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        block_state_id: vanilla_block_state_id(
            "minecraft:oak_sign",
            [("rotation", "0"), ("waterlogged", "false")],
        ),
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockEntityData(
        BlockEntityData {
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
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );

    let pos = BlockPos {
        x: 16,
        y: -64,
        z: -32,
    };
    assert_eq!(
        world.sign_text_lines(pos, true),
        Some([
            "Front A".to_string(),
            "Front B".to_string(),
            "Front C".to_string(),
            "Front D".to_string(),
        ])
    );
    assert_eq!(
        world.sign_text_lines(pos, false),
        Some([
            "Back A".to_string(),
            "Back B".to_string(),
            "Back C".to_string(),
            "Back D".to_string(),
        ])
    );
    assert_eq!(world.counters().block_entity_updates_applied, 1);

    let scene = sign_scene_from_world(&world, Some(&NativeItemRuntime::empty_for_test()), false);
    assert_eq!(scene.instances.len(), 1);
    assert_eq!(scene.text_surfaces.len(), 2);
    assert!(scene
        .text_surfaces
        .iter()
        .all(|surface| surface.vertex_count() > 0));
}

#[test]
fn terrain_chunk_ignored_counters_stay_in_world_store() {
    let (tx, mut rx) = mpsc::channel(6);
    tx.try_send(NetEvent::Play(PlayClientbound::BlockUpdate(BlockUpdate {
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        block_state_id: 5,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SectionBlocksUpdate(
        SectionBlocksUpdate {
            section_x: 1,
            section_y: -4,
            section_z: -2,
            updates: vec![BlockUpdate {
                pos: ProtocolBlockPos {
                    x: 17,
                    y: -64,
                    z: -31,
                },
                block_state_id: 6,
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockEntityData(
        BlockEntityData {
            pos: ProtocolBlockPos {
                x: 16,
                y: -64,
                z: -32,
            },
            block_entity_type_id:
                bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
            raw_nbt: vec![0],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LightUpdate(LightUpdate {
        chunk_x: 1,
        chunk_z: -2,
        light_data: empty_light_update_data(),
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ChunksBiomes(
        ChunksBiomes {
            chunks: vec![ChunkBiomeData {
                pos: ProtocolChunkPos { x: 1, z: -2 },
                raw_biomes: Vec::new(),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ForgetLevelChunk(
        ForgetLevelChunk {
            pos: ProtocolChunkPos { x: 1, z: -2 },
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        6
    );
    assert!(world.probe_chunk(ChunkPos { x: 1, z: -2 }).is_none());

    let world_counters = world.counters();
    macro_rules! assert_chunk_counter {
        ($field:ident, $value:expr) => {
            assert_eq!(world_counters.$field, $value);
        };
    }

    assert_chunk_counter!(block_updates_received, 2);
    assert_chunk_counter!(block_updates_applied, 0);
    assert_chunk_counter!(block_updates_ignored, 2);
    assert_chunk_counter!(block_entity_updates_received, 1);
    assert_chunk_counter!(block_entity_updates_applied, 0);
    assert_chunk_counter!(block_entity_updates_ignored, 1);
    assert_chunk_counter!(light_updates_received, 1);
    assert_chunk_counter!(light_updates_applied, 0);
    assert_chunk_counter!(light_updates_ignored, 1);
    assert_chunk_counter!(biome_updates_received, 1);
    assert_chunk_counter!(biome_updates_applied, 0);
    assert_chunk_counter!(biome_updates_ignored, 1);
    assert_chunk_counter!(world_apply_errors, 0);
    assert_chunk_counter!(chunk_forgets_received, 1);
    assert_chunk_counter!(chunks_forgotten, 0);
    assert_chunk_counter!(chunk_forgets_ignored, 1);
    assert!(world.apply_diagnostics().apply_errors.is_empty());
    assert!(counters.last_error.is_none());
}

#[test]
fn terrain_apply_errors_stay_in_world_store() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockEntityData(
        BlockEntityData {
            pos: ProtocolBlockPos {
                x: 16,
                y: -64,
                z: -32,
            },
            block_entity_type_id:
                bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
            raw_nbt: vec![1],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );

    assert!(counters.last_error.is_none());
    assert_eq!(world.counters().world_apply_errors, 1);
    let diagnostics = world.apply_diagnostics();
    assert_eq!(diagnostics.apply_errors.len(), 1);
    assert_eq!(diagnostics.apply_errors[0].source, "block_entity_data");
    assert!(
        diagnostics.apply_errors[0].message.contains("nbt byte"),
        "{:?}",
        diagnostics.apply_errors
    );
}

#[test]
fn terrain_chunk_decode_errors_stay_in_world_store() {
    let (tx, mut rx) = mpsc::channel(1);
    let mut chunk = synthetic_native_level_chunk_packet();
    chunk.chunk_data.section_data = vec![0xff];
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(chunk)))
        .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert!(counters.last_error.is_none());
    assert_eq!(world.counters().chunks_received, 0);
    assert_eq!(world.counters().world_apply_errors, 1);
    let diagnostics = world.apply_diagnostics();
    assert_eq!(diagnostics.apply_errors.len(), 1);
    assert_eq!(diagnostics.apply_errors[0].source, "level_chunk_with_light");
}

#[test]
fn terrain_biome_decode_errors_stay_in_world_store() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ChunksBiomes(
        ChunksBiomes {
            chunks: vec![ChunkBiomeData {
                pos: ProtocolChunkPos { x: 1, z: -2 },
                raw_biomes: vec![65],
            }],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );

    assert!(counters.last_error.is_none());
    assert_eq!(world.counters().chunks_received, 1);
    assert_eq!(world.counters().biome_updates_received, 1);
    assert_eq!(world.counters().world_apply_errors, 1);
    let diagnostics = world.apply_diagnostics();
    assert_eq!(diagnostics.apply_errors.len(), 1);
    assert_eq!(diagnostics.apply_errors[0].source, "chunks_biomes");
}

#[test]
fn respawn_clears_world_first_chunk_when_world_changes() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();

    let mut spawn_info = protocol_play_login(9).common_spawn_info;
    spawn_info.dimension_type_id = 1;
    spawn_info.dimension = "minecraft:the_nether".to_string();
    tx.try_send(NetEvent::Play(PlayClientbound::Respawn(Respawn {
        common_spawn_info: spawn_info,
        data_to_keep: 0,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(55));
    world.apply_update_mob_effect(protocol_update_mob_effect(55, 3));
    assert!(
        world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
            id: 4,
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5,
            },
            progress: 6,
        })
    );
    world.apply_block_event(bbb_protocol::packets::BlockEvent {
        pos: ProtocolBlockPos {
            x: 12,
            y: 65,
            z: -5,
        },
        b0: 2,
        b1: 9,
        block_id: 54,
    });
    world.apply_level_event(bbb_protocol::packets::LevelEvent {
        event_type: 1001,
        pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
        data: 42,
        global: true,
    });
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert_eq!(world.first_chunk(), None);
    assert_eq!(world.counters().respawns_received, 1);
    assert_eq!(world.counters().chunks_received, 1);
    assert_eq!(world.counters().chunks_decoded, 1);
    assert_eq!(world.counters().entities_tracked, 0);
    assert_eq!(world.counters().active_mob_effects_tracked, 0);
    assert_eq!(world.counters().block_destructions_tracked, 0);
    assert_eq!(world.counters().block_events_tracked, 0);
    assert_eq!(world.counters().level_events_tracked, 0);
}

#[test]
fn configuration_state_change_clears_client_level_state() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::StateChanged {
        state: bbb_net::ConnectionState::Configuration,
    })
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_login(&protocol_play_login(9));
    world.apply_add_entity(protocol_add_entity(55));
    assert!(
        world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
            id: 4,
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5,
            },
            progress: 6,
        })
    );
    world.apply_block_event(bbb_protocol::packets::BlockEvent {
        pos: ProtocolBlockPos {
            x: 12,
            y: 65,
            z: -5,
        },
        b0: 2,
        b1: 9,
        block_id: 54,
    });
    world.apply_level_event(bbb_protocol::packets::LevelEvent {
        event_type: 1001,
        pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
        data: 42,
        global: true,
    });
    world.set_local_using_item(true);
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );

    assert_eq!(counters.state.as_deref(), Some("Configuration"));
    assert_eq!(world.first_chunk(), None);
    assert_eq!(world.entity_count(), 0);
    assert_eq!(
        world.local_player(),
        &bbb_world::LocalPlayerState::default()
    );
    assert_eq!(world.counters().chunks_received, 1);
    assert_eq!(world.counters().chunks_decoded, 1);
    assert_eq!(world.counters().entities_tracked, 0);
    assert_eq!(world.counters().block_destructions_tracked, 0);
    assert_eq!(world.counters().block_events_tracked, 0);
    assert_eq!(world.counters().level_events_tracked, 0);
}

#[test]
fn start_configuration_flushes_pending_chat_acknowledgement_before_clearing_level() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::StartConfiguration).unwrap();

    let mut world = WorldStore::new();
    assert_eq!(
        world.apply_player_chat(player_chat_with_signature(
            0,
            MessageSignature {
                bytes: vec![7; 256],
            },
        )),
        None
    );
    world.apply_add_entity(protocol_add_entity(55));
    world.set_local_using_item(true);
    let (command_tx, mut command_rx) = mpsc::channel(1);
    let net_commands = Some(command_tx);
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &net_commands),
        1
    );

    assert_eq!(
        command_rx.try_recv().unwrap(),
        NetCommand::AcknowledgeConfiguration {
            pending_chat_acknowledgement: Some(bbb_protocol::packets::ChatAcknowledgement {
                offset: 1
            }),
        }
    );
    assert_eq!(world.entity_count(), 0);
    assert_eq!(
        world.local_player(),
        &bbb_world::LocalPlayerState::default()
    );
    assert_eq!(
        world.counters().player_chat_acknowledgement_pending_offset,
        0
    );
    assert_eq!(world.counters().player_chat_acknowledgement_packets, 1);
}

#[test]
fn play_reconfiguration_clears_level_and_stops_the_previous_audio_generation() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::StartConfiguration).unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(55));
    let prior_level_generation = world.client_level_generation();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio),),
        1
    );

    assert_eq!(
        world.client_level_generation(),
        prior_level_generation.saturating_add(1)
    );
    assert_eq!(world.entity_count(), 0);
    assert_eq!(
        audio.commands,
        vec![AudioCommand::StopSound(bbb_audio::StopSoundCommand {
            category: None,
            name: None,
        })]
    );
}

#[test]
fn play_reconfiguration_protocol_switch_does_not_clear_the_level_twice() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::StartConfiguration).unwrap();
    tx.try_send(NetEvent::StateChanged {
        state: bbb_net::ConnectionState::Configuration,
    })
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_login(&protocol_play_login(9));
    let prior_level_generation = world.client_level_generation();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio),),
        2
    );
    assert_eq!(
        world.client_level_generation(),
        prior_level_generation.saturating_add(1)
    );
    assert_eq!(audio.commands.len(), 1);
    assert_eq!(counters.state.as_deref(), Some("Configuration"));
}

#[test]
fn initial_configuration_transition_does_not_stop_a_nonexistent_level_audio_generation() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::StateChanged {
        state: bbb_net::ConnectionState::Configuration,
    })
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio),),
        1
    );
    assert!(audio.commands.is_empty());
}

#[test]
fn start_configuration_accepts_delayed_chat_before_final_acknowledgement() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerChat(
        player_chat_with_signature(
            0,
            MessageSignature {
                bytes: vec![8; 256],
            },
        ),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerChat(
        player_chat_with_signature(
            1,
            MessageSignature {
                bytes: vec![9; 256],
            },
        ),
    )))
    .unwrap();
    tx.try_send(NetEvent::StartConfiguration).unwrap();

    let mut world = WorldStore::new();
    world.set_chat_presentation_config(bbb_world::ChatPresentationConfig {
        message_delay_millis: 6_000,
        ..bbb_world::ChatPresentationConfig::default()
    });
    let (command_tx, mut command_rx) = mpsc::channel(2);
    let net_commands = Some(command_tx);
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &net_commands),
        3
    );

    assert_eq!(world.client_chat().messages.len(), 2);
    assert!(world.client_chat().delayed_messages.is_empty());
    assert_eq!(
        command_rx.try_recv().unwrap(),
        NetCommand::AcknowledgeConfiguration {
            pending_chat_acknowledgement: Some(bbb_protocol::packets::ChatAcknowledgement {
                offset: 2,
            }),
        }
    );
}
