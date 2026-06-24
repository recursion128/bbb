use super::super::keyframe::{
    degree_vec, keyframe, keyframe_animated_pose, keyframe_animated_scale,
    keyframe_elapsed_seconds, keyframe_walk_sample, pos_vec, sample_bone_offsets,
    sample_bone_offsets_with_scale, scale_vec, AnimationChannel, AnimationDefinition,
    AnimationTarget, BoneAnimation, Keyframe, KeyframeInterpolation,
};
use super::{
    model_cube as cube, ModelCubeDesc, PartPose, PART_POSE_ZERO, SNIFFER_BROWN, SNIFFER_NOSE,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

// Vanilla 26.1 `SnifferModel.createBodyLayer` (atlas 192×192). The mesh root holds one `bone`
// part at `offset(0, 5, 0)` parenting the body and the six legs; `body` parents the head, which
// parents the two ears, the nose, and the lower beak. `setupAnim` sets `head.xRot/yRot` from the
// plain look (reproduced through the projected look angles, the head's ear/nose/beak children
// inheriting the turn), then applies a walk: while NOT searching it samples the looping
// `SnifferAnimation.SNIFFER_WALK` ([`SNIFFER_WALK`]) via `applyWalk(..., 9, 100)`, rocking the body,
// the head (the walk pitch ADDS onto the look), the two ears, and the six legs. It then layers the
// active synced-`DATA_STATE` one-shot over the walk: DIGGING→[`SNIFFER_DIG`], SNIFFING→
// [`SNIFFER_LONGSNIFF`], RISING→[`SNIFFER_STAND_UP`], FEELING_HAPPY→[`SNIFFER_HAPPY`], SCENTING→
// [`SNIFFER_SNIFFSNIFF`] (driven by the projected `sniffer_animation_id`/`_seconds`). The search-walk
// variant (`SNIFFER_SNIFF_SEARCH`, gated on the un-synced `isSearching`) and the baby-transform stay
// deferred. The texture-backed path is deferred.

// `body`: the 25×29×40 trunk, a 25×24×40 inner block inflated by `CubeDeformation(0.5)` (geometry
// `min -= 0.5`, `size += 1`), and the 25×0×40 belly plane.
pub(in crate::entity_models) const SNIFFER_BODY_CUBES: [ModelCubeDesc; 3] = [
    cube([-12.5, -14.0, -20.0], [25.0, 29.0, 40.0], SNIFFER_BROWN),
    cube([-13.0, -14.5, -20.5], [26.0, 25.0, 41.0], SNIFFER_BROWN),
    cube([-12.5, 12.0, -20.0], [25.0, 0.0, 40.0], SNIFFER_BROWN),
];

// `head`: the 13×18×11 skull plus a 13×0×11 top plane.
pub(in crate::entity_models) const SNIFFER_HEAD_CUBES: [ModelCubeDesc; 2] = [
    cube([-6.5, -7.5, -11.5], [13.0, 18.0, 11.0], SNIFFER_BROWN),
    cube([-6.5, 7.5, -11.5], [13.0, 0.0, 11.0], SNIFFER_BROWN),
];

pub(in crate::entity_models) const SNIFFER_LEFT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, 0.0, -3.0], [1.0, 19.0, 7.0], SNIFFER_BROWN)];
pub(in crate::entity_models) const SNIFFER_RIGHT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, -3.0], [1.0, 19.0, 7.0], SNIFFER_BROWN)];

// The 13×2×9 nose pad (the sniffer's distinctive snout) and the 13×12×9 lower beak / jaw.
pub(in crate::entity_models) const SNIFFER_NOSE_CUBES: [ModelCubeDesc; 1] =
    [cube([-6.5, -2.0, -9.0], [13.0, 2.0, 9.0], SNIFFER_NOSE)];
pub(in crate::entity_models) const SNIFFER_LOWER_BEAK_CUBES: [ModelCubeDesc; 1] =
    [cube([-6.5, -7.0, -8.0], [13.0, 12.0, 9.0], SNIFFER_BROWN)];

// All six legs share one 7×10×8 box.
pub(in crate::entity_models) const SNIFFER_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.5, -1.0, -4.0], [7.0, 10.0, 8.0], SNIFFER_BROWN)];

