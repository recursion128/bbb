use super::super::EntityModelTextureRef;

pub(in crate::entity_models) const PLAYER_WIDE_STEVE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/player/wide/steve.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const PLAYER_SLIM_STEVE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/player/slim/steve.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const PLAYER_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [PLAYER_WIDE_STEVE_TEXTURE_REF, PLAYER_SLIM_STEVE_TEXTURE_REF];

pub fn player_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &PLAYER_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const ZOMBIE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/zombie/zombie.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const ZOMBIE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/zombie/zombie_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HUSK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/zombie/husk.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HUSK_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/zombie/husk_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const DROWNED_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/zombie/drowned.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const DROWNED_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/zombie/drowned_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const ZOMBIE_VILLAGER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/zombie_villager.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const ZOMBIE_VILLAGER_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/zombie_villager_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const PIGLIN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/piglin/piglin.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const PIGLIN_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/piglin/piglin_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const PIGLIN_BRUTE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/piglin/piglin_brute.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const ZOMBIFIED_PIGLIN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/piglin/zombified_piglin.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const ZOMBIFIED_PIGLIN_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/piglin/zombified_piglin_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HOGLIN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/hoglin/hoglin.png",
        size: [128, 64],
    };

pub(in crate::entity_models) const HOGLIN_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/hoglin/hoglin_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const ZOGLIN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/hoglin/zoglin.png",
        size: [128, 64],
    };

pub(in crate::entity_models) const ZOGLIN_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/hoglin/zoglin_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const RAVAGER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/illager/ravager.png",
        size: [128, 128],
    };

pub(in crate::entity_models) const SKELETON_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/skeleton/skeleton.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const STRAY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/skeleton/stray.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const PARCHED_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/skeleton/parched.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const WITHER_SKELETON_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/skeleton/wither_skeleton.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const BOGGED_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/skeleton/bogged.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const SHEEP_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/sheep/sheep.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const SHEEP_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/sheep/sheep_baby.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const SHEEP_WOOL_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/sheep/sheep_wool.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const SHEEP_WOOL_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/sheep/sheep_wool_baby.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const SHEEP_WOOL_UNDERCOAT_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/sheep/sheep_wool_undercoat.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const SHEEP_WOOL_LAYER_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [SHEEP_WOOL_TEXTURE_REF];
pub(in crate::entity_models) const SHEEP_COLORED_WOOL_LAYER_TEXTURE_REFS: [EntityModelTextureRef;
    2] = [SHEEP_WOOL_UNDERCOAT_TEXTURE_REF, SHEEP_WOOL_TEXTURE_REF];
pub(in crate::entity_models) const SHEEP_UNDERCOAT_LAYER_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [SHEEP_WOOL_UNDERCOAT_TEXTURE_REF];
pub(in crate::entity_models) const BABY_SHEEP_WOOL_LAYER_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [SHEEP_WOOL_BABY_TEXTURE_REF];

pub(in crate::entity_models) const SHEEP_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 5] = [
    SHEEP_TEXTURE_REF,
    SHEEP_BABY_TEXTURE_REF,
    SHEEP_WOOL_UNDERCOAT_TEXTURE_REF,
    SHEEP_WOOL_TEXTURE_REF,
    SHEEP_WOOL_BABY_TEXTURE_REF,
];

pub fn sheep_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &SHEEP_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const VILLAGER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/villager/villager.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const VILLAGER_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/villager/villager_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const WANDERING_TRADER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/wandering_trader/wandering_trader.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const CHICKEN_TEMPERATE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/chicken/chicken_temperate.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const CHICKEN_TEMPERATE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/chicken/chicken_temperate_baby.png",
        size: [16, 16],
    };

pub(in crate::entity_models) const CHICKEN_WARM_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/chicken/chicken_warm.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const CHICKEN_WARM_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/chicken/chicken_warm_baby.png",
        size: [16, 16],
    };

pub(in crate::entity_models) const CHICKEN_COLD_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/chicken/chicken_cold.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const CHICKEN_COLD_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/chicken/chicken_cold_baby.png",
        size: [16, 16],
    };

