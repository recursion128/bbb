use crate::control::NetCounters;
use bbb_client_assets::{JukeboxSongRegistry, JukeboxSongRegistryEntry, SoundEventRegistry};
use bbb_client_net::{
    AcceptTeleportCommand, ConnectionState, DisconnectReason, NetCommand, NetEvent,
    PlayerMoveCommand,
};
use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_world::particles::ParticleKind;
use bbb_client_world::{
    AllayDuplicationParticleState, AnimalLoveParticleState, ArrowEffectParticleState,
    BlockPos as WorldBlockPos, ChatDisconnectReason, DolphinHappyParticleState,
    EntityTamingParticleState, EntityTrackingEmitterParticleKind, ExplosionEventState,
    FireworkRocketExplosionParticleState, FoxEatParticleState, HoneyBlockParticleState,
    LevelEventSoundRandomState, LivingEntityDrownParticleState, LivingEntityItemParticleState,
    LivingEntityPoofParticleState, LivingEntityPortalParticleState,
    LlamaSpitRecreationParticleState, NoteBlockParticleState, PlayApplyEffects,
    RabbitSprintParticleState, RavagerRoarParticleState, SnowballHitParticleState,
    TakeItemEntityPickupParticleState, ThrownEggHitParticleState, VillagerParticleState,
    WitchMagicParticleState, WorldStore,
};
use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID;
use bbb_protocol::packets::{
    BlockPos as ProtocolBlockPos, PlayClientbound, Vec3d as ProtocolVec3d,
};
use tokio::sync::mpsc;

use crate::audio_runtime::AudioEventSink;
use crate::input::{
    queue_container_close_request_command, queue_perform_respawn_command,
    queue_vehicle_move_command,
};
use crate::particle_runtime::{
    pointed_dripstone_drip_context, vibration_entity_position, ClientParticle,
    LevelEventGrowthParticleContext, LevelEventGrowthParticleMode, LevelEventGrowthParticleSupport,
    LevelEventParticleContext, LevelParticleEmission, LevelParticleSpawnContext,
    ParticleBiomeSampler, ParticleTarget, TrackingEmitterParticleState,
};
use crate::server_list::ServerListPersistence;

use super::control_state::apply_control_projection_event;
use crate::runtime::{HudDebugNetworkSampler, HudDebugTpsSampler};

const COMPOSTER_FILL_LEVEL_EVENT: i32 = 1500;
const DRIPSTONE_DRIP_LEVEL_EVENT: i32 = 1504;
const PLANT_GROWTH_LEVEL_EVENT: i32 = 1505;
const DESTROY_BLOCK_PARTICLES_LEVEL_EVENT: i32 = 2001;
const BEE_GROWTH_PARTICLES_LEVEL_EVENT: i32 = 2011;
const TURTLE_EGG_PLACEMENT_PARTICLES_LEVEL_EVENT: i32 = 2012;
const SMASH_ATTACK_PARTICLES_LEVEL_EVENT: i32 = 2013;
const BRUSH_BLOCK_COMPLETE_LEVEL_EVENT: i32 = 3008;
const VAULT_ACTIVATE_LEVEL_EVENT: i32 = 3015;
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

/// Keeps the screens the packets open.
#[cfg(test)]
pub(in crate::runtime) fn drain_net_events_with_input(
    input: &mut crate::input::ClientInputState,
    rx: &mut mpsc::Receiver<NetEvent>,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) -> usize {
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);
    drain_net_events_with_sinks_and_debug(
        input,
        rx,
        world,
        counters,
        net_commands,
        NetEventDispatchSinks::default(),
        &mut level_event_sound_random,
    )
}

#[cfg(test)]
pub(in crate::runtime) fn drain_net_events_with_server_list(
    rx: &mut mpsc::Receiver<NetEvent>,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    server_list: &mut ServerListPersistence,
) -> usize {
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);
    drain_net_events_with_sinks_and_debug(
        &mut crate::input::ClientInputState::new(true),
        rx,
        world,
        counters,
        &None,
        NetEventDispatchSinks {
            server_list: Some(server_list),
            ..Default::default()
        },
        &mut level_event_sound_random,
    )
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(in crate::runtime) fn drain_net_events_with_sinks(
    rx: &mut mpsc::Receiver<NetEvent>,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    audio_events: Option<&mut dyn AudioEventSink>,
    particles: Option<ParticleTarget<'_>>,
    item_runtime: Option<&NativeItemRuntime>,
    level_event_sound_random: &mut LevelEventSoundRandomState,
) -> usize {
    drain_net_events_with_sinks_and_debug(
        &mut crate::input::ClientInputState::new(true),
        rx,
        world,
        counters,
        net_commands,
        NetEventDispatchSinks {
            audio_events,
            particles,
            item_runtime,
            ..Default::default()
        },
        level_event_sound_random,
    )
}

#[derive(Default)]
pub(in crate::runtime) struct NetEventDispatchSinks<'a, 'audio, 'particles, 'item> {
    pub(in crate::runtime) audio_events: Option<&'audio mut dyn AudioEventSink>,
    pub(in crate::runtime) particles: Option<ParticleTarget<'particles>>,
    pub(in crate::runtime) item_runtime: Option<&'item NativeItemRuntime>,
    pub(in crate::runtime) hud_debug_network_sampler: Option<&'a mut HudDebugNetworkSampler>,
    pub(in crate::runtime) hud_debug_tps_sampler: Option<&'a mut HudDebugTpsSampler>,
    pub(in crate::runtime) server_list: Option<&'a mut ServerListPersistence>,
    pub(in crate::runtime) remote_disconnect: Option<&'a mut Option<DisconnectReason>>,
}

