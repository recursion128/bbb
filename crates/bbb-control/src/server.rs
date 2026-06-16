use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use anyhow::{Context, Result};
use bbb_world::{BlockPos, ChunkColumn, ChunkPos};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

use crate::types::{
    AppStatus, CodeOfConductControlRequest, ContainerClickControlRequest, ControlRequest,
    ControlResponse, ControlSnapshot, NetControlRequest, SharedSnapshot,
};

pub fn shared_snapshot(version: impl Into<String>) -> SharedSnapshot {
    Arc::new(RwLock::new(ControlSnapshot {
        app: AppStatus {
            version: version.into(),
            running: true,
        },
        ..ControlSnapshot::default()
    }))
}

pub async fn serve(addr: SocketAddr, snapshot: SharedSnapshot) -> Result<()> {
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind control API on {addr}"))?;
    tracing::info!(%addr, "native control API listening");

    loop {
        let (stream, _) = listener.accept().await?;
        let snapshot = Arc::clone(&snapshot);
        tokio::spawn(async move {
            if let Err(err) = handle_client(stream, snapshot).await {
                tracing::debug!(?err, "control client failed");
            }
        });
    }
}

async fn handle_client(stream: TcpStream, snapshot: SharedSnapshot) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await? {
        let response = match serde_json::from_str::<ControlRequest>(&line) {
            Ok(request) => dispatch(request, &snapshot),
            Err(err) => ControlResponse {
                ok: false,
                result: None,
                error: Some(err.to_string()),
            },
        };
        writer
            .write_all(serde_json::to_string(&response)?.as_bytes())
            .await?;
        writer.write_all(b"\n").await?;
    }
    Ok(())
}

