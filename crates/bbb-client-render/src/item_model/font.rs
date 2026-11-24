//! Resource-pack font providers and shared glyph atlas.

#[cfg(test)]
mod bitmap_tests;
mod dynamic_atlas;
mod missing;
pub mod providers;
#[cfg(test)]
mod runtime_tests;
mod selection;
mod ttf;
mod unihex;

use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use anyhow::{bail, Context, Result};
use bbb_client_assets::{
    PackResourceStack, PackRoots, ResourceLocation, ResourcePngContentInvalid, SpriteImage,
};
use bbb_render_types::{
    HudDigitGlyph, HudDynamicFontAtlas, HudFontGlyphBook, HudFontGlyphMap, HudUvRect,
};

use crate::texture_sampling::copy_rgba_image;

use dynamic_atlas::{DynamicFontAtlas, LazyGlyphDescriptor, LazyProviderSource};
use providers::{
    FontBitmapProviderDefinition, FontProviderDefinition, FontSpaceProviderDefinition,
};
use selection::{GlyphCandidate, ProviderGlyphs};
use ttf::TtfGlyphSource;
use unihex::{UnihexGlyphSource, UNIHEX_GLYPH_HEIGHT};

/// Vanilla `Style.DEFAULT_FONT` for components without an explicit font.
pub const DEFAULT_FONT_ID: &str = "minecraft:default";

/// Contiguous RGBA pixel backing for the static portion of [`HudFontAtlasImage`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HudFontAtlasBytes(Vec<u8>);

impl HudFontAtlasBytes {
    fn zeroed(byte_count: usize) -> Self {
        Self(vec![0; byte_count])
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl Deref for HudFontAtlasBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for HudFontAtlasBytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<[u8]> for HudFontAtlasBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Every selected resource-pack font: static bitmap pages plus the missing glyph
/// baked into `rgba`, with Unihex and TrueType glyphs rasterized lazily on demand
/// via `fonts.dynamic_atlas()`.
#[derive(Debug, Clone, PartialEq)]
pub struct HudFontAtlasImage {
    pub width: u32,
    pub height: u32,
    pub rgba: HudFontAtlasBytes,
    pub glyphs: HudFontGlyphMap,
    pub fonts: HudFontGlyphBook,
}

impl HudFontAtlasImage {
    /// Active GPU texture dimensions, including any dynamic glyph shelf reserved
    /// below the static bitmap region.
    pub fn atlas_dimensions(&self) -> (u32, u32) {
        self.fonts
            .dynamic_atlas()
            .map_or((self.width, self.height), |atlas| atlas.atlas_dimensions())
    }

    /// Reads one RGBA texel from either the static bitmap region or a lazily
    /// rasterized dynamic glyph.
    pub fn pixel_rgba(&self, x: u32, y: u32) -> [u8; 4] {
        if let Some(dynamic) = self.fonts.dynamic_atlas() {
            return dynamic.pixel_rgba(x, y);
        }
        rgba_offset(self.width, x, y)
            .ok()
            .and_then(|offset| self.rgba.get(offset..offset + 4))
            .map_or([0; 4], |pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
    }
}

/// One flattened provider ready for atlas baking: bitmap providers carry
/// their loaded page image, space providers carry only their advances (they
/// contribute no atlas pixels, mirroring vanilla `EmptyGlyph`), and unihex and
/// truetype providers carry compact lazy glyph sources.
#[derive(Clone)]
enum FontAtlasEntry {
    Missing,
    Bitmap(FontBitmapProviderDefinition, Arc<SpriteImage>),
    Space(FontSpaceProviderDefinition),
    Unihex(Arc<UnihexGlyphSource>),
    TrueType(Arc<TtfGlyphSource>),
}

#[derive(Clone)]
enum FontProviderLoad {
    Loaded(FontAtlasEntry),
    Invalid(Arc<str>),
}

#[derive(Default)]
struct FontAtlasLoadCache {
    entries: Vec<(Arc<FontProviderDefinition>, FontProviderLoad)>,
}

#[derive(Debug)]
struct MissingBitmapResource(String);

impl std::fmt::Display for MissingBitmapResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "missing font bitmap texture {}", self.0)
    }
}
impl std::error::Error for MissingBitmapResource {}

/// Loads every selected `font/*.json` definition and bakes all bitmap pages
/// into one shared atlas. Per-font glyph maps keep their own first-provider-wins
/// semantics while their UVs address that shared image, so Component font
/// changes do not require per-run texture binds.
pub fn load_hud_font_atlas(roots: &PackRoots) -> Result<HudFontAtlasImage> {
    load_hud_font_atlas_with_options(roots, providers::FontOptions::default())
}

pub(crate) use dynamic_atlas::MAX_DYNAMIC_FONT_ATLAS_BYTES;

pub fn load_hud_font_atlas_with_options(
    roots: &PackRoots,
    options: providers::FontOptions,
) -> Result<HudFontAtlasImage> {
    load_hud_font_atlas_with_options_and_max_dimension(
        roots,
        options,
        dynamic_atlas::DEFAULT_MAX_DYNAMIC_ATLAS_DIMENSION,
    )
}

