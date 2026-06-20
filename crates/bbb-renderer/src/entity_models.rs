use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use glam::{EulerRot, Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::{camera::TerrainBounds, gpu::DEPTH_FORMAT, Renderer};

mod model_layers;

use model_layers::*;
pub use model_layers::{
    entity_model_texture_refs, sheep_entity_texture_refs, wolf_entity_texture_refs,
};

const VANILLA_MODEL_ROOT_Y_OFFSET: f32 = 1.501;
const MODEL_UNIT_SCALE: f32 = 1.0 / 16.0;
const MESH_TRANSFORMER_ROOT_Y_OFFSET_PIXELS: f32 = 24.016;
const VILLAGER_LIKE_SCALE: f32 = 0.9375;
const HUSK_SCALE: f32 = 1.0625;
const WITHER_SKELETON_SCALE: f32 = 1.2;
const CAVE_SPIDER_SCALE: f32 = 0.7;
const AVATAR_RENDERER_SCALE: f32 = 0.9375;
const HORSE_SCALE: f32 = 1.1;
const DONKEY_SCALE: f32 = 0.87;
const MULE_SCALE: f32 = 0.92;
const POLAR_BEAR_SCALE: f32 = 1.2;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EntityModelKind {
    Chicken {
        variant: ChickenModelVariant,
        baby: bool,
    },
    Pig {
        variant: PigModelVariant,
        baby: bool,
    },
    Player {
        slim: bool,
    },
    Humanoid {
        family: HumanoidModelFamily,
        baby: bool,
    },
    ArmorStand {
        small: bool,
        show_arms: bool,
        show_base_plate: bool,
        pose: ArmorStandModelPose,
    },
    Slime {
        size: i32,
    },
    MagmaCube {
        size: i32,
    },
    Zombie {
        baby: bool,
    },
    ZombieVariant {
        family: ZombieVariantModelFamily,
        baby: bool,
    },
    Piglin {
        family: PiglinModelFamily,
        baby: bool,
    },
    Hoglin {
        family: HoglinModelFamily,
        baby: bool,
    },
    Ravager,
    Skeleton,
    SkeletonVariant {
        family: SkeletonModelFamily,
    },
    Cow {
        variant: CowModelVariant,
        baby: bool,
    },
    Sheep {
        baby: bool,
        sheared: bool,
        wool_color: SheepWoolColor,
    },
    Villager {
        baby: bool,
    },
    WanderingTrader,
    Wolf {
        baby: bool,
        tame: bool,
        angry: bool,
        collar_color: Option<EntityDyeColor>,
    },
    Horse {
        baby: bool,
    },
    Donkey {
        family: DonkeyModelFamily,
        baby: bool,
        has_chest: bool,
    },
    UndeadHorse {
        family: UndeadHorseModelFamily,
        baby: bool,
    },
    Camel {
        family: CamelModelFamily,
        baby: bool,
    },
    Llama {
        family: LlamaModelFamily,
        variant: LlamaVariant,
        baby: bool,
        has_chest: bool,
    },
    Goat {
        baby: bool,
        left_horn: bool,
        right_horn: bool,
    },
    PolarBear {
        baby: bool,
    },
    Quadruped {
        family: QuadrupedModelFamily,
        baby: bool,
    },
    Creeper,
    Spider,
    CaveSpider,
    Enderman,
    IronGolem,
    SnowGolem,
    Witch,
    Illager {
        family: IllagerModelFamily,
    },
    Minecart,
    Boat {
        family: BoatModelFamily,
        chest: bool,
    },
    Placeholder {
        name: &'static str,
        bounds: EntityModelBounds,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZombieVariantModelFamily {
    Husk,
    Drowned,
    ZombieVillager,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiglinModelFamily {
    Piglin,
    PiglinBrute,
    ZombifiedPiglin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoglinModelFamily {
    Hoglin,
    Zoglin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanoidModelFamily {
    Player,
    Zombie,
    Skeleton,
    Villager,
    Illager,
    ArmorStand,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmorStandModelPose {
    pub head: [f32; 3],
    pub body: [f32; 3],
    pub left_arm: [f32; 3],
    pub right_arm: [f32; 3],
    pub left_leg: [f32; 3],
    pub right_leg: [f32; 3],
}

pub const DEFAULT_ARMOR_STAND_MODEL_POSE: ArmorStandModelPose = ArmorStandModelPose {
    head: [0.0, 0.0, 0.0],
    body: [0.0, 0.0, 0.0],
    left_arm: [-10.0, 0.0, -10.0],
    right_arm: [-15.0, 0.0, 10.0],
    left_leg: [-1.0, 0.0, -1.0],
    right_leg: [1.0, 0.0, 1.0],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkeletonModelFamily {
    Stray,
    Parched,
    WitherSkeleton,
    Bogged { sheared: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IllagerModelFamily {
    Evoker,
    Illusioner,
    Pillager,
    Vindicator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuadrupedModelFamily {
    Pig,
    Cow,
    Sheep,
    Horse,
    Wolf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DonkeyModelFamily {
    Donkey,
    Mule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndeadHorseModelFamily {
    Skeleton,
    Zombie,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CamelModelFamily {
    Camel,
    CamelHusk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlamaModelFamily {
    Llama,
    TraderLlama,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlamaVariant {
    Creamy,
    White,
    Brown,
    Gray,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoatModelFamily {
    Acacia,
    Bamboo,
    Birch,
    Cherry,
    DarkOak,
    Jungle,
    Mangrove,
    Oak,
    PaleOak,
    Spruce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityDyeColor {
    White,
    Orange,
    Magenta,
    LightBlue,
    Yellow,
    Lime,
    Pink,
    Gray,
    LightGray,
    Cyan,
    Purple,
    Blue,
    Brown,
    Green,
    Red,
    Black,
}

impl EntityDyeColor {
    pub fn from_vanilla_id(id: i32) -> Self {
        match id {
            0 => Self::White,
            1 => Self::Orange,
            2 => Self::Magenta,
            3 => Self::LightBlue,
            4 => Self::Yellow,
            5 => Self::Lime,
            6 => Self::Pink,
            7 => Self::Gray,
            8 => Self::LightGray,
            9 => Self::Cyan,
            10 => Self::Purple,
            11 => Self::Blue,
            12 => Self::Brown,
            13 => Self::Green,
            14 => Self::Red,
            15 => Self::Black,
            _ => Self::White,
        }
    }

    pub fn vanilla_id(self) -> i32 {
        match self {
            Self::White => 0,
            Self::Orange => 1,
            Self::Magenta => 2,
            Self::LightBlue => 3,
            Self::Yellow => 4,
            Self::Lime => 5,
            Self::Pink => 6,
            Self::Gray => 7,
            Self::LightGray => 8,
            Self::Cyan => 9,
            Self::Purple => 10,
            Self::Blue => 11,
            Self::Brown => 12,
            Self::Green => 13,
            Self::Red => 14,
            Self::Black => 15,
        }
    }

    pub fn texture_diffuse_color(self) -> [f32; 4] {
        let [red, green, blue] = match self {
            Self::White => [249, 255, 254],
            Self::Orange => [249, 128, 29],
            Self::Magenta => [199, 78, 189],
            Self::LightBlue => [58, 179, 218],
            Self::Yellow => [254, 216, 61],
            Self::Lime => [128, 199, 31],
            Self::Pink => [243, 139, 170],
            Self::Gray => [71, 79, 82],
            Self::LightGray => [157, 157, 151],
            Self::Cyan => [22, 156, 156],
            Self::Purple => [137, 50, 184],
            Self::Blue => [60, 68, 170],
            Self::Brown => [131, 84, 50],
            Self::Green => [94, 124, 22],
            Self::Red => [176, 46, 38],
            Self::Black => [29, 29, 33],
        };
        [
            red as f32 / 255.0,
            green as f32 / 255.0,
            blue as f32 / 255.0,
            1.0,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChickenModelVariant {
    Temperate,
    Warm,
    Cold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PigModelVariant {
    Temperate,
    Warm,
    Cold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CowModelVariant {
    Temperate,
    Warm,
    Cold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheepWoolColor {
    White,
    Orange,
    Magenta,
    LightBlue,
    Yellow,
    Lime,
    Pink,
    Gray,
    LightGray,
    Cyan,
    Purple,
    Blue,
    Brown,
    Green,
    Red,
    Black,
}

impl LlamaVariant {
    pub fn from_vanilla_id(id: i32) -> Self {
        match id.clamp(0, 3) {
            0 => Self::Creamy,
            1 => Self::White,
            2 => Self::Brown,
            _ => Self::Gray,
        }
    }
}

impl SheepWoolColor {
    pub fn from_vanilla_id(id: u8) -> Self {
        match id {
            0 => Self::White,
            1 => Self::Orange,
            2 => Self::Magenta,
            3 => Self::LightBlue,
            4 => Self::Yellow,
            5 => Self::Lime,
            6 => Self::Pink,
            7 => Self::Gray,
            8 => Self::LightGray,
            9 => Self::Cyan,
            10 => Self::Purple,
            11 => Self::Blue,
            12 => Self::Brown,
            13 => Self::Green,
            14 => Self::Red,
            15 => Self::Black,
            _ => Self::White,
        }
    }

    pub fn vanilla_id(self) -> u8 {
        match self {
            Self::White => 0,
            Self::Orange => 1,
            Self::Magenta => 2,
            Self::LightBlue => 3,
            Self::Yellow => 4,
            Self::Lime => 5,
            Self::Pink => 6,
            Self::Gray => 7,
            Self::LightGray => 8,
            Self::Cyan => 9,
            Self::Purple => 10,
            Self::Blue => 11,
            Self::Brown => 12,
            Self::Green => 13,
            Self::Red => 14,
            Self::Black => 15,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityModelBounds {
    pub width: f32,
    pub height: f32,
    pub depth: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityModelTextureRef {
    pub path: &'static str,
    pub size: [u32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityModelTextureImage {
    pub texture: EntityModelTextureRef,
    pub rgba: Vec<u8>,
}

impl EntityModelTextureImage {
    pub fn new(texture: EntityModelTextureRef, rgba: Vec<u8>) -> Self {
        Self { texture, rgba }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityModelUvRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityModelTextureAtlasEntry {
    pub texture: EntityModelTextureRef,
    pub uv: EntityModelUvRect,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityModelTextureAtlasLayout {
    pub width: u32,
    pub height: u32,
    pub entries: Vec<EntityModelTextureAtlasEntry>,
}

impl EntityModelKind {
    pub fn model_key(self) -> &'static str {
        match self {
            Self::Chicken { variant, baby } => chicken_model_key(variant, baby),
            Self::Pig { variant, baby } => pig_model_key(variant, baby),
            Self::Player { slim: false } => "player",
            Self::Player { slim: true } => "player_slim",
            Self::Humanoid {
                family: HumanoidModelFamily::Player,
                baby: false,
            } => "humanoid_player",
            Self::Humanoid {
                family: HumanoidModelFamily::Player,
                baby: true,
            } => "humanoid_player_baby",
            Self::Humanoid {
                family: HumanoidModelFamily::Zombie,
                baby: false,
            } => "humanoid_zombie",
            Self::Humanoid {
                family: HumanoidModelFamily::Zombie,
                baby: true,
            } => "humanoid_zombie_baby",
            Self::Humanoid {
                family: HumanoidModelFamily::Skeleton,
                baby: false,
            } => "humanoid_skeleton",
            Self::Humanoid {
                family: HumanoidModelFamily::Skeleton,
                baby: true,
            } => "humanoid_skeleton_baby",
            Self::Humanoid {
                family: HumanoidModelFamily::Villager,
                baby: false,
            } => "humanoid_villager",
            Self::Humanoid {
                family: HumanoidModelFamily::Villager,
                baby: true,
            } => "humanoid_villager_baby",
            Self::Humanoid {
                family: HumanoidModelFamily::Illager,
                baby: false,
            } => "humanoid_illager",
            Self::Humanoid {
                family: HumanoidModelFamily::Illager,
                baby: true,
            } => "humanoid_illager_baby",
            Self::Humanoid {
                family: HumanoidModelFamily::ArmorStand,
                baby: false,
            } => "humanoid_armor_stand",
            Self::Humanoid {
                family: HumanoidModelFamily::ArmorStand,
                baby: true,
            } => "humanoid_armor_stand_baby",
            Self::ArmorStand { small: false, .. } => "armor_stand",
            Self::ArmorStand { small: true, .. } => "armor_stand_small",
            Self::Slime { .. } => "slime",
            Self::MagmaCube { .. } => "magma_cube",
            Self::Zombie { baby: false } => "zombie",
            Self::Zombie { baby: true } => "zombie_baby",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: false,
            } => "husk",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: true,
            } => "husk_baby",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: false,
            } => "drowned",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: true,
            } => "drowned_baby",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: false,
            } => "zombie_villager",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: true,
            } => "zombie_villager_baby",
            Self::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: false,
            } => "piglin",
            Self::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: true,
            } => "piglin_baby",
            Self::Piglin {
                family: PiglinModelFamily::PiglinBrute,
                ..
            } => "piglin_brute",
            Self::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: false,
            } => "zombified_piglin",
            Self::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: true,
            } => "zombified_piglin_baby",
            Self::Hoglin {
                family: HoglinModelFamily::Hoglin,
                baby: false,
            } => "hoglin",
            Self::Hoglin {
                family: HoglinModelFamily::Hoglin,
                baby: true,
            } => "hoglin_baby",
            Self::Hoglin {
                family: HoglinModelFamily::Zoglin,
                baby: false,
            } => "zoglin",
            Self::Hoglin {
                family: HoglinModelFamily::Zoglin,
                baby: true,
            } => "zoglin_baby",
            Self::Ravager => "ravager",
            Self::Skeleton => "skeleton",
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Stray,
            } => "stray",
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Parched,
            } => "parched",
            Self::SkeletonVariant {
                family: SkeletonModelFamily::WitherSkeleton,
            } => "wither_skeleton",
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Bogged { .. },
            } => "bogged",
            Self::Cow { variant, baby } => cow_model_key(variant, baby),
            Self::Sheep {
                baby,
                sheared,
                wool_color,
            } => sheep_model_key(baby, sheared, wool_color),
            Self::Villager { baby: false } => "villager",
            Self::Villager { baby: true } => "villager_baby",
            Self::WanderingTrader => "wandering_trader",
            Self::Wolf {
                baby, tame, angry, ..
            } => wolf_model_key(baby, tame, angry),
            Self::Horse { baby: false } => "horse",
            Self::Horse { baby: true } => "horse_baby",
            Self::Donkey {
                family: DonkeyModelFamily::Donkey,
                baby: false,
                ..
            } => "donkey",
            Self::Donkey {
                family: DonkeyModelFamily::Donkey,
                baby: true,
                ..
            } => "donkey_baby",
            Self::Donkey {
                family: DonkeyModelFamily::Mule,
                baby: false,
                ..
            } => "mule",
            Self::Donkey {
                family: DonkeyModelFamily::Mule,
                baby: true,
                ..
            } => "mule_baby",
            Self::UndeadHorse {
                family: UndeadHorseModelFamily::Skeleton,
                baby: false,
            } => "skeleton_horse",
            Self::UndeadHorse {
                family: UndeadHorseModelFamily::Skeleton,
                baby: true,
            } => "skeleton_horse_baby",
            Self::UndeadHorse {
                family: UndeadHorseModelFamily::Zombie,
                baby: false,
            } => "zombie_horse",
            Self::UndeadHorse {
                family: UndeadHorseModelFamily::Zombie,
                baby: true,
            } => "zombie_horse_baby",
            Self::Camel {
                family: CamelModelFamily::Camel,
                baby: false,
            } => "camel",
            Self::Camel {
                family: CamelModelFamily::Camel,
                baby: true,
            } => "camel_baby",
            Self::Camel {
                family: CamelModelFamily::CamelHusk,
                ..
            } => "camel_husk",
            Self::Llama {
                family,
                variant,
                baby,
                ..
            } => llama_model_key(family, variant, baby),
            Self::Goat { baby: false, .. } => "goat",
            Self::Goat { baby: true, .. } => "goat_baby",
            Self::PolarBear { baby: false } => "polar_bear",
            Self::PolarBear { baby: true } => "polar_bear_baby",
            Self::Quadruped {
                family: QuadrupedModelFamily::Pig,
                baby: false,
            } => "quadruped_pig",
            Self::Quadruped {
                family: QuadrupedModelFamily::Pig,
                baby: true,
            } => "quadruped_pig_baby",
            Self::Quadruped {
                family: QuadrupedModelFamily::Cow,
                baby: false,
            } => "quadruped_cow",
            Self::Quadruped {
                family: QuadrupedModelFamily::Cow,
                baby: true,
            } => "quadruped_cow_baby",
            Self::Quadruped {
                family: QuadrupedModelFamily::Sheep,
                baby: false,
            } => "quadruped_sheep",
            Self::Quadruped {
                family: QuadrupedModelFamily::Sheep,
                baby: true,
            } => "quadruped_sheep_baby",
            Self::Quadruped {
                family: QuadrupedModelFamily::Horse,
                baby: false,
            } => "quadruped_horse",
            Self::Quadruped {
                family: QuadrupedModelFamily::Horse,
                baby: true,
            } => "quadruped_horse_baby",
            Self::Quadruped {
                family: QuadrupedModelFamily::Wolf,
                baby: false,
            } => "quadruped_wolf",
            Self::Quadruped {
                family: QuadrupedModelFamily::Wolf,
                baby: true,
            } => "quadruped_wolf_baby",
            Self::Creeper => "creeper",
            Self::Spider => "spider",
            Self::CaveSpider => "cave_spider",
            Self::Enderman => "enderman",
            Self::IronGolem => "iron_golem",
            Self::SnowGolem => "snow_golem",
            Self::Witch => "witch",
            Self::Illager {
                family: IllagerModelFamily::Evoker,
            } => "evoker",
            Self::Illager {
                family: IllagerModelFamily::Illusioner,
            } => "illusioner",
            Self::Illager {
                family: IllagerModelFamily::Pillager,
            } => "pillager",
            Self::Illager {
                family: IllagerModelFamily::Vindicator,
            } => "vindicator",
            Self::Minecart => "minecart",
            Self::Boat { family, chest } => boat_model_key(family, chest),
            Self::Placeholder { name, .. } => name,
        }
    }

    pub fn vanilla_texture_ref(self) -> Option<EntityModelTextureRef> {
        match self {
            Self::Chicken { variant, baby } => Some(chicken_texture_ref(variant, baby)),
            Self::Pig { variant, baby } => Some(pig_texture_ref(variant, baby)),
            Self::Player { slim: false } => Some(PLAYER_WIDE_STEVE_TEXTURE_REF),
            Self::Player { slim: true } => Some(PLAYER_SLIM_STEVE_TEXTURE_REF),
            Self::ArmorStand { .. } => Some(ARMOR_STAND_TEXTURE_REF),
            Self::Slime { .. } => Some(SLIME_TEXTURE_REF),
            Self::MagmaCube { .. } => Some(MAGMA_CUBE_TEXTURE_REF),
            Self::Zombie { baby: false } => Some(ZOMBIE_TEXTURE_REF),
            Self::Zombie { baby: true } => Some(ZOMBIE_BABY_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: false,
            } => Some(HUSK_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: true,
            } => Some(HUSK_BABY_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: false,
            } => Some(DROWNED_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: true,
            } => Some(DROWNED_BABY_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: false,
            } => Some(ZOMBIE_VILLAGER_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: true,
            } => Some(ZOMBIE_VILLAGER_BABY_TEXTURE_REF),
            Self::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: false,
            } => Some(PIGLIN_TEXTURE_REF),
            Self::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: true,
            } => Some(PIGLIN_BABY_TEXTURE_REF),
            Self::Piglin {
                family: PiglinModelFamily::PiglinBrute,
                ..
            } => Some(PIGLIN_BRUTE_TEXTURE_REF),
            Self::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: false,
            } => Some(ZOMBIFIED_PIGLIN_TEXTURE_REF),
            Self::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: true,
            } => Some(ZOMBIFIED_PIGLIN_BABY_TEXTURE_REF),
            Self::Hoglin {
                family: HoglinModelFamily::Hoglin,
                baby: false,
            } => Some(HOGLIN_TEXTURE_REF),
            Self::Hoglin {
                family: HoglinModelFamily::Hoglin,
                baby: true,
            } => Some(HOGLIN_BABY_TEXTURE_REF),
            Self::Hoglin {
                family: HoglinModelFamily::Zoglin,
                baby: false,
            } => Some(ZOGLIN_TEXTURE_REF),
            Self::Hoglin {
                family: HoglinModelFamily::Zoglin,
                baby: true,
            } => Some(ZOGLIN_BABY_TEXTURE_REF),
            Self::Ravager => Some(RAVAGER_TEXTURE_REF),
            Self::Skeleton => Some(SKELETON_TEXTURE_REF),
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Stray,
            } => Some(STRAY_TEXTURE_REF),
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Parched,
            } => Some(PARCHED_TEXTURE_REF),
            Self::SkeletonVariant {
                family: SkeletonModelFamily::WitherSkeleton,
            } => Some(WITHER_SKELETON_TEXTURE_REF),
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Bogged { .. },
            } => Some(BOGGED_TEXTURE_REF),
            Self::Cow { variant, baby } => Some(cow_texture_ref(variant, baby)),
            Self::Sheep { baby: false, .. } => Some(SHEEP_TEXTURE_REF),
            Self::Sheep { baby: true, .. } => Some(SHEEP_BABY_TEXTURE_REF),
            Self::Villager { baby: false } => Some(VILLAGER_TEXTURE_REF),
            Self::Villager { baby: true } => Some(VILLAGER_BABY_TEXTURE_REF),
            Self::WanderingTrader => Some(WANDERING_TRADER_TEXTURE_REF),
            Self::Wolf {
                baby, tame, angry, ..
            } => Some(wolf_texture_ref(baby, tame, angry)),
            Self::Horse { baby: false } => Some(HORSE_WHITE_TEXTURE_REF),
            Self::Horse { baby: true } => Some(HORSE_WHITE_BABY_TEXTURE_REF),
            Self::Donkey {
                family: DonkeyModelFamily::Donkey,
                baby: false,
                ..
            } => Some(DONKEY_TEXTURE_REF),
            Self::Donkey {
                family: DonkeyModelFamily::Donkey,
                baby: true,
                ..
            } => Some(DONKEY_BABY_TEXTURE_REF),
            Self::Donkey {
                family: DonkeyModelFamily::Mule,
                baby: false,
                ..
            } => Some(MULE_TEXTURE_REF),
            Self::Donkey {
                family: DonkeyModelFamily::Mule,
                baby: true,
                ..
            } => Some(MULE_BABY_TEXTURE_REF),
            Self::UndeadHorse {
                family: UndeadHorseModelFamily::Skeleton,
                baby: false,
            } => Some(SKELETON_HORSE_TEXTURE_REF),
            Self::UndeadHorse {
                family: UndeadHorseModelFamily::Skeleton,
                baby: true,
            } => Some(SKELETON_HORSE_BABY_TEXTURE_REF),
            Self::UndeadHorse {
                family: UndeadHorseModelFamily::Zombie,
                baby: false,
            } => Some(ZOMBIE_HORSE_TEXTURE_REF),
            Self::UndeadHorse {
                family: UndeadHorseModelFamily::Zombie,
                baby: true,
            } => Some(ZOMBIE_HORSE_BABY_TEXTURE_REF),
            Self::Camel {
                family: CamelModelFamily::Camel,
                baby: false,
            } => Some(CAMEL_TEXTURE_REF),
            Self::Camel {
                family: CamelModelFamily::Camel,
                baby: true,
            } => Some(CAMEL_BABY_TEXTURE_REF),
            Self::Camel {
                family: CamelModelFamily::CamelHusk,
                ..
            } => Some(CAMEL_HUSK_TEXTURE_REF),
            Self::Llama { variant, baby, .. } => Some(llama_texture_ref(variant, baby)),
            Self::Goat { baby: false, .. } => Some(GOAT_TEXTURE_REF),
            Self::Goat { baby: true, .. } => Some(GOAT_BABY_TEXTURE_REF),
            Self::PolarBear { baby: false } => Some(POLAR_BEAR_TEXTURE_REF),
            Self::PolarBear { baby: true } => Some(POLAR_BEAR_BABY_TEXTURE_REF),
            Self::Creeper => Some(CREEPER_TEXTURE_REF),
            Self::Spider => Some(SPIDER_TEXTURE_REF),
            Self::CaveSpider => Some(CAVE_SPIDER_TEXTURE_REF),
            Self::Enderman => Some(ENDERMAN_TEXTURE_REF),
            Self::IronGolem => Some(IRON_GOLEM_TEXTURE_REF),
            Self::SnowGolem => Some(SNOW_GOLEM_TEXTURE_REF),
            Self::Witch => Some(WITCH_TEXTURE_REF),
            Self::Illager {
                family: IllagerModelFamily::Evoker,
            } => Some(EVOKER_TEXTURE_REF),
            Self::Illager {
                family: IllagerModelFamily::Illusioner,
            } => Some(ILLUSIONER_TEXTURE_REF),
            Self::Illager {
                family: IllagerModelFamily::Pillager,
            } => Some(PILLAGER_TEXTURE_REF),
            Self::Illager {
                family: IllagerModelFamily::Vindicator,
            } => Some(VINDICATOR_TEXTURE_REF),
            Self::Boat { family, chest } => Some(boat_texture_ref(family, chest)),
            _ => None,
        }
    }

    pub fn vanilla_layer_texture_refs(self) -> &'static [EntityModelTextureRef] {
        match self {
            Self::Sheep {
                baby: false,
                sheared: false,
                wool_color: SheepWoolColor::White,
            } => &SHEEP_WOOL_LAYER_TEXTURE_REFS,
            Self::Sheep {
                baby: false,
                sheared: false,
                ..
            } => &SHEEP_COLORED_WOOL_LAYER_TEXTURE_REFS,
            Self::Sheep {
                baby: false,
                sheared: true,
                wool_color: SheepWoolColor::White,
            } => &[],
            Self::Sheep {
                baby: false,
                sheared: true,
                ..
            } => &SHEEP_UNDERCOAT_LAYER_TEXTURE_REFS,
            Self::Sheep {
                baby: true,
                sheared: false,
                ..
            } => &BABY_SHEEP_WOOL_LAYER_TEXTURE_REFS,
            Self::Sheep {
                baby: true,
                sheared: true,
                ..
            } => &[],
            Self::Wolf {
                baby: false,
                tame: true,
                collar_color: Some(_),
                ..
            } => &WOLF_COLLAR_LAYER_TEXTURE_REFS,
            Self::Wolf {
                baby: true,
                tame: true,
                collar_color: Some(_),
                ..
            } => &WOLF_BABY_COLLAR_LAYER_TEXTURE_REFS,
            _ => &[],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityModelInstance {
    pub entity_id: i32,
    pub kind: EntityModelKind,
    pub position: [f32; 3],
    pub y_rot: f32,
}

impl EntityModelInstance {
    pub fn new(entity_id: i32, kind: EntityModelKind, position: [f32; 3], y_rot: f32) -> Self {
        Self {
            entity_id,
            kind,
            position,
            y_rot,
        }
    }

    pub fn chicken(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::chicken_variant(
            entity_id,
            position,
            y_rot,
            ChickenModelVariant::Temperate,
            baby,
        )
    }

    pub fn chicken_variant(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        variant: ChickenModelVariant,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Chicken { variant, baby },
            position,
            y_rot,
        )
    }

    pub fn pig(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        variant: PigModelVariant,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Pig { variant, baby },
            position,
            y_rot,
        )
    }

    pub fn player(entity_id: i32, position: [f32; 3], y_rot: f32, slim: bool) -> Self {
        Self::new(entity_id, EntityModelKind::Player { slim }, position, y_rot)
    }

    pub fn humanoid(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: HumanoidModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Humanoid { family, baby },
            position,
            y_rot,
        )
    }

    pub fn armor_stand(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        small: bool,
        show_arms: bool,
        show_base_plate: bool,
        pose: ArmorStandModelPose,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::ArmorStand {
                small,
                show_arms,
                show_base_plate,
                pose,
            },
            position,
            y_rot,
        )
    }

    pub fn slime(entity_id: i32, position: [f32; 3], y_rot: f32, size: i32) -> Self {
        Self::new(entity_id, EntityModelKind::Slime { size }, position, y_rot)
    }

    pub fn magma_cube(entity_id: i32, position: [f32; 3], y_rot: f32, size: i32) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::MagmaCube { size },
            position,
            y_rot,
        )
    }

    pub fn zombie(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(entity_id, EntityModelKind::Zombie { baby }, position, y_rot)
    }

    pub fn zombie_variant(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: ZombieVariantModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::ZombieVariant { family, baby },
            position,
            y_rot,
        )
    }

    pub fn piglin(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: PiglinModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Piglin { family, baby },
            position,
            y_rot,
        )
    }

    pub fn hoglin(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: HoglinModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Hoglin { family, baby },
            position,
            y_rot,
        )
    }

    pub fn ravager(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Ravager, position, y_rot)
    }

    pub fn boat(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: BoatModelFamily,
        chest: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Boat { family, chest },
            position,
            y_rot,
        )
    }

    pub fn skeleton(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Skeleton, position, y_rot)
    }

    pub fn skeleton_variant(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: SkeletonModelFamily,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::SkeletonVariant { family },
            position,
            y_rot,
        )
    }

    pub fn cow(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::cow_variant(entity_id, position, y_rot, CowModelVariant::Temperate, baby)
    }

    pub fn cow_variant(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        variant: CowModelVariant,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Cow { variant, baby },
            position,
            y_rot,
        )
    }

    pub fn sheep(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::sheep_wool(
            entity_id,
            position,
            y_rot,
            baby,
            false,
            SheepWoolColor::White,
        )
    }

    pub fn sheep_wool(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        sheared: bool,
        wool_color: SheepWoolColor,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Sheep {
                baby,
                sheared,
                wool_color,
            },
            position,
            y_rot,
        )
    }

    pub fn villager(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Villager { baby },
            position,
            y_rot,
        )
    }

    pub fn wandering_trader(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::WanderingTrader, position, y_rot)
    }

    pub fn wolf(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::wolf_state(entity_id, position, y_rot, baby, false, false, None)
    }

    pub fn wolf_state(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        tame: bool,
        angry: bool,
        collar_color: Option<EntityDyeColor>,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Wolf {
                baby,
                tame,
                angry,
                collar_color: tame.then_some(collar_color).flatten(),
            },
            position,
            y_rot,
        )
    }

    pub fn horse(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(entity_id, EntityModelKind::Horse { baby }, position, y_rot)
    }

    pub fn donkey(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: DonkeyModelFamily,
        baby: bool,
        has_chest: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Donkey {
                family,
                baby,
                has_chest,
            },
            position,
            y_rot,
        )
    }

    pub fn undead_horse(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: UndeadHorseModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::UndeadHorse { family, baby },
            position,
            y_rot,
        )
    }

    pub fn camel(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: CamelModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Camel { family, baby },
            position,
            y_rot,
        )
    }

    pub fn llama(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: LlamaModelFamily,
        variant: LlamaVariant,
        baby: bool,
        has_chest: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Llama {
                family,
                variant,
                baby,
                has_chest,
            },
            position,
            y_rot,
        )
    }

    pub fn goat(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        left_horn: bool,
        right_horn: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Goat {
                baby,
                left_horn,
                right_horn,
            },
            position,
            y_rot,
        )
    }

    pub fn polar_bear(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::PolarBear { baby },
            position,
            y_rot,
        )
    }

    pub fn spider(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Spider, position, y_rot)
    }

    pub fn cave_spider(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::CaveSpider, position, y_rot)
    }

    pub fn enderman(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Enderman, position, y_rot)
    }

    pub fn iron_golem(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::IronGolem, position, y_rot)
    }

    pub fn snow_golem(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::SnowGolem, position, y_rot)
    }

    pub fn witch(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Witch, position, y_rot)
    }

    pub fn illager(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: IllagerModelFamily,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Illager { family },
            position,
            y_rot,
        )
    }

    pub fn quadruped(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: QuadrupedModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Quadruped { family, baby },
            position,
            y_rot,
        )
    }

    pub fn placeholder(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        name: &'static str,
        width: f32,
        height: f32,
        depth: f32,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Placeholder {
                name,
                bounds: EntityModelBounds {
                    width,
                    height,
                    depth,
                },
            },
            position,
            y_rot,
        )
    }
}

