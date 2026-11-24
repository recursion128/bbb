use super::*;
use std::mem::size_of;

#[derive(Clone, Copy)]
pub(super) struct GlyphCandidate {
    pub(super) baked: HudDigitGlyph,
    pub(super) lazy_key: u32,
    advance: f32,
    bold_advance: f32,
}

impl GlyphCandidate {
    pub(super) fn new(glyph: HudDigitGlyph) -> Self {
        Self::new_lazy(glyph, 0)
    }

    pub(super) fn new_lazy(glyph: HudDigitGlyph, lazy_key: u32) -> Self {
        Self {
            baked: glyph,
            lazy_key,
            advance: glyph.advance,
            bold_advance: glyph.advance + glyph.bold_offset,
        }
    }

    fn is_fishy(self) -> bool {
        // FontSet deliberately lets NaN pass both comparisons.
        self.advance < 0.0
            || self.advance > 32.0
            || self.bold_advance < 0.0
            || self.bold_advance > 32.0
    }
}

pub(super) type ProviderGlyphs = Vec<(u32, GlyphCandidate)>;
type OrderedCandidate<'a> = (u32, usize, &'a GlyphCandidate);

pub(super) fn slot_count(entry: &FontAtlasEntry) -> usize {
    match entry {
        FontAtlasEntry::Missing => 0,
        FontAtlasEntry::Bitmap(definition, _) => definition.chars.iter().map(Vec::len).sum(),
        FontAtlasEntry::Space(definition) => definition.advances.len(),
        FontAtlasEntry::Unihex(page) => page.glyphs.len(),
        FontAtlasEntry::TrueType(page) => page.glyphs.len(),
    }
}

