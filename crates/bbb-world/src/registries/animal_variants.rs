use crate::{
    chunks::{decode_nbt_root, find_nbt_entry, NbtValue},
    WorldStore,
};

use super::normalize_codec_identifier;

pub const CHICKEN_VARIANT_REGISTRY: &str = "minecraft:chicken_variant";
pub const COW_VARIANT_REGISTRY: &str = "minecraft:cow_variant";
pub const PIG_VARIANT_REGISTRY: &str = "minecraft:pig_variant";
pub const CAT_VARIANT_REGISTRY: &str = "minecraft:cat_variant";
pub const FROG_VARIANT_REGISTRY: &str = "minecraft:frog_variant";
pub const ZOMBIE_NAUTILUS_VARIANT_REGISTRY: &str = "minecraft:zombie_nautilus_variant";

/// The serialized model selector shared by the chicken, cow, and pig variant codecs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FarmAnimalVariantModel {
    Normal,
    Cold,
    Warm,
}

/// The client presentation fields carried by one synchronized farm-animal variant element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FarmAnimalVariantPresentation {
    pub model: FarmAnimalVariantModel,
    pub adult_asset_id: String,
    pub adult_texture_path: String,
    pub baby_asset_id: String,
    pub baby_texture_path: String,
}

/// Identifies the exact NETWORK_CODEC field that could not be projected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FarmAnimalVariantProjectionError {
    pub field: String,
    pub reason: &'static str,
}

/// The client presentation fields carried by one synchronized cat variant element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatVariantPresentation {
    pub adult_asset_id: String,
    pub adult_texture_path: String,
    pub baby_asset_id: String,
    pub baby_texture_path: String,
}

/// Identifies the exact cat `NETWORK_CODEC` field that could not be projected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatVariantProjectionError {
    pub field: String,
    pub reason: &'static str,
}

/// The client presentation field carried by one synchronized frog variant element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrogVariantPresentation {
    pub asset_id: String,
    pub texture_path: String,
}

/// Identifies the exact frog `NETWORK_CODEC` field that could not be projected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrogVariantProjectionError {
    pub field: String,
    pub reason: &'static str,
}

/// The model selector serialized by `ZombieNautilusVariant.ModelType.CODEC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZombieNautilusVariantModelType {
    Normal,
    Warm,
}

/// The client presentation fields carried by one synchronized zombie-nautilus variant element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZombieNautilusVariantPresentation {
    pub model: ZombieNautilusVariantModelType,
    pub asset_id: String,
    pub texture_path: String,
}

/// Identifies the exact zombie-nautilus `NETWORK_CODEC` field that could not be projected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZombieNautilusVariantProjectionError {
    pub field: String,
    pub reason: &'static str,
}

impl WorldStore {
    /// Decodes a synchronized chicken/cow/pig variant element in current server holder order.
    /// Known-pack stubs remain `None`; their built-in presentation is selected by entry identity.
    pub fn synchronized_farm_animal_variant_presentation(
        &self,
        registry: &str,
        holder_id: i32,
    ) -> Result<Option<FarmAnimalVariantPresentation>, FarmAnimalVariantProjectionError> {
        ensure_supported_registry(registry)?;
        let Some(entry) = self.registry_entry_by_holder_id(registry, holder_id) else {
            return Ok(None);
        };
        let Some(raw_data) = entry.raw_data() else {
            return Ok(None);
        };
        decode_farm_animal_variant(registry, raw_data).map(Some)
    }

    /// Decodes a synchronized cat variant element in current server holder order.
    /// Known-pack stubs remain `None`; their built-in presentation is selected by entry identity.
    pub fn synchronized_cat_variant_presentation(
        &self,
        holder_id: i32,
    ) -> Result<Option<CatVariantPresentation>, CatVariantProjectionError> {
        let Some(entry) = self.registry_entry_by_holder_id(CAT_VARIANT_REGISTRY, holder_id) else {
            return Ok(None);
        };
        let Some(raw_data) = entry.raw_data() else {
            return Ok(None);
        };
        decode_cat_variant(raw_data).map(Some)
    }

