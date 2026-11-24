use super::*;

#[tokio::test]
async fn play_local_player_state_packets_emit_matching_events_without_responses() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(7);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let mut stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Play,
            player_position_state: PlayerPositionState::default(),
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };

    let abilities = packets::PlayerAbilities {
        invulnerable: true,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    };
    stream
        .handle_play_packet(PlayClientbound::PlayerAbilities(abilities))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("player abilities event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::PlayerAbilities(update)) if update == abilities)
    );

    let health = packets::PlayerHealth {
        health: 7.5,
        food: 16,
        saturation: 2.0,
    };
    stream
        .handle_play_packet(PlayClientbound::SetHealth(health))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("player health event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::SetHealth(update)) if update == health)
    );

    let experience = packets::PlayerExperience {
        progress: 0.75,
        level: 8,
        total: 123,
    };
    stream
        .handle_play_packet(PlayClientbound::SetExperience(experience))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("player experience event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::SetExperience(update)) if update == experience
    ));

    let held_slot = packets::SetHeldSlot { slot: 5 };
    stream
        .handle_play_packet(PlayClientbound::SetHeldSlot(held_slot))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("held slot event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::SetHeldSlot(update)) if update == held_slot)
    );
    assert_eq!(stream.conn.serverbound_carried_hotbar_slot(), Some(5));

    let invalid_held_slot = packets::SetHeldSlot { slot: 99 };
    stream
        .handle_play_packet(PlayClientbound::SetHeldSlot(invalid_held_slot))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("invalid held slot event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::SetHeldSlot(update)) if update == invalid_held_slot)
    );
    assert_eq!(
        stream.conn.serverbound_carried_hotbar_slot(),
        Some(5),
        "vanilla ignores invalid clientbound held slots"
    );

    let spawn = packets::SetDefaultSpawnPosition {
        dimension: "minecraft:overworld".to_string(),
        pos: packets::BlockPos {
            x: -5,
            y: 70,
            z: 12,
        },
        yaw: 90.0,
        pitch: -10.0,
    };
    stream
        .handle_play_packet(PlayClientbound::SetDefaultSpawnPosition(spawn.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("default spawn event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::SetDefaultSpawnPosition(update)) if update == spawn
    ));

    let simulation_distance = packets::SetSimulationDistance { distance: 12 };
    stream
        .handle_play_packet(PlayClientbound::SetSimulationDistance(simulation_distance))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("simulation distance event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::SetSimulationDistance(update)) if update == simulation_distance
    ));

    let look_at = packets::PlayerLookAt {
        from_anchor: packets::EntityAnchor::Eyes,
        position: packets::Vec3d {
            x: 12.0,
            y: 65.0,
            z: -7.0,
        },
        target: Some(packets::PlayerLookAtTarget {
            entity_id: 99,
            to_anchor: packets::EntityAnchor::Feet,
        }),
    };
    stream
        .handle_play_packet(PlayClientbound::PlayerLookAt(look_at))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("player look-at event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::PlayerLookAt(update)) if update == look_at)
    );

    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "local-player state packets must not send serverbound responses"
    );
}

#[tokio::test]
async fn play_level_state_packets_emit_matching_events_without_responses() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(8);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let mut stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Play,
            player_position_state: PlayerPositionState::default(),
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };

    let time = packets::PlayTime {
        game_time: 123,
        clock_updates: vec![packets::ClockUpdate {
            clock_id: 0,
            total_ticks: 6000,
            partial_tick: 0.25,
            rate: 1.0,
        }],
    };
    stream
        .handle_play_packet(PlayClientbound::SetTime(time.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("world time event should be emitted")
        .unwrap();
    assert!(matches!(event, NetEvent::Play(PlayClientbound::SetTime(update)) if update == time));

    let game_event = packets::GameEvent {
        event_id: 7,
        param: 0.5,
    };
    stream
        .handle_play_packet(PlayClientbound::GameEvent(game_event))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("game event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::GameEvent(update)) if update == game_event)
    );

    let ticking_state = packets::TickingState {
        tick_rate: 0.25,
        frozen: true,
    };
    stream
        .handle_play_packet(PlayClientbound::TickingState(ticking_state))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("ticking state event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::TickingState(update)) if update == ticking_state
    ));

    let ticking_step = packets::TickingStep { tick_steps: 7 };
    stream
        .handle_play_packet(PlayClientbound::TickingStep(ticking_step))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("ticking step event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::TickingStep(update)) if update == ticking_step)
    );

    let ack = packets::BlockChangedAck { sequence: 17 };
    stream
        .handle_play_packet(PlayClientbound::BlockChangedAck(ack))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("block changed ack event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::BlockChangedAck(update)) if update == ack)
    );

    let block_entity = packets::BlockEntityData {
        pos: packets::BlockPos {
            x: 12,
            y: 65,
            z: -5,
        },
        block_entity_type_id:
            bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_JUKEBOX_ID,
        raw_nbt: vec![10, 0],
    };
    stream
        .handle_play_packet(PlayClientbound::BlockEntityData(block_entity.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("block entity data event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::BlockEntityData(update)) if update == block_entity
    ));

    let block_event = packets::BlockEvent {
        pos: packets::BlockPos {
            x: 12,
            y: 65,
            z: -5,
        },
        b0: 2,
        b1: 9,
        block_id: 54,
    };
    stream
        .handle_play_packet(PlayClientbound::BlockEvent(block_event))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("block event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::BlockEvent(update)) if update == block_event)
    );

    let level_event = packets::LevelEvent {
        event_type: 1001,
        pos: packets::BlockPos { x: 3, y: 4, z: 5 },
        data: 42,
        global: true,
    };
    stream
        .handle_play_packet(PlayClientbound::LevelEvent(level_event))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("level event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::LevelEvent(update)) if update == level_event)
    );

    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "level state packets must not send serverbound responses"
    );
}

