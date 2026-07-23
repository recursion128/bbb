use std::{
    collections::HashSet,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_renderer::{HudGameEventButton, HudGameEventHitTarget};
use bbb_world::{ClientGameEventScreenState, WorldStore};
use tokio::sync::mpsc;
use winit::keyboard::KeyCode;

use super::{queue_perform_respawn_command, queue_sign_editor_removed, ClientInputState};

pub(crate) const BUY_MINECRAFT_JAVA_URL: &str = "https://aka.ms/BuyMinecraftJava";

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GameEventScreenRuntime {
    Win(WinScreenInputState),
    DemoHelp(DemoHelpInputState),
    DemoLinkConfirmation(DemoLinkConfirmationInputState),
}

impl GameEventScreenRuntime {
    pub(crate) fn generation(&self) -> u64 {
        match self {
            Self::Win(state) => state.generation,
            Self::DemoHelp(state) => state.generation,
            Self::DemoLinkConfirmation(state) => state.generation,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WinScreenInputState {
    pub(crate) generation: u64,
    pub(crate) scroll: f32,
    pub(crate) scroll_speed: f32,
    pub(crate) show_easter_egg_logo: bool,
    direction: i8,
    speedup_active: bool,
    speedup_modifiers: HashSet<KeyCode>,
}

impl WinScreenInputState {
    fn new(generation: u64, show_easter_egg_logo: bool) -> Self {
        Self {
            generation,
            scroll: 0.0,
            scroll_speed: 0.5,
            show_easter_egg_logo,
            direction: 1,
            speedup_active: false,
            speedup_modifiers: HashSet::new(),
        }
    }

    fn handle_key(&mut self, code: KeyCode, pressed: bool) {
        match (code, pressed) {
            (KeyCode::ArrowUp, true) => self.direction = -1,
            (KeyCode::ArrowUp, false) => self.direction = 1,
            (KeyCode::ControlLeft | KeyCode::ControlRight, true) => {
                self.speedup_modifiers.insert(code);
            }
            (KeyCode::ControlLeft | KeyCode::ControlRight, false) => {
                self.speedup_modifiers.remove(&code);
            }
            (KeyCode::Space, true) => self.speedup_active = true,
            (KeyCode::Space, false) => self.speedup_active = false,
            _ => {}
        }
        let direction = f32::from(self.direction);
        self.scroll_speed = if self.speedup_active {
            0.5 * (5.0 + self.speedup_modifiers.len() as f32 * 15.0) * direction
        } else {
            0.5 * direction
        };
    }

    fn advance(&mut self, delta_ticks: f32) {
        if !delta_ticks.is_finite() || delta_ticks <= 0.0 {
            return;
        }
        self.scroll = (self.scroll + delta_ticks * self.scroll_speed).max(0.0);
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum DemoHelpFocus {
    #[default]
    Buy,
    Later,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DemoHelpInputState {
    pub(crate) generation: u64,
    pub(crate) focus: DemoHelpFocus,
    pub(crate) hovered: Option<DemoHelpFocus>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum DemoLinkConfirmationFocus {
    #[default]
    Open,
    Copy,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DemoLinkConfirmationInputState {
    pub(crate) generation: u64,
    pub(crate) focus: DemoLinkConfirmationFocus,
    pub(crate) hovered: Option<DemoLinkConfirmationFocus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GameEventScreenPlatformRequest {
    OpenUri(String),
    ClipboardWrite(String),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct GameEventScreenInputState {
    active: Option<GameEventScreenRuntime>,
    platform_requests: Vec<GameEventScreenPlatformRequest>,
}

impl GameEventScreenInputState {
    pub(crate) fn active(&self) -> Option<&GameEventScreenRuntime> {
        self.active.as_ref()
    }

    fn synchronize_with_logo_sample(
        &mut self,
        screen: Option<ClientGameEventScreenState>,
        show_easter_egg_logo: bool,
    ) -> bool {
        if screen
            .zip(self.active.as_ref())
            .is_some_and(|(screen, active)| screen.generation() == active.generation())
        {
            return false;
        }
        self.active = screen.map(|screen| match screen {
            ClientGameEventScreenState::Win { generation } => GameEventScreenRuntime::Win(
                WinScreenInputState::new(generation, show_easter_egg_logo),
            ),
            ClientGameEventScreenState::DemoHelp { generation } => {
                GameEventScreenRuntime::DemoHelp(DemoHelpInputState {
                    generation,
                    focus: DemoHelpFocus::Buy,
                    hovered: None,
                })
            }
        });
        true
    }

    fn take_platform_requests(&mut self) -> Vec<GameEventScreenPlatformRequest> {
        std::mem::take(&mut self.platform_requests)
    }
}

impl ClientInputState {
    pub(crate) fn game_event_screen_runtime(&self) -> Option<&GameEventScreenRuntime> {
        self.game_event_screen.active()
    }

    pub(crate) fn sync_game_event_screen(
        &mut self,
        world: &WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) -> bool {
        let screen = world.game_event_screen();
        let sample_logo = matches!(screen, Some(ClientGameEventScreenState::Win { .. }))
            && screen
                .zip(self.game_event_screen.active.as_ref())
                .is_none_or(|(screen, active)| screen.generation() != active.generation());
        let changed = self
            .game_event_screen
            .synchronize_with_logo_sample(screen, sample_logo && sample_easter_egg_logo());
        if !changed || screen.is_none() {
            return changed;
        }

        self.debug_options_screen = None;
        self.debug_pause_screen = None;
        self.debug_game_mode_switcher = None;
        self.pause_screen_disconnect_requested = false;
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

    #[cfg(test)]
    pub(crate) fn sync_game_event_screen_with_logo_sample(
        &mut self,
        world: &WorldStore,
        show_easter_egg_logo: bool,
    ) -> bool {
        self.game_event_screen
            .synchronize_with_logo_sample(world.game_event_screen(), show_easter_egg_logo)
    }

    pub(crate) fn handle_game_event_screen_key(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        code: KeyCode,
        pressed: bool,
        just_pressed: bool,
    ) -> bool {
        let reverse_tab = self.shift_down();
        let Some(active) = self.game_event_screen.active.as_mut() else {
            return false;
        };
        let generation = active.generation();
        match active {
            GameEventScreenRuntime::Win(win) => {
                win.handle_key(code, pressed);
                if just_pressed && code == KeyCode::Escape {
                    complete_game_event_screen(
                        &mut self.game_event_screen,
                        counters,
                        world,
                        net_commands,
                        generation,
                    );
                }
            }
            GameEventScreenRuntime::DemoHelp(demo) => {
                if !just_pressed {
                    return true;
                }
                match code {
                    KeyCode::Escape => {
                        close_game_event_screen(&mut self.game_event_screen, world, generation);
                    }
                    KeyCode::Tab | KeyCode::ArrowLeft | KeyCode::ArrowRight => {
                        demo.focus = match demo.focus {
                            DemoHelpFocus::Buy => DemoHelpFocus::Later,
                            DemoHelpFocus::Later => DemoHelpFocus::Buy,
                        };
                    }
                    KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => match demo.focus {
                        DemoHelpFocus::Buy => {
                            self.game_event_screen.active =
                                Some(GameEventScreenRuntime::DemoLinkConfirmation(
                                    DemoLinkConfirmationInputState {
                                        generation,
                                        focus: DemoLinkConfirmationFocus::Open,
                                        hovered: None,
                                    },
                                ));
                        }
                        DemoHelpFocus::Later => {
                            close_game_event_screen(&mut self.game_event_screen, world, generation);
                        }
                    },
                    _ => {}
                }
            }
            GameEventScreenRuntime::DemoLinkConfirmation(confirmation) => {
                if !just_pressed {
                    return true;
                }
                match code {
                    KeyCode::Escape => {
                        close_game_event_screen(&mut self.game_event_screen, world, generation);
                    }
                    KeyCode::Tab if reverse_tab => {
                        confirmation.focus = match confirmation.focus {
                            DemoLinkConfirmationFocus::Open => DemoLinkConfirmationFocus::Cancel,
                            DemoLinkConfirmationFocus::Copy => DemoLinkConfirmationFocus::Open,
                            DemoLinkConfirmationFocus::Cancel => DemoLinkConfirmationFocus::Copy,
                        };
                    }
                    KeyCode::Tab | KeyCode::ArrowRight => {
                        confirmation.focus = match confirmation.focus {
                            DemoLinkConfirmationFocus::Open => DemoLinkConfirmationFocus::Copy,
                            DemoLinkConfirmationFocus::Copy => DemoLinkConfirmationFocus::Cancel,
                            DemoLinkConfirmationFocus::Cancel => DemoLinkConfirmationFocus::Open,
                        };
                    }
                    KeyCode::ArrowLeft => {
                        confirmation.focus = match confirmation.focus {
                            DemoLinkConfirmationFocus::Open => DemoLinkConfirmationFocus::Cancel,
                            DemoLinkConfirmationFocus::Copy => DemoLinkConfirmationFocus::Open,
                            DemoLinkConfirmationFocus::Cancel => DemoLinkConfirmationFocus::Copy,
                        };
                    }
                    KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                        match confirmation.focus {
                            DemoLinkConfirmationFocus::Open => self
                                .game_event_screen
                                .platform_requests
                                .push(GameEventScreenPlatformRequest::OpenUri(
                                    BUY_MINECRAFT_JAVA_URL.to_string(),
                                )),
                            DemoLinkConfirmationFocus::Copy => self
                                .game_event_screen
                                .platform_requests
                                .push(GameEventScreenPlatformRequest::ClipboardWrite(
                                    BUY_MINECRAFT_JAVA_URL.to_string(),
                                )),
                            DemoLinkConfirmationFocus::Cancel => {}
                        }
                        close_game_event_screen(&mut self.game_event_screen, world, generation);
                    }
                    _ => {}
                }
            }
        }
        true
    }

    pub(crate) fn advance_win_screen(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        delta_ticks: f32,
        total_scroll_length: i32,
        screen_height: i32,
    ) -> bool {
        let Some(GameEventScreenRuntime::Win(win)) = self.game_event_screen.active.as_mut() else {
            return false;
        };
        win.advance(delta_ticks);
        let generation = win.generation;
        let max_scroll = total_scroll_length
            .saturating_add(screen_height.saturating_mul(2))
            .saturating_add(24) as f32;
        if win.scroll > max_scroll {
            complete_game_event_screen(
                &mut self.game_event_screen,
                counters,
                world,
                net_commands,
                generation,
            );
        }
        true
    }

    pub(crate) fn advance_win_screen_for_generation(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        generation: u64,
        delta_ticks: f32,
        total_scroll_length: i32,
        screen_height: i32,
    ) -> bool {
        if self
            .game_event_screen
            .active()
            .map(GameEventScreenRuntime::generation)
            != Some(generation)
        {
            return false;
        }
        self.advance_win_screen(
            counters,
            world,
            net_commands,
            delta_ticks,
            total_scroll_length,
            screen_height,
        )
    }

    pub(crate) fn handle_game_event_screen_hover(
        &mut self,
        target: Option<HudGameEventHitTarget>,
    ) -> bool {
        let Some(active) = self.game_event_screen.active.as_mut() else {
            return false;
        };
        let target = target.filter(|target| target.generation == active.generation());
        match active {
            GameEventScreenRuntime::Win(_) => {}
            GameEventScreenRuntime::DemoHelp(state) => {
                state.hovered = target.and_then(|target| match target.button {
                    HudGameEventButton::DemoBuy => Some(DemoHelpFocus::Buy),
                    HudGameEventButton::DemoLater => Some(DemoHelpFocus::Later),
                    _ => None,
                });
            }
            GameEventScreenRuntime::DemoLinkConfirmation(state) => {
                state.hovered = target.and_then(|target| match target.button {
                    HudGameEventButton::LinkOpen => Some(DemoLinkConfirmationFocus::Open),
                    HudGameEventButton::LinkCopy => Some(DemoLinkConfirmationFocus::Copy),
                    HudGameEventButton::LinkCancel => Some(DemoLinkConfirmationFocus::Cancel),
                    _ => None,
                });
            }
        }
        true
    }

    pub(crate) fn handle_game_event_screen_mouse_input(
        &mut self,
        world: &mut WorldStore,
        button: winit::event::MouseButton,
        state: winit::event::ElementState,
        target: Option<HudGameEventHitTarget>,
    ) -> bool {
        if button != winit::event::MouseButton::Left || state != winit::event::ElementState::Pressed
        {
            return self.game_event_screen.active.is_some();
        }
        let Some(active) = self.game_event_screen.active.as_ref() else {
            return false;
        };
        let generation = active.generation();
        let Some(target) = target.filter(|target| target.generation == generation) else {
            return true;
        };
        match target.button {
            HudGameEventButton::DemoBuy
                if matches!(active, GameEventScreenRuntime::DemoHelp(_)) =>
            {
                self.game_event_screen.active = Some(GameEventScreenRuntime::DemoLinkConfirmation(
                    DemoLinkConfirmationInputState {
                        generation,
                        focus: DemoLinkConfirmationFocus::Open,
                        hovered: None,
                    },
                ));
            }
            HudGameEventButton::DemoLater
                if matches!(active, GameEventScreenRuntime::DemoHelp(_)) =>
            {
                close_game_event_screen(&mut self.game_event_screen, world, generation);
            }
            HudGameEventButton::LinkOpen
                if matches!(active, GameEventScreenRuntime::DemoLinkConfirmation(_)) =>
            {
                self.game_event_screen.platform_requests.push(
                    GameEventScreenPlatformRequest::OpenUri(BUY_MINECRAFT_JAVA_URL.to_string()),
                );
                close_game_event_screen(&mut self.game_event_screen, world, generation);
            }
            HudGameEventButton::LinkCopy
                if matches!(active, GameEventScreenRuntime::DemoLinkConfirmation(_)) =>
            {
                self.game_event_screen.platform_requests.push(
                    GameEventScreenPlatformRequest::ClipboardWrite(
                        BUY_MINECRAFT_JAVA_URL.to_string(),
                    ),
                );
                close_game_event_screen(&mut self.game_event_screen, world, generation);
            }
            HudGameEventButton::LinkCancel
                if matches!(active, GameEventScreenRuntime::DemoLinkConfirmation(_)) =>
            {
                close_game_event_screen(&mut self.game_event_screen, world, generation);
            }
            _ => {}
        }
        true
    }

    pub(crate) fn take_game_event_screen_platform_requests(
        &mut self,
    ) -> Vec<GameEventScreenPlatformRequest> {
        self.game_event_screen.take_platform_requests()
    }
}

fn close_game_event_screen(
    state: &mut GameEventScreenInputState,
    world: &mut WorldStore,
    generation: u64,
) {
    world.close_game_event_screen(generation);
    state.active = None;
}

fn complete_game_event_screen(
    state: &mut GameEventScreenInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    generation: u64,
) {
    if !world.close_game_event_screen(generation) {
        state.active = None;
        return;
    }
    queue_perform_respawn_command(counters, net_commands);
    state.active = None;
}

fn sample_easter_egg_logo() -> bool {
    static SAMPLE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let sequence = SAMPLE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let epoch_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let mut sample = epoch_nanos ^ sequence.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    sample ^= sample >> 30;
    sample = sample.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    sample ^= sample >> 27;
    sample = sample.wrapping_mul(0x94D0_49BB_1331_11EB);
    sample ^= sample >> 31;
    sample % 10_000 == 0
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        game_event_types::{
            VANILLA_CLIENT_GAME_EVENT_DEMO_EVENT_ID, VANILLA_CLIENT_GAME_EVENT_WIN_GAME_ID,
        },
        packets::GameEvent,
    };
    use tokio::sync::mpsc;

    use super::*;

    fn open_event(world: &mut WorldStore, event_id: u8) {
        world.apply_game_event(GameEvent {
            event_id,
            param: 0.0,
        });
    }

    #[test]
    fn win_controls_match_vanilla_speed_and_complete_once() {
        let mut world = WorldStore::new();
        open_event(&mut world, VANILLA_CLIENT_GAME_EVENT_WIN_GAME_ID);
        let mut input = ClientInputState::new(true);
        assert!(input.sync_game_event_screen_with_logo_sample(&world, true));
        let Some(GameEventScreenRuntime::Win(win)) = input.game_event_screen_runtime() else {
            panic!("win runtime");
        };
        assert!(win.show_easter_egg_logo);
        assert_eq!(win.scroll_speed, 0.5);

        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(4);
        let net_commands = Some(tx);
        assert!(input.handle_game_event_screen_key(
            &mut counters,
            &mut world,
            &net_commands,
            KeyCode::ControlLeft,
            true,
            true,
        ));
        assert!(input.handle_game_event_screen_key(
            &mut counters,
            &mut world,
            &net_commands,
            KeyCode::Space,
            true,
            true,
        ));
        let Some(GameEventScreenRuntime::Win(win)) = input.game_event_screen_runtime() else {
            panic!("win runtime");
        };
        assert_eq!(win.scroll_speed, 10.0);
        input.advance_win_screen(&mut counters, &mut world, &net_commands, 10.0, 12, 20);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::PerformRespawn);
        assert!(rx.try_recv().is_err());
        assert_eq!(world.game_event_screen(), None);
        assert_eq!(input.game_event_screen_runtime(), None);
    }

    #[test]
    fn up_reverses_only_while_held_and_scroll_clamps_at_zero() {
        let mut world = WorldStore::new();
        open_event(&mut world, VANILLA_CLIENT_GAME_EVENT_WIN_GAME_ID);
        let mut input = ClientInputState::new(true);
        input.sync_game_event_screen_with_logo_sample(&world, false);
        let mut counters = NetCounters::default();
        let net_commands = None;

        input.handle_game_event_screen_key(
            &mut counters,
            &mut world,
            &net_commands,
            KeyCode::ArrowUp,
            true,
            true,
        );
        input.advance_win_screen(&mut counters, &mut world, &net_commands, 4.0, 100, 100);
        let Some(GameEventScreenRuntime::Win(win)) = input.game_event_screen_runtime() else {
            panic!("win runtime");
        };
        assert_eq!(win.scroll, 0.0);
        assert_eq!(win.scroll_speed, -0.5);

        input.handle_game_event_screen_key(
            &mut counters,
            &mut world,
            &net_commands,
            KeyCode::ArrowUp,
            false,
            false,
        );
        let Some(GameEventScreenRuntime::Win(win)) = input.game_event_screen_runtime() else {
            panic!("win runtime");
        };
        assert_eq!(win.scroll_speed, 0.5);
    }

    #[test]
    fn demo_buttons_and_trusted_confirmation_close_to_gameplay() {
        let mut world = WorldStore::new();
        open_event(&mut world, VANILLA_CLIENT_GAME_EVENT_DEMO_EVENT_ID);
        let mut input = ClientInputState::new(true);
        input.sync_game_event_screen_with_logo_sample(&world, false);
        let mut counters = NetCounters::default();
        let net_commands = None;

        input.handle_game_event_screen_key(
            &mut counters,
            &mut world,
            &net_commands,
            KeyCode::Enter,
            true,
            true,
        );
        assert!(matches!(
            input.game_event_screen_runtime(),
            Some(GameEventScreenRuntime::DemoLinkConfirmation(_))
        ));
        input.handle_game_event_screen_key(
            &mut counters,
            &mut world,
            &net_commands,
            KeyCode::Enter,
            true,
            true,
        );
        assert_eq!(
            input.take_game_event_screen_platform_requests(),
            vec![GameEventScreenPlatformRequest::OpenUri(
                BUY_MINECRAFT_JAVA_URL.to_string()
            )]
        );
        assert_eq!(world.game_event_screen(), None);

        open_event(&mut world, VANILLA_CLIENT_GAME_EVENT_DEMO_EVENT_ID);
        input.sync_game_event_screen_with_logo_sample(&world, false);
        input.handle_game_event_screen_key(
            &mut counters,
            &mut world,
            &net_commands,
            KeyCode::Tab,
            true,
            true,
        );
        input.handle_game_event_screen_key(
            &mut counters,
            &mut world,
            &net_commands,
            KeyCode::Enter,
            true,
            true,
        );
        assert_eq!(world.game_event_screen(), None);
        assert!(input.take_game_event_screen_platform_requests().is_empty());
    }

    #[test]
    fn demo_pointer_targets_are_generation_bound_and_share_keyboard_actions() {
        let mut world = WorldStore::new();
        open_event(&mut world, VANILLA_CLIENT_GAME_EVENT_DEMO_EVENT_ID);
        let mut input = ClientInputState::new(true);
        input.sync_game_event_screen_with_logo_sample(&world, false);
        let generation = world.game_event_screen().unwrap().generation();

        assert!(
            input.handle_game_event_screen_hover(Some(HudGameEventHitTarget {
                generation,
                button: HudGameEventButton::DemoLater,
            }))
        );
        let Some(GameEventScreenRuntime::DemoHelp(screen)) = input.game_event_screen_runtime()
        else {
            panic!("demo help");
        };
        assert_eq!(screen.hovered, Some(DemoHelpFocus::Later));

        input.handle_game_event_screen_mouse_input(
            &mut world,
            winit::event::MouseButton::Left,
            winit::event::ElementState::Pressed,
            Some(HudGameEventHitTarget {
                generation: generation.saturating_add(1),
                button: HudGameEventButton::DemoLater,
            }),
        );
        assert!(world.game_event_screen().is_some());
        input.handle_game_event_screen_mouse_input(
            &mut world,
            winit::event::MouseButton::Left,
            winit::event::ElementState::Pressed,
            Some(HudGameEventHitTarget {
                generation,
                button: HudGameEventButton::DemoLater,
            }),
        );
        assert_eq!(world.game_event_screen(), None);
    }

    #[test]
    fn replacement_resets_runtime_without_respawn() {
        let mut world = WorldStore::new();
        open_event(&mut world, VANILLA_CLIENT_GAME_EVENT_WIN_GAME_ID);
        let mut input = ClientInputState::new(true);
        input.sync_game_event_screen_with_logo_sample(&world, false);
        let (tx, mut rx) = mpsc::channel(1);
        let mut counters = NetCounters::default();

        open_event(&mut world, VANILLA_CLIENT_GAME_EVENT_DEMO_EVENT_ID);
        assert!(input.sync_game_event_screen(&world, &mut counters, &Some(tx)));

        assert!(matches!(
            input.game_event_screen_runtime(),
            Some(GameEventScreenRuntime::DemoHelp(_))
        ));
        assert!(rx.try_recv().is_err());
    }
}
