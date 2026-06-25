use super::super::keyframe::{
    degree_vec, keyframe, keyframe_animated_pose, keyframe_elapsed_seconds, keyframe_walk_sample,
    pos_vec, sample_bone_offsets, sample_bone_offsets_with_scale, scale_vec, AnimationChannel,
    AnimationDefinition, AnimationTarget, BoneAnimation, Keyframe, KeyframeInterpolation,
};
use super::{PartPose, CREAKING_BARK, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 `CreakingModel.createBodyLayer` (atlas 64×64). The mesh root holds one `root` part
// at `offset(0, 24, 0)` parenting `upper_body` and the two legs; `upper_body` (an empty pivot)
// parents the head (with its two antler/branch planes), the body, and the two arms. `setupAnim`
// sets `head.xRot/yRot` from the plain look (reproduced through the projected look angles) and, when
// `canMove` (the projected synced `CAN_MOVE`, default true), applies the looping
// `CreakingAnimation.CREAKING_WALK` ([`CREAKING_WALK`]) which offsets the upper body, head, arms,
// and legs. The head channel ADDS onto the look. A creaking frozen while observed (`canMove` false)
// holds the bind pose plus its look — it turns to a statue. The three triggered combat/death keyframe
// one-shots ([`CREAKING_ATTACK`] event-driven, [`CREAKING_INVULNERABLE`] event-driven,
// [`CREAKING_DEATH`] driven by the synced `isTearingDown()`) then apply additively over their
// projected elapsed seconds, exactly like vanilla `setupAnim`. The emissive eyes layer
// (`createEyesLayer`, the `head` part only) reuses the identical head UVs and is deferred.

// `head`: the 6×10×6 skull, the 6×3×6 brow, and two 9×14×0 antler/branch planes.
pub(in crate::entity_models) const CREAKING_HEAD_CUBES: [ModelCube; 4] = [
    ModelCube::new(
        [-3.0, -10.0, -3.0],
        [6.0, 10.0, 6.0],
        CREAKING_BARK,
        [6.0, 10.0, 6.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-3.0, -13.0, -3.0],
        [6.0, 3.0, 6.0],
        CREAKING_BARK,
        [6.0, 3.0, 6.0],
        [28.0, 31.0],
        false,
    ),
    ModelCube::new(
        [3.0, -13.0, 0.0],
        [9.0, 14.0, 0.0],
        CREAKING_BARK,
        [9.0, 14.0, 0.0],
        [12.0, 40.0],
        false,
    ),
    ModelCube::new(
        [-12.0, -14.0, 0.0],
        [9.0, 14.0, 0.0],
        CREAKING_BARK,
        [9.0, 14.0, 0.0],
        [34.0, 12.0],
        false,
    ),
];

// `body`: the 6×13×5 trunk plus the 6×7×5 upper block.
pub(in crate::entity_models) const CREAKING_BODY_CUBES: [ModelCube; 2] = [
    ModelCube::new(
        [0.0, -3.0, -3.0],
        [6.0, 13.0, 5.0],
        CREAKING_BARK,
        [6.0, 13.0, 5.0],
        [0.0, 16.0],
        false,
    ),
    ModelCube::new(
        [-6.0, -4.0, -3.0],
        [6.0, 7.0, 5.0],
        CREAKING_BARK,
        [6.0, 7.0, 5.0],
        [24.0, 0.0],
        false,
    ),
];

// `right_arm`: a 3×21×3 limb plus a 3×4×3 hand.
pub(in crate::entity_models) const CREAKING_RIGHT_ARM_CUBES: [ModelCube; 2] = [
    ModelCube::new(
        [-2.0, -1.5, -1.5],
        [3.0, 21.0, 3.0],
        CREAKING_BARK,
        [3.0, 21.0, 3.0],
        [22.0, 13.0],
        false,
    ),
    ModelCube::new(
        [-2.0, 19.5, -1.5],
        [3.0, 4.0, 3.0],
        CREAKING_BARK,
        [3.0, 4.0, 3.0],
        [46.0, 0.0],
        false,
    ),
];

// `left_arm`: a 3×16×3 limb with a 3×4×3 shoulder block and a 3×4×3 hand.
pub(in crate::entity_models) const CREAKING_LEFT_ARM_CUBES: [ModelCube; 3] = [
    ModelCube::new(
        [0.0, -1.0, -1.5],
        [3.0, 16.0, 3.0],
        CREAKING_BARK,
        [3.0, 16.0, 3.0],
        [30.0, 40.0],
        false,
    ),
    ModelCube::new(
        [0.0, -5.0, -1.5],
        [3.0, 4.0, 3.0],
        CREAKING_BARK,
        [3.0, 4.0, 3.0],
        [52.0, 12.0],
        false,
    ),
    ModelCube::new(
        [0.0, 15.0, -1.5],
        [3.0, 4.0, 3.0],
        CREAKING_BARK,
        [3.0, 4.0, 3.0],
        [52.0, 19.0],
        false,
    ),
];

// `left_leg`: a 3×16×3 limb plus a 5×0×9 foot plane.
pub(in crate::entity_models) const CREAKING_LEFT_LEG_CUBES: [ModelCube; 2] = [
    ModelCube::new(
        [-1.5, 0.0, -1.5],
        [3.0, 16.0, 3.0],
        CREAKING_BARK,
        [3.0, 16.0, 3.0],
        [42.0, 40.0],
        false,
    ),
    ModelCube::new(
        [-1.5, 15.7, -4.5],
        [5.0, 0.0, 9.0],
        CREAKING_BARK,
        [5.0, 0.0, 9.0],
        [45.0, 55.0],
        false,
    ),
];

// `right_leg`: a 3×19×3 limb, a 5×0×9 foot plane, and a 3×3×3 hip block.
pub(in crate::entity_models) const CREAKING_RIGHT_LEG_CUBES: [ModelCube; 3] = [
    ModelCube::new(
        [-3.0, -1.5, -1.5],
        [3.0, 19.0, 3.0],
        CREAKING_BARK,
        [3.0, 19.0, 3.0],
        [0.0, 34.0],
        false,
    ),
    ModelCube::new(
        [-5.0, 17.2, -4.5],
        [5.0, 0.0, 9.0],
        CREAKING_BARK,
        [5.0, 0.0, 9.0],
        [45.0, 46.0],
        false,
    ),
    ModelCube::new(
        [-3.0, -4.5, -1.5],
        [3.0, 3.0, 3.0],
        CREAKING_BARK,
        [3.0, 3.0, 3.0],
        [12.0, 34.0],
        false,
    ),
];

/// Vanilla `CreakingModel.createBodyLayer` rest-pose hierarchy, rooted at the `root` part. Sixteen
/// cubes. Each part's bind pose:
/// `root` part pose: `PartPose.offset(0, 24, 0)`.
pub(in crate::entity_models) const CREAKING_ROOT_POSE: PartPose = PartPose {
    offset: [0.0, 24.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `upper_body` empty pivot pose: `PartPose.offset(-1, -19, 0)`.
pub(in crate::entity_models) const CREAKING_UPPER_BODY_POSE: PartPose = PartPose {
    offset: [-1.0, -19.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `head` part pose: `PartPose.offset(-3, -11, 0)`.
pub(in crate::entity_models) const CREAKING_HEAD_POSE: PartPose = PartPose {
    offset: [-3.0, -11.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `body` part pose: `PartPose.offset(0, -7, 1)`.
pub(in crate::entity_models) const CREAKING_BODY_POSE: PartPose = PartPose {
    offset: [0.0, -7.0, 1.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_arm` part pose: `PartPose.offset(-7, -9.5, 1.5)`.
pub(in crate::entity_models) const CREAKING_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-7.0, -9.5, 1.5],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_arm` part pose: `PartPose.offset(6, -9, 0.5)`.
pub(in crate::entity_models) const CREAKING_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [6.0, -9.0, 0.5],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_leg` part pose: `PartPose.offset(1.5, -16, 0.5)`.
pub(in crate::entity_models) const CREAKING_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [1.5, -16.0, 0.5],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_leg` part pose: `PartPose.offset(-1, -17.5, 0.5)`.
pub(in crate::entity_models) const CREAKING_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-1.0, -17.5, 0.5],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds the creaking's named part tree: the cubeless `root` pivot parenting the cubeless
/// `upper_body` pivot (which parents `head`, `body`, `right_arm`, `left_arm`) and the two legs.
fn creaking_root() -> ModelPart {
    let upper_body = ModelPart::new(
        CREAKING_UPPER_BODY_POSE,
        Vec::new(),
        vec![
            (
                "head",
                ModelPart::leaf(CREAKING_HEAD_POSE, CREAKING_HEAD_CUBES.to_vec()),
            ),
            (
                "body",
                ModelPart::leaf(CREAKING_BODY_POSE, CREAKING_BODY_CUBES.to_vec()),
            ),
            (
                "right_arm",
                ModelPart::leaf(CREAKING_RIGHT_ARM_POSE, CREAKING_RIGHT_ARM_CUBES.to_vec()),
            ),
            (
                "left_arm",
                ModelPart::leaf(CREAKING_LEFT_ARM_POSE, CREAKING_LEFT_ARM_CUBES.to_vec()),
            ),
        ],
    );
    ModelPart::new(
        CREAKING_ROOT_POSE,
        Vec::new(),
        vec![
            ("upper_body", upper_body),
            (
                "left_leg",
                ModelPart::leaf(CREAKING_LEFT_LEG_POSE, CREAKING_LEFT_LEG_CUBES.to_vec()),
            ),
            (
                "right_leg",
                ModelPart::leaf(CREAKING_RIGHT_LEG_POSE, CREAKING_RIGHT_LEG_CUBES.to_vec()),
            ),
        ],
    )
}

// ----- `CreakingAnimation.CREAKING_WALK` (length 1.125s, looping). All keyframes are LINEAR. The
// animated bones map to the named part tree as: `upper_body`, `left_leg`, `right_leg` (the `root`
// children) and `head`, `right_arm`, `left_arm` (the `upper_body` children); the `body` is not
// animated. The `head` rotation channel adds onto the head look. -----

const LINEAR: KeyframeInterpolation = KeyframeInterpolation::Linear;

const CREAKING_WALK_UPPER_BODY_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(26.8802, -23.399, -9.0616), LINEAR),
    keyframe(0.125, degree_vec(-2.2093, 5.9119, 0.0675), LINEAR),
    keyframe(0.5417, degree_vec(23.0778, 14.2906, 4.6066), LINEAR),
    keyframe(0.7083, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(0.875, degree_vec(7.5, 0.0, 0.0), LINEAR),
    keyframe(1.125, degree_vec(26.8802, -23.399, -9.0616), LINEAR),
];
const CREAKING_WALK_HEAD_ROT: [Keyframe; 9] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.0417, degree_vec(-17.5, -62.5, 0.0), LINEAR),
    keyframe(0.0833, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.4167, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.4583, degree_vec(0.0, 15.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0417, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0833, degree_vec(-37.1532, 81.1131, -28.3621), LINEAR),
    keyframe(1.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_WALK_RIGHT_ARM_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(12.5, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(-32.0, 0.0, 0.0), LINEAR),
    keyframe(0.875, degree_vec(12.0, 0.0, 0.0), LINEAR),
    keyframe(1.125, degree_vec(-15.0, 0.0, 0.0), LINEAR),
];
const CREAKING_WALK_LEFT_ARM_ROT: [Keyframe; 8] = [
    keyframe(0.0, degree_vec(-15.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(10.0, 0.0, 0.0), LINEAR),
    keyframe(0.5417, degree_vec(-25.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(-9.0923, 0.0, 0.0), LINEAR),
    keyframe(0.7917, degree_vec(-15.137, -66.7758, 13.9603), LINEAR),
    keyframe(0.8333, degree_vec(-9.0923, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(10.0, 0.0, 0.0), LINEAR),
    keyframe(1.125, degree_vec(-15.0, 0.0, 0.0), LINEAR),
];
const CREAKING_WALK_LEFT_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(30.0, 0.0, 0.0), LINEAR),
    keyframe(0.375, degree_vec(49.8924, -3.8282, 3.2187), LINEAR),
    keyframe(0.5, degree_vec(17.5, 0.0, 0.0), LINEAR),
    keyframe(0.625, degree_vec(-56.5613, -12.2403, -8.7374), LINEAR),
    keyframe(0.9167, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_WALK_LEFT_LEG_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 2.0), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.1846, 0.5979), LINEAR),
    keyframe(0.375, pos_vec(0.0, -0.0665, -2.2177), LINEAR),
    keyframe(0.5, pos_vec(0.0, 1.3563, -4.3474), LINEAR),
    keyframe(0.625, pos_vec(0.0, 0.1047, -1.6556), LINEAR),
    keyframe(0.9167, pos_vec(0.0, 0.0, -1.0), LINEAR),
    keyframe(1.125, pos_vec(0.0, 0.0, 2.0), LINEAR),
];
const CREAKING_WALK_RIGHT_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(25.5305, 11.3125, 5.3525), LINEAR),
    keyframe(0.125, degree_vec(-49.5628, 7.3556, 6.7933), LINEAR),
    keyframe(0.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.4583, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.9167, degree_vec(30.0, 0.0, 0.0), LINEAR),
    keyframe(1.0417, degree_vec(55.0, 0.0, 0.0), LINEAR),
    keyframe(1.125, degree_vec(25.5305, 11.3125, 5.3525), LINEAR),
];
const CREAKING_WALK_RIGHT_LEG_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 0.9674, -3.6578), LINEAR),
    keyframe(0.125, pos_vec(0.0, -0.2979, -0.9411), LINEAR),
    keyframe(0.25, pos_vec(0.0, -0.3, -0.94), LINEAR),
    keyframe(0.4583, pos_vec(0.0, -0.3, 1.06), LINEAR),
    keyframe(1.125, pos_vec(0.0, 0.9674, -3.6578), LINEAR),
];

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

const CREAKING_WALK_UPPER_BODY_CHANNELS: [AnimationChannel; 1] =
    [rot(&CREAKING_WALK_UPPER_BODY_ROT)];
const CREAKING_WALK_HEAD_CHANNELS: [AnimationChannel; 1] = [rot(&CREAKING_WALK_HEAD_ROT)];
const CREAKING_WALK_RIGHT_ARM_CHANNELS: [AnimationChannel; 1] = [rot(&CREAKING_WALK_RIGHT_ARM_ROT)];
const CREAKING_WALK_LEFT_ARM_CHANNELS: [AnimationChannel; 1] = [rot(&CREAKING_WALK_LEFT_ARM_ROT)];
const CREAKING_WALK_LEFT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CREAKING_WALK_LEFT_LEG_ROT),
    pos(&CREAKING_WALK_LEFT_LEG_POS),
];
const CREAKING_WALK_RIGHT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CREAKING_WALK_RIGHT_LEG_ROT),
    pos(&CREAKING_WALK_RIGHT_LEG_POS),
];

