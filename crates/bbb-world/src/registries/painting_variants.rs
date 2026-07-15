use serde::{Deserialize, Serialize};

use crate::{
    chunks::{decode_nbt_root, find_nbt_entry, NbtValue},
    painting_variants::{vanilla_painting_variant, VanillaPaintingVariant},
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
    /// Resolves a painting variant through the current server holder order. Full synchronized
    /// entries use their own codec fields, known-pack stubs use resource identity, and only a
    /// session with no synchronized contents uses the fixed vanilla holder order.
    pub fn painting_variant_client_projection(
        &self,
        holder_id: i32,
    ) -> Result<Option<PaintingVariantClientProjection>, PaintingVariantProjectionError> {
        if let Some(entry) = self.registry_entry_by_holder_id(PAINTING_VARIANT_REGISTRY, holder_id)
        {
            if let Some(raw_data) = entry.raw_data() {
                return decode_painting_variant(raw_data).map(Some);
            }
            if entry.has_data {
                return Ok(None);
            }
            return Ok(vanilla_painting_variant_by_resource_id(&entry.id)
                .map(client_projection_from_vanilla));
        }

        if self.registry_content(PAINTING_VARIANT_REGISTRY).is_some() {
            return Ok(None);
        }
        Ok(vanilla_painting_variant(holder_id).map(client_projection_from_vanilla))
    }
}

fn vanilla_painting_variant_by_resource_id(
    resource_id: &str,
) -> Option<&'static VanillaPaintingVariant> {
    crate::painting_variants::VANILLA_PAINTING_VARIANTS
        .iter()
        .find(|variant| variant.resource_id == resource_id)
}

fn client_projection_from_vanilla(
    variant: &VanillaPaintingVariant,
) -> PaintingVariantClientProjection {
    PaintingVariantClientProjection {
        width: variant.width,
        height: variant.height,
        asset_id: variant.resource_id.to_string(),
    }
}

fn decode_painting_variant(
    raw_data: &[u8],
) -> Result<PaintingVariantClientProjection, PaintingVariantProjectionError> {
    let root = decode_nbt_root(raw_data)
        .map_err(|_| error("<root>", "invalid NBT"))?
        .ok_or_else(|| error("<root>", "missing root compound"))?;
    let NbtValue::Compound(fields) = root else {
        return Err(error("<root>", "expected compound"));
    };
    let width = required_size(&fields, "width")?;
    let height = required_size(&fields, "height")?;
    let asset_id = match find_nbt_entry(&fields, "asset_id") {
        Some(NbtValue::String(asset_id)) => normalize_codec_identifier(asset_id)
            .ok_or_else(|| error("asset_id", "invalid identifier"))?,
        Some(_) => return Err(error("asset_id", "expected string")),
        None => return Err(error("asset_id", "missing required field")),
    };
    Ok(PaintingVariantClientProjection {
        width,
        height,
        asset_id,
    })
}

fn required_size(
    fields: &[(String, NbtValue)],
    field: &str,
) -> Result<i32, PaintingVariantProjectionError> {
    match find_nbt_entry(fields, field) {
        Some(NbtValue::Int(value)) if (1..=16).contains(value) => Ok(*value),
        Some(NbtValue::Int(_)) => Err(error(field, "integer outside 1..=16")),
        Some(_) => Err(error(field, "expected integer")),
        None => Err(error(field, "missing required field")),
    }
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
        let mut out = vec![10];
        write_int(&mut out, "width", width);
        write_int(&mut out, "height", height);
        write_string(&mut out, "asset_id", asset_id);
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
