use std::{
    collections::BTreeSet,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use bbb_protocol::packets::decode_profile_textures_from_properties;
use bbb_renderer::{HudPlayerReportingFocus, HudPlayerReportingHitTarget};
use bbb_world::{
    player_report_reasons, BuiltPlayerReport, ChatVisibilityState, PlayerReportDraft,
    PlayerReportReason, PlayerReportType, SocialPlayerState, WorldStore,
    MAX_REPORT_COMMENT_UTF16_UNITS, MAX_SELECTED_REPORT_MESSAGES,
};
use uuid::Uuid;
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::KeyCode,
};

use super::{
    death_screen::DeathScreenMainInputState, pause_screen::DebugPauseScreenState,
    social::SocialInteractionsRuntime, text_edit, ClientInputState,
};

mod support;
#[cfg(test)]
mod tests;

use support::*;

const REPORT_ROW_HEIGHT: i32 = 24;
const REPORTING_HELP_URL: &str = "https://aka.ms/aboutjavareporting";
static NEXT_REPORT_NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlayerReportingTarget {
    pub(crate) profile_id: Uuid,
    pub(crate) name: String,
    pub(crate) chat_enabled: bool,
    pub(crate) skin_url: Option<String>,
}

impl PlayerReportingTarget {
    fn from_social_player(player: &SocialPlayerState) -> Self {
        let skin_url = decode_profile_textures_from_properties(
            player
                .profile
                .properties
                .iter()
                .map(|property| (property.name.as_str(), property.value.as_str())),
        )
        .and_then(|textures| textures.skin.map(|skin| skin.url));
        Self {
            profile_id: player.profile.uuid,
            name: player.profile.name.clone(),
            chat_enabled: !player.blocked && player.chat_reportable && player.recent_messages,
            skin_url,
        }
    }

    fn from_draft(draft: &PlayerReportDraft) -> Self {
        let profile_id = draft.common().reported_profile_id;
        Self {
            profile_id,
            name: match draft {
                PlayerReportDraft::Username { reported_name, .. } => reported_name.clone(),
                _ => profile_id.to_string(),
            },
            chat_enabled: draft.report_type() == PlayerReportType::Chat,
            skin_url: match draft {
                PlayerReportDraft::Skin { skin_url, .. } => skin_url.clone(),
                _ => None,
            },
        }
    }
}

#[derive(Debug, Clone)]
enum PlayerReportingPreviousInputState {
    SocialInteractions(SocialInteractionsRuntime),
    Death(DeathScreenMainInputState),
    Pause(DebugPauseScreenState),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlayerReportingRuntimeScreen {
    TypeChooser,
    Editor(PlayerReportType),
    ReasonSelection {
        editor_type: PlayerReportType,
        pending: Option<PlayerReportReason>,
    },
    ChatSelection {
        pending: BTreeSet<u64>,
    },
    DraftChoice,
    DiscardWarning(PlayerReportType),
    Sending {
        report_type: PlayerReportType,
        report_id: Uuid,
        attempt: u64,
    },
    Success,
    Failure,
}

#[derive(Debug, Clone)]
pub(crate) struct PlayerReportingRuntime {
    generation: u64,
    target: PlayerReportingTarget,
    requested_target: PlayerReportingTarget,
    previous_screen: Option<PlayerReportingPreviousInputState>,
    screen: PlayerReportingRuntimeScreen,
    focus: HudPlayerReportingFocus,
    hovered: Option<HudPlayerReportingFocus>,
    scroll_amount: f64,
    comments_cursor: usize,
    comments_selection: usize,
}

impl PlayerReportingRuntime {
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn target(&self) -> &PlayerReportingTarget {
        &self.target
    }

    pub(crate) fn screen(&self) -> &PlayerReportingRuntimeScreen {
        &self.screen
    }

    pub(crate) fn focus(&self) -> HudPlayerReportingFocus {
        self.focus
    }

    pub(crate) fn hovered(&self) -> Option<HudPlayerReportingFocus> {
        self.hovered
    }

    pub(crate) fn scroll_amount(&self) -> i32 {
        self.scroll_amount.max(0.0) as i32
    }

