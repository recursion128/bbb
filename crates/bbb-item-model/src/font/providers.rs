//! Parses the vanilla `font/default.json` provider chain
//! (`FontManager.FontDefinitionFile`) into a flattened, priority-ordered list
//! of bitmap provider definitions.
//!
//! Only `bitmap` and `reference` providers are consumed in this slice:
//! - `space` advances stay hardcoded at the atlas builder (follow-up slice),
//! - `unihex` (unifont zips) is deferred — the assets tree ships no unifont
//!   archive — so uncovered codepoints keep the `?` replacement fallback.

use std::collections::HashSet;

use anyhow::{bail, Context, Result};
use bbb_pack::{PackRoots, ResourceLocation};
use serde_json::Value;

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

/// Loads and flattens the bitmap providers of `font_id` (e.g.
/// `minecraft:default`) through the resource stack, recursing `reference`
/// providers depth-first in place like vanilla `FontManager`'s builder stack,
/// which preserves the referencing file's provider order.
pub fn load_font_bitmap_providers(
    roots: &PackRoots,
    font_id: &str,
) -> Result<Vec<FontBitmapProviderDefinition>> {
    let stack = roots.resource_stack();
    let mut load_definition = |id: &str| -> Result<String> {
        let location = font_definition_location(id)?;
        let resource = stack
            .get_resource(&location)
            .with_context(|| format!("missing font definition {}", location.id()))?;
        std::fs::read_to_string(&resource.path)
            .with_context(|| format!("read font definition {}", resource.path.display()))
    };
    flatten_font_bitmap_providers(font_id, &mut load_definition)
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

pub(crate) fn flatten_font_bitmap_providers(
    font_id: &str,
    load_definition: &mut dyn FnMut(&str) -> Result<String>,
) -> Result<Vec<FontBitmapProviderDefinition>> {
    let mut visited = HashSet::new();
    let mut providers = Vec::new();
    flatten_into(font_id, load_definition, &mut visited, &mut providers)?;
    Ok(providers)
}

fn flatten_into(
    font_id: &str,
    load_definition: &mut dyn FnMut(&str) -> Result<String>,
    visited: &mut HashSet<String>,
    out: &mut Vec<FontBitmapProviderDefinition>,
) -> Result<()> {
    // Re-visiting a definition is a no-op: vanilla resolves glyphs first
    // provider wins (`FontSet.computeGlyphInfo`), so a second inclusion of the
    // same providers can never change lookup results; skipping also guards
    // against reference cycles.
    if !visited.insert(font_id.to_string()) {
        return Ok(());
    }
    let json = load_definition(font_id)?;
    let root: Value =
        serde_json::from_str(&json).with_context(|| format!("parse font definition {font_id}"))?;
    let providers = root
        .get("providers")
        .and_then(Value::as_array)
        .with_context(|| format!("font definition {font_id} has no \"providers\" array"))?;
    for provider in providers {
        let provider_type = provider
            .get("type")
            .and_then(Value::as_str)
            .with_context(|| format!("font provider in {font_id} has no \"type\""))?;
        match provider_type {
            "bitmap" => out.push(parse_bitmap_provider(font_id, provider)?),
            "reference" => {
                let id = provider
                    .get("id")
                    .and_then(Value::as_str)
                    .with_context(|| {
                        format!("reference font provider in {font_id} has no \"id\"")
                    })?;
                // The optional `filter` selects on FontOptions (`uniform`,
                // `jp`). We run the fixed vanilla defaults (both off), under
                // which every vanilla reference passes (`include/default`
                // filters `uniform: false`), so it is intentionally ignored.
                flatten_into(id, load_definition, visited, out)?;
            }
            // `space` and `unihex` (and any other provider type) are outside
            // this slice; skipping keeps first-provider-wins semantics intact
            // for the codepoints the bitmap pages do cover.
            _ => {}
        }
    }
    Ok(())
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

fn parse_provider_int(value: &Value) -> Result<i32> {
    value
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())
        .context("expected an integer")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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

        let providers = flatten_font_bitmap_providers("minecraft:default", &mut load).unwrap();

        assert_eq!(
            providers
                .iter()
                .map(|provider| (provider.file.as_str(), provider.height, provider.ascent))
                .collect::<Vec<_>>(),
            vec![
                ("minecraft:font/nonlatin_european.png", 8, 7),
                ("minecraft:font/accented.png", 12, 10),
                ("minecraft:font/ascii.png", 8, 7),
            ]
        );
        assert_eq!(providers[0].chars, vec![vec!['λ', 'ж']]);
        assert_eq!(providers[1].chars, vec![vec!['é', 'ü']]);
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

        let providers = flatten_font_bitmap_providers("minecraft:default", &mut load).unwrap();

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
        let providers = flatten_font_bitmap_providers("minecraft:default", &mut load).unwrap();
        assert_eq!(providers[0].height, 8);
        assert_eq!(providers[0].chars.len(), 2);

        // ascent > height is rejected (BitmapProvider.Definition.validate).
        let mut load = test_loader(&[(
            "minecraft:default",
            r#"{"providers":[{"type":"bitmap","file":"minecraft:font/x.png","height":8,"ascent":9,"chars":["a"]}]}"#,
        )]);
        let err = flatten_font_bitmap_providers("minecraft:default", &mut load).unwrap_err();
        assert!(err.to_string().contains("higher than height"));

        // Ragged rows are rejected (validateDimensions).
        let mut load = test_loader(&[(
            "minecraft:default",
            r#"{"providers":[{"type":"bitmap","file":"minecraft:font/x.png","ascent":7,"chars":["ab","c"]}]}"#,
        )]);
        let err = flatten_font_bitmap_providers("minecraft:default", &mut load).unwrap_err();
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

        let providers = flatten_font_bitmap_providers("minecraft:default", &mut load).unwrap();

        assert_eq!(providers[0].chars, vec![vec!['\u{10330}', 'A']]);
    }
}
