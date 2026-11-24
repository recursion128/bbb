use super::*;
use bbb_client_render::HudAsciiGlyph;

fn glyphs() -> HudFontGlyphMap {
    let mut glyphs = HudFontGlyphMap::new();
    let missing = HudAsciiGlyph::empty(6.0);
    glyphs.set_missing_glyph(missing);
    for ch in ['A', 'B', 'X', ' ', char::REPLACEMENT_CHARACTER] {
        glyphs.insert_selected_codepoint_first_wins(ch as u32, missing, missing, 6.0);
    }
    let emoji = HudAsciiGlyph::empty(12.0);
    glyphs.insert_selected_codepoint_first_wins('😀' as u32, emoji, emoji, 12.0);
    glyphs
}

fn set_text(state: &mut TextField, text: &str, layout: Option<&TextFieldLayout<'_>>) {
    state.set_value(&text.encode_utf16().collect::<Vec<_>>(), layout);
}

#[test]
fn text_field_preserves_split_surrogate_cursor_selection_and_scroll_history() {
    let glyphs = glyphs();
    let layout = TextFieldLayout {
        glyphs: glyphs.non_fishy(),
        inner_width: 18,
    };
    let layout = Some(&layout);
    let mut state = TextField::new(256);
    set_text(&mut state, "😀AB", layout);
    assert_eq!(
        (state.cursor(), state.selection(), state.display_start()),
        (4, 4, 1)
    );
    state.set_cursor(1, layout);
    assert_eq!(
        (state.cursor(), state.selection(), state.display_start()),
        (1, 4, 0)
    );
    state.set_selection(4, layout);
    assert_eq!(
        (state.cursor(), state.selection(), state.display_start()),
        (1, 4, 1)
    );
    state.move_cursor(2, true, layout);
    assert_eq!((state.cursor(), state.selection()), (2, 4));
    state.move_cursor(1, false, layout);
    assert_eq!(
        (state.cursor(), state.selection(), state.display_start()),
        (1, 1, 0)
    );
    state.select_all(layout);
    assert_eq!(
        (state.cursor(), state.selection(), state.display_start()),
        (4, 0, 0)
    );
    assert_eq!(state.clone(), state);
    state.set_cursor(usize::MAX, layout);
    state.set_selection(usize::MAX, layout);
    assert_eq!((state.cursor(), state.selection()), (4, 4));
}

#[test]
fn text_field_set_value_and_insert_have_distinct_utf16_truncation_rules() {
    let glyphs = glyphs();
    let layout = TextFieldLayout {
        glyphs: glyphs.non_fishy(),
        inner_width: 18,
    };
    let layout = Some(&layout);
    let mut state = TextField::new(256);
    let value = format!("{}😀", "A".repeat(255));
    set_text(&mut state, &value, layout);
    assert_eq!(state.value().len(), 256);
    assert_eq!(state.value().last(), Some(&0xd83d));
    assert!(state.text().ends_with(char::REPLACEMENT_CHARACTER));
    set_text(&mut state, &"A".repeat(255), layout);
    state.insert_text("😀X", layout);
    assert_eq!(state.value().len(), 255);
    assert_eq!((state.cursor(), state.selection()), (255, 255));
    set_text(&mut state, &"A".repeat(254), layout);
    state.insert_text("😀X", layout);
    assert_eq!(state.value().len(), 256);
    assert_eq!(&state.value()[254..], &[0xd83d, 0xde00]);
    let full = state.clone();
    state.insert_text("X", layout);
    assert_eq!(state, full);
    state.select_all(layout);
    state.insert_text("\n\r\t§\u{7f}A😀中\u{a0}", layout);
    assert_eq!(state.text(), "A😀中\u{a0}");
    assert_eq!(state.cursor(), 5);
}

