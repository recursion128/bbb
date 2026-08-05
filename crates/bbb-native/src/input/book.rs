use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::packets::EditBook;
use bbb_world::{
    BookScreenState, WorldStore, MAX_BOOK_PAGE_UTF16_UNITS, MAX_BOOK_TITLE_UTF16_UNITS,
};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
};

use super::{
    commands::queue_container_close_command, copy_selection_to_clipboard, queue_edit_book_command,
    queue_lectern_button_click, text_edit, ClientInputState, CommandSuggestionTextMeasurer,
    DebugClipboard,
};

pub(crate) const BOOK_SCREEN_WIDTH: i32 = 192;
pub(crate) const BOOK_PAGE_BUTTON_Y: i32 = 157;
pub(crate) const BOOK_PAGE_BACK_BUTTON_X: i32 = 43;
pub(crate) const BOOK_PAGE_FORWARD_BUTTON_X: i32 = 116;
pub(crate) const BOOK_PAGE_BUTTON_WIDTH: i32 = 23;
pub(crate) const BOOK_PAGE_BUTTON_HEIGHT: i32 = 13;
pub(crate) const BOOK_MENU_BUTTON_Y: i32 = 194;
pub(crate) const BOOK_MENU_DONE_BUTTON_X: i32 = -4;
pub(crate) const BOOK_MENU_BUTTON_WIDTH: i32 = 200;
pub(crate) const BOOK_MENU_BUTTON_HEIGHT: i32 = 20;

