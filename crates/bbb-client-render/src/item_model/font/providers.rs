//! Parses the vanilla `font/default.json` provider chain
//! (`FontManager.FontDefinitionFile`) into a flattened, priority-ordered list
//! of provider definitions.
//!
//! `bitmap`, `space`, `unihex`, and `reference` providers are filtered by the
//! same `uniform` / `jp` option set that Vanilla passes to `FontSet.reload`.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

mod codec;
use codec::{Conditional, Definition};

use anyhow::{bail, Context, Result};
use bbb_client_assets::{PackRoots, ResourceJsonValue as Value, ResourceLocation};

const MAX_FONT_DEFINITIONS: u64 = 256;
const MAX_FONT_PROVIDERS: u64 = 4_096;
const MAX_FONT_BITMAP_GLYPH_SLOTS: u64 = 1_048_576;
const MAX_FONT_SPACE_ADVANCES: u64 = 65_536;
const MAX_FONT_UNIHEX_OVERRIDES: u64 = 4_096;
const FONT_DEFINITION_LOGICAL_BYTES: u64 = 512;
const FONT_PROVIDER_LOGICAL_BYTES: u64 = 256;
const FONT_GLYPH_SLOT_LOGICAL_BYTES: u64 = 64;

#[derive(Debug)]
struct UnsafeUnihexWidth {
    left: i32,
    right: i32,
}

impl std::fmt::Display for UnsafeUnihexWidth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unihex size override [{0};{1}] has unsafe width {2}",
            self.left,
            self.right,
            i64::from(self.right) - i64::from(self.left) + 1
        )
    }
}
impl std::error::Error for UnsafeUnihexWidth {}

/// `FontManager.getFontOptions`: the two options consulted by provider
/// `FontOption.Filter` values before reference expansion.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FontOptions {
    pub uniform: bool,
    pub japanese_variants: bool,
}

/// One flattened `"type": "bitmap"` provider, mirroring vanilla
/// `BitmapProvider.Definition` (codec fields `file`, `height` default 8,
/// `ascent`, `chars`).
#[derive(Debug, Clone, PartialEq)]
pub struct FontBitmapProviderDefinition {
    /// Texture id without the `textures/` prefix, e.g. `minecraft:font/ascii.png`
    /// (vanilla prepends `textures/` at load: `file.withPrefix("textures/")`).
    pub file: String,
    /// Rendered glyph height in font pixels (`Codec.INT.optionalFieldOf("height", 8)`).
    pub height: i32,
    /// Distance from the reference baseline up to the glyph cell top.
    pub ascent: i32,
    /// Codepoint grid rows; U+0000 marks an empty slot.
    pub chars: Vec<Vec<u32>>,
}

/// A space provider maps single codepoints to finite floating-point advances.
#[derive(Debug, Clone, PartialEq)]
pub struct FontSpaceProviderDefinition {
    pub advances: Vec<(u32, f32)>,
}

/// One inclusive codepoint range whose retained bitmap columns are forced by
/// `UnihexProvider.Definition.sizeOverrides`. Vanilla removes each matching
/// glyph from the automatic-width map in list order, so overlapping ranges are
/// deliberately retained in their source order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontUnihexSizeOverride {
    pub from: u32,
    pub to: u32,
    pub left: i32,
    pub right: i32,
}

/// One `UnihexProvider.Definition`: the resource id points directly at a ZIP
/// resource (there is no `textures/` prefix).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontUnihexProviderDefinition {
    pub hex_file: String,
    pub size_overrides: Vec<FontUnihexSizeOverride>,
}

/// One `"type": "ttf"` provider, mirroring vanilla
/// `TrueTypeGlyphProviderDefinition` (codec fields `file`, `size` default
/// 11.0, `oversample` default 1.0, `shift` default `[0, 0]`, `skip` default
/// `""`).
#[derive(Debug, Clone, PartialEq)]
pub struct FontTrueTypeProviderDefinition {
    /// Font asset id without the `font/` prefix, e.g. `minecraft:example.ttf`
    /// (vanilla prepends `font/` at load: `location.withPrefix("font/")`).
    pub file: String,
    pub size: f32,
    pub oversample: f32,
    pub shift_x: f32,
    pub shift_y: f32,
    /// The codepoints `TrueTypeGlyphProvider` removes from the face charmap
    /// (`skip.codePoints()`), after the codec joined a list form.
    pub skip: Vec<u32>,
}

/// One flattened provider in `providers` order, after `reference` expansion.
/// Vanilla `FontSet.computeGlyphInfo` walks providers of every type
/// (`bitmap`, `space`, `unihex`, `ttf`, ...) in this same flattened order and
/// takes the first one that supplies a codepoint, so callers must preserve
/// occurrence order across variants when merging glyphs.
#[derive(Debug, Clone, PartialEq)]
pub enum FontProviderDefinition {
    Bitmap(FontBitmapProviderDefinition),
    Space(FontSpaceProviderDefinition),
    Unihex(FontUnihexProviderDefinition),
    TrueType(FontTrueTypeProviderDefinition),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FontReferenceFailure {
    MissingDefinition(String),
    Cycle(String),
}

impl std::fmt::Display for FontReferenceFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingDefinition(id) => write!(f, "missing font definition {id}"),
            Self::Cycle(id) => write!(f, "font reference cycle at {id}"),
        }
    }
}

impl std::error::Error for FontReferenceFailure {}

#[derive(Debug)]
pub(super) struct FontCandidate {
    pub(super) definition: Arc<FontProviderDefinition>,
    pub(super) enabled: bool,
}

#[derive(Debug)]
pub(super) struct FontCandidates {
    pub(super) providers: Vec<FontCandidate>,
    pub(super) reference_failure: Option<FontReferenceFailure>,
}

