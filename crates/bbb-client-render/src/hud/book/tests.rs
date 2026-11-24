use super::*;
use crate::{ClearColor, HudAsciiGlyph, HudRichTooltipLine, Renderer};

fn glyphs_for(text: &str) -> HudFontGlyphMap {
    let mut glyphs = HudFontGlyphMap::new();
    let glyph = HudAsciiGlyph {
        width: 5,
        height: 8,
        advance: 6.0,
        ..HudAsciiGlyph::default()
    };
    for ch in text.chars().chain(['?', ' ']) {
        glyphs.insert_first_wins(ch, glyph);
    }
    glyphs
}

fn view_screen(text: &str) -> HudBookScreen {
    HudBookScreen {
        editor_text: text.encode_utf16().collect(),
        editor_display_start: 0,
        generation: 7,
        page: 0,
        page_count: 1,
        resource_generation: 9,
        resources_loaded: true,
        text_filtering_enabled: false,
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
        take_book: "Take Book".to_string(),
        may_build: false,
        sign: "Sign".to_string(),
        finalize: "Finalize".to_string(),
        cancel: "Cancel".to_string(),
        finalize_enabled: false,
        preedit: None,
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
            mode: HudBookMode::View,
            generation: 7,
            page: 0,
            resource_generation: 9,
            resources_loaded: true,
            text_filtering_enabled: false,
            source_run_index: 0,
        })
    );
}

#[test]
fn book_view_and_lectern_hits_follow_formatted_text_and_split_reset() {
    let fonts = HudFontGlyphBook::from(glyphs_for("A B"));
    let size = PhysicalSize::new(320, 240);
    let layout = hud_book_layout(size);
    let position = |x, line| {
        (
            f64::from(layout.origin_x + HUD_BOOK_TEXT_X + x),
            f64::from(layout.origin_y + HUD_BOOK_TEXT_Y + line * HUD_BOOK_LINE_HEIGHT + 1),
        )
    };
    for mode in [HudBookMode::View, HudBookMode::Lectern] {
        let mut screen = view_screen("§aA");
        screen.mode = mode;
        assert!(book_interaction_at(&screen, &fonts, size, position(1, 0)).is_some());
        assert!(book_interaction_at(&screen, &fonts, size, position(7, 0)).is_none());

        let mut screen = view_screen(&format!("{}§rB", "A".repeat(19)));
        screen.mode = mode;
        assert!(book_interaction_at(&screen, &fonts, size, position(1, 0)).is_some());
        assert!(book_interaction_at(&screen, &fonts, size, position(1, 1)).is_none());
        let lines = hud_wrap_formatted_component_runs_with_interactions(
            &screen.runs,
            &book_chat_interactions(&screen),
            HUD_BOOK_TEXT_WIDTH,
            &fonts,
        );
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1].runs[0].visual.text, "B");
    }
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
            advance: 2.0,
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
    let wrapped = hud_wrap_formatted_component_runs_with_interactions(
        &screen.runs,
        &book_chat_interactions(&screen),
        HUD_BOOK_TEXT_WIDTH,
        &fonts,
    );
    assert_eq!(wrapped.len(), 1);
    assert_eq!(wrapped[0].interactions[0].target_index, 0);
    let geometry =
        hud_component_text_hit_geometry(&wrapped[0].runs, &wrapped[0].targets, &fonts, true);
    assert_eq!(geometry.glyph_areas.len(), 30);
    // Thirty two-pixel advances fill the page line without rounding drift.
    assert_eq!(geometry.glyph_areas[29].0.right, 60.0);

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
                advance: advance as f32,
                ..HudAsciiGlyph::default()
            },
        );
    }

    assert_eq!(formatted_width("لا", &glyphs), 17);
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
fn hud_book_lectern_keeps_custom_font_and_revision_safe_hit_mode() {
    let mut screen = view_screen("abc");
    screen.mode = HudBookMode::Lectern;
    screen.runs[0].font = Some("example:book".to_string());
    let screen = sanitize_hud_book_screen(screen).unwrap();
    assert_eq!(screen.runs[0].font.as_deref(), Some("example:book"));

    let fonts = HudFontGlyphBook::from(glyphs_for("abc"));
    let layout = hud_book_layout(PhysicalSize::new(320, 240));
    let target = hud_book_hit_target_at(
        &screen,
        &fonts,
        PhysicalSize::new(320, 240),
        (
            f64::from(layout.origin_x + HUD_BOOK_TEXT_X + 1),
            f64::from(layout.origin_y + HUD_BOOK_TEXT_Y + 1),
        ),
    )
    .unwrap();
    assert_eq!(target.mode, HudBookMode::Lectern);
    assert_eq!(target.generation, screen.generation);
}

