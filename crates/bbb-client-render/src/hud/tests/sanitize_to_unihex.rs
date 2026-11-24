use super::*;

#[test]
fn sign_editor_ime_caret_is_projected_in_logical_gui_coordinates() {
    // The same fixture as the Vanilla-geometry test, but at the renderer's
    // logical GUI surface for a 1920x1080 framebuffer at scale 4: the
    // caret must land at the logical center (240) and the caller converts
    // back to physical at the platform boundary.
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, advance) in [('a', 5), ('b', 6), ('c', 6), ('?', 6)] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                advance: advance as f32,
                ..HudAsciiGlyph::default()
            },
        );
    }
    let screen = HudSignEditorScreen {
        kind: HudSignEditorKind::Standing {
            wood: SignModelWood::Oak,
            attachment: SignModelAttachment::Standing,
        },
        sign_preview: None,
        title: "Edit Sign Message".to_string(),
        lines: [
            "ab".to_string(),
            String::new(),
            String::new(),
            String::new(),
        ],
        line: 0,
        cursor: 1,
        selection: 1,
        cursor_visible: true,
        text_tint: [0.0, 0.0, 0.0, 1.0],
        done: "Done".to_string(),
        done_hovered: true,
        preedit: Some(HudImePreedit {
            text: "c".to_string(),
            focused_start: 0,
            focused_end: 1,
            caret: Some(1),
            caret_visible: true,
        }),
    };
    let logical_gui = PhysicalSize::new(480, 270);
    let projection = hud_sign_editor_preedit_projection(&screen, logical_gui, &glyphs).unwrap();
    assert_eq!(projection.ime_position, PhysicalPosition::new(238, 68));
    // A physical-framebuffer layout would center at 960, not 240.
    let physical_layout =
        hud_sign_editor_preedit_projection(&screen, PhysicalSize::new(1920, 1080), &glyphs)
            .unwrap();
    assert_ne!(physical_layout.ime_position, projection.ime_position);
}

#[test]
fn sound_subtitle_offscreen_pixels_pin_background_brightness_and_arrow() {
    use crate::camera::ClearColor;

    let Some(mut renderer) = Renderer::new_offscreen(64, 64) else {
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer.update_camera();
    let mut glyphs = HudFontGlyphMap::new();
    for (character, advance) in [('N', 6), ('<', 6), ('>', 6), (' ', 4)] {
        glyphs.insert_first_wins(
            character,
            HudAsciiGlyph {
                uv: HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                width: 1,
                height: 1,
                advance: advance as f32,
                bearing_top: HUD_FONT_BASELINE as f32,
                ..HudAsciiGlyph::default()
            },
        );
    }
    renderer
        .upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], glyphs)
        .expect("sound subtitle font");
    renderer.set_hud_sound_subtitles(HudSoundSubtitles {
        rows: vec![HudSoundSubtitleRow {
            text: "N".to_string(),
            brightness: 75,
            direction: HudSoundSubtitleDirection::Right,
        }],
    });

    let pixels = renderer
        .render_offscreen_frame()
        .expect("sound subtitle frame");

    let background = pixels.pixel(36, 24);
    assert!(
        background[2] >= 48 && background[2] <= 54 && background[0] < 4,
        "80% black background over blue: {background:?}"
    );
    let text = pixels.pixel(46, 25);
    assert!(
        text.iter()
            .take(3)
            .all(|channel| (72..=78).contains(channel)),
        "75-brightness centered text: {text:?}"
    );
    let arrow = pixels.pixel(56, 25);
    assert!(
        arrow
            .iter()
            .take(3)
            .all(|channel| (72..=78).contains(channel)),
        "right arrow shares row brightness: {arrow:?}"
    );
}

#[test]
fn unihex_cjk_offscreen_uses_scaled_bitmap_and_common_hud_text_path() {
    use crate::camera::ClearColor;

    let Some(mut renderer) = Renderer::new_offscreen(96, 48) else {
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer.update_camera();
    let mut rgba = vec![0_u8; 16 * 16 * 4];
    for y in 1..15_usize {
        for x in 1..15_usize {
            if matches!(x, 1 | 7 | 14) || matches!(y, 1 | 7 | 14) {
                let offset = (y * 16 + x) * 4;
                rgba[offset..offset + 4].copy_from_slice(&[255; 4]);
            }
        }
    }
    let glyph = HudAsciiGlyph {
        uv: HudUvRect {
            min: [0.01 / 16.0, 0.01 / 16.0],
            max: [1.0 - 0.01 / 16.0, 1.0 - 0.01 / 16.0],
        },
        width: 16,
        height: 16,
        advance: 9.0,
        render_scale: 0.5,
        bold_offset: 0.5,
        shadow_offset: 0.5,
        ..HudAsciiGlyph::default()
    };
    let mut glyphs = HudFontGlyphMap::new();
    for codepoint in ['中', '文', '?'] {
        glyphs.insert_first_wins(codepoint, glyph);
    }
    renderer
        .upload_hud_font_atlas(16, 16, &rgba, glyphs)
        .expect("unihex fixture atlas");
    renderer.set_hud_debug_overlay(Some(HudDebugOverlay {
        left_lines: vec!["中文".to_string()],
        ..HudDebugOverlay::default()
    }));

    let pixels = renderer.render_offscreen_frame().expect("CJK HUD frame");
    let bright_pixels = pixels
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel[0] > 200 && pixel[1] > 200 && pixel[2] > 200)
        .count();
    assert!(
        bright_pixels >= 20,
        "CJK glyphs did not reach the HUD frame"
    );
    if let Some(path) = std::env::var_os("BBB_UNIHEX_ANCHOR_PATH") {
        image::save_buffer(
            path,
            &pixels.rgba,
            pixels.width,
            pixels.height,
            image::ColorType::Rgba8,
        )
        .expect("save CJK visual anchor");
    }
}

