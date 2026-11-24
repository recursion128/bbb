//! The client simulation owner, stepped and rendered by the one event loop.

use bbb_client_clock::Instant;
use std::{sync::Arc, time::Duration};

use crate::control::{AudioCounters, NetCounters};
use bbb_client_net::{NetCommand, NetEvent};
use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_render::{HudDebugProfilerChart, Renderer};
use bbb_client_world::WorldStore;
use tokio::sync::mpsc;

pub(crate) mod commands;
pub(crate) mod effects;
pub(crate) mod input_commands;
pub(crate) mod publication;

use crate::terrain_upload::{
    synchronize_terrain_biome_registry, TerrainPrioritizeChunkUpdates, TerrainUploadState,
};
use crate::{
    audio_runtime::AudioEventSink,
    camera_fov::CameraFovTickState,
    code_of_conduct::CodeOfConductAcceptance,
    entity_variant_textures::NativeEntityVariantTextureRuntime,
    environment_animate_runtime::EnvironmentAnimateRuntime,
    environment_audio_runtime::EnvironmentAudioRuntime,
    input::ClientInputState,
    item_models::FirstPersonHandPresentation,
    level_load_runtime::LevelLoadRuntime,
    music_runtime::MusicRuntime,
    particle_runtime::NativeParticleRuntime,
    remote_debug::RemoteDebugSelectionCache,
    resource_pack_runtime::ServerResourcePackRuntime,
    runtime::{
        advance_simulation, audio_registry_snapshot_from_world, ClientAnimationTickState,
        DebugCollisionCache, DebugSkyLightSectionCache, DebugSupportingBlockCache,
        HudDebugFpsSampler, HudDebugNetworkSampler, HudDebugTpsSampler, HudRenderRuntime,
        LevelEventSoundRandomState, LightmapTickState, SimulationFrameState,
    },
};
use bbb_client_render::scene::terrain_textures::TerrainTextureState;
use commands::{CommandQueue, SimulationCommand};
use effects::{PlatformEffects, ResourceReloadEffect};
use input_commands::{AppliedInputLatency, SimulationInputPublication, SimulationInputRuntime};

/// The single mutable owner of canonical client state. The event loop steps it
/// for due ticks and queued commands, drains the effects it collected, and
/// renders from borrowed views of it.
pub(crate) struct SimulationCore {
    pub(crate) net_events: Option<mpsc::Receiver<NetEvent>>,
    pub(crate) net_commands: Option<mpsc::Sender<NetCommand>>,
    pub(crate) server_resource_packs: ServerResourcePackRuntime,
    pub(crate) input: ClientInputState,
    pub(crate) first_person_hand_presentation: FirstPersonHandPresentation,
    pub(crate) world: WorldStore,
    pub(crate) net_counters: NetCounters,
    pub(crate) client_animation_ticks: ClientAnimationTickState,
    pub(crate) camera_fov_ticks: CameraFovTickState,
    pub(crate) camera_eye_height_ticks: crate::camera_pose::CameraEyeHeightTickState,
    pub(crate) camera_spinning_ticks: crate::runtime::CameraSpinningEffectTickState,
    pub(crate) hud_debug_network_sampler: HudDebugNetworkSampler,
    pub(crate) hud_debug_tps_sampler: HudDebugTpsSampler,
    pub(crate) lightmap_ticks: LightmapTickState,
    pub(crate) level_event_sound_random: LevelEventSoundRandomState,
    pub(crate) environment_audio: EnvironmentAudioRuntime,
    pub(crate) environment_animate: EnvironmentAnimateRuntime,
    pub(crate) weather_effects: crate::weather_effect_runtime::WeatherEffectRuntime,
    pub(crate) music: MusicRuntime,
    pub(crate) win_screen_resources: Arc<crate::win_screen_resources::WinScreenResources>,
    pub(crate) particle_runtime: Option<NativeParticleRuntime>,
    /// Live particles; ticked with the client tick, extracted per frame.
    pub(crate) particles: bbb_client_world::particles::ParticleEngine,
    pub(crate) terrain_textures: TerrainTextureState,
    pub(crate) item_runtime: Option<NativeItemRuntime>,
    pub(crate) code_of_conduct: CodeOfConductAcceptance,
    pub(crate) server_list: Option<crate::server_list::ServerListPersistence>,
    pub(crate) input_runtime: SimulationInputRuntime,
    pub(crate) level_load_runtime: LevelLoadRuntime,
    /// Audio status shown when no audio backend exists.
    pub(crate) audio_counters: AudioCounters,
    pub(crate) hide_lightning_flash: bool,
    pub(crate) platform_effects: PlatformEffects,
    /// Frame state of the last step; render reads it until the next step.
    pub(crate) frame: Option<SimulationFrameState>,
    /// Bumped by every step that may have mutated the world.
    pub(crate) world_revision: u64,
    pub(crate) terrain_textures_changed: bool,
    pub(crate) synced_content_registry: Option<(
        Arc<bbb_protocol::content_registry::ContentRegistry>,
        Option<u64>,
    )>,
    pub(crate) launched_idle: bool,
    pub(crate) next_tick_deadline: Instant,
}

