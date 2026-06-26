use super::{apply_head_look, PartPose, COPPER_GOLEM_COPPER};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 `CopperGolemModel.createBodyLayer()` (atlas 64x64). The mesh root is transformed by
// `(0, 24, 0)`, so that translation is preserved as the root `PartPose`. `CubeDeformation` grows or
// insets the rendered cube geometry while the textured `uv_size` keeps the original addBox size.
pub(in crate::entity_models) const COPPER_GOLEM_BODY: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -6.0, -3.0],
    [8.0, 6.0, 6.0],
    COPPER_GOLEM_COPPER,
    [8.0, 6.0, 6.0],
    [0.0, 15.0],
    false,
)];

pub(in crate::entity_models) const COPPER_GOLEM_HEAD: [ModelCube; 4] = [
    ModelCube::new(
        [-4.015, -5.015, -5.015],
        [8.03, 5.03, 10.03],
        COPPER_GOLEM_COPPER,
        [8.0, 5.0, 10.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-1.0, -2.0, -6.0],
        [2.0, 3.0, 2.0],
        COPPER_GOLEM_COPPER,
        [2.0, 3.0, 2.0],
        [56.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-0.985, -8.985, -0.985],
        [1.97, 3.97, 1.97],
        COPPER_GOLEM_COPPER,
        [2.0, 4.0, 2.0],
        [37.0, 8.0],
        false,
    ),
    ModelCube::new(
        [-1.985, -12.985, -1.985],
        [3.97, 3.97, 3.97],
        COPPER_GOLEM_COPPER,
        [4.0, 4.0, 4.0],
        [37.0, 0.0],
        false,
    ),
];

pub(in crate::entity_models) const COPPER_GOLEM_RIGHT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -1.0, -2.0],
    [3.0, 10.0, 4.0],
    COPPER_GOLEM_COPPER,
    [3.0, 10.0, 4.0],
    [36.0, 16.0],
    false,
)];

pub(in crate::entity_models) const COPPER_GOLEM_LEFT_ARM: [ModelCube; 1] = [ModelCube::new(
    [0.0, -1.0, -2.0],
    [3.0, 10.0, 4.0],
    COPPER_GOLEM_COPPER,
    [3.0, 10.0, 4.0],
    [50.0, 16.0],
    false,
)];

pub(in crate::entity_models) const COPPER_GOLEM_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.0, -2.0],
    [4.0, 5.0, 4.0],
    COPPER_GOLEM_COPPER,
    [4.0, 5.0, 4.0],
    [0.0, 27.0],
    false,
)];

pub(in crate::entity_models) const COPPER_GOLEM_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, -2.0],
    [4.0, 5.0, 4.0],
    COPPER_GOLEM_COPPER,
    [4.0, 5.0, 4.0],
    [16.0, 27.0],
    false,
)];

pub(in crate::entity_models) const COPPER_GOLEM_ROOT_POSE: PartPose = PartPose {
    offset: [0.0, 24.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const COPPER_GOLEM_BODY_POSE: PartPose = PartPose {
    offset: [0.0, -5.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const COPPER_GOLEM_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, -6.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const COPPER_GOLEM_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-4.0, -6.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const COPPER_GOLEM_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [4.0, -6.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const COPPER_GOLEM_LEG_POSE: PartPose = PartPose {
    offset: [0.0, -5.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const MODEL_LAYER_COPPER_GOLEM: &str = "minecraft:copper_golem#main";

fn copper_golem_tree() -> ModelPart {
    let body = ModelPart::new(
        COPPER_GOLEM_BODY_POSE,
        COPPER_GOLEM_BODY.to_vec(),
        vec![
            (
                "head",
                ModelPart::leaf(COPPER_GOLEM_HEAD_POSE, COPPER_GOLEM_HEAD.to_vec()),
            ),
            (
                "right_arm",
                ModelPart::leaf(COPPER_GOLEM_RIGHT_ARM_POSE, COPPER_GOLEM_RIGHT_ARM.to_vec()),
            ),
            (
                "left_arm",
                ModelPart::leaf(COPPER_GOLEM_LEFT_ARM_POSE, COPPER_GOLEM_LEFT_ARM.to_vec()),
            ),
        ],
    );
    ModelPart::new(
        COPPER_GOLEM_ROOT_POSE,
        Vec::new(),
        vec![
            ("body", body),
            (
                "right_leg",
                ModelPart::leaf(COPPER_GOLEM_LEG_POSE, COPPER_GOLEM_RIGHT_LEG.to_vec()),
            ),
            (
                "left_leg",
                ModelPart::leaf(COPPER_GOLEM_LEG_POSE, COPPER_GOLEM_LEFT_LEG.to_vec()),
            ),
        ],
    )
}

fn pose_held_item_arms_if_still(root: &mut ModelPart) {
    // Vanilla `CopperGolemModel.poseHeldItemArmsIfStill`: clamp the arms into the resting held-item pose
    // when either rendered hand is non-empty. Walk/interaction keyframes still stay deferred.
    let body = root.child_mut("body");
    let right_arm = body.child_mut("right_arm");
    right_arm.pose.rotation[0] = right_arm.pose.rotation[0].min(-0.87266463);
    right_arm.pose.rotation[1] = right_arm.pose.rotation[1].min(-0.1134464);
    right_arm.pose.rotation[2] = right_arm.pose.rotation[2].min(-0.064577185);
    let left_arm = body.child_mut("left_arm");
    left_arm.pose.rotation[0] = left_arm.pose.rotation[0].min(-0.87266463);
    left_arm.pose.rotation[1] = left_arm.pose.rotation[1].max(0.1134464);
    left_arm.pose.rotation[2] = left_arm.pose.rotation[2].max(0.064577185);
}

/// Mutable copper golem model, mirroring vanilla `CopperGolemModel.createBodyLayer`. The base
/// renderer uses this same tree for both the cutout body and the emissive eyes texture. The vanilla
/// keyframe walk/idle/interaction animations, custom head, and antenna block transform are deferred; the
/// head look and the static held-item arm pose are projected now.
pub(in crate::entity_models) struct CopperGolemModel {
    root: ModelPart,
}

impl CopperGolemModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: copper_golem_tree(),
        }
    }
}

impl EntityModel for CopperGolemModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;
        let head = self.root.child_mut("body").child_mut("head");
        apply_head_look(head, render_state.head_yaw, render_state.head_pitch);
        if render_state.copper_golem_holding_item {
            pose_held_item_arms_if_still(&mut self.root);
        }
    }
}
