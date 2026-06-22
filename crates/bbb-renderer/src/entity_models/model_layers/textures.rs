use super::super::EntityModelTextureRef;

mod equine;

pub(in crate::entity_models) use equine::*;

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

pub(in crate::entity_models) const ZOMBIE_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [ZOMBIE_TEXTURE_REF, ZOMBIE_BABY_TEXTURE_REF];

pub fn zombie_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &ZOMBIE_ENTITY_TEXTURE_REFS
}

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

pub(in crate::entity_models) const HUSK_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [HUSK_TEXTURE_REF, HUSK_BABY_TEXTURE_REF];

pub fn husk_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &HUSK_ENTITY_TEXTURE_REFS
}

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

pub(in crate::entity_models) const DROWNED_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [DROWNED_TEXTURE_REF, DROWNED_BABY_TEXTURE_REF];

pub fn drowned_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &DROWNED_ENTITY_TEXTURE_REFS
}

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

pub(in crate::entity_models) const ZOMBIE_VILLAGER_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] = [
    ZOMBIE_VILLAGER_TEXTURE_REF,
    ZOMBIE_VILLAGER_BABY_TEXTURE_REF,
];

pub fn zombie_villager_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &ZOMBIE_VILLAGER_ENTITY_TEXTURE_REFS
}

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

pub(in crate::entity_models) const PIGLIN_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 5] = [
    PIGLIN_TEXTURE_REF,
    PIGLIN_BABY_TEXTURE_REF,
    PIGLIN_BRUTE_TEXTURE_REF,
    ZOMBIFIED_PIGLIN_TEXTURE_REF,
    ZOMBIFIED_PIGLIN_BABY_TEXTURE_REF,
];

pub fn piglin_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &PIGLIN_ENTITY_TEXTURE_REFS
}

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

pub(in crate::entity_models) const HOGLIN_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 4] = [
    HOGLIN_TEXTURE_REF,
    HOGLIN_BABY_TEXTURE_REF,
    ZOGLIN_TEXTURE_REF,
    ZOGLIN_BABY_TEXTURE_REF,
];

pub fn hoglin_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &HOGLIN_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const RAVAGER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/illager/ravager.png",
        size: [128, 128],
    };

pub(in crate::entity_models) const RAVAGER_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [RAVAGER_TEXTURE_REF];

pub fn ravager_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &RAVAGER_ENTITY_TEXTURE_REFS
}

// Vanilla `VexRenderer` non-charging texture. The `vex_charging.png` swap and the constant
// full-bright `getBlockLightLevel` glow are deferred.
pub(in crate::entity_models) const VEX_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/illager/vex.png",
    size: [32, 32],
};

// Vanilla `AllayRenderer` texture. The constant full-bright `getBlockLightLevel` glow is
// deferred lighting.
pub(in crate::entity_models) const ALLAY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/allay/allay.png",
        size: [32, 32],
    };

// Vanilla `StriderRenderer` non-suffocating textures. The cold/suffocating swaps
// (`strider_cold.png` / `strider_cold_baby.png`) and the saddle equipment layer are deferred.
pub(in crate::entity_models) const STRIDER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/strider/strider.png",
        size: [64, 128],
    };

pub(in crate::entity_models) const STRIDER_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/strider/strider_baby.png",
        size: [32, 32],
    };

// Vanilla `TurtleRenderer` textures (adult 128×64, baby 16×16).
pub(in crate::entity_models) const TURTLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/turtle/turtle.png",
        size: [128, 64],
    };

pub(in crate::entity_models) const TURTLE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/turtle/turtle_baby.png",
        size: [16, 16],
    };

// Vanilla `BatRenderer` texture.
pub(in crate::entity_models) const BAT_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/bat/bat.png",
    size: [32, 32],
};

// Vanilla `BeeRenderer` base textures (the non-angry, non-nectar adult and baby).
pub(in crate::entity_models) const BEE_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/bee/bee.png",
    size: [64, 64],
};
pub(in crate::entity_models) const BEE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/bee/bee_baby.png",
        size: [32, 32],
    };

// Vanilla `BreezeRenderer` base body texture (the swirling wind layer / emissive eyes use the
// separate `breeze_wind.png` / `breeze_eyes.png`).
pub(in crate::entity_models) const BREEZE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/breeze/breeze.png",
        size: [32, 32],
    };

