use super::*;

/// Where an atlas-sized band sits in the shared image, so a test can read the
/// coverage the provider baked.
fn coverage_at(atlas: &HudFontAtlasImage, glyph: &HudDigitGlyph, x: u32, y: u32) -> u8 {
    let (atlas_w, atlas_h) = atlas.atlas_dimensions();
    let origin_x = (glyph.uv.min[0] * atlas_w as f32).round() as u32;
    let origin_y = (glyph.uv.min[1] * atlas_h as f32).round() as u32;
    atlas.pixel_rgba(origin_x + x, origin_y + y)[0]
}

#[test]
fn runtime_truetype_provider_loads_metrics_and_coverage_through_the_pack() {
    let pack = Pack::new();
    pack.write(
        "font/fixture.ttf",
        super::super::ttf::fixture::fixture_ttf(),
    );
    pack.font(
        "default",
        r#"[{"type":"ttf","file":"minecraft:fixture.ttf","size":10}]"#,
    );
    let atlas = load_hud_font_atlas(&pack.roots()).unwrap();

    // The `font/` prefix resolves `minecraft:fixture.ttf` to
    // `assets/minecraft/font/fixture.ttf`.
    let a = atlas.glyphs.get('A').expect("A should come from the TTF");
    assert_eq!(a.advance, 6.0);
    assert_eq!((a.bearing_left, a.bearing_top), (1.0, 7.0));
    assert_eq!((a.width, a.height), (8, 7));
    assert_eq!(a.render_scale, 1.0);
    assert_eq!(a.baseline_offset(), 0.0);
    assert!(!a.is_empty);

    let c = atlas.glyphs.get('C').unwrap();
    assert_eq!((c.bearing_left, c.bearing_top), (-1.0, 7.0));
    assert_eq!((c.width, c.height), (6, 7));

    // U+0020 has an advance but no bitmap: vanilla's `EmptyGlyph`.
    let space = atlas.glyphs.get(' ').unwrap();
    assert!(space.is_empty);
    assert_eq!(space.advance, 3.0);
    assert!(space
        .styled_quads(0.0, 0.0, Default::default(), false)
        .is_empty());

    // The coverage bitmap reaches the shared atlas image.
    assert!(coverage_at(&atlas, &a, 3, 0) > 0);
    assert_eq!(coverage_at(&atlas, &a, 7, 0), 0);
    assert!(coverage_at(&atlas, &a, 0, 6) > 0);

    // `B` spans x 0.5..5.5 (6px wide, left edges 50% covered) and y 0..6 (full height).
    let b = atlas.glyphs.resolve('B');
    assert_eq!((b.advance, b.bearing_left, b.bearing_top), (6.0, 0.0, 6.0));
    assert_eq!((b.width, b.height), (6, 6));
    for y in 0..6 {
        assert_eq!(coverage_at(&atlas, &b, 0, y), 128, "(0, {y})");
        for x in 1..5 {
            assert_eq!(coverage_at(&atlas, &b, x, y), 255, "({x}, {y})");
        }
        assert_eq!(coverage_at(&atlas, &b, 5, y), 128, "(5, {y})");
    }
    let quads = b.styled_quads(10.0, 20.0, Default::default(), false);
    assert_eq!(
        quads[0].corners,
        [[10.0, 21.0], [10.0, 27.0], [16.0, 27.0], [16.0, 21.0]]
    );
}

#[test]
fn runtime_truetype_missing_codepoint_answers_vanillas_miss() {
    let pack = Pack::new();
    pack.write(
        "font/fixture.ttf",
        super::super::ttf::fixture::fixture_ttf(),
    );
    pack.font(
        "default",
        r#"[{"type":"ttf","file":"minecraft:fixture.ttf","size":10}]"#,
    );
    let atlas = load_hud_font_atlas(&pack.roots()).unwrap();
    let missing = atlas.glyphs.missing_glyph().unwrap();

    // `TrueTypeGlyphProvider.getGlyph` returns null for a codepoint outside
    // the charmap; it never borrows another provider's glyph.
    for absent in ['Z', '中', '\u{10330}'] {
        assert_eq!(atlas.glyphs.resolve(absent), missing, "{absent}");
    }
    assert_eq!(atlas.glyphs.resolve('A').advance, 6.0);
    assert_eq!(atlas.glyphs.resolve('A').width, 8);
}

