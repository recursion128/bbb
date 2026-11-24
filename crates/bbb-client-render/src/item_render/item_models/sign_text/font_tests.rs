use super::*;
use bbb_render_types::{HudAsciiGlyph, HudFontGlyphMap, HudTextStyle, HudUvRect};
use std::collections::BTreeMap;

fn fonts() -> HudFontGlyphBook {
    let mut default = HudFontGlyphMap::new();
    let mut custom = HudFontGlyphMap::new();
    let glyph = |advance, u| HudAsciiGlyph {
        advance,
        width: 4,
        height: 8,
        uv: HudUvRect {
            min: [u, 0.0],
            max: [u + 0.05, 1.0],
        },
        ..Default::default()
    };
    default.insert_first_wins('a', glyph(6.25, 0.1));
    default.insert_first_wins('b', glyph(10.5, 0.2));
    default.set_missing_glyph(glyph(3.5, 0.9));
    custom.insert_first_wins('a', glyph(10.5, 0.6));
    custom.insert_first_wins('b', glyph(10.75, 0.7));
    HudFontGlyphBook::from_fonts(
        "minecraft:default",
        BTreeMap::from([
            ("minecraft:default".into(), default),
            ("test:wide".into(), custom),
        ]),
    )
}

fn bake(runs: Vec<WorldTextRun>, fonts: &HudFontGlyphBook) -> SignTextSurface {
    bake_sign_text_surface(
        [0.0; 3],
        SignModelAttachment::Standing,
        0.0,
        true,
        &[runs, vec![], vec![], vec![]],
        0,
        false,
        [0.0, 1.0],
        fonts,
    )
    .unwrap()
}

#[test]
fn sign_custom_fonts_control_truncation_centering_and_atlas_uvs() {
    let fonts = fonts();
    let runs = vec![
        WorldTextRun::plain("a"),
        WorldTextRun {
            text: "aa".into(),
            font: Some("test:wide".into()),
            ..Default::default()
        },
    ];
    let truncated = truncate_sign_line_runs_to_width(&runs, 18, &fonts);
    assert_eq!(truncated.len(), 2);
    assert_eq!(truncated[0].font, None);
    assert_eq!(truncated[1].font.as_deref(), Some("test:wide"));
    assert_eq!(truncated[1].text, "a");
    assert_eq!(sign_line_runs_width(&truncated, &fonts), 17);
    let surface = bake(truncated, &fonts);
    let placed = surface.placed_line(0, &fonts);
    assert_eq!(placed[0].x, -8.0);
    assert_eq!(placed[1].x, -1.75);
    let geometry = merge_sign_text_surfaces(
        &[surface],
        &[[0; 3]],
        &fonts,
        0,
        SignTextOutlineContext::default(),
    );
    assert_eq!(geometry.primary.vertices[0].uv[0], 0.1);
    assert_eq!(geometry.primary.vertices[4].uv[0], 0.6);
}

#[test]
fn sign_missing_font_uses_book_missing_glyph() {
    let fonts = fonts();
    let runs = vec![WorldTextRun {
        text: "aa".into(),
        font: Some("test:absent".into()),
        ..Default::default()
    }];
    assert_eq!(sign_line_runs_width(&runs, &fonts), 7);
    let surface = bake(runs, &fonts);
    let placed = surface.placed_line(0, &fonts);
    assert_eq!(placed[0].x, -3.0);
    assert!(placed.iter().all(|glyph| glyph.glyph.uv.min[0] == 0.9));
}

#[test]
fn sign_obfuscation_selects_only_the_slots_font_pool() {
    let fonts = fonts();
    let surface = bake(
        vec![WorldTextRun {
            text: "aa".into(),
            font: Some("test:wide".into()),
            style: HudTextStyle {
                obfuscated: true,
                ..Default::default()
            },
            ..Default::default()
        }],
        &fonts,
    );
    let mut selections = std::collections::BTreeSet::new();
    for seed in 0..32 {
        let geometry = merge_sign_text_surfaces(
            std::slice::from_ref(&surface),
            &[[0; 3]],
            &fonts,
            seed * 7919,
            SignTextOutlineContext::default(),
        );
        for quad in geometry.primary.vertices.as_chunks::<4>().0 {
            let u = quad[0].uv[0];
            assert!([0.6, 0.7].contains(&u));
            selections.insert(u.to_bits());
        }
    }
    assert_eq!(selections.len(), 2);
}
