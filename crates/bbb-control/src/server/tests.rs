use std::collections::{BTreeMap, BTreeSet};

use super::params::{
    EDIT_BOOK_MAX_PAGES, EDIT_BOOK_MAX_PAGE_CHARS, EDIT_BOOK_MAX_TITLE_CHARS,
    RENAME_ITEM_MAX_NAME_CHARS,
};
use super::*;
use crate::types::{AudioCounters, NetCounters, RecipeBookTypeControl};
use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, AdvancementSummary, AwardStats, BlockChangedAck,
    BlockDestruction, BlockEvent, BlockPos as ProtocolBlockPos, ChatFormatting, ChatTypeBound,
    ChatTypeHolder, CommandArgumentParser, CommandNode, CommandNodeType, Commands,
    CommonPlayerSpawnInfo as ProtocolSpawnInfo, ContainerClose, ContainerSetContent,
    ContainerSetData, ContainerSetSlot, Cooldown, CustomChatCompletions,
    CustomChatCompletionsAction, CustomPayload, CustomPayloadBody, CustomReportDetails,
    DamageEvent as ProtocolDamageEvent, DebugBlockValue, DialogHolder,
    DisguisedChat as ProtocolDisguisedChat, EntityEvent as ProtocolEntityEvent,
    EntityPositionSync as ProtocolEntityPositionSync, Explosion as ProtocolExplosion, GameEvent,
    GameProfile, GameProfileProperty, GameRuleValue, GameRuleValues, GameType,
    HurtAnimation as ProtocolHurtAnimation, IngredientSummary, InitializeBorder, InteractionHand,
    ItemCostSummary, ItemStackSummary, KnownPack, LevelEvent,
    LevelParticles as ProtocolLevelParticles, MapColorPatch, MapDecoration, MapItemData,
    MerchantOffer, MerchantOffers, MobEffectFlags, MountScreenOpen, ObjectiveRenderType, OpenBook,
    OpenScreen, OpenSignEditor, ParticlePayload, PlaceGhostRecipe, PlayLogin as ProtocolPlayLogin,
    PlayTime, PlayerAbilities, PlayerCombatKill, PlayerExperience, PlayerHealth, PlayerInfoAction,
    PlayerInfoEntry, PlayerInfoUpdate, PlayerTeamMethod, PlayerTeamParameters, PongResponse,
    ProjectilePower, RecipeBookAdd, RecipeBookAddEntry, RecipeBookRemove, RecipeBookSettings,
    RecipeBookTypeSettings, RecipeDisplayEntry, RecipeDisplayId, RecipeDisplaySummary,
    RecipeDisplayType, RecipePropertySetSummary, RegistryData, RegistryDataEntry, RegistryTags,
    ScoreboardDisplaySlot, SelectAdvancementsTab, ServerLinkEntry, ServerLinkKnownType,
    ServerLinkType, ServerLinks, SetActionBarText, SetBorderCenter, SetBorderLerpSize,
    SetBorderWarningDelay, SetBorderWarningDistance, SetChunkCacheCenter, SetChunkCacheRadius,
    SetCursorItem, SetDefaultSpawnPosition, SetDisplayObjective, SetObjective, SetObjectiveMethod,
    SetObjectiveParameters, SetPlayerInventory, SetPlayerTeam, SetScore, SetSimulationDistance,
    SetSubtitleText, SetTitleText, SetTitlesAnimation, ShowDialog, SlotDisplaySummary, SoundEvent,
    SoundEventHolder, SoundSource, StatUpdate, StonecutterSelectableRecipeSummary, StopSound,
    SystemChat, TagNetworkPayload, TagQuery, TeamCollisionRule, TeamVisibility, TickingState,
    TickingStep, TrackedWaypoint, TrackedWaypointPacket, Transfer, UpdateAdvancements,
    UpdateEnabledFeatures, UpdateMobEffect, UpdateRecipes, UpdateTags, Vec3d as ProtocolVec3d,
    WaypointData, WaypointIcon, WaypointIdentifier, WaypointOperation, WaypointVec3i,
};
use bbb_world::{
    BlockEntityRecord, ChunkColumn, ChunkSection, ChunkState, HeightmapData, LightData,
    PaletteDomain, PaletteKind, PalettedContainerData, WorldDimension, WorldStore,
};
use serde_json::json;
use uuid::Uuid;

#[test]
fn app_quit_marks_snapshot_not_running() {
    let state = ControlState::new("test");
    let response = dispatch(
        ControlRequest {
            method: "app.quit".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    assert!(!state.snapshot.read().unwrap().app.running);
}

#[test]
fn app_status_reads_world_counters_from_world_store() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_block_changed_ack(BlockChangedAck { sequence: 17 });
        state.snapshot.write().unwrap().audio = AudioCounters {
            enabled: true,
            catalog_events: 123,
            commands_submitted: 4,
            ..AudioCounters::default()
        };
        *state.world.write().unwrap() = store;
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
fn audio_counters_reads_runtime_projection() {
    let state = ControlState::new("test");
    state.snapshot.write().unwrap().audio = AudioCounters {
        enabled: false,
        disabled_reason: Some("initialize Kira audio runtime".to_string()),
        resolve_failures: 2,
        submit_failures: 1,
        last_resolve_error: Some("missing sound event minecraft:missing".to_string()),
        last_submit_error: Some("failed to submit audio command".to_string()),
        ..AudioCounters::default()
    };

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
    let state = ControlState::new("test");
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
        *state.world.write().unwrap() = store;
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
    let state = ControlState::new("test");
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
        *state.world.write().unwrap() = store;
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

    state.world.write().unwrap().clear_client_level();
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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

    let counters: NetCounters = serde_json::from_value(value).unwrap();

    assert_eq!(counters.change_difficulty_commands_queued, 0);
    assert_eq!(counters.lock_difficulty_commands_queued, 0);
}

#[test]
fn net_set_held_slot_queues_request() {
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
fn net_perform_respawn_queues_request() {
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");

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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
fn net_update_sign_queues_request_and_validates_lines() {
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
            crate::types::CreativeModeSlotControlRequest {
                slot_num: 36,
                item: crate::types::CreativeModeItemStackControl::Item {
                    item_id: 42,
                    count: 64,
                },
            }
        )]
    );
}

#[test]
fn net_set_creative_mode_slot_defaults_missing_item_to_empty_drop() {
    let state = ControlState::new("test");
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
            crate::types::CreativeModeSlotControlRequest {
                slot_num: -1,
                item: crate::types::CreativeModeItemStackControl::Empty,
            }
        )]
    );
}

