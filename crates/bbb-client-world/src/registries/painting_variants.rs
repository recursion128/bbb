use std::borrow::Cow;

use bbb_protocol::nbt_summary::{checked_int_value as numeric_i32, compound_field as field};
use bbb_protocol::{
    styled_component_runs_from_nbt_value, styled_runs_summary_text, NbtSummaryEntry,
    NbtSummaryValue, StyledTextRun,
};
use serde::{Deserialize, Serialize};

use crate::{
    painting_variants::{vanilla_painting_variant_for_resource_id, VanillaPaintingVariant},
    WorldStore,
};

pub const PAINTING_VARIANT_REGISTRY: &str = "minecraft:painting_variant";

/// The fields read by vanilla's painting bounds and renderer from one synchronized variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaintingVariantClientProjection {
    pub width: i32,
    pub height: i32,
    pub asset_id: String,
}

/// The fields consumed by painting item tooltips and entity rendering from one named holder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaintingVariantPresentation {
    pub width: i32,
    pub height: i32,
    pub asset_id: String,
    pub title: Option<String>,
    pub title_styled: Vec<StyledTextRun>,
    #[serde(default)]
    pub title_nbt: Option<NbtSummaryValue>,
    pub title_translation_key: Option<String>,
    pub author: Option<String>,
    pub author_styled: Vec<StyledTextRun>,
    #[serde(default)]
    pub author_nbt: Option<NbtSummaryValue>,
    pub author_translation_key: Option<String>,
}

impl WorldStore {
    /// Resolves a painting variant through the current server holder order. Materialized
    /// synchronized entries use their own codec fields, unresolved data-less stubs use resource
    /// identity, and only a session with no synchronized contents uses the fixed vanilla order.
    pub fn painting_variant_client_projection(
        &self,
        holder_id: i32,
    ) -> Option<PaintingVariantClientProjection> {
        self.painting_variant_presentation(holder_id)
            .map(client_projection_from_presentation)
    }

    /// Resolves the complete client-consumed painting presentation through the current server
    /// holder order. A synchronized registry is authoritative; only unresolved vanilla data-less
    /// stubs use their resource identity, and the fixed bootstrap order is used only when no
    /// synchronized painting registry exists.
    pub fn painting_variant_presentation(
        &self,
        holder_id: i32,
    ) -> Option<Cow<'_, PaintingVariantPresentation>> {
        if let Some(entry) = self.registry_entry_by_holder_id(PAINTING_VARIANT_REGISTRY, holder_id)
        {
            if let Some(presentation) = self.registries.typed.painting_variants.get(&entry.id) {
                return Some(Cow::Borrowed(presentation));
            }
            if entry.has_data {
                return None;
            }
            return vanilla_painting_variant_for_resource_id(&entry.id)
                .map(presentation_from_vanilla)
                .map(Cow::Owned);
        }

        if self.registry_content(PAINTING_VARIANT_REGISTRY).is_some() {
            return None;
        }
        // `minecraft:painting_variant` is data-loaded, so the fallback holder id
        // is a sorted-resource-id index rather than the bootstrap one the
        // catalog is written in. Resolve the id first, then look the variant up
        // by name.
        self.registry_resource_id_by_holder_id(PAINTING_VARIANT_REGISTRY, holder_id)
            .and_then(vanilla_painting_variant_for_resource_id)
            .map(presentation_from_vanilla)
            .map(Cow::Owned)
    }
}

fn client_projection_from_presentation(
    variant: Cow<'_, PaintingVariantPresentation>,
) -> PaintingVariantClientProjection {
    PaintingVariantClientProjection {
        width: variant.width,
        height: variant.height,
        asset_id: match variant {
            Cow::Borrowed(variant) => variant.asset_id.clone(),
            Cow::Owned(variant) => variant.asset_id,
        },
    }
}

fn presentation_from_vanilla(variant: &VanillaPaintingVariant) -> PaintingVariantPresentation {
    PaintingVariantPresentation {
        width: variant.width,
        height: variant.height,
        asset_id: variant.resource_id.to_string(),
        title: None,
        title_styled: Vec::new(),
        title_nbt: None,
        title_translation_key: painting_variant_translation_key(variant.resource_id, "title"),
        author: None,
        author_styled: Vec::new(),
        author_nbt: None,
        author_translation_key: if variant.has_author {
            painting_variant_translation_key(variant.resource_id, "author")
        } else {
            None
        },
    }
}

