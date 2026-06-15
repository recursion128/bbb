use bbb_control::NetCounters;
use bbb_net::NetEvent;
use bbb_world::WorldStore;

use super::client_state::apply_player_look_at_update;

pub(super) fn apply_control_projection_event(
    event: NetEvent,
    counters: &mut NetCounters,
    world: &mut WorldStore,
) -> Option<NetEvent> {
    match event {
        NetEvent::Connected => {
            counters.connected = true;
            counters.last_error = None;
        }
        NetEvent::Disconnected { reason } => {
            counters.connected = false;
            counters.last_error = reason;
        }
        NetEvent::StateChanged { state } => {
            counters.state = Some(format!("{state:?}"));
        }
        NetEvent::CompressionSet { threshold } => {
            counters.compression_threshold = Some(threshold);
        }
        NetEvent::CookieRequest {
            key,
            response_payload_present,
        } => {
            world.apply_cookie_request(key, response_payload_present);
            sync_cookie_counters(counters, world);
        }
        NetEvent::StoreCookie {
            key,
            payload_len,
            stored_cookie_count,
        } => {
            world.apply_store_cookie(key, payload_len, stored_cookie_count);
            sync_cookie_counters(counters, world);
        }
        NetEvent::CustomReportDetails(details) => {
            world.apply_custom_report_details(details);
            sync_custom_report_detail_counters(counters, world);
        }
        NetEvent::ResetChat => {
            world.apply_reset_chat();
            sync_chat_counters(counters, world);
        }
        NetEvent::UpdateEnabledFeatures(update) => {
            world.apply_update_enabled_features(update);
            sync_enabled_feature_counters(counters, world);
        }
        NetEvent::CodeOfConduct { text } => {
            world.apply_code_of_conduct(text);
            sync_client_ui_counters(counters, world);
        }
        NetEvent::CustomChatCompletions(update) => {
            world.apply_custom_chat_completions(update);
            sync_custom_chat_completion_counters(counters, world);
        }
        NetEvent::CustomPayload(update) => {
            world.apply_custom_payload(update);
            sync_custom_payload_counters(counters, world);
        }
        NetEvent::ServerLinks(links) => {
            world.apply_server_links(links);
            sync_server_link_counters(counters, world);
        }
        NetEvent::AwardStats(update) => {
            world.apply_award_stats(update);
            sync_client_stats_counters(counters, world);
        }
        NetEvent::LowDiskSpaceWarning => {
            world.apply_low_disk_space_warning();
            sync_client_ui_counters(counters, world);
        }
        NetEvent::MapItemData(update) => {
            world.apply_map_item_data(update);
            sync_map_counters(counters, world);
        }
        NetEvent::MountScreenOpen(update) => {
            world.apply_mount_screen_open(update);
            sync_client_ui_counters(counters, world);
        }
        NetEvent::OpenBook(update) => {
            world.apply_open_book(update);
            sync_client_ui_counters(counters, world);
        }
        NetEvent::OpenSignEditor(update) => {
            world.apply_open_sign_editor(update);
            sync_client_ui_counters(counters, world);
        }
        NetEvent::PlaceGhostRecipe(update) => {
            world.apply_place_ghost_recipe(update);
            sync_client_ui_counters(counters, world);
        }
        NetEvent::ClearDialog => {
            world.apply_clear_dialog();
            sync_client_ui_counters(counters, world);
        }
        NetEvent::ShowDialog(update) => {
            world.apply_show_dialog(update);
            sync_client_ui_counters(counters, world);
        }
        NetEvent::Waypoint(update) => {
            world.apply_waypoint(update);
            sync_waypoint_counters(counters, world);
        }
        NetEvent::PlayerCombatEnd(update) => {
            world.apply_player_combat_end(update);
            sync_client_combat_counters(counters, world);
        }
        NetEvent::PlayerCombatEnter => {
            world.apply_player_combat_enter();
            sync_client_combat_counters(counters, world);
        }
        NetEvent::PlayerCombatKill(update) => {
            world.apply_player_combat_kill(update);
            sync_client_combat_counters(counters, world);
        }
        NetEvent::PlayerLookAt(update) => {
            apply_player_look_at_update(counters, world, update);
        }
        NetEvent::PongResponse(update) => {
            world.apply_pong_response(update);
            sync_client_ui_counters(counters, world);
        }
        NetEvent::Explosion(update) => {
            world.apply_explosion(update);
            sync_client_effect_counters(counters, world);
        }
        NetEvent::LevelParticles(update) => {
            world.apply_level_particles(update);
            sync_client_effect_counters(counters, world);
        }
        NetEvent::ProjectilePower(update) => {
            world.apply_projectile_power(update);
            sync_entity_projectile_counters(counters, world);
        }
        NetEvent::DebugBlockValue(update) => {
            world.apply_debug_block_value(update);
            sync_debug_game_counters(counters, world);
        }
        NetEvent::DebugChunkValue(update) => {
            world.apply_debug_chunk_value(update);
            sync_debug_game_counters(counters, world);
        }
        NetEvent::DebugEntityValue(update) => {
            world.apply_debug_entity_value(update);
            sync_debug_game_counters(counters, world);
        }
        NetEvent::DebugEvent(update) => {
            world.apply_debug_event(update);
            sync_debug_game_counters(counters, world);
        }
        NetEvent::DebugSample(update) => {
            world.apply_debug_sample(update);
            sync_debug_game_counters(counters, world);
        }
        NetEvent::DeleteChat(update) => {
            world.apply_delete_chat(update);
            sync_chat_counters(counters, world);
        }
        NetEvent::DisguisedChat(update) => {
            world.apply_disguised_chat(update);
            sync_chat_counters(counters, world);
        }
        NetEvent::PlayerChat(update) => {
            world.apply_player_chat(update);
            sync_chat_counters(counters, world);
        }
        NetEvent::GameRuleValues(update) => {
            world.apply_game_rule_values(update);
            sync_debug_game_counters(counters, world);
        }
        NetEvent::GameTestHighlightPos(update) => {
            world.apply_game_test_highlight_pos(update);
            sync_debug_game_counters(counters, world);
        }
        NetEvent::TestInstanceBlockStatus(update) => {
            world.apply_test_instance_block_status(update);
            sync_debug_game_counters(counters, world);
        }
        NetEvent::Transfer(transfer) => {
            world.apply_transfer(transfer);
            sync_transfer_counters(counters, world);
        }
        NetEvent::PacketSeen { .. } => {
            counters.packets_seen += 1;
        }
        NetEvent::SelectAdvancementsTab(update) => {
            world.apply_select_advancements_tab(update);
            sync_advancement_counters(counters, world);
        }
        NetEvent::TagQuery(update) => {
            world.apply_tag_query(update);
            sync_tag_query_counters(counters, world);
        }
        other => return Some(other),
    }

    None
}

