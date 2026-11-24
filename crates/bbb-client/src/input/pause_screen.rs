use crate::control::NetCounters;
use bbb_client_net::NetCommand;
use bbb_client_render::{
    hud_pause_advancements_button_rect, hud_pause_disconnect_button_rect,
    hud_pause_feedback_back_button_rect, hud_pause_feedback_report_bugs_button_rect,
    hud_pause_feedback_send_feedback_button_rect, hud_pause_options_button_rect,
    hud_pause_report_bugs_button_rect, hud_pause_return_to_game_button_rect,
    hud_pause_send_feedback_button_rect, hud_pause_stats_button_rect,
};
use bbb_client_world::{PauseScreenCustomDialogState, WorldStore};
use bbb_protocol::MC_STABLE;
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton},
    keyboard::{KeyCode, PhysicalKey},
};

use super::{physical_position_floor_i32, ClientInputState, Screen};

const RELEASE_FEEDBACK_URL: &str = "https://aka.ms/javafeedback?ref=game";
const SNAPSHOT_FEEDBACK_URL: &str = "https://aka.ms/snapshotfeedback?ref=game";
const SNAPSHOT_BUGS_URL: &str = "https://aka.ms/snapshotbugs?ref=game";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum PauseScreenPage {
    #[default]
    Main,
    Feedback,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DebugPauseScreenState {
    pub(crate) show_pause_menu: bool,
    pub(crate) page: PauseScreenPage,
    pub(crate) cursor_position: Option<(i32, i32)>,
    pub(crate) custom_dialog: Option<PauseScreenCustomDialogState>,
    pub(crate) disconnect_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PauseScreenLinkRequest {
    SendFeedback { url: &'static str },
    ReportBugs { url: &'static str },
}

impl ClientInputState {
    /// Vanilla-shape constructor for `PauseScreen(showPauseMenu = false)`.
    /// Reachable in vanilla only when an unpublished singleplayer server can
    /// really pause (F3+Esc); the external client never opens it in
    /// production, so this exists only to pin the no-menu render contract.
    #[cfg(test)]
    pub(crate) fn open_debug_pause_screen_without_menu_for_test(&mut self) {
        self.set_screen_for_test(Screen::Pause(DebugPauseScreenState {
            show_pause_menu: false,
            page: PauseScreenPage::Main,
            cursor_position: None,
            custom_dialog: None,
            disconnect_requested: false,
        }));
    }

    pub(crate) fn open_debug_pause_screen_with_menu(
        &mut self,
        world: &WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        let screen = Screen::Pause(DebugPauseScreenState {
            show_pause_menu: true,
            page: PauseScreenPage::Main,
            cursor_position: None,
            custom_dialog: world.pause_screen_custom_dialog(),
            disconnect_requested: false,
        });
        self.set_screen(screen, counters, net_commands);
    }

    pub(crate) fn close_debug_pause_screen(&mut self) {
        self.screen
            .close_if(|screen| matches!(screen, Screen::Pause(_)));
    }

    pub(crate) fn debug_pause_screen(&self) -> Option<&DebugPauseScreenState> {
        self.screen.pause()
    }

    #[cfg(test)]
    pub(crate) fn debug_pause_screen_is_open(&self) -> bool {
        self.screen.pause().is_some()
    }

    pub(crate) fn handle_debug_pause_screen_key(
        &mut self,
        _world: &mut WorldStore,
        physical_key: PhysicalKey,
        state: ElementState,
    ) -> bool {
        if self.screen.pause().is_none() {
            return false;
        }
        let PhysicalKey::Code(code) = physical_key else {
            return true;
        };
        let pressed = matches!(state, ElementState::Pressed);
        if matches!(code, KeyCode::ShiftLeft | KeyCode::ShiftRight) {
            self.set_shift_key(code, pressed);
        }
        if matches!(code, KeyCode::ControlLeft | KeyCode::ControlRight) {
            self.set_control_key(code, pressed);
        }
        if matches!(state, ElementState::Pressed) && matches!(code, KeyCode::Escape) {
            self.set_key_down(code, false);
            match self.screen.pause().map(|screen| screen.page) {
                Some(PauseScreenPage::Feedback) => {
                    if let Some(screen) = self.screen.pause_mut() {
                        screen.page = PauseScreenPage::Main;
                    }
                }
                _ => self.close_debug_pause_screen(),
            }
            return true;
        }
        if matches!(code, KeyCode::F3) || self.debug_modifier_down {
            return false;
        }
        self.set_key_down(code, false);
        true
    }

    pub(crate) fn handle_debug_pause_screen_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
    ) -> bool {
        let Some(screen) = self.screen.pause_mut() else {
            return false;
        };
        screen.cursor_position = cursor_position.and_then(physical_position_floor_i32);
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_debug_pause_screen_mouse_input(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some((show_pause_menu, page, cursor_position, custom_dialog)) =
            self.screen.pause_mut().map(|screen| {
                screen.cursor_position = cursor_position.and_then(physical_position_floor_i32);
                (
                    screen.show_pause_menu,
                    screen.page,
                    screen.cursor_position,
                    screen.custom_dialog.clone(),
                )
            })
        else {
            return false;
        };
        if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            return true;
        }
        if !show_pause_menu {
            return true;
        }
        if page == PauseScreenPage::Feedback {
            if pause_screen_feedback_send_feedback_button_contains(cursor_position, surface_size) {
                self.pause_screen_link_requests
                    .push(PauseScreenLinkRequest::SendFeedback {
                        url: pause_screen_feedback_url(),
                    });
            } else if pause_screen_feedback_report_bugs_button_contains(
                cursor_position,
                surface_size,
            ) && pause_screen_report_bugs_button_enabled()
            {
                self.pause_screen_link_requests
                    .push(PauseScreenLinkRequest::ReportBugs {
                        url: SNAPSHOT_BUGS_URL,
                    });
            } else if pause_screen_feedback_back_button_contains(cursor_position, surface_size) {
                if let Some(screen) = self.screen.pause_mut() {
                    screen.page = PauseScreenPage::Main;
                }
            }
            return true;
        }

        if pause_screen_return_to_game_button_contains(cursor_position, surface_size) {
            self.close_debug_pause_screen();
        } else if pause_screen_advancements_button_contains(cursor_position, surface_size) {
            self.open_advancements_screen(counters, world, net_commands, true);
        } else if pause_screen_stats_button_contains(cursor_position, surface_size) {
            self.open_stats_screen(counters, world, net_commands);
        } else if pause_screen_send_feedback_button_contains(cursor_position, surface_size) {
            if custom_dialog.is_some() {
                if let Some(screen) = self.screen.pause_mut() {
                    screen.page = PauseScreenPage::Feedback;
                }
            } else {
                self.pause_screen_link_requests
                    .push(PauseScreenLinkRequest::SendFeedback {
                        url: pause_screen_feedback_url(),
                    });
            }
        } else if pause_screen_report_bugs_button_contains(cursor_position, surface_size) {
            if let Some(custom_dialog) = custom_dialog.as_ref() {
                self.show_dialog(custom_dialog.dialog.clone());
            } else if pause_screen_report_bugs_button_enabled() {
                self.pause_screen_link_requests
                    .push(PauseScreenLinkRequest::ReportBugs {
                        url: SNAPSHOT_BUGS_URL,
                    });
            }
        } else if pause_screen_disconnect_button_contains(cursor_position, surface_size) {
            if let Some(screen) = self.screen.pause_mut() {
                screen.disconnect_requested = true;
            }
        }
        true
    }

    pub(crate) fn pause_screen_disconnect_requested(&self) -> bool {
        self.screen
            .pause()
            .is_some_and(|screen| screen.disconnect_requested)
    }

    pub(crate) fn take_pause_screen_link_requests(&mut self) -> Vec<PauseScreenLinkRequest> {
        std::mem::take(&mut self.pause_screen_link_requests)
    }
}

/// Vanilla pauses only an unpublished integrated server. The native runtime is
/// an external client and owns no integrated server, so neither PauseScreen
/// variant is eligible to freeze raw client or running level ticks.
pub(crate) fn native_pause_freezes_client_ticks(
    _input: &ClientInputState,
    _world: &WorldStore,
) -> bool {
    false
}

pub(crate) fn pause_screen_return_to_game_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        hud_pause_return_to_game_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_advancements_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        hud_pause_advancements_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_stats_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(cursor_position, hud_pause_stats_button_rect(surface_size))
}

pub(crate) fn pause_screen_send_feedback_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        hud_pause_send_feedback_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_report_bugs_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        hud_pause_report_bugs_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_report_bugs_button_enabled() -> bool {
    bbb_protocol::MC_DATA_VERSION_SERIES == "main"
}

pub(crate) fn pause_screen_options_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(cursor_position, hud_pause_options_button_rect(surface_size))
}

pub(crate) fn pause_screen_options_button_enabled() -> bool {
    false
}

pub(crate) fn pause_screen_disconnect_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        hud_pause_disconnect_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_feedback_send_feedback_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        hud_pause_feedback_send_feedback_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_feedback_report_bugs_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        hud_pause_feedback_report_bugs_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_feedback_back_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        hud_pause_feedback_back_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_feedback_url() -> &'static str {
    if MC_STABLE {
        RELEASE_FEEDBACK_URL
    } else {
        SNAPSHOT_FEEDBACK_URL
    }
}