#[test]
fn net_set_creative_mode_slot_rejects_invalid_item_stack() {
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
                input: crate::types::ContainerInputControl::Pickup,
                changed_slots: vec![crate::types::ContainerChangedSlotControl {
                    slot: 5,
                    stack: crate::types::HashedStackControl::Item {
                        item_id: 42,
                        count: 64,
                        components: crate::types::HashedComponentPatchControl {
                            added_components: BTreeMap::from([(10, 0x0102_0304)]),
                            removed_components: BTreeSet::from([20]),
                        },
                    },
                }],
                carried_item: crate::types::HashedStackControl::Empty,
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
    let state = ControlState::new("test");
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
            crate::types::ContainerClickSlotControlRequest {
                slot_num: 5,
                button_num: 1,
                input: crate::types::ContainerInputControl::Pickup,
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
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
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_system_chat(SystemChat {
            content: "Server restart soon".to_string(),
            overlay: false,
        });
        store.apply_action_bar_text(SetActionBarText {
            content: "Action ready".to_string(),
        });
        store.apply_titles_animation(SetTitlesAnimation {
            fade_in: 5,
            stay: 40,
            fade_out: 15,
        });
        store.apply_title_text(SetTitleText {
            content: "Quest complete".to_string(),
        });
        store.apply_subtitle_text(SetSubtitleText {
            content: "Return to camp".to_string(),
        });
        *state.world.write().unwrap() = store;
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
    assert_eq!(hud["action_bar"]["content"], "Action ready");
    assert_eq!(hud["action_bar"]["display_ticks"], 60);
    assert_eq!(hud["title"]["title"], "Quest complete");
    assert_eq!(hud["title"]["subtitle"], "Return to camp");
    assert_eq!(hud["title"]["fade_in"], 5);
    assert_eq!(hud["title"]["stay"], 40);
    assert_eq!(hud["title"]["fade_out"], 15);
    assert_eq!(hud["title"]["title_time"], 60);
}

#[test]
fn client_inventory_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(SetPlayerInventory {
            slot: 36,
            item: item_stack(43, 2),
        });
        store.apply_set_cursor_item(SetCursorItem {
            item: item_stack(99, 1),
        });
        store.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: 2,
            title: "Chest".to_string(),
            title_styled: Vec::new(),
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
        *state.world.write().unwrap() = store;
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
    assert_eq!(inventory["cursor_item"]["item_id"], serde_json::Value::Null);
    assert_eq!(inventory["cursor_item"]["count"], 0);
    assert_eq!(inventory["open_container"]["container_id"], 7);
    assert_eq!(inventory["open_container"]["menu_type_id"], 2);
    assert_eq!(inventory["open_container"]["title"], "Chest");
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
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: 19,
            title: "Merchant".to_string(),
            title_styled: Vec::new(),
        });
        assert!(store.apply_merchant_offers(MerchantOffers {
            container_id: 7,
            offers: vec![
                MerchantOffer {
                    buy_a: item_cost(42, 3),
                    sell: item_stack(99, 1),
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
        *state.world.write().unwrap() = store;
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

    assert!(state
        .world
        .write()
        .unwrap()
        .apply_container_close(ContainerClose { container_id: 7 }));
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
    let state = ControlState::new("test");
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
        *state.world.write().unwrap() = store;
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
    assert_eq!(recipe_book["known"]["7"]["category_id"], 10);
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
    let state = ControlState::new("test");
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
                    display_type_id: 4,
                    raw_payload: vec![4, 77],
                    item_stack: None,
                    tag: None,
                },
            }],
        });
        *state.world.write().unwrap() = store;
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
        4
    );
    assert_eq!(
        recipes["stonecutter_recipes"][0]["option_display"]["raw_payload"],
        json!([4, 77])
    );
}

#[test]
fn client_chat_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_disguised_chat(ProtocolDisguisedChat {
            message: "server notice".to_string(),
            chat_type: ChatTypeBound {
                chat_type: ChatTypeHolder::Registry { id: 0 },
                name: "Server".to_string(),
                target_name: None,
            },
        });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_chat".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let chat = response.result.unwrap();
    assert_eq!(chat["messages"][0]["kind"], "Disguised");
    assert_eq!(chat["messages"][0]["content"], "server notice");
    assert_eq!(chat["messages"][0]["sender_name"], "Server");
    assert_eq!(chat["messages"][0]["chat_type"]["registry_id"], 0);
    assert_eq!(chat["messages"][0]["validation_state"], "Unsigned");
    assert_eq!(chat["deleted_messages"].as_array().unwrap().len(), 0);
}

#[test]
fn client_combat_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_player_combat_kill(PlayerCombatKill {
            player_id: 123,
            message: "You died".to_string(),
        });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_combat".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let combat = response.result.unwrap();
    assert_eq!(combat["last_combat"]["kind"], "kill");
    assert_eq!(combat["last_combat"]["duration"], serde_json::Value::Null);
    assert_eq!(combat["last_combat"]["player_id"], 123);
    assert_eq!(combat["last_combat"]["message"], "You died");
}

#[test]
fn client_cooldowns_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_cooldown(Cooldown {
            cooldown_group: "minecraft:ender_pearl".to_string(),
            duration: 20,
        });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_cooldowns".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let cooldowns = response.result.unwrap();
    assert_eq!(
        cooldowns["minecraft:ender_pearl"]["cooldown_group"],
        "minecraft:ender_pearl"
    );
    assert_eq!(cooldowns["minecraft:ender_pearl"]["duration"], 20);

    state.world.write().unwrap().apply_cooldown(Cooldown {
        cooldown_group: "minecraft:ender_pearl".to_string(),
        duration: 0,
    });
    let response = dispatch(
        ControlRequest {
            method: "world.client_cooldowns".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    assert!(response
        .result
        .unwrap()
        .get("minecraft:ender_pearl")
        .is_none());
}

#[test]
fn client_effects_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_explosion(ProtocolExplosion {
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
        });
        store.apply_level_particles(ProtocolLevelParticles {
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
        });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_effects".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let effects = response.result.unwrap();
    assert_eq!(effects["last_explosion"]["center"]["x"], 1.0);
    assert_eq!(effects["last_explosion"]["center"]["y"], 2.0);
    assert_eq!(effects["last_explosion"]["center"]["z"], 3.0);
    assert_eq!(effects["last_explosion"]["radius"], 4.5);
    assert_eq!(effects["last_explosion"]["block_count"], 7);
    assert_eq!(effects["last_explosion"]["player_knockback"]["x"], 0.25);
    assert_eq!(effects["last_explosion"]["player_knockback"]["y"], -0.5);
    assert_eq!(effects["last_explosion"]["player_knockback"]["z"], 1.5);
    assert_eq!(effects["last_explosion"]["raw_effect_payload_len"], 4);
    assert_eq!(effects["last_level_particles"]["override_limiter"], true);
    assert_eq!(effects["last_level_particles"]["always_show"], false);
    assert_eq!(effects["last_level_particles"]["position"]["x"], 10.0);
    assert_eq!(effects["last_level_particles"]["position"]["y"], 64.5);
    assert_eq!(effects["last_level_particles"]["position"]["z"], -3.25);
    assert_eq!(effects["last_level_particles"]["max_speed"], 1.5);
    assert_eq!(effects["last_level_particles"]["count"], 16);
    assert_eq!(effects["last_level_particles"]["particle_type_id"], 45);
    assert_eq!(effects["last_level_particles"]["raw_options_len"], 2);
}

