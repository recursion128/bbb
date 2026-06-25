use super::super::keyframe::{
    degree_vec, keyframe, keyframe_animated_pose, keyframe_elapsed_seconds, keyframe_walk_sample,
    pos_vec, sample_bone_offsets, AnimationChannel, AnimationDefinition, AnimationTarget,
    BoneAnimation, Keyframe, KeyframeInterpolation,
};
use super::{PartPose, CAMEL_TAN, PART_POSE_ZERO};
use crate::entity_models::catalog::CamelModelFamily;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

const fn camel_cube(min: [f32; 3], size: [f32; 3], tex: [f32; 2]) -> ModelCube {
    // `CubeDeformation.NONE`, so `uv_size == size`; never mirrored. Each cube carries both render
    // paths' data: the colored debug tint (`CAMEL_TAN`) and the textured `uv_size` / `texOffs`.
    ModelCube::new(min, size, CAMEL_TAN, size, tex, false)
}

// Vanilla 26.1 `AdultCamelModel.createBodyMesh` cubes (atlas 128×128). The tail is a zero-thickness
// (depth 0) plane.
pub(in crate::entity_models) const ADULT_CAMEL_BODY: [ModelCube; 1] = [camel_cube(
    [-7.5, -12.0, -23.5],
    [15.0, 12.0, 27.0],
    [0.0, 25.0],
)];
pub(in crate::entity_models) const ADULT_CAMEL_HUMP: [ModelCube; 1] = [camel_cube(
    [-4.5, -5.0, -5.5],
    [9.0, 5.0, 11.0],
    [74.0, 0.0],
)];
pub(in crate::entity_models) const ADULT_CAMEL_TAIL: [ModelCube; 1] =
    [camel_cube([-1.5, 0.0, 0.0], [3.0, 14.0, 0.0], [122.0, 0.0])];
pub(in crate::entity_models) const ADULT_CAMEL_HEAD: [ModelCube; 3] = [
    camel_cube([-3.5, -7.0, -15.0], [7.0, 8.0, 19.0], [60.0, 24.0]),
    camel_cube([-3.5, -21.0, -15.0], [7.0, 14.0, 7.0], [21.0, 0.0]),
    camel_cube([-2.5, -21.0, -21.0], [5.0, 5.0, 6.0], [50.0, 0.0]),
];
pub(in crate::entity_models) const ADULT_CAMEL_LEFT_EAR: [ModelCube; 1] =
    [camel_cube([-0.5, 0.5, -1.0], [3.0, 1.0, 2.0], [45.0, 0.0])];
pub(in crate::entity_models) const ADULT_CAMEL_RIGHT_EAR: [ModelCube; 1] =
    [camel_cube([-2.5, 0.5, -1.0], [3.0, 1.0, 2.0], [67.0, 0.0])];
pub(in crate::entity_models) const ADULT_CAMEL_LEFT_HIND_LEG: [ModelCube; 1] = [camel_cube(
    [-2.5, 2.0, -2.5],
    [5.0, 21.0, 5.0],
    [58.0, 16.0],
)];
pub(in crate::entity_models) const ADULT_CAMEL_RIGHT_HIND_LEG: [ModelCube; 1] = [camel_cube(
    [-2.5, 2.0, -2.5],
    [5.0, 21.0, 5.0],
    [94.0, 16.0],
)];
pub(in crate::entity_models) const ADULT_CAMEL_LEFT_FRONT_LEG: [ModelCube; 1] =
    [camel_cube([-2.5, 2.0, -2.5], [5.0, 21.0, 5.0], [0.0, 0.0])];
pub(in crate::entity_models) const ADULT_CAMEL_RIGHT_FRONT_LEG: [ModelCube; 1] =
    [camel_cube([-2.5, 2.0, -2.5], [5.0, 21.0, 5.0], [0.0, 26.0])];

// Vanilla 26.1 `BabyCamelModel.createBodyLayer` cubes (atlas 64×64). Each leg has its own `texOffs`.
pub(in crate::entity_models) const BABY_CAMEL_BODY: [ModelCube; 1] = [camel_cube(
    [-4.5, -4.0, -8.0],
    [9.0, 8.0, 16.0],
    [0.0, 14.0],
)];
pub(in crate::entity_models) const BABY_CAMEL_TAIL: [ModelCube; 1] =
    [camel_cube([-1.5, -0.5, 0.0], [3.0, 9.0, 0.0], [50.0, 38.0])];
pub(in crate::entity_models) const BABY_CAMEL_HEAD: [ModelCube; 3] = [
    camel_cube([-2.5, -3.0, -7.5], [5.0, 5.0, 7.0], [20.0, 0.0]),
    camel_cube([-2.5, -12.0, -7.5], [5.0, 9.0, 5.0], [0.0, 0.0]),
    camel_cube([-2.5, -12.0, -10.5], [5.0, 4.0, 3.0], [0.0, 14.0]),
];
pub(in crate::entity_models) const BABY_CAMEL_RIGHT_EAR: [ModelCube; 1] =
    [camel_cube([-3.0, -0.5, -1.0], [3.0, 1.0, 2.0], [37.0, 0.0])];
pub(in crate::entity_models) const BABY_CAMEL_LEFT_EAR: [ModelCube; 1] =
    [camel_cube([0.0, -0.5, -1.0], [3.0, 1.0, 2.0], [47.0, 0.0])];
pub(in crate::entity_models) const BABY_CAMEL_RIGHT_FRONT_LEG: [ModelCube; 1] = [camel_cube(
    [-1.5, -0.5, -1.5],
    [3.0, 13.0, 3.0],
    [36.0, 14.0],
)];
pub(in crate::entity_models) const BABY_CAMEL_LEFT_FRONT_LEG: [ModelCube; 1] = [camel_cube(
    [-1.5, -0.5, -1.5],
    [3.0, 13.0, 3.0],
    [48.0, 14.0],
)];
pub(in crate::entity_models) const BABY_CAMEL_LEFT_HIND_LEG: [ModelCube; 1] = [camel_cube(
    [-1.5, -0.5, -1.5],
    [3.0, 13.0, 3.0],
    [12.0, 38.0],
)];
pub(in crate::entity_models) const BABY_CAMEL_RIGHT_HIND_LEG: [ModelCube; 1] = [camel_cube(
    [-1.5, -0.5, -1.5],
    [3.0, 13.0, 3.0],
    [0.0, 38.0],
)];

const fn pose(offset: [f32; 3]) -> PartPose {
    PartPose {
        offset,
        rotation: [0.0, 0.0, 0.0],
    }
}

// Adult part poses (`AdultCamelModel.createBodyLayer`).
pub(in crate::entity_models) const ADULT_CAMEL_BODY_POSE: PartPose = pose([0.0, 4.0, 9.5]);
pub(in crate::entity_models) const ADULT_CAMEL_HUMP_POSE: PartPose = pose([0.0, -12.0, -10.0]);
pub(in crate::entity_models) const ADULT_CAMEL_TAIL_POSE: PartPose = pose([0.0, -9.0, 3.5]);
pub(in crate::entity_models) const ADULT_CAMEL_HEAD_POSE: PartPose = pose([0.0, -3.0, -19.5]);
pub(in crate::entity_models) const ADULT_CAMEL_LEFT_EAR_POSE: PartPose = pose([2.5, -21.0, -9.5]);
pub(in crate::entity_models) const ADULT_CAMEL_RIGHT_EAR_POSE: PartPose = pose([-2.5, -21.0, -9.5]);
pub(in crate::entity_models) const ADULT_CAMEL_LEFT_HIND_LEG_POSE: PartPose = pose([4.9, 1.0, 9.5]);
pub(in crate::entity_models) const ADULT_CAMEL_RIGHT_HIND_LEG_POSE: PartPose =
    pose([-4.9, 1.0, 9.5]);
pub(in crate::entity_models) const ADULT_CAMEL_LEFT_FRONT_LEG_POSE: PartPose =
    pose([4.9, 1.0, -10.5]);
pub(in crate::entity_models) const ADULT_CAMEL_RIGHT_FRONT_LEG_POSE: PartPose =
    pose([-4.9, 1.0, -10.5]);

