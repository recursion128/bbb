use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_render_types::BannerPatternKind;

use crate::{BannerPatternHolderState, WorldStore};

use super::normalize_codec_identifier;

pub const BANNER_PATTERN_REGISTRY: &str = "minecraft:banner_pattern";

/// Client-consumed fields from vanilla 26.1 `BannerPattern.DIRECT_CODEC`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BannerPatternPresentation {
    pub asset_id: String,
    pub translation_key: String,
}

/// Exact synchronized banner-pattern field that failed its direct codec contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BannerPatternProjectionError {
    pub resource_id: String,
    pub field: String,
    pub reason: &'static str,
}

impl std::fmt::Display for BannerPatternProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} {}: {}",
            self.resource_id, self.field, self.reason
        )
    }
}

impl std::error::Error for BannerPatternProjectionError {}

impl WorldStore {
    /// Projects one numeric holder through the current synchronized registry order.
    ///
    /// A synchronized registry is authoritative. Only unresolved data-less stubs may use a
    /// resource-identity fallback; malformed, unknown, or missing entries never fall through to a
    /// bootstrap ordinal. The fixed vanilla order is used only when no synchronized registry exists.
    pub fn synchronized_banner_pattern_presentation(
        &self,
        holder_id: i32,
    ) -> Result<Option<BannerPatternPresentation>, BannerPatternProjectionError> {
        if let Some(entry) = self.registry_entry_by_holder_id(BANNER_PATTERN_REGISTRY, holder_id) {
            if let Some(presentation) = self.registries.typed.banner_patterns.get(&entry.id) {
                return Ok(Some(presentation.clone()));
            }
            return presentation_from_registry_entry(entry);
        }
        if self.registry_content(BANNER_PATTERN_REGISTRY).is_some() {
            return Ok(None);
        }
        Ok(BannerPatternKind::for_vanilla_registry_id(holder_id).map(presentation_from_vanilla))
    }

    /// Projects a persistent named or direct holder stored in banner block-entity NBT.
    pub fn banner_pattern_presentation(
        &self,
        holder: &BannerPatternHolderState,
    ) -> Result<Option<BannerPatternPresentation>, BannerPatternProjectionError> {
        match holder {
            BannerPatternHolderState::Named(resource_id) => {
                if self.registry_content(BANNER_PATTERN_REGISTRY).is_some() {
                    let Some(holder_id) = self
                        .registry_holder_id_by_resource_id(BANNER_PATTERN_REGISTRY, resource_id)
                    else {
                        return Ok(None);
                    };
                    self.synchronized_banner_pattern_presentation(holder_id)
                } else {
                    Ok(BannerPatternKind::for_resource_id(resource_id)
                        .map(presentation_from_vanilla))
                }
            }
            BannerPatternHolderState::Direct {
                asset_id,
                translation_key,
            } => Ok(direct_banner_pattern_presentation(
                asset_id,
                translation_key,
            )),
        }
    }
}

/// Validates and normalizes one direct banner-pattern value decoded from an item component or NBT.
pub fn direct_banner_pattern_presentation(
    asset_id: &str,
    translation_key: &str,
) -> Option<BannerPatternPresentation> {
    Some(BannerPatternPresentation {
        asset_id: normalize_codec_identifier(asset_id)?,
        translation_key: translation_key.to_string(),
    })
}

fn presentation_from_registry_entry(
    entry: &crate::RegistryPacketEntry,
) -> Result<Option<BannerPatternPresentation>, BannerPatternProjectionError> {
    if let Some(data) = entry.element_data() {
        return decode_banner_pattern_value(&data.value)
            .map(Some)
            .map_err(|failure| BannerPatternProjectionError {
                resource_id: entry.id.clone(),
                field: failure.field.to_string(),
                reason: failure.reason,
            });
    }
    if entry.has_data {
        return Err(BannerPatternProjectionError {
            resource_id: entry.id.clone(),
            field: "<root>".to_string(),
            reason: "typed payload unavailable",
        });
    }
    Ok(BannerPatternKind::for_resource_id(&entry.id).map(presentation_from_vanilla))
}

