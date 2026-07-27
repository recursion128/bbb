use bbb_control::NetCounters;
use bbb_item_model::NativeItemRuntime;
use bbb_net::{AcceptTeleportCommand, ConnectionState, NetCommand, NetEvent, PlayerMoveCommand};
use bbb_pack::{JukeboxSongRegistry, JukeboxSongRegistryEntry, SoundEventRegistry};
use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID;
use bbb_protocol::packets::{
    BlockPos as ProtocolBlockPos, LevelParticles, ParticlePayload, Vec3d as ProtocolVec3d,
};
use bbb_protocol::particle_types::{
    VANILLA_PARTICLE_TYPE_CRIT_ID, VANILLA_PARTICLE_TYPE_ELDER_GUARDIAN_ID,
    VANILLA_PARTICLE_TYPE_ENCHANTED_HIT_ID, VANILLA_PARTICLE_TYPE_NOTE_ID,
    VANILLA_PARTICLE_TYPE_TOTEM_OF_UNDYING_ID,
};
use bbb_world::{
    AllayDuplicationParticleState, AnimalLoveParticleState, ArrowEffectParticleState,
    BlockPos as WorldBlockPos, ChatDisconnectReason, ClientParticleOptions,
    DolphinHappyParticleState, EntityTamingParticleState, EntityTrackingEmitterParticleKind,
    ExplosionEventState, FireworkRocketExplosionParticleState, FoxEatParticleState,
    HoneyBlockParticleState, LevelEventGrowthRandomMode, LevelEventSoundRandomState,
    LivingEntityDrownParticleState, LivingEntityItemBreakState, LivingEntityPoofParticleState,
    LivingEntityPortalParticleState, LlamaSpitRecreationParticleState, NoteBlockParticleState,
    PlayApplyEffects, RabbitSprintParticleState, RavagerRoarParticleState,
    SnowballHitParticleState, TakeItemEntityPickupParticleState, TerrainFluidKind,
    ThrownEggHitParticleState, VillagerParticleState, WitchMagicParticleState, WorldStore,
};
use tokio::sync::mpsc;

use crate::audio_runtime::AudioEventSink;
use crate::input::{
    queue_container_close_request_command, queue_perform_respawn_command,
    queue_vehicle_move_command,
};
use crate::particle_runtime::{
    simple_client_particle_options, vibration_entity_position_source_from_options,
    LevelEventGrowthParticleContext, LevelEventGrowthParticleMode, LevelEventGrowthParticleSupport,
    LevelEventParticleContext, LevelParticleEntityPosition, LevelParticleSpawnContext,
    ParticleBiomeSampler, ParticleEventSink, TrackingEmitterParticleState,
};
use crate::simulation::effects::ParticleBatchSink;

use super::control_state::apply_control_projection_event;
use crate::runtime::{HudDebugNetworkSampler, HudDebugTpsSampler};

const COMPOSTER_FILL_LEVEL_EVENT: i32 = 1500;
const DRIPSTONE_DRIP_LEVEL_EVENT: i32 = 1504;
const PLANT_GROWTH_LEVEL_EVENT: i32 = 1505;
const BEE_GROWTH_PARTICLES_LEVEL_EVENT: i32 = 2011;
const TURTLE_EGG_PLACEMENT_PARTICLES_LEVEL_EVENT: i32 = 2012;
const VAULT_ACTIVATE_LEVEL_EVENT: i32 = 3015;
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
        None,
        &mut level_event_sound_random,
    )
}

#[cfg(test)]
pub(in crate::runtime) fn drain_net_events_with_sinks(
    rx: &mut mpsc::Receiver<NetEvent>,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    audio_events: Option<&mut dyn AudioEventSink>,
    particle_events: Option<&mut dyn ParticleEventSink>,
    particle_target: Option<&mut bbb_renderer::Renderer>,
    item_runtime: Option<&NativeItemRuntime>,
    level_event_sound_random: &mut LevelEventSoundRandomState,
) -> usize {
    let particle_target = particle_target.map(|renderer| renderer as &mut dyn ParticleBatchSink);
    let mut play_session_encrypted = false;
    drain_net_events_with_sinks_and_debug(
        rx,
        world,
        counters,
        &mut play_session_encrypted,
        net_commands,
        audio_events,
        particle_events,
        particle_target,
        item_runtime,
        None,
        None,
        level_event_sound_random,
        None,
    )
}

