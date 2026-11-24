use crate::control::NetCounters;
use bbb_client_net::NetCommand;
use bbb_client_world::{
    BookScreenState, WorldStore, MAX_BOOK_PAGE_UTF16_UNITS, MAX_BOOK_TITLE_UTF16_UNITS,
};
use bbb_protocol::packets::{EditBook, InteractionHand};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
};

use super::{
    commands::queue_container_close_command,
    queue_edit_book_command, queue_lectern_button_click,
    screen::Screen,
    text_field::{self, MultilineTextField, TextField, TextFieldLayout, TextLineFont},
    ClientInputState, CommandSuggestionTextMeasurer, DebugClipboard,
};

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
const BOOK_EDIT_TEXT_WIDTH: i32 = 114;
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

/// `BookViewScreen`, `BookEditScreen` or `BookSignScreen` with its edit box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BookScreen {
    pub(crate) book: BookScreenState,
    pub(crate) input: Option<BookInputState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BookInputState {
    pub(crate) mode: BookInputMode,
    pub(crate) cursor: usize,
    pub(crate) selection: usize,
    pub(crate) dragging: bool,
    pub(crate) display_start: usize,
    pub(crate) focused_at: u64,
    pub(super) ime_preedit: Option<super::ime_preedit::ImePreeditState>,
}

/// The page box's font: `MultiLineEditBox` at the page width.
struct BookPageFont<'a>(&'a bbb_client_render::HudFontGlyphMap);

impl TextLineFont for BookPageFont<'_> {
    fn lines(&self, value: &[u16]) -> Vec<std::ops::Range<usize>> {
        bbb_client_render::hud_formatted_utf16_line_ranges(
            value,
            BOOK_EDIT_TEXT_WIDTH as u32,
            self.0,
        )
    }

    fn width(&self, units: &[u16]) -> i32 {
        bbb_client_render::hud_formatted_utf16_width(units, self.0)
    }

    fn head_len(&self, units: &[u16], width: i32) -> usize {
        self.0.plain_utf16_head_len(units, width).unwrap_or(0)
    }
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

impl BookScreen {
    pub(crate) fn new(book: BookScreenState) -> Self {
        let mut screen = Self { book, input: None };
        screen.sync_input();
        screen
    }

    /// A fresh edit box for each page or sign screen `init`; returns whether
    /// one was made.
    fn sync_input(&mut self) -> bool {
        let (mode, text_len) = match &self.book {
            BookScreenState::View(_) => {
                self.input = None;
                return false;
            }
            BookScreenState::Edit(screen) => (
                BookInputMode::Edit {
                    page: screen.current_page,
                },
                screen
                    .pages
                    .get(screen.current_page)
                    .map_or(0, |page| page.units().len()),
            ),
            BookScreenState::Sign(screen) => (BookInputMode::Sign, screen.title.units().len()),
        };
        if self.input.as_ref().is_some_and(|input| input.mode == mode) {
            return false;
        }
        self.input = Some(BookInputState {
            mode,
            cursor: text_len,
            selection: text_len,
            dragging: false,
            display_start: 0,
            focused_at: text_field::clock_millis(),
            ime_preedit: None,
        });
        true
    }

    fn sync_input_with_layout(&mut self, measurer: &impl CommandSuggestionTextMeasurer) {
        if self.sync_input() {
            // BookSignScreen.init: setValue scrolls the fresh title box.
            self.edit_title(measurer, |field, layout| {
                let value = field.value().to_vec();
                field.set_value(&value, Some(layout));
            });
        }
    }

    fn turn_page(&mut self, delta: i32, measurer: &impl CommandSuggestionTextMeasurer) {
        self.book.turn_page(delta);
        self.sync_input_with_layout(measurer);
    }

