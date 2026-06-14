use std::collections::BTreeMap;

use bbb_world::{BlockPos, ChunkPos};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetCounters {
    pub connected: bool,
    pub state: Option<String>,
    pub compression_threshold: Option<i32>,
    pub packets_seen: usize,
    pub registries_seen: usize,
    #[serde(default)]
    pub registry_entries_seen: usize,
    #[serde(default)]
    pub registry_entries_with_data: usize,
    #[serde(default)]
    pub registry_entry_stubs: usize,
    #[serde(default)]
    pub registry_entry_payload_bytes: usize,
    #[serde(default)]
    pub registry_content_registries_tracked: usize,
    #[serde(default)]
    pub registry_content_packets_tracked: usize,
    #[serde(default)]
    pub registry_content_entries_tracked: usize,
    #[serde(default)]
    pub registry_duplicate_entries: usize,
    #[serde(default)]
    pub registry_duplicate_entry_ids_tracked: usize,
    #[serde(default)]
    pub last_registry_data_registry: Option<String>,
    #[serde(default)]
    pub last_registry_data_entry_count: usize,
    #[serde(default)]
    pub update_tags_packets: usize,
    #[serde(default)]
    pub last_update_tags_registry_count: usize,
    #[serde(default)]
    pub last_update_tags_total_tag_count: usize,
    #[serde(default)]
    pub last_update_tags_total_value_count: usize,
    #[serde(default)]
    pub tag_registries_tracked: usize,
    #[serde(default)]
    pub tags_tracked: usize,
    #[serde(default)]
    pub tag_entries_tracked: usize,
    pub first_chunk: Option<ChunkPos>,
    pub chunk_cache_center: Option<ChunkPos>,
    pub chunk_cache_radius: Option<i32>,
    pub player_entity_id: Option<i32>,
    pub player_pose: Option<PlayerPose>,
    pub last_player_combat: Option<PlayerCombatState>,
    pub last_player_look_at: Option<PlayerLookAtState>,
    pub player_abilities: Option<PlayerAbilities>,
    pub player_health: Option<PlayerHealth>,
    pub player_experience: Option<PlayerExperience>,
    pub selected_hotbar_slot: u8,
    pub default_spawn: Option<DefaultSpawn>,
    pub simulation_distance: Option<i32>,
    pub world_time: Option<WorldTime>,
    pub weather: WeatherState,
    pub last_cookie_key: Option<String>,
    pub custom_report_details: BTreeMap<String, String>,
    pub server_links: Vec<ServerLinkState>,
    pub last_system_chat: Option<SystemChatLine>,
    pub last_player_chat: Option<ClientChatLine>,
    pub last_disguised_chat: Option<ClientChatLine>,
    pub last_deleted_chat: Option<DeletedChatLine>,
    pub last_action_bar: Option<ActionBarText>,
    pub title: TitleState,
    pub ticking: ClientTickingState,
    pub camera: CameraState,
    pub last_transfer: Option<TransferTarget>,
    pub last_custom_chat_completion: Option<CustomChatCompletionState>,
    pub last_custom_payload: Option<CustomPayloadState>,
    pub last_mount_screen: Option<MountScreenState>,
    pub last_open_book_hand: Option<String>,
    pub last_open_sign_editor: Option<OpenSignEditorState>,
    pub last_ghost_recipe: Option<GhostRecipeState>,
    pub last_show_dialog: Option<ShowDialogState>,
    pub last_waypoint: Option<WaypointState>,
    pub last_pong_response_time: Option<i64>,
    pub last_sound: Option<ClientSoundState>,
    pub last_sound_entity: Option<ClientSoundEntityState>,
    pub last_stop_sound: Option<StopSoundState>,
    pub last_explosion: Option<ExplosionState>,
    pub last_level_particles: Option<LevelParticlesState>,
    pub last_projectile_power: Option<ProjectilePowerState>,
    pub last_debug_block_value: Option<DebugBlockValueState>,
    pub last_debug_chunk_value: Option<DebugChunkValueState>,
    pub last_debug_entity_value: Option<DebugEntityValueState>,
    pub last_debug_event: Option<DebugEventState>,
    pub last_debug_sample: Option<DebugSampleState>,
    pub last_game_rule_values: Option<GameRuleValuesState>,
    pub last_game_test_highlight_pos: Option<GameTestHighlightPosState>,
    pub last_test_instance_block_status: Option<TestInstanceBlockStatusState>,
    pub selected_advancements_tab: Option<String>,
    pub last_tag_query: Option<TagQueryState>,
    pub player_position_packets: usize,
    pub player_info_update_packets: usize,
    pub player_info_remove_packets: usize,
    #[serde(default)]
    pub player_info_entries_tracked: usize,
    #[serde(default)]
    pub listed_players_tracked: usize,
    pub server_data_packets: usize,
    pub resource_pack_push_packets: usize,
    pub resource_pack_pop_packets: usize,
    #[serde(default)]
    pub resource_packs_tracked: usize,
    pub cooldown_packets: usize,
    #[serde(default)]
    pub cooldowns_tracked: usize,
    pub damage_event_packets: usize,
    #[serde(default)]
    pub damage_events_applied: usize,
    pub update_mob_effect_packets: usize,
    pub remove_mob_effect_packets: usize,
    #[serde(default)]
    pub active_mob_effects_tracked: usize,
    pub cookie_request_packets: usize,
    pub cookie_response_hits: usize,
    pub cookie_response_misses: usize,
    pub store_cookie_packets: usize,
    pub stored_cookie_count: usize,
    pub stored_cookie_bytes: usize,
    pub custom_report_detail_packets: usize,
    #[serde(default)]
    pub reset_chat_packets: usize,
    #[serde(default)]
    pub update_enabled_features_packets: usize,
    #[serde(default)]
    pub enabled_features: Vec<String>,
    #[serde(default)]
    pub code_of_conduct_packets: usize,
    #[serde(default)]
    pub last_code_of_conduct_len: usize,
    pub server_link_packets: usize,
    pub server_link_invalid_entries: usize,
    pub player_abilities_packets: usize,
    pub player_health_packets: usize,
    pub player_experience_packets: usize,
    pub held_slot_packets: usize,
    pub default_spawn_position_packets: usize,
    pub simulation_distance_packets: usize,
    pub system_chat_packets: usize,
    pub player_chat_packets: usize,
    pub disguised_chat_packets: usize,
    pub delete_chat_packets: usize,
    pub chat_messages_tracked: usize,
    pub deleted_chat_messages_tracked: usize,
    pub chat_signature_cache_entries: usize,
    pub player_chat_index_mismatches: usize,
    pub chat_unknown_packed_signatures: usize,
    pub player_chat_unsigned_content_packets: usize,
    pub player_chat_filtered_packets: usize,
    pub player_chat_fully_filtered_packets: usize,
    pub block_changed_ack_packets: usize,
    pub block_destruction_packets: usize,
    #[serde(default)]
    pub block_destructions_tracked: usize,
    pub block_event_packets: usize,
    #[serde(default)]
    pub block_events_tracked: usize,
    pub level_event_packets: usize,
    #[serde(default)]
    pub level_events_tracked: usize,
    pub boss_event_packets: usize,
    #[serde(default)]
    pub boss_bars_tracked: usize,
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
    #[serde(default)]
    pub command_tree_packets: usize,
    #[serde(default)]
    pub command_nodes_tracked: usize,
    #[serde(default)]
    pub command_literal_nodes_tracked: usize,
    #[serde(default)]
    pub command_argument_nodes_tracked: usize,
    #[serde(default)]
    pub command_redirect_nodes_tracked: usize,
    #[serde(default)]
    pub command_executable_nodes_tracked: usize,
    #[serde(default)]
    pub command_restricted_nodes_tracked: usize,
    #[serde(default)]
    pub last_command_root_index: Option<i32>,
    pub command_suggestion_packets: usize,
    #[serde(default)]
    pub command_suggestion_entries_tracked: usize,
    pub player_combat_end_packets: usize,
    pub player_combat_enter_packets: usize,
    pub player_combat_kill_packets: usize,
    pub player_look_at_packets: usize,
    pub player_rotation_packets: usize,
    pub move_vehicle_packets: usize,
    #[serde(default)]
    pub vehicle_moves_applied: usize,
    #[serde(default)]
    pub vehicle_moves_acked: usize,
    #[serde(default)]
    pub vehicle_moves_snapped: usize,
    pub action_bar_packets: usize,
    pub title_text_packets: usize,
    pub subtitle_text_packets: usize,
    pub clear_titles_packets: usize,
    pub titles_animation_packets: usize,
    pub ticking_state_packets: usize,
    pub ticking_step_packets: usize,
    pub set_camera_packets: usize,
    pub transfer_packets: usize,
    pub take_item_entity_packets: usize,
    #[serde(default)]
    pub take_item_entities_applied: usize,
    #[serde(default)]
    pub item_entity_stack_shrinks: usize,
    #[serde(default)]
    pub take_item_entities_removed: usize,
    pub custom_chat_completion_packets: usize,
    pub custom_payload_packets: usize,
    #[serde(default)]
    pub custom_payload_brand_packets: usize,
    #[serde(default)]
    pub custom_payload_unknown_packets: usize,
    pub clear_dialog_packets: usize,
    pub show_dialog_packets: usize,
    pub waypoint_packets: usize,
    #[serde(default)]
    pub waypoints_tracked: usize,
    #[serde(default)]
    pub waypoint_updates_applied: usize,
    #[serde(default)]
    pub waypoint_updates_ignored: usize,
    #[serde(default)]
    pub waypoint_untracks_ignored: usize,
    pub low_disk_space_warnings: usize,
    pub mount_screen_open_packets: usize,
    pub open_book_packets: usize,
    pub open_sign_editor_packets: usize,
    pub ghost_recipe_packets: usize,
    pub pong_response_packets: usize,
    pub sound_packets: usize,
    pub sound_entity_packets: usize,
    #[serde(default)]
    pub sound_entity_events_applied: usize,
    #[serde(default)]
    pub sound_entity_events_ignored: usize,
    pub stop_sound_packets: usize,
    pub explosion_packets: usize,
    pub level_particles_packets: usize,
    pub projectile_power_packets: usize,
    pub debug_block_value_packets: usize,
    pub debug_chunk_value_packets: usize,
    pub debug_entity_value_packets: usize,
    pub debug_event_packets: usize,
    pub debug_sample_packets: usize,
    pub game_rule_value_packets: usize,
    pub game_test_highlight_pos_packets: usize,
    pub test_instance_block_status_packets: usize,
    pub select_advancements_tab_packets: usize,
    pub tag_query_packets: usize,
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCombatState {
    pub kind: String,
    pub duration: Option<i32>,
    pub player_id: Option<i32>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLookAtState {
    pub from_anchor: String,
    pub position: NetVec3,
    pub target_entity_id: Option<i32>,
    pub to_anchor: Option<String>,
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
pub struct ClientChatLine {
    pub kind: String,
    pub content: String,
    pub sender: Option<String>,
    pub sender_name: String,
    pub target_name: Option<String>,
    pub global_index: Option<i32>,
    pub message_index: Option<i32>,
    pub chat_type_id: Option<i32>,
    pub signature_checksum: Option<i32>,
    pub unsigned_content_present: bool,
    pub filter_mask: String,
    pub validation_state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletedChatLine {
    pub signature_checksum: Option<i32>,
    pub cache_id: Option<i32>,
    pub resolved: bool,
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferTarget {
    pub host: String,
    pub port: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerLinkState {
    pub label: String,
    pub url: String,
    pub known_type: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomChatCompletionState {
    pub action: String,
    pub entries: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomPayloadState {
    pub id: String,
    pub kind: String,
    pub brand: Option<String>,
    pub raw_payload_len: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountScreenState {
    pub container_id: i32,
    pub inventory_columns: i32,
    pub entity_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenSignEditorState {
    pub pos: BlockPos,
    pub is_front_text: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhostRecipeState {
    pub container_id: i32,
    pub recipe_display_type_id: i32,
    pub recipe_display_type: String,
    pub recipe_display_body_len: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowDialogState {
    pub holder_kind: String,
    pub registry_id: Option<i32>,
    pub raw_dialog_payload_len: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaypointState {
    pub operation: String,
    pub identifier_kind: String,
    pub identifier: String,
    pub icon_style: String,
    pub icon_color_rgb: Option<u32>,
    pub waypoint_kind: String,
    pub position: Option<NetVec3i>,
    pub chunk: Option<ChunkPos>,
    pub azimuth: Option<f32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagQueryState {
    pub transaction_id: i32,
    pub tag_present: bool,
    pub raw_nbt_len: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundHolderState {
    pub kind: String,
    pub registry_id: Option<i32>,
    pub location: Option<String>,
    pub fixed_range: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientSoundState {
    pub sound: SoundHolderState,
    pub source: String,
    pub position: NetVec3,
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientSoundEntityState {
    pub sound: SoundHolderState,
    pub source: String,
    pub entity_id: i32,
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopSoundState {
    pub source: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplosionState {
    pub center: NetVec3,
    pub radius: f32,
    pub block_count: i32,
    pub player_knockback: Option<NetVec3>,
    pub raw_effect_payload_len: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelParticlesState {
    pub override_limiter: bool,
    pub always_show: bool,
    pub position: NetVec3,
    pub offset: NetVec3,
    pub max_speed: f32,
    pub count: i32,
    pub particle_type_id: i32,
    pub raw_options_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProjectilePowerState {
    pub entity_id: i32,
    pub acceleration_power: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetVec3i {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugBlockValueState {
    pub pos: BlockPos,
    pub raw_update_payload_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugChunkValueState {
    pub pos: ChunkPos,
    pub raw_update_payload_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugEntityValueState {
    pub entity_id: i32,
    pub raw_update_payload_len: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugEventState {
    pub raw_event_payload_len: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugSampleState {
    pub sample_len: usize,
    pub sample_type: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRuleValuesState {
    pub values: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameTestHighlightPosState {
    pub absolute_pos: BlockPos,
    pub relative_pos: BlockPos,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestInstanceBlockStatusState {
    pub status: String,
    pub size: Option<NetVec3i>,
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
