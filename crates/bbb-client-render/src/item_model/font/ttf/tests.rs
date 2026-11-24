use super::fixture::{fixture_ttf, fixture_ttf_with, table_checksum, CmapKind, FixtureSpec};
use super::*;
use ttf_parser::{Face, GlyphId};

fn definition(
    size: f32,
    oversample: f32,
    shift: [f32; 2],
    skip: &str,
) -> FontTrueTypeProviderDefinition {
    FontTrueTypeProviderDefinition {
        file: "minecraft:font/fixture.ttf".to_string(),
        size,
        oversample,
        shift_x: shift[0],
        shift_y: shift[1],
        skip: skip.chars().map(u32::from).collect(),
    }
}

fn build(bytes: &[u8], definition: &FontTrueTypeProviderDefinition) -> Result<TtfGlyphSource> {
    build_ttf_glyph_source(Arc::from(bytes), definition, None)
}

fn page(definition: &FontTrueTypeProviderDefinition) -> TtfGlyphSource {
    build(&fixture_ttf(), definition).unwrap()
}

fn glyph_with_index(page: &TtfGlyphSource, codepoint: char) -> (usize, TtfGlyph) {
    page.glyphs
        .iter()
        .enumerate()
        .find(|(_, glyph)| glyph.codepoint == u32::from(codepoint))
        .map(|(idx, glyph)| (idx, *glyph))
        .unwrap_or_else(|| panic!("{codepoint} should be in the fixture charmap"))
}

fn glyph(page: &TtfGlyphSource, codepoint: char) -> TtfGlyph {
    glyph_with_index(page, codepoint).1
}

/// Byte offset of a table body from the sfnt table directory.
fn table_offset(bytes: &[u8], tag: [u8; 4]) -> usize {
    let count = u16::from_be_bytes(bytes[4..6].try_into().unwrap()) as usize;
    for index in 0..count {
        let record = 12 + 16 * index;
        if bytes[record..record + 4] == tag {
            return u32::from_be_bytes(bytes[record + 8..record + 12].try_into().unwrap()) as usize;
        }
    }
    panic!("font has no {:?} table", String::from_utf8_lossy(&tag));
}

#[test]
fn fixture_font_is_a_checksummed_truetype_face() {
    let bytes = fixture_ttf();
    let face = Face::parse(&bytes, 0).unwrap();
    assert_eq!(face.units_per_em(), 1000);
    assert_eq!(face.number_of_glyphs(), 5);
    assert!(face.tables().glyf.is_some());
    for (codepoint, glyph_id) in [('\u{20}', 1), ('A', 2), ('B', 3), ('C', 4)] {
        assert_eq!(face.glyph_index(codepoint), Some(GlyphId(glyph_id)));
    }
    assert_eq!(face.glyph_index('Z'), None);
    // The directory checksums and `head.checkSumAdjustment` (offset 8 of the
    // head table) satisfy the sfnt invariant.
    assert_eq!(
        table_checksum(&bytes),
        0xB1B0_AFBA,
        "a checked font's whole-file checksum plus checkSumAdjustment is 0xB1B0AFBA"
    );
    // Zeroing `checkSumAdjustment` must make the whole-file checksum
    // its complement, which is what the patched value was derived from.
    let head_offset = table_offset(&bytes, *b"head");
    let adjustment =
        u32::from_be_bytes(bytes[head_offset + 8..head_offset + 12].try_into().unwrap());
    let mut zeroed = bytes.clone();
    zeroed[head_offset + 8..head_offset + 12].fill(0);
    assert_eq!(
        adjustment,
        0xB1B0_AFBA_u32.wrapping_sub(table_checksum(&zeroed))
    );
}

#[test]
fn truetype_scaling_matches_the_fixture_metrics() {
    // size 10 at unitsPerEm 1000 is a 0.01 px/unit scale: `A`'s 600-unit
    // advance is exactly 6px, its 100..900 outline lands on cells 1..9.
    let page = page(&definition(10.0, 1.0, [0.0, 0.0], ""));
    assert_eq!(page.oversample, 1.0);

    let a = glyph(&page, 'A');
    assert_eq!(a.advance, 6.0);
    assert_eq!((a.bearing_left, a.bearing_top), (1.0, 7.0));
    assert_eq!(
        a.bounds,
        Some(TtfGlyphBounds {
            width: 8,
            height: 7
        })
    );

    // `B` spans x 50..550 (0.5..5.5px -> pixel bounds 0..6) and y 0..600 (6px).
    let b = glyph(&page, 'B');
    assert_eq!(b.advance, 6.0);
    assert_eq!((b.bearing_left, b.bearing_top), (0.0, 6.0));
    assert_eq!(b.bounds.unwrap().width, 6);
    assert_eq!(b.bounds.unwrap().height, 6);

    // `C` has a negative left side bearing (-20 -> -0.2px -> floor -1).
    let c = glyph(&page, 'C');
    assert_eq!((c.bearing_left, c.bearing_top), (-1.0, 7.0));
    assert_eq!(c.bounds.unwrap().width, 6);

    // U+0020 has no outline: Vanilla answers `EmptyGlyph`, which keeps the
    // advance and has no bearings.
    let space = glyph(&page, ' ');
    assert_eq!(space.advance, 3.0);
    assert_eq!(space.bounds, None);
}

#[test]
fn default_size_11_scales_outline_bounds_and_advance() {
    let page = page(&definition(11.0, 1.0, [0.0, 0.0], ""));
    let a = glyph(&page, 'A');
    assert!((a.advance - 6.6).abs() < 1e-5);
    assert_eq!(a.bearing_top, 8.0);
    assert_eq!(a.bounds.unwrap().width, 9);
    assert_eq!(a.bounds.unwrap().height, 8);
}

