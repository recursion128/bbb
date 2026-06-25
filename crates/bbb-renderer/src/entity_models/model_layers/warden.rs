use super::super::keyframe::{
    degree_vec, keyframe, keyframe_animated_pose, keyframe_elapsed_seconds, pos_vec,
    sample_bone_offsets, AnimationChannel, AnimationDefinition, AnimationTarget, BoneAnimation,
    Keyframe, KeyframeInterpolation,
};
use super::{PartPose, PART_POSE_ZERO, WARDEN_BODY, WARDEN_TENDRIL};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

const CATMULLROM: KeyframeInterpolation = KeyframeInterpolation::CatmullRom;
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

// Vanilla 26.1 `WardenModel.createBodyLayer` (atlas 128×128). The mesh root holds one `bone` part
// at `offset(0, 24, 0)` parenting the body and the two legs; `body` parents the two ribcage
// planes, the head (which parents the two tendril planes), and the two arms. Four non-keyframe
// `WardenModel.setupAnim` motions are reproduced ([`warden_head_pose`] / [`warden_idle_body_pose`] /
// [`warden_walk_pose`] / [`warden_tendril_x_rot`]): the head look (`animateHeadLookTarget`), the
// always-on idle wobble (`animateIdlePose`), the walk (`animateWalk`, which swings the head, body,
// legs, and arms off `walkAnimationPos/Speed` and composes additively onto the look/idle pose via
// [`warden_add_x_z_rot`]), and the tendril sway (`animateTendrils`, which swings the two head
// tendrils off the projected `tendrilAnimation` pulse and `ageInTicks`). The four triggered combat
// keyframe animations are also reproduced and applied additively in the vanilla `setupAnim` order
// (attack → sonic_boom → dig → emerge → roar → sniff): [`WARDEN_ATTACK`] (event 4),
// [`WARDEN_SONIC_BOOM`] (event 62), [`WARDEN_DIG`] (`Pose.DIGGING`), [`WARDEN_EMERGE`]
// (`Pose.EMERGING`), [`WARDEN_ROAR`] (`Pose.ROARING`), and [`WARDEN_SNIFF`] (`Pose.SNIFFING`), each
// applied only when its projected elapsed-seconds value is `>= 0`. The four emissive overlay layers
// (tendrils, heart, bioluminescent, pulsating spots) and the texture-backed path are deferred.

// `body`: one 18×21×11 box (`texOffs(0,0)`).
pub(in crate::entity_models) const WARDEN_BODY_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-9.0, -13.0, -4.0],
    [18.0, 21.0, 11.0],
    WARDEN_BODY,
    [18.0, 21.0, 11.0],
    [0.0, 0.0],
    false,
)];

// The two ribcage planes (`texOffs(90,11)`, the left mirrored); both are 9×21×0.
pub(in crate::entity_models) const WARDEN_RIGHT_RIBCAGE_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -11.0, -0.1],
    [9.0, 21.0, 0.0],
    WARDEN_BODY,
    [9.0, 21.0, 0.0],
    [90.0, 11.0],
    false,
)];
pub(in crate::entity_models) const WARDEN_LEFT_RIBCAGE_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-7.0, -11.0, -0.1],
    [9.0, 21.0, 0.0],
    WARDEN_BODY,
    [9.0, 21.0, 0.0],
    [90.0, 11.0],
    true,
)];

// `head`: one 16×16×10 box (`texOffs(0,32)`).
pub(in crate::entity_models) const WARDEN_HEAD_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-8.0, -16.0, -5.0],
    [16.0, 16.0, 10.0],
    WARDEN_BODY,
    [16.0, 16.0, 10.0],
    [0.0, 32.0],
    false,
)];

// The two tendril planes (16×16×0), the warden's iconic glow antennae. The right tendril is
// `texOffs(52,32)`, the left `texOffs(58,0)` (distinct UV regions, not mirrors).
pub(in crate::entity_models) const WARDEN_RIGHT_TENDRIL_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-16.0, -13.0, 0.0],
    [16.0, 16.0, 0.0],
    WARDEN_TENDRIL,
    [16.0, 16.0, 0.0],
    [52.0, 32.0],
    false,
)];
pub(in crate::entity_models) const WARDEN_LEFT_TENDRIL_CUBES: [ModelCube; 1] = [ModelCube::new(
    [0.0, -13.0, 0.0],
    [16.0, 16.0, 0.0],
    WARDEN_TENDRIL,
    [16.0, 16.0, 0.0],
    [58.0, 0.0],
    false,
)];

// Both arms add the identical 8×28×8 box (`mirror=false`) but draw from DISTINCT UV regions: the
// right arm `texOffs(44,50)`, the left arm `texOffs(0,58)` (these are NOT mirrors).
pub(in crate::entity_models) const WARDEN_RIGHT_ARM_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.0, -4.0],
    [8.0, 28.0, 8.0],
    WARDEN_BODY,
    [8.0, 28.0, 8.0],
    [44.0, 50.0],
    false,
)];
pub(in crate::entity_models) const WARDEN_LEFT_ARM_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.0, -4.0],
    [8.0, 28.0, 8.0],
    WARDEN_BODY,
    [8.0, 28.0, 8.0],
    [0.0, 58.0],
    false,
)];

// The legs (6×13×6) differ in X origin and UV: the right `texOffs(76,48)`, the left `texOffs(76,76)`.
pub(in crate::entity_models) const WARDEN_RIGHT_LEG_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-3.1, 0.0, -3.0],
    [6.0, 13.0, 6.0],
    WARDEN_BODY,
    [6.0, 13.0, 6.0],
    [76.0, 48.0],
    false,
)];
pub(in crate::entity_models) const WARDEN_LEFT_LEG_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-2.9, 0.0, -3.0],
    [6.0, 13.0, 6.0],
    WARDEN_BODY,
    [6.0, 13.0, 6.0],
    [76.0, 76.0],
    false,
)];