#[test]
fn the_offhand_slot_and_hotbar_attack_indicator_draw_only_when_projected() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer
        .add_test_gui_sprite(
            "hud/hotbar",
            182,
            22,
            &[255, 255, 255, 255].repeat(182 * 22),
        )
        .expect("hotbar");
    renderer
        .add_test_gui_sprite(
            "hud/hotbar_offhand_left",
            29,
            24,
            &[255, 0, 0, 255].repeat(29 * 24),
        )
        .expect("offhand left");
    renderer
        .add_test_gui_sprite(
            "hud/hotbar_offhand_right",
            29,
            24,
            &[0, 255, 0, 255].repeat(29 * 24),
        )
        .expect("offhand right");
    renderer
        .add_test_gui_sprite(
            "hud/hotbar_attack_indicator_background",
            18,
            18,
            &[0; 4].repeat(18 * 18),
        )
        .expect("indicator background");
    renderer
        .add_test_gui_sprite(
            "hud/hotbar_attack_indicator_progress",
            18,
            18,
            &[0; 4].repeat(18 * 18),
        )
        .expect("indicator progress");
    renderer.set_hud_hotbar_mode(HudHotbarMode::Ordinary);

    assert_eq!(renderer.collect_hud_draws().commands.len(), 1);

    renderer.set_hud_offhand_slot(Some(HudOffhandSlot {
        on_left: true,
        icon: HudItemIcon::single(HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        }),
    }));
    assert!(renderer.hud_offhand_slot.is_some());
    assert_eq!(renderer.collect_hud_draws().commands.len(), 2);

    renderer.set_hud_hotbar_attack_indicator(HudHotbarAttackIndicator::Progress {
        strength_scale: 0.5,
        main_arm_left: false,
    });
    assert_eq!(renderer.collect_hud_draws().commands.len(), 4);

    renderer.set_hud_hotbar_attack_indicator(HudHotbarAttackIndicator::Progress {
        strength_scale: 0.0,
        main_arm_left: false,
    });
    assert_eq!(renderer.collect_hud_draws().commands.len(), 3);

    renderer.set_hud_hotbar_attack_indicator(HudHotbarAttackIndicator::Progress {
        strength_scale: f32::NAN,
        main_arm_left: false,
    });
    assert_eq!(
        renderer.hud_hotbar_attack_indicator,
        HudHotbarAttackIndicator::None
    );

    renderer.set_hud_offhand_slot(None);
    renderer.set_hud_hotbar_mode(HudHotbarMode::Hidden);
    assert_eq!(renderer.collect_hud_draws().commands.len(), 0);
}

#[test]
fn test_block_editor_sanitizes_message_utf16_boundary() {
    let screen = sanitize_hud_test_block_editor_screen(HudTestBlockEditorScreen {
        title: "Test Block".to_string(),
        message_label: "Message:".to_string(),
        message: HudTextField {
            text: "😀".repeat(200),
            cursor: usize::MAX,
            selection: usize::MAX,
            scroll_to: usize::MAX,
            max_length: usize::MAX,
            focused: true,
            ..HudTextField::default()
        },
        message_visible: true,
        mode: "Log".to_string(),
        mode_info: "Log Mode - Log a message".to_string(),
        done: "Done".to_string(),
        cancel: "Cancel".to_string(),
        mode_hovered: false,
        done_hovered: false,
        cancel_hovered: false,
    })
    .unwrap();
    assert_eq!(screen.message.text.encode_utf16().count(), 128);
    assert_eq!(screen.message.cursor, 128);
    assert_eq!(screen.message.selection, 128);
    assert_eq!(screen.message.max_length, 128);
}

#[test]
fn space_provider_zero_pixel_glyphs_advance_without_a_visible_quad() {
    // EmptyGlyph retains advance without a bitmap submission.
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, advance, width, height) in [
        ('a', 6, 6, 8),
        ('b', 6, 6, 8),
        ('?', 5, 6, 8),
        (' ', 4, 0, 0),
        ('\u{200c}', 0, 0, 0),
    ] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                is_empty: matches!(ch, ' ' | '\u{200c}'),
                advance: advance as f32,
                width,
                height,
                ..HudAsciiGlyph::default()
            },
        );
    }

    let zwnj = hud_font_glyph('\u{200c}', &glyphs);
    assert_eq!(zwnj.advance, 0.0);
    // No `?` fallback: the space provider glyph resolves directly.
    assert_ne!(zwnj, hud_font_glyph('?', &glyphs));
    assert!(zwnj.is_empty);
    assert!(zwnj
        .styled_quads(0.0, 0.0, Default::default(), false)
        .is_empty());

    // Inserting a ZWNJ between two glyphs must not change the total
    // advance width.
    assert_eq!(
        hud_font_text_width("a\u{200c}b", &glyphs),
        hud_font_text_width("ab", &glyphs)
    );
}

#[test]
fn styled_text_pass_geometry_plain_runs_match_the_legacy_axis_aligned_cells() {
    // Default-style runs must reproduce the old per-glyph rect path
    // exactly: one axis-aligned cell per glyph at the pen position, no
    // effects, with the base tint encoded as ARGB channels.
    let glyphs = styled_test_glyphs();
    let geometry = hud_styled_text_geometry(
        &[HudStyledTextRun::plain("ab")],
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        0,
        (100.0, 50.0),
        false,
        [1.0, 0.5, 0.25, 1.0],
        None,
        1.0,
    );
    assert!(geometry.effect_rects.is_empty());
    assert_eq!(geometry.glyph_quads.len(), 2);
    assert_eq!(
        geometry.glyph_quads[0].0.corners,
        [[100.0, 50.0], [100.0, 58.0], [105.0, 58.0], [105.0, 50.0],]
    );
    // Second glyph pen-advanced by the plain advance (6).
    assert_eq!(geometry.glyph_quads[1].0.corners[0], [106.0, 50.0]);
    assert!(geometry
        .glyph_quads
        .iter()
        .all(|(_, tint)| *tint == [1.0, 127.0 / 255.0, 63.0 / 255.0, 1.0]));
}

