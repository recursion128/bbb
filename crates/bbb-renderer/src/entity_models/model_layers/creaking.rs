use super::super::keyframe::{
    degree_vec, keyframe, pos_vec, AnimationChannel, AnimationDefinition, AnimationTarget,
    BoneAnimation, Keyframe, KeyframeInterpolation,
};
use super::{bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc, CREAKING_BARK};

// Vanilla 26.1 `CreakingModel.createBodyLayer` (atlas 64×64). The mesh root holds one `root` part
// at `offset(0, 24, 0)` parenting `upper_body` and the two legs; `upper_body` (an empty pivot)
// parents the head (with its two antler/branch planes), the body, and the two arms. `setupAnim`
// sets `head.xRot/yRot` from the plain look (reproduced through the projected look angles) and, when
// `canMove`, applies the looping `CreakingAnimation.CREAKING_WALK` ([`CREAKING_WALK`]) which offsets
// the upper body, head, arms, and legs. The head channel ADDS onto the look. The attack,
// invulnerable, and death keyframe animations are deferred, as is the un-projected `canMove` freeze
// gate (a frozen creaking has walk speed ≈ 0, so the amplitude already collapses to the rest pose —
// fittingly, the creaking turns to a statue while observed). The emissive eyes layer
// (`createEyesLayer`, the `head` part only) and the texture-backed path are also deferred.

// `head`: the 6×10×6 skull, the 6×3×6 brow, and two 9×14×0 antler/branch planes.
const CREAKING_HEAD_CUBES: [ModelCubeDesc; 4] = [
    cube([-3.0, -10.0, -3.0], [6.0, 10.0, 6.0], CREAKING_BARK),
    cube([-3.0, -13.0, -3.0], [6.0, 3.0, 6.0], CREAKING_BARK),
    cube([3.0, -13.0, 0.0], [9.0, 14.0, 0.0], CREAKING_BARK),
    cube([-12.0, -14.0, 0.0], [9.0, 14.0, 0.0], CREAKING_BARK),
];

// `body`: the 6×13×5 trunk plus the 6×7×5 upper block.
const CREAKING_BODY_CUBES: [ModelCubeDesc; 2] = [
    cube([0.0, -3.0, -3.0], [6.0, 13.0, 5.0], CREAKING_BARK),
    cube([-6.0, -4.0, -3.0], [6.0, 7.0, 5.0], CREAKING_BARK),
];

// `right_arm`: a 3×21×3 limb plus a 3×4×3 hand.
const CREAKING_RIGHT_ARM_CUBES: [ModelCubeDesc; 2] = [
    cube([-2.0, -1.5, -1.5], [3.0, 21.0, 3.0], CREAKING_BARK),
    cube([-2.0, 19.5, -1.5], [3.0, 4.0, 3.0], CREAKING_BARK),
];

// `left_arm`: a 3×16×3 limb with a 3×4×3 shoulder block and a 3×4×3 hand.
const CREAKING_LEFT_ARM_CUBES: [ModelCubeDesc; 3] = [
    cube([0.0, -1.0, -1.5], [3.0, 16.0, 3.0], CREAKING_BARK),
    cube([0.0, -5.0, -1.5], [3.0, 4.0, 3.0], CREAKING_BARK),
    cube([0.0, 15.0, -1.5], [3.0, 4.0, 3.0], CREAKING_BARK),
];

// `left_leg`: a 3×16×3 limb plus a 5×0×9 foot plane.
const CREAKING_LEFT_LEG_CUBES: [ModelCubeDesc; 2] = [
    cube([-1.5, 0.0, -1.5], [3.0, 16.0, 3.0], CREAKING_BARK),
    cube([-1.5, 15.7, -4.5], [5.0, 0.0, 9.0], CREAKING_BARK),
];

// `right_leg`: a 3×19×3 limb, a 5×0×9 foot plane, and a 3×3×3 hip block.
const CREAKING_RIGHT_LEG_CUBES: [ModelCubeDesc; 3] = [
    cube([-3.0, -1.5, -1.5], [3.0, 19.0, 3.0], CREAKING_BARK),
    cube([-5.0, 17.2, -4.5], [5.0, 0.0, 9.0], CREAKING_BARK),
    cube([-3.0, -4.5, -1.5], [3.0, 3.0, 3.0], CREAKING_BARK),
];

// `upper_body` children: head, body, and the two arms.
const CREAKING_UPPER_BODY_CHILDREN: [ModelPartDesc; 4] = [
    part([-3.0, -11.0, 0.0], &CREAKING_HEAD_CUBES, &[]),
    part([0.0, -7.0, 1.0], &CREAKING_BODY_CUBES, &[]),
    part([-7.0, -9.5, 1.5], &CREAKING_RIGHT_ARM_CUBES, &[]),
    part([6.0, -9.0, 0.5], &CREAKING_LEFT_ARM_CUBES, &[]),
];

// `root` children: the `upper_body` pivot and the two legs.
const CREAKING_ROOT_CHILDREN: [ModelPartDesc; 3] = [
    part([-1.0, -19.0, 0.0], &[], &CREAKING_UPPER_BODY_CHILDREN),
    part([1.5, -16.0, 0.5], &CREAKING_LEFT_LEG_CUBES, &[]),
    part([-1.0, -17.5, 0.5], &CREAKING_RIGHT_LEG_CUBES, &[]),
];

/// Vanilla `CreakingModel.createBodyLayer` rest-pose hierarchy, rooted at the `root` part
/// (`offset(0, 24, 0)`). Sixteen cubes.
pub(in crate::entity_models) const CREAKING_PARTS: [ModelPartDesc; 1] =
    [part([0.0, 24.0, 0.0], &[], &CREAKING_ROOT_CHILDREN)];

// ----- `CreakingAnimation.CREAKING_WALK` (length 1.125s, looping). All keyframes are LINEAR. The
// animated bones map to the part tree as: `upper_body` = root child 0, `left_leg`/`right_leg` = root
// children 1/2, `head`/`right_arm`/`left_arm` = upper_body children 0/2/3 (the `body`, child 1, is
// not animated). The `head` rotation channel adds onto the head look. -----

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