pub(in crate::runtime) fn drain_net_events_with_sinks_and_debug(
    input: &mut crate::input::ClientInputState,
    rx: &mut mpsc::Receiver<NetEvent>,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    sinks: NetEventDispatchSinks<'_, '_, '_, '_>,
    level_event_sound_random: &mut LevelEventSoundRandomState,
) -> usize {
    let NetEventDispatchSinks {
        mut audio_events,
        mut particles,
        item_runtime,
        mut hud_debug_network_sampler,
        mut hud_debug_tps_sampler,
        mut server_list,
        mut remote_disconnect,
    } = sinks;
    let mut drained = 0;
    let mut pending_resource_pack_prompt_refresh = false;
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
        bbb_client_clock::replay::count(&bbb_client_clock::replay::NET_EVENTS_DRAINED);
        if let NetEvent::Disconnected {
            reason: Some(reason),
        } = &event
        {
            if let Some(output) = remote_disconnect.as_deref_mut() {
                if output.is_none() {
                    *output = Some(reason.clone());
                }
            }
        }

        if matches!(&event, NetEvent::Disconnected { .. }) {
            if let Some(sampler) = hud_debug_network_sampler.as_deref_mut() {
                let (packets, _) = sampler.drain_inbound_metrics();
                counters.packets_seen = counters.packets_seen.saturating_add(packets as usize);
            }
        }
        if let Some(sampler) = hud_debug_network_sampler.as_deref_mut() {
            sampler.record_net_event(&event);
        }
        if let Some(sampler) = hud_debug_tps_sampler.as_deref_mut() {
            sampler.record_net_event(&event);
        }
        apply_control_projection_event(&event, counters);

        match event {
            NetEvent::PlayerPosition { update } => {
                let teleport_id = update.id;
                let mut effects = NativePlayEffects {
                    input,
                    counters,
                    net_commands,
                    audio_events: &mut audio_events,
                    particles: &mut particles,
                    item_runtime,
                    server_list: &mut server_list,
                };
                let _ = world.apply_play_packet(
                    bbb_protocol::packets::PlayClientbound::PlayerPosition(update),
                    level_event_sound_random,
                    &mut effects,
                );
                match world.local_player_pose().map(|pose| pose.position_state()) {
                    Some(state) => queue_connection_answer(
                        net_commands,
                        NetCommand::AcceptTeleport(AcceptTeleportCommand { teleport_id, state }),
                    ),
                    None => tracing::error!(
                        teleport_id,
                        "canonical player pose missing after position correction"
                    ),
                }
            }
            NetEvent::ClearDialog => {
                world.apply_clear_dialog();
            }
            NetEvent::Play(packet) => {
                let transferring = matches!(&packet, PlayClientbound::Transfer(_));
                let player_rotation = matches!(&packet, PlayClientbound::PlayerRotation(_));
                let prev_content_registry = std::sync::Arc::as_ptr(world.content_registry());
                {
                    let mut effects = NativePlayEffects {
                        input,
                        counters,
                        net_commands,
                        audio_events: &mut audio_events,
                        particles: &mut particles,
                        item_runtime,
                        server_list: &mut server_list,
                    };
                    // Other connection-owned leftovers (resource-pack replies)
                    // are already handled by the event stream.
                    let _ = world.apply_play_packet(packet, level_event_sound_random, &mut effects);
                }
                if std::sync::Arc::as_ptr(world.content_registry()) != prev_content_registry {
                    if let Some(audio_events) = audio_events.as_deref_mut() {
                        synchronize_audio_registries_from_world(world, audio_events);
                    }
                }
                if player_rotation {
                    match world.local_player_pose() {
                        Some(pose) => queue_connection_answer(
                            net_commands,
                            NetCommand::AcknowledgePlayerRotation {
                                y_rot: pose.y_rot,
                                x_rot: pose.x_rot,
                            },
                        ),
                        None => tracing::error!(
                            "canonical player pose missing after rotation correction"
                        ),
                    }
                }
                if transferring {
                    apply_transfer_session_teardown(world, &mut audio_events);
                }
            }
            NetEvent::StartConfiguration => {
                for acknowledgement in
                    world.flush_delayed_chat_messages(crate::runtime::monotonic_millis())
                {
                    queue_chat_acknowledgement(net_commands, counters, acknowledgement);
                }
                let command = world.take_pending_player_chat_acknowledgement();
                queue_connection_answer(
                    net_commands,
                    NetCommand::AcknowledgeConfiguration {
                        pending_chat_acknowledgement: command,
                    },
                );
                world.clear_client_level();
                // Vanilla routes play reconfiguration through
                // `Minecraft.clearClientLevel`, whose engine update stops the
                // current sound generation.  Keep this on the explicit
                // StartConfiguration edge: the initial login -> configuration
                // transition has no prior ClientLevel and must not fabricate a
                // backend stop.
                if let Some(audio_events) = audio_events.as_deref_mut() {
                    audio_events.stop_all_sounds();
                }
            }
            NetEvent::StateChanged {
                state: ConnectionState::Login,
            } => {
                // The preceding Connected event atomically installed the new
                // connection session. Play reconfiguration reaches
                // Configuration directly and retains CommonListenerCookie.
            }
            NetEvent::StateChanged {
                state: ConnectionState::Configuration,
            } => {
                if world.level_info().is_some() {
                    world.clear_client_level();
                }
                if let Some(p) = particles.as_mut() {
                    p.engine.clear();
                }
                world.begin_registry_configuration();
            }
            NetEvent::FinishConfiguration => match world.finish_registry_configuration() {
                Ok(()) => {
                    if let Some(audio_events) = audio_events.as_deref_mut() {
                        synchronize_audio_registries_from_world(world, audio_events);
                    }
                    queue_connection_answer(net_commands, NetCommand::FinishConfiguration);
                }
                Err(_) => {
                    queue_connection_answer(net_commands, NetCommand::Disconnect);
                }
            },
            NetEvent::ServicesPublicKeys {
                profile_property_keys,
                player_certificate_keys,
            } => {
                world.replace_services_public_keys(profile_property_keys, player_certificate_keys);
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
                let prev_content_registry = std::sync::Arc::as_ptr(world.content_registry());
                world.apply_custom_payload(update);
                if std::sync::Arc::as_ptr(world.content_registry()) != prev_content_registry {
                    if let Some(audio_events) = audio_events.as_deref_mut() {
                        synchronize_audio_registries_from_world(world, audio_events);
                    }
                }
            }
            NetEvent::ServerLinks(links) => {
                world.apply_server_links(links);
            }
            NetEvent::Transfer(transfer) => {
                world.apply_transfer(transfer);
                if let Some(p) = particles.as_mut() {
                    p.engine.clear();
                }
                apply_transfer_session_teardown(world, &mut audio_events);
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
            NetEvent::UpdateTags(update) => {
                world.apply_update_tags(update);
            }
            NetEvent::ShowDialog(update) => {
                world.apply_show_dialog(update);
            }
            NetEvent::ResourcePackPush(update) => {
                world.apply_resource_pack_push(update);
            }
            NetEvent::ResourcePackPrompt { id } => world.request_resource_pack_consent(id),
            NetEvent::ResourcePackResponse { id, action } => {
                world.apply_resource_pack_response(id, action);
            }
            NetEvent::ResourcePackPop(update) => {
                world.apply_resource_pack_pop(update);
            }
            NetEvent::RegistryData(update) => {
                let registry = update.registry.clone();
                world.record_registry_data(update);
                if let Some(audio_events) = audio_events.as_deref_mut() {
                    synchronize_audio_registry_from_world(world, audio_events, &registry);
                }
            }
            NetEvent::Connected => {
                let keep_resource_packs = world.last_transfer().is_some();
                world.begin_connection_session(keep_resource_packs);
                if let Some(store) = server_list.as_deref() {
                    world.set_resource_pack_preference(store.pack_preference());
                }
            }
            NetEvent::Disconnected { .. } => {}
            NetEvent::StateChanged { .. }
            | NetEvent::CompressionSet { .. }
            | NetEvent::InboundMetrics(_)
            | NetEvent::UnsupportedPacket { .. } => {
                // Runtime/control projection events have no canonical world mutation here.
            }
        }
        let requests = world.screen_requests();
        if !requests.is_empty() {
            if requests.iter().all(|request| {
                matches!(
                    request,
                    bbb_client_world::ScreenRequest::RefreshResourcePackPrompt
                )
            }) {
                let _ = world.take_screen_requests();
                pending_resource_pack_prompt_refresh = true;
            } else {
                if pending_resource_pack_prompt_refresh {
                    input.refresh_resource_pack_prompt(world, item_runtime);
                    pending_resource_pack_prompt_refresh = false;
                }
                let creative_items_changed = requests.iter().any(|request| {
                    matches!(
                        request,
                        bbb_client_world::ScreenRequest::CreativeInventory(_)
                            | bbb_client_world::ScreenRequest::RefreshCreativeInventory
                    )
                });
                input.apply_screen_requests_with_resources(
                    world,
                    counters,
                    net_commands,
                    item_runtime,
                );
                if creative_items_changed {
                    input.refresh_creative_visible_items(world, item_runtime);
                }
            }
        }
    }
    if pending_resource_pack_prompt_refresh {
        input.refresh_resource_pack_prompt(world, item_runtime);
    }
    if let Some(sampler) = hud_debug_network_sampler {
        let (packets, _) = sampler.drain_inbound_metrics();
        counters.packets_seen = counters.packets_seen.saturating_add(packets as usize);
    }
    drained
}