impl FontCandidates {
    fn enabled_definitions(self) -> Result<Vec<FontProviderDefinition>> {
        if let Some(failure) = self.reference_failure {
            return Err(failure.into());
        }
        Ok(self
            .providers
            .into_iter()
            .filter(|entry| entry.enabled)
            .map(|entry| (*entry.definition).clone())
            .collect())
    }
}

#[derive(Clone, Default)]
struct ResolvedDefinition {
    providers: Vec<Conditional>,
    reference_failure: Option<FontReferenceFailure>,
}

/// Loads and flattens the providers of `font_id` (e.g. `minecraft:default`)
/// through the resource stack, recursing `reference` providers depth-first in
/// place like vanilla `FontManager`'s builder stack, which preserves the
/// referencing file's provider order.
pub fn load_font_providers(
    roots: &PackRoots,
    font_id: &str,
) -> Result<Vec<FontProviderDefinition>> {
    load_font_providers_with_options(roots, font_id, FontOptions::default())
}

pub fn load_font_providers_with_options(
    roots: &PackRoots,
    font_id: &str,
    options: FontOptions,
) -> Result<Vec<FontProviderDefinition>> {
    load_font_candidates(roots, font_id, options)?.enabled_definitions()
}

pub(super) fn load_font_candidates(
    roots: &PackRoots,
    font_id: &str,
    options: FontOptions,
) -> Result<FontCandidates> {
    let stack = roots.resource_stack();
    let mut definition_count = 0_u64;
    let mut raw_counts = [0_u64; 4];
    let mut load_definition = |id: &str| -> Result<Vec<Conditional>> {
        let location = font_definition_location(id)?;
        let resources = stack.get_resource_stack(&location);
        if resources.is_empty() {
            return Err(FontReferenceFailure::MissingDefinition(id.to_string()).into());
        }
        let mut providers = Vec::new();
        // FontManager reverses each file; apply:211-214 reverses the stack.
        for resource in resources.into_iter().rev() {
            definition_count = definition_count
                .checked_add(1)
                .context("font definition count overflow")?;
            stack.admit_resource_collection_count(
                definition_count,
                MAX_FONT_DEFINITIONS,
                "font definitions",
            )?;
            let value: Value = match stack.read_resource_json(&resource, "font definition") {
                Ok(value) => value,
                Err(error)
                    if error
                        .downcast_ref::<serde_json::Error>()
                        .is_some_and(|error| !error.is_io()) =>
                {
                    tracing::warn!(font = id, path = %resource.path.display(), %error, "skipping invalid font JSON");
                    continue;
                }
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("read font definition {}", resource.path.display())
                    })
                }
            };
            admit_raw_definition(&stack, &value, &mut raw_counts)?;
            match codec::decode(id, &value) {
                Ok(entries) => providers.extend(entries),
                Err(error) if error.is::<UnsafeUnihexWidth>() => return Err(error),
                Err(error) => {
                    tracing::warn!(font = id, path = %resource.path.display(), %error, "skipping invalid font definition")
                }
            }
        }
        Ok(providers)
    };
    let providers = flatten_definitions(font_id, options, &mut load_definition, Some(&stack))?;
    admit_font_collections(&stack, definition_count, &providers.providers)?;
    Ok(providers)
}

/// Resolves a font id to its definition asset path, e.g.
/// `minecraft:include/default` -> `font/include/default.json` (vanilla
/// `FontManager.FONT_DEFINITIONS.idToFile`).
fn font_definition_location(id: &str) -> Result<ResourceLocation> {
    let location = ResourceLocation::parse(id)?;
    ResourceLocation::new(
        location.namespace(),
        format!("font/{}.json", location.path()),
    )
}

#[cfg(test)]
pub(crate) fn flatten_font_providers(
    font_id: &str,
    load_definition: &mut dyn FnMut(&str) -> Result<String>,
) -> Result<Vec<FontProviderDefinition>> {
    flatten_font_providers_with_options(font_id, FontOptions::default(), load_definition)
}

#[cfg(test)]
fn flatten_font_providers_with_options(
    font_id: &str,
    options: FontOptions,
    load_definition: &mut dyn FnMut(&str) -> Result<String>,
) -> Result<Vec<FontProviderDefinition>> {
    let mut load_value = |id: &str| -> Result<Vec<Conditional>> {
        let json = load_definition(id)?;
        let value = Value::from_slice(json.as_bytes())
            .with_context(|| format!("parse font definition {id}"))?;
        codec::decode(id, &value)
    };
    flatten_definitions(font_id, options, &mut load_value, None)?.enabled_definitions()
}

fn flatten_definitions(
    font_id: &str,
    options: FontOptions,
    load: &mut dyn FnMut(&str) -> Result<Vec<Conditional>>,
    stack: Option<&bbb_client_assets::PackResourceStack>,
) -> Result<FontCandidates> {
    // Resolve before filtering: a disabled unresolved reference still invalidates the font.
    let resolved = resolve_definition(
        font_id,
        load,
        &mut HashSet::new(),
        &mut HashMap::new(),
        stack,
    )?;
    let mut counts = [0usize; 3];
    for entry in &resolved.providers {
        if let Definition::Raster(provider) = &entry.provider {
            match provider.as_ref() {
                FontProviderDefinition::Bitmap(bitmap) => {
                    counts[0] += bitmap.chars.iter().map(Vec::len).sum::<usize>()
                }
                FontProviderDefinition::Space(space) => counts[1] += space.advances.len(),
                FontProviderDefinition::Unihex(unihex) => counts[2] += unihex.size_overrides.len(),
                // A TTF provider's slots are its face charmap, admitted at load.
                FontProviderDefinition::TrueType(_) => {}
            }
        }
    }
    if counts[0] > MAX_FONT_BITMAP_GLYPH_SLOTS as usize
        || counts[1] > MAX_FONT_SPACE_ADVANCES as usize
        || counts[2] > MAX_FONT_UNIHEX_OVERRIDES as usize
    {
        bail!("font expanded provider collection limit exceeded");
    }
    let mut providers = Vec::new();
    for entry in resolved.providers {
        match entry.provider {
            Definition::Raster(definition) => providers.push(FontCandidate {
                definition,
                enabled: entry.filter.enabled(options),
            }),
            Definition::Reference(_) => unreachable!("references resolved"),
        }
    }
    Ok(FontCandidates {
        providers,
        reference_failure: resolved.reference_failure,
    })
}