#[test]
fn runtime_truetype_skip_removes_codepoints_before_selection() {
    let pack = Pack::new();
    pack.write(
        "font/fixture.ttf",
        super::super::ttf::fixture::fixture_ttf(),
    );
    pack.font(
        "default",
        r#"[{"type":"ttf","file":"minecraft:fixture.ttf","size":10,"skip":"AC"}]"#,
    );
    let atlas = load_hud_font_atlas(&pack.roots()).unwrap();
    let missing = atlas.glyphs.missing_glyph().unwrap();
    assert_eq!(atlas.glyphs.resolve('A'), missing);
    assert_eq!(atlas.glyphs.resolve('C'), missing);
    // `skip` is a filter, not a font setting: B and the space survive.
    assert_eq!(atlas.glyphs.resolve('B').advance, 6.0);
    assert_eq!(atlas.glyphs.resolve('B').width, 6);
    assert!(atlas.glyphs.resolve(' ').is_empty);

    // Skipping every glyf-backed codepoint leaves a provider with no raster
    // page at all; its remaining `EmptyGlyph` still carries its advance.
    pack.font(
        "default",
        r#"[{"type":"ttf","file":"minecraft:fixture.ttf","size":10,"skip":"ABC"}]"#,
    );
    let atlas = load_hud_font_atlas(&pack.roots()).unwrap();
    assert_eq!((atlas.width, atlas.height), (5, 8));
    let space = atlas.glyphs.resolve(' ');
    assert!(space.is_empty);
    assert_eq!(space.advance, 3.0);
    assert_eq!(
        atlas.glyphs.resolve('B'),
        atlas.glyphs.missing_glyph().unwrap()
    );
}

