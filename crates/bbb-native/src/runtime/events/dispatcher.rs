use bbb_control::NetCounters;
use bbb_net::{NetCommand, NetEvent};
use bbb_world::{ChunkPos, WorldStore};
use tokio::sync::mpsc;

use crate::input::queue_vehicle_move_command;

use super::client_state::*;
use super::{apply_block_changed_ack, apply_game_event, apply_world_time_update};

pub(in crate::runtime) fn drain_net_events(
    rx: &mut mpsc::Receiver<NetEvent>,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) -> usize {
    let mut drained = 0;
    while drained < 4096 {
        let event = match rx.try_recv() {
            Ok(event) => event,
            Err(mpsc::error::TryRecvError::Empty) => break,
            Err(mpsc::error::TryRecvError::Disconnected) => {
                counters.connected = false;
                break;
            }
        };
        drained += 1;

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
                counters.last_cookie_key = Some(key);
                counters.cookie_request_packets += 1;
                if response_payload_present {
                    counters.cookie_response_hits += 1;
                } else {
                    counters.cookie_response_misses += 1;
                }
            }
            NetEvent::StoreCookie {
                key,
                payload_len,
                stored_cookie_count,
            } => {
                counters.last_cookie_key = Some(key);
                counters.store_cookie_packets += 1;
                counters.stored_cookie_count = stored_cookie_count;
                counters.stored_cookie_bytes =
                    counters.stored_cookie_bytes.saturating_add(payload_len);
            }
            NetEvent::CustomReportDetails(details) => {
                counters.custom_report_details = details.details;
                counters.custom_report_detail_packets += 1;
            }
            NetEvent::Transfer(transfer) => {
                counters.last_transfer = Some(bbb_control::TransferTarget {
                    host: transfer.host,
                    port: transfer.port,
                });
                counters.transfer_packets += 1;
            }
            NetEvent::PacketSeen { .. } => {
                counters.packets_seen += 1;
            }
            NetEvent::PlayerInfoUpdate(update) => {
                counters.player_info_update_packets += 1;
                world.apply_player_info_update(update);
            }
            NetEvent::PlayerInfoRemove(update) => {
                counters.player_info_remove_packets += 1;
                world.apply_player_info_remove(update);
            }
            NetEvent::ServerData(update) => {
                counters.server_data_packets += 1;
                world.apply_server_data(update);
            }
            NetEvent::ResourcePackPush(update) => {
                counters.resource_pack_push_packets += 1;
                world.apply_resource_pack_push(update);
            }
            NetEvent::ResourcePackPop(update) => {
                counters.resource_pack_pop_packets += 1;
                world.apply_resource_pack_pop(update);
            }
            NetEvent::Cooldown(update) => {
                counters.cooldown_packets += 1;
                world.apply_cooldown(update);
            }
            NetEvent::DamageEvent(update) => {
                counters.damage_event_packets += 1;
                world.apply_damage_event(update);
            }
            NetEvent::UpdateMobEffect(update) => {
                counters.update_mob_effect_packets += 1;
                world.apply_update_mob_effect(update);
            }
            NetEvent::RemoveMobEffect(update) => {
                counters.remove_mob_effect_packets += 1;
                world.apply_remove_mob_effect(update);
            }
            NetEvent::ContainerClose(update) => {
                world.apply_container_close(update);
            }
            NetEvent::ContainerSetContent(update) => {
                world.apply_container_set_content(update);
            }
            NetEvent::ContainerSetData(update) => {
                world.apply_container_set_data(update);
            }
            NetEvent::ContainerSetSlot(update) => {
                world.apply_container_set_slot(update);
            }
            NetEvent::OpenScreen(update) => {
                world.apply_open_screen(update);
            }
            NetEvent::SetCursorItem(update) => {
                world.apply_set_cursor_item(update);
            }
            NetEvent::SetPlayerInventory(update) => {
                world.apply_set_player_inventory(update);
            }
            NetEvent::BlockDestruction(update) => {
                counters.block_destruction_packets += 1;
                world.apply_block_destruction(update);
            }
            NetEvent::BossEvent(update) => {
                counters.boss_event_packets += 1;
                world.apply_boss_event(update);
            }
            NetEvent::ChangeDifficulty(update) => {
                counters.change_difficulty_packets += 1;
                world.apply_change_difficulty(update);
            }
            NetEvent::BlockEvent(event) => {
                counters.block_event_packets += 1;
                world.apply_block_event(event);
            }
            NetEvent::LevelEvent(event) => {
                counters.level_event_packets += 1;
                world.apply_level_event(event);
            }
            NetEvent::InitializeBorder(border) => {
                counters.initialize_border_packets += 1;
                world.apply_initialize_border(border);
            }
            NetEvent::SetBorderCenter(update) => {
                counters.set_border_center_packets += 1;
                world.apply_set_border_center(update);
            }
            NetEvent::SetBorderLerpSize(update) => {
                counters.set_border_lerp_size_packets += 1;
                world.apply_set_border_lerp_size(update);
            }
            NetEvent::SetBorderSize(update) => {
                counters.set_border_size_packets += 1;
                world.apply_set_border_size(update);
            }
            NetEvent::SetBorderWarningDelay(update) => {
                counters.set_border_warning_delay_packets += 1;
                world.apply_set_border_warning_delay(update);
            }
            NetEvent::SetBorderWarningDistance(update) => {
                counters.set_border_warning_distance_packets += 1;
                world.apply_set_border_warning_distance(update);
            }
            NetEvent::ResetScore(update) => {
                counters.reset_score_packets += 1;
                world.apply_reset_score(update);
            }
            NetEvent::SetDisplayObjective(update) => {
                counters.set_display_objective_packets += 1;
                world.apply_set_display_objective(update);
            }
            NetEvent::SetObjective(update) => {
                counters.set_objective_packets += 1;
                world.apply_set_objective(update);
            }
            NetEvent::SetPlayerTeam(update) => {
                counters.set_player_team_packets += 1;
                world.apply_set_player_team(update);
            }
            NetEvent::SetScore(update) => {
                counters.set_score_packets += 1;
                world.apply_set_score(update);
            }
            NetEvent::CommandSuggestions(update) => {
                counters.command_suggestion_packets += 1;
                world.apply_command_suggestions(update);
            }
            NetEvent::TabList(update) => {
                counters.tab_list_packets += 1;
                world.apply_tab_list(update);
            }
            NetEvent::AddEntity(entity) => {
                world.apply_add_entity(entity);
            }
            NetEvent::EntityAnimation(update) => {
                world.apply_entity_animation(update);
            }
            NetEvent::EntityEvent(update) => {
                world.apply_entity_event(update);
            }
            NetEvent::HurtAnimation(update) => {
                world.apply_hurt_animation(update);
            }
            NetEvent::MoveEntity(update) => {
                world.apply_entity_move(update);
            }
            NetEvent::MoveVehicle(update) => {
                counters.move_vehicle_packets += 1;
                if let Some(report) = world.apply_move_vehicle(update) {
                    queue_vehicle_move_command(counters, net_commands, report);
                }
            }
            NetEvent::EntityPositionSync(update) => {
                world.apply_entity_position_sync(update);
            }
            NetEvent::RemoveEntities(update) => {
                world.apply_remove_entities(update);
            }
            NetEvent::RotateHead(update) => {
                world.apply_rotate_head(update);
            }
            NetEvent::SetEntityMotion(update) => {
                world.apply_set_entity_motion(update);
            }
            NetEvent::SetEntityLink(update) => {
                world.apply_set_entity_link(update);
            }
            NetEvent::SetEquipment(update) => {
                world.apply_set_equipment(update);
            }
            NetEvent::TakeItemEntity(update) => {
                world.apply_take_item_entity(update);
                counters.take_item_entity_packets += 1;
            }
            NetEvent::SetPassengers(update) => {
                world.apply_set_passengers(update);
            }
            NetEvent::UpdateAttributes(update) => {
                world.apply_update_attributes(update);
            }
            NetEvent::SetEntityData(update) => {
                world.apply_set_entity_data(update);
            }
            NetEvent::TeleportEntity(update) => {
                world.apply_teleport_entity(update);
            }
            NetEvent::RegistryData {
                registry,
                raw_payload_len,
            } => {
                world.record_registry(registry, raw_payload_len);
                counters.registries_seen = world.counters().registries_seen;
            }
            NetEvent::Login(login) => {
                counters.player_entity_id = Some(login.player_id);
                world.apply_login(&login);
            }
            NetEvent::Respawn(respawn) => {
                world.apply_respawn(&respawn);
            }
            NetEvent::PlayerPosition(update) => {
                apply_player_position_update(counters, update);
            }
            NetEvent::PlayerRotation(update) => {
                apply_player_rotation_update(counters, update);
            }
            NetEvent::PlayerAbilities(abilities) => {
                apply_player_abilities_update(counters, abilities);
            }
            NetEvent::PlayerHealth(health) => {
                apply_player_health_update(counters, health);
            }
            NetEvent::PlayerExperience(experience) => {
                apply_player_experience_update(counters, experience);
            }
            NetEvent::HeldSlot(slot) => {
                apply_held_slot_update(counters, slot);
            }
            NetEvent::SetDefaultSpawnPosition(spawn) => {
                apply_default_spawn_update(counters, spawn);
            }
            NetEvent::SetSimulationDistance(distance) => {
                apply_simulation_distance_update(counters, distance);
            }
            NetEvent::SystemChat(chat) => {
                apply_system_chat_update(counters, chat);
            }
            NetEvent::SetActionBarText(text) => {
                apply_action_bar_update(counters, text);
            }
            NetEvent::SetTitleText(text) => {
                apply_title_text_update(counters, text);
            }
            NetEvent::SetSubtitleText(text) => {
                apply_subtitle_text_update(counters, text);
            }
            NetEvent::ClearTitles(clear) => {
                apply_clear_titles_update(counters, clear);
            }
            NetEvent::SetTitlesAnimation(animation) => {
                apply_titles_animation_update(counters, animation);
            }
            NetEvent::TickingState(ticking) => {
                apply_ticking_state_update(counters, ticking);
            }
            NetEvent::TickingStep(step) => {
                apply_ticking_step_update(counters, step);
            }
            NetEvent::SetCamera(camera) => {
                apply_set_camera_update(counters, world, camera);
            }
            NetEvent::BlockChangedAck(ack) => {
                apply_block_changed_ack(counters, ack);
            }
            NetEvent::BlockEntityData(update) => match world.apply_block_entity_data(update) {
                Ok(_) => {}
                Err(err) => {
                    counters.last_error = Some(err.to_string());
                }
            },
            NetEvent::GameEvent(event) => {
                apply_game_event(counters, event);
            }
            NetEvent::SetTime(time) => {
                apply_world_time_update(counters, time);
            }
            NetEvent::LevelChunkWithLight(chunk) => {
                match world.insert_level_chunk_with_light(chunk) {
                    Ok(pos) => {
                        counters.first_chunk.get_or_insert(pos);
                    }
                    Err(err) => {
                        counters.last_error = Some(err.to_string());
                    }
                }
            }
            NetEvent::LightUpdate(update) => match world.apply_light_update(update) {
                Ok(_) => {}
                Err(err) => {
                    counters.last_error = Some(err.to_string());
                }
            },
            NetEvent::ChunksBiomes(update) => match world.apply_biome_update(update) {
                Ok(_) => {}
                Err(err) => {
                    counters.last_error = Some(err.to_string());
                }
            },
            NetEvent::ForgetLevelChunk(update) => {
                world.forget_chunk(ChunkPos {
                    x: update.pos.x,
                    z: update.pos.z,
                });
            }
            NetEvent::BlockUpdate(update) => {
                world.apply_block_update(update);
            }
            NetEvent::SectionBlocksUpdate(update) => {
                world.apply_section_blocks_update(update);
            }
            NetEvent::SetChunkCacheCenter(update) => {
                counters.chunk_cache_center = Some(ChunkPos {
                    x: update.chunk_x,
                    z: update.chunk_z,
                });
            }
            NetEvent::SetChunkCacheRadius(update) => {
                counters.chunk_cache_radius = Some(update.radius);
            }
        }
    }
    drained
}
