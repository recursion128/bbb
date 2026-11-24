use std::{collections::BTreeSet, sync::Arc};

use ab_glyph::{point, Font, FontRef, GlyphId, OutlinedGlyph, PxScale, ScaleFont};
use anyhow::{bail, Context, Result};
use bbb_client_assets::{PackResourceStack, PackRoots, ResourceLocation};

use super::providers::FontTrueTypeProviderDefinition;

const MAX_TTF_ASSET_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TTF_GLYPHS: usize = 1_114_112;
const TTF_RETAINED_BYTES_PER_GLYPH: u64 = 96;
const TTF_PAGE_MAX_DIMENSION: u32 = 8_192;
const MIN_TTF_OVERSAMPLE: f32 = 0.1;
const MAX_TTF_ADVANCE: f32 = 4_096.0;

/// A face or asset problem Vanilla answers by rejecting the provider (and with
/// it the font): a missing resource and the `FT_Get_Font_Format` /
/// `FT_Select_Charmap` assertions are all caught by `FontManager.safeLoad`.
#[derive(Debug)]
pub(super) struct TtfProviderInvalid(anyhow::Error);

impl std::fmt::Display for TtfProviderInvalid {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, formatter)
    }
}

impl std::error::Error for TtfProviderInvalid {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

fn invalid(message: String) -> anyhow::Error {
    TtfProviderInvalid(anyhow::anyhow!(message)).into()
}

/// Raster bounds for a TrueType glyph when its outline has non-zero area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TtfGlyphBounds {
    pub width: u32,
    pub height: u32,
}

/// One retained charmap entry plus the metrics
/// `TrueTypeGlyphProvider.loadGlyph` derives from it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TtfGlyph {
    pub codepoint: u32,
    pub glyph_id: u16,
    /// `GlyphInfo.simple(advance / oversample)`.
    pub advance: f32,
    /// `GlyphBitmap.getBearingLeft()` = `bitmap_left / oversample`.
    pub bearing_left: f32,
    /// `GlyphBitmap.getBearingTop()` = `bitmap_top / oversample`.
    pub bearing_top: f32,
    /// `None` where Vanilla answers `EmptyGlyph` (a zero-area bitmap).
    pub bounds: Option<TtfGlyphBounds>,
}

/// Lazy TrueType glyph source: retains validated font bytes and charmap metrics,
/// rasterizing individual glyph outlines on demand instead of baking an eager RGBA page.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct TtfGlyphSource {
    font_bytes: Arc<[u8]>,
    px_scale_x: f32,
    px_scale_y: f32,
    shift_x: f32,
    shift_y: f32,
    /// `GlyphBitmap.getOversample()`.
    pub oversample: f32,
    pub glyphs: Vec<TtfGlyph>,
}

impl TtfGlyphSource {
    pub(super) fn has_raster_glyphs(&self) -> bool {
        self.glyphs.iter().any(|glyph| {
            glyph
                .bounds
                .is_some_and(|bounds| bounds.width <= 256 && bounds.height <= 256)
        })
    }

    pub(super) fn rasterize_glyph_rgba(&self, index: usize) -> Vec<u8> {
        let glyph = self.glyphs[index];
        let Some(bounds) = glyph.bounds else {
            return Vec::new();
        };
        let mut rgba = vec![0_u8; (bounds.width as usize) * (bounds.height as usize) * 4];
        let Ok(font) = FontRef::try_from_slice(&self.font_bytes) else {
            return rgba;
        };
        let scale = PxScale {
            x: self.px_scale_x,
            y: self.px_scale_y,
        };
        let positioned = GlyphId(glyph.glyph_id)
            .with_scale_and_position(scale, point(self.shift_x, self.shift_y));
        if let Some(outlined) = font.outline_glyph(positioned) {
            blit_ttf_outlined(&outlined, bounds.width, bounds.height, &mut rgba);
        }
        rgba
    }
}

/// Vanilla `TrueTypeGlyphProviderDefinition.load`: the definition's `file`
/// gains a `font/` prefix (`location.withPrefix("font/")`) before lookup.
pub(super) fn load_ttf_glyph_source(
    roots: &PackRoots,
    definition: &FontTrueTypeProviderDefinition,
) -> Result<TtfGlyphSource> {
    let location = ResourceLocation::parse(&definition.file)?;
    let location =
        ResourceLocation::new(location.namespace(), format!("font/{}", location.path()))?;
    let stack = roots.resource_stack();
    let resource = stack
        .get_resource(&location)
        .ok_or_else(|| invalid(format!("missing font asset {}", location.id())))?;
    stack.validate_resource(&resource, "font asset")?;
    let bytes = resource.read_bounded_bytes(MAX_TTF_ASSET_BYTES, "font asset")?;
    build_ttf_glyph_source(Arc::from(bytes), definition, Some(&stack))
        .with_context(|| format!("load font asset {}", resource.path.display()))
}

