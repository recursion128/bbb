use bbb_control::NetCounters;
use bbb_net::{ConnectionState, NetCommand, NetEvent};
use bbb_pack::{JukeboxSongRegistry, SoundEventRegistry};
use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, RegistryData, Vec3d as ProtocolVec3d};
use bbb_world::{
    advance_cobweb_place_particle_randoms, ChunkPos, LevelEventSoundRandomState, TerrainFluidKind,
    WorldStore,
};
use tokio::sync::mpsc;

use crate::audio_runtime::AudioEventSink;
use crate::input::queue_vehicle_move_command;
use crate::particle_runtime::{
    LevelEventDripstoneDripParticle, LevelEventGrowthParticleContext, LevelEventGrowthParticleMode,
    LevelEventGrowthParticleSupport, LevelEventParticleContext, LevelParticleSpawnContext,
    ParticleEventSink,
};

use super::client_state::*;
use super::control_state::apply_control_projection_event;

const COBWEB_PLACE_LEVEL_EVENT: i32 = 3018;
const COMPOSTER_FILL_LEVEL_EVENT: i32 = 1500;
const DRIPSTONE_DRIP_LEVEL_EVENT: i32 = 1504;
const PLANT_GROWTH_LEVEL_EVENT: i32 = 1505;
const BEE_GROWTH_PARTICLES_LEVEL_EVENT: i32 = 2011;
const TURTLE_EGG_PLACEMENT_PARTICLES_LEVEL_EVENT: i32 = 2012;
const POINTED_DRIPSTONE_ROOT_SEARCH_LENGTH: i32 = 11;

#[cfg(test)]
pub(in crate::runtime) fn drain_net_events(
    rx: &mut mpsc::Receiver<NetEvent>,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) -> usize {
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);
    drain_net_events_with_sinks(
        rx,
        world,
        counters,
        net_commands,
        None,
        None,
        None,
        &mut level_event_sound_random,
    )
}

#[cfg(test)]
pub(in crate::runtime) fn drain_net_events_with_audio(
    rx: &mut mpsc::Receiver<NetEvent>,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    audio_events: Option<&mut dyn AudioEventSink>,
) -> usize {
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);
    drain_net_events_with_sinks(
        rx,
        world,
        counters,
        net_commands,
        audio_events,
        None,
        None,
        &mut level_event_sound_random,
    )
}

