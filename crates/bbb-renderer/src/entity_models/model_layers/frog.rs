use super::super::keyframe::{
    degree_vec, keyframe, keyframe_animated_pose, keyframe_animated_scale,
    keyframe_elapsed_seconds, keyframe_walk_sample, pos_vec, sample_bone_offsets,
    sample_bone_offsets_with_scale, scale_vec, AnimationChannel, AnimationDefinition,
    AnimationTarget, BoneAnimation, Keyframe, KeyframeInterpolation,
};
use super::{PartPose, FROG_BODY, FROG_EYE, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 `FrogModel.createBodyLayer` (atlas 48×48). The mesh root holds one `root` part at
// `offset(0, 24, 0)` parenting `body` and the two legs; `body` parents the head (with its eye
// chain), the `croaking_body` pouch, the tongue, and the two arms (with their hands). The looping
// The looping `FrogAnimation.FROG_WALK` cycle ([`FROG_WALK`]), the triggered `FROG_CROAK` pouch
// ([`FROG_CROAK`], gated on `frog_croak_seconds >= 0`), the `FROG_JUMP` long-jump hold pose
// ([`FROG_JUMP`], gated on `frog_jump_seconds >= 0`), the looping in-water `FROG_IDLE_WATER`
// hover ([`FROG_IDLE_WATER`], gated on `frog_swim_idle_seconds >= 0`), and the triggered
// `FROG_TONGUE` lash ([`FROG_TONGUE`], gated on `frog_tongue_seconds >= 0`) are all reproduced — so a
// still, dry, idle frog renders at the walk-sampled pose. Only the cross-entity prey-targeting that
// visually aims the tongue at the eaten entity (`DATA_TONGUE_TARGET_ID`) stays deferred; it is not
// part of the `FrogModel` animation. The three frog texture variants share this geometry on the
// wired texture-backed path.

// `body`: the `texOffs(3,1)` 7×3×9 box plus the `texOffs(23,22)` 7×0×9 underside plane.
pub(in crate::entity_models) const FROG_BODY_CUBES: [ModelCube; 2] = [
    ModelCube::new(
        [-3.5, -2.0, -8.0],
        [7.0, 3.0, 9.0],
        FROG_BODY,
        [7.0, 3.0, 9.0],
        [3.0, 1.0],
        false,
    ),
    ModelCube::new(
        [-3.5, -1.0, -8.0],
        [7.0, 0.0, 9.0],
        FROG_BODY,
        [7.0, 0.0, 9.0],
        [23.0, 22.0],
        false,
    ),
];

// `head`: the `texOffs(23,13)` 7×0×9 top plane plus the `texOffs(0,13)` 7×3×9 box.
pub(in crate::entity_models) const FROG_HEAD_CUBES: [ModelCube; 2] = [
    ModelCube::new(
        [-3.5, -1.0, -7.0],
        [7.0, 0.0, 9.0],
        FROG_BODY,
        [7.0, 0.0, 9.0],
        [23.0, 13.0],
        false,
    ),
    ModelCube::new(
        [-3.5, -2.0, -7.0],
        [7.0, 3.0, 9.0],
        FROG_BODY,
        [7.0, 3.0, 9.0],
        [0.0, 13.0],
        false,
    ),
];

// Each eye is the same 3×2×3 box, but the two eyes draw distinct UV regions (not mirrors): the
// right eye `texOffs(0,0)`, the left eye `texOffs(0,5)`.
pub(in crate::entity_models) const FROG_RIGHT_EYE_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.5, -1.0, -1.5],
    [3.0, 2.0, 3.0],
    FROG_EYE,
    [3.0, 2.0, 3.0],
    [0.0, 0.0],
    false,
)];
pub(in crate::entity_models) const FROG_LEFT_EYE_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.5, -1.0, -1.5],
    [3.0, 2.0, 3.0],
    FROG_EYE,
    [3.0, 2.0, 3.0],
    [0.0, 5.0],
    false,
)];

