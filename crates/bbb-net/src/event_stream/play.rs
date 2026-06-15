use anyhow::{bail, Result};
use bbb_protocol::packets::{self, PlayClientbound, ResourcePackResponseAction};

use crate::{
    driver::maybe_send_perform_respawn,
    event_stream::{emit, EventStreamContext},
    types::{ConnectionState, NetEvent},
};

impl EventStreamContext {
    pub(super) async fn handle_play_packet(&mut self, packet: PlayClientbound) -> Result<()> {
        match packet {
            PlayClientbound::BundleDelimiter => {}
            PlayClientbound::AddEntity(entity) => {
                emit(&self.events, NetEvent::AddEntity(entity)).await?;
            }
            PlayClientbound::EntityAnimation(update) => {
                emit(&self.events, NetEvent::EntityAnimation(update)).await?;
            }
            PlayClientbound::AwardStats(update) => {
                emit(&self.events, NetEvent::AwardStats(update)).await?;
            }
            PlayClientbound::BlockDestruction(update) => {
                emit(&self.events, NetEvent::BlockDestruction(update)).await?;
            }
            PlayClientbound::BossEvent(update) => {
                emit(&self.events, NetEvent::BossEvent(update)).await?;
            }
            PlayClientbound::ChangeDifficulty(update) => {
                emit(&self.events, NetEvent::ChangeDifficulty(update)).await?;
            }
            PlayClientbound::Cooldown(update) => {
                emit(&self.events, NetEvent::Cooldown(update)).await?;
            }
            PlayClientbound::CustomChatCompletions(update) => {
                emit(&self.events, NetEvent::CustomChatCompletions(update)).await?;
            }
            PlayClientbound::CustomPayload(update) => {
                emit(&self.events, NetEvent::CustomPayload(update)).await?;
            }
            PlayClientbound::DamageEvent(update) => {
                emit(&self.events, NetEvent::DamageEvent(update)).await?;
            }
            PlayClientbound::DebugBlockValue(update) => {
                emit(&self.events, NetEvent::DebugBlockValue(update)).await?;
            }
            PlayClientbound::DebugChunkValue(update) => {
                emit(&self.events, NetEvent::DebugChunkValue(update)).await?;
            }
            PlayClientbound::DebugEntityValue(update) => {
                emit(&self.events, NetEvent::DebugEntityValue(update)).await?;
            }
            PlayClientbound::DebugEvent(update) => {
                emit(&self.events, NetEvent::DebugEvent(update)).await?;
            }
            PlayClientbound::DebugSample(update) => {
                emit(&self.events, NetEvent::DebugSample(update)).await?;
            }
            PlayClientbound::DeleteChat(update) => {
                emit(&self.events, NetEvent::DeleteChat(update)).await?;
            }
            PlayClientbound::DisguisedChat(update) => {
                emit(&self.events, NetEvent::DisguisedChat(update)).await?;
            }
            PlayClientbound::UpdateMobEffect(update) => {
                emit(&self.events, NetEvent::UpdateMobEffect(update)).await?;
            }
            PlayClientbound::UpdateTags(update) => {
                emit(&self.events, NetEvent::UpdateTags(update)).await?;
            }
            PlayClientbound::RemoveMobEffect(update) => {
                emit(&self.events, NetEvent::RemoveMobEffect(update)).await?;
            }
            PlayClientbound::MoveEntity(update) => {
                emit(&self.events, NetEvent::MoveEntity(update)).await?;
            }
            PlayClientbound::MoveMinecartAlongTrack(update) => {
                emit(&self.events, NetEvent::MoveMinecartAlongTrack(update)).await?;
            }
            PlayClientbound::MoveVehicle(update) => {
                emit(&self.events, NetEvent::MoveVehicle(update)).await?;
            }
            PlayClientbound::KeepAlive { id } => {
                let (id, payload) = packets::encode_play_keep_alive(id);
                self.conn.send_packet(id, &payload).await?;
            }
            PlayClientbound::LowDiskSpaceWarning => {
                emit(&self.events, NetEvent::LowDiskSpaceWarning).await?;
            }
            PlayClientbound::MapItemData(update) => {
                emit(&self.events, NetEvent::MapItemData(update)).await?;
            }
            PlayClientbound::MountScreenOpen(update) => {
                emit(&self.events, NetEvent::MountScreenOpen(update)).await?;
            }
            PlayClientbound::ChunkBatchStart => {}
            PlayClientbound::ChunkBatchFinished { .. } => {
                let (id, payload) = packets::encode_play_chunk_batch_received(9.0);
                self.conn.send_packet(id, &payload).await?;
            }
            PlayClientbound::ContainerClose(update) => {
                emit(&self.events, NetEvent::ContainerClose(update)).await?;
            }
            PlayClientbound::ContainerSetContent(update) => {
                emit(&self.events, NetEvent::ContainerSetContent(update)).await?;
            }
            PlayClientbound::ContainerSetData(update) => {
                emit(&self.events, NetEvent::ContainerSetData(update)).await?;
            }
            PlayClientbound::ContainerSetSlot(update) => {
                emit(&self.events, NetEvent::ContainerSetSlot(update)).await?;
            }
            PlayClientbound::MerchantOffers(update) => {
                emit(&self.events, NetEvent::MerchantOffers(update)).await?;
            }
            PlayClientbound::CookieRequest(request) => {
                let payload = self.server_cookies.get(&request.key).map(Vec::as_slice);
                let payload_present = payload.is_some();
                let (id, response) = packets::encode_play_cookie_response(&request.key, payload);
                self.conn.send_packet(id, &response).await?;
                emit(
                    &self.events,
                    NetEvent::CookieRequest {
                        key: request.key,
                        response_payload_present: payload_present,
                    },
                )
                .await?;
            }
            PlayClientbound::CustomReportDetails(details) => {
                emit(&self.events, NetEvent::CustomReportDetails(details)).await?;
            }
            PlayClientbound::ServerLinks(links) => {
                emit(&self.events, NetEvent::ServerLinks(links)).await?;
            }
            PlayClientbound::OpenScreen(update) => {
                emit(&self.events, NetEvent::OpenScreen(update)).await?;
            }
            PlayClientbound::OpenBook(update) => {
                emit(&self.events, NetEvent::OpenBook(update)).await?;
            }
            PlayClientbound::OpenSignEditor(update) => {
                emit(&self.events, NetEvent::OpenSignEditor(update)).await?;
            }
            PlayClientbound::Disconnect(disconnect) => {
                bail!("play disconnected: {}", disconnect.reason)
            }
            PlayClientbound::Ping { id } => {
                let (id, payload) = packets::encode_play_pong(id);
                self.conn.send_packet(id, &payload).await?;
            }
            PlayClientbound::PongResponse(update) => {
                emit(&self.events, NetEvent::PongResponse(update)).await?;
            }
            PlayClientbound::PlaceGhostRecipe(update) => {
                emit(&self.events, NetEvent::PlaceGhostRecipe(update)).await?;
            }
            PlayClientbound::StartConfiguration => {
                let (id, payload) = packets::encode_play_configuration_acknowledged();
                self.conn.send_packet(id, &payload).await?;
                self.state = ConnectionState::Configuration;
                self.play_tick = None;
                self.seen_code_of_conduct = false;
                emit(&self.events, NetEvent::StateChanged { state: self.state }).await?;
            }
            PlayClientbound::StoreCookie(cookie) => {
                let key = cookie.key;
                let payload_len = cookie.payload.len();
                self.server_cookies.insert(key.clone(), cookie.payload);
                emit(
                    &self.events,
                    NetEvent::StoreCookie {
                        key,
                        payload_len,
                        stored_cookie_count: self.server_cookies.len(),
                    },
                )
                .await?;
            }
            PlayClientbound::Login(login) => {
                emit(&self.events, NetEvent::Login(login)).await?;
            }
            PlayClientbound::Respawn(respawn) => {
                self.player_was_dead = false;
                emit(&self.events, NetEvent::Respawn(respawn)).await?;
            }
            PlayClientbound::PlayerCombatEnd(update) => {
                emit(&self.events, NetEvent::PlayerCombatEnd(update)).await?;
            }
            PlayClientbound::PlayerCombatEnter => {
                emit(&self.events, NetEvent::PlayerCombatEnter).await?;
            }
            PlayClientbound::PlayerCombatKill(update) => {
                emit(&self.events, NetEvent::PlayerCombatKill(update)).await?;
            }
            PlayClientbound::PlayerChat(update) => {
                emit(&self.events, NetEvent::PlayerChat(update)).await?;
            }
            PlayClientbound::SetHealth(health) => {
                maybe_send_perform_respawn(&mut self.conn, health, &mut self.player_was_dead)
                    .await?;
                emit(&self.events, NetEvent::PlayerHealth(health)).await?;
            }
            PlayClientbound::SetExperience(experience) => {
                emit(&self.events, NetEvent::PlayerExperience(experience)).await?;
            }
            PlayClientbound::SetHeldSlot(slot) => {
                emit(&self.events, NetEvent::HeldSlot(slot)).await?;
            }
            PlayClientbound::SetCursorItem(update) => {
                emit(&self.events, NetEvent::SetCursorItem(update)).await?;
            }
            PlayClientbound::SetPlayerInventory(update) => {
                emit(&self.events, NetEvent::SetPlayerInventory(update)).await?;
            }
            PlayClientbound::GameEvent(event) => {
                emit(&self.events, NetEvent::GameEvent(event)).await?;
            }
            PlayClientbound::GameRuleValues(update) => {
                emit(&self.events, NetEvent::GameRuleValues(update)).await?;
            }
            PlayClientbound::GameTestHighlightPos(update) => {
                emit(&self.events, NetEvent::GameTestHighlightPos(update)).await?;
            }
            PlayClientbound::SetTime(time) => {
                emit(&self.events, NetEvent::SetTime(time)).await?;
            }
            PlayClientbound::BlockChangedAck(ack) => {
                emit(&self.events, NetEvent::BlockChangedAck(ack)).await?;
            }
            PlayClientbound::BlockEntityData(update) => {
                emit(&self.events, NetEvent::BlockEntityData(update)).await?;
            }
            PlayClientbound::BlockEvent(event) => {
                emit(&self.events, NetEvent::BlockEvent(event)).await?;
            }
            PlayClientbound::PlayerLookAt(update) => {
                emit(&self.events, NetEvent::PlayerLookAt(update)).await?;
            }
            PlayClientbound::LevelEvent(event) => {
                emit(&self.events, NetEvent::LevelEvent(event)).await?;
            }
            PlayClientbound::PlayerPosition(update) => {
                self.player_position_state = update.apply_to_state(self.player_position_state);
                emit(&self.events, NetEvent::PlayerPosition(update)).await?;
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
                emit(&self.events, NetEvent::PlayerRotation(update)).await?;
            }
            PlayClientbound::PlayerInfoUpdate(update) => {
                emit(&self.events, NetEvent::PlayerInfoUpdate(update)).await?;
            }
            PlayClientbound::PlayerInfoRemove(update) => {
                emit(&self.events, NetEvent::PlayerInfoRemove(update)).await?;
            }
            PlayClientbound::ServerData(update) => {
                emit(&self.events, NetEvent::ServerData(update)).await?;
            }
            PlayClientbound::ResourcePackPush(update) => {
                let (id, payload) = packets::encode_play_resource_pack_response(
                    update.id,
                    ResourcePackResponseAction::Declined,
                );
                self.conn.send_packet(id, &payload).await?;
                emit(&self.events, NetEvent::ResourcePackPush(update)).await?;
            }
            PlayClientbound::ResourcePackPop(update) => {
                emit(&self.events, NetEvent::ResourcePackPop(update)).await?;
            }
            PlayClientbound::EntityPositionSync(update) => {
                emit(&self.events, NetEvent::EntityPositionSync(update)).await?;
            }
            PlayClientbound::Explosion(update) => {
                emit(&self.events, NetEvent::Explosion(update)).await?;
            }
            PlayClientbound::EntityEvent(update) => {
                emit(&self.events, NetEvent::EntityEvent(update)).await?;
            }
            PlayClientbound::HurtAnimation(update) => {
                emit(&self.events, NetEvent::HurtAnimation(update)).await?;
            }
            PlayClientbound::RemoveEntities(update) => {
                emit(&self.events, NetEvent::RemoveEntities(update)).await?;
            }
            PlayClientbound::RotateHead(update) => {
                emit(&self.events, NetEvent::RotateHead(update)).await?;
            }
            PlayClientbound::SetEntityMotion(update) => {
                emit(&self.events, NetEvent::SetEntityMotion(update)).await?;
            }
            PlayClientbound::SetEntityLink(update) => {
                emit(&self.events, NetEvent::SetEntityLink(update)).await?;
            }
            PlayClientbound::SetEquipment(update) => {
                emit(&self.events, NetEvent::SetEquipment(update)).await?;
            }
            PlayClientbound::TakeItemEntity(update) => {
                emit(&self.events, NetEvent::TakeItemEntity(update)).await?;
            }
            PlayClientbound::SetPassengers(update) => {
                emit(&self.events, NetEvent::SetPassengers(update)).await?;
            }
            PlayClientbound::UpdateAttributes(update) => {
                emit(&self.events, NetEvent::UpdateAttributes(update)).await?;
            }
            PlayClientbound::SetEntityData(update) => {
                emit(&self.events, NetEvent::SetEntityData(update)).await?;
            }
            PlayClientbound::TeleportEntity(update) => {
                emit(&self.events, NetEvent::TeleportEntity(update)).await?;
            }
            PlayClientbound::PlayerAbilities(abilities) => {
                emit(&self.events, NetEvent::PlayerAbilities(abilities)).await?;
            }
            PlayClientbound::SetDefaultSpawnPosition(spawn) => {
                emit(&self.events, NetEvent::SetDefaultSpawnPosition(spawn)).await?;
            }
            PlayClientbound::SetSimulationDistance(distance) => {
                emit(&self.events, NetEvent::SetSimulationDistance(distance)).await?;
            }
            PlayClientbound::SystemChat(chat) => {
                emit(&self.events, NetEvent::SystemChat(chat)).await?;
            }
            PlayClientbound::SetActionBarText(text) => {
                emit(&self.events, NetEvent::SetActionBarText(text)).await?;
            }
            PlayClientbound::SetTitleText(text) => {
                emit(&self.events, NetEvent::SetTitleText(text)).await?;
            }
            PlayClientbound::SetSubtitleText(text) => {
                emit(&self.events, NetEvent::SetSubtitleText(text)).await?;
            }
            PlayClientbound::ClearTitles(clear) => {
                emit(&self.events, NetEvent::ClearTitles(clear)).await?;
            }
            PlayClientbound::SetTitlesAnimation(animation) => {
                emit(&self.events, NetEvent::SetTitlesAnimation(animation)).await?;
            }
            PlayClientbound::ShowDialog(update) => {
                emit(&self.events, NetEvent::ShowDialog(update)).await?;
            }
            PlayClientbound::Sound(sound) => {
                emit(&self.events, NetEvent::Sound(sound)).await?;
            }
            PlayClientbound::SoundEntity(sound) => {
                emit(&self.events, NetEvent::SoundEntity(sound)).await?;
            }
            PlayClientbound::StopSound(stop) => {
                emit(&self.events, NetEvent::StopSound(stop)).await?;
            }
            PlayClientbound::TickingState(ticking) => {
                emit(&self.events, NetEvent::TickingState(ticking)).await?;
            }
            PlayClientbound::TickingStep(step) => {
                emit(&self.events, NetEvent::TickingStep(step)).await?;
            }
            PlayClientbound::Transfer(transfer) => {
                emit(&self.events, NetEvent::Transfer(transfer)).await?;
            }
            PlayClientbound::UpdateAdvancements(update) => {
                emit(&self.events, NetEvent::UpdateAdvancements(update)).await?;
            }
            PlayClientbound::UpdateRecipes(update) => {
                emit(&self.events, NetEvent::UpdateRecipes(update)).await?;
            }
            PlayClientbound::Waypoint(update) => {
                emit(&self.events, NetEvent::Waypoint(update)).await?;
            }
            PlayClientbound::SetCamera(camera) => {
                emit(&self.events, NetEvent::SetCamera(camera)).await?;
            }
            PlayClientbound::InitializeBorder(border) => {
                emit(&self.events, NetEvent::InitializeBorder(border)).await?;
            }
            PlayClientbound::SetBorderCenter(update) => {
                emit(&self.events, NetEvent::SetBorderCenter(update)).await?;
            }
            PlayClientbound::SetBorderLerpSize(update) => {
                emit(&self.events, NetEvent::SetBorderLerpSize(update)).await?;
            }
            PlayClientbound::SetBorderSize(update) => {
                emit(&self.events, NetEvent::SetBorderSize(update)).await?;
            }
            PlayClientbound::SetBorderWarningDelay(update) => {
                emit(&self.events, NetEvent::SetBorderWarningDelay(update)).await?;
            }
            PlayClientbound::SetBorderWarningDistance(update) => {
                emit(&self.events, NetEvent::SetBorderWarningDistance(update)).await?;
            }
            PlayClientbound::ResetScore(update) => {
                emit(&self.events, NetEvent::ResetScore(update)).await?;
            }
            PlayClientbound::SetDisplayObjective(update) => {
                emit(&self.events, NetEvent::SetDisplayObjective(update)).await?;
            }
            PlayClientbound::SetObjective(update) => {
                emit(&self.events, NetEvent::SetObjective(update)).await?;
            }
            PlayClientbound::SetPlayerTeam(update) => {
                emit(&self.events, NetEvent::SetPlayerTeam(update)).await?;
            }
            PlayClientbound::SetScore(update) => {
                emit(&self.events, NetEvent::SetScore(update)).await?;
            }
            PlayClientbound::Commands(update) => {
                emit(&self.events, NetEvent::Commands(update)).await?;
            }
            PlayClientbound::CommandSuggestions(update) => {
                emit(&self.events, NetEvent::CommandSuggestions(update)).await?;
            }
            PlayClientbound::SelectAdvancementsTab(update) => {
                emit(&self.events, NetEvent::SelectAdvancementsTab(update)).await?;
            }
            PlayClientbound::TabList(update) => {
                emit(&self.events, NetEvent::TabList(update)).await?;
            }
            PlayClientbound::TagQuery(update) => {
                emit(&self.events, NetEvent::TagQuery(update)).await?;
            }
            PlayClientbound::TestInstanceBlockStatus(update) => {
                emit(&self.events, NetEvent::TestInstanceBlockStatus(update)).await?;
            }
            PlayClientbound::ClearDialog => {
                emit(&self.events, NetEvent::ClearDialog).await?;
            }
            PlayClientbound::LevelChunkWithLight(chunk) => {
                emit(&self.events, NetEvent::LevelChunkWithLight(chunk)).await?;
            }
            PlayClientbound::LevelParticles(update) => {
                emit(&self.events, NetEvent::LevelParticles(update)).await?;
            }
            PlayClientbound::LightUpdate(update) => {
                emit(&self.events, NetEvent::LightUpdate(update)).await?;
            }
            PlayClientbound::ChunksBiomes(update) => {
                emit(&self.events, NetEvent::ChunksBiomes(update)).await?;
            }
            PlayClientbound::ForgetLevelChunk(update) => {
                emit(&self.events, NetEvent::ForgetLevelChunk(update)).await?;
            }
            PlayClientbound::BlockUpdate(update) => {
                emit(&self.events, NetEvent::BlockUpdate(update)).await?;
            }
            PlayClientbound::SectionBlocksUpdate(update) => {
                emit(&self.events, NetEvent::SectionBlocksUpdate(update)).await?;
            }
            PlayClientbound::SetChunkCacheCenter(update) => {
                emit(&self.events, NetEvent::SetChunkCacheCenter(update)).await?;
            }
            PlayClientbound::SetChunkCacheRadius(update) => {
                emit(&self.events, NetEvent::SetChunkCacheRadius(update)).await?;
            }
            PlayClientbound::ProjectilePower(update) => {
                emit(&self.events, NetEvent::ProjectilePower(update)).await?;
            }
            PlayClientbound::RecipeBookAdd(update) => {
                emit(&self.events, NetEvent::RecipeBookAdd(update)).await?;
            }
            PlayClientbound::RecipeBookRemove(update) => {
                emit(&self.events, NetEvent::RecipeBookRemove(update)).await?;
            }
            PlayClientbound::RecipeBookSettings(update) => {
                emit(&self.events, NetEvent::RecipeBookSettings(update)).await?;
            }
            PlayClientbound::Unknown { .. } => {}
        }
        Ok(())
    }
}