pub(super) struct EntityModelMeshGpu {
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) index_buffer: wgpu::Buffer,
    pub(super) index_count: u32,
    pub(super) bounds: Option<TerrainBounds>,
}

pub(super) struct EntityModelTexturedMeshGpu {
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) index_buffer: wgpu::Buffer,
    pub(super) index_count: u32,
    pub(super) bounds: Option<TerrainBounds>,
}

pub(super) struct EntityModelTextureAtlasGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) layout: EntityModelTextureAtlasLayout,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct EntityModelVertex {
    pub(super) position: [f32; 3],
    pub(super) color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct EntityModelTexturedVertex {
    pub(super) position: [f32; 3],
    pub(super) uv: [f32; 2],
    pub(super) tint: [f32; 4],
}

struct EntityModelMesh {
    vertices: Vec<EntityModelVertex>,
    indices: Vec<u32>,
    opaque_faces: usize,
}

impl EntityModelMesh {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            opaque_faces: 0,
        }
    }
}

struct EntityModelTexturedMesh {
    vertices: Vec<EntityModelTexturedVertex>,
    indices: Vec<u32>,
    cutout_faces: usize,
}

impl EntityModelTexturedMesh {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            cutout_faces: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ModelPartDesc {
    pose: PartPose,
    cubes: &'static [ModelCubeDesc],
    children: &'static [ModelPartDesc],
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ModelCubeDesc {
    min: [f32; 3],
    size: [f32; 3],
    color: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TexturedModelPartDesc {
    pose: PartPose,
    cubes: &'static [TexturedModelCubeDesc],
    children: &'static [TexturedModelPartDesc],
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TexturedModelCubeDesc {
    min: [f32; 3],
    size: [f32; 3],
    uv_size: [f32; 3],
    tex: [f32; 2],
    mirror: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityModelLayerKind {
    SheepBase,
    SheepWool,
    SheepWoolUndercoat,
    WolfBase,
    WolfCollar,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct EntityModelLayerPass {
    kind: EntityModelLayerKind,
    model_layer: &'static str,
    texture: EntityModelTextureRef,
    parts: &'static [TexturedModelPartDesc],
    tint: [f32; 4],
    collector_order: i32,
    submit_sequence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PartPose {
    offset: [f32; 3],
    rotation: [f32; 3],
}

const PART_POSE_ZERO: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

const ENTITY_MODEL_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
const ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4];

const ENTITY_MODEL_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

const ENTITY_MODEL_TEXTURED_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var entity_texture_atlas: texture_2d<f32>;

@group(0) @binding(2)
var entity_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tint: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.tint = input.tint;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(entity_texture_atlas, entity_sampler, input.uv) * input.tint;
    if texel.a <= 0.01 {
        discard;
    }
    return texel;
}
"#;

pub(crate) fn create_entity_model_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-entity-model-shader"),
        source: wgpu::ShaderSource::Wgsl(ENTITY_MODEL_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-entity-model-pipeline-layout"),
        bind_group_layouts: &[camera_bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-entity-model-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[entity_model_vertex_layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

pub(crate) fn create_entity_model_textured_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-entity-model-textured-shader"),
        source: wgpu::ShaderSource::Wgsl(ENTITY_MODEL_TEXTURED_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-entity-model-textured-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-entity-model-textured-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[entity_model_textured_vertex_layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

impl Renderer {
    pub fn upload_entity_model_textures(
        &mut self,
        images: &[EntityModelTextureImage],
    ) -> Result<()> {
        self.entity_model_texture_atlas = Some(create_entity_model_texture_atlas_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffer,
            images,
        )?);
        self.rebuild_entity_model_meshes();
        Ok(())
    }

    pub fn set_entity_model_instances(&mut self, instances: Vec<EntityModelInstance>) {
        let instances = sanitize_entity_model_instances(instances);
        if self.entity_model_instances.as_slice() == instances.as_slice() {
            return;
        }

        self.entity_model_instances = instances;
        self.rebuild_entity_model_meshes();
    }

    fn rebuild_entity_model_meshes(&mut self) {
        self.entity_model_mesh =
            create_entity_model_mesh_gpu(&self.device, self.entity_model_instances.clone());
        self.entity_model_textured_mesh =
            self.entity_model_texture_atlas.as_ref().and_then(|atlas| {
                create_entity_model_textured_mesh_gpu(
                    &self.device,
                    &self.entity_model_instances,
                    &atlas.layout,
                )
            });
        self.entity_model_bounds = merged_entity_model_bounds(
            self.entity_model_mesh.as_ref().and_then(|mesh| mesh.bounds),
            self.entity_model_textured_mesh
                .as_ref()
                .and_then(|mesh| mesh.bounds),
        );
        self.update_camera();
    }
}

fn create_entity_model_texture_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    images: &[EntityModelTextureImage],
) -> Result<EntityModelTextureAtlasGpu> {
    let (layout, rgba) = build_entity_model_texture_atlas(images)?;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-entity-model-texture-atlas"),
        size: wgpu::Extent3d {
            width: layout.width,
            height: layout.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(layout.width * 4),
            rows_per_image: Some(layout.height),
        },
        wgpu::Extent3d {
            width: layout.width,
            height: layout.height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-entity-model-texture-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-entity-model-texture-bind-group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    Ok(EntityModelTextureAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        layout,
    })
}

fn build_entity_model_texture_atlas(
    images: &[EntityModelTextureImage],
) -> Result<(EntityModelTextureAtlasLayout, Vec<u8>)> {
    if images.is_empty() {
        bail!("entity model texture atlas requires at least one image");
    }
    let mut seen = BTreeMap::new();
    let mut width = 0u32;
    let mut height = 0u32;
    for image in images {
        validate_entity_model_texture_image(image)?;
        if seen.insert(image.texture.path, ()).is_some() {
            bail!("duplicate entity model texture {}", image.texture.path);
        }
        width = width.max(image.texture.size[0]);
        height = height
            .checked_add(image.texture.size[1])
            .ok_or_else(|| anyhow!("entity model texture atlas height overflow"))?;
    }
    if width == 0 || height == 0 {
        bail!("entity model texture atlas dimensions must be non-zero");
    }
    let atlas_len = rgba_len(width, height, "entity model texture atlas")?;
    let mut rgba = vec![0u8; atlas_len];
    let mut entries = Vec::with_capacity(images.len());
    let mut y = 0u32;
    for image in images {
        let image_width = image.texture.size[0];
        let image_height = image.texture.size[1];
        let row_len = usize::try_from(image_width)
            .ok()
            .and_then(|width| width.checked_mul(4))
            .ok_or_else(|| anyhow!("entity model texture row size overflow"))?;
        for row in 0..image_height {
            let src_start = rgba_offset(image_width, row, 0, "entity model texture source")?;
            let src_end = src_start + row_len;
            let dst_start = rgba_offset(width, y + row, 0, "entity model texture atlas")?;
            let dst_end = dst_start + row_len;
            rgba[dst_start..dst_end].copy_from_slice(&image.rgba[src_start..src_end]);
        }
        entries.push(EntityModelTextureAtlasEntry {
            texture: image.texture,
            uv: EntityModelUvRect {
                min: [0.0, y as f32 / height as f32],
                max: [
                    image_width as f32 / width as f32,
                    (y + image_height) as f32 / height as f32,
                ],
            },
        });
        y += image_height;
    }

    Ok((
        EntityModelTextureAtlasLayout {
            width,
            height,
            entries,
        },
        rgba,
    ))
}

fn validate_entity_model_texture_image(image: &EntityModelTextureImage) -> Result<()> {
    let [width, height] = image.texture.size;
    if width == 0 || height == 0 {
        bail!(
            "entity model texture {} has zero-sized dimensions",
            image.texture.path
        );
    }
    let expected_len = rgba_len(width, height, image.texture.path)?;
    if image.rgba.len() != expected_len {
        bail!(
            "entity model texture {} has {} RGBA bytes, expected {} for {}x{}",
            image.texture.path,
            image.rgba.len(),
            expected_len,
            width,
            height
        );
    }
    Ok(())
}

fn rgba_len(width: u32, height: u32, label: &str) -> Result<usize> {
    usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("{label} RGBA size overflow"))
}

fn rgba_offset(width: u32, y: u32, x: u32, label: &str) -> Result<usize> {
    let width = usize::try_from(width).map_err(|_| anyhow!("{label} width overflow"))?;
    let x = usize::try_from(x).map_err(|_| anyhow!("{label} x overflow"))?;
    let y = usize::try_from(y).map_err(|_| anyhow!("{label} y overflow"))?;
    y.checked_mul(width)
        .and_then(|offset| offset.checked_add(x))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("{label} RGBA offset overflow"))
}

fn create_entity_model_mesh_gpu(
    device: &wgpu::Device,
    instances: Vec<EntityModelInstance>,
) -> Option<EntityModelMeshGpu> {
    let mesh = entity_model_colored_runtime_mesh(&instances);
    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
        return None;
    }
    let bounds = TerrainBounds::from_points(
        mesh.vertices
            .iter()
            .map(|vertex| Vec3::from_array(vertex.position)),
    );
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-entity-model-vertices"),
        contents: bytemuck::cast_slice(&mesh.vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-entity-model-indices"),
        contents: bytemuck::cast_slice(&mesh.indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    Some(EntityModelMeshGpu {
        vertex_buffer,
        index_buffer,
        index_count: mesh.indices.len() as u32,
        bounds,
    })
}

fn create_entity_model_textured_mesh_gpu(
    device: &wgpu::Device,
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> Option<EntityModelTexturedMeshGpu> {
    let mesh = entity_model_textured_mesh(instances, atlas);
    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
        return None;
    }
    let bounds = TerrainBounds::from_points(
        mesh.vertices
            .iter()
            .map(|vertex| Vec3::from_array(vertex.position)),
    );
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-entity-model-textured-vertices"),
        contents: bytemuck::cast_slice(&mesh.vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-entity-model-textured-indices"),
        contents: bytemuck::cast_slice(&mesh.indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    Some(EntityModelTexturedMeshGpu {
        vertex_buffer,
        index_buffer,
        index_count: mesh.indices.len() as u32,
        bounds,
    })
}

fn merged_entity_model_bounds(
    colored: Option<TerrainBounds>,
    textured: Option<TerrainBounds>,
) -> Option<TerrainBounds> {
    match (colored, textured) {
        (Some(mut colored), Some(textured)) => {
            colored.include_bounds(textured);
            Some(colored)
        }
        (Some(bounds), None) | (None, Some(bounds)) => Some(bounds),
        (None, None) => None,
    }
}

fn sanitize_entity_model_instances(
    instances: Vec<EntityModelInstance>,
) -> Vec<EntityModelInstance> {
    instances
        .into_iter()
        .filter(|instance| {
            instance.y_rot.is_finite()
                && instance
                    .position
                    .iter()
                    .all(|component| component.is_finite())
        })
        .collect()
}

#[cfg(test)]
fn entity_model_mesh(instances: &[EntityModelInstance]) -> EntityModelMesh {
    entity_model_mesh_with_options(instances, false)
}

fn entity_model_colored_runtime_mesh(instances: &[EntityModelInstance]) -> EntityModelMesh {
    entity_model_mesh_with_options(instances, true)
}

fn entity_model_mesh_with_options(
    instances: &[EntityModelInstance],
    skip_texture_backed_entities: bool,
) -> EntityModelMesh {
    let mut mesh = EntityModelMesh::new();
    for instance in instances {
        match instance.kind {
            EntityModelKind::Chicken { variant, baby } => emit_model_parts(
                &mut mesh,
                chicken_model_parts(variant, baby),
                entity_model_root_transform(*instance),
            ),
            EntityModelKind::Pig { variant, baby } => {
                emit_pig_model(&mut mesh, *instance, variant, baby)
            }
            EntityModelKind::Player { slim } => emit_player_model(&mut mesh, *instance, slim),
            EntityModelKind::Humanoid { family, baby } => {
                emit_humanoid_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::ArmorStand {
                small,
                show_arms,
                show_base_plate,
                pose,
            } => emit_armor_stand_model(
                &mut mesh,
                *instance,
                small,
                show_arms,
                show_base_plate,
                pose,
            ),
            EntityModelKind::Slime { size } => emit_slime_model(&mut mesh, *instance, size),
            EntityModelKind::MagmaCube { size } => {
                emit_magma_cube_model(&mut mesh, *instance, size)
            }
            EntityModelKind::Zombie { baby } => emit_zombie_model(&mut mesh, *instance, baby),
            EntityModelKind::ZombieVariant { family, baby } => {
                emit_zombie_variant_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::Piglin { family, baby } => {
                emit_piglin_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::Hoglin { family, baby } => {
                emit_hoglin_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::Ravager => emit_ravager_model(&mut mesh, *instance),
            EntityModelKind::Skeleton => emit_skeleton_model(&mut mesh, *instance),
            EntityModelKind::SkeletonVariant { family } => {
                emit_skeleton_variant_model(&mut mesh, *instance, family)
            }
            EntityModelKind::Cow { variant, baby } => {
                emit_cow_model(&mut mesh, *instance, variant, baby)
            }
            EntityModelKind::Sheep {
                baby,
                sheared,
                wool_color,
            } => {
                if !skip_texture_backed_entities {
                    emit_sheep_model(&mut mesh, *instance, baby, sheared, wool_color);
                }
            }
            EntityModelKind::Villager { baby } => emit_villager_model(&mut mesh, *instance, baby),
            EntityModelKind::WanderingTrader => emit_wandering_trader_model(&mut mesh, *instance),
            EntityModelKind::Wolf { baby, .. } => {
                if !skip_texture_backed_entities {
                    emit_wolf_model(&mut mesh, *instance, baby);
                }
            }
            EntityModelKind::Horse { baby } => emit_horse_model(&mut mesh, *instance, baby),
            EntityModelKind::Donkey {
                family,
                baby,
                has_chest,
            } => emit_donkey_model(&mut mesh, *instance, family, baby, has_chest),
            EntityModelKind::UndeadHorse { family, baby } => {
                emit_undead_horse_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::Camel { family, baby } => {
                emit_camel_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::Llama {
                family,
                variant,
                baby,
                has_chest,
            } => emit_llama_model(&mut mesh, *instance, family, variant, baby, has_chest),
            EntityModelKind::Goat {
                baby,
                left_horn,
                right_horn,
            } => emit_goat_model(&mut mesh, *instance, baby, left_horn, right_horn),
            EntityModelKind::PolarBear { baby } => {
                emit_polar_bear_model(&mut mesh, *instance, baby)
            }
            EntityModelKind::Quadruped { family, baby } => {
                emit_quadruped_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::Creeper => emit_creeper_model(&mut mesh, *instance),
            EntityModelKind::Spider => emit_spider_model(&mut mesh, *instance),
            EntityModelKind::CaveSpider => emit_cave_spider_model(&mut mesh, *instance),
            EntityModelKind::Enderman => emit_enderman_model(&mut mesh, *instance),
            EntityModelKind::IronGolem => emit_iron_golem_model(&mut mesh, *instance),
            EntityModelKind::SnowGolem => emit_snow_golem_model(&mut mesh, *instance),
            EntityModelKind::Witch => emit_witch_model(&mut mesh, *instance),
            EntityModelKind::Illager { family } => emit_illager_model(&mut mesh, *instance, family),
            EntityModelKind::Minecart => emit_minecart_model(&mut mesh, *instance),
            EntityModelKind::Boat { family, chest } => {
                emit_boat_model(&mut mesh, *instance, family, chest)
            }
            EntityModelKind::Placeholder { bounds, .. } => {
                emit_placeholder_bounds_model(&mut mesh, *instance, bounds)
            }
        }
    }
    mesh
}

fn entity_model_textured_mesh(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMesh {
    let mut mesh = EntityModelTexturedMesh::new();
    for instance in instances {
        match instance.kind {
            EntityModelKind::Sheep {
                baby,
                sheared,
                wool_color,
            } => {
                emit_sheep_textured_model(&mut mesh, *instance, baby, sheared, wool_color, atlas);
            }
            EntityModelKind::Wolf {
                baby,
                tame,
                angry,
                collar_color,
            } => {
                emit_wolf_textured_model(
                    &mut mesh,
                    *instance,
                    baby,
                    tame,
                    angry,
                    collar_color,
                    atlas,
                );
            }
            _ => {}
        }
    }
    mesh
}

fn emit_armor_stand_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    small: bool,
    show_arms: bool,
    show_base_plate: bool,
    pose: ArmorStandModelPose,
) {
    let parts = if small {
        &SMALL_ARMOR_STAND_PARTS
    } else {
        &ARMOR_STAND_PARTS
    };
    let transform = entity_model_root_transform(instance);
    emit_armor_stand_part(mesh, transform, &parts[0], degrees_to_radians3(pose.head));
    emit_armor_stand_part(mesh, transform, &parts[1], degrees_to_radians3(pose.body));
    if show_arms {
        emit_armor_stand_part(
            mesh,
            transform,
            &parts[2],
            degrees_to_radians3(pose.right_arm),
        );
        emit_armor_stand_part(
            mesh,
            transform,
            &parts[3],
            degrees_to_radians3(pose.left_arm),
        );
    }
    emit_armor_stand_part(
        mesh,
        transform,
        &parts[4],
        degrees_to_radians3(pose.right_leg),
    );
    emit_armor_stand_part(
        mesh,
        transform,
        &parts[5],
        degrees_to_radians3(pose.left_leg),
    );
    emit_armor_stand_part(mesh, transform, &parts[6], degrees_to_radians3(pose.body));
    emit_armor_stand_part(mesh, transform, &parts[7], degrees_to_radians3(pose.body));
    emit_armor_stand_part(mesh, transform, &parts[8], degrees_to_radians3(pose.body));
    if show_base_plate {
        emit_armor_stand_part(
            mesh,
            transform,
            &parts[9],
            [0.0, -instance.y_rot.to_radians(), 0.0],
        );
    }
}

fn emit_armor_stand_part(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    part: &ModelPartDesc,
    rotation: [f32; 3],
) {
    emit_model_cubes_at_pose(
        mesh,
        transform,
        PartPose {
            offset: part.pose.offset,
            rotation,
        },
        part.cubes,
    );
}

fn emit_slime_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, size: i32) {
    let size = size as f32;
    let transform = living_entity_model_root_transform_with_renderer_transform(
        instance,
        Mat4::from_scale(Vec3::splat(0.999))
            * Mat4::from_translation(Vec3::new(0.0, 0.001, 0.0))
            * Mat4::from_scale(Vec3::splat(size)),
    );
    emit_model_parts(mesh, &SLIME_PARTS, transform);
}

fn emit_magma_cube_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, size: i32) {
    let transform = living_entity_model_root_transform_with_renderer_transform(
        instance,
        Mat4::from_scale(Vec3::splat(size as f32)),
    );
    emit_model_parts(mesh, &MAGMA_CUBE_PARTS, transform);
}

fn emit_player_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, slim: bool) {
    let transform = living_entity_model_root_transform_with_renderer_transform(
        instance,
        Mat4::from_scale(Vec3::splat(AVATAR_RENDERER_SCALE)),
    );
    emit_model_parts(
        mesh,
        if slim {
            &PLAYER_SLIM_PARTS
        } else {
            &PLAYER_WIDE_PARTS
        },
        transform,
    );
}

fn emit_humanoid_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: HumanoidModelFamily,
    baby: bool,
) {
    let color = humanoid_model_color(family);
    let transform = scaled_model_root_transform(instance, if baby { 0.5 } else { 1.0 });
    emit_model_cube(
        mesh,
        transform * part_pose_transform(PART_POSE_ZERO),
        ModelCubeDesc {
            min: [-4.0, -8.0, -4.0],
            size: [8.0, 8.0, 8.0],
            color,
        },
    );
    emit_model_cube(
        mesh,
        transform * part_pose_transform(PART_POSE_ZERO),
        ModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 12.0, 4.0],
            color,
        },
    );

    let limb_width = if family == HumanoidModelFamily::Skeleton {
        2.0
    } else {
        4.0
    };
    let arm_half = limb_width / 2.0;
    for (x, min_x) in [(-5.0, -arm_half), (5.0, -arm_half)] {
        emit_model_cube(
            mesh,
            transform
                * part_pose_transform(PartPose {
                    offset: [x, 2.0, 0.0],
                    rotation: [0.0, 0.0, 0.0],
                }),
            ModelCubeDesc {
                min: [min_x, -2.0, -arm_half],
                size: [limb_width, 12.0, limb_width],
                color,
            },
        );
    }
    for (x, min_x) in [(-1.9, -arm_half), (1.9, -arm_half)] {
        emit_model_cube(
            mesh,
            transform
                * part_pose_transform(PartPose {
                    offset: [x, 12.0, 0.0],
                    rotation: [0.0, 0.0, 0.0],
                }),
            ModelCubeDesc {
                min: [min_x, 0.0, -arm_half],
                size: [limb_width, 12.0, limb_width],
                color,
            },
        );
    }

    if matches!(
        family,
        HumanoidModelFamily::Villager | HumanoidModelFamily::Illager
    ) {
        emit_model_cube(
            mesh,
            transform * part_pose_transform(PART_POSE_ZERO),
            ModelCubeDesc {
                min: [-2.0, -2.0, -6.0],
                size: [4.0, 4.0, 2.0],
                color,
            },
        );
    }
}

fn emit_zombie_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    emit_model_parts(
        mesh,
        if baby {
            &BABY_ZOMBIE_PARTS
        } else {
            &ADULT_ZOMBIE_PARTS
        },
        entity_model_root_transform(instance),
    );
}

fn emit_zombie_variant_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: ZombieVariantModelFamily,
    baby: bool,
) {
    match (family, baby) {
        (ZombieVariantModelFamily::Husk, false) => emit_model_parts_with_color(
            mesh,
            &ADULT_ZOMBIE_PARTS,
            mesh_transformer_scaled_model_root_transform(instance, HUSK_SCALE),
            HUSK_TAN,
        ),
        (ZombieVariantModelFamily::Husk, true) => emit_model_parts_with_color(
            mesh,
            &BABY_ZOMBIE_PARTS,
            entity_model_root_transform(instance),
            HUSK_TAN,
        ),
        (ZombieVariantModelFamily::Drowned, false) => emit_model_parts_with_color(
            mesh,
            &ADULT_ZOMBIE_PARTS,
            entity_model_root_transform(instance),
            DROWNED_BLUE,
        ),
        (ZombieVariantModelFamily::Drowned, true) => emit_model_parts_with_color(
            mesh,
            &BABY_ZOMBIE_PARTS,
            entity_model_root_transform(instance),
            DROWNED_BLUE,
        ),
        (ZombieVariantModelFamily::ZombieVillager, false) => emit_model_parts_with_color(
            mesh,
            &ADULT_ZOMBIE_VILLAGER_PARTS,
            entity_model_root_transform(instance),
            ZOMBIE_VILLAGER_ROBE,
        ),
        (ZombieVariantModelFamily::ZombieVillager, true) => emit_model_parts_with_color(
            mesh,
            &BABY_ZOMBIE_VILLAGER_PARTS,
            entity_model_root_transform(instance),
            ZOMBIE_VILLAGER_ROBE,
        ),
    }
}

fn emit_piglin_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: PiglinModelFamily,
    baby: bool,
) {
    let parts = if baby && family != PiglinModelFamily::PiglinBrute {
        &BABY_PIGLIN_PARTS
    } else {
        &ADULT_PIGLIN_PARTS
    };
    emit_model_parts_with_color(
        mesh,
        parts,
        entity_model_root_transform(instance),
        piglin_model_color(family),
    );
}

fn emit_hoglin_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: HoglinModelFamily,
    baby: bool,
) {
    emit_model_parts_with_color(
        mesh,
        if baby {
            &BABY_HOGLIN_PARTS
        } else {
            &ADULT_HOGLIN_PARTS
        },
        entity_model_root_transform(instance),
        hoglin_model_color(family),
    );
}

fn emit_ravager_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(mesh, &RAVAGER_PARTS, entity_model_root_transform(instance));
}

fn emit_skeleton_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(mesh, &SKELETON_PARTS, entity_model_root_transform(instance));
}

fn emit_skeleton_variant_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: SkeletonModelFamily,
) {
    match family {
        SkeletonModelFamily::Stray => {
            emit_model_parts(mesh, &SKELETON_PARTS, entity_model_root_transform(instance));
        }
        SkeletonModelFamily::Parched => {
            emit_model_parts(mesh, &PARCHED_PARTS, entity_model_root_transform(instance));
        }
        SkeletonModelFamily::Bogged { sheared } => emit_model_parts(
            mesh,
            if sheared {
                &BOGGED_SHEARED_PARTS
            } else {
                &BOGGED_PARTS
            },
            entity_model_root_transform(instance),
        ),
        SkeletonModelFamily::WitherSkeleton => emit_model_parts_with_color(
            mesh,
            &SKELETON_PARTS,
            mesh_transformer_scaled_model_root_transform(instance, WITHER_SKELETON_SCALE),
            WITHER_SKELETON_DARK,
        ),
    }
}

fn emit_cow_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    variant: CowModelVariant,
    baby: bool,
) {
    emit_model_parts(
        mesh,
        cow_model_parts(variant, baby),
        entity_model_root_transform(instance),
    );
}

fn emit_sheep_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    baby: bool,
    sheared: bool,
    wool_color: SheepWoolColor,
) {
    let transform = entity_model_root_transform(instance);
    emit_model_parts(
        mesh,
        if baby {
            &BABY_SHEEP_PARTS
        } else {
            &ADULT_SHEEP_PARTS
        },
        transform,
    );
    let wool_layer_color = sheep_wool_layer_color(wool_color);
    if !baby && wool_color != SheepWoolColor::White {
        emit_model_parts_with_color(mesh, &ADULT_SHEEP_PARTS, transform, wool_layer_color);
    }
    if !sheared {
        emit_model_parts_with_color(
            mesh,
            if baby {
                &BABY_SHEEP_PARTS
            } else {
                &ADULT_SHEEP_WOOL_PARTS
            },
            transform,
            wool_layer_color,
        );
    }
}

fn emit_sheep_textured_model(
    mesh: &mut EntityModelTexturedMesh,
    instance: EntityModelInstance,
    baby: bool,
    sheared: bool,
    wool_color: SheepWoolColor,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in sheep_textured_layer_passes(baby, sheared, wool_color) {
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        emit_textured_model_parts(
            mesh,
            pass.parts,
            transform,
            pass.texture,
            entry.uv,
            pass.tint,
        );
    }
}

fn emit_wolf_textured_model(
    mesh: &mut EntityModelTexturedMesh,
    instance: EntityModelInstance,
    baby: bool,
    tame: bool,
    angry: bool,
    collar_color: Option<EntityDyeColor>,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in wolf_textured_layer_passes(baby, tame, angry, collar_color) {
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        emit_textured_model_parts(
            mesh,
            pass.parts,
            transform,
            pass.texture,
            entry.uv,
            pass.tint,
        );
    }
}

fn sheep_textured_layer_passes(
    baby: bool,
    sheared: bool,
    wool_color: SheepWoolColor,
) -> Vec<EntityModelLayerPass> {
    let wool_tint = sheep_wool_layer_color(wool_color);
    let mut passes = Vec::with_capacity(3);
    passes.push(EntityModelLayerPass {
        kind: EntityModelLayerKind::SheepBase,
        model_layer: if baby {
            MODEL_LAYER_SHEEP_BABY
        } else {
            MODEL_LAYER_SHEEP
        },
        texture: if baby {
            SHEEP_BABY_TEXTURE_REF
        } else {
            SHEEP_TEXTURE_REF
        },
        parts: if baby {
            &BABY_SHEEP_TEXTURED_PARTS
        } else {
            &ADULT_SHEEP_TEXTURED_PARTS
        },
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    });
    if !baby && wool_color != SheepWoolColor::White {
        passes.push(EntityModelLayerPass {
            kind: EntityModelLayerKind::SheepWoolUndercoat,
            model_layer: MODEL_LAYER_SHEEP_WOOL_UNDERCOAT,
            texture: SHEEP_WOOL_UNDERCOAT_TEXTURE_REF,
            parts: &ADULT_SHEEP_TEXTURED_PARTS,
            tint: wool_tint,
            collector_order: 1,
            submit_sequence: 1,
        });
    }
    if !sheared {
        passes.push(EntityModelLayerPass {
            kind: EntityModelLayerKind::SheepWool,
            model_layer: if baby {
                MODEL_LAYER_SHEEP_BABY_WOOL
            } else {
                MODEL_LAYER_SHEEP_WOOL
            },
            texture: if baby {
                SHEEP_WOOL_BABY_TEXTURE_REF
            } else {
                SHEEP_WOOL_TEXTURE_REF
            },
            parts: if baby {
                &BABY_SHEEP_TEXTURED_PARTS
            } else {
                &ADULT_SHEEP_WOOL_TEXTURED_PARTS
            },
            tint: wool_tint,
            collector_order: if baby { 1 } else { 0 },
            submit_sequence: 2,
        });
    }
    passes.sort_by_key(|pass| (pass.collector_order, pass.submit_sequence));
    passes
}

fn wolf_textured_layer_passes(
    baby: bool,
    tame: bool,
    angry: bool,
    collar_color: Option<EntityDyeColor>,
) -> Vec<EntityModelLayerPass> {
    let parts = if baby {
        BABY_WOLF_TEXTURED_PARTS.as_slice()
    } else {
        ADULT_WOLF_TEXTURED_PARTS.as_slice()
    };
    let model_layer = if baby {
        MODEL_LAYER_WOLF_BABY
    } else {
        MODEL_LAYER_WOLF
    };
    let mut passes = Vec::with_capacity(2);
    passes.push(EntityModelLayerPass {
        kind: EntityModelLayerKind::WolfBase,
        model_layer,
        texture: wolf_texture_ref(baby, tame, angry),
        parts,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    });
    if let Some(collar_color) = tame.then_some(collar_color).flatten() {
        passes.push(EntityModelLayerPass {
            kind: EntityModelLayerKind::WolfCollar,
            model_layer,
            texture: if baby {
                WOLF_BABY_COLLAR_TEXTURE_REF
            } else {
                WOLF_COLLAR_TEXTURE_REF
            },
            parts,
            tint: collar_color.texture_diffuse_color(),
            collector_order: 1,
            submit_sequence: 1,
        });
    }
    passes
}

fn entity_model_texture_atlas_entry(
    atlas: &EntityModelTextureAtlasLayout,
    texture: EntityModelTextureRef,
) -> Option<EntityModelTextureAtlasEntry> {
    atlas
        .entries
        .iter()
        .copied()
        .find(|entry| entry.texture == texture)
}

fn emit_villager_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    if baby {
        emit_model_parts(
            mesh,
            &BABY_VILLAGER_PARTS,
            entity_model_root_transform(instance),
        );
    } else {
        emit_model_parts(
            mesh,
            &ADULT_VILLAGER_PARTS,
            villager_adult_model_root_transform(instance),
        );
    }
}

