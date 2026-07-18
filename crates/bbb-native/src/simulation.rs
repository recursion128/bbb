//! Native client simulation ownership.
//!
//! Phase one keeps this owner on the winit thread. Later phases replace the
//! borrowed backend context with value commands and move this same core to the
//! dedicated simulation thread; they must not introduce a second mutation path.

use bbb_control::{AudioCounters, NetCounters, SharedSnapshot};
use bbb_item_model::NativeItemRuntime;
use bbb_net::{NetCommand, NetEvent};
use bbb_renderer::{HudDebugProfilerChart, Renderer};
use bbb_world::WorldStore;
use tokio::sync::mpsc;
use winit::dpi::PhysicalSize;

pub(crate) mod commands;
pub(crate) mod effects;
pub(crate) mod input_commands;

use crate::{
    audio_runtime::AudioEventSink,
    code_of_conduct::CodeOfConductAcceptance,
    entity_variant_textures::NativeEntityVariantTextureRuntime,
    input::ClientInputState,
    level_load_runtime::LevelLoadRuntime,
    particle_runtime::NativeParticleRuntime,
    resource_pack_runtime::ServerResourcePackRuntime,
    runtime::{
        advance_simulation, audio_registry_snapshot_from_world, update_renderer_from_simulation,
        CameraFovTickState, ClientAnimationTickState, DebugCollisionCache,
        DebugSkyLightSectionCache, DebugSupportingBlockCache, HudDebugFpsSampler,
        HudDebugNetworkSampler, HudDebugTpsSampler, LevelEventSoundRandomState, LightmapTickState,
        SimulationFrameState,
    },
    terrain_runtime::{TerrainTextureState, TerrainUploadState},
};
use commands::SimulationMailbox;
use input_commands::SimulationInputRuntime;

/// The single mutable owner of canonical state and the current client-tick path.
///
/// Fields stay crate-visible during the synchronous migration phases because
/// render extraction still borrows the live owner. Winit and control mutation
/// already enter through the typed command mailbox; phase four replaces the
/// remaining render-side reads with an immutable publication.
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
    pub(crate) command_mailbox: SimulationMailbox,
    pub(crate) input_runtime: SimulationInputRuntime,
    pub(crate) next_effect_sequence: u64,
}

/// Backend/platform borrows that still surround one synchronous simulation step.
///
/// These are deliberately not fields of [`SimulationCore`]. Effect isolation
/// and immutable render publication remove them before the core moves threads.
pub(crate) struct SimulationStepContext<'a> {
    pub(crate) audio_enabled: bool,
    pub(crate) audio_status: &'a AudioCounters,
    pub(crate) terrain_textures: &'a mut TerrainTextureState,
    pub(crate) item_runtime: Option<&'a NativeItemRuntime>,
    pub(crate) code_of_conduct: Option<&'a mut CodeOfConductAcceptance>,
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
        let mut platform_effects = effects.platform();
        let commands = self.command_mailbox.drain_through_barrier();
        let frame = advance_simulation(
            &mut self.net_events,
            &self.net_commands,
            &mut self.server_resource_packs,
            context
                .audio_enabled
                .then_some(&mut audio_effects as &mut dyn AudioEventSink),
            &mut self.particle_runtime,
            &mut particle_batches,
            &mut self.input,
            &mut self.world,
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
            context.code_of_conduct,
            commands,
            &mut self.input_runtime,
            &mut platform_effects,
            context.render_distance_chunks,
            context.hide_lightning_flash,
        );
        drop((audio_effects, particle_batches, platform_effects));
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

    pub(crate) fn update_code_of_conduct_overlay_renderer(
        &mut self,
        renderer: &mut Renderer,
        accepted_current_code_of_conduct: bool,
    ) {
        self.input_runtime.update_code_of_conduct_overlay_renderer(
            renderer,
            &self.world,
            accepted_current_code_of_conduct,
        );
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
        render: &mut dyn effects::RenderEffectSink,
    ) -> Result<
        (SimulationFrameState, Option<effects::ResourceReloadEffect>),
        effects::RenderEffectSequenceError,
    > {
        let reload = consumer.consume(self.effects, audio, render)?;
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

    #[test]
    fn winit_input_callbacks_only_enqueue_typed_simulation_commands() {
        let source = include_str!("main.rs");
        let start = source
            .find("Event::WindowEvent { event, window_id }")
            .expect("window event dispatcher");
        let end = source[start..]
            .find("WindowEvent::RedrawRequested")
            .map(|offset| start + offset)
            .expect("redraw boundary");
        let input_callbacks = &source[start..end];
        assert!(input_callbacks.contains("enqueue_simulation_window_event("));
        assert!(!input_callbacks.contains("simulation.input"));
        assert!(!input_callbacks.contains("simulation.world"));
        assert!(!input_callbacks.contains("simulation.net_counters"));
        assert!(!input_callbacks.contains("handle_key_input"));
        assert!(!input_callbacks.contains("handle_mouse_"));

        let runtime = include_str!("runtime.rs");
        let advance = runtime
            .split_once("pub(crate) fn advance_simulation(")
            .expect("advance_simulation")
            .1
            .split_once("pub(crate) fn update_renderer_from_simulation(")
            .expect("advance_simulation end")
            .0;
        assert!(advance.contains("input_runtime.apply_commands("));
        assert!(!advance.contains("pump_control_input_requests("));
        assert!(!advance.contains("pump_control_net_requests("));
    }
}
