use bbb_protocol::{
    styled_component_runs_from_nbt_value, styled_runs_summary_text, validate_component_nbt,
    NbtSummaryEntry, NbtSummaryValue, StyledTextRun,
};
use serde::{Deserialize, Serialize};

use crate::{
    painting_variants::{
        vanilla_painting_variant, vanilla_painting_variant_for_resource_id, VanillaPaintingVariant,
    },
    WorldStore,
};

use super::normalize_codec_identifier;

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
    pub title_translation_key: Option<String>,
    pub author: Option<String>,
    pub author_styled: Vec<StyledTextRun>,
    pub author_translation_key: Option<String>,
}

/// Identifies the exact `PaintingVariant.DIRECT_CODEC` field that could not be projected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaintingVariantProjectionError {
    pub field: String,
    pub reason: &'static str,
}

impl std::fmt::Display for PaintingVariantProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.field, self.reason)
    }
}

impl WorldStore {
    /// Resolves a painting variant through the current server holder order. Materialized
    /// synchronized entries use their own codec fields, unresolved data-less stubs use resource
    /// identity, and only a session with no synchronized contents uses the fixed vanilla order.
    pub fn painting_variant_client_projection(
        &self,
        holder_id: i32,
    ) -> Result<Option<PaintingVariantClientProjection>, PaintingVariantProjectionError> {
        self.painting_variant_presentation(holder_id)
            .map(|presentation| presentation.map(client_projection_from_presentation))
    }

    /// Resolves the complete client-consumed painting presentation through the current server
    /// holder order. A synchronized registry is authoritative; only unresolved vanilla data-less
    /// stubs use their resource identity, and the fixed bootstrap order is used only when no
    /// synchronized painting registry exists.
    pub fn painting_variant_presentation(
        &self,
        holder_id: i32,
    ) -> Result<Option<PaintingVariantPresentation>, PaintingVariantProjectionError> {
        if let Some(entry) = self.registry_entry_by_holder_id(PAINTING_VARIANT_REGISTRY, holder_id)
        {
            if let Some(data) = entry.element_data() {
                return decode_painting_variant(&data.value).map(Some);
            }
            if entry.has_data {
                return Ok(None);
            }
            return Ok(
                vanilla_painting_variant_for_resource_id(&entry.id).map(presentation_from_vanilla)
            );
        }

        if self.registry_content(PAINTING_VARIANT_REGISTRY).is_some() {
            return Ok(None);
        }
        Ok(vanilla_painting_variant(holder_id).map(presentation_from_vanilla))
    }
}

fn client_projection_from_presentation(
    variant: PaintingVariantPresentation,
) -> PaintingVariantClientProjection {
    PaintingVariantClientProjection {
        width: variant.width,
        height: variant.height,
        asset_id: variant.asset_id,
    }
}

fn presentation_from_vanilla(variant: &VanillaPaintingVariant) -> PaintingVariantPresentation {
    PaintingVariantPresentation {
        width: variant.width,
        height: variant.height,
        asset_id: variant.resource_id.to_string(),
        title: None,
        title_styled: Vec::new(),
        title_translation_key: painting_variant_translation_key(variant.resource_id, "title"),
        author: None,
        author_styled: Vec::new(),
        author_translation_key: if variant.has_author {
            painting_variant_translation_key(variant.resource_id, "author")
        } else {
            None
        },
    }
}

fn decode_painting_variant(
    value: &NbtSummaryValue,
) -> Result<PaintingVariantPresentation, PaintingVariantProjectionError> {
    let NbtSummaryValue::Compound(fields) = value else {
        return Err(error("<root>", "expected compound"));
    };
    let width = required_size(fields, "width")?;
    let height = required_size(fields, "height")?;
    let asset_id = match field(fields, "asset_id") {
        Some(NbtSummaryValue::String(asset_id)) => normalize_codec_identifier(asset_id)
            .ok_or_else(|| error("asset_id", "invalid identifier"))?,
        Some(_) => return Err(error("asset_id", "expected string")),
        None => return Err(error("asset_id", "missing required field")),
    };
    let (title, title_styled) = optional_component(fields, "title")?;
    let (author, author_styled) = optional_component(fields, "author")?;
    Ok(PaintingVariantPresentation {
        width,
        height,
        asset_id,
        title,
        title_styled,
        title_translation_key: None,
        author,
        author_styled,
        author_translation_key: None,
    })
}

