use super::*;
use crate::text_mesh::{argb_color as argb_to_color, world_text_visual_order};
use bbb_render_types::{HudAsciiGlyph, HudFontGlyphMap, HudStyledTextRun, HudTextStyle};

#[test]
fn display_text_shadow_override_preserves_runs_and_controls_geometry() {
    let glyph = HudAsciiGlyph {
        width: 4,
        height: 8,
        advance: 5.0,
        shadow_offset: 0.5,
        bold_offset: 0.5,
        ..Default::default()
    };
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins('a', glyph);
    glyphs.insert_first_wins(
        ' ',
        HudAsciiGlyph {
            shadow_offset: 0.5,
            ..HudAsciiGlyph::empty(3.0)
        },
    );
    let fonts = HudFontGlyphBook::from(glyphs);
    let runs = [None, Some(0), Some(0x8000ff00)].map(|shadow_color| WorldTextRun {
        text: "a".into(),
        shadow_color,
        ..Default::default()
    });
    assert_eq!(
        world_text_visual_order(&runs, &fonts)
            .iter()
            .map(|glyph| glyph.1.clone())
            .collect::<Vec<_>>(),
        runs
    );
    assert_eq!(split_display_text_lines(&runs, 100, &fonts)[0].runs, runs);
    for explicit in [None, Some(0), Some(0x8000ff00), Some(0x0000ff00)] {
        for flags in [
            0,
            DISPLAY_TEXT_FLAG_SHADOW,
            DISPLAY_TEXT_FLAG_SEE_THROUGH,
            DISPLAY_TEXT_FLAG_SHADOW | DISPLAY_TEXT_FLAG_SEE_THROUGH,
        ] {
            let run = WorldTextRun {
                text: "a ".into(),
                color: Some(0xff0000),
                shadow_color: explicit,
                style: HudTextStyle {
                    bold: true,
                    underlined: true,
                    ..Default::default()
                },
                ..Default::default()
            };
            let surface = bake_display_text_surface(
                1,
                0,
                None,
                Mat4::IDENTITY,
                &[run],
                100,
                0,
                128_u8 as i8,
                flags as i8,
                [1.0; 2],
                &fonts,
                0,
            )
            .unwrap();
            let resolved = HudStyledTextRun::resolve_shadow_argb(
                0x80ff0000,
                explicit,
                flags & DISPLAY_TEXT_FLAG_SHADOW != 0,
            );
            let shadow = resolved != 0;
            let glyph_quads = if shadow { 4 } else { 2 };
            assert_eq!(
                surface.text_mesh().vertices.len(),
                (glyph_quads + if shadow { 4 } else { 2 }) * 4
            );
            let inverse = surface.submission.transform.inverse();
            let local = |quad: usize| {
                inverse.transform_point3(Vec3::from_array(
                    surface.text_mesh().vertices[quad * 4].position,
                ))
            };
            let base = if shadow { 2 } else { 0 };
            assert!((local(base).z - if shadow { 0.03 } else { 0.0 }).abs() < 1e-5);
            assert!((local(base + 1).z - local(base).z - 0.001).abs() < 1e-5);
            assert!((local(base + 1).x - local(base).x - 0.5).abs() < 1e-5);
            if shadow {
                assert_eq!(
                    surface.text_mesh().vertices[0].color,
                    argb_to_color(resolved)
                );
                assert!((local(0).x - local(2).x - 0.5).abs() < 1e-5);
                assert!((local(glyph_quads).z - 0.01).abs() < 1e-5);
                assert!((local(glyph_quads + 1).z - 0.04).abs() < 1e-5);
                assert!((local(glyph_quads).x - local(glyph_quads + 1).x - 0.5).abs() < 1e-5);
            }
        }
    }
}

#[test]
fn display_text_obfuscated_glyph_owns_advance_effects_and_offsets() {
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
    let mut choices = std::collections::BTreeSet::new();
    for seed in (0..32).map(|seed| seed * 7919) {
        let surface = bake_display_text_surface(
            1,
            0,
            None,
            Mat4::IDENTITY,
            std::slice::from_ref(&run),
            100,
            0,
            -1,
            0,
            [1.0; 2],
            &fonts,
            seed,
        )
        .unwrap();
        let vertices = &surface.text_mesh().vertices;
        let inverse = surface.submission.transform.inverse();
        let local =
            |index: usize| inverse.transform_point3(Vec3::from_array(vertices[index].position));
        let alternate = vertices[8].uv[0] > 0.5;
        choices.insert(alternate);
        let (advance, offset) = if alternate { (6.75, 1.0) } else { (5.75, 0.5) };
        assert!((local(24).x - local(8).x - advance).abs() < 1e-5);
        assert!((local(0).x - local(8).x - offset).abs() < 1e-5);
        assert!((local(32).x - local(36).x - offset).abs() < 1e-5);
        assert!((local(38).x - local(8).x - advance - 0.1).abs() < 1e-5);
    }
    assert_eq!(choices.len(), 2);
}
