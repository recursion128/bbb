use super::{
    apply_head_look, apply_iron_golem_walk, snow_golem_arm_pose, snow_golem_upper_body_pose,
    snow_golem_upper_body_yrot, PartPose, IRON_GOLEM_STONE, SNOW_GOLEM_WHITE,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 IronGolemModel.createBodyLayer(). Each cube carries both render paths' data: the
// colored debug tint and the textured `uv_size` / `texOffs` / `mirror`.
pub(in crate::entity_models) const IRON_GOLEM_HEAD: [ModelCube; 2] = [
    ModelCube::new(
        [-4.0, -12.0, -5.5],
        [8.0, 10.0, 8.0],
        IRON_GOLEM_STONE,
        [8.0, 10.0, 8.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-1.0, -5.0, -7.5],
        [2.0, 4.0, 2.0],
        IRON_GOLEM_STONE,
        [2.0, 4.0, 2.0],
        [24.0, 0.0],
        false,
    ),
];

pub(in crate::entity_models) const IRON_GOLEM_BODY: [ModelCube; 2] = [
    ModelCube::new(
        [-9.0, -2.0, -6.0],
        [18.0, 12.0, 11.0],
        IRON_GOLEM_STONE,
        [18.0, 12.0, 11.0],
        [0.0, 40.0],
        false,
    ),
    ModelCube::new(
        [-5.0, 9.5, -3.5],
        [10.0, 6.0, 7.0],
        IRON_GOLEM_STONE,
        [9.0, 5.0, 6.0],
        [0.0, 70.0],
        false,
    ),
];

pub(in crate::entity_models) const IRON_GOLEM_RIGHT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-13.0, -2.5, -3.0],
    [4.0, 30.0, 6.0],
    IRON_GOLEM_STONE,
    [4.0, 30.0, 6.0],
    [60.0, 21.0],
    false,
)];

pub(in crate::entity_models) const IRON_GOLEM_LEFT_ARM: [ModelCube; 1] = [ModelCube::new(
    [9.0, -2.5, -3.0],
    [4.0, 30.0, 6.0],
    IRON_GOLEM_STONE,
    [4.0, 30.0, 6.0],
    [60.0, 58.0],
    false,
)];

pub(in crate::entity_models) const IRON_GOLEM_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-3.5, -3.0, -3.0],
    [6.0, 16.0, 5.0],
    IRON_GOLEM_STONE,
    [6.0, 16.0, 5.0],
    [37.0, 0.0],
    false,
)];

pub(in crate::entity_models) const IRON_GOLEM_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-3.5, -3.0, -3.0],
    [6.0, 16.0, 5.0],
    IRON_GOLEM_STONE,
    [6.0, 16.0, 5.0],
    [60.0, 0.0],
    true,
)];

