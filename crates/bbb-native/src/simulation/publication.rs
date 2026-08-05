//! Immutable simulation generations shared by render and control readers.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use arc_swap::ArcSwap;
use bbb_control::{
    AudioCounters, ControlSnapshot, ControlWorldSnapshot, NetCounters, RendererCounters,
    SharedSnapshot, SharedWorld,
};
use bbb_world::WorldStore;

use super::input_commands::SimulationInputPublication;
use crate::{
    camera_fov::CameraFovTickState,
    camera_pose::CameraEyeHeightTickState,
    input::{ClientInputState, RenderInputPublication},
    item_models::FirstPersonHandPresentation,
    runtime::{HudDebugNetworkSampler, HudDebugTpsSampler, SimulationFrameState},
};

pub(crate) type SharedSimulationPublication = Arc<ArcSwap<SimulationPublication>>;

#[derive(Debug)]
pub(crate) struct SimulationPublication {
    pub(crate) generation: u64,
    pub(crate) effect_sequence: u64,
    pub(crate) client_tick_index: u64,
    pub(crate) world: Arc<WorldStore>,
    pub(crate) input: Arc<RenderInputPublication>,
    pub(crate) first_person_hand_presentation: FirstPersonHandPresentation,
    pub(crate) win_screen_resources: Arc<crate::win_screen_resources::WinScreenResources>,
    pub(crate) input_runtime: SimulationInputPublication,
    pub(crate) frame: Option<SimulationFrameState>,
    pub(crate) tick_interval: Duration,
    pub(crate) tick_residual: f32,
    pub(crate) tick_anchor: Instant,
    pub(crate) frozen: bool,
    pub(crate) running: bool,
    pub(crate) code_of_conduct_accepted: bool,
    pub(crate) terrain_revision: u64,
    pub(crate) client_level_generation: u64,
    pub(crate) net_counters: NetCounters,
    pub(crate) renderer_counters: RendererCounters,
    pub(crate) audio_counters: AudioCounters,
    pub(crate) camera_fov_ticks: CameraFovTickState,
    pub(crate) camera_eye_height_ticks: CameraEyeHeightTickState,
    pub(crate) hud_debug_network_sampler: HudDebugNetworkSampler,
    pub(crate) hud_debug_tps_sampler: HudDebugTpsSampler,
}

impl SimulationPublication {
    /// Projects the existing vanilla timer residual forward without mutating
    /// simulation. A renderer delayed beyond the next tick clamps rather than
    /// extrapolating canonical state.
    pub(crate) fn partial_tick_at(&self, now: Instant) -> f32 {
        // `DeltaTracker.Timer.getGameTimeDeltaPartialTick(false)` returns one
        // while the level tick-rate manager is frozen (vanilla 26.1
        // `DeltaTracker.java`), regardless of the carried timer residual.
        if self.frozen {
            return 1.0;
        }
        let elapsed = now.saturating_duration_since(self.tick_anchor);
        let interval_seconds = self.tick_interval.as_secs_f32();
        let advanced = if interval_seconds > 0.0 {
            elapsed.as_secs_f32() / interval_seconds
        } else {
            0.0
        };
        (self.tick_residual + advanced).clamp(0.0, 1.0)
    }
}

pub(crate) struct SimulationPublisher {
    shared: SharedSimulationPublication,
    control_snapshot: SharedSnapshot,
    control_world: SharedWorld,
    next_generation: u64,
    publication_build_peak: Duration,
    entity_snapshot_build_peak: Duration,
}

