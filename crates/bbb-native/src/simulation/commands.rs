//! Ordered commands entering the native simulation owner.

use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Condvar, Mutex, PoisonError},
    time::{Duration, Instant},
};

use bbb_control::{AudioCounters, ControlRequestSink, ControlWriteRequest, RendererCounters};
use bbb_renderer::{
    HudBookHitTarget, HudChatHitTarget, HudDeathScreenHitTarget, HudFontGlyphMap,
    HudGameEventHitTarget, HudSocialInteractionsHitTarget,
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::PhysicalKey,
};

const DEFAULT_COALESCING_THRESHOLD: usize = 256;

#[derive(Debug)]
pub(crate) enum DebugProfilingLifecycle {
    Started,
    Stopped {
        duration: Duration,
        ticks: usize,
        ticks_per_second: f64,
    },
    ReportSaved {
        path: PathBuf,
    },
    ReportFailed {
        error: String,
    },
}

/// One platform or control value applied by [`super::SimulationCore`].
#[derive(Debug)]
pub(crate) enum SimulationCommand {
    FocusChanged(bool),
    Key {
        physical_key: PhysicalKey,
        state: ElementState,
        clipboard_text: Option<String>,
    },
    MouseButton {
        button: MouseButton,
        state: ElementState,
        chat_target: Option<HudChatHitTarget>,
        book_target: Option<HudBookHitTarget>,
        death_screen_target: Option<HudDeathScreenHitTarget>,
        game_event_target: Option<HudGameEventHitTarget>,
        social_interactions_target: Option<HudSocialInteractionsHitTarget>,
    },
    MouseMotion {
        delta: (f64, f64),
    },
    MouseWheel {
        delta: MouseScrollDelta,
    },
    CursorMoved {
        position: PhysicalPosition<f64>,
        death_screen_target: Option<HudDeathScreenHitTarget>,
        game_event_target: Option<HudGameEventHitTarget>,
        social_interactions_target: Option<HudSocialInteractionsHitTarget>,
    },
    GameEventScreenRenderDelta {
        generation: u64,
        delta_ticks: f32,
    },
    CursorLeft,
    ImePreedit {
        text: String,
        cursor_range: Option<(usize, usize)>,
    },
    ImeCommit(String),
    ImeDisabled,
    SurfaceResized(PhysicalSize<u32>),
    TextMeasurements(HudFontGlyphMap),
    CursorCaptureResult {
        captured: bool,
    },
    ClipboardWriteResult {
        success: bool,
    },
    OpenUriResult {
        url: String,
        success: bool,
    },
    ResourceReloadAck(crate::resource_reload::ResourceReloadAck),
    TerrainReadiness {
        level_generation: u64,
        player_section_compiled_and_visible: bool,
    },
    DebugProfilingLifecycle(DebugProfilingLifecycle),
    Control(ControlWriteRequest),
    Shutdown,
}

impl SimulationCommand {
    /// Conservatively identifies commands whose application can mutate the
    /// canonical world. The false cases are render/platform-owned values whose
    /// handlers only update simulation input runtime or diagnostics.
    pub(crate) fn requires_world_snapshot_rebuild(&self) -> bool {
        !matches!(
            self,
            Self::SurfaceResized(_)
                | Self::TextMeasurements(_)
                | Self::CursorCaptureResult { .. }
                | Self::ClipboardWriteResult { .. }
                | Self::OpenUriResult { .. }
                | Self::Shutdown
        )
    }
}

/// A command with the global producer sequence assigned while holding the
/// mailbox's short mutex.
#[derive(Debug)]
pub(crate) struct SequencedSimulationCommand {
    pub(crate) sequence: u64,
    pub(crate) command: SimulationCommand,
}

#[derive(Debug)]
pub(crate) struct BackendTelemetry {
    pub(crate) renderer: RendererCounters,
    pub(crate) audio: AudioCounters,
    pub(crate) render_wakes_sent: u64,
    pub(crate) render_wakes_coalesced: u64,
    pub(crate) output_backlog_current: u64,
    pub(crate) output_backlog_peak: u64,
}