const CREAKING_WALK_BONES: [BoneAnimation; 6] = [
    BoneAnimation {
        bone: "upper_body",
        channels: &CREAKING_WALK_UPPER_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &CREAKING_WALK_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_arm",
        channels: &CREAKING_WALK_RIGHT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_arm",
        channels: &CREAKING_WALK_LEFT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_leg",
        channels: &CREAKING_WALK_LEFT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_leg",
        channels: &CREAKING_WALK_RIGHT_LEG_CHANNELS,
    },
];

/// Vanilla `CreakingAnimation.CREAKING_WALK`: the looping 1.125s walk cycle, sampled by
/// `CreakingModel.setupAnim` via `applyWalk(walkAnimationPos, walkAnimationSpeed, 1.0, 1.0)` while
/// `canMove`. The `head` channel adds onto the look the head already tracks.
pub(in crate::entity_models) const CREAKING_WALK: AnimationDefinition = AnimationDefinition {
    length_seconds: 1.125,
    looping: true,
    bones: &CREAKING_WALK_BONES,
};

// ----- `CreakingAnimation.CREAKING_ATTACK` (length 0.7083s, looping). The lunge: the upper body,
// head (with a 1.3× y stretch at the strike), arms, and legs all swing; all keyframes are LINEAR. -----
const CREAKING_ATTACK_UPPER_BODY_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.0833, degree_vec(0.0, 45.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(-115.0, 67.5, -90.0), LINEAR),
    keyframe(0.375, degree_vec(67.5, 0.0, 0.0), LINEAR),
    keyframe(0.5417, degree_vec(0.0, 45.0, 0.0), LINEAR),
    keyframe(0.7083, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_ATTACK_UPPER_BODY_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.0833, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2917, pos_vec(0.0, -2.7716, -1.1481), LINEAR),
    keyframe(0.375, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5417, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.7083, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_ATTACK_UPPER_BODY_SCALE: [Keyframe; 2] = [
    keyframe(0.0, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(0.7083, scale_vec(1.0, 1.0, 1.0), LINEAR),
];
const CREAKING_ATTACK_HEAD_ROT: [Keyframe; 9] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(0.0, -45.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(-11.25, -45.0, 0.0), LINEAR),
    keyframe(0.2917, degree_vec(-117.3939, 76.6331, -130.1483), LINEAR),
    keyframe(0.4167, degree_vec(-45.0, -45.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(60.0, -45.0, 0.0), LINEAR),
    keyframe(0.5833, degree_vec(60.0, -45.0, 0.0), LINEAR),
    keyframe(0.625, degree_vec(0.0, -45.0, 0.0), LINEAR),
    keyframe(0.7083, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_ATTACK_HEAD_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.4167, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, pos_vec(0.3827, 0.5133, -0.7682), LINEAR),
    keyframe(0.5833, pos_vec(0.3827, 0.5133, -0.7682), LINEAR),
    keyframe(0.625, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.7083, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_ATTACK_HEAD_SCALE: [Keyframe; 4] = [
    keyframe(0.1667, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(0.4167, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(0.5, scale_vec(1.0, 1.3, 1.0), LINEAR),
    keyframe(0.625, scale_vec(1.0, 1.0, 1.0), LINEAR),
];
const CREAKING_ATTACK_RIGHT_ARM_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(7.5, 0.0, 0.0), LINEAR),
    keyframe(0.4583, degree_vec(55.0, 0.0, 0.0), LINEAR),
    keyframe(0.625, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.7083, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_ATTACK_RIGHT_ARM_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.625, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.7083, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_ATTACK_LEFT_LEG_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.625, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.7083, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_ATTACK_LEFT_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, pos_vec(0.0, 0.0, -2.0), LINEAR),
    keyframe(0.625, pos_vec(0.0, 0.0, -2.0), LINEAR),
    keyframe(0.7083, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_ATTACK_RIGHT_LEG_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(0.0, 45.0, 0.0), LINEAR),
    keyframe(0.625, degree_vec(0.0, 45.0, 0.0), LINEAR),
    keyframe(0.7083, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_ATTACK_RIGHT_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, pos_vec(0.7071, 0.0, 0.0), LINEAR),
    keyframe(0.625, pos_vec(0.7071, 0.0, 0.0), LINEAR),
    keyframe(0.7083, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_ATTACK_LEFT_ARM_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(10.3453, 14.7669, 2.664), LINEAR),
    keyframe(0.4583, degree_vec(57.5, 0.0, 0.0), LINEAR),
    keyframe(0.625, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.7083, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_ATTACK_LEFT_ARM_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.7083, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_ATTACK_UPPER_BODY_CHANNELS: [AnimationChannel; 3] = [
    rot(&CREAKING_ATTACK_UPPER_BODY_ROT),
    pos(&CREAKING_ATTACK_UPPER_BODY_POS),
    scale_channel(&CREAKING_ATTACK_UPPER_BODY_SCALE),
];
const CREAKING_ATTACK_HEAD_CHANNELS: [AnimationChannel; 3] = [
    rot(&CREAKING_ATTACK_HEAD_ROT),
    pos(&CREAKING_ATTACK_HEAD_POS),
    scale_channel(&CREAKING_ATTACK_HEAD_SCALE),
];
const CREAKING_ATTACK_RIGHT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&CREAKING_ATTACK_RIGHT_ARM_ROT),
    pos(&CREAKING_ATTACK_RIGHT_ARM_POS),
];
const CREAKING_ATTACK_LEFT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CREAKING_ATTACK_LEFT_LEG_ROT),
    pos(&CREAKING_ATTACK_LEFT_LEG_POS),
];
const CREAKING_ATTACK_RIGHT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CREAKING_ATTACK_RIGHT_LEG_ROT),
    pos(&CREAKING_ATTACK_RIGHT_LEG_POS),
];
const CREAKING_ATTACK_LEFT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&CREAKING_ATTACK_LEFT_ARM_ROT),
    pos(&CREAKING_ATTACK_LEFT_ARM_POS),
];
const CREAKING_ATTACK_BONES: [BoneAnimation; 6] = [
    BoneAnimation {
        bone: "upper_body",
        channels: &CREAKING_ATTACK_UPPER_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &CREAKING_ATTACK_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_arm",
        channels: &CREAKING_ATTACK_RIGHT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_leg",
        channels: &CREAKING_ATTACK_LEFT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_leg",
        channels: &CREAKING_ATTACK_RIGHT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_arm",
        channels: &CREAKING_ATTACK_LEFT_ARM_CHANNELS,
    },
];