pub(super) fn sync_registry_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.registries_seen = world_counters.registries_seen;
    counters.registry_entries_seen = world_counters.registry_entries_seen;
    counters.registry_entries_with_data = world_counters.registry_entries_with_data;
    counters.registry_entry_stubs = world_counters.registry_entry_stubs;
    counters.registry_entry_payload_bytes = world_counters.registry_entry_payload_bytes;
    counters.registry_content_registries_tracked =
        world_counters.registry_content_registries_tracked;
    counters.registry_content_packets_tracked = world_counters.registry_content_packets_tracked;
    counters.registry_content_entries_tracked = world_counters.registry_content_entries_tracked;
    counters.registry_duplicate_entries = world_counters.registry_duplicate_entries;
    counters.registry_duplicate_entry_ids_tracked =
        world_counters.registry_duplicate_entry_ids_tracked;
    counters.last_registry_data_registry = world_counters.last_registry_data_registry.clone();
    counters.last_registry_data_entry_count = world_counters.last_registry_data_entry_count;
}

pub(super) fn sync_update_tags_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.update_tags_packets = world_counters.update_tags_packets;
    counters.last_update_tags_registry_count = world_counters.last_update_tags_registry_count;
    counters.last_update_tags_total_tag_count = world_counters.last_update_tags_total_tag_count;
    counters.last_update_tags_total_value_count = world_counters.last_update_tags_total_value_count;
    counters.tag_registries_tracked = world_counters.tag_registries_tracked;
    counters.tags_tracked = world_counters.tags_tracked;
    counters.tag_entries_tracked = world_counters.tag_entries_tracked;
}

