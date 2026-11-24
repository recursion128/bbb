use super::*;

#[test]
fn the_event_stream_keeps_no_chat_session_or_cipher_state() {
    // Online mode is a declared non-goal: no profile-key runtime, no
    // session credentials, and no transport cipher may reappear on the
    // listener/transport boundary.
    let production = include_str!("../../event_stream.rs")
        .split("mod tests;")
        .next()
        .unwrap();
    assert!(!production.contains("session_credentials"));
    assert!(!production.contains("ChatSessionRuntime"));
    assert!(!production.contains("enable_encryption"));
    assert!(production.contains("read_packet_or_drive_connection"));
}

#[tokio::test]
async fn login_compression_query_and_finish_follow_vanilla_state_sequence() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(1);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let mut stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Login,
            player_position_state: PlayerPositionState::default(),
            player_movement_flags: crate::session::PlayerMovementFlags::default(),
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            pending_teleports: 0,
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: true,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation {
                language: "zh_cn".to_string(),
                view_distance: 12,
                chat_visibility: packets::ClientChatVisibility::System,
                chat_colors: false,
                displayed_skin_parts: 0x15,
                main_hand: packets::ClientMainHand::Left,
                text_filtering_enabled: true,
                allows_listing: true,
                particle_status: packets::ClientParticleStatus::Minimal,
            },
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };

    stream
        .handle_login_packet(LoginClientbound::CustomQuery { transaction_id: 42 })
        .await
        .unwrap();
    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("custom query answer should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::login::SERVERBOUND_CUSTOM_QUERY_ANSWER);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());

    stream
        .handle_login_packet(LoginClientbound::SetCompression { threshold: -1 })
        .await
        .unwrap();
    assert_eq!(stream.conn.compression_threshold, None);
    assert!(matches!(
        events_rx.recv().await,
        Some(NetEvent::CompressionSet { threshold: -1 })
    ));

    stream
        .handle_login_packet(LoginClientbound::SetCompression { threshold: 16 })
        .await
        .unwrap();
    assert_eq!(stream.conn.compression_threshold, Some(16));
    assert!(matches!(
        events_rx.recv().await,
        Some(NetEvent::CompressionSet { threshold: 16 })
    ));
    server.set_compression_threshold(16);

    stream
        .handle_login_packet(LoginClientbound::LoginFinished {
            profile: GameProfile {
                uuid: uuid::Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678),
                name: "bbb-client".to_string(),
                properties: Vec::new(),
            },
        })
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("login acknowledgement should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::login::SERVERBOUND_LOGIN_ACKNOWLEDGED);
    assert!(payload.is_empty());

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("brand custom payload should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::configuration::SERVERBOUND_CUSTOM_PAYLOAD);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:brand");
    assert_eq!(decoder.read_string(32767).unwrap(), "bbb-client");
    assert!(decoder.is_empty());

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("client information should be sent")
        .unwrap();
    assert_eq!(
        packet_id,
        ids::configuration::SERVERBOUND_CLIENT_INFORMATION
    );
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(16).unwrap(), "zh_cn");
    assert_eq!(decoder.read_i8().unwrap(), 12);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert!(!decoder.read_bool().unwrap());
    assert_eq!(decoder.read_u8().unwrap(), 0x15);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.read_bool().unwrap());
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    assert!(decoder.is_empty());

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

    let error = stream
        .handle_login_packet(LoginClientbound::Unknown {
            packet_id: 0x7d,
            len: 0,
        })
        .await
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("connection state is Configuration"));
}

#[tokio::test]
async fn configuration_control_responses_precede_finish_and_play_handoff() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(1);
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
            client_information: packets::ClientInformation {
                language: "ja_jp".to_string(),
                view_distance: 8,
                chat_visibility: packets::ClientChatVisibility::Hidden,
                chat_colors: true,
                displayed_skin_parts: 0x03,
                main_hand: packets::ClientMainHand::Right,
                text_filtering_enabled: false,
                allows_listing: true,
                particle_status: packets::ClientParticleStatus::Decreased,
            },
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };

    stream
        .handle_configuration_packet(packets::ConfigurationClientbound::KeepAlive {
            id: -123_456_789,
        })
        .await
        .unwrap();
    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("configuration keepalive should be answered on the connection")
        .unwrap();
    assert_eq!(packet_id, ids::configuration::SERVERBOUND_KEEP_ALIVE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i64().unwrap(), -123_456_789);
    assert!(decoder.is_empty());

    stream
        .handle_configuration_packet(packets::ConfigurationClientbound::Ping { id: 0x1234_5678 })
        .await
        .unwrap();
    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("configuration pong should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::configuration::SERVERBOUND_PONG);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i32().unwrap(), 0x1234_5678);
    assert!(decoder.is_empty());

    stream
        .handle_configuration_packet(packets::ConfigurationClientbound::Finish)
        .await
        .unwrap();

    assert!(timeout(Duration::from_millis(50), server.read_packet())
        .await
        .is_err());
    assert_eq!(stream.state, ConnectionState::Configuration);

    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("finish-configuration event should be emitted")
        .unwrap();
    assert!(matches!(event, NetEvent::FinishConfiguration));

    stream.finish_configuration().await.unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("finish configuration should be sent after canonical completion")
        .unwrap();
    assert_eq!(
        packet_id,
        ids::configuration::SERVERBOUND_FINISH_CONFIGURATION
    );
    assert!(payload.is_empty());

    assert!(timeout(Duration::from_millis(50), server.read_packet())
        .await
        .is_err());
    assert_eq!(stream.state, ConnectionState::Play);

    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("state-changed event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::StateChanged {
            state: ConnectionState::Play
        }
    ));
}

#[tokio::test]
async fn configuration_code_of_conduct_emits_event_without_immediate_accept() {
    let (client, mut server) = raw_connection_pair().await;
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

    stream
        .handle_configuration_packet(packets::ConfigurationClientbound::CodeOfConduct {
            text: "Follow the server rules.".to_string(),
        })
        .await
        .unwrap();

    assert!(timeout(Duration::from_millis(50), server.read_packet())
        .await
        .is_err());

    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("code-of-conduct event should be emitted")
        .unwrap();
    match event {
        NetEvent::CodeOfConduct { text } => {
            assert_eq!(text, "Follow the server rules.");
        }
        other => panic!("expected code-of-conduct event, got {other:?}"),
    }
}

