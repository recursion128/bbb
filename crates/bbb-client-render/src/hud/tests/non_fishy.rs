use super::*;

fn selected_glyphs(advance: f32) -> HudFontGlyphMap {
    let mut glyphs = HudFontGlyphMap::new();
    let any = HudAsciiGlyph {
        uv: HudUvRect {
            min: [0.0, 0.0],
            max: [0.5, 1.0],
        },
        width: 2,
        height: 2,
        advance: 40.0,
        ..Default::default()
    };
    let filtered = HudAsciiGlyph {
        uv: HudUvRect {
            min: [0.5, 0.0],
            max: [1.0, 1.0],
        },
        advance,
        ..any
    };
    for ch in ['a', '_'] {
        glyphs.insert_selected_codepoint_first_wins(ch as u32, any, filtered, 40.0);
    }
    glyphs
}

#[test]
fn non_fishy_input_layout_keeps_visible_substring_cursor_and_selection_together() {
    let glyphs = selected_glyphs(4.0);
    let label = HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: 0,
        y: 0,
        width: 8,
        text: "aaaa".into(),
        tint: HUD_TINT_WHITE,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs: Vec::new(),
    };
    let input = HudInventoryTextInputDecoration {
        cursor: 4,
        selection: 3,
        scroll_to: 4,
        max_length: 256,
        cursor_visible: true,
        cursor_tint: HUD_TINT_WHITE,
    };
    let layout = utf16_widget_input_layout(&label, input, glyphs.non_fishy());
    assert_eq!(layout.displayed_text, "aa");
    assert_eq!(layout.cursor_x, 9.0);
    assert_eq!(layout.selection_rect, Some((3.0, 6)));
    assert!(layout.cursor_on_screen);
    assert!(!layout.insert_cursor);
    let geometry = hud_logical_styled_text_geometry(
        &[HudStyledTextRun::plain(layout.displayed_text)],
        glyphs.non_fishy(),
        &Default::default(),
        0,
        (0.0, 0.0),
        false,
        HUD_TINT_WHITE,
        Some(8),
        1.0,
    );
    assert_eq!(geometry.glyph_quads.len(), 2);
    assert_eq!(geometry.glyph_quads[1].0.corners[0][0], 4.0);
    assert_eq!(geometry.glyph_quads[1].0.uv.min[0], 0.5);
    assert_eq!(glyphs.plain_text_width("aa"), 80);
}

#[test]
fn non_fishy_chat_frame_filters_input_suffix_and_append_cursor_only() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer
        .upload_hud_font_atlas(
            2,
            1,
            &[255, 0, 0, 255, 0, 255, 0, 255],
            selected_glyphs(4.0),
        )
        .expect("font atlas");
    let before_reload = renderer.hud_text_measurements();
    assert_eq!(before_reload.plain_text_width("aa"), 80);
    assert_eq!(before_reload.non_fishy().plain_text_width("aa"), 8);
    renderer
        .upload_hud_font_atlas(
            2,
            1,
            &[255, 0, 0, 255, 0, 255, 0, 255],
            selected_glyphs(6.0),
        )
        .expect("reloaded font atlas");
    assert_eq!(before_reload.non_fishy().plain_text_width("aa"), 8);
    assert_eq!(
        renderer
            .hud_text_measurements()
            .non_fishy()
            .plain_text_width("aa"),
        12
    );
    assert_eq!(
        renderer.hud_text_measurements_ref().plain_text_width("aa"),
        80
    );
    let mut state = HudCommandSuggestions {
        input_text: "aa".into(),
        cursor: 2,
        selection: 2,
        cursor_visible: true,
        suffix: Some("a".into()),
        list_x: 20,
        list_y: 40,
        list_width: 81,
        list_height: 12,
        entries: vec![HudCommandSuggestionEntry {
            runs: vec![HudStyledTextRun::plain("aa")],
            selected: false,
        }],
        tooltip: Some(HudCommandSuggestionTooltip {
            runs: vec![HudStyledTextRun::plain("a")],
            x: 100,
            y: 80,
        }),
        ..Default::default()
    };
    renderer.set_hud_command_suggestions(Some(state.clone()));
    let pixels = renderer.render_offscreen_frame().expect("chat frame");
    for x in [4, 10, 16, 17] {
        let p = pixels.pixel(x, 228);
        assert!(
            p[1] > 70 && p[0] < 40,
            "filtered input/suffix/cursor x={x}: {p:?}"
        );
    }
    for x in [21, 61] {
        let p = pixels.pixel(x, 42);
        assert!(p[0] > 70 && p[1] < 40, "ordinary completion x={x}: {p:?}");
    }
    let origin = hud_inventory_tooltip_line_origin(
        PhysicalSize::new(320, 240),
        320,
        240,
        100,
        80,
        40,
        hud_inventory_tooltip_text_height(1).unwrap(),
        0,
    );
    let p = pixels.pixel(origin.0 as u32, origin.1 as u32);
    assert!(
        p[0] > 150 && p[1] < 40 && p[2] < 40,
        "ordinary tooltip: {p:?}"
    );
    state.layout = HudCommandSuggestionsLayout::CommandBlock;
    state.tooltip = None;
    renderer.set_hud_command_suggestions(Some(state));
    let pixels = renderer
        .render_offscreen_frame()
        .expect("command block frame");
    let layout = hud_command_block_editor_layout(PhysicalSize::new(320, 240));
    let x = (layout.command_input.x + 4) as u32;
    let y = (layout.command_input.y + 6) as u32;
    for offset in [0, 40, 80, 81] {
        let p = pixels.pixel(x + offset, y);
        assert!(
            p[0] > 70 && p[1] < 40,
            "ordinary command input/suffix/cursor x={offset}: {p:?}"
        );
    }
}