pub(super) fn sync_player_info_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.player_info_update_packets = world_counters.player_info_update_packets;
    counters.player_info_remove_packets = world_counters.player_info_remove_packets;
    counters.player_info_entries_tracked = world_counters.player_info_entries_tracked;
    counters.listed_players_tracked = world_counters.listed_players_tracked;
}

pub(super) fn sync_server_presentation_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.server_data_packets = world_counters.server_data_packets;
    counters.resource_pack_push_packets = world_counters.resource_pack_push_packets;
    counters.resource_pack_pop_packets = world_counters.resource_pack_pop_packets;
    counters.resource_pack_pop_updates_applied = world_counters.resource_pack_pop_updates_applied;
    counters.resource_pack_pop_updates_ignored = world_counters.resource_pack_pop_updates_ignored;
    counters.resource_packs_tracked = world_counters.resource_packs_tracked;
}

pub(super) fn sync_entity_status_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.cooldown_packets = world_counters.cooldown_packets;
    counters.cooldowns_tracked = world_counters.cooldowns_tracked;
    counters.damage_event_packets = world_counters.damage_event_packets;
    counters.damage_events_applied = world_counters.damage_events_applied;
    counters.damage_events_ignored = world_counters.damage_events_ignored;
    counters.update_mob_effect_packets = world_counters.update_mob_effect_packets;
    counters.update_mob_effects_ignored = world_counters.update_mob_effects_ignored;
    counters.remove_mob_effect_packets = world_counters.remove_mob_effect_packets;
    counters.remove_mob_effects_ignored = world_counters.remove_mob_effects_ignored;
    counters.active_mob_effects_tracked = world_counters.active_mob_effects_tracked;
}

pub(super) fn sync_inventory_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.inventory_slot_updates_received = world_counters.inventory_slot_updates_received;
    counters.inventory_slots_tracked = world_counters.inventory_slots_tracked;
    counters.cursor_item_updates_received = world_counters.cursor_item_updates_received;
    counters.container_open_updates_received = world_counters.container_open_updates_received;
    counters.container_content_updates_received = world_counters.container_content_updates_received;
    counters.container_slot_updates_received = world_counters.container_slot_updates_received;
    counters.container_data_updates_received = world_counters.container_data_updates_received;
    counters.container_close_updates_received = world_counters.container_close_updates_received;
    counters.container_close_updates_applied = world_counters.container_close_updates_applied;
    counters.container_close_updates_ignored = world_counters.container_close_updates_ignored;
    counters.merchant_offer_packets_received = world_counters.merchant_offer_packets_received;
    counters.merchant_offer_packets_applied = world_counters.merchant_offer_packets_applied;
    counters.merchant_offer_packets_ignored = world_counters.merchant_offer_packets_ignored;
    counters.merchant_offers_tracked = world_counters.merchant_offers_tracked;
}

pub(super) fn sync_entity_interaction_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.move_vehicle_packets = world_counters.vehicle_moves_received;
    counters.vehicle_moves_applied = world_counters.vehicle_moves_applied;
    counters.vehicle_moves_acked = world_counters.vehicle_moves_acked;
    counters.vehicle_moves_snapped = world_counters.vehicle_moves_snapped;
    counters.vehicle_moves_ignored = world_counters.vehicle_moves_ignored;
    counters.take_item_entity_packets = world_counters.take_item_entities_received;
    counters.take_item_entities_applied = world_counters.take_item_entities_applied;
    counters.take_item_entities_ignored = world_counters.take_item_entities_ignored;
    counters.item_entity_stack_shrinks = world_counters.item_entity_stack_shrinks;
    counters.take_item_entities_removed = world_counters.take_item_entities_removed;
}