#[test]
fn runtime_truetype_precedence_follows_provider_order_and_first_wins() {
    let pack = Pack::new();
    pack.write(
        "font/fixture.ttf",
        super::super::ttf::fixture::fixture_ttf(),
    );
    pack.png();
    // The bitmap page covers `B` (and `A` via the shared space provider).
    let bitmap = r#"{"type":"bitmap","file":"font/page.png","height":8,"ascent":7,"chars":["B"]}"#;
    let ttf = r#"{"type":"ttf","file":"minecraft:fixture.ttf","size":10}"#;

    pack.font("default", &format!(r#"[{bitmap},{ttf}]"#));
    let atlas = load_hud_font_atlas(&pack.roots()).unwrap();
    // FontSet.computeGlyphInfo takes the first provider supplying a codepoint.
    let b = atlas.glyphs.resolve('B');
    assert_eq!(b.width, 8, "the bitmap page wins the shared codepoint");
    assert_eq!(b.bearing_top, 7.0);
    // Codepoints only the TTF supplies still come from it.
    let a = atlas.glyphs.resolve('A');
    assert_eq!((a.advance, a.width), (6.0, 8));
    assert_eq!(a.bearing_left, 1.0);

    pack.font("default", &format!(r#"[{ttf},{bitmap}]"#));
    let atlas = load_hud_font_atlas(&pack.roots()).unwrap();
    let b = atlas.glyphs.resolve('B');
    assert_eq!(b.width, 6, "the ttf provider wins when it is listed first");
    assert_eq!(b.bearing_left, 0.0);
}

#[test]
fn runtime_truetype_reload_releases_the_previous_provider_page() {
    let pack = Pack::new();
    pack.write(
        "font/fixture.ttf",
        super::super::ttf::fixture::fixture_ttf(),
    );
    let ttf = r#"{"type":"ttf","file":"minecraft:fixture.ttf","size":10}"#;
    pack.font("default", &format!(r#"[{ttf}]"#));
    let make_roots = |generation: u64| {
        let roots = pack.roots();
        let selected = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(
            &roots,
            std::iter::empty::<PathBuf>(),
        )
        .unwrap();
        roots
            .with_client_resource_generation(generation, selected)
            .unwrap()
    };

    let roots = make_roots(1);
    let atlas = load_hud_font_atlas(&roots).unwrap();
    let retained = roots.resource_data_memory_observation();
    assert!(roots.resource_image_memory_observation().charged_rgba_bytes > 0);

    // A reload that drops the provider releases its catalog and page bytes
    // with the generation that retained them.
    pack.font("default", r#"[{"type":"space","advances":{"A":6}}]"#);
    let roots = make_roots(2);
    let reloaded = load_hud_font_atlas(&roots).unwrap();
    // Only the shared 5x8 missing page is left: the 1024-wide truetype page
    // is gone and nothing but the atlas itself is charged.
    assert_eq!((reloaded.width, reloaded.height), (5, 8));
    assert!(reloaded.glyphs.resolve('A').is_empty);
    assert_eq!(
        roots.resource_image_memory_observation().charged_rgba_bytes,
        u64::from(reloaded.width) * u64::from(reloaded.height) * 4
    );
    assert!(
        roots
            .resource_data_memory_observation()
            .derived_logical_bytes
            < retained.derived_logical_bytes,
        "the truetype glyph catalog is released on reload"
    );
    // The previous generation's atlas is still intact and still owns its page.
    let a = atlas.glyphs.get('A').unwrap();
    assert_eq!((a.advance, a.width), (6.0, 8));
    assert!(coverage_at(&atlas, &a, 0, 6) > 0);
}

#[test]
fn runtime_truetype_glyph_draws_with_its_own_origin_and_advance() {
    let pack = Pack::new();
    pack.write(
        "font/fixture.ttf",
        super::super::ttf::fixture::fixture_ttf(),
    );
    pack.font(
        "default",
        r#"[{"type":"ttf","file":"minecraft:fixture.ttf","size":10}]"#,
    );
    let atlas = load_hud_font_atlas(&pack.roots()).unwrap();
    let a = atlas.glyphs.resolve('A');

    // BakedSheetGlyph renders `[x + getLeft(), x + getRight()]` by
    // `[y + getTop(), y + getBottom()]`; a 8x7 cell at bearing (1, 7) with
    // advance 6.
    let quads = a.styled_quads(10.0, 20.0, Default::default(), false);
    assert_eq!(quads.len(), 1);
    let corners = quads[0].corners;
    assert_eq!(corners[0], [11.0, 20.0]);
    assert_eq!(corners[1], [11.0, 27.0]);
    assert_eq!(corners[2], [19.0, 27.0]);
    assert_eq!(corners[3], [19.0, 20.0]);

    let area = a.styled_glyph_area(10.0, 20.0, Default::default(), false);
    assert_eq!((area.left, area.top, area.right), (11.0, 20.0, 16.0));
    assert_eq!((area.bounds_right, area.bottom), (19.0, 27.0));
    assert!(!area.empty);

    // The shared measurement path advances by the glyph's own advance.
    assert_eq!(a.styled_advance_f32(Default::default()), 6.0);
    assert_eq!(
        a.styled_advance_f32(bbb_render_types::HudTextStyle {
            bold: true,
            ..Default::default()
        }),
        7.0
    );
}

/// Local acceptance against a real system font. The committed fixtures build
/// their own face, so this only runs when `BBB_TRUETYPE_FONT` names a TTF
/// (for example `/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf`).
#[test]
fn local_system_truetype_font_acceptance() {
    let Some(path) = std::env::var_os("BBB_TRUETYPE_FONT") else {
        return;
    };
    let bytes = std::fs::read(&path).expect("BBB_TRUETYPE_FONT is readable");
    let pack = Pack::new();
    pack.write("font/system.ttf", &bytes);
    let load = |size: f32, oversample: f32| {
        pack.font(
            "default",
            &format!(
                r#"[{{"type":"ttf","file":"minecraft:system.ttf","size":{size},"oversample":{oversample}}}]"#
            ),
        );
        load_hud_font_atlas(&pack.roots()).unwrap()
    };

    let atlas = load(11.0, 1.0);
    let mut latin = 0;
    for codepoint in 'A'..='Z' {
        let glyph = atlas.glyphs.resolve(codepoint);
        assert!(!glyph.is_empty, "{codepoint}");
        assert!(glyph.advance > 0.0, "{codepoint}");
        assert!(glyph.width > 0 && glyph.height > 0, "{codepoint}");
        assert!(
            coverage_at(&atlas, &glyph, 0, 0) > 0 || glyph.width > 1,
            "{codepoint}"
        );
        latin += 1;
    }
    assert_eq!(latin, 26);
    // A private use codepoint is not in the charmap: Vanilla's miss, not a
    // borrowed glyph.
    assert_eq!(
        atlas.glyphs.resolve('\u{e000}'),
        atlas.glyphs.missing_glyph().unwrap()
    );

    // `pixelsPerEm = round(size * oversample)`: rendering at twice the
    // oversample doubles the raster cell and halves it back through
    // `render_scale`, so the rendered size, bearings and advance land on the
    // same pixel grid up to the outward control-box rounding.
    let doubled = load(11.0, 2.0);
    let plain_a = atlas.glyphs.resolve('A');
    let doubled_a = doubled.glyphs.resolve('A');
    assert_eq!(doubled_a.render_scale, 0.5);
    assert!(doubled_a.width > plain_a.width);
    assert!((doubled_a.rendered_width() - plain_a.rendered_width()).abs() <= 1.0);
    assert!((doubled_a.rendered_height() - plain_a.rendered_height()).abs() <= 1.0);
    assert!((doubled_a.advance - plain_a.advance).abs() <= 1.0);
    assert!((doubled_a.bearing_top - plain_a.bearing_top).abs() <= 1.0);
    eprintln!(
        "system truetype acceptance: A advance {} cell {}x{} oversample {}",
        plain_a.advance, plain_a.width, plain_a.height, doubled_a.render_scale
    );
}
