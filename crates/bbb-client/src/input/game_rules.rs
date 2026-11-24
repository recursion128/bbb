use crate::control::NetCounters;
use bbb_client_net::NetCommand;
use bbb_client_render::{
    hud_game_rules_confirmation_buttons, hud_game_rules_layout, HUD_GAME_RULES_ROW_HEIGHT,
};
use bbb_client_world::{GameRuleEditValueState, GameRulesScreenState, WorldStore};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::KeyCode,
};

use super::{
    physical_position_floor_i32, queue_container_close_request_command,
    queue_request_game_rule_values_command, queue_set_game_rule_command, release_active_input,
    text_field::{BoundedTextField, TextField},
    ClientInputState, DebugClipboard, Screen,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GameRuleSearchEntry {
    pub(crate) rule: String,
    pub(crate) category: String,
    pub(crate) label: String,
    pub(crate) category_label: String,
    pub(crate) description: Option<String>,
    pub(crate) searchable_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GameRulesInteractionRow {
    Category { category: String, label: String },
    Rule { rule: String },
}

impl GameRulesInteractionRow {
    pub(crate) fn rule(&self) -> Option<&str> {
        match self {
            Self::Category { .. } => None,
            Self::Rule { rule } => Some(rule),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum GameRulesScreenFocus {
    #[default]
    Search,
    Rule(String),
    Done,
    Cancel,
    Discard,
    KeepEditing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GameRuleIntegerInput {
    rule: String,
    edit: TextField,
}

impl GameRuleIntegerInput {
    fn new(rule: String, text: &str) -> Self {
        Self {
            rule,
            edit: TextField::with_text(text, GAME_RULE_EDIT_BOX_MAX_LENGTH),
        }
    }
}

const GAME_RULE_EDIT_BOX_MAX_LENGTH: usize = 32;

pub(crate) struct GameRulesProjectedView<'a> {
    pub(crate) rows: std::borrow::Cow<'a, [GameRulesInteractionRow]>,
    pub(crate) focus: &'a GameRulesScreenFocus,
    pub(crate) scroll_amount: i32,
    pub(crate) max_scroll_amount: i32,
    pub(crate) scroller_height: i32,
    pub(crate) hovered_row: Option<usize>,
}

impl GameRulesProjectedView<'_> {
    pub(crate) fn hovered_rule(&self) -> Option<&str> {
        self.hovered_row
            .and_then(|row| self.rows.get(row))
            .and_then(GameRulesInteractionRow::rule)
    }
}

/// `InGameGameRulesScreen`: the edits plus the list, search and focus state.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GameRulesScreen {
    pub(crate) rules: GameRulesScreenState,
    search: BoundedTextField<GAME_RULE_EDIT_BOX_MAX_LENGTH>,
    focus: GameRulesScreenFocus,
    search_entries: Vec<GameRuleSearchEntry>,
    rows: Vec<GameRulesInteractionRow>,
    integer_input: Option<GameRuleIntegerInput>,
    cursor_position: Option<(i32, i32)>,
    scroll_amount: f64,
    max_scroll_amount: f64,
    scroller_height: i32,
    scrolling: bool,
    surface_size: Option<PhysicalSize<u32>>,
}

impl GameRulesScreen {
    fn new() -> Self {
        Self {
            rules: GameRulesScreenState::loading(),
            search: BoundedTextField::default(),
            focus: GameRulesScreenFocus::Search,
            search_entries: Vec::new(),
            rows: Vec::new(),
            integer_input: None,
            cursor_position: None,
            scroll_amount: 0.0,
            max_scroll_amount: 0.0,
            scroller_height: 0,
            scrolling: false,
            surface_size: None,
        }
    }
}

impl ClientInputState {
    pub(crate) fn game_rules_screen(&self) -> Option<&GameRulesScreen> {
        self.screen.game_rules()
    }

    pub(crate) fn game_rules_screen_mut(&mut self) -> Option<&mut GameRulesScreen> {
        self.screen.game_rules_mut()
    }

    /// The game rules request opens `InGameGameRulesScreen` for an operator
    /// and asks the server for the current values.
    pub(crate) fn open_game_rules_screen(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) -> bool {
        if self.screen.game_rules().is_some() || !world.local_player_has_gamemaster_permission() {
            return false;
        }
        if let Some(container_id) = world.open_container_id() {
            world.close_local_container(container_id);
            queue_container_close_request_command(counters, net_commands, container_id);
        }
        release_active_input(self, world, counters, net_commands);
        world.replace_screen();
        self.set_screen(
            Screen::GameRules(GameRulesScreen::new()),
            counters,
            net_commands,
        );
        queue_request_game_rule_values_command(counters, net_commands);
        true
    }

    pub(super) fn close_game_rules_screen(&mut self) {
        self.screen
            .close_if(|screen| matches!(screen, Screen::GameRules(_)));
    }

    /// Whether a game-rules text field would take a commit right now: the
    /// buttons are not text, and a discard confirmation owns the keyboard.
    pub(crate) fn game_rules_text_is_focused(&self) -> bool {
        self.screen.game_rules().is_some_and(|state| {
            !state.rules.discard_confirmation_open
                && matches!(
                    state.focus,
                    GameRulesScreenFocus::Search | GameRulesScreenFocus::Rule(_)
                )
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_game_rules_screen_key(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        code: KeyCode,
        pressed: bool,
        surface_size: PhysicalSize<u32>,
        clipboard: &mut Option<&mut dyn DebugClipboard>,
    ) -> bool {
        let forward = !self.shift_down();
        let modifiers = self.text_field_modifiers();
        let Some(state) = self.screen.game_rules_mut() else {
            return false;
        };
        state.surface_size = Some(surface_size);
        if !pressed {
            return true;
        }
        if state.rules.discard_confirmation_open {
            let mut discard = false;
            match code {
                KeyCode::Escape => {
                    state.rules.cancel_discard();
                    state.focus = GameRulesScreenFocus::Cancel;
                }
                KeyCode::Tab | KeyCode::ArrowLeft | KeyCode::ArrowRight => {
                    state.focus = match state.focus {
                        GameRulesScreenFocus::Discard => GameRulesScreenFocus::KeepEditing,
                        _ => GameRulesScreenFocus::Discard,
                    };
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                    discard = matches!(state.focus, GameRulesScreenFocus::Discard);
                    if !discard {
                        state.rules.cancel_discard();
                    }
                }
                KeyCode::KeyY => discard = true,
                KeyCode::KeyN => {
                    state.rules.cancel_discard();
                }
                _ => {}
            }
            if discard {
                self.close_game_rules_screen();
            }
            return true;
        }

        if matches!(code, KeyCode::Escape) {
            if state.rules.request_close() {
                self.close_game_rules_screen();
            } else {
                state.focus = GameRulesScreenFocus::KeepEditing;
            }
            return true;
        }
        if matches!(code, KeyCode::Tab) {
            state.move_focus(forward);
            state.ensure_focused_rule_visible(surface_size);
            return true;
        }
        if matches!(code, KeyCode::ArrowUp | KeyCode::ArrowDown) && !state.text_field_is_focused() {
            state.move_focus(matches!(code, KeyCode::ArrowDown));
            state.ensure_focused_rule_visible(surface_size);
            return true;
        }
        if matches!(code, KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space) {
            self.activate_game_rules_screen_focus(counters, net_commands);
            return true;
        }
        state.edit_focused(surface_size, |field| {
            field.key(code, modifiers, clipboard, None);
        });
        true
    }

    pub(crate) fn handle_game_rules_screen_text(&mut self, text: &str) -> bool {
        let Some(state) = self.screen.game_rules_mut() else {
            return false;
        };
        if state.rules.discard_confirmation_open {
            return false;
        }
        let surface_size = state
            .surface_size
            .unwrap_or_else(|| PhysicalSize::new(1280, 720));
        state.edit_focused(surface_size, |field| {
            field.type_text(text, None);
        });
        true
    }

    pub(crate) fn handle_game_rules_screen_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some(state) = self.screen.game_rules_mut() else {
            return false;
        };
        state.surface_size = Some(surface_size);
        let previous = state.cursor_position;
        state.cursor_position = cursor_position.and_then(physical_position_floor_i32);
        if state.scrolling {
            let layout = hud_game_rules_layout(surface_size);
            let Some((_, mouse_y)) = state.cursor_position else {
                return true;
            };
            if mouse_y <= layout.scrollbar.y {
                state.scroll_amount = 0.0;
            } else if mouse_y >= layout.scrollbar.y.saturating_add(layout.scrollbar.height) {
                state.scroll_amount = state.max_scroll_amount;
            } else if let Some((_, previous_y)) = previous {
                let travel = (layout.scrollbar.height - state.scroller_height).max(1);
                let scale = (state.max_scroll_amount / f64::from(travel)).max(1.0);
                state.scroll_amount = (state.scroll_amount
                    + f64::from(mouse_y - previous_y) * scale)
                    .clamp(0.0, state.max_scroll_amount);
            }
        }
        true
    }

    pub(crate) fn handle_game_rules_screen_mouse_wheel(&mut self, delta: MouseScrollDelta) -> bool {
        let Some(state) = self.screen.game_rules_mut() else {
            return false;
        };
        let pixels = match delta {
            MouseScrollDelta::LineDelta(_, y) => {
                f64::from(y) * f64::from(HUD_GAME_RULES_ROW_HEIGHT / 2)
            }
            MouseScrollDelta::PixelDelta(position) => position.y,
        };
        state.scroll_amount = (state.scroll_amount - pixels).clamp(0.0, state.max_scroll_amount);
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_game_rules_screen_mouse_input(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: MouseButton,
        element_state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some(state) = self.screen.game_rules_mut() else {
            return false;
        };
        state.surface_size = Some(surface_size);
        state.cursor_position = cursor_position.and_then(physical_position_floor_i32);
        if matches!(
            (button, element_state),
            (MouseButton::Left, ElementState::Released)
        ) {
            state.scrolling = false;
            return true;
        }
        if !matches!(
            (button, element_state),
            (MouseButton::Left, ElementState::Pressed)
        ) {
            return true;
        }
        if state.rules.discard_confirmation_open {
            let (discard, keep_editing) = hud_game_rules_confirmation_buttons(surface_size);
            if discard.contains(state.cursor_position) {
                self.close_game_rules_screen();
            } else if keep_editing.contains(state.cursor_position) {
                state.rules.cancel_discard();
                state.focus = GameRulesScreenFocus::KeepEditing;
            }
            return true;
        }

        let layout = hud_game_rules_layout(surface_size);
        if layout.search.contains(state.cursor_position) {
            state.focus = GameRulesScreenFocus::Search;
            state.integer_input = None;
            return true;
        }
        if state.max_scroll_amount > 0.0 && layout.scrollbar.contains(state.cursor_position) {
            state.scrolling = true;
            return true;
        }
        if layout.done.contains(state.cursor_position) {
            state.focus = GameRulesScreenFocus::Done;
            self.activate_game_rules_screen_focus(counters, net_commands);
            return true;
        }
        if layout.cancel.contains(state.cursor_position) {
            state.focus = GameRulesScreenFocus::Cancel;
            self.activate_game_rules_screen_focus(counters, net_commands);
            return true;
        }
        let Some(row_index) = state.row_at_cursor(surface_size) else {
            return true;
        };
        let Some(rule) = state
            .rows
            .get(row_index)
            .and_then(GameRulesInteractionRow::rule)
            .map(str::to_string)
        else {
            return true;
        };
        let value_rect =
            hud_game_rules_layout(surface_size).value_rect(row_index, state.scroll_amount as i32);
        if !value_rect.contains(state.cursor_position) {
            return true;
        }
        state.focus = GameRulesScreenFocus::Rule(rule.clone());
        match state.rule_value(&rule) {
            Some(GameRuleEditValueState::Boolean { value }) => {
                state.rules.set_boolean(&rule, !value);
                state.integer_input = None;
            }
            Some(GameRuleEditValueState::Integer { input, .. }) => {
                state.integer_input = Some(GameRuleIntegerInput::new(rule, &input));
            }
            None => {}
        }
        true
    }

    fn activate_game_rules_screen_focus(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        let Some(state) = self.screen.game_rules_mut() else {
            return;
        };
        match state.focus.clone() {
            GameRulesScreenFocus::Rule(rule) => {
                if let Some(GameRuleEditValueState::Boolean { value }) = state.rule_value(&rule) {
                    state.rules.set_boolean(&rule, !value);
                }
            }
            GameRulesScreenFocus::Done => {
                if let Some(command) = state.rules.finish() {
                    queue_set_game_rule_command(counters, net_commands, command);
                    self.close_game_rules_screen();
                }
            }
            GameRulesScreenFocus::Cancel => {
                if state.rules.request_close() {
                    self.close_game_rules_screen();
                } else {
                    state.focus = GameRulesScreenFocus::KeepEditing;
                }
            }
            _ => {}
        }
    }
}

impl GameRulesScreen {
    pub(crate) fn search(&self) -> &TextField {
        &self.search
    }

    #[cfg(test)]
    pub(crate) fn rows(&self) -> &[GameRulesInteractionRow] {
        &self.rows
    }

    pub(crate) fn integer_input(&self, rule: &str) -> Option<&TextField> {
        self.integer_input
            .as_ref()
            .filter(|input| input.rule == rule)
            .map(|input| &input.edit)
    }

    pub(crate) fn focused_text_is_search(&self) -> Option<bool> {
        if self.rules.discard_confirmation_open {
            return None;
        }
        match &self.focus {
            GameRulesScreenFocus::Search => Some(true),
            GameRulesScreenFocus::Rule(rule)
                if self
                    .integer_input
                    .as_ref()
                    .is_some_and(|input| input.rule == *rule) =>
            {
                Some(false)
            }
            _ => None,
        }
    }

    pub(super) fn set_focused_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
    ) {
        self.clear_ime_preedit();
        match &self.focus {
            GameRulesScreenFocus::Search => self.search.set_ime_preedit(text, cursor_range),
            GameRulesScreenFocus::Rule(rule) => {
                if let Some(integer) = self
                    .integer_input
                    .as_mut()
                    .filter(|integer| integer.rule == *rule)
                {
                    integer.edit.set_ime_preedit(text, cursor_range);
                }
            }
            _ => {}
        }
    }

    pub(super) fn clear_ime_preedit(&mut self) {
        self.search.clear_ime_preedit();
        if let Some(integer) = self.integer_input.as_mut() {
            integer.edit.clear_ime_preedit();
        }
    }

    /// Applies `edit` to the focused text field and reports the new value
    /// like `EditBox.setResponder`.
    fn edit_focused(&mut self, surface_size: PhysicalSize<u32>, edit: impl FnOnce(&mut TextField)) {
        match self.focus.clone() {
            GameRulesScreenFocus::Search => {
                let before = self.search.value().to_vec();
                edit(&mut self.search);
                if self.search.value() != before {
                    self.rebuild_rows(surface_size);
                    self.validate_focus();
                }
            }
            GameRulesScreenFocus::Rule(rule) => {
                if let Some(integer) = self
                    .integer_input
                    .as_mut()
                    .filter(|integer| integer.rule == rule)
                {
                    let before = integer.edit.value().to_vec();
                    edit(&mut integer.edit);
                    if integer.edit.value() != before {
                        self.rules.set_integer_input(&rule, integer.edit.text());
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn cursor_position(&self) -> Option<(i32, i32)> {
        self.cursor_position
    }

    /// Projects filtered rows, scroll metrics, and validated focus without cloning the screen.
    pub(crate) fn project_view<'a>(
        &'a self,
        entries: &[GameRuleSearchEntry],
        surface_size: PhysicalSize<u32>,
    ) -> GameRulesProjectedView<'a> {
        let rows: std::borrow::Cow<'a, [GameRulesInteractionRow]> =
            if self.surface_size == Some(surface_size) && self.search_entries.as_slice() == entries
            {
                std::borrow::Cow::Borrowed(&self.rows)
            } else {
                let query = self.search.text().to_lowercase();
                let mut rows = Vec::new();
                let mut last_category = None::<&str>;
                for entry in entries
                    .iter()
                    .filter(|entry| query.is_empty() || entry.searchable_text.contains(&query))
                {
                    if last_category != Some(entry.category.as_str()) {
                        rows.push(GameRulesInteractionRow::Category {
                            category: entry.category.clone(),
                            label: entry.category_label.clone(),
                        });
                        last_category = Some(entry.category.as_str());
                    }
                    rows.push(GameRulesInteractionRow::Rule {
                        rule: entry.rule.clone(),
                    });
                }
                std::borrow::Cow::Owned(rows)
            };
        let layout = hud_game_rules_layout(surface_size);
        let content_height = i32::try_from(rows.len())
            .unwrap_or(i32::MAX / HUD_GAME_RULES_ROW_HEIGHT)
            .saturating_mul(HUD_GAME_RULES_ROW_HEIGHT);
        let max_scroll = content_height.saturating_sub(layout.list.height).max(0);
        let max_scroll_amount = f64::from(max_scroll);
        let scroll_amount = self.scroll_amount.clamp(0.0, max_scroll_amount);
        let scroller_height = if max_scroll > 0 && layout.list.height > 0 {
            layout
                .list
                .height
                .saturating_mul(layout.list.height)
                .checked_div(content_height.max(1))
                .unwrap_or(layout.list.height)
                .clamp(1, layout.list.height)
        } else {
            0
        };
        let focus_valid = match &self.focus {
            GameRulesScreenFocus::Rule(rule) => {
                rows.iter().any(|row| row.rule() == Some(rule.as_str()))
            }
            GameRulesScreenFocus::Done => self.rules.can_submit(),
            _ => true,
        };
        let focus = if focus_valid {
            &self.focus
        } else {
            &GameRulesScreenFocus::Search
        };
        let hovered_row = self.cursor_position.and_then(|(x, y)| {
            if !layout.list.contains(Some((x, y))) {
                return None;
            }
            let content_y = y
                .saturating_sub(layout.list.y)
                .saturating_add(scroll_amount as i32);
            usize::try_from(content_y / HUD_GAME_RULES_ROW_HEIGHT)
                .ok()
                .filter(|row| *row < rows.len())
        });
        GameRulesProjectedView {
            rows,
            focus,
            scroll_amount: scroll_amount as i32,
            max_scroll_amount: max_scroll_amount as i32,
            scroller_height,
            hovered_row,
        }
    }

    fn rebuild_rows(&mut self, surface_size: PhysicalSize<u32>) {
        let query = self.search.text().to_lowercase();
        self.rows.clear();
        let mut last_category = None::<&str>;
        for entry in self
            .search_entries
            .iter()
            .filter(|entry| query.is_empty() || entry.searchable_text.contains(&query))
        {
            if last_category != Some(entry.category.as_str()) {
                self.rows.push(GameRulesInteractionRow::Category {
                    category: entry.category.clone(),
                    label: entry.category_label.clone(),
                });
                last_category = Some(entry.category.as_str());
            }
            self.rows.push(GameRulesInteractionRow::Rule {
                rule: entry.rule.clone(),
            });
        }
        self.sync_viewport(surface_size);
    }

    fn sync_viewport(&mut self, surface_size: PhysicalSize<u32>) {
        let layout = hud_game_rules_layout(surface_size);
        let content_height = i32::try_from(self.rows.len())
            .unwrap_or(i32::MAX / HUD_GAME_RULES_ROW_HEIGHT)
            .saturating_mul(HUD_GAME_RULES_ROW_HEIGHT);
        let max_scroll = content_height.saturating_sub(layout.list.height).max(0);
        self.max_scroll_amount = f64::from(max_scroll);
        self.scroll_amount = self.scroll_amount.clamp(0.0, self.max_scroll_amount);
        self.scroller_height = if max_scroll > 0 && layout.list.height > 0 {
            layout
                .list
                .height
                .saturating_mul(layout.list.height)
                .checked_div(content_height.max(1))
                .unwrap_or(layout.list.height)
                .clamp(1, layout.list.height)
        } else {
            0
        };
    }

    fn rule_value(&self, rule: &str) -> Option<GameRuleEditValueState> {
        self.rules
            .rules
            .iter()
            .find(|edit| edit.rule == rule)
            .map(|edit| edit.current_value.clone())
    }

    fn text_field_is_focused(&self) -> bool {
        matches!(self.focus, GameRulesScreenFocus::Search)
            || matches!(
                &self.focus,
                GameRulesScreenFocus::Rule(rule)
                    if self.integer_input.as_ref().is_some_and(|input| input.rule == *rule)
            )
    }

    /// Feeds the projected rule labels and rebuilds the filtered rows.
    pub(crate) fn sync_projection(
        &mut self,
        entries: Vec<GameRuleSearchEntry>,
        surface_size: PhysicalSize<u32>,
    ) {
        self.surface_size = Some(surface_size);
        self.search_entries = entries;
        self.rebuild_rows(surface_size);
        self.sync_integer_input();
        self.validate_focus();
    }

    fn sync_integer_input(&mut self) {
        let GameRulesScreenFocus::Rule(rule) = self.focus.clone() else {
            self.integer_input = None;
            return;
        };
        let Some(GameRuleEditValueState::Integer { input, .. }) = self.rule_value(&rule) else {
            self.integer_input = None;
            return;
        };
        if self
            .integer_input
            .as_ref()
            .is_some_and(|current| current.rule == rule)
        {
            return;
        }
        self.integer_input = Some(GameRuleIntegerInput::new(rule, &input));
    }

    fn validate_focus(&mut self) {
        let valid = match &self.focus {
            GameRulesScreenFocus::Rule(rule) => self
                .rows
                .iter()
                .any(|row| row.rule() == Some(rule.as_str())),
            GameRulesScreenFocus::Done => self.rules.can_submit(),
            _ => true,
        };
        if !valid {
            self.focus = GameRulesScreenFocus::Search;
            self.integer_input = None;
        }
    }

    fn move_focus(&mut self, forward: bool) {
        let mut focuses = Vec::with_capacity(self.rows.len() + 3);
        focuses.push(GameRulesScreenFocus::Search);
        focuses.extend(self.rows.iter().filter_map(|row| {
            row.rule()
                .map(|rule| GameRulesScreenFocus::Rule(rule.to_string()))
        }));
        if self.rules.can_submit() {
            focuses.push(GameRulesScreenFocus::Done);
        }
        focuses.push(GameRulesScreenFocus::Cancel);
        let current = focuses.iter().position(|focus| focus == &self.focus);
        let next = match (current, forward) {
            (Some(index), true) => (index + 1) % focuses.len(),
            (Some(0), false) | (None, false) => focuses.len() - 1,
            (Some(index), false) => index - 1,
            (None, true) => 0,
        };
        self.focus = focuses[next].clone();
        self.sync_integer_input();
    }

    fn ensure_focused_rule_visible(&mut self, surface_size: PhysicalSize<u32>) {
        let GameRulesScreenFocus::Rule(rule) = &self.focus else {
            return;
        };
        let Some(index) = self
            .rows
            .iter()
            .position(|row| row.rule() == Some(rule.as_str()))
        else {
            return;
        };
        let layout = hud_game_rules_layout(surface_size);
        let top = i32::try_from(index)
            .unwrap_or(i32::MAX / HUD_GAME_RULES_ROW_HEIGHT)
            .saturating_mul(HUD_GAME_RULES_ROW_HEIGHT);
        let bottom = top.saturating_add(HUD_GAME_RULES_ROW_HEIGHT);
        let scroll = self.scroll_amount as i32;
        if top < scroll {
            self.scroll_amount = f64::from(top);
        } else if bottom > scroll.saturating_add(layout.list.height) {
            self.scroll_amount = f64::from(bottom.saturating_sub(layout.list.height));
        }
        self.scroll_amount = self.scroll_amount.clamp(0.0, self.max_scroll_amount);
    }

    fn row_at_cursor(&self, surface_size: PhysicalSize<u32>) -> Option<usize> {
        let layout = hud_game_rules_layout(surface_size);
        let (x, y) = self.cursor_position?;
        if !layout.list.contains(Some((x, y))) {
            return None;
        }
        let content_y = y
            .saturating_sub(layout.list.y)
            .saturating_add(self.scroll_amount as i32);
        usize::try_from(content_y / HUD_GAME_RULES_ROW_HEIGHT)
            .ok()
            .filter(|row| *row < self.rows.len())
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::packets::{
        CommonPlayerSpawnInfo, EntityEvent, GameRuleValue as ProtocolGameRuleValue, GameRuleValues,
        PlayLogin,
    };

    use super::*;

    fn operator_world() -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_login(&PlayLogin {
            player_id: 42,
            hardcore: false,
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
                game_type: 1,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        });
        assert!(world.apply_entity_event(EntityEvent {
            entity_id: 42,
            event_id: 26,
        }));
        world.take_screen_requests();
        world
    }

    fn open_loaded_screen(input: &mut ClientInputState, world: &mut WorldStore) {
        assert!(input.open_game_rules_screen(world, &mut NetCounters::default(), &None));
        world.apply_game_rule_values(GameRuleValues {
            values: vec![
                ProtocolGameRuleValue {
                    rule: "minecraft:keep_inventory".to_string(),
                    value: "false".to_string(),
                },
                ProtocolGameRuleValue {
                    rule: "minecraft:random_tick_speed".to_string(),
                    value: "3".to_string(),
                },
            ],
        });
        input.apply_screen_requests(world, &mut NetCounters::default(), &None);
        input
            .game_rules_screen_mut()
            .unwrap()
            .sync_projection(entries(), PhysicalSize::new(854, 480));
    }

    fn entries() -> Vec<GameRuleSearchEntry> {
        vec![
            GameRuleSearchEntry {
                rule: "minecraft:keep_inventory".to_string(),
                category: "player".to_string(),
                label: "Keep Inventory".to_string(),
                category_label: "Player".to_string(),
                description: Some("Keep items after death".to_string()),
                searchable_text: "keep_inventory keep inventory player keep items after death"
                    .to_string(),
            },
            GameRuleSearchEntry {
                rule: "minecraft:random_tick_speed".to_string(),
                category: "updates".to_string(),
                label: "Random Tick Speed".to_string(),
                category_label: "Updates".to_string(),
                description: None,
                searchable_text: "random_tick_speed random tick speed updates".to_string(),
            },
        ]
    }

    #[test]
    fn projection_filters_localized_metadata_and_keeps_category_rows() {
        let mut world = operator_world();
        let mut input = ClientInputState::default();
        open_loaded_screen(&mut input, &mut world);

        assert!(input.handle_game_rules_screen_text("after death",));
        let state = input.game_rules_screen().unwrap();
        assert_eq!(
            state.rows(),
            &[
                GameRulesInteractionRow::Category {
                    category: "player".to_string(),
                    label: "Player".to_string(),
                },
                GameRulesInteractionRow::Rule {
                    rule: "minecraft:keep_inventory".to_string(),
                },
            ]
        );
    }

    #[test]
    fn boolean_and_integer_edits_submit_changed_only_packet() {
        let mut world = operator_world();
        let mut input = ClientInputState::default();
        open_loaded_screen(&mut input, &mut world);
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();

        input.game_rules_screen_mut().unwrap().focus =
            GameRulesScreenFocus::Rule("minecraft:keep_inventory".to_string());
        input.activate_game_rules_screen_focus(&mut counters, &commands);
        input.game_rules_screen_mut().unwrap().focus =
            GameRulesScreenFocus::Rule("minecraft:random_tick_speed".to_string());
        input.game_rules_screen_mut().unwrap().sync_integer_input();
        assert!(input.handle_game_rules_screen_key(
            &mut counters,
            &commands,
            KeyCode::KeyA,
            true,
            PhysicalSize::new(854, 480),
            &mut None,
        ));
        input.control_left_down = true;
        assert!(input.handle_game_rules_screen_key(
            &mut counters,
            &commands,
            KeyCode::KeyA,
            true,
            PhysicalSize::new(854, 480),
            &mut None,
        ));
        input.control_left_down = false;
        assert!(input.handle_game_rules_screen_text("6",));
        input.game_rules_screen_mut().unwrap().focus = GameRulesScreenFocus::Done;
        input.activate_game_rules_screen_focus(&mut counters, &commands);

        let NetCommand::SetGameRule(packet) = rx.try_recv().unwrap() else {
            panic!("set game rule");
        };
        assert_eq!(packet.entries.len(), 2);
        assert_eq!(counters.set_game_rule_commands_queued, 1);
        assert!(input.game_rules_screen().is_none());
    }

    #[test]
    fn escape_requires_confirmation_only_after_changes() {
        let mut world = operator_world();
        let mut input = ClientInputState::default();
        open_loaded_screen(&mut input, &mut world);
        let mut counters = NetCounters::default();

        input.game_rules_screen_mut().unwrap().focus =
            GameRulesScreenFocus::Rule("minecraft:keep_inventory".to_string());
        input.activate_game_rules_screen_focus(&mut counters, &None);
        assert!(input.handle_game_rules_screen_key(
            &mut counters,
            &None,
            KeyCode::Escape,
            true,
            PhysicalSize::new(854, 480),
            &mut None,
        ));
        assert!(
            input
                .game_rules_screen()
                .unwrap()
                .rules
                .discard_confirmation_open
        );
        assert!(input.handle_game_rules_screen_key(
            &mut counters,
            &None,
            KeyCode::KeyN,
            true,
            PhysicalSize::new(854, 480),
            &mut None,
        ));
        assert!(
            !input
                .game_rules_screen()
                .unwrap()
                .rules
                .discard_confirmation_open
        );
    }
    fn show_notice(world: &mut WorldStore) {
        use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
        world.apply_show_dialog(bbb_protocol::packets::ShowDialog {
            dialog: bbb_protocol::packets::DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "type".into(),
                        value: NbtSummaryValue::String("minecraft:notice".into()),
                    },
                    NbtSummaryEntry {
                        name: "title".into(),
                        value: NbtSummaryValue::String("Notice".into()),
                    },
                ]),
            },
        });
    }

    #[test]
    fn game_rules_dialog_parent_retains_search_selection_until_level_change() {
        let mut world = operator_world();
        let mut input = ClientInputState::default();
        let mut counters = NetCounters::default();
        open_loaded_screen(&mut input, &mut world);
        assert!(input.handle_game_rules_screen_text("tick"));
        input
            .game_rules_screen_mut()
            .unwrap()
            .search
            .set_selection(1, None);
        show_notice(&mut world);
        input.apply_screen_requests(&mut world, &mut counters, &None);
        assert!(input.game_rules_screen().is_none());
        assert!(input.close_dialog());
        input.apply_screen_requests(&mut world, &mut counters, &None);
        let state = input.game_rules_screen().unwrap();
        assert_eq!(state.search.text(), "tick");
        assert_eq!(state.search.cursor(), 4);
        assert_eq!(state.search.selection(), 1);

        show_notice(&mut world);
        input.apply_screen_requests(&mut world, &mut counters, &None);
        world.clear_client_level();
        input.apply_screen_requests(&mut world, &mut counters, &None);
        input.close_dialog();
        input.apply_screen_requests(&mut world, &mut counters, &None);
        assert!(input.game_rules_screen().is_none());
    }

    #[test]
    fn game_rules_screen_opens_once_for_operators_and_closes_on_permission_loss() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let mut input = ClientInputState::default();
        assert!(!input.open_game_rules_screen(&mut WorldStore::new(), &mut counters, &commands));

        let mut world = operator_world();
        assert!(input.open_game_rules_screen(&mut world, &mut counters, &commands));
        assert!(!input.open_game_rules_screen(&mut world, &mut counters, &commands));
        assert!(input.game_rules_screen().unwrap().rules.loading);
        assert_eq!(counters.request_game_rule_values_commands_queued, 1);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::RequestGameRuleValues);
        assert!(rx.try_recv().is_err());

        assert!(world.apply_entity_event(EntityEvent {
            entity_id: 42,
            event_id: 25,
        }));
        input.apply_screen_requests(&mut world, &mut counters, &commands);
        assert!(input.game_rules_screen().is_none());
    }

    #[test]
    fn permission_loss_under_a_dialog_keeps_the_game_rules_screen_closed() {
        let mut world = operator_world();
        let mut input = ClientInputState::default();
        let mut counters = NetCounters::default();
        open_loaded_screen(&mut input, &mut world);
        show_notice(&mut world);
        input.apply_screen_requests(&mut world, &mut counters, &None);

        assert!(world.apply_entity_event(EntityEvent {
            entity_id: 42,
            event_id: 25,
        }));
        input.apply_screen_requests(&mut world, &mut counters, &None);
        assert!(input.close_dialog());
        input.apply_screen_requests(&mut world, &mut counters, &None);
        assert!(input.game_rules_screen().is_none());
    }
}
