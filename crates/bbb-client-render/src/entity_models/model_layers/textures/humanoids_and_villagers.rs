use super::super::super::EntityModelTextureRef;
use crate::entity_models::catalog::EntityDefaultPlayerSkin;
use crate::entity_models::temperature_animal_variants::{
    VANILLA_CHICKEN_VARIANT_TEXTURE_REFS, VANILLA_COW_VARIANT_TEXTURE_REFS,
    VANILLA_PIG_VARIANT_TEXTURE_REFS,
};

const fn player_default_skin_ref(path: &'static str) -> EntityModelTextureRef {
    EntityModelTextureRef {
        path,
        size: [64, 64],
    }
}

pub(in crate::entity_models) const PLAYER_SLIM_ALEX_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/slim/alex.png");
pub(in crate::entity_models) const PLAYER_SLIM_ARI_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/slim/ari.png");
pub(in crate::entity_models) const PLAYER_SLIM_EFE_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/slim/efe.png");
pub(in crate::entity_models) const PLAYER_SLIM_KAI_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/slim/kai.png");
pub(in crate::entity_models) const PLAYER_SLIM_MAKENA_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/slim/makena.png");
pub(in crate::entity_models) const PLAYER_SLIM_NOOR_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/slim/noor.png");
pub(in crate::entity_models) const PLAYER_SLIM_STEVE_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/slim/steve.png");
pub(in crate::entity_models) const PLAYER_SLIM_SUNNY_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/slim/sunny.png");
pub(in crate::entity_models) const PLAYER_SLIM_ZURI_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/slim/zuri.png");

pub(in crate::entity_models) const PLAYER_WIDE_ALEX_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/wide/alex.png");
pub(in crate::entity_models) const PLAYER_WIDE_ARI_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/wide/ari.png");
pub(in crate::entity_models) const PLAYER_WIDE_EFE_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/wide/efe.png");
pub(in crate::entity_models) const PLAYER_WIDE_KAI_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/wide/kai.png");
pub(in crate::entity_models) const PLAYER_WIDE_MAKENA_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/wide/makena.png");
pub(in crate::entity_models) const PLAYER_WIDE_NOOR_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/wide/noor.png");
pub(in crate::entity_models) const PLAYER_WIDE_STEVE_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/wide/steve.png");
pub(in crate::entity_models) const PLAYER_PROFILE_CAPE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "dynamic/player/cape",
        size: [64, 32],
    };
pub(in crate::entity_models) const PLAYER_PROFILE_ELYTRA_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "dynamic/player/elytra",
        size: [64, 32],
    };
pub(in crate::entity_models) const PLAYER_WIDE_SUNNY_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/wide/sunny.png");
pub(in crate::entity_models) const PLAYER_WIDE_ZURI_TEXTURE_REF: EntityModelTextureRef =
    player_default_skin_ref("textures/entity/player/wide/zuri.png");

pub(in crate::entity_models) const EXPERIENCE_ORB_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/experience/experience_orb.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const DRAGON_FIREBALL_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/enderdragon/dragon_fireball.png",
        size: [16, 16],
    };

pub(in crate::entity_models) const FISHING_HOOK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fishing/fishing_hook.png",
        size: [16, 16],
    };

