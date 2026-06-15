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
        }
        NetEvent::StoreCookie {
            key,
            payload_len,
            stored_cookie_count,
        } => {
            world.apply_store_cookie(key, payload_len, stored_cookie_count);
        }
        NetEvent::CustomReportDetails(details) => {
            world.apply_custom_report_details(details);
        }
        NetEvent::ResetChat => {
            world.apply_reset_chat();
        }
        NetEvent::UpdateEnabledFeatures(update) => {
            world.apply_update_enabled_features(update);
        }
        NetEvent::CodeOfConduct { text } => {
            world.apply_code_of_conduct(text);
        }
        NetEvent::CustomChatCompletions(update) => {
            world.apply_custom_chat_completions(update);
        }
        NetEvent::CustomPayload(update) => {
            world.apply_custom_payload(update);
        }
        NetEvent::ServerLinks(links) => {
            world.apply_server_links(links);
        }
        NetEvent::AwardStats(update) => {
            world.apply_award_stats(update);
        }
        NetEvent::LowDiskSpaceWarning => {
            world.apply_low_disk_space_warning();
        }
        NetEvent::MapItemData(update) => {
            world.apply_map_item_data(update);
        }
        NetEvent::MountScreenOpen(update) => {
            world.apply_mount_screen_open(update);
        }
        NetEvent::OpenBook(update) => {
            world.apply_open_book(update);
        }
        NetEvent::OpenSignEditor(update) => {
            world.apply_open_sign_editor(update);
        }
        NetEvent::PlaceGhostRecipe(update) => {
            world.apply_place_ghost_recipe(update);
        }
        NetEvent::ClearDialog => {
            world.apply_clear_dialog();
        }
        NetEvent::ShowDialog(update) => {
            world.apply_show_dialog(update);
        }
        NetEvent::Waypoint(update) => {
            world.apply_waypoint(update);
        }
        NetEvent::PlayerCombatEnd(update) => {
            world.apply_player_combat_end(update);
        }
        NetEvent::PlayerCombatEnter => {
            world.apply_player_combat_enter();
        }
        NetEvent::PlayerCombatKill(update) => {
            world.apply_player_combat_kill(update);
        }
        NetEvent::PlayerLookAt(update) => {
            apply_player_look_at_update(counters, world, update);
        }
        NetEvent::PongResponse(update) => {
            world.apply_pong_response(update);
        }
        NetEvent::Explosion(update) => {
            world.apply_explosion(update);
        }
        NetEvent::LevelParticles(update) => {
            world.apply_level_particles(update);
        }
        NetEvent::ProjectilePower(update) => {
            world.apply_projectile_power(update);
        }
        NetEvent::DebugBlockValue(update) => {
            world.apply_debug_block_value(update);
        }
        NetEvent::DebugChunkValue(update) => {
            world.apply_debug_chunk_value(update);
        }
        NetEvent::DebugEntityValue(update) => {
            world.apply_debug_entity_value(update);
        }
        NetEvent::DebugEvent(update) => {
            world.apply_debug_event(update);
        }
        NetEvent::DebugSample(update) => {
            world.apply_debug_sample(update);
        }
        NetEvent::DeleteChat(update) => {
            world.apply_delete_chat(update);
        }
        NetEvent::DisguisedChat(update) => {
            world.apply_disguised_chat(update);
        }
        NetEvent::PlayerChat(update) => {
            world.apply_player_chat(update);
        }
        NetEvent::GameRuleValues(update) => {
            world.apply_game_rule_values(update);
        }
        NetEvent::GameTestHighlightPos(update) => {
            world.apply_game_test_highlight_pos(update);
        }
        NetEvent::TestInstanceBlockStatus(update) => {
            world.apply_test_instance_block_status(update);
        }
        NetEvent::Transfer(transfer) => {
            world.apply_transfer(transfer);
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
        }
        other => return Some(other),
    }

    None
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
    counters.block_destruction_packets = world_counters.block_destructions_received;
    counters.block_destructions_tracked = world_counters.block_destructions_tracked;
    counters.block_destructions_removed = world_counters.block_destructions_removed;
    counters.block_destructions_ignored = world_counters.block_destructions_ignored;
    counters.block_event_packets = world_counters.block_events_received;
    counters.block_events_tracked = world_counters.block_events_tracked;
    counters.level_event_packets = world_counters.level_events_received;
    counters.level_events_tracked = world_counters.level_events_tracked;
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
