use super::super::EntityModelTextureRef;
use crate::entity_models::catalog::{
    AxolotlModelVariant, CatModelVariant, EntityDyeColor, FoxModelVariant, FrogModelVariant,
    PandaModelVariant, ParrotModelVariant, RabbitModelVariant,
};

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
// Vanilla `StriderRenderer.getTextureLocation`: a suffocating (cold, off-lava shivering) strider
// swaps to the `strider_cold` texture × age.
pub(in crate::entity_models) const STRIDER_COLD_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/strider/strider_cold.png",
        size: [64, 128],
    };
pub(in crate::entity_models) const STRIDER_COLD_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/strider/strider_cold_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) fn strider_texture_ref(
    baby: bool,
    cold: bool,
) -> EntityModelTextureRef {
    match (cold, baby) {
        (false, false) => STRIDER_TEXTURE_REF,
        (false, true) => STRIDER_BABY_TEXTURE_REF,
        (true, false) => STRIDER_COLD_TEXTURE_REF,
        (true, true) => STRIDER_COLD_BABY_TEXTURE_REF,
    }
}

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

pub(in crate::entity_models) const ENTITY_MODEL_TEXTURE_REFS: [EntityModelTextureRef; 278] = [
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
    LEASH_KNOT_TEXTURE_REF,
    TRIDENT_TEXTURE_REF,
    EVOKER_FANGS_TEXTURE_REF,
    TADPOLE_TEXTURE_REF,
    CREAKING_TEXTURE_REF,
    CREAKING_EYES_TEXTURE_REF,
    SNIFFER_TEXTURE_REF,
    PARROT_RED_BLUE_TEXTURE_REF,
    PARROT_BLUE_TEXTURE_REF,
    PARROT_GREEN_TEXTURE_REF,
    PARROT_YELLOW_BLUE_TEXTURE_REF,
    PARROT_GRAY_TEXTURE_REF,
    SHULKER_TEXTURE_REF,
    SHULKER_WHITE_TEXTURE_REF,
    SHULKER_ORANGE_TEXTURE_REF,
    SHULKER_MAGENTA_TEXTURE_REF,
    SHULKER_LIGHT_BLUE_TEXTURE_REF,
    SHULKER_YELLOW_TEXTURE_REF,
    SHULKER_LIME_TEXTURE_REF,
    SHULKER_PINK_TEXTURE_REF,
    SHULKER_GRAY_TEXTURE_REF,
    SHULKER_LIGHT_GRAY_TEXTURE_REF,
    SHULKER_CYAN_TEXTURE_REF,
    SHULKER_PURPLE_TEXTURE_REF,
    SHULKER_BLUE_TEXTURE_REF,
    SHULKER_BROWN_TEXTURE_REF,
    SHULKER_GREEN_TEXTURE_REF,
    SHULKER_RED_TEXTURE_REF,
    SHULKER_BLACK_TEXTURE_REF,
    ENDER_DRAGON_TEXTURE_REF,
    ENDER_DRAGON_EYES_TEXTURE_REF,
    NAUTILUS_TEXTURE_REF,
    NAUTILUS_BABY_TEXTURE_REF,
    PANDA_NORMAL_TEXTURE_REF,
    PANDA_NORMAL_BABY_TEXTURE_REF,
    PANDA_LAZY_TEXTURE_REF,
    PANDA_LAZY_BABY_TEXTURE_REF,
    PANDA_WORRIED_TEXTURE_REF,
    PANDA_WORRIED_BABY_TEXTURE_REF,
    PANDA_PLAYFUL_TEXTURE_REF,
    PANDA_PLAYFUL_BABY_TEXTURE_REF,
    PANDA_BROWN_TEXTURE_REF,
    PANDA_BROWN_BABY_TEXTURE_REF,
    PANDA_WEAK_TEXTURE_REF,
    PANDA_WEAK_BABY_TEXTURE_REF,
    PANDA_AGGRESSIVE_TEXTURE_REF,
    PANDA_AGGRESSIVE_BABY_TEXTURE_REF,
    AXOLOTL_LUCY_TEXTURE_REF,
    AXOLOTL_LUCY_BABY_TEXTURE_REF,
    AXOLOTL_WILD_TEXTURE_REF,
    AXOLOTL_WILD_BABY_TEXTURE_REF,
    AXOLOTL_GOLD_TEXTURE_REF,
    AXOLOTL_GOLD_BABY_TEXTURE_REF,
    AXOLOTL_CYAN_TEXTURE_REF,
    AXOLOTL_CYAN_BABY_TEXTURE_REF,
    AXOLOTL_BLUE_TEXTURE_REF,
    AXOLOTL_BLUE_BABY_TEXTURE_REF,
    FOX_RED_TEXTURE_REF,
    FOX_RED_BABY_TEXTURE_REF,
    FOX_RED_SLEEP_TEXTURE_REF,
    FOX_RED_SLEEP_BABY_TEXTURE_REF,
    FOX_SNOW_TEXTURE_REF,
    FOX_SNOW_BABY_TEXTURE_REF,
    FOX_SNOW_SLEEP_TEXTURE_REF,
    FOX_SNOW_SLEEP_BABY_TEXTURE_REF,
    RABBIT_BROWN_TEXTURE_REF,
    RABBIT_BROWN_BABY_TEXTURE_REF,
    RABBIT_WHITE_TEXTURE_REF,
    RABBIT_WHITE_BABY_TEXTURE_REF,
    RABBIT_BLACK_TEXTURE_REF,
    RABBIT_BLACK_BABY_TEXTURE_REF,
    RABBIT_WHITE_SPLOTCHED_TEXTURE_REF,
    RABBIT_WHITE_SPLOTCHED_BABY_TEXTURE_REF,
    RABBIT_GOLD_TEXTURE_REF,
    RABBIT_GOLD_BABY_TEXTURE_REF,
    RABBIT_SALT_TEXTURE_REF,
    RABBIT_SALT_BABY_TEXTURE_REF,
    RABBIT_CAERBANNOG_TEXTURE_REF,
    RABBIT_CAERBANNOG_BABY_TEXTURE_REF,
    RABBIT_TOAST_TEXTURE_REF,
    RABBIT_TOAST_BABY_TEXTURE_REF,
    FELINE_CAT_TABBY_TEXTURE_REF,
    FELINE_CAT_TABBY_BABY_TEXTURE_REF,
    FELINE_CAT_BLACK_TEXTURE_REF,
    FELINE_CAT_BLACK_BABY_TEXTURE_REF,
    FELINE_CAT_RED_TEXTURE_REF,
    FELINE_CAT_RED_BABY_TEXTURE_REF,
    FELINE_CAT_SIAMESE_TEXTURE_REF,
    FELINE_CAT_SIAMESE_BABY_TEXTURE_REF,
    FELINE_CAT_BRITISH_SHORTHAIR_TEXTURE_REF,
    FELINE_CAT_BRITISH_SHORTHAIR_BABY_TEXTURE_REF,
    FELINE_CAT_CALICO_TEXTURE_REF,
    FELINE_CAT_CALICO_BABY_TEXTURE_REF,
    FELINE_CAT_PERSIAN_TEXTURE_REF,
    FELINE_CAT_PERSIAN_BABY_TEXTURE_REF,
    FELINE_CAT_RAGDOLL_TEXTURE_REF,
    FELINE_CAT_RAGDOLL_BABY_TEXTURE_REF,
    FELINE_CAT_WHITE_TEXTURE_REF,
    FELINE_CAT_WHITE_BABY_TEXTURE_REF,
    FELINE_CAT_JELLIE_TEXTURE_REF,
    FELINE_CAT_JELLIE_BABY_TEXTURE_REF,
    FELINE_CAT_ALL_BLACK_TEXTURE_REF,
    FELINE_CAT_ALL_BLACK_BABY_TEXTURE_REF,
    FELINE_OCELOT_TEXTURE_REF,
    FELINE_OCELOT_BABY_TEXTURE_REF,
    FELINE_CAT_COLLAR_TEXTURE_REF,
    FELINE_CAT_COLLAR_BABY_TEXTURE_REF,
    MOOSHROOM_TEXTURE_REF,
    MOOSHROOM_BABY_TEXTURE_REF,
    ARROW_TEXTURE_REF,
    LLAMA_SPIT_TEXTURE_REF,
    SHULKER_BULLET_TEXTURE_REF,
    WITHER_TEXTURE_REF,
    WITHER_INVULNERABLE_TEXTURE_REF,
    WIND_CHARGE_TEXTURE_REF,
    GUARDIAN_TEXTURE_REF,
    GUARDIAN_ELDER_TEXTURE_REF,
    WARDEN_TEXTURE_REF,
    FROG_TEMPERATE_TEXTURE_REF,
    FROG_WARM_TEXTURE_REF,
    FROG_COLD_TEXTURE_REF,
    ARMADILLO_TEXTURE_REF,
    ARMADILLO_BABY_TEXTURE_REF,
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
    STRIDER_COLD_TEXTURE_REF,
    STRIDER_COLD_BABY_TEXTURE_REF,
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

pub(in crate::entity_models) const STRIDER_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 4] = [
    STRIDER_TEXTURE_REF,
    STRIDER_BABY_TEXTURE_REF,
    STRIDER_COLD_TEXTURE_REF,
    STRIDER_COLD_BABY_TEXTURE_REF,
];

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

