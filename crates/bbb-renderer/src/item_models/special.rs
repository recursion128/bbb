//! Renderer-owned `minecraft:special` item geometry and texture residency.
//!
//! Native resolves resource-pack ids and stack components into these values;
//! the renderer resolves the selected texture source against one of its entity
//! atlases, bakes the matching model layer into residency, and keeps only
//! placement, light, overlay, foil, target, and ordering in frame instances.

use std::sync::Arc;

use glam::Mat4;

use crate::{
    BedModelPart, ChestModelHalf, DecoratedPotPattern, EntityDyeColor, EntityDynamicPlayerSkin,
    EntityDynamicTexture, EntityModelOwnedTextureRef, EntityModelTextureRef, SignModelAttachment,
};

use super::{ItemModelDrawTarget, ItemModelFoil};

/// Texture source sampled by one special-model geometry part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecialItemTexture {
    Static(EntityModelTextureRef),
    Pack(EntityModelOwnedTextureRef),
    Dynamic {
        texture: EntityDynamicTexture,
        logical_size: [u32; 2],
    },
    DynamicPlayerSkin(EntityDynamicPlayerSkin),
}

impl From<EntityModelTextureRef> for SpecialItemTexture {
    fn from(texture: EntityModelTextureRef) -> Self {
        Self::Static(texture)
    }
}

impl From<EntityModelOwnedTextureRef> for SpecialItemTexture {
    fn from(texture: EntityModelOwnedTextureRef) -> Self {
        Self::Pack(texture)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecialItemPatternLayer {
    pub texture: SpecialItemTexture,
    pub color: EntityDyeColor,
}

impl SpecialItemPatternLayer {
    pub fn new(texture: SpecialItemTexture, color: EntityDyeColor) -> Self {
        Self { texture, color }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialItemBannerAttachment {
    Ground,
    Wall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialItemCopperGolemPose {
    Standing,
    Sitting,
    Star,
    Running,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialItemHeadKind {
    Skeleton,
    WitherSkeleton,
    Zombie,
    Creeper,
    Dragon,
    Piglin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialItemEndCubeEffect {
    Portal,
    Gateway,
}

/// All sixteen Vanilla 26.1 special renderer registrations. Texture choices
/// are already resource-resolved, while banner/shield/pot/player-head values
/// carry the effective stack-component argument.
#[derive(Debug, Clone, PartialEq)]
pub enum SpecialItemModel {
    Bed {
        texture: SpecialItemTexture,
        part: BedModelPart,
    },
    Bell {
        texture: SpecialItemTexture,
    },
    Banner {
        attachment: SpecialItemBannerAttachment,
        base_texture: SpecialItemTexture,
        pattern_base_texture: SpecialItemTexture,
        base_color: EntityDyeColor,
        patterns: Arc<[SpecialItemPatternLayer]>,
    },
    Book {
        texture: SpecialItemTexture,
        open_angle_radians: f32,
        page1: f32,
        page2: f32,
    },
    Conduit {
        texture: SpecialItemTexture,
    },
    Chest {
        texture: SpecialItemTexture,
        openness: f32,
        half: ChestModelHalf,
    },
    CopperGolemStatue {
        texture: SpecialItemTexture,
        pose: SpecialItemCopperGolemPose,
    },
    Head {
        texture: SpecialItemTexture,
        kind: SpecialItemHeadKind,
        animation: f32,
    },
    PlayerHead {
        /// A ready dynamic profile skin uses the dynamic skin atlas. Loading or
        /// failed profiles use `fallback_texture`, matching Vanilla's default
        /// skin fallback.
        skin: Option<EntityDynamicPlayerSkin>,
        fallback_texture: EntityModelTextureRef,
    },
    ShulkerBox {
        texture: SpecialItemTexture,
        openness: f32,
    },
    Shield {
        base_texture: SpecialItemTexture,
        pattern_base_texture: Option<SpecialItemTexture>,
        base_color: Option<EntityDyeColor>,
        patterns: Arc<[SpecialItemPatternLayer]>,
    },
    Trident {
        texture: SpecialItemTexture,
    },
    DecoratedPot {
        base_texture: SpecialItemTexture,
        /// Back, left, right, front, preserving `PotDecorations` field
        /// identity even though the renderer submits front first.
        sides: [SpecialItemTexture; 4],
        decorations: [Option<DecoratedPotPattern>; 4],
    },
    StandingSign {
        texture: SpecialItemTexture,
        attachment: SignModelAttachment,
    },
    HangingSign {
        texture: SpecialItemTexture,
        attachment: SignModelAttachment,
    },
    EndCube {
        sky_texture: SpecialItemTexture,
        portal_texture: SpecialItemTexture,
        effect: SpecialItemEndCubeEffect,
    },
}

impl SpecialItemModel {
    pub fn registered_type_name(&self) -> &'static str {
        match self {
            Self::Bed { .. } => "bed",
            Self::Bell { .. } => "bell",
            Self::Banner { .. } => "banner",
            Self::Book { .. } => "book",
            Self::Conduit { .. } => "conduit",
            Self::Chest { .. } => "chest",
            Self::CopperGolemStatue { .. } => "copper_golem_statue",
            Self::Head { .. } => "head",
            Self::PlayerHead { .. } => "player_head",
            Self::ShulkerBox { .. } => "shulker_box",
            Self::Shield { .. } => "shield",
            Self::Trident { .. } => "trident",
            Self::DecoratedPot { .. } => "decorated_pot",
            Self::StandingSign { .. } => "standing_sign",
            Self::HangingSign { .. } => "hanging_sign",
            Self::EndCube { .. } => "end_cube",
        }
    }
}

/// Exact model-space bounds generated from the same special geometry used by
/// residency. Values are in the renderer's unit model space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpecialItemModelBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl SpecialItemModelBounds {
    pub fn size(self) -> [f32; 3] {
        std::array::from_fn(|axis| self.max[axis] - self.min[axis])
    }
}

/// Narrow special submission. Static JSON and effective component arguments
/// live in `model` and therefore participate in exact resident identity;
/// placement and render-lane data remain frame-varying.
#[derive(Debug, Clone, PartialEq)]
pub struct SpecialItemInstanceRequest {
    pub model: SpecialItemModel,
    pub transform: Mat4,
    pub light: [f32; 2],
    pub overlay: [f32; 2],
    pub foil: ItemModelFoil,
    pub target: ItemModelDrawTarget,
    pub submit_order: u32,
}
