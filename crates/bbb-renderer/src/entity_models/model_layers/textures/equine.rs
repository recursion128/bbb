use super::super::super::{EntityArmorMaterial, EntityModelTextureRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) struct HorseBodyArmorTextureLayer {
    pub texture: EntityModelTextureRef,
    pub dyeable: bool,
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

pub(in crate::entity_models) const HORSE_CREAMY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_creamy.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_CREAMY_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_creamy_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_CHESTNUT_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_chestnut.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_CHESTNUT_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_chestnut_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_BROWN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_brown.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_BROWN_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_brown_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_BLACK_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_black.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_BLACK_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_black_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_GRAY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_gray.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_GRAY_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_gray_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_DARKBROWN_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_darkbrown.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_DARKBROWN_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_darkbrown_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_MARKINGS_WHITE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_markings_white.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_MARKINGS_WHITE_BABY_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_markings_white_baby.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_MARKINGS_WHITEFIELD_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_markings_whitefield.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_MARKINGS_WHITEFIELD_BABY_TEXTURE_REF:
    EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/horse/horse_markings_whitefield_baby.png",
    size: [64, 64],
};

pub(in crate::entity_models) const HORSE_MARKINGS_WHITEDOTS_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_markings_whitedots.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_MARKINGS_WHITEDOTS_BABY_TEXTURE_REF:
    EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/horse/horse_markings_whitedots_baby.png",
    size: [64, 64],
};

pub(in crate::entity_models) const HORSE_MARKINGS_BLACKDOTS_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/horse/horse_markings_blackdots.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const HORSE_MARKINGS_BLACKDOTS_BABY_TEXTURE_REF:
    EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/horse/horse_markings_blackdots_baby.png",
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

pub(in crate::entity_models) const HORSE_SADDLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/horse_saddle/saddle.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const DONKEY_SADDLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/donkey_saddle/saddle.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const MULE_SADDLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/mule_saddle/saddle.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const SKELETON_HORSE_SADDLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/skeleton_horse_saddle/saddle.png",
        size: [64, 64],
    };

pub(in crate::entity_models) const ZOMBIE_HORSE_SADDLE_TEXTURE_REF: EntityModelTextureRef =
    EntityModelTextureRef {
        path: "textures/entity/equipment/zombie_horse_saddle/saddle.png",
        size: [64, 64],
    };

// Vanilla `EquipmentClientInfo.LayerType.HORSE_BODY`, resolved from horse armor equipment assets to
// `textures/entity/equipment/horse_body/<asset>.png`. Leather horse armor has the same two-layer
// shape as its equipment JSON: a dyeable leather layer followed by the non-dyeable overlay.
const fn horse_body_armor_ref(asset: &'static str) -> EntityModelTextureRef {
    EntityModelTextureRef {
        path: asset,
        size: [64, 64],
    }
}

pub(in crate::entity_models) const HORSE_BODY_LEATHER_TEXTURE_REF: EntityModelTextureRef =
    horse_body_armor_ref("textures/entity/equipment/horse_body/leather.png");
pub(in crate::entity_models) const HORSE_BODY_LEATHER_OVERLAY_TEXTURE_REF: EntityModelTextureRef =
    horse_body_armor_ref("textures/entity/equipment/horse_body/leather_overlay.png");
pub(in crate::entity_models) const HORSE_BODY_COPPER_TEXTURE_REF: EntityModelTextureRef =
    horse_body_armor_ref("textures/entity/equipment/horse_body/copper.png");
pub(in crate::entity_models) const HORSE_BODY_IRON_TEXTURE_REF: EntityModelTextureRef =
    horse_body_armor_ref("textures/entity/equipment/horse_body/iron.png");
pub(in crate::entity_models) const HORSE_BODY_GOLD_TEXTURE_REF: EntityModelTextureRef =
    horse_body_armor_ref("textures/entity/equipment/horse_body/gold.png");
pub(in crate::entity_models) const HORSE_BODY_DIAMOND_TEXTURE_REF: EntityModelTextureRef =
    horse_body_armor_ref("textures/entity/equipment/horse_body/diamond.png");
pub(in crate::entity_models) const HORSE_BODY_NETHERITE_TEXTURE_REF: EntityModelTextureRef =
    horse_body_armor_ref("textures/entity/equipment/horse_body/netherite.png");

const HORSE_BODY_LEATHER_LAYERS: [HorseBodyArmorTextureLayer; 2] = [
    HorseBodyArmorTextureLayer {
        texture: HORSE_BODY_LEATHER_TEXTURE_REF,
        dyeable: true,
    },
    HorseBodyArmorTextureLayer {
        texture: HORSE_BODY_LEATHER_OVERLAY_TEXTURE_REF,
        dyeable: false,
    },
];
const HORSE_BODY_COPPER_LAYERS: [HorseBodyArmorTextureLayer; 1] = [HorseBodyArmorTextureLayer {
    texture: HORSE_BODY_COPPER_TEXTURE_REF,
    dyeable: false,
}];
const HORSE_BODY_IRON_LAYERS: [HorseBodyArmorTextureLayer; 1] = [HorseBodyArmorTextureLayer {
    texture: HORSE_BODY_IRON_TEXTURE_REF,
    dyeable: false,
}];
const HORSE_BODY_GOLD_LAYERS: [HorseBodyArmorTextureLayer; 1] = [HorseBodyArmorTextureLayer {
    texture: HORSE_BODY_GOLD_TEXTURE_REF,
    dyeable: false,
}];
const HORSE_BODY_DIAMOND_LAYERS: [HorseBodyArmorTextureLayer; 1] = [HorseBodyArmorTextureLayer {
    texture: HORSE_BODY_DIAMOND_TEXTURE_REF,
    dyeable: false,
}];
const HORSE_BODY_NETHERITE_LAYERS: [HorseBodyArmorTextureLayer; 1] = [HorseBodyArmorTextureLayer {
    texture: HORSE_BODY_NETHERITE_TEXTURE_REF,
    dyeable: false,
}];

pub(in crate::entity_models) fn horse_body_armor_texture_layers(
    material: EntityArmorMaterial,
) -> Option<&'static [HorseBodyArmorTextureLayer]> {
    Some(match material {
        EntityArmorMaterial::Leather => &HORSE_BODY_LEATHER_LAYERS,
        EntityArmorMaterial::Copper => &HORSE_BODY_COPPER_LAYERS,
        EntityArmorMaterial::Iron => &HORSE_BODY_IRON_LAYERS,
        EntityArmorMaterial::Gold => &HORSE_BODY_GOLD_LAYERS,
        EntityArmorMaterial::Diamond => &HORSE_BODY_DIAMOND_LAYERS,
        EntityArmorMaterial::Netherite => &HORSE_BODY_NETHERITE_LAYERS,
        EntityArmorMaterial::Chainmail | EntityArmorMaterial::TurtleScute => return None,
    })
}
