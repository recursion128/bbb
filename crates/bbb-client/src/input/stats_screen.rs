//! `StatsScreen`: tabs, the sortable item list and their focus path.

use bbb_client_net::NetCommand;
use bbb_client_render::{
    hud_stats_done_button_rect, hud_stats_tab_rects, HUD_STATS_LIST_TOP as STATS_SCREEN_LIST_TOP,
};
use bbb_client_world::WorldStore;
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::KeyCode,
};

use super::{
    physical_position_floor_i32, queue_request_stats_command, release_active_input,
    ClientInputState, Screen,
};
use crate::control::NetCounters;

const STATS_SCREEN_FOOTER_HEIGHT: i32 = 33;
const STATS_SCREEN_ROW_HEIGHT: i32 = 14;
const STATS_SCREEN_LIST_WIDTH: i32 = 280;
const STATS_SCREEN_LIST_ENTRY_PADDING: i32 = 2;
const STATS_SCREEN_SCROLLBAR_WIDTH: i32 = 6;
const STATS_SCREEN_ITEM_COLUMN_FIRST_X: i32 = 57;
const STATS_SCREEN_ITEM_COLUMN_STRIDE: i32 = 40;
const STATS_SCREEN_ITEM_COLUMN_BUTTON_SIZE: i32 = 18;

/// Focus path owned by the native StatsScreen interaction model. This mirrors the
/// concrete vanilla widgets without leaking localized labels into input state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum StatsScreenFocus {
    #[default]
    None,
    Tabs,
    List,
    GeneralRow(usize),
    ItemHeader(usize),
    ItemRow(usize),
    MobRow(usize),
    Done,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StatsScreen {
    /// `StatsScreen.isLoading`: true until the first `AwardStats` arrives.
    pub(crate) loading: bool,
    pub(crate) cursor_position: Option<(i32, i32)>,
    pub(crate) scroll_amount: f64,
    max_scroll_amount: f64,
    scroller_height: i32,
    scrolling: bool,
    pub(crate) tab: usize,
    item_sort_column: Option<usize>,
    item_sort_order: i8,
    pub(crate) item_effective_sort: Option<(usize, i8)>,
    tab_row_counts: [usize; 3],
    total_rows: usize,
    pub(crate) focus: StatsScreenFocus,
    pub(super) was_loading: bool,
}

impl Default for StatsScreen {
    fn default() -> Self {
        Self {
            loading: true,
            cursor_position: None,
            scroll_amount: 0.0,
            max_scroll_amount: 0.0,
            scroller_height: 0,
            scrolling: false,
            tab: 0,
            item_sort_column: None,
            item_sort_order: 0,
            item_effective_sort: None,
            tab_row_counts: [0; 3],
            total_rows: 0,
            focus: StatsScreenFocus::None,
            was_loading: true,
        }
    }
}

impl StatsScreen {
    pub(crate) fn sync_scroll_metrics(&mut self, max_scroll_amount: i32, scroller_height: i32) {
        self.max_scroll_amount = f64::from(max_scroll_amount.max(0));
        self.scroller_height = scroller_height.max(0);
        self.scroll_amount = self.scroll_amount.clamp(0.0, self.max_scroll_amount);
        if self.max_scroll_amount == 0.0 {
            self.scrolling = false;
        }
    }

    pub(crate) fn reset_scroll(&mut self) {
        self.scroll_amount = 0.0;
        self.scrolling = false;
    }

    pub(crate) fn keep_focus_visible(&mut self, viewport_height: i32, row_height: i32) {
        self.scroll_amount = self.focused_scroll_amount(
            viewport_height,
            row_height,
            self.max_scroll_amount as i32,
            self.scroll_amount,
        );
    }

