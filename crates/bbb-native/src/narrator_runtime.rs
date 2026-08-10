use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use crate::input::LastInputType;

const SCREEN_NARRATION_SUPPRESS_AFTER_INIT: Duration = Duration::from_secs(2);
const SCREEN_NARRATION_MOUSE_MOVE_DELAY: Duration = Duration::from_millis(750);
const SCREEN_NARRATION_ACTION_DELAY: Duration = Duration::from_millis(200);

/// Vanilla's four narrator modes. The TTS backend is a runtime/platform concern, while callers
/// identify whether a message is system or chat narration before submitting it here.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum NarratorStatus {
    #[default]
    Off,
    All,
    Chat,
    System,
}

impl NarratorStatus {
    pub(crate) fn option_name(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::All => "Narrates All",
            Self::Chat => "Narrates Chat",
            Self::System => "Narrates System",
        }
    }

    pub(crate) fn id(self) -> i32 {
        match self {
            Self::Off => 0,
            Self::All => 1,
            Self::Chat => 2,
            Self::System => 3,
        }
    }

    pub(crate) fn next(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::Chat,
            Self::Chat => Self::System,
            Self::System => Self::Off,
        }
    }

    fn should_narrate_chat(self) -> bool {
        matches!(self, Self::All | Self::Chat)
    }

    fn should_narrate_system(self) -> bool {
        matches!(self, Self::All | Self::System)
    }

    fn should_narrate_system_or_chat(self) -> bool {
        !matches!(self, Self::Off)
    }
}

/// Narrow runtime boundary matching Vanilla's interrupting system narration
/// and its three non-interrupting chat/system queue entry points.
pub(crate) trait SystemNarrator {
    fn say_system_now(&mut self, message: &str);
    fn say_chat_queued(&mut self, message: &str);
    fn say_system_chat_queued(&mut self, message: &str);
    fn say_system_queued(&mut self, message: &str);
    fn update_status(&mut self, status: NarratorStatus);
}

pub(crate) fn submit_chat_narration(
    narrator: &mut dyn SystemNarrator,
    kind: bbb_world::ChatNarrationKind,
    message: &str,
) {
    match kind {
        bbb_world::ChatNarrationKind::Chat => narrator.say_chat_queued(message),
        bbb_world::ChatNarrationKind::SystemChat => narrator.say_system_chat_queued(message),
        bbb_world::ChatNarrationKind::System => narrator.say_system_queued(message),
        bbb_world::ChatNarrationKind::SystemNow => narrator.say_system_now(message),
    }
}