    pub(crate) fn comments_cursor(&self) -> usize {
        self.comments_cursor
    }

    pub(crate) fn comments_selection(&self) -> usize {
        self.comments_selection
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PlayerReportingInputState {
    active: Option<PlayerReportingRuntime>,
    pending_submissions: Vec<PlayerReportingSubmissionRequest>,
    cancelled_submissions: Vec<(Uuid, u64)>,
    platform_requests: Vec<PlayerReportingPlatformRequest>,
    next_submission_attempt: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlayerReportingPlatformRequest {
    OpenUri(String),
}

#[derive(Debug, Clone)]
pub(crate) struct PlayerReportingSubmissionRequest {
    pub(crate) report: BuiltPlayerReport,
    pub(crate) attempt: u64,
}

impl ClientInputState {
    /// Whether the report comment box would take a commit right now.
    pub(crate) fn player_reporting_comments_are_focused(&self) -> bool {
        self.player_reporting
            .active
            .as_ref()
            .is_some_and(|active| active.focus == HudPlayerReportingFocus::Comments)
    }

    pub(crate) fn player_reporting_runtime(&self) -> Option<&PlayerReportingRuntime> {
        self.player_reporting.active.as_ref()
    }

    pub(crate) fn open_player_reporting_screen(
        &mut self,
        world: &mut WorldStore,
        profile_id: Uuid,
    ) -> bool {
        if world.player_reporting().environment.is_none()
            || Some(profile_id) == world.local_player_profile_id()
        {
            return false;
        }
        let Some(player) = self
            .social_interactions_runtime()
            .and_then(|runtime| {
                runtime
                    .rows()
                    .iter()
                    .find(|player| player.profile.uuid == profile_id)
            })
            .cloned()
        else {
            return false;
        };
        let mut requested_target = PlayerReportingTarget::from_social_player(&player);
        requested_target.chat_enabled &=
            world.chat_presentation_config().visibility == ChatVisibilityState::Full;
        let initial_type_focus = if requested_target.chat_enabled {
            HudPlayerReportingFocus::ReportType(0)
        } else {
            HudPlayerReportingFocus::ReportType(1)
        };
        let previous_screen = self
            .take_social_interactions_for_dialog()
            .map(PlayerReportingPreviousInputState::SocialInteractions);
        let canonical = world.open_player_reporting_screen();
        let has_draft = world.player_reporting().draft.is_some();
        self.player_reporting.active = Some(PlayerReportingRuntime {
            generation: canonical.generation,
            target: requested_target.clone(),
            requested_target,
            previous_screen,
            screen: if has_draft {
                PlayerReportingRuntimeScreen::DraftChoice
            } else {
                PlayerReportingRuntimeScreen::TypeChooser
            },
            focus: if has_draft {
                HudPlayerReportingFocus::DraftEdit
            } else {
                initial_type_focus
            },
            hovered: None,
            scroll_amount: 0.0,
            comments_cursor: 0,
            comments_selection: 0,
        });
        true
    }

    pub(super) fn open_player_reporting_draft_from_death(
        &mut self,
        world: &mut WorldStore,
        parent: DeathScreenMainInputState,
    ) -> bool {
        if world.death_screen().map(|screen| screen.generation) != Some(parent.generation) {
            return false;
        }
        self.open_player_reporting_draft(world, PlayerReportingPreviousInputState::Death(parent))
    }

    /// Vanilla `ConfirmScreen` Continue Editing path of
    /// `draftReportHandled(quitToTitle = true)`: the draft report editor
    /// reopens over the pause screen that asked to disconnect.
    pub(super) fn open_player_reporting_draft_from_pause(
        &mut self,
        world: &mut WorldStore,
        parent: DebugPauseScreenState,
    ) -> bool {
        self.open_player_reporting_draft(world, PlayerReportingPreviousInputState::Pause(parent))
    }

    fn open_player_reporting_draft(
        &mut self,
        world: &mut WorldStore,
        previous_screen: PlayerReportingPreviousInputState,
    ) -> bool {
        if world.player_reporting().environment.is_none() {
            return false;
        }
        let Some(draft) = world.copy_player_report_draft() else {
            return false;
        };
        let report_type = draft.report_type();
        let target = PlayerReportingTarget::from_draft(&draft);
        let comments_len = text_edit::char_len(&draft.common().comments);
        let canonical = world.open_player_reporting_screen();
        self.player_reporting.active = Some(PlayerReportingRuntime {
            generation: canonical.generation,
            target: target.clone(),
            requested_target: target,
            previous_screen: Some(previous_screen),
            screen: PlayerReportingRuntimeScreen::Editor(report_type),
            focus: editor_initial_focus(report_type),
            hovered: None,
            scroll_amount: 0.0,
            comments_cursor: comments_len,
            comments_selection: comments_len,
        });
        true
    }

    pub(crate) fn sync_player_reporting_screen(&mut self, world: &WorldStore) -> bool {
        let generation = world
            .player_reporting_screen()
            .map(|screen| screen.generation);
        if self
            .player_reporting
            .active
            .as_ref()
            .is_some_and(|active| Some(active.generation) == generation)
        {
            return false;
        }
        self.player_reporting.active.take().is_some()
    }

    pub(crate) fn handle_player_reporting_key(
        &mut self,
        world: &mut WorldStore,
        code: KeyCode,
        just_pressed: bool,
    ) -> bool {
        self.sync_player_reporting_screen(world);
        if self.player_reporting.active.is_none() {
            return false;
        }
        if !just_pressed {
            return true;
        }
        let comments_focused = self
            .player_reporting
            .active
            .as_ref()
            .is_some_and(|active| active.focus == HudPlayerReportingFocus::Comments);
        let shift_down = self.shift_down();
        let control_down = self.control_down();
        if comments_focused
            && edit_comments_key(
                world,
                self.player_reporting
                    .active
                    .as_mut()
                    .expect("active report screen"),
                code,
                shift_down,
                control_down,
            )
        {
            return true;
        }
        match code {
            KeyCode::Escape => self.back_player_reporting_screen(world),
            KeyCode::Tab => {
                let forward = !self.shift_down();
                self.player_reporting
                    .active
                    .as_mut()
                    .expect("active report screen")
                    .move_focus(world, forward);
            }
            KeyCode::ArrowLeft | KeyCode::ArrowUp if !comments_focused => {
                self.player_reporting
                    .active
                    .as_mut()
                    .expect("active report screen")
                    .move_focus(world, false);
            }
            KeyCode::ArrowRight | KeyCode::ArrowDown if !comments_focused => {
                self.player_reporting
                    .active
                    .as_mut()
                    .expect("active report screen")
                    .move_focus(world, true);
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space if !comments_focused => {
                self.activate_player_reporting_focus(world);
            }
            _ => {}
        }
        true
    }

    pub(crate) fn handle_player_reporting_text(
        &mut self,
        world: &mut WorldStore,
        text: &str,
    ) -> bool {
        self.sync_player_reporting_screen(world);
        let Some(active) = self.player_reporting.active.as_mut() else {
            return false;
        };
        if active.focus != HudPlayerReportingFocus::Comments {
            return true;
        }
        let Some(draft) = world.copy_player_report_draft() else {
            return true;
        };
        let mut comments = draft.common().comments.clone();
        replace_selection(
            &mut comments,
            &mut active.comments_cursor,
            &mut active.comments_selection,
            text,
        );
        comments = truncate_utf16(comments, MAX_REPORT_COMMENT_UTF16_UNITS);
        let len = text_edit::char_len(&comments);
        active.comments_cursor = active.comments_cursor.min(len);
        active.comments_selection = active.comments_selection.min(len);
        world.set_player_report_comments(draft.common().report_id, comments);
        true
    }

    pub(crate) fn handle_player_reporting_hover(
        &mut self,
        world: &WorldStore,
        target: Option<HudPlayerReportingHitTarget>,
    ) -> bool {
        self.sync_player_reporting_screen(world);
        let Some(active) = self.player_reporting.active.as_mut() else {
            return false;
        };
        active.hovered = target
            .filter(|target| target.generation == active.generation)
            .map(|target| target.focus)
            .filter(|focus| active.focusables(world).contains(focus));
        true
    }

    pub(crate) fn handle_player_reporting_mouse_input(
        &mut self,
        world: &mut WorldStore,
        button: MouseButton,
        state: ElementState,
        target: Option<HudPlayerReportingHitTarget>,
    ) -> bool {
        self.sync_player_reporting_screen(world);
        let Some(active) = self.player_reporting.active.as_ref() else {
            return false;
        };
        if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            return true;
        }
        let Some(target) = target.filter(|target| {
            target.generation == active.generation
                && active.focusables(world).contains(&target.focus)
        }) else {
            return true;
        };
        self.player_reporting
            .active
            .as_mut()
            .expect("active report screen")
            .focus = target.focus;
        self.activate_player_reporting_focus(world);
        true
    }

    pub(crate) fn handle_player_reporting_mouse_wheel(
        &mut self,
        world: &WorldStore,
        delta: MouseScrollDelta,
    ) -> bool {
        self.sync_player_reporting_screen(world);
        let Some(active) = self.player_reporting.active.as_mut() else {
            return false;
        };
        let delta = match delta {
            MouseScrollDelta::LineDelta(_, y) => f64::from(y) * f64::from(REPORT_ROW_HEIGHT / 2),
            MouseScrollDelta::PixelDelta(position) => position.y,
        };
        active.scroll_amount = (active.scroll_amount - delta).max(0.0);
        true
    }

    pub(crate) fn take_player_reporting_submissions(
        &mut self,
    ) -> Vec<PlayerReportingSubmissionRequest> {
        std::mem::take(&mut self.player_reporting.pending_submissions)
    }

    pub(crate) fn take_cancelled_player_reporting_submissions(&mut self) -> Vec<(Uuid, u64)> {
        std::mem::take(&mut self.player_reporting.cancelled_submissions)
    }

    pub(crate) fn take_player_reporting_platform_requests(
        &mut self,
    ) -> Vec<PlayerReportingPlatformRequest> {
        std::mem::take(&mut self.player_reporting.platform_requests)
    }

    pub(crate) fn apply_player_reporting_submission_result(
        &mut self,
        world: &mut WorldStore,
        service_generation: u64,
        report_id: Uuid,
        attempt: u64,
        succeeded: bool,
    ) -> bool {
        let Some(generation) = self.player_reporting.active.as_ref().and_then(|active| {
            let PlayerReportingRuntimeScreen::Sending {
                report_type: _,
                report_id: active_report_id,
                attempt: active_attempt,
            } = active.screen
            else {
                return None;
            };
            (active_report_id == report_id && active_attempt == attempt)
                .then_some(active.generation)
        }) else {
            return false;
        };
        if !world.complete_player_report_submission(
            service_generation,
            report_id,
            attempt,
            succeeded,
        ) {
            return false;
        }
        if succeeded {
            world.remove_submitted_player_report_draft(service_generation, report_id, attempt);
        }
        let Some(canonical) = world.transition_player_reporting_screen(generation) else {
            self.player_reporting.active = None;
            return false;
        };
        let active = self
            .player_reporting
            .active
            .as_mut()
            .expect("matching report runtime exists");
        active.generation = canonical.generation;
        active.screen = if succeeded {
            PlayerReportingRuntimeScreen::Success
        } else {
            PlayerReportingRuntimeScreen::Failure
        };
        active.focus = if succeeded {
            HudPlayerReportingFocus::Done
        } else {
            HudPlayerReportingFocus::Retry
        };
        active.hovered = None;
        active.scroll_amount = 0.0;
        true
    }

    fn activate_player_reporting_focus(&mut self, world: &mut WorldStore) {
        let Some((screen, focus)) = self
            .player_reporting
            .active
            .as_ref()
            .map(|active| (active.screen.clone(), active.focus))
        else {
            return;
        };
        match screen {
            PlayerReportingRuntimeScreen::TypeChooser => match focus {
                HudPlayerReportingFocus::ReportType(index) => {
                    let report_type = match index {
                        0 => PlayerReportType::Chat,
                        1 => PlayerReportType::Skin,
                        2 => PlayerReportType::Username,
                        _ => return,
                    };
                    if report_type == PlayerReportType::Chat
                        && self
                            .player_reporting
                            .active
                            .as_ref()
                            .is_none_or(|active| !active.target.chat_enabled)
                    {
                        return;
                    }
                    if self.start_player_reporting_draft(world, report_type) {
                        self.transition_player_reporting_screen(
                            world,
                            PlayerReportingRuntimeScreen::Editor(report_type),
                            editor_initial_focus(report_type),
                        );
                    }
                }
                HudPlayerReportingFocus::Cancel => self.close_player_reporting_screen(world),
                _ => {}
            },
            PlayerReportingRuntimeScreen::Editor(report_type) => match focus {
                HudPlayerReportingFocus::SelectMessages
                    if report_type == PlayerReportType::Chat =>
                {
                    let selected = selected_chat_messages(world);
                    self.transition_player_reporting_screen(
                        world,
                        PlayerReportingRuntimeScreen::ChatSelection { pending: selected },
                        reportable_message_ids(world, self.current_report_target())
                            .into_iter()
                            .next()
                            .map(HudPlayerReportingFocus::Message)
                            .unwrap_or(HudPlayerReportingFocus::Back),
                    );
                    if let Some(active) = self.player_reporting.active.as_mut() {
                        active.scroll_amount = f64::from(i32::MAX);
                    }
                }
                HudPlayerReportingFocus::SelectReason
                    if report_type != PlayerReportType::Username =>
                {
                    let pending = selected_reason(world);
                    self.transition_player_reporting_screen(
                        world,
                        PlayerReportingRuntimeScreen::ReasonSelection {
                            editor_type: report_type,
                            pending,
                        },
                        pending
                            .and_then(|selected| {
                                player_report_reasons(report_type)
                                    .iter()
                                    .position(|reason| *reason == selected)
                            })
                            .and_then(|index| u8::try_from(index).ok())
                            .map(HudPlayerReportingFocus::Reason)
                            .unwrap_or(HudPlayerReportingFocus::Reason(0)),
                    );
                }
                HudPlayerReportingFocus::Comments => {}
                HudPlayerReportingFocus::Attestation => {
                    if let Some(draft) = world.copy_player_report_draft() {
                        world.set_player_report_attested(
                            draft.common().report_id,
                            !draft.common().attested,
                        );
                    }
                }
                HudPlayerReportingFocus::Back => self.back_player_reporting_screen(world),
                HudPlayerReportingFocus::Send => {
                    let Ok(report) = world.build_player_report() else {
                        return;
                    };
                    let report_id = report.id;
                    self.player_reporting.next_submission_attempt = self
                        .player_reporting
                        .next_submission_attempt
                        .checked_add(1)
                        .expect("player-report submission attempt exhausted");
                    let attempt = self.player_reporting.next_submission_attempt;
                    self.player_reporting
                        .pending_submissions
                        .push(PlayerReportingSubmissionRequest { report, attempt });
                    self.transition_player_reporting_screen(
                        world,
                        PlayerReportingRuntimeScreen::Sending {
                            report_type,
                            report_id,
                            attempt,
                        },
                        HudPlayerReportingFocus::Cancel,
                    );
                }
                _ => {}
            },
            PlayerReportingRuntimeScreen::ReasonSelection {
                editor_type,
                mut pending,
            } => match focus {
                HudPlayerReportingFocus::Reason(index) => {
                    pending = player_report_reasons(editor_type)
                        .get(usize::from(index))
                        .copied();
                    if let Some(active) = self.player_reporting.active.as_mut() {
                        active.screen = PlayerReportingRuntimeScreen::ReasonSelection {
                            editor_type,
                            pending,
                        };
                    }
                }
                HudPlayerReportingFocus::Done => {
                    if let (Some(reason), Some(draft)) = (pending, world.copy_player_report_draft())
                    {
                        world.set_player_report_reason(draft.common().report_id, reason);
                    }
                    self.transition_player_reporting_screen(
                        world,
                        PlayerReportingRuntimeScreen::Editor(editor_type),
                        HudPlayerReportingFocus::SelectReason,
                    );
                }
                HudPlayerReportingFocus::ReadInfo => {
                    self.player_reporting.platform_requests.push(
                        PlayerReportingPlatformRequest::OpenUri(REPORTING_HELP_URL.to_string()),
                    );
                }
                HudPlayerReportingFocus::Back => {
                    self.transition_player_reporting_screen(
                        world,
                        PlayerReportingRuntimeScreen::Editor(editor_type),
                        HudPlayerReportingFocus::SelectReason,
                    );
                }
                _ => {}
            },
            PlayerReportingRuntimeScreen::ChatSelection { mut pending } => match focus {
                HudPlayerReportingFocus::Message(message_id) => {
                    if !pending.remove(&message_id)
                        && pending.len() < MAX_SELECTED_REPORT_MESSAGES
                        && reportable_message_ids(world, self.current_report_target())
                            .contains(&message_id)
                    {
                        pending.insert(message_id);
                    }
                    if let Some(active) = self.player_reporting.active.as_mut() {
                        active.screen = PlayerReportingRuntimeScreen::ChatSelection { pending };
                    }
                }
                HudPlayerReportingFocus::Done => {
                    if let Some(draft) = world.copy_player_report_draft() {
                        world.set_chat_report_messages(draft.common().report_id, pending);
                    }
                    self.transition_player_reporting_screen(
                        world,
                        PlayerReportingRuntimeScreen::Editor(PlayerReportType::Chat),
                        HudPlayerReportingFocus::SelectMessages,
                    );
                }
                HudPlayerReportingFocus::Back => {
                    self.transition_player_reporting_screen(
                        world,
                        PlayerReportingRuntimeScreen::Editor(PlayerReportType::Chat),
                        HudPlayerReportingFocus::SelectMessages,
                    );
                }
                _ => {}
            },
            PlayerReportingRuntimeScreen::DraftChoice => match focus {
                HudPlayerReportingFocus::DraftEdit => self.resume_player_reporting_draft(world),
                HudPlayerReportingFocus::DraftDiscard => {
                    if let Some(draft) = world.copy_player_report_draft() {
                        world.discard_player_report_draft(draft.common().report_id);
                    }
                    if let Some(active) = self.player_reporting.active.as_mut() {
                        active.target = active.requested_target.clone();
                    }
                    self.transition_player_reporting_screen(
                        world,
                        PlayerReportingRuntimeScreen::TypeChooser,
                        if self
                            .player_reporting
                            .active
                            .as_ref()
                            .is_some_and(|active| active.target.chat_enabled)
                        {
                            HudPlayerReportingFocus::ReportType(0)
                        } else {
                            HudPlayerReportingFocus::ReportType(1)
                        },
                    );
                }
                HudPlayerReportingFocus::Cancel => self.close_player_reporting_screen(world),
                _ => {}
            },
            PlayerReportingRuntimeScreen::DiscardWarning(report_type) => match focus {
                HudPlayerReportingFocus::WarningReturn => {
                    self.transition_player_reporting_screen(
                        world,
                        PlayerReportingRuntimeScreen::Editor(report_type),
                        HudPlayerReportingFocus::Back,
                    );
                }
                HudPlayerReportingFocus::WarningSaveDraft => {
                    self.close_player_reporting_screen(world);
                }
                HudPlayerReportingFocus::WarningDiscard => {
                    if let Some(draft) = world.copy_player_report_draft() {
                        world.discard_player_report_draft(draft.common().report_id);
                    }
                    self.close_player_reporting_screen(world);
                }
                _ => {}
            },
            PlayerReportingRuntimeScreen::Sending {
                report_type,
                report_id,
                attempt,
            } => {
                if focus == HudPlayerReportingFocus::Cancel {
                    self.cancel_player_report_submission(world, report_type, report_id, attempt);
                }
            }
            PlayerReportingRuntimeScreen::Success => {
                if focus == HudPlayerReportingFocus::Done {
                    self.finish_player_reporting_screen(world);
                }
            }
            PlayerReportingRuntimeScreen::Failure => match focus {
                HudPlayerReportingFocus::Retry => {
                    let Ok(report) = world.build_player_report() else {
                        return;
                    };
                    let report_type = report.report_type;
                    let report_id = report.id;
                    self.player_reporting.next_submission_attempt = self
                        .player_reporting
                        .next_submission_attempt
                        .checked_add(1)
                        .expect("player-report submission attempt exhausted");
                    let attempt = self.player_reporting.next_submission_attempt;
                    self.player_reporting
                        .pending_submissions
                        .push(PlayerReportingSubmissionRequest { report, attempt });
                    self.transition_player_reporting_screen(
                        world,
                        PlayerReportingRuntimeScreen::Sending {
                            report_type,
                            report_id,
                            attempt,
                        },
                        HudPlayerReportingFocus::Cancel,
                    );
                }
                HudPlayerReportingFocus::Back => {
                    self.return_failed_player_report_submission_to_editor(world);
                }
                _ => {}
            },
        }
    }

    fn start_player_reporting_draft(
        &mut self,
        world: &mut WorldStore,
        report_type: PlayerReportType,
    ) -> bool {
        let Some(target) = self
            .player_reporting
            .active
            .as_ref()
            .map(|active| active.target.clone())
        else {
            return false;
        };
        let report_id = next_report_id();
        let created_at = epoch_millis();
        match report_type {
            PlayerReportType::Chat => world
                .start_chat_report_draft(report_id, created_at, target.profile_id)
                .is_ok(),
            PlayerReportType::Skin => world
                .start_skin_report_draft(report_id, created_at, target.profile_id, target.skin_url)
                .is_ok(),
            PlayerReportType::Username => world
                .start_username_report_draft(report_id, created_at, target.profile_id, target.name)
                .is_ok(),
        }
    }

    fn resume_player_reporting_draft(&mut self, world: &mut WorldStore) {
        let Some(draft) = world.copy_player_report_draft() else {
            return;
        };
        let report_type = draft.report_type();
        let profile_id = draft.common().reported_profile_id;
        if let Some(active) = self.player_reporting.active.as_mut() {
            if let Some(player) = active
                .previous_screen
                .as_ref()
                .and_then(|previous| match previous {
                    PlayerReportingPreviousInputState::SocialInteractions(social) => social
                        .rows()
                        .iter()
                        .find(|player| player.profile.uuid == profile_id),
                    PlayerReportingPreviousInputState::Death(_)
                    | PlayerReportingPreviousInputState::Pause(_) => None,
                })
                .cloned()
            {
                active.target = PlayerReportingTarget::from_social_player(&player);
            } else {
                active.target = PlayerReportingTarget::from_draft(&draft);
            }
            let len = text_edit::char_len(&draft.common().comments);
            active.comments_cursor = len;
            active.comments_selection = len;
        }
        self.transition_player_reporting_screen(
            world,
            PlayerReportingRuntimeScreen::Editor(report_type),
            editor_initial_focus(report_type),
        );
    }

    fn back_player_reporting_screen(&mut self, world: &mut WorldStore) {
        let Some(screen) = self
            .player_reporting
            .active
            .as_ref()
            .map(|active| active.screen.clone())
        else {
            return;
        };
        match screen {
            PlayerReportingRuntimeScreen::TypeChooser
            | PlayerReportingRuntimeScreen::DraftChoice => {
                self.close_player_reporting_screen(world);
            }
            PlayerReportingRuntimeScreen::Editor(report_type) => {
                let Some(draft) = world.copy_player_report_draft() else {
                    self.close_player_reporting_screen(world);
                    return;
                };
                if draft.has_content() {
                    self.transition_player_reporting_screen(
                        world,
                        PlayerReportingRuntimeScreen::DiscardWarning(report_type),
                        HudPlayerReportingFocus::WarningReturn,
                    );
                } else {
                    world.discard_player_report_draft(draft.common().report_id);
                    self.close_player_reporting_screen(world);
                }
            }
            PlayerReportingRuntimeScreen::ReasonSelection { editor_type, .. } => {
                self.transition_player_reporting_screen(
                    world,
                    PlayerReportingRuntimeScreen::Editor(editor_type),
                    HudPlayerReportingFocus::SelectReason,
                );
            }
            PlayerReportingRuntimeScreen::ChatSelection { .. } => {
                self.transition_player_reporting_screen(
                    world,
                    PlayerReportingRuntimeScreen::Editor(PlayerReportType::Chat),
                    HudPlayerReportingFocus::SelectMessages,
                );
            }
            PlayerReportingRuntimeScreen::DiscardWarning(report_type) => {
                self.transition_player_reporting_screen(
                    world,
                    PlayerReportingRuntimeScreen::Editor(report_type),
                    HudPlayerReportingFocus::Back,
                );
            }
            PlayerReportingRuntimeScreen::Sending { .. } => {}
            PlayerReportingRuntimeScreen::Success => {
                self.finish_player_reporting_screen(world);
            }
            PlayerReportingRuntimeScreen::Failure => {
                self.return_failed_player_report_submission_to_editor(world);
            }
        }
    }

    fn transition_player_reporting_screen(
        &mut self,
        world: &mut WorldStore,
        screen: PlayerReportingRuntimeScreen,
        focus: HudPlayerReportingFocus,
    ) {
        let Some(generation) = self
            .player_reporting
            .active
            .as_ref()
            .map(|active| active.generation)
        else {
            return;
        };
        let Some(canonical) = world.transition_player_reporting_screen(generation) else {
            self.player_reporting.active = None;
            return;
        };
        let active = self
            .player_reporting
            .active
            .as_mut()
            .expect("transition keeps the active report runtime");
        active.generation = canonical.generation;
        active.screen = screen;
        active.focus = focus;
        active.hovered = None;
        active.scroll_amount = 0.0;
    }

    fn close_player_reporting_screen(&mut self, world: &mut WorldStore) {
        let Some(generation) = self
            .player_reporting
            .active
            .as_ref()
            .map(|active| active.generation)
        else {
            return;
        };
        if !world.close_player_reporting_screen(generation) {
            return;
        }
        let active = self
            .player_reporting
            .active
            .take()
            .expect("matching report runtime remains active");
        if matches!(
            active.screen,
            PlayerReportingRuntimeScreen::Success | PlayerReportingRuntimeScreen::Failure
        ) {
            world.clear_player_report_submission();
        }
        match active.previous_screen {
            Some(PlayerReportingPreviousInputState::SocialInteractions(social)) => {
                self.restore_social_interactions_after_dialog(world, social);
            }
            Some(PlayerReportingPreviousInputState::Death(parent)) => {
                self.restore_death_screen_after_reporting(world, parent);
            }
            Some(PlayerReportingPreviousInputState::Pause(parent)) => {
                self.restore_pause_screen_after_reporting(world, parent);
            }
            None => {}
        }
    }

    fn finish_player_reporting_screen(&mut self, world: &mut WorldStore) {
        let Some(generation) = self
            .player_reporting
            .active
            .as_ref()
            .map(|active| active.generation)
        else {
            return;
        };
        if world.finish_player_reporting_screen(generation) {
            self.player_reporting.active = None;
            world.clear_player_report_submission();
        }
    }

    fn cancel_player_report_submission(
        &mut self,
        world: &mut WorldStore,
        report_type: PlayerReportType,
        report_id: Uuid,
        attempt: u64,
    ) {
        self.player_reporting
            .pending_submissions
            .retain(|request| request.report.id != report_id || request.attempt != attempt);
        self.player_reporting
            .cancelled_submissions
            .push((report_id, attempt));
        world.clear_player_report_submission();
        self.transition_player_reporting_screen(
            world,
            PlayerReportingRuntimeScreen::Editor(report_type),
            editor_initial_focus(report_type),
        );
    }

    fn return_failed_player_report_submission_to_editor(&mut self, world: &mut WorldStore) {
        let Some(report_type) = world
            .player_reporting()
            .draft
            .as_ref()
            .map(PlayerReportDraft::report_type)
        else {
            self.close_player_reporting_screen(world);
            return;
        };
        world.clear_player_report_submission();
        self.transition_player_reporting_screen(
            world,
            PlayerReportingRuntimeScreen::Editor(report_type),
            editor_initial_focus(report_type),
        );
    }

    fn current_report_target(&self) -> Uuid {
        self.player_reporting
            .active
            .as_ref()
            .map_or(Uuid::nil(), |active| active.target.profile_id)
    }
}
