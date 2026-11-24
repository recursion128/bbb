use super::super::super::EntityModelTextureRef;
use crate::entity_models::catalog::{
    ChestModelHalf, ChestModelTexture, CopperGolemWeathering, EntityDyeColor, SignModelAttachment,
    SignModelWood,
};
use crate::entity_models::decorated_pot_patterns::*;
use crate::entity_models::DecoratedPotPattern;

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
pub(in crate::entity_models) const CAMEL_SADDLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/camel_saddle/saddle.png",
        size: [128, 128],
    };
pub(in crate::entity_models) const CAMEL_HUSK_SADDLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/camel_husk_saddle/saddle.png",
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

pub(in crate::entity_models) const LLAMA_BODY_WHITE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/white.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const LLAMA_BODY_ORANGE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/orange.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const LLAMA_BODY_MAGENTA_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/magenta.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const LLAMA_BODY_LIGHT_BLUE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/light_blue.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const LLAMA_BODY_YELLOW_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/yellow.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const LLAMA_BODY_LIME_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/lime.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const LLAMA_BODY_PINK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/pink.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const LLAMA_BODY_GRAY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/gray.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const LLAMA_BODY_LIGHT_GRAY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/light_gray.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const LLAMA_BODY_CYAN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/cyan.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const LLAMA_BODY_PURPLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/purple.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const LLAMA_BODY_BLUE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/blue.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const LLAMA_BODY_BROWN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/brown.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const LLAMA_BODY_GREEN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/green.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const LLAMA_BODY_RED_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/red.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const LLAMA_BODY_BLACK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/black.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const LLAMA_BODY_TRADER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/trader_llama.png",
        size: [128, 64],
    };
pub(in crate::entity_models) const LLAMA_BODY_TRADER_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/llama_body/trader_llama_baby.png",
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
// Vanilla `CreeperPowerLayer.POWER_LOCATION`: the charged-creeper energy-swirl overlay, scrolled and
// additively blended over the inflated `CREEPER_ARMOR` model.
pub(in crate::entity_models) const CREEPER_ARMOR_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/creeper/creeper_armor.png",
        size: [64, 32],
    };

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

pub(in crate::entity_models) const COPPER_GOLEM_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/copper_golem/copper_golem.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const COPPER_GOLEM_EXPOSED_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/copper_golem/copper_golem_exposed.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const COPPER_GOLEM_WEATHERED_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/copper_golem/copper_golem_weathered.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const COPPER_GOLEM_OXIDIZED_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/copper_golem/copper_golem_oxidized.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const COPPER_GOLEM_EYES_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/copper_golem/copper_golem_eyes.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const COPPER_GOLEM_EYES_EXPOSED_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/copper_golem/copper_golem_eyes_exposed.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const COPPER_GOLEM_EYES_WEATHERED_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/copper_golem/copper_golem_eyes_weathered.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const COPPER_GOLEM_EYES_OXIDIZED_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/copper_golem/copper_golem_eyes_oxidized.png",
        size: [64, 64],
    };

pub(in crate::entity_models) fn copper_golem_texture_ref(
    weathering: CopperGolemWeathering,
) -> EntityModelTextureRef {
    match weathering {
        CopperGolemWeathering::Unaffected => COPPER_GOLEM_TEXTURE_REF,
        CopperGolemWeathering::Exposed => COPPER_GOLEM_EXPOSED_TEXTURE_REF,
        CopperGolemWeathering::Weathered => COPPER_GOLEM_WEATHERED_TEXTURE_REF,
        CopperGolemWeathering::Oxidized => COPPER_GOLEM_OXIDIZED_TEXTURE_REF,
    }
}

pub(in crate::entity_models) fn copper_golem_eyes_texture_ref(
    weathering: CopperGolemWeathering,
) -> EntityModelTextureRef {
    match weathering {
        CopperGolemWeathering::Unaffected => COPPER_GOLEM_EYES_TEXTURE_REF,
        CopperGolemWeathering::Exposed => COPPER_GOLEM_EYES_EXPOSED_TEXTURE_REF,
        CopperGolemWeathering::Weathered => COPPER_GOLEM_EYES_WEATHERED_TEXTURE_REF,
        CopperGolemWeathering::Oxidized => COPPER_GOLEM_EYES_OXIDIZED_TEXTURE_REF,
    }
}