/// Vanilla `CreakingAnimation.CREAKING_ATTACK`: the looping 0.7083s lunge, applied additively by
/// `CreakingModel.setupAnim` via `attackAnimation.apply(attackAnimationState, ageInTicks)` over the
/// projected `creaking_attack_seconds`.
pub(in crate::entity_models) const CREAKING_ATTACK: AnimationDefinition = AnimationDefinition {
    length_seconds: 0.7083,
    looping: true,
    bones: &CREAKING_ATTACK_BONES,
};

// ----- `CreakingAnimation.CREAKING_INVULNERABLE` (length 0.2917s, NOT looping). The heart-bound
// stagger: a quick upper-body and arm recoil; all keyframes are LINEAR. -----
const CREAKING_INVULNERABLE_UPPER_BODY_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.0833, degree_vec(-5.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(5.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_INVULNERABLE_UPPER_BODY_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.0833, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_INVULNERABLE_RIGHT_ARM_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.0833, degree_vec(17.5, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(-15.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_INVULNERABLE_RIGHT_ARM_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_INVULNERABLE_LEFT_ARM_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.0833, degree_vec(20.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(-15.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_INVULNERABLE_LEFT_ARM_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_INVULNERABLE_UPPER_BODY_CHANNELS: [AnimationChannel; 2] = [
    rot(&CREAKING_INVULNERABLE_UPPER_BODY_ROT),
    pos(&CREAKING_INVULNERABLE_UPPER_BODY_POS),
];
const CREAKING_INVULNERABLE_RIGHT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&CREAKING_INVULNERABLE_RIGHT_ARM_ROT),
    pos(&CREAKING_INVULNERABLE_RIGHT_ARM_POS),
];
const CREAKING_INVULNERABLE_LEFT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&CREAKING_INVULNERABLE_LEFT_ARM_ROT),
    pos(&CREAKING_INVULNERABLE_LEFT_ARM_POS),
];
const CREAKING_INVULNERABLE_BONES: [BoneAnimation; 3] = [
    BoneAnimation {
        bone: "upper_body",
        channels: &CREAKING_INVULNERABLE_UPPER_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "right_arm",
        channels: &CREAKING_INVULNERABLE_RIGHT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_arm",
        channels: &CREAKING_INVULNERABLE_LEFT_ARM_CHANNELS,
    },
];

/// Vanilla `CreakingAnimation.CREAKING_INVULNERABLE`: the 0.2917s stagger, applied additively by
/// `CreakingModel.setupAnim` via `invulnerableAnimation.apply(invulnerabilityAnimationState,
/// ageInTicks)` over the projected `creaking_invulnerable_seconds`. Non-looping: it clamps past its
/// length to the resting final frame.
pub(in crate::entity_models) const CREAKING_INVULNERABLE: AnimationDefinition =
    AnimationDefinition {
        length_seconds: 0.2917,
        looping: false,
        bones: &CREAKING_INVULNERABLE_BONES,
    };

// ----- `CreakingAnimation.CREAKING_DEATH` (length 2.25s, NOT looping). The collapse: the upper body
// (with a y squash/stretch), head, and arms wind down to a final slumped frame; all keyframes are
// LINEAR. -----
const CREAKING_DEATH_UPPER_BODY_ROT: [Keyframe; 12] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.0833, degree_vec(-40.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(-5.0, 0.0, 0.0), LINEAR),
    keyframe(0.2917, degree_vec(7.5, 0.0, 0.0), LINEAR),
    keyframe(0.5833, degree_vec(16.25, 0.0, 0.0), LINEAR),
    keyframe(0.6667, degree_vec(29.0814, 62.5516, 26.5771), LINEAR),
    keyframe(0.75, degree_vec(12.2115, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(10.25, 0.0, 0.0), LINEAR),
    keyframe(1.0417, degree_vec(-47.64, 0.0, 0.0), LINEAR),
    keyframe(1.125, degree_vec(21.96, 0.0, 0.0), LINEAR),
    keyframe(1.25, degree_vec(12.5, 0.0, 0.0), LINEAR),
    keyframe(2.25, degree_vec(17.3266, 7.9022, -0.1381), LINEAR),
];
const CREAKING_DEATH_UPPER_BODY_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.0833, pos_vec(0.0, 0.557, 1.2659), LINEAR),
    keyframe(0.1667, pos_vec(0.0, -2.0889, -0.3493), LINEAR),
    keyframe(0.2917, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_DEATH_UPPER_BODY_SCALE: [Keyframe; 4] = [
    keyframe(0.0, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(0.0833, scale_vec(1.0, 1.1, 1.0), LINEAR),
    keyframe(0.1667, scale_vec(1.0, 0.9, 1.0), LINEAR),
    keyframe(0.2917, scale_vec(1.0, 1.0, 1.0), LINEAR),
];
const CREAKING_DEATH_RIGHT_ARM_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2917, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.25, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(1.5417, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(1.5833, degree_vec(-12.1479, -34.3927, 6.9326), LINEAR),
    keyframe(1.6667, degree_vec(-10.0, 0.0, 0.0), LINEAR),
];
const CREAKING_DEATH_RIGHT_ARM_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2917, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_DEATH_LEFT_ARM_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2917, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.8333, degree_vec(-4.4444, 0.0, 0.0), LINEAR),
    keyframe(0.875, degree_vec(-26.7402, -78.831, 26.3025), LINEAR),
    keyframe(0.9583, degree_vec(-5.5556, 0.0, 0.0), LINEAR),
    keyframe(1.25, degree_vec(-10.0, 0.0, 0.0), LINEAR),
];
const CREAKING_DEATH_LEFT_ARM_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2917, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_DEATH_HEAD_ROT: [Keyframe; 12] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.0833, degree_vec(-5.0, 0.0, 0.0), LINEAR),
    keyframe(0.2917, degree_vec(10.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(2.5, 0.0, 0.0), LINEAR),
    keyframe(0.5417, degree_vec(5.5, 0.0, 0.0), LINEAR),
    keyframe(0.5833, degree_vec(-67.4168, -12.9552, -8.0231), LINEAR),
    keyframe(0.6667, degree_vec(8.5, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(10.773, -29.5608, -5.3627), LINEAR),
    keyframe(1.25, degree_vec(10.0, 0.0, 0.0), LINEAR),
    keyframe(1.7917, degree_vec(10.0, 0.0, 0.0), LINEAR),
    keyframe(1.8333, degree_vec(12.9625, 39.2735, 8.2901), LINEAR),
    keyframe(1.9167, degree_vec(10.0, 0.0, 0.0), LINEAR),
];
const CREAKING_DEATH_HEAD_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2917, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CREAKING_DEATH_UPPER_BODY_CHANNELS: [AnimationChannel; 3] = [
    rot(&CREAKING_DEATH_UPPER_BODY_ROT),
    pos(&CREAKING_DEATH_UPPER_BODY_POS),
    scale_channel(&CREAKING_DEATH_UPPER_BODY_SCALE),
];
const CREAKING_DEATH_RIGHT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&CREAKING_DEATH_RIGHT_ARM_ROT),
    pos(&CREAKING_DEATH_RIGHT_ARM_POS),
];
const CREAKING_DEATH_LEFT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&CREAKING_DEATH_LEFT_ARM_ROT),
    pos(&CREAKING_DEATH_LEFT_ARM_POS),
];
const CREAKING_DEATH_HEAD_CHANNELS: [AnimationChannel; 2] =
    [rot(&CREAKING_DEATH_HEAD_ROT), pos(&CREAKING_DEATH_HEAD_POS)];