pub(crate) struct SimulationPublicationInput<'a> {
    pub(crate) effect_sequence: u64,
    pub(crate) world: &'a WorldStore,
    pub(crate) input: &'a ClientInputState,
    pub(crate) first_person_hand_presentation: FirstPersonHandPresentation,
    pub(crate) win_screen_resources: Arc<crate::win_screen_resources::WinScreenResources>,
    pub(crate) input_runtime: SimulationInputPublication,
    pub(crate) frame: Option<SimulationFrameState>,
    pub(crate) client_tick_index: u64,
    pub(crate) tick_interval: Duration,
    pub(crate) tick_residual: f32,
    pub(crate) tick_anchor: Instant,
    pub(crate) running: bool,
    pub(crate) code_of_conduct_accepted: bool,
    pub(crate) net_counters: &'a NetCounters,
    pub(crate) renderer_counters: &'a RendererCounters,
    pub(crate) audio_counters: &'a AudioCounters,
    pub(crate) camera_fov_ticks: CameraFovTickState,
    pub(crate) camera_eye_height_ticks: CameraEyeHeightTickState,
    pub(crate) hud_debug_network_sampler: &'a HudDebugNetworkSampler,
    pub(crate) hud_debug_tps_sampler: &'a HudDebugTpsSampler,
}

impl SimulationPublisher {
    pub(crate) fn new(
        initial: SimulationPublicationInput<'_>,
        control_snapshot: SharedSnapshot,
        control_world: SharedWorld,
    ) -> Self {
        let initial_generation = control_snapshot.load().generation;
        let (publication, publication_build_peak, entity_snapshot_build_peak) = build_publication(
            initial_generation,
            initial,
            None,
            Duration::ZERO,
            Duration::ZERO,
        );
        publish_control_views(&control_snapshot, &control_world, &publication);
        let shared = Arc::new(ArcSwap::from(publication));
        Self {
            shared,
            control_snapshot,
            control_world,
            next_generation: initial_generation.saturating_add(1),
            publication_build_peak,
            entity_snapshot_build_peak,
        }
    }

    pub(crate) fn shared(&self) -> SharedSimulationPublication {
        Arc::clone(&self.shared)
    }

    pub(crate) fn publish(
        &mut self,
        input: SimulationPublicationInput<'_>,
    ) -> Arc<SimulationPublication> {
        self.publish_with_world(input, None)
    }

    /// Publishes diagnostics/input state while retaining the exact immutable
    /// world Arc from the previous generation.
    pub(crate) fn publish_reusing_world(
        &mut self,
        input: SimulationPublicationInput<'_>,
    ) -> Arc<SimulationPublication> {
        let world = Arc::clone(&self.shared.load().world);
        self.publish_with_world(input, Some(world))
    }

    fn publish_with_world(
        &mut self,
        input: SimulationPublicationInput<'_>,
        reused_world: Option<Arc<WorldStore>>,
    ) -> Arc<SimulationPublication> {
        let generation = self.next_generation;
        self.next_generation = self.next_generation.saturating_add(1);
        let (publication, publication_build_peak, entity_snapshot_build_peak) = build_publication(
            generation,
            input,
            reused_world,
            self.publication_build_peak,
            self.entity_snapshot_build_peak,
        );
        self.publication_build_peak = publication_build_peak;
        self.entity_snapshot_build_peak = entity_snapshot_build_peak;
        publish_control_views(&self.control_snapshot, &self.control_world, &publication);
        self.shared.store(Arc::clone(&publication));
        publication
    }
}

