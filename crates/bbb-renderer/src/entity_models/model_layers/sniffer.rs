use super::super::keyframe::{
    degree_vec, keyframe, pos_vec, AnimationChannel, AnimationDefinition, AnimationTarget,
    BoneAnimation, Keyframe, KeyframeInterpolation,
};
use super::{
    bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc, SNIFFER_BROWN,
    SNIFFER_NOSE,
};

// Vanilla 26.1 `SnifferModel.createBodyLayer` (atlas 192×192). The mesh root holds one `bone`
// part at `offset(0, 5, 0)` parenting the body and the six legs; `body` parents the head, which
// parents the two ears, the nose, and the lower beak. `setupAnim` sets `head.xRot/yRot` from the
// plain look (reproduced through the projected look angles, the head's ear/nose/beak children
// inheriting the turn), then applies a walk: while NOT searching it samples the looping
// `SnifferAnimation.SNIFFER_WALK` ([`SNIFFER_WALK`]) via `applyWalk(..., 9, 100)`, rocking the body,
// the head (the walk pitch ADDS onto the look), the two ears, and the six legs. The search-walk
// variant (`SNIFFER_SNIFF_SEARCH`, gated on the un-synced `isSearching`) and the dig / long-sniff /
// stand-up / happy / scenting keyframe animations are deferred. The texture-backed path is deferred.

// `body`: the 25×29×40 trunk, a 25×24×40 inner block inflated by `CubeDeformation(0.5)` (geometry
// `min -= 0.5`, `size += 1`), and the 25×0×40 belly plane.
const SNIFFER_BODY_CUBES: [ModelCubeDesc; 3] = [
    cube([-12.5, -14.0, -20.0], [25.0, 29.0, 40.0], SNIFFER_BROWN),
    cube([-13.0, -14.5, -20.5], [26.0, 25.0, 41.0], SNIFFER_BROWN),
    cube([-12.5, 12.0, -20.0], [25.0, 0.0, 40.0], SNIFFER_BROWN),
];

// `head`: the 13×18×11 skull plus a 13×0×11 top plane.
const SNIFFER_HEAD_CUBES: [ModelCubeDesc; 2] = [
    cube([-6.5, -7.5, -11.5], [13.0, 18.0, 11.0], SNIFFER_BROWN),
    cube([-6.5, 7.5, -11.5], [13.0, 0.0, 11.0], SNIFFER_BROWN),
];

const SNIFFER_LEFT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, 0.0, -3.0], [1.0, 19.0, 7.0], SNIFFER_BROWN)];
const SNIFFER_RIGHT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, -3.0], [1.0, 19.0, 7.0], SNIFFER_BROWN)];

// The 13×2×9 nose pad (the sniffer's distinctive snout) and the 13×12×9 lower beak / jaw.
const SNIFFER_NOSE_CUBES: [ModelCubeDesc; 1] =
    [cube([-6.5, -2.0, -9.0], [13.0, 2.0, 9.0], SNIFFER_NOSE)];
const SNIFFER_LOWER_BEAK_CUBES: [ModelCubeDesc; 1] =
    [cube([-6.5, -7.0, -8.0], [13.0, 12.0, 9.0], SNIFFER_BROWN)];

// All six legs share one 7×10×8 box.
const SNIFFER_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.5, -1.0, -4.0], [7.0, 10.0, 8.0], SNIFFER_BROWN)];

// `head` children: the two ears, the nose, and the lower beak.
const SNIFFER_HEAD_CHILDREN: [ModelPartDesc; 4] = [
    part([6.51, -7.5, -4.51], &SNIFFER_LEFT_EAR_CUBES, &[]),
    part([-6.51, -7.5, -4.51], &SNIFFER_RIGHT_EAR_CUBES, &[]),
    part([0.0, -4.5, -11.5], &SNIFFER_NOSE_CUBES, &[]),
    part([0.0, 2.5, -12.5], &SNIFFER_LOWER_BEAK_CUBES, &[]),
];

