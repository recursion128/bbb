//! Vanilla font consumption for HUD and map text: parses the
//! `font/default.json` provider chain and bakes a codepoint-keyed multi-page
//! glyph atlas (`BitmapProvider` semantics), plus the `font/ascii.png` digit
//! strip used by item count labels.

pub mod providers;

use anyhow::{bail, Context, Result};
use bbb_pack::{PackRoots, ResourceLocation, SpriteImage};
use bbb_render_types::{HudDigitGlyph, HudFontGlyphMap, HudUvRect};

use providers::FontBitmapProviderDefinition;

/// The font consumed by every current text consumer (HUD labels, tooltips,
/// map decoration names): vanilla `Style.DEFAULT_FONT` `minecraft:default`.
pub const DEFAULT_FONT_ID: &str = "minecraft:default";

/// `font/include/space.json` pins `" ": 4` through a `space` provider listed
/// ahead of the bitmap pages in `font/default.json`. The space provider type
/// is a follow-up slice, so the blank `font/ascii.png` space cell keeps this
/// hardcoded vanilla advance for now.
const SPACE_ADVANCE: u32 = 4;

const ASCII_FONT_GRID_COLUMNS: u32 = 16;
const ASCII_FONT_GRID_ROWS: u32 = 16;
const ASCII_DIGIT_ROW: u32 = 3;
const HUD_DIGIT_COUNT: u32 = 10;

#[derive(Debug, Clone, PartialEq)]
pub struct HudDigitAtlasImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub glyphs: [HudDigitGlyph; 10],
}

/// The baked `minecraft:default` font: every bitmap provider page blitted
/// into one RGBA atlas (pages stacked top to bottom in provider order, each
/// keeping its original grid layout) plus the codepoint-keyed glyph table.
#[derive(Debug, Clone, PartialEq)]
pub struct HudFontAtlasImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub glyphs: HudFontGlyphMap,
}

/// Loads the `minecraft:default` bitmap provider chain and bakes the
/// multi-page glyph atlas.
pub fn load_hud_font_atlas(roots: &PackRoots) -> Result<HudFontAtlasImage> {
    let definitions = providers::load_font_bitmap_providers(roots, DEFAULT_FONT_ID)?;
    if definitions.is_empty() {
        bail!("font {DEFAULT_FONT_ID} resolved no bitmap providers");
    }
    let pages = definitions
        .into_iter()
        .map(|definition| {
            let image = load_font_bitmap_texture(roots, &definition.file)?;
            Ok((definition, image))
        })
        .collect::<Result<Vec<_>>>()?;
    build_hud_font_atlas(&pages)
}

pub fn load_ascii_font_texture(roots: &PackRoots) -> Result<SpriteImage> {
    let location = ResourceLocation::parse("textures/font/ascii.png")?;
    let resource = roots
        .resource_stack()
        .get_resource(&location)
        .context("missing ASCII font texture minecraft:textures/font/ascii.png")?;
    SpriteImage::from_png_file("minecraft:textures/font/ascii", resource.path)
}

/// Vanilla `BitmapProvider.Definition.load`: the provider `file` gains a
/// `textures/` prefix (`file.withPrefix("textures/")`) before resolution.
fn load_font_bitmap_texture(roots: &PackRoots, file: &str) -> Result<SpriteImage> {
    let location = ResourceLocation::parse(file)?;
    let location = ResourceLocation::new(
        location.namespace(),
        format!("textures/{}", location.path()),
    )?;
    let resource = roots
        .resource_stack()
        .get_resource(&location)
        .with_context(|| format!("missing font bitmap texture {}", location.id()))?;
    SpriteImage::from_png_file(location.id(), resource.path)
}

fn build_hud_font_atlas(
    pages: &[(FontBitmapProviderDefinition, SpriteImage)],
) -> Result<HudFontAtlasImage> {
    let width = pages
        .iter()
        .map(|(_, image)| image.width)
        .max()
        .context("font atlas needs at least one bitmap page")?;
    let height = pages
        .iter()
        .try_fold(0u32, |height, (_, image)| height.checked_add(image.height))
        .context("font atlas height overflow")?;
    if width == 0 || height == 0 {
        bail!("font bitmap pages must not be empty");
    }
    let mut rgba = vec![0; rgba_len(width, height)?];
    let mut glyphs = HudFontGlyphMap::new();

    let mut page_y = 0u32;
    for (definition, image) in pages {
        blit_font_page(image, &mut rgba, width, page_y)?;
        collect_page_glyphs(definition, image, width, height, page_y, &mut glyphs)?;
        page_y += image.height;
    }

    Ok(HudFontAtlasImage {
        width,
        height,
        rgba,
        glyphs,
    })
}