pub(in crate::entity_models) const LEASH_KNOT_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/lead_knot/lead_knot.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const LEASH_KNOT_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [LEASH_KNOT_TEXTURE_REF];

pub fn leash_knot_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &LEASH_KNOT_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const TRIDENT_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/trident/trident.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const TRIDENT_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [TRIDENT_TEXTURE_REF];

pub fn trident_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &TRIDENT_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const EVOKER_FANGS_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/illager/evoker_fangs.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const EVOKER_FANGS_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [EVOKER_FANGS_TEXTURE_REF];

pub fn evoker_fangs_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &EVOKER_FANGS_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const TADPOLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/tadpole/tadpole.png",
        size: [16, 16],
    };

pub(in crate::entity_models) const TADPOLE_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [TADPOLE_TEXTURE_REF];

pub fn tadpole_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &TADPOLE_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const CREAKING_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/creaking/creaking.png",
        size: [64, 64],
    };
// Vanilla `CreakingRenderer`'s `LivingEntityEmissiveLayer`: the glowing-eye overlay, shown when the
// creaking is active (`creaking_eyes.png`, re-rendering the whole model in the eyes render type).
pub(in crate::entity_models) const CREAKING_EYES_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/creaking/creaking_eyes.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const CREAKING_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [CREAKING_TEXTURE_REF, CREAKING_EYES_TEXTURE_REF];

pub fn creaking_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &CREAKING_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const SNIFFER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/sniffer/sniffer.png",
        size: [192, 192],
    };

pub(in crate::entity_models) const SNIFFER_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [SNIFFER_TEXTURE_REF];

pub fn sniffer_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &SNIFFER_ENTITY_TEXTURE_REFS
}

// The five `Parrot.Variant` colours (`ParrotRenderer.getVariantTexture`) share one `ParrotModel` and
// differ only by texture. `parrot_red_blue.png` is the vanilla `DEFAULT`; note the `GRAY` variant's
// file is the British-spelled `parrot_grey.png`.
pub(in crate::entity_models) const PARROT_RED_BLUE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/parrot/parrot_red_blue.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const PARROT_BLUE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/parrot/parrot_blue.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const PARROT_GREEN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/parrot/parrot_green.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const PARROT_YELLOW_BLUE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/parrot/parrot_yellow_blue.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const PARROT_GRAY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/parrot/parrot_grey.png",
        size: [32, 32],
    };

