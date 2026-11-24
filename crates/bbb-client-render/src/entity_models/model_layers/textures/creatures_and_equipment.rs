use super::super::super::EntityModelTextureRef;
use crate::entity_models::catalog::{
    ArrowModelTexture, AxolotlModelVariant, EntityDyeColor, FoxModelVariant, PandaModelVariant,
    ParrotModelVariant, RabbitModelVariant,
};
use crate::entity_models::temperature_animal_variants::VANILLA_FROG_VARIANT_TEXTURE_REFS;
use crate::entity_models::zombie_nautilus_variants::VANILLA_ZOMBIE_NAUTILUS_VARIANT_TEXTURE_REFS;
use crate::entity_models::{CatModelVariant, FrogModelVariant};

pub(in crate::entity_models) const LEASH_KNOT_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/lead_knot/lead_knot.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const TRIDENT_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/trident/trident.png",
        size: [32, 32],
    };

pub(in crate::entity_models) const TRIDENT_RIPTIDE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/trident/trident_riptide.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const ENCHANTED_GLINT_ITEM_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/misc/enchanted_glint_item.png",
        size: [128, 128],
    };

pub(in crate::entity_models) const ENCHANTED_GLINT_ARMOR_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/misc/enchanted_glint_armor.png",
        size: [128, 128],
    };

pub(in crate::entity_models) const EVOKER_FANGS_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/illager/evoker_fangs.png",
        size: [64, 32],
    };

pub(in crate::entity_models) const TADPOLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/tadpole/tadpole.png",
        size: [16, 16],
    };

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

pub(in crate::entity_models) const SNIFFER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/sniffer/sniffer.png",
        size: [192, 192],
    };

pub(in crate::entity_models) const SNIFFLET_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/sniffer/snifflet.png",
        size: [128, 128],
    };

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

pub(in crate::entity_models) const ENDER_DRAGON_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/enderdragon/dragon.png",
        size: [256, 256],
    };
// Vanilla `EnderDragonRenderer.DYING_RENDER_TYPE`
// (`RenderTypes.entityCutoutDissolve(dragon.png, dragon_exploding.png)`): the dissolve mask
// texture sampled while `EnderDragonRenderState.deathTime > 0`.
pub(in crate::entity_models) const ENDER_DRAGON_EXPLODING_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/enderdragon/dragon_exploding.png",
        size: [256, 256],
    };
// Vanilla `EnderDragonRenderer.EYES` (`RenderTypes.eyes`): the always-on emissive eye overlay,
// re-rendering the whole model with `dragon_eyes.png`.
pub(in crate::entity_models) const ENDER_DRAGON_EYES_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/enderdragon/dragon_eyes.png",
        size: [256, 256],
    };

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
// Vanilla `EquipmentClientInfo.LayerType.NAUTILUS_SADDLE`, resolved from the common saddle equipment
// asset to `textures/entity/equipment/nautilus_saddle/saddle.png`.
pub(in crate::entity_models) const NAUTILUS_SADDLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/nautilus_saddle/saddle.png",
        size: [128, 128],
    };
// Vanilla `EquipmentClientInfo.LayerType.NAUTILUS_BODY`, resolved from the armor material equipment
// asset to `textures/entity/equipment/nautilus_body/<asset>.png`. Vanilla 26.1 has five nautilus
// body armor items/materials; there is no leather/chainmail/turtle-scute nautilus armor texture.
const fn nautilus_body_armor_ref(asset: &'static str) -> EntityModelTextureRef {
    EntityModelTextureRef {
        path: asset,
        size: [128, 128],
    }
}
pub(in crate::entity_models) const NAUTILUS_BODY_COPPER_TEXTURE_REF: EntityModelTextureRef =
    nautilus_body_armor_ref("textures/entity/equipment/nautilus_body/copper.png");