/// Iron golem part poses (vanilla `IronGolemModel.createBodyLayer`).
pub(in crate::entity_models) const IRON_GOLEM_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, -7.0, -2.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const IRON_GOLEM_BODY_POSE: PartPose = PartPose {
    offset: [0.0, -7.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const IRON_GOLEM_ARM_POSE: PartPose = PartPose {
    offset: [0.0, -7.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const IRON_GOLEM_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-4.0, 11.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const IRON_GOLEM_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [5.0, 11.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const MODEL_LAYER_IRON_GOLEM: &str = "minecraft:iron_golem#main";

// Vanilla 26.1 SnowGolemModel.createBodyLayer().
pub(in crate::entity_models) const SNOW_GOLEM_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-3.5, -7.5, -3.5],
    [7.0, 7.0, 7.0],
    SNOW_GOLEM_WHITE,
    [8.0, 8.0, 8.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const SNOW_GOLEM_ARM: [ModelCube; 1] = [ModelCube::new(
    [-0.5, 0.5, -0.5],
    [11.0, 1.0, 1.0],
    SNOW_GOLEM_WHITE,
    [12.0, 2.0, 2.0],
    [32.0, 0.0],
    false,
)];

pub(in crate::entity_models) const SNOW_GOLEM_UPPER_BODY: [ModelCube; 1] = [ModelCube::new(
    [-4.5, -9.5, -4.5],
    [9.0, 9.0, 9.0],
    SNOW_GOLEM_WHITE,
    [10.0, 10.0, 10.0],
    [0.0, 16.0],
    false,
)];

pub(in crate::entity_models) const SNOW_GOLEM_LOWER_BODY: [ModelCube; 1] = [ModelCube::new(
    [-5.5, -11.5, -5.5],
    [11.0, 11.0, 11.0],
    SNOW_GOLEM_WHITE,
    [12.0, 12.0, 12.0],
    [0.0, 36.0],
    false,
)];

/// Snow golem part poses (vanilla `SnowGolemModel.createBodyLayer`): head, left arm, right arm,
/// upper body (middle snow ball), lower body.
pub(in crate::entity_models) const SNOW_GOLEM_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 4.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const SNOW_GOLEM_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [5.0, 6.0, 1.0],
    rotation: [0.0, 0.0, 1.0],
};
pub(in crate::entity_models) const SNOW_GOLEM_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-5.0, 6.0, -1.0],
    rotation: [0.0, std::f32::consts::PI, -1.0],
};
pub(in crate::entity_models) const SNOW_GOLEM_UPPER_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 13.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const SNOW_GOLEM_LOWER_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 24.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const MODEL_LAYER_SNOW_GOLEM: &str = "minecraft:snow_golem#main";

/// Builds a leaf part at `pose` carrying `cubes`.
fn leaf(pose: PartPose, cubes: &[ModelCube]) -> ModelPart {
    ModelPart::leaf(pose, cubes.to_vec())
}

/// Builds the unified iron golem tree under the vanilla `IronGolemModel` child names (`head`, `body`,
/// `right_arm`, `left_arm`, `right_leg`, `left_leg`).
fn iron_golem_tree() -> ModelPart {
    ModelPart::new(
        super::PART_POSE_ZERO,
        Vec::new(),
        vec![
            ("head", leaf(IRON_GOLEM_HEAD_POSE, &IRON_GOLEM_HEAD)),
            ("body", leaf(IRON_GOLEM_BODY_POSE, &IRON_GOLEM_BODY)),
            (
                "right_arm",
                leaf(IRON_GOLEM_ARM_POSE, &IRON_GOLEM_RIGHT_ARM),
            ),
            ("left_arm", leaf(IRON_GOLEM_ARM_POSE, &IRON_GOLEM_LEFT_ARM)),
            (
                "right_leg",
                leaf(IRON_GOLEM_RIGHT_LEG_POSE, &IRON_GOLEM_RIGHT_LEG),
            ),
            (
                "left_leg",
                leaf(IRON_GOLEM_LEFT_LEG_POSE, &IRON_GOLEM_LEFT_LEG),
            ),
        ],
    )
}

/// Builds the unified snow golem tree under the vanilla `SnowGolemModel` child names. Vanilla lists
/// the parts head / left arm / right arm / upper body / lower body, preserved here so the colored
/// render order stays byte-identical, while the head look, body twist, and arm orbit resolve their
/// parts by name.
fn snow_golem_tree() -> ModelPart {
    ModelPart::new(
        super::PART_POSE_ZERO,
        Vec::new(),
        vec![
            ("head", leaf(SNOW_GOLEM_HEAD_POSE, &SNOW_GOLEM_HEAD)),
            ("left_arm", leaf(SNOW_GOLEM_LEFT_ARM_POSE, &SNOW_GOLEM_ARM)),
            (
                "right_arm",
                leaf(SNOW_GOLEM_RIGHT_ARM_POSE, &SNOW_GOLEM_ARM),
            ),
            (
                "upper_body",
                leaf(SNOW_GOLEM_UPPER_BODY_POSE, &SNOW_GOLEM_UPPER_BODY),
            ),
            (
                "lower_body",
                leaf(SNOW_GOLEM_LOWER_BODY_POSE, &SNOW_GOLEM_LOWER_BODY),
            ),
        ],
    )
}

/// Mutable iron golem model, mirroring vanilla `IronGolemModel`. The unified tree is built with the
/// vanilla child names: `head`, `body`, `right_arm`/`left_arm`, `right_leg`/`left_leg`. `setup_anim`
/// follows the head look ([`apply_head_look`] on `head`) then swings the arms and legs
/// ([`apply_iron_golem_walk`]). The attack swing and offer-flower arm pose are deferred event
/// animations.
pub(in crate::entity_models) struct IronGolemModel {
    root: ModelPart,
}

impl IronGolemModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: iron_golem_tree(),
        }
    }
}

impl EntityModel for IronGolemModel {
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
        apply_iron_golem_walk(
            &mut self.root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
        );
    }
}

/// Mutable snow golem model, mirroring vanilla `SnowGolemModel`. The unified tree is built with the
/// vanilla child names: `head`, `left_arm`/`right_arm` (stick arms), `upper_body` (the middle snow
/// ball), `lower_body`. `setup_anim` looks the head ([`apply_head_look`] on `head`), twists the upper
/// body by a quarter of the head yaw ([`snow_golem_upper_body_pose`]), and orbits the two arms around
/// that twist ([`snow_golem_arm_pose`]). The arm orbit overwrites the body-layer `x`/`z` even at rest,
/// so the tree is always re-posed.
pub(in crate::entity_models) struct SnowGolemModel {
    root: ModelPart,
}

impl SnowGolemModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: snow_golem_tree(),
        }
    }
}

impl EntityModel for SnowGolemModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;
        let upper_body_yrot = snow_golem_upper_body_yrot(render_state.head_yaw);
        apply_head_look(
            self.root.child_mut("head"),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        let upper_body = self.root.child_mut("upper_body");
        upper_body.pose = snow_golem_upper_body_pose(upper_body.pose, upper_body_yrot);
        let left_arm = self.root.child_mut("left_arm");
        left_arm.pose = snow_golem_arm_pose(left_arm.pose, upper_body_yrot, false);
        let right_arm = self.root.child_mut("right_arm");
        right_arm.pose = snow_golem_arm_pose(right_arm.pose, upper_body_yrot, true);
    }
}
