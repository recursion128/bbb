//! Vanilla font consumption for HUD and map text: parses the
//! `font/default.json` provider chain and bakes a codepoint-keyed multi-page
//! glyph atlas (`BitmapProvider` semantics), plus the `font/ascii.png` digit
//! strip used by item count labels.

pub mod providers;
mod unihex;

use std::{collections::BTreeMap, sync::Arc};

use anyhow::{bail, Context, Result};
use bbb_pack::{PackRoots, ResourceLocation, SpriteImage};
use bbb_render_types::{HudDigitGlyph, HudFontGlyphBook, HudFontGlyphMap, HudUvRect};

use providers::{
    FontBitmapProviderDefinition, FontProviderDefinition, FontSpaceProviderDefinition,
    FontUnihexProviderDefinition,
};
use unihex::UnihexAtlasPage;

/// Vanilla `Style.DEFAULT_FONT`, also used when a Component does not select a
/// loaded custom font.
pub const DEFAULT_FONT_ID: &str = "minecraft:default";

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

/// Every selected resource-pack font baked into one RGBA atlas, plus the
/// default glyph table retained for HUD consumers and the per-font book used
/// by world Component text.
#[derive(Debug, Clone, PartialEq)]
pub struct HudFontAtlasImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub glyphs: HudFontGlyphMap,
    pub fonts: HudFontGlyphBook,
}

/// One flattened provider ready for atlas baking: bitmap providers carry
/// their loaded page image, space providers carry only their advances (they
/// contribute no atlas pixels, mirroring vanilla `EmptyGlyph`).
#[derive(Clone)]
enum FontAtlasEntry {
    Bitmap(FontBitmapProviderDefinition, Arc<SpriteImage>),
    Space(FontSpaceProviderDefinition),
    Unihex(Arc<UnihexAtlasPage>),
}

#[derive(Default)]
struct FontAtlasLoadCache {
    bitmaps: Vec<(FontBitmapProviderDefinition, Arc<SpriteImage>)>,
    unihex: Vec<(FontUnihexProviderDefinition, Arc<UnihexAtlasPage>)>,
}

/// Loads every selected `font/*.json` definition and bakes all bitmap pages
/// into one shared atlas. Per-font glyph maps keep their own first-provider-wins
/// semantics while their UVs address that shared image, so Component font
/// changes do not require per-run texture binds.
pub fn load_hud_font_atlas(roots: &PackRoots) -> Result<HudFontAtlasImage> {
    let stack = roots.resource_stack();
    let mut fonts = Vec::new();
    let mut cache = FontAtlasLoadCache::default();
    for resource in stack.list_resources("font", ".json")? {
        let Some(path) = resource
            .location
            .path()
            .strip_prefix("font/")
            .and_then(|path| path.strip_suffix(".json"))
        else {
            continue;
        };
        let font_id = format!("{}:{path}", resource.location.namespace());
        let definitions = providers::load_font_providers(roots, &font_id)?;
        let entries = load_font_atlas_entries(roots, definitions, &mut cache)?;
        fonts.push((font_id, entries));
    }
    if !fonts.iter().any(|(font_id, _)| font_id == DEFAULT_FONT_ID) {
        bail!("font {DEFAULT_FONT_ID} has no selected definition");
    }
    build_hud_font_book_atlas(&fonts)
}

