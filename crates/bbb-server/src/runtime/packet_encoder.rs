use std::sync::Arc;

use bbb_protocol::{
    codec::ProtocolError,
    content_registry::ContentRegistry,
    packets::client_common::encode_play_clientbound_custom_payload,
    packets::{
        encode_play_clientbound_add_entity, encode_play_clientbound_animate,
        encode_play_clientbound_award_stats, encode_play_clientbound_block_changed_ack,
        encode_play_clientbound_block_destruction, encode_play_clientbound_block_entity_data,
        encode_play_clientbound_block_event_with_content_registry,
        encode_play_clientbound_block_update_with_content_registry,
        encode_play_clientbound_boss_event, encode_play_clientbound_bundle_delimiter,
        encode_play_clientbound_change_difficulty, encode_play_clientbound_chunk_batch_finished,
        encode_play_clientbound_chunk_batch_start, encode_play_clientbound_chunks_biomes,
        encode_play_clientbound_clear_dialog, encode_play_clientbound_clear_titles,
        encode_play_clientbound_command_suggestions, encode_play_clientbound_commands,
        encode_play_clientbound_container_close,
        encode_play_clientbound_container_set_content_with_content_registry,
        encode_play_clientbound_container_set_data,
        encode_play_clientbound_container_set_slot_with_content_registry,
        encode_play_clientbound_cooldown, encode_play_clientbound_custom_chat_completions,
        encode_play_clientbound_damage_event, encode_play_clientbound_debug_block_value,
        encode_play_clientbound_debug_chunk_value, encode_play_clientbound_debug_entity_value,
        encode_play_clientbound_debug_event, encode_play_clientbound_debug_sample,
        encode_play_clientbound_delete_chat, encode_play_clientbound_disguised_chat,
        encode_play_clientbound_entity_event, encode_play_clientbound_entity_position_sync,
        encode_play_clientbound_explode, encode_play_clientbound_forget_level_chunk,
        encode_play_clientbound_game_event, encode_play_clientbound_game_rule_values,
        encode_play_clientbound_hurt_animation, encode_play_clientbound_initialize_border,
        encode_play_clientbound_keep_alive, encode_play_clientbound_level_chunk_with_light,
        encode_play_clientbound_level_event, encode_play_clientbound_level_particles,
        encode_play_clientbound_light_update, encode_play_clientbound_login,
        encode_play_clientbound_map_item_data,
        encode_play_clientbound_merchant_offers_with_content_registry,
        encode_play_clientbound_mount_screen_open, encode_play_clientbound_move_entity,
        encode_play_clientbound_move_minecart_along_track, encode_play_clientbound_move_vehicle,
        encode_play_clientbound_open_book, encode_play_clientbound_open_screen,
        encode_play_clientbound_open_sign_editor,
        encode_play_clientbound_place_ghost_recipe_with_content_registry,
        encode_play_clientbound_player_abilities, encode_play_clientbound_player_chat,
        encode_play_clientbound_player_combat_end, encode_play_clientbound_player_combat_enter,
        encode_play_clientbound_player_combat_kill, encode_play_clientbound_player_info_remove,
        encode_play_clientbound_player_info_update, encode_play_clientbound_player_look_at,
        encode_play_clientbound_player_position, encode_play_clientbound_player_rotation,
        encode_play_clientbound_pong_response, encode_play_clientbound_projectile_power,
        encode_play_clientbound_recipe_book_add_with_content_registry,
        encode_play_clientbound_recipe_book_remove, encode_play_clientbound_recipe_book_settings,
        encode_play_clientbound_remove_entities, encode_play_clientbound_remove_mob_effect,
        encode_play_clientbound_reset_score, encode_play_clientbound_respawn,
        encode_play_clientbound_rotate_head,
        encode_play_clientbound_section_blocks_update_with_content_registry,
        encode_play_clientbound_select_advancements_tab, encode_play_clientbound_server_data,
        encode_play_clientbound_set_action_bar_text, encode_play_clientbound_set_border_center,
        encode_play_clientbound_set_border_lerp_size, encode_play_clientbound_set_border_size,
        encode_play_clientbound_set_border_warning_delay,
        encode_play_clientbound_set_border_warning_distance, encode_play_clientbound_set_camera,
        encode_play_clientbound_set_chunk_cache_center,
        encode_play_clientbound_set_chunk_cache_radius,
        encode_play_clientbound_set_cursor_item_with_content_registry,
        encode_play_clientbound_set_default_spawn_position,
        encode_play_clientbound_set_display_objective,
        encode_play_clientbound_set_entity_data_with_content_registry,
        encode_play_clientbound_set_entity_link, encode_play_clientbound_set_entity_motion,
        encode_play_clientbound_set_equipment_with_content_registry,
        encode_play_clientbound_set_experience, encode_play_clientbound_set_health,
        encode_play_clientbound_set_held_slot, encode_play_clientbound_set_objective,
        encode_play_clientbound_set_passengers,
        encode_play_clientbound_set_player_inventory_with_content_registry,
        encode_play_clientbound_set_player_team, encode_play_clientbound_set_score,
        encode_play_clientbound_set_simulation_distance, encode_play_clientbound_set_subtitle_text,
        encode_play_clientbound_set_time, encode_play_clientbound_set_title_text,
        encode_play_clientbound_set_titles_animation, encode_play_clientbound_show_dialog,
        encode_play_clientbound_sound, encode_play_clientbound_sound_entity,
        encode_play_clientbound_stop_sound, encode_play_clientbound_system_chat,
        encode_play_clientbound_tag_query, encode_play_clientbound_take_item_entity,
        encode_play_clientbound_teleport_entity, encode_play_clientbound_ticking_state,
        encode_play_clientbound_ticking_step, encode_play_clientbound_tracked_waypoint,
        encode_play_clientbound_transfer,
        encode_play_clientbound_update_advancements_with_content_registry,
        encode_play_clientbound_update_attributes, encode_play_clientbound_update_mob_effect,
        encode_play_clientbound_update_recipes_with_content_registry,
        encode_play_clientbound_update_tags,
    },
};
use bbb_server_core::connection::ClientboundPlayPacket;
use bbb_server_net::ConnectionCommand;

