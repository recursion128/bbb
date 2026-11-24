use super::*;

#[test]
fn app_quit_queues_shutdown_for_the_simulation_owner() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "app.quit".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["running"], false);
    assert!(state.snapshot.load().app.running);
    assert!(state.requests.lock().unwrap().shutdown_requested);
}

#[test]
fn write_dispatch_reports_a_closed_command_owner() {
    let state = ControlState::new(
        "test",
        SharedControlRequests::new(Arc::new(ClosedControlRequestSink)),
    );
    let response = dispatch(
        ControlRequest {
            method: "net.request_stats".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(!response.ok);
    assert!(response.result.is_none());
    assert_eq!(
        response.error.as_deref(),
        Some("control command rejected: client command queue is closed")
    );
}

#[test]
fn app_status_reads_world_counters_from_world_store() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_block_changed_ack(BlockChangedAck { sequence: 17 });
        state.snapshot.rcu(|current| {
            let mut next = (**current).clone();
            next.audio = AudioCounters {
                enabled: true,
                catalog_events: 123,
                commands_submitted: 4,
                ..AudioCounters::default()
            };
            next
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "app.status".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let status = response.result.unwrap();
    assert_eq!(status["app"]["version"], "test");
    assert_eq!(status["audio"]["enabled"], true);
    assert_eq!(status["audio"]["catalog_events"], 123);
    assert_eq!(status["audio"]["commands_submitted"], 4);
    assert_eq!(status["world"]["block_changed_ack_packets"], 1);
    assert_eq!(status["world"]["block_destructions_tracked"], 0);
}

#[test]
fn high_frequency_queries_match_the_published_world_generation() {
    let state = test_control_state();
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.insert_decoded_chunk(single_section_chunk());
    world.apply_block_changed_ack(BlockChangedAck { sequence: 23 });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 17.25,
            y: 1.0,
            z: -30.75,
        },
        y_rot: 35.0,
        x_rot: -12.0,
        ..LocalPlayerPoseState::default()
    });
    let expected_counters = serde_json::to_value(world.counters()).unwrap();
    let expected_local = serde_json::to_value(world.client_local_player()).unwrap();
    let expected_block = serde_json::to_value(
        world
            .probe_block(bbb_client_world::BlockPos {
                x: 17,
                y: 0,
                z: -31,
            })
            .unwrap(),
    )
    .unwrap();
    publish_test_world(&state, world);

    let counters = dispatch(
        ControlRequest {
            method: "world.counters".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );
    let local = dispatch(
        ControlRequest {
            method: "world.client_local_player".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );
    let block = dispatch(
        ControlRequest {
            method: "world.probe_block".to_string(),
            params: json!({"x": 17, "y": 0, "z": -31}),
        },
        &state,
    );

    assert_eq!(counters.result.unwrap(), expected_counters);
    assert_eq!(local.result.unwrap(), expected_local);
    assert_eq!(block.result.unwrap(), expected_block);
}

#[test]
fn audio_counters_reads_runtime_projection() {
    let state = test_control_state();
    state.snapshot.rcu(|current| {
        let mut next = (**current).clone();
        next.audio = AudioCounters {
            enabled: false,
            disabled_reason: Some("initialize Kira audio runtime".to_string()),
            active_music_event_id: Some("minecraft:music.dragon".to_string()),
            sound_static_channels_used: 3,
            sound_static_channels_capacity: 248,
            sound_streaming_channels_used: 1,
            sound_streaming_channels_capacity: 8,
            sound_channel_admission_rejections: 2,
            sound_channel_capacity_origin: Some("kira-logical-main-plus-spatial".to_string()),
            sound_mood_percent: 42,
            resolve_failures: 2,
            submit_failures: 1,
            last_resolve_error: Some("missing sound event minecraft:missing".to_string()),
            last_submit_error: Some("failed to submit audio command".to_string()),
            ..AudioCounters::default()
        };
        next
    });

    let response = dispatch(
        ControlRequest {
            method: "audio.counters".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let audio = response.result.unwrap();
    assert_eq!(audio["enabled"], false);
    assert_eq!(audio["disabled_reason"], "initialize Kira audio runtime");
    assert_eq!(audio["active_music_event_id"], "minecraft:music.dragon");
    assert_eq!(audio["sound_static_channels_used"], 3);
    assert_eq!(audio["sound_static_channels_capacity"], 248);
    assert_eq!(audio["sound_streaming_channels_used"], 1);
    assert_eq!(audio["sound_streaming_channels_capacity"], 8);
    assert_eq!(audio["sound_channel_admission_rejections"], 2);
    assert_eq!(
        audio["sound_channel_capacity_origin"],
        "kira-logical-main-plus-spatial"
    );
    assert_eq!(audio["sound_mood_percent"], 42);
    assert_eq!(audio["resolve_failures"], 2);
    assert_eq!(audio["submit_failures"], 1);
    assert_eq!(
        audio["last_resolve_error"],
        "missing sound event minecraft:missing"
    );
    assert_eq!(audio["last_submit_error"], "failed to submit audio command");
}

#[test]
fn registries_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.record_registry_data(RegistryData {
            registry: "minecraft:chat_type".to_string(),
            raw_payload_len: 128,
            entries: vec![
                RegistryDataEntry {
                    id: "minecraft:chat".to_string(),
                    raw_data: Some(vec![1, 2, 3]),
                },
                RegistryDataEntry {
                    id: "minecraft:raw".to_string(),
                    raw_data: None,
                },
            ],
        });
        store.record_registry_data(RegistryData {
            registry: "minecraft:chat_type".to_string(),
            raw_payload_len: 96,
            entries: vec![RegistryDataEntry {
                id: "minecraft:chat".to_string(),
                raw_data: Some(vec![4, 5]),
            }],
        });
        store.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:logs".to_string(),
                    entries: vec![5, 6, 7],
                }],
            }],
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.registries".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let registries = response.result.unwrap();
    assert_eq!(registries["registries"][0]["name"], "minecraft:chat_type");
    assert_eq!(registries["registries"][0]["raw_payload_len"], 128);
    assert_eq!(
        registries["registries"][0]["entries"][0]["id"],
        "minecraft:chat"
    );
    assert_eq!(registries["registries"][0]["entries"][0]["has_data"], true);
    assert_eq!(registries["registries"][0]["entries"][0]["raw_data_len"], 3);
    assert!(registries["registries"][0]["entries"][0]
        .get("raw_data")
        .is_none());
    assert_eq!(registries["registries"][1]["raw_payload_len"], 96);
    assert_eq!(
        registries["contents"]["minecraft:chat_type"]["packet_count"],
        2
    );
    assert_eq!(
        registries["contents"]["minecraft:chat_type"]["duplicate_entry_ids"]["minecraft:chat"],
        1
    );
    assert_eq!(
        registries["tags"]["minecraft:item"]["tags"]["minecraft:logs"][2],
        7
    );
    assert!(registries.get("block_states").is_none());
}

