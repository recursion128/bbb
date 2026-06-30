use super::{
    apply_head_look, degree_vec, keyframe, pos_vec, AnimationChannel, AnimationDefinition,
    AnimationTarget, BoneAnimation, Keyframe, KeyframeInterpolation, PartPose, COPPER_GOLEM_COPPER,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::{
    keyframe_animated_pose, keyframe_elapsed_seconds, keyframe_walk_sample, sample_bone_offsets,
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

// Vanilla 26.1 `CopperGolemAnimation.COPPER_GOLEM_IDLE` (length 3.5s, non-looping).
// `CopperGolemModel.setupAnim` applies it after the walk / held-item pose branch, so the body/head
// rotations are additive on top of the current walk and head-look pose.
const COPPER_GOLEM_IDLE_BODY_ROT: [Keyframe; 8] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(0.0, -35.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, -35.0, 0.0), LINEAR),
    keyframe(0.625, degree_vec(0.0, 35.0, 0.0), LINEAR),
    keyframe(1.2083, degree_vec(0.0, 35.0, 0.0), LINEAR),
    keyframe(2.7083, degree_vec(0.0, 35.0, 0.0), LINEAR),
    keyframe(3.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(3.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const COPPER_GOLEM_IDLE_HEAD_ROT: [Keyframe; 11] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.625, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2083, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5, degree_vec(0.0, 300.0, 0.0), LINEAR),
    keyframe(1.6667, degree_vec(0.0, 300.0, 0.0), LINEAR),
    keyframe(1.75, degree_vec(-25.0, 300.0, 0.0), LINEAR),
    keyframe(2.7083, degree_vec(-25.0, 300.0, 0.0), LINEAR),
    keyframe(3.0, degree_vec(0.0, 360.0, 0.0), LINEAR),
    keyframe(3.5, degree_vec(0.0, 360.0, 0.0), LINEAR),
];

const COPPER_GOLEM_IDLE_BODY_CHANNELS: [AnimationChannel; 1] = [rot(&COPPER_GOLEM_IDLE_BODY_ROT)];
const COPPER_GOLEM_IDLE_HEAD_CHANNELS: [AnimationChannel; 1] = [rot(&COPPER_GOLEM_IDLE_HEAD_ROT)];
const COPPER_GOLEM_IDLE_BONES: [BoneAnimation; 2] = [
    BoneAnimation {
        bone: "body",
        channels: &COPPER_GOLEM_IDLE_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &COPPER_GOLEM_IDLE_HEAD_CHANNELS,
    },
];

pub(in crate::entity_models) const COPPER_GOLEM_IDLE: AnimationDefinition = AnimationDefinition {
    length_seconds: 3.5,
    looping: false,
    bones: &COPPER_GOLEM_IDLE_BONES,
};

// Vanilla 26.1 `CopperGolemAnimation.COPPER_GOLEM_CHEST_INTERACTION_NOITEM_GET`
// (length 3.0s, looping). `CopperGolemModel.setupAnim` applies this after idle while
// `CopperGolemState.GETTING_ITEM` is active.
const COPPER_GOLEM_CHEST_NOITEM_GET_BODY_ROT: [Keyframe; 31] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(18.0, 0.0, 0.0), LINEAR),
    keyframe(0.2917, degree_vec(24.0, 0.0, 0.0), LINEAR),
    keyframe(0.375, degree_vec(15.0, 0.0, 0.0), LINEAR),
    keyframe(0.4167, degree_vec(12.5, 0.0, 0.0), LINEAR),
    keyframe(0.5833, degree_vec(12.5, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(12.5, 0.0, 0.0), LINEAR),
    keyframe(0.8333, degree_vec(14.72765, -31.63886, -7.85085), LINEAR),
    keyframe(0.9167, degree_vec(14.72765, -31.63886, -7.85085), LINEAR),
    keyframe(1.0417, degree_vec(14.72765, -31.63886, -7.85085), LINEAR),
    keyframe(1.125, degree_vec(12.40525, -4.4E-4, 0.00829), LINEAR),
    keyframe(1.2083, degree_vec(12.40525, -4.4E-4, 0.00829), LINEAR),
    keyframe(1.2917, degree_vec(13.92716, 26.80536, 6.38918), LINEAR),
    keyframe(1.625, degree_vec(13.93, 26.81, 6.39), LINEAR),
    keyframe(1.6667, degree_vec(21.43, 26.81, 6.39), LINEAR),
    keyframe(1.7917, degree_vec(21.43, 26.81, 6.39), LINEAR),
    keyframe(1.8333, degree_vec(13.93, 26.81, 6.39), LINEAR),
    keyframe(2.0417, degree_vec(13.93, 26.81, 6.39), LINEAR),
    keyframe(2.125, degree_vec(12.40725, 0.0, 0.00783), LINEAR),
    keyframe(2.1667, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.2917, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.375, degree_vec(15.0, 0.0, 0.0), LINEAR),
    keyframe(2.4167, degree_vec(17.5, 0.0, 0.0), LINEAR),
    keyframe(2.4583, degree_vec(22.5, 0.0, 0.0), LINEAR),
    keyframe(2.625, degree_vec(24.14867, -20.70481, -9.00717), LINEAR),
    keyframe(2.7083, degree_vec(24.14867, -20.70481, -9.00717), LINEAR),
    keyframe(2.75, degree_vec(22.5, 0.0, 0.0), LINEAR),
    keyframe(2.7917, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, degree_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_GET_BODY_POS: [Keyframe; 16] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, pos_vec(0.0, 0.6, 0.0), LINEAR),
    keyframe(0.4167, pos_vec(0.0, 0.5, 0.0), LINEAR),
    keyframe(1.625, pos_vec(0.0, 0.4, 0.0), LINEAR),
    keyframe(1.6667, pos_vec(-0.01805, 0.88303, -0.09783), LINEAR),
    keyframe(1.7917, pos_vec(-0.01805, 0.88303, -0.09783), LINEAR),
    keyframe(1.8333, pos_vec(0.0, 0.6, 0.0), LINEAR),
    keyframe(2.0417, pos_vec(0.0, 0.6, 0.0), LINEAR),
    keyframe(2.1667, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.25, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.2917, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.625, pos_vec(0.0, 0.46194, -0.19134), LINEAR),
    keyframe(2.7083, pos_vec(0.0, 0.46194, -0.19134), LINEAR),
    keyframe(2.7917, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, pos_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_GET_HEAD_ROT: [Keyframe; 39] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(-2.5, 0.0, 0.0), LINEAR),
    keyframe(0.2917, degree_vec(-5.0, 0.0, 0.0), LINEAR),
    keyframe(0.375, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.4167, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5833, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.6667, degree_vec(0.0, -20.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(0.0, -20.0, 0.0), LINEAR),
    keyframe(0.8333, degree_vec(0.0, 10.0, 0.0), LINEAR),
    keyframe(1.0417, degree_vec(0.0, 10.0, 0.0), LINEAR),
    keyframe(1.1667, degree_vec(0.0, 10.0, 0.0), LINEAR),
    keyframe(1.2083, degree_vec(0.0, 27.5, 0.0), LINEAR),
    keyframe(1.2917, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4167, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5833, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.625, degree_vec(0.0, -2.5, 0.0), LINEAR),
    keyframe(1.6667, degree_vec(10.16381, -16.71134, -6.35306), LINEAR),
    keyframe(1.8333, degree_vec(10.16381, -16.71134, -6.35306), LINEAR),
    keyframe(1.9167, degree_vec(10.16381, -16.71134, -6.35306), LINEAR),
    keyframe(1.9583, degree_vec(10.16381, -16.71134, -6.35306), LINEAR),
    keyframe(2.0, degree_vec(5.16381, -16.71134, -6.35306), LINEAR),
    keyframe(2.0417, degree_vec(0.16381, -16.71134, -6.35306), LINEAR),
    keyframe(2.0833, degree_vec(0.15732, -4.21139, -6.31751), LINEAR),
    keyframe(2.125, degree_vec(0.07901, 5.3943, -3.15187), LINEAR),
    keyframe(2.1667, degree_vec(0.0, 7.5, 0.0), LINEAR),
    keyframe(2.2917, degree_vec(4.53867, 7.47675, 0.59181), LINEAR),
    keyframe(2.3333, degree_vec(-2.53852, 9.99038, -0.44067), LINEAR),
    keyframe(2.4167, degree_vec(-12.68664, 9.76061, -2.18558), LINEAR),
    keyframe(2.625, degree_vec(-15.19938, 22.36971, -3.52259), LINEAR),
    keyframe(2.6667, degree_vec(-3.02173, 22.37156, -2.41802), LINEAR),
    keyframe(2.7083, degree_vec(-0.52173, 22.37156, -2.41802), LINEAR),
    keyframe(2.75, degree_vec(-12.40598, -0.4674, -1.79838), LINEAR),
    keyframe(2.8333, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, degree_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_GET_HEAD_POS: [Keyframe; 25] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, pos_vec(0.0, -0.15451, 0.47553), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.4167, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0417, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.1667, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2917, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4167, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5833, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.625, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.6667, pos_vec(-0.22438, 0.82319, -1.27252), LINEAR),
    keyframe(1.8333, pos_vec(-0.22438, 0.82319, -1.27252), LINEAR),
    keyframe(1.9167, pos_vec(-0.22438, 0.82319, -1.27252), LINEAR),
    keyframe(1.9583, pos_vec(-0.52521, 0.96725, -0.32978), LINEAR),
    keyframe(2.0, pos_vec(-0.52521, 0.96725, -0.32978), LINEAR),
    keyframe(2.0417, pos_vec(-0.5345, 1.16541, -0.37206), LINEAR),
    keyframe(2.0833, pos_vec(-0.5345, 1.16541, -0.37206), LINEAR),
    keyframe(2.1667, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.3333, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.625, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.8333, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, pos_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_GET_RIGHT_ARM_ROT: [Keyframe; 37] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(-7.38733, 1.29876, 9.91615), LINEAR),
    keyframe(0.2917, degree_vec(-7.38733, 1.29876, 9.91615), LINEAR),
    keyframe(0.375, degree_vec(10.0, 0.0, 32.5), LINEAR),
    keyframe(0.4167, degree_vec(-34.55418, 11.73507, 36.8361), LINEAR),
    keyframe(0.4583, degree_vec(-82.47403, 17.82361, 2.17224), LINEAR),
    keyframe(0.5, degree_vec(-85.08388, 14.26971, 1.99595), LINEAR),
    keyframe(0.5417, degree_vec(-85.16266, 13.19102, 2.43976), LINEAR),
    keyframe(0.5833, degree_vec(-92.79, 0.73, 1.39), LINEAR),
    keyframe(0.75, degree_vec(-92.79, 0.73, 1.39), LINEAR),
    keyframe(0.8333, degree_vec(-95.83405, 33.18639, -0.40081), LINEAR),
    keyframe(1.25, degree_vec(-95.83, 33.19, -0.4), LINEAR),
    keyframe(1.2917, degree_vec(-98.33, 33.19, -0.4), LINEAR),
    keyframe(1.5417, degree_vec(-56.46674, 3.3853, 14.45894), LINEAR),
    keyframe(1.6667, degree_vec(-56.46674, 3.3853, 14.45894), LINEAR),
    keyframe(1.8333, degree_vec(-56.46674, 3.3853, 14.45894), LINEAR),
    keyframe(2.0, degree_vec(-56.46674, 3.3853, 14.45894), LINEAR),
    keyframe(2.0417, degree_vec(-56.46674, 3.3853, 14.45894), LINEAR),
    keyframe(2.1667, degree_vec(-84.12204, 8.95753, 14.11779), LINEAR),
    keyframe(2.2083, degree_vec(-84.12204, 8.95753, 14.11779), LINEAR),
    keyframe(2.25, degree_vec(-93.6065, 13.90544, 15.98524), LINEAR),
    keyframe(2.3333, degree_vec(-124.48661, 66.29146, -7.28605), LINEAR),
    keyframe(2.375, degree_vec(-129.4866, 66.29146, -7.28605), LINEAR),
    keyframe(2.4167, degree_vec(-108.91607, 1.79762, 20.93924), LINEAR),
    keyframe(2.5, degree_vec(-102.18303, 4.35881, 17.40962), LINEAR),
    keyframe(2.5417, degree_vec(-98.33642, -0.70114, 4.09322), LINEAR),
    keyframe(2.625, degree_vec(-98.39385, 6.71929, 3.00137), LINEAR),
    keyframe(2.6667, degree_vec(-98.33981, 1.77244, 3.7307), LINEAR),
    keyframe(2.7083, degree_vec(-100.70987, 3.48829, 7.1138), LINEAR),
    keyframe(2.75, degree_vec(-97.95, 6.92, 13.88), LINEAR),
    keyframe(2.7917, degree_vec(-87.95, 6.92, 13.88), LINEAR),
    keyframe(2.8333, degree_vec(-97.95, 6.92, 13.88), LINEAR),
    keyframe(2.875, degree_vec(-102.95, 6.92, 13.88), LINEAR),
    keyframe(2.9167, degree_vec(-76.475, 3.46, 6.94), LINEAR),
    keyframe(2.9583, degree_vec(-26.475, 3.46, 6.94), LINEAR),
    keyframe(3.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_GET_RIGHT_ARM_POS: [Keyframe; 29] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2917, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.375, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5833, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.8333, pos_vec(0.25358, -0.20153, 2.21248), LINEAR),
    keyframe(1.25, pos_vec(0.25, -0.2, 2.21), LINEAR),
    keyframe(1.2917, pos_vec(0.25, -0.2, 2.21), LINEAR),
    keyframe(1.5417, pos_vec(-0.26323, -1.46323, 0.66566), LINEAR),
    keyframe(1.6667, pos_vec(-0.26323, -1.46323, 0.66566), LINEAR),
    keyframe(1.8333, pos_vec(-0.26323, -1.46323, 0.66566), LINEAR),
    keyframe(2.0, pos_vec(-0.26323, -1.46323, 0.66566), LINEAR),
    keyframe(2.0417, pos_vec(-0.26323, -1.46323, 0.66566), LINEAR),
    keyframe(2.25, pos_vec(-0.51, -0.38, 0.8), LINEAR),
    keyframe(2.3333, pos_vec(-0.51, -0.38, 0.8), LINEAR),
    keyframe(2.375, pos_vec(-0.51, -0.38, 0.8), LINEAR),
    keyframe(2.4167, pos_vec(-2.14094, 0.69619, 1.23422), LINEAR),
    keyframe(2.4583, pos_vec(-0.97932, 0.38244, 0.12884), LINEAR),
    keyframe(2.5417, pos_vec(-1.55232, 1.79904, 0.37956), LINEAR),
    keyframe(2.625, pos_vec(-1.53125, 1.64598, 1.41168), LINEAR),
    keyframe(2.6667, pos_vec(-1.57256, 1.05375, 1.32469), LINEAR),
    keyframe(2.75, pos_vec(-1.33, 0.16, 1.02), LINEAR),
    keyframe(2.7917, pos_vec(-1.33, 0.16, 1.02), LINEAR),
    keyframe(2.8333, pos_vec(-1.33, 0.16, 1.02), LINEAR),
    keyframe(2.875, pos_vec(-1.33, 0.16, 1.02), LINEAR),
    keyframe(2.9167, pos_vec(-0.5748, 0.38848, 1.45646), LINEAR),
    keyframe(2.9583, pos_vec(-0.67, 0.08, 0.51), LINEAR),
    keyframe(3.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_GET_LEFT_ARM_ROT: [Keyframe; 25] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, degree_vec(25.0, 0.0, -37.5), LINEAR),
    keyframe(0.25, degree_vec(-21.59341, -12.60837, -45.69252), LINEAR),
    keyframe(0.2917, degree_vec(-120.7755, -5.21988, -2.02064), LINEAR),
    keyframe(0.375, degree_vec(-98.27419, -1.79323, -1.15048), LINEAR),
    keyframe(0.4167, degree_vec(-93.27, -1.79, -1.15), LINEAR),
    keyframe(0.5417, degree_vec(-93.27419, -1.79323, -1.15048), LINEAR),
    keyframe(0.5833, degree_vec(-93.55693, -22.3224, 3.64383), LINEAR),
    keyframe(0.7083, degree_vec(-93.55693, -22.3224, 3.64383), LINEAR),
    keyframe(1.1667, degree_vec(-93.55693, -22.3224, 3.64383), LINEAR),
    keyframe(1.2083, degree_vec(-95.75, -2.42, 5.97), LINEAR),
    keyframe(1.25, degree_vec(-98.4029, -17.39503, 6.85104), LINEAR),
    keyframe(1.2917, degree_vec(-101.24523, -29.87096, 7.69993), LINEAR),
    keyframe(1.5833, degree_vec(-101.25, -29.87, 7.7), LINEAR),
    keyframe(1.6667, degree_vec(-88.17772, -42.09094, 10.96195), LINEAR),
    keyframe(1.8333, degree_vec(-88.17772, -42.09094, 10.96195), LINEAR),
    keyframe(1.9583, degree_vec(-88.17772, -42.09094, 10.96195), LINEAR),
    keyframe(2.1667, degree_vec(-88.17772, -42.09094, 10.96195), LINEAR),
    keyframe(2.2083, degree_vec(-88.58526, -17.10045, 11.7676), LINEAR),
    keyframe(2.3333, degree_vec(-88.59, -17.1, 11.77), LINEAR),
    keyframe(2.4167, degree_vec(-46.59531, -16.13694, -3.85578), LINEAR),
    keyframe(2.5, degree_vec(-24.5317, -19.0214, -13.70805), LINEAR),
    keyframe(2.8333, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, degree_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_GET_LEFT_ARM_POS: [Keyframe; 10] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.4167, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5833, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.6667, pos_vec(-0.00677, -0.76064, 3.19059), LINEAR),
    keyframe(2.3333, pos_vec(0.0512, -0.76176, 3.12882), LINEAR),
    keyframe(2.8333, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, pos_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_GET_RIGHT_LEG_ROT: [Keyframe; 9] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, degree_vec(7.5, 0.0, 0.0), LINEAR),
    keyframe(2.2917, degree_vec(7.5, 0.0, 0.0), LINEAR),
    keyframe(2.7083, degree_vec(7.5, 0.0, 0.0), LINEAR),
    keyframe(2.7917, degree_vec(5.0, 0.0, 0.0), LINEAR),
    keyframe(2.8333, degree_vec(5.0, 0.0, 0.0), LINEAR),
    keyframe(2.875, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, degree_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_GET_RIGHT_LEG_POS: [Keyframe; 8] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.2917, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.7083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.7917, pos_vec(0.0, 0.09, -0.11), LINEAR),
    keyframe(2.8333, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, pos_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_GET_LEFT_LEG_ROT: [Keyframe; 11] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(2.0417, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(2.25, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(2.3333, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(2.4167, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(2.4583, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(2.5, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(2.625, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, degree_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_GET_LEFT_LEG_POS: [Keyframe; 11] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.0417, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.25, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.3333, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.4167, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.4583, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.625, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, pos_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_GET_BODY_CHANNELS: [AnimationChannel; 2] = [
    rot(&COPPER_GOLEM_CHEST_NOITEM_GET_BODY_ROT),
    pos(&COPPER_GOLEM_CHEST_NOITEM_GET_BODY_POS),
];
const COPPER_GOLEM_CHEST_NOITEM_GET_HEAD_CHANNELS: [AnimationChannel; 2] = [
    rot(&COPPER_GOLEM_CHEST_NOITEM_GET_HEAD_ROT),
    pos(&COPPER_GOLEM_CHEST_NOITEM_GET_HEAD_POS),
];
const COPPER_GOLEM_CHEST_NOITEM_GET_RIGHT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&COPPER_GOLEM_CHEST_NOITEM_GET_RIGHT_ARM_ROT),
    pos(&COPPER_GOLEM_CHEST_NOITEM_GET_RIGHT_ARM_POS),
];
const COPPER_GOLEM_CHEST_NOITEM_GET_LEFT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&COPPER_GOLEM_CHEST_NOITEM_GET_LEFT_ARM_ROT),
    pos(&COPPER_GOLEM_CHEST_NOITEM_GET_LEFT_ARM_POS),
];
const COPPER_GOLEM_CHEST_NOITEM_GET_RIGHT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&COPPER_GOLEM_CHEST_NOITEM_GET_RIGHT_LEG_ROT),
    pos(&COPPER_GOLEM_CHEST_NOITEM_GET_RIGHT_LEG_POS),
];
const COPPER_GOLEM_CHEST_NOITEM_GET_LEFT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&COPPER_GOLEM_CHEST_NOITEM_GET_LEFT_LEG_ROT),
    pos(&COPPER_GOLEM_CHEST_NOITEM_GET_LEFT_LEG_POS),
];

const COPPER_GOLEM_CHEST_NOITEM_GET_BONES: [BoneAnimation; 6] = [
    BoneAnimation {
        bone: "body",
        channels: &COPPER_GOLEM_CHEST_NOITEM_GET_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &COPPER_GOLEM_CHEST_NOITEM_GET_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_arm",
        channels: &COPPER_GOLEM_CHEST_NOITEM_GET_RIGHT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_arm",
        channels: &COPPER_GOLEM_CHEST_NOITEM_GET_LEFT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "right_leg",
        channels: &COPPER_GOLEM_CHEST_NOITEM_GET_RIGHT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_leg",
        channels: &COPPER_GOLEM_CHEST_NOITEM_GET_LEFT_LEG_CHANNELS,
    },
];

pub(in crate::entity_models) const COPPER_GOLEM_CHEST_INTERACTION_NOITEM_GET: AnimationDefinition =
    AnimationDefinition {
        length_seconds: 3.0,
        looping: true,
        bones: &COPPER_GOLEM_CHEST_NOITEM_GET_BONES,
    };

// Vanilla 26.1 `CopperGolemAnimation.COPPER_GOLEM_CHEST_INTERACTION_NOITEM_NOGET`
// (length 3.0s, looping). `CopperGolemModel.setupAnim` applies this after the get-item
// interaction while `CopperGolemState.GETTING_NO_ITEM` is active.
const COPPER_GOLEM_CHEST_NOITEM_NOGET_BODY_ROT: [Keyframe; 23] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(18.0, 0.0, 0.0), LINEAR),
    keyframe(0.2917, degree_vec(24.0, 0.0, 0.0), LINEAR),
    keyframe(0.375, degree_vec(15.0, 0.0, 0.0), LINEAR),
    keyframe(0.4167, degree_vec(12.5, 0.0, 0.0), LINEAR),
    keyframe(0.5833, degree_vec(12.5, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(12.5, 0.0, 0.0), LINEAR),
    keyframe(0.8333, degree_vec(14.72765, -31.63886, -7.85085), LINEAR),
    keyframe(0.9167, degree_vec(14.72765, -31.63886, -7.85085), LINEAR),
    keyframe(1.0417, degree_vec(14.72765, -31.63886, -7.85085), LINEAR),
    keyframe(1.125, degree_vec(12.40525, -4.4E-4, 0.00829), LINEAR),
    keyframe(1.2083, degree_vec(12.40525, -4.4E-4, 0.00829), LINEAR),
    keyframe(1.2917, degree_vec(13.92716, 26.80536, 6.38918), LINEAR),
    keyframe(1.625, degree_vec(13.93, 26.81, 6.39), LINEAR),
    keyframe(1.7083, degree_vec(12.40725, 0.00444, 0.00783), LINEAR),
    keyframe(2.0417, degree_vec(12.40725, 0.00444, 0.00783), LINEAR),
    keyframe(2.125, degree_vec(12.40725, 0.0, 0.00783), LINEAR),
    keyframe(2.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.4583, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.6667, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, degree_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_NOGET_BODY_POS: [Keyframe; 12] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, pos_vec(0.0, 0.6, 0.0), LINEAR),
    keyframe(0.4167, pos_vec(0.0, 0.5, 0.0), LINEAR),
    keyframe(1.625, pos_vec(0.0, 0.4, 0.0), LINEAR),
    keyframe(1.7083, pos_vec(0.0, 0.34, 0.0), LINEAR),
    keyframe(2.0417, pos_vec(0.0, 0.34, 0.0), LINEAR),
    keyframe(2.25, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.4583, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.6667, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, pos_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_NOGET_HEAD_ROT: [Keyframe; 32] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(-2.5, 0.0, 0.0), LINEAR),
    keyframe(0.2917, degree_vec(-5.0, 0.0, 0.0), LINEAR),
    keyframe(0.375, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.4167, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5833, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.6667, degree_vec(0.0, -20.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(0.0, -20.0, 0.0), LINEAR),
    keyframe(0.8333, degree_vec(0.0, 10.0, 0.0), LINEAR),
    keyframe(1.0417, degree_vec(0.0, 10.0, 0.0), LINEAR),
    keyframe(1.1667, degree_vec(0.0, 10.0, 0.0), LINEAR),
    keyframe(1.2083, degree_vec(0.0, 27.5, 0.0), LINEAR),
    keyframe(1.2917, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4167, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5833, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.625, degree_vec(0.0, -2.5, 0.0), LINEAR),
    keyframe(1.7083, degree_vec(0.57, -1.25, 0.07), LINEAR),
    keyframe(1.75, degree_vec(0.89798, -18.12465, -0.16276), LINEAR),
    keyframe(1.7917, degree_vec(1.21328, -21.15422, -0.2148), LINEAR),
    keyframe(1.875, degree_vec(1.21328, -21.15422, -0.2148), LINEAR),
    keyframe(2.0, degree_vec(1.21328, -21.15422, -0.2148), LINEAR),
    keyframe(2.0417, degree_vec(2.56546, 0.76525, 0.57246), LINEAR),
    keyframe(2.2917, degree_vec(4.53867, 7.47675, 0.59181), LINEAR),
    keyframe(2.5, degree_vec(4.53867, 7.47675, 0.59181), LINEAR),
    keyframe(2.5417, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, degree_vec(0.0, -360.0, 0.0), LINEAR),
    keyframe(3.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_NOGET_HEAD_POS: [Keyframe; 17] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, pos_vec(0.0, -0.15451, 0.47553), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.4167, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0417, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.1667, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2917, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4167, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5833, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.625, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.7083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.5417, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, pos_vec(0.0, -0.01, -0.03), LINEAR),
    keyframe(3.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_NOGET_RIGHT_ARM_ROT: [Keyframe; 25] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(-7.38733, 1.29876, 9.91615), LINEAR),
    keyframe(0.2917, degree_vec(-7.38733, 1.29876, 9.91615), LINEAR),
    keyframe(0.375, degree_vec(10.0, 0.0, 32.5), LINEAR),
    keyframe(0.4167, degree_vec(-34.55418, 11.73507, 36.8361), LINEAR),
    keyframe(0.4583, degree_vec(-82.47403, 17.82361, 2.17224), LINEAR),
    keyframe(0.5, degree_vec(-85.08388, 14.26971, 1.99595), LINEAR),
    keyframe(0.5417, degree_vec(-85.16266, 13.19102, 2.43976), LINEAR),
    keyframe(0.5833, degree_vec(-92.79, 0.73, 1.39), LINEAR),
    keyframe(0.75, degree_vec(-92.79, 0.73, 1.39), LINEAR),
    keyframe(0.8333, degree_vec(-95.83405, 33.18639, -0.40081), LINEAR),
    keyframe(1.25, degree_vec(-95.83, 33.19, -0.4), LINEAR),
    keyframe(1.2917, degree_vec(-98.33, 33.19, -0.4), LINEAR),
    keyframe(1.5417, degree_vec(-56.46674, 3.3853, 14.45894), LINEAR),
    keyframe(1.6667, degree_vec(-56.46674, 3.3853, 14.45894), LINEAR),
    keyframe(1.8333, degree_vec(-56.46674, 3.3853, 14.45894), LINEAR),
    keyframe(2.0, degree_vec(-56.46674, 3.3853, 14.45894), LINEAR),
    keyframe(2.0417, degree_vec(-56.46674, 3.3853, 14.45894), LINEAR),
    keyframe(2.1667, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.5417, degree_vec(3.9, -4.38, 3.36), LINEAR),
    keyframe(2.9167, degree_vec(3.9, -4.38, 3.36), LINEAR),
    keyframe(2.9583, degree_vec(3.90089, -4.3843, 3.35549), LINEAR),
    keyframe(3.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_NOGET_RIGHT_ARM_POS: [Keyframe; 20] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2917, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.375, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5833, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.8333, pos_vec(0.25358, -0.20153, 2.21248), LINEAR),
    keyframe(1.25, pos_vec(0.25, -0.2, 2.21), LINEAR),
    keyframe(1.2917, pos_vec(0.25, -0.2, 2.21), LINEAR),
    keyframe(1.5417, pos_vec(-0.26323, -1.46323, 0.66566), LINEAR),
    keyframe(1.6667, pos_vec(-0.26323, -1.46323, 0.66566), LINEAR),
    keyframe(1.8333, pos_vec(-0.26323, -1.46323, 0.66566), LINEAR),
    keyframe(2.0, pos_vec(-0.26323, -1.46323, 0.66566), LINEAR),
    keyframe(2.0417, pos_vec(-0.26323, -1.46323, 0.66566), LINEAR),
    keyframe(2.1667, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.5417, pos_vec(-0.46, -0.88, -0.3), LINEAR),
    keyframe(2.9167, pos_vec(-0.46, -0.88, -0.3), LINEAR),
    keyframe(2.9583, pos_vec(-0.46, 0.1159, -0.30086), LINEAR),
    keyframe(3.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_NOGET_LEFT_ARM_ROT: [Keyframe; 28] = [
    keyframe(0.0, degree_vec(-2.5, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, degree_vec(25.0, 0.0, -37.5), LINEAR),
    keyframe(0.25, degree_vec(-21.59341, -12.60837, -45.69252), LINEAR),
    keyframe(0.2917, degree_vec(-120.7755, -5.21988, -2.02064), LINEAR),
    keyframe(0.375, degree_vec(-98.27419, -1.79323, -1.15048), LINEAR),
    keyframe(0.4167, degree_vec(-93.27, -1.79, -1.15), LINEAR),
    keyframe(0.5417, degree_vec(-93.27419, -1.79323, -1.15048), LINEAR),
    keyframe(0.5833, degree_vec(-93.55693, -22.3224, 3.64383), LINEAR),
    keyframe(0.7083, degree_vec(-93.55693, -22.3224, 3.64383), LINEAR),
    keyframe(1.1667, degree_vec(-93.55693, -22.3224, 3.64383), LINEAR),
    keyframe(1.2083, degree_vec(-95.75, -2.42, 5.97), LINEAR),
    keyframe(1.25, degree_vec(-98.4029, -17.39503, 6.85104), LINEAR),
    keyframe(1.2917, degree_vec(-101.24523, -29.87096, 7.69993), LINEAR),
    keyframe(1.5833, degree_vec(-101.25, -29.87, 7.7), LINEAR),
    keyframe(1.6667, degree_vec(-88.17772, -42.09094, 10.96195), LINEAR),
    keyframe(1.8333, degree_vec(-88.17772, -42.09094, 10.96195), LINEAR),
    keyframe(2.0833, degree_vec(-88.17772, -42.09094, 10.96195), LINEAR),
    keyframe(2.1667, degree_vec(-88.17772, -42.09094, 10.96195), LINEAR),
    keyframe(2.2083, degree_vec(-88.58526, -17.10045, 11.7676), LINEAR),
    keyframe(2.3333, degree_vec(-88.59, -17.1, 11.77), LINEAR),
    keyframe(2.4167, degree_vec(-46.59531, -16.13694, -3.85578), LINEAR),
    keyframe(2.4583, degree_vec(-24.5317, -19.0214, -13.70805), LINEAR),
    keyframe(2.5, degree_vec(-24.5317, -19.0214, -13.70805), LINEAR),
    keyframe(2.5417, degree_vec(2.41, -0.65, -5.01), LINEAR),
    keyframe(2.9167, degree_vec(2.41, -0.65, -5.01), LINEAR),
    keyframe(2.9583, degree_vec(2.41492, -0.64686, -5.01363), LINEAR),
    keyframe(3.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_NOGET_LEFT_ARM_POS: [Keyframe; 14] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.4167, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5833, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.6667, pos_vec(-0.00677, -0.76064, 3.19059), LINEAR),
    keyframe(2.3333, pos_vec(0.0512, -0.76176, 3.12882), LINEAR),
    keyframe(2.4583, pos_vec(0.03, -0.51, 2.09), LINEAR),
    keyframe(2.5, pos_vec(0.03, -0.51, 2.09), LINEAR),
    keyframe(2.5417, pos_vec(0.03, -1.28, -0.07), LINEAR),
    keyframe(2.9167, pos_vec(0.03, -1.28, -0.07), LINEAR),
    keyframe(2.9583, pos_vec(0.03, -0.28229, -0.07133), LINEAR),
    keyframe(3.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_NOGET_RIGHT_LEG_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, degree_vec(7.5, 0.0, 0.0), LINEAR),
    keyframe(2.2917, degree_vec(7.5, 0.0, 0.0), LINEAR),
    keyframe(2.3333, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, degree_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_NOGET_RIGHT_LEG_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.2917, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.3333, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, pos_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_NOGET_LEFT_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(2.0417, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(2.2917, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(2.3333, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, degree_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_NOGET_LEFT_LEG_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.0417, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.2917, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.3333, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.9583, pos_vec(0.0, 0.0, 0.0), LINEAR),
];

const COPPER_GOLEM_CHEST_NOITEM_NOGET_BODY_CHANNELS: [AnimationChannel; 2] = [
    rot(&COPPER_GOLEM_CHEST_NOITEM_NOGET_BODY_ROT),
    pos(&COPPER_GOLEM_CHEST_NOITEM_NOGET_BODY_POS),
];
const COPPER_GOLEM_CHEST_NOITEM_NOGET_HEAD_CHANNELS: [AnimationChannel; 2] = [
    rot(&COPPER_GOLEM_CHEST_NOITEM_NOGET_HEAD_ROT),
    pos(&COPPER_GOLEM_CHEST_NOITEM_NOGET_HEAD_POS),
];
const COPPER_GOLEM_CHEST_NOITEM_NOGET_RIGHT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&COPPER_GOLEM_CHEST_NOITEM_NOGET_RIGHT_ARM_ROT),
    pos(&COPPER_GOLEM_CHEST_NOITEM_NOGET_RIGHT_ARM_POS),
];
const COPPER_GOLEM_CHEST_NOITEM_NOGET_LEFT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&COPPER_GOLEM_CHEST_NOITEM_NOGET_LEFT_ARM_ROT),
    pos(&COPPER_GOLEM_CHEST_NOITEM_NOGET_LEFT_ARM_POS),
];
const COPPER_GOLEM_CHEST_NOITEM_NOGET_RIGHT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&COPPER_GOLEM_CHEST_NOITEM_NOGET_RIGHT_LEG_ROT),
    pos(&COPPER_GOLEM_CHEST_NOITEM_NOGET_RIGHT_LEG_POS),
];
const COPPER_GOLEM_CHEST_NOITEM_NOGET_LEFT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&COPPER_GOLEM_CHEST_NOITEM_NOGET_LEFT_LEG_ROT),
    pos(&COPPER_GOLEM_CHEST_NOITEM_NOGET_LEFT_LEG_POS),
];