/// What a productive step changed.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StepOutcome {
    pub(crate) world_changed: bool,
}

/// Vanilla `AttackIndicatorStatus` as the render step consumes it: where
/// `Gui` draws the attack-strength charge, or nowhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttackIndicatorOption {
    Off,
    Crosshair,
    Hotbar,
}

/// The simulation state a frame is extracted from, borrowed for the frame.
pub(crate) struct SimulationView<'a> {
    pub(crate) world: &'a WorldStore,
    pub(crate) particles: &'a bbb_client_world::particles::ParticleEngine,
    pub(crate) particle_runtime: Option<&'a NativeParticleRuntime>,
    pub(crate) input: &'a ClientInputState,
    pub(crate) input_runtime: SimulationInputPublication,
    pub(crate) first_person_hand_presentation: &'a FirstPersonHandPresentation,
    pub(crate) win_screen_resources: &'a Arc<crate::win_screen_resources::WinScreenResources>,
    pub(crate) frame: &'a SimulationFrameState,
    pub(crate) client_tick_index: u64,
    pub(crate) world_revision: u64,
    pub(crate) net_counters: &'a NetCounters,
    pub(crate) audio_counters: &'a AudioCounters,
    pub(crate) camera_fov_ticks: CameraFovTickState,
    pub(crate) camera_eye_height_ticks: crate::camera_pose::CameraEyeHeightTickState,
    pub(crate) hud_debug_network_sampler: &'a HudDebugNetworkSampler,
    pub(crate) hud_debug_tps_sampler: &'a HudDebugTpsSampler,
    pub(crate) tick_interval: Duration,
    pub(crate) tick_residual: f32,
    pub(crate) terrain_textures_changed: bool,
    pub(crate) idle_mode: bool,
}

impl SimulationView<'_> {
    /// Projects the timer residual of the last step to `now`. A frame drawn
    /// after the next tick was due clamps rather than extrapolating.
    pub(crate) fn partial_tick_at(&self, now: Instant) -> f32 {
        partial_tick_since(
            self.world.ticking().frozen,
            self.tick_residual,
            self.frame.tick_anchor(),
            self.tick_interval,
            now,
        )
    }
}

fn partial_tick_since(
    frozen: bool,
    residual: f32,
    anchor: Instant,
    interval: Duration,
    now: Instant,
) -> f32 {
    // `DeltaTracker.Timer.getGameTimeDeltaPartialTick(false)` returns one
    // while the level tick-rate manager is frozen (vanilla 26.1
    // `DeltaTracker.java`), regardless of the carried timer residual.
    if frozen {
        return 1.0;
    }
    let interval_seconds = interval.as_secs_f32();
    let advanced = if interval_seconds > 0.0 {
        now.saturating_duration_since(anchor).as_secs_f32() / interval_seconds
    } else {
        0.0
    };
    (residual + advanced).clamp(0.0, 1.0)
}