// Baby part poses (`BabyCamelModel.createBodyLayer`).
pub(in crate::entity_models) const BABY_CAMEL_BODY_POSE: PartPose = pose([0.0, 7.0, 0.0]);
pub(in crate::entity_models) const BABY_CAMEL_TAIL_POSE: PartPose = pose([0.0, -1.5, 8.05]);
pub(in crate::entity_models) const BABY_CAMEL_HEAD_POSE: PartPose = pose([0.0, 1.0, -7.5]);
pub(in crate::entity_models) const BABY_CAMEL_RIGHT_EAR_POSE: PartPose = pose([-2.5, -11.0, -4.0]);
pub(in crate::entity_models) const BABY_CAMEL_LEFT_EAR_POSE: PartPose = pose([2.5, -11.0, -4.0]);
// Vanilla `BabyCamelModel` lists the four legs at these offsets, in the root-child order
// right_front, left_front, left_hind, right_hind.
pub(in crate::entity_models) const BABY_CAMEL_RIGHT_FRONT_LEG_POSE: PartPose =
    pose([-3.0, 11.5, -5.5]);
pub(in crate::entity_models) const BABY_CAMEL_LEFT_FRONT_LEG_POSE: PartPose =
    pose([3.0, 11.5, -5.5]);
pub(in crate::entity_models) const BABY_CAMEL_LEFT_HIND_LEG_POSE: PartPose = pose([3.0, 11.5, 5.5]);
pub(in crate::entity_models) const BABY_CAMEL_RIGHT_HIND_LEG_POSE: PartPose =
    pose([-3.0, 11.5, 5.5]);

/// Vanilla `CamelModel.applyHeadRotation`: the net head look clamped to `yRot ∈ [-30, 30]` and
/// `xRot ∈ [-25, 45]` (a camel turns its long neck only so far) before `head.yRot/xRot` are set from
/// the clamped degrees. Returns the clamped `(yaw, pitch)` in degrees. The transient `jumpCooldown`
/// extra-pitch boost (`45 * jumpCooldown / 55`, re-clamped to `70`) needs the un-projected
/// `jumpCooldown` render state and is deferred.
pub(in crate::entity_models) fn camel_clamped_head_look(
    head_yaw_deg: f32,
    head_pitch_deg: f32,
) -> (f32, f32) {
    (
        head_yaw_deg.clamp(-30.0, 30.0),
        head_pitch_deg.clamp(-25.0, 45.0),
    )
}

// Vanilla 26.1 `ModelLayers.CAMEL` / `CAMEL_BABY` (`CamelRenderer`,
// `CamelHuskRenderer`). The camel husk shares the adult camel's baked mesh; only the
// texture differs, so it reuses the `camel#main` layer/parts.
pub(in crate::entity_models) const MODEL_LAYER_CAMEL: &str = "minecraft:camel#main";
pub(in crate::entity_models) const MODEL_LAYER_CAMEL_BABY: &str = "minecraft:camel_baby#main";

// ----- `CamelAnimation.CAMEL_WALK` (the adult walk; length 1.5s, looping) -----
//
// `CamelModel.setupAnim` samples it via `applyWalk(walkAnimationPos, walkAnimationSpeed, 2.0, 2.5)`.
// The `root` channel rolls the whole model (a CatmullRom z-sway applied at the entity root), the four
// legs swing (rotation + position), the `head` adds a small pitch onto the clamped look, the two ears
// flap (z-roll), and the `tail` swishes. All keyframes are CatmullRom except the two `left_hind_leg`
// closing keyframes. The baby (`CamelBabyAnimation.CAMEL_BABY_WALK`, a different cycle/topology) stays
// deferred. The adult `head`/leg/ear/tail bone names line up with the colored and textured layers.

const LINEAR: KeyframeInterpolation = KeyframeInterpolation::Linear;
const CATMULLROM: KeyframeInterpolation = KeyframeInterpolation::CatmullRom;

const CAMEL_WALK_ROOT_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 2.5), CATMULLROM),
    keyframe(1.0, degree_vec(0.0, 0.0, -2.5), CATMULLROM),
    keyframe(1.5, degree_vec(0.0, 0.0, 2.5), CATMULLROM),
];
const CAMEL_WALK_HEAD_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(2.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.375, degree_vec(-2.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, degree_vec(2.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.125, degree_vec(-2.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(2.5, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_RIGHT_FRONT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, degree_vec(-22.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_RIGHT_FRONT_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.4583, pos_vec(0.0, 4.0, 0.0), CATMULLROM),
    keyframe(0.75, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_LEFT_FRONT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(-22.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(-22.5, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_LEFT_FRONT_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.2083, pos_vec(0.0, 4.0, 0.0), CATMULLROM),
    keyframe(1.5, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_LEFT_HIND_LEG_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(-20.4, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.375, degree_vec(-22.5, 0.0, 0.0), LINEAR),
    keyframe(1.5, degree_vec(-20.4, 0.0, 0.0), LINEAR),
];
const CAMEL_WALK_LEFT_HIND_LEG_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, -0.21, 0.0), CATMULLROM),
    keyframe(0.75, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.0833, pos_vec(0.0, 4.0, 0.0), CATMULLROM),
    keyframe(1.375, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5, pos_vec(0.0, -0.21, 0.0), LINEAR),
];
const CAMEL_WALK_RIGHT_HIND_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.625, degree_vec(-22.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_RIGHT_HIND_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.375, pos_vec(0.0, 4.0, 0.0), CATMULLROM),
    keyframe(0.625, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_LEFT_EAR_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.375, degree_vec(0.0, 0.0, -22.5), CATMULLROM),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.125, degree_vec(0.0, 0.0, -22.5), CATMULLROM),
    keyframe(1.5, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_RIGHT_EAR_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.375, degree_vec(0.0, 0.0, 22.5), CATMULLROM),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.125, degree_vec(0.0, 0.0, 22.5), CATMULLROM),
    keyframe(1.5, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_TAIL_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(15.94102, -8.42106, 20.94102), CATMULLROM),
    keyframe(0.75, degree_vec(15.94102, 8.42106, -20.94102), CATMULLROM),
    keyframe(1.5, degree_vec(15.94102, -8.42106, 20.94102), CATMULLROM),
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

const CAMEL_WALK_ROOT_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_WALK_ROOT_ROT)];
const CAMEL_WALK_HEAD_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_WALK_HEAD_ROT)];
const CAMEL_WALK_RIGHT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_WALK_RIGHT_FRONT_LEG_ROT),
    pos(&CAMEL_WALK_RIGHT_FRONT_LEG_POS),
];
const CAMEL_WALK_LEFT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_WALK_LEFT_FRONT_LEG_ROT),
    pos(&CAMEL_WALK_LEFT_FRONT_LEG_POS),
];
const CAMEL_WALK_LEFT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_WALK_LEFT_HIND_LEG_ROT),
    pos(&CAMEL_WALK_LEFT_HIND_LEG_POS),
];
const CAMEL_WALK_RIGHT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_WALK_RIGHT_HIND_LEG_ROT),
    pos(&CAMEL_WALK_RIGHT_HIND_LEG_POS),
];
const CAMEL_WALK_LEFT_EAR_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_WALK_LEFT_EAR_ROT)];
const CAMEL_WALK_RIGHT_EAR_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_WALK_RIGHT_EAR_ROT)];
const CAMEL_WALK_TAIL_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_WALK_TAIL_ROT)];

const CAMEL_WALK_BONES: [BoneAnimation; 9] = [
    BoneAnimation {
        bone: "root",
        channels: &CAMEL_WALK_ROOT_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &CAMEL_WALK_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_front_leg",
        channels: &CAMEL_WALK_RIGHT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_front_leg",
        channels: &CAMEL_WALK_LEFT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_hind_leg",
        channels: &CAMEL_WALK_LEFT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_hind_leg",
        channels: &CAMEL_WALK_RIGHT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_ear",
        channels: &CAMEL_WALK_LEFT_EAR_CHANNELS,
    },
    BoneAnimation {
        bone: "right_ear",
        channels: &CAMEL_WALK_RIGHT_EAR_CHANNELS,
    },
    BoneAnimation {
        bone: "tail",
        channels: &CAMEL_WALK_TAIL_CHANNELS,
    },
];

