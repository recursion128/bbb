use super::*;

fn chunk(output: &mut Vec<u8>, kind: &[u8; 4], bytes: &[u8]) {
    output.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    output.extend_from_slice(kind);
    output.extend_from_slice(bytes);
    let mut crc = !0_u32;
    for byte in kind.iter().chain(bytes) {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320 & 0_u32.wrapping_sub(crc & 1));
        }
    }
    output.extend_from_slice(&(!crc).to_be_bytes());
}

fn no_pixels_png(width: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = width.to_be_bytes().to_vec();
    ihdr.extend_from_slice(&1_u32.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut bytes, b"IHDR", &ihdr);
    chunk(&mut bytes, b"IEND", &[]);
    bytes
}

fn bad_provider(file: &str) -> String {
    format!(r#"{{"type":"bitmap","file":"font/{file}","ascent":7,"chars":["X"]}}"#)
}

fn roots_with_limit(pack: &Pack, generation: u64, bytes: u64) -> PackRoots {
    let roots = pack.roots();
    let selected = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(
        &roots,
        std::iter::empty::<PathBuf>(),
    )
    .unwrap();
    roots
        .with_client_resource_generation_image_limit(generation, selected, bytes)
        .unwrap()
}

#[test]
fn malformed_custom_png_isolated_transitively_without_leaking_valid_providers() {
    for bytes in [
        b"short".to_vec(),
        no_pixels_png(1)[..20].to_vec(),
        no_pixels_png(0),
        no_pixels_png(1),
    ] {
        let pack = Pack::new();
        pack.write("textures/font/bad.png", bytes);
        pack.font("broken", &format!(r#"[{{"type":"space","advances":{{"K":42}}}},{},{{"type":"space","advances":{{"L":43}}}}]"#, bad_provider("bad.png")));
        pack.font(
            "indirect",
            r#"[{"type":"reference","id":"broken"},{"type":"space","advances":{"M":44}}]"#,
        );
        let atlas = load_hud_font_atlas(&pack.roots())
            .expect("content-invalid custom font must be isolated");
        assert_eq!(atlas.glyphs.get('D').unwrap().advance, 3.0);
        for id in ["minecraft:broken", "minecraft:indirect"] {
            assert!(!atlas.fonts.contains_font(id));
            let map = atlas.fonts.glyphs(Some(id));
            for ch in ['K', 'L', 'M', 'X'] {
                assert!(map.get(ch).is_none(), "{id} leaked {ch}");
                assert_eq!(map.resolve(ch), atlas.glyphs.missing_glyph().unwrap());
            }
        }
    }
}

#[test]
fn default_reference_to_invalid_png_fails_even_when_option_disables_provider() {
    for filter in ["", r#", "filter":{"uniform":true}"#] {
        let pack = Pack::new();
        pack.write("textures/font/bad.png", no_pixels_png(1));
        pack.font("broken", &format!("[{}]", bad_provider("bad.png")));
        pack.font("indirect", r#"[{"type":"reference","id":"broken"}]"#);
        pack.font("default", &format!(r#"[{{"type":"space","advances":{{"D":3}}}},{{"type":"reference","id":"indirect"{filter}}}]"#));
        let error = load_hud_font_atlas(&pack.roots()).unwrap_err();
        assert!(
            format!("{error:#}").starts_with("default font failed to load:"),
            "{error:#}"
        );
    }
}

#[test]
fn invalid_provider_cache_reuses_failed_decode_charge_across_references() {
    let pack = Pack::new();
    pack.write("textures/font/bad.png", no_pixels_png(1));
    pack.font("broken", &format!("[{}]", bad_provider("bad.png")));
    pack.font(
        "first",
        r#"[{"type":"reference","id":"broken"},{"type":"reference","id":"broken"}]"#,
    );
    pack.font("second", r#"[{"type":"reference","id":"first"},{"type":"reference","id":"broken","filter":{"uniform":true}}]"#);
    let roots = roots_with_limit(&pack, 7, 4 + 5 * 8 * 4);
    let atlas = load_hud_font_atlas(&roots)
        .expect("failed 1x1 page is charged once, then shared missing atlas fits");
    assert_eq!(
        roots.resource_image_memory_observation().charged_rgba_bytes,
        164
    );
    assert_eq!(atlas.glyphs.get('D').unwrap().advance, 3.0);
    for id in ["minecraft:broken", "minecraft:first", "minecraft:second"] {
        assert!(!atlas.fonts.contains_font(id));
    }
}

#[test]
fn later_crc_and_file_size_errors_preempt_earlier_png_content_failure() {
    for oversized in [false, true] {
        let pack = Pack::new();
        pack.write("textures/font/bad.png", no_pixels_png(1));
        let mut corrupt = no_pixels_png(1);
        corrupt[29] ^= 1;
        pack.write("textures/font/later.png", corrupt);
        if oversized {
            std::fs::OpenOptions::new()
                .write(true)
                .open(pack.assets.join("textures/font/later.png"))
                .unwrap()
                .set_len(64 * 1024 * 1024 + 1)
                .unwrap();
        }
        pack.font(
            "broken",
            &format!(
                "[{},{}]",
                bad_provider("bad.png"),
                bad_provider("later.png")
            ),
        );
        let error = load_hud_font_atlas(&pack.roots()).unwrap_err();
        assert!(
            format!("{error:#}").contains("later.png"),
            "later hard failure was hidden: {error:#}"
        );
        assert!(
            !error.is::<ResourcePngContentInvalid>(),
            "integrity and size failures cannot be isolated: {error:#}"
        );
    }
}

#[test]
fn later_disabled_png_still_checks_generation_budget_after_content_failure() {
    let pack = Pack::new();
    pack.write("textures/font/bad.png", no_pixels_png(1));
    pack.png();
    pack.font("broken", &format!(r#"[{},{{"type":"bitmap","file":"font/page.png","ascent":7,"chars":["A","B"],"filter":{{"uniform":true}}}}]"#, bad_provider("bad.png")));
    let roots = roots_with_limit(&pack, 9, 4);
    let error = load_hud_font_atlas(&roots).unwrap_err();
    assert!(
        format!("{error:#}").contains("resource image generation"),
        "{error:#}"
    );
    assert_eq!(
        roots.resource_image_memory_observation().charged_rgba_bytes,
        4
    );
}

#[test]
fn reload_valid_png_restores_custom_font_without_mutating_previous_book() {
    let pack = Pack::new();
    pack.write("textures/font/page.png", no_pixels_png(1));
    pack.font(
        "custom",
        r#"[{"type":"bitmap","file":"font/page.png","ascent":7,"chars":["A","B"]}]"#,
    );
    pack.font("indirect", r#"[{"type":"reference","id":"custom"}]"#);
    let before = load_hud_font_atlas(&pack.roots()).unwrap();
    assert!(!before.fonts.contains_font("minecraft:custom"));
    let old_missing = before.fonts.glyphs(Some("minecraft:custom")).resolve('A');
    pack.png();
    pack.write(
        "textures/font/page.png.mcmeta",
        "malformed metadata is irrelevant to font PNGs",
    );
    let after = load_hud_font_atlas(&pack.roots()).unwrap();
    for id in ["minecraft:custom", "minecraft:indirect"] {
        assert!(after.fonts.contains_font(id));
        let glyph = after.fonts.glyphs(Some(id)).get('A').unwrap();
        assert_eq!((glyph.width, glyph.height, glyph.advance), (8, 8, 9.0));
        let x = (glyph.uv.min[0] * after.width as f32).round() as u32;
        let y = (glyph.uv.min[1] * after.height as f32).round() as u32;
        let offset = rgba_offset(after.width, x, y).unwrap();
        assert_eq!(&after.rgba[offset..offset + 4], &[255, 0, 0, 255]);
    }
    assert_eq!(after.glyphs.get('D').unwrap().advance, 3.0);
    assert!(!before.fonts.contains_font("minecraft:custom"));
    assert_eq!(
        before.fonts.glyphs(Some("minecraft:custom")).resolve('A'),
        old_missing
    );
}
