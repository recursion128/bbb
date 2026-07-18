//! Native client simulation ownership.
//!
//! Phases one through four keep this owner on the winit thread. Phase five
//! replaces the remaining borrowed resource context with owned simulation
//! projections and moves this same core to the dedicated thread; it must not
//! introduce a second mutation path.

use std::sync::Arc;

use bbb_control::{AudioCounters, NetCounters, RendererCounters};
use bbb_item_model::NativeItemRuntime;
use bbb_net::{NetCommand, NetEvent};
use bbb_renderer::{HudDebugProfilerChart, Renderer};
use bbb_world::WorldStore;
use tokio::sync::mpsc;

pub(crate) mod commands;
pub(crate) mod effects;
pub(crate) mod input_commands;
pub(crate) mod publication;

use crate::{
    audio_runtime::AudioEventSink,
    code_of_conduct::CodeOfConductAcceptance,
    entity_variant_textures::NativeEntityVariantTextureRuntime,
    input::ClientInputState,
    level_load_runtime::LevelLoadRuntime,
    particle_runtime::NativeParticleRuntime,
    resource_pack_runtime::ServerResourcePackRuntime,
    runtime::{
        advance_simulation, audio_registry_snapshot_from_world,
        update_renderer_from_publication as update_renderer, CameraFovTickState,
        ClientAnimationTickState, DebugCollisionCache, DebugSkyLightSectionCache,
        DebugSupportingBlockCache, HudDebugFpsSampler, HudDebugNetworkSampler, HudDebugTpsSampler,
        HudRenderRuntime, LevelEventSoundRandomState, LightmapTickState,
    },
    terrain_runtime::{TerrainTextureState, TerrainUploadState},
};
use commands::SimulationMailbox;
use input_commands::SimulationInputRuntime;
use publication::{SimulationPublication, SimulationPublicationInput, SimulationPublisher};

/// The single mutable owner of canonical state and the current client-tick path.
///
/// Fields stay crate-visible while construction remains in `main`, but the
/// event loop may only call [`SimulationCore::step`]. Winit/control mutation
/// enters through the typed command mailbox and every render/control read uses
/// the immutable publication sealed by that step.
pub(crate) struct SimulationCore {
    pub(crate) net_events: Option<mpsc::Receiver<NetEvent>>,
    pub(crate) net_commands: Option<mpsc::Sender<NetCommand>>,
    pub(crate) server_resource_packs: ServerResourcePackRuntime,
    pub(crate) input: ClientInputState,
    pub(crate) world: WorldStore,
    pub(crate) net_counters: NetCounters,
    pub(crate) client_animation_ticks: ClientAnimationTickState,
    pub(crate) camera_fov_ticks: CameraFovTickState,
    pub(crate) hud_debug_network_sampler: HudDebugNetworkSampler,
    pub(crate) hud_debug_tps_sampler: HudDebugTpsSampler,
    pub(crate) lightmap_ticks: LightmapTickState,
    pub(crate) level_event_sound_random: LevelEventSoundRandomState,
    pub(crate) particle_runtime: Option<NativeParticleRuntime>,
    pub(crate) command_mailbox: SimulationMailbox,
    pub(crate) input_runtime: SimulationInputRuntime,
    pub(crate) level_load_runtime: LevelLoadRuntime,
    pub(crate) renderer_counters: RendererCounters,
    pub(crate) audio_counters: AudioCounters,
    pub(crate) publisher: SimulationPublisher,
    pub(crate) next_effect_sequence: u64,
}