pub(in crate::runtime) fn drain_net_events_with_sinks_and_debug(
    rx: &mut mpsc::Receiver<NetEvent>,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    play_session_encrypted: &mut bool,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    mut audio_events: Option<&mut dyn AudioEventSink>,
    mut particle_events: Option<&mut dyn ParticleEventSink>,
    mut particle_target: Option<&mut dyn ParticleBatchSink>,
    item_runtime: Option<&NativeItemRuntime>,
    mut hud_debug_network_sampler: Option<&mut HudDebugNetworkSampler>,
    mut hud_debug_tps_sampler: Option<&mut HudDebugTpsSampler>,
    level_event_sound_random: &mut LevelEventSoundRandomState,
    mut network_terminal: Option<&mut bool>,
) -> usize {
    let mut drained = 0;
    while drained < 4096 {
        let event = match rx.try_recv() {
            Ok(event) => event,
            Err(mpsc::error::TryRecvError::Empty) => break,
            Err(mpsc::error::TryRecvError::Disconnected) => {
                counters.connected = false;
                *play_session_encrypted = false;
                if let Some(terminal) = network_terminal.as_deref_mut() {
                    *terminal = true;
                }
                break;
            }
        };
        drained += 1;
        if matches!(&event, NetEvent::Disconnected { .. }) {
            if let Some(terminal) = network_terminal.as_deref_mut() {
                *terminal = true;
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
                    counters,
                    net_commands,
                    audio_events: &mut audio_events,
                    particle_events: &mut particle_events,
                    particle_target: &mut particle_target,
                    item_runtime,
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
            NetEvent::Play(packet) => {
                let mut effects = NativePlayEffects {
                    counters,
                    net_commands,
                    audio_events: &mut audio_events,
                    particle_events: &mut particle_events,
                    particle_target: &mut particle_target,
                    item_runtime,
                };
                // Other connection-owned leftovers (rotation/movement
                // responses and resource-pack replies) are already handled by
                // the event stream.
                let _ = world.apply_play_packet(packet, level_event_sound_random, &mut effects);
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
            }
            NetEvent::StateChanged {
                state: ConnectionState::Login,
            } => {
                // A login listener starts from the client built-in registry
                // layer. Play reconfiguration reaches Configuration directly
                // and intentionally retains the current frozen access.
                world.reset_registry_session();
            }
            NetEvent::StateChanged {
                state: ConnectionState::Configuration,
            } => {
                world.clear_client_level();
                world.begin_registry_configuration();
            }
            NetEvent::StateChanged {
                state: ConnectionState::Play,
            } => match world.finish_registry_configuration() {
                Ok(()) => {
                    if let Some(audio_events) = audio_events.as_deref_mut() {
                        synchronize_audio_registries_from_world(world, audio_events);
                    }
                }
                Err(_) => {
                    if let Some(tx) = net_commands {
                        let _ = tx.try_send(NetCommand::Disconnect);
                    }
                }
            },
            NetEvent::ServicesProfileKeys { public_keys } => {
                world.replace_services_profile_keys(public_keys);
            }
            NetEvent::PlaySessionSecurity { encrypted } => {
                *play_session_encrypted = encrypted;
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
                data_source,
            } => {
                world.apply_select_known_packs_with_data(known_packs, selected_packs, data_source);
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
            NetEvent::ClearDialog => {
                world.apply_clear_dialog();
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
            NetEvent::RegistryData(update) => {
                let registry = update.registry.clone();
                world.record_registry_data(update);
                if let Some(audio_events) = audio_events.as_deref_mut() {
                    synchronize_audio_registry_from_world(world, audio_events, &registry);
                }
            }
            NetEvent::Connected => {
                *play_session_encrypted = false;
                world.reset_resource_pack_session();
            }
            NetEvent::Disconnected { .. } => {
                *play_session_encrypted = false;
            }
            NetEvent::StateChanged { .. }
            | NetEvent::CompressionSet { .. }
            | NetEvent::PacketSeen { .. }
            | NetEvent::UnsupportedPacket { .. } => {
                // Runtime/control projection events have no canonical world mutation here.
            }
        }
    }
    drained
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
struct NativePlayEffects<'a, 'b, 'c, 'audio, 'particle, 'renderer> {
    counters: &'a mut NetCounters,
    net_commands: &'b Option<mpsc::Sender<NetCommand>>,
    audio_events: &'c mut Option<&'audio mut dyn AudioEventSink>,
    particle_events: &'c mut Option<&'particle mut dyn ParticleEventSink>,
    particle_target: &'c mut Option<&'renderer mut dyn ParticleBatchSink>,
    item_runtime: Option<&'c NativeItemRuntime>,
}

impl PlayApplyEffects for NativePlayEffects<'_, '_, '_, '_, '_, '_> {
    fn positioned_sound(&mut self, state: &bbb_world::SoundEventState) {
        emit_positioned_sound(self.audio_events, state);
    }

    fn note_block_particle(&mut self, world: &WorldStore, state: NoteBlockParticleState) {
        let packet = LevelParticles {
            override_limiter: false,
            always_show: false,
            position: state.position,
            offset: state.velocity,
            max_speed: 1.0,
            count: 0,
            particle: ParticlePayload {
                particle_type_id: VANILLA_PARTICLE_TYPE_NOTE_ID,
                raw_options: Vec::new(),
            },
        };
        let biome_sampler = WorldParticleBiomeSampler { world };
        emit_level_particles(
            self.particle_events,
            self.particle_target,
            &packet,
            level_particle_spawn_context(world, &packet),
            Some(&biome_sampler),
            self.item_runtime,
        );
    }

    fn local_sound(&mut self, state: &bbb_world::LocalSoundEventState) {
        emit_local_sound(self.audio_events, state);
    }

    fn entity_sound(
        &mut self,
        state: &bbb_world::SoundEntityEventState,
        position: Option<[f64; 3]>,
    ) {
        emit_entity_sound(self.audio_events, state, position);
    }

    fn stop_sound(&mut self, state: &bbb_world::StopSoundEventState) {
        emit_stop_sound(self.audio_events, state);
    }

    fn stop_music(&mut self) {
        if let Some(audio_events) = self.audio_events.as_deref_mut() {
            audio_events.stop_music();
        }
    }

    fn perform_respawn(&mut self) {
        queue_perform_respawn_command(self.counters, self.net_commands);
    }

    fn jukebox_level_event(&mut self, state: &bbb_world::JukeboxLevelEventState) {
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

    fn vehicle_move_report(&mut self, report: bbb_world::VehicleMoveReport) {
        queue_vehicle_move_command(self.counters, self.net_commands, report);
    }

    fn removed_player_vehicle_teleport(
        &mut self,
        recovery: bbb_world::RemovedPlayerVehicleTeleportRecovery,
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
        let biome_sampler = WorldParticleBiomeSampler { world };
        emit_level_particles(
            self.particle_events,
            self.particle_target,
            packet,
            level_particle_spawn_context(world, packet),
            Some(&biome_sampler),
            self.item_runtime,
        );
    }

    fn explosion_particles(&mut self, world: &WorldStore, state: &ExplosionEventState) {
        let packet = LevelParticles {
            override_limiter: false,
            always_show: false,
            position: state.center,
            offset: ProtocolVec3d {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            max_speed: 1.0,
            count: 0,
            particle: state.explosion_particle.clone(),
        };
        let biome_sampler = WorldParticleBiomeSampler { world };
        let Some(particle_events) = self.particle_events.as_deref_mut() else {
            return;
        };
        let batch = particle_events.spawn_explosion_particles(
            state,
            world.client_level_generation(),
            level_particle_spawn_context(world, &packet),
            Some(&biome_sampler),
            self.item_runtime,
        );
        if let Some(renderer) = self.particle_target.as_deref_mut() {
            renderer.submit_particle_batch(batch);
        }
    }

    fn firework_empty_explosion_particles(&mut self, world: &WorldStore, position: [f64; 3]) {
        emit_firework_empty_explosion_particles(
            self.particle_events,
            self.particle_target,
            position,
            camera_audio_position_from_world(world)
                .map(|position| [position.x, position.y, position.z]),
        );
    }

    fn firework_explosion_particles(
        &mut self,
        world: &WorldStore,
        state: &FireworkRocketExplosionParticleState,
    ) {
        emit_firework_explosion_particles(
            self.particle_events,
            self.particle_target,
            state,
            camera_audio_position_from_world(world)
                .map(|position| [position.x, position.y, position.z]),
        );
    }

    fn llama_spit_recreation_particles(
        &mut self,
        _world: &WorldStore,
        state: LlamaSpitRecreationParticleState,
    ) {
        emit_llama_spit_recreation_particles(self.particle_events, self.particle_target, state);
    }

    fn elder_guardian_effect_particles(&mut self, world: &WorldStore, position: ProtocolVec3d) {
        emit_elder_guardian_effect_particles(
            self.particle_events,
            self.particle_target,
            position,
            camera_audio_position_from_world(world),
        );
    }

    fn tracking_emitter_particles(
        &mut self,
        world: &WorldStore,
        entity_id: i32,
        kind: EntityTrackingEmitterParticleKind,
        lifetime_ticks: u32,
    ) {
        emit_tracking_emitter_particles(
            self.particle_events,
            self.particle_target,
            world,
            entity_id,
            kind,
            lifetime_ticks,
        );
    }

    fn take_item_entity_pickup_particles(
        &mut self,
        _world: &WorldStore,
        state: &TakeItemEntityPickupParticleState,
    ) {
        emit_take_item_entity_pickup_particles(self.particle_events, self.particle_target, state);
    }

    fn ravager_roar_particles(&mut self, _world: &WorldStore, state: RavagerRoarParticleState) {
        emit_ravager_roar_particles(self.particle_events, self.particle_target, state);
    }

    fn witch_magic_particles(&mut self, _world: &WorldStore, state: WitchMagicParticleState) {
        emit_witch_magic_particles(self.particle_events, self.particle_target, state);
    }

    fn living_entity_poof_particles(
        &mut self,
        _world: &WorldStore,
        state: LivingEntityPoofParticleState,
    ) {
        emit_living_entity_poof_particles(self.particle_events, self.particle_target, state);
    }

    fn living_entity_item_break_particles(
        &mut self,
        _world: &WorldStore,
        state: LivingEntityItemBreakState,
    ) {
        emit_living_entity_item_break_particles(
            self.particle_events,
            self.particle_target,
            state,
            self.item_runtime,
        );
    }

    fn living_entity_drown_particles(
        &mut self,
        _world: &WorldStore,
        state: LivingEntityDrownParticleState,
    ) {
        emit_living_entity_drown_particles(self.particle_events, self.particle_target, state);
    }

    fn living_entity_portal_particles(
        &mut self,
        _world: &WorldStore,
        state: LivingEntityPortalParticleState,
    ) {
        emit_living_entity_portal_particles(self.particle_events, self.particle_target, state);
    }

    fn arrow_effect_particles(&mut self, _world: &WorldStore, state: ArrowEffectParticleState) {
        emit_arrow_effect_particles(self.particle_events, self.particle_target, state);
    }

    fn entity_taming_particles(&mut self, _world: &WorldStore, state: EntityTamingParticleState) {
        emit_entity_taming_particles(self.particle_events, self.particle_target, state);
    }

    fn villager_particles(&mut self, _world: &WorldStore, state: VillagerParticleState) {
        emit_villager_particles(self.particle_events, self.particle_target, state);
    }

    fn dolphin_happy_particles(&mut self, _world: &WorldStore, state: DolphinHappyParticleState) {
        emit_dolphin_happy_particles(self.particle_events, self.particle_target, state);
    }

    fn fox_eat_particles(&mut self, _world: &WorldStore, state: FoxEatParticleState) {
        emit_fox_eat_particles(
            self.particle_events,
            self.particle_target,
            state,
            self.item_runtime,
        );
    }

    fn animal_love_particles(&mut self, _world: &WorldStore, state: AnimalLoveParticleState) {
        emit_animal_love_particles(self.particle_events, self.particle_target, state);
    }

    fn allay_duplication_particles(
        &mut self,
        _world: &WorldStore,
        state: AllayDuplicationParticleState,
    ) {
        emit_allay_duplication_particles(self.particle_events, self.particle_target, state);
    }

    fn snowball_hit_particles(&mut self, _world: &WorldStore, state: SnowballHitParticleState) {
        emit_snowball_hit_particles(
            self.particle_events,
            self.particle_target,
            state,
            self.item_runtime,
        );
    }

    fn thrown_egg_hit_particles(&mut self, _world: &WorldStore, state: ThrownEggHitParticleState) {
        emit_thrown_egg_hit_particles(
            self.particle_events,
            self.particle_target,
            state,
            self.item_runtime,
        );
    }

    fn honey_block_particles(&mut self, _world: &WorldStore, state: HoneyBlockParticleState) {
        emit_honey_block_particles(self.particle_events, self.particle_target, state);
    }

    fn rabbit_sprint_particles(&mut self, _world: &WorldStore, state: RabbitSprintParticleState) {
        emit_rabbit_sprint_particles(self.particle_events, self.particle_target, state);
    }

    fn level_event_particles(
        &mut self,
        world: &WorldStore,
        event: &bbb_protocol::packets::LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> bool {
        let biome_sampler = WorldParticleBiomeSampler { world };
        emit_level_event_particles(
            self.particle_events,
            self.particle_target,
            event,
            level_event_particle_context(world, event),
            random,
            Some(&biome_sampler),
            self.item_runtime,
        )
    }

    fn sculk_charge_pop_full_block(
        &mut self,
        world: &WorldStore,
        event: &bbb_protocol::packets::LevelEvent,
    ) -> Option<bool> {
        sculk_charge_pop_full_block_context(world, event)
    }

    fn growth_particle_random_mode(
        &mut self,
        world: &WorldStore,
        event: &bbb_protocol::packets::LevelEvent,
    ) -> Option<LevelEventGrowthRandomMode> {
        growth_particle_context(world, event).map(|context| match context.mode {
            LevelEventGrowthParticleMode::InBlock { .. } => LevelEventGrowthRandomMode::InBlock,
            LevelEventGrowthParticleMode::WideNoFloating { .. } => {
                LevelEventGrowthRandomMode::WideNoFloating
            }
        })
    }
}

fn sound_event_registry_from_world(world: &WorldStore) -> SoundEventRegistry {
    world
        .registry_resource_ids_in_holder_order("minecraft:sound_event")
        .map(SoundEventRegistry::from_ids)
        .unwrap_or_else(SoundEventRegistry::vanilla_26_1)
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
                let projection = i32::try_from(holder_id).ok().and_then(|holder_id| {
                    world
                        .synchronized_jukebox_song_presentation(holder_id)
                        .ok()
                        .flatten()
                });
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
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    packet: &bbb_protocol::packets::LevelParticles,
    context: LevelParticleSpawnContext,
    biome_sampler: Option<&dyn ParticleBiomeSampler>,
    item_runtime: Option<&NativeItemRuntime>,
) {
    if let Some(particle_events) = particle_events.as_deref_mut() {
        let batch =
            particle_events.spawn_level_particles(packet, context, biome_sampler, item_runtime);
        if let Some(renderer) = particle_target.as_deref_mut() {
            renderer.submit_particle_batch(batch);
        }
    }
}

fn emit_firework_empty_explosion_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    position: [f64; 3],
    camera_position: Option<[f64; 3]>,
) {
    if let Some(particle_events) = particle_events.as_deref_mut() {
        let batch =
            particle_events.spawn_firework_empty_explosion_particles(position, camera_position);
        if let Some(renderer) = particle_target.as_deref_mut() {
            renderer.submit_particle_batch(batch);
        }
    }
}

fn emit_firework_explosion_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: &FireworkRocketExplosionParticleState,
    camera_position: Option<[f64; 3]>,
) {
    if let Some(particle_events) = particle_events.as_deref_mut() {
        let batch = particle_events.spawn_firework_explosion_particles(state, camera_position);
        if let Some(renderer) = particle_target.as_deref_mut() {
            renderer.submit_particle_batch(batch);
        }
    }
}

fn emit_llama_spit_recreation_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: LlamaSpitRecreationParticleState,
) {
    if let Some(particle_events) = particle_events.as_deref_mut() {
        let batch = particle_events.spawn_llama_spit_recreation_particles(state);
        if let Some(renderer) = particle_target.as_deref_mut() {
            renderer.submit_particle_batch(batch);
        }
    }
}

fn emit_elder_guardian_effect_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    position: ProtocolVec3d,
    camera_position: Option<ProtocolVec3d>,
) {
    if let Some(particle_events) = particle_events.as_deref_mut() {
        let packet = LevelParticles {
            override_limiter: false,
            always_show: false,
            position,
            offset: ProtocolVec3d::default(),
            max_speed: 1.0,
            count: 0,
            particle: ParticlePayload {
                particle_type_id: VANILLA_PARTICLE_TYPE_ELDER_GUARDIAN_ID,
                raw_options: Vec::new(),
            },
        };
        let batch = particle_events.spawn_level_particles(
            &packet,
            LevelParticleSpawnContext {
                camera_position: camera_position
                    .map(|position| [position.x, position.y, position.z]),
                ..LevelParticleSpawnContext::default()
            },
            None,
            None,
        );
        if let Some(renderer) = particle_target.as_deref_mut() {
            renderer.submit_particle_batch(batch);
        }
    }
}

fn emit_tracking_emitter_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    world: &WorldStore,
    entity_id: i32,
    kind: EntityTrackingEmitterParticleKind,
    lifetime_ticks: u32,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let Some(transform) = world.probe_entity_transform(entity_id) else {
        return;
    };
    let Some(bounds) = world.probe_entity_pick_bounds(entity_id) else {
        return;
    };
    let particle_type_id = match kind {
        EntityTrackingEmitterParticleKind::Crit => VANILLA_PARTICLE_TYPE_CRIT_ID,
        EntityTrackingEmitterParticleKind::EnchantedHit => VANILLA_PARTICLE_TYPE_ENCHANTED_HIT_ID,
        EntityTrackingEmitterParticleKind::TotemOfUndying => {
            VANILLA_PARTICLE_TYPE_TOTEM_OF_UNDYING_ID
        }
    };
    let state = TrackingEmitterParticleState {
        particle_type_id,
        position: [
            transform.position.x,
            transform.position.y,
            transform.position.z,
        ],
        width: bounds.max[0] - bounds.min[0],
        height: bounds.max[1] - bounds.min[1],
        lifetime_ticks,
    };
    let batch = particle_events.spawn_tracking_emitter_particles(state);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_take_item_entity_pickup_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: &TakeItemEntityPickupParticleState,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_take_item_entity_pickup_particles(state);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_ravager_roar_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: RavagerRoarParticleState,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_ravager_roar_particles(state);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_witch_magic_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: WitchMagicParticleState,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_witch_magic_particles(state);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_living_entity_poof_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: LivingEntityPoofParticleState,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_living_entity_poof_particles(state);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_living_entity_item_break_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: LivingEntityItemBreakState,
    item_runtime: Option<&NativeItemRuntime>,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_living_entity_item_break_particles(state, item_runtime);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_living_entity_drown_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: LivingEntityDrownParticleState,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_living_entity_drown_particles(state);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_living_entity_portal_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: LivingEntityPortalParticleState,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_living_entity_portal_particles(state);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_snowball_hit_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: SnowballHitParticleState,
    item_runtime: Option<&NativeItemRuntime>,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_snowball_hit_particles(state, item_runtime);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_thrown_egg_hit_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: ThrownEggHitParticleState,
    item_runtime: Option<&NativeItemRuntime>,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_thrown_egg_hit_particles(state, item_runtime);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_arrow_effect_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: ArrowEffectParticleState,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_arrow_effect_particles(state);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_entity_taming_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: EntityTamingParticleState,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_entity_taming_particles(state);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_villager_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: VillagerParticleState,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_villager_particles(state);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_dolphin_happy_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: DolphinHappyParticleState,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_dolphin_happy_particles(state);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_fox_eat_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: FoxEatParticleState,
    item_runtime: Option<&NativeItemRuntime>,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_fox_eat_particles(state, item_runtime);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_animal_love_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: AnimalLoveParticleState,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_animal_love_particles(state);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_allay_duplication_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: AllayDuplicationParticleState,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_allay_duplication_particles(state);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_honey_block_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: HoneyBlockParticleState,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_honey_block_particles(state);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

fn emit_rabbit_sprint_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    state: RabbitSprintParticleState,
) {
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    let batch = particle_events.spawn_rabbit_sprint_particles(state);
    if let Some(renderer) = particle_target.as_deref_mut() {
        renderer.submit_particle_batch(batch);
    }
}

struct WorldParticleBiomeSampler<'a> {
    world: &'a WorldStore,
}

impl ParticleBiomeSampler for WorldParticleBiomeSampler<'_> {
    fn biome_id_at(&self, pos: WorldBlockPos) -> Option<i32> {
        self.world.probe_block(pos).and_then(|probe| probe.biome_id)
    }
}