fn build_ttf_glyph_source(
    bytes: Arc<[u8]>,
    definition: &FontTrueTypeProviderDefinition,
    budget: Option<&PackResourceStack>,
) -> Result<TtfGlyphSource> {
    let oversample = if definition.oversample.is_finite() {
        definition.oversample.max(MIN_TTF_OVERSAMPLE)
    } else {
        1.0
    };
    let face = Face::new(&bytes, definition, oversample)?;
    let skip = definition.skip.iter().copied().collect::<BTreeSet<_>>();
    let mut glyphs = Vec::new();
    for (codepoint, glyph_id) in face.charmap(MAX_TTF_GLYPHS)? {
        if skip.contains(&codepoint) {
            continue;
        }
        let prepared = face.prepare_glyph(glyph_id)?;
        if let Some(bounds) = prepared.bounds {
            if bounds.width > TTF_PAGE_MAX_DIMENSION || bounds.height > TTF_PAGE_MAX_DIMENSION {
                bail!("truetype glyphs exceed the {TTF_PAGE_MAX_DIMENSION} pixel atlas page limit");
            }
        }
        let has_bounds = prepared.bounds.is_some();
        let raw_advance = prepared.advance / oversample;
        let advance = if raw_advance.is_finite() {
            raw_advance.clamp(-MAX_TTF_ADVANCE, MAX_TTF_ADVANCE)
        } else {
            0.0
        };
        glyphs.push(TtfGlyph {
            codepoint,
            glyph_id: u16::try_from(glyph_id).context("truetype glyph id overflow")?,
            advance,
            bearing_left: if has_bounds {
                prepared.left as f32 / oversample
            } else {
                0.0
            },
            bearing_top: if has_bounds {
                prepared.top as f32 / oversample
            } else {
                0.0
            },
            bounds: prepared.bounds,
        });
    }
    if let Some(budget) = budget {
        let retained = u64::try_from(glyphs.len())
            .context("truetype glyph count overflow")?
            .checked_mul(TTF_RETAINED_BYTES_PER_GLYPH)
            .and_then(|bytes_used| bytes_used.checked_add(bytes.len() as u64))
            .context("truetype retained glyph byte count overflow")?;
        budget.reserve_derived_resource_bytes(retained, "retained truetype glyph catalog")?;
    }
    glyphs.shrink_to_fit();
    let px_scale = face.px_scale;
    let shift_x = face.shift_x;
    let shift_y = face.shift_y;
    Ok(TtfGlyphSource {
        font_bytes: bytes,
        px_scale_x: px_scale.x,
        px_scale_y: px_scale.y,
        shift_x,
        shift_y,
        oversample,
        glyphs,
    })
}

struct PreparedGlyph {
    advance: f32,
    left: i32,
    top: i32,
    bounds: Option<TtfGlyphBounds>,
}

struct Face<'a> {
    font: FontRef<'a>,
    px_scale: PxScale,
    shift_x: f32,
    shift_y: f32,
}

impl<'a> Face<'a> {
    fn new(
        bytes: &'a [u8],
        definition: &FontTrueTypeProviderDefinition,
        oversample: f32,
    ) -> Result<Self> {
        validate_sfnt_truetype_tables(bytes)?;
        let font = FontRef::try_from_slice(bytes)
            .map_err(|err| invalid(format!("initialize TrueType face: {err}")))?;
        let units_per_em = font
            .units_per_em()
            .ok_or_else(|| invalid("font asset has no units_per_em".to_string()))?;
        let pixels = java_round_float(definition.size * oversample).max(1) as f32;
        let px_scale = PxScale::from(pixels * (font.height_unscaled() / units_per_em));
        let shift_x = java_round_float(definition.shift_x * oversample * 64.0) as f32 / 64.0;
        let shift_y = java_round_float(definition.shift_y * oversample * 64.0) as f32 / 64.0;
        Ok(Self {
            font,
            px_scale,
            shift_x,
            shift_y,
        })
    }

    fn charmap(&self, limit: usize) -> Result<Vec<(u32, u32)>> {
        let mut entries: Vec<(u32, u32)> = self
            .font
            .codepoint_ids()
            .filter(|(glyph_id, _)| glyph_id.0 != 0)
            .map(|(glyph_id, ch)| (u32::from(ch), u32::from(glyph_id.0)))
            .collect();
        entries.sort_unstable_by_key(|(codepoint, _)| *codepoint);
        entries.dedup_by_key(|(codepoint, _)| *codepoint);
        if entries.len() > limit {
            bail!("font asset charmap exceeds the {limit} glyph scalar limit");
        }
        Ok(entries)
    }