#[tokio::test]
async fn play_chunk_terrain_packets_emit_matching_events_without_responses() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(8);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let mut stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Play,
            player_position_state: PlayerPositionState::default(),
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };

    let chunk = packets::LevelChunkWithLight {
        x: 1,
        z: -2,
        chunk_data: packets::LevelChunkData {
            heightmaps: vec![packets::ChunkHeightmapData {
                kind_id: VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
                data: vec![42],
            }],
            section_data: vec![0, 1, 2],
            block_entities: vec![packets::LevelChunkBlockEntity {
                packed_xz: 0,
                y: -64,
                block_entity_type_id:
                    bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
                raw_nbt: vec![0],
            }],
        },
        light_data: packets::LightUpdateData {
            sky_y_mask: Vec::new(),
            block_y_mask: Vec::new(),
            empty_sky_y_mask: Vec::new(),
            empty_block_y_mask: Vec::new(),
            sky_updates: Vec::new(),
            block_updates: Vec::new(),
        },
    };
    stream
        .handle_play_packet(PlayClientbound::LevelChunkWithLight(chunk.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("level chunk event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::LevelChunkWithLight(update)) if update == chunk
    ));

    let light = packets::LightUpdate {
        chunk_x: 1,
        chunk_z: -2,
        light_data: packets::LightUpdateData {
            sky_y_mask: vec![0b10],
            block_y_mask: vec![0b10],
            empty_sky_y_mask: Vec::new(),
            empty_block_y_mask: Vec::new(),
            sky_updates: vec![vec![4; 2048]],
            block_updates: vec![vec![13; 2048]],
        },
    };
    stream
        .handle_play_packet(PlayClientbound::LightUpdate(light.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("light update event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::LightUpdate(update)) if update == light)
    );

    let biomes = packets::ChunksBiomes {
        chunks: vec![packets::ChunkBiomeData {
            pos: packets::ChunkPos { x: 1, z: -2 },
            raw_biomes: vec![0, 7],
        }],
    };
    stream
        .handle_play_packet(PlayClientbound::ChunksBiomes(biomes.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("chunk biome event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::ChunksBiomes(update)) if update == biomes)
    );

    let forget = packets::ForgetLevelChunk {
        pos: packets::ChunkPos { x: 1, z: -2 },
    };
    stream
        .handle_play_packet(PlayClientbound::ForgetLevelChunk(forget))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("chunk forget event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::ForgetLevelChunk(update)) if update == forget
    ));

    let block = packets::BlockUpdate {
        pos: packets::BlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        block_state_id: 9,
    };
    stream
        .handle_play_packet(PlayClientbound::BlockUpdate(block))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("block update event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::BlockUpdate(update)) if update == block)
    );

    let section = packets::SectionBlocksUpdate {
        section_x: 1,
        section_y: -4,
        section_z: -2,
        updates: vec![
            packets::BlockUpdate {
                pos: packets::BlockPos {
                    x: 17,
                    y: -64,
                    z: -32,
                },
                block_state_id: 9,
            },
            packets::BlockUpdate {
                pos: packets::BlockPos {
                    x: 18,
                    y: -64,
                    z: -32,
                },
                block_state_id: 9,
            },
        ],
    };
    stream
        .handle_play_packet(PlayClientbound::SectionBlocksUpdate(section.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("section blocks update event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::SectionBlocksUpdate(update)) if update == section
    ));

    let center = packets::SetChunkCacheCenter {
        chunk_x: 1,
        chunk_z: -2,
    };
    stream
        .handle_play_packet(PlayClientbound::SetChunkCacheCenter(center))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("chunk cache center event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::SetChunkCacheCenter(update)) if update == center
    ));

    let radius = packets::SetChunkCacheRadius { radius: 7 };
    stream
        .handle_play_packet(PlayClientbound::SetChunkCacheRadius(radius))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("chunk cache radius event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::SetChunkCacheRadius(update)) if update == radius
    ));

    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "chunk terrain packets must not send serverbound responses"
    );
}

#[tokio::test]
async fn play_border_and_scoreboard_packets_emit_matching_events_without_responses() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(11);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let mut stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Play,
            player_position_state: PlayerPositionState::default(),
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };

    macro_rules! assert_matching_event {
        ($packet:expr, $message:literal, $pattern:pat $(if $guard:expr)? ) => {{
            stream.handle_play_packet($packet).await.unwrap();
            let event = timeout(Duration::from_secs(1), events_rx.recv())
                .await
                .expect($message)
                .unwrap();
            assert!(matches!(event, $pattern $(if $guard)?));
        }};
    }

    let initialize_border = packets::InitializeBorder {
        new_center_x: 1.0,
        new_center_z: 2.0,
        old_size: 100.0,
        new_size: 200.0,
        lerp_time: 30,
        new_absolute_max_size: 500,
        warning_blocks: 6,
        warning_time: 7,
    };
    assert_matching_event!(
        PlayClientbound::InitializeBorder(initialize_border),
        "initialize border event should be emitted",
        NetEvent::Play(PlayClientbound::InitializeBorder(update)) if update == initialize_border
    );

    let border_center = packets::SetBorderCenter {
        new_center_x: 3.0,
        new_center_z: 4.0,
    };
    assert_matching_event!(
        PlayClientbound::SetBorderCenter(border_center),
        "border center event should be emitted",
        NetEvent::Play(PlayClientbound::SetBorderCenter(update)) if update == border_center
    );

    let border_lerp = packets::SetBorderLerpSize {
        old_size: 200.0,
        new_size: 300.0,
        lerp_time: 50,
    };
    assert_matching_event!(
        PlayClientbound::SetBorderLerpSize(border_lerp),
        "border lerp size event should be emitted",
        NetEvent::Play(PlayClientbound::SetBorderLerpSize(update)) if update == border_lerp
    );

    let border_size = packets::SetBorderSize { size: 250.0 };
    assert_matching_event!(
        PlayClientbound::SetBorderSize(border_size),
        "border size event should be emitted",
        NetEvent::Play(PlayClientbound::SetBorderSize(update)) if update == border_size
    );

    let warning_delay = packets::SetBorderWarningDelay { warning_delay: 9 };
    assert_matching_event!(
        PlayClientbound::SetBorderWarningDelay(warning_delay),
        "border warning delay event should be emitted",
        NetEvent::Play(PlayClientbound::SetBorderWarningDelay(update)) if update == warning_delay
    );

    let warning_distance = packets::SetBorderWarningDistance { warning_blocks: 8 };
    assert_matching_event!(
        PlayClientbound::SetBorderWarningDistance(warning_distance),
        "border warning distance event should be emitted",
        NetEvent::Play(PlayClientbound::SetBorderWarningDistance(update)) if update == warning_distance
    );

    let objective = packets::SetObjective {
        objective_name: "kills".into(),
        method: packets::SetObjectiveMethod::Add.into(),
        parameters: Some(packets::SetObjectiveParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Kills"),
            render_type: packets::ObjectiveRenderType::Integer,
            number_format: Some(packets::NumberFormatSummary::Blank),
        }),
    };
    assert_matching_event!(
        PlayClientbound::SetObjective(objective.clone()),
        "scoreboard objective event should be emitted",
        NetEvent::Play(PlayClientbound::SetObjective(update)) if update == objective
    );

    let display_objective = packets::SetDisplayObjective {
        slot: packets::ScoreboardDisplaySlot::Sidebar,
        objective_name: Some("kills".into()),
    };
    assert_matching_event!(
        PlayClientbound::SetDisplayObjective(display_objective.clone()),
        "scoreboard display objective event should be emitted",
        NetEvent::Play(PlayClientbound::SetDisplayObjective(update)) if update == display_objective
    );

    let score = packets::SetScore {
        owner: "Steve".into(),
        objective_name: "kills".into(),
        score: 4,
        display: Some(bbb_protocol::RichTextComponent::plain("Four")),
        number_format: None,
    };
    assert_matching_event!(
        PlayClientbound::SetScore(score.clone()),
        "scoreboard score event should be emitted",
        NetEvent::Play(PlayClientbound::SetScore(update)) if update == score
    );

    let team = packets::SetPlayerTeam {
        name: "red".into(),
        method: packets::PlayerTeamMethod::Add.into(),
        parameters: Some(packets::PlayerTeamParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Red Team"),
            options: 0b11,
            nametag_visibility: packets::TeamVisibility::Always,
            collision_rule: packets::TeamCollisionRule::Never,
            color: packets::ChatFormatting::Red,
            player_prefix: bbb_protocol::RichTextComponent::plain("[R]"),
            player_suffix: bbb_protocol::RichTextComponent::plain("!"),
        }),
        players: vec!["Steve".into()],
    };
    assert_matching_event!(
        PlayClientbound::SetPlayerTeam(team.clone()),
        "scoreboard team event should be emitted",
        NetEvent::Play(PlayClientbound::SetPlayerTeam(update)) if update == team
    );

    let reset_score = packets::ResetScore {
        owner: "Alex".into(),
        objective_name: Some("kills".into()),
    };
    assert_matching_event!(
        PlayClientbound::ResetScore(reset_score.clone()),
        "scoreboard reset score event should be emitted",
        NetEvent::Play(PlayClientbound::ResetScore(update)) if update == reset_score
    );

    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "border and scoreboard packets must not send serverbound responses"
    );
}