pub(in crate::entity_models) const CHICKEN_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 6] = [
    CHICKEN_TEMPERATE_TEXTURE_REF,
    CHICKEN_TEMPERATE_BABY_TEXTURE_REF,
    CHICKEN_WARM_TEXTURE_REF,
    CHICKEN_WARM_BABY_TEXTURE_REF,
    CHICKEN_COLD_TEXTURE_REF,
    CHICKEN_COLD_BABY_TEXTURE_REF,
];

pub fn chicken_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &CHICKEN_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const PIG_TEMPERATE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/pig/pig_temperate.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const PIG_TEMPERATE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/pig/pig_temperate_baby.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const PIG_WARM_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/pig/pig_warm.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const PIG_WARM_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/pig/pig_warm_baby.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const PIG_COLD_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/pig/pig_cold.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const PIG_COLD_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/pig/pig_cold_baby.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const PIG_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 6] = [
    PIG_TEMPERATE_TEXTURE_REF,
    PIG_TEMPERATE_BABY_TEXTURE_REF,
    PIG_WARM_TEXTURE_REF,
    PIG_WARM_BABY_TEXTURE_REF,
    PIG_COLD_TEXTURE_REF,
    PIG_COLD_BABY_TEXTURE_REF,
];

pub fn pig_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &PIG_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const COW_TEMPERATE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cow/cow_temperate.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const COW_TEMPERATE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cow/cow_temperate_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const COW_WARM_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cow/cow_warm.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const COW_WARM_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cow/cow_warm_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const COW_COLD_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cow/cow_cold.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const COW_COLD_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cow/cow_cold_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const COW_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 6] = [
    COW_TEMPERATE_TEXTURE_REF,
    COW_TEMPERATE_BABY_TEXTURE_REF,
    COW_WARM_TEXTURE_REF,
    COW_WARM_BABY_TEXTURE_REF,
    COW_COLD_TEXTURE_REF,
    COW_COLD_BABY_TEXTURE_REF,
];

pub fn cow_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &COW_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const BOAT_ACACIA_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/boat/acacia.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const CHEST_BOAT_ACACIA_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/chest_boat/acacia.png",
        size: [128, 128],
    };
pub(in crate::entity_models) const BOAT_BAMBOO_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/boat/bamboo.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const CHEST_BOAT_BAMBOO_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/chest_boat/bamboo.png",
        size: [128, 128],
    };
pub(in crate::entity_models) const BOAT_BIRCH_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/boat/birch.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const CHEST_BOAT_BIRCH_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/chest_boat/birch.png",
        size: [128, 128],
    };
pub(in crate::entity_models) const BOAT_CHERRY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/boat/cherry.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const CHEST_BOAT_CHERRY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/chest_boat/cherry.png",
        size: [128, 128],
    };
pub(in crate::entity_models) const BOAT_DARK_OAK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/boat/dark_oak.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const CHEST_BOAT_DARK_OAK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/chest_boat/dark_oak.png",
        size: [128, 128],
    };
pub(in crate::entity_models) const BOAT_JUNGLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/boat/jungle.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const CHEST_BOAT_JUNGLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/chest_boat/jungle.png",
        size: [128, 128],
    };
pub(in crate::entity_models) const BOAT_MANGROVE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/boat/mangrove.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const CHEST_BOAT_MANGROVE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/chest_boat/mangrove.png",
        size: [128, 128],
    };
pub(in crate::entity_models) const BOAT_OAK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/boat/oak.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const CHEST_BOAT_OAK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/chest_boat/oak.png",
        size: [128, 128],
    };
pub(in crate::entity_models) const BOAT_PALE_OAK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/boat/pale_oak.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const CHEST_BOAT_PALE_OAK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/chest_boat/pale_oak.png",
        size: [128, 128],
    };
pub(in crate::entity_models) const BOAT_SPRUCE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/boat/spruce.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const CHEST_BOAT_SPRUCE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/chest_boat/spruce.png",
        size: [128, 128],
    };

