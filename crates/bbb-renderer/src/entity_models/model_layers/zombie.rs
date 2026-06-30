use super::{
    apply_head_look, apply_humanoid_attack_animation, apply_humanoid_leg_swing_named,
    apply_humanoid_mob_spear_arm_poses, apply_humanoid_stab_attack_animation, apply_humanoid_walk,
    apply_zombie_arms_held_out_named, drowned_outer_root, humanoid_arm_bob_pose, PartPose,
    PART_POSE_ZERO,
};
use crate::entity_models::catalog::ZombieVariantModelFamily;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_ZOMBIE: &str = "minecraft:zombie#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOMBIE_BABY: &str = "minecraft:zombie_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_HUSK: &str = "minecraft:husk#main";
pub(in crate::entity_models) const MODEL_LAYER_HUSK_BABY: &str = "minecraft:husk_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_DROWNED: &str = "minecraft:drowned#main";
pub(in crate::entity_models) const MODEL_LAYER_DROWNED_BABY: &str = "minecraft:drowned_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOMBIE_VILLAGER: &str =
    "minecraft:zombie_villager#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOMBIE_VILLAGER_NO_HAT: &str =
    "minecraft:zombie_villager_no_hat#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOMBIE_VILLAGER_BABY: &str =
    "minecraft:zombie_villager_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOMBIE_VILLAGER_BABY_NO_HAT: &str =
    "minecraft:zombie_villager_baby_no_hat#main";

pub(in crate::entity_models) const ZOMBIE_GREEN: [f32; 4] = [0.33, 0.62, 0.34, 1.0];
pub(in crate::entity_models) const HUSK_TAN: [f32; 4] = [0.60, 0.50, 0.31, 1.0];
pub(in crate::entity_models) const DROWNED_BLUE: [f32; 4] = [0.23, 0.48, 0.55, 1.0];
pub(in crate::entity_models) const ZOMBIE_VILLAGER_ROBE: [f32; 4] = [0.38, 0.55, 0.34, 1.0];

// Vanilla 26.1 ModelLayers.ZOMBIE: HumanoidModel.createMesh(CubeDeformation.NONE, 0.0F). Each cube
// carries both render paths' data: the colored debug tint and the textured `uv_size`/`texOffs`/
// `mirror`. The deformed hat / baby-head overlay inflate their colored geometry but keep the base box
// as `uv_size` (the squid precedent). The left arm/leg share the colored geometry but carry the
// mirrored zombie UV.
pub(in crate::entity_models) const ADULT_ZOMBIE_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -8.0, -4.0],
    [8.0, 8.0, 8.0],
    ZOMBIE_GREEN,
    [8.0, 8.0, 8.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ADULT_ZOMBIE_HAT: [ModelCube; 1] = [ModelCube::new(
    [-4.5, -8.5, -4.5],
    [9.0, 9.0, 9.0],
    ZOMBIE_GREEN,
    [8.0, 8.0, 8.0],
    [32.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ADULT_ZOMBIE_BODY: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.0, -2.0],
    [8.0, 12.0, 4.0],
    ZOMBIE_GREEN,
    [8.0, 12.0, 4.0],
    [16.0, 16.0],
    false,
)];

pub(in crate::entity_models) const ADULT_ZOMBIE_RIGHT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    ZOMBIE_GREEN,
    [4.0, 12.0, 4.0],
    [40.0, 16.0],
    false,
)];

pub(in crate::entity_models) const ADULT_ZOMBIE_LEFT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    ZOMBIE_GREEN,
    [4.0, 12.0, 4.0],
    [40.0, 16.0],
    true,
)];

pub(in crate::entity_models) const ADULT_ZOMBIE_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    ZOMBIE_GREEN,
    [4.0, 12.0, 4.0],
    [0.0, 16.0],
    false,
)];

pub(in crate::entity_models) const ADULT_ZOMBIE_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    ZOMBIE_GREEN,
    [4.0, 12.0, 4.0],
    [0.0, 16.0],
    true,
)];

/// Shared adult humanoid limb part poses (vanilla `HumanoidModel.createMesh`).
const ADULT_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const ADULT_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const ADULT_ZOMBIE_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-1.9, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const ADULT_ZOMBIE_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [1.9, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const BABY_ZOMBIE_BODY: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -2.5, -1.0],
    [4.0, 5.0, 2.0],
    ZOMBIE_GREEN,
    [4.0, 5.0, 2.0],
    [16.0, 16.0],
    false,
)];

