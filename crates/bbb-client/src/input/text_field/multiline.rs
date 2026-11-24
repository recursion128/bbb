//! Vanilla `MultilineTextField` over a Java `String`: unlike `EditBox` it
//! steps by UTF-16 unit, splits words on `Character.isWhitespace`, keeps
//! `\n`, and truncates insertions by raw units without selection credit.

use std::ops::Range;

use bbb_protocol::java_character_is_whitespace;
use winit::keyboard::KeyCode;

use super::{copy_units_to_clipboard, is_allowed_chat_character, TextFieldModifiers};
use crate::input::DebugClipboard;

/// `MultilineTextField.LINE_SEEK_PIXEL_BIAS`.
const LINE_SEEK_PIXEL_BIAS: i32 = 2;

/// The owner's font for one text area.
pub(crate) trait TextLineFont {
    /// `MultilineTextField.reflowDisplayLines` of `value` at the area width.
    fn lines(&self, value: &[u16]) -> Vec<Range<usize>>;
    /// `Font.width` of a plain run.
    fn width(&self, units: &[u16]) -> i32;
    /// `Font.plainSubstrByWidth(run, width).length()`.
    fn head_len(&self, units: &[u16], width: i32) -> usize;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MultilineTextField {
    value: Vec<u16>,
    cursor: usize,
    selection: usize,
    character_limit: usize,
    line_limit: Option<usize>,
}

impl MultilineTextField {
    pub(crate) fn new(
        value: &[u16],
        cursor: usize,
        selection: usize,
        character_limit: usize,
        line_limit: Option<usize>,
    ) -> Self {
        Self {
            value: value.to_vec(),
            cursor: cursor.min(value.len()),
            selection: selection.min(value.len()),
            character_limit,
            line_limit,
        }
    }