pub(in crate::entity_models) const BOAT_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 20] = [
    BOAT_ACACIA_TEXTURE_REF,
    CHEST_BOAT_ACACIA_TEXTURE_REF,
    BOAT_BAMBOO_TEXTURE_REF,
    CHEST_BOAT_BAMBOO_TEXTURE_REF,
    BOAT_BIRCH_TEXTURE_REF,
    CHEST_BOAT_BIRCH_TEXTURE_REF,
    BOAT_CHERRY_TEXTURE_REF,
    CHEST_BOAT_CHERRY_TEXTURE_REF,
    BOAT_DARK_OAK_TEXTURE_REF,
    CHEST_BOAT_DARK_OAK_TEXTURE_REF,
    BOAT_JUNGLE_TEXTURE_REF,
    CHEST_BOAT_JUNGLE_TEXTURE_REF,
    BOAT_MANGROVE_TEXTURE_REF,
    CHEST_BOAT_MANGROVE_TEXTURE_REF,
    BOAT_OAK_TEXTURE_REF,
    CHEST_BOAT_OAK_TEXTURE_REF,
    BOAT_PALE_OAK_TEXTURE_REF,
    CHEST_BOAT_PALE_OAK_TEXTURE_REF,
    BOAT_SPRUCE_TEXTURE_REF,
    CHEST_BOAT_SPRUCE_TEXTURE_REF,
];

pub fn boat_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &BOAT_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const WOLF_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/wolf/wolf.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const WOLF_TAME_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/wolf/wolf_tame.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const WOLF_ANGRY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/wolf/wolf_angry.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const WOLF_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/wolf/wolf_baby.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const WOLF_TAME_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/wolf/wolf_tame_baby.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const WOLF_ANGRY_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/wolf/wolf_angry_baby.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const WOLF_COLLAR_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/wolf/wolf_collar.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const WOLF_BABY_COLLAR_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/wolf/wolf_collar_baby.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const WOLF_COLLAR_LAYER_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [WOLF_COLLAR_TEXTURE_REF];
pub(in crate::entity_models) const WOLF_BABY_COLLAR_LAYER_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [WOLF_BABY_COLLAR_TEXTURE_REF];

pub(in crate::entity_models) const WOLF_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 8] = [
    WOLF_TEXTURE_REF,
    WOLF_TAME_TEXTURE_REF,
    WOLF_ANGRY_TEXTURE_REF,
    WOLF_BABY_TEXTURE_REF,
    WOLF_TAME_BABY_TEXTURE_REF,
    WOLF_ANGRY_BABY_TEXTURE_REF,
    WOLF_COLLAR_TEXTURE_REF,
    WOLF_BABY_COLLAR_TEXTURE_REF,
];

pub fn wolf_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &WOLF_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const ENTITY_MODEL_TEXTURE_REFS: [EntityModelTextureRef; 54] = [
    PLAYER_WIDE_STEVE_TEXTURE_REF,
    PLAYER_SLIM_STEVE_TEXTURE_REF,
    SHEEP_TEXTURE_REF,
    SHEEP_BABY_TEXTURE_REF,
    SHEEP_WOOL_UNDERCOAT_TEXTURE_REF,
    SHEEP_WOOL_TEXTURE_REF,
    SHEEP_WOOL_BABY_TEXTURE_REF,
    WOLF_TEXTURE_REF,
    WOLF_TAME_TEXTURE_REF,
    WOLF_ANGRY_TEXTURE_REF,
    WOLF_BABY_TEXTURE_REF,
    WOLF_TAME_BABY_TEXTURE_REF,
    WOLF_ANGRY_BABY_TEXTURE_REF,
    WOLF_COLLAR_TEXTURE_REF,
    WOLF_BABY_COLLAR_TEXTURE_REF,
    CHICKEN_TEMPERATE_TEXTURE_REF,
    CHICKEN_TEMPERATE_BABY_TEXTURE_REF,
    CHICKEN_WARM_TEXTURE_REF,
    CHICKEN_WARM_BABY_TEXTURE_REF,
    CHICKEN_COLD_TEXTURE_REF,
    CHICKEN_COLD_BABY_TEXTURE_REF,
    PIG_TEMPERATE_TEXTURE_REF,
    PIG_TEMPERATE_BABY_TEXTURE_REF,
    PIG_WARM_TEXTURE_REF,
    PIG_WARM_BABY_TEXTURE_REF,
    PIG_COLD_TEXTURE_REF,
    PIG_COLD_BABY_TEXTURE_REF,
    COW_TEMPERATE_TEXTURE_REF,
    COW_TEMPERATE_BABY_TEXTURE_REF,
    COW_WARM_TEXTURE_REF,
    COW_WARM_BABY_TEXTURE_REF,
    COW_COLD_TEXTURE_REF,
    COW_COLD_BABY_TEXTURE_REF,
    CREEPER_TEXTURE_REF,
    BOAT_ACACIA_TEXTURE_REF,
    CHEST_BOAT_ACACIA_TEXTURE_REF,
    BOAT_BAMBOO_TEXTURE_REF,
    CHEST_BOAT_BAMBOO_TEXTURE_REF,
    BOAT_BIRCH_TEXTURE_REF,
    CHEST_BOAT_BIRCH_TEXTURE_REF,
    BOAT_CHERRY_TEXTURE_REF,
    CHEST_BOAT_CHERRY_TEXTURE_REF,
    BOAT_DARK_OAK_TEXTURE_REF,
    CHEST_BOAT_DARK_OAK_TEXTURE_REF,
    BOAT_JUNGLE_TEXTURE_REF,
    CHEST_BOAT_JUNGLE_TEXTURE_REF,
    BOAT_MANGROVE_TEXTURE_REF,
    CHEST_BOAT_MANGROVE_TEXTURE_REF,
    BOAT_OAK_TEXTURE_REF,
    CHEST_BOAT_OAK_TEXTURE_REF,
    BOAT_PALE_OAK_TEXTURE_REF,
    CHEST_BOAT_PALE_OAK_TEXTURE_REF,
    BOAT_SPRUCE_TEXTURE_REF,
    CHEST_BOAT_SPRUCE_TEXTURE_REF,
];