fn resolve_definition(
    font_id: &str,
    load: &mut dyn FnMut(&str) -> Result<Vec<Conditional>>,
    visiting: &mut HashSet<String>,
    cache: &mut HashMap<String, ResolvedDefinition>,
    stack: Option<&bbb_client_assets::PackResourceStack>,
) -> Result<ResolvedDefinition> {
    let font_id = ResourceLocation::parse(font_id)?.id();
    if let Some(cached) = cache.get(&font_id) {
        return Ok(cached.clone());
    }
    if !visiting.insert(font_id.clone()) {
        return Ok(ResolvedDefinition {
            reference_failure: Some(FontReferenceFailure::Cycle(font_id)),
            ..ResolvedDefinition::default()
        });
    }
    if visiting.len() + cache.len() > MAX_FONT_DEFINITIONS as usize {
        bail!("font provider graph exceeds the {MAX_FONT_DEFINITIONS} definition limit");
    }
    let mut resolved = ResolvedDefinition::default();
    let entries = match load(&font_id) {
        Ok(entries) => entries,
        Err(error) => {
            let failure = error.downcast::<FontReferenceFailure>()?;
            resolved.reference_failure = Some(failure);
            Vec::new()
        }
    };
    // Failed references invalidate the font, but siblings still owe resource and budget checks.
    for entry in entries {
        match entry.provider {
            Definition::Reference(id) => {
                let referenced = resolve_definition(&id, load, visiting, cache, stack)?;
                if resolved.reference_failure.is_none() {
                    resolved.reference_failure = referenced.reference_failure;
                }
                for inner in referenced.providers {
                    resolved.providers.push(Conditional {
                        provider: inner.provider,
                        filter: entry.filter.merge(inner.filter),
                    });
                }
            }
            _ => resolved.providers.push(entry),
        }
        if resolved.providers.len() > MAX_FONT_PROVIDERS as usize {
            bail!("font provider graph exceeds the {MAX_FONT_PROVIDERS} provider limit");
        }
    }
    visiting.remove(&font_id);
    if let Some(stack) = stack {
        stack.reserve_derived_resource_bytes(
            (resolved.providers.len() as u64)
                .checked_mul(FONT_PROVIDER_LOGICAL_BYTES)
                .context("font resolved cache byte count overflow")?,
            "resolved font reference cache",
        )?;
    }
    cache.insert(font_id, resolved.clone());
    Ok(resolved)
}

fn admit_raw_definition(
    stack: &bbb_client_assets::PackResourceStack,
    root: &Value,
    totals: &mut [u64; 4],
) -> Result<()> {
    let Some(entries) = root.get("providers").and_then(Value::as_array) else {
        return Ok(());
    };
    let mut counts = [entries.len() as u64, 0, 0, 0];
    for entry in entries {
        if let Some(rows) = entry.get("chars").and_then(Value::as_array) {
            for row in rows {
                counts[1] += row
                    .as_string()
                    .map_or(0, |row| row.codepoints().count() as u64);
            }
        }
        counts[2] += entry
            .get("advances")
            .and_then(Value::as_object)
            .map_or(0, |entries| entries.len() as u64);
        counts[3] += entry
            .get("size_overrides")
            .and_then(Value::as_array)
            .map_or(0, |entries| entries.len() as u64);
    }
    for (index, (limit, label)) in [
        (MAX_FONT_PROVIDERS, "font providers"),
        (MAX_FONT_BITMAP_GLYPH_SLOTS, "font bitmap glyph slots"),
        (MAX_FONT_SPACE_ADVANCES, "font space advances"),
        (MAX_FONT_UNIHEX_OVERRIDES, "font unihex size overrides"),
    ]
    .into_iter()
    .enumerate()
    {
        totals[index] = totals[index]
            .checked_add(counts[index])
            .context("font collection count overflow")?;
        stack.admit_resource_collection_count(totals[index], limit, label)?;
    }
    admit_font_collection_counts(stack, 1, counts[0], counts[1], counts[2], counts[3])
}

fn admit_font_collection_counts(
    stack: &bbb_client_assets::PackResourceStack,
    definition_count: u64,
    provider_count: u64,
    glyph_slots: u64,
    space_advances: u64,
    unihex_overrides: u64,
) -> Result<()> {
    stack.admit_resource_collection_count(
        definition_count,
        MAX_FONT_DEFINITIONS,
        "font definitions",
    )?;
    stack.admit_resource_collection_count(provider_count, MAX_FONT_PROVIDERS, "font providers")?;
    stack.admit_resource_collection_count(
        glyph_slots,
        MAX_FONT_BITMAP_GLYPH_SLOTS,
        "font bitmap glyph slots",
    )?;
    stack.admit_resource_collection_count(
        space_advances,
        MAX_FONT_SPACE_ADVANCES,
        "font space advances",
    )?;
    stack.admit_resource_collection_count(
        unihex_overrides,
        MAX_FONT_UNIHEX_OVERRIDES,
        "font unihex size overrides",
    )?;
    let logical_bytes = definition_count
        .checked_mul(FONT_DEFINITION_LOGICAL_BYTES)
        .and_then(|bytes| {
            provider_count
                .checked_mul(FONT_PROVIDER_LOGICAL_BYTES)
                .and_then(|providers| bytes.checked_add(providers))
        })
        .and_then(|bytes| {
            glyph_slots
                .checked_add(space_advances)
                .and_then(|glyphs| glyphs.checked_add(unihex_overrides))
                .and_then(|glyphs| glyphs.checked_mul(FONT_GLYPH_SLOT_LOGICAL_BYTES))
                .and_then(|glyphs| bytes.checked_add(glyphs))
        })
        .context("font derived logical byte count overflow")?;
    stack.reserve_derived_resource_bytes(logical_bytes, "retained font provider catalog")
}

