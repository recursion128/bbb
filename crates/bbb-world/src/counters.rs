use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldCounters {
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
    pub play_logins_received: usize,
    pub respawns_received: usize,
    #[serde(default)]
    pub world_time_packets: usize,
    #[serde(default)]
    pub game_event_packets: usize,
    #[serde(default)]
    pub ticking_state_packets: usize,
    #[serde(default)]
    pub ticking_step_packets: usize,
    #[serde(default)]
    pub player_abilities_packets: usize,
    #[serde(default)]
    pub player_health_packets: usize,
    #[serde(default)]
    pub player_experience_packets: usize,
    #[serde(default)]
    pub held_slot_packets: usize,
    #[serde(default)]
    pub default_spawn_position_packets: usize,
    #[serde(default)]
    pub simulation_distance_packets: usize,
    #[serde(default)]
    pub set_camera_packets: usize,
    pub chunks_received: usize,
    pub chunks_decoded: usize,
    pub sections_decoded: usize,
    pub block_entities_seen: usize,
    pub block_entity_updates_received: usize,
    pub block_entity_updates_applied: usize,
    pub light_arrays_seen: usize,
    pub light_updates_received: usize,
    pub light_updates_applied: usize,
    pub biome_updates_received: usize,
    pub biome_updates_applied: usize,
    pub block_updates_received: usize,
    pub block_updates_applied: usize,
    #[serde(default)]
    pub chunk_cache_center_updates_received: usize,
    #[serde(default)]
    pub chunk_cache_radius_updates_received: usize,
    #[serde(default)]
    pub block_destructions_received: usize,
    #[serde(default)]
    pub block_destructions_tracked: usize,
    #[serde(default)]
    pub block_destructions_removed: usize,
    #[serde(default)]
    pub block_events_received: usize,
    #[serde(default)]
    pub block_events_tracked: usize,
    #[serde(default)]
    pub level_events_received: usize,
    #[serde(default)]
    pub level_events_tracked: usize,
    #[serde(default)]
    pub world_border_initializes_received: usize,
    #[serde(default)]
    pub world_border_center_updates_received: usize,
    #[serde(default)]
    pub world_border_lerp_size_updates_received: usize,
    #[serde(default)]
    pub world_border_size_updates_received: usize,
    #[serde(default)]
    pub world_border_warning_delay_updates_received: usize,
    #[serde(default)]
    pub world_border_warning_distance_updates_received: usize,
    #[serde(default)]
    pub reset_score_packets: usize,
    #[serde(default)]
    pub set_display_objective_packets: usize,
    #[serde(default)]
    pub set_objective_packets: usize,
    #[serde(default)]
    pub set_player_team_packets: usize,
    #[serde(default)]
    pub set_score_packets: usize,
    #[serde(default)]
    pub boss_event_packets: usize,
    #[serde(default)]
    pub boss_bars_tracked: usize,
    #[serde(default)]
    pub tab_list_packets: usize,
    #[serde(default)]
    pub change_difficulty_packets: usize,
    #[serde(default)]
    pub player_info_update_packets: usize,
    #[serde(default)]
    pub player_info_remove_packets: usize,
    #[serde(default)]
    pub player_info_entries_tracked: usize,
    #[serde(default)]
    pub listed_players_tracked: usize,
    #[serde(default)]
    pub server_data_packets: usize,
    #[serde(default)]
    pub resource_pack_push_packets: usize,
    #[serde(default)]
    pub resource_pack_pop_packets: usize,
    #[serde(default)]
    pub resource_packs_tracked: usize,
    #[serde(default)]
    pub server_link_packets: usize,
    #[serde(default)]
    pub server_link_invalid_entries: usize,
    #[serde(default)]
    pub server_links_tracked: usize,
    #[serde(default)]
    pub custom_payload_packets: usize,
    #[serde(default)]
    pub custom_payload_brand_packets: usize,
    #[serde(default)]
    pub custom_payload_unknown_packets: usize,
    #[serde(default)]
    pub transfer_packets: usize,
    #[serde(default)]
    pub cooldown_packets: usize,
    #[serde(default)]
    pub cooldowns_tracked: usize,
    #[serde(default)]
    pub update_mob_effect_packets: usize,
    #[serde(default)]
    pub remove_mob_effect_packets: usize,
    #[serde(default)]
    pub active_mob_effects_tracked: usize,
    #[serde(default)]
    pub damage_event_packets: usize,
    #[serde(default)]
    pub damage_events_applied: usize,
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
    #[serde(default)]
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
    #[serde(default)]
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
    #[serde(default)]
    pub player_chat_packets: usize,
    #[serde(default)]
    pub disguised_chat_packets: usize,
    #[serde(default)]
    pub delete_chat_packets: usize,
    #[serde(default)]
    pub chat_messages_tracked: usize,
    #[serde(default)]
    pub deleted_chat_messages_tracked: usize,
    #[serde(default)]
    pub chat_signature_cache_entries: usize,
    #[serde(default)]
    pub player_chat_index_mismatches: usize,
    #[serde(default)]
    pub chat_unknown_packed_signatures: usize,
    #[serde(default)]
    pub player_chat_unsigned_content_packets: usize,
    #[serde(default)]
    pub player_chat_filtered_packets: usize,
    #[serde(default)]
    pub player_chat_fully_filtered_packets: usize,
    #[serde(default)]
    pub low_disk_space_warnings: usize,
    #[serde(default)]
    pub clear_dialog_packets: usize,
    #[serde(default)]
    pub show_dialog_packets: usize,
    #[serde(default)]
    pub mount_screen_open_packets: usize,
    #[serde(default)]
    pub open_book_packets: usize,
    #[serde(default)]
    pub open_sign_editor_packets: usize,
    #[serde(default)]
    pub ghost_recipe_packets: usize,
    #[serde(default)]
    pub pong_response_packets: usize,
    #[serde(default)]
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
    pub chunk_forgets_received: usize,
    pub chunks_forgotten: usize,
    pub inventory_slot_updates_received: usize,
    pub inventory_slots_tracked: usize,
    pub cursor_item_updates_received: usize,
    pub container_open_updates_received: usize,
    pub container_content_updates_received: usize,
    pub container_slot_updates_received: usize,
    pub container_data_updates_received: usize,
    pub container_close_updates_received: usize,
    #[serde(default)]
    pub merchant_offer_packets_received: usize,
    #[serde(default)]
    pub merchant_offer_packets_applied: usize,
    #[serde(default)]
    pub merchant_offer_packets_ignored: usize,
    #[serde(default)]
    pub merchant_offers_tracked: usize,
    pub entities_tracked: usize,
    pub entities_received: usize,
    pub entity_position_syncs_received: usize,
    pub entity_position_syncs_applied: usize,
    pub entity_moves_received: usize,
    pub entity_moves_applied: usize,
    pub entity_teleports_received: usize,
    pub entity_teleports_applied: usize,
    pub entity_animation_updates_received: usize,
    pub entity_animation_updates_applied: usize,
    pub entity_events_received: usize,
    pub entity_events_applied: usize,
    pub entity_hurt_animations_received: usize,
    pub entity_hurt_animations_applied: usize,
    pub entity_data_updates_received: usize,
    pub entity_data_values_received: usize,
    pub entity_data_updates_applied: usize,
    pub entity_equipment_updates_received: usize,
    pub entity_equipment_slots_received: usize,
    pub entity_equipment_updates_applied: usize,
    pub entity_attribute_updates_received: usize,
    pub entity_attributes_received: usize,
    pub entity_attribute_updates_applied: usize,
    pub entity_passenger_updates_received: usize,
    pub entity_passenger_ids_received: usize,
    pub entity_passenger_updates_applied: usize,
    #[serde(default)]
    pub vehicle_moves_received: usize,
    #[serde(default)]
    pub vehicle_moves_applied: usize,
    #[serde(default)]
    pub vehicle_moves_acked: usize,
    #[serde(default)]
    pub vehicle_moves_snapped: usize,
    #[serde(default)]
    pub minecart_moves_received: usize,
    #[serde(default)]
    pub minecart_moves_applied: usize,
    #[serde(default)]
    pub minecart_lerp_steps_received: usize,
    #[serde(default)]
    pub minecart_lerp_steps_tracked: usize,
    pub entity_link_updates_received: usize,
    pub entity_link_updates_applied: usize,
    pub entity_motion_updates_received: usize,
    pub entity_motion_updates_applied: usize,
    pub entity_head_rotations_received: usize,
    pub entity_head_rotations_applied: usize,
    pub take_item_entities_received: usize,
    pub take_item_entities_applied: usize,
    pub item_entity_stack_shrinks: usize,
    pub take_item_entities_removed: usize,
    pub entity_removes_received: usize,
    pub entities_removed: usize,
}

impl WorldStore {
    pub fn counters(&self) -> WorldCounters {
        self.counters.clone()
    }
}
