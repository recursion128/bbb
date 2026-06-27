use super::{
    PartPose, ARMOR_CHAINMAIL_BABY_HUMANOID_TEXTURE_REF, ARMOR_CHAINMAIL_HUMANOID_TEXTURE_REF,
    ARMOR_CHAINMAIL_LEGGINGS_TEXTURE_REF, ARMOR_COPPER_BABY_HUMANOID_TEXTURE_REF,
    ARMOR_COPPER_HUMANOID_TEXTURE_REF, ARMOR_COPPER_LEGGINGS_TEXTURE_REF,
    ARMOR_DIAMOND_BABY_HUMANOID_TEXTURE_REF, ARMOR_DIAMOND_HUMANOID_TEXTURE_REF,
    ARMOR_DIAMOND_LEGGINGS_TEXTURE_REF, ARMOR_GOLD_BABY_HUMANOID_TEXTURE_REF,
    ARMOR_GOLD_HUMANOID_TEXTURE_REF, ARMOR_GOLD_LEGGINGS_TEXTURE_REF,
    ARMOR_IRON_BABY_HUMANOID_TEXTURE_REF, ARMOR_IRON_HUMANOID_TEXTURE_REF,
    ARMOR_IRON_LEGGINGS_TEXTURE_REF, ARMOR_LEATHER_BABY_HUMANOID_TEXTURE_REF,
    ARMOR_LEATHER_HUMANOID_TEXTURE_REF, ARMOR_LEATHER_LEGGINGS_TEXTURE_REF,
    ARMOR_NETHERITE_BABY_HUMANOID_TEXTURE_REF, ARMOR_NETHERITE_HUMANOID_TEXTURE_REF,
    ARMOR_NETHERITE_LEGGINGS_TEXTURE_REF, ARMOR_TURTLE_SCUTE_BABY_HUMANOID_TEXTURE_REF,
    ARMOR_TURTLE_SCUTE_HUMANOID_TEXTURE_REF, PART_POSE_ZERO,
};
use crate::entity_models::catalog::{EntityArmorMaterial, EntityModelTextureRef};
use crate::entity_models::model::{ModelCube, ModelPart};

// Vanilla 26.1 humanoid armor layer (`HumanoidModel.createBaseArmorMesh` / `createArmorMeshSet`,
// atlas 64×32). The armor is the standard humanoid mesh (`createMesh`) grown by a `CubeDeformation`
// so it floats just outside the body, then split into four per-slot models by `retainExactParts`:
//   HEAD  (helmet):     head (+ hat child), OUTER deformation (hat `g.extend(0.5)`)
//   CHEST (chestplate): body + both arms,   OUTER deformation
//   LEGS  (leggings):   body + both legs,   INNER deformation `0.5` (legs `g.extend(-0.1)` = `0.4`)
//   FEET  (boots):      both legs,          OUTER deformation (legs `g.extend(-0.1)`)
// The OUTER deformation is `1.0` for the standard humanoid wearers and `1.02` for the piglin family.
// The legs are grown by `g - 0.1` so the leggings (inner) and the body/boots (outer) layers do not
// z-fight where they overlap. Each per-slot tree is draped on the host humanoid model's posed limbs
// via [`ModelPart::copy_child_poses_from`] (vanilla `copyPropertiesTo`), so the armor inherits the
// host's `setup_anim` without re-running it. The mesh carries the textured `uv_size` / `texOffs`; the
// colored debug tint is a never-rendered placeholder (armor is a textured-only overlay).

const ARMOR_PLACEHOLDER_COLOR: [f32; 4] = [0.55, 0.55, 0.58, 1.0];

