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
    pub resource_pack_pop_updates_applied: usize,
    #[serde(default)]
    pub resource_pack_pop_updates_ignored: usize,
    #[serde(default)]
    pub resource_packs_tracked: usize,
    pub cooldown_packets: usize,
    #[serde(default)]
    pub cooldowns_tracked: usize,
    pub damage_event_packets: usize,
    #[serde(default)]
    pub damage_events_applied: usize,
    #[serde(default)]
    pub damage_events_ignored: usize,
    pub update_mob_effect_packets: usize,
    #[serde(default)]
    pub update_mob_effects_ignored: usize,
    pub remove_mob_effect_packets: usize,
    #[serde(default)]
    pub remove_mob_effects_ignored: usize,
    #[serde(default)]
    pub active_mob_effects_tracked: usize,
    #[serde(default)]
    pub inventory_slot_updates_received: usize,
    #[serde(default)]
    pub inventory_slots_tracked: usize,
    #[serde(default)]
    pub cursor_item_updates_received: usize,
    #[serde(default)]
    pub container_open_updates_received: usize,
    #[serde(default)]
    pub container_content_updates_received: usize,
    #[serde(default)]
    pub container_slot_updates_received: usize,
    #[serde(default)]
    pub container_data_updates_received: usize,
    #[serde(default)]
    pub container_close_updates_received: usize,
    #[serde(default)]
    pub container_close_updates_applied: usize,
    #[serde(default)]
    pub container_close_updates_ignored: usize,
    #[serde(default)]
    pub merchant_offer_packets_received: usize,
    #[serde(default)]
    pub merchant_offer_packets_applied: usize,
    #[serde(default)]
    pub merchant_offer_packets_ignored: usize,
    #[serde(default)]
    pub merchant_offers_tracked: usize,
    pub cookie_request_packets: usize,
    pub cookie_response_hits: usize,
    pub cookie_response_misses: usize,
    pub store_cookie_packets: usize,
    pub stored_cookie_bytes: usize,
    pub custom_report_detail_packets: usize,
    #[serde(default)]
    pub custom_report_details_tracked: usize,
    #[serde(default)]
    pub reset_chat_packets: usize,
    #[serde(default)]
    pub update_enabled_features_packets: usize,
    #[serde(default)]
    pub enabled_features_tracked: usize,
    #[serde(default)]
    pub enabled_features_ignored: usize,
    #[serde(default)]
    pub code_of_conduct_packets: usize,
    #[serde(default)]
    pub server_link_packets: usize,
    pub server_link_invalid_entries: usize,
    #[serde(default)]
    pub server_links_tracked: usize,
    #[serde(default)]
    pub award_stats_packets: usize,
    #[serde(default)]
    pub award_stats_entries_received: usize,
    #[serde(default)]
    pub stats_tracked: usize,
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
    #[serde(default)]
    pub block_destructions_removed: usize,
    #[serde(default)]
    pub block_destructions_ignored: usize,
    pub block_event_packets: usize,
    #[serde(default)]
    pub block_events_tracked: usize,
    pub level_event_packets: usize,
    #[serde(default)]
    pub level_events_tracked: usize,
    pub boss_event_packets: usize,
    #[serde(default)]
    pub boss_bars_tracked: usize,
    #[serde(default)]
    pub boss_events_ignored: usize,
    pub change_difficulty_packets: usize,
    pub tab_list_packets: usize,
    pub initialize_border_packets: usize,
    pub set_border_center_packets: usize,
    pub set_border_lerp_size_packets: usize,
    pub set_border_size_packets: usize,
    pub set_border_warning_delay_packets: usize,
    pub set_border_warning_distance_packets: usize,
    pub reset_score_packets: usize,
    #[serde(default)]
    pub reset_score_updates_applied: usize,
    #[serde(default)]
    pub reset_score_updates_ignored: usize,
    pub set_display_objective_packets: usize,
    #[serde(default)]
    pub set_display_objective_updates_applied: usize,
    #[serde(default)]
    pub set_display_objective_updates_ignored: usize,
    pub set_objective_packets: usize,
    #[serde(default)]
    pub set_objective_updates_applied: usize,
    #[serde(default)]
    pub set_objective_updates_ignored: usize,
    pub set_player_team_packets: usize,
    #[serde(default)]
    pub set_player_team_updates_applied: usize,
    #[serde(default)]
    pub set_player_team_updates_ignored: usize,
    pub set_score_packets: usize,
    #[serde(default)]
    pub set_score_updates_applied: usize,
    #[serde(default)]
    pub set_score_updates_ignored: usize,
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
    pub command_suggestion_packets: usize,
    #[serde(default)]
    pub command_suggestion_entries_tracked: usize,
    #[serde(default)]
    pub recipe_book_add_packets: usize,
    #[serde(default)]
    pub recipe_book_remove_packets: usize,
    #[serde(default)]
    pub recipe_book_settings_packets: usize,
    #[serde(default)]
    pub recipe_book_replace_packets: usize,
    #[serde(default)]
    pub recipe_book_entries_received: usize,
    #[serde(default)]
    pub recipe_book_removed_entries_received: usize,
    #[serde(default)]
    pub recipe_book_entries_tracked: usize,
    #[serde(default)]
    pub recipe_book_highlights_tracked: usize,
    #[serde(default)]
    pub recipe_book_notifications_received: usize,
    #[serde(default)]
    pub update_recipes_packets: usize,
    #[serde(default)]
    pub recipe_property_sets_tracked: usize,
    #[serde(default)]
    pub recipe_property_set_items_tracked: usize,
    #[serde(default)]
    pub stonecutter_recipes_tracked: usize,
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
    #[serde(default)]
    pub vehicle_moves_ignored: usize,
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
    #[serde(default)]
    pub entities_tracked: usize,
    #[serde(default)]
    pub entities_received: usize,
    #[serde(default)]
    pub entity_position_syncs_received: usize,
    #[serde(default)]
    pub entity_position_syncs_applied: usize,
    #[serde(default)]
    pub entity_position_syncs_ignored: usize,
    #[serde(default)]
    pub entity_moves_received: usize,
    #[serde(default)]
    pub entity_moves_applied: usize,
    #[serde(default)]
    pub entity_moves_ignored: usize,
    #[serde(default)]
    pub entity_teleports_received: usize,
    #[serde(default)]
    pub entity_teleports_applied: usize,
    #[serde(default)]
    pub entity_teleports_ignored: usize,
    #[serde(default)]
    pub entity_animation_updates_received: usize,
    #[serde(default)]
    pub entity_animation_updates_applied: usize,
    #[serde(default)]
    pub entity_animation_updates_ignored: usize,
    #[serde(default)]
    pub entity_events_received: usize,
    #[serde(default)]
    pub entity_events_applied: usize,
    #[serde(default)]
    pub entity_events_ignored: usize,
    #[serde(default)]
    pub entity_hurt_animations_received: usize,
    #[serde(default)]
    pub entity_hurt_animations_applied: usize,
    #[serde(default)]
    pub entity_hurt_animations_ignored: usize,
    #[serde(default)]
    pub entity_data_updates_received: usize,
    #[serde(default)]
    pub entity_data_values_received: usize,
    #[serde(default)]
    pub entity_data_updates_applied: usize,
    #[serde(default)]
    pub entity_data_updates_ignored: usize,
    #[serde(default)]
    pub entity_equipment_updates_received: usize,
    #[serde(default)]
    pub entity_equipment_slots_received: usize,
    #[serde(default)]
    pub entity_equipment_updates_applied: usize,
    #[serde(default)]
    pub entity_equipment_updates_ignored: usize,
    #[serde(default)]
    pub entity_attribute_updates_received: usize,
    #[serde(default)]
    pub entity_attributes_received: usize,
    #[serde(default)]
    pub entity_attribute_updates_applied: usize,
    #[serde(default)]
    pub entity_attribute_updates_ignored: usize,
    #[serde(default)]
    pub entity_passenger_updates_received: usize,
    #[serde(default)]
    pub entity_passenger_ids_received: usize,
    #[serde(default)]
    pub entity_passenger_updates_applied: usize,
    #[serde(default)]
    pub entity_passenger_updates_ignored: usize,
    #[serde(default)]
    pub entity_link_updates_received: usize,
    #[serde(default)]
    pub entity_link_updates_applied: usize,
    #[serde(default)]
    pub entity_link_updates_ignored: usize,
    #[serde(default)]
    pub entity_motion_updates_received: usize,
    #[serde(default)]
    pub entity_motion_updates_applied: usize,
    #[serde(default)]
    pub entity_motion_updates_ignored: usize,
    #[serde(default)]
    pub entity_head_rotations_received: usize,
    #[serde(default)]
    pub entity_head_rotations_applied: usize,
    #[serde(default)]
    pub entity_head_rotations_ignored: usize,
    #[serde(default)]
    pub entity_removes_received: usize,
    #[serde(default)]
    pub entities_removed: usize,
    #[serde(default)]
    pub entity_removes_ignored: usize,
    #[serde(default)]
    pub minecart_moves_received: usize,
    #[serde(default)]
    pub minecart_moves_applied: usize,
    #[serde(default)]
    pub minecart_moves_ignored: usize,
    #[serde(default)]
    pub minecart_lerp_steps_received: usize,
    #[serde(default)]
    pub minecart_lerp_steps_tracked: usize,
    pub transfer_packets: usize,
    pub take_item_entity_packets: usize,
    #[serde(default)]
    pub take_item_entities_applied: usize,
    #[serde(default)]
    pub take_item_entities_ignored: usize,
    #[serde(default)]
    pub item_entity_stack_shrinks: usize,
    #[serde(default)]
    pub take_item_entities_removed: usize,
    pub custom_chat_completion_packets: usize,
    #[serde(default)]
    pub custom_chat_completions_tracked: usize,
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
    #[serde(default)]
    pub map_item_data_packets: usize,
    #[serde(default)]
    pub maps_tracked: usize,
    #[serde(default)]
    pub map_decorations_tracked: usize,
    #[serde(default)]
    pub map_color_patches_applied: usize,
    #[serde(default)]
    pub map_color_patches_ignored: usize,
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
    #[serde(default)]
    pub projectile_power_updates_applied: usize,
    #[serde(default)]
    pub projectile_power_updates_ignored: usize,
    pub debug_block_value_packets: usize,
    pub debug_chunk_value_packets: usize,
    pub debug_entity_value_packets: usize,
    pub debug_event_packets: usize,
    pub debug_sample_packets: usize,
    pub game_rule_value_packets: usize,
    pub game_test_highlight_pos_packets: usize,
    pub test_instance_block_status_packets: usize,
    pub select_advancements_tab_packets: usize,
    #[serde(default)]
    pub update_advancements_packets: usize,
    #[serde(default)]
    pub update_advancements_reset_packets: usize,
    #[serde(default)]
    pub update_advancements_show_packets: usize,
    #[serde(default)]
    pub advancements_added_received: usize,
    #[serde(default)]
    pub advancements_removed_received: usize,
    #[serde(default)]
    pub advancements_adds_ignored: usize,
    #[serde(default)]
    pub advancement_progress_received: usize,
    #[serde(default)]
    pub advancement_progress_updates_ignored: usize,
    #[serde(default)]
    pub advancements_tracked: usize,
    #[serde(default)]
    pub advancement_roots_tracked: usize,
    #[serde(default)]
    pub advancement_progress_tracked: usize,
    #[serde(default)]
    pub advancement_progress_criteria_tracked: usize,
    pub tag_query_packets: usize,
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
