//! Resident entity-model meshes indexed by [`EntityMeshLane`] plus sorted draw ranges,
//! updated from retained instance submissions independently of camera sorting.

use std::ops::{Index, IndexMut};

use super::*;

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntityMeshLane {
    WaterMask = 0,
    Solid,
    Textured,
    TexturedCull,
    Dissolve,
    SolidZOffsetForward,
    CutoutZOffset,
    ArmorCutout,
    ArmorTrim,
    ArmorTrimDecal,
    Eyes,
    Outline,
    OutlineCull,
    Translucent,
    ArmorTranslucent,
    TranslucentEmissive,
    ItemEntityTranslucent,
    ItemEntityTranslucentCull,
    SkinCutout,
    SkinSolid,
    SkinCutoutCull,
    SkinSolidZOffsetForward,
    SkinCutoutZOffset,
    SkinTranslucent,
    SkinItemEntityTranslucent,
    SkinItemEntityTranslucentCull,
    SkinOutline,
    SkinOutlineCull,
    PlayerTextureCutout,
    PlayerTextureSolid,
    PlayerTextureCutoutCull,
    PlayerTextureSolidZOffsetForward,
    PlayerTextureCutoutZOffset,
    PlayerTextureArmorCutout,
    PlayerTextureArmorTrim,
    PlayerTextureArmorTrimDecal,
    PlayerTextureTranslucent,
    PlayerTextureItemEntityTranslucent,
    PlayerTextureItemEntityTranslucentCull,
    PlayerTextureOutline,
    PlayerTextureOutlineCull,
    DragonRays,
    DragonRaysDepth,
    Leash,
    EndPortal,
    EndGateway,
    Scroll,
    ScrollAdditive,
    GuardianBeam,
    EndCrystalBeam,
    BeaconBeamOpaque,
    BeaconBeamTranslucent,
    EntityGlint,
    ArmorEntityGlint,
}

