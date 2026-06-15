use bbb_control::NetCounters;
use bbb_net::{NetCommand, NetEvent};
use bbb_world::{ChunkPos, WorldStore};
use tokio::sync::mpsc;

use crate::audio_runtime::AudioEventSink;
use crate::input::queue_vehicle_move_command;
use crate::particle_runtime::ParticleEventSink;

use super::client_state::*;
use super::control_state::{
    apply_control_projection_event, sync_advancement_counters, sync_block_event_counters,
    sync_client_audio_counters, sync_command_counters, sync_entity_counters,
    sync_entity_interaction_counters, sync_entity_status_counters, sync_hud_session_counters,
    sync_inventory_counters, sync_recipe_access_counters, sync_recipe_book_counters,
    sync_scoreboard_counters, sync_world_border_counters,
};
use super::{sync_weather_counters, sync_world_time_counters};

#[cfg(test)]
pub(in crate::runtime) fn drain_net_events(
    rx: &mut mpsc::Receiver<NetEvent>,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) -> usize {
    drain_net_events_with_sinks(rx, world, counters, net_commands, None, None, None)
}

#[cfg(test)]
pub(in crate::runtime) fn drain_net_events_with_audio(
    rx: &mut mpsc::Receiver<NetEvent>,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    audio_events: Option<&mut dyn AudioEventSink>,
) -> usize {
    drain_net_events_with_sinks(rx, world, counters, net_commands, audio_events, None, None)
}

