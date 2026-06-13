use bbb_control::NetCounters;
use bbb_net::{NetCommand, NetEvent};
use bbb_world::{ChunkPos, WorldStore};
use tokio::sync::mpsc;

use crate::input::queue_vehicle_move_command;

use super::client_state::*;
use super::control_state::{apply_control_projection_event, sync_registry_counters};
use super::{apply_block_changed_ack, sync_weather_counters, sync_world_time_counters};

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

        let Some(event) = apply_control_projection_event(event, counters, world) else {
            continue;
        };

        match event {
            NetEvent::RecipeBookAdd(update) => {
                world.apply_recipe_book_add(update);
            }
            NetEvent::RecipeBookRemove(update) => {
                world.apply_recipe_book_remove(update);
            }
            NetEvent::RecipeBookSettings(update) => {
                world.apply_recipe_book_settings(update);
            }
            NetEvent::UpdateAdvancements(update) => {
                world.apply_update_advancements(update);
            }
            NetEvent::UpdateRecipes(update) => {
                world.apply_update_recipes(update);
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
            NetEvent::MerchantOffers(update) => {
                world.apply_merchant_offers(update);
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
            NetEvent::Commands(update) => {
                counters.command_tree_packets += 1;
                world.apply_commands(update);
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
            NetEvent::MoveMinecartAlongTrack(update) => {
                world.apply_move_minecart_along_track(update);
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
            NetEvent::RegistryData(update) => {
                world.record_registry_data(update);
                sync_registry_counters(counters, world);
            }
            NetEvent::UpdateTags(update) => {
                world.apply_update_tags(update);
                let world_counters = world.counters();
                counters.update_tags_packets = world_counters.update_tags_packets;
                counters.last_update_tags_registry_count =
                    world_counters.last_update_tags_registry_count;
                counters.last_update_tags_total_tag_count =
                    world_counters.last_update_tags_total_tag_count;
                counters.last_update_tags_total_value_count =
                    world_counters.last_update_tags_total_value_count;
                counters.tag_registries_tracked = world_counters.tag_registries_tracked;
                counters.tags_tracked = world_counters.tags_tracked;
                counters.tag_entries_tracked = world_counters.tag_entries_tracked;
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
                world.apply_player_abilities(abilities);
                sync_local_player_counters(counters, world);
            }
            NetEvent::PlayerHealth(health) => {
                world.apply_player_health(health);
                sync_local_player_counters(counters, world);
            }
            NetEvent::PlayerExperience(experience) => {
                world.apply_player_experience(experience);
                sync_local_player_counters(counters, world);
            }
            NetEvent::HeldSlot(slot) => {
                world.apply_held_slot(slot);
                sync_local_player_counters(counters, world);
            }
            NetEvent::SetDefaultSpawnPosition(spawn) => {
                world.apply_default_spawn_position(spawn);
                sync_local_player_counters(counters, world);
            }
            NetEvent::SetSimulationDistance(distance) => {
                world.apply_simulation_distance(distance);
                sync_local_player_counters(counters, world);
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
                world.apply_ticking_state(ticking);
                sync_ticking_counters(counters, world);
            }
            NetEvent::TickingStep(step) => {
                world.apply_ticking_step(step);
                sync_ticking_counters(counters, world);
            }
            NetEvent::SetCamera(camera) => {
                world.apply_set_camera(camera);
                sync_local_player_counters(counters, world);
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
                world.apply_game_event(event);
                sync_weather_counters(counters, world);
            }
            NetEvent::SetTime(time) => {
                world.apply_world_time(time);
                sync_world_time_counters(counters, world);
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
                world.apply_set_chunk_cache_center(update);
                sync_chunk_cache_counters(counters, world);
            }
            NetEvent::SetChunkCacheRadius(update) => {
                world.apply_set_chunk_cache_radius(update);
                sync_chunk_cache_counters(counters, world);
            }
            _ => unreachable!("control projection event reached world dispatcher"),
        }
    }
    drained
}

fn sync_chunk_cache_counters(counters: &mut NetCounters, world: &WorldStore) {
    counters.chunk_cache_center = world.chunk_cache_center();
    counters.chunk_cache_radius = world.chunk_cache_radius();
}