/// Vanilla `WardenModel.createBodyLayer` rest-pose part poses, rooted at the cubeless `bone` part
/// (`offset(0, 24, 0)`) parenting the `body` and the two legs; `body` parents the two ribcages, the
/// `head` (which parents the two tendrils), and the two arms. Ten cubes.
/// `bone` cubeless-pivot part pose: `PartPose.offset(0, 24, 0)`.
pub(in crate::entity_models) const WARDEN_BONE_POSE: PartPose = PartPose {
    offset: [0.0, 24.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `body` part pose: `PartPose.offset(0, -21, 0)`. The idle wobble and walk swing add onto its bind.
pub(in crate::entity_models) const WARDEN_BODY_POSE: PartPose = PartPose {
    offset: [0.0, -21.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_ribcage` part pose: `PartPose.offset(-7, -2, -4)`.
pub(in crate::entity_models) const WARDEN_RIGHT_RIBCAGE_POSE: PartPose = PartPose {
    offset: [-7.0, -2.0, -4.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_ribcage` part pose: `PartPose.offset(7, -2, -4)`.
pub(in crate::entity_models) const WARDEN_LEFT_RIBCAGE_POSE: PartPose = PartPose {
    offset: [7.0, -2.0, -4.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `head` part pose: `PartPose.offset(0, -13, 0)`. `animateHeadLookTarget` sets `head.xRot/yRot`
/// from the look; the tendrils nested under the head inherit the turn.
pub(in crate::entity_models) const WARDEN_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, -13.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_tendril` part pose: `PartPose.offset(-8, -12, 0)`. Vanilla sets `rightTendril.xRot =
/// -tendrilXRot`.
pub(in crate::entity_models) const WARDEN_RIGHT_TENDRIL_POSE: PartPose = PartPose {
    offset: [-8.0, -12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_tendril` part pose: `PartPose.offset(8, -12, 0)`. Vanilla sets `leftTendril.xRot =
/// +tendrilXRot`.
pub(in crate::entity_models) const WARDEN_LEFT_TENDRIL_POSE: PartPose = PartPose {
    offset: [8.0, -12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_arm` part pose: `PartPose.offset(-13, -13, 1)`. `animateWalk` swings its `xRot`.
pub(in crate::entity_models) const WARDEN_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-13.0, -13.0, 1.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_arm` part pose: `PartPose.offset(13, -13, 1)`. `animateWalk` swings its `xRot`.
pub(in crate::entity_models) const WARDEN_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [13.0, -13.0, 1.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_leg` part pose: `PartPose.offset(-5.9, -13, 0)`. `animateWalk` swings its `xRot`.
pub(in crate::entity_models) const WARDEN_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-5.9, -13.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_leg` part pose: `PartPose.offset(5.9, -13, 0)`. `animateWalk` swings its `xRot`.
pub(in crate::entity_models) const WARDEN_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [5.9, -13.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds the warden's synthetic root parenting the single cubeless `bone` part, which parents the
/// cube-bearing `body` (→ two ribcages / `head` → two tendrils / two arms) and the two legs, in
/// vanilla `addOrReplaceChild` order. The `bone`, `body`, `head`, both tendrils, both arms, and both
/// legs are name-addressed by `setup_anim`, so `bone`, `body`, and `head` carry named children.
fn warden_root() -> ModelPart {
    let head = ModelPart::new(
        WARDEN_HEAD_POSE,
        WARDEN_HEAD_CUBES.to_vec(),
        vec![
            (
                "right_tendril",
                ModelPart::leaf(
                    WARDEN_RIGHT_TENDRIL_POSE,
                    WARDEN_RIGHT_TENDRIL_CUBES.to_vec(),
                ),
            ),
            (
                "left_tendril",
                ModelPart::leaf(WARDEN_LEFT_TENDRIL_POSE, WARDEN_LEFT_TENDRIL_CUBES.to_vec()),
            ),
        ],
    );
    let body = ModelPart::new(
        WARDEN_BODY_POSE,
        WARDEN_BODY_CUBES.to_vec(),
        vec![
            (
                "right_ribcage",
                ModelPart::leaf(
                    WARDEN_RIGHT_RIBCAGE_POSE,
                    WARDEN_RIGHT_RIBCAGE_CUBES.to_vec(),
                ),
            ),
            (
                "left_ribcage",
                ModelPart::leaf(WARDEN_LEFT_RIBCAGE_POSE, WARDEN_LEFT_RIBCAGE_CUBES.to_vec()),
            ),
            ("head", head),
            (
                "right_arm",
                ModelPart::leaf(WARDEN_RIGHT_ARM_POSE, WARDEN_RIGHT_ARM_CUBES.to_vec()),
            ),
            (
                "left_arm",
                ModelPart::leaf(WARDEN_LEFT_ARM_POSE, WARDEN_LEFT_ARM_CUBES.to_vec()),
            ),
        ],
    );
    let bone = ModelPart::new(
        WARDEN_BONE_POSE,
        Vec::new(),
        vec![
            ("body", body),
            (
                "right_leg",
                ModelPart::leaf(WARDEN_RIGHT_LEG_POSE, WARDEN_RIGHT_LEG_CUBES.to_vec()),
            ),
            (
                "left_leg",
                ModelPart::leaf(WARDEN_LEFT_LEG_POSE, WARDEN_LEFT_LEG_CUBES.to_vec()),
            ),
        ],
    );
    ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![("bone", bone)])
}

/// Vanilla `WardenModel.animateIdlePose` body roll: with `s = ageInTicks·0.1`, the body adds
/// `xRot += 0.025·cos(s)` and `zRot += 0.025·sin(s)` onto its bind pose. Always on (no gating
/// state), so every warden sways gently. Mirrors the head roll in [`warden_head_pose`].
pub(in crate::entity_models) fn warden_idle_body_pose(
    base: PartPose,
    age_in_ticks: f32,
) -> PartPose {
    let s = age_in_ticks * 0.1;
    PartPose {
        offset: base.offset,
        rotation: [
            base.rotation[0] + 0.025 * s.cos(),
            base.rotation[1],
            base.rotation[2] + 0.025 * s.sin(),
        ],
    }
}

/// Vanilla `WardenModel` head pose: `animateHeadLookTarget` first sets `head.xRot = xRot·π/180`,
/// `head.yRot = yRot·π/180` (overwriting the bind), then `animateIdlePose` adds the always-on roll
/// `head.xRot += 0.06·sin(s)`, `head.zRot += 0.06·cos(s)` with `s = ageInTicks·0.1`. The walk
/// ([`warden_walk_pose`]) then adds further to `head.xRot/zRot` via [`warden_add_x_z_rot`]. The base
/// `head.zRot` is the bind `0`, so the idle roll lands on `base.rotation[2]`.
pub(in crate::entity_models) fn warden_head_pose(
    base: PartPose,
    head_yaw_deg: f32,
    head_pitch_deg: f32,
    age_in_ticks: f32,
) -> PartPose {
    let s = age_in_ticks * 0.1;
    PartPose {
        offset: base.offset,
        rotation: [
            head_pitch_deg.to_radians() + 0.06 * s.sin(),
            head_yaw_deg.to_radians(),
            base.rotation[2] + 0.06 * s.cos(),
        ],
    }
}

/// The per-bone `xRot`/`zRot` offsets produced by vanilla `WardenModel.animateWalk(walkPos,
/// walkSpeed)`. Every term derives from `speedModifier = min(0.5, 3·walkSpeed)`,
/// `speedModifierWithMin = min(0.35, speedModifier)`, and `adjustedPos = walkPos·0.8662`. The head
/// and body offsets ADD onto the look/idle composition (vanilla uses `+=` on the head and SETs the
/// body, whose bind rotation is zero); the legs and arms are SET from their zero bind rotation.
/// `animateWalk` ends with `resetArmPoses`, which restores the arms' bind position and `yRot`, so
/// only the arm `xRot` moves. Because all of these compose additively onto a zero/known base, the
/// warden emit applies them through [`warden_add_x_z_rot`] after the look/idle pass — addition is
/// commutative, so the vanilla order (look → walk → idle) is preserved.
pub(in crate::entity_models) struct WardenWalkPose {
    pub(in crate::entity_models) head_x_rot: f32,
    pub(in crate::entity_models) head_z_rot: f32,
    pub(in crate::entity_models) body_x_rot: f32,
    pub(in crate::entity_models) body_z_rot: f32,
    pub(in crate::entity_models) left_leg_x_rot: f32,
    pub(in crate::entity_models) right_leg_x_rot: f32,
    pub(in crate::entity_models) left_arm_x_rot: f32,
    pub(in crate::entity_models) right_arm_x_rot: f32,
}

/// Samples vanilla `WardenModel.animateWalk` into a [`WardenWalkPose`]. At `walkSpeed = 0` every
/// term is zero, so a standing warden adds nothing on top of the look/idle pose.
pub(in crate::entity_models) fn warden_walk_pose(walk_pos: f32, walk_speed: f32) -> WardenWalkPose {
    use std::f32::consts::{FRAC_PI_2, PI};
    let speed = (3.0 * walk_speed).min(0.5);
    let speed_with_min = speed.min(0.35);
    let adjusted_pos = walk_pos * 0.8662;
    let cos = adjusted_pos.cos();
    let sin = adjusted_pos.sin();
    WardenWalkPose {
        head_x_rot: 1.2 * (adjusted_pos + FRAC_PI_2).cos() * speed_with_min,
        head_z_rot: 0.3 * sin * speed,
        body_x_rot: 1.0 * cos * speed_with_min,
        body_z_rot: 0.1 * sin * speed,
        left_leg_x_rot: 1.0 * cos * speed,
        right_leg_x_rot: 1.0 * (adjusted_pos + PI).cos() * speed,
        left_arm_x_rot: -(0.8 * cos * speed),
        right_arm_x_rot: -(0.8 * sin * speed),
    }
}

/// Adds `x_rot`/`z_rot` onto a pose's `rotation[0]`/`rotation[2]`, leaving `rotation[1]` (yRot) and
/// the offset untouched. The warden's [`warden_walk_pose`] offsets compose this way onto every
/// animated bone (the head/body after their look/idle pass, the legs/arms over their bind pose).
pub(in crate::entity_models) fn warden_add_x_z_rot(
    base: PartPose,
    x_rot: f32,
    z_rot: f32,
) -> PartPose {
    PartPose {
        offset: base.offset,
        rotation: [
            base.rotation[0] + x_rot,
            base.rotation[1],
            base.rotation[2] + z_rot,
        ],
    }
}

/// Vanilla `WardenModel.animateTendrils`: the magnitude of the tendril `xRot` sway,
/// `tendrilXRot = tendrilAnimation · cos(ageInTicks · 2.25) · π · 0.1`. The left tendril takes
/// `+tendrilXRot` and the right takes `-tendrilXRot`. `tendrilAnimation` is the projected `0..=1`
/// pulse (`Warden.getTendrilAnimation`), so a resting warden (`tendrilAnimation = 0`) holds its
/// antennae at bind. Vanilla computes the `cos·π·0.1` factor in double precision before the
/// `(float)` cast, which this mirrors.
pub(in crate::entity_models) fn warden_tendril_x_rot(
    tendril_animation: f32,
    age_in_ticks: f32,
) -> f32 {
    let factor = ((age_in_ticks as f64 * 2.25).cos() * std::f64::consts::PI * 0.1) as f32;
    tendril_animation * factor
}

// ----- `WardenAnimation.WARDEN_ATTACK` (length 0.33333s, NOT looping). The melee swing: the `body`
// rocks and dips forward, the `head` whips down, and the two arms slam (each ROTATION + POSITION).
// All keyframes are CATMULLROM; `posVec` negates the y axis and `degreeVec` converts to radians. -----

const WARDEN_ATTACK_BODY_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.0417, degree_vec(-22.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.2083, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.3333, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_ATTACK_BODY_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.0417, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.2083, pos_vec(0.0, -1.0, -2.0), CATMULLROM),
    keyframe(0.3333, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_ATTACK_HEAD_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.0417, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, degree_vec(-30.17493, 0.0, 0.0), CATMULLROM),
    keyframe(0.3333, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_ATTACK_HEAD_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.0417, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, pos_vec(0.0, -2.0, -2.0), CATMULLROM),
    keyframe(0.3333, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_ATTACK_RIGHT_ARM_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(
        0.0417,
        degree_vec(-120.36119, 40.78947, -20.94102),
        CATMULLROM,
    ),
    keyframe(0.1667, degree_vec(-90.0, -45.0, 0.0), CATMULLROM),
    keyframe(0.3333, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_ATTACK_RIGHT_ARM_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.0417, pos_vec(4.0, 0.0, 5.0), CATMULLROM),
    keyframe(0.1667, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.3333, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_ATTACK_LEFT_ARM_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(
        0.0417,
        degree_vec(-120.36119, -40.78947, 20.94102),
        CATMULLROM,
    ),
    keyframe(0.1667, degree_vec(-61.1632, 42.85882, 11.52421), CATMULLROM),
    keyframe(0.3333, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_ATTACK_LEFT_ARM_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.0417, pos_vec(-4.0, 0.0, 5.0), CATMULLROM),
    keyframe(0.1667, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.3333, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_ATTACK_BODY_CHANNELS: [AnimationChannel; 2] =
    [rot(&WARDEN_ATTACK_BODY_ROT), pos(&WARDEN_ATTACK_BODY_POS)];
const WARDEN_ATTACK_HEAD_CHANNELS: [AnimationChannel; 2] =
    [rot(&WARDEN_ATTACK_HEAD_ROT), pos(&WARDEN_ATTACK_HEAD_POS)];
const WARDEN_ATTACK_RIGHT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&WARDEN_ATTACK_RIGHT_ARM_ROT),
    pos(&WARDEN_ATTACK_RIGHT_ARM_POS),
];
const WARDEN_ATTACK_LEFT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&WARDEN_ATTACK_LEFT_ARM_ROT),
    pos(&WARDEN_ATTACK_LEFT_ARM_POS),
];
const WARDEN_ATTACK_BONES: [BoneAnimation; 4] = [
    BoneAnimation {
        bone: "body",
        channels: &WARDEN_ATTACK_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &WARDEN_ATTACK_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_arm",
        channels: &WARDEN_ATTACK_RIGHT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_arm",
        channels: &WARDEN_ATTACK_LEFT_ARM_CHANNELS,
    },
];
/// Vanilla `WardenAnimation.WARDEN_ATTACK`: the 0.33333s melee swing (NOT looping),
/// `attackAnimation.apply(attackAnimationState, ageInTicks)`. Started by entity event `4`, which
/// also stops the roar. The renderer applies it only while the projected `warden_attack_seconds
/// >= 0`, clamping past the length to the resting final frame.
pub(in crate::entity_models) const WARDEN_ATTACK: AnimationDefinition = AnimationDefinition {
    length_seconds: 0.33333,
    looping: false,
    bones: &WARDEN_ATTACK_BONES,
};

// ----- `WardenAnimation.WARDEN_SONIC_BOOM` (length 3.0s, NOT looping). The charge/blast: the `body`
// rears then rocks back, the two `ribcage` planes fan open (yaw ±125°), the `head` cranes up then
// snaps down, and the two arms wind up and thrust (ROTATION + POSITION). All keyframes CATMULLROM. --

const WARDEN_SONIC_BOOM_BODY_ROT: [Keyframe; 8] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.0833, degree_vec(47.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.625, degree_vec(55.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.9167, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.0, degree_vec(-32.5, 0.0, 0.0), CATMULLROM),
    keyframe(2.4583, degree_vec(-32.5, 0.0, 0.0), CATMULLROM),
    keyframe(2.7083, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.875, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_SONIC_BOOM_BODY_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.0833, pos_vec(0.0, -3.0, 0.0), CATMULLROM),
    keyframe(1.625, pos_vec(0.0, -4.0, -1.0), CATMULLROM),
    keyframe(1.9167, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.7083, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.875, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_SONIC_BOOM_RIGHT_RIBCAGE_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.5417, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.7917, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.875, degree_vec(0.0, 125.0, 0.0), CATMULLROM),
    keyframe(2.5, degree_vec(0.0, 125.0, 0.0), CATMULLROM),
    keyframe(2.6667, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_SONIC_BOOM_LEFT_RIBCAGE_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.5417, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.7917, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.875, degree_vec(0.0, -125.0, 0.0), CATMULLROM),
    keyframe(2.5, degree_vec(0.0, -125.0, 0.0), CATMULLROM),
    keyframe(2.6667, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_SONIC_BOOM_HEAD_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.0, degree_vec(67.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.75, degree_vec(80.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.9167, degree_vec(-45.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.5, degree_vec(-45.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.7083, degree_vec(-45.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.875, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_SONIC_BOOM_HEAD_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.9167, pos_vec(0.0, 0.0, -3.0), CATMULLROM),
    keyframe(2.5, pos_vec(0.0, 0.0, -3.0), CATMULLROM),
    keyframe(2.7083, pos_vec(0.0, 0.0, -3.0), CATMULLROM),
    keyframe(2.875, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_SONIC_BOOM_RIGHT_ARM_ROT: [Keyframe; 10] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(
        0.875,
        degree_vec(-42.28659, -32.69813, -5.00825),
        CATMULLROM,
    ),
    keyframe(
        1.1667,
        degree_vec(-29.83757, -35.39626, -45.28089),
        CATMULLROM,
    ),
    keyframe(
        1.3333,
        degree_vec(-29.83757, -35.39626, -45.28089),
        CATMULLROM,
    ),
    keyframe(
        1.6667,
        degree_vec(-72.28659, -32.69813, -5.00825),
        CATMULLROM,
    ),
    keyframe(1.8333, degree_vec(35.26439, -30.0, 35.26439), CATMULLROM),
    keyframe(1.9167, degree_vec(73.75484, -13.0931, 19.20518), CATMULLROM),
    keyframe(2.5, degree_vec(73.75484, -13.0931, 19.20518), CATMULLROM),
    keyframe(2.75, degree_vec(58.20713, -21.1064, 28.7261), CATMULLROM),
    keyframe(3.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_SONIC_BOOM_RIGHT_ARM_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.8333, pos_vec(3.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.75, pos_vec(3.0, 0.0, 0.0), CATMULLROM),
    keyframe(3.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_SONIC_BOOM_LEFT_ARM_ROT: [Keyframe; 10] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.875, degree_vec(-33.80694, 32.31058, 6.87997), CATMULLROM),
    keyframe(
        1.1667,
        degree_vec(-17.87827, 34.62115, 49.02433),
        CATMULLROM,
    ),
    keyframe(
        1.3333,
        degree_vec(-17.87827, 34.62115, 49.02433),
        CATMULLROM,
    ),
    keyframe(1.6667, degree_vec(-51.30694, 32.31058, 6.87997), CATMULLROM),
    keyframe(1.8333, degree_vec(35.26439, 30.0, -35.26439), CATMULLROM),
    keyframe(1.9167, degree_vec(73.75484, 13.0931, -19.20518), CATMULLROM),
    keyframe(2.5, degree_vec(73.75484, 13.0931, -19.20518), CATMULLROM),
    keyframe(2.75, degree_vec(58.20713, 21.1064, -28.7261), CATMULLROM),
    keyframe(3.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_SONIC_BOOM_LEFT_ARM_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.8333, pos_vec(-3.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.75, pos_vec(-3.0, 0.0, 0.0), CATMULLROM),
    keyframe(3.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_SONIC_BOOM_BODY_CHANNELS: [AnimationChannel; 2] = [
    rot(&WARDEN_SONIC_BOOM_BODY_ROT),
    pos(&WARDEN_SONIC_BOOM_BODY_POS),
];
const WARDEN_SONIC_BOOM_RIGHT_RIBCAGE_CHANNELS: [AnimationChannel; 1] =
    [rot(&WARDEN_SONIC_BOOM_RIGHT_RIBCAGE_ROT)];
const WARDEN_SONIC_BOOM_LEFT_RIBCAGE_CHANNELS: [AnimationChannel; 1] =
    [rot(&WARDEN_SONIC_BOOM_LEFT_RIBCAGE_ROT)];
const WARDEN_SONIC_BOOM_HEAD_CHANNELS: [AnimationChannel; 2] = [
    rot(&WARDEN_SONIC_BOOM_HEAD_ROT),
    pos(&WARDEN_SONIC_BOOM_HEAD_POS),
];
const WARDEN_SONIC_BOOM_RIGHT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&WARDEN_SONIC_BOOM_RIGHT_ARM_ROT),
    pos(&WARDEN_SONIC_BOOM_RIGHT_ARM_POS),
];
const WARDEN_SONIC_BOOM_LEFT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&WARDEN_SONIC_BOOM_LEFT_ARM_ROT),
    pos(&WARDEN_SONIC_BOOM_LEFT_ARM_POS),
];
const WARDEN_SONIC_BOOM_BONES: [BoneAnimation; 6] = [
    BoneAnimation {
        bone: "body",
        channels: &WARDEN_SONIC_BOOM_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "right_ribcage",
        channels: &WARDEN_SONIC_BOOM_RIGHT_RIBCAGE_CHANNELS,
    },
    BoneAnimation {
        bone: "left_ribcage",
        channels: &WARDEN_SONIC_BOOM_LEFT_RIBCAGE_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &WARDEN_SONIC_BOOM_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_arm",
        channels: &WARDEN_SONIC_BOOM_RIGHT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_arm",
        channels: &WARDEN_SONIC_BOOM_LEFT_ARM_CHANNELS,
    },
];
/// Vanilla `WardenAnimation.WARDEN_SONIC_BOOM`: the 3.0s charge/blast (NOT looping),
/// `sonicBoomAnimation.apply(sonicBoomAnimationState, ageInTicks)`. Started by entity event `62`.
/// The two ribcage planes fan open at the blast. The renderer applies it only while the projected
/// `warden_sonic_boom_seconds >= 0`, clamping past the length to the resting final frame.
pub(in crate::entity_models) const WARDEN_SONIC_BOOM: AnimationDefinition = AnimationDefinition {
    length_seconds: 3.0,
    looping: false,
    bones: &WARDEN_SONIC_BOOM_BONES,
};

// ----- `WardenAnimation.WARDEN_ROAR` (length 4.2s, NOT looping). The threat roar: the `body` rears
// up, the `head` shakes side to side, and the two arms fling wide and shudder (ROTATION + POSITION).
// All keyframes CATMULLROM. -----

const WARDEN_ROAR_BODY_ROT: [Keyframe; 8] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.24, degree_vec(-25.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.6, degree_vec(32.5, 0.0, -7.5), CATMULLROM),
    keyframe(1.84, degree_vec(38.33, 0.0, 2.99), CATMULLROM),
    keyframe(2.08, degree_vec(40.97, 0.0, -4.3), CATMULLROM),
    keyframe(2.36, degree_vec(44.41, 0.0, 6.29), CATMULLROM),
    keyframe(3.0, degree_vec(47.5, 0.0, 0.0), CATMULLROM),
    keyframe(4.2, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_ROAR_BODY_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.24, pos_vec(0.0, -1.0, 3.0), CATMULLROM),
    keyframe(1.6, pos_vec(0.0, -3.0, -6.0), CATMULLROM),
    keyframe(3.0, pos_vec(0.0, -3.0, -6.0), CATMULLROM),
    keyframe(4.2, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_ROAR_HEAD_ROT: [Keyframe; 8] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.24, degree_vec(-32.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.6, degree_vec(-32.5, 0.0, -27.5), CATMULLROM),
    keyframe(1.8, degree_vec(-32.5, 0.0, 26.0), CATMULLROM),
    keyframe(2.04, degree_vec(-32.5, 0.0, -27.5), CATMULLROM),
    keyframe(2.44, degree_vec(-32.5, 0.0, 26.0), CATMULLROM),
    keyframe(2.84, degree_vec(-5.0, 0.0, -12.5), CATMULLROM),
    keyframe(4.2, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_ROAR_HEAD_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.24, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.6, pos_vec(0.0, -2.0, -6.0), CATMULLROM),
    keyframe(2.2, pos_vec(0.0, -2.0, -6.0), CATMULLROM),
    keyframe(2.48, pos_vec(0.0, -2.0, -6.0), CATMULLROM),
    keyframe(4.2, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_ROAR_RIGHT_ARM_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.72, degree_vec(-120.0, 0.0, -20.0), CATMULLROM),
    keyframe(1.24, degree_vec(-77.5, 3.75, 15.0), CATMULLROM),
    keyframe(1.48, degree_vec(67.5, -32.5, 20.0), CATMULLROM),
    keyframe(2.48, degree_vec(37.5, -32.5, 25.0), CATMULLROM),
    keyframe(2.88, degree_vec(27.6, -17.1, 32.5), CATMULLROM),
    keyframe(4.2, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_ROAR_RIGHT_ARM_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.72, pos_vec(3.0, -2.0, 0.0), CATMULLROM),
    keyframe(1.48, pos_vec(4.0, -2.0, 0.0), CATMULLROM),
    keyframe(2.48, pos_vec(4.0, -2.0, 0.0), CATMULLROM),
    keyframe(4.2, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_ROAR_LEFT_ARM_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.72, degree_vec(-125.0, 0.0, 20.0), CATMULLROM),
    keyframe(1.24, degree_vec(-76.25, -17.5, -7.5), CATMULLROM),
    keyframe(1.48, degree_vec(62.5, 42.5, -12.5), CATMULLROM),
    keyframe(2.48, degree_vec(37.5, 27.5, -27.5), CATMULLROM),
    keyframe(2.88, degree_vec(25.0, 18.4, -30.0), CATMULLROM),
    keyframe(4.2, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_ROAR_LEFT_ARM_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.72, pos_vec(-3.0, -2.0, 0.0), CATMULLROM),
    keyframe(1.48, pos_vec(-4.0, -2.0, 0.0), CATMULLROM),
    keyframe(2.48, pos_vec(-4.0, -2.0, 0.0), CATMULLROM),
    keyframe(4.2, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_ROAR_BODY_CHANNELS: [AnimationChannel; 2] =
    [rot(&WARDEN_ROAR_BODY_ROT), pos(&WARDEN_ROAR_BODY_POS)];
const WARDEN_ROAR_HEAD_CHANNELS: [AnimationChannel; 2] =
    [rot(&WARDEN_ROAR_HEAD_ROT), pos(&WARDEN_ROAR_HEAD_POS)];
const WARDEN_ROAR_RIGHT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&WARDEN_ROAR_RIGHT_ARM_ROT),
    pos(&WARDEN_ROAR_RIGHT_ARM_POS),
];
const WARDEN_ROAR_LEFT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&WARDEN_ROAR_LEFT_ARM_ROT),
    pos(&WARDEN_ROAR_LEFT_ARM_POS),
];
const WARDEN_ROAR_BONES: [BoneAnimation; 4] = [
    BoneAnimation {
        bone: "body",
        channels: &WARDEN_ROAR_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &WARDEN_ROAR_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_arm",
        channels: &WARDEN_ROAR_RIGHT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_arm",
        channels: &WARDEN_ROAR_LEFT_ARM_CHANNELS,
    },
];
/// Vanilla `WardenAnimation.WARDEN_ROAR`: the 4.2s threat roar (NOT looping),
/// `roarAnimation.apply(roarAnimationState, ageInTicks)`. Started when the synced `DATA_POSE`
/// changes to `Pose.ROARING`; cancelled by the attack event. The renderer applies it only while the
/// projected `warden_roar_seconds >= 0`, clamping past the length to the resting final frame.
pub(in crate::entity_models) const WARDEN_ROAR: AnimationDefinition = AnimationDefinition {
    length_seconds: 4.2,
    looping: false,
    bones: &WARDEN_ROAR_BONES,
};

// ----- `WardenAnimation.WARDEN_SNIFF` (length 4.16s, NOT looping). The investigative sniff: the
// `body` and `head` turn and dip to scent, and the two arms ease (ROTATION only). All keyframes
// CATMULLROM. -----

const WARDEN_SNIFF_BODY_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.56, degree_vec(17.5, 32.5, 0.0), CATMULLROM),
    keyframe(0.96, degree_vec(0.0, 32.5, 0.0), CATMULLROM),
    keyframe(2.2, degree_vec(10.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.8, degree_vec(10.0, -30.0, 0.0), CATMULLROM),
    keyframe(3.32, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_SNIFF_HEAD_ROT: [Keyframe; 9] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.68, degree_vec(0.0, 40.0, 0.0), CATMULLROM),
    keyframe(0.96, degree_vec(-22.5, 40.0, 0.0), CATMULLROM),
    keyframe(1.24, degree_vec(0.0, 20.0, 0.0), CATMULLROM),
    keyframe(1.52, degree_vec(-35.0, 20.0, 0.0), CATMULLROM),
    keyframe(1.76, degree_vec(0.0, 20.0, 0.0), CATMULLROM),
    keyframe(2.28, degree_vec(0.0, -20.0, 0.0), CATMULLROM),
    keyframe(2.88, degree_vec(0.0, -20.0, 0.0), CATMULLROM),
    keyframe(3.32, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_SNIFF_RIGHT_ARM_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.96, degree_vec(17.5, 0.0, 0.0), CATMULLROM),
    keyframe(2.2, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.76, degree_vec(-15.0, 0.0, 0.0), CATMULLROM),
    keyframe(3.32, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_SNIFF_LEFT_ARM_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.96, degree_vec(-15.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.2, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.76, degree_vec(17.5, 0.0, 0.0), CATMULLROM),
    keyframe(3.32, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_SNIFF_BODY_CHANNELS: [AnimationChannel; 1] = [rot(&WARDEN_SNIFF_BODY_ROT)];
const WARDEN_SNIFF_HEAD_CHANNELS: [AnimationChannel; 1] = [rot(&WARDEN_SNIFF_HEAD_ROT)];
const WARDEN_SNIFF_RIGHT_ARM_CHANNELS: [AnimationChannel; 1] = [rot(&WARDEN_SNIFF_RIGHT_ARM_ROT)];
const WARDEN_SNIFF_LEFT_ARM_CHANNELS: [AnimationChannel; 1] = [rot(&WARDEN_SNIFF_LEFT_ARM_ROT)];
const WARDEN_SNIFF_BONES: [BoneAnimation; 4] = [
    BoneAnimation {
        bone: "body",
        channels: &WARDEN_SNIFF_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &WARDEN_SNIFF_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_arm",
        channels: &WARDEN_SNIFF_RIGHT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_arm",
        channels: &WARDEN_SNIFF_LEFT_ARM_CHANNELS,
    },
];
/// Vanilla `WardenAnimation.WARDEN_SNIFF`: the 4.16s investigative sniff (NOT looping),
/// `sniffAnimation.apply(sniffAnimationState, ageInTicks)`. Started when the synced `DATA_POSE`
/// changes to `Pose.SNIFFING`. The renderer applies it only while the projected
/// `warden_sniff_seconds >= 0`, clamping past the length to the resting final frame.
pub(in crate::entity_models) const WARDEN_SNIFF: AnimationDefinition = AnimationDefinition {
    length_seconds: 4.16,
    looping: false,
    bones: &WARDEN_SNIFF_BONES,
};

// ----- `WardenAnimation.WARDEN_DIG` (length 5.0s, NOT looping). The despawn burrow: the warden
// rocks forward, claws at the ground, and sinks out of sight (every bone ROTATION + POSITION). Most
// keyframes are CATMULLROM; the terminal frame of several channels is LINEAR (a straight slide into
// the ground). `posVec` negates the y axis and `degreeVec` converts to radians. -----

const WARDEN_DIG_BODY_ROT: [Keyframe; 19] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, degree_vec(4.13441, 0.94736, 1.2694), CATMULLROM),
    keyframe(0.5, degree_vec(50.0, 0.0, 0.0), CATMULLROM),
    keyframe(
        0.7083,
        degree_vec(54.45407, -13.53935, -18.14183),
        CATMULLROM,
    ),
    keyframe(1.0417, degree_vec(59.46442, -10.8885, 35.7954), CATMULLROM),
    keyframe(1.3333, degree_vec(82.28261, 0.0, 0.0), CATMULLROM),
    keyframe(1.625, degree_vec(53.23606, 10.04715, -29.72932), CATMULLROM),
    keyframe(2.2083, degree_vec(-17.71739, 0.0, 0.0), CATMULLROM),
    keyframe(2.5417, degree_vec(112.28261, 0.0, 0.0), CATMULLROM),
    keyframe(
        2.6667,
        degree_vec(116.06889, 5.11581, -24.50117),
        CATMULLROM,
    ),
    keyframe(
        2.8333,
        degree_vec(121.56244, -4.17248, 19.57737),
        CATMULLROM,
    ),
    keyframe(3.0417, degree_vec(138.5689, 5.11581, -24.50117), CATMULLROM),
    keyframe(3.25, degree_vec(144.06244, -4.17248, 19.57737), CATMULLROM),
    keyframe(3.375, degree_vec(147.28261, 0.0, 0.0), CATMULLROM),
    keyframe(3.625, degree_vec(147.28261, 0.0, 0.0), CATMULLROM),
    keyframe(3.875, degree_vec(134.36221, 8.81113, -8.90172), CATMULLROM),
    keyframe(4.0417, degree_vec(132.05966, -8.35927, 9.70506), CATMULLROM),
    keyframe(4.25, degree_vec(134.36221, 8.81113, -8.90172), CATMULLROM),
    keyframe(4.5, degree_vec(147.5, 0.0, 0.0), LINEAR),
];
const WARDEN_DIG_BODY_POS: [Keyframe; 11] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.5, pos_vec(0.0, -16.48454, -6.5784), CATMULLROM),
    keyframe(0.7083, pos_vec(0.0, -16.48454, -6.5784), CATMULLROM),
    keyframe(1.0417, pos_vec(0.0, -16.97, -7.11), CATMULLROM),
    keyframe(1.625, pos_vec(0.0, -13.97, -7.11), CATMULLROM),
    keyframe(2.2083, pos_vec(0.0, -11.48454, -0.5784), CATMULLROM),
    keyframe(2.5417, pos_vec(0.0, -16.48454, -6.5784), CATMULLROM),
    keyframe(2.6667, pos_vec(0.0, -20.27, -5.42), CATMULLROM),
    keyframe(3.375, pos_vec(0.0, -21.48454, -5.5784), CATMULLROM),
    keyframe(4.0417, pos_vec(0.0, -22.48454, -5.5784), CATMULLROM),
    keyframe(4.5, pos_vec(0.0, -40.0, -8.0), LINEAR),
];
const WARDEN_DIG_HEAD_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.6667, degree_vec(12.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.2083, degree_vec(12.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.75, degree_vec(45.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.375, degree_vec(-22.5, 0.0, 0.0), CATMULLROM),
    keyframe(2.5417, degree_vec(67.5, 0.0, 0.0), CATMULLROM),
    keyframe(4.375, degree_vec(67.5, 0.0, 0.0), CATMULLROM),
];
const WARDEN_DIG_HEAD_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(4.375, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const WARDEN_DIG_RIGHT_ARM_ROT: [Keyframe; 10] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.5, degree_vec(-101.8036, -21.29587, 30.61478), CATMULLROM),
    keyframe(
        0.7083,
        degree_vec(-101.8036, -21.29587, 30.61478),
        CATMULLROM,
    ),
    keyframe(1.0, degree_vec(48.7585, -17.61941, 9.9865), CATMULLROM),
    keyframe(1.1667, degree_vec(48.7585, -17.61941, 9.9865), CATMULLROM),
    keyframe(
        1.4583,
        degree_vec(-101.8036, -21.29587, 30.61478),
        CATMULLROM,
    ),
    keyframe(1.75, degree_vec(-89.04994, -4.19657, -1.47845), CATMULLROM),
    keyframe(2.2083, degree_vec(-158.30728, 3.7152, -1.52352), CATMULLROM),
    keyframe(
        2.5417,
        degree_vec(-89.04994, -4.19657, -1.47845),
        CATMULLROM,
    ),
    keyframe(4.375, degree_vec(-120.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_DIG_RIGHT_ARM_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.7083, pos_vec(2.22, 0.0, 0.86), CATMULLROM),
    keyframe(1.0, pos_vec(3.12, 0.0, 4.29), CATMULLROM),
    keyframe(2.2083, pos_vec(1.0, 0.0, 4.0), CATMULLROM),
    keyframe(4.375, pos_vec(0.0, 0.0, 4.0), CATMULLROM),
];
const WARDEN_DIG_LEFT_ARM_ROT: [Keyframe; 12] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.2917, degree_vec(-63.89288, -0.52011, 2.09491), CATMULLROM),
    keyframe(0.5, degree_vec(-63.89288, -0.52011, 2.09491), CATMULLROM),
    keyframe(0.7083, degree_vec(-62.87857, 15.15061, 9.97445), CATMULLROM),
    keyframe(0.9167, degree_vec(-86.93642, 17.45026, 4.05284), CATMULLROM),
    keyframe(1.1667, degree_vec(-86.93642, 17.45026, 4.05284), CATMULLROM),
    keyframe(1.4583, degree_vec(-86.93642, 17.45026, 4.05284), CATMULLROM),
    keyframe(1.6667, degree_vec(63.0984, 8.83573, -8.71284), CATMULLROM),
    keyframe(1.8333, degree_vec(35.5984, 8.83573, -8.71284), CATMULLROM),
    keyframe(2.2083, degree_vec(-153.27473, -0.02953, 3.5235), CATMULLROM),
    keyframe(2.5417, degree_vec(-87.07754, -0.02625, 3.132), CATMULLROM),
    keyframe(4.375, degree_vec(-120.0, 0.0, 0.0), LINEAR),
];
const WARDEN_DIG_LEFT_ARM_POS: [Keyframe; 8] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.5, pos_vec(-0.28, 5.0, 10.0), CATMULLROM),
    keyframe(0.7083, pos_vec(-1.51, 4.35, 4.33), CATMULLROM),
    keyframe(0.9167, pos_vec(-0.6, 3.61, 4.63), CATMULLROM),
    keyframe(1.1667, pos_vec(-0.6, 3.61, 0.63), CATMULLROM),
    keyframe(1.6667, pos_vec(-2.85, -0.1, 3.33), CATMULLROM),
    keyframe(2.2083, pos_vec(-1.0, 0.0, 4.0), CATMULLROM),
    keyframe(4.375, pos_vec(0.0, 0.0, 4.0), LINEAR),
];
const WARDEN_DIG_RIGHT_LEG_ROT: [Keyframe; 9] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.5, degree_vec(113.27, 0.0, 0.0), CATMULLROM),
    keyframe(0.7083, degree_vec(113.27, 0.0, 0.0), CATMULLROM),
    keyframe(3.3333, degree_vec(113.27, 0.0, 0.0), CATMULLROM),
    keyframe(3.5833, degree_vec(182.5, 0.0, 0.0), CATMULLROM),
    keyframe(3.8333, degree_vec(120.0, 0.0, 0.0), CATMULLROM),
    keyframe(4.0833, degree_vec(182.5, 0.0, 0.0), CATMULLROM),
    keyframe(4.2917, degree_vec(120.0, 0.0, 0.0), CATMULLROM),
    keyframe(4.5, degree_vec(90.0, 0.0, 0.0), LINEAR),
];
const WARDEN_DIG_RIGHT_LEG_POS: [Keyframe; 8] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.5, pos_vec(0.0, -13.98, -2.37), CATMULLROM),
    keyframe(0.7083, pos_vec(0.0, -13.98, -2.37), CATMULLROM),
    keyframe(3.3333, pos_vec(0.0, -13.98, -2.37), CATMULLROM),
    keyframe(3.5833, pos_vec(0.0, -7.0, -3.0), CATMULLROM),
    keyframe(3.8333, pos_vec(0.0, -9.0, -3.0), CATMULLROM),
    keyframe(4.0833, pos_vec(0.0, -16.71, -3.69), CATMULLROM),
    keyframe(4.2917, pos_vec(0.0, -28.0, -5.0), LINEAR),
];
const WARDEN_DIG_LEFT_LEG_ROT: [Keyframe; 9] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.5, degree_vec(114.98, 0.0, 0.0), CATMULLROM),
    keyframe(0.7083, degree_vec(114.98, 0.0, 0.0), CATMULLROM),
    keyframe(3.3333, degree_vec(114.98, 0.0, 0.0), CATMULLROM),
    keyframe(3.5833, degree_vec(90.0, 0.0, 0.0), CATMULLROM),
    keyframe(3.8333, degree_vec(172.5, 0.0, 0.0), CATMULLROM),
    keyframe(4.0833, degree_vec(90.0, 0.0, 0.0), CATMULLROM),
    keyframe(4.2917, degree_vec(197.5, 0.0, 0.0), CATMULLROM),
    keyframe(4.5, degree_vec(90.0, 0.0, 0.0), LINEAR),
];
const WARDEN_DIG_LEFT_LEG_POS: [Keyframe; 8] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.5, pos_vec(0.0, -14.01, -2.35), CATMULLROM),
    keyframe(0.7083, pos_vec(0.0, -14.01, -2.35), CATMULLROM),
    keyframe(3.3333, pos_vec(0.0, -14.01, -2.35), CATMULLROM),
    keyframe(3.5833, pos_vec(0.0, -5.0, -4.0), CATMULLROM),
    keyframe(3.8333, pos_vec(0.0, -7.0, -4.0), CATMULLROM),
    keyframe(4.0833, pos_vec(0.0, -15.5, -3.76), CATMULLROM),
    keyframe(4.2917, pos_vec(0.0, -28.0, -5.0), LINEAR),
];
const WARDEN_DIG_BODY_CHANNELS: [AnimationChannel; 2] =
    [rot(&WARDEN_DIG_BODY_ROT), pos(&WARDEN_DIG_BODY_POS)];
const WARDEN_DIG_HEAD_CHANNELS: [AnimationChannel; 2] =
    [rot(&WARDEN_DIG_HEAD_ROT), pos(&WARDEN_DIG_HEAD_POS)];
const WARDEN_DIG_RIGHT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&WARDEN_DIG_RIGHT_ARM_ROT),
    pos(&WARDEN_DIG_RIGHT_ARM_POS),
];
const WARDEN_DIG_LEFT_ARM_CHANNELS: [AnimationChannel; 2] =
    [rot(&WARDEN_DIG_LEFT_ARM_ROT), pos(&WARDEN_DIG_LEFT_ARM_POS)];