pub fn load_hud_font_atlas_with_options_and_max_dimension(
    roots: &PackRoots,
    options: providers::FontOptions,
    max_dimension: u32,
) -> Result<HudFontAtlasImage> {
    let stack = roots.resource_stack();
    let mut fonts = Vec::new();
    let mut cache = FontAtlasLoadCache::default();
    let mut default_failure = None;
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
        let candidates = providers::load_font_candidates(roots, &font_id, options)?;
        let (entries, failure) = load_font_atlas_entries(roots, candidates, &mut cache)?;
        if let Some(failure) = failure {
            tracing::warn!(font = font_id, %failure, "rejecting font with an invalid provider");
            if font_id == DEFAULT_FONT_ID {
                default_failure = Some(failure);
            }
        } else {
            fonts.push((font_id, entries));
        }
    }
    drop(cache);
    if let Some(failure) = default_failure {
        bail!("default font failed to load: {failure}");
    }
    if !fonts.iter().any(|(font_id, _)| font_id == DEFAULT_FONT_ID) {
        bail!("font {DEFAULT_FONT_ID} has no selected definition");
    }
    build_hud_font_book_atlas_with_budget_and_limit(fonts, Some(&stack), max_dimension)
}

fn load_font_atlas_entries(
    roots: &PackRoots,
    candidates: providers::FontCandidates,
    cache: &mut FontAtlasLoadCache,
) -> Result<(Vec<FontAtlasEntry>, Option<Arc<str>>)> {
    let mut failure = candidates
        .reference_failure
        .map(|error| Arc::from(error.to_string()));
    let mut entries = Vec::new();
    // FontManager loads every builder before FontSet option filtering.
    for candidate in candidates.providers {
        let loaded = if let Some((_, loaded)) = cache
            .entries
            .iter()
            .find(|(definition, _)| definition.as_ref() == candidate.definition.as_ref())
        {
            loaded.clone()
        } else {
            let loaded = match load_font_atlas_entry(roots, candidate.definition.as_ref()) {
                Ok(entry) => FontProviderLoad::Loaded(entry),
                Err(error)
                    if error.is::<MissingBitmapResource>()
                        || error.is::<ResourcePngContentInvalid>()
                        || error.is::<unihex::UnihexProviderInvalid>()
                        || error.is::<ttf::TtfProviderInvalid>() =>
                {
                    FontProviderLoad::Invalid(Arc::from(format!("{error:#}")))
                }
                Err(error) => return Err(error),
            };
            cache.entries.push((candidate.definition, loaded.clone()));
            loaded
        };
        match loaded {
            FontProviderLoad::Loaded(entry) if candidate.enabled => entries.push(entry),
            FontProviderLoad::Loaded(_) => {}
            FontProviderLoad::Invalid(error) => {
                failure.get_or_insert(error);
            }
        }
    }
    Ok((entries, failure))
}

fn load_font_atlas_entry(
    roots: &PackRoots,
    definition: &FontProviderDefinition,
) -> Result<FontAtlasEntry> {
    Ok(match definition {
        FontProviderDefinition::Bitmap(bitmap) => {
            let image = Arc::new(load_font_bitmap_texture(roots, &bitmap.file)?);
            FontAtlasEntry::Bitmap(bitmap.clone(), image)
        }
        FontProviderDefinition::Space(space) => FontAtlasEntry::Space(space.clone()),
        FontProviderDefinition::Unihex(unihex) => {
            FontAtlasEntry::Unihex(Arc::new(unihex::load_unihex_glyph_source(roots, unihex)?))
        }
        FontProviderDefinition::TrueType(ttf) => {
            FontAtlasEntry::TrueType(Arc::new(ttf::load_ttf_glyph_source(roots, ttf)?))
        }
    })
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
        .ok_or_else(|| MissingBitmapResource(location.id()))?;
    SpriteImage::from_pack_resource_pixels(location.id(), &resource, &resources)
}

#[cfg(test)]
fn build_hud_font_atlas(entries: &[FontAtlasEntry]) -> Result<HudFontAtlasImage> {
    build_hud_font_book_atlas(&[(DEFAULT_FONT_ID.to_string(), entries.to_vec())])
}

#[cfg(test)]
fn build_hud_font_book_atlas(fonts: &[(String, Vec<FontAtlasEntry>)]) -> Result<HudFontAtlasImage> {
    build_hud_font_book_atlas_with_budget_and_limit(
        fonts.to_vec(),
        None,
        dynamic_atlas::DEFAULT_MAX_DYNAMIC_ATLAS_DIMENSION,
    )
}

enum RegisteredLazySource {
    Unihex {
        source: Arc<UnihexGlyphSource>,
        base_lazy_key: u32,
    },
    TrueType {
        source: Arc<TtfGlyphSource>,
        glyph_lazy_keys: Vec<u32>,
    },
}

struct LazyFontRegistry {
    sources: Vec<LazyProviderSource>,
    registered: Vec<RegisteredLazySource>,
    entries: Vec<LazyGlyphDescriptor>,
    max_glyph_width: u32,
    max_glyph_height: u32,
}

