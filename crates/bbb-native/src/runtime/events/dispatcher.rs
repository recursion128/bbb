use bbb_control::NetCounters;
use bbb_net::{NetCommand, NetEvent};
use bbb_world::{ChunkPos, WorldStore};
use tokio::sync::mpsc;

use crate::audio_runtime::AudioEventSink;
use crate::input::queue_vehicle_move_command;
use crate::particle_runtime::ParticleEventSink;

use super::client_state::*;
use super::control_state::apply_control_projection_event;
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
                if let Some(state) = world.last_sound() {
                    emit_positioned_sound(&mut audio_events, state);
                }
            }
            NetEvent::SoundEntity(update) => {
                let applied = world.apply_sound_entity_event(update);
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
                if let Some(state) = world.last_stop_sound() {
                    emit_stop_sound(&mut audio_events, state);
                }
            }
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
            }
            NetEvent::DamageEvent(update) => {
                world.apply_damage_event(update);
            }
            NetEvent::UpdateMobEffect(update) => {
                world.apply_update_mob_effect(update);
            }
            NetEvent::RemoveMobEffect(update) => {
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
                world.apply_block_destruction(update);
            }
            NetEvent::BossEvent(update) => {
                world.apply_boss_event(update);
            }
            NetEvent::ChangeDifficulty(update) => {
                world.apply_change_difficulty(update);
            }
            NetEvent::BlockEvent(event) => {
                world.apply_block_event(event);
            }
            NetEvent::LevelEvent(event) => {
                world.apply_level_event(event);
            }
            NetEvent::InitializeBorder(border) => {
                world.apply_initialize_border(border);
            }
            NetEvent::SetBorderCenter(update) => {
                world.apply_set_border_center(update);
            }
            NetEvent::SetBorderLerpSize(update) => {
                world.apply_set_border_lerp_size(update);
            }
            NetEvent::SetBorderSize(update) => {
                world.apply_set_border_size(update);
            }
            NetEvent::SetBorderWarningDelay(update) => {
                world.apply_set_border_warning_delay(update);
            }
            NetEvent::SetBorderWarningDistance(update) => {
                world.apply_set_border_warning_distance(update);
            }
            NetEvent::ResetScore(update) => {
                world.apply_reset_score(update);
            }
            NetEvent::SetDisplayObjective(update) => {
                world.apply_set_display_objective(update);
            }
            NetEvent::SetObjective(update) => {
                world.apply_set_objective(update);
            }
            NetEvent::SetPlayerTeam(update) => {
                world.apply_set_player_team(update);
            }
            NetEvent::SetScore(update) => {
                world.apply_set_score(update);
            }
            NetEvent::Commands(update) => {
                world.apply_commands(update);
            }
            NetEvent::CommandSuggestions(update) => {
                world.apply_command_suggestions(update);
            }
            NetEvent::TabList(update) => {
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
                let report = world.apply_move_vehicle(update);
                if let Some(report) = report {
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
            }
            NetEvent::UpdateTags(update) => {
                world.apply_update_tags(update);
            }
            NetEvent::Login(login) => {
                world.apply_login(&login);
                sync_local_player_counters(counters, world);
            }
            NetEvent::Respawn(respawn) => {
                world.apply_respawn(&respawn);
                sync_local_player_counters(counters, world);
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