/// The standard `HumanoidModel.createArmorMeshSet` outer deformation (`1.0`), used by every base
/// humanoid armor wearer (zombie / skeleton / player families).
pub(in crate::entity_models) const STANDARD_OUTER_ARMOR_DEFORMATION: f32 = 1.0;
/// The piglin family's outer deformation (vanilla `LayerDefinitions` piglin armor,
/// `PiglinModel.createArmorMeshSet(INNER_ARMOR_DEFORMATION, new CubeDeformation(1.02F))`): the same
/// base armor mesh grown a hair more so it clears the slightly chunkier piglin body. The inner
/// (leggings) deformation is unchanged.
pub(in crate::entity_models) const PIGLIN_OUTER_ARMOR_DEFORMATION: f32 = 1.02;
const INNER_ARMOR_DEFORMATION: f32 = 0.5;

/// Vanilla `CubeListBuilder.addBox(..., g)` with `CubeDeformation(g)`: grows the box (`min -= g`,
/// `size += 2·g`) while keeping the base `texOffs` and the base box dims for the UV mapping.
const fn armor_cube_deformed(
    min: [f32; 3],
    size: [f32; 3],
    tex: [f32; 2],
    mirror: bool,
    g: [f32; 3],
) -> ModelCube {
    ModelCube::new(
        [min[0] - g[0], min[1] - g[1], min[2] - g[2]],
        [
            size[0] + 2.0 * g[0],
            size[1] + 2.0 * g[1],
            size[2] + 2.0 * g[2],
        ],
        ARMOR_PLACEHOLDER_COLOR,
        size,
        tex,
        mirror,
    )
}

const fn armor_cube(
    min: [f32; 3],
    size: [f32; 3],
    tex: [f32; 2],
    mirror: bool,
    g: f32,
) -> ModelCube {
    armor_cube_deformed(min, size, tex, mirror, [g, g, g])
}

const fn extend_deformation(g: [f32; 3], factor: f32) -> [f32; 3] {
    [g[0] + factor, g[1] + factor, g[2] + factor]
}

// The base humanoid boxes (`HumanoidModel.createMesh`, g = 0). Grown per slot below.
// `head` texOffs(0,0) 8×8×8; `hat` texOffs(32,0) 8×8×8; `body` texOffs(16,16) 8×12×4;
// arms texOffs(40,16) 4×12×4; legs texOffs(0,16) 4×12×4.
const HEAD_MIN: [f32; 3] = [-4.0, -8.0, -4.0];
const HEAD_SIZE: [f32; 3] = [8.0, 8.0, 8.0];
const BODY_MIN: [f32; 3] = [-4.0, 0.0, -2.0];
const BODY_SIZE: [f32; 3] = [8.0, 12.0, 4.0];
const RIGHT_ARM_MIN: [f32; 3] = [-3.0, -2.0, -2.0];
const LEFT_ARM_MIN: [f32; 3] = [-1.0, -2.0, -2.0];
const ARM_SIZE: [f32; 3] = [4.0, 12.0, 4.0];
const LEG_MIN: [f32; 3] = [-2.0, 0.0, -2.0];
const LEG_SIZE: [f32; 3] = [4.0, 12.0, 4.0];

// The OUTER model parts (head/hat/body/arms and the boots' legs) are grown at render time by the
// wearer's outer deformation — `STANDARD_OUTER_ARMOR_DEFORMATION` (1.0) or
// `PIGLIN_OUTER_ARMOR_DEFORMATION` (1.02) — so `build_tree` bakes them per call (hat `g + 0.5`,
// boots' legs `g - 0.1`). Only the INNER (leggings) parts are deformation-fixed.

// INNER model parts (g = 0.5; legs `g - 0.1 = 0.4`).
const ARMOR_BODY_INNER: [ModelCube; 1] = [armor_cube(
    BODY_MIN,
    BODY_SIZE,
    [16.0, 16.0],
    false,
    INNER_ARMOR_DEFORMATION,
)];
const ARMOR_RIGHT_LEG_INNER: [ModelCube; 1] = [armor_cube(
    LEG_MIN,
    LEG_SIZE,
    [0.0, 16.0],
    false,
    INNER_ARMOR_DEFORMATION - 0.1,
)];
const ARMOR_LEFT_LEG_INNER: [ModelCube; 1] = [armor_cube(
    LEG_MIN,
    LEG_SIZE,
    [0.0, 16.0],
    true,
    INNER_ARMOR_DEFORMATION - 0.1,
)];

