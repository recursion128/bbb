//! `AdvancementsScreen`: root tabs, drag and wheel scrolling, the Done button.

use std::collections::BTreeMap;

use bbb_client_net::NetCommand;
use bbb_client_world::WorldStore;
use bbb_protocol::packets::SeenAdvancements;
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::KeyCode,
};

use super::{
    physical_position_floor_i32, queue_seen_advancements_command, release_active_input,
    ClientInputState, Screen,
};
use crate::{
    control::NetCounters,
    runtime::{
        advancements_done_button_origin_for_surface, advancements_window_origin_for_surface,
    },
};

const ADVANCEMENTS_DONE_BUTTON_WIDTH: i32 = 200;
const ADVANCEMENTS_DONE_BUTTON_HEIGHT: i32 = 20;
const ADVANCEMENTS_TAB_ABOVE_WIDTH: u32 = 28;
const ADVANCEMENTS_TAB_ABOVE_HEIGHT: u32 = 32;
const ADVANCEMENTS_TAB_ABOVE_MAX: usize = 8;
const ADVANCEMENTS_TAB_BELOW_WIDTH: u32 = 28;
const ADVANCEMENTS_TAB_BELOW_HEIGHT: u32 = 32;
const ADVANCEMENTS_TAB_BELOW_MAX: usize = 8;
const ADVANCEMENTS_TAB_LEFT_WIDTH: u32 = 32;
const ADVANCEMENTS_TAB_LEFT_HEIGHT: u32 = 28;
const ADVANCEMENTS_TAB_LEFT_MAX: usize = 5;
const ADVANCEMENTS_TAB_RIGHT_WIDTH: u32 = 32;
const ADVANCEMENTS_TAB_RIGHT_HEIGHT: u32 = 28;
const ADVANCEMENTS_TAB_RIGHT_MAX: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdvancementTabType {
    Above,
    Below,
    Left,
    Right,
}

impl AdvancementTabType {
    pub(crate) fn for_display_index(mut display_index: usize) -> Option<(Self, usize)> {
        if display_index < ADVANCEMENTS_TAB_ABOVE_MAX {
            return Some((Self::Above, display_index));
        }
        display_index -= ADVANCEMENTS_TAB_ABOVE_MAX;
        if display_index < ADVANCEMENTS_TAB_BELOW_MAX {
            return Some((Self::Below, display_index));
        }
        display_index -= ADVANCEMENTS_TAB_BELOW_MAX;
        if display_index < ADVANCEMENTS_TAB_LEFT_MAX {
            return Some((Self::Left, display_index));
        }
        display_index -= ADVANCEMENTS_TAB_LEFT_MAX;
        (display_index < ADVANCEMENTS_TAB_RIGHT_MAX).then_some((Self::Right, display_index))
    }

    pub(crate) fn max(self) -> usize {
        match self {
            Self::Above => ADVANCEMENTS_TAB_ABOVE_MAX,
            Self::Below => ADVANCEMENTS_TAB_BELOW_MAX,
            Self::Left => ADVANCEMENTS_TAB_LEFT_MAX,
            Self::Right => ADVANCEMENTS_TAB_RIGHT_MAX,
        }
    }

    pub(crate) fn width(self) -> u32 {
        match self {
            Self::Above => ADVANCEMENTS_TAB_ABOVE_WIDTH,
            Self::Below => ADVANCEMENTS_TAB_BELOW_WIDTH,
            Self::Left => ADVANCEMENTS_TAB_LEFT_WIDTH,
            Self::Right => ADVANCEMENTS_TAB_RIGHT_WIDTH,
        }
    }

    pub(crate) fn height(self) -> u32 {
        match self {
            Self::Above => ADVANCEMENTS_TAB_ABOVE_HEIGHT,
            Self::Below => ADVANCEMENTS_TAB_BELOW_HEIGHT,
            Self::Left => ADVANCEMENTS_TAB_LEFT_HEIGHT,
            Self::Right => ADVANCEMENTS_TAB_RIGHT_HEIGHT,
        }
    }