pub(super) fn encode_world_packet(
    packet: ClientboundPlayPacket,
    content_registry: &ContentRegistry,
) -> Result<ConnectionCommand, ProtocolError> {
    if matches!(packet, ClientboundPlayPacket::StartConfiguration) {
        return Ok(ConnectionCommand::StartConfiguration { config: None });
    }
    let (packet_id, payload) = match packet {
        ClientboundPlayPacket::BossEvent(update) => encode_play_clientbound_boss_event(&update)?,
        ClientboundPlayPacket::BlockChangedAck(update) => {
            encode_play_clientbound_block_changed_ack(update)
        }
        ClientboundPlayPacket::BlockDestruction(update) => {
            encode_play_clientbound_block_destruction(update)
        }
        ClientboundPlayPacket::BlockEntityData(update) => {
            encode_play_clientbound_block_entity_data(&update)?
        }
        ClientboundPlayPacket::BlockEvent(update) => {
            encode_play_clientbound_block_event_with_content_registry(update, content_registry)?
        }
        ClientboundPlayPacket::BlockUpdate(update) => {
            encode_play_clientbound_block_update_with_content_registry(update, content_registry)?
        }
        ClientboundPlayPacket::OpenSignEditor(update) => {
            encode_play_clientbound_open_sign_editor(update)
        }
        ClientboundPlayPacket::OpenBook(update) => encode_play_clientbound_open_book(update),
        ClientboundPlayPacket::MountScreenOpen(update) => {
            encode_play_clientbound_mount_screen_open(update)
        }
        ClientboundPlayPacket::ChangeDifficulty(update) => {
            encode_play_clientbound_change_difficulty(update)
        }
        ClientboundPlayPacket::ChunkBatchStart => encode_play_clientbound_chunk_batch_start(),
        ClientboundPlayPacket::ChunkBatchFinished { batch_size } => {
            encode_play_clientbound_chunk_batch_finished(batch_size)
        }
        ClientboundPlayPacket::Login(login) => encode_play_clientbound_login(&login)?,
        ClientboundPlayPacket::KeepAlive { id } => encode_play_clientbound_keep_alive(id),
        ClientboundPlayPacket::PlayerAbilities(update) => {
            encode_play_clientbound_player_abilities(update)
        }
        ClientboundPlayPacket::SetHeldSlot(update) => encode_play_clientbound_set_held_slot(update),
        ClientboundPlayPacket::Cooldown(update) => encode_play_clientbound_cooldown(&update),
        ClientboundPlayPacket::EntityEvent(update) => encode_play_clientbound_entity_event(update),
        ClientboundPlayPacket::UpdateMobEffect(update) => {
            encode_play_clientbound_update_mob_effect(&update)?
        }
        ClientboundPlayPacket::RemoveMobEffect(update) => {
            encode_play_clientbound_remove_mob_effect(&update)?
        }
        ClientboundPlayPacket::AwardStats(update) => encode_play_clientbound_award_stats(&update)?,
        ClientboundPlayPacket::SelectAdvancementsTab(update) => {
            encode_play_clientbound_select_advancements_tab(&update)?
        }
        ClientboundPlayPacket::CommandSuggestions(update) => {
            encode_play_clientbound_command_suggestions(&update)?
        }
        ClientboundPlayPacket::Commands(update) => encode_play_clientbound_commands(&update)?,
        ClientboundPlayPacket::ContainerSetContent(update) => {
            encode_play_clientbound_container_set_content_with_content_registry(
                &update,
                content_registry,
            )?
        }
        ClientboundPlayPacket::ContainerSetSlot(update) => {
            encode_play_clientbound_container_set_slot_with_content_registry(
                &update,
                content_registry,
            )?
        }
        ClientboundPlayPacket::ContainerSetData(update) => {
            encode_play_clientbound_container_set_data(update)
        }
        ClientboundPlayPacket::SetCursorItem(update) => {
            encode_play_clientbound_set_cursor_item_with_content_registry(
                &update,
                content_registry,
            )?
        }
        ClientboundPlayPacket::OpenScreen(update) => encode_play_clientbound_open_screen(&update)?,
        ClientboundPlayPacket::ContainerClose(update) => {
            encode_play_clientbound_container_close(update)
        }
        ClientboundPlayPacket::MerchantOffers(update) => {
            encode_play_clientbound_merchant_offers_with_content_registry(
                &update,
                content_registry,
            )?
        }
        ClientboundPlayPacket::SetPlayerInventory(update) => {
            encode_play_clientbound_set_player_inventory_with_content_registry(
                &update,
                content_registry,
            )?
        }
        ClientboundPlayPacket::SystemChat(update) => encode_play_clientbound_system_chat(&update)?,
        ClientboundPlayPacket::PlayerChat(chat) => encode_play_clientbound_player_chat(&chat)?,
        ClientboundPlayPacket::DisguisedChat(chat) => {
            encode_play_clientbound_disguised_chat(&chat)?
        }
        ClientboundPlayPacket::DeleteChat(update) => encode_play_clientbound_delete_chat(&update)?,
        ClientboundPlayPacket::CustomChatCompletions(update) => {
            encode_play_clientbound_custom_chat_completions(&update)?
        }
        ClientboundPlayPacket::PlayerPosition(update) => {
            encode_play_clientbound_player_position(update)
        }
        ClientboundPlayPacket::PlayerLookAt(update) => {
            encode_play_clientbound_player_look_at(update)
        }
        ClientboundPlayPacket::SetChunkCacheCenter(update) => {
            encode_play_clientbound_set_chunk_cache_center(update)
        }
        ClientboundPlayPacket::LevelChunkWithLight(chunk) => {
            encode_play_clientbound_level_chunk_with_light(&chunk)?
        }
        ClientboundPlayPacket::LightUpdate(update) => {
            encode_play_clientbound_light_update(&update)?
        }
        ClientboundPlayPacket::ForgetLevelChunk(update) => {
            encode_play_clientbound_forget_level_chunk(update)
        }
        ClientboundPlayPacket::SetTime(update) => encode_play_clientbound_set_time(&update)?,
        ClientboundPlayPacket::MapItemData(update) => {
            encode_play_clientbound_map_item_data(&update)?
        }
        ClientboundPlayPacket::TrackedWaypoint(update) => {
            encode_play_clientbound_tracked_waypoint(&update)?
        }
        ClientboundPlayPacket::SetChunkCacheRadius(update) => {
            encode_play_clientbound_set_chunk_cache_radius(update)
        }
        ClientboundPlayPacket::SetSimulationDistance(update) => {
            encode_play_clientbound_set_simulation_distance(update)
        }
        ClientboundPlayPacket::PongResponse(response) => {
            encode_play_clientbound_pong_response(response)
        }
        ClientboundPlayPacket::PlayerInfoUpdate(update) => {
            encode_play_clientbound_player_info_update(&update)?
        }
        ClientboundPlayPacket::PlayerInfoRemove(update) => {
            encode_play_clientbound_player_info_remove(&update)?
        }
        ClientboundPlayPacket::BundleDelimiter => encode_play_clientbound_bundle_delimiter(),
        ClientboundPlayPacket::AddEntity(update) => encode_play_clientbound_add_entity(&update),
        ClientboundPlayPacket::MoveEntity(update) => encode_play_clientbound_move_entity(&update)?,
        ClientboundPlayPacket::MoveMinecartAlongTrack(update) => {
            encode_play_clientbound_move_minecart_along_track(&update)?
        }
        ClientboundPlayPacket::MoveVehicle(update) => encode_play_clientbound_move_vehicle(update),
        ClientboundPlayPacket::EntityPositionSync(update) => {
            encode_play_clientbound_entity_position_sync(&update)
        }
        ClientboundPlayPacket::TeleportEntity(update) => {
            encode_play_clientbound_teleport_entity(&update)
        }
        ClientboundPlayPacket::RemoveEntities(update) => {
            encode_play_clientbound_remove_entities(&update)?
        }
        ClientboundPlayPacket::SetEntityData(update) => {
            encode_play_clientbound_set_entity_data_with_content_registry(
                &update,
                content_registry,
            )?
        }
        ClientboundPlayPacket::DebugEntityValue(update) => {
            encode_play_clientbound_debug_entity_value(&update)?
        }
        ClientboundPlayPacket::DebugBlockValue(update) => {
            encode_play_clientbound_debug_block_value(&update)?
        }
        ClientboundPlayPacket::DebugChunkValue(update) => {
            encode_play_clientbound_debug_chunk_value(&update)?
        }
        ClientboundPlayPacket::DebugEvent(update) => encode_play_clientbound_debug_event(&update)?,
        ClientboundPlayPacket::SetEquipment(update) => {
            encode_play_clientbound_set_equipment_with_content_registry(&update, content_registry)?
        }
        ClientboundPlayPacket::SetPassengers(update) => {
            encode_play_clientbound_set_passengers(&update)?
        }
        ClientboundPlayPacket::SetEntityLink(update) => {
            encode_play_clientbound_set_entity_link(&update)
        }
        ClientboundPlayPacket::UpdateAttributes(update) => {
            encode_play_clientbound_update_attributes(&update)?
        }
        ClientboundPlayPacket::RotateHead(update) => encode_play_clientbound_rotate_head(&update),
        ClientboundPlayPacket::SetEntityMotion(update) => {
            encode_play_clientbound_set_entity_motion(&update)
        }
        ClientboundPlayPacket::ProjectilePower(update) => {
            encode_play_clientbound_projectile_power(update)
        }
        ClientboundPlayPacket::TakeItemEntity(update) => {
            encode_play_clientbound_take_item_entity(&update)
        }
        ClientboundPlayPacket::Sound(update) => encode_play_clientbound_sound(&update)?,
        ClientboundPlayPacket::SoundEntity(update) => {
            encode_play_clientbound_sound_entity(&update)?
        }
        ClientboundPlayPacket::SectionBlocksUpdate(update) => {
            encode_play_clientbound_section_blocks_update_with_content_registry(
                &update,
                content_registry,
            )?
        }
        ClientboundPlayPacket::ServerData(update) => encode_play_clientbound_server_data(&update)?,
        ClientboundPlayPacket::DebugSample(sample) => encode_play_clientbound_debug_sample(&sample),
        ClientboundPlayPacket::StopSound(update) => encode_play_clientbound_stop_sound(&update)?,
        ClientboundPlayPacket::UpdateAdvancements(update) => {
            encode_play_clientbound_update_advancements_with_content_registry(
                &update,
                content_registry,
            )?
        }
        ClientboundPlayPacket::RecipeBookAdd(update) => {
            encode_play_clientbound_recipe_book_add_with_content_registry(
                &update,
                content_registry,
            )?
        }
        ClientboundPlayPacket::RecipeBookRemove(update) => {
            encode_play_clientbound_recipe_book_remove(&update)?
        }
        ClientboundPlayPacket::PlaceGhostRecipe(update) => {
            encode_play_clientbound_place_ghost_recipe_with_content_registry(
                &update,
                content_registry,
            )?
        }
        ClientboundPlayPacket::RecipeBookSettings(update) => {
            encode_play_clientbound_recipe_book_settings(&update)?
        }
        ClientboundPlayPacket::UpdateRecipes(update) => {
            encode_play_clientbound_update_recipes_with_content_registry(&update, content_registry)?
        }
        ClientboundPlayPacket::ClearTitles(update) => {
            encode_play_clientbound_clear_titles(&update)?
        }
        ClientboundPlayPacket::SetTitleText(update) => {
            encode_play_clientbound_set_title_text(&update)?
        }
        ClientboundPlayPacket::SetSubtitleText(update) => {
            encode_play_clientbound_set_subtitle_text(&update)?
        }
        ClientboundPlayPacket::SetActionBarText(update) => {
            encode_play_clientbound_set_action_bar_text(&update)?
        }
        ClientboundPlayPacket::SetTitlesAnimation(update) => {
            encode_play_clientbound_set_titles_animation(&update)?
        }
        ClientboundPlayPacket::InitializeBorder(update) => {
            encode_play_clientbound_initialize_border(&update)
        }
        ClientboundPlayPacket::SetBorderCenter(update) => {
            encode_play_clientbound_set_border_center(&update)
        }
        ClientboundPlayPacket::SetBorderLerpSize(update) => {
            encode_play_clientbound_set_border_lerp_size(&update)
        }
        ClientboundPlayPacket::SetBorderSize(update) => {
            encode_play_clientbound_set_border_size(&update)
        }
        ClientboundPlayPacket::SetBorderWarningDelay(update) => {
            encode_play_clientbound_set_border_warning_delay(&update)
        }
        ClientboundPlayPacket::SetBorderWarningDistance(update) => {
            encode_play_clientbound_set_border_warning_distance(&update)
        }
        ClientboundPlayPacket::GameEvent(update) => encode_play_clientbound_game_event(update)?,
        ClientboundPlayPacket::LevelEvent(update) => encode_play_clientbound_level_event(update),
        ClientboundPlayPacket::LevelParticles(update) => {
            encode_play_clientbound_level_particles(update)
        }
        ClientboundPlayPacket::Explode(update) => encode_play_clientbound_explode(update)?,
        ClientboundPlayPacket::TickingState(update) => {
            encode_play_clientbound_ticking_state(update)
        }
        ClientboundPlayPacket::TickingStep(update) => encode_play_clientbound_ticking_step(update),
        ClientboundPlayPacket::SetHealth(update) => encode_play_clientbound_set_health(update),
        ClientboundPlayPacket::SetExperience(update) => {
            encode_play_clientbound_set_experience(update)
        }
        ClientboundPlayPacket::DamageEvent(update) => encode_play_clientbound_damage_event(update),
        ClientboundPlayPacket::HurtAnimation(update) => {
            encode_play_clientbound_hurt_animation(update)
        }
        ClientboundPlayPacket::TagQuery(update) => encode_play_clientbound_tag_query(&update)?,
        ClientboundPlayPacket::GameRuleValues(update) => {
            encode_play_clientbound_game_rule_values(&update)?
        }
        ClientboundPlayPacket::PlayerCombatEnter => encode_play_clientbound_player_combat_enter(),
        ClientboundPlayPacket::PlayerCombatEnd(update) => {
            encode_play_clientbound_player_combat_end(update)
        }
        ClientboundPlayPacket::PlayerCombatKill(update) => {
            encode_play_clientbound_player_combat_kill(&update)?
        }
        ClientboundPlayPacket::StartConfiguration => unreachable!("handled before payload match"),
        ClientboundPlayPacket::Respawn(update) => encode_play_clientbound_respawn(&update)?,
        ClientboundPlayPacket::SetDefaultSpawnPosition(update) => {
            encode_play_clientbound_set_default_spawn_position(&update)?
        }
        ClientboundPlayPacket::EntityAnimation(update) => encode_play_clientbound_animate(update),
        ClientboundPlayPacket::PlayerRotation(update) => {
            encode_play_clientbound_player_rotation(update)
        }
        ClientboundPlayPacket::SetCamera(update) => encode_play_clientbound_set_camera(update),
        ClientboundPlayPacket::Transfer(update) => encode_play_clientbound_transfer(&update)?,
        ClientboundPlayPacket::UpdateTags(update) => encode_play_clientbound_update_tags(&update)?,
        ClientboundPlayPacket::SetObjective(update) => {
            encode_play_clientbound_set_objective(&update)?
        }
        ClientboundPlayPacket::SetScore(update) => encode_play_clientbound_set_score(&update)?,
        ClientboundPlayPacket::ResetScore(update) => encode_play_clientbound_reset_score(&update)?,
        ClientboundPlayPacket::SetDisplayObjective(update) => {
            encode_play_clientbound_set_display_objective(&update)?
        }
        ClientboundPlayPacket::SetPlayerTeam(update) => {
            encode_play_clientbound_set_player_team(&update)?
        }
        ClientboundPlayPacket::ClearDialog => encode_play_clientbound_clear_dialog(),
        ClientboundPlayPacket::ShowDialog(update) => encode_play_clientbound_show_dialog(&update)?,
        ClientboundPlayPacket::ChunksBiomes(update) => {
            encode_play_clientbound_chunks_biomes(&update)?
        }
        ClientboundPlayPacket::CustomPayload(payload) => {
            encode_play_clientbound_custom_payload(&payload)?
        }
    };
    Ok(ConnectionCommand::SendPlayPacket {
        packet_id,
        payload: Arc::new(payload),
    })
}

