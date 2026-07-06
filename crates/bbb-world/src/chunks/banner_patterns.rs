use crate::Result;

use super::{
    nbt::{decode_nbt_root, find_entry, NbtValue},
    state::{BannerPatternLayerState, BannerPatternsState},
};

/// Decodes the banner's `patterns` list from a block-entity NBT payload
/// (`BannerBlockEntity.loadAdditional` reading `BannerPatternLayers.CODEC` —
/// a list of `{pattern: <BannerPattern.CODEC registry id>, color:
/// <DyeColor.CODEC name>}` compounds, `BannerPatternLayers.Layer.CODEC`).
/// `Ok(None)` when the payload has no `patterns` list — `saveAdditional`
/// skips the field entirely for a plain banner — and when any entry is not a
/// `{pattern: string, color: string}` compound: `BannerBlockEntity` reads the
/// whole list through `input.read("patterns", CODEC).orElse(EMPTY)`, so one
/// malformed layer folds the entire stack to `BannerPatternLayers.EMPTY`.
/// (A data-driven pattern serialized as an inline compound instead of a
/// registry id lands in that fold too — bbb has no texture for it either.)
pub(crate) fn decode_banner_patterns(raw_nbt: &[u8]) -> Result<Option<BannerPatternsState>> {
    let Some(root) = decode_nbt_root(raw_nbt)? else {
        return Ok(None);
    };
    let NbtValue::Compound(entries) = root else {
        return Ok(None);
    };
    let Some(NbtValue::List(patterns)) = find_entry(&entries, "patterns") else {
        return Ok(None);
    };
    let mut layers = Vec::with_capacity(patterns.len());
    for entry in patterns {
        let NbtValue::Compound(fields) = entry else {
            return Ok(None);
        };
        let Some(NbtValue::String(pattern)) = find_entry(fields, "pattern") else {
            return Ok(None);
        };
        let Some(NbtValue::String(color)) = find_entry(fields, "color") else {
            return Ok(None);
        };
        layers.push(BannerPatternLayerState {
            pattern: pattern.clone(),
            color: color.clone(),
        });
    }
    Ok(Some(BannerPatternsState { layers }))
}