fn admit_font_collections(
    stack: &bbb_client_assets::PackResourceStack,
    definition_count: u64,
    providers: &[FontCandidate],
) -> Result<()> {
    let provider_count = u64::try_from(providers.len()).context("font provider count overflow")?;
    let mut glyph_slots = 0_u64;
    let mut space_advances = 0_u64;
    let mut unihex_overrides = 0_u64;
    for provider in providers {
        match provider.definition.as_ref() {
            FontProviderDefinition::Bitmap(bitmap) => {
                for row in &bitmap.chars {
                    glyph_slots = glyph_slots
                        .checked_add(
                            u64::try_from(row.len()).context("font glyph slot count overflow")?,
                        )
                        .context("font glyph slot count overflow")?;
                }
            }
            FontProviderDefinition::Space(space) => {
                space_advances = space_advances
                    .checked_add(
                        u64::try_from(space.advances.len())
                            .context("font space advance count overflow")?,
                    )
                    .context("font space advance count overflow")?;
            }
            FontProviderDefinition::Unihex(unihex) => {
                unihex_overrides = unihex_overrides
                    .checked_add(
                        u64::try_from(unihex.size_overrides.len())
                            .context("font unihex override count overflow")?,
                    )
                    .context("font unihex override count overflow")?;
            }
            // A TTF provider's glyph slots come from its face charmap, which
            // only the loaded asset can answer.
            FontProviderDefinition::TrueType(_) => {}
        }
    }
    admit_font_collection_counts(
        stack,
        definition_count,
        provider_count,
        glyph_slots,
        space_advances,
        unihex_overrides,
    )
}

fn parse_bitmap_provider(font_id: &str, provider: &Value) -> Result<FontBitmapProviderDefinition> {
    let file = provider
        .get("file")
        .and_then(Value::as_str)
        .with_context(|| format!("bitmap font provider in {font_id} has no \"file\""))?
        .to_string();
    ResourceLocation::parse(&file)?;
    // BitmapProvider.Definition.CODEC: `Codec.INT.optionalFieldOf("height", 8)`.
    let height = match provider.get("height") {
        None => 8,
        Some(value) => parse_provider_int(value)
            .with_context(|| format!("bitmap font provider {file} has a non-integer \"height\""))?,
    };
    let ascent = provider
        .get("ascent")
        .context("bitmap font provider has no \"ascent\"")
        .and_then(parse_provider_int)
        .with_context(|| format!("bitmap font provider {file} needs an integer \"ascent\""))?;
    // BitmapProvider.Definition.validate: `ascent > height` is rejected.
    if ascent > height {
        bail!("bitmap font provider {file} ascent {ascent} is higher than height {height}");
    }
    let rows = provider
        .get("chars")
        .and_then(Value::as_array)
        .with_context(|| format!("bitmap font provider {file} has no \"chars\" array"))?;
    let chars = rows
        .iter()
        .map(|row| {
            row.as_string()
                .map(|row| row.codepoints().collect::<Vec<u32>>())
                .with_context(|| format!("bitmap font provider {file} has a non-string chars row"))
        })
        .collect::<Result<Vec<_>>>()?;
    // BitmapProvider.Definition.validateDimensions: non-empty grid with
    // uniform row codepoint counts.
    let Some(first_row) = chars.first() else {
        bail!("bitmap font provider {file} has an empty codepoint grid");
    };
    if first_row.is_empty() {
        bail!("bitmap font provider {file} has an empty codepoint grid row");
    }
    if chars.iter().any(|row| row.len() != first_row.len()) {
        bail!(
            "bitmap font provider {file} has codepoint grid rows of different lengths, pad with \\u0000"
        );
    }
    Ok(FontBitmapProviderDefinition {
        file,
        height,
        ascent,
        chars,
    })
}

const MAX_SPACE_ADVANCE: f32 = 4_096.0;
const MIN_TTF_OVERSAMPLE: f32 = 0.1;

fn parse_space_provider(font_id: &str, provider: &Value) -> Result<FontSpaceProviderDefinition> {
    let advances = provider
        .get("advances")
        .and_then(Value::as_object)
        .with_context(|| format!("space font provider in {font_id} has no \"advances\" object"))?;
    let mut parsed = Vec::with_capacity(advances.len());
    for (key, value) in advances {
        let mut chars = key.codepoints();
        let codepoint = chars.next().with_context(|| {
            format!("space font provider in {font_id} has an empty advances key")
        })?;
        if chars.next().is_some() {
            bail!(
                "space font provider in {font_id} advances key {key:?} is not a single codepoint"
            );
        }
        let advance = value.as_f64().with_context(|| {
            format!("space font provider in {font_id} has a non-numeric advance for {key:?}")
        })?;
        let finite_advance = advance as f32;
        if !advance.is_finite() || !finite_advance.is_finite() {
            bail!("space font provider in {font_id} advance {advance} for {key:?} is not a finite f32");
        }
        let clamped_advance = finite_advance.clamp(-MAX_SPACE_ADVANCE, MAX_SPACE_ADVANCE);
        parsed.push((codepoint, clamped_advance));
    }
    Ok(FontSpaceProviderDefinition { advances: parsed })
}