#[tokio::test]
async fn configuration_code_of_conduct_auto_accepts_matching_hash() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(4);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let text = "Follow the server rules.";
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
            accepted_code_of_conduct_hash: Some(code_of_conduct_text_hash(text)),
            client_information: packets::ClientInformation::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            #[cfg(not(target_arch = "wasm32"))]
            play_tick: None,
        },
        events: NetEventSender::new(events_tx, || {}),
        commands: commands_rx,
    };

    stream
        .handle_configuration_packet(packets::ConfigurationClientbound::CodeOfConduct {
            text: text.to_string(),
        })
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("accept packet should be sent")
        .unwrap();
    assert_eq!(
        packet_id,
        ids::configuration::SERVERBOUND_ACCEPT_CODE_OF_CONDUCT
    );
    assert!(payload.is_empty());

    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("code-of-conduct event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::CodeOfConduct { text } if text == "Follow the server rules.")
    );
}

#[tokio::test]
async fn configuration_code_of_conduct_rejects_duplicate_packet() {
    let (client, mut server) = raw_connection_pair().await;
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

    stream
        .handle_configuration_packet(packets::ConfigurationClientbound::CodeOfConduct {
            text: "First rules.".to_string(),
        })
        .await
        .unwrap();
    timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("first code-of-conduct event should be emitted")
        .unwrap();

    let err = stream
        .handle_configuration_packet(packets::ConfigurationClientbound::CodeOfConduct {
            text: "Second rules.".to_string(),
        })
        .await
        .unwrap_err();

    assert!(
        err.to_string().contains("duplicate Code of Conduct"),
        "{err:?}"
    );
    assert!(matches!(
        events_rx.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    assert!(timeout(Duration::from_millis(50), server.read_packet())
        .await
        .is_err());
}

#[tokio::test]
async fn configuration_resource_pack_push_emits_push_and_response_events() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(2);
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
    let pack_id = uuid::Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678);

    stream
        .handle_configuration_packet(packets::ConfigurationClientbound::ResourcePackPush(
            packets::ResourcePackPush {
                id: pack_id,
                url: "https://example.invalid/config-pack.zip".to_string(),
                hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
                required: false,
                prompt: None,
            },
        ))
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("resource pack response should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::configuration::SERVERBOUND_RESOURCE_PACK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_uuid().unwrap(), pack_id);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        packets::ResourcePackResponseAction::Declined.ordinal()
    );
    assert!(decoder.is_empty());

    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("resource pack response event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::ResourcePackResponse {
            id,
            action: packets::ResourcePackResponseAction::Declined
        } if id == pack_id
    ));
    // Optional declined packs must not emit `ResourcePackPush` into world state.
    assert!(events_rx.try_recv().is_err());
}

#[tokio::test]
async fn configuration_resource_pack_push_with_invalid_url_sends_invalid_url_response() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(2);
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
    let pack_id = uuid::Uuid::from_u128(0x33333333_1234_5678_90ab_cdef12345678);

    stream
        .handle_configuration_packet(packets::ConfigurationClientbound::ResourcePackPush(
            packets::ResourcePackPush {
                id: pack_id,
                url: "ftp://example.invalid/config-pack.zip".to_string(),
                hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
                required: false,
                prompt: None,
            },
        ))
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("resource pack response should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::configuration::SERVERBOUND_RESOURCE_PACK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_uuid().unwrap(), pack_id);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        packets::ResourcePackResponseAction::InvalidUrl.ordinal()
    );
    assert!(decoder.is_empty());

    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("resource pack response event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::ResourcePackResponse {
            id,
            action: packets::ResourcePackResponseAction::InvalidUrl
        } if id == pack_id
    ));
    // Invalid-URL pushes must not forward `ResourcePackPush` into world state.
    assert!(events_rx.try_recv().is_err());
}

#[tokio::test]
async fn configuration_select_known_packs_sends_exact_local_intersection() {
    let (client, mut server) = raw_connection_pair().await;
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

    let core_pack = packets::KnownPack {
        namespace: "minecraft".to_string(),
        id: "core".to_string(),
        version: "26.1".to_string(),
    };
    stream
        .handle_configuration_packet_with_resource_pack_policy(
            packets::ConfigurationClientbound::SelectKnownPacks {
                known_packs: vec![core_pack.clone()],
            },
            crate::types::ServerResourcePackPolicy::Decline,
            &[core_pack],
        )
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("select-known-packs response should be sent")
        .unwrap();
    assert_eq!(
        packet_id,
        ids::configuration::SERVERBOUND_SELECT_KNOWN_PACKS
    );
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_string(32_767).unwrap(), "minecraft");
    assert_eq!(decoder.read_string(32_767).unwrap(), "core");
    assert_eq!(decoder.read_string(32_767).unwrap(), "26.1");
    assert!(decoder.is_empty());

    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("select-known-packs event should be emitted")
        .unwrap();
    match event {
        NetEvent::SelectKnownPacks {
            known_packs,
            selected_packs,
        } => {
            assert_eq!(known_packs.len(), 1);
            assert_eq!(known_packs[0].namespace, "minecraft");
            assert_eq!(known_packs[0].id, "core");
            assert_eq!(known_packs[0].version, "26.1");
            assert_eq!(selected_packs, known_packs);
        }
        other => panic!("expected select-known-packs event, got {other:?}"),
    }
}

#[tokio::test]
async fn configuration_select_known_packs_emits_event_and_sends_empty_response() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(1);
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

    stream
        .handle_configuration_packet(packets::ConfigurationClientbound::SelectKnownPacks {
            known_packs: vec![packets::KnownPack {
                namespace: "minecraft".to_string(),
                id: "core".to_string(),
                version: "26.1".to_string(),
            }],
        })
        .await
        .unwrap();

    let (packet_id, payload) = server.read_packet().await.unwrap();
    assert_eq!(
        packet_id,
        ids::configuration::SERVERBOUND_SELECT_KNOWN_PACKS
    );
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());
    assert!(matches!(
        events_rx.recv().await,
        Some(NetEvent::SelectKnownPacks {
            selected_packs,
            ..
        }) if selected_packs.is_empty()
    ));
}

#[tokio::test]
async fn configuration_transfer_emits_target_and_requests_reconnect() {
    let (client, _server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(1);
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
    let transfer = Transfer {
        host: "next.example.invalid".into(),
        port: 25566,
    };

    let reconnect = stream
        .handle_configuration_packet(packets::ConfigurationClientbound::Transfer(
            transfer.clone(),
        ))
        .await
        .unwrap();

    assert_eq!(reconnect, Some(transfer.clone()));
    assert!(matches!(
        events_rx.recv().await,
        Some(NetEvent::Transfer(update)) if update == transfer
    ));
}

#[tokio::test]
async fn configuration_unknown_packets_emit_unsupported_packet_events() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(1);
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

    stream
        .handle_configuration_packet(packets::ConfigurationClientbound::Unknown {
            packet_id: 0x7f,
            len: 12,
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
            state: ConnectionState::Configuration,
            packet_id: 0x7f,
            len: 12,
        }
    ));
}