#[test]
fn level_state_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_login(&ProtocolPlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec![
                "minecraft:overworld".to_string(),
                "minecraft:the_nether".to_string(),
                "minecraft:the_end".to_string(),
            ],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: ProtocolSpawnInfo {
                dimension_type_id: 1,
                dimension: "minecraft:the_nether".to_string(),
                seed: 12345,
                game_type: 1,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 32,
            },
            enforces_secure_chat: true,
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.level_state".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let level_state = response.result.unwrap();
    assert_eq!(level_state["dimension"]["min_y"], 0);
    assert_eq!(level_state["dimension"]["height"], 256);
    assert_eq!(level_state["level"]["dimension"], "minecraft:the_nether");
    assert_eq!(level_state["level"]["dimension_type_id"], 1);
    assert_eq!(
        level_state["level"]["dimension_type_name"],
        "minecraft:the_nether"
    );
    assert_eq!(level_state["level"]["sea_level"], 32);
    assert_eq!(level_state["level"]["is_debug"], false);
    assert_eq!(level_state["level"]["is_flat"], false);
    assert_eq!(level_state["gameplay"]["game_type"], 1);
    assert_eq!(level_state["gameplay"]["game_type_name"], "creative");
    assert_eq!(
        level_state["gameplay"]["previous_game_type"],
        serde_json::Value::Null
    );
    assert_eq!(level_state["gameplay"]["show_death_screen"], true);
    assert_eq!(level_state["gameplay"]["do_limited_crafting"], false);

    let mut cleared_world = (*state.snapshot.load().world_view.world).clone();
    cleared_world.clear_client_level();
    publish_test_world(&state, cleared_world);
    let response = dispatch(
        ControlRequest {
            method: "world.level_state".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let cleared = response.result.unwrap();
    assert_eq!(cleared["dimension"]["min_y"], -64);
    assert_eq!(cleared["dimension"]["height"], 384);
    assert!(cleared["level"].is_null());
}

#[test]
fn renderer_screenshot_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "renderer.screenshot".to_string(),
            params: json!({"path": "target/control-shot.png"}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["queued"], true);
    assert_eq!(
        state.requests.lock().unwrap().screenshot_request.as_deref(),
        Some("target/control-shot.png")
    );

    let missing_path = dispatch(
        ControlRequest {
            method: "renderer.screenshot".to_string(),
            params: json!({}),
        },
        &state,
    );
    assert!(!missing_path.ok);
}

#[test]
fn net_accept_code_of_conduct_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.accept_code_of_conduct".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["queued"], true);
    assert_eq!(
        state.requests.lock().unwrap().code_of_conduct_requests,
        vec![CodeOfConductControlRequest::Accept { remember: false }]
    );
}

#[test]
fn net_accept_code_of_conduct_can_queue_persistent_accept() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.accept_code_of_conduct".to_string(),
            params: json!({"remember": true}),
        },
        &state,
    );

    assert!(response.ok);
    let result = response.result.unwrap();
    assert_eq!(result["queued"], true);
    assert_eq!(result["remember"], true);
    assert_eq!(
        state.requests.lock().unwrap().code_of_conduct_requests,
        vec![CodeOfConductControlRequest::Accept { remember: true }]
    );
}

#[test]
fn net_decline_code_of_conduct_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.decline_code_of_conduct".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["queued"], true);
    assert_eq!(
        state.requests.lock().unwrap().code_of_conduct_requests,
        vec![CodeOfConductControlRequest::Decline]
    );
}

#[test]
fn net_clear_code_of_conduct_acceptance_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.clear_code_of_conduct_acceptance".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(
        state.requests.lock().unwrap().code_of_conduct_requests,
        vec![CodeOfConductControlRequest::ClearAcceptance]
    );
}

#[test]
fn net_send_chat_command_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.send_chat_command".to_string(),
            params: json!({"command": "give @p minecraft:stone"}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::ChatCommand {
            command: "give @p minecraft:stone".to_string()
        }]
    );

    let missing_command = dispatch(
        ControlRequest {
            method: "net.send_chat_command".to_string(),
            params: json!({}),
        },
        &state,
    );
    assert!(!missing_command.ok);
}

#[test]
fn net_request_command_suggestions_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.request_command_suggestions".to_string(),
            params: json!({"id": 18, "command": "/give @p minecraft:stone"}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::CommandSuggestionRequest {
            id: 18,
            command: "/give @p minecraft:stone".to_string()
        }]
    );

    let missing_id = dispatch(
        ControlRequest {
            method: "net.request_command_suggestions".to_string(),
            params: json!({"command": "/help"}),
        },
        &state,
    );
    assert!(!missing_id.ok);
}

#[test]
fn net_tag_query_block_entity_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.query_block_entity_tag".to_string(),
            params: json!({"transaction_id": 42, "x": -5, "y": 70, "z": 12}),
        },
        &state,
    );

    assert!(response.ok);
    let result = response.result.unwrap();
    assert_eq!(result["queued"], true);
    assert_eq!(result["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::QueryBlockEntityTag {
            transaction_id: 42,
            x: -5,
            y: 70,
            z: 12,
        }]
    );
}

#[test]
fn net_tag_query_block_entity_rejects_missing_or_non_integer_params() {
    let state = test_control_state();
    let missing_transaction = dispatch(
        ControlRequest {
            method: "net.query_block_entity_tag".to_string(),
            params: json!({"x": -5, "y": 70, "z": 12}),
        },
        &state,
    );
    let non_integer_y = dispatch(
        ControlRequest {
            method: "net.query_block_entity_tag".to_string(),
            params: json!({"transaction_id": 42, "x": -5, "y": "70", "z": 12}),
        },
        &state,
    );

    assert!(!missing_transaction.ok);
    assert_eq!(
        missing_transaction.error.as_deref(),
        Some("net.query_block_entity_tag requires integer param transaction_id")
    );
    assert!(!non_integer_y.ok);
    assert_eq!(
        non_integer_y.error.as_deref(),
        Some("net.query_block_entity_tag requires integer param y")
    );
    assert!(state.requests.lock().unwrap().net_requests.is_empty());
}

#[test]
fn net_tag_query_entity_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.query_entity_tag".to_string(),
            params: json!({"transaction_id": 43, "entity_id": 99}),
        },
        &state,
    );

    assert!(response.ok);
    let result = response.result.unwrap();
    assert_eq!(result["queued"], true);
    assert_eq!(result["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::QueryEntityTag {
            transaction_id: 43,
            entity_id: 99,
        }]
    );
}

#[test]
fn net_tag_query_entity_rejects_missing_or_non_integer_params() {
    let state = test_control_state();
    let missing_transaction = dispatch(
        ControlRequest {
            method: "net.query_entity_tag".to_string(),
            params: json!({"entity_id": 99}),
        },
        &state,
    );
    let non_integer_entity = dispatch(
        ControlRequest {
            method: "net.query_entity_tag".to_string(),
            params: json!({"transaction_id": 43, "entity_id": "99"}),
        },
        &state,
    );

    assert!(!missing_transaction.ok);
    assert_eq!(
        missing_transaction.error.as_deref(),
        Some("net.query_entity_tag requires integer param transaction_id")
    );
    assert!(!non_integer_entity.ok);
    assert_eq!(
        non_integer_entity.error.as_deref(),
        Some("net.query_entity_tag requires integer param entity_id")
    );
    assert!(state.requests.lock().unwrap().net_requests.is_empty());
}

#[test]
fn net_tag_query_counters_deserialize_with_defaults() {
    let mut value = serde_json::to_value(NetCounters::default()).unwrap();
    let object = value.as_object_mut().unwrap();
    object.remove("block_entity_tag_query_commands_queued");
    object.remove("entity_tag_query_commands_queued");

    let counters: NetCounters = serde_json::from_value(value).unwrap();

    assert_eq!(counters.block_entity_tag_query_commands_queued, 0);
    assert_eq!(counters.entity_tag_query_commands_queued, 0);
}