fn level_particle_spawn_context(
    world: &WorldStore,
    packet: &bbb_protocol::packets::LevelParticles,
) -> LevelParticleSpawnContext {
    LevelParticleSpawnContext {
        camera_position: camera_audio_position_from_world(world)
            .map(|position| [position.x, position.y, position.z]),
        vibration_entity_position: vibration_entity_position_source_from_options(
            packet.particle.particle_type_id,
            &packet.particle.raw_options,
        )
        .and_then(|source| {
            let transform = world.probe_entity_transform(source.entity_id)?;
            Some(LevelParticleEntityPosition {
                entity_id: source.entity_id,
                position: [
                    transform.position.x,
                    transform.position.y,
                    transform.position.z,
                ],
            })
        }),
    }
}

fn emit_level_event_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_target: &mut Option<&mut dyn ParticleBatchSink>,
    event: &bbb_protocol::packets::LevelEvent,
    context: LevelEventParticleContext,
    random: &mut LevelEventSoundRandomState,
    biome_sampler: Option<&dyn ParticleBiomeSampler>,
    item_runtime: Option<&NativeItemRuntime>,
) -> bool {
    if let Some(particle_events) = particle_events.as_deref_mut() {
        let batch = particle_events.spawn_level_event_particles(
            event,
            context,
            random,
            biome_sampler,
            item_runtime,
        );
        if let Some(renderer) = particle_target.as_deref_mut() {
            renderer.submit_particle_batch(batch);
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
    world
        .probe_block(protocol_to_world_block_pos(event.pos))
        .and_then(|probe| probe.biome_id)
}

fn vault_block_entity_at_event_pos_context(
    world: &WorldStore,
    event: &bbb_protocol::packets::LevelEvent,
) -> bool {
    event.event_type == VAULT_ACTIVATE_LEVEL_EVENT
        && world.block_entity_type_id_at(protocol_to_world_block_pos(event.pos))
            == Some(VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID)
}

fn vault_connection_particle_context(
    world: &WorldStore,
    event: &bbb_protocol::packets::LevelEvent,
) -> Option<bbb_world::VaultConnectionParticleState> {
    (event.event_type == VAULT_ACTIVATE_LEVEL_EVENT)
        .then(|| world.vault_connection_particle_state(protocol_to_world_block_pos(event.pos)))
        .flatten()
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
    probe
        .block_name
        .as_deref()
        .is_some_and(bbb_world::block_name_is_air)
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
) -> Option<ClientParticleOptions> {
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

    if above_root.block_name.as_deref() == Some("minecraft:mud")
        && !level_water_evaporates(world, above_root_pos)
    {
        return Some(simple_client_particle_options(
            "minecraft:dripping_dripstone_water",
        ));
    }

    match above_root.fluid.map(|fluid| fluid.kind) {
        Some(TerrainFluidKind::Lava) => Some(simple_client_particle_options(
            "minecraft:dripping_dripstone_lava",
        )),
        Some(TerrainFluidKind::Water) => Some(simple_client_particle_options(
            "minecraft:dripping_dripstone_water",
        )),
        None => Some(default_dripstone_drip_particle(world, above_root_pos)),
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

fn default_dripstone_drip_particle(
    world: &WorldStore,
    position: bbb_world::BlockPos,
) -> ClientParticleOptions {
    if let Some(particle) = world
        .sampled_client_environment_attributes_at(
            [
                f64::from(position.x),
                f64::from(position.y),
                f64::from(position.z),
            ],
            bbb_world::EnvironmentBiomeSampling::Direct,
        )
        .ok()
        .and_then(|attributes| attributes.default_dripstone_particle)
    {
        return particle;
    }
    if world
        .registry_content(bbb_world::DIMENSION_TYPE_REGISTRY)
        .is_some()
    {
        return simple_client_particle_options("minecraft:dripping_dripstone_water");
    }
    if level_water_evaporates(world, position) {
        simple_client_particle_options("minecraft:dripping_dripstone_lava")
    } else {
        simple_client_particle_options("minecraft:dripping_dripstone_water")
    }
}

fn level_water_evaporates(world: &WorldStore, position: bbb_world::BlockPos) -> bool {
    world.water_evaporates_at(position)
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

#[cfg(test)]
mod terminal_tests {
    use super::*;

    #[test]
    fn play_session_security_tracks_event_and_connection_lifecycle() {
        let (tx, mut rx) = mpsc::channel(8);
        tx.try_send(NetEvent::Connected).unwrap();
        tx.try_send(NetEvent::PlaySessionSecurity { encrypted: true })
            .unwrap();
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();
        let mut play_session_encrypted = false;
        let mut random = LevelEventSoundRandomState::with_seed(0);

        let drained = drain_net_events_with_sinks_and_debug(
            &mut rx,
            &mut world,
            &mut counters,
            &mut play_session_encrypted,
            &None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut random,
            None,
        );
        assert_eq!(drained, 2);
        assert!(play_session_encrypted);

        tx.try_send(NetEvent::Disconnected { reason: None })
            .unwrap();
        assert_eq!(
            drain_net_events_with_sinks_and_debug(
                &mut rx,
                &mut world,
                &mut counters,
                &mut play_session_encrypted,
                &None,
                None,
                None,
                None,
                None,
                None,
                None,
                &mut random,
                None,
            ),
            1
        );
        assert!(!play_session_encrypted);

        play_session_encrypted = true;
        drop(tx);
        assert_eq!(
            drain_net_events_with_sinks_and_debug(
                &mut rx,
                &mut world,
                &mut counters,
                &mut play_session_encrypted,
                &None,
                None,
                None,
                None,
                None,
                None,
                None,
                &mut random,
                None,
            ),
            0
        );
        assert!(!play_session_encrypted);
    }

    #[test]
    fn terminal_flag_observes_connected_and_disconnected_in_one_drain() {
        let (tx, mut rx) = mpsc::channel(4);
        tx.try_send(NetEvent::Connected).unwrap();
        tx.try_send(NetEvent::Disconnected { reason: None })
            .unwrap();
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();
        let mut play_session_encrypted = true;
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut terminal = false;

        let drained = drain_net_events_with_sinks_and_debug(
            &mut rx,
            &mut world,
            &mut counters,
            &mut play_session_encrypted,
            &None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut random,
            Some(&mut terminal),
        );

        assert_eq!(drained, 2);
        assert!(terminal);
        assert!(!counters.connected);
        assert!(!play_session_encrypted);
    }
}