/// Vanilla `ParrotRenderer.getVariantTexture`: the parrot's colour texture.
pub(in crate::entity_models) fn parrot_texture_ref(
    variant: ParrotModelVariant,
) -> EntityModelTextureRef {
    match variant {
        ParrotModelVariant::RedBlue => PARROT_RED_BLUE_TEXTURE_REF,
        ParrotModelVariant::Blue => PARROT_BLUE_TEXTURE_REF,
        ParrotModelVariant::Green => PARROT_GREEN_TEXTURE_REF,
        ParrotModelVariant::YellowBlue => PARROT_YELLOW_BLUE_TEXTURE_REF,
        ParrotModelVariant::Gray => PARROT_GRAY_TEXTURE_REF,
    }
}

pub(in crate::entity_models) const PARROT_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 5] = [
    PARROT_RED_BLUE_TEXTURE_REF,
    PARROT_BLUE_TEXTURE_REF,
    PARROT_GREEN_TEXTURE_REF,
    PARROT_YELLOW_BLUE_TEXTURE_REF,
    PARROT_GRAY_TEXTURE_REF,
];

pub fn parrot_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &PARROT_ENTITY_TEXTURE_REFS
}

// bbb's `EntityModelKind::Shulker` is a unit variant (no dye colour), so only the default (uncoloured)
// shulker texture is selectable; the 16 dyed shulker textures need an enum extension to wire.
// The shulker texture set (`ShulkerRenderer.getTextureLocation`): the default `shulker.png` (used by
// an uncolored shulker, `DATA_COLOR_ID == 16`) plus the sixteen dyed textures in `DyeColor` id order.
pub(in crate::entity_models) const SHULKER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/shulker.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const SHULKER_WHITE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/shulker_white.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const SHULKER_ORANGE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/shulker_orange.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const SHULKER_MAGENTA_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/shulker_magenta.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const SHULKER_LIGHT_BLUE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/shulker_light_blue.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const SHULKER_YELLOW_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/shulker_yellow.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const SHULKER_LIME_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/shulker_lime.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const SHULKER_PINK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/shulker_pink.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const SHULKER_GRAY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/shulker_gray.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const SHULKER_LIGHT_GRAY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/shulker_light_gray.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const SHULKER_CYAN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/shulker_cyan.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const SHULKER_PURPLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/shulker_purple.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const SHULKER_BLUE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/shulker_blue.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const SHULKER_BROWN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/shulker_brown.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const SHULKER_GREEN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/shulker_green.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const SHULKER_RED_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/shulker_red.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const SHULKER_BLACK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/shulker_black.png",
        size: [64, 64],
    };
pub(in crate::entity_models) fn shulker_texture_ref(
    color: Option<EntityDyeColor>,
) -> EntityModelTextureRef {
    match color {
        None => SHULKER_TEXTURE_REF,
        Some(EntityDyeColor::White) => SHULKER_WHITE_TEXTURE_REF,
        Some(EntityDyeColor::Orange) => SHULKER_ORANGE_TEXTURE_REF,
        Some(EntityDyeColor::Magenta) => SHULKER_MAGENTA_TEXTURE_REF,
        Some(EntityDyeColor::LightBlue) => SHULKER_LIGHT_BLUE_TEXTURE_REF,
        Some(EntityDyeColor::Yellow) => SHULKER_YELLOW_TEXTURE_REF,
        Some(EntityDyeColor::Lime) => SHULKER_LIME_TEXTURE_REF,
        Some(EntityDyeColor::Pink) => SHULKER_PINK_TEXTURE_REF,
        Some(EntityDyeColor::Gray) => SHULKER_GRAY_TEXTURE_REF,
        Some(EntityDyeColor::LightGray) => SHULKER_LIGHT_GRAY_TEXTURE_REF,
        Some(EntityDyeColor::Cyan) => SHULKER_CYAN_TEXTURE_REF,
        Some(EntityDyeColor::Purple) => SHULKER_PURPLE_TEXTURE_REF,
        Some(EntityDyeColor::Blue) => SHULKER_BLUE_TEXTURE_REF,
        Some(EntityDyeColor::Brown) => SHULKER_BROWN_TEXTURE_REF,
        Some(EntityDyeColor::Green) => SHULKER_GREEN_TEXTURE_REF,
        Some(EntityDyeColor::Red) => SHULKER_RED_TEXTURE_REF,
        Some(EntityDyeColor::Black) => SHULKER_BLACK_TEXTURE_REF,
    }
}

pub(in crate::entity_models) const SHULKER_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 17] = [
    SHULKER_TEXTURE_REF,
    SHULKER_WHITE_TEXTURE_REF,
    SHULKER_ORANGE_TEXTURE_REF,
    SHULKER_MAGENTA_TEXTURE_REF,
    SHULKER_LIGHT_BLUE_TEXTURE_REF,
    SHULKER_YELLOW_TEXTURE_REF,
    SHULKER_LIME_TEXTURE_REF,
    SHULKER_PINK_TEXTURE_REF,
    SHULKER_GRAY_TEXTURE_REF,
    SHULKER_LIGHT_GRAY_TEXTURE_REF,
    SHULKER_CYAN_TEXTURE_REF,
    SHULKER_PURPLE_TEXTURE_REF,
    SHULKER_BLUE_TEXTURE_REF,
    SHULKER_BROWN_TEXTURE_REF,
    SHULKER_GREEN_TEXTURE_REF,
    SHULKER_RED_TEXTURE_REF,
    SHULKER_BLACK_TEXTURE_REF,
];

pub fn shulker_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &SHULKER_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const ENDER_DRAGON_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/enderdragon/dragon.png",
        size: [256, 256],
    };
// Vanilla `EnderDragonRenderer.EYES` (`RenderTypes.eyes`): the always-on emissive eye overlay,
// re-rendering the whole model with `dragon_eyes.png`.
pub(in crate::entity_models) const ENDER_DRAGON_EYES_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/enderdragon/dragon_eyes.png",
        size: [256, 256],
    };