pub(super) fn sync_entity_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.entities_tracked = world_counters.entities_tracked;
    counters.entities_received = world_counters.entities_received;
    counters.entity_position_syncs_received = world_counters.entity_position_syncs_received;
    counters.entity_position_syncs_applied = world_counters.entity_position_syncs_applied;
    counters.entity_position_syncs_ignored = world_counters.entity_position_syncs_ignored;
    counters.entity_moves_received = world_counters.entity_moves_received;
    counters.entity_moves_applied = world_counters.entity_moves_applied;
    counters.entity_moves_ignored = world_counters.entity_moves_ignored;
    counters.entity_teleports_received = world_counters.entity_teleports_received;
    counters.entity_teleports_applied = world_counters.entity_teleports_applied;
    counters.entity_teleports_ignored = world_counters.entity_teleports_ignored;
    counters.entity_animation_updates_received = world_counters.entity_animation_updates_received;
    counters.entity_animation_updates_applied = world_counters.entity_animation_updates_applied;
    counters.entity_animation_updates_ignored = world_counters.entity_animation_updates_ignored;
    counters.entity_events_received = world_counters.entity_events_received;
    counters.entity_events_applied = world_counters.entity_events_applied;
    counters.entity_events_ignored = world_counters.entity_events_ignored;
    counters.entity_hurt_animations_received = world_counters.entity_hurt_animations_received;
    counters.entity_hurt_animations_applied = world_counters.entity_hurt_animations_applied;
    counters.entity_hurt_animations_ignored = world_counters.entity_hurt_animations_ignored;
    counters.entity_data_updates_received = world_counters.entity_data_updates_received;
    counters.entity_data_values_received = world_counters.entity_data_values_received;
    counters.entity_data_updates_applied = world_counters.entity_data_updates_applied;
    counters.entity_data_updates_ignored = world_counters.entity_data_updates_ignored;
    counters.entity_equipment_updates_received = world_counters.entity_equipment_updates_received;
    counters.entity_equipment_slots_received = world_counters.entity_equipment_slots_received;
    counters.entity_equipment_updates_applied = world_counters.entity_equipment_updates_applied;
    counters.entity_equipment_updates_ignored = world_counters.entity_equipment_updates_ignored;
    counters.entity_attribute_updates_received = world_counters.entity_attribute_updates_received;
    counters.entity_attributes_received = world_counters.entity_attributes_received;
    counters.entity_attribute_updates_applied = world_counters.entity_attribute_updates_applied;
    counters.entity_attribute_updates_ignored = world_counters.entity_attribute_updates_ignored;
    counters.entity_passenger_updates_received = world_counters.entity_passenger_updates_received;
    counters.entity_passenger_ids_received = world_counters.entity_passenger_ids_received;
    counters.entity_passenger_updates_applied = world_counters.entity_passenger_updates_applied;
    counters.entity_passenger_updates_ignored = world_counters.entity_passenger_updates_ignored;
    counters.entity_link_updates_received = world_counters.entity_link_updates_received;
    counters.entity_link_updates_applied = world_counters.entity_link_updates_applied;
    counters.entity_link_updates_ignored = world_counters.entity_link_updates_ignored;
    counters.entity_motion_updates_received = world_counters.entity_motion_updates_received;
    counters.entity_motion_updates_applied = world_counters.entity_motion_updates_applied;
    counters.entity_motion_updates_ignored = world_counters.entity_motion_updates_ignored;
    counters.entity_head_rotations_received = world_counters.entity_head_rotations_received;
    counters.entity_head_rotations_applied = world_counters.entity_head_rotations_applied;
    counters.entity_head_rotations_ignored = world_counters.entity_head_rotations_ignored;
    counters.entity_removes_received = world_counters.entity_removes_received;
    counters.entities_removed = world_counters.entities_removed;
    counters.entity_removes_ignored = world_counters.entity_removes_ignored;
    counters.minecart_moves_received = world_counters.minecart_moves_received;
    counters.minecart_moves_applied = world_counters.minecart_moves_applied;
    counters.minecart_moves_ignored = world_counters.minecart_moves_ignored;
    counters.minecart_lerp_steps_received = world_counters.minecart_lerp_steps_received;
    counters.minecart_lerp_steps_tracked = world_counters.minecart_lerp_steps_tracked;
}

pub(super) fn sync_block_event_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.block_changed_ack_packets = world_counters.block_changed_ack_packets;
    counters.last_block_changed_ack_sequence =
        world.last_block_changed_ack().map(|ack| ack.sequence);
    counters.block_destruction_packets = world_counters.block_destructions_received;
    counters.block_destructions_tracked = world_counters.block_destructions_tracked;
    counters.block_destructions_removed = world_counters.block_destructions_removed;
    counters.block_destructions_ignored = world_counters.block_destructions_ignored;
    counters.block_event_packets = world_counters.block_events_received;
    counters.block_events_tracked = world_counters.block_events_tracked;
    counters.level_event_packets = world_counters.level_events_received;
    counters.level_events_tracked = world_counters.level_events_tracked;
}

