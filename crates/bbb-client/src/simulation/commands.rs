//! Commands queued for the next simulation step.

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc, Mutex,
    },
    time::Duration,
};

use crate::control::{
    ControlCompletionSender, ControlRequestSink, ControlSubmitError, ControlSubmitReceipt,
    ControlWriteRequest,
};
use bbb_client_clock::replay;
use bbb_client_render::{
    HudBookHitTarget, HudChatHitTarget, HudDeathScreenHitTarget, HudDisconnectedScreenHitTarget,
    HudFontGlyphMap, HudGameEventHitTarget, HudSocialInteractionsHitTarget,
};
use bbb_protocol::packets::{ClientInformation, Vec3d};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, MouseScrollDelta},
    event_loop::EventLoopProxy,
    keyboard::PhysicalKey,
};

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

/// One window, render, platform or control value applied by the next
/// [`super::SimulationCore`] step.
#[derive(Debug)]
pub(crate) enum SimulationCommand {
    WithCompletion {
        command: Box<SimulationCommand>,
        completion: ControlCompletionSender,
    },
    FocusChanged(bool),
    Key {
        physical_key: PhysicalKey,
        state: ElementState,
        clipboard_text: Option<String>,
    },
    MouseButton {
        button: MouseButton,
        state: ElementState,
        /// The current logical GUI projection of the raw physical cursor
        /// cache, resolved at click time (`MouseHandler.getScaledXPos`).
        position: Option<PhysicalPosition<f64>>,
        chat_target: Option<HudChatHitTarget>,
        book_target: Option<HudBookHitTarget>,
        disconnected_screen_target: Option<HudDisconnectedScreenHitTarget>,
        command_confirmation_target: Option<bbb_client_render::HudCommandConfirmationHitTarget>,
        death_screen_target: Option<HudDeathScreenHitTarget>,
        game_event_target: Option<HudGameEventHitTarget>,
        social_interactions_target: Option<HudSocialInteractionsHitTarget>,
    },
    MouseMotion {
        delta: (f64, f64),
    },
    MouseWheel {
        delta: MouseScrollDelta,
        position: Option<PhysicalPosition<f64>>,
    },
    CursorMoved {
        position: PhysicalPosition<f64>,
        disconnected_screen_target: Option<HudDisconnectedScreenHitTarget>,
        death_screen_target: Option<HudDeathScreenHitTarget>,
        game_event_target: Option<HudGameEventHitTarget>,
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
    /// The framebuffer size the window owner resized the renderer to; a
    /// zero dimension means minimized.
    SurfaceResized {
        size: winit::dpi::PhysicalSize<u32>,
    },
    TextMeasurements(HudFontGlyphMap),
    HudChatScrollPage(bbb_client_render::HudChatScrollPage),
    HudChatAppendedLines(bbb_client_render::HudChatAppendedLines),
    ScreenshotSaved {
        name: String,
        error: Option<String>,
    },
    RecordingNotice(String),
    /// Render/platform owner completed the `options.txt` write attempt;
    /// Vanilla calls `broadcastOptions` after this point even on failure.
    ClientOptionsSaveCompleted(ClientInformation),
    ResourceReloadAck(Box<crate::resource_reload::ResourceReloadAck>),
    TerrainReadiness {
        level_generation: u64,
        level_load_start: usize,
        player_section_compiled_and_visible: bool,
    },
    DebugProfilingLifecycle(DebugProfilingLifecycle),
    Control(ControlWriteRequest),
    Shutdown,
}

impl SimulationCommand {
    /// Native key text follows the key action; Ctrl shortcuts carry control characters.
    pub(crate) fn native_key_text(state: ElementState, text: Option<&str>) -> Option<Self> {
        if state != ElementState::Pressed {
            return None;
        }
        let text: String = text?
            .chars()
            .filter(|character| !character.is_control())
            .collect();
        (!text.is_empty()).then_some(Self::ImeCommit(text))
    }

    fn from_control(request: ControlWriteRequest) -> Self {
        match request {
            ControlWriteRequest::Shutdown => Self::Shutdown,
            request => Self::Control(request),
        }
    }

    /// Conservatively identifies commands whose application can mutate the
    /// canonical world. The false cases only update input runtime state or
    /// diagnostics, so render caches and the control world stay valid.
    pub(crate) fn requires_world_snapshot_rebuild(&self) -> bool {
        if let Self::WithCompletion { command, .. } = self {
            return command.requires_world_snapshot_rebuild();
        }
        !matches!(
            self,
            Self::SurfaceResized { .. }
                | Self::TextMeasurements(_)
                | Self::HudChatScrollPage(_)
                | Self::ClientOptionsSaveCompleted(_)
                | Self::CursorMoved { .. }
                | Self::CursorLeft
                | Self::MouseMotion { .. }
                | Self::Shutdown
        )
    }

