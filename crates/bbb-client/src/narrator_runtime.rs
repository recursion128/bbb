use bbb_client_clock::Instant;
pub(crate) mod command_confirmation;

use std::{collections::BTreeMap, time::Duration};
#[cfg(any(not(target_arch = "wasm32"), test))]
use std::{
    collections::VecDeque,
    ffi::OsString,
    io,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
};

use crate::input::LastInputType;

const SCREEN_NARRATION_SUPPRESS_AFTER_INIT: Duration = Duration::from_secs(2);
const SCREEN_NARRATION_MOUSE_MOVE_DELAY: Duration = Duration::from_millis(750);
const SCREEN_NARRATION_ACTION_DELAY: Duration = Duration::from_millis(200);
#[cfg(any(not(target_arch = "wasm32"), test))]
const MAX_PENDING_DESKTOP_SPEECH_MESSAGES: usize = 32;
pub(crate) const NARRATOR_OPTION_TRANSLATION_KEY: &str = "options.narrator";

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
    pub(crate) fn translation_key(self) -> &'static str {
        match self {
            Self::Off => "options.narrator.off",
            Self::All => "options.narrator.all",
            Self::Chat => "options.narrator.chat",
            Self::System => "options.narrator.system",
        }
    }

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

    pub(crate) fn should_narrate_chat(self) -> bool {
        matches!(self, Self::All | Self::Chat)
    }

    pub(crate) fn should_narrate_system(self) -> bool {
        matches!(self, Self::All | Self::System)
    }

    pub(crate) fn should_narrate_system_or_chat(self) -> bool {
        !matches!(self, Self::Off)
    }
}

struct NarratorStatusLabels {
    narrator: String,
    off: String,
    all: String,
    chat: String,
    system: String,
}

impl Default for NarratorStatusLabels {
    fn default() -> Self {
        Self {
            narrator: "Narrator".to_string(),
            off: NarratorStatus::Off.option_name().to_string(),
            all: NarratorStatus::All.option_name().to_string(),
            chat: NarratorStatus::Chat.option_name().to_string(),
            system: NarratorStatus::System.option_name().to_string(),
        }
    }
}

impl NarratorStatusLabels {
    fn status_label(&self, status: NarratorStatus) -> &str {
        match status {
            NarratorStatus::Off => &self.off,
            NarratorStatus::All => &self.all,
            NarratorStatus::Chat => &self.chat,
            NarratorStatus::System => &self.system,
        }
    }
}

fn narrator_status_labels() -> &'static std::sync::RwLock<NarratorStatusLabels> {
    static LABELS: std::sync::OnceLock<std::sync::RwLock<NarratorStatusLabels>> =
        std::sync::OnceLock::new();
    LABELS.get_or_init(|| std::sync::RwLock::new(NarratorStatusLabels::default()))
}

fn localized_or_default<'a>(
    key: &'a str,
    default: &str,
    localize: &impl Fn(&'a str) -> &'a str,
) -> String {
    let translated = localize(key);
    if translated.is_empty() || translated == key {
        default.to_string()
    } else {
        translated.to_string()
    }
}

/// Refreshes the cached localized narrator toggle labels from the active
/// resource-pack language catalog (Vanilla `GameNarrator.java:81`).
pub(crate) fn update_localized_status_labels<'a>(localize: impl Fn(&'a str) -> &'a str) {
    let updated = NarratorStatusLabels {
        narrator: localized_or_default(NARRATOR_OPTION_TRANSLATION_KEY, "Narrator", &localize),
        off: localized_or_default(
            NarratorStatus::Off.translation_key(),
            NarratorStatus::Off.option_name(),
            &localize,
        ),
        all: localized_or_default(
            NarratorStatus::All.translation_key(),
            NarratorStatus::All.option_name(),
            &localize,
        ),
        chat: localized_or_default(
            NarratorStatus::Chat.translation_key(),
            NarratorStatus::Chat.option_name(),
            &localize,
        ),
        system: localized_or_default(
            NarratorStatus::System.translation_key(),
            NarratorStatus::System.option_name(),
            &localize,
        ),
    };
    let mut guard = narrator_status_labels()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *guard = updated;
}

/// Formats Vanilla `GameNarrator.updateNarratorStatus`:
/// `Component.translatable("options.narrator").append(" : ").append(status.getName())`.
pub(crate) fn localized_status_announcement(status: NarratorStatus) -> String {
    let guard = narrator_status_labels()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    format!("{} : {}", guard.narrator, guard.status_label(status))
}

