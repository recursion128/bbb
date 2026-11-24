use super::*;
use crate::item_model::font::{load_hud_font_atlas_with_options, HudFontAtlasImage};
use std::path::{Path, PathBuf};

mod codepoints;

struct Packs {
    root: PathBuf,
    low: PathBuf,
    high: PathBuf,
}

impl Packs {
    fn new() -> Self {
        let root = super::tests::unique_temp_dir("font-resource-stack");
        let low = root.join("packs/low");
        let high = root.join("packs/high");
        for pack in [&low, &high] {
            write(
                &pack.join("pack.mcmeta"),
                r#"{"pack":{"description":"font test","min_format":[84,0],"max_format":[84,0]}}"#,
            );
        }
        let source = root.join("sources/26.1/assets/minecraft");
        write(
            &source.join("font/default.json"),
            r#"{"providers":[{"type":"bitmap","file":"minecraft:font/page.png","ascent":7,"chars":["Z"]}]}"#,
        );
        let texture = source.join("textures/font/page.png");
        std::fs::create_dir_all(texture.parent().unwrap()).unwrap();
        image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 255, 255, 255]))
            .save(texture)
            .unwrap();
        Self { root, low, high }
    }

    fn font(&self, pack: &Path, name: &str, value: &str) {
        write(
            &pack.join(format!("assets/minecraft/font/{name}.json")),
            value,
        );
    }

    /// A real TrueType asset: the definition's `file` gains `font/` at load,
    /// so `minecraft:fixture.ttf` is this file.
    fn ttf_asset(&self, pack: &Path, name: &str) {
        let path = pack.join(format!("assets/minecraft/font/{name}"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, super::super::ttf::fixture::fixture_ttf()).unwrap();
    }

    fn roots(&self, generation: u64) -> PackRoots {
        let sources = self.root.join("sources/26.1");
        let roots = PackRoots::from_paths(&self.root, &sources, sources.join("assets/minecraft"));
        let selected = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(
            &roots,
            [self.low.clone(), self.high.clone()],
        )
        .unwrap();
        roots
            .with_client_resource_generation(generation, selected)
            .unwrap()
    }

    fn atlas(&self, generation: u64, options: FontOptions) -> HudFontAtlasImage {
        load_hud_font_atlas_with_options(&self.roots(generation), options).unwrap()
    }
}

impl Drop for Packs {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

fn write(path: &Path, value: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, value).unwrap();
}