pub(in crate::entity_models) fn default_player_skin_texture_ref(
    skin: EntityDefaultPlayerSkin,
) -> EntityModelTextureRef {
    match skin {
        EntityDefaultPlayerSkin::SlimAlex => PLAYER_SLIM_ALEX_TEXTURE_REF,
        EntityDefaultPlayerSkin::SlimAri => PLAYER_SLIM_ARI_TEXTURE_REF,
        EntityDefaultPlayerSkin::SlimEfe => PLAYER_SLIM_EFE_TEXTURE_REF,
        EntityDefaultPlayerSkin::SlimKai => PLAYER_SLIM_KAI_TEXTURE_REF,
        EntityDefaultPlayerSkin::SlimMakena => PLAYER_SLIM_MAKENA_TEXTURE_REF,
        EntityDefaultPlayerSkin::SlimNoor => PLAYER_SLIM_NOOR_TEXTURE_REF,
        EntityDefaultPlayerSkin::SlimSteve => PLAYER_SLIM_STEVE_TEXTURE_REF,
        EntityDefaultPlayerSkin::SlimSunny => PLAYER_SLIM_SUNNY_TEXTURE_REF,
        EntityDefaultPlayerSkin::SlimZuri => PLAYER_SLIM_ZURI_TEXTURE_REF,
        EntityDefaultPlayerSkin::WideAlex => PLAYER_WIDE_ALEX_TEXTURE_REF,
        EntityDefaultPlayerSkin::WideAri => PLAYER_WIDE_ARI_TEXTURE_REF,
        EntityDefaultPlayerSkin::WideEfe => PLAYER_WIDE_EFE_TEXTURE_REF,
        EntityDefaultPlayerSkin::WideKai => PLAYER_WIDE_KAI_TEXTURE_REF,
        EntityDefaultPlayerSkin::WideMakena => PLAYER_WIDE_MAKENA_TEXTURE_REF,
        EntityDefaultPlayerSkin::WideNoor => PLAYER_WIDE_NOOR_TEXTURE_REF,
        EntityDefaultPlayerSkin::WideSteve => PLAYER_WIDE_STEVE_TEXTURE_REF,
        EntityDefaultPlayerSkin::WideSunny => PLAYER_WIDE_SUNNY_TEXTURE_REF,
        EntityDefaultPlayerSkin::WideZuri => PLAYER_WIDE_ZURI_TEXTURE_REF,
    }
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

// Vanilla `DrownedOuterLayer` overlay texture (the adult white outer-layer copy).
pub(in crate::entity_models) const DROWNED_OUTER_LAYER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/zombie/drowned_outer_layer.png",
        size: [64, 64],
    };

// Vanilla baby `DrownedOuterLayer` overlay texture (the white baby outer-layer copy). Master atlas
// only, like the adult outer ref above.
pub(in crate::entity_models) const DROWNED_OUTER_LAYER_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/zombie/drowned_outer_layer_baby.png",
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

pub(in crate::entity_models) const ZOMBIE_VILLAGER_TYPE_TEXTURE_REFS: [EntityModelTextureRef; 7] = [
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/type/desert.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/type/jungle.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/type/plains.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/type/savanna.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/type/snow.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/type/swamp.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/type/taiga.png",
        size: [64, 64],
    },
];

pub(in crate::entity_models) const ZOMBIE_VILLAGER_BABY_TYPE_TEXTURE_REFS: [EntityModelTextureRef;
    7] = [
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/baby/desert.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/baby/jungle.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/baby/plains.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/baby/savanna.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/baby/snow.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/baby/swamp.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/baby/taiga.png",
        size: [64, 64],
    },
];

pub(in crate::entity_models) const ZOMBIE_VILLAGER_PROFESSION_TEXTURE_REFS:
    [EntityModelTextureRef; 14] = [
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession/armorer.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession/butcher.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession/cartographer.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession/cleric.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession/farmer.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession/fisherman.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession/fletcher.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession/leatherworker.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession/librarian.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession/mason.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession/nitwit.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession/shepherd.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession/toolsmith.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession/weaponsmith.png",
        size: [64, 64],
    },
];

pub(in crate::entity_models) const ZOMBIE_VILLAGER_LEVEL_TEXTURE_REFS: [EntityModelTextureRef; 5] = [
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession_level/stone.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession_level/iron.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession_level/gold.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession_level/emerald.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/zombie_villager/profession_level/diamond.png",
        size: [64, 64],
    },
];

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

// Vanilla `VexRenderer.getTextureLocation` textures: the idle `vex.png` and the `vex_charging.png`
// swap while the vex charges an attack (`isCharging`). The constant full-bright `getBlockLightLevel`
// glow is supplied by native entity light coords.
pub(in crate::entity_models) const VEX_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/illager/vex.png",
    size: [32, 32],
};

pub(in crate::entity_models) const VEX_CHARGING_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/illager/vex_charging.png",
        size: [32, 32],
    };

// Vanilla `AllayRenderer` texture. The constant full-bright `getBlockLightLevel` glow is supplied
// by native entity light coords.
pub(in crate::entity_models) const ALLAY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/allay/allay.png",
        size: [32, 32],
    };

// Vanilla `StriderRenderer` textures. The cold/suffocating swaps select the cold variants, and the
// `SimpleEquipmentLayer(STRIDER_SADDLE)` renders the adult model again with the equipment texture.
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
pub(in crate::entity_models) const STRIDER_SADDLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/strider_saddle/saddle.png",
        size: [64, 128],
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