pub(in crate::entity_models) const FROG_TONGUE_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -7.1],
    [4.0, 0.0, 7.0],
    FROG_BODY,
    [4.0, 0.0, 7.0],
    [17.0, 13.0],
    false,
)];

// `croaking_body`: the `texOffs(26,5)` 7×2×3 pouch box with `CubeDeformation(-0.1)`, so the colored
// geometry is deflated 0.1 on every face (`min -= grow`, `max += grow` with `grow = -0.1`): min
// `(-3.5, -0.1, -2.9) + 0.1` = `(-3.4, 0.0, -2.8)`, size `(7, 2, 3) - 0.2` = `(6.8, 1.8, 2.8)`. The
// `uv_size` stays the integer pre-deformation `addBox` dims `(7, 2, 3)`. The pouch is hidden at rest
// and the `FROG_CROAK` SCALE channel puffs it out and back.
pub(in crate::entity_models) const FROG_CROAKING_BODY_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-3.4, 0.0, -2.8],
    [6.8, 1.8, 2.8],
    FROG_BODY,
    [7.0, 2.0, 3.0],
    [26.0, 5.0],
    false,
)];

// Both arms share the 2×3×3 box (distinct UV regions, not mirrors): the left arm `texOffs(0,32)`,
// the right arm `texOffs(0,38)`. The webbed hands are 8×0×8 planes differing in Z origin and UV.
pub(in crate::entity_models) const FROG_LEFT_ARM_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 3.0, 3.0],
    FROG_BODY,
    [2.0, 3.0, 3.0],
    [0.0, 32.0],
    false,
)];
pub(in crate::entity_models) const FROG_RIGHT_ARM_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 3.0, 3.0],
    FROG_BODY,
    [2.0, 3.0, 3.0],
    [0.0, 38.0],
    false,
)];
pub(in crate::entity_models) const FROG_LEFT_HAND_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.01, -4.0],
    [8.0, 0.0, 8.0],
    FROG_BODY,
    [8.0, 0.0, 8.0],
    [18.0, 40.0],
    false,
)];
pub(in crate::entity_models) const FROG_RIGHT_HAND_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.01, -5.0],
    [8.0, 0.0, 8.0],
    FROG_BODY,
    [8.0, 0.0, 8.0],
    [2.0, 40.0],
    false,
)];