const COPPER_GOLEM_CHEST_NOITEM_NOGET_BONES: [BoneAnimation; 6] = [
    BoneAnimation {
        bone: "body",
        channels: &COPPER_GOLEM_CHEST_NOITEM_NOGET_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &COPPER_GOLEM_CHEST_NOITEM_NOGET_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_arm",
        channels: &COPPER_GOLEM_CHEST_NOITEM_NOGET_RIGHT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_arm",
        channels: &COPPER_GOLEM_CHEST_NOITEM_NOGET_LEFT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "right_leg",
        channels: &COPPER_GOLEM_CHEST_NOITEM_NOGET_RIGHT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_leg",
        channels: &COPPER_GOLEM_CHEST_NOITEM_NOGET_LEFT_LEG_CHANNELS,
    },
];

pub(in crate::entity_models) const COPPER_GOLEM_CHEST_INTERACTION_NOITEM_NOGET:
    AnimationDefinition = AnimationDefinition {
    length_seconds: 3.0,
    looping: true,
    bones: &COPPER_GOLEM_CHEST_NOITEM_NOGET_BONES,
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

fn apply_copper_golem_full_keyframe(
    definition: &AnimationDefinition,
    root: &mut ModelPart,
    seconds: f32,
    scale: f32,
) {
    {
        let body = root.child_mut("body");
        apply_copper_golem_keyframe(definition, body, "body", seconds, scale);
        apply_copper_golem_keyframe(definition, body.child_mut("head"), "head", seconds, scale);
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
        root.child_mut("right_leg"),
        "right_leg",
        seconds,
        scale,
    );
    apply_copper_golem_keyframe(
        definition,
        root.child_mut("left_leg"),
        "left_leg",
        seconds,
        scale,
    );
}

/// Mutable copper golem model, mirroring vanilla `CopperGolemModel.createBodyLayer`. The base
/// renderer uses this same tree for both the cutout body and the emissive eyes texture. The vanilla
/// remaining interaction keyframe animations and custom head are deferred; the head look, walking /
/// walking-with-item keyframes, idle / noitem-get interaction keyframes, static held-item arm clamp,
/// and antenna block transform are projected now.
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

        if render_state.copper_golem_idle_seconds >= 0.0 {
            let idle_seconds = keyframe_elapsed_seconds(
                &COPPER_GOLEM_IDLE,
                render_state.copper_golem_idle_seconds,
            );
            let body = self.root.child_mut("body");
            apply_copper_golem_keyframe(&COPPER_GOLEM_IDLE, body, "body", idle_seconds, 1.0);
            apply_copper_golem_keyframe(
                &COPPER_GOLEM_IDLE,
                body.child_mut("head"),
                "head",
                idle_seconds,
                1.0,
            );
        }

        if render_state.copper_golem_get_item_seconds >= 0.0 {
            let interaction_seconds = keyframe_elapsed_seconds(
                &COPPER_GOLEM_CHEST_INTERACTION_NOITEM_GET,
                render_state.copper_golem_get_item_seconds,
            );
            apply_copper_golem_full_keyframe(
                &COPPER_GOLEM_CHEST_INTERACTION_NOITEM_GET,
                &mut self.root,
                interaction_seconds,
                1.0,
            );
        }

        if render_state.copper_golem_get_no_item_seconds >= 0.0 {
            let interaction_seconds = keyframe_elapsed_seconds(
                &COPPER_GOLEM_CHEST_INTERACTION_NOITEM_NOGET,
                render_state.copper_golem_get_no_item_seconds,
            );
            apply_copper_golem_full_keyframe(
                &COPPER_GOLEM_CHEST_INTERACTION_NOITEM_NOGET,
                &mut self.root,
                interaction_seconds,
                1.0,
            );
        }
    }
}