pub(super) fn sync_hud_session_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.boss_event_packets = world_counters.boss_event_packets;
    counters.boss_bars_tracked = world_counters.boss_bars_tracked;
    counters.boss_events_ignored = world_counters.boss_events_ignored;
    counters.tab_list_packets = world_counters.tab_list_packets;
    counters.change_difficulty_packets = world_counters.change_difficulty_packets;
}

pub(super) fn sync_world_border_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.initialize_border_packets = world_counters.world_border_initializes_received;
    counters.set_border_center_packets = world_counters.world_border_center_updates_received;
    counters.set_border_lerp_size_packets = world_counters.world_border_lerp_size_updates_received;
    counters.set_border_size_packets = world_counters.world_border_size_updates_received;
    counters.set_border_warning_delay_packets =
        world_counters.world_border_warning_delay_updates_received;
    counters.set_border_warning_distance_packets =
        world_counters.world_border_warning_distance_updates_received;
}

pub(super) fn sync_scoreboard_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.reset_score_packets = world_counters.reset_score_packets;
    counters.reset_score_updates_applied = world_counters.reset_score_updates_applied;
    counters.reset_score_updates_ignored = world_counters.reset_score_updates_ignored;
    counters.set_display_objective_packets = world_counters.set_display_objective_packets;
    counters.set_display_objective_updates_applied =
        world_counters.set_display_objective_updates_applied;
    counters.set_display_objective_updates_ignored =
        world_counters.set_display_objective_updates_ignored;
    counters.set_objective_packets = world_counters.set_objective_packets;
    counters.set_objective_updates_applied = world_counters.set_objective_updates_applied;
    counters.set_objective_updates_ignored = world_counters.set_objective_updates_ignored;
    counters.set_player_team_packets = world_counters.set_player_team_packets;
    counters.set_player_team_updates_applied = world_counters.set_player_team_updates_applied;
    counters.set_player_team_updates_ignored = world_counters.set_player_team_updates_ignored;
    counters.set_score_packets = world_counters.set_score_packets;
    counters.set_score_updates_applied = world_counters.set_score_updates_applied;
    counters.set_score_updates_ignored = world_counters.set_score_updates_ignored;
}

pub(super) fn sync_command_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.command_tree_packets = world_counters.command_tree_packets;
    counters.command_nodes_tracked = world_counters.command_nodes_tracked;
    counters.command_literal_nodes_tracked = world_counters.command_literal_nodes_tracked;
    counters.command_argument_nodes_tracked = world_counters.command_argument_nodes_tracked;
    counters.command_redirect_nodes_tracked = world_counters.command_redirect_nodes_tracked;
    counters.command_executable_nodes_tracked = world_counters.command_executable_nodes_tracked;
    counters.command_restricted_nodes_tracked = world_counters.command_restricted_nodes_tracked;
    counters.last_command_root_index = world_counters.last_command_root_index;
    counters.command_suggestion_packets = world_counters.command_suggestion_packets;
    counters.command_suggestion_entries_tracked = world_counters.command_suggestion_entries_tracked;
}

pub(super) fn sync_recipe_book_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.recipe_book_add_packets = world_counters.recipe_book_add_packets;
    counters.recipe_book_remove_packets = world_counters.recipe_book_remove_packets;
    counters.recipe_book_settings_packets = world_counters.recipe_book_settings_packets;
    counters.recipe_book_replace_packets = world_counters.recipe_book_replace_packets;
    counters.recipe_book_entries_received = world_counters.recipe_book_entries_received;
    counters.recipe_book_removed_entries_received =
        world_counters.recipe_book_removed_entries_received;
    counters.recipe_book_entries_tracked = world_counters.recipe_book_entries_tracked;
    counters.recipe_book_highlights_tracked = world_counters.recipe_book_highlights_tracked;
    counters.recipe_book_notifications_received = world_counters.recipe_book_notifications_received;
}