#[test]
fn client_stats_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_award_stats(AwardStats {
            stats: vec![
                StatUpdate {
                    stat_type_id: 8,
                    value_id: 10,
                    amount: 3,
                },
                StatUpdate {
                    stat_type_id: 0,
                    value_id: 4,
                    amount: 11,
                },
                StatUpdate {
                    stat_type_id: 8,
                    value_id: 10,
                    amount: 5,
                },
            ],
        });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_stats".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let stats = response.result.unwrap();
    assert_eq!(stats["values"][0]["stat_type_id"], 0);
    assert_eq!(stats["values"][0]["value_id"], 4);
    assert_eq!(stats["values"][0]["amount"], 11);
    assert_eq!(stats["values"][1]["stat_type_id"], 8);
    assert_eq!(stats["values"][1]["value_id"], 10);
    assert_eq!(stats["values"][1]["amount"], 5);
    assert_eq!(stats["last_update"]["entries"][0]["stat_type_id"], 8);
    assert_eq!(stats["last_update"]["entries"][0]["value_id"], 10);
    assert_eq!(stats["last_update"]["entries"][0]["amount"], 3);
    assert_eq!(stats["last_update"]["entries"][2]["stat_type_id"], 8);
    assert_eq!(stats["last_update"]["entries"][2]["value_id"], 10);
    assert_eq!(stats["last_update"]["entries"][2]["amount"], 5);
}

#[test]
fn client_waypoints_reads_canonical_world_state() {
    let state = ControlState::new("test");
    let waypoint_id = uuid::Uuid::from_u128(0x00112233445566778899aabbccddeeff);
    {
        let mut store = WorldStore::new();
        store.apply_waypoint(TrackedWaypointPacket {
            operation: WaypointOperation::Track,
            waypoint: TrackedWaypoint {
                identifier: WaypointIdentifier::Uuid(waypoint_id),
                icon: WaypointIcon {
                    style: "minecraft:default".to_string(),
                    color_rgb: Some(0x112233),
                },
                data: WaypointData::Position(WaypointVec3i {
                    x: 10,
                    y: 64,
                    z: -5,
                }),
            },
        });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_waypoints".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let waypoints = response.result.unwrap();
    let key = format!("uuid:{waypoint_id}");
    assert_eq!(
        waypoints["tracked"][key.as_str()]["identifier_kind"],
        "uuid"
    );
    assert_eq!(
        waypoints["tracked"][key.as_str()]["data"]["position"]["x"],
        10
    );
    assert_eq!(waypoints["last_event"]["operation"], "track");
    assert_eq!(waypoints["last_event"]["applied"], true);
    assert_eq!(
        waypoints["last_event"]["waypoint"]["icon_color_rgb"],
        0x112233
    );
}

#[test]
fn client_ui_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_low_disk_space_warning();
        store.apply_code_of_conduct("Respect the realm.".to_string());
        store.apply_mount_screen_open(MountScreenOpen {
            container_id: 11,
            inventory_columns: 5,
            entity_id: 42,
        });
        store.apply_open_book(OpenBook {
            hand: InteractionHand::OffHand,
        });
        store.apply_open_sign_editor(OpenSignEditor {
            pos: ProtocolBlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            is_front_text: false,
        });
        store.apply_place_ghost_recipe(PlaceGhostRecipe {
            container_id: 9,
            recipe_display: RecipeDisplaySummary {
                display_type: RecipeDisplayType::Stonecutter,
                raw_body: vec![3, 4, 100, 4, 101, 4, 102],
                crafting: None,
            },
        });
        store.apply_show_dialog(ShowDialog {
            dialog: DialogHolder::Reference { registry_id: 7 },
        });
        store.apply_pong_response(PongResponse { time: 123456789 });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_ui".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let ui = response.result.unwrap();
    assert_eq!(ui["low_disk_space_warning_count"], 1);
    assert_eq!(ui["last_code_of_conduct"]["text"], "Respect the realm.");
    assert_eq!(
        ui["last_code_of_conduct"]["text_hash"],
        bbb_world::code_of_conduct_text_hash("Respect the realm.")
    );
    assert_eq!(ui["last_mount_screen"]["container_id"], 11);
    assert_eq!(ui["last_mount_screen"]["inventory_columns"], 5);
    assert_eq!(ui["last_mount_screen"]["entity_id"], 42);
    assert_eq!(ui["last_open_book"]["hand"], "off_hand");
    assert_eq!(ui["last_open_sign_editor"]["pos"]["x"], -5);
    assert_eq!(ui["last_open_sign_editor"]["pos"]["y"], 70);
    assert_eq!(ui["last_open_sign_editor"]["pos"]["z"], 12);
    assert_eq!(ui["last_open_sign_editor"]["is_front_text"], false);
    assert_eq!(ui["last_ghost_recipe"]["container_id"], 9);
    assert_eq!(ui["last_ghost_recipe"]["recipe_display_type_id"], 3);
    assert_eq!(
        ui["last_ghost_recipe"]["recipe_display_type"],
        "stonecutter"
    );
    assert_eq!(ui["last_ghost_recipe"]["recipe_display_body_len"], 7);
    assert_eq!(ui["current_dialog"]["holder_kind"], "reference");
    assert_eq!(ui["current_dialog"]["registry_id"], 7);
    assert_eq!(ui["last_pong_response"]["time"], 123456789);
}

