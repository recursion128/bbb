//! Parses the vanilla `font/default.json` provider chain
//! (`FontManager.FontDefinitionFile`) into a flattened, priority-ordered list
//! of provider definitions.
//!
//! `bitmap`, `space`, `unihex`, and `reference` providers are consumed with
//! Vanilla's fixed default font options (`uniform=false`, `jp=false`).

use std::collections::HashSet;

use anyhow::{bail, Context, Result};
use bbb_pack::{PackRoots, ResourceLocation};
use serde_json::Value;

const MAX_FONT_DEFINITIONS: u64 = 256;
const MAX_FONT_PROVIDERS: u64 = 4_096;
const MAX_FONT_BITMAP_GLYPH_SLOTS: u64 = 1_048_576;
const MAX_FONT_SPACE_ADVANCES: u64 = 65_536;
const MAX_FONT_UNIHEX_OVERRIDES: u64 = 4_096;
const FONT_DEFINITION_LOGICAL_BYTES: u64 = 512;
const FONT_PROVIDER_LOGICAL_BYTES: u64 = 256;
const FONT_GLYPH_SLOT_LOGICAL_BYTES: u64 = 64;

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
    /// Codepoint grid rows; `'\0'` marks an empty slot (vanilla skips them).
    pub chars: Vec<Vec<char>>,
}

/// One flattened `"type": "space"` provider, mirroring vanilla
/// `SpaceProvider.Definition` (codec field `advances`: a map of single-codepoint
/// keys to a float advance). Every vanilla `font/include/space.json` entry
/// (`" ": 4`, `"‌": 0`) is a whole number, so the advance stays `u32`
/// like the rest of this crate's advance pipeline instead of widening to
/// float; a fractional or negative advance is rejected at parse time.
#[derive(Debug, Clone, PartialEq)]
pub struct FontSpaceProviderDefinition {
    pub advances: Vec<(char, u32)>,
}

/// One inclusive codepoint range whose retained bitmap columns are forced by
/// `UnihexProvider.Definition.sizeOverrides`. Vanilla removes each matching
/// glyph from the automatic-width map in list order, so overlapping ranges are
/// deliberately retained in their source order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontUnihexSizeOverride {
    pub from: char,
    pub to: char,
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

/// One flattened provider in `providers` order, after `reference` expansion.
/// Vanilla `FontSet.computeGlyphInfo` walks providers of every type
/// (`bitmap`, `space`, `unihex`, ...) in this same flattened order and takes
/// the first one that supplies a codepoint, so callers must preserve
/// occurrence order across variants when merging glyphs.
#[derive(Debug, Clone, PartialEq)]
pub enum FontProviderDefinition {
    Bitmap(FontBitmapProviderDefinition),
    Space(FontSpaceProviderDefinition),
    Unihex(FontUnihexProviderDefinition),
}

/// Loads and flattens the providers of `font_id` (e.g. `minecraft:default`)
/// through the resource stack, recursing `reference` providers depth-first in
/// place like vanilla `FontManager`'s builder stack, which preserves the
/// referencing file's provider order.
pub fn load_font_providers(
    roots: &PackRoots,
    font_id: &str,
) -> Result<Vec<FontProviderDefinition>> {
    let stack = roots.resource_stack();
    let mut definition_count = 0_u64;
    let mut load_definition = |id: &str| -> Result<Value> {
        let next_definition_count = definition_count
            .checked_add(1)
            .context("font definition count overflow")?;
        stack.admit_resource_collection_count(
            next_definition_count,
            MAX_FONT_DEFINITIONS,
            "font definitions",
        )?;
        let location = font_definition_location(id)?;
        let resource = roots
            .resource_with_indexed_vanilla_fallback(&location)?
            .with_context(|| format!("missing font definition {}", location.id()))?;
        let value: Value = stack
            .read_json_resource(&resource, "font definition")
            .with_context(|| format!("read font definition {}", resource.path.display()))?;
        definition_count = next_definition_count;
        Ok(value)
    };
    let providers = flatten_font_provider_values(font_id, &mut load_definition)?;
    admit_font_collections(&stack, definition_count, &providers)?;
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
    let mut load_value = |id: &str| -> Result<Value> {
        let json = load_definition(id)?;
        serde_json::from_str(&json).with_context(|| format!("parse font definition {id}"))
    };
    flatten_font_provider_values(font_id, &mut load_value)
}

