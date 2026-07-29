use super::*;
use crate::{HudAsciiGlyph, HudRichTooltipLine};

fn glyphs_for(text: &str) -> HudFontGlyphMap {
    let mut glyphs = HudFontGlyphMap::new();
    let glyph = HudAsciiGlyph {
        width: 5,
        height: 8,
        advance: 6,
        ..HudAsciiGlyph::default()
    };
    for ch in text.chars().chain(['?', ' ']) {
        glyphs.insert_first_wins(ch, glyph);
    }
    glyphs
}

fn view_screen(text: &str) -> HudBookScreen {
    HudBookScreen {
        generation: 7,
        page: 0,
        page_count: 1,
        resource_generation: 9,
        resources_loaded: true,
        advanced_item_tooltips: false,
        mode: HudBookMode::View,
        runs: vec![HudComponentTextRun::plain(text)],
        interactions: vec![HudBookInteraction {
            start_char: 0,
            end_char: text.chars().count(),
            source_run_index: 0,
            clickable: true,
            tooltip: Some(HudRichTooltip {
                style: None,
                lines: vec![HudRichTooltipLine::plain("tip")],
                advanced_lines: None,
                wrap_text: true,
            }),
        }],
        cursor: 0,
        selection: 0,
        cursor_visible: false,
        cursor_position: None,
        page_indicator: "Page 1 of 1".to_string(),
        title: String::new(),
        author: String::new(),
        warning: String::new(),
        done: "Done".to_string(),
        sign: "Sign".to_string(),
        finalize: "Finalize".to_string(),
        cancel: "Cancel".to_string(),
        finalize_enabled: false,
    }
}

#[test]
fn book_screen_layout_is_top_centered() {
    assert_eq!(
        hud_book_layout(PhysicalSize::new(854, 480)),
        HudBookLayout {
            origin_x: 331,
            origin_y: 2,
        }
    );
}

#[test]
fn book_screen_hit_uses_wrapped_source_interaction_and_revision_token() {
    let glyphs = glyphs_for("alpha beta");
    let fonts = HudFontGlyphBook::from(glyphs);
    let screen = view_screen("alpha beta");
    let layout = hud_book_layout(PhysicalSize::new(320, 240));
    let hit = hud_book_hit_target_at(
        &screen,
        &fonts,
        PhysicalSize::new(320, 240),
        (
            f64::from(layout.origin_x + HUD_BOOK_TEXT_X + 2),
            f64::from(layout.origin_y + HUD_BOOK_TEXT_Y + 2),
        ),
    );
    assert_eq!(
        hit,
        Some(HudBookHitTarget {
            generation: 7,
            page: 0,
            resource_generation: 9,
            resources_loaded: true,
            source_run_index: 0,
        })
    );
}

#[test]
fn hud_book_custom_font_controls_wrapping_and_interaction_hit_geometry() {
    let default = glyphs_for("a");
    let mut compact = HudFontGlyphMap::new();
    compact.insert_first_wins(
        'a',
        HudAsciiGlyph {
            width: 1,
            height: 8,
            advance: 2,
            ..HudAsciiGlyph::default()
        },
    );
    let fonts = HudFontGlyphBook::from_fonts(
        "minecraft:default",
        std::collections::BTreeMap::from([
            ("example:compact".to_string(), compact),
            ("minecraft:default".to_string(), default),
        ]),
    );
    let mut screen = view_screen(&"a".repeat(30));
    screen.runs[0].font = Some("example:compact".to_string());
    let wrapped = hud_wrap_component_runs_with_interactions(
        &screen.runs,
        &book_chat_interactions(&screen),
        HUD_BOOK_TEXT_WIDTH,
        &fonts,
    );
    assert_eq!(wrapped.len(), 1);
    assert_eq!(wrapped[0].interactions[0].end_x, 60);

    let layout = hud_book_layout(PhysicalSize::new(320, 240));
    assert!(book_interaction_at(
        &screen,
        &fonts,
        PhysicalSize::new(320, 240),
        (
            f64::from(layout.origin_x + HUD_BOOK_TEXT_X + 1),
            f64::from(layout.origin_y + HUD_BOOK_TEXT_Y + HUD_BOOK_LINE_HEIGHT + 1),
        ),
    )
    .is_none());
}

