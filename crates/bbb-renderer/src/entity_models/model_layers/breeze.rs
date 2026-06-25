use super::{
    degree_vec, keyframe, pos_vec, AnimationChannel, AnimationDefinition, AnimationTarget,
    BoneAnimation, Keyframe, KeyframeInterpolation, PartPose, BREEZE_SLATE, PART_POSE_ZERO,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::{
    keyframe_animated_pose, keyframe_elapsed_seconds, sample_bone_offsets,
    sample_bone_offsets_with_scale, scale_vec,
};

const LINEAR: KeyframeInterpolation = KeyframeInterpolation::Linear;

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
const fn scale_channel(keyframes: &'static [Keyframe]) -> AnimationChannel {
    AnimationChannel {
        target: AnimationTarget::Scale,
        keyframes,
    }
}
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

use KeyframeInterpolation::{CatmullRom, Linear};

// Vanilla 26.1 `BreezeModel.createBodyLayer` (atlas 32×32): the base body layer retains only the
// `head` (with its emissive `eyes` child) and the three `rods` under the `body` pivot; the swirling
// `wind_body` is a separate translucent layer. The colored path approximates the wind body's
// translucent blue with a single representative slate. Each cube carries both render paths' data:
// the colored debug tint (`BREEZE_SLATE`) and the textured `uv_size` / `texOffs` / `mirror`; no
// `CubeDeformation`, so each `uv_size` matches its box `size`. The head is the `texOffs(4, 24)`
// 10×3×4 jaw plate plus the `texOffs(0, 0)` 8×8×8 head cube.
pub(in crate::entity_models) const BREEZE_HEAD: [ModelCube; 2] = [
    ModelCube::new(
        [-5.0, -5.0, -4.2],
        [10.0, 3.0, 4.0],
        BREEZE_SLATE,
        [10.0, 3.0, 4.0],
        [4.0, 24.0],
        false,
    ),
    ModelCube::new(
        [-4.0, -8.0, -4.0],
        [8.0, 8.0, 8.0],
        BREEZE_SLATE,
        [8.0, 8.0, 8.0],
        [0.0, 0.0],
        false,
    ),
];

// All three rods share the same `texOffs(0, 17)` 2×8×2 box; only their bind pose differs.
pub(in crate::entity_models) const BREEZE_ROD: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -3.0],
    [2.0, 8.0, 2.0],
    BREEZE_SLATE,
    [2.0, 8.0, 2.0],
    [0.0, 17.0],
    false,
)];