    pub(crate) fn focused_scroll_amount(
        &self,
        viewport_height: i32,
        row_height: i32,
        max_scroll_amount: i32,
        scroll_amount: f64,
    ) -> f64 {
        let scroll_amount = scroll_amount.clamp(0.0, f64::from(max_scroll_amount));
        let entry = match self.focus {
            StatsScreenFocus::GeneralRow(row) | StatsScreenFocus::MobRow(row) => row,
            StatsScreenFocus::ItemHeader(_) => 0,
            StatsScreenFocus::ItemRow(row) => row.saturating_add(1),
            _ => return scroll_amount,
        };
        let entry = i32::try_from(entry).unwrap_or(i32::MAX / row_height.max(1));
        let top = STATS_SCREEN_LIST_ENTRY_PADDING.saturating_add(entry.saturating_mul(row_height));
        let bottom = top.saturating_add(row_height);
        let scroll = scroll_amount as i32;
        if top < scroll {
            f64::from(top.max(0))
        } else if bottom > scroll.saturating_add(viewport_height) {
            f64::from(
                bottom
                    .saturating_sub(viewport_height)
                    .clamp(0, max_scroll_amount),
            )
        } else {
            scroll_amount
        }
    }

    pub(crate) fn item_sort(&self) -> (Option<usize>, i8) {
        (self.item_sort_column, self.item_sort_order)
    }

    /// Synchronizes the concrete focus path after the runtime has projected the
    /// active tab's rows. Vanilla replaces the loading tabs with real widgets on
    /// the first stats response and explicitly focuses the tab navigation bar.
    pub(crate) fn sync_interaction(
        &mut self,
        is_loading: bool,
        active_tab: usize,
        total_rows: usize,
    ) {
        if active_tab < self.tab_row_counts.len() {
            self.tab_row_counts[active_tab] = total_rows;
        }
        self.total_rows = total_rows;
        if is_loading {
            self.was_loading = true;
            self.focus = StatsScreenFocus::None;
            return;
        }
        if self.was_loading {
            self.was_loading = false;
            self.focus = StatsScreenFocus::Tabs;
        }
        self.focus = match self.focus {
            StatsScreenFocus::None => StatsScreenFocus::None,
            StatsScreenFocus::GeneralRow(row) if active_tab == 0 && row < total_rows => {
                StatsScreenFocus::GeneralRow(row)
            }
            StatsScreenFocus::ItemHeader(column) if active_tab == 1 && column < 6 => {
                StatsScreenFocus::ItemHeader(column)
            }
            StatsScreenFocus::ItemRow(row) if active_tab == 1 && row < total_rows => {
                StatsScreenFocus::ItemRow(row)
            }
            StatsScreenFocus::MobRow(row) if active_tab == 2 && row < total_rows => {
                StatsScreenFocus::MobRow(row)
            }
            StatsScreenFocus::List | StatsScreenFocus::Tabs | StatsScreenFocus::Done => self.focus,
            _ => StatsScreenFocus::List,
        };
    }

    pub(crate) fn sync_tab_row_counts(&mut self, row_counts: [usize; 3]) {
        self.tab_row_counts = row_counts;
        self.total_rows = row_counts[self.tab.min(2)];
    }

    pub(crate) fn item_hovered_column(
        &self,
        surface_size: PhysicalSize<u32>,
        scroll_amount: i32,
    ) -> Option<usize> {
        (self.tab == 1)
            .then(|| {
                stats_screen_item_column_at_position(
                    self.cursor_position,
                    surface_size,
                    scroll_amount,
                )
            })
            .flatten()
    }

    pub(crate) fn hovered_tab(&self, surface_size: PhysicalSize<u32>) -> Option<usize> {
        stats_screen_tab_at_position(self.cursor_position, surface_size)
    }

    pub(crate) fn hovered_item_row(
        &self,
        surface_size: PhysicalSize<u32>,
        scroll_amount: i32,
    ) -> Option<usize> {
        if self.tab != 1 {
            return None;
        }
        let (mouse_x, mouse_y) = self.cursor_position?;
        let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
        let list_x = width / 2 - STATS_SCREEN_LIST_WIDTH / 2;
        let list_bottom = i32::try_from(surface_size.height)
            .unwrap_or(i32::MAX)
            .saturating_sub(STATS_SCREEN_FOOTER_HEIGHT);
        if !(list_x..list_x + STATS_SCREEN_ITEM_COLUMN_BUTTON_SIZE).contains(&mouse_x)
            || !(STATS_SCREEN_LIST_TOP..list_bottom).contains(&mouse_y)
        {
            return None;
        }
        let content_y =
            mouse_y - STATS_SCREEN_LIST_TOP - STATS_SCREEN_LIST_ENTRY_PADDING + scroll_amount;
        if content_y < 0 {
            return None;
        }
        let entry = content_y / 22;
        (entry > 0 && content_y % 22 < STATS_SCREEN_ITEM_COLUMN_BUTTON_SIZE)
            .then(|| usize::try_from(entry - 1).ok())
            .flatten()
    }