    pub(crate) fn origin(self, index: usize) -> (i32, i32) {
        match self {
            Self::Above => (
                ((self.width() + 4) * index as u32) as i32,
                -(self.height() as i32) + 4,
            ),
            Self::Below => (((self.width() + 4) * index as u32) as i32, 136),
            Self::Left => (
                -(self.width() as i32) + 4,
                (self.height() * index as u32) as i32,
            ),
            Self::Right => (248, (self.height() * index as u32) as i32),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct AdvancementsScreen {
    /// Per root tab `AdvancementTab.scrollX/scrollY` moved by dragging and the wheel.
    scroll_deltas: BTreeMap<String, (f64, f64)>,
    /// `AdvancementTab.fade`.
    hover_fade: f32,
    pub(super) mouse_left_down: bool,
    pub(super) is_scrolling: bool,
}

impl AdvancementsScreen {
    pub(crate) fn scroll_delta(&self, selected_tab: Option<&str>) -> Option<(f64, f64)> {
        selected_tab.and_then(|tab| self.scroll_deltas.get(tab).copied())
    }

    pub(crate) fn update_hover_fade(&mut self, hovering: bool) {
        if hovering {
            self.hover_fade = (self.hover_fade + 0.02).clamp(0.0, 0.3);
        } else {
            self.hover_fade = (self.hover_fade - 0.04).clamp(0.0, 1.0);
        }
    }

    pub(crate) fn hover_fade(&self) -> f32 {
        self.hover_fade
    }

    pub(super) fn clear_pressed(&mut self) {
        self.hover_fade = 0.0;
        self.mouse_left_down = false;
        self.is_scrolling = false;
    }

    fn scroll_by(&mut self, world: &WorldStore, dx: f64, dy: f64) {
        if let Some(tab) = world.selected_advancements_tab() {
            let entry = self.scroll_deltas.entry(tab.to_string()).or_default();
            entry.0 += dx;
            entry.1 += dy;
        }
    }
    pub(crate) fn done_button_contains(
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some(cursor) = cursor_position else {
            return false;
        };
        let (x, y) = advancements_done_button_origin_for_surface(surface_size);
        let (x, y) = (f64::from(x), f64::from(y));
        cursor.x >= x
            && cursor.x < x + f64::from(ADVANCEMENTS_DONE_BUTTON_WIDTH)
            && cursor.y >= y
            && cursor.y < y + f64::from(ADVANCEMENTS_DONE_BUTTON_HEIGHT)
    }
}

impl ClientInputState {
    pub(crate) fn advancements_screen(&self) -> Option<&AdvancementsScreen> {
        self.screen.advancements()
    }

    pub(crate) fn advancements_screen_mut(&mut self) -> Option<&mut AdvancementsScreen> {
        self.screen.advancements_mut()
    }

    pub(crate) fn advancements_screen_is_open(&self) -> bool {
        self.screen.advancements().is_some()
    }

    /// `key.advancements` opens `AdvancementsScreen(advancements)`; the pause
    /// screen's button passes itself as the screen to return to.
    pub(crate) fn open_advancements_screen(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        from_pause: bool,
    ) {
        if self.advancements_screen_is_open() {
            return;
        }
        self.interrupt_chat_screen();
        release_active_input(self, world, counters, net_commands);
        world.replace_screen();
        let screen = Screen::Advancements(AdvancementsScreen::default());
        if from_pause {
            self.screen.open_child(screen);
        } else {
            self.set_screen(screen, counters, net_commands);
        }
        if let Some(tab) = world.ensure_advancements_screen_selected_tab() {
            queue_seen_advancements_command(
                counters,
                net_commands,
                SeenAdvancements::OpenedTab { tab },
            );
        }
    }

    /// `onClose` returns to `lastScreen`; `key.advancements` closes to the game.
    fn close_advancements_screen(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        to_game: bool,
    ) {
        if self
            .screen
            .close_if(|screen| matches!(screen, Screen::Advancements(_)))
            .is_none()
        {
            return;
        }
        if to_game {
            self.close_screens(counters, net_commands);
        }
        release_active_input(self, world, counters, net_commands);
        queue_seen_advancements_command(counters, net_commands, SeenAdvancements::ClosedScreen);
    }

    pub(crate) fn handle_advancements_screen_key(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        code: KeyCode,
        pressed: bool,
    ) -> bool {
        if !self.advancements_screen_is_open() {
            return false;
        }
        if pressed && matches!(code, KeyCode::Escape | KeyCode::KeyL) {
            let to_game = code == KeyCode::KeyL;
            self.close_advancements_screen(counters, world, net_commands, to_game);
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_advancements_screen_mouse_input(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some(screen) = self.screen.advancements_mut() else {
            return false;
        };
        if matches!(state, ElementState::Released) {
            screen.is_scrolling = false;
            if matches!(button, MouseButton::Left) {
                screen.mouse_left_down = false;
            }
        } else if matches!(button, MouseButton::Left) {
            screen.mouse_left_down = true;
            screen.is_scrolling = false;
        }
        self.inventory_cursor_position = cursor_position.and_then(physical_position_floor_i32);
        if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            return true;
        }
        if let Some(tab) = advancements_tab_at_position(world, cursor_position, surface_size) {
            if let Some(tab) = world.select_advancements_root_tab(&tab) {
                queue_seen_advancements_command(
                    counters,
                    net_commands,
                    SeenAdvancements::OpenedTab { tab },
                );
            }
        } else if AdvancementsScreen::done_button_contains(cursor_position, surface_size) {
            self.close_advancements_screen(counters, world, net_commands, false);
        }
        true
    }

    pub(crate) fn handle_advancements_screen_cursor_moved(
        &mut self,
        world: &WorldStore,
        previous_position: Option<PhysicalPosition<f64>>,
        cursor_position: Option<PhysicalPosition<f64>>,
    ) -> bool {
        if !self.advancements_screen_is_open() {
            return false;
        }
        self.inventory_cursor_position = cursor_position.and_then(physical_position_floor_i32);
        let Some(screen) = self.screen.advancements_mut() else {
            return true;
        };
        if !screen.mouse_left_down {
            return true;
        }
        let (Some(previous), Some(current)) = (previous_position, cursor_position) else {
            return true;
        };
        if !screen.is_scrolling {
            screen.is_scrolling = true;
            return true;
        }
        screen.scroll_by(world, current.x - previous.x, current.y - previous.y);
        true
    }

    pub(crate) fn handle_advancements_screen_mouse_wheel(
        &mut self,
        world: &WorldStore,
        delta: MouseScrollDelta,
    ) -> bool {
        if !self.advancements_screen_is_open() {
            return false;
        }
        if world.selected_advancements_tab().is_none() {
            return true;
        }
        let Some((wheel_x, wheel_y)) = advancement_wheel_steps_from_scroll(self, delta) else {
            return true;
        };
        if let Some(screen) = self.screen.advancements_mut() {
            screen.scroll_by(world, f64::from(wheel_x) * 16.0, f64::from(wheel_y) * 16.0);
        }
        true
    }
}

fn advancement_wheel_steps_from_scroll(
    input: &mut ClientInputState,
    delta: MouseScrollDelta,
) -> Option<(i32, i32)> {
    let (x, y) = crate::input::scroll_delta_lines(delta);
    if input.scroll_accumulated_x != 0.0
        && advancement_scroll_signum(x) != advancement_scroll_signum(input.scroll_accumulated_x)
    {
        input.scroll_accumulated_x = 0.0;
    }
    if input.scroll_accumulated_y != 0.0
        && advancement_scroll_signum(y) != advancement_scroll_signum(input.scroll_accumulated_y)
    {
        input.scroll_accumulated_y = 0.0;
    }

    input.scroll_accumulated_x += x;
    input.scroll_accumulated_y += y;
    let wheel_x = input.scroll_accumulated_x as i32;
    let wheel_y = input.scroll_accumulated_y as i32;
    if wheel_x == 0 && wheel_y == 0 {
        return None;
    }

    input.scroll_accumulated_x -= f64::from(wheel_x);
    input.scroll_accumulated_y -= f64::from(wheel_y);
    Some((wheel_x, wheel_y))
}

fn advancement_scroll_signum(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

fn advancements_tab_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<String> {
    let cursor = cursor_position?;
    let tabs = world.advancement_root_tabs();
    if tabs.len() <= 1 {
        return None;
    }
    let (window_x, window_y) = advancements_window_origin_for_surface(surface_size);
    tabs.into_iter().find_map(|tab| {
        let (x, y, width, height) =
            advancements_tab_bounds(tab.display_index, f64::from(window_x), f64::from(window_y))?;
        (cursor.x > x && cursor.x < x + width && cursor.y > y && cursor.y < y + height)
            .then_some(tab.id)
    })
}

fn advancements_tab_bounds(
    display_index: usize,
    window_x: f64,
    window_y: f64,
) -> Option<(f64, f64, f64, f64)> {
    let (tab_type, index) = AdvancementTabType::for_display_index(display_index)?;
    let (x, y) = tab_type.origin(index);
    Some((
        window_x + f64::from(x),
        window_y + f64::from(y),
        f64::from(tab_type.width()),
        f64::from(tab_type.height()),
    ))
}
