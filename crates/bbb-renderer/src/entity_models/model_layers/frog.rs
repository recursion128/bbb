use super::super::keyframe::{
    degree_vec, keyframe, keyframe_animated_pose, keyframe_animated_scale,
    keyframe_elapsed_seconds, keyframe_walk_sample, pos_vec, sample_bone_offsets,
    sample_bone_offsets_with_scale, scale_vec, AnimationChannel, AnimationDefinition,
    AnimationTarget, BoneAnimation, Keyframe, KeyframeInterpolation,
};
use super::{model_cube as cube, ModelCubeDesc, PartPose, FROG_BODY, FROG_EYE, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

// Vanilla 26.1 `FrogModel.createBodyLayer` (atlas 48×48). The mesh root holds one `root` part at
// `offset(0, 24, 0)` parenting `body` and the two legs; `body` parents the head (with its eye
// chain), the `croaking_body` pouch, the tongue, and the two arms (with their hands). The looping
// `FrogAnimation.FROG_WALK` keyframe animation is reproduced ([`FROG_WALK`]) and the triggered
// `FrogAnimation.FROG_CROAK` pouch animation is reproduced ([`FROG_CROAK`], applied only while the
// projected `frog_croak_seconds >= 0`); the jump, tongue, and in-water swim/idle keyframe
// animations stay deferred (un-projected `AnimationState`s), so a still or non-swimming,
// non-croaking frog renders at the walk-sampled pose. The three frog texture variants share this
// geometry and are deferred with the texture-backed path.

// `body`: the `texOffs(3,1)` 7×3×9 box plus the `texOffs(23,22)` 7×0×9 underside plane.
pub(in crate::entity_models) const FROG_BODY_CUBES: [ModelCubeDesc; 2] = [
    cube([-3.5, -2.0, -8.0], [7.0, 3.0, 9.0], FROG_BODY),
    cube([-3.5, -1.0, -8.0], [7.0, 0.0, 9.0], FROG_BODY),
];

// `head`: the `texOffs(23,13)` 7×0×9 top plane plus the `texOffs(0,13)` 7×3×9 box.
pub(in crate::entity_models) const FROG_HEAD_CUBES: [ModelCubeDesc; 2] = [
    cube([-3.5, -1.0, -7.0], [7.0, 0.0, 9.0], FROG_BODY),
    cube([-3.5, -2.0, -7.0], [7.0, 3.0, 9.0], FROG_BODY),
];

// Each eye is the same 3×2×3 box (`texOffs(0,0)`/`(0,5)`).
pub(in crate::entity_models) const FROG_EYE_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.5, -1.0, -1.5], [3.0, 2.0, 3.0], FROG_EYE)];

pub(in crate::entity_models) const FROG_TONGUE_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.0, 0.0, -7.1], [4.0, 0.0, 7.0], FROG_BODY)];

// `croaking_body`: the `texOffs(26,5)` 7×2×3 pouch box with `CubeDeformation(-0.1)`, so the colored
// geometry is deflated 0.1 on every face (`min -= grow`, `max += grow` with `grow = -0.1`): min
// `(-3.5, -0.1, -2.9) + 0.1` = `(-3.4, 0.0, -2.8)`, size `(7, 2, 3) - 0.2` = `(6.8, 1.8, 2.8)`. The
// pouch is hidden at rest and the `FROG_CROAK` SCALE channel puffs it out and back.
pub(in crate::entity_models) const FROG_CROAKING_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.4, 0.0, -2.8], [6.8, 1.8, 2.8], FROG_BODY)];

// Both arms share the 2×3×3 box; the webbed hands are 8×0×8 planes that differ only in Z origin.
pub(in crate::entity_models) const FROG_ARM_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, -1.0], [2.0, 3.0, 3.0], FROG_BODY)];
pub(in crate::entity_models) const FROG_LEFT_HAND_CUBES: [ModelCubeDesc; 1] =
    [cube([-4.0, 0.01, -4.0], [8.0, 0.0, 8.0], FROG_BODY)];