#[test]
fn text_field_allows_inserting_and_deleting_at_half_surrogates() {
    let glyphs = glyphs();
    let layout = TextFieldLayout {
        glyphs: glyphs.non_fishy(),
        inner_width: 18,
    };
    let layout = Some(&layout);
    let mut state = TextField::new(256);
    set_text(&mut state, "😀A", layout);
    state.move_cursor(1, false, layout);
    state.insert_text("X", layout);
    assert_eq!(state.value(), &[0xd83d, 88, 0xde00, 65]);
    assert_eq!(state.text(), "�X�A");
    assert_eq!((state.cursor(), state.selection()), (2, 2));
    set_text(&mut state, "😀A", layout);
    state.move_cursor(1, false, layout);
    state.delete_chars(1, false, layout);
    assert_eq!(state.value(), &[0xd83d, 65]);
    set_text(&mut state, "😀A", layout);
    state.move_cursor(1, false, layout);
    state.delete_chars(-1, false, layout);
    assert_eq!(state.value(), &[0xde00, 65]);
    assert_eq!((state.cursor(), state.selection()), (0, 0));
    set_text(&mut state, "😀A", layout);
    state.move_cursor(0, false, layout);
    state.move_cursor(1, true, layout);
    state.delete_chars(1, true, layout);
    assert_eq!(state.value(), &[0xde00, 65]);
}

#[test]
fn text_field_codepoint_offsets_keep_java_pair_detection_and_lone_units() {
    let glyphs = glyphs();
    let layout = TextFieldLayout {
        glyphs: glyphs.non_fishy(),
        inner_width: 18,
    };
    let layout = Some(&layout);
    let mut state = TextField::new(256);
    state.set_value(&[0xd83d, 0xde00, 65, 0xd800, 66, 0xdc00], layout);
    for (cursor, direction, expected) in [
        (0, 1, 2),
        (1, 1, 2),
        (1, -1, 0),
        (2, -1, 0),
        (6, -1, 5),
        (6, -2, 4),
        (3, 1, 4),
        (2, i32::MAX, 6),
        (4, i32::MIN, 0),
        (1, 0, 1),
    ] {
        state.set_cursor(cursor, layout);
        assert_eq!(
            state.offset_by_codepoints(direction),
            expected,
            "{cursor}, {direction}"
        );
    }
}

#[test]
fn text_field_word_movement_and_deletion_only_recognize_ascii_spaces() {
    let glyphs = glyphs();
    let layout = TextFieldLayout {
        glyphs: glyphs.non_fishy(),
        inner_width: 18,
    };
    let layout = Some(&layout);
    let mut state = TextField::new(256);
    set_text(&mut state, "ab\u{a0}cd  ef\tgh ij", layout);
    assert_eq!(state.word_position(0, 1), 7);
    assert_eq!(state.word_position(0, 2), 13);
    assert_eq!(state.word_position(15, -1), 13);
    assert_eq!(state.word_position(15, -2), 7);
    assert_eq!(state.word_position(15, -3), 0);
    assert_eq!(state.word_position(7, i32::MIN), 7);
    state.delete_chars(-1, true, layout);
    assert_eq!(state.text(), "ab\u{a0}cd  ef\tgh ");
    state.move_cursor(0, false, layout);
    state.delete_chars(1, true, layout);
    assert_eq!(state.text(), "ef\tgh ");
    assert_eq!((state.cursor(), state.selection()), (0, 0));
    state.select_all(layout);
    state.delete_chars(-1, false, layout);
    assert!(state.value().is_empty());
    state.delete_chars(-1, true, layout);
    assert_eq!(state, TextField::new(256));
}