/// Borrows split out of the core for one frame.
pub(crate) struct RenderParts<'a> {
    pub(crate) view: SimulationView<'a>,
    pub(crate) terrain_textures: &'a mut TerrainTextureState,
    pub(crate) item_runtime: Option<&'a NativeItemRuntime>,
    pub(crate) code_of_conduct_accepted: bool,
}

/// Render-owner state and options a frame needs beside the simulation view.
pub(crate) struct SimulationRenderContext<'a> {
    /// Client tick count of the last rendered frame. Render-side tick
    /// consumers advance by the difference, so a state drawn twice neither
    /// repeats nor drops ticks.
    pub(crate) rendered_client_tick: &'a mut Option<u64>,
    pub(crate) gui_cursor_position: Option<winit::dpi::PhysicalPosition<f64>>,
    pub(crate) audio_events: Option<&'a mut dyn AudioEventSink>,
    pub(crate) renderer: &'a mut Renderer,
    pub(crate) commands: &'a CommandQueue,
    pub(crate) input_latency: Option<AppliedInputLatency>,
    pub(crate) terrain_upload: &'a mut TerrainUploadState,
    pub(crate) lod_terrain: &'a mut crate::lod_terrain::LodTerrainRuntime,
    pub(crate) terrain_textures: &'a mut TerrainTextureState,
    pub(crate) entity_variant_textures: &'a mut NativeEntityVariantTextureRuntime,
    pub(crate) entity_render_extraction: &'a mut crate::entity_scene::EntityRenderExtractionScratch,
    pub(crate) non_entity_render_extraction:
        &'a mut crate::runtime::NonEntityRenderExtractionScratch,
    pub(crate) camera_environment_samples: &'a mut crate::runtime::CameraEnvironmentSampleCache,
    pub(crate) item_runtime: Option<&'a NativeItemRuntime>,
    pub(crate) hud_render_runtime: &'a mut HudRenderRuntime,
    pub(crate) hud_debug_fps_sampler: &'a mut HudDebugFpsSampler,
    pub(crate) debug_collision_cache: &'a mut DebugCollisionCache,
    pub(crate) debug_supporting_block_cache: &'a mut DebugSupportingBlockCache,
    pub(crate) debug_sky_light_section_cache: &'a mut DebugSkyLightSectionCache,
    pub(crate) remote_debug_selection_cache: &'a mut RemoteDebugSelectionCache,
    pub(crate) render_distance_chunks: u32,
    pub(crate) prioritize_chunk_updates: TerrainPrioritizeChunkUpdates,
    pub(crate) entity_distance_scaling: f64,
    pub(crate) weather_radius: u32,
    pub(crate) cloud_range_chunks: u32,
    pub(crate) client_framerate_limit: u32,
    pub(crate) client_vsync: bool,
    pub(crate) damage_tilt_strength: f32,
    pub(crate) view_bobbing: bool,
    pub(crate) vignette: bool,
    pub(crate) attack_indicator: AttackIndicatorOption,
    pub(crate) screen_effect_scale: f32,
    pub(crate) notification_display_time: f32,
    pub(crate) music_toast_display: crate::startup::MusicToastDisplayStateArg,
    pub(crate) final_music_volume: f32,
    pub(crate) entity_shadows: bool,
    pub(crate) high_contrast_block_outline: bool,
    pub(crate) improved_transparency: bool,
    pub(crate) dynamic_lights_mode: bbb_client_render::DynamicLightsMode,
    pub(crate) lod_distance_chunks: u32,
    pub(crate) extensions: crate::qol_extensions::ClientExtensionSet,
    pub(crate) hud_debug_profiler_chart: Option<HudDebugProfilerChart>,
}