// `body` (at `offset(0, 0, 0)`) parents the head.
const SNIFFER_BODY_CHILDREN: [ModelPartDesc; 1] = [part(
    [0.0, 6.5, -19.48],
    &SNIFFER_HEAD_CUBES,
    &SNIFFER_HEAD_CHILDREN,
)];

// `bone` children: the body and the six legs (right/left × front/mid/hind).
const SNIFFER_BONE_CHILDREN: [ModelPartDesc; 7] = [
    part([0.0, 0.0, 0.0], &SNIFFER_BODY_CUBES, &SNIFFER_BODY_CHILDREN),
    part([-7.5, 10.0, -15.0], &SNIFFER_LEG_CUBES, &[]),
    part([-7.5, 10.0, 0.0], &SNIFFER_LEG_CUBES, &[]),
    part([-7.5, 10.0, 15.0], &SNIFFER_LEG_CUBES, &[]),
    part([7.5, 10.0, -15.0], &SNIFFER_LEG_CUBES, &[]),
    part([7.5, 10.0, 0.0], &SNIFFER_LEG_CUBES, &[]),
    part([7.5, 10.0, 15.0], &SNIFFER_LEG_CUBES, &[]),
];

/// Vanilla `SnifferModel.createBodyLayer` rest-pose hierarchy, rooted at the `bone` part
/// (`offset(0, 5, 0)`). Fifteen cubes.
pub(in crate::entity_models) const SNIFFER_PARTS: [ModelPartDesc; 1] =
    [part([0.0, 5.0, 0.0], &[], &SNIFFER_BONE_CHILDREN)];

// ----- `SnifferAnimation.SNIFFER_WALK` (length 2.0s, looping) -----
//
// `SnifferModel.setupAnim` samples it (while not searching) via
// `applyWalk(walkAnimationPos, walkAnimationSpeed, 9.0, 100.0)`. It animates the six legs (rotation +
// position), the `body` (a small pitch/roll sway with a y-dip), the `head` (a CatmullRom pitch that
// ADDS onto the look), and the two ears (a CatmullRom z-roll). The `bone` root is not animated.

const LINEAR: KeyframeInterpolation = KeyframeInterpolation::Linear;
const CATMULLROM: KeyframeInterpolation = KeyframeInterpolation::CatmullRom;

