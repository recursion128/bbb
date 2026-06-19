use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    resources::{PackResourceStack, ResourceLocation},
    roots::PackRoots,
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipmentAssetCatalog {
    assets: BTreeMap<String, EquipmentAsset>,
}

impl EquipmentAssetCatalog {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        Self::load_resource_stack(&roots.resource_stack())
    }

    pub fn load_resource_stack(stack: &PackResourceStack) -> Result<Self> {
        let mut assets = BTreeMap::new();
        for resource in stack.list_resources("equipment", ".json")? {
            let asset_id = equipment_asset_id_from_resource(&resource.location)?;
            let bytes = std::fs::read(&resource.path)
                .with_context(|| format!("read equipment asset {}", resource.path.display()))?;
            let asset = EquipmentAsset::from_json_bytes(&bytes)
                .with_context(|| format!("parse equipment asset {}", resource.path.display()))?;
            assets.insert(asset_id, asset);
        }
        Ok(Self { assets })
    }

    pub fn asset(&self, asset_id: &str) -> Option<&EquipmentAsset> {
        let asset_id = ResourceLocation::parse(asset_id).ok()?.id();
        self.assets.get(&asset_id)
    }

    pub fn assets(&self) -> &BTreeMap<String, EquipmentAsset> {
        &self.assets
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipmentAsset {
    pub layers: BTreeMap<EquipmentLayerType, Vec<EquipmentLayer>>,
}

impl EquipmentAsset {
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self> {
        let raw: RawEquipmentAsset = serde_json::from_slice(bytes)?;
        raw.into_asset()
    }

    pub fn layers(&self, layer_type: EquipmentLayerType) -> &[EquipmentLayer] {
        self.layers.get(&layer_type).map_or(&[], Vec::as_slice)
    }

    pub fn layer_type_count(&self) -> usize {
        self.layers.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentLayerType {
    Humanoid,
    HumanoidLeggings,
    HumanoidBaby,
    Wings,
    WolfBody,
    HorseBody,
    LlamaBody,
    PigSaddle,
    StriderSaddle,
    CamelSaddle,
    CamelHuskSaddle,
    HorseSaddle,
    DonkeySaddle,
    MuleSaddle,
    ZombieHorseSaddle,
    SkeletonHorseSaddle,
    HappyGhastBody,
    NautilusSaddle,
    NautilusBody,
}

impl EquipmentLayerType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Humanoid => "humanoid",
            Self::HumanoidLeggings => "humanoid_leggings",
            Self::HumanoidBaby => "humanoid_baby",
            Self::Wings => "wings",
            Self::WolfBody => "wolf_body",
            Self::HorseBody => "horse_body",
            Self::LlamaBody => "llama_body",
            Self::PigSaddle => "pig_saddle",
            Self::StriderSaddle => "strider_saddle",
            Self::CamelSaddle => "camel_saddle",
            Self::CamelHuskSaddle => "camel_husk_saddle",
            Self::HorseSaddle => "horse_saddle",
            Self::DonkeySaddle => "donkey_saddle",
            Self::MuleSaddle => "mule_saddle",
            Self::ZombieHorseSaddle => "zombie_horse_saddle",
            Self::SkeletonHorseSaddle => "skeleton_horse_saddle",
            Self::HappyGhastBody => "happy_ghast_body",
            Self::NautilusSaddle => "nautilus_saddle",
            Self::NautilusBody => "nautilus_body",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "humanoid" => Self::Humanoid,
            "humanoid_leggings" => Self::HumanoidLeggings,
            "humanoid_baby" => Self::HumanoidBaby,
            "wings" => Self::Wings,
            "wolf_body" => Self::WolfBody,
            "horse_body" => Self::HorseBody,
            "llama_body" => Self::LlamaBody,
            "pig_saddle" => Self::PigSaddle,
            "strider_saddle" => Self::StriderSaddle,
            "camel_saddle" => Self::CamelSaddle,
            "camel_husk_saddle" => Self::CamelHuskSaddle,
            "horse_saddle" => Self::HorseSaddle,
            "donkey_saddle" => Self::DonkeySaddle,
            "mule_saddle" => Self::MuleSaddle,
            "zombie_horse_saddle" => Self::ZombieHorseSaddle,
            "skeleton_horse_saddle" => Self::SkeletonHorseSaddle,
            "happy_ghast_body" => Self::HappyGhastBody,
            "nautilus_saddle" => Self::NautilusSaddle,
            "nautilus_body" => Self::NautilusBody,
            other => bail!("unsupported equipment layer type {other:?}"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipmentLayer {
    pub texture: String,
    pub texture_location: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dyeable: Option<EquipmentDyeable>,
    pub use_player_texture: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipmentDyeable {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_when_undyed: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct RawEquipmentAsset {
    layers: BTreeMap<String, Vec<RawEquipmentLayer>>,
}

impl RawEquipmentAsset {
    fn into_asset(self) -> Result<EquipmentAsset> {
        if self.layers.is_empty() {
            bail!("equipment asset layers must not be empty");
        }

        let mut layers = BTreeMap::new();
        for (layer_type, raw_layers) in self.layers {
            if raw_layers.is_empty() {
                bail!("equipment asset layer list for {layer_type:?} must not be empty");
            }
            let layer_type = EquipmentLayerType::parse(&layer_type)?;
            let converted_layers = raw_layers
                .into_iter()
                .map(|layer| layer.into_layer(layer_type))
                .collect::<Result<Vec<_>>>()?;
            layers.insert(layer_type, converted_layers);
        }
        Ok(EquipmentAsset { layers })
    }
}

#[derive(Debug, Deserialize)]
struct RawEquipmentLayer {
    texture: String,
    #[serde(default)]
    dyeable: Option<EquipmentDyeable>,
    #[serde(default)]
    use_player_texture: bool,
}

impl RawEquipmentLayer {
    fn into_layer(self, layer_type: EquipmentLayerType) -> Result<EquipmentLayer> {
        let texture = ResourceLocation::parse(&self.texture)?;
        let texture_location = equipment_texture_location(layer_type, &texture)?;
        Ok(EquipmentLayer {
            texture: texture.id(),
            texture_location,
            dyeable: self.dyeable,
            use_player_texture: self.use_player_texture,
        })
    }
}

fn equipment_texture_location(
    layer_type: EquipmentLayerType,
    texture: &ResourceLocation,
) -> Result<String> {
    ResourceLocation::new(
        texture.namespace().to_string(),
        format!(
            "textures/entity/equipment/{}/{}.png",
            layer_type.as_str(),
            texture.path()
        ),
    )
    .map(|location| location.id())
}

fn equipment_asset_id_from_resource(location: &ResourceLocation) -> Result<String> {
    let path = location
        .path()
        .strip_prefix("equipment/")
        .and_then(|path| path.strip_suffix(".json"))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "equipment asset resource {} is outside equipment",
                location.id()
            )
        })?;
    ResourceLocation::new(location.namespace().to_string(), path.to_string()).map(|id| id.id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn equipment_asset_catalog_parses_dyeable_layers() {
        let root = unique_temp_dir("equipment-dyeable");
        write_json(
            &equipment_dir(&root).join("leather.json"),
            r#"{
              "layers": {
                "humanoid": [
                  {
                    "texture": "minecraft:leather",
                    "dyeable": {
                      "color_when_undyed": -6265536
                    }
                  },
                  {
                    "texture": "minecraft:leather_overlay"
                  }
                ],
                "humanoid_leggings": [
                  {
                    "texture": "minecraft:leather",
                    "dyeable": {}
                  }
                ]
              }
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_equipment_asset_catalog()
            .unwrap();
        let leather = catalog.asset("leather").unwrap();

        assert_eq!(catalog.len(), 1);
        assert_eq!(leather.layer_type_count(), 2);
        assert_eq!(
            leather.layers(EquipmentLayerType::Humanoid),
            &[
                EquipmentLayer {
                    texture: "minecraft:leather".to_string(),
                    texture_location: "minecraft:textures/entity/equipment/humanoid/leather.png"
                        .to_string(),
                    dyeable: Some(EquipmentDyeable {
                        color_when_undyed: Some(-6265536),
                    }),
                    use_player_texture: false,
                },
                EquipmentLayer {
                    texture: "minecraft:leather_overlay".to_string(),
                    texture_location:
                        "minecraft:textures/entity/equipment/humanoid/leather_overlay.png"
                            .to_string(),
                    dyeable: None,
                    use_player_texture: false,
                },
            ]
        );
        assert_eq!(
            leather.layers(EquipmentLayerType::HumanoidLeggings)[0].dyeable,
            Some(EquipmentDyeable {
                color_when_undyed: None,
            })
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn equipment_asset_catalog_parses_use_player_texture_layer() {
        let root = unique_temp_dir("equipment-player-texture");
        write_json(
            &equipment_dir(&root).join("elytra.json"),
            r#"{
              "layers": {
                "wings": [
                  {
                    "texture": "minecraft:elytra",
                    "use_player_texture": true
                  }
                ]
              }
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_equipment_asset_catalog()
            .unwrap();
        let layer = &catalog
            .asset("elytra")
            .unwrap()
            .layers(EquipmentLayerType::Wings)[0];

        assert_eq!(layer.texture, "minecraft:elytra");
        assert_eq!(
            layer.texture_location,
            "minecraft:textures/entity/equipment/wings/elytra.png"
        );
        assert!(layer.use_player_texture);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn equipment_asset_catalog_uses_resource_pack_precedence() {
        let root = unique_temp_dir("equipment-precedence");
        let pack = root.join("resource-pack");
        write_json(
            &equipment_dir(&root).join("iron.json"),
            r#"{
              "layers": {
                "humanoid": [
                  {
                    "texture": "minecraft:iron"
                  }
                ]
              }
            }"#,
        );
        write_json(
            &pack
                .join("assets")
                .join("minecraft")
                .join("equipment")
                .join("iron.json"),
            r#"{
              "layers": {
                "humanoid": [
                  {
                    "texture": "custom:iron_override"
                  }
                ],
                "humanoid_baby": [
                  {
                    "texture": "custom:iron_baby"
                  }
                ]
              }
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([pack])
            .load_equipment_asset_catalog()
            .unwrap();
        let iron = catalog.asset("minecraft:iron").unwrap();

        assert_eq!(iron.layer_type_count(), 2);
        assert_eq!(
            iron.layers(EquipmentLayerType::Humanoid)[0].texture,
            "custom:iron_override"
        );
        assert_eq!(
            iron.layers(EquipmentLayerType::HumanoidBaby)[0].texture_location,
            "custom:textures/entity/equipment/humanoid_baby/iron_baby.png"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn equipment_asset_rejects_empty_or_unknown_layer_shapes() {
        let err = EquipmentAsset::from_json_bytes(br#"{ "layers": {} }"#).unwrap_err();
        assert!(err
            .to_string()
            .contains("equipment asset layers must not be empty"));

        let err = EquipmentAsset::from_json_bytes(
            br#"{
              "layers": {
                "humanoid": []
              }
            }"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("must not be empty"));

        let err = EquipmentAsset::from_json_bytes(
            br#"{
              "layers": {
                "unknown": [
                  {
                    "texture": "minecraft:test"
                  }
                ]
              }
            }"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("unsupported equipment layer type"));
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_equipment_asset_catalog() {
        let catalog = PackRoots::discover()
            .unwrap()
            .load_equipment_asset_catalog()
            .unwrap();
        assert_eq!(catalog.len(), 45);

        let leather = catalog.asset("minecraft:leather").unwrap();
        assert_eq!(leather.layer_type_count(), 4);
        assert_eq!(
            leather.layers(EquipmentLayerType::Humanoid)[0].dyeable,
            Some(EquipmentDyeable {
                color_when_undyed: Some(-6265536),
            })
        );

        let elytra = catalog.asset("minecraft:elytra").unwrap();
        assert!(elytra.layers(EquipmentLayerType::Wings)[0].use_player_texture);

        let armadillo = catalog.asset("minecraft:armadillo_scute").unwrap();
        assert_eq!(
            armadillo.layers(EquipmentLayerType::WolfBody)[1].dyeable,
            Some(EquipmentDyeable {
                color_when_undyed: None,
            })
        );
    }

    fn equipment_dir(root: &Path) -> PathBuf {
        root.join("sources")
            .join(crate::MC_VERSION)
            .join("assets")
            .join("minecraft")
            .join("equipment")
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-pack-{label}-{nanos}"))
    }
}
