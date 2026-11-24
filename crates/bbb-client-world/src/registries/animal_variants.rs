use bbb_registry_codec::registry_codec::animal_variants::{
    AnimalVariantElement, CatVariantElement, ChickenModel, ChickenVariantElement,
    ClientAssetResourceTexture, CowModel, CowVariantElement, FrogVariantElement, PigModel,
    PigVariantElement, WolfAssetInfo, WolfVariantElement, ZombieNautilusModel,
    ZombieNautilusVariantElement,
};

use crate::WorldStore;

use super::state::RegistryTypedValues;

pub const CHICKEN_VARIANT_REGISTRY: &str = "minecraft:chicken_variant";
pub const COW_VARIANT_REGISTRY: &str = "minecraft:cow_variant";
pub const PIG_VARIANT_REGISTRY: &str = "minecraft:pig_variant";
pub const CAT_VARIANT_REGISTRY: &str = "minecraft:cat_variant";
pub const FROG_VARIANT_REGISTRY: &str = "minecraft:frog_variant";
pub const WOLF_VARIANT_REGISTRY: &str = "minecraft:wolf_variant";
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

/// The client presentation fields carried by one synchronized cat variant element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatVariantPresentation {
    pub adult_asset_id: String,
    pub adult_texture_path: String,
    pub baby_asset_id: String,
    pub baby_texture_path: String,
}

/// The client presentation field carried by one synchronized frog variant element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrogVariantPresentation {
    pub asset_id: String,
    pub texture_path: String,
}

/// The wild/tame/angry resource textures carried by one wolf age group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WolfVariantAssetPresentation {
    pub wild_asset_id: String,
    pub wild_texture_path: String,
    pub tame_asset_id: String,
    pub tame_texture_path: String,
    pub angry_asset_id: String,
    pub angry_texture_path: String,
}

/// The adult and baby client presentation fields carried by a synchronized wolf variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WolfVariantPresentation {
    pub adult: WolfVariantAssetPresentation,
    pub baby: WolfVariantAssetPresentation,
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

impl RegistryTypedValues {
    pub(super) fn insert_animal_variant(
        &mut self,
        resource_id: &str,
        element: AnimalVariantElement,
    ) {
        let (registry, presentation) = match element {
            AnimalVariantElement::Chicken(value) => {
                (CHICKEN_VARIANT_REGISTRY, adapt_chicken_variant(value))
            }
            AnimalVariantElement::Cow(value) => (COW_VARIANT_REGISTRY, adapt_cow_variant(value)),
            AnimalVariantElement::Pig(value) => (PIG_VARIANT_REGISTRY, adapt_pig_variant(value)),
            AnimalVariantElement::Cat(value) => {
                self.cat_variants
                    .insert(resource_id.to_string(), adapt_cat_variant(value));
                return;
            }
            AnimalVariantElement::Frog(value) => {
                self.frog_variants
                    .insert(resource_id.to_string(), adapt_frog_variant(value));
                return;
            }
            AnimalVariantElement::Wolf(value) => {
                self.wolf_variants
                    .insert(resource_id.to_string(), adapt_wolf_variant(value));
                return;
            }
            AnimalVariantElement::ZombieNautilus(value) => {
                self.zombie_nautilus_variants.insert(
                    resource_id.to_string(),
                    adapt_zombie_nautilus_variant(value),
                );
                return;
            }
        };
        self.farm_animal_variants
            .get_mut(registry)
            .expect("farm-animal registry initialized")
            .insert(resource_id.to_string(), presentation);
    }
}

impl WorldStore {
    /// Resolves a synchronized chicken/cow/pig variant element in current server holder order.
    /// Unresolved data-less stubs remain `None`; fallback stays consumer-owned by entry identity.
    pub fn synchronized_farm_animal_variant_presentation(
        &self,
        registry: &str,
        holder_id: i32,
    ) -> Option<&FarmAnimalVariantPresentation> {
        let entry = self.registry_entry_by_holder_id(registry, holder_id)?;
        self.registries
            .typed
            .farm_animal_variants
            .get(registry)?
            .get(&entry.id)
    }