#[test]
fn net_spectate_entity_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.spectate_entity".to_string(),
            params: json!({"entity_id": 99}),
        },
        &state,
    );

    assert!(response.ok);
    let result = response.result.unwrap();
    assert_eq!(result["queued"], true);
    assert_eq!(result["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::SpectateEntity { entity_id: 99 }]
    );
}

#[test]
fn net_spectate_entity_rejects_missing_or_non_integer_entity_id() {
    let state = test_control_state();
    let missing_entity_id = dispatch(
        ControlRequest {
            method: "net.spectate_entity".to_string(),
            params: json!({}),
        },
        &state,
    );
    let non_integer_entity_id = dispatch(
        ControlRequest {
            method: "net.spectate_entity".to_string(),
            params: json!({"entity_id": "99"}),
        },
        &state,
    );

    assert!(!missing_entity_id.ok);
    assert_eq!(
        missing_entity_id.error.as_deref(),
        Some("net.spectate_entity requires integer param entity_id")
    );
    assert!(!non_integer_entity_id.ok);
    assert_eq!(
        non_integer_entity_id.error.as_deref(),
        Some("net.spectate_entity requires integer param entity_id")
    );
    assert!(state.requests.lock().unwrap().net_requests.is_empty());
}

#[test]
fn net_teleport_to_entity_queues_request() {
    let state = test_control_state();
    let uuid = Uuid::from_u128(0x00112233445566778899aabbccddeeff);
    let response = dispatch(
        ControlRequest {
            method: "net.teleport_to_entity".to_string(),
            params: json!({"uuid": uuid.to_string()}),
        },
        &state,
    );

    assert!(response.ok);
    let result = response.result.unwrap();
    assert_eq!(result["queued"], true);
    assert_eq!(result["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::TeleportToEntity { uuid }]
    );
}

#[test]
fn net_teleport_to_entity_rejects_missing_non_string_or_invalid_uuid() {
    let state = test_control_state();
    let missing_uuid = dispatch(
        ControlRequest {
            method: "net.teleport_to_entity".to_string(),
            params: json!({}),
        },
        &state,
    );
    let non_string_uuid = dispatch(
        ControlRequest {
            method: "net.teleport_to_entity".to_string(),
            params: json!({"uuid": 42}),
        },
        &state,
    );
    let invalid_uuid = dispatch(
        ControlRequest {
            method: "net.teleport_to_entity".to_string(),
            params: json!({"uuid": "not-a-uuid"}),
        },
        &state,
    );

    assert!(!missing_uuid.ok);
    assert_eq!(
        missing_uuid.error.as_deref(),
        Some("net.teleport_to_entity requires string param uuid")
    );
    assert!(!non_string_uuid.ok);
    assert_eq!(
        non_string_uuid.error.as_deref(),
        Some("net.teleport_to_entity requires string param uuid")
    );
    assert!(!invalid_uuid.ok);
    assert!(invalid_uuid.error.as_deref().is_some_and(
        |err| err.starts_with("net.teleport_to_entity requires valid UUID param uuid:")
    ));
    assert!(state.requests.lock().unwrap().net_requests.is_empty());
}

#[test]
fn net_spectate_and_teleport_to_entity_counters_deserialize_with_defaults() {
    let mut value = serde_json::to_value(NetCounters::default()).unwrap();
    let object = value.as_object_mut().unwrap();
    object.remove("spectate_entity_commands_queued");
    object.remove("teleport_to_entity_commands_queued");

    let counters: NetCounters = serde_json::from_value(value).unwrap();

    assert_eq!(counters.spectate_entity_commands_queued, 0);
    assert_eq!(counters.teleport_to_entity_commands_queued, 0);
}

#[test]
fn net_change_difficulty_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.change_difficulty".to_string(),
            params: json!({"difficulty": "hard"}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::change_difficulty_named("hard").unwrap()]
    );
}

#[test]
fn net_change_difficulty_rejects_missing_or_invalid_difficulty() {
    let state = test_control_state();
    let missing_difficulty = dispatch(
        ControlRequest {
            method: "net.change_difficulty".to_string(),
            params: json!({}),
        },
        &state,
    );
    let invalid_difficulty = dispatch(
        ControlRequest {
            method: "net.change_difficulty".to_string(),
            params: json!({"difficulty": "nightmare"}),
        },
        &state,
    );
    let non_string_difficulty = dispatch(
        ControlRequest {
            method: "net.change_difficulty".to_string(),
            params: json!({"difficulty": 3}),
        },
        &state,
    );

    assert!(!missing_difficulty.ok);
    assert_eq!(
        missing_difficulty.error.as_deref(),
        Some(
            "net.change_difficulty requires string param difficulty: peaceful, easy, normal, or hard"
        )
    );
    assert!(!invalid_difficulty.ok);
    assert_eq!(
        invalid_difficulty.error.as_deref(),
        Some("net.change_difficulty requires difficulty peaceful, easy, normal, or hard")
    );
    assert!(!non_string_difficulty.ok);
    assert!(state.requests.lock().unwrap().net_requests.is_empty());
}

#[test]
fn net_change_game_mode_queues_request() {
    let state = test_control_state();
    let game_modes = ["survival", "creative", "adventure", "spectator"];

    for (index, game_mode) in game_modes.iter().enumerate() {
        let response = dispatch(
            ControlRequest {
                method: "net.change_game_mode".to_string(),
                params: json!({"game_mode": game_mode}),
            },
            &state,
        );

        assert!(response.ok);
        assert_eq!(response.result.unwrap()["pending"], index + 1);
    }

    let expected_requests = game_modes
        .iter()
        .map(|game_mode| NetControlRequest::change_game_mode_named(game_mode).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        expected_requests
    );
}

#[test]
fn net_change_game_mode_rejects_missing_non_string_or_invalid_game_mode() {
    let state = test_control_state();
    let missing_game_mode = dispatch(
        ControlRequest {
            method: "net.change_game_mode".to_string(),
            params: json!({}),
        },
        &state,
    );
    let non_string_game_mode = dispatch(
        ControlRequest {
            method: "net.change_game_mode".to_string(),
            params: json!({"game_mode": 1}),
        },
        &state,
    );
    let invalid_game_mode = dispatch(
        ControlRequest {
            method: "net.change_game_mode".to_string(),
            params: json!({"game_mode": "builder"}),
        },
        &state,
    );

    assert!(!missing_game_mode.ok);
    assert_eq!(
        missing_game_mode.error.as_deref(),
        Some("net.change_game_mode requires string param game_mode")
    );
    assert!(!non_string_game_mode.ok);
    assert_eq!(
        non_string_game_mode.error.as_deref(),
        Some("net.change_game_mode param game_mode must be a string")
    );
    assert!(!invalid_game_mode.ok);
    assert_eq!(
        invalid_game_mode.error.as_deref(),
        Some("net.change_game_mode requires game_mode survival, creative, adventure, or spectator")
    );
    assert!(state.requests.lock().unwrap().net_requests.is_empty());
}

#[test]
fn net_change_game_mode_counters_deserialize_with_defaults() {
    let mut value = serde_json::to_value(NetCounters::default()).unwrap();
    let object = value.as_object_mut().unwrap();
    object.remove("change_game_mode_commands_queued");

    let counters: NetCounters = serde_json::from_value(value).unwrap();

    assert_eq!(counters.change_game_mode_commands_queued, 0);
}

#[test]
fn net_lock_difficulty_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.lock_difficulty".to_string(),
            params: json!({"locked": false}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::LockDifficulty { locked: false }]
    );
}

