//! Native client simulation ownership.
//!
//! Phases one through four kept this owner on the winit thread. Phase five
//! gives it owned simulation resource projections and moves this same core to
//! the dedicated thread without introducing a second mutation path.

use std::time::{Duration, Instant};

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
pub(crate) mod thread;

use crate::{
    audio_runtime::AudioEventSink,
    code_of_conduct::CodeOfConductAcceptance,
    entity_variant_textures::NativeEntityVariantTextureRuntime,
    environment_animate_runtime::EnvironmentAnimateRuntime,
    environment_audio_runtime::EnvironmentAudioRuntime,
    input::ClientInputState,
    level_load_runtime::LevelLoadRuntime,
    music_runtime::MusicRuntime,
    particle_runtime::NativeParticleRuntime,
    remote_debug::RemoteDebugSelectionCache,
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
/// Fields stay crate-visible while construction remains in `main`, but only
/// the named simulation executor calls [`SimulationCore::step`]. Winit/control
/// mutation enters through the typed command mailbox and every render/control
/// read uses the immutable publication sealed by that step.
pub(crate) struct SimulationCore {
    pub(crate) net_events: Option<mpsc::Receiver<NetEvent>>,
    pub(crate) net_commands: Option<mpsc::Sender<NetCommand>>,
    pub(crate) server_resource_packs: ServerResourcePackRuntime,
    pub(crate) input: ClientInputState,
    pub(crate) world: WorldStore,
    pub(crate) net_counters: NetCounters,
    pub(crate) client_animation_ticks: ClientAnimationTickState,
    pub(crate) camera_fov_ticks: CameraFovTickState,
    pub(crate) camera_spinning_ticks: crate::runtime::CameraSpinningEffectTickState,
    pub(crate) hud_debug_network_sampler: HudDebugNetworkSampler,
    pub(crate) hud_debug_tps_sampler: HudDebugTpsSampler,
    pub(crate) lightmap_ticks: LightmapTickState,
    pub(crate) level_event_sound_random: LevelEventSoundRandomState,
    pub(crate) environment_audio: EnvironmentAudioRuntime,
    pub(crate) environment_animate: EnvironmentAnimateRuntime,
    pub(crate) music: MusicRuntime,
    pub(crate) win_screen_resources: crate::win_screen_resources::WinScreenResources,
    pub(crate) particle_runtime: Option<NativeParticleRuntime>,
    pub(crate) terrain_textures: TerrainTextureState,
    pub(crate) item_runtime: Option<NativeItemRuntime>,
    pub(crate) code_of_conduct: CodeOfConductAcceptance,
    pub(crate) command_mailbox: SimulationMailbox,
    pub(crate) input_runtime: SimulationInputRuntime,
    pub(crate) level_load_runtime: LevelLoadRuntime,
    pub(crate) renderer_counters: RendererCounters,
    pub(crate) audio_counters: AudioCounters,
    pub(crate) publisher: SimulationPublisher,
    pub(crate) audio_enabled: bool,
    pub(crate) render_distance_chunks: u32,
    pub(crate) hide_lightning_flash: bool,
    pub(crate) next_effect_sequence: u64,
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
    pub(crate) entity_render_extraction: &'a mut crate::entity_scene::EntityRenderExtractionScratch,
    pub(crate) non_entity_render_extraction:
        &'a mut crate::runtime::NonEntityRenderExtractionScratch,
    pub(crate) item_runtime: Option<&'a NativeItemRuntime>,
    pub(crate) hud_render_runtime: &'a mut HudRenderRuntime,
    pub(crate) hud_debug_fps_sampler: &'a mut HudDebugFpsSampler,
    pub(crate) debug_collision_cache: &'a mut DebugCollisionCache,
    pub(crate) debug_supporting_block_cache: &'a mut DebugSupportingBlockCache,
    pub(crate) debug_sky_light_section_cache: &'a mut DebugSkyLightSectionCache,
    pub(crate) remote_debug_selection_cache: &'a mut RemoteDebugSelectionCache,
    pub(crate) render_distance_chunks: u32,
    pub(crate) client_framerate_limit: u32,
    pub(crate) client_vsync: bool,
    pub(crate) damage_tilt_strength: f32,
    pub(crate) entity_shadows: bool,
    pub(crate) hud_debug_profiler_chart: Option<HudDebugProfilerChart>,
}

impl SimulationCore {
    /// Runs one ordered network/command/tick pump at the scheduler's monotonic
    /// timestamp. Producer wakes without visible work return without creating
    /// an effect batch, publication generation, or render output.
    pub(crate) fn step(
        &mut self,
        now: Instant,
        tick_deadline_reached: bool,
    ) -> Option<SimulationStepResult> {
        let commands = self.command_mailbox.drain_through_barrier();
        let network_work_pending = self
            .net_events
            .as_ref()
            .is_some_and(|events| !events.is_empty() || events.is_closed());
        if !tick_deadline_reached && !network_work_pending && commands.is_empty() {
            self.net_counters.simulation_noop_steps =
                self.net_counters.simulation_noop_steps.saturating_add(1);
            return None;
        }
        self.net_counters.simulation_productive_steps = self
            .net_counters
            .simulation_productive_steps
            .saturating_add(1);
        if let Some(telemetry) = self.command_mailbox.take_backend_telemetry() {
            self.renderer_counters = telemetry.renderer;
            self.audio_counters = telemetry.audio;
            self.net_counters.simulation_render_wakes_sent = telemetry.render_wakes_sent;
            self.net_counters.simulation_render_wakes_coalesced = telemetry.render_wakes_coalesced;
            self.net_counters.simulation_output_backlog_current = telemetry.output_backlog_current;
            self.net_counters.simulation_output_backlog_peak = telemetry.output_backlog_peak;
        }
        let mut rebuild_world_snapshot = tick_deadline_reached
            || network_work_pending
            || commands
                .iter()
                .any(|command| command.command.requires_world_snapshot_rebuild());
        let next_effect_sequence = self.next_effect_sequence;
        let effects =
            effects::RenderEffectCollector::new(next_effect_sequence, self.audio_counters.clone());
        let mut audio_effects = effects.audio();
        let mut particle_batches = effects.particles();
        let mut platform_effects = effects.platform();
        let frame = advance_simulation(
            &mut self.net_events,
            &self.net_commands,
            &mut self.server_resource_packs,
            self.audio_enabled
                .then_some(&mut audio_effects as &mut dyn AudioEventSink),
            &mut self.particle_runtime,
            &mut particle_batches,
            &mut self.input,
            &mut self.world,
            &mut self.net_counters,
            &mut self.client_animation_ticks,
            &mut self.camera_fov_ticks,
            &mut self.camera_spinning_ticks,
            &mut self.hud_debug_network_sampler,
            &mut self.hud_debug_tps_sampler,
            &mut self.lightmap_ticks,
            &mut self.level_event_sound_random,
            &mut self.environment_audio,
            &mut self.environment_animate,
            &mut self.music,
            &mut self.win_screen_resources,
            &mut self.terrain_textures,
            &mut self.item_runtime,
            &mut self.code_of_conduct,
            commands,
            &mut self.input_runtime,
            &mut self.level_load_runtime,
            &mut platform_effects,
            self.render_distance_chunks,
            self.hide_lightning_flash,
            now,
        );
        // Duration rounding can let the timer convert a tick at the edge of a
        // conservatively classified input-only wake. Never reuse a world Arc
        // when the actual pump reports canonical tick mutation.
        rebuild_world_snapshot |= frame.advanced_ticks() > 0;
        drop((audio_effects, particle_batches, platform_effects));
        let resource_reload = self.server_resource_packs.take_reload().map(|request| {
            let (sound_events, jukebox_songs) = audio_registry_snapshot_from_world(&self.world);
            effects::ResourceReloadEffect {
                request,
                sound_events,
                jukebox_songs,
            }
        });
        let effects = effects.finish(resource_reload);
        let effect_sequence = if effects.is_some() {
            self.net_counters.simulation_effect_batches = self
                .net_counters
                .simulation_effect_batches
                .saturating_add(1);
            self.next_effect_sequence = self.next_effect_sequence.saturating_add(1);
            next_effect_sequence
        } else {
            self.net_counters.simulation_empty_effect_batches_suppressed = self
                .net_counters
                .simulation_empty_effect_batches_suppressed
                .saturating_add(1);
            next_effect_sequence.saturating_sub(1)
        };
        self.net_counters.simulation_publications =
            self.net_counters.simulation_publications.saturating_add(1);
        if rebuild_world_snapshot {
            self.net_counters.simulation_world_snapshot_rebuilds = self
                .net_counters
                .simulation_world_snapshot_rebuilds
                .saturating_add(1);
        } else {
            self.net_counters.simulation_world_snapshot_reuses = self
                .net_counters
                .simulation_world_snapshot_reuses
                .saturating_add(1);
        }
        let tick_anchor = frame.tick_anchor();
        let publication_input = SimulationPublicationInput {
            effect_sequence,
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
            code_of_conduct_accepted: self
                .code_of_conduct
                .current_world_acceptance_matches(&self.world),
        };
        let publication = if rebuild_world_snapshot {
            self.publisher.publish(publication_input)
        } else {
            self.publisher.publish_reusing_world(publication_input)
        };
        Some(SimulationStepResult {
            effects,
            running: publication.running,
        })
    }

    pub(crate) fn next_tick_deadline(&self, now: Instant) -> Instant {
        let residual = f64::from(self.client_animation_ticks.entity_partial_tick());
        let remaining = (1.0 - residual).clamp(0.0, 1.0);
        let delay = self
            .world_tick_interval()
            .mul_f64(remaining)
            .max(Duration::from_micros(1));
        now.checked_add(delay).unwrap_or(now)
    }

    fn world_tick_interval(&self) -> Duration {
        crate::runtime::world_client_tick_interval(&self.world)
    }

    pub(crate) fn running(&self) -> bool {
        self.input_runtime.running()
    }
}

pub(crate) fn update_renderer_from_publication(
    publication: &SimulationPublication,
    context: SimulationRenderContext<'_>,
) {
    update_renderer(publication, context)
}

pub(crate) struct SimulationStepResult {
    effects: Option<effects::RenderEffectBatch>,
    running: bool,
}

impl SimulationStepResult {
    pub(crate) fn running(&self) -> bool {
        self.running
    }

    pub(crate) fn consume_render_effects(
        self,
        consumer: &mut effects::RenderEffectConsumer,
        audio: Option<&mut dyn AudioEventSink>,
        render: &mut dyn effects::RenderEffectSink,
    ) -> Result<Option<effects::ResourceReloadEffect>, effects::RenderEffectSequenceError> {
        let Some(effects) = self.effects else {
            return Ok(None);
        };
        consumer.consume(effects, audio, render)
    }
}

#[cfg(test)]
mod tests {
    use super::{SimulationCore, SimulationStepResult};

    #[test]
    fn core_and_step_output_cross_the_thread_boundary_without_shared_mutation() {
        fn assert_send<T: Send>() {}
        assert_send::<SimulationCore>();
        assert_send::<SimulationStepResult>();
    }

    #[test]
    fn noop_guard_precedes_effect_collection_and_publication() {
        let source = include_str!("simulation.rs");
        let step = source
            .split_once("pub(crate) fn step(")
            .expect("step")
            .1
            .split_once("pub(crate) fn next_tick_deadline")
            .expect("step end")
            .0;
        let noop = step.find("return None;").expect("no-op return");
        assert!(noop < step.find("RenderEffectCollector::new").unwrap());
        assert!(noop < step.find("self.publisher.publish(").unwrap());
    }

    #[test]
    fn chat_narration_enters_the_lossless_effect_gate_before_publication() {
        let runtime = include_str!("runtime.rs");
        let advance = runtime
            .split_once("pub(crate) fn advance_simulation(")
            .expect("advance_simulation")
            .1
            .split_once("pub(crate) fn update_renderer_from_publication(")
            .expect("advance_simulation end")
            .0;
        let narration = advance
            .find("world.take_pending_chat_narrations()")
            .expect("chat narration drain");
        assert!(advance[narration..].contains("SimulationFrameState {"));
        assert!(advance[narration..].contains("PlatformEffect::ChatNarration"));

        let step = include_str!("simulation.rs")
            .split_once("pub(crate) fn step(")
            .expect("step")
            .1
            .split_once("pub(crate) fn next_tick_deadline")
            .expect("step end")
            .0;
        assert!(
            step.find("effects.finish(").unwrap() < step.find("self.publisher.publish(").unwrap()
        );
    }

    #[test]
    fn queued_light_poll_follows_the_single_network_drain_before_presentation_sync() {
        let runtime = include_str!("runtime.rs");
        let advance = runtime
            .split_once("pub(crate) fn advance_simulation(")
            .expect("advance_simulation")
            .1
            .split_once("pub(crate) fn update_renderer_from_publication(")
            .expect("advance_simulation end")
            .0;
        let drain = advance
            .find("events::drain_net_events_with_sinks_and_debug(")
            .expect("single network drain");
        let light_poll = advance
            .find("world.poll_queued_light_updates();")
            .expect("queued light poll");
        let presentation = advance
            .find("server_resource_packs.synchronize(")
            .expect("first presentation synchronization");
        assert_eq!(
            advance
                .matches("events::drain_net_events_with_sinks_and_debug(")
                .count(),
            1
        );
        assert!(drain < light_poll);
        assert!(light_poll < presentation);
    }

    #[test]
    fn main_spawns_one_named_simulation_owner_and_only_consumes_its_outputs() {
        let main = include_str!("main.rs");
        let thread = include_str!("simulation/thread.rs");
        assert!(main.contains("let simulation = SimulationCore"));
        assert_eq!(main.matches("SimulationThread::spawn(").count(), 1);
        assert_eq!(main.matches(".consume_render_effects(").count(), 1);
        assert!(!main.contains("SimulationStepContext"));
        assert!(!main.contains("simulation.step("));
        assert!(!main.contains("snapshot_is_running"));
        assert_eq!(
            thread
                .matches("core.step(now, tick_deadline_reached)")
                .count(),
            1
        );
        assert!(thread.contains(".name(\"bbb-simulation\".to_string())"));
        assert!(!main.contains("pump_network_and_terrain("));
        assert!(!main.contains(".server_resource_packs.take_reload("));
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
        assert_eq!(main.matches("update_renderer_from_publication(").count(), 1);
        assert!(main.contains(
            "latest_publication.effect_sequence <= render_effect_consumer.last_sequence()"
        ));
        for forbidden in [
            "simulation.world",
            "simulation.input",
            "simulation.net_counters",
            "simulation.net_commands",
            "simulation.client_animation_ticks",
            "simulation.update_renderer",
            "simulation.step",
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
        assert!(render.contains("let input = publication.input.as_state();"));
        assert!(!render.contains("let mut render_input = publication.input.as_ref().clone()"));
        assert!(!render.contains("must contain a frame\")\n        .clone()"));
        assert!(!render.contains("&mut WorldStore"));

        let publication = include_str!("simulation/publication.rs");
        assert!(publication.contains("input.input.render_publication()"));
        assert!(!publication.contains("input.input.clone()"));
    }
}