pub(in crate::entity_models) const BREEZE_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BREEZE_RODS_POSE: PartPose = PartPose {
    offset: [0.0, 8.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BREEZE_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 4.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BREEZE_ROD_1_POSE: PartPose = PartPose {
    offset: [2.5981, -3.0, 1.5],
    rotation: [-2.7489, -1.0472, 3.1416],
};
pub(in crate::entity_models) const BREEZE_ROD_2_POSE: PartPose = PartPose {
    offset: [-2.5981, -3.0, 1.5],
    rotation: [-2.7489, 1.0472, 3.1416],
};
pub(in crate::entity_models) const BREEZE_ROD_3_POSE: PartPose = PartPose {
    offset: [0.0, -3.0, -3.0],
    rotation: [0.3927, 0.0, 0.0],
};

// Vanilla 26.1 `BreezeModel.createWindLayer` (atlas 128×128): the swirling translucent wind body, a
// SEPARATE scrolling `breeze_wind.png` layer ([`BreezeWindModel`]) retaining only the `wind_body`
// pivot → `wind_bottom` → `wind_mid` → `wind_top` chain (the base body's 32×32 `head`/`rods` are a
// different layer). Each tier nests three concentric shells of decreasing radius. The colored debug
// path approximates the translucent blue with `BREEZE_SLATE`; no `CubeDeformation`, so each
// `uv_size` matches its box `size`, and `texOffs` is normalized against the 128×128 wind atlas.
pub(in crate::entity_models) const BREEZE_WIND_BOTTOM: [ModelCube; 1] = [ModelCube::new(
    [-2.5, -7.0, -2.5],
    [5.0, 7.0, 5.0],
    BREEZE_SLATE,
    [5.0, 7.0, 5.0],
    [1.0, 83.0],
    false,
)];
pub(in crate::entity_models) const BREEZE_WIND_MID: [ModelCube; 3] = [
    ModelCube::new(
        [-6.0, -6.0, -6.0],
        [12.0, 6.0, 12.0],
        BREEZE_SLATE,
        [12.0, 6.0, 12.0],
        [74.0, 28.0],
        false,
    ),
    ModelCube::new(
        [-4.0, -6.0, -4.0],
        [8.0, 6.0, 8.0],
        BREEZE_SLATE,
        [8.0, 6.0, 8.0],
        [78.0, 32.0],
        false,
    ),
    ModelCube::new(
        [-2.5, -6.0, -2.5],
        [5.0, 6.0, 5.0],
        BREEZE_SLATE,
        [5.0, 6.0, 5.0],
        [49.0, 71.0],
        false,
    ),
];
pub(in crate::entity_models) const BREEZE_WIND_TOP: [ModelCube; 3] = [
    ModelCube::new(
        [-9.0, -8.0, -9.0],
        [18.0, 8.0, 18.0],
        BREEZE_SLATE,
        [18.0, 8.0, 18.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-6.0, -8.0, -6.0],
        [12.0, 8.0, 12.0],
        BREEZE_SLATE,
        [12.0, 8.0, 12.0],
        [6.0, 6.0],
        false,
    ),
    ModelCube::new(
        [-2.5, -8.0, -2.5],
        [5.0, 8.0, 5.0],
        BREEZE_SLATE,
        [5.0, 8.0, 5.0],
        [105.0, 57.0],
        false,
    ),
];

pub(in crate::entity_models) const BREEZE_WIND_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BREEZE_WIND_BOTTOM_POSE: PartPose = PartPose {
    offset: [0.0, 24.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BREEZE_WIND_MID_POSE: PartPose = PartPose {
    offset: [0.0, -7.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BREEZE_WIND_TOP_POSE: PartPose = PartPose {
    offset: [0.0, -6.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

// Vanilla 26.1 `BreezeAnimation.IDLE` (length 2.0s, looping). The head bobs on a CATMULLROM position
// spline; the rods spin a full `1080°` of yaw per cycle (LINEAR) while bobbing on a LINEAR position
// spline. The `wind_top` / `wind_mid` pivots sway on their own LINEAR position splines; those bones
// live only in the deferred wind layer ([`BreezeWindModel`]), so the base body's `apply_breeze_anim`
// (which samples `body`/`head`/`rods`) ignores them while the wind layer picks them up.
const BREEZE_IDLE_WIND_TOP_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.5, 0.0, 0.0), Linear),
    keyframe(0.25, pos_vec(0.5, 0.0, -0.5), Linear),
    keyframe(0.75, pos_vec(-0.5, 0.0, -0.5), Linear),
    keyframe(1.25, pos_vec(-0.5, 0.0, 0.5), Linear),
    keyframe(1.75, pos_vec(0.5, 0.0, 0.5), Linear),
    keyframe(2.0, pos_vec(0.5, 0.0, 0.0), Linear),
];
const BREEZE_IDLE_WIND_MID_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.5, 0.0, -0.5), Linear),
    keyframe(0.5, pos_vec(-0.5, 0.0, -0.5), Linear),
    keyframe(1.0, pos_vec(-0.5, 0.0, 0.5), Linear),
    keyframe(1.5, pos_vec(0.5, 0.0, 0.5), Linear),
    keyframe(2.0, pos_vec(0.5, 0.0, -0.5), Linear),
];
const BREEZE_IDLE_HEAD_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CatmullRom),
    keyframe(1.0, pos_vec(0.0, 1.0, 0.0), CatmullRom),
    keyframe(2.0, pos_vec(0.0, 0.0, 0.0), CatmullRom),
];
const BREEZE_IDLE_RODS_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), Linear),
    keyframe(2.0, degree_vec(0.0, 1080.0, 0.0), Linear),
];
const BREEZE_IDLE_RODS_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), Linear),
    keyframe(1.0, pos_vec(0.0, -1.0, 0.0), Linear),
    keyframe(2.0, pos_vec(0.0, 0.0, 0.0), Linear),
];

const BREEZE_IDLE_BONES: [BoneAnimation; 4] = [
    BoneAnimation {
        bone: "wind_top",
        channels: &[AnimationChannel {
            target: AnimationTarget::Position,
            keyframes: &BREEZE_IDLE_WIND_TOP_POS,
        }],
    },
    BoneAnimation {
        bone: "wind_mid",
        channels: &[AnimationChannel {
            target: AnimationTarget::Position,
            keyframes: &BREEZE_IDLE_WIND_MID_POS,
        }],
    },
    BoneAnimation {
        bone: "head",
        channels: &[AnimationChannel {
            target: AnimationTarget::Position,
            keyframes: &BREEZE_IDLE_HEAD_POS,
        }],
    },
    BoneAnimation {
        bone: "rods",
        channels: &[
            AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &BREEZE_IDLE_RODS_ROT,
            },
            AnimationChannel {
                target: AnimationTarget::Position,
                keyframes: &BREEZE_IDLE_RODS_POS,
            },
        ],
    },
];