pub(in crate::entity_models) const FROG_RIGHT_HAND_CUBES: [ModelCubeDesc; 1] =
    [cube([-4.0, 0.01, -5.0], [8.0, 0.0, 8.0], FROG_BODY)];

// The legs differ only in X origin; both feet share one 8×0×8 plane.
pub(in crate::entity_models) const FROG_LEFT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, -2.0], [3.0, 3.0, 4.0], FROG_BODY)];
pub(in crate::entity_models) const FROG_RIGHT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.0, 0.0, -2.0], [3.0, 3.0, 4.0], FROG_BODY)];
pub(in crate::entity_models) const FROG_FOOT_CUBES: [ModelCubeDesc; 1] =
    [cube([-4.0, 0.01, -4.0], [8.0, 0.0, 8.0], FROG_BODY)];

/// Vanilla `FrogModel.createBodyLayer` rest-pose part poses, rooted at the cubeless `root` part
/// (`offset(0, 24, 0)`) parenting `body` and the two legs; `body` parents the head (with its eye
/// chain), the `croaking_body` pouch, the tongue, and the two arms (with their hands). Fifteen
/// visible cubes at rest; the sixteenth `croaking_body` cube is hidden until the frog croaks.
/// `root` cubeless-pivot part pose: `PartPose.offset(0, 24, 0)`.
pub(in crate::entity_models) const FROG_ROOT_POSE: PartPose = PartPose {
    offset: [0.0, 24.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `body` part pose: `PartPose.offset(0, -2, 4)`.
pub(in crate::entity_models) const FROG_BODY_POSE: PartPose = PartPose {
    offset: [0.0, -2.0, 4.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `head` part pose: `PartPose.offset(0, -2, -1)`.
pub(in crate::entity_models) const FROG_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, -2.0, -1.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `eyes` cubeless-pivot part pose: `PartPose.offset(-0.5, 0, 2)`.
pub(in crate::entity_models) const FROG_EYES_POSE: PartPose = PartPose {
    offset: [-0.5, 0.0, 2.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_eye` part pose: `PartPose.offset(-1.5, -3, -6.5)`.
pub(in crate::entity_models) const FROG_LEFT_EYE_POSE: PartPose = PartPose {
    offset: [-1.5, -3.0, -6.5],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_eye` part pose: `PartPose.offset(2.5, -3, -6.5)`.
pub(in crate::entity_models) const FROG_RIGHT_EYE_POSE: PartPose = PartPose {
    offset: [2.5, -3.0, -6.5],
    rotation: [0.0, 0.0, 0.0],
};
/// `croaking_body` part pose: `PartPose.offset(0, -1, -5)`.
pub(in crate::entity_models) const FROG_CROAKING_BODY_POSE: PartPose = PartPose {
    offset: [0.0, -1.0, -5.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `tongue` part pose: `PartPose.offset(0, -1.01, 1)`.
pub(in crate::entity_models) const FROG_TONGUE_POSE: PartPose = PartPose {
    offset: [0.0, -1.01, 1.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_arm` part pose: `PartPose.offset(4, -1, -6.5)`.
pub(in crate::entity_models) const FROG_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [4.0, -1.0, -6.5],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_arm` part pose: `PartPose.offset(-4, -1, -6.5)`.
pub(in crate::entity_models) const FROG_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-4.0, -1.0, -6.5],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_hand` part pose: `PartPose.offset(0, 3, -1)`.
pub(in crate::entity_models) const FROG_LEFT_HAND_POSE: PartPose = PartPose {
    offset: [0.0, 3.0, -1.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_hand` part pose: `PartPose.offset(0, 3, 0)`.
pub(in crate::entity_models) const FROG_RIGHT_HAND_POSE: PartPose = PartPose {
    offset: [0.0, 3.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_leg` part pose: `PartPose.offset(3.5, -3, 4)`.
pub(in crate::entity_models) const FROG_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [3.5, -3.0, 4.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_leg` part pose: `PartPose.offset(-3.5, -3, 4)`.
pub(in crate::entity_models) const FROG_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-3.5, -3.0, 4.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_foot` part pose: `PartPose.offset(2, 3, 0)`.
pub(in crate::entity_models) const FROG_LEFT_FOOT_POSE: PartPose = PartPose {
    offset: [2.0, 3.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_foot` part pose: `PartPose.offset(-2, 3, 0)`.
pub(in crate::entity_models) const FROG_RIGHT_FOOT_POSE: PartPose = PartPose {
    offset: [-2.0, 3.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds the frog's synthetic root parenting the single cubeless `root` part, which parents the
/// cube-bearing `body` (head → eyes → two eyes; croaking_body; tongue; two arms → hands) and the
/// two legs (each → foot), in vanilla `addOrReplaceChild` order. The `body`, `croaking_body`,
/// `left_arm`/`right_arm`, and the two legs are name-addressed by `setup_anim`, so `body` and
/// `root` carry named children.
fn frog_root() -> ModelPart {
    let head = ModelPart::colored(
        FROG_HEAD_POSE,
        &FROG_HEAD_CUBES,
        vec![ModelPart::new(
            FROG_EYES_POSE,
            Vec::new(),
            vec![
                (
                    "left_eye",
                    ModelPart::leaf_colored(FROG_LEFT_EYE_POSE, &FROG_EYE_CUBES),
                ),
                (
                    "right_eye",
                    ModelPart::leaf_colored(FROG_RIGHT_EYE_POSE, &FROG_EYE_CUBES),
                ),
            ],
        )],
    );
    let left_arm = ModelPart::colored(
        FROG_LEFT_ARM_POSE,
        &FROG_ARM_CUBES,
        vec![ModelPart::leaf_colored(
            FROG_LEFT_HAND_POSE,
            &FROG_LEFT_HAND_CUBES,
        )],
    );
    let right_arm = ModelPart::colored(
        FROG_RIGHT_ARM_POSE,
        &FROG_ARM_CUBES,
        vec![ModelPart::leaf_colored(
            FROG_RIGHT_HAND_POSE,
            &FROG_RIGHT_HAND_CUBES,
        )],
    );
    let body = ModelPart::colored_named(
        FROG_BODY_POSE,
        &FROG_BODY_CUBES,
        vec![
            ("head", head),
            (
                "croaking_body",
                ModelPart::leaf_colored(FROG_CROAKING_BODY_POSE, &FROG_CROAKING_BODY_CUBES),
            ),
            (
                "tongue",
                ModelPart::leaf_colored(FROG_TONGUE_POSE, &FROG_TONGUE_CUBES),
            ),
            ("left_arm", left_arm),
            ("right_arm", right_arm),
        ],
    );
    let left_leg = ModelPart::colored(
        FROG_LEFT_LEG_POSE,
        &FROG_LEFT_LEG_CUBES,
        vec![ModelPart::leaf_colored(
            FROG_LEFT_FOOT_POSE,
            &FROG_FOOT_CUBES,
        )],
    );
    let right_leg = ModelPart::colored(
        FROG_RIGHT_LEG_POSE,
        &FROG_RIGHT_LEG_CUBES,
        vec![ModelPart::leaf_colored(
            FROG_RIGHT_FOOT_POSE,
            &FROG_FOOT_CUBES,
        )],
    );
    let frog_root = ModelPart::new(
        FROG_ROOT_POSE,
        Vec::new(),
        vec![
            ("body", body),
            ("left_leg", left_leg),
            ("right_leg", right_leg),
        ],
    );
    ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![("root", frog_root)])
}

// ----- `FrogAnimation.FROG_WALK` (length 1.25s, looping). All keyframes are LINEAR; `degreeVec`
// converts degrees → radians and `posVec` negates the y axis. The animated bones map to the part
// tree as: `body` = root child 0, `left_leg`/`right_leg` = root children 1/2, `left_arm`/`right_arm`
// = body children 2/3. -----

const LINEAR: KeyframeInterpolation = KeyframeInterpolation::Linear;

const FROG_WALK_LEFT_ARM_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(0.0, -5.0, 0.0), LINEAR),
    keyframe(0.2917, degree_vec(7.5, -2.67, -7.5), LINEAR),
    keyframe(0.625, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.7917, degree_vec(22.5, 0.0, 0.0), LINEAR),
    keyframe(1.125, degree_vec(-45.0, 0.0, 0.0), LINEAR),
    keyframe(1.25, degree_vec(0.0, -5.0, 0.0), LINEAR),
];
const FROG_WALK_LEFT_ARM_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 0.1, -2.0), LINEAR),
    keyframe(0.2917, pos_vec(-0.5, -0.25, -0.13), LINEAR),
    keyframe(0.625, pos_vec(-0.5, 0.1, 2.0), LINEAR),
    keyframe(0.9583, pos_vec(0.5, 1.0, -0.11), LINEAR),
    keyframe(1.25, pos_vec(0.0, 0.1, -2.0), LINEAR),
];
const FROG_WALK_RIGHT_ARM_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(22.5, 0.0, 0.0), LINEAR),
    keyframe(0.4583, degree_vec(-45.0, 0.0, 0.0), LINEAR),
    keyframe(0.625, degree_vec(0.0, 5.0, 0.0), LINEAR),
    keyframe(0.9583, degree_vec(7.5, 2.33, 7.5), LINEAR),
    keyframe(1.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const FROG_WALK_RIGHT_ARM_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.5, 0.1, 2.0), LINEAR),
    keyframe(0.2917, pos_vec(-0.5, 1.0, 0.12), LINEAR),
    keyframe(0.625, pos_vec(0.0, 0.1, -2.0), LINEAR),
    keyframe(0.9583, pos_vec(0.5, -0.25, -0.13), LINEAR),
    keyframe(1.25, pos_vec(0.5, 0.1, 2.0), LINEAR),
];
const FROG_WALK_LEFT_LEG_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2917, degree_vec(45.0, 0.0, 0.0), LINEAR),
    keyframe(0.625, degree_vec(-45.0, 0.0, 0.0), LINEAR),
    keyframe(0.7917, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const FROG_WALK_LEFT_LEG_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 0.1, 1.2), LINEAR),
    keyframe(0.1667, pos_vec(0.0, 0.1, 2.0), LINEAR),
    keyframe(0.4583, pos_vec(0.0, 2.0, 1.06), LINEAR),
    keyframe(0.7917, pos_vec(0.0, 0.1, -1.0), LINEAR),
    keyframe(1.25, pos_vec(0.0, 0.1, 1.2), LINEAR),
];
const FROG_WALK_RIGHT_LEG_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(-33.75, 0.0, 0.0), LINEAR),
    keyframe(0.0417, degree_vec(-45.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.7917, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.9583, degree_vec(45.0, 0.0, 0.0), LINEAR),
    keyframe(1.25, degree_vec(-33.75, 0.0, 0.0), LINEAR),
];
const FROG_WALK_RIGHT_LEG_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 1.14, 0.11), LINEAR),
    keyframe(0.1667, pos_vec(0.0, 0.1, -1.0), LINEAR),
    keyframe(0.7917, pos_vec(0.0, 0.1, 2.0), LINEAR),
    keyframe(1.125, pos_vec(0.0, 2.0, 0.95), LINEAR),
    keyframe(1.25, pos_vec(0.0, 1.14, 0.11), LINEAR),
];
const FROG_WALK_BODY_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 5.0, 0.0), LINEAR),
    keyframe(0.2917, degree_vec(-7.5, 0.33, 7.5), LINEAR),
    keyframe(0.625, degree_vec(0.0, -5.0, 0.0), LINEAR),
    keyframe(0.9583, degree_vec(-7.5, 0.33, -7.5), LINEAR),
    keyframe(1.25, degree_vec(0.0, 5.0, 0.0), LINEAR),
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

