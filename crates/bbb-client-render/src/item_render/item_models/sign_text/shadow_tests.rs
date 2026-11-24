use super::*;
use bbb_render_types::{HudAsciiGlyph, HudFontGlyphMap, HudStyledTextRun, HudTextStyle};

#[test]
fn sign_text_preserves_shadow_override_in_truncation_outline_and_effects() {
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
    let runs = [None, Some(0), Some(0x8000ff00)].map(|shadow_color| WorldTextRun {
        font: None,
        text: "a".into(),
        shadow_color,
        ..Default::default()
    });
    assert_eq!(
        truncate_sign_line_runs_to_width(&runs, 100, &HudFontGlyphBook::from(glyphs.clone())),
        runs
    );
    for explicit in [None, Some(0), Some(0x8000ff00), Some(0x0000ff00)] {
        let run = WorldTextRun {
            font: None,
            text: "a ".into(),
            shadow_color: explicit,
            color: Some(0xff0000),
            style: HudTextStyle {
                bold: true,
                underlined: true,
                ..Default::default()
            },
            event_id: None,
        };
        let surface = bake_sign_text_surface(
            [0.0; 3],
            SignModelAttachment::Standing,
            0.0,
            true,
            &[vec![run], vec![], vec![], vec![]],
            0,
            true,
            [1.0; 2],
            &HudFontGlyphBook::from(glyphs.clone()),
        )
        .unwrap();
        let inverse = surface.submission.transform.inverse();
        let geometry = merge_sign_text_surfaces(
            &[surface],
            &[[0; 3]],
            &HudFontGlyphBook::from(glyphs.clone()),
            0,
            SignTextOutlineContext::default(),
        );
        let resolved = HudStyledTextRun::resolve_shadow_argb(0xffff0000, explicit, false);
        let shadow = resolved != 0;
        let quads = if shadow { 4 } else { 2 };
        assert_eq!(geometry.primary.vertices.len(), quads * 4);
        assert_eq!(geometry.outline.vertices.len(), quads * 4 * 8);
        assert_eq!(geometry.effects.vertices.len(), if shadow { 16 } else { 8 });
        let local = |mesh: &ItemModelMesh, index: usize| {
            inverse.transform_point3(Vec3::from_array(mesh.vertices[index * 4].position))
        };
        let main = if shadow { 2 } else { 0 };
        assert!((local(&geometry.primary, main).z - if shadow { 0.03 } else { 0.0 }).abs() < 1e-5);
        assert!(
            (local(&geometry.outline, main).x - local(&geometry.primary, main).x + 0.5).abs()
                < 1e-5
        );
        assert!(
            (local(&geometry.outline, main).y - local(&geometry.primary, main).y + 0.5).abs()
                < 1e-5
        );
        if shadow {
            for mesh in [&geometry.primary, &geometry.outline, &geometry.effects] {
                assert_eq!(
                    mesh.vertices[0].color,
                    crate::text_mesh::argb_color(resolved)
                );
            }
            assert!((local(&geometry.effects, 0).z - 0.01).abs() < 1e-5);
            assert!((local(&geometry.effects, 1).z - 0.04).abs() < 1e-5);
            assert!(
                (local(&geometry.effects, 0).x - local(&geometry.effects, 1).x - 0.5).abs() < 1e-5
            );
        }
    }
}

#[test]
fn sign_text_obfuscated_metrics_drive_primary_and_outline_pen_and_effects() {
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
    let run = WorldTextRun {
        font: None,
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
    let surface = bake_sign_text_surface(
        [0.0; 3],
        SignModelAttachment::Standing,
        0.0,
        true,
        &[vec![run], vec![], vec![], vec![]],
        0,
        true,
        [1.0; 2],
        &HudFontGlyphBook::from(glyphs.clone()),
    )
    .unwrap();
    let inverse = surface.submission.transform.inverse();
    let mut choices = std::collections::BTreeSet::new();
    for seed in (0..32).map(|seed| seed * 7919) {
        let geometry = merge_sign_text_surfaces(
            std::slice::from_ref(&surface),
            &[[0; 3]],
            &HudFontGlyphBook::from(glyphs.clone()),
            seed,
            SignTextOutlineContext::default(),
        );
        let vertices = &geometry.primary.vertices;
        let local = |mesh: &ItemModelMesh, index: usize| {
            inverse.transform_point3(Vec3::from_array(mesh.vertices[index].position))
        };
        let alternate = vertices[8].uv[0] > 0.5;
        choices.insert(alternate);
        let (advance, offset) = if alternate { (6.75, 1.0) } else { (5.75, 0.5) };
        assert!(
            (local(&geometry.primary, 24).x - local(&geometry.primary, 8).x - advance).abs() < 1e-5
        );
        assert!(
            (local(&geometry.effects, 0).x - local(&geometry.effects, 4).x - offset).abs() < 1e-5
        );
        assert!(
            (local(&geometry.effects, 6).x - local(&geometry.primary, 8).x - advance - 0.1).abs()
                < 1e-5
        );
        let first_alternate = geometry.outline.vertices[8].uv[0] > 0.5;
        let (outline_advance, first_offset) = if first_alternate {
            (6.75, 1.0)
        } else {
            (5.75, 0.5)
        };
        let second_offset = if geometry.outline.vertices[24].uv[0] > 0.5 {
            1.0
        } else {
            0.5
        };
        assert!(
            (local(&geometry.outline, 24).x - local(&geometry.outline, 8).x - outline_advance
                + second_offset
                - first_offset)
                .abs()
                < 1e-5
        );
    }
    assert_eq!(choices.len(), 2);
}