pub(super) fn sync_recipe_access_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.update_recipes_packets = world_counters.update_recipes_packets;
    counters.recipe_property_sets_tracked = world_counters.recipe_property_sets_tracked;
    counters.recipe_property_set_items_tracked = world_counters.recipe_property_set_items_tracked;
    counters.stonecutter_recipes_tracked = world_counters.stonecutter_recipes_tracked;
}

pub(super) fn sync_advancement_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.selected_advancements_tab = world.selected_advancements_tab().map(str::to_string);
    counters.select_advancements_tab_packets = world_counters.select_advancements_tab_packets;
    counters.update_advancements_packets = world_counters.update_advancements_packets;
    counters.update_advancements_reset_packets = world_counters.update_advancements_reset_packets;
    counters.update_advancements_show_packets = world_counters.update_advancements_show_packets;
    counters.advancements_added_received = world_counters.advancements_added_received;
    counters.advancements_removed_received = world_counters.advancements_removed_received;
    counters.advancements_adds_ignored = world_counters.advancements_adds_ignored;
    counters.advancement_progress_received = world_counters.advancement_progress_received;
    counters.advancement_progress_updates_ignored =
        world_counters.advancement_progress_updates_ignored;
    counters.advancements_tracked = world_counters.advancements_tracked;
    counters.advancement_roots_tracked = world_counters.advancement_roots_tracked;
    counters.advancement_progress_tracked = world_counters.advancement_progress_tracked;
    counters.advancement_progress_criteria_tracked =
        world_counters.advancement_progress_criteria_tracked;
}

fn sync_enabled_feature_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.update_enabled_features_packets = world_counters.update_enabled_features_packets;
    counters.enabled_features_tracked = world_counters.enabled_features_tracked;
    counters.enabled_features_ignored = world_counters.enabled_features_ignored;
    counters.enabled_features = world.enabled_feature_list();
}

fn sync_chat_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.player_chat_packets = world_counters.player_chat_packets;
    counters.reset_chat_packets = world_counters.reset_chat_packets;
    counters.disguised_chat_packets = world_counters.disguised_chat_packets;
    counters.delete_chat_packets = world_counters.delete_chat_packets;
    counters.chat_messages_tracked = world_counters.chat_messages_tracked;
    counters.deleted_chat_messages_tracked = world_counters.deleted_chat_messages_tracked;
    counters.chat_signature_cache_entries = world_counters.chat_signature_cache_entries;
    counters.player_chat_index_mismatches = world_counters.player_chat_index_mismatches;
    counters.chat_unknown_packed_signatures = world_counters.chat_unknown_packed_signatures;
    counters.player_chat_unsigned_content_packets =
        world_counters.player_chat_unsigned_content_packets;
    counters.player_chat_filtered_packets = world_counters.player_chat_filtered_packets;
    counters.player_chat_fully_filtered_packets = world_counters.player_chat_fully_filtered_packets;
}

fn sync_custom_payload_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.custom_payload_packets = world_counters.custom_payload_packets;
    counters.custom_payload_brand_packets = world_counters.custom_payload_brand_packets;
    counters.custom_payload_unknown_packets = world_counters.custom_payload_unknown_packets;
}

fn sync_custom_report_detail_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.custom_report_detail_packets = world_counters.custom_report_detail_packets;
    counters.custom_report_details_tracked = world_counters.custom_report_details_tracked;
}

fn sync_cookie_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.cookie_request_packets = world_counters.cookie_request_packets;
    counters.cookie_response_hits = world_counters.cookie_response_hits;
    counters.cookie_response_misses = world_counters.cookie_response_misses;
    counters.store_cookie_packets = world_counters.store_cookie_packets;
    counters.stored_cookie_count = world.stored_cookie_count();
    counters.stored_cookie_bytes = world_counters.stored_cookie_bytes;
}

fn sync_custom_chat_completion_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.custom_chat_completion_packets = world_counters.custom_chat_completion_packets;
    counters.custom_chat_completions_tracked = world_counters.custom_chat_completions_tracked;
    counters.last_custom_chat_completion =
        world.last_custom_chat_completion_update().map(|update| {
            bbb_control::CustomChatCompletionState {
                action: update.action.clone(),
                entries: update.entries,
            }
        });
}

fn sync_tag_query_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.tag_query_packets = world_counters.tag_query_packets;
}