#[tokio::test]
async fn play_debug_game_packets_emit_matching_events_without_responses() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(8);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let mut stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Play,
            player_position_state: PlayerPositionState::default(),
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };

    let debug_block = packets::DebugBlockValue {
        pos: packets::BlockPos { x: 1, y: 64, z: -2 },
        update: packets::DebugSubscriptionUpdate {
            subscription_id: 6,
            value: Some(packets::DebugSubscriptionValue::EntityBlockIntersection(
                packets::DebugEntityBlockIntersection::InAir,
            )),
        },
    };
    stream
        .handle_play_packet(PlayClientbound::DebugBlockValue(debug_block.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("debug block value event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::DebugBlockValue(update)) if update == debug_block
    ));

    let debug_chunk = packets::DebugChunkValue {
        pos: packets::ChunkPos { x: 3, z: -4 },
        update: packets::DebugSubscriptionUpdate {
            subscription_id: 11,
            value: None,
        },
    };
    stream
        .handle_play_packet(PlayClientbound::DebugChunkValue(debug_chunk.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("debug chunk value event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::DebugChunkValue(update)) if update == debug_chunk
    ));

    let debug_entity = packets::DebugEntityValue {
        entity_id: 123,
        update: packets::DebugSubscriptionUpdate {
            subscription_id: 3,
            value: None,
        },
    };
    stream
        .handle_play_packet(PlayClientbound::DebugEntityValue(debug_entity.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("debug entity value event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::DebugEntityValue(update)) if update == debug_entity
    ));

    let debug_event = packets::DebugEvent {
        event: packets::DebugSubscriptionEvent {
            subscription_id: 14,
            value: packets::DebugSubscriptionValue::NeighborUpdate(packets::BlockPos {
                x: 4,
                y: 5,
                z: 6,
            }),
        },
    };
    stream
        .handle_play_packet(PlayClientbound::DebugEvent(debug_event.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("debug event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::DebugEvent(update)) if update == debug_event)
    );

    let debug_sample = packets::DebugSample {
        sample: vec![100, -50],
        sample_type: packets::RemoteDebugSampleType::TickTime,
    };
    stream
        .handle_play_packet(PlayClientbound::DebugSample(debug_sample.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("debug sample event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::DebugSample(update)) if update == debug_sample
    ));

    let game_rules = packets::GameRuleValues {
        values: vec![
            packets::GameRuleValue {
                rule: "minecraft:do_daylight_cycle".to_string(),
                value: "false".to_string(),
            },
            packets::GameRuleValue {
                rule: "minecraft:random_tick_speed".to_string(),
                value: "3".to_string(),
            },
        ],
    };
    stream
        .handle_play_packet(PlayClientbound::GameRuleValues(game_rules.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("game rule values event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::GameRuleValues(update)) if update == game_rules
    ));

    let game_test_highlight = packets::GameTestHighlightPos {
        absolute_pos: packets::BlockPos {
            x: -10,
            y: 70,
            z: 22,
        },
        relative_pos: packets::BlockPos { x: 1, y: 2, z: 3 },
    };
    stream
        .handle_play_packet(PlayClientbound::GameTestHighlightPos(game_test_highlight))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("game test highlight event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::GameTestHighlightPos(update)) if update == game_test_highlight
    ));

    let test_instance_status = packets::TestInstanceBlockStatus {
        status: bbb_protocol::RichTextComponent::plain("Ready"),
        size: Some(packets::Vec3i { x: 3, y: 4, z: 5 }),
    };
    stream
        .handle_play_packet(PlayClientbound::TestInstanceBlockStatus(
            test_instance_status.clone(),
        ))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("test instance block status event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::TestInstanceBlockStatus(update)) if update == test_instance_status
    ));

    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "debug/game-state packets must not send serverbound responses"
    );
}

#[tokio::test]
async fn play_passive_world_apply_packets_emit_matching_events() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(8);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let mut stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Play,
            player_position_state: PlayerPositionState::default(),
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };

    let projectile_power = packets::ProjectilePower {
        entity_id: 42,
        acceleration_power: 1.5,
    };
    stream
        .handle_play_packet(PlayClientbound::ProjectilePower(projectile_power))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("projectile power event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::ProjectilePower(update)) if update == projectile_power
    ));

    let waypoint = packets::TrackedWaypointPacket {
        operation: packets::WaypointOperation::Track,
        waypoint: packets::TrackedWaypoint {
            identifier: packets::WaypointIdentifier::Name("base".to_string()),
            icon: packets::WaypointIcon {
                style: "minecraft:default".to_string(),
                color_rgb: Some(0x33_66_99),
            },
            data: packets::WaypointData::Position(packets::WaypointVec3i {
                x: 12,
                y: 64,
                z: -8,
            }),
        },
    };
    stream
        .handle_play_packet(PlayClientbound::Waypoint(waypoint.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("waypoint event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::Waypoint(update)) if update == waypoint)
    );

    let recipe_id = packets::RecipeDisplayId { index: 7 };
    let recipe_book_add = packets::RecipeBookAdd {
        entries: vec![packets::RecipeBookAddEntry {
            contents: packets::RecipeDisplayEntry {
                id: recipe_id,
                display: packets::RecipeDisplaySummary {
                    display_type: packets::RecipeDisplayType::Stonecutter,
                    raw_body: vec![1, 2, 3],
                    crafting: None,
                    furnace: None,
                    stonecutter: None,
                    smithing: None,
                },
                group: Some(4),
                category_id: VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_EQUIPMENT_ID,
                crafting_requirements: Some(vec![packets::IngredientSummary {
                    tag: None,
                    item_ids: vec![3, 5],
                }]),
            },
            flags: 0b11,
            notification: true,
            highlight: true,
        }],
        replace: true,
    };
    stream
        .handle_play_packet(PlayClientbound::RecipeBookAdd(recipe_book_add.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("recipe book add event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::RecipeBookAdd(update)) if update == recipe_book_add
    ));

    let recipe_book_remove = packets::RecipeBookRemove {
        recipe_ids: vec![recipe_id],
    };
    stream
        .handle_play_packet(PlayClientbound::RecipeBookRemove(
            recipe_book_remove.clone(),
        ))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("recipe book remove event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::RecipeBookRemove(update)) if update == recipe_book_remove
    ));

    let recipe_book_settings = packets::RecipeBookSettings {
        crafting: packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: packets::RecipeBookTypeSettings {
            open: false,
            filtering: true,
        },
        blast_furnace: packets::RecipeBookTypeSettings {
            open: true,
            filtering: true,
        },
        smoker: packets::RecipeBookTypeSettings {
            open: false,
            filtering: false,
        },
    };
    stream
        .handle_play_packet(PlayClientbound::RecipeBookSettings(recipe_book_settings))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("recipe book settings event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::RecipeBookSettings(update)) if update == recipe_book_settings
    ));

    let update_advancements = packets::UpdateAdvancements {
        reset: true,
        added: Vec::new(),
        removed: vec!["minecraft:story/root".to_string()],
        progress: Vec::new(),
        show_advancements: true,
    };
    stream
        .handle_play_packet(PlayClientbound::UpdateAdvancements(
            update_advancements.clone(),
        ))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("advancements update event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::UpdateAdvancements(update)) if update == update_advancements
    ));

    let select_advancements_tab = packets::SelectAdvancementsTab {
        tab: Some("minecraft:story/root".to_string()),
    };
    stream
        .handle_play_packet(PlayClientbound::SelectAdvancementsTab(
            select_advancements_tab.clone(),
        ))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("select advancements tab event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::SelectAdvancementsTab(update)) if update == select_advancements_tab
    ));

    let update_recipes = packets::UpdateRecipes {
        property_sets: vec![packets::RecipePropertySetSummary {
            key: "minecraft:planks".to_string(),
            item_ids: vec![5, 6],
        }],
        stonecutter_recipes: vec![packets::StonecutterSelectableRecipeSummary {
            input: packets::IngredientSummary {
                tag: Some("minecraft:stone_tool_materials".to_string()),
                item_ids: vec![1],
            },
            option_display: packets::SlotDisplaySummary {
                display_type_id: VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID,
                raw_payload: vec![9, 8, 7],
                item_stack: None,
                tag: None,
            },
        }],
    };
    stream
        .handle_play_packet(PlayClientbound::UpdateRecipes(update_recipes.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("recipes update event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::UpdateRecipes(update)) if update == update_recipes
    ));

    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "passive world-apply packets must not send serverbound responses"
    );
}

