use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetCounters {
    pub connected: bool,
    pub state: Option<String>,
    pub compression_threshold: Option<i32>,
    pub packets_seen: usize,
    pub play_logins_received: usize,
    #[serde(default)]
    pub respawns_received: usize,
    pub player_position_packets: usize,
    pub player_abilities_packets: usize,
    pub player_health_packets: usize,
    pub player_experience_packets: usize,
    pub held_slot_packets: usize,
    #[serde(default)]
    pub held_slot_updates_applied: usize,
    #[serde(default)]
    pub held_slot_updates_ignored: usize,
    pub default_spawn_position_packets: usize,
    pub simulation_distance_packets: usize,
    pub system_chat_packets: usize,
    pub player_look_at_packets: usize,
    pub player_rotation_packets: usize,
    pub action_bar_packets: usize,
    pub title_text_packets: usize,
    pub subtitle_text_packets: usize,
    pub clear_titles_packets: usize,
    pub titles_animation_packets: usize,
    pub ticking_state_packets: usize,
    pub ticking_step_packets: usize,
    pub set_camera_packets: usize,
    #[serde(default)]
    pub set_camera_updates_applied: usize,
    #[serde(default)]
    pub set_camera_updates_ignored: usize,
    pub held_slot_commands_queued: usize,
    pub player_action_commands_queued: usize,
    pub player_command_commands_queued: usize,
    pub player_input_commands_queued: usize,
    #[serde(default)]
    pub chat_command_commands_queued: usize,
    #[serde(default)]
    pub attack_entity_commands_queued: usize,
    #[serde(default)]
    pub interact_entity_commands_queued: usize,
    pub swing_commands_queued: usize,
    pub use_item_on_commands_queued: usize,
    pub use_item_commands_queued: usize,
    pub pick_item_from_block_commands_queued: usize,
    #[serde(default)]
    pub pick_item_from_entity_commands_queued: usize,
    pub command_suggestion_commands_queued: usize,
    #[serde(default)]
    pub container_close_commands_queued: usize,
    #[serde(default)]
    pub container_button_click_commands_queued: usize,
    #[serde(default)]
    pub container_click_commands_queued: usize,
    #[serde(default)]
    pub container_slot_state_changed_commands_queued: usize,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLookAtState {
    pub from_anchor: String,
    pub position: NetVec3,
    pub target_entity_id: Option<i32>,
    pub to_anchor: Option<String>,
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