#[test]
fn net_lock_difficulty_rejects_missing_or_non_bool_locked() {
    let state = test_control_state();
    let missing_locked = dispatch(
        ControlRequest {
            method: "net.lock_difficulty".to_string(),
            params: json!({}),
        },
        &state,
    );
    let non_bool_locked = dispatch(
        ControlRequest {
            method: "net.lock_difficulty".to_string(),
            params: json!({"locked": "true"}),
        },
        &state,
    );

    assert!(!missing_locked.ok);
    assert_eq!(
        missing_locked.error.as_deref(),
        Some("net.lock_difficulty requires boolean param locked")
    );
    assert!(!non_bool_locked.ok);
    assert_eq!(
        non_bool_locked.error.as_deref(),
        Some("net.lock_difficulty requires boolean param locked")
    );
    assert!(state.requests.lock().unwrap().net_requests.is_empty());
}

#[test]
fn net_difficulty_counters_deserialize_with_defaults() {
    let mut value = serde_json::to_value(NetCounters::default()).unwrap();
    let object = value.as_object_mut().unwrap();
    object.remove("change_difficulty_commands_queued");
    object.remove("lock_difficulty_commands_queued");
    object.remove("set_game_rule_commands_queued");

    let counters: NetCounters = serde_json::from_value(value).unwrap();

    assert_eq!(counters.change_difficulty_commands_queued, 0);
    assert_eq!(counters.lock_difficulty_commands_queued, 0);
    assert_eq!(counters.set_game_rule_commands_queued, 0);
}

#[test]
fn net_set_held_slot_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.set_held_slot".to_string(),
            params: json!({"slot": 4}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::SetHeldSlot { slot: 4 }]
    );

    let missing_slot = dispatch(
        ControlRequest {
            method: "net.set_held_slot".to_string(),
            params: json!({}),
        },
        &state,
    );
    assert!(!missing_slot.ok);

    let invalid_slot = dispatch(
        ControlRequest {
            method: "net.set_held_slot".to_string(),
            params: json!({"slot": 9}),
        },
        &state,
    );
    assert!(!invalid_slot.ok);
}

#[test]
fn net_set_flying_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.set_flying".to_string(),
            params: json!({"flying": true}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::SetFlying { flying: true }]
    );

    let missing_flying = dispatch(
        ControlRequest {
            method: "net.set_flying".to_string(),
            params: json!({}),
        },
        &state,
    );
    assert!(!missing_flying.ok);
}

#[test]
fn input_focus_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "input.focus".to_string(),
            params: json!({"focused": true}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(
        state.requests.lock().unwrap().input_requests,
        vec![InputControlRequest::Focus { focused: true }]
    );

    let missing_focused = dispatch(
        ControlRequest {
            method: "input.focus".to_string(),
            params: json!({}),
        },
        &state,
    );
    assert!(!missing_focused.ok);
}

#[test]
fn input_key_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "input.key".to_string(),
            params: json!({"key": "forward", "pressed": true}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().input_requests,
        vec![InputControlRequest::Key {
            key: InputKeyControl::Forward,
            pressed: true,
        }]
    );

    let unknown_key = dispatch(
        ControlRequest {
            method: "input.key".to_string(),
            params: json!({"key": "fly", "pressed": true}),
        },
        &state,
    );
    assert!(!unknown_key.ok);

    let missing_pressed = dispatch(
        ControlRequest {
            method: "input.key".to_string(),
            params: json!({"key": "jump"}),
        },
        &state,
    );
    assert!(!missing_pressed.ok);
}

#[test]
fn input_text_queues_bounded_owned_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "input.text".to_string(),
            params: json!({"text": "hello 中"}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(
        state.requests.lock().unwrap().input_requests,
        vec![InputControlRequest::Text {
            text: "hello 中".to_string(),
        }]
    );
    assert!(
        !dispatch(
            ControlRequest {
                method: "input.text".to_string(),
                params: json!({}),
            },
            &state,
        )
        .ok
    );
    assert!(
        !dispatch(
            ControlRequest {
                method: "input.text".to_string(),
                params: json!({"text": "x".repeat(4097)}),
            },
            &state,
        )
        .ok
    );
}

#[test]
fn input_mouse_button_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "input.mouse_button".to_string(),
            params: json!({"button": "left", "pressed": true}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(
        state.requests.lock().unwrap().input_requests,
        vec![InputControlRequest::MouseButton {
            button: InputMouseButtonControl::Left,
            pressed: true,
        }]
    );

    let unknown_button = dispatch(
        ControlRequest {
            method: "input.mouse_button".to_string(),
            params: json!({"button": "back", "pressed": true}),
        },
        &state,
    );
    assert!(!unknown_button.ok);

    let missing_pressed = dispatch(
        ControlRequest {
            method: "input.mouse_button".to_string(),
            params: json!({"button": "right"}),
        },
        &state,
    );
    assert!(!missing_pressed.ok);
}

#[test]
fn input_mouse_tap_queues_atomic_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "input.mouse_tap".to_string(),
            params: json!({"button": "left"}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().input_requests,
        vec![InputControlRequest::MouseTap {
            button: InputMouseButtonControl::Left,
        }]
    );

    let unknown_button = dispatch(
        ControlRequest {
            method: "input.mouse_tap".to_string(),
            params: json!({"button": "back"}),
        },
        &state,
    );
    assert!(!unknown_button.ok);
}

#[test]
fn input_look_delta_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "input.look_delta".to_string(),
            params: json!({"dx": 12.5, "dy": -3.0}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().input_requests,
        vec![InputControlRequest::LookDelta { dx: 12.5, dy: -3.0 }]
    );

    let non_finite = dispatch(
        ControlRequest {
            method: "input.look_delta".to_string(),
            params: json!({"dx": f64::NAN, "dy": 0.0}),
        },
        &state,
    );
    assert!(!non_finite.ok);

    let missing_dy = dispatch(
        ControlRequest {
            method: "input.look_delta".to_string(),
            params: json!({"dx": 1.0}),
        },
        &state,
    );
    assert!(!missing_dy.ok);
}

#[test]
fn net_perform_respawn_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.perform_respawn".to_string(),
            params: json!({}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::PerformRespawn]
    );
}

#[test]
fn net_client_command_requests_queue_requests() {
    let state = test_control_state();

    let stats = dispatch(
        ControlRequest {
            method: "net.request_stats".to_string(),
            params: json!({}),
        },
        &state,
    );
    let game_rules = dispatch(
        ControlRequest {
            method: "net.request_game_rule_values".to_string(),
            params: json!({}),
        },
        &state,
    );

    assert!(stats.ok);
    assert!(game_rules.ok);
    assert_eq!(game_rules.result.unwrap()["pending"], 2);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![
            NetControlRequest::RequestStats,
            NetControlRequest::RequestGameRuleValues,
        ]
    );
}

#[test]
fn net_place_recipe_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.place_recipe".to_string(),
            params: json!({"container_id": 7, "recipe_index": 123, "use_max_items": true}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::PlaceRecipe {
            container_id: 7,
            recipe_index: 123,
            use_max_items: true,
        }]
    );

    let missing_recipe = dispatch(
        ControlRequest {
            method: "net.place_recipe".to_string(),
            params: json!({"container_id": 7, "use_max_items": true}),
        },
        &state,
    );
    assert!(!missing_recipe.ok);
}