// Vanilla `DolphinRenderer` textures (adult and baby share the same 64×64 geometry).
pub(in crate::entity_models) const DOLPHIN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/dolphin/dolphin.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const DOLPHIN_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/dolphin/dolphin_baby.png",
        size: [64, 64],
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

pub(in crate::entity_models) const STRAY_OVERLAY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/skeleton/stray_overlay.png",
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

pub(in crate::entity_models) const BOGGED_OVERLAY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/skeleton/bogged_overlay.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const STRAY_OVERLAY_LAYER_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [STRAY_OVERLAY_TEXTURE_REF];

pub(in crate::entity_models) const BOGGED_OVERLAY_LAYER_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [BOGGED_OVERLAY_TEXTURE_REF];

pub(in crate::entity_models) const SKELETON_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 7] = [
    SKELETON_TEXTURE_REF,
    STRAY_TEXTURE_REF,
    STRAY_OVERLAY_TEXTURE_REF,
    PARCHED_TEXTURE_REF,
    WITHER_SKELETON_TEXTURE_REF,
    BOGGED_TEXTURE_REF,
    BOGGED_OVERLAY_TEXTURE_REF,
];

pub fn skeleton_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &SKELETON_ENTITY_TEXTURE_REFS
}

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

pub(in crate::entity_models) const VILLAGER_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 3] = [
    VILLAGER_TEXTURE_REF,
    VILLAGER_BABY_TEXTURE_REF,
    WANDERING_TRADER_TEXTURE_REF,
];

pub fn villager_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &VILLAGER_ENTITY_TEXTURE_REFS
}

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

pub(in crate::entity_models) const ENTITY_MODEL_TEXTURE_REFS: [EntityModelTextureRef; 153] = [
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
    GOAT_TEXTURE_REF,
    GOAT_BABY_TEXTURE_REF,
    POLAR_BEAR_TEXTURE_REF,
    POLAR_BEAR_BABY_TEXTURE_REF,
    HOGLIN_TEXTURE_REF,
    HOGLIN_BABY_TEXTURE_REF,
    ZOGLIN_TEXTURE_REF,
    ZOGLIN_BABY_TEXTURE_REF,
    RAVAGER_TEXTURE_REF,
    VILLAGER_TEXTURE_REF,
    VILLAGER_BABY_TEXTURE_REF,
    WANDERING_TRADER_TEXTURE_REF,
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
    SKELETON_TEXTURE_REF,
    STRAY_TEXTURE_REF,
    STRAY_OVERLAY_TEXTURE_REF,
    PARCHED_TEXTURE_REF,
    WITHER_SKELETON_TEXTURE_REF,
    BOGGED_TEXTURE_REF,
    BOGGED_OVERLAY_TEXTURE_REF,
    CREEPER_TEXTURE_REF,
    SPIDER_TEXTURE_REF,
    CAVE_SPIDER_TEXTURE_REF,
    SPIDER_EYES_TEXTURE_REF,
    ENDERMAN_TEXTURE_REF,
    ENDERMAN_EYES_TEXTURE_REF,
    IRON_GOLEM_TEXTURE_REF,
    SNOW_GOLEM_TEXTURE_REF,
    WITCH_TEXTURE_REF,
    SLIME_TEXTURE_REF,
    MAGMA_CUBE_TEXTURE_REF,
    GHAST_TEXTURE_REF,
    BLAZE_TEXTURE_REF,
    ENDERMITE_TEXTURE_REF,
    SILVERFISH_TEXTURE_REF,
    PHANTOM_TEXTURE_REF,
    PHANTOM_EYES_TEXTURE_REF,
    PUFFERFISH_TEXTURE_REF,
    HAPPY_GHAST_TEXTURE_REF,
    MINECART_TEXTURE_REF,
    ARMOR_STAND_TEXTURE_REF,
    ZOMBIE_TEXTURE_REF,
    ZOMBIE_BABY_TEXTURE_REF,
    HUSK_TEXTURE_REF,
    HUSK_BABY_TEXTURE_REF,
    DROWNED_TEXTURE_REF,
    DROWNED_BABY_TEXTURE_REF,
    ZOMBIE_VILLAGER_TEXTURE_REF,
    ZOMBIE_VILLAGER_BABY_TEXTURE_REF,
    PIGLIN_TEXTURE_REF,
    PIGLIN_BABY_TEXTURE_REF,
    PIGLIN_BRUTE_TEXTURE_REF,
    ZOMBIFIED_PIGLIN_TEXTURE_REF,
    ZOMBIFIED_PIGLIN_BABY_TEXTURE_REF,
    EVOKER_TEXTURE_REF,
    ILLUSIONER_TEXTURE_REF,
    PILLAGER_TEXTURE_REF,
    VINDICATOR_TEXTURE_REF,
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
    LLAMA_CREAMY_TEXTURE_REF,
    LLAMA_CREAMY_BABY_TEXTURE_REF,
    LLAMA_WHITE_TEXTURE_REF,
    LLAMA_WHITE_BABY_TEXTURE_REF,
    LLAMA_BROWN_TEXTURE_REF,
    LLAMA_BROWN_BABY_TEXTURE_REF,
    LLAMA_GRAY_TEXTURE_REF,
    LLAMA_GRAY_BABY_TEXTURE_REF,
    CAMEL_TEXTURE_REF,
    CAMEL_BABY_TEXTURE_REF,
    CAMEL_HUSK_TEXTURE_REF,
    SQUID_TEXTURE_REF,
    SQUID_BABY_TEXTURE_REF,
    GLOW_SQUID_TEXTURE_REF,
    GLOW_SQUID_BABY_TEXTURE_REF,
    COD_TEXTURE_REF,
    SALMON_TEXTURE_REF,
    TROPICAL_FISH_SMALL_TEXTURE_REF,
    TROPICAL_FISH_LARGE_TEXTURE_REF,
    VEX_TEXTURE_REF,
    ALLAY_TEXTURE_REF,
    STRIDER_TEXTURE_REF,
    STRIDER_BABY_TEXTURE_REF,
    TURTLE_TEXTURE_REF,
    TURTLE_BABY_TEXTURE_REF,
    BAT_TEXTURE_REF,
    BEE_TEXTURE_REF,
    BEE_BABY_TEXTURE_REF,
    BREEZE_TEXTURE_REF,
    DOLPHIN_TEXTURE_REF,
    DOLPHIN_BABY_TEXTURE_REF,
    TROPICAL_FISH_KOB_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_SUNSTREAK_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_SNOOPER_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_DASHER_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_BRINELY_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_SPOTTY_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_FLOPPER_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_STRIPEY_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_GLITTER_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_BLOCKFISH_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_BETTY_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_CLAYFISH_PATTERN_TEXTURE_REF,
];

