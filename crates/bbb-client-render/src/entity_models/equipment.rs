use std::collections::BTreeMap;

use anyhow::{bail, Result};
use hashbrown::HashMap;

use super::{EntityModelOwnedTextureAtlasEntry, EntityModelOwnedTextureRef, EntityModelTextureRef};
use bbb_render_types::EntityEquipmentAssetId;

const MAX_EQUIPMENT_LAYERS_PER_TYPE: usize = 64;
const MAX_TOTAL_EQUIPMENT_LAYERS: usize = 16_384;

/// Vanilla `EquipmentClientInfo.LayerType`.
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

/// Renderer-side CPU catalog in the same sorted-id order as the pack-owned equipment asset catalog.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntityEquipmentAssetCatalog {
    generation: u64,
    assets: Vec<EntityEquipmentAsset>,
}

impl EntityEquipmentAssetCatalog {
    pub fn new(generation: u64, assets: Vec<EntityEquipmentAsset>) -> Self {
        Self { generation, assets }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn asset(&self, id: EntityEquipmentAssetId) -> Option<&EntityEquipmentAsset> {
        (id.generation == self.generation)
            .then_some(id.index)
            .and_then(|index| self.assets.get(index as usize))
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
        let mut owned_index = HashMap::with_capacity(owned_entries.len());
        for (index, entry) in owned_entries.iter().enumerate() {
            if let Ok(index) = u32::try_from(index) {
                owned_index
                    .entry(entry.texture.path.as_ref())
                    .or_insert(index);
            }
        }
        let mut total_layers = 0usize;
        let mut assets = Vec::with_capacity(self.assets.len());
        for asset in &self.assets {
            let mut layers = BTreeMap::new();
            for (layer_type, source_layers) in &asset.layers {
                if source_layers.len() > MAX_EQUIPMENT_LAYERS_PER_TYPE {
                    bail!(
                        "renderer equipment asset layer list length {} exceeds limit {MAX_EQUIPMENT_LAYERS_PER_TYPE}",
                        source_layers.len()
                    );
                }
                total_layers = total_layers.saturating_add(source_layers.len());
                if total_layers > MAX_TOTAL_EQUIPMENT_LAYERS {
                    bail!(
                        "renderer equipment asset catalog total layer count {total_layers} exceeds limit {MAX_TOTAL_EQUIPMENT_LAYERS}"
                    );
                }
                let resolved_layers = source_layers
                    .iter()
                    .map(|layer| {
                        let texture = match &layer.texture {
                            EntityEquipmentAssetLayerTexture::Static(texture) => {
                                ResolvedEntityEquipmentAssetLayerTexture::Static(*texture)
                            }
                            EntityEquipmentAssetLayerTexture::Owned(texture) => {
                                let index = owned_index
                                    .get(texture.path.as_ref())
                                    .copied()
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
        Ok(ResolvedEntityEquipmentAssetCatalog {
            generation: self.generation,
            assets,
        })
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
    generation: u64,
    assets: Vec<ResolvedEntityEquipmentAsset>,
}

impl ResolvedEntityEquipmentAssetCatalog {
    pub(crate) fn asset(
        &self,
        id: EntityEquipmentAssetId,
    ) -> Option<&ResolvedEntityEquipmentAsset> {
        (id.generation == self.generation)
            .then_some(id.index)
            .and_then(|index| self.assets.get(index as usize))
    }

    /// Vanilla `EquipmentAssetManager.get(asset).getLayers(type)`: layer
    /// presence is a generation-local renderer query, not a frame-projected
    /// boolean that can drift from this catalog.
    pub(crate) fn has_layer(
        &self,
        id: Option<EntityEquipmentAssetId>,
        layer_type: EntityEquipmentLayerType,
    ) -> bool {
        id.and_then(|id| self.asset(id))
            .is_some_and(|asset| !asset.layers(layer_type).is_empty())
    }
}

/// Vanilla `EquipmentLayerRenderer.getColorForLayer`: non-dyeable layers are
/// white; dyeable layers use the stack dye, then their opaque undyed default,
/// and are skipped when neither exists.
pub(crate) fn equipment_layer_tint(
    dyeable: Option<EntityEquipmentDyeable>,
    dye: Option<u32>,
) -> Option<[f32; 4]> {
    match dyeable {
        None => Some([1.0; 4]),
        Some(dyeable) => dye
            .or_else(|| dyeable.color_when_undyed.map(|color| color as u32))
            .map(opaque_rgb_to_tint),
    }
}

fn opaque_rgb_to_tint(rgb: u32) -> [f32; 4] {
    [
        ((rgb >> 16) & 0xFF) as f32 / 255.0,
        ((rgb >> 8) & 0xFF) as f32 / 255.0,
        (rgb & 0xFF) as f32 / 255.0,
        1.0,
    ]
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
        let catalog = EntityEquipmentAssetCatalog::new(
            7,
            vec![EntityEquipmentAsset {
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
            }],
        );
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
            .asset(EntityEquipmentAssetId::new(7, 0))
            .unwrap()
            .layers(EntityEquipmentLayerType::Humanoid)[0];

        assert_eq!(
            layer.texture,
            ResolvedEntityEquipmentAssetLayerTexture::Owned {
                index: 0,
                logical_size: [64, 32],
            }
        );
        assert!(
            resolved.asset(EntityEquipmentAssetId::new(6, 0)).is_none(),
            "an old world generation must not address a reloaded atlas catalog"
        );
        assert!(resolved.has_layer(
            Some(EntityEquipmentAssetId::new(7, 0)),
            EntityEquipmentLayerType::Humanoid
        ));
        assert!(!resolved.has_layer(
            Some(EntityEquipmentAssetId::new(6, 0)),
            EntityEquipmentLayerType::Humanoid
        ));
        assert_eq!(
            layer.dyeable,
            Some(EntityEquipmentDyeable {
                color_when_undyed: None
            })
        );
    }

    #[test]
    fn equipment_layer_tint_matches_vanilla_dye_skip_and_opaque_rules() {
        assert_eq!(equipment_layer_tint(None, Some(0x12_34_56)), Some([1.0; 4]));
        assert_eq!(
            equipment_layer_tint(
                Some(EntityEquipmentDyeable {
                    color_when_undyed: Some(0x80_11_22_33_u32 as i32),
                }),
                None,
            ),
            Some([
                0x11 as f32 / 255.0,
                0x22 as f32 / 255.0,
                0x33 as f32 / 255.0,
                1.0,
            ])
        );
        assert_eq!(
            equipment_layer_tint(
                Some(EntityEquipmentDyeable {
                    color_when_undyed: Some(0x11_22_33),
                }),
                Some(0x44_55_66),
            ),
            Some([
                0x44 as f32 / 255.0,
                0x55 as f32 / 255.0,
                0x66 as f32 / 255.0,
                1.0,
            ])
        );
        assert_eq!(
            equipment_layer_tint(
                Some(EntityEquipmentDyeable {
                    color_when_undyed: None,
                }),
                None,
            ),
            None
        );
    }

    #[test]
    fn equipment_catalog_resolve_uses_indexed_lookup_and_enforces_layer_caps() {
        // R20-02: resolving many owned equipment assets must remain O(L + M) and enforce caps.
        let owned_entries: Vec<EntityModelOwnedTextureAtlasEntry> = (0..1_024)
            .map(|i| EntityModelOwnedTextureAtlasEntry {
                texture: EntityModelOwnedTextureRef::new(
                    format!("textures/entity/equipment/humanoid/mod_{i}.png"),
                    [64, 32],
                ),
                uv: crate::entity_models::EntityModelUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
            })
            .collect();
        let assets: Vec<EntityEquipmentAsset> = (0..1_024)
            .map(|i| {
                let mut layers = BTreeMap::new();
                layers.insert(
                    EntityEquipmentLayerType::Humanoid,
                    vec![EntityEquipmentAssetLayer {
                        texture: EntityEquipmentAssetLayerTexture::Owned(
                            EntityModelOwnedTextureRef::new(
                                format!("textures/entity/equipment/humanoid/mod_{i}.png"),
                                [64, 32],
                            ),
                        ),
                        dyeable: None,
                        use_player_texture: false,
                    }],
                );
                EntityEquipmentAsset { layers }
            })
            .collect();
        let catalog = EntityEquipmentAssetCatalog::new(1, assets);
        let resolved = catalog.resolve(&owned_entries).expect("indexed resolve");
        assert_eq!(resolved.assets.len(), 1_024);

        let too_many_layers = EntityEquipmentAssetCatalog::new(
            2,
            vec![EntityEquipmentAsset {
                layers: BTreeMap::from([(
                    EntityEquipmentLayerType::Humanoid,
                    (0..=MAX_EQUIPMENT_LAYERS_PER_TYPE)
                        .map(|_| EntityEquipmentAssetLayer {
                            texture: EntityEquipmentAssetLayerTexture::Owned(
                                EntityModelOwnedTextureRef::new(
                                    "textures/entity/equipment/humanoid/mod_0.png",
                                    [64, 32],
                                ),
                            ),
                            dyeable: None,
                            use_player_texture: false,
                        })
                        .collect(),
                )]),
            }],
        );
        assert!(too_many_layers.resolve(&owned_entries).is_err());
    }
}