pub(in crate::entity_models) const NAUTILUS_BODY_IRON_TEXTURE_REF: EntityModelTextureRef =
    nautilus_body_armor_ref("textures/entity/equipment/nautilus_body/iron.png");
pub(in crate::entity_models) const NAUTILUS_BODY_GOLD_TEXTURE_REF: EntityModelTextureRef =
    nautilus_body_armor_ref("textures/entity/equipment/nautilus_body/gold.png");
pub(in crate::entity_models) const NAUTILUS_BODY_DIAMOND_TEXTURE_REF: EntityModelTextureRef =
    nautilus_body_armor_ref("textures/entity/equipment/nautilus_body/diamond.png");
pub(in crate::entity_models) const NAUTILUS_BODY_NETHERITE_TEXTURE_REF: EntityModelTextureRef =
    nautilus_body_armor_ref("textures/entity/equipment/nautilus_body/netherite.png");
// Stable aliases keep focused nautilus tests readable; the generated bootstrap catalog owns the
// built-in model/texture presentation facts and official PNG dimensions.
pub(in crate::entity_models) const ZOMBIE_NAUTILUS_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_ZOMBIE_NAUTILUS_VARIANT_TEXTURE_REFS[0];
pub(in crate::entity_models) const ZOMBIE_NAUTILUS_CORAL_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_ZOMBIE_NAUTILUS_VARIANT_TEXTURE_REFS[1];

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
// The generated `CatVariants` catalog owns all eleven adult/baby built-in cat textures. Ocelot
// and collar textures remain renderer-owned because they are not cat-variant registry payloads.
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
    variant.builtin_texture_ref(baby)
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
// The mooshroom shares the cow model (`ModelLayers.MOOSHROOM` is the cow mesh), so it reuses the cow
// UV layout and only swaps in the mooshroom recolor. Vanilla `MushroomCowRenderer` keys the texture on
// the red/brown variant (`MushroomCowRenderer.getTextureLocation`), keyed on the synced
// `MushroomCow.DATA_TYPE`; `RED` is the vanilla default. The `MushroomCowMushroomLayer`
// block-mushrooms are baked by the entity-attached block-model path.
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
pub(in crate::entity_models) const MOOSHROOM_BROWN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cow/mooshroom_brown.png",
        size: [64, 64],
    };
pub(in crate::entity_models) const MOOSHROOM_BROWN_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/cow/mooshroom_brown_baby.png",
        size: [64, 64],
    };
// Small single-texture non-living models. `WITHER_TEXTURE_REF` (wither.png) is shared by the wither
// skull and the wither boss; the skull swaps to `WITHER_INVULNERABLE_TEXTURE_REF` when dangerous, while
// the boss swaps to it during its spawn charge. The arrow texture variants are selected from entity
// state.
pub(in crate::entity_models) const END_CRYSTAL_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/end_crystal/end_crystal.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const END_CRYSTAL_BEAM_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/end_crystal/end_crystal_beam.png",
        size: [16, 256],
    };
pub(in crate::entity_models) const END_SKY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/environment/end_sky.png",
        size: [128, 128],
    };
pub(in crate::entity_models) const END_PORTAL_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/end_portal/end_portal.png",
        size: [256, 256],
    };
pub(in crate::entity_models) const END_GATEWAY_BEAM_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/end_portal/end_gateway_beam.png",
        size: [16, 256],
    };
pub(in crate::entity_models) const BEACON_BEAM_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/beacon/beacon_beam.png",
        size: [16, 16],
    };
pub(in crate::entity_models) const ARROW_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/projectiles/arrow.png",
        size: [32, 32],
    };
// Vanilla `TippableArrowRenderer.TIPPED_ARROW_LOCATION`: a tipped arrow (any potion, `getColor() > 0`)
// swaps to `arrow_tipped.png`. `SpectralArrowRenderer.SPECTRAL_ARROW_LOCATION`: the spectral arrow
// (a distinct entity type) binds `arrow_spectral.png`.
pub(in crate::entity_models) const ARROW_TIPPED_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/projectiles/arrow_tipped.png",
        size: [32, 32],
    };