pub(in crate::entity_models) const IRON_GOLEM_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/iron_golem/iron_golem.png",
        size: [128, 128],
    };

// Vanilla `IronGolemCrackinessLayer`: the low/medium/high damage cracks overlaid (Cutout) on the
// base golem mesh, selected by `IronGolem.getCrackiness()` (`Crackiness.GOLEM.byFraction(health/max)`).
pub(in crate::entity_models) const IRON_GOLEM_CRACKINESS_LOW_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/iron_golem/iron_golem_crackiness_low.png",
        size: [128, 128],
    };
pub(in crate::entity_models) const IRON_GOLEM_CRACKINESS_MEDIUM_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/iron_golem/iron_golem_crackiness_medium.png",
        size: [128, 128],
    };
pub(in crate::entity_models) const IRON_GOLEM_CRACKINESS_HIGH_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/iron_golem/iron_golem_crackiness_high.png",
        size: [128, 128],
    };

pub(in crate::entity_models) const SNOW_GOLEM_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/snow_golem/snow_golem.png",
        size: [64, 64],
    };

// The base iron + snow golem faces used to build the per-entity test atlas. The iron golem's three
// `IronGolemCrackinessLayer` overlays are NOT listed here — they resolve through the global entity
// atlas (`ENTITY_MODEL_TEXTURE_REFS`, which carries all three), so this stays the small set the
// per-entity golem tests pack (keeping their pinned atlas layout stable).
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

pub(in crate::entity_models) const GHAST_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/ghast/ghast.png",
        size: [64, 32],
    };

/// Vanilla `GhastRenderer.GHAST_SHOOTING_LOCATION`: the open-mouth shooting face, drawn while
/// `isCharging`.
pub(in crate::entity_models) const GHAST_SHOOTING_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/ghast/ghast_shooting.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const HAPPY_GHAST_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/ghast/happy_ghast.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HAPPY_GHAST_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/ghast/happy_ghast_baby.png",
        size: [64, 64],
    };

/// Vanilla `HappyGhastRenderer.GHAST_ROPES`. The asset is physically 128x128 in the vanilla jar,
/// while `HappyGhastModel.createBodyLayer` bakes its UVs against a logical 64x64 layer definition.
pub(in crate::entity_models) const HAPPY_GHAST_ROPES_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/ghast/happy_ghast_ropes.png",
        size: [64, 64],
    };

macro_rules! happy_ghast_harness_texture_ref {
    ($name:ident, $color:literal) => {
        pub(in crate::entity_models) const $name: EntityModelTextureRef = EntityModelTextureRef {
            path: concat!(
                "textures/entity/equipment/happy_ghast_body/",
                $color,
                "_harness.png"
            ),
            size: [64, 64],
        };
    };
}

happy_ghast_harness_texture_ref!(HAPPY_GHAST_HARNESS_WHITE_TEXTURE_REF, "white");
happy_ghast_harness_texture_ref!(HAPPY_GHAST_HARNESS_ORANGE_TEXTURE_REF, "orange");
happy_ghast_harness_texture_ref!(HAPPY_GHAST_HARNESS_MAGENTA_TEXTURE_REF, "magenta");
happy_ghast_harness_texture_ref!(HAPPY_GHAST_HARNESS_LIGHT_BLUE_TEXTURE_REF, "light_blue");
happy_ghast_harness_texture_ref!(HAPPY_GHAST_HARNESS_YELLOW_TEXTURE_REF, "yellow");
happy_ghast_harness_texture_ref!(HAPPY_GHAST_HARNESS_LIME_TEXTURE_REF, "lime");
happy_ghast_harness_texture_ref!(HAPPY_GHAST_HARNESS_PINK_TEXTURE_REF, "pink");
happy_ghast_harness_texture_ref!(HAPPY_GHAST_HARNESS_GRAY_TEXTURE_REF, "gray");
happy_ghast_harness_texture_ref!(HAPPY_GHAST_HARNESS_LIGHT_GRAY_TEXTURE_REF, "light_gray");
happy_ghast_harness_texture_ref!(HAPPY_GHAST_HARNESS_CYAN_TEXTURE_REF, "cyan");
happy_ghast_harness_texture_ref!(HAPPY_GHAST_HARNESS_PURPLE_TEXTURE_REF, "purple");
happy_ghast_harness_texture_ref!(HAPPY_GHAST_HARNESS_BLUE_TEXTURE_REF, "blue");
happy_ghast_harness_texture_ref!(HAPPY_GHAST_HARNESS_BROWN_TEXTURE_REF, "brown");
happy_ghast_harness_texture_ref!(HAPPY_GHAST_HARNESS_GREEN_TEXTURE_REF, "green");
happy_ghast_harness_texture_ref!(HAPPY_GHAST_HARNESS_RED_TEXTURE_REF, "red");
happy_ghast_harness_texture_ref!(HAPPY_GHAST_HARNESS_BLACK_TEXTURE_REF, "black");