fn flatten_font_provider_values(
    font_id: &str,
    load_definition: &mut dyn FnMut(&str) -> Result<Value>,
) -> Result<Vec<FontProviderDefinition>> {
    let mut visited = HashSet::new();
    let mut providers = Vec::new();
    flatten_into(font_id, load_definition, &mut visited, &mut providers)?;
    Ok(providers)
}

fn flatten_into(
    font_id: &str,
    load_definition: &mut dyn FnMut(&str) -> Result<Value>,
    visited: &mut HashSet<String>,
    out: &mut Vec<FontProviderDefinition>,
) -> Result<()> {
    // Re-visiting a definition is a no-op: vanilla resolves glyphs first
    // provider wins (`FontSet.computeGlyphInfo`), so a second inclusion of the
    // same providers can never change lookup results; skipping also guards
    // against reference cycles.
    if visited.contains(font_id) {
        return Ok(());
    }
    visited.insert(font_id.to_string());
    let root = load_definition(font_id)?;
    if visited.len() > usize::try_from(MAX_FONT_DEFINITIONS).expect("font limit fits usize") {
        bail!("font provider graph exceeds the {MAX_FONT_DEFINITIONS} definition limit");
    }
    let providers = root
        .get("providers")
        .and_then(Value::as_array)
        .with_context(|| format!("font definition {font_id} has no \"providers\" array"))?;
    for provider in providers {
        if !provider_enabled(font_id, provider)? {
            continue;
        }
        let provider_type = provider
            .get("type")
            .and_then(Value::as_str)
            .with_context(|| format!("font provider in {font_id} has no \"type\""))?;
        match provider_type {
            "bitmap" => out.push(FontProviderDefinition::Bitmap(parse_bitmap_provider(
                font_id, provider,
            )?)),
            "space" => out.push(FontProviderDefinition::Space(parse_space_provider(
                font_id, provider,
            )?)),
            "unihex" => out.push(FontProviderDefinition::Unihex(parse_unihex_provider(
                font_id, provider,
            )?)),
            "reference" => {
                let id = provider
                    .get("id")
                    .and_then(Value::as_str)
                    .with_context(|| {
                        format!("reference font provider in {font_id} has no \"id\"")
                    })?;
                flatten_into(id, load_definition, visited, out)?;
            }
            // Vanilla safely skips an individual provider whose type codec is
            // unavailable. Preserve that forward-compatible behavior for
            // provider types introduced after 26.1.
            _ => {}
        }
    }
    Ok(())
}