fn rect_contains(cursor_position: Option<(i32, i32)>, rect: (i32, i32, i32, i32)) -> bool {
    let Some((mouse_x, mouse_y)) = cursor_position else {
        return false;
    };
    let (x, y, width, height) = rect;
    mouse_x >= x && mouse_x < x + width && mouse_y >= y && mouse_y < y + height
}

#[cfg(test)]
mod tests {
    use bbb_client_world::RegistryPacketEntry;
    use bbb_protocol::packets::{RegistryTags, TagNetworkPayload, UpdateTags};

    use super::*;

    fn open_pause_and_click_disconnect(
        input: &mut ClientInputState,
        world: &mut WorldStore,
        counters: &mut NetCounters,
    ) {
        let surface = PhysicalSize::new(320, 240);
        let (x, y, width, height) = bbb_client_render::hud_pause_disconnect_button_rect(surface);
        let position =
            winit::dpi::PhysicalPosition::new(f64::from(x + width / 2), f64::from(y + height / 2));
        input.open_debug_pause_screen_with_menu(world, &mut Default::default(), &None);
        assert!(input.handle_debug_pause_screen_mouse_input(
            counters,
            world,
            &None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(position),
            surface,
        ));
    }

    #[test]
    fn disconnect_without_draft_requests_disconnect_directly() {
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();
        let mut input = ClientInputState::new(true);

        open_pause_and_click_disconnect(&mut input, &mut world, &mut counters);
        assert!(input.pause_screen_disconnect_requested());
        assert_eq!(
            input.debug_pause_screen().map(|screen| screen.page),
            Some(PauseScreenPage::Main)
        );
    }