pub(in crate::runtime) fn drain_net_events_with_sinks(
    rx: &mut mpsc::Receiver<NetEvent>,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    mut audio_events: Option<&mut dyn AudioEventSink>,
    mut particle_events: Option<&mut dyn ParticleEventSink>,
    mut particle_renderer: Option<&mut bbb_renderer::Renderer>,
    level_event_sound_random: &mut LevelEventSoundRandomState,
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

        apply_control_projection_event(&event, counters);

        match event {
            NetEvent::StartConfiguration {
                pending_chat_acknowledgement,
            } => {
                let command = world.take_pending_player_chat_acknowledgement();
                let _ = pending_chat_acknowledgement.send(command);
                world.clear_client_level();
            }
            NetEvent::StateChanged {
                state: ConnectionState::Configuration,
            } => {
                world.clear_client_level();
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
            NetEvent::CustomPayload(update) => {
                world.apply_custom_payload(update);
            }
            NetEvent::ServerLinks(links) => {
                world.apply_server_links(links);
            }
            NetEvent::Transfer(transfer) => {
                world.apply_transfer(transfer);
            }
            NetEvent::ResetChat => {
                world.apply_reset_chat();
            }
            NetEvent::UpdateEnabledFeatures(update) => {
                world.apply_update_enabled_features(update);
            }
            NetEvent::SelectKnownPacks {
                known_packs,
                selected_packs,
            } => {
                world.apply_select_known_packs(known_packs, selected_packs);
            }
            NetEvent::CodeOfConduct { text } => {
                world.apply_code_of_conduct(text);
            }
            NetEvent::CustomChatCompletions(update) => {
                world.apply_custom_chat_completions(update);
            }
            NetEvent::Sound(update) => {
                let state = world.apply_sound_event(update);
                emit_positioned_sound(&mut audio_events, &state);
            }
            NetEvent::SoundEntity(update) => {
                if let Some(state) = world.apply_sound_entity_event(update) {
                    let position = world
                        .probe_entity_transform(state.entity_id)
                        .map(|entity| audio_position(entity.position));
                    emit_entity_sound(&mut audio_events, &state, position);
                }
            }
            NetEvent::StopSound(update) => {
                let state = world.apply_stop_sound(update);
                emit_stop_sound(&mut audio_events, &state);
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
                apply_player_look_at_update(world, update);
            }
            NetEvent::PongResponse(update) => {
                world.apply_pong_response(update);
            }
            NetEvent::Explosion(update) => {
                world.apply_explosion(update);
            }
            NetEvent::LevelParticles(update) => {
                world.apply_level_particles(update.clone());
                emit_level_particles(
                    &mut particle_events,
                    &mut particle_renderer,
                    &update,
                    level_particle_spawn_context(world),
                );
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
                if let Some(command) = world.apply_player_chat(update) {
                    queue_chat_acknowledgement(net_commands, counters, command);
                }
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
            NetEvent::SelectAdvancementsTab(update) => {
                world.apply_select_advancements_tab(update);
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
            NetEvent::ResourcePackResponse { id, action } => {
                world.apply_resource_pack_response(id, action);
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
                let jukebox_event = world.apply_level_event(event);
                if let Some(jukebox_event) = jukebox_event {
                    emit_jukebox_level_event(&mut audio_events, &jukebox_event);
                }
                if let Some(state) =
                    camera_audio_position_from_world(world).and_then(|camera_position| {
                        world.global_level_event_sound(event, camera_position)
                    })
                {
                    let state = world.record_positioned_sound(with_level_event_sound_seed(
                        state,
                        level_event_sound_random,
                    ));
                    emit_positioned_sound(&mut audio_events, &state);
                }
                if let Some(state) = world
                    .level_event_local_sound_with_random(event, || {
                        level_event_sound_random.next_float()
                    })
                    .map(|state| world.record_local_sound(state))
                {
                    emit_local_sound(&mut audio_events, &state);
                }
                if event.event_type == COBWEB_PLACE_LEVEL_EVENT {
                    let particles_consumed_random = emit_level_event_particles(
                        &mut particle_events,
                        &mut particle_renderer,
                        &event,
                        level_event_particle_context(world, &event),
                        level_event_sound_random,
                    );
                    if !particles_consumed_random {
                        advance_cobweb_place_particle_randoms(level_event_sound_random);
                    }
                    if let Some(state) = world
                        .cobweb_place_level_event_sound_with_random(event, || {
                            level_event_sound_random.next_float()
                        })
                    {
                        let state = world.record_positioned_sound(with_level_event_sound_seed(
                            state,
                            level_event_sound_random,
                        ));
                        emit_positioned_sound(&mut audio_events, &state);
                    }
                } else {
                    if let Some(state) = world.level_event_sound_with_random(event, || {
                        level_event_sound_random.next_float()
                    }) {
                        let state = world.record_positioned_sound(with_level_event_sound_seed(
                            state,
                            level_event_sound_random,
                        ));
                        emit_positioned_sound(&mut audio_events, &state);
                    }
                    emit_level_event_particles(
                        &mut particle_events,
                        &mut particle_renderer,
                        &event,
                        level_event_particle_context(world, &event),
                        level_event_sound_random,
                    );
                }
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
            NetEvent::TagQuery(update) => {
                world.apply_tag_query(update);
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
                let sound_event_registry = sound_event_registry_from_registry_data(&update);
                let jukebox_song_registry = jukebox_song_registry_from_registry_data(&update);
                world.record_registry_data(update);
                if let Some(audio_events) = audio_events.as_mut() {
                    if let Some(registry) = sound_event_registry {
                        audio_events.set_sound_event_registry(registry);
                    }
                    if let Some(registry) = jukebox_song_registry {
                        audio_events.set_jukebox_song_registry(registry);
                    }
                }
            }
            NetEvent::UpdateTags(update) => {
                world.apply_update_tags(update);
            }
            NetEvent::Login(login) => {
                world.apply_login(&login);
            }
            NetEvent::Respawn(respawn) => {
                world.apply_respawn(&respawn);
            }
            NetEvent::PlayerPosition(update) => {
                apply_player_position_update(world, update);
            }
            NetEvent::PlayerRotation(update) => {
                apply_player_rotation_update(world, update);
            }
            NetEvent::PlayerAbilities(abilities) => {
                world.apply_player_abilities(abilities);
            }
            NetEvent::PlayerHealth(health) => {
                world.apply_player_health(health);
            }
            NetEvent::PlayerExperience(experience) => {
                world.apply_player_experience(experience);
            }
            NetEvent::HeldSlot(slot) => {
                world.apply_held_slot(slot);
            }
            NetEvent::SetDefaultSpawnPosition(spawn) => {
                world.apply_default_spawn_position(spawn);
            }
            NetEvent::SetSimulationDistance(distance) => {
                world.apply_simulation_distance(distance);
            }
            NetEvent::SystemChat(chat) => {
                apply_system_chat_update(world, chat);
            }
            NetEvent::SetActionBarText(text) => {
                apply_action_bar_update(world, text);
            }
            NetEvent::SetTitleText(text) => {
                apply_title_text_update(world, text);
            }
            NetEvent::SetSubtitleText(text) => {
                apply_subtitle_text_update(world, text);
            }
            NetEvent::ClearTitles(clear) => {
                apply_clear_titles_update(world, clear);
            }
            NetEvent::SetTitlesAnimation(animation) => {
                apply_titles_animation_update(world, animation);
            }
            NetEvent::TickingState(ticking) => {
                world.apply_ticking_state(ticking);
            }
            NetEvent::TickingStep(step) => {
                world.apply_ticking_step(step);
            }
            NetEvent::SetCamera(camera) => {
                world.apply_set_camera(camera);
            }
            NetEvent::BlockChangedAck(ack) => {
                world.apply_block_changed_ack(ack);
            }
            NetEvent::BlockEntityData(update) => match world.apply_block_entity_data(update) {
                Ok(_) => {}
                Err(_) => {}
            },
            NetEvent::GameEvent(event) => {
                world.apply_game_event(event);
            }
            NetEvent::SetTime(time) => {
                world.apply_world_time(time);
            }
            NetEvent::LevelChunkWithLight(chunk) => {
                match world.insert_level_chunk_with_light(chunk) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
            NetEvent::LightUpdate(update) => match world.apply_light_update(update) {
                Ok(_) => {}
                Err(_) => {}
            },
            NetEvent::ChunksBiomes(update) => match world.apply_biome_update(update) {
                Ok(_) => {}
                Err(_) => {}
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
            NetEvent::Connected
            | NetEvent::Disconnected { .. }
            | NetEvent::StateChanged { .. }
            | NetEvent::CompressionSet { .. }
            | NetEvent::PacketSeen { .. }
            | NetEvent::UnsupportedPacket { .. } => {
                // Runtime/control projection events have no canonical world mutation here.
            }
        }
    }
    drained
}

fn sound_event_registry_from_registry_data(update: &RegistryData) -> Option<SoundEventRegistry> {
    if update.registry != "minecraft:sound_event" || update.entries.is_empty() {
        return None;
    }
    Some(SoundEventRegistry::from_ids(
        update.entries.iter().map(|entry| entry.id.as_str()),
    ))
}

fn jukebox_song_registry_from_registry_data(update: &RegistryData) -> Option<JukeboxSongRegistry> {
    if update.registry != "minecraft:jukebox_song" || update.entries.is_empty() {
        return None;
    }
    Some(JukeboxSongRegistry::from_registry_entry_ids(
        update.entries.iter().map(|entry| entry.id.as_str()),
    ))
}

fn emit_positioned_sound(
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    state: &bbb_world::SoundEventState,
) {
    if let Some(audio_events) = audio_events.as_deref_mut() {
        audio_events.play_positioned_sound(state);
    }
}

fn emit_local_sound(
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    state: &bbb_world::LocalSoundEventState,
) {
    if let Some(audio_events) = audio_events.as_deref_mut() {
        audio_events.play_local_sound(state);
    }
}

fn emit_jukebox_level_event(
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    state: &bbb_world::JukeboxLevelEventState,
) {
    if let Some(audio_events) = audio_events.as_deref_mut() {
        match state.action {
            bbb_world::JukeboxLevelEventAction::Start => audio_events.play_jukebox_song(state),
            bbb_world::JukeboxLevelEventAction::Stop => audio_events.stop_jukebox_song(state),
        }
    }
}

fn queue_chat_acknowledgement(
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    counters: &mut NetCounters,
    command: bbb_protocol::packets::ChatAcknowledgement,
) {
    if let Some(tx) = net_commands {
        if tx
            .try_send(NetCommand::ChatAcknowledgement(command))
            .is_ok()
        {
            counters.chat_acknowledgement_commands_queued += 1;
        }
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
    context: LevelParticleSpawnContext,
) {
    if let Some(particle_events) = particle_events.as_deref_mut() {
        let batch = particle_events.spawn_level_particles(packet, context);
        if let Some(renderer) = particle_renderer.as_deref_mut() {
            renderer.submit_particle_spawns(batch);
        }
    }
}

fn level_particle_spawn_context(world: &WorldStore) -> LevelParticleSpawnContext {
    LevelParticleSpawnContext {
        camera_position: camera_audio_position_from_world(world)
            .map(|position| [position.x, position.y, position.z]),
    }
}

fn emit_level_event_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_renderer: &mut Option<&mut bbb_renderer::Renderer>,
    event: &bbb_protocol::packets::LevelEvent,
    context: LevelEventParticleContext,
    random: &mut LevelEventSoundRandomState,
) -> bool {
    if let Some(particle_events) = particle_events.as_deref_mut() {
        let batch = particle_events.spawn_level_event_particles(event, context, random);
        if let Some(renderer) = particle_renderer.as_deref_mut() {
            renderer.submit_particle_spawns(batch);
        }
        return true;
    }
    false
}

pub(super) fn level_event_particle_context(
    world: &WorldStore,
    event: &bbb_protocol::packets::LevelEvent,
) -> LevelEventParticleContext {
    LevelEventParticleContext {
        sculk_charge_pop_full_block: sculk_charge_pop_full_block_context(world, event),
        block_state_id_at_event_pos: event_pos_block_state_id_context(world, event),
        dripstone_drip_particle: dripstone_drip_particle_context(world, event),
        growth_particles: growth_particle_context(world, event),
        in_block_particle_spread_height: in_block_particle_spread_height_context(world, event),
        composter_fill_center_shape_max_y: composter_fill_center_shape_max_y_context(world, event),
    }
}

fn event_pos_block_state_id_context(
    world: &WorldStore,
    event: &bbb_protocol::packets::LevelEvent,
) -> Option<i32> {
    let pos = bbb_world::BlockPos {
        x: event.pos.x,
        y: event.pos.y,
        z: event.pos.z,
    };
    world.probe_block(pos).map(|probe| probe.block_state_id)
}

fn sculk_charge_pop_full_block_context(
    world: &WorldStore,
    event: &bbb_protocol::packets::LevelEvent,
) -> Option<bool> {
    if event.event_type != 3006 || event.data >> 6 > 0 {
        return None;
    }
    let pos = bbb_world::BlockPos {
        x: event.pos.x,
        y: event.pos.y,
        z: event.pos.z,
    };
    world
        .probe_block(pos)
        .map(|probe| crate::block_outline::block_probe_has_full_block_shape(&probe))
}

fn in_block_particle_spread_height_context(
    world: &WorldStore,
    event: &bbb_protocol::packets::LevelEvent,
) -> Option<f64> {
    match event.event_type {
        BEE_GROWTH_PARTICLES_LEVEL_EVENT | TURTLE_EGG_PLACEMENT_PARTICLES_LEVEL_EVENT => {
            Some(in_block_particle_spread_height(world, event.pos))
        }
        _ => None,
    }
}

fn in_block_particle_spread_height(world: &WorldStore, pos: ProtocolBlockPos) -> f64 {
    world
        .probe_block(protocol_to_world_block_pos(pos))
        .and_then(|probe| crate::block_outline::block_probe_shape_max_y(&probe))
        .unwrap_or(1.0)
}

fn composter_fill_center_shape_max_y_context(
    world: &WorldStore,
    event: &bbb_protocol::packets::LevelEvent,
) -> Option<f64> {
    if event.event_type != COMPOSTER_FILL_LEVEL_EVENT {
        return None;
    }
    Some(
        world
            .probe_block(protocol_to_world_block_pos(event.pos))
            .and_then(|probe| crate::block_outline::block_probe_shape_center_max_y(&probe))
            .unwrap_or(1.0),
    )
}

fn growth_particle_context(
    world: &WorldStore,
    event: &bbb_protocol::packets::LevelEvent,
) -> Option<LevelEventGrowthParticleContext> {
    if event.event_type != PLANT_GROWTH_LEVEL_EVENT || event.data <= 0 {
        return None;
    }
    let event_pos = protocol_to_world_block_pos(event.pos);
    let probe = world.probe_block(event_pos)?;
    let block_name = probe.block_name.as_deref()?;

    if block_name == "minecraft:water" {
        return Some(wide_growth_particle_context(world, event.pos));
    }
    if is_neighbor_spreader_bonemealable_block_name(block_name) {
        return Some(wide_growth_particle_context(
            world,
            protocol_block_pos_relative_y(event.pos, 1)?,
        ));
    }
    if is_below_particle_pos_bonemealable_block_name(block_name) {
        let pos = protocol_block_pos_relative_y(event.pos, -1)?;
        return Some(LevelEventGrowthParticleContext {
            pos,
            mode: LevelEventGrowthParticleMode::InBlock {
                spread_height: in_block_particle_spread_height(world, pos),
            },
        });
    }
    if is_grower_bonemealable_block_name(block_name) {
        return Some(LevelEventGrowthParticleContext {
            pos: event.pos,
            mode: LevelEventGrowthParticleMode::InBlock {
                spread_height: in_block_particle_spread_height(world, event.pos),
            },
        });
    }
    None
}

fn wide_growth_particle_context(
    world: &WorldStore,
    pos: ProtocolBlockPos,
) -> LevelEventGrowthParticleContext {
    LevelEventGrowthParticleContext {
        pos,
        mode: LevelEventGrowthParticleMode::WideNoFloating {
            support: growth_particle_support(world, pos),
        },
    }
}

fn growth_particle_support(
    world: &WorldStore,
    pos: ProtocolBlockPos,
) -> LevelEventGrowthParticleSupport {
    let mut support = LevelEventGrowthParticleSupport::empty();
    let Some(y) = pos.y.checked_sub(1) else {
        return support;
    };
    for dx in -3..=3 {
        for dz in -3..=3 {
            let (Some(x), Some(z)) = (pos.x.checked_add(dx), pos.z.checked_add(dz)) else {
                continue;
            };
            let probe = world.probe_block(bbb_world::BlockPos { x, y, z });
            if probe
                .as_ref()
                .is_some_and(|probe| !block_probe_is_air(probe))
            {
                support.insert(dx, dz);
            }
        }
    }
    support
}

fn is_neighbor_spreader_bonemealable_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:grass_block"
            | "minecraft:netherrack"
            | "minecraft:warped_nylium"
            | "minecraft:crimson_nylium"
            | "minecraft:moss_block"
            | "minecraft:pale_moss_block"
    )
}

fn is_below_particle_pos_bonemealable_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:rooted_dirt" | "minecraft:mangrove_leaves"
    )
}

fn is_grower_bonemealable_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:oak_sapling"
            | "minecraft:spruce_sapling"
            | "minecraft:birch_sapling"
            | "minecraft:jungle_sapling"
            | "minecraft:acacia_sapling"
            | "minecraft:cherry_sapling"
            | "minecraft:dark_oak_sapling"
            | "minecraft:pale_oak_sapling"
            | "minecraft:short_grass"
            | "minecraft:fern"
            | "minecraft:bush"
            | "minecraft:short_dry_grass"
            | "minecraft:tall_dry_grass"
            | "minecraft:seagrass"
            | "minecraft:sea_pickle"
            | "minecraft:wheat"
            | "minecraft:carrots"
            | "minecraft:potatoes"
            | "minecraft:beetroots"
            | "minecraft:pumpkin_stem"
            | "minecraft:melon_stem"
            | "minecraft:cocoa"
            | "minecraft:torchflower_crop"
            | "minecraft:pitcher_crop"
            | "minecraft:bamboo_sapling"
            | "minecraft:bamboo"
            | "minecraft:sweet_berry_bush"
            | "minecraft:warped_fungus"
            | "minecraft:crimson_fungus"
            | "minecraft:azalea"
            | "minecraft:flowering_azalea"
            | "minecraft:pink_petals"
            | "minecraft:wildflowers"
            | "minecraft:big_dripleaf"
            | "minecraft:big_dripleaf_stem"
            | "minecraft:small_dripleaf"
            | "minecraft:pale_moss_carpet"
            | "minecraft:pale_hanging_moss"
            | "minecraft:firefly_bush"
            | "minecraft:hanging_moss"
            | "minecraft:glow_lichen"
            | "minecraft:sunflower"
            | "minecraft:lilac"
            | "minecraft:rose_bush"
            | "minecraft:peony"
            | "minecraft:brown_mushroom"
            | "minecraft:red_mushroom"
            | "minecraft:cave_vines"
            | "minecraft:cave_vines_plant"
            | "minecraft:weeping_vines"
            | "minecraft:weeping_vines_plant"
            | "minecraft:twisting_vines"
            | "minecraft:twisting_vines_plant"
            | "minecraft:kelp"
            | "minecraft:kelp_plant"
    )
}