/// Vanilla `SnifferModel.createBodyLayer` rest-pose part poses, rooted at the cubeless `bone` part
/// (`offset(0, 5, 0)`) parenting the `body` and the six legs; `body` parents the `head`, which
/// parents the two ears, the nose, and the lower beak. Fifteen cubes.
/// `bone` cubeless-pivot part pose: `PartPose.offset(0, 5, 0)`.
pub(in crate::entity_models) const SNIFFER_BONE_POSE: PartPose = PartPose {
    offset: [0.0, 5.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `body` part pose: `PartPose.offset(0, 0, 0)`.
pub(in crate::entity_models) const SNIFFER_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `head` part pose: `PartPose.offset(0, 6.5, -19.48)`.
pub(in crate::entity_models) const SNIFFER_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 6.5, -19.48],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_ear` part pose: `PartPose.offset(6.51, -7.5, -4.51)`.
pub(in crate::entity_models) const SNIFFER_LEFT_EAR_POSE: PartPose = PartPose {
    offset: [6.51, -7.5, -4.51],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_ear` part pose: `PartPose.offset(-6.51, -7.5, -4.51)`.
pub(in crate::entity_models) const SNIFFER_RIGHT_EAR_POSE: PartPose = PartPose {
    offset: [-6.51, -7.5, -4.51],
    rotation: [0.0, 0.0, 0.0],
};
/// `nose` part pose: `PartPose.offset(0, -4.5, -11.5)`.
pub(in crate::entity_models) const SNIFFER_NOSE_POSE: PartPose = PartPose {
    offset: [0.0, -4.5, -11.5],
    rotation: [0.0, 0.0, 0.0],
};
/// `lower_beak` part pose: `PartPose.offset(0, 2.5, -12.5)`.
pub(in crate::entity_models) const SNIFFER_LOWER_BEAK_POSE: PartPose = PartPose {
    offset: [0.0, 2.5, -12.5],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_front_leg` part pose: `PartPose.offset(-7.5, 10, -15)`.
pub(in crate::entity_models) const SNIFFER_RIGHT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-7.5, 10.0, -15.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_mid_leg` part pose: `PartPose.offset(-7.5, 10, 0)`.
pub(in crate::entity_models) const SNIFFER_RIGHT_MID_LEG_POSE: PartPose = PartPose {
    offset: [-7.5, 10.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_hind_leg` part pose: `PartPose.offset(-7.5, 10, 15)`.
pub(in crate::entity_models) const SNIFFER_RIGHT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-7.5, 10.0, 15.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_front_leg` part pose: `PartPose.offset(7.5, 10, -15)`.
pub(in crate::entity_models) const SNIFFER_LEFT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [7.5, 10.0, -15.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_mid_leg` part pose: `PartPose.offset(7.5, 10, 0)`.
pub(in crate::entity_models) const SNIFFER_LEFT_MID_LEG_POSE: PartPose = PartPose {
    offset: [7.5, 10.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_hind_leg` part pose: `PartPose.offset(7.5, 10, 15)`.
pub(in crate::entity_models) const SNIFFER_LEFT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [7.5, 10.0, 15.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds the sniffer's synthetic root parenting the single cubeless `bone` part, which parents the
/// cube-bearing `body` (→ `head` → two ears / nose / lower beak) and the six legs, in vanilla
/// `addOrReplaceChild` order. The `bone`, `body`, `head`, both ears, and the six legs are
/// name-addressed by `setup_anim`, so `bone`, `body`, and `head` carry named children.
fn sniffer_root() -> ModelPart {
    let head = ModelPart::colored_named(
        SNIFFER_HEAD_POSE,
        &SNIFFER_HEAD_CUBES,
        vec![
            (
                "left_ear",
                ModelPart::leaf_colored(SNIFFER_LEFT_EAR_POSE, &SNIFFER_LEFT_EAR_CUBES),
            ),
            (
                "right_ear",
                ModelPart::leaf_colored(SNIFFER_RIGHT_EAR_POSE, &SNIFFER_RIGHT_EAR_CUBES),
            ),
            (
                "nose",
                ModelPart::leaf_colored(SNIFFER_NOSE_POSE, &SNIFFER_NOSE_CUBES),
            ),
            (
                "lower_beak",
                ModelPart::leaf_colored(SNIFFER_LOWER_BEAK_POSE, &SNIFFER_LOWER_BEAK_CUBES),
            ),
        ],
    );
    let body =
        ModelPart::colored_named(SNIFFER_BODY_POSE, &SNIFFER_BODY_CUBES, vec![("head", head)]);
    let bone = ModelPart::new(
        SNIFFER_BONE_POSE,
        Vec::new(),
        vec![
            ("body", body),
            (
                "right_front_leg",
                ModelPart::leaf_colored(SNIFFER_RIGHT_FRONT_LEG_POSE, &SNIFFER_LEG_CUBES),
            ),
            (
                "right_mid_leg",
                ModelPart::leaf_colored(SNIFFER_RIGHT_MID_LEG_POSE, &SNIFFER_LEG_CUBES),
            ),
            (
                "right_hind_leg",
                ModelPart::leaf_colored(SNIFFER_RIGHT_HIND_LEG_POSE, &SNIFFER_LEG_CUBES),
            ),
            (
                "left_front_leg",
                ModelPart::leaf_colored(SNIFFER_LEFT_FRONT_LEG_POSE, &SNIFFER_LEG_CUBES),
            ),
            (
                "left_mid_leg",
                ModelPart::leaf_colored(SNIFFER_LEFT_MID_LEG_POSE, &SNIFFER_LEG_CUBES),
            ),
            (
                "left_hind_leg",
                ModelPart::leaf_colored(SNIFFER_LEFT_HIND_LEG_POSE, &SNIFFER_LEG_CUBES),
            ),
        ],
    );
    ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![("bone", bone)])
}

// ----- `SnifferAnimation.SNIFFER_WALK` (length 2.0s, looping) -----
//
// `SnifferModel.setupAnim` samples it (while not searching) via
// `applyWalk(walkAnimationPos, walkAnimationSpeed, 9.0, 100.0)`. It animates the six legs (rotation +
// position), the `body` (a small pitch/roll sway with a y-dip), the `head` (a CatmullRom pitch that
// ADDS onto the look), and the two ears (a CatmullRom z-roll). The `bone` root is not animated.

const LINEAR: KeyframeInterpolation = KeyframeInterpolation::Linear;
const CATMULLROM: KeyframeInterpolation = KeyframeInterpolation::CatmullRom;

/// `Sniffer.State` ordinals (the synced `DATA_STATE` VarInt) whose one-shot keyframe the renderer
/// applies, matching the projected `sniffer_animation_id`. `IDLING(0)` and `SEARCHING(4)` carry no
/// one-shot here.
const SNIFFER_STATE_FEELING_HAPPY_ID: i32 = 1;
const SNIFFER_STATE_SCENTING_ID: i32 = 2;
const SNIFFER_STATE_SNIFFING_ID: i32 = 3;
const SNIFFER_STATE_DIGGING_ID: i32 = 5;
const SNIFFER_STATE_RISING_ID: i32 = 6;

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
const fn scale_channel(keyframes: &'static [Keyframe]) -> AnimationChannel {
    AnimationChannel {
        target: AnimationTarget::Scale,
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

// ----- The synced-state one-shot keyframe animations applied by `SnifferModel.setupAnim`:
// `digAnimation`(DIGGING) / `longSniffAnimation`(SNIFFING) / `standUpAnimation`(RISING) /
// `happyAnimation`(FEELING_HAPPY) / `sniffSniffAnimation`(SCENTING). The `head` ROTATION channels
// ADD onto the plain look; the `nose` SCALE channels (`scaleVec` stores `value - 1`) puff the snout.
// IDLING and SEARCHING have no one-shot here (idle rests, search drives the looping search-walk). -----

// `SNIFFER_LONGSNIFF` (length 1.0s, NOT looping): the `nose` puffs `(1, 4, 1)` and the `head` dips.
const SNIFFER_LONGSNIFF_NOSE_SCALE: [Keyframe; 7] = [
    keyframe(0.0, scale_vec(1.0, 1.0, 1.0), CATMULLROM),
    keyframe(0.0833, scale_vec(1.0, 0.7, 1.0), CATMULLROM),
    keyframe(0.125, scale_vec(1.0, 3.0, 1.0), CATMULLROM),
    keyframe(0.25, scale_vec(1.0, 3.0, 1.0), CATMULLROM),
    keyframe(0.7083, scale_vec(1.0, 4.0, 1.0), CATMULLROM),
    keyframe(0.8333, scale_vec(1.0, 1.0, 1.0), CATMULLROM),
    keyframe(1.0, scale_vec(1.0, 1.0, 1.0), CATMULLROM),
];
const SNIFFER_LONGSNIFF_HEAD_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(-5.0, 0.0, 0.0), LINEAR),
    keyframe(0.875, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const SNIFFER_LONGSNIFF_NOSE_CHANNELS: [AnimationChannel; 1] =
    [scale_channel(&SNIFFER_LONGSNIFF_NOSE_SCALE)];
const SNIFFER_LONGSNIFF_HEAD_CHANNELS: [AnimationChannel; 1] = [rot(&SNIFFER_LONGSNIFF_HEAD_ROT)];
const SNIFFER_LONGSNIFF_BONES: [BoneAnimation; 2] = [
    BoneAnimation {
        bone: "nose",
        channels: &SNIFFER_LONGSNIFF_NOSE_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &SNIFFER_LONGSNIFF_HEAD_CHANNELS,
    },
];
/// Vanilla `SnifferAnimation.SNIFFER_LONGSNIFF`: the 1.0s one-shot `SNIFFING` sniff (NOT looping),
/// `longSniffAnimation.apply(sniffingAnimationState, …)`. The `nose` SCALE channel stretches the
/// snout and the `head` ROTATION channel dips it (ADDING onto the look).
pub(in crate::entity_models) const SNIFFER_LONGSNIFF: AnimationDefinition = AnimationDefinition {
    length_seconds: 1.0,
    looping: false,
    bones: &SNIFFER_LONGSNIFF_BONES,
};

// `SNIFFER_SNIFFSNIFF` (length 8.0s, looping): the `nose` SCALE channel bobs the snout (the rest is
// flat), sampled by `sniffSniffAnimation.apply(scentingAnimationState, …)` for SCENTING.
const SNIFFER_SNIFFSNIFF_NOSE_SCALE: [Keyframe; 9] = [
    keyframe(0.0, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(0.5417, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(0.5833, scale_vec(1.0, 0.5, 1.0), CATMULLROM),
    keyframe(0.6667, scale_vec(1.0, 2.5, 1.0), CATMULLROM),
    keyframe(0.7917, scale_vec(1.0, 1.0, 1.0), CATMULLROM),
    keyframe(0.9167, scale_vec(1.0, 1.0, 1.0), CATMULLROM),
    keyframe(1.0, scale_vec(1.0, 3.0, 1.0), CATMULLROM),
    keyframe(1.125, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(2.0, scale_vec(1.0, 1.0, 1.0), LINEAR),
];
const SNIFFER_SNIFFSNIFF_NOSE_CHANNELS: [AnimationChannel; 1] =
    [scale_channel(&SNIFFER_SNIFFSNIFF_NOSE_SCALE)];
const SNIFFER_SNIFFSNIFF_BONES: [BoneAnimation; 1] = [BoneAnimation {
    bone: "nose",
    channels: &SNIFFER_SNIFFSNIFF_NOSE_CHANNELS,
}];
/// Vanilla `SnifferAnimation.SNIFFER_SNIFFSNIFF`: the looping 8.0s scenting bob,
/// `sniffSniffAnimation.apply(scentingAnimationState, …)`. Only the `nose` SCALE channel moves; the
/// renderer wraps the elapsed seconds by the 8.0s length before sampling.
pub(in crate::entity_models) const SNIFFER_SNIFFSNIFF: AnimationDefinition = AnimationDefinition {
    length_seconds: 8.0,
    looping: true,
    bones: &SNIFFER_SNIFFSNIFF_BONES,
};

// `SNIFFER_DIG` (length 8.0s, NOT looping): the `body` sinks/scales, the `head` digs (ROTATION +
// POSITION), the two ears droop, and the six legs fold flat. Sampled by
// `digAnimation.apply(diggingAnimationState, …)` for DIGGING.
const SNIFFER_DIG_BODY_ROT: [Keyframe; 13] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(1.5, 0.0, 0.0), LINEAR),
    keyframe(1.3333, degree_vec(-5.0, 0.0, 0.0), LINEAR),
    keyframe(1.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(2.5, degree_vec(2.5, 0.0, 0.0), LINEAR),
    keyframe(3.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(3.5, degree_vec(2.5, 0.0, 0.0), LINEAR),
    keyframe(4.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(4.5, degree_vec(2.5, 0.0, 0.0), LINEAR),
    keyframe(5.6667, degree_vec(5.0, 0.0, 0.0), LINEAR),
    keyframe(5.8333, degree_vec(-2.5, 0.0, 0.0), LINEAR),
    keyframe(6.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const SNIFFER_DIG_BODY_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.3333, pos_vec(0.0, 1.0, 0.0), LINEAR),
    keyframe(1.5, pos_vec(0.0, -7.0, 0.0), LINEAR),
];
const SNIFFER_DIG_BODY_SCALE: [Keyframe; 4] = [
    keyframe(0.0, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(1.5, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(1.5417, scale_vec(1.04, 0.98, 1.02), LINEAR),
    keyframe(1.5833, scale_vec(1.0, 1.0, 1.0), LINEAR),
];
const SNIFFER_DIG_HEAD_ROT: [Keyframe; 24] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.1667, degree_vec(10.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.4167, degree_vec(-10.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.5833, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.875, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.0833, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.5, degree_vec(47.5, 0.0, 0.0), CATMULLROM),
    keyframe(2.6667, degree_vec(38.44, 0.0, 0.0), CATMULLROM),
    keyframe(2.875, degree_vec(10.95951, 13.57454, -14.93501), CATMULLROM),
    keyframe(3.2083, degree_vec(47.5, 0.0, 0.0), CATMULLROM),
    keyframe(3.5833, degree_vec(55.0, 0.0, 0.0), CATMULLROM),
    keyframe(3.7917, degree_vec(4.2932, -16.187, 10.90042), CATMULLROM),
    keyframe(4.125, degree_vec(47.5, 0.0, 0.0), CATMULLROM),
    keyframe(4.4167, degree_vec(54.71135, 7.98009, -5.56662), CATMULLROM),
    keyframe(4.5, degree_vec(55.72895, -6.77684, 4.46197), CATMULLROM),
    keyframe(4.5833, degree_vec(54.71135, 7.98009, -5.56662), CATMULLROM),
    keyframe(4.6667, degree_vec(55.72895, -6.77684, 4.46197), CATMULLROM),
    keyframe(4.75, degree_vec(54.71135, 7.98009, -5.56662), CATMULLROM),
    keyframe(4.8333, degree_vec(55.72895, -6.77684, 4.46197), CATMULLROM),
    keyframe(5.0, degree_vec(65.0, 0.0, 0.0), CATMULLROM),
    keyframe(5.75, degree_vec(65.0, 0.0, 0.0), CATMULLROM),
    keyframe(5.9167, degree_vec(-32.5, 0.0, 0.0), CATMULLROM),
    keyframe(6.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const SNIFFER_DIG_HEAD_POS: [Keyframe; 16] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.625, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.375, pos_vec(0.0, 1.0, 0.0), LINEAR),
    keyframe(1.5, pos_vec(0.0, 1.0, 0.0), LINEAR),
    keyframe(1.5833, pos_vec(0.0, 1.0, 0.0), LINEAR),
    keyframe(1.875, pos_vec(0.0, 1.0, 0.0), LINEAR),
    keyframe(2.0833, pos_vec(0.0, 3.0, 0.0), LINEAR),
    keyframe(2.2917, pos_vec(0.0, 6.0, 0.0), LINEAR),
    keyframe(2.6667, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(3.2083, pos_vec(0.0, 4.0, 0.0), LINEAR),
    keyframe(3.5833, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(4.125, pos_vec(0.0, 4.0, 0.0), LINEAR),
    keyframe(5.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(5.75, pos_vec(0.0, 1.0, 0.0), LINEAR),
    keyframe(6.0, pos_vec(0.0, 1.5, 0.0), LINEAR),
    keyframe(6.25, pos_vec(0.0, 1.0, 0.0), LINEAR),
];
const SNIFFER_DIG_LEFT_EAR_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, -2.5), LINEAR),
    keyframe(1.25, degree_vec(0.0, 0.0, -2.5), LINEAR),
    keyframe(1.4167, degree_vec(0.0, 0.0, -50.0), LINEAR),
    keyframe(1.5833, degree_vec(0.0, 0.0, -30.0), LINEAR),
    keyframe(5.9167, degree_vec(0.0, 0.0, -30.0), LINEAR),
    keyframe(6.0833, degree_vec(0.0, 0.0, -65.0), LINEAR),
    keyframe(6.3333, degree_vec(0.0, 0.0, -30.0), LINEAR),
];
const SNIFFER_DIG_RIGHT_EAR_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 2.5), LINEAR),
    keyframe(1.25, degree_vec(0.0, 0.0, 2.5), LINEAR),
    keyframe(1.4167, degree_vec(0.0, 0.0, 50.0), LINEAR),
    keyframe(1.5833, degree_vec(0.0, 0.0, 30.0), LINEAR),
    keyframe(5.9167, degree_vec(0.0, 0.0, 30.0), LINEAR),
    keyframe(6.0833, degree_vec(0.0, 0.0, 65.0), LINEAR),
    keyframe(6.3333, degree_vec(0.0, 0.0, 30.0), LINEAR),
];
const SNIFFER_DIG_RIGHT_FRONT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2083, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.375, degree_vec(0.0, 0.0, 90.0), LINEAR),
];
const SNIFFER_DIG_RIGHT_FRONT_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2917, pos_vec(-2.0, -0.75, 0.0), LINEAR),
    keyframe(1.375, pos_vec(-4.0, -5.5, 0.0), LINEAR),
];
const SNIFFER_DIG_RIGHT_MID_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4167, degree_vec(0.0, 0.0, 90.0), LINEAR),
];
const SNIFFER_DIG_RIGHT_MID_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.25, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.3333, pos_vec(-2.0, -0.75, 0.0), LINEAR),
    keyframe(1.4167, pos_vec(-4.0, -5.5, 0.0), LINEAR),
];
const SNIFFER_DIG_RIGHT_HIND_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.3333, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5, degree_vec(0.0, 0.0, 90.0), LINEAR),
];
const SNIFFER_DIG_RIGHT_HIND_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.3333, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4167, pos_vec(-2.0, -0.75, 0.0), LINEAR),
    keyframe(1.5, pos_vec(-4.0, -5.5, 0.0), LINEAR),
];
const SNIFFER_DIG_LEFT_FRONT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2083, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.375, degree_vec(0.0, 0.0, -90.0), LINEAR),
];
const SNIFFER_DIG_LEFT_FRONT_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2917, pos_vec(2.0, -0.75, 0.0), LINEAR),
    keyframe(1.375, pos_vec(4.0, -5.5, 0.0), LINEAR),
];
const SNIFFER_DIG_LEFT_MID_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4167, degree_vec(0.0, 0.0, -90.0), LINEAR),
];
const SNIFFER_DIG_LEFT_MID_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.25, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.3333, pos_vec(2.0, -0.75, 0.0), LINEAR),
    keyframe(1.4167, pos_vec(4.0, -5.5, 0.0), LINEAR),
];
const SNIFFER_DIG_LEFT_HIND_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.3333, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5, degree_vec(0.0, 0.0, -90.0), LINEAR),
];
const SNIFFER_DIG_LEFT_HIND_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.3333, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4167, pos_vec(2.0, -0.75, 0.0), LINEAR),
    keyframe(1.5, pos_vec(4.0, -5.5, 0.0), LINEAR),
];
const SNIFFER_DIG_BODY_CHANNELS: [AnimationChannel; 3] = [
    rot(&SNIFFER_DIG_BODY_ROT),
    pos(&SNIFFER_DIG_BODY_POS),
    scale_channel(&SNIFFER_DIG_BODY_SCALE),
];
const SNIFFER_DIG_HEAD_CHANNELS: [AnimationChannel; 2] =
    [rot(&SNIFFER_DIG_HEAD_ROT), pos(&SNIFFER_DIG_HEAD_POS)];
