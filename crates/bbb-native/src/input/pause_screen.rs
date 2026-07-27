use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::MC_STABLE;
use bbb_world::{PauseScreenCustomDialogState, WorldStore};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton},
    keyboard::{KeyCode, PhysicalKey},
};

use super::{
    open_advancements_screen, open_social_interactions_screen, open_stats_screen,
    physical_position_floor_i32, ClientInputState,
};

const RETURN_TO_GAME_BUTTON_WIDTH: i32 = 204;
const HALF_BUTTON_WIDTH: i32 = 98;
const BUTTON_HEIGHT: i32 = 20;
const RETURN_TO_GAME_TOP_OFFSET: i32 = 8;
const SECOND_ROW_TOP_OFFSET: i32 = 32;
const THIRD_ROW_TOP_OFFSET: i32 = 56;
const FOURTH_ROW_TOP_OFFSET: i32 = 80;
const DISCONNECT_ROW_TOP_OFFSET: i32 = 128;
const FEEDBACK_HEADER_HEIGHT: i32 = 33;
const FEEDBACK_FOOTER_HEIGHT: i32 = 33;
const FEEDBACK_CONTENT_MARGIN_TOP: i32 = 30;
const FEEDBACK_CELL_PADDING: i32 = 4;
const FEEDBACK_BACK_BUTTON_WIDTH: i32 = 200;