    fn prepare_glyph(&self, index: u32) -> Result<PreparedGlyph> {
        let glyph_id = GlyphId(u16::try_from(index).context("truetype glyph id overflow")?);
        let scaled = self.font.as_scaled(self.px_scale);
        let advance = scaled.h_advance(glyph_id);
        let glyph =
            glyph_id.with_scale_and_position(self.px_scale, point(self.shift_x, self.shift_y));
        match self.font.outline_glyph(glyph) {
            Some(outlined) => {
                let bounds = outlined.px_bounds();
                let width = bounds.width().max(0.0) as u32;
                let height = bounds.height().max(0.0) as u32;
                let left = bounds.min.x.floor() as i32;
                let top = (-bounds.min.y.floor()) as i32;
                let bounds = (width > 0 && height > 0).then_some(TtfGlyphBounds { width, height });
                Ok(PreparedGlyph {
                    advance,
                    left,
                    top,
                    bounds,
                })
            }
            None => Ok(PreparedGlyph {
                advance,
                left: 0,
                top: 0,
                bounds: None,
            }),
        }
    }
}

fn validate_sfnt_truetype_tables(bytes: &[u8]) -> Result<()> {
    if bytes.len() < 12 || !matches!(&bytes[..4], [0, 1, 0, 0] | b"true") {
        return Err(invalid("font asset is not in TTF format".to_string()));
    }
    let num_tables = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
    let directory_end = 12_usize
        .checked_add(num_tables.saturating_mul(16))
        .filter(|&end| end <= bytes.len())
        .ok_or_else(|| invalid("initialize TrueType face: truncated sfnt directory".to_string()))?;

    let mut has_glyf = false;
    let mut cmap_slice: Option<&[u8]> = None;
    for record in bytes[12..directory_end].as_chunks::<16>().0 {
        let tag = &record[0..4];
        let offset = u32::from_be_bytes([record[8], record[9], record[10], record[11]]) as usize;
        let length = u32::from_be_bytes([record[12], record[13], record[14], record[15]]) as usize;
        let end = offset
            .checked_add(length)
            .filter(|&end| end <= bytes.len())
            .ok_or_else(|| invalid("initialize TrueType face: table out of bounds".to_string()))?;
        if tag == b"glyf" {
            has_glyf = true;
        } else if tag == b"cmap" {
            cmap_slice = Some(&bytes[offset..end]);
        }
    }
    if !has_glyf {
        return Err(invalid(
            "initialize TrueType face: missing glyf table".to_string(),
        ));
    }
    let cmap = cmap_slice
        .filter(|cmap| cmap.len() >= 4)
        .ok_or_else(|| invalid("font asset has no unicode character map".to_string()))?;
    let subtables = usize::from(u16::from_be_bytes([cmap[2], cmap[3]]));
    let records_end = 4_usize
        .checked_add(subtables.saturating_mul(8))
        .filter(|&end| end <= cmap.len())
        .ok_or_else(|| invalid("initialize TrueType face: truncated cmap table".to_string()))?;
    let has_unicode_cmap = cmap[4..records_end].as_chunks::<8>().0.iter().any(|rec| {
        let platform = u16::from_be_bytes([rec[0], rec[1]]);
        let encoding = u16::from_be_bytes([rec[2], rec[3]]);
        platform == 0 || (platform == 3 && matches!(encoding, 1 | 10))
    });
    if !has_unicode_cmap {
        return Err(invalid(
            "font asset has no unicode character map".to_string(),
        ));
    }
    Ok(())
}

fn blit_ttf_outlined(outlined: &OutlinedGlyph, width: u32, height: u32, rgba: &mut [u8]) {
    // `rendertype_text_intensity.fsh` samples `texture(...).rrrr * vertexColor`;
    // the shared HUD shader samples RGBA and multiplies by the tint, so the
    // equivalent texel is grey with coverage as alpha.
    let cell_w = width as usize;
    let cell_h = height as usize;
    outlined.draw(|x, y, c| {
        let x = x as usize;
        let y = y as usize;
        if x < cell_w && y < cell_h {
            let alpha = (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            let offset = (y * cell_w + x) * 4;
            rgba[offset..offset + 4].copy_from_slice(&[alpha; 4]);
        }
    });
}

/// Java `Math.round(float)`: `floor(value + 0.5)`, which differs from Rust's
/// half-away-from-zero `round` below zero.
fn java_round_float(value: f32) -> i32 {
    (f64::from(value) + 0.5).floor() as i32
}

#[cfg(test)]
pub(super) mod fixture;
#[cfg(test)]
mod tests;