#[test]
fn net_recipe_book_commands_queue_requests() {
    let state = test_control_state();
    let change_response = dispatch(
        ControlRequest {
            method: "net.change_recipe_book_settings".to_string(),
            params: json!({
                "book_type": "blast_furnace",
                "open": true,
                "filtering": false
            }),
        },
        &state,
    );
    let seen_response = dispatch(
        ControlRequest {
            method: "net.mark_recipe_seen".to_string(),
            params: json!({"recipe_index": 321}),
        },
        &state,
    );

    assert!(change_response.ok);
    assert!(seen_response.ok);
    assert_eq!(seen_response.result.unwrap()["pending"], 2);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![
            NetControlRequest::ChangeRecipeBookSettings {
                book_type: RecipeBookTypeControl::BlastFurnace,
                open: true,
                filtering: false,
            },
            NetControlRequest::MarkRecipeSeen { recipe_index: 321 },
        ]
    );

    let missing_open = dispatch(
        ControlRequest {
            method: "net.change_recipe_book_settings".to_string(),
            params: json!({"book_type": "crafting", "filtering": false}),
        },
        &state,
    );
    assert!(!missing_open.ok);

    let invalid_type = dispatch(
        ControlRequest {
            method: "net.change_recipe_book_settings".to_string(),
            params: json!({"book_type": "campfire", "open": true, "filtering": false}),
        },
        &state,
    );
    assert!(!invalid_type.ok);

    let missing_recipe = dispatch(
        ControlRequest {
            method: "net.mark_recipe_seen".to_string(),
            params: json!({}),
        },
        &state,
    );
    assert!(!missing_recipe.ok);
}

#[test]
fn net_rename_item_queues_request_and_validates_name() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.rename_item".to_string(),
            params: json!({"name": "Polished Pickaxe"}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::RenameItem {
            name: "Polished Pickaxe".to_string()
        }]
    );

    let non_string_name = dispatch(
        ControlRequest {
            method: "net.rename_item".to_string(),
            params: json!({"name": 7}),
        },
        &state,
    );
    assert!(!non_string_name.ok);

    let oversized_name = dispatch(
        ControlRequest {
            method: "net.rename_item".to_string(),
            params: json!({"name": "x".repeat(RENAME_ITEM_MAX_NAME_CHARS + 1)}),
        },
        &state,
    );
    assert!(!oversized_name.ok);
    assert_eq!(state.requests.lock().unwrap().net_requests.len(), 1);
}

#[test]
fn net_edit_book_queues_unsigned_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.edit_book".to_string(),
            params: json!({
                "slot": 36,
                "pages": ["first page", "second page"],
            }),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::EditBook {
            slot: 36,
            pages: vec!["first page".to_string(), "second page".to_string()],
            title: None,
        }]
    );
}

#[test]
fn net_edit_book_queues_signed_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.edit_book".to_string(),
            params: json!({
                "slot": 8,
                "pages": ["signed page"],
                "title": "Field Notes",
            }),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::EditBook {
            slot: 8,
            pages: vec!["signed page".to_string()],
            title: Some("Field Notes".to_string()),
        }]
    );

    let null_title = dispatch(
        ControlRequest {
            method: "net.edit_book".to_string(),
            params: json!({
                "slot": 8,
                "pages": ["still unsigned"],
                "title": null,
            }),
        },
        &state,
    );
    assert!(null_title.ok);
    assert_eq!(
        state.requests.lock().unwrap().net_requests[1],
        NetControlRequest::EditBook {
            slot: 8,
            pages: vec!["still unsigned".to_string()],
            title: None,
        }
    );
}

#[test]
fn net_edit_book_rejects_missing_or_non_array_pages() {
    let state = test_control_state();
    let missing_pages = dispatch(
        ControlRequest {
            method: "net.edit_book".to_string(),
            params: json!({"slot": 36}),
        },
        &state,
    );
    let non_array_pages = dispatch(
        ControlRequest {
            method: "net.edit_book".to_string(),
            params: json!({"slot": 36, "pages": "not pages"}),
        },
        &state,
    );

    assert!(!missing_pages.ok);
    assert!(!non_array_pages.ok);
    assert!(state.requests.lock().unwrap().net_requests.is_empty());
}

#[test]
fn net_edit_book_rejects_too_many_pages() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.edit_book".to_string(),
            params: json!({"slot": 36, "pages": vec!["x"; EDIT_BOOK_MAX_PAGES + 1]}),
        },
        &state,
    );

    assert!(!response.ok);
    assert!(state.requests.lock().unwrap().net_requests.is_empty());
}

#[test]
fn net_edit_book_rejects_oversized_page() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.edit_book".to_string(),
            params: json!({
                "slot": 36,
                "pages": ["x".repeat(EDIT_BOOK_MAX_PAGE_CHARS + 1)],
            }),
        },
        &state,
    );

    assert!(!response.ok);
    assert!(state.requests.lock().unwrap().net_requests.is_empty());
}

#[test]
fn net_edit_book_rejects_oversized_title() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.edit_book".to_string(),
            params: json!({
                "slot": 36,
                "pages": ["page"],
                "title": "x".repeat(EDIT_BOOK_MAX_TITLE_CHARS + 1),
            }),
        },
        &state,
    );

    assert!(!response.ok);
    assert!(state.requests.lock().unwrap().net_requests.is_empty());
}

#[test]
fn net_edit_book_limits_count_utf16_code_units() {
    let state = test_control_state();
    for params in [
        json!({
            "slot": 3,
            "pages": ["😀".repeat(EDIT_BOOK_MAX_PAGE_CHARS / 2 + 1)],
        }),
        json!({
            "slot": 3,
            "pages": ["page"],
            "title": "😀".repeat(EDIT_BOOK_MAX_TITLE_CHARS / 2 + 1),
        }),
    ] {
        let response = dispatch(
            ControlRequest {
                method: "net.edit_book".to_string(),
                params,
            },
            &state,
        );
        assert!(!response.ok);
    }
    assert!(state.requests.lock().unwrap().net_requests.is_empty());
}

#[test]
fn net_update_sign_queues_request_and_validates_lines() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.update_sign".to_string(),
            params: json!({
                "x": -5,
                "y": 70,
                "z": 12,
                "is_front_text": false,
                "lines": ["line 0", "line 1", "line 2", "line 3"]
            }),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::SignUpdate {
            x: -5,
            y: 70,
            z: 12,
            is_front_text: false,
            lines: [
                "line 0".to_string(),
                "line 1".to_string(),
                "line 2".to_string(),
                "line 3".to_string(),
            ],
        }]
    );

    let wrong_line_count = dispatch(
        ControlRequest {
            method: "net.update_sign".to_string(),
            params: json!({
                "x": -5,
                "y": 70,
                "z": 12,
                "is_front_text": false,
                "lines": ["line 0", "line 1", "line 2"]
            }),
        },
        &state,
    );
    assert!(!wrong_line_count.ok);

    let oversized_line = dispatch(
        ControlRequest {
            method: "net.update_sign".to_string(),
            params: json!({
                "x": -5,
                "y": 70,
                "z": 12,
                "is_front_text": false,
                "lines": ["a".repeat(385), "line 1", "line 2", "line 3"]
            }),
        },
        &state,
    );
    assert!(!oversized_line.ok);
}

#[test]
fn net_advancements_open_tab_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.open_advancements_tab".to_string(),
            params: json!({"tab": "minecraft:story/root"}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::OpenAdvancementsTab {
            tab: "minecraft:story/root".to_string()
        }]
    );
}

#[test]
fn net_advancements_open_tab_accepts_default_namespace_path() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.open_advancements_tab".to_string(),
            params: json!({"tab": "story/root"}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["queued"], true);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::OpenAdvancementsTab {
            tab: "story/root".to_string()
        }]
    );
}