pub(super) fn decode_validated_painting_variant(
    value: NbtSummaryValue,
) -> PaintingVariantPresentation {
    let NbtSummaryValue::Compound(mut fields) = value else {
        unreachable!("canonical painting variant");
    };
    let width = field(&fields, "width")
        .and_then(numeric_i32)
        .expect("canonical painting width");
    let height = field(&fields, "height")
        .and_then(numeric_i32)
        .expect("canonical painting height");
    let Some(NbtSummaryValue::String(asset_id)) = field(&fields, "asset_id") else {
        unreachable!("canonical painting asset id");
    };
    let asset_id = asset_id
        .try_to_string()
        .expect("canonical painting asset id");
    let (title, title_styled, title_nbt) = optional_component(&mut fields, "title");
    let (author, author_styled, author_nbt) = optional_component(&mut fields, "author");
    PaintingVariantPresentation {
        width,
        height,
        asset_id,
        title,
        title_styled,
        title_nbt,
        title_translation_key: None,
        author,
        author_styled,
        author_nbt,
        author_translation_key: None,
    }
}

fn optional_component(
    fields: &mut Vec<NbtSummaryEntry>,
    name: &str,
) -> (Option<String>, Vec<StyledTextRun>, Option<NbtSummaryValue>) {
    let Some(index) = fields.iter().position(|entry| entry.name == name) else {
        return (None, Vec::new(), None);
    };
    let value = fields.swap_remove(index).value;
    let styled = styled_component_runs_from_nbt_value(&value);
    let text = styled_runs_summary_text(&styled);
    (Some(text), styled, Some(value))
}