#[tokio::test]
async fn login_unknown_packets_emit_unsupported_packet_events() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(1);
    let (_commands_tx, commands_rx) = mpsc::channel(1);
    let mut stream = EventStreamContext {
        session: ClientProtocolSession {
            conn: client,
            state: ConnectionState::Login,
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
        .handle_login_packet(LoginClientbound::Unknown {
            packet_id: 0x7d,
            len: 5,
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
            state: ConnectionState::Login,
            packet_id: 0x7d,
            len: 5,
        }
    ));
}

#[tokio::test]
async fn play_chunk_batch_feedback_uses_vanilla_calculator() {
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
        .handle_play_packet(PlayClientbound::ChunkBatchStart)
        .await
        .unwrap();
    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "chunk batch start must not send serverbound packets"
    );
    assert!(
        timeout(Duration::from_millis(50), events_rx.recv())
            .await
            .is_err(),
        "chunk batch start must not emit gameplay events"
    );

    stream
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
async fn play_keep_alive_and_ping_answer_on_the_connection() {
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
        .handle_play_packet(PlayClientbound::KeepAlive {
            id: 0x1122_3344_5566_7788,
        })
        .await
        .unwrap();
    stream
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

    assert!(
        events_rx.try_recv().is_err(),
        "connection-owned keepalive/ping must not defer events to the client loop"
    );
}

#[tokio::test]
async fn play_entity_and_respawn_packets_emit_matching_events() {
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

    let block_destruction = packets::BlockDestruction {
        id: 4,
        pos: packets::BlockPos {
            x: 12,
            y: 64,
            z: -5,
        },
        progress: 6,
    };
    stream
        .handle_play_packet(PlayClientbound::BlockDestruction(block_destruction))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("block destruction event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::BlockDestruction(update)) if update == block_destruction
    ));

    let entity_move = packets::EntityMove {
        id: 123,
        has_position: true,
        delta_x: 4096,
        delta_y: 0,
        delta_z: -2048,
        y_rot: Some(-90.0),
        x_rot: Some(45.0),
        on_ground: false,
    };
    stream
        .handle_play_packet(PlayClientbound::MoveEntity(entity_move))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("entity move event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::MoveEntity(update)) if update == entity_move
    ));

    let entity_event = packets::EntityEvent {
        entity_id: 123,
        event_id: 35,
    };
    stream
        .handle_play_packet(PlayClientbound::EntityEvent(entity_event))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("entity event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::EntityEvent(update)) if update == entity_event
    ));

    let respawn = packets::Respawn {
        common_spawn_info: packets::CommonPlayerSpawnInfo {
            dimension_type_id: 1,
            dimension: "minecraft:the_nether".to_string(),
            seed: 42,
            game_type: 1,
            previous_game_type: 0,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 20,
            sea_level: 32,
        },
        data_to_keep: 0,
    };
    stream
        .handle_play_packet(PlayClientbound::Respawn(respawn.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("respawn event should be emitted")
        .unwrap();
    assert!(matches!(event, NetEvent::Play(PlayClientbound::Respawn(update)) if update == respawn));

    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "entity and respawn dispatcher packets must not send serverbound responses"
    );
}

#[tokio::test]
async fn play_session_and_player_info_packets_emit_matching_events_without_responses() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(6);
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

    let login = packets::PlayLogin {
        player_id: 99,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: packets::CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 0,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: false,
    };
    stream
        .handle_play_packet(PlayClientbound::Login(login.clone()))
        .await
        .unwrap();
    assert!(matches!(
        events_rx.recv().await,
        Some(NetEvent::Play(PlayClientbound::Login(update))) if update == login
    ));

    let profile_id = uuid::Uuid::from_u128(1);
    let player_info = packets::PlayerInfoUpdate {
        actions: vec![
            packets::PlayerInfoAction::AddPlayer,
            packets::PlayerInfoAction::InitializeChat,
            packets::PlayerInfoAction::UpdateGameMode,
            packets::PlayerInfoAction::UpdateListed,
            packets::PlayerInfoAction::UpdateLatency,
            packets::PlayerInfoAction::UpdateDisplayName,
            packets::PlayerInfoAction::UpdateListOrder,
            packets::PlayerInfoAction::UpdateHat,
        ],
        entries: vec![packets::PlayerInfoEntry {
            profile_id,
            profile: Some(GameProfile {
                uuid: profile_id,
                name: "Ada".to_string(),
                properties: vec![packets::GameProfileProperty {
                    name: "textures".to_string(),
                    value: "skin".to_string(),
                    signature: Some("signature".to_string()),
                }],
            }),
            listed: true,
            latency: 42,
            game_mode: packets::GameType::Creative,
            display_name: Some(bbb_protocol::RichTextComponent::plain("Ada Lovelace")),
            show_hat: true,
            list_order: 3,
            chat_session: Some(packets::PlayerInfoChatSession {
                session_id: uuid::Uuid::from_u128(3),
                expires_at_epoch_millis: 99,
                public_key: vec![1, 2],
                key_signature: vec![3, 4],
            }),
        }],
    };
    assert_matching_event!(
        PlayClientbound::PlayerInfoUpdate(player_info.clone()),
        "player info update event should be emitted",
        NetEvent::Play(PlayClientbound::PlayerInfoUpdate(update)) if update == player_info
    );

    let player_info_remove = packets::PlayerInfoRemove {
        profile_ids: vec![profile_id],
    };
    assert_matching_event!(
        PlayClientbound::PlayerInfoRemove(player_info_remove.clone()),
        "player info remove event should be emitted",
        NetEvent::Play(PlayClientbound::PlayerInfoRemove(update)) if update == player_info_remove
    );

    let camera = packets::SetCamera { camera_id: 99 };
    assert_matching_event!(
        PlayClientbound::SetCamera(camera),
        "set camera event should be emitted",
        NetEvent::Play(PlayClientbound::SetCamera(update)) if update == camera
    );

    let difficulty = packets::ChangeDifficulty {
        difficulty: packets::Difficulty::Hard,
        locked: true,
    };
    assert_matching_event!(
        PlayClientbound::ChangeDifficulty(difficulty),
        "change difficulty event should be emitted",
        NetEvent::Play(PlayClientbound::ChangeDifficulty(update)) if update == difficulty
    );

    let tab_list = packets::TabList {
        header: bbb_protocol::RichTextComponent::plain("Welcome"),
        footer: bbb_protocol::RichTextComponent::plain(""),
    };
    assert_matching_event!(
        PlayClientbound::TabList(tab_list.clone()),
        "tab list event should be emitted",
        NetEvent::Play(PlayClientbound::TabList(update)) if update == tab_list
    );

    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "session and player-info packets must not send serverbound responses"
    );
}

