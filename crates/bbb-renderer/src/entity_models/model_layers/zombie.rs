use super::{
    apply_head_look, apply_humanoid_leg_swing_named, apply_zombie_arms_held_out_named, PartPose,
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
pub(in crate::entity_models) const MODEL_LAYER_ZOMBIE_VILLAGER_BABY: &str =
    "minecraft:zombie_villager_baby#main";

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
/// head ([`apply_head_look`] on `head`), run the humanoid leg swing
/// ([`apply_humanoid_leg_swing_named`]), then override the arms with the held-out `animateZombieArms`
/// pose ([`apply_zombie_arms_held_out_named`], `isAggressive`-driven and swung by `attack_anim`).
fn apply_zombie_family_anim(root: &mut ModelPart, instance: &EntityModelInstance) {
    let render_state = &instance.render_state;
    apply_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
    );
    apply_humanoid_leg_swing_named(
        root,
        render_state.walk_animation_pos,
        render_state.walk_animation_speed,
    );
    apply_zombie_arms_held_out_named(
        root,
        render_state.is_aggressive,
        render_state.attack_anim,
        render_state.age_in_ticks,
    );
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
/// builds its own robed tree; `setup_anim` runs the shared [`apply_zombie_family_anim`]. The per-family
/// root scale (husk) and the colored recolor / textured texture are supplied by the caller; the drowned
/// swim/outer layer and the profession overlays defer.
pub(in crate::entity_models) struct ZombieVariantModel {
    root: ModelPart,
}

impl ZombieVariantModel {
    pub(in crate::entity_models) fn new(family: ZombieVariantModelFamily, baby: bool) -> Self {
        let root = match family {
            ZombieVariantModelFamily::ZombieVillager => zombie_villager_tree(baby),
            _ => zombie_tree(baby),
        };
        Self { root }
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
    }
}