fn admit_font_collection_counts(
    stack: &bbb_pack::PackResourceStack,
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
    stack: &bbb_pack::PackResourceStack,
    definition_count: u64,
    providers: &[FontProviderDefinition],
) -> Result<()> {
    let provider_count = u64::try_from(providers.len()).context("font provider count overflow")?;
    let mut glyph_slots = 0_u64;
    let mut space_advances = 0_u64;
    let mut unihex_overrides = 0_u64;
    for provider in providers {
        match provider {
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

/// `FontOption.Filter.apply`: every declared option must match the enabled
/// option set. BBB's startup contract uses Vanilla's default options, where
/// neither `uniform` nor `jp` is enabled.
fn provider_enabled(font_id: &str, provider: &Value) -> Result<bool> {
    let Some(filter) = provider.get("filter") else {
        return Ok(true);
    };
    let filter = filter
        .as_object()
        .with_context(|| format!("font provider in {font_id} has a non-object filter"))?;
    for (option, expected) in filter {
        if !matches!(option.as_str(), "uniform" | "jp") {
            bail!("font provider in {font_id} has unknown font option {option:?}");
        }
        let expected = expected.as_bool().with_context(|| {
            format!("font provider in {font_id} has non-boolean font option {option:?}")
        })?;
        if expected {
            return Ok(false);
        }
    }
    Ok(true)
}

fn parse_bitmap_provider(font_id: &str, provider: &Value) -> Result<FontBitmapProviderDefinition> {
    let file = provider
        .get("file")
        .and_then(Value::as_str)
        .with_context(|| format!("bitmap font provider in {font_id} has no \"file\""))?
        .to_string();
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
            row.as_str()
                .map(|row| row.chars().collect::<Vec<char>>())
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

/// `SpaceProvider.Definition.CODEC`: `Codec.unboundedMap(ExtraCodecs.CODEPOINT,
/// Codec.FLOAT).fieldOf("advances")`. Each key is a single-codepoint string
/// (`ExtraCodecs.CODEPOINT`); the advance is a float that this crate narrows
/// to `u32`, rejecting fractional or negative values (none appear in vanilla
/// assets).
fn parse_space_provider(font_id: &str, provider: &Value) -> Result<FontSpaceProviderDefinition> {
    let advances = provider
        .get("advances")
        .and_then(Value::as_object)
        .with_context(|| format!("space font provider in {font_id} has no \"advances\" object"))?;
    let mut parsed = Vec::with_capacity(advances.len());
    for (key, value) in advances {
        let mut chars = key.chars();
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
        if advance < 0.0 || advance.fract() != 0.0 {
            bail!(
                "space font provider in {font_id} advance {advance} for {key:?} is not a \
                 non-negative integer (fractional/negative space advances are unsupported)"
            );
        }
        parsed.push((codepoint, advance as u32));
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
        // Vanilla's codec accepts arbitrary ints and unpacking treats columns
        // outside 0..31 as transparent. Keep those semantics while rejecting
        // a non-positive/overflowing retained width before atlas allocation.
        let width = right
            .checked_sub(left)
            .and_then(|width| width.checked_add(1))
            .context("unihex size override width overflow")?;
        if width <= 0 || width > 64 {
            bail!(
                "unihex font provider {hex_file} override [{left};{right}] has unsafe width {width}"
            );
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

fn parse_codepoint_field(value: &Value, field: &str, hex_file: &str) -> Result<char> {
    let text = value
        .get(field)
        .and_then(Value::as_str)
        .with_context(|| format!("unihex size override in {hex_file} has no {field:?}"))?;
    let mut chars = text.chars();
    let codepoint = chars
        .next()
        .with_context(|| format!("unihex size override {field} is empty"))?;
    if chars.next().is_some() {
        bail!("unihex size override {field} {text:?} is not one codepoint");
    }
    Ok(codepoint)
}

fn parse_provider_int(value: &Value) -> Result<i32> {
    value
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())
        .context("expected an integer")
}

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
            bbb_pack::SelectedClientPackStack::from_startup_dirs(&roots, [low, high]).unwrap();
        let roots = roots.with_client_resource_generation(41, selected).unwrap();

        let providers = load_font_providers(&roots, "example:default").unwrap();

        assert_eq!(roots.client_resource_generation().unwrap().generation(), 41);
        let FontProviderDefinition::Space(space) = &providers[0] else {
            panic!("expected selected space provider");
        };
        assert_eq!(space.advances, vec![(' ', 5)]);
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
        let exact = bbb_pack::PackResourceStack::default();
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
            let stack = bbb_pack::PackResourceStack::default();
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

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "bbb-item-model-font-{label}-{}-{nonce}",
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
        assert_eq!(space.advances, vec![(' ', 4), ('\u{200c}', 0)]);

        let bitmaps: Vec<&FontBitmapProviderDefinition> = providers[1..4]
            .iter()
            .map(|provider| match provider {
                FontProviderDefinition::Bitmap(bitmap) => bitmap,
                FontProviderDefinition::Space(_) => panic!("unexpected second space provider"),
                FontProviderDefinition::Unihex(_) => panic!("unexpected early unihex provider"),
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
        assert_eq!(bitmaps[0].chars, vec![vec!['λ', 'ж']]);
        assert_eq!(bitmaps[1].chars, vec![vec!['é', 'ü']]);
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
        assert_eq!(space.advances, vec![(' ', 4)]);
        let FontProviderDefinition::Unihex(unihex) = &providers[1] else {
            panic!("default unihex should pass");
        };
        assert_eq!(unihex.hex_file, "minecraft:font/default.zip");
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
        assert_eq!(unihex.size_overrides[0].from, '一');
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
    fn repeated_and_cyclic_references_are_visited_once() {
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

        let providers = flatten_bitmaps("minecraft:default", &mut load).unwrap();

        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].file, "minecraft:font/a.png");
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

        assert_eq!(providers[0].chars, vec![vec!['\u{10330}', 'A']]);
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
        assert_eq!(space.advances, vec![(' ', 4), ('\u{200c}', 0)]);
    }

    #[test]
    fn space_provider_rejects_non_codepoint_keys_and_fractional_advances() {
        let mut load = test_loader(&[(
            "minecraft:default",
            r#"{"providers":[{"type":"space","advances":{"ab":4}}]}"#,
        )]);
        let err = flatten_font_providers("minecraft:default", &mut load).unwrap_err();
        assert!(err.to_string().contains("not a single codepoint"));

        let mut load = test_loader(&[(
            "minecraft:default",
            r#"{"providers":[{"type":"space","advances":{" ":4.5}}]}"#,
        )]);
        let err = flatten_font_providers("minecraft:default", &mut load).unwrap_err();
        assert!(err.to_string().contains("is not a non-negative integer"));
    }
}