impl SimulationCore {
    /// Runs one network/command/tick step at `now`. Without a due tick,
    /// pending network event or command the step does nothing and returns
    /// `None`.
    pub(crate) fn step(
        &mut self,
        now: Instant,
        commands: Vec<SimulationCommand>,
        audio: Option<&mut dyn AudioEventSink>,
    ) -> Option<StepOutcome> {
        let tick_due = now >= self.next_tick_deadline;
        let network_work_pending = self.network_work_pending();
        if !tick_due && !network_work_pending && commands.is_empty() {
            return None;
        }
        let cursor_drag_may_mutate_world = self.input_runtime.active_mouse_button().is_some()
            && matches!(
                self.input.current_screen(),
                Some(
                    crate::input::Screen::Container(_)
                        | crate::input::Screen::Creative(_)
                        | crate::input::Screen::Dialog(_)
                        | crate::input::Screen::DialogWarning(_)
                        | crate::input::Screen::DialogLink(_)
                        | crate::input::Screen::DialogWaiting(_)
                )
            );
        let commands_change_world = commands.iter().any(|command| {
            command.requires_world_snapshot_rebuild()
                || (cursor_drag_may_mutate_world
                    && matches!(command, SimulationCommand::CursorMoved { .. }))
        });
        let frame = advance_simulation(
            crate::runtime::SimulationStepContext {
                net_events: &mut self.net_events,
                net_commands: &self.net_commands,
                server_resource_packs: &mut self.server_resource_packs,
                particle_runtime: &mut self.particle_runtime,
                particles: &mut self.particles,
                input: &mut self.input,
                first_person_hand_presentation: &mut self.first_person_hand_presentation,
                world: &mut self.world,
                net_counters: &mut self.net_counters,
                client_animation_ticks: &mut self.client_animation_ticks,
                camera_fov_ticks: &mut self.camera_fov_ticks,
                camera_eye_height_ticks: &mut self.camera_eye_height_ticks,
                camera_spinning_ticks: &mut self.camera_spinning_ticks,
                hud_debug_network_sampler: &mut self.hud_debug_network_sampler,
                hud_debug_tps_sampler: &mut self.hud_debug_tps_sampler,
                lightmap_ticks: &mut self.lightmap_ticks,
                level_event_sound_random: &mut self.level_event_sound_random,
                environment_audio: &mut self.environment_audio,
                environment_animate: &mut self.environment_animate,
                weather_effects: &mut self.weather_effects,
                music: &mut self.music,
                win_screen_resources: &mut self.win_screen_resources,
                terrain_textures: &mut self.terrain_textures,
                item_runtime: &mut self.item_runtime,
                code_of_conduct: &mut self.code_of_conduct,
                server_list: &mut self.server_list,
                commands,
                input_runtime: &mut self.input_runtime,
                level_load_runtime: &mut self.level_load_runtime,
                platform_effects: &mut self.platform_effects,
                hide_lightning_flash: self.hide_lightning_flash,
                now,
            },
            audio,
        );
        // Duration rounding can let the timer convert a tick at the edge of
        // an input-only step; a tick always invalidates the world revision.
        let world_changed = tick_due
            || commands_change_world
            || frame.network_work_applied
            || frame.advanced_ticks() > 0;
        if world_changed {
            self.world_revision = self.world_revision.wrapping_add(1);
        }
        self.terrain_textures_changed |= frame.terrain_textures_changed();
        self.next_tick_deadline = self.next_tick_deadline_after(now);
        self.frame = Some(frame);
        Some(StepOutcome { world_changed })
    }

    pub(crate) fn network_work_pending(&self) -> bool {
        self.net_events
            .as_ref()
            .is_some_and(|events| !events.is_empty() || events.is_closed())
    }

    fn next_tick_deadline_after(&self, now: Instant) -> Instant {
        let residual = f64::from(self.client_animation_ticks.entity_partial_tick());
        let remaining = (1.0 - residual).clamp(0.0, 1.0);
        let delay = crate::runtime::world_client_tick_interval(&self.world)
            .mul_f64(remaining)
            .max(Duration::from_micros(1));
        now.checked_add(delay).unwrap_or(now)
    }

    /// The resource reload the last step left pending, with the registries
    /// the reloaded audio backend starts from.
    pub(crate) fn take_resource_reload(&mut self) -> Option<ResourceReloadEffect> {
        let request = self.server_resource_packs.take_reload()?;
        let (sound_events, jukebox_songs) = audio_registry_snapshot_from_world(&self.world);
        Some(ResourceReloadEffect {
            request,
            sound_events,
            jukebox_songs,
        })
    }

