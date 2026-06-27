use super::super::catalog::*;
use super::super::model_layers::*;
use super::transforms::{DONKEY_SCALE, MULE_SCALE};

pub(super) fn humanoid_model_color(family: HumanoidModelFamily) -> [f32; 4] {
    match family {
        HumanoidModelFamily::Player => PLAYER_BLUE,
        HumanoidModelFamily::Zombie => ZOMBIE_GREEN,
        HumanoidModelFamily::Skeleton => SKELETON_BONE,
        HumanoidModelFamily::Villager => VILLAGER_ROBE,
        HumanoidModelFamily::Illager => ILLAGER_GRAY,
        HumanoidModelFamily::ArmorStand => ARMOR_STAND_WOOD,
    }
}

pub(super) fn piglin_model_color(family: PiglinModelFamily) -> [f32; 4] {
    match family {
        PiglinModelFamily::Piglin => PIGLIN_SKIN,
        PiglinModelFamily::PiglinBrute => PIGLIN_BRUTE_SKIN,
        PiglinModelFamily::ZombifiedPiglin => ZOMBIFIED_PIGLIN_SKIN,
    }
}

pub(super) fn hoglin_model_color(family: HoglinModelFamily) -> [f32; 4] {
    match family {
        HoglinModelFamily::Hoglin => HOGLIN_RED,
        HoglinModelFamily::Zoglin => ZOGLIN_GREEN,
    }
}

pub(super) fn quadruped_model_color(family: QuadrupedModelFamily) -> [f32; 4] {
    match family {
        QuadrupedModelFamily::Pig => PIG_PINK,
        QuadrupedModelFamily::Cow => COW_BROWN,
        QuadrupedModelFamily::Sheep => SHEEP_WOOL,
        QuadrupedModelFamily::Horse => HORSE_BROWN,
        QuadrupedModelFamily::Wolf => WOLF_GRAY,
    }
}

pub(super) fn donkey_model_scale(family: DonkeyModelFamily) -> f32 {
    match family {
        DonkeyModelFamily::Donkey => DONKEY_SCALE,
        DonkeyModelFamily::Mule => MULE_SCALE,
    }
}

pub(super) fn donkey_model_color(family: DonkeyModelFamily) -> [f32; 4] {
    match family {
        DonkeyModelFamily::Donkey => DONKEY_GRAY,
        DonkeyModelFamily::Mule => MULE_BROWN,
    }
}

pub(super) fn undead_horse_model_color(family: UndeadHorseModelFamily) -> [f32; 4] {
    match family {
        UndeadHorseModelFamily::Skeleton => SKELETON_HORSE_BONE,
        UndeadHorseModelFamily::Zombie => ZOMBIE_HORSE_GREEN,
    }
}

pub(in crate::entity_models) fn camel_model_color(family: CamelModelFamily) -> [f32; 4] {
    match family {
        CamelModelFamily::Camel => CAMEL_TAN,
        CamelModelFamily::CamelHusk => CAMEL_HUSK_BROWN,
    }
}

pub(super) fn llama_model_color(_family: LlamaModelFamily, variant: LlamaVariant) -> [f32; 4] {
    match variant {
        LlamaVariant::Creamy => LLAMA_CREAMY,
        LlamaVariant::White => LLAMA_WHITE,
        LlamaVariant::Brown => LLAMA_BROWN,
        LlamaVariant::Gray => LLAMA_GRAY,
    }
}