const SNIFFER_DIG_LEFT_EAR_CHANNELS: [AnimationChannel; 1] = [rot(&SNIFFER_DIG_LEFT_EAR_ROT)];
const SNIFFER_DIG_RIGHT_EAR_CHANNELS: [AnimationChannel; 1] = [rot(&SNIFFER_DIG_RIGHT_EAR_ROT)];
const SNIFFER_DIG_RIGHT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_DIG_RIGHT_FRONT_LEG_ROT),
    pos(&SNIFFER_DIG_RIGHT_FRONT_LEG_POS),
];
const SNIFFER_DIG_RIGHT_MID_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_DIG_RIGHT_MID_LEG_ROT),
    pos(&SNIFFER_DIG_RIGHT_MID_LEG_POS),
];
const SNIFFER_DIG_RIGHT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_DIG_RIGHT_HIND_LEG_ROT),
    pos(&SNIFFER_DIG_RIGHT_HIND_LEG_POS),
];
const SNIFFER_DIG_LEFT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_DIG_LEFT_FRONT_LEG_ROT),
    pos(&SNIFFER_DIG_LEFT_FRONT_LEG_POS),
];
const SNIFFER_DIG_LEFT_MID_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_DIG_LEFT_MID_LEG_ROT),
    pos(&SNIFFER_DIG_LEFT_MID_LEG_POS),
];
const SNIFFER_DIG_LEFT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_DIG_LEFT_HIND_LEG_ROT),
    pos(&SNIFFER_DIG_LEFT_HIND_LEG_POS),
];
const SNIFFER_DIG_BONES: [BoneAnimation; 10] = [
    BoneAnimation {
        bone: "body",
        channels: &SNIFFER_DIG_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &SNIFFER_DIG_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "left_ear",
        channels: &SNIFFER_DIG_LEFT_EAR_CHANNELS,
    },
    BoneAnimation {
        bone: "right_ear",
        channels: &SNIFFER_DIG_RIGHT_EAR_CHANNELS,
    },
    BoneAnimation {
        bone: "right_front_leg",
        channels: &SNIFFER_DIG_RIGHT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_mid_leg",
        channels: &SNIFFER_DIG_RIGHT_MID_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_hind_leg",
        channels: &SNIFFER_DIG_RIGHT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_front_leg",
        channels: &SNIFFER_DIG_LEFT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_mid_leg",
        channels: &SNIFFER_DIG_LEFT_MID_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_hind_leg",
        channels: &SNIFFER_DIG_LEFT_HIND_LEG_CHANNELS,
    },
];
/// Vanilla `SnifferAnimation.SNIFFER_DIG`: the 8.0s one-shot `DIGGING` animation (NOT looping),
/// `digAnimation.apply(diggingAnimationState, …)`. The `body` sinks and squashes, the `head` dives
/// (ROTATION + POSITION, ROTATION ADDING onto the look), the ears droop, and the six legs fold flat.
pub(in crate::entity_models) const SNIFFER_DIG: AnimationDefinition = AnimationDefinition {
    length_seconds: 8.0,
    looping: false,
    bones: &SNIFFER_DIG_BONES,
};