fn apply_transfer_session_teardown(
    world: &mut WorldStore,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
) {
    world.clear_client_level();
    if let Some(audio_events) = audio_events.as_deref_mut() {
        audio_events.stop_all_sounds();
    }
}

/// Queues a connection answer the session cannot proceed without (teleport
/// accept, reconfiguration acknowledgement).  A saturated command channel
/// already means the session is unusable, but the drop must be loud because
/// the server keeps the player frozen until the answer arrives.
fn queue_connection_answer(net_commands: &Option<mpsc::Sender<NetCommand>>, command: NetCommand) {
    let Some(tx) = net_commands else { return };
    if let Err(err) = tx.try_send(command) {
        tracing::error!(
            ?err,
            "dropped connection answer on saturated command channel"
        );
    }
}

/// Runtime sink routing for `WorldStore::apply_play_packet`.
struct NativePlayEffects<'a, 'b, 'c, 'audio, 'particle, 'server> {
    input: &'a mut crate::input::ClientInputState,
    counters: &'a mut NetCounters,
    net_commands: &'b Option<mpsc::Sender<NetCommand>>,
    audio_events: &'c mut Option<&'audio mut dyn AudioEventSink>,
    particles: &'c mut Option<ParticleTarget<'particle>>,
    item_runtime: Option<&'c NativeItemRuntime>,
    server_list: &'c mut Option<&'server mut ServerListPersistence>,
}

impl NativePlayEffects<'_, '_, '_, '_, '_, '_> {
    /// Runs `spawn` against the particle resolver and engine, when present.
    fn particles(&mut self, spawn: impl FnOnce(&mut ParticleTarget<'_>)) {
        if let Some(particles) = self.particles.as_mut() {
            spawn(particles);
        }
    }
}

impl PlayApplyEffects for NativePlayEffects<'_, '_, '_, '_, '_, '_> {
    fn creative_inventory_context(&self) -> Option<bool> {
        self.input.creative_inventory_context()
    }

    fn server_data_updated(&mut self, world: &WorldStore) {
        let Some(server_list) = self.server_list.as_deref_mut() else {
            return;
        };
        if let Err(err) = server_list.persist_current_server_data(world) {
            tracing::error!(?err, "failed to persist server-data update");
        }
    }

    fn positioned_sound(&mut self, state: &bbb_client_world::SoundEventState) {
        emit_positioned_sound(self.audio_events, state);
    }

    fn note_block_particle(&mut self, world: &WorldStore, state: NoteBlockParticleState) {
        let biome_sampler = WorldParticleBiomeSampler { world };
        let item_runtime = self.item_runtime;
        self.particles(|p| {
            p.runtime.spawn_level_particles(
                p.engine,
                &ClientParticle::simple(ParticleKind::NOTE),
                LevelParticleEmission::single(state.position, state.velocity),
                LevelParticleSpawnContext {
                    camera_position: camera_position(world),
                    vibration_entity_position: None,
                },
                Some(&biome_sampler),
                item_runtime,
            )
        });
    }

    fn local_sound(&mut self, state: &bbb_client_world::LocalSoundEventState) {
        emit_local_sound(self.audio_events, state);
    }

    fn entity_sound(
        &mut self,
        state: &bbb_client_world::SoundEntityEventState,
        position: Option<[f64; 3]>,
    ) {
        emit_entity_sound(self.audio_events, state, position);
    }

    fn stop_sound(&mut self, state: &bbb_client_world::StopSoundEventState) {
        emit_stop_sound(self.audio_events, state);
    }

    fn stop_music(&mut self) {
        if let Some(audio_events) = self.audio_events.as_deref_mut() {
            audio_events.stop_music();
        }
    }

    fn stop_all_sounds(&mut self) {
        if let Some(audio_events) = self.audio_events.as_deref_mut() {
            audio_events.stop_all_sounds();
        }
    }

    fn perform_respawn(&mut self) {
        queue_perform_respawn_command(self.counters, self.net_commands);
    }

    fn jukebox_level_event(&mut self, state: &bbb_client_world::JukeboxLevelEventState) {
        emit_jukebox_level_event(self.audio_events, state);
    }

    fn chat_acknowledgement(&mut self, command: bbb_protocol::packets::ChatAcknowledgement) {
        queue_chat_acknowledgement(self.net_commands, self.counters, command);
    }

