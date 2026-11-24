use super::*;
use bbb_render_types::{HudTextStyle, HudUvRect};

fn fonts(entries: &[(char, f32)]) -> HudFontGlyphBook {
    let mut map = HudFontGlyphMap::new();
    for &(ch, advance) in entries {
        map.insert_first_wins(ch, HudAsciiGlyph::empty(advance));
    }
    HudFontGlyphBook::from_fonts("default", BTreeMap::from([("default".into(), map)]))
}

fn bake(runs: &[WorldTextRun], fonts: &HudFontGlyphBook, seed: u64) -> ItemFrameMapTextSurface {
    bake_item_frame_map_text_surface(1, runs, 0, 0, 0, Mat4::IDENTITY, [1.0; 2], 0, fonts, seed)
        .unwrap()
}

fn bounds(surface: &ItemFrameMapTextSurface) -> [f32; 4] {
    let local = surface.submission.transform.inverse() * surface.background.transform;
    let top_left = local.transform_point3(Vec3::ZERO);
    let bottom_right = local.transform_point3(Vec3::new(1.0, 1.0, 0.0));
    [top_left.x, top_left.y, bottom_right.x, bottom_right.y]
}

fn near(actual: [f32; 4], expected: [f32; 4]) {
    for (a, e) in actual.into_iter().zip(expected) {
        assert!((a - e).abs() < 0.0001, "{actual:?} != {expected:?}");
    }
}

#[test]
fn empty_and_invisible_glyphs_accumulate_visual_advance_bounds() {
    let fonts = fonts(&[(' ', 2.5), ('N', -4.5), ('P', 6.25)]);
    for (text, expected) in [
        ("", [-1.0, -1.0, 0.0, 9.0]),
        ("  ", [-1.0, -1.0, 5.0, 9.0]),
        ("N", [-1.0, -1.0, 0.0, 9.0]),
        ("NP", [-5.5, -1.0, 1.75, 9.0]),
        ("PN", [-1.0, -1.0, 6.25, 9.0]),
    ] {
        let surface = bake(&[WorldTextRun::plain(text)], &fonts, 0);
        assert!(surface.glyphs.is_empty());
        assert!(!surface.is_empty());
        assert_eq!(surface.vertex_count(), 4);
        assert_eq!(surface.index_count(), 6);
        if surface.submission.scale == 0.0 {
            assert_eq!(text, "N");
            assert_eq!(surface.submission.width, -4.0);
            assert_eq!(surface.background.transform.x_axis, glam::Vec4::ZERO);
        } else {
            near(bounds(&surface), expected);
        }
        assert!(surface.background.solid);
        assert_eq!(surface.background.tint, [0.0, 0.0, 0.0, 128.0 / 255.0]);
        assert_eq!(surface.background.uv.min, [0.0; 2]);
        assert_eq!(surface.background.uv.max, [0.0; 2]);
    }
    let surface = bake(&[], &fonts, 0);
    near(bounds(&surface), [-1.0, -1.0, 0.0, 9.0]);
}

#[test]
fn background_uses_bold_fractional_advance_and_target_depth() {
    let fonts = fonts(&[(' ', 2.5)]);
    let runs = [WorldTextRun {
        text: "  ".into(),
        style: HudTextStyle {
            bold: true,
            ..Default::default()
        },
        ..Default::default()
    }];
    for (baker, depth) in [
        (
            bake_item_frame_map_text_surface as fn(_, _, _, _, _, _, _, _, _, _) -> _,
            -0.01,
        ),
        (bake_first_person_map_text_surface, -0.01),
        (bake_hud_map_text_surface, -0.01),
    ] {
        let surface = baker(1, &runs, 0, 0, 0, Mat4::IDENTITY, [1.0; 2], 0, &fonts, 0).unwrap();
        let advance = fonts
            .glyphs(None)
            .resolve(' ')
            .styled_advance_f32(runs[0].style);
        near(bounds(&surface), [-1.0, -1.0, advance * 2.0, 9.0]);
        let local = surface.submission.transform.inverse() * surface.background.transform;
        assert!((local.w_axis.z - depth).abs() < 0.0001);
    }
}

#[test]
fn obfuscation_background_tracks_replacement_metrics_not_logical_width() {
    let mut map = HudFontGlyphMap::new();
    for (ch, advance, u) in [('A', 2.1, 0.1), ('B', 2.9, 0.2)] {
        map.insert_first_wins(
            ch,
            HudAsciiGlyph {
                width: 4,
                height: 8,
                advance,
                uv: HudUvRect {
                    min: [u, 0.0],
                    max: [u + 0.05, 1.0],
                },
                ..Default::default()
            },
        );
    }
    let fonts = HudFontGlyphBook::from_fonts("default", BTreeMap::from([("default".into(), map)]));
    let runs = [WorldTextRun {
        text: "AA".into(),
        style: HudTextStyle {
            obfuscated: true,
            bold: true,
            ..Default::default()
        },
        ..Default::default()
    }];
    let mut saw_replacement = false;
    for seed in 0..32 {
        let surface = bake(&runs, &fonts, seed);
        assert_eq!(surface.submission.width, 7.0);
        let advance: f32 = surface
            .glyphs
            .iter()
            .step_by(2)
            .map(|g| {
                if g.uv.min[0] == 0.1 {
                    3.1
                } else {
                    saw_replacement = true;
                    assert_eq!(g.uv.min[0], 0.2);
                    3.9
                }
            })
            .sum();
        near(bounds(&surface), [-1.0, -1.0, advance, 9.0]);
    }
    assert!(saw_replacement);
}

#[test]
fn nan_visual_advance_propagates_into_background_even_after_finite_glyphs() {
    let fonts = fonts(&[('N', f32::NAN), ('P', 2.5)]);
    for text in ["N", "NP", "PN"] {
        let surface = bake(&[WorldTextRun::plain(text)], &fonts, 0);
        assert!(
            surface.background.transform.x_axis.x.is_nan(),
            "{text}: Java Math.max must retain NaN"
        );
    }
}