    /// Decodes a synchronized frog variant element in current server holder order.
    /// Known-pack stubs remain `None`; their built-in presentation is selected by entry identity.
    pub fn synchronized_frog_variant_presentation(
        &self,
        holder_id: i32,
    ) -> Result<Option<FrogVariantPresentation>, FrogVariantProjectionError> {
        let Some(entry) = self.registry_entry_by_holder_id(FROG_VARIANT_REGISTRY, holder_id) else {
            return Ok(None);
        };
        let Some(raw_data) = entry.raw_data() else {
            return Ok(None);
        };
        decode_frog_variant(raw_data).map(Some)
    }

    /// Decodes a synchronized zombie-nautilus variant in current server holder order.
    /// Known-pack stubs remain `None`; their built-in presentation is selected by entry identity.
    pub fn synchronized_zombie_nautilus_variant_presentation(
        &self,
        holder_id: i32,
    ) -> Result<Option<ZombieNautilusVariantPresentation>, ZombieNautilusVariantProjectionError>
    {
        let Some(entry) =
            self.registry_entry_by_holder_id(ZOMBIE_NAUTILUS_VARIANT_REGISTRY, holder_id)
        else {
            return Ok(None);
        };
        let Some(raw_data) = entry.raw_data() else {
            return Ok(None);
        };
        decode_zombie_nautilus_variant(raw_data).map(Some)
    }
}

fn decode_farm_animal_variant(
    registry: &str,
    raw_data: &[u8],
) -> Result<FarmAnimalVariantPresentation, FarmAnimalVariantProjectionError> {
    ensure_supported_registry(registry)?;
    let root = decode_nbt_root(raw_data)
        .map_err(|_| error("<root>", "invalid NBT"))?
        .ok_or_else(|| error("<root>", "missing root compound"))?;
    let NbtValue::Compound(fields) = root else {
        return Err(error("<root>", "expected compound"));
    };

    let model = match find_nbt_entry(&fields, "model") {
        None => FarmAnimalVariantModel::Normal,
        Some(NbtValue::String(model)) => decode_model(registry, model)?,
        Some(_) => return Err(error("model", "expected string")),
    };
    let (adult_asset_id, adult_texture_path) = decode_resource_texture(&fields, "asset_id")?;
    let (baby_asset_id, baby_texture_path) = decode_resource_texture(&fields, "baby_asset_id")?;

    Ok(FarmAnimalVariantPresentation {
        model,
        adult_asset_id,
        adult_texture_path,
        baby_asset_id,
        baby_texture_path,
    })
}

fn decode_cat_variant(
    raw_data: &[u8],
) -> Result<CatVariantPresentation, CatVariantProjectionError> {
    let root = decode_nbt_root(raw_data)
        .map_err(|_| cat_error("<root>", "invalid NBT"))?
        .ok_or_else(|| cat_error("<root>", "missing root compound"))?;
    let NbtValue::Compound(fields) = root else {
        return Err(cat_error("<root>", "expected compound"));
    };

    let (adult_asset_id, adult_texture_path) =
        decode_resource_texture(&fields, "asset_id").map_err(CatVariantProjectionError::from)?;
    let (baby_asset_id, baby_texture_path) = decode_resource_texture(&fields, "baby_asset_id")
        .map_err(CatVariantProjectionError::from)?;
    Ok(CatVariantPresentation {
        adult_asset_id,
        adult_texture_path,
        baby_asset_id,
        baby_texture_path,
    })
}