#[test]
fn client_audio_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_sound_event(SoundEvent {
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
        });
        store.record_local_sound(bbb_world::LocalSoundEventState {
            sound: bbb_world::SoundHolderState {
                kind: "direct".to_string(),
                registry_id: None,
                location: Some("minecraft:block.portal.travel".to_string()),
                fixed_range: None,
            },
            source: "ambient".to_string(),
            volume: 0.25,
            pitch: 1.0,
            seed: 0,
        });
        store.apply_level_event(LevelEvent {
            event_type: 1010,
            pos: bbb_protocol::packets::BlockPos { x: 4, y: 65, z: -7 },
            data: 27,
            global: false,
        });
        store.apply_stop_sound(StopSound {
            source: Some(SoundSource::Music),
            name: Some("minecraft:music.menu".to_string()),
        });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_audio".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let audio = response.result.unwrap();
    assert_eq!(audio["last_sound"]["sound"]["kind"], "reference");
    assert_eq!(audio["last_sound"]["sound"]["registry_id"], 41);
    assert_eq!(audio["last_sound"]["position"]["x"], 2.5);
    assert_eq!(audio["last_sound"]["position"]["y"], -1.0);
    assert_eq!(audio["last_sound"]["position"]["z"], 0.0);
    assert_eq!(
        audio["last_local_sound"]["sound"]["location"],
        "minecraft:block.portal.travel"
    );
    assert_eq!(audio["last_local_sound"]["source"], "ambient");
    assert_eq!(audio["last_local_sound"]["volume"], 0.25);
    assert_eq!(audio["playing_jukebox_songs"][0]["song_registry_id"], 27);
    assert_eq!(audio["playing_jukebox_songs"][0]["pos"]["x"], 4);
    assert_eq!(audio["last_jukebox_event"]["action"], "start");
    assert_eq!(audio["last_jukebox_event"]["song_registry_id"], 27);
    assert_eq!(audio["last_stop_sound"]["source"], "music");
    assert_eq!(audio["last_stop_sound"]["name"], "minecraft:music.menu");
}

#[test]
fn client_debug_game_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_debug_block_value(DebugBlockValue {
            pos: ProtocolBlockPos { x: 4, y: 65, z: -9 },
            raw_update_payload: vec![0xaa, 0xbb, 0xcc],
        });
        store.apply_game_rule_values(GameRuleValues {
            values: vec![
                GameRuleValue {
                    rule: "minecraft:do_daylight_cycle".to_string(),
                    value: "false".to_string(),
                },
                GameRuleValue {
                    rule: "minecraft:random_tick_speed".to_string(),
                    value: "12".to_string(),
                },
            ],
        });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_debug_game".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let debug_game = response.result.unwrap();
    assert_eq!(debug_game["last_debug_block_value"]["pos"]["x"], 4);
    assert_eq!(debug_game["last_debug_block_value"]["pos"]["y"], 65);
    assert_eq!(debug_game["last_debug_block_value"]["pos"]["z"], -9);
    assert_eq!(
        debug_game["last_debug_block_value"]["raw_update_payload_len"],
        3
    );
    assert_eq!(
        debug_game["last_game_rule_values"]["values"],
        json!([
            {
                "rule": "minecraft:do_daylight_cycle",
                "value": "false"
            },
            {
                "rule": "minecraft:random_tick_speed",
                "value": "12"
            }
        ])
    );
}

#[test]
fn client_debug_query_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_tag_query(TagQuery {
            transaction_id: 12,
            tag_present: true,
            raw_nbt: vec![10, 0],
        });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_debug_query".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let debug_query = response.result.unwrap();
    assert_eq!(debug_query["last_tag_query"]["transaction_id"], 12);
    assert_eq!(debug_query["last_tag_query"]["tag_present"], true);
    assert_eq!(debug_query["last_tag_query"]["raw_nbt"], json!([10, 0]));
}

#[test]
fn client_command_suggestions_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_custom_chat_completions(CustomChatCompletions {
            action: CustomChatCompletionsAction::Set,
            entries: vec!["/warp".to_string(), "/spawn".to_string()],
        });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_command_suggestions".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let suggestions = response.result.unwrap();
    assert_eq!(
        suggestions["custom_completions"],
        json!(["/spawn", "/warp"])
    );
    assert_eq!(
        suggestions["last_custom_completion_update"]["action"],
        "set"
    );
    assert_eq!(suggestions["last_custom_completion_update"]["entries"], 2);
}

#[test]
fn client_features_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_update_enabled_features(UpdateEnabledFeatures {
            features: vec![
                "minecraft:unknown".to_string(),
                "minecraft:vanilla".to_string(),
                "minecraft:trade_rebalance".to_string(),
            ],
        });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_features".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(
        response.result.unwrap(),
        json!(["minecraft:trade_rebalance", "minecraft:vanilla"])
    );
}

#[test]
fn client_known_packs_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_select_known_packs(
            vec![KnownPack {
                namespace: "minecraft".to_string(),
                id: "core".to_string(),
                version: "26.1".to_string(),
            }],
            Vec::new(),
        );
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_known_packs".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let known_packs = response.result.unwrap();
    assert_eq!(known_packs["offered"][0]["namespace"], "minecraft");
    assert_eq!(known_packs["offered"][0]["id"], "core");
    assert_eq!(known_packs["offered"][0]["version"], "26.1");
    assert_eq!(known_packs["selected"], json!([]));
}

#[test]
fn apply_diagnostics_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.record_apply_error("light_update", "invalid light payload");
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.apply_diagnostics".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let diagnostics = response.result.unwrap();
    assert_eq!(diagnostics["apply_errors"][0]["source"], "light_update");
    assert_eq!(
        diagnostics["apply_errors"][0]["message"],
        "invalid light payload"
    );
    assert_eq!(
        diagnostics["last_apply_error"]["message"],
        "invalid light payload"
    );
}

#[test]
fn command_tree_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_commands(command_tree("say"));
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.command_tree".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let commands = response.result.unwrap();
    assert_eq!(commands["root_index"], 0);
    assert_eq!(commands["nodes"][1]["name"], "say");
    assert_eq!(commands["nodes"][2]["parser"]["name"], "brigadier:string");
    assert_eq!(commands["nodes"][2]["suggestions"], "minecraft:ask_server");
    assert_eq!(commands["nodes"][2]["executable"], true);
    assert_eq!(commands["nodes"][2]["restricted"], true);
}

#[test]
fn last_block_changed_ack_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_block_changed_ack(BlockChangedAck { sequence: 17 });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.last_block_changed_ack".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["sequence"], 17);
}

#[test]
fn client_block_events_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        assert!(store.apply_block_destruction(BlockDestruction {
            id: 7,
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5
            },
            progress: 3,
        }));
        store.apply_block_event(BlockEvent {
            pos: ProtocolBlockPos {
                x: 13,
                y: 65,
                z: -6,
            },
            b0: 1,
            b1: 5,
            block_id: 123,
        });
        store.apply_level_event(LevelEvent {
            event_type: 2001,
            pos: ProtocolBlockPos {
                x: 14,
                y: 66,
                z: -7,
            },
            data: 9,
            global: true,
        });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_block_events".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let events = response.result.unwrap();
    assert_eq!(events["destructions"][0]["id"], 7);
    assert_eq!(events["destructions"][0]["pos"]["x"], 12);
    assert_eq!(events["destructions"][0]["pos"]["z"], -5);
    assert_eq!(events["destructions"][0]["progress"], 3);
    assert_eq!(events["block_events"][0]["pos"]["x"], 13);
    assert_eq!(events["block_events"][0]["b0"], 1);
    assert_eq!(events["block_events"][0]["b1"], 5);
    assert_eq!(events["block_events"][0]["block_id"], 123);
    assert_eq!(events["level_events"][0]["event_type"], 2001);
    assert_eq!(events["level_events"][0]["pos"]["y"], 66);
    assert_eq!(events["level_events"][0]["data"], 9);
    assert_eq!(events["level_events"][0]["global"], true);
}