fn load_font_atlas_entries(
    roots: &PackRoots,
    definitions: Vec<FontProviderDefinition>,
    cache: &mut FontAtlasLoadCache,
) -> Result<Vec<FontAtlasEntry>> {
    definitions
        .into_iter()
        .map(|definition| {
            Ok(match definition {
                FontProviderDefinition::Bitmap(bitmap) => {
                    let image = if let Some((_, image)) = cache
                        .bitmaps
                        .iter()
                        .find(|(existing, _)| existing == &bitmap)
                    {
                        Arc::clone(image)
                    } else {
                        let image = Arc::new(load_font_bitmap_texture(roots, &bitmap.file)?);
                        cache.bitmaps.push((bitmap.clone(), Arc::clone(&image)));
                        image
                    };
                    FontAtlasEntry::Bitmap(bitmap, image)
                }
                FontProviderDefinition::Space(space) => FontAtlasEntry::Space(space),
                FontProviderDefinition::Unihex(unihex) => {
                    let page = if let Some((_, page)) = cache
                        .unihex
                        .iter()
                        .find(|(existing, _)| existing == &unihex)
                    {
                        Arc::clone(page)
                    } else {
                        let page = Arc::new(unihex::load_unihex_atlas_page(roots, &unihex)?);
                        cache.unihex.push((unihex.clone(), Arc::clone(&page)));
                        page
                    };
                    FontAtlasEntry::Unihex(page)
                }
            })
        })
        .collect()
}

pub fn load_ascii_font_texture(roots: &PackRoots) -> Result<SpriteImage> {
    let location = ResourceLocation::parse("textures/font/ascii.png")?;
    let resources = roots.resource_stack();
    let resource = resources
        .get_resource(&location)
        .context("missing ASCII font texture minecraft:textures/font/ascii.png")?;
    resources.validate_resource(&resource, "ASCII font texture")?;
    resources.validate_resource_metadata(&resource, "ASCII font texture metadata")?;
    SpriteImage::from_png_file_with_resource_stack(
        "minecraft:textures/font/ascii",
        resource.path,
        &resources,
    )
}

/// Vanilla `BitmapProvider.Definition.load`: the provider `file` gains a
/// `textures/` prefix (`file.withPrefix("textures/")`) before resolution.
fn load_font_bitmap_texture(roots: &PackRoots, file: &str) -> Result<SpriteImage> {
    let location = ResourceLocation::parse(file)?;
    let location = ResourceLocation::new(
        location.namespace(),
        format!("textures/{}", location.path()),
    )?;
    let resources = roots.resource_stack();
    let resource = resources
        .get_resource(&location)
        .with_context(|| format!("missing font bitmap texture {}", location.id()))?;
    resources.validate_resource(&resource, "font bitmap texture")?;
    resources.validate_resource_metadata(&resource, "font bitmap texture metadata")?;
    SpriteImage::from_png_file_with_resource_stack(location.id(), resource.path, &resources)
}

#[cfg(test)]
fn build_hud_font_atlas(entries: &[FontAtlasEntry]) -> Result<HudFontAtlasImage> {
    build_hud_font_book_atlas(&[(DEFAULT_FONT_ID.to_string(), entries.to_vec())])
}

fn build_hud_font_book_atlas(fonts: &[(String, Vec<FontAtlasEntry>)]) -> Result<HudFontAtlasImage> {
    let (width, height, placements) = layout_unique_font_pages(fonts)?;
    let mut rgba = vec![0; rgba_len(width, height)?];
    for placement in &placements {
        match placement.entry {
            FontAtlasEntry::Bitmap(_, image) => {
                blit_font_page(image, &mut rgba, width, placement.x, placement.y)?;
            }
            FontAtlasEntry::Unihex(page) => {
                blit_rgba_page(
                    page.width,
                    page.height,
                    &page.rgba,
                    &mut rgba,
                    width,
                    placement.x,
                    placement.y,
                )?;
            }
            FontAtlasEntry::Space(_) => unreachable!("space provider has no raster page"),
        }
    }
    let mut maps = BTreeMap::new();
    for (font_id, entries) in fonts {
        let mut glyphs = HudFontGlyphMap::new();
        for entry in entries {
            match entry {
                FontAtlasEntry::Bitmap(definition, image) => {
                    let placement = placement_for(entry, &placements)
                        .context("font bitmap page has no atlas placement")?;
                    collect_page_glyphs(
                        definition,
                        image,
                        width,
                        height,
                        placement.x,
                        placement.y,
                        &mut glyphs,
                    )?;
                }
                FontAtlasEntry::Space(space) => collect_space_glyphs(space, &mut glyphs),
                FontAtlasEntry::Unihex(page) if page.glyphs.is_empty() => {}
                FontAtlasEntry::Unihex(page) => {
                    let placement = placement_for(entry, &placements)
                        .context("unihex page has no atlas placement")?;
                    collect_unihex_glyphs(
                        page,
                        width,
                        height,
                        placement.x,
                        placement.y,
                        &mut glyphs,
                    );
                }
            }
        }
        maps.insert(font_id.clone(), glyphs);
    }
    let glyphs = maps
        .get(DEFAULT_FONT_ID)
        .cloned()
        .context("default font glyph map was not built")?;
    let font_book = HudFontGlyphBook::from_fonts(DEFAULT_FONT_ID, maps);
    Ok(HudFontAtlasImage {
        width,
        height,
        rgba,
        glyphs,
        fonts: font_book,
    })
}

