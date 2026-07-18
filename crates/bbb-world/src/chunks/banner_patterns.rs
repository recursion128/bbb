use crate::Result;

use super::{
    nbt::{decode_nbt_root, find_entry, NbtValue},
    state::{BannerPatternHolderState, BannerPatternLayerState, BannerPatternsState},
};

/// Decodes the banner's `patterns` list from a block-entity NBT payload
/// (`BannerBlockEntity.loadAdditional` reading `BannerPatternLayers.CODEC` —
/// a list of `{pattern: <BannerPattern.CODEC registry id>, color:
/// <DyeColor.CODEC name>}` compounds, `BannerPatternLayers.Layer.CODEC`).
/// `Ok(None)` when the payload has no `patterns` list — `saveAdditional`
/// skips the field entirely for a plain banner — and when any entry is not a
/// `{pattern: named-or-direct holder, color: string}` compound:
/// `BannerBlockEntity` reads the
/// whole list through `input.read("patterns", CODEC).orElse(EMPTY)`, so one
/// malformed layer folds the entire stack to `BannerPatternLayers.EMPTY`.
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
        let Some(pattern) = find_entry(fields, "pattern") else {
            return Ok(None);
        };
        let pattern = match pattern {
            NbtValue::String(resource_id) => {
                let Some(resource_id) = crate::registries::normalize_codec_identifier(resource_id)
                else {
                    return Ok(None);
                };
                BannerPatternHolderState::Named(resource_id)
            }
            NbtValue::Compound(pattern_fields) => {
                let Some(NbtValue::String(asset_id)) = find_entry(pattern_fields, "asset_id")
                else {
                    return Ok(None);
                };
                let Some(asset_id) = crate::registries::normalize_codec_identifier(asset_id) else {
                    return Ok(None);
                };
                let Some(NbtValue::String(translation_key)) =
                    find_entry(pattern_fields, "translation_key")
                else {
                    return Ok(None);
                };
                BannerPatternHolderState::Direct {
                    asset_id,
                    translation_key: translation_key.clone(),
                }
            }
            _ => return Ok(None),
        };
        let Some(NbtValue::String(color)) = find_entry(fields, "color") else {
            return Ok(None);
        };
        layers.push(BannerPatternLayerState {
            pattern,
            color: color.clone(),
        });
    }
    Ok(Some(BannerPatternsState { layers }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_named_and_direct_persistent_holder_forms() {
        let mut payload = vec![10, 9];
        write_string(&mut payload, "patterns");
        payload.push(10);
        payload.extend_from_slice(&2_i32.to_be_bytes());

        write_string_tag(&mut payload, "pattern", "minecraft:stripe_top");
        write_string_tag(&mut payload, "color", "red");
        payload.push(0);

        payload.push(10);
        write_string(&mut payload, "pattern");
        write_string_tag(&mut payload, "asset_id", "example:banner/custom");
        write_string_tag(
            &mut payload,
            "translation_key",
            "block.example.banner.custom",
        );
        payload.push(0);
        write_string_tag(&mut payload, "color", "blue");
        payload.push(0);
        payload.push(0);

        assert_eq!(
            decode_banner_patterns(&payload).unwrap(),
            Some(BannerPatternsState {
                layers: vec![
                    BannerPatternLayerState {
                        pattern: BannerPatternHolderState::Named(
                            "minecraft:stripe_top".to_string(),
                        ),
                        color: "red".to_string(),
                    },
                    BannerPatternLayerState {
                        pattern: BannerPatternHolderState::Direct {
                            asset_id: "example:banner/custom".to_string(),
                            translation_key: "block.example.banner.custom".to_string(),
                        },
                        color: "blue".to_string(),
                    },
                ],
            })
        );
    }

    #[test]
    fn malformed_direct_holder_folds_the_whole_layer_stack_to_empty() {
        let mut payload = vec![10, 9];
        write_string(&mut payload, "patterns");
        payload.push(10);
        payload.extend_from_slice(&1_i32.to_be_bytes());
        payload.push(10);
        write_string(&mut payload, "pattern");
        write_string_tag(&mut payload, "asset_id", "Invalid Asset");
        write_string_tag(
            &mut payload,
            "translation_key",
            "block.example.banner.invalid",
        );
        payload.push(0);
        write_string_tag(&mut payload, "color", "red");
        payload.push(0);
        payload.push(0);

        assert_eq!(decode_banner_patterns(&payload).unwrap(), None);
    }

    fn write_string_tag(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_string(out, name);
        write_string(out, value);
    }

    fn write_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
