use std::borrow::Cow;

use bbb_render_types::BannerPatternKind;

use crate::{BannerPatternElement, BannerPatternHolderState, WorldStore};

use super::loader_order::loader_ordered_resource_id;
use super::normalize_codec_identifier;

pub const BANNER_PATTERN_REGISTRY: &str = "minecraft:banner_pattern";

impl WorldStore {
    /// Projects one numeric holder through the current synchronized registry order.
    ///
    /// A synchronized registry is authoritative. Only unresolved data-less stubs may use a
    /// resource-identity fallback; malformed, unknown, or missing entries never fall through to a
    /// bootstrap ordinal. The vanilla fallback is used only when no synchronized registry exists,
    /// and it counts holder ids in loader order: `minecraft:banner_pattern` is loaded from data
    /// files (RegistryDataLoader.java:117) and registered in sorted `Identifier` order
    /// (ResourceManagerRegistryLoadTask.java:62), not in `BannerPatterns` declaration order.
    pub fn synchronized_banner_pattern_presentation(
        &self,
        holder_id: i32,
    ) -> Option<Cow<'_, BannerPatternElement>> {
        if let Some(entry) = self.registry_entry_by_holder_id(BANNER_PATTERN_REGISTRY, holder_id) {
            if let Some(presentation) = self.registries.typed.banner_patterns.get(&entry.id) {
                return Some(Cow::Borrowed(presentation));
            }
            if entry.has_data {
                return None;
            }
            return BannerPatternKind::for_resource_id(&entry.id)
                .map(presentation_from_vanilla)
                .map(Cow::Owned);
        }
        if self.registry_content(BANNER_PATTERN_REGISTRY).is_some() {
            return None;
        }
        loader_ordered_resource_id(BANNER_PATTERN_REGISTRY, holder_id)
            .and_then(BannerPatternKind::for_resource_id)
            .map(presentation_from_vanilla)
            .map(Cow::Owned)
    }

    /// Projects a persistent named or direct holder stored in banner block-entity NBT.
    pub fn banner_pattern_presentation(
        &self,
        holder: &BannerPatternHolderState,
    ) -> Option<Cow<'_, BannerPatternElement>> {
        match holder {
            BannerPatternHolderState::Named(resource_id) => {
                if self.registry_content(BANNER_PATTERN_REGISTRY).is_some() {
                    let holder_id = self
                        .registry_holder_id_by_resource_id(BANNER_PATTERN_REGISTRY, resource_id)?;
                    self.synchronized_banner_pattern_presentation(holder_id)
                } else {
                    BannerPatternKind::for_resource_id(resource_id)
                        .map(presentation_from_vanilla)
                        .map(Cow::Owned)
                }
            }
            BannerPatternHolderState::Direct {
                asset_id,
                translation_key,
            } => direct_banner_pattern_presentation(asset_id, translation_key).map(Cow::Owned),
        }
    }
}

/// Validates and normalizes one direct banner-pattern value decoded from an item component or NBT.
pub fn direct_banner_pattern_presentation(
    asset_id: &str,
    translation_key: &bbb_protocol::JavaString,
) -> Option<BannerPatternElement> {
    Some(BannerPatternElement {
        asset_id: normalize_codec_identifier(asset_id)?,
        translation_key: translation_key.clone(),
    })
}