    fn chat_validation_epoch_millis(&mut self) -> i64 {
        crate::runtime::wall_clock_millis_i64()
    }

    fn chat_monotonic_millis(&mut self) -> u64 {
        crate::runtime::monotonic_millis()
    }

    fn client_monotonic_millis(&mut self) -> u64 {
        crate::runtime::monotonic_millis()
    }

    fn chat_disconnect(&mut self, reason: ChatDisconnectReason) {
        tracing::error!(?reason, "disconnecting after invalid signed-chat stream");
        queue_chat_disconnect(self.net_commands);
    }

    fn container_close(&mut self, container_id: i32) {
        queue_container_close_request_command(self.counters, self.net_commands, container_id);
    }

    fn vehicle_move_report(&mut self, report: bbb_client_world::VehicleMoveReport) {
        queue_vehicle_move_command(self.counters, self.net_commands, report);
    }

    fn removed_player_vehicle_teleport(
        &mut self,
        recovery: bbb_client_world::RemovedPlayerVehicleTeleportRecovery,
    ) {
        let Some(tx) = self.net_commands else {
            return;
        };
        let pose = recovery.player_pose;
        if tx
            .try_send(NetCommand::MovePlayer(PlayerMoveCommand {
                state: pose.position_state(),
                on_ground: false,
                horizontal_collision: false,
                force_position: true,
                force_pos_rot: true,
                force_rotation_only: false,
            }))
            .is_ok()
        {
            self.counters.player_move_commands_queued += 1;
        }
    }

    fn level_particles(
        &mut self,
        world: &WorldStore,
        packet: &bbb_protocol::packets::LevelParticles,
    ) {
        let Some(particle) =
            ClientParticle::from_payload_with_registry(&packet.particle, world.content_registry())
        else {
            return;
        };
        let biome_sampler = WorldParticleBiomeSampler { world };
        let item_runtime = self.item_runtime;
        self.particles(|p| {
            p.runtime.spawn_level_particles(
                p.engine,
                &particle,
                LevelParticleEmission::from_packet(packet),
                level_particle_spawn_context(world, &particle),
                Some(&biome_sampler),
                item_runtime,
            )
        });
    }

    fn explosion_particles(&mut self, world: &WorldStore, state: &ExplosionEventState) {
        let content_registry = world.content_registry();
        let context =
            ClientParticle::from_payload_with_registry(&state.explosion_particle, content_registry)
                .map(|particle| level_particle_spawn_context(world, &particle))
                .unwrap_or_default();
        let biome_sampler = WorldParticleBiomeSampler { world };
        let item_runtime = self.item_runtime;
        self.particles(|p| {
            p.runtime.spawn_explosion_particles(
                p.engine,
                state,
                world.client_level_generation(),
                content_registry,
                context,
                Some(&biome_sampler),
                item_runtime,
            )
        });
    }

    fn firework_empty_explosion_particles(&mut self, world: &WorldStore, position: [f64; 3]) {
        self.particles(|p| {
            p.runtime.spawn_firework_empty_explosion_particles(
                p.engine,
                position,
                camera_position(world),
            )
        });
    }

    fn firework_explosion_particles(
        &mut self,
        world: &WorldStore,
        state: &FireworkRocketExplosionParticleState,
    ) {
        self.particles(|p| {
            p.runtime
                .spawn_firework_explosion_particles(p.engine, state, camera_position(world))
        });
    }

    fn llama_spit_recreation_particles(
        &mut self,
        _world: &WorldStore,
        state: LlamaSpitRecreationParticleState,
    ) {
        self.particles(|p| {
            p.runtime
                .spawn_llama_spit_recreation_particles(p.engine, state)
        });
    }

    fn elder_guardian_effect_particles(&mut self, world: &WorldStore, position: ProtocolVec3d) {
        self.particles(|p| {
            p.runtime.spawn_level_particles(
                p.engine,
                &ClientParticle::simple(ParticleKind::ELDER_GUARDIAN),
                LevelParticleEmission::single(position, ProtocolVec3d::default()),
                LevelParticleSpawnContext {
                    camera_position: camera_position(world),
                    vibration_entity_position: None,
                },
                None,
                None,
            )
        });
    }

    fn tracking_emitter_particles(
        &mut self,
        world: &WorldStore,
        entity_id: i32,
        kind: EntityTrackingEmitterParticleKind,
        lifetime_ticks: u32,
    ) {
        let Some(transform) = world.probe_entity_transform(entity_id) else {
            return;
        };
        let Some(bounds) = world.probe_entity_pick_bounds(entity_id) else {
            return;
        };
        let state = TrackingEmitterParticleState {
            kind: match kind {
                EntityTrackingEmitterParticleKind::Crit => ParticleKind::CRIT,
                EntityTrackingEmitterParticleKind::EnchantedHit => ParticleKind::ENCHANTED_HIT,
                EntityTrackingEmitterParticleKind::TotemOfUndying => ParticleKind::TOTEM_OF_UNDYING,
            },
            position: [
                transform.position.x,
                transform.position.y,
                transform.position.z,
            ],
            width: bounds.max[0] - bounds.min[0],
            height: bounds.max[1] - bounds.min[1],
            lifetime_ticks,
        };
        self.particles(|p| p.runtime.spawn_tracking_emitter_particles(p.engine, state));
    }

    fn take_item_entity_pickup_particles(
        &mut self,
        _world: &WorldStore,
        state: &TakeItemEntityPickupParticleState,
    ) {
        self.particles(|p| p.engine.spawn_item_pickup(state));
    }

    fn ravager_roar_particles(&mut self, _world: &WorldStore, state: RavagerRoarParticleState) {
        self.particles(|p| p.runtime.spawn_ravager_roar_particles(p.engine, state));
    }

    fn witch_magic_particles(&mut self, _world: &WorldStore, state: WitchMagicParticleState) {
        self.particles(|p| p.runtime.spawn_witch_magic_particles(p.engine, state));
    }

    fn living_entity_poof_particles(
        &mut self,
        _world: &WorldStore,
        state: LivingEntityPoofParticleState,
    ) {
        self.particles(|p| {
            p.runtime
                .spawn_living_entity_poof_particles(p.engine, state)
        });
    }

    fn living_entity_item_particles(
        &mut self,
        _world: &WorldStore,
        state: LivingEntityItemParticleState,
    ) {
        let item_runtime = self.item_runtime;
        self.particles(|p| {
            p.runtime
                .spawn_living_entity_item_particles(p.engine, state, item_runtime)
        });
    }