pub(super) fn admit_candidates(
    entries: &[FontAtlasEntry],
    budget: Option<&PackResourceStack>,
) -> Result<()> {
    let Some(budget) = budget else { return Ok(()) };
    let slots = entries.iter().try_fold(0_usize, |total, entry| {
        total
            .checked_add(slot_count(entry))
            .context("font selection slot count overflow")
    })?;
    // Provider vectors, stable-sort scratch, ordered references, and active-provider flags.
    let bytes = slots
        .checked_mul(2 * size_of::<(u32, GlyphCandidate)>() + size_of::<OrderedCandidate<'_>>())
        .and_then(|bytes| {
            entries
                .len()
                .checked_mul(size_of::<ProviderGlyphs>() + 1)
                .and_then(|outer| bytes.checked_add(outer))
        })
        .context("font selection candidate byte count overflow")?;
    budget.reserve_derived_resource_bytes(bytes as u64, "font selection candidates")
}

pub(super) fn select(
    providers: &[ProviderGlyphs],
    missing: HudDigitGlyph,
    budget: Option<&PackResourceStack>,
) -> Result<HudFontGlyphMap> {
    let count = providers.iter().map(Vec::len).sum();
    let mut ordered = Vec::with_capacity(count);
    for (index, glyphs) in providers.iter().enumerate() {
        ordered.extend(
            glyphs
                .iter()
                .map(|(codepoint, glyph)| (*codepoint, index, glyph)),
        );
    }
    ordered.sort_unstable_by_key(|(codepoint, index, _)| (*codepoint, *index));
    let mut active = vec![false; providers.len()];
    let mut unique = 0_usize;
    // FontSet keeps only providers supplying at least one first glyph.
    for group in ordered.chunk_by(|left, right| left.0 == right.0) {
        active[group[0].1] = true;
        unique += 1;
    }
    admit_map(unique, budget)?;
    let mut glyphs = HudFontGlyphMap::with_capacity(unique);
    glyphs.set_missing_glyph(missing);
    for group in ordered.chunk_by(|left, right| left.0 == right.0) {
        let (codepoint, _, first) = group[0];
        let (filtered, filtered_lazy_key) = group
            .iter()
            .find(|(_, provider, candidate)| active[*provider] && !candidate.is_fishy())
            .map_or((missing, 0), |(_, _, candidate)| {
                (candidate.baked, candidate.lazy_key)
            });
        glyphs.insert_selected_codepoint_with_lazy_keys(
            codepoint,
            first.baked,
            first.lazy_key,
            filtered,
            filtered_lazy_key,
            first.advance,
        );
    }
    glyphs.shrink_to_fit();
    Ok(glyphs)
}

fn admit_map(count: usize, budget: Option<&PackResourceStack>) -> Result<()> {
    let Some(budget) = budget else { return Ok(()) };
    // Both projections retain glyph and raw-width vectors, including Vec growth capacity.
    let capacity = count
        .checked_mul(2)
        .context("font projection capacity overflow")?
        .max(4);
    let bytes = capacity
        .checked_mul(2 * (size_of::<(u32, HudDigitGlyph)>() + size_of::<(u32, f32)>()))
        .and_then(|bytes| bytes.checked_add(2 * size_of::<HudFontGlyphMap>()))
        .context("font projection byte count overflow")?;
    budget.reserve_derived_resource_bytes(bytes as u64, "selected font glyph projections")
}

pub(super) fn admit_book(
    maps: &BTreeMap<String, HudFontGlyphMap>,
    budget: Option<&PackResourceStack>,
) -> Result<()> {
    let Some(budget) = budget else { return Ok(()) };
    let default = maps
        .get(DEFAULT_FONT_ID)
        .context("default font glyph map was not built")?;
    let mut bytes = default
        .retained_logical_bytes()
        .checked_mul(2)
        .context("default font projection byte count overflow")?;
    // The book and atlas each clone default; every book map also gets an obfuscation pool.
    for glyphs in maps.values().chain(std::iter::once(default)) {
        let capacity = glyphs
            .len()
            .checked_mul(2)
            .context("font pool capacity overflow")?
            .max(4);
        let pool_bytes = capacity
            .checked_mul(size_of::<(i32, Vec<HudDigitGlyph>)>() + 2 * size_of::<HudDigitGlyph>())
            .context("font pool byte count overflow")?;
        bytes = bytes
            .checked_add(pool_bytes)
            .context("font book byte count overflow")?;
    }
    for font_id in maps.keys() {
        let owners =
            2 * size_of::<String>() + size_of::<bbb_render_types::HudObfuscatedGlyphPool>() + 128;
        bytes = font_id
            .len()
            .checked_mul(2)
            .and_then(|names| names.checked_add(owners))
            .and_then(|owners| bytes.checked_add(owners))
            .context("font book map owner byte count overflow")?;
    }
    bytes = bytes
        .checked_add(size_of::<HudFontGlyphBook>())
        .context("font book inline byte count overflow")?;
    budget.reserve_derived_resource_bytes(bytes as u64, "retained font book projections and pools")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_fishy_candidate_and_projection_admissions_are_hard_and_sticky() {
        for candidate_admission in [true, false] {
            let budget = PackResourceStack::from_roots(std::iter::empty::<std::path::PathBuf>());
            let available = budget.resource_data_memory_observation().max_logical_bytes;
            budget
                .reserve_derived_resource_bytes(available - 1, "other retained resources")
                .unwrap();
            let error = if candidate_admission {
                admit_candidates(
                    &[FontAtlasEntry::Space(FontSpaceProviderDefinition {
                        advances: vec![(65, -1.0)],
                    })],
                    Some(&budget),
                )
                .unwrap_err()
            } else {
                admit_map(1, Some(&budget)).unwrap_err()
            };
            assert!(
                error.to_string().contains(if candidate_admission {
                    "font selection candidates"
                } else {
                    "selected font glyph projections"
                }),
                "{error:#}"
            );
            assert!(budget.ensure_resource_data_admitted().is_err());
        }
    }

    #[test]
    fn non_fishy_comparisons_preserve_nan_and_both_advance_boundaries() {
        for (advance, bold_offset, expected) in [
            (f32::NAN, 1.0, false),
            (0.0, 1.0, false),
            (-0.0, 1.0, false),
            (31.0, 1.0, false),
            (31.5, 0.5, false),
            (32.0, 0.0, false),
            (32.0, 0.5, true),
            (32.0, 1.0, true),
            (-0.1, 1.0, true),
            (f32::INFINITY, 1.0, true),
            (f32::NEG_INFINITY, 1.0, true),
        ] {
            let candidate = GlyphCandidate::new(HudDigitGlyph {
                advance,
                bold_offset,
                ..HudDigitGlyph::default()
            });
            assert_eq!(candidate.is_fishy(), expected, "{advance}, {bold_offset}");
        }
    }
}
