use crate::control::NetCounters;
use bbb_client_net::NetCommand;
use bbb_client_render::{
    HudDeathScreenButton, HudDeathScreenHitTarget, HudDeathScreenHitTargetKind,
};
use bbb_client_world::ClientDeathScreenState;
use bbb_protocol::ComponentClickEvent;
use tokio::sync::mpsc;
use winit::{
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
};

use super::{queue_perform_respawn_command, ClientInputState, Screen};

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

impl DeathScreenQuitConfirmationInputState {
    pub(crate) fn buttons_active(self) -> bool {
        self.elapsed_ticks >= DEATH_SCREEN_BUTTON_DELAY_TICKS
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeathScreenLinkConfirmationInputState {
    pub(crate) confirmation: super::dialog::DialogLinkConfirmationState,
    pub(crate) generation: u64,
    pub(crate) parent: DeathScreenMainInputState,
    pub(crate) focus: HudDeathScreenButton,
    pub(crate) hovered: Option<HudDeathScreenButton>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DeathScreenRuntime {
    Main(DeathScreenMainInputState),
    QuitConfirmation(DeathScreenQuitConfirmationInputState),
    LinkConfirmation(DeathScreenLinkConfirmationInputState),
}

impl DeathScreenRuntime {
    pub(crate) fn generation(&self) -> u64 {
        match self {
            Self::Main(state) => state.generation,
            Self::QuitConfirmation(state) => state.generation,
            Self::LinkConfirmation(state) => state.generation,
        }
    }
}

/// `DeathScreen` with its title confirmation and cause-link confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeathScreen {
    pub(crate) screen: ClientDeathScreenState,
    pub(crate) runtime: DeathScreenRuntime,
}

impl DeathScreen {
    pub(crate) fn new(screen: ClientDeathScreenState) -> Self {
        let runtime = DeathScreenRuntime::Main(DeathScreenMainInputState {
            generation: screen.generation,
            elapsed_ticks: 0,
            focus: HudDeathScreenButton::Respawn,
            hovered: None,
        });
        Self { screen, runtime }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DeathScreenPlatformRequest {
    OpenUri(bbb_protocol::JavaString),
    ClipboardWrite(bbb_protocol::JavaString),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DeathScreenInputState {
    platform_requests: Vec<DeathScreenPlatformRequest>,
}

impl ClientInputState {
    pub(crate) fn death_screen(&self) -> Option<&DeathScreen> {
        self.screen.death()
    }

    pub(crate) fn death_screen_runtime(&self) -> Option<&DeathScreenRuntime> {
        self.screen.death().map(|death| &death.runtime)
    }

    pub(crate) fn death_screen_status(&self) -> Option<crate::control::DeathScreenStatus> {
        let death = self.screen.death()?;
        let runtime = &death.runtime;
        let (screen, focused, elapsed_ticks, buttons_active) = match runtime {
            DeathScreenRuntime::Main(state) => (
                crate::control::DeathScreenControlKind::Main,
                death_screen_control_focus(state.focus),
                state.elapsed_ticks,
                state.buttons_active(),
            ),
            DeathScreenRuntime::QuitConfirmation(state) => (
                crate::control::DeathScreenControlKind::QuitConfirmation,
                death_screen_control_focus(state.focus),
                state.elapsed_ticks,
                state.buttons_active(),
            ),
            DeathScreenRuntime::LinkConfirmation(state) => (
                crate::control::DeathScreenControlKind::LinkConfirmation,
                death_screen_control_focus(state.focus),
                state.parent.elapsed_ticks,
                true,
            ),
        };
        Some(crate::control::DeathScreenStatus {
            generation: runtime.generation(),
            screen,
            focused,
            elapsed_ticks,
            buttons_active,
            hardcore: death.screen.hardcore,
        })
    }

    pub(crate) fn advance_death_screen_ticks(&mut self, client_ticks: u32) -> bool {
        if client_ticks == 0 {
            return self.screen.death().is_some();
        }
        match self.screen.death_mut().map(|death| &mut death.runtime) {
            Some(DeathScreenRuntime::Main(state)) => {
                state.elapsed_ticks = state.elapsed_ticks.saturating_add(client_ticks);
                true
            }
            Some(DeathScreenRuntime::QuitConfirmation(state)) => {
                state.elapsed_ticks = state.elapsed_ticks.saturating_add(client_ticks);
                true
            }
            Some(DeathScreenRuntime::LinkConfirmation(_)) => true,
            None => false,
        }
    }

    pub(crate) fn handle_death_screen_key(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        code: KeyCode,
        just_pressed: bool,
    ) -> bool {
        let reverse_tab = self.shift_down();
        let Some(active) = self.screen.death_mut().map(|death| &mut death.runtime) else {
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
            self.activate_death_screen_button(counters, net_commands, button);
        }
        true
    }

    pub(crate) fn handle_death_screen_hover(
        &mut self,
        target: Option<HudDeathScreenHitTarget>,
    ) -> bool {
        let Some(active) = self.screen.death_mut().map(|death| &mut death.runtime) else {
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
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: MouseButton,
        state: ElementState,
        target: Option<HudDeathScreenHitTarget>,
    ) -> bool {
        if button != MouseButton::Left || state != ElementState::Pressed {
            return self.screen.death().is_some();
        }
        let Some(active) = self.death_screen_runtime() else {
            return false;
        };
        let Some(target) = target.filter(|target| target.generation == active.generation()) else {
            return true;
        };
        match target.kind {
            HudDeathScreenHitTargetKind::Button(button) => {
                self.activate_death_screen_button(counters, net_commands, button);
            }
            HudDeathScreenHitTargetKind::CauseLink { run_index }
                if matches!(active, DeathScreenRuntime::Main(_)) =>
            {
                self.activate_death_screen_cause_link(run_index);
            }
            HudDeathScreenHitTargetKind::CauseLink { .. } => {}
        }
        true
    }

    pub(crate) fn take_death_screen_platform_requests(
        &mut self,
    ) -> Vec<DeathScreenPlatformRequest> {
        std::mem::take(&mut self.death_screen.platform_requests)
    }

    fn close_death_screen(&mut self) {
        self.screen
            .close_if(|screen| matches!(screen, Screen::Death(_)));
    }

    fn set_death_screen_runtime(&mut self, runtime: DeathScreenRuntime) {
        if let Some(death) = self.screen.death_mut() {
            death.runtime = runtime;
        }
    }

    fn activate_death_screen_button(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: HudDeathScreenButton,
    ) {
        let Some(death) = self.screen.death() else {
            return;
        };
        let (active, hardcore) = (&death.runtime, death.screen.hardcore);
        match (active, button) {
            (DeathScreenRuntime::Main(state), HudDeathScreenButton::Respawn)
                if state.buttons_active() =>
            {
                self.complete_death_screen_respawn(counters, net_commands);
            }
            (DeathScreenRuntime::Main(state), HudDeathScreenButton::TitleScreen)
                if state.buttons_active() =>
            {
                if hardcore {
                    self.complete_death_screen_disconnect(net_commands);
                } else {
                    self.set_death_screen_runtime(DeathScreenRuntime::QuitConfirmation(
                        DeathScreenQuitConfirmationInputState {
                            generation: state.generation,
                            elapsed_ticks: 0,
                            focus: HudDeathScreenButton::QuitTitleScreen,
                            hovered: None,
                        },
                    ));
                }
            }
            (
                DeathScreenRuntime::QuitConfirmation(state),
                HudDeathScreenButton::QuitTitleScreen,
            ) if state.buttons_active() => {
                self.complete_death_screen_disconnect(net_commands);
            }
            (DeathScreenRuntime::QuitConfirmation(state), HudDeathScreenButton::QuitRespawn)
                if state.buttons_active() =>
            {
                self.complete_death_screen_respawn(counters, net_commands);
            }
            (
                DeathScreenRuntime::LinkConfirmation(state),
                button @ (HudDeathScreenButton::LinkOpen
                | HudDeathScreenButton::LinkCopy
                | HudDeathScreenButton::LinkCancel),
            ) => {
                let parent = state.parent;
                let runtime = &mut self.screen.death_mut().unwrap().runtime;
                let DeathScreenRuntime::LinkConfirmation(state) =
                    std::mem::replace(runtime, DeathScreenRuntime::Main(parent))
                else {
                    unreachable!()
                };
                match button {
                    HudDeathScreenButton::LinkOpen => self
                        .death_screen
                        .platform_requests
                        .push(DeathScreenPlatformRequest::OpenUri(state.confirmation.url)),
                    HudDeathScreenButton::LinkCopy => self.death_screen.platform_requests.push(
                        DeathScreenPlatformRequest::ClipboardWrite(state.confirmation.url),
                    ),
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn complete_death_screen_respawn(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        self.close_death_screen();
        queue_perform_respawn_command(counters, net_commands);
    }

    fn complete_death_screen_disconnect(
        &mut self,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        self.close_death_screen();
        self.mark_local_disconnect_requested();
        if let Some(tx) = net_commands {
            let _ = tx.try_send(NetCommand::Disconnect);
        }
    }

    fn activate_death_screen_cause_link(&mut self, run_index: u16) {
        let Some(DeathScreen {
            screen,
            runtime: DeathScreenRuntime::Main(parent),
        }) = self.screen.death()
        else {
            return;
        };
        let parent = *parent;
        let Some(ComponentClickEvent::OpenUrl { url }) = screen
            .cause_of_death
            .styled()
            .get(usize::from(run_index))
            .and_then(|run| run.style.click_event.clone())
        else {
            return;
        };
        match self.dialog_url_action(url, "none".into()) {
            None => {}
            Some(super::dialog::DialogUrlAction::Open(url)) => self
                .death_screen
                .platform_requests
                .push(DeathScreenPlatformRequest::OpenUri(url)),
            Some(super::dialog::DialogUrlAction::Confirm(confirmation)) => {
                self.set_death_screen_runtime(DeathScreenRuntime::LinkConfirmation(
                    DeathScreenLinkConfirmationInputState {
                        confirmation,
                        generation: parent.generation,
                        parent,
                        focus: HudDeathScreenButton::LinkOpen,
                        hovered: None,
                    },
                ));
            }
        }
    }
}

fn death_screen_control_focus(
    focus: HudDeathScreenButton,
) -> crate::control::DeathScreenControlFocus {
    match focus {
        HudDeathScreenButton::Respawn => crate::control::DeathScreenControlFocus::Respawn,
        HudDeathScreenButton::TitleScreen => crate::control::DeathScreenControlFocus::TitleScreen,
        HudDeathScreenButton::QuitTitleScreen => {
            crate::control::DeathScreenControlFocus::QuitTitleScreen
        }
        HudDeathScreenButton::QuitRespawn => crate::control::DeathScreenControlFocus::QuitRespawn,
        HudDeathScreenButton::LinkOpen => crate::control::DeathScreenControlFocus::LinkOpen,
        HudDeathScreenButton::LinkCopy => crate::control::DeathScreenControlFocus::LinkCopy,
        HudDeathScreenButton::LinkCancel => crate::control::DeathScreenControlFocus::LinkCancel,
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

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        packets::{CommonPlayerSpawnInfo, PlayLogin, PlayerCombatKill as ProtocolPlayerCombatKill},
        NbtSummaryEntry, NbtSummaryValue, RichTextComponent,
    };

    use super::*;
    use bbb_client_world::WorldStore;

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
            common_spawn_info: overworld_spawn_info(),
            enforces_secure_chat: false,
        });
        world.apply_player_combat_kill(ProtocolPlayerCombatKill {
            player_id: 7,
            message: cause,
        });
        world
    }

    fn overworld_spawn_info() -> CommonPlayerSpawnInfo {
        CommonPlayerSpawnInfo {
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
        }
    }

    fn link_cause() -> RichTextComponent {
        RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".into(),
                value: NbtSummaryValue::string("Open details"),
            },
            NbtSummaryEntry {
                name: "click_event".into(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "action".into(),
                        value: NbtSummaryValue::string("open_url"),
                    },
                    NbtSummaryEntry {
                        name: "url".into(),
                        value: NbtSummaryValue::string("https://example.invalid/death"),
                    },
                ]),
            },
        ]))
        .unwrap()
    }

    fn hit(generation: u64, kind: HudDeathScreenHitTargetKind) -> HudDeathScreenHitTarget {
        HudDeathScreenHitTarget { generation, kind }
    }

    #[test]
    fn death_screen_buttons_unlock_after_twenty_ticks_and_respawn_once() {
        let mut world = death_world(false, RichTextComponent::plain("You died"));
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        input.apply_screen_requests(&mut world, &mut counters, &commands);
        let generation = input.death_screen().unwrap().screen.generation;
        assert_eq!(
            input.death_screen_status(),
            Some(crate::control::DeathScreenStatus {
                generation,
                screen: crate::control::DeathScreenControlKind::Main,
                focused: crate::control::DeathScreenControlFocus::Respawn,
                elapsed_ticks: 0,
                buttons_active: false,
                hardcore: false,
            })
        );

        assert!(input.handle_death_screen_key(&mut counters, &commands, KeyCode::Enter, true));
        assert!(rx.try_recv().is_err());
        assert!(input.death_screen().is_some());

        input.advance_death_screen_ticks(19);
        input.handle_death_screen_mouse_input(
            &mut counters,
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
            .death_screen_status()
            .is_some_and(|status| status.elapsed_ticks == 20 && status.buttons_active));
        input.handle_death_screen_mouse_input(
            &mut counters,
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
        assert_eq!(input.death_screen(), None);
        assert_eq!(input.death_screen_runtime(), None);
        assert_eq!(input.death_screen_status(), None);
    }

    #[test]
    fn escape_never_closes_main_screen_and_delayed_quit_escape_respawns() {
        let mut world = death_world(false, RichTextComponent::plain("You died"));
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        input.apply_screen_requests(&mut world, &mut counters, &commands);

        input.handle_death_screen_key(&mut counters, &commands, KeyCode::Escape, true);
        assert!(input.death_screen().is_some());
        input.advance_death_screen_ticks(20);
        input.handle_death_screen_key(&mut counters, &commands, KeyCode::Tab, true);
        input.handle_death_screen_key(&mut counters, &commands, KeyCode::Enter, true);
        assert!(matches!(
            input.death_screen_runtime(),
            Some(DeathScreenRuntime::QuitConfirmation(_))
        ));

        input.handle_death_screen_key(&mut counters, &commands, KeyCode::Escape, true);
        assert!(rx.try_recv().is_err());
        input.advance_death_screen_ticks(20);
        input.handle_death_screen_key(&mut counters, &commands, KeyCode::Escape, true);
        assert_eq!(rx.try_recv(), Ok(NetCommand::PerformRespawn));
        assert!(rx.try_recv().is_err());
        assert_eq!(input.death_screen(), None);
    }

    #[test]
    fn hardcore_title_button_disconnects_once() {
        let mut world = death_world(true, RichTextComponent::plain("Hardcore death"));
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        input.apply_screen_requests(&mut world, &mut counters, &commands);
        input.advance_death_screen_ticks(20);
        input.handle_death_screen_key(&mut counters, &commands, KeyCode::Tab, true);
        input.handle_death_screen_key(&mut counters, &commands, KeyCode::Enter, true);

        assert_eq!(rx.try_recv(), Ok(NetCommand::Disconnect));
        assert!(rx.try_recv().is_err());
        assert_eq!(input.death_screen(), None);
    }

    #[test]
    fn cause_link_is_generation_checked_and_returns_to_same_death_screen() {
        let mut world = death_world(false, link_cause());
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        input.apply_screen_requests(&mut world, &mut counters, &None);
        let generation = input.death_screen().unwrap().screen.generation;

        input.handle_death_screen_mouse_input(
            &mut counters,
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
        let projected =
            input.dialog_control_projection(serde_json::to_value(world.client_ui()).unwrap());
        assert_eq!(
            projected["dialog_link_confirmation"]["url"],
            "https://example.invalid/death"
        );
        assert!(projected["current_dialog"].is_null());

        input.handle_death_screen_key(&mut counters, &None, KeyCode::Tab, true);
        input.handle_death_screen_key(&mut counters, &None, KeyCode::Enter, true);
        assert_eq!(
            input.take_death_screen_platform_requests(),
            vec![DeathScreenPlatformRequest::ClipboardWrite(
                "https://example.invalid/death".into()
            )]
        );
        assert!(matches!(
            input.death_screen_runtime(),
            Some(DeathScreenRuntime::Main(state)) if state.generation == generation
        ));
        assert!(input.death_screen().is_some());
        let projected =
            input.dialog_control_projection(serde_json::to_value(world.client_ui()).unwrap());
        assert!(projected.get("dialog_link_confirmation").is_none());

        input.handle_death_screen_mouse_input(
            &mut counters,
            &None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(hit(
                generation,
                HudDeathScreenHitTargetKind::CauseLink { run_index: 0 },
            )),
        );
        input.close_screens(&mut counters, &None);
        assert_eq!(input.death_screen_runtime(), None);
        let projected =
            input.dialog_control_projection(serde_json::to_value(world.client_ui()).unwrap());
        assert!(projected.get("dialog_link_confirmation").is_none());
    }

    #[test]
    fn cause_link_confirmation_stays_a_page_of_the_death_screen_across_frames() {
        let mut world = death_world(false, link_cause());
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        input.apply_screen_requests(&mut world, &mut counters, &None);
        let generation = input.death_screen().unwrap().screen.generation;
        input.handle_death_screen_mouse_input(
            &mut counters,
            &None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(hit(
                generation,
                HudDeathScreenHitTargetKind::CauseLink { run_index: 0 },
            )),
        );
        // The runtime's frame tail (`run_frame`).
        let frame_tail = |input: &mut ClientInputState, world: &mut WorldStore| {
            input.apply_screen_requests(world, &mut NetCounters::default(), &None);
        };
        frame_tail(&mut input, &mut world);
        frame_tail(&mut input, &mut world);
        assert_eq!(
            input.death_screen_status().map(|status| status.screen),
            Some(crate::control::DeathScreenControlKind::LinkConfirmation)
        );

        input.handle_death_screen_key(&mut counters, &None, KeyCode::Escape, true);
        frame_tail(&mut input, &mut world);
        assert_eq!(
            input.death_screen_status().map(|status| status.screen),
            Some(crate::control::DeathScreenControlKind::Main)
        );
    }

    #[test]
    fn respawn_under_a_dialog_drops_the_parked_death_screen() {
        let mut input = ClientInputState::new(true);
        let mut world = death_world(false, RichTextComponent::plain("You died"));
        let mut counters = NetCounters::default();
        input.apply_screen_requests(&mut world, &mut counters, &None);
        world.apply_show_dialog(bbb_protocol::packets::ShowDialog {
            dialog: bbb_protocol::packets::DialogHolder::Reference { registry_id: 1 },
        });
        input.apply_screen_requests(&mut world, &mut counters, &None);
        assert!(input.death_screen().is_none());

        world.apply_respawn(&bbb_protocol::packets::Respawn {
            common_spawn_info: overworld_spawn_info(),
            data_to_keep: 0,
        });
        input.apply_screen_requests(&mut world, &mut counters, &None);
        world.apply_clear_dialog();
        input.apply_screen_requests(&mut world, &mut counters, &None);
        assert!(input.death_screen().is_none());
    }
}
