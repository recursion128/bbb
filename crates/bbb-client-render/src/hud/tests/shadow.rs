use super::*;

fn run(text: &str, color: u32, shadow_color: Option<u32>) -> HudStyledTextRun {
    HudStyledTextRun {
        text: text.into(),
        color: Some(color),
        shadow_color,
        ..Default::default()
    }
}

#[test]
fn shadow_geometry_resolves_argb_without_default_shadow_and_keeps_glyph_order() {
    let glyphs = styled_test_glyphs();
    let runs = [
        run("a", 0xff0000, Some(0)),
        run("b", 0x00ff00, Some(0x7f0000ff)),
    ];
    let geometry = hud_styled_text_geometry(
        &runs,
        &glyphs,
        &Default::default(),
        0,
        (0.0, 0.0),
        false,
        [1.0, 1.0, 1.0, 128.0 / 255.0],
        None,
        1.0,
    );
    assert_eq!(geometry.glyph_quads.len(), 3);
    assert_eq!(
        geometry
            .glyph_quads
            .iter()
            .map(|(q, _)| q.shadow)
            .collect::<Vec<_>>(),
        [false, true, false]
    );
    assert_eq!(geometry.glyph_quads[0].1, [1.0, 0.0, 0.0, 128.0 / 255.0]);
    assert_eq!(geometry.glyph_quads[1].1, [0.0, 0.0, 1.0, 63.0 / 255.0]);
    assert_eq!(geometry.glyph_quads[2].1, [0.0, 1.0, 0.0, 128.0 / 255.0]);
    let suppressed = hud_styled_text_geometry(
        &[run("a", 0xffffff, Some(0))],
        &glyphs,
        &Default::default(),
        0,
        (0.0, 0.0),
        true,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    assert_eq!(suppressed.glyph_quads.len(), 1);
    let invisible = hud_styled_text_geometry(
        &[run("a", 0xffffff, Some(0x000000ff))],
        &glyphs,
        &Default::default(),
        0,
        (0.0, 0.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    assert_eq!(invisible.glyph_quads.len(), 2);
    assert_eq!(invisible.glyph_quads[0].1, [0.0, 0.0, 1.0, 0.0]);
}

#[test]
fn shadow_bold_and_empty_effects_use_selected_offsets_and_order() {
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(
        'a',
        HudAsciiGlyph {
            advance: 3.5,
            width: 3,
            height: 8,
            shadow_offset: 0.5,
            bold_offset: 0.5,
            ..Default::default()
        },
    );
    glyphs.insert_first_wins(
        ' ',
        HudAsciiGlyph {
            shadow_offset: 0.5,
            bold_offset: 0.5,
            ..HudAsciiGlyph::empty(3.5)
        },
    );
    let mut text = run("a ", 0xffffff, Some(0xff00ff00));
    text.style = HudTextStyle {
        bold: true,
        underlined: true,
        strikethrough: true,
        ..Default::default()
    };
    let geometry = hud_styled_text_geometry(
        &[text],
        &glyphs,
        &Default::default(),
        0,
        (10.0, 20.0),
        false,
        HUD_TINT_WHITE,
        None,
        2.0,
    );
    assert_eq!(geometry.glyph_quads.len(), 4);
    assert_eq!(
        geometry
            .glyph_quads
            .iter()
            .map(|(q, _)| q.shadow)
            .collect::<Vec<_>>(),
        [true, true, false, false]
    );
    let corners = &geometry.glyph_quads;
    assert_eq!(corners[0].0.corners[0][0] - corners[2].0.corners[0][0], 1.0);
    assert_eq!(corners[1].0.corners[0][0] - corners[0].0.corners[0][0], 1.0);
    assert_eq!(geometry.effect_rects.len(), 8);
    for pair in geometry.effect_rects.as_chunks::<2>().0 {
        assert_eq!(pair[0].0.x0 - pair[1].0.x0, 1.0);
        assert_eq!(pair[0].0.y0 - pair[1].0.y0, 1.0);
        assert_eq!(pair[0].1, [0.0, 1.0, 0.0, 1.0]);
        assert_eq!(pair[1].1, HUD_TINT_WHITE);
    }
}

#[test]
fn shadow_chat_wrap_and_sanitize_keep_overrides_and_fade_sensitive_cache_boundary() {
    let glyphs = styled_test_glyphs();
    let fonts = HudFontGlyphBook::from(glyphs);
    let runs = vec![
        HudComponentTextRun::from(run("a", 0xffffff, Some(0))),
        HudComponentTextRun::from(run("b", 0xffffff, Some(0x80123456))),
    ];
    let mut budget = 16;
    let sanitized = sanitize_hud_component_runs(runs.clone(), "", &mut budget, false);
    assert_eq!(sanitized, runs);
    let lines = hud_wrap_component_runs_with_interactions(&runs, &[], 6, &fonts);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].runs[0].visual.shadow_color, Some(0));
    assert_eq!(lines[1].runs[0].visual.shadow_color, Some(0x80123456));
    assert!(hud_chat_line_geometry(&runs, &[], &fonts).is_none());
    assert!(hud_chat_line_geometry(&[HudComponentTextRun::plain("ab")], &[], &fonts).is_some());
}

#[test]
fn shadow_chat_cached_and_immediate_draws_match_during_fade() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer
        .upload_hud_font_atlas(1, 1, &[255; 4], component_test_font_book())
        .unwrap();
    let mut visual = run("aa", 0x123456, None);
    visual.style.underlined = true;
    visual.style.bold = true;
    renderer.set_hud_chat_messages(vec![HudChatMessage {
        display_sequence: 1,
        added_tick: 0,
        runs: vec![HudComponentTextRun::from(visual)],
        interactions: vec![],
        tag: None,
    }]);
    assert!(renderer.hud_chat_lines[0].cached_geometry.is_some());
    renderer.set_hud_chat_context(HudChatContext {
        gui_ticks: 190,
        ..Default::default()
    });
    let vertices = |renderer: &Renderer| {
        renderer
            .collect_hud_draws()
            .vertices
            .iter()
            .map(|v| (v.position, v.uv, v.tint, v.local_uv))
            .collect::<Vec<_>>()
    };
    let cached = vertices(&renderer);
    assert!(cached.iter().any(|v| v.2[3] == 63.0 / 255.0));
    renderer.hud_chat_lines[0].cached_geometry = None;
    assert_eq!(vertices(&renderer), cached);
}