pub(in crate::runtime) fn drain_net_events_with_sinks(
    rx: &mut mpsc::Receiver<NetEvent>,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    mut audio_events: Option<&mut dyn AudioEventSink>,
    mut particle_events: Option<&mut dyn ParticleEventSink>,
    mut particle_renderer: Option<&mut bbb_renderer::Renderer>,
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

        if let NetEvent::LevelParticles(update) = &event {
            emit_level_particles(&mut particle_events, &mut particle_renderer, update);
        }

        let Some(event) = apply_control_projection_event(event, counters, world) else {
            continue;
        };

        match event {
            NetEvent::Sound(update) => {
                world.apply_sound_event(update);
                sync_client_audio_counters(counters, world);
                if let Some(state) = world.last_sound() {
                    emit_positioned_sound(&mut audio_events, state);
                }
            }
            NetEvent::SoundEntity(update) => {
                let applied = world.apply_sound_entity_event(update);
                sync_client_audio_counters(counters, world);
                if applied {
                    if let Some(state) = world.last_sound_entity() {
                        let position = world
                            .probe_entity_transform(state.entity_id)
                            .map(|entity| audio_position(entity.position));
                        emit_entity_sound(&mut audio_events, state, position);
                    }
                }
            }
            NetEvent::StopSound(update) => {
                world.apply_stop_sound(update);
                sync_client_audio_counters(counters, world);
                if let Some(state) = world.last_stop_sound() {
                    emit_stop_sound(&mut audio_events, state);
                }
            }
            NetEvent::RecipeBookAdd(update) => {
                world.apply_recipe_book_add(update);
                sync_recipe_book_counters(counters, world);
            }
            NetEvent::RecipeBookRemove(update) => {
                world.apply_recipe_book_remove(update);
                sync_recipe_book_counters(counters, world);
            }
            NetEvent::RecipeBookSettings(update) => {
                world.apply_recipe_book_settings(update);
                sync_recipe_book_counters(counters, world);
            }
            NetEvent::UpdateAdvancements(update) => {
                world.apply_update_advancements(update);
                sync_advancement_counters(counters, world);
            }
            NetEvent::UpdateRecipes(update) => {
                world.apply_update_recipes(update);
                sync_recipe_access_counters(counters, world);
            }
            NetEvent::PlayerInfoUpdate(update) => {
                world.apply_player_info_update(update);
            }
            NetEvent::PlayerInfoRemove(update) => {
                world.apply_player_info_remove(update);
            }
            NetEvent::ServerData(update) => {
                world.apply_server_data(update);
            }
            NetEvent::ResourcePackPush(update) => {
                world.apply_resource_pack_push(update);
            }
            NetEvent::ResourcePackPop(update) => {
                world.apply_resource_pack_pop(update);
            }
            NetEvent::Cooldown(update) => {
                world.apply_cooldown(update);
                sync_entity_status_counters(counters, world);
            }
            NetEvent::DamageEvent(update) => {
                world.apply_damage_event(update);
                sync_entity_status_counters(counters, world);
            }
            NetEvent::UpdateMobEffect(update) => {
                world.apply_update_mob_effect(update);
                sync_entity_status_counters(counters, world);
            }
            NetEvent::RemoveMobEffect(update) => {
                world.apply_remove_mob_effect(update);
                sync_entity_status_counters(counters, world);
            }
            NetEvent::ContainerClose(update) => {
                world.apply_container_close(update);
                sync_inventory_counters(counters, world);
            }
            NetEvent::ContainerSetContent(update) => {
                world.apply_container_set_content(update);
                sync_inventory_counters(counters, world);
            }
            NetEvent::ContainerSetData(update) => {
                world.apply_container_set_data(update);
                sync_inventory_counters(counters, world);
            }
            NetEvent::ContainerSetSlot(update) => {
                world.apply_container_set_slot(update);
                sync_inventory_counters(counters, world);
            }
            NetEvent::MerchantOffers(update) => {
                world.apply_merchant_offers(update);
                sync_inventory_counters(counters, world);
            }
            NetEvent::OpenScreen(update) => {
                world.apply_open_screen(update);
                sync_inventory_counters(counters, world);
            }
            NetEvent::SetCursorItem(update) => {
                world.apply_set_cursor_item(update);
                sync_inventory_counters(counters, world);
            }
            NetEvent::SetPlayerInventory(update) => {
                world.apply_set_player_inventory(update);
                sync_inventory_counters(counters, world);
            }
            NetEvent::BlockDestruction(update) => {
                world.apply_block_destruction(update);
                sync_block_event_counters(counters, world);
            }
            NetEvent::BossEvent(update) => {
                world.apply_boss_event(update);
                sync_hud_session_counters(counters, world);
            }
            NetEvent::ChangeDifficulty(update) => {
                world.apply_change_difficulty(update);
                sync_hud_session_counters(counters, world);
            }
            NetEvent::BlockEvent(event) => {
                world.apply_block_event(event);
                sync_block_event_counters(counters, world);
            }
            NetEvent::LevelEvent(event) => {
                world.apply_level_event(event);
                sync_block_event_counters(counters, world);
            }
            NetEvent::InitializeBorder(border) => {
                world.apply_initialize_border(border);
                sync_world_border_counters(counters, world);
            }
            NetEvent::SetBorderCenter(update) => {
                world.apply_set_border_center(update);
                sync_world_border_counters(counters, world);
            }
            NetEvent::SetBorderLerpSize(update) => {
                world.apply_set_border_lerp_size(update);
                sync_world_border_counters(counters, world);
            }
            NetEvent::SetBorderSize(update) => {
                world.apply_set_border_size(update);
                sync_world_border_counters(counters, world);
            }
            NetEvent::SetBorderWarningDelay(update) => {
                world.apply_set_border_warning_delay(update);
                sync_world_border_counters(counters, world);
            }
            NetEvent::SetBorderWarningDistance(update) => {
                world.apply_set_border_warning_distance(update);
                sync_world_border_counters(counters, world);
            }
            NetEvent::ResetScore(update) => {
                world.apply_reset_score(update);
                sync_scoreboard_counters(counters, world);
            }
            NetEvent::SetDisplayObjective(update) => {
                world.apply_set_display_objective(update);
                sync_scoreboard_counters(counters, world);
            }
            NetEvent::SetObjective(update) => {
                world.apply_set_objective(update);
                sync_scoreboard_counters(counters, world);
            }
            NetEvent::SetPlayerTeam(update) => {
                world.apply_set_player_team(update);
                sync_scoreboard_counters(counters, world);
            }
            NetEvent::SetScore(update) => {
                world.apply_set_score(update);
                sync_scoreboard_counters(counters, world);
            }
            NetEvent::Commands(update) => {
                world.apply_commands(update);
                sync_command_counters(counters, world);
            }
            NetEvent::CommandSuggestions(update) => {
                world.apply_command_suggestions(update);
                sync_command_counters(counters, world);
            }
            NetEvent::TabList(update) => {
                world.apply_tab_list(update);
                sync_hud_session_counters(counters, world);
            }
            NetEvent::AddEntity(entity) => {
                world.apply_add_entity(entity);
                sync_entity_counters(counters, world);
                sync_entity_status_counters(counters, world);
            }
            NetEvent::EntityAnimation(update) => {
                world.apply_entity_animation(update);
                sync_entity_counters(counters, world);
            }
            NetEvent::EntityEvent(update) => {
                world.apply_entity_event(update);
                sync_entity_counters(counters, world);
            }
            NetEvent::HurtAnimation(update) => {
                world.apply_hurt_animation(update);
                sync_entity_counters(counters, world);
            }
            NetEvent::MoveEntity(update) => {
                world.apply_entity_move(update);
                sync_entity_counters(counters, world);
            }
            NetEvent::MoveMinecartAlongTrack(update) => {
                world.apply_move_minecart_along_track(update);
                sync_entity_counters(counters, world);
            }
            NetEvent::MoveVehicle(update) => {
                let report = world.apply_move_vehicle(update);
                sync_entity_interaction_counters(counters, world);
                if let Some(report) = report {
                    queue_vehicle_move_command(counters, net_commands, report);
                }
            }
            NetEvent::EntityPositionSync(update) => {
                world.apply_entity_position_sync(update);
                sync_entity_counters(counters, world);
            }
            NetEvent::RemoveEntities(update) => {
                world.apply_remove_entities(update);
                sync_entity_counters(counters, world);
                sync_entity_status_counters(counters, world);
            }
            NetEvent::RotateHead(update) => {
                world.apply_rotate_head(update);
                sync_entity_counters(counters, world);
            }
            NetEvent::SetEntityMotion(update) => {
                world.apply_set_entity_motion(update);
                sync_entity_counters(counters, world);
            }
            NetEvent::SetEntityLink(update) => {
                world.apply_set_entity_link(update);
                sync_entity_counters(counters, world);
            }
            NetEvent::SetEquipment(update) => {
                world.apply_set_equipment(update);
                sync_entity_counters(counters, world);
            }
            NetEvent::TakeItemEntity(update) => {
                world.apply_take_item_entity(update);
                sync_entity_interaction_counters(counters, world);
                sync_entity_counters(counters, world);
            }
            NetEvent::SetPassengers(update) => {
                world.apply_set_passengers(update);
                sync_entity_counters(counters, world);
            }
            NetEvent::UpdateAttributes(update) => {
                world.apply_update_attributes(update);
                sync_entity_counters(counters, world);
            }
            NetEvent::SetEntityData(update) => {
                world.apply_set_entity_data(update);
                sync_entity_counters(counters, world);
            }
            NetEvent::TeleportEntity(update) => {
                world.apply_teleport_entity(update);
                sync_entity_counters(counters, world);
            }
            NetEvent::RegistryData(update) => {
                world.record_registry_data(update);
            }
            NetEvent::UpdateTags(update) => {
                world.apply_update_tags(update);
            }
            NetEvent::Login(login) => {
                world.apply_login(&login);
                sync_local_player_counters(counters, world);
                sync_entity_counters(counters, world);
                sync_entity_status_counters(counters, world);
                sync_block_event_counters(counters, world);
            }
            NetEvent::Respawn(respawn) => {
                world.apply_respawn(&respawn);
                sync_local_player_counters(counters, world);
                sync_entity_counters(counters, world);
                sync_entity_status_counters(counters, world);
                sync_block_event_counters(counters, world);
            }
            NetEvent::PlayerPosition(update) => {
                apply_player_position_update(counters, world, update);
            }
            NetEvent::PlayerRotation(update) => {
                apply_player_rotation_update(counters, world, update);
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
                apply_system_chat_update(counters, world, chat);
            }
            NetEvent::SetActionBarText(text) => {
                apply_action_bar_update(counters, world, text);
            }
            NetEvent::SetTitleText(text) => {
                apply_title_text_update(counters, world, text);
            }
            NetEvent::SetSubtitleText(text) => {
                apply_subtitle_text_update(counters, world, text);
            }
            NetEvent::ClearTitles(clear) => {
                apply_clear_titles_update(counters, world, clear);
            }
            NetEvent::SetTitlesAnimation(animation) => {
                apply_titles_animation_update(counters, world, animation);
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
                world.apply_block_changed_ack(ack);
                sync_block_event_counters(counters, world);
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
                    Ok(_) => {}
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
            }
            NetEvent::SetChunkCacheRadius(update) => {
                world.apply_set_chunk_cache_radius(update);
            }
            _ => unreachable!("control projection event reached world dispatcher"),
        }
    }
    drained
}

fn emit_positioned_sound(
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    state: &bbb_world::SoundEventState,
) {
    if let Some(audio_events) = audio_events.as_deref_mut() {
        audio_events.play_positioned_sound(state);
    }
}

fn emit_entity_sound(
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    state: &bbb_world::SoundEntityEventState,
    position: Option<[f64; 3]>,
) {
    if let Some(audio_events) = audio_events.as_deref_mut() {
        audio_events.play_entity_sound(state, position);
    }
}

fn emit_stop_sound(
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    state: &bbb_world::StopSoundEventState,
) {
    if let Some(audio_events) = audio_events.as_deref_mut() {
        audio_events.stop_sound(state);
    }
}

fn emit_level_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_renderer: &mut Option<&mut bbb_renderer::Renderer>,
    packet: &bbb_protocol::packets::LevelParticles,
) {
    if let Some(particle_events) = particle_events.as_deref_mut() {
        let batch = particle_events.spawn_level_particles(packet);
        if let Some(renderer) = particle_renderer.as_deref_mut() {
            renderer.submit_particle_spawns(batch);
        }
    }
}

fn audio_position(position: bbb_world::EntityVec3) -> [f64; 3] {
    [position.x, position.y, position.z]
}