#[tokio::test]
async fn play_unknown_packets_emit_unsupported_packet_events() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(1);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let mut stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Play,
            player_position_state: PlayerPositionState::default(),
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };

    stream
        .handle_play_packet(PlayClientbound::Unknown {
            packet_id: 0x7e,
            len: 9,
        })
        .await
        .unwrap();

    assert!(timeout(Duration::from_millis(50), server.read_packet())
        .await
        .is_err());
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("unsupported packet event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::UnsupportedPacket {
            state: ConnectionState::Play,
            packet_id: 0x7e,
            len: 9,
        }
    ));
}

#[tokio::test]
async fn play_bundle_delimiter_is_transport_noop() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(1);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let mut stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Play,
            player_position_state: PlayerPositionState::default(),
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };

    stream
        .handle_play_packet(PlayClientbound::BundleDelimiter)
        .await
        .unwrap();

    assert!(timeout(Duration::from_millis(50), server.read_packet())
        .await
        .is_err());
    assert!(timeout(Duration::from_millis(50), events_rx.recv())
        .await
        .is_err());
}

#[tokio::test]
async fn play_disconnect_packet_returns_disconnect_error() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(1);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let mut stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Play,
            player_position_state: PlayerPositionState::default(),
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };

    let err = stream
        .handle_play_packet(PlayClientbound::Disconnect(packets::Disconnect {
            reason: "Kicked".to_string(),
            styled_reason: Vec::new(),
            raw_reason: Vec::new(),
        }))
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), "play disconnected: Kicked");
    assert!(timeout(Duration::from_millis(50), server.read_packet())
        .await
        .is_err());
    assert!(timeout(Duration::from_millis(50), events_rx.recv())
        .await
        .is_err());
}

#[test]
fn event_stream_result_classifies_only_typed_remote_disconnects() {
    let styled = vec![bbb_protocol::StyledTextRun {
        text: "Kicked".to_string(),
        style: bbb_protocol::ComponentStyle {
            bold: Some(true),
            color: Some(0xAA_00_11),
            ..bbb_protocol::ComponentStyle::default()
        },
    }];
    let remote = classify_event_stream_result(Err(remote_disconnect_error(
        "play",
        "Kicked".to_string(),
        styled,
    )))
    .unwrap();
    let EventStreamExit::RemoteDisconnect(reason) = remote else {
        panic!("typed remote disconnect must not become a local completion");
    };
    assert_eq!(reason.summary(), "Kicked");
    assert!(reason.runs[0].bold);
    assert_eq!(reason.runs[0].color, Some(0xAA_00_11));

    assert_eq!(
        classify_event_stream_result(Ok(())).unwrap(),
        EventStreamExit::LocalDisconnect
    );
    assert!(classify_event_stream_result(Err(anyhow!("transport failed"))).is_err());
}

#[tokio::test]
async fn play_player_position_gates_moves_until_canonical_accept() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(4);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let mut stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Play,
            player_position_state: PlayerPositionState::default(),
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };
    let update = packets::PlayerPositionUpdate {
        id: 17,
        position: packets::Vec3d {
            x: 1.25,
            y: 64.5,
            z: -8.75,
        },
        delta_movement: packets::Vec3d::default(),
        y_rot: 90.0,
        x_rot: -15.0,
        relatives_mask: 0,
    };

    stream
        .handle_play_packet(PlayClientbound::PlayerPosition(update))
        .await
        .unwrap();

    // The packet only records the pending teleport and defers to the
    // canonical world; nothing goes on the wire yet and the connection is
    // free to keep reading.
    assert_eq!(stream.pending_teleports, 1);
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("player position event should be emitted")
        .unwrap();
    match event {
        NetEvent::PlayerPosition { update: emitted } => assert_eq!(emitted, update),
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "the teleport handshake must wait for the canonical accept"
    );

    let acknowledged_state = PlayerPositionState {
        position: packets::Vec3d {
            x: 40.0,
            y: 80.0,
            z: 12.0,
        },
        delta_movement: packets::Vec3d::default(),
        y_rot: 30.0,
        x_rot: 10.0,
    };
    stream
        .accept_teleport(crate::types::AcceptTeleportCommand {
            teleport_id: 17,
            state: acknowledged_state,
        })
        .await
        .unwrap();

    assert_eq!(stream.pending_teleports, 0);
    assert_eq!(stream.player_position_state, acknowledged_state);
    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("teleport ack should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_ACCEPT_TELEPORTATION);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 17);
    assert!(decoder.is_empty());

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("move player pos/rot ack should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT);
    assert_move_player_pos_rot_payload(&payload, 40.0, 80.0, 12.0, 30.0, 10.0, 0);
    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "player position must not bypass native level-load readiness"
    );
}

#[tokio::test]
async fn pending_teleport_drops_player_moves_until_accept_drives_through() {
    let (mut conn, mut server) = raw_connection_pair().await;
    let (commands_tx, mut commands) = mpsc::channel(4);
    let mut player_position_state = PlayerPositionState::default();
    let move_command = crate::types::PlayerMoveCommand {
        state: PlayerPositionState {
            position: packets::Vec3d {
                x: 7.0,
                y: 64.0,
                z: 7.0,
            },
            delta_movement: packets::Vec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
        },
        on_ground: true,
        horizontal_collision: false,
        force_position: false,
        force_pos_rot: true,
        force_rotation_only: false,
    };

    // A stale move queued before the canonical accept is dropped while
    // the teleport is pending.
    commands_tx
        .send(NetCommand::MovePlayer(move_command))
        .await
        .unwrap();
    let accept = crate::types::AcceptTeleportCommand {
        teleport_id: 21,
        state: PlayerPositionState {
            position: packets::Vec3d {
                x: -3.0,
                y: 70.0,
                z: 11.0,
            },
            delta_movement: packets::Vec3d::default(),
            y_rot: 45.0,
            x_rot: -5.0,
        },
    };
    commands_tx
        .send(NetCommand::AcceptTeleport(accept))
        .await
        .unwrap();

    let mut player_movement_flags = crate::session::PlayerMovementFlags::default();
    let drive = read_packet_or_drive_connection(
        &mut conn,
        ConnectionState::Play,
        &mut commands,
        &mut player_position_state,
        &mut player_movement_flags,
        true,
    )
    .await
    .unwrap();
    match drive {
        ConnectionDrive::AcceptTeleport(command) => assert_eq!(command, accept),
        other => panic!("unexpected drive: {other:?}"),
    }
    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "gated moves must not reach the wire while a teleport is pending"
    );

    // With the gate lifted the same move goes through.
    commands_tx
        .send(NetCommand::MovePlayer(move_command))
        .await
        .unwrap();
    let drive = tokio::spawn(async move {
        read_packet_or_drive_connection(
            &mut conn,
            ConnectionState::Play,
            &mut commands,
            &mut player_position_state,
            &mut player_movement_flags,
            false,
        )
        .await
    });
    let (packet_id, _) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("ungated move should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT);
    drive.abort();
}