fn presentation_from_vanilla(pattern: BannerPatternKind) -> BannerPatternElement {
    BannerPatternElement {
        asset_id: pattern.resource_id().to_string(),
        translation_key: pattern.translation_key().into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RegistryElementData, RegistryPacketEntry};
    use bbb_registry_codec::registry_codec::{
        direct_leaf::{
            decode_direct_leaf_registry_element, BannerPatternElement, DirectLeafRegistryElement,
        },
        RegistryCodecError,
    };

    fn decode_banner_pattern_data(
        data: &RegistryElementData,
    ) -> Result<BannerPatternElement, RegistryCodecError> {
        let DirectLeafRegistryElement::BannerPattern(element) =
            decode_direct_leaf_registry_element(BANNER_PATTERN_REGISTRY, data)?
        else {
            unreachable!("banner-pattern registry route returned another leaf element")
        };
        Ok(element)
    }

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

        world.begin_registry_configuration();
        world.finish_registry_configuration().unwrap();

        assert_eq!(
            world
                .synchronized_banner_pattern_presentation(0)
                .map(Cow::into_owned),
            Some(BannerPatternElement {
                asset_id: "example:patterns/custom".to_string(),
                translation_key: "block.example.banner.custom".into(),
            })
        );
        assert_eq!(
            world
                .synchronized_banner_pattern_presentation(1)
                .map(Cow::into_owned),
            Some(BannerPatternElement {
                asset_id: "example:replacement".to_string(),
                translation_key: "block.example.banner.replacement".into(),
            })
        );
    }

    #[test]
    fn decodes_known_pack_json_through_the_same_banner_projection() {
        let data = super::super::element_value::decode_known_pack_element(
            br#"{
                "asset_id": "example:patterns/known",
                "translation_key": "block.example.banner.known",
                "unknown":{"nested":null}
            }"#,
        )
        .unwrap();

        assert_eq!(
            decode_banner_pattern_data(&data).unwrap(),
            BannerPatternElement {
                asset_id: "example:patterns/known".to_string(),
                translation_key: "block.example.banner.known".into(),
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
                .translation_key,
            "block.minecraft.banner.guster"
        );
        assert_eq!(
            world
                .synchronized_banner_pattern_presentation(1)
                .map(Cow::into_owned),
            None
        );
        assert_eq!(
            world
                .synchronized_banner_pattern_presentation(2)
                .unwrap()
                .translation_key,
            "block.minecraft.banner.base"
        );
        assert_eq!(world.synchronized_banner_pattern_presentation(43), None);
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

        world.begin_registry_configuration();
        world.finish_registry_configuration().unwrap();

        assert_eq!(
            world
                .banner_pattern_presentation(&BannerPatternHolderState::Named(
                    "example:named".to_string()
                ))
                .unwrap()
                .asset_id,
            "example:named_asset"
        );
        assert_eq!(
            world
                .banner_pattern_presentation(&BannerPatternHolderState::Direct {
                    asset_id: "example:direct_asset".to_string(),
                    translation_key: "block.example.banner.direct".into(),
                })
                .map(Cow::into_owned),
            Some(BannerPatternElement {
                asset_id: "example:direct_asset".to_string(),
                translation_key: "block.example.banner.direct".into(),
            })
        );
        assert_eq!(
            world.banner_pattern_presentation(&BannerPatternHolderState::Named(
                "minecraft:base".to_string()
            )),
            None,
            "a synchronized registry must not fall through to a static named pattern"
        );
    }

    #[test]
    fn malformed_synchronized_entries_are_rejected_during_configuration() {
        let mut world = WorldStore::new();
        world.record_registry_entries(
            BANNER_PATTERN_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "example:bad",
                banner_pattern_nbt("Bad Identifier", "block.example.banner.bad"),
            )],
        );
        world.begin_registry_configuration();
        assert!(world.finish_registry_configuration().is_err());
    }

    /// `minecraft:banner_pattern` is loaded from `data/minecraft/banner_pattern`
    /// (RegistryDataLoader.java:117) and registered in sorted `Identifier` order
    /// (ResourceManagerRegistryLoadTask.java:62), so the fallback holder id is
    /// the sorted index: `minecraft:guster` answers to 16, and 42 -- its
    /// `BannerPatterns` declaration index -- is `minecraft:triangles_top`.
    #[test]
    fn no_synchronized_registry_uses_vanilla_loader_order() {
        let world = WorldStore::new();
        assert_eq!(
            world
                .synchronized_banner_pattern_presentation(16)
                .unwrap()
                .asset_id,
            "minecraft:guster"
        );
        assert_eq!(
            world
                .synchronized_banner_pattern_presentation(42)
                .unwrap()
                .asset_id,
            "minecraft:triangles_top"
        );
        assert_eq!(
            world
                .synchronized_banner_pattern_presentation(0)
                .unwrap()
                .asset_id,
            "minecraft:base",
            "base heads both the declaration order and the sorted order"
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