fn emit_wandering_trader_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &ADULT_VILLAGER_PARTS,
        villager_adult_model_root_transform(instance),
    );
}

fn emit_wolf_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    emit_model_parts(
        mesh,
        if baby {
            &BABY_WOLF_PARTS
        } else {
            &ADULT_WOLF_PARTS
        },
        entity_model_root_transform(instance),
    );
}

fn emit_horse_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    emit_model_parts(
        mesh,
        if baby {
            &BABY_HORSE_PARTS
        } else {
            &ADULT_HORSE_PARTS
        },
        if baby {
            entity_model_root_transform(instance)
        } else {
            mesh_transformer_scaled_model_root_transform(instance, HORSE_SCALE)
        },
    );
}

fn emit_donkey_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: DonkeyModelFamily,
    baby: bool,
    has_chest: bool,
) {
    let parts: &[ModelPartDesc] = if baby {
        &BABY_DONKEY_PARTS
    } else if has_chest {
        &ADULT_DONKEY_PARTS_WITH_CHEST
    } else {
        &ADULT_DONKEY_PARTS
    };
    let transform = if baby {
        entity_model_root_transform(instance)
    } else {
        mesh_transformer_scaled_model_root_transform(instance, donkey_model_scale(family))
    };
    emit_model_parts_with_color(mesh, parts, transform, donkey_model_color(family));
}