#[tokio::test]
async fn play_entity_state_packets_emit_matching_events_without_responses() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(20);
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

    let add_entity = packets::AddEntity {
        id: 123,
        uuid: uuid::Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678),
        entity_type_id: VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        position: packets::Vec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        delta_movement: packets::Vec3d::default(),
        x_rot: -10.0,
        y_rot: 20.0,
        y_head_rot: 30.0,
        data: 99,
    };
    assert_matching_event!(
        PlayClientbound::AddEntity(add_entity.clone()),
        "add entity event should be emitted",
        NetEvent::Play(PlayClientbound::AddEntity(update)) if update == add_entity
    );

    let entity_animation = packets::EntityAnimation { id: 123, action: 3 };
    assert_matching_event!(
        PlayClientbound::EntityAnimation(entity_animation),
        "entity animation event should be emitted",
        NetEvent::Play(PlayClientbound::EntityAnimation(update)) if update == entity_animation
    );

    let hurt_animation = packets::HurtAnimation { id: 123, yaw: 45.5 };
    assert_matching_event!(
        PlayClientbound::HurtAnimation(hurt_animation),
        "hurt animation event should be emitted",
        NetEvent::Play(PlayClientbound::HurtAnimation(update)) if update == hurt_animation
    );

    let position_sync = packets::EntityPositionSync {
        id: 123,
        position: packets::Vec3d {
            x: 2.0,
            y: 65.0,
            z: -3.0,
        },
        delta_movement: packets::Vec3d {
            x: 0.0,
            y: 0.25,
            z: 0.0,
        },
        y_rot: 180.0,
        x_rot: 30.0,
        on_ground: true,
    };
    assert_matching_event!(
        PlayClientbound::EntityPositionSync(position_sync),
        "entity position sync event should be emitted",
        NetEvent::Play(PlayClientbound::EntityPositionSync(update)) if update == position_sync
    );

    let teleport = packets::TeleportEntity {
        id: 123,
        position: packets::Vec3d {
            x: 0.5,
            y: 70.0,
            z: -4.0,
        },
        delta_movement: packets::Vec3d {
            x: 0.0,
            y: 0.2,
            z: 0.0,
        },
        y_rot: 10.0,
        x_rot: -120.0,
        relatives_mask: 0,
        on_ground: true,
    };
    assert_matching_event!(
        PlayClientbound::TeleportEntity(teleport),
        "entity teleport event should be emitted",
        NetEvent::Play(PlayClientbound::TeleportEntity(update)) if update == teleport
    );

    let rotate_head = packets::RotateHead {
        id: 123,
        y_head_rot: 90.0,
    };
    assert_matching_event!(
        PlayClientbound::RotateHead(rotate_head),
        "entity head rotation event should be emitted",
        NetEvent::Play(PlayClientbound::RotateHead(update)) if update == rotate_head
    );

    let motion = packets::SetEntityMotion {
        id: 123,
        delta_movement: packets::Vec3d {
            x: 0.1,
            y: 0.0,
            z: -0.1,
        },
    };
    assert_matching_event!(
        PlayClientbound::SetEntityMotion(motion),
        "entity motion event should be emitted",
        NetEvent::Play(PlayClientbound::SetEntityMotion(update)) if update == motion
    );

    let link = packets::SetEntityLink {
        source_id: 123,
        dest_id: 456,
    };
    assert_matching_event!(
        PlayClientbound::SetEntityLink(link),
        "entity link event should be emitted",
        NetEvent::Play(PlayClientbound::SetEntityLink(update)) if update == link
    );

    let passengers = packets::SetPassengers {
        vehicle_id: 123,
        passenger_ids: vec![456],
    };
    assert_matching_event!(
        PlayClientbound::SetPassengers(passengers.clone()),
        "entity passengers event should be emitted",
        NetEvent::Play(PlayClientbound::SetPassengers(update)) if update == passengers
    );

    let equipment = packets::SetEquipment {
        entity_id: 123,
        slots: vec![packets::EquipmentSlotUpdate {
            slot: packets::EquipmentSlot::Head,
            item: test_item_stack(42, 1),
        }],
    };
    assert_matching_event!(
        PlayClientbound::SetEquipment(equipment.clone()),
        "entity equipment event should be emitted",
        NetEvent::Play(PlayClientbound::SetEquipment(update)) if update == equipment
    );

    let attributes = packets::UpdateAttributes {
        entity_id: 123,
        attributes: vec![packets::AttributeSnapshot {
            attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
            base: 20.0,
            modifiers: vec![packets::AttributeModifier {
                id: "minecraft:health_bonus".to_string(),
                amount: 4.0,
                operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            }],
        }],
    };
    assert_matching_event!(
        PlayClientbound::UpdateAttributes(attributes.clone()),
        "entity attributes event should be emitted",
        NetEvent::Play(PlayClientbound::UpdateAttributes(update)) if update == attributes
    );

    let entity_data = packets::SetEntityData {
        id: 123,
        values: vec![entity_invisible_data(), entity_air_supply_data(301)],
    };
    assert_matching_event!(
        PlayClientbound::SetEntityData(entity_data.clone()),
        "entity data event should be emitted",
        NetEvent::Play(PlayClientbound::SetEntityData(update)) if update == entity_data
    );

    let take_item = packets::TakeItemEntity {
        item_id: 300,
        player_id: 123,
        amount: 1,
    };
    assert_matching_event!(
        PlayClientbound::TakeItemEntity(take_item),
        "take item entity event should be emitted",
        NetEvent::Play(PlayClientbound::TakeItemEntity(update)) if update == take_item
    );

    let mob_effect = packets::UpdateMobEffect {
        entity_id: 123,
        effect_id: VANILLA_MOB_EFFECT_MINING_FATIGUE_ID,
        amplifier: 1,
        duration_ticks: 200,
        flags: packets::MobEffectFlags {
            raw: 0b0110,
            ambient: false,
            visible: true,
            show_icon: true,
            blend: false,
        },
    };
    assert_matching_event!(
        PlayClientbound::UpdateMobEffect(mob_effect),
        "mob effect update event should be emitted",
        NetEvent::Play(PlayClientbound::UpdateMobEffect(update)) if update == mob_effect
    );

    let damage = packets::DamageEvent {
        entity_id: 123,
        source_type_id: 5,
        source_cause_id: 456,
        source_direct_id: 300,
        source_position: Some(packets::Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        }),
    };
    assert_matching_event!(
        PlayClientbound::DamageEvent(damage),
        "damage event should be emitted",
        NetEvent::Play(PlayClientbound::DamageEvent(update)) if update == damage
    );

    let remove_effect = packets::RemoveMobEffect {
        entity_id: 123,
        effect_id: VANILLA_MOB_EFFECT_MINING_FATIGUE_ID,
    };
    assert_matching_event!(
        PlayClientbound::RemoveMobEffect(remove_effect),
        "remove mob effect event should be emitted",
        NetEvent::Play(PlayClientbound::RemoveMobEffect(update)) if update == remove_effect
    );

    let minecart = packets::MoveMinecartAlongTrack {
        entity_id: 300,
        lerp_steps: vec![packets::MinecartStep {
            position: packets::Vec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            movement: packets::Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.1,
            },
            y_rot: 30.0,
            x_rot: 0.0,
            weight: 1.0,
        }],
    };
    assert_matching_event!(
        PlayClientbound::MoveMinecartAlongTrack(minecart.clone()),
        "minecart along-track event should be emitted",
        NetEvent::Play(PlayClientbound::MoveMinecartAlongTrack(update)) if update == minecart
    );

    let vehicle = packets::MoveVehicle {
        position: packets::Vec3d {
            x: 5.0,
            y: 65.0,
            z: -6.0,
        },
        y_rot: 45.0,
        x_rot: 5.0,
    };
    assert_matching_event!(
        PlayClientbound::MoveVehicle(vehicle),
        "move vehicle event should be emitted",
        NetEvent::Play(PlayClientbound::MoveVehicle(update)) if update == vehicle
    );

    let explosion = packets::Explosion {
        center: packets::Vec3d {
            x: 3.0,
            y: 66.0,
            z: -4.0,
        },
        radius: 2.5,
        block_count: 3,
        player_knockback: Some(packets::Vec3d {
            x: 0.1,
            y: 0.2,
            z: 0.3,
        }),
        explosion_particle: packets::ParticlePayload {
            particle_type_id: bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
            raw_options: Vec::new(),
        },
        explosion_sound: packets::SoundEventHolder::Reference { registry_id: 41 },
        block_particles: vec![packets::WeightedExplosionParticle {
            particle: packets::ParticlePayload {
                particle_type_id: bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_CLOUD_ID,
                raw_options: Vec::new(),
            },
            scaling: 0.5,
            speed: 1.25,
            weight: 3,
        }],
    };
    assert_matching_event!(
        PlayClientbound::Explosion(explosion.clone()),
        "explosion event should be emitted",
        NetEvent::Play(PlayClientbound::Explosion(update)) if update == explosion
    );

    let remove_entities = packets::RemoveEntities {
        entity_ids: vec![456, 404],
    };
    assert_matching_event!(
        PlayClientbound::RemoveEntities(remove_entities.clone()),
        "remove entities event should be emitted",
        NetEvent::Play(PlayClientbound::RemoveEntities(update)) if update == remove_entities
    );

    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "entity state packets must not send serverbound responses"
    );
}