    /// Runs `edit` on the page box built from the open page and writes the
    /// result back like `MultiLineEditBox.setValueListener`.
    fn edit_page<R>(
        &mut self,
        measurer: &impl CommandSuggestionTextMeasurer,
        edit: impl FnOnce(&mut MultilineTextField, &BookPageFont<'_>) -> R,
    ) -> Option<R> {
        let input = self.input.as_mut()?;
        if !matches!(input.mode, BookInputMode::Edit { .. }) {
            return None;
        }
        let current = book_units(&self.book)?;
        let mut field = MultilineTextField::new(
            current,
            input.cursor,
            input.selection,
            MAX_BOOK_PAGE_UTF16_UNITS,
            Some(BOOK_EDIT_LINE_LIMIT),
        );
        let font = BookPageFont(measurer.command_suggestion_glyphs());
        let result = edit(&mut field, &font);
        if field.value() != current && !self.book.replace_page(field.value().to_vec()) {
            return Some(result);
        }
        input.cursor = field.cursor();
        input.selection = field.selection();
        Some(result)
    }

    /// The sign screen's title `EditBox` over the book's title.
    fn edit_title<R>(
        &mut self,
        measurer: &impl CommandSuggestionTextMeasurer,
        edit: impl FnOnce(&mut TextField, &TextFieldLayout<'_>) -> R,
    ) -> Option<R> {
        let input = self.input.as_mut()?;
        if input.mode != BookInputMode::Sign {
            return None;
        }
        let current = book_units(&self.book)?;
        let mut field = TextField::from_parts(
            current,
            input.cursor,
            input.selection,
            input.display_start,
            MAX_BOOK_TITLE_UTF16_UNITS,
        );
        let layout = TextFieldLayout {
            glyphs: measurer.command_suggestion_glyphs(),
            inner_width: BOOK_SIGN_TITLE_WIDTH as i32,
        };
        let result = edit(&mut field, &layout);
        if field.value() != current && !self.book.replace_title(field.value().to_vec()) {
            return Some(result);
        }
        input.cursor = field.cursor();
        input.selection = field.selection();
        input.display_start = field.display_start();
        Some(result)
    }

    fn seek_cursor_to_point(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        selecting: bool,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) {
        let Some(cursor_position) = cursor_position else {
            return;
        };
        let layout = bbb_client_render::hud_book_layout(surface_size);
        let x = cursor_position.x - f64::from(layout.origin_x + BOOK_EDIT_TEXT_X);
        let line = ((cursor_position.y - f64::from(layout.origin_y + BOOK_EDIT_TEXT_Y))
            / f64::from(BOOK_EDIT_LINE_HEIGHT))
        .floor()
        .max(0.0) as usize;
        if self
            .edit_page(measurer, |field, font| {
                field.seek_to_point(x.floor() as i32, line, selecting, font);
            })
            .is_some()
        {
            return;
        }
        self.edit_title(measurer, |field, layout| {
            let start = field.display_start();
            let visible = &field.value()[start..];
            let end = layout
                .glyphs
                .plain_utf16_head_len(visible, layout.inner_width)
                .unwrap_or(0);
            let width =
                bbb_client_render::hud_formatted_utf16_width(&visible[..end], layout.glyphs);
            // BookSignScreen draws the title centred in the box.
            let text_x = (surface_size.width as i32 - layout.inner_width) / 2 - 3
                + (layout.inner_width - width) / 2;
            let column = layout
                .glyphs
                .plain_utf16_head_len(
                    visible,
                    ((cursor_position.x.floor() as i32) - text_x).min(layout.inner_width),
                )
                .unwrap_or(0);
            field.move_cursor(start + column, selecting, Some(layout));
        });
    }
}

impl ClientInputState {
    pub(crate) fn book_screen(&self) -> Option<&BookScreen> {
        self.screen.book()
    }

    pub(crate) fn book_screen_mut(&mut self) -> Option<&mut BookScreen> {
        self.screen.book_mut()
    }

    pub(crate) fn book_input_state(&self) -> Option<&BookInputState> {
        self.screen.book().and_then(|screen| screen.input.as_ref())
    }

    pub(crate) fn open_writable_book_editor(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        hand: InteractionHand,
    ) -> bool {
        let Some(book) = world.writable_book_editor(hand, self.client_text_filtering_enabled())
        else {
            return false;
        };
        world.replace_screen();
        self.set_screen(Screen::Book(BookScreen::new(book)), counters, net_commands);
        true
    }

    /// Closes an edit or sign screen whose writable book left its slot.
    pub(crate) fn sync_book_screen(&mut self, world: &WorldStore) {
        if self
            .screen
            .book()
            .is_some_and(|screen| !world.book_is_held(&screen.book))
        {
            self.screen.close();
        }
    }

    #[allow(clippy::too_many_arguments)]
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
        let modifiers = self.text_field_modifiers();
        let container_is_current = self.screen.container().is_some();
        let Some(screen) = self.screen.book_mut() else {
            if !container_is_current {
                return false;
            }
            let Some(lectern) = world.lectern_book_view() else {
                return false;
            };
            if !pressed {
                return true;
            }
            match code {
                KeyCode::Escape => {
                    queue_container_close_command(counters, world, net_commands);
                    self.apply_screen_requests(world, counters, net_commands);
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
        };
        screen.sync_input_with_layout(measurer);
        if !pressed {
            return true;
        }
        match (screen.input.as_ref().map(|input| input.mode), code) {
            (_, KeyCode::Escape) => {
                self.screen.close();
            }
            (None | Some(BookInputMode::Edit { .. }), KeyCode::PageUp) => {
                screen.turn_page(-1, measurer);
            }
            (None | Some(BookInputMode::Edit { .. }), KeyCode::PageDown) => {
                screen.turn_page(1, measurer);
            }
            (Some(BookInputMode::Edit { .. }), _) => {
                screen.edit_page(measurer, |field, font| {
                    field.key(code, modifiers, clipboard, font)
                });
            }
            (Some(BookInputMode::Sign), KeyCode::Enter | KeyCode::NumpadEnter) => {
                self.submit_book(counters, world, net_commands);
            }
            (Some(BookInputMode::Sign), _) => {
                screen.edit_title(measurer, |field, layout| {
                    field.key(code, modifiers, clipboard, Some(layout))
                });
            }
            (None, _) => {}
        }
        true
    }

    pub(crate) fn handle_book_screen_text(
        &mut self,
        text: &str,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) -> bool {
        let Some(screen) = self.screen.book_mut() else {
            return false;
        };
        screen.sync_input_with_layout(measurer);
        if screen
            .edit_page(measurer, |field, font| field.type_text(text, font))
            .is_none()
        {
            screen.edit_title(measurer, |field, layout| {
                field.type_text(text, Some(layout))
            });
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
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
        let Some(screen) = self.screen.book_mut() else {
            return false;
        };
        screen.sync_input_with_layout(measurer);
        if button != MouseButton::Left {
            return true;
        }
        if state == ElementState::Released {
            if let Some(input) = &mut screen.input {
                input.dragging = false;
            }
            return true;
        }
        match book_click_target(&screen.book, cursor_position, surface_size) {
            Some(BookClickTarget::Done) if screen.book.is_edit() => {
                self.submit_book(counters, world, net_commands);
            }
            Some(BookClickTarget::Done) => {
                self.screen.close();
            }
            Some(BookClickTarget::Sign) => {
                let author = world.local_player_scoreboard_name().unwrap_or_default();
                if screen.book.begin_sign(author) {
                    screen.sync_input_with_layout(measurer);
                }
            }
            Some(BookClickTarget::Finalize) => {
                self.submit_book(counters, world, net_commands);
            }
            Some(BookClickTarget::Cancel) => {
                if screen.book.cancel_sign() {
                    screen.sync_input_with_layout(measurer);
                }
            }
            Some(BookClickTarget::PreviousPage) => screen.turn_page(-1, measurer),
            Some(BookClickTarget::NextPage) => screen.turn_page(1, measurer),
            Some(BookClickTarget::EditText) | Some(BookClickTarget::SignTitle) => {
                screen.seek_cursor_to_point(cursor_position, surface_size, false, measurer);
                if let Some(input) = &mut screen.input {
                    input.dragging = true;
                }
            }
            None => {}
        }
        true
    }

    pub(crate) fn handle_book_screen_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) -> bool {
        let Some(screen) = self.screen.book_mut() else {
            return false;
        };
        if screen.input.as_ref().is_some_and(|input| input.dragging) {
            screen.seek_cursor_to_point(cursor_position, surface_size, true, measurer);
        }
        true
    }

    /// `BookEditScreen.saveChanges` (Done) or `BookSignScreen.saveChanges`.
    fn submit_book(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        let Some(submission) = self
            .screen
            .book()
            .and_then(|screen| world.book_submission(&screen.book))
        else {
            return;
        };
        // BookEditScreen: the local copy changes before sending.
        if !world.commit_book_submission(&submission) {
            return;
        }
        self.screen.close();
        queue_edit_book_command(
            counters,
            net_commands,
            EditBook {
                slot: submission.slot,
                pages: submission.pages,
                title: submission.title,
            },
        );
    }
}

fn book_units(book: &BookScreenState) -> Option<&[u16]> {
    match book {
        BookScreenState::Edit(screen) => screen
            .pages
            .get(screen.current_page)
            .map(bbb_client_world::BookEditorText::units),
        BookScreenState::Sign(screen) => Some(screen.title.units()),
        BookScreenState::View(_) => None,
    }
}

fn book_click_target(
    book: &BookScreenState,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<BookClickTarget> {
    let cursor = cursor_position?;
    let layout = bbb_client_render::hud_book_layout(surface_size);
    let x = cursor.x - f64::from(layout.origin_x);
    let y = cursor.y - f64::from(layout.origin_y);
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
                && screen.can_finalize_with_button()
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

#[cfg(test)]
mod tests;