const FROG_WALK_LEFT_ARM_CHANNELS: [AnimationChannel; 2] =
    [rot(&FROG_WALK_LEFT_ARM_ROT), pos(&FROG_WALK_LEFT_ARM_POS)];
const FROG_WALK_RIGHT_ARM_CHANNELS: [AnimationChannel; 2] =
    [rot(&FROG_WALK_RIGHT_ARM_ROT), pos(&FROG_WALK_RIGHT_ARM_POS)];
const FROG_WALK_LEFT_LEG_CHANNELS: [AnimationChannel; 2] =
    [rot(&FROG_WALK_LEFT_LEG_ROT), pos(&FROG_WALK_LEFT_LEG_POS)];
const FROG_WALK_RIGHT_LEG_CHANNELS: [AnimationChannel; 2] =
    [rot(&FROG_WALK_RIGHT_LEG_ROT), pos(&FROG_WALK_RIGHT_LEG_POS)];
const FROG_WALK_BODY_CHANNELS: [AnimationChannel; 1] = [rot(&FROG_WALK_BODY_ROT)];

const FROG_WALK_BONES: [BoneAnimation; 5] = [
    BoneAnimation {
        bone: "left_arm",
        channels: &FROG_WALK_LEFT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "right_arm",
        channels: &FROG_WALK_RIGHT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_leg",
        channels: &FROG_WALK_LEFT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_leg",
        channels: &FROG_WALK_RIGHT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "body",
        channels: &FROG_WALK_BODY_CHANNELS,
    },
];