pub fn entity_model_texture_refs() -> &'static [EntityModelTextureRef] {
    &ENTITY_MODEL_TEXTURE_REFS
}

pub(in crate::entity_models) const COD_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [COD_TEXTURE_REF];

pub fn cod_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &COD_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const SALMON_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [SALMON_TEXTURE_REF];

pub fn salmon_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &SALMON_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const TROPICAL_FISH_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 14] = [
    TROPICAL_FISH_SMALL_TEXTURE_REF,
    TROPICAL_FISH_LARGE_TEXTURE_REF,
    TROPICAL_FISH_KOB_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_SUNSTREAK_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_SNOOPER_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_DASHER_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_BRINELY_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_SPOTTY_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_FLOPPER_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_STRIPEY_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_GLITTER_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_BLOCKFISH_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_BETTY_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_CLAYFISH_PATTERN_TEXTURE_REF,
];

pub fn tropical_fish_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &TROPICAL_FISH_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const VEX_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [VEX_TEXTURE_REF];

pub fn vex_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &VEX_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const ALLAY_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [ALLAY_TEXTURE_REF];

pub fn allay_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &ALLAY_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const STRIDER_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [STRIDER_TEXTURE_REF, STRIDER_BABY_TEXTURE_REF];

pub fn strider_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &STRIDER_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const TURTLE_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [TURTLE_TEXTURE_REF, TURTLE_BABY_TEXTURE_REF];

pub fn turtle_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &TURTLE_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const BAT_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [BAT_TEXTURE_REF];

pub fn bat_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &BAT_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const BEE_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [BEE_TEXTURE_REF, BEE_BABY_TEXTURE_REF];

pub fn bee_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &BEE_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const BREEZE_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [BREEZE_TEXTURE_REF];