/// Vanilla `CamelAnimation.CAMEL_WALK`: the looping 1.5s adult walk cycle, sampled by
/// `CamelModel.setupAnim` via `applyWalk(walkAnimationPos, walkAnimationSpeed, 2.0, 2.5)`. The `root`
/// channel rolls the whole model, the `head` pitch ADDS onto the clamped look, and the legs / ears /
/// tail swing. Mostly CatmullRom (the two closing `left_hind_leg` keyframes are Linear).
pub(in crate::entity_models) const CAMEL_WALK: AnimationDefinition = AnimationDefinition {
    length_seconds: 1.5,
    looping: true,
    bones: &CAMEL_WALK_BONES,
};

/// Vanilla `CamelModel.applyWalk(..., 2.0F, 2.5F)` factors (`MAX_WALK_ANIMATION_SPEED` drives the
/// sample time, `WALK_ANIMATION_SCALE_FACTOR` the amplitude). The base `CamelModel` passes these for
/// both the adult and the baby walk.
pub(in crate::entity_models) const CAMEL_WALK_SPEED_FACTOR: f32 = 2.0;
pub(in crate::entity_models) const CAMEL_WALK_SCALE_FACTOR: f32 = 2.5;

// ----- `CamelBabyAnimation.CAMEL_BABY_WALK` (the baby walk; length 1.5s, looping) -----
//
// The baby walk animates the same nine bones as the adult plus a `body` position dip and a `head`
// position nudge (the adult had neither). The baby leg/ear order differs from the adult, but the bone
// names match, so the named-children tree drives both. Sampled like the adult via `applyWalk(..., 2.0,
// 2.5)`.

const CAMEL_BABY_WALK_ROOT_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 2.5), LINEAR),
    keyframe(0.75, degree_vec(0.0, 0.0, -2.5), CATMULLROM),
    keyframe(1.5, degree_vec(0.0, 0.0, 2.5), LINEAR),
];
const CAMEL_BABY_WALK_HEAD_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(2.5, 0.0, 0.0), LINEAR),
    keyframe(0.375, degree_vec(-2.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, degree_vec(2.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.125, degree_vec(-2.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(2.5, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_HEAD_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.4583, pos_vec(0.0, 0.0, 0.1), LINEAR),
];
const CAMEL_BABY_WALK_RIGHT_FRONT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(-22.5, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(-22.5, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_RIGHT_FRONT_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.075, 0.0, 0.0), LINEAR),
    keyframe(0.75, pos_vec(0.075, 0.0, 0.0), CATMULLROM),
    keyframe(1.2083, pos_vec(0.075, 4.0, 0.0), CATMULLROM),
    keyframe(1.5, pos_vec(0.075, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_LEFT_FRONT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(22.5, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(-22.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(22.5, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_LEFT_FRONT_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(-0.1, 0.0, 0.0), LINEAR),
    keyframe(0.4583, pos_vec(-0.1, 4.0, 0.0), CATMULLROM),
    keyframe(0.75, pos_vec(-0.1, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, pos_vec(-0.1, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_LEFT_HIND_LEG_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(22.5, 0.0, 0.0), LINEAR),
    keyframe(0.375, degree_vec(-9.49, 0.0, 0.0), CATMULLROM),
    keyframe(0.5833, degree_vec(-17.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.2083, degree_vec(7.38, 0.0, 0.0), LINEAR),
    keyframe(1.5, degree_vec(22.5, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_LEFT_HIND_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(-0.1, 0.0, 0.0), LINEAR),
    keyframe(0.25, pos_vec(-0.1, 5.0, 0.0), CATMULLROM),
    keyframe(0.5833, pos_vec(-0.1, 0.0, -0.1), CATMULLROM),
    keyframe(1.5, pos_vec(-0.1, 0.0, 0.0), CATMULLROM),
];
const CAMEL_BABY_WALK_RIGHT_HIND_LEG_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(-15.83, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.0, degree_vec(-7.38, 0.0, 0.0), CATMULLROM),
    keyframe(1.25, degree_vec(-21.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(-15.83, 0.0, 0.0), CATMULLROM),
];
const CAMEL_BABY_WALK_RIGHT_HIND_LEG_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.1, 0.0, 0.0), LINEAR),
    keyframe(0.6667, pos_vec(0.1, 0.0, 0.0), CATMULLROM),
    keyframe(1.0, pos_vec(0.1, 4.0, 0.17), CATMULLROM),
    keyframe(1.2083, pos_vec(0.1, 0.0, -0.11), CATMULLROM),
    keyframe(1.5, pos_vec(0.1, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_LEFT_EAR_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.375, degree_vec(0.0, 0.0, 22.5), CATMULLROM),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.125, degree_vec(0.0, 0.0, 22.5), CATMULLROM),
    keyframe(1.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_RIGHT_EAR_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.375, degree_vec(0.0, 0.0, -22.5), CATMULLROM),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.125, degree_vec(0.0, 0.0, -22.5), CATMULLROM),
    keyframe(1.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_TAIL_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(15.94, -8.42, 20.94), LINEAR),
    keyframe(0.75, degree_vec(15.94, 8.42, -20.94), CATMULLROM),
    keyframe(1.5, degree_vec(15.94, -8.42, 20.94), LINEAR),
];
const CAMEL_BABY_WALK_BODY_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, -0.6, 0.0), LINEAR),
    keyframe(0.4583, pos_vec(0.0, -0.6, 0.0), LINEAR),
];

const CAMEL_BABY_WALK_ROOT_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_BABY_WALK_ROOT_ROT)];
const CAMEL_BABY_WALK_HEAD_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_BABY_WALK_HEAD_ROT),
    pos(&CAMEL_BABY_WALK_HEAD_POS),
];
const CAMEL_BABY_WALK_RIGHT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_BABY_WALK_RIGHT_FRONT_LEG_ROT),
    pos(&CAMEL_BABY_WALK_RIGHT_FRONT_LEG_POS),
];
const CAMEL_BABY_WALK_LEFT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_BABY_WALK_LEFT_FRONT_LEG_ROT),
    pos(&CAMEL_BABY_WALK_LEFT_FRONT_LEG_POS),
];
const CAMEL_BABY_WALK_LEFT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_BABY_WALK_LEFT_HIND_LEG_ROT),
    pos(&CAMEL_BABY_WALK_LEFT_HIND_LEG_POS),
];
const CAMEL_BABY_WALK_RIGHT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_BABY_WALK_RIGHT_HIND_LEG_ROT),
    pos(&CAMEL_BABY_WALK_RIGHT_HIND_LEG_POS),
];
const CAMEL_BABY_WALK_LEFT_EAR_CHANNELS: [AnimationChannel; 1] =
    [rot(&CAMEL_BABY_WALK_LEFT_EAR_ROT)];
const CAMEL_BABY_WALK_RIGHT_EAR_CHANNELS: [AnimationChannel; 1] =
    [rot(&CAMEL_BABY_WALK_RIGHT_EAR_ROT)];
const CAMEL_BABY_WALK_TAIL_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_BABY_WALK_TAIL_ROT)];
const CAMEL_BABY_WALK_BODY_CHANNELS: [AnimationChannel; 1] = [pos(&CAMEL_BABY_WALK_BODY_POS)];

const CAMEL_BABY_WALK_BONES: [BoneAnimation; 10] = [
    BoneAnimation {
        bone: "root",
        channels: &CAMEL_BABY_WALK_ROOT_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &CAMEL_BABY_WALK_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_front_leg",
        channels: &CAMEL_BABY_WALK_RIGHT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_front_leg",
        channels: &CAMEL_BABY_WALK_LEFT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_hind_leg",
        channels: &CAMEL_BABY_WALK_LEFT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_hind_leg",
        channels: &CAMEL_BABY_WALK_RIGHT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_ear",
        channels: &CAMEL_BABY_WALK_LEFT_EAR_CHANNELS,
    },
    BoneAnimation {
        bone: "right_ear",
        channels: &CAMEL_BABY_WALK_RIGHT_EAR_CHANNELS,
    },
    BoneAnimation {
        bone: "tail",
        channels: &CAMEL_BABY_WALK_TAIL_CHANNELS,
    },
    BoneAnimation {
        bone: "body",
        channels: &CAMEL_BABY_WALK_BODY_CHANNELS,
    },
];

