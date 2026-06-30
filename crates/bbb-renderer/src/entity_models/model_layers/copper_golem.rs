use super::{
    apply_head_look, degree_vec, keyframe, pos_vec, AnimationChannel, AnimationDefinition,
    AnimationTarget, BoneAnimation, Keyframe, KeyframeInterpolation, PartPose, COPPER_GOLEM_COPPER,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::{
    keyframe_animated_pose, keyframe_walk_sample, sample_bone_offsets,
};
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

const LINEAR: KeyframeInterpolation = KeyframeInterpolation::Linear;
const CATMULLROM: KeyframeInterpolation = KeyframeInterpolation::CatmullRom;
const COPPER_GOLEM_WALK_SPEED_FACTOR: f32 = 2.0;
const COPPER_GOLEM_WALK_SCALE_FACTOR: f32 = 2.5;

const fn rot(keyframes: &'static [Keyframe]) -> AnimationChannel {
    AnimationChannel {
        target: AnimationTarget::Rotation,
        keyframes,
    }
}

const fn pos(keyframes: &'static [Keyframe]) -> AnimationChannel {
    AnimationChannel {
        target: AnimationTarget::Position,
        keyframes,
    }
}

// Vanilla 26.1 `CopperGolemAnimation.COPPER_GOLEM_WALK` (length 0.8333s, looping).
// `CopperGolemModel.setupAnim` samples it with `applyWalk(pos, speed, 2.0F, 2.5F)` while both
// rendered hands are empty. Every keyframe is CatmullRom.
const COPPER_GOLEM_WALK_BODY_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(10.0, 15.0, 0.0), CATMULLROM),
    keyframe(0.2083, degree_vec(10.0, -1.87, -10.0), CATMULLROM),
    keyframe(0.4167, degree_vec(10.0, -15.0, 0.0), CATMULLROM),
    keyframe(0.625, degree_vec(10.0, -0.82, 10.0), CATMULLROM),
    keyframe(0.8333, degree_vec(10.0, 15.0, 0.0), CATMULLROM),
];
const COPPER_GOLEM_WALK_HEAD_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(-10.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.2083, degree_vec(-10.0, 1.87, 10.0), CATMULLROM),
    keyframe(0.4167, degree_vec(-10.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.625, degree_vec(-10.0, 0.82, -10.0), CATMULLROM),
    keyframe(0.8333, degree_vec(-10.0, 0.0, 0.0), CATMULLROM),
];
const COPPER_GOLEM_WALK_RIGHT_ARM_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(70.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.4167, degree_vec(-80.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.8333, degree_vec(70.0, 0.0, 0.0), CATMULLROM),
];
const COPPER_GOLEM_WALK_LEFT_ARM_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(-80.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.4167, degree_vec(70.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.8333, degree_vec(-80.0, 0.0, 0.0), CATMULLROM),
];
const COPPER_GOLEM_WALK_RIGHT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(-60.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.4167, degree_vec(60.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.8333, degree_vec(-60.0, 0.0, 0.0), CATMULLROM),
];
const COPPER_GOLEM_WALK_LEFT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(60.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.4167, degree_vec(-60.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.8333, degree_vec(60.0, 0.0, 0.0), CATMULLROM),
];

const COPPER_GOLEM_WALK_BODY_CHANNELS: [AnimationChannel; 1] = [rot(&COPPER_GOLEM_WALK_BODY_ROT)];
const COPPER_GOLEM_WALK_HEAD_CHANNELS: [AnimationChannel; 1] = [rot(&COPPER_GOLEM_WALK_HEAD_ROT)];
const COPPER_GOLEM_WALK_RIGHT_ARM_CHANNELS: [AnimationChannel; 1] =
    [rot(&COPPER_GOLEM_WALK_RIGHT_ARM_ROT)];
const COPPER_GOLEM_WALK_LEFT_ARM_CHANNELS: [AnimationChannel; 1] =
    [rot(&COPPER_GOLEM_WALK_LEFT_ARM_ROT)];