    /// Resolves a synchronized cat variant element in current server holder order.
    /// Unresolved data-less stubs remain `None`; fallback stays consumer-owned by entry identity.
    pub fn synchronized_cat_variant_presentation(
        &self,
        holder_id: i32,
    ) -> Option<&CatVariantPresentation> {
        let entry = self.registry_entry_by_holder_id(CAT_VARIANT_REGISTRY, holder_id)?;
        self.registries.typed.cat_variants.get(&entry.id)
    }

    /// Resolves a synchronized frog variant element in current server holder order.
    /// Unresolved data-less stubs remain `None`; fallback stays consumer-owned by entry identity.
    pub fn synchronized_frog_variant_presentation(
        &self,
        holder_id: i32,
    ) -> Option<&FrogVariantPresentation> {
        let entry = self.registry_entry_by_holder_id(FROG_VARIANT_REGISTRY, holder_id)?;
        self.registries.typed.frog_variants.get(&entry.id)
    }

    /// Resolves a synchronized wolf variant in current server holder order.
    /// Unresolved data-less stubs remain `None`; fallback stays consumer-owned by entry identity.
    pub fn synchronized_wolf_variant_presentation(
        &self,
        holder_id: i32,
    ) -> Option<&WolfVariantPresentation> {
        let entry = self.registry_entry_by_holder_id(WOLF_VARIANT_REGISTRY, holder_id)?;
        self.registries.typed.wolf_variants.get(&entry.id)
    }

    /// Resolves a synchronized zombie-nautilus variant in current server holder order.
    /// Unresolved data-less stubs remain `None`; fallback stays consumer-owned by entry identity.
    pub fn synchronized_zombie_nautilus_variant_presentation(
        &self,
        holder_id: i32,
    ) -> Option<&ZombieNautilusVariantPresentation> {
        let entry =
            self.registry_entry_by_holder_id(ZOMBIE_NAUTILUS_VARIANT_REGISTRY, holder_id)?;
        self.registries
            .typed
            .zombie_nautilus_variants
            .get(&entry.id)
    }
}

fn adapt_chicken_variant(value: ChickenVariantElement) -> FarmAnimalVariantPresentation {
    let model = match value.model_and_texture.model {
        ChickenModel::Normal => FarmAnimalVariantModel::Normal,
        ChickenModel::Cold => FarmAnimalVariantModel::Cold,
    };
    adapt_farm_animal_variant(model, value.model_and_texture.asset, value.baby_asset)
}

fn adapt_cow_variant(value: CowVariantElement) -> FarmAnimalVariantPresentation {
    let model = match value.model_and_texture.model {
        CowModel::Normal => FarmAnimalVariantModel::Normal,
        CowModel::Cold => FarmAnimalVariantModel::Cold,
        CowModel::Warm => FarmAnimalVariantModel::Warm,
    };
    adapt_farm_animal_variant(model, value.model_and_texture.asset, value.baby_asset)
}

fn adapt_pig_variant(value: PigVariantElement) -> FarmAnimalVariantPresentation {
    let model = match value.model_and_texture.model {
        PigModel::Normal => FarmAnimalVariantModel::Normal,
        PigModel::Cold => FarmAnimalVariantModel::Cold,
    };
    adapt_farm_animal_variant(model, value.model_and_texture.asset, value.baby_asset)
}

fn adapt_farm_animal_variant(
    model: FarmAnimalVariantModel,
    adult_asset: ClientAssetResourceTexture,
    baby_asset: ClientAssetResourceTexture,
) -> FarmAnimalVariantPresentation {
    let (adult_asset_id, adult_texture_path) = resource_texture_presentation(adult_asset);
    let (baby_asset_id, baby_texture_path) = resource_texture_presentation(baby_asset);
    FarmAnimalVariantPresentation {
        model,
        adult_asset_id,
        adult_texture_path,
        baby_asset_id,
        baby_texture_path,
    }
}