// `SNIFFER_STAND_UP` (length 3.0s, NOT looping): the inverse of the dig — the `body` lifts back up,
// the `head` rocks, the ears un-droop, and the six legs un-fold from flat. Sampled by
// `standUpAnimation.apply(risingAnimationState, …)` for RISING.
const SNIFFER_STAND_UP_BODY_ROT: [Keyframe; 4] = [
    keyframe(0.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(2.5, 0.0, 0.0), LINEAR),
    keyframe(1.5, degree_vec(-2.5, 0.0, 0.0), LINEAR),
    keyframe(1.7083, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const SNIFFER_STAND_UP_BODY_POS: [Keyframe; 4] = [
    keyframe(0.25, pos_vec(0.0, -7.0, 0.0), LINEAR),
    keyframe(0.75, pos_vec(0.0, -7.0, 0.0), LINEAR),
    keyframe(1.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.7083, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const SNIFFER_STAND_UP_HEAD_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.3333, degree_vec(-5.0, 0.0, 0.0), LINEAR),
    keyframe(0.7083, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(10.0, 0.0, 0.0), LINEAR),
    keyframe(1.375, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const SNIFFER_STAND_UP_HEAD_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 1.0, 0.0), LINEAR),
    keyframe(1.375, pos_vec(0.0, 1.0, 0.0), LINEAR),
];
const SNIFFER_STAND_UP_LEFT_EAR_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(0.0, 0.0, -30.0), LINEAR),
    keyframe(0.9167, degree_vec(0.0, 0.0, -30.0), LINEAR),
    keyframe(1.2083, degree_vec(0.0, 0.0, -5.0), LINEAR),
];
const SNIFFER_STAND_UP_RIGHT_EAR_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 30.0), LINEAR),
    keyframe(0.9167, degree_vec(0.0, 0.0, 30.0), LINEAR),
    keyframe(1.2083, degree_vec(0.0, 0.0, 5.0), LINEAR),
];
const SNIFFER_STAND_UP_RIGHT_FRONT_LEG_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 90.0), CATMULLROM),
    keyframe(0.4583, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const SNIFFER_STAND_UP_RIGHT_FRONT_LEG_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(-4.0, -5.5, 0.0), CATMULLROM),
    keyframe(0.2083, pos_vec(6.0, -5.5, 0.0), CATMULLROM),
    keyframe(0.4583, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const SNIFFER_STAND_UP_RIGHT_MID_LEG_ROT: [Keyframe; 2] = [
    keyframe(0.0833, degree_vec(0.0, 0.0, 90.0), CATMULLROM),
    keyframe(0.5833, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const SNIFFER_STAND_UP_RIGHT_MID_LEG_POS: [Keyframe; 3] = [
    keyframe(0.0833, pos_vec(-4.0, -5.5, 0.0), CATMULLROM),
    keyframe(0.3333, pos_vec(6.0, -5.5, 0.0), CATMULLROM),
    keyframe(0.5833, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const SNIFFER_STAND_UP_RIGHT_HIND_LEG_ROT: [Keyframe; 2] = [
    keyframe(0.1667, degree_vec(0.0, 0.0, 90.0), CATMULLROM),
    keyframe(0.6667, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const SNIFFER_STAND_UP_RIGHT_HIND_LEG_POS: [Keyframe; 3] = [
    keyframe(0.1667, pos_vec(-4.0, -5.5, 0.0), CATMULLROM),
    keyframe(0.4167, pos_vec(6.0, -5.5, 0.0), CATMULLROM),
    keyframe(0.6667, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const SNIFFER_STAND_UP_LEFT_FRONT_LEG_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(0.0, 0.0, -90.0), CATMULLROM),
    keyframe(0.4583, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const SNIFFER_STAND_UP_LEFT_FRONT_LEG_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(4.0, -5.5, 0.0), CATMULLROM),
    keyframe(0.2083, pos_vec(-6.0, -5.5, 0.0), CATMULLROM),
    keyframe(0.4583, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const SNIFFER_STAND_UP_LEFT_MID_LEG_ROT: [Keyframe; 2] = [
    keyframe(0.0833, degree_vec(0.0, 0.0, -90.0), CATMULLROM),
    keyframe(0.5833, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const SNIFFER_STAND_UP_LEFT_MID_LEG_POS: [Keyframe; 3] = [
    keyframe(0.0833, pos_vec(4.0, -5.5, 0.0), CATMULLROM),
    keyframe(0.3333, pos_vec(-6.0, -5.5, 0.0), CATMULLROM),
    keyframe(0.5833, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const SNIFFER_STAND_UP_LEFT_HIND_LEG_ROT: [Keyframe; 2] = [
    keyframe(0.1667, degree_vec(0.0, 0.0, -90.0), CATMULLROM),
    keyframe(0.6667, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const SNIFFER_STAND_UP_LEFT_HIND_LEG_POS: [Keyframe; 3] = [
    keyframe(0.1667, pos_vec(4.0, -5.5, 0.0), CATMULLROM),
    keyframe(0.4167, pos_vec(-6.0, -5.5, 0.0), CATMULLROM),
    keyframe(0.6667, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const SNIFFER_STAND_UP_BODY_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_STAND_UP_BODY_ROT),
    pos(&SNIFFER_STAND_UP_BODY_POS),
];
const SNIFFER_STAND_UP_HEAD_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_STAND_UP_HEAD_ROT),
    pos(&SNIFFER_STAND_UP_HEAD_POS),
];
const SNIFFER_STAND_UP_LEFT_EAR_CHANNELS: [AnimationChannel; 1] =
    [rot(&SNIFFER_STAND_UP_LEFT_EAR_ROT)];
const SNIFFER_STAND_UP_RIGHT_EAR_CHANNELS: [AnimationChannel; 1] =
    [rot(&SNIFFER_STAND_UP_RIGHT_EAR_ROT)];
const SNIFFER_STAND_UP_RIGHT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_STAND_UP_RIGHT_FRONT_LEG_ROT),
    pos(&SNIFFER_STAND_UP_RIGHT_FRONT_LEG_POS),
];
const SNIFFER_STAND_UP_RIGHT_MID_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_STAND_UP_RIGHT_MID_LEG_ROT),
    pos(&SNIFFER_STAND_UP_RIGHT_MID_LEG_POS),
];
const SNIFFER_STAND_UP_RIGHT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_STAND_UP_RIGHT_HIND_LEG_ROT),
    pos(&SNIFFER_STAND_UP_RIGHT_HIND_LEG_POS),
];
const SNIFFER_STAND_UP_LEFT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_STAND_UP_LEFT_FRONT_LEG_ROT),
    pos(&SNIFFER_STAND_UP_LEFT_FRONT_LEG_POS),
];
const SNIFFER_STAND_UP_LEFT_MID_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_STAND_UP_LEFT_MID_LEG_ROT),
    pos(&SNIFFER_STAND_UP_LEFT_MID_LEG_POS),
];
const SNIFFER_STAND_UP_LEFT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&SNIFFER_STAND_UP_LEFT_HIND_LEG_ROT),
    pos(&SNIFFER_STAND_UP_LEFT_HIND_LEG_POS),
];
const SNIFFER_STAND_UP_BONES: [BoneAnimation; 10] = [
    BoneAnimation {
        bone: "body",
        channels: &SNIFFER_STAND_UP_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &SNIFFER_STAND_UP_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "left_ear",
        channels: &SNIFFER_STAND_UP_LEFT_EAR_CHANNELS,
    },
    BoneAnimation {
        bone: "right_ear",
        channels: &SNIFFER_STAND_UP_RIGHT_EAR_CHANNELS,
    },
    BoneAnimation {
        bone: "right_front_leg",
        channels: &SNIFFER_STAND_UP_RIGHT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_mid_leg",
        channels: &SNIFFER_STAND_UP_RIGHT_MID_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_hind_leg",
        channels: &SNIFFER_STAND_UP_RIGHT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_front_leg",
        channels: &SNIFFER_STAND_UP_LEFT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_mid_leg",
        channels: &SNIFFER_STAND_UP_LEFT_MID_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_hind_leg",
        channels: &SNIFFER_STAND_UP_LEFT_HIND_LEG_CHANNELS,
    },
];
/// Vanilla `SnifferAnimation.SNIFFER_STAND_UP`: the 3.0s one-shot `RISING` stand-up (NOT looping),
/// `standUpAnimation.apply(risingAnimationState, …)`. The `body` lifts from the dug-in pose, the
/// `head` rocks, the ears un-droop, and the six legs un-fold from the flat dig pose.
pub(in crate::entity_models) const SNIFFER_STAND_UP: AnimationDefinition = AnimationDefinition {
    length_seconds: 3.0,
    looping: false,
    bones: &SNIFFER_STAND_UP_BONES,
};

// `SNIFFER_HAPPY` (length 2.0s, looping): the `head` waggles side to side and the two ears flop.
// Sampled by `happyAnimation.apply(feelingHappyAnimationState, …)` for FEELING_HAPPY.
const SNIFFER_HAPPY_HEAD_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(-32.00206, 19.3546, -11.70092), CATMULLROM),
    keyframe(1.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(-32.00206, -19.3546, 11.70092), CATMULLROM),
    keyframe(2.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const SNIFFER_HAPPY_LEFT_EAR_ROT: [Keyframe; 5] = [
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, degree_vec(0.0, 0.0, -67.5), CATMULLROM),
    keyframe(0.9583, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.125, degree_vec(0.0, 0.0, -67.5), CATMULLROM),
    keyframe(1.2917, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const SNIFFER_HAPPY_RIGHT_EAR_ROT: [Keyframe; 5] = [
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, degree_vec(0.0, 0.0, 67.5), CATMULLROM),
    keyframe(0.9583, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.125, degree_vec(0.0, 0.0, 67.5), CATMULLROM),
    keyframe(1.2917, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const SNIFFER_HAPPY_HEAD_CHANNELS: [AnimationChannel; 1] = [rot(&SNIFFER_HAPPY_HEAD_ROT)];
const SNIFFER_HAPPY_LEFT_EAR_CHANNELS: [AnimationChannel; 1] = [rot(&SNIFFER_HAPPY_LEFT_EAR_ROT)];
const SNIFFER_HAPPY_RIGHT_EAR_CHANNELS: [AnimationChannel; 1] = [rot(&SNIFFER_HAPPY_RIGHT_EAR_ROT)];
const SNIFFER_HAPPY_BONES: [BoneAnimation; 3] = [
    BoneAnimation {
        bone: "head",
        channels: &SNIFFER_HAPPY_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "left_ear",
        channels: &SNIFFER_HAPPY_LEFT_EAR_CHANNELS,
    },
    BoneAnimation {
        bone: "right_ear",
        channels: &SNIFFER_HAPPY_RIGHT_EAR_CHANNELS,
    },
];
/// Vanilla `SnifferAnimation.SNIFFER_HAPPY`: the looping 2.0s feeling-happy waggle,
/// `happyAnimation.apply(feelingHappyAnimationState, …)`. The `head` swings side to side (ADDING
/// onto the look) and the two ears flop; the renderer wraps the elapsed seconds by the 2.0s length.
pub(in crate::entity_models) const SNIFFER_HAPPY: AnimationDefinition = AnimationDefinition {
    length_seconds: 2.0,
    looping: true,
    bones: &SNIFFER_HAPPY_BONES,
};

/// Mutable sniffer model, mirroring vanilla `SnifferModel`. The cubeless `bone` root (parenting the
/// body and the six legs; `body` parents the head, which parents the two ears, nose, and beak) hangs
/// off a synthetic root, built from the baked colored geometry as a named-children tree. Colored-only:
/// `setup_anim` sets the head look, adds the looping `SNIFFER_WALK` cycle onto the body, head, ears,
/// and the six legs, then layers the active synced-state one-shot (dig / long-sniff / stand-up /
/// happy / sniff-sniff) on top. The un-synced search-walk variant (`SNIFFER_SNIFF_SEARCH`) and the
/// baby-transform stay deferred.
pub(in crate::entity_models) struct SnifferModel {
    root: ModelPart,
}

impl SnifferModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: sniffer_root(),
        }
    }
}

