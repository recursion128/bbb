use super::*;

pub(super) const WIDTH: u32 = 5;
pub(super) const HEIGHT: u32 = 8;

pub(super) fn blit(dst: &mut [u8], atlas_width: u32, x: u32, y: u32) -> Result<()> {
    let mut pixels = [0; (WIDTH * HEIGHT * 4) as usize];
    for row in 0..HEIGHT {
        for column in 0..WIDTH {
            if column == 0 || column == WIDTH - 1 || row == 0 || row == HEIGHT - 1 {
                let offset = ((row * WIDTH + column) * 4) as usize;
                pixels[offset..offset + 4].fill(255);
            }
        }
    }
    copy_rgba_image(dst, atlas_width, x, y, WIDTH, &pixels)
}

pub(super) fn glyph(atlas_width: u32, atlas_height: u32, x: u32, y: u32) -> HudDigitGlyph {
    HudDigitGlyph {
        uv: HudUvRect {
            min: [
                (x as f32 + 0.01) / atlas_width as f32,
                (y as f32 + 0.01) / atlas_height as f32,
            ],
            max: [
                (x as f32 - 0.01 + WIDTH as f32) / atlas_width as f32,
                (y as f32 - 0.01 + HEIGHT as f32) / atlas_height as f32,
            ],
        },
        width: WIDTH,
        height: HEIGHT,
        advance: 6.0,
        ..HudDigitGlyph::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_missing(atlas: &HudFontAtlasImage) {
        let glyph = atlas.glyphs.missing_glyph().unwrap();
        assert_eq!((glyph.width, glyph.height, glyph.advance), (5, 8, 6.0));
        assert_eq!(
            (
                glyph.baseline_offset(),
                glyph.bold_offset,
                glyph.shadow_offset
            ),
            (0.0, 1.0, 1.0)
        );
        let x = (glyph.uv.min[0] * atlas.width as f32).round() as u32;
        let y = (glyph.uv.min[1] * atlas.height as f32).round() as u32;
        for row in 0..HEIGHT {
            for column in 0..WIDTH {
                let offset = rgba_offset(atlas.width, x + column, y + row).unwrap();
                let expected = if column == 0 || column == 4 || row == 0 || row == 7 {
                    [255; 4]
                } else {
                    [0; 4]
                };
                assert_eq!(atlas.rgba[offset..offset + 4], expected);
            }
        }
        assert_eq!(
            glyph.uv.min,
            [
                (x as f32 + 0.01) / atlas.width as f32,
                (y as f32 + 0.01) / atlas.height as f32
            ]
        );
        assert_eq!(
            glyph.uv.max,
            [
                (x as f32 - 0.01 + 5.0) / atlas.width as f32,
                (y as f32 - 0.01 + 8.0) / atlas.height as f32
            ]
        );
        assert_eq!(atlas.glyphs.resolve('缺'), glyph);
        assert_eq!(atlas.fonts.glyphs(Some("unknown:font")).resolve('?'), glyph);
        assert_eq!(atlas.rgba.len(), (atlas.width * atlas.height * 4) as usize);
    }

    #[test]
    fn missing_atlas_supports_empty_and_space_only_fonts_without_codepoint_aliases() {
        let fonts = vec![
            (DEFAULT_FONT_ID.to_string(), vec![]),
            (
                "example:space".to_string(),
                vec![FontAtlasEntry::Space(FontSpaceProviderDefinition {
                    advances: vec![(u32::from(' '), -2.5), (u32::from('?'), 0.5)],
                })],
            ),
        ];
        let atlas = build_hud_font_book_atlas(&fonts).unwrap();
        assert_eq!((atlas.width, atlas.height), (5, 8));
        assert!(atlas.glyphs.is_available());
        assert!(atlas.glyphs.is_empty());
        assert_eq!(atlas.glyphs.get('?'), None);
        let spaces = atlas.fonts.glyphs(Some("example:space"));
        assert_eq!(spaces.get(' ').unwrap().advance, -2.5);
        assert_eq!(spaces.get('?').unwrap().advance, 0.5);
        assert_eq!(spaces.missing_glyph(), atlas.glyphs.missing_glyph());
        assert_eq!(spaces.len(), 2);
        check_missing(&atlas);
    }

    fn bitmap(width: u32, height: u32) -> FontAtlasEntry {
        FontAtlasEntry::Bitmap(
            FontBitmapProviderDefinition {
                file: "example:font/test.png".to_string(),
                height: 8,
                ascent: 7,
                chars: vec![vec![u32::from('?')]],
            },
            Arc::new(
                SpriteImage::new(
                    "test",
                    width,
                    height,
                    vec![255; (width * height * 4) as usize],
                )
                .unwrap(),
            ),
        )
    }

    #[test]
    fn missing_atlas_rebuilds_uvs_and_preserves_real_question_mark() {
        let small = build_hud_font_atlas(&[bitmap(8, 8)]).unwrap();
        let wide = build_hud_font_atlas(&[bitmap(16, 8)]).unwrap();
        check_missing(&small);
        check_missing(&wide);
        assert_ne!(
            small.glyphs.missing_glyph().unwrap().uv,
            wide.glyphs.missing_glyph().unwrap().uv
        );
        for atlas in [&small, &wide] {
            assert_eq!(atlas.glyphs.len(), 1);
            assert_ne!(atlas.glyphs.resolve('?'), atlas.glyphs.resolve('缺'));
            assert_eq!(atlas.glyphs.resolve('?'), atlas.glyphs.get('?').unwrap());
        }
    }

    #[test]
    fn missing_atlas_reserves_budget_and_wraps_at_maximum_texture_width() {
        let fonts = vec![(DEFAULT_FONT_ID.to_string(), vec![bitmap(8192, 1)])];
        let (width, height, placements) = layout_unique_static_pages(&fonts, None).unwrap();
        assert_eq!((width, height), (8192, 9));
        let missing = placements
            .iter()
            .find(|placement| same_static_page(placement.entry, &MISSING_PAGE))
            .unwrap();
        assert_eq!((missing.x, missing.y), (0, 1));
        let too_tall = vec![(DEFAULT_FONT_ID.to_string(), vec![bitmap(5, 8192)])];
        assert!(layout_unique_static_pages(&too_tall, None)
            .err()
            .unwrap()
            .to_string()
            .contains("8192"));
        let too_wide = vec![(DEFAULT_FONT_ID.to_string(), vec![bitmap(8193, 1)])];
        assert!(layout_unique_static_pages(&too_wide, None)
            .err()
            .unwrap()
            .to_string()
            .contains("8192"));
    }
}
