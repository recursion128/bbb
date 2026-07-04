use serde::{Deserialize, Serialize};

use crate::WorldStore;

/// Generates the [`WorldCounters`] struct from a compact block list.
///
/// The struct is pure diagnostic-counter boilerplate: hundreds of
/// `#[serde(default)] pub <name>: usize` fields, many of them
/// `<prefix>_received / <prefix>_applied / <prefix>_ignored` triples. This
/// macro factors out the repeated attribute + type noise while keeping every
/// serde field name, `#[serde(default)]` attribute, and declaration order
/// byte-for-byte identical to the hand-written struct (the serialized shape is
/// part of a deterministic snapshot and must never change).
///
/// Block forms (consumed in order, so field order is preserved):
/// - `plain { a, b, .. }` -> `pub a: usize,` (no `#[serde(default)]`)
/// - `with_default { a, b, .. }` -> `#[serde(default)] pub a: usize,`
/// - `verbatim { <tokens> }` -> emitted as written (non-`usize` / irregular fields)
/// - `triple r, a, i;` -> received/applied plain + ignored `#[serde(default)]`
/// - `triple_all r, a, i;` -> all three `#[serde(default)]`
macro_rules! world_counters {
    // Terminal: every block consumed, emit the accumulated struct.
    (@emit [$($fields:tt)*]) => {
        #[derive(Debug, Clone, Default, Serialize, Deserialize)]
        pub struct WorldCounters {
            $($fields)*
        }
    };

    (@emit [$($fields:tt)*] plain { $($name:ident),* $(,)? } $($rest:tt)*) => {
        world_counters! {
            @emit [ $($fields)* $( pub $name: usize, )* ] $($rest)*
        }
    };

    (@emit [$($fields:tt)*] with_default { $($name:ident),* $(,)? } $($rest:tt)*) => {
        world_counters! {
            @emit [ $($fields)* $( #[serde(default)] pub $name: usize, )* ] $($rest)*
        }
    };

    (@emit [$($fields:tt)*] verbatim { $($tok:tt)* } $($rest:tt)*) => {
        world_counters! {
            @emit [ $($fields)* $($tok)* ] $($rest)*
        }
    };

    (@emit [$($fields:tt)*] triple $r:ident, $a:ident, $i:ident; $($rest:tt)*) => {
        world_counters! {
            @emit [
                $($fields)*
                pub $r: usize,
                pub $a: usize,
                #[serde(default)] pub $i: usize,
            ] $($rest)*
        }
    };

    (@emit [$($fields:tt)*] triple_all $r:ident, $a:ident, $i:ident; $($rest:tt)*) => {
        world_counters! {
            @emit [
                $($fields)*
                #[serde(default)] pub $r: usize,
                #[serde(default)] pub $a: usize,
                #[serde(default)] pub $i: usize,
            ] $($rest)*
        }
    };

    // Public entry point: prime the accumulator and start consuming blocks.
    ($($spec:tt)*) => {
        world_counters! { @emit [] $($spec)* }
    };
}

world_counters! {
    plain { registries_seen }
    with_default { registry_entries_seen, registry_entries_with_data, registry_entry_stubs, registry_entry_payload_bytes, registry_content_registries_tracked, registry_content_packets_tracked, registry_content_entries_tracked, registry_duplicate_entries, registry_duplicate_entry_ids_tracked }
    verbatim { #[serde(default)] pub last_registry_data_registry: Option<String>, }
    with_default { last_registry_data_entry_count }
    plain { play_logins_received, respawns_received }
    with_default { world_time_packets, game_event_packets, update_enabled_features_packets, enabled_features_tracked, enabled_features_ignored, select_known_packs_packets, known_packs_offered, known_packs_selected, ticking_state_packets, ticking_step_packets, code_of_conduct_packets, last_code_of_conduct_len, player_abilities_packets, player_health_packets, player_experience_packets, held_slot_packets, held_slot_updates_applied, held_slot_updates_ignored, default_spawn_position_packets, simulation_distance_packets, set_camera_packets, set_camera_updates_applied, set_camera_updates_ignored, player_position_packets, player_rotation_packets, player_look_at_packets, system_chat_packets, action_bar_packets, title_text_packets, subtitle_text_packets, clear_titles_packets, titles_animation_packets }
    plain { chunks_received, chunks_decoded, sections_decoded, block_entities_seen }
    triple block_entity_updates_received, block_entity_updates_applied, block_entity_updates_ignored;
    plain { light_arrays_seen }
    triple light_updates_received, light_updates_applied, light_updates_ignored;
    triple biome_updates_received, biome_updates_applied, biome_updates_ignored;
    with_default { world_apply_errors }
    triple block_updates_received, block_updates_applied, block_updates_ignored;
    with_default { chunk_cache_center_updates_received, chunk_cache_radius_updates_received, block_destructions_received, block_destructions_tracked, block_destructions_removed, block_destructions_expired, block_destructions_ignored, block_changed_ack_packets, local_block_predictions_created, local_block_predictions_reconciled_by_ack, local_block_predictions_reconciled_by_update, local_block_predictions_failed, local_block_predictions_tracked, block_events_received, block_events_tracked, level_events_received, level_events_tracked, world_border_initializes_received, world_border_center_updates_received, world_border_lerp_size_updates_received, world_border_size_updates_received, world_border_warning_delay_updates_received, world_border_warning_distance_updates_received, reset_score_packets, reset_score_updates_applied, reset_score_updates_ignored, set_display_objective_packets, set_display_objective_updates_applied, set_display_objective_updates_ignored, set_objective_packets, set_objective_updates_applied, set_objective_updates_ignored, set_player_team_packets, set_player_team_updates_applied, set_player_team_updates_ignored, set_score_packets, set_score_updates_applied, set_score_updates_ignored, boss_event_packets, boss_bars_tracked, boss_events_ignored, tab_list_packets, change_difficulty_packets, player_info_update_packets, player_info_remove_packets, player_info_entries_tracked, listed_players_tracked, server_data_packets, resource_pack_push_packets, resource_pack_response_packets, resource_pack_response_updates_applied, resource_pack_response_updates_ignored, resource_pack_required_declines, resource_pack_pop_packets, resource_pack_pop_updates_applied, resource_pack_pop_updates_ignored, resource_packs_tracked, server_link_packets, server_link_invalid_entries, server_links_tracked, custom_payload_packets, custom_payload_brand_packets, custom_payload_unknown_packets, custom_report_detail_packets, custom_report_details_tracked, cookie_request_packets, cookie_response_hits, cookie_response_misses, store_cookie_packets, stored_cookie_bytes, transfer_packets, award_stats_packets, award_stats_entries_received, last_award_stats_entry_count, stats_tracked, cooldown_packets, cooldowns_tracked, update_mob_effect_packets, update_mob_effects_ignored, remove_mob_effect_packets, remove_mob_effects_ignored, active_mob_effects_tracked, damage_event_packets, damage_events_applied, damage_events_ignored, command_tree_packets, command_nodes_tracked, command_literal_nodes_tracked, command_argument_nodes_tracked, command_redirect_nodes_tracked, command_executable_nodes_tracked, command_restricted_nodes_tracked }
    verbatim { #[serde(default)] pub last_command_root_index: Option<i32>, }
    with_default { command_suggestion_packets, command_suggestion_entries_tracked, custom_chat_completion_packets, custom_chat_completions_tracked, recipe_book_add_packets, recipe_book_remove_packets, recipe_book_settings_packets, recipe_book_replace_packets, recipe_book_entries_received, recipe_book_removed_entries_received, recipe_book_entries_tracked, recipe_book_highlights_tracked, recipe_book_notifications_received, update_recipes_packets, recipe_property_sets_tracked, recipe_property_set_items_tracked, stonecutter_recipes_tracked, select_advancements_tab_packets, update_advancements_packets, update_advancements_reset_packets, update_advancements_show_packets, advancements_added_received, advancements_removed_received, advancements_adds_ignored, advancement_progress_received, advancement_progress_updates_ignored, advancements_tracked, advancement_roots_tracked, advancement_progress_tracked, advancement_progress_criteria_tracked, reset_chat_packets, player_chat_packets, disguised_chat_packets, delete_chat_packets, chat_messages_tracked, deleted_chat_messages_tracked, chat_signature_cache_entries, player_chat_index_mismatches, chat_unknown_packed_signatures, player_chat_unsigned_content_packets, player_chat_filtered_packets, player_chat_fully_filtered_packets, player_chat_acknowledgement_packets, player_chat_acknowledgement_pending_offset, player_combat_end_packets, player_combat_enter_packets, player_combat_kill_packets, low_disk_space_warnings, clear_dialog_packets, show_dialog_packets, mount_screen_open_packets, open_book_packets, open_sign_editor_packets, ghost_recipe_packets, pong_response_packets, sound_packets, sound_entity_packets, sound_entity_events_applied, sound_entity_events_ignored, stop_sound_packets, explosion_packets, level_particles_packets, projectile_power_packets, projectile_power_updates_applied, projectile_power_updates_ignored, debug_block_value_packets, debug_chunk_value_packets, debug_entity_value_packets, debug_event_packets, debug_sample_packets, game_rule_value_packets, game_test_highlight_pos_packets, test_instance_block_status_packets, waypoint_packets, waypoints_tracked, waypoint_updates_applied, waypoint_updates_ignored, waypoint_untracks_ignored, tag_query_packets, map_item_data_packets, maps_tracked, map_decorations_tracked, map_color_patches_applied, map_color_patches_ignored, update_tags_packets, last_update_tags_registry_count, last_update_tags_total_tag_count, last_update_tags_total_value_count, tag_registries_tracked, tags_tracked, tag_entries_tracked }
    plain { chunk_forgets_received, chunks_forgotten }
    with_default { chunk_forgets_ignored }
    plain { inventory_slot_updates_received, inventory_slots_tracked, cursor_item_updates_received, container_open_updates_received, container_content_updates_received, container_slot_updates_received, container_data_updates_received, container_close_updates_received }
    with_default { container_close_updates_applied, container_close_updates_ignored }
    triple_all merchant_offer_packets_received, merchant_offer_packets_applied, merchant_offer_packets_ignored;
    with_default { merchant_offers_tracked }
    plain { entities_tracked, entities_received }
    triple entity_position_syncs_received, entity_position_syncs_applied, entity_position_syncs_ignored;
    triple entity_moves_received, entity_moves_applied, entity_moves_ignored;
    triple entity_teleports_received, entity_teleports_applied, entity_teleports_ignored;
    triple entity_animation_updates_received, entity_animation_updates_applied, entity_animation_updates_ignored;
    triple entity_events_received, entity_events_applied, entity_events_ignored;
    triple entity_hurt_animations_received, entity_hurt_animations_applied, entity_hurt_animations_ignored;
    plain { entity_data_updates_received, entity_data_values_received, entity_data_updates_applied }
    with_default { entity_data_updates_ignored }
    plain { entity_equipment_updates_received, entity_equipment_slots_received, entity_equipment_updates_applied }
    with_default { entity_equipment_updates_ignored }
    plain { entity_attribute_updates_received, entity_attributes_received, entity_attribute_updates_applied }
    with_default { entity_attribute_updates_ignored }
    plain { entity_passenger_updates_received, entity_passenger_ids_received, entity_passenger_updates_applied }
    with_default { entity_passenger_updates_ignored, vehicle_moves_received, vehicle_moves_applied, vehicle_moves_acked, vehicle_moves_snapped, vehicle_moves_ignored }
    triple_all minecart_moves_received, minecart_moves_applied, minecart_moves_ignored;
    with_default { minecart_lerp_steps_received, minecart_lerp_steps_tracked }
    triple entity_link_updates_received, entity_link_updates_applied, entity_link_updates_ignored;
    triple entity_motion_updates_received, entity_motion_updates_applied, entity_motion_updates_ignored;
    triple entity_head_rotations_received, entity_head_rotations_applied, entity_head_rotations_ignored;
    triple take_item_entities_received, take_item_entities_applied, take_item_entities_ignored;
    plain { item_entity_stack_shrinks, take_item_entities_removed, entity_removes_received, entities_removed }
    with_default { entity_removes_ignored }
}

impl WorldStore {
    pub fn counters(&self) -> WorldCounters {
        self.counters.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::WorldCounters;

    /// Every serde field name of `WorldCounters`, in declaration order.
    ///
    /// This is the load-bearing guard for the `world_counters!` expansion: the
    /// serialized field names, their count, and their order are part of a
    /// deterministic snapshot and must never change. Any dropped, renamed,
    /// reordered, or extra field breaks one of the tests below.
    const EXPECTED_FIELDS: &[&str] = &[
        "registries_seen",
        "registry_entries_seen",
        "registry_entries_with_data",
        "registry_entry_stubs",
        "registry_entry_payload_bytes",
        "registry_content_registries_tracked",
        "registry_content_packets_tracked",
        "registry_content_entries_tracked",
        "registry_duplicate_entries",
        "registry_duplicate_entry_ids_tracked",
        "last_registry_data_registry",
        "last_registry_data_entry_count",
        "play_logins_received",
        "respawns_received",
        "world_time_packets",
        "game_event_packets",
        "update_enabled_features_packets",
        "enabled_features_tracked",
        "enabled_features_ignored",
        "select_known_packs_packets",
        "known_packs_offered",
        "known_packs_selected",
        "ticking_state_packets",
        "ticking_step_packets",
        "code_of_conduct_packets",
        "last_code_of_conduct_len",
        "player_abilities_packets",
        "player_health_packets",
        "player_experience_packets",
        "held_slot_packets",
        "held_slot_updates_applied",
        "held_slot_updates_ignored",
        "default_spawn_position_packets",
        "simulation_distance_packets",
        "set_camera_packets",
        "set_camera_updates_applied",
        "set_camera_updates_ignored",
        "player_position_packets",
        "player_rotation_packets",
        "player_look_at_packets",
        "system_chat_packets",
        "action_bar_packets",
        "title_text_packets",
        "subtitle_text_packets",
        "clear_titles_packets",
        "titles_animation_packets",
        "chunks_received",
        "chunks_decoded",
        "sections_decoded",
        "block_entities_seen",
        "block_entity_updates_received",
        "block_entity_updates_applied",
        "block_entity_updates_ignored",
        "light_arrays_seen",
        "light_updates_received",
        "light_updates_applied",
        "light_updates_ignored",
        "biome_updates_received",
        "biome_updates_applied",
        "biome_updates_ignored",
        "world_apply_errors",
        "block_updates_received",
        "block_updates_applied",
        "block_updates_ignored",
        "chunk_cache_center_updates_received",
        "chunk_cache_radius_updates_received",
        "block_destructions_received",
        "block_destructions_tracked",
        "block_destructions_removed",
        "block_destructions_expired",
        "block_destructions_ignored",
        "block_changed_ack_packets",
        "local_block_predictions_created",
        "local_block_predictions_reconciled_by_ack",
        "local_block_predictions_reconciled_by_update",
        "local_block_predictions_failed",
        "local_block_predictions_tracked",
        "block_events_received",
        "block_events_tracked",
        "level_events_received",
        "level_events_tracked",
        "world_border_initializes_received",
        "world_border_center_updates_received",
        "world_border_lerp_size_updates_received",
        "world_border_size_updates_received",
        "world_border_warning_delay_updates_received",
        "world_border_warning_distance_updates_received",
        "reset_score_packets",
        "reset_score_updates_applied",
        "reset_score_updates_ignored",
        "set_display_objective_packets",
        "set_display_objective_updates_applied",
        "set_display_objective_updates_ignored",
        "set_objective_packets",
        "set_objective_updates_applied",
        "set_objective_updates_ignored",
        "set_player_team_packets",
        "set_player_team_updates_applied",
        "set_player_team_updates_ignored",
        "set_score_packets",
        "set_score_updates_applied",
        "set_score_updates_ignored",
        "boss_event_packets",
        "boss_bars_tracked",
        "boss_events_ignored",
        "tab_list_packets",
        "change_difficulty_packets",
        "player_info_update_packets",
        "player_info_remove_packets",
        "player_info_entries_tracked",
        "listed_players_tracked",
        "server_data_packets",
        "resource_pack_push_packets",
        "resource_pack_response_packets",
        "resource_pack_response_updates_applied",
        "resource_pack_response_updates_ignored",
        "resource_pack_required_declines",
        "resource_pack_pop_packets",
        "resource_pack_pop_updates_applied",
        "resource_pack_pop_updates_ignored",
        "resource_packs_tracked",
        "server_link_packets",
        "server_link_invalid_entries",
        "server_links_tracked",
        "custom_payload_packets",
        "custom_payload_brand_packets",
        "custom_payload_unknown_packets",
        "custom_report_detail_packets",
        "custom_report_details_tracked",
        "cookie_request_packets",
        "cookie_response_hits",
        "cookie_response_misses",
        "store_cookie_packets",
        "stored_cookie_bytes",
        "transfer_packets",
        "award_stats_packets",
        "award_stats_entries_received",
        "last_award_stats_entry_count",
        "stats_tracked",
        "cooldown_packets",
        "cooldowns_tracked",
        "update_mob_effect_packets",
        "update_mob_effects_ignored",
        "remove_mob_effect_packets",
        "remove_mob_effects_ignored",
        "active_mob_effects_tracked",
        "damage_event_packets",
        "damage_events_applied",
        "damage_events_ignored",
        "command_tree_packets",
        "command_nodes_tracked",
        "command_literal_nodes_tracked",
        "command_argument_nodes_tracked",
        "command_redirect_nodes_tracked",
        "command_executable_nodes_tracked",
        "command_restricted_nodes_tracked",
        "last_command_root_index",
        "command_suggestion_packets",
        "command_suggestion_entries_tracked",
        "custom_chat_completion_packets",
        "custom_chat_completions_tracked",
        "recipe_book_add_packets",
        "recipe_book_remove_packets",
        "recipe_book_settings_packets",
        "recipe_book_replace_packets",
        "recipe_book_entries_received",
        "recipe_book_removed_entries_received",
        "recipe_book_entries_tracked",
        "recipe_book_highlights_tracked",
        "recipe_book_notifications_received",
        "update_recipes_packets",
        "recipe_property_sets_tracked",
        "recipe_property_set_items_tracked",
        "stonecutter_recipes_tracked",
        "select_advancements_tab_packets",
        "update_advancements_packets",
        "update_advancements_reset_packets",
        "update_advancements_show_packets",
        "advancements_added_received",
        "advancements_removed_received",
        "advancements_adds_ignored",
        "advancement_progress_received",
        "advancement_progress_updates_ignored",
        "advancements_tracked",
        "advancement_roots_tracked",
        "advancement_progress_tracked",
        "advancement_progress_criteria_tracked",
        "reset_chat_packets",
        "player_chat_packets",
        "disguised_chat_packets",
        "delete_chat_packets",
        "chat_messages_tracked",
        "deleted_chat_messages_tracked",
        "chat_signature_cache_entries",
        "player_chat_index_mismatches",
        "chat_unknown_packed_signatures",
        "player_chat_unsigned_content_packets",
        "player_chat_filtered_packets",
        "player_chat_fully_filtered_packets",
        "player_chat_acknowledgement_packets",
        "player_chat_acknowledgement_pending_offset",
        "player_combat_end_packets",
        "player_combat_enter_packets",
        "player_combat_kill_packets",
        "low_disk_space_warnings",
        "clear_dialog_packets",
        "show_dialog_packets",
        "mount_screen_open_packets",
        "open_book_packets",
        "open_sign_editor_packets",
        "ghost_recipe_packets",
        "pong_response_packets",
        "sound_packets",
        "sound_entity_packets",
        "sound_entity_events_applied",
        "sound_entity_events_ignored",
        "stop_sound_packets",
        "explosion_packets",
        "level_particles_packets",
        "projectile_power_packets",
        "projectile_power_updates_applied",
        "projectile_power_updates_ignored",
        "debug_block_value_packets",
        "debug_chunk_value_packets",
        "debug_entity_value_packets",
        "debug_event_packets",
        "debug_sample_packets",
        "game_rule_value_packets",
        "game_test_highlight_pos_packets",
        "test_instance_block_status_packets",
        "waypoint_packets",
        "waypoints_tracked",
        "waypoint_updates_applied",
        "waypoint_updates_ignored",
        "waypoint_untracks_ignored",
        "tag_query_packets",
        "map_item_data_packets",
        "maps_tracked",
        "map_decorations_tracked",
        "map_color_patches_applied",
        "map_color_patches_ignored",
        "update_tags_packets",
        "last_update_tags_registry_count",
        "last_update_tags_total_tag_count",
        "last_update_tags_total_value_count",
        "tag_registries_tracked",
        "tags_tracked",
        "tag_entries_tracked",
        "chunk_forgets_received",
        "chunks_forgotten",
        "chunk_forgets_ignored",
        "inventory_slot_updates_received",
        "inventory_slots_tracked",
        "cursor_item_updates_received",
        "container_open_updates_received",
        "container_content_updates_received",
        "container_slot_updates_received",
        "container_data_updates_received",
        "container_close_updates_received",
        "container_close_updates_applied",
        "container_close_updates_ignored",
        "merchant_offer_packets_received",
        "merchant_offer_packets_applied",
        "merchant_offer_packets_ignored",
        "merchant_offers_tracked",
        "entities_tracked",
        "entities_received",
        "entity_position_syncs_received",
        "entity_position_syncs_applied",
        "entity_position_syncs_ignored",
        "entity_moves_received",
        "entity_moves_applied",
        "entity_moves_ignored",
        "entity_teleports_received",
        "entity_teleports_applied",
        "entity_teleports_ignored",
        "entity_animation_updates_received",
        "entity_animation_updates_applied",
        "entity_animation_updates_ignored",
        "entity_events_received",
        "entity_events_applied",
        "entity_events_ignored",
        "entity_hurt_animations_received",
        "entity_hurt_animations_applied",
        "entity_hurt_animations_ignored",
        "entity_data_updates_received",
        "entity_data_values_received",
        "entity_data_updates_applied",
        "entity_data_updates_ignored",
        "entity_equipment_updates_received",
        "entity_equipment_slots_received",
        "entity_equipment_updates_applied",
        "entity_equipment_updates_ignored",
        "entity_attribute_updates_received",
        "entity_attributes_received",
        "entity_attribute_updates_applied",
        "entity_attribute_updates_ignored",
        "entity_passenger_updates_received",
        "entity_passenger_ids_received",
        "entity_passenger_updates_applied",
        "entity_passenger_updates_ignored",
        "vehicle_moves_received",
        "vehicle_moves_applied",
        "vehicle_moves_acked",
        "vehicle_moves_snapped",
        "vehicle_moves_ignored",
        "minecart_moves_received",
        "minecart_moves_applied",
        "minecart_moves_ignored",
        "minecart_lerp_steps_received",
        "minecart_lerp_steps_tracked",
        "entity_link_updates_received",
        "entity_link_updates_applied",
        "entity_link_updates_ignored",
        "entity_motion_updates_received",
        "entity_motion_updates_applied",
        "entity_motion_updates_ignored",
        "entity_head_rotations_received",
        "entity_head_rotations_applied",
        "entity_head_rotations_ignored",
        "take_item_entities_received",
        "take_item_entities_applied",
        "take_item_entities_ignored",
        "item_entity_stack_shrinks",
        "take_item_entities_removed",
        "entity_removes_received",
        "entities_removed",
        "entity_removes_ignored",
    ];

    #[test]
    fn world_counters_field_count_and_names_are_stable() {
        let value = serde_json::to_value(WorldCounters::default()).unwrap();
        let obj = value
            .as_object()
            .expect("WorldCounters serializes to a JSON object");
        // Total field count must stay at 328 (guards dropped / extra fields).
        assert_eq!(obj.len(), 328, "WorldCounters field count changed");
        assert_eq!(obj.len(), EXPECTED_FIELDS.len());
        // Exact name set (guards renames).
        for name in EXPECTED_FIELDS {
            assert!(obj.contains_key(*name), "missing serde field: {name}");
        }
    }

    #[test]
    fn world_counters_field_order_is_stable() {
        // `to_string` serializes struct fields in declaration order; assert the
        // on-the-wire key order is byte-identical to the pre-macro layout.
        let json = serde_json::to_string(&WorldCounters::default()).unwrap();
        let mut last = 0usize;
        for name in EXPECTED_FIELDS {
            let needle = format!("\"{name}\":");
            let pos = json
                .find(&needle)
                .unwrap_or_else(|| panic!("missing serde field: {name}"));
            assert!(pos >= last, "serde field out of declaration order: {name}");
            last = pos;
        }
    }
}