    pub(crate) fn hovered_row(&self, surface_size: PhysicalSize<u32>) -> Option<usize> {
        if self.tab == 1 {
            return self.hovered_item_row(surface_size, self.scroll_amount as i32);
        }
        stats_screen_row_at_position(
            self.cursor_position,
            surface_size,
            self.tab,
            self.scroll_amount as i32,
        )
        .filter(|row| *row < self.total_rows)
    }

    fn mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let row_height = stats_screen_row_height(self.tab);
        let delta_pixels = match delta {
            MouseScrollDelta::LineDelta(_, y) => f64::from(y) * f64::from((row_height / 2).max(1)),
            MouseScrollDelta::PixelDelta(position) => position.y,
        };
        self.scroll_amount = (self.scroll_amount - delta_pixels).clamp(0.0, self.max_scroll_amount);
    }

    fn cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) {
        let previous = self.cursor_position;
        self.cursor_position = cursor_position.and_then(physical_position_floor_i32);
        if !self.scrolling {
            return;
        }
        let Some((_, mouse_y)) = self.cursor_position else {
            return;
        };
        let list_bottom = i32::try_from(surface_size.height)
            .unwrap_or(i32::MAX)
            .saturating_sub(STATS_SCREEN_FOOTER_HEIGHT);
        if mouse_y < STATS_SCREEN_LIST_TOP {
            self.scroll_amount = 0.0;
        } else if mouse_y > list_bottom {
            self.scroll_amount = self.max_scroll_amount;
        } else if let Some((_, previous_y)) = previous {
            let viewport_height = (list_bottom - STATS_SCREEN_LIST_TOP).max(1);
            let thumb_travel = (viewport_height - self.scroller_height).max(1);
            let drag_scale = (self.max_scroll_amount / f64::from(thumb_travel)).max(1.0);
            self.scroll_amount = (self.scroll_amount
                + f64::from(mouse_y - previous_y) * drag_scale)
                .clamp(0.0, self.max_scroll_amount);
        }
    }

    /// Returns whether the Done button was pressed.
    fn mouse_input(
        &mut self,
        world: &WorldStore,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        self.cursor_position = cursor_position.and_then(physical_position_floor_i32);
        if matches!((button, state), (MouseButton::Left, ElementState::Released)) {
            self.scrolling = false;
        }
        if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            return false;
        }
        if self.max_scroll_amount > 0.0
            && stats_screen_scrollbar_contains(self.cursor_position, surface_size)
        {
            self.scrolling = true;
            self.focus = StatsScreenFocus::List;
            return false;
        }
        let item_column = (self.tab == 1 && !self.loading)
            .then(|| {
                stats_screen_item_column_at_position(
                    self.cursor_position,
                    surface_size,
                    self.scroll_amount as i32,
                )
            })
            .flatten();
        if let Some(column) = item_column {
            self.activate_item_column(column);
            self.focus = StatsScreenFocus::ItemHeader(column);
        } else if let Some(tab) = stats_screen_tab_at_position(self.cursor_position, surface_size) {
            if self.tab_is_enabled(tab, world) {
                self.tab = tab;
                self.total_rows = self.tab_row_counts[tab];
                self.scroll_amount = 0.0;
                self.focus = StatsScreenFocus::Tabs;
            }
        } else if stats_screen_done_button_contains(self.cursor_position, surface_size) {
            self.focus = StatsScreenFocus::Done;
            return true;
        } else if let Some(row) = self.hovered_row(surface_size) {
            self.focus = match self.tab {
                1 => StatsScreenFocus::ItemRow(row),
                2 => StatsScreenFocus::MobRow(row),
                _ => StatsScreenFocus::GeneralRow(row),
            };
        }
        false
    }

    pub(crate) fn activate_item_column(&mut self, column: usize) {
        match (self.item_sort_column, self.item_sort_order) {
            (Some(current), -1) if current == column => {
                self.item_sort_order = 1;
                self.item_effective_sort = Some((column, 1));
            }
            (Some(current), 1) if current == column => {
                self.item_sort_column = None;
                self.item_sort_order = 0;
            }
            _ => {
                self.item_sort_column = Some(column);
                self.item_sort_order = -1;
                self.item_effective_sort = Some((column, -1));
            }
        }
        self.scroll_amount = 0.0;
    }

    /// Returns whether the key pressed the Done button.
    fn key(&mut self, world: &WorldStore, code: KeyCode, control: bool, shift: bool) -> bool {
        if control {
            let requested = stats_screen_digit_tab(code).or_else(|| match code {
                KeyCode::Tab => self.next_enabled_tab(if shift { -1 } else { 1 }, world),
                _ => None,
            });
            if let Some(tab) = requested.filter(|tab| self.tab_is_enabled(*tab, world)) {
                let old_tab = self.tab;
                self.tab = tab;
                self.total_rows = self.tab_row_counts[tab];
                self.scroll_amount = 0.0;
                self.scrolling = false;
                if tab != old_tab
                    && matches!(
                        self.focus,
                        StatsScreenFocus::List
                            | StatsScreenFocus::GeneralRow(_)
                            | StatsScreenFocus::ItemHeader(_)
                            | StatsScreenFocus::ItemRow(_)
                            | StatsScreenFocus::MobRow(_)
                    )
                {
                    // `TabManager` removes the old list widget and `Screen.removeWidget`
                    // clears focus when the shortcut is used outside the tab bar.
                    self.focus = StatsScreenFocus::None;
                }
            }
            return false;
        }
        match code {
            KeyCode::Tab => self.move_tab_focus(!shift),
            KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::ArrowUp | KeyCode::ArrowDown => {
                self.move_arrow_focus(world, code)
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                return self.activate_focus();
            }
            _ => {}
        }
        false
    }

    pub(crate) fn move_tab_focus(&mut self, forward: bool) {
        let total_rows = self.total_rows;
        let last_list_focus = || match self.tab {
            1 if total_rows > 0 => StatsScreenFocus::ItemRow(total_rows - 1),
            1 => StatsScreenFocus::ItemHeader(5),
            2 if total_rows > 0 => StatsScreenFocus::MobRow(total_rows - 1),
            0 if total_rows > 0 => StatsScreenFocus::GeneralRow(total_rows - 1),
            _ => StatsScreenFocus::List,
        };
        let first_list_focus = || match self.tab {
            1 => StatsScreenFocus::ItemHeader(0),
            2 if total_rows > 0 => StatsScreenFocus::MobRow(0),
            0 if total_rows > 0 => StatsScreenFocus::GeneralRow(0),
            _ => StatsScreenFocus::List,
        };
        self.focus = if forward {
            match self.focus {
                StatsScreenFocus::None => StatsScreenFocus::Tabs,
                StatsScreenFocus::Tabs => first_list_focus(),
                StatsScreenFocus::ItemHeader(column) if column < 5 => {
                    StatsScreenFocus::ItemHeader(column + 1)
                }
                StatsScreenFocus::ItemHeader(_) if total_rows > 0 => StatsScreenFocus::ItemRow(0),
                StatsScreenFocus::ItemRow(row) if row + 1 < total_rows => {
                    StatsScreenFocus::ItemRow(row + 1)
                }
                StatsScreenFocus::List
                | StatsScreenFocus::GeneralRow(_)
                | StatsScreenFocus::ItemHeader(_)
                | StatsScreenFocus::ItemRow(_)
                | StatsScreenFocus::MobRow(_) => StatsScreenFocus::Done,
                StatsScreenFocus::Done => StatsScreenFocus::Tabs,
            }
        } else {
            match self.focus {
                StatsScreenFocus::None => StatsScreenFocus::Done,
                StatsScreenFocus::Tabs => StatsScreenFocus::Done,
                StatsScreenFocus::ItemHeader(column) if column > 0 => {
                    StatsScreenFocus::ItemHeader(column - 1)
                }
                StatsScreenFocus::ItemRow(0) => StatsScreenFocus::ItemHeader(5),
                StatsScreenFocus::ItemRow(row) => StatsScreenFocus::ItemRow(row - 1),
                StatsScreenFocus::List
                | StatsScreenFocus::GeneralRow(_)
                | StatsScreenFocus::ItemHeader(_)
                | StatsScreenFocus::MobRow(_) => StatsScreenFocus::Tabs,
                StatsScreenFocus::Done => last_list_focus(),
            }
        };
    }

    pub(crate) fn move_arrow_focus(&mut self, world: &WorldStore, code: KeyCode) {
        let total_rows = self.total_rows;
        self.focus = match (self.focus, code) {
            (StatsScreenFocus::Tabs, KeyCode::ArrowLeft | KeyCode::ArrowRight) => {
                let direction = if code == KeyCode::ArrowLeft { -1 } else { 1 };
                let tab = self.next_enabled_tab(direction, world).unwrap_or(self.tab);
                self.tab = tab;
                self.total_rows = self.tab_row_counts[tab];
                self.scroll_amount = 0.0;
                StatsScreenFocus::Tabs
            }
            (StatsScreenFocus::Tabs, KeyCode::ArrowDown) => match self.tab {
                1 => StatsScreenFocus::ItemHeader(0),
                2 if total_rows > 0 => StatsScreenFocus::MobRow(0),
                0 if total_rows > 0 => StatsScreenFocus::GeneralRow(0),
                _ => StatsScreenFocus::List,
            },
            (StatsScreenFocus::GeneralRow(0), KeyCode::ArrowUp)
            | (StatsScreenFocus::MobRow(0), KeyCode::ArrowUp)
            | (StatsScreenFocus::ItemHeader(_), KeyCode::ArrowUp) => StatsScreenFocus::Tabs,
            (StatsScreenFocus::GeneralRow(row), KeyCode::ArrowUp) => {
                StatsScreenFocus::GeneralRow(row - 1)
            }
            (StatsScreenFocus::MobRow(row), KeyCode::ArrowUp) => StatsScreenFocus::MobRow(row - 1),
            (StatsScreenFocus::ItemRow(0), KeyCode::ArrowUp) => StatsScreenFocus::ItemHeader(0),
            (StatsScreenFocus::ItemRow(row), KeyCode::ArrowUp) => {
                StatsScreenFocus::ItemRow(row - 1)
            }
            (StatsScreenFocus::GeneralRow(row), KeyCode::ArrowDown) if row + 1 < total_rows => {
                StatsScreenFocus::GeneralRow(row + 1)
            }
            (StatsScreenFocus::MobRow(row), KeyCode::ArrowDown) if row + 1 < total_rows => {
                StatsScreenFocus::MobRow(row + 1)
            }
            (StatsScreenFocus::ItemHeader(_), KeyCode::ArrowDown) if total_rows > 0 => {
                StatsScreenFocus::ItemRow(0)
            }
            (StatsScreenFocus::ItemRow(row), KeyCode::ArrowDown) if row + 1 < total_rows => {
                StatsScreenFocus::ItemRow(row + 1)
            }
            (
                StatsScreenFocus::GeneralRow(_)
                | StatsScreenFocus::MobRow(_)
                | StatsScreenFocus::ItemRow(_),
                KeyCode::ArrowDown,
            ) => StatsScreenFocus::Done,
            (StatsScreenFocus::ItemHeader(column), KeyCode::ArrowLeft) if column > 0 => {
                StatsScreenFocus::ItemHeader(column - 1)
            }
            (StatsScreenFocus::ItemHeader(column), KeyCode::ArrowRight) if column < 5 => {
                StatsScreenFocus::ItemHeader(column + 1)
            }
            (StatsScreenFocus::Done, KeyCode::ArrowUp) => match self.tab {
                1 if total_rows > 0 => StatsScreenFocus::ItemRow(total_rows - 1),
                1 => StatsScreenFocus::ItemHeader(0),
                2 if total_rows > 0 => StatsScreenFocus::MobRow(total_rows - 1),
                0 if total_rows > 0 => StatsScreenFocus::GeneralRow(total_rows - 1),
                _ => StatsScreenFocus::List,
            },
            (focus, _) => focus,
        };
    }

    /// Returns whether the focused Done button should close the screen.
    pub(crate) fn activate_focus(&mut self) -> bool {
        match self.focus {
            StatsScreenFocus::ItemHeader(column) => {
                self.activate_item_column(column);
                false
            }
            StatsScreenFocus::Done => true,
            _ => false,
        }
    }

    pub(crate) fn tab_is_enabled(&self, tab: usize, world: &WorldStore) -> bool {
        if tab == 0 {
            return true;
        }
        if self.loading {
            return false;
        }
        match tab {
            1 => [
                "minecraft:mined",
                "minecraft:broken",
                "minecraft:crafted",
                "minecraft:used",
                "minecraft:picked_up",
                "minecraft:dropped",
            ]
            .into_iter()
            .any(|stat_type| world.has_positive_stats_for_type(stat_type)),
            2 => {
                world.has_positive_stats_for_type("minecraft:killed")
                    || world.has_positive_stats_for_type("minecraft:killed_by")
            }
            _ => false,
        }
    }

    fn next_enabled_tab(&self, direction: i32, world: &WorldStore) -> Option<usize> {
        let mut next = self.tab;
        for _ in 0..3 {
            next = usize::try_from((next as i32 + direction).rem_euclid(3)).ok()?;
            if self.tab_is_enabled(next, world) {
                return Some(next);
            }
        }
        None
    }
}

