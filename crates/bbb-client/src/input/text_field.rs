//! Vanilla `EditBox` / `TextFieldHelper` editing over a Java `String`: every
//! cursor, selection and length is in UTF-16 code units.

use bbb_client_render::{HudCommandBlockEditorRect, HudFontGlyphMap, HudTextField};
use winit::keyboard::KeyCode;

use super::DebugClipboard;

mod multiline;
pub(crate) use multiline::{MultilineTextField, TextLineFont};

/// `EditBox.maxLength` default (EditBox.java:41).
const DEFAULT_MAX_LENGTH: usize = 32;
/// `TextCursorUtils.CURSOR_BLINK_INTERVAL_MS`.
const CURSOR_BLINK_INTERVAL_MS: u64 = 300;

/// Font and inner width for a field that keeps Vanilla's stateful
/// `displayPos`. Fields without one are drawn scrolled from the start to the
/// last position Vanilla would have scrolled to.
pub(crate) struct TextFieldLayout<'a> {
    pub(crate) glyphs: &'a HudFontGlyphMap,
    pub(crate) inner_width: i32,
}

/// Modifier state as `KeyEvent` reports it to a text widget.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TextFieldModifiers {
    pub(crate) shift: bool,
    /// `hasControlDownWithQuirk`: word steps for arrows and deletes.
    pub(crate) word: bool,
    /// `isSelectAll`/`isCopy`/`isPaste`/`isCut` modifier shape.
    pub(crate) shortcut: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct TextField {
    value: Vec<u16>,
    cursor: usize,
    selection: usize,
    display_start: usize,
    scroll_target: usize,
    max_length: usize,
    focused_at: u64,
    ime_preedit: Option<super::ime_preedit::ImePreeditState>,
}

impl Default for TextField {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_LENGTH)
    }
}

// The focus time only phases the caret blink; two fields holding the same
// edit state are the same field.
impl PartialEq for TextField {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
            && self.cursor == other.cursor
            && self.selection == other.selection
            && self.display_start == other.display_start
            && self.scroll_target == other.scroll_target
            && self.max_length == other.max_length
            && self.ime_preedit == other.ime_preedit
    }
}

impl Eq for TextField {}

impl TextField {
    pub(crate) fn new(max_length: usize) -> Self {
        Self {
            value: Vec::new(),
            cursor: 0,
            selection: 0,
            display_start: 0,
            scroll_target: 0,
            max_length,
            focused_at: clock_millis(),
            ime_preedit: None,
        }
    }

    /// An `EditBox` whose value and selection live in another owner.
    pub(crate) fn from_parts(
        value: &[u16],
        cursor: usize,
        selection: usize,
        display_start: usize,
        max_length: usize,
    ) -> Self {
        Self {
            value: value.to_vec(),
            cursor: cursor.min(value.len()),
            selection: selection.min(value.len()),
            display_start: display_start.min(value.len()),
            scroll_target: selection.min(value.len()),
            max_length,
            focused_at: 0,
            ime_preedit: None,
        }
    }

    /// `new EditBox` + `setMaxLength` + `setValue`.
    pub(crate) fn with_text(text: &str, max_length: usize) -> Self {
        let mut field = Self::new(max_length);
        field.set_text(text, None);
        field
    }

    pub(crate) fn with_value(value: &[u16], max_length: usize) -> Self {
        let mut field = Self::new(max_length);
        field.set_value(value, None);
        field
    }

    pub(crate) fn value(&self) -> &[u16] {
        &self.value
    }