/// Vanilla `FrogAnimation.FROG_WALK`: the looping 1.25s ground-walk cycle, sampled by
/// `FrogModel.setupAnim` via `applyWalk(walkAnimationPos, walkAnimationSpeed, 1.5, 2.5)`.
pub(in crate::entity_models) const FROG_WALK: AnimationDefinition = AnimationDefinition {
    length_seconds: 1.25,
    looping: true,
    bones: &FROG_WALK_BONES,
};

/// Vanilla `FrogModel.setupAnim` walk-call factors: `applyWalk(pos, speed, 1.5, 2.5)` (the swim
/// variant uses `1.0, 2.5` and is deferred).
pub(in crate::entity_models) const FROG_WALK_SPEED_FACTOR: f32 = 1.5;
pub(in crate::entity_models) const FROG_WALK_SCALE_FACTOR: f32 = 2.5;

// ----- `FrogAnimation.FROG_CROAK` (length 3.0s, NOT looping). The single `croaking_body` bone has
// a POSITION channel (the pouch lifts `+1` y once inflated) and a SCALE channel (the pouch puffs
// `(1.3, 2.1, 1.6)` twice and rests collapsed at `(0, 0, 0)`, so it is invisible until the croak).
// All keyframes are LINEAR; `posVec` negates the y axis and `scaleVec` stores `value - 1`. -----