pub(in crate::entity_models) const BABY_ZOMBIE_HEAD: [ModelCube; 2] = [
    ModelCube::new(
        [-3.0, -6.25, -3.0],
        [6.0, 6.0, 6.0],
        ZOMBIE_GREEN,
        [6.0, 6.0, 6.0],
        [3.0, 3.0],
        false,
    ),
    // BabyZombieModel bakes CubeDeformation(0.25F) into ModelPart.Cube bounds; the textured UV keeps
    // the base 6x6x6 box.
    ModelCube::new(
        [-3.25, -6.4, -3.25],
        [6.5, 6.5, 6.5],
        ZOMBIE_GREEN,
        [6.0, 6.0, 6.0],
        [35.0, 3.0],
        false,
    ),
];

pub(in crate::entity_models) const BABY_ZOMBIE_RIGHT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -0.5, -1.0],
    [2.0, 5.0, 2.0],
    ZOMBIE_GREEN,
    [2.0, 5.0, 2.0],
    [36.0, 16.0],
    false,
)];

pub(in crate::entity_models) const BABY_ZOMBIE_LEFT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -0.5, -1.0],
    [2.0, 5.0, 2.0],
    ZOMBIE_GREEN,
    [2.0, 5.0, 2.0],
    [28.0, 16.0],
    false,
)];

pub(in crate::entity_models) const BABY_ZOMBIE_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 4.0, 2.0],
    ZOMBIE_GREEN,
    [2.0, 4.0, 2.0],
    [8.0, 16.0],
    false,
)];

pub(in crate::entity_models) const BABY_ZOMBIE_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 4.0, 2.0],
    ZOMBIE_GREEN,
    [2.0, 4.0, 2.0],
    [0.0, 16.0],
    false,
)];

/// Baby zombie part poses (vanilla `BabyZombieModel.createBodyLayer`).
const BABY_ZOMBIE_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 17.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ZOMBIE_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 15.25, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ZOMBIE_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-3.0, 15.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ZOMBIE_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [3.0, 15.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ZOMBIE_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-1.0, 20.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ZOMBIE_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [1.0, 20.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const MODEL_LAYER_DROWNED_BABY_OUTER_LAYER: &str =
    "minecraft:drowned_baby#outer";

// Vanilla 26.1 ModelLayers.DROWNED_BABY_OUTER_LAYER:
// BabyDrownedModel.createBodyLayer(CubeDeformation(0.25F)) = BabyZombieModel.createBodyLayer(0.25),
// 64x64. The baby drowned outer layer is a DISTINCT baby-zombie inflated mesh, NOT the adult humanoid
// + drowned left-limb overrides — `BabyZombieModel` never applies the drowned `texOffs`, so the left
// arm/leg keep the baby zombie's own `texOffs(28, 16)`/`texOffs(0, 16)`. Each cube is the base
// `BABY_ZOMBIE_*` box inflated by 0.25 (`min -= 0.25`, `size += 0.5`), keeping the base box as
// `uv_size`. The head still carries TWO boxes (the second is the literal-`0.25` overlay box, identical
// to the base baby head's `texOffs(35, 3)` box). These render textured-only, so `color` is an unused
// placeholder.
const DROWNED_OUTER_PLACEHOLDER_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

pub(in crate::entity_models) const BABY_DROWNED_OUTER_BODY: [ModelCube; 1] = [ModelCube::new(
    [-2.25, -2.75, -1.25],
    [4.5, 5.5, 2.5],
    DROWNED_OUTER_PLACEHOLDER_COLOR,
    [4.0, 5.0, 2.0],
    [16.0, 16.0],
    false,
)];

pub(in crate::entity_models) const BABY_DROWNED_OUTER_HEAD: [ModelCube; 2] = [
    ModelCube::new(
        [-3.25, -6.5, -3.25],
        [6.5, 6.5, 6.5],
        DROWNED_OUTER_PLACEHOLDER_COLOR,
        [6.0, 6.0, 6.0],
        [3.0, 3.0],
        false,
    ),
    ModelCube::new(
        [-3.25, -6.4, -3.25],
        [6.5, 6.5, 6.5],
        DROWNED_OUTER_PLACEHOLDER_COLOR,
        [6.0, 6.0, 6.0],
        [35.0, 3.0],
        false,
    ),
];

pub(in crate::entity_models) const BABY_DROWNED_OUTER_RIGHT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.25, -0.75, -1.25],
    [2.5, 5.5, 2.5],
    DROWNED_OUTER_PLACEHOLDER_COLOR,
    [2.0, 5.0, 2.0],
    [36.0, 16.0],
    false,
)];