const CREAKING_DEATH_BONES: [BoneAnimation; 4] = [
    BoneAnimation {
        bone: "upper_body",
        channels: &CREAKING_DEATH_UPPER_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "right_arm",
        channels: &CREAKING_DEATH_RIGHT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_arm",
        channels: &CREAKING_DEATH_LEFT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &CREAKING_DEATH_HEAD_CHANNELS,
    },
];

/// Vanilla `CreakingAnimation.CREAKING_DEATH`: the 2.25s collapse, applied additively by
/// `CreakingModel.setupAnim` via `deathAnimation.apply(deathAnimationState, ageInTicks)` over the
/// projected `creaking_death_seconds`. Non-looping: it holds the final slumped frame.
pub(in crate::entity_models) const CREAKING_DEATH: AnimationDefinition = AnimationDefinition {
    length_seconds: 2.25,
    looping: false,
    bones: &CREAKING_DEATH_BONES,
};

/// Mutable creaking model, mirroring vanilla `CreakingModel`. The cubeless `root` part (parenting
/// the empty `upper_body` pivot and the two legs; `upper_body` parents head, body, and two arms)
/// hangs off a synthetic root, built from the baked geometry with named children carrying both the
/// colored tint and the textured UVs. `setup_anim` sets the head look, adds the looping
/// `CREAKING_WALK` cycle (while `canMove`) onto the upper body, head, arms, and legs, then layers the
/// three triggered combat/death one-shots ([`CREAKING_ATTACK`], [`CREAKING_INVULNERABLE`],
/// [`CREAKING_DEATH`]) additively, addressing each bone via `child_mut`.
pub(in crate::entity_models) struct CreakingModel {
    root: ModelPart,
}