#[test]
fn hud_book_lectern_controls_match_page_and_may_build_state() {
    let layout = hud_book_layout(PhysicalSize::new(320, 240));
    let mut screen = view_screen("page");
    screen.mode = HudBookMode::Lectern;
    screen.page_count = 3;
    screen.page = 1;
    screen.may_build = true;

    assert_eq!(page_button_visibility(&screen), (true, true));
    assert_eq!(
        book_menu_buttons(&screen, layout),
        vec![
            (layout.origin_x - 4, 98, "Done", true),
            (layout.origin_x + 98, 98, "Take Book", true),
        ]
    );

    screen.may_build = false;
    assert_eq!(
        book_menu_buttons(&screen, layout),
        vec![(layout.origin_x - 4, 200, "Done", true)]
    );
    screen.page = 0;
    screen.page_count = 1;
    assert_eq!(page_button_visibility(&screen), (false, false));
}

#[test]
fn hud_book_lectern_renders_deterministic_background_hover_and_footer_pixels() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        eprintln!("skipping offscreen renderer test without a compatible adapter");
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });
    renderer
        .upload_gui_texture(
            "textures/gui/book.png",
            256,
            256,
            &[255, 0, 0, 255].repeat(256 * 256),
            crate::StandaloneTextureSampling::default(),
        )
        .unwrap();
    renderer
        .add_test_gui_sprite(
            "widget/page_backward",
            23,
            13,
            &[0, 255, 0, 255].repeat(23 * 13),
        )
        .unwrap();
    renderer
        .add_test_gui_sprite(
            "widget/page_backward_highlighted",
            23,
            13,
            &[255, 255, 0, 255].repeat(23 * 13),
        )
        .unwrap();
    renderer
        .add_test_gui_sprite(
            "widget/page_forward",
            23,
            13,
            &[0, 255, 0, 255].repeat(23 * 13),
        )
        .unwrap();
    renderer
        .add_test_gui_sprite(
            "widget/page_forward_highlighted",
            23,
            13,
            &[0, 0, 255, 255].repeat(23 * 13),
        )
        .unwrap();
    renderer
        .upload_hud_font_atlas(
            1,
            1,
            &[255, 255, 255, 255],
            glyphs_for("Page 2 of 3DoneTake Bookpage"),
        )
        .unwrap();

    let layout = hud_book_layout(PhysicalSize::new(320, 240));
    let mut screen = view_screen("page");
    screen.mode = HudBookMode::Lectern;
    screen.page = 1;
    screen.page_count = 3;
    screen.page_indicator = "Page 2 of 3".to_string();
    screen.may_build = true;
    screen.cursor_position = Some((
        layout.origin_x + HUD_BOOK_PAGE_FORWARD_X,
        layout.origin_y + HUD_BOOK_PAGE_BUTTON_Y,
    ));
    renderer.set_hud_book_screen(Some(screen.clone()));

    let first = renderer.render_offscreen_frame().unwrap();
    assert_eq!(first.pixel(70, 10), [255, 0, 0, 255]);
    assert_eq!(
        first.pixel(
            u32::try_from(layout.origin_x + HUD_BOOK_PAGE_BACK_X + 1).unwrap(),
            u32::try_from(layout.origin_y + HUD_BOOK_PAGE_BUTTON_Y + 1).unwrap(),
        ),
        [0, 255, 0, 255]
    );
    assert_eq!(
        first.pixel(
            u32::try_from(layout.origin_x + HUD_BOOK_PAGE_FORWARD_X + 1).unwrap(),
            u32::try_from(layout.origin_y + HUD_BOOK_PAGE_BUTTON_Y + 1).unwrap(),
        ),
        [0, 0, 255, 255]
    );
    let footer_gap = first.pixel(4, 205);
    assert_eq!(first.pixel(160, 205), footer_gap);
    assert_ne!(first.pixel(170, 205), footer_gap);
    let repeated = renderer.render_offscreen_frame().unwrap();
    assert_eq!(first.rgba, repeated.rgba);

    screen.may_build = false;
    renderer.set_hud_book_screen(Some(screen));
    let full_width = renderer.render_offscreen_frame().unwrap();
    assert_ne!(full_width.pixel(160, 205), footer_gap);
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