// The legs differ in X origin and UV (left `texOffs(14,25)`, right `texOffs(0,25)`); the two feet
// share the 8×0×8 plane shape but draw distinct UV regions: the left `texOffs(2,32)`, the right
// `texOffs(18,32)`.
pub(in crate::entity_models) const FROG_LEFT_LEG_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -2.0],
    [3.0, 3.0, 4.0],
    FROG_BODY,
    [3.0, 3.0, 4.0],
    [14.0, 25.0],
    false,
)];
pub(in crate::entity_models) const FROG_RIGHT_LEG_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [3.0, 3.0, 4.0],
    FROG_BODY,
    [3.0, 3.0, 4.0],
    [0.0, 25.0],
    false,
)];
pub(in crate::entity_models) const FROG_LEFT_FOOT_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.01, -4.0],
    [8.0, 0.0, 8.0],
    FROG_BODY,
    [8.0, 0.0, 8.0],
    [2.0, 32.0],
    false,
)];
pub(in crate::entity_models) const FROG_RIGHT_FOOT_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.01, -4.0],
    [8.0, 0.0, 8.0],
    FROG_BODY,
    [8.0, 0.0, 8.0],
    [18.0, 32.0],
    false,
)];

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
    let head = ModelPart::new(
        FROG_HEAD_POSE,
        FROG_HEAD_CUBES.to_vec(),
        vec![(
            "0",
            ModelPart::new(
                FROG_EYES_POSE,
                Vec::new(),
                vec![
                    (
                        "left_eye",
                        ModelPart::leaf(FROG_LEFT_EYE_POSE, FROG_LEFT_EYE_CUBES.to_vec()),
                    ),
                    (
                        "right_eye",
                        ModelPart::leaf(FROG_RIGHT_EYE_POSE, FROG_RIGHT_EYE_CUBES.to_vec()),
                    ),
                ],
            ),
        )],
    );
    let left_arm = ModelPart::new(
        FROG_LEFT_ARM_POSE,
        FROG_LEFT_ARM_CUBES.to_vec(),
        vec![(
            "0",
            ModelPart::leaf(FROG_LEFT_HAND_POSE, FROG_LEFT_HAND_CUBES.to_vec()),
        )],
    );
    let right_arm = ModelPart::new(
        FROG_RIGHT_ARM_POSE,
        FROG_RIGHT_ARM_CUBES.to_vec(),
        vec![(
            "0",
            ModelPart::leaf(FROG_RIGHT_HAND_POSE, FROG_RIGHT_HAND_CUBES.to_vec()),
        )],
    );
    let body = ModelPart::new(
        FROG_BODY_POSE,
        FROG_BODY_CUBES.to_vec(),
        vec![
            ("head", head),
            (
                "croaking_body",
                ModelPart::leaf(FROG_CROAKING_BODY_POSE, FROG_CROAKING_BODY_CUBES.to_vec()),
            ),
            (
                "tongue",
                ModelPart::leaf(FROG_TONGUE_POSE, FROG_TONGUE_CUBES.to_vec()),
            ),
            ("left_arm", left_arm),
            ("right_arm", right_arm),
        ],
    );
    let left_leg = ModelPart::new(
        FROG_LEFT_LEG_POSE,
        FROG_LEFT_LEG_CUBES.to_vec(),
        vec![(
            "0",
            ModelPart::leaf(FROG_LEFT_FOOT_POSE, FROG_LEFT_FOOT_CUBES.to_vec()),
        )],
    );
    let right_leg = ModelPart::new(
        FROG_RIGHT_LEG_POSE,
        FROG_RIGHT_LEG_CUBES.to_vec(),
        vec![(
            "0",
            ModelPart::leaf(FROG_RIGHT_FOOT_POSE, FROG_RIGHT_FOOT_CUBES.to_vec()),
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

// ----- `FrogAnimation.FROG_TONGUE` (length 0.5s, NOT looping). The `head` dips down (-60° xRot) for
// the lash and back; the `tongue` rocks (-18° xRot) and lashes forward via a z-SCALE to 5×
// (`scaleVec(0.5, 1, 5)`) then retracts. All keyframes LINEAR. NOTE the vanilla quirk: the `head`
// SCALE channel uses `degreeVec` (not `scaleVec`), so the head gains a tiny `~0.0174` scale offset
// for the whole lash (`degreeVec(1,1,1) = (π/180, …)`) — transcribed exactly. -----

const FROG_TONGUE_HEAD_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.0833, degree_vec(-60.0, 0.0, 0.0), LINEAR),
    keyframe(0.4167, degree_vec(-60.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const FROG_TONGUE_HEAD_SCALE: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(0.0833, degree_vec(0.998, 1.0, 1.0), LINEAR),
    keyframe(0.4167, degree_vec(0.998, 1.0, 1.0), LINEAR),
    keyframe(0.5, degree_vec(1.0, 1.0, 1.0), LINEAR),
];
const FROG_TONGUE_TONGUE_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.0833, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.4167, degree_vec(-18.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const FROG_TONGUE_TONGUE_SCALE: [Keyframe; 3] = [
    keyframe(0.0833, scale_vec(1.0, 1.0, 1.0), LINEAR),
    keyframe(0.1667, scale_vec(0.5, 1.0, 5.0), LINEAR),
    keyframe(0.4167, scale_vec(1.0, 1.0, 1.0), LINEAR),
];

const FROG_TONGUE_HEAD_CHANNELS: [AnimationChannel; 2] = [
    rot(&FROG_TONGUE_HEAD_ROT),
    scale_channel(&FROG_TONGUE_HEAD_SCALE),
];
const FROG_TONGUE_TONGUE_CHANNELS: [AnimationChannel; 2] = [
    rot(&FROG_TONGUE_TONGUE_ROT),
    scale_channel(&FROG_TONGUE_TONGUE_SCALE),
];
const FROG_TONGUE_BONES: [BoneAnimation; 2] = [
    BoneAnimation {
        bone: "head",
        channels: &FROG_TONGUE_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "tongue",
        channels: &FROG_TONGUE_TONGUE_CHANNELS,
    },
];

/// Vanilla `FrogAnimation.FROG_TONGUE`: the triggered 0.5s tongue-lash (NOT looping), sampled by
/// `FrogModel.setupAnim` via `tongueAnimation.apply(tongueAnimationState, ageInTicks)` while the frog
/// is in `Pose.USING_TONGUE`. The renderer applies it only when the projected `frog_tongue_seconds >=
/// 0`. The head dips and the `tongue` part scales forward; the cross-entity prey-targeting that aims
/// the tongue at the eaten entity is NOT part of the model animation and is not reproduced.
pub(in crate::entity_models) const FROG_TONGUE: AnimationDefinition = AnimationDefinition {
    length_seconds: 0.5,
    looping: false,
    bones: &FROG_TONGUE_BONES,
};

// ----- `FrogAnimation.FROG_JUMP` (length 0.5s, NOT looping). A static long-jump hold pose: every
// channel holds one value across both `0.0` and `0.5` keyframes (LINEAR). The `body` tips back
// `-22.5°`; the two arms tuck back `-56.14°` and lift `+1` y; the two legs cock `45°`. `posVec`
// negates the y axis and `degreeVec` converts to radians. -----

const FROG_JUMP_BODY_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(-22.5, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(-22.5, 0.0, 0.0), LINEAR),
];
const FROG_JUMP_BODY_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const FROG_JUMP_LEFT_ARM_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(-56.14, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(-56.14, 0.0, 0.0), LINEAR),
];
const FROG_JUMP_LEFT_ARM_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 1.0, 0.0), LINEAR),
    keyframe(0.5, pos_vec(0.0, 1.0, 0.0), LINEAR),
];
const FROG_JUMP_RIGHT_ARM_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(-56.14, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(-56.14, 0.0, 0.0), LINEAR),
];
const FROG_JUMP_RIGHT_ARM_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 1.0, 0.0), LINEAR),
    keyframe(0.5, pos_vec(0.0, 1.0, 0.0), LINEAR),
];
const FROG_JUMP_LEFT_LEG_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(45.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(45.0, 0.0, 0.0), LINEAR),
];
const FROG_JUMP_LEFT_LEG_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const FROG_JUMP_RIGHT_LEG_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(45.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(45.0, 0.0, 0.0), LINEAR),
];
const FROG_JUMP_RIGHT_LEG_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
];