#[tokio::test]
async fn play_resource_pack_push_emits_push_and_response_events() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(2);
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
    let pack_id = uuid::Uuid::from_u128(0x87654321_4321_8765_90ab_cdef12345678);

    stream
        .handle_play_packet(PlayClientbound::ResourcePackPush(
            packets::ResourcePackPush {
                id: pack_id,
                url: "https://example.invalid/play-pack.zip".to_string(),
                hash: "abcdef0123456789abcdef0123456789abcdef01".to_string(),
                required: true,
                prompt: Some(bbb_protocol::RichTextComponent::plain("Install pack?")),
            },
        ))
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("resource pack response should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_RESOURCE_PACK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_uuid().unwrap(), pack_id);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        packets::ResourcePackResponseAction::Declined.ordinal()
    );
    assert!(decoder.is_empty());

    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("resource pack push event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::ResourcePackPush(update)) if update.id == pack_id
    ));
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("resource pack response event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::ResourcePackResponse {
            id,
            action: packets::ResourcePackResponseAction::Declined
        } if id == pack_id
    ));
}

#[tokio::test]
async fn play_resource_pack_push_with_invalid_url_sends_invalid_url_response() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(2);
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
    let pack_id = uuid::Uuid::from_u128(0x44444444_4321_8765_90ab_cdef12345678);

    stream
        .handle_play_packet(PlayClientbound::ResourcePackPush(
            packets::ResourcePackPush {
                id: pack_id,
                url: "not a valid resource pack url".to_string(),
                hash: "abcdef0123456789abcdef0123456789abcdef01".to_string(),
                required: true,
                prompt: Some(bbb_protocol::RichTextComponent::plain("Install pack?")),
            },
        ))
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("resource pack response should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_RESOURCE_PACK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_uuid().unwrap(), pack_id);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        packets::ResourcePackResponseAction::InvalidUrl.ordinal()
    );
    assert!(decoder.is_empty());

    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("resource pack response event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::ResourcePackResponse {
            id,
            action: packets::ResourcePackResponseAction::InvalidUrl
        } if id == pack_id
    ));
    // Invalid-URL pushes must not forward `PlayClientbound::ResourcePackPush` into world state.
    assert!(events_rx.try_recv().is_err());
}