// Vanilla `BeeRenderer.getTextureLocation`: the eight angry × nectar × baby texture variants.
// Adults are 64x64, babies 32x32 (`BabyBeeModel` reuses the adult mesh at half scale).
pub(in crate::entity_models) const BEE_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/bee/bee.png",
    size: [64, 64],
};
/// Vanilla `BeeStingerLayer.BEE_STINGER_LOCATION`; the model layer declares a 16×16 UV sheet.
pub(in crate::entity_models) const BEE_STINGER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/bee/bee_stinger.png",
        size: [16, 16],
    };
pub(in crate::entity_models) const BEE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/bee/bee_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const BEE_ANGRY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/bee/bee_angry.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const BEE_NECTAR_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/bee/bee_nectar.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const BEE_ANGRY_NECTAR_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/bee/bee_angry_nectar.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const BEE_ANGRY_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/bee/bee_angry_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const BEE_NECTAR_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/bee/bee_nectar_baby.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const BEE_ANGRY_NECTAR_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/bee/bee_angry_nectar_baby.png",
        size: [32, 32],
    };

/// Vanilla `BeeRenderer.getTextureLocation`: select the bee face from the `isAngry`, `hasNectar`
/// and `isBaby` render-state flags. The eight-way matrix is transcribed verbatim.
pub(in crate::entity_models) fn bee_texture_ref(
    baby: bool,
    angry: bool,
    has_nectar: bool,
) -> EntityModelTextureRef {
    match (angry, has_nectar, baby) {
        (true, true, true) => BEE_ANGRY_NECTAR_BABY_TEXTURE_REF,
        (true, true, false) => BEE_ANGRY_NECTAR_TEXTURE_REF,
        (true, false, true) => BEE_ANGRY_BABY_TEXTURE_REF,
        (true, false, false) => BEE_ANGRY_TEXTURE_REF,
        (false, true, true) => BEE_NECTAR_BABY_TEXTURE_REF,
        (false, true, false) => BEE_NECTAR_TEXTURE_REF,
        (false, false, true) => BEE_BABY_TEXTURE_REF,
        (false, false, false) => BEE_TEXTURE_REF,
    }
}

// Vanilla `BreezeRenderer` base body texture (the swirling wind layer / emissive eyes use the
// separate `breeze_wind.png` / `breeze_eyes.png`).
pub(in crate::entity_models) const BREEZE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/breeze/breeze.png",
        size: [32, 32],
    };
// Vanilla `BreezeEyesLayer`: the always-on emissive eye glow (`breeze_eyes.png`, re-rendering the
// model in the eyes render type; the texture is transparent except at the head's eye UVs).
pub(in crate::entity_models) const BREEZE_EYES_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/breeze/breeze_eyes.png",
        size: [32, 32],
    };
// Vanilla `BreezeWindLayer`: the swirling translucent wind body (`breeze_wind.png`, a 128×128 atlas —
// the `createWindLayer` retains only `wind_body`), drawn with the `breezeWind` render type whose U
// coordinate scrolls by `ageInTicks · 0.02`.
pub(in crate::entity_models) const BREEZE_WIND_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/breeze/breeze_wind.png",
        size: [128, 128],
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

pub(in crate::entity_models) const VILLAGER_TYPE_TEXTURE_REFS: [EntityModelTextureRef; 7] = [
    EntityModelTextureRef {
        path: "textures/entity/villager/type/desert.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/type/jungle.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/type/plains.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/type/savanna.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/type/snow.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/type/swamp.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/type/taiga.png",
        size: [64, 64],
    },
];

pub(in crate::entity_models) const VILLAGER_BABY_TYPE_TEXTURE_REFS: [EntityModelTextureRef; 7] = [
    EntityModelTextureRef {
        path: "textures/entity/villager/baby/desert.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/baby/jungle.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/baby/plains.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/baby/savanna.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/baby/snow.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/baby/swamp.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/baby/taiga.png",
        size: [64, 64],
    },
];

pub(in crate::entity_models) const VILLAGER_PROFESSION_TEXTURE_REFS: [EntityModelTextureRef; 14] = [
    EntityModelTextureRef {
        path: "textures/entity/villager/profession/armorer.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/profession/butcher.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/profession/cartographer.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/profession/cleric.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/profession/farmer.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/profession/fisherman.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/profession/fletcher.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/profession/leatherworker.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/profession/librarian.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/profession/mason.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/profession/nitwit.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/profession/shepherd.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/profession/toolsmith.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/profession/weaponsmith.png",
        size: [64, 64],
    },
];