fn decode_frog_variant(
    raw_data: &[u8],
) -> Result<FrogVariantPresentation, FrogVariantProjectionError> {
    let root = decode_nbt_root(raw_data)
        .map_err(|_| frog_error("<root>", "invalid NBT"))?
        .ok_or_else(|| frog_error("<root>", "missing root compound"))?;
    let NbtValue::Compound(fields) = root else {
        return Err(frog_error("<root>", "expected compound"));
    };

    let (asset_id, texture_path) =
        decode_resource_texture(&fields, "asset_id").map_err(FrogVariantProjectionError::from)?;
    Ok(FrogVariantPresentation {
        asset_id,
        texture_path,
    })
}

fn decode_zombie_nautilus_variant(
    raw_data: &[u8],
) -> Result<ZombieNautilusVariantPresentation, ZombieNautilusVariantProjectionError> {
    let root = decode_nbt_root(raw_data)
        .map_err(|_| zombie_nautilus_error("<root>", "invalid NBT"))?
        .ok_or_else(|| zombie_nautilus_error("<root>", "missing root compound"))?;
    let NbtValue::Compound(fields) = root else {
        return Err(zombie_nautilus_error("<root>", "expected compound"));
    };

    let model = match find_nbt_entry(&fields, "model") {
        None => ZombieNautilusVariantModelType::Normal,
        Some(NbtValue::String(model)) if model == "normal" => {
            ZombieNautilusVariantModelType::Normal
        }
        Some(NbtValue::String(model)) if model == "warm" => ZombieNautilusVariantModelType::Warm,
        Some(NbtValue::String(_)) => {
            return Err(zombie_nautilus_error("model", "unknown model type"));
        }
        Some(_) => return Err(zombie_nautilus_error("model", "expected string")),
    };
    let (asset_id, texture_path) = decode_resource_texture(&fields, "asset_id")
        .map_err(ZombieNautilusVariantProjectionError::from)?;
    Ok(ZombieNautilusVariantPresentation {
        model,
        asset_id,
        texture_path,
    })
}

fn ensure_supported_registry(registry: &str) -> Result<(), FarmAnimalVariantProjectionError> {
    match registry {
        CHICKEN_VARIANT_REGISTRY | COW_VARIANT_REGISTRY | PIG_VARIANT_REGISTRY => Ok(()),
        _ => Err(error("<registry>", "unsupported farm-animal registry")),
    }
}

fn decode_model(
    registry: &str,
    model: &str,
) -> Result<FarmAnimalVariantModel, FarmAnimalVariantProjectionError> {
    match model {
        "normal" => Ok(FarmAnimalVariantModel::Normal),
        "cold" => Ok(FarmAnimalVariantModel::Cold),
        "warm" if registry == COW_VARIANT_REGISTRY => Ok(FarmAnimalVariantModel::Warm),
        _ => Err(error("model", "unsupported model type")),
    }
}