fn block_probe_is_air(probe: &bbb_world::BlockProbe) -> bool {
    matches!(
        probe.block_name.as_deref(),
        Some("minecraft:air" | "minecraft:cave_air" | "minecraft:void_air")
    )
}

fn protocol_to_world_block_pos(pos: ProtocolBlockPos) -> bbb_world::BlockPos {
    bbb_world::BlockPos {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

fn protocol_block_pos_relative_y(pos: ProtocolBlockPos, dy: i32) -> Option<ProtocolBlockPos> {
    Some(ProtocolBlockPos {
        x: pos.x,
        y: pos.y.checked_add(dy)?,
        z: pos.z,
    })
}

fn dripstone_drip_particle_context(
    world: &WorldStore,
    event: &bbb_protocol::packets::LevelEvent,
) -> Option<LevelEventDripstoneDripParticle> {
    if event.event_type != DRIPSTONE_DRIP_LEVEL_EVENT {
        return None;
    }
    let tip_pos = bbb_world::BlockPos {
        x: event.pos.x,
        y: event.pos.y,
        z: event.pos.z,
    };
    let tip = world.probe_block(tip_pos)?;
    if !block_probe_can_drip(&tip) {
        return None;
    }
    let root_pos = pointed_dripstone_root_pos(world, tip_pos)?;
    let above_root_pos = block_pos_above(root_pos)?;
    let above_root = world.probe_block(above_root_pos)?;

    if above_root.block_name.as_deref() == Some("minecraft:mud") && !level_water_evaporates(world) {
        return Some(LevelEventDripstoneDripParticle::Water);
    }

    match above_root.fluid.map(|fluid| fluid.kind) {
        Some(TerrainFluidKind::Lava) => Some(LevelEventDripstoneDripParticle::Lava),
        Some(TerrainFluidKind::Water) => Some(LevelEventDripstoneDripParticle::Water),
        None => Some(default_dripstone_drip_particle(world)),
    }
}

fn pointed_dripstone_root_pos(
    world: &WorldStore,
    tip_pos: bbb_world::BlockPos,
) -> Option<bbb_world::BlockPos> {
    for step in 1..POINTED_DRIPSTONE_ROOT_SEARCH_LENGTH {
        let pos = bbb_world::BlockPos {
            x: tip_pos.x,
            y: tip_pos.y.checked_add(step)?,
            z: tip_pos.z,
        };
        let probe = world.probe_block(pos)?;
        if probe.block_name.as_deref() != Some("minecraft:pointed_dripstone") {
            return Some(pos);
        }
        if !block_probe_is_down_pointed_dripstone(&probe) {
            return None;
        }
    }
    None
}

fn block_probe_can_drip(probe: &bbb_world::BlockProbe) -> bool {
    block_probe_is_down_pointed_dripstone(probe)
        && probe.block_properties.get("thickness").map(String::as_str) == Some("tip")
        && probe
            .block_properties
            .get("waterlogged")
            .map(String::as_str)
            == Some("false")
}

fn block_probe_is_down_pointed_dripstone(probe: &bbb_world::BlockProbe) -> bool {
    probe.block_name.as_deref() == Some("minecraft:pointed_dripstone")
        && probe
            .block_properties
            .get("vertical_direction")
            .map(String::as_str)
            == Some("down")
}

fn block_pos_above(pos: bbb_world::BlockPos) -> Option<bbb_world::BlockPos> {
    Some(bbb_world::BlockPos {
        x: pos.x,
        y: pos.y.checked_add(1)?,
        z: pos.z,
    })
}

fn default_dripstone_drip_particle(world: &WorldStore) -> LevelEventDripstoneDripParticle {
    if level_water_evaporates(world) {
        LevelEventDripstoneDripParticle::Lava
    } else {
        LevelEventDripstoneDripParticle::Water
    }
}

fn level_water_evaporates(world: &WorldStore) -> bool {
    world.level_info().is_some_and(|level| {
        level.dimension_type_id == 1
            || level.dimension == "minecraft:the_nether"
            || level.dimension_type_name.as_deref() == Some("minecraft:the_nether")
    })
}

fn with_level_event_sound_seed(
    mut state: bbb_world::SoundEventState,
    random: &mut LevelEventSoundRandomState,
) -> bbb_world::SoundEventState {
    state.seed = random.next_long();
    state
}

fn audio_position(position: bbb_world::EntityVec3) -> [f64; 3] {
    [position.x, position.y, position.z]
}

fn camera_audio_position_from_world(world: &WorldStore) -> Option<ProtocolVec3d> {
    let camera = world.local_player().camera;
    if let Some(camera_id) = camera.entity_id {
        if !camera.follows_player {
            if let Some(camera_pose) = world.probe_entity_camera_pose(camera_id) {
                return Some(ProtocolVec3d {
                    x: camera_pose.position.x,
                    y: camera_pose.position.y + f64::from(camera_pose.eye_height),
                    z: camera_pose.position.z,
                });
            }
        }
    }

    world.local_player_pose().map(|pose| ProtocolVec3d {
        x: pose.position.x,
        y: pose.position.y + pose.eye_height(),
        z: pose.position.z,
    })
}
