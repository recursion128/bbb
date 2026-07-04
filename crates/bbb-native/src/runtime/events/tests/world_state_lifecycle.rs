use super::*;

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
        block_state_id: 5,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SectionBlocksUpdate(
        SectionBlocksUpdate {
            section_x: 1,
            section_y: 0,
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
            block_entity_type_id: 7,
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
    assert_chunk_counter!(sections_decoded, 1);
    assert_chunk_counter!(block_entities_seen, 1);
    assert_chunk_counter!(light_arrays_seen, 0);
    assert_chunk_counter!(block_updates_received, 2);
    assert_chunk_counter!(block_updates_applied, 2);
    assert_chunk_counter!(block_updates_ignored, 0);
    assert_chunk_counter!(block_entity_updates_received, 1);
    assert_chunk_counter!(block_entity_updates_applied, 1);
    assert_chunk_counter!(block_entity_updates_ignored, 0);
    assert_chunk_counter!(light_updates_received, 1);
    assert_chunk_counter!(light_updates_applied, 1);
    assert_chunk_counter!(light_updates_ignored, 0);
    assert_chunk_counter!(biome_updates_received, 1);
    assert_chunk_counter!(biome_updates_applied, 1);
    assert_chunk_counter!(biome_updates_ignored, 0);
    assert_chunk_counter!(chunk_forgets_received, 1);
    assert_chunk_counter!(chunks_forgotten, 1);
    assert_chunk_counter!(chunk_forgets_ignored, 0);
}

#[test]
fn block_entity_data_sign_text_updates_world_through_event_dispatcher() {
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
            block_entity_type_id: 7,
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
        2
    );

    let pos = BlockPos {
        x: 16,
        y: -64,
        z: -32,
    };
    assert_eq!(
        world.sign_text_lines(pos, true),
        Some(&[
            "Front A".to_string(),
            "Front B".to_string(),
            "Front C".to_string(),
            "Front D".to_string(),
        ])
    );
    assert_eq!(
        world.sign_text_lines(pos, false),
        Some(&[
            "Back A".to_string(),
            "Back B".to_string(),
            "Back C".to_string(),
            "Back D".to_string(),
        ])
    );
    assert_eq!(world.counters().block_entity_updates_applied, 1);
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
            section_y: 0,
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
            block_entity_type_id: 7,
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
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::BlockEntityData(
        BlockEntityData {
            pos: ProtocolBlockPos { x: 0, y: 64, z: 0 },
            block_entity_type_id: 7,
            raw_nbt: vec![1],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
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
    let (ack_tx, mut ack_rx) = tokio::sync::oneshot::channel();
    tx.try_send(NetEvent::StartConfiguration {
        pending_chat_acknowledgement: ack_tx,
    })
    .unwrap();

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
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(
        ack_rx.try_recv().unwrap(),
        Some(bbb_protocol::packets::ChatAcknowledgement { offset: 1 })
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