fn dispatch(request: ControlRequest, snapshot: &SharedSnapshot) -> ControlResponse {
    if request.method == "app.quit" {
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard.app.running = false;
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({"running": false})),
            error: None,
        };
    }

    if request.method == "renderer.screenshot" {
        let Some(path) = string_param(&request.params, "path") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("renderer.screenshot requires string param path".to_string()),
            };
        };
        let path = path.to_string();
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard.screenshot_request = Some(path.clone());
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({"queued": true, "path": path})),
            error: None,
        };
    }

    if request.method == "net.accept_code_of_conduct" {
        let remember = bool_param(&request.params, "remember").unwrap_or(false);
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::Accept { remember });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "remember": remember,
                "pending": snapshot_guard.code_of_conduct_requests.len()
            })),
            error: None,
        };
    }

    if request.method == "net.decline_code_of_conduct" {
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::Decline);
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.code_of_conduct_requests.len()
            })),
            error: None,
        };
    }

    if request.method == "net.clear_code_of_conduct_acceptance" {
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::ClearAcceptance);
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.code_of_conduct_requests.len()
            })),
            error: None,
        };
    }

    if request.method == "net.send_chat_command" {
        let Some(command) = non_empty_string_param(&request.params, "command") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.send_chat_command requires non-empty string param command".to_string(),
                ),
            };
        };
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::ChatCommand {
                command: command.to_string(),
            });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
            })),
            error: None,
        };
    }

    if request.method == "net.request_command_suggestions" {
        let Some(id) = i32_param(&request.params, "id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.request_command_suggestions requires integer param id".to_string(),
                ),
            };
        };
        let Some(command) = non_empty_string_param(&request.params, "command") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.request_command_suggestions requires non-empty string param command"
                        .to_string(),
                ),
            };
        };
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::CommandSuggestionRequest {
                id,
                command: command.to_string(),
            });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
            })),
            error: None,
        };
    }

    if request.method == "net.container_button_click" {
        let Some(container_id) = i32_param(&request.params, "container_id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.container_button_click requires integer param container_id".to_string(),
                ),
            };
        };
        let Some(button_id) = i32_param(&request.params, "button_id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.container_button_click requires integer param button_id".to_string(),
                ),
            };
        };
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::ContainerButtonClick {
                container_id,
                button_id,
            });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
            })),
            error: None,
        };
    }

    if request.method == "net.container_click" {
        let click =
            match serde_json::from_value::<ContainerClickControlRequest>(request.params.clone()) {
                Ok(click) => click,
                Err(err) => {
                    return ControlResponse {
                        ok: false,
                        result: None,
                        error: Some(format!("net.container_click invalid params: {err}")),
                    };
                }
            };
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::ContainerClick(click));
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
            })),
            error: None,
        };
    }

    if request.method == "net.container_close" {
        let Some(container_id) = i32_param(&request.params, "container_id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.container_close requires integer param container_id".to_string()),
            };
        };
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::ContainerClose { container_id });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
            })),
            error: None,
        };
    }

    if request.method == "net.container_slot_state_changed" {
        let Some(slot_id) = i32_param(&request.params, "slot_id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.container_slot_state_changed requires integer param slot_id".to_string(),
                ),
            };
        };
        let Some(container_id) = i32_param(&request.params, "container_id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.container_slot_state_changed requires integer param container_id"
                        .to_string(),
                ),
            };
        };
        let Some(new_state) = bool_param(&request.params, "new_state") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.container_slot_state_changed requires boolean param new_state".to_string(),
                ),
            };
        };
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::ContainerSlotStateChanged {
                slot_id,
                container_id,
                new_state,
            });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
            })),
            error: None,
        };
    }

    let snapshot_guard = snapshot.read().expect("control snapshot poisoned");
    let json = match request.method.as_str() {
        "app.status" => Ok(serde_json::json!({
            "app": &snapshot_guard.app,
            "net": &snapshot_guard.net,
            "renderer": &snapshot_guard.renderer,
            "world": snapshot_guard.world_store.counters(),
        })),
        "net.counters" => serde_json::to_value(&snapshot_guard.net),
        "renderer.counters" => serde_json::to_value(&snapshot_guard.renderer),
        "world.counters" => serde_json::to_value(snapshot_guard.world_store.counters()),
        "world.client_advancements" => {
            serde_json::to_value(snapshot_guard.world_store.client_advancements())
        }
        "world.client_audio" => serde_json::to_value(snapshot_guard.world_store.client_audio()),
        "world.client_chat" => serde_json::to_value(snapshot_guard.world_store.client_chat()),
        "world.client_combat" => serde_json::to_value(snapshot_guard.world_store.client_combat()),
        "world.client_command_suggestions" => {
            serde_json::to_value(snapshot_guard.world_store.client_command_suggestions())
        }
        "world.client_features" => {
            serde_json::to_value(snapshot_guard.world_store.enabled_feature_list())
        }
        "world.client_debug_query" => {
            serde_json::to_value(snapshot_guard.world_store.client_debug_query())
        }
        "world.client_debug_game" => {
            serde_json::to_value(snapshot_guard.world_store.client_debug_game())
        }
        "world.client_hud" => serde_json::to_value(snapshot_guard.world_store.client_hud()),
        "world.client_local_player" => {
            serde_json::to_value(snapshot_guard.world_store.client_local_player())
        }
        "world.client_stats" => serde_json::to_value(snapshot_guard.world_store.client_stats()),
        "world.client_waypoints" => {
            serde_json::to_value(snapshot_guard.world_store.client_waypoints())
        }
        "world.client_ui" => serde_json::to_value(snapshot_guard.world_store.client_ui()),
        "world.last_map_color_patch" => {
            serde_json::to_value(snapshot_guard.world_store.last_map_color_patch())
        }
        "world.command_tree" => serde_json::to_value(snapshot_guard.world_store.commands()),
        "world.last_block_changed_ack" => {
            serde_json::to_value(snapshot_guard.world_store.last_block_changed_ack())
        }
        "world.level_clock" => Ok(serde_json::json!({
            "world_time": snapshot_guard.world_store.world_time(),
            "weather": snapshot_guard.world_store.weather(),
            "ticking": snapshot_guard.world_store.ticking(),
        })),
        "world.chunk_view" => {
            let view = snapshot_guard.world_store.chunk_view();
            Ok(serde_json::json!({
                "first_chunk": snapshot_guard.world_store.first_chunk(),
                "center": view.center,
                "radius": view.radius,
            }))
        }
        "world.server_presentation" => {
            serde_json::to_value(snapshot_guard.world_store.presentation())
        }
        "world.probe_chunk" => {
            let x = i32_param(&request.params, "x");
            let z = i32_param(&request.params, "z");
            let result = match (x, z) {
                (Some(x), Some(z)) => snapshot_guard
                    .world_store
                    .probe_chunk(ChunkPos { x, z })
                    .map(chunk_probe_summary)
                    .unwrap_or(serde_json::Value::Null),
                _ => serde_json::Value::Null,
            };
            Ok(result)
        }
        "world.probe_entity" => {
            let id = i32_param(&request.params, "id");
            let result = match id {
                Some(id) => snapshot_guard
                    .world_store
                    .probe_entity(id)
                    .map(|entity| serde_json::to_value(entity).expect("entity state serializes"))
                    .unwrap_or(serde_json::Value::Null),
                None => serde_json::Value::Null,
            };
            Ok(result)
        }
        "world.probe_entity_transform" => {
            let id = i32_param(&request.params, "id");
            let result = match id {
                Some(id) => snapshot_guard
                    .world_store
                    .probe_entity_transform(id)
                    .map(|entity| {
                        serde_json::to_value(entity).expect("entity transform state serializes")
                    })
                    .unwrap_or(serde_json::Value::Null),
                None => serde_json::Value::Null,
            };
            Ok(result)
        }
        "world.entity_transforms" => {
            serde_json::to_value(snapshot_guard.world_store.entity_transforms())
        }
        "world.entity_pick_targets" => {
            let partial_tick = f32_param(&request.params, "partial_tick").unwrap_or(1.0);
            serde_json::to_value(
                snapshot_guard
                    .world_store
                    .entity_pick_targets_at_partial_tick(partial_tick),
            )
        }
        "world.probe_block" => {
            let x = i32_param(&request.params, "x");
            let y = i32_param(&request.params, "y");
            let z = i32_param(&request.params, "z");
            let result = match (x, y, z) {
                (Some(x), Some(y), Some(z)) => snapshot_guard
                    .world_store
                    .probe_block(BlockPos { x, y, z })
                    .map(|probe| serde_json::to_value(probe).expect("block probe serializes"))
                    .unwrap_or(serde_json::Value::Null),
                _ => serde_json::Value::Null,
            };
            Ok(result)
        }
        "world.terrain_chunk_summary" => {
            let x = i32_param(&request.params, "x");
            let z = i32_param(&request.params, "z");
            let result = match (x, z) {
                (Some(x), Some(z)) => snapshot_guard
                    .world_store
                    .extract_terrain_chunk(ChunkPos { x, z })
                    .map(|snapshot| {
                        serde_json::to_value(snapshot.summary())
                            .expect("terrain summary serializes")
                    })
                    .unwrap_or(serde_json::Value::Null),
                _ => serde_json::Value::Null,
            };
            Ok(result)
        }
        _ => {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(format!("unknown method {}", request.method)),
            };
        }
    };

    match json {
        Ok(value) => ControlResponse {
            ok: true,
            result: Some(value),
            error: None,
        },
        Err(err) => ControlResponse {
            ok: false,
            result: None,
            error: Some(err.to_string()),
        },
    }
}