fn adapt_cat_variant(value: CatVariantElement) -> CatVariantPresentation {
    let (adult_asset_id, adult_texture_path) = resource_texture_presentation(value.adult_asset);
    let (baby_asset_id, baby_texture_path) = resource_texture_presentation(value.baby_asset);
    CatVariantPresentation {
        adult_asset_id,
        adult_texture_path,
        baby_asset_id,
        baby_texture_path,
    }
}

fn adapt_frog_variant(value: FrogVariantElement) -> FrogVariantPresentation {
    let (asset_id, texture_path) = resource_texture_presentation(value.asset);
    FrogVariantPresentation {
        asset_id,
        texture_path,
    }
}

fn adapt_wolf_variant(value: WolfVariantElement) -> WolfVariantPresentation {
    WolfVariantPresentation {
        adult: adapt_wolf_assets(value.assets),
        baby: adapt_wolf_assets(value.baby_assets),
    }
}

fn adapt_wolf_assets(value: WolfAssetInfo) -> WolfVariantAssetPresentation {
    let (wild_asset_id, wild_texture_path) = resource_texture_presentation(value.wild);
    let (tame_asset_id, tame_texture_path) = resource_texture_presentation(value.tame);
    let (angry_asset_id, angry_texture_path) = resource_texture_presentation(value.angry);
    WolfVariantAssetPresentation {
        wild_asset_id,
        wild_texture_path,
        tame_asset_id,
        tame_texture_path,
        angry_asset_id,
        angry_texture_path,
    }
}

fn adapt_zombie_nautilus_variant(
    value: ZombieNautilusVariantElement,
) -> ZombieNautilusVariantPresentation {
    let model = match value.model_and_texture.model {
        ZombieNautilusModel::Normal => ZombieNautilusVariantModelType::Normal,
        ZombieNautilusModel::Warm => ZombieNautilusVariantModelType::Warm,
    };
    let (asset_id, texture_path) = resource_texture_presentation(value.model_and_texture.asset);
    ZombieNautilusVariantPresentation {
        model,
        asset_id,
        texture_path,
    }
}

