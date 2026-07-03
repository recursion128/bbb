use bbb_control::NetCounters;
use bbb_item_model::NativeItemRuntime;
use bbb_net::{ConnectionState, NetCommand, NetEvent};
use bbb_pack::{JukeboxSongRegistry, SoundEventRegistry};
use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, RegistryData, Vec3d as ProtocolVec3d};
use bbb_world::{
    BlockPos as WorldBlockPos, LevelEventGrowthRandomMode, LevelEventSoundRandomState,
    PlayApplyEffects, TerrainFluidKind, WorldStore,
};
use tokio::sync::mpsc;

use crate::audio_runtime::AudioEventSink;
use crate::input::queue_vehicle_move_command;
use crate::particle_runtime::{
    vibration_entity_position_source_from_options, LevelEventDripstoneDripParticle,
    LevelEventGrowthParticleContext, LevelEventGrowthParticleMode, LevelEventGrowthParticleSupport,
    LevelEventParticleContext, LevelParticleEntityPosition, LevelParticleSpawnContext,
    ParticleBiomeSampler, ParticleEventSink,
};

use super::control_state::apply_control_projection_event;

const COMPOSTER_FILL_LEVEL_EVENT: i32 = 1500;
const DRIPSTONE_DRIP_LEVEL_EVENT: i32 = 1504;
const PLANT_GROWTH_LEVEL_EVENT: i32 = 1505;
const BEE_GROWTH_PARTICLES_LEVEL_EVENT: i32 = 2011;
const TURTLE_EGG_PLACEMENT_PARTICLES_LEVEL_EVENT: i32 = 2012;
const VAULT_ACTIVATE_LEVEL_EVENT: i32 = 3015;
const POINTED_DRIPSTONE_ROOT_SEARCH_LENGTH: i32 = 11;
// Vanilla 26.1 BlockEntityType registry order in BlockEntityType.java.
const VANILLA_VAULT_BLOCK_ENTITY_TYPE_ID: i32 = 45;

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

pub(in crate::runtime) fn drain_net_events_with_sinks(
    rx: &mut mpsc::Receiver<NetEvent>,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    mut audio_events: Option<&mut dyn AudioEventSink>,
    mut particle_events: Option<&mut dyn ParticleEventSink>,
    mut particle_renderer: Option<&mut bbb_renderer::Renderer>,
    item_runtime: Option<&NativeItemRuntime>,
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
            NetEvent::Play(packet) => {
                let mut effects = NativePlayEffects {
                    counters,
                    net_commands,
                    audio_events: &mut audio_events,
                    particle_events: &mut particle_events,
                    particle_renderer: &mut particle_renderer,
                    item_runtime,
                };
                // Connection-owned leftovers (movement responses, resource
                // pack replies) are already handled by the event stream.
                let _ = world.apply_play_packet(packet, level_event_sound_random, &mut effects);
            }
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

/// Runtime sink routing for `WorldStore::apply_play_packet`.
struct NativePlayEffects<'a, 'b, 'c, 'audio, 'particle, 'renderer> {
    counters: &'a mut NetCounters,
    net_commands: &'b Option<mpsc::Sender<NetCommand>>,
    audio_events: &'c mut Option<&'audio mut dyn AudioEventSink>,
    particle_events: &'c mut Option<&'particle mut dyn ParticleEventSink>,
    particle_renderer: &'c mut Option<&'renderer mut bbb_renderer::Renderer>,
    item_runtime: Option<&'c NativeItemRuntime>,
}

impl PlayApplyEffects for NativePlayEffects<'_, '_, '_, '_, '_, '_> {
    fn positioned_sound(&mut self, state: &bbb_world::SoundEventState) {
        emit_positioned_sound(self.audio_events, state);
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

    fn jukebox_level_event(&mut self, state: &bbb_world::JukeboxLevelEventState) {
        emit_jukebox_level_event(self.audio_events, state);
    }

    fn chat_acknowledgement(&mut self, command: bbb_protocol::packets::ChatAcknowledgement) {
        queue_chat_acknowledgement(self.net_commands, self.counters, command);
    }

    fn vehicle_move_report(&mut self, report: bbb_world::VehicleMoveReport) {
        queue_vehicle_move_command(self.counters, self.net_commands, report);
    }

    fn level_particles(
        &mut self,
        world: &WorldStore,
        packet: &bbb_protocol::packets::LevelParticles,
    ) {
        let biome_sampler = WorldParticleBiomeSampler { world };
        emit_level_particles(
            self.particle_events,
            self.particle_renderer,
            packet,
            level_particle_spawn_context(world, packet),
            Some(&biome_sampler),
            self.item_runtime,
        );
    }

    fn firework_empty_explosion_particles(&mut self, world: &WorldStore, position: [f64; 3]) {
        emit_firework_empty_explosion_particles(
            self.particle_events,
            self.particle_renderer,
            position,
            camera_audio_position_from_world(world)
                .map(|position| [position.x, position.y, position.z]),
        );
    }

    fn level_event_particles(
        &mut self,
        world: &WorldStore,
        event: &bbb_protocol::packets::LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> bool {
        emit_level_event_particles(
            self.particle_events,
            self.particle_renderer,
            event,
            level_event_particle_context(world, event),
            random,
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
    biome_sampler: Option<&dyn ParticleBiomeSampler>,
    item_runtime: Option<&NativeItemRuntime>,
) {
    if let Some(particle_events) = particle_events.as_deref_mut() {
        let batch =
            particle_events.spawn_level_particles(packet, context, biome_sampler, item_runtime);
        if let Some(renderer) = particle_renderer.as_deref_mut() {
            renderer.submit_particle_spawns(batch);
        }
    }
}

fn emit_firework_empty_explosion_particles(
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    particle_renderer: &mut Option<&mut bbb_renderer::Renderer>,
    position: [f64; 3],
    camera_position: Option<[f64; 3]>,
) {
    if let Some(particle_events) = particle_events.as_deref_mut() {
        let batch =
            particle_events.spawn_firework_empty_explosion_particles(position, camera_position);
        if let Some(renderer) = particle_renderer.as_deref_mut() {
            renderer.submit_particle_spawns(batch);
        }
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
        biome_id_at_event_pos: event_pos_biome_id_context(world, event),
        vault_block_entity_at_event_pos: vault_block_entity_at_event_pos_context(world, event),
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
            == Some(VANILLA_VAULT_BLOCK_ENTITY_TYPE_ID)
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