const FROG_JUMP_BODY_CHANNELS: [AnimationChannel; 2] =
    [rot(&FROG_JUMP_BODY_ROT), pos(&FROG_JUMP_BODY_POS)];
const FROG_JUMP_LEFT_ARM_CHANNELS: [AnimationChannel; 2] =
    [rot(&FROG_JUMP_LEFT_ARM_ROT), pos(&FROG_JUMP_LEFT_ARM_POS)];
const FROG_JUMP_RIGHT_ARM_CHANNELS: [AnimationChannel; 2] =
    [rot(&FROG_JUMP_RIGHT_ARM_ROT), pos(&FROG_JUMP_RIGHT_ARM_POS)];
const FROG_JUMP_LEFT_LEG_CHANNELS: [AnimationChannel; 2] =
    [rot(&FROG_JUMP_LEFT_LEG_ROT), pos(&FROG_JUMP_LEFT_LEG_POS)];
const FROG_JUMP_RIGHT_LEG_CHANNELS: [AnimationChannel; 2] =
    [rot(&FROG_JUMP_RIGHT_LEG_ROT), pos(&FROG_JUMP_RIGHT_LEG_POS)];

const FROG_JUMP_BONES: [BoneAnimation; 5] = [
    BoneAnimation {
        bone: "body",
        channels: &FROG_JUMP_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "left_arm",
        channels: &FROG_JUMP_LEFT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "right_arm",
        channels: &FROG_JUMP_RIGHT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_leg",
        channels: &FROG_JUMP_LEFT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_leg",
        channels: &FROG_JUMP_RIGHT_LEG_CHANNELS,
    },
];

