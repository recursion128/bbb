use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_renderer::{
    hud_game_rules_confirmation_buttons, hud_game_rules_layout, HUD_GAME_RULES_ROW_HEIGHT,
    HUD_GAME_RULES_VALUE_HEIGHT, HUD_GAME_RULES_VALUE_WIDTH,
};
use bbb_world::{GameRuleEditValueState, WorldStore};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::KeyCode,
};

use super::{
    physical_position_floor_i32, queue_set_game_rule_command, text_edit, ClientInputState,
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
    text: String,
    cursor: usize,
    selection: usize,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct GameRulesScreenInputState {
    generation: u64,
    search_text: String,
    search_cursor: usize,
    search_selection: usize,
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

impl ClientInputState {
    pub(crate) fn sync_game_rules_screen(&mut self, world: &WorldStore) {
        let Some(screen) = world.game_rules_screen() else {
            self.game_rules_screen = None;
            return;
        };
        if self
            .game_rules_screen
            .as_ref()
            .is_some_and(|input| input.generation == screen.generation)
        {
            return;
        }
        self.game_rules_screen = Some(GameRulesScreenInputState {
            generation: screen.generation,
            focus: GameRulesScreenFocus::Search,
            ..GameRulesScreenInputState::default()
        });
    }

    pub(crate) fn sync_game_rules_screen_projection(
        &mut self,
        world: &WorldStore,
        entries: Vec<GameRuleSearchEntry>,
        surface_size: PhysicalSize<u32>,
    ) {
        self.sync_game_rules_screen(world);
        let Some(state) = self.game_rules_screen.as_mut() else {
            return;
        };
        state.surface_size = Some(surface_size);
        state.search_entries = entries;
        state.rebuild_rows(surface_size);
        state.sync_integer_input(world);
        state.validate_focus(world);
    }

    /// Whether a game-rules text field would take a commit right now: the
    /// buttons are not text, and a discard confirmation owns the keyboard.
    pub(crate) fn game_rules_text_is_focused(&self, world: &WorldStore) -> bool {
        if world
            .game_rules_screen()
            .is_none_or(|screen| screen.discard_confirmation_open)
        {
            return false;
        }
        self.game_rules_screen.as_ref().is_some_and(|state| {
            matches!(
                state.focus,
                GameRulesScreenFocus::Search | GameRulesScreenFocus::Rule(_)
            )
        })
    }

    pub(crate) fn game_rules_screen_input(&self) -> Option<&GameRulesScreenInputState> {
        self.game_rules_screen.as_ref()
    }

    pub(crate) fn handle_game_rules_screen_key(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        code: KeyCode,
        pressed: bool,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        self.sync_game_rules_screen(world);
        if self.game_rules_screen.is_none() {
            return false;
        }
        if let Some(state) = self.game_rules_screen.as_mut() {
            state.surface_size = Some(surface_size);
        }
        if !pressed {
            return true;
        }
        if world
            .game_rules_screen()
            .is_some_and(|screen| screen.discard_confirmation_open)
        {
            match code {
                KeyCode::Escape => {
                    world.cancel_game_rules_screen_discard();
                    if let Some(state) = self.game_rules_screen.as_mut() {
                        state.focus = GameRulesScreenFocus::Cancel;
                    }
                }
                KeyCode::Tab | KeyCode::ArrowLeft | KeyCode::ArrowRight => {
                    if let Some(state) = self.game_rules_screen.as_mut() {
                        state.focus = match state.focus {
                            GameRulesScreenFocus::Discard => GameRulesScreenFocus::KeepEditing,
                            _ => GameRulesScreenFocus::Discard,
                        };
                    }
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                    let discard = self
                        .game_rules_screen
                        .as_ref()
                        .is_some_and(|state| matches!(state.focus, GameRulesScreenFocus::Discard));
                    if discard {
                        world.confirm_game_rules_screen_discard();
                        self.game_rules_screen = None;
                    } else {
                        world.cancel_game_rules_screen_discard();
                    }
                }
                KeyCode::KeyY => {
                    world.confirm_game_rules_screen_discard();
                    self.game_rules_screen = None;
                }
                KeyCode::KeyN => {
                    world.cancel_game_rules_screen_discard();
                }
                _ => {}
            }
            return true;
        }

        if matches!(code, KeyCode::Escape) {
            world.request_close_game_rules_screen();
            if world.game_rules_screen().is_none() {
                self.game_rules_screen = None;
            } else if let Some(state) = self.game_rules_screen.as_mut() {
                state.focus = GameRulesScreenFocus::KeepEditing;
            }
            return true;
        }
        if matches!(code, KeyCode::Tab) {
            let forward = !self.shift_down();
            if let Some(state) = self.game_rules_screen.as_mut() {
                state.move_focus(world, forward);
                state.ensure_focused_rule_visible(surface_size);
            }
            return true;
        }
        if matches!(code, KeyCode::ArrowUp | KeyCode::ArrowDown)
            && !self.game_rules_text_field_is_focused()
        {
            let forward = matches!(code, KeyCode::ArrowDown);
            if let Some(state) = self.game_rules_screen.as_mut() {
                state.move_focus(world, forward);
                state.ensure_focused_rule_visible(surface_size);
            }
            return true;
        }
        if matches!(code, KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space) {
            self.activate_game_rules_screen_focus(counters, world, net_commands);
            return true;
        }

        let shift_down = self.shift_down();
        let control_down = self.control_down();
        let Some(state) = self.game_rules_screen.as_mut() else {
            return true;
        };
        match state.focus.clone() {
            GameRulesScreenFocus::Search => {
                if edit_text_key(
                    &mut state.search_text,
                    &mut state.search_cursor,
                    &mut state.search_selection,
                    code,
                    shift_down,
                    control_down,
                ) {
                    state.rebuild_rows(surface_size);
                    state.validate_focus(world);
                }
            }
            GameRulesScreenFocus::Rule(rule) => {
                if let Some(integer) = state
                    .integer_input
                    .as_mut()
                    .filter(|integer| integer.rule == rule)
                {
                    if edit_text_key(
                        &mut integer.text,
                        &mut integer.cursor,
                        &mut integer.selection,
                        code,
                        shift_down,
                        control_down,
                    ) {
                        world.set_game_rule_integer_input(&rule, integer.text.clone());
                    }
                }
            }
            _ => {}
        }
        true
    }

    pub(crate) fn handle_game_rules_screen_text(
        &mut self,
        world: &mut WorldStore,
        text: &str,
    ) -> bool {
        self.sync_game_rules_screen(world);
        if world
            .game_rules_screen()
            .is_none_or(|screen| screen.discard_confirmation_open)
        {
            return false;
        }
        let Some(state) = self.game_rules_screen.as_mut() else {
            return false;
        };
        let surface_size = state
            .surface_size
            .unwrap_or_else(|| PhysicalSize::new(1280, 720));
        match state.focus.clone() {
            GameRulesScreenFocus::Search => {
                if insert_text(
                    &mut state.search_text,
                    &mut state.search_cursor,
                    &mut state.search_selection,
                    text,
                    256,
                ) {
                    state.rebuild_rows(surface_size);
                    state.validate_focus(world);
                }
            }
            GameRulesScreenFocus::Rule(rule) => {
                let Some(integer) = state
                    .integer_input
                    .as_mut()
                    .filter(|integer| integer.rule == rule)
                else {
                    return true;
                };
                if insert_text(
                    &mut integer.text,
                    &mut integer.cursor,
                    &mut integer.selection,
                    text,
                    64,
                ) {
                    world.set_game_rule_integer_input(&rule, integer.text.clone());
                }
            }
            _ => {}
        }
        true
    }

    pub(crate) fn handle_game_rules_screen_cursor_moved(
        &mut self,
        world: &WorldStore,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        self.sync_game_rules_screen(world);
        let Some(state) = self.game_rules_screen.as_mut() else {
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

    pub(crate) fn handle_game_rules_screen_mouse_wheel(
        &mut self,
        world: &WorldStore,
        delta: MouseScrollDelta,
    ) -> bool {
        self.sync_game_rules_screen(world);
        let Some(state) = self.game_rules_screen.as_mut() else {
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
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: MouseButton,
        element_state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        self.sync_game_rules_screen(world);
        let Some(state) = self.game_rules_screen.as_mut() else {
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
        if world
            .game_rules_screen()
            .is_some_and(|screen| screen.discard_confirmation_open)
        {
            let (discard, keep_editing) = hud_game_rules_confirmation_buttons(surface_size);
            if discard.contains(state.cursor_position) {
                world.confirm_game_rules_screen_discard();
                self.game_rules_screen = None;
            } else if keep_editing.contains(state.cursor_position) {
                world.cancel_game_rules_screen_discard();
                if let Some(state) = self.game_rules_screen.as_mut() {
                    state.focus = GameRulesScreenFocus::KeepEditing;
                }
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
            self.activate_game_rules_screen_focus(counters, world, net_commands);
            return true;
        }
        if layout.cancel.contains(state.cursor_position) {
            state.focus = GameRulesScreenFocus::Cancel;
            self.activate_game_rules_screen_focus(counters, world, net_commands);
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
        let value_rect = state.value_rect_for_row(row_index, surface_size);
        if !value_rect.contains(state.cursor_position) {
            return true;
        }
        state.focus = GameRulesScreenFocus::Rule(rule.clone());
        let edit_value = world
            .game_rules_screen()
            .and_then(|screen| screen.rules.iter().find(|edit| edit.rule == rule))
            .map(|edit| edit.current_value.clone());
        match edit_value {
            Some(GameRuleEditValueState::Boolean { value }) => {
                world.set_game_rule_boolean(&rule, !value);
                state.integer_input = None;
            }
            Some(GameRuleEditValueState::Integer { input, .. }) => {
                let cursor = text_edit::char_len(&input);
                state.integer_input = Some(GameRuleIntegerInput {
                    rule,
                    text: input,
                    cursor,
                    selection: cursor,
                });
            }
            None => {}
        }
        true
    }

    fn game_rules_text_field_is_focused(&self) -> bool {
        let Some(state) = self.game_rules_screen.as_ref() else {
            return false;
        };
        matches!(state.focus, GameRulesScreenFocus::Search)
            || matches!(
                &state.focus,
                GameRulesScreenFocus::Rule(rule)
                    if state.integer_input.as_ref().is_some_and(|input| input.rule == *rule)
            )
    }

    fn activate_game_rules_screen_focus(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        let focus = self
            .game_rules_screen
            .as_ref()
            .map(|state| state.focus.clone());
        match focus {
            Some(GameRulesScreenFocus::Rule(rule)) => {
                let value = world
                    .game_rules_screen()
                    .and_then(|screen| screen.rules.iter().find(|edit| edit.rule == rule))
                    .map(|edit| edit.current_value.clone());
                if let Some(GameRuleEditValueState::Boolean { value }) = value {
                    world.set_game_rule_boolean(&rule, !value);
                }
            }
            Some(GameRulesScreenFocus::Done) => {
                if let Some(command) = world.finish_game_rules_screen() {
                    queue_set_game_rule_command(counters, net_commands, command);
                    self.game_rules_screen = None;
                }
            }
            Some(GameRulesScreenFocus::Cancel) => {
                world.request_close_game_rules_screen();
                if world.game_rules_screen().is_none() {
                    self.game_rules_screen = None;
                } else if let Some(state) = self.game_rules_screen.as_mut() {
                    state.focus = GameRulesScreenFocus::KeepEditing;
                }
            }
            _ => {}
        }
    }
}

impl GameRulesScreenInputState {
    pub(crate) fn search_text(&self) -> &str {
        &self.search_text
    }

    pub(crate) fn search_cursor(&self) -> usize {
        self.search_cursor
    }

    pub(crate) fn search_selection(&self) -> usize {
        self.search_selection
    }

    pub(crate) fn focus(&self) -> &GameRulesScreenFocus {
        &self.focus
    }

    pub(crate) fn rows(&self) -> &[GameRulesInteractionRow] {
        &self.rows
    }

    pub(crate) fn integer_input(&self, rule: &str) -> Option<(&str, usize, usize)> {
        let input = self
            .integer_input
            .as_ref()
            .filter(|input| input.rule == rule)?;
        Some((&input.text, input.cursor, input.selection))
    }

    pub(crate) fn cursor_position(&self) -> Option<(i32, i32)> {
        self.cursor_position
    }

    pub(crate) fn scroll_amount(&self) -> i32 {
        self.scroll_amount as i32
    }

    pub(crate) fn max_scroll_amount(&self) -> i32 {
        self.max_scroll_amount as i32
    }

    pub(crate) fn scroller_height(&self) -> i32 {
        self.scroller_height
    }

    pub(crate) fn hovered_rule(&self, surface_size: PhysicalSize<u32>) -> Option<&str> {
        self.row_at_cursor(surface_size)
            .and_then(|row| self.rows.get(row))
            .and_then(GameRulesInteractionRow::rule)
    }

    fn rebuild_rows(&mut self, surface_size: PhysicalSize<u32>) {
        let query = self.search_text.to_lowercase();
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

    fn sync_integer_input(&mut self, world: &WorldStore) {
        let GameRulesScreenFocus::Rule(rule) = self.focus.clone() else {
            self.integer_input = None;
            return;
        };
        let Some(GameRuleEditValueState::Integer { input, .. }) = world
            .game_rules_screen()
            .and_then(|screen| screen.rules.iter().find(|edit| edit.rule == rule))
            .map(|edit| edit.current_value.clone())
        else {
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
        let cursor = text_edit::char_len(&input);
        self.integer_input = Some(GameRuleIntegerInput {
            rule,
            text: input,
            cursor,
            selection: cursor,
        });
    }

    fn validate_focus(&mut self, world: &WorldStore) {
        let valid = match &self.focus {
            GameRulesScreenFocus::Rule(rule) => self
                .rows
                .iter()
                .any(|row| row.rule() == Some(rule.as_str())),
            GameRulesScreenFocus::Done => world
                .game_rules_screen()
                .is_some_and(|screen| screen.can_submit()),
            _ => true,
        };
        if !valid {
            self.focus = GameRulesScreenFocus::Search;
            self.integer_input = None;
        }
    }

    fn move_focus(&mut self, world: &WorldStore, forward: bool) {
        let mut focuses = Vec::with_capacity(self.rows.len() + 3);
        focuses.push(GameRulesScreenFocus::Search);
        focuses.extend(self.rows.iter().filter_map(|row| {
            row.rule()
                .map(|rule| GameRulesScreenFocus::Rule(rule.to_string()))
        }));
        if world
            .game_rules_screen()
            .is_some_and(|screen| screen.can_submit())
        {
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
        self.sync_integer_input(world);
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

    fn value_rect_for_row(
        &self,
        row: usize,
        surface_size: PhysicalSize<u32>,
    ) -> bbb_renderer::HudGameRulesRect {
        let layout = hud_game_rules_layout(surface_size);
        bbb_renderer::HudGameRulesRect {
            x: layout.list.x + layout.list.width - HUD_GAME_RULES_VALUE_WIDTH - 5,
            y: layout.list.y + i32::try_from(row).unwrap_or(i32::MAX) * HUD_GAME_RULES_ROW_HEIGHT
                - self.scroll_amount as i32
                + 2,
            width: HUD_GAME_RULES_VALUE_WIDTH,
            height: HUD_GAME_RULES_VALUE_HEIGHT,
        }
    }
}

fn edit_text_key(
    text: &mut String,
    cursor: &mut usize,
    selection: &mut usize,
    code: KeyCode,
    shift_down: bool,
    control_down: bool,
) -> bool {
    match code {
        KeyCode::KeyA if control_down => {
            *cursor = text_edit::char_len(text);
            *selection = 0;
            false
        }
        KeyCode::ArrowLeft => {
            let next = if control_down {
                text_edit::word_position(text, *cursor, -1)
            } else {
                cursor.saturating_sub(1)
            };
            set_cursor(cursor, selection, next, shift_down, text);
            false
        }
        KeyCode::ArrowRight => {
            let next = if control_down {
                text_edit::word_position(text, *cursor, 1)
            } else {
                cursor.saturating_add(1)
            };
            set_cursor(cursor, selection, next, shift_down, text);
            false
        }
        KeyCode::Home => {
            set_cursor(cursor, selection, 0, shift_down, text);
            false
        }
        KeyCode::End => {
            set_cursor(
                cursor,
                selection,
                text_edit::char_len(text),
                shift_down,
                text,
            );
            false
        }
        KeyCode::Backspace => {
            if delete_selection(text, cursor, selection) {
                return true;
            }
            if control_down {
                text_edit::remove_word_before_cursor(text, cursor);
            } else if *cursor > 0 {
                let start = text_edit::byte_index(text, *cursor - 1);
                let end = text_edit::byte_index(text, *cursor);
                text.replace_range(start..end, "");
                *cursor -= 1;
            }
            *selection = *cursor;
            true
        }
        KeyCode::Delete => {
            if delete_selection(text, cursor, selection) {
                return true;
            }
            if control_down {
                text_edit::remove_word_at_cursor(text, *cursor);
            } else if *cursor < text_edit::char_len(text) {
                let start = text_edit::byte_index(text, *cursor);
                let end = text_edit::byte_index(text, *cursor + 1);
                text.replace_range(start..end, "");
            }
            *selection = *cursor;
            true
        }
        _ => false,
    }
}

fn insert_text(
    text: &mut String,
    cursor: &mut usize,
    selection: &mut usize,
    inserted: &str,
    max_chars: usize,
) -> bool {
    let inserted = inserted
        .chars()
        .filter(|ch| !ch.is_control())
        .take(max_chars)
        .collect::<String>();
    if inserted.is_empty() {
        return false;
    }
    delete_selection(text, cursor, selection);
    let remaining = max_chars.saturating_sub(text_edit::char_len(text));
    let inserted = inserted.chars().take(remaining).collect::<String>();
    if inserted.is_empty() {
        return false;
    }
    let byte = text_edit::byte_index(text, *cursor);
    text.insert_str(byte, &inserted);
    *cursor += text_edit::char_len(&inserted);
    *selection = *cursor;
    true
}

fn delete_selection(text: &mut String, cursor: &mut usize, selection: &mut usize) -> bool {
    if *cursor == *selection {
        return false;
    }
    let start = (*cursor).min(*selection);
    let end = (*cursor).max(*selection);
    let start_byte = text_edit::byte_index(text, start);
    let end_byte = text_edit::byte_index(text, end);
    text.replace_range(start_byte..end_byte, "");
    *cursor = start;
    *selection = start;
    true
}

fn set_cursor(cursor: &mut usize, selection: &mut usize, next: usize, selecting: bool, text: &str) {
    *cursor = next.min(text_edit::char_len(text));
    if !selecting {
        *selection = *cursor;
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
        world
    }

    fn open_loaded_screen(world: &mut WorldStore) {
        assert!(world.open_game_rules_screen());
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
        open_loaded_screen(&mut world);
        let mut input = ClientInputState::default();
        input.sync_game_rules_screen_projection(&world, entries(), PhysicalSize::new(854, 480));

        assert!(input.handle_game_rules_screen_text(&mut world, "after death",));
        let state = input.game_rules_screen_input().unwrap();
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
        open_loaded_screen(&mut world);
        let mut input = ClientInputState::default();
        input.sync_game_rules_screen_projection(&world, entries(), PhysicalSize::new(854, 480));
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();

        input.game_rules_screen.as_mut().unwrap().focus =
            GameRulesScreenFocus::Rule("minecraft:keep_inventory".to_string());
        input.activate_game_rules_screen_focus(&mut counters, &mut world, &commands);
        input.game_rules_screen.as_mut().unwrap().focus =
            GameRulesScreenFocus::Rule("minecraft:random_tick_speed".to_string());
        input
            .game_rules_screen
            .as_mut()
            .unwrap()
            .sync_integer_input(&world);
        assert!(input.handle_game_rules_screen_key(
            &mut counters,
            &mut world,
            &commands,
            KeyCode::KeyA,
            true,
            PhysicalSize::new(854, 480),
        ));
        input.control_left_down = true;
        assert!(input.handle_game_rules_screen_key(
            &mut counters,
            &mut world,
            &commands,
            KeyCode::KeyA,
            true,
            PhysicalSize::new(854, 480),
        ));
        input.control_left_down = false;
        assert!(input.handle_game_rules_screen_text(&mut world, "6",));
        input.game_rules_screen.as_mut().unwrap().focus = GameRulesScreenFocus::Done;
        input.activate_game_rules_screen_focus(&mut counters, &mut world, &commands);

        let NetCommand::SetGameRule(packet) = rx.try_recv().unwrap() else {
            panic!("set game rule");
        };
        assert_eq!(packet.entries.len(), 2);
        assert_eq!(counters.set_game_rule_commands_queued, 1);
        assert!(world.game_rules_screen().is_none());
    }

    #[test]
    fn escape_requires_confirmation_only_after_changes() {
        let mut world = operator_world();
        open_loaded_screen(&mut world);
        let mut input = ClientInputState::default();
        input.sync_game_rules_screen_projection(&world, entries(), PhysicalSize::new(854, 480));
        let mut counters = NetCounters::default();

        input.game_rules_screen.as_mut().unwrap().focus =
            GameRulesScreenFocus::Rule("minecraft:keep_inventory".to_string());
        input.activate_game_rules_screen_focus(&mut counters, &mut world, &None);
        assert!(input.handle_game_rules_screen_key(
            &mut counters,
            &mut world,
            &None,
            KeyCode::Escape,
            true,
            PhysicalSize::new(854, 480),
        ));
        assert!(world.game_rules_screen().unwrap().discard_confirmation_open);
        assert!(input.handle_game_rules_screen_key(
            &mut counters,
            &mut world,
            &None,
            KeyCode::KeyN,
            true,
            PhysicalSize::new(854, 480),
        ));
        assert!(!world.game_rules_screen().unwrap().discard_confirmation_open);
    }
}
