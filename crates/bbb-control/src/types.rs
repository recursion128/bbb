use std::sync::{Arc, RwLock};

use bbb_world::{BlockPos, ChunkPos, WorldCounters, WorldStore};
use serde::{Deserialize, Serialize};

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
    pub last_action_bar: Option<ActionBarText>,
    pub title: TitleState,
    pub ticking: ClientTickingState,
    pub camera: CameraState,
    pub player_position_packets: usize,
    pub player_info_update_packets: usize,
    pub player_info_remove_packets: usize,
    pub server_data_packets: usize,
    pub resource_pack_push_packets: usize,
    pub resource_pack_pop_packets: usize,
    pub cooldown_packets: usize,
    pub damage_event_packets: usize,
    pub update_mob_effect_packets: usize,
    pub remove_mob_effect_packets: usize,
    pub player_abilities_packets: usize,
    pub player_health_packets: usize,
    pub player_experience_packets: usize,
    pub held_slot_packets: usize,
    pub default_spawn_position_packets: usize,
    pub simulation_distance_packets: usize,
    pub system_chat_packets: usize,
    pub block_changed_ack_packets: usize,
    pub block_destruction_packets: usize,
    pub block_event_packets: usize,
    pub level_event_packets: usize,
    pub boss_event_packets: usize,
    pub change_difficulty_packets: usize,
    pub tab_list_packets: usize,
    pub initialize_border_packets: usize,
    pub set_border_center_packets: usize,
    pub set_border_lerp_size_packets: usize,
    pub set_border_size_packets: usize,
    pub set_border_warning_delay_packets: usize,
    pub set_border_warning_distance_packets: usize,
    pub reset_score_packets: usize,
    pub set_display_objective_packets: usize,
    pub set_objective_packets: usize,
    pub set_player_team_packets: usize,
    pub set_score_packets: usize,
    pub command_suggestion_packets: usize,
    pub player_rotation_packets: usize,
    pub move_vehicle_packets: usize,
    pub action_bar_packets: usize,
    pub title_text_packets: usize,
    pub subtitle_text_packets: usize,
    pub titles_animation_packets: usize,
    pub ticking_state_packets: usize,
    pub ticking_step_packets: usize,
    pub set_camera_packets: usize,
    pub take_item_entity_packets: usize,
    pub last_block_changed_ack_sequence: Option<i32>,
    pub held_slot_commands_queued: usize,
    pub player_action_commands_queued: usize,
    pub player_command_commands_queued: usize,
    pub player_input_commands_queued: usize,
    pub swing_commands_queued: usize,
    pub use_item_on_commands_queued: usize,
    pub use_item_commands_queued: usize,
    pub pick_item_from_block_commands_queued: usize,
    pub command_suggestion_commands_queued: usize,
    pub world_time_packets: usize,
    pub game_event_packets: usize,
    pub player_move_commands_queued: usize,
    pub move_vehicle_commands_queued: usize,
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionBarText {
    pub content: String,
    pub display_ticks: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TitleState {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub fade_in: i32,
    pub stay: i32,
    pub fade_out: i32,
    pub title_time: i32,
}

impl Default for TitleState {
    fn default() -> Self {
        Self {
            title: None,
            subtitle: None,
            fade_in: 10,
            stay: 70,
            fade_out: 20,
            title_time: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClientTickingState {
    pub tick_rate: f32,
    pub frozen: bool,
    pub frozen_ticks_to_run: i32,
}

impl Default for ClientTickingState {
    fn default() -> Self {
        Self {
            tick_rate: 20.0,
            frozen: false,
            frozen_ticks_to_run: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraState {
    pub entity_id: Option<i32>,
    pub follows_player: bool,
    pub entity_known: bool,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            entity_id: None,
            follows_player: true,
            entity_known: true,
        }
    }
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