#[test]
fn world_border_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_initialize_border(InitializeBorder {
            new_center_x: 10.5,
            new_center_z: -20.25,
            old_size: 100.0,
            new_size: 200.0,
            lerp_time: 0,
            new_absolute_max_size: 400,
            warning_blocks: 6,
            warning_time: 7,
        });
        store.apply_set_border_center(SetBorderCenter {
            new_center_x: 30.0,
            new_center_z: -40.0,
        });
        store.apply_set_border_lerp_size(SetBorderLerpSize {
            old_size: 200.0,
            new_size: 300.0,
            lerp_time: 50,
        });
        store.apply_set_border_warning_delay(SetBorderWarningDelay { warning_delay: 11 });
        store.apply_set_border_warning_distance(SetBorderWarningDistance { warning_blocks: 12 });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.world_border".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let border = response.result.unwrap();
    assert_eq!(border["center_x"], 30.0);
    assert_eq!(border["center_z"], -40.0);
    assert_eq!(border["size"], 200.0);
    assert_eq!(border["lerp_target"], 300.0);
    assert_eq!(border["lerp_time"], 50);
    assert_eq!(border["absolute_max_size"], 400);
    assert_eq!(border["warning_blocks"], 12);
    assert_eq!(border["warning_time"], 11);
}

#[test]
fn client_advancements_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_update_advancements(UpdateAdvancements {
            reset: true,
            added: vec![AdvancementSummary {
                id: "minecraft:story/root".to_string(),
                parent: None,
                display: None,
                requirements: Vec::new(),
                sends_telemetry_event: false,
            }],
            removed: Vec::new(),
            progress: Vec::new(),
            show_advancements: false,
        });
        store.apply_select_advancements_tab(SelectAdvancementsTab {
            tab: Some("minecraft:story/root".to_string()),
        });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_advancements".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let advancements = response.result.unwrap();
    assert_eq!(advancements["selected_tab"], "minecraft:story/root");
    assert!(advancements["advancements"]
        .as_object()
        .unwrap()
        .contains_key("minecraft:story/root"));
}

#[test]
fn client_local_player_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_player_abilities(PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        store.apply_player_health(PlayerHealth {
            health: 7.5,
            food: 16,
            saturation: 2.0,
        });
        store.apply_player_experience(PlayerExperience {
            progress: 0.75,
            level: 8,
            total: 123,
        });
        store.apply_default_spawn_position(SetDefaultSpawnPosition {
            dimension: "minecraft:overworld".to_string(),
            pos: ProtocolBlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            yaw: 90.0,
            pitch: -10.0,
        });
        store.apply_simulation_distance(SetSimulationDistance { distance: 12 });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_local_player".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let local = response.result.unwrap();
    assert_eq!(local["abilities"]["can_fly"], true);
    assert_eq!(local["health"]["food"], 16);
    assert_eq!(local["experience"]["total"], 123);
    assert_eq!(local["default_spawn"]["pos"]["x"], -5);
    assert_eq!(local["default_spawn"]["yaw"], 90.0);
    assert_eq!(local["simulation_distance"], 12);
}

#[test]
fn client_player_info_reads_canonical_world_state() {
    let state = ControlState::new("test");
    let profile_id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
    {
        let mut store = WorldStore::new();
        let applied = store.apply_player_info_update(PlayerInfoUpdate {
            actions: vec![
                PlayerInfoAction::AddPlayer,
                PlayerInfoAction::UpdateGameMode,
                PlayerInfoAction::UpdateListed,
                PlayerInfoAction::UpdateLatency,
                PlayerInfoAction::UpdateDisplayName,
                PlayerInfoAction::UpdateHat,
                PlayerInfoAction::UpdateListOrder,
            ],
            entries: vec![PlayerInfoEntry {
                profile_id,
                profile: Some(GameProfile {
                    uuid: profile_id,
                    name: "Ada".to_string(),
                    properties: vec![GameProfileProperty {
                        name: "textures".to_string(),
                        value: "skin-payload".to_string(),
                        signature: Some("skin-signature".to_string()),
                    }],
                }),
                listed: true,
                latency: 42,
                game_mode: GameType::Creative,
                display_name: Some("{\"text\":\"Ada Lovelace\"}".to_string()),
                show_hat: true,
                list_order: 7,
                chat_session: None,
            }],
        });
        assert_eq!(applied, 1);
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_player_info".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let player_info = response.result.unwrap();
    let key = profile_id.to_string();
    let entry = &player_info["entries"][&key];
    assert_eq!(entry["profile"]["uuid"], key);
    assert_eq!(entry["profile"]["name"], "Ada");
    assert_eq!(entry["profile"]["properties"][0]["name"], "textures");
    assert_eq!(entry["listed"], true);
    assert_eq!(entry["latency"], 42);
    assert_eq!(entry["game_mode"], "creative");
    assert_eq!(entry["display_name"], "{\"text\":\"Ada Lovelace\"}");
    assert_eq!(entry["show_hat"], true);
    assert_eq!(entry["list_order"], 7);
    assert_eq!(entry["chat_session_present"], false);
    assert_eq!(
        player_info["listed_players"],
        json!([profile_id.to_string()])
    );
}

#[test]
fn client_scoreboard_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        assert!(store.apply_set_objective(SetObjective {
            objective_name: "kills".to_string(),
            method: SetObjectiveMethod::Add,
            parameters: Some(SetObjectiveParameters {
                display_name: "Kills".to_string(),
                render_type: ObjectiveRenderType::Hearts,
                number_format: Some(vec![1, 2, 3]),
            }),
        }));
        assert!(store.apply_set_display_objective(SetDisplayObjective {
            slot: ScoreboardDisplaySlot::Sidebar,
            objective_name: Some("kills".to_string()),
        }));
        assert!(store.apply_set_score(SetScore {
            owner: "Alice".to_string(),
            objective_name: "kills".to_string(),
            score: 4,
            display: Some("Alice".to_string()),
            number_format: Some(vec![9]),
        }));
        assert!(store.apply_set_player_team(SetPlayerTeam {
            name: "red".to_string(),
            method: PlayerTeamMethod::Add,
            parameters: Some(PlayerTeamParameters {
                display_name: "Red Team".to_string(),
                options: 3,
                nametag_visibility: TeamVisibility::Always,
                collision_rule: TeamCollisionRule::PushOtherTeams,
                color: ChatFormatting::Red,
                player_prefix: "[R] ".to_string(),
                player_suffix: "!".to_string(),
            }),
            players: vec!["Alice".to_string(), "Bob".to_string()],
        }));
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_scoreboard".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let scoreboard = response.result.unwrap();
    assert_eq!(scoreboard["objectives"]["kills"]["display_name"], "Kills");
    assert_eq!(scoreboard["objectives"]["kills"]["render_type"], "hearts");
    assert_eq!(
        scoreboard["objectives"]["kills"]["number_format"],
        json!([1, 2, 3])
    );
    assert_eq!(scoreboard["display_slots"]["sidebar"], "kills");
    assert_eq!(scoreboard["scores"]["Alice"]["kills"]["value"], 4);
    assert_eq!(scoreboard["scores"]["Alice"]["kills"]["display"], "Alice");
    assert_eq!(
        scoreboard["scores"]["Alice"]["kills"]["number_format"],
        json!([9])
    );
    assert_eq!(
        scoreboard["teams"]["red"]["parameters"]["display_name"],
        "Red Team"
    );
    assert_eq!(scoreboard["teams"]["red"]["parameters"]["color"], "red");
    assert_eq!(
        scoreboard["teams"]["red"]["parameters"]["collision_rule"],
        "pushOtherTeams"
    );
    assert_eq!(
        scoreboard["teams"]["red"]["players"],
        json!(["Alice", "Bob"])
    );
}