fn parse_unihex_provider(font_id: &str, provider: &Value) -> Result<FontUnihexProviderDefinition> {
    let hex_file = provider
        .get("hex_file")
        .and_then(Value::as_str)
        .with_context(|| format!("unihex font provider in {font_id} has no \"hex_file\""))?;
    ResourceLocation::parse(hex_file)
        .with_context(|| format!("unihex font provider in {font_id} has invalid hex_file"))?;
    let overrides = match provider.get("size_overrides") {
        None => &[][..],
        Some(value) => value.as_array().map(Vec::as_slice).with_context(|| {
            format!("unihex font provider {hex_file} has non-array size_overrides")
        })?,
    };
    let mut size_overrides = Vec::with_capacity(overrides.len());
    for value in overrides {
        let from = parse_codepoint_field(value, "from", hex_file)?;
        let to = parse_codepoint_field(value, "to", hex_file)?;
        if from >= to {
            bail!("unihex font provider {hex_file} has invalid override range [{from};{to}]");
        }
        let left = value
            .get("left")
            .context("unihex size override has no left")
            .and_then(parse_provider_int)?;
        let right = value
            .get("right")
            .context("unihex size override has no right")
            .and_then(parse_provider_int)?;
        // UnihexProvider accepts arbitrary ints; this is a BBB allocation limit.
        let width = i64::from(right) - i64::from(left) + 1;
        if !(1..=64).contains(&width) {
            return Err(UnsafeUnihexWidth { left, right }.into());
        }
        size_overrides.push(FontUnihexSizeOverride {
            from,
            to,
            left,
            right,
        });
    }
    Ok(FontUnihexProviderDefinition {
        hex_file: hex_file.to_string(),
        size_overrides,
    })
}

fn parse_codepoint_field(value: &Value, field: &str, hex_file: &str) -> Result<u32> {
    let text = value
        .get(field)
        .and_then(Value::as_string)
        .with_context(|| format!("unihex size override in {hex_file} has no {field:?}"))?;
    let mut chars = text.codepoints();
    let codepoint = chars
        .next()
        .with_context(|| format!("unihex size override {field} is empty"))?;
    if chars.next().is_some() {
        bail!("unihex size override {field} {text:?} is not one codepoint");
    }
    Ok(codepoint)
}

/// `TrueTypeGlyphProviderDefinition.CODEC`: `file` is the font asset id,
/// `size`/`oversample` are floats, `shift` is a two-element float list within
/// `[-512, 512]`, and `skip` is either a string or a string list joined before
/// `skip.codePoints()` is taken.
fn parse_ttf_provider(font_id: &str, provider: &Value) -> Result<FontTrueTypeProviderDefinition> {
    let file = provider
        .get("file")
        .and_then(Value::as_str)
        .with_context(|| format!("ttf font provider in {font_id} has no \"file\""))?
        .to_string();
    ResourceLocation::parse(&file)?;
    let size = ttf_float(provider, "size", 11.0, &file)?.max(0.1);
    let oversample = ttf_float(provider, "oversample", 1.0, &file)?.max(MIN_TTF_OVERSAMPLE);
    let (shift_x, shift_y) = match provider.get("shift") {
        None => (0.0, 0.0),
        Some(value) => {
            let shift = value
                .as_array()
                .with_context(|| format!("ttf font provider {file} has a non-list \"shift\""))?;
            let [shift_x, shift_y] = shift.as_slice() else {
                bail!("ttf font provider {file} shift must be a pair");
            };
            let (shift_x, shift_y) = (
                ttf_float_value(shift_x, "shift", &file)?,
                ttf_float_value(shift_y, "shift", &file)?,
            );
            for value in [shift_x, shift_y] {
                if !(-512.0..=512.0).contains(&value) {
                    bail!("ttf font provider {file} shift {value} is outside [-512, 512]");
                }
            }
            (shift_x, shift_y)
        }
    };
    let skip = match provider.get("skip") {
        None => Vec::new(),
        Some(value) => match value.as_string() {
            Some(text) => text.codepoints().collect(),
            None => {
                let entries = value.as_array().with_context(|| {
                    format!("ttf font provider {file} skip must be a string or a string list")
                })?;
                let mut skip = Vec::new();
                for entry in entries {
                    let text = entry.as_string().with_context(|| {
                        format!("ttf font provider {file} has a non-string skip entry")
                    })?;
                    skip.extend(text.codepoints());
                }
                skip
            }
        },
    };
    Ok(FontTrueTypeProviderDefinition {
        file,
        size,
        oversample,
        shift_x,
        shift_y,
        skip,
    })
}

fn ttf_float(provider: &Value, field: &str, default: f32, file: &str) -> Result<f32> {
    match provider.get(field) {
        None => Ok(default),
        Some(value) => ttf_float_value(value, field, file),
    }
}

/// `Codec.FLOAT`: the JSON number, refused when it is not a finite `f32`.
fn ttf_float_value(value: &Value, field: &str, file: &str) -> Result<f32> {
    let number = value.as_f64().map(|number| number as f32);
    number
        .filter(|number| number.is_finite())
        .with_context(|| format!("ttf font provider {file} has a non-float {field:?}"))
}

fn parse_provider_int(value: &Value) -> Result<i32> {
    value
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())
        .context("expected an integer")
}