impl ClientInputState {
    pub(crate) fn stats_screen(&self) -> Option<&StatsScreen> {
        self.screen.stats()
    }

    pub(crate) fn stats_screen_mut(&mut self) -> Option<&mut StatsScreen> {
        self.screen.stats_mut()
    }

    pub(crate) fn stats_screen_is_open(&self) -> bool {
        self.screen.stats().is_some()
    }

    /// `PauseScreen`'s Statistics button: `StatsScreen(this, stats)`, then
    /// `ServerboundClientCommandPacket.REQUEST_STATS`.
    pub(crate) fn open_stats_screen(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        if self.stats_screen_is_open() {
            return;
        }
        self.interrupt_chat_screen();
        release_active_input(self, world, counters, net_commands);
        world.replace_screen();
        self.screen
            .open_child(Screen::Stats(StatsScreen::default()));
        queue_request_stats_command(counters, net_commands);
    }

    /// `StatsScreen.onClose`: back to the screen it was opened from.
    pub(crate) fn close_stats_screen(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        if self
            .screen
            .close_if(|screen| matches!(screen, Screen::Stats(_)))
            .is_some()
        {
            release_active_input(self, world, counters, net_commands);
        }
    }

    pub(crate) fn handle_stats_screen_key(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        code: KeyCode,
        pressed: bool,
    ) -> bool {
        let (control, shift) = (self.control_down(), self.shift_down());
        let Some(stats) = self.screen.stats_mut() else {
            return false;
        };
        if pressed && (code == KeyCode::Escape || stats.key(world, code, control, shift)) {
            self.close_stats_screen(counters, world, net_commands);
        }
        true
    }

