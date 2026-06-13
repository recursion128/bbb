use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use anyhow::{Context, Result};
use bbb_world::{BlockPos, ChunkColumn, ChunkPos, WorldCounters, WorldStore};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppStatus {
    pub version: String,
    pub running: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetCounters {
    pub connected: bool,
    pub state: Option<String>,
    pub compression_threshold: Option<i32>,
    pub packets_seen: usize,
    pub registries_seen: usize,
    pub first_chunk: Option<ChunkPos>,
    pub chunk_cache_center: Option<ChunkPos>,
    pub chunk_cache_radius: Option<i32>,
    pub player_entity_id: Option<i32>,
    pub player_pose: Option<PlayerPose>,
    pub player_abilities: Option<PlayerAbilities>,
    pub player_health: Option<PlayerHealth>,
    pub player_experience: Option<PlayerExperience>,
    pub selected_hotbar_slot: u8,
    pub default_spawn: Option<DefaultSpawn>,
    pub simulation_distance: Option<i32>,
    pub world_time: Option<WorldTime>,
    pub weather: WeatherState,
    pub last_system_chat: Option<SystemChatLine>,
    pub player_position_packets: usize,
    pub player_abilities_packets: usize,
    pub player_health_packets: usize,
    pub player_experience_packets: usize,
    pub held_slot_packets: usize,
    pub default_spawn_position_packets: usize,
    pub simulation_distance_packets: usize,
    pub system_chat_packets: usize,
    pub block_changed_ack_packets: usize,
    pub player_rotation_packets: usize,
    pub last_block_changed_ack_sequence: Option<i32>,
    pub held_slot_commands_queued: usize,
    pub player_action_commands_queued: usize,
    pub player_command_commands_queued: usize,
    pub player_input_commands_queued: usize,
    pub swing_commands_queued: usize,
    pub use_item_on_commands_queued: usize,
    pub use_item_commands_queued: usize,
    pub pick_item_from_block_commands_queued: usize,
    pub world_time_packets: usize,
    pub game_event_packets: usize,
    pub player_move_commands_queued: usize,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct NetVec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerPose {
    pub position: NetVec3,
    pub delta_movement: NetVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub last_teleport_id: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerAbilities {
    pub invulnerable: bool,
    pub flying: bool,
    pub can_fly: bool,
    pub instabuild: bool,
    pub flying_speed: f32,
    pub walking_speed: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerHealth {
    pub health: f32,
    pub food: i32,
    pub saturation: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerExperience {
    pub progress: f32,
    pub level: i32,
    pub total: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefaultSpawn {
    pub dimension: String,
    pub pos: BlockPos,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldTime {
    pub game_time: i64,
    pub day_time: i64,
    pub clock_updates: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemChatLine {
    pub content: String,
    pub overlay: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WeatherState {
    pub raining: bool,
    pub rain_level: f32,
    pub thunder_level: f32,
    pub last_game_event_id: Option<u8>,
    pub last_game_event_param: f32,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            raining: false,
            rain_level: 0.0,
            thunder_level: 0.0,
            last_game_event_id: None,
            last_game_event_param: 0.0,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RendererCounters {
    pub frame_index: u64,
    pub width: u32,
    pub height: u32,
    pub draw_calls: u64,
    pub opaque_draw_calls: u64,
    pub cutout_draw_calls: u64,
    pub translucent_draw_calls: u64,
    pub selection_draw_calls: u64,
    pub hud_draw_calls: u64,
    pub pipeline_switches: u64,
    pub screenshots_written: u64,
    pub queued_sections: usize,
    pub meshed_sections: usize,
    pub uploaded_sections: usize,
    pub visible_sections: usize,
    pub upload_bytes: u64,
    pub resident_bytes: u64,
    pub atlas_pages: usize,
    pub atlas_reallocations: u64,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub hud_crosshair_width: u32,
    pub hud_crosshair_height: u32,
    pub terrain_vertices: usize,
    pub terrain_indices: usize,
    pub opaque_faces: usize,
    pub cutout_faces: usize,
    pub translucent_faces: usize,
    pub culled_faces: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ControlSnapshot {
    pub app: AppStatus,
    pub net: NetCounters,
    pub renderer: RendererCounters,
    pub world: WorldCounters,
    #[serde(skip)]
    pub screenshot_request: Option<String>,
    #[serde(skip)]
    pub world_store: WorldStore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlRequest {
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub type SharedSnapshot = Arc<RwLock<ControlSnapshot>>;

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
    use bbb_world::{
        BlockEntityRecord, ChunkSection, ChunkState, HeightmapData, LightData, PaletteDomain,
        PaletteKind, PalettedContainerData, WorldDimension,
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
}