/// Backend/platform borrows that still surround one synchronous simulation step.
///
/// These are deliberately not fields of [`SimulationCore`]. Phase five
/// replaces these resource and acceptance borrows with owned simulation
/// projections before the core moves threads.
pub(crate) struct SimulationStepContext<'a> {
    pub(crate) audio_enabled: bool,
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
    pub(crate) renderer: &'a mut Renderer,
    pub(crate) mailbox: &'a SimulationMailbox,
    pub(crate) terrain_upload: &'a mut TerrainUploadState,
    pub(crate) terrain_textures: &'a mut TerrainTextureState,
    pub(crate) entity_variant_textures: &'a mut NativeEntityVariantTextureRuntime,
    pub(crate) item_runtime: Option<&'a NativeItemRuntime>,
    pub(crate) hud_render_runtime: &'a mut HudRenderRuntime,
    pub(crate) hud_debug_fps_sampler: &'a mut HudDebugFpsSampler,
    pub(crate) debug_collision_cache: &'a mut DebugCollisionCache,
    pub(crate) debug_supporting_block_cache: &'a mut DebugSupportingBlockCache,
    pub(crate) debug_sky_light_section_cache: &'a mut DebugSkyLightSectionCache,
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
            effects::RenderEffectCollector::new(effect_sequence, self.audio_counters.clone());
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
            &mut self.renderer_counters,
            &mut self.audio_counters,
            &mut self.client_animation_ticks,
            &mut self.camera_fov_ticks,
            &mut self.hud_debug_network_sampler,
            &mut self.hud_debug_tps_sampler,
            &mut self.lightmap_ticks,
            &mut self.level_event_sound_random,
            context.terrain_textures,
            context.item_runtime,
            context.code_of_conduct,
            commands,
            &mut self.input_runtime,
            &mut self.level_load_runtime,
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
        let tick_anchor = frame.tick_anchor();
        let publication = self.publisher.publish(SimulationPublicationInput {
            world: &self.world,
            input: &self.input,
            input_runtime: self.input_runtime.publication(),
            frame: Some(frame),
            client_tick_index: self.lightmap_ticks.client_tick_count(),
            tick_interval: crate::runtime::world_client_tick_interval(&self.world),
            tick_residual: self.client_animation_ticks.entity_partial_tick(),
            tick_anchor,
            running: self.input_runtime.running(),
            net_counters: &self.net_counters,
            renderer_counters: &self.renderer_counters,
            audio_counters: &self.audio_counters,
            camera_fov_ticks: self.camera_fov_ticks,
            hud_debug_network_sampler: &self.hud_debug_network_sampler,
            hud_debug_tps_sampler: &self.hud_debug_tps_sampler,
        });
        SimulationStepResult {
            effects: effects.finish(resource_reload),
            publication,
        }
    }
}

pub(crate) fn update_renderer_from_publication(
    publication: &SimulationPublication,
    context: SimulationRenderContext<'_>,
) {
    update_renderer(publication, context)
}

pub(crate) struct SimulationStepResult {
    effects: effects::RenderEffectBatch,
    publication: Arc<SimulationPublication>,
}

impl SimulationStepResult {
    pub(crate) fn consume_render_effects(
        self,
        consumer: &mut effects::RenderEffectConsumer,
        audio: Option<&mut dyn AudioEventSink>,
        render: &mut dyn effects::RenderEffectSink,
    ) -> Result<
        (
            Arc<SimulationPublication>,
            Option<effects::ResourceReloadEffect>,
        ),
        effects::RenderEffectSequenceError,
    > {
        let reload = consumer.consume(self.effects, audio, render)?;
        Ok((self.publication, reload))
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
            .split_once("pub(crate) fn update_renderer_from_publication(")
            .expect("advance_simulation end")
            .0;
        assert!(advance.contains("input_runtime.apply_commands("));
        assert!(!advance.contains("pump_control_input_requests("));
        assert!(!advance.contains("pump_control_net_requests("));
    }

    #[test]
    fn render_and_event_loop_cannot_read_the_live_simulation_owner() {
        let main = include_str!("main.rs");
        assert_eq!(
            main.matches("simulation.step(SimulationStepContext")
                .count(),
            2
        );
        for forbidden in [
            "simulation.world",
            "simulation.input",
            "simulation.net_counters",
            "simulation.net_commands",
            "simulation.client_animation_ticks",
            "simulation.update_renderer",
        ] {
            assert!(
                !main.contains(forbidden),
                "found live-owner read: {forbidden}"
            );
        }

        let runtime = include_str!("runtime.rs");
        let render = runtime
            .split_once("pub(crate) fn update_renderer_from_publication(")
            .expect("publication render entry")
            .1
            .split_once("fn renderer_particle_block_fluid_surface_sample(")
            .expect("publication render end")
            .0;
        assert!(render.contains("publication.world.as_ref()"));
        assert!(render.contains("publication.input.as_ref().clone()"));
        assert!(!render.contains("&mut WorldStore"));
    }
}