const SNIFFER_WALK_RIGHT_FRONT_LEG_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5833, degree_vec(35.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(-35.0, 0.0, 0.0), LINEAR),
    keyframe(1.1667, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const SNIFFER_WALK_RIGHT_FRONT_LEG_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 3.0), LINEAR),
    keyframe(0.75, pos_vec(0.0, 4.0, -1.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.1667, pos_vec(0.0, 0.0, -1.0), LINEAR),
    keyframe(2.0, pos_vec(0.0, 0.0, 3.0), LINEAR),
];
const SNIFFER_WALK_RIGHT_MID_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(-7.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(-35.0, 0.0, 0.0), LINEAR),
    keyframe(0.3333, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.1667, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.75, degree_vec(35.0, 0.0, 0.0), LINEAR),
    keyframe(2.0, degree_vec(-7.0, 0.0, 0.0), LINEAR),
];
const SNIFFER_WALK_RIGHT_MID_LEG_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 2.67, -0.67), LINEAR),
    keyframe(0.1667, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.3333, pos_vec(0.0, 0.0, -2.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, 0.0, 2.0), LINEAR),
    keyframe(1.1667, pos_vec(0.0, 0.0, 3.0), LINEAR),
    keyframe(1.9167, pos_vec(0.0, 4.0, -1.0), LINEAR),
    keyframe(2.0, pos_vec(0.0, 2.67, -0.67), LINEAR),
];
const SNIFFER_WALK_RIGHT_HIND_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5833, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(25.0, 0.0, 0.0), LINEAR),
    keyframe(1.1667, degree_vec(35.0, 0.0, 0.0), LINEAR),
    keyframe(1.5833, degree_vec(-35.0, 0.0, 0.0), LINEAR),
    keyframe(1.75, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const SNIFFER_WALK_RIGHT_HIND_LEG_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(0.5833, pos_vec(0.0, 0.0, 2.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, 2.22, 0.78), LINEAR),
    keyframe(1.3333, pos_vec(0.0, 4.0, -1.0), LINEAR),
    keyframe(1.5833, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.75, pos_vec(0.0, 0.0, -2.0), LINEAR),
    keyframe(2.0, pos_vec(0.0, 0.0, -0.5), LINEAR),
];
const SNIFFER_WALK_LEFT_FRONT_LEG_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(-35.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5833, degree_vec(35.0, 0.0, 0.0), LINEAR),
    keyframe(2.0, degree_vec(-35.0, 0.0, 0.0), LINEAR),
];
const SNIFFER_WALK_LEFT_FRONT_LEG_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, pos_vec(0.0, 0.0, -1.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, 0.0, 3.0), LINEAR),
    keyframe(1.75, pos_vec(0.0, 4.0, -1.0), LINEAR),
    keyframe(2.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const SNIFFER_WALK_LEFT_MID_LEG_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(35.0, 0.0, 0.0), LINEAR),
    keyframe(1.1667, degree_vec(-35.0, 0.0, 0.0), LINEAR),
    keyframe(1.3333, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const SNIFFER_WALK_LEFT_MID_LEG_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 2.0), LINEAR),
    keyframe(0.1667, pos_vec(0.0, 0.0, 3.0), LINEAR),
    keyframe(0.9167, pos_vec(0.0, 4.0, -1.0), LINEAR),
    keyframe(1.1667, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.3333, pos_vec(0.0, 0.0, -2.0), LINEAR),
    keyframe(2.0, pos_vec(0.0, 0.0, 2.0), LINEAR),
];
const SNIFFER_WALK_LEFT_HIND_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(25.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(35.0, 0.0, 0.0), LINEAR),
    keyframe(0.5833, degree_vec(-35.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5833, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.0, degree_vec(25.0, 0.0, 0.0), LINEAR),
];
const SNIFFER_WALK_LEFT_HIND_LEG_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 2.22, 0.78), LINEAR),
    keyframe(0.3333, pos_vec(0.0, 4.0, -1.0), LINEAR),
    keyframe(0.5833, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, pos_vec(0.0, 0.0, -2.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(1.5833, pos_vec(0.0, 0.0, 2.0), LINEAR),
    keyframe(2.0, pos_vec(0.0, 2.22, 0.78), LINEAR),
];
const SNIFFER_WALK_BODY_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(1.0, 0.0, -2.5), LINEAR),
    keyframe(0.5, degree_vec(-1.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(1.0, 0.0, 2.5), LINEAR),
    keyframe(1.5, degree_vec(-1.0, 0.0, 0.0), LINEAR),
    keyframe(2.0, degree_vec(1.0, 0.0, -2.5), LINEAR),
];
const SNIFFER_WALK_BODY_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.375, pos_vec(0.0, -1.0, 0.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.375, pos_vec(0.0, -1.0, 0.0), LINEAR),
    keyframe(2.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const SNIFFER_WALK_HEAD_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(7.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.1667, degree_vec(9.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.875, degree_vec(-1.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.25, degree_vec(7.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.75, degree_vec(5.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.0, degree_vec(7.5, 0.0, 0.0), CATMULLROM),
];
const SNIFFER_WALK_LEFT_EAR_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, -2.5), CATMULLROM),
    keyframe(0.5, degree_vec(0.0, 0.0, -7.5), CATMULLROM),
    keyframe(1.0, degree_vec(0.0, 0.0, -2.5), CATMULLROM),
    keyframe(1.5, degree_vec(0.0, 0.0, -7.5), CATMULLROM),
    keyframe(2.0, degree_vec(0.0, 0.0, -2.5), CATMULLROM),
];
const SNIFFER_WALK_RIGHT_EAR_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 2.5), CATMULLROM),
    keyframe(0.5, degree_vec(0.0, 0.0, 7.5), CATMULLROM),
    keyframe(1.0, degree_vec(0.0, 0.0, 2.5), CATMULLROM),
    keyframe(1.5, degree_vec(0.0, 0.0, 7.5), CATMULLROM),
    keyframe(2.0, degree_vec(0.0, 0.0, 2.5), CATMULLROM),
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