pub(in crate::entity_models) const MINECART_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/minecart/minecart.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const BLAZE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/blaze/blaze.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const ENDERMITE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/endermite/endermite.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const SILVERFISH_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/silverfish/silverfish.png",
        size: [64, 32],
    };

const fn chest_sheet_texture_ref(path: &'static str) -> EntityModelTextureRef {
    EntityModelTextureRef {
        path,
        size: [64, 64],
    }
}

// Vanilla `Sheets.CHEST_MAPPER` sprites (`entity/chest/<name>` on the 64×64 chest sheet), named per
// `ChestSpecialRenderer.createDefaultTextures(prefix)`: `<prefix>` / `<prefix>_left` /
// `<prefix>_right` for single/left/right, plus the always-single `ender`.
pub(in crate::entity_models) const CHEST_NORMAL_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/normal.png");
pub(in crate::entity_models) const CHEST_NORMAL_LEFT_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/normal_left.png");
pub(in crate::entity_models) const CHEST_NORMAL_RIGHT_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/normal_right.png");
pub(in crate::entity_models) const CHEST_TRAPPED_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/trapped.png");
pub(in crate::entity_models) const CHEST_TRAPPED_LEFT_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/trapped_left.png");
pub(in crate::entity_models) const CHEST_TRAPPED_RIGHT_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/trapped_right.png");
pub(in crate::entity_models) const CHEST_CHRISTMAS_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/christmas.png");
pub(in crate::entity_models) const CHEST_CHRISTMAS_LEFT_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/christmas_left.png");
pub(in crate::entity_models) const CHEST_CHRISTMAS_RIGHT_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/christmas_right.png");
pub(in crate::entity_models) const CHEST_ENDER_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/ender.png");
pub(in crate::entity_models) const CHEST_COPPER_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/copper.png");
pub(in crate::entity_models) const CHEST_COPPER_LEFT_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/copper_left.png");
pub(in crate::entity_models) const CHEST_COPPER_RIGHT_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/copper_right.png");
pub(in crate::entity_models) const CHEST_COPPER_EXPOSED_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/copper_exposed.png");
pub(in crate::entity_models) const CHEST_COPPER_EXPOSED_LEFT_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/copper_exposed_left.png");
pub(in crate::entity_models) const CHEST_COPPER_EXPOSED_RIGHT_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/copper_exposed_right.png");
pub(in crate::entity_models) const CHEST_COPPER_WEATHERED_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/copper_weathered.png");
pub(in crate::entity_models) const CHEST_COPPER_WEATHERED_LEFT_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/copper_weathered_left.png");
pub(in crate::entity_models) const CHEST_COPPER_WEATHERED_RIGHT_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/copper_weathered_right.png");
pub(in crate::entity_models) const CHEST_COPPER_OXIDIZED_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/copper_oxidized.png");
pub(in crate::entity_models) const CHEST_COPPER_OXIDIZED_LEFT_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/copper_oxidized_left.png");
pub(in crate::entity_models) const CHEST_COPPER_OXIDIZED_RIGHT_TEXTURE_REF: EntityModelTextureRef =
    chest_sheet_texture_ref("textures/entity/chest/copper_oxidized_right.png");