#[test]
fn font_stack_atlas_keeps_lower_glyphs_and_orders_conflicts_before_reload() {
    let packs = Packs::new();
    packs.font(
        &packs.low,
        "default",
        r#"{"providers":[{"type":"space","advances":{"A":3,"C":4}}]}"#,
    );
    packs.font(&packs.high, "default", r#"{"providers":[{"type":"space","advances":{"B":5,"C":6}},{"type":"space","advances":{"C":9}}]}"#);
    let first = packs.atlas(1, FontOptions::default());
    let glyphs = first.fonts.glyphs(None);
    for (ch, advance) in [('A', 3.0), ('B', 5.0), ('C', 6.0)] {
        assert_eq!(glyphs.get(ch).unwrap().advance, advance);
    }
    assert!(glyphs.get('Z').unwrap().width > 0);
    assert!(first.rgba.iter().any(|byte| *byte != 0));
    packs.font(
        &packs.high,
        "default",
        r#"{"providers":[{"type":"space","advances":{"B":7}}]}"#,
    );
    let second = packs.atlas(2, FontOptions::default());
    assert_eq!(second.fonts.glyphs(None).get('B').unwrap().advance, 7.0);
    assert_eq!(second.fonts.glyphs(None).get('C').unwrap().advance, 4.0);
    assert_eq!(first.fonts.glyphs(None).get('B').unwrap().advance, 5.0);
}

#[test]
fn font_stack_references_resolve_all_layers_and_preserve_option_filters() {
    let packs = Packs::new();
    packs.font(&packs.high, "default", r#"{"providers":[{"type":"reference","id":"minecraft:shared","filter":{"uniform":false}}]}"#);
    packs.font(
        &packs.low,
        "shared",
        r#"{"providers":[{"type":"space","advances":{"L":2,"C":3}}]}"#,
    );
    packs.font(
        &packs.high,
        "shared",
        r#"{"providers":[{"type":"space","advances":{"H":5,"C":6}}]}"#,
    );
    let normal = packs.atlas(1, FontOptions::default());
    for (ch, advance) in [('L', 2.0), ('H', 5.0), ('C', 6.0)] {
        assert_eq!(normal.fonts.glyphs(None).get(ch).unwrap().advance, advance);
    }
    let uniform = packs.atlas(
        2,
        FontOptions {
            uniform: true,
            ..FontOptions::default()
        },
    );
    assert!(uniform.fonts.glyphs(None).get('L').is_none());
    assert!(uniform.fonts.glyphs(None).get('Z').is_some());
}

#[test]
fn font_stack_pack_filter_removes_lower_definitions_before_reference_expansion() {
    let packs = Packs::new();
    packs.font(
        &packs.high,
        "default",
        r#"{"providers":[{"type":"reference","id":"minecraft:shared"}]}"#,
    );
    packs.font(
        &packs.low,
        "shared",
        r#"{"providers":[{"type":"space","advances":{"L":2}}]}"#,
    );
    packs.font(
        &packs.high,
        "shared",
        r#"{"providers":[{"type":"space","advances":{"H":5}}]}"#,
    );
    write(
        &packs.high.join("pack.mcmeta"),
        r#"{"pack":{"description":"filter","min_format":[84,0],"max_format":[84,0]},"filter":{"block":[{"namespace":"minecraft","path":"font/shared\\.json"}]}}"#,
    );
    let atlas = packs.atlas(1, FontOptions::default());
    assert!(atlas.fonts.glyphs(None).get('L').is_none());
    assert_eq!(atlas.fonts.glyphs(None).get('H').unwrap().advance, 5.0);
}

#[test]
fn font_stack_invalid_high_json_keeps_valid_lower_providers() {
    let packs = Packs::new();
    packs.font(
        &packs.low,
        "default",
        r#"{"providers":[{"type":"space","advances":{"L":2}}]}"#,
    );
    for invalid in ["{", r#"{"providers":null}"#] {
        packs.font(&packs.high, "default", invalid);
        let atlas = packs.atlas(1, FontOptions::default());
        assert_eq!(atlas.fonts.glyphs(None).get('L').unwrap().advance, 2.0);
        assert!(atlas.fonts.glyphs(None).get('Z').is_some());
    }
}

#[test]
fn font_stack_all_invalid_json_keeps_empty_providers_and_bakes_missing_atlas() {
    let packs = Packs::new();
    write(
        &packs
            .root
            .join("sources/26.1/assets/minecraft/font/default.json"),
        "{",
    );
    packs.font(&packs.low, "default", "{");
    packs.font(&packs.high, "default", "{");
    assert!(load_font_providers(&packs.roots(1), "minecraft:default")
        .unwrap()
        .is_empty());
    let atlas = load_hud_font_atlas_with_options(&packs.roots(2), FontOptions::default()).unwrap();
    assert!(atlas.fonts.glyphs(None).is_empty());
    let missing = atlas
        .fonts
        .glyphs(None)
        .missing_glyph()
        .expect("missing glyph is baked even without providers");
    assert!(missing.width > 0 && missing.height > 0);
    assert!(atlas.rgba.iter().any(|byte| *byte != 0));
}

#[test]
fn font_stack_resource_limit_errors_do_not_fall_back_to_lower_packs() {
    let packs = Packs::new();
    packs.font(&packs.high, "default", "{}");
    std::fs::OpenOptions::new()
        .write(true)
        .open(packs.high.join("assets/minecraft/font/default.json"))
        .unwrap()
        .set_len(32 * 1024 * 1024)
        .unwrap();
    let error = load_font_providers(&packs.roots(1), "minecraft:default").unwrap_err();
    assert!(format!("{error:#}").contains("byte limit"));
}

#[test]
fn font_stack_fractional_and_negative_spaces_reach_atlas_without_per_glyph_rounding() {
    let packs = Packs::new();
    packs.font(
        &packs.low,
        "default",
        r#"{"providers":[{"type":"space","advances":{"A":9,"B":9,"C":9}}]}"#,
    );
    packs.font(&packs.high, "default", r#"{"providers":[{"type":"space","advances":{"A":0.5,"B":-2.25,"C":6}},{"type":"space","advances":{"A":8}}]}"#);
    let atlas = packs.atlas(1, FontOptions::default());
    let glyphs = atlas.fonts.glyphs(None);
    assert_eq!(glyphs.plain_text_width_f32("AA"), 1.0);
    assert_eq!(glyphs.plain_text_width("AA"), 1);
    assert_eq!(glyphs.plain_text_width_f32("CBA"), 4.25);
    assert_eq!(glyphs.plain_text_width("CBA"), 5);
    let mut pen = 0.0;
    for ch in "CBA".chars() {
        let glyph = glyphs.get(ch).unwrap();
        assert_eq!((glyph.width, glyph.height), (0, 0));
        pen += glyph.styled_advance_f32(Default::default());
    }
    assert_eq!(pen, 4.25);
    let bold = bbb_render_types::HudTextStyle {
        bold: true,
        ..Default::default()
    };
    assert_eq!(glyphs.get('A').unwrap().styled_advance_f32(bold), 1.5);
}

#[test]
fn font_stack_unihex_bold_half_pixel_advance_reaches_loaded_atlas() {
    use std::io::Write;
    let packs = Packs::new();
    packs.font(&packs.high, "default", r#"{"providers":[{"type":"unihex","hex_file":"minecraft:font/test.zip","size_overrides":[]}]}"#);
    let file = std::fs::File::create(packs.high.join("assets/minecraft/font/test.zip")).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("glyph.hex", zip::write::SimpleFileOptions::default())
        .unwrap();
    writeln!(zip, "0044:{}", "FFFF".repeat(16)).unwrap();
    zip.finish().unwrap();
    let atlas = packs.atlas(1, FontOptions::default());
    let glyph = atlas.fonts.glyphs(None).get('D').unwrap();
    assert_eq!(glyph.advance, 9.0);
    let bold = bbb_render_types::HudTextStyle {
        bold: true,
        ..Default::default()
    };
    assert_eq!(glyph.styled_advance_f32(bold), 9.5);
    assert_eq!(glyph.styled_advance_f32(bold) * 2.0, 19.0);
    let quads = glyph.styled_quads(0.0, 0.0, bold, false);
    assert_eq!(quads.len(), 2);
    assert_eq!(quads[1].corners[0][0] - quads[0].corners[0][0], 0.5);
}

#[test]
fn font_codec_failure_discards_entire_high_resource_even_when_filtered_out() {
    let packs = Packs::new();
    packs.font(
        &packs.low,
        "default",
        r#"{"providers":[{"type":"space","advances":{"L":2}}]}"#,
    );
    for invalid in [
        r#"{"type":"unknown","filter":{"uniform":true}}"#,
        r#"{"type":"bitmap","filter":{"uniform":true},"file":"minecraft:bad","ascent":7,"chars":[]}"#,
        r#"{"type":"space","advances":{"X":2},"filter":{"jp":true,"unknown":false}}"#,
        r#"{"type":"space","advances":{"X":2},"filter":{"jp":true,"uniform":1}}"#,
        r#"{"type":"reference","id":"BAD ID","filter":{"uniform":true}}"#,
    ] {
        packs.font(
            &packs.high,
            "default",
            &format!(r#"{{"providers":[{{"type":"space","advances":{{"H":9}}}},{invalid}]}}"#),
        );
        let atlas = packs.atlas(1, FontOptions::default());
        let glyphs = atlas.fonts.glyphs(None);
        assert!(glyphs.get('H').is_none(), "{invalid}");
        assert_eq!(glyphs.get('L').unwrap().advance, 2.0);
    }
}

#[test]
fn font_ttf_codec_validates_every_field_and_rejects_unusable_providers() {
    let packs = Packs::new();
    // `minecraft:a.ttf` resolves through the pack to `assets/minecraft/font/a.ttf`
    // (vanilla `location.withPrefix("font/")`).
    packs.ttf_asset(&packs.high, "a.ttf");
    for valid in [
        r#"{"type":"ttf","file":"minecraft:a.ttf"}"#,
        r#"{"type":"ttf","file":"minecraft:a.ttf","size":-2.5,"oversample":-1,"shift":[-512,512]}"#,
        r#"{"type":"ttf","file":"minecraft:a.ttf","shift":[512,-512],"skip":["abc","中"]}"#,
        r#"{"type":"ttf","file":"minecraft:a.ttf","skip":"abc"}"#,
        r#"{"type":"ttf","file":"minecraft:a.ttf","oversample":0}"#,
        r#"{"type":"ttf","file":"minecraft:a.ttf","skip":"\ud800"}"#,
    ] {
        packs.font(
            &packs.high,
            "default",
            &format!(r#"{{"providers":[{{"type":"space","advances":{{"H":9}}}},{valid}]}}"#),
        );
        assert_eq!(
            packs
                .atlas(1, FontOptions::default())
                .fonts
                .glyphs(None)
                .get('H')
                .unwrap()
                .advance,
            9.0,
            "{valid}"
        );
    }
    // Missing assets reject the provider and its containing default font.
    {
        let unusable = r#"{"type":"ttf","file":"minecraft:absent.ttf"}"#;
        packs.font(
            &packs.high,
            "default",
            &format!(r#"{{"providers":[{{"type":"space","advances":{{"H":9}}}},{unusable}]}}"#),
        );
        let error = load_hud_font_atlas_with_options(&packs.roots(1), FontOptions::default())
            .expect_err("an unusable ttf provider rejects the default font");
        assert!(
            format!("{error:#}").contains("default font failed to load"),
            "{unusable}: {error:#}"
        );
    }
    for invalid in [
        r#"{"type":"ttf"}"#,
        r#"{"type":"ttf","file":"BAD ID"}"#,
        r#"{"type":"ttf","file":"a","size":1e39}"#,
        r#"{"type":"ttf","file":"a","oversample":"x"}"#,
        r#"{"type":"ttf","file":"a","shift":[0]}"#,
        r#"{"type":"ttf","file":"a","shift":[0,513]}"#,
        r#"{"type":"ttf","file":"a","skip":["x",4]}"#,
    ] {
        packs.font(
            &packs.high,
            "default",
            &format!(r#"{{"providers":[{{"type":"space","advances":{{"H":9}}}},{invalid}]}}"#),
        );
        assert!(
            packs
                .atlas(1, FontOptions::default())
                .fonts
                .glyphs(None)
                .get('H')
                .is_none(),
            "{invalid}"
        );
    }
}

#[test]
fn font_reference_outer_filters_override_inner_and_repeated_paths_remain_distinct() {
    let packs = Packs::new();
    packs.font(&packs.low, "default", r#"{"providers":[{"type":"reference","id":"minecraft:shared","filter":{"uniform":false}}]}"#);
    packs.font(
        &packs.high,
        "default",
        r#"{"providers":[{"type":"reference","id":"minecraft:shared","filter":{"uniform":true}}]}"#,
    );
    packs.font(&packs.low, "shared", r#"{"providers":[{"type":"space","advances":{"L":2},"filter":{"uniform":true}},{"type":"space","advances":{"J":3},"filter":{"jp":true}}]}"#);
    let atlas = packs.atlas(1, FontOptions::default());
    let glyphs = atlas.fonts.glyphs(None);
    assert_eq!(glyphs.get('L').unwrap().advance, 2.0);
    assert!(glyphs.get('J').is_none());
}

#[test]
fn font_filtered_missing_reference_still_invalidates_font() {
    let packs = Packs::new();
    packs.font(&packs.high, "default", r#"{"providers":[{"type":"reference","id":"minecraft:missing","filter":{"uniform":true}}]}"#);
    let error = load_font_providers(&packs.roots(1), "minecraft:default").unwrap_err();
    assert!(error.to_string().contains("missing font definition"));
}

#[test]
fn font_invalid_codec_does_not_hide_provider_collection_limit() {
    let packs = Packs::new();
    let entries = vec![serde_json::json!({"type":"unknown"}); MAX_FONT_PROVIDERS as usize + 1];
    packs.font(
        &packs.high,
        "default",
        &serde_json::json!({"providers": entries}).to_string(),
    );
    let error = load_font_providers(&packs.roots(1), "minecraft:default").unwrap_err();
    assert!(format!("{error:#}").contains("font providers"));
}

#[test]
fn font_stack_unihex_width_limit_is_not_a_discardable_codec_error() {
    let packs = Packs::new();
    for (left, right) in [(0, 64), (i32::MIN, i32::MAX), (1, 0)] {
        packs.font(&packs.high, "default", &format!(r#"{{"providers":[{{"type":"unihex","hex_file":"minecraft:font/unsafe.zip","filter":{{"uniform":true}},"size_overrides":[{{"from":"A","to":"B","left":{left},"right":{right}}}]}}]}}"#));
        let error = load_font_providers(&packs.roots(1), "minecraft:default").unwrap_err();
        assert!(error.is::<UnsafeUnihexWidth>(), "{error:#}");
    }
}

#[test]
fn font_candidates_keep_disabled_reference_contexts_and_shared_definitions() {
    let packs = Packs::new();
    packs.font(
        &packs.high,
        "custom",
        r#"{"providers":[
        {"type":"reference","id":"minecraft:shared","filter":{"uniform":true}},
        {"type":"reference","id":"minecraft:shared","filter":{"uniform":false}}
    ]}"#,
    );
    packs.font(
        &packs.high,
        "shared",
        r#"{"providers":[
        {"type":"space","advances":{"A":3},"filter":{"uniform":true}},
        {"type":"bitmap","file":"minecraft:font/page.png","ascent":7,"chars":["A"]},
        {"type":"unihex","hex_file":"minecraft:font/u.zip"}
    ]}"#,
    );
    let roots = packs.roots(1);
    let candidates =
        load_font_candidates(&roots, "minecraft:custom", FontOptions::default()).unwrap();
    assert!(candidates.reference_failure.is_none());
    assert_eq!(
        candidates
            .providers
            .iter()
            .map(|p| p.enabled)
            .collect::<Vec<_>>(),
        vec![false, false, false, true, true, true]
    );
    for i in 0..3 {
        assert!(Arc::ptr_eq(
            &candidates.providers[i].definition,
            &candidates.providers[i + 3].definition
        ));
    }
    assert_eq!(
        load_font_providers(&roots, "minecraft:custom")
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn font_candidates_retain_siblings_after_missing_and_cyclic_references() {
    for cycle in [false, true] {
        let packs = Packs::new();
        packs.font(
            &packs.high,
            "custom",
            r#"{"providers":[
            {"type":"reference","id":"minecraft:child"},
            {"type":"reference","id":"minecraft:child","filter":{"uniform":true}},
            {"type":"space","advances":{"B":5}}
        ]}"#,
        );
        let target = if cycle { "custom" } else { "missing" };
        packs.font(
            &packs.high,
            "child",
            &format!(
                r#"{{"providers":[
            {{"type":"reference","id":"minecraft:{target}"}},
            {{"type":"space","advances":{{"A":3}}}}
        ]}}"#
            ),
        );
        let roots = packs.roots(1);
        let candidates =
            load_font_candidates(&roots, "minecraft:custom", FontOptions::default()).unwrap();
        let failure = if cycle {
            FontReferenceFailure::Cycle("minecraft:custom".into())
        } else {
            FontReferenceFailure::MissingDefinition("minecraft:missing".into())
        };
        assert_eq!(candidates.reference_failure, Some(failure.clone()));
        assert_eq!(candidates.providers.len(), 3);
        assert_eq!(
            candidates
                .providers
                .iter()
                .map(|p| p.enabled)
                .collect::<Vec<_>>(),
            vec![true, false, true]
        );
        let error = load_font_providers(&roots, "minecraft:custom").unwrap_err();
        assert_eq!(error.downcast_ref::<FontReferenceFailure>(), Some(&failure));
    }
}

#[test]
fn missing_font_does_not_hide_later_provider_collection_limit() {
    let packs = Packs::new();
    packs.font(
        &packs.high,
        "custom",
        r#"{"providers":[
        {"type":"reference","id":"minecraft:missing"},
        {"type":"reference","id":"minecraft:large"}
    ]}"#,
    );
    let entries = vec![r#"{"type":"space","advances":{}}"#; MAX_FONT_PROVIDERS as usize];
    packs.font(
        &packs.high,
        "large",
        &format!(r#"{{"providers":[{}]}}"#, entries.join(",")),
    );
    let error = load_font_candidates(&packs.roots(1), "minecraft:custom", FontOptions::default())
        .unwrap_err();
    assert!(!error.is::<FontReferenceFailure>());
    assert!(error.to_string().contains("font providers"), "{error:#}");
}

#[test]
fn missing_font_does_not_hide_unsafe_width_in_later_reference() {
    let packs = Packs::new();
    packs.font(
        &packs.high,
        "custom",
        r#"{"providers":[
        {"type":"reference","id":"minecraft:missing"},
        {"type":"reference","id":"minecraft:unsafe","filter":{"uniform":true}}
    ]}"#,
    );
    packs.font(
        &packs.high,
        "unsafe",
        r#"{"providers":[
        {"type":"unihex","hex_file":"minecraft:font/u.zip","size_overrides":[
            {"from":"A","to":"Z","left":-2147483648,"right":2147483647}
        ]}
    ]}"#,
    );
    let error = load_font_candidates(&packs.roots(1), "minecraft:custom", FontOptions::default())
        .unwrap_err();
    assert!(error.is::<UnsafeUnihexWidth>(), "{error:#}");
}

#[test]
fn missing_definition_does_not_soften_later_io_failure() {
    let mut load = |id: &str| -> Result<Vec<Conditional>> {
        match id {
            "minecraft:custom" => codec::decode(
                id,
                &Value::from_slice(
                    br#"{"providers":[
                    {"type":"reference","id":"minecraft:missing"},
                    {"type":"reference","id":"minecraft:io"}
                ]}"#,
                )
                .unwrap(),
            ),
            "minecraft:missing" => Err(FontReferenceFailure::MissingDefinition(id.into()).into()),
            "minecraft:io" => Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "fixture denied",
            )
            .into()),
            _ => panic!("unexpected reference"),
        }
    };
    let error = flatten_definitions("minecraft:custom", FontOptions::default(), &mut load, None)
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<std::io::Error>().unwrap().kind(),
        std::io::ErrorKind::PermissionDenied
    );
}