    pub(crate) fn text(&self) -> String {
        String::from_utf16_lossy(&self.value)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn selection(&self) -> usize {
        self.selection
    }

    pub(crate) fn display_start(&self) -> usize {
        self.display_start
    }

    /// `EditBox.getHighlighted`.
    pub(crate) fn selected(&self) -> &[u16] {
        &self.value[self.cursor.min(self.selection)..self.cursor.max(self.selection)]
    }

    /// `EditBox.setFocused(true)` restarts the caret blink.
    pub(crate) fn focus(&mut self) {
        self.focused_at = clock_millis();
    }

    pub(crate) fn caret_visible(&self, now_millis: u64) -> bool {
        caret_visible(self.focused_at, now_millis)
    }

    pub(crate) fn set_text(&mut self, text: &str, layout: Option<&TextFieldLayout<'_>>) {
        self.set_value(&text.encode_utf16().collect::<Vec<_>>(), layout);
    }

    /// `EditBox.setValue` truncates by raw units, splitting a pair if needed.
    pub(crate) fn set_value(&mut self, value: &[u16], layout: Option<&TextFieldLayout<'_>>) {
        self.value.clear();
        self.value
            .extend_from_slice(&value[..value.len().min(self.max_length)]);
        // EditBox scrolls cursor, selection, then selection again.
        self.move_cursor(self.value.len(), false, layout);
        self.set_selection(self.cursor, layout);
    }

    /// `EditBox.setCursorPosition`.
    pub(crate) fn set_cursor(&mut self, position: usize, layout: Option<&TextFieldLayout<'_>>) {
        self.cursor = position.min(self.value.len());
        self.scroll_to(self.cursor, layout);
    }

    /// `EditBox.setHighlightPos`.
    pub(crate) fn set_selection(&mut self, position: usize, layout: Option<&TextFieldLayout<'_>>) {
        self.selection = position.min(self.value.len());
        self.scroll_to(self.selection, layout);
    }

    /// `EditBox.moveCursorTo`.
    pub(crate) fn move_cursor(
        &mut self,
        position: usize,
        extend: bool,
        layout: Option<&TextFieldLayout<'_>>,
    ) {
        self.set_cursor(position, layout);
        if !extend {
            self.set_selection(self.cursor, layout);
        }
    }

    pub(crate) fn select_all(&mut self, layout: Option<&TextFieldLayout<'_>>) {
        self.move_cursor(self.value.len(), false, layout);
        self.set_selection(0, layout);
    }

    /// `EditBox.insertText`: `true` when the responder would fire.
    pub(crate) fn insert_text(&mut self, text: &str, layout: Option<&TextFieldLayout<'_>>) -> bool {
        self.insert_units(text.encode_utf16(), layout)
    }

    pub(crate) fn insert_units(
        &mut self,
        units: impl Iterator<Item = u16>,
        layout: Option<&TextFieldLayout<'_>>,
    ) -> bool {
        let start = self.cursor.min(self.selection);
        let end = self.cursor.max(self.selection);
        let available = (self.max_length + end - start).saturating_sub(self.value.len());
        if available == 0 {
            return false;
        }
        // StringUtil filters UTF-16 units, not Unicode whitespace.
        let mut inserted: Vec<u16> = units
            .filter(|&unit| is_allowed_chat_unit(unit))
            .take(available + 1)
            .collect();
        if inserted.len() > available {
            inserted.truncate(available);
            if inserted.last().is_some_and(|unit| is_high_surrogate(*unit)) {
                inserted.pop();
            }
        }
        let insertion_length = inserted.len();
        self.value.splice(start..end, inserted);
        self.set_cursor(start + insertion_length, layout);
        self.set_selection(self.cursor, layout);
        true
    }

    /// `EditBox.charTyped` once per code point.
    pub(crate) fn type_text(&mut self, text: &str, layout: Option<&TextFieldLayout<'_>>) -> bool {
        self.type_text_filtered(text, |_, _| true, layout)
    }

    /// `charTyped` for a screen that rejects some code points first, like the
    /// structure block name.
    pub(crate) fn type_text_filtered(
        &mut self,
        text: &str,
        mut accept: impl FnMut(&Self, char) -> bool,
        layout: Option<&TextFieldLayout<'_>>,
    ) -> bool {
        let mut fired = false;
        let mut buffer = [0; 4];
        for ch in text.chars() {
            if is_allowed_chat_character(ch) && accept(self, ch) {
                fired |= self.insert_text(ch.encode_utf8(&mut buffer), layout);
            }
        }
        fired
    }

    /// `TextFieldHelper.insertText`: no length limit or filter, but the whole
    /// candidate must pass `valid`.
    pub(crate) fn insert_validated(&mut self, text: &str, valid: impl Fn(&[u16]) -> bool) -> bool {
        let len = self.value.len();
        let start = self.cursor.min(self.selection).min(len);
        let end = self.cursor.max(self.selection).min(len);
        let inserted: Vec<u16> = text.encode_utf16().collect();
        let mut candidate = self.value.clone();
        candidate.splice(start..end, inserted.iter().copied());
        if !valid(&candidate) {
            return false;
        }
        self.cursor = start + inserted.len();
        self.selection = self.cursor;
        self.value = candidate;
        true
    }

    /// `TextFieldHelper.charTyped` once per code point.
    pub(crate) fn type_validated(&mut self, text: &str, valid: impl Fn(&[u16]) -> bool) -> bool {
        let mut buffer = [0; 4];
        let mut changed = false;
        for ch in text.chars().filter(|ch| is_allowed_chat_character(*ch)) {
            changed |= self.insert_validated(ch.encode_utf8(&mut buffer), &valid);
        }
        changed
    }

    /// `EditBox.deleteText`.
    pub(crate) fn delete_chars(
        &mut self,
        direction: i32,
        whole_word: bool,
        layout: Option<&TextFieldLayout<'_>>,
    ) -> bool {
        if self.value.is_empty() {
            return false;
        }
        if self.cursor != self.selection {
            return self.insert_text("", layout);
        }
        let target = if whole_word {
            self.word_position(self.cursor, direction)
        } else {
            self.offset_by_codepoints(direction)
        };
        let start = self.cursor.min(target);
        let end = self.cursor.max(target);
        if start == end {
            return false;
        }
        self.value.drain(start..end);
        // EditBox scrolls before moveCursorTo repeats cursor and selection.
        self.set_cursor(start, layout);
        self.move_cursor(start, false, layout);
        true
    }

    /// `Util.offsetByCodepoints` from the cursor.
    pub(crate) fn offset_by_codepoints(&self, direction: i32) -> usize {
        let mut position = self.cursor;
        for _ in 0..direction.unsigned_abs() {
            if direction < 0 {
                if position == 0 {
                    break;
                }
                position -= 1;
                if is_low_surrogate(self.value[position])
                    && position > 0
                    && is_high_surrogate(self.value[position - 1])
                {
                    position -= 1;
                }
            } else {
                if position == self.value.len() {
                    break;
                }
                position += 1;
                if is_high_surrogate(self.value[position - 1])
                    && position < self.value.len()
                    && is_low_surrogate(self.value[position])
                {
                    position += 1;
                }
            }
        }
        position
    }

    /// `EditBox.getWordPosition`: only ASCII spaces separate words.
    pub(crate) fn word_position(&self, from: usize, direction: i32) -> usize {
        word_position(&self.value, from, direction, |unit| unit == 32)
    }

    /// `EditBox.keyPressed`: `None` when the box does not take the key,
    /// otherwise whether the responder fired.
    pub(crate) fn key(
        &mut self,
        code: KeyCode,
        modifiers: TextFieldModifiers,
        clipboard: &mut Option<&mut dyn DebugClipboard>,
        layout: Option<&TextFieldLayout<'_>>,
    ) -> Option<bool> {
        let selecting = modifiers.shift;
        let fired = match code {
            KeyCode::Backspace => self.delete_chars(-1, modifiers.word, layout),
            KeyCode::Delete => self.delete_chars(1, modifiers.word, layout),
            KeyCode::ArrowLeft | KeyCode::ArrowRight => {
                let direction = if code == KeyCode::ArrowLeft { -1 } else { 1 };
                let target = if modifiers.word {
                    self.word_position(self.cursor, direction)
                } else {
                    self.offset_by_codepoints(direction)
                };
                self.move_cursor(target, selecting, layout);
                false
            }
            KeyCode::Home => {
                self.move_cursor(0, selecting, layout);
                false
            }
            KeyCode::End => {
                self.move_cursor(self.value.len(), selecting, layout);
                false
            }
            KeyCode::KeyA if modifiers.shortcut => {
                self.select_all(layout);
                false
            }
            KeyCode::KeyC if modifiers.shortcut => {
                copy_units_to_clipboard(clipboard, self.selected());
                false
            }
            KeyCode::KeyX if modifiers.shortcut => {
                copy_units_to_clipboard(clipboard, self.selected()) && self.insert_text("", layout)
            }
            KeyCode::KeyV if modifiers.shortcut => clipboard
                .as_deref_mut()
                .and_then(DebugClipboard::get_debug_clipboard_text)
                .is_some_and(|text| self.insert_text(&text, layout)),
            _ => return None,
        };
        Some(fired)
    }

    /// `EditBox.findClickedPositionInText` for a field without a stateful
    /// `displayPos`: `text_x` is relative to the text origin.
    pub(crate) fn clicked_position(
        &self,
        text_x: i32,
        inner_width: i32,
        glyphs: &HudFontGlyphMap,
    ) -> usize {
        let start = glyphs
            .plain_utf16_scroll_to(&self.value, 0, self.scroll_target, inner_width)
            .unwrap_or(0);
        start
            + glyphs
                .plain_utf16_head_len(&self.value[start..], text_x.min(inner_width))
                .unwrap_or(0)
    }

    /// `EditBox.onClick` (single click) and `onDrag` (`extend`).
    pub(crate) fn click(
        &mut self,
        text_x: i32,
        inner_width: i32,
        glyphs: &HudFontGlyphMap,
        extend: bool,
    ) {
        let position = self.clicked_position(text_x, inner_width, glyphs);
        self.move_cursor(position, extend, None);
    }

    /// `EditBox.selectWord` (double click); both bounds come from the
    /// clicked position before either move.
    pub(crate) fn select_word_at(
        &mut self,
        text_x: i32,
        inner_width: i32,
        glyphs: &HudFontGlyphMap,
    ) {
        let position = self.clicked_position(text_x, inner_width, glyphs);
        let start = self.word_position(position, -1);
        let end = self.word_position(position, 1);
        self.move_cursor(start, false, None);
        self.move_cursor(end, true, None);
    }

    /// `onClick` on a bordered box, whose text starts 4 px inside.
    pub(crate) fn click_bordered(
        &mut self,
        rect: HudCommandBlockEditorRect,
        mouse_x: i32,
        glyphs: &HudFontGlyphMap,
        extend: bool,
    ) {
        self.click(
            mouse_x.saturating_sub(rect.x).saturating_sub(4),
            i32::try_from(rect.width.saturating_sub(8)).unwrap_or(i32::MAX),
            glyphs,
            extend,
        );
    }

    pub(crate) fn set_ime_preedit(&mut self, text: &str, cursor_range: Option<(usize, usize)>) {
        self.ime_preedit = super::ime_preedit::ImePreeditState::from_event(
            text,
            cursor_range,
            bbb_client_clock::now(),
        );
    }

    pub(crate) fn clear_ime_preedit(&mut self) {
        self.ime_preedit = None;
    }

    pub(crate) fn ime_preedit_hud_state(&self) -> Option<super::ime_preedit::ImePreeditHudState> {
        Some(
            self.ime_preedit
                .as_ref()?
                .hud_state(bbb_client_clock::now()),
        )
    }

    /// The renderer's view of this box.
    pub(crate) fn hud(&self, focused: bool, hovered: bool) -> HudTextField {
        HudTextField {
            text: self.text(),
            cursor: self.cursor,
            selection: self.selection,
            scroll_to: self.scroll_target,
            max_length: self.max_length,
            focused,
            hovered,
            cursor_visible: focused && self.caret_visible(clock_millis()),
            preedit: focused
                .then(|| {
                    self.ime_preedit_hud_state()
                        .map(super::ime_preedit::ImePreeditHudState::into_hud_ime_preedit)
                })
                .flatten(),
        }
    }

    fn scroll_to(&mut self, target: usize, layout: Option<&TextFieldLayout<'_>>) {
        self.scroll_target = target;
        let Some(layout) = layout else {
            return;
        };
        self.display_start = layout
            .glyphs
            .plain_utf16_scroll_to(&self.value, self.display_start, target, layout.inner_width)
            .unwrap_or_else(|| self.display_start.min(self.value.len()));
    }
}

/// A [`TextField`] whose `Default` already carries the owning screen's
/// `setMaxLength`, for state structs that derive `Default`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoundedTextField<const MAX_LENGTH: usize>(TextField);

impl<const MAX_LENGTH: usize> Default for BoundedTextField<MAX_LENGTH> {
    fn default() -> Self {
        Self(TextField::new(MAX_LENGTH))
    }
}

impl<const MAX_LENGTH: usize> std::ops::Deref for BoundedTextField<MAX_LENGTH> {
    type Target = TextField;

