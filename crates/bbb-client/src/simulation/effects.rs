//! Side effects a simulation step asks the loop to apply after it returns.

use std::path::PathBuf;

use bbb_client_assets::{JukeboxSongRegistry, SoundEventRegistry};

use crate::resource_pack_runtime::ServerResourcePackReload;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugProfilingCommand {
    Start,
    Stop,
}

/// Render/winit-owned work requested by the simulation input path.
#[derive(Debug)]
pub(crate) enum PlatformEffect {
    SetCursorCapture(bool),
    /// `TextInputManager.startTextInput`/`stopTextInput` plus the `tick`
    /// that revokes the IME outside text input.
    /// Vanilla keeps the IME off unless a text field has focus, and with an
    /// input method actually installed that is load-bearing: an IME grabs
    /// every letter key it is offered.
    SetTextInputEnabled(bool),
    ClipboardWrite(bbb_protocol::JavaString),
    /// `Util.getPlatform().openUri`. The units survive to the process
    /// argument, where the JDK's encoder turns an unpaired surrogate into `?`.
    OpenUri(bbb_protocol::JavaString),
    Screenshot {
        path: PathBuf,
        completion: Option<crate::control::ControlCompletionSender>,
    },
    Recording {
        request: crate::control::RecordingControlRequest,
        completion: Option<crate::control::ControlCompletionSender>,
    },
    ToggleRecording,
    ReloadTerrain,
    DumpDynamicTextures {
        requests: u32,
    },
    /// Replace the render/resource owner's selected locale before the
    /// terminal resource-reload effect consumes it.
    SetClientLocale(String),
    /// `KeyboardHandler`'s `options.save()` after a debug hotkey flipped a
    /// persisted field.
    SaveClientOptions {
        writes: Vec<crate::options::ClientOptionWrite>,
        /// `Options.save` always broadcasts after the write attempt finishes,
        /// including when the file owner is absent or the write fails.
        broadcast: bbb_protocol::packets::ClientInformation,
    },
    ProfilingToggled {
        commands: Vec<DebugProfilingCommand>,
    },
    NavigateProfiler {
        digits: Vec<u8>,
    },
    ScreenNarrationInput {
        target: crate::narrator_runtime::ScreenNarrationTarget,
        input: crate::narrator_runtime::ScreenNarrationInput,
    },
    CommandConfirmationInitialized {
        frame:
            Box<crate::narrator_runtime::command_confirmation::CommandConfirmationNarrationFrame>,
        last_input_type: crate::input::LastInputType,
        at: bbb_client_clock::Instant,
    },
    OpenPath(PathBuf),
    ChatNarration {
        kind: bbb_client_world::ChatNarrationKind,
        text: String,
    },
    /// `KeyboardHandler`'s Ctrl+B narrator hotkey, carrying the cycled status
    /// and `Screen.updateNarratorStatus`'s re-enable narration schedule.
    /// `KeyboardHandler`'s `keyScreenshot` branch, which reaches
    /// `Screenshot.grab` for the main render target.
    GrabScreenshot,
    /// `KeyboardHandler`'s `keyFullscreen` branch, which flips
    /// `Window.toggleFullScreen` before saving the option.
    SetFullscreen(bool),
    NarratorStatusChanged {
        requests: Vec<crate::input::NarratorStatusRequest>,
    },
    Reconnect,
    Exit,
}

/// Platform effects a step requested, applied in order by the loop once the
/// step returns.
#[derive(Debug, Default)]
pub(crate) struct PlatformEffects(Vec<PlatformEffect>);

impl PlatformEffects {
    pub(crate) fn submit(&mut self, effect: PlatformEffect) {
        self.0.push(effect);
    }

    pub(crate) fn take(&mut self) -> Vec<PlatformEffect> {
        std::mem::take(&mut self.0)
    }
}

/// Resource reload a step requested, applied by the loop after the step's
/// other effects and before the next step.
pub(crate) struct ResourceReloadEffect {
    pub(crate) request: ServerResourcePackReload,
    pub(crate) sound_events: SoundEventRegistry,
    pub(crate) jukebox_songs: JukeboxSongRegistry,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_confirmation_initializations_survive_unrendered_close_in_order() {
        use crate::input::{ClientInputState, FallbackTextMeasurer, LastInputType};
        use crate::narrator_runtime::{
            NarratorStatus, ScreenNarrationFrames, ScreenNarrations, SystemNarrator,
        };
        #[derive(Default)]
        struct Speech(Vec<String>);
        impl SystemNarrator for Speech {
            fn say_system_now(&mut self, text: &str) {
                self.0.push(text.into());
            }
            fn say_chat_queued(&mut self, _: &str) {}
            fn say_system_chat_queued(&mut self, _: &str) {}
            fn say_system_queued(&mut self, _: &str) {}
            fn update_status(&mut self, _: NarratorStatus) {}
        }
        let mut input = ClientInputState::default();
        let mut world = bbb_client_world::WorldStore::new();
        let mut counters = crate::control::NetCounters::default();
        input.start_unattended_command(&"/first".into(), &mut world, &mut counters, &None);
        let first = input.command_confirmation().unwrap().generation;
        input.resolve_command_confirmation(
            false,
            &mut world,
            &mut counters,
            &None,
            &FallbackTextMeasurer,
        );
        assert!(input.command_confirmation().is_none());
        input.start_unattended_command(&"/second".into(), &mut world, &mut counters, &None);
        let second = input.command_confirmation().unwrap().generation;
        input.handle_command_confirmation_key(
            &mut world,
            &mut counters,
            &None,
            winit::keyboard::KeyCode::Tab,
            true,
            &FallbackTextMeasurer,
        );
        let mut platform = PlatformEffects::default();
        crate::runtime::flush_command_confirmation_narrations(&mut input, None, &mut platform);
        let effects = platform.take();
        let frames = effects
            .iter()
            .map(|effect| match effect {
                PlatformEffect::CommandConfirmationInitialized { frame, .. } => {
                    frame.as_ref().clone()
                }
                _ => panic!("unexpected effect"),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            frames
                .iter()
                .map(|frame| frame.generation)
                .collect::<Vec<_>>(),
            [first, second]
        );
        assert_eq!(frames[1].focused, None);
        assert!(input.command_confirmation().unwrap().focused.is_some());
        let mut runtime = ScreenNarrations::default();
        let mut speech = Speech::default();
        for effect in effects {
            if let PlatformEffect::CommandConfirmationInitialized {
                frame,
                last_input_type,
                at,
            } = effect
            {
                runtime.initialize_confirmation(&frame, at, last_input_type, Some(&mut speech));
            }
        }
        assert_eq!(speech.0.len(), 2);
        let mut published = ScreenNarrationFrames::default();
        published.confirmation = Some(frames[1].clone());
        runtime.tick(
            &published,
            bbb_client_clock::Instant::now(),
            LastInputType::Mouse,
            Some(&mut speech),
        );
        assert_eq!(speech.0.len(), 2);
        assert!(input.take_command_confirmation_narrations().is_empty());
    }
}