/// Vanilla `Sheets.chooseSprite(material, type)`: the ender chest is always the single `ender`
/// sprite; every other family selects `<prefix>` / `<prefix>_left` / `<prefix>_right` by
/// `ChestType` (`MultiblockChestResources.select`).
pub(in crate::entity_models) fn chest_texture_ref(
    texture: ChestModelTexture,
    half: ChestModelHalf,
) -> EntityModelTextureRef {
    let (single, left, right) = match texture {
        ChestModelTexture::Normal => (
            CHEST_NORMAL_TEXTURE_REF,
            CHEST_NORMAL_LEFT_TEXTURE_REF,
            CHEST_NORMAL_RIGHT_TEXTURE_REF,
        ),
        ChestModelTexture::Trapped => (
            CHEST_TRAPPED_TEXTURE_REF,
            CHEST_TRAPPED_LEFT_TEXTURE_REF,
            CHEST_TRAPPED_RIGHT_TEXTURE_REF,
        ),
        ChestModelTexture::Christmas => (
            CHEST_CHRISTMAS_TEXTURE_REF,
            CHEST_CHRISTMAS_LEFT_TEXTURE_REF,
            CHEST_CHRISTMAS_RIGHT_TEXTURE_REF,
        ),
        ChestModelTexture::Ender => (
            CHEST_ENDER_TEXTURE_REF,
            CHEST_ENDER_TEXTURE_REF,
            CHEST_ENDER_TEXTURE_REF,
        ),
        ChestModelTexture::Copper => (
            CHEST_COPPER_TEXTURE_REF,
            CHEST_COPPER_LEFT_TEXTURE_REF,
            CHEST_COPPER_RIGHT_TEXTURE_REF,
        ),
        ChestModelTexture::CopperExposed => (
            CHEST_COPPER_EXPOSED_TEXTURE_REF,
            CHEST_COPPER_EXPOSED_LEFT_TEXTURE_REF,
            CHEST_COPPER_EXPOSED_RIGHT_TEXTURE_REF,
        ),
        ChestModelTexture::CopperWeathered => (
            CHEST_COPPER_WEATHERED_TEXTURE_REF,
            CHEST_COPPER_WEATHERED_LEFT_TEXTURE_REF,
            CHEST_COPPER_WEATHERED_RIGHT_TEXTURE_REF,
        ),
        ChestModelTexture::CopperOxidized => (
            CHEST_COPPER_OXIDIZED_TEXTURE_REF,
            CHEST_COPPER_OXIDIZED_LEFT_TEXTURE_REF,
            CHEST_COPPER_OXIDIZED_RIGHT_TEXTURE_REF,
        ),
    };
    match half {
        ChestModelHalf::Single => single,
        ChestModelHalf::Left => left,
        ChestModelHalf::Right => right,
    }
}

const fn sign_sheet_texture_ref(path: &'static str) -> EntityModelTextureRef {
    EntityModelTextureRef {
        path,
        size: [64, 32],
    }
}