fn decode_resource_texture(
    fields: &[(String, NbtValue)],
    field: &str,
) -> Result<(String, String), ResourceTextureProjectionError> {
    let raw_asset_id = match find_nbt_entry(fields, field) {
        Some(NbtValue::String(asset_id)) => asset_id,
        Some(_) => return Err(resource_texture_error(field, "expected string")),
        None => return Err(resource_texture_error(field, "missing required field")),
    };
    let asset_id = normalize_codec_identifier(raw_asset_id)
        .ok_or_else(|| resource_texture_error(field, "invalid texture identifier"))?;
    let (namespace, path) = asset_id
        .split_once(':')
        .expect("normalized identifier always has a namespace");
    let texture_path = format!("{namespace}:textures/{path}.png");
    Ok((asset_id, texture_path))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResourceTextureProjectionError {
    field: String,
    reason: &'static str,
}

impl From<ResourceTextureProjectionError> for FarmAnimalVariantProjectionError {
    fn from(error: ResourceTextureProjectionError) -> Self {
        Self {
            field: error.field,
            reason: error.reason,
        }
    }
}

impl From<ResourceTextureProjectionError> for CatVariantProjectionError {
    fn from(error: ResourceTextureProjectionError) -> Self {
        Self {
            field: error.field,
            reason: error.reason,
        }
    }
}

impl From<ResourceTextureProjectionError> for FrogVariantProjectionError {
    fn from(error: ResourceTextureProjectionError) -> Self {
        Self {
            field: error.field,
            reason: error.reason,
        }
    }
}

impl From<ResourceTextureProjectionError> for ZombieNautilusVariantProjectionError {
    fn from(error: ResourceTextureProjectionError) -> Self {
        Self {
            field: error.field,
            reason: error.reason,
        }
    }
}

fn resource_texture_error(
    field: impl Into<String>,
    reason: &'static str,
) -> ResourceTextureProjectionError {
    ResourceTextureProjectionError {
        field: field.into(),
        reason,
    }
}

fn error(field: impl Into<String>, reason: &'static str) -> FarmAnimalVariantProjectionError {
    FarmAnimalVariantProjectionError {
        field: field.into(),
        reason,
    }
}

fn cat_error(field: impl Into<String>, reason: &'static str) -> CatVariantProjectionError {
    CatVariantProjectionError {
        field: field.into(),
        reason,
    }
}

fn frog_error(field: impl Into<String>, reason: &'static str) -> FrogVariantProjectionError {
    FrogVariantProjectionError {
        field: field.into(),
        reason,
    }
}

fn zombie_nautilus_error(
    field: impl Into<String>,
    reason: &'static str,
) -> ZombieNautilusVariantProjectionError {
    ZombieNautilusVariantProjectionError {
        field: field.into(),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPacketEntry;

    #[test]
    fn decodes_all_farm_animal_network_codec_shapes() {
        assert_eq!(
            decode_farm_animal_variant(
                CHICKEN_VARIANT_REGISTRY,
                &variant_nbt(None, Some("entity/chicken/custom"), Some("example:chick")),
            )
            .unwrap(),
            FarmAnimalVariantPresentation {
                model: FarmAnimalVariantModel::Normal,
                adult_asset_id: "minecraft:entity/chicken/custom".to_string(),
                adult_texture_path: "minecraft:textures/entity/chicken/custom.png".to_string(),
                baby_asset_id: "example:chick".to_string(),
                baby_texture_path: "example:textures/chick.png".to_string(),
            }
        );
        assert_eq!(
            decode_farm_animal_variant(
                COW_VARIANT_REGISTRY,
                &variant_nbt(
                    Some("warm"),
                    Some("example:cattle/adult"),
                    Some("example:cattle/baby"),
                ),
            )
            .unwrap()
            .model,
            FarmAnimalVariantModel::Warm
        );
        assert_eq!(
            decode_farm_animal_variant(
                PIG_VARIANT_REGISTRY,
                &variant_nbt(
                    Some("cold"),
                    Some("example:swine/adult"),
                    Some("example:swine/baby"),
                ),
            )
            .unwrap()
            .model,
            FarmAnimalVariantModel::Cold
        );
    }

    #[test]
    fn rejects_invalid_model_and_texture_fields() {
        assert_eq!(
            decode_farm_animal_variant(
                CHICKEN_VARIANT_REGISTRY,
                &variant_nbt(Some("warm"), Some("example:adult"), Some("example:baby")),
            )
            .unwrap_err(),
            FarmAnimalVariantProjectionError {
                field: "model".to_string(),
                reason: "unsupported model type",
            }
        );
        assert_eq!(
            decode_farm_animal_variant(
                COW_VARIANT_REGISTRY,
                &variant_nbt(Some("normal"), Some("Example:adult"), Some("example:baby")),
            )
            .unwrap_err(),
            FarmAnimalVariantProjectionError {
                field: "asset_id".to_string(),
                reason: "invalid texture identifier",
            }
        );
        assert_eq!(
            decode_farm_animal_variant(
                PIG_VARIANT_REGISTRY,
                &variant_nbt(Some("normal"), Some("example:adult"), None),
            )
            .unwrap_err(),
            FarmAnimalVariantProjectionError {
                field: "baby_asset_id".to_string(),
                reason: "missing required field",
            }
        );
    }

    #[test]
    fn decodes_cat_network_codec_shape_and_rejects_invalid_assets() {
        assert_eq!(
            decode_cat_variant(&variant_nbt(
                None,
                Some("example:entity/cat/lynx"),
                Some("entity/cat/lynx_baby"),
            ))
            .unwrap(),
            CatVariantPresentation {
                adult_asset_id: "example:entity/cat/lynx".to_string(),
                adult_texture_path: "example:textures/entity/cat/lynx.png".to_string(),
                baby_asset_id: "minecraft:entity/cat/lynx_baby".to_string(),
                baby_texture_path: "minecraft:textures/entity/cat/lynx_baby.png".to_string(),
            }
        );
        assert_eq!(
            decode_cat_variant(&variant_nbt(
                None,
                Some("example:entity/cat/lynx"),
                Some("Example:entity/cat/lynx_baby"),
            ))
            .unwrap_err(),
            CatVariantProjectionError {
                field: "baby_asset_id".to_string(),
                reason: "invalid texture identifier",
            }
        );
    }

    #[test]
    fn decodes_frog_network_codec_shape_and_rejects_invalid_assets() {
        assert_eq!(
            decode_frog_variant(&variant_nbt(None, Some("example:entity/frog/blue"), None,))
                .unwrap(),
            FrogVariantPresentation {
                asset_id: "example:entity/frog/blue".to_string(),
                texture_path: "example:textures/entity/frog/blue.png".to_string(),
            }
        );
        assert_eq!(
            decode_frog_variant(&variant_nbt(None, Some("Example:entity/frog/blue"), None,))
                .unwrap_err(),
            FrogVariantProjectionError {
                field: "asset_id".to_string(),
                reason: "invalid texture identifier",
            }
        );
    }

    #[test]
    fn decodes_zombie_nautilus_network_codec_model_and_texture() {
        assert_eq!(
            decode_zombie_nautilus_variant(&variant_nbt(
                Some("warm"),
                Some("example:entity/nautilus/coral"),
                None,
            ))
            .unwrap(),
            ZombieNautilusVariantPresentation {
                model: ZombieNautilusVariantModelType::Warm,
                asset_id: "example:entity/nautilus/coral".to_string(),
                texture_path: "example:textures/entity/nautilus/coral.png".to_string(),
            }
        );
        assert_eq!(
            decode_zombie_nautilus_variant(&variant_nbt(
                None,
                Some("example:entity/nautilus/plain"),
                None,
            ))
            .unwrap()
            .model,
            ZombieNautilusVariantModelType::Normal
        );
        assert_eq!(
            decode_zombie_nautilus_variant(&variant_nbt(
                Some("cold"),
                Some("example:entity/nautilus/plain"),
                None,
            ))
            .unwrap_err(),
            ZombieNautilusVariantProjectionError {
                field: "model".to_string(),
                reason: "unknown model type",
            }
        );
    }

    #[test]
    fn world_query_uses_server_holder_order_and_omits_stubs() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            COW_VARIANT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:custom",
                    variant_nbt(
                        Some("warm"),
                        Some("example:cow/adult"),
                        Some("example:cow/baby"),
                    ),
                ),
                RegistryPacketEntry::stub("minecraft:temperate"),
            ],
        );

        assert_eq!(
            store
                .synchronized_farm_animal_variant_presentation(COW_VARIANT_REGISTRY, 0)
                .unwrap(),
            Some(FarmAnimalVariantPresentation {
                model: FarmAnimalVariantModel::Warm,
                adult_asset_id: "example:cow/adult".to_string(),
                adult_texture_path: "example:textures/cow/adult.png".to_string(),
                baby_asset_id: "example:cow/baby".to_string(),
                baby_texture_path: "example:textures/cow/baby.png".to_string(),
            })
        );
        assert_eq!(
            store
                .synchronized_farm_animal_variant_presentation(COW_VARIANT_REGISTRY, 1)
                .unwrap(),
            None
        );
        assert_eq!(
            store
                .synchronized_farm_animal_variant_presentation(COW_VARIANT_REGISTRY, 2)
                .unwrap(),
            None
        );
    }

    #[test]
    fn cat_world_query_uses_server_holder_order_and_omits_stubs() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            CAT_VARIANT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:lynx",
                    variant_nbt(
                        None,
                        Some("example:entity/cat/lynx"),
                        Some("example:entity/cat/lynx_baby"),
                    ),
                ),
                RegistryPacketEntry::stub("minecraft:tabby"),
            ],
        );

        assert_eq!(
            store.synchronized_cat_variant_presentation(0).unwrap(),
            Some(CatVariantPresentation {
                adult_asset_id: "example:entity/cat/lynx".to_string(),
                adult_texture_path: "example:textures/entity/cat/lynx.png".to_string(),
                baby_asset_id: "example:entity/cat/lynx_baby".to_string(),
                baby_texture_path: "example:textures/entity/cat/lynx_baby.png".to_string(),
            })
        );
        assert_eq!(
            store.synchronized_cat_variant_presentation(1).unwrap(),
            None
        );
        assert_eq!(
            store.synchronized_cat_variant_presentation(2).unwrap(),
            None
        );
    }

    #[test]
    fn frog_world_query_uses_server_holder_order_and_omits_stubs() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            FROG_VARIANT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:blue",
                    variant_nbt(None, Some("example:entity/frog/blue"), None),
                ),
                RegistryPacketEntry::stub("minecraft:cold"),
            ],
        );

        assert_eq!(
            store.synchronized_frog_variant_presentation(0).unwrap(),
            Some(FrogVariantPresentation {
                asset_id: "example:entity/frog/blue".to_string(),
                texture_path: "example:textures/entity/frog/blue.png".to_string(),
            })
        );
        assert_eq!(
            store.synchronized_frog_variant_presentation(1).unwrap(),
            None
        );
        assert_eq!(
            store.synchronized_frog_variant_presentation(2).unwrap(),
            None
        );
    }

    #[test]
    fn zombie_nautilus_world_query_uses_server_holder_order_and_omits_stubs() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            ZOMBIE_NAUTILUS_VARIANT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:coral",
                    variant_nbt(Some("warm"), Some("example:entity/nautilus/coral"), None),
                ),
                RegistryPacketEntry::stub("minecraft:temperate"),
            ],
        );

        assert_eq!(
            store
                .synchronized_zombie_nautilus_variant_presentation(0)
                .unwrap(),
            Some(ZombieNautilusVariantPresentation {
                model: ZombieNautilusVariantModelType::Warm,
                asset_id: "example:entity/nautilus/coral".to_string(),
                texture_path: "example:textures/entity/nautilus/coral.png".to_string(),
            })
        );
        assert_eq!(
            store
                .synchronized_zombie_nautilus_variant_presentation(1)
                .unwrap(),
            None
        );
        assert_eq!(
            store
                .synchronized_zombie_nautilus_variant_presentation(2)
                .unwrap(),
            None
        );
    }

    fn variant_nbt(
        model: Option<&str>,
        asset_id: Option<&str>,
        baby_asset_id: Option<&str>,
    ) -> Vec<u8> {
        let mut out = vec![10];
        if let Some(model) = model {
            write_string(&mut out, "model", model);
        }
        if let Some(asset_id) = asset_id {
            write_string(&mut out, "asset_id", asset_id);
        }
        if let Some(baby_asset_id) = baby_asset_id {
            write_string(&mut out, "baby_asset_id", baby_asset_id);
        }
        out.push(0);
        out
    }

    fn write_string(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_utf(out, name);
        write_utf(out, value);
    }

    fn write_utf(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