#[test]
fn hud_book_unavailable_font_falls_back_for_wrapping_and_hit_geometry() {
    let fonts = HudFontGlyphBook::from(glyphs_for("a"));
    let mut screen = view_screen(&"a".repeat(30));
    screen.runs[0].font = Some("example:missing".to_string());
    let layout = hud_book_layout(PhysicalSize::new(320, 240));

    assert!(book_interaction_at(
        &screen,
        &fonts,
        PhysicalSize::new(320, 240),
        (
            f64::from(layout.origin_x + HUD_BOOK_TEXT_X + 1),
            f64::from(layout.origin_y + HUD_BOOK_TEXT_Y + HUD_BOOK_LINE_HEIGHT + 1),
        ),
    )
    .is_some());
}

#[test]
fn book_screen_hit_clips_after_fourteen_lines() {
    let text = (0..16).map(|_| "a").collect::<Vec<_>>().join("\n");
    let fonts = HudFontGlyphBook::from(glyphs_for(&text));
    let screen = view_screen(&text);
    let layout = hud_book_layout(PhysicalSize::new(320, 240));
    assert!(hud_book_hit_target_at(
        &screen,
        &fonts,
        PhysicalSize::new(320, 240),
        (
            f64::from(layout.origin_x + HUD_BOOK_TEXT_X + 1),
            f64::from(
                layout.origin_y
                    + HUD_BOOK_TEXT_Y
                    + HUD_BOOK_LINE_HEIGHT * HUD_BOOK_MAX_LINES as i32
            ),
        ),
    )
    .is_none());
}

#[test]
fn book_screen_editor_layout_wraps_at_word_boundary_and_keeps_trailing_line() {
    let glyphs = glyphs_for("abcdefghij klmnopqrstuv\n");
    let lines = editor_lines("abcdefghij klmnopqrstuv\n", &glyphs);
    assert_eq!(lines[0], EditorLine { start: 0, end: 10 });
    assert_eq!(lines.last(), Some(&EditorLine { start: 24, end: 24 }));
}

#[test]
fn book_editor_cursor_width_keeps_the_unshaped_logical_buffer_order() {
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, advance) in [('ل', 9_u32), ('ا', 8_u32), ('\u{fefb}', 5_u32)] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                advance,
                ..HudAsciiGlyph::default()
            },
        );
    }

    assert_eq!(plain_width("لا", &glyphs), 17);
}

#[test]
fn book_screen_sanitize_rejects_out_of_range_interactions() {
    let mut screen = view_screen("abc");
    screen.interactions[0].end_char = 9;
    let sanitized = sanitize_hud_book_screen(screen).unwrap();
    assert!(sanitized.interactions.is_empty());
}

#[test]
fn hud_book_edit_sanitization_forces_the_default_font() {
    let mut screen = view_screen("abc");
    screen.mode = HudBookMode::Edit;
    screen.runs[0].font = Some("example:untrusted".to_string());

    let sanitized = sanitize_hud_book_screen(screen).unwrap();
    assert_eq!(sanitized.runs[0].font, None);
}

#[test]
fn book_screen_sanitize_preserves_newlines_and_hover_only_spans_are_not_click_targets() {
    let glyphs = glyphs_for("a\nb");
    let fonts = HudFontGlyphBook::from(glyphs);
    let mut screen = view_screen("a\nb");
    screen.interactions[0].clickable = false;
    let screen = sanitize_hud_book_screen(screen).unwrap();
    assert_eq!(screen.runs[0].visual.text, "a\nb");
    let layout = hud_book_layout(PhysicalSize::new(320, 240));
    assert!(hud_book_hit_target_at(
        &screen,
        &fonts,
        PhysicalSize::new(320, 240),
        (
            f64::from(layout.origin_x + HUD_BOOK_TEXT_X + 1),
            f64::from(layout.origin_y + HUD_BOOK_TEXT_Y + 1),
        ),
    )
    .is_none());
}

#[test]
fn book_screen_page_button_hover_uses_absolute_top_centered_geometry() {
    let layout = hud_book_layout(PhysicalSize::new(320, 240));
    assert!(page_button_hovered(
        Some((
            layout.origin_x + HUD_BOOK_PAGE_FORWARD_X,
            layout.origin_y + HUD_BOOK_PAGE_BUTTON_Y,
        )),
        layout,
        HUD_BOOK_PAGE_FORWARD_X,
    ));
    assert!(!page_button_hovered(
        Some((layout.origin_x, layout.origin_y)),
        layout,
        HUD_BOOK_PAGE_FORWARD_X,
    ));
}