// Vanilla `Sheets.SIGN_MAPPER` / `HANGING_SIGN_MAPPER` sprites
// (`entity/signs/<wood>` and `entity/signs/hanging/<wood>` on the signs
// sheet, both 64×32 like the `createSignLayer` / `createHangingSignLayer`
// atlas), one per 26.1 `WoodType` with sign blocks.
pub(in crate::entity_models) const SIGN_OAK_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/oak.png");
pub(in crate::entity_models) const SIGN_SPRUCE_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/spruce.png");
pub(in crate::entity_models) const SIGN_BIRCH_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/birch.png");
pub(in crate::entity_models) const SIGN_ACACIA_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/acacia.png");
pub(in crate::entity_models) const SIGN_CHERRY_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/cherry.png");
pub(in crate::entity_models) const SIGN_JUNGLE_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/jungle.png");
pub(in crate::entity_models) const SIGN_DARK_OAK_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/dark_oak.png");
pub(in crate::entity_models) const SIGN_PALE_OAK_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/pale_oak.png");
pub(in crate::entity_models) const SIGN_CRIMSON_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/crimson.png");
pub(in crate::entity_models) const SIGN_WARPED_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/warped.png");
pub(in crate::entity_models) const SIGN_MANGROVE_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/mangrove.png");
pub(in crate::entity_models) const SIGN_BAMBOO_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/bamboo.png");
pub(in crate::entity_models) const HANGING_SIGN_OAK_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/hanging/oak.png");
pub(in crate::entity_models) const HANGING_SIGN_SPRUCE_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/hanging/spruce.png");
pub(in crate::entity_models) const HANGING_SIGN_BIRCH_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/hanging/birch.png");
pub(in crate::entity_models) const HANGING_SIGN_ACACIA_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/hanging/acacia.png");
pub(in crate::entity_models) const HANGING_SIGN_CHERRY_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/hanging/cherry.png");
pub(in crate::entity_models) const HANGING_SIGN_JUNGLE_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/hanging/jungle.png");
pub(in crate::entity_models) const HANGING_SIGN_DARK_OAK_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/hanging/dark_oak.png");
pub(in crate::entity_models) const HANGING_SIGN_PALE_OAK_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/hanging/pale_oak.png");
pub(in crate::entity_models) const HANGING_SIGN_CRIMSON_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/hanging/crimson.png");
pub(in crate::entity_models) const HANGING_SIGN_WARPED_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/hanging/warped.png");
pub(in crate::entity_models) const HANGING_SIGN_MANGROVE_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/hanging/mangrove.png");
pub(in crate::entity_models) const HANGING_SIGN_BAMBOO_TEXTURE_REF: EntityModelTextureRef =
    sign_sheet_texture_ref("textures/entity/signs/hanging/bamboo.png");

/// Vanilla `Sheets.getSignSprite(type)` / `getHangingSignSprite(type)`:
/// `entity/signs/<wood>` for the plain sign forms,
/// `entity/signs/hanging/<wood>` for the hanging forms.
pub(in crate::entity_models) fn sign_texture_ref(
    wood: SignModelWood,
    attachment: SignModelAttachment,
) -> EntityModelTextureRef {
    if attachment.is_hanging() {
        match wood {
            SignModelWood::Oak => HANGING_SIGN_OAK_TEXTURE_REF,
            SignModelWood::Spruce => HANGING_SIGN_SPRUCE_TEXTURE_REF,
            SignModelWood::Birch => HANGING_SIGN_BIRCH_TEXTURE_REF,
            SignModelWood::Acacia => HANGING_SIGN_ACACIA_TEXTURE_REF,
            SignModelWood::Cherry => HANGING_SIGN_CHERRY_TEXTURE_REF,
            SignModelWood::Jungle => HANGING_SIGN_JUNGLE_TEXTURE_REF,
            SignModelWood::DarkOak => HANGING_SIGN_DARK_OAK_TEXTURE_REF,
            SignModelWood::PaleOak => HANGING_SIGN_PALE_OAK_TEXTURE_REF,
            SignModelWood::Crimson => HANGING_SIGN_CRIMSON_TEXTURE_REF,
            SignModelWood::Warped => HANGING_SIGN_WARPED_TEXTURE_REF,
            SignModelWood::Mangrove => HANGING_SIGN_MANGROVE_TEXTURE_REF,
            SignModelWood::Bamboo => HANGING_SIGN_BAMBOO_TEXTURE_REF,
        }
    } else {
        match wood {
            SignModelWood::Oak => SIGN_OAK_TEXTURE_REF,
            SignModelWood::Spruce => SIGN_SPRUCE_TEXTURE_REF,
            SignModelWood::Birch => SIGN_BIRCH_TEXTURE_REF,
            SignModelWood::Acacia => SIGN_ACACIA_TEXTURE_REF,
            SignModelWood::Cherry => SIGN_CHERRY_TEXTURE_REF,
            SignModelWood::Jungle => SIGN_JUNGLE_TEXTURE_REF,
            SignModelWood::DarkOak => SIGN_DARK_OAK_TEXTURE_REF,
            SignModelWood::PaleOak => SIGN_PALE_OAK_TEXTURE_REF,
            SignModelWood::Crimson => SIGN_CRIMSON_TEXTURE_REF,
            SignModelWood::Warped => SIGN_WARPED_TEXTURE_REF,
            SignModelWood::Mangrove => SIGN_MANGROVE_TEXTURE_REF,
            SignModelWood::Bamboo => SIGN_BAMBOO_TEXTURE_REF,
        }
    }
}

