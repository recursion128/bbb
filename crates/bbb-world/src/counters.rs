use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldCounters {
    pub registries_seen: usize,
    pub play_logins_received: usize,
    pub respawns_received: usize,
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
    pub command_suggestion_packets: usize,
    #[serde(default)]
    pub command_suggestion_entries_tracked: usize,
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