    fn living_entity_drown_particles(
        &mut self,
        _world: &WorldStore,
        state: LivingEntityDrownParticleState,
    ) {
        self.particles(|p| {
            p.runtime
                .spawn_living_entity_drown_particles(p.engine, state)
        });
    }

    fn living_entity_portal_particles(
        &mut self,
        _world: &WorldStore,
        state: LivingEntityPortalParticleState,
    ) {
        self.particles(|p| {
            p.runtime
                .spawn_living_entity_portal_particles(p.engine, state)
        });
    }

    fn arrow_effect_particles(&mut self, _world: &WorldStore, state: ArrowEffectParticleState) {
        self.particles(|p| p.runtime.spawn_arrow_effect_particles(p.engine, state));
    }

    fn entity_taming_particles(&mut self, _world: &WorldStore, state: EntityTamingParticleState) {
        self.particles(|p| p.runtime.spawn_entity_taming_particles(p.engine, state));
    }

    fn villager_particles(&mut self, _world: &WorldStore, state: VillagerParticleState) {
        self.particles(|p| p.runtime.spawn_villager_particles(p.engine, state));
    }

    fn dolphin_happy_particles(&mut self, _world: &WorldStore, state: DolphinHappyParticleState) {
        self.particles(|p| p.runtime.spawn_dolphin_happy_particles(p.engine, state));
    }

    fn fox_eat_particles(&mut self, _world: &WorldStore, state: FoxEatParticleState) {
        let item_runtime = self.item_runtime;
        self.particles(|p| {
            p.runtime
                .spawn_fox_eat_particles(p.engine, state, item_runtime)
        });
    }

    fn animal_love_particles(&mut self, _world: &WorldStore, state: AnimalLoveParticleState) {
        self.particles(|p| p.runtime.spawn_animal_love_particles(p.engine, state));
    }

    fn allay_duplication_particles(
        &mut self,
        _world: &WorldStore,
        state: AllayDuplicationParticleState,
    ) {
        self.particles(|p| p.runtime.spawn_allay_duplication_particles(p.engine, state));
    }

    fn snowball_hit_particles(&mut self, _world: &WorldStore, state: SnowballHitParticleState) {
        let item_runtime = self.item_runtime;
        self.particles(|p| {
            p.runtime
                .spawn_snowball_hit_particles(p.engine, state, item_runtime)
        });
    }

    fn thrown_egg_hit_particles(&mut self, _world: &WorldStore, state: ThrownEggHitParticleState) {
        let item_runtime = self.item_runtime;
        self.particles(|p| {
            p.runtime
                .spawn_thrown_egg_hit_particles(p.engine, state, item_runtime)
        });
    }

    fn honey_block_particles(&mut self, _world: &WorldStore, state: HoneyBlockParticleState) {
        self.particles(|p| p.runtime.spawn_honey_block_particles(p.engine, state));
    }

    fn rabbit_sprint_particles(&mut self, _world: &WorldStore, state: RabbitSprintParticleState) {
        self.particles(|p| p.runtime.spawn_rabbit_sprint_particles(p.engine, state));
    }