const WARDEN_DIG_RIGHT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&WARDEN_DIG_RIGHT_LEG_ROT),
    pos(&WARDEN_DIG_RIGHT_LEG_POS),
];
const WARDEN_DIG_LEFT_LEG_CHANNELS: [AnimationChannel; 2] =
    [rot(&WARDEN_DIG_LEFT_LEG_ROT), pos(&WARDEN_DIG_LEFT_LEG_POS)];
const WARDEN_DIG_BONES: [BoneAnimation; 6] = [
    BoneAnimation {
        bone: "body",
        channels: &WARDEN_DIG_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &WARDEN_DIG_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_arm",
        channels: &WARDEN_DIG_RIGHT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_arm",
        channels: &WARDEN_DIG_LEFT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "right_leg",
        channels: &WARDEN_DIG_RIGHT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_leg",
        channels: &WARDEN_DIG_LEFT_LEG_CHANNELS,
    },
];
/// Vanilla `WardenAnimation.WARDEN_DIG`: the 5.0s despawn burrow (NOT looping),
/// `diggingAnimation.apply(diggingAnimationState, ageInTicks)`. Started when the synced `DATA_POSE`
/// changes to `Pose.DIGGING`. The renderer applies it only while the projected `warden_dig_seconds
/// >= 0`, clamping past the length to the resting final frame (the warden held underground).
pub(in crate::entity_models) const WARDEN_DIG: AnimationDefinition = AnimationDefinition {
    length_seconds: 5.0,
    looping: false,
    bones: &WARDEN_DIG_BONES,
};