#[test]
fn net_advancements_close_screen_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.close_advancements_screen".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::CloseAdvancementsScreen]
    );
}

#[test]
fn net_advancements_open_tab_rejects_invalid_tab() {
    let state = test_control_state();
    for tab in [
        "",
        "minecraft:",
        ":story/root",
        "Minecraft:story/root",
        "minecraft:Story/root",
        "minecraft:story root",
        "mine/craft:story/root",
        "minecraft:story:root",
    ] {
        let response = dispatch(
            ControlRequest {
                method: "net.open_advancements_tab".to_string(),
                params: json!({"tab": tab}),
            },
            &state,
        );
        assert!(!response.ok, "{tab:?} should be rejected");
    }

    assert!(state.requests.lock().unwrap().net_requests.is_empty());
}

#[test]
fn net_advancements_open_tab_requires_tab() {
    let state = test_control_state();
    let missing_tab = dispatch(
        ControlRequest {
            method: "net.open_advancements_tab".to_string(),
            params: json!({}),
        },
        &state,
    );
    let non_string_tab = dispatch(
        ControlRequest {
            method: "net.open_advancements_tab".to_string(),
            params: json!({"tab": 7}),
        },
        &state,
    );

    assert!(!missing_tab.ok);
    assert!(!non_string_tab.ok);
    assert!(state.requests.lock().unwrap().net_requests.is_empty());
}

#[test]
fn net_select_trade_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.select_trade".to_string(),
            params: json!({"item": 2}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::SelectTrade { item: 2 }]
    );

    let missing_item = dispatch(
        ControlRequest {
            method: "net.select_trade".to_string(),
            params: json!({}),
        },
        &state,
    );
    assert!(!missing_item.ok);

    let invalid_item = dispatch(
        ControlRequest {
            method: "net.select_trade".to_string(),
            params: json!({"item": -1}),
        },
        &state,
    );
    assert!(!invalid_item.ok);
}

#[test]
fn net_set_beacon_queues_both_effects() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.set_beacon".to_string(),
            params: json!({"primary_effect": 1, "secondary_effect": 5}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::SetBeacon {
            primary_effect: Some(1),
            secondary_effect: Some(5),
        }]
    );
}

#[test]
fn net_set_beacon_queues_null_and_missing_effects_as_none() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.set_beacon".to_string(),
            params: json!({"primary_effect": null}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::SetBeacon {
            primary_effect: None,
            secondary_effect: None,
        }]
    );
}

#[test]
fn net_set_beacon_rejects_non_integer_effects() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.set_beacon".to_string(),
            params: json!({"primary_effect": "speed"}),
        },
        &state,
    );

    assert!(!response.ok);
    assert_eq!(
        response.error.as_deref(),
        Some("net.set_beacon requires integer or null param primary_effect")
    );
    assert!(state.requests.lock().unwrap().net_requests.is_empty());
}

#[test]
fn net_set_beacon_counter_deserializes_with_default() {
    let mut value = serde_json::to_value(NetCounters::default()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .remove("set_beacon_commands_queued");

    let counters: NetCounters = serde_json::from_value(value).unwrap();

    assert_eq!(counters.set_beacon_commands_queued, 0);
}

#[test]
fn net_set_creative_mode_slot_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.set_creative_mode_slot".to_string(),
            params: json!({
                "slot_num": 36,
                "item": {
                    "kind": "item",
                    "item_id": 42,
                    "count": 64
                }
            }),
        },
        &state,
    );

    assert!(response.ok, "{response:?}");
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::SetCreativeModeSlot(
            crate::control::types::CreativeModeSlotControlRequest {
                slot_num: 36,
                item: crate::control::types::CreativeModeItemStackControl::Item {
                    item_id: 42,
                    count: 64,
                },
            }
        )]
    );
}

#[test]
fn net_set_creative_mode_slot_defaults_missing_item_to_empty_drop() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.set_creative_mode_slot".to_string(),
            params: json!({"slot_num": -1}),
        },
        &state,
    );

    assert!(response.ok, "{response:?}");
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::SetCreativeModeSlot(
            crate::control::types::CreativeModeSlotControlRequest {
                slot_num: -1,
                item: crate::control::types::CreativeModeItemStackControl::Empty,
            }
        )]
    );
}

#[test]
fn net_set_creative_mode_slot_rejects_invalid_item_stack() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.set_creative_mode_slot".to_string(),
            params: json!({
                "slot_num": 36,
                "item": {
                    "kind": "item",
                    "item_id": 42,
                    "count": 0
                }
            }),
        },
        &state,
    );

    assert!(!response.ok);
    assert_eq!(
        response.error.as_deref(),
        Some("net.set_creative_mode_slot requires item.count > 0")
    );
    assert!(state.requests.lock().unwrap().net_requests.is_empty());
}

#[test]
fn net_set_creative_mode_slot_counter_deserializes_with_default() {
    let mut value = serde_json::to_value(NetCounters::default()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .remove("set_creative_mode_slot_commands_queued");

    let counters: NetCounters = serde_json::from_value(value).unwrap();

    assert_eq!(counters.set_creative_mode_slot_commands_queued, 0);
}

#[test]
fn net_select_bundle_item_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.select_bundle_item".to_string(),
            params: json!({"slot_id": 12, "selected_item_index": 3}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::SelectBundleItem {
            slot_id: 12,
            selected_item_index: 3,
        }]
    );

    let unselect_response = dispatch(
        ControlRequest {
            method: "net.select_bundle_item".to_string(),
            params: json!({"slot_id": 12, "selected_item_index": -1}),
        },
        &state,
    );

    assert!(unselect_response.ok);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![
            NetControlRequest::SelectBundleItem {
                slot_id: 12,
                selected_item_index: 3,
            },
            NetControlRequest::SelectBundleItem {
                slot_id: 12,
                selected_item_index: -1,
            },
        ]
    );

    let missing_index = dispatch(
        ControlRequest {
            method: "net.select_bundle_item".to_string(),
            params: json!({"slot_id": 12}),
        },
        &state,
    );
    assert!(!missing_index.ok);

    let invalid_index = dispatch(
        ControlRequest {
            method: "net.select_bundle_item".to_string(),
            params: json!({"slot_id": 12, "selected_item_index": -2}),
        },
        &state,
    );
    assert!(!invalid_index.ok);
    assert_eq!(state.requests.lock().unwrap().net_requests.len(), 2);
}

#[test]
fn net_container_button_click_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.container_button_click".to_string(),
            params: json!({"container_id": 7, "button_id": 2}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::ContainerButtonClick {
            container_id: 7,
            button_id: 2,
        }]
    );

    let missing_button = dispatch(
        ControlRequest {
            method: "net.container_button_click".to_string(),
            params: json!({"container_id": 7}),
        },
        &state,
    );
    assert!(!missing_button.ok);
}