pub(in crate::entity_models) const BREEZE_IDLE: AnimationDefinition = AnimationDefinition {
    length_seconds: 2.0,
    looping: true,
    bones: &BREEZE_IDLE_BONES,
};

// ----- `BreezeAnimation.SHOOT` (length 1.125s, NOT looping). All keyframes LINEAR. The
// body-layer bones (`body`/`head`/`rods`) are applied here; the `wind_*` channels target the
// deferred wind-body layer's parts (absent from this model, so `sample_bone_offsets` skips them). -----
const BREEZE_SHOOT_HEAD_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(-12.5, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(-12.5, 0.0, 0.0), LINEAR),
    keyframe(0.9167, degree_vec(5.0, 0.0, 0.0), LINEAR),
    keyframe(1.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const BREEZE_SHOOT_HEAD_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, pos_vec(0.0, -2.0, 0.0), LINEAR),
    keyframe(0.7917, pos_vec(0.0, -1.0, 2.0), LINEAR),
    keyframe(0.9583, pos_vec(0.0, -1.0, 0.0), LINEAR),
    keyframe(1.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const BREEZE_SHOOT_WIND_BOTTOM_ROT: [Keyframe; 1] =
    [keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR)];
const BREEZE_SHOOT_WIND_MID_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(12.5, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(12.5, 0.0, 0.0), LINEAR),
    keyframe(0.9167, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(1.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const BREEZE_SHOOT_WIND_MID_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, 5.0), LINEAR),
    keyframe(0.75, pos_vec(0.0, 0.0, 6.0), LINEAR),
    keyframe(0.9167, pos_vec(0.0, 0.0, -2.0), LINEAR),
    keyframe(1.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const BREEZE_SHOOT_WIND_TOP_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(15.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(15.0, 0.0, 0.0), LINEAR),
    keyframe(0.9167, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(1.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const BREEZE_SHOOT_WIND_TOP_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, 3.0), LINEAR),
    keyframe(0.8333, pos_vec(0.0, 0.0, 4.0), LINEAR),
    keyframe(0.9583, pos_vec(0.0, 0.0, -2.0), LINEAR),
    keyframe(1.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const BREEZE_SHOOT_BODY_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(12.5, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(12.5, 0.0, 0.0), LINEAR),
    keyframe(0.9167, degree_vec(-2.5, 0.0, 0.0), LINEAR),
    keyframe(1.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const BREEZE_SHOOT_BODY_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, pos_vec(0.0, 3.0, 5.0), LINEAR),
    keyframe(0.8333, pos_vec(0.0, 3.0, 6.0), LINEAR),
    keyframe(0.9583, pos_vec(0.0, 3.0, -1.0), LINEAR),
    keyframe(1.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const BREEZE_SHOOT_RODS_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(0.0, 360.0, 0.0), LINEAR),
];
const BREEZE_SHOOT_HEAD_CHANNELS: [AnimationChannel; 2] =
    [rot(&BREEZE_SHOOT_HEAD_ROT), pos(&BREEZE_SHOOT_HEAD_POS)];
const BREEZE_SHOOT_WIND_BOTTOM_CHANNELS: [AnimationChannel; 1] =
    [rot(&BREEZE_SHOOT_WIND_BOTTOM_ROT)];
const BREEZE_SHOOT_WIND_MID_CHANNELS: [AnimationChannel; 2] = [
    rot(&BREEZE_SHOOT_WIND_MID_ROT),
    pos(&BREEZE_SHOOT_WIND_MID_POS),
];
const BREEZE_SHOOT_WIND_TOP_CHANNELS: [AnimationChannel; 2] = [
    rot(&BREEZE_SHOOT_WIND_TOP_ROT),
    pos(&BREEZE_SHOOT_WIND_TOP_POS),
];
const BREEZE_SHOOT_BODY_CHANNELS: [AnimationChannel; 2] =
    [rot(&BREEZE_SHOOT_BODY_ROT), pos(&BREEZE_SHOOT_BODY_POS)];
const BREEZE_SHOOT_RODS_CHANNELS: [AnimationChannel; 1] = [rot(&BREEZE_SHOOT_RODS_ROT)];
const BREEZE_SHOOT_BONES: [BoneAnimation; 6] = [
    BoneAnimation {
        bone: "head",
        channels: &BREEZE_SHOOT_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "wind_bottom",
        channels: &BREEZE_SHOOT_WIND_BOTTOM_CHANNELS,
    },
    BoneAnimation {
        bone: "wind_mid",
        channels: &BREEZE_SHOOT_WIND_MID_CHANNELS,
    },
    BoneAnimation {
        bone: "wind_top",
        channels: &BREEZE_SHOOT_WIND_TOP_CHANNELS,
    },
    BoneAnimation {
        bone: "body",
        channels: &BREEZE_SHOOT_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "rods",
        channels: &BREEZE_SHOOT_RODS_CHANNELS,
    },
];

/// Vanilla `BreezeAnimation.SHOOT`: applied additively by `BreezeModel.setupAnim` over the
/// projected `breeze_shoot_seconds` (a non-looping one-shot held at its final frame past the end).
pub(in crate::entity_models) const BREEZE_SHOOT: AnimationDefinition = AnimationDefinition {
    length_seconds: 1.125,
    looping: false,
    bones: &BREEZE_SHOOT_BONES,
};

// ----- `BreezeAnimation.JUMP` (length 0.5s, NOT looping). All keyframes LINEAR. The
// body-layer bones (`body`/`head`/`rods`) are applied here; the `wind_*` channels target the
// deferred wind-body layer's parts (absent from this model, so `sample_bone_offsets` skips them). -----
const BREEZE_JUMP_BODY_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.0, -10.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 11.0, 0.0), LINEAR),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const BREEZE_JUMP_HEAD_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(22.5, 0.0, 0.0), LINEAR),
    keyframe(0.2083, degree_vec(-19.25, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const BREEZE_JUMP_WIND_BODY_SCALE: [Keyframe; 3] = [
    keyframe(0.0, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(0.125, scale_vec(1.0, 1.3, 1.0), LINEAR),
    keyframe(0.5, scale_vec(1.0, 1.0, 1.0), LINEAR),
];
const BREEZE_JUMP_WIND_BOTTOM_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(0.0, 90.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 360.0, 0.0), LINEAR),
];
const BREEZE_JUMP_WIND_BOTTOM_SCALE: [Keyframe; 3] = [
    keyframe(0.0, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(0.125, scale_vec(1.0, 1.1, 1.0), LINEAR),
    keyframe(0.5, scale_vec(1.0, 1.0, 1.0), LINEAR),
];
const BREEZE_JUMP_WIND_MID_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 180.0, 0.0), LINEAR),
];
const BREEZE_JUMP_WIND_MID_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.0, -6.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 2.0, 0.0), LINEAR),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const BREEZE_JUMP_WIND_TOP_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 90.0, 0.0), LINEAR),
];
const BREEZE_JUMP_WIND_TOP_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.0, -5.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 2.0, 0.0), LINEAR),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const BREEZE_JUMP_RODS_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 360.0, 0.0), LINEAR),
];
const BREEZE_JUMP_BODY_CHANNELS: [AnimationChannel; 1] = [pos(&BREEZE_JUMP_BODY_POS)];
const BREEZE_JUMP_HEAD_CHANNELS: [AnimationChannel; 1] = [rot(&BREEZE_JUMP_HEAD_ROT)];
const BREEZE_JUMP_WIND_BODY_CHANNELS: [AnimationChannel; 1] =
    [scale_channel(&BREEZE_JUMP_WIND_BODY_SCALE)];
