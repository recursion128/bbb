use std::path::PathBuf;

use bbb_net::DisconnectReason;
use bbb_renderer::{HudDisconnectedScreenButton, HudDisconnectedScreenHitTarget};
use winit::{
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
};

use super::ClientInputState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DisconnectedScreenTitle {
    ConnectFailed,
    ConnectionLost,
}

impl DisconnectedScreenTitle {
    pub(crate) const fn translation_key(self) -> &'static str {
        match self {
            Self::ConnectFailed => "connect.failed",
            Self::ConnectionLost => "disconnect.lost",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DisconnectedScreenDetails {
    pub(crate) reason: DisconnectReason,
    pub(crate) report_directory: Option<PathBuf>,
    pub(crate) bug_report_link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DisconnectedScreenRuntime {
    pub(crate) generation: u64,
    pub(crate) title: DisconnectedScreenTitle,
    pub(crate) details: DisconnectedScreenDetails,
    pub(crate) focus: HudDisconnectedScreenButton,
    pub(crate) hovered: Option<HudDisconnectedScreenButton>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DisconnectedScreenPlatformRequest {
    OpenUri(String),
    OpenPath(PathBuf),
    Exit,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DisconnectedScreenInputState {
    next_generation: u64,
    active: Option<DisconnectedScreenRuntime>,
    platform_requests: Vec<DisconnectedScreenPlatformRequest>,
}

impl ClientInputState {
    pub(crate) fn open_disconnected_screen(
        &mut self,
        title: DisconnectedScreenTitle,
        details: DisconnectedScreenDetails,
    ) -> u64 {
        let generation = self.disconnected_screen.next_generation.saturating_add(1);
        self.disconnected_screen.next_generation = generation;
        let focus = first_button(&details);
        self.disconnected_screen.active = Some(DisconnectedScreenRuntime {
            generation,
            title,
            details,
            focus,
            hovered: None,
        });
        self.debug_options_screen = None;
        self.debug_pause_screen = None;
        self.debug_game_mode_switcher = None;
        self.pause_screen_disconnect_requested = false;
        self.interrupt_chat_screen();
        self.chat_entry = None;
        self.sign_editor = None;
        self.sign_editor_ime_preedit = None;
        self.book_input = None;
        self.game_rules_screen = None;
        self.clear_pressed();
        generation
    }

    pub(crate) fn disconnected_screen_runtime(&self) -> Option<&DisconnectedScreenRuntime> {
        self.disconnected_screen.active.as_ref()
    }

    pub(crate) fn disconnected_screen_status(
        &self,
    ) -> Option<bbb_control::DisconnectedScreenStatus> {
        let active = self.disconnected_screen.active.as_ref()?;
        let mut reason_preview = String::with_capacity(256);
        for ch in active
            .details
            .reason
            .runs
            .iter()
            .flat_map(|run| run.text.chars())
        {
            if reason_preview.len().saturating_add(ch.len_utf8()) > 256 {
                break;
            }
            reason_preview.push(ch);
        }
        let reason_truncated = active.details.reason.truncated
            || active.details.reason.source_text_bytes > reason_preview.len();
        Some(bbb_control::DisconnectedScreenStatus {
            generation: active.generation,
            title_translation_key: active.title.translation_key().to_string(),
            reason_preview,
            reason_retained_bytes: active.details.reason.retained_bytes,
            reason_truncated,
            report_to_server_available: active.details.bug_report_link.is_some(),
            report_directory_available: active.details.report_directory.is_some(),
        })
    }

    pub(crate) fn handle_disconnected_screen_key(
        &mut self,
        code: KeyCode,
        state: ElementState,
    ) -> bool {
        let reverse = self.shift_down();
        let Some(active) = self.disconnected_screen.active.as_mut() else {
            return false;
        };
        if state != ElementState::Pressed {
            return true;
        }
        let buttons = screen_buttons(&active.details);
        let activate = match code {
            KeyCode::Escape => None,
            KeyCode::Tab if reverse => {
                active.focus = adjacent_button(&buttons, active.focus, false);
                None
            }
            KeyCode::Tab | KeyCode::ArrowRight | KeyCode::ArrowDown => {
                active.focus = adjacent_button(&buttons, active.focus, true);
                None
            }
            KeyCode::ArrowLeft | KeyCode::ArrowUp => {
                active.focus = adjacent_button(&buttons, active.focus, false);
                None
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => Some(active.focus),
            _ => None,
        };
        if let Some(button) = activate {
            self.activate_disconnected_screen_button(button);
        }
        true
    }

    pub(crate) fn handle_disconnected_screen_hover(
        &mut self,
        target: Option<HudDisconnectedScreenHitTarget>,
    ) -> bool {
        let Some(active) = self.disconnected_screen.active.as_mut() else {
            return false;
        };
        active.hovered = target
            .filter(|target| target.generation == active.generation)
            .map(|target| target.button)
            .filter(|button| screen_buttons(&active.details).contains(button));
        true
    }

    pub(crate) fn handle_disconnected_screen_mouse_input(
        &mut self,
        button: MouseButton,
        state: ElementState,
        target: Option<HudDisconnectedScreenHitTarget>,
    ) -> bool {
        let Some(active) = self.disconnected_screen.active.as_ref() else {
            return false;
        };
        let activate = (button == MouseButton::Left && state == ElementState::Pressed)
            .then_some(target)
            .flatten()
            .filter(|target| target.generation == active.generation)
            .map(|target| target.button)
            .filter(|button| screen_buttons(&active.details).contains(button));
        if let Some(button) = activate {
            self.activate_disconnected_screen_button(button);
        }
        true
    }

    pub(crate) fn take_disconnected_screen_platform_requests(
        &mut self,
    ) -> Vec<DisconnectedScreenPlatformRequest> {
        std::mem::take(&mut self.disconnected_screen.platform_requests)
    }

    fn activate_disconnected_screen_button(&mut self, button: HudDisconnectedScreenButton) {
        let Some(active) = self.disconnected_screen.active.as_ref() else {
            return;
        };
        let request = match button {
            HudDisconnectedScreenButton::ReportToServer => active
                .details
                .bug_report_link
                .as_ref()
                .map(|url| DisconnectedScreenPlatformRequest::OpenUri(url.clone())),
            HudDisconnectedScreenButton::OpenReportDirectory => active
                .details
                .report_directory
                .as_ref()
                .map(|path| DisconnectedScreenPlatformRequest::OpenPath(path.clone())),
            HudDisconnectedScreenButton::Back => Some(DisconnectedScreenPlatformRequest::Exit),
        };
        if let Some(request) = request {
            self.disconnected_screen.platform_requests.push(request);
        }
    }
}

fn screen_buttons(details: &DisconnectedScreenDetails) -> Vec<HudDisconnectedScreenButton> {
    let mut buttons = Vec::with_capacity(3);
    if details.bug_report_link.is_some() {
        buttons.push(HudDisconnectedScreenButton::ReportToServer);
    }
    if details.report_directory.is_some() {
        buttons.push(HudDisconnectedScreenButton::OpenReportDirectory);
    }
    buttons.push(HudDisconnectedScreenButton::Back);
    buttons
}

fn first_button(details: &DisconnectedScreenDetails) -> HudDisconnectedScreenButton {
    screen_buttons(details)
        .first()
        .copied()
        .unwrap_or(HudDisconnectedScreenButton::Back)
}

fn adjacent_button(
    buttons: &[HudDisconnectedScreenButton],
    current: HudDisconnectedScreenButton,
    forward: bool,
) -> HudDisconnectedScreenButton {
    let current = buttons
        .iter()
        .position(|button| *button == current)
        .unwrap_or(0);
    let next = if forward {
        (current + 1) % buttons.len()
    } else {
        current.checked_sub(1).unwrap_or(buttons.len() - 1)
    };
    buttons[next]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn details() -> DisconnectedScreenDetails {
        DisconnectedScreenDetails {
            reason: DisconnectReason::from_plain("bye"),
            report_directory: None,
            bug_report_link: None,
        }
    }

    #[test]
    fn escape_is_consumed_and_stale_hit_target_cannot_activate_back() {
        let mut input = ClientInputState::default();
        let generation =
            input.open_disconnected_screen(DisconnectedScreenTitle::ConnectionLost, details());
        assert!(input.handle_disconnected_screen_key(KeyCode::Escape, ElementState::Pressed));
        assert!(input
            .take_disconnected_screen_platform_requests()
            .is_empty());
        assert!(input.handle_disconnected_screen_mouse_input(
            MouseButton::Left,
            ElementState::Pressed,
            Some(HudDisconnectedScreenHitTarget {
                generation: generation.saturating_sub(1),
                button: HudDisconnectedScreenButton::Back,
            }),
        ));
        assert!(input
            .take_disconnected_screen_platform_requests()
            .is_empty());
        input.handle_disconnected_screen_key(KeyCode::Enter, ElementState::Pressed);
        assert_eq!(
            input.take_disconnected_screen_platform_requests(),
            vec![DisconnectedScreenPlatformRequest::Exit]
        );
    }

    #[test]
    fn optional_buttons_preserve_vanilla_order_and_platform_ownership() {
        let mut input = ClientInputState::default();
        input.open_disconnected_screen(
            DisconnectedScreenTitle::ConnectFailed,
            DisconnectedScreenDetails {
                reason: DisconnectReason::from_plain("nope"),
                report_directory: Some(PathBuf::from("reports")),
                bug_report_link: Some("https://example.invalid/bug".to_string()),
            },
        );
        assert_eq!(
            input.disconnected_screen_runtime().unwrap().focus,
            HudDisconnectedScreenButton::ReportToServer
        );
        input.handle_disconnected_screen_key(KeyCode::Enter, ElementState::Pressed);
        input.handle_disconnected_screen_key(KeyCode::ArrowDown, ElementState::Pressed);
        input.handle_disconnected_screen_key(KeyCode::Enter, ElementState::Pressed);
        assert_eq!(
            input.take_disconnected_screen_platform_requests(),
            vec![
                DisconnectedScreenPlatformRequest::OpenUri(
                    "https://example.invalid/bug".to_string()
                ),
                DisconnectedScreenPlatformRequest::OpenPath(PathBuf::from("reports")),
            ]
        );
    }
}