#[test]
fn net_container_click_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.container_click".to_string(),
            params: json!({
                "container_id": 7,
                "state_id": 33,
                "slot_num": 5,
                "button_num": 1,
                "input": "pickup",
                "changed_slots": [{
                    "slot": 5,
                    "stack": {
                        "kind": "item",
                        "item_id": 42,
                        "count": 64,
                        "components": {
                            "added_components": {"10": 16909060},
                            "removed_components": [20]
                        }
                    }
                }],
                "carried_item": {"kind": "empty"}
            }),
        },
        &state,
    );

    assert!(response.ok, "{response:?}");
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::ContainerClick(
            ContainerClickControlRequest {
                container_id: 7,
                state_id: 33,
                slot_num: 5,
                button_num: 1,
                input: crate::control::types::ContainerInputControl::Pickup,
                changed_slots: vec![crate::control::types::ContainerChangedSlotControl {
                    slot: 5,
                    stack: crate::control::types::HashedStackControl::Item {
                        item_id: 42,
                        count: 64,
                        components: crate::control::types::HashedComponentPatchControl {
                            added_components: BTreeMap::from([(10, 0x0102_0304)]),
                            removed_components: BTreeSet::from([20]),
                        },
                    },
                }],
                carried_item: crate::control::types::HashedStackControl::Empty,
            }
        )]
    );

    let missing_input = dispatch(
        ControlRequest {
            method: "net.container_click".to_string(),
            params: json!({
                "container_id": 7,
                "state_id": 33,
                "slot_num": 5,
                "button_num": 1
            }),
        },
        &state,
    );
    assert!(!missing_input.ok);
}

#[test]
fn net_container_click_slot_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.container_click_slot".to_string(),
            params: json!({
                "slot_num": 5,
                "button_num": 1,
                "input": "pickup"
            }),
        },
        &state,
    );

    assert!(response.ok, "{response:?}");
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::ContainerClickSlot(
            crate::control::types::ContainerClickSlotControlRequest {
                slot_num: 5,
                button_num: 1,
                input: crate::control::types::ContainerInputControl::Pickup,
            }
        )]
    );

    let missing_input = dispatch(
        ControlRequest {
            method: "net.container_click_slot".to_string(),
            params: json!({
                "slot_num": 5,
                "button_num": 1
            }),
        },
        &state,
    );
    assert!(!missing_input.ok);
}

#[test]
fn net_container_close_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.container_close".to_string(),
            params: json!({"container_id": 7}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::ContainerClose { container_id: 7 }]
    );

    let missing_container = dispatch(
        ControlRequest {
            method: "net.container_close".to_string(),
            params: json!({}),
        },
        &state,
    );
    assert!(!missing_container.ok);
}