#[tokio::test]
async fn play_player_rotation_sends_vanilla_rot_ack() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(4);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let mut stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Play,
            player_position_state: PlayerPositionState {
                y_rot: 30.0,
                x_rot: 5.0,
                ..PlayerPositionState::default()
            },
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };
    let update = packets::PlayerRotationUpdate {
        y_rot: 15.0,
        relative_y: true,
        x_rot: -10.0,
        relative_x: false,
    };

    stream
        .handle_play_packet(PlayClientbound::PlayerRotation(update))
        .await
        .unwrap();

    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("player rotation event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::PlayerRotation(event_update)) if event_update == update)
    );
    // Network task must wait for main-thread canonical rotation before sending MovePlayerRot.
    assert!(
        timeout(Duration::from_millis(25), server.read_packet())
            .await
            .is_err(),
        "network task must not reply before canonical main-thread apply"
    );

    stream
        .acknowledge_player_rotation(45.0, -10.0)
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("move player rot ack should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_ROT);
    assert_move_player_rot_payload(&payload, 45.0, -10.0, 0);
    assert_eq!(stream.player_position_state.y_rot, 45.0);
    assert_eq!(stream.player_position_state.x_rot, -10.0);
}

#[tokio::test]
async fn play_player_rotation_acknowledgement_sends_false_movement_flags_per_vanilla() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(4);
    let (commands_tx, commands_rx) = mpsc::channel(4);
    let mut stream =
        EventStreamContext::new(client, NetEventSender::new(events_tx, || {}), commands_rx);
    stream.state = ConnectionState::Play;

    // Drive a player move that updates position, rotation, and movement flags.
    commands_tx
        .send(NetCommand::MovePlayer(crate::types::PlayerMoveCommand {
            state: PlayerPositionState {
                position: packets::Vec3d {
                    x: 4.0,
                    y: 64.0,
                    z: -2.0,
                },
                delta_movement: packets::Vec3d::default(),
                y_rot: 30.0,
                x_rot: 5.0,
            },
            on_ground: true,
            horizontal_collision: true,
            force_position: false,
            force_pos_rot: true,
            force_rotation_only: false,
        }))
        .await
        .unwrap();
    commands_tx.send(NetCommand::Disconnect).await.unwrap();
    let drive = read_packet_or_drive_connection(
        &mut stream.session.conn,
        stream.session.state,
        &mut stream.commands,
        &mut stream.session.player_position_state,
        &mut stream.session.player_movement_flags,
        false,
    )
    .await
    .unwrap();
    assert!(matches!(drive, ConnectionDrive::Disconnect));
    let (move_id, _) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("initial move packet should be sent")
        .unwrap();
    assert_eq!(move_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT);
    assert_eq!(
        stream.player_movement_flags,
        crate::session::PlayerMovementFlags {
            on_ground: true,
            horizontal_collision: true,
        }
    );

    // Vanilla ClientPacketListener.handleRotatePlayer always sends (false, false) flags.
    let update = packets::PlayerRotationUpdate {
        y_rot: 15.0,
        relative_y: true,
        x_rot: -10.0,
        relative_x: false,
    };
    stream
        .handle_play_packet(PlayClientbound::PlayerRotation(update))
        .await
        .unwrap();
    let _ = events_rx.recv().await.unwrap();
    stream
        .acknowledge_player_rotation(45.0, -10.0)
        .await
        .unwrap();
    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("move player rot ack should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_ROT);
    assert_move_player_rot_payload(&payload, 45.0, -10.0, 0);
    assert_eq!(
        stream.player_movement_flags,
        crate::session::PlayerMovementFlags::default()
    );

    // Accepting a teleport also resets movement flags back to false, false.
    stream
        .accept_teleport(crate::types::AcceptTeleportCommand {
            teleport_id: 9,
            state: PlayerPositionState {
                y_rot: 90.0,
                x_rot: 0.0,
                ..PlayerPositionState::default()
            },
        })
        .await
        .unwrap();
    let _ = server.read_packet().await.unwrap();
    let _ = server.read_packet().await.unwrap();
    assert_eq!(
        stream.player_movement_flags,
        crate::session::PlayerMovementFlags::default()
    );
}

#[tokio::test]
async fn play_start_configuration_acknowledges_and_resets_configuration_dedup_state() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(4);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Play,
            player_position_state: PlayerPositionState::default(),
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: true,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };

    let mut stream = stream;
    stream
        .handle_play_packet(PlayClientbound::StartConfiguration)
        .await
        .unwrap();

    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("start-configuration event should be emitted")
        .unwrap();
    assert!(matches!(event, NetEvent::StartConfiguration));
    assert_eq!(stream.state, ConnectionState::Play);
    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "the configuration switch must wait for the canonical acknowledgement"
    );

    stream.acknowledge_start_configuration().await.unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("configuration acknowledgement should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED);
    assert!(payload.is_empty());
    assert_eq!(stream.state, ConnectionState::Configuration);
    assert!(!stream.seen_code_of_conduct);

    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("state-changed event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::StateChanged {
            state: ConnectionState::Configuration
        }
    ));

    stream
        .handle_configuration_packet(packets::ConfigurationClientbound::CodeOfConduct {
            text: "Fresh configuration rules.".to_string(),
        })
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("code-of-conduct event should be emitted after reconfiguration")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::CodeOfConduct { text } if text == "Fresh configuration rules."
    ));
}

#[tokio::test]
async fn play_start_configuration_flushes_pending_chat_acknowledgement_first() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(4);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Play,
            player_position_state: PlayerPositionState::default(),
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: true,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };

    let mut stream = stream;
    stream
        .handle_play_packet(PlayClientbound::StartConfiguration)
        .await
        .unwrap();

    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("start-configuration event should be emitted")
        .unwrap();
    assert!(matches!(event, NetEvent::StartConfiguration));

    // The canonical acknowledgement drives the flush through the same
    // path the run loop uses: chat acknowledgement first, then the
    // configuration switch.
    stream
        .acknowledge_start_configuration_with_chat(Some(ChatAcknowledgement { offset: 1 }))
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
    assert_eq!(stream.state, ConnectionState::Configuration);

    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("state-changed event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::StateChanged {
            state: ConnectionState::Configuration
        }
    ));
}