pub(in crate::entity_models) const ENDER_DRAGON_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [ENDER_DRAGON_TEXTURE_REF, ENDER_DRAGON_EYES_TEXTURE_REF];

pub fn ender_dragon_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &ENDER_DRAGON_ENTITY_TEXTURE_REFS
}

// Baby-pair entities: bbb's `EntityModelKind` carries the `baby` flag, selecting the adult or baby
// texture (the baby has its own smaller atlas + body layer). The colour/breed variants (axolotl
// colour, fox snow, rabbit colour) need a further enum extension; only the default is wired here.
pub(in crate::entity_models) const NAUTILUS_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/nautilus/nautilus.png",
        size: [128, 128],
    };
pub(in crate::entity_models) const NAUTILUS_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/nautilus/nautilus_baby.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const NAUTILUS_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [NAUTILUS_TEXTURE_REF, NAUTILUS_BABY_TEXTURE_REF];
pub fn nautilus_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &NAUTILUS_ENTITY_TEXTURE_REFS
}

// The seven panda genes × age (`PandaRenderer.TEXTURES` / `BABY_TEXTURES`); all share the 64×64
// atlas. Note the inconsistent vanilla baby filenames (`panda_baby.png` but `lazy_panda_baby.png`).
pub(in crate::entity_models) const PANDA_NORMAL_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/panda/panda.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const PANDA_NORMAL_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/panda/panda_baby.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const PANDA_LAZY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/panda/panda_lazy.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const PANDA_LAZY_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/panda/lazy_panda_baby.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const PANDA_WORRIED_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/panda/panda_worried.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const PANDA_WORRIED_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/panda/worried_panda_baby.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const PANDA_PLAYFUL_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/panda/panda_playful.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const PANDA_PLAYFUL_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/panda/playful_panda_baby.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const PANDA_BROWN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/panda/panda_brown.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const PANDA_BROWN_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/panda/brown_panda_baby.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const PANDA_WEAK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/panda/panda_weak.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const PANDA_WEAK_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/panda/weak_panda_baby.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const PANDA_AGGRESSIVE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/panda/panda_aggressive.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const PANDA_AGGRESSIVE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/panda/aggressive_panda_baby.png",
        size: [64, 64],
    };
/// `PandaRenderer.getTextureLocation`: the displayed `Panda.Gene` × `isBaby` selects the texture,
/// defaulting (here folded into [`PandaModelVariant::from_id`]) to the `NORMAL` gene.
pub(in crate::entity_models) fn panda_texture_ref(
    variant: PandaModelVariant,
    baby: bool,
) -> EntityModelTextureRef {
    match (variant, baby) {
        (PandaModelVariant::Normal, false) => PANDA_NORMAL_TEXTURE_REF,
        (PandaModelVariant::Normal, true) => PANDA_NORMAL_BABY_TEXTURE_REF,
        (PandaModelVariant::Lazy, false) => PANDA_LAZY_TEXTURE_REF,
        (PandaModelVariant::Lazy, true) => PANDA_LAZY_BABY_TEXTURE_REF,
        (PandaModelVariant::Worried, false) => PANDA_WORRIED_TEXTURE_REF,
        (PandaModelVariant::Worried, true) => PANDA_WORRIED_BABY_TEXTURE_REF,
        (PandaModelVariant::Playful, false) => PANDA_PLAYFUL_TEXTURE_REF,
        (PandaModelVariant::Playful, true) => PANDA_PLAYFUL_BABY_TEXTURE_REF,
        (PandaModelVariant::Brown, false) => PANDA_BROWN_TEXTURE_REF,
        (PandaModelVariant::Brown, true) => PANDA_BROWN_BABY_TEXTURE_REF,
        (PandaModelVariant::Weak, false) => PANDA_WEAK_TEXTURE_REF,
        (PandaModelVariant::Weak, true) => PANDA_WEAK_BABY_TEXTURE_REF,
        (PandaModelVariant::Aggressive, false) => PANDA_AGGRESSIVE_TEXTURE_REF,
        (PandaModelVariant::Aggressive, true) => PANDA_AGGRESSIVE_BABY_TEXTURE_REF,
    }
}
pub(in crate::entity_models) const PANDA_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 14] = [
    PANDA_NORMAL_TEXTURE_REF,
    PANDA_NORMAL_BABY_TEXTURE_REF,
    PANDA_LAZY_TEXTURE_REF,
    PANDA_LAZY_BABY_TEXTURE_REF,
    PANDA_WORRIED_TEXTURE_REF,
    PANDA_WORRIED_BABY_TEXTURE_REF,
    PANDA_PLAYFUL_TEXTURE_REF,
    PANDA_PLAYFUL_BABY_TEXTURE_REF,
    PANDA_BROWN_TEXTURE_REF,
    PANDA_BROWN_BABY_TEXTURE_REF,
    PANDA_WEAK_TEXTURE_REF,
    PANDA_WEAK_BABY_TEXTURE_REF,
    PANDA_AGGRESSIVE_TEXTURE_REF,
    PANDA_AGGRESSIVE_BABY_TEXTURE_REF,
];
pub fn panda_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &PANDA_ENTITY_TEXTURE_REFS
}