/// Vanilla `FrogAnimation.FROG_JUMP`: the triggered 0.5s long-jump hold pose (NOT looping), sampled
/// by `FrogModel.setupAnim` via `jumpAnimation.apply(jumpAnimationState, ageInTicks)` (before the
/// croak) while the frog is in `Pose.LONG_JUMPING`. The renderer applies it only when the projected
/// `frog_jump_seconds >= 0`.
pub(in crate::entity_models) const FROG_JUMP: AnimationDefinition = AnimationDefinition {
    length_seconds: 0.5,
    looping: false,
    bones: &FROG_JUMP_BONES,
};

// ----- `FrogAnimation.FROG_IDLE_WATER` (length 3.0s, LOOPING). A slow hover wave for a frog idling
// underwater: the `body` dips `-10°`, the two arms splay `±22.5°→±45°` and sink `-0.5` y, and the
// two legs swing out (`22.5°` x, `±22.5°` y, with a `±45°` z mid-cycle) and sink `-1` y. Every
// keyframe is CATMULLROM (the cubic spline). `degreeVec` converts to radians and `posVec` negates
// the y axis. -----

const CATMULLROM: KeyframeInterpolation = KeyframeInterpolation::CatmullRom;

const FROG_IDLE_WATER_BODY_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.625, degree_vec(-10.0, 0.0, 0.0), CATMULLROM),
    keyframe(3.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const FROG_IDLE_WATER_LEFT_ARM_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(0.0, 0.0, -22.5), CATMULLROM),
    keyframe(2.2083, degree_vec(0.0, 0.0, -45.0), CATMULLROM),
    keyframe(3.0, degree_vec(0.0, 0.0, -22.5), CATMULLROM),
];
const FROG_IDLE_WATER_LEFT_ARM_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(-1.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.2083, pos_vec(-1.0, -0.5, 0.0), CATMULLROM),
    keyframe(3.0, pos_vec(-1.0, 0.0, 0.0), CATMULLROM),
];
const FROG_IDLE_WATER_RIGHT_ARM_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 22.5), CATMULLROM),
    keyframe(2.2083, degree_vec(0.0, 0.0, 45.0), CATMULLROM),
    keyframe(3.0, degree_vec(0.0, 0.0, 22.5), CATMULLROM),
];
const FROG_IDLE_WATER_RIGHT_ARM_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(1.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.2083, pos_vec(1.0, -0.5, 0.0), CATMULLROM),
    keyframe(3.0, pos_vec(1.0, 0.0, 0.0), CATMULLROM),
];
const FROG_IDLE_WATER_LEFT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(22.5, -22.5, 0.0), CATMULLROM),
    keyframe(1.0, degree_vec(22.5, -22.5, -45.0), CATMULLROM),
    keyframe(3.0, degree_vec(22.5, -22.5, 0.0), CATMULLROM),
];
const FROG_IDLE_WATER_LEFT_LEG_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 1.0), CATMULLROM),
    keyframe(1.0, pos_vec(0.0, -1.0, 1.0), CATMULLROM),
    keyframe(3.0, pos_vec(0.0, 0.0, 1.0), CATMULLROM),
];
const FROG_IDLE_WATER_RIGHT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(22.5, 22.5, 0.0), CATMULLROM),
    keyframe(1.0, degree_vec(22.5, 22.5, 45.0), CATMULLROM),
    keyframe(3.0, degree_vec(22.5, 22.5, 0.0), CATMULLROM),
];
const FROG_IDLE_WATER_RIGHT_LEG_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 1.0), CATMULLROM),
    keyframe(1.0, pos_vec(0.0, -1.0, 1.0), CATMULLROM),
    keyframe(3.0, pos_vec(0.0, 0.0, 1.0), CATMULLROM),
];