#[test]
fn book_editor_formatting_resolves_each_cursor_segment_and_measures_logically() {
    let glyphs = glyphs_for("AB§a");
    let all = resolved_book_runs(&[HudStyledTextRun::plain("§aAB")]);
    assert_eq!(all[0].text, "AB");
    assert_eq!(all[0].color, Some(0x55ff55));
    assert_eq!(formatted_width("§aAB", &glyphs), 12);
    assert_eq!(formatted_width("§lA", &glyphs), 7);
    let before = resolved_book_runs(&[HudStyledTextRun::plain("§")]);
    let after = resolved_book_runs(&[HudStyledTextRun::plain("aAB")]);
    assert!(before.is_empty());
    assert_eq!(after[0].text, "aAB");
    assert_eq!(after[0].color, None);
    assert_eq!(formatted_width("§", &glyphs), 0);
}

#[test]
fn book_editor_raw_projection_retains_unit_cursor_and_split_low_surrogate() {
    let mut screen = view_screen("");
    screen.mode = HudBookMode::Edit;
    screen.editor_text = vec![0xa7, 0xd83d, 0xde00, 65];
    screen.cursor = 4;
    screen.selection = 2;
    let screen = sanitize_hud_book_screen(screen).unwrap();
    assert_eq!((screen.cursor, screen.selection), (4, 2));
    assert_eq!(screen.editor_text, [0xa7, 0xd83d, 0xde00, 65]);
    let lines = editor_utf16_lines(&screen.editor_text, &glyphs_for("A�"));
    assert_eq!(lines, [EditorLine { start: 0, end: 4 }]);
    let before = super::super::hud_resolve_formatted_utf16_text(&screen.editor_text[..2]);
    let after = super::super::hud_resolve_formatted_utf16_text(&screen.editor_text[2..]);
    assert!(before.is_empty());
    assert_eq!(after[0].text, "�A");
}

fn page_click(
    screen: &HudBookScreen,
    fonts: &HudFontGlyphBook,
    local_x: i32,
    local_y: i32,
) -> Option<HudBookHitTarget> {
    let layout = hud_book_layout(PhysicalSize::new(320, 240));
    hud_book_hit_target_at(
        screen,
        fonts,
        PhysicalSize::new(320, 240),
        (
            f64::from(layout.origin_x + HUD_BOOK_TEXT_X + local_x),
            f64::from(layout.origin_y + HUD_BOOK_TEXT_Y + local_y),
        ),
    )
}

#[test]
fn book_page_click_uses_glyph_areas_and_rejects_gaps_and_padding() {
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(' ', HudAsciiGlyph::empty(4.0));
    for ch in ['a', ' '] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                width: 5,
                height: 8,
                advance: 6.0,
                ..HudAsciiGlyph::default()
            },
        );
    }
    let fonts = HudFontGlyphBook::from(glyphs);
    let mut screen = view_screen("a a");
    screen.interactions = vec![
        HudBookInteraction {
            start_char: 0,
            end_char: 1,
            source_run_index: 4,
            clickable: true,
            tooltip: None,
        },
        HudBookInteraction {
            start_char: 2,
            end_char: 3,
            source_run_index: 6,
            clickable: true,
            tooltip: None,
        },
    ];

    assert_eq!(
        page_click(&screen, &fonts, 1, 4).map(|target| target.source_run_index),
        Some(4)
    );
    // The space is an `EmptyArea` with no interaction of its own.
    assert!(page_click(&screen, &fonts, 7, 4).is_none());
    assert_eq!(
        page_click(&screen, &fonts, 11, 4).map(|target| target.source_run_index),
        Some(6)
    );
    // Prepared bounds clip the line to its drawn ink on both axes.
    assert!(page_click(&screen, &fonts, 100, 4).is_none());
    assert!(page_click(&screen, &fonts, 1, -1).is_none());
    assert!(page_click(&screen, &fonts, 1, 9).is_none());
    // The click finder always prepares with `dropShadow = true`, so the
    // accepted box keeps the shadow row the page itself does not draw.
    assert!(page_click(&screen, &fonts, 1, 8).is_some());
}

#[test]
fn book_page_click_ignores_styles_without_a_click_event() {
    let fonts = HudFontGlyphBook::from(glyphs_for("h"));
    let mut screen = view_screen("h");
    screen.interactions[0].clickable = false;
    assert!(page_click(&screen, &fonts, 1, 4).is_none());
    // The tooltip hover still reports the style under the cursor.
    let layout = hud_book_layout(PhysicalSize::new(320, 240));
    assert!(book_interaction_at(
        &screen,
        &fonts,
        PhysicalSize::new(320, 240),
        (
            f64::from(layout.origin_x + HUD_BOOK_TEXT_X + 1),
            f64::from(layout.origin_y + HUD_BOOK_TEXT_Y + 4),
        ),
    )
    .is_some());
}