const FROG_CROAK_BODY_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.375, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.4167, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.4583, pos_vec(0.0, 1.0, 0.0), LINEAR),
    keyframe(2.9583, pos_vec(0.0, 1.0, 0.0), LINEAR),
    keyframe(3.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const FROG_CROAK_BODY_SCALE: [Keyframe; 16] = [
    keyframe(0.0, scale_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.375, scale_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.4167, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(0.4583, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(0.5417, scale_vec(1.3, 2.1, 1.6), LINEAR),
    keyframe(0.625, scale_vec(1.3, 2.1, 1.6), LINEAR),
    keyframe(0.7083, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(2.25, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(2.3333, scale_vec(1.3, 2.1, 1.6), LINEAR),
    keyframe(2.4167, scale_vec(1.3, 2.1, 1.6), LINEAR),
    keyframe(2.5, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(2.5833, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(2.6667, scale_vec(1.3, 2.1, 1.6), LINEAR),
    keyframe(2.875, scale_vec(1.3, 2.1, 1.6), LINEAR),
    keyframe(2.9583, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(3.0, scale_vec(0.0, 0.0, 0.0), LINEAR),
];

const fn scale_channel(keyframes: &'static [Keyframe]) -> AnimationChannel {
    AnimationChannel {
        target: AnimationTarget::Scale,
        keyframes,
    }
}

const FROG_CROAK_BODY_CHANNELS: [AnimationChannel; 2] = [
    pos(&FROG_CROAK_BODY_POS),
    scale_channel(&FROG_CROAK_BODY_SCALE),
];

const FROG_CROAK_BONES: [BoneAnimation; 1] = [BoneAnimation {
    bone: "croaking_body",
    channels: &FROG_CROAK_BODY_CHANNELS,
}];

/// Vanilla `FrogAnimation.FROG_CROAK`: the triggered 3.0s pouch animation (NOT looping), sampled by
/// `FrogModel.setupAnim` via `croakAnimation.apply(croakAnimationState, ageInTicks)` while the frog
/// is in `Pose.CROAKING`. The renderer applies it only when the projected `frog_croak_seconds >= 0`.
pub(in crate::entity_models) const FROG_CROAK: AnimationDefinition = AnimationDefinition {
    length_seconds: 3.0,
    looping: false,
    bones: &FROG_CROAK_BONES,
};

/// Mutable frog model, mirroring vanilla `FrogModel`. The cubeless `root` part (parenting `body`
/// and the two legs; `body` parents the head, croaking_body pouch, tongue, and two arms) hangs off
/// a synthetic root, built from the baked colored geometry as a named-children tree. Colored-only:
/// `setup_anim` applies the looping `FROG_WALK` keyframe cycle to the body, arms, and legs, and the
/// triggered `FROG_CROAK` pouch animation while croaking (the jump / tongue / swim animations stay
/// deferred).
pub(in crate::entity_models) struct FrogModel {
    root: ModelPart,
}

impl FrogModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self { root: frog_root() }
    }
}

impl EntityModel for FrogModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `FrogModel.setupAnim` runs `applyWalk(walkAnimationPos, walkAnimationSpeed, 1.5,
        // 2.5)`: the walk position drives the keyframe sample time and the speed scales the amplitude
        // (a still frog samples the cycle's rest frame). The cycle offsets the `body` (rotation), the
        // two arms (`body` children), and the two legs (`root` children); the head and tongue hold.
        let (seconds, scale) = keyframe_walk_sample(
            &FROG_WALK,
            instance.render_state.walk_animation_pos,
            instance.render_state.walk_animation_speed,
            FROG_WALK_SPEED_FACTOR,
            FROG_WALK_SCALE_FACTOR,
        );
        let animate = |part: &mut ModelPart, bone: &str| {
            let (position, rotation) = sample_bone_offsets(&FROG_WALK, bone, seconds, scale);
            part.pose = keyframe_animated_pose(part.pose, position, rotation);
        };

        // Vanilla `FrogModel.setupAnim` then runs `croakAnimation.apply(croakAnimationState,
        // ageInTicks)` and `croakingBody.visible = croakAnimationState.isStarted()`. The projected
        // `frog_croak_seconds` carries the elapsed seconds since the croak started, or `-1` when the
        // frog is not croaking (the `croakAnimationState` is stopped). While croaking, the pouch is
        // shown and the `FROG_CROAK` POSITION/SCALE channels lift and puff it; otherwise it stays
        // hidden at its collapsed bind pose.
        let croak_seconds = instance.render_state.frog_croak_seconds;

        let frog_root = self.root.child_mut("root");
        {
            let body = frog_root.child_mut("body");
            animate(body, "body");
            animate(body.child_mut("left_arm"), "left_arm");
            animate(body.child_mut("right_arm"), "right_arm");

            let croaking_body = body.child_mut("croaking_body");
            if croak_seconds >= 0.0 {
                croaking_body.visible = true;
                let seconds = keyframe_elapsed_seconds(&FROG_CROAK, croak_seconds);
                let (position, rotation, scale_offset) =
                    sample_bone_offsets_with_scale(&FROG_CROAK, "croaking_body", seconds, 1.0);
                croaking_body.pose = keyframe_animated_pose(croaking_body.pose, position, rotation);
                croaking_body.scale = keyframe_animated_scale(scale_offset);
            } else {
                croaking_body.visible = false;
            }
        }
        animate(frog_root.child_mut("left_leg"), "left_leg");
        animate(frog_root.child_mut("right_leg"), "right_leg");
    }
}