fn emit_undead_horse_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: UndeadHorseModelFamily,
    baby: bool,
) {
    emit_model_parts_with_color(
        mesh,
        if baby {
            &BABY_HORSE_PARTS
        } else {
            &ADULT_HORSE_PARTS
        },
        entity_model_root_transform(instance),
        undead_horse_model_color(family),
    );
}

fn emit_camel_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: CamelModelFamily,
    baby: bool,
) {
    emit_model_parts_with_color(
        mesh,
        if family == CamelModelFamily::Camel && baby {
            &BABY_CAMEL_PARTS
        } else {
            &ADULT_CAMEL_PARTS
        },
        entity_model_root_transform(instance),
        camel_model_color(family),
    );
}

fn emit_llama_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: LlamaModelFamily,
    variant: LlamaVariant,
    baby: bool,
    has_chest: bool,
) {
    let parts: &[ModelPartDesc] = if baby {
        &BABY_LLAMA_PARTS
    } else if has_chest {
        &ADULT_LLAMA_PARTS_WITH_CHEST
    } else {
        &ADULT_LLAMA_PARTS
    };
    emit_model_parts_with_color(
        mesh,
        parts,
        entity_model_root_transform(instance),
        llama_model_color(family, variant),
    );
}

fn emit_goat_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    baby: bool,
    left_horn: bool,
    right_horn: bool,
) {
    let (parts, head_index, left_horn_child_index, right_horn_child_index): (
        &[ModelPartDesc],
        usize,
        usize,
        usize,
    ) = if baby {
        (
            &BABY_GOAT_PARTS,
            BABY_GOAT_HEAD_INDEX,
            BABY_GOAT_LEFT_HORN_CHILD_INDEX,
            BABY_GOAT_RIGHT_HORN_CHILD_INDEX,
        )
    } else {
        (
            &ADULT_GOAT_PARTS,
            ADULT_GOAT_HEAD_INDEX,
            ADULT_GOAT_LEFT_HORN_CHILD_INDEX,
            ADULT_GOAT_RIGHT_HORN_CHILD_INDEX,
        )
    };
    let transform = entity_model_root_transform(instance);
    emit_goat_parts(
        mesh,
        parts,
        transform,
        head_index,
        left_horn_child_index,
        right_horn_child_index,
        left_horn,
        right_horn,
    );
}

