use super::{
    degree_vec, inflate_textured_cube, keyframe, pos_vec, AnimationChannel, AnimationDefinition,
    AnimationTarget, BoneAnimation, Keyframe, KeyframeInterpolation, ModelCubeDesc, ModelPartDesc,
    PartPose, TexturedModelCubeDesc, TexturedModelPartDesc, PART_POSE_ZERO,
};

pub(super) const CHICKEN_WHITE: [f32; 4] = [0.94, 0.94, 0.86, 1.0];
pub(super) const CHICKEN_WING: [f32; 4] = [0.82, 0.82, 0.76, 1.0];
pub(super) const CHICKEN_BEAK: [f32; 4] = [0.95, 0.62, 0.18, 1.0];
pub(super) const CHICKEN_RED: [f32; 4] = [0.86, 0.08, 0.08, 1.0];
pub(super) const CHICKEN_LEG: [f32; 4] = [0.82, 0.48, 0.12, 1.0];
pub(super) const PLAYER_BLUE: [f32; 4] = [0.22, 0.42, 0.78, 1.0];
pub(super) const HOGLIN_RED: [f32; 4] = [0.60, 0.28, 0.24, 1.0];
pub(super) const ZOGLIN_GREEN: [f32; 4] = [0.42, 0.55, 0.39, 1.0];
pub(super) const RAVAGER_GRAY: [f32; 4] = [0.44, 0.38, 0.34, 1.0];
pub(super) const VILLAGER_ROBE: [f32; 4] = [0.48, 0.34, 0.23, 1.0];
pub(super) const ILLAGER_GRAY: [f32; 4] = [0.42, 0.45, 0.48, 1.0];
pub(super) const PIG_PINK: [f32; 4] = [0.92, 0.55, 0.62, 1.0];
pub(super) const PIG_COLD_FUR: [f32; 4] = [0.82, 0.78, 0.70, 1.0];
pub(super) const SHEEP_WOOL: [f32; 4] = [0.86, 0.86, 0.80, 1.0];
pub(super) const HORSE_BROWN: [f32; 4] = [0.44, 0.27, 0.14, 1.0];
pub(super) const DONKEY_GRAY: [f32; 4] = [0.46, 0.45, 0.42, 1.0];
pub(super) const MULE_BROWN: [f32; 4] = [0.34, 0.24, 0.17, 1.0];
pub(super) const SKELETON_HORSE_BONE: [f32; 4] = [0.78, 0.78, 0.68, 1.0];
pub(super) const ZOMBIE_HORSE_GREEN: [f32; 4] = [0.32, 0.54, 0.32, 1.0];
pub(super) const CAMEL_TAN: [f32; 4] = [0.72, 0.50, 0.31, 1.0];
pub(super) const CAMEL_HUSK_BROWN: [f32; 4] = [0.42, 0.33, 0.25, 1.0];
pub(super) const LLAMA_CREAMY: [f32; 4] = [0.78, 0.65, 0.45, 1.0];
pub(super) const LLAMA_WHITE: [f32; 4] = [0.86, 0.84, 0.76, 1.0];
pub(super) const LLAMA_BROWN: [f32; 4] = [0.43, 0.27, 0.16, 1.0];
pub(super) const LLAMA_GRAY: [f32; 4] = [0.45, 0.44, 0.40, 1.0];
pub(super) const GOAT_WHITE: [f32; 4] = [0.84, 0.80, 0.70, 1.0];
pub(super) const GOAT_HORN: [f32; 4] = [0.72, 0.66, 0.54, 1.0];
pub(super) const GOAT_BEARD: [f32; 4] = [0.48, 0.42, 0.32, 1.0];
pub(super) const POLAR_BEAR_WHITE: [f32; 4] = [0.88, 0.88, 0.82, 1.0];
pub(super) const CREEPER_GREEN: [f32; 4] = [0.24, 0.68, 0.23, 1.0];
pub(super) const SPIDER_DARK: [f32; 4] = [0.16, 0.12, 0.12, 1.0];
pub(super) const ENDERMAN_DARK: [f32; 4] = [0.08, 0.06, 0.10, 1.0];
pub(super) const IRON_GOLEM_STONE: [f32; 4] = [0.74, 0.74, 0.68, 1.0];
pub(super) const SNOW_GOLEM_WHITE: [f32; 4] = [0.90, 0.92, 0.88, 1.0];
pub(super) const SQUID_BLUE: [f32; 4] = [0.39, 0.45, 0.55, 1.0];
pub(super) const COD_TAN: [f32; 4] = [0.62, 0.55, 0.42, 1.0];
pub(super) const SALMON_RED: [f32; 4] = [0.74, 0.33, 0.31, 1.0];
pub(super) const TROPICAL_FISH_ORANGE: [f32; 4] = [0.93, 0.52, 0.18, 1.0];
pub(super) const VEX_GREY: [f32; 4] = [0.62, 0.69, 0.74, 1.0];
pub(super) const ALLAY_BLUE: [f32; 4] = [0.42, 0.62, 0.86, 1.0];
pub(super) const STRIDER_MAROON: [f32; 4] = [0.49, 0.20, 0.27, 1.0];
pub(super) const STRIDER_LEG: [f32; 4] = [0.78, 0.32, 0.30, 1.0];
pub(super) const TURTLE_GREEN: [f32; 4] = [0.40, 0.60, 0.36, 1.0];
pub(super) const TURTLE_SHELL: [f32; 4] = [0.42, 0.45, 0.30, 1.0];
pub(super) const BAT_BROWN: [f32; 4] = [0.32, 0.27, 0.22, 1.0];
pub(super) const BEE_YELLOW: [f32; 4] = [0.93, 0.77, 0.20, 1.0];
pub(super) const BREEZE_SLATE: [f32; 4] = [0.36, 0.40, 0.52, 1.0];
pub(super) const DOLPHIN_GRAY: [f32; 4] = [0.66, 0.72, 0.78, 1.0];
pub(super) const GLOW_SQUID_TEAL: [f32; 4] = [0.13, 0.65, 0.62, 1.0];
pub(super) const WITCH_ROBE: [f32; 4] = [0.28, 0.17, 0.36, 1.0];
pub(super) const WITCH_HAT_COLOR: [f32; 4] = [0.16, 0.11, 0.20, 1.0];
pub(super) const ILLAGER_ROBE: [f32; 4] = [0.38, 0.40, 0.44, 1.0];
pub(super) const ILLAGER_HAT_COLOR: [f32; 4] = [0.30, 0.31, 0.34, 1.0];
pub(super) const BOAT_WOOD: [f32; 4] = [0.55, 0.36, 0.18, 1.0];
pub(super) const PLACEHOLDER_COLOR: [f32; 4] = [0.80, 0.20, 0.72, 1.0];