#[test]
fn client_maps_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        assert!(store.apply_map_item_data(MapItemData {
            map_id: 42,
            scale: 2,
            locked: true,
            decorations: Some(vec![MapDecoration {
                type_id: 4,
                x: -20,
                y: 30,
                rot: 7,
                name: Some("Village".to_string()),
            }]),
            color_patch: Some(MapColorPatch {
                start_x: 3,
                start_y: 4,
                width: 2,
                height: 2,
                colors: vec![1, 2, 3, 4],
            }),
        }));
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_maps".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let maps = response.result.unwrap();
    let map = &maps["42"];
    assert_eq!(map["id"], 42);
    assert_eq!(map["scale"], 2);
    assert_eq!(map["locked"], true);
    assert_eq!(map["decorations"][0]["type_id"], 4);
    assert_eq!(map["decorations"][0]["x"], -20);
    assert_eq!(map["decorations"][0]["name"], "Village");
    assert_eq!(map["last_color_patch"]["start_x"], 3);
    assert_eq!(map["last_color_patch"]["height"], 2);
    let colors = map["colors"].as_array().unwrap();
    assert_eq!(colors[3 + 4 * 128], 1);
    assert_eq!(colors[4 + 4 * 128], 2);
    assert_eq!(colors[3 + 5 * 128], 3);
    assert_eq!(colors[4 + 5 * 128], 4);
}

#[test]
fn last_map_color_patch_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_map_item_data(MapItemData {
            map_id: 42,
            scale: 2,
            locked: true,
            decorations: Some(vec![MapDecoration {
                type_id: 4,
                x: -20,
                y: 30,
                rot: 7,
                name: Some("Village".to_string()),
            }]),
            color_patch: Some(MapColorPatch {
                start_x: 3,
                start_y: 4,
                width: 2,
                height: 2,
                colors: vec![1, 2, 3, 4],
            }),
        });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.last_map_color_patch".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let patch = response.result.unwrap();
    assert_eq!(patch["map_id"], 42);
    assert_eq!(patch["start_x"], 3);
    assert_eq!(patch["start_y"], 4);
    assert_eq!(patch["width"], 2);
    assert_eq!(patch["height"], 2);
}

#[test]
fn level_clock_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_world_time(PlayTime {
            game_time: 123,
            clock_updates: vec![bbb_protocol::packets::ClockUpdate {
                clock_id: 0,
                total_ticks: 6000,
                partial_tick: 0.0,
                rate: 1.0,
            }],
        });
        store.apply_game_event(GameEvent {
            event_id: 7,
            param: 0.5,
        });
        store.apply_ticking_state(TickingState {
            tick_rate: 0.25,
            frozen: true,
        });
        store.apply_ticking_step(TickingStep { tick_steps: 7 });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.level_clock".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let clock = response.result.unwrap();
    assert_eq!(clock["world_time"]["game_time"], 123);
    assert_eq!(clock["world_time"]["day_time"], 6000);
    assert_eq!(clock["world_time"]["clock_updates"][0]["total_ticks"], 6000);
    assert_eq!(clock["weather"]["raining"], true);
    assert_eq!(clock["weather"]["rain_level"], 0.5);
    assert_eq!(clock["ticking"]["tick_rate"], 1.0);
    assert_eq!(clock["ticking"]["frozen"], true);
    assert_eq!(clock["ticking"]["frozen_ticks_to_run"], 7);
}

#[test]
fn server_presentation_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_transfer(Transfer {
            host: "next.example.com".to_string(),
            port: 25566,
        });
        store.apply_custom_report_details(CustomReportDetails {
            details: BTreeMap::from([
                ("Region".to_string(), "local".to_string()),
                ("Server".to_string(), "bbb test shard".to_string()),
            ]),
        });
        store.apply_store_cookie("bbb:session", 3, 1);
        store.apply_custom_payload(CustomPayload {
            id: "minecraft:brand".to_string(),
            payload: CustomPayloadBody::Brand {
                brand: "vanilla".to_string(),
            },
        });
        store.apply_server_links(ServerLinks {
            links: vec![ServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::Support),
                url: "https://example.invalid/support".to_string(),
            }],
        });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.server_presentation".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let presentation = response.result.unwrap();
    assert_eq!(presentation["last_transfer"]["host"], "next.example.com");
    assert_eq!(presentation["last_transfer"]["port"], 25566);
    assert_eq!(presentation["custom_report_details"]["Region"], "local");
    assert_eq!(
        presentation["custom_report_details"]["Server"],
        "bbb test shard"
    );
    assert_eq!(presentation["server_cookies"]["last_key"], "bbb:session");
    assert_eq!(presentation["server_cookies"]["stored_count"], 1);
    assert_eq!(presentation["server_brand"], "vanilla");
    assert_eq!(presentation["last_custom_payload"]["id"], "minecraft:brand");
    assert_eq!(presentation["last_custom_payload"]["kind"], "brand");
    assert_eq!(presentation["last_custom_payload"]["brand"], "vanilla");
    assert_eq!(
        presentation["server_links"][0]["label"],
        "known_server_link.support"
    );
    assert_eq!(
        presentation["server_links"][0]["url"],
        "https://example.invalid/support"
    );
    assert_eq!(presentation["server_links"][0]["known_type"], "support");
}

