use super::*;
use crate::text_mesh::argb_color as argb_to_color;
use bbb_render_types::{HudAsciiGlyph, HudFontGlyphMap, HudStyledTextRun, HudTextStyle};

#[test]
fn entity_name_tag_resolves_each_pass_alpha_and_keeps_nonzero_transparent_shadow_depth() {
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(
        'a',
        HudAsciiGlyph {
            width: 4,
            height: 8,
            advance: 5.0,
            shadow_offset: 0.5,
            bold_offset: 0.5,
            ..Default::default()
        },
    );
    glyphs.insert_first_wins(
        ' ',
        HudAsciiGlyph {
            shadow_offset: 0.5,
            ..HudAsciiGlyph::empty(3.0)
        },
    );
    let fonts = HudFontGlyphBook::from(glyphs);
    for alpha in [0x80, 0xff] {
        for explicit in [None, Some(0), Some(0x8000ff00), Some(0x0000ff00)] {
            let run = WorldTextRun {
                text: "a ".into(),
                shadow_color: explicit,
                color: Some(0xff0000),
                style: HudTextStyle {
                    bold: true,
                    underlined: true,
                    ..Default::default()
                },
                ..Default::default()
            };
            let layer = EntityNameTagLayerSubmission {
                render_type: EntityNameTagRenderType::Normal,
                base_text_argb: alpha << 24 | 0xffffff,
                background_argb: 0x40000000,
                light: [1.0; 2],
            };
            let mesh = layout_layer(
                &world_text_visual_order(std::slice::from_ref(&run), &fonts),
                5,
                0,
                &layer,
                &fonts,
                0,
            )
            .0;
            let resolved =
                HudStyledTextRun::resolve_shadow_argb(alpha << 24 | 0xff0000, explicit, false);
            let shadow = resolved != 0;
            let glyph_quads = if shadow { 4 } else { 2 };
            assert_eq!(
                mesh.vertices.len(),
                (1 + glyph_quads + if shadow { 4 } else { 2 }) * 4
            );
            assert_eq!(mesh.vertices[0].color, argb_to_color(0x40000000));
            let main = if shadow { 3 } else { 1 };
            assert_eq!(
                mesh.vertices[main * 4].position[2],
                if shadow { 0.03 } else { 0.0 }
            );
            assert_eq!(
                mesh.vertices[main * 4].color,
                argb_to_color(alpha << 24 | 0xff0000)
            );
            if shadow {
                assert_eq!(mesh.vertices[4].color, argb_to_color(resolved));
                assert_eq!(mesh.vertices[(1 + glyph_quads) * 4].position[2], 0.01);
                assert_eq!(mesh.vertices[(2 + glyph_quads) * 4].position[2], 0.04);
                assert!(
                    (mesh.vertices[4].position[0] - mesh.vertices[12].position[0] - 0.5).abs()
                        < 1e-6
                );
            }
        }
    }
}

#[test]
fn entity_name_tag_obfuscated_metrics_drive_pen_and_effect_shadow() {
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, advance, offset, u) in [('a', 5.25, 0.5, 0.1), ('b', 5.75, 1.0, 0.6)] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                width: 4,
                height: 8,
                advance,
                bold_offset: offset,
                shadow_offset: offset,
                uv: bbb_render_types::HudUvRect {
                    min: [u, 0.0],
                    max: [u + 0.1, 1.0],
                },
                ..Default::default()
            },
        );
    }
    let fonts = HudFontGlyphBook::from(glyphs);
    let run = WorldTextRun {
        text: "aa".into(),
        shadow_color: Some(0x8000ff00),
        style: HudTextStyle {
            bold: true,
            underlined: true,
            obfuscated: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let layer = EntityNameTagLayerSubmission {
        render_type: EntityNameTagRenderType::Normal,
        base_text_argb: 0xffffffff,
        background_argb: 0,
        light: [1.0; 2],
    };
    let mut choices = std::collections::BTreeSet::new();
    for seed in (0..32).map(|seed| seed * 7919) {
        let mesh = layout_layer(
            &world_text_visual_order(std::slice::from_ref(&run), &fonts),
            12,
            0,
            &layer,
            &fonts,
            seed,
        )
        .0;
        let vertices = &mesh.vertices;
        let alternate = vertices[8].uv[0] > 0.5;
        choices.insert(alternate);
        let (advance, offset) = if alternate { (6.75, 1.0) } else { (5.75, 0.5) };
        assert!((vertices[24].position[0] - vertices[8].position[0] - advance).abs() < 1e-5);
        assert!((vertices[0].position[0] - vertices[8].position[0] - offset).abs() < 1e-5);
        assert!((vertices[32].position[0] - vertices[36].position[0] - offset).abs() < 1e-5);
        assert!((vertices[38].position[0] - vertices[8].position[0] - advance - 0.1).abs() < 1e-5);
    }
    assert_eq!(choices.len(), 2);
}