// The axolotl texture matrix (`AxolotlRenderer.TEXTURE_BY_TYPE`): the five `Axolotl.Variant` colours
// × {adult 64×64, baby 32×32}. `getTextureLocation` picks `axolotl_<name>.png` /
// `axolotl_<name>_baby.png` from the variant and `isBaby`.
pub(in crate::entity_models) const AXOLOTL_LUCY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/axolotl/axolotl_lucy.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const AXOLOTL_LUCY_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/axolotl/axolotl_lucy_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const AXOLOTL_WILD_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/axolotl/axolotl_wild.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const AXOLOTL_WILD_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/axolotl/axolotl_wild_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const AXOLOTL_GOLD_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/axolotl/axolotl_gold.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const AXOLOTL_GOLD_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/axolotl/axolotl_gold_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const AXOLOTL_CYAN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/axolotl/axolotl_cyan.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const AXOLOTL_CYAN_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/axolotl/axolotl_cyan_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const AXOLOTL_BLUE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/axolotl/axolotl_blue.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const AXOLOTL_BLUE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/axolotl/axolotl_blue_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) fn axolotl_texture_ref(
    variant: AxolotlModelVariant,
    baby: bool,
) -> EntityModelTextureRef {
    match (variant, baby) {
        (AxolotlModelVariant::Lucy, false) => AXOLOTL_LUCY_TEXTURE_REF,
        (AxolotlModelVariant::Lucy, true) => AXOLOTL_LUCY_BABY_TEXTURE_REF,
        (AxolotlModelVariant::Wild, false) => AXOLOTL_WILD_TEXTURE_REF,
        (AxolotlModelVariant::Wild, true) => AXOLOTL_WILD_BABY_TEXTURE_REF,
        (AxolotlModelVariant::Gold, false) => AXOLOTL_GOLD_TEXTURE_REF,
        (AxolotlModelVariant::Gold, true) => AXOLOTL_GOLD_BABY_TEXTURE_REF,
        (AxolotlModelVariant::Cyan, false) => AXOLOTL_CYAN_TEXTURE_REF,
        (AxolotlModelVariant::Cyan, true) => AXOLOTL_CYAN_BABY_TEXTURE_REF,
        (AxolotlModelVariant::Blue, false) => AXOLOTL_BLUE_TEXTURE_REF,
        (AxolotlModelVariant::Blue, true) => AXOLOTL_BLUE_BABY_TEXTURE_REF,
    }
}
pub(in crate::entity_models) const AXOLOTL_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 10] = [
    AXOLOTL_LUCY_TEXTURE_REF,
    AXOLOTL_LUCY_BABY_TEXTURE_REF,
    AXOLOTL_WILD_TEXTURE_REF,
    AXOLOTL_WILD_BABY_TEXTURE_REF,
    AXOLOTL_GOLD_TEXTURE_REF,
    AXOLOTL_GOLD_BABY_TEXTURE_REF,
    AXOLOTL_CYAN_TEXTURE_REF,
    AXOLOTL_CYAN_BABY_TEXTURE_REF,
    AXOLOTL_BLUE_TEXTURE_REF,
    AXOLOTL_BLUE_BABY_TEXTURE_REF,
];
pub fn axolotl_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &AXOLOTL_ENTITY_TEXTURE_REFS
}

// The fox texture matrix (`FoxRenderer.TEXTURES_BY_VARIANT`): {red, snow} × {adult 48×32, baby
// 32×32} × {idle, sleeping}. `getTextureLocation` picks the cell from the variant, `isBaby`, and
// `isSleeping`. All eight share the `AdultFoxModel` / `BabyFoxModel` UV layout.
pub(in crate::entity_models) const FOX_RED_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fox/fox.png",
        size: [48, 32],
    };
pub(in crate::entity_models) const FOX_RED_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fox/fox_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const FOX_RED_SLEEP_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fox/fox_sleep.png",
        size: [48, 32],
    };
pub(in crate::entity_models) const FOX_RED_SLEEP_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fox/fox_sleep_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const FOX_SNOW_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fox/fox_snow.png",
        size: [48, 32],
    };
pub(in crate::entity_models) const FOX_SNOW_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fox/fox_snow_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const FOX_SNOW_SLEEP_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fox/fox_snow_sleep.png",
        size: [48, 32],
    };
pub(in crate::entity_models) const FOX_SNOW_SLEEP_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fox/fox_snow_sleep_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) fn fox_texture_ref(
    variant: FoxModelVariant,
    baby: bool,
    sleeping: bool,
) -> EntityModelTextureRef {
    match (variant, baby, sleeping) {
        (FoxModelVariant::Red, false, false) => FOX_RED_TEXTURE_REF,
        (FoxModelVariant::Red, true, false) => FOX_RED_BABY_TEXTURE_REF,
        (FoxModelVariant::Red, false, true) => FOX_RED_SLEEP_TEXTURE_REF,
        (FoxModelVariant::Red, true, true) => FOX_RED_SLEEP_BABY_TEXTURE_REF,
        (FoxModelVariant::Snow, false, false) => FOX_SNOW_TEXTURE_REF,
        (FoxModelVariant::Snow, true, false) => FOX_SNOW_BABY_TEXTURE_REF,
        (FoxModelVariant::Snow, false, true) => FOX_SNOW_SLEEP_TEXTURE_REF,
        (FoxModelVariant::Snow, true, true) => FOX_SNOW_SLEEP_BABY_TEXTURE_REF,
    }
}
pub(in crate::entity_models) const FOX_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 8] = [
    FOX_RED_TEXTURE_REF,
    FOX_RED_BABY_TEXTURE_REF,
    FOX_RED_SLEEP_TEXTURE_REF,
    FOX_RED_SLEEP_BABY_TEXTURE_REF,
    FOX_SNOW_TEXTURE_REF,
    FOX_SNOW_BABY_TEXTURE_REF,
    FOX_SNOW_SLEEP_TEXTURE_REF,
    FOX_SNOW_SLEEP_BABY_TEXTURE_REF,
];
pub fn fox_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &FOX_ENTITY_TEXTURE_REFS
}

