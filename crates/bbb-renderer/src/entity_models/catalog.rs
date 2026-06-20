use super::model_layers::*;

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
        parts: PlayerModelPartVisibility,
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
pub struct PlayerModelPartVisibility {
    pub cape: bool,
    pub jacket: bool,
    pub left_sleeve: bool,
    pub right_sleeve: bool,
    pub left_pants: bool,
    pub right_pants: bool,
    pub hat: bool,
}

impl PlayerModelPartVisibility {
    pub const CAPE_MASK: u8 = 1 << 0;
    pub const JACKET_MASK: u8 = 1 << 1;
    pub const LEFT_SLEEVE_MASK: u8 = 1 << 2;
    pub const RIGHT_SLEEVE_MASK: u8 = 1 << 3;
    pub const LEFT_PANTS_MASK: u8 = 1 << 4;
    pub const RIGHT_PANTS_MASK: u8 = 1 << 5;
    pub const HAT_MASK: u8 = 1 << 6;
    pub const ALL_MASK: u8 = Self::CAPE_MASK
        | Self::JACKET_MASK
        | Self::LEFT_SLEEVE_MASK
        | Self::RIGHT_SLEEVE_MASK
        | Self::LEFT_PANTS_MASK
        | Self::RIGHT_PANTS_MASK
        | Self::HAT_MASK;

    pub const fn from_vanilla_mask(mask: u8) -> Self {
        Self {
            cape: (mask & Self::CAPE_MASK) == Self::CAPE_MASK,
            jacket: (mask & Self::JACKET_MASK) == Self::JACKET_MASK,
            left_sleeve: (mask & Self::LEFT_SLEEVE_MASK) == Self::LEFT_SLEEVE_MASK,
            right_sleeve: (mask & Self::RIGHT_SLEEVE_MASK) == Self::RIGHT_SLEEVE_MASK,
            left_pants: (mask & Self::LEFT_PANTS_MASK) == Self::LEFT_PANTS_MASK,
            right_pants: (mask & Self::RIGHT_PANTS_MASK) == Self::RIGHT_PANTS_MASK,
            hat: (mask & Self::HAT_MASK) == Self::HAT_MASK,
        }
    }

    pub const fn vanilla_mask(self) -> u8 {
        (if self.cape { Self::CAPE_MASK } else { 0 })
            | (if self.jacket { Self::JACKET_MASK } else { 0 })
            | (if self.left_sleeve {
                Self::LEFT_SLEEVE_MASK
            } else {
                0
            })
            | (if self.right_sleeve {
                Self::RIGHT_SLEEVE_MASK
            } else {
                0
            })
            | (if self.left_pants {
                Self::LEFT_PANTS_MASK
            } else {
                0
            })
            | (if self.right_pants {
                Self::RIGHT_PANTS_MASK
            } else {
                0
            })
            | (if self.hat { Self::HAT_MASK } else { 0 })
    }
}

pub const PLAYER_MODEL_PARTS_ALL_VISIBLE: PlayerModelPartVisibility =
    PlayerModelPartVisibility::from_vanilla_mask(PlayerModelPartVisibility::ALL_MASK);
pub const PLAYER_MODEL_PARTS_ALL_HIDDEN: PlayerModelPartVisibility =
    PlayerModelPartVisibility::from_vanilla_mask(0);

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
            Self::Player { slim: false, .. } => "player",
            Self::Player { slim: true, .. } => "player_slim",
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
            Self::Player { slim, .. } => Some(player_texture_ref(slim)),
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
            Self::Spider | Self::CaveSpider => &SPIDER_EYES_LAYER_TEXTURE_REFS,
            Self::Enderman => &ENDERMAN_EYES_LAYER_TEXTURE_REFS,
            _ => &[],
        }
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