#[test]
fn styled_text_geometry_uses_selected_rtl_fallback_for_neutral_lines() {
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, uv_x) in [('1', 0.1), ('2', 0.2), ('3', 0.3), ('-', 0.4)] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                uv: HudUvRect {
                    min: [uv_x, 0.0],
                    max: [uv_x + 0.05, 1.0],
                },
                width: 5,
                height: 8,
                advance: 6.0,
                ..HudAsciiGlyph::default()
            },
        );
    }
    glyphs.set_default_right_to_left(true);

    let geometry = hud_styled_text_geometry(
        &[HudStyledTextRun::plain("123-")],
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        0,
        (0.0, 0.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    assert_eq!(
        geometry
            .glyph_quads
            .iter()
            .map(|(quad, _)| quad.uv.min[0])
            .collect::<Vec<_>>(),
        vec![0.4, 0.1, 0.2, 0.3]
    );
}

#[test]
fn styled_text_width_and_geometry_use_icu_lamalef_visual_glyph() {
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, advance, uv_x) in [
        ('ل', 9_u32, 0.1),
        ('ا', 8_u32, 0.2),
        ('\u{fefb}', 5_u32, 0.7),
    ] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                uv: HudUvRect {
                    min: [uv_x, 0.0],
                    max: [uv_x + 0.05, 1.0],
                },
                width: advance.saturating_sub(1),
                height: 8,
                advance: advance as f32,
                ..HudAsciiGlyph::default()
            },
        );
    }
    let runs = [HudStyledTextRun::plain("لا")];

    assert_eq!(hud_font_runs_width(&runs, &glyphs), Some(5));
    assert_eq!(hud_logical_font_runs_width(&runs, &glyphs), Some(17));
    assert_eq!(hud_raw_string_width("لا", &glyphs), 17);
    let geometry = hud_styled_text_geometry(
        &runs,
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        0,
        (0.0, 0.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    assert_eq!(geometry.glyph_quads.len(), 1);
    assert_eq!(geometry.glyph_quads[0].0.uv.min[0], 0.7);

    let logical_geometry = hud_logical_styled_text_geometry(
        &runs,
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        0,
        (0.0, 0.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    assert_eq!(logical_geometry.glyph_quads.len(), 2);
    assert_eq!(
        logical_geometry
            .glyph_quads
            .iter()
            .map(|(quad, _)| quad.uv.min[0])
            .collect::<Vec<_>>(),
        vec![0.1, 0.2]
    );
}

#[test]
fn styled_text_pass_geometry_bold_runs_double_quads_and_widen_the_pen() {
    let glyphs = styled_test_glyphs();
    let geometry = hud_styled_text_geometry(
        &[HudStyledTextRun {
            shadow_color: None,
            text: "ab".to_string(),
            style: HudTextStyle {
                bold: true,
                ..Default::default()
            },
            color: None,
        }],
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        0,
        (100.0, 50.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    // Vanilla bold: each glyph renders twice, the second pass shifted by
    // getBoldOffset()=1.
    assert_eq!(geometry.glyph_quads.len(), 4);
    for pair in geometry.glyph_quads.chunks(2) {
        for (main_corner, bold_corner) in pair[0].0.corners.iter().zip(pair[1].0.corners.iter()) {
            assert!((bold_corner[0] - main_corner[0] - HUD_FONT_BOLD_OFFSET).abs() < 1e-4);
            assert!((bold_corner[1] - main_corner[1]).abs() < 1e-4);
        }
    }
    // The pen advances by the bold-aware advance (6 + 1), with the
    // extraThickness=0.1 expansion on the cell.
    let second_char_left = geometry.glyph_quads[2].0.corners[0][0];
    assert!((second_char_left - (107.0 - HUD_FONT_BOLD_EXTRA_THICKNESS)).abs() < 1e-4);
}

#[test]
fn styled_text_pass_geometry_emits_underline_and_strikethrough_bars_after_glyphs() {
    let glyphs = styled_test_glyphs();
    let geometry = hud_styled_text_geometry(
        &[HudStyledTextRun {
            shadow_color: None,
            text: "a".to_string(),
            style: HudTextStyle {
                underlined: true,
                strikethrough: true,
                ..Default::default()
            },
            color: None,
        }],
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        0,
        (100.0, 50.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    assert_eq!(geometry.glyph_quads.len(), 1);
    assert_eq!(geometry.effect_rects.len(), 2);
    let (strike, _) = &geometry.effect_rects[0];
    let (underline, _) = &geometry.effect_rects[1];
    assert_eq!(strike.kind, bbb_render_types::HudEffectKind::Strikethrough);
    assert_eq!(underline.kind, bbb_render_types::HudEffectKind::Underline);
    // First-in-line bars extend one pixel left (vanilla position == 0)
    // and span to x + advance.
    assert!((strike.x0 - 99.0).abs() < 1e-4);
    assert!((strike.x1 - 106.0).abs() < 1e-4);
    // Vanilla bar bands: strikethrough y+3.5..y+4.5, underline y+8..y+9.
    assert!((strike.y0 - 53.5).abs() < 1e-4);
    assert!((strike.y1 - 54.5).abs() < 1e-4);
    assert!((underline.y0 - 58.0).abs() < 1e-4);
    assert!((underline.y1 - 59.0).abs() < 1e-4);
}

#[test]
fn styled_text_pass_geometry_shadow_pass_offsets_and_scales_the_run_color() {
    let glyphs = styled_test_glyphs();
    let run = HudStyledTextRun {
        shadow_color: None,
        text: "a".to_string(),
        style: HudTextStyle::default(),
        color: Some(0xFF_55_55), // ChatFormatting RED
    };
    let main = hud_styled_text_geometry(
        std::slice::from_ref(&run),
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        0,
        (100.0, 50.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    let shadow = hud_styled_text_geometry(
        &[run],
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        0,
        (100.0, 50.0),
        true,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    // Run colour overrides the line tint on the main pass.
    assert_eq!(
        main.glyph_quads[0].1,
        [1.0, 85.0 / 255.0, 85.0 / 255.0, 1.0]
    );
    // Shadow pass: same glyph at +1,+1 in the vanilla shadow colour
    // (ARGB.scaleRGB(textColor, 0.25)).
    assert_eq!(
        shadow.glyph_quads[0].1,
        [63.0 / 255.0, 21.0 / 255.0, 21.0 / 255.0, 1.0]
    );
    for (shadow_corner, main_corner) in shadow.glyph_quads[0]
        .0
        .corners
        .iter()
        .zip(main.glyph_quads[0].0.corners.iter())
    {
        assert!((shadow_corner[0] - main_corner[0] - 1.0).abs() < 1e-4);
        assert!((shadow_corner[1] - main_corner[1] - 1.0).abs() < 1e-4);
    }
    assert_eq!(
        HudStyledTextRun::resolve_shadow_argb(0xffffffff, None, true),
        0xff3f3f3f
    );
}

#[test]
fn styled_text_pass_geometry_honors_the_label_width_budget() {
    let mut glyphs = HudFontGlyphMap::new();
    // Vanilla ascii.png uses 8-pixel cells with advances 6 for O and 5 for k.
    for (ch, advance) in [('O', 6.0), ('k', 5.0)] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                width: 8,
                height: 8,
                advance,
                ..HudAsciiGlyph::default()
            },
        );
    }
    let runs = [HudStyledTextRun::plain("Ok")];
    assert_eq!(hud_coded_string_width(&runs, &glyphs), 11);
    for (budget, expected_quads) in [(11, 2), (10, 1)] {
        let geometry = hud_styled_text_geometry(
            &runs,
            &glyphs,
            &HudObfuscatedGlyphPool::default(),
            0,
            (0.0, 0.0),
            false,
            HUD_TINT_WHITE,
            Some(budget),
            1.0,
        );
        assert_eq!(geometry.glyph_quads.len(), expected_quads);
        assert_eq!(geometry.glyph_quads[0].0.corners[0], [0.0, 0.0]);
        if budget == 11 {
            assert_eq!(geometry.glyph_quads[1].0.corners[0], [6.0, 0.0]);
        }
    }
}

#[test]
fn styled_text_pass_geometry_scale_multiplies_pen_cells_shadow_and_effects() {
    // `scale` mirrors a vanilla pose scale around the origin: cells, the
    // pen advance, the shadow offset and effect bars all multiply.
    let glyphs = styled_test_glyphs();
    let geometry = hud_styled_text_geometry(
        &[HudStyledTextRun::plain("ab")],
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        0,
        (100.0, 50.0),
        false,
        HUD_TINT_WHITE,
        None,
        4.0,
    );
    assert_eq!(geometry.glyph_quads.len(), 2);
    // 5x8 cell at the origin becomes 20x32.
    assert_eq!(
        geometry.glyph_quads[0].0.corners,
        [[100.0, 50.0], [100.0, 82.0], [120.0, 82.0], [120.0, 50.0]]
    );
    // Second glyph: pen advance 6 font px -> 24 HUD px.
    assert_eq!(geometry.glyph_quads[1].0.corners[0], [124.0, 50.0]);

    // The +1,+1 shadow offset rides the pose scale too (vanilla scales the
    // whole `textWithBackdrop` draw).
    let shadow = hud_styled_text_geometry(
        &[HudStyledTextRun::plain("a")],
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        0,
        (100.0, 50.0),
        true,
        HUD_TINT_WHITE,
        None,
        4.0,
    );
    assert_eq!(shadow.glyph_quads[0].0.corners[0], [104.0, 54.0]);

    // Underline bar: first-in-line -1 font px, span to advance 6, band
    // y+8..y+9, all doubled at scale 2.
    let effects = hud_styled_text_geometry(
        &[HudStyledTextRun {
            shadow_color: None,
            text: "a".to_string(),
            style: HudTextStyle {
                underlined: true,
                ..Default::default()
            },
            color: None,
        }],
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        0,
        (100.0, 50.0),
        false,
        HUD_TINT_WHITE,
        None,
        2.0,
    );
    let (bar, _) = &effects.effect_rects[0];
    assert_eq!((bar.x0, bar.x1), (98.0, 112.0));
    assert_eq!((bar.y0, bar.y1), (66.0, 68.0));
}

#[test]
fn text_background_options_split_chat_from_text_with_backdrop_consumers() {
    assert_eq!(hud_argb_black_alpha(0.5), 127.0 / 255.0);
    assert_eq!(hud_argb_black_alpha(0.25), 63.0 / 255.0);
    assert_eq!(hud_argb_black_alpha(f32::NAN), 0.0);
    assert_eq!(hud_text_backdrop_alpha(0.5, 127.0 / 255.0), 63.0 / 255.0);
    assert_eq!(hud_text_backdrop_alpha(1.0, 1.0), 1.0);
    assert_eq!(hud_text_backdrop_alpha(f32::NAN, 1.0), 0.0);
    assert_eq!(
        hud_text_backdrop_rect((136.0, 80.0), 12, 4.0),
        Some(HudRect {
            x: 128.0,
            y: 72.0,
            width: 64,
            height: 52,
        })
    );

    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer
        .upload_hud_font_atlas(
            1,
            1,
            &[255, 255, 255, 255],
            HudFontGlyphBook::from(styled_test_glyphs()),
        )
        .expect("font atlas");
    renderer.set_hud_action_bar_text(Some(HudActionBarText {
        runs: vec![HudComponentTextRun::plain("a")],
        remaining_ticks: 60,
        partial_tick: 0.0,
        animate_color: false,
    }));
    renderer.set_hud_chat_messages(vec![HudChatMessage {
        display_sequence: 1,
        added_tick: 0,
        runs: vec![HudComponentTextRun::plain("a")],
        interactions: Vec::new(),
        tag: None,
    }]);
    renderer.set_hud_chat_context(HudChatContext::default());

    // Action-bar origin is (157, 168), so (155, 166) is its top-left
    // 2px backdrop padding. Chat's first row begins at y=191; x=1 is
    // outside its glyph but inside the line background.
    let action_padding = (155, 166);
    let chat_padding = (1, 191);

    renderer.set_hud_text_background_options(0.0, false);
    let disabled = renderer.render_offscreen_frame().expect("disabled frame");
    assert_eq!(
        disabled.pixel(action_padding.0, action_padding.1),
        [0, 0, 255, 255]
    );
    assert_eq!(
        disabled.pixel(chat_padding.0, chat_padding.1),
        [0, 0, 255, 255]
    );

    renderer.set_hud_text_background_options(0.5, true);
    let chat_only = renderer.render_offscreen_frame().expect("chat-only frame");
    assert_eq!(
        chat_only.pixel(action_padding.0, action_padding.1),
        [0, 0, 255, 255]
    );
    assert!(
        chat_only.pixel(chat_padding.0, chat_padding.1)[2] < 230,
        "chat opacity must darken the chat row"
    );

    renderer.set_hud_text_background_options(0.5, false);
    let everywhere = renderer.render_offscreen_frame().expect("everywhere frame");
    assert!(
        everywhere.pixel(action_padding.0, action_padding.1)[2] < 230,
        "everywhere mode must darken textWithBackdrop padding"
    );
    assert!(
        everywhere.pixel(157, 168)[0] > 230,
        "the backdrop must submit before the action-bar glyph"
    );
    assert!(
        everywhere.pixel(chat_padding.0, chat_padding.1)[2] < 230,
        "chat background must remain enabled in everywhere mode"
    );
}

#[test]
fn sleep_overlay_uses_vanilla_color_and_rejects_zero_alpha() {
    assert_eq!(
        hud_sleep_overlay_tint(220),
        [16.0 / 255.0, 16.0 / 255.0, 32.0 / 255.0, 220.0 / 255.0,]
    );
    assert_eq!(sanitize_hud_sleep_overlay(None), None);
    assert_eq!(
        sanitize_hud_sleep_overlay(Some(HudSleepOverlay { alpha: 0 })),
        None
    );
    assert_eq!(
        sanitize_hud_sleep_overlay(Some(HudSleepOverlay { alpha: 22 })),
        Some(HudSleepOverlay { alpha: 22 })
    );
}

#[test]
fn ttf_shaped_glyph_bearing_flows_through_the_live_styled_text_pass() {
    // A `ttf` provider's `GlyphBitmap.getLeft()` is `bitmap_left / oversample`
    // and is not zero, unlike the bitmap provider's default. The live styled
    // text pass must place the quad and the interaction area by that bearing
    // and still advance the pen by the glyph's own advance.
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(
        'a',
        HudAsciiGlyph {
            advance: 6.0,
            width: 8,
            height: 7,
            bearing_left: -1.0,
            bearing_top: 7.0,
            ..HudAsciiGlyph::default()
        },
    );
    let style = HudTextStyle::default();
    let geometry = hud_styled_text_geometry(
        &[HudStyledTextRun {
            shadow_color: None,
            text: "a".to_string(),
            style,
            color: None,
        }],
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        0,
        (100.0, 50.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    let expected = hud_font_glyph('a', &glyphs).styled_quads(100.0, 50.0, style, false);
    assert_eq!(geometry.glyph_quads.len(), 1);
    assert_eq!(geometry.glyph_quads[0].0.corners, expected[0].corners);
    // Negative bearing pulls the 8px cell one pixel left of the pen and the
    // quad still ends seven pixels right of it.
    assert_eq!(geometry.glyph_quads[0].0.corners[0], [99.0, 50.0]);
    assert_eq!(geometry.glyph_quads[0].0.corners[2], [107.0, 57.0]);
    // `activeRight` stays the pen plus the advance, not the cell extent.
    let area = hud_font_glyph('a', &glyphs).styled_glyph_area(100.0, 50.0, style, false);
    assert_eq!((area.left, area.right), (99.0, 106.0));
    assert_eq!((area.bounds_right, area.bottom), (107.0, 57.0));
}

#[test]
fn styled_text_pass_geometry_italic_shears_through_the_mechanism_primitive() {
    // The live path now feeds `run.style` straight into `styled_quads`, so
    // an italic run's corners must equal the locked mechanism output for the
    // same glyph/pen — a direct comparison against `styled_quads`.
    let glyphs = styled_test_glyphs();
    let style = HudTextStyle {
        italic: true,
        ..Default::default()
    };
    let geometry = hud_styled_text_geometry(
        &[HudStyledTextRun {
            shadow_color: None,
            text: "a".to_string(),
            style,
            color: None,
        }],
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        0,
        (100.0, 50.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    let expected = hud_font_glyph('a', &glyphs).styled_quads(100.0, 50.0, style, false);
    assert_eq!(geometry.glyph_quads.len(), expected.len());
    for (drawn, mech) in geometry.glyph_quads.iter().zip(expected.iter()) {
        assert_eq!(drawn.0.corners, mech.corners);
    }
    // Shear is real: the top edge shifts right of the plain cell.
    let plain =
        hud_font_glyph('a', &glyphs).styled_quads(100.0, 50.0, HudTextStyle::default(), false);
    assert!(geometry.glyph_quads[0].0.corners[0][0] > plain[0].corners[0][0]);
}

#[test]
fn styled_text_pass_geometry_non_italic_runs_do_not_regress() {
    // Releasing italic must leave every non-italic run byte-identical: a
    // bold+underlined run drawn upright equals the mechanism output.
    let glyphs = styled_test_glyphs();
    let style = HudTextStyle {
        bold: true,
        underlined: true,
        ..Default::default()
    };
    let geometry = hud_styled_text_geometry(
        &[HudStyledTextRun {
            shadow_color: None,
            text: "a".to_string(),
            style,
            color: None,
        }],
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        7,
        (100.0, 50.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    let glyph = hud_font_glyph('a', &glyphs);
    let expected = glyph.styled_quads(100.0, 50.0, style, false);
    for (drawn, mech) in geometry.glyph_quads.iter().zip(expected.iter()) {
        assert_eq!(drawn.0.corners, mech.corners);
    }
    // No shear on any corner (italic false).
    for quad in &geometry.glyph_quads {
        assert_eq!(quad.0.corners[0][0], quad.0.corners[1][0]);
    }
    // The seed is inert for non-obfuscated runs.
    let reseeded = hud_styled_text_geometry(
        &[HudStyledTextRun {
            shadow_color: None,
            text: "a".to_string(),
            style,
            color: None,
        }],
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        999,
        (100.0, 50.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    assert_eq!(geometry, reseeded);
}

#[test]
fn styled_text_pass_geometry_obfuscated_substitutes_equal_advance_glyphs() {
    let glyphs = obfuscated_test_glyphs();
    let pool = HudObfuscatedGlyphPool::from_glyph_map(&glyphs);
    let run = |text: &str| HudStyledTextRun {
        shadow_color: None,
        text: text.to_string(),
        style: HudTextStyle {
            obfuscated: true,
            ..Default::default()
        },
        color: None,
    };
    let pass = |seed: u64| {
        hud_styled_text_geometry(
            &[run("aaaa")],
            &glyphs,
            &pool,
            seed,
            (0.0, 0.0),
            false,
            HUD_TINT_WHITE,
            None,
            1.0,
        )
    };
    // Fixed seed -> deterministic, reproducible glyph sequence.
    let first = pass(42);
    assert_eq!(first, pass(42));
    assert_eq!(first.glyph_quads.len(), 4);
    // Every substitute is an advance-6 pooled glyph (one of the three uvs);
    // pen positions stay on the advance-6 grid (0, 6, 12, 18), so obfuscation
    // never shifts layout.
    let advance6_uvs = [0.1_f32, 0.2, 0.3];
    for (index, (quad, _)) in first.glyph_quads.iter().enumerate() {
        assert!(advance6_uvs.contains(&quad.uv.min[0]));
        assert_eq!(quad.corners[0][0], (index as f32) * 6.0);
    }
    // The four draws are not all the same glyph (the LCG actually varies).
    let uvs: Vec<f32> = first.glyph_quads.iter().map(|(q, _)| q.uv.min[0]).collect();
    assert!(uvs.iter().any(|uv| *uv != uvs[0]));
    // A different frame seed changes the sequence (per-frame jitter).
    let other = pass(43);
    let other_uvs: Vec<f32> = other.glyph_quads.iter().map(|(q, _)| q.uv.min[0]).collect();
    assert_ne!(uvs, other_uvs);
}

#[test]
fn styled_text_pass_geometry_obfuscated_never_substitutes_spaces() {
    // Vanilla `Font.getGlyph` guards `codepoint != 32`: spaces stay spaces.
    // Map: visible 'a' (advance 6) + empty space (advance 4) + a *visible*
    // advance-4 glyph 'd', so a broken guard would draw 'd' where the space
    // is. The space must stay invisible for every seed.
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(
        'a',
        HudAsciiGlyph {
            uv: HudUvRect {
                min: [0.1, 0.0],
                max: [0.15, 0.1],
            },
            advance: 6.0,
            width: 5,
            height: 8,
            ..HudAsciiGlyph::default()
        },
    );
    glyphs.insert_first_wins(' ', HudAsciiGlyph::empty(4.0));
    glyphs.insert_first_wins(
        'd',
        HudAsciiGlyph {
            uv: HudUvRect {
                min: [0.4, 0.0],
                max: [0.45, 0.1],
            },
            advance: 4.0,
            width: 5,
            height: 8,
            ..HudAsciiGlyph::default()
        },
    );
    let pool = HudObfuscatedGlyphPool::from_glyph_map(&glyphs);
    for seed in 0..16u64 {
        let geometry = hud_styled_text_geometry(
            &[HudStyledTextRun {
                shadow_color: None,
                text: "a a".to_string(),
                style: HudTextStyle {
                    obfuscated: true,
                    ..Default::default()
                },
                color: None,
            }],
            &glyphs,
            &pool,
            seed,
            (0.0, 0.0),
            false,
            HUD_TINT_WHITE,
            None,
            1.0,
        );
        // Only the two visible 'a's draw; the space never becomes a glyph.
        assert_eq!(geometry.glyph_quads.len(), 2, "seed {seed}");
        // The space advances by 4, so the second glyph sits at pen 6 + 4.
        assert_eq!(geometry.glyph_quads[0].0.corners[0][0], 0.0);
        assert_eq!(geometry.glyph_quads[1].0.corners[0][0], 10.0);
    }
}

#[test]
fn scrolling_widget_scissor_intersects_the_dialog_body_scissor() {
    let screen = hud_inventory_screen_with_entity_previews(Vec::new());
    let surface_size = PhysicalSize::new(320, 240);
    let parent = hud_inventory_scissor_rect(
        surface_size,
        &screen,
        HudInventoryItemScissor {
            x: 10,
            y: 20,
            width: 80,
            height: 60,
        },
    );
    let intersection = hud_inventory_nested_scissor_rect(
        surface_size,
        &screen,
        HudInventoryItemScissor {
            x: 0,
            y: 30,
            width: 100,
            height: 20,
        },
        Some(parent),
    )
    .unwrap();

    assert_eq!(hud_rect_bounds(intersection), (82.0, 67.0, 162.0, 87.0));
}

#[test]
fn scrolling_widget_draws_restore_the_enclosing_body_scissor() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    let glyph = HudAsciiGlyph {
        uv: HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        width: 1,
        height: 1,
        advance: 6.0,
        bearing_top: HUD_FONT_BASELINE as f32,
        ..HudAsciiGlyph::default()
    };
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins('a', glyph);
    renderer
        .upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], glyphs)
        .unwrap();

    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.scroll_area = Some(HudInventoryScrollArea {
        scissor: HudInventoryItemScissor {
            x: 2,
            y: 30,
            width: 10,
            height: 20,
        },
        background_layers: Vec::new(),
        fill_layers: Vec::new(),
        text_labels: vec![HudInventoryTextLabel {
            component_runs: Vec::new(),
            x: 0,
            y: 30,
            width: 10,
            text: "aaaa".to_string(),
            tint: HUD_TINT_WHITE,
            background: None,
            input: None,
            scrolling: Some(HudInventoryScrollingText {
                x: 0,
                y: 30,
                width: 10,
                height: 20,
            }),
            shadow: false,
            font: None,
            runs: vec![HudStyledTextRun::plain("aaaa")],
        }],
        text_areas: Vec::new(),
        scrollbar: None,
    });
    renderer.set_hud_inventory_screen(Some(screen));

    let draws = renderer.collect_hud_draws();
    let scissors = draws
        .commands
        .iter()
        .filter_map(|command| match command {
            HudDrawCommand::Scissor {
                x,
                y,
                width,
                height,
            } => Some((*x, *y, *width, *height)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(scissors.ends_with(&[
        (74, 67, 10, 20),
        (74, 67, 8, 20),
        (74, 67, 10, 20),
        (0, 0, 320, 240),
    ]));
}

#[test]
fn sanitize_inventory_keeps_entity_in_ui_preview_render_plan() {
    let screen = sanitize_hud_inventory_screen(hud_inventory_screen_with_entity_previews(vec![
        hud_entity_preview_for_test(),
    ]));

    assert_eq!(screen.entity_previews.len(), 1);
    let preview = &screen.entity_previews[0];
    assert_eq!(preview.lighting, GuiItemLightingEntry::EntityInUi);
    assert_eq!(
        preview.rect,
        HudEntityPreviewRect {
            x: 26,
            y: 8,
            width: 49,
            height: 70,
        }
    );
    assert_eq!(
        preview.visible_bounds(),
        Some(HudEntityPreviewRect {
            x: 30,
            y: 20,
            width: 10,
            height: 12,
        })
    );
    assert_eq!(preview.translation, [0.0, 0.875, 0.0]);
    assert_eq!(preview.rotation, [0.0, 0.0, 1.0, 0.0]);
    assert_eq!(preview.scale, 30.0);
    assert!(preview.depth_isolated);
    assert_eq!(
        preview.entity.render_state.light_coords,
        ENTITY_FULL_BRIGHT_LIGHT_COORDS
    );
    assert_eq!(preview.entity.render_state.outline_color, 0);
    assert!(!preview.entity.render_state.appears_glowing);
    assert_eq!(
        preview.item_layers,
        vec![HudEntityPreviewItemLayer {
            slot: HudEntityPreviewItemSlot::LeftHand,
            display_context: HudEntityPreviewItemDisplayContext::ThirdPersonLeftHand,
            item_id: 12,
            count: 1,
            foil: true,
            light_coords: ENTITY_FULL_BRIGHT_LIGHT_COORDS,
            overlay: ITEM_MODEL_NO_OVERLAY,
            order: 0,
            submit_sequence: 1,
        }]
    );
}

#[test]
fn sanitize_sign_editor_screen_clamps_text_state_and_drops_hanging_preview() {
    let sign_preview = HudEntityPreview {
        entity: EntityModelInstance::sign(
            -1,
            [0.0, 0.0, 0.0],
            0.0,
            SignModelWood::Oak,
            SignModelAttachment::Standing,
        ),
        lighting: GuiItemLightingEntry::ItemsFlat,
        rect: HudEntityPreviewRect {
            x: 0,
            y: 0,
            width: 96,
            height: 102,
        },
        scissor: None,
        translation: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        override_camera_rotation: None,
        scale: 62.500_004,
        depth_isolated: true,
        after_background_layer: None,
        item_layers: Vec::new(),
    };
    let standing = sanitize_hud_sign_editor_screen(HudSignEditorScreen {
        kind: HudSignEditorKind::Standing {
            wood: SignModelWood::Oak,
            attachment: SignModelAttachment::Standing,
        },
        sign_preview: Some(sign_preview.clone()),
        title: "Edit Sign Message".to_string(),
        lines: [
            String::new(),
            "front".to_string(),
            "third".to_string(),
            "fourth".to_string(),
        ],
        line: 7,
        cursor: 99,
        selection: 98,
        cursor_visible: true,
        text_tint: [2.0, 0.5, -1.0, 1.5],
        done: "Done".to_string(),
        done_hovered: false,
        preedit: None,
    })
    .expect("standing sign editor");
    assert_eq!(standing.line, 3);
    assert_eq!(standing.cursor, "fourth".chars().count());
    assert_eq!(standing.selection, "fourth".chars().count());
    assert_eq!(standing.lines[0], "");
    assert_eq!(standing.text_tint, [1.0, 0.5, 0.0, 1.0]);
    assert!(standing.sign_preview.is_some());

    let hanging = sanitize_hud_sign_editor_screen(HudSignEditorScreen {
        kind: HudSignEditorKind::Hanging {
            wood: SignModelWood::Bamboo,
        },
        sign_preview: Some(sign_preview),
        title: "Edit Hanging Sign Message".to_string(),
        lines: std::array::from_fn(|_| String::new()),
        line: 0,
        cursor: 0,
        selection: 0,
        cursor_visible: true,
        text_tint: [1.0, 1.0, 1.0, 1.0],
        done: "Done".to_string(),
        done_hovered: false,
        preedit: None,
    })
    .expect("hanging sign editor");
    assert!(hanging.sign_preview.is_none());
}

#[test]
fn sanitize_pause_screen_strips_control_text_and_drops_empty_title() {
    let screen = sanitize_hud_pause_screen(HudPauseScreen {
        title: "Game\nPaused".to_string(),
        labels: HudPauseScreenLabels {
            return_to_game: "Zurueck\nzum Spiel".to_string(),
            advancements: String::new(),
            ..HudPauseScreenLabels::default()
        },
        show_pause_menu: false,
        feedback_subscreen: false,
        return_to_game_hovered: true,
        advancements_hovered: true,
        stats_hovered: true,
        send_feedback_hovered: true,
        report_bugs_hovered: true,
        report_bugs_enabled: false,
        custom_dialog_label: Some("Server\nOptions".to_string()),
        custom_dialog_hovered: true,
        custom_dialog_tooltip: Some(HudPauseScreenTooltip {
            text: "Third party\nHandle carefully".to_string(),
            x: 5,
            y: 6,
        }),
        feedback_back_hovered: true,
        options_hovered: true,
        options_enabled: false,
        options_tooltip: Some(HudPauseScreenTooltip {
            text: "Options via CLI\nor options.txt".to_string(),
            x: 10,
            y: 20,
        }),
        disconnect_hovered: true,
        disconnect_enabled: false,
    })
    .expect("pause screen");
    assert_eq!(screen.title, "GamePaused");
    assert_eq!(screen.labels.return_to_game, "Zurueckzum Spiel");
    assert_eq!(screen.labels.advancements, "Advancements");
    assert!(!screen.show_pause_menu);
    assert!(screen.return_to_game_hovered);
    assert!(screen.advancements_hovered);
    assert!(screen.stats_hovered);
    assert!(screen.send_feedback_hovered);
    assert!(screen.report_bugs_hovered);
    assert!(!screen.report_bugs_enabled);
    assert_eq!(screen.custom_dialog_label.as_deref(), Some("ServerOptions"));
    assert_eq!(
        screen
            .custom_dialog_tooltip
            .as_ref()
            .map(|tooltip| tooltip.text.as_str()),
        Some("Third party\nHandle carefully")
    );
    assert_eq!(
        screen
            .options_tooltip
            .as_ref()
            .map(|tooltip| tooltip.text.as_str()),
        Some("Options via CLI\nor options.txt")
    );
    assert!(screen.options_hovered);
    assert!(!screen.options_enabled);
    assert!(screen.disconnect_hovered);
    assert!(!screen.disconnect_enabled);

    assert!(sanitize_hud_pause_screen(HudPauseScreen {
        title: "\n\t".to_string(),
        labels: HudPauseScreenLabels::default(),
        show_pause_menu: true,
        feedback_subscreen: false,
        return_to_game_hovered: false,
        advancements_hovered: false,
        stats_hovered: false,
        send_feedback_hovered: false,
        report_bugs_hovered: false,
        report_bugs_enabled: true,
        custom_dialog_label: None,
        custom_dialog_hovered: false,
        custom_dialog_tooltip: None,
        feedback_back_hovered: false,
        options_hovered: false,
        options_enabled: false,
        options_tooltip: None,
        disconnect_hovered: false,
        disconnect_enabled: true,
    })
    .is_none());
}

#[test]
fn sanitize_stats_screen_strips_control_text_and_drops_empty_lines() {
    let screen = sanitize_hud_stats_screen(HudStatsScreen {
        title: "St\nats".to_string(),
        loading_text: "Downloading\n statistics...".to_string(),
        is_loading: false,
        general_tab: "Gen\neral".to_string(),
        items_tab: "Items".to_string(),
        mobs_tab: "Mobs".to_string(),
        rows: vec![HudStatsRow {
            label: "Walk\none cm".to_string(),
            value: "1\n.00 m".to_string(),
            secondary_value: None,
        }],
        active_tab: 0,
        items_tab_enabled: false,
        mobs_tab_enabled: false,
        item_rows: Vec::new(),
        item_column_labels: std::array::from_fn(|index| format!("Column {index}")),
        item_no_value: "-".to_string(),
        item_sort_column: None,
        item_sort_order: 0,
        item_hovered_column: None,
        item_focused_column: None,
        scroll_row: 3,
        scroll_amount: 999,
        max_scroll_amount: 100,
        scroller_height: 5_000,
        total_rows: 12,
        visible_rows: 1,
        row_height: 14,
        tooltip: Some(HudDebugOptionsTooltip {
            text: "Craft\ned".to_string(),
            x: 12,
            y: 34,
        }),
        done: "Fer\ntig".to_string(),
        done_hovered: true,
        done_focused: false,
    })
    .expect("stats screen");
    assert_eq!(screen.title, "Stats");
    assert_eq!(screen.done, "Fertig");
    assert_eq!(screen.loading_text, "Downloading statistics...");
    assert_eq!(screen.general_tab, "General");
    assert_eq!(screen.rows[0].label, "Walkone cm");
    assert_eq!(screen.rows[0].value, "1.00 m");
    assert_eq!(screen.scroll_amount, 100);
    assert_eq!(screen.max_scroll_amount, 100);
    assert_eq!(screen.scroller_height, 4_096);
    assert_eq!(
        screen.tooltip.as_ref().map(|tooltip| tooltip.text.as_str()),
        Some("Crafted")
    );
    assert!(screen.done_hovered);

    assert!(sanitize_hud_stats_screen(HudStatsScreen {
        title: "Stats".to_string(),
        loading_text: "\n\t".to_string(),
        is_loading: true,
        general_tab: "General".to_string(),
        items_tab: "Items".to_string(),
        mobs_tab: "Mobs".to_string(),
        rows: Vec::new(),
        active_tab: 0,
        items_tab_enabled: false,
        mobs_tab_enabled: false,
        item_rows: Vec::new(),
        item_column_labels: std::array::from_fn(|index| format!("Column {index}")),
        item_no_value: "-".to_string(),
        item_sort_column: None,
        item_sort_order: 0,
        item_hovered_column: None,
        item_focused_column: None,
        scroll_row: 0,
        scroll_amount: 0,
        max_scroll_amount: 0,
        scroller_height: 0,
        total_rows: 0,
        visible_rows: 0,
        row_height: 14,
        tooltip: None,
        done: String::new(),
        done_hovered: false,
        done_focused: false,
    })
    .is_none());
}

#[test]
fn sanitize_inventory_keeps_renderable_block_models_and_drops_empty_ones() {
    let quad = crate::item_models::ItemModelQuad {
        corners: [[0.0, 0.0, 0.0]; 4],
        uvs: [[0.0, 0.0]; 4],
        tint: [1.0, 1.0, 1.0, 1.0],
        normal: [0.0, 0.0, 1.0],
        shade: 1.0,
        translucent: false,
    };
    let model = |quads: Vec<crate::item_models::ItemModelQuad>| {
        test_hud_block_item_model_with_quads(
            quads,
            glam::Mat4::IDENTITY,
            GuiItemLightingEntry::Items3d,
            false,
        )
    };

    // A slot whose block model has geometry keeps it; one with no quads drops it (None).
    let kept = sanitize_hud_inventory_slot(HudInventorySlot {
        slot_id: 1,
        x: 0,
        y: 0,
        icon: None,
        block_model: Some(model(vec![quad])),
    });
    assert!(kept.block_model.is_some());

    let dropped = sanitize_hud_inventory_slot(HudInventorySlot {
        slot_id: 2,
        x: 0,
        y: 0,
        icon: None,
        block_model: Some(model(Vec::new())),
    });
    assert!(dropped.block_model.is_none());

    let wrong_lighting = sanitize_hud_inventory_slot(HudInventorySlot {
        slot_id: 3,
        x: 0,
        y: 0,
        icon: None,
        block_model: Some(test_hud_block_item_model_with_quads(
            vec![quad],
            glam::Mat4::IDENTITY,
            GuiItemLightingEntry::ItemsFlat,
            false,
        )),
    });
    assert!(wrong_lighting.block_model.is_none());

    // The same filtering applies to floating (cursor / preview) items.
    let floating = sanitize_hud_inventory_item(HudInventoryItem {
        x: 0,
        y: 0,
        scale: 1.0,
        scale_y: 1.0,
        icon: HudItemIcon::single(HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        }),
        scissor: None,
        draw_decorations: true,
        block_model: Some(model(vec![quad])),
    })
    .unwrap();
    assert!(floating.block_model.is_some());

    let scissored_partial = sanitize_hud_inventory_item(HudInventoryItem {
        x: 0,
        y: 0,
        scale: 1.0,
        scale_y: 1.0,
        icon: HudItemIcon::single(HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        }),
        scissor: Some(HudInventoryItemScissor {
            x: 9,
            y: 18,
            width: 600,
            height: 700,
        }),
        draw_decorations: true,
        block_model: Some(model(vec![quad])),
    })
    .unwrap();
    assert_eq!(
        scissored_partial.scissor,
        Some(HudInventoryItemScissor {
            x: 9,
            y: 18,
            width: 512,
            height: 512,
        })
    );
    assert!(scissored_partial.block_model.is_some());

    let scissored_inside = sanitize_hud_inventory_item(HudInventoryItem {
        x: 9,
        y: 18,
        scale: 1.0,
        scale_y: 1.0,
        icon: HudItemIcon::single(HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        }),
        scissor: Some(HudInventoryItemScissor {
            x: 9,
            y: 18,
            width: 16,
            height: 16,
        }),
        draw_decorations: true,
        block_model: Some(model(vec![quad])),
    })
    .unwrap();
    assert!(scissored_inside.block_model.is_some());
}