const RELEASE_FEEDBACK_URL: &str = "https://aka.ms/javafeedback?ref=game";
const SNAPSHOT_FEEDBACK_URL: &str = "https://aka.ms/snapshotfeedback?ref=game";
const SNAPSHOT_BUGS_URL: &str = "https://aka.ms/snapshotbugs?ref=game";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum PauseScreenPage {
    #[default]
    Main,
    Feedback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DebugPauseScreenState {
    pub(crate) show_pause_menu: bool,
    pub(crate) page: PauseScreenPage,
    pub(crate) cursor_position: Option<(i32, i32)>,
    pub(crate) custom_dialog: Option<PauseScreenCustomDialogState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PauseScreenLinkRequest {
    SendFeedback { url: &'static str },
    ReportBugs { url: &'static str },
}

impl ClientInputState {
    pub(crate) fn open_debug_pause_screen_without_menu(&mut self) {
        self.debug_options_screen = None;
        self.pause_screen_disconnect_requested = false;
        self.debug_pause_screen = Some(DebugPauseScreenState {
            show_pause_menu: false,
            page: PauseScreenPage::Main,
            cursor_position: None,
            custom_dialog: None,
        });
    }

    pub(crate) fn open_debug_pause_screen_with_menu(&mut self, world: &WorldStore) {
        self.debug_options_screen = None;
        self.pause_screen_disconnect_requested = false;
        self.debug_pause_screen = Some(DebugPauseScreenState {
            show_pause_menu: true,
            page: PauseScreenPage::Main,
            cursor_position: None,
            custom_dialog: world.pause_screen_custom_dialog(),
        });
    }

    pub(crate) fn close_debug_pause_screen(&mut self) {
        self.debug_pause_screen = None;
        self.pause_screen_disconnect_requested = false;
    }

    pub(crate) fn debug_pause_screen(&self) -> Option<&DebugPauseScreenState> {
        self.debug_pause_screen.as_ref()
    }

    pub(crate) fn debug_pause_screen_is_open(&self) -> bool {
        self.debug_pause_screen.is_some()
    }

    pub(crate) fn handle_debug_pause_screen_key(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
    ) -> bool {
        if self.debug_pause_screen.is_none() {
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
            let feedback = self
                .debug_pause_screen
                .as_ref()
                .is_some_and(|screen| screen.page == PauseScreenPage::Feedback);
            if feedback {
                if let Some(screen) = self.debug_pause_screen.as_mut() {
                    screen.page = PauseScreenPage::Main;
                }
            } else {
                self.close_debug_pause_screen();
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
        let Some(screen) = self.debug_pause_screen.as_mut() else {
            return false;
        };
        screen.cursor_position = cursor_position.and_then(physical_position_floor_i32);
        true
    }

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
            self.debug_pause_screen.as_mut().map(|screen| {
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
                if let Some(screen) = self.debug_pause_screen.as_mut() {
                    screen.page = PauseScreenPage::Main;
                }
            }
            return true;
        }

        if pause_screen_return_to_game_button_contains(cursor_position, surface_size) {
            self.close_debug_pause_screen();
        } else if pause_screen_advancements_button_contains(cursor_position, surface_size) {
            self.close_debug_pause_screen();
            open_advancements_screen(self, counters, world, net_commands);
        } else if pause_screen_stats_button_contains(cursor_position, surface_size) {
            self.close_debug_pause_screen();
            open_stats_screen(self, counters, world, net_commands);
        } else if pause_screen_player_reporting_button_contains(cursor_position, surface_size)
            && pause_screen_player_reporting_button_enabled()
        {
            open_social_interactions_screen(self, counters, world, net_commands, true);
        } else if pause_screen_send_feedback_button_contains(cursor_position, surface_size) {
            if custom_dialog.is_some() {
                if let Some(screen) = self.debug_pause_screen.as_mut() {
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
                world.open_pause_screen_custom_dialog(custom_dialog);
            } else if pause_screen_report_bugs_button_enabled() {
                self.pause_screen_link_requests
                    .push(PauseScreenLinkRequest::ReportBugs {
                        url: SNAPSHOT_BUGS_URL,
                    });
            }
        } else if pause_screen_disconnect_button_contains(cursor_position, surface_size) {
            self.pause_screen_disconnect_requested = true;
        }
        true
    }

    pub(crate) fn pause_screen_disconnect_requested(&self) -> bool {
        self.pause_screen_disconnect_requested
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
        pause_screen_return_to_game_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_advancements_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        pause_screen_advancements_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_stats_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        pause_screen_stats_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_send_feedback_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        pause_screen_send_feedback_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_report_bugs_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        pause_screen_report_bugs_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_report_bugs_button_enabled() -> bool {
    bbb_protocol::MC_DATA_VERSION_SERIES == "main"
}

pub(crate) fn pause_screen_options_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        pause_screen_options_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_options_button_enabled() -> bool {
    false
}

pub(crate) fn pause_screen_player_reporting_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        pause_screen_player_reporting_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_player_reporting_button_enabled() -> bool {
    true
}

pub(crate) fn pause_screen_disconnect_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        pause_screen_disconnect_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_feedback_send_feedback_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        pause_screen_feedback_send_feedback_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_feedback_report_bugs_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        pause_screen_feedback_report_bugs_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_feedback_back_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    rect_contains(
        cursor_position,
        pause_screen_feedback_back_button_rect(surface_size),
    )
}

pub(crate) fn pause_screen_feedback_url() -> &'static str {
    if MC_STABLE {
        RELEASE_FEEDBACK_URL
    } else {
        SNAPSHOT_FEEDBACK_URL
    }
}

fn pause_screen_return_to_game_button_rect(
    surface_size: PhysicalSize<u32>,
) -> (i32, i32, i32, i32) {
    let (width, height) = surface_dimensions(surface_size);
    (
        width / 2 - RETURN_TO_GAME_BUTTON_WIDTH / 2,
        height / 4 + RETURN_TO_GAME_TOP_OFFSET,
        RETURN_TO_GAME_BUTTON_WIDTH,
        BUTTON_HEIGHT,
    )
}

fn pause_screen_advancements_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let (width, height) = surface_dimensions(surface_size);
    (
        width / 2 - RETURN_TO_GAME_BUTTON_WIDTH / 2,
        height / 4 + SECOND_ROW_TOP_OFFSET,
        HALF_BUTTON_WIDTH,
        BUTTON_HEIGHT,
    )
}

fn pause_screen_stats_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let (width, height) = surface_dimensions(surface_size);
    (
        width / 2 + 4,
        height / 4 + SECOND_ROW_TOP_OFFSET,
        HALF_BUTTON_WIDTH,
        BUTTON_HEIGHT,
    )
}

fn pause_screen_send_feedback_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let (width, height) = surface_dimensions(surface_size);
    (
        width / 2 - RETURN_TO_GAME_BUTTON_WIDTH / 2,
        height / 4 + THIRD_ROW_TOP_OFFSET,
        HALF_BUTTON_WIDTH,
        BUTTON_HEIGHT,
    )
}

fn pause_screen_report_bugs_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let (width, height) = surface_dimensions(surface_size);
    (
        width / 2 + 4,
        height / 4 + THIRD_ROW_TOP_OFFSET,
        HALF_BUTTON_WIDTH,
        BUTTON_HEIGHT,
    )
}

fn pause_screen_options_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let (width, height) = surface_dimensions(surface_size);
    (
        width / 2 - RETURN_TO_GAME_BUTTON_WIDTH / 2,
        height / 4 + FOURTH_ROW_TOP_OFFSET,
        HALF_BUTTON_WIDTH,
        BUTTON_HEIGHT,
    )
}

fn pause_screen_player_reporting_button_rect(
    surface_size: PhysicalSize<u32>,
) -> (i32, i32, i32, i32) {
    let (width, height) = surface_dimensions(surface_size);
    (
        width / 2 + 4,
        height / 4 + FOURTH_ROW_TOP_OFFSET,
        HALF_BUTTON_WIDTH,
        BUTTON_HEIGHT,
    )
}

fn pause_screen_disconnect_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let (width, height) = surface_dimensions(surface_size);
    (
        width / 2 - RETURN_TO_GAME_BUTTON_WIDTH / 2,
        height / 4 + DISCONNECT_ROW_TOP_OFFSET,
        RETURN_TO_GAME_BUTTON_WIDTH,
        BUTTON_HEIGHT,
    )
}