// Vanilla `HumanoidModel.createMesh` bind poses (yOffset = 0).
const HEAD_POSE: PartPose = PART_POSE_ZERO;
const BODY_POSE: PartPose = PART_POSE_ZERO;
const RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const LEFT_ARM_POSE: PartPose = PartPose {
    offset: [5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-1.9, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const LEFT_LEG_POSE: PartPose = PartPose {
    offset: [1.9, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

// Vanilla `LayerDefinitions` standard baby humanoid armor deformations and
// `HumanoidModel.createBabyArmorMesh` parts. These are used by zombie, husk, and drowned baby armor.
const BABY_OUTER_ARMOR_DEFORMATION: [f32; 3] = [-0.1, 0.5, 0.3];
const BABY_INNER_ARMOR_DEFORMATION: [f32; 3] = [-0.1, 0.3, 0.3];
const BABY_PIGLIN_ARMOR_DEFORMATION: [f32; 3] = [0.7, 0.7, 0.7];
const BABY_PIGLIN_ARMOR_ARM_OFFSET: [f32; 3] = [0.5, -0.5, 0.0];

const BABY_ARMOR_HEAD_MIN: [f32; 3] = [-4.5, -7.0, -4.5];
const BABY_ARMOR_HEAD_SIZE: [f32; 3] = [9.0, 8.0, 8.0];
const BABY_ARMOR_BODY_MIN: [f32; 3] = [-3.0, -3.0, -1.5];
const BABY_ARMOR_BODY_SIZE: [f32; 3] = [6.0, 5.0, 3.0];
const BABY_ARMOR_WAIST_MIN: [f32; 3] = [-3.0, -1.2, -1.49];
const BABY_ARMOR_WAIST_SIZE: [f32; 3] = [5.9, 2.0, 2.9];
const BABY_ARMOR_ARM_MIN: [f32; 3] = [-1.0, 0.0, -1.53];
const BABY_ARMOR_ARM_SIZE: [f32; 3] = [2.0, 5.0, 3.0];
const BABY_ARMOR_LEFT_LEG_MIN: [f32; 3] = [-2.0, -0.2, -2.0];
const BABY_ARMOR_RIGHT_LEG_MIN: [f32; 3] = [-1.0, -0.2, -2.0];
const BABY_ARMOR_LEG_SIZE: [f32; 3] = [3.0, 4.0, 3.0];
const BABY_ARMOR_RIGHT_FOOT_MIN: [f32; 3] = [-2.0, 2.9, -2.0];
const BABY_ARMOR_LEFT_FOOT_MIN: [f32; 3] = [-1.0, 2.9, -2.0];
const BABY_ARMOR_FOOT_SIZE: [f32; 3] = [3.0, 1.0, 3.0];

const BABY_ARMOR_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 15.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ARMOR_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 18.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ARMOR_WAIST_POSE: PartPose = PartPose {
    offset: [0.0, 19.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ARMOR_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-3.5, 15.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ARMOR_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [3.5, 15.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ARMOR_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [1.5, 20.0, 0.5],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ARMOR_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-1.5, 20.0, 0.5],
    rotation: [0.0, 0.0, 0.0],
};

/// The four humanoid armor slots, in the vanilla `HumanoidArmorLayer.submit` render order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum HumanoidArmorSlot {
    Chest,
    Legs,
    Feet,
    Head,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum HumanoidBabyArmorKind {
    Standard,
    Piglin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) struct HumanoidArmorModelLayerSet {
    helmet: &'static str,
    chestplate: &'static str,
    leggings: &'static str,
    boots: &'static str,
}

impl HumanoidArmorModelLayerSet {
    pub(in crate::entity_models) const fn model_layer(
        self,
        slot: HumanoidArmorSlot,
    ) -> &'static str {
        match slot {
            HumanoidArmorSlot::Head => self.helmet,
            HumanoidArmorSlot::Chest => self.chestplate,
            HumanoidArmorSlot::Legs => self.leggings,
            HumanoidArmorSlot::Feet => self.boots,
        }
    }
}

pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_BOGGED: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: "minecraft:bogged#helmet",
        chestplate: "minecraft:bogged#chestplate",
        leggings: "minecraft:bogged#leggings",
        boots: "minecraft:bogged#boots",
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_DROWNED: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: "minecraft:drowned#helmet",
        chestplate: "minecraft:drowned#chestplate",
        leggings: "minecraft:drowned#leggings",
        boots: "minecraft:drowned#boots",
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_DROWNED_BABY:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: "minecraft:drowned_baby#helmet",
    chestplate: "minecraft:drowned_baby#chestplate",
    leggings: "minecraft:drowned_baby#leggings",
    boots: "minecraft:drowned_baby#boots",
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_GIANT: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: "minecraft:giant#helmet",
        chestplate: "minecraft:giant#chestplate",
        leggings: "minecraft:giant#leggings",
        boots: "minecraft:giant#boots",
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_HUSK: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: "minecraft:husk#helmet",
        chestplate: "minecraft:husk#chestplate",
        leggings: "minecraft:husk#leggings",
        boots: "minecraft:husk#boots",
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_HUSK_BABY:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: "minecraft:husk_baby#helmet",
    chestplate: "minecraft:husk_baby#chestplate",
    leggings: "minecraft:husk_baby#leggings",
    boots: "minecraft:husk_baby#boots",
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_PARCHED: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: "minecraft:parched#helmet",
        chestplate: "minecraft:parched#chestplate",
        leggings: "minecraft:parched#leggings",
        boots: "minecraft:parched#boots",
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: "minecraft:piglin#helmet",
        chestplate: "minecraft:piglin#chestplate",
        leggings: "minecraft:piglin#leggings",
        boots: "minecraft:piglin#boots",
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN_BABY:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: "minecraft:piglin_baby#helmet",
    chestplate: "minecraft:piglin_baby#chestplate",
    leggings: "minecraft:piglin_baby#leggings",
    boots: "minecraft:piglin_baby#boots",
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN_BRUTE:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: "minecraft:piglin_brute#helmet",
    chestplate: "minecraft:piglin_brute#chestplate",
    leggings: "minecraft:piglin_brute#leggings",
    boots: "minecraft:piglin_brute#boots",
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_PLAYER: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: "minecraft:player#helmet",
        chestplate: "minecraft:player#chestplate",
        leggings: "minecraft:player#leggings",
        boots: "minecraft:player#boots",
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_PLAYER_SLIM:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: "minecraft:player_slim#helmet",
    chestplate: "minecraft:player_slim#chestplate",
    leggings: "minecraft:player_slim#leggings",
    boots: "minecraft:player_slim#boots",
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_SKELETON:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: "minecraft:skeleton#helmet",
    chestplate: "minecraft:skeleton#chestplate",
    leggings: "minecraft:skeleton#leggings",
    boots: "minecraft:skeleton#boots",
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_STRAY: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: "minecraft:stray#helmet",
        chestplate: "minecraft:stray#chestplate",
        leggings: "minecraft:stray#leggings",
        boots: "minecraft:stray#boots",
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_WITHER_SKELETON:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: "minecraft:wither_skeleton#helmet",
    chestplate: "minecraft:wither_skeleton#chestplate",
    leggings: "minecraft:wither_skeleton#leggings",
    boots: "minecraft:wither_skeleton#boots",
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE: HumanoidArmorModelLayerSet =
    HumanoidArmorModelLayerSet {
        helmet: "minecraft:zombie#helmet",
        chestplate: "minecraft:zombie#chestplate",
        leggings: "minecraft:zombie#leggings",
        boots: "minecraft:zombie#boots",
    };
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_BABY:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: "minecraft:zombie_baby#helmet",
    chestplate: "minecraft:zombie_baby#chestplate",
    leggings: "minecraft:zombie_baby#leggings",
    boots: "minecraft:zombie_baby#boots",
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_VILLAGER:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: "minecraft:zombie_villager#helmet",
    chestplate: "minecraft:zombie_villager#chestplate",
    leggings: "minecraft:zombie_villager#leggings",
    boots: "minecraft:zombie_villager#boots",
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_VILLAGER_BABY:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: "minecraft:zombie_villager_baby#helmet",
    chestplate: "minecraft:zombie_villager_baby#chestplate",
    leggings: "minecraft:zombie_villager_baby#leggings",
    boots: "minecraft:zombie_villager_baby#boots",
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIFIED_PIGLIN:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: "minecraft:zombified_piglin#helmet",
    chestplate: "minecraft:zombified_piglin#chestplate",
    leggings: "minecraft:zombified_piglin#leggings",
    boots: "minecraft:zombified_piglin#boots",
};
pub(in crate::entity_models) const HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIFIED_PIGLIN_BABY:
    HumanoidArmorModelLayerSet = HumanoidArmorModelLayerSet {
    helmet: "minecraft:zombified_piglin_baby#helmet",
    chestplate: "minecraft:zombified_piglin_baby#chestplate",
    leggings: "minecraft:zombified_piglin_baby#leggings",
    boots: "minecraft:zombified_piglin_baby#boots",
};

impl HumanoidBabyArmorKind {
    const fn inner_deformation(self) -> [f32; 3] {
        match self {
            Self::Standard => BABY_INNER_ARMOR_DEFORMATION,
            Self::Piglin => BABY_PIGLIN_ARMOR_DEFORMATION,
        }
    }

    const fn outer_deformation(self) -> [f32; 3] {
        match self {
            Self::Standard => BABY_OUTER_ARMOR_DEFORMATION,
            Self::Piglin => BABY_PIGLIN_ARMOR_DEFORMATION,
        }
    }

    const fn arm_offset(self) -> [f32; 3] {
        match self {
            Self::Standard => [0.0, 0.0, 0.0],
            Self::Piglin => BABY_PIGLIN_ARMOR_ARM_OFFSET,
        }
    }
}

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

    /// Whether this slot uses the inner (leggings) model and the `humanoid_leggings` texture; only
    /// `LEGS` does (vanilla `HumanoidArmorLayer.usesInnerModel`).
    pub(in crate::entity_models) fn uses_inner_model(self) -> bool {
        matches!(self, Self::Legs)
    }

    /// Builds this slot's armor overlay tree: a fresh root carrying exactly the slot's parts, each at
    /// its humanoid bind pose with the inflated armor cubes. The OUTER parts (helmet / chestplate /
    /// boots) are grown by `outer` (`1.0` standard, `1.02` piglin); the inner leggings are fixed. The
    /// host's posed limbs are copied in by [`ModelPart::copy_child_poses_from`] before rendering.
    pub(in crate::entity_models) fn build_tree(self, outer: f32) -> ModelPart {
        let children: Vec<(&'static str, ModelPart)> = match self {
            Self::Head => vec![(
                "head",
                ModelPart::new(
                    HEAD_POSE,
                    vec![armor_cube(HEAD_MIN, HEAD_SIZE, [0.0, 0.0], false, outer)],
                    vec![(
                        "hat",
                        ModelPart::leaf(
                            PART_POSE_ZERO,
                            vec![armor_cube(
                                HEAD_MIN,
                                HEAD_SIZE,
                                [32.0, 0.0],
                                false,
                                outer + 0.5,
                            )],
                        ),
                    )],
                ),
            )],
            Self::Chest => vec![
                (
                    "body",
                    ModelPart::leaf(
                        BODY_POSE,
                        vec![armor_cube(BODY_MIN, BODY_SIZE, [16.0, 16.0], false, outer)],
                    ),
                ),
                (
                    "right_arm",
                    ModelPart::leaf(
                        RIGHT_ARM_POSE,
                        vec![armor_cube(
                            RIGHT_ARM_MIN,
                            ARM_SIZE,
                            [40.0, 16.0],
                            false,
                            outer,
                        )],
                    ),
                ),
                (
                    "left_arm",
                    ModelPart::leaf(
                        LEFT_ARM_POSE,
                        vec![armor_cube(
                            LEFT_ARM_MIN,
                            ARM_SIZE,
                            [40.0, 16.0],
                            true,
                            outer,
                        )],
                    ),
                ),
            ],
            Self::Legs => vec![
                (
                    "body",
                    ModelPart::leaf(BODY_POSE, ARMOR_BODY_INNER.to_vec()),
                ),
                (
                    "right_leg",
                    ModelPart::leaf(RIGHT_LEG_POSE, ARMOR_RIGHT_LEG_INNER.to_vec()),
                ),
                (
                    "left_leg",
                    ModelPart::leaf(LEFT_LEG_POSE, ARMOR_LEFT_LEG_INNER.to_vec()),
                ),
            ],
            Self::Feet => vec![
                (
                    "right_leg",
                    ModelPart::leaf(
                        RIGHT_LEG_POSE,
                        vec![armor_cube(
                            LEG_MIN,
                            LEG_SIZE,
                            [0.0, 16.0],
                            false,
                            outer - 0.1,
                        )],
                    ),
                ),
                (
                    "left_leg",
                    ModelPart::leaf(
                        LEFT_LEG_POSE,
                        vec![armor_cube(
                            LEG_MIN,
                            LEG_SIZE,
                            [0.0, 16.0],
                            true,
                            outer - 0.1,
                        )],
                    ),
                ),
            ],
        };
        ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
    }

    /// Builds a baby humanoid armor model from vanilla `HumanoidModel.createBabyArmorMeshSet`. The
    /// standard zombie/husk/drowned set uses axis-specific baby deformations; the piglin family uses a
    /// uniform `0.7` deformation plus the vanilla baby piglin arm offset.
    pub(in crate::entity_models) fn build_baby_tree(
        self,
        kind: HumanoidBabyArmorKind,
    ) -> ModelPart {
        let outer = kind.outer_deformation();
        let inner = kind.inner_deformation();
        let arm_offset = kind.arm_offset();
        let children: Vec<(&'static str, ModelPart)> = match self {
            Self::Head => vec![(
                "head",
                ModelPart::new(
                    BABY_ARMOR_HEAD_POSE,
                    vec![armor_cube_deformed(
                        BABY_ARMOR_HEAD_MIN,
                        BABY_ARMOR_HEAD_SIZE,
                        [0.0, 0.0],
                        false,
                        outer,
                    )],
                    vec![("hat", ModelPart::leaf(PART_POSE_ZERO, Vec::new()))],
                ),
            )],
            Self::Chest => vec![
                (
                    "body",
                    ModelPart::leaf(
                        BABY_ARMOR_BODY_POSE,
                        vec![armor_cube_deformed(
                            BABY_ARMOR_BODY_MIN,
                            BABY_ARMOR_BODY_SIZE,
                            [0.0, 17.0],
                            false,
                            outer,
                        )],
                    ),
                ),
                (
                    "right_arm",
                    ModelPart::leaf(
                        PartPose {
                            offset: [
                                BABY_ARMOR_RIGHT_ARM_POSE.offset[0] - arm_offset[0],
                                BABY_ARMOR_RIGHT_ARM_POSE.offset[1] + arm_offset[1],
                                BABY_ARMOR_RIGHT_ARM_POSE.offset[2] + arm_offset[2],
                            ],
                            rotation: BABY_ARMOR_RIGHT_ARM_POSE.rotation,
                        },
                        vec![armor_cube_deformed(
                            BABY_ARMOR_ARM_MIN,
                            BABY_ARMOR_ARM_SIZE,
                            [30.0, 25.0],
                            false,
                            outer,
                        )],
                    ),
                ),
                (
                    "left_arm",
                    ModelPart::leaf(
                        PartPose {
                            offset: [
                                BABY_ARMOR_LEFT_ARM_POSE.offset[0] + arm_offset[0],
                                BABY_ARMOR_LEFT_ARM_POSE.offset[1] + arm_offset[1],
                                BABY_ARMOR_LEFT_ARM_POSE.offset[2] + arm_offset[2],
                            ],
                            rotation: BABY_ARMOR_LEFT_ARM_POSE.rotation,
                        },
                        vec![armor_cube_deformed(
                            BABY_ARMOR_ARM_MIN,
                            BABY_ARMOR_ARM_SIZE,
                            [30.0, 17.0],
                            false,
                            outer,
                        )],
                    ),
                ),
            ],
            Self::Legs => vec![
                (
                    "waist",
                    ModelPart::leaf(
                        BABY_ARMOR_WAIST_POSE,
                        vec![armor_cube_deformed(
                            BABY_ARMOR_WAIST_MIN,
                            BABY_ARMOR_WAIST_SIZE,
                            [0.0, 36.0],
                            false,
                            extend_deformation(inner, -0.1),
                        )],
                    ),
                ),
                (
                    "left_leg",
                    ModelPart::leaf(
                        BABY_ARMOR_LEFT_LEG_POSE,
                        vec![armor_cube_deformed(
                            BABY_ARMOR_LEFT_LEG_MIN,
                            BABY_ARMOR_LEG_SIZE,
                            [18.0, 24.0],
                            false,
                            extend_deformation(inner, -0.1),
                        )],
                    ),
                ),
                (
                    "right_leg",
                    ModelPart::leaf(
                        BABY_ARMOR_RIGHT_LEG_POSE,
                        vec![armor_cube_deformed(
                            BABY_ARMOR_RIGHT_LEG_MIN,
                            BABY_ARMOR_LEG_SIZE,
                            [18.0, 17.0],
                            false,
                            extend_deformation(inner, -0.1),
                        )],
                    ),
                ),
            ],
            Self::Feet => vec![
                (
                    "left_leg",
                    ModelPart::new(
                        BABY_ARMOR_LEFT_LEG_POSE,
                        Vec::new(),
                        vec![(
                            "right_foot",
                            ModelPart::leaf(
                                PART_POSE_ZERO,
                                vec![armor_cube_deformed(
                                    BABY_ARMOR_RIGHT_FOOT_MIN,
                                    BABY_ARMOR_FOOT_SIZE,
                                    [0.0, 25.0],
                                    false,
                                    outer,
                                )],
                            ),
                        )],
                    ),
                ),
                (
                    "right_leg",
                    ModelPart::new(
                        BABY_ARMOR_RIGHT_LEG_POSE,
                        Vec::new(),
                        vec![(
                            "left_foot",
                            ModelPart::leaf(
                                PART_POSE_ZERO,
                                vec![armor_cube_deformed(
                                    BABY_ARMOR_LEFT_FOOT_MIN,
                                    BABY_ARMOR_FOOT_SIZE,
                                    [0.0, 29.0],
                                    true,
                                    outer,
                                )],
                            ),
                        )],
                    ),
                ),
            ],
        };
        ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
    }
}

/// The vanilla `DyedItemColor.LEATHER_COLOR` (`0xA06540`), leather's `colorWhenUndyed`.
const LEATHER_DEFAULT_COLOR: u32 = 0x00A0_6540;

/// The vanilla `EquipmentLayerRenderer.getColorForLayer` per-layer tint: leather is the only dyeable
/// humanoid material, so it tints by the worn item's `DyedItemColor` when custom-dyed and otherwise by
/// its default `DyedItemColor.LEATHER_COLOR` (`0xA06540`); every other material is non-dyeable and
/// renders white (vanilla color `-1`), ignoring any stray dye. `dye` is the projected per-slot
/// `dyed_color` component (a packed RGB), forced opaque here exactly as `DyedItemColor.getOrDefault`
/// applies `ARGB.opaque` before `getColorForLayer` reads it.
pub(in crate::entity_models) fn armor_layer_tint(
    material: EntityArmorMaterial,
    dye: Option<u32>,
) -> [f32; 4] {
    match material {
        EntityArmorMaterial::Leather => opaque_rgb_to_tint(dye.unwrap_or(LEATHER_DEFAULT_COLOR)),
        _ => [1.0, 1.0, 1.0, 1.0],
    }
}

/// Unpack a 24-bit RGB color into a fully-opaque `[r, g, b, a]` float tint (vanilla `ARGB.opaque`
/// forces alpha to `0xFF`, so the incoming high byte is discarded).
fn opaque_rgb_to_tint(rgb: u32) -> [f32; 4] {
    [
        ((rgb >> 16) & 0xFF) as f32 / 255.0,
        ((rgb >> 8) & 0xFF) as f32 / 255.0,
        (rgb & 0xFF) as f32 / 255.0,
        1.0,
    ]
}

/// The equipment-asset texture for a given humanoid armor material in a given slot and age layer.
/// Adult armor uses the `humanoid_leggings` variant for the inner (legs) slot and the `humanoid`
/// variant otherwise; baby armor always uses `EquipmentClientInfo.LayerType.HUMANOID_BABY`. Materials
/// without humanoid equipment textures, such as wolf armor's `ArmadilloScute`, return `None`.
pub(in crate::entity_models) fn armor_slot_texture_for_layer(
    material: EntityArmorMaterial,
    slot: HumanoidArmorSlot,
    baby: bool,
) -> Option<EntityModelTextureRef> {
    use EntityArmorMaterial::*;
    if baby {
        return Some(match material {
            Leather => ARMOR_LEATHER_BABY_HUMANOID_TEXTURE_REF,
            Copper => ARMOR_COPPER_BABY_HUMANOID_TEXTURE_REF,
            Chainmail => ARMOR_CHAINMAIL_BABY_HUMANOID_TEXTURE_REF,
            Iron => ARMOR_IRON_BABY_HUMANOID_TEXTURE_REF,
            Gold => ARMOR_GOLD_BABY_HUMANOID_TEXTURE_REF,
            Diamond => ARMOR_DIAMOND_BABY_HUMANOID_TEXTURE_REF,
            TurtleScute => ARMOR_TURTLE_SCUTE_BABY_HUMANOID_TEXTURE_REF,
            Netherite => ARMOR_NETHERITE_BABY_HUMANOID_TEXTURE_REF,
            ArmadilloScute => return None,
        });
    }
    Some(if slot.uses_inner_model() {
        match material {
            Leather => ARMOR_LEATHER_LEGGINGS_TEXTURE_REF,
            Copper => ARMOR_COPPER_LEGGINGS_TEXTURE_REF,
            Chainmail => ARMOR_CHAINMAIL_LEGGINGS_TEXTURE_REF,
            Iron => ARMOR_IRON_LEGGINGS_TEXTURE_REF,
            Gold => ARMOR_GOLD_LEGGINGS_TEXTURE_REF,
            Diamond => ARMOR_DIAMOND_LEGGINGS_TEXTURE_REF,
            Netherite => ARMOR_NETHERITE_LEGGINGS_TEXTURE_REF,
            TurtleScute => ARMOR_TURTLE_SCUTE_HUMANOID_TEXTURE_REF,
            ArmadilloScute => return None,
        }
    } else {
        match material {
            Leather => ARMOR_LEATHER_HUMANOID_TEXTURE_REF,
            Copper => ARMOR_COPPER_HUMANOID_TEXTURE_REF,
            Chainmail => ARMOR_CHAINMAIL_HUMANOID_TEXTURE_REF,
            Iron => ARMOR_IRON_HUMANOID_TEXTURE_REF,
            Gold => ARMOR_GOLD_HUMANOID_TEXTURE_REF,
            Diamond => ARMOR_DIAMOND_HUMANOID_TEXTURE_REF,
            TurtleScute => ARMOR_TURTLE_SCUTE_HUMANOID_TEXTURE_REF,
            Netherite => ARMOR_NETHERITE_HUMANOID_TEXTURE_REF,
            ArmadilloScute => return None,
        }
    })
}