const BREEZE_JUMP_WIND_BOTTOM_CHANNELS: [AnimationChannel; 2] = [
    rot(&BREEZE_JUMP_WIND_BOTTOM_ROT),
    scale_channel(&BREEZE_JUMP_WIND_BOTTOM_SCALE),
];
const BREEZE_JUMP_WIND_MID_CHANNELS: [AnimationChannel; 2] = [
    rot(&BREEZE_JUMP_WIND_MID_ROT),
    pos(&BREEZE_JUMP_WIND_MID_POS),
];
const BREEZE_JUMP_WIND_TOP_CHANNELS: [AnimationChannel; 2] = [
    rot(&BREEZE_JUMP_WIND_TOP_ROT),
    pos(&BREEZE_JUMP_WIND_TOP_POS),
];
const BREEZE_JUMP_RODS_CHANNELS: [AnimationChannel; 1] = [rot(&BREEZE_JUMP_RODS_ROT)];
const BREEZE_JUMP_BONES: [BoneAnimation; 7] = [
    BoneAnimation {
        bone: "body",
        channels: &BREEZE_JUMP_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &BREEZE_JUMP_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "wind_body",
        channels: &BREEZE_JUMP_WIND_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "wind_bottom",
        channels: &BREEZE_JUMP_WIND_BOTTOM_CHANNELS,
    },
    BoneAnimation {
        bone: "wind_mid",
        channels: &BREEZE_JUMP_WIND_MID_CHANNELS,
    },
    BoneAnimation {
        bone: "wind_top",
        channels: &BREEZE_JUMP_WIND_TOP_CHANNELS,
    },
    BoneAnimation {
        bone: "rods",
        channels: &BREEZE_JUMP_RODS_CHANNELS,
    },
];