pub fn entity_model_texture_refs() -> &'static [EntityModelTextureRef] {
    &ENTITY_MODEL_TEXTURE_REFS
}

pub(in crate::entity_models) const HORSE_WHITE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_white.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_WHITE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_white_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const DONKEY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/donkey.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const DONKEY_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/donkey_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const MULE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/mule.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const MULE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/mule_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const SKELETON_HORSE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_skeleton.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const SKELETON_HORSE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_skeleton_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const ZOMBIE_HORSE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_zombie.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const ZOMBIE_HORSE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_zombie_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const CAMEL_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/camel/camel.png",
        size: [128, 128],
    };

pub(in crate::entity_models) const CAMEL_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/camel/camel_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const CAMEL_HUSK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/camel/camel_husk.png",
        size: [128, 128],
    };

pub(in crate::entity_models) const LLAMA_CREAMY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/llama/llama_creamy.png",
        size: [128, 64],
    };

pub(in crate::entity_models) const LLAMA_CREAMY_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/llama/llama_creamy_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const LLAMA_WHITE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/llama/llama_white.png",
        size: [128, 64],
    };

pub(in crate::entity_models) const LLAMA_WHITE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/llama/llama_white_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const LLAMA_BROWN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/llama/llama_brown.png",
        size: [128, 64],
    };

pub(in crate::entity_models) const LLAMA_BROWN_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/llama/llama_brown_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const LLAMA_GRAY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/llama/llama_gray.png",
        size: [128, 64],
    };

pub(in crate::entity_models) const LLAMA_GRAY_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/llama/llama_gray_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const GOAT_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/goat/goat.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const GOAT_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/goat/goat_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const POLAR_BEAR_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/bear/polarbear.png",
        size: [128, 64],
    };

pub(in crate::entity_models) const POLAR_BEAR_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/bear/polarbear_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const CREEPER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/creeper/creeper.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const CREEPER_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [CREEPER_TEXTURE_REF];

pub fn creeper_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &CREEPER_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const SPIDER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/spider/spider.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const CAVE_SPIDER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/spider/cave_spider.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const ENDERMAN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/enderman/enderman.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const IRON_GOLEM_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/iron_golem/iron_golem.png",
        size: [128, 128],
    };

pub(in crate::entity_models) const SNOW_GOLEM_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/snow_golem/snow_golem.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const WITCH_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/witch/witch.png",
        size: [64, 128],
    };

pub(in crate::entity_models) const EVOKER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/illager/evoker.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const ILLUSIONER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/illager/illusioner.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const PILLAGER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/illager/pillager.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const VINDICATOR_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/illager/vindicator.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const ARMOR_STAND_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/armorstand/armorstand.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const SLIME_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/slime/slime.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const MAGMA_CUBE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/slime/magmacube.png",
        size: [64, 64],
    };