/// Vanilla `CamelBabyAnimation.CAMEL_BABY_WALK`: the looping 1.5s baby walk cycle, sampled like the
/// adult via `applyWalk(walkAnimationPos, walkAnimationSpeed, 2.0, 2.5)`. Adds a `body` y-dip and a
/// `head` position nudge the adult lacks, and reorders the legs/ears (but the bone names match the
/// named-children tree).
pub(in crate::entity_models) const CAMEL_BABY_WALK: AnimationDefinition = AnimationDefinition {
    length_seconds: 1.5,
    looping: true,
    bones: &CAMEL_BABY_WALK_BONES,
};

// ----- `CamelAnimation.CAMEL_SIT` (the sit-down transition; length 2.0s, NOT looping) -----
//
// Folds the camel down: the `body` pitches and drops `y -19.9`, the four legs tuck under
// (rotation + position), the `head` dips and returns, and the `tail` flicks up. `CamelModel.setupAnim`
// applies it ADDITIVELY (`sitAnimation.apply(sitAnimationState, ageInTicks)`) onto the walk pose; the
// renderer samples it while `camel_sit_seconds >= 0` (clamping past 2.0 s to the seated final frame).

const CAMEL_SIT_BODY_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.3, degree_vec(30.0, 0.0, 0.0), LINEAR),
    keyframe(1.8, degree_vec(24.0, 0.0, 0.0), LINEAR),
    keyframe(2.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_SIT_BODY_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.3, pos_vec(0.0, 0.0, 1.0), LINEAR),
    keyframe(1.8, pos_vec(0.0, -6.0, 1.0), LINEAR),
    keyframe(2.0, pos_vec(0.0, -19.9, 0.0), LINEAR),
];
const CAMEL_SIT_RIGHT_FRONT_LEG_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(-30.0, 0.0, 0.0), LINEAR),
    keyframe(1.5, degree_vec(-30.0, 0.0, 0.0), LINEAR),
    keyframe(2.0, degree_vec(-90.0, 10.0, 0.0), LINEAR),
];
const CAMEL_SIT_RIGHT_FRONT_LEG_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, -2.0, 11.0), LINEAR),
    keyframe(1.5, pos_vec(0.0, -2.0, 11.0), LINEAR),
    keyframe(1.7, pos_vec(0.0, -8.4, 11.4), LINEAR),
    keyframe(2.0, pos_vec(0.0, -20.6, 12.0), LINEAR),
];
const CAMEL_SIT_LEFT_FRONT_LEG_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(-30.0, 0.0, 0.0), LINEAR),
    keyframe(1.5, degree_vec(-30.0, 0.0, 0.0), LINEAR),
    keyframe(2.0, degree_vec(-90.0, -10.0, 0.0), LINEAR),
];
const CAMEL_SIT_LEFT_FRONT_LEG_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, -2.0, 11.0), LINEAR),
    keyframe(1.5, pos_vec(0.0, -2.0, 11.0), LINEAR),
    keyframe(1.7, pos_vec(0.0, -8.4, 11.4), LINEAR),
    keyframe(2.0, pos_vec(0.0, -20.6, 12.0), LINEAR),
];
const CAMEL_SIT_LEFT_HIND_LEG_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(1.7, degree_vec(-15.0, -3.0, 0.0), LINEAR),
    keyframe(1.9, degree_vec(-65.0, -9.0, 0.0), LINEAR),
    keyframe(2.0, degree_vec(-90.0, -15.0, 0.0), LINEAR),
];
const CAMEL_SIT_LEFT_HIND_LEG_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5, pos_vec(0.0, 0.0, 1.0), LINEAR),
    keyframe(1.7, pos_vec(1.0, -0.62, 0.25), LINEAR),
    keyframe(1.9, pos_vec(0.5, -11.25, 2.5), LINEAR),
    keyframe(2.0, pos_vec(1.0, -20.5, 5.0), LINEAR),
];
const CAMEL_SIT_RIGHT_HIND_LEG_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5, degree_vec(-10.0, 0.0, 0.0), LINEAR),
    keyframe(1.7, degree_vec(-15.0, 3.0, 0.0), LINEAR),
    keyframe(1.9, degree_vec(-65.0, 9.0, 0.0), LINEAR),
    keyframe(2.0, degree_vec(-90.0, 15.0, 0.0), LINEAR),
];
const CAMEL_SIT_RIGHT_HIND_LEG_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5, pos_vec(0.0, 0.0, 1.0), LINEAR),
    keyframe(1.7, pos_vec(-1.0, -0.62, 0.25), LINEAR),
    keyframe(1.9, pos_vec(-0.5, -11.25, 2.5), LINEAR),
    keyframe(2.0, pos_vec(-1.0, -20.5, 5.0), LINEAR),
];
const CAMEL_SIT_HEAD_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.7, degree_vec(-27.5, 0.0, 0.0), LINEAR),
    keyframe(1.5, degree_vec(-21.25, 0.0, 0.0), LINEAR),
    keyframe(2.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_SIT_TAIL_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(5.0, 0.0, 0.0), LINEAR),
    keyframe(1.7, degree_vec(5.0, 0.0, 0.0), LINEAR),
    keyframe(1.9, degree_vec(80.0, 0.0, 0.0), LINEAR),
    keyframe(2.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
];

const CAMEL_SIT_BODY_CHANNELS: [AnimationChannel; 2] =
    [rot(&CAMEL_SIT_BODY_ROT), pos(&CAMEL_SIT_BODY_POS)];
const CAMEL_SIT_RIGHT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_SIT_RIGHT_FRONT_LEG_ROT),
    pos(&CAMEL_SIT_RIGHT_FRONT_LEG_POS),
];
const CAMEL_SIT_LEFT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_SIT_LEFT_FRONT_LEG_ROT),
    pos(&CAMEL_SIT_LEFT_FRONT_LEG_POS),
];
const CAMEL_SIT_LEFT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_SIT_LEFT_HIND_LEG_ROT),
    pos(&CAMEL_SIT_LEFT_HIND_LEG_POS),
];
const CAMEL_SIT_RIGHT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_SIT_RIGHT_HIND_LEG_ROT),
    pos(&CAMEL_SIT_RIGHT_HIND_LEG_POS),
];
const CAMEL_SIT_HEAD_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_SIT_HEAD_ROT)];
const CAMEL_SIT_TAIL_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_SIT_TAIL_ROT)];

const CAMEL_SIT_BONES: [BoneAnimation; 7] = [
    BoneAnimation {
        bone: "body",
        channels: &CAMEL_SIT_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "right_front_leg",
        channels: &CAMEL_SIT_RIGHT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_front_leg",
        channels: &CAMEL_SIT_LEFT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_hind_leg",
        channels: &CAMEL_SIT_LEFT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_hind_leg",
        channels: &CAMEL_SIT_RIGHT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &CAMEL_SIT_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "tail",
        channels: &CAMEL_SIT_TAIL_CHANNELS,
    },
];

/// Vanilla `CamelAnimation.CAMEL_SIT`: the 2.0 s sit-down transition (NOT looping), sampled by
/// `CamelModel.setupAnim` via `sitAnimation.apply(sitAnimationState, ageInTicks)` while the camel is
/// visually sitting down. All keyframes are Linear. The renderer applies it ADDITIVELY when
/// `camel_sit_seconds >= 0`, clamping past 2.0 s to the seated final frame.
pub(in crate::entity_models) const CAMEL_SIT: AnimationDefinition = AnimationDefinition {
    length_seconds: 2.0,
    looping: false,
    bones: &CAMEL_SIT_BONES,
};

// ----- `CamelAnimation.CAMEL_SIT_POSE` (the steady seated hold; length 1.0s, NOT looping) -----
//
// Holds the camel in the seated pose: every channel is a constant pair of identical keyframes (the
// final frame of `CAMEL_SIT`). `CamelModel.setupAnim` applies it ADDITIVELY while the camel holds the
// seated pose (past the sit-down window). All keyframes are Linear.