#[derive(Debug)]
struct SimulationMailboxState {
    next_sequence: u64,
    wake_generation: u64,
    coalescing_threshold: usize,
    commands: VecDeque<SequencedSimulationCommand>,
    backend_telemetry: Option<BackendTelemetry>,
}

impl Default for SimulationMailboxState {
    fn default() -> Self {
        Self {
            next_sequence: 1,
            wake_generation: 0,
            coalescing_threshold: DEFAULT_COALESCING_THRESHOLD,
            commands: VecDeque::new(),
            backend_telemetry: None,
        }
    }
}

#[derive(Debug, Default)]
struct SimulationMailboxShared {
    state: Mutex<SimulationMailboxState>,
    wake: Condvar,
}

/// Multi-producer, single-consumer command mailbox.
///
/// The capacity is deliberately soft: once the threshold is reached,
/// adjacent high-rate values coalesce at the queue tail, but discrete values
/// still append. A key release, mouse release, control action, or shutdown is
/// therefore never silently lost and producers never wait for simulation.
#[derive(Debug, Clone, Default)]
pub(crate) struct SimulationMailbox {
    shared: Arc<SimulationMailboxShared>,
}

impl SimulationMailbox {
    #[cfg(test)]
    fn with_coalescing_threshold(coalescing_threshold: usize) -> Self {
        Self {
            shared: Arc::new(SimulationMailboxShared {
                state: Mutex::new(SimulationMailboxState {
                    coalescing_threshold,
                    ..SimulationMailboxState::default()
                }),
                wake: Condvar::new(),
            }),
        }
    }

    /// Enqueues one command and returns its assigned monotonic sequence.
    pub(crate) fn enqueue(&self, command: SimulationCommand) -> u64 {
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let sequence = state.next_sequence;
        state.next_sequence = state
            .next_sequence
            .checked_add(1)
            .expect("simulation command sequence exhausted");
        state.wake_generation = state.wake_generation.saturating_add(1);

        if state.commands.len() >= state.coalescing_threshold
            && try_coalesce_tail(&mut state.commands, sequence, &command)
        {
            drop(state);
            self.shared.wake.notify_one();
            return sequence;
        }
        state
            .commands
            .push_back(SequencedSimulationCommand { sequence, command });
        drop(state);
        self.shared.wake.notify_one();
        sequence
    }

    /// Wakes the simulation owner after a producer outside this command queue
    /// (currently the ordered network-event forwarder) made work visible.
    pub(crate) fn notify_external_work(&self) {
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        state.wake_generation = state.wake_generation.saturating_add(1);
        drop(state);
        self.shared.wake.notify_one();
    }

    /// Replaces render-owned telemetry without waking simulation. The latest
    /// value is absorbed at the next tick, network, or input step, so a high
    /// render rate cannot create a simulation feedback loop.
    pub(crate) fn update_backend_telemetry(
        &self,
        renderer: RendererCounters,
        audio: AudioCounters,
        render_wakes_sent: u64,
        render_wakes_coalesced: u64,
        output_backlog_current: u64,
        output_backlog_peak: u64,
    ) {
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        state.backend_telemetry = Some(BackendTelemetry {
            renderer,
            audio,
            render_wakes_sent,
            render_wakes_coalesced,
            output_backlog_current,
            output_backlog_peak,
        });
    }

    pub(crate) fn take_backend_telemetry(&self) -> Option<BackendTelemetry> {
        self.shared
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .backend_telemetry
            .take()
    }

    pub(crate) fn wake_generation(&self) -> u64 {
        self.shared
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .wake_generation
    }