    fn level_event_particles(
        &mut self,
        world: &WorldStore,
        event: &bbb_protocol::packets::LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) {
        if let Some(p) = self.particles.as_mut() {
            let biome_sampler = WorldParticleBiomeSampler { world };
            p.runtime.spawn_level_event_particles(
                p.engine,
                event,
                level_event_particle_context(world, event),
                random,
                Some(&biome_sampler),
                self.item_runtime,
            );
        }
    }
}

fn sound_event_registry_from_world(world: &WorldStore) -> SoundEventRegistry {
    world
        .registry_resource_ids_in_holder_order("minecraft:sound_event")
        .map(SoundEventRegistry::from_ids)
        .unwrap_or_else(|| SoundEventRegistry::with_content_registry(world.content_registry()))
}

fn jukebox_song_registry_from_world(world: &WorldStore) -> Option<JukeboxSongRegistry> {
    let resource_ids = world
        .registry_resource_ids_in_holder_order("minecraft:jukebox_song")?
        .map(str::to_string)
        .collect::<Vec<_>>();
    Some(JukeboxSongRegistry::from_entries(
        resource_ids
            .into_iter()
            .enumerate()
            .map(|(holder_id, song_id)| {
                let projection = i32::try_from(holder_id)
                    .ok()
                    .and_then(|holder_id| world.synchronized_jukebox_song_presentation(holder_id));
                if let Some(projection) = projection {
                    return JukeboxSongRegistryEntry::new(
                        song_id,
                        Some(projection.sound_event.resource_id()),
                        projection.sound_event.fixed_range(),
                    );
                }
                let fallback = world
                    .registry_entry_by_holder_id(
                        "minecraft:jukebox_song",
                        i32::try_from(holder_id).unwrap_or(i32::MAX),
                    )
                    .filter(|entry| !entry.has_data)
                    .and_then(|_| JukeboxSongRegistry::vanilla_26_1_sound_event_id(&song_id));
                JukeboxSongRegistryEntry::new(song_id, fallback, None)
            }),
    ))
}

fn synchronize_audio_registry_from_world(
    world: &WorldStore,
    audio_events: &mut dyn AudioEventSink,
    registry: &str,
) {
    match registry {
        "minecraft:sound_event" => {
            audio_events.set_sound_event_registry(sound_event_registry_from_world(world));
        }
        "minecraft:jukebox_song" => {
            if let Some(registry) = jukebox_song_registry_from_world(world) {
                audio_events.set_jukebox_song_registry(registry);
            }
        }
        _ => {}
    }
}

pub(crate) fn synchronize_audio_registries_from_world(
    world: &WorldStore,
    audio_events: &mut dyn AudioEventSink,
) {
    audio_events.set_sound_event_registry(sound_event_registry_from_world(world));
    audio_events
        .set_jukebox_song_registry(jukebox_song_registry_from_world(world).unwrap_or_default());
}

pub(crate) fn audio_registry_snapshot_from_world(
    world: &WorldStore,
) -> (SoundEventRegistry, JukeboxSongRegistry) {
    (
        sound_event_registry_from_world(world),
        jukebox_song_registry_from_world(world).unwrap_or_default(),
    )
}

fn emit_positioned_sound(
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    state: &bbb_client_world::SoundEventState,
) {
    if let Some(audio_events) = audio_events.as_deref_mut() {
        audio_events.play_positioned_sound(state);
    }
}

fn emit_local_sound(
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    state: &bbb_client_world::LocalSoundEventState,
) {
    if let Some(audio_events) = audio_events.as_deref_mut() {
        audio_events.play_local_sound(state);
    }
}

fn emit_jukebox_level_event(
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    state: &bbb_client_world::JukeboxLevelEventState,
) {
    if let Some(audio_events) = audio_events.as_deref_mut() {
        match state.action {
            bbb_client_world::JukeboxLevelEventAction::Start => {
                audio_events.play_jukebox_song(state)
            }
            bbb_client_world::JukeboxLevelEventAction::Stop => {
                audio_events.stop_jukebox_song(state)
            }
        }
    }
}

pub(crate) fn queue_chat_acknowledgement(
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

fn queue_chat_disconnect(net_commands: &Option<mpsc::Sender<NetCommand>>) {
    let Some(tx) = net_commands else { return };
    if let Err(err) = tx.try_send(NetCommand::Disconnect) {
        tracing::error!(
            ?err,
            "dropped required disconnect after invalid signed-chat stream"
        );
    }
}

fn emit_entity_sound(
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    state: &bbb_client_world::SoundEntityEventState,
    position: Option<[f64; 3]>,
) {
    if let Some(audio_events) = audio_events.as_deref_mut() {
        audio_events.play_entity_sound(state, position);
    }
}

fn emit_stop_sound(
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    state: &bbb_client_world::StopSoundEventState,
) {
    if let Some(audio_events) = audio_events.as_deref_mut() {
        audio_events.stop_sound(state);
    }
}

/// Dispatches a client-local `LevelEvent` (e.g., locally predicted block
/// break `2001`) through the same positioned-sound and particle pipelines used
/// for server-sent level events.
pub(in crate::runtime) fn dispatch_local_level_event(
    world: &mut WorldStore,
    event: &bbb_protocol::packets::LevelEvent,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    particles: Option<ParticleTarget<'_>>,
    item_runtime: Option<&NativeItemRuntime>,
    random: &mut LevelEventSoundRandomState,
) {
    if let Some(sound) = world.level_event_sound_with_random(*event, || random.next_float()) {
        let sound = world.record_positioned_sound(sound);
        emit_positioned_sound(audio_events, &sound);
    }
    if let Some(p) = particles {
        let biome_sampler = WorldParticleBiomeSampler { world };
        p.runtime.spawn_level_event_particles(
            p.engine,
            event,
            level_event_particle_context(world, event),
            random,
            Some(&biome_sampler),
            item_runtime,
        );
    }
}

pub(in crate::runtime) fn dispatch_pending_client_particle_spawns(
    world: &mut WorldStore,
    particle_runtime: Option<&crate::particle_runtime::NativeParticleRuntime>,
    particles: &mut bbb_client_world::particles::ParticleEngine,
    item_runtime: Option<&NativeItemRuntime>,
) {
    let spawns = world.take_pending_client_particle_spawns();
    if spawns.is_empty() {
        return;
    }
    let biome_sampler = WorldParticleBiomeSampler { world };
    for spawn in spawns {
        if let Some(runtime) = particle_runtime {
            runtime.spawn_client_particle_spawn(
                particles,
                spawn,
                Some(&biome_sampler),
                item_runtime,
            );
        } else {
            particles.spawn(spawn);
        }
    }
}

struct WorldParticleBiomeSampler<'a> {
    world: &'a WorldStore,
}

impl ParticleBiomeSampler for WorldParticleBiomeSampler<'_> {
    fn biome_id_at(&self, pos: WorldBlockPos) -> Option<i32> {
        self.world.biome_id_at(pos)
    }
}

fn camera_position(world: &WorldStore) -> Option<[f64; 3]> {
    camera_audio_position_from_world(world).map(|position| [position.x, position.y, position.z])
}

fn level_particle_spawn_context(
    world: &WorldStore,
    particle: &ClientParticle,
) -> LevelParticleSpawnContext {
    LevelParticleSpawnContext {
        camera_position: camera_position(world),
        vibration_entity_position: vibration_entity_position(world, particle),
    }
}

pub(super) fn level_event_particle_context(
    world: &WorldStore,
    event: &bbb_protocol::packets::LevelEvent,
) -> LevelEventParticleContext {
    LevelEventParticleContext {
        sculk_charge_pop_full_block: sculk_charge_pop_full_block_context(world, event),
        block_state_id_at_event_pos: event_pos_block_state_id_context(world, event),
        biome_id_at_event_pos: event_pos_biome_id_context(world, event),
        vault_block_entity_at_event_pos: vault_block_entity_at_event_pos_context(world, event),
        vault_connection_particles: vault_connection_particle_context(world, event),
        dripstone_drip_particle: dripstone_drip_particle_context(world, event),
        growth_particles: growth_particle_context(world, event),
        in_block_particle_spread_height: in_block_particle_spread_height_context(world, event),
        composter_fill_center_shape_max_y: composter_fill_center_shape_max_y_context(world, event),
    }
}

fn event_pos_biome_id_context(
    world: &WorldStore,
    event: &bbb_protocol::packets::LevelEvent,
) -> Option<i32> {
    if !matches!(
        event.event_type,
        DESTROY_BLOCK_PARTICLES_LEVEL_EVENT | BRUSH_BLOCK_COMPLETE_LEVEL_EVENT
    ) {
        return None;
    }
    world.biome_id_at(event.pos)
}

fn vault_block_entity_at_event_pos_context(
    world: &WorldStore,
    event: &bbb_protocol::packets::LevelEvent,
) -> bool {
    event.event_type == VAULT_ACTIVATE_LEVEL_EVENT
        && world.block_entity_type_id_at(event.pos) == Some(VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID)
}

fn vault_connection_particle_context(
    world: &WorldStore,
    event: &bbb_protocol::packets::LevelEvent,
) -> Option<bbb_client_world::VaultConnectionParticleState> {
    (event.event_type == VAULT_ACTIVATE_LEVEL_EVENT)
        .then(|| world.vault_connection_particle_state(event.pos))
        .flatten()
}

fn event_pos_block_state_id_context(
    world: &WorldStore,
    event: &bbb_protocol::packets::LevelEvent,
) -> Option<i32> {
    if event.event_type != SMASH_ATTACK_PARTICLES_LEVEL_EVENT {
        return None;
    }
    world.block_state_id_at(event.pos)
}