    /// Binds terrain textures replaced by a resource reload to the world's
    /// biome registry before the next frame meshes with them.
    pub(crate) fn resynchronize_terrain_textures(&mut self) {
        if synchronize_terrain_biome_registry(&mut self.terrain_textures, &self.world) {
            self.terrain_textures_changed = true;
            if let Some(particles) = self.particle_runtime.as_mut() {
                particles.set_terrain_biomes(&self.terrain_textures);
            }
        }
    }

    pub(crate) fn invalidate_content_registry_sync(&mut self) {
        self.synced_content_registry = None;
    }

    /// Synchronizes session `ContentRegistry` mod blocks, items, and particles
    /// into terrain textures, item runtime, and particle runtime whenever the
    /// world's registry or active resource packs change.
    pub(crate) fn synchronize_content_registry(
        &mut self,
        pack_roots: Option<&bbb_client_assets::PackRoots>,
        renderer: Option<&mut Renderer>,
    ) -> bool {
        let current_registry = self.world.content_registry();
        let pack_generation = pack_roots.and_then(|roots| {
            roots
                .client_resource_generation()
                .map(|generation| generation.generation())
        });
        if let Some((synced_registry, synced_generation)) = &self.synced_content_registry {
            if Arc::ptr_eq(synced_registry, current_registry)
                && *synced_generation == pack_generation
            {
                return false;
            }
        }
        let registry = Arc::clone(current_registry);
        let projection = if registry.is_vanilla_only() {
            None
        } else {
            pack_roots
                .and_then(|roots| match roots.project_content_registry(&registry) {
                    Ok(projection) => Some(projection),
                    Err(err) => {
                        tracing::warn!(?err, "failed to project ContentRegistry pack assets");
                        None
                    }
                })
                .map(Arc::new)
        };
        let terrain_changed = self
            .terrain_textures
            .set_content_registry(Arc::clone(&registry), projection.clone());
        let items_changed = self.item_runtime.as_mut().is_some_and(|runtime| {
            runtime.sync_content_registry(Arc::clone(&registry), projection.clone())
        });
        if let Some(particles) = self.particle_runtime.as_mut() {
            particles.sync_content_registry(Arc::clone(&registry), projection.as_deref());
            if terrain_changed {
                particles.set_terrain_textures(&self.terrain_textures);
            }
            if items_changed {
                if let Some(items) = self.item_runtime.as_ref() {
                    particles.set_item_runtime(items);
                }
            }
        }
        if terrain_changed {
            self.terrain_textures_changed = true;
        }
        if items_changed {
            if let Some(renderer) = renderer {
                renderer.invalidate_item_model_resources();
            }
        }
        self.synced_content_registry = Some((registry, pack_generation));
        terrain_changed || items_changed
    }

    pub(crate) fn running(&self) -> bool {
        self.input_runtime.running()
    }

    pub(crate) fn client_tick_index(&self) -> u64 {
        self.lightmap_ticks.client_tick_count()
    }