fn required_size(
    fields: &[NbtSummaryEntry],
    field: &str,
) -> Result<i32, PaintingVariantProjectionError> {
    match self::field(fields, field) {
        Some(value) => match numeric_i32(value) {
            Some(value) if (1..=16).contains(&value) => Ok(value),
            Some(_) => Err(error(field, "integer outside 1..=16")),
            None => Err(error(field, "expected integer")),
        },
        None => Err(error(field, "missing required field")),
    }
}

fn numeric_i32(value: &NbtSummaryValue) -> Option<i32> {
    match value {
        NbtSummaryValue::Byte(value) => Some(i32::from(*value)),
        NbtSummaryValue::Short(value) => Some(i32::from(*value)),
        NbtSummaryValue::Int(value) => Some(*value),
        NbtSummaryValue::Long(value) => i32::try_from(*value).ok(),
        _ => None,
    }
}

fn optional_component(
    fields: &[NbtSummaryEntry],
    name: &str,
) -> Result<(Option<String>, Vec<StyledTextRun>), PaintingVariantProjectionError> {
    let Some(value) = field(fields, name) else {
        return Ok((None, Vec::new()));
    };
    validate_component_nbt(value, "painting variant component")
        .map_err(|_| error(name, "invalid Component"))?;
    let styled = styled_component_runs_from_nbt_value(value);
    let text = styled_runs_summary_text(&styled);
    Ok((Some(text), styled))
}

fn field<'a>(fields: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    fields
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

pub fn painting_variant_translation_key(resource_id: &str, suffix: &str) -> Option<String> {
    let (namespace, path) = resource_id.split_once(':')?;
    Some(format!(
        "painting.{namespace}.{}.{suffix}",
        path.replace('/', ".")
    ))
}

fn error(field: impl Into<String>, reason: &'static str) -> PaintingVariantProjectionError {
    PaintingVariantProjectionError {
        field: field.into(),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPacketEntry;

    #[test]
    fn decodes_synchronized_direct_codec_fields() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            PAINTING_VARIANT_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "example:wide",
                painting_variant_nbt(4, 2, "example:gallery/wide"),
            )],
        );

        assert_eq!(
            store.painting_variant_client_projection(0).unwrap(),
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

        let presentation = store.painting_variant_presentation(0).unwrap().unwrap();
        assert_eq!(presentation.title.as_deref(), Some("Custom title"));
        assert_eq!(presentation.author.as_deref(), Some("Custom author"));
        assert_eq!(presentation.title_styled[0].text, "Custom title");
        assert_eq!(presentation.author_styled[0].text, "Custom author");
        assert_eq!(presentation.title_translation_key, None);
        assert_eq!(presentation.author_translation_key, None);

        let vanilla = WorldStore::new()
            .painting_variant_presentation(0)
            .unwrap()
            .unwrap();
        assert_eq!(
            vanilla.title_translation_key.as_deref(),
            Some("painting.minecraft.kebab.title")
        );
        assert_eq!(
            vanilla.author_translation_key.as_deref(),
            Some("painting.minecraft.kebab.author")
        );
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

        let dennis = store
            .painting_variant_client_projection(0)
            .unwrap()
            .unwrap();
        let kebab = store
            .painting_variant_client_projection(1)
            .unwrap()
            .unwrap();
        assert_eq!((dennis.width, dennis.height), (3, 3));
        assert_eq!((kebab.width, kebab.height), (1, 1));
    }

    #[test]
    fn missing_synchronized_registry_uses_fixed_vanilla_order() {
        let store = WorldStore::new();

        let match_variant = store
            .painting_variant_client_projection(14)
            .unwrap()
            .unwrap();
        assert_eq!((match_variant.width, match_variant.height), (2, 2));
        assert_eq!(match_variant.asset_id, "minecraft:match");
    }

    #[test]
    fn rejects_invalid_persistent_shape_without_ordinal_substitution() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            PAINTING_VARIANT_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "minecraft:kebab",
                painting_variant_nbt(17, 1, "minecraft:kebab"),
            )],
        );

        assert_eq!(
            store.painting_variant_client_projection(0).unwrap_err(),
            error("width", "integer outside 1..=16")
        );
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