// The rabbit texture matrix (`RabbitRenderer.RABBIT_LOCATIONS` / `BABY_RABBIT_LOCATIONS`): the seven
// `Rabbit.Variant` colours × {adult 64×64, baby 32×32}, plus the `Toast` named-rabbit override
// (`getTextureLocation` returns `toast`/`toast_baby` when `checkMagicName(entity, "Toast")`). EVIL
// uses the `caerbannog` texture.
pub(in crate::entity_models) const RABBIT_BROWN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/rabbit/rabbit_brown.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const RABBIT_BROWN_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/rabbit/rabbit_brown_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const RABBIT_WHITE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/rabbit/rabbit_white.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const RABBIT_WHITE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/rabbit/rabbit_white_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const RABBIT_BLACK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/rabbit/rabbit_black.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const RABBIT_BLACK_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/rabbit/rabbit_black_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const RABBIT_WHITE_SPLOTCHED_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/rabbit/rabbit_white_splotched.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const RABBIT_WHITE_SPLOTCHED_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/rabbit/rabbit_white_splotched_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const RABBIT_GOLD_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/rabbit/rabbit_gold.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const RABBIT_GOLD_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/rabbit/rabbit_gold_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const RABBIT_SALT_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/rabbit/rabbit_salt.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const RABBIT_SALT_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/rabbit/rabbit_salt_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const RABBIT_CAERBANNOG_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/rabbit/rabbit_caerbannog.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const RABBIT_CAERBANNOG_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/rabbit/rabbit_caerbannog_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const RABBIT_TOAST_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/rabbit/rabbit_toast.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const RABBIT_TOAST_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/rabbit/rabbit_toast_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) fn rabbit_texture_ref(
    variant: RabbitModelVariant,
    baby: bool,
    toast: bool,
) -> EntityModelTextureRef {
    if toast {
        return if baby {
            RABBIT_TOAST_BABY_TEXTURE_REF
        } else {
            RABBIT_TOAST_TEXTURE_REF
        };
    }
    match (variant, baby) {
        (RabbitModelVariant::Brown, false) => RABBIT_BROWN_TEXTURE_REF,
        (RabbitModelVariant::Brown, true) => RABBIT_BROWN_BABY_TEXTURE_REF,
        (RabbitModelVariant::White, false) => RABBIT_WHITE_TEXTURE_REF,
        (RabbitModelVariant::White, true) => RABBIT_WHITE_BABY_TEXTURE_REF,
        (RabbitModelVariant::Black, false) => RABBIT_BLACK_TEXTURE_REF,
        (RabbitModelVariant::Black, true) => RABBIT_BLACK_BABY_TEXTURE_REF,
        (RabbitModelVariant::WhiteSplotched, false) => RABBIT_WHITE_SPLOTCHED_TEXTURE_REF,
        (RabbitModelVariant::WhiteSplotched, true) => RABBIT_WHITE_SPLOTCHED_BABY_TEXTURE_REF,
        (RabbitModelVariant::Gold, false) => RABBIT_GOLD_TEXTURE_REF,
        (RabbitModelVariant::Gold, true) => RABBIT_GOLD_BABY_TEXTURE_REF,
        (RabbitModelVariant::Salt, false) => RABBIT_SALT_TEXTURE_REF,
        (RabbitModelVariant::Salt, true) => RABBIT_SALT_BABY_TEXTURE_REF,
        (RabbitModelVariant::Evil, false) => RABBIT_CAERBANNOG_TEXTURE_REF,
        (RabbitModelVariant::Evil, true) => RABBIT_CAERBANNOG_BABY_TEXTURE_REF,
    }
}
pub(in crate::entity_models) const RABBIT_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 16] = [
    RABBIT_BROWN_TEXTURE_REF,
    RABBIT_BROWN_BABY_TEXTURE_REF,
    RABBIT_WHITE_TEXTURE_REF,
    RABBIT_WHITE_BABY_TEXTURE_REF,
    RABBIT_BLACK_TEXTURE_REF,
    RABBIT_BLACK_BABY_TEXTURE_REF,
    RABBIT_WHITE_SPLOTCHED_TEXTURE_REF,
    RABBIT_WHITE_SPLOTCHED_BABY_TEXTURE_REF,
    RABBIT_GOLD_TEXTURE_REF,
    RABBIT_GOLD_BABY_TEXTURE_REF,
    RABBIT_SALT_TEXTURE_REF,
    RABBIT_SALT_BABY_TEXTURE_REF,
    RABBIT_CAERBANNOG_TEXTURE_REF,
    RABBIT_CAERBANNOG_BABY_TEXTURE_REF,
    RABBIT_TOAST_TEXTURE_REF,
    RABBIT_TOAST_BABY_TEXTURE_REF,
];
pub fn rabbit_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &RABBIT_ENTITY_TEXTURE_REFS
}

// The cat and ocelot share `AbstractFelineModel` (one UV layout per age), so the four refs differ only
// in the image: the cat's default `cat_tabby` variant (`CatRenderState.DEFAULT_TEXTURE`) and the
// ocelot, each with a 32×32 baby. The cat colour/breed variants need a further enum extension; only the
// default `cat_tabby` is wired here.
// The feline texture set: the eleven `CatVariant` breeds × {adult 64×32, baby 32×32}
// (`cat_<breed>.png` / `cat_<breed>_baby.png`, the variant's `assetInfo(isBaby)`), plus the two
// `ocelot` textures used by `OcelotRenderer`.
pub(in crate::entity_models) const FELINE_CAT_TABBY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_tabby.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_TABBY_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_tabby_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_BLACK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_black.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_BLACK_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_black_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_RED_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_red.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_RED_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_red_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_SIAMESE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_siamese.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_SIAMESE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_siamese_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_BRITISH_SHORTHAIR_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_british_shorthair.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_BRITISH_SHORTHAIR_BABY_TEXTURE_REF:
    EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/cat/cat_british_shorthair_baby.png",
    size: [32, 32],
};
pub(in crate::entity_models) const FELINE_CAT_CALICO_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_calico.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_CALICO_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_calico_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_PERSIAN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_persian.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_PERSIAN_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_persian_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_RAGDOLL_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_ragdoll.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_RAGDOLL_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_ragdoll_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_WHITE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_white.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_WHITE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_white_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_JELLIE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_jellie.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_JELLIE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_jellie_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_ALL_BLACK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_all_black.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_ALL_BLACK_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_all_black_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const FELINE_OCELOT_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/ocelot.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const FELINE_OCELOT_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/ocelot_baby.png",
        size: [32, 32],
    };
