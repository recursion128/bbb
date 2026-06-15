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
                self.world.apply_move_vehicle(update);
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
                let (id, response) = packets::encode_play_cookie_response(&request.key, payload);
                self.conn.send_packet(id, &response).await?;
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
                let (id, payload) = packets::encode_play_configuration_acknowledged();
                self.conn.send_packet(id, &payload).await?;
                self.state = ConnectionState::Configuration;
                self.play_tick = None;
                self.seen_code_of_conduct = false;
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
                self.world.apply_player_health(health);
                maybe_send_perform_respawn(&mut self.conn, health, &mut self.player_was_dead)
                    .await?;
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
                self.world.apply_player_chat(update);
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
                self.world.apply_block_entity_data(update)?;
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
            PlayClientbound::LevelParticles(update) => {
                self.world.apply_level_particles(update);
            }
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
            PlayClientbound::Unknown { .. } => {}
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::RawConnection;
    use bbb_protocol::packets::{
        AwardStats, BlockChangedAck, BlockPos as ProtocolBlockPos, ChunkPos as ProtocolChunkPos,
        ClockUpdate, DebugBlockValue, DebugChunkValue, DebugEntityValue, DebugEvent, DebugSample,
        DialogHolder, EntityAnchor, GameEvent, GameRuleValue, GameRuleValues, GameTestHighlightPos,
        InteractionHand, MountScreenOpen, OpenBook, OpenSignEditor, PlaceGhostRecipe, PlayTime,
        PlayerHealth, PlayerLookAt, PlayerPositionUpdate, PlayerRotationUpdate, PongResponse,
        RecipeDisplayType, RemoteDebugSampleType, ShowDialog, StatUpdate, TestInstanceBlockStatus,
        TickingState, TickingStep, Vec3d as ProtocolVec3d, Vec3i as ProtocolVec3i,
    };
    use bbb_protocol::{codec::Decoder, ids};
    use bbb_world::{BlockPos, ChunkPos};
    use bytes::BytesMut;
    use std::time::Duration;
    use tokio::net::TcpListener;
    use tokio::time::timeout;

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

        let report = probe.finish(5, ChunkPos { x: 0, z: 0 });
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
                last_game_event_id: Some(8),
                last_game_event_param: 0.75,
            }
        );
        assert_eq!(
            report.world.ticking(),
            bbb_world::WorldTickingState {
                tick_rate: 1.0,
                frozen: true,
                frozen_ticks_to_run: 7,
            }
        );
        assert_eq!(report.world_counters.world_time_packets, 1);
        assert_eq!(report.world_counters.game_event_packets, 2);
        assert_eq!(report.world_counters.ticking_state_packets, 1);
        assert_eq!(report.world_counters.ticking_step_packets, 1);
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
}