const FROG_IDLE_WATER_BODY_CHANNELS: [AnimationChannel; 1] = [rot(&FROG_IDLE_WATER_BODY_ROT)];
const FROG_IDLE_WATER_LEFT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&FROG_IDLE_WATER_LEFT_ARM_ROT),
    pos(&FROG_IDLE_WATER_LEFT_ARM_POS),
];
const FROG_IDLE_WATER_RIGHT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&FROG_IDLE_WATER_RIGHT_ARM_ROT),
    pos(&FROG_IDLE_WATER_RIGHT_ARM_POS),
];
const FROG_IDLE_WATER_LEFT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&FROG_IDLE_WATER_LEFT_LEG_ROT),
    pos(&FROG_IDLE_WATER_LEFT_LEG_POS),
];
const FROG_IDLE_WATER_RIGHT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&FROG_IDLE_WATER_RIGHT_LEG_ROT),
    pos(&FROG_IDLE_WATER_RIGHT_LEG_POS),
];

const FROG_IDLE_WATER_BONES: [BoneAnimation; 5] = [
    BoneAnimation {
        bone: "body",
        channels: &FROG_IDLE_WATER_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "left_arm",
        channels: &FROG_IDLE_WATER_LEFT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "right_arm",
        channels: &FROG_IDLE_WATER_RIGHT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_leg",
        channels: &FROG_IDLE_WATER_LEFT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_leg",
        channels: &FROG_IDLE_WATER_RIGHT_LEG_CHANNELS,
    },
];

/// Vanilla `FrogAnimation.FROG_IDLE_WATER`: the looping 3.0s in-water idle hover, sampled by
/// `FrogModel.setupAnim` via `idleWaterAnimation.apply(swimIdleAnimationState, ageInTicks)` LAST
/// (after the walk/swim, croak, and jump) while the frog idles underwater. The renderer applies it
/// only when the projected `frog_swim_idle_seconds >= 0`.
pub(in crate::entity_models) const FROG_IDLE_WATER: AnimationDefinition = AnimationDefinition {
    length_seconds: 3.0,
    looping: true,
    bones: &FROG_IDLE_WATER_BONES,
};