impl LazyFontRegistry {
    fn from_fonts(fonts: &[(String, Vec<FontAtlasEntry>)]) -> Result<Self> {
        let mut sources = Vec::new();
        let mut registered = Vec::new();
        let mut entries = Vec::new();
        let mut max_glyph_width = 0_u32;
        let mut max_glyph_height = 0_u32;

        for entry in fonts.iter().flat_map(|(_, entries)| entries) {
            match entry {
                FontAtlasEntry::Unihex(source) if !source.glyphs.is_empty() => {
                    if registered.iter().any(|existing| {
                        matches!(
                            existing,
                            RegisteredLazySource::Unihex { source: prev, .. }
                                if Arc::ptr_eq(prev, source)
                        )
                    }) {
                        continue;
                    }
                    let source_index =
                        u32::try_from(sources.len()).context("lazy font source count overflow")?;
                    let base_lazy_key = u32::try_from(entries.len())
                        .ok()
                        .and_then(|n| n.checked_add(1))
                        .context("lazy font glyph count overflow")?;
                    for (glyph_idx, glyph) in source.glyphs.iter().enumerate() {
                        max_glyph_width = max_glyph_width.max(glyph.pixel_width);
                        max_glyph_height = max_glyph_height.max(UNIHEX_GLYPH_HEIGHT);
                        entries.push(LazyGlyphDescriptor {
                            source_index,
                            glyph_index: u32::try_from(glyph_idx)
                                .context("unihex glyph index overflow")?,
                            width: glyph.pixel_width,
                            height: UNIHEX_GLYPH_HEIGHT,
                        });
                    }
                    sources.push(LazyProviderSource::Unihex(Arc::clone(source)));
                    registered.push(RegisteredLazySource::Unihex {
                        source: Arc::clone(source),
                        base_lazy_key,
                    });
                }
                FontAtlasEntry::TrueType(source) if source.has_raster_glyphs() => {
                    if registered.iter().any(|existing| {
                        matches!(
                            existing,
                            RegisteredLazySource::TrueType { source: prev, .. }
                                if Arc::ptr_eq(prev, source)
                        )
                    }) {
                        continue;
                    }
                    let source_index =
                        u32::try_from(sources.len()).context("lazy font source count overflow")?;
                    let mut glyph_lazy_keys = Vec::with_capacity(source.glyphs.len());
                    for (glyph_idx, glyph) in source.glyphs.iter().enumerate() {
                        match glyph.bounds {
                            Some(bounds) if bounds.width <= 256 && bounds.height <= 256 => {
                                let lazy_key = u32::try_from(entries.len())
                                    .ok()
                                    .and_then(|n| n.checked_add(1))
                                    .context("lazy font glyph count overflow")?;
                                max_glyph_width = max_glyph_width.max(bounds.width);
                                max_glyph_height = max_glyph_height.max(bounds.height);
                                entries.push(LazyGlyphDescriptor {
                                    source_index,
                                    glyph_index: u32::try_from(glyph_idx)
                                        .context("truetype glyph index overflow")?,
                                    width: bounds.width,
                                    height: bounds.height,
                                });
                                glyph_lazy_keys.push(lazy_key);
                            }
                            _ => glyph_lazy_keys.push(0),
                        }
                    }
                    sources.push(LazyProviderSource::TrueType(Arc::clone(source)));
                    registered.push(RegisteredLazySource::TrueType {
                        source: Arc::clone(source),
                        glyph_lazy_keys,
                    });
                }
                _ => {}
            }
        }

        Ok(Self {
            sources,
            registered,
            entries,
            max_glyph_width,
            max_glyph_height,
        })
    }

    fn unihex_base_lazy_key(&self, source: &Arc<UnihexGlyphSource>) -> Option<u32> {
        self.registered.iter().find_map(|item| match item {
            RegisteredLazySource::Unihex {
                source: prev,
                base_lazy_key,
            } if Arc::ptr_eq(prev, source) => Some(*base_lazy_key),
            _ => None,
        })
    }

    fn ttf_lazy_keys(&self, source: &Arc<TtfGlyphSource>) -> Option<&[u32]> {
        self.registered.iter().find_map(|item| match item {
            RegisteredLazySource::TrueType {
                source: prev,
                glyph_lazy_keys,
            } if Arc::ptr_eq(prev, source) => Some(glyph_lazy_keys.as_slice()),
            _ => None,
        })
    }
}