    pub(crate) fn value(&self) -> &[u16] {
        &self.value
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn selection(&self) -> usize {
        self.selection
    }

    fn selected(&self) -> Range<usize> {
        self.cursor.min(self.selection)..self.cursor.max(self.selection)
    }

    /// `MultilineTextField.insertText`: `true` when the value listener fired.
    pub(crate) fn insert_text(&mut self, text: &str, font: &dyn TextLineFont) -> bool {
        let selected = self.selected();
        if text.is_empty() && selected.is_empty() {
            return false;
        }
        let remaining = self.character_limit.saturating_sub(self.value.len());
        // StringUtil.filterText(input, true) keeps `\n` and filters units.
        let inserted: Vec<u16> = text
            .encode_utf16()
            .filter(|&unit| unit == 10 || (unit >= 32 && unit != 127 && unit != 167))
            .take(remaining)
            .collect();
        let mut candidate = self.value.clone();
        let cursor = selected.start + inserted.len();
        candidate.splice(selected, inserted);
        if self
            .line_limit
            .is_some_and(|limit| font.lines(&candidate).len() > limit)
        {
            return false;
        }
        self.value = candidate;
        self.cursor = cursor;
        self.selection = cursor;
        true
    }

    /// `MultiLineEditBox.charTyped` once per code point.
    pub(crate) fn type_text(&mut self, text: &str, font: &dyn TextLineFont) -> bool {
        let mut buffer = [0; 4];
        let mut fired = false;
        for ch in text.chars().filter(|&ch| is_allowed_chat_character(ch)) {
            fired |= self.insert_text(ch.encode_utf8(&mut buffer), font);
        }
        fired
    }

    fn delete(&mut self, direction: isize, font: &dyn TextLineFont) -> bool {
        if self.cursor == self.selection {
            self.selection = self
                .cursor
                .saturating_add_signed(direction)
                .min(self.value.len());
        }
        self.insert_text("", font)
    }

    /// `MultilineTextField.seekCursor(ABSOLUTE, ..)`.
    pub(crate) fn seek(&mut self, position: usize, selecting: bool) {
        self.cursor = position.min(self.value.len());
        if !selecting {
            self.selection = self.cursor;
        }
    }

    /// `MultilineTextField.getLineAtCursor` falling back to the last line.
    pub(crate) fn cursor_line(&self, lines: &[Range<usize>]) -> usize {
        lines
            .iter()
            .position(|line| self.cursor >= line.start && self.cursor <= line.end)
            .unwrap_or(lines.len().saturating_sub(1))
    }

    fn seek_line(&mut self, offset: isize, selecting: bool, font: &dyn TextLineFont) {
        let lines = font.lines(&self.value);
        let Some(current) = lines.get(self.cursor_line(&lines)).cloned() else {
            return;
        };
        let left = font
            .width(&self.value[current.start..self.cursor.max(current.start)])
            .saturating_add(LINE_SEEK_PIXEL_BIAS);
        let target = self
            .cursor_line(&lines)
            .saturating_add_signed(offset)
            .min(lines.len() - 1);
        let line = lines[target].clone();
        let column = font.head_len(&self.value[line.clone()], left);
        self.seek(line.start + column, selecting);
    }

    /// `MultilineTextField.seekCursorToPoint` with `x` relative to the text
    /// origin and `line` already `floor(y / 9)`.
    pub(crate) fn seek_to_point(
        &mut self,
        x: i32,
        line: usize,
        selecting: bool,
        font: &dyn TextLineFont,
    ) {
        let lines = font.lines(&self.value);
        let Some(line) = lines.get(line.min(lines.len().saturating_sub(1))).cloned() else {
            return;
        };
        let column = font.head_len(&self.value[line.clone()], x);
        self.seek(line.start + column, selecting);
    }

    /// `MultilineTextField.selectWordAtCursor`.
    pub(crate) fn select_word_at_cursor(&mut self) {
        let word = self.previous_word();
        self.seek(word.start, false);
        self.seek(word.end, true);
    }

    fn previous_word(&self) -> Range<usize> {
        if self.value.is_empty() {
            return 0..0;
        }
        let mut start = self.cursor.min(self.value.len() - 1);
        while start > 0 && java_character_is_whitespace(self.value[start - 1]) {
            start -= 1;
        }
        while start > 0 && !java_character_is_whitespace(self.value[start - 1]) {
            start -= 1;
        }
        start..self.word_end(start)
    }

    fn next_word(&self) -> Range<usize> {
        if self.value.is_empty() {
            return 0..0;
        }
        let mut start = self.cursor.min(self.value.len() - 1);
        while start < self.value.len() && !java_character_is_whitespace(self.value[start]) {
            start += 1;
        }
        while start < self.value.len() && java_character_is_whitespace(self.value[start]) {
            start += 1;
        }
        start..self.word_end(start)
    }

    fn word_end(&self, from: usize) -> usize {
        from + self.value[from..]
            .iter()
            .take_while(|&&unit| !java_character_is_whitespace(unit))
            .count()
    }

    /// `MultilineTextField.keyPressed`: `None` when the field does not take
    /// the key, otherwise whether the value listener fired.
    pub(crate) fn key(
        &mut self,
        code: KeyCode,
        modifiers: TextFieldModifiers,
        clipboard: &mut Option<&mut dyn DebugClipboard>,
        font: &dyn TextLineFont,
    ) -> Option<bool> {
        let selecting = modifiers.shift;
        let word = modifiers.word;
        let fired = match code {
            KeyCode::KeyA if modifiers.shortcut => {
                self.cursor = self.value.len();
                self.selection = 0;
                false
            }
            KeyCode::KeyC if modifiers.shortcut => {
                copy_units_to_clipboard(clipboard, &self.value[self.selected()]);
                false
            }
            KeyCode::KeyV if modifiers.shortcut => clipboard
                .as_deref_mut()
                .and_then(DebugClipboard::get_debug_clipboard_text)
                .is_some_and(|text| self.insert_text(&text, font)),
            KeyCode::KeyX if modifiers.shortcut => {
                copy_units_to_clipboard(clipboard, &self.value[self.selected()]);
                self.insert_text("", font)
            }
            KeyCode::Enter | KeyCode::NumpadEnter => self.insert_text("\n", font),
            KeyCode::Backspace if word => {
                let target = self.previous_word().start;
                self.delete(target as isize - self.cursor as isize, font)
            }
            KeyCode::Backspace => self.delete(-1, font),
            KeyCode::Delete if word => {
                let target = self.next_word().start;
                self.delete(target as isize - self.cursor as isize, font)
            }
            KeyCode::Delete => self.delete(1, font),
            KeyCode::ArrowRight if word => {
                self.seek(self.next_word().start, selecting);
                false
            }
            KeyCode::ArrowRight => {
                self.seek(self.cursor + 1, selecting);
                false
            }
            KeyCode::ArrowLeft if word => {
                self.seek(self.previous_word().start, selecting);
                false
            }
            KeyCode::ArrowLeft => {
                self.seek(self.cursor.saturating_sub(1), selecting);
                false
            }
            KeyCode::ArrowDown | KeyCode::ArrowUp => {
                if !word {
                    let offset = if code == KeyCode::ArrowDown { 1 } else { -1 };
                    self.seek_line(offset, selecting, font);
                }
                false
            }
            KeyCode::PageUp => {
                self.seek(0, selecting);
                false
            }
            KeyCode::PageDown => {
                self.seek(self.value.len(), selecting);
                false
            }
            KeyCode::Home | KeyCode::End => {
                let target = if word {
                    if code == KeyCode::Home {
                        0
                    } else {
                        self.value.len()
                    }
                } else {
                    let lines = font.lines(&self.value);
                    lines.get(self.cursor_line(&lines)).map_or(0, |line| {
                        if code == KeyCode::Home {
                            line.start
                        } else {
                            line.end
                        }
                    })
                };
                self.seek(target, selecting);
                false
            }
            _ => return None,
        };
        Some(fired)
    }
}