fn blit_font_page(image: &SpriteImage, dst: &mut [u8], dst_width: u32, dst_y: u32) -> Result<()> {
    let row_len = usize::try_from(image.width)
        .ok()
        .and_then(|width| width.checked_mul(4))
        .context("font page row length overflow")?;
    for y in 0..image.height {
        let src_offset = rgba_offset(image.width, 0, y)?;
        let dst_offset = rgba_offset(dst_width, 0, dst_y + y)?;
        let src = image
            .rgba
            .get(src_offset..src_offset + row_len)
            .context("font page source row is outside image")?;
        dst.get_mut(dst_offset..dst_offset + row_len)
            .context("font atlas destination row is outside image")?
            .copy_from_slice(src);
    }
    Ok(())
}

fn collect_page_glyphs(
    definition: &FontBitmapProviderDefinition,
    image: &SpriteImage,
    atlas_width: u32,
    atlas_height: u32,
    page_y: u32,
    glyphs: &mut HudFontGlyphMap,
) -> Result<()> {
    // Vanilla BitmapProvider.Definition.load: integer cell size from the image
    // dimensions and the codepoint grid shape.
    let columns = u32::try_from(definition.chars[0].len()).context("font grid column overflow")?;
    let rows = u32::try_from(definition.chars.len()).context("font grid row overflow")?;
    let glyph_width = image.width / columns;
    let glyph_height = image.height / rows;
    if glyph_width == 0 || glyph_height == 0 {
        bail!(
            "font bitmap page {} is smaller than its {}x{} codepoint grid",
            definition.file,
            columns,
            rows
        );
    }
    if definition.height <= 0 {
        bail!(
            "font bitmap page {} has non-positive height {}",
            definition.file,
            definition.height
        );
    }
    // `pixelScale = (float)height / glyphHeight` (BitmapProvider.load).
    let pixel_scale = definition.height as f32 / glyph_height as f32;

    // Within one page a duplicated codepoint replaces the earlier slot
    // (vanilla `CodepointMap.put` overwrites and warns); across pages the
    // first provider wins, so pages merge through `insert_first_wins`.
    let mut page_glyphs: Vec<(char, HudDigitGlyph)> = Vec::new();
    for (row_index, row) in definition.chars.iter().enumerate() {
        for (column_index, &codepoint) in row.iter().enumerate() {
            // Vanilla skips `\u0000` grid slots (`if (c != 0)`).
            if codepoint == '\0' {
                continue;
            }
            let src_x =
                u32::try_from(column_index).context("font grid column overflow")? * glyph_width;
            let src_y = u32::try_from(row_index).context("font grid row overflow")? * glyph_height;
            let advance = if codepoint == ' ' {
                SPACE_ADVANCE
            } else {
                let actual_width =
                    glyph_actual_width(image, src_x, src_y, glyph_width, glyph_height);
                // `(int)(0.5 + actualGlyphWidth * pixelScale) + 1` (BitmapProvider.load).
                (0.5 + actual_width as f32 * pixel_scale) as u32 + 1
            };
            let glyph = HudDigitGlyph {
                uv: HudUvRect {
                    min: [
                        src_x as f32 / atlas_width as f32,
                        (page_y + src_y) as f32 / atlas_height as f32,
                    ],
                    max: [
                        (src_x + glyph_width) as f32 / atlas_width as f32,
                        (page_y + src_y + glyph_height) as f32 / atlas_height as f32,
                    ],
                },
                // Rendered size in font pixels: cell size divided by the
                // oversample, i.e. `cell * pixelScale` (GlyphBitmap.getRight /
                // getBottom); the height lands exactly on the provider height.
                width: (glyph_width as f32 * pixel_scale).round() as u32,
                height: (glyph_height as f32 * pixel_scale).round() as u32,
                advance,
                // GlyphBitmap.getBearingTop() returns the provider ascent.
                ascent: definition.ascent,
            };
            if let Some(slot) = page_glyphs
                .iter_mut()
                .find(|(existing, _)| *existing == codepoint)
            {
                slot.1 = glyph;
            } else {
                page_glyphs.push((codepoint, glyph));
            }
        }
    }
    for (codepoint, glyph) in page_glyphs {
        glyphs.insert_first_wins(codepoint, glyph);
    }
    Ok(())
}