    pub(crate) fn is_platform_input(&self) -> bool {
        if let Self::WithCompletion { command, .. } = self {
            return command.is_platform_input();
        }
        matches!(
            self,
            Self::FocusChanged(_)
                | Self::Key { .. }
                | Self::MouseButton { .. }
                | Self::MouseMotion { .. }
                | Self::MouseWheel { .. }
                | Self::CursorMoved { .. }
                | Self::CursorLeft
                | Self::ImePreedit { .. }
                | Self::ImeCommit(_)
                | Self::ImeDisabled
        )
    }
}

/// Coalesced wake of the client event loop: producers off the loop thread
/// (network, control, async completions) send at most one user event until
/// the loop has drained.
#[derive(Default)]
pub(crate) struct LoopWake {
    proxy: Option<Mutex<EventLoopProxy<()>>>,
    pending: AtomicBool,
}

impl LoopWake {
    pub(crate) fn new(proxy: EventLoopProxy<()>) -> Self {
        Self {
            proxy: Some(Mutex::new(proxy)),
            pending: AtomicBool::new(false),
        }
    }

    pub(crate) fn wake(&self) {
        if self.pending.swap(true, Ordering::AcqRel) {
            return;
        }
        if let Some(proxy) = &self.proxy {
            let proxy = proxy
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let _ = proxy.send_event(());
        }
    }

    /// Called by the loop before it drains, so work arriving afterwards wakes
    /// it again.
    pub(crate) fn rearm(&self) {
        self.pending.store(false, Ordering::Release);
    }
}

/// Producer half of the one FIFO feeding the simulation step.
#[derive(Clone)]
pub(crate) struct CommandQueue {
    sender: mpsc::Sender<SimulationCommand>,
    pending: Arc<AtomicUsize>,
    wake: Arc<LoopWake>,
}

/// Loop-owned consumer half of the command FIFO.
pub(crate) struct CommandInbox {
    receiver: Option<mpsc::Receiver<SimulationCommand>>,
    pending: Arc<AtomicUsize>,
    wake: Arc<LoopWake>,
}

pub(crate) fn command_queue(wake: Arc<LoopWake>) -> (CommandQueue, CommandInbox) {
    let (sender, receiver) = mpsc::channel();
    let pending = Arc::new(AtomicUsize::new(0));
    (
        CommandQueue {
            sender,
            pending: Arc::clone(&pending),
            wake: Arc::clone(&wake),
        },
        CommandInbox {
            receiver: Some(receiver),
            pending,
            wake,
        },
    )
}

impl CommandQueue {
    /// Queues `command` for the next step; `false` once the loop is gone.
    pub(crate) fn push(&self, command: SimulationCommand) -> bool {
        if self.sender.send(command).is_err() {
            return false;
        }
        self.pending.fetch_add(1, Ordering::AcqRel);
        replay::count(&replay::COMMANDS_QUEUED);
        self.wake.wake();
        true
    }
}

impl CommandInbox {
    /// Takes every queued command in FIFO order. The returned count is the
    /// lockstep queue counter to publish once these commands are applied.
    pub(crate) fn drain(&self) -> (Vec<SimulationCommand>, u64) {
        self.wake.rearm();
        let queued = replay::get(&replay::COMMANDS_QUEUED);
        let commands = self
            .receiver
            .iter()
            .flat_map(|receiver| receiver.try_iter())
            .collect::<Vec<_>>();
        self.pending.fetch_sub(commands.len(), Ordering::AcqRel);
        (commands, queued)
    }

    pub(crate) fn close(&mut self) {
        self.receiver.take();
    }
}

impl ControlRequestSink for CommandQueue {
    fn submit(
        &self,
        request: ControlWriteRequest,
    ) -> Result<ControlSubmitReceipt, ControlSubmitError> {
        let command = SimulationCommand::from_control(request);
        if !self.push(command) {
            return Err(ControlSubmitError::Closed);
        }
        Ok(ControlSubmitReceipt {
            pending: self.pending.load(Ordering::Acquire),
        })
    }

    fn submit_confirmed(
        &self,
        request: ControlWriteRequest,
        completion: ControlCompletionSender,
    ) -> Result<ControlSubmitReceipt, ControlSubmitError> {
        let command = SimulationCommand::from_control(request);
        if !self.push(SimulationCommand::WithCompletion {
            command: Box::new(command),
            completion,
        }) {
            return Err(ControlSubmitError::Closed);
        }
        Ok(ControlSubmitReceipt {
            pending: self.pending.load(Ordering::Acquire),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_drain_in_fifo_order_and_wake_once_until_rearmed() {
        let wake = Arc::new(LoopWake::default());
        let (queue, inbox) = command_queue(Arc::clone(&wake));
        assert!(queue.push(SimulationCommand::FocusChanged(true)));
        assert!(wake.pending.load(Ordering::Acquire));
        let receipt = queue
            .submit(ControlWriteRequest::Shutdown)
            .expect("open queue accepts control writes");
        assert_eq!(receipt.pending, 2);

        let (drained, _) = inbox.drain();
        assert!(!wake.pending.load(Ordering::Acquire));
        assert!(matches!(
            drained.as_slice(),
            [
                SimulationCommand::FocusChanged(true),
                SimulationCommand::Shutdown
            ]
        ));
        drop(inbox);
        assert!(!queue.push(SimulationCommand::FocusChanged(false)));
        assert_eq!(
            queue.submit(ControlWriteRequest::LockstepWake),
            Err(ControlSubmitError::Closed)
        );
    }
}