    /// Blocks only the simulation consumer until a producer wake or the next
    /// client-tick deadline. The observed generation closes the race between
    /// a step's input barrier and entering the condition-variable wait.
    pub(crate) fn wait_for_wake_since(&self, observed: u64, deadline: Instant) {
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        while state.wake_generation == observed {
            let now = Instant::now();
            if now >= deadline {
                return;
            }
            let (next, wait) = self
                .shared
                .wake
                .wait_timeout(state, deadline.saturating_duration_since(now))
                .unwrap_or_else(PoisonError::into_inner);
            state = next;
            if wait.timed_out() {
                return;
            }
        }
    }

    /// Captures the current producer barrier and drains only commands at or
    /// before it. Producers that acquire the mutex afterwards remain queued
    /// for the next simulation step.
    pub(crate) fn drain_through_barrier(&self) -> Vec<SequencedSimulationCommand> {
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let barrier = state.next_sequence.saturating_sub(1);
        let drain_len = state
            .commands
            .iter()
            .take_while(|command| command.sequence <= barrier)
            .count();
        state.commands.drain(..drain_len).collect()
    }

    pub(crate) fn pending_len(&self) -> usize {
        self.shared
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .commands
            .len()
    }
}

impl ControlRequestSink for SimulationMailbox {
    fn submit(&self, request: ControlWriteRequest) -> usize {
        let command = match request {
            ControlWriteRequest::Shutdown => SimulationCommand::Shutdown,
            request => SimulationCommand::Control(request),
        };
        self.enqueue(command);
        self.pending_len()
    }
}

fn try_coalesce_tail(
    commands: &mut VecDeque<SequencedSimulationCommand>,
    sequence: u64,
    incoming: &SimulationCommand,
) -> bool {
    let Some(tail) = commands.back_mut() else {
        return false;
    };
    let coalesced = match (&mut tail.command, incoming) {
        (
            SimulationCommand::MouseMotion { delta },
            SimulationCommand::MouseMotion {
                delta: incoming_delta,
            },
        ) => {
            delta.0 += incoming_delta.0;
            delta.1 += incoming_delta.1;
            true
        }
        (
            SimulationCommand::MouseWheel { delta },
            SimulationCommand::MouseWheel {
                delta: incoming_delta,
            },
        ) => coalesce_wheel_delta(delta, incoming_delta),
        (
            SimulationCommand::CursorMoved {
                position,
                death_screen_target,
                game_event_target,
                social_interactions_target,
            },
            SimulationCommand::CursorMoved {
                position: incoming_position,
                death_screen_target: incoming_death_screen_target,
                game_event_target: incoming_game_event_target,
                social_interactions_target: incoming_social_interactions_target,
            },
        ) => {
            *position = *incoming_position;
            *death_screen_target = *incoming_death_screen_target;
            *game_event_target = *incoming_game_event_target;
            *social_interactions_target = *incoming_social_interactions_target;
            true
        }
        (
            SimulationCommand::GameEventScreenRenderDelta {
                generation,
                delta_ticks,
            },
            SimulationCommand::GameEventScreenRenderDelta {
                generation: incoming_generation,
                delta_ticks: incoming_delta,
            },
        ) if generation == incoming_generation => {
            *delta_ticks += incoming_delta;
            true
        }
        (
            SimulationCommand::SurfaceResized(size),
            SimulationCommand::SurfaceResized(incoming_size),
        ) => {
            *size = *incoming_size;
            true
        }
        (
            SimulationCommand::TerrainReadiness {
                level_generation,
                player_section_compiled_and_visible,
            },
            SimulationCommand::TerrainReadiness {
                level_generation: incoming_generation,
                player_section_compiled_and_visible: incoming_ready,
            },
        ) => {
            *level_generation = *incoming_generation;
            *player_section_compiled_and_visible = *incoming_ready;
            true
        }
        _ => false,
    };
    if coalesced {
        tail.sequence = sequence;
    }
    coalesced
}