#[tokio::test]
async fn play_server_presentation_packets_emit_matching_events_and_cookie_response() {
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

    stream
        .handle_play_packet(PlayClientbound::StoreCookie(packets::StoreCookie {
            key: "bbb:session".to_string(),
            payload: vec![4, 5, 6],
        }))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("store cookie event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::StoreCookie {
            key,
            payload_len: 3,
            stored_cookie_count: 1,
        } if key == "bbb:session"
    ));

    stream
        .handle_play_packet(PlayClientbound::CookieRequest(packets::CookieRequest {
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
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("cookie request event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::CookieRequest {
            key,
            response_payload_present: true,
        } if key == "bbb:session"
    ));

    stream
        .handle_play_packet(PlayClientbound::CookieRequest(packets::CookieRequest {
            key: "bbb:missing".to_string(),
        }))
        .await
        .unwrap();
    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("missing cookie response should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_COOKIE_RESPONSE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32767).unwrap(), "bbb:missing");
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("missing cookie request event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::CookieRequest {
            key,
            response_payload_present: false,
        } if key == "bbb:missing"
    ));

    let custom_payload = packets::CustomPayload {
        id: "minecraft:brand".to_string(),
        payload: packets::CustomPayloadBody::Brand {
            brand: "vanilla".to_string(),
        },
    };
    stream
        .handle_play_packet(PlayClientbound::CustomPayload(custom_payload.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("custom payload event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::CustomPayload(update)) if update == custom_payload)
    );

    let mut details = BTreeMap::new();
    details.insert("Server".to_string(), "play".to_string());
    let custom_report_details = packets::CustomReportDetails { details };
    stream
        .handle_play_packet(PlayClientbound::CustomReportDetails(
            custom_report_details.clone(),
        ))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("custom report details event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::CustomReportDetails(update)) if update == custom_report_details
    ));

    let server_links = packets::ServerLinks {
        links: vec![packets::ServerLinkEntry {
            link_type: packets::ServerLinkType::Known(packets::ServerLinkKnownType::Website),
            url: "https://example.invalid".to_string(),
        }],
    };
    stream
        .handle_play_packet(PlayClientbound::ServerLinks(server_links.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("server links event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::ServerLinks(update)) if update == server_links)
    );

    let server_data = packets::ServerData {
        motd: bbb_protocol::RichTextComponent::plain("Offline play server"),
        icon_bytes: Some(vec![0x89, b'P', b'N', b'G']),
    };
    stream
        .handle_play_packet(PlayClientbound::ServerData(server_data.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("server data event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::ServerData(update)) if update == server_data)
    );

    let resource_pack_pop = packets::ResourcePackPop {
        id: Some(uuid::Uuid::from_u128(
            0x11111111_2222_3333_4444_555555555555,
        )),
    };
    stream
        .handle_play_packet(PlayClientbound::ResourcePackPop(resource_pack_pop.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("resource pack pop event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::ResourcePackPop(update)) if update == resource_pack_pop
    ));

    let transfer = packets::Transfer {
        host: "next.example.invalid".into(),
        port: 25566,
    };
    let reconnect = stream
        .handle_play_packet(PlayClientbound::Transfer(transfer.clone()))
        .await
        .unwrap();
    assert_eq!(reconnect, Some(transfer.clone()));
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("transfer event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::Transfer(update)) if update == transfer)
    );

    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "server-presentation packets after cookie response must not send extra responses"
    );
}

#[tokio::test]
async fn play_hud_and_ui_packets_emit_matching_events_without_responses() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(13);
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

    let action_bar = packets::SetActionBarText::plain("Action ready");
    assert_matching_event!(
        PlayClientbound::SetActionBarText(action_bar.clone()),
        "action bar event should be emitted",
        NetEvent::Play(PlayClientbound::SetActionBarText(update)) if update == action_bar
    );

    let title = packets::SetTitleText::plain("Quest complete");
    assert_matching_event!(
        PlayClientbound::SetTitleText(title.clone()),
        "title text event should be emitted",
        NetEvent::Play(PlayClientbound::SetTitleText(update)) if update == title
    );

    let subtitle = packets::SetSubtitleText::plain("Return to camp");
    assert_matching_event!(
        PlayClientbound::SetSubtitleText(subtitle.clone()),
        "subtitle text event should be emitted",
        NetEvent::Play(PlayClientbound::SetSubtitleText(update)) if update == subtitle
    );

    let clear_titles = packets::ClearTitles { reset_times: true };
    assert_matching_event!(
        PlayClientbound::ClearTitles(clear_titles),
        "clear titles event should be emitted",
        NetEvent::Play(PlayClientbound::ClearTitles(update)) if update == clear_titles
    );

    let titles_animation = packets::SetTitlesAnimation {
        fade_in: 5,
        stay: 40,
        fade_out: 15,
    };
    assert_matching_event!(
        PlayClientbound::SetTitlesAnimation(titles_animation),
        "titles animation event should be emitted",
        NetEvent::Play(PlayClientbound::SetTitlesAnimation(update)) if update == titles_animation
    );

    let show_dialog = packets::ShowDialog {
        dialog: packets::DialogHolder::Direct {
            dialog: bbb_protocol::NbtSummaryValue::Compound(Vec::new()),
        },
    };
    assert_matching_event!(
        PlayClientbound::ShowDialog(show_dialog.clone()),
        "show dialog event should be emitted",
        NetEvent::Play(PlayClientbound::ShowDialog(update)) if update == show_dialog
    );

    assert_matching_event!(
        PlayClientbound::ClearDialog,
        "clear dialog event should be emitted",
        NetEvent::Play(PlayClientbound::ClearDialog)
    );

    assert_matching_event!(
        PlayClientbound::LowDiskSpaceWarning,
        "low disk warning event should be emitted",
        NetEvent::Play(PlayClientbound::LowDiskSpaceWarning)
    );

    let mount_screen = packets::MountScreenOpen {
        container_id: 11,
        inventory_columns: 5,
        entity_id: 42,
    };
    assert_matching_event!(
        PlayClientbound::MountScreenOpen(mount_screen),
        "mount screen open event should be emitted",
        NetEvent::Play(PlayClientbound::MountScreenOpen(update)) if update == mount_screen
    );

    let open_book = packets::OpenBook {
        hand: packets::InteractionHand::OffHand,
    };
    assert_matching_event!(
        PlayClientbound::OpenBook(open_book),
        "open book event should be emitted",
        NetEvent::Play(PlayClientbound::OpenBook(update)) if update == open_book
    );

    let open_sign = packets::OpenSignEditor {
        pos: packets::BlockPos {
            x: -5,
            y: 70,
            z: 12,
        },
        is_front_text: false,
    };
    assert_matching_event!(
        PlayClientbound::OpenSignEditor(open_sign),
        "open sign editor event should be emitted",
        NetEvent::Play(PlayClientbound::OpenSignEditor(update)) if update == open_sign
    );

    let item_display = |item_id| packets::SlotDisplaySummary {
        display_type_id: bbb_protocol::recipe_wire_types::VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
        raw_payload: vec![
            bbb_protocol::recipe_wire_types::VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID as u8,
            item_id as u8,
        ],
        item_stack: Some(packets::ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: packets::DataComponentPatchSummary::default().into(),
        }),
        tag: None,
    };
    let ghost_recipe = packets::PlaceGhostRecipe {
        container_id: 9,
        recipe_display: packets::RecipeDisplaySummary {
            display_type: packets::RecipeDisplayType::Stonecutter,
            raw_body: vec![3, 4, 100, 4, 101, 4, 102],
            crafting: None,
            furnace: None,
            stonecutter: Some(Box::new(packets::StonecutterRecipeDisplaySummary {
                input: item_display(100),
                result: item_display(101),
                crafting_station: item_display(102),
            })),
            smithing: None,
        },
    };
    assert_matching_event!(
        PlayClientbound::PlaceGhostRecipe(ghost_recipe.clone()),
        "place ghost recipe event should be emitted",
        NetEvent::Play(PlayClientbound::PlaceGhostRecipe(update)) if update == ghost_recipe
    );

    let pong = packets::PongResponse { time: 123456789 };
    assert_matching_event!(
        PlayClientbound::PongResponse(pong),
        "pong response event should be emitted",
        NetEvent::Play(PlayClientbound::PongResponse(update)) if update == pong
    );

    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "HUD and UI packets must not send serverbound responses"
    );
}

#[tokio::test]
async fn play_stats_cooldown_boss_and_combat_packets_emit_matching_events_without_responses() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(6);
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

    let stats = packets::AwardStats {
        stats: vec![
            packets::StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_CUSTOM_ID,
                value_id: 10,
                amount: 3,
            },
            packets::StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_BLOCK_MINED_ID,
                value_id: 4,
                amount: 11,
            },
        ],
    };
    assert_matching_event!(
        PlayClientbound::AwardStats(stats.clone()),
        "award stats event should be emitted",
        NetEvent::Play(PlayClientbound::AwardStats(update)) if update == stats
    );

    let cooldown = packets::Cooldown {
        cooldown_group: "minecraft:ender_pearl".to_string(),
        duration: 20,
    };
    assert_matching_event!(
        PlayClientbound::Cooldown(cooldown.clone()),
        "cooldown event should be emitted",
        NetEvent::Play(PlayClientbound::Cooldown(update)) if update == cooldown
    );

    let boss_event = packets::BossEvent {
        id: uuid::Uuid::from_u128(1),
        operation: packets::BossEventOperation::Add {
            name: Box::new(bbb_protocol::RichTextComponent::plain("Ender Dragon")),
            progress: 0.75,
            color: packets::BossBarColor::Purple,
            overlay: packets::BossBarOverlay::Progress,
            flags: packets::BossEventFlags {
                darken_screen: true,
                play_music: false,
                create_world_fog: true,
            },
        },
    };
    assert_matching_event!(
        PlayClientbound::BossEvent(boss_event.clone()),
        "boss event should be emitted",
        NetEvent::Play(PlayClientbound::BossEvent(update)) if update == boss_event
    );

    assert_matching_event!(
        PlayClientbound::PlayerCombatEnter,
        "player combat enter event should be emitted",
        NetEvent::Play(PlayClientbound::PlayerCombatEnter)
    );

    let combat_end = packets::PlayerCombatEnd { duration: 37 };
    assert_matching_event!(
        PlayClientbound::PlayerCombatEnd(combat_end),
        "player combat end event should be emitted",
        NetEvent::Play(PlayClientbound::PlayerCombatEnd(update)) if update == combat_end
    );

    let combat_kill = packets::PlayerCombatKill {
        player_id: 123,
        message: bbb_protocol::RichTextComponent::plain("You died"),
    };
    assert_matching_event!(
        PlayClientbound::PlayerCombatKill(combat_kill.clone()),
        "player combat kill event should be emitted",
        NetEvent::Play(PlayClientbound::PlayerCombatKill(update)) if update == combat_kill
    );

    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "stats, cooldown, boss, and combat packets must not send serverbound responses"
    );
}

