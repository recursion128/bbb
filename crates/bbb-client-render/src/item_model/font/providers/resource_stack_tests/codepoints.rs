use super::*;

#[test]
fn font_resource_codepoints_survive_all_provider_kinds_and_unknown_fields() {
    use std::io::{Cursor, Write};
    let packs = Packs::new();
    packs.font(&packs.high, "default", r#"{"ignored":"\ud800","\udfff":null,"providers":[{"type":"reference","id":"spaces"},{"type":"reference","id":"bitmap"},{"type":"reference","id":"hex"}]}"#);
    packs.ttf_asset(&packs.high, "fixture.ttf");
    packs.font(&packs.high, "spaces", r#"{"providers":[{"type":"space","advances":{"A":6,"\ud800":6,"\ufffd":9}},{"type":"ttf","file":"minecraft:fixture.ttf","skip":["\ud800"]}]}"#);
    packs.font(&packs.high, "bitmap", r#"{"providers":[{"type":"bitmap","file":"font/page.png","ascent":7,"chars":["\ud801\ud800\udc00"]}]}"#);
    packs.font(&packs.high, "hex", r#"{"providers":[{"type":"unihex","hex_file":"font/surrogate.zip","size_overrides":[{"from":"\ud800","to":"\udfff","left":0,"right":9}]}]}"#);
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("font.hex", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(format!("D800:{}\nDFFF:{}\n", "8000".repeat(16), "8000".repeat(16)).as_bytes())
        .unwrap();
    let path = packs.high.join("assets/minecraft/font/surrogate.zip");
    std::fs::write(path, zip.finish().unwrap().into_inner()).unwrap();
    let roots = packs.roots(1);
    let atlas = load_hud_font_atlas_with_options(&roots, FontOptions::default()).unwrap();
    let map = atlas.fonts.glyphs(None);
    let lookup = |cp| map.iter().find(|(key, _)| *key == cp).unwrap().1;
    assert!(lookup(0xd800).is_empty);
    assert_eq!(lookup(0xd800).advance, 6.0);
    assert_eq!((lookup(0xd801).width, lookup(0xd801).height), (4, 8));
    assert_eq!(map.get('\u{10000}').unwrap().width, 4);
    assert_ne!(lookup(0xd801).uv, map.get('\u{10000}').unwrap().uv);
    assert_eq!(map.get('\u{fffd}').unwrap().advance, 9.0);
    assert_eq!((lookup(0xdfff).width, lookup(0xdfff).advance), (10, 6.0));
    let hex = atlas.fonts.glyphs(Some("minecraft:hex"));
    assert_eq!(hex.iter().count(), 2);
    assert!(hex
        .iter()
        .all(|(_, glyph)| glyph.width == 10 && !glyph.is_empty));
    let observation = roots.resource_data_memory_observation();
    assert!(observation.json_documents >= 5);
    assert!(observation.json_string_bytes > 0);
}

#[test]
fn font_codepoint_invalid_cardinality_and_ranges_still_discard_the_file() {
    let packs = Packs::new();
    packs.font(
        &packs.low,
        "default",
        r#"{"providers":[{"type":"space","advances":{"L":3}}]}"#,
    );
    for invalid in [
        r#"{"type":"space","advances":{"":3}}"#,
        r#"{"type":"space","advances":{"\ud800A":3}}"#,
        r#"{"type":"bitmap","file":"font/page.png","ascent":7,"chars":["\ud800","\ud800A"]}"#,
        r#"{"type":"unihex","hex_file":"font/u.zip","size_overrides":[{"from":"\ud800","to":"\ud800","left":0,"right":7}]}"#,
        r#"{"type":"unihex","hex_file":"font/u.zip","size_overrides":[{"from":"\udfff","to":"\ud800","left":0,"right":7}]}"#,
    ] {
        packs.font(
            &packs.high,
            "default",
            &format!(r#"{{"providers":[{{"type":"space","advances":{{"X":5}}}},{invalid}]}}"#),
        );
        let atlas = packs.atlas(1, FontOptions::default());
        assert_eq!(atlas.glyphs.get('X'), None);
        assert_eq!(atlas.glyphs.get('L').unwrap().advance, 3.0);
    }
}

#[test]
fn font_codepoint_pair_and_raw_key_replace_without_aliasing_lone_surrogates() {
    let packs = Packs::new();
    packs.font(&packs.high, "default", r#"{"providers":[{"type":"space","advances":{"𐀀":3,"\ud800\udc00":4,"\ud800":5,"\ufffd":6}}]}"#);
    let atlas = packs.atlas(1, FontOptions::default());
    assert_eq!(atlas.glyphs.get('\u{10000}').unwrap().advance, 4.0);
    assert_eq!(atlas.glyphs.get('\u{fffd}').unwrap().advance, 6.0);
    assert_eq!(
        atlas
            .glyphs
            .iter()
            .find(|(cp, _)| *cp == 0xd800)
            .unwrap()
            .1
            .advance,
        5.0
    );
}