fn pause_screen_feedback_send_feedback_button_rect(
    surface_size: PhysicalSize<u32>,
) -> (i32, i32, i32, i32) {
    let (width, _) = surface_dimensions(surface_size);
    (
        width / 2 - RETURN_TO_GAME_BUTTON_WIDTH / 2,
        pause_screen_feedback_button_y(surface_size),
        HALF_BUTTON_WIDTH,
        BUTTON_HEIGHT,
    )
}

fn pause_screen_feedback_report_bugs_button_rect(
    surface_size: PhysicalSize<u32>,
) -> (i32, i32, i32, i32) {
    let (width, _) = surface_dimensions(surface_size);
    (
        width / 2 + 4,
        pause_screen_feedback_button_y(surface_size),
        HALF_BUTTON_WIDTH,
        BUTTON_HEIGHT,
    )
}

fn pause_screen_feedback_back_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let (width, height) = surface_dimensions(surface_size);
    (
        width / 2 - FEEDBACK_BACK_BUTTON_WIDTH / 2,
        height - FEEDBACK_FOOTER_HEIGHT + (FEEDBACK_FOOTER_HEIGHT - BUTTON_HEIGHT) / 2,
        FEEDBACK_BACK_BUTTON_WIDTH,
        BUTTON_HEIGHT,
    )
}

fn pause_screen_feedback_button_y(surface_size: PhysicalSize<u32>) -> i32 {
    let (_, height) = surface_dimensions(surface_size);
    let grid_height = BUTTON_HEIGHT + FEEDBACK_CELL_PADDING;
    (FEEDBACK_HEADER_HEIGHT + FEEDBACK_CONTENT_MARGIN_TOP)
        .min(height - FEEDBACK_FOOTER_HEIGHT - grid_height)
        + FEEDBACK_CELL_PADDING
}

fn surface_dimensions(surface_size: PhysicalSize<u32>) -> (i32, i32) {
    (
        i32::try_from(surface_size.width).unwrap_or(i32::MAX),
        i32::try_from(surface_size.height).unwrap_or(i32::MAX),
    )
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
    use bbb_protocol::packets::{RegistryTags, TagNetworkPayload, UpdateTags};
    use bbb_world::RegistryPacketEntry;

    use super::*;

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

        input.open_debug_pause_screen_with_menu(&world);
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
            world
                .current_dialog()
                .and_then(|dialog| dialog.resource_id.as_deref()),
            Some("example:custom")
        );
        let expected_restored = input.debug_pause_screen().cloned().expect("dialog parent");

        input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);
        assert!(!input.debug_pause_screen_is_open());
        assert!(world.close_dialog());
        input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);
        assert_eq!(input.debug_pause_screen(), Some(&expected_restored));
    }

    #[test]
    fn pause_screen_feedback_subscreen_navigates_and_queues_exact_links() {
        let mut world = custom_pause_dialog_world();
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let commands = None;
        let surface = PhysicalSize::new(320, 240);

        input.open_debug_pause_screen_with_menu(&world);
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