pub(in crate::entity_models) const ARROW_SPECTRAL_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/projectiles/arrow_spectral.png",
        size: [32, 32],
    };
pub(in crate::entity_models) fn arrow_texture_ref(
    texture: ArrowModelTexture,
) -> EntityModelTextureRef {
    match texture {
        ArrowModelTexture::Normal => ARROW_TEXTURE_REF,
        ArrowModelTexture::Tipped => ARROW_TIPPED_TEXTURE_REF,
        ArrowModelTexture::Spectral => ARROW_SPECTRAL_TEXTURE_REF,
    }
}
pub(in crate::entity_models) const LLAMA_SPIT_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/llama/llama_spit.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const SHULKER_BULLET_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/shulker/spark.png",
        size: [64, 32],
    };
pub(in crate::entity_models) const WITHER_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/wither/wither.png",
        size: [64, 64],
    };
// The wither boss swaps to this during its spawn charge (`WitherBossRenderer.getTextureLocation`), and
// dangerous wither skulls use it directly (`WitherSkullRenderer.getTextureLocation`).
pub(in crate::entity_models) const WITHER_INVULNERABLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/wither/wither_invulnerable.png",
        size: [64, 64],
    };
// The `WitherArmorLayer` energy swirl overlay (vanilla `EnergySwirlLayer`), shares the
// `createBodyLayer` 64×64 UV layout of `wither.png`.
pub(in crate::entity_models) const WITHER_ARMOR_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/wither/wither_armor.png",
        size: [64, 64],
    };
pub(in crate::entity_models) fn wither_skull_texture_ref(dangerous: bool) -> EntityModelTextureRef {
    if dangerous {
        WITHER_INVULNERABLE_TEXTURE_REF
    } else {
        WITHER_TEXTURE_REF
    }
}
// The wither boss covers its normal, spawn-charge, and powered energy-swirl textures.
pub(in crate::entity_models) const WIND_CHARGE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/projectiles/wind_charge.png",
        size: [64, 32],
    };
// Vanilla `HumanoidArmorLayer` equipment-asset textures (`EquipmentClientInfo.Layer.getTextureLocation`).
// Adult `HUMANOID` (head / chest / feet) and `HUMANOID_LEGGINGS` (legs) textures are 64x32. Baby
// humanoid armor uses the distinct `HUMANOID_BABY` layer for every slot and those textures are 64x64.
// The asset id is the `ArmorMaterials.<MAT>` → `EquipmentAssets.<MAT>` name (`CHAINMAIL` →
// `chainmail`, `TURTLE_SCUTE` → `turtle_scute`). Turtle scute is helmet-only for real equipment, so
// it has no leggings texture.
const fn armor_humanoid_ref(path: &'static str) -> EntityModelTextureRef {
    EntityModelTextureRef {
        path,
        size: [64, 32],
    }
}

const fn baby_armor_humanoid_ref(path: &'static str) -> EntityModelTextureRef {
    EntityModelTextureRef {
        path,
        size: [64, 64],
    }
}

pub(in crate::entity_models) const ARMOR_LEATHER_HUMANOID_TEXTURE_REF: EntityModelTextureRef =
    armor_humanoid_ref("textures/entity/equipment/humanoid/leather.png");
pub(in crate::entity_models) const ARMOR_COPPER_HUMANOID_TEXTURE_REF: EntityModelTextureRef =
    armor_humanoid_ref("textures/entity/equipment/humanoid/copper.png");
pub(in crate::entity_models) const ARMOR_CHAINMAIL_HUMANOID_TEXTURE_REF: EntityModelTextureRef =
    armor_humanoid_ref("textures/entity/equipment/humanoid/chainmail.png");
pub(in crate::entity_models) const ARMOR_IRON_HUMANOID_TEXTURE_REF: EntityModelTextureRef =
    armor_humanoid_ref("textures/entity/equipment/humanoid/iron.png");