#[cfg(test)]
fn localized_status_announcement_with<'a>(
    status: NarratorStatus,
    localize: impl Fn(&'a str) -> &'a str,
) -> String {
    let narrator = localized_or_default(NARRATOR_OPTION_TRANSLATION_KEY, "Narrator", &localize);
    let mode = localized_or_default(status.translation_key(), status.option_name(), &localize);
    format!("{narrator} : {mode}")
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
    kind: bbb_client_world::ChatNarrationKind,
    message: &str,
) {
    match kind {
        bbb_client_world::ChatNarrationKind::Chat => narrator.say_chat_queued(message),
        bbb_client_world::ChatNarrationKind::SystemChat => narrator.say_system_chat_queued(message),
        bbb_client_world::ChatNarrationKind::System => narrator.say_system_queued(message),
        bbb_client_world::ChatNarrationKind::SystemNow => narrator.say_system_now(message),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScreenNarrationTarget {
    DebugOptions,
    SocialInteractions,
    Stats,
    RecipeBook,
    GameEvent,
    Disconnected,
    Chat,
    Death,
    Pause,
    Inventory,
    Book,
    Sign,
    CommandBlock,
    StructureBlock,
    JigsawBlock,
    Dialog,
    CommandConfirmation,
}

const ORDINARY_SCREEN_NARRATION_COUNT: usize = 16;

impl ScreenNarrationTarget {
    fn ordinary_index(self) -> Option<usize> {
        match self {
            Self::DebugOptions => Some(0),
            Self::SocialInteractions => Some(1),
            Self::Stats => Some(2),
            Self::RecipeBook => Some(3),
            Self::GameEvent => Some(4),
            Self::Disconnected => Some(5),
            Self::Chat => Some(6),
            Self::Death => Some(7),
            Self::Pause => Some(8),
            Self::Inventory => Some(9),
            Self::Book => Some(10),
            Self::Sign => Some(11),
            Self::CommandBlock => Some(12),
            Self::StructureBlock => Some(13),
            Self::JigsawBlock => Some(14),
            Self::Dialog => Some(15),
            Self::CommandConfirmation => None,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct ScreenNarrationFrames {
    ordinary: [Option<ScreenNarrationFrame>; ORDINARY_SCREEN_NARRATION_COUNT],
    pub(crate) confirmation: Option<command_confirmation::CommandConfirmationNarrationFrame>,
}

impl ScreenNarrationFrames {
    pub(crate) fn set(
        &mut self,
        target: ScreenNarrationTarget,
        frame: Option<ScreenNarrationFrame>,
    ) {
        if let Some(index) = target.ordinary_index() {
            self.ordinary[index] = frame;
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct ScreenNarrations {
    ordinary: [ScreenNarrationRuntime; ORDINARY_SCREEN_NARRATION_COUNT],
    confirmation: command_confirmation::CommandConfirmationNarrationRuntime,
}

impl ScreenNarrations {
    pub(crate) fn after_input(
        &mut self,
        target: ScreenNarrationTarget,
        input: ScreenNarrationInput,
        now: Instant,
    ) {
        match target.ordinary_index() {
            Some(index) => self.ordinary[index].after_input(input, now),
            None => self.confirmation.timing.after_input(input, now),
        }
    }

    pub(crate) fn schedule_after_narrator_enabled(&mut self, now: Instant) {
        for runtime in &mut self.ordinary {
            runtime.schedule_after_narrator_enabled(now);
        }
        self.confirmation
            .timing
            .schedule_after_narrator_enabled(now);
    }

    pub(crate) fn initialize_confirmation(
        &mut self,
        frame: &command_confirmation::CommandConfirmationNarrationFrame,
        now: Instant,
        last_input_type: LastInputType,
        narrator: Option<&mut dyn SystemNarrator>,
    ) {
        self.confirmation
            .initialize(frame, now, last_input_type, narrator);
    }

    pub(crate) fn tick(
        &mut self,
        frames: &ScreenNarrationFrames,
        now: Instant,
        last_input_type: LastInputType,
        mut narrator: Option<&mut dyn SystemNarrator>,
    ) {
        self.confirmation.tick(
            frames.confirmation.as_ref(),
            now,
            last_input_type,
            narrator
                .as_mut()
                .map(|narrator| &mut **narrator as &mut dyn SystemNarrator),
        );
        for (runtime, frame) in self.ordinary.iter_mut().zip(&frames.ordinary) {
            runtime.tick(
                frame.as_ref(),
                now,
                last_input_type,
                narrator
                    .as_mut()
                    .map(|narrator| &mut **narrator as &mut dyn SystemNarrator),
            );
        }
    }
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
        self.tick_lazy(
            frame.map(|frame| || frame.clone()),
            now,
            last_input_type,
            narrator,
        );
    }

    fn tick_lazy(
        &mut self,
        frame: Option<impl FnOnce() -> ScreenNarrationFrame>,
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
            if let Some(narrator) = narrator {
                if let Some(text) = self.collector.update_and_collect(&frame(), true) {
                    narrator.say_system_now(&text);
                }
            }
            return;
        }
        let Some(narrate_at) = self.narrate_at else {
            return;
        };
        if now <= narrate_at || self.suppression.blocks(now) {
            return;
        }
        let Some(narrator) = narrator else {
            return;
        };
        self.narrate_at = None;
        if let Some(text) = self.collector.update_and_collect(&frame(), false) {
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

#[cfg(any(not(target_arch = "wasm32"), test))]
const MAX_OS_TTS_MESSAGE_BYTES: usize = 16_384;
#[cfg(any(not(target_arch = "wasm32"), test))]
const SPEECH_WORKER_POLL_INTERVAL: Duration = Duration::from_millis(20);
#[cfg(any(not(target_arch = "wasm32"), test))]
const WINDOWS_POWERSHELL_SPEAK_SCRIPT: &str = "Add-Type -AssemblyName System.Speech; \
$s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
$s.Volume = [Math]::Max(0, [Math]::Min(100, [int]$env:BBB_NARRATOR_VOLUME)); \
if ($env:BBB_NARRATOR_TEXT) { $s.Speak($env:BBB_NARRATOR_TEXT) }";

#[cfg(any(not(target_arch = "wasm32"), test))]
#[derive(Debug, Clone, PartialEq)]
enum SpeechBackendSpec {
    Command {
        executable: PathBuf,
        arguments: Vec<String>,
        volume: f32,
    },
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    WindowsPowerShell { executable: PathBuf, volume: f32 },
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    MacOsSay { executable: PathBuf, volume: f32 },
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    LinuxSpdSay { executable: PathBuf, volume: f32 },
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    LinuxEspeakNg { executable: PathBuf, volume: f32 },
}

#[cfg(any(not(target_arch = "wasm32"), test))]
impl SpeechBackendSpec {
    fn executable(&self) -> &Path {
        match self {
            Self::Command { executable, .. }
            | Self::WindowsPowerShell { executable, .. }
            | Self::MacOsSay { executable, .. }
            | Self::LinuxSpdSay { executable, .. }
            | Self::LinuxEspeakNg { executable, .. } => executable.as_path(),
        }
    }

    fn build_command(&self, message: &str) -> Option<Command> {
        match self {
            Self::Command {
                executable,
                arguments,
                volume,
            } => {
                let args = narrator_command_arguments(arguments, message, *volume);
                let mut command = Command::new(executable);
                command
                    .args(args)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                Some(command)
            }
            Self::WindowsPowerShell { executable, volume } => {
                let clamped = clamp_unit_volume(*volume);
                if clamped <= 0.0 {
                    return None;
                }
                let volume_percent = (clamped * 100.0).round() as u32;
                let sanitized = sanitize_os_tts_message(message);
                if sanitized.is_empty() {
                    return None;
                }
                let mut command = Command::new(executable);
                command
                    .args([
                        "-NoProfile",
                        "-NonInteractive",
                        "-WindowStyle",
                        "Hidden",
                        "-Command",
                        WINDOWS_POWERSHELL_SPEAK_SCRIPT,
                    ])
                    .env("BBB_NARRATOR_TEXT", sanitized)
                    .env("BBB_NARRATOR_VOLUME", volume_percent.to_string())
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                #[cfg(target_os = "windows")]
                {
                    use std::os::windows::process::CommandExt;
                    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                    command.creation_flags(CREATE_NO_WINDOW);
                }
                Some(command)
            }
            Self::MacOsSay { executable, volume } => {
                if clamp_unit_volume(*volume) <= 0.0 {
                    return None;
                }
                let sanitized = sanitize_os_tts_message(message);
                if sanitized.is_empty() {
                    return None;
                }
                let mut command = Command::new(executable);
                command
                    .arg("--")
                    .arg(sanitized)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                Some(command)
            }
            Self::LinuxSpdSay { executable, volume } => {
                let clamped = clamp_unit_volume(*volume);
                if clamped <= 0.0 {
                    return None;
                }
                let sanitized = sanitize_os_tts_message(message);
                if sanitized.is_empty() {
                    return None;
                }
                let spd_volume = (clamped * 200.0 - 100.0).round() as i32;
                let mut command = Command::new(executable);
                command
                    .args(["-w", "-i", &spd_volume.to_string(), "--"])
                    .arg(sanitized)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                Some(command)
            }
            Self::LinuxEspeakNg { executable, volume } => {
                let clamped = clamp_unit_volume(*volume);
                if clamped <= 0.0 {
                    return None;
                }
                let sanitized = sanitize_os_tts_message(message);
                if sanitized.is_empty() {
                    return None;
                }
                let amplitude = (clamped * 100.0).round() as u32;
                let mut command = Command::new(executable);
                command
                    .args(["-a", &amplitude.to_string(), "--"])
                    .arg(sanitized)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                Some(command)
            }
        }
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn clamp_unit_volume(volume: f32) -> f32 {
    if volume.is_finite() {
        volume.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn sanitize_os_tts_message(message: &str) -> String {
    let mut sanitized = String::with_capacity(message.len().min(MAX_OS_TTS_MESSAGE_BYTES));
    for ch in message.chars() {
        if ch == '\0' {
            continue;
        }
        if sanitized.len().saturating_add(ch.len_utf8()) > MAX_OS_TTS_MESSAGE_BYTES {
            break;
        }
        sanitized.push(ch);
    }
    sanitized
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn find_executable_in_paths(
    search_dirs: impl IntoIterator<Item = PathBuf>,
    name: &str,
    extra_candidates: &[PathBuf],
) -> Option<PathBuf> {
    for dir in search_dirs {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    extra_candidates
        .iter()
        .find(|candidate| candidate.is_file())
        .cloned()
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn find_executable_on_path(name: &str, extra_candidates: &[PathBuf]) -> Option<PathBuf> {
    let path_dirs = std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .unwrap_or_default();
    find_executable_in_paths(path_dirs, name, extra_candidates)
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn platform_default_speech_backends(volume: f32) -> Vec<SpeechBackendSpec> {
    #[cfg(target_os = "windows")]
    {
        let mut extra = Vec::new();
        if let Some(root) = std::env::var_os("SystemRoot") {
            extra.push(
                PathBuf::from(root)
                    .join("System32")
                    .join("WindowsPowerShell")
                    .join("v1.0")
                    .join("powershell.exe"),
            );
        }
        find_executable_on_path("powershell.exe", &extra)
            .map(|executable| vec![SpeechBackendSpec::WindowsPowerShell { executable, volume }])
            .unwrap_or_default()
    }
    #[cfg(target_os = "macos")]
    {
        let extra = [PathBuf::from("/usr/bin/say")];
        find_executable_on_path("say", &extra)
            .map(|executable| vec![SpeechBackendSpec::MacOsSay { executable, volume }])
            .unwrap_or_default()
    }
    #[cfg(target_os = "linux")]
    {
        let mut backends = Vec::new();
        if let Some(executable) =
            find_executable_on_path("spd-say", &[PathBuf::from("/usr/bin/spd-say")])
        {
            backends.push(SpeechBackendSpec::LinuxSpdSay { executable, volume });
        }
        if let Some(executable) =
            find_executable_on_path("espeak-ng", &[PathBuf::from("/usr/bin/espeak-ng")])
        {
            backends.push(SpeechBackendSpec::LinuxEspeakNg { executable, volume });
        }
        backends
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = volume;
        Vec::new()
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
#[derive(Default)]
struct SpeechWorkerShared {
    pending: VecDeque<String>,
    generation: u64,
    shutdown: bool,
    active_child_pid: Option<u32>,
    unavailable_warned: bool,
    backend_disabled: bool,
}

#[cfg(any(not(target_arch = "wasm32"), test))]
enum SpawnSpeechOutcome {
    Spawned(Child),
    Skipped,
    Unavailable(io::Error, PathBuf),
    Failed(io::Error, PathBuf),
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn spawn_speech_message(
    backends: &[SpeechBackendSpec],
    backend_index: &mut usize,
    message: &str,
) -> SpawnSpeechOutcome {
    while let Some(spec) = backends.get(*backend_index) {
        let Some(mut command) = spec.build_command(message) else {
            return SpawnSpeechOutcome::Skipped;
        };
        match command.spawn() {
            Ok(child) => return SpawnSpeechOutcome::Spawned(child),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                if *backend_index + 1 < backends.len() {
                    *backend_index += 1;
                    continue;
                }
                return SpawnSpeechOutcome::Unavailable(err, spec.executable().to_path_buf());
            }
            Err(err) => {
                return SpawnSpeechOutcome::Failed(err, spec.executable().to_path_buf());
            }
        }
    }
    SpawnSpeechOutcome::Unavailable(
        io::Error::new(io::ErrorKind::NotFound, "no narrator backend configured"),
        PathBuf::new(),
    )
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn kill_and_reap_child(child: &mut Child) {
    if let Err(err) = child.kill() {
        tracing::debug!(?err, "narrator process already stopped before clear");
    }
    if let Err(err) = child.wait() {
        tracing::warn!(?err, "failed to reap cleared narrator process");
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn lock_shared(
    shared: &(Mutex<SpeechWorkerShared>, Condvar),
) -> std::sync::MutexGuard<'_, SpeechWorkerShared> {
    match shared.0.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn run_speech_worker(
    backends: Vec<SpeechBackendSpec>,
    shared: Arc<(Mutex<SpeechWorkerShared>, Condvar)>,
) {
    let mut active_child: Option<(Child, u64)> = None;
    let mut backend_index = 0usize;

    loop {
        let (should_shutdown, interrupted, next_message) = {
            let mut state = lock_shared(&shared);
            loop {
                if state.shutdown {
                    break (true, true, None);
                }
                if let Some((_, child_gen)) = active_child.as_ref() {
                    if state.generation != *child_gen {
                        break (false, true, None);
                    }
                    let (guard, _) = match shared.1.wait_timeout(state, SPEECH_WORKER_POLL_INTERVAL)
                    {
                        Ok(result) => result,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    state = guard;
                    let was_interrupted = state.generation != *child_gen;
                    break (state.shutdown, was_interrupted, None);
                }
                if state.backend_disabled {
                    state.pending.clear();
                    state = match shared.1.wait(state) {
                        Ok(guard) => guard,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    continue;
                }
                if let Some(message) = state.pending.pop_front() {
                    break (false, false, Some((message, state.generation)));
                }
                state = match shared.1.wait(state) {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
            }
        };

        if should_shutdown || interrupted {
            if let Some((mut child, _)) = active_child.take() {
                kill_and_reap_child(&mut child);
                let mut state = lock_shared(&shared);
                state.active_child_pid = None;
                shared.1.notify_all();
            }
            if should_shutdown {
                break;
            }
            continue;
        }

        if let Some((child, _)) = active_child.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        tracing::warn!(?status, "narrator process exited unsuccessfully");
                    }
                    active_child = None;
                    let mut state = lock_shared(&shared);
                    state.active_child_pid = None;
                    shared.1.notify_all();
                }
                Ok(None) => {}
                Err(err) => {
                    tracing::warn!(?err, "failed to poll narrator process");
                    active_child = None;
                    let mut state = lock_shared(&shared);
                    state.active_child_pid = None;
                    shared.1.notify_all();
                }
            }
            continue;
        }

        if let Some((message, msg_gen)) = next_message {
            match spawn_speech_message(&backends, &mut backend_index, &message) {
                SpawnSpeechOutcome::Spawned(child) => {
                    let pid = child.id();
                    {
                        let mut state = lock_shared(&shared);
                        state.active_child_pid = Some(pid);
                        shared.1.notify_all();
                    }
                    active_child = Some((child, msg_gen));
                }
                SpawnSpeechOutcome::Skipped => {}
                SpawnSpeechOutcome::Unavailable(err, executable) => {
                    let mut state = lock_shared(&shared);
                    if !state.unavailable_warned {
                        tracing::warn!(
                            ?err,
                            executable = %executable.display(),
                            "desktop TTS narrator binary unavailable; disabling TTS output for this session"
                        );
                        state.unavailable_warned = true;
                    }
                    state.backend_disabled = true;
                    state.pending.clear();
                    shared.1.notify_all();
                }
                SpawnSpeechOutcome::Failed(err, executable) => {
                    tracing::warn!(
                        ?err,
                        executable = %executable.display(),
                        "failed to start narrator process"
                    );
                    let mut state = lock_shared(&shared);
                    state.pending.clear();
                    shared.1.notify_all();
                }
            }
        }
    }
}

/// A process-backed TTS adapter executing on a dedicated background worker thread
/// so speech startup, polling, and interruption never block the main event loop.
/// Explicit `--narrator-command` executables replace exact `{text}` and `{volume}`
/// placeholders (`GameNarrator.narrateMessage` passes `SoundSource.VOICE` final
/// volume); when no explicit command is configured, built-in OS TTS backends are
/// selected per desktop platform.
#[cfg(any(not(target_arch = "wasm32"), test))]
struct CommandTextToSpeech {
    backends: Vec<SpeechBackendSpec>,
    shared: Arc<(Mutex<SpeechWorkerShared>, Condvar)>,
    worker: Option<JoinHandle<()>>,
}

#[cfg(any(not(target_arch = "wasm32"), test))]
impl CommandTextToSpeech {
    fn new(executable: PathBuf, arguments: Vec<String>, volume: f32) -> Self {
        Self::from_backends(vec![SpeechBackendSpec::Command {
            executable,
            arguments,
            volume,
        }])
    }

    fn from_backends(backends: Vec<SpeechBackendSpec>) -> Self {
        Self {
            backends,
            shared: Arc::new((Mutex::new(SpeechWorkerShared::default()), Condvar::new())),
            worker: None,
        }
    }

    fn ensure_worker_started(&mut self) {
        if self.worker.is_some() {
            return;
        }
        let backends = self.backends.clone();
        let shared = Arc::clone(&self.shared);
        match thread::Builder::new()
            .name("bbb-narrator".to_string())
            .spawn(move || run_speech_worker(backends, shared))
        {
            Ok(handle) => self.worker = Some(handle),
            Err(err) => {
                let mut state = lock_shared(&self.shared);
                if !state.unavailable_warned {
                    tracing::warn!(
                        ?err,
                        "failed to spawn narrator worker thread; disabling TTS output"
                    );
                    state.unavailable_warned = true;
                }
                state.backend_disabled = true;
                state.pending.clear();
            }
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
        {
            let mut state = lock_shared(&self.shared);
            if state.backend_disabled {
                return;
            }
            if interrupt {
                state.pending.clear();
                state.generation = state.generation.wrapping_add(1);
            } else if state.pending.len() >= MAX_PENDING_DESKTOP_SPEECH_MESSAGES {
                state.pending.pop_front();
            }
            state.pending.push_back(message.to_string());
            self.shared.1.notify_all();
        }
        self.ensure_worker_started();
    }

    fn clear(&mut self) {
        let mut state = lock_shared(&self.shared);
        state.pending.clear();
        state.generation = state.generation.wrapping_add(1);
        self.shared.1.notify_all();
    }

    fn tick(&mut self) {
        // The dedicated background worker handles process polling and reaping.
    }

    #[cfg(test)]
    fn active_child_pid(&self) -> Option<u32> {
        lock_shared(&self.shared).active_child_pid
    }

    #[cfg(test)]
    fn pending_snapshot(&self) -> Vec<String> {
        lock_shared(&self.shared).pending.iter().cloned().collect()
    }

    #[cfg(test)]
    fn is_disabled_and_warned(&self) -> (bool, bool) {
        let state = lock_shared(&self.shared);
        (state.backend_disabled, state.unavailable_warned)
    }

    #[cfg(all(test, unix))]
    fn wait_for_active_child_pid(&self, timeout: Duration) -> Option<u32> {
        let deadline = std::time::Instant::now() + timeout;
        let mut state = lock_shared(&self.shared);
        while state.active_child_pid.is_none() && !state.backend_disabled {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            let (guard, _) = match self.shared.1.wait_timeout(state, remaining) {
                Ok(result) => result,
                Err(poisoned) => poisoned.into_inner(),
            };
            state = guard;
        }
        state.active_child_pid
    }

    #[cfg(test)]
    fn wait_until_disabled(&self, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        let mut state = lock_shared(&self.shared);
        while !state.backend_disabled {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            let (guard, _) = match self.shared.1.wait_timeout(state, remaining) {
                Ok(result) => result,
                Err(poisoned) => poisoned.into_inner(),
            };
            state = guard;
        }
        state.backend_disabled
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
impl Drop for CommandTextToSpeech {
    fn drop(&mut self) {
        {
            let mut state = lock_shared(&self.shared);
            state.pending.clear();
            state.generation = state.generation.wrapping_add(1);
            state.shutdown = true;
            self.shared.1.notify_all();
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
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
/// Non-interrupting chat/system queues share the non-blocking background worker queue.
#[cfg(any(not(target_arch = "wasm32"), test))]
pub(crate) struct NativeNarratorRuntime {
    status: NarratorStatus,
    speech: CommandTextToSpeech,
}

#[cfg(any(not(target_arch = "wasm32"), test))]
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

    /// Selects a built-in desktop OS TTS backend when `--narrator-command` is
    /// omitted (Windows PowerShell `System.Speech`, macOS `say`, Linux
    /// `spd-say` / `espeak-ng`). Returns `None` if no OS TTS binary is installed.
    pub(crate) fn with_platform_default(status: NarratorStatus, volume: f32) -> Option<Self> {
        let backends = platform_default_speech_backends(volume);
        (!backends.is_empty()).then(|| Self {
            status,
            speech: CommandTextToSpeech::from_backends(backends),
        })
    }

    /// Resolves the desktop narrator runtime: explicit `--narrator-command`
    /// always takes priority over the built-in OS TTS backend.
    pub(crate) fn resolve(
        status: NarratorStatus,
        command: Option<PathBuf>,
        arguments: Vec<String>,
        volume: f32,
    ) -> Option<Self> {
        match command {
            Some(executable) => Some(Self::new(status, executable, arguments, volume)),
            None => Self::with_platform_default(status, volume),
        }
    }

    pub(crate) fn tick(&mut self) {
        self.speech.tick();
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
impl SystemNarrator for NativeNarratorRuntime {
    fn update_status(&mut self, status: NarratorStatus) {
        if self.status != NarratorStatus::Off {
            self.speech.clear();
        }
        self.status = status;
        self.speech
            .say(&localized_status_announcement(status), true);
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
    fn screen_narrations_preserve_order_across_closing_confirmation_and_enable_actions() {
        let start = Instant::now();
        let keyboard = LastInputType::KeyboardTab;
        let mut runtime = ScreenNarrations::default();
        let mut speech = RecordingNarrator::default();
        let frame = |text| {
            let mut frame = ScreenNarrationFrame::default();
            frame.add(NarratedElementKind::Title, 0, text);
            frame
        };
        let confirmation =
            |presentation, text| command_confirmation::CommandConfirmationNarrationFrame {
                generation: 1,
                presentation,
                retained: vec![1],
                focused: None,
                hovered: None,
                unfocused_frame: frame(text),
                focused_frames: [frame(text), frame(text)],
                hovered_frames: [frame(text), frame(text)],
            };
        let mut frames = ScreenNarrationFrames::default();
        frames.set(ScreenNarrationTarget::DebugOptions, Some(frame("Debug")));
        runtime.tick(&frames, start, keyboard, Some(&mut speech));
        frames = ScreenNarrationFrames::default();
        runtime.tick(&frames, start, keyboard, Some(&mut speech));
        frames.set(ScreenNarrationTarget::Stats, Some(frame("Statistics")));
        runtime.tick(&frames, start, keyboard, Some(&mut speech));
        let parent = confirmation(1, "Parent");
        runtime.initialize_confirmation(&parent, start, keyboard, Some(&mut speech));
        frames = ScreenNarrationFrames::default();
        runtime.tick(&frames, start, keyboard, Some(&mut speech));
        runtime.initialize_confirmation(
            &confirmation(2, "Child"),
            start,
            keyboard,
            Some(&mut speech),
        );
        let restored = confirmation(3, "Parent");
        runtime.initialize_confirmation(&restored, start, keyboard, Some(&mut speech));
        frames.confirmation = Some(restored);
        runtime.tick(&frames, start, keyboard, Some(&mut speech));
        frames.confirmation = Some(confirmation(3, "Changed parent"));
        runtime.schedule_after_narrator_enabled(start);
        runtime.after_input(
            ScreenNarrationTarget::CommandConfirmation,
            ScreenNarrationInput::KeyboardAction,
            start,
        );
        runtime.tick(
            &frames,
            start + Duration::from_millis(201),
            keyboard,
            Some(&mut speech),
        );
        frames = ScreenNarrationFrames::default();
        frames.set(ScreenNarrationTarget::DebugOptions, Some(frame("Debug")));
        runtime.tick(
            &frames,
            start + Duration::from_secs(1),
            keyboard,
            Some(&mut speech),
        );
        assert_eq!(
            speech.0,
            [
                "Debug",
                "Statistics",
                "Parent",
                "Child",
                "Parent",
                "Changed parent",
                "Debug"
            ]
        );
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
            bbb_client_world::ChatNarrationKind::Chat,
            bbb_client_world::ChatNarrationKind::SystemChat,
            bbb_client_world::ChatNarrationKind::System,
            bbb_client_world::ChatNarrationKind::SystemNow,
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
    #[cfg(target_os = "linux")]
    fn dropping_the_narrator_runtime_kills_and_reaps_the_speaking_child() {
        let mut speech = CommandTextToSpeech::new(
            PathBuf::from("/bin/sh"),
            vec!["-c".to_string(), "sleep 10".to_string()],
            1.0,
        );
        speech.say("spoken", true);
        let id = speech
            .wait_for_active_child_pid(Duration::from_secs(2))
            .expect("child spawned");
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
    #[cfg(unix)]
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
        assert!(off.speech.pending_snapshot().is_empty());
        assert!(off.speech.active_child_pid().is_none());

        let mut chat = NativeNarratorRuntime::new(
            NarratorStatus::Chat,
            executable.clone(),
            arguments.clone(),
            1.0,
        );
        chat.say_system_now("ignored");
        assert!(chat.speech.pending_snapshot().is_empty());
        assert!(chat.speech.active_child_pid().is_none());

        let mut all = NativeNarratorRuntime::new(
            NarratorStatus::All,
            executable.clone(),
            arguments.clone(),
            1.0,
        );
        all.say_system_now("spoken");
        assert!(all
            .speech
            .wait_for_active_child_pid(Duration::from_secs(2))
            .is_some());
        assert!(all.speech.pending_snapshot().is_empty());
        all.speech.clear();

        let mut system =
            NativeNarratorRuntime::new(NarratorStatus::System, executable, arguments, 1.0);
        system.say_system_now("spoken");
        assert!(system
            .speech
            .wait_for_active_child_pid(Duration::from_secs(2))
            .is_some());
        assert!(system.speech.pending_snapshot().is_empty());
    }

    #[test]
    #[cfg(unix)]
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
        assert!(chat
            .speech
            .wait_for_active_child_pid(Duration::from_secs(2))
            .is_some());
        chat.say_system_chat_queued("system chat two");
        assert_eq!(chat.speech.pending_snapshot(), ["system chat two"]);
        chat.say_system_queued("ignored system");
        assert_eq!(chat.speech.pending_snapshot(), ["system chat two"]);
        chat.speech.clear();

        let mut system =
            NativeNarratorRuntime::new(NarratorStatus::System, executable, arguments, 1.0);
        system.say_chat_queued("ignored chat");
        assert!(system.speech.active_child_pid().is_none());
        system.say_system_chat_queued("system chat");
        assert!(system
            .speech
            .wait_for_active_child_pid(Duration::from_secs(2))
            .is_some());
        system.say_system_queued("system queued");
        assert_eq!(system.speech.pending_snapshot(), ["system queued"]);
        system.speech.clear();
    }

    #[test]
    fn explicit_narrator_command_takes_priority_over_platform_default() {
        let custom = PathBuf::from("/custom/tts-bin");
        let runtime = NativeNarratorRuntime::resolve(
            NarratorStatus::All,
            Some(custom.clone()),
            vec!["--say".to_string(), "{text}".to_string()],
            0.5,
        )
        .expect("explicit command must always produce a narrator runtime");
        assert_eq!(
            runtime.speech.backends,
            vec![SpeechBackendSpec::Command {
                executable: custom,
                arguments: vec!["--say".to_string(), "{text}".to_string()],
                volume: 0.5,
            }]
        );
    }

    #[test]
    fn missing_tts_binary_degrades_cleanly_with_single_warning_without_blocking() {
        let mut runtime = NativeNarratorRuntime::new(
            NarratorStatus::All,
            PathBuf::from("bbb-nonexistent-tts-binary-for-test"),
            Vec::new(),
            1.0,
        );
        runtime.say_system_now("first utterance");
        assert!(
            runtime.speech.wait_until_disabled(Duration::from_secs(2)),
            "missing binary must disable backend cleanly on worker thread"
        );
        assert_eq!(runtime.speech.is_disabled_and_warned(), (true, true));
        // Subsequent utterances must be ignored immediately without re-queuing.
        runtime.say_system_now("second utterance");
        runtime.say_chat_queued("third utterance");
        assert!(runtime.speech.pending_snapshot().is_empty());
    }

    #[test]
    fn os_tts_specs_format_commands_and_volumes_safely() {
        let ps = SpeechBackendSpec::WindowsPowerShell {
            executable: PathBuf::from("powershell.exe"),
            volume: 0.6,
        };
        let cmd = ps
            .build_command("Hello; $(calc.exe)\0")
            .expect("positive volume builds command");
        let envs: BTreeMap<OsString, Option<OsString>> = cmd
            .get_envs()
            .map(|(k, v)| (k.to_os_string(), v.map(OsString::from)))
            .collect();
        assert_eq!(
            envs.get(&OsString::from("BBB_NARRATOR_TEXT")).cloned(),
            Some(Some(OsString::from("Hello; $(calc.exe)")))
        );
        assert_eq!(
            envs.get(&OsString::from("BBB_NARRATOR_VOLUME")).cloned(),
            Some(Some(OsString::from("60")))
        );
        assert!(SpeechBackendSpec::WindowsPowerShell {
            executable: PathBuf::from("powershell.exe"),
            volume: 0.0,
        }
        .build_command("muted")
        .is_none());

        let say = SpeechBackendSpec::MacOsSay {
            executable: PathBuf::from("/usr/bin/say"),
            volume: 1.0,
        };
        let say_args: Vec<OsString> = say
            .build_command("-v leading-dash")
            .unwrap()
            .get_args()
            .map(OsString::from)
            .collect();
        assert_eq!(
            say_args,
            [OsString::from("--"), OsString::from("-v leading-dash")]
        );

        let spd = SpeechBackendSpec::LinuxSpdSay {
            executable: PathBuf::from("/usr/bin/spd-say"),
            volume: 0.5,
        };
        let spd_args: Vec<OsString> = spd
            .build_command("hello")
            .unwrap()
            .get_args()
            .map(OsString::from)
            .collect();
        assert_eq!(
            spd_args,
            [
                OsString::from("-w"),
                OsString::from("-i"),
                OsString::from("0"),
                OsString::from("--"),
                OsString::from("hello"),
            ]
        );

        let espeak = SpeechBackendSpec::LinuxEspeakNg {
            executable: PathBuf::from("/usr/bin/espeak-ng"),
            volume: 0.75,
        };
        let espeak_args: Vec<OsString> = espeak
            .build_command("hello")
            .unwrap()
            .get_args()
            .map(OsString::from)
            .collect();
        assert_eq!(
            espeak_args,
            [
                OsString::from("-a"),
                OsString::from("75"),
                OsString::from("--"),
                OsString::from("hello"),
            ]
        );
    }

    #[test]
    fn expanded_screen_narration_targets_tick_independently() {
        let start = Instant::now();
        let keyboard = LastInputType::KeyboardTab;
        let mut runtime = ScreenNarrations::default();
        let mut speech = RecordingNarrator::default();
        let make_frame = |text: &str| {
            let mut frame = ScreenNarrationFrame::default();
            frame.add(NarratedElementKind::Title, 0, text);
            frame
        };
        let targets = [
            (ScreenNarrationTarget::Pause, "Game Menu"),
            (ScreenNarrationTarget::Inventory, "Inventory"),
            (ScreenNarrationTarget::Book, "Book"),
            (ScreenNarrationTarget::Sign, "Edit Sign Message"),
            (
                ScreenNarrationTarget::CommandBlock,
                "Set Console Command for Block",
            ),
            (ScreenNarrationTarget::StructureBlock, "Structure Block"),
            (ScreenNarrationTarget::JigsawBlock, "Jigsaw Block"),
            (ScreenNarrationTarget::Dialog, "Server Dialog"),
        ];
        for (index, (target, label)) in targets.iter().enumerate() {
            let mut frames = ScreenNarrationFrames::default();
            frames.set(*target, Some(make_frame(label)));
            runtime.tick(
                &frames,
                start + Duration::from_secs(index as u64),
                keyboard,
                Some(&mut speech),
            );
            let mut updated = ScreenNarrationFrames::default();
            let mut next = make_frame(label);
            next.add(NarratedElementKind::Title, 1, format!("{label} action"));
            updated.set(*target, Some(next));
            runtime.after_input(
                *target,
                ScreenNarrationInput::KeyboardAction,
                start + Duration::from_secs(index as u64),
            );
            runtime.tick(
                &updated,
                start + Duration::from_secs(index as u64) + Duration::from_millis(201),
                keyboard,
                Some(&mut speech),
            );
        }
        assert_eq!(speech.0.len(), targets.len() * 2);
        assert_eq!(speech.0[0], "Game Menu");
        assert_eq!(speech.0[1], "Game Menu action");
        assert_eq!(speech.0[14], "Server Dialog");
        assert_eq!(speech.0[15], "Server Dialog action");
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
    fn screen_runtime_without_backend_preserves_collector_and_pending_narration() {
        let start = Instant::now();
        let mut runtime = ScreenNarrationRuntime::default();
        let mut frame = ScreenNarrationFrame::default();
        frame.add(NarratedElementKind::Title, 0, "Confirm command");
        runtime.tick_lazy(
            Some(|| panic!("no backend must not select widgets")),
            start,
            LastInputType::Mouse,
            None,
        );
        assert_eq!(runtime.collector.generation, 0);
        runtime.after_input(ScreenNarrationInput::KeyboardAction, start);
        runtime.tick_lazy(
            Some(|| panic!("no backend must not select widgets")),
            start + Duration::from_secs(1),
            LastInputType::Mouse,
            None,
        );
        assert_eq!(runtime.collector.generation, 0);
        assert!(runtime.narrate_at.is_some());
        runtime.schedule_after_narrator_enabled(start + Duration::from_secs(1));
        let mut narrator = RecordingNarrator::default();
        runtime.tick(
            Some(&frame),
            start + Duration::from_secs(3),
            LastInputType::Mouse,
            Some(&mut narrator),
        );
        assert!(narrator.0.is_empty());
        runtime.tick(
            Some(&frame),
            start + Duration::from_millis(3001),
            LastInputType::Mouse,
            Some(&mut narrator),
        );
        assert_eq!(narrator.0, ["Confirm command"]);
    }

    #[test]
    fn screen_runtime_off_mode_with_backend_still_collects_without_speaking() {
        let start = Instant::now();
        let mut runtime = ScreenNarrationRuntime::default();
        let mut narrator = NativeNarratorRuntime::new(
            NarratorStatus::Off,
            PathBuf::from("/bin/false"),
            Vec::new(),
            1.0,
        );
        let mut frame = ScreenNarrationFrame::default();
        frame.add(NarratedElementKind::Title, 0, "Confirm command");
        runtime.tick(
            Some(&frame),
            start,
            LastInputType::Mouse,
            Some(&mut narrator),
        );
        assert_eq!(runtime.collector.generation, 1);
        assert!(narrator.speech.active_child_pid().is_none());
        assert!(narrator.speech.pending_snapshot().is_empty());
        runtime.schedule_after_narrator_enabled(start);
        let mut enabled = RecordingNarrator::default();
        runtime.tick(
            Some(&frame),
            start + Duration::from_millis(2001),
            LastInputType::Mouse,
            Some(&mut enabled),
        );
        assert!(enabled.0.is_empty());
        frame.add(NarratedElementKind::Title, 1, "Run command button");
        runtime.after_input(
            ScreenNarrationInput::KeyboardAction,
            start + Duration::from_secs(3),
        );
        runtime.tick(
            Some(&frame),
            start + Duration::from_millis(3201),
            LastInputType::Mouse,
            Some(&mut enabled),
        );
        assert_eq!(enabled.0, ["Run command button"]);
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

    #[test]
    fn narrator_status_announcement_uses_localized_keys_with_english_default() {
        let localized = localized_status_announcement_with(NarratorStatus::Chat, |key| match key {
            "options.narrator" => "Sprecher",
            "options.narrator.chat" => "Chat vorlesen",
            other => other,
        });
        assert_eq!(localized, "Sprecher : Chat vorlesen");

        let default_english = localized_status_announcement_with(NarratorStatus::System, |key| key);
        assert_eq!(default_english, "Narrator : Narrates System");
    }

    #[test]
    fn desktop_tts_pending_queue_is_bounded_by_cap() {
        // Hold a disabled-free shared queue without spawning a worker thread by
        // pushing directly through the same bounded enqueue contract via `say`
        // while keeping `worker` occupied with a dummy join handle.
        let mut tts = CommandTextToSpeech::from_backends(Vec::new());
        // Spawn a dormant dummy handle so `ensure_worker_started` does not spawn
        // the real worker thread and drain `state.pending` while we inspect it.
        tts.worker = Some(thread::spawn(|| {}));
        for index in 0..(MAX_PENDING_DESKTOP_SPEECH_MESSAGES + 5) {
            tts.say(&format!("msg-{index}"), false);
        }
        let pending = tts.pending_snapshot();
        assert_eq!(pending.len(), MAX_PENDING_DESKTOP_SPEECH_MESSAGES);
        assert_eq!(pending.first().map(String::as_str), Some("msg-5"));
        assert_eq!(
            pending.last().map(String::as_str),
            Some(format!("msg-{}", MAX_PENDING_DESKTOP_SPEECH_MESSAGES + 4).as_str())
        );
    }
}
