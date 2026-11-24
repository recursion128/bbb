use super::*;

fn atlas(
    width: u32,
    height: u32,
    declared_height: i32,
    ascent: i32,
    grid: &[&str],
) -> HudFontAtlasImage {
    let image = SpriteImage::new(
        "bitmap",
        width,
        height,
        vec![255; (width * height * 4) as usize],
    )
    .unwrap();
    build_hud_font_atlas(&[FontAtlasEntry::Bitmap(
        FontBitmapProviderDefinition {
            file: "minecraft:font/bitmap.png".into(),
            height: declared_height,
            ascent,
            chars: grid
                .iter()
                .map(|row| row.chars().map(u32::from).collect())
                .collect(),
        },
        Arc::new(image),
    )])
    .unwrap()
}

#[test]
fn fractional_bitmap_geometry_preserves_reciprocal_rounding_and_uv_inset() {
    for (height, width_bits, advance) in [(3, 0x40092493, 3.0), (-3, 0xc0092493, 0.0)] {
        let atlas = atlas(5, 7, height, height, &["A"]);
        let glyph = atlas.glyphs.get('A').unwrap();
        assert_eq!((glyph.width, glyph.height), (5, 7));
        // Binary32 results of BitmapProvider/GlyphBitmap arithmetic, including reciprocal rounding.
        assert_eq!(glyph.rendered_width().to_bits(), width_bits);
        assert_eq!(glyph.rendered_height(), height as f32);
        assert_eq!(glyph.advance, advance);
        assert_eq!(glyph.uv.min, [0.01 / 5.0, 0.01 / 15.0]);
        assert_eq!(glyph.uv.max, [4.99 / 5.0, 6.99 / 15.0]);
        let quad = &glyph.styled_quads(0.0, 0.0, Default::default(), false)[0];
        assert_eq!(quad.corners[2], [f32::from_bits(width_bits), 7.0]);
    }
}

#[test]
fn bitmap_advance_uses_double_addition_then_java_int_conversion() {
    for (height, advance) in [
        (8_388_609, 8_388_610.0),
        (i32::MAX, -2_147_483_648.0),
        (i32::MIN, -2_147_483_648.0),
        (-3, -1.0),
        (0, 1.0),
    ] {
        let atlas = atlas(1, 1, height, height, &["A"]);
        let glyph = atlas.glyphs.get('A').unwrap();
        assert_eq!(glyph.advance, advance, "height={height}");
        assert_eq!(glyph.baseline_offset(), 7.0 - height as f32);
    }
}

#[test]
fn zero_bitmap_cells_preserve_glyph_and_metrics() {
    for (width, height, grid, expected_size) in [
        (1, 2, vec!["AB"], (0, 2)),
        (2, 1, vec!["A", "B"], (2, 0)),
        (1, 1, vec!["AB", "CD"], (0, 0)),
    ] {
        let atlas = atlas(width, height, 8, 7, &grid);
        let glyph = atlas.glyphs.get('A').unwrap();
        assert_eq!((glyph.width, glyph.height), expected_size);
        assert_eq!(glyph.advance, 1.0);
        if expected_size.1 == 0 {
            assert!(glyph.rendered_height().is_nan());
        }
    }
}

#[test]
fn oversized_bitmap_cell_uses_missing_metrics_and_pixels() {
    for (width, height) in [(257, 1), (1, 257), (256, 256)] {
        let atlas = atlas(width, height, 8, 7, &["A"]);
        let glyph = atlas.glyphs.get('A').unwrap();
        if width > 256 || height > 256 {
            assert_eq!(glyph, atlas.glyphs.missing_glyph().unwrap());
        } else {
            assert_eq!((glyph.width, glyph.height), (256, 256));
        }
    }
}

#[test]
fn oversized_first_bitmap_does_not_select_later_provider() {
    let entries = [(257, "first"), (8, "later")].map(|(width, name)| {
        FontAtlasEntry::Bitmap(
            FontBitmapProviderDefinition {
                file: format!("minecraft:font/{name}.png"),
                height: 8,
                ascent: 7,
                chars: vec![vec![u32::from('A')]],
            },
            Arc::new(
                SpriteImage::new(name, width, 8, vec![255; (width * 8 * 4) as usize]).unwrap(),
            ),
        )
    });
    let atlas = build_hud_font_atlas(&entries).unwrap();
    assert_eq!(
        atlas.glyphs.get('A').unwrap(),
        atlas.glyphs.missing_glyph().unwrap()
    );
}

#[test]
fn zero_cell_height_keeps_nonfinite_scale_without_losing_advance() {
    for height in [-8, 0, 8] {
        let atlas = atlas(1, 1, height, height, &["A", "B"]);
        let glyph = atlas.glyphs.get('A').unwrap();
        assert_eq!(glyph.advance, 1.0);
        if height == 0 {
            assert!(glyph.render_scale.is_nan());
            assert!(glyph.rendered_width().is_nan());
        } else {
            assert!(glyph.render_scale.is_infinite());
            assert_eq!(
                glyph.rendered_width(),
                if height < 0 {
                    f32::NEG_INFINITY
                } else {
                    f32::INFINITY
                }
            );
        }
        assert!(glyph.rendered_height().is_nan());
    }
}