// Vanilla `CatCollarLayer`: the tame cat's collar overlay, tinted by the dye's diffuse color and
// sharing the feline mesh, so it carries the same 64×32 adult / 32×32 baby atlas as the cat body.
pub(in crate::entity_models) const FELINE_CAT_COLLAR_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_collar.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const FELINE_CAT_COLLAR_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cat/cat_collar_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) fn feline_collar_texture_ref(baby: bool) -> EntityModelTextureRef {
    if baby {
        FELINE_CAT_COLLAR_BABY_TEXTURE_REF
    } else {
        FELINE_CAT_COLLAR_TEXTURE_REF
    }
}
fn feline_cat_texture_ref(variant: CatModelVariant, baby: bool) -> EntityModelTextureRef {
    match (variant, baby) {
        (CatModelVariant::Tabby, false) => FELINE_CAT_TABBY_TEXTURE_REF,
        (CatModelVariant::Tabby, true) => FELINE_CAT_TABBY_BABY_TEXTURE_REF,
        (CatModelVariant::Black, false) => FELINE_CAT_BLACK_TEXTURE_REF,
        (CatModelVariant::Black, true) => FELINE_CAT_BLACK_BABY_TEXTURE_REF,
        (CatModelVariant::Red, false) => FELINE_CAT_RED_TEXTURE_REF,
        (CatModelVariant::Red, true) => FELINE_CAT_RED_BABY_TEXTURE_REF,
        (CatModelVariant::Siamese, false) => FELINE_CAT_SIAMESE_TEXTURE_REF,
        (CatModelVariant::Siamese, true) => FELINE_CAT_SIAMESE_BABY_TEXTURE_REF,
        (CatModelVariant::BritishShorthair, false) => FELINE_CAT_BRITISH_SHORTHAIR_TEXTURE_REF,
        (CatModelVariant::BritishShorthair, true) => FELINE_CAT_BRITISH_SHORTHAIR_BABY_TEXTURE_REF,
        (CatModelVariant::Calico, false) => FELINE_CAT_CALICO_TEXTURE_REF,
        (CatModelVariant::Calico, true) => FELINE_CAT_CALICO_BABY_TEXTURE_REF,
        (CatModelVariant::Persian, false) => FELINE_CAT_PERSIAN_TEXTURE_REF,
        (CatModelVariant::Persian, true) => FELINE_CAT_PERSIAN_BABY_TEXTURE_REF,
        (CatModelVariant::Ragdoll, false) => FELINE_CAT_RAGDOLL_TEXTURE_REF,
        (CatModelVariant::Ragdoll, true) => FELINE_CAT_RAGDOLL_BABY_TEXTURE_REF,
        (CatModelVariant::White, false) => FELINE_CAT_WHITE_TEXTURE_REF,
        (CatModelVariant::White, true) => FELINE_CAT_WHITE_BABY_TEXTURE_REF,
        (CatModelVariant::Jellie, false) => FELINE_CAT_JELLIE_TEXTURE_REF,
        (CatModelVariant::Jellie, true) => FELINE_CAT_JELLIE_BABY_TEXTURE_REF,
        (CatModelVariant::AllBlack, false) => FELINE_CAT_ALL_BLACK_TEXTURE_REF,
        (CatModelVariant::AllBlack, true) => FELINE_CAT_ALL_BLACK_BABY_TEXTURE_REF,
    }
}
pub(in crate::entity_models) fn feline_texture_ref(
    cat: bool,
    baby: bool,
    cat_variant: CatModelVariant,
) -> EntityModelTextureRef {
    match (cat, baby) {
        (true, _) => feline_cat_texture_ref(cat_variant, baby),
        (false, false) => FELINE_OCELOT_TEXTURE_REF,
        (false, true) => FELINE_OCELOT_BABY_TEXTURE_REF,
    }
}
pub(in crate::entity_models) const FELINE_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 26] = [
    FELINE_CAT_TABBY_TEXTURE_REF,
    FELINE_CAT_TABBY_BABY_TEXTURE_REF,
    FELINE_CAT_BLACK_TEXTURE_REF,
    FELINE_CAT_BLACK_BABY_TEXTURE_REF,
    FELINE_CAT_RED_TEXTURE_REF,
    FELINE_CAT_RED_BABY_TEXTURE_REF,
    FELINE_CAT_SIAMESE_TEXTURE_REF,
    FELINE_CAT_SIAMESE_BABY_TEXTURE_REF,
    FELINE_CAT_BRITISH_SHORTHAIR_TEXTURE_REF,
    FELINE_CAT_BRITISH_SHORTHAIR_BABY_TEXTURE_REF,
    FELINE_CAT_CALICO_TEXTURE_REF,
    FELINE_CAT_CALICO_BABY_TEXTURE_REF,
    FELINE_CAT_PERSIAN_TEXTURE_REF,
    FELINE_CAT_PERSIAN_BABY_TEXTURE_REF,
    FELINE_CAT_RAGDOLL_TEXTURE_REF,
    FELINE_CAT_RAGDOLL_BABY_TEXTURE_REF,
    FELINE_CAT_WHITE_TEXTURE_REF,
    FELINE_CAT_WHITE_BABY_TEXTURE_REF,
    FELINE_CAT_JELLIE_TEXTURE_REF,
    FELINE_CAT_JELLIE_BABY_TEXTURE_REF,
    FELINE_CAT_ALL_BLACK_TEXTURE_REF,
    FELINE_CAT_ALL_BLACK_BABY_TEXTURE_REF,
    FELINE_OCELOT_TEXTURE_REF,
    FELINE_OCELOT_BABY_TEXTURE_REF,
    FELINE_CAT_COLLAR_TEXTURE_REF,
    FELINE_CAT_COLLAR_BABY_TEXTURE_REF,
];
pub fn feline_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &FELINE_ENTITY_TEXTURE_REFS
}