#[test]
fn net_container_slot_state_changed_queues_request() {
    let state = test_control_state();
    let response = dispatch(
        ControlRequest {
            method: "net.container_slot_state_changed".to_string(),
            params: json!({"slot_id": 12, "container_id": 7, "new_state": true}),
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["pending"], 1);
    assert_eq!(
        state.requests.lock().unwrap().net_requests,
        vec![NetControlRequest::ContainerSlotStateChanged {
            slot_id: 12,
            container_id: 7,
            new_state: true,
        }]
    );

    let missing_state = dispatch(
        ControlRequest {
            method: "net.container_slot_state_changed".to_string(),
            params: json!({"slot_id": 12, "container_id": 7}),
        },
        &state,
    );
    assert!(!missing_state.ok);
}

#[test]
fn client_hud_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_system_chat(SystemChat::plain("Server restart soon", false));
        store.apply_action_bar_text(SetActionBarText::plain("Action ready"));
        store.apply_titles_animation(SetTitlesAnimation {
            fade_in: 5,
            stay: 40,
            fade_out: 15,
        });
        store.apply_title_text(SetTitleText::plain("Quest complete"));
        store.apply_subtitle_text(SetSubtitleText::plain("Return to camp"));
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_hud".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let hud = response.result.unwrap();
    assert_eq!(hud["system_chat"]["content"], "Server restart soon");
    assert_eq!(hud["system_chat"]["overlay"], false);
    assert_eq!(hud["action_bar"]["content"]["text"], "Action ready");
    assert_eq!(hud["action_bar"]["display_ticks"], 60);
    assert_eq!(hud["title"]["title"]["text"], "Quest complete");
    assert_eq!(hud["title"]["subtitle"]["text"], "Return to camp");
    assert_eq!(hud["title"]["fade_in"], 5);
    assert_eq!(hud["title"]["stay"], 40);
    assert_eq!(hud["title"]["fade_out"], 15);
    assert_eq!(hud["title"]["title_time"], 60);

    let mut store = WorldStore::new();
    store.apply_title_text(SetTitleText {
        content: bbb_protocol::RichTextComponent::try_from_component_nbt(
            bbb_protocol::NbtSummaryValue::String(vec![0xd800, 0x41].into()),
        )
        .unwrap(),
    });
    publish_test_world(&state, store);
    let response = dispatch(
        ControlRequest {
            method: "world.client_hud".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );
    assert!(response.ok);
    assert_eq!(
        response.result.unwrap()["title"]["title"]["component"],
        json!({"type": "String", "value": "\u{fffd}A"})
    );
    let snapshot = state.snapshot.load();
    let hud = snapshot.world_view.world.client_hud();
    let component = hud.title.title.as_ref().unwrap();
    let bbb_protocol::NbtSummaryValue::String(raw) = component.component() else {
        panic!("expected the original string component");
    };
    assert_eq!(raw.units(), &[0xd800, 0x41]);
}

#[test]
fn client_inventory_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        let mut locked_stack = item_stack(43, 2);
        locked_stack.component_patch.added_type_ids = vec![
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID,
        ];
        locked_stack.component_patch.creative_slot_lock = true;
        locked_stack.component_patch.lore = vec!["Exported lore".to_string()];
        locked_stack.component_patch.lore_nbt =
            vec![bbb_protocol::NbtSummaryValue::Compound(vec![
                bbb_protocol::NbtSummaryEntry {
                    name: "text".into(),
                    value: bbb_protocol::NbtSummaryValue::string("Exported lore"),
                },
                bbb_protocol::NbtSummaryEntry {
                    name: "italic".into(),
                    value: bbb_protocol::NbtSummaryValue::Byte(0),
                },
            ])];
        locked_stack.component_patch.lore_styled = locked_stack
            .component_patch
            .lore_nbt
            .iter()
            .map(bbb_protocol::styled_component_runs_from_nbt_value)
            .collect();
        store.apply_set_player_inventory(SetPlayerInventory {
            slot: 36,
            item: locked_stack,
        });
        store.apply_set_cursor_item(SetCursorItem {
            item: item_stack(99, 1),
        });
        store.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
            title: bbb_protocol::RichTextComponent::plain("Chest"),
        });
        store.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![ItemStackSummary::empty(), item_stack(42, 64)],
            carried_item: ItemStackSummary::empty(),
        });
        store.apply_container_set_slot(ContainerSetSlot {
            container_id: 7,
            state_id: 13,
            slot: 1,
            item: item_stack(44, 3),
        });
        store.apply_container_set_data(ContainerSetData {
            container_id: 7,
            id: 2,
            value: 10,
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_inventory".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let inventory = response.result.unwrap();
    assert_eq!(inventory["player_slots"][0]["slot"], 36);
    assert_eq!(inventory["player_slots"][0]["item"]["item_id"], 43);
    assert_eq!(inventory["player_slots"][0]["item"]["count"], 2);
    assert_eq!(
        inventory["player_slots"][0]["item"]["component_patch"]["creative_slot_lock"],
        true
    );
    let lore = &inventory["player_slots"][0]["item"]["component_patch"]["lore_styled"];
    assert_eq!(lore[0][0]["text"], "Exported lore");
    assert_eq!(lore[0][0]["style"]["italic"], false);
    assert_eq!(inventory["cursor_item"]["item_id"], serde_json::Value::Null);
    assert_eq!(inventory["cursor_item"]["count"], 0);
    assert_eq!(inventory["open_container"]["container_id"], 7);
    assert_eq!(inventory["open_container"]["menu_type_id"], 2);
    assert_eq!(inventory["open_container"]["title"]["text"], "Chest");
    assert_eq!(
        inventory["open_container"]["title"]["styled"][0]["text"],
        "Chest"
    );
    assert_eq!(
        inventory["open_container"]["title"]["component"]["type"],
        "String"
    );
    assert_eq!(
        inventory["open_container"]["title"]["component"]["value"],
        "Chest"
    );
    assert_eq!(inventory["open_container"]["state_id"], 13);
    assert_eq!(
        inventory["open_container"]["slots"][0]["item"]["item_id"],
        serde_json::Value::Null
    );
    assert_eq!(inventory["open_container"]["slots"][1]["slot"], 1);
    assert_eq!(
        inventory["open_container"]["slots"][1]["item"]["item_id"],
        44
    );
    assert_eq!(inventory["open_container"]["slots"][1]["item"]["count"], 3);
    assert_eq!(inventory["open_container"]["data_values"][0]["id"], 2);
    assert_eq!(inventory["open_container"]["data_values"][0]["value"], 10);
}

#[test]
fn client_inventory_reads_merchant_offers_and_close_from_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
            title: bbb_protocol::RichTextComponent::plain("Merchant"),
        });
        let mut sell_with_additional_cost = item_stack(99, 1);
        sell_with_additional_cost.component_patch.added_type_ids = vec![
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID,
        ];
        sell_with_additional_cost
            .component_patch
            .additional_trade_cost = Some(-7);
        assert!(store.apply_merchant_offers(MerchantOffers {
            container_id: 7,
            offers: vec![
                MerchantOffer {
                    buy_a: item_cost(42, 3),
                    sell: sell_with_additional_cost,
                    buy_b: Some(item_cost(43, 4)),
                    is_out_of_stock: false,
                    uses: 1,
                    max_uses: 12,
                    xp: 8,
                    special_price_diff: -2,
                    price_multiplier: 0.05,
                    demand: 6,
                },
                MerchantOffer {
                    buy_a: item_cost(44, 5),
                    sell: item_stack(100, 2),
                    buy_b: None,
                    is_out_of_stock: true,
                    uses: 12,
                    max_uses: 12,
                    xp: 9,
                    special_price_diff: 1,
                    price_multiplier: 0.1,
                    demand: 7,
                },
            ],
            villager_level: 3,
            villager_xp: 120,
            show_progress: true,
            can_restock: false,
        }));
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_inventory".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let inventory = response.result.unwrap();
    let offers = &inventory["open_container"]["merchant_offers"];
    assert_eq!(offers["container_id"], 7);
    assert_eq!(offers["villager_level"], 3);
    assert_eq!(offers["villager_xp"], 120);
    assert_eq!(offers["show_progress"], true);
    assert_eq!(offers["can_restock"], false);
    assert_eq!(offers["offers"].as_array().unwrap().len(), 2);
    assert_eq!(offers["offers"][0]["buy_a"]["item_id"], 42);
    assert_eq!(offers["offers"][0]["buy_a"]["count"], 3);
    assert_eq!(offers["offers"][0]["sell"]["item_id"], 99);
    assert_eq!(offers["offers"][0]["sell"]["count"], 1);
    assert_eq!(
        offers["offers"][0]["sell"]["component_patch"]["additional_trade_cost"],
        -7
    );
    assert_eq!(offers["offers"][0]["buy_b"]["item_id"], 43);
    assert_eq!(offers["offers"][0]["buy_b"]["count"], 4);
    assert_eq!(offers["offers"][0]["is_out_of_stock"], false);
    assert_eq!(offers["offers"][0]["uses"], 1);
    assert_eq!(offers["offers"][0]["max_uses"], 12);
    assert_eq!(offers["offers"][0]["xp"], 8);
    assert_eq!(offers["offers"][0]["special_price_diff"], -2);
    assert!((offers["offers"][0]["price_multiplier"].as_f64().unwrap() - 0.05).abs() < 0.0001);
    assert_eq!(offers["offers"][0]["demand"], 6);
    assert_eq!(offers["offers"][1]["buy_b"], serde_json::Value::Null);
    assert_eq!(offers["offers"][1]["is_out_of_stock"], true);

    let mut closed_world = (*state.snapshot.load().world_view.world).clone();
    assert!(closed_world.apply_container_close(ContainerClose { container_id: 7 }));
    publish_test_world(&state, closed_world);
    let response = dispatch(
        ControlRequest {
            method: "world.client_inventory".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    assert!(response.result.unwrap()["open_container"].is_null());
}

#[test]
fn client_recipe_book_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_recipe_book_add(RecipeBookAdd {
            replace: true,
            entries: vec![
                recipe_book_entry(7, true, true),
                recipe_book_entry(8, false, false),
            ],
        });
        store.apply_recipe_book_remove(RecipeBookRemove {
            recipe_ids: vec![RecipeDisplayId { index: 8 }],
        });
        store.apply_recipe_book_settings(RecipeBookSettings {
            crafting: RecipeBookTypeSettings {
                open: true,
                filtering: false,
            },
            furnace: RecipeBookTypeSettings {
                open: false,
                filtering: true,
            },
            blast_furnace: RecipeBookTypeSettings {
                open: true,
                filtering: true,
            },
            smoker: RecipeBookTypeSettings {
                open: false,
                filtering: false,
            },
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_recipe_book".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let recipe_book = response.result.unwrap();
    let known = recipe_book["known"].as_object().unwrap();
    assert!(known.contains_key("7"));
    assert!(!known.contains_key("8"));
    assert_eq!(recipe_book["known"]["7"]["id"]["index"], 7);
    assert_eq!(
        recipe_book["known"]["7"]["category_id"],
        VANILLA_RECIPE_BOOK_CATEGORY_STONECUTTER_ID
    );
    assert_eq!(
        recipe_book["known"]["7"]["crafting_requirements"][0]["item_ids"],
        json!([42])
    );
    assert_eq!(recipe_book["highlights"], json!([7]));
    assert_eq!(recipe_book["notification_ids"], json!([7]));
    assert_eq!(recipe_book["settings"]["crafting"]["open"], true);
    assert_eq!(recipe_book["settings"]["furnace"]["filtering"], true);
    assert_eq!(recipe_book["settings"]["smoker"]["open"], false);
}

#[test]
fn client_recipes_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_update_recipes(UpdateRecipes {
            property_sets: vec![RecipePropertySetSummary {
                key: "minecraft:furnace_input".to_string(),
                item_ids: vec![42, 43],
            }],
            stonecutter_recipes: vec![StonecutterSelectableRecipeSummary {
                input: IngredientSummary {
                    tag: None,
                    item_ids: vec![11, 12],
                },
                option_display: SlotDisplaySummary {
                    display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
                    raw_payload: vec![4, 77],
                    item_stack: None,
                    tag: None,
                },
            }],
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_recipes".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let recipes = response.result.unwrap();
    assert_eq!(
        recipes["property_sets"]["minecraft:furnace_input"],
        json!([42, 43])
    );
    assert_eq!(
        recipes["stonecutter_recipes"][0]["input"]["item_ids"],
        json!([11, 12])
    );
    assert_eq!(
        recipes["stonecutter_recipes"][0]["option_display"]["display_type_id"],
        VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID
    );
    assert_eq!(
        recipes["stonecutter_recipes"][0]["option_display"]["raw_payload"],
        json!([4, 77])
    );
}
