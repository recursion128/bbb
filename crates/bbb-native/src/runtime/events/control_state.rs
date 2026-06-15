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
        }
        NetEvent::TagQuery(update) => {
            world.apply_tag_query(update);
        }
        other => return Some(other),
    }

    None
}

pub(super) fn sync_entity_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.minecart_moves_received = world_counters.minecart_moves_received;
    counters.minecart_moves_applied = world_counters.minecart_moves_applied;
    counters.minecart_moves_ignored = world_counters.minecart_moves_ignored;
    counters.minecart_lerp_steps_received = world_counters.minecart_lerp_steps_received;
    counters.minecart_lerp_steps_tracked = world_counters.minecart_lerp_steps_tracked;
}