impl EntityMeshLane {
    pub(crate) const COUNT: usize = Self::ArmorEntityGlint as usize + 1;

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::WaterMask => "bbb-entity-model-water-mask",
            Self::Solid => "bbb-entity-model-solid",
            Self::Textured => "bbb-entity-model-textured",
            Self::TexturedCull => "bbb-entity-model-textured-cull",
            Self::Dissolve => "bbb-entity-model-dissolve",
            Self::SolidZOffsetForward => "bbb-entity-model-solid-z-offset-forward",
            Self::CutoutZOffset => "bbb-entity-model-cutout-z-offset",
            Self::ArmorCutout => "bbb-entity-model-armor-cutout",
            Self::ArmorTrim => "bbb-entity-model-armor-trim",
            Self::ArmorTrimDecal => "bbb-entity-model-armor-trim-decal",
            Self::Eyes => "bbb-entity-model-eyes",
            Self::Outline => "bbb-entity-model-outline",
            Self::OutlineCull => "bbb-entity-model-outline-cull",
            Self::Translucent => "bbb-entity-model-translucent",
            Self::ArmorTranslucent => "bbb-entity-model-armor-translucent",
            Self::TranslucentEmissive => "bbb-entity-model-translucent-emissive",
            Self::ItemEntityTranslucent => "bbb-entity-model-item-entity-translucent",
            Self::ItemEntityTranslucentCull => "bbb-entity-model-item-entity-translucent-cull",
            Self::SkinCutout => "bbb-entity-dynamic-player-skin-cutout",
            Self::SkinSolid => "bbb-entity-dynamic-player-skin-solid",
            Self::SkinCutoutCull => "bbb-entity-dynamic-player-skin-cutout-cull",
            Self::SkinSolidZOffsetForward => {
                "bbb-entity-dynamic-player-skin-solid-z-offset-forward"
            }
            Self::SkinCutoutZOffset => "bbb-entity-dynamic-player-skin-cutout-z-offset",
            Self::SkinTranslucent => "bbb-entity-dynamic-player-skin-translucent",
            Self::SkinItemEntityTranslucent => {
                "bbb-entity-dynamic-player-skin-item-entity-translucent"
            }
            Self::SkinItemEntityTranslucentCull => {
                "bbb-entity-dynamic-player-skin-item-entity-translucent-cull"
            }
            Self::SkinOutline => "bbb-entity-dynamic-player-skin-outline",
            Self::SkinOutlineCull => "bbb-entity-dynamic-player-skin-outline-cull",
            Self::PlayerTextureCutout => "bbb-entity-dynamic-player-texture-cutout",
            Self::PlayerTextureSolid => "bbb-entity-dynamic-player-texture-solid",
            Self::PlayerTextureCutoutCull => "bbb-entity-dynamic-player-texture-cutout-cull",
            Self::PlayerTextureSolidZOffsetForward => {
                "bbb-entity-dynamic-player-texture-solid-z-offset-forward"
            }
            Self::PlayerTextureCutoutZOffset => "bbb-entity-dynamic-player-texture-cutout-z-offset",
            Self::PlayerTextureArmorCutout => "bbb-entity-dynamic-player-texture-armor-cutout",
            Self::PlayerTextureArmorTrim => "bbb-entity-dynamic-player-texture-armor-trim",
            Self::PlayerTextureArmorTrimDecal => {
                "bbb-entity-dynamic-player-texture-armor-trim-decal"
            }
            Self::PlayerTextureTranslucent => "bbb-entity-dynamic-player-texture-translucent",
            Self::PlayerTextureItemEntityTranslucent => {
                "bbb-entity-dynamic-player-texture-item-entity-translucent"
            }
            Self::PlayerTextureItemEntityTranslucentCull => {
                "bbb-entity-dynamic-player-texture-item-entity-translucent-cull"
            }
            Self::PlayerTextureOutline => "bbb-entity-dynamic-player-texture-outline",
            Self::PlayerTextureOutlineCull => "bbb-entity-dynamic-player-texture-outline-cull",
            Self::DragonRays => "bbb-entity-model-dragon-rays",
            Self::DragonRaysDepth => "bbb-entity-model-dragon-rays-depth",
            Self::Leash => "bbb-entity-model-leash",
            Self::EndPortal => "bbb-entity-model-end-portal",
            Self::EndGateway => "bbb-entity-model-end-gateway",
            Self::Scroll => "bbb-entity-model-scroll",
            Self::ScrollAdditive => "bbb-entity-model-scroll-additive",
            Self::GuardianBeam => "bbb-entity-model-guardian-beam",
            Self::EndCrystalBeam => "bbb-entity-model-end-crystal-beam",
            Self::BeaconBeamOpaque => "bbb-entity-model-end-gateway-beam-opaque",
            Self::BeaconBeamTranslucent => "bbb-entity-model-end-gateway-beam-translucent",
            Self::EntityGlint => "bbb-entity-model-entity-glint",
            Self::ArmorEntityGlint => "bbb-entity-model-armor-entity-glint",
        }
    }
}

pub(crate) struct EntityMeshes {
    pub(crate) lanes: [Option<EntityModelMeshGpu>; EntityMeshLane::COUNT],
    pub(crate) opaque_beam_draws: Vec<EntityModelDrawRange>,
    pub(crate) sorted_main_translucent_draws: Vec<EntityModelTranslucentDrawRange>,
    pub(crate) sorted_item_entity_draws: Vec<EntityModelTexturedDrawRange>,
}

impl Default for EntityMeshes {
    fn default() -> Self {
        Self {
            lanes: std::array::from_fn(|_| None),
            opaque_beam_draws: Vec::new(),
            sorted_main_translucent_draws: Vec::new(),
            sorted_item_entity_draws: Vec::new(),
        }
    }
}

impl EntityMeshes {
    pub(crate) fn has_any(&self, lanes: &[EntityMeshLane]) -> bool {
        lanes
            .iter()
            .any(|&lane| self.lanes[lane as usize].is_some())
    }

    pub(crate) fn bounds_meshes(&self) -> impl Iterator<Item = &EntityModelMeshGpu> {
        self.lanes
            .iter()
            .enumerate()
            .filter(|&(idx, _)| idx != EntityMeshLane::Leash as usize)
            .filter_map(|(_, mesh)| mesh.as_ref())
    }
}

impl Index<EntityMeshLane> for EntityMeshes {
    type Output = Option<EntityModelMeshGpu>;

    fn index(&self, lane: EntityMeshLane) -> &Self::Output {
        &self.lanes[lane as usize]
    }
}

impl IndexMut<EntityMeshLane> for EntityMeshes {
    fn index_mut(&mut self, lane: EntityMeshLane) -> &mut Self::Output {
        &mut self.lanes[lane as usize]
    }
}