    /// Splits the frame's borrows: the immutable view plus the render-shared
    /// terrain textures. `None` until the first step has run.
    pub(crate) fn render_parts(&mut self) -> Option<RenderParts<'_>> {
        let code_of_conduct_accepted = self
            .code_of_conduct
            .current_world_acceptance_matches(&self.world);
        let tick_interval = crate::runtime::world_client_tick_interval(&self.world);
        let terrain_textures_changed = std::mem::take(&mut self.terrain_textures_changed);
        let client_tick_index = self.client_tick_index();
        let idle_mode = self.launched_idle;
        let Self {
            world,
            particles,
            particle_runtime,
            input,
            input_runtime,
            first_person_hand_presentation,
            win_screen_resources,
            frame,
            world_revision,
            net_counters,
            audio_counters,
            camera_fov_ticks,
            camera_eye_height_ticks,
            hud_debug_network_sampler,
            hud_debug_tps_sampler,
            client_animation_ticks,
            terrain_textures,
            item_runtime,
            ..
        } = self;
        let frame = frame.as_ref()?;
        Some(RenderParts {
            view: SimulationView {
                world,
                particles,
                particle_runtime: particle_runtime.as_ref(),
                input,
                input_runtime: input_runtime.publication(),
                first_person_hand_presentation,
                win_screen_resources,
                frame,
                client_tick_index,
                world_revision: *world_revision,
                net_counters,
                audio_counters,
                camera_fov_ticks: *camera_fov_ticks,
                camera_eye_height_ticks: *camera_eye_height_ticks,
                hud_debug_network_sampler,
                hud_debug_tps_sampler,
                tick_interval,
                tick_residual: client_animation_ticks.entity_partial_tick(),
                terrain_textures_changed,
                idle_mode,
            },
            terrain_textures,
            item_runtime: item_runtime.as_ref(),
            code_of_conduct_accepted,
        })
    }
}

/// `--renderer-input-latency-diagnostics`: the newest platform input event is
/// stamped when it is queued and when a step applies it, and the stamp rides
/// the frames until one presents it.
#[derive(Debug, Default)]
pub(crate) struct InputLatencyTracker {
    enabled: bool,
    next_sequence: u64,
    queued: Option<(u64, Instant, u64)>,
    applied: Option<AppliedInputLatency>,
}

impl InputLatencyTracker {
    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            next_sequence: 1,
            ..Self::default()
        }
    }

    pub(crate) fn queued(&mut self, at: Instant) {
        if !self.enabled {
            return;
        }
        let superseded = self
            .queued
            .map_or(0, |(_, _, superseded)| superseded.saturating_add(1));
        self.queued = Some((self.next_sequence, at, superseded));
        self.next_sequence = self.next_sequence.saturating_add(1);
    }

    pub(crate) fn applied(&mut self, at: Instant) {
        let Some((sequence, enqueued_at, superseded)) = self.queued.take() else {
            return;
        };
        let previously_unpresented = self
            .applied
            .map_or(0, |previous| previous.superseded_events.saturating_add(1));
        self.applied = Some(AppliedInputLatency {
            sequence,
            enqueued_at,
            simulation_applied_at: at,
            superseded_events: superseded.saturating_add(previously_unpresented),
        });
    }

    pub(crate) fn pending(&self) -> Option<AppliedInputLatency> {
        self.applied
    }

    pub(crate) fn presented(&mut self, sequence: u64) {
        if self
            .applied
            .is_some_and(|applied| applied.sequence <= sequence)
        {
            self.applied = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpresented_input_latency_counts_replaced_events() {
        let start = Instant::now();
        let mut tracker = InputLatencyTracker::new(true);
        tracker.queued(start);
        tracker.queued(start + Duration::from_millis(1));
        tracker.applied(start + Duration::from_millis(2));
        let first = tracker.pending().expect("applied input");
        assert_eq!(first.sequence, 2);
        assert_eq!(first.superseded_events, 1);

        tracker.queued(start + Duration::from_millis(3));
        tracker.applied(start + Duration::from_millis(4));
        let replaced = tracker.pending().expect("replacing input");
        assert_eq!(replaced.sequence, 3);
        assert_eq!(replaced.superseded_events, 2);

        tracker.presented(3);
        assert!(tracker.pending().is_none());
        tracker.queued(start);
        assert!(InputLatencyTracker::new(false).pending().is_none());
    }

    #[test]
    fn partial_tick_projection_clamps_delayed_frames() {
        let now = Instant::now();
        let interval = Duration::from_millis(50);
        assert_eq!(partial_tick_since(false, 0.25, now, interval, now), 0.25);
        assert_eq!(
            partial_tick_since(false, 0.25, now, interval, now + Duration::from_millis(25)),
            0.75
        );
        assert_eq!(
            partial_tick_since(false, 0.25, now, interval, now + Duration::from_millis(100)),
            1.0
        );
        assert_eq!(partial_tick_since(true, 0.25, now, interval, now), 1.0);
    }
}