pub(in crate::entity_models) const ARMOR_GOLD_HUMANOID_TEXTURE_REF: EntityModelTextureRef =
    armor_humanoid_ref("textures/entity/equipment/humanoid/gold.png");
pub(in crate::entity_models) const ARMOR_DIAMOND_HUMANOID_TEXTURE_REF: EntityModelTextureRef =
    armor_humanoid_ref("textures/entity/equipment/humanoid/diamond.png");
pub(in crate::entity_models) const ARMOR_TURTLE_SCUTE_HUMANOID_TEXTURE_REF: EntityModelTextureRef =
    armor_humanoid_ref("textures/entity/equipment/humanoid/turtle_scute.png");
pub(in crate::entity_models) const ARMOR_NETHERITE_HUMANOID_TEXTURE_REF: EntityModelTextureRef =
    armor_humanoid_ref("textures/entity/equipment/humanoid/netherite.png");
pub(in crate::entity_models) const ARMOR_LEATHER_LEGGINGS_TEXTURE_REF: EntityModelTextureRef =
    armor_humanoid_ref("textures/entity/equipment/humanoid_leggings/leather.png");
pub(in crate::entity_models) const ARMOR_COPPER_LEGGINGS_TEXTURE_REF: EntityModelTextureRef =
    armor_humanoid_ref("textures/entity/equipment/humanoid_leggings/copper.png");
pub(in crate::entity_models) const ARMOR_CHAINMAIL_LEGGINGS_TEXTURE_REF: EntityModelTextureRef =
    armor_humanoid_ref("textures/entity/equipment/humanoid_leggings/chainmail.png");
pub(in crate::entity_models) const ARMOR_IRON_LEGGINGS_TEXTURE_REF: EntityModelTextureRef =
    armor_humanoid_ref("textures/entity/equipment/humanoid_leggings/iron.png");
pub(in crate::entity_models) const ARMOR_GOLD_LEGGINGS_TEXTURE_REF: EntityModelTextureRef =
    armor_humanoid_ref("textures/entity/equipment/humanoid_leggings/gold.png");
pub(in crate::entity_models) const ARMOR_DIAMOND_LEGGINGS_TEXTURE_REF: EntityModelTextureRef =
    armor_humanoid_ref("textures/entity/equipment/humanoid_leggings/diamond.png");
pub(in crate::entity_models) const ARMOR_NETHERITE_LEGGINGS_TEXTURE_REF: EntityModelTextureRef =
    armor_humanoid_ref("textures/entity/equipment/humanoid_leggings/netherite.png");

pub(in crate::entity_models) const ARMOR_LEATHER_BABY_HUMANOID_TEXTURE_REF: EntityModelTextureRef =
    baby_armor_humanoid_ref("textures/entity/equipment/humanoid_baby/leather.png");
pub(in crate::entity_models) const ARMOR_COPPER_BABY_HUMANOID_TEXTURE_REF: EntityModelTextureRef =
    baby_armor_humanoid_ref("textures/entity/equipment/humanoid_baby/copper.png");
pub(in crate::entity_models) const ARMOR_CHAINMAIL_BABY_HUMANOID_TEXTURE_REF:
    EntityModelTextureRef =
    baby_armor_humanoid_ref("textures/entity/equipment/humanoid_baby/chainmail.png");
pub(in crate::entity_models) const ARMOR_IRON_BABY_HUMANOID_TEXTURE_REF: EntityModelTextureRef =
    baby_armor_humanoid_ref("textures/entity/equipment/humanoid_baby/iron.png");
pub(in crate::entity_models) const ARMOR_GOLD_BABY_HUMANOID_TEXTURE_REF: EntityModelTextureRef =
    baby_armor_humanoid_ref("textures/entity/equipment/humanoid_baby/gold.png");
