use super::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Pack {
    root: PathBuf,
    assets: PathBuf,
}

impl Pack {
    fn new() -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "bbb-font-runtime-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let assets = root.join("sources/26.1/assets/minecraft");
        let pack = Self { root, assets };
        pack.font("default", r#"[{"type":"space","advances":{"D":3}}]"#);
        pack
    }
    fn write(&self, path: &str, bytes: impl AsRef<[u8]>) {
        let path = self.assets.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    fn font(&self, name: &str, providers: &str) {
        self.write(
            &format!("font/{name}.json"),
            format!(r#"{{"providers":{providers}}}"#),
        );
    }
    fn png(&self) {
        let path = self.assets.join("textures/font/page.png");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::RgbaImage::from_fn(8, 16, |_, y| {
            if y < 8 {
                image::Rgba([255, 0, 0, 255])
            } else {
                image::Rgba([0, 255, 0, 255])
            }
        })
        .save(path)
        .unwrap();
    }
    fn roots(&self) -> PackRoots {
        PackRoots::from_paths(&self.root, self.root.join("sources/26.1"), &self.assets)
    }
}
impl Drop for Pack {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn runtime_font_failure_is_transitive_and_custom_fonts_use_missing_map() {
    let pack = Pack::new();
    pack.font("broken", r#"[{"type":"bitmap","file":"font/absent.png","ascent":7,"chars":["X"]},{"type":"space","advances":{"K":42}}]"#);
    pack.font(
        "indirect",
        r#"[{"type":"reference","id":"broken"},{"type":"space","advances":{"L":12}}]"#,
    );
    pack.font("unresolved", r#"[{"type":"reference","id":"absent"}]"#);
    pack.font("cycle_a", r#"[{"type":"reference","id":"cycle_b"}]"#);
    pack.font("cycle_b", r#"[{"type":"reference","id":"cycle_a"}]"#);
    let atlas = load_hud_font_atlas(&pack.roots()).unwrap();
    assert_eq!(atlas.glyphs.resolve('D').advance, 3.0);
    for name in ["broken", "indirect", "unresolved", "cycle_a", "cycle_b"] {
        let id = format!("minecraft:{name}");
        assert!(!atlas.fonts.contains_font(&id));
        assert_eq!(
            atlas.fonts.glyphs(Some(&id)).resolve('D'),
            atlas.glyphs.missing_glyph().unwrap()
        );
    }
    pack.font(
        "default",
        r#"[{"type":"reference","id":"indirect","filter":{"uniform":true}}]"#,
    );
    let error = load_hud_font_atlas(&pack.roots()).unwrap_err();
    assert!(format!("{error:#}").contains("default font failed to load"));
}

#[test]
fn runtime_disabled_providers_are_loaded_once_and_invalid_ones_reject_the_font() {
    let pack = Pack::new();
    pack.png();
    let bitmap = r#"{"type":"bitmap","file":"font/page.png","ascent":7,"chars":["A","B"],"filter":{"uniform":true}}"#;
    pack.font("default", &format!("[{bitmap}]"));
    pack.font("shared", &format!("[{bitmap}]"));
    let roots = pack.roots();
    let atlas = load_hud_font_atlas(&roots).unwrap();
    assert!(atlas.glyphs.is_empty());
    assert!(atlas.fonts.contains_font("minecraft:shared"));
    assert_eq!(
        roots.resource_image_memory_observation().charged_rgba_bytes,
        8 * 16 * 4 + 5 * 8 * 4
    );
    pack.font("broken", r#"[{"type":"bitmap","file":"font/absent.png","ascent":7,"chars":["X"],"filter":{"uniform":true}}]"#);
    let atlas = load_hud_font_atlas(&pack.roots()).unwrap();
    assert!(!atlas.fonts.contains_font("minecraft:broken"));
}

#[test]
fn runtime_missing_reference_cannot_hide_later_disabled_image_budget_failure() {
    let pack = Pack::new();
    pack.png();
    let path = pack.assets.join("textures/font/page.png");
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[16..20].copy_from_slice(&8193_u32.to_be_bytes());
    std::fs::write(path, bytes).unwrap();
    pack.font("broken", r#"[{"type":"reference","id":"absent"},{"type":"bitmap","file":"font/page.png","ascent":7,"chars":["X"],"filter":{"uniform":true}}]"#);
    let error = load_hud_font_atlas(&pack.roots()).unwrap_err();
    assert!(
        format!("{error:#}").contains("dimension limit"),
        "{error:#}"
    );
}

#[test]
fn runtime_font_bitmap_reads_full_png_and_ignores_sprite_metadata() {
    let pack = Pack::new();
    pack.png();
    for metadata in [
        r#"{"animation":{"width":8,"height":8,"frames":[0]}}"#,
        "bad JSON",
    ] {
        pack.write("textures/font/page.png.mcmeta", metadata);
        pack.font(
            "default",
            r#"[{"type":"bitmap","file":"font/page.png","ascent":7,"chars":["A","B"]}]"#,
        );
        let roots = pack.roots();
        let atlas = load_hud_font_atlas(&roots).unwrap();
        for (ch, color) in [('A', [255, 0, 0, 255]), ('B', [0, 255, 0, 255])] {
            let glyph = atlas.glyphs.get(ch).unwrap();
            assert_eq!((glyph.width, glyph.height, glyph.advance), (8, 8, 9.0));
            let x = (glyph.uv.min[0] * atlas.width as f32).round() as u32;
            let y = (glyph.uv.min[1] * atlas.height as f32).round() as u32;
            let offset = rgba_offset(atlas.width, x, y).unwrap();
            assert_eq!(atlas.rgba[offset..offset + 4], color);
        }
        assert_eq!(
            roots.resource_image_memory_observation().charged_rgba_bytes,
            8 * 16 * 4 + u64::from(atlas.width * atlas.height * 4)
        );
    }
}

#[test]
fn runtime_unihex_content_failure_discards_custom_font_and_checks_later_resources() {
    let pack = Pack::new();
    pack.write("font/bad.zip", b"not a ZIP archive");
    pack.font(
        "bad_hex",
        r#"[{"type":"unihex","hex_file":"font/bad.zip","filter":{"uniform":true}}]"#,
    );
    let atlas = load_hud_font_atlas(&pack.roots()).unwrap();
    assert!(!atlas.fonts.contains_font("minecraft:bad_hex"));
    assert_eq!(atlas.glyphs.resolve('D').advance, 3.0);
    pack.png();
    pack.font("bad_hex", r#"[{"type":"unihex","hex_file":"font/bad.zip"},{"type":"bitmap","file":"font/page.png","ascent":7,"chars":["A","B"],"filter":{"uniform":true}}]"#);
    let roots = pack.roots();
    let selected = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(
        &roots,
        std::iter::empty::<PathBuf>(),
    )
    .unwrap();
    let roots = roots
        .with_client_resource_generation_image_limit(1, selected, 1)
        .unwrap();
    let error = load_hud_font_atlas(&roots).unwrap_err();
    assert!(
        format!("{error:#}").contains("resource image generation"),
        "{error:#}"
    );
}

#[test]
fn runtime_image_crc_failure_stays_fatal_after_a_soft_reference_failure() {
    let pack = Pack::new();
    pack.png();
    let path = pack.assets.join("textures/font/page.png");
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[29] ^= 1;
    std::fs::write(path, bytes).unwrap();
    pack.font("broken", r#"[{"type":"reference","id":"absent"},{"type":"bitmap","file":"font/page.png","ascent":7,"chars":["X"],"filter":{"uniform":true}}]"#);
    let error = load_hud_font_atlas(&pack.roots()).unwrap_err();
    assert!(!error.is::<ResourcePngContentInvalid>(), "{error:#}");
    let diagnostic = format!("{error:#}");
    assert!(
        diagnostic.contains("textures/font/page.png"),
        "{diagnostic}"
    );
    assert!(diagnostic.contains("invalid CRC"), "{diagnostic}");
}

#[test]
fn runtime_disabled_unihex_pages_share_cache_and_pay_cumulative_derived_budget() {
    use std::io::{Cursor, Write};
    let pack = Pack::new();
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("font.hex", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(format!("D800:{}\n", "80".repeat(16)).as_bytes())
        .unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    for name in ["one", "two"] {
        pack.write(&format!("font/{name}.zip"), &bytes);
    }
    let providers = r#"[{"type":"unihex","hex_file":"font/one.zip","filter":{"uniform":true}}]"#;
    pack.font("default", providers);
    pack.font("shared", providers);
    let make_roots = |data_limit: u64| {
        let roots = pack.roots();
        let selected = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(
            &roots,
            std::iter::empty::<PathBuf>(),
        )
        .unwrap();
        roots
            .with_client_resource_generation_limits(1, selected, 5 * 8 * 4, data_limit)
            .unwrap()
    };
    let roots = make_roots(bbb_client_assets::MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES);
    let atlas = load_hud_font_atlas(&roots).unwrap();
    assert!(atlas.glyphs.is_empty());
    // Disabled lazy unihex providers do not allocate an eager 8192px RGBA raster page.
    assert_eq!(
        roots.resource_image_memory_observation().charged_rgba_bytes,
        5 * 8 * 4
    );
    let one_provider_data = roots
        .resource_data_memory_observation()
        .charged_logical_bytes;
    // Two fonts referencing the same disabled unihex provider share the loaded catalog;
    // adding a second distinct disabled unihex provider charges its catalog against the budget.
    pack.font(
        "second",
        r#"[{"type":"unihex","hex_file":"font/two.zip","filter":{"uniform":true}}]"#,
    );
    let roots = make_roots(one_provider_data);
    let error = load_hud_font_atlas(&roots).unwrap_err();
    assert!(
        format!("{error:#}").contains("retained unihex glyph catalog")
            || format!("{error:#}").contains("resource data generation"),
        "{error:#}"
    );
}

#[test]
fn runtime_shared_missing_atlas_checks_budget_before_allocation() {
    let pack = Pack::new();
    let roots = pack.roots();
    let selected = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(
        &roots,
        std::iter::empty::<PathBuf>(),
    )
    .unwrap();
    let roots = roots
        .with_client_resource_generation_image_limit(1, selected, 159)
        .unwrap();
    let error = load_hud_font_atlas(&roots).unwrap_err();
    assert!(
        format!("{error:#}").contains("shared font atlas"),
        "{error:#}"
    );
    assert_eq!(
        roots.resource_image_memory_observation().charged_rgba_bytes,
        0
    );
}

#[test]
fn runtime_bitmap_accepts_signed_height_and_extreme_ascent() {
    let pack = Pack::new();
    pack.png();
    for (height, ascent, advance) in [(0, 0, 1.0), (-4, -4, 0.0), (8, i32::MIN, 5.0)] {
        pack.font("default", &format!(r#"[{{"type":"bitmap","file":"font/page.png","height":{height},"ascent":{ascent},"chars":["A"]}}]"#));
        let atlas = load_hud_font_atlas(&pack.roots()).unwrap();
        let glyph = atlas.glyphs.get('A').unwrap();
        assert_eq!((glyph.width, glyph.height), (8, 16));
        assert_eq!(glyph.advance, advance);
        assert_eq!(glyph.baseline_offset(), 7.0 - ascent as f32);
        assert_eq!(glyph.rendered_height(), height as f32);
    }
}

#[test]
fn runtime_space_and_zero_bitmap_have_distinct_submission_kinds() {
    let pack = Pack::new();
    let path = pack.assets.join("textures/font/page.png");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    image::RgbaImage::from_pixel(1, 1, image::Rgba([255; 4]))
        .save(path)
        .unwrap();
    for chars in [r#"["AB"]"#, r#"["A","B"]"#] {
        pack.font("default", &format!(r#"[{{"type":"space","advances":{{"X":1}}}},{{"type":"bitmap","file":"font/page.png","ascent":7,"chars":{chars}}}]"#));
        let atlas = load_hud_font_atlas(&pack.roots()).unwrap();
        let bitmap = atlas.glyphs.get('A').unwrap();
        let empty = atlas.glyphs.get('X').unwrap();
        assert!(!bitmap.is_empty);
        assert!(empty.is_empty);
        assert_eq!(bitmap.advance, empty.advance);
        assert_eq!(
            bitmap
                .styled_quads(0.0, 0.0, Default::default(), false)
                .len(),
            1
        );
        assert!(empty
            .styled_quads(0.0, 0.0, Default::default(), false)
            .is_empty());
    }
}

#[test]
fn runtime_surrogate_atlas_preserves_provider_order_and_obfuscation_across_reload() {
    use std::io::{Cursor, Write};
    let pack = Pack::new();
    let write_hex = |name: &str, row: &str| {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file("font.hex", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(format!("D800:{}\n", row.repeat(16)).as_bytes())
            .unwrap();
        pack.write(
            &format!("font/{name}.zip"),
            zip.finish().unwrap().into_inner(),
        );
    };
    write_hex("one", "FFC0");
    write_hex("two", "FFFF");
    pack.font("default", r#"[{"type":"space","advances":{"A":6,"�":9,"𐀀":10}},{"type":"unihex","hex_file":"font/one.zip"},{"type":"unihex","hex_file":"font/two.zip"}]"#);
    pack.font("custom", r#"[{"type":"reference","id":"default"}]"#);
    let atlas = load_hud_font_atlas(&pack.roots()).unwrap();
    for font in [None, Some("minecraft:custom")] {
        let map = atlas.fonts.glyphs(font);
        let surrogate = map.iter().find(|(cp, _)| *cp == 0xd800).unwrap().1;
        assert_eq!((surrogate.width, surrogate.advance), (10, 6.0));
        assert_eq!(map.get('A').unwrap().advance, 6.0);
        assert_eq!(map.get('\u{fffd}').unwrap().advance, 9.0);
        assert_eq!(map.get('\u{10000}').unwrap().advance, 10.0);
        let mut selected_surrogate = false;
        for seed in (0..32).map(|seed| seed * 7919) {
            let glyph = atlas
                .fonts
                .obfuscated_pool(font)
                .random_glyph(
                    6.0,
                    &mut bbb_render_types::HudObfuscatedRandom::with_seed(seed),
                )
                .unwrap();
            if !glyph.is_empty {
                assert_eq!(glyph, surrogate);
                let (atlas_w, atlas_h) = atlas.atlas_dimensions();
                let x = (glyph.uv.min[0] * atlas_w as f32).round() as u32;
                let y = (glyph.uv.min[1] * atlas_h as f32).round() as u32;
                assert_eq!(atlas.pixel_rgba(x, y), [255; 4]);
                selected_surrogate = true;
            }
        }
        assert!(selected_surrogate);
    }
    write_hex("one", "8000");
    let reloaded = load_hud_font_atlas(&pack.roots()).unwrap();
    let glyph = reloaded
        .glyphs
        .iter()
        .find(|(cp, _)| *cp == 0xd800)
        .unwrap()
        .1;
    assert_eq!((glyph.width, glyph.advance), (1, 1.0));
    for seed in 0..8 {
        assert!(
            reloaded
                .fonts
                .obfuscated_pool(None)
                .random_glyph(
                    6.0,
                    &mut bbb_render_types::HudObfuscatedRandom::with_seed(seed)
                )
                .unwrap()
                .is_empty
        );
    }
    assert_eq!(
        atlas
            .glyphs
            .iter()
            .find(|(cp, _)| *cp == 0xd800)
            .unwrap()
            .1
            .width,
        10
    );
}

mod non_fishy;
mod png;
mod ttf;
