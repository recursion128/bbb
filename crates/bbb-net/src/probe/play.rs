use anyhow::{bail, Result};
use bbb_protocol::packets::{self, PlayClientbound, ResourcePackResponseAction};
use bbb_world::ChunkPos;

use crate::{driver::maybe_send_perform_respawn, probe::ProbeContext, types::ConnectionState};

impl ProbeContext {
    pub(super) async fn handle_play_packet(
        &mut self,
        packet: PlayClientbound,
    ) -> Result<Option<ChunkPos>> {
        match packet {
            PlayClientbound::BundleDelimiter => {}
            PlayClientbound::AddEntity(entity) => {
                self.world.apply_add_entity(entity);
            }
            PlayClientbound::EntityAnimation(update) => {
                self.world.apply_entity_animation(update);
            }
            PlayClientbound::AwardStats(_) => {}
            PlayClientbound::BlockDestruction(update) => {
                self.world.apply_block_destruction(update);
            }
            PlayClientbound::BossEvent(update) => {
                self.world.apply_boss_event(update);
            }
            PlayClientbound::ChangeDifficulty(update) => {
                self.world.apply_change_difficulty(update);
            }
            PlayClientbound::Cooldown(update) => {
                self.world.apply_cooldown(update);
            }
            PlayClientbound::CustomChatCompletions(_)
            | PlayClientbound::CustomPayload(_)
            | PlayClientbound::CustomReportDetails(_)
            | PlayClientbound::ServerLinks(_) => {}
            PlayClientbound::DamageEvent(update) => {
                self.world.apply_damage_event(update);
            }
            PlayClientbound::DebugBlockValue(_)
            | PlayClientbound::DebugChunkValue(_)
            | PlayClientbound::DebugEntityValue(_)
            | PlayClientbound::DebugEvent(_)
            | PlayClientbound::DebugSample(_) => {}
            PlayClientbound::DeleteChat(update) => {
                self.world.apply_delete_chat(update);
            }
            PlayClientbound::DisguisedChat(update) => {
                self.world.apply_disguised_chat(update);
            }
            PlayClientbound::UpdateMobEffect(update) => {
                self.world.apply_update_mob_effect(update);
            }
            PlayClientbound::UpdateTags(update) => {
                self.world.apply_update_tags(update);
            }
            PlayClientbound::RemoveMobEffect(update) => {
                self.world.apply_remove_mob_effect(update);
            }
            PlayClientbound::MoveEntity(update) => {
                self.world.apply_entity_move(update);
            }
            PlayClientbound::MoveMinecartAlongTrack(update) => {
                self.world.apply_move_minecart_along_track(update);
            }
            PlayClientbound::MoveVehicle(update) => {
                self.world.apply_move_vehicle(update);
            }
            PlayClientbound::KeepAlive { id } => {
                let (id, payload) = packets::encode_play_keep_alive(id);
                self.conn.send_packet(id, &payload).await?;
            }
            PlayClientbound::LowDiskSpaceWarning | PlayClientbound::MountScreenOpen(_) => {}
            PlayClientbound::ChunkBatchStart => {}
            PlayClientbound::ChunkBatchFinished { .. } => {
                let (id, payload) = packets::encode_play_chunk_batch_received(9.0);
                self.conn.send_packet(id, &payload).await?;
            }
            PlayClientbound::ContainerClose(update) => {
                self.world.apply_container_close(update);
            }
            PlayClientbound::ContainerSetContent(update) => {
                self.world.apply_container_set_content(update);
            }
            PlayClientbound::ContainerSetData(update) => {
                self.world.apply_container_set_data(update);
            }
            PlayClientbound::ContainerSetSlot(update) => {
                self.world.apply_container_set_slot(update);
            }
            PlayClientbound::MerchantOffers(update) => {
                self.world.apply_merchant_offers(update);
            }
            PlayClientbound::CookieRequest(request) => {
                let payload = self.server_cookies.get(&request.key).map(Vec::as_slice);
                let (id, response) = packets::encode_play_cookie_response(&request.key, payload);
                self.conn.send_packet(id, &response).await?;
            }
            PlayClientbound::OpenScreen(update) => {
                self.world.apply_open_screen(update);
            }
            PlayClientbound::OpenBook(_)
            | PlayClientbound::OpenSignEditor(_)
            | PlayClientbound::PlaceGhostRecipe(_)
            | PlayClientbound::PongResponse(_) => {}
            PlayClientbound::Disconnect(disconnect) => {
                bail!("play disconnected: {}", disconnect.reason)
            }
            PlayClientbound::Ping { id } => {
                let (id, payload) = packets::encode_play_pong(id);
                self.conn.send_packet(id, &payload).await?;
            }
            PlayClientbound::StartConfiguration => {
                let (id, payload) = packets::encode_play_configuration_acknowledged();
                self.conn.send_packet(id, &payload).await?;
                self.state = ConnectionState::Configuration;
                self.play_tick = None;
            }
            PlayClientbound::StoreCookie(cookie) => {
                self.server_cookies.insert(cookie.key, cookie.payload);
            }
            PlayClientbound::Login(login) => {
                self.world.apply_login(&login);
            }
            PlayClientbound::Respawn(respawn) => {
                self.player_was_dead = false;
                self.world.apply_respawn(&respawn);
            }
            PlayClientbound::SetHealth(health) => {
                maybe_send_perform_respawn(&mut self.conn, health, &mut self.player_was_dead)
                    .await?;
            }
            PlayClientbound::EntityPositionSync(update) => {
                self.world.apply_entity_position_sync(update);
            }
            PlayClientbound::Explosion(_) => {}
            PlayClientbound::EntityEvent(update) => {
                self.world.apply_entity_event(update);
            }
            PlayClientbound::HurtAnimation(update) => {
                self.world.apply_hurt_animation(update);
            }
            PlayClientbound::RemoveEntities(update) => {
                self.world.apply_remove_entities(update);
            }
            PlayClientbound::RotateHead(update) => {
                self.world.apply_rotate_head(update);
            }
            PlayClientbound::SetEntityMotion(update) => {
                self.world.apply_set_entity_motion(update);
            }
            PlayClientbound::SetEntityLink(update) => {
                self.world.apply_set_entity_link(update);
            }
            PlayClientbound::SetEquipment(update) => {
                self.world.apply_set_equipment(update);
            }
            PlayClientbound::TakeItemEntity(update) => {
                self.world.apply_take_item_entity(update);
            }
            PlayClientbound::SetPassengers(update) => {
                self.world.apply_set_passengers(update);
            }
            PlayClientbound::UpdateAttributes(update) => {
                self.world.apply_update_attributes(update);
            }
            PlayClientbound::SetEntityData(update) => {
                self.world.apply_set_entity_data(update);
            }
            PlayClientbound::TeleportEntity(update) => {
                self.world.apply_teleport_entity(update);
            }
            PlayClientbound::PlayerAbilities(_) => {}
            PlayClientbound::PlayerChat(update) => {
                self.world.apply_player_chat(update);
            }
            PlayClientbound::SetExperience(_) => {}
            PlayClientbound::SetHeldSlot(_) => {}
            PlayClientbound::SetCursorItem(update) => {
                self.world.apply_set_cursor_item(update);
            }
            PlayClientbound::SetPlayerInventory(update) => {
                self.world.apply_set_player_inventory(update);
            }
            PlayClientbound::SetDefaultSpawnPosition(_) => {}
            PlayClientbound::SetSimulationDistance(_) => {}
            PlayClientbound::SystemChat(_) => {}
            PlayClientbound::PlayerCombatEnd(_)
            | PlayClientbound::PlayerCombatEnter
            | PlayClientbound::PlayerCombatKill(_)
            | PlayClientbound::PlayerLookAt(_) => {}
            PlayClientbound::MapItemData(update) => {
                self.world.apply_map_item_data(update);
            }
            PlayClientbound::SetActionBarText(_)
            | PlayClientbound::SetTitleText(_)
            | PlayClientbound::SetSubtitleText(_)
            | PlayClientbound::ClearTitles(_)
            | PlayClientbound::SetTitlesAnimation(_)
            | PlayClientbound::Sound(_)
            | PlayClientbound::SoundEntity(_)
            | PlayClientbound::StopSound(_)
            | PlayClientbound::TickingState(_)
            | PlayClientbound::TickingStep(_)
            | PlayClientbound::Transfer(_)
            | PlayClientbound::SetCamera(_) => {}
            PlayClientbound::InitializeBorder(border) => {
                self.world.apply_initialize_border(border);
            }
            PlayClientbound::SetBorderCenter(update) => {
                self.world.apply_set_border_center(update);
            }
            PlayClientbound::SetBorderLerpSize(update) => {
                self.world.apply_set_border_lerp_size(update);
            }
            PlayClientbound::SetBorderSize(update) => {
                self.world.apply_set_border_size(update);
            }
            PlayClientbound::SetBorderWarningDelay(update) => {
                self.world.apply_set_border_warning_delay(update);
            }
            PlayClientbound::SetBorderWarningDistance(update) => {
                self.world.apply_set_border_warning_distance(update);
            }
            PlayClientbound::ResetScore(update) => {
                self.world.apply_reset_score(update);
            }
            PlayClientbound::SetDisplayObjective(update) => {
                self.world.apply_set_display_objective(update);
            }
            PlayClientbound::SetObjective(update) => {
                self.world.apply_set_objective(update);
            }
            PlayClientbound::SetPlayerTeam(update) => {
                self.world.apply_set_player_team(update);
            }
            PlayClientbound::SetScore(update) => {
                self.world.apply_set_score(update);
            }
            PlayClientbound::Commands(update) => {
                self.world.apply_commands(update);
            }
            PlayClientbound::CommandSuggestions(update) => {
                self.world.apply_command_suggestions(update);
            }
            PlayClientbound::SelectAdvancementsTab(_)
            | PlayClientbound::TagQuery(_)
            | PlayClientbound::ClearDialog
            | PlayClientbound::ShowDialog(_)
            | PlayClientbound::TestInstanceBlockStatus(_) => {}
            PlayClientbound::TabList(update) => {
                self.world.apply_tab_list(update);
            }
            PlayClientbound::BlockChangedAck(_) => {}
            PlayClientbound::BlockEntityData(update) => {
                self.world.apply_block_entity_data(update)?;
            }
            PlayClientbound::BlockEvent(event) => {
                self.world.apply_block_event(event);
            }
            PlayClientbound::LevelEvent(event) => {
                self.world.apply_level_event(event);
            }
            PlayClientbound::GameEvent(_)
            | PlayClientbound::GameRuleValues(_)
            | PlayClientbound::GameTestHighlightPos(_)
            | PlayClientbound::SetTime(_) => {}
            PlayClientbound::PlayerPosition(update) => {
                self.player_position_state = update.apply_to_state(self.player_position_state);
                let (id, payload) = packets::encode_play_accept_teleportation(update.id);
                self.conn.send_packet(id, &payload).await?;
                let (id, payload) = packets::encode_play_move_player_pos_rot(
                    self.player_position_state.position.x,
                    self.player_position_state.position.y,
                    self.player_position_state.position.z,
                    self.player_position_state.y_rot,
                    self.player_position_state.x_rot,
                    false,
                    false,
                );
                self.conn.send_packet(id, &payload).await?;
                if !self.player_loaded_sent {
                    let (id, payload) = packets::encode_play_player_loaded();
                    self.conn.send_packet(id, &payload).await?;
                    self.player_loaded_sent = true;
                }
            }
            PlayClientbound::PlayerRotation(update) => {
                self.player_position_state = update.apply_to_state(self.player_position_state);
            }
            PlayClientbound::PlayerInfoUpdate(update) => {
                self.world.apply_player_info_update(update);
            }
            PlayClientbound::PlayerInfoRemove(update) => {
                self.world.apply_player_info_remove(update);
            }
            PlayClientbound::ServerData(update) => {
                self.world.apply_server_data(update);
            }
            PlayClientbound::ResourcePackPush(update) => {
                let (id, payload) = packets::encode_play_resource_pack_response(
                    update.id,
                    ResourcePackResponseAction::Declined,
                );
                self.conn.send_packet(id, &payload).await?;
                self.world.apply_resource_pack_push(update);
            }
            PlayClientbound::ResourcePackPop(update) => {
                self.world.apply_resource_pack_pop(update);
            }
            PlayClientbound::LevelChunkWithLight(chunk) => {
                return Ok(Some(self.world.insert_level_chunk_with_light(chunk)?));
            }
            PlayClientbound::LevelParticles(_) => {}
            PlayClientbound::LightUpdate(update) => {
                self.world.apply_light_update(update)?;
            }
            PlayClientbound::ChunksBiomes(update) => {
                self.world.apply_biome_update(update)?;
            }
            PlayClientbound::ForgetLevelChunk(update) => {
                self.world.forget_chunk(ChunkPos {
                    x: update.pos.x,
                    z: update.pos.z,
                });
            }
            PlayClientbound::BlockUpdate(update) => {
                self.world.apply_block_update(update);
            }
            PlayClientbound::SectionBlocksUpdate(update) => {
                self.world.apply_section_blocks_update(update);
            }
            PlayClientbound::SetChunkCacheCenter(_)
            | PlayClientbound::SetChunkCacheRadius(_)
            | PlayClientbound::ProjectilePower(_)
            | PlayClientbound::Waypoint(_) => {}
            PlayClientbound::RecipeBookAdd(update) => {
                self.world.apply_recipe_book_add(update);
            }
            PlayClientbound::RecipeBookRemove(update) => {
                self.world.apply_recipe_book_remove(update);
            }
            PlayClientbound::RecipeBookSettings(update) => {
                self.world.apply_recipe_book_settings(update);
            }
            PlayClientbound::UpdateAdvancements(update) => {
                self.world.apply_update_advancements(update);
            }
            PlayClientbound::UpdateRecipes(update) => {
                self.world.apply_update_recipes(update);
            }
            PlayClientbound::Unknown { .. } => {}
        }
        Ok(None)
    }
}