#[tokio::test]
async fn play_map_particle_and_audio_packets_emit_matching_events_without_responses() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(5);
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

    let map = packets::MapItemData {
        map_id: 12,
        scale: 1,
        locked: false,
        decorations: Some(vec![packets::MapDecoration {
            type_id: 4,
            x: -8,
            y: 9,
            rot: 3,
            name: Some(bbb_protocol::RichTextComponent::plain("Camp")),
        }]),
        color_patch: Some(packets::MapColorPatch {
            start_x: 5,
            start_y: 6,
            width: 2,
            height: 1,
            colors: vec![11, 12],
        }),
    };
    assert_matching_event!(
        PlayClientbound::MapItemData(map.clone()),
        "map item data event should be emitted",
        NetEvent::Play(PlayClientbound::MapItemData(update)) if update == map
    );

    let particles = packets::LevelParticles {
        override_limiter: true,
        always_show: false,
        position: packets::Vec3d {
            x: 10.0,
            y: 64.5,
            z: -3.25,
        },
        offset: packets::Vec3d {
            x: 0.1,
            y: 0.2,
            z: 0.3,
        },
        max_speed: 0.4,
        count: 12,
        particle: packets::ParticlePayload {
            particle_type_id: VANILLA_PARTICLE_TYPE_DAMAGE_INDICATOR_ID,
            raw_options: vec![0x01, 0x02],
        },
    };
    assert_matching_event!(
        PlayClientbound::LevelParticles(particles.clone()),
        "level particles event should be emitted",
        NetEvent::Play(PlayClientbound::LevelParticles(update)) if update == particles
    );

    let sound = packets::SoundEvent {
        sound: packets::SoundEventHolder::Reference { registry_id: 41 },
        source: packets::SoundSource::Blocks,
        position: packets::Vec3d {
            x: 2.5,
            y: -1.0,
            z: 0.0,
        },
        volume: 0.75,
        pitch: 1.25,
        seed: 123456789,
    };
    assert_matching_event!(
        PlayClientbound::Sound(sound.clone()),
        "sound event should be emitted",
        NetEvent::Play(PlayClientbound::Sound(update)) if update == sound
    );

    let entity_sound = packets::SoundEntityEvent {
        sound: packets::SoundEventHolder::Direct {
            location: "minecraft:entity.cat.ambient".to_string(),
            fixed_range: Some(32.0),
        },
        source: packets::SoundSource::Neutral,
        entity_id: 123,
        volume: 1.0,
        pitch: 0.5,
        seed: -9,
    };
    assert_matching_event!(
        PlayClientbound::SoundEntity(entity_sound.clone()),
        "entity sound event should be emitted",
        NetEvent::Play(PlayClientbound::SoundEntity(update)) if update == entity_sound
    );

    let stop_sound = packets::StopSound {
        source: Some(packets::SoundSource::Music),
        name: Some("minecraft:music.menu".to_string()),
    };
    assert_matching_event!(
        PlayClientbound::StopSound(stop_sound.clone()),
        "stop sound event should be emitted",
        NetEvent::Play(PlayClientbound::StopSound(update)) if update == stop_sound
    );

    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "map, particle, and audio packets must not send serverbound responses"
    );
}