#[test]
fn text_field_uses_filtered_fractional_and_nan_widths_without_merging_scroll_calls() {
    let mut glyphs = glyphs();
    glyphs.insert_selected_codepoint_first_wins(
        'N' as u32,
        HudAsciiGlyph::empty(-10.0),
        HudAsciiGlyph::empty(1.25),
        -10.0,
    );
    glyphs.insert_selected_codepoint_first_wins(
        'Q' as u32,
        HudAsciiGlyph::empty(f32::NAN),
        HudAsciiGlyph::empty(f32::NAN),
        f32::NAN,
    );
    let layout = TextFieldLayout {
        glyphs: glyphs.non_fishy(),
        inner_width: 2,
    };
    let layout = Some(&layout);
    let mut state = TextField::new(256);
    set_text(&mut state, "NNN", layout);
    assert_eq!(state.display_start(), 2);
    set_text(&mut state, "", layout);
    state.insert_text("Q", layout);
    assert_eq!(state.display_start(), 0);
    set_text(&mut state, "Q", layout);
    assert_eq!(state.display_start(), 1);
    set_text(&mut state, "QQQ", layout);
    assert_eq!(state.display_start(), 3);
    state.delete_chars(-1, false, layout);
    assert_eq!(
        (
            state.text(),
            state.cursor(),
            state.selection(),
            state.display_start()
        ),
        ("QQ".to_string(), 2, 2, 0)
    );
    let negative = TextFieldLayout {
        glyphs: glyphs.non_fishy(),
        inner_width: -1,
    };
    let negative = Some(&negative);
    set_text(&mut state, "AB", negative);
    assert_eq!(state.display_start(), 2);
}

#[test]
fn text_field_unavailable_font_only_clamps_the_existing_display_start() {
    let glyphs = glyphs();
    let layout = TextFieldLayout {
        glyphs: glyphs.non_fishy(),
        inner_width: 18,
    };
    let layout = Some(&layout);
    let mut state = TextField::new(256);
    set_text(&mut state, "AAAAAA", layout);
    assert_eq!(state.display_start(), 3);
    let unavailable = HudFontGlyphMap::default();
    let layout = TextFieldLayout {
        glyphs: &unavailable,
        inner_width: 0,
    };
    let layout = Some(&layout);
    state.move_cursor(0, false, layout);
    assert_eq!(state.display_start(), 3);
    state.set_value(&[0xd800], layout);
    assert_eq!(
        (state.cursor(), state.selection(), state.display_start()),
        (1, 1, 1)
    );
    assert_eq!(state.value(), &[0xd800]);
}

struct Board(Option<String>);

impl DebugClipboard for Board {
    fn set_debug_clipboard_text(&mut self, text: &str) -> bool {
        self.0 = Some(text.to_string());
        true
    }

    fn get_debug_clipboard_text(&mut self) -> Option<String> {
        self.0.clone()
    }
}

const SHORTCUT: TextFieldModifiers = TextFieldModifiers {
    shift: false,
    word: false,
    shortcut: true,
};

#[test]
fn text_field_typing_drops_a_non_bmp_char_that_no_longer_fits() {
    let mut field = TextField::new(3);
    assert!(field.type_text("A😀😀", None));
    // charTyped inserts per code point; the second pair would leave a lone half.
    assert_eq!(field.text(), "A😀");
    assert!(!field.type_text("\u{7f}\u{a7}\n", None));
    assert_eq!(field.value().len(), 3);
}

#[test]
fn text_field_typing_and_paste_replace_the_selection_with_its_length_credit() {
    let mut field = TextField::with_text("ABC", 3);
    field.set_selection(1, None);
    assert!(field.type_text("😀", None));
    assert_eq!(field.text(), "A😀");
    assert_eq!((field.cursor(), field.selection()), (3, 3));

    let mut board = Board(Some("x\u{a7}y\tz\u{7f}\r\n😀".to_string()));
    let mut clipboard: Option<&mut dyn DebugClipboard> = Some(&mut board);
    let mut field = TextField::new(32);
    assert_eq!(
        field.key(KeyCode::KeyV, SHORTCUT, &mut clipboard, None),
        Some(true)
    );
    // StringUtil.filterText keeps neither controls nor the section sign.
    assert_eq!(field.text(), "xyz😀");
    field.select_all(None);
    assert_eq!(
        field.key(KeyCode::KeyX, SHORTCUT, &mut clipboard, None),
        Some(true)
    );
    assert!(field.is_empty());
    assert_eq!(board.0.as_deref(), Some("xyz😀"));
}