pub(super) fn chicken_texture_ref(
    variant: ChickenModelVariant,
    baby: bool,
) -> EntityModelTextureRef {
    match (variant, baby) {
        (ChickenModelVariant::Temperate, false) => CHICKEN_TEMPERATE_TEXTURE_REF,
        (ChickenModelVariant::Temperate, true) => CHICKEN_TEMPERATE_BABY_TEXTURE_REF,
        (ChickenModelVariant::Warm, false) => CHICKEN_WARM_TEXTURE_REF,
        (ChickenModelVariant::Warm, true) => CHICKEN_WARM_BABY_TEXTURE_REF,
        (ChickenModelVariant::Cold, false) => CHICKEN_COLD_TEXTURE_REF,
        (ChickenModelVariant::Cold, true) => CHICKEN_COLD_BABY_TEXTURE_REF,
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

pub(super) fn pig_texture_ref(variant: PigModelVariant, baby: bool) -> EntityModelTextureRef {
    match (variant, baby) {
        (PigModelVariant::Temperate, false) => PIG_TEMPERATE_TEXTURE_REF,
        (PigModelVariant::Temperate, true) => PIG_TEMPERATE_BABY_TEXTURE_REF,
        (PigModelVariant::Warm, false) => PIG_WARM_TEXTURE_REF,
        (PigModelVariant::Warm, true) => PIG_WARM_BABY_TEXTURE_REF,
        (PigModelVariant::Cold, false) => PIG_COLD_TEXTURE_REF,
        (PigModelVariant::Cold, true) => PIG_COLD_BABY_TEXTURE_REF,
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

pub(super) fn cow_texture_ref(variant: CowModelVariant, baby: bool) -> EntityModelTextureRef {
    match (variant, baby) {
        (CowModelVariant::Temperate, false) => COW_TEMPERATE_TEXTURE_REF,
        (CowModelVariant::Temperate, true) => COW_TEMPERATE_BABY_TEXTURE_REF,
        (CowModelVariant::Warm, false) => COW_WARM_TEXTURE_REF,
        (CowModelVariant::Warm, true) => COW_WARM_BABY_TEXTURE_REF,
        (CowModelVariant::Cold, false) => COW_COLD_TEXTURE_REF,
        (CowModelVariant::Cold, true) => COW_COLD_BABY_TEXTURE_REF,
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

pub(super) fn sheep_wool_layer_color(wool_color: SheepWoolColor) -> [f32; 4] {
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

pub(super) fn wolf_texture_ref(baby: bool, tame: bool, angry: bool) -> EntityModelTextureRef {
    match (baby, tame, angry) {
        (false, true, _) => WOLF_TAME_TEXTURE_REF,
        (false, false, true) => WOLF_ANGRY_TEXTURE_REF,
        (false, false, false) => WOLF_TEXTURE_REF,
        (true, true, _) => WOLF_TAME_BABY_TEXTURE_REF,
        (true, false, true) => WOLF_ANGRY_BABY_TEXTURE_REF,
        (true, false, false) => WOLF_BABY_TEXTURE_REF,
    }
}

pub(super) fn player_texture_ref(slim: bool) -> EntityModelTextureRef {
    if slim {
        PLAYER_SLIM_STEVE_TEXTURE_REF
    } else {
        PLAYER_WIDE_STEVE_TEXTURE_REF
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

pub(super) fn boat_texture_ref(family: BoatModelFamily, chest: bool) -> EntityModelTextureRef {
    match (family, chest) {
        (BoatModelFamily::Acacia, false) => BOAT_ACACIA_TEXTURE_REF,
        (BoatModelFamily::Acacia, true) => CHEST_BOAT_ACACIA_TEXTURE_REF,
        (BoatModelFamily::Bamboo, false) => BOAT_BAMBOO_TEXTURE_REF,
        (BoatModelFamily::Bamboo, true) => CHEST_BOAT_BAMBOO_TEXTURE_REF,
        (BoatModelFamily::Birch, false) => BOAT_BIRCH_TEXTURE_REF,
        (BoatModelFamily::Birch, true) => CHEST_BOAT_BIRCH_TEXTURE_REF,
        (BoatModelFamily::Cherry, false) => BOAT_CHERRY_TEXTURE_REF,
        (BoatModelFamily::Cherry, true) => CHEST_BOAT_CHERRY_TEXTURE_REF,
        (BoatModelFamily::DarkOak, false) => BOAT_DARK_OAK_TEXTURE_REF,
        (BoatModelFamily::DarkOak, true) => CHEST_BOAT_DARK_OAK_TEXTURE_REF,
        (BoatModelFamily::Jungle, false) => BOAT_JUNGLE_TEXTURE_REF,
        (BoatModelFamily::Jungle, true) => CHEST_BOAT_JUNGLE_TEXTURE_REF,
        (BoatModelFamily::Mangrove, false) => BOAT_MANGROVE_TEXTURE_REF,
        (BoatModelFamily::Mangrove, true) => CHEST_BOAT_MANGROVE_TEXTURE_REF,
        (BoatModelFamily::Oak, false) => BOAT_OAK_TEXTURE_REF,
        (BoatModelFamily::Oak, true) => CHEST_BOAT_OAK_TEXTURE_REF,
        (BoatModelFamily::PaleOak, false) => BOAT_PALE_OAK_TEXTURE_REF,
        (BoatModelFamily::PaleOak, true) => CHEST_BOAT_PALE_OAK_TEXTURE_REF,
        (BoatModelFamily::Spruce, false) => BOAT_SPRUCE_TEXTURE_REF,
        (BoatModelFamily::Spruce, true) => CHEST_BOAT_SPRUCE_TEXTURE_REF,
    }
}