const CAMEL_SIT_POSE_BODY_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_SIT_POSE_BODY_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, -19.9, 0.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, -19.9, 0.0), LINEAR),
];
const CAMEL_SIT_POSE_RIGHT_FRONT_LEG_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(-90.0, 10.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(-90.0, 10.0, 0.0), LINEAR),
];
const CAMEL_SIT_POSE_RIGHT_FRONT_LEG_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, -20.6, 12.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, -20.6, 12.0), LINEAR),
];
const CAMEL_SIT_POSE_LEFT_FRONT_LEG_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(-90.0, -10.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(-90.0, -10.0, 0.0), LINEAR),
];
const CAMEL_SIT_POSE_LEFT_FRONT_LEG_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, -20.6, 12.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, -20.6, 12.0), LINEAR),
];
const CAMEL_SIT_POSE_LEFT_HIND_LEG_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(-90.0, -15.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(-90.0, -15.0, 0.0), LINEAR),
];
const CAMEL_SIT_POSE_LEFT_HIND_LEG_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(1.0, -20.5, 5.0), LINEAR),
    keyframe(1.0, pos_vec(1.0, -20.5, 5.0), LINEAR),
];
const CAMEL_SIT_POSE_RIGHT_HIND_LEG_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(-90.0, 15.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(-90.0, 15.0, 0.0), LINEAR),
];
const CAMEL_SIT_POSE_RIGHT_HIND_LEG_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(-1.0, -20.5, 5.0), LINEAR),
    keyframe(1.0, pos_vec(-1.0, -20.5, 5.0), LINEAR),
];
const CAMEL_SIT_POSE_HEAD_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_SIT_POSE_TAIL_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
];

const CAMEL_SIT_POSE_BODY_CHANNELS: [AnimationChannel; 2] =
    [rot(&CAMEL_SIT_POSE_BODY_ROT), pos(&CAMEL_SIT_POSE_BODY_POS)];
const CAMEL_SIT_POSE_RIGHT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_SIT_POSE_RIGHT_FRONT_LEG_ROT),
    pos(&CAMEL_SIT_POSE_RIGHT_FRONT_LEG_POS),
];
const CAMEL_SIT_POSE_LEFT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_SIT_POSE_LEFT_FRONT_LEG_ROT),
    pos(&CAMEL_SIT_POSE_LEFT_FRONT_LEG_POS),
];
const CAMEL_SIT_POSE_LEFT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_SIT_POSE_LEFT_HIND_LEG_ROT),
    pos(&CAMEL_SIT_POSE_LEFT_HIND_LEG_POS),
];
const CAMEL_SIT_POSE_RIGHT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_SIT_POSE_RIGHT_HIND_LEG_ROT),
    pos(&CAMEL_SIT_POSE_RIGHT_HIND_LEG_POS),
];
const CAMEL_SIT_POSE_HEAD_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_SIT_POSE_HEAD_ROT)];
const CAMEL_SIT_POSE_TAIL_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_SIT_POSE_TAIL_ROT)];

const CAMEL_SIT_POSE_BONES: [BoneAnimation; 7] = [
    BoneAnimation {
        bone: "body",
        channels: &CAMEL_SIT_POSE_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "right_front_leg",
        channels: &CAMEL_SIT_POSE_RIGHT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_front_leg",
        channels: &CAMEL_SIT_POSE_LEFT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_hind_leg",
        channels: &CAMEL_SIT_POSE_LEFT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_hind_leg",
        channels: &CAMEL_SIT_POSE_RIGHT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &CAMEL_SIT_POSE_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "tail",
        channels: &CAMEL_SIT_POSE_TAIL_CHANNELS,
    },
];

/// Vanilla `CamelAnimation.CAMEL_SIT_POSE`: the 1.0 s steady seated hold (NOT looping), sampled by
/// `CamelModel.setupAnim` via `sitPoseAnimation.apply(sitPoseAnimationState, ageInTicks)` while the
/// camel holds the seated pose past the sit-down window. Every channel is a constant pair (the seated
/// final frame of `CAMEL_SIT`). The renderer applies it ADDITIVELY when `camel_sit_pose_seconds >= 0`.
pub(in crate::entity_models) const CAMEL_SIT_POSE: AnimationDefinition = AnimationDefinition {
    length_seconds: 1.0,
    looping: false,
    bones: &CAMEL_SIT_POSE_BONES,
};

// ----- `CamelAnimation.CAMEL_STANDUP` (the stand-up transition; length 2.6s, NOT looping) -----
//
// Unfolds the camel back to standing: the `body` rises and rolls forward, the four legs extend, the
// `head` swings up and down, and the `tail` settles. `CamelModel.setupAnim` applies it ADDITIVELY
// while the camel is in the stand-up transition. The `body` keyframes mix CatmullRom and Linear; the
// rest are Linear.