#[tokio::test]
async fn transfer_reconnect_uses_transfer_intent_and_carries_session_cookies() {
    let transfer_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let transfer_address = transfer_listener.local_addr().unwrap();
    let (mut old_client, mut old_server) = raw_connection_pair().await;
    old_client.set_compression_threshold(32);
    let old_closed = tokio::spawn(async move {
        // Old peer must observe EOF before the transfer target sees the new connection.
        old_server.read_packet_or_eof().await.unwrap()
    });
    let transfer_server = tokio::spawn(async move {
        let (server_stream, _) = transfer_listener.accept().await.unwrap();
        let mut server = RawConnection::from_stream(server_stream, None);
        let handshake = server.read_packet().await.unwrap();
        let hello = server.read_packet().await.unwrap();
        (handshake, hello)
    });
    let (events_tx, mut events_rx) = mpsc::channel(4);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let mut stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: old_client,
            state: ConnectionState::Play,
            player_position_state: PlayerPositionState {
                y_rot: 90.0,
                ..PlayerPositionState::default()
            },
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::from([("minecraft:transfer".to_string(), vec![1, 2, 3])]),
            seen_code_of_conduct: true,
            accepted_code_of_conduct_hash: Some(123),
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };
    let options = ConnectionOptions::offline("old.example.invalid:25565", "bbb-client").unwrap();

    reconnect_for_transfer(
        &mut stream,
        &options,
        Transfer {
            host: transfer_address.ip().to_string().into(),
            port: i32::from(transfer_address.port()),
        },
    )
    .await
    .unwrap();

    assert_eq!(stream.state, ConnectionState::Login);
    assert_eq!(stream.conn.compression_threshold, None);
    assert_eq!(stream.player_position_state, PlayerPositionState::default());
    assert_eq!(
        stream.server_cookies.get("minecraft:transfer"),
        Some(&vec![1, 2, 3])
    );
    assert!(!stream.seen_code_of_conduct);
    assert_eq!(stream.accepted_code_of_conduct_hash, Some(123));
    assert!(matches!(events_rx.recv().await, Some(NetEvent::Connected)));
    assert!(matches!(
        events_rx.recv().await,
        Some(NetEvent::StateChanged {
            state: ConnectionState::Login
        })
    ));

    assert_eq!(
        timeout(Duration::from_secs(1), old_closed)
            .await
            .expect("old connection must close during transfer")
            .unwrap(),
        None
    );

    let ((handshake_id, handshake_payload), (hello_id, hello_payload)) =
        transfer_server.await.unwrap();
    assert_eq!(handshake_id, ids::handshake::SERVERBOUND_CLIENT_INTENTION);
    let mut decoder = Decoder::new(&handshake_payload);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        bbb_protocol::PROTOCOL_VERSION
    );
    assert_eq!(
        decoder.read_string(255).unwrap(),
        transfer_address.ip().to_string()
    );
    assert_eq!(decoder.read_u16().unwrap(), transfer_address.port());
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        ClientIntent::Transfer as i32
    );
    assert!(decoder.is_empty());

    assert_eq!(hello_id, ids::login::SERVERBOUND_HELLO);
    let mut decoder = Decoder::new(&hello_payload);
    assert_eq!(decoder.read_string(16).unwrap(), options.username);
    assert_eq!(decoder.read_uuid().unwrap(), options.profile_id);
    assert!(decoder.is_empty());
}

#[tokio::test]
async fn play_transfer_closing_window_handles_buffered_cookie_and_latest_transfer_only() {
    let (mut client, _server) = raw_connection_pair().await;
    client
        .read_buf
        .extend_from_slice(&encoded_store_cookie_packet(
            ids::play::CLIENTBOUND_STORE_COOKIE,
            "bbb:handoff",
            &[4, 5, 6],
        ));
    let mut ignored_ping = Encoder::new();
    ignored_ping.write_i32(19);
    client.read_buf.extend_from_slice(&encode_packet(
        ids::play::CLIENTBOUND_PING,
        &ignored_ping.into_inner(),
    ));
    client.read_buf.extend_from_slice(&encoded_transfer_packet(
        ids::play::CLIENTBOUND_TRANSFER,
        "latest.example.invalid",
        25567,
    ));

    let (events_tx, mut events_rx) = mpsc::channel(4);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let mut stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Play,
            player_position_state: PlayerPositionState::default(),
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };

    let transfer = stream
        .drain_transfer_closing_packets(
            ConnectionState::Play,
            Transfer {
                host: "first.example.invalid".into(),
                port: 25566,
            },
        )
        .await
        .unwrap();

    assert_eq!(
        transfer,
        Transfer {
            host: "latest.example.invalid".into(),
            port: 25567,
        }
    );
    assert_eq!(
        stream.server_cookies.get("bbb:handoff"),
        Some(&vec![4, 5, 6])
    );
    assert!(matches!(
        events_rx.recv().await,
        Some(NetEvent::StoreCookie {
            key,
            payload_len: 3,
            stored_cookie_count: 1,
        }) if key == "bbb:handoff"
    ));
    assert!(matches!(
        events_rx.recv().await,
        Some(NetEvent::Play(PlayClientbound::Transfer(Transfer {
            host,
            port: 25567,
        }))) if host == "latest.example.invalid"
    ));
    assert!(events_rx.try_recv().is_err());
}

#[tokio::test]
async fn configuration_transfer_closing_window_handles_buffered_cookie_and_latest_transfer_only() {
    let (mut client, _server) = raw_connection_pair().await;
    client
        .read_buf
        .extend_from_slice(&encoded_store_cookie_packet(
            ids::configuration::CLIENTBOUND_STORE_COOKIE,
            "bbb:handoff",
            &[7, 8],
        ));
    let mut ignored_ping = Encoder::new();
    ignored_ping.write_i32(23);
    client.read_buf.extend_from_slice(&encode_packet(
        ids::configuration::CLIENTBOUND_PING,
        &ignored_ping.into_inner(),
    ));
    client.read_buf.extend_from_slice(&encoded_transfer_packet(
        ids::configuration::CLIENTBOUND_TRANSFER,
        "latest.example.invalid",
        25568,
    ));

    let (events_tx, mut events_rx) = mpsc::channel(4);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let mut stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Configuration,
            player_position_state: PlayerPositionState::default(),
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };

    let transfer = stream
        .drain_transfer_closing_packets(
            ConnectionState::Configuration,
            Transfer {
                host: "first.example.invalid".into(),
                port: 25566,
            },
        )
        .await
        .unwrap();

    assert_eq!(
        transfer,
        Transfer {
            host: "latest.example.invalid".into(),
            port: 25568,
        }
    );
    assert_eq!(stream.server_cookies.get("bbb:handoff"), Some(&vec![7, 8]));
    assert!(matches!(
        events_rx.recv().await,
        Some(NetEvent::StoreCookie {
            key,
            payload_len: 2,
            stored_cookie_count: 1,
        }) if key == "bbb:handoff"
    ));
    assert!(matches!(
        events_rx.recv().await,
        Some(NetEvent::Transfer(Transfer {
            host,
            port: 25568,
        })) if host == "latest.example.invalid"
    ));
    assert!(events_rx.try_recv().is_err());
}

