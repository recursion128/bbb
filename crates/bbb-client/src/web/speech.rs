//! Browser Web Speech API (`window.speechSynthesis`) narrator backend for `wasm32`.

use web_sys::{SpeechSynthesis, SpeechSynthesisUtterance, Window};

use crate::narrator_runtime::{localized_status_announcement, NarratorStatus, SystemNarrator};

pub(super) struct WebSpeechNarrator {
    synthesis: SpeechSynthesis,
    status: NarratorStatus,
    volume: f32,
}

impl WebSpeechNarrator {
    pub(super) fn try_new(window: &Window, status: NarratorStatus, volume: f32) -> Option<Self> {
        let synthesis = window.speech_synthesis().ok()?;
        Some(Self {
            synthesis,
            status,
            volume: if volume.is_finite() {
                volume.clamp(0.0, 1.0)
            } else {
                1.0
            },
        })
    }

    fn speak(&self, message: &str, interrupt: bool) {
        if message.trim().is_empty() {
            return;
        }
        if interrupt {
            self.synthesis.cancel();
        }
        if self.volume <= 0.0 {
            return;
        }
        let Ok(utterance) = SpeechSynthesisUtterance::new_with_text(message) else {
            return;
        };
        utterance.set_volume(self.volume);
        self.synthesis.speak(&utterance);
    }
}

impl SystemNarrator for WebSpeechNarrator {
    fn update_status(&mut self, status: NarratorStatus) {
        if self.status != NarratorStatus::Off {
            self.synthesis.cancel();
        }
        self.status = status;
        self.speak(&localized_status_announcement(status), true);
    }

    fn say_system_now(&mut self, message: &str) {
        if self.status.should_narrate_system() {
            self.speak(message, true);
        }
    }

    fn say_chat_queued(&mut self, message: &str) {
        if self.status.should_narrate_chat() {
            self.speak(message, false);
        }
    }

    fn say_system_chat_queued(&mut self, message: &str) {
        if self.status.should_narrate_system_or_chat() {
            self.speak(message, false);
        }
    }

    fn say_system_queued(&mut self, message: &str) {
        if self.status.should_narrate_system() {
            self.speak(message, false);
        }
    }
}