#[cfg(test)]
mod resource_stack_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn selected_resource_generation_applies_font_pack_precedence() {
        let root = unique_temp_dir("selected-font-generation");
        let low = root.join("packs/low");
        let high = root.join("packs/high");
        std::fs::create_dir_all(root.join("sources/26.1/assets/minecraft")).unwrap();
        for pack in [&low, &high] {
            write_file(
                &pack.join("pack.mcmeta"),
                br#"{"pack":{"description":"test","min_format":[84,0],"max_format":[84,0]}}"#,
            );
        }
        write_file(
            &low.join("assets/example/font/default.json"),
            br#"{"providers":[{"type":"space","advances":{" ":3}}]}"#,
        );
        write_file(
            &high.join("assets/example/font/default.json"),
            br#"{"providers":[{"type":"space","advances":{" ":5}}]}"#,
        );
        let roots = PackRoots::from_root(&root).unwrap();
        let selected =
            bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&roots, [low, high])
                .unwrap();
        let roots = roots.with_client_resource_generation(41, selected).unwrap();

        let providers = load_font_providers(&roots, "example:default").unwrap();

        assert_eq!(roots.client_resource_generation().unwrap().generation(), 41);
        let FontProviderDefinition::Space(space) = &providers[0] else {
            panic!("expected selected space provider");
        };
        assert_eq!(space.advances, vec![(u32::from(' '), 5.0)]);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn font_reference_graph_accepts_exact_limit_and_rejects_next_definition() {
        let mut exact = reference_chain_loader(MAX_FONT_DEFINITIONS);
        assert!(flatten_font_providers("minecraft:0", &mut exact).is_ok());

        let mut overflow = reference_chain_loader(MAX_FONT_DEFINITIONS + 1);
        let error = flatten_font_providers("minecraft:0", &mut overflow)
            .unwrap_err()
            .to_string();
        assert!(error.contains("256 definition limit"), "{error}");
    }

    #[test]
    fn font_semantic_counts_accept_exact_limits_and_reject_overflow() {
        let exact = bbb_client_assets::PackResourceStack::default();
        admit_font_collection_counts(
            &exact,
            MAX_FONT_DEFINITIONS,
            MAX_FONT_PROVIDERS,
            MAX_FONT_BITMAP_GLYPH_SLOTS,
            MAX_FONT_SPACE_ADVANCES,
            MAX_FONT_UNIHEX_OVERRIDES,
        )
        .unwrap();
        assert!(exact.ensure_resource_data_admitted().is_ok());

        for (label, counts) in [
            ("definitions", (MAX_FONT_DEFINITIONS + 1, 0, 0, 0, 0)),
            ("providers", (0, MAX_FONT_PROVIDERS + 1, 0, 0, 0)),
            ("glyph slots", (0, 0, MAX_FONT_BITMAP_GLYPH_SLOTS + 1, 0, 0)),
            ("space advances", (0, 0, 0, MAX_FONT_SPACE_ADVANCES + 1, 0)),
            (
                "unihex size overrides",
                (0, 0, 0, 0, MAX_FONT_UNIHEX_OVERRIDES + 1),
            ),
        ] {
            let stack = bbb_client_assets::PackResourceStack::default();
            let error = admit_font_collection_counts(
                &stack, counts.0, counts.1, counts.2, counts.3, counts.4,
            )
            .unwrap_err()
            .to_string();
            assert!(error.contains(label), "{error}");
            assert!(stack.ensure_resource_data_admitted().is_err());
        }
    }

    fn reference_chain_loader(definitions: u64) -> impl FnMut(&str) -> Result<String> {
        move |id: &str| {
            let index = id
                .strip_prefix("minecraft:")
                .context("reference fixture namespace")?
                .parse::<u64>()?;
            if index + 1 < definitions {
                Ok(format!(
                    r#"{{"providers":[{{"type":"reference","id":"minecraft:{}"}}]}}"#,
                    index + 1
                ))
            } else {
                Ok(r#"{"providers":[]}"#.to_string())
            }
        }
    }

    fn test_loader(definitions: &[(&str, &str)]) -> impl FnMut(&str) -> Result<String> {
        let definitions: HashMap<String, String> = definitions
            .iter()
            .map(|(id, json)| (id.to_string(), json.to_string()))
            .collect();
        move |id: &str| {
            definitions
                .get(id)
                .cloned()
                .with_context(|| format!("missing font definition {id}"))
        }
    }

    fn write_file(path: &Path, payload: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, payload).unwrap();
    }

    pub(super) fn unique_temp_dir(label: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "bbb-client-item-model-font-{label}-{}-{nonce}-{sequence}",
            std::process::id()
        ))
    }

    /// Test convenience for fixtures that only exercise bitmap providers:
    /// flattens and asserts every entry is a `Bitmap` variant.
    fn flatten_bitmaps(
        font_id: &str,
        load_definition: &mut dyn FnMut(&str) -> Result<String>,
    ) -> Result<Vec<FontBitmapProviderDefinition>> {
        Ok(flatten_font_providers(font_id, load_definition)?
            .into_iter()
            .map(|provider| match provider {
                FontProviderDefinition::Bitmap(bitmap) => bitmap,
                FontProviderDefinition::Space(_) => {
                    panic!("expected only bitmap providers in this fixture")
                }
                FontProviderDefinition::Unihex(_) => {
                    panic!("expected only bitmap providers in this fixture")
                }
                FontProviderDefinition::TrueType(_) => {
                    panic!("expected only bitmap providers in this fixture")
                }
            })
            .collect())
    }

    #[test]
    fn flattens_vanilla_reference_chain_in_provider_order() {
        // Mirrors the vanilla asset shape: font/default.json references
        // include/space, include/default (filter uniform:false), and
        // include/unifont; include/default lists nonlatin_european, accented,
        // then ascii bitmap providers in that priority order.
        let mut load = test_loader(&[
            (
                "minecraft:default",
                r#"{"providers":[
                    {"type":"reference","id":"minecraft:include/space"},
                    {"type":"reference","id":"minecraft:include/default","filter":{"uniform":false}},
                    {"type":"reference","id":"minecraft:include/unifont"}
                ]}"#,
            ),
            (
                "minecraft:include/space",
                r#"{"providers":[{"type":"space","advances":{" ":4,"\u200c":0}}]}"#,
            ),
            (
                "minecraft:include/default",
                r#"{"providers":[
                    {"type":"bitmap","file":"minecraft:font/nonlatin_european.png","ascent":7,"chars":["λж"]},
                    {"type":"bitmap","file":"minecraft:font/accented.png","height":12,"ascent":10,"chars":["éü"]},
                    {"type":"bitmap","file":"minecraft:font/ascii.png","ascent":7,"chars":["Aé"]}
                ]}"#,
            ),
            (
                "minecraft:include/unifont",
                r#"{"providers":[{"type":"unihex","hex_file":"minecraft:font/unifont.zip","size_overrides":[]}]}"#,
            ),
        ]);

        let providers = flatten_font_providers("minecraft:default", &mut load).unwrap();

        // The space provider is first (matching include/space's reference
        // position ahead of include/default), followed by the three bitmap
        // pages in include/default's priority order and the default unihex
        // provider (the jp=true provider is filtered out).
        let FontProviderDefinition::Space(space) = &providers[0] else {
            panic!("expected the space provider first");
        };
        assert_eq!(
            space.advances,
            vec![(u32::from(' '), 4.0), (u32::from('\u{200c}'), 0.0)]
        );

        let bitmaps: Vec<&FontBitmapProviderDefinition> = providers[1..4]
            .iter()
            .map(|provider| match provider {
                FontProviderDefinition::Bitmap(bitmap) => bitmap,
                FontProviderDefinition::Space(_) => panic!("unexpected second space provider"),
                FontProviderDefinition::Unihex(_) => panic!("unexpected early unihex provider"),
                FontProviderDefinition::TrueType(_) => panic!("unexpected truetype provider"),
            })
            .collect();
        assert_eq!(
            bitmaps
                .iter()
                .map(|provider| (provider.file.as_str(), provider.height, provider.ascent))
                .collect::<Vec<_>>(),
            vec![
                ("minecraft:font/nonlatin_european.png", 8, 7),
                ("minecraft:font/accented.png", 12, 10),
                ("minecraft:font/ascii.png", 8, 7),
            ]
        );
        assert_eq!(bitmaps[0].chars, vec![vec![u32::from('λ'), u32::from('ж')]]);
        assert_eq!(bitmaps[1].chars, vec![vec![u32::from('é'), u32::from('ü')]]);
        let FontProviderDefinition::Unihex(unihex) = &providers[4] else {
            panic!("expected default unihex provider last");
        };
        assert_eq!(unihex.hex_file, "minecraft:font/unifont.zip");
    }

    #[test]
    fn font_options_filter_providers_before_reference_expansion() {
        let mut load = test_loader(&[
            (
                "minecraft:default",
                r#"{"providers":[
                    {"type":"reference","id":"minecraft:uniform","filter":{"uniform":true}},
                    {"type":"reference","id":"minecraft:normal","filter":{"uniform":false}},
                    {"type":"unihex","hex_file":"minecraft:font/jp.zip","filter":{"jp":true}},
                    {"type":"unihex","hex_file":"minecraft:font/default.zip"}
                ]}"#,
            ),
            (
                "minecraft:uniform",
                r#"{"providers":[{"type":"space","advances":{" ":99}}]}"#,
            ),
            (
                "minecraft:normal",
                r#"{"providers":[{"type":"space","advances":{" ":4}}]}"#,
            ),
        ]);

        let providers = flatten_font_providers("minecraft:default", &mut load).unwrap();
        assert_eq!(providers.len(), 2);
        let FontProviderDefinition::Space(space) = &providers[0] else {
            panic!("normal reference should pass");
        };
        assert_eq!(space.advances, vec![(u32::from(' '), 4.0)]);
        let FontProviderDefinition::Unihex(unihex) = &providers[1] else {
            panic!("default unihex should pass");
        };
        assert_eq!(unihex.hex_file, "minecraft:font/default.zip");

        let mut load = test_loader(&[
            (
                "minecraft:default",
                r#"{"providers":[
                    {"type":"reference","id":"minecraft:uniform","filter":{"uniform":true}},
                    {"type":"reference","id":"minecraft:normal","filter":{"uniform":false}},
                    {"type":"unihex","hex_file":"minecraft:font/jp.zip","filter":{"jp":true}},
                    {"type":"unihex","hex_file":"minecraft:font/default.zip","filter":{"jp":false}}
                ]}"#,
            ),
            (
                "minecraft:uniform",
                r#"{"providers":[{"type":"space","advances":{" ":99}}]}"#,
            ),
            (
                "minecraft:normal",
                r#"{"providers":[{"type":"space","advances":{" ":4}}]}"#,
            ),
        ]);
        let providers = flatten_font_providers_with_options(
            "minecraft:default",
            FontOptions {
                uniform: true,
                japanese_variants: true,
            },
            &mut load,
        )
        .unwrap();
        assert_eq!(providers.len(), 2);
        let FontProviderDefinition::Space(space) = &providers[0] else {
            panic!("uniform reference should pass");
        };
        assert_eq!(space.advances, vec![(u32::from(' '), 99.0)]);
        let FontProviderDefinition::Unihex(unihex) = &providers[1] else {
            panic!("jp unihex should pass");
        };
        assert_eq!(unihex.hex_file, "minecraft:font/jp.zip");
    }

    #[test]
    fn unihex_provider_parses_ordered_overrides_and_validates_ranges() {
        let mut load = test_loader(&[(
            "minecraft:default",
            r#"{"providers":[{"type":"unihex","hex_file":"minecraft:font/u.zip","size_overrides":[
                {"from":"一","to":"龯","left":0,"right":15},
                {"from":"가","to":"힣","left":1,"right":15}
            ]}]}"#,
        )]);
        let providers = flatten_font_providers("minecraft:default", &mut load).unwrap();
        let FontProviderDefinition::Unihex(unihex) = &providers[0] else {
            panic!("expected unihex provider");
        };
        assert_eq!(unihex.size_overrides.len(), 2);
        assert_eq!(unihex.size_overrides[0].from, u32::from('一'));
        assert_eq!(unihex.size_overrides[1].left, 1);

        let mut invalid = test_loader(&[(
            "minecraft:default",
            r#"{"providers":[{"type":"unihex","hex_file":"minecraft:font/u.zip","size_overrides":[
                {"from":"龯","to":"一","left":0,"right":15}
            ]}]}"#,
        )]);
        let error = flatten_font_providers("minecraft:default", &mut invalid)
            .unwrap_err()
            .to_string();
        assert!(error.contains("invalid override range"), "{error}");
    }

    #[test]
    fn repeated_references_reuse_decoded_cache_and_retain_filter_context() {
        let mut loads = HashMap::<String, usize>::new();
        let mut load = |id: &str| -> Result<String> {
            *loads.entry(id.to_string()).or_default() += 1;
            Ok(if id == "minecraft:default" {
                r#"{"providers":[{"type":"reference","id":"minecraft:shared","filter":{"uniform":true}},{"type":"reference","id":"minecraft:shared","filter":{"uniform":false}}]}"#.to_string()
            } else {
                r#"{"providers":[{"type":"space","advances":{"A":0.5},"filter":{"uniform":true}}]}"#
                    .to_string()
            })
        };
        let providers = flatten_font_providers("minecraft:default", &mut load).unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(loads["minecraft:shared"], 1);
    }

    #[test]
    fn cyclic_references_invalidate_the_whole_font() {
        let mut load = test_loader(&[
            (
                "minecraft:default",
                r#"{"providers":[
                    {"type":"reference","id":"minecraft:a"},
                    {"type":"reference","id":"minecraft:a"}
                ]}"#,
            ),
            (
                "minecraft:a",
                r#"{"providers":[
                    {"type":"reference","id":"minecraft:default"},
                    {"type":"bitmap","file":"minecraft:font/a.png","ascent":7,"chars":["A"]}
                ]}"#,
            ),
        ]);

        let error = flatten_bitmaps("minecraft:default", &mut load).unwrap_err();
        assert!(error.to_string().contains("reference cycle"));
    }

    #[test]
    fn bitmap_provider_codec_defaults_and_validation_follow_vanilla() {
        // Missing height defaults to 8 (BitmapProvider.Definition.CODEC).
        let mut load = test_loader(&[(
            "minecraft:default",
            r#"{"providers":[{"type":"bitmap","file":"minecraft:font/x.png","ascent":7,"chars":["ab","cd"]}]}"#,
        )]);
        let providers = flatten_bitmaps("minecraft:default", &mut load).unwrap();
        assert_eq!(providers[0].height, 8);
        assert_eq!(providers[0].chars.len(), 2);

        // ascent > height is rejected (BitmapProvider.Definition.validate).
        let mut load = test_loader(&[(
            "minecraft:default",
            r#"{"providers":[{"type":"bitmap","file":"minecraft:font/x.png","height":8,"ascent":9,"chars":["a"]}]}"#,
        )]);
        let err = flatten_font_providers("minecraft:default", &mut load).unwrap_err();
        assert!(err.to_string().contains("higher than height"));

        // Ragged rows are rejected (validateDimensions).
        let mut load = test_loader(&[(
            "minecraft:default",
            r#"{"providers":[{"type":"bitmap","file":"minecraft:font/x.png","ascent":7,"chars":["ab","c"]}]}"#,
        )]);
        let err = flatten_font_providers("minecraft:default", &mut load).unwrap_err();
        assert!(err.to_string().contains("different lengths"));
    }

    #[test]
    fn chars_rows_count_codepoints_not_utf16_units() {
        // Vanilla decodes rows via String.codePoints(); surrogate pairs in the
        // JSON (e.g. Gothic letters in nonlatin_european) are single slots.
        let mut load = test_loader(&[(
            "minecraft:default",
            r#"{"providers":[{"type":"bitmap","file":"minecraft:font/x.png","ascent":7,"chars":["𐌰A"]}]}"#,
        )]);

        let providers = flatten_bitmaps("minecraft:default", &mut load).unwrap();

        assert_eq!(
            providers[0].chars,
            vec![vec![u32::from('\u{10330}'), u32::from('A')]]
        );
    }

    #[test]
    fn space_provider_parses_advances_map_including_zwnj() {
        // font/include/space.json: `" ": 4` (ascii space) and `"‌": 0`
        // (zero-width non-joiner) (SpaceProvider.Definition.CODEC).
        let mut load = test_loader(&[(
            "minecraft:default",
            r#"{"providers":[{"type":"space","advances":{" ":4,"‌":0}}]}"#,
        )]);

        let providers = flatten_font_providers("minecraft:default", &mut load).unwrap();

        assert_eq!(providers.len(), 1);
        let FontProviderDefinition::Space(space) = &providers[0] else {
            panic!("expected a space provider");
        };
        assert_eq!(
            space.advances,
            vec![(u32::from(' '), 4.0), (u32::from('\u{200c}'), 0.0)]
        );
    }

    #[test]
    fn space_provider_rejects_non_codepoint_keys_and_overflowing_advances() {
        let mut load = test_loader(&[(
            "minecraft:default",
            r#"{"providers":[{"type":"space","advances":{"ab":4}}]}"#,
        )]);
        let err = flatten_font_providers("minecraft:default", &mut load).unwrap_err();
        assert!(err.to_string().contains("not a single codepoint"));

        for advance in ["1e39", "-1e39"] {
            let definition =
                format!(r#"{{"providers":[{{"type":"space","advances":{{" ":{advance}}}}}]}}"#);
            let mut load = test_loader(&[("minecraft:default", &definition)]);
            let err = flatten_font_providers("minecraft:default", &mut load).unwrap_err();
            assert!(err.to_string().contains("is not a finite f32"));
        }
    }
}