fn emit_goat_parts(
    mesh: &mut EntityModelMesh,
    parts: &[ModelPartDesc],
    parent_transform: Mat4,
    head_index: usize,
    left_horn_child_index: usize,
    right_horn_child_index: usize,
    left_horn: bool,
    right_horn: bool,
) {
    let head = &parts[head_index];
    let head_transform = parent_transform * part_pose_transform(head.pose);
    for cube in head.cubes {
        emit_model_cube(mesh, head_transform, *cube);
    }
    for (index, child) in head.children.iter().enumerate() {
        if (index == left_horn_child_index && !left_horn)
            || (index == right_horn_child_index && !right_horn)
        {
            continue;
        }
        emit_model_part(mesh, child, head_transform);
    }
    for (index, part) in parts.iter().enumerate() {
        if index != head_index {
            emit_model_part(mesh, part, parent_transform);
        }
    }
}

fn emit_polar_bear_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    emit_model_parts(
        mesh,
        if baby {
            &BABY_POLAR_BEAR_PARTS
        } else {
            &ADULT_POLAR_BEAR_PARTS
        },
        if baby {
            entity_model_root_transform(instance)
        } else {
            mesh_transformer_scaled_model_root_transform(instance, POLAR_BEAR_SCALE)
        },
    );
}

