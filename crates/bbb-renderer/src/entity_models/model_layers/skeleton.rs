use super::{
    apply_head_look, apply_humanoid_walk_named, bogged_clothing_root, stray_clothing_root,
    PartPose, PART_POSE_ZERO,
};
use crate::entity_models::catalog::SkeletonModelFamily;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_SKELETON: &str = "minecraft:skeleton#main";
pub(in crate::entity_models) const MODEL_LAYER_STRAY: &str = "minecraft:stray#main";
pub(in crate::entity_models) const MODEL_LAYER_PARCHED: &str = "minecraft:parched#main";
pub(in crate::entity_models) const MODEL_LAYER_WITHER_SKELETON: &str =
    "minecraft:wither_skeleton#main";
pub(in crate::entity_models) const MODEL_LAYER_BOGGED: &str = "minecraft:bogged#main";

pub(in crate::entity_models) const SKELETON_BONE: [f32; 4] = [0.82, 0.82, 0.72, 1.0];
pub(in crate::entity_models) const WITHER_SKELETON_DARK: [f32; 4] = [0.14, 0.14, 0.14, 1.0];
pub(in crate::entity_models) const PARCHED_BONE: [f32; 4] = [0.70, 0.62, 0.48, 1.0];
pub(in crate::entity_models) const BOGGED_BONE: [f32; 4] = [0.53, 0.61, 0.42, 1.0];
pub(in crate::entity_models) const BOGGED_RED_MUSHROOM_COLOR: [f32; 4] = [0.78, 0.15, 0.12, 1.0];
pub(in crate::entity_models) const BOGGED_BROWN_MUSHROOM_COLOR: [f32; 4] = [0.48, 0.31, 0.18, 1.0];

// Vanilla 26.1 SkeletonModel.createBodyLayer(). Each cube carries both render paths' data: the
// colored debug tint and the textured uv_size/texOffs/mirror. The left arm/leg share the colored
// geometry but carry the mirrored UV.
pub(in crate::entity_models) const SKELETON_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -8.0, -4.0],
    [8.0, 8.0, 8.0],
    SKELETON_BONE,
    [8.0, 8.0, 8.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const SKELETON_HAT: [ModelCube; 1] = [ModelCube::new(
    [-4.5, -8.5, -4.5],
    [9.0, 9.0, 9.0],
    SKELETON_BONE,
    [8.0, 8.0, 8.0],
    [32.0, 0.0],
    false,
)];

pub(in crate::entity_models) const SKELETON_BODY: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.0, -2.0],
    [8.0, 12.0, 4.0],
    SKELETON_BONE,
    [8.0, 12.0, 4.0],
    [16.0, 16.0],
    false,
)];

pub(in crate::entity_models) const SKELETON_RIGHT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -2.0, -1.0],
    [2.0, 12.0, 2.0],
    SKELETON_BONE,
    [2.0, 12.0, 2.0],
    [40.0, 16.0],
    false,
)];

pub(in crate::entity_models) const SKELETON_LEFT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -2.0, -1.0],
    [2.0, 12.0, 2.0],
    SKELETON_BONE,
    [2.0, 12.0, 2.0],
    [40.0, 16.0],
    true,
)];

pub(in crate::entity_models) const SKELETON_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 12.0, 2.0],
    SKELETON_BONE,
    [2.0, 12.0, 2.0],
    [0.0, 16.0],
    false,
)];

pub(in crate::entity_models) const SKELETON_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 12.0, 2.0],
    SKELETON_BONE,
    [2.0, 12.0, 2.0],
    [0.0, 16.0],
    true,
)];

/// Shared humanoid limb part poses (vanilla `HumanoidModel.createMesh`).
const HUMANOID_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const HUMANOID_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const SKELETON_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-2.0, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const SKELETON_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [2.0, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const BOGGED_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -8.0, -4.0],
    [8.0, 8.0, 8.0],
    BOGGED_BONE,
    [8.0, 8.0, 8.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BOGGED_HAT: [ModelCube; 1] = [ModelCube::new(
    [-4.5, -8.5, -4.5],
    [9.0, 9.0, 9.0],
    BOGGED_BONE,
    [8.0, 8.0, 8.0],
    [32.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BOGGED_BODY: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.0, -2.0],
    [8.0, 12.0, 4.0],
    BOGGED_BONE,
    [8.0, 12.0, 4.0],
    [16.0, 16.0],
    false,
)];

pub(in crate::entity_models) const BOGGED_RIGHT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -2.0, -1.0],
    [2.0, 12.0, 2.0],
    BOGGED_BONE,
    [2.0, 12.0, 2.0],
    [40.0, 16.0],
    false,
)];