fn coalesce_wheel_delta(current: &mut MouseScrollDelta, incoming: &MouseScrollDelta) -> bool {
    match (current, incoming) {
        (MouseScrollDelta::LineDelta(x, y), MouseScrollDelta::LineDelta(dx, dy)) => {
            *x += dx;
            *y += dy;
            true
        }
        (MouseScrollDelta::PixelDelta(position), MouseScrollDelta::PixelDelta(delta)) => {
            position.x += delta.x;
            position.y += delta.y;
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_control::{InputControlRequest, InputKeyControl, NetControlRequest};
    use winit::keyboard::KeyCode;

    #[test]
    fn commands_from_winit_and_control_share_one_monotonic_sequence() {
        let mailbox = SimulationMailbox::default();
        assert_eq!(mailbox.enqueue(SimulationCommand::FocusChanged(true)), 1);
        ControlRequestSink::submit(
            &mailbox,
            ControlWriteRequest::Net(NetControlRequest::RequestStats),
        );
        mailbox.enqueue(SimulationCommand::Key {
            physical_key: PhysicalKey::Code(KeyCode::KeyW),
            state: ElementState::Released,
            clipboard_text: None,
        });

        let drained = mailbox.drain_through_barrier();
        assert_eq!(
            drained
                .iter()
                .map(|command| command.sequence)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert!(matches!(
            drained[1].command,
            SimulationCommand::Control(ControlWriteRequest::Net(NetControlRequest::RequestStats))
        ));
    }

    #[test]
    fn saturated_mailbox_never_drops_key_mouse_or_control_releases() {
        let mailbox = SimulationMailbox::with_coalescing_threshold(1);
        mailbox.enqueue(SimulationCommand::MouseMotion { delta: (1.0, 2.0) });
        mailbox.enqueue(SimulationCommand::Key {
            physical_key: PhysicalKey::Code(KeyCode::KeyW),
            state: ElementState::Released,
            clipboard_text: None,
        });
        mailbox.enqueue(SimulationCommand::MouseButton {
            button: MouseButton::Left,
            state: ElementState::Released,
            chat_target: None,
            book_target: None,
            death_screen_target: None,
            game_event_target: None,
            social_interactions_target: None,
        });
        ControlRequestSink::submit(
            &mailbox,
            ControlWriteRequest::Input(InputControlRequest::Key {
                key: InputKeyControl::Forward,
                pressed: false,
            }),
        );
        ControlRequestSink::submit(&mailbox, ControlWriteRequest::Shutdown);

        let drained = mailbox.drain_through_barrier();
        assert_eq!(drained.len(), 5);
        assert!(matches!(
            drained[1].command,
            SimulationCommand::Key {
                state: ElementState::Released,
                ..
            }
        ));
        assert!(matches!(
            drained[2].command,
            SimulationCommand::MouseButton {
                state: ElementState::Released,
                ..
            }
        ));
        assert!(matches!(drained[4].command, SimulationCommand::Shutdown));
    }

    #[test]
    fn high_rate_values_coalesce_only_without_a_discrete_barrier() {
        let mailbox = SimulationMailbox::with_coalescing_threshold(1);
        mailbox.enqueue(SimulationCommand::MouseMotion { delta: (1.0, 2.0) });
        mailbox.enqueue(SimulationCommand::MouseMotion { delta: (3.0, 4.0) });
        mailbox.enqueue(SimulationCommand::FocusChanged(true));
        mailbox.enqueue(SimulationCommand::MouseMotion { delta: (5.0, 6.0) });
        mailbox.enqueue(SimulationCommand::MouseMotion { delta: (7.0, 8.0) });

        let drained = mailbox.drain_through_barrier();
        assert_eq!(drained.len(), 3);
        assert_eq!(drained[0].sequence, 2);
        assert_eq!(drained[2].sequence, 5);
        assert!(matches!(
            drained[0].command,
            SimulationCommand::MouseMotion { delta: (4.0, 6.0) }
        ));
        assert!(matches!(
            drained[2].command,
            SimulationCommand::MouseMotion {
                delta: (12.0, 14.0)
            }
        ));
    }

    #[test]
    fn death_screen_targets_survive_cursor_coalescing_and_mouse_barrier() {
        use bbb_renderer::{
            HudDeathScreenButton, HudDeathScreenHitTarget, HudDeathScreenHitTargetKind,
        };

        let mailbox = SimulationMailbox::with_coalescing_threshold(1);
        mailbox.enqueue(SimulationCommand::CursorMoved {
            position: PhysicalPosition::new(10.0, 20.0),
            death_screen_target: Some(HudDeathScreenHitTarget {
                generation: 7,
                kind: HudDeathScreenHitTargetKind::Button(HudDeathScreenButton::Respawn),
            }),
            game_event_target: None,
            social_interactions_target: None,
        });
        let latest = HudDeathScreenHitTarget {
            generation: 8,
            kind: HudDeathScreenHitTargetKind::CauseLink { run_index: 3 },
        };
        mailbox.enqueue(SimulationCommand::CursorMoved {
            position: PhysicalPosition::new(30.0, 40.0),
            death_screen_target: Some(latest),
            game_event_target: None,
            social_interactions_target: None,
        });
        mailbox.enqueue(SimulationCommand::MouseButton {
            button: MouseButton::Left,
            state: ElementState::Pressed,
            chat_target: None,
            book_target: None,
            death_screen_target: Some(latest),
            game_event_target: None,
            social_interactions_target: None,
        });

        let drained = mailbox.drain_through_barrier();
        assert_eq!(drained.len(), 2);
        assert!(matches!(
            drained[0].command,
            SimulationCommand::CursorMoved {
                position,
                death_screen_target: Some(target),
                ..
            } if position == PhysicalPosition::new(30.0, 40.0) && target == latest
        ));
        assert!(matches!(
            drained[1].command,
            SimulationCommand::MouseButton {
                death_screen_target: Some(target),
                ..
            } if target == latest
        ));
    }

    #[test]
    fn hud_social_interactions_targets_survive_cursor_coalescing_and_mouse_barrier() {
        use bbb_renderer::{
            HudSocialInteractionsFocus, HudSocialInteractionsHitTarget,
            HudSocialInteractionsHitTargetKind,
        };

        let mailbox = SimulationMailbox::with_coalescing_threshold(1);
        mailbox.enqueue(SimulationCommand::CursorMoved {
            position: PhysicalPosition::new(10.0, 20.0),
            death_screen_target: None,
            game_event_target: None,
            social_interactions_target: None,
        });
        let latest = HudSocialInteractionsHitTarget {
            generation: 8,
            kind: HudSocialInteractionsHitTargetKind::Focus(HudSocialInteractionsFocus::Search),
        };
        mailbox.enqueue(SimulationCommand::CursorMoved {
            position: PhysicalPosition::new(30.0, 40.0),
            death_screen_target: None,
            game_event_target: None,
            social_interactions_target: Some(latest),
        });
        mailbox.enqueue(SimulationCommand::MouseButton {
            button: MouseButton::Left,
            state: ElementState::Pressed,
            chat_target: None,
            book_target: None,
            death_screen_target: None,
            game_event_target: None,
            social_interactions_target: Some(latest),
        });

        let drained = mailbox.drain_through_barrier();
        assert_eq!(drained.len(), 2);
        assert!(matches!(
            drained[0].command,
            SimulationCommand::CursorMoved {
                position,
                social_interactions_target: Some(target),
                ..
            } if position == PhysicalPosition::new(30.0, 40.0) && target == latest
        ));
        assert!(matches!(
            drained[1].command,
            SimulationCommand::MouseButton {
                social_interactions_target: Some(target),
                ..
            } if target == latest
        ));
    }

    #[test]
    fn win_screen_render_deltas_coalesce_only_within_one_generation() {
        let mailbox = SimulationMailbox::with_coalescing_threshold(1);
        mailbox.enqueue(SimulationCommand::GameEventScreenRenderDelta {
            generation: 7,
            delta_ticks: 0.25,
        });
        mailbox.enqueue(SimulationCommand::GameEventScreenRenderDelta {
            generation: 7,
            delta_ticks: 0.75,
        });
        mailbox.enqueue(SimulationCommand::GameEventScreenRenderDelta {
            generation: 8,
            delta_ticks: 0.5,
        });

        let drained = mailbox.drain_through_barrier();
        assert_eq!(drained.len(), 2);
        assert!(matches!(
            drained[0].command,
            SimulationCommand::GameEventScreenRenderDelta {
                generation: 7,
                delta_ticks: 1.0,
            }
        ));
        assert!(matches!(
            drained[1].command,
            SimulationCommand::GameEventScreenRenderDelta {
                generation: 8,
                delta_ticks: 0.5,
            }
        ));
    }

    #[test]
    fn backend_telemetry_is_latest_value_and_does_not_wake_or_enqueue() {
        let mailbox = SimulationMailbox::default();
        let wake_generation = mailbox.wake_generation();
        let mut latest_renderer = RendererCounters::default();
        latest_renderer.frame_index = 259;
        let mut latest_audio = AudioCounters::default();
        latest_audio.commands_submitted = 259;

        for frame in 0..260 {
            let mut renderer = RendererCounters::default();
            renderer.frame_index = frame;
            let mut audio = AudioCounters::default();
            audio.commands_submitted = frame;
            mailbox.update_backend_telemetry(
                renderer,
                audio,
                frame,
                frame + 1,
                frame + 2,
                frame + 3,
            );
        }

        assert_eq!(mailbox.wake_generation(), wake_generation);
        assert_eq!(mailbox.pending_len(), 0);
        let telemetry = mailbox.take_backend_telemetry().unwrap();
        assert_eq!(telemetry.renderer.frame_index, latest_renderer.frame_index);
        assert_eq!(
            telemetry.audio.commands_submitted,
            latest_audio.commands_submitted
        );
        assert_eq!(telemetry.render_wakes_sent, 259);
        assert_eq!(telemetry.render_wakes_coalesced, 260);
        assert_eq!(telemetry.output_backlog_current, 261);
        assert_eq!(telemetry.output_backlog_peak, 262);
        assert!(mailbox.take_backend_telemetry().is_none());
    }

    #[test]
    fn only_narrow_render_runtime_commands_reuse_the_world_snapshot() {
        assert!(!SimulationCommand::SurfaceResized(PhysicalSize::new(1, 1))
            .requires_world_snapshot_rebuild());
        assert!(!SimulationCommand::CursorCaptureResult { captured: true }
            .requires_world_snapshot_rebuild());
        assert!(SimulationCommand::TerrainReadiness {
            level_generation: 1,
            player_section_compiled_and_visible: true,
        }
        .requires_world_snapshot_rebuild());
        assert!(
            SimulationCommand::MouseMotion { delta: (1.0, 1.0) }.requires_world_snapshot_rebuild()
        );
    }

    #[test]
    fn mailbox_and_commands_are_send() {
        fn assert_send<T: Send>() {}
        assert_send::<SimulationMailbox>();
        assert_send::<SequencedSimulationCommand>();
    }

    #[test]
    fn concurrent_producers_share_one_contiguous_sequence() {
        let mailbox = SimulationMailbox::default();
        let control_mailbox = mailbox.clone();
        let producer = std::thread::spawn(move || {
            for _ in 0..32 {
                ControlRequestSink::submit(
                    &control_mailbox,
                    ControlWriteRequest::Net(NetControlRequest::RequestStats),
                );
            }
        });
        for index in 0..32 {
            mailbox.enqueue(SimulationCommand::FocusChanged(index % 2 == 0));
        }
        producer.join().expect("control producer");

        let drained = mailbox.drain_through_barrier();
        assert_eq!(drained.len(), 64);
        assert_eq!(
            drained
                .iter()
                .map(|command| command.sequence)
                .collect::<Vec<_>>(),
            (1..=64).collect::<Vec<_>>()
        );
    }
}
