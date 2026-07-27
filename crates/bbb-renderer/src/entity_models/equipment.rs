use std::collections::BTreeMap;

use anyhow::{bail, Result};

use super::{EntityModelOwnedTextureAtlasEntry, EntityModelOwnedTextureRef, EntityModelTextureRef};
use bbb_render_types::EntityEquipmentAssetId;

/// Vanilla `EquipmentClientInfo.LayerType`, kept renderer-owned so `bbb-renderer`
/// does not depend on the pack parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntityEquipmentLayerType {
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

impl EntityEquipmentLayerType {
    /// Logical UV extent of the existing model baked for this layer type.
    pub const fn texture_size(self) -> [u32; 2] {
        match self {
            Self::Humanoid | Self::HumanoidLeggings | Self::Wings | Self::WolfBody => [64, 32],
            Self::HumanoidBaby
            | Self::HorseBody
            | Self::HorseSaddle
            | Self::DonkeySaddle
            | Self::MuleSaddle
            | Self::ZombieHorseSaddle
            | Self::SkeletonHorseSaddle
            | Self::HappyGhastBody
            | Self::PigSaddle => [64, 64],
            Self::LlamaBody => [128, 64],
            Self::StriderSaddle => [64, 128],
            Self::CamelSaddle
            | Self::CamelHuskSaddle
            | Self::NautilusSaddle
            | Self::NautilusBody => [128, 128],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityEquipmentDyeable {
    pub color_when_undyed: Option<i32>,
}

/// Texture identity before the renderer atlas resolves an owned path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityEquipmentAssetLayerTexture {
    Static(EntityModelTextureRef),
    Owned(EntityModelOwnedTextureRef),
}

impl EntityEquipmentAssetLayerTexture {
    pub fn logical_size(&self) -> [u32; 2] {
        match self {
            Self::Static(texture) => texture.size,
            Self::Owned(texture) => texture.size,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityEquipmentAssetLayer {
    pub texture: EntityEquipmentAssetLayerTexture,
    pub dyeable: Option<EntityEquipmentDyeable>,
    pub use_player_texture: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntityEquipmentAsset {
    pub layers: BTreeMap<EntityEquipmentLayerType, Vec<EntityEquipmentAssetLayer>>,
}

impl EntityEquipmentAsset {
    pub fn layers(&self, layer_type: EntityEquipmentLayerType) -> &[EntityEquipmentAssetLayer] {
        self.layers.get(&layer_type).map_or(&[], Vec::as_slice)
    }
}

/// Renderer-side CPU catalog in the same sorted-id order as
/// `bbb_pack::EquipmentAssetCatalog`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntityEquipmentAssetCatalog {
    assets: Vec<EntityEquipmentAsset>,
}

impl EntityEquipmentAssetCatalog {
    pub fn new(assets: Vec<EntityEquipmentAsset>) -> Self {
        Self { assets }
    }

    pub fn asset(&self, id: EntityEquipmentAssetId) -> Option<&EntityEquipmentAsset> {
        self.assets.get(id.0 as usize)
    }

    pub fn assets(&self) -> &[EntityEquipmentAsset] {
        &self.assets
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    pub(crate) fn resolve(
        &self,
        owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    ) -> Result<ResolvedEntityEquipmentAssetCatalog> {
        let mut assets = Vec::with_capacity(self.assets.len());
        for asset in &self.assets {
            let mut layers = BTreeMap::new();
            for (layer_type, source_layers) in &asset.layers {
                let resolved_layers = source_layers
                    .iter()
                    .map(|layer| {
                        let texture = match &layer.texture {
                            EntityEquipmentAssetLayerTexture::Static(texture) => {
                                ResolvedEntityEquipmentAssetLayerTexture::Static(*texture)
                            }
                            EntityEquipmentAssetLayerTexture::Owned(texture) => {
                                let index = owned_entries
                                    .iter()
                                    .position(|entry| entry.texture.path == texture.path)
                                    .and_then(|index| u32::try_from(index).ok())
                                    .ok_or_else(|| {
                                        anyhow::anyhow!(
                                            "equipment texture {} is missing from owned entity atlas",
                                            texture.path
                                        )
                                    })?;
                                ResolvedEntityEquipmentAssetLayerTexture::Owned {
                                    index,
                                    logical_size: texture.size,
                                }
                            }
                        };
                        Ok(ResolvedEntityEquipmentAssetLayer {
                            texture,
                            dyeable: layer.dyeable,
                            use_player_texture: layer.use_player_texture,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                if resolved_layers.is_empty() {
                    bail!("renderer equipment asset layer list must not be empty");
                }
                layers.insert(*layer_type, resolved_layers);
            }
            if layers.is_empty() {
                bail!("renderer equipment asset must contain at least one layer type");
            }
            assets.push(ResolvedEntityEquipmentAsset { layers });
        }
        Ok(ResolvedEntityEquipmentAssetCatalog { assets })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedEntityEquipmentAssetLayerTexture {
    Static(EntityModelTextureRef),
    Owned { index: u32, logical_size: [u32; 2] },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResolvedEntityEquipmentAssetLayer {
    pub(crate) texture: ResolvedEntityEquipmentAssetLayerTexture,
    pub(crate) dyeable: Option<EntityEquipmentDyeable>,
    pub(crate) use_player_texture: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ResolvedEntityEquipmentAsset {
    layers: BTreeMap<EntityEquipmentLayerType, Vec<ResolvedEntityEquipmentAssetLayer>>,
}

// Stage 1 resolves these values atomically with the atlas. The following
// renderer migration calls the lookups from geometry submission.
#[allow(dead_code)]
impl ResolvedEntityEquipmentAsset {
    pub(crate) fn layers(
        &self,
        layer_type: EntityEquipmentLayerType,
    ) -> &[ResolvedEntityEquipmentAssetLayer] {
        self.layers.get(&layer_type).map_or(&[], Vec::as_slice)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ResolvedEntityEquipmentAssetCatalog {
    assets: Vec<ResolvedEntityEquipmentAsset>,
}

#[allow(dead_code)]
impl ResolvedEntityEquipmentAssetCatalog {
    pub(crate) fn asset(
        &self,
        id: EntityEquipmentAssetId,
    ) -> Option<&ResolvedEntityEquipmentAsset> {
        self.assets.get(id.0 as usize)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::entity_models::{EntityModelOwnedTextureAtlasEntry, EntityModelUvRect};

    #[test]
    fn equipment_catalog_resolves_owned_paths_without_changing_asset_ids() {
        let texture =
            EntityModelOwnedTextureRef::new(Arc::<str>::from("example:equipment.png"), [64, 32]);
        let catalog = EntityEquipmentAssetCatalog::new(vec![EntityEquipmentAsset {
            layers: BTreeMap::from([(
                EntityEquipmentLayerType::Humanoid,
                vec![EntityEquipmentAssetLayer {
                    texture: EntityEquipmentAssetLayerTexture::Owned(texture.clone()),
                    dyeable: Some(EntityEquipmentDyeable {
                        color_when_undyed: None,
                    }),
                    use_player_texture: false,
                }],
            )]),
        }]);
        let resolved = catalog
            .resolve(&[EntityModelOwnedTextureAtlasEntry {
                texture,
                uv: EntityModelUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
            }])
            .unwrap();
        let layer = resolved
            .asset(EntityEquipmentAssetId(0))
            .unwrap()
            .layers(EntityEquipmentLayerType::Humanoid)[0];

        assert_eq!(
            layer.texture,
            ResolvedEntityEquipmentAssetLayerTexture::Owned {
                index: 0,
                logical_size: [64, 32],
            }
        );
        assert_eq!(
            layer.dyeable,
            Some(EntityEquipmentDyeable {
                color_when_undyed: None
            })
        );
    }
}