fn block_view_outline_boxes(
    view: &bbb_client_world::BlockView,
) -> Option<Vec<([f64; 3], [f64; 3])>> {
    let state = bbb_client_world::BlockStateRegistry::vanilla_26_1().by_id(view.block_state_id)?;
    bbb_client_world::block_outline::block_state_shape_boxes_at(state, view.pos)
}

fn sculk_charge_pop_full_block_context(
    world: &WorldStore,
    event: &bbb_protocol::packets::LevelEvent,
) -> Option<bool> {
    if event.event_type != 3006 || event.data >> 6 > 0 {
        return None;
    }
    world.block_view(event.pos).map(|view| {
        if matches!(
            view.material,
            bbb_client_world::TerrainMaterialClass::Empty
                | bbb_client_world::TerrainMaterialClass::Fluid
        ) {
            return false;
        }
        block_view_outline_boxes(&view)
            .is_some_and(|boxes| boxes.len() == 1 && boxes[0] == ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]))
    })
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
        .block_view(pos)
        .and_then(|view| {
            if matches!(view.material, bbb_client_world::TerrainMaterialClass::Empty) {
                return Some(1.0);
            }
            block_view_outline_boxes(&view)
                .map(|boxes| boxes.iter().map(|(_, max)| max[1]).fold(0.0, f64::max))
        })
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
            .block_view(event.pos)
            .and_then(|view| {
                block_view_outline_boxes(&view).map(|boxes| {
                    boxes
                        .iter()
                        .filter(|(min, max)| {
                            0.5 >= min[0] && 0.5 <= max[0] && 0.5 >= min[2] && 0.5 <= max[2]
                        })
                        .map(|(_, max)| max[1])
                        .fold(0.0, f64::max)
                })
            })
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
    let view = world.block_view(event.pos)?;
    let block_name = view.block_name?;

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
            if world
                .block_view(bbb_client_world::BlockPos { x, y, z })
                .is_some_and(|view| !block_view_is_air(&view))
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

fn block_view_is_air(view: &bbb_client_world::BlockView) -> bool {
    view.block_name
        .is_some_and(bbb_client_world::block_name_is_air)
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
) -> Option<ClientParticle> {
    if event.event_type != DRIPSTONE_DRIP_LEVEL_EVENT {
        return None;
    }
    pointed_dripstone_drip_context(world, event.pos).and_then(|context| context.particle)
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
        y: pose.position.y + world.local_player_eye_height(pose),
        z: pose.position.z,
    })
}

#[cfg(test)]
mod terminal_tests {
    use super::*;

    #[test]
    fn remote_disconnect_outcome_observes_reason_in_fifo_order() {
        let (tx, mut rx) = mpsc::channel(4);
        tx.try_send(NetEvent::Connected).unwrap();
        let reason = DisconnectReason::from_plain("server closed");
        tx.try_send(NetEvent::Disconnected {
            reason: Some(reason.clone()),
        })
        .unwrap();
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut remote_disconnect = None;

        let drained = drain_net_events_with_sinks_and_debug(
            &mut crate::input::ClientInputState::new(true),
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            NetEventDispatchSinks {
                remote_disconnect: Some(&mut remote_disconnect),
                ..Default::default()
            },
            &mut random,
        );

        assert_eq!(drained, 2);
        assert_eq!(remote_disconnect, Some(reason));
        assert!(!counters.connected);
    }

