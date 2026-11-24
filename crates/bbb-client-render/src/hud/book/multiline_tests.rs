use super::*;
use crate::HudAsciiGlyph;

fn glyphs() -> HudFontGlyphMap {
    let mut glyphs = HudFontGlyphMap::new();
    for ch in ['a', 'b', 'c', '?', ' '] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                advance: 6.0,
                ..HudAsciiGlyph::default()
            },
        );
    }
    glyphs
}

#[test]
fn book_multiline_selection_extends_through_newlines_and_empty_lines() {
    let text = "a\n\nb";
    let glyphs = glyphs();
    let lines = editor_lines(text, &glyphs);
    for (index, line) in lines.iter().enumerate() {
        let rect = editor_selection_rect(text, *line, &glyphs, 0, 4, 99, editor_line_y(index));
        assert_eq!(
            rect,
            Some(absolute_hud_rect(
                99.0,
                editor_line_y(index) as f32,
                if index == 2 { 6 } else { 118 },
                9,
            ))
        );
        assert_eq!(
            rect,
            editor_selection_rect(text, *line, &glyphs, 4, 0, 99, editor_line_y(index))
        );
    }
    assert_eq!(
        editor_selection_rect(text, lines[0], &glyphs, 1, 2, 99, 32),
        Some(absolute_hud_rect(105.0, 32.0, 112, 9))
    );
    assert!(editor_selection_rect(text, lines[2], &glyphs, 0, 3, 99, 50).is_none());
    assert!(editor_selection_rect(text, lines[1], &glyphs, 2, 2, 99, 41).is_none());
}

#[test]
fn book_multiline_insert_cursor_owns_only_first_soft_wrap_boundary() {
    let text = "a".repeat(20);
    let lines = editor_lines(&text, &glyphs());
    assert_eq!(lines[0].end, lines[1].start);
    assert_eq!(editor_cursor_line(&lines, 19, true), Some(0));
    assert_eq!(editor_cursor_line(&lines, 20, false), Some(1));
}

#[test]
fn book_multiline_append_cursor_follows_the_last_visible_line_including_empty() {
    let glyphs = glyphs();
    assert_eq!(
        editor_cursor_line(&editor_lines("", &glyphs), 0, false),
        Some(0)
    );
    assert_eq!(
        editor_cursor_line(&editor_lines("a\n", &glyphs), 2, false),
        Some(1)
    );
    let text = "a\n".repeat(20);
    let lines = editor_lines(&text, &glyphs);
    assert_eq!(
        editor_cursor_line(&lines, text.chars().count(), false),
        Some(13)
    );
    assert_eq!(editor_cursor_line(&lines, 30, true), None);
}

#[test]
fn book_multiline_scissor_uses_widget_inset_instead_of_text_wrap_width() {
    let mut commands = Vec::new();
    assert!(push_hud_rect_scissor(
        &mut commands,
        PhysicalSize::new(320, 240),
        editor_scissor_rect(99),
    ));
    assert!(matches!(
        commands.as_slice(),
        [HudDrawCommand::Scissor {
            x: 96,
            y: 29,
            width: 120,
            height: 132
        }]
    ));
    assert!(!push_hud_rect_scissor(
        &mut commands,
        PhysicalSize::new(320, 20),
        editor_scissor_rect(99),
    ));
}

#[test]
fn book_editor_wrap_keeps_zero_prefix_with_first_wide_glyph_and_trailing_line() {
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, advance) in [('z', 0.0), ('w', 120.0), ('a', 57.0)] {
        glyphs.insert_first_wins(ch, HudAsciiGlyph::empty(advance));
    }
    let lines = editor_lines("zzwa\n", &glyphs);
    assert_eq!(
        lines
            .iter()
            .map(|line| (line.start, line.end))
            .collect::<Vec<_>>(),
        [(0, 3), (3, 4), (5, 5)]
    );
}