const COPPER_GOLEM_WALK_RIGHT_LEG_CHANNELS: [AnimationChannel; 1] =
    [rot(&COPPER_GOLEM_WALK_RIGHT_LEG_ROT)];
const COPPER_GOLEM_WALK_LEFT_LEG_CHANNELS: [AnimationChannel; 1] =
    [rot(&COPPER_GOLEM_WALK_LEFT_LEG_ROT)];

const COPPER_GOLEM_WALK_BONES: [BoneAnimation; 6] = [
    BoneAnimation {
        bone: "body",
        channels: &COPPER_GOLEM_WALK_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &COPPER_GOLEM_WALK_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_arm",
        channels: &COPPER_GOLEM_WALK_RIGHT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_arm",
        channels: &COPPER_GOLEM_WALK_LEFT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "right_leg",
        channels: &COPPER_GOLEM_WALK_RIGHT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_leg",
        channels: &COPPER_GOLEM_WALK_LEFT_LEG_CHANNELS,
    },
];

pub(in crate::entity_models) const COPPER_GOLEM_WALK: AnimationDefinition = AnimationDefinition {
    length_seconds: 0.8333,
    looping: true,
    bones: &COPPER_GOLEM_WALK_BONES,
};

// Vanilla 26.1 `CopperGolemAnimation.COPPER_GOLEM_WALK_ITEM` (length 0.8333s, looping).
// The body sway and leg stride are smaller; the arms hold the carried-item pose, with a small
// left-arm position offset, before `poseHeldItemArmsIfStill` clamps the final rotations.
const COPPER_GOLEM_WALK_ITEM_BODY_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(10.0, 7.5, 0.0), CATMULLROM),
    keyframe(0.2083, degree_vec(10.0, -1.87, -5.0), CATMULLROM),
    keyframe(0.4167, degree_vec(10.0, -7.5, 0.0), CATMULLROM),
    keyframe(0.625, degree_vec(10.0, -0.82, 5.0), CATMULLROM),
    keyframe(0.8333, degree_vec(10.0, 7.5, 0.0), CATMULLROM),
];
const COPPER_GOLEM_WALK_ITEM_RIGHT_ARM_ROT: [Keyframe; 1] = [keyframe(
    0.0,
    degree_vec(-59.78638, -6.49053, -3.76613),
    LINEAR,
)];
const COPPER_GOLEM_WALK_ITEM_LEFT_ARM_ROT: [Keyframe; 1] = [keyframe(
    0.0,
    degree_vec(-59.78638, 6.49053, 3.76613),
    LINEAR,
)];
const COPPER_GOLEM_WALK_ITEM_LEFT_ARM_POS: [Keyframe; 1] =
    [keyframe(0.0, pos_vec(-0.21129, -0.0212, -0.07004), LINEAR)];
const COPPER_GOLEM_WALK_ITEM_RIGHT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(-30.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.4167, degree_vec(30.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.8333, degree_vec(-30.0, 0.0, 0.0), CATMULLROM),
];
const COPPER_GOLEM_WALK_ITEM_LEFT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(30.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.4167, degree_vec(-30.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.8333, degree_vec(30.0, 0.0, 0.0), CATMULLROM),
];

const COPPER_GOLEM_WALK_ITEM_BODY_CHANNELS: [AnimationChannel; 1] =
    [rot(&COPPER_GOLEM_WALK_ITEM_BODY_ROT)];
const COPPER_GOLEM_WALK_ITEM_HEAD_CHANNELS: [AnimationChannel; 1] =
    [rot(&COPPER_GOLEM_WALK_HEAD_ROT)];
const COPPER_GOLEM_WALK_ITEM_RIGHT_ARM_CHANNELS: [AnimationChannel; 1] =
    [rot(&COPPER_GOLEM_WALK_ITEM_RIGHT_ARM_ROT)];