/// Vanilla `BreezeAnimation.JUMP`: applied additively by `BreezeModel.setupAnim` over the
/// projected `breeze_long_jump_seconds` (a non-looping one-shot held at its final frame past the end).
pub(in crate::entity_models) const BREEZE_JUMP: AnimationDefinition = AnimationDefinition {
    length_seconds: 0.5,
    looping: false,
    bones: &BREEZE_JUMP_BONES,
};

// ----- `BreezeAnimation.INHALE` (length 2.0s, NOT looping). All keyframes LINEAR. The
// body-layer bones (`body`/`head`/`rods`) are applied here; the `wind_*` channels target the
// deferred wind-body layer's parts (absent from this model, so `sample_bone_offsets` skips them). -----
const BREEZE_INHALE_BODY_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, pos_vec(0.0, -10.0, 0.0), LINEAR),
    keyframe(0.625, pos_vec(0.0, -10.0, 0.0), LINEAR),
];
const BREEZE_INHALE_HEAD_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(22.5, 0.0, 0.0), LINEAR),
    keyframe(0.625, degree_vec(22.5, 0.0, 0.0), LINEAR),
];
const BREEZE_INHALE_WIND_BODY_SCALE: [Keyframe; 3] = [
    keyframe(0.0, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(0.5, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(0.625, scale_vec(1.0, 1.0, 1.0), LINEAR),
];
const BREEZE_INHALE_WIND_BOTTOM_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.625, degree_vec(0.0, 90.0, 0.0), LINEAR),
];
const BREEZE_INHALE_WIND_BOTTOM_SCALE: [Keyframe; 3] = [
    keyframe(0.0, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(0.5, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(0.625, scale_vec(1.0, 1.0, 1.0), LINEAR),
];
const BREEZE_INHALE_WIND_MID_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.625, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const BREEZE_INHALE_WIND_MID_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, pos_vec(0.0, -6.0, 0.0), LINEAR),
    keyframe(0.625, pos_vec(0.0, -6.0, 0.0), LINEAR),
];
const BREEZE_INHALE_WIND_TOP_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.625, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const BREEZE_INHALE_WIND_TOP_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, pos_vec(0.0, -5.0, 0.0), LINEAR),
    keyframe(0.625, pos_vec(0.0, -5.0, 0.0), LINEAR),
];
const BREEZE_INHALE_RODS_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.625, degree_vec(0.0, 360.0, 0.0), LINEAR),
];
const BREEZE_INHALE_BODY_CHANNELS: [AnimationChannel; 1] = [pos(&BREEZE_INHALE_BODY_POS)];
const BREEZE_INHALE_HEAD_CHANNELS: [AnimationChannel; 1] = [rot(&BREEZE_INHALE_HEAD_ROT)];
const BREEZE_INHALE_WIND_BODY_CHANNELS: [AnimationChannel; 1] =
    [scale_channel(&BREEZE_INHALE_WIND_BODY_SCALE)];
const BREEZE_INHALE_WIND_BOTTOM_CHANNELS: [AnimationChannel; 2] = [
    rot(&BREEZE_INHALE_WIND_BOTTOM_ROT),
    scale_channel(&BREEZE_INHALE_WIND_BOTTOM_SCALE),
];
const BREEZE_INHALE_WIND_MID_CHANNELS: [AnimationChannel; 2] = [
    rot(&BREEZE_INHALE_WIND_MID_ROT),
    pos(&BREEZE_INHALE_WIND_MID_POS),
];
const BREEZE_INHALE_WIND_TOP_CHANNELS: [AnimationChannel; 2] = [
    rot(&BREEZE_INHALE_WIND_TOP_ROT),
    pos(&BREEZE_INHALE_WIND_TOP_POS),
];
const BREEZE_INHALE_RODS_CHANNELS: [AnimationChannel; 1] = [rot(&BREEZE_INHALE_RODS_ROT)];
const BREEZE_INHALE_BONES: [BoneAnimation; 7] = [
    BoneAnimation {
        bone: "body",
        channels: &BREEZE_INHALE_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &BREEZE_INHALE_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "wind_body",
        channels: &BREEZE_INHALE_WIND_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "wind_bottom",
        channels: &BREEZE_INHALE_WIND_BOTTOM_CHANNELS,
    },
    BoneAnimation {
        bone: "wind_mid",
        channels: &BREEZE_INHALE_WIND_MID_CHANNELS,
    },
    BoneAnimation {
        bone: "wind_top",
        channels: &BREEZE_INHALE_WIND_TOP_CHANNELS,
    },
    BoneAnimation {
        bone: "rods",
        channels: &BREEZE_INHALE_RODS_CHANNELS,
    },
];

/// Vanilla `BreezeAnimation.INHALE`: applied additively by `BreezeModel.setupAnim` over the
/// projected `breeze_inhale_seconds` (a non-looping one-shot held at its final frame past the end).
pub(in crate::entity_models) const BREEZE_INHALE: AnimationDefinition = AnimationDefinition {
    length_seconds: 2.0,
    looping: false,
    bones: &BREEZE_INHALE_BONES,
};