fn build_hud_font_book_atlas_with_budget_and_limit(
    fonts: Vec<(String, Vec<FontAtlasEntry>)>,
    budget: Option<&PackResourceStack>,
    max_dimension: u32,
) -> Result<HudFontAtlasImage> {
    let max_dimension = max_dimension.clamp(64, dynamic_atlas::DEFAULT_MAX_DYNAMIC_ATLAS_DIMENSION);
    let lazy = LazyFontRegistry::from_fonts(&fonts)?;
    let has_lazy = !lazy.entries.is_empty();
    let min_static_width = if has_lazy {
        let base = if lazy.entries.len() > 64 {
            1_024
        } else {
            lazy.max_glyph_width.max(64).next_power_of_two().min(1_024)
        };
        Some(base.max(lazy.max_glyph_width).min(max_dimension))
    } else {
        None
    };

    let (static_width, static_height, owned_placements) = {
        let (width, height, placements) = layout_unique_static_pages(&fonts, min_static_width)?;
        let owned = placements
            .into_iter()
            .map(|p| (p.entry.clone(), p.x, p.y))
            .collect::<Vec<_>>();
        (width, height, owned)
    };
    let atlas_height = if has_lazy {
        let target = if lazy.entries.len() > 64 {
            static_height
                .saturating_add(512)
                .next_power_of_two()
                .clamp(512, 1_024)
        } else {
            static_height
                .saturating_add(lazy.max_glyph_height.max(32))
                .saturating_add(16)
                .next_power_of_two()
                .clamp(64, 1_024)
        };
        target
            .max(
                static_height
                    .saturating_add(lazy.max_glyph_height)
                    .saturating_add(1),
            )
            .min(max_dimension)
            .max(static_height)
    } else {
        static_height
    };
    let find_origin = |target: &FontAtlasEntry| -> Option<(u32, u32)> {
        owned_placements
            .iter()
            .find(|(existing, _, _)| same_static_page(existing, target))
            .map(|&(_, x, y)| (x, y))
    };
    let byte_count = rgba_len(static_width, static_height)?;
    if let Some(budget) = budget {
        budget.reserve_image_bytes(byte_count as u64, "shared font atlas")?;
    }
    let (missing_x, missing_y) =
        find_origin(&MISSING_PAGE).context("missing glyph has no atlas placement")?;
    let missing_glyph = missing::glyph(static_width, atlas_height, missing_x, missing_y);
    let mut maps = BTreeMap::new();
    for (font_id, entries) in fonts {
        selection::admit_candidates(&entries, budget)?;
        let mut provider_glyphs = Vec::with_capacity(entries.len());
        for entry in &entries {
            let mut glyphs = Vec::with_capacity(selection::slot_count(entry));
            match entry {
                FontAtlasEntry::Bitmap(definition, image) => {
                    let (px, py) =
                        find_origin(entry).context("font bitmap page has no atlas placement")?;
                    collect_page_glyphs(
                        definition,
                        image,
                        static_width,
                        atlas_height,
                        px,
                        py,
                        missing_glyph,
                        &mut glyphs,
                    )?;
                }
                FontAtlasEntry::Missing => {}
                FontAtlasEntry::Space(space) => collect_space_glyphs(space, &mut glyphs),
                FontAtlasEntry::TrueType(source) => {
                    let lazy_keys = lazy.ttf_lazy_keys(source);
                    collect_ttf_glyphs(source, lazy_keys, missing_glyph, &mut glyphs);
                }
                FontAtlasEntry::Unihex(source) if source.glyphs.is_empty() => {}
                FontAtlasEntry::Unihex(source) => {
                    let base_lazy_key = lazy
                        .unihex_base_lazy_key(source)
                        .context("unihex glyph source was not registered in lazy atlas")?;
                    collect_unihex_glyphs(source, base_lazy_key, &mut glyphs);
                }
            }
            provider_glyphs.push(glyphs);
        }
        drop(entries);
        let glyphs = selection::select(&provider_glyphs, missing_glyph, budget)?;
        drop(provider_glyphs);
        maps.insert(font_id, glyphs);
    }
    selection::admit_book(&maps, budget)?;

    let mut rgba = HudFontAtlasBytes::zeroed(byte_count);
    for (entry, x, y) in owned_placements {
        match entry {
            FontAtlasEntry::Bitmap(_, image) => copy_rgba_image(
                &mut rgba,
                static_width,
                x,
                y,
                image.width,
                image.rgba.as_slice(),
            )?,
            FontAtlasEntry::Missing => {
                missing::blit(&mut rgba, static_width, x, y)?;
            }
            FontAtlasEntry::Space(_) | FontAtlasEntry::Unihex(_) | FontAtlasEntry::TrueType(_) => {
                unreachable!("non-static provider has no eager raster page")
            }
        }
    }

    if has_lazy {
        let dynamic: Arc<dyn HudDynamicFontAtlas> = Arc::new(DynamicFontAtlas::new(
            static_width,
            static_height,
            Arc::from(rgba.as_slice()),
            static_width,
            atlas_height,
            max_dimension,
            missing_glyph.uv,
            lazy.sources,
            lazy.entries,
        ));
        for map in maps.values_mut() {
            map.set_dynamic_atlas(Arc::clone(&dynamic));
        }
    }

    let glyphs = maps
        .get(DEFAULT_FONT_ID)
        .cloned()
        .context("default font glyph map was not built")?;
    let font_book = HudFontGlyphBook::from_fonts(DEFAULT_FONT_ID, maps);

    Ok(HudFontAtlasImage {
        width: static_width,
        height: static_height,
        rgba,
        glyphs,
        fonts: font_book,
    })
}

static MISSING_PAGE: FontAtlasEntry = FontAtlasEntry::Missing;

struct FontPagePlacement<'a> {
    entry: &'a FontAtlasEntry,
    x: u32,
    y: u32,
}

