use std::{
    collections::VecDeque,
    ffi::OsString,
    path::PathBuf,
    process::{Child, Command, Stdio},
};

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
    fn should_narrate_system(self) -> bool {
        matches!(self, Self::All | Self::System)
    }
}

/// Narrow runtime boundary used by UI and session consumers that match vanilla
/// `GameNarrator.saySystemNow` behavior.
pub(crate) trait SystemNarrator {
    fn say_system_now(&mut self, message: &str);
}

/// A process-backed TTS adapter. The executable and arguments come only from startup CLI
/// configuration. Every exact `{text}` argument is replaced with the narration; when no
/// placeholder is present, the narration is appended as the final argument.
struct CommandTextToSpeech {
    executable: PathBuf,
    arguments: Vec<String>,
    pending: VecDeque<String>,
    child: Option<Child>,
}

impl CommandTextToSpeech {
    fn new(executable: PathBuf, arguments: Vec<String>) -> Self {
        Self {
            executable,
            arguments,
            pending: VecDeque::new(),
            child: None,
        }
    }

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
        let arguments = narrator_command_arguments(&self.arguments, &message);
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

fn narrator_command_arguments(arguments: &[String], message: &str) -> Vec<OsString> {
    let mut replaced = false;
    let mut result = arguments
        .iter()
        .map(|argument| {
            if argument == "{text}" {
                replaced = true;
                OsString::from(message)
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
    pub(crate) fn new(status: NarratorStatus, executable: PathBuf, arguments: Vec<String>) -> Self {
        Self {
            status,
            speech: CommandTextToSpeech::new(executable, arguments),
        }
    }

    pub(crate) fn tick(&mut self) {
        self.speech.tick();
    }
}

impl SystemNarrator for NativeNarratorRuntime {
    fn say_system_now(&mut self, message: &str) {
        if self.status.should_narrate_system() {
            self.speech.say(message, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_arguments_replace_exact_text_placeholder_or_append_message() {
        assert_eq!(
            narrator_command_arguments(&["--voice".to_string(), "{text}".to_string()], "hello"),
            [OsString::from("--voice"), OsString::from("hello")]
        );
        assert_eq!(
            narrator_command_arguments(&["--voice={text}".to_string()], "hello"),
            [OsString::from("--voice={text}"), OsString::from("hello")]
        );
    }

    #[test]
    fn narrator_status_filters_interrupting_system_messages() {
        let executable = PathBuf::from("/bin/true");
        let mut off = NativeNarratorRuntime::new(NarratorStatus::Off, executable.clone(), vec![]);
        off.say_system_now("ignored");
        assert!(off.speech.pending.is_empty());
        assert!(off.speech.child.is_none());

        let mut chat = NativeNarratorRuntime::new(NarratorStatus::Chat, executable.clone(), vec![]);
        chat.say_system_now("ignored");
        assert!(chat.speech.pending.is_empty());
        assert!(chat.speech.child.is_none());

        let mut all = NativeNarratorRuntime::new(NarratorStatus::All, executable.clone(), vec![]);
        all.say_system_now("spoken");
        assert!(all.speech.pending.is_empty());
        assert!(all.speech.child.is_some());
        all.speech.clear();

        let mut system = NativeNarratorRuntime::new(NarratorStatus::System, executable, vec![]);
        system.say_system_now("spoken");
        assert!(system.speech.pending.is_empty());
        assert!(system.speech.child.is_some());
    }
}