fn build_publication(
    generation: u64,
    input: SimulationPublicationInput<'_>,
    reused_world: Option<Arc<WorldStore>>,
    previous_build_peak: Duration,
    previous_entity_snapshot_build_peak: Duration,
) -> (Arc<SimulationPublication>, Duration, Duration) {
    let build_started_at = Instant::now();
    let (world, entity_snapshot_duration, rebuilt_entities) = match reused_world {
        Some(world) => (world, Duration::ZERO, 0),
        None => {
            let world_snapshot = input.world.publication_snapshot();
            let observation = world_snapshot.entity_publication_snapshot_observation();
            (
                Arc::new(world_snapshot),
                observation.duration,
                observation.rebuilt_entities,
            )
        }
    };
    let mut publication = Arc::new(SimulationPublication {
        generation,
        effect_sequence: input.effect_sequence,
        client_tick_index: input.client_tick_index,
        terrain_revision: world.terrain_revision(),
        client_level_generation: world.client_level_generation(),
        frozen: world.ticking().frozen,
        world,
        input: Arc::new(input.input.render_publication()),
        first_person_hand_presentation: input.first_person_hand_presentation,
        win_screen_resources: input.win_screen_resources,
        input_runtime: input.input_runtime,
        frame: input.frame,
        tick_interval: input.tick_interval,
        tick_residual: input.tick_residual,
        tick_anchor: input.tick_anchor,
        running: input.running,
        code_of_conduct_accepted: input.code_of_conduct_accepted,
        net_counters: input.net_counters.clone(),
        renderer_counters: input.renderer_counters.clone(),
        audio_counters: input.audio_counters.clone(),
        camera_fov_ticks: input.camera_fov_ticks,
        camera_eye_height_ticks: input.camera_eye_height_ticks,
        hud_debug_network_sampler: input.hud_debug_network_sampler.clone(),
        hud_debug_tps_sampler: input.hud_debug_tps_sampler.clone(),
    });
    let build_duration = build_started_at.elapsed();
    let build_peak = previous_build_peak.max(build_duration);
    let entity_snapshot_build_peak =
        previous_entity_snapshot_build_peak.max(entity_snapshot_duration);
    let net_counters = &mut Arc::get_mut(&mut publication)
        .expect("new simulation publication must have one owner")
        .net_counters;
    net_counters.simulation_publication_build_nanos = duration_nanos(build_duration);
    net_counters.simulation_publication_build_peak_nanos = duration_nanos(build_peak);
    net_counters.simulation_entity_snapshot_build_nanos = duration_nanos(entity_snapshot_duration);
    net_counters.simulation_entity_snapshot_build_peak_nanos =
        duration_nanos(entity_snapshot_build_peak);
    net_counters.simulation_entity_snapshot_rebuilt_entities = rebuilt_entities;
    (publication, build_peak, entity_snapshot_build_peak)
}