pub fn breeze_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &BREEZE_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const DOLPHIN_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [DOLPHIN_TEXTURE_REF, DOLPHIN_BABY_TEXTURE_REF];

pub fn dolphin_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &DOLPHIN_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const SQUID_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 4] = [
    SQUID_TEXTURE_REF,
    SQUID_BABY_TEXTURE_REF,
    GLOW_SQUID_TEXTURE_REF,
    GLOW_SQUID_BABY_TEXTURE_REF,
];

pub fn squid_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &SQUID_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const CAMEL_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 3] = [
    CAMEL_TEXTURE_REF,
    CAMEL_BABY_TEXTURE_REF,
    CAMEL_HUSK_TEXTURE_REF,
];

pub fn camel_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &CAMEL_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const LLAMA_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 8] = [
    LLAMA_CREAMY_TEXTURE_REF,
    LLAMA_CREAMY_BABY_TEXTURE_REF,
    LLAMA_WHITE_TEXTURE_REF,
    LLAMA_WHITE_BABY_TEXTURE_REF,
    LLAMA_BROWN_TEXTURE_REF,
    LLAMA_BROWN_BABY_TEXTURE_REF,
    LLAMA_GRAY_TEXTURE_REF,
    LLAMA_GRAY_BABY_TEXTURE_REF,
];

pub fn llama_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &LLAMA_ENTITY_TEXTURE_REFS
}

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

pub(in crate::entity_models) const SQUID_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/squid/squid.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const SQUID_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/squid/squid_baby.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const GLOW_SQUID_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/squid/glow_squid.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const GLOW_SQUID_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/squid/glow_squid_baby.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const COD_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/fish/cod.png",
    size: [32, 32],
};

pub(in crate::entity_models) const SALMON_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fish/salmon.png",
        size: [32, 32],
    };

// Vanilla `TropicalFishRenderer` base textures: `tropical_a` for the small (kob) body
// and `tropical_b` for the large (flopper) body. Both are tinted per-entity by the base
// color; the twelve pattern overlays are separate textures (deferred).
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fish/tropical_a.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fish/tropical_b.png",
        size: [32, 32],
    };

// `TropicalFishPatternLayer` overlay textures (vanilla 26.1, all 32×32). The six `tropical_a`
// patterns ride the small (kob) body, the six `tropical_b` patterns the large (flopper) body;
// the trailing index is the pattern's `index + 1`.
pub(in crate::entity_models) const TROPICAL_FISH_KOB_PATTERN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fish/tropical_a_pattern_1.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const TROPICAL_FISH_SUNSTREAK_PATTERN_TEXTURE_REF:
    EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/fish/tropical_a_pattern_2.png",
    size: [32, 32],
};

pub(in crate::entity_models) const TROPICAL_FISH_SNOOPER_PATTERN_TEXTURE_REF:
    EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/fish/tropical_a_pattern_3.png",
    size: [32, 32],
};

pub(in crate::entity_models) const TROPICAL_FISH_DASHER_PATTERN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fish/tropical_a_pattern_4.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const TROPICAL_FISH_BRINELY_PATTERN_TEXTURE_REF:
    EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/fish/tropical_a_pattern_5.png",
    size: [32, 32],
};

pub(in crate::entity_models) const TROPICAL_FISH_SPOTTY_PATTERN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fish/tropical_a_pattern_6.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const TROPICAL_FISH_FLOPPER_PATTERN_TEXTURE_REF:
    EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/fish/tropical_b_pattern_1.png",
    size: [32, 32],
};

pub(in crate::entity_models) const TROPICAL_FISH_STRIPEY_PATTERN_TEXTURE_REF:
    EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/fish/tropical_b_pattern_2.png",
    size: [32, 32],
};

pub(in crate::entity_models) const TROPICAL_FISH_GLITTER_PATTERN_TEXTURE_REF:
    EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/fish/tropical_b_pattern_3.png",
    size: [32, 32],
};

pub(in crate::entity_models) const TROPICAL_FISH_BLOCKFISH_PATTERN_TEXTURE_REF:
    EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/fish/tropical_b_pattern_4.png",
    size: [32, 32],
};

pub(in crate::entity_models) const TROPICAL_FISH_BETTY_PATTERN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fish/tropical_b_pattern_5.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const TROPICAL_FISH_CLAYFISH_PATTERN_TEXTURE_REF:
    EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/fish/tropical_b_pattern_6.png",
    size: [32, 32],
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

