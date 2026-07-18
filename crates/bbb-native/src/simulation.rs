//! Native client simulation ownership.
//!
//! Phase one keeps this owner on the winit thread. Later phases replace the
//! borrowed backend context with value commands and move this same core to the
//! dedicated simulation thread; they must not introduce a second mutation path.

use bbb_control::{AudioCounters, NetCounters, SharedControlRequests, SharedSnapshot};
use bbb_item_model::NativeItemRuntime;
use bbb_net::{NetCommand, NetEvent};
use bbb_renderer::{HudDebugProfilerChart, Renderer};
use bbb_world::WorldStore;
use tokio::sync::mpsc;
use winit::dpi::PhysicalSize;

pub(crate) mod effects;

use crate::{
    audio_runtime::AudioEventSink,
    code_of_conduct::CodeOfConductAcceptance,
    entity_variant_textures::NativeEntityVariantTextureRuntime,
    input::ClientInputState,
    level_load_runtime::LevelLoadRuntime,
    particle_runtime::{NativeParticleRuntime, ParticleEventSink},
    resource_pack_runtime::ServerResourcePackRuntime,
    resource_reload::ResourceReloadAck,
    runtime::{
        advance_simulation, audio_registry_snapshot_from_world, update_renderer_from_simulation,
        CameraFovTickState, ClientAnimationTickState, DebugCollisionCache,
        DebugSkyLightSectionCache, DebugSupportingBlockCache, HudDebugFpsSampler,
        HudDebugNetworkSampler, HudDebugTpsSampler, LevelEventSoundRandomState, LightmapTickState,
        SimulationFrameState,
    },
    terrain_runtime::{TerrainTextureState, TerrainUploadState},
};

/// The single mutable owner of canonical state and the current client-tick path.
///
/// Fields stay crate-visible during the synchronous migration phases because
/// existing winit input callbacks still call the canonical helpers directly.
/// Phase three replaces those accesses with the typed command mailbox.
pub(crate) struct SimulationCore {
    pub(crate) net_events: Option<mpsc::Receiver<NetEvent>>,
    pub(crate) net_commands: Option<mpsc::Sender<NetCommand>>,
    pub(crate) server_resource_packs: ServerResourcePackRuntime,
    pub(crate) input: ClientInputState,
    pub(crate) world: WorldStore,
    pub(crate) net_counters: NetCounters,
    pub(crate) client_animation_ticks: ClientAnimationTickState,
    pub(crate) camera_fov_ticks: CameraFovTickState,
    pub(crate) hud_debug_fps_sampler: HudDebugFpsSampler,
    pub(crate) hud_debug_network_sampler: HudDebugNetworkSampler,
    pub(crate) hud_debug_tps_sampler: HudDebugTpsSampler,
    pub(crate) debug_collision_cache: DebugCollisionCache,
    pub(crate) debug_supporting_block_cache: DebugSupportingBlockCache,
    pub(crate) debug_sky_light_section_cache: DebugSkyLightSectionCache,
    pub(crate) lightmap_ticks: LightmapTickState,
    pub(crate) level_event_sound_random: LevelEventSoundRandomState,
    pub(crate) particle_runtime: Option<NativeParticleRuntime>,
    pub(crate) next_effect_sequence: u64,
}

/// Backend/platform borrows that still surround one synchronous simulation step.
///
/// These are deliberately not fields of [`SimulationCore`]. Effect isolation
/// and immutable render publication remove them before the core moves threads.
pub(crate) struct SimulationStepContext<'a> {
    pub(crate) audio_enabled: bool,
    pub(crate) audio_status: &'a AudioCounters,
    pub(crate) surface_size: PhysicalSize<u32>,
    pub(crate) terrain_textures: &'a mut TerrainTextureState,
    pub(crate) item_runtime: Option<&'a NativeItemRuntime>,
    pub(crate) control_requests: &'a SharedControlRequests,
    pub(crate) code_of_conduct: Option<&'a mut CodeOfConductAcceptance>,
    pub(crate) code_of_conduct_overlay_visible: bool,
    pub(crate) render_distance_chunks: u32,
    pub(crate) hide_lightning_flash: bool,
}

/// Render-owner borrows used only after the simulation step has sealed its
/// world mutations and value effects.
pub(crate) struct SimulationRenderContext<'a> {
    pub(crate) audio_events: Option<&'a mut dyn AudioEventSink>,
    pub(crate) audio_status: &'a AudioCounters,
    pub(crate) renderer: &'a mut Renderer,
    pub(crate) surface_size: PhysicalSize<u32>,
    pub(crate) terrain_upload: &'a mut TerrainUploadState,
    pub(crate) level_load_runtime: &'a mut LevelLoadRuntime,
    pub(crate) terrain_textures: &'a mut TerrainTextureState,
    pub(crate) entity_variant_textures: &'a mut NativeEntityVariantTextureRuntime,
    pub(crate) item_runtime: Option<&'a NativeItemRuntime>,
    pub(crate) snapshot: &'a SharedSnapshot,
    pub(crate) render_distance_chunks: u32,
    pub(crate) client_framerate_limit: u32,
    pub(crate) client_vsync: bool,
    pub(crate) damage_tilt_strength: f32,
    pub(crate) hud_debug_profiler_chart: Option<HudDebugProfilerChart>,
}