fn duration_nanos(duration: Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

fn publish_control_views(
    snapshot: &SharedSnapshot,
    published_world: &SharedWorld,
    publication: &Arc<SimulationPublication>,
) {
    published_world.store(Arc::new(ControlWorldSnapshot {
        generation: publication.generation,
        world: Arc::clone(&publication.world),
    }));
    snapshot.rcu(|current| {
        let mut app = current.app.clone();
        app.running = publication.running;
        app.disconnected_screen = publication.input.disconnected_screen_status();
        app.chat_screen = publication
            .input
            .chat_screen_status(publication.world.chat_presentation_config().visibility);
        app.death_screen = publication.input.death_screen_status(&publication.world);
        ControlSnapshot {
            generation: publication.generation,
            app,
            net: publication.net_counters.clone(),
            audio: publication.audio_counters.clone(),
            renderer: publication.renderer_counters.clone(),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        camera_fov::CameraFovTickState,
        runtime::{HudDebugNetworkSampler, HudDebugTpsSampler},
        simulation::input_commands::SimulationInputRuntime,
    };
    use bbb_protocol::packets::ClientInformation;
    use winit::{
        dpi::PhysicalSize,
        event::ElementState,
        keyboard::{KeyCode, PhysicalKey},
    };

    fn test_control_state() -> bbb_control::ControlState {
        bbb_control::ControlState::new(
            "test",
            bbb_control::SharedControlRequests::new(Arc::new(
                crate::simulation::commands::SimulationMailbox::default(),
            )),
        )
    }

    fn input<'a>(
        world: &'a WorldStore,
        client_input: &'a ClientInputState,
        input_runtime: &'a SimulationInputRuntime,
        net: &'a NetCounters,
        renderer: &'a RendererCounters,
        audio: &'a AudioCounters,
        network: &'a HudDebugNetworkSampler,
        tps: &'a HudDebugTpsSampler,
        now: Instant,
    ) -> SimulationPublicationInput<'a> {
        SimulationPublicationInput {
            effect_sequence: 0,
            world,
            input: client_input,
            first_person_hand_presentation: FirstPersonHandPresentation::default(),
            win_screen_resources: Arc::new(
                crate::win_screen_resources::WinScreenResources::default(),
            ),
            input_runtime: input_runtime.publication(),
            frame: None,
            client_tick_index: 0,
            tick_interval: Duration::from_millis(50),
            tick_residual: 0.25,
            tick_anchor: now,
            running: true,
            code_of_conduct_accepted: false,
            net_counters: net,
            renderer_counters: renderer,
            audio_counters: audio,
            camera_fov_ticks: CameraFovTickState::default(),
            camera_eye_height_ticks: CameraEyeHeightTickState::default(),
            hud_debug_network_sampler: network,
            hud_debug_tps_sampler: tps,
        }
    }

    #[test]
    fn publication_reuses_world_and_carries_play_session_security() {
        let control = test_control_state();
        let world = WorldStore::new();
        let client_input = ClientInputState::default();
        let input_runtime = SimulationInputRuntime::new(
            PhysicalSize::new(1280, 720),
            false,
            ClientInformation::default(),
        );
        let net = NetCounters::default();
        let renderer = RendererCounters::default();
        let audio = AudioCounters::default();
        let network = HudDebugNetworkSampler::default();
        let tps = HudDebugTpsSampler::default();
        let now = Instant::now();
        let initial = input(
            &world,
            &client_input,
            &input_runtime,
            &net,
            &renderer,
            &audio,
            &network,
            &tps,
            now,
        );
        let publisher = SimulationPublisher::new(
            initial,
            Arc::clone(&control.snapshot),
            Arc::clone(&control.world),
        );

        let render = publisher.shared().load_full();
        let control_world = control.world.load_full();
        let control_snapshot = control.snapshot.load_full();
        assert_eq!(render.generation, control_world.generation);
        assert_eq!(render.generation, control_snapshot.generation);
        assert!(Arc::ptr_eq(&render.world, &control_world.world));
    }

    #[test]
    fn disconnected_screen_diagnostics_are_bounded_immutable_publication() {
        let control = test_control_state();
        let world = WorldStore::new();
        let mut client_input = ClientInputState::default();
        client_input.open_disconnected_screen(
            crate::input::disconnected_screen::DisconnectedScreenTitle::ConnectionLost,
            crate::input::disconnected_screen::DisconnectedScreenDetails {
                reason: bbb_net::DisconnectReason::from_plain("x".repeat(300)),
                report_directory: None,
                bug_report_link: None,
            },
        );
        let input_runtime = SimulationInputRuntime::new(
            PhysicalSize::new(1280, 720),
            false,
            ClientInformation::default(),
        );
        let net = NetCounters::default();
        let renderer = RendererCounters::default();
        let audio = AudioCounters::default();
        let network = HudDebugNetworkSampler::default();
        let tps = HudDebugTpsSampler::default();
        let now = Instant::now();
        let initial = input(
            &world,
            &client_input,
            &input_runtime,
            &net,
            &renderer,
            &audio,
            &network,
            &tps,
            now,
        );
        let _publisher = SimulationPublisher::new(
            initial,
            Arc::clone(&control.snapshot),
            Arc::clone(&control.world),
        );

        let snapshot = control.snapshot.load_full();
        let status = snapshot.app.disconnected_screen.as_ref().unwrap();
        assert_eq!(status.title_translation_key, "disconnect.lost");
        assert_eq!(status.reason_preview.len(), 256);
        assert!(status.reason_truncated);
        assert_eq!(status.reason_retained_bytes, 300);
    }

    #[test]
    fn chat_screen_status_is_bounded_and_published_with_its_world_permissions() {
        let control = test_control_state();
        let mut world = WorldStore::new();
        let mut chat = world.chat_presentation_config().clone();
        chat.visibility = bbb_world::ChatVisibilityState::System;
        world.set_chat_presentation_config(chat);
        let mut client_input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        crate::input::handle_key_input(
            &mut client_input,
            &mut counters,
            &mut world,
            &None,
            PhysicalKey::Code(KeyCode::KeyT),
            ElementState::Pressed,
        );
        crate::input::handle_text_input(
            &mut client_input,
            &mut counters,
            &mut world,
            &None,
            "draft 中",
        );
        assert!(client_input.open_chat_restrictions_screen());
        let input_runtime = SimulationInputRuntime::new(
            PhysicalSize::new(1280, 720),
            false,
            ClientInformation::default(),
        );
        let renderer = RendererCounters::default();
        let audio = AudioCounters::default();
        let network = HudDebugNetworkSampler::default();
        let tps = HudDebugTpsSampler::default();
        let initial = input(
            &world,
            &client_input,
            &input_runtime,
            &counters,
            &renderer,
            &audio,
            &network,
            &tps,
            Instant::now(),
        );
        let _publisher = SimulationPublisher::new(
            initial,
            Arc::clone(&control.snapshot),
            Arc::clone(&control.world),
        );

        let snapshot = control.snapshot.load_full();
        let status = snapshot.app.chat_screen.as_ref().expect("chat status");
        assert_eq!(
            status.screen,
            bbb_control::ChatScreenControlKind::Restrictions
        );
        assert_eq!(status.input_text, "draft 中");
        assert_eq!(status.input_utf16_units, 7);
        assert!(!status.can_send_messages);
        assert!(status.can_send_commands);
        assert!(client_input.close_chat_restrictions_screen());
        assert_eq!(
            snapshot.app.chat_screen.as_ref().unwrap().screen,
            bbb_control::ChatScreenControlKind::Restrictions
        );
    }

    #[test]
    fn camera_eye_height_publication_carries_one_immutable_tick_state() {
        let control = test_control_state();
        let mut world = WorldStore::new();
        world.set_local_player_pose(bbb_world::LocalPlayerPoseState::default());
        let client_input = ClientInputState::default();
        let input_runtime = SimulationInputRuntime::new(
            PhysicalSize::new(1280, 720),
            false,
            ClientInformation::default(),
        );
        let net = NetCounters::default();
        let renderer = RendererCounters::default();
        let audio = AudioCounters::default();
        let network = HudDebugNetworkSampler::default();
        let tps = HudDebugTpsSampler::default();
        let now = Instant::now();
        let mut eye_height = CameraEyeHeightTickState::default();
        eye_height.advance_for_world(1, &world);
        let mut initial = input(
            &world,
            &client_input,
            &input_runtime,
            &net,
            &renderer,
            &audio,
            &network,
            &tps,
            now,
        );
        initial.camera_eye_height_ticks = eye_height;

        let publisher = SimulationPublisher::new(initial, control.snapshot, control.world);

        assert_eq!(
            publisher.shared().load().camera_eye_height_ticks,
            eye_height
        );
    }

    #[test]
    fn partial_tick_projection_clamps_delayed_renderers() {
        let control = test_control_state();
        let world = WorldStore::new();
        let client_input = ClientInputState::default();
        let input_runtime = SimulationInputRuntime::new(
            PhysicalSize::new(1280, 720),
            false,
            ClientInformation::default(),
        );
        let net = NetCounters::default();
        let renderer = RendererCounters::default();
        let audio = AudioCounters::default();
        let network = HudDebugNetworkSampler::default();
        let tps = HudDebugTpsSampler::default();
        let now = Instant::now();
        let publisher = SimulationPublisher::new(
            input(
                &world,
                &client_input,
                &input_runtime,
                &net,
                &renderer,
                &audio,
                &network,
                &tps,
                now,
            ),
            control.snapshot,
            control.world,
        );
        let publication = publisher.shared().load_full();

        assert_eq!(publication.partial_tick_at(now), 0.25);
        assert_eq!(
            publication.partial_tick_at(now + Duration::from_millis(25)),
            0.75
        );
        assert_eq!(
            publication.partial_tick_at(now + Duration::from_millis(100)),
            1.0
        );

        let mut frozen_input = input(
            &world,
            &client_input,
            &input_runtime,
            &net,
            &renderer,
            &audio,
            &network,
            &tps,
            now,
        );
        let mut frozen_world = world.clone();
        frozen_world.apply_ticking_state(bbb_protocol::packets::TickingState {
            tick_rate: 20.0,
            frozen: true,
        });
        frozen_input.world = &frozen_world;
        let (frozen, _, _) =
            build_publication(2, frozen_input, None, Duration::ZERO, Duration::ZERO);
        assert_eq!(frozen.partial_tick_at(now), 1.0);
    }

    #[test]
    fn publication_and_publisher_are_thread_transfer_safe() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SimulationPublication>();
        assert_send_sync::<SharedSimulationPublication>();
        assert_send_sync::<SimulationPublisher>();
    }

    #[test]
    fn publication_freezes_world_and_publishes_matching_runtime_state() {
        use bbb_protocol::packets::BlockChangedAck;

        let control = test_control_state();
        let mut world = WorldStore::new();
        let client_input = ClientInputState::default();
        let input_runtime = SimulationInputRuntime::new(
            PhysicalSize::new(1280, 720),
            false,
            ClientInformation::default(),
        );
        let net = NetCounters::default();
        let renderer = RendererCounters::default();
        let mut audio = AudioCounters::default();
        let network = HudDebugNetworkSampler::default();
        let tps = HudDebugTpsSampler::default();
        let now = Instant::now();
        let mut publisher = SimulationPublisher::new(
            input(
                &world,
                &client_input,
                &input_runtime,
                &net,
                &renderer,
                &audio,
                &network,
                &tps,
                now,
            ),
            Arc::clone(&control.snapshot),
            Arc::clone(&control.world),
        );
        let previous = publisher.shared().load_full();

        world.apply_block_changed_ack(BlockChangedAck { sequence: 4 });
        audio.enabled = true;
        audio.commands_submitted = 9;
        let mut next = input(
            &world,
            &client_input,
            &input_runtime,
            &net,
            &renderer,
            &audio,
            &network,
            &tps,
            now,
        );
        next.running = false;
        let published = publisher.publish(next);

        assert_eq!(previous.world.counters().block_changed_ack_packets, 0);
        assert_eq!(published.world.counters().block_changed_ack_packets, 1);
        assert_eq!(control.snapshot.load().generation, published.generation);
        assert_eq!(control.snapshot.load().audio, audio);
        assert!(!control.snapshot.load().app.running);
        let control_world = control.world.load_full();
        assert_eq!(control_world.generation, published.generation);
        assert!(Arc::ptr_eq(&control_world.world, &published.world));
    }

    #[test]
    fn input_only_publication_reuses_exact_world_arc_for_render_and_control() {
        let control = test_control_state();
        let mut world = WorldStore::new();
        let client_input = ClientInputState::default();
        let input_runtime = SimulationInputRuntime::new(
            PhysicalSize::new(1280, 720),
            false,
            ClientInformation::default(),
        );
        let net = NetCounters::default();
        let renderer = RendererCounters::default();
        let audio = AudioCounters::default();
        let network = HudDebugNetworkSampler::default();
        let tps = HudDebugTpsSampler::default();
        let now = Instant::now();
        let mut publisher = SimulationPublisher::new(
            input(
                &world,
                &client_input,
                &input_runtime,
                &net,
                &renderer,
                &audio,
                &network,
                &tps,
                now,
            ),
            Arc::clone(&control.snapshot),
            Arc::clone(&control.world),
        );
        let generation_a = publisher.shared().load_full();

        let generation_b = publisher.publish_reusing_world(input(
            &world,
            &client_input,
            &input_runtime,
            &net,
            &renderer,
            &audio,
            &network,
            &tps,
            now,
        ));

        assert!(Arc::ptr_eq(&generation_a.world, &generation_b.world));
        assert_eq!(
            generation_b
                .net_counters
                .simulation_entity_snapshot_build_nanos,
            0
        );
        assert_eq!(
            generation_b
                .net_counters
                .simulation_entity_snapshot_rebuilt_entities,
            0
        );
        let control_world = control.world.load_full();
        assert_eq!(control_world.generation, generation_b.generation);
        assert!(Arc::ptr_eq(&generation_b.world, &control_world.world));

        world.apply_block_changed_ack(bbb_protocol::packets::BlockChangedAck { sequence: 3 });
        let generation_c = publisher.publish(input(
            &world,
            &client_input,
            &input_runtime,
            &net,
            &renderer,
            &audio,
            &network,
            &tps,
            now,
        ));
        assert_eq!(generation_a.world.counters().block_changed_ack_packets, 0);
        assert_eq!(generation_b.world.counters().block_changed_ack_packets, 0);
        assert_eq!(generation_c.world.counters().block_changed_ack_packets, 1);
        assert!(!Arc::ptr_eq(&generation_b.world, &generation_c.world));
    }

    #[test]
    fn publication_reports_generation_matched_build_duration_and_running_peak() {
        let control = test_control_state();
        let world = WorldStore::new();
        let client_input = ClientInputState::default();
        let input_runtime = SimulationInputRuntime::new(
            PhysicalSize::new(1280, 720),
            false,
            ClientInformation::default(),
        );
        let net = NetCounters::default();
        let renderer = RendererCounters::default();
        let audio = AudioCounters::default();
        let network = HudDebugNetworkSampler::default();
        let tps = HudDebugTpsSampler::default();
        let now = Instant::now();
        let mut publisher = SimulationPublisher::new(
            input(
                &world,
                &client_input,
                &input_runtime,
                &net,
                &renderer,
                &audio,
                &network,
                &tps,
                now,
            ),
            Arc::clone(&control.snapshot),
            Arc::clone(&control.world),
        );
        let initial = publisher.shared().load_full();
        assert_eq!(
            initial.net_counters.simulation_publication_build_nanos,
            initial.net_counters.simulation_publication_build_peak_nanos
        );
        assert_eq!(
            initial.net_counters.simulation_entity_snapshot_build_nanos,
            initial
                .net_counters
                .simulation_entity_snapshot_build_peak_nanos
        );
        assert_eq!(
            initial
                .net_counters
                .simulation_entity_snapshot_rebuilt_entities,
            initial.world.entity_count()
        );

        let published = publisher.publish(input(
            &world,
            &client_input,
            &input_runtime,
            &net,
            &renderer,
            &audio,
            &network,
            &tps,
            now,
        ));
        assert!(
            published
                .net_counters
                .simulation_publication_build_peak_nanos
                >= initial.net_counters.simulation_publication_build_peak_nanos
        );
        assert!(
            published
                .net_counters
                .simulation_publication_build_peak_nanos
                >= published.net_counters.simulation_publication_build_nanos
        );
        assert!(
            published
                .net_counters
                .simulation_entity_snapshot_build_peak_nanos
                >= initial
                    .net_counters
                    .simulation_entity_snapshot_build_peak_nanos
        );
        assert!(
            published
                .net_counters
                .simulation_entity_snapshot_build_peak_nanos
                >= published
                    .net_counters
                    .simulation_entity_snapshot_build_nanos
        );
        let control_snapshot = control.snapshot.load_full();
        assert_eq!(control_snapshot.generation, published.generation);
        assert_eq!(
            control_snapshot.net.simulation_publication_build_nanos,
            published.net_counters.simulation_publication_build_nanos
        );
        assert_eq!(
            control_snapshot.net.simulation_publication_build_peak_nanos,
            published
                .net_counters
                .simulation_publication_build_peak_nanos
        );
        assert_eq!(
            control_snapshot.net.simulation_entity_snapshot_build_nanos,
            published
                .net_counters
                .simulation_entity_snapshot_build_nanos
        );
        assert_eq!(
            control_snapshot
                .net
                .simulation_entity_snapshot_rebuilt_entities,
            published
                .net_counters
                .simulation_entity_snapshot_rebuilt_entities
        );
    }

    #[test]
    fn publisher_world_construction_uses_the_snapshot_boundary() {
        let source = include_str!("publication.rs");
        assert!(source.contains(concat!("input.world.", "publication_snapshot()")));
        assert!(!source.contains(concat!("input.world.", "clone()")));
    }
}