fn presentation_from_vanilla(pattern: BannerPatternKind) -> BannerPatternPresentation {
    BannerPatternPresentation {
        asset_id: pattern.resource_id().to_string(),
        translation_key: pattern.translation_key().to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DecodeFailure {
    pub field: &'static str,
    pub reason: &'static str,
}

pub(super) fn decode_banner_pattern_value(
    value: &NbtSummaryValue,
) -> Result<BannerPatternPresentation, DecodeFailure> {
    let NbtSummaryValue::Compound(fields) = value else {
        return Err(failure("<root>", "expected compound"));
    };
    let asset_id = required_identifier(fields, "asset_id")?;
    let translation_key = required_string(fields, "translation_key")?;
    Ok(BannerPatternPresentation {
        asset_id,
        translation_key,
    })
}

fn required_identifier(
    fields: &[NbtSummaryEntry],
    field: &'static str,
) -> Result<String, DecodeFailure> {
    let value = required_string(fields, field)?;
    normalize_codec_identifier(&value).ok_or_else(|| failure(field, "invalid identifier"))
}

fn required_string(
    fields: &[NbtSummaryEntry],
    field: &'static str,
) -> Result<String, DecodeFailure> {
    match field_value(fields, field) {
        Some(NbtSummaryValue::String(value)) => Ok(value.clone()),
        Some(_) => Err(failure(field, "expected string")),
        None => Err(failure(field, "missing required field")),
    }
}

fn field_value<'a>(fields: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    fields
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

const fn failure(field: &'static str, reason: &'static str) -> DecodeFailure {
    DecodeFailure { field, reason }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPacketEntry;

    #[test]
    fn synchronized_presentations_follow_server_order_and_direct_codec_fields() {
        let mut world = WorldStore::new();
        world.record_registry_entries(
            BANNER_PATTERN_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:custom",
                    banner_pattern_nbt("example:patterns/custom", "block.example.banner.custom"),
                ),
                RegistryPacketEntry::with_raw_data(
                    "minecraft:base",
                    banner_pattern_nbt("example:replacement", "block.example.banner.replacement"),
                ),
            ],
        );

        assert_eq!(
            world.synchronized_banner_pattern_presentation(0).unwrap(),
            Some(BannerPatternPresentation {
                asset_id: "example:patterns/custom".to_string(),
                translation_key: "block.example.banner.custom".to_string(),
            })
        );
        assert_eq!(
            world.synchronized_banner_pattern_presentation(1).unwrap(),
            Some(BannerPatternPresentation {
                asset_id: "example:replacement".to_string(),
                translation_key: "block.example.banner.replacement".to_string(),
            })
        );
    }

    #[test]
    fn decodes_known_pack_json_through_the_same_banner_projection() {
        let data = super::super::element_value::decode_known_pack_element(
            br#"{
                "asset_id": "example:patterns/known",
                "translation_key": "block.example.banner.known"
            }"#,
        )
        .unwrap();

        assert_eq!(
            decode_banner_pattern_value(&data.value).unwrap(),
            BannerPatternPresentation {
                asset_id: "example:patterns/known".to_string(),
                translation_key: "block.example.banner.known".to_string(),
            }
        );
    }

    #[test]
    fn stubs_use_resource_identity_without_bootstrap_ordinal_substitution() {
        let mut world = WorldStore::new();
        world.record_registry_entries(
            BANNER_PATTERN_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::stub("minecraft:guster"),
                RegistryPacketEntry::stub("example:unknown"),
                RegistryPacketEntry::stub("minecraft:base"),
            ],
        );

        assert_eq!(
            world
                .synchronized_banner_pattern_presentation(0)
                .unwrap()
                .unwrap()
                .translation_key,
            "block.minecraft.banner.guster"
        );
        assert_eq!(
            world.synchronized_banner_pattern_presentation(1).unwrap(),
            None
        );
        assert_eq!(
            world
                .synchronized_banner_pattern_presentation(2)
                .unwrap()
                .unwrap()
                .translation_key,
            "block.minecraft.banner.base"
        );
        assert_eq!(
            world.synchronized_banner_pattern_presentation(43).unwrap(),
            None
        );
    }

    #[test]
    fn named_and_direct_block_entity_holders_share_typed_projection() {
        let mut world = WorldStore::new();
        world.record_registry_entries(
            BANNER_PATTERN_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "example:named",
                banner_pattern_nbt("example:named_asset", "block.example.banner.named"),
            )],
        );

        assert_eq!(
            world
                .banner_pattern_presentation(&BannerPatternHolderState::Named(
                    "example:named".to_string()
                ))
                .unwrap()
                .unwrap()
                .asset_id,
            "example:named_asset"
        );
        assert_eq!(
            world
                .banner_pattern_presentation(&BannerPatternHolderState::Direct {
                    asset_id: "example:direct_asset".to_string(),
                    translation_key: "block.example.banner.direct".to_string(),
                })
                .unwrap(),
            Some(BannerPatternPresentation {
                asset_id: "example:direct_asset".to_string(),
                translation_key: "block.example.banner.direct".to_string(),
            })
        );
        assert_eq!(
            world
                .banner_pattern_presentation(&BannerPatternHolderState::Named(
                    "minecraft:base".to_string()
                ))
                .unwrap(),
            None,
            "a synchronized registry must not fall through to a static named pattern"
        );
    }

    #[test]
    fn malformed_synchronized_entries_report_the_exact_codec_field() {
        let mut world = WorldStore::new();
        world.record_registry_entries(
            BANNER_PATTERN_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "example:bad",
                banner_pattern_nbt("Bad Identifier", "block.example.banner.bad"),
            )],
        );

        assert_eq!(
            world
                .synchronized_banner_pattern_presentation(0)
                .unwrap_err(),
            BannerPatternProjectionError {
                resource_id: "example:bad".to_string(),
                field: "asset_id".to_string(),
                reason: "invalid identifier",
            }
        );
    }

    #[test]
    fn no_synchronized_registry_uses_vanilla_fallback_order() {
        let world = WorldStore::new();
        assert_eq!(
            world
                .synchronized_banner_pattern_presentation(42)
                .unwrap()
                .unwrap()
                .asset_id,
            "minecraft:guster"
        );
    }

    fn banner_pattern_nbt(asset_id: &str, translation_key: &str) -> Vec<u8> {
        let mut out = vec![10];
        write_string(&mut out, "asset_id", asset_id);
        write_string(&mut out, "translation_key", translation_key);
        out.push(0);
        out
    }

    fn write_string(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_nbt_string(out, name);
        write_nbt_string(out, value);
    }

    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