    fn network_notice_dialog(title: &str, external_title: &str) -> Vec<u8> {
        fn write_string_tag(payload: &mut Vec<u8>, name: &str, value: &str) {
            payload.push(8);
            payload.extend_from_slice(&(name.len() as u16).to_be_bytes());
            payload.extend_from_slice(name.as_bytes());
            payload.extend_from_slice(&(value.len() as u16).to_be_bytes());
            payload.extend_from_slice(value.as_bytes());
        }

        let mut payload = vec![10];
        write_string_tag(&mut payload, "type", "minecraft:notice");
        write_string_tag(&mut payload, "title", title);
        write_string_tag(&mut payload, "external_title", external_title);
        payload.push(0);
        payload
    }

    fn custom_pause_dialog_world() -> WorldStore {
        let mut world = WorldStore::new();
        world.record_registry_entries(
            "minecraft:dialog",
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "example:custom",
                network_notice_dialog("Internal", "External"),
            )],
        );
        apply_pause_dialog_tag(&mut world, vec![0]);
        world
    }

    fn apply_pause_dialog_tag(world: &mut WorldStore, entries: Vec<i32>) {
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:dialog".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:pause_screen_additions".to_string(),
                    entries,
                }],
            }],
        });
    }

    #[test]
    fn pause_screen_custom_dialog_snapshot_opens_and_restores_exact_parent() {
        let mut world = custom_pause_dialog_world();
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let commands = None;
        let surface = PhysicalSize::new(320, 240);

        input.open_debug_pause_screen_with_menu(&world, &mut Default::default(), &None);
        let expected = input.debug_pause_screen().expect("pause screen");
        assert_eq!(
            expected
                .custom_dialog
                .as_ref()
                .map(PauseScreenCustomDialogState::resource_id),
            Some("example:custom")
        );

        apply_pause_dialog_tag(&mut world, Vec::new());
        assert_eq!(world.pause_screen_custom_dialog(), None);
        assert!(input.handle_debug_pause_screen_mouse_input(
            &mut counters,
            &mut world,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(170.0, 126.0)),
            surface,
        ));
        assert_eq!(
            input
                .current_dialog()
                .and_then(|dialog| dialog.resource_id.as_deref()),
            Some("example:custom")
        );
        let Some(Screen::Pause(expected_restored)) = input.screen.parent() else {
            panic!("dialog parent");
        };
        let expected_restored = expected_restored.clone();

        input.apply_screen_requests(&mut world, &mut counters, &commands);
        assert!(!input.debug_pause_screen_is_open());
        assert!(input.close_dialog());
        input.apply_screen_requests(&mut world, &mut counters, &commands);
        assert_eq!(input.debug_pause_screen(), Some(&expected_restored));
    }

    #[test]
    fn pause_screen_feedback_subscreen_navigates_and_queues_exact_links() {
        let mut world = custom_pause_dialog_world();
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let commands = None;
        let surface = PhysicalSize::new(320, 240);

        input.open_debug_pause_screen_with_menu(&world, &mut Default::default(), &None);
        assert!(input.handle_debug_pause_screen_mouse_input(
            &mut counters,
            &mut world,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(68.0, 126.0)),
            surface,
        ));
        assert_eq!(
            input.debug_pause_screen().map(|screen| screen.page),
            Some(PauseScreenPage::Feedback)
        );

        assert!(input.handle_debug_pause_screen_key(
            &mut world,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
        ));
        assert_eq!(
            input.debug_pause_screen().map(|screen| screen.page),
            Some(PauseScreenPage::Main)
        );

        assert!(input.handle_debug_pause_screen_mouse_input(
            &mut counters,
            &mut world,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(68.0, 126.0)),
            surface,
        ));
        assert!(input.handle_debug_pause_screen_mouse_input(
            &mut counters,
            &mut world,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(68.0, 68.0)),
            surface,
        ));
        assert_eq!(
            input.take_pause_screen_link_requests(),
            vec![PauseScreenLinkRequest::SendFeedback {
                url: pause_screen_feedback_url(),
            }]
        );

        assert!(input.handle_debug_pause_screen_mouse_input(
            &mut counters,
            &mut world,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(170.0, 68.0)),
            surface,
        ));
        assert_eq!(
            input.take_pause_screen_link_requests(),
            vec![PauseScreenLinkRequest::ReportBugs {
                url: SNAPSHOT_BUGS_URL,
            }]
        );

        assert!(input.handle_debug_pause_screen_mouse_input(
            &mut counters,
            &mut world,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(68.0, 220.0)),
            surface,
        ));
        assert_eq!(
            input.debug_pause_screen().map(|screen| screen.page),
            Some(PauseScreenPage::Main)
        );
    }
}