pub(in crate::entity_models) const ARMOR_DIAMOND_BABY_HUMANOID_TEXTURE_REF: EntityModelTextureRef =
    baby_armor_humanoid_ref("textures/entity/equipment/humanoid_baby/diamond.png");
pub(in crate::entity_models) const ARMOR_TURTLE_SCUTE_BABY_HUMANOID_TEXTURE_REF:
    EntityModelTextureRef =
    baby_armor_humanoid_ref("textures/entity/equipment/humanoid_baby/turtle_scute.png");
pub(in crate::entity_models) const ARMOR_NETHERITE_BABY_HUMANOID_TEXTURE_REF:
    EntityModelTextureRef =
    baby_armor_humanoid_ref("textures/entity/equipment/humanoid_baby/netherite.png");

// Vanilla `EquipmentClientInfo.LayerType.WINGS` for the elytra equipment asset. The layer is 64x32
// and may be overridden by a player's profile elytra/cape texture when `use_player_texture` is true.
pub(in crate::entity_models) const ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/wings/elytra.png",
        size: [64, 32],
    };

// The guardian and elder guardian share one mesh/UV layout, differing only by texture (and the elder's
// 2.35 root scale). The shared attack beam (`guardian_beam.png`, 32×32) is the `GuardianRenderer`
// `BEAM_RENDER_TYPE` texture, tiled vertically along the beam via the scroll (fract-wrap) pipeline.
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
pub(in crate::entity_models) const GUARDIAN_BEAM_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/guardian/guardian_beam.png",
        size: [32, 32],
    };
// The warden's base body texture (atlas 128×128). Four emissive overlay layers (tendrils, heart,
// bioluminescent, pulsating spots) follow their posed part subsets; dig/emerge use model keyframes.
pub(in crate::entity_models) const WARDEN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/warden/warden.png",
        size: [128, 128],
    };
// Vanilla `WardenRenderer`'s always-on bioluminescent emissive layer (alpha 1.0): the warden's
// glowing body markings, restricted by `createBioluminescentLayer` to the head, arms, and legs.
pub(in crate::entity_models) const WARDEN_BIOLUMINESCENT_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/warden/warden_bioluminescent_layer.png",
        size: [128, 128],
    };
// Vanilla `WardenRenderer`'s two pulsating-spots emissive layers: the body spots that fade in/out on
// the `cos(ageInTicks · 0.045)` pulse (the second offset by π so the two sets alternate).
pub(in crate::entity_models) const WARDEN_PULSATING_SPOTS_1_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/warden/warden_pulsating_spots_1.png",
        size: [128, 128],
    };
pub(in crate::entity_models) const WARDEN_PULSATING_SPOTS_2_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/warden/warden_pulsating_spots_2.png",
        size: [128, 128],
    };
// Vanilla `WardenRenderer`'s heart emissive layer: the chest heart that pulses on each heartbeat at
// the lerped `heartAnimation` alpha (`createHeartLayer` retains only the body).
pub(in crate::entity_models) const WARDEN_HEART_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/warden/warden_heart.png",
        size: [128, 128],
    };
// Stable aliases keep focused frog tests readable; generated `FrogVariants` presentation owns
// the asset paths and atlas dimensions.
pub(in crate::entity_models) const FROG_TEMPERATE_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_FROG_VARIANT_TEXTURE_REFS[0];
pub(in crate::entity_models) const FROG_WARM_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_FROG_VARIANT_TEXTURE_REFS[1];
pub(in crate::entity_models) const FROG_COLD_TEXTURE_REF: EntityModelTextureRef =
    VANILLA_FROG_VARIANT_TEXTURE_REFS[2];
pub(in crate::entity_models) fn frog_texture_ref(
    variant: FrogModelVariant,
) -> EntityModelTextureRef {
    variant.builtin_texture_ref()
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

pub(in crate::entity_models) const PUFFERFISH_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/fish/pufferfish.png",
        size: [32, 32],
    };