const CAMEL_STANDUP_BODY_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.7, degree_vec(-17.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.8, degree_vec(-17.83, 0.0, 0.0), CATMULLROM),
    keyframe(2.3, degree_vec(-5.83, 0.0, 0.0), LINEAR),
    keyframe(2.6, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_STANDUP_BODY_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, -19.9, 0.0), LINEAR),
    keyframe(0.7, pos_vec(0.0, -19.9, -3.0), LINEAR),
    keyframe(1.4, pos_vec(0.0, -12.76, -4.0), CATMULLROM),
    keyframe(1.8, pos_vec(0.0, -10.1, -4.0), CATMULLROM),
    keyframe(2.3, pos_vec(0.0, -2.9, -2.0), LINEAR),
    keyframe(2.6, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_STANDUP_RIGHT_FRONT_LEG_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(-90.0, 10.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(-90.0, 10.0, 0.0), LINEAR),
    keyframe(1.1, degree_vec(-49.06, 10.0, 0.0), LINEAR),
    keyframe(1.8, degree_vec(-22.5, 10.0, 0.0), LINEAR),
    keyframe(2.3, degree_vec(-25.0, 10.0, 0.0), LINEAR),
    keyframe(2.6, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_STANDUP_RIGHT_FRONT_LEG_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, -20.6, 12.0), LINEAR),
    keyframe(0.5, pos_vec(0.0, -20.6, 8.0), LINEAR),
    keyframe(1.1, pos_vec(0.0, -7.14, 4.42), LINEAR),
    keyframe(1.8, pos_vec(0.0, -1.27, -1.33), LINEAR),
    keyframe(2.3, pos_vec(0.0, -1.27, -0.33), LINEAR),
    keyframe(2.6, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_STANDUP_LEFT_FRONT_LEG_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(-90.0, -10.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(-90.0, -10.0, 0.0), LINEAR),
    keyframe(1.1, degree_vec(-49.06, -10.0, 0.0), LINEAR),
    keyframe(1.8, degree_vec(-22.5, -10.0, 0.0), LINEAR),
    keyframe(2.3, degree_vec(-25.0, -10.0, 0.0), LINEAR),
    keyframe(2.6, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_STANDUP_LEFT_FRONT_LEG_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, -20.6, 12.0), LINEAR),
    keyframe(0.5, pos_vec(0.0, -20.6, 8.0), LINEAR),
    keyframe(1.1, pos_vec(0.0, -7.14, 4.42), LINEAR),
    keyframe(1.8, pos_vec(0.0, -1.27, -1.33), LINEAR),
    keyframe(2.3, pos_vec(0.0, -1.27, -0.33), LINEAR),
    keyframe(2.6, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_STANDUP_LEFT_HIND_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(-90.0, -15.0, 0.0), LINEAR),
    keyframe(0.3, degree_vec(-90.0, 0.0, 0.0), LINEAR),
    keyframe(0.6, degree_vec(-90.0, 0.0, 0.0), LINEAR),
    keyframe(1.1, degree_vec(-60.0, 0.0, 0.0), LINEAR),
    keyframe(1.9, degree_vec(35.0, 0.0, 0.0), LINEAR),
    keyframe(2.2, degree_vec(30.0, 0.0, 0.0), LINEAR),
    keyframe(2.6, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_STANDUP_LEFT_HIND_LEG_POS: [Keyframe; 8] = [
    keyframe(0.0, pos_vec(1.0, -20.5, 5.0), LINEAR),
    keyframe(0.3, pos_vec(-2.0, -20.5, 3.0), LINEAR),
    keyframe(0.6, pos_vec(-2.0, -20.5, 3.0), LINEAR),
    keyframe(1.1, pos_vec(-2.0, -10.5, 2.0), LINEAR),
    keyframe(1.5, pos_vec(-2.0, -0.4, -3.9), LINEAR),
    keyframe(1.9, pos_vec(-2.0, -4.3, -9.8), LINEAR),
    keyframe(2.2, pos_vec(-1.0, -2.5, -5.0), LINEAR),
    keyframe(2.6, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_STANDUP_RIGHT_HIND_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(-90.0, 15.0, 0.0), LINEAR),
    keyframe(0.3, degree_vec(-90.0, 0.0, 0.0), LINEAR),
    keyframe(0.6, degree_vec(-90.0, 0.0, 0.0), LINEAR),
    keyframe(1.1, degree_vec(-60.0, 0.0, 0.0), LINEAR),
    keyframe(1.9, degree_vec(35.0, 0.0, 0.0), LINEAR),
    keyframe(2.2, degree_vec(30.0, 0.0, 0.0), LINEAR),
    keyframe(2.6, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_STANDUP_RIGHT_HIND_LEG_POS: [Keyframe; 8] = [
    keyframe(0.0, pos_vec(-1.0, -20.5, 5.0), LINEAR),
    keyframe(0.3, pos_vec(2.0, -20.5, 3.0), LINEAR),
    keyframe(0.6, pos_vec(2.0, -20.5, 3.0), LINEAR),
    keyframe(1.1, pos_vec(2.0, -10.5, 2.0), LINEAR),
    keyframe(1.5, pos_vec(2.0, -0.4, -3.9), LINEAR),
    keyframe(1.9, pos_vec(2.0, -4.3, -9.8), LINEAR),
    keyframe(2.2, pos_vec(1.0, -2.5, -5.0), LINEAR),
    keyframe(2.6, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_STANDUP_HEAD_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.3, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.8, degree_vec(55.0, 0.0, 0.0), LINEAR),
    keyframe(2.0, degree_vec(65.0, 0.0, 0.0), LINEAR),
    keyframe(2.4, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_STANDUP_TAIL_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(0.4, degree_vec(55.0, 0.0, 0.0), LINEAR),
    keyframe(0.9, degree_vec(55.0, 0.0, 0.0), LINEAR),
    keyframe(1.5, degree_vec(17.5, 0.0, 0.0), LINEAR),
    keyframe(2.6, degree_vec(5.0, 0.0, 0.0), LINEAR),
];

const CAMEL_STANDUP_BODY_CHANNELS: [AnimationChannel; 2] =
    [rot(&CAMEL_STANDUP_BODY_ROT), pos(&CAMEL_STANDUP_BODY_POS)];
const CAMEL_STANDUP_RIGHT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_STANDUP_RIGHT_FRONT_LEG_ROT),
    pos(&CAMEL_STANDUP_RIGHT_FRONT_LEG_POS),
];
const CAMEL_STANDUP_LEFT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_STANDUP_LEFT_FRONT_LEG_ROT),
    pos(&CAMEL_STANDUP_LEFT_FRONT_LEG_POS),
];
const CAMEL_STANDUP_LEFT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_STANDUP_LEFT_HIND_LEG_ROT),
    pos(&CAMEL_STANDUP_LEFT_HIND_LEG_POS),
];
const CAMEL_STANDUP_RIGHT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_STANDUP_RIGHT_HIND_LEG_ROT),
    pos(&CAMEL_STANDUP_RIGHT_HIND_LEG_POS),
];
const CAMEL_STANDUP_HEAD_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_STANDUP_HEAD_ROT)];
const CAMEL_STANDUP_TAIL_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_STANDUP_TAIL_ROT)];

const CAMEL_STANDUP_BONES: [BoneAnimation; 7] = [
    BoneAnimation {
        bone: "body",
        channels: &CAMEL_STANDUP_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "right_front_leg",
        channels: &CAMEL_STANDUP_RIGHT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_front_leg",
        channels: &CAMEL_STANDUP_LEFT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_hind_leg",
        channels: &CAMEL_STANDUP_LEFT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_hind_leg",
        channels: &CAMEL_STANDUP_RIGHT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &CAMEL_STANDUP_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "tail",
        channels: &CAMEL_STANDUP_TAIL_CHANNELS,
    },
];

/// Vanilla `CamelAnimation.CAMEL_STANDUP`: the 2.6 s stand-up transition (NOT looping), sampled by
/// `CamelModel.setupAnim` via `standupAnimation.apply(sitUpAnimationState, ageInTicks)` while the
/// camel is in the stand-up pose transition. The `body` keyframes mix CatmullRom and Linear; the rest
/// are Linear. The renderer applies it ADDITIVELY when `camel_standup_seconds >= 0`, clamping past
/// 2.6 s to the standing final frame.
pub(in crate::entity_models) const CAMEL_STANDUP: AnimationDefinition = AnimationDefinition {
    length_seconds: 2.6,
    looping: false,
    bones: &CAMEL_STANDUP_BONES,
};

// ----- `CamelAnimation.CAMEL_DASH` (the gallop; length 0.5s, LOOPING) -----
//
// All channels are ROTATION. The `body` and the two ears hold a constant offset (two Linear
// keyframes); the `head`, `tail`, and four legs swing across 5 CatmullRom keyframes. Applied
// ADDITIVELY (last, over the walk pose) while the camel is dashing.

const CAMEL_DASH_BODY_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(5.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(5.0, 0.0, 0.0), LINEAR),
];
const CAMEL_DASH_TAIL_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(67.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.125, degree_vec(112.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, degree_vec(67.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.375, degree_vec(112.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.5, degree_vec(67.5, 0.0, 0.0), CATMULLROM),
];
const CAMEL_DASH_HEAD_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(10.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, degree_vec(10.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.375, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.5, degree_vec(10.0, 0.0, 0.0), CATMULLROM),
];
const CAMEL_DASH_RIGHT_FRONT_LEG_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(44.97272, 1.76749, -1.76833), CATMULLROM),
    keyframe(0.125, degree_vec(-90.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, degree_vec(44.97272, 1.76749, -1.76833), CATMULLROM),
    keyframe(0.375, degree_vec(-90.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.5, degree_vec(44.97272, 1.76749, -1.76833), CATMULLROM),
];
const CAMEL_DASH_LEFT_FRONT_LEG_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(-90.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.125, degree_vec(44.97272, -1.76749, 1.76833), CATMULLROM),
    keyframe(0.25, degree_vec(-90.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.375, degree_vec(44.97272, -1.76749, 1.76833), CATMULLROM),
    keyframe(0.5, degree_vec(-90.0, 0.0, 0.0), CATMULLROM),
];
const CAMEL_DASH_LEFT_HIND_LEG_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(90.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.125, degree_vec(-45.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, degree_vec(90.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.375, degree_vec(-45.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.5, degree_vec(90.0, 0.0, 0.0), CATMULLROM),
];
const CAMEL_DASH_RIGHT_HIND_LEG_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(-45.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.125, degree_vec(90.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, degree_vec(-45.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.375, degree_vec(90.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.5, degree_vec(-45.0, 0.0, 0.0), CATMULLROM),
];
const CAMEL_DASH_LEFT_EAR_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(0.0, -67.5, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, -67.5, 0.0), LINEAR),
];
const CAMEL_DASH_RIGHT_EAR_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(0.0, 67.5, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 67.5, 0.0), LINEAR),
];

const CAMEL_DASH_BODY_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_DASH_BODY_ROT)];
const CAMEL_DASH_TAIL_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_DASH_TAIL_ROT)];
const CAMEL_DASH_HEAD_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_DASH_HEAD_ROT)];
const CAMEL_DASH_RIGHT_FRONT_LEG_CHANNELS: [AnimationChannel; 1] =
    [rot(&CAMEL_DASH_RIGHT_FRONT_LEG_ROT)];
const CAMEL_DASH_LEFT_FRONT_LEG_CHANNELS: [AnimationChannel; 1] =
    [rot(&CAMEL_DASH_LEFT_FRONT_LEG_ROT)];
