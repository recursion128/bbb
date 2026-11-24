use super::*;
use bbb_render_types::HudObfuscatedRandom;

#[test]
fn non_fishy_excludes_fully_shadowed_providers_until_another_codepoint_activates_them() {
    let pack = Pack::new();
    for (second, expected) in [(r#"{"A":4}"#, None), (r#"{"A":4,"B":5}"#, Some(4.0))] {
        pack.font("default", &format!(r#"[{{"type":"space","advances":{{"A":-3}}}},{{"type":"space","advances":{second}}}]"#));
        let atlas = load_hud_font_atlas(&pack.roots()).unwrap();
        assert_eq!(atlas.glyphs.get('A').unwrap().advance, -3.0);
        let filtered = atlas.fonts.non_fishy_glyphs(None).get('A').unwrap();
        assert_eq!(filtered.advance, expected.unwrap_or(6.0));
        if expected.is_none() {
            assert_eq!(filtered, atlas.glyphs.missing_glyph().unwrap());
        }
        assert_eq!(atlas.glyphs.non_fishy().get('A'), Some(filtered));
        assert_eq!(
            atlas
                .fonts
                .obfuscated_pool(None)
                .random_glyph(-3.0, &mut HudObfuscatedRandom::with_seed(1)),
            Some(filtered)
        );
    }
}

#[test]
fn non_fishy_checks_normal_and_bold_advances_without_changing_ordinary_metrics() {
    let pack = Pack::new();
    pack.font("default", r#"[{"type":"space","advances":{"N":-0.25,"Z":0,"A":31,"B":31.5,"C":32,"H":1e38}},{"type":"space","advances":{"N":7,"B":8,"C":9,"H":10,"X":11}}]"#);
    let atlas = load_hud_font_atlas(&pack.roots()).unwrap();
    for (ch, any, filtered) in [
        ('N', -0.25, 7.0),
        ('Z', 0.0, 0.0),
        ('A', 31.0, 31.0),
        ('B', 31.5, 8.0),
        ('C', 32.0, 9.0),
        // R9-06 clamps raw space advances to `MAX_SPACE_ADVANCE` (4096.0); `non_fishy` still rejects it.
        ('H', 4_096.0, 10.0),
    ] {
        assert_eq!(atlas.glyphs.get(ch).unwrap().advance, any, "{ch}");
        assert_eq!(
            atlas.glyphs.non_fishy().get(ch).unwrap().advance,
            filtered,
            "{ch}"
        );
    }
    assert_eq!(atlas.glyphs.get('A').unwrap().bold_offset, 1.0);
}

#[test]
fn non_fishy_uses_raw_bitmap_info_before_missing_bake_and_raw_surrogate_buckets() {
    let pack = Pack::new();
    for (width, height, raw_advance, filtered_advance) in
        [(257, 1, 2057.0, 4.0), (1, 257, 1.0, 6.0)]
    {
        let path = pack.assets.join("textures/font/oversized.png");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::RgbaImage::from_pixel(width, height, image::Rgba([255; 4]))
            .save(path)
            .unwrap();
        pack.font("default", r#"[{"type":"bitmap","file":"font/oversized.png","ascent":7,"chars":["\uD800"]},{"type":"space","advances":{"\uD800":4,"B":5}}]"#);
        let atlas = load_hud_font_atlas(&pack.roots()).unwrap();
        let any = atlas.glyphs.iter().find(|(cp, _)| *cp == 0xd800).unwrap().1;
        let filtered = atlas
            .glyphs
            .non_fishy()
            .iter()
            .find(|(cp, _)| *cp == 0xd800)
            .unwrap()
            .1;
        assert_eq!(any, atlas.glyphs.missing_glyph().unwrap());
        assert_eq!(filtered.advance, filtered_advance);
        assert_eq!(
            atlas
                .fonts
                .obfuscated_pool(None)
                .random_glyph(raw_advance, &mut HudObfuscatedRandom::with_seed(1)),
            Some(filtered)
        );
    }
}

#[test]
fn non_fishy_bitmap_duplicate_slots_keep_the_last_unbaked_advance() {
    let pack = Pack::new();
    let path = pack.assets.join("textures/font/duplicates.png");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    image::RgbaImage::from_fn(64, 8, |x, _| {
        image::Rgba([255, 255, 255, if x <= 32 { 255 } else { 0 }])
    })
    .save(path)
    .unwrap();
    pack.font(
        "default",
        r#"[{"type":"bitmap","file":"font/duplicates.png","ascent":7,"chars":["AA"]}]"#,
    );
    let atlas = load_hud_font_atlas(&pack.roots()).unwrap();
    let glyph = atlas.glyphs.get('A').unwrap();
    assert_eq!(glyph.advance, 2.0);
    assert_eq!(atlas.glyphs.non_fishy().get('A'), Some(glyph));
    assert_eq!(
        atlas
            .fonts
            .obfuscated_pool(None)
            .random_glyph(2.0, &mut HudObfuscatedRandom::with_seed(1)),
        Some(glyph)
    );
}

#[test]
fn non_fishy_unihex_half_pixel_bold_offset_obeys_the_real_integer_advance() {
    use std::io::{Cursor, Write};
    let pack = Pack::new();
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file("font.hex", zip::write::SimpleFileOptions::default())
        .unwrap();
    archive
        .write_all(format!("0041:{}\n0042:{}\n", "80".repeat(16), "80".repeat(16)).as_bytes())
        .unwrap();
    pack.write("font/widths.zip", archive.finish().unwrap().into_inner());
    pack.font("default", r#"[{"type":"unihex","hex_file":"font/widths.zip","size_overrides":[{"from":"@","to":"A","left":0,"right":59},{"from":"B","to":"C","left":0,"right":61}]},{"type":"space","advances":{"A":4,"B":5,"X":6}}]"#);
    let atlas = load_hud_font_atlas(&pack.roots()).unwrap();
    for (ch, advance, bold, filtered) in [('A', 31.0, 31.5, 31.0), ('B', 32.0, 32.5, 5.0)] {
        let glyph = atlas.glyphs.get(ch).unwrap();
        assert_eq!(glyph.advance, advance);
        assert_eq!(glyph.advance + glyph.bold_offset, bold);
        assert_eq!(atlas.glyphs.non_fishy().get(ch).unwrap().advance, filtered);
    }
}

#[test]
fn non_fishy_reference_filters_and_reload_recompute_active_providers() {
    let pack = Pack::new();
    pack.font("leaf", r#"[{"type":"space","advances":{"A":7,"B":8}}]"#);
    pack.font("default", r#"[{"type":"space","advances":{"A":-2}},{"type":"reference","id":"leaf","filter":{"uniform":false}}]"#);
    let before = load_hud_font_atlas(&pack.roots()).unwrap();
    assert_eq!(before.glyphs.non_fishy().get('A').unwrap().advance, 7.0);
    let uniform = load_hud_font_atlas_with_options(
        &pack.roots(),
        providers::FontOptions {
            uniform: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        uniform.glyphs.non_fishy().get('A'),
        uniform.glyphs.missing_glyph()
    );
    pack.font("leaf", r#"[{"type":"space","advances":{"A":7}}]"#);
    let after = load_hud_font_atlas(&pack.roots()).unwrap();
    assert_eq!(
        after.glyphs.non_fishy().get('A'),
        after.glyphs.missing_glyph()
    );
    assert_eq!(before.glyphs.non_fishy().get('A').unwrap().advance, 7.0);
    assert_eq!(
        after
            .fonts
            .non_fishy_glyphs(Some("minecraft:leaf"))
            .get('A')
            .unwrap()
            .advance,
        7.0
    );
    assert_eq!(
        after.fonts.non_fishy_glyphs(Some("unknown:font")).get('A'),
        None
    );
}

#[test]
fn non_fishy_inactive_providers_still_load_and_pay_image_and_selection_budgets() {
    let pack = Pack::new();
    pack.png();
    pack.font("default", r#"[{"type":"space","advances":{"A":-1,"B":-2}},{"type":"bitmap","file":"font/page.png","ascent":7,"chars":["A","B"]}]"#);
    let make_roots = |limit| {
        let roots = pack.roots();
        let selected = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(
            &roots,
            std::iter::empty::<PathBuf>(),
        )
        .unwrap();
        roots
            .with_client_resource_generation_limits(
                1,
                selected,
                bbb_client_assets::MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES,
                limit,
            )
            .unwrap()
    };
    let roots = make_roots(bbb_client_assets::MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES);
    let atlas = load_hud_font_atlas(&roots).unwrap();
    assert_eq!(
        atlas.glyphs.non_fishy().get('A'),
        atlas.glyphs.missing_glyph()
    );
    assert_eq!(
        roots.resource_image_memory_observation().charged_rgba_bytes,
        8 * 16 * 4 + u64::from(atlas.width * atlas.height * 4)
    );
    let data = roots.resource_data_memory_observation();
    let constrained = make_roots(data.charged_logical_bytes - 1);
    let error = load_hud_font_atlas(&constrained).unwrap_err();
    assert!(
        format!("{error:#}").contains("retained font book projections and pools"),
        "{error:#}"
    );
    assert!(constrained.ensure_resource_data_admitted().is_err());
    pack.font("default", r#"[{"type":"space","advances":{"A":-1}},{"type":"bitmap","file":"font/absent.png","ascent":7,"chars":["A"]}]"#);
    let error = load_hud_font_atlas(&pack.roots()).unwrap_err();
    assert!(
        format!("{error:#}").contains("default font failed to load"),
        "{error:#}"
    );
}