    fn notice() -> bbb_protocol::packets::ShowDialog {
        use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
        bbb_protocol::packets::ShowDialog {
            dialog: bbb_protocol::packets::DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "type".into(),
                        value: NbtSummaryValue::String("minecraft:notice".into()),
                    },
                    NbtSummaryEntry {
                        name: "title".into(),
                        value: NbtSummaryValue::String("Notice".into()),
                    },
                ]),
            },
        }
    }

    fn drain_with_input(
        input: &mut crate::input::ClientInputState,
        rx: &mut mpsc::Receiver<NetEvent>,
        world: &mut WorldStore,
        counters: &mut NetCounters,
    ) -> usize {
        drain_net_events_with_sinks_and_debug(
            input,
            rx,
            world,
            counters,
            &None,
            NetEventDispatchSinks::default(),
            &mut LevelEventSoundRandomState::with_seed(0),
        )
    }

    #[test]
    fn command_confirmation_ignores_configuration_and_play_clear_dialog() {
        let mut input = crate::input::ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.apply_show_dialog(notice());
        input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
        let revision = input.dialog_screen_revision();
        let mut counters = NetCounters::default();
        let (command_tx, mut commands_rx) = mpsc::channel(4);
        let commands = Some(command_tx);
        input.start_dialog_unattended_command(
            &"/seed".into(),
            "close",
            &mut world,
            &mut counters,
            &commands,
        );
        let (tx, mut rx) = mpsc::channel(4);
        tx.try_send(NetEvent::ClearDialog).unwrap();
        tx.try_send(NetEvent::Play(PlayClientbound::ClearDialog))
            .unwrap();
        assert_eq!(
            drain_with_input(&mut input, &mut rx, &mut world, &mut counters),
            2
        );
        assert!(input.command_confirmation().is_some());
        assert!(input.current_dialog().is_some());
        assert_eq!(input.dialog_screen_revision(), revision);
        assert_eq!(world.counters().clear_dialog_packets, 2);
        assert!(commands_rx.try_recv().is_err());
        input.resolve_command_confirmation(
            true,
            &mut world,
            &mut counters,
            &commands,
            &crate::input::FallbackTextMeasurer,
        );
        assert!(input.current_dialog().is_none());
        assert!(
            matches!(commands_rx.try_recv(), Ok(NetCommand::ChatCommand(packet)) if packet.signing.is_none())
        );
        world.apply_show_dialog(notice());
        input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
        tx.try_send(NetEvent::Play(PlayClientbound::ClearDialog))
            .unwrap();
        drain_with_input(&mut input, &mut rx, &mut world, &mut counters);
        assert!(input.current_dialog().is_none());
        assert_eq!(world.counters().clear_dialog_packets, 3);
    }

    #[test]
    fn command_confirmation_invalidation_precedes_later_clear_in_same_event_batch() {
        let mut input = crate::input::ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.apply_show_dialog(notice());
        input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
        let mut counters = NetCounters::default();
        input.start_dialog_unattended_command(
            &"/seed".into(),
            "close",
            &mut world,
            &mut counters,
            &None,
        );
        let (tx, mut rx) = mpsc::channel(4);
        tx.try_send(NetEvent::Play(PlayClientbound::OpenScreen(
            bbb_protocol::packets::OpenScreen {
                container_id: 2,
                menu_type_id: 0,
                title: bbb_protocol::RichTextComponent::plain("Chest"),
            },
        )))
        .unwrap();
        tx.try_send(NetEvent::ClearDialog).unwrap();
        assert_eq!(
            drain_with_input(&mut input, &mut rx, &mut world, &mut counters),
            2
        );
        assert!(input.command_confirmation().is_none());
        assert_eq!(world.open_container_id(), Some(2));
        assert_eq!(world.counters().clear_dialog_packets, 1);
    }

    #[test]
    fn command_confirmation_show_dialog_nested_wait_restores_original_form() {
        use bbb_protocol::{NbtSummaryEntry as Entry, NbtSummaryValue as Value};
        let mut original = notice();
        if let bbb_protocol::packets::DialogHolder::Direct {
            dialog: Value::Compound(fields),
        } = &mut original.dialog
        {
            fields.push(Entry {
                name: "inputs".into(),
                value: Value::List(vec![Value::Compound(vec![
                    Entry {
                        name: "key".into(),
                        value: Value::String("enabled".into()),
                    },
                    Entry {
                        name: "type".into(),
                        value: Value::String("minecraft:boolean".into()),
                    },
                    Entry {
                        name: "label".into(),
                        value: Value::String("Enabled".into()),
                    },
                    Entry {
                        name: "initial".into(),
                        value: Value::Byte(1),
                    },
                ])]),
            });
        }
        let mut world = WorldStore::new();
        let mut input = crate::input::ClientInputState::new(true);
        world.apply_show_dialog(original);
        input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
        assert!(input.toggle_dialog_boolean_input(0));
        let values = input.current_dialog_input_values().to_vec();
        let mut counters = NetCounters::default();
        let (command_tx, mut command_rx) = mpsc::channel(4);
        let commands = Some(command_tx);
        input.start_dialog_unattended_command(
            &"/seed".into(),
            "close",
            &mut world,
            &mut counters,
            &commands,
        );
        let first = input.command_confirmation().unwrap().generation;
        let (tx, mut rx) = mpsc::channel(8);
        tx.try_send(NetEvent::ShowDialog(notice())).unwrap();
        drain_with_input(&mut input, &mut rx, &mut world, &mut counters);
        assert!(input.command_confirmation().is_none());

        assert!(input.current_dialog_input_values().is_empty());
        input.start_dialog_unattended_command(
            &"/missing".into(),
            "none",
            &mut world,
            &mut counters,
            &commands,
        );
        let second = input.command_confirmation().unwrap().generation;
        tx.try_send(NetEvent::Play(PlayClientbound::ShowDialog(notice())))
            .unwrap();
        tx.try_send(NetEvent::Play(PlayClientbound::ClearDialog))
            .unwrap();
        tx.try_send(NetEvent::ClearDialog).unwrap();
        drain_with_input(&mut input, &mut rx, &mut world, &mut counters);
        assert_eq!(input.command_confirmation().unwrap().generation, second);
        input.resolve_command_confirmation(
            false,
            &mut world,
            &mut counters,
            &commands,
            &crate::input::FallbackTextMeasurer,
        );
        assert!(input.wait_for_dialog_response());
        input.advance_dialog_waiting_ticks(101);
        assert!(input.close_dialog_waiting_response());
        input.apply_screen_requests(&mut world, &mut counters, &None);
        assert_eq!(input.command_confirmation().unwrap().generation, first);
        assert_eq!(input.current_dialog_input_values(), values);
        assert!(command_rx.try_recv().is_err());
        input.resolve_command_confirmation(
            true,
            &mut world,
            &mut counters,
            &commands,
            &crate::input::FallbackTextMeasurer,
        );
        assert!(input.current_dialog().is_none());
        assert!(
            matches!(command_rx.try_recv(), Ok(NetCommand::ChatCommand(packet)) if packet.command == "seed")
        );
    }

    #[test]
    fn command_confirmation_show_clear_show_fifo_and_container_replacement() {
        let mut input = crate::input::ClientInputState::new(true);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();
        input.start_unattended_command(&"/seed".into(), &mut world, &mut counters, &None);
        let (tx, mut rx) = mpsc::channel(8);
        tx.try_send(NetEvent::ShowDialog(notice())).unwrap();
        tx.try_send(NetEvent::ClearDialog).unwrap();
        tx.try_send(NetEvent::Play(PlayClientbound::ShowDialog(notice())))
            .unwrap();
        assert_eq!(
            drain_with_input(&mut input, &mut rx, &mut world, &mut counters),
            3
        );
        assert!(input.command_confirmation().is_none());

        tx.try_send(NetEvent::Play(PlayClientbound::OpenScreen(
            bbb_protocol::packets::OpenScreen {
                container_id: 3,
                menu_type_id: 0,
                title: bbb_protocol::RichTextComponent::plain("Chest"),
            },
        )))
        .unwrap();
        drain_with_input(&mut input, &mut rx, &mut world, &mut counters);

        assert!(input.command_confirmation().is_none());
        assert_eq!(world.open_container_id(), Some(3));
    }

    #[test]
    fn command_confirmation_link_parent_survives_nested_dialog_wait_return() {
        let mut input = crate::input::ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.apply_show_dialog(notice());
        input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
        assert!(input.open_dialog_url("https://example.invalid".into(), "none".into()));
        world.apply_show_dialog(notice());
        input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
        let mut counters = NetCounters::default();
        input.start_dialog_unattended_command(
            &"/seed".into(),
            "none",
            &mut world,
            &mut counters,
            &None,
        );
        let token = input.command_confirmation().unwrap().generation;
        let (tx, mut rx) = mpsc::channel(4);
        tx.try_send(NetEvent::ShowDialog(notice())).unwrap();
        drain_with_input(&mut input, &mut rx, &mut world, &mut counters);
        assert!(input.wait_for_dialog_response());
        input.advance_dialog_waiting_ticks(101);
        assert!(input.close_dialog_waiting_response());
        input.apply_screen_requests(&mut world, &mut counters, &None);
        assert_eq!(input.command_confirmation().unwrap().generation, token);
        input.resolve_command_confirmation(
            false,
            &mut world,
            &mut counters,
            &None,
            &crate::input::FallbackTextMeasurer,
        );
        assert!(input.close_dialog());
        assert_eq!(
            input.dialog_link_confirmation().unwrap().url,
            "https://example.invalid"
        );
        assert!(input.current_dialog().is_some());
    }
}
