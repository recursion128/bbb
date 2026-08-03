//! Ordered commands entering the native simulation owner.

use std::{
    collections::VecDeque,
    fmt,
    path::PathBuf,
    sync::{Arc, Condvar, Mutex, PoisonError},
    time::{Duration, Instant},
};

use bbb_control::{
    AudioCounters, ControlRequestSink, ControlSubmitError, ControlSubmitReceipt,
    ControlWriteRequest, RendererCounters, SimulationCommandMailboxClassCounters,
    SimulationCommandMailboxCounters,
};
use bbb_protocol::packets::Vec3d;
use bbb_renderer::{
    HudBookHitTarget, HudChatHitTarget, HudDeathScreenHitTarget, HudDisconnectedScreenHitTarget,
    HudFontGlyphMap, HudGameEventHitTarget, HudPlayerReportingHitTarget,
    HudSocialInteractionsHitTarget,
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::PhysicalKey,
};

const DEFAULT_COALESCING_THRESHOLD: usize = 256;
const MIB: usize = 1024 * 1024;
const PRODUCER_CLASS_COUNT: usize = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationCommandProducerClass {
    PlatformInput,
    Control,
    RenderFeedback,
    AsyncCompletion,
    Lifecycle,
}

impl SimulationCommandProducerClass {
    const fn index(self) -> usize {
        match self {
            Self::PlatformInput => 0,
            Self::Control => 1,
            Self::RenderFeedback => 2,
            Self::AsyncCompletion => 3,
            Self::Lifecycle => 4,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SimulationCommandClassLimit {
    commands: usize,
    retained_bytes: usize,
}

#[derive(Clone, Copy, Debug)]
struct SimulationMailboxLimits {
    coalescing_threshold: usize,
    classes: [SimulationCommandClassLimit; PRODUCER_CLASS_COUNT],
}

impl SimulationMailboxLimits {
    const DEFAULT: Self = Self {
        coalescing_threshold: DEFAULT_COALESCING_THRESHOLD,
        classes: [
            SimulationCommandClassLimit {
                commands: 2048,
                retained_bytes: 8 * MIB,
            },
            SimulationCommandClassLimit {
                commands: 512,
                retained_bytes: 8 * MIB,
            },
            SimulationCommandClassLimit {
                commands: 64,
                retained_bytes: 1088 * MIB,
            },
            SimulationCommandClassLimit {
                commands: 32,
                retained_bytes: MIB,
            },
            SimulationCommandClassLimit {
                commands: 1,
                retained_bytes: 4 * 1024,
            },
        ],
    };

    fn command_limit(self) -> usize {
        self.classes
            .iter()
            .map(|limit| limit.commands)
            .fold(0usize, usize::saturating_add)
    }

    fn retained_byte_limit(self) -> usize {
        self.classes
            .iter()
            .map(|limit| limit.retained_bytes)
            .fold(0usize, usize::saturating_add)
    }
}

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
        disconnected_screen_target: Option<HudDisconnectedScreenHitTarget>,
        death_screen_target: Option<HudDeathScreenHitTarget>,
        game_event_target: Option<HudGameEventHitTarget>,
        player_reporting_target: Option<HudPlayerReportingHitTarget>,
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
        disconnected_screen_target: Option<HudDisconnectedScreenHitTarget>,
        death_screen_target: Option<HudDeathScreenHitTarget>,
        game_event_target: Option<HudGameEventHitTarget>,
        player_reporting_target: Option<HudPlayerReportingHitTarget>,
        social_interactions_target: Option<HudSocialInteractionsHitTarget>,
    },
    GameEventScreenRenderDelta {
        generation: u64,
        delta_ticks: f32,
    },
    HudAirBubblePopSound {
        level_generation: u64,
        position: Vec3d,
        volume: f32,
        pitch: f32,
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
    OpenPathResult {
        path: PathBuf,
        success: bool,
    },
    ResourceReloadAck(Box<crate::resource_reload::ResourceReloadAck>),
    TerrainReadiness {
        level_generation: u64,
        player_section_compiled_and_visible: bool,
    },
    DebugProfilingLifecycle(DebugProfilingLifecycle),
    ServicesBlockListCompletion(crate::services_block_list_runtime::ServicesBlockListCompletion),
    PlayerReportSubmissionCompletion(
        crate::services_block_list_runtime::PlayerReportSubmissionCompletion,
    ),
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
                | Self::OpenPathResult { .. }
                | Self::Shutdown
        )
    }