#[tokio::test]
async fn configuration_content_registry_updates_play_decoder_and_resets_on_reconfiguration() {
    let (client, _server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(8);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let mut stream =
        EventStreamContext::new(client, NetEventSender::new(events_tx, || {}), commands_rx);
    stream.state = ConnectionState::Configuration;

    let mut builder = ContentRegistryBuilder::new();
    builder.register_block(ModBlockRegistration {
        resource_id: "examplemod:crystal_block".to_string(),
        state_count: 2,
        light_emission: 0,
        destroy_speed: 1.5,
        full_cube_collision: true,
        dynamic_light_rgb: None,
    });
    let registry = builder.freeze().unwrap();
    let wire_bytes = registry.encode_wire();

    stream
        .handle_configuration_packet(packets::ConfigurationClientbound::CustomPayload(
            packets::CustomPayload {
                id: BBB_CONTENT_REGISTRY_CHANNEL.to_string(),
                payload: CustomPayloadBody::Unknown {
                    raw_payload: wire_bytes.clone(),
                },
            },
        ))
        .await
        .unwrap();

    assert!(!stream.content_registry().is_vanilla_only());
    assert_eq!(stream.content_registry().blocks().len(), 1);
    assert!(matches!(
        events_rx.recv().await,
        Some(NetEvent::CustomPayload(payload))
            if payload.id == BBB_CONTENT_REGISTRY_CHANNEL
                && matches!(payload.payload, CustomPayloadBody::Unknown { ref raw_payload } if *raw_payload == wire_bytes)
    ));

    let mod_state = registry.blocks()[0].first_state_id + 1;
    let block_update = packets::BlockUpdate {
        pos: packets::BlockPos { x: 1, y: 64, z: 2 },
        block_state_id: mod_state,
    };
    let (packet_id, packet_payload) =
        packets::encode_play_clientbound_block_update_with_content_registry(
            block_update,
            &registry,
        )
        .unwrap();
    assert!(packets::decode_play_clientbound(packet_id, &packet_payload).is_err());

    stream.finish_configuration().await.unwrap();
    assert!(matches!(
        events_rx.recv().await,
        Some(NetEvent::StateChanged {
            state: ConnectionState::Play
        })
    ));
    assert_eq!(
        stream
            .decode_play_packet(packet_id, &packet_payload)
            .unwrap(),
        PlayClientbound::BlockUpdate(block_update)
    );

    stream.acknowledge_start_configuration().await.unwrap();
    assert_eq!(stream.state, ConnectionState::Configuration);
    assert!(stream.content_registry().is_vanilla_only());
    assert!(matches!(
        events_rx.recv().await,
        Some(NetEvent::StateChanged {
            state: ConnectionState::Configuration
        })
    ));
}

#[tokio::test]
async fn acknowledge_configuration_resets_open_play_bundle_across_reconfig() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(2048);
    let (commands_tx, commands_rx) = mpsc::channel(8);
    let mut stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Play,
            player_position_state: PlayerPositionState::default(),
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };
    let inbound_metrics = stream.events.metrics();
    let options = ConnectionOptions::offline("127.0.0.1:25565", "Player").unwrap();

    let driver = tokio::spawn(async move {
        crate::event_stream::drive_event_stream(&mut stream, &options)
            .await
            .unwrap();
    });

    // Open a Play bundle without closing it, then reconfigure before a closing delimiter arrives.
    server
        .send_packet(ids::play::CLIENTBOUND_BUNDLE_DELIMITER, &[])
        .await
        .unwrap();
    let ping_id = ids::play::CLIENTBOUND_PING;
    server
        .send_packet(ping_id, &111_i32.to_be_bytes())
        .await
        .unwrap();
    server.flush().await.unwrap();

    // Wait until the open bundle has consumed both the delimiter and the buffered ping packet.
    timeout(Duration::from_secs(1), async {
        while inbound_metrics.packets_seen() < 2 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("bundle delimiter and ping packets should be recorded");
    assert!(events_rx.try_recv().is_err());

    commands_tx
        .send(NetCommand::AcknowledgeConfiguration {
            pending_chat_acknowledgement: None,
        })
        .await
        .unwrap();
    let (ack_id, _) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("config ack")
        .unwrap();
    assert_eq!(ack_id, ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED);

    // Re-enter Play via FinishConfiguration and send a normal unbundled Play ping.
    commands_tx
        .send(NetCommand::FinishConfiguration)
        .await
        .unwrap();
    let (finish_id, _) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("finish config")
        .unwrap();
    assert_eq!(
        finish_id,
        ids::configuration::SERVERBOUND_FINISH_CONFIGURATION
    );

    server
        .send_packet(ids::play::CLIENTBOUND_PING, &222_i32.to_be_bytes())
        .await
        .unwrap();
    server.flush().await.unwrap();

    // If the old bundle had not been reset, packet 222 would still be trapped inside the open
    // bundle instead of being immediately processed and replied to with a Pong.
    let (pong_id, pong_payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("fresh ping must be processed immediately after reconfig")
        .unwrap();
    assert_eq!(pong_id, ids::play::SERVERBOUND_PONG);
    assert_eq!(pong_payload, 222_i32.to_be_bytes());

    commands_tx.send(NetCommand::Disconnect).await.unwrap();
    driver.await.unwrap();
}

#[tokio::test]
async fn protocol_swap_pauses_inbound_reads_across_start_and_finish_configuration() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(64);
    let (commands_tx, commands_rx) = mpsc::channel(16);
    let mut stream =
        EventStreamContext::new(client, NetEventSender::new(events_tx, || {}), commands_rx);
    stream.state = ConnectionState::Play;
    let options = ConnectionOptions::offline("127.0.0.1:25565", "Player").unwrap();

    let driver = tokio::spawn(async move {
        crate::event_stream::drive_event_stream(&mut stream, &options)
            .await
            .unwrap();
    });

    // Pipeline Play::StartConfiguration immediately followed by Configuration::KeepAlive
    // in the same flush (mirroring Vanilla `ServerGamePacketListenerImpl.switchToConfig`).
    let config_keep_alive_id = 0x1122_3344_5566_7788_i64;
    server
        .send_packet(ids::play::CLIENTBOUND_START_CONFIGURATION, &[])
        .await
        .unwrap();
    server
        .send_packet(
            ids::configuration::CLIENTBOUND_KEEP_ALIVE,
            &config_keep_alive_id.to_be_bytes(),
        )
        .await
        .unwrap();
    server.flush().await.unwrap();

    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("start configuration event")
        .unwrap();
    assert!(matches!(event, NetEvent::StartConfiguration));

    // Pre-ack Play outbound commands must still drain while inbound reads are paused,
    // and the buffered Configuration KeepAlive must NOT be decoded under Play yet.
    commands_tx
        .send(NetCommand::ChatAcknowledgement(ChatAcknowledgement {
            offset: 3,
        }))
        .await
        .unwrap();
    let (chat_ack_id, chat_ack_payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("pre-ack chat acknowledgement")
        .unwrap();
    assert_eq!(chat_ack_id, ids::play::SERVERBOUND_CHAT_ACK);
    assert_eq!(Decoder::new(&chat_ack_payload).read_var_i32().unwrap(), 3);
    assert!(
        timeout(Duration::from_millis(40), server.read_packet())
            .await
            .is_err(),
        "inbound Configuration packet must stay paused until AcknowledgeConfiguration"
    );

    commands_tx
        .send(NetCommand::AcknowledgeConfiguration {
            pending_chat_acknowledgement: None,
        })
        .await
        .unwrap();
    let (ack_id, ack_payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("configuration ack")
        .unwrap();
    assert_eq!(ack_id, ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED);
    assert!(ack_payload.is_empty());

    // Once resumed in Configuration, the buffered Configuration KeepAlive is decoded and answered.
    let (ka_id, ka_payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("buffered configuration keepalive reply")
        .unwrap();
    assert_eq!(ka_id, ids::configuration::SERVERBOUND_KEEP_ALIVE);
    assert_eq!(ka_payload, config_keep_alive_id.to_be_bytes());
    let state_event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("configuration state change")
        .unwrap();
    assert!(matches!(
        state_event,
        NetEvent::StateChanged {
            state: ConnectionState::Configuration
        }
    ));

    // Pipeline Configuration::Finish immediately followed by Play::Ping.
    let play_ping_id = 0x1357_2468_i32;
    server
        .send_packet(ids::configuration::CLIENTBOUND_FINISH_CONFIGURATION, &[])
        .await
        .unwrap();
    server
        .send_packet(ids::play::CLIENTBOUND_PING, &play_ping_id.to_be_bytes())
        .await
        .unwrap();
    server.flush().await.unwrap();

    let finish_event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("finish configuration event")
        .unwrap();
    assert!(matches!(finish_event, NetEvent::FinishConfiguration));
    assert!(
        timeout(Duration::from_millis(40), server.read_packet())
            .await
            .is_err(),
        "inbound Play packet must stay paused until FinishConfiguration"
    );

    commands_tx
        .send(NetCommand::FinishConfiguration)
        .await
        .unwrap();
    let (finish_ack_id, finish_ack_payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("finish configuration ack")
        .unwrap();
    assert_eq!(
        finish_ack_id,
        ids::configuration::SERVERBOUND_FINISH_CONFIGURATION
    );
    assert!(finish_ack_payload.is_empty());

    let (pong_id, pong_payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("buffered play ping reply")
        .unwrap();
    assert_eq!(pong_id, ids::play::SERVERBOUND_PONG);
    assert_eq!(pong_payload, play_ping_id.to_be_bytes());
    let play_state_event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("play state change")
        .unwrap();
    assert!(matches!(
        play_state_event,
        NetEvent::StateChanged {
            state: ConnectionState::Play
        }
    ));

    // Disconnecting while inbound reads are paused must exit immediately without deadlocking.
    server
        .send_packet(ids::play::CLIENTBOUND_START_CONFIGURATION, &[])
        .await
        .unwrap();
    server.flush().await.unwrap();
    let second_start = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("second start configuration event")
        .unwrap();
    assert!(matches!(second_start, NetEvent::StartConfiguration));

    commands_tx.send(NetCommand::Disconnect).await.unwrap();
    timeout(Duration::from_secs(1), driver)
        .await
        .expect("disconnect while paused must not deadlock")
        .unwrap();
}

#[tokio::test]
async fn outbound_creative_slot_and_container_click_use_session_content_registry() {
    use bbb_protocol::content_registry::ModItemRegistration;

    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(64);
    let (commands_tx, commands_rx) = mpsc::channel(16);
    let mut stream =
        EventStreamContext::new(client, NetEventSender::new(events_tx, || {}), commands_rx);
    stream.state = ConnectionState::Configuration;
    let options = ConnectionOptions::offline("127.0.0.1:25565", "Player").unwrap();

    let mut builder = ContentRegistryBuilder::new();
    builder.register_item(ModItemRegistration {
        resource_id: "examplemod:crystal_ingot".to_string(),
        max_stack_size: 64,
        is_block_item: false,
    });
    let registry = builder.freeze().unwrap();
    let custom_item_id = registry.items()[0].id;
    assert!(custom_item_id >= 1506);

    let driver = tokio::spawn(async move {
        crate::event_stream::drive_event_stream(&mut stream, &options)
            .await
            .unwrap();
    });

    // Deliver synced bbb:content_registry and finish configuration.
    let mut custom_payload = Encoder::new();
    custom_payload.write_string(BBB_CONTENT_REGISTRY_CHANNEL);
    custom_payload.write_bytes(&registry.encode_wire());
    server
        .send_packet(
            ids::configuration::CLIENTBOUND_CUSTOM_PAYLOAD,
            &custom_payload.into_inner(),
        )
        .await
        .unwrap();
    server
        .send_packet(ids::configuration::CLIENTBOUND_FINISH_CONFIGURATION, &[])
        .await
        .unwrap();
    server.flush().await.unwrap();

    assert!(matches!(
        timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .unwrap(),
        Some(NetEvent::CustomPayload(_))
    ));
    assert!(matches!(
        timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .unwrap(),
        Some(NetEvent::FinishConfiguration)
    ));

    commands_tx
        .send(NetCommand::FinishConfiguration)
        .await
        .unwrap();
    let (finish_id, _) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        finish_id,
        ids::configuration::SERVERBOUND_FINISH_CONFIGURATION
    );
    assert!(matches!(
        timeout(Duration::from_secs(1), events_rx.recv())
            .await
            .unwrap(),
        Some(NetEvent::StateChanged {
            state: ConnectionState::Play
        })
    ));

    // Send both SetCreativeModeSlot and ContainerClick carrying the custom mod item id.
    commands_tx
        .send(NetCommand::SetCreativeModeSlot(
            packets::SetCreativeModeSlot {
                slot_num: 36,
                item: packets::ItemStackSummary {
                    item_id: Some(custom_item_id),
                    count: 8,
                    component_patch: packets::DataComponentPatchSummary::default().into(),
                },
            },
        ))
        .await
        .unwrap();
    let mut changed_slots = BTreeMap::new();
    changed_slots.insert(
        36,
        packets::HashedStack::Item(packets::HashedItemStack {
            item_id: custom_item_id,
            count: 8,
            components: packets::HashedComponentPatch::default(),
        }),
    );
    commands_tx
        .send(NetCommand::ContainerClick(packets::ContainerClick {
            container_id: 0,
            state_id: 1,
            slot_num: 36,
            button_num: 0,
            input: packets::ContainerInput::Pickup,
            changed_slots,
            carried_item: packets::HashedStack::Item(packets::HashedItemStack {
                item_id: custom_item_id,
                count: 1,
                components: packets::HashedComponentPatch::default(),
            }),
        }))
        .await
        .unwrap();
    commands_tx.send(NetCommand::Disconnect).await.unwrap();

    let (creative_id, creative_payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("creative slot packet with custom item")
        .unwrap();
    assert_eq!(creative_id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut creative_dec = Decoder::new(&creative_payload);
    assert_eq!(creative_dec.read_i16().unwrap(), 36);
    assert_eq!(creative_dec.read_var_i32().unwrap(), 8);
    assert_eq!(creative_dec.read_var_i32().unwrap(), custom_item_id);

    let (click_id, click_payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("container click packet with custom item")
        .unwrap();
    assert_eq!(click_id, ids::play::SERVERBOUND_CONTAINER_CLICK);
    let mut click_dec = Decoder::new(&click_payload);
    assert_eq!(click_dec.read_var_i32().unwrap(), 0);
    assert_eq!(click_dec.read_var_i32().unwrap(), 1);
    assert_eq!(click_dec.read_i16().unwrap(), 36);
    assert_eq!(click_dec.read_i8().unwrap(), 0);
    assert_eq!(click_dec.read_var_i32().unwrap(), 0);
    assert_eq!(click_dec.read_var_i32().unwrap(), 1);
    assert_eq!(click_dec.read_i16().unwrap(), 36);
    assert!(click_dec.read_bool().unwrap());
    assert_eq!(click_dec.read_var_i32().unwrap(), custom_item_id);
    assert_eq!(click_dec.read_var_i32().unwrap(), 8);

    driver.await.unwrap();
}