    pub(crate) fn handle_stats_screen_mouse_wheel(&mut self, delta: MouseScrollDelta) -> bool {
        let Some(stats) = self.screen.stats_mut() else {
            return false;
        };
        stats.mouse_wheel(delta);
        true
    }

    pub(crate) fn handle_stats_screen_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some(stats) = self.screen.stats_mut() else {
            return false;
        };
        stats.cursor_moved(cursor_position, surface_size);
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_stats_screen_mouse_input(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some(stats) = self.screen.stats_mut() else {
            return false;
        };
        if stats.mouse_input(world, button, state, cursor_position, surface_size) {
            self.close_stats_screen(counters, world, net_commands);
        }
        true
    }
}

pub(crate) fn stats_screen_done_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((mouse_x, mouse_y)) = cursor_position else {
        return false;
    };
    let (x, y, width, height) = hud_stats_done_button_rect(surface_size);
    mouse_x >= x && mouse_x < x + width && mouse_y >= y && mouse_y < y + height
}

pub(crate) fn stats_screen_tab_at_position(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> Option<usize> {
    let (mouse_x, mouse_y) = cursor_position?;
    hud_stats_tab_rects(surface_size)
        .iter()
        .position(|&(x, y, width, height)| {
            (x..x + width).contains(&mouse_x) && (y..y + height).contains(&mouse_y)
        })
}

pub(crate) fn stats_screen_item_column_at_position(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
    scroll_amount: i32,
) -> Option<usize> {
    let (mouse_x, mouse_y) = cursor_position?;
    let list_bottom = i32::try_from(surface_size.height)
        .unwrap_or(i32::MAX)
        .saturating_sub(STATS_SCREEN_FOOTER_HEIGHT);
    if !(STATS_SCREEN_LIST_TOP..list_bottom).contains(&mouse_y)
        || !(STATS_SCREEN_LIST_TOP + STATS_SCREEN_LIST_ENTRY_PADDING + 1
            ..STATS_SCREEN_LIST_TOP
                + STATS_SCREEN_LIST_ENTRY_PADDING
                + 1
                + STATS_SCREEN_ITEM_COLUMN_BUTTON_SIZE)
            .contains(&(mouse_y.saturating_add(scroll_amount)))
    {
        return None;
    }
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let list_x = width / 2 - STATS_SCREEN_LIST_WIDTH / 2;
    (0..6).find(|column| {
        let x = list_x
            + STATS_SCREEN_ITEM_COLUMN_FIRST_X
            + i32::try_from(*column).unwrap_or(i32::MAX) * STATS_SCREEN_ITEM_COLUMN_STRIDE;
        (x..x + STATS_SCREEN_ITEM_COLUMN_BUTTON_SIZE).contains(&mouse_x)
    })
}

fn stats_screen_row_at_position(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
    tab: usize,
    scroll_amount: i32,
) -> Option<usize> {
    let (mouse_x, mouse_y) = cursor_position?;
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let list_x = width / 2 - STATS_SCREEN_LIST_WIDTH / 2;
    let list_bottom = i32::try_from(surface_size.height)
        .unwrap_or(i32::MAX)
        .saturating_sub(STATS_SCREEN_FOOTER_HEIGHT);
    if !(list_x..list_x + STATS_SCREEN_LIST_WIDTH).contains(&mouse_x)
        || !(STATS_SCREEN_LIST_TOP..list_bottom).contains(&mouse_y)
    {
        return None;
    }
    let content_y =
        mouse_y - STATS_SCREEN_LIST_TOP - STATS_SCREEN_LIST_ENTRY_PADDING + scroll_amount;
    let row_height = stats_screen_row_height(tab);
    (content_y >= 0)
        .then(|| usize::try_from(content_y / row_height).ok())
        .flatten()
}

fn stats_screen_row_height(tab: usize) -> i32 {
    match tab {
        1 => 22,
        2 => 36,
        _ => STATS_SCREEN_ROW_HEIGHT,
    }
}

fn stats_screen_scrollbar_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((mouse_x, mouse_y)) = cursor_position else {
        return false;
    };
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let x = width / 2 + STATS_SCREEN_LIST_WIDTH / 2 + STATS_SCREEN_SCROLLBAR_WIDTH + 2;
    (x..=x + STATS_SCREEN_SCROLLBAR_WIDTH).contains(&mouse_x)
        && (STATS_SCREEN_LIST_TOP..height.saturating_sub(STATS_SCREEN_FOOTER_HEIGHT))
            .contains(&mouse_y)
}

fn stats_screen_digit_tab(code: KeyCode) -> Option<usize> {
    match code {
        KeyCode::Digit1 | KeyCode::Numpad1 => Some(0),
        KeyCode::Digit2 | KeyCode::Numpad2 => Some(1),
        KeyCode::Digit0
        | KeyCode::Digit3
        | KeyCode::Digit4
        | KeyCode::Digit5
        | KeyCode::Digit6
        | KeyCode::Digit7
        | KeyCode::Digit8
        | KeyCode::Digit9
        | KeyCode::Numpad0
        | KeyCode::Numpad3
        | KeyCode::Numpad4
        | KeyCode::Numpad5
        | KeyCode::Numpad6
        | KeyCode::Numpad7
        | KeyCode::Numpad8
        | KeyCode::Numpad9 => Some(2),
        _ => None,
    }
}