    fn producer_class(&self) -> SimulationCommandProducerClass {
        match self {
            Self::FocusChanged(_)
            | Self::Key { .. }
            | Self::MouseButton { .. }
            | Self::MouseMotion { .. }
            | Self::MouseWheel { .. }
            | Self::CursorMoved { .. }
            | Self::CursorLeft
            | Self::ImePreedit { .. }
            | Self::ImeCommit(_)
            | Self::ImeDisabled => SimulationCommandProducerClass::PlatformInput,
            Self::Control(_) => SimulationCommandProducerClass::Control,
            Self::ServicesBlockListCompletion(_) | Self::PlayerReportSubmissionCompletion(_) => {
                SimulationCommandProducerClass::AsyncCompletion
            }
            Self::Shutdown => SimulationCommandProducerClass::Lifecycle,
            Self::GameEventScreenRenderDelta { .. }
            | Self::HudAirBubblePopSound { .. }
            | Self::SurfaceResized(_)
            | Self::TextMeasurements(_)
            | Self::CursorCaptureResult { .. }
            | Self::ClipboardWriteResult { .. }
            | Self::OpenUriResult { .. }
            | Self::OpenPathResult { .. }
            | Self::ResourceReloadAck(_)
            | Self::TerrainReadiness { .. }
            | Self::DebugProfilingLifecycle(_) => SimulationCommandProducerClass::RenderFeedback,
        }
    }

    fn retained_logical_bytes(&self) -> usize {
        let dynamic = match self {
            Self::Key { clipboard_text, .. } => clipboard_text.as_ref().map_or(0, String::capacity),
            Self::ImePreedit { text, .. } | Self::ImeCommit(text) => text.capacity(),
            Self::TextMeasurements(glyphs) => glyphs
                .retained_logical_bytes()
                .saturating_sub(std::mem::size_of::<HudFontGlyphMap>()),
            Self::OpenUriResult { url, .. } => url.capacity(),
            Self::OpenPathResult { path, .. } => path.capacity(),
            Self::ResourceReloadAck(ack) => ack.mailbox_retained_logical_bytes(),
            Self::DebugProfilingLifecycle(DebugProfilingLifecycle::ReportSaved { path }) => {
                path.capacity()
            }
            Self::DebugProfilingLifecycle(DebugProfilingLifecycle::ReportFailed { error }) => {
                error.capacity()
            }
            Self::ServicesBlockListCompletion(completion) => {
                completion.result.as_ref().ok().map_or(0, |blocked| {
                    blocked
                        .capacity()
                        .saturating_mul(std::mem::size_of::<uuid::Uuid>())
                })
            }
            Self::Control(request) => request
                .retained_logical_bytes()
                .saturating_sub(std::mem::size_of::<ControlWriteRequest>()),
            _ => 0,
        };
        std::mem::size_of::<SequencedSimulationCommand>().saturating_add(dynamic)
    }
}

/// A command with the global producer sequence assigned while holding the
/// mailbox's short mutex.
#[derive(Debug)]
pub(crate) struct SequencedSimulationCommand {
    pub(crate) sequence: u64,
    pub(crate) command: SimulationCommand,
    producer_class: SimulationCommandProducerClass,
    retained_bytes: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct SimulationMailboxClassState {
    pending: usize,
    peak_pending: usize,
    retained_bytes: usize,
    peak_retained_bytes: usize,
    blocked_submissions: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SimulationMailboxSubmit {
    pub(crate) sequence: u64,
    pub(crate) pending: usize,
    pub(crate) retained_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationMailboxSubmitError {
    Oversized {
        producer_class: SimulationCommandProducerClass,
        retained_bytes: usize,
        retained_byte_limit: usize,
    },
    Closed,
}

impl fmt::Display for SimulationMailboxSubmitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Oversized {
                producer_class,
                retained_bytes,
                retained_byte_limit,
            } => write!(
                formatter,
                "{producer_class:?} simulation command retains {retained_bytes} bytes, limit {retained_byte_limit}"
            ),
            Self::Closed => formatter.write_str("simulation command mailbox is closed"),
        }
    }
}

impl std::error::Error for SimulationMailboxSubmitError {}

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
    limits: SimulationMailboxLimits,
    commands: VecDeque<SequencedSimulationCommand>,
    classes: [SimulationMailboxClassState; PRODUCER_CLASS_COUNT],
    pending: usize,
    peak_pending: usize,
    retained_bytes: usize,
    peak_retained_bytes: usize,
    blocked_submissions: u64,
    coalesced_submissions: u64,
    oversized_rejections: u64,
    closed_rejections: u64,
    closed: bool,
    backend_telemetry: Option<BackendTelemetry>,
}

impl Default for SimulationMailboxState {
    fn default() -> Self {
        Self {
            next_sequence: 1,
            wake_generation: 0,
            limits: SimulationMailboxLimits::DEFAULT,
            commands: VecDeque::new(),
            classes: [SimulationMailboxClassState::default(); PRODUCER_CLASS_COUNT],
            pending: 0,
            peak_pending: 0,
            retained_bytes: 0,
            peak_retained_bytes: 0,
            blocked_submissions: 0,
            coalesced_submissions: 0,
            oversized_rejections: 0,
            closed_rejections: 0,
            closed: false,
            backend_telemetry: None,
        }
    }
}