const fn bed_sheet_texture_ref(path: &'static str) -> EntityModelTextureRef {
    EntityModelTextureRef {
        path,
        size: [64, 64],
    }
}

// Vanilla `Sheets.BED_MAPPER` sprites (`entity/bed/<color>` on the 64×64 bed sheet), named per
// `Sheets.colorToResourceSprite(color)` = `DyeColor.getName()`, one per vanilla dye color in
// `DyeColor` id order.
pub(in crate::entity_models) const BED_WHITE_TEXTURE_REF: EntityModelTextureRef =
    bed_sheet_texture_ref("textures/entity/bed/white.png");
pub(in crate::entity_models) const BED_ORANGE_TEXTURE_REF: EntityModelTextureRef =
    bed_sheet_texture_ref("textures/entity/bed/orange.png");
pub(in crate::entity_models) const BED_MAGENTA_TEXTURE_REF: EntityModelTextureRef =
    bed_sheet_texture_ref("textures/entity/bed/magenta.png");
pub(in crate::entity_models) const BED_LIGHT_BLUE_TEXTURE_REF: EntityModelTextureRef =
    bed_sheet_texture_ref("textures/entity/bed/light_blue.png");
pub(in crate::entity_models) const BED_YELLOW_TEXTURE_REF: EntityModelTextureRef =
    bed_sheet_texture_ref("textures/entity/bed/yellow.png");
pub(in crate::entity_models) const BED_LIME_TEXTURE_REF: EntityModelTextureRef =
    bed_sheet_texture_ref("textures/entity/bed/lime.png");
pub(in crate::entity_models) const BED_PINK_TEXTURE_REF: EntityModelTextureRef =
    bed_sheet_texture_ref("textures/entity/bed/pink.png");
pub(in crate::entity_models) const BED_GRAY_TEXTURE_REF: EntityModelTextureRef =
    bed_sheet_texture_ref("textures/entity/bed/gray.png");
pub(in crate::entity_models) const BED_LIGHT_GRAY_TEXTURE_REF: EntityModelTextureRef =
    bed_sheet_texture_ref("textures/entity/bed/light_gray.png");
pub(in crate::entity_models) const BED_CYAN_TEXTURE_REF: EntityModelTextureRef =
    bed_sheet_texture_ref("textures/entity/bed/cyan.png");
pub(in crate::entity_models) const BED_PURPLE_TEXTURE_REF: EntityModelTextureRef =
    bed_sheet_texture_ref("textures/entity/bed/purple.png");
pub(in crate::entity_models) const BED_BLUE_TEXTURE_REF: EntityModelTextureRef =
    bed_sheet_texture_ref("textures/entity/bed/blue.png");
pub(in crate::entity_models) const BED_BROWN_TEXTURE_REF: EntityModelTextureRef =
    bed_sheet_texture_ref("textures/entity/bed/brown.png");
pub(in crate::entity_models) const BED_GREEN_TEXTURE_REF: EntityModelTextureRef =
    bed_sheet_texture_ref("textures/entity/bed/green.png");
pub(in crate::entity_models) const BED_RED_TEXTURE_REF: EntityModelTextureRef =
    bed_sheet_texture_ref("textures/entity/bed/red.png");
pub(in crate::entity_models) const BED_BLACK_TEXTURE_REF: EntityModelTextureRef =
    bed_sheet_texture_ref("textures/entity/bed/black.png");