/// Mutable frog model, mirroring vanilla `FrogModel`. The cubeless `root` part (parenting `body`
/// and the two legs; `body` parents the head, croaking_body pouch, tongue, and two arms) hangs off
/// a synthetic root, built from the baked colored geometry as a named-children tree. Colored-only:
/// `setup_anim` applies the looping `FROG_WALK` keyframe cycle to the body, arms, and legs, the
/// triggered `FROG_JUMP` long-jump hold pose while long-jumping, the triggered `FROG_CROAK` pouch
/// animation while croaking, and the looping `FROG_IDLE_WATER` hover while idling underwater (the
/// tongue and the moving swim/walk cycles stay deferred).
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

        // Vanilla `FrogModel.setupAnim` applies `jumpAnimation.apply(jumpAnimationState, ageInTicks)`
        // first (before the croak). The projected `frog_jump_seconds` carries the elapsed seconds
        // since the long-jump started, or `-1` when the frog is not long-jumping (the
        // `jumpAnimationState` is stopped). While jumping, the static `FROG_JUMP` POSITION/ROTATION
        // hold pose is added onto the walk pose of the body, arms, and legs; otherwise it is skipped.
        let jump_seconds = instance.render_state.frog_jump_seconds;
        let jump = |part: &mut ModelPart, bone: &str| {
            if jump_seconds < 0.0 {
                return;
            }
            let elapsed = keyframe_elapsed_seconds(&FROG_JUMP, jump_seconds);
            let (position, rotation) = sample_bone_offsets(&FROG_JUMP, bone, elapsed, 1.0);
            part.pose = keyframe_animated_pose(part.pose, position, rotation);
        };

        // Vanilla `FrogModel.setupAnim` applies `idleWaterAnimation.apply(swimIdleAnimationState,
        // ageInTicks)` LAST (after the walk/swim, jump, and croak). The projected
        // `frog_swim_idle_seconds` carries the elapsed seconds since the in-water idle started
        // (`Frog.tick` drives it off the per-tick `isInWater() && !walkAnimation.isMoving()`), or
        // `-1` when the frog is dry or moving (the `swimIdleAnimationState` is stopped). While idling
        // underwater, the looping `FROG_IDLE_WATER` ROTATION/POSITION hover is added onto the
        // body, arms, and legs; otherwise it is skipped.
        let swim_idle_seconds = instance.render_state.frog_swim_idle_seconds;
        let swim_idle = |part: &mut ModelPart, bone: &str| {
            if swim_idle_seconds < 0.0 {
                return;
            }
            let elapsed = keyframe_elapsed_seconds(&FROG_IDLE_WATER, swim_idle_seconds);
            let (position, rotation) = sample_bone_offsets(&FROG_IDLE_WATER, bone, elapsed, 1.0);
            part.pose = keyframe_animated_pose(part.pose, position, rotation);
        };

        // Vanilla `FrogModel.setupAnim` then runs `croakAnimation.apply(croakAnimationState,
        // ageInTicks)` and `croakingBody.visible = croakAnimationState.isStarted()`. The projected
        // `frog_croak_seconds` carries the elapsed seconds since the croak started, or `-1` when the
        // frog is not croaking (the `croakAnimationState` is stopped). While croaking, the pouch is
        // shown and the `FROG_CROAK` POSITION/SCALE channels lift and puff it; otherwise it stays
        // hidden at its collapsed bind pose.
        let croak_seconds = instance.render_state.frog_croak_seconds;

        // Vanilla `FrogModel.setupAnim` also runs `tongueAnimation.apply(tongueAnimationState,
        // ageInTicks)`. The projected `frog_tongue_seconds` carries the elapsed seconds since the
        // `Pose.USING_TONGUE` lash started, or `-1` when the frog is not using its tongue. While
        // active, the `FROG_TONGUE` ROTATION/SCALE channels dip the `head` and lash the `tongue`
        // forward (a z-scale); otherwise the head and tongue hold their bind pose. (Additive offsets,
        // like the other one-shots, so the apply order vs vanilla does not matter.)
        let tongue_seconds = instance.render_state.frog_tongue_seconds;
        let tongue_apply = |part: &mut ModelPart, bone: &str| {
            if tongue_seconds < 0.0 {
                return;
            }
            let elapsed = keyframe_elapsed_seconds(&FROG_TONGUE, tongue_seconds);
            let (position, rotation, scale_offset) =
                sample_bone_offsets_with_scale(&FROG_TONGUE, bone, elapsed, 1.0);
            part.pose = keyframe_animated_pose(part.pose, position, rotation);
            part.scale = keyframe_animated_scale(scale_offset);
        };

        let frog_root = self.root.child_mut("root");
        {
            let body = frog_root.child_mut("body");
            animate(body, "body");
            jump(body, "body");
            swim_idle(body, "body");
            {
                let left_arm = body.child_mut("left_arm");
                animate(left_arm, "left_arm");
                jump(left_arm, "left_arm");
                swim_idle(left_arm, "left_arm");
            }
            {
                let right_arm = body.child_mut("right_arm");
                animate(right_arm, "right_arm");
                jump(right_arm, "right_arm");
                swim_idle(right_arm, "right_arm");
            }

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

            // The `head` dips and the `tongue` lashes during `Pose.USING_TONGUE`; both hold the bind
            // pose otherwise (the frog model applies no head look).
            tongue_apply(body.child_mut("head"), "head");
            tongue_apply(body.child_mut("tongue"), "tongue");
        }
        {
            let left_leg = frog_root.child_mut("left_leg");
            animate(left_leg, "left_leg");
            jump(left_leg, "left_leg");
            swim_idle(left_leg, "left_leg");
        }
        {
            let right_leg = frog_root.child_mut("right_leg");
            animate(right_leg, "right_leg");
            jump(right_leg, "right_leg");
            swim_idle(right_leg, "right_leg");
        }
    }
}
