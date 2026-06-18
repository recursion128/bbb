use anyhow::{bail, Result};
use bbb_protocol::packets::{self, PlayClientbound};
use bbb_world::ChunkPos;

use crate::{probe::ProbeContext, resource_pack::response_action_for_push, types::ConnectionState};

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
            PlayClientbound::AwardStats(update) => {
                self.world.apply_award_stats(update);
            }
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
            PlayClientbound::CustomChatCompletions(update) => {
                self.world.apply_custom_chat_completions(update);
            }
            PlayClientbound::CustomPayload(payload) => {
                self.world.apply_custom_payload(payload);
            }
            PlayClientbound::ServerLinks(links) => {
                self.world.apply_server_links(links);
            }
            PlayClientbound::CustomReportDetails(details) => {
                self.world.apply_custom_report_details(details);
            }
            PlayClientbound::DamageEvent(update) => {
                self.world.apply_damage_event(update);
            }
            PlayClientbound::DebugBlockValue(update) => {
                self.world.apply_debug_block_value(update);
            }
            PlayClientbound::DebugChunkValue(update) => {
                self.world.apply_debug_chunk_value(update);
            }
            PlayClientbound::DebugEntityValue(update) => {
                self.world.apply_debug_entity_value(update);
            }
            PlayClientbound::DebugEvent(update) => {
                self.world.apply_debug_event(update);
            }
            PlayClientbound::DebugSample(update) => {
                self.world.apply_debug_sample(update);
            }
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
                if let Some(report) = self.world.apply_move_vehicle(update) {
                    let (id, payload) = packets::encode_play_move_vehicle(
                        report.position.x,
                        report.position.y,
                        report.position.z,
                        report.y_rot,
                        report.x_rot,
                        report.on_ground,
                    );
                    self.conn.send_packet(id, &payload).await?;
                }
            }
            PlayClientbound::KeepAlive { id } => {
                let (id, payload) = packets::encode_play_keep_alive(id);
                self.conn.send_packet(id, &payload).await?;
            }
            PlayClientbound::LowDiskSpaceWarning => {
                self.world.apply_low_disk_space_warning();
            }
            PlayClientbound::MountScreenOpen(update) => {
                self.world.apply_mount_screen_open(update);
            }
            PlayClientbound::ChunkBatchStart => {
                self.chunk_batch_size.on_batch_start();
            }
            PlayClientbound::ChunkBatchFinished { batch_size } => {
                let desired_chunks_per_tick = self.chunk_batch_size.on_batch_finished(batch_size);
                let (id, payload) =
                    packets::encode_play_chunk_batch_received(desired_chunks_per_tick);
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
                let payload_present = payload.is_some();
                let (id, response) = packets::encode_play_cookie_response(&request.key, payload);
                self.conn.send_packet(id, &response).await?;
                self.world
                    .apply_cookie_request(request.key, payload_present);
            }
            PlayClientbound::OpenScreen(update) => {
                self.world.apply_open_screen(update);
            }
            PlayClientbound::OpenBook(update) => {
                self.world.apply_open_book(update);
            }
            PlayClientbound::OpenSignEditor(update) => {
                self.world.apply_open_sign_editor(update);
            }
            PlayClientbound::PlaceGhostRecipe(update) => {
                self.world.apply_place_ghost_recipe(update);
            }
            PlayClientbound::PongResponse(update) => {
                self.world.apply_pong_response(update);
            }
            PlayClientbound::Disconnect(disconnect) => {
                bail!("play disconnected: {}", disconnect.reason)
            }
            PlayClientbound::Ping { id } => {
                let (id, payload) = packets::encode_play_pong(id);
                self.conn.send_packet(id, &payload).await?;
            }
            PlayClientbound::StartConfiguration => {
                if let Some(command) = self.world.take_pending_player_chat_acknowledgement() {
                    let (id, payload) = packets::encode_play_chat_acknowledgement(command);
                    self.conn.send_packet(id, &payload).await?;
                }
                let (id, payload) = packets::encode_play_configuration_acknowledged();
                self.conn.send_packet(id, &payload).await?;
                self.state = ConnectionState::Configuration;
                self.play_tick = None;
                self.seen_code_of_conduct = false;
                self.world.clear_client_level();
            }
            PlayClientbound::StoreCookie(cookie) => {
                let key = cookie.key;
                let payload_len = cookie.payload.len();
                self.server_cookies.insert(key.clone(), cookie.payload);
                self.world
                    .apply_store_cookie(key, payload_len, self.server_cookies.len());
            }
            PlayClientbound::Login(login) => {
                self.world.apply_login(&login);
            }
            PlayClientbound::Respawn(respawn) => {
                self.world.apply_respawn(&respawn);
            }
            PlayClientbound::SetHealth(health) => {
                self.world.apply_player_health(health);
            }
            PlayClientbound::EntityPositionSync(update) => {
                self.world.apply_entity_position_sync(update);
            }
            PlayClientbound::Explosion(update) => {
                self.world.apply_explosion(update);
            }
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
            PlayClientbound::PlayerAbilities(update) => {
                self.world.apply_player_abilities(update);
            }
            PlayClientbound::PlayerChat(update) => {
                if let Some(command) = self.world.apply_player_chat(update) {
                    let (id, payload) = packets::encode_play_chat_acknowledgement(command);
                    self.conn.send_packet(id, &payload).await?;
                }
            }
            PlayClientbound::SetExperience(update) => {
                self.world.apply_player_experience(update);
            }
            PlayClientbound::SetHeldSlot(update) => {
                self.world.apply_held_slot(update);
            }
            PlayClientbound::SetCursorItem(update) => {
                self.world.apply_set_cursor_item(update);
            }
            PlayClientbound::SetPlayerInventory(update) => {
                self.world.apply_set_player_inventory(update);
            }
            PlayClientbound::SetDefaultSpawnPosition(update) => {
                self.world.apply_default_spawn_position(update);
            }
            PlayClientbound::SetSimulationDistance(update) => {
                self.world.apply_simulation_distance(update);
            }
            PlayClientbound::SystemChat(update) => {
                self.world.apply_system_chat(update);
            }
            PlayClientbound::PlayerCombatEnd(update) => {
                self.world.apply_player_combat_end(update);
            }
            PlayClientbound::PlayerCombatEnter => {
                self.world.apply_player_combat_enter();
            }
            PlayClientbound::PlayerCombatKill(update) => {
                self.world.apply_player_combat_kill(update);
            }
            PlayClientbound::PlayerLookAt(update) => {
                self.world.apply_player_look_at(update);
            }
            PlayClientbound::MapItemData(update) => {
                self.world.apply_map_item_data(update);
            }
            PlayClientbound::SetActionBarText(update) => {
                self.world.apply_action_bar_text(update);
            }
            PlayClientbound::SetTitleText(update) => {
                self.world.apply_title_text(update);
            }
            PlayClientbound::SetSubtitleText(update) => {
                self.world.apply_subtitle_text(update);
            }
            PlayClientbound::ClearTitles(update) => {
                self.world.apply_clear_titles(update);
            }
            PlayClientbound::SetTitlesAnimation(update) => {
                self.world.apply_titles_animation(update);
            }
            PlayClientbound::Sound(update) => {
                self.world.apply_sound_event(update);
            }
            PlayClientbound::SoundEntity(update) => {
                self.world.apply_sound_entity_event(update);
            }
            PlayClientbound::StopSound(update) => {
                self.world.apply_stop_sound(update);
            }
            PlayClientbound::TickingState(update) => {
                self.world.apply_ticking_state(update);
            }
            PlayClientbound::TickingStep(update) => {
                self.world.apply_ticking_step(update);
            }
            PlayClientbound::Transfer(update) => {
                self.world.apply_transfer(update);
            }
            PlayClientbound::SetCamera(update) => {
                self.world.apply_set_camera(update);
            }
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
            PlayClientbound::SelectAdvancementsTab(update) => {
                self.world.apply_select_advancements_tab(update);
            }
            PlayClientbound::TagQuery(update) => {
                self.world.apply_tag_query(update);
            }
            PlayClientbound::ClearDialog => {
                self.world.apply_clear_dialog();
            }
            PlayClientbound::ShowDialog(update) => {
                self.world.apply_show_dialog(update);
            }
            PlayClientbound::TestInstanceBlockStatus(update) => {
                self.world.apply_test_instance_block_status(update);
            }
            PlayClientbound::TabList(update) => {
                self.world.apply_tab_list(update);
            }
            PlayClientbound::BlockChangedAck(update) => {
                self.world.apply_block_changed_ack(update);
            }
            PlayClientbound::BlockEntityData(update) => {
                let _ = self.world.apply_block_entity_data(update);
            }
            PlayClientbound::BlockEvent(event) => {
                self.world.apply_block_event(event);
            }
            PlayClientbound::LevelEvent(event) => {
                self.world.apply_level_event(event);
            }
            PlayClientbound::GameEvent(update) => {
                self.world.apply_game_event(update);
            }
            PlayClientbound::GameRuleValues(update) => {
                self.world.apply_game_rule_values(update);
            }
            PlayClientbound::GameTestHighlightPos(update) => {
                self.world.apply_game_test_highlight_pos(update);
            }
            PlayClientbound::SetTime(update) => {
                self.world.apply_world_time(update);
            }
            PlayClientbound::PlayerPosition(update) => {
                self.world.apply_player_position(update);
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
                self.world.apply_player_rotation(update);
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
                let pack_id = update.id;
                let action = response_action_for_push(&update);
                let (id, payload) = packets::encode_play_resource_pack_response(pack_id, action);
                self.conn.send_packet(id, &payload).await?;
                self.world.apply_resource_pack_push(update);
                self.world.apply_resource_pack_response(pack_id, action);
            }
            PlayClientbound::ResourcePackPop(update) => {
                self.world.apply_resource_pack_pop(update);
            }
            PlayClientbound::LevelChunkWithLight(chunk) => {
                match self.world.insert_level_chunk_with_light(chunk) {
                    Ok(pos) => return Ok(Some(pos)),
                    Err(_) => {}
                }
            }
            PlayClientbound::LevelParticles(update) => {
                self.world.apply_level_particles(update);
            }
            PlayClientbound::LightUpdate(update) => {
                let _ = self.world.apply_light_update(update);
            }
            PlayClientbound::ChunksBiomes(update) => {
                let _ = self.world.apply_biome_update(update);
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
            PlayClientbound::SetChunkCacheCenter(update) => {
                self.world.apply_set_chunk_cache_center(update);
            }
            PlayClientbound::SetChunkCacheRadius(update) => {
                self.world.apply_set_chunk_cache_radius(update);
            }
            PlayClientbound::ProjectilePower(update) => {
                self.world.apply_projectile_power(update);
            }
            PlayClientbound::Waypoint(update) => {
                self.world.apply_waypoint(update);
            }
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
            PlayClientbound::Unknown { packet_id, len } => {
                self.record_unsupported_packet(self.state, packet_id, len);
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::RawConnection;
    use bbb_protocol::packets::{
        AddEntity, AdvancementCriterionProgressSummary, AdvancementProgressSummary,
        AdvancementSummary, AwardStats, BlockChangedAck, BlockEntityData, BlockEvent,
        BlockPos as ProtocolBlockPos, BossBarColor, BossBarOverlay, BossEvent, BossEventFlags,
        BossEventOperation, ChangeDifficulty, ChatFormatting, ChatTypeBound, ChatTypeHolder,
        ChunkHeightmapData, ChunkPos as ProtocolChunkPos, ClockUpdate, CommandSuggestion,
        CommandSuggestions, CommonPlayerSpawnInfo, CookieRequest, CustomChatCompletions,
        CustomChatCompletionsAction, CustomPayload, CustomPayloadBody, CustomReportDetails,
        DebugBlockValue, DebugChunkValue, DebugEntityValue, DebugEvent, DebugSample, DialogHolder,
        Difficulty, EntityAnchor, Explosion, FilterMask, FilterMaskKind, GameEvent, GameProfile,
        GameProfileProperty, GameRuleValue, GameRuleValues, GameTestHighlightPos, GameType,
        IngredientSummary, InteractionHand, LevelChunkBlockEntity, LevelChunkData,
        LevelChunkWithLight, LevelEvent, LevelParticles, LightUpdateData, MapColorPatch,
        MapDecoration, MapItemData, MessageSignature, MountScreenOpen, MoveVehicle,
        ObjectiveRenderType, OpenBook, OpenSignEditor, ParticlePayload, PlaceGhostRecipe,
        PlayLogin, PlayTime, PlayerAbilities, PlayerChat, PlayerExperience, PlayerHealth,
        PlayerInfoAction, PlayerInfoChatSession, PlayerInfoEntry, PlayerInfoRemove,
        PlayerInfoUpdate, PlayerLookAt, PlayerPositionUpdate, PlayerRotationUpdate,
        PlayerTeamMethod, PlayerTeamParameters, PongResponse, ProjectilePower, RecipeBookAdd,
        RecipeBookAddEntry, RecipeBookRemove, RecipeBookSettings, RecipeBookTypeSettings,
        RecipeDisplayEntry, RecipeDisplayId, RecipeDisplaySummary, RecipeDisplayType,
        RecipePropertySetSummary, RemoteDebugSampleType, ResetScore, ResourcePackPop,
        ResourcePackPush, ResourcePackResponseAction, ScoreboardDisplaySlot, SelectAdvancementsTab,
        ServerData, ServerLinkEntry, ServerLinkKnownType, ServerLinkType, ServerLinks, SetCamera,
        SetDefaultSpawnPosition, SetDisplayObjective, SetHeldSlot, SetObjective,
        SetObjectiveMethod, SetObjectiveParameters, SetPassengers, SetPlayerTeam, SetScore,
        SetSimulationDistance, ShowDialog, SignedMessageBody, SlotDisplaySummary, SoundEntityEvent,
        SoundEvent, SoundEventHolder, SoundSource, StatUpdate, StonecutterSelectableRecipeSummary,
        StopSound, StoreCookie, TabList, TagQuery, TeamCollisionRule, TeamVisibility,
        TestInstanceBlockStatus, TickingState, TickingStep, TrackedWaypoint, TrackedWaypointPacket,
        Transfer, UpdateAdvancements, UpdateRecipes, Vec3d as ProtocolVec3d,
        Vec3i as ProtocolVec3i, WaypointData, WaypointIcon, WaypointIdentifier, WaypointOperation,
        WaypointVec3i,
    };
    use bbb_protocol::{
        codec::{Decoder, Encoder},
        ids,
    };
    use bbb_world::{BlockPos, ChunkPos, LocalPlayerPoseState};
    use bytes::BytesMut;
    use std::{collections::BTreeMap, time::Duration};
    use tokio::net::TcpListener;
    use tokio::time::timeout;
    use uuid::Uuid;

    #[tokio::test]
    async fn probe_applies_debug_game_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::DebugBlockValue(DebugBlockValue {
                pos: ProtocolBlockPos { x: 1, y: 64, z: -2 },
                raw_update_payload: vec![5, 1, 0xaa],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::DebugChunkValue(DebugChunkValue {
                pos: ProtocolChunkPos { x: 3, z: -4 },
                raw_update_payload: vec![7, 0],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::DebugEntityValue(DebugEntityValue {
                entity_id: 123,
                raw_update_payload: vec![9, 1, 0xbb],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::DebugEvent(DebugEvent {
                raw_event_payload: vec![4, 0xcc],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::DebugSample(DebugSample {
                sample: vec![100, -50],
                sample_type: RemoteDebugSampleType::TickTime,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::GameRuleValues(GameRuleValues {
                values: vec![
                    GameRuleValue {
                        rule: "minecraft:do_daylight_cycle".to_string(),
                        value: "false".to_string(),
                    },
                    GameRuleValue {
                        rule: "minecraft:random_tick_speed".to_string(),
                        value: "3".to_string(),
                    },
                ],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::GameTestHighlightPos(
                GameTestHighlightPos {
                    absolute_pos: ProtocolBlockPos {
                        x: -10,
                        y: 70,
                        z: 22,
                    },
                    relative_pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
                },
            ))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::TestInstanceBlockStatus(
                TestInstanceBlockStatus {
                    status: "Ready".to_string(),
                    size: Some(ProtocolVec3i { x: 3, y: 4, z: 5 }),
                },
            ))
            .await
            .unwrap();

        let report = probe.finish(8, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.debug_block_value_packets, 1);
        assert_eq!(report.world_counters.debug_chunk_value_packets, 1);
        assert_eq!(report.world_counters.debug_entity_value_packets, 1);
        assert_eq!(report.world_counters.debug_event_packets, 1);
        assert_eq!(report.world_counters.debug_sample_packets, 1);
        assert_eq!(report.world_counters.game_rule_value_packets, 1);
        assert_eq!(report.world_counters.game_test_highlight_pos_packets, 1);
        assert_eq!(report.world_counters.test_instance_block_status_packets, 1);
        assert_eq!(
            report.world.last_debug_block_value(),
            Some(&bbb_world::DebugBlockValueState {
                pos: BlockPos { x: 1, y: 64, z: -2 },
                raw_update_payload_len: 3,
            })
        );
        assert_eq!(
            report
                .world
                .last_game_rule_values()
                .map(|state| state.len()),
            Some(2)
        );
        assert_eq!(
            report.world.last_test_instance_block_status(),
            Some(&bbb_world::TestInstanceBlockStatusState {
                status: "Ready".to_string(),
                size: Some(bbb_world::DebugVec3iState { x: 3, y: 4, z: 5 }),
            })
        );
    }

    #[tokio::test]
    async fn probe_applies_award_stats_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::AwardStats(AwardStats {
                stats: vec![
                    StatUpdate {
                        stat_type_id: 8,
                        value_id: 10,
                        amount: 3,
                    },
                    StatUpdate {
                        stat_type_id: 0,
                        value_id: 4,
                        amount: 11,
                    },
                ],
            }))
            .await
            .unwrap();

        let report = probe.finish(8, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world.stat_value(8, 10), Some(3));
        assert_eq!(report.world.stat_value(0, 4), Some(11));
        assert_eq!(report.world_counters.award_stats_packets, 1);
        assert_eq!(report.world_counters.award_stats_entries_received, 2);
        assert_eq!(report.world_counters.last_award_stats_entry_count, 2);
        assert_eq!(report.world_counters.stats_tracked, 2);
    }

    #[tokio::test]
    async fn probe_applies_tag_query_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::TagQuery(TagQuery {
                transaction_id: 12,
                tag_present: true,
                raw_nbt: vec![10, 0],
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });

        assert_eq!(
            report.world.last_tag_query(),
            Some(&bbb_world::TagQueryResponseState {
                transaction_id: 12,
                tag_present: true,
                raw_nbt: vec![10, 0],
            })
        );
        assert_eq!(report.world.last_tag_query().unwrap().raw_nbt_len(), 2);
        assert_eq!(report.world_counters.tag_query_packets, 1);
    }

    #[tokio::test]
    async fn probe_applies_command_suggestion_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::CustomChatCompletions(
                CustomChatCompletions {
                    action: CustomChatCompletionsAction::Set,
                    entries: vec!["/spawn".to_string(), "/warp".to_string()],
                },
            ))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::CustomChatCompletions(
                CustomChatCompletions {
                    action: CustomChatCompletionsAction::Remove,
                    entries: vec!["/spawn".to_string()],
                },
            ))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::CommandSuggestions(CommandSuggestions {
                id: 77,
                start: 1,
                length: 4,
                suggestions: vec![
                    CommandSuggestion {
                        text: "give".to_string(),
                        tooltip: Some("Run give".to_string()),
                    },
                    CommandSuggestion {
                        text: "gamemode".to_string(),
                        tooltip: None,
                    },
                ],
            }))
            .await
            .unwrap();

        let report = probe.finish(3, ChunkPos { x: 0, z: 0 });
        let completions = report.world.custom_chat_completions();
        assert_eq!(completions.len(), 1);
        assert!(completions.contains("/warp"));
        assert_eq!(
            report.world.last_custom_chat_completion_update(),
            Some(&bbb_world::CustomChatCompletionUpdateState {
                action: "remove".to_string(),
                entries: 1,
            })
        );

        let suggestions = report
            .world
            .command_suggestions_by_id(77)
            .expect("suggestions tracked by id");
        assert_eq!(suggestions.start, 1);
        assert_eq!(suggestions.length, 4);
        assert_eq!(suggestions.suggestions.len(), 2);
        assert_eq!(suggestions.suggestions[0].text, "give");
        assert_eq!(
            suggestions.suggestions[0].tooltip.as_deref(),
            Some("Run give")
        );
        assert_eq!(suggestions.suggestions[1].text, "gamemode");
        assert_eq!(suggestions.suggestions[1].tooltip, None);
        assert_eq!(report.world.last_command_suggestions(), Some(suggestions));
        assert_eq!(report.world_counters.custom_chat_completion_packets, 2);
        assert_eq!(report.world_counters.custom_chat_completions_tracked, 1);
        assert_eq!(report.world_counters.command_suggestion_packets, 1);
        assert_eq!(report.world_counters.command_suggestion_entries_tracked, 2);
    }

    #[tokio::test]
    async fn probe_applies_map_item_data_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::MapItemData(MapItemData {
                map_id: 12,
                scale: 1,
                locked: false,
                decorations: Some(vec![MapDecoration {
                    type_id: 4,
                    x: -8,
                    y: 9,
                    rot: 3,
                    name: Some("Camp".to_string()),
                }]),
                color_patch: Some(MapColorPatch {
                    start_x: 5,
                    start_y: 6,
                    width: 2,
                    height: 1,
                    colors: vec![11, 12],
                }),
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });
        let map = report.world.map_item(12).expect("map state is tracked");
        assert_eq!(map.decorations.len(), 1);
        assert_eq!(map.decorations[0].name.as_deref(), Some("Camp"));
        assert_eq!(map.colors[5 + 6 * 128], 11);
        assert_eq!(map.colors[6 + 6 * 128], 12);
        assert_eq!(
            report.world.last_map_color_patch(),
            Some(&bbb_world::LastMapColorPatchState {
                map_id: 12,
                start_x: 5,
                start_y: 6,
                width: 2,
                height: 1,
            })
        );
        assert_eq!(report.world_counters.map_item_data_packets, 1);
        assert_eq!(report.world_counters.maps_tracked, 1);
        assert_eq!(report.world_counters.map_decorations_tracked, 1);
        assert_eq!(report.world_counters.map_color_patches_applied, 1);
    }

    #[tokio::test]
    async fn probe_applies_audio_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(123)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::Sound(SoundEvent {
                sound: SoundEventHolder::Reference { registry_id: 41 },
                source: SoundSource::Blocks,
                position: ProtocolVec3d {
                    x: 2.5,
                    y: -1.0,
                    z: 0.0,
                },
                volume: 0.75,
                pitch: 1.25,
                seed: 123456789,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SoundEntity(SoundEntityEvent {
                sound: SoundEventHolder::Direct {
                    location: "minecraft:entity.cat.ambient".to_string(),
                    fixed_range: Some(32.0),
                },
                source: SoundSource::Neutral,
                entity_id: 123,
                volume: 1.0,
                pitch: 0.5,
                seed: -9,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::StopSound(StopSound {
                source: Some(SoundSource::Music),
                name: Some("minecraft:music.menu".to_string()),
            }))
            .await
            .unwrap();

        let report = probe.finish(4, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.sound_packets, 1);
        assert_eq!(
            report.world.last_sound(),
            Some(&bbb_world::SoundEventState {
                sound: bbb_world::SoundHolderState {
                    kind: "reference".to_string(),
                    registry_id: Some(41),
                    location: None,
                    fixed_range: None,
                },
                source: "block".to_string(),
                position: ProtocolVec3d {
                    x: 2.5,
                    y: -1.0,
                    z: 0.0,
                },
                volume: 0.75,
                pitch: 1.25,
                seed: 123456789,
            })
        );
        assert_eq!(report.world_counters.sound_entity_packets, 1);
        assert_eq!(report.world_counters.sound_entity_events_applied, 1);
        assert_eq!(report.world_counters.sound_entity_events_ignored, 0);
        assert_eq!(
            report.world.last_sound_entity(),
            Some(&bbb_world::SoundEntityEventState {
                sound: bbb_world::SoundHolderState {
                    kind: "direct".to_string(),
                    registry_id: None,
                    location: Some("minecraft:entity.cat.ambient".to_string()),
                    fixed_range: Some(32.0),
                },
                source: "neutral".to_string(),
                entity_id: 123,
                volume: 1.0,
                pitch: 0.5,
                seed: -9,
            })
        );
        assert_eq!(report.world_counters.stop_sound_packets, 1);
        assert_eq!(
            report.world.last_stop_sound(),
            Some(&bbb_world::StopSoundEventState {
                source: Some("music".to_string()),
                name: Some("minecraft:music.menu".to_string()),
            })
        );
    }

    #[tokio::test]
    async fn probe_applies_visual_effect_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        probe.world.set_local_player_pose(LocalPlayerPoseState {
            delta_movement: ProtocolVec3d {
                x: 0.5,
                y: -0.25,
                z: 1.0,
            },
            ..LocalPlayerPoseState::default()
        });

        probe
            .handle_play_packet(PlayClientbound::Explosion(Explosion {
                center: ProtocolVec3d {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
                radius: 4.5,
                block_count: 7,
                player_knockback: Some(ProtocolVec3d {
                    x: 0.25,
                    y: -0.5,
                    z: 1.5,
                }),
                raw_effect_payload: vec![0x2d, 0x2a, 0x01, 0x00],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::LevelParticles(LevelParticles {
                override_limiter: true,
                always_show: false,
                position: ProtocolVec3d {
                    x: 10.0,
                    y: 64.5,
                    z: -3.25,
                },
                offset: ProtocolVec3d {
                    x: f64::from(0.1_f32),
                    y: f64::from(0.2_f32),
                    z: f64::from(0.3_f32),
                },
                max_speed: 1.5,
                count: 16,
                particle: ParticlePayload {
                    particle_type_id: 45,
                    raw_options: vec![0xaa, 0xbb],
                },
            }))
            .await
            .unwrap();

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.explosion_packets, 1);
        assert_eq!(
            report.world.last_explosion(),
            Some(&bbb_world::ExplosionEventState {
                center: ProtocolVec3d {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
                radius: 4.5,
                block_count: 7,
                player_knockback: Some(ProtocolVec3d {
                    x: 0.25,
                    y: -0.5,
                    z: 1.5,
                }),
                raw_effect_payload_len: 4,
            })
        );
        assert_eq!(
            report.world.local_player_pose().unwrap().delta_movement,
            ProtocolVec3d {
                x: 0.75,
                y: -0.75,
                z: 2.5,
            }
        );
        assert_eq!(report.world_counters.level_particles_packets, 1);
        assert_eq!(
            report.world.last_level_particles(),
            Some(&bbb_world::LevelParticlesEventState {
                override_limiter: true,
                always_show: false,
                position: ProtocolVec3d {
                    x: 10.0,
                    y: 64.5,
                    z: -3.25,
                },
                offset: ProtocolVec3d {
                    x: f64::from(0.1_f32),
                    y: f64::from(0.2_f32),
                    z: f64::from(0.3_f32),
                },
                max_speed: 1.5,
                count: 16,
                particle_type_id: 45,
                raw_options_len: 2,
            })
        );
    }

    #[tokio::test]
    async fn probe_chunk_batch_feedback_uses_vanilla_calculator() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::ChunkBatchFinished { batch_size: 0 })
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("chunk batch received packet should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CHUNK_BATCH_RECEIVED);
        let desired = Decoder::new(&payload).read_f32().unwrap();
        assert_eq!(desired, 3.5);
    }

    #[tokio::test]
    async fn probe_player_chat_sends_chat_acknowledgement_after_threshold() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        for index in 0..65 {
            probe
                .handle_play_packet(PlayClientbound::PlayerChat(
                    protocol_player_chat_with_signature(
                        index,
                        MessageSignature {
                            bytes: vec![index as u8; 256],
                        },
                    ),
                ))
                .await
                .unwrap();
        }

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("chat acknowledgement packet should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CHAT_ACK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 65);
        assert!(decoder.is_empty());
        assert_eq!(probe.world.counters().player_chat_packets, 65);
        assert_eq!(
            probe.world.counters().player_chat_acknowledgement_packets,
            1
        );
    }

    #[tokio::test]
    async fn probe_start_configuration_acknowledges_and_resets_configuration_dedup_state() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        probe.state = ConnectionState::Play;
        probe.play_tick = Some(crate::connection::play_tick_interval());
        probe.seen_code_of_conduct = true;
        probe.world.apply_add_entity(protocol_add_entity(55));
        probe.world.set_local_using_item(true);

        probe
            .handle_play_packet(PlayClientbound::StartConfiguration)
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("configuration acknowledgement should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED);
        assert!(payload.is_empty());
        assert_eq!(probe.state, ConnectionState::Configuration);
        assert!(probe.play_tick.is_none());
        assert!(!probe.seen_code_of_conduct);
        assert_eq!(probe.world.entity_count(), 0);
        assert_eq!(
            probe.world.local_player(),
            &bbb_world::LocalPlayerState::default()
        );

        probe
            .handle_configuration_packet(packets::ConfigurationClientbound::CodeOfConduct {
                text: "Fresh configuration rules.".to_string(),
            })
            .await
            .unwrap();

        assert!(probe.seen_code_of_conduct);
        assert_eq!(
            probe.world.last_code_of_conduct().unwrap().text,
            "Fresh configuration rules."
        );
        assert_eq!(probe.world.counters().code_of_conduct_packets, 1);
    }

    #[tokio::test]
    async fn probe_start_configuration_flushes_pending_chat_acknowledgement_first() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        probe.state = ConnectionState::Play;

        probe
            .handle_play_packet(PlayClientbound::PlayerChat(
                protocol_player_chat_with_signature(
                    0,
                    MessageSignature {
                        bytes: vec![9; 256],
                    },
                ),
            ))
            .await
            .unwrap();

        probe
            .handle_play_packet(PlayClientbound::StartConfiguration)
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("chat acknowledgement should be sent first")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CHAT_ACK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("configuration acknowledgement should be sent second")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED);
        assert!(payload.is_empty());
        assert_eq!(probe.state, ConnectionState::Configuration);
        assert_eq!(
            probe
                .world
                .counters()
                .player_chat_acknowledgement_pending_offset,
            0
        );
    }

    #[tokio::test]
    async fn probe_records_nonfatal_chunk_update_errors() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::BlockEntityData(BlockEntityData {
                pos: ProtocolBlockPos { x: 0, y: 64, z: 0 },
                block_entity_type_id: 9,
                raw_nbt: vec![1],
            }))
            .await
            .unwrap();

        let counters = probe.world.counters();
        assert_eq!(counters.block_entity_updates_received, 1);
        assert_eq!(counters.block_entity_updates_applied, 0);
        let diagnostics = probe.world.apply_diagnostics();
        assert_eq!(diagnostics.apply_errors.len(), 1);
        assert_eq!(diagnostics.apply_errors[0].source, "block_entity_data");
        assert!(
            diagnostics.apply_errors[0].message.contains("nbt byte"),
            "{:?}",
            diagnostics.apply_errors
        );
        assert_eq!(counters.world_apply_errors, 1);

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });
        assert_eq!(report.world_apply_errors.len(), 1);
        assert!(report.world_apply_errors[0].contains("nbt byte"));
        assert_eq!(report.world_counters.world_apply_errors, 1);
        assert_eq!(report.world.apply_diagnostics().apply_errors.len(), 1);
    }

    #[tokio::test]
    async fn probe_records_bad_chunk_and_continues_to_good_chunk() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let mut bad_chunk = synthetic_probe_level_chunk_packet();
        bad_chunk.chunk_data.section_data = vec![0xff];

        let first_chunk = probe
            .handle_play_packet(PlayClientbound::LevelChunkWithLight(bad_chunk))
            .await
            .unwrap();

        assert_eq!(first_chunk, None);
        assert_eq!(probe.world.counters().world_apply_errors, 1);
        assert_eq!(
            probe.world.apply_diagnostics().apply_errors[0].source,
            "level_chunk_with_light"
        );

        let first_chunk = probe
            .handle_play_packet(PlayClientbound::LevelChunkWithLight(
                synthetic_probe_level_chunk_packet(),
            ))
            .await
            .unwrap();

        assert_eq!(first_chunk, Some(ChunkPos { x: 1, z: -2 }));
        assert_eq!(probe.world.counters().chunks_received, 1);
        assert_eq!(probe.world.counters().world_apply_errors, 1);
    }

    #[tokio::test]
    async fn probe_play_cookie_events_update_world_and_respond() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::StoreCookie(StoreCookie {
                key: "bbb:session".to_string(),
                payload: vec![4, 5, 6],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::CookieRequest(CookieRequest {
                key: "bbb:session".to_string(),
            }))
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("cookie response should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_COOKIE_RESPONSE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_string(32767).unwrap(), "bbb:session");
        assert!(decoder.read_bool().unwrap());
        let len = decoder.read_len().unwrap();
        assert_eq!(
            decoder.read_exact(len, "cookie response").unwrap(),
            &[4, 5, 6]
        );
        assert!(decoder.is_empty());

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });
        assert_eq!(report.world.last_cookie_key(), Some("bbb:session"));
        assert_eq!(report.world.stored_cookie_count(), 1);
        assert_eq!(report.world_counters.store_cookie_packets, 1);
        assert_eq!(report.world_counters.stored_cookie_bytes, 3);
        assert_eq!(report.world_counters.cookie_request_packets, 1);
        assert_eq!(report.world_counters.cookie_response_hits, 1);
        assert_eq!(report.world_counters.cookie_response_misses, 0);
    }

    #[tokio::test]
    async fn probe_play_server_presentation_packets_update_world_and_decline_resource_pack() {
        let pack_id = Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678);
        let missing_pack_id = Uuid::from_u128(0xaaaaaaaa_bbbb_cccc_dddd_eeeeeeeeeeee);
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::ServerData(ServerData {
                motd: "Native test server".to_string(),
                icon_bytes: Some(vec![1, 2, 3, 4]),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ResourcePackPush(ResourcePackPush {
                id: pack_id,
                url: "https://example.invalid/pack.zip".to_string(),
                hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
                required: true,
                prompt: Some("Install pack?".to_string()),
            }))
            .await
            .unwrap();

        let pack = probe
            .world
            .resource_pack(pack_id)
            .expect("resource pack should be tracked after push");
        assert_eq!(pack.url, "https://example.invalid/pack.zip");
        assert_eq!(pack.hash, "0123456789abcdef0123456789abcdef01234567");
        assert!(pack.required);
        assert_eq!(pack.prompt.as_deref(), Some("Install pack?"));
        assert_eq!(
            pack.last_response.as_ref().map(|response| response.action),
            Some(ResourcePackResponseAction::Declined)
        );

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("resource pack response should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_RESOURCE_PACK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_uuid().unwrap(), pack_id);
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            ResourcePackResponseAction::Declined.ordinal()
        );
        assert!(decoder.is_empty());

        probe
            .handle_play_packet(PlayClientbound::ResourcePackPop(ResourcePackPop {
                id: Some(pack_id),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ResourcePackPop(ResourcePackPop {
                id: Some(missing_pack_id),
            }))
            .await
            .unwrap();

        let report = probe.finish(4, ChunkPos { x: 0, z: 0 });
        let server_data = report
            .world
            .server_data()
            .expect("server data should be tracked");
        assert_eq!(server_data.motd, "Native test server");
        assert_eq!(server_data.icon_byte_len(), Some(4));
        assert!(report.world.resource_packs().is_empty());
        assert_eq!(report.world_counters.server_data_packets, 1);
        assert_eq!(report.world_counters.resource_pack_push_packets, 1);
        assert_eq!(report.world_counters.resource_pack_response_packets, 1);
        assert_eq!(
            report.world_counters.resource_pack_response_updates_applied,
            1
        );
        assert_eq!(
            report.world_counters.resource_pack_response_updates_ignored,
            0
        );
        assert_eq!(report.world_counters.resource_pack_required_declines, 1);
        assert_eq!(report.world_counters.resource_pack_pop_packets, 2);
        assert_eq!(report.world_counters.resource_pack_pop_updates_applied, 1);
        assert_eq!(report.world_counters.resource_pack_pop_updates_ignored, 1);
        assert_eq!(report.world_counters.resource_packs_tracked, 0);
    }

    #[tokio::test]
    async fn probe_play_common_server_presentation_packets_update_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let details = BTreeMap::from([
            ("Region".to_string(), "local".to_string()),
            ("Server".to_string(), "probe-play".to_string()),
        ]);

        probe
            .handle_play_packet(PlayClientbound::CustomReportDetails(CustomReportDetails {
                details: details.clone(),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ServerLinks(ServerLinks {
                links: vec![
                    ServerLinkEntry {
                        link_type: ServerLinkType::Known(ServerLinkKnownType::Support),
                        url: "https://example.invalid/support".to_string(),
                    },
                    ServerLinkEntry {
                        link_type: ServerLinkType::Custom {
                            label: "Rules".to_string(),
                        },
                        url: "http://example.invalid/rules".to_string(),
                    },
                    ServerLinkEntry {
                        link_type: ServerLinkType::Known(ServerLinkKnownType::Website),
                        url: "ftp://example.invalid/file".to_string(),
                    },
                ],
            }))
            .await
            .unwrap();

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });
        assert_eq!(report.world.custom_report_details(), &details);
        assert_eq!(report.world.server_links().len(), 2);
        assert_eq!(
            report.world.server_links()[0].label,
            "known_server_link.support"
        );
        assert_eq!(
            report.world.server_links()[0].known_type.as_deref(),
            Some("support")
        );
        assert_eq!(
            report.world.server_links()[0].url,
            "https://example.invalid/support"
        );
        assert_eq!(report.world.server_links()[1].label, "Rules");
        assert_eq!(report.world.server_links()[1].known_type, None);
        assert_eq!(
            report.world.server_links()[1].url,
            "http://example.invalid/rules"
        );
        assert_eq!(report.world_counters.custom_report_detail_packets, 1);
        assert_eq!(report.world_counters.custom_report_details_tracked, 2);
        assert_eq!(report.world_counters.server_link_packets, 1);
        assert_eq!(report.world_counters.server_links_tracked, 2);
        assert_eq!(report.world_counters.server_link_invalid_entries, 1);
    }

    #[tokio::test]
    async fn probe_play_custom_payload_updates_world_brand_and_unknown_payload() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::CustomPayload(CustomPayload {
                id: "minecraft:brand".to_string(),
                payload: CustomPayloadBody::Brand {
                    brand: "vanilla".to_string(),
                },
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::CustomPayload(CustomPayload {
                id: "example:diagnostic".to_string(),
                payload: CustomPayloadBody::Unknown {
                    raw_payload: vec![1, 2, 3, 4],
                },
            }))
            .await
            .unwrap();

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world.server_brand(), Some("vanilla"));
        assert_eq!(
            report.world.last_custom_payload(),
            Some(&bbb_world::CustomPayloadState {
                id: "example:diagnostic".to_string(),
                kind: "unknown".to_string(),
                brand: None,
                raw_payload_len: 4,
            })
        );
        assert_eq!(report.world_counters.custom_payload_packets, 2);
        assert_eq!(report.world_counters.custom_payload_brand_packets, 1);
        assert_eq!(report.world_counters.custom_payload_unknown_packets, 1);
    }

    #[tokio::test]
    async fn probe_play_resource_pack_invalid_url_updates_world_and_response() {
        let pack_id = Uuid::from_u128(0x77777777_3333_4444_5555_666666666666);
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::ResourcePackPush(ResourcePackPush {
                id: pack_id,
                url: "not a valid resource pack url".to_string(),
                hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
                required: true,
                prompt: Some("Install pack?".to_string()),
            }))
            .await
            .unwrap();

        let pack = probe
            .world
            .resource_pack(pack_id)
            .expect("resource pack should be tracked after push");
        assert_eq!(
            pack.last_response.as_ref().map(|response| response.action),
            Some(ResourcePackResponseAction::InvalidUrl)
        );

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("resource pack response should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_RESOURCE_PACK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_uuid().unwrap(), pack_id);
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            ResourcePackResponseAction::InvalidUrl.ordinal()
        );
        assert!(decoder.is_empty());
    }

    #[tokio::test]
    async fn probe_play_transfer_updates_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::Transfer(Transfer {
                host: "next.example.invalid".to_string(),
                port: 25566,
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.transfer_packets, 1);
        assert_eq!(
            report.world.last_transfer(),
            Some(&bbb_world::TransferTargetState {
                host: "next.example.invalid".to_string(),
                port: 25566,
            })
        );
    }

    #[tokio::test]
    async fn probe_move_vehicle_sends_vehicle_ack() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::Login(protocol_play_login(99)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(10)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetPassengers(SetPassengers {
                vehicle_id: 10,
                passenger_ids: vec![99],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::MoveVehicle(MoveVehicle {
                position: ProtocolVec3d {
                    x: 5.0,
                    y: 66.0,
                    z: -7.0,
                },
                y_rot: 45.0,
                x_rot: -5.0,
            }))
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("move vehicle ack should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_VEHICLE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_f64().unwrap(), 5.0);
        assert_eq!(decoder.read_f64().unwrap(), 66.0);
        assert_eq!(decoder.read_f64().unwrap(), -7.0);
        assert_eq!(decoder.read_f32().unwrap(), 45.0);
        assert_eq!(decoder.read_f32().unwrap(), -5.0);
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.is_empty());

        let report = probe.finish(4, ChunkPos { x: 0, z: 0 });
        let vehicle = report.world.probe_entity(10).unwrap();
        assert_eq!(vehicle.position.x, 5.0);
        assert_eq!(vehicle.position.y, 66.0);
        assert_eq!(vehicle.position.z, -7.0);
        assert_eq!(report.world_counters.vehicle_moves_received, 1);
        assert_eq!(report.world_counters.vehicle_moves_applied, 1);
        assert_eq!(report.world_counters.vehicle_moves_acked, 1);
        assert_eq!(report.world_counters.vehicle_moves_snapped, 1);
    }

    #[tokio::test]
    async fn probe_projectile_power_updates_world_entity_state_and_counters() {
        const VANILLA_ENTITY_TYPE_FIREBALL_ID: i32 = 52;

        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity_with_type(
                123,
                VANILLA_ENTITY_TYPE_FIREBALL_ID,
            )))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity_with_type(
                456, 7,
            )))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ProjectilePower(ProjectilePower {
                entity_id: 123,
                acceleration_power: 0.75,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ProjectilePower(ProjectilePower {
                entity_id: 456,
                acceleration_power: 0.25,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ProjectilePower(ProjectilePower {
                entity_id: 404,
                acceleration_power: 0.5,
            }))
            .await
            .unwrap();

        let report = probe.finish(5, ChunkPos { x: 0, z: 0 });

        assert_eq!(
            report.world.hurting_projectile(123),
            Some(bbb_world::HurtingProjectileState {
                acceleration_power: 0.75,
            })
        );
        assert_eq!(report.world.hurting_projectile(456), None);
        assert_eq!(report.world_counters.projectile_power_packets, 3);
        assert_eq!(report.world_counters.projectile_power_updates_applied, 1);
        assert_eq!(report.world_counters.projectile_power_updates_ignored, 2);
        assert_eq!(
            report.world.last_projectile_power_update(),
            Some(&bbb_world::ProjectilePowerUpdateState {
                entity_id: 404,
                acceleration_power: 0.5,
                applied: false,
            })
        );
    }

    #[tokio::test]
    async fn probe_applies_waypoint_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let waypoint_id = Uuid::from_u128(0x00112233445566778899aabbccddeeff);

        probe
            .handle_play_packet(PlayClientbound::Waypoint(TrackedWaypointPacket {
                operation: WaypointOperation::Track,
                waypoint: TrackedWaypoint {
                    identifier: WaypointIdentifier::Uuid(waypoint_id),
                    icon: WaypointIcon {
                        style: "minecraft:default".to_string(),
                        color_rgb: Some(0x112233),
                    },
                    data: WaypointData::Position(WaypointVec3i {
                        x: 12,
                        y: 70,
                        z: -8,
                    }),
                },
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });
        let key = format!("uuid:{waypoint_id}");
        let waypoint = report
            .world
            .tracked_waypoints()
            .get(&key)
            .expect("waypoint should be tracked");

        assert_eq!(report.world_counters.waypoint_packets, 1);
        assert_eq!(report.world_counters.waypoints_tracked, 1);
        assert_eq!(waypoint.identifier_kind, "uuid");
        assert_eq!(waypoint.identifier, waypoint_id.to_string());
        assert_eq!(waypoint.icon_style, "minecraft:default");
        assert_eq!(waypoint.icon_color_rgb, Some(0x112233));
        assert_eq!(waypoint.data.kind, "position");
        assert_eq!(
            waypoint.data.position,
            Some(bbb_world::WaypointVec3iState {
                x: 12,
                y: 70,
                z: -8,
            })
        );
        assert_eq!(
            report
                .world
                .last_waypoint_event()
                .map(|event| event.applied),
            Some(true)
        );
    }

    #[tokio::test]
    async fn probe_applies_client_ui_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::LowDiskSpaceWarning)
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ShowDialog(ShowDialog {
                dialog: DialogHolder::Direct {
                    raw_dialog_payload: vec![0xaa, 0xbb, 0xcc],
                },
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ClearDialog)
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::MountScreenOpen(MountScreenOpen {
                container_id: 11,
                inventory_columns: 5,
                entity_id: 42,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::OpenBook(OpenBook {
                hand: InteractionHand::OffHand,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::OpenSignEditor(OpenSignEditor {
                pos: ProtocolBlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                is_front_text: false,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::PlaceGhostRecipe(PlaceGhostRecipe {
                container_id: 9,
                recipe_display_type: RecipeDisplayType::Stonecutter,
                recipe_display_body: vec![1, 2, 3],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::PongResponse(PongResponse {
                time: 123456789,
            }))
            .await
            .unwrap();

        let report = probe.finish(8, ChunkPos { x: 0, z: 0 });
        let ui = report.world.client_ui();

        assert_eq!(ui.low_disk_space_warning_count, 1);
        assert_eq!(ui.current_dialog, None);
        assert_eq!(
            ui.last_mount_screen,
            Some(bbb_world::MountScreenState {
                container_id: 11,
                inventory_columns: 5,
                entity_id: 42,
            })
        );
        assert_eq!(
            ui.last_open_book,
            Some(bbb_world::OpenBookState {
                hand: "off_hand".to_string()
            })
        );
        assert_eq!(
            ui.last_open_sign_editor,
            Some(bbb_world::OpenSignEditorState {
                pos: BlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                is_front_text: false,
            })
        );
        assert_eq!(
            ui.last_ghost_recipe,
            Some(bbb_world::GhostRecipeState {
                container_id: 9,
                recipe_display_type_id: 3,
                recipe_display_type: "stonecutter".to_string(),
                recipe_display_body_len: 3,
            })
        );
        assert_eq!(
            ui.last_pong_response,
            Some(bbb_world::PongResponseState { time: 123456789 })
        );
        assert_eq!(report.world_counters.low_disk_space_warnings, 1);
        assert_eq!(report.world_counters.show_dialog_packets, 1);
        assert_eq!(report.world_counters.clear_dialog_packets, 1);
        assert_eq!(report.world_counters.mount_screen_open_packets, 1);
        assert_eq!(report.world_counters.open_book_packets, 1);
        assert_eq!(report.world_counters.open_sign_editor_packets, 1);
        assert_eq!(report.world_counters.ghost_recipe_packets, 1);
        assert_eq!(report.world_counters.pong_response_packets, 1);
    }

    #[tokio::test]
    async fn probe_applies_recipe_book_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let settings = RecipeBookSettings {
            crafting: RecipeBookTypeSettings {
                open: true,
                filtering: false,
            },
            furnace: RecipeBookTypeSettings {
                open: false,
                filtering: true,
            },
            blast_furnace: RecipeBookTypeSettings {
                open: true,
                filtering: true,
            },
            smoker: RecipeBookTypeSettings {
                open: false,
                filtering: false,
            },
        };

        probe
            .handle_play_packet(PlayClientbound::RecipeBookAdd(RecipeBookAdd {
                replace: true,
                entries: vec![
                    probe_recipe_entry(7, true, true),
                    probe_recipe_entry(8, false, false),
                ],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::RecipeBookAdd(RecipeBookAdd {
                replace: false,
                entries: vec![probe_recipe_entry(9, true, false)],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::RecipeBookRemove(RecipeBookRemove {
                recipe_ids: vec![RecipeDisplayId { index: 7 }],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::RecipeBookSettings(settings))
            .await
            .unwrap();

        let report = probe.finish(5, ChunkPos { x: 0, z: 0 });
        let recipe_book = report.world.recipe_book();

        assert!(!recipe_book.known.contains_key(&7));
        assert_eq!(recipe_book.known.get(&8).unwrap().category_id, 18);
        assert_eq!(recipe_book.known.get(&9).unwrap().category_id, 19);
        assert!(recipe_book.highlights.is_empty());
        assert_eq!(recipe_book.notification_ids, vec![9]);
        assert_eq!(recipe_book.settings, settings);

        assert_eq!(report.world_counters.recipe_book_add_packets, 2);
        assert_eq!(report.world_counters.recipe_book_replace_packets, 1);
        assert_eq!(report.world_counters.recipe_book_remove_packets, 1);
        assert_eq!(report.world_counters.recipe_book_settings_packets, 1);
        assert_eq!(report.world_counters.recipe_book_entries_received, 3);
        assert_eq!(
            report.world_counters.recipe_book_removed_entries_received,
            1
        );
        assert_eq!(report.world_counters.recipe_book_entries_tracked, 2);
        assert_eq!(report.world_counters.recipe_book_highlights_tracked, 0);
        assert_eq!(report.world_counters.recipe_book_notifications_received, 2);
    }

    #[tokio::test]
    async fn probe_applies_advancement_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::UpdateAdvancements(UpdateAdvancements {
                reset: true,
                added: vec![AdvancementSummary {
                    id: "minecraft:story/root".to_string(),
                    parent: None,
                    display: None,
                    requirements: vec![vec!["mine_stone".to_string(), "get_log".to_string()]],
                    sends_telemetry_event: true,
                }],
                removed: Vec::new(),
                progress: vec![AdvancementProgressSummary {
                    id: "minecraft:story/root".to_string(),
                    criteria: vec![AdvancementCriterionProgressSummary {
                        name: "mine_stone".to_string(),
                        obtained_epoch_millis: Some(1_700_000_000_000),
                    }],
                }],
                show_advancements: true,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SelectAdvancementsTab(
                SelectAdvancementsTab {
                    tab: Some("minecraft:story/root".to_string()),
                },
            ))
            .await
            .unwrap();

        let report = probe.finish(3, ChunkPos { x: 0, z: 0 });

        assert_eq!(
            report.world.selected_advancements_tab(),
            Some("minecraft:story/root")
        );
        assert!(report
            .world
            .advancements()
            .advancements
            .contains_key("minecraft:story/root"));
        let progress = report
            .world
            .advancements()
            .progress
            .get("minecraft:story/root")
            .unwrap();
        assert_eq!(progress.criteria.len(), 2);

        assert_eq!(report.world_counters.update_advancements_packets, 1);
        assert_eq!(report.world_counters.update_advancements_reset_packets, 1);
        assert_eq!(report.world_counters.update_advancements_show_packets, 1);
        assert_eq!(report.world_counters.advancements_added_received, 1);
        assert_eq!(report.world_counters.advancements_removed_received, 0);
        assert_eq!(report.world_counters.advancements_adds_ignored, 0);
        assert_eq!(report.world_counters.advancement_progress_received, 1);
        assert_eq!(
            report.world_counters.advancement_progress_updates_ignored,
            0
        );
        assert_eq!(report.world_counters.advancements_tracked, 1);
        assert_eq!(report.world_counters.advancement_roots_tracked, 1);
        assert_eq!(report.world_counters.advancement_progress_tracked, 1);
        assert_eq!(
            report.world_counters.advancement_progress_criteria_tracked,
            2
        );
        assert_eq!(report.world_counters.select_advancements_tab_packets, 1);
    }

    #[tokio::test]
    async fn probe_applies_update_recipes_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::UpdateRecipes(UpdateRecipes {
                property_sets: vec![
                    RecipePropertySetSummary {
                        key: "minecraft:furnace_input".to_string(),
                        item_ids: vec![42, 43],
                    },
                    RecipePropertySetSummary {
                        key: "minecraft:smithing_base".to_string(),
                        item_ids: vec![99],
                    },
                ],
                stonecutter_recipes: vec![StonecutterSelectableRecipeSummary {
                    input: IngredientSummary {
                        tag: None,
                        item_ids: vec![11, 12],
                    },
                    option_display: SlotDisplaySummary {
                        display_type_id: 4,
                        raw_payload: vec![4, 77],
                    },
                }],
            }))
            .await
            .unwrap();

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });
        let recipes = report.world.recipes();

        assert_eq!(
            recipes.property_sets.get("minecraft:furnace_input"),
            Some(&vec![42, 43])
        );
        assert_eq!(
            recipes.property_sets.get("minecraft:smithing_base"),
            Some(&vec![99])
        );
        assert_eq!(recipes.stonecutter_recipes.len(), 1);
        assert_eq!(recipes.stonecutter_recipes[0].input.item_ids, vec![11, 12]);
        assert_eq!(
            recipes.stonecutter_recipes[0]
                .option_display
                .display_type_id,
            4
        );
        assert_eq!(
            recipes.stonecutter_recipes[0].option_display.raw_payload,
            vec![4, 77]
        );

        assert_eq!(report.world_counters.update_recipes_packets, 1);
        assert_eq!(report.world_counters.recipe_property_sets_tracked, 2);
        assert_eq!(report.world_counters.recipe_property_set_items_tracked, 3);
        assert_eq!(report.world_counters.stonecutter_recipes_tracked, 1);
    }

    #[tokio::test]
    async fn probe_applies_world_time_weather_and_ticking_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::SetTime(PlayTime {
                game_time: 123,
                clock_updates: vec![ClockUpdate {
                    clock_id: 0,
                    total_ticks: 6000,
                    partial_tick: 0.25,
                    rate: 1.0,
                }],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::GameEvent(GameEvent {
                event_id: 7,
                param: 0.5,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::GameEvent(GameEvent {
                event_id: 8,
                param: 0.75,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::GameEvent(GameEvent {
                event_id: 3,
                param: 2.0,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::GameEvent(GameEvent {
                event_id: 11,
                param: 1.0,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::GameEvent(GameEvent {
                event_id: 12,
                param: 1.0,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::TickingState(TickingState {
                tick_rate: 0.25,
                frozen: true,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::TickingStep(TickingStep { tick_steps: 7 }))
            .await
            .unwrap();

        let report = probe.finish(8, ChunkPos { x: 0, z: 0 });
        let time = report.world.world_time().unwrap();

        assert_eq!(time.game_time, 123);
        assert_eq!(time.day_time, 6000);
        assert_eq!(
            time.clock_updates,
            vec![bbb_world::ClockUpdateState {
                clock_id: 0,
                total_ticks: 6000,
                partial_tick: 0.25,
                rate: 1.0,
            }]
        );
        assert_eq!(
            report.world.weather(),
            bbb_world::WorldWeatherState {
                raining: true,
                rain_level: 0.5,
                thunder_level: 0.75,
                last_game_event_id: Some(12),
                last_game_event_param: 1.0,
            }
        );
        assert_eq!(report.world.gameplay().game_type, 2);
        assert_eq!(report.world.gameplay().game_type_name, "adventure");
        assert_eq!(report.world.gameplay().previous_game_type, Some(0));
        assert!(!report.world.gameplay().show_death_screen);
        assert!(report.world.gameplay().do_limited_crafting);
        assert_eq!(
            report.world.ticking(),
            bbb_world::WorldTickingState {
                tick_rate: 1.0,
                frozen: true,
                frozen_ticks_to_run: 7,
            }
        );
        assert_eq!(report.world_counters.world_time_packets, 1);
        assert_eq!(report.world_counters.game_event_packets, 5);
        assert_eq!(report.world_counters.ticking_state_packets, 1);
        assert_eq!(report.world_counters.ticking_step_packets, 1);
    }

    #[tokio::test]
    async fn probe_applies_hud_session_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let boss_id = Uuid::from_u128(1);

        probe
            .handle_play_packet(PlayClientbound::BossEvent(BossEvent {
                id: boss_id,
                operation: BossEventOperation::Add {
                    name: "Ender Dragon".to_string(),
                    progress: 0.75,
                    color: BossBarColor::Purple,
                    overlay: BossBarOverlay::Progress,
                    flags: BossEventFlags {
                        darken_screen: true,
                        play_music: false,
                        create_world_fog: true,
                    },
                },
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::BossEvent(BossEvent {
                id: boss_id,
                operation: BossEventOperation::UpdateProgress { progress: 0.25 },
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::BossEvent(BossEvent {
                id: Uuid::from_u128(99),
                operation: BossEventOperation::UpdateProgress { progress: 1.0 },
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::TabList(TabList {
                header: Some("Welcome".to_string()),
                footer: None,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ChangeDifficulty(ChangeDifficulty {
                difficulty: Difficulty::Hard,
                locked: true,
            }))
            .await
            .unwrap();

        let report = probe.finish(5, ChunkPos { x: 0, z: 0 });
        let boss = report.world.boss_bars().get(&boss_id).unwrap();

        assert_eq!(boss.name, "Ender Dragon");
        assert_eq!(boss.progress, 0.25);
        assert_eq!(boss.color, "purple");
        assert_eq!(boss.overlay, "progress");
        assert!(boss.darken_screen);
        assert!(boss.create_world_fog);
        assert_eq!(report.world.tab_list().header.as_deref(), Some("Welcome"));
        assert_eq!(report.world.tab_list().footer, None);
        assert_eq!(report.world.difficulty().difficulty, "hard");
        assert!(report.world.difficulty().difficulty_locked);

        assert_eq!(report.world_counters.boss_event_packets, 3);
        assert_eq!(report.world_counters.boss_bars_tracked, 1);
        assert_eq!(report.world_counters.boss_events_ignored, 1);
        assert_eq!(report.world_counters.tab_list_packets, 1);
        assert_eq!(report.world_counters.change_difficulty_packets, 1);
    }

    #[tokio::test]
    async fn probe_applies_scoreboard_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::SetObjective(SetObjective {
                objective_name: "kills".to_string(),
                method: SetObjectiveMethod::Add,
                parameters: Some(SetObjectiveParameters {
                    display_name: "Kills".to_string(),
                    render_type: ObjectiveRenderType::Integer,
                    number_format: Some(vec![9]),
                }),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetDisplayObjective(SetDisplayObjective {
                slot: ScoreboardDisplaySlot::Sidebar,
                objective_name: Some("kills".to_string()),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetScore(SetScore {
                owner: "Steve".to_string(),
                objective_name: "kills".to_string(),
                score: 4,
                display: Some("Four".to_string()),
                number_format: None,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetScore(SetScore {
                owner: "Alex".to_string(),
                objective_name: "kills".to_string(),
                score: 1,
                display: None,
                number_format: None,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetPlayerTeam(SetPlayerTeam {
                name: "red".to_string(),
                method: PlayerTeamMethod::Add,
                parameters: Some(PlayerTeamParameters {
                    display_name: "Red Team".to_string(),
                    options: 0b11,
                    nametag_visibility: TeamVisibility::Always,
                    collision_rule: TeamCollisionRule::Never,
                    color: ChatFormatting::Red,
                    player_prefix: "[R]".to_string(),
                    player_suffix: "!".to_string(),
                }),
                players: vec!["Steve".to_string()],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ResetScore(ResetScore {
                owner: "Alex".to_string(),
                objective_name: Some("kills".to_string()),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetObjective(SetObjective {
                objective_name: "missing".to_string(),
                method: SetObjectiveMethod::Change,
                parameters: Some(SetObjectiveParameters {
                    display_name: "Missing".to_string(),
                    render_type: ObjectiveRenderType::Integer,
                    number_format: None,
                }),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetDisplayObjective(SetDisplayObjective {
                slot: ScoreboardDisplaySlot::List,
                objective_name: Some("missing".to_string()),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetScore(SetScore {
                owner: "Nobody".to_string(),
                objective_name: "missing".to_string(),
                score: 9,
                display: None,
                number_format: None,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetPlayerTeam(SetPlayerTeam {
                name: "missing".to_string(),
                method: PlayerTeamMethod::Join,
                parameters: None,
                players: vec!["Nobody".to_string()],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ResetScore(ResetScore {
                owner: "Nobody".to_string(),
                objective_name: Some("missing".to_string()),
            }))
            .await
            .unwrap();

        let report = probe.finish(11, ChunkPos { x: 0, z: 0 });
        let counters = &report.world_counters;
        assert_eq!(counters.set_objective_packets, 2);
        assert_eq!(counters.set_objective_updates_applied, 1);
        assert_eq!(counters.set_objective_updates_ignored, 1);
        assert_eq!(counters.set_display_objective_packets, 2);
        assert_eq!(counters.set_display_objective_updates_applied, 1);
        assert_eq!(counters.set_display_objective_updates_ignored, 1);
        assert_eq!(counters.set_score_packets, 3);
        assert_eq!(counters.set_score_updates_applied, 2);
        assert_eq!(counters.set_score_updates_ignored, 1);
        assert_eq!(counters.set_player_team_packets, 2);
        assert_eq!(counters.set_player_team_updates_applied, 1);
        assert_eq!(counters.set_player_team_updates_ignored, 1);
        assert_eq!(counters.reset_score_packets, 2);
        assert_eq!(counters.reset_score_updates_applied, 1);
        assert_eq!(counters.reset_score_updates_ignored, 1);

        let scoreboard = report.world.scoreboard();
        let objective = scoreboard.objectives.get("kills").unwrap();
        assert_eq!(objective.display_name, "Kills");
        assert_eq!(objective.render_type, "integer");
        assert_eq!(objective.number_format, Some(vec![9]));
        assert_eq!(
            scoreboard.display_slots.get("sidebar").map(String::as_str),
            Some("kills")
        );

        let steve_score = &scoreboard.scores["Steve"]["kills"];
        assert_eq!(steve_score.value, 4);
        assert_eq!(steve_score.display.as_deref(), Some("Four"));
        assert!(!scoreboard.scores.contains_key("Alex"));

        let team = scoreboard.teams.get("red").unwrap();
        assert!(team.players.contains("Steve"));
        let parameters = team.parameters.as_ref().unwrap();
        assert_eq!(parameters.display_name, "Red Team");
        assert_eq!(parameters.color, "red");
    }

    #[tokio::test]
    async fn probe_applies_block_changed_ack_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::BlockChangedAck(BlockChangedAck {
                sequence: 17,
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.block_changed_ack_packets, 1);
        assert_eq!(
            report.world.last_block_changed_ack(),
            Some(&bbb_world::BlockChangedAckState { sequence: 17 })
        );
    }

    #[tokio::test]
    async fn probe_applies_block_and_level_events_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::BlockEvent(BlockEvent {
                pos: ProtocolBlockPos {
                    x: 12,
                    y: 65,
                    z: -5,
                },
                b0: 2,
                b1: 9,
                block_id: 54,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::LevelEvent(LevelEvent {
                event_type: 1001,
                pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
                data: 42,
                global: true,
            }))
            .await
            .unwrap();

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.block_events_received, 1);
        assert_eq!(report.world_counters.block_events_tracked, 1);
        assert_eq!(report.world_counters.level_events_received, 1);
        assert_eq!(report.world_counters.level_events_tracked, 1);

        let block_event = report.world.block_events().first().unwrap();
        assert_eq!(
            block_event.pos,
            BlockPos {
                x: 12,
                y: 65,
                z: -5,
            }
        );
        assert_eq!(block_event.b0, 2);
        assert_eq!(block_event.b1, 9);
        assert_eq!(block_event.block_id, 54);

        let level_event = report.world.level_events().first().unwrap();
        assert_eq!(level_event.event_type, 1001);
        assert_eq!(level_event.pos, BlockPos { x: 3, y: 4, z: 5 });
        assert_eq!(level_event.data, 42);
        assert!(level_event.global);
    }

    #[tokio::test]
    async fn probe_applies_player_look_at_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::PlayerLookAt(PlayerLookAt {
                from_anchor: EntityAnchor::Eyes,
                position: ProtocolVec3d {
                    x: 12.0,
                    y: 65.0,
                    z: -7.0,
                },
                target: None,
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.player_look_at_packets, 1);
        assert_eq!(
            report.world.local_player().last_look_at,
            Some(bbb_world::LocalPlayerLookAtState {
                from_anchor: EntityAnchor::Eyes,
                position: ProtocolVec3d {
                    x: 12.0,
                    y: 65.0,
                    z: -7.0,
                },
                target_entity_id: None,
                to_anchor: None,
            })
        );
    }

    #[tokio::test]
    async fn probe_applies_player_health_position_and_rotation_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::SetHealth(PlayerHealth {
                health: 7.5,
                food: 16,
                saturation: 2.0,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::PlayerPosition(PlayerPositionUpdate {
                id: 23,
                position: ProtocolVec3d {
                    x: 10.0,
                    y: 64.0,
                    z: -5.0,
                },
                delta_movement: ProtocolVec3d {
                    x: 0.125,
                    y: 0.0,
                    z: 0.25,
                },
                y_rot: 90.0,
                x_rot: 15.0,
                relatives_mask: 0,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::PlayerRotation(PlayerRotationUpdate {
                y_rot: 10.0,
                relative_y: true,
                x_rot: -5.0,
                relative_x: false,
            }))
            .await
            .unwrap();

        let report = probe.finish(3, ChunkPos { x: 0, z: 0 });
        let local = report.world.local_player();
        assert_eq!(
            local.health,
            Some(bbb_world::LocalPlayerHealthState {
                health: 7.5,
                food: 16,
                saturation: 2.0,
            })
        );
        assert_eq!(report.world_counters.player_health_packets, 1);
        assert_eq!(report.world_counters.player_position_packets, 1);
        assert_eq!(report.world_counters.player_rotation_packets, 1);

        let pose = report.world.local_player_pose().unwrap();
        assert_eq!(
            pose.position,
            ProtocolVec3d {
                x: 10.0,
                y: 64.0,
                z: -5.0,
            }
        );
        assert_eq!(
            pose.delta_movement,
            ProtocolVec3d {
                x: 0.125,
                y: 0.0,
                z: 0.25,
            }
        );
        assert_eq!(pose.y_rot, 100.0);
        assert_eq!(pose.x_rot, -5.0);
        assert_eq!(pose.last_teleport_id, 23);
    }

    #[tokio::test]
    async fn probe_applies_local_player_status_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::PlayerAbilities(PlayerAbilities {
                invulnerable: true,
                flying: false,
                can_fly: true,
                instabuild: false,
                flying_speed: 0.08,
                walking_speed: 0.12,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetExperience(PlayerExperience {
                progress: 0.5,
                level: 12,
                total: 345,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetHeldSlot(SetHeldSlot { slot: 5 }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetHeldSlot(SetHeldSlot { slot: 99 }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetDefaultSpawnPosition(
                SetDefaultSpawnPosition {
                    dimension: "minecraft:overworld".to_string(),
                    pos: ProtocolBlockPos {
                        x: -12,
                        y: 70,
                        z: 44,
                    },
                    yaw: 180.0,
                    pitch: -15.0,
                },
            ))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetSimulationDistance(
                SetSimulationDistance { distance: 12 },
            ))
            .await
            .unwrap();

        let report = probe.finish(6, ChunkPos { x: 0, z: 0 });
        let local = report.world.local_player();

        assert_eq!(
            local.abilities,
            Some(bbb_world::LocalPlayerAbilitiesState {
                invulnerable: true,
                flying: false,
                can_fly: true,
                instabuild: false,
                flying_speed: 0.08,
                walking_speed: 0.12,
            })
        );
        assert_eq!(
            local.experience,
            Some(bbb_world::LocalPlayerExperienceState {
                progress: 0.5,
                level: 12,
                total: 345,
            })
        );
        assert_eq!(local.selected_hotbar_slot, 5);
        assert_eq!(
            local.default_spawn,
            Some(bbb_world::DefaultSpawnState {
                dimension: "minecraft:overworld".to_string(),
                pos: BlockPos {
                    x: -12,
                    y: 70,
                    z: 44,
                },
                yaw: 180.0,
                pitch: -15.0,
            })
        );
        assert_eq!(local.simulation_distance, Some(12));
        assert_eq!(report.world_counters.player_abilities_packets, 1);
        assert_eq!(report.world_counters.player_experience_packets, 1);
        assert_eq!(report.world_counters.held_slot_packets, 2);
        assert_eq!(report.world_counters.held_slot_updates_applied, 1);
        assert_eq!(report.world_counters.held_slot_updates_ignored, 1);
        assert_eq!(report.world_counters.default_spawn_position_packets, 1);
        assert_eq!(report.world_counters.simulation_distance_packets, 1);
    }

    #[tokio::test]
    async fn probe_applies_player_info_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let profile_id = Uuid::from_u128(1);
        let removed_profile_id = Uuid::from_u128(2);

        probe
            .handle_play_packet(PlayClientbound::PlayerInfoUpdate(PlayerInfoUpdate {
                actions: vec![
                    PlayerInfoAction::AddPlayer,
                    PlayerInfoAction::InitializeChat,
                    PlayerInfoAction::UpdateGameMode,
                    PlayerInfoAction::UpdateListed,
                    PlayerInfoAction::UpdateLatency,
                    PlayerInfoAction::UpdateDisplayName,
                    PlayerInfoAction::UpdateListOrder,
                    PlayerInfoAction::UpdateHat,
                ],
                entries: vec![
                    PlayerInfoEntry {
                        profile_id,
                        profile: Some(GameProfile {
                            uuid: profile_id,
                            name: "Ada".to_string(),
                            properties: vec![GameProfileProperty {
                                name: "textures".to_string(),
                                value: "skin".to_string(),
                                signature: Some("signature".to_string()),
                            }],
                        }),
                        listed: true,
                        latency: 42,
                        game_mode: GameType::Creative,
                        display_name: Some("Ada Lovelace".to_string()),
                        show_hat: true,
                        list_order: 3,
                        chat_session: Some(PlayerInfoChatSession {
                            session_id: Uuid::from_u128(3),
                            expires_at_epoch_millis: 99,
                            public_key: vec![1, 2],
                            key_signature: vec![3, 4],
                        }),
                    },
                    PlayerInfoEntry {
                        profile_id: removed_profile_id,
                        profile: Some(GameProfile {
                            uuid: removed_profile_id,
                            name: "Removed".to_string(),
                            properties: Vec::new(),
                        }),
                        listed: true,
                        latency: 7,
                        game_mode: GameType::Survival,
                        display_name: None,
                        show_hat: false,
                        list_order: 0,
                        chat_session: None,
                    },
                ],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::PlayerInfoRemove(PlayerInfoRemove {
                profile_ids: vec![removed_profile_id],
            }))
            .await
            .unwrap();

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });
        let entry = report.world.player_info_entry(profile_id).unwrap();
        assert_eq!(entry.profile.uuid, profile_id);
        assert_eq!(entry.profile.name, "Ada");
        assert_eq!(entry.profile.properties.len(), 1);
        assert!(entry.listed);
        assert_eq!(entry.latency, 42);
        assert_eq!(entry.game_mode, "creative");
        assert_eq!(entry.display_name.as_deref(), Some("Ada Lovelace"));
        assert!(entry.show_hat);
        assert_eq!(entry.list_order, 3);
        assert!(entry.chat_session_present);
        assert!(report.world.listed_players().contains(&profile_id));
        assert!(report.world.player_info_entry(removed_profile_id).is_none());
        assert!(!report.world.listed_players().contains(&removed_profile_id));

        assert_eq!(report.world_counters.player_info_update_packets, 1);
        assert_eq!(report.world_counters.player_info_remove_packets, 1);
        assert_eq!(report.world_counters.player_info_entries_tracked, 1);
        assert_eq!(report.world_counters.listed_players_tracked, 1);
    }

    #[tokio::test]
    async fn probe_applies_set_camera_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::Login(protocol_play_login(9)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetCamera(SetCamera { camera_id: 9 }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetCamera(SetCamera { camera_id: 123 }))
            .await
            .unwrap();

        let report = probe.finish(3, ChunkPos { x: 0, z: 0 });

        assert_eq!(
            report.world.local_player().camera,
            bbb_world::CameraState {
                entity_id: Some(9),
                follows_player: true,
                entity_known: true,
            }
        );
        assert_eq!(report.world_counters.set_camera_packets, 2);
        assert_eq!(report.world_counters.set_camera_updates_applied, 1);
        assert_eq!(report.world_counters.set_camera_updates_ignored, 1);
    }

    #[tokio::test]
    async fn probe_set_health_records_dead_health_without_auto_respawn() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::SetHealth(PlayerHealth {
                health: 0.0,
                food: 3,
                saturation: 0.5,
            }))
            .await
            .unwrap();

        assert!(
            timeout(Duration::from_millis(100), server.read_packet())
                .await
                .is_err(),
            "dead health alone must not send perform respawn"
        );

        assert_eq!(
            probe.world.local_player().health,
            Some(bbb_world::LocalPlayerHealthState {
                health: 0.0,
                food: 3,
                saturation: 0.5,
            })
        );
        assert!(probe.world.local_player_is_dead());
        assert_eq!(probe.world.counters().player_health_packets, 1);
    }

    #[tokio::test]
    async fn probe_play_unknown_packets_update_report_diagnostics() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        probe.state = ConnectionState::Play;

        probe
            .handle_play_packet(PlayClientbound::Unknown {
                packet_id: 0x7e,
                len: 9,
            })
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });
        assert_eq!(report.unsupported_packets, 1);
        assert_eq!(
            report.last_unsupported_packet_state.as_deref(),
            Some("Play")
        );
        assert_eq!(report.last_unsupported_packet_id, Some(0x7e));
        assert_eq!(report.last_unsupported_packet_len, Some(9));
        assert_eq!(report.world_counters.play_logins_received, 0);
    }

    fn probe_recipe_entry(id: i32, notification: bool, highlight: bool) -> RecipeBookAddEntry {
        RecipeBookAddEntry {
            contents: RecipeDisplayEntry {
                id: RecipeDisplayId { index: id },
                display: RecipeDisplaySummary {
                    display_type: RecipeDisplayType::Stonecutter,
                    raw_body: vec![3, id as u8],
                },
                group: None,
                category_id: id + 10,
                crafting_requirements: Some(vec![IngredientSummary {
                    tag: None,
                    item_ids: vec![40 + id],
                }]),
            },
            flags: (u8::from(notification)) | (u8::from(highlight) << 1),
            notification,
            highlight,
        }
    }

    async fn raw_connection_pair() -> (RawConnection, RawConnection) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let client = tokio::spawn(async move {
            RawConnection::connect(&addr.to_string(), None)
                .await
                .unwrap()
        });
        let (server_stream, _) = listener.accept().await.unwrap();
        let client = client.await.unwrap();
        let server = RawConnection {
            stream: server_stream,
            read_buf: BytesMut::with_capacity(8192),
            compression_threshold: None,
        };
        (client, server)
    }

    fn synthetic_probe_level_chunk_packet() -> LevelChunkWithLight {
        let mut sections = Encoder::new();
        sections.write_i16(0);
        sections.write_i16(0);
        sections.write_u8(0);
        sections.write_var_i32(0);
        sections.write_u8(0);
        sections.write_var_i32(0);

        LevelChunkWithLight {
            x: 1,
            z: -2,
            chunk_data: LevelChunkData {
                heightmaps: vec![ChunkHeightmapData {
                    kind_id: 1,
                    data: vec![42],
                }],
                section_data: sections.into_inner(),
                block_entities: vec![LevelChunkBlockEntity {
                    packed_xz: 0,
                    y: -64,
                    block_entity_type_id: 7,
                    raw_nbt: vec![0],
                }],
            },
            light_data: LightUpdateData {
                sky_y_mask: Vec::new(),
                block_y_mask: Vec::new(),
                empty_sky_y_mask: Vec::new(),
                empty_block_y_mask: Vec::new(),
                sky_updates: Vec::new(),
                block_updates: Vec::new(),
            },
        }
    }

    fn protocol_play_login(player_id: i32) -> PlayLogin {
        PlayLogin {
            player_id,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        }
    }

    fn protocol_player_chat_with_signature(
        global_index: i32,
        signature: MessageSignature,
    ) -> PlayerChat {
        PlayerChat {
            global_index,
            sender: Uuid::from_u128(0x1234),
            index: global_index,
            signature: Some(signature),
            body: SignedMessageBody {
                content: format!("message {global_index}"),
                timestamp_millis: i64::from(global_index),
                salt: i64::from(global_index) + 1,
                last_seen: Vec::new(),
            },
            unsigned_content: None,
            filter_mask: FilterMask {
                kind: FilterMaskKind::PassThrough,
                mask_words: Vec::new(),
            },
            chat_type: ChatTypeBound {
                chat_type: ChatTypeHolder::Registry { id: 0 },
                name: "Alice".to_string(),
                target_name: None,
            },
        }
    }

    fn protocol_add_entity(id: i32) -> AddEntity {
        AddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: 7,
            position: ProtocolVec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: -10.0,
            y_rot: 20.0,
            y_head_rot: 30.0,
            data: 99,
        }
    }

    fn protocol_add_entity_with_type(id: i32, entity_type_id: i32) -> AddEntity {
        AddEntity {
            entity_type_id,
            ..protocol_add_entity(id)
        }
    }
}