fn resource_texture_presentation(asset: ClientAssetResourceTexture) -> (String, String) {
    let asset_id = asset.asset_id.into_string();
    let (namespace, path) = asset_id
        .split_once(':')
        .expect("typed registry identifiers always have a namespace");
    let texture_path = format!("{namespace}:textures/{path}.png");
    (asset_id, texture_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RegistryPacketEntry, RegistryValidationError};

    fn variant_world(
        registry: &str,
        entry: RegistryPacketEntry,
    ) -> Result<WorldStore, RegistryValidationError> {
        let mut store = WorldStore::new();
        store.record_registry_entries(registry, 0, vec![entry]);
        store.begin_registry_configuration();
        store.finish_registry_configuration()?;
        Ok(store)
    }

    fn configured_farm_animal_variant(
        registry: &str,
        raw_data: &[u8],
    ) -> Result<FarmAnimalVariantPresentation, RegistryValidationError> {
        let store = variant_world(
            registry,
            RegistryPacketEntry::with_raw_data("example:variant", raw_data.to_vec()),
        )?;
        Ok(store
            .synchronized_farm_animal_variant_presentation(registry, 0)
            .unwrap()
            .clone())
    }

    fn configured_cat_variant(
        raw_data: &[u8],
    ) -> Result<CatVariantPresentation, RegistryValidationError> {
        let store = variant_world(
            CAT_VARIANT_REGISTRY,
            RegistryPacketEntry::with_raw_data("example:variant", raw_data.to_vec()),
        )?;
        Ok(store
            .synchronized_cat_variant_presentation(0)
            .unwrap()
            .clone())
    }

    fn configured_frog_variant(
        raw_data: &[u8],
    ) -> Result<FrogVariantPresentation, RegistryValidationError> {
        let store = variant_world(
            FROG_VARIANT_REGISTRY,
            RegistryPacketEntry::with_raw_data("example:variant", raw_data.to_vec()),
        )?;
        Ok(store
            .synchronized_frog_variant_presentation(0)
            .unwrap()
            .clone())
    }

    fn configured_wolf_variant(
        raw_data: &[u8],
    ) -> Result<WolfVariantPresentation, RegistryValidationError> {
        let store = variant_world(
            WOLF_VARIANT_REGISTRY,
            RegistryPacketEntry::with_raw_data("example:variant", raw_data.to_vec()),
        )?;
        Ok(store
            .synchronized_wolf_variant_presentation(0)
            .unwrap()
            .clone())
    }

    fn configured_zombie_nautilus_variant(
        raw_data: &[u8],
    ) -> Result<ZombieNautilusVariantPresentation, RegistryValidationError> {
        let store = variant_world(
            ZOMBIE_NAUTILUS_VARIANT_REGISTRY,
            RegistryPacketEntry::with_raw_data("example:variant", raw_data.to_vec()),
        )?;
        Ok(store
            .synchronized_zombie_nautilus_variant_presentation(0)
            .unwrap()
            .clone())
    }

    #[test]
    fn decodes_all_farm_animal_network_codec_shapes() {
        assert_eq!(
            configured_farm_animal_variant(
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
            configured_farm_animal_variant(
                COW_VARIANT_REGISTRY,
                &variant_nbt(
                    Some("warm"),
                    Some("example:cattle/adult"),
                    Some("example:cattle/baby"),
                ),
            )
            .unwrap(),
            FarmAnimalVariantPresentation {
                model: FarmAnimalVariantModel::Warm,
                adult_asset_id: "example:cattle/adult".to_string(),
                adult_texture_path: "example:textures/cattle/adult.png".to_string(),
                baby_asset_id: "example:cattle/baby".to_string(),
                baby_texture_path: "example:textures/cattle/baby.png".to_string(),
            }
        );
        assert_eq!(
            configured_farm_animal_variant(
                PIG_VARIANT_REGISTRY,
                &variant_nbt(
                    Some("cold"),
                    Some("example:swine/adult"),
                    Some("example:swine/baby"),
                ),
            )
            .unwrap(),
            FarmAnimalVariantPresentation {
                model: FarmAnimalVariantModel::Cold,
                adult_asset_id: "example:swine/adult".to_string(),
                adult_texture_path: "example:textures/swine/adult.png".to_string(),
                baby_asset_id: "example:swine/baby".to_string(),
                baby_texture_path: "example:textures/swine/baby.png".to_string(),
            }
        );
    }

    #[test]
    fn known_pack_network_codec_skips_spawn_conditions() {
        let store = variant_world(
            COW_VARIANT_REGISTRY,
            RegistryPacketEntry::with_known_pack_data_for_test(
                "example:variant",
                br#"{
                "model": "warm",
                "asset_id": "example:cattle/adult",
                "baby_asset_id": "example:cattle/baby"
            }"#
                .to_vec(),
            ),
        )
        .unwrap();

        assert_eq!(
            store
                .synchronized_farm_animal_variant_presentation(COW_VARIANT_REGISTRY, 0)
                .unwrap()
                .clone(),
            FarmAnimalVariantPresentation {
                model: FarmAnimalVariantModel::Warm,
                adult_asset_id: "example:cattle/adult".to_string(),
                adult_texture_path: "example:textures/cattle/adult.png".to_string(),
                baby_asset_id: "example:cattle/baby".to_string(),
                baby_texture_path: "example:textures/cattle/baby.png".to_string(),
            }
        );
    }

    #[test]
    fn rejects_invalid_model_and_texture_fields() {
        assert!(configured_farm_animal_variant(
            CHICKEN_VARIANT_REGISTRY,
            &variant_nbt(Some("warm"), Some("example:adult"), Some("example:baby")),
        )
        .is_err());
        assert!(configured_farm_animal_variant(
            COW_VARIANT_REGISTRY,
            &variant_nbt(Some("normal"), Some("Example:adult"), Some("example:baby")),
        )
        .is_err());
        assert!(configured_farm_animal_variant(
            PIG_VARIANT_REGISTRY,
            &variant_nbt(Some("normal"), Some("example:adult"), None),
        )
        .is_err());
    }

    #[test]
    fn decodes_cat_network_codec_shape_and_rejects_invalid_assets() {
        assert_eq!(
            configured_cat_variant(&variant_nbt(
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
        assert!(configured_cat_variant(&variant_nbt(
            None,
            Some("example:entity/cat/lynx"),
            Some("Example:entity/cat/lynx_baby"),
        ))
        .is_err());
    }

    #[test]
    fn decodes_frog_network_codec_shape_and_rejects_invalid_assets() {
        assert_eq!(
            configured_frog_variant(&variant_nbt(None, Some("example:entity/frog/blue"), None,))
                .unwrap(),
            FrogVariantPresentation {
                asset_id: "example:entity/frog/blue".to_string(),
                texture_path: "example:textures/entity/frog/blue.png".to_string(),
            }
        );
        assert!(configured_frog_variant(&variant_nbt(
            None,
            Some("Example:entity/frog/blue"),
            None,
        ))
        .is_err());
    }

    #[test]
    fn decodes_wolf_network_codec_six_texture_shape() {
        let presentation = configured_wolf_variant(&wolf_variant_nbt(
            [
                "example:entity/wolf/adult_wild",
                "example:entity/wolf/adult_tame",
                "example:entity/wolf/adult_angry",
            ],
            [
                "example:entity/wolf/baby_wild",
                "example:entity/wolf/baby_tame",
                "entity/wolf/baby_angry",
            ],
        ))
        .unwrap();
        assert_eq!(
            presentation.adult,
            WolfVariantAssetPresentation {
                wild_asset_id: "example:entity/wolf/adult_wild".to_string(),
                wild_texture_path: "example:textures/entity/wolf/adult_wild.png".to_string(),
                tame_asset_id: "example:entity/wolf/adult_tame".to_string(),
                tame_texture_path: "example:textures/entity/wolf/adult_tame.png".to_string(),
                angry_asset_id: "example:entity/wolf/adult_angry".to_string(),
                angry_texture_path: "example:textures/entity/wolf/adult_angry.png".to_string(),
            }
        );
        assert_eq!(
            presentation.baby.angry_texture_path,
            "minecraft:textures/entity/wolf/baby_angry.png"
        );

        assert!(configured_wolf_variant(&wolf_variant_nbt(
            [
                "example:entity/wolf/adult_wild",
                "Example:entity/wolf/adult_tame",
                "example:entity/wolf/adult_angry",
            ],
            [
                "example:entity/wolf/baby_wild",
                "example:entity/wolf/baby_tame",
                "example:entity/wolf/baby_angry",
            ],
        ))
        .is_err());

        assert!(configured_wolf_variant(&[10, 0]).is_err());
    }

    #[test]
    fn decodes_zombie_nautilus_network_codec_model_and_texture() {
        assert_eq!(
            configured_zombie_nautilus_variant(&variant_nbt(
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
            configured_zombie_nautilus_variant(&variant_nbt(
                None,
                Some("example:entity/nautilus/plain"),
                None,
            ))
            .unwrap()
            .model,
            ZombieNautilusVariantModelType::Normal
        );
        assert!(configured_zombie_nautilus_variant(&variant_nbt(
            Some("cold"),
            Some("example:entity/nautilus/plain"),
            None,
        ))
        .is_err());
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
        store.begin_registry_configuration();
        store.finish_registry_configuration().unwrap();

        assert_eq!(
            store
                .synchronized_farm_animal_variant_presentation(COW_VARIANT_REGISTRY, 0)
                .cloned(),
            Some(FarmAnimalVariantPresentation {
                model: FarmAnimalVariantModel::Warm,
                adult_asset_id: "example:cow/adult".to_string(),
                adult_texture_path: "example:textures/cow/adult.png".to_string(),
                baby_asset_id: "example:cow/baby".to_string(),
                baby_texture_path: "example:textures/cow/baby.png".to_string(),
            })
        );
        assert_eq!(
            store.synchronized_farm_animal_variant_presentation(COW_VARIANT_REGISTRY, 1),
            None
        );
        assert_eq!(
            store.synchronized_farm_animal_variant_presentation(COW_VARIANT_REGISTRY, 2),
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
        store.begin_registry_configuration();
        store.finish_registry_configuration().unwrap();

        assert_eq!(
            store.synchronized_cat_variant_presentation(0).cloned(),
            Some(CatVariantPresentation {
                adult_asset_id: "example:entity/cat/lynx".to_string(),
                adult_texture_path: "example:textures/entity/cat/lynx.png".to_string(),
                baby_asset_id: "example:entity/cat/lynx_baby".to_string(),
                baby_texture_path: "example:textures/entity/cat/lynx_baby.png".to_string(),
            })
        );
        assert_eq!(store.synchronized_cat_variant_presentation(1), None);
        assert_eq!(store.synchronized_cat_variant_presentation(2), None);
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
        store.begin_registry_configuration();
        store.finish_registry_configuration().unwrap();

        assert_eq!(
            store.synchronized_frog_variant_presentation(0).cloned(),
            Some(FrogVariantPresentation {
                asset_id: "example:entity/frog/blue".to_string(),
                texture_path: "example:textures/entity/frog/blue.png".to_string(),
            })
        );
        assert_eq!(store.synchronized_frog_variant_presentation(1), None);
        assert_eq!(store.synchronized_frog_variant_presentation(2), None);
    }

    #[test]
    fn wolf_world_query_uses_server_holder_order_and_omits_stubs() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            WOLF_VARIANT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:dire",
                    wolf_variant_nbt(
                        [
                            "example:adult/wild",
                            "example:adult/tame",
                            "example:adult/angry",
                        ],
                        [
                            "example:baby/wild",
                            "example:baby/tame",
                            "example:baby/angry",
                        ],
                    ),
                ),
                RegistryPacketEntry::stub("minecraft:pale"),
            ],
        );
        store.begin_registry_configuration();
        store.finish_registry_configuration().unwrap();

        let presentation = store.synchronized_wolf_variant_presentation(0).unwrap();
        assert_eq!(presentation.adult.wild_asset_id, "example:adult/wild");
        assert_eq!(presentation.adult.tame_asset_id, "example:adult/tame");
        assert_eq!(presentation.adult.angry_asset_id, "example:adult/angry");
        assert_eq!(presentation.baby.wild_asset_id, "example:baby/wild");
        assert_eq!(presentation.baby.tame_asset_id, "example:baby/tame");
        assert_eq!(presentation.baby.angry_asset_id, "example:baby/angry");
        assert_eq!(store.synchronized_wolf_variant_presentation(1), None);
        assert_eq!(store.synchronized_wolf_variant_presentation(2), None);
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
        store.begin_registry_configuration();
        store.finish_registry_configuration().unwrap();

        assert_eq!(
            store
                .synchronized_zombie_nautilus_variant_presentation(0)
                .cloned(),
            Some(ZombieNautilusVariantPresentation {
                model: ZombieNautilusVariantModelType::Warm,
                asset_id: "example:entity/nautilus/coral".to_string(),
                texture_path: "example:textures/entity/nautilus/coral.png".to_string(),
            })
        );
        assert_eq!(
            store.synchronized_zombie_nautilus_variant_presentation(1),
            None
        );
        assert_eq!(
            store.synchronized_zombie_nautilus_variant_presentation(2),
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

    fn wolf_variant_nbt(adult: [&str; 3], baby: [&str; 3]) -> Vec<u8> {
        let mut out = vec![10];
        write_wolf_asset_compound(&mut out, "assets", adult);
        write_wolf_asset_compound(&mut out, "baby_assets", baby);
        out.push(0);
        out
    }

    fn write_wolf_asset_compound(out: &mut Vec<u8>, name: &str, assets: [&str; 3]) {
        out.push(10);
        write_utf(out, name);
        write_string(out, "wild", assets[0]);
        write_string(out, "tame", assets[1]);
        write_string(out, "angry", assets[2]);
        out.push(0);
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