const COPPER_GOLEM_WALK_ITEM_LEFT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&COPPER_GOLEM_WALK_ITEM_LEFT_ARM_ROT),
    pos(&COPPER_GOLEM_WALK_ITEM_LEFT_ARM_POS),
];
const COPPER_GOLEM_WALK_ITEM_RIGHT_LEG_CHANNELS: [AnimationChannel; 1] =
    [rot(&COPPER_GOLEM_WALK_ITEM_RIGHT_LEG_ROT)];
const COPPER_GOLEM_WALK_ITEM_LEFT_LEG_CHANNELS: [AnimationChannel; 1] =
    [rot(&COPPER_GOLEM_WALK_ITEM_LEFT_LEG_ROT)];

const COPPER_GOLEM_WALK_ITEM_BONES: [BoneAnimation; 6] = [
    BoneAnimation {
        bone: "body",
        channels: &COPPER_GOLEM_WALK_ITEM_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &COPPER_GOLEM_WALK_ITEM_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_arm",
        channels: &COPPER_GOLEM_WALK_ITEM_RIGHT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_arm",
        channels: &COPPER_GOLEM_WALK_ITEM_LEFT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "right_leg",
        channels: &COPPER_GOLEM_WALK_ITEM_RIGHT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_leg",
        channels: &COPPER_GOLEM_WALK_ITEM_LEFT_LEG_CHANNELS,
    },
];

pub(in crate::entity_models) const COPPER_GOLEM_WALK_ITEM: AnimationDefinition =
    AnimationDefinition {
        length_seconds: 0.8333,
        looping: true,
        bones: &COPPER_GOLEM_WALK_ITEM_BONES,
    };

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
    // when either rendered hand is non-empty, after the walk-with-item animation has been sampled.
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

fn apply_copper_golem_keyframe(
    definition: &AnimationDefinition,
    part: &mut ModelPart,
    bone: &str,
    seconds: f32,
    scale: f32,
) {
    let (position, rotation) = sample_bone_offsets(definition, bone, seconds, scale);
    part.pose = keyframe_animated_pose(part.pose, position, rotation);
}

/// Mutable copper golem model, mirroring vanilla `CopperGolemModel.createBodyLayer`. The base
/// renderer uses this same tree for both the cutout body and the emissive eyes texture. The vanilla
/// idle/interaction keyframe animations and custom head are deferred; the head look, walking /
/// walking-with-item keyframes, static held-item arm clamp, and antenna block transform are projected now.
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
        let definition = if render_state.copper_golem_holding_item {
            &COPPER_GOLEM_WALK_ITEM
        } else {
            &COPPER_GOLEM_WALK
        };
        let (seconds, scale) = keyframe_walk_sample(
            definition,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
            COPPER_GOLEM_WALK_SPEED_FACTOR,
            COPPER_GOLEM_WALK_SCALE_FACTOR,
        );
        {
            let body = self.root.child_mut("body");
            apply_copper_golem_keyframe(definition, body, "body", seconds, scale);
            {
                let head = body.child_mut("head");
                apply_head_look(head, render_state.head_yaw, render_state.head_pitch);
                apply_copper_golem_keyframe(definition, head, "head", seconds, scale);
            }
            apply_copper_golem_keyframe(
                definition,
                body.child_mut("right_arm"),
                "right_arm",
                seconds,
                scale,
            );
            apply_copper_golem_keyframe(
                definition,
                body.child_mut("left_arm"),
                "left_arm",
                seconds,
                scale,
            );
        }
        apply_copper_golem_keyframe(
            definition,
            self.root.child_mut("right_leg"),
            "right_leg",
            seconds,
            scale,
        );
        apply_copper_golem_keyframe(
            definition,
            self.root.child_mut("left_leg"),
            "left_leg",
            seconds,
            scale,
        );

        if render_state.copper_golem_holding_item {
            pose_held_item_arms_if_still(&mut self.root);
        }
    }
}