    fn deref(&self) -> &TextField {
        &self.0
    }
}

impl<const MAX_LENGTH: usize> std::ops::DerefMut for BoundedTextField<MAX_LENGTH> {
    fn deref_mut(&mut self) -> &mut TextField {
        &mut self.0
    }
}

/// `EditBox.getWordPosition` / `StringSplitter.getWordPosition` with stripped
/// separators.
pub(crate) fn word_position(
    value: &[u16],
    from: usize,
    direction: i32,
    separator: impl Fn(u16) -> bool,
) -> usize {
    let mut position = from.min(value.len());
    // Math.abs(Integer.MIN_VALUE) stays negative, so Java runs no step.
    let steps = if direction == i32::MIN {
        0
    } else {
        direction.unsigned_abs()
    };
    for _ in 0..steps {
        let before = position;
        if direction < 0 {
            while position > 0 && separator(value[position - 1]) {
                position -= 1;
            }
            while position > 0 && !separator(value[position - 1]) {
                position -= 1;
            }
        } else {
            while position < value.len() && !separator(value[position]) {
                position += 1;
            }
            while position < value.len() && separator(value[position]) {
                position += 1;
            }
        }
        if position == before {
            break;
        }
    }
    position
}

/// `StringUtil.isAllowedChatCharacter`.
pub(crate) fn is_allowed_chat_character(ch: char) -> bool {
    ch != '\u{a7}' && ch >= ' ' && ch != '\u{7f}'
}

fn is_allowed_chat_unit(unit: u16) -> bool {
    unit >= 32 && unit != 127 && unit != 167
}

pub(crate) fn is_high_surrogate(unit: u16) -> bool {
    (0xd800..=0xdbff).contains(&unit)
}

pub(crate) fn is_low_surrogate(unit: u16) -> bool {
    (0xdc00..=0xdfff).contains(&unit)
}

/// `TextCursorUtils.isCursorVisible(Util.getMillis() - focusedTime)`.
pub(crate) fn caret_visible(focused_at: u64, now_millis: u64) -> bool {
    (now_millis.saturating_sub(focused_at) / CURSOR_BLINK_INTERVAL_MS).is_multiple_of(2)
}

/// `KeyboardHandler.setClipboard` of a Java substring: the JDK's UTF-8
/// encoder writes `?` for a lone surrogate. An empty selection leaves the
/// clipboard alone.
fn copy_units_to_clipboard(clipboard: &mut Option<&mut dyn DebugClipboard>, units: &[u16]) -> bool {
    let text: String = char::decode_utf16(units.iter().copied())
        .map(|decoded| decoded.unwrap_or('?'))
        .collect();
    text.is_empty()
        || clipboard
            .as_deref_mut()
            .is_some_and(|clipboard| clipboard.set_debug_clipboard_text(&text))
}

/// `Util.getMillis()` on the injected client clock.
pub(crate) fn clock_millis() -> u64 {
    bbb_client_clock::system_now()
        .duration_since(bbb_client_clock::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64)
}

#[cfg(test)]
mod tests;