struct FontPagePlacement<'a> {
    entry: &'a FontAtlasEntry,
    x: u32,
    y: u32,
}

fn layout_unique_font_pages(
    fonts: &[(String, Vec<FontAtlasEntry>)],
) -> Result<(u32, u32, Vec<FontPagePlacement<'_>>)> {
    let mut unique = Vec::<&FontAtlasEntry>::new();
    for entry in fonts.iter().flat_map(|(_, entries)| entries) {
        if entry_page_dimensions(entry).is_some()
            && !unique
                .iter()
                .any(|existing| same_raster_page(existing, entry))
        {
            unique.push(entry);
        }
    }
    let width = unique
        .iter()
        .filter_map(|entry| entry_page_dimensions(entry).map(|(width, _)| width))
        .max()
        .context("font atlas needs at least one raster page")?;
    if width > 8_192 {
        bail!("font atlas width {width} exceeds the 8192 pixel texture limit");
    }
    let mut placements = Vec::with_capacity(unique.len());
    let mut x = 0_u32;
    let mut y = 0_u32;
    let mut row_height = 0_u32;
    for entry in unique {
        let (page_width, page_height) = entry_page_dimensions(entry).unwrap();
        if x > 0 && x + page_width > width {
            x = 0;
            y = y
                .checked_add(row_height)
                .context("font atlas height overflow")?;
            row_height = 0;
        }
        placements.push(FontPagePlacement { entry, x, y });
        x = x
            .checked_add(page_width)
            .context("font atlas row overflow")?;
        row_height = row_height.max(page_height);
    }
    let height = y
        .checked_add(row_height)
        .context("font atlas height overflow")?;
    if height == 0 || height > 8_192 {
        bail!("font atlas height {height} is outside the 1..=8192 pixel limit");
    }
    Ok((width, height, placements))
}

fn entry_page_dimensions(entry: &FontAtlasEntry) -> Option<(u32, u32)> {
    match entry {
        FontAtlasEntry::Bitmap(_, image) if image.width > 0 && image.height > 0 => {
            Some((image.width, image.height))
        }
        FontAtlasEntry::Unihex(page) if page.width > 0 && page.height > 0 => {
            Some((page.width, page.height))
        }
        FontAtlasEntry::Bitmap(_, _) | FontAtlasEntry::Space(_) | FontAtlasEntry::Unihex(_) => None,
    }
}

fn same_raster_page(left: &FontAtlasEntry, right: &FontAtlasEntry) -> bool {
    match (left, right) {
        (FontAtlasEntry::Bitmap(_, left), FontAtlasEntry::Bitmap(_, right)) => {
            Arc::ptr_eq(left, right)
        }
        (FontAtlasEntry::Unihex(left), FontAtlasEntry::Unihex(right)) => Arc::ptr_eq(left, right),
        _ => false,
    }
}

fn placement_for<'a>(
    entry: &FontAtlasEntry,
    placements: &'a [FontPagePlacement<'a>],
) -> Option<&'a FontPagePlacement<'a>> {
    placements
        .iter()
        .find(|placement| same_raster_page(entry, placement.entry))
}