// ----- `BreezeAnimation.SLIDE` (length 0.2s, NOT looping). All keyframes LINEAR. The
// body-layer bones (`body`/`head`/`rods`) are applied here; the `wind_*` channels target the
// deferred wind-body layer's parts (absent from this model, so `sample_bone_offsets` skips them). -----
const BREEZE_SLIDE_BODY_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2, pos_vec(0.0, 0.0, -6.0), LINEAR),
];
const BREEZE_SLIDE_WIND_MID_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2, pos_vec(0.0, 0.0, -3.0), LINEAR),
];
const BREEZE_SLIDE_WIND_TOP_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2, pos_vec(0.0, 0.0, -2.0), LINEAR),
];
const BREEZE_SLIDE_BODY_CHANNELS: [AnimationChannel; 1] = [pos(&BREEZE_SLIDE_BODY_POS)];
const BREEZE_SLIDE_WIND_MID_CHANNELS: [AnimationChannel; 1] = [pos(&BREEZE_SLIDE_WIND_MID_POS)];
const BREEZE_SLIDE_WIND_TOP_CHANNELS: [AnimationChannel; 1] = [pos(&BREEZE_SLIDE_WIND_TOP_POS)];
const BREEZE_SLIDE_BONES: [BoneAnimation; 3] = [
    BoneAnimation {
        bone: "body",
        channels: &BREEZE_SLIDE_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "wind_mid",
        channels: &BREEZE_SLIDE_WIND_MID_CHANNELS,
    },
    BoneAnimation {
        bone: "wind_top",
        channels: &BREEZE_SLIDE_WIND_TOP_CHANNELS,
    },
];

/// Vanilla `BreezeAnimation.SLIDE`: applied additively by `BreezeModel.setupAnim` over the
/// projected `breeze_slide_seconds` (a non-looping one-shot held at its final frame past the end).
pub(in crate::entity_models) const BREEZE_SLIDE: AnimationDefinition = AnimationDefinition {
    length_seconds: 0.2,
    looping: false,
    bones: &BREEZE_SLIDE_BONES,
};