pub(in crate::entity_models) const BOGGED_LEFT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -2.0, -1.0],
    [2.0, 12.0, 2.0],
    BOGGED_BONE,
    [2.0, 12.0, 2.0],
    [40.0, 16.0],
    true,
)];

pub(in crate::entity_models) const BOGGED_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 12.0, 2.0],
    BOGGED_BONE,
    [2.0, 12.0, 2.0],
    [0.0, 16.0],
    false,
)];

pub(in crate::entity_models) const BOGGED_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 12.0, 2.0],
    BOGGED_BONE,
    [2.0, 12.0, 2.0],
    [0.0, 16.0],
    true,
)];

pub(in crate::entity_models) const BOGGED_RED_MUSHROOM_PLANE: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -3.0, 0.0],
    [6.0, 4.0, 0.0],
    BOGGED_RED_MUSHROOM_COLOR,
    [6.0, 4.0, 0.0],
    [50.0, 16.0],
    false,
)];

pub(in crate::entity_models) const BOGGED_BROWN_MUSHROOM_PLANE: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -3.0, 0.0],
    [6.0, 4.0, 0.0],
    BOGGED_BROWN_MUSHROOM_COLOR,
    [6.0, 4.0, 0.0],
    [50.0, 22.0],
    false,
)];

pub(in crate::entity_models) const BOGGED_BROWN_TOP_MUSHROOM_PLANE: [ModelCube; 1] =
    [ModelCube::new(
        [-3.0, -4.0, 0.0],
        [6.0, 4.0, 0.0],
        BOGGED_BROWN_MUSHROOM_COLOR,
        [6.0, 4.0, 0.0],
        [50.0, 28.0],
        false,
    )];

/// The six bogged-mushroom plane child poses (vanilla `BoggedModel.createBodyLayer`), in declaration
/// order: two red, two brown, two brown-top.
const BOGGED_MUSHROOM_POSES: [PartPose; 6] = [
    PartPose {
        offset: [3.0, -8.0, 3.0],
        rotation: [0.0, std::f32::consts::FRAC_PI_4, 0.0],
    },
    PartPose {
        offset: [3.0, -8.0, 3.0],
        rotation: [0.0, std::f32::consts::FRAC_PI_4 * 3.0, 0.0],
    },
    PartPose {
        offset: [-3.0, -8.0, -3.0],
        rotation: [0.0, std::f32::consts::FRAC_PI_4, 0.0],
    },
    PartPose {
        offset: [-3.0, -8.0, -3.0],
        rotation: [0.0, std::f32::consts::FRAC_PI_4 * 3.0, 0.0],
    },
    PartPose {
        offset: [-2.0, -1.0, 4.0],
        rotation: [
            -std::f32::consts::FRAC_PI_2,
            0.0,
            std::f32::consts::FRAC_PI_4,
        ],
    },
    PartPose {
        offset: [-2.0, -1.0, 4.0],
        rotation: [
            -std::f32::consts::FRAC_PI_2,
            0.0,
            std::f32::consts::FRAC_PI_4 * 3.0,
        ],
    },
];

pub(in crate::entity_models) const PARCHED_BODY: [ModelCube; 3] = [
    ModelCube::new(
        [-4.0, 0.0, -2.0],
        [8.0, 12.0, 4.0],
        PARCHED_BONE,
        [8.0, 12.0, 4.0],
        [16.0, 16.0],
        false,
    ),
    ModelCube::new(
        [-4.0, 10.0, -2.0],
        [8.0, 1.0, 4.0],
        PARCHED_BONE,
        [8.0, 1.0, 4.0],
        [28.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-4.025, -0.025, -2.025],
        [8.05, 12.05, 4.05],
        PARCHED_BONE,
        [8.0, 12.0, 4.0],
        [16.0, 48.0],
        false,
    ),
];

pub(in crate::entity_models) const PARCHED_HEAD: [ModelCube; 2] = [
    ModelCube::new(
        [-4.0, -8.0, -4.0],
        [8.0, 8.0, 8.0],
        PARCHED_BONE,
        [8.0, 8.0, 8.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-4.2, -8.2, -4.2],
        [8.4, 8.4, 8.4],
        PARCHED_BONE,
        [8.0, 8.0, 8.0],
        [0.0, 32.0],
        false,
    ),
];

pub(in crate::entity_models) const PARCHED_RIGHT_ARM: [ModelCube; 2] = [
    ModelCube::new(
        [-1.0, -2.0, -1.0],
        [2.0, 12.0, 2.0],
        PARCHED_BONE,
        [2.0, 12.0, 2.0],
        [40.0, 16.0],
        false,
    ),
    ModelCube::new(
        [-1.55, -2.025, -1.5],
        [3.0, 12.0, 3.0],
        PARCHED_BONE,
        [3.0, 12.0, 3.0],
        [42.0, 33.0],
        false,
    ),
];

