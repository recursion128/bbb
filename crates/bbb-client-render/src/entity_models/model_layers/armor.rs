use crate::entity_models::model_layer::ModelLayer;

/// The four humanoid armor slots, in the vanilla `HumanoidArmorLayer.submit` render order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum HumanoidArmorSlot {
    Chest,
    Legs,
    Feet,
    Head,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) struct HumanoidArmorModelLayerSet {
    helmet: ModelLayer,
    chestplate: ModelLayer,
    leggings: ModelLayer,
    boots: ModelLayer,
}

impl HumanoidArmorModelLayerSet {
    pub(in crate::entity_models) const fn model_layer(self, slot: HumanoidArmorSlot) -> ModelLayer {
        match slot {
            HumanoidArmorSlot::Head => self.helmet,
            HumanoidArmorSlot::Chest => self.chestplate,
            HumanoidArmorSlot::Legs => self.leggings,
            HumanoidArmorSlot::Feet => self.boots,
        }
    }
}

pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_ARMOR_STAND:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: ModelLayer::ARMOR_STAND_HELMET,
    chestplate: ModelLayer::ARMOR_STAND_CHESTPLATE,
    leggings: ModelLayer::ARMOR_STAND_LEGGINGS,
    boots: ModelLayer::ARMOR_STAND_BOOTS,
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_ARMOR_STAND_SMALL:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: ModelLayer::ARMOR_STAND_SMALL_HELMET,
    chestplate: ModelLayer::ARMOR_STAND_SMALL_CHESTPLATE,
    leggings: ModelLayer::ARMOR_STAND_SMALL_LEGGINGS,
    boots: ModelLayer::ARMOR_STAND_SMALL_BOOTS,
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_BOGGED: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: ModelLayer::BOGGED_HELMET,
        chestplate: ModelLayer::BOGGED_CHESTPLATE,
        leggings: ModelLayer::BOGGED_LEGGINGS,
        boots: ModelLayer::BOGGED_BOOTS,
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_DROWNED: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: ModelLayer::DROWNED_HELMET,
        chestplate: ModelLayer::DROWNED_CHESTPLATE,
        leggings: ModelLayer::DROWNED_LEGGINGS,
        boots: ModelLayer::DROWNED_BOOTS,
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_DROWNED_BABY:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: ModelLayer::DROWNED_BABY_HELMET,
    chestplate: ModelLayer::DROWNED_BABY_CHESTPLATE,
    leggings: ModelLayer::DROWNED_BABY_LEGGINGS,
    boots: ModelLayer::DROWNED_BABY_BOOTS,
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_GIANT: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: ModelLayer::GIANT_HELMET,
        chestplate: ModelLayer::GIANT_CHESTPLATE,
        leggings: ModelLayer::GIANT_LEGGINGS,
        boots: ModelLayer::GIANT_BOOTS,
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_HUSK: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: ModelLayer::HUSK_HELMET,
        chestplate: ModelLayer::HUSK_CHESTPLATE,
        leggings: ModelLayer::HUSK_LEGGINGS,
        boots: ModelLayer::HUSK_BOOTS,
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_HUSK_BABY:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: ModelLayer::HUSK_BABY_HELMET,
    chestplate: ModelLayer::HUSK_BABY_CHESTPLATE,
    leggings: ModelLayer::HUSK_BABY_LEGGINGS,
    boots: ModelLayer::HUSK_BABY_BOOTS,
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_PARCHED: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: ModelLayer::PARCHED_HELMET,
        chestplate: ModelLayer::PARCHED_CHESTPLATE,
        leggings: ModelLayer::PARCHED_LEGGINGS,
        boots: ModelLayer::PARCHED_BOOTS,
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: ModelLayer::PIGLIN_HELMET,
        chestplate: ModelLayer::PIGLIN_CHESTPLATE,
        leggings: ModelLayer::PIGLIN_LEGGINGS,
        boots: ModelLayer::PIGLIN_BOOTS,
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN_BABY:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: ModelLayer::PIGLIN_BABY_HELMET,
    chestplate: ModelLayer::PIGLIN_BABY_CHESTPLATE,
    leggings: ModelLayer::PIGLIN_BABY_LEGGINGS,
    boots: ModelLayer::PIGLIN_BABY_BOOTS,
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN_BRUTE:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: ModelLayer::PIGLIN_BRUTE_HELMET,
    chestplate: ModelLayer::PIGLIN_BRUTE_CHESTPLATE,
    leggings: ModelLayer::PIGLIN_BRUTE_LEGGINGS,
    boots: ModelLayer::PIGLIN_BRUTE_BOOTS,
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_PLAYER: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: ModelLayer::PLAYER_HELMET,
        chestplate: ModelLayer::PLAYER_CHESTPLATE,
        leggings: ModelLayer::PLAYER_LEGGINGS,
        boots: ModelLayer::PLAYER_BOOTS,
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_PLAYER_SLIM:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: ModelLayer::PLAYER_SLIM_HELMET,
    chestplate: ModelLayer::PLAYER_SLIM_CHESTPLATE,
    leggings: ModelLayer::PLAYER_SLIM_LEGGINGS,
    boots: ModelLayer::PLAYER_SLIM_BOOTS,
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_SKELETON:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: ModelLayer::SKELETON_HELMET,
    chestplate: ModelLayer::SKELETON_CHESTPLATE,
    leggings: ModelLayer::SKELETON_LEGGINGS,
    boots: ModelLayer::SKELETON_BOOTS,
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_STRAY: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: ModelLayer::STRAY_HELMET,
        chestplate: ModelLayer::STRAY_CHESTPLATE,
        leggings: ModelLayer::STRAY_LEGGINGS,
        boots: ModelLayer::STRAY_BOOTS,
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_WITHER_SKELETON:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: ModelLayer::WITHER_SKELETON_HELMET,
    chestplate: ModelLayer::WITHER_SKELETON_CHESTPLATE,
    leggings: ModelLayer::WITHER_SKELETON_LEGGINGS,
    boots: ModelLayer::WITHER_SKELETON_BOOTS,
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: ModelLayer::ZOMBIE_HELMET,
        chestplate: ModelLayer::ZOMBIE_CHESTPLATE,
        leggings: ModelLayer::ZOMBIE_LEGGINGS,
        boots: ModelLayer::ZOMBIE_BOOTS,
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_BABY:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: ModelLayer::ZOMBIE_BABY_HELMET,
    chestplate: ModelLayer::ZOMBIE_BABY_CHESTPLATE,
    leggings: ModelLayer::ZOMBIE_BABY_LEGGINGS,
    boots: ModelLayer::ZOMBIE_BABY_BOOTS,
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_VILLAGER:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: ModelLayer::ZOMBIE_VILLAGER_HELMET,
    chestplate: ModelLayer::ZOMBIE_VILLAGER_CHESTPLATE,
    leggings: ModelLayer::ZOMBIE_VILLAGER_LEGGINGS,
    boots: ModelLayer::ZOMBIE_VILLAGER_BOOTS,
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_VILLAGER_BABY:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: ModelLayer::ZOMBIE_VILLAGER_BABY_HELMET,
    chestplate: ModelLayer::ZOMBIE_VILLAGER_BABY_CHESTPLATE,
    leggings: ModelLayer::ZOMBIE_VILLAGER_BABY_LEGGINGS,
    boots: ModelLayer::ZOMBIE_VILLAGER_BABY_BOOTS,
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIFIED_PIGLIN:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: ModelLayer::ZOMBIFIED_PIGLIN_HELMET,
    chestplate: ModelLayer::ZOMBIFIED_PIGLIN_CHESTPLATE,
    leggings: ModelLayer::ZOMBIFIED_PIGLIN_LEGGINGS,
    boots: ModelLayer::ZOMBIFIED_PIGLIN_BOOTS,
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIFIED_PIGLIN_BABY:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: ModelLayer::ZOMBIFIED_PIGLIN_BABY_HELMET,
    chestplate: ModelLayer::ZOMBIFIED_PIGLIN_BABY_CHESTPLATE,
    leggings: ModelLayer::ZOMBIFIED_PIGLIN_BABY_LEGGINGS,
    boots: ModelLayer::ZOMBIFIED_PIGLIN_BABY_BOOTS,
};

impl HumanoidArmorSlot {
    /// The humanoid part names this slot's armor model retains — also the parts whose host pose is
    /// copied onto the armor tree.
    pub(in crate::entity_models) fn part_names(self) -> &'static [&'static str] {
        match self {
            Self::Head => &["head"],
            Self::Chest => &["body", "right_arm", "left_arm"],
            Self::Legs => &["body", "right_leg", "left_leg"],
            Self::Feet => &["right_leg", "left_leg"],
        }
    }

    /// Direct humanoid child poses that vanilla `HumanoidModel.setupAnim` mutates on the baby armor
    /// model. The retained baby `waist` and nested foot cubes keep their own bind poses; the root leg
    /// parents carry walking/swimming rotations.
    pub(in crate::entity_models) fn baby_pose_part_names(self) -> &'static [&'static str] {
        match self {
            Self::Head => &["head"],
            Self::Chest => &["body", "right_arm", "left_arm"],
            Self::Legs | Self::Feet => &["right_leg", "left_leg"],
        }
    }
}