fn emit_witch_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &WITCH_PARTS,
        villager_adult_model_root_transform(instance),
    );
}

fn emit_illager_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: IllagerModelFamily,
) {
    emit_model_parts(
        mesh,
        illager_model_parts(family),
        villager_adult_model_root_transform(instance),
    );
}

fn illager_model_parts(family: IllagerModelFamily) -> &'static [ModelPartDesc] {
    match family {
        IllagerModelFamily::Evoker | IllagerModelFamily::Vindicator => {
            &ILLAGER_SHARED_CROSSED_PARTS
        }
        IllagerModelFamily::Illusioner => &ILLAGER_ILLUSIONER_PARTS,
        IllagerModelFamily::Pillager => &ILLAGER_SHARED_UNCROSSED_PARTS,
    }
}

fn emit_quadruped_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: QuadrupedModelFamily,
    baby: bool,
) {
    if family == QuadrupedModelFamily::Pig {
        emit_pig_model(mesh, instance, PigModelVariant::Temperate, baby);
        return;
    }

    let color = quadruped_model_color(family);
    let scale = if baby { 0.5 } else { 1.0 };
    let transform = scaled_model_root_transform(instance, scale);
    let (head, body, leg_size, head_offset, body_offset, leg_x) = match family {
        QuadrupedModelFamily::Pig => (
            ([-4.0, -4.0, -8.0], [8.0, 8.0, 8.0]),
            ([-5.0, -10.0, -7.0], [10.0, 16.0, 8.0]),
            6.0,
            [0.0, 12.0, -6.0],
            [0.0, 11.0, 2.0],
            3.0,
        ),
        QuadrupedModelFamily::Cow => (
            ([-4.0, -4.0, -6.0], [8.0, 8.0, 6.0]),
            ([-6.0, -10.0, -7.0], [12.0, 18.0, 10.0]),
            12.0,
            [0.0, 4.0, -8.0],
            [0.0, 5.0, 2.0],
            4.0,
        ),
        QuadrupedModelFamily::Sheep => (
            ([-3.0, -4.0, -6.0], [6.0, 6.0, 8.0]),
            ([-4.0, -10.0, -7.0], [8.0, 16.0, 6.0]),
            12.0,
            [0.0, 6.0, -8.0],
            [0.0, 5.0, 2.0],
            3.0,
        ),
        QuadrupedModelFamily::Horse => (
            ([-3.0, -4.0, -8.0], [6.0, 5.0, 7.0]),
            ([-5.0, -8.0, -9.0], [10.0, 10.0, 22.0]),
            12.0,
            [0.0, 7.0, -10.0],
            [0.0, 11.0, 2.0],
            4.0,
        ),
        QuadrupedModelFamily::Wolf => (
            ([-3.0, -3.0, -4.0], [6.0, 6.0, 6.0]),
            ([-4.0, -2.0, -3.0], [8.0, 6.0, 9.0]),
            8.0,
            [0.0, 13.5, -7.0],
            [0.0, 14.0, 2.0],
            2.5,
        ),
    };

    emit_model_cube(
        mesh,
        transform
            * part_pose_transform(PartPose {
                offset: head_offset,
                rotation: [0.0, 0.0, 0.0],
            }),
        ModelCubeDesc {
            min: head.0,
            size: head.1,
            color,
        },
    );
    emit_model_cube(
        mesh,
        transform
            * part_pose_transform(PartPose {
                offset: body_offset,
                rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
            }),
        ModelCubeDesc {
            min: body.0,
            size: body.1,
            color,
        },
    );
    for (x, z) in [(-leg_x, 7.0), (leg_x, 7.0), (-leg_x, -5.0), (leg_x, -5.0)] {
        emit_model_cube(
            mesh,
            transform
                * part_pose_transform(PartPose {
                    offset: [x, 24.0 - leg_size, z],
                    rotation: [0.0, 0.0, 0.0],
                }),
            ModelCubeDesc {
                min: [-2.0, 0.0, -2.0],
                size: [4.0, leg_size, 4.0],
                color,
            },
        );
    }
}

fn emit_pig_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    variant: PigModelVariant,
    baby: bool,
) {
    emit_model_parts(
        mesh,
        pig_model_parts(variant, baby),
        entity_model_root_transform(instance),
    );
}

fn emit_creeper_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(mesh, &CREEPER_PARTS, entity_model_root_transform(instance));
}

fn emit_spider_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(mesh, &SPIDER_PARTS, entity_model_root_transform(instance));
}

fn emit_cave_spider_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &SPIDER_PARTS,
        mesh_transformer_scaled_model_root_transform(instance, CAVE_SPIDER_SCALE),
    );
}

fn emit_enderman_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(mesh, &ENDERMAN_PARTS, entity_model_root_transform(instance));
}

fn emit_iron_golem_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &IRON_GOLEM_PARTS,
        entity_model_root_transform(instance),
    );
}

fn emit_snow_golem_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &SNOW_GOLEM_PARTS,
        entity_model_root_transform(instance),
    );
}

fn emit_minecart_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    let transform = entity_model_root_transform(instance);
    for (min, size, pose) in [
        (
            [-10.0, -8.0, -1.0],
            [20.0, 16.0, 2.0],
            PartPose {
                offset: [0.0, 4.0, 0.0],
                rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
            },
        ),
        (
            [-8.0, -9.0, -1.0],
            [16.0, 8.0, 2.0],
            PartPose {
                offset: [-9.0, 4.0, 0.0],
                rotation: [0.0, std::f32::consts::PI * 1.5, 0.0],
            },
        ),
        (
            [-8.0, -9.0, -1.0],
            [16.0, 8.0, 2.0],
            PartPose {
                offset: [9.0, 4.0, 0.0],
                rotation: [0.0, std::f32::consts::FRAC_PI_2, 0.0],
            },
        ),
        (
            [-8.0, -9.0, -1.0],
            [16.0, 8.0, 2.0],
            PartPose {
                offset: [0.0, 4.0, -7.0],
                rotation: [0.0, std::f32::consts::PI, 0.0],
            },
        ),
        (
            [-8.0, -9.0, -1.0],
            [16.0, 8.0, 2.0],
            PartPose {
                offset: [0.0, 4.0, 7.0],
                rotation: [0.0, 0.0, 0.0],
            },
        ),
    ] {
        emit_model_cube(
            mesh,
            transform * part_pose_transform(pose),
            ModelCubeDesc {
                min,
                size,
                color: MINECART_GRAY,
            },
        );
    }
}

fn emit_boat_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: BoatModelFamily,
    chest: bool,
) {
    let transform = boat_model_root_transform(instance);
    if family == BoatModelFamily::Bamboo {
        emit_model_parts(mesh, &RAFT_COMMON_PARTS, transform);
        if chest {
            emit_model_parts(mesh, &RAFT_CHEST_PARTS, transform);
        }
    } else {
        emit_model_parts(mesh, &BOAT_COMMON_PARTS, transform);
        if chest {
            emit_model_parts(mesh, &BOAT_CHEST_PARTS, transform);
        }
    }
}

fn emit_placeholder_bounds_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    bounds: EntityModelBounds,
) {
    let width = bounds.width.max(0.0625);
    let height = bounds.height.max(0.0625);
    let depth = bounds.depth.max(0.0625);
    let transform = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_rotation_y((180.0 - instance.y_rot).to_radians());
    emit_model_cube_world_units(
        mesh,
        transform,
        [-width * 0.5, 0.0, -depth * 0.5],
        [width, height, depth],
        PLACEHOLDER_COLOR,
    );
}

fn scaled_model_root_transform(instance: EntityModelInstance, scale: f32) -> Mat4 {
    entity_model_root_transform(instance) * Mat4::from_scale(Vec3::splat(scale))
}

fn mesh_transformer_scaled_model_root_transform(instance: EntityModelInstance, scale: f32) -> Mat4 {
    entity_model_root_transform(instance)
        * part_pose_transform(PartPose {
            offset: [
                0.0,
                MESH_TRANSFORMER_ROOT_Y_OFFSET_PIXELS * (1.0 - scale),
                0.0,
            ],
            rotation: [0.0, 0.0, 0.0],
        })
        * Mat4::from_scale(Vec3::splat(scale))
}

fn villager_adult_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    mesh_transformer_scaled_model_root_transform(instance, VILLAGER_LIKE_SCALE)
}

fn humanoid_model_color(family: HumanoidModelFamily) -> [f32; 4] {
    match family {
        HumanoidModelFamily::Player => PLAYER_BLUE,
        HumanoidModelFamily::Zombie => ZOMBIE_GREEN,
        HumanoidModelFamily::Skeleton => SKELETON_BONE,
        HumanoidModelFamily::Villager => VILLAGER_ROBE,
        HumanoidModelFamily::Illager => ILLAGER_GRAY,
        HumanoidModelFamily::ArmorStand => ARMOR_STAND_WOOD,
    }
}

fn piglin_model_color(family: PiglinModelFamily) -> [f32; 4] {
    match family {
        PiglinModelFamily::Piglin => PIGLIN_SKIN,
        PiglinModelFamily::PiglinBrute => PIGLIN_BRUTE_SKIN,
        PiglinModelFamily::ZombifiedPiglin => ZOMBIFIED_PIGLIN_SKIN,
    }
}

fn hoglin_model_color(family: HoglinModelFamily) -> [f32; 4] {
    match family {
        HoglinModelFamily::Hoglin => HOGLIN_RED,
        HoglinModelFamily::Zoglin => ZOGLIN_GREEN,
    }
}

fn quadruped_model_color(family: QuadrupedModelFamily) -> [f32; 4] {
    match family {
        QuadrupedModelFamily::Pig => PIG_PINK,
        QuadrupedModelFamily::Cow => COW_BROWN,
        QuadrupedModelFamily::Sheep => SHEEP_WOOL,
        QuadrupedModelFamily::Horse => HORSE_BROWN,
        QuadrupedModelFamily::Wolf => WOLF_GRAY,
    }
}

fn donkey_model_scale(family: DonkeyModelFamily) -> f32 {
    match family {
        DonkeyModelFamily::Donkey => DONKEY_SCALE,
        DonkeyModelFamily::Mule => MULE_SCALE,
    }
}

fn donkey_model_color(family: DonkeyModelFamily) -> [f32; 4] {
    match family {
        DonkeyModelFamily::Donkey => DONKEY_GRAY,
        DonkeyModelFamily::Mule => MULE_BROWN,
    }
}

fn undead_horse_model_color(family: UndeadHorseModelFamily) -> [f32; 4] {
    match family {
        UndeadHorseModelFamily::Skeleton => SKELETON_HORSE_BONE,
        UndeadHorseModelFamily::Zombie => ZOMBIE_HORSE_GREEN,
    }
}

fn camel_model_color(family: CamelModelFamily) -> [f32; 4] {
    match family {
        CamelModelFamily::Camel => CAMEL_TAN,
        CamelModelFamily::CamelHusk => CAMEL_HUSK_BROWN,
    }
}

fn llama_model_color(_family: LlamaModelFamily, variant: LlamaVariant) -> [f32; 4] {
    match variant {
        LlamaVariant::Creamy => LLAMA_CREAMY,
        LlamaVariant::White => LLAMA_WHITE,
        LlamaVariant::Brown => LLAMA_BROWN,
        LlamaVariant::Gray => LLAMA_GRAY,
    }
}

fn chicken_model_parts(variant: ChickenModelVariant, baby: bool) -> &'static [ModelPartDesc] {
    match (variant, baby) {
        (_, true) => &BABY_CHICKEN_PARTS,
        (ChickenModelVariant::Cold, false) => &COLD_CHICKEN_PARTS,
        (_, false) => &ADULT_CHICKEN_PARTS,
    }
}

fn chicken_model_key(variant: ChickenModelVariant, baby: bool) -> &'static str {
    match (variant, baby) {
        (ChickenModelVariant::Temperate, false) => "chicken_temperate",
        (ChickenModelVariant::Temperate, true) => "chicken_temperate_baby",
        (ChickenModelVariant::Warm, false) => "chicken_warm",
        (ChickenModelVariant::Warm, true) => "chicken_warm_baby",
        (ChickenModelVariant::Cold, false) => "chicken_cold",
        (ChickenModelVariant::Cold, true) => "chicken_cold_baby",
    }
}

fn chicken_texture_ref(variant: ChickenModelVariant, baby: bool) -> EntityModelTextureRef {
    match (variant, baby) {
        (ChickenModelVariant::Temperate, false) => EntityModelTextureRef {
            path: "textures/entity/chicken/chicken_temperate.png",
            size: [64, 32],
        },
        (ChickenModelVariant::Temperate, true) => EntityModelTextureRef {
            path: "textures/entity/chicken/chicken_temperate_baby.png",
            size: [16, 16],
        },
        (ChickenModelVariant::Warm, false) => EntityModelTextureRef {
            path: "textures/entity/chicken/chicken_warm.png",
            size: [64, 32],
        },
        (ChickenModelVariant::Warm, true) => EntityModelTextureRef {
            path: "textures/entity/chicken/chicken_warm_baby.png",
            size: [16, 16],
        },
        (ChickenModelVariant::Cold, false) => EntityModelTextureRef {
            path: "textures/entity/chicken/chicken_cold.png",
            size: [64, 32],
        },
        (ChickenModelVariant::Cold, true) => EntityModelTextureRef {
            path: "textures/entity/chicken/chicken_cold_baby.png",
            size: [16, 16],
        },
    }
}