pub(in crate::entity_models) const PARCHED_LEFT_ARM: [ModelCube; 2] = [
    ModelCube::new(
        [-1.0, -2.0, -1.0],
        [2.0, 12.0, 2.0],
        PARCHED_BONE,
        [2.0, 12.0, 2.0],
        [56.0, 16.0],
        false,
    ),
    ModelCube::new(
        [-1.45, -2.025, -1.5],
        [3.0, 12.0, 3.0],
        PARCHED_BONE,
        [3.0, 12.0, 3.0],
        [40.0, 48.0],
        false,
    ),
];

pub(in crate::entity_models) const PARCHED_RIGHT_LEG: [ModelCube; 2] = [
    ModelCube::new(
        [-1.0, 0.0, -1.0],
        [2.0, 12.0, 2.0],
        PARCHED_BONE,
        [2.0, 12.0, 2.0],
        [0.0, 16.0],
        false,
    ),
    ModelCube::new(
        [-1.5, 0.0, -1.5],
        [3.0, 12.0, 3.0],
        PARCHED_BONE,
        [3.0, 12.0, 3.0],
        [0.0, 49.0],
        false,
    ),
];

pub(in crate::entity_models) const PARCHED_LEFT_LEG: [ModelCube; 2] = [
    ModelCube::new(
        [-1.0, 0.0, -1.0],
        [2.0, 12.0, 2.0],
        PARCHED_BONE,
        [2.0, 12.0, 2.0],
        [0.0, 16.0],
        false,
    ),
    ModelCube::new(
        [-1.5, 0.0, -1.5],
        [3.0, 12.0, 3.0],
        PARCHED_BONE,
        [3.0, 12.0, 3.0],
        [4.0, 49.0],
        false,
    ),
];