mod allay;
mod armor_stand;
mod bat;
mod bee;
mod blaze;
mod boat;
mod breeze;
mod camel;
mod chicken;
mod cod;
mod cow;
mod creeper;
mod dolphin;
mod enderman;
mod endermite;
mod equine;
mod ghast;
mod goat;
mod golem;
mod happy_ghast;
mod head_look;
mod hoglin;
mod illager;
mod llama;
mod minecart;
mod phantom;
mod pig;
mod piglin;
mod player;
mod polar_bear;
mod pufferfish;
mod ravager;
mod salmon;
mod sheep;
mod silverfish;
mod skeleton;
mod skeleton_clothing;
mod slime;
mod spider;
mod squid;
mod strider;
mod textures;
mod tropical_fish;
mod turtle;
mod vex;
mod villager;
mod witch;
mod wolf;
mod zombie;

pub(super) use allay::*;
pub(super) use armor_stand::*;
pub(super) use bat::*;
pub(super) use bee::*;
pub(super) use blaze::*;
pub(super) use boat::*;
pub(super) use breeze::*;
pub(super) use camel::*;
pub(super) use chicken::*;
pub(super) use cod::*;
pub(super) use cow::*;
pub(super) use creeper::*;
pub(super) use dolphin::*;
pub(super) use enderman::*;
pub(super) use endermite::*;
pub(super) use equine::*;
pub(super) use ghast::*;
pub(super) use goat::*;
pub(super) use golem::*;
pub(super) use happy_ghast::*;
pub(super) use head_look::*;
pub(super) use hoglin::*;
pub(super) use illager::*;
pub(super) use llama::*;
pub(super) use minecart::*;
pub(super) use phantom::*;
pub(super) use pig::*;
pub(super) use piglin::*;
pub(super) use player::*;
pub(super) use polar_bear::*;
pub(super) use pufferfish::*;
pub(super) use ravager::*;
pub(super) use salmon::*;
pub use sheep::SheepHeadEatPose;
pub(super) use sheep::*;
pub(super) use silverfish::*;
pub(super) use skeleton::*;
pub(super) use skeleton_clothing::*;
pub(super) use slime::*;
pub(super) use spider::*;
pub(super) use squid::*;
pub(super) use strider::*;
pub(super) use textures::*;
pub use textures::{
    allay_entity_texture_refs, armor_stand_entity_texture_refs, bat_entity_texture_refs,
    bee_entity_texture_refs, blaze_entity_texture_refs, boat_entity_texture_refs,
    breeze_entity_texture_refs, camel_entity_texture_refs, chicken_entity_texture_refs,
    cod_entity_texture_refs, cow_entity_texture_refs, creeper_entity_texture_refs,
    dolphin_entity_texture_refs, drowned_entity_texture_refs, enderman_entity_texture_refs,
    endermite_entity_texture_refs, entity_model_texture_refs, ghast_entity_texture_refs,
    goat_entity_texture_refs, happy_ghast_entity_texture_refs, hoglin_entity_texture_refs,
    husk_entity_texture_refs, illager_entity_texture_refs, llama_entity_texture_refs,
    minecart_entity_texture_refs, phantom_entity_texture_refs, pig_entity_texture_refs,
    piglin_entity_texture_refs, player_entity_texture_refs, polar_bear_entity_texture_refs,
    pufferfish_entity_texture_refs, ravager_entity_texture_refs, salmon_entity_texture_refs,
    sheep_entity_texture_refs, silverfish_entity_texture_refs, skeleton_entity_texture_refs,
    slime_entity_texture_refs, spider_entity_texture_refs, squid_entity_texture_refs,
    strider_entity_texture_refs, tropical_fish_entity_texture_refs, turtle_entity_texture_refs,
    vex_entity_texture_refs, villager_entity_texture_refs, witch_entity_texture_refs,
    wolf_entity_texture_refs, zombie_entity_texture_refs, zombie_villager_entity_texture_refs,
};
pub(super) use tropical_fish::*;
pub(super) use turtle::*;
pub(super) use vex::*;
pub(super) use villager::*;
pub(super) use witch::*;
pub(super) use wolf::*;
pub(super) use zombie::*;
