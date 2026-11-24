use super::*;

fn fractional_glyphs() -> HudFontGlyphMap {
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, advance, width) in [
        ('a', 0.5, 1),
        ('b', 6.0, 5),
        ('c', -2.25, 0),
        ('d', -6.0, 0),
    ] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                is_empty: width == 0,
                advance,
                width,
                height: if width == 0 { 0 } else { 8 },
                ..HudAsciiGlyph::default()
            },
        );
    }
    glyphs
}

#[test]
fn fractional_hud_metrics_and_geometry_keep_signed_pen() {
    let glyphs = fractional_glyphs();
    assert_eq!(hud_raw_string_width("aa", &glyphs), 1);
    assert_eq!(hud_font_text_width("c", &glyphs), Some(-2));
    assert_eq!(hud_font_text_width("bd", &glyphs), Some(0));
    let runs = [HudStyledTextRun::plain("bcaa")];
    assert_eq!(hud_font_runs_width(&runs, &glyphs), Some(5));
    let geometry = hud_styled_text_geometry(
        &runs,
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        0,
        (10.0, 20.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    assert_eq!(geometry.glyph_quads.len(), 3);
    assert_eq!(geometry.glyph_quads[0].0.corners[0][0], 10.0);
    assert_eq!(geometry.glyph_quads[1].0.corners[0][0], 13.75);
    assert_eq!(geometry.glyph_quads[2].0.corners[0][0], 14.25);
    let rect = hud_text_backdrop_rect((10.0, 20.0), -6, 1.0).unwrap();
    assert_eq!((rect.x, rect.width), (6.0, 2));
}

#[test]
fn fractional_hud_cursor_stops_at_first_overflow() {
    let glyphs = fractional_glyphs();
    assert_eq!(hud_plain_text_cursor_for_width("aa", 1, &glyphs), Some(2));
    assert_eq!(hud_plain_text_cursor_for_width("bda", 5, &glyphs), Some(0));
    assert_eq!(hud_plain_text_cursor_for_width("bda", 6, &glyphs), Some(3));
    assert_eq!(hud_plain_text_cursor_for_width("ca", -2, &glyphs), Some(1));
    assert_eq!(
        hud_clip_styled_text_runs(&[HudStyledTextRun::plain("bda")], 5, &glyphs),
        Vec::<HudStyledTextRun>::new()
    );
    assert_eq!(hud_text_input_display_start("baaaa", 5, 1, &glyphs), 3);
    assert_eq!(hud_wrap_plain_text("aaaaa", 2, &glyphs), ["aaaa", "a"]);
}

#[test]
fn fractional_digit_width_rounds_once_and_keeps_zero() {
    let mut glyphs = [HudDigitGlyph::default(); 10];
    glyphs[1].advance = 0.5;
    glyphs[2].advance = -1.0;
    assert_eq!(hud_digit_text_width("11", &glyphs), Some(1));
    assert_eq!(hud_digit_text_width("112", &glyphs), Some(0));
    assert_eq!(hud_digit_text_width("2", &glyphs), Some(-1));
    let item = absolute_hud_rect(10.0, 20.0, 16, 16);
    let digit = hud_item_count_digit_quad(item, 1, 0.5, false, glyphs[1]);
    assert_eq!(digit.corners[0][0], 26.5);
}

#[test]
fn fractional_input_geometry_can_return_inside_the_width_limit() {
    let glyphs = fractional_glyphs();
    assert_eq!(hud_plain_text_cursor_for_width("bda", 6, &glyphs), Some(3));
    let runs = [HudStyledTextRun::plain("bda")];
    let geometry = hud_logical_styled_text_geometry(
        &runs,
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        0,
        (10.0, 20.0),
        false,
        HUD_TINT_WHITE,
        Some(6),
        1.0,
    );
    assert_eq!(geometry.glyph_quads.len(), 2);
    assert_eq!(geometry.glyph_quads[0].0.corners[0][0], 10.0);
    assert_eq!(geometry.glyph_quads[1].0.corners[0][0], 10.0);
}

#[test]
fn fractional_extreme_negative_width_uses_java_centering_arithmetic() {
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(
        'x',
        HudAsciiGlyph {
            advance: -2147483648.0,
            ..Default::default()
        },
    );
    let width = hud_raw_string_width("x", &glyphs);
    assert_eq!(width, i32::MIN);
    let screen = PhysicalSize::new(320, 240);
    let expected_x = (320_i32.wrapping_sub(i32::MIN) / 2) as f32;
    assert_eq!(
        hud_selected_item_name_origin(screen, width, false).0,
        expected_x
    );
    assert_eq!(
        hud_experience_level_text_origin(screen, width).0,
        expected_x
    );
}

#[test]
fn missing_hud_glyphs_measure_draw_and_move_cursor_without_registered_characters() {
    let mut glyphs = HudFontGlyphMap::new();
    let missing = HudAsciiGlyph {
        width: 5,
        height: 8,
        advance: 6.0,
        ..Default::default()
    };
    glyphs.set_missing_glyph(missing);
    assert!(glyphs.is_empty());
    assert_eq!(hud_font_text_width("缺字", &glyphs), Some(12));
    assert_eq!(hud_plain_text_cursor_for_width("缺字", 7, &glyphs), Some(1));
    assert_eq!(
        hud_plain_text_display_start_for_width("缺字", 2, 6, &glyphs),
        Some(1)
    );
    let runs = [HudStyledTextRun::plain("缺字")];
    let geometry = hud_styled_text_geometry(
        &runs,
        &glyphs,
        &HudObfuscatedGlyphPool::default(),
        0,
        (10.0, 20.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    assert_eq!(geometry.glyph_quads.len(), 2);
    assert_eq!(geometry.glyph_quads[0].0.uv, missing.uv);
    assert_eq!(geometry.glyph_quads[1].0.corners[0], [16.0, 20.0]);
    glyphs.insert_first_wins(
        '?',
        HudAsciiGlyph {
            advance: 99.0,
            ..Default::default()
        },
    );
    assert_eq!(hud_font_glyph('缺', &glyphs), missing);
}

#[test]
fn item_count_quad_uses_provider_scale_ascent_and_shadow() {
    let glyph = HudDigitGlyph {
        width: 9,
        height: 16,
        render_scale: 0.5,
        bearing_top: 10.0,
        shadow_offset: 0.5,
        advance: 5.5,
        ..Default::default()
    };
    let item = absolute_hud_rect(10.0, 20.0, 16, 16);
    let plain = hud_item_count_digit_quad(item, 6, 0.0, false, glyph);
    assert_eq!(plain.corners[0], [21.0, 26.0]);
    assert_eq!(plain.corners[2], [25.5, 34.0]);
    let shadow = hud_item_count_digit_quad(item, 6, 0.0, true, glyph);
    assert_eq!(shadow.corners[0], [21.5, 26.5]);
}

#[test]
fn item_count_quad_preserves_signed_fractional_bitmap_geometry() {
    for (render_scale, width) in [
        (3.0 / 7.0, f32::from_bits(0x40092493)),
        (-3.0 / 7.0, f32::from_bits(0xc0092493)),
        (0.0, 0.0),
    ] {
        let glyph = HudDigitGlyph {
            width: 5,
            height: 7,
            render_scale,
            bearing_top: -3.0,
            advance: 3.0,
            ..Default::default()
        };
        let item = absolute_hud_rect(0.0, 0.0, 16, 16);
        let quad = hud_item_count_digit_quad(item, 3, 0.0, false, glyph);
        assert_eq!(quad.corners[0], [14.0, 19.0]);
        assert_eq!(
            quad.corners[2],
            [14.0 + width, 19.0 + glyph.rendered_height()]
        );
    }
}

#[test]
fn zero_bitmap_and_empty_hud_glyphs_keep_submission_and_pen_contracts() {
    for render_scale in [1.0, f32::INFINITY] {
        let bitmap = HudAsciiGlyph {
            advance: 3.0,
            render_scale,
            ..Default::default()
        };
        let mut glyphs = HudFontGlyphMap::new();
        glyphs.insert_first_wins('a', bitmap);
        glyphs.insert_first_wins(' ', HudAsciiGlyph::empty(3.0));
        let geometry = hud_styled_text_geometry(
            &[HudStyledTextRun::plain("a a")],
            &glyphs,
            &HudObfuscatedGlyphPool::default(),
            0,
            (0.0, 0.0),
            false,
            HUD_TINT_WHITE,
            None,
            1.0,
        );
        assert_eq!(geometry.glyph_quads.len(), 2);
        assert_eq!(geometry.glyph_quads[1].0.corners[0][0], 6.0);
        if render_scale.is_infinite() {
            assert!(geometry.glyph_quads[0].0.corners[2][0].is_nan());
        } else {
            assert_eq!(geometry.glyph_quads[0].0.corners[2][0], 0.0);
        }
        let mut digits = [HudDigitGlyph::empty(3.0); 10];
        digits[0].width = 5;
        digits[0].height = 8;
        digits[1] = bitmap;
        let submitted = hud_item_count_glyphs("101", &digits).collect::<Vec<_>>();
        assert_eq!(submitted.len(), 2);
        assert_eq!(submitted[1].0, 6.0);
        let quad = hud_item_count_digit_quad(
            absolute_hud_rect(0.0, 0.0, 16, 16),
            9,
            submitted[0].0,
            false,
            submitted[0].1,
        );
        assert_eq!(quad.corners[2][0].is_nan(), render_scale.is_infinite());
    }
}

#[test]
fn component_text_metrics_and_geometry_share_bidi_order_across_fonts_and_styles() {
    let glyph = |advance| HudAsciiGlyph {
        advance,
        width: 1,
        height: 8,
        ..Default::default()
    };
    let mut default = HudFontGlyphMap::new();
    default.insert_first_wins('א', glyph(33_554_432.0));
    default.insert_first_wins('ג', glyph(1.0));
    let mut custom = HudFontGlyphMap::new();
    custom.insert_first_wins('ב', glyph(-33_554_432.0));
    let fonts = HudFontGlyphBook::from_fonts(
        "minecraft:default",
        std::collections::BTreeMap::from([
            ("minecraft:default".into(), default),
            ("test:custom".into(), custom),
        ]),
    );
    let mut runs = vec![
        HudComponentTextRun::plain("א"),
        HudComponentTextRun::plain("ב"),
        HudComponentTextRun::plain("ג"),
    ];
    runs[0].visual.color = Some(0xff0000);
    runs[1].font = Some("test:custom".into());
    runs[1].visual.color = Some(0x00ff00);
    runs[1].visual.style.italic = true;
    runs[2].visual.color = Some(0x0000ff);
    assert_eq!(hud_logical_component_text_width_f32(&runs, &fonts), 1.0);
    assert_eq!(hud_component_text_width(&runs, &fonts), 0);
    let geometry = hud_component_text_geometry(
        &runs,
        &fonts,
        0,
        (0.0, 0.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    assert_eq!(geometry.glyph_quads.len(), 3);
    assert_eq!(geometry.glyph_quads[0].1, [0.0, 0.0, 1.0, 1.0]);
    assert_eq!(geometry.glyph_quads[1].1, [0.0, 1.0, 0.0, 1.0]);
    assert_eq!(geometry.glyph_quads[2].1, [1.0, 0.0, 0.0, 1.0]);
}