/// Vanilla orders collected screen narration first by element kind and then by
/// nesting depth, with at most one value retained for each pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum NarratedElementKind {
    Title,
    Position,
    Hint,
    Usage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScreenNarrationElement {
    kind: NarratedElementKind,
    depth: u8,
    text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ScreenNarrationFrame {
    elements: Vec<ScreenNarrationElement>,
}

impl ScreenNarrationFrame {
    pub(crate) fn add(&mut self, kind: NarratedElementKind, depth: u8, text: impl Into<String>) {
        let text = text.into();
        if !text.is_empty() {
            self.elements
                .push(ScreenNarrationElement { kind, depth, text });
        }
    }

    #[cfg(test)]
    pub(crate) fn all_text_for_test(&self) -> String {
        ScreenNarrationCollector::default()
            .update_and_collect(self, true)
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ScreenNarrationEntry {
    text: String,
    generation: u64,
    already_narrated: bool,
}

#[derive(Debug, Default)]
struct ScreenNarrationCollector {
    generation: u64,
    entries: BTreeMap<(NarratedElementKind, u8), ScreenNarrationEntry>,
}

impl ScreenNarrationCollector {
    fn update_and_collect(&mut self, frame: &ScreenNarrationFrame, force: bool) -> Option<String> {
        self.generation = self.generation.checked_add(1).unwrap_or_else(|| {
            self.entries.clear();
            1
        });
        for element in &frame.elements {
            let entry = self
                .entries
                .entry((element.kind, element.depth))
                .or_default();
            if entry.text != element.text
                || entry.generation.checked_add(1) != Some(self.generation)
            {
                entry.already_narrated = false;
            }
            entry.text.clone_from(&element.text);
            entry.generation = self.generation;
        }
        let narration = self
            .entries
            .values_mut()
            .filter(|entry| {
                entry.generation == self.generation && (force || !entry.already_narrated)
            })
            .map(|entry| {
                entry.already_narrated = true;
                entry.text.as_str()
            })
            .collect::<Vec<_>>()
            .join(". ");
        (!narration.is_empty()).then_some(narration)
    }

    fn clear(&mut self) {
        self.generation = 0;
        self.entries.clear();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScreenNarrationInput {
    Immediate,
    MouseMove,
    MouseAction,
    KeyboardAction,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ScreenNarrationSuppression {
    #[default]
    None,
    Until(Instant),
    UntilNextAction,
}

impl ScreenNarrationSuppression {
    fn blocks(self, now: Instant) -> bool {
        match self {
            Self::None => false,
            Self::Until(until) => now <= until,
            Self::UntilNextAction => true,
        }
    }
}

/// Per-screen delayed narration owner matching `Screen.scheduleNarration` and
/// `ScreenNarrationCollector`. The latest localized frame remains in native
/// runtime state; canonical world and renderer state never own speech timing.
#[derive(Debug, Default)]
pub(crate) struct ScreenNarrationRuntime {
    collector: ScreenNarrationCollector,
    active: bool,
    suppression: ScreenNarrationSuppression,
    narrate_at: Option<Instant>,
}

impl ScreenNarrationRuntime {
    pub(crate) fn after_input(&mut self, input: ScreenNarrationInput, now: Instant) {
        if !self.active {
            return;
        }
        let (delay, ignore_suppression) = match input {
            ScreenNarrationInput::Immediate => (Duration::ZERO, true),
            ScreenNarrationInput::MouseMove => (SCREEN_NARRATION_MOUSE_MOVE_DELAY, false),
            ScreenNarrationInput::MouseAction | ScreenNarrationInput::KeyboardAction => {
                (SCREEN_NARRATION_ACTION_DELAY, true)
            }
        };
        self.narrate_at = now.checked_add(delay);
        if ignore_suppression {
            self.suppression = ScreenNarrationSuppression::None;
        }
    }

    pub(crate) fn schedule_after_narrator_enabled(&mut self, now: Instant) {
        if !self.active {
            return;
        }
        self.narrate_at = now.checked_add(SCREEN_NARRATION_SUPPRESS_AFTER_INIT);
        self.suppression = ScreenNarrationSuppression::None;
    }

    pub(crate) fn tick(
        &mut self,
        frame: Option<&ScreenNarrationFrame>,
        now: Instant,
        last_input_type: LastInputType,
        narrator: Option<&mut dyn SystemNarrator>,
    ) {
        let Some(frame) = frame else {
            self.reset();
            return;
        };
        if !self.active {
            self.active = true;
            self.suppression = if last_input_type.is_keyboard() {
                ScreenNarrationSuppression::UntilNextAction
            } else {
                now.checked_add(SCREEN_NARRATION_SUPPRESS_AFTER_INIT)
                    .map_or(ScreenNarrationSuppression::None, |until| {
                        ScreenNarrationSuppression::Until(until)
                    })
            };
            let text = self.collector.update_and_collect(frame, true);
            if let (Some(narrator), Some(text)) = (narrator, text) {
                narrator.say_system_now(&text);
            }
            return;
        }
        let Some(narrate_at) = self.narrate_at else {
            return;
        };
        if now <= narrate_at || self.suppression.blocks(now) {
            return;
        }
        self.narrate_at = None;
        let text = self.collector.update_and_collect(frame, false);
        if let (Some(narrator), Some(text)) = (narrator, text) {
            narrator.say_system_now(&text);
        }
    }

    fn reset(&mut self) {
        self.collector.clear();
        self.active = false;
        self.suppression = ScreenNarrationSuppression::None;
        self.narrate_at = None;
    }
}

/// A process-backed TTS adapter. The executable and arguments come only from startup CLI
/// configuration. Every exact `{text}` argument is replaced with the narration; when no
/// placeholder is present, the narration is appended as the final argument. Every exact
/// `{volume}` argument is replaced with the narrator volume — vanilla
/// `GameNarrator.narrateMessage` passes
/// `options.getFinalSoundSourceVolume(SoundSource.VOICE)` (voice times master) with
/// every `Narrator.say`; a command without the placeholder simply does not consume it.
struct CommandTextToSpeech {
    executable: PathBuf,
    arguments: Vec<String>,
    volume: f32,
    pending: VecDeque<String>,
    child: Option<Child>,
}

impl CommandTextToSpeech {
    fn new(executable: PathBuf, arguments: Vec<String>, volume: f32) -> Self {
        Self {
            executable,
            arguments,
            volume,
            pending: VecDeque::new(),
            child: None,
        }
    }

    /// `GameNarrator.narrateMessage`. Its `logNarratedMessage` companion is
    /// gated on `SharedConstants.IS_RUNNING_IN_IDE`, false in every shipped
    /// vanilla client, so the narrated text is deliberately not logged here —
    /// only process lifecycle failures are.
    fn say(&mut self, message: &str, interrupt: bool) {
        if message.is_empty() {
            return;
        }
        if interrupt {
            self.clear();
        }
        self.pending.push_back(message.to_string());
        self.tick();
    }

    fn clear(&mut self) {
        self.pending.clear();
        let Some(mut child) = self.child.take() else {
            return;
        };
        if let Err(err) = child.kill() {
            tracing::debug!(?err, "narrator process already stopped before clear");
        }
        if let Err(err) = child.wait() {
            tracing::warn!(?err, "failed to reap cleared narrator process");
        }
    }

    fn tick(&mut self) {
        if let Some(child) = self.child.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        tracing::warn!(?status, "narrator process exited unsuccessfully");
                    }
                    self.child = None;
                }
                Ok(None) => return,
                Err(err) => {
                    tracing::warn!(?err, "failed to poll narrator process");
                    self.child = None;
                }
            }
        }
        let Some(message) = self.pending.pop_front() else {
            return;
        };
        let arguments = narrator_command_arguments(&self.arguments, &message, self.volume);
        match Command::new(&self.executable)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => self.child = Some(child),
            Err(err) => {
                tracing::warn!(
                    ?err,
                    executable = %self.executable.display(),
                    "failed to start narrator process"
                );
                self.pending.clear();
            }
        }
    }
}

impl Drop for CommandTextToSpeech {
    fn drop(&mut self) {
        self.clear();
    }
}

fn narrator_command_arguments(arguments: &[String], message: &str, volume: f32) -> Vec<OsString> {
    let mut replaced = false;
    let mut result = arguments
        .iter()
        .map(|argument| {
            if argument == "{text}" {
                replaced = true;
                OsString::from(message)
            } else if argument == "{volume}" {
                OsString::from(format!("{volume}"))
            } else {
                OsString::from(argument)
            }
        })
        .collect::<Vec<_>>();
    if !replaced {
        result.push(OsString::from(message));
    }
    result
}

/// Native owner matching the interrupting system-message subset of vanilla `GameNarrator`.
/// Non-interrupting chat/system queues can reuse the process queue when those consumers land.
pub(crate) struct NativeNarratorRuntime {
    status: NarratorStatus,
    speech: CommandTextToSpeech,
}

impl NativeNarratorRuntime {
    /// `volume` is `getFinalSoundSourceVolume(VOICE)` resolved once at
    /// startup: sound volumes only change through `options.txt` (the in-game
    /// options UI is a non-goal), so there is no runtime volume writer.
    pub(crate) fn new(
        status: NarratorStatus,
        executable: PathBuf,
        arguments: Vec<String>,
        volume: f32,
    ) -> Self {
        Self {
            status,
            speech: CommandTextToSpeech::new(executable, arguments, volume),
        }
    }

    pub(crate) fn tick(&mut self) {
        self.speech.tick();
    }
}

impl SystemNarrator for NativeNarratorRuntime {
    fn update_status(&mut self, status: NarratorStatus) {
        if self.status != NarratorStatus::Off {
            self.speech.clear();
        }
        self.status = status;
        self.speech
            .say(&format!("Narrator : {}", status.option_name()), true);
    }

    fn say_system_now(&mut self, message: &str) {
        if self.status.should_narrate_system() {
            self.speech.say(message, true);
        }
    }

    fn say_chat_queued(&mut self, message: &str) {
        if self.status.should_narrate_chat() {
            self.speech.say(message, false);
        }
    }

    fn say_system_chat_queued(&mut self, message: &str) {
        if self.status.should_narrate_system_or_chat() {
            self.speech.say(message, false);
        }
    }

    fn say_system_queued(&mut self, message: &str) {
        if self.status.should_narrate_system() {
            self.speech.say(message, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingNarrator(Vec<String>);

    impl SystemNarrator for RecordingNarrator {
        fn update_status(&mut self, status: NarratorStatus) {
            self.0.push(format!("status:{}", status.id()));
        }

        fn say_system_now(&mut self, message: &str) {
            self.0.push(message.to_string());
        }

        fn say_chat_queued(&mut self, message: &str) {
            self.0.push(message.to_string());
        }

        fn say_system_chat_queued(&mut self, message: &str) {
            self.0.push(message.to_string());
        }

        fn say_system_queued(&mut self, message: &str) {
            self.0.push(message.to_string());
        }
    }

    #[test]
    fn typed_chat_narration_dispatch_preserves_interrupting_system_requests() {
        #[derive(Default)]
        struct DispatchRecorder(Vec<&'static str>);

        impl SystemNarrator for DispatchRecorder {
            fn update_status(&mut self, _status: NarratorStatus) {
                self.0.push("status");
            }

            fn say_system_now(&mut self, _message: &str) {
                self.0.push("system_now");
            }

            fn say_chat_queued(&mut self, _message: &str) {
                self.0.push("chat");
            }

            fn say_system_chat_queued(&mut self, _message: &str) {
                self.0.push("system_chat");
            }

            fn say_system_queued(&mut self, _message: &str) {
                self.0.push("system");
            }
        }

        let mut recorder = DispatchRecorder::default();
        for kind in [
            bbb_world::ChatNarrationKind::Chat,
            bbb_world::ChatNarrationKind::SystemChat,
            bbb_world::ChatNarrationKind::System,
            bbb_world::ChatNarrationKind::SystemNow,
        ] {
            submit_chat_narration(&mut recorder, kind, "message");
        }
        assert_eq!(recorder.0, ["chat", "system_chat", "system", "system_now"]);
    }

    #[test]
    fn command_arguments_replace_exact_text_placeholder_or_append_message() {
        assert_eq!(
            narrator_command_arguments(
                &["--voice".to_string(), "{text}".to_string()],
                "hello",
                1.0
            ),
            [OsString::from("--voice"), OsString::from("hello")]
        );
        assert_eq!(
            narrator_command_arguments(&["--voice={text}".to_string()], "hello", 1.0),
            [OsString::from("--voice={text}"), OsString::from("hello")]
        );
    }

    #[test]
    fn command_arguments_substitute_the_final_voice_volume_placeholder() {
        assert_eq!(
            narrator_command_arguments(
                &["--volume".to_string(), "{volume}".to_string()],
                "hello",
                0.25
            ),
            [
                OsString::from("--volume"),
                OsString::from("0.25"),
                OsString::from("hello")
            ]
        );
        assert_eq!(
            narrator_command_arguments(&["{text}".to_string()], "hello", 0.25),
            [OsString::from("hello")]
        );
    }

    #[test]
    fn dropping_the_narrator_runtime_kills_and_reaps_the_speaking_child() {
        let mut speech = CommandTextToSpeech::new(
            PathBuf::from("/bin/sh"),
            vec!["-c".to_string(), "sleep 10".to_string()],
            1.0,
        );
        speech.say("spoken", true);
        let id = speech.child.as_ref().expect("child spawned").id();
        drop(speech);
        // `GameNarrator.destroy` tears the TTS backend down; the RAII drop
        // must have killed and reaped the child, so its pid no longer exists
        // (or is at most an unreaped zombie belonging to someone else).
        let alive = std::path::Path::new(&format!("/proc/{id}/stat"))
            .exists()
            .then(|| std::fs::read_to_string(format!("/proc/{id}/stat")).ok())
            .flatten()
            .is_some_and(|stat| !stat.contains(") Z"));
        assert!(!alive, "narrator child {id} survived the runtime drop");
    }

    #[test]
    fn narrator_status_filters_interrupting_system_messages() {
        let executable = PathBuf::from("/bin/sh");
        let arguments = vec![
            "-c".to_string(),
            "sleep 10".to_string(),
            "--".to_string(),
            "{text}".to_string(),
        ];
        let mut off = NativeNarratorRuntime::new(
            NarratorStatus::Off,
            executable.clone(),
            arguments.clone(),
            1.0,
        );
        off.say_system_now("ignored");
        assert!(off.speech.pending.is_empty());
        assert!(off.speech.child.is_none());

        let mut chat = NativeNarratorRuntime::new(
            NarratorStatus::Chat,
            executable.clone(),
            arguments.clone(),
            1.0,
        );
        chat.say_system_now("ignored");
        assert!(chat.speech.pending.is_empty());
        assert!(chat.speech.child.is_none());

        let mut all = NativeNarratorRuntime::new(
            NarratorStatus::All,
            executable.clone(),
            arguments.clone(),
            1.0,
        );
        all.say_system_now("spoken");
        assert!(all.speech.pending.is_empty());
        assert!(all.speech.child.is_some());
        all.speech.clear();

        let mut system =
            NativeNarratorRuntime::new(NarratorStatus::System, executable, arguments, 1.0);
        system.say_system_now("spoken");
        assert!(system.speech.pending.is_empty());
        assert!(system.speech.child.is_some());
    }

    #[test]
    fn chat_narration_modes_queue_without_interrupting_current_speech() {
        let executable = PathBuf::from("/bin/sh");
        let arguments = vec![
            "-c".to_string(),
            "sleep 1".to_string(),
            "--".to_string(),
            "{text}".to_string(),
        ];
        let mut chat = NativeNarratorRuntime::new(
            NarratorStatus::Chat,
            executable.clone(),
            arguments.clone(),
            1.0,
        );
        chat.say_chat_queued("chat one");
        chat.say_system_chat_queued("system chat two");
        assert!(chat.speech.child.is_some());
        assert_eq!(chat.speech.pending, ["system chat two"]);
        chat.say_system_queued("ignored system");
        assert_eq!(chat.speech.pending, ["system chat two"]);
        chat.speech.clear();

        let mut system =
            NativeNarratorRuntime::new(NarratorStatus::System, executable, arguments, 1.0);
        system.say_chat_queued("ignored chat");
        assert!(system.speech.child.is_none());
        system.say_system_chat_queued("system chat");
        system.say_system_queued("system queued");
        assert!(system.speech.child.is_some());
        assert_eq!(system.speech.pending, ["system queued"]);
        system.speech.clear();
    }

    #[test]
    fn screen_collector_orders_by_kind_and_depth_and_only_repeats_changes() {
        let mut collector = ScreenNarrationCollector::default();
        let mut frame = ScreenNarrationFrame::default();
        frame.add(NarratedElementKind::Usage, 0, "screen usage");
        frame.add(NarratedElementKind::Position, 2, "row 2");
        frame.add(NarratedElementKind::Title, 1, "row title");
        frame.add(NarratedElementKind::Title, 0, "screen title");
        assert_eq!(
            collector.update_and_collect(&frame, false).as_deref(),
            Some("screen title. row title. row 2. screen usage")
        );
        assert_eq!(collector.update_and_collect(&frame, false), None);

        let mut changed = frame.clone();
        changed.add(NarratedElementKind::Title, 1, "replacement title");
        assert_eq!(
            collector.update_and_collect(&changed, false).as_deref(),
            Some("replacement title")
        );
        assert_eq!(
            collector.update_and_collect(&changed, true).as_deref(),
            Some("screen title. replacement title. row 2. screen usage")
        );

        let mut absent = ScreenNarrationFrame::default();
        absent.add(NarratedElementKind::Title, 0, "screen title");
        assert_eq!(collector.update_and_collect(&absent, false), None);
        assert_eq!(
            collector.update_and_collect(&changed, false).as_deref(),
            Some("replacement title. row 2. screen usage")
        );
    }

    #[test]
    fn screen_runtime_applies_init_suppression_and_vanilla_input_delays() {
        let start = Instant::now();
        let mut runtime = ScreenNarrationRuntime::default();
        let mut narrator = RecordingNarrator::default();
        let mut first = ScreenNarrationFrame::default();
        first.add(NarratedElementKind::Title, 0, "Stats");
        runtime.tick(
            Some(&first),
            start,
            LastInputType::Mouse,
            Some(&mut narrator),
        );
        assert_eq!(narrator.0, ["Stats"]);

        let mut changed = first.clone();
        changed.add(NarratedElementKind::Title, 1, "General tab");
        runtime.after_input(
            ScreenNarrationInput::MouseMove,
            start + Duration::from_millis(100),
        );
        runtime.tick(
            Some(&changed),
            start + Duration::from_millis(900),
            LastInputType::Mouse,
            Some(&mut narrator),
        );
        assert_eq!(narrator.0, ["Stats"]);
        runtime.tick(
            Some(&changed),
            start + Duration::from_millis(2_001),
            LastInputType::Mouse,
            Some(&mut narrator),
        );
        assert_eq!(narrator.0, ["Stats", "General tab"]);

        let mut keyboard_changed = changed.clone();
        keyboard_changed.add(
            NarratedElementKind::Position,
            0,
            "Screen element 2 out of 3",
        );
        runtime.after_input(
            ScreenNarrationInput::KeyboardAction,
            start + Duration::from_millis(2_100),
        );
        runtime.tick(
            Some(&keyboard_changed),
            start + Duration::from_millis(2_300),
            LastInputType::Mouse,
            Some(&mut narrator),
        );
        assert_eq!(narrator.0.len(), 2);
        runtime.tick(
            Some(&keyboard_changed),
            start + Duration::from_millis(2_301),
            LastInputType::Mouse,
            Some(&mut narrator),
        );
        assert_eq!(
            narrator.0,
            ["Stats", "General tab", "Screen element 2 out of 3"]
        );

        let mut immediate_changed = keyboard_changed.clone();
        immediate_changed.add(NarratedElementKind::Hint, 0, "Filtered");
        runtime.after_input(
            ScreenNarrationInput::Immediate,
            start + Duration::from_millis(2_400),
        );
        runtime.tick(
            Some(&immediate_changed),
            start + Duration::from_millis(2_401),
            LastInputType::Mouse,
            Some(&mut narrator),
        );
        assert_eq!(narrator.0.last().map(String::as_str), Some("Filtered"));

        runtime.tick(
            None,
            start + Duration::from_secs(3),
            LastInputType::Mouse,
            Some(&mut narrator),
        );
        runtime.tick(
            Some(&immediate_changed),
            start + Duration::from_secs(4),
            LastInputType::Mouse,
            Some(&mut narrator),
        );
        assert_eq!(
            narrator.0.last().map(String::as_str),
            Some("Stats. General tab. Screen element 2 out of 3. Filtered")
        );
    }

    #[test]
    fn a_keyboard_opened_screen_suppresses_narration_until_the_next_action() {
        let start = Instant::now();
        let mut runtime = ScreenNarrationRuntime::default();
        let mut narrator = RecordingNarrator::default();
        let mut frame = ScreenNarrationFrame::default();
        frame.add(NarratedElementKind::Title, 0, "Stats");
        runtime.tick(
            Some(&frame),
            start,
            LastInputType::KeyboardTab,
            Some(&mut narrator),
        );
        assert_eq!(narrator.0, ["Stats"]);

        let mut changed = frame.clone();
        changed.add(NarratedElementKind::Title, 1, "General tab");
        runtime.after_input(
            ScreenNarrationInput::MouseMove,
            start + Duration::from_millis(100),
        );
        runtime.tick(
            Some(&changed),
            start + Duration::from_secs(60),
            LastInputType::KeyboardTab,
            Some(&mut narrator),
        );
        assert_eq!(narrator.0, ["Stats"]);

        runtime.after_input(
            ScreenNarrationInput::KeyboardAction,
            start + Duration::from_secs(61),
        );
        runtime.tick(
            Some(&changed),
            start + Duration::from_millis(61_201),
            LastInputType::KeyboardTab,
            Some(&mut narrator),
        );
        assert_eq!(narrator.0, ["Stats", "General tab"]);
    }
}