const BOOK_MENU_LEFT_BUTTON_WIDTH: i32 = 98;
const BOOK_MENU_RIGHT_BUTTON_X: i32 = 98;
const BOOK_EDIT_TEXT_X: i32 = 35;
const BOOK_EDIT_TEXT_Y: i32 = 30;
const BOOK_EDIT_TEXT_WIDTH: u32 = 114;
const BOOK_EDIT_LINE_HEIGHT: i32 = 9;
const BOOK_EDIT_LINE_LIMIT: usize = 14;
const BOOK_SIGN_TITLE_X: i32 = 36;
const BOOK_SIGN_TITLE_Y: i32 = 50;
const BOOK_SIGN_TITLE_WIDTH: u32 = 114;
const BOOK_SIGN_TITLE_HEIGHT: i32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BookInputMode {
    Edit { page: usize },
    Sign,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BookInputState {
    pub(crate) generation: u64,
    pub(crate) mode: BookInputMode,
    pub(crate) cursor: usize,
    pub(crate) selection: usize,
    pub(crate) dragging: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BookTextLine {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BookClickTarget {
    Done,
    Sign,
    Finalize,
    Cancel,
    PreviousPage,
    NextPage,
    EditText,
    SignTitle,
}

impl ClientInputState {
    pub(crate) fn book_input_state(&self) -> Option<&BookInputState> {
        self.book_input.as_ref()
    }

    pub(crate) fn sync_book_screen(&mut self, world: &WorldStore) {
        let Some(book) = world.current_book() else {
            self.book_input = None;
            return;
        };
        let (mode, text_len) = match book {
            BookScreenState::View(_) => {
                self.book_input = None;
                return;
            }
            BookScreenState::Edit(screen) => (
                BookInputMode::Edit {
                    page: screen.current_page,
                },
                screen
                    .pages
                    .get(screen.current_page)
                    .map_or(0, |page| text_edit::char_len(page)),
            ),
            BookScreenState::Sign(screen) => {
                (BookInputMode::Sign, text_edit::char_len(&screen.title))
            }
        };
        let signature_matches = self
            .book_input
            .as_ref()
            .is_some_and(|input| input.generation == book.generation() && input.mode == mode);
        if signature_matches {
            if let Some(input) = &mut self.book_input {
                input.cursor = input.cursor.min(text_len);
                input.selection = input.selection.min(text_len);
            }
        } else {
            self.book_input = Some(BookInputState {
                generation: book.generation(),
                mode,
                cursor: text_len,
                selection: text_len,
                dragging: false,
            });
        }
    }

    pub(crate) fn handle_book_screen_key(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        code: KeyCode,
        pressed: bool,
        measurer: &impl CommandSuggestionTextMeasurer,
        clipboard: &mut Option<&mut dyn DebugClipboard>,
    ) -> bool {
        if world.current_book().is_none() {
            self.book_input = None;
            let Some(lectern) = world.lectern_book_view() else {
                return false;
            };
            if !pressed {
                return true;
            }
            match code {
                KeyCode::Escape | KeyCode::KeyE => {
                    queue_container_close_command(counters, world, net_commands);
                }
                KeyCode::PageUp if lectern.current_page > 0 => {
                    queue_lectern_button_click(world, counters, net_commands, 1);
                }
                KeyCode::PageDown
                    if lectern.current_page.saturating_add(1) < lectern.pages.len() =>
                {
                    queue_lectern_button_click(world, counters, net_commands, 2);
                }
                _ => {}
            }
            return true;
        }
        self.sync_book_screen(world);
        if !pressed {
            return true;
        }

        let mode = world.current_book().map(|book| match book {
            BookScreenState::View(_) => None,
            BookScreenState::Edit(_) => Some(BookInputMode::Edit {
                page: book.current_page(),
            }),
            BookScreenState::Sign(_) => Some(BookInputMode::Sign),
        });
        match mode {
            Some(None) => match code {
                KeyCode::Escape | KeyCode::KeyE => {
                    world.close_current_book();
                }
                KeyCode::PageUp => {
                    world.turn_current_book_page(-1);
                }
                KeyCode::PageDown => {
                    world.turn_current_book_page(1);
                }
                _ => {}
            },
            Some(Some(BookInputMode::Edit { .. })) => {
                let control_down = self.control_down();
                let shift_down = self.shift_down();
                match code {
                    KeyCode::Escape | KeyCode::KeyE => {
                        world.close_current_book();
                        self.book_input = None;
                    }
                    KeyCode::PageUp => {
                        world.turn_current_book_page(-1);
                        self.sync_book_screen(world);
                    }
                    KeyCode::PageDown => {
                        world.turn_current_book_page(1);
                        self.sync_book_screen(world);
                    }
                    KeyCode::KeyA if self.text_edit_shortcut_down() => {
                        self.select_all_book_text(world)
                    }
                    KeyCode::KeyC if self.text_edit_shortcut_down() => {
                        if let Some(selected) = self.selected_book_text(world) {
                            copy_selection_to_clipboard(clipboard, &selected);
                        }
                    }
                    KeyCode::KeyX if self.text_edit_shortcut_down() => {
                        if self.selected_book_text(world).is_some_and(|selected| {
                            copy_selection_to_clipboard(clipboard, &selected)
                        }) {
                            self.replace_book_selection(world, "", measurer);
                        }
                    }
                    KeyCode::KeyV if self.text_edit_shortcut_down() => {
                        if let Some(text) = clipboard
                            .as_deref_mut()
                            .and_then(DebugClipboard::get_debug_clipboard_text)
                        {
                            self.replace_book_selection(world, &text, measurer);
                        }
                    }
                    KeyCode::Enter | KeyCode::NumpadEnter => {
                        self.replace_book_selection(world, "\n", measurer);
                    }
                    KeyCode::Backspace => {
                        self.delete_book_text(world, -1, control_down, measurer);
                    }
                    KeyCode::Delete => {
                        self.delete_book_text(world, 1, control_down, measurer);
                    }
                    KeyCode::ArrowLeft => {
                        self.move_book_cursor_horizontal(world, -1, control_down, shift_down);
                    }
                    KeyCode::ArrowRight => {
                        self.move_book_cursor_horizontal(world, 1, control_down, shift_down);
                    }
                    KeyCode::ArrowUp => {
                        self.move_book_cursor_line(world, -1, shift_down, measurer);
                    }
                    KeyCode::ArrowDown => {
                        self.move_book_cursor_line(world, 1, shift_down, measurer);
                    }
                    KeyCode::Home => {
                        self.move_book_cursor_home_end(
                            world,
                            false,
                            control_down,
                            shift_down,
                            measurer,
                        );
                    }
                    KeyCode::End => {
                        self.move_book_cursor_home_end(
                            world,
                            true,
                            control_down,
                            shift_down,
                            measurer,
                        );
                    }
                    _ => {}
                }
            }
            Some(Some(BookInputMode::Sign)) => {
                let control_down = self.control_down();
                let shift_down = self.shift_down();
                match code {
                    KeyCode::Escape | KeyCode::KeyE => {
                        world.close_current_book();
                        self.book_input = None;
                    }
                    KeyCode::Enter | KeyCode::NumpadEnter => {
                        self.submit_signed_book(counters, world, net_commands);
                    }
                    KeyCode::KeyA if self.text_edit_shortcut_down() => {
                        self.select_all_book_text(world)
                    }
                    KeyCode::KeyC if self.text_edit_shortcut_down() => {
                        if let Some(selected) = self.selected_book_text(world) {
                            copy_selection_to_clipboard(clipboard, &selected);
                        }
                    }
                    KeyCode::KeyX if self.text_edit_shortcut_down() => {
                        if self.selected_book_text(world).is_some_and(|selected| {
                            copy_selection_to_clipboard(clipboard, &selected)
                        }) {
                            self.replace_book_selection(world, "", measurer);
                        }
                    }
                    KeyCode::KeyV if self.text_edit_shortcut_down() => {
                        if let Some(text) = clipboard
                            .as_deref_mut()
                            .and_then(DebugClipboard::get_debug_clipboard_text)
                        {
                            self.replace_book_selection(world, &text, measurer);
                        }
                    }
                    KeyCode::Backspace => {
                        self.delete_book_text(world, -1, control_down, measurer);
                    }
                    KeyCode::Delete => {
                        self.delete_book_text(world, 1, control_down, measurer);
                    }
                    KeyCode::ArrowLeft => {
                        self.move_book_cursor_horizontal(world, -1, control_down, shift_down);
                    }
                    KeyCode::ArrowRight => {
                        self.move_book_cursor_horizontal(world, 1, control_down, shift_down);
                    }
                    KeyCode::Home => self.move_book_cursor_absolute(world, 0, shift_down),
                    KeyCode::End => {
                        let end = current_book_text(world).map_or(0, text_edit::char_len);
                        self.move_book_cursor_absolute(world, end, shift_down);
                    }
                    _ => {}
                }
            }
            None => {}
        }
        true
    }

    pub(crate) fn handle_book_screen_text(
        &mut self,
        world: &mut WorldStore,
        text: &str,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) -> bool {
        if world.current_book().is_none() {
            self.book_input = None;
            return false;
        }
        self.sync_book_screen(world);
        if self.book_input.is_some() {
            self.replace_book_selection(world, text, measurer);
        }
        true
    }

    pub(crate) fn handle_book_screen_mouse_input(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) -> bool {
        if world.current_book().is_none() {
            self.book_input = None;
            return false;
        }
        self.sync_book_screen(world);
        if button != MouseButton::Left {
            return true;
        }
        if state == ElementState::Released {
            if let Some(input) = &mut self.book_input {
                input.dragging = false;
            }
            return true;
        }
        match book_click_target(world, cursor_position, surface_size) {
            Some(BookClickTarget::Done) => {
                if world.current_book().is_some_and(BookScreenState::is_edit) {
                    self.submit_unsigned_book(counters, world, net_commands);
                } else {
                    world.close_current_book();
                    self.book_input = None;
                }
            }
            Some(BookClickTarget::Sign) => {
                let generation = world.current_book().map(BookScreenState::generation);
                if generation.is_some_and(|generation| world.begin_sign_current_book(generation)) {
                    self.sync_book_screen(world);
                }
            }
            Some(BookClickTarget::Finalize) => {
                self.submit_signed_book(counters, world, net_commands);
            }
            Some(BookClickTarget::Cancel) => {
                let generation = world.current_book().map(BookScreenState::generation);
                if generation.is_some_and(|generation| world.cancel_sign_current_book(generation)) {
                    self.sync_book_screen(world);
                }
            }
            Some(BookClickTarget::PreviousPage) => {
                world.turn_current_book_page(-1);
                self.sync_book_screen(world);
            }
            Some(BookClickTarget::NextPage) => {
                world.turn_current_book_page(1);
                self.sync_book_screen(world);
            }
            Some(BookClickTarget::EditText) | Some(BookClickTarget::SignTitle) => {
                self.seek_book_cursor_to_point(
                    world,
                    cursor_position,
                    surface_size,
                    false,
                    measurer,
                );
                if let Some(input) = &mut self.book_input {
                    input.dragging = true;
                }
            }
            None => {}
        }
        true
    }

    pub(crate) fn handle_book_screen_cursor_moved(
        &mut self,
        world: &WorldStore,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) -> bool {
        if world.current_book().is_none() {
            self.book_input = None;
            return false;
        }
        let dragging = self.book_input.as_ref().is_some_and(|input| input.dragging);
        if dragging {
            self.seek_book_cursor_to_point(world, cursor_position, surface_size, true, measurer);
        }
        true
    }

    fn replace_book_selection(
        &mut self,
        world: &mut WorldStore,
        inserted: &str,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) {
        let Some(input) = self.book_input.as_ref() else {
            return;
        };
        let Some(current) = current_book_text(world) else {
            return;
        };
        let generation = input.generation;
        let mode = input.mode;
        let start = input
            .cursor
            .min(input.selection)
            .min(text_edit::char_len(current));
        let end = input
            .cursor
            .max(input.selection)
            .min(text_edit::char_len(current));
        let start_byte = text_edit::byte_index(current, start);
        let end_byte = text_edit::byte_index(current, end);
        let retained_units = current[..start_byte]
            .encode_utf16()
            .count()
            .saturating_add(current[end_byte..].encode_utf16().count());
        let (allow_newline, limit) = match mode {
            BookInputMode::Edit { .. } => (true, MAX_BOOK_PAGE_UTF16_UNITS),
            BookInputMode::Sign => (false, MAX_BOOK_TITLE_UTF16_UNITS),
        };
        let mut remaining = limit.saturating_sub(retained_units);
        let mut filtered = String::new();
        for ch in inserted
            .chars()
            .filter(|ch| allowed_book_char(*ch, allow_newline))
        {
            let units = ch.len_utf16();
            if units > remaining {
                break;
            }
            filtered.push(ch);
            remaining -= units;
        }
        let mut candidate = String::with_capacity(current.len() + filtered.len());
        candidate.push_str(&current[..start_byte]);
        candidate.push_str(&filtered);
        candidate.push_str(&current[end_byte..]);
        if mode != BookInputMode::Sign
            && book_text_lines(&candidate, measurer).len() > BOOK_EDIT_LINE_LIMIT
        {
            return;
        }
        if candidate == current {
            return;
        }
        let changed = match mode {
            BookInputMode::Edit { .. } => world.replace_current_book_page(generation, candidate),
            BookInputMode::Sign => world.replace_current_book_title(generation, candidate),
        };
        if changed {
            let cursor = start + text_edit::char_len(&filtered);
            if let Some(input) = &mut self.book_input {
                input.cursor = cursor;
                input.selection = cursor;
            }
        }
    }

    fn delete_book_text(
        &mut self,
        world: &mut WorldStore,
        direction: i32,
        by_word: bool,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) {
        let Some(input) = self.book_input.as_ref() else {
            return;
        };
        let Some(current) = current_book_text(world) else {
            return;
        };
        if input.cursor != input.selection {
            self.replace_book_selection(world, "", measurer);
            return;
        }
        let cursor = input.cursor.min(text_edit::char_len(current));
        let selection = if by_word {
            text_edit::word_position(current, cursor, direction)
        } else if direction < 0 {
            cursor.saturating_sub(1)
        } else {
            (cursor + 1).min(text_edit::char_len(current))
        };
        if selection == cursor {
            return;
        }
        if let Some(input) = &mut self.book_input {
            input.selection = selection;
        }
        self.replace_book_selection(world, "", measurer);
    }

    fn select_all_book_text(&mut self, world: &WorldStore) {
        let Some(length) = current_book_text(world).map(text_edit::char_len) else {
            return;
        };
        if let Some(input) = &mut self.book_input {
            input.selection = 0;
            input.cursor = length;
        }
    }

    fn selected_book_text(&self, world: &WorldStore) -> Option<String> {
        let input = self.book_input.as_ref()?;
        let text = current_book_text(world)?;
        let start = input
            .cursor
            .min(input.selection)
            .min(text_edit::char_len(text));
        let end = input
            .cursor
            .max(input.selection)
            .min(text_edit::char_len(text));
        Some(text[text_edit::byte_index(text, start)..text_edit::byte_index(text, end)].to_string())
    }

    fn move_book_cursor_horizontal(
        &mut self,
        world: &WorldStore,
        direction: i32,
        by_word: bool,
        selecting: bool,
    ) {
        let Some(input) = self.book_input.as_ref() else {
            return;
        };
        let Some(text) = current_book_text(world) else {
            return;
        };
        let cursor = input.cursor.min(text_edit::char_len(text));
        let target = if by_word {
            text_edit::word_position(text, cursor, direction)
        } else if direction < 0 {
            cursor.saturating_sub(1)
        } else {
            (cursor + 1).min(text_edit::char_len(text))
        };
        self.move_book_cursor_absolute(world, target, selecting);
    }

    fn move_book_cursor_line(
        &mut self,
        world: &WorldStore,
        direction: i32,
        selecting: bool,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) {
        let Some(input) = self.book_input.as_ref() else {
            return;
        };
        let Some(text) = current_book_text(world) else {
            return;
        };
        let lines = book_text_lines(text, measurer);
        let Some(line_index) = line_at_cursor(&lines, input.cursor) else {
            return;
        };
        let target_line =
            (line_index as i32 + direction).clamp(0, lines.len().saturating_sub(1) as i32) as usize;
        let current_line = lines[line_index];
        let target = lines[target_line];
        let prefix = char_slice(text, current_line.start, input.cursor);
        let desired_width = measurer
            .command_suggestion_text_width(prefix)
            .saturating_add(2);
        let target_text = char_slice(text, target.start, target.end);
        let column = measurer.command_suggestion_cursor_for_width(target_text, 0, desired_width);
        self.move_book_cursor_absolute(
            world,
            target.start + column.min(target.end - target.start),
            selecting,
        );
    }

    fn move_book_cursor_home_end(
        &mut self,
        world: &WorldStore,
        end: bool,
        whole_text: bool,
        selecting: bool,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) {
        let Some(input) = self.book_input.as_ref() else {
            return;
        };
        let Some(text) = current_book_text(world) else {
            return;
        };
        let target = if whole_text {
            if end {
                text_edit::char_len(text)
            } else {
                0
            }
        } else {
            let lines = book_text_lines(text, measurer);
            let Some(line_index) = line_at_cursor(&lines, input.cursor) else {
                return;
            };
            if end {
                lines[line_index].end
            } else {
                lines[line_index].start
            }
        };
        self.move_book_cursor_absolute(world, target, selecting);
    }

    fn move_book_cursor_absolute(&mut self, world: &WorldStore, target: usize, selecting: bool) {
        let Some(length) = current_book_text(world).map(text_edit::char_len) else {
            return;
        };
        if let Some(input) = &mut self.book_input {
            input.cursor = target.min(length);
            if !selecting {
                input.selection = input.cursor;
            }
        }
    }

    fn seek_book_cursor_to_point(
        &mut self,
        world: &WorldStore,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        selecting: bool,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) {
        let Some(cursor_position) = cursor_position else {
            return;
        };
        let Some(input) = self.book_input.as_ref() else {
            return;
        };
        let Some(text) = current_book_text(world) else {
            return;
        };
        let (origin_x, origin_y) = book_screen_origin(surface_size);
        let (text_x, text_y, single_line) = match input.mode {
            BookInputMode::Edit { .. } => (
                origin_x + f64::from(BOOK_EDIT_TEXT_X),
                origin_y + f64::from(BOOK_EDIT_TEXT_Y),
                false,
            ),
            BookInputMode::Sign => (
                origin_x + f64::from(BOOK_SCREEN_WIDTH) * 0.5
                    - f64::from(measurer.command_suggestion_text_width(text)) * 0.5,
                f64::from(BOOK_SIGN_TITLE_Y),
                true,
            ),
        };
        let lines = if single_line {
            vec![BookTextLine {
                start: 0,
                end: text_edit::char_len(text),
            }]
        } else {
            book_text_lines(text, measurer)
        };
        let line = if single_line {
            0
        } else {
            ((cursor_position.y - text_y) / f64::from(BOOK_EDIT_LINE_HEIGHT))
                .floor()
                .clamp(0.0, lines.len().saturating_sub(1) as f64) as usize
        };
        let line = lines[line];
        let line_text = char_slice(text, line.start, line.end);
        let width = (cursor_position.x - text_x).floor().max(0.0) as u32;
        let column = measurer.command_suggestion_cursor_for_width(line_text, 0, width);
        self.move_book_cursor_absolute(
            world,
            line.start + column.min(line.end - line.start),
            selecting,
        );
    }

    fn submit_unsigned_book(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        let Some(submission) = world.current_book_unsigned_submission() else {
            return;
        };
        let command = EditBook {
            slot: submission.slot,
            pages: submission.pages.clone(),
            title: None,
        };
        if queue_edit_book_command(counters, net_commands, command)
            && world.commit_current_book_submission(&submission)
        {
            self.book_input = None;
        }
    }

    fn submit_signed_book(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        let Some(submission) = world.current_book_signed_submission() else {
            return;
        };
        let command = EditBook {
            slot: submission.slot,
            pages: submission.pages.clone(),
            title: submission.title.clone(),
        };
        if queue_edit_book_command(counters, net_commands, command)
            && world.commit_current_book_submission(&submission)
        {
            self.book_input = None;
        }
    }
}

#[cfg(test)]
pub(crate) fn handle_book_screen_mouse_input(
    world: &mut WorldStore,
    button: MouseButton,
    state: ElementState,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if world.current_book().is_none() {
        return false;
    }
    if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
        return true;
    }
    match book_click_target(world, cursor_position, surface_size) {
        Some(BookClickTarget::Done) => {
            world.close_current_book();
        }
        Some(BookClickTarget::PreviousPage) => {
            world.turn_current_book_page(-1);
        }
        Some(BookClickTarget::NextPage) => {
            world.turn_current_book_page(1);
        }
        _ => {}
    }
    true
}

fn current_book_text(world: &WorldStore) -> Option<&str> {
    match world.current_book()? {
        BookScreenState::Edit(screen) => screen.pages.get(screen.current_page).map(String::as_str),
        BookScreenState::Sign(screen) => Some(&screen.title),
        BookScreenState::View(_) => None,
    }
}

fn allowed_book_char(ch: char, allow_newline: bool) -> bool {
    (ch >= ' ' && ch != '\u{7f}' && ch != '\u{a7}') || (allow_newline && ch == '\n')
}

fn book_text_lines(text: &str, measurer: &impl CommandSuggestionTextMeasurer) -> Vec<BookTextLine> {
    let length = text_edit::char_len(text);
    if length == 0 {
        return vec![BookTextLine { start: 0, end: 0 }];
    }
    let chars = text.chars().collect::<Vec<_>>();
    let mut lines = Vec::new();
    let mut start = 0;
    while start < length {
        let mut cursor = start;
        let mut last_space = None;
        let mut split = length;
        while cursor < length {
            let ch = chars[cursor];
            if ch == '\n' {
                split = cursor;
                break;
            }
            if ch == ' ' {
                last_space = Some(cursor);
            }
            let width = measurer.command_suggestion_text_width(char_slice(text, start, cursor + 1));
            if cursor > start && width > BOOK_EDIT_TEXT_WIDTH {
                split = last_space.filter(|space| *space >= start).unwrap_or(cursor);
                break;
            }
            cursor += 1;
        }
        lines.push(BookTextLine { start, end: split });
        if split == length {
            break;
        }
        start = if matches!(chars[split], '\n' | ' ') {
            split + 1
        } else {
            split
        };
    }
    if text.ends_with('\n') {
        lines.push(BookTextLine {
            start: length,
            end: length,
        });
    }
    lines
}

fn line_at_cursor(lines: &[BookTextLine], cursor: usize) -> Option<usize> {
    lines
        .iter()
        .position(|line| cursor >= line.start && cursor <= line.end)
        .or_else(|| (!lines.is_empty()).then_some(lines.len() - 1))
}

fn char_slice(text: &str, start: usize, end: usize) -> &str {
    &text[text_edit::byte_index(text, start)..text_edit::byte_index(text, end)]
}

fn book_click_target(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<BookClickTarget> {
    let book = world.current_book()?;
    let cursor = cursor_position?;
    let (origin_x, origin_y) = book_screen_origin(surface_size);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    let in_menu_y = y >= f64::from(BOOK_MENU_BUTTON_Y)
        && y < f64::from(BOOK_MENU_BUTTON_Y + BOOK_MENU_BUTTON_HEIGHT);
    match book {
        BookScreenState::View(_) if in_menu_y => {
            if x >= f64::from(BOOK_MENU_DONE_BUTTON_X)
                && x < f64::from(BOOK_MENU_DONE_BUTTON_X + BOOK_MENU_BUTTON_WIDTH)
            {
                return Some(BookClickTarget::Done);
            }
        }
        BookScreenState::Edit(_) if in_menu_y => {
            if x >= f64::from(BOOK_MENU_DONE_BUTTON_X)
                && x < f64::from(BOOK_MENU_DONE_BUTTON_X + BOOK_MENU_LEFT_BUTTON_WIDTH)
            {
                return Some(BookClickTarget::Sign);
            }
            if x >= f64::from(BOOK_MENU_RIGHT_BUTTON_X)
                && x < f64::from(BOOK_MENU_RIGHT_BUTTON_X + BOOK_MENU_LEFT_BUTTON_WIDTH)
            {
                return Some(BookClickTarget::Done);
            }
        }
        BookScreenState::Sign(screen) if in_menu_y => {
            if x >= f64::from(BOOK_MENU_DONE_BUTTON_X)
                && x < f64::from(BOOK_MENU_DONE_BUTTON_X + BOOK_MENU_LEFT_BUTTON_WIDTH)
                && !screen.title.trim().is_empty()
            {
                return Some(BookClickTarget::Finalize);
            }
            if x >= f64::from(BOOK_MENU_RIGHT_BUTTON_X)
                && x < f64::from(BOOK_MENU_RIGHT_BUTTON_X + BOOK_MENU_LEFT_BUTTON_WIDTH)
            {
                return Some(BookClickTarget::Cancel);
            }
        }
        _ => {}
    }
    if matches!(book, BookScreenState::Edit(_))
        && x >= f64::from(BOOK_EDIT_TEXT_X)
        && x < f64::from(BOOK_EDIT_TEXT_X) + f64::from(BOOK_EDIT_TEXT_WIDTH)
        && y >= f64::from(BOOK_EDIT_TEXT_Y)
        && y < f64::from(BOOK_EDIT_TEXT_Y + BOOK_EDIT_LINE_HEIGHT * BOOK_EDIT_LINE_LIMIT as i32)
    {
        return Some(BookClickTarget::EditText);
    }
    if matches!(book, BookScreenState::Sign(_))
        && x >= f64::from(BOOK_SIGN_TITLE_X)
        && x < f64::from(BOOK_SIGN_TITLE_X) + f64::from(BOOK_SIGN_TITLE_WIDTH)
        && cursor.y >= f64::from(BOOK_SIGN_TITLE_Y)
        && cursor.y < f64::from(BOOK_SIGN_TITLE_Y + BOOK_SIGN_TITLE_HEIGHT)
    {
        return Some(BookClickTarget::SignTitle);
    }
    if matches!(book, BookScreenState::Sign(_))
        || y < f64::from(BOOK_PAGE_BUTTON_Y)
        || y >= f64::from(BOOK_PAGE_BUTTON_Y + BOOK_PAGE_BUTTON_HEIGHT)
    {
        return None;
    }
    if book.current_page() > 0
        && x >= f64::from(BOOK_PAGE_BACK_BUTTON_X)
        && x < f64::from(BOOK_PAGE_BACK_BUTTON_X + BOOK_PAGE_BUTTON_WIDTH)
    {
        return Some(BookClickTarget::PreviousPage);
    }
    let next_visible = match book {
        BookScreenState::View(_) => book.current_page() + 1 < book.page_count(),
        BookScreenState::Edit(_) => true,
        BookScreenState::Sign(_) => false,
    };
    if next_visible
        && x >= f64::from(BOOK_PAGE_FORWARD_BUTTON_X)
        && x < f64::from(BOOK_PAGE_FORWARD_BUTTON_X + BOOK_PAGE_BUTTON_WIDTH)
    {
        return Some(BookClickTarget::NextPage);
    }
    None
}

fn book_screen_origin(surface_size: PhysicalSize<u32>) -> (f64, f64) {
    (
        (f64::from(surface_size.width.max(1)) - f64::from(BOOK_SCREEN_WIDTH)) * 0.5,
        2.0,
    )
}

#[cfg(test)]
mod tests;