pub(in crate::entity_models) const GOAT_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [GOAT_TEXTURE_REF, GOAT_BABY_TEXTURE_REF];

pub fn goat_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &GOAT_ENTITY_TEXTURE_REFS
}

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

pub(in crate::entity_models) const POLAR_BEAR_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [POLAR_BEAR_TEXTURE_REF, POLAR_BEAR_BABY_TEXTURE_REF];

pub fn polar_bear_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &POLAR_BEAR_ENTITY_TEXTURE_REFS
}

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

pub(in crate::entity_models) const SPIDER_EYES_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/spider/spider_eyes.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const SPIDER_EYES_LAYER_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [SPIDER_EYES_TEXTURE_REF];

pub(in crate::entity_models) const SPIDER_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 3] = [
    SPIDER_TEXTURE_REF,
    CAVE_SPIDER_TEXTURE_REF,
    SPIDER_EYES_TEXTURE_REF,
];

pub fn spider_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &SPIDER_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const ENDERMAN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/enderman/enderman.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const ENDERMAN_EYES_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/enderman/enderman_eyes.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const ENDERMAN_EYES_LAYER_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [ENDERMAN_EYES_TEXTURE_REF];

pub(in crate::entity_models) const ENDERMAN_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [ENDERMAN_TEXTURE_REF, ENDERMAN_EYES_TEXTURE_REF];

pub fn enderman_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &ENDERMAN_ENTITY_TEXTURE_REFS
}

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

#[cfg(test)]
pub(in crate::entity_models) const GOLEM_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [IRON_GOLEM_TEXTURE_REF, SNOW_GOLEM_TEXTURE_REF];

#[cfg(test)]
pub(in crate::entity_models) fn golem_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &GOLEM_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const WITCH_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/witch/witch.png",
        size: [64, 128],
    };

pub(in crate::entity_models) const WITCH_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [WITCH_TEXTURE_REF];

pub fn witch_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &WITCH_ENTITY_TEXTURE_REFS
}

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

pub(in crate::entity_models) const ILLAGER_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 4] = [
    EVOKER_TEXTURE_REF,
    ILLUSIONER_TEXTURE_REF,
    PILLAGER_TEXTURE_REF,
    VINDICATOR_TEXTURE_REF,
];

pub fn illager_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &ILLAGER_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const ARMOR_STAND_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/armorstand/armorstand.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const ARMOR_STAND_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [ARMOR_STAND_TEXTURE_REF];

pub fn armor_stand_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &ARMOR_STAND_ENTITY_TEXTURE_REFS
}

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

pub(in crate::entity_models) const SLIME_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [SLIME_TEXTURE_REF, MAGMA_CUBE_TEXTURE_REF];

pub fn slime_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &SLIME_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const GHAST_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/ghast/ghast.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const GHAST_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [GHAST_TEXTURE_REF];

pub fn ghast_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &GHAST_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const HAPPY_GHAST_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/ghast/happy_ghast.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HAPPY_GHAST_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [HAPPY_GHAST_TEXTURE_REF];

pub fn happy_ghast_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &HAPPY_GHAST_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const MINECART_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/minecart/minecart.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const MINECART_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [MINECART_TEXTURE_REF];

pub fn minecart_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &MINECART_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const BLAZE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/blaze/blaze.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const BLAZE_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [BLAZE_TEXTURE_REF];

pub fn blaze_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &BLAZE_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const ENDERMITE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/endermite/endermite.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const ENDERMITE_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [ENDERMITE_TEXTURE_REF];

pub fn endermite_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &ENDERMITE_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const SILVERFISH_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/silverfish/silverfish.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const SILVERFISH_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [SILVERFISH_TEXTURE_REF];

pub fn silverfish_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &SILVERFISH_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const PHANTOM_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/phantom/phantom.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const PHANTOM_EYES_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/phantom/phantom_eyes.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const PHANTOM_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [PHANTOM_TEXTURE_REF, PHANTOM_EYES_TEXTURE_REF];

pub fn phantom_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &PHANTOM_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const PUFFERFISH_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fish/pufferfish.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const PUFFERFISH_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [PUFFERFISH_TEXTURE_REF];

pub fn pufferfish_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &PUFFERFISH_ENTITY_TEXTURE_REFS
}