pub(super) fn world_packet_kind(packet: &ClientboundPlayPacket) -> &'static str {
    match packet {
        ClientboundPlayPacket::BossEvent(_) => "boss_event",
        ClientboundPlayPacket::BlockChangedAck(_) => "block_changed_ack",
        ClientboundPlayPacket::BlockDestruction(_) => "block_destruction",
        ClientboundPlayPacket::BlockEntityData(_) => "block_entity_data",
        ClientboundPlayPacket::BlockEvent(_) => "block_event",
        ClientboundPlayPacket::BlockUpdate(_) => "block_update",
        ClientboundPlayPacket::OpenSignEditor(_) => "open_sign_editor",
        ClientboundPlayPacket::OpenBook(_) => "open_book",
        ClientboundPlayPacket::MountScreenOpen(_) => "mount_screen_open",
        ClientboundPlayPacket::ChangeDifficulty(_) => "change_difficulty",
        ClientboundPlayPacket::ChunkBatchStart => "chunk_batch_start",
        ClientboundPlayPacket::ChunkBatchFinished { .. } => "chunk_batch_finished",
        ClientboundPlayPacket::Login(_) => "login",
        ClientboundPlayPacket::KeepAlive { .. } => "keep_alive",
        ClientboundPlayPacket::PlayerAbilities(_) => "player_abilities",
        ClientboundPlayPacket::SetHeldSlot(_) => "set_held_slot",
        ClientboundPlayPacket::Cooldown(_) => "cooldown",
        ClientboundPlayPacket::EntityEvent(_) => "entity_event",
        ClientboundPlayPacket::UpdateMobEffect(_) => "update_mob_effect",
        ClientboundPlayPacket::RemoveMobEffect(_) => "remove_mob_effect",
        ClientboundPlayPacket::AwardStats(_) => "award_stats",
        ClientboundPlayPacket::SetExperience(_) => "set_experience",
        ClientboundPlayPacket::SelectAdvancementsTab(_) => "select_advancements_tab",
        ClientboundPlayPacket::CommandSuggestions(_) => "command_suggestions",
        ClientboundPlayPacket::Commands(_) => "commands",
        ClientboundPlayPacket::ContainerSetContent(_) => "container_set_content",
        ClientboundPlayPacket::ContainerSetSlot(_) => "container_set_slot",
        ClientboundPlayPacket::ContainerSetData(_) => "container_set_data",
        ClientboundPlayPacket::SetCursorItem(_) => "set_cursor_item",
        ClientboundPlayPacket::OpenScreen(_) => "open_screen",
        ClientboundPlayPacket::ContainerClose(_) => "container_close",
        ClientboundPlayPacket::MerchantOffers(_) => "merchant_offers",
        ClientboundPlayPacket::SetPlayerInventory(_) => "set_player_inventory",
        ClientboundPlayPacket::SystemChat(_) => "system_chat",
        ClientboundPlayPacket::PlayerChat(_) => "player_chat",
        ClientboundPlayPacket::DisguisedChat(_) => "disguised_chat",
        ClientboundPlayPacket::DeleteChat(_) => "delete_chat",
        ClientboundPlayPacket::CustomChatCompletions(_) => "custom_chat_completions",
        ClientboundPlayPacket::PlayerPosition(_) => "player_position",
        ClientboundPlayPacket::PlayerLookAt(_) => "player_look_at",
        ClientboundPlayPacket::SetChunkCacheCenter(_) => "set_chunk_cache_center",
        ClientboundPlayPacket::LevelChunkWithLight(_) => "level_chunk_with_light",
        ClientboundPlayPacket::LightUpdate(_) => "light_update",
        ClientboundPlayPacket::LevelEvent(_) => "level_event",
        ClientboundPlayPacket::LevelParticles(_) => "level_particles",
        ClientboundPlayPacket::Explode(_) => "explode",
        ClientboundPlayPacket::ForgetLevelChunk(_) => "forget_level_chunk",
        ClientboundPlayPacket::SetTime(_) => "set_time",
        ClientboundPlayPacket::MapItemData(_) => "map_item_data",
        ClientboundPlayPacket::TrackedWaypoint(_) => "tracked_waypoint",
        ClientboundPlayPacket::SetChunkCacheRadius(_) => "set_chunk_cache_radius",
        ClientboundPlayPacket::SetSimulationDistance(_) => "set_simulation_distance",
        ClientboundPlayPacket::PongResponse(_) => "pong_response",
        ClientboundPlayPacket::PlayerInfoUpdate(_) => "player_info_update",
        ClientboundPlayPacket::PlayerInfoRemove(_) => "player_info_remove",
        ClientboundPlayPacket::BundleDelimiter => "bundle_delimiter",
        ClientboundPlayPacket::AddEntity(_) => "add_entity",
        ClientboundPlayPacket::MoveEntity(_) => "move_entity",
        ClientboundPlayPacket::MoveMinecartAlongTrack(_) => "move_minecart_along_track",
        ClientboundPlayPacket::MoveVehicle(_) => "move_vehicle",
        ClientboundPlayPacket::EntityPositionSync(_) => "entity_position_sync",
        ClientboundPlayPacket::TeleportEntity(_) => "teleport_entity",
        ClientboundPlayPacket::RemoveEntities(_) => "remove_entities",
        ClientboundPlayPacket::SetEntityData(_) => "set_entity_data",
        ClientboundPlayPacket::DebugEntityValue(_) => "debug_entity_value",
        ClientboundPlayPacket::DebugBlockValue(_) => "debug_block_value",
        ClientboundPlayPacket::DebugChunkValue(_) => "debug_chunk_value",
        ClientboundPlayPacket::DebugEvent(_) => "debug_event",
        ClientboundPlayPacket::SetEquipment(_) => "set_equipment",
        ClientboundPlayPacket::SetPassengers(_) => "set_passengers",
        ClientboundPlayPacket::SetEntityLink(_) => "set_entity_link",
        ClientboundPlayPacket::UpdateAttributes(_) => "update_attributes",
        ClientboundPlayPacket::RotateHead(_) => "rotate_head",
        ClientboundPlayPacket::SetEntityMotion(_) => "set_entity_motion",
        ClientboundPlayPacket::ProjectilePower(_) => "projectile_power",
        ClientboundPlayPacket::TakeItemEntity(_) => "take_item_entity",
        ClientboundPlayPacket::Sound(_) => "sound",
        ClientboundPlayPacket::SoundEntity(_) => "sound_entity",
        ClientboundPlayPacket::SectionBlocksUpdate(_) => "section_blocks_update",
        ClientboundPlayPacket::ServerData(_) => "server_data",
        ClientboundPlayPacket::DebugSample(_) => "debug_sample",
        ClientboundPlayPacket::StopSound(_) => "stop_sound",
        ClientboundPlayPacket::UpdateAdvancements(_) => "update_advancements",
        ClientboundPlayPacket::RecipeBookAdd(_) => "recipe_book_add",
        ClientboundPlayPacket::RecipeBookRemove(_) => "recipe_book_remove",
        ClientboundPlayPacket::PlaceGhostRecipe(_) => "place_ghost_recipe",
        ClientboundPlayPacket::RecipeBookSettings(_) => "recipe_book_settings",
        ClientboundPlayPacket::UpdateRecipes(_) => "update_recipes",
        ClientboundPlayPacket::ClearTitles(_) => "clear_titles",
        ClientboundPlayPacket::SetTitleText(_) => "set_title_text",
        ClientboundPlayPacket::SetSubtitleText(_) => "set_subtitle_text",
        ClientboundPlayPacket::SetActionBarText(_) => "set_action_bar_text",
        ClientboundPlayPacket::SetTitlesAnimation(_) => "set_titles_animation",
        ClientboundPlayPacket::InitializeBorder(_) => "initialize_border",
        ClientboundPlayPacket::SetBorderCenter(_) => "set_border_center",
        ClientboundPlayPacket::SetBorderLerpSize(_) => "set_border_lerp_size",
        ClientboundPlayPacket::SetBorderSize(_) => "set_border_size",
        ClientboundPlayPacket::SetBorderWarningDelay(_) => "set_border_warning_delay",
        ClientboundPlayPacket::SetBorderWarningDistance(_) => "set_border_warning_distance",
        ClientboundPlayPacket::GameEvent(_) => "game_event",
        ClientboundPlayPacket::TickingState(_) => "ticking_state",
        ClientboundPlayPacket::TickingStep(_) => "ticking_step",
        ClientboundPlayPacket::SetHealth(_) => "set_health",
        ClientboundPlayPacket::DamageEvent(_) => "damage_event",
        ClientboundPlayPacket::HurtAnimation(_) => "hurt_animation",
        ClientboundPlayPacket::TagQuery(_) => "tag_query",
        ClientboundPlayPacket::GameRuleValues(_) => "game_rule_values",
        ClientboundPlayPacket::PlayerCombatEnter => "player_combat_enter",
        ClientboundPlayPacket::PlayerCombatEnd(_) => "player_combat_end",
        ClientboundPlayPacket::PlayerCombatKill(_) => "player_combat_kill",
        ClientboundPlayPacket::StartConfiguration => "start_configuration",
        ClientboundPlayPacket::Respawn(_) => "respawn",
        ClientboundPlayPacket::SetDefaultSpawnPosition(_) => "set_default_spawn_position",
        ClientboundPlayPacket::EntityAnimation(_) => "entity_animation",
        ClientboundPlayPacket::PlayerRotation(_) => "player_rotation",
        ClientboundPlayPacket::SetCamera(_) => "set_camera",
        ClientboundPlayPacket::Transfer(_) => "transfer",
        ClientboundPlayPacket::UpdateTags(_) => "update_tags",
        ClientboundPlayPacket::SetObjective(_) => "set_objective",
        ClientboundPlayPacket::SetScore(_) => "set_score",
        ClientboundPlayPacket::ResetScore(_) => "reset_score",
        ClientboundPlayPacket::SetDisplayObjective(_) => "set_display_objective",
        ClientboundPlayPacket::SetPlayerTeam(_) => "set_player_team",
        ClientboundPlayPacket::ClearDialog => "clear_dialog",
        ClientboundPlayPacket::ShowDialog(_) => "show_dialog",
        ClientboundPlayPacket::ChunksBiomes(_) => "chunks_biomes",
        ClientboundPlayPacket::CustomPayload(_) => "custom_payload",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        content_registry::{ContentRegistryBuilder, ModBlockRegistration, ModItemRegistration},
        equipment_slots::EquipmentSlot,
        packets::{
            decode_play_clientbound_with_content_registry, BlockEvent, BlockPos, BlockUpdate,
            ContainerSetSlot, EquipmentSlotUpdate, ItemStackSummary, PlayClientbound,
            SectionBlocksUpdate, SetCursorItem, SetEquipment, SetPlayerInventory,
            SparseDataComponentPatchSummary,
        },
    };

    #[test]
    fn encode_world_packet_encodes_custom_mod_blocks_and_items_with_registry() {
        let mut builder = ContentRegistryBuilder::new();
        builder.register_block(ModBlockRegistration {
            resource_id: "modded:crystal_block".to_string(),
            state_count: 2,
            light_emission: 0,
            destroy_speed: 1.5,
            full_cube_collision: true,
            dynamic_light_rgb: None,
        });
        builder.register_item(ModItemRegistration {
            resource_id: "modded:crystal_shard".to_string(),
            max_stack_size: 16,
            is_block_item: false,
        });
        let registry = builder.freeze().unwrap();
        let block = registry
            .block_by_resource_id("modded:crystal_block")
            .unwrap();
        let item = registry
            .item_by_resource_id("modded:crystal_shard")
            .unwrap();
        let custom_state = block.first_state_id + 1;
        let custom_stack = ItemStackSummary {
            item_id: Some(item.id),
            count: 3,
            component_patch: SparseDataComponentPatchSummary::default(),
        };

        let cases = [
            (
                ClientboundPlayPacket::BlockUpdate(BlockUpdate {
                    pos: BlockPos { x: 1, y: 64, z: -2 },
                    block_state_id: custom_state,
                }),
                PlayClientbound::BlockUpdate(BlockUpdate {
                    pos: BlockPos { x: 1, y: 64, z: -2 },
                    block_state_id: custom_state,
                }),
            ),
            (
                ClientboundPlayPacket::SectionBlocksUpdate(SectionBlocksUpdate {
                    section_x: 0,
                    section_y: 4,
                    section_z: 0,
                    updates: vec![BlockUpdate {
                        pos: BlockPos { x: 1, y: 66, z: 3 },
                        block_state_id: custom_state,
                    }],
                }),
                PlayClientbound::SectionBlocksUpdate(SectionBlocksUpdate {
                    section_x: 0,
                    section_y: 4,
                    section_z: 0,
                    updates: vec![BlockUpdate {
                        pos: BlockPos { x: 1, y: 66, z: 3 },
                        block_state_id: custom_state,
                    }],
                }),
            ),
            (
                ClientboundPlayPacket::BlockEvent(BlockEvent {
                    pos: BlockPos { x: 2, y: 65, z: 3 },
                    b0: 1,
                    b1: 0,
                    block_id: block.block_id,
                }),
                PlayClientbound::BlockEvent(BlockEvent {
                    pos: BlockPos { x: 2, y: 65, z: 3 },
                    b0: 1,
                    b1: 0,
                    block_id: block.block_id,
                }),
            ),
            (
                ClientboundPlayPacket::ContainerSetSlot(ContainerSetSlot {
                    container_id: 0,
                    state_id: 1,
                    slot: 36,
                    item: custom_stack.clone(),
                }),
                PlayClientbound::ContainerSetSlot(ContainerSetSlot {
                    container_id: 0,
                    state_id: 1,
                    slot: 36,
                    item: custom_stack.clone(),
                }),
            ),
            (
                ClientboundPlayPacket::SetCursorItem(SetCursorItem {
                    item: custom_stack.clone(),
                }),
                PlayClientbound::SetCursorItem(SetCursorItem {
                    item: custom_stack.clone(),
                }),
            ),
            (
                ClientboundPlayPacket::SetPlayerInventory(SetPlayerInventory {
                    slot: 0,
                    item: custom_stack.clone(),
                }),
                PlayClientbound::SetPlayerInventory(SetPlayerInventory {
                    slot: 0,
                    item: custom_stack.clone(),
                }),
            ),
            (
                ClientboundPlayPacket::SetEquipment(SetEquipment {
                    entity_id: 9,
                    slots: vec![EquipmentSlotUpdate {
                        slot: EquipmentSlot::MainHand,
                        item: custom_stack,
                    }],
                }),
                PlayClientbound::SetEquipment(SetEquipment {
                    entity_id: 9,
                    slots: vec![EquipmentSlotUpdate {
                        slot: EquipmentSlot::MainHand,
                        item: ItemStackSummary {
                            item_id: Some(item.id),
                            count: 3,
                            component_patch: SparseDataComponentPatchSummary::default(),
                        },
                    }],
                }),
            ),
        ];

        for (packet, expected) in cases {
            assert!(
                encode_world_packet(packet.clone(), ContentRegistry::vanilla()).is_err(),
                "vanilla registry must reject custom mod id in {}",
                world_packet_kind(&packet)
            );
            let ConnectionCommand::SendPlayPacket { packet_id, payload } =
                encode_world_packet(packet, &registry).unwrap()
            else {
                panic!("world packet encoder must produce one play packet");
            };
            assert_eq!(
                decode_play_clientbound_with_content_registry(packet_id, &payload, &registry)
                    .unwrap(),
                expected
            );
        }
    }
}