pub(in crate::entity_models) const VILLAGER_LEVEL_TEXTURE_REFS: [EntityModelTextureRef; 5] = [
    EntityModelTextureRef {
        path: "textures/entity/villager/profession_level/stone.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/profession_level/iron.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/profession_level/gold.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/profession_level/emerald.png",
        size: [64, 64],
    },
    EntityModelTextureRef {
        path: "textures/entity/villager/profession_level/diamond.png",
        size: [64, 64],
    },
];

// Stable aliases keep focused renderer tests readable; the generated bootstrap catalog owns the
// model/texture presentation facts and official PNG dimensions.
pub(in crate::entity_models) const CHICKEN_TEMPERATE_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_CHICKEN_VARIANT_TEXTURE_REFS[0];
pub(in crate::entity_models) const CHICKEN_TEMPERATE_BABY_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_CHICKEN_VARIANT_TEXTURE_REFS[1];
pub(in crate::entity_models) const CHICKEN_WARM_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_CHICKEN_VARIANT_TEXTURE_REFS[2];
pub(in crate::entity_models) const CHICKEN_WARM_BABY_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_CHICKEN_VARIANT_TEXTURE_REFS[3];
pub(in crate::entity_models) const CHICKEN_COLD_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_CHICKEN_VARIANT_TEXTURE_REFS[4];
pub(in crate::entity_models) const CHICKEN_COLD_BABY_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_CHICKEN_VARIANT_TEXTURE_REFS[5];

pub(in crate::entity_models) const PIG_TEMPERATE_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_PIG_VARIANT_TEXTURE_REFS[0];
pub(in crate::entity_models) const PIG_TEMPERATE_BABY_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_PIG_VARIANT_TEXTURE_REFS[1];
pub(in crate::entity_models) const PIG_WARM_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_PIG_VARIANT_TEXTURE_REFS[2];
pub(in crate::entity_models) const PIG_WARM_BABY_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_PIG_VARIANT_TEXTURE_REFS[3];
pub(in crate::entity_models) const PIG_COLD_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_PIG_VARIANT_TEXTURE_REFS[4];
pub(in crate::entity_models) const PIG_COLD_BABY_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_PIG_VARIANT_TEXTURE_REFS[5];

pub(in crate::entity_models) const PIG_SADDLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/pig_saddle/saddle.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const COW_TEMPERATE_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_COW_VARIANT_TEXTURE_REFS[0];
pub(in crate::entity_models) const COW_TEMPERATE_BABY_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_COW_VARIANT_TEXTURE_REFS[1];
pub(in crate::entity_models) const COW_WARM_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_COW_VARIANT_TEXTURE_REFS[2];
pub(in crate::entity_models) const COW_WARM_BABY_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_COW_VARIANT_TEXTURE_REFS[3];
pub(in crate::entity_models) const COW_COLD_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_COW_VARIANT_TEXTURE_REFS[4];
pub(in crate::entity_models) const COW_COLD_BABY_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_COW_VARIANT_TEXTURE_REFS[5];

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

const fn wolf_body_armor_ref(path: &'static str) -> EntityModelTextureRef {
    EntityModelTextureRef {
        path,
        size: [64, 32],
    }
}

pub(in crate::entity_models) const WOLF_BODY_ARMADILLO_SCUTE_TEXTURE_REF: EntityModelTextureRef =
    wolf_body_armor_ref("textures/entity/equipment/wolf_body/armadillo_scute.png");
pub(in crate::entity_models) const WOLF_BODY_ARMADILLO_SCUTE_OVERLAY_TEXTURE_REF:
    EntityModelTextureRef =
    wolf_body_armor_ref("textures/entity/equipment/wolf_body/armadillo_scute_overlay.png");
pub(in crate::entity_models) const WOLF_ARMOR_CRACKINESS_LOW_TEXTURE_REF: EntityModelTextureRef =
    wolf_body_armor_ref("textures/entity/wolf/wolf_armor_crackiness_low.png");
pub(in crate::entity_models) const WOLF_ARMOR_CRACKINESS_MEDIUM_TEXTURE_REF: EntityModelTextureRef =
    wolf_body_armor_ref("textures/entity/wolf/wolf_armor_crackiness_medium.png");
pub(in crate::entity_models) const WOLF_ARMOR_CRACKINESS_HIGH_TEXTURE_REF: EntityModelTextureRef =
    wolf_body_armor_ref("textures/entity/wolf/wolf_armor_crackiness_high.png");