const SNIFFER_WALK_RIGHT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_WALK_RIGHT_FRONT_LEG_ROT),
    pos(&SNIFFER_WALK_RIGHT_FRONT_LEG_POS),
];
const SNIFFER_WALK_RIGHT_MID_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_WALK_RIGHT_MID_LEG_ROT),
    pos(&SNIFFER_WALK_RIGHT_MID_LEG_POS),
];
const SNIFFER_WALK_RIGHT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_WALK_RIGHT_HIND_LEG_ROT),
    pos(&SNIFFER_WALK_RIGHT_HIND_LEG_POS),
];
const SNIFFER_WALK_LEFT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_WALK_LEFT_FRONT_LEG_ROT),
    pos(&SNIFFER_WALK_LEFT_FRONT_LEG_POS),
];
const SNIFFER_WALK_LEFT_MID_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_WALK_LEFT_MID_LEG_ROT),
    pos(&SNIFFER_WALK_LEFT_MID_LEG_POS),
];
const SNIFFER_WALK_LEFT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_WALK_LEFT_HIND_LEG_ROT),
    pos(&SNIFFER_WALK_LEFT_HIND_LEG_POS),
];
const SNIFFER_WALK_BODY_CHANNELS: [AnimationChannel; 2] =
    [rot(&SNIFFER_WALK_BODY_ROT), pos(&SNIFFER_WALK_BODY_POS)];
const SNIFFER_WALK_HEAD_CHANNELS: [AnimationChannel; 1] = [rot(&SNIFFER_WALK_HEAD_ROT)];
const SNIFFER_WALK_LEFT_EAR_CHANNELS: [AnimationChannel; 1] = [rot(&SNIFFER_WALK_LEFT_EAR_ROT)];
const SNIFFER_WALK_RIGHT_EAR_CHANNELS: [AnimationChannel; 1] = [rot(&SNIFFER_WALK_RIGHT_EAR_ROT)];

const SNIFFER_WALK_BONES: [BoneAnimation; 10] = [
    BoneAnimation {
        bone: "right_front_leg",
        channels: &SNIFFER_WALK_RIGHT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_mid_leg",
        channels: &SNIFFER_WALK_RIGHT_MID_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_hind_leg",
        channels: &SNIFFER_WALK_RIGHT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_front_leg",
        channels: &SNIFFER_WALK_LEFT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_mid_leg",
        channels: &SNIFFER_WALK_LEFT_MID_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_hind_leg",
        channels: &SNIFFER_WALK_LEFT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "body",
        channels: &SNIFFER_WALK_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &SNIFFER_WALK_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "left_ear",
        channels: &SNIFFER_WALK_LEFT_EAR_CHANNELS,
    },
    BoneAnimation {
        bone: "right_ear",
        channels: &SNIFFER_WALK_RIGHT_EAR_CHANNELS,
    },
];

/// Vanilla `SnifferAnimation.SNIFFER_WALK`: the looping 2.0s walk cycle, sampled by
/// `SnifferModel.setupAnim` (while not searching) via
/// `applyWalk(walkAnimationPos, walkAnimationSpeed, 9.0, 100.0)`. The `head` pitch and the two ear
/// rolls use CatmullRom interpolation; the `head` channel ADDS onto the plain look the head tracks.
pub(in crate::entity_models) const SNIFFER_WALK: AnimationDefinition = AnimationDefinition {
    length_seconds: 2.0,
    looping: true,
    bones: &SNIFFER_WALK_BONES,
};

/// Vanilla `SnifferModel.applyWalk(..., 9.0F, 100.0F)` factors (`WALK_ANIMATION_SPEED_MAX` drives
/// the sample time, `WALK_ANIMATION_SCALE_FACTOR` the amplitude).
pub(in crate::entity_models) const SNIFFER_WALK_SPEED_FACTOR: f32 = 9.0;
pub(in crate::entity_models) const SNIFFER_WALK_SCALE_FACTOR: f32 = 100.0;