pub(in crate::entity_models) const BABY_DROWNED_OUTER_LEFT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.25, -0.75, -1.25],
    [2.5, 5.5, 2.5],
    DROWNED_OUTER_PLACEHOLDER_COLOR,
    [2.0, 5.0, 2.0],
    [28.0, 16.0],
    false,
)];

pub(in crate::entity_models) const BABY_DROWNED_OUTER_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.25, -0.25, -1.25],
    [2.5, 4.5, 2.5],
    DROWNED_OUTER_PLACEHOLDER_COLOR,
    [2.0, 4.0, 2.0],
    [8.0, 16.0],
    false,
)];

pub(in crate::entity_models) const BABY_DROWNED_OUTER_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.25, -0.25, -1.25],
    [2.5, 4.5, 2.5],
    DROWNED_OUTER_PLACEHOLDER_COLOR,
    [2.0, 4.0, 2.0],
    [0.0, 16.0],
    false,
)];

/// Builds the baby drowned outer-layer root (vanilla `BabyDrownedModel.createBodyLayer(0.25)` =
/// `BabyZombieModel.createBodyLayer(0.25)`), mirroring the base baby zombie tree shape (body first,
/// then the head leaf, arms, legs — the vanilla empty `hat` child carries no cubes, so it is omitted)
/// at the shared `BABY_ZOMBIE_*_POSE` offsets, so the shared `apply_zombie_family_anim` poses it like
/// the base baby body.
fn baby_drowned_outer_root() -> ModelPart {
    let children = vec![
        (
            "body",
            part(BABY_ZOMBIE_BODY_POSE, &BABY_DROWNED_OUTER_BODY),
        ),
        (
            "head",
            part(BABY_ZOMBIE_HEAD_POSE, &BABY_DROWNED_OUTER_HEAD),
        ),
        (
            "right_arm",
            part(BABY_ZOMBIE_RIGHT_ARM_POSE, &BABY_DROWNED_OUTER_RIGHT_ARM),
        ),
        (
            "left_arm",
            part(BABY_ZOMBIE_LEFT_ARM_POSE, &BABY_DROWNED_OUTER_LEFT_ARM),
        ),
        (
            "right_leg",
            part(BABY_ZOMBIE_RIGHT_LEG_POSE, &BABY_DROWNED_OUTER_RIGHT_LEG),
        ),
        (
            "left_leg",
            part(BABY_ZOMBIE_LEFT_LEG_POSE, &BABY_DROWNED_OUTER_LEFT_LEG),
        ),
    ];
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

// Vanilla 26.1 ZombieVillagerModel.createBodyLayer().
pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_HEAD: [ModelCube; 2] = [
    ModelCube::new(
        [-4.0, -10.0, -4.0],
        [8.0, 10.0, 8.0],
        ZOMBIE_VILLAGER_ROBE,
        [8.0, 10.0, 8.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-1.0, -3.0, -6.0],
        [2.0, 4.0, 2.0],
        ZOMBIE_VILLAGER_ROBE,
        [2.0, 4.0, 2.0],
        [24.0, 0.0],
        false,
    ),
];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_HAT: [ModelCube; 1] = [ModelCube::new(
    [-4.5, -10.5, -4.5],
    [9.0, 11.0, 9.0],
    ZOMBIE_VILLAGER_ROBE,
    [8.0, 10.0, 8.0],
    [32.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_HAT_RIM: [ModelCube; 1] =
    [ModelCube::new(
        [-8.0, -8.0, -6.0],
        [16.0, 16.0, 1.0],
        ZOMBIE_VILLAGER_ROBE,
        [16.0, 16.0, 1.0],
        [30.0, 47.0],
        false,
    )];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_BODY: [ModelCube; 2] = [
    ModelCube::new(
        [-4.0, 0.0, -3.0],
        [8.0, 12.0, 6.0],
        ZOMBIE_VILLAGER_ROBE,
        [8.0, 12.0, 6.0],
        [16.0, 20.0],
        false,
    ),
    ModelCube::new(
        [-4.05, -0.05, -3.05],
        [8.1, 20.1, 6.1],
        ZOMBIE_VILLAGER_ROBE,
        [8.0, 20.0, 6.0],
        [0.0, 38.0],
        false,
    ),
];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_RIGHT_ARM: [ModelCube; 1] =
    [ModelCube::new(
        [-3.0, -2.0, -2.0],
        [4.0, 12.0, 4.0],
        ZOMBIE_VILLAGER_ROBE,
        [4.0, 12.0, 4.0],
        [44.0, 22.0],
        false,
    )];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_LEFT_ARM: [ModelCube; 1] =
    [ModelCube::new(
        [-1.0, -2.0, -2.0],
        [4.0, 12.0, 4.0],
        ZOMBIE_VILLAGER_ROBE,
        [4.0, 12.0, 4.0],
        [44.0, 22.0],
        true,
    )];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_RIGHT_LEG: [ModelCube; 1] =
    [ModelCube::new(
        [-2.0, 0.0, -2.0],
        [4.0, 12.0, 4.0],
        ZOMBIE_VILLAGER_ROBE,
        [4.0, 12.0, 4.0],
        [0.0, 22.0],
        false,
    )];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_LEFT_LEG: [ModelCube; 1] =
    [ModelCube::new(
        [-2.0, 0.0, -2.0],
        [4.0, 12.0, 4.0],
        ZOMBIE_VILLAGER_ROBE,
        [4.0, 12.0, 4.0],
        [0.0, 22.0],
        true,
    )];

/// Adult zombie-villager hat-rim child pose (vanilla `ZombieVillagerModel`): the rim hangs off the
/// hat, tilted back by `-π/2`.
const ADULT_ZOMBIE_VILLAGER_HAT_RIM_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
};
const ADULT_ZOMBIE_VILLAGER_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-2.0, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const ADULT_ZOMBIE_VILLAGER_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [2.0, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

// Vanilla 26.1 BabyZombieVillagerModel.createBodyLayer().
pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_BODY: [ModelCube; 2] = [
    ModelCube::new(
        [-2.0, -2.75, -1.5],
        [4.0, 5.0, 3.0],
        ZOMBIE_VILLAGER_ROBE,
        [4.0, 5.0, 3.0],
        [0.0, 15.0],
        false,
    ),
    ModelCube::new(
        [-2.1, -2.85, -1.6],
        [4.2, 6.2, 3.2],
        ZOMBIE_VILLAGER_ROBE,
        [4.0, 6.0, 3.0],
        [16.0, 22.0],
        false,
    ),
];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -8.0, -3.5],
    [8.0, 8.0, 7.0],
    ZOMBIE_VILLAGER_ROBE,
    [8.0, 8.0, 7.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_HAT: [ModelCube; 1] = [ModelCube::new(
    [-4.3, -4.3, -3.8],
    [8.6, 8.6, 7.6],
    ZOMBIE_VILLAGER_ROBE,
    [8.0, 8.0, 7.0],
    [0.0, 31.0],
    false,
)];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_HAT_RIM: [ModelCube; 1] = [ModelCube::new(
    [-7.0, -0.5, -6.0],
    [14.0, 1.0, 12.0],
    ZOMBIE_VILLAGER_ROBE,
    [14.0, 1.0, 12.0],
    [0.0, 46.0],
    false,
)];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_NOSE: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -1.0, -0.5],
    [2.0, 2.0, 1.0],
    ZOMBIE_VILLAGER_ROBE,
    [2.0, 2.0, 1.0],
    [23.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_RIGHT_ARM: [ModelCube; 1] =
    [ModelCube::new(
        [-1.0, -0.5, -1.0],
        [2.0, 5.0, 2.0],
        ZOMBIE_VILLAGER_ROBE,
        [2.0, 5.0, 2.0],
        [24.0, 15.0],
        false,
    )];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_LEFT_ARM: [ModelCube; 1] =
    [ModelCube::new(
        [-1.0, -0.5, -1.0],
        [2.0, 5.0, 2.0],
        ZOMBIE_VILLAGER_ROBE,
        [2.0, 5.0, 2.0],
        [16.0, 15.0],
        false,
    )];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_RIGHT_LEG: [ModelCube; 1] =
    [ModelCube::new(
        [-1.0, -0.5, -1.0],
        [2.0, 3.0, 2.0],
        ZOMBIE_VILLAGER_ROBE,
        [2.0, 3.0, 2.0],
        [8.0, 23.0],
        false,
    )];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_LEFT_LEG: [ModelCube; 1] =
    [ModelCube::new(
        [-1.0, -0.5, -1.0],
        [2.0, 3.0, 2.0],
        ZOMBIE_VILLAGER_ROBE,
        [2.0, 3.0, 2.0],
        [0.0, 23.0],
        false,
    )];

/// Baby zombie-villager head child poses (vanilla `BabyZombieVillagerModel`): the hat, the hat rim,
/// and the nose all hang off the head.
const BABY_ZOMBIE_VILLAGER_HAT_POSE: PartPose = PartPose {
    offset: [0.0, -4.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ZOMBIE_VILLAGER_HAT_RIM_POSE: PartPose = PartPose {
    offset: [0.0, -4.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ZOMBIE_VILLAGER_NOSE_POSE: PartPose = PartPose {
    offset: [0.0, -1.0, -4.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ZOMBIE_VILLAGER_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 18.75, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ZOMBIE_VILLAGER_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 16.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ZOMBIE_VILLAGER_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-3.0, 15.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ZOMBIE_VILLAGER_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [3.0, 15.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ZOMBIE_VILLAGER_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-1.0, 21.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_ZOMBIE_VILLAGER_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [1.0, 21.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds a leaf part at `pose` carrying `cubes`.
fn part(pose: PartPose, cubes: &[ModelCube]) -> ModelPart {
    ModelPart::leaf(pose, cubes.to_vec())
}

/// Builds the unified plain-zombie root for `baby`, with the vanilla `HumanoidModel` child names
/// (`head` -> `hat`, `body`, `right_arm`, `left_arm`, `right_leg`, `left_leg`). The husk and drowned
/// reuse this tree (only the texture/recolor differs). The adult lists the head first; the baby lists
/// the body first (vanilla `BabyZombieModel`), so the emit order is preserved to keep the mesh
/// byte-identical.
fn zombie_tree(baby: bool) -> ModelPart {
    if baby {
        let children = vec![
            ("body", part(BABY_ZOMBIE_BODY_POSE, &BABY_ZOMBIE_BODY)),
            ("head", part(BABY_ZOMBIE_HEAD_POSE, &BABY_ZOMBIE_HEAD)),
            (
                "right_arm",
                part(BABY_ZOMBIE_RIGHT_ARM_POSE, &BABY_ZOMBIE_RIGHT_ARM),
            ),
            (
                "left_arm",
                part(BABY_ZOMBIE_LEFT_ARM_POSE, &BABY_ZOMBIE_LEFT_ARM),
            ),
            (
                "right_leg",
                part(BABY_ZOMBIE_RIGHT_LEG_POSE, &BABY_ZOMBIE_RIGHT_LEG),
            ),
            (
                "left_leg",
                part(BABY_ZOMBIE_LEFT_LEG_POSE, &BABY_ZOMBIE_LEFT_LEG),
            ),
        ];
        return ModelPart::new(PART_POSE_ZERO, Vec::new(), children);
    }
    let head = ModelPart::new(
        PART_POSE_ZERO,
        ADULT_ZOMBIE_HEAD.to_vec(),
        vec![("hat", part(PART_POSE_ZERO, &ADULT_ZOMBIE_HAT))],
    );
    let children = vec![
        ("head", head),
        ("body", part(PART_POSE_ZERO, &ADULT_ZOMBIE_BODY)),
        (
            "right_arm",
            part(ADULT_RIGHT_ARM_POSE, &ADULT_ZOMBIE_RIGHT_ARM),
        ),
        (
            "left_arm",
            part(ADULT_LEFT_ARM_POSE, &ADULT_ZOMBIE_LEFT_ARM),
        ),
        (
            "right_leg",
            part(ADULT_ZOMBIE_RIGHT_LEG_POSE, &ADULT_ZOMBIE_RIGHT_LEG),
        ),
        (
            "left_leg",
            part(ADULT_ZOMBIE_LEFT_LEG_POSE, &ADULT_ZOMBIE_LEFT_LEG),
        ),
    ];
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Builds the unified zombie-villager root for `baby`, with the vanilla child names. The adult head
/// parents the hat (which parents the hat rim); the baby lists the body first and parents the hat,
/// hat rim, and nose under the head. The emit order is preserved to keep the mesh byte-identical.
fn zombie_villager_tree(baby: bool) -> ModelPart {
    if baby {
        let head = ModelPart::new(
            BABY_ZOMBIE_VILLAGER_HEAD_POSE,
            BABY_ZOMBIE_VILLAGER_HEAD.to_vec(),
            vec![
                (
                    "hat",
                    part(BABY_ZOMBIE_VILLAGER_HAT_POSE, &BABY_ZOMBIE_VILLAGER_HAT),
                ),
                (
                    "hat_rim",
                    part(
                        BABY_ZOMBIE_VILLAGER_HAT_RIM_POSE,
                        &BABY_ZOMBIE_VILLAGER_HAT_RIM,
                    ),
                ),
                (
                    "nose",
                    part(BABY_ZOMBIE_VILLAGER_NOSE_POSE, &BABY_ZOMBIE_VILLAGER_NOSE),
                ),
            ],
        );
        let children = vec![
            (
                "body",
                part(BABY_ZOMBIE_VILLAGER_BODY_POSE, &BABY_ZOMBIE_VILLAGER_BODY),
            ),
            ("head", head),
            (
                "right_arm",
                part(
                    BABY_ZOMBIE_VILLAGER_RIGHT_ARM_POSE,
                    &BABY_ZOMBIE_VILLAGER_RIGHT_ARM,
                ),
            ),
            (
                "left_arm",
                part(
                    BABY_ZOMBIE_VILLAGER_LEFT_ARM_POSE,
                    &BABY_ZOMBIE_VILLAGER_LEFT_ARM,
                ),
            ),
            (
                "right_leg",
                part(
                    BABY_ZOMBIE_VILLAGER_RIGHT_LEG_POSE,
                    &BABY_ZOMBIE_VILLAGER_RIGHT_LEG,
                ),
            ),
            (
                "left_leg",
                part(
                    BABY_ZOMBIE_VILLAGER_LEFT_LEG_POSE,
                    &BABY_ZOMBIE_VILLAGER_LEFT_LEG,
                ),
            ),
        ];
        return ModelPart::new(PART_POSE_ZERO, Vec::new(), children);
    }
    let hat = ModelPart::new(
        PART_POSE_ZERO,
        ADULT_ZOMBIE_VILLAGER_HAT.to_vec(),
        vec![(
            "hat_rim",
            part(
                ADULT_ZOMBIE_VILLAGER_HAT_RIM_POSE,
                &ADULT_ZOMBIE_VILLAGER_HAT_RIM,
            ),
        )],
    );
    let head = ModelPart::new(
        PART_POSE_ZERO,
        ADULT_ZOMBIE_VILLAGER_HEAD.to_vec(),
        vec![("hat", hat)],
    );
    let children = vec![
        ("head", head),
        ("body", part(PART_POSE_ZERO, &ADULT_ZOMBIE_VILLAGER_BODY)),
        (
            "right_arm",
            part(ADULT_RIGHT_ARM_POSE, &ADULT_ZOMBIE_VILLAGER_RIGHT_ARM),
        ),
        (
            "left_arm",
            part(ADULT_LEFT_ARM_POSE, &ADULT_ZOMBIE_VILLAGER_LEFT_ARM),
        ),
        (
            "right_leg",
            part(
                ADULT_ZOMBIE_VILLAGER_RIGHT_LEG_POSE,
                &ADULT_ZOMBIE_VILLAGER_RIGHT_LEG,
            ),
        ),
        (
            "left_leg",
            part(
                ADULT_ZOMBIE_VILLAGER_LEFT_LEG_POSE,
                &ADULT_ZOMBIE_VILLAGER_LEFT_LEG,
            ),
        ),
    ];
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Vanilla `ZombieModel.setupAnim` (`super.setupAnim` then `AnimationUtils.animateZombieArms`),
/// shared by the plain zombie and every zombie variant (husk, drowned, zombie villager): look the
/// head ([`apply_head_look`] on `head`), then either run the vanilla STAB swing branch for an attack-arm
/// spear or override the arms with the held-out `animateZombieArms` pose
/// ([`apply_zombie_arms_held_out_named`], `isAggressive`-driven and swung by `attack_anim`).
fn apply_zombie_family_anim(root: &mut ModelPart, instance: &EntityModelInstance) {
    let render_state = &instance.render_state;
    apply_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
    );
    if render_state.main_hand_swing_is_stab {
        apply_humanoid_walk(
            root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
            render_state.age_in_ticks,
        );
        apply_humanoid_mob_spear_arm_poses(
            root,
            render_state.head_yaw,
            render_state.head_pitch,
            render_state.humanoid_mob_main_hand_spear_pose,
            render_state.humanoid_mob_off_hand_spear_pose,
            render_state.swim_amount,
        );
        apply_humanoid_stab_attack_animation(
            root,
            render_state.attack_anim,
            render_state.attack_arm_off_hand,
            1.0,
        );
        // Vanilla `AnimationUtils.animateZombieArms` still runs after `super.setupAnim`, but the STAB branch
        // skips the held-out arm rewrite and only applies `bobArms`.
        for name in ["right_arm", "left_arm"] {
            let arm = root.child_mut(name);
            arm.pose = humanoid_arm_bob_pose(arm.pose, render_state.age_in_ticks);
        }
    } else {
        apply_humanoid_leg_swing_named(
            root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
        );
        apply_humanoid_attack_animation(
            root,
            render_state.attack_anim,
            render_state.attack_arm_off_hand,
            render_state.head_pitch,
            1.0,
        );
        apply_zombie_arms_held_out_named(
            root,
            render_state.is_aggressive,
            render_state.attack_anim,
            render_state.age_in_ticks,
        );
    }
}

/// Vanilla `DrownedModel.setupAnim` `THROW_TRIDENT` override: after the held-out zombie arms, the main
/// (right) arm raises the trident straight overhead to throw (`rightArm.xRot = rightArm.xRot * 0.5 - π`,
/// `rightArm.yRot = 0`). Only the main arm is posed (`getMainArm() == arm`); left-handed mobs are not
/// projected, so this always poses the right arm.
fn apply_drowned_throw_trident(root: &mut ModelPart) {
    let right = root.child_mut("right_arm");
    right.pose.rotation[0] = right.pose.rotation[0] * 0.5 - std::f32::consts::PI;
    right.pose.rotation[1] = 0.0;
}

/// Vanilla `DrownedModel.setupAnim` swim override, applied after the trident pose. `swimAmount`
/// blends both arms toward the folded-back swimming pose, adds the out-of-phase arm/leg sine wave,
/// rolls the arms, and zeroes the head pitch.
fn apply_drowned_swim_pose(root: &mut ModelPart, instance: &EntityModelInstance) {
    let swim_amount = instance.render_state.swim_amount;
    if swim_amount <= 0.0 {
        return;
    }

    let age_wave = (0.1 * instance.render_state.age_in_ticks).sin();
    let right = root.child_mut("right_arm");
    right.pose.rotation[0] = rot_lerp_rad(
        swim_amount,
        right.pose.rotation[0],
        -std::f32::consts::PI * 4.0 / 5.0,
    ) + swim_amount * 0.35 * age_wave;
    right.pose.rotation[2] = rot_lerp_rad(swim_amount, right.pose.rotation[2], -0.15);

    let left = root.child_mut("left_arm");
    left.pose.rotation[0] = rot_lerp_rad(
        swim_amount,
        left.pose.rotation[0],
        -std::f32::consts::PI * 4.0 / 5.0,
    ) - swim_amount * 0.35 * age_wave;
    left.pose.rotation[2] = rot_lerp_rad(swim_amount, left.pose.rotation[2], 0.15);

    root.child_mut("left_leg").pose.rotation[0] -= swim_amount * 0.55 * age_wave;
    root.child_mut("right_leg").pose.rotation[0] += swim_amount * 0.55 * age_wave;
    root.child_mut("head").pose.rotation[0] = 0.0;
}

/// Vanilla `Mth.rotLerpRad(a, from, to)`: wraps the angular delta to `[-π, π)` before lerping.
fn rot_lerp_rad(a: f32, from: f32, to: f32) -> f32 {
    let mut diff = to - from;
    while diff < -std::f32::consts::PI {
        diff += 2.0 * std::f32::consts::PI;
    }
    while diff >= std::f32::consts::PI {
        diff -= 2.0 * std::f32::consts::PI;
    }
    from + a * diff
}

/// Mutable zombie model, mirroring vanilla `ZombieModel` (an `AbstractZombieModel` over `HumanoidModel`).
/// The unified tree is built for the selected `baby` layout with the vanilla child names. `setup_anim`
/// runs the shared [`apply_zombie_family_anim`].
pub(in crate::entity_models) struct ZombieModel {
    root: ModelPart,
}

impl ZombieModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        Self {
            root: zombie_tree(baby),
        }
    }
}

impl EntityModel for ZombieModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_zombie_family_anim(&mut self.root, instance);
    }
}

/// Mutable zombie-variant model, mirroring vanilla `HuskRenderer`/`DrownedRenderer`/
/// `ZombieVillagerRenderer` — all of which inherit `ZombieModel.setupAnim`. The unified tree is
/// selected by `family`/`baby`: the husk and drowned reuse the plain zombie body, the zombie villager
/// builds its own robed tree; `setup_anim` runs the shared [`apply_zombie_family_anim`], then an aggressive
/// trident-holding drowned raises the trident to throw ([`apply_drowned_throw_trident`]) and then applies
/// the `swimAmount` limb override. The per-family root scale (husk) and the colored recolor / textured
/// texture are supplied by the caller; the profession overlays defer.
pub(in crate::entity_models) struct ZombieVariantModel {
    root: ModelPart,
    family: ZombieVariantModelFamily,
}

impl ZombieVariantModel {
    pub(in crate::entity_models) fn new(family: ZombieVariantModelFamily, baby: bool) -> Self {
        let root = match family {
            ZombieVariantModelFamily::ZombieVillager => zombie_villager_tree(baby),
            _ => zombie_tree(baby),
        };
        Self { root, family }
    }
}

impl EntityModel for ZombieVariantModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_zombie_family_anim(&mut self.root, instance);
        // Vanilla `DrownedModel.setupAnim` raises the trident to throw after the held-out zombie arms.
        // Only the drowned sets this flag (the husk / zombie villager never throw).
        if instance.render_state.drowned_throw_trident {
            apply_drowned_throw_trident(&mut self.root);
        }
        if self.family == ZombieVariantModelFamily::Drowned {
            apply_drowned_swim_pose(&mut self.root, instance);
        }
    }
}

/// The drowned outer-layer overlay model, mirroring vanilla `DrownedOuterLayer`: a second white
/// `coloredCutoutModelCopyLayerRender` copy of the inflated drowned body drawn with
/// `drowned_outer_layer.png` (adult) / `drowned_outer_layer_baby.png` (baby). It is a `DrownedModel`
/// (adult `DrownedModel.createBodyLayer(0.25)`) / `BabyDrownedModel` (baby
/// `BabyZombieModel.createBodyLayer(0.25)`) in its own right, so it is posed by the exact same animator
/// as the base (the shared `ZombieModel.setupAnim` plus the drowned trident throw and swim override),
/// keeping the inflated shell glued to the limbs.
pub(in crate::entity_models) struct DrownedOuterModel {
    root: ModelPart,
}

impl DrownedOuterModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        Self {
            root: if baby {
                baby_drowned_outer_root()
            } else {
                drowned_outer_root()
            },
        }
    }
}

impl EntityModel for DrownedOuterModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_zombie_family_anim(&mut self.root, instance);
        if instance.render_state.drowned_throw_trident {
            apply_drowned_throw_trident(&mut self.root);
        }
        apply_drowned_swim_pose(&mut self.root, instance);
    }
}