// The mooshroom shares the cow model (`ModelLayers.MOOSHROOM` is the cow mesh), so it reuses the cow
// UV layout and only swaps in the mooshroom recolor. Vanilla `MushroomCowRenderer` keys the texture on
// the red/brown variant; bbb's `Mooshroom { baby }` carries no colour yet, so only the default red is
// wired (the brown variant and the `MushroomCowMushroomLayer` block-mushrooms stay deferred).
pub(in crate::entity_models) const MOOSHROOM_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cow/mooshroom_red.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const MOOSHROOM_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cow/mooshroom_red_baby.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const MOOSHROOM_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [MOOSHROOM_TEXTURE_REF, MOOSHROOM_BABY_TEXTURE_REF];
pub fn mooshroom_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &MOOSHROOM_ENTITY_TEXTURE_REFS
}

// Projectiles: small single-texture models. `WITHER_TEXTURE_REF` (wither.png) is shared by the wither
// skull and the wither boss (which also swaps in `WITHER_INVULNERABLE_TEXTURE_REF` mid-spawn); the
// tipped-arrow / spectral arrow variants stay deferred.
pub(in crate::entity_models) const ARROW_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/projectiles/arrow.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const ARROW_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [ARROW_TEXTURE_REF];
pub fn arrow_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &ARROW_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const LLAMA_SPIT_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/llama/llama_spit.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const LLAMA_SPIT_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [LLAMA_SPIT_TEXTURE_REF];
pub fn llama_spit_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &LLAMA_SPIT_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const SHULKER_BULLET_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/spark.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const SHULKER_BULLET_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [SHULKER_BULLET_TEXTURE_REF];
pub fn shulker_bullet_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &SHULKER_BULLET_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const WITHER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/wither/wither.png",
        size: [64, 64],
    };
// The wither boss swaps to this during its spawn charge (`WitherBossRenderer.getTextureLocation`).
pub(in crate::entity_models) const WITHER_INVULNERABLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/wither/wither_invulnerable.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const WITHER_SKULL_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [WITHER_TEXTURE_REF];
pub fn wither_skull_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &WITHER_SKULL_ENTITY_TEXTURE_REFS
}
// The wither boss covers both its normal and spawn-charge textures.
pub(in crate::entity_models) const WITHER_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [WITHER_TEXTURE_REF, WITHER_INVULNERABLE_TEXTURE_REF];
pub fn wither_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &WITHER_ENTITY_TEXTURE_REFS
}

pub(in crate::entity_models) const WIND_CHARGE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/projectiles/wind_charge.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const WIND_CHARGE_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [WIND_CHARGE_TEXTURE_REF];
pub fn wind_charge_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &WIND_CHARGE_ENTITY_TEXTURE_REFS
}

// The guardian and elder guardian share one mesh/UV layout, differing only by texture (and the elder's
// 2.35 root scale). The guardian's attack beam (guardian_beam.png) stays deferred.
pub(in crate::entity_models) const GUARDIAN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/guardian/guardian.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const GUARDIAN_ELDER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/guardian/guardian_elder.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const GUARDIAN_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [GUARDIAN_TEXTURE_REF, GUARDIAN_ELDER_TEXTURE_REF];
pub fn guardian_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &GUARDIAN_ENTITY_TEXTURE_REFS
}

// The warden's base body texture (atlas 128×128). The four emissive overlay layers (tendrils,
// heart, bioluminescent, pulsating spots) and the dig/emerge spawn animations stay deferred.
pub(in crate::entity_models) const WARDEN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/warden/warden.png",
        size: [128, 128],
    };
pub(in crate::entity_models) const WARDEN_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 1] =
    [WARDEN_TEXTURE_REF];
pub fn warden_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &WARDEN_ENTITY_TEXTURE_REFS
}

// The frog's three temperature textures (atlas 48×48). All share one `FrogModel` geometry/UV and
// differ only by texture; `FrogRenderer.getTextureLocation` reads the variant's `assetInfo` path.
pub(in crate::entity_models) const FROG_TEMPERATE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/frog/frog_temperate.png",
        size: [48, 48],
    };
pub(in crate::entity_models) const FROG_WARM_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/frog/frog_warm.png",
        size: [48, 48],
    };
pub(in crate::entity_models) const FROG_COLD_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/frog/frog_cold.png",
        size: [48, 48],
    };
pub(in crate::entity_models) fn frog_texture_ref(
    variant: FrogModelVariant,
) -> EntityModelTextureRef {
    match variant {
        FrogModelVariant::Temperate => FROG_TEMPERATE_TEXTURE_REF,
        FrogModelVariant::Warm => FROG_WARM_TEXTURE_REF,
        FrogModelVariant::Cold => FROG_COLD_TEXTURE_REF,
    }
}
pub(in crate::entity_models) const FROG_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 3] = [
    FROG_TEMPERATE_TEXTURE_REF,
    FROG_WARM_TEXTURE_REF,
    FROG_COLD_TEXTURE_REF,
];
pub fn frog_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &FROG_ENTITY_TEXTURE_REFS
}

// The armadillo is an `AgeableMobRenderer` two-model entity: the adult and baby share the same UV
// layout but bind their own 64×64 textures.
pub(in crate::entity_models) const ARMADILLO_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/armadillo/armadillo.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const ARMADILLO_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/armadillo/armadillo_baby.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const ARMADILLO_ENTITY_TEXTURE_REFS: [EntityModelTextureRef; 2] =
    [ARMADILLO_TEXTURE_REF, ARMADILLO_BABY_TEXTURE_REF];
pub fn armadillo_entity_texture_refs() -> &'static [EntityModelTextureRef] {
    &ARMADILLO_ENTITY_TEXTURE_REFS
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