const CAMEL_DASH_LEFT_HIND_LEG_CHANNELS: [AnimationChannel; 1] =
    [rot(&CAMEL_DASH_LEFT_HIND_LEG_ROT)];
const CAMEL_DASH_RIGHT_HIND_LEG_CHANNELS: [AnimationChannel; 1] =
    [rot(&CAMEL_DASH_RIGHT_HIND_LEG_ROT)];
const CAMEL_DASH_LEFT_EAR_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_DASH_LEFT_EAR_ROT)];
const CAMEL_DASH_RIGHT_EAR_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_DASH_RIGHT_EAR_ROT)];

const CAMEL_DASH_BONES_ANIM: [BoneAnimation; 9] = [
    BoneAnimation {
        bone: "body",
        channels: &CAMEL_DASH_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "tail",
        channels: &CAMEL_DASH_TAIL_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &CAMEL_DASH_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_front_leg",
        channels: &CAMEL_DASH_RIGHT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_front_leg",
        channels: &CAMEL_DASH_LEFT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_hind_leg",
        channels: &CAMEL_DASH_LEFT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_hind_leg",
        channels: &CAMEL_DASH_RIGHT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_ear",
        channels: &CAMEL_DASH_LEFT_EAR_CHANNELS,
    },
    BoneAnimation {
        bone: "right_ear",
        channels: &CAMEL_DASH_RIGHT_EAR_CHANNELS,
    },
];

/// Vanilla `CamelAnimation.CAMEL_DASH`: the looping 0.5 s gallop, sampled by `CamelModel.setupAnim`
/// via `dashAnimation.apply(dashAnimationState, ageInTicks)` while the camel is dashing. All channels
/// are ROTATION; the `body` and ears hold a constant Linear offset, the head/tail/legs swing on
/// CatmullRom. The renderer applies it ADDITIVELY (last, over the walk pose) while
/// `camel_dash_seconds >= 0`.
pub(in crate::entity_models) const CAMEL_DASH: AnimationDefinition = AnimationDefinition {
    length_seconds: 0.5,
    looping: true,
    bones: &CAMEL_DASH_BONES_ANIM,
};

/// The nine bones `CAMEL_DASH` animates (every bone except the whole-model `root`), in vanilla
/// `addAnimation` order. `setup_anim` walks these to add the dash pose onto the walk pose.
const CAMEL_DASH_BONES: [&str; 9] = [
    "body",
    "tail",
    "head",
    "right_front_leg",
    "left_front_leg",
    "left_hind_leg",
    "right_hind_leg",
    "left_ear",
    "right_ear",
];

/// The camel sit/stand bones positioned by the body/leg/head/tail keyframes (the `body` and `head`
/// are handled specially in `setup_anim`; these are the remaining swing bones). The two ears are not
/// animated by the sit/stand transitions, only by the walk.
const CAMEL_SIT_SWING_BONES: [&str; 5] = [
    "left_hind_leg",
    "right_hind_leg",
    "left_front_leg",
    "right_front_leg",
    "tail",
];

/// The camel walk bones that are positioned by leg/ear/tail keyframes (every animated bone except the
/// `root` whole-model roll, the `body`, and the `head` — which are handled specially in `setup_anim`).
const CAMEL_WALK_SWING_BONES: [&str; 7] = [
    "left_hind_leg",
    "right_hind_leg",
    "left_front_leg",
    "right_front_leg",
    "left_ear",
    "right_ear",
    "tail",
];

