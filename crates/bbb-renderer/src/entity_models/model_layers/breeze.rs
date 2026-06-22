use super::{
    degree_vec, keyframe, pos_vec, AnimationChannel, AnimationDefinition, AnimationTarget,
    BoneAnimation, Keyframe, KeyframeInterpolation, ModelCubeDesc, PartPose, TexturedModelCubeDesc,
    BREEZE_SLATE,
};

use KeyframeInterpolation::{CatmullRom, Linear};

// Vanilla 26.1 `BreezeModel.createBodyLayer` (atlas 32×32): the base body layer retains only the
// `head` (with its emissive `eyes` child) and the three `rods` under the `body` pivot; the swirling
// `wind_body` is a separate translucent layer. The colored path approximates the wind body's
// translucent blue with a single representative slate.
pub(in crate::entity_models) const BREEZE_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-5.0, -5.0, -4.2],
        size: [10.0, 3.0, 4.0],
        color: BREEZE_SLATE,
    },
    ModelCubeDesc {
        min: [-4.0, -8.0, -4.0],
        size: [8.0, 8.0, 8.0],
        color: BREEZE_SLATE,
    },
];

// All three rods share the same `texOffs(0, 17)` 2×8×2 box; only their bind pose differs.
pub(in crate::entity_models) const BREEZE_ROD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -3.0],
    size: [2.0, 8.0, 2.0],
    color: BREEZE_SLATE,
}];

// The same geometry with the vanilla `BreezeModel.createBaseMesh` texOffs UV coordinates (atlas
// 32×32); no `CubeDeformation`, so each `uv_size` matches its box `size`.
pub(in crate::entity_models) const BREEZE_TEXTURED_HEAD: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-5.0, -5.0, -4.2],
        size: [10.0, 3.0, 4.0],
        uv_size: [10.0, 3.0, 4.0],
        tex: [4.0, 24.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-4.0, -8.0, -4.0],
        size: [8.0, 8.0, 8.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BREEZE_TEXTURED_ROD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -3.0],
        size: [2.0, 8.0, 2.0],
        uv_size: [2.0, 8.0, 2.0],
        tex: [0.0, 17.0],
        mirror: false,
    }];

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

// Vanilla 26.1 `BreezeAnimation.IDLE` (length 2.0s, looping), restricted to the base body layer's
// bones. The head bobs on a CATMULLROM position spline; the rods spin a full `1080°` of yaw per
// cycle (LINEAR) while bobbing on a LINEAR position spline. The `wind_top` / `wind_mid` channels
// drive the deferred wind layer and are omitted here.
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

const BREEZE_IDLE_BONES: [BoneAnimation; 2] = [
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