#[derive(Debug, Default)]
struct SimulationMailboxShared {
    state: Mutex<SimulationMailboxState>,
    wake: Condvar,
    space: Condvar,
}

/// Multi-producer, single-consumer command mailbox.
///
/// Each producer class has a hard command/retained-byte partition. Once the
/// soft threshold is reached, adjacent high-rate values still coalesce at the
/// queue tail; otherwise a saturated producer waits for the simulation owner
/// to drain space, preserving release, interaction and shutdown commands.
#[derive(Debug, Clone, Default)]
pub(crate) struct SimulationMailbox {
    shared: Arc<SimulationMailboxShared>,
}

impl SimulationMailbox {
    #[cfg(test)]
    fn with_coalescing_threshold(coalescing_threshold: usize) -> Self {
        let mut limits = SimulationMailboxLimits::DEFAULT;
        limits.coalescing_threshold = coalescing_threshold;
        Self::with_limits(limits)
    }

    #[cfg(test)]
    fn with_limits(limits: SimulationMailboxLimits) -> Self {
        Self {
            shared: Arc::new(SimulationMailboxShared {
                state: Mutex::new(SimulationMailboxState {
                    limits,
                    ..SimulationMailboxState::default()
                }),
                wake: Condvar::new(),
                space: Condvar::new(),
            }),
        }
    }