fn blit_font_page(
    image: &SpriteImage,
    dst: &mut [u8],
    dst_width: u32,
    dst_x: u32,
    dst_y: u32,
) -> Result<()> {
    blit_rgba_page(
        image.width,
        image.height,
        &image.rgba,
        dst,
        dst_width,
        dst_x,
        dst_y,
    )
}

fn blit_rgba_page(
    src_width: u32,
    src_height: u32,
    src_rgba: &[u8],
    dst: &mut [u8],
    dst_width: u32,
    dst_x: u32,
    dst_y: u32,
) -> Result<()> {
    let row_len = usize::try_from(src_width)
        .ok()
        .and_then(|width| width.checked_mul(4))
        .context("font page row length overflow")?;
    for y in 0..src_height {
        let src_offset = rgba_offset(src_width, 0, y)?;
        let dst_offset = rgba_offset(dst_width, dst_x, dst_y + y)?;
        let src = src_rgba
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
    page_x: u32,
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
            // Vanilla `BitmapProvider.load` computes every glyph's advance
            // (including the blank space cell) from the actual pixel scan;
            // the codepoint's real advance comes from a higher-priority
            // `space` provider instead (`FontSet.computeGlyphInfo`
            // first-provider-wins), inserted into the glyph map ahead of
            // this page's provider order.
            let actual_width = glyph_actual_width(image, src_x, src_y, glyph_width, glyph_height);
            // `(int)(0.5 + actualGlyphWidth * pixelScale) + 1` (BitmapProvider.load).
            let advance = (0.5 + actual_width as f32 * pixel_scale) as u32 + 1;
            let glyph = HudDigitGlyph {
                uv: HudUvRect {
                    min: [
                        (page_x + src_x) as f32 / atlas_width as f32,
                        (page_y + src_y) as f32 / atlas_height as f32,
                    ],
                    max: [
                        (page_x + src_x + glyph_width) as f32 / atlas_width as f32,
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
                ..HudDigitGlyph::default()
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

fn collect_unihex_glyphs(
    page: &UnihexAtlasPage,
    atlas_width: u32,
    atlas_height: u32,
    page_x: u32,
    page_y: u32,
    glyphs: &mut HudFontGlyphMap,
) {
    for glyph in &page.glyphs {
        glyphs.insert_first_wins(
            glyph.codepoint,
            HudDigitGlyph {
                uv: HudUvRect {
                    min: [
                        (page_x + glyph.x) as f32 / atlas_width as f32 + 0.01 / atlas_width as f32,
                        (page_y + glyph.y) as f32 / atlas_height as f32
                            + 0.01 / atlas_height as f32,
                    ],
                    max: [
                        (page_x + glyph.x + glyph.pixel_width) as f32 / atlas_width as f32
                            - 0.01 / atlas_width as f32,
                        (page_y + glyph.y + 16) as f32 / atlas_height as f32
                            - 0.01 / atlas_height as f32,
                    ],
                },
                width: glyph.pixel_width,
                height: 16,
                // `UnihexProvider.Glyph.info`: integer division happens
                // before widening to float.
                advance: glyph.pixel_width / 2 + 1,
                ascent: 7,
                render_scale: 0.5,
                bold_offset: 0.5,
                shadow_offset: 0.5,
            },
        );
    }
}

/// Vanilla `SpaceProvider`: every codepoint maps to an `EmptyGlyph`, which
/// carries only an advance and bakes to a `BakedGlyph` whose
/// `createGlyph` returns `null` (no quad, no pixels). The zero-size UV/`width`
/// `height` here rely on the draw loops' existing `width > 0 && height > 0`
/// guard (`bbb-renderer` `hud.rs`) to skip emitting a quad while the advance
/// still accumulates pen position.
fn collect_space_glyphs(definition: &FontSpaceProviderDefinition, glyphs: &mut HudFontGlyphMap) {
    for &(codepoint, advance) in &definition.advances {
        glyphs.insert_first_wins(
            codepoint,
            HudDigitGlyph {
                advance,
                ..HudDigitGlyph::default()
            },
        );
    }
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

    fn bitmap_entry(
        definition: FontBitmapProviderDefinition,
        image: SpriteImage,
    ) -> FontAtlasEntry {
        FontAtlasEntry::Bitmap(definition, Arc::new(image))
    }

    fn space_entry(advances: &[(char, u32)]) -> FontAtlasEntry {
        FontAtlasEntry::Space(FontSpaceProviderDefinition {
            advances: advances.to_vec(),
        })
    }

    fn unihex_entry(glyphs: &[(char, u32, u32)]) -> FontAtlasEntry {
        let width = glyphs
            .iter()
            .map(|(_, x, width)| x + width)
            .max()
            .unwrap_or(1);
        let mut rgba = vec![0; rgba_len(width, 16).unwrap()];
        let glyphs = glyphs
            .iter()
            .map(|&(codepoint, x, pixel_width)| {
                let offset = rgba_offset(width, x, 0).unwrap();
                rgba[offset..offset + 4].copy_from_slice(&[255; 4]);
                unihex::UnihexAtlasGlyph {
                    codepoint,
                    x,
                    y: 0,
                    pixel_width,
                }
            })
            .collect();
        FontAtlasEntry::Unihex(Arc::new(UnihexAtlasPage {
            width,
            height: 16,
            rgba,
            glyphs,
        }))
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
        let atlas = build_hud_font_atlas(&[bitmap_entry(
            definition("minecraft:font/big.png", 12, 10, &["é"]),
            image,
        )])
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
            bitmap_entry(definition("minecraft:font/first.png", 8, 7, &["λ"]), first),
            bitmap_entry(
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
    fn unihex_fallback_uses_provider_order_metrics_and_one_shared_raster_page() {
        let bitmap = bitmap_entry(
            definition("minecraft:font/ascii.png", 8, 7, &["A"]),
            page_image("ascii", 8, 8, &[(3, 0)]),
        );
        let unihex = unihex_entry(&[('A', 0, 8), ('中', 9, 16)]);
        let fonts = vec![
            (
                DEFAULT_FONT_ID.to_string(),
                vec![bitmap.clone(), unihex.clone()],
            ),
            ("minecraft:uniform".to_string(), vec![unihex]),
        ];
        let atlas = build_hud_font_book_atlas(&fonts).unwrap();

        // Default's higher bitmap page wins A; CJK falls through to unihex.
        assert_eq!(atlas.glyphs.get('A').unwrap().render_scale, 1.0);
        let cjk = atlas.glyphs.get('中').unwrap();
        assert_eq!((cjk.width, cjk.height, cjk.advance), (16, 16, 9));
        assert_eq!(cjk.rendered_width(), 8.0);
        assert_eq!((cjk.bold_offset, cjk.shadow_offset), (0.5, 0.5));
        // The cloned provider Arc is placed once even though two font maps
        // consume it. The 16-row unihex page plus the 8-row bitmap page is 24.
        assert_eq!(atlas.height, 24);
        assert_eq!(
            atlas.fonts.glyphs(Some("minecraft:uniform")).get('中'),
            Some(cjk)
        );
    }

    #[test]
    fn font_book_stacks_fonts_into_one_atlas_and_keeps_per_font_metrics() {
        let default = page_image("default", 8, 8, &[(2, 0)]);
        let custom = page_image("custom", 16, 8, &[(9, 0)]);
        let fonts = vec![
            (
                DEFAULT_FONT_ID.to_string(),
                vec![bitmap_entry(
                    definition("minecraft:font/default.png", 8, 7, &["A"]),
                    default,
                )],
            ),
            (
                "example:custom".to_string(),
                vec![bitmap_entry(
                    definition("example:font/custom.png", 8, 7, &["A"]),
                    custom,
                )],
            ),
        ];
        let atlas = build_hud_font_book_atlas(&fonts).unwrap();

        assert_eq!((atlas.width, atlas.height), (16, 16));
        assert_eq!(atlas.fonts.font_count(), 2);
        let default = atlas.fonts.glyphs(None).get('A').unwrap();
        let custom = atlas.fonts.glyphs(Some("example:custom")).get('A').unwrap();
        assert_eq!((default.advance, custom.advance), (4, 11));
        assert_eq!(
            default.uv,
            HudUvRect {
                min: [0.0, 0.0],
                max: [0.5, 0.5]
            }
        );
        assert_eq!(
            custom.uv,
            HudUvRect {
                min: [0.0, 0.5],
                max: [1.0, 1.0]
            }
        );
        assert_eq!(atlas.glyphs, *atlas.fonts.glyphs(None));
    }

    #[test]
    fn pages_stack_vertically_and_null_slots_are_skipped() {
        let ascii_like = page_image("ascii", 16, 8, &[(1, 1), (8 + 2, 3)]);
        let accented_like = page_image("accented", 18, 12, &[(4, 11)]);
        let atlas = build_hud_font_atlas(&[
            bitmap_entry(
                definition("minecraft:font/ascii.png", 8, 7, &["A\u{0}"]),
                ascii_like,
            ),
            bitmap_entry(
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
        // ascii (ascent 7) carries e. This bitmap-only fixture intentionally
        // has no unihex provider, so CJK remains unmapped here.
        let nonlatin = page_image("nonlatin", 16, 8, &[(4, 2), (8 + 5, 3)]);
        let accented = page_image("accented", 27, 12, &[(3, 1), (9 + 4, 5), (18 + 2, 6)]);
        let ascii = page_image("ascii", 8, 8, &[(3, 4)]);
        let atlas = build_hud_font_atlas(&[
            bitmap_entry(
                definition("minecraft:font/nonlatin_european.png", 8, 7, &["жλ"]),
                nonlatin,
            ),
            bitmap_entry(
                definition("minecraft:font/accented.png", 12, 10, &["éüñ"]),
                accented,
            ),
            bitmap_entry(definition("minecraft:font/ascii.png", 8, 7, &["e"]), ascii),
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
    fn space_provider_advance_wins_over_the_blank_bitmap_space_cell() {
        // font/include/space.json: `" ": 4`, listed ahead of the bitmap pages
        // in font/default.json (FontSet first-provider-wins). The ascii page
        // still carries a (blank) space grid cell, whose own actual-width
        // advance formula must lose to the space provider's.
        let image = page_image("ascii", 16, 8, &[(8 + 7, 0)]);
        let atlas = build_hud_font_atlas(&[
            space_entry(&[(' ', 4), ('\u{200c}', 0)]),
            bitmap_entry(definition("minecraft:font/ascii.png", 8, 7, &[" ~"]), image),
        ])
        .unwrap();

        let space = atlas.glyphs.get(' ').expect("space glyph");
        assert_eq!(space.advance, 4);
        // The space provider's `EmptyGlyph` carries no pixels.
        assert_eq!((space.width, space.height), (0, 0));
        assert_eq!(atlas.glyphs.get('~').unwrap().advance, 9);
    }

    #[test]
    fn space_provider_precedes_a_bitmap_page_with_its_own_space_cell() {
        // If a bitmap page also mapped ' ' to a real (non-blank) glyph cell,
        // the space provider (listed first) still wins the codepoint.
        let image = page_image("ascii", 16, 8, &[(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]);
        let atlas = build_hud_font_atlas(&[
            space_entry(&[(' ', 4)]),
            bitmap_entry(definition("minecraft:font/ascii.png", 8, 7, &[" "]), image),
        ])
        .unwrap();

        assert_eq!(atlas.glyphs.get(' ').unwrap().advance, 4);
    }

    #[test]
    fn zero_width_non_joiner_advances_zero_and_bakes_no_pixels() {
        // The atlas always needs at least one bitmap page for its pixel
        // dimensions; the space provider still contributes the ZWNJ glyph.
        let image = page_image("ascii", 8, 8, &[(3, 4)]);
        let atlas = build_hud_font_atlas(&[
            space_entry(&[(' ', 4), ('\u{200c}', 0)]),
            bitmap_entry(definition("minecraft:font/ascii.png", 8, 7, &["e"]), image),
        ])
        .unwrap();

        let zwnj = atlas.glyphs.get('\u{200c}').expect("ZWNJ glyph");
        assert_eq!(zwnj.advance, 0);
        assert_eq!((zwnj.width, zwnj.height), (0, 0));
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
        let atlas = build_hud_font_atlas(&[bitmap_entry(
            definition("minecraft:font/page.png", 8, 7, &["A"]),
            image,
        )])
        .unwrap();

        assert_eq!(atlas.glyphs.get('A').unwrap().advance, 4);
    }

    #[test]
    fn duplicate_codepoint_within_a_page_keeps_the_last_slot() {
        // Vanilla CodepointMap.put overwrites earlier slots within one
        // provider (and only warns).
        let image = page_image("page", 16, 8, &[(2, 0), (8 + 5, 0)]);
        let atlas = build_hud_font_atlas(&[bitmap_entry(
            definition("minecraft:font/page.png", 8, 7, &["AA"]),
            image,
        )])
        .unwrap();

        let glyph = atlas.glyphs.get('A').expect("A glyph");
        assert_eq!(glyph.advance, 7);
        assert_eq!(glyph.uv.min, [0.5, 0.0]);
    }

    /// Local acceptance against the SHA-1-verified launcher object store. CI
    /// fixtures cover the same parser without external files; setting
    /// `BBB_MC_CODE_ROOT` turns this into the real 26.1 asset check.
    #[test]
    fn local_vanilla_26_1_unihex_cjk_acceptance() {
        let Some(root) = std::env::var_os("BBB_MC_CODE_ROOT") else {
            return;
        };
        let roots = PackRoots::from_root(root).unwrap();
        let atlas = load_hud_font_atlas(&roots).unwrap();
        eprintln!(
            "Vanilla 26.1 font atlas: {}x{}, {} default glyphs",
            atlas.width,
            atlas.height,
            atlas.glyphs.len()
        );
        assert!(atlas.width <= 8_192);
        assert!(atlas.height <= 8_192);
        for codepoint in ['中', '日', '한', '\u{20000}'] {
            let glyph = atlas
                .glyphs
                .get(codepoint)
                .unwrap_or_else(|| panic!("missing real Vanilla unihex glyph {codepoint}"));
            assert_eq!(glyph.height, 16, "{codepoint}");
            assert_eq!(glyph.render_scale, 0.5, "{codepoint}");
            assert_eq!(glyph.bold_offset, 0.5, "{codepoint}");
            assert_eq!(glyph.shadow_offset, 0.5, "{codepoint}");
            assert!(glyph.width > 0, "{codepoint}");
            assert!(glyph.advance > 0, "{codepoint}");
            let quads =
                glyph.styled_quads(0.0, 0.0, bbb_render_types::HudTextStyle::default(), false);
            assert_eq!(quads.len(), 1, "{codepoint}");
            assert_eq!(quads[0].corners[2][1], 8.0, "{codepoint}");
        }
        // CJK size override is full 16 columns; the default provider remains
        // first-provider-wins behind bitmap/space entries.
        assert_eq!(atlas.glyphs.get('中').unwrap().width, 16);
        assert_eq!(atlas.glyphs.get('中').unwrap().advance, 9);
        assert_eq!(atlas.glyphs.get('A').unwrap().render_scale, 1.0);
        assert_eq!(atlas.glyphs.get(' ').unwrap().advance, 4);
    }
}
