use super::super::model_layers::*;
use super::*;

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
                jeb,
                ..
            } => sheep_model_key(baby, sheared, wool_color, jeb),
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
                invisible: false,
                jeb: false,
                wool_color: SheepWoolColor::White,
                ..
            } => &SHEEP_WOOL_LAYER_TEXTURE_REFS,
            Self::Sheep {
                baby: false,
                sheared: false,
                invisible: false,
                ..
            } => &SHEEP_COLORED_WOOL_LAYER_TEXTURE_REFS,
            Self::Sheep {
                baby: false,
                sheared: true,
                invisible: false,
                jeb: false,
                wool_color: SheepWoolColor::White,
                ..
            } => &[],
            Self::Sheep {
                baby: false,
                sheared: true,
                invisible: false,
                ..
            } => &SHEEP_UNDERCOAT_LAYER_TEXTURE_REFS,
            Self::Sheep {
                baby: true,
                sheared: false,
                invisible: false,
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
                invisible: false,
                collar_color: Some(_),
                ..
            } => &WOLF_COLLAR_LAYER_TEXTURE_REFS,
            Self::Wolf {
                baby: true,
                tame: true,
                invisible: false,
                collar_color: Some(_),
                ..
            } => &WOLF_BABY_COLLAR_LAYER_TEXTURE_REFS,
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Stray,
            } => &STRAY_OVERLAY_LAYER_TEXTURE_REFS,
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Bogged { .. },
            } => &BOGGED_OVERLAY_LAYER_TEXTURE_REFS,
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

pub(in crate::entity_models) fn chicken_texture_ref(
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

pub(in crate::entity_models) fn pig_texture_ref(
    variant: PigModelVariant,
    baby: bool,
) -> EntityModelTextureRef {
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

pub(in crate::entity_models) fn cow_texture_ref(
    variant: CowModelVariant,
    baby: bool,
) -> EntityModelTextureRef {
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

fn sheep_model_key(
    baby: bool,
    sheared: bool,
    wool_color: SheepWoolColor,
    jeb: bool,
) -> &'static str {
    if jeb {
        return match (baby, sheared) {
            (false, false) => "sheep_jeb",
            (true, false) => "sheep_jeb_baby",
            (false, true) => "sheep_jeb_sheared",
            (true, true) => "sheep_jeb_baby_sheared",
        };
    }

    let color = wool_color.vanilla_id() as usize;
    match (baby, sheared) {
        (false, false) => SHEEP_WOOL_COLOR_MODEL_KEYS[color],
        (true, false) => BABY_SHEEP_WOOL_COLOR_MODEL_KEYS[color],
        (false, true) => SHEEP_SHEARED_COLOR_MODEL_KEYS[color],
        (true, true) => "sheep_baby_sheared",
    }
}

pub(in crate::entity_models) fn sheep_wool_layer_color(wool_color: SheepWoolColor) -> [f32; 4] {
    let [red, green, blue] = SHEEP_WOOL_LAYER_COLOR_BYTES[wool_color.vanilla_id() as usize];
    sheep_wool_layer_color_from_bytes([red, green, blue])
}

pub(in crate::entity_models) fn sheep_wool_render_color(
    wool_color: SheepWoolColor,
    jeb: bool,
    age_ticks: f32,
) -> [f32; 4] {
    if jeb {
        sheep_jeb_wool_layer_color(age_ticks)
    } else {
        sheep_wool_layer_color(wool_color)
    }
}

pub(in crate::entity_models) fn sheep_jeb_wool_layer_color(age_ticks: f32) -> [f32; 4] {
    const SHEEP_COLOR_DURATION_TICKS: i32 = 25;
    let tick = age_ticks.max(0.0);
    let tick_floor = tick.floor() as i32;
    let color_step = tick_floor / SHEEP_COLOR_DURATION_TICKS;
    let color_count = SHEEP_WOOL_LAYER_COLOR_BYTES.len() as i32;
    let from = color_step.rem_euclid(color_count) as usize;
    let to = (color_step + 1).rem_euclid(color_count) as usize;
    let alpha = ((tick_floor % SHEEP_COLOR_DURATION_TICKS) as f32 + tick.fract())
        / SHEEP_COLOR_DURATION_TICKS as f32;
    sheep_wool_layer_color_from_bytes(sheep_lerp_color_bytes(
        SHEEP_WOOL_LAYER_COLOR_BYTES[from],
        SHEEP_WOOL_LAYER_COLOR_BYTES[to],
        alpha,
    ))
}

fn sheep_wool_layer_color_from_bytes([red, green, blue]: [u8; 3]) -> [f32; 4] {
    [
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        1.0,
    ]
}

fn sheep_lerp_color_bytes(from: [u8; 3], to: [u8; 3], alpha: f32) -> [u8; 3] {
    [
        sheep_lerp_color_channel(from[0], to[0], alpha),
        sheep_lerp_color_channel(from[1], to[1], alpha),
        sheep_lerp_color_channel(from[2], to[2], alpha),
    ]
}

fn sheep_lerp_color_channel(from: u8, to: u8, alpha: f32) -> u8 {
    let from = i32::from(from);
    let to = i32::from(to);
    (from + (alpha * (to - from) as f32).floor() as i32).clamp(0, 255) as u8
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

pub(in crate::entity_models) fn wolf_texture_ref(
    baby: bool,
    tame: bool,
    angry: bool,
) -> EntityModelTextureRef {
    match (baby, tame, angry) {
        (false, true, _) => WOLF_TAME_TEXTURE_REF,
        (false, false, true) => WOLF_ANGRY_TEXTURE_REF,
        (false, false, false) => WOLF_TEXTURE_REF,
        (true, true, _) => WOLF_TAME_BABY_TEXTURE_REF,
        (true, false, true) => WOLF_ANGRY_BABY_TEXTURE_REF,
        (true, false, false) => WOLF_BABY_TEXTURE_REF,
    }
}

pub(in crate::entity_models) fn player_texture_ref(slim: bool) -> EntityModelTextureRef {
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

pub(in crate::entity_models) fn boat_texture_ref(
    family: BoatModelFamily,
    chest: bool,
) -> EntityModelTextureRef {
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