#[tokio::test]
async fn play_inventory_packets_emit_matching_events_without_responses() {
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

    let open_screen = packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    };
    stream
        .handle_play_packet(PlayClientbound::OpenScreen(open_screen.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("open screen event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::OpenScreen(update)) if update == open_screen)
    );

    let content = packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![packets::ItemStackSummary::empty(), test_item_stack(42, 3)],
        carried_item: test_item_stack(99, 1),
    };
    stream
        .handle_play_packet(PlayClientbound::ContainerSetContent(content.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("container content event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::ContainerSetContent(update)) if update == content
    ));

    let slot = packets::ContainerSetSlot {
        container_id: 7,
        state_id: 13,
        slot: 1,
        item: test_item_stack(43, 2),
    };
    stream
        .handle_play_packet(PlayClientbound::ContainerSetSlot(slot.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("container slot event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::ContainerSetSlot(update)) if update == slot)
    );

    let data = packets::ContainerSetData {
        container_id: 7,
        id: 2,
        value: 10,
    };
    stream
        .handle_play_packet(PlayClientbound::ContainerSetData(data))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("container data event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::ContainerSetData(update)) if update == data)
    );

    let player_slot = packets::SetPlayerInventory {
        slot: 36,
        item: test_item_stack(44, 1),
    };
    stream
        .handle_play_packet(PlayClientbound::SetPlayerInventory(player_slot.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("player inventory event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::SetPlayerInventory(update)) if update == player_slot
    ));

    let cursor = packets::SetCursorItem {
        item: test_item_stack(45, 1),
    };
    stream
        .handle_play_packet(PlayClientbound::SetCursorItem(cursor.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("cursor item event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::SetCursorItem(update)) if update == cursor)
    );

    let offers = packets::MerchantOffers {
        container_id: 7,
        offers: vec![packets::MerchantOffer {
            buy_a: packets::ItemCostSummary {
                item_id: 42,
                count: 3,
                component_predicate: packets::ItemCostComponentPredicateSummary::default(),
            },
            sell: test_item_stack(99, 1),
            buy_b: None,
            is_out_of_stock: false,
            uses: 1,
            max_uses: 8,
            xp: 5,
            special_price_diff: 0,
            price_multiplier: 0.05,
            demand: 2,
        }],
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    };
    stream
        .handle_play_packet(PlayClientbound::MerchantOffers(offers.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("merchant offers event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::MerchantOffers(update)) if update == offers)
    );

    let close = packets::ContainerClose { container_id: 7 };
    stream
        .handle_play_packet(PlayClientbound::ContainerClose(close))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("container close event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::ContainerClose(update)) if update == close)
    );

    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "inventory update packets must not send serverbound responses"
    );
}

#[tokio::test]
async fn play_command_and_chat_packets_emit_matching_events_without_responses() {
    let (client, mut server) = raw_connection_pair().await;
    let (events_tx, mut events_rx) = mpsc::channel(9);
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

    let commands = command_tree_packet("say");
    stream
        .handle_play_packet(PlayClientbound::Commands(commands.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("commands event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::Commands(update)) if update == commands)
    );

    let completions = packets::CustomChatCompletions {
        action: packets::CustomChatCompletionsAction::Set,
        entries: vec!["/spawn".to_string(), "/warp".to_string()],
    };
    stream
        .handle_play_packet(PlayClientbound::CustomChatCompletions(completions.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("custom chat completions event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::CustomChatCompletions(update)) if update == completions
    ));

    let suggestions = packets::CommandSuggestions {
        id: 77,
        start: 1,
        length: 4,
        suggestions: vec![
            packets::CommandSuggestion {
                text: "give".into(),
                tooltip: Some("Run give".to_string()),
                tooltip_styled: None,
                tooltip_component: None,
            },
            packets::CommandSuggestion {
                text: "gamemode".into(),
                tooltip: None,
                tooltip_styled: None,
                tooltip_component: None,
            },
        ],
    };
    stream
        .handle_play_packet(PlayClientbound::CommandSuggestions(suggestions.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("command suggestions event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::CommandSuggestions(update)) if update == suggestions
    ));

    let tag_query = packets::TagQuery {
        transaction_id: 12,
        tag_present: true,
        raw_nbt: vec![10, 0],
    };
    stream
        .handle_play_packet(PlayClientbound::TagQuery(tag_query.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("tag query event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::TagQuery(update)) if update == tag_query)
    );

    let player_chat = player_chat_packet(3);
    stream
        .handle_play_packet(PlayClientbound::PlayerChat(player_chat.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("player chat event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::PlayerChat(update)) if update == player_chat)
    );

    let delete_chat = packets::DeleteChat {
        message_signature: packets::PackedMessageSignature {
            cache_id: Some(2),
            full_signature: None,
        },
    };
    stream
        .handle_play_packet(PlayClientbound::DeleteChat(delete_chat.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("delete chat event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::DeleteChat(update)) if update == delete_chat)
    );

    let disguised_chat = packets::DisguisedChat {
        message: "Server says hi".to_string(),
        message_styled: Vec::new(),
        message_component: None,
        chat_type: chat_type_bound("Server"),
    };
    stream
        .handle_play_packet(PlayClientbound::DisguisedChat(disguised_chat.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("disguised chat event should be emitted")
        .unwrap();
    assert!(matches!(
        event,
        NetEvent::Play(PlayClientbound::DisguisedChat(update)) if update == disguised_chat
    ));

    let system_chat = packets::SystemChat::plain("Welcome", false);
    stream
        .handle_play_packet(PlayClientbound::SystemChat(system_chat.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("system chat event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::SystemChat(update)) if update == system_chat)
    );

    let update_tags = packets::UpdateTags {
        registries: vec![packets::RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![packets::TagNetworkPayload {
                tag: "minecraft:logs".to_string(),
                entries: vec![5, 6, 7],
            }],
        }],
    };
    stream
        .handle_play_packet(PlayClientbound::UpdateTags(update_tags.clone()))
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(1), events_rx.recv())
        .await
        .expect("update tags event should be emitted")
        .unwrap();
    assert!(
        matches!(event, NetEvent::Play(PlayClientbound::UpdateTags(update)) if update == update_tags)
    );

    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "command and chat client state packets must not send serverbound responses"
    );
}