pub fn painting_variant_translation_key(resource_id: &str, suffix: &str) -> Option<String> {
    let (namespace, path) = resource_id.split_once(':')?;
    Some(format!(
        "painting.{namespace}.{}.{suffix}",
        path.replace('/', ".")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPacketEntry;

    #[test]
    fn decodes_synchronized_direct_codec_fields() {
        let mut store = WorldStore::new();
        store.begin_registry_configuration();
        super::super::tests::record_minimal_non_empty_synchronized_registries(
            &mut store,
            Some(PAINTING_VARIANT_REGISTRY),
        );
        store.record_registry_entries(
            PAINTING_VARIANT_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "example:wide",
                painting_variant_nbt(4, 2, "example:gallery/wide"),
            )],
        );
        store.finish_registry_configuration().unwrap();

        assert_eq!(
            store.painting_variant_client_projection(0),
            Some(PaintingVariantClientProjection {
                width: 4,
                height: 2,
                asset_id: "example:gallery/wide".to_string(),
            })
        );
    }

    #[test]
    fn presentation_retains_optional_components_and_static_translation_keys() {
        let mut store = WorldStore::new();
        store.begin_registry_configuration();
        super::super::tests::record_minimal_non_empty_synchronized_registries(
            &mut store,
            Some(PAINTING_VARIANT_REGISTRY),
        );
        store.record_registry_entries(
            PAINTING_VARIANT_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "example:wide",
                painting_variant_nbt_with_components(
                    4,
                    2,
                    "example:gallery/wide",
                    Some("Custom title"),
                    Some("Custom author"),
                ),
            )],
        );
        store.finish_registry_configuration().unwrap();

        let presentation = store.painting_variant_presentation(0).unwrap();
        assert_eq!(presentation.title.as_deref(), Some("Custom title"));
        assert_eq!(presentation.author.as_deref(), Some("Custom author"));
        assert_eq!(presentation.title_styled[0].text, "Custom title");
        assert_eq!(presentation.author_styled[0].text, "Custom author");
        assert_eq!(presentation.title_translation_key, None);
        assert_eq!(presentation.author_translation_key, None);

        // Without a synchronized registry the holder id is the loader-order
        // index, whose head is `minecraft:alban`, not the bootstrap head
        // `minecraft:kebab`.
        let vanilla_store = WorldStore::new();
        let vanilla = vanilla_store.painting_variant_presentation(0).unwrap();
        assert_eq!(
            vanilla.title_translation_key.as_deref(),
            Some("painting.minecraft.alban.title")
        );
        assert_eq!(
            vanilla.author_translation_key.as_deref(),
            Some("painting.minecraft.alban.author")
        );
        let kebab = vanilla_store.painting_variant_presentation(24).unwrap();
        assert_eq!(
            kebab.title_translation_key.as_deref(),
            Some("painting.minecraft.kebab.title")
        );
    }

    #[test]
    fn raw_title_and_author_survive_painting_projection() {
        let entry = |name: &str, value| NbtSummaryEntry {
            name: name.into(),
            value,
        };
        let title = NbtSummaryValue::Compound(vec![entry(
            "translate",
            NbtSummaryValue::string_from_utf16(vec![0xd800]),
        )]);
        let author = NbtSummaryValue::Compound(vec![entry(
            "translate",
            NbtSummaryValue::string_from_utf16(vec![0xd801]),
        )]);
        let source = NbtSummaryValue::Compound(vec![
            entry("width", NbtSummaryValue::Int(2)),
            entry("height", NbtSummaryValue::Int(3)),
            entry("asset_id", NbtSummaryValue::String("example:art".into())),
            entry("title", title.clone()),
            entry("author", author.clone()),
        ]);
        let data = bbb_registry_codec::RegistryElementData::from_network_nbt(source);
        let registries = crate::RegistrySet::vanilla_26_1();
        let value = super::super::revision::exact_network_value(
            PAINTING_VARIANT_REGISTRY,
            &data,
            registries.frozen_access(),
        )
        .unwrap();
        let presentation = decode_validated_painting_variant(value);
        assert_eq!(presentation.title_nbt, Some(title));
        assert_eq!(presentation.author_nbt, Some(author));
        let encoded = serde_json::to_string(&presentation).unwrap();
        assert_eq!(
            serde_json::from_str::<PaintingVariantPresentation>(&encoded).unwrap(),
            presentation
        );
        let mut old_json = bbb_protocol::to_display_json_value(&presentation).unwrap();
        let fields = old_json.as_object_mut().unwrap();
        fields.remove("title_nbt");
        fields.remove("author_nbt");
        let restored: PaintingVariantPresentation = serde_json::from_value(old_json).unwrap();
        assert_eq!(restored.title_nbt, None);
        assert_eq!(restored.author_nbt, None);
    }

    #[test]
    fn stub_resolution_uses_resource_identity_in_reordered_registry() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            PAINTING_VARIANT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::stub("minecraft:dennis"),
                RegistryPacketEntry::stub("minecraft:kebab"),
            ],
        );

        let dennis = store.painting_variant_client_projection(0).unwrap();
        let kebab = store.painting_variant_client_projection(1).unwrap();
        assert_eq!((dennis.width, dennis.height), (3, 3));
        assert_eq!((kebab.width, kebab.height), (1, 1));
    }

    #[test]
    fn missing_synchronized_registry_uses_fixed_vanilla_order() {
        let store = WorldStore::new();

        // 26 is `minecraft:match` in loader order; 14 is its bootstrap index.
        let match_variant = store.painting_variant_client_projection(26).unwrap();
        assert_eq!((match_variant.width, match_variant.height), (2, 2));
        assert_eq!(match_variant.asset_id, "minecraft:match");
        assert_ne!(
            store
                .painting_variant_client_projection(14)
                .unwrap()
                .asset_id,
            "minecraft:match"
        );
    }

    #[test]
    fn rejects_invalid_persistent_shape_without_ordinal_substitution() {
        let mut store = WorldStore::new();
        store.begin_registry_configuration();
        super::super::tests::record_minimal_non_empty_synchronized_registries(
            &mut store,
            Some(PAINTING_VARIANT_REGISTRY),
        );
        store.record_registry_entries(
            PAINTING_VARIANT_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "minecraft:kebab",
                painting_variant_nbt(17, 1, "minecraft:kebab"),
            )],
        );

        assert!(store.finish_registry_configuration().is_err());
    }

    fn painting_variant_nbt(width: i32, height: i32, asset_id: &str) -> Vec<u8> {
        painting_variant_nbt_with_components(width, height, asset_id, None, None)
    }

    fn painting_variant_nbt_with_components(
        width: i32,
        height: i32,
        asset_id: &str,
        title: Option<&str>,
        author: Option<&str>,
    ) -> Vec<u8> {
        let mut out = vec![10];
        write_int(&mut out, "width", width);
        write_int(&mut out, "height", height);
        write_string(&mut out, "asset_id", asset_id);
        if let Some(title) = title {
            write_string(&mut out, "title", title);
        }
        if let Some(author) = author {
            write_string(&mut out, "author", author);
        }
        out.push(0);
        out
    }

    fn write_int(out: &mut Vec<u8>, name: &str, value: i32) {
        out.push(3);
        write_nbt_string(out, name);
        out.extend_from_slice(&value.to_be_bytes());
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