fn layout_unique_static_pages(
    fonts: &[(String, Vec<FontAtlasEntry>)],
    min_width: Option<u32>,
) -> Result<(u32, u32, Vec<FontPagePlacement<'_>>)> {
    let mut unique = Vec::<&FontAtlasEntry>::new();
    for entry in fonts.iter().flat_map(|(_, entries)| entries) {
        if static_page_dimensions(entry).is_some()
            && !unique
                .iter()
                .any(|existing| same_static_page(existing, entry))
        {
            unique.push(entry);
        }
    }
    unique.push(&MISSING_PAGE);
    let mut width = unique
        .iter()
        .filter_map(|entry| static_page_dimensions(entry).map(|(width, _)| width))
        .max()
        .context("font atlas needs at least one raster page")?;
    if let Some(min_width) = min_width {
        width = width.max(min_width);
    }
    if width > 8_192 {
        bail!("font atlas width {width} exceeds the 8192 pixel texture limit");
    }
    let mut placements = Vec::with_capacity(unique.len());
    let mut x = 0_u32;
    let mut y = 0_u32;
    let mut row_height = 0_u32;
    for entry in unique {
        let (page_width, page_height) = static_page_dimensions(entry).unwrap();
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

fn static_page_dimensions(entry: &FontAtlasEntry) -> Option<(u32, u32)> {
    match entry {
        FontAtlasEntry::Missing => Some((missing::WIDTH, missing::HEIGHT)),
        FontAtlasEntry::Bitmap(_, image) if image.width > 0 && image.height > 0 => {
            Some((image.width, image.height))
        }
        FontAtlasEntry::Bitmap(_, _)
        | FontAtlasEntry::Space(_)
        | FontAtlasEntry::Unihex(_)
        | FontAtlasEntry::TrueType(_) => None,
    }
}

fn same_static_page(left: &FontAtlasEntry, right: &FontAtlasEntry) -> bool {
    match (left, right) {
        (FontAtlasEntry::Missing, FontAtlasEntry::Missing) => true,
        (FontAtlasEntry::Bitmap(_, left), FontAtlasEntry::Bitmap(_, right)) => {
            Arc::ptr_eq(left, right)
        }
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_page_glyphs(
    definition: &FontBitmapProviderDefinition,
    image: &SpriteImage,
    atlas_width: u32,
    atlas_height: u32,
    page_x: u32,
    page_y: u32,
    missing_glyph: HudDigitGlyph,
    glyphs: &mut ProviderGlyphs,
) -> Result<()> {
    // Vanilla BitmapProvider.Definition.load: integer cell size from the image
    // dimensions and the codepoint grid shape.
    let columns = u32::try_from(definition.chars[0].len()).context("font grid column overflow")?;
    let rows = u32::try_from(definition.chars.len()).context("font grid row overflow")?;
    let glyph_width = image.width / columns;
    let glyph_height = image.height / rows;
    // `pixelScale = (float)height / glyphHeight` (BitmapProvider.load).
    let pixel_scale = definition.height as f32 / glyph_height as f32;

    for (row_index, row) in definition.chars.iter().enumerate() {
        for (column_index, &codepoint) in row.iter().enumerate() {
            // Vanilla skips `\u0000` grid slots (`if (c != 0)`).
            if codepoint == 0 {
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
            let advance = ((0.5_f64 + f64::from(actual_width as f32 * pixel_scale)) as i32)
                .wrapping_add(1) as f32;
            let glyph = HudDigitGlyph {
                uv: HudUvRect {
                    min: [
                        ((page_x + src_x) as f32 + 0.01) / atlas_width as f32,
                        ((page_y + src_y) as f32 + 0.01) / atlas_height as f32,
                    ],
                    max: [
                        ((page_x + src_x) as f32 - 0.01 + glyph_width as f32) / atlas_width as f32,
                        ((page_y + src_y) as f32 - 0.01 + glyph_height as f32)
                            / atlas_height as f32,
                    ],
                },
                width: glyph_width,
                height: glyph_height,
                render_scale: pixel_scale,
                advance,
                // GlyphBitmap.getBearingTop() returns the provider ascent.
                bearing_top: definition.ascent as f32,
                ..HudDigitGlyph::default()
            };
            let mut candidate = GlyphCandidate::new(glyph);
            // FontTexture bake failure leaves the unbaked info intact.
            if glyph_width > 256 || glyph_height > 256 {
                candidate.baked = missing_glyph;
            }
            glyphs.push((codepoint, candidate));
        }
    }
    // BitmapProvider replaces duplicate slots before provider selection.
    glyphs.sort_by_key(|(codepoint, _)| *codepoint);
    glyphs.dedup_by(|later, earlier| {
        if later.0 == earlier.0 {
            *earlier = *later;
            true
        } else {
            false
        }
    });
    Ok(())
}

fn collect_unihex_glyphs(
    source: &UnihexGlyphSource,
    base_lazy_key: u32,
    glyphs: &mut ProviderGlyphs,
) {
    for (index, glyph) in source.glyphs.iter().enumerate() {
        glyphs.push((
            glyph.codepoint,
            GlyphCandidate::new_lazy(
                HudDigitGlyph {
                    uv: HudUvRect {
                        min: [0.0, 0.0],
                        max: [0.0, 0.0],
                    },
                    width: glyph.pixel_width,
                    height: UNIHEX_GLYPH_HEIGHT,
                    // `UnihexProvider.Glyph.info`: integer division happens
                    // before widening to float.
                    advance: (glyph.pixel_width / 2 + 1) as f32,
                    bearing_top: 7.0,
                    bearing_left: 0.0,
                    render_scale: 0.5,
                    bold_offset: 0.5,
                    shadow_offset: 0.5,
                    is_empty: false,
                },
                base_lazy_key + index as u32,
            ),
        ));
    }
}

/// `TrueTypeGlyphProvider`: the charmap entry's own metrics drive the glyph,
/// the rasterized coverage cell only supplies pixels.
fn collect_ttf_glyphs(
    source: &TtfGlyphSource,
    lazy_keys: Option<&[u32]>,
    missing_glyph: HudDigitGlyph,
    glyphs: &mut ProviderGlyphs,
) {
    for (index, glyph) in source.glyphs.iter().enumerate() {
        let candidate = match glyph.bounds {
            Some(bounds) => {
                let lazy_key = lazy_keys.map_or(0, |keys| keys[index]);
                let mut candidate = GlyphCandidate::new_lazy(
                    HudDigitGlyph {
                        uv: HudUvRect {
                            min: [0.0, 0.0],
                            max: [0.0, 0.0],
                        },
                        width: bounds.width,
                        height: bounds.height,
                        advance: glyph.advance,
                        bearing_top: glyph.bearing_top,
                        bearing_left: glyph.bearing_left,
                        // `GlyphBitmap.getOversample()` for a ttf glyph.
                        render_scale: 1.0 / source.oversample,
                        ..HudDigitGlyph::default()
                    },
                    lazy_key,
                );
                // FontTexture refuses a bitmap larger than its 256px sheet.
                if bounds.width > 256 || bounds.height > 256 {
                    candidate.baked = missing_glyph;
                    candidate.lazy_key = 0;
                }
                candidate
            }
            // `TrueTypeGlyphProvider.loadGlyph` answers `EmptyGlyph` for a
            // glyph whose bitmap has no area.
            None => GlyphCandidate::new(HudDigitGlyph::empty(glyph.advance)),
        };
        glyphs.push((glyph.codepoint, candidate));
    }
}

// EmptyGlyph emits no bitmap regardless of advance.
fn collect_space_glyphs(definition: &FontSpaceProviderDefinition, glyphs: &mut ProviderGlyphs) {
    for &(codepoint, advance) in &definition.advances {
        glyphs.push((
            codepoint,
            GlyphCandidate::new(HudDigitGlyph::empty(advance)),
        ));
    }
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
            chars: chars
                .iter()
                .map(|row| row.chars().map(u32::from).collect())
                .collect(),
        }
    }

    fn bitmap_entry(
        definition: FontBitmapProviderDefinition,
        image: SpriteImage,
    ) -> FontAtlasEntry {
        FontAtlasEntry::Bitmap(definition, Arc::new(image))
    }

    fn space_entry(advances: &[(char, f32)]) -> FontAtlasEntry {
        FontAtlasEntry::Space(FontSpaceProviderDefinition {
            advances: advances
                .iter()
                .map(|&(ch, advance)| (u32::from(ch), advance))
                .collect(),
        })
    }

    fn unihex_entry(glyphs: &[(char, u32, u32)]) -> FontAtlasEntry {
        let mapped = glyphs
            .iter()
            .map(|&(codepoint, _, pixel_width)| (u32::from(codepoint), pixel_width))
            .collect::<Vec<_>>();
        FontAtlasEntry::Unihex(Arc::new(UnihexGlyphSource::from_test_glyphs(&mapped)))
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
        assert_eq!(glyph.advance, 7.0);
        // Rendered size is the cell scaled by pixelScale: 24 * 0.5 = 12.
        assert_eq!(glyph.width, 24);
        assert_eq!(glyph.height, 24);
        assert_eq!(glyph.rendered_width(), 12.0);
        assert_eq!(glyph.rendered_height(), 12.0);
        assert_eq!(glyph.bearing_top, 10.0);
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
        assert_eq!(glyph.uv.min, [0.01 / 8.0, 0.01 / 24.0]);
        assert_eq!(glyph.uv.max, [7.99 / 8.0, 7.99 / 24.0]);
        // Advance from the first page's 4px actual width, not the second's 7px.
        assert_eq!(glyph.advance, 5.0);
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

        // Default's higher bitmap page wins A; CJK falls through to unihex and
        // rasterizes lazily on demand without baking a monolithic RGBA page.
        assert_eq!(atlas.glyphs.get('A').unwrap().render_scale, 1.0);
        assert_eq!(
            atlas.fonts.dynamic_atlas().unwrap().upload_revision(),
            0,
            "reading a static bitmap glyph must not realize lazy unihex glyphs"
        );
        let cjk = atlas.glyphs.get('中').unwrap();
        assert_eq!((cjk.width, cjk.height, cjk.advance), (16, 16, 9.0));
        assert_eq!(cjk.rendered_width(), 8.0);
        assert_eq!((cjk.bold_offset, cjk.shadow_offset), (0.5, 0.5));
        assert_eq!(
            cjk.styled_advance_f32(bbb_render_types::HudTextStyle {
                bold: true,
                ..Default::default()
            }),
            9.5
        );
        // The static RGBA backing only holds the bitmap + missing pages (height 8).
        assert_eq!(atlas.height, 8);
        // Both default and uniform fonts share the same lazy placement and UV.
        assert_eq!(
            atlas.fonts.glyphs(Some("minecraft:uniform")).get('中'),
            Some(cjk)
        );
        assert_eq!(
            atlas.fonts.dynamic_atlas().unwrap().upload_revision(),
            1,
            "shared unihex glyph is realized only once across both fonts"
        );
        let (dyn_w, dyn_h) = atlas.atlas_dimensions();
        let x = (cjk.uv.min[0] * dyn_w as f32).round() as u32;
        let y = (cjk.uv.min[1] * dyn_h as f32).round() as u32;
        assert_eq!(atlas.pixel_rgba(x, y), [255, 255, 255, 255]);
    }

    #[test]
    fn lazy_unihex_measurement_avoids_rasterization_and_dynamic_atlas_grows_preserving_static_uvs()
    {
        let bitmap = bitmap_entry(
            definition("minecraft:font/ascii.png", 8, 7, &["A"]),
            page_image("ascii", 8, 8, &[(3, 0)]),
        );
        let many_glyphs: Vec<(u32, u32)> = (0..400).map(|i| (0x4E00 + i, 16)).collect();
        let unihex =
            FontAtlasEntry::Unihex(Arc::new(UnihexGlyphSource::from_test_glyphs(&many_glyphs)));
        let atlas = build_hud_font_atlas(&[bitmap, unihex]).unwrap();
        let dynamic = atlas.fonts.dynamic_atlas().expect("dynamic atlas present");

        // Measuring plain text width must not realize any lazy glyph.
        assert_eq!(atlas.glyphs.plain_text_width("一二三"), 27);
        assert_eq!(dynamic.upload_revision(), 0);
        // Initial static RGBA is compact (1024 x 8), far smaller than a monolithic atlas.
        assert!(atlas.rgba.len() <= 1024 * 16 * 4);

        // Realize enough glyphs to verify placements & pixel readback remain exact.
        let initial_gen = dynamic.layout_generation();
        for i in 0..400_u32 {
            let ch = char::from_u32(0x4E00 + i).unwrap();
            let glyph = atlas.glyphs.get(ch).unwrap();
            assert_eq!((glyph.width, glyph.height), (16, 16));
        }
        assert_eq!(dynamic.upload_revision(), 400);
        assert!(dynamic.layout_generation() >= initial_gen);

        let (dyn_w, dyn_h) = atlas.atlas_dimensions();
        let a = atlas.glyphs.get('A').unwrap();
        let ax = (a.uv.min[0] * dyn_w as f32).round() as u32;
        let ay = (a.uv.min[1] * dyn_h as f32).round() as u32;
        assert_eq!(atlas.pixel_rgba(ax + 3, ay), [255, 255, 255, 255]);

        let last = atlas
            .glyphs
            .get(char::from_u32(0x4E00 + 399).unwrap())
            .unwrap();
        let lx = (last.uv.min[0] * dyn_w as f32).round() as u32;
        let ly = (last.uv.min[1] * dyn_h as f32).round() as u32;
        assert_eq!(atlas.pixel_rgba(lx, ly), [255, 255, 255, 255]);
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

        assert_eq!((atlas.width, atlas.height), (16, 24));
        assert_eq!(atlas.fonts.font_count(), 2);
        let default = atlas.fonts.glyphs(None).get('A').unwrap();
        let custom = atlas.fonts.glyphs(Some("example:custom")).get('A').unwrap();
        assert_eq!((default.advance, custom.advance), (4.0, 11.0));
        assert_eq!(
            default.uv,
            HudUvRect {
                min: [0.01 / 16.0, 0.01 / 24.0],
                max: [7.99 / 16.0, 7.99 / 24.0]
            }
        );
        assert_eq!(
            custom.uv,
            HudUvRect {
                min: [0.01 / 16.0, 8.01 / 24.0],
                max: [15.99 / 16.0, 15.99 / 24.0]
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
        assert_eq!(atlas.height, 28);
        // The `\u{0}` cells produce no glyphs (vanilla skips them).
        assert_eq!(atlas.glyphs.len(), 2);

        let a = atlas.glyphs.get('A').expect("A glyph");
        assert_eq!(a.uv.min, [0.01 / 18.0, 0.01 / 28.0]);
        assert_eq!(a.uv.max, [7.99 / 18.0, 7.99 / 28.0]);
        assert_eq!((a.width, a.height, a.bearing_top), (8, 8, 7.0));

        let e = atlas.glyphs.get('é').expect("é glyph");
        assert_eq!(e.uv.min, [0.01 / 18.0, 8.01 / 28.0]);
        assert_eq!(e.uv.max, [8.99 / 18.0, 19.99 / 28.0]);
        assert_eq!((e.width, e.height, e.bearing_top), (9, 12, 10.0));

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
            ('ж', 6.0, 7.0),
            ('λ', 7.0, 7.0),
            ('é', 5.0, 10.0),
            ('ü', 6.0, 10.0),
            ('ñ', 4.0, 10.0),
            ('e', 5.0, 7.0),
        ] {
            let glyph = atlas
                .glyphs
                .get(codepoint)
                .unwrap_or_else(|| panic!("{codepoint} should have a glyph"));
            assert_eq!(glyph.advance, advance, "{codepoint} advance");
            assert_eq!(glyph.bearing_top, ascent, "{codepoint} ascent");
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
            space_entry(&[(' ', 4.0), ('\u{200c}', 0.0)]),
            bitmap_entry(definition("minecraft:font/ascii.png", 8, 7, &[" ~"]), image),
        ])
        .unwrap();

        let space = atlas.glyphs.get(' ').expect("space glyph");
        assert_eq!(space.advance, 4.0);
        // The space provider's `EmptyGlyph` carries no pixels.
        assert_eq!((space.width, space.height), (0, 0));
        assert_eq!(atlas.glyphs.get('~').unwrap().advance, 9.0);
    }

    #[test]
    fn space_provider_precedes_a_bitmap_page_with_its_own_space_cell() {
        // If a bitmap page also mapped ' ' to a real (non-blank) glyph cell,
        // the space provider (listed first) still wins the codepoint.
        let image = page_image("ascii", 16, 8, &[(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]);
        let atlas = build_hud_font_atlas(&[
            space_entry(&[(' ', 4.0)]),
            bitmap_entry(definition("minecraft:font/ascii.png", 8, 7, &[" "]), image),
        ])
        .unwrap();

        assert_eq!(atlas.glyphs.get(' ').unwrap().advance, 4.0);
    }

    #[test]
    fn zero_width_non_joiner_advances_zero_and_bakes_no_pixels() {
        // The atlas always needs at least one bitmap page for its pixel
        // dimensions; the space provider still contributes the ZWNJ glyph.
        let image = page_image("ascii", 8, 8, &[(3, 4)]);
        let atlas = build_hud_font_atlas(&[
            space_entry(&[(' ', 4.0), ('\u{200c}', 0.0)]),
            bitmap_entry(definition("minecraft:font/ascii.png", 8, 7, &["e"]), image),
        ])
        .unwrap();

        let zwnj = atlas.glyphs.get('\u{200c}').expect("ZWNJ glyph");
        assert_eq!(zwnj.advance, 0.0);
        assert_eq!((zwnj.width, zwnj.height), (0, 0));
    }

    #[test]
    fn transparent_white_pixels_are_invisible_to_the_advance_scan() {
        // The vanilla font PNGs are white-palette images whose transparent
        // pixels decode to (255,255,255,0); `getLuminanceOrAlpha` reads only
        // alpha, so they must not count toward the actual glyph width.
        let mut rgba = vec![255u8; rgba_len(8, 8).unwrap()];
        for pixel in rgba.as_chunks_mut::<4>().0 {
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

        assert_eq!(atlas.glyphs.get('A').unwrap().advance, 4.0);
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
        assert_eq!(glyph.advance, 7.0);
        assert_eq!(glyph.uv.min, [8.01 / 16.0, 0.01 / 16.0]);
    }

    /// Local acceptance against the SHA-1-verified launcher object store. CI
    /// fixtures cover the same parser without external files; setting
    /// `BBB_VANILLA_ROOT` turns this into the real 26.1 asset check.
    #[test]
    fn local_vanilla_26_1_unihex_cjk_acceptance() {
        let Some(root) = std::env::var_os("BBB_VANILLA_ROOT") else {
            return;
        };
        let roots = PackRoots::from_root(root).unwrap();
        let atlas = load_hud_font_atlas(&roots).unwrap();
        eprintln!(
            "Vanilla 26.1 font atlas: {}x{}, {} default glyphs, data {:?}",
            atlas.width,
            atlas.height,
            atlas.glyphs.len(),
            roots.resource_data_memory_observation()
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
            assert!(glyph.advance > 0.0, "{codepoint}");
            let quads =
                glyph.styled_quads(0.0, 0.0, bbb_render_types::HudTextStyle::default(), false);
            assert_eq!(quads.len(), 1, "{codepoint}");
            assert_eq!(quads[0].corners[2][1], 8.0, "{codepoint}");
        }
        // CJK size override is full 16 columns; the default provider remains
        // first-provider-wins behind bitmap/space entries.
        assert_eq!(atlas.glyphs.get('中').unwrap().width, 16);
        assert_eq!(atlas.glyphs.get('中').unwrap().advance, 9.0);
        assert_eq!(atlas.glyphs.get('A').unwrap().render_scale, 1.0);
        assert_eq!(atlas.glyphs.get(' ').unwrap().advance, 4.0);
    }
}
