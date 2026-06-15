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
    AppStatus, CodeOfConductControlRequest, ControlRequest, ControlResponse, ControlSnapshot,
    SharedSnapshot,
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

    let snapshot_guard = snapshot.read().expect("control snapshot poisoned");
    let json = match request.method.as_str() {
        "app.status" => serde_json::to_value(&*snapshot_guard),
        "net.counters" => serde_json::to_value(&snapshot_guard.net),
        "renderer.counters" => serde_json::to_value(&snapshot_guard.renderer),
        "world.counters" => serde_json::to_value(snapshot_guard.world_store.counters()),
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

fn string_param<'a>(params: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    params.get(key)?.as_str()
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
    use super::*;
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, EntityPositionSync as ProtocolEntityPositionSync,
        Vec3d as ProtocolVec3d,
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
    fn probes_chunk_and_block_from_world_store() {
        let snapshot = shared_snapshot("test");
        {
            let mut store = WorldStore::with_dimension(WorldDimension {
                min_y: 0,
                height: 16,
            });
            store.insert_decoded_chunk(single_section_chunk());

            let mut guard = snapshot.write().unwrap();
            guard.net.first_chunk = Some(ChunkPos { x: 1, z: -2 });
            guard.world = store.counters();
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