/// Parched part poses (vanilla `SkeletonModel.createSingleModelDualBodyLayer`): the arms sit slightly
/// wider (`±5.5`) than the base skeleton.
const PARCHED_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-5.5, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const PARCHED_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [5.5, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds a leaf part at `pose` carrying `cubes`.
fn part(pose: PartPose, cubes: &[ModelCube]) -> ModelPart {
    ModelPart::leaf(pose, cubes.to_vec())
}

/// Builds the six adult-humanoid limbs (right/left arm, right/left leg) under the vanilla
/// `HumanoidModel` child names, shared by the skeleton and bogged layouts (same poses + bone cubes).
fn humanoid_limbs(
    right_arm: &[ModelCube],
    left_arm: &[ModelCube],
    right_leg: &[ModelCube],
    left_leg: &[ModelCube],
) -> Vec<(&'static str, ModelPart)> {
    vec![
        ("right_arm", part(HUMANOID_RIGHT_ARM_POSE, right_arm)),
        ("left_arm", part(HUMANOID_LEFT_ARM_POSE, left_arm)),
        ("right_leg", part(SKELETON_RIGHT_LEG_POSE, right_leg)),
        ("left_leg", part(SKELETON_LEFT_LEG_POSE, left_leg)),
    ]
}

/// Builds the bogged mushroom container (an empty parent with the six mushroom plane children), a
/// `head` child that the renderer hides on a sheared bogged.
fn bogged_mushrooms() -> ModelPart {
    let children = vec![
        (
            "red_a",
            part(BOGGED_MUSHROOM_POSES[0], &BOGGED_RED_MUSHROOM_PLANE),
        ),
        (
            "red_b",
            part(BOGGED_MUSHROOM_POSES[1], &BOGGED_RED_MUSHROOM_PLANE),
        ),
        (
            "brown_a",
            part(BOGGED_MUSHROOM_POSES[2], &BOGGED_BROWN_MUSHROOM_PLANE),
        ),
        (
            "brown_b",
            part(BOGGED_MUSHROOM_POSES[3], &BOGGED_BROWN_MUSHROOM_PLANE),
        ),
        (
            "brown_top_a",
            part(BOGGED_MUSHROOM_POSES[4], &BOGGED_BROWN_TOP_MUSHROOM_PLANE),
        ),
        (
            "brown_top_b",
            part(BOGGED_MUSHROOM_POSES[5], &BOGGED_BROWN_TOP_MUSHROOM_PLANE),
        ),
    ];
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Builds the unified skeleton-family root for `family`, with the vanilla `HumanoidModel` child names
/// (`head` -> `hat` [+ bogged `mushrooms`], `body`, `right_arm`, `left_arm`, `right_leg`, `left_leg`).
/// Skeleton/stray/wither share the plain bone tree; bogged adds the mushroom layer (hidden when
/// sheared); parched lists the body first (vanilla dual body layer) with its inflated overlay cubes.
fn skeleton_tree(family: Option<SkeletonModelFamily>) -> ModelPart {
    match family {
        Some(SkeletonModelFamily::Parched) => {
            // Vanilla lists the body first, then the head (with an empty hat child).
            let head = ModelPart::new(
                PART_POSE_ZERO,
                PARCHED_HEAD.to_vec(),
                vec![("hat", part(PART_POSE_ZERO, &[]))],
            );
            let mut children = vec![
                ("body", part(PART_POSE_ZERO, &PARCHED_BODY)),
                ("head", head),
                (
                    "right_arm",
                    part(PARCHED_RIGHT_ARM_POSE, &PARCHED_RIGHT_ARM),
                ),
                ("left_arm", part(PARCHED_LEFT_ARM_POSE, &PARCHED_LEFT_ARM)),
            ];
            children.push((
                "right_leg",
                part(SKELETON_RIGHT_LEG_POSE, &PARCHED_RIGHT_LEG),
            ));
            children.push(("left_leg", part(SKELETON_LEFT_LEG_POSE, &PARCHED_LEFT_LEG)));
            ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
        }
        Some(SkeletonModelFamily::Bogged { sheared }) => {
            let mut head_children = vec![("hat", part(PART_POSE_ZERO, &BOGGED_HAT))];
            if !sheared {
                head_children.push(("mushrooms", bogged_mushrooms()));
            }
            let head = ModelPart::new(PART_POSE_ZERO, BOGGED_HEAD.to_vec(), head_children);
            let mut children = vec![("head", head), ("body", part(PART_POSE_ZERO, &BOGGED_BODY))];
            children.extend(humanoid_limbs(
                &BOGGED_RIGHT_ARM,
                &BOGGED_LEFT_ARM,
                &BOGGED_RIGHT_LEG,
                &BOGGED_LEFT_LEG,
            ));
            ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
        }
        // Skeleton / stray / wither-skeleton share the plain bone tree.
        _ => {
            let head = ModelPart::new(
                PART_POSE_ZERO,
                SKELETON_HEAD.to_vec(),
                vec![("hat", part(PART_POSE_ZERO, &SKELETON_HAT))],
            );
            let mut children = vec![
                ("head", head),
                ("body", part(PART_POSE_ZERO, &SKELETON_BODY)),
            ];
            children.extend(humanoid_limbs(
                &SKELETON_RIGHT_ARM,
                &SKELETON_LEFT_ARM,
                &SKELETON_RIGHT_LEG,
                &SKELETON_LEFT_LEG,
            ));
            ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
        }
    }
}

/// Mutable skeleton model, mirroring vanilla `SkeletonModel` (the base `HumanoidModel`) and its
/// stray / parched / bogged / wither-skeleton variants. The unified tree is built for the selected
/// family ([`skeleton_tree`]) with the vanilla child names. `setup_anim` runs the shared
/// `HumanoidModel.setupAnim` (head look + arm/leg walk swing). The bow-aiming arm pose is deferred;
/// the wither dark tint / root transform and the stray / bogged clothing overlay
/// ([`SkeletonClothingModel`]) are applied at the call site.
pub(in crate::entity_models) struct SkeletonModel {
    root: ModelPart,
}

impl SkeletonModel {
    pub(in crate::entity_models) fn new(family: Option<SkeletonModelFamily>) -> Self {
        Self {
            root: skeleton_tree(family),
        }
    }
}

impl EntityModel for SkeletonModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;
        apply_head_look(
            self.root.child_mut("head"),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        apply_humanoid_walk_named(
            &mut self.root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
            render_state.age_in_ticks,
        );
    }
}

/// Mutable textured-only skeleton clothing overlay (the stray frost layer / bogged mushroom layer): an
/// inflated `HumanoidModel`-shaped overlay built as a named-children tree (`stray_clothing_root` /
/// `bogged_clothing_root`) and posed by the SAME shared `HumanoidModel.setupAnim` as the base body, so
/// the overlay tracks the limbs. It has no colored variant.
pub(in crate::entity_models) struct SkeletonClothingModel {
    root: ModelPart,
}

impl SkeletonClothingModel {
    pub(in crate::entity_models) fn new(family: Option<SkeletonModelFamily>) -> Self {
        let root = match family {
            Some(SkeletonModelFamily::Bogged { .. }) => bogged_clothing_root(),
            _ => stray_clothing_root(),
        };
        Self { root }
    }
}

impl EntityModel for SkeletonClothingModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;
        apply_head_look(
            self.root.child_mut("head"),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        apply_humanoid_walk_named(
            &mut self.root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
            render_state.age_in_ticks,
        );
    }
}