impl SimulationCore {
    /// Runs the existing ordered pump through the new single owner.
    ///
    /// This remains synchronous in phases one through four. Moving the core to
    /// a thread changes the executor only after all backend borrows disappear.
    pub(crate) fn step(&mut self, context: SimulationStepContext<'_>) -> SimulationStepResult {
        let effect_sequence = self.next_effect_sequence;
        self.next_effect_sequence = self.next_effect_sequence.saturating_add(1);
        let effects =
            effects::RenderEffectCollector::new(effect_sequence, context.audio_status.clone());
        let mut audio_effects = effects.audio();
        let mut particle_batches = effects.particles();
        let frame = advance_simulation(
            &mut self.net_events,
            &self.net_commands,
            &mut self.server_resource_packs,
            context
                .audio_enabled
                .then_some(&mut audio_effects as &mut dyn AudioEventSink),
            self.particle_runtime
                .as_mut()
                .map(|runtime| runtime as &mut dyn ParticleEventSink),
            &mut particle_batches,
            &mut self.input,
            &mut self.world,
            context.surface_size,
            &mut self.net_counters,
            &mut self.client_animation_ticks,
            &mut self.camera_fov_ticks,
            &mut self.hud_debug_fps_sampler,
            &mut self.hud_debug_network_sampler,
            &mut self.hud_debug_tps_sampler,
            &mut self.lightmap_ticks,
            &mut self.level_event_sound_random,
            context.terrain_textures,
            context.item_runtime,
            context.control_requests,
            context.code_of_conduct,
            context.code_of_conduct_overlay_visible,
            context.render_distance_chunks,
            context.hide_lightning_flash,
        );
        drop((audio_effects, particle_batches));
        let resource_reload = self.server_resource_packs.take_reload().map(|request| {
            let (sound_events, jukebox_songs) = audio_registry_snapshot_from_world(&self.world);
            effects::ResourceReloadEffect {
                request,
                sound_events,
                jukebox_songs,
            }
        });
        SimulationStepResult {
            effects: effects.finish(resource_reload),
            frame,
        }
    }

    pub(crate) fn update_renderer(
        &mut self,
        frame: SimulationFrameState,
        context: SimulationRenderContext<'_>,
    ) -> bool {
        update_renderer_from_simulation(
            frame,
            context.audio_events,
            context.audio_status,
            &mut self.input,
            &mut self.world,
            context.renderer,
            context.surface_size,
            &mut self.net_counters,
            &self.net_commands,
            &mut self.client_animation_ticks,
            &self.camera_fov_ticks,
            &self.hud_debug_fps_sampler,
            &self.hud_debug_network_sampler,
            &self.hud_debug_tps_sampler,
            &mut self.debug_collision_cache,
            &mut self.debug_supporting_block_cache,
            &mut self.debug_sky_light_section_cache,
            &self.lightmap_ticks,
            context.terrain_upload,
            context.level_load_runtime,
            context.terrain_textures,
            context.entity_variant_textures,
            context.item_runtime,
            context.snapshot,
            context.render_distance_chunks,
            context.client_framerate_limit,
            context.client_vsync,
            context.damage_tilt_strength,
            context.hud_debug_profiler_chart,
        )
    }

    pub(crate) fn acknowledge_resource_reload(&mut self, ack: ResourceReloadAck) -> bool {
        let status = ack.result.as_ref().map(|_| ()).map_err(ToString::to_string);
        if !self
            .server_resource_packs
            .finish_reload(ack.generation, status)
        {
            tracing::warn!(
                generation = ack.generation,
                "ignored stale server resource pack reload acknowledgement"
            );
            return false;
        }
        match ack.result {
            Ok(commit) => {
                commit.item_defaults.apply(&mut self.world);
                self.world
                    .set_default_block_destroy_profiles(commit.block_destroy_profiles);
                self.world
                    .set_default_block_sound_profiles(commit.block_sound_profiles);
                self.particle_runtime = Some(commit.particle_runtime);
                tracing::info!(
                    generation = ack.generation,
                    "applied server resource pack reload"
                );
            }
            Err(error) => {
                tracing::warn!(
                    generation = ack.generation,
                    error = %error,
                    "server resource pack reload failed; retained previous resources"
                );
            }
        }
        self.server_resource_packs
            .publish_runtime_diagnostics(&mut self.world);
        true
    }
}

pub(crate) struct SimulationStepResult {
    effects: effects::RenderEffectBatch,
    frame: SimulationFrameState,
}

impl SimulationStepResult {
    pub(crate) fn consume_render_effects(
        self,
        consumer: &mut effects::RenderEffectConsumer,
        audio: Option<&mut dyn AudioEventSink>,
        particles: &mut dyn effects::ParticleBatchSink,
    ) -> Result<
        (SimulationFrameState, Option<effects::ResourceReloadEffect>),
        effects::RenderEffectSequenceError,
    > {
        let reload = consumer.consume(self.effects, audio, particles)?;
        Ok((self.frame, reload))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn main_routes_both_pump_sites_through_the_single_simulation_owner() {
        let source = include_str!("main.rs");
        assert!(source.contains("let mut simulation = SimulationCore"));
        assert_eq!(source.matches(".step(SimulationStepContext").count(), 2);
        assert_eq!(source.matches(".consume_render_effects(").count(), 2);
        assert!(!source.contains("pump_network_and_terrain("));
        assert!(!source.contains(".server_resource_packs.take_reload("));
    }
}