fn pig_model_parts(variant: PigModelVariant, baby: bool) -> &'static [ModelPartDesc] {
    match (variant, baby) {
        (_, true) => &BABY_PIG_PARTS,
        (PigModelVariant::Cold, false) => &COLD_PIG_PARTS,
        (_, false) => &ADULT_PIG_PARTS,
    }
}

fn pig_model_key(variant: PigModelVariant, baby: bool) -> &'static str {
    match (variant, baby) {
        (PigModelVariant::Temperate, false) => "pig_temperate",
        (PigModelVariant::Temperate, true) => "pig_temperate_baby",
        (PigModelVariant::Warm, false) => "pig_warm",
        (PigModelVariant::Warm, true) => "pig_warm_baby",
        (PigModelVariant::Cold, false) => "pig_cold",
        (PigModelVariant::Cold, true) => "pig_cold_baby",
    }
}

fn pig_texture_ref(variant: PigModelVariant, baby: bool) -> EntityModelTextureRef {
    match (variant, baby) {
        (PigModelVariant::Temperate, false) => EntityModelTextureRef {
            path: "textures/entity/pig/pig_temperate.png",
            size: [64, 64],
        },
        (PigModelVariant::Temperate, true) => EntityModelTextureRef {
            path: "textures/entity/pig/pig_temperate_baby.png",
            size: [32, 32],
        },
        (PigModelVariant::Warm, false) => EntityModelTextureRef {
            path: "textures/entity/pig/pig_warm.png",
            size: [64, 64],
        },
        (PigModelVariant::Warm, true) => EntityModelTextureRef {
            path: "textures/entity/pig/pig_warm_baby.png",
            size: [32, 32],
        },
        (PigModelVariant::Cold, false) => EntityModelTextureRef {
            path: "textures/entity/pig/pig_cold.png",
            size: [64, 64],
        },
        (PigModelVariant::Cold, true) => EntityModelTextureRef {
            path: "textures/entity/pig/pig_cold_baby.png",
            size: [32, 32],
        },
    }
}

fn cow_model_parts(variant: CowModelVariant, baby: bool) -> &'static [ModelPartDesc] {
    match (variant, baby) {
        (_, true) => &BABY_COW_PARTS,
        (CowModelVariant::Warm, false) => &WARM_COW_PARTS,
        (CowModelVariant::Cold, false) => &COLD_COW_PARTS,
        (CowModelVariant::Temperate, false) => &ADULT_COW_PARTS,
    }
}

fn cow_model_key(variant: CowModelVariant, baby: bool) -> &'static str {
    match (variant, baby) {
        (CowModelVariant::Temperate, false) => "cow_temperate",
        (CowModelVariant::Temperate, true) => "cow_temperate_baby",
        (CowModelVariant::Warm, false) => "cow_warm",
        (CowModelVariant::Warm, true) => "cow_warm_baby",
        (CowModelVariant::Cold, false) => "cow_cold",
        (CowModelVariant::Cold, true) => "cow_cold_baby",
    }
}

fn cow_texture_ref(variant: CowModelVariant, baby: bool) -> EntityModelTextureRef {
    match (variant, baby) {
        (CowModelVariant::Temperate, false) => EntityModelTextureRef {
            path: "textures/entity/cow/cow_temperate.png",
            size: [64, 64],
        },
        (CowModelVariant::Temperate, true) => EntityModelTextureRef {
            path: "textures/entity/cow/cow_temperate_baby.png",
            size: [64, 64],
        },
        (CowModelVariant::Warm, false) => EntityModelTextureRef {
            path: "textures/entity/cow/cow_warm.png",
            size: [64, 64],
        },
        (CowModelVariant::Warm, true) => EntityModelTextureRef {
            path: "textures/entity/cow/cow_warm_baby.png",
            size: [64, 64],
        },
        (CowModelVariant::Cold, false) => EntityModelTextureRef {
            path: "textures/entity/cow/cow_cold.png",
            size: [64, 64],
        },
        (CowModelVariant::Cold, true) => EntityModelTextureRef {
            path: "textures/entity/cow/cow_cold_baby.png",
            size: [64, 64],
        },
    }
}

const SHEEP_WOOL_COLOR_MODEL_KEYS: [&str; 16] = [
    "sheep",
    "sheep_orange",
    "sheep_magenta",
    "sheep_light_blue",
    "sheep_yellow",
    "sheep_lime",
    "sheep_pink",
    "sheep_gray",
    "sheep_light_gray",
    "sheep_cyan",
    "sheep_purple",
    "sheep_blue",
    "sheep_brown",
    "sheep_green",
    "sheep_red",
    "sheep_black",
];

const BABY_SHEEP_WOOL_COLOR_MODEL_KEYS: [&str; 16] = [
    "sheep_baby",
    "sheep_baby_orange",
    "sheep_baby_magenta",
    "sheep_baby_light_blue",
    "sheep_baby_yellow",
    "sheep_baby_lime",
    "sheep_baby_pink",
    "sheep_baby_gray",
    "sheep_baby_light_gray",
    "sheep_baby_cyan",
    "sheep_baby_purple",
    "sheep_baby_blue",
    "sheep_baby_brown",
    "sheep_baby_green",
    "sheep_baby_red",
    "sheep_baby_black",
];

const SHEEP_SHEARED_COLOR_MODEL_KEYS: [&str; 16] = [
    "sheep_sheared",
    "sheep_orange_sheared",
    "sheep_magenta_sheared",
    "sheep_light_blue_sheared",
    "sheep_yellow_sheared",
    "sheep_lime_sheared",
    "sheep_pink_sheared",
    "sheep_gray_sheared",
    "sheep_light_gray_sheared",
    "sheep_cyan_sheared",
    "sheep_purple_sheared",
    "sheep_blue_sheared",
    "sheep_brown_sheared",
    "sheep_green_sheared",
    "sheep_red_sheared",
    "sheep_black_sheared",
];

const SHEEP_WOOL_LAYER_COLOR_BYTES: [[u8; 3]; 16] = [
    [230, 230, 230],
    [186, 96, 21],
    [149, 58, 141],
    [43, 134, 163],
    [190, 162, 45],
    [96, 149, 23],
    [182, 104, 127],
    [53, 59, 61],
    [117, 117, 113],
    [16, 117, 117],
    [102, 37, 138],
    [45, 51, 127],
    [98, 63, 37],
    [70, 93, 16],
    [132, 34, 28],
    [21, 21, 24],
];

fn sheep_model_key(baby: bool, sheared: bool, wool_color: SheepWoolColor) -> &'static str {
    let color = wool_color.vanilla_id() as usize;
    match (baby, sheared) {
        (false, false) => SHEEP_WOOL_COLOR_MODEL_KEYS[color],
        (true, false) => BABY_SHEEP_WOOL_COLOR_MODEL_KEYS[color],
        (false, true) => SHEEP_SHEARED_COLOR_MODEL_KEYS[color],
        (true, true) => "sheep_baby_sheared",
    }
}

fn sheep_wool_layer_color(wool_color: SheepWoolColor) -> [f32; 4] {
    let [red, green, blue] = SHEEP_WOOL_LAYER_COLOR_BYTES[wool_color.vanilla_id() as usize];
    [
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        1.0,
    ]
}

fn wolf_model_key(baby: bool, tame: bool, angry: bool) -> &'static str {
    match (baby, tame, angry) {
        (false, true, _) => "wolf_tame",
        (false, false, true) => "wolf_angry",
        (false, false, false) => "wolf",
        (true, true, _) => "wolf_tame_baby",
        (true, false, true) => "wolf_angry_baby",
        (true, false, false) => "wolf_baby",
    }
}

fn wolf_texture_ref(baby: bool, tame: bool, angry: bool) -> EntityModelTextureRef {
    match (baby, tame, angry) {
        (false, true, _) => WOLF_TAME_TEXTURE_REF,
        (false, false, true) => WOLF_ANGRY_TEXTURE_REF,
        (false, false, false) => WOLF_TEXTURE_REF,
        (true, true, _) => WOLF_TAME_BABY_TEXTURE_REF,
        (true, false, true) => WOLF_ANGRY_BABY_TEXTURE_REF,
        (true, false, false) => WOLF_BABY_TEXTURE_REF,
    }
}

fn llama_model_key(family: LlamaModelFamily, variant: LlamaVariant, baby: bool) -> &'static str {
    match (family, variant, baby) {
        (LlamaModelFamily::Llama, LlamaVariant::Creamy, false) => "llama_creamy",
        (LlamaModelFamily::Llama, LlamaVariant::Creamy, true) => "llama_creamy_baby",
        (LlamaModelFamily::Llama, LlamaVariant::White, false) => "llama_white",
        (LlamaModelFamily::Llama, LlamaVariant::White, true) => "llama_white_baby",
        (LlamaModelFamily::Llama, LlamaVariant::Brown, false) => "llama_brown",
        (LlamaModelFamily::Llama, LlamaVariant::Brown, true) => "llama_brown_baby",
        (LlamaModelFamily::Llama, LlamaVariant::Gray, false) => "llama_gray",
        (LlamaModelFamily::Llama, LlamaVariant::Gray, true) => "llama_gray_baby",
        (LlamaModelFamily::TraderLlama, LlamaVariant::Creamy, false) => "trader_llama_creamy",
        (LlamaModelFamily::TraderLlama, LlamaVariant::Creamy, true) => "trader_llama_creamy_baby",
        (LlamaModelFamily::TraderLlama, LlamaVariant::White, false) => "trader_llama_white",
        (LlamaModelFamily::TraderLlama, LlamaVariant::White, true) => "trader_llama_white_baby",
        (LlamaModelFamily::TraderLlama, LlamaVariant::Brown, false) => "trader_llama_brown",
        (LlamaModelFamily::TraderLlama, LlamaVariant::Brown, true) => "trader_llama_brown_baby",
        (LlamaModelFamily::TraderLlama, LlamaVariant::Gray, false) => "trader_llama_gray",
        (LlamaModelFamily::TraderLlama, LlamaVariant::Gray, true) => "trader_llama_gray_baby",
    }
}

fn llama_texture_ref(variant: LlamaVariant, baby: bool) -> EntityModelTextureRef {
    match (variant, baby) {
        (LlamaVariant::Creamy, false) => LLAMA_CREAMY_TEXTURE_REF,
        (LlamaVariant::Creamy, true) => LLAMA_CREAMY_BABY_TEXTURE_REF,
        (LlamaVariant::White, false) => LLAMA_WHITE_TEXTURE_REF,
        (LlamaVariant::White, true) => LLAMA_WHITE_BABY_TEXTURE_REF,
        (LlamaVariant::Brown, false) => LLAMA_BROWN_TEXTURE_REF,
        (LlamaVariant::Brown, true) => LLAMA_BROWN_BABY_TEXTURE_REF,
        (LlamaVariant::Gray, false) => LLAMA_GRAY_TEXTURE_REF,
        (LlamaVariant::Gray, true) => LLAMA_GRAY_BABY_TEXTURE_REF,
    }
}

fn boat_model_key(family: BoatModelFamily, chest: bool) -> &'static str {
    match (family, chest) {
        (BoatModelFamily::Acacia, false) => "boat_acacia",
        (BoatModelFamily::Acacia, true) => "chest_boat_acacia",
        (BoatModelFamily::Bamboo, false) => "boat_bamboo",
        (BoatModelFamily::Bamboo, true) => "chest_boat_bamboo",
        (BoatModelFamily::Birch, false) => "boat_birch",
        (BoatModelFamily::Birch, true) => "chest_boat_birch",
        (BoatModelFamily::Cherry, false) => "boat_cherry",
        (BoatModelFamily::Cherry, true) => "chest_boat_cherry",
        (BoatModelFamily::DarkOak, false) => "boat_dark_oak",
        (BoatModelFamily::DarkOak, true) => "chest_boat_dark_oak",
        (BoatModelFamily::Jungle, false) => "boat_jungle",
        (BoatModelFamily::Jungle, true) => "chest_boat_jungle",
        (BoatModelFamily::Mangrove, false) => "boat_mangrove",
        (BoatModelFamily::Mangrove, true) => "chest_boat_mangrove",
        (BoatModelFamily::Oak, false) => "boat_oak",
        (BoatModelFamily::Oak, true) => "chest_boat_oak",
        (BoatModelFamily::PaleOak, false) => "boat_pale_oak",
        (BoatModelFamily::PaleOak, true) => "chest_boat_pale_oak",
        (BoatModelFamily::Spruce, false) => "boat_spruce",
        (BoatModelFamily::Spruce, true) => "chest_boat_spruce",
    }
}