#[test]
fn text_field_copies_lone_surrogates_as_question_marks() {
    let mut field = TextField::from_parts(&[0xd83d, 65], 0, 2, 0, 32);
    let mut board = Board(None);
    let mut clipboard: Option<&mut dyn DebugClipboard> = Some(&mut board);
    assert_eq!(
        field.key(KeyCode::KeyC, SHORTCUT, &mut clipboard, None),
        Some(false)
    );
    assert_eq!(board.0.as_deref(), Some("?A"));
}

#[test]
fn text_field_shift_arrows_extend_by_code_point_and_unhandled_keys_fall_through() {
    let mut field = TextField::with_text("A😀B", 32);
    let shift = TextFieldModifiers {
        shift: true,
        ..TextFieldModifiers::default()
    };
    field.key(KeyCode::ArrowLeft, shift, &mut None, None);
    field.key(KeyCode::ArrowLeft, shift, &mut None, None);
    assert_eq!((field.cursor(), field.selection()), (1, 4));
    assert_eq!(field.selected(), "😀B".encode_utf16().collect::<Vec<_>>());
    assert_eq!(
        field.key(
            KeyCode::Enter,
            TextFieldModifiers::default(),
            &mut None,
            None
        ),
        None
    );
}

struct Monospace;

impl TextLineFont for Monospace {
    fn lines(&self, value: &[u16]) -> Vec<std::ops::Range<usize>> {
        let mut lines = Vec::new();
        let mut start = 0;
        for (index, &unit) in value.iter().enumerate() {
            if unit == 10 {
                lines.push(start..index);
                start = index + 1;
            }
        }
        lines.push(start..value.len());
        lines
    }

    fn width(&self, units: &[u16]) -> i32 {
        units.len() as i32 * 6
    }

    fn head_len(&self, units: &[u16], width: i32) -> usize {
        units.len().min(usize::try_from(width / 6).unwrap_or(0))
    }
}

#[test]
fn multiline_steps_units_truncates_raw_and_keeps_the_line_limit() {
    let value: Vec<u16> = "A😀".encode_utf16().collect();
    let mut field = MultilineTextField::new(&value, 3, 3, 4, Some(2));
    let none = TextFieldModifiers::default();
    field.key(KeyCode::ArrowLeft, none, &mut None, &Monospace);
    assert_eq!(field.cursor(), 2);
    // No selection credit and no pair check: one raw unit fits.
    field.seek(3, false);
    assert!(field.type_text("😀", &Monospace));
    assert_eq!(field.value(), &[65, 0xd83d, 0xde00, 0xd83d]);

    let mut field = MultilineTextField::new(&[], 0, 0, 100, Some(2));
    assert_eq!(
        field.key(KeyCode::Enter, none, &mut None, &Monospace),
        Some(true)
    );
    assert_eq!(
        field.key(KeyCode::Enter, none, &mut None, &Monospace),
        Some(false)
    );
    assert_eq!(field.value(), &[10]);
    assert!(!field.type_text("\n", &Monospace));
}

#[test]
fn multiline_words_split_on_java_whitespace_and_lines_seek_by_width() {
    let value: Vec<u16> = "ab\u{3000}cd\nefgh".encode_utf16().collect();
    let mut field = MultilineTextField::new(&value, 0, 0, 100, None);
    let word = TextFieldModifiers {
        word: true,
        ..TextFieldModifiers::default()
    };
    field.key(KeyCode::ArrowRight, word, &mut None, &Monospace);
    assert_eq!(field.cursor(), 3);
    field.key(KeyCode::ArrowRight, word, &mut None, &Monospace);
    assert_eq!(field.cursor(), 6);
    field.seek(1, false);
    field.key(
        KeyCode::ArrowDown,
        TextFieldModifiers::default(),
        &mut None,
        &Monospace,
    );
    // Line seek keeps the pixel column plus LINE_SEEK_PIXEL_BIAS.
    assert_eq!(field.cursor(), 7);
    field.select_word_at_cursor();
    assert_eq!((field.selection(), field.cursor()), (6, 10));
}