pub(super) fn sync_client_audio_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.sound_packets = world_counters.sound_packets;
    counters.sound_entity_packets = world_counters.sound_entity_packets;
    counters.sound_entity_events_applied = world_counters.sound_entity_events_applied;
    counters.sound_entity_events_ignored = world_counters.sound_entity_events_ignored;
    counters.stop_sound_packets = world_counters.stop_sound_packets;
}

fn sync_client_effect_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.explosion_packets = world_counters.explosion_packets;
    counters.level_particles_packets = world_counters.level_particles_packets;
}

fn sync_entity_projectile_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.projectile_power_packets = world_counters.projectile_power_packets;
    counters.projectile_power_updates_applied = world_counters.projectile_power_updates_applied;
    counters.projectile_power_updates_ignored = world_counters.projectile_power_updates_ignored;
}

fn sync_debug_game_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.debug_block_value_packets = world_counters.debug_block_value_packets;
    counters.debug_chunk_value_packets = world_counters.debug_chunk_value_packets;
    counters.debug_entity_value_packets = world_counters.debug_entity_value_packets;
    counters.debug_event_packets = world_counters.debug_event_packets;
    counters.debug_sample_packets = world_counters.debug_sample_packets;
    counters.game_rule_value_packets = world_counters.game_rule_value_packets;
    counters.game_test_highlight_pos_packets = world_counters.game_test_highlight_pos_packets;
    counters.test_instance_block_status_packets = world_counters.test_instance_block_status_packets;
}

fn sync_client_ui_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.low_disk_space_warnings = world_counters.low_disk_space_warnings;
    counters.clear_dialog_packets = world_counters.clear_dialog_packets;
    counters.show_dialog_packets = world_counters.show_dialog_packets;
    counters.code_of_conduct_packets = world_counters.code_of_conduct_packets;
    counters.last_code_of_conduct_len = world_counters.last_code_of_conduct_len;
    counters.mount_screen_open_packets = world_counters.mount_screen_open_packets;
    counters.open_book_packets = world_counters.open_book_packets;
    counters.open_sign_editor_packets = world_counters.open_sign_editor_packets;
    counters.ghost_recipe_packets = world_counters.ghost_recipe_packets;
    counters.pong_response_packets = world_counters.pong_response_packets;
}

fn sync_waypoint_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.waypoint_packets = world_counters.waypoint_packets;
    counters.waypoints_tracked = world_counters.waypoints_tracked;
    counters.waypoint_updates_applied = world_counters.waypoint_updates_applied;
    counters.waypoint_updates_ignored = world_counters.waypoint_updates_ignored;
    counters.waypoint_untracks_ignored = world_counters.waypoint_untracks_ignored;
}

fn sync_map_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.map_item_data_packets = world_counters.map_item_data_packets;
    counters.maps_tracked = world_counters.maps_tracked;
    counters.map_decorations_tracked = world_counters.map_decorations_tracked;
    counters.map_color_patches_applied = world_counters.map_color_patches_applied;
    counters.map_color_patches_ignored = world_counters.map_color_patches_ignored;
}

fn sync_client_combat_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.player_combat_end_packets = world_counters.player_combat_end_packets;
    counters.player_combat_enter_packets = world_counters.player_combat_enter_packets;
    counters.player_combat_kill_packets = world_counters.player_combat_kill_packets;
}

fn sync_transfer_counters(counters: &mut NetCounters, world: &WorldStore) {
    counters.transfer_packets = world.counters().transfer_packets;
}

fn sync_server_link_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.server_link_packets = world_counters.server_link_packets;
    counters.server_link_invalid_entries = world_counters.server_link_invalid_entries;
    counters.server_links_tracked = world_counters.server_links_tracked;
    counters.server_links = world
        .server_links()
        .iter()
        .map(|link| bbb_control::ServerLinkState {
            label: link.label.clone(),
            url: link.url.clone(),
            known_type: link.known_type.clone(),
        })
        .collect();
}

fn sync_client_stats_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.award_stats_packets = world_counters.award_stats_packets;
    counters.award_stats_entries_received = world_counters.award_stats_entries_received;
    counters.last_award_stats_entry_count = world_counters.last_award_stats_entry_count;
    counters.stats_tracked = world_counters.stats_tracked;
}