#[test]
fn shadow_screen_full_frame_blends_later_shadow_over_prior_glyph_without_default_shadow() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });
    renderer.update_camera();
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, width) in [('a', 6), ('b', 2)] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                advance: 2.0,
                width,
                height: 6,
                shadow_offset: 2.0,
                uv: HudUvRect {
                    min: [0.0; 2],
                    max: [1.0; 2],
                },
                ..Default::default()
            },
        );
    }
    renderer
        .upload_hud_font_atlas(1, 1, &[255; 4], HudFontGlyphBook::from(glyphs))
        .unwrap();
    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.width = 320;
    screen.height = 240;
    screen.text_labels.push(HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: 10,
        y: 10,
        width: 100,
        text: "ab".into(),
        tint: HUD_TINT_WHITE,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs: vec![
            run("a", 0xff0000, Some(0)),
            run("b", 0x00ff00, Some(0x800000ff)),
        ],
    });
    renderer.set_hud_inventory_screen(Some(screen.clone()));
    let pixels = renderer.render_offscreen_frame().unwrap();
    assert_eq!(pixels.pixel(15, 13), [127, 0, 128, 255]);
    assert_eq!(pixels.pixel(12, 11), [0, 255, 0, 255]);
    assert_eq!(pixels.pixel(15, 17), [0, 0, 128, 255]);
    screen.text_labels[0].shadow = true;
    screen.text_labels[0].runs[1].shadow_color = Some(0);
    renderer.set_hud_inventory_screen(Some(screen));
    let pixels = renderer.render_offscreen_frame().unwrap();
    assert_eq!(pixels.pixel(15, 13), [255, 0, 0, 255]);
    assert_eq!(pixels.pixel(15, 17), [0, 0, 0, 255]);
}