#[test]
fn probes_chunk_and_block_from_world_store() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        store.insert_decoded_chunk(single_section_chunk());

        *state.world.write().unwrap() = store;
    }

    let chunk_response = dispatch(
        ControlRequest {
            method: "world.probe_chunk".to_string(),
            params: json!({"x": 1, "z": -2}),
        },
        &state,
    );
    assert!(chunk_response.ok);
    let chunk = chunk_response.result.unwrap();
    assert_eq!(chunk["state"], "Decoded");
    assert_eq!(chunk["sections"], 1);
    assert_eq!(chunk["heightmaps"], 1);

    let block_response = dispatch(
        ControlRequest {
            method: "world.probe_block".to_string(),
            params: json!({"x": 17, "y": 0, "z": -31}),
        },
        &state,
    );
    assert!(block_response.ok);
    let block = block_response.result.unwrap();
    assert_eq!(block["block_state_id"], 42);
    assert_eq!(block["block_name"], "minecraft:dark_oak_sapling");
    assert_eq!(block["block_properties"]["stage"], "1");
    assert_eq!(block["block_palette_kind"], "SingleValue");
    assert_eq!(block["biome_id"], 3);

    let terrain_response = dispatch(
        ControlRequest {
            method: "world.terrain_chunk_summary".to_string(),
            params: json!({"x": 1, "z": -2}),
        },
        &state,
    );
    assert!(terrain_response.ok);
    let terrain = terrain_response.result.unwrap();
    assert_eq!(terrain["total_blocks"], 4096);
    assert_eq!(terrain["opaque_blocks"], 0);
    assert_eq!(terrain["cutout_blocks"], 4096);
    assert_eq!(terrain["empty_blocks"], 0);

    let missing_response = dispatch(
        ControlRequest {
            method: "world.probe_block".to_string(),
            params: json!({"x": 17, "y": 16, "z": -31}),
        },
        &state,
    );
    assert!(missing_response.ok);
    assert!(missing_response.result.unwrap().is_null());
}

#[test]
fn chunk_view_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.insert_decoded_chunk(single_section_chunk());
        store.apply_set_chunk_cache_center(SetChunkCacheCenter {
            chunk_x: -4,
            chunk_z: 7,
        });
        store.apply_set_chunk_cache_radius(SetChunkCacheRadius { radius: 10 });
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.chunk_view".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let view = response.result.unwrap();
    assert_eq!(view["first_chunk"]["x"], 1);
    assert_eq!(view["first_chunk"]["z"], -2);
    assert_eq!(view["center"]["x"], -4);
    assert_eq!(view["center"]["z"], 7);
    assert_eq!(view["radius"], 10);
}

#[test]
fn probes_entity_transforms_from_world_store_components() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(7, 3));
        store.apply_add_entity(protocol_add_entity(9, 85));
        assert!(
            store.apply_entity_position_sync(ProtocolEntityPositionSync {
                id: 9,
                position: ProtocolVec3d {
                    x: 5.0,
                    y: 70.0,
                    z: -8.0,
                },
                delta_movement: ProtocolVec3d {
                    x: 0.1,
                    y: 0.2,
                    z: 0.3,
                },
                y_rot: 45.0,
                x_rot: -15.0,
                on_ground: true,
            })
        );

        *state.world.write().unwrap() = store;
    }

    let one_response = dispatch(
        ControlRequest {
            method: "world.probe_entity_transform".to_string(),
            params: json!({"id": 9}),
        },
        &state,
    );
    assert!(one_response.ok);
    let one = one_response.result.unwrap();
    assert_eq!(one["id"], 9);
    assert_eq!(one["entity_type_id"], 85);
    assert_eq!(one["position"]["x"], 5.0);
    assert_eq!(one["delta_movement"]["z"], 0.3);
    assert_eq!(one["y_rot"], 45.0);
    assert_eq!(one["x_rot"], -15.0);
    assert_eq!(one["on_ground"], true);

    let all_response = dispatch(
        ControlRequest {
            method: "world.entity_transforms".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );
    assert!(all_response.ok);
    let all = all_response.result.unwrap();
    let ids: Vec<i64> = all
        .as_array()
        .unwrap()
        .iter()
        .map(|entity| entity["id"].as_i64().unwrap())
        .collect();
    assert_eq!(ids, vec![7, 9]);

    let missing_response = dispatch(
        ControlRequest {
            method: "world.probe_entity_transform".to_string(),
            params: json!({"id": 999}),
        },
        &state,
    );
    assert!(missing_response.ok);
    assert!(missing_response.result.unwrap().is_null());
}