pub fn hud_ascii_digit_atlas_from_image(image: &SpriteImage) -> Result<HudDigitAtlasImage> {
    let glyph_width = image.width / ASCII_FONT_GRID_COLUMNS;
    let glyph_height = image.height / ASCII_FONT_GRID_ROWS;
    if glyph_width == 0 || glyph_height == 0 {
        bail!("ascii font texture must contain a non-empty 16x16 glyph grid");
    }

    let width = HUD_DIGIT_COUNT * glyph_width;
    let height = glyph_height;
    let mut rgba = vec![0; rgba_len(width, height)?];
    let mut glyphs = [HudDigitGlyph::default(); 10];

    for digit in 0..HUD_DIGIT_COUNT {
        let src_x = digit * glyph_width;
        let src_y = ASCII_DIGIT_ROW * glyph_height;
        let dst_x = digit * glyph_width;
        copy_ascii_glyph(
            image,
            &mut rgba,
            width,
            dst_x,
            src_x,
            src_y,
            glyph_width,
            glyph_height,
        )?;
        let advance = glyph_actual_width(image, src_x, src_y, glyph_width, glyph_height) + 1;
        glyphs[digit as usize] = HudDigitGlyph {
            uv: HudUvRect {
                min: [dst_x as f32 / width as f32, 0.0],
                max: [(dst_x + glyph_width) as f32 / width as f32, 1.0],
            },
            width: glyph_width,
            height: glyph_height,
            advance,
            ..HudDigitGlyph::default()
        };
    }

    Ok(HudDigitAtlasImage {
        width,
        height,
        rgba,
        glyphs,
    })
}

fn copy_ascii_glyph(
    image: &SpriteImage,
    dst: &mut [u8],
    dst_width: u32,
    dst_x: u32,
    src_x: u32,
    src_y: u32,
    glyph_width: u32,
    glyph_height: u32,
) -> Result<()> {
    for y in 0..glyph_height {
        for x in 0..glyph_width {
            let src_offset = rgba_offset(image.width, src_x + x, src_y + y)?;
            let dst_offset = rgba_offset(dst_width, dst_x + x, y)?;
            copy_rgba_pixel(&image.rgba, src_offset, dst, dst_offset)?;
        }
    }
    Ok(())
}

fn copy_rgba_pixel(
    src_rgba: &[u8],
    src_offset: usize,
    dst_rgba: &mut [u8],
    dst_offset: usize,
) -> Result<()> {
    let src_end = src_offset
        .checked_add(4)
        .filter(|end| *end <= src_rgba.len())
        .context("ascii font source pixel is outside image")?;
    let dst_end = dst_offset
        .checked_add(4)
        .filter(|end| *end <= dst_rgba.len())
        .context("ascii atlas destination pixel is outside image")?;
    dst_rgba[dst_offset..dst_end].copy_from_slice(&src_rgba[src_offset..src_end]);
    Ok(())
}

/// Vanilla `BitmapProvider.Definition.getActualGlyphWidth`: scan cell columns
/// right to left for the first column with a visible pixel.
fn glyph_actual_width(
    image: &SpriteImage,
    src_x: u32,
    src_y: u32,
    glyph_width: u32,
    glyph_height: u32,
) -> u32 {
    for x in (0..glyph_width).rev() {
        for y in 0..glyph_height {
            if font_pixel_visible(image, src_x + x, src_y + y) {
                return x + 1;
            }
        }
    }
    0
}

/// Vanilla `NativeImage.getLuminanceOrAlpha` on the RGBA-decoded font pages
/// reads only the alpha byte. The vanilla font PNGs are white-palette images
/// whose transparent pixels decode to `(255, 255, 255, 0)`, so an
/// "alpha or RGB non-zero" check would see every pixel as visible and inflate
/// every advance to the full cell width.
fn font_pixel_visible(image: &SpriteImage, x: u32, y: u32) -> bool {
    rgba_offset(image.width, x, y)
        .ok()
        .and_then(|offset| image.rgba.get(offset..offset + 4))
        .is_some_and(|pixel| pixel[3] != 0)
}

fn rgba_len(width: u32, height: u32) -> Result<usize> {
    usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .context("atlas RGBA size overflow")
}