/// Vanilla `Sheets.getBedSprite(color)`: `BED_MAPPER.apply(colorToResourceSprite(color))` —
/// `entity/bed/<DyeColor.getName()>`, both bed halves sharing the one 64×64 sprite.
pub(in crate::entity_models) fn bed_texture_ref(color: EntityDyeColor) -> EntityModelTextureRef {
    match color {
        EntityDyeColor::White => BED_WHITE_TEXTURE_REF,
        EntityDyeColor::Orange => BED_ORANGE_TEXTURE_REF,
        EntityDyeColor::Magenta => BED_MAGENTA_TEXTURE_REF,
        EntityDyeColor::LightBlue => BED_LIGHT_BLUE_TEXTURE_REF,
        EntityDyeColor::Yellow => BED_YELLOW_TEXTURE_REF,
        EntityDyeColor::Lime => BED_LIME_TEXTURE_REF,
        EntityDyeColor::Pink => BED_PINK_TEXTURE_REF,
        EntityDyeColor::Gray => BED_GRAY_TEXTURE_REF,
        EntityDyeColor::LightGray => BED_LIGHT_GRAY_TEXTURE_REF,
        EntityDyeColor::Cyan => BED_CYAN_TEXTURE_REF,
        EntityDyeColor::Purple => BED_PURPLE_TEXTURE_REF,
        EntityDyeColor::Blue => BED_BLUE_TEXTURE_REF,
        EntityDyeColor::Brown => BED_BROWN_TEXTURE_REF,
        EntityDyeColor::Green => BED_GREEN_TEXTURE_REF,
        EntityDyeColor::Red => BED_RED_TEXTURE_REF,
        EntityDyeColor::Black => BED_BLACK_TEXTURE_REF,
    }
}

/// Vanilla `BellRenderer.BELL_TEXTURE`
/// (`Sheets.BLOCK_ENTITIES_MAPPER.defaultNamespaceApply("bell/bell_body")`), sized like the
/// `BellModel.createBodyLayer` 32×32 atlas.
pub(in crate::entity_models) const BELL_BODY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/bell/bell_body.png",
        size: [32, 32],
    };

/// Vanilla `EnchantTableRenderer.BOOK_TEXTURE`
/// (`Sheets.BLOCK_ENTITIES_MAPPER.defaultNamespaceApply("enchantment/enchanting_table_book")`),
/// sized like the `BookModel.createBodyLayer` 64×32 atlas. Shared by the enchanting-table
/// hovering book and the lectern's static open book (`LecternRenderer` binds the same sprite).
pub(in crate::entity_models) const BOOK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/enchantment/enchanting_table_book.png",
        size: [64, 32],
    };

/// Vanilla `ConduitRenderer` textures under `textures/entity/conduit/`, sized like the matching
/// `CONDUIT_*` model layers.
pub(in crate::entity_models) const CONDUIT_BASE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/conduit/base.png",
        size: [32, 16],
    };
pub(in crate::entity_models) const CONDUIT_CAGE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/conduit/cage.png",
        size: [32, 16],
    };
pub(in crate::entity_models) const CONDUIT_WIND_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/conduit/wind.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const CONDUIT_WIND_VERTICAL_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/conduit/wind_vertical.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const CONDUIT_OPEN_EYE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/conduit/open_eye.png",
        size: [16, 16],
    };
pub(in crate::entity_models) const CONDUIT_CLOSED_EYE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/conduit/closed_eye.png",
        size: [16, 16],
    };

/// Vanilla `Sheets.DECORATED_POT_BASE`
/// (`DECORATED_POT_MAPPER.defaultNamespaceApply("decorated_pot_base")`), sized like the
/// `DecoratedPotRenderer.createBaseLayer` 32×32 atlas.
pub(in crate::entity_models) const DECORATED_POT_BASE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/decorated_pot/decorated_pot_base.png",
        size: [32, 32],
    };
/// Vanilla `DecoratedPotRenderer.getSideSprite`: a decorated side binds
/// `Sheets.getDecoratedPotSprite(pattern)` (the pattern's
/// `entity/decorated_pot/<name>_pottery_pattern` sheet), an undecorated side — no sherd, a brick,
/// or an item with no registered pattern — falls back to `Sheets.DECORATED_POT_SIDE`.
pub(in crate::entity_models) fn decorated_pot_side_texture_ref(
    pattern: Option<DecoratedPotPattern>,
) -> EntityModelTextureRef {
    pattern.map_or(DECORATED_POT_SIDE_TEXTURE_REF, |pattern| {
        pattern.texture_ref()
    })
}

/// Vanilla `Sheets.BANNER_BASE` (`BANNER_MAPPER.defaultNamespaceApply("banner_base")`) — the
/// untinted pole/bar/flag sheet both `BannerModel` and `BannerFlagModel` bind.
pub(in crate::entity_models) const BANNER_BASE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/banner/banner_base.png",
        size: [64, 64],
    };