    /// Enqueues one command, applying backpressure until its producer
    /// partition has both count and retained-byte capacity.
    pub(crate) fn enqueue(
        &self,
        command: SimulationCommand,
    ) -> Result<SimulationMailboxSubmit, SimulationMailboxSubmitError> {
        let producer_class = command.producer_class();
        let class_index = producer_class.index();
        let retained_bytes = command.retained_logical_bytes();
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let class_limit = state.limits.classes[class_index];
        if retained_bytes > class_limit.retained_bytes {
            state.oversized_rejections = state.oversized_rejections.saturating_add(1);
            return Err(SimulationMailboxSubmitError::Oversized {
                producer_class,
                retained_bytes,
                retained_byte_limit: class_limit.retained_bytes,
            });
        }

        let mut recorded_block = false;
        loop {
            if state.closed {
                state.closed_rejections = state.closed_rejections.saturating_add(1);
                return Err(SimulationMailboxSubmitError::Closed);
            }

            let sequence = state.next_sequence;
            if state.pending >= state.limits.coalescing_threshold
                && try_coalesce_tail(&mut state.commands, sequence, &command)
            {
                state.next_sequence = next_sequence(sequence);
                state.wake_generation = state.wake_generation.saturating_add(1);
                state.coalesced_submissions = state.coalesced_submissions.saturating_add(1);
                let receipt = SimulationMailboxSubmit {
                    sequence,
                    pending: state.pending,
                    retained_bytes: state.retained_bytes,
                };
                drop(state);
                self.shared.wake.notify_one();
                return Ok(receipt);
            }

            let class = state.classes[class_index];
            let class_fits = class.pending < class_limit.commands
                && class.retained_bytes.saturating_add(retained_bytes)
                    <= class_limit.retained_bytes;
            let total_fits = state.pending < state.limits.command_limit()
                && state.retained_bytes.saturating_add(retained_bytes)
                    <= state.limits.retained_byte_limit();
            if class_fits && total_fits {
                state.next_sequence = next_sequence(sequence);
                state.wake_generation = state.wake_generation.saturating_add(1);
                state.pending = state.pending.saturating_add(1);
                state.peak_pending = state.peak_pending.max(state.pending);
                state.retained_bytes = state.retained_bytes.saturating_add(retained_bytes);
                state.peak_retained_bytes = state.peak_retained_bytes.max(state.retained_bytes);
                let class = &mut state.classes[class_index];
                class.pending = class.pending.saturating_add(1);
                class.peak_pending = class.peak_pending.max(class.pending);
                class.retained_bytes = class.retained_bytes.saturating_add(retained_bytes);
                class.peak_retained_bytes = class.peak_retained_bytes.max(class.retained_bytes);
                state.commands.push_back(SequencedSimulationCommand {
                    sequence,
                    command,
                    producer_class,
                    retained_bytes,
                });
                let receipt = SimulationMailboxSubmit {
                    sequence,
                    pending: state.pending,
                    retained_bytes: state.retained_bytes,
                };
                drop(state);
                self.shared.wake.notify_one();
                return Ok(receipt);
            }

            if !recorded_block {
                state.blocked_submissions = state.blocked_submissions.saturating_add(1);
                state.classes[class_index].blocked_submissions = state.classes[class_index]
                    .blocked_submissions
                    .saturating_add(1);
                recorded_block = true;
            }
            state = self
                .shared
                .space
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Runtime producer convenience that preserves explicit diagnostics for
    /// terminal/oversized failures. Capacity pressure itself is handled by
    /// `enqueue` backpressure and therefore never reaches this branch.
    pub(crate) fn enqueue_or_report(&self, command: SimulationCommand, producer: &'static str) {
        if let Err(error) = self.enqueue(command) {
            tracing::warn!(producer, %error, "simulation command submission rejected");
        }
    }

    pub(crate) fn close(&self) {
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if state.closed {
            return;
        }
        state.closed = true;
        state.wake_generation = state.wake_generation.saturating_add(1);
        drop(state);
        self.shared.wake.notify_all();
        self.shared.space.notify_all();
    }

    /// Wakes the simulation owner after a producer outside this command queue
    /// (currently the ordered network-event forwarder) made work visible.
    pub(crate) fn notify_external_work(&self) {
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if state.closed {
            return;
        }
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
        while state.wake_generation == observed && !state.closed {
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
        let drained = state.commands.drain(..drain_len).collect::<Vec<_>>();
        for command in &drained {
            state.pending = state.pending.saturating_sub(1);
            state.retained_bytes = state.retained_bytes.saturating_sub(command.retained_bytes);
            let class = &mut state.classes[command.producer_class.index()];
            class.pending = class.pending.saturating_sub(1);
            class.retained_bytes = class.retained_bytes.saturating_sub(command.retained_bytes);
        }
        let released_space = !drained.is_empty();
        drop(state);
        if released_space {
            self.shared.space.notify_all();
        }
        drained
    }

    #[cfg(test)]
    pub(crate) fn pending_len(&self) -> usize {
        self.shared
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pending
    }

    pub(crate) fn observation(&self) -> SimulationCommandMailboxCounters {
        let state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let class = |producer_class: SimulationCommandProducerClass| {
            let index = producer_class.index();
            let limit = state.limits.classes[index];
            let counters = state.classes[index];
            SimulationCommandMailboxClassCounters {
                command_limit: limit.commands,
                retained_byte_limit: usize_u64(limit.retained_bytes),
                pending: counters.pending,
                peak_pending: counters.peak_pending,
                retained_bytes: usize_u64(counters.retained_bytes),
                peak_retained_bytes: usize_u64(counters.peak_retained_bytes),
                blocked_submissions: counters.blocked_submissions,
            }
        };
        SimulationCommandMailboxCounters {
            command_limit: state.limits.command_limit(),
            retained_byte_limit: usize_u64(state.limits.retained_byte_limit()),
            pending: state.pending,
            peak_pending: state.peak_pending,
            retained_bytes: usize_u64(state.retained_bytes),
            peak_retained_bytes: usize_u64(state.peak_retained_bytes),
            blocked_submissions: state.blocked_submissions,
            coalesced_submissions: state.coalesced_submissions,
            oversized_rejections: state.oversized_rejections,
            closed_rejections: state.closed_rejections,
            platform_input: class(SimulationCommandProducerClass::PlatformInput),
            control: class(SimulationCommandProducerClass::Control),
            render_feedback: class(SimulationCommandProducerClass::RenderFeedback),
            async_completion: class(SimulationCommandProducerClass::AsyncCompletion),
            lifecycle: class(SimulationCommandProducerClass::Lifecycle),
        }
    }
}

impl ControlRequestSink for SimulationMailbox {
    fn submit(
        &self,
        request: ControlWriteRequest,
    ) -> Result<ControlSubmitReceipt, ControlSubmitError> {
        let command = match request {
            ControlWriteRequest::Shutdown => SimulationCommand::Shutdown,
            request => SimulationCommand::Control(request),
        };
        self.enqueue(command)
            .map(|receipt| ControlSubmitReceipt {
                pending: receipt.pending,
                retained_bytes: receipt.retained_bytes,
            })
            .map_err(|error| match error {
                SimulationMailboxSubmitError::Oversized {
                    retained_bytes,
                    retained_byte_limit,
                    ..
                } => ControlSubmitError::Oversized {
                    retained_bytes,
                    retained_byte_limit,
                },
                SimulationMailboxSubmitError::Closed => ControlSubmitError::Closed,
            })
    }
}

fn next_sequence(sequence: u64) -> u64 {
    sequence
        .checked_add(1)
        .expect("simulation command sequence exhausted")
}

fn usize_u64(value: usize) -> u64 {
    value.try_into().unwrap_or(u64::MAX)
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
                disconnected_screen_target,
                death_screen_target,
                game_event_target,
                player_reporting_target,
                social_interactions_target,
            },
            SimulationCommand::CursorMoved {
                position: incoming_position,
                disconnected_screen_target: incoming_disconnected_screen_target,
                death_screen_target: incoming_death_screen_target,
                game_event_target: incoming_game_event_target,
                player_reporting_target: incoming_player_reporting_target,
                social_interactions_target: incoming_social_interactions_target,
            },
        ) => {
            *position = *incoming_position;
            *disconnected_screen_target = *incoming_disconnected_screen_target;
            *death_screen_target = *incoming_death_screen_target;
            *game_event_target = *incoming_game_event_target;
            *player_reporting_target = *incoming_player_reporting_target;
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

    #[track_caller]
    fn enqueue(mailbox: &SimulationMailbox, command: SimulationCommand) -> SimulationMailboxSubmit {
        mailbox.enqueue(command).unwrap()
    }

    fn limits_for(
        producer_class: SimulationCommandProducerClass,
        commands: usize,
        retained_bytes: usize,
    ) -> SimulationMailboxLimits {
        let mut limits = SimulationMailboxLimits::DEFAULT;
        limits.classes[producer_class.index()] = SimulationCommandClassLimit {
            commands,
            retained_bytes,
        };
        limits
    }

    #[track_caller]
    fn wait_for_blocked_submission(mailbox: &SimulationMailbox) {
        let deadline = Instant::now() + Duration::from_secs(1);
        while mailbox.observation().blocked_submissions == 0 {
            assert!(
                Instant::now() < deadline,
                "producer did not reach backpressure"
            );
            std::thread::yield_now();
        }
    }

    #[test]
    fn default_producer_partitions_match_the_frozen_contract() {
        let observation = SimulationMailbox::default().observation();

        assert_eq!(observation.command_limit, 2_657);
        assert_eq!(observation.platform_input.command_limit, 2_048);
        assert_eq!(
            observation.platform_input.retained_byte_limit,
            8 * MIB as u64
        );
        assert_eq!(observation.control.command_limit, 512);
        assert_eq!(observation.control.retained_byte_limit, 8 * MIB as u64);
        assert_eq!(observation.render_feedback.command_limit, 64);
        assert_eq!(
            observation.render_feedback.retained_byte_limit,
            1_088 * MIB as u64
        );
        assert_eq!(observation.async_completion.command_limit, 32);
        assert_eq!(observation.async_completion.retained_byte_limit, MIB as u64);
        assert_eq!(observation.lifecycle.command_limit, 1);
        assert_eq!(observation.lifecycle.retained_byte_limit, 4 * 1_024);
    }

    #[test]
    fn every_producer_class_admits_its_exact_command_and_byte_limit() {
        let commands = [
            SimulationCommand::FocusChanged(true),
            SimulationCommand::Control(ControlWriteRequest::Net(NetControlRequest::RequestStats)),
            SimulationCommand::SurfaceResized(PhysicalSize::new(800, 600)),
            SimulationCommand::ServicesBlockListCompletion(
                crate::services_block_list_runtime::ServicesBlockListCompletion {
                    generation: 1,
                    result: Ok(Vec::new()),
                    next_refresh_epoch_seconds: None,
                    completed_epoch_seconds: 1,
                },
            ),
            SimulationCommand::Shutdown,
        ];

        for command in commands {
            let producer_class = command.producer_class();
            let retained_bytes = command.retained_logical_bytes();
            let mailbox =
                SimulationMailbox::with_limits(limits_for(producer_class, 1, retained_bytes));
            let receipt = mailbox.enqueue(command).unwrap();
            assert_eq!(receipt.pending, 1);
            assert_eq!(receipt.retained_bytes, retained_bytes);
            let class = match producer_class {
                SimulationCommandProducerClass::PlatformInput => {
                    mailbox.observation().platform_input
                }
                SimulationCommandProducerClass::Control => mailbox.observation().control,
                SimulationCommandProducerClass::RenderFeedback => {
                    mailbox.observation().render_feedback
                }
                SimulationCommandProducerClass::AsyncCompletion => {
                    mailbox.observation().async_completion
                }
                SimulationCommandProducerClass::Lifecycle => mailbox.observation().lifecycle,
            };
            assert_eq!(class.command_limit, 1);
            assert_eq!(class.retained_byte_limit, retained_bytes as u64);
            assert_eq!(class.pending, 1);
            assert_eq!(class.retained_bytes, retained_bytes as u64);
            mailbox.drain_through_barrier();
            assert_eq!(mailbox.observation().pending, 0);
        }
    }

    #[test]
    fn exact_retained_limit_is_admitted_and_oversized_command_is_rejected() {
        let empty = SimulationCommand::ImeCommit(String::new()).retained_logical_bytes();
        let exact = String::with_capacity(32);
        let limit = empty + exact.capacity();
        let mailbox = SimulationMailbox::with_limits(limits_for(
            SimulationCommandProducerClass::PlatformInput,
            2,
            limit,
        ));

        let receipt = mailbox
            .enqueue(SimulationCommand::ImeCommit(exact))
            .unwrap();
        assert_eq!(receipt.retained_bytes, limit);
        mailbox.drain_through_barrier();

        let oversized = String::with_capacity(64);
        let oversized_retained_bytes = empty + oversized.capacity();
        assert_eq!(
            mailbox.enqueue(SimulationCommand::ImeCommit(oversized)),
            Err(SimulationMailboxSubmitError::Oversized {
                producer_class: SimulationCommandProducerClass::PlatformInput,
                retained_bytes: oversized_retained_bytes,
                retained_byte_limit: limit,
            })
        );
        let observation = mailbox.observation();
        assert_eq!(observation.peak_retained_bytes, limit as u64);
        assert_eq!(observation.oversized_rejections, 1);
    }

    #[test]
    fn lifecycle_partition_reserves_shutdown_behind_saturated_input() {
        let retained_bytes = SimulationCommand::FocusChanged(true).retained_logical_bytes();
        let mailbox = SimulationMailbox::with_limits(limits_for(
            SimulationCommandProducerClass::PlatformInput,
            1,
            retained_bytes,
        ));
        assert_eq!(
            enqueue(&mailbox, SimulationCommand::FocusChanged(true)).sequence,
            1
        );

        assert_eq!(
            ControlRequestSink::submit(&mailbox, ControlWriteRequest::Shutdown)
                .unwrap()
                .pending,
            2
        );
        let drained = mailbox.drain_through_barrier();
        assert!(matches!(drained[1].command, SimulationCommand::Shutdown));
    }

    #[test]
    fn saturated_input_partition_backpressures_until_release_can_queue() {
        let mailbox = SimulationMailbox::with_limits(limits_for(
            SimulationCommandProducerClass::PlatformInput,
            2,
            MIB,
        ));
        enqueue(&mailbox, SimulationCommand::FocusChanged(true));
        enqueue(
            &mailbox,
            SimulationCommand::Key {
                physical_key: PhysicalKey::Code(KeyCode::KeyW),
                state: ElementState::Pressed,
                clipboard_text: None,
            },
        );

        let producer_mailbox = mailbox.clone();
        let producer = std::thread::spawn(move || {
            producer_mailbox.enqueue(SimulationCommand::Key {
                physical_key: PhysicalKey::Code(KeyCode::KeyW),
                state: ElementState::Released,
                clipboard_text: None,
            })
        });
        wait_for_blocked_submission(&mailbox);
        assert!(!producer.is_finished());
        assert_eq!(mailbox.observation().platform_input.blocked_submissions, 1);

        assert_eq!(mailbox.drain_through_barrier().len(), 2);
        let receipt = producer.join().unwrap().unwrap();
        assert_eq!(receipt.sequence, 3);
        let release = mailbox.drain_through_barrier();
        assert!(matches!(
            release[0].command,
            SimulationCommand::Key {
                state: ElementState::Released,
                ..
            }
        ));
        let observation = mailbox.observation();
        assert_eq!(observation.peak_pending, 2);
        assert_eq!(observation.blocked_submissions, 1);
    }

    #[test]
    fn close_unblocks_a_saturated_producer_and_rejects_control() {
        let mailbox = SimulationMailbox::with_limits(limits_for(
            SimulationCommandProducerClass::PlatformInput,
            1,
            MIB,
        ));
        enqueue(&mailbox, SimulationCommand::FocusChanged(true));

        let producer_mailbox = mailbox.clone();
        let producer = std::thread::spawn(move || {
            producer_mailbox.enqueue(SimulationCommand::FocusChanged(false))
        });
        wait_for_blocked_submission(&mailbox);
        assert!(!producer.is_finished());
        mailbox.close();

        assert_eq!(
            producer.join().unwrap(),
            Err(SimulationMailboxSubmitError::Closed)
        );
        assert_eq!(
            ControlRequestSink::submit(
                &mailbox,
                ControlWriteRequest::Net(NetControlRequest::RequestStats),
            ),
            Err(ControlSubmitError::Closed)
        );
        let observation = mailbox.observation();
        assert_eq!(observation.blocked_submissions, 1);
        assert_eq!(observation.closed_rejections, 2);
    }

    #[test]
    fn control_submission_maps_oversized_retained_bytes() {
        let request = ControlWriteRequest::Net(NetControlRequest::ChatMessage {
            message: String::with_capacity(64),
        });
        let retained_bytes = std::mem::size_of::<SequencedSimulationCommand>()
            + request.retained_logical_bytes()
            - std::mem::size_of::<ControlWriteRequest>();
        let retained_byte_limit = retained_bytes - 1;
        let mailbox = SimulationMailbox::with_limits(limits_for(
            SimulationCommandProducerClass::Control,
            1,
            retained_byte_limit,
        ));

        assert_eq!(
            ControlRequestSink::submit(&mailbox, request),
            Err(ControlSubmitError::Oversized {
                retained_bytes,
                retained_byte_limit,
            })
        );
        assert_eq!(mailbox.observation().oversized_rejections, 1);
    }

    #[test]
    fn resource_reload_ack_charges_generation_logical_bytes() {
        let diagnostics = bbb_control::ClientResourceDiagnostics {
            data_logical_charged_bytes: 1_024,
            image_rgba_charged_bytes: 2_048,
            ..bbb_control::ClientResourceDiagnostics::default()
        };
        let error = String::with_capacity(32);
        let command = SimulationCommand::ResourceReloadAck(Box::new(
            crate::resource_reload::ResourceReloadAck {
                generation: 7,
                diagnostics,
                result: Err(error),
            },
        ));

        assert_eq!(
            command.producer_class(),
            SimulationCommandProducerClass::RenderFeedback
        );
        assert_eq!(
            command.retained_logical_bytes(),
            std::mem::size_of::<SequencedSimulationCommand>()
                + std::mem::size_of::<crate::resource_reload::ResourceReloadAck>()
                + 1_024
                + 2_048
                + 32
        );
    }

    #[test]
    fn production_producers_cannot_restore_ignored_mailbox_results() {
        for (name, source) in [
            ("main", include_str!("../main.rs")),
            ("runtime", include_str!("../runtime.rs")),
            (
                "services",
                include_str!("../services_block_list_runtime.rs"),
            ),
            ("thread", include_str!("thread.rs")),
        ] {
            let production = source.split("#[cfg(test)]").next().unwrap();
            assert!(
                !production.contains(".enqueue("),
                "{name} restored an unchecked mailbox enqueue"
            );
        }
        assert!(include_str!("../main.rs").contains("enqueue_or_report("));
        assert!(include_str!("../runtime.rs").contains("enqueue_or_report("));
        assert!(include_str!("../services_block_list_runtime.rs").contains("enqueue_or_report("));
        assert!(include_str!("thread.rs").contains("enqueue_or_report("));
    }

    #[test]
    fn commands_from_winit_and_control_share_one_monotonic_sequence() {
        let mailbox = SimulationMailbox::default();
        assert_eq!(
            mailbox
                .enqueue(SimulationCommand::FocusChanged(true))
                .unwrap()
                .sequence,
            1
        );
        ControlRequestSink::submit(
            &mailbox,
            ControlWriteRequest::Net(NetControlRequest::RequestStats),
        )
        .unwrap();
        enqueue(
            &mailbox,
            SimulationCommand::Key {
                physical_key: PhysicalKey::Code(KeyCode::KeyW),
                state: ElementState::Released,
                clipboard_text: None,
            },
        );

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
        enqueue(
            &mailbox,
            SimulationCommand::MouseMotion { delta: (1.0, 2.0) },
        );
        enqueue(
            &mailbox,
            SimulationCommand::Key {
                physical_key: PhysicalKey::Code(KeyCode::KeyW),
                state: ElementState::Released,
                clipboard_text: None,
            },
        );
        enqueue(
            &mailbox,
            SimulationCommand::MouseButton {
                button: MouseButton::Left,
                state: ElementState::Released,
                chat_target: None,
                book_target: None,
                disconnected_screen_target: None,
                death_screen_target: None,
                game_event_target: None,
                player_reporting_target: None,
                social_interactions_target: None,
            },
        );
        ControlRequestSink::submit(
            &mailbox,
            ControlWriteRequest::Input(InputControlRequest::Key {
                key: InputKeyControl::Forward,
                pressed: false,
            }),
        )
        .unwrap();
        ControlRequestSink::submit(&mailbox, ControlWriteRequest::Shutdown).unwrap();

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
        enqueue(
            &mailbox,
            SimulationCommand::MouseMotion { delta: (1.0, 2.0) },
        );
        enqueue(
            &mailbox,
            SimulationCommand::MouseMotion { delta: (3.0, 4.0) },
        );
        enqueue(&mailbox, SimulationCommand::FocusChanged(true));
        enqueue(
            &mailbox,
            SimulationCommand::MouseMotion { delta: (5.0, 6.0) },
        );
        enqueue(
            &mailbox,
            SimulationCommand::MouseMotion { delta: (7.0, 8.0) },
        );

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
        enqueue(
            &mailbox,
            SimulationCommand::CursorMoved {
                position: PhysicalPosition::new(10.0, 20.0),
                disconnected_screen_target: None,
                death_screen_target: Some(HudDeathScreenHitTarget {
                    generation: 7,
                    kind: HudDeathScreenHitTargetKind::Button(HudDeathScreenButton::Respawn),
                }),
                game_event_target: None,
                player_reporting_target: None,
                social_interactions_target: None,
            },
        );
        let latest = HudDeathScreenHitTarget {
            generation: 8,
            kind: HudDeathScreenHitTargetKind::CauseLink { run_index: 3 },
        };
        enqueue(
            &mailbox,
            SimulationCommand::CursorMoved {
                position: PhysicalPosition::new(30.0, 40.0),
                disconnected_screen_target: None,
                death_screen_target: Some(latest),
                game_event_target: None,
                player_reporting_target: None,
                social_interactions_target: None,
            },
        );
        enqueue(
            &mailbox,
            SimulationCommand::MouseButton {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                chat_target: None,
                book_target: None,
                disconnected_screen_target: None,
                death_screen_target: Some(latest),
                game_event_target: None,
                player_reporting_target: None,
                social_interactions_target: None,
            },
        );

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
    fn disconnected_screen_targets_survive_cursor_coalescing_and_mouse_barrier() {
        use bbb_renderer::{HudDisconnectedScreenButton, HudDisconnectedScreenHitTarget};

        let mailbox = SimulationMailbox::with_coalescing_threshold(1);
        enqueue(
            &mailbox,
            SimulationCommand::CursorMoved {
                position: PhysicalPosition::new(10.0, 20.0),
                disconnected_screen_target: Some(HudDisconnectedScreenHitTarget {
                    generation: 7,
                    button: HudDisconnectedScreenButton::Back,
                }),
                death_screen_target: None,
                game_event_target: None,
                player_reporting_target: None,
                social_interactions_target: None,
            },
        );
        let latest = HudDisconnectedScreenHitTarget {
            generation: 8,
            button: HudDisconnectedScreenButton::OpenReportDirectory,
        };
        enqueue(
            &mailbox,
            SimulationCommand::CursorMoved {
                position: PhysicalPosition::new(30.0, 40.0),
                disconnected_screen_target: Some(latest),
                death_screen_target: None,
                game_event_target: None,
                player_reporting_target: None,
                social_interactions_target: None,
            },
        );
        enqueue(
            &mailbox,
            SimulationCommand::MouseButton {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                chat_target: None,
                book_target: None,
                disconnected_screen_target: Some(latest),
                death_screen_target: None,
                game_event_target: None,
                player_reporting_target: None,
                social_interactions_target: None,
            },
        );

        let drained = mailbox.drain_through_barrier();
        assert_eq!(drained.len(), 2);
        assert!(matches!(
            drained[0].command,
            SimulationCommand::CursorMoved {
                position,
                disconnected_screen_target: Some(target),
                ..
            } if position == PhysicalPosition::new(30.0, 40.0) && target == latest
        ));
        assert!(matches!(
            drained[1].command,
            SimulationCommand::MouseButton {
                disconnected_screen_target: Some(target),
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
        enqueue(
            &mailbox,
            SimulationCommand::CursorMoved {
                position: PhysicalPosition::new(10.0, 20.0),
                disconnected_screen_target: None,
                death_screen_target: None,
                game_event_target: None,
                player_reporting_target: None,
                social_interactions_target: None,
            },
        );
        let latest = HudSocialInteractionsHitTarget {
            generation: 8,
            kind: HudSocialInteractionsHitTargetKind::Focus(HudSocialInteractionsFocus::Search),
        };
        enqueue(
            &mailbox,
            SimulationCommand::CursorMoved {
                position: PhysicalPosition::new(30.0, 40.0),
                disconnected_screen_target: None,
                death_screen_target: None,
                game_event_target: None,
                player_reporting_target: None,
                social_interactions_target: Some(latest),
            },
        );
        enqueue(
            &mailbox,
            SimulationCommand::MouseButton {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                chat_target: None,
                book_target: None,
                disconnected_screen_target: None,
                death_screen_target: None,
                game_event_target: None,
                player_reporting_target: None,
                social_interactions_target: Some(latest),
            },
        );

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
        enqueue(
            &mailbox,
            SimulationCommand::GameEventScreenRenderDelta {
                generation: 7,
                delta_ticks: 0.25,
            },
        );
        enqueue(
            &mailbox,
            SimulationCommand::GameEventScreenRenderDelta {
                generation: 7,
                delta_ticks: 0.75,
            },
        );
        enqueue(
            &mailbox,
            SimulationCommand::GameEventScreenRenderDelta {
                generation: 8,
                delta_ticks: 0.5,
            },
        );

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
                )
                .unwrap();
            }
        });
        for index in 0..32 {
            enqueue(&mailbox, SimulationCommand::FocusChanged(index % 2 == 0));
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