/// Builds the adult camel tree (vanilla `AdultCamelModel.createBodyLayer`): the `body` carries
/// `[hump, tail, head]`, the `head` carries `[left_ear, right_ear]`, and the four legs hang off the
/// root in the order `[left_hind_leg, right_hind_leg, left_front_leg, right_front_leg]`.
fn adult_camel_tree() -> ModelPart {
    let head = ModelPart::new(
        ADULT_CAMEL_HEAD_POSE,
        ADULT_CAMEL_HEAD.to_vec(),
        vec![
            (
                "left_ear",
                ModelPart::leaf(ADULT_CAMEL_LEFT_EAR_POSE, ADULT_CAMEL_LEFT_EAR.to_vec()),
            ),
            (
                "right_ear",
                ModelPart::leaf(ADULT_CAMEL_RIGHT_EAR_POSE, ADULT_CAMEL_RIGHT_EAR.to_vec()),
            ),
        ],
    );
    let body = ModelPart::new(
        ADULT_CAMEL_BODY_POSE,
        ADULT_CAMEL_BODY.to_vec(),
        vec![
            (
                "hump",
                ModelPart::leaf(ADULT_CAMEL_HUMP_POSE, ADULT_CAMEL_HUMP.to_vec()),
            ),
            (
                "tail",
                ModelPart::leaf(ADULT_CAMEL_TAIL_POSE, ADULT_CAMEL_TAIL.to_vec()),
            ),
            ("head", head),
        ],
    );
    let children: Vec<(&'static str, ModelPart)> = vec![
        ("body", body),
        (
            "left_hind_leg",
            ModelPart::leaf(
                ADULT_CAMEL_LEFT_HIND_LEG_POSE,
                ADULT_CAMEL_LEFT_HIND_LEG.to_vec(),
            ),
        ),
        (
            "right_hind_leg",
            ModelPart::leaf(
                ADULT_CAMEL_RIGHT_HIND_LEG_POSE,
                ADULT_CAMEL_RIGHT_HIND_LEG.to_vec(),
            ),
        ),
        (
            "left_front_leg",
            ModelPart::leaf(
                ADULT_CAMEL_LEFT_FRONT_LEG_POSE,
                ADULT_CAMEL_LEFT_FRONT_LEG.to_vec(),
            ),
        ),
        (
            "right_front_leg",
            ModelPart::leaf(
                ADULT_CAMEL_RIGHT_FRONT_LEG_POSE,
                ADULT_CAMEL_RIGHT_FRONT_LEG.to_vec(),
            ),
        ),
    ];
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Builds the baby camel tree (vanilla `BabyCamelModel.createBodyLayer`): the `body` carries
/// `[tail, head]` (no hump), the `head` carries `[right_ear, left_ear]`, and the four legs hang off the
/// root in the order `[right_front_leg, left_front_leg, left_hind_leg, right_hind_leg]`.
fn baby_camel_tree() -> ModelPart {
    let head = ModelPart::new(
        BABY_CAMEL_HEAD_POSE,
        BABY_CAMEL_HEAD.to_vec(),
        vec![
            (
                "right_ear",
                ModelPart::leaf(BABY_CAMEL_RIGHT_EAR_POSE, BABY_CAMEL_RIGHT_EAR.to_vec()),
            ),
            (
                "left_ear",
                ModelPart::leaf(BABY_CAMEL_LEFT_EAR_POSE, BABY_CAMEL_LEFT_EAR.to_vec()),
            ),
        ],
    );
    let body = ModelPart::new(
        BABY_CAMEL_BODY_POSE,
        BABY_CAMEL_BODY.to_vec(),
        vec![
            (
                "tail",
                ModelPart::leaf(BABY_CAMEL_TAIL_POSE, BABY_CAMEL_TAIL.to_vec()),
            ),
            ("head", head),
        ],
    );
    let children: Vec<(&'static str, ModelPart)> = vec![
        ("body", body),
        (
            "right_front_leg",
            ModelPart::leaf(
                BABY_CAMEL_RIGHT_FRONT_LEG_POSE,
                BABY_CAMEL_RIGHT_FRONT_LEG.to_vec(),
            ),
        ),
        (
            "left_front_leg",
            ModelPart::leaf(
                BABY_CAMEL_LEFT_FRONT_LEG_POSE,
                BABY_CAMEL_LEFT_FRONT_LEG.to_vec(),
            ),
        ),
        (
            "left_hind_leg",
            ModelPart::leaf(
                BABY_CAMEL_LEFT_HIND_LEG_POSE,
                BABY_CAMEL_LEFT_HIND_LEG.to_vec(),
            ),
        ),
        (
            "right_hind_leg",
            ModelPart::leaf(
                BABY_CAMEL_RIGHT_HIND_LEG_POSE,
                BABY_CAMEL_RIGHT_HIND_LEG.to_vec(),
            ),
        ),
    ];
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Mutable camel model, mirroring vanilla `AdultCamelModel` / `BabyCamelModel`. The unified tree is
/// built once with the vanilla bone names: the `body` (carrying the `hump`/`tail`/`head`, the `head`
/// carrying the two ears) plus the four legs hanging off the root. The leg/ear declaration order and
/// the adult hump differ between variants, but the bone names match, so one named-children tree drives
/// both render paths and both walks. `new` picks the adult or baby tree and the matching
/// [`AnimationDefinition`] (the camel husk shares the adult mesh/walk); `setup_anim` clamps the head
/// look ([`camel_clamped_head_look`]) and samples the looping walk keyframes (`applyWalk(..., 2.0,
/// 2.5)`): the `root` rolls the model, the legs / ears / tail swing, the `head` pitch ADDS onto the
/// clamped look, and the baby `body` dips. The colored fallback recolors the whole model with the
/// family tint; the textured path uses the family texture.
pub(in crate::entity_models) struct CamelModel {
    root: ModelPart,
    walk: &'static AnimationDefinition,
}

impl CamelModel {
    pub(in crate::entity_models) fn new(family: CamelModelFamily, baby: bool) -> Self {
        // The camel husk reuses the adult camel mesh/walk; only a real baby camel uses the baby layer.
        if family == CamelModelFamily::Camel && baby {
            Self {
                root: baby_camel_tree(),
                walk: &CAMEL_BABY_WALK,
            }
        } else {
            Self {
                root: adult_camel_tree(),
                walk: &CAMEL_WALK,
            }
        }
    }
}

impl EntityModel for CamelModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let walk = self.walk;
        let (head_yaw, head_pitch) = camel_clamped_head_look(
            instance.render_state.head_yaw,
            instance.render_state.head_pitch,
        );
        let (seconds, scale) = keyframe_walk_sample(
            walk,
            instance.render_state.walk_animation_pos,
            instance.render_state.walk_animation_speed,
            CAMEL_WALK_SPEED_FACTOR,
            CAMEL_WALK_SCALE_FACTOR,
        );
        let sample = |bone: &str| sample_bone_offsets(walk, bone, seconds, scale);

        // Vanilla `CamelModel.setupAnim` then applies the sit/sit-pose/stand-up one-shots ADDITIVELY
        // (`sitAnimation.apply(...)`, `sitPoseAnimation.apply(...)`, `standupAnimation.apply(...)`),
        // each only while its projected elapsed seconds are `>= 0`; a non-looping def clamps past its
        // length to the resting final frame. The active definitions, paired with their sample seconds.
        // (`dash` and `idle` stay deferred — see `docs/unsupported-features.md`.)
        let sit_stand: [(&AnimationDefinition, f32); 3] = [
            (&CAMEL_SIT, instance.render_state.camel_sit_seconds),
            (
                &CAMEL_SIT_POSE,
                instance.render_state.camel_sit_pose_seconds,
            ),
            (&CAMEL_STANDUP, instance.render_state.camel_standup_seconds),
        ];
        // Sums every active sit/stand one-shot's position/rotation offsets for one bone, so they can
        // be ADDED onto a bone already posed by the walk (and, for the head, the clamped look).
        let sit_stand_offsets = |bone: &str| {
            let mut position = [0.0_f32; 3];
            let mut rotation = [0.0_f32; 3];
            for (definition, elapsed) in sit_stand {
                if elapsed < 0.0 {
                    continue;
                }
                let sample_seconds = keyframe_elapsed_seconds(definition, elapsed);
                let (pos, rot) = sample_bone_offsets(definition, bone, sample_seconds, 1.0);
                for axis in 0..3 {
                    position[axis] += pos[axis];
                    rotation[axis] += rot[axis];
                }
            }
            (position, rotation)
        };

        // `root` rolls the whole model: no bind offset/rotation, so the z-sway applies at the entity
        // root. The synthetic root part carries it. (The sit/stand animations do not touch `root`.)
        let (root_pos, root_rot) = sample("root");
        self.root.pose = keyframe_animated_pose(PART_POSE_ZERO, root_pos, root_rot);

        // `body` (root child 0): the adult walk leaves it at bind, the baby walk dips it; the sit/stand
        // transitions pitch and drop it (the marquee fold-down). Walk then sit/stand, both ADDED.
        let (body_walk_pos, body_walk_rot) = sample("body");
        let (body_sit_pos, body_sit_rot) = sit_stand_offsets("body");
        let (head_walk_pos, head_walk_rot) = sample("head");
        let (head_sit_pos, head_sit_rot) = sit_stand_offsets("head");
        let body = self.root.child_mut("body");
        let body_bind = body.pose;
        body.pose = keyframe_animated_pose(
            body_bind,
            [
                body_walk_pos[0] + body_sit_pos[0],
                body_walk_pos[1] + body_sit_pos[1],
                body_walk_pos[2] + body_sit_pos[2],
            ],
            [
                body_walk_rot[0] + body_sit_rot[0],
                body_walk_rot[1] + body_sit_rot[1],
                body_walk_rot[2] + body_sit_rot[2],
            ],
        );

        // The head (clamped look + walk + sit/stand) carrying the two ears (walk swing handled below),
        // and the tail (walk + sit/stand). The adult hump is static, so it stays at its bind pose.
        let head = body.child_mut("head");
        let head_bind = head.pose;
        head.pose = PartPose {
            offset: [
                head_bind.offset[0] + head_walk_pos[0] + head_sit_pos[0],
                head_bind.offset[1] + head_walk_pos[1] + head_sit_pos[1],
                head_bind.offset[2] + head_walk_pos[2] + head_sit_pos[2],
            ],
            rotation: [
                head_pitch.to_radians() + head_walk_rot[0] + head_sit_rot[0],
                head_yaw.to_radians() + head_walk_rot[1] + head_sit_rot[1],
                head_bind.rotation[2] + head_walk_rot[2] + head_sit_rot[2],
            ],
        };

        // The two ears (head children, walk only), then the tail (body child) and the four legs (root
        // children), all addressed by their vanilla bone name regardless of the per-variant declaration
        // order. Walk first, then the sit/stand offsets ADDED onto the four legs and the tail.
        for bone in CAMEL_WALK_SWING_BONES {
            let (walk_pos, walk_rot) = sample(bone);
            let part = match bone {
                "left_ear" | "right_ear" => self.root.child_mut("body").child_mut("head"),
                "tail" => self.root.child_mut("body"),
                _ => &mut self.root,
            }
            .child_mut(bone);
            part.pose = keyframe_animated_pose(part.pose, walk_pos, walk_rot);
        }
        for bone in CAMEL_SIT_SWING_BONES {
            let (sit_pos, sit_rot) = sit_stand_offsets(bone);
            let part = match bone {
                "tail" => self.root.child_mut("body"),
                _ => &mut self.root,
            }
            .child_mut(bone);
            part.pose = keyframe_animated_pose(part.pose, sit_pos, sit_rot);
        }

        // Vanilla `CamelModel.setupAnim` applies `dashAnimation.apply(dashAnimationState, ageInTicks)`
        // LAST, after the walk and sit/stand poses. The looping `CAMEL_DASH` gallop is ADDED onto every
        // animated bone (all nine — body, head, ears, tail, four legs) while the camel is dashing
        // (`camel_dash_seconds >= 0`); otherwise it is skipped. A looping def wraps its sample time.
        let dash_seconds = instance.render_state.camel_dash_seconds;
        if dash_seconds >= 0.0 {
            let elapsed = keyframe_elapsed_seconds(&CAMEL_DASH, dash_seconds);
            for bone in CAMEL_DASH_BONES {
                let (dash_pos, dash_rot) = sample_bone_offsets(&CAMEL_DASH, bone, elapsed, 1.0);
                let part = match bone {
                    "body" => self.root.child_mut("body"),
                    "head" => self.root.child_mut("body").child_mut("head"),
                    "left_ear" | "right_ear" => self
                        .root
                        .child_mut("body")
                        .child_mut("head")
                        .child_mut(bone),
                    "tail" => self.root.child_mut("body").child_mut("tail"),
                    _ => self.root.child_mut(bone),
                };
                part.pose = keyframe_animated_pose(part.pose, dash_pos, dash_rot);
            }
        }
    }
}