// ----- `BreezeAnimation.SLIDE_BACK` (length 0.1s, NOT looping). All keyframes LINEAR. The
// body-layer bones (`body`/`head`/`rods`) are applied here; the `wind_*` channels target the
// deferred wind-body layer's parts (absent from this model, so `sample_bone_offsets` skips them). -----
const BREEZE_SLIDE_BACK_BODY_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, -6.0), LINEAR),
    keyframe(0.1, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const BREEZE_SLIDE_BACK_WIND_MID_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, -3.0), LINEAR),
    keyframe(0.1, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const BREEZE_SLIDE_BACK_WIND_TOP_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, -2.0), LINEAR),
    keyframe(0.1, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const BREEZE_SLIDE_BACK_BODY_CHANNELS: [AnimationChannel; 1] = [pos(&BREEZE_SLIDE_BACK_BODY_POS)];
const BREEZE_SLIDE_BACK_WIND_MID_CHANNELS: [AnimationChannel; 1] =
    [pos(&BREEZE_SLIDE_BACK_WIND_MID_POS)];
const BREEZE_SLIDE_BACK_WIND_TOP_CHANNELS: [AnimationChannel; 1] =
    [pos(&BREEZE_SLIDE_BACK_WIND_TOP_POS)];
const BREEZE_SLIDE_BACK_BONES: [BoneAnimation; 3] = [
    BoneAnimation {
        bone: "body",
        channels: &BREEZE_SLIDE_BACK_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "wind_mid",
        channels: &BREEZE_SLIDE_BACK_WIND_MID_CHANNELS,
    },
    BoneAnimation {
        bone: "wind_top",
        channels: &BREEZE_SLIDE_BACK_WIND_TOP_CHANNELS,
    },
];

/// Vanilla `BreezeAnimation.SLIDE_BACK`: applied additively by `BreezeModel.setupAnim` over the
/// projected `breeze_slide_back_seconds` (a non-looping one-shot held at its final frame past the end).
pub(in crate::entity_models) const BREEZE_SLIDE_BACK: AnimationDefinition = AnimationDefinition {
    length_seconds: 0.1,
    looping: false,
    bones: &BREEZE_SLIDE_BACK_BONES,
};

/// Applies the vanilla `BreezeModel.setupAnim` animation stack to the base body layer's bones
/// (`body`/`head`/`rods`): the looping `IDLE` (the `head` bobs on its CATMULLROM position spline and
/// the `rods` pivot spins 1080°/cycle while bobbing, sampled from `ageInTicks`; the `body` has no IDLE
/// channel) plus the pose-driven action one-shots `SHOOT`/`SLIDE`/`SLIDE_BACK`/`INHALE`/`JUMP`, each
/// ADDED on top in vanilla `setupAnim` order over its projected elapsed seconds (the non-looping
/// actions clamp past their length to the final frame). The actions' `wind_*` channels target the
/// deferred wind layer's parts and are skipped (those bones are absent). The emissive eyes and the
/// translucent wind body stay deferred entity-side state.
fn apply_breeze_anim(root: &mut ModelPart, instance: &EntityModelInstance) {
    let idle_seconds =
        keyframe_elapsed_seconds(&BREEZE_IDLE, instance.render_state.age_in_ticks * 0.05);
    // The action one-shots in vanilla `setupAnim` order (shoot → slide → slideBack → inhale → jump),
    // each applied only while its projected elapsed seconds is `>= 0`.
    let actions: [(&AnimationDefinition, f32); 5] = [
        (&BREEZE_SHOOT, instance.render_state.breeze_shoot_seconds),
        (&BREEZE_SLIDE, instance.render_state.breeze_slide_seconds),
        (
            &BREEZE_SLIDE_BACK,
            instance.render_state.breeze_slide_back_seconds,
        ),
        (&BREEZE_INHALE, instance.render_state.breeze_inhale_seconds),
        (&BREEZE_JUMP, instance.render_state.breeze_long_jump_seconds),
    ];
    // Each bone: bind + the looping idle offset + every active action offset, added in order. The
    // idle has no `body` channel (returns the identity offset), so the `body` pivot takes only the
    // actions.
    let apply = |part: &mut ModelPart, bind: PartPose, bone: &str| {
        let (idle_pos, idle_rot) = sample_bone_offsets(&BREEZE_IDLE, bone, idle_seconds, 1.0);
        let mut pose = keyframe_animated_pose(bind, idle_pos, idle_rot);
        for (definition, seconds) in actions {
            if seconds < 0.0 {
                continue;
            }
            let sample = keyframe_elapsed_seconds(definition, seconds);
            let (position, rotation) = sample_bone_offsets(definition, bone, sample, 1.0);
            pose = keyframe_animated_pose(pose, position, rotation);
        }
        part.pose = pose;
    };

    let body = root.child_mut("body");
    apply(body, BREEZE_BODY_POSE, "body");
    apply(body.child_mut("head"), BREEZE_HEAD_POSE, "head");
    apply(body.child_mut("rods"), BREEZE_RODS_POSE, "rods");
}

/// Mutable breeze model, mirroring vanilla `BreezeModel`'s base body layer. A synthetic root holds the
/// `body` pivot → (`head`, `rods` pivot → the three rods at their fixed bind poses); each cube carries
/// both the colored tint and the textured UV, so one tree drives both render paths. `setup_anim` runs
/// [`apply_breeze_anim`] (idle + the pose-driven action one-shots). The translucent wind body and the
/// emissive eyes layers stay deferred.
pub(in crate::entity_models) struct BreezeModel {
    root: ModelPart,
}

impl BreezeModel {
    pub(in crate::entity_models) fn new() -> Self {
        let rods = ModelPart::new(
            BREEZE_RODS_POSE,
            Vec::new(),
            vec![
                (
                    "rod_1",
                    ModelPart::leaf(BREEZE_ROD_1_POSE, BREEZE_ROD.to_vec()),
                ),
                (
                    "rod_2",
                    ModelPart::leaf(BREEZE_ROD_2_POSE, BREEZE_ROD.to_vec()),
                ),
                (
                    "rod_3",
                    ModelPart::leaf(BREEZE_ROD_3_POSE, BREEZE_ROD.to_vec()),
                ),
            ],
        );
        let body = ModelPart::new(
            BREEZE_BODY_POSE,
            Vec::new(),
            vec![
                (
                    "head",
                    ModelPart::leaf(BREEZE_HEAD_POSE, BREEZE_HEAD.to_vec()),
                ),
                ("rods", rods),
            ],
        );
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![("body", body)]),
        }
    }
}

impl EntityModel for BreezeModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_breeze_anim(&mut self.root, instance);
    }
}