fn rgba_offset(width: u32, x: u32, y: u32) -> Result<usize> {
    y.checked_mul(width)
        .and_then(|row| row.checked_add(x))
        .and_then(|pixel| pixel.checked_mul(4))
        .and_then(|offset| usize::try_from(offset).ok())
        .context("RGBA pixel offset overflow")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page_image(id: &str, width: u32, height: u32, pixels: &[(u32, u32)]) -> SpriteImage {
        let mut rgba = vec![0; rgba_len(width, height).unwrap()];
        for &(x, y) in pixels {
            let offset = rgba_offset(width, x, y).unwrap();
            rgba[offset..offset + 4].copy_from_slice(&[255, 255, 255, 255]);
        }
        SpriteImage::new(id, width, height, rgba).unwrap()
    }

    fn definition(
        file: &str,
        height: i32,
        ascent: i32,
        chars: &[&str],
    ) -> FontBitmapProviderDefinition {
        FontBitmapProviderDefinition {
            file: file.to_string(),
            height,
            ascent,
            chars: chars.iter().map(|row| row.chars().collect()).collect(),
        }
    }

    #[test]
    fn bitmap_advance_uses_vanilla_pixel_scale_formula() {
        // A height-12 provider over 24px cells scales by 12/24 = 0.5
        // (BitmapProvider.load pixelScale); an 11px-wide glyph advances
        // (int)(0.5 + 11 * 0.5) + 1 = 7.
        let mut pixels = Vec::new();
        for y in 0..24 {
            pixels.push((10, y));
        }
        let image = page_image("page", 24, 24, &pixels);
        let atlas =
            build_hud_font_atlas(&[(definition("minecraft:font/big.png", 12, 10, &["é"]), image)])
                .unwrap();

        let glyph = atlas.glyphs.get('é').expect("é glyph");
        assert_eq!(glyph.advance, 7);
        // Rendered size is the cell scaled by pixelScale: 24 * 0.5 = 12.
        assert_eq!(glyph.width, 12);
        assert_eq!(glyph.height, 12);
        assert_eq!(glyph.ascent, 10);
    }

    #[test]
    fn fallback_prefers_first_provider_page() {
        // include/default order: nonlatin_european before ascii; a codepoint
        // present on both pages resolves from the first page
        // (FontSet.computeGlyphInfo first-provider-wins).
        let first = page_image("first", 8, 8, &[(0, 0), (3, 4)]);
        let second = page_image("second", 8, 8, &[(0, 0), (6, 2)]);
        let atlas = build_hud_font_atlas(&[
            (definition("minecraft:font/first.png", 8, 7, &["λ"]), first),
            (
                definition("minecraft:font/second.png", 8, 7, &["λ"]),
                second,
            ),
        ])
        .unwrap();

        let glyph = atlas.glyphs.get('λ').expect("λ glyph");
        // First page occupies atlas rows 0..8 of the 8x16 stacked atlas.
        assert_eq!(glyph.uv.min, [0.0, 0.0]);
        assert_eq!(glyph.uv.max, [1.0, 0.5]);
        // Advance from the first page's 4px actual width, not the second's 7px.
        assert_eq!(glyph.advance, 5);
    }

    #[test]
    fn pages_stack_vertically_and_null_slots_are_skipped() {
        let ascii_like = page_image("ascii", 16, 8, &[(1, 1), (8 + 2, 3)]);
        let accented_like = page_image("accented", 18, 12, &[(4, 11)]);
        let atlas = build_hud_font_atlas(&[
            (
                definition("minecraft:font/ascii.png", 8, 7, &["A\u{0}"]),
                ascii_like,
            ),
            (
                definition("minecraft:font/accented.png", 12, 10, &["é\u{0}"]),
                accented_like,
            ),
        ])
        .unwrap();

        assert_eq!(atlas.width, 18);
        assert_eq!(atlas.height, 20);
        // The `\u{0}` cells produce no glyphs (vanilla skips them).
        assert_eq!(atlas.glyphs.len(), 2);

        let a = atlas.glyphs.get('A').expect("A glyph");
        assert_eq!(a.uv.min, [0.0, 0.0]);
        assert_eq!(a.uv.max, [8.0 / 18.0, 8.0 / 20.0]);
        assert_eq!((a.width, a.height, a.ascent), (8, 8, 7));

        let e = atlas.glyphs.get('é').expect("é glyph");
        assert_eq!(e.uv.min, [0.0, 8.0 / 20.0]);
        assert_eq!(e.uv.max, [9.0 / 18.0, 1.0]);
        assert_eq!((e.width, e.height, e.ascent), (9, 12, 10));

        // The ascii page pixel (1,1) lands at atlas (1,1); the accented page
        // pixel (4,11) lands below the first page at (4, 8 + 11).
        let first = rgba_offset(18, 1, 1).unwrap();
        assert_eq!(&atlas.rgba[first..first + 4], &[255, 255, 255, 255]);
        let second = rgba_offset(18, 4, 19).unwrap();
        assert_eq!(&atlas.rgba[second..second + 4], &[255, 255, 255, 255]);
    }

    #[test]
    fn european_codepoints_resolve_with_vanilla_page_metrics() {
        // Vanilla include/default.json shape: nonlatin_european (ascent 7)
        // carries ж and λ, accented (height 12, ascent 10) carries é/ü/ñ,
        // ascii (ascent 7) carries e. CJK stays unmapped (unifont deferred).
        let nonlatin = page_image("nonlatin", 16, 8, &[(4, 2), (8 + 5, 3)]);
        let accented = page_image("accented", 27, 12, &[(3, 1), (9 + 4, 5), (18 + 2, 6)]);
        let ascii = page_image("ascii", 8, 8, &[(3, 4)]);
        let atlas = build_hud_font_atlas(&[
            (
                definition("minecraft:font/nonlatin_european.png", 8, 7, &["жλ"]),
                nonlatin,
            ),
            (
                definition("minecraft:font/accented.png", 12, 10, &["éüñ"]),
                accented,
            ),
            (definition("minecraft:font/ascii.png", 8, 7, &["e"]), ascii),
        ])
        .unwrap();

        for (codepoint, advance, ascent) in [
            ('ж', 6, 7),
            ('λ', 7, 7),
            ('é', 5, 10),
            ('ü', 6, 10),
            ('ñ', 4, 10),
            ('e', 5, 7),
        ] {
            let glyph = atlas
                .glyphs
                .get(codepoint)
                .unwrap_or_else(|| panic!("{codepoint} should have a glyph"));
            assert_eq!(glyph.advance, advance, "{codepoint} advance");
            assert_eq!(glyph.ascent, ascent, "{codepoint} ascent");
        }
        // é (ascent 10) hangs 3px above e (ascent 7) when baselines align.
        let e = atlas.glyphs.get('e').unwrap();
        let e_accent = atlas.glyphs.get('é').unwrap();
        assert_eq!(e.baseline_offset(), 0.0);
        assert_eq!(e_accent.baseline_offset(), -3.0);
        assert!(atlas.glyphs.get('钻').is_none());
    }

    #[test]
    fn space_keeps_hardcoded_space_provider_advance() {
        // font/include/space.json: `" ": 4` (space provider slice deferred).
        let image = page_image("ascii", 16, 8, &[(8 + 7, 0)]);
        let atlas =
            build_hud_font_atlas(&[(definition("minecraft:font/ascii.png", 8, 7, &[" ~"]), image)])
                .unwrap();

        assert_eq!(atlas.glyphs.get(' ').unwrap().advance, SPACE_ADVANCE);
        assert_eq!(atlas.glyphs.get('~').unwrap().advance, 9);
    }

    #[test]
    fn transparent_white_pixels_are_invisible_to_the_advance_scan() {
        // The vanilla font PNGs are white-palette images whose transparent
        // pixels decode to (255,255,255,0); `getLuminanceOrAlpha` reads only
        // alpha, so they must not count toward the actual glyph width.
        let mut rgba = vec![255u8; rgba_len(8, 8).unwrap()];
        for pixel in rgba.chunks_exact_mut(4) {
            pixel[3] = 0;
        }
        let offset = rgba_offset(8, 2, 3).unwrap();
        rgba[offset + 3] = 255;
        let image = SpriteImage::new("page", 8, 8, rgba).unwrap();
        let atlas =
            build_hud_font_atlas(&[(definition("minecraft:font/page.png", 8, 7, &["A"]), image)])
                .unwrap();

        assert_eq!(atlas.glyphs.get('A').unwrap().advance, 4);
    }

    #[test]
    fn duplicate_codepoint_within_a_page_keeps_the_last_slot() {
        // Vanilla CodepointMap.put overwrites earlier slots within one
        // provider (and only warns).
        let image = page_image("page", 16, 8, &[(2, 0), (8 + 5, 0)]);
        let atlas =
            build_hud_font_atlas(&[(definition("minecraft:font/page.png", 8, 7, &["AA"]), image)])
                .unwrap();

        let glyph = atlas.glyphs.get('A').expect("A glyph");
        assert_eq!(glyph.advance, 7);
        assert_eq!(glyph.uv.min, [0.5, 0.0]);
    }
}