impl EntityModel for SnifferModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `SnifferModel.setupAnim` sets `head.xRot/yRot` from the plain look, then runs
        // `applyWalk(..., 9, 100)`: the body sways/dips, the head walk pitch ADDS onto the look, the
        // two ears z-roll, and the six legs swing. A still sniffer samples amplitude 0, collapsing to
        // the bind pose plus the head look. The nose and beak ride the head; the `bone` root holds.
        // Vanilla then `apply`s the active synced-state one-shot (dig/long-sniff/stand-up/happy/
        // sniff-sniff), whose offsets ADD onto the already-walk-posed parts (and onto the look for the
        // head). The projected `sniffer_animation_id` selects the def and `_seconds` the sample time.
        let head_pitch = instance.render_state.head_pitch.to_radians();
        let head_yaw = instance.render_state.head_yaw.to_radians();
        let (seconds, scale) = keyframe_walk_sample(
            &SNIFFER_WALK,
            instance.render_state.walk_animation_pos,
            instance.render_state.walk_animation_speed,
            SNIFFER_WALK_SPEED_FACTOR,
            SNIFFER_WALK_SCALE_FACTOR,
        );
        let animate = |part: &mut ModelPart, bone: &str| {
            let (position, rotation) = sample_bone_offsets(&SNIFFER_WALK, bone, seconds, scale);
            part.pose = keyframe_animated_pose(part.pose, position, rotation);
        };

        // The active synced-state one-shot, sampled at the projected elapsed seconds (wrapped by the
        // def length for the looping happy/sniff-sniff). `None` for an idling/searching sniffer.
        let one_shot =
            sniffer_state_animation(instance.render_state.sniffer_animation_id).map(|definition| {
                (
                    definition,
                    keyframe_elapsed_seconds(
                        definition,
                        instance.render_state.sniffer_animation_seconds,
                    ),
                )
            });
        // Adds the one-shot's position/rotation offsets (and scale) onto a part already walk-posed.
        let apply_one_shot = |part: &mut ModelPart, bone: &str| {
            if let Some((definition, one_shot_seconds)) = one_shot {
                let (position, rotation, scale_offset) =
                    sample_bone_offsets_with_scale(definition, bone, one_shot_seconds, 1.0);
                part.pose = keyframe_animated_pose(part.pose, position, rotation);
                part.scale = keyframe_animated_scale(scale_offset);
            }
        };

        let bone = self.root.child_mut("bone");
        {
            let body = bone.child_mut("body");
            animate(body, "body");
            apply_one_shot(body, "body");

            let head = body.child_mut("head");
            let (_, head_walk_rot) = sample_bone_offsets(&SNIFFER_WALK, "head", seconds, scale);
            head.pose.rotation = [
                head_pitch + head_walk_rot[0],
                head_yaw + head_walk_rot[1],
                head.pose.rotation[2] + head_walk_rot[2],
            ];
            // The one-shot head channels (the dig dive, long-sniff dip, stand-up rock, happy waggle)
            // ADD onto the composed look + walk rotation.
            apply_one_shot(head, "head");

            // The two ears z-roll with the walk; the nose and beak ride the head. The one-shot adds
            // the ear droop/flop and (long-sniff / sniff-sniff) the nose SCALE puff.
            for ear in ["left_ear", "right_ear"] {
                animate(head.child_mut(ear), ear);
                apply_one_shot(head.child_mut(ear), ear);
            }
            apply_one_shot(head.child_mut("nose"), "nose");
        }
        for bone_name in [
            "right_front_leg",
            "right_mid_leg",
            "right_hind_leg",
            "left_front_leg",
            "left_mid_leg",
            "left_hind_leg",
        ] {
            animate(bone.child_mut(bone_name), bone_name);
            apply_one_shot(bone.child_mut(bone_name), bone_name);
        }
    }
}

/// Maps the projected `sniffer_animation_id` (the active `Sniffer.State` ordinal, or `-1` for none)
/// to the one-shot keyframe def `SnifferModel.setupAnim` applies. `None` for `-1` (idling/searching)
/// and any unknown id, so no one-shot is layered onto the walk.
fn sniffer_state_animation(animation_id: i32) -> Option<&'static AnimationDefinition> {
    match animation_id {
        SNIFFER_STATE_FEELING_HAPPY_ID => Some(&SNIFFER_HAPPY),
        SNIFFER_STATE_SCENTING_ID => Some(&SNIFFER_SNIFFSNIFF),
        SNIFFER_STATE_SNIFFING_ID => Some(&SNIFFER_LONGSNIFF),
        SNIFFER_STATE_DIGGING_ID => Some(&SNIFFER_DIG),
        SNIFFER_STATE_RISING_ID => Some(&SNIFFER_STAND_UP),
        _ => None,
    }
}