#[test]
fn oversample_scales_the_face_and_divides_every_metric() {
    // `pixelsPerEm = round(size * oversample)`: oversample 2 renders at 20px
    // and divides the advance, bearings and pixel size by two.
    let page = page(&definition(10.0, 2.0, [0.0, 0.0], ""));
    assert_eq!(page.oversample, 2.0);
    let a = glyph(&page, 'A');
    assert_eq!(a.advance, 6.0);
    assert_eq!((a.bearing_left, a.bearing_top), (1.0, 7.0));
    assert_eq!(
        a.bounds,
        Some(TtfGlyphBounds {
            width: 16,
            height: 14
        })
    );
    // `B` spans 50..550 units, i.e. 1px..11px at 20 pixels per em.
    let b = glyph(&page, 'B');
    assert_eq!(b.bearing_left, 0.5);
    assert_eq!(b.bounds.unwrap().width, 10);
    assert_eq!(b.bounds.unwrap().height, 12);
}

#[test]
fn shift_translates_the_outline_but_not_the_advance() {
    let page = page(&definition(10.0, 1.0, [1.0, 2.0], ""));
    let a = glyph(&page, 'A');
    assert_eq!(a.advance, 6.0);
    assert_eq!(a.bearing_left, 2.0);
    // Positive shift.y moves the outline down two pixels.
    assert_eq!(a.bearing_top, 5.0);
    assert_eq!(
        a.bounds,
        Some(TtfGlyphBounds {
            width: 8,
            height: 7
        })
    );
}

#[test]
fn skip_removes_exactly_the_named_codepoints() {
    let page = page(&definition(10.0, 1.0, [0.0, 0.0], "AC\u{20}"));
    let codepoints = page
        .glyphs
        .iter()
        .map(|glyph| glyph.codepoint)
        .collect::<Vec<_>>();
    assert_eq!(codepoints, vec![u32::from('B')]);
}

#[test]
fn rasterized_triangle_stores_grayscale_coverage_in_the_shared_atlas() {
    let page = page(&definition(10.0, 1.0, [0.0, 0.0], ""));
    let (index, a) = glyph_with_index(&page, 'A');
    let cell = a.bounds.unwrap();
    let rgba = page.rasterize_glyph_rgba(index);
    let coverage = |x: u32, y: u32| rgba[((y * cell.width + x) * 4) as usize];
    // The 8x7 cell holds a triangle whose apex points up: rasterizer rows run
    // downwards, so row 0 is the apex and row 6 the base.
    assert!(coverage(3, 0) > 0);
    assert_eq!(coverage(7, 0), 0);
    assert!(coverage(0, 6) > 0);
    assert!(coverage(7, 6) > 0);
    let total = (0..cell.height)
        .flat_map(|y| (0..cell.width).map(move |x| (x, y)))
        .map(|(x, y)| u32::from(coverage(x, y)))
        .sum::<u32>();
    let area = cell.width * cell.height * 255;
    assert!(
        total > area * 2 / 5 && total < area * 3 / 5,
        "triangle coverage {total} of {area} should be about half"
    );
    // `rendertype_text_intensity.fsh` samples `.rrrr`, so every channel
    // carries the coverage.
    for y in 0..cell.height {
        for x in 0..cell.width {
            let offset = ((y * cell.width + x) * 4) as usize;
            assert_eq!(rgba[offset..offset + 4], [coverage(x, y); 4], "({x}, {y})");
        }
    }
}

#[test]
fn glyphs_without_a_bitmap_area_stay_empty_and_pages_hold_every_cell() {
    let page = page(&definition(10.0, 1.0, [0.0, 0.0], ""));
    let cells = page
        .glyphs
        .iter()
        .filter_map(|glyph| glyph.bounds)
        .collect::<Vec<_>>();
    assert_eq!(cells.len(), 3);
    assert!(page.has_raster_glyphs());
    assert_eq!(
        page.rasterize_glyph_rgba(0).len(),
        0,
        "space glyph without outline rasterizes to empty"
    );
}

#[test]
fn faces_without_glyf_or_a_unicode_charmap_are_rejected() {
    let error = build(
        &fixture_ttf_with(FixtureSpec {
            glyf: false,
            cmap: CmapKind::WindowsUnicodeBmp,
        }),
        &definition(10.0, 1.0, [0.0, 0.0], ""),
    )
    .unwrap_err();
    assert!(error.is::<TtfProviderInvalid>(), "{error:#}");
    assert!(
        format!("{error:#}").contains("initialize TrueType face"),
        "{error:#}"
    );

    let error = build(
        &fixture_ttf_with(FixtureSpec {
            glyf: true,
            cmap: CmapKind::MacintoshRoman,
        }),
        &definition(10.0, 1.0, [0.0, 0.0], ""),
    )
    .unwrap_err();
    assert!(error.is::<TtfProviderInvalid>(), "{error:#}");
    assert!(
        format!("{error:#}").contains("no unicode character map"),
        "{error:#}"
    );

    // R9-06 clamps zero/negative `oversample` to `>= 0.1` so TTF glyphs never produce infinite advances.
    let zero = build(&fixture_ttf(), &definition(-2.5, 0.0, [0.0, 0.0], "")).unwrap();
    assert_eq!(zero.oversample, 0.1);
    assert!(glyph(&zero, ' ').advance.is_finite());
    assert!(glyph(&zero, 'A').advance.is_finite());

    let error = build(&[0_u8; 16], &definition(10.0, 1.0, [0.0, 0.0], "")).unwrap_err();
    assert!(error.is::<TtfProviderInvalid>(), "{error:#}");
}

#[test]
fn java_rounding_matches_provider_transform() {
    assert_eq!(java_round_float(2.5), 3);
    assert_eq!(java_round_float(-2.5), -2);
}