// ----- `WardenAnimation.WARDEN_EMERGE` (length 6.68s, NOT looping). The spawn rise: the warden
// surfaces from 63 units underground (the `body`/`right_leg`/`left_leg` POSITION channels open at
// `posVec(0, -63, 0)`), heaves its torso up, plants its arms, and stands (every bone ROTATION +
// POSITION). All keyframes are CATMULLROM. -----

const WARDEN_EMERGE_BODY_ROT: [Keyframe; 16] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.52, degree_vec(0.0, 0.0, -22.5), CATMULLROM),
    keyframe(1.2, degree_vec(0.0, 0.0, -7.5), CATMULLROM),
    keyframe(1.68, degree_vec(0.0, 0.0, 10.0), CATMULLROM),
    keyframe(1.8, degree_vec(0.0, 0.0, 10.0), CATMULLROM),
    keyframe(2.28, degree_vec(0.0, 0.0, 10.0), CATMULLROM),
    keyframe(2.88, degree_vec(0.0, 0.0, 10.0), CATMULLROM),
    keyframe(3.76, degree_vec(25.0, 0.0, -7.5), CATMULLROM),
    keyframe(3.92, degree_vec(35.0, 0.0, -7.5), CATMULLROM),
    keyframe(4.08, degree_vec(25.0, 0.0, -7.5), CATMULLROM),
    keyframe(4.44, degree_vec(47.5, 0.0, 0.0), CATMULLROM),
    keyframe(4.56, degree_vec(47.5, 0.0, 0.0), CATMULLROM),
    keyframe(4.68, degree_vec(47.5, 0.0, 0.0), CATMULLROM),
    keyframe(5.0, degree_vec(70.0, 0.0, 2.5), CATMULLROM),
    keyframe(5.8, degree_vec(70.0, 0.0, 2.5), CATMULLROM),
    keyframe(6.64, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_EMERGE_BODY_POS: [Keyframe; 17] = [
    keyframe(0.0, pos_vec(0.0, -63.0, 0.0), CATMULLROM),
    keyframe(0.52, pos_vec(0.0, -56.0, 0.0), CATMULLROM),
    keyframe(1.2, pos_vec(0.0, -32.0, 0.0), CATMULLROM),
    keyframe(1.68, pos_vec(0.0, -32.0, 0.0), CATMULLROM),
    keyframe(1.8, pos_vec(0.0, -32.0, 0.0), CATMULLROM),
    keyframe(2.28, pos_vec(0.0, -32.0, 0.0), CATMULLROM),
    keyframe(2.88, pos_vec(0.0, -32.0, 0.0), CATMULLROM),
    keyframe(3.16, pos_vec(0.0, -27.0, 0.0), CATMULLROM),
    keyframe(3.76, pos_vec(0.0, -14.0, 0.0), CATMULLROM),
    keyframe(3.92, pos_vec(0.0, -11.0, 0.0), CATMULLROM),
    keyframe(4.08, pos_vec(0.0, -14.0, 0.0), CATMULLROM),
    keyframe(4.44, pos_vec(0.0, -6.0, -3.0), CATMULLROM),
    keyframe(4.56, pos_vec(0.0, -4.0, -3.0), CATMULLROM),
    keyframe(4.68, pos_vec(0.0, -6.0, -3.0), CATMULLROM),
    keyframe(5.0, pos_vec(0.0, -3.0, -4.0), CATMULLROM),
    keyframe(5.8, pos_vec(0.0, -3.0, -4.0), CATMULLROM),
    keyframe(6.64, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_EMERGE_HEAD_ROT: [Keyframe; 18] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.52, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.92, degree_vec(0.74, 0.0, -40.38), CATMULLROM),
    keyframe(1.16, degree_vec(-67.5, 0.0, -2.5), CATMULLROM),
    keyframe(1.24, degree_vec(-67.5, 0.0, -2.5), CATMULLROM),
    keyframe(1.32, degree_vec(-47.5, 0.0, -2.5), CATMULLROM),
    keyframe(1.4, degree_vec(-67.5, 0.0, -2.5), CATMULLROM),
    keyframe(1.68, degree_vec(-67.5, 0.0, 15.0), CATMULLROM),
    keyframe(1.76, degree_vec(-67.5, 0.0, -5.0), CATMULLROM),
    keyframe(1.84, degree_vec(-52.5, 0.0, -5.0), CATMULLROM),
    keyframe(1.92, degree_vec(-67.5, 0.0, -5.0), CATMULLROM),
    keyframe(2.64, degree_vec(-17.5, 0.0, -10.0), CATMULLROM),
    keyframe(3.76, degree_vec(70.0, 0.0, 12.5), CATMULLROM),
    keyframe(4.04, degree_vec(70.0, 0.0, 12.5), CATMULLROM),
    keyframe(4.12, degree_vec(80.0, 0.0, 12.5), CATMULLROM),
    keyframe(4.24, degree_vec(70.0, 0.0, 12.5), CATMULLROM),
    keyframe(5.0, degree_vec(77.5, 0.0, -2.5), CATMULLROM),
    keyframe(6.64, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_EMERGE_HEAD_POS: [Keyframe; 17] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.52, pos_vec(-8.0, -11.0, 0.0), CATMULLROM),
    keyframe(0.92, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.24, pos_vec(0.0, 0.47, -0.95), CATMULLROM),
    keyframe(1.32, pos_vec(0.0, 0.47, -0.95), CATMULLROM),
    keyframe(1.4, pos_vec(0.0, 0.47, -0.95), CATMULLROM),
    keyframe(1.68, pos_vec(0.0, 1.0, -2.0), CATMULLROM),
    keyframe(1.76, pos_vec(0.0, 1.0, -2.0), CATMULLROM),
    keyframe(1.84, pos_vec(0.0, 1.0, -2.0), CATMULLROM),
    keyframe(1.92, pos_vec(0.0, 1.0, -2.0), CATMULLROM),
    keyframe(2.64, pos_vec(0.0, -2.0, -2.0), CATMULLROM),
    keyframe(3.76, pos_vec(0.0, -4.0, 1.0), CATMULLROM),
    keyframe(4.04, pos_vec(0.0, -1.0, 1.0), CATMULLROM),
    keyframe(4.12, pos_vec(0.0, -1.0, 1.0), CATMULLROM),
    keyframe(4.24, pos_vec(0.0, -1.0, 1.0), CATMULLROM),
    keyframe(5.0, pos_vec(0.0, -1.0, 1.0), CATMULLROM),
    keyframe(6.64, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_EMERGE_RIGHT_ARM_ROT: [Keyframe; 19] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.52, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.2, degree_vec(-152.5, 2.5, 7.5), CATMULLROM),
    keyframe(1.68, degree_vec(-180.0, 12.5, -10.0), CATMULLROM),
    keyframe(1.8, degree_vec(-90.0, 12.5, -10.0), CATMULLROM),
    keyframe(2.28, degree_vec(-90.0, 12.5, -10.0), CATMULLROM),
    keyframe(2.88, degree_vec(-90.0, 12.5, -10.0), CATMULLROM),
    keyframe(3.08, degree_vec(-95.0, 12.5, -10.0), CATMULLROM),
    keyframe(3.24, degree_vec(-83.93, 3.93, 5.71), CATMULLROM),
    keyframe(3.36, degree_vec(-80.0, 7.5, 17.5), CATMULLROM),
    keyframe(3.76, degree_vec(-67.5, 2.5, 0.0), CATMULLROM),
    keyframe(4.08, degree_vec(-67.5, 2.5, 0.0), CATMULLROM),
    keyframe(4.44, degree_vec(-55.0, 2.5, 0.0), CATMULLROM),
    keyframe(4.56, degree_vec(-60.0, 2.5, 0.0), CATMULLROM),
    keyframe(4.68, degree_vec(-55.0, 2.5, 0.0), CATMULLROM),
    keyframe(5.0, degree_vec(-67.5, 0.0, 0.0), CATMULLROM),
    keyframe(5.56, degree_vec(-50.45, 0.0, 2.69), CATMULLROM),
    keyframe(6.08, degree_vec(-62.72, 0.0, 4.3), CATMULLROM),
    keyframe(6.64, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_EMERGE_RIGHT_ARM_POS: [Keyframe; 17] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.52, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.2, pos_vec(0.0, -21.0, 9.0), CATMULLROM),
    keyframe(1.68, pos_vec(2.0, -2.0, 0.0), CATMULLROM),
    keyframe(1.8, pos_vec(2.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.28, pos_vec(2.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.88, pos_vec(2.0, 0.0, 0.0), CATMULLROM),
    keyframe(3.08, pos_vec(2.0, -2.0, 0.0), CATMULLROM),
    keyframe(3.24, pos_vec(2.0, 2.71, 3.86), CATMULLROM),
    keyframe(3.36, pos_vec(2.0, 1.0, 5.0), CATMULLROM),
    keyframe(3.76, pos_vec(2.0, 3.0, 3.0), CATMULLROM),
    keyframe(4.08, pos_vec(2.0, 3.0, 3.0), CATMULLROM),
    keyframe(4.44, pos_vec(2.67, 4.0, 0.0), CATMULLROM),
    keyframe(4.56, pos_vec(2.67, 0.0, 0.0), CATMULLROM),
    keyframe(4.68, pos_vec(2.67, 4.0, 0.0), CATMULLROM),
    keyframe(5.0, pos_vec(0.67, 3.0, 4.0), CATMULLROM),
    keyframe(6.64, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_EMERGE_LEFT_ARM_ROT: [Keyframe; 22] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.12, degree_vec(-167.5, -17.5, -7.5), CATMULLROM),
    keyframe(0.6, degree_vec(-167.5, -17.5, -7.5), CATMULLROM),
    keyframe(0.88, degree_vec(-175.0, -17.5, 15.0), CATMULLROM),
    keyframe(1.16, degree_vec(-190.0, -17.5, 5.0), CATMULLROM),
    keyframe(1.28, degree_vec(-90.0, -5.0, 5.0), CATMULLROM),
    keyframe(1.68, degree_vec(-90.0, -17.5, -12.5), CATMULLROM),
    keyframe(1.8, degree_vec(-90.0, -17.5, -12.5), CATMULLROM),
    keyframe(2.28, degree_vec(-90.0, -17.5, -12.5), CATMULLROM),
    keyframe(2.88, degree_vec(-90.0, -17.5, -12.5), CATMULLROM),
    keyframe(3.04, degree_vec(-81.29, -10.64, -14.21), CATMULLROM),
    keyframe(3.16, degree_vec(-83.5, -5.5, -15.5), CATMULLROM),
    keyframe(3.76, degree_vec(-62.5, -7.5, 5.0), CATMULLROM),
    keyframe(3.92, degree_vec(-58.75, -3.75, 5.0), CATMULLROM),
    keyframe(4.08, degree_vec(-55.0, 0.0, 0.0), CATMULLROM),
    keyframe(4.44, degree_vec(-52.5, 0.0, 5.0), CATMULLROM),
    keyframe(4.56, degree_vec(-50.0, 0.0, 5.0), CATMULLROM),
    keyframe(4.68, degree_vec(-52.5, 0.0, 5.0), CATMULLROM),
    keyframe(5.0, degree_vec(-72.5, -2.5, 5.0), CATMULLROM),
    keyframe(5.56, degree_vec(-57.5, -4.54, 2.99), CATMULLROM),
    keyframe(6.08, degree_vec(-70.99, -5.77, 1.78), CATMULLROM),
    keyframe(6.64, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_EMERGE_LEFT_ARM_POS: [Keyframe; 19] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.12, pos_vec(0.0, -8.0, 0.0), CATMULLROM),
    keyframe(0.6, pos_vec(0.0, -8.0, 0.0), CATMULLROM),
    keyframe(0.88, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.2, pos_vec(-2.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.68, pos_vec(-4.0, 3.0, 0.0), CATMULLROM),
    keyframe(1.8, pos_vec(-4.0, 3.0, 0.0), CATMULLROM),
    keyframe(2.28, pos_vec(-4.0, 3.0, 0.0), CATMULLROM),
    keyframe(2.88, pos_vec(-4.0, 3.0, 0.0), CATMULLROM),
    keyframe(3.04, pos_vec(-3.23, 5.7, 4.97), CATMULLROM),
    keyframe(3.16, pos_vec(-1.49, 2.22, 5.25), CATMULLROM),
    keyframe(3.76, pos_vec(-1.14, 1.71, 1.86), CATMULLROM),
    keyframe(3.92, pos_vec(-1.14, 1.21, 3.86), CATMULLROM),
    keyframe(4.08, pos_vec(-1.14, 2.71, 4.86), CATMULLROM),
    keyframe(4.44, pos_vec(-1.0, 1.0, 3.0), CATMULLROM),
    keyframe(4.56, pos_vec(0.0, 1.0, 1.0), CATMULLROM),
    keyframe(4.68, pos_vec(0.0, 1.0, 3.0), CATMULLROM),
    keyframe(5.0, pos_vec(-2.0, 0.0, 4.0), CATMULLROM),
    keyframe(6.64, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_EMERGE_RIGHT_LEG_ROT: [Keyframe; 11] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.52, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.28, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.88, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(3.36, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(4.32, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(4.48, degree_vec(55.0, 0.0, 0.0), CATMULLROM),
    keyframe(4.6, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(5.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(5.8, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(6.64, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_EMERGE_RIGHT_LEG_POS: [Keyframe; 17] = [
    keyframe(0.0, pos_vec(0.0, -63.0, 0.0), CATMULLROM),
    keyframe(0.52, pos_vec(0.0, -56.0, 0.0), CATMULLROM),
    keyframe(1.2, pos_vec(0.0, -32.0, 0.0), CATMULLROM),
    keyframe(1.68, pos_vec(0.0, -32.0, 0.0), CATMULLROM),
    keyframe(1.8, pos_vec(0.0, -32.0, 0.0), CATMULLROM),
    keyframe(2.28, pos_vec(0.0, -32.0, 0.0), CATMULLROM),
    keyframe(2.88, pos_vec(0.0, -32.0, 0.0), CATMULLROM),
    keyframe(3.36, pos_vec(0.0, -22.0, 0.0), CATMULLROM),
    keyframe(3.76, pos_vec(0.0, -12.28, 2.48), CATMULLROM),
    keyframe(3.92, pos_vec(0.0, -9.28, 2.48), CATMULLROM),
    keyframe(4.08, pos_vec(0.0, -12.28, 2.48), CATMULLROM),
    keyframe(4.32, pos_vec(0.0, -4.14, 4.14), CATMULLROM),
    keyframe(4.48, pos_vec(0.0, -0.57, -8.43), CATMULLROM),
    keyframe(4.6, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(5.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(5.8, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(6.64, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_EMERGE_LEFT_LEG_ROT: [Keyframe; 12] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.52, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.28, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(2.88, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(3.36, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(3.84, degree_vec(20.0, 0.0, -17.5), CATMULLROM),
    keyframe(4.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(4.68, degree_vec(20.0, 0.0, 0.0), CATMULLROM),
    keyframe(4.84, degree_vec(10.0, 0.0, 0.0), CATMULLROM),
    keyframe(5.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(5.8, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(6.64, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_EMERGE_LEFT_LEG_POS: [Keyframe; 15] = [
    keyframe(0.0, pos_vec(0.0, -63.0, 0.0), CATMULLROM),
    keyframe(0.52, pos_vec(0.0, -56.0, 0.0), CATMULLROM),
    keyframe(1.2, pos_vec(0.0, -32.0, 0.0), CATMULLROM),
    keyframe(1.68, pos_vec(0.0, -32.0, 0.0), CATMULLROM),
    keyframe(1.8, pos_vec(0.0, -32.0, 0.0), CATMULLROM),
    keyframe(2.28, pos_vec(0.0, -32.0, 0.0), CATMULLROM),
    keyframe(2.88, pos_vec(0.0, -32.0, 0.0), CATMULLROM),
    keyframe(3.36, pos_vec(0.0, -22.0, 0.0), CATMULLROM),
    keyframe(3.84, pos_vec(-4.0, 2.0, -7.0), CATMULLROM),
    keyframe(4.0, pos_vec(-4.0, 0.0, -5.0), CATMULLROM),
    keyframe(4.68, pos_vec(-4.0, 0.0, -9.0), CATMULLROM),
    keyframe(4.84, pos_vec(-2.0, 2.0, -3.5), CATMULLROM),
    keyframe(5.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(5.8, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(6.64, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const WARDEN_EMERGE_BODY_CHANNELS: [AnimationChannel; 2] =
    [rot(&WARDEN_EMERGE_BODY_ROT), pos(&WARDEN_EMERGE_BODY_POS)];
const WARDEN_EMERGE_HEAD_CHANNELS: [AnimationChannel; 2] =
    [rot(&WARDEN_EMERGE_HEAD_ROT), pos(&WARDEN_EMERGE_HEAD_POS)];
const WARDEN_EMERGE_RIGHT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&WARDEN_EMERGE_RIGHT_ARM_ROT),
    pos(&WARDEN_EMERGE_RIGHT_ARM_POS),
];
const WARDEN_EMERGE_LEFT_ARM_CHANNELS: [AnimationChannel; 2] = [
    rot(&WARDEN_EMERGE_LEFT_ARM_ROT),
    pos(&WARDEN_EMERGE_LEFT_ARM_POS),
];
const WARDEN_EMERGE_RIGHT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&WARDEN_EMERGE_RIGHT_LEG_ROT),
    pos(&WARDEN_EMERGE_RIGHT_LEG_POS),
];
const WARDEN_EMERGE_LEFT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&WARDEN_EMERGE_LEFT_LEG_ROT),
    pos(&WARDEN_EMERGE_LEFT_LEG_POS),
];
const WARDEN_EMERGE_BONES: [BoneAnimation; 6] = [
    BoneAnimation {
        bone: "body",
        channels: &WARDEN_EMERGE_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &WARDEN_EMERGE_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_arm",
        channels: &WARDEN_EMERGE_RIGHT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "left_arm",
        channels: &WARDEN_EMERGE_LEFT_ARM_CHANNELS,
    },
    BoneAnimation {
        bone: "right_leg",
        channels: &WARDEN_EMERGE_RIGHT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_leg",
        channels: &WARDEN_EMERGE_LEFT_LEG_CHANNELS,
    },
];
/// Vanilla `WardenAnimation.WARDEN_EMERGE`: the 6.68s spawn rise (NOT looping),
/// `emergeAnimation.apply(emergeAnimationState, ageInTicks)`. Started when the synced `DATA_POSE`
/// changes to `Pose.EMERGING`. The renderer applies it only while the projected
/// `warden_emerge_seconds >= 0`, clamping past the length to the resting final frame (fully risen).
pub(in crate::entity_models) const WARDEN_EMERGE: AnimationDefinition = AnimationDefinition {
    length_seconds: 6.68,
    looping: false,
    bones: &WARDEN_EMERGE_BONES,
};

/// Mutable warden model, mirroring vanilla `WardenModel`. The cubeless `bone` root (parenting the
/// body and two legs; `body` parents the ribcages, head, and two arms; `head` parents the two
/// tendrils) hangs off a synthetic root, built from the baked colored geometry as a named-children
/// tree. Colored-only: `setup_anim` reproduces the four non-keyframe motions — the head look, the
/// idle wobble, the walk swing, and the tendril sway — then layers the six triggered combat keyframe
/// one-shots additively in the vanilla order (attack, sonic_boom, dig, emerge, roar, sniff).
pub(in crate::entity_models) struct WardenModel {
    root: ModelPart,
}

impl WardenModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: warden_root(),
        }
    }
}

impl EntityModel for WardenModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `WardenModel.setupAnim` composes four non-keyframe motions: the head look
        // ([`warden_head_pose`]), the always-on idle wobble ([`warden_idle_body_pose`] on the body,
        // folded into the head pose), the walk swing ([`warden_walk_pose`], ADDED via
        // [`warden_add_x_z_rot`] onto the look/idle composition — addition is commutative, preserving
        // the vanilla order), and the tendril sway ([`warden_tendril_x_rot`]). At `walkSpeed = 0` the
        // walk adds nothing; the idle wobble and tendril sway advance every frame off `ageInTicks`.
        let head_yaw = instance.render_state.head_yaw;
        let head_pitch = instance.render_state.head_pitch;
        let age = instance.render_state.age_in_ticks;
        let walk = warden_walk_pose(
            instance.render_state.walk_animation_pos,
            instance.render_state.walk_animation_speed,
        );
        let tendril_x = warden_tendril_x_rot(instance.render_state.tendril_animation, age);

        // Vanilla `WardenModel.setupAnim` then applies the six triggered combat keyframe one-shots
        // ADDITIVELY (`KeyframeAnimation.apply` folds `offsetPos`/`offsetRotation` onto the
        // already-posed bones) in the order attack → sonic_boom → dig → emerge → roar → sniff. Each
        // is applied only while its projected elapsed-seconds value is `>= 0`; a non-looping def
        // clamps past its length to the resting final frame (vanilla's "hold the last frame"). The
        // active definitions, paired with their (clamped) sample seconds.
        let combat: [(&AnimationDefinition, f32); 6] = [
            (&WARDEN_ATTACK, instance.render_state.warden_attack_seconds),
            (
                &WARDEN_SONIC_BOOM,
                instance.render_state.warden_sonic_boom_seconds,
            ),
            (&WARDEN_DIG, instance.render_state.warden_dig_seconds),
            (&WARDEN_EMERGE, instance.render_state.warden_emerge_seconds),
            (&WARDEN_ROAR, instance.render_state.warden_roar_seconds),
            (&WARDEN_SNIFF, instance.render_state.warden_sniff_seconds),
        ];
        // Adds every active combat one-shot's position/rotation offsets onto a part already posed by
        // the look/idle/walk (and, for the head, the tendril pulse).
        let apply_combat = |part: &mut ModelPart, bone: &str| {
            for (definition, seconds) in combat {
                if seconds < 0.0 {
                    continue;
                }
                let sample = keyframe_elapsed_seconds(definition, seconds);
                let (position, rotation) = sample_bone_offsets(definition, bone, sample, 1.0);
                part.pose = keyframe_animated_pose(part.pose, position, rotation);
            }
        };

        let bone = self.root.child_mut("bone");
        {
            let body = bone.child_mut("body");
            body.pose = warden_add_x_z_rot(
                warden_idle_body_pose(body.pose, age),
                walk.body_x_rot,
                walk.body_z_rot,
            );
            apply_combat(body, "body");

            {
                let head = body.child_mut("head");
                head.pose = warden_add_x_z_rot(
                    warden_head_pose(head.pose, head_yaw, head_pitch, age),
                    walk.head_x_rot,
                    walk.head_z_rot,
                );
                apply_combat(head, "head");

                // The two tendrils sway their `xRot` off the pulse (left `+`, right `-`).
                let right = head.child_mut("right_tendril");
                right.pose = warden_add_x_z_rot(right.pose, -tendril_x, 0.0);
                let left = head.child_mut("left_tendril");
                left.pose = warden_add_x_z_rot(left.pose, tendril_x, 0.0);
            }

            // The two ribcages hold at bind for the walk/idle, but the sonic boom fans them open.
            apply_combat(body.child_mut("right_ribcage"), "right_ribcage");
            apply_combat(body.child_mut("left_ribcage"), "left_ribcage");

            // The two arms swing their `xRot` with the walk, then the combat one-shots add on top.
            let right_arm = body.child_mut("right_arm");
            right_arm.pose = warden_add_x_z_rot(right_arm.pose, walk.right_arm_x_rot, 0.0);
            apply_combat(right_arm, "right_arm");
            let left_arm = body.child_mut("left_arm");
            left_arm.pose = warden_add_x_z_rot(left_arm.pose, walk.left_arm_x_rot, 0.0);
            apply_combat(left_arm, "left_arm");
        }

        // The two legs swing their `xRot` with the walk, then the dig/emerge spawn/despawn one-shots
        // add on top (only those two touch the legs — attack/sonic_boom/roar/sniff carry no leg
        // bone, so `apply_combat` adds zero for them).
        let right_leg = bone.child_mut("right_leg");
        right_leg.pose = warden_add_x_z_rot(right_leg.pose, walk.right_leg_x_rot, 0.0);
        apply_combat(right_leg, "right_leg");
        let left_leg = bone.child_mut("left_leg");
        left_leg.pose = warden_add_x_z_rot(left_leg.pose, walk.left_leg_x_rot, 0.0);
        apply_combat(left_leg, "left_leg");
    }
}