impl CreakingModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![("root", creaking_root())]),
        }
    }
}

impl EntityModel for CreakingModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `CreakingModel.setupAnim` (after `resetPose`): SET `head.xRot/yRot` from the plain
        // look, then (while `canMove`) `walkAnimation.applyWalk(walkAnimationPos, walkAnimationSpeed,
        // 1, 1)`, then ADD the three combat/death one-shots — `attackAnimation`, `invulnerableAnimation`,
        // `deathAnimation` — over their projected elapsed seconds. The walk and the one-shots offset the
        // upper body, head (ADDING onto the look), arms, and legs; the `body` holds. A `canMove`-false
        // (observed/frozen) creaking skips the walk and holds the bind pose plus its look.
        let head_pitch = instance.render_state.head_pitch.to_radians();
        let head_yaw = instance.render_state.head_yaw.to_radians();
        let can_move = instance.render_state.creaking_can_move;

        // The looping walk cycle, sampled by `applyWalk` while `canMove`; a frozen creaking skips it.
        let (walk_seconds, walk_scale) = keyframe_walk_sample(
            &CREAKING_WALK,
            instance.render_state.walk_animation_pos,
            instance.render_state.walk_animation_speed,
            1.0,
            1.0,
        );
        let apply_walk = |part: &mut ModelPart, bone: &str| {
            if !can_move {
                return;
            }
            let (position, rotation) =
                sample_bone_offsets(&CREAKING_WALK, bone, walk_seconds, walk_scale);
            part.pose = keyframe_animated_pose(part.pose, position, rotation);
        };

        // The three triggered one-shots applied ADDITIVELY in vanilla order (attack → invulnerable →
        // death), each only while its projected elapsed seconds is `>= 0`. `CREAKING_ATTACK` loops;
        // the non-looping invulnerable/death clamp past their length to the resting final frame. The
        // attack and death carry `SCALE` channels (a head/upper-body stretch), folded onto the part's
        // reset `[1, 1, 1]` scale.
        let combat: [(&AnimationDefinition, f32); 3] = [
            (
                &CREAKING_ATTACK,
                instance.render_state.creaking_attack_seconds,
            ),
            (
                &CREAKING_INVULNERABLE,
                instance.render_state.creaking_invulnerable_seconds,
            ),
            (
                &CREAKING_DEATH,
                instance.render_state.creaking_death_seconds,
            ),
        ];
        let apply_combat = |part: &mut ModelPart, bone: &str| {
            for (definition, seconds) in combat {
                if seconds < 0.0 {
                    continue;
                }
                let sample = keyframe_elapsed_seconds(definition, seconds);
                let (position, rotation, scale_offset) =
                    sample_bone_offsets_with_scale(definition, bone, sample, 1.0);
                part.pose = keyframe_animated_pose(part.pose, position, rotation);
                // Vanilla `ModelPart.offsetScale` adds the `scaleVec` offset onto the (reset `1.0`)
                // base scale; folding each active def in turn accumulates them like vanilla.
                part.scale = [
                    part.scale[0] + scale_offset[0],
                    part.scale[1] + scale_offset[1],
                    part.scale[2] + scale_offset[2],
                ];
            }
        };

        let creaking_root = self.root.child_mut("root");
        {
            let upper_body = creaking_root.child_mut("upper_body");
            apply_walk(upper_body, "upper_body");
            apply_combat(upper_body, "upper_body");

            // head: the look (set), then the walk and combat rotations (added).
            let head = upper_body.child_mut("head");
            head.pose.rotation = [head_pitch, head_yaw, head.pose.rotation[2]];
            apply_walk(head, "head");
            apply_combat(head, "head");

            // `body` holds; the two arms take the walk + combat.
            let right_arm = upper_body.child_mut("right_arm");
            apply_walk(right_arm, "right_arm");
            apply_combat(right_arm, "right_arm");
            let left_arm = upper_body.child_mut("left_arm");
            apply_walk(left_arm, "left_arm");
            apply_combat(left_arm, "left_arm");
        }
        let left_leg = creaking_root.child_mut("left_leg");
        apply_walk(left_leg, "left_leg");
        apply_combat(left_leg, "left_leg");
        let right_leg = creaking_root.child_mut("right_leg");
        apply_walk(right_leg, "right_leg");
        apply_combat(right_leg, "right_leg");
    }
}