fn boat_texture_ref(family: BoatModelFamily, chest: bool) -> EntityModelTextureRef {
    match (family, chest) {
        (BoatModelFamily::Acacia, false) => EntityModelTextureRef {
            path: "textures/entity/boat/acacia.png",
            size: [128, 64],
        },
        (BoatModelFamily::Acacia, true) => EntityModelTextureRef {
            path: "textures/entity/chest_boat/acacia.png",
            size: [128, 128],
        },
        (BoatModelFamily::Bamboo, false) => EntityModelTextureRef {
            path: "textures/entity/boat/bamboo.png",
            size: [128, 64],
        },
        (BoatModelFamily::Bamboo, true) => EntityModelTextureRef {
            path: "textures/entity/chest_boat/bamboo.png",
            size: [128, 128],
        },
        (BoatModelFamily::Birch, false) => EntityModelTextureRef {
            path: "textures/entity/boat/birch.png",
            size: [128, 64],
        },
        (BoatModelFamily::Birch, true) => EntityModelTextureRef {
            path: "textures/entity/chest_boat/birch.png",
            size: [128, 128],
        },
        (BoatModelFamily::Cherry, false) => EntityModelTextureRef {
            path: "textures/entity/boat/cherry.png",
            size: [128, 64],
        },
        (BoatModelFamily::Cherry, true) => EntityModelTextureRef {
            path: "textures/entity/chest_boat/cherry.png",
            size: [128, 128],
        },
        (BoatModelFamily::DarkOak, false) => EntityModelTextureRef {
            path: "textures/entity/boat/dark_oak.png",
            size: [128, 64],
        },
        (BoatModelFamily::DarkOak, true) => EntityModelTextureRef {
            path: "textures/entity/chest_boat/dark_oak.png",
            size: [128, 128],
        },
        (BoatModelFamily::Jungle, false) => EntityModelTextureRef {
            path: "textures/entity/boat/jungle.png",
            size: [128, 64],
        },
        (BoatModelFamily::Jungle, true) => EntityModelTextureRef {
            path: "textures/entity/chest_boat/jungle.png",
            size: [128, 128],
        },
        (BoatModelFamily::Mangrove, false) => EntityModelTextureRef {
            path: "textures/entity/boat/mangrove.png",
            size: [128, 64],
        },
        (BoatModelFamily::Mangrove, true) => EntityModelTextureRef {
            path: "textures/entity/chest_boat/mangrove.png",
            size: [128, 128],
        },
        (BoatModelFamily::Oak, false) => EntityModelTextureRef {
            path: "textures/entity/boat/oak.png",
            size: [128, 64],
        },
        (BoatModelFamily::Oak, true) => EntityModelTextureRef {
            path: "textures/entity/chest_boat/oak.png",
            size: [128, 128],
        },
        (BoatModelFamily::PaleOak, false) => EntityModelTextureRef {
            path: "textures/entity/boat/pale_oak.png",
            size: [128, 64],
        },
        (BoatModelFamily::PaleOak, true) => EntityModelTextureRef {
            path: "textures/entity/chest_boat/pale_oak.png",
            size: [128, 128],
        },
        (BoatModelFamily::Spruce, false) => EntityModelTextureRef {
            path: "textures/entity/boat/spruce.png",
            size: [128, 64],
        },
        (BoatModelFamily::Spruce, true) => EntityModelTextureRef {
            path: "textures/entity/chest_boat/spruce.png",
            size: [128, 128],
        },
    }
}

fn emit_model_parts(mesh: &mut EntityModelMesh, parts: &[ModelPartDesc], parent_transform: Mat4) {
    for part in parts {
        emit_model_part(mesh, part, parent_transform);
    }
}

fn emit_model_parts_with_color(
    mesh: &mut EntityModelMesh,
    parts: &[ModelPartDesc],
    parent_transform: Mat4,
    color: [f32; 4],
) {
    for part in parts {
        emit_model_part_with_color(mesh, part, parent_transform, color);
    }
}

fn emit_textured_model_parts(
    mesh: &mut EntityModelTexturedMesh,
    parts: &[TexturedModelPartDesc],
    parent_transform: Mat4,
    texture: EntityModelTextureRef,
    uv_rect: EntityModelUvRect,
    tint: [f32; 4],
) {
    for part in parts {
        emit_textured_model_part(mesh, part, parent_transform, texture, uv_rect, tint);
    }
}

fn emit_model_cubes_at_pose(
    mesh: &mut EntityModelMesh,
    parent_transform: Mat4,
    pose: PartPose,
    cubes: &[ModelCubeDesc],
) {
    let transform = parent_transform * part_pose_transform(pose);
    for cube in cubes {
        emit_model_cube(mesh, transform, *cube);
    }
}

fn emit_model_part(mesh: &mut EntityModelMesh, part: &ModelPartDesc, parent_transform: Mat4) {
    let transform = parent_transform * part_pose_transform(part.pose);
    for cube in part.cubes {
        emit_model_cube(mesh, transform, *cube);
    }
    emit_model_parts(mesh, part.children, transform);
}

fn emit_textured_model_part(
    mesh: &mut EntityModelTexturedMesh,
    part: &TexturedModelPartDesc,
    parent_transform: Mat4,
    texture: EntityModelTextureRef,
    uv_rect: EntityModelUvRect,
    tint: [f32; 4],
) {
    let transform = parent_transform * part_pose_transform(part.pose);
    for cube in part.cubes {
        emit_textured_model_cube(mesh, transform, *cube, texture, uv_rect, tint);
    }
    emit_textured_model_parts(mesh, part.children, transform, texture, uv_rect, tint);
}

fn emit_model_part_with_color(
    mesh: &mut EntityModelMesh,
    part: &ModelPartDesc,
    parent_transform: Mat4,
    color: [f32; 4],
) {
    let transform = parent_transform * part_pose_transform(part.pose);
    for cube in part.cubes {
        emit_model_cube_with_color(mesh, transform, *cube, color);
    }
    emit_model_parts_with_color(mesh, part.children, transform, color);
}

fn emit_model_cube_with_color(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    cube: ModelCubeDesc,
    color: [f32; 4],
) {
    emit_model_cube(
        mesh,
        transform,
        ModelCubeDesc {
            min: cube.min,
            size: cube.size,
            color,
        },
    );
}

fn entity_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_rotation_y((180.0 - instance.y_rot).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

fn living_entity_model_root_transform_with_renderer_transform(
    instance: EntityModelInstance,
    renderer_transform: Mat4,
) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_rotation_y((180.0 - instance.y_rot).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * renderer_transform
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

fn boat_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_translation(Vec3::new(0.0, 0.375, 0.0))
        * Mat4::from_rotation_y((180.0 - instance.y_rot).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2)
}

fn part_pose_transform(pose: PartPose) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(pose.offset) * MODEL_UNIT_SCALE)
        * Mat4::from_euler(
            EulerRot::ZYX,
            pose.rotation[2],
            pose.rotation[1],
            pose.rotation[0],
        )
}

fn degrees_to_radians3(rotation: [f32; 3]) -> [f32; 3] {
    [
        rotation[0].to_radians(),
        rotation[1].to_radians(),
        rotation[2].to_radians(),
    ]
}

fn emit_model_cube(mesh: &mut EntityModelMesh, transform: Mat4, cube: ModelCubeDesc) {
    let min = Vec3::from_array(cube.min) * MODEL_UNIT_SCALE;
    let max = min + Vec3::from_array(cube.size) * MODEL_UNIT_SCALE;
    emit_model_cube_from_min_max(mesh, transform, min, max, cube.color);
}

fn emit_textured_model_cube(
    mesh: &mut EntityModelTexturedMesh,
    transform: Mat4,
    cube: TexturedModelCubeDesc,
    texture: EntityModelTextureRef,
    uv_rect: EntityModelUvRect,
    tint: [f32; 4],
) {
    let mut min = Vec3::from_array(cube.min) * MODEL_UNIT_SCALE;
    let mut max = min + Vec3::from_array(cube.size) * MODEL_UNIT_SCALE;
    if cube.mirror {
        std::mem::swap(&mut min.x, &mut max.x);
    }
    emit_textured_model_cube_from_min_max(mesh, transform, min, max, cube, texture, uv_rect, tint);
}

fn emit_model_cube_world_units(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    min: [f32; 3],
    size: [f32; 3],
    color: [f32; 4],
) {
    let min = Vec3::from_array(min);
    let max = min + Vec3::from_array(size);
    emit_model_cube_from_min_max(mesh, transform, min, max, color);
}

fn emit_model_cube_from_min_max(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    min: Vec3,
    max: Vec3,
    color: [f32; 4],
) {
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ];
    let faces = [
        ([4, 0, 1, 5], 0.56),
        ([2, 3, 7, 6], 1.0),
        ([0, 3, 2, 1], 0.78),
        ([5, 6, 7, 4], 0.86),
        ([0, 4, 7, 3], 0.68),
        ([1, 2, 6, 5], 0.68),
    ];

    for (face, shade) in faces {
        emit_model_face(
            mesh,
            face.map(|index| transform.transform_point3(corners[index])),
            shade_color(color, shade),
        );
    }
}

fn emit_textured_model_cube_from_min_max(
    mesh: &mut EntityModelTexturedMesh,
    transform: Mat4,
    min: Vec3,
    max: Vec3,
    cube: TexturedModelCubeDesc,
    texture: EntityModelTextureRef,
    uv_rect: EntityModelUvRect,
    tint: [f32; 4],
) {
    let t0 = Vec3::new(min.x, min.y, min.z);
    let t1 = Vec3::new(max.x, min.y, min.z);
    let t2 = Vec3::new(max.x, max.y, min.z);
    let t3 = Vec3::new(min.x, max.y, min.z);
    let l0 = Vec3::new(min.x, min.y, max.z);
    let l1 = Vec3::new(max.x, min.y, max.z);
    let l2 = Vec3::new(max.x, max.y, max.z);
    let l3 = Vec3::new(min.x, max.y, max.z);

    let width = cube.uv_size[0];
    let height = cube.uv_size[1];
    let depth = cube.uv_size[2];
    let x_tex = cube.tex[0];
    let y_tex = cube.tex[1];
    let u0 = x_tex;
    let u1 = x_tex + depth;
    let u2 = x_tex + depth + width;
    let u22 = x_tex + depth + width + width;
    let u3 = x_tex + depth + width + depth;
    let u4 = x_tex + depth + width + depth + width;
    let v0 = y_tex;
    let v1 = y_tex + depth;
    let v2 = y_tex + depth + height;

    emit_textured_model_polygon(
        mesh,
        [l1, l0, t0, t1].map(|corner| transform.transform_point3(corner)),
        [u1, v0, u2, v1],
        texture,
        uv_rect,
        tint,
        cube.mirror,
    );
    emit_textured_model_polygon(
        mesh,
        [t2, t3, l3, l2].map(|corner| transform.transform_point3(corner)),
        [u2, v1, u22, v0],
        texture,
        uv_rect,
        tint,
        cube.mirror,
    );
    emit_textured_model_polygon(
        mesh,
        [t0, l0, l3, t3].map(|corner| transform.transform_point3(corner)),
        [u0, v1, u1, v2],
        texture,
        uv_rect,
        tint,
        cube.mirror,
    );
    emit_textured_model_polygon(
        mesh,
        [t1, t0, t3, t2].map(|corner| transform.transform_point3(corner)),
        [u1, v1, u2, v2],
        texture,
        uv_rect,
        tint,
        cube.mirror,
    );
    emit_textured_model_polygon(
        mesh,
        [l1, t1, t2, l2].map(|corner| transform.transform_point3(corner)),
        [u2, v1, u3, v2],
        texture,
        uv_rect,
        tint,
        cube.mirror,
    );
    emit_textured_model_polygon(
        mesh,
        [l0, l1, l2, l3].map(|corner| transform.transform_point3(corner)),
        [u3, v1, u4, v2],
        texture,
        uv_rect,
        tint,
        cube.mirror,
    );
}

fn emit_textured_model_polygon(
    mesh: &mut EntityModelTexturedMesh,
    corners: [Vec3; 4],
    texture_uv: [f32; 4],
    texture: EntityModelTextureRef,
    uv_rect: EntityModelUvRect,
    tint: [f32; 4],
    mirror: bool,
) {
    let [u0, v0, u1, v1] = texture_uv;
    let source_uv = [[u1, v0], [u0, v0], [u0, v1], [u1, v1]];
    let mut vertices = [
        (corners[0], source_uv[0]),
        (corners[1], source_uv[1]),
        (corners[2], source_uv[2]),
        (corners[3], source_uv[3]),
    ];
    if mirror {
        vertices.reverse();
    }
    let base = mesh.vertices.len() as u32;
    mesh.vertices
        .extend(vertices.map(|(position, uv)| EntityModelTexturedVertex {
            position: position.to_array(),
            uv: atlas_uv(uv, texture, uv_rect),
            tint,
        }));
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.cutout_faces += 1;
}

fn atlas_uv(
    texture_uv: [f32; 2],
    texture: EntityModelTextureRef,
    rect: EntityModelUvRect,
) -> [f32; 2] {
    let source = [
        texture_uv[0] / texture.size[0] as f32,
        texture_uv[1] / texture.size[1] as f32,
    ];
    [
        rect.min[0] + source[0] * (rect.max[0] - rect.min[0]),
        rect.min[1] + source[1] * (rect.max[1] - rect.min[1]),
    ]
}

fn emit_model_face(mesh: &mut EntityModelMesh, corners: [Vec3; 4], color: [f32; 4]) {
    let base = mesh.vertices.len() as u32;
    mesh.vertices
        .extend(corners.map(|position| EntityModelVertex {
            position: position.to_array(),
            color,
        }));
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.opaque_faces += 1;
}

fn shade_color(color: [f32; 4], shade: f32) -> [f32; 4] {
    [
        (color[0] * shade).clamp(0.0, 1.0),
        (color[1] * shade).clamp(0.0, 1.0),
        (color[2] * shade).clamp(0.0, 1.0),
        color[3].clamp(0.0, 1.0),
    ]
}

fn entity_model_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<EntityModelVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ENTITY_MODEL_VERTEX_ATTRIBUTES,
    }
}

fn entity_model_textured_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<EntityModelTexturedVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES,
    }
}

#[cfg(test)]
mod tests;
