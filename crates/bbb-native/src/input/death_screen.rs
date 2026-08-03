use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::ComponentClickEvent;
use bbb_renderer::{HudDeathScreenButton, HudDeathScreenHitTarget, HudDeathScreenHitTargetKind};
use bbb_world::WorldStore;
use tokio::sync::mpsc;
use winit::{
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
};

use super::{queue_perform_respawn_command, queue_sign_editor_removed, ClientInputState};

const DEATH_SCREEN_BUTTON_DELAY_TICKS: u32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DeathScreenMainInputState {
    pub(crate) generation: u64,
    pub(crate) elapsed_ticks: u32,
    pub(crate) focus: HudDeathScreenButton,
    pub(crate) hovered: Option<HudDeathScreenButton>,
}

impl DeathScreenMainInputState {
    pub(crate) fn buttons_active(self) -> bool {
        self.elapsed_ticks >= DEATH_SCREEN_BUTTON_DELAY_TICKS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DeathScreenQuitConfirmationInputState {
    pub(crate) generation: u64,
    pub(crate) elapsed_ticks: u32,
    pub(crate) focus: HudDeathScreenButton,
    pub(crate) hovered: Option<HudDeathScreenButton>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DeathScreenDraftConfirmationInputState {
    pub(crate) generation: u64,
    pub(crate) parent: DeathScreenMainInputState,
    pub(crate) focus: HudDeathScreenButton,
    pub(crate) hovered: Option<HudDeathScreenButton>,
}

impl DeathScreenQuitConfirmationInputState {
    pub(crate) fn buttons_active(self) -> bool {
        self.elapsed_ticks >= DEATH_SCREEN_BUTTON_DELAY_TICKS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DeathScreenLinkConfirmationInputState {
    pub(crate) generation: u64,
    pub(crate) parent: DeathScreenMainInputState,
    pub(crate) focus: HudDeathScreenButton,
    pub(crate) hovered: Option<HudDeathScreenButton>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeathScreenRuntime {
    Main(DeathScreenMainInputState),
    DraftConfirmation(DeathScreenDraftConfirmationInputState),
    QuitConfirmation(DeathScreenQuitConfirmationInputState),
    LinkConfirmation(DeathScreenLinkConfirmationInputState),
}

impl DeathScreenRuntime {
    pub(crate) fn generation(self) -> u64 {
        match self {
            Self::Main(state) => state.generation,
            Self::DraftConfirmation(state) => state.generation,
            Self::QuitConfirmation(state) => state.generation,
            Self::LinkConfirmation(state) => state.generation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DeathScreenPlatformRequest {
    OpenUri(String),
    ClipboardWrite(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DeathScreenInputState {
    active: Option<DeathScreenRuntime>,
    platform_requests: Vec<DeathScreenPlatformRequest>,
}

impl DeathScreenInputState {
    fn synchronize(&mut self, screen_generation: Option<u64>) -> bool {
        if screen_generation
            .zip(self.active)
            .is_some_and(|(generation, active)| generation == active.generation())
        {
            return false;
        }
        self.active = screen_generation.map(|generation| {
            DeathScreenRuntime::Main(DeathScreenMainInputState {
                generation,
                elapsed_ticks: 0,
                focus: HudDeathScreenButton::Respawn,
                hovered: None,
            })
        });
        true
    }

    fn take_platform_requests(&mut self) -> Vec<DeathScreenPlatformRequest> {
        std::mem::take(&mut self.platform_requests)
    }
}

impl ClientInputState {
    pub(crate) fn death_screen_runtime(&self) -> Option<DeathScreenRuntime> {
        self.death_screen.active
    }

    pub(crate) fn death_screen_status(
        &self,
        world: &WorldStore,
    ) -> Option<bbb_control::DeathScreenStatus> {
        let runtime = self.death_screen.active?;
        let canonical = world
            .death_screen()
            .filter(|screen| screen.generation == runtime.generation())?;
        let (screen, focused, elapsed_ticks, buttons_active) = match runtime {
            DeathScreenRuntime::Main(state) => (
                bbb_control::DeathScreenControlKind::Main,
                death_screen_control_focus(state.focus),
                state.elapsed_ticks,
                state.buttons_active(),
            ),
            DeathScreenRuntime::DraftConfirmation(state) => (
                bbb_control::DeathScreenControlKind::DraftConfirmation,
                death_screen_control_focus(state.focus),
                state.parent.elapsed_ticks,
                true,
            ),
            DeathScreenRuntime::QuitConfirmation(state) => (
                bbb_control::DeathScreenControlKind::QuitConfirmation,
                death_screen_control_focus(state.focus),
                state.elapsed_ticks,
                state.buttons_active(),
            ),
            DeathScreenRuntime::LinkConfirmation(state) => (
                bbb_control::DeathScreenControlKind::LinkConfirmation,
                death_screen_control_focus(state.focus),
                state.parent.elapsed_ticks,
                true,
            ),
        };
        Some(bbb_control::DeathScreenStatus {
            generation: runtime.generation(),
            screen,
            focused,
            elapsed_ticks,
            buttons_active,
            hardcore: canonical.hardcore,
            has_draft_report: world.player_reporting().draft.is_some(),
        })
    }

    pub(crate) fn sync_death_screen(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) -> bool {
        let generation = world.death_screen().map(|screen| screen.generation);
        let previous_was_link_confirmation = matches!(
            self.death_screen.active,
            Some(DeathScreenRuntime::LinkConfirmation(_))
        );
        let changed = self.death_screen.synchronize(generation);

        if generation.is_none() {
            if previous_was_link_confirmation && world.dialog_link_confirmation().is_some() {
                world.resolve_dialog_link_confirmation(false);
            }
            return changed;
        }

        if let Some(DeathScreenRuntime::LinkConfirmation(state)) = self.death_screen.active {
            if world.dialog_link_confirmation().is_none() {
                self.death_screen.active = Some(DeathScreenRuntime::Main(state.parent));
            }
        }
        if !changed {
            return false;
        }

        self.debug_options_screen = None;
        self.debug_pause_screen = None;
        self.debug_game_mode_switcher = None;
        self.pause_screen_disconnect_requested = false;
        self.interrupt_chat_screen();
        self.chat_entry = None;
        if let Some(editor) = self.sign_editor.take() {
            let signature = editor.signature.clone();
            queue_sign_editor_removed(counters, net_commands, &editor);
            self.dismissed_sign_editor = Some(signature);
        } else if let Some(signature) = super::sign_editor_signature_from_world(world) {
            self.dismissed_sign_editor = Some(signature);
        }
        self.sign_editor_ime_preedit = None;
        true
    }

    pub(crate) fn advance_death_screen_ticks(&mut self, client_ticks: u32) -> bool {
        if client_ticks == 0 {
            return self.death_screen.active.is_some();
        }
        match self.death_screen.active.as_mut() {
            Some(DeathScreenRuntime::Main(state)) => {
                state.elapsed_ticks = state.elapsed_ticks.saturating_add(client_ticks);
                true
            }
            Some(DeathScreenRuntime::QuitConfirmation(state)) => {
                state.elapsed_ticks = state.elapsed_ticks.saturating_add(client_ticks);
                true
            }
            Some(
                DeathScreenRuntime::DraftConfirmation(_) | DeathScreenRuntime::LinkConfirmation(_),
            ) => true,
            None => false,
        }
    }

    pub(crate) fn handle_death_screen_key(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        code: KeyCode,
        just_pressed: bool,
    ) -> bool {
        let reverse_tab = self.shift_down();
        let Some(active) = self.death_screen.active.as_mut() else {
            return false;
        };
        if !just_pressed {
            return true;
        }

        let mut activate = None;
        match active {
            DeathScreenRuntime::Main(state) => match code {
                KeyCode::Tab
                | KeyCode::ArrowLeft
                | KeyCode::ArrowRight
                | KeyCode::ArrowUp
                | KeyCode::ArrowDown => {
                    state.focus = match state.focus {
                        HudDeathScreenButton::Respawn => HudDeathScreenButton::TitleScreen,
                        _ => HudDeathScreenButton::Respawn,
                    };
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                    activate = Some(state.focus);
                }
                KeyCode::Escape => {}
                _ => {}
            },
            DeathScreenRuntime::DraftConfirmation(state) => match code {
                KeyCode::Tab
                | KeyCode::ArrowLeft
                | KeyCode::ArrowRight
                | KeyCode::ArrowUp
                | KeyCode::ArrowDown => {
                    state.focus = match state.focus {
                        HudDeathScreenButton::DraftEdit => HudDeathScreenButton::DraftDiscard,
                        _ => HudDeathScreenButton::DraftEdit,
                    };
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                    activate = Some(state.focus);
                }
                KeyCode::Escape => activate = Some(HudDeathScreenButton::DraftDiscard),
                _ => {}
            },
            DeathScreenRuntime::QuitConfirmation(state) => match code {
                KeyCode::Tab
                | KeyCode::ArrowLeft
                | KeyCode::ArrowRight
                | KeyCode::ArrowUp
                | KeyCode::ArrowDown => {
                    state.focus = match state.focus {
                        HudDeathScreenButton::QuitTitleScreen => HudDeathScreenButton::QuitRespawn,
                        _ => HudDeathScreenButton::QuitTitleScreen,
                    };
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                    activate = Some(state.focus);
                }
                KeyCode::Escape if state.buttons_active() => {
                    activate = Some(HudDeathScreenButton::QuitRespawn);
                }
                _ => {}
            },
            DeathScreenRuntime::LinkConfirmation(state) => match code {
                KeyCode::Escape => activate = Some(HudDeathScreenButton::LinkCancel),
                KeyCode::Tab if reverse_tab => {
                    state.focus = previous_link_button(state.focus);
                }
                KeyCode::Tab | KeyCode::ArrowRight | KeyCode::ArrowDown => {
                    state.focus = next_link_button(state.focus);
                }
                KeyCode::ArrowLeft | KeyCode::ArrowUp => {
                    state.focus = previous_link_button(state.focus);
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                    activate = Some(state.focus);
                }
                _ => {}
            },
        }
        if let Some(button) = activate {
            self.activate_death_screen_button(counters, world, net_commands, button);
        }
        true
    }

    pub(crate) fn handle_death_screen_hover(
        &mut self,
        target: Option<HudDeathScreenHitTarget>,
    ) -> bool {
        let Some(active) = self.death_screen.active.as_mut() else {
            return false;
        };
        let target = target.filter(|target| target.generation == active.generation());
        let hovered = target.and_then(|target| match target.kind {
            HudDeathScreenHitTargetKind::Button(button) => Some(button),
            HudDeathScreenHitTargetKind::CauseLink { .. } => None,
        });
        match active {
            DeathScreenRuntime::Main(state) => {
                state.hovered = hovered.filter(|button| {
                    matches!(
                        button,
                        HudDeathScreenButton::Respawn | HudDeathScreenButton::TitleScreen
                    )
                });
            }
            DeathScreenRuntime::DraftConfirmation(state) => {
                state.hovered = hovered.filter(|button| {
                    matches!(
                        button,
                        HudDeathScreenButton::DraftEdit | HudDeathScreenButton::DraftDiscard
                    )
                });
            }
            DeathScreenRuntime::QuitConfirmation(state) => {
                state.hovered = hovered.filter(|button| {
                    matches!(
                        button,
                        HudDeathScreenButton::QuitTitleScreen | HudDeathScreenButton::QuitRespawn
                    )
                });
            }
            DeathScreenRuntime::LinkConfirmation(state) => {
                state.hovered = hovered.filter(|button| {
                    matches!(
                        button,
                        HudDeathScreenButton::LinkOpen
                            | HudDeathScreenButton::LinkCopy
                            | HudDeathScreenButton::LinkCancel
                    )
                });
            }
        }
        true
    }

    pub(crate) fn handle_death_screen_mouse_input(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: MouseButton,
        state: ElementState,
        target: Option<HudDeathScreenHitTarget>,
    ) -> bool {
        if button != MouseButton::Left || state != ElementState::Pressed {
            return self.death_screen.active.is_some();
        }
        let Some(active) = self.death_screen.active else {
            return false;
        };
        let Some(target) = target.filter(|target| target.generation == active.generation()) else {
            return true;
        };
        match target.kind {
            HudDeathScreenHitTargetKind::Button(button) => {
                self.activate_death_screen_button(counters, world, net_commands, button);
            }
            HudDeathScreenHitTargetKind::CauseLink { run_index }
                if matches!(active, DeathScreenRuntime::Main(_)) =>
            {
                self.activate_death_screen_cause_link(world, run_index);
            }
            HudDeathScreenHitTargetKind::CauseLink { .. } => {}
        }
        true
    }

    pub(crate) fn take_death_screen_platform_requests(
        &mut self,
    ) -> Vec<DeathScreenPlatformRequest> {
        self.death_screen.take_platform_requests()
    }

    fn activate_death_screen_button(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: HudDeathScreenButton,
    ) {
        let Some(active) = self.death_screen.active else {
            return;
        };
        let generation = active.generation();
        if world.death_screen().map(|screen| screen.generation) != Some(generation) {
            self.death_screen.active = None;
            return;
        }
        match (active, button) {
            (DeathScreenRuntime::Main(state), HudDeathScreenButton::Respawn)
                if state.buttons_active() =>
            {
                complete_death_screen_respawn(
                    &mut self.death_screen,
                    counters,
                    world,
                    net_commands,
                    generation,
                );
            }
            (DeathScreenRuntime::Main(state), HudDeathScreenButton::TitleScreen)
                if state.buttons_active() =>
            {
                if world.player_reporting().draft.is_some() {
                    self.death_screen.active = Some(DeathScreenRuntime::DraftConfirmation(
                        DeathScreenDraftConfirmationInputState {
                            generation,
                            parent: state,
                            focus: HudDeathScreenButton::DraftEdit,
                            hovered: None,
                        },
                    ));
                } else {
                    continue_death_screen_exit(&mut self.death_screen, world, net_commands, state);
                }
            }
            (DeathScreenRuntime::DraftConfirmation(state), HudDeathScreenButton::DraftEdit) => {
                if self.open_player_reporting_draft_from_death(world, state.parent) {
                    self.death_screen.active = None;
                }
            }
            (DeathScreenRuntime::DraftConfirmation(state), HudDeathScreenButton::DraftDiscard) => {
                if let Some(draft) = world.copy_player_report_draft() {
                    world.discard_player_report_draft(draft.common().report_id);
                }
                continue_death_screen_exit(
                    &mut self.death_screen,
                    world,
                    net_commands,
                    state.parent,
                );
            }
            (
                DeathScreenRuntime::QuitConfirmation(state),
                HudDeathScreenButton::QuitTitleScreen,
            ) if state.buttons_active() => {
                complete_death_screen_disconnect(
                    &mut self.death_screen,
                    world,
                    net_commands,
                    generation,
                );
            }
            (DeathScreenRuntime::QuitConfirmation(state), HudDeathScreenButton::QuitRespawn)
                if state.buttons_active() =>
            {
                complete_death_screen_respawn(
                    &mut self.death_screen,
                    counters,
                    world,
                    net_commands,
                    generation,
                );
            }
            (DeathScreenRuntime::LinkConfirmation(state), HudDeathScreenButton::LinkOpen) => {
                world.resolve_dialog_link_confirmation(true);
                self.death_screen.platform_requests.extend(
                    world
                        .take_dialog_open_url_requests()
                        .into_iter()
                        .map(DeathScreenPlatformRequest::OpenUri),
                );
                self.death_screen.active = Some(DeathScreenRuntime::Main(state.parent));
            }
            (DeathScreenRuntime::LinkConfirmation(state), HudDeathScreenButton::LinkCopy) => {
                if let Some(url) = world
                    .dialog_link_confirmation()
                    .map(|confirmation| confirmation.url.clone())
                {
                    self.death_screen
                        .platform_requests
                        .push(DeathScreenPlatformRequest::ClipboardWrite(url));
                }
                world.resolve_dialog_link_confirmation(false);
                self.death_screen.active = Some(DeathScreenRuntime::Main(state.parent));
            }
            (DeathScreenRuntime::LinkConfirmation(state), HudDeathScreenButton::LinkCancel) => {
                world.resolve_dialog_link_confirmation(false);
                self.death_screen.active = Some(DeathScreenRuntime::Main(state.parent));
            }
            _ => {}
        }
    }

    fn activate_death_screen_cause_link(&mut self, world: &mut WorldStore, run_index: u16) {
        let Some(DeathScreenRuntime::Main(parent)) = self.death_screen.active else {
            return;
        };
        let Some(screen) = world
            .death_screen()
            .filter(|screen| screen.generation == parent.generation)
        else {
            self.death_screen.active = None;
            return;
        };
        let Some(ComponentClickEvent::OpenUrl { url }) = screen
            .cause_of_death
            .styled()
            .get(usize::from(run_index))
            .and_then(|run| run.style.click_event.clone())
        else {
            return;
        };
        if !world.open_chat_url(url) {
            return;
        }
        self.death_screen.platform_requests.extend(
            world
                .take_dialog_open_url_requests()
                .into_iter()
                .map(DeathScreenPlatformRequest::OpenUri),
        );
        if world.dialog_link_confirmation().is_some() {
            self.death_screen.active = Some(DeathScreenRuntime::LinkConfirmation(
                DeathScreenLinkConfirmationInputState {
                    generation: parent.generation,
                    parent,
                    focus: HudDeathScreenButton::LinkOpen,
                    hovered: None,
                },
            ));
        }
    }

    pub(super) fn restore_death_screen_after_reporting(
        &mut self,
        world: &WorldStore,
        parent: DeathScreenMainInputState,
    ) {
        self.death_screen.active = (world.death_screen().map(|screen| screen.generation)
            == Some(parent.generation))
        .then_some(DeathScreenRuntime::Main(parent));
    }
}

fn death_screen_control_focus(focus: HudDeathScreenButton) -> bbb_control::DeathScreenControlFocus {
    match focus {
        HudDeathScreenButton::Respawn => bbb_control::DeathScreenControlFocus::Respawn,
        HudDeathScreenButton::TitleScreen => bbb_control::DeathScreenControlFocus::TitleScreen,
        HudDeathScreenButton::DraftEdit => bbb_control::DeathScreenControlFocus::DraftEdit,
        HudDeathScreenButton::DraftDiscard => bbb_control::DeathScreenControlFocus::DraftDiscard,
        HudDeathScreenButton::QuitTitleScreen => {
            bbb_control::DeathScreenControlFocus::QuitTitleScreen
        }
        HudDeathScreenButton::QuitRespawn => bbb_control::DeathScreenControlFocus::QuitRespawn,
        HudDeathScreenButton::LinkOpen => bbb_control::DeathScreenControlFocus::LinkOpen,
        HudDeathScreenButton::LinkCopy => bbb_control::DeathScreenControlFocus::LinkCopy,
        HudDeathScreenButton::LinkCancel => bbb_control::DeathScreenControlFocus::LinkCancel,
    }
}

fn next_link_button(button: HudDeathScreenButton) -> HudDeathScreenButton {
    match button {
        HudDeathScreenButton::LinkOpen => HudDeathScreenButton::LinkCopy,
        HudDeathScreenButton::LinkCopy => HudDeathScreenButton::LinkCancel,
        _ => HudDeathScreenButton::LinkOpen,
    }
}

fn previous_link_button(button: HudDeathScreenButton) -> HudDeathScreenButton {
    match button {
        HudDeathScreenButton::LinkOpen => HudDeathScreenButton::LinkCancel,
        HudDeathScreenButton::LinkCopy => HudDeathScreenButton::LinkOpen,
        _ => HudDeathScreenButton::LinkCopy,
    }
}

fn complete_death_screen_respawn(
    state: &mut DeathScreenInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    generation: u64,
) {
    if world.close_death_screen(generation) {
        queue_perform_respawn_command(counters, net_commands);
    }
    state.active = None;
}

fn complete_death_screen_disconnect(
    state: &mut DeathScreenInputState,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    generation: u64,
) {
    if world.close_death_screen(generation) {
        if let Some(tx) = net_commands {
            let _ = tx.try_send(NetCommand::Disconnect);
        }
    }
    state.active = None;
}

fn continue_death_screen_exit(
    state: &mut DeathScreenInputState,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    parent: DeathScreenMainInputState,
) {
    if world.death_screen().is_some_and(|screen| screen.hardcore) {
        complete_death_screen_disconnect(state, world, net_commands, parent.generation);
    } else {
        state.active = Some(DeathScreenRuntime::QuitConfirmation(
            DeathScreenQuitConfirmationInputState {
                generation: parent.generation,
                elapsed_ticks: 0,
                focus: HudDeathScreenButton::QuitTitleScreen,
                hovered: None,
            },
        ));
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        packets::{CommonPlayerSpawnInfo, PlayLogin, PlayerCombatKill as ProtocolPlayerCombatKill},
        NbtSummaryEntry, NbtSummaryValue, RichTextComponent,
    };
    use bbb_renderer::{HudPlayerReportingFocus, HudPlayerReportingHitTarget};
    use bbb_world::{PlayerReportEnvironment, PlayerReportType};
    use uuid::Uuid;

    use super::*;
    use crate::input::reporting::PlayerReportingRuntimeScreen;

    fn death_world(hardcore: bool, cause: RichTextComponent) -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_login(&PlayLogin {
            player_id: 7,
            hardcore,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        });
        world.apply_player_combat_kill(ProtocolPlayerCombatKill {
            player_id: 7,
            message: cause,
        });
        world
    }

    fn hit(generation: u64, kind: HudDeathScreenHitTargetKind) -> HudDeathScreenHitTarget {
        HudDeathScreenHitTarget { generation, kind }
    }

    fn add_username_draft(world: &mut WorldStore, comments: &str) -> Uuid {
        let report_id = Uuid::from_u128(11);
        world.set_player_report_environment(Some(PlayerReportEnvironment::third_party(
            "26.1",
            "en-US",
            "example.org:25565",
        )));
        world
            .start_username_report_draft(report_id, 0, Uuid::from_u128(12), "ReportedPlayer")
            .unwrap();
        assert!(world.set_player_report_comments(report_id, comments.to_string()));
        report_id
    }

    #[test]
    fn death_screen_buttons_unlock_after_twenty_ticks_and_respawn_once() {
        let mut world = death_world(false, RichTextComponent::plain("You died"));
        let generation = world.death_screen().unwrap().generation;
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        assert!(input.sync_death_screen(&mut world, &mut counters, &commands));
        assert_eq!(
            input.death_screen_status(&world),
            Some(bbb_control::DeathScreenStatus {
                generation,
                screen: bbb_control::DeathScreenControlKind::Main,
                focused: bbb_control::DeathScreenControlFocus::Respawn,
                elapsed_ticks: 0,
                buttons_active: false,
                hardcore: false,
                has_draft_report: false,
            })
        );

        assert!(input.handle_death_screen_key(
            &mut counters,
            &mut world,
            &commands,
            KeyCode::Enter,
            true,
        ));
        assert!(rx.try_recv().is_err());
        assert!(world.death_screen().is_some());

        input.advance_death_screen_ticks(19);
        input.handle_death_screen_mouse_input(
            &mut counters,
            &mut world,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(hit(
                generation,
                HudDeathScreenHitTargetKind::Button(HudDeathScreenButton::Respawn),
            )),
        );
        assert!(rx.try_recv().is_err());

        input.advance_death_screen_ticks(1);
        assert!(input
            .death_screen_status(&world)
            .is_some_and(|status| status.elapsed_ticks == 20 && status.buttons_active));
        input.handle_death_screen_mouse_input(
            &mut counters,
            &mut world,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(hit(
                generation,
                HudDeathScreenHitTargetKind::Button(HudDeathScreenButton::Respawn),
            )),
        );
        assert_eq!(rx.try_recv(), Ok(NetCommand::PerformRespawn));
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.perform_respawn_commands_queued, 1);
        assert_eq!(world.death_screen(), None);
        assert_eq!(input.death_screen_runtime(), None);
        assert_eq!(input.death_screen_status(&world), None);
    }

    #[test]
    fn escape_never_closes_main_screen_and_delayed_quit_escape_respawns() {
        let mut world = death_world(false, RichTextComponent::plain("You died"));
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        input.sync_death_screen(&mut world, &mut counters, &commands);

        input.handle_death_screen_key(&mut counters, &mut world, &commands, KeyCode::Escape, true);
        assert!(world.death_screen().is_some());
        input.advance_death_screen_ticks(20);
        input.handle_death_screen_key(&mut counters, &mut world, &commands, KeyCode::Tab, true);
        input.handle_death_screen_key(&mut counters, &mut world, &commands, KeyCode::Enter, true);
        assert!(matches!(
            input.death_screen_runtime(),
            Some(DeathScreenRuntime::QuitConfirmation(_))
        ));

        input.handle_death_screen_key(&mut counters, &mut world, &commands, KeyCode::Escape, true);
        assert!(rx.try_recv().is_err());
        input.advance_death_screen_ticks(20);
        input.handle_death_screen_key(&mut counters, &mut world, &commands, KeyCode::Escape, true);
        assert_eq!(rx.try_recv(), Ok(NetCommand::PerformRespawn));
        assert!(rx.try_recv().is_err());
        assert_eq!(world.death_screen(), None);
    }

    #[test]
    fn hardcore_title_button_disconnects_once() {
        let mut world = death_world(true, RichTextComponent::plain("Hardcore death"));
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        input.sync_death_screen(&mut world, &mut counters, &commands);
        input.advance_death_screen_ticks(20);
        input.handle_death_screen_key(&mut counters, &mut world, &commands, KeyCode::Tab, true);
        input.handle_death_screen_key(&mut counters, &mut world, &commands, KeyCode::Enter, true);

        assert_eq!(rx.try_recv(), Ok(NetCommand::Disconnect));
        assert!(rx.try_recv().is_err());
        assert_eq!(world.death_screen(), None);
    }

    #[test]
    fn draft_discard_escape_continues_normal_and_hardcore_exit_paths() {
        for hardcore in [false, true] {
            let mut world = death_world(hardcore, RichTextComponent::plain("You died"));
            add_username_draft(&mut world, "saved draft");
            let mut input = ClientInputState::new(true);
            let mut counters = NetCounters::default();
            let (tx, mut rx) = mpsc::channel(4);
            let commands = Some(tx);
            input.sync_death_screen(&mut world, &mut counters, &commands);
            input.advance_death_screen_ticks(20);
            input.handle_death_screen_key(&mut counters, &mut world, &commands, KeyCode::Tab, true);
            input.handle_death_screen_key(
                &mut counters,
                &mut world,
                &commands,
                KeyCode::Enter,
                true,
            );
            assert!(matches!(
                input.death_screen_runtime(),
                Some(DeathScreenRuntime::DraftConfirmation(_))
            ));

            input.handle_death_screen_key(
                &mut counters,
                &mut world,
                &commands,
                KeyCode::Escape,
                true,
            );
            assert!(world.player_reporting().draft.is_none());
            if hardcore {
                assert_eq!(rx.try_recv(), Ok(NetCommand::Disconnect));
                assert!(world.death_screen().is_none());
            } else {
                assert!(matches!(
                    input.death_screen_runtime(),
                    Some(DeathScreenRuntime::QuitConfirmation(state))
                        if !state.buttons_active()
                ));
                input.advance_death_screen_ticks(20);
                input.handle_death_screen_key(
                    &mut counters,
                    &mut world,
                    &commands,
                    KeyCode::Escape,
                    true,
                );
                assert_eq!(rx.try_recv(), Ok(NetCommand::PerformRespawn));
            }
            assert!(rx.try_recv().is_err());
        }
    }

    #[test]
    fn draft_edit_restores_same_death_screen_after_saving_draft() {
        let mut world = death_world(false, RichTextComponent::plain("You died"));
        let report_id = add_username_draft(&mut world, "saved draft");
        let generation = world.death_screen().unwrap().generation;
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        input.sync_death_screen(&mut world, &mut counters, &commands);
        input.advance_death_screen_ticks(20);
        input.handle_death_screen_key(&mut counters, &mut world, &commands, KeyCode::Tab, true);
        input.handle_death_screen_key(&mut counters, &mut world, &commands, KeyCode::Enter, true);
        input.handle_death_screen_key(&mut counters, &mut world, &commands, KeyCode::Enter, true);
        assert!(world.death_screen().is_none());
        assert!(matches!(
            input
                .player_reporting_runtime()
                .map(|runtime| runtime.screen()),
            Some(PlayerReportingRuntimeScreen::Editor(
                PlayerReportType::Username
            ))
        ));

        input.handle_player_reporting_key(&mut world, KeyCode::Escape, true);
        assert!(matches!(
            input
                .player_reporting_runtime()
                .map(|runtime| runtime.screen()),
            Some(PlayerReportingRuntimeScreen::DiscardWarning(
                PlayerReportType::Username
            ))
        ));
        let reporting_generation = input.player_reporting_runtime().unwrap().generation();
        input.handle_player_reporting_mouse_input(
            &mut world,
            MouseButton::Left,
            ElementState::Pressed,
            Some(HudPlayerReportingHitTarget {
                generation: reporting_generation,
                focus: HudPlayerReportingFocus::WarningSaveDraft,
            }),
        );
        assert_eq!(
            world
                .player_reporting()
                .draft
                .as_ref()
                .map(|draft| draft.common().report_id),
            Some(report_id)
        );
        assert_eq!(
            world.death_screen().map(|screen| screen.generation),
            Some(generation)
        );
        assert!(matches!(
            input.death_screen_runtime(),
            Some(DeathScreenRuntime::Main(state))
                if state.generation == generation && state.elapsed_ticks == 20
        ));

        input.handle_death_screen_key(&mut counters, &mut world, &commands, KeyCode::Tab, true);
        input.handle_death_screen_key(&mut counters, &mut world, &commands, KeyCode::Enter, true);
        assert_eq!(rx.try_recv(), Ok(NetCommand::PerformRespawn));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn cause_link_is_generation_checked_and_returns_to_same_death_screen() {
        let cause = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("Open details".to_string()),
            },
            NbtSummaryEntry {
                name: "click_event".to_string(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "action".to_string(),
                        value: NbtSummaryValue::String("open_url".to_string()),
                    },
                    NbtSummaryEntry {
                        name: "url".to_string(),
                        value: NbtSummaryValue::String("https://example.invalid/death".to_string()),
                    },
                ]),
            },
        ]))
        .unwrap();
        let mut world = death_world(false, cause);
        let generation = world.death_screen().unwrap().generation;
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        input.sync_death_screen(&mut world, &mut counters, &None);

        input.handle_death_screen_mouse_input(
            &mut counters,
            &mut world,
            &None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(hit(
                generation.saturating_add(1),
                HudDeathScreenHitTargetKind::CauseLink { run_index: 0 },
            )),
        );
        assert!(matches!(
            input.death_screen_runtime(),
            Some(DeathScreenRuntime::Main(_))
        ));

        input.handle_death_screen_mouse_input(
            &mut counters,
            &mut world,
            &None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(hit(
                generation,
                HudDeathScreenHitTargetKind::CauseLink { run_index: 0 },
            )),
        );
        assert!(matches!(
            input.death_screen_runtime(),
            Some(DeathScreenRuntime::LinkConfirmation(_))
        ));
        assert!(world.dialog_link_confirmation().is_some());

        input.handle_death_screen_key(&mut counters, &mut world, &None, KeyCode::Tab, true);
        input.handle_death_screen_key(&mut counters, &mut world, &None, KeyCode::Enter, true);
        assert_eq!(
            input.take_death_screen_platform_requests(),
            vec![DeathScreenPlatformRequest::ClipboardWrite(
                "https://example.invalid/death".to_string()
            )]
        );
        assert!(matches!(
            input.death_screen_runtime(),
            Some(DeathScreenRuntime::Main(state)) if state.generation == generation
        ));
        assert!(world.death_screen().is_some());
        assert!(world.dialog_link_confirmation().is_none());

        input.handle_death_screen_mouse_input(
            &mut counters,
            &mut world,
            &None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(hit(
                generation,
                HudDeathScreenHitTargetKind::CauseLink { run_index: 0 },
            )),
        );
        assert!(world.dialog_link_confirmation().is_some());
        world.clear_death_screen();
        assert!(input.sync_death_screen(&mut world, &mut counters, &None));
        assert_eq!(input.death_screen_runtime(), None);
        assert!(world.dialog_link_confirmation().is_none());
    }

    #[test]
    fn death_reporting_return_path_cannot_restore_social_only_shadow_state() {
        let native_reporting = include_str!("reporting.rs");
        let world_reporting = include_str!("../../../bbb-world/src/client/reporting.rs");
        let world_ui = include_str!("../../../bbb-world/src/client/ui.rs");
        for source in [native_reporting, world_reporting, world_ui] {
            assert!(!source.contains("suspended_social"));
            assert!(!source.contains("player_reporting_previous_social_screen"));
        }
        assert!(native_reporting.contains("enum PlayerReportingPreviousInputState"));
        assert!(world_ui.contains("enum PlayerReportingPreviousScreenState"));
        for variant in ["SocialInteractions", "Death"] {
            assert!(native_reporting.contains(variant));
            assert!(world_ui.contains(variant));
        }
    }
}