#[test]
fn last_projectile_power_reads_canonical_world_state() {
    let state = ControlState::new("test");

    let empty_response = dispatch(
        ControlRequest {
            method: "world.last_projectile_power".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );
    assert!(empty_response.ok);
    assert!(empty_response.result.unwrap().is_null());

    {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(10, 52));
        store.apply_add_entity(protocol_add_entity(20, 7));
        assert!(store.apply_projectile_power(ProjectilePower {
            entity_id: 10,
            acceleration_power: 0.75,
        }));
        *state.world.write().unwrap() = store;
    }

    let applied_response = dispatch(
        ControlRequest {
            method: "world.last_projectile_power".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );
    assert!(applied_response.ok);
    let applied = applied_response.result.unwrap();
    assert_eq!(applied["entity_id"], 10);
    assert_eq!(applied["acceleration_power"], 0.75);
    assert_eq!(applied["applied"], true);

    assert!(!state
        .world
        .write()
        .unwrap()
        .apply_projectile_power(ProjectilePower {
            entity_id: 20,
            acceleration_power: 0.25,
        }));

    let ignored_response = dispatch(
        ControlRequest {
            method: "world.last_projectile_power".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );
    assert!(ignored_response.ok);
    let ignored = ignored_response.result.unwrap();
    assert_eq!(ignored["entity_id"], 20);
    assert_eq!(ignored["acceleration_power"], 0.25);
    assert_eq!(ignored["applied"], false);
}

#[test]
fn probe_entity_status_reads_canonical_world_state() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(7, 7));
        assert!(store.apply_entity_event(ProtocolEntityEvent {
            entity_id: 7,
            event_id: 35,
        }));
        assert!(store.apply_hurt_animation(ProtocolHurtAnimation { id: 7, yaw: 45.5 }));
        assert!(store.apply_damage_event(ProtocolDamageEvent {
            entity_id: 7,
            source_type_id: 5,
            source_cause_id: -1,
            source_direct_id: 42,
            source_position: Some(ProtocolVec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            }),
        }));
        assert!(store.apply_update_mob_effect(UpdateMobEffect {
            entity_id: 7,
            effect_id: 3,
            amplifier: 2,
            duration_ticks: 400,
            flags: MobEffectFlags {
                raw: 0b1011,
                ambient: true,
                visible: true,
                show_icon: false,
                blend: true,
            },
        }));
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.probe_entity_status".to_string(),
            params: json!({"id": 7}),
        },
        &state,
    );

    assert!(response.ok);
    let status = response.result.unwrap();
    assert_eq!(status["id"], 7);
    assert_eq!(status["entity_type_id"], 7);
    assert_eq!(status["last_event_id"], 35);
    assert_eq!(status["last_hurt_yaw"], 45.5);
    assert_eq!(status["mob_effects"]["3"]["effect_id"], 3);
    assert_eq!(status["mob_effects"]["3"]["amplifier"], 2);
    assert_eq!(status["mob_effects"]["3"]["duration_ticks"], 400);
    assert_eq!(status["mob_effects"]["3"]["ambient"], true);
    assert_eq!(status["mob_effects"]["3"]["visible"], true);
    assert_eq!(status["mob_effects"]["3"]["show_icon"], false);
    assert_eq!(status["mob_effects"]["3"]["blend"], true);
    assert_eq!(status["last_damage"]["source_type_id"], 5);
    assert_eq!(status["last_damage"]["source_cause_id"], -1);
    assert_eq!(status["last_damage"]["source_direct_id"], 42);
    assert_eq!(status["last_damage"]["source_position"]["x"], 1.0);
    assert_eq!(status["last_damage"]["source_position"]["y"], 2.0);
    assert_eq!(status["last_damage"]["source_position"]["z"], 3.0);

    let missing_response = dispatch(
        ControlRequest {
            method: "world.probe_entity_status".to_string(),
            params: json!({"id": 999}),
        },
        &state,
    );
    assert!(missing_response.ok);
    assert!(missing_response.result.unwrap().is_null());
}

#[test]
fn entity_pick_targets_probe_exposes_ender_dragon_part_targets() {
    let state = ControlState::new("test");
    {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(100, 43));
        store.advance_entity_client_animations(1);
        *state.world.write().unwrap() = store;
    }

    let response = dispatch(
        ControlRequest {
            method: "world.entity_pick_targets".to_string(),
            params: json!({"partial_tick": 1.0}),
        },
        &state,
    );

    assert!(response.ok);
    let targets = response.result.unwrap();
    let targets = targets.as_array().unwrap();
    let ids = targets
        .iter()
        .map(|target| target["entity_id"].as_i64().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![101, 102, 103, 104, 105, 106, 107, 108]);
    assert!(!ids.contains(&100));

    let head = &targets[0];
    assert!(head["position"]["x"].is_number());
    assert!(head["position"]["y"].is_number());
    assert!(head["position"]["z"].is_number());
    assert_eq!(head["bounds"]["max"][0], 0.5);
    assert_eq!(head["bounds"]["max"][1], 1.0);

    let wing = &targets[6];
    assert_eq!(wing["entity_id"], 107);
    assert_eq!(wing["bounds"]["min"][0], -2.0);
    assert_eq!(wing["bounds"]["max"][0], 2.0);
    assert_eq!(wing["bounds"]["max"][1], 2.0);
}

fn single_section_chunk() -> ChunkColumn {
    ChunkColumn {
        pos: ChunkPos { x: 1, z: -2 },
        state: ChunkState::Decoded,
        heightmaps: vec![HeightmapData {
            kind_id: 1,
            data: vec![0],
        }],
        sections: vec![ChunkSection {
            non_empty_block_count: 1,
            fluid_count: 0,
            block_states: single_value_container(PaletteDomain::BlockStates, 42, 4096),
            biomes: single_value_container(PaletteDomain::Biomes, 3, 64),
        }],
        block_entities: Vec::<BlockEntityRecord>::new(),
        light: LightData::default(),
    }
}

fn single_value_container(
    domain: PaletteDomain,
    global_id: i32,
    entry_count: usize,
) -> PalettedContainerData {
    PalettedContainerData {
        domain,
        bits_per_entry: 0,
        palette_kind: PaletteKind::SingleValue,
        palette_global_ids: vec![global_id],
        packed_data: Vec::new(),
        entry_count,
    }
}

fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn item_cost(item_id: i32, count: i32) -> ItemCostSummary {
    ItemCostSummary {
        item_id,
        count,
        component_predicate: Default::default(),
    }
}

fn recipe_book_entry(id: i32, notification: bool, highlight: bool) -> RecipeBookAddEntry {
    RecipeBookAddEntry {
        contents: RecipeDisplayEntry {
            id: RecipeDisplayId { index: id },
            display: RecipeDisplaySummary {
                display_type: RecipeDisplayType::Stonecutter,
                raw_body: vec![3, 0, 0, 0],
                crafting: None,
            },
            group: None,
            category_id: 10,
            crafting_requirements: Some(vec![IngredientSummary {
                tag: None,
                item_ids: vec![42],
            }]),
        },
        flags: u8::from(notification) | (u8::from(highlight) << 1),
        notification,
        highlight,
    }
}

fn command_tree(literal: &str) -> Commands {
    Commands {
        root_index: 0,
        nodes: vec![
            CommandNode {
                node_type: CommandNodeType::Root,
                flags: 0,
                children: vec![1],
                redirect: None,
                name: None,
                parser: None,
                suggestions: None,
                executable: false,
                restricted: false,
            },
            CommandNode {
                node_type: CommandNodeType::Literal,
                flags: 1,
                children: vec![2],
                redirect: None,
                name: Some(literal.to_string()),
                parser: None,
                suggestions: None,
                executable: false,
                restricted: false,
            },
            CommandNode {
                node_type: CommandNodeType::Argument,
                flags: 2,
                children: Vec::new(),
                redirect: None,
                name: Some("message".to_string()),
                parser: Some(CommandArgumentParser {
                    type_id: 5,
                    name: "brigadier:string".to_string(),
                    properties: vec![1, 2],
                }),
                suggestions: Some("minecraft:ask_server".to_string()),
                executable: true,
                restricted: true,
            },
        ],
    }
}

fn protocol_add_entity(id: i32, entity_type_id: i32) -> ProtocolAddEntity {
    ProtocolAddEntity {
        id,
        uuid: uuid::Uuid::from_u128(id as u128 + 1),
        entity_type_id,
        position: ProtocolVec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: -10.0,
        y_rot: 20.0,
        y_head_rot: 30.0,
        data: 99,
    }
}