/// Applies the vanilla `BreezeModel.setupAnim` stack to the SEPARATE wind layer's bones
/// (`wind_body`/`wind_bottom`/`wind_mid`/`wind_top`): the looping `IDLE` (the `wind_top` / `wind_mid`
/// pivots sway on their LINEAR position splines; `wind_bottom` / `wind_body` have no idle channel)
/// plus the pose-driven action one-shots `SHOOT`/`SLIDE`/`SLIDE_BACK`/`INHALE`/`JUMP`, each ADDED on
/// top in vanilla `setupAnim` order over its projected elapsed seconds. The actions carry the wind
/// pivots' rotation/position swirls and the `JUMP`/`INHALE` `wind_body` / `wind_bottom` `SCALE`
/// pulses, folded onto the part's reset `[1, 1, 1]` scale (vanilla `ModelPart.offsetScale`). The
/// actions' `body`/`head`/`rods` channels target the base body layer's parts and are skipped here.
fn apply_breeze_wind_anim(root: &mut ModelPart, instance: &EntityModelInstance) {
    let idle_seconds =
        keyframe_elapsed_seconds(&BREEZE_IDLE, instance.render_state.age_in_ticks * 0.05);
    let actions: [(&AnimationDefinition, f32); 5] = [
        (&BREEZE_SHOOT, instance.render_state.breeze_shoot_seconds),
        (&BREEZE_SLIDE, instance.render_state.breeze_slide_seconds),
        (
            &BREEZE_SLIDE_BACK,
            instance.render_state.breeze_slide_back_seconds,
        ),
        (&BREEZE_INHALE, instance.render_state.breeze_inhale_seconds),
        (&BREEZE_JUMP, instance.render_state.breeze_long_jump_seconds),
    ];
    // Each bone: bind + the looping idle offset + every active action offset/scale, added in order.
    // The idle carries no `wind_body` / `wind_bottom` channel (identity offset), and no wind bone has
    // an idle scale channel, so the idle leaves the scale at the reset `[1, 1, 1]`.
    let apply = |part: &mut ModelPart, bind: PartPose, bone: &str| {
        let (idle_pos, idle_rot) = sample_bone_offsets(&BREEZE_IDLE, bone, idle_seconds, 1.0);
        part.pose = keyframe_animated_pose(bind, idle_pos, idle_rot);
        for (definition, seconds) in actions {
            if seconds < 0.0 {
                continue;
            }
            let sample = keyframe_elapsed_seconds(definition, seconds);
            let (position, rotation, scale_offset) =
                sample_bone_offsets_with_scale(definition, bone, sample, 1.0);
            part.pose = keyframe_animated_pose(part.pose, position, rotation);
            part.scale = [
                part.scale[0] + scale_offset[0],
                part.scale[1] + scale_offset[1],
                part.scale[2] + scale_offset[2],
            ];
        }
    };

    let wind_body = root.child_mut("wind_body");
    apply(wind_body, BREEZE_WIND_BODY_POSE, "wind_body");
    let wind_bottom = wind_body.child_mut("wind_bottom");
    apply(wind_bottom, BREEZE_WIND_BOTTOM_POSE, "wind_bottom");
    let wind_mid = wind_bottom.child_mut("wind_mid");
    apply(wind_mid, BREEZE_WIND_MID_POSE, "wind_mid");
    let wind_top = wind_mid.child_mut("wind_top");
    apply(wind_top, BREEZE_WIND_TOP_POSE, "wind_top");
}

/// Mutable breeze WIND model, mirroring vanilla `BreezeModel.createWindLayer` / `BreezeWindLayer`: a
/// synthetic root holds the `wind_body` pivot → `wind_bottom` → `wind_mid` → `wind_top` shell chain
/// (atlas 128×128, `breeze_wind.png`). It is a SEPARATE model from the base body so the two layers can
/// carry different atlas sizes; `setup_anim` runs [`apply_breeze_wind_anim`] (idle sway + the
/// pose-driven action swirls/pulses). It is rendered only on the textured scrolling-overlay path
/// (vanilla `RenderTypes.breezeWind`, the U coordinate scrolled by `ageInTicks · 0.02`); the colored
/// debug path keeps the base body only.
pub(in crate::entity_models) struct BreezeWindModel {
    root: ModelPart,
}

impl BreezeWindModel {
    pub(in crate::entity_models) fn new() -> Self {
        let wind_top = ModelPart::leaf(BREEZE_WIND_TOP_POSE, BREEZE_WIND_TOP.to_vec());
        let wind_mid = ModelPart::new(
            BREEZE_WIND_MID_POSE,
            BREEZE_WIND_MID.to_vec(),
            vec![("wind_top", wind_top)],
        );
        let wind_bottom = ModelPart::new(
            BREEZE_WIND_BOTTOM_POSE,
            BREEZE_WIND_BOTTOM.to_vec(),
            vec![("wind_mid", wind_mid)],
        );
        let wind_body = ModelPart::new(
            BREEZE_WIND_BODY_POSE,
            Vec::new(),
            vec![("wind_bottom", wind_bottom)],
        );
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![("wind_body", wind_body)]),
        }
    }
}

impl EntityModel for BreezeWindModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_breeze_wind_anim(&mut self.root, instance);
    }
}