fn i32_param(params: &serde_json::Value, key: &str) -> Option<i32> {
    params.get(key)?.as_i64()?.try_into().ok()
}

fn f32_param(params: &serde_json::Value, key: &str) -> Option<f32> {
    params
        .get(key)?
        .as_f64()
        .filter(|value| value.is_finite())
        .map(|value| value as f32)
}

fn string_param<'a>(params: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    params.get(key)?.as_str()
}

fn non_empty_string_param<'a>(params: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    let value = string_param(params, key)?;
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn bool_param(params: &serde_json::Value, key: &str) -> Option<bool> {
    params.get(key)?.as_bool()
}

fn chunk_probe_summary(chunk: &ChunkColumn) -> serde_json::Value {
    serde_json::json!({
        "pos": chunk.pos,
        "state": chunk.state,
        "heightmaps": chunk.heightmaps.len(),
        "sections": chunk.sections.len(),
        "block_entities": chunk.block_entities.len(),
        "sky_light_arrays": chunk.light.sky_updates.len(),
        "block_light_arrays": chunk.light.block_updates.len(),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, AdvancementSummary, AwardStats, BlockChangedAck,
        BlockPos as ProtocolBlockPos, ChatTypeBound, ChatTypeHolder, CommandArgumentParser,
        CommandNode, CommandNodeType, Commands, CustomChatCompletions, CustomChatCompletionsAction,
        CustomPayload, CustomPayloadBody, CustomReportDetails, DebugBlockValue, DialogHolder,
        DisguisedChat as ProtocolDisguisedChat, EntityPositionSync as ProtocolEntityPositionSync,
        GameEvent, GameRuleValue, GameRuleValues, InteractionHand, MapColorPatch, MapDecoration,
        MapItemData, MountScreenOpen, OpenBook, OpenSignEditor, PlaceGhostRecipe, PlayTime,
        PlayerAbilities, PlayerCombatKill, PlayerExperience, PlayerHealth, PongResponse,
        RecipeDisplayType, SelectAdvancementsTab, ServerLinkEntry, ServerLinkKnownType,
        ServerLinkType, ServerLinks, SetActionBarText, SetChunkCacheCenter, SetChunkCacheRadius,
        SetDefaultSpawnPosition, SetSimulationDistance, SetSubtitleText, SetTitleText,
        SetTitlesAnimation, ShowDialog, SoundEvent, SoundEventHolder, SoundSource, StatUpdate,
        StopSound, SystemChat, TagQuery, TickingState, TickingStep, TrackedWaypoint,
        TrackedWaypointPacket, Transfer, UpdateAdvancements, UpdateEnabledFeatures,
        Vec3d as ProtocolVec3d, WaypointData, WaypointIcon, WaypointIdentifier, WaypointOperation,
        WaypointVec3i,
    };
    use bbb_world::{
        BlockEntityRecord, ChunkSection, ChunkState, HeightmapData, LightData, PaletteDomain,
        PaletteKind, PalettedContainerData, WorldDimension, WorldStore,
    };
    use serde_json::json;

    #[test]
    fn app_quit_marks_snapshot_not_running() {
        let snapshot = shared_snapshot("test");
        let response = dispatch(
            ControlRequest {
                method: "app.quit".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
        );

        assert!(response.ok);
        assert!(!snapshot.read().unwrap().app.running);
    }

    #[test]
    fn app_status_reads_world_counters_from_world_store() {
        let snapshot = shared_snapshot("test");
        {
            let mut store = WorldStore::new();
            store.apply_block_changed_ack(BlockChangedAck { sequence: 17 });
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "app.status".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
        );

        assert!(response.ok);
        let status = response.result.unwrap();
        assert_eq!(status["app"]["version"], "test");
        assert_eq!(status["world"]["block_changed_ack_packets"], 1);
        assert_eq!(status["world"]["block_destructions_tracked"], 0);
    }

    #[test]
    fn renderer_screenshot_queues_request() {
        let snapshot = shared_snapshot("test");
        let response = dispatch(
            ControlRequest {
                method: "renderer.screenshot".to_string(),
                params: json!({"path": "target/control-shot.png"}),
            },
            &snapshot,
        );

        assert!(response.ok);
        assert_eq!(response.result.unwrap()["queued"], true);
        assert_eq!(
            snapshot.read().unwrap().screenshot_request.as_deref(),
            Some("target/control-shot.png")
        );

        let missing_path = dispatch(
            ControlRequest {
                method: "renderer.screenshot".to_string(),
                params: json!({}),
            },
            &snapshot,
        );
        assert!(!missing_path.ok);
    }

    #[test]
    fn net_accept_code_of_conduct_queues_request() {
        let snapshot = shared_snapshot("test");
        let response = dispatch(
            ControlRequest {
                method: "net.accept_code_of_conduct".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
        );

        assert!(response.ok);
        assert_eq!(response.result.unwrap()["queued"], true);
        assert_eq!(
            snapshot.read().unwrap().code_of_conduct_requests,
            vec![CodeOfConductControlRequest::Accept { remember: false }]
        );
    }

    #[test]
    fn net_accept_code_of_conduct_can_queue_persistent_accept() {
        let snapshot = shared_snapshot("test");
        let response = dispatch(
            ControlRequest {
                method: "net.accept_code_of_conduct".to_string(),
                params: json!({"remember": true}),
            },
            &snapshot,
        );

        assert!(response.ok);
        let result = response.result.unwrap();
        assert_eq!(result["queued"], true);
        assert_eq!(result["remember"], true);
        assert_eq!(
            snapshot.read().unwrap().code_of_conduct_requests,
            vec![CodeOfConductControlRequest::Accept { remember: true }]
        );
    }

    #[test]
    fn net_decline_code_of_conduct_queues_request() {
        let snapshot = shared_snapshot("test");
        let response = dispatch(
            ControlRequest {
                method: "net.decline_code_of_conduct".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
        );

        assert!(response.ok);
        assert_eq!(response.result.unwrap()["queued"], true);
        assert_eq!(
            snapshot.read().unwrap().code_of_conduct_requests,
            vec![CodeOfConductControlRequest::Decline]
        );
    }

    #[test]
    fn net_clear_code_of_conduct_acceptance_queues_request() {
        let snapshot = shared_snapshot("test");
        let response = dispatch(
            ControlRequest {
                method: "net.clear_code_of_conduct_acceptance".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
        );

        assert!(response.ok);
        assert_eq!(
            snapshot.read().unwrap().code_of_conduct_requests,
            vec![CodeOfConductControlRequest::ClearAcceptance]
        );
    }

    #[test]
    fn net_send_chat_command_queues_request() {
        let snapshot = shared_snapshot("test");
        let response = dispatch(
            ControlRequest {
                method: "net.send_chat_command".to_string(),
                params: json!({"command": "give @p minecraft:stone"}),
            },
            &snapshot,
        );

        assert!(response.ok);
        assert_eq!(response.result.unwrap()["pending"], 1);
        assert_eq!(
            snapshot.read().unwrap().net_requests,
            vec![NetControlRequest::ChatCommand {
                command: "give @p minecraft:stone".to_string()
            }]
        );

        let missing_command = dispatch(
            ControlRequest {
                method: "net.send_chat_command".to_string(),
                params: json!({}),
            },
            &snapshot,
        );
        assert!(!missing_command.ok);
    }

    #[test]
    fn net_request_command_suggestions_queues_request() {
        let snapshot = shared_snapshot("test");
        let response = dispatch(
            ControlRequest {
                method: "net.request_command_suggestions".to_string(),
                params: json!({"id": 18, "command": "/give @p minecraft:stone"}),
            },
            &snapshot,
        );

        assert!(response.ok);
        assert_eq!(response.result.unwrap()["pending"], 1);
        assert_eq!(
            snapshot.read().unwrap().net_requests,
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
            &snapshot,
        );
        assert!(!missing_id.ok);
    }

    #[test]
    fn net_container_button_click_queues_request() {
        let snapshot = shared_snapshot("test");
        let response = dispatch(
            ControlRequest {
                method: "net.container_button_click".to_string(),
                params: json!({"container_id": 7, "button_id": 2}),
            },
            &snapshot,
        );

        assert!(response.ok);
        assert_eq!(response.result.unwrap()["pending"], 1);
        assert_eq!(
            snapshot.read().unwrap().net_requests,
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
            &snapshot,
        );
        assert!(!missing_button.ok);
    }

    #[test]
    fn net_container_click_queues_request() {
        let snapshot = shared_snapshot("test");
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
            &snapshot,
        );

        assert!(response.ok, "{response:?}");
        assert_eq!(response.result.unwrap()["pending"], 1);
        assert_eq!(
            snapshot.read().unwrap().net_requests,
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
            &snapshot,
        );
        assert!(!missing_input.ok);
    }

    #[test]
    fn net_container_close_queues_request() {
        let snapshot = shared_snapshot("test");
        let response = dispatch(
            ControlRequest {
                method: "net.container_close".to_string(),
                params: json!({"container_id": 7}),
            },
            &snapshot,
        );

        assert!(response.ok);
        assert_eq!(response.result.unwrap()["pending"], 1);
        assert_eq!(
            snapshot.read().unwrap().net_requests,
            vec![NetControlRequest::ContainerClose { container_id: 7 }]
        );

        let missing_container = dispatch(
            ControlRequest {
                method: "net.container_close".to_string(),
                params: json!({}),
            },
            &snapshot,
        );
        assert!(!missing_container.ok);
    }

    #[test]
    fn net_container_slot_state_changed_queues_request() {
        let snapshot = shared_snapshot("test");
        let response = dispatch(
            ControlRequest {
                method: "net.container_slot_state_changed".to_string(),
                params: json!({"slot_id": 12, "container_id": 7, "new_state": true}),
            },
            &snapshot,
        );

        assert!(response.ok);
        assert_eq!(response.result.unwrap()["pending"], 1);
        assert_eq!(
            snapshot.read().unwrap().net_requests,
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
            &snapshot,
        );
        assert!(!missing_state.ok);
    }

    #[test]
    fn client_hud_reads_canonical_world_state() {
        let snapshot = shared_snapshot("test");
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
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.client_hud".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
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
    fn client_chat_reads_canonical_world_state() {
        let snapshot = shared_snapshot("test");
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
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.client_chat".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
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
        let snapshot = shared_snapshot("test");
        {
            let mut store = WorldStore::new();
            store.apply_player_combat_kill(PlayerCombatKill {
                player_id: 123,
                message: "You died".to_string(),
            });
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.client_combat".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
        );

        assert!(response.ok);
        let combat = response.result.unwrap();
        assert_eq!(combat["last_combat"]["kind"], "kill");
        assert_eq!(combat["last_combat"]["duration"], serde_json::Value::Null);
        assert_eq!(combat["last_combat"]["player_id"], 123);
        assert_eq!(combat["last_combat"]["message"], "You died");
    }

    #[test]
    fn client_stats_reads_canonical_world_state() {
        let snapshot = shared_snapshot("test");
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
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.client_stats".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
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
        let snapshot = shared_snapshot("test");
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
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.client_waypoints".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
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
        let snapshot = shared_snapshot("test");
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
                recipe_display_type: RecipeDisplayType::Stonecutter,
                recipe_display_body: vec![1, 2, 3],
            });
            store.apply_show_dialog(ShowDialog {
                dialog: DialogHolder::Reference { registry_id: 7 },
            });
            store.apply_pong_response(PongResponse { time: 123456789 });
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.client_ui".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
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
        assert_eq!(ui["last_ghost_recipe"]["recipe_display_body_len"], 3);
        assert_eq!(ui["current_dialog"]["holder_kind"], "reference");
        assert_eq!(ui["current_dialog"]["registry_id"], 7);
        assert_eq!(ui["last_pong_response"]["time"], 123456789);
    }

    #[test]
    fn client_audio_reads_canonical_world_state() {
        let snapshot = shared_snapshot("test");
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
            store.apply_stop_sound(StopSound {
                source: Some(SoundSource::Music),
                name: Some("minecraft:music.menu".to_string()),
            });
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.client_audio".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
        );

        assert!(response.ok);
        let audio = response.result.unwrap();
        assert_eq!(audio["last_sound"]["sound"]["kind"], "reference");
        assert_eq!(audio["last_sound"]["sound"]["registry_id"], 41);
        assert_eq!(audio["last_sound"]["position"]["x"], 2.5);
        assert_eq!(audio["last_sound"]["position"]["y"], -1.0);
        assert_eq!(audio["last_sound"]["position"]["z"], 0.0);
        assert_eq!(audio["last_stop_sound"]["source"], "music");
        assert_eq!(audio["last_stop_sound"]["name"], "minecraft:music.menu");
    }

    #[test]
    fn client_debug_game_reads_canonical_world_state() {
        let snapshot = shared_snapshot("test");
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
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.client_debug_game".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
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
        let snapshot = shared_snapshot("test");
        {
            let mut store = WorldStore::new();
            store.apply_tag_query(TagQuery {
                transaction_id: 12,
                tag_present: true,
                raw_nbt: vec![10, 0],
            });
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.client_debug_query".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
        );

        assert!(response.ok);
        let debug_query = response.result.unwrap();
        assert_eq!(debug_query["last_tag_query"]["transaction_id"], 12);
        assert_eq!(debug_query["last_tag_query"]["tag_present"], true);
        assert_eq!(debug_query["last_tag_query"]["raw_nbt"], json!([10, 0]));
    }

    #[test]
    fn client_command_suggestions_reads_canonical_world_state() {
        let snapshot = shared_snapshot("test");
        {
            let mut store = WorldStore::new();
            store.apply_custom_chat_completions(CustomChatCompletions {
                action: CustomChatCompletionsAction::Set,
                entries: vec!["/warp".to_string(), "/spawn".to_string()],
            });
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.client_command_suggestions".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
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
        let snapshot = shared_snapshot("test");
        {
            let mut store = WorldStore::new();
            store.apply_update_enabled_features(UpdateEnabledFeatures {
                features: vec![
                    "minecraft:unknown".to_string(),
                    "minecraft:vanilla".to_string(),
                    "minecraft:trade_rebalance".to_string(),
                ],
            });
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.client_features".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
        );

        assert!(response.ok);
        assert_eq!(
            response.result.unwrap(),
            json!(["minecraft:trade_rebalance", "minecraft:vanilla"])
        );
    }

    #[test]
    fn command_tree_reads_canonical_world_state() {
        let snapshot = shared_snapshot("test");
        {
            let mut store = WorldStore::new();
            store.apply_commands(command_tree("say"));
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.command_tree".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
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
        let snapshot = shared_snapshot("test");
        {
            let mut store = WorldStore::new();
            store.apply_block_changed_ack(BlockChangedAck { sequence: 17 });
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.last_block_changed_ack".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
        );

        assert!(response.ok);
        assert_eq!(response.result.unwrap()["sequence"], 17);
    }

    #[test]
    fn client_advancements_reads_canonical_world_state() {
        let snapshot = shared_snapshot("test");
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
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.client_advancements".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
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
        let snapshot = shared_snapshot("test");
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
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.client_local_player".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
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
    fn last_map_color_patch_reads_canonical_world_state() {
        let snapshot = shared_snapshot("test");
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
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.last_map_color_patch".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
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
        let snapshot = shared_snapshot("test");
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
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.level_clock".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
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
        let snapshot = shared_snapshot("test");
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
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.server_presentation".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
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
        let snapshot = shared_snapshot("test");
        {
            let mut store = WorldStore::with_dimension(WorldDimension {
                min_y: 0,
                height: 16,
            });
            store.insert_decoded_chunk(single_section_chunk());

            let mut guard = snapshot.write().unwrap();
            guard.world_store = store;
        }

        let chunk_response = dispatch(
            ControlRequest {
                method: "world.probe_chunk".to_string(),
                params: json!({"x": 1, "z": -2}),
            },
            &snapshot,
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
            &snapshot,
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
            &snapshot,
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
            &snapshot,
        );
        assert!(missing_response.ok);
        assert!(missing_response.result.unwrap().is_null());
    }

    #[test]
    fn chunk_view_reads_canonical_world_state() {
        let snapshot = shared_snapshot("test");
        {
            let mut store = WorldStore::new();
            store.insert_decoded_chunk(single_section_chunk());
            store.apply_set_chunk_cache_center(SetChunkCacheCenter {
                chunk_x: -4,
                chunk_z: 7,
            });
            store.apply_set_chunk_cache_radius(SetChunkCacheRadius { radius: 10 });
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.chunk_view".to_string(),
                params: serde_json::Value::Null,
            },
            &snapshot,
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
        let snapshot = shared_snapshot("test");
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

            snapshot.write().unwrap().world_store = store;
        }

        let one_response = dispatch(
            ControlRequest {
                method: "world.probe_entity_transform".to_string(),
                params: json!({"id": 9}),
            },
            &snapshot,
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
            &snapshot,
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
            &snapshot,
        );
        assert!(missing_response.ok);
        assert!(missing_response.result.unwrap().is_null());
    }

    #[test]
    fn entity_pick_targets_probe_exposes_ender_dragon_part_targets() {
        let snapshot = shared_snapshot("test");
        {
            let mut store = WorldStore::new();
            store.apply_add_entity(protocol_add_entity(100, 43));
            store.advance_entity_client_animations(1);
            snapshot.write().unwrap().world_store = store;
        }

        let response = dispatch(
            ControlRequest {
                method: "world.entity_pick_targets".to_string(),
                params: json!({"partial_tick": 1.0}),
            },
            &snapshot,
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
}
