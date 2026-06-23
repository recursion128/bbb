use super::{
    bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc, PartPose, WARDEN_BODY,
    WARDEN_TENDRIL,
};

// Vanilla 26.1 `WardenModel.createBodyLayer` (atlas 128×128). The mesh root holds one `bone` part
// at `offset(0, 24, 0)` parenting the body and the two legs; `body` parents the two ribcage
// planes, the head (which parents the two tendril planes), and the two arms. Two non-keyframe
// `WardenModel.setupAnim` motions are reproduced ([`warden_head_pose`] / [`warden_idle_body_pose`]):
// the head look (`animateHeadLookTarget`) and the always-on idle wobble (`animateIdlePose`). The
// walk (`animateWalk`), the tendril sway (`animateTendrils`), and the attack / sonic-boom / digging
// / emerge / roar / sniff keyframe animations stay deferred. The four emissive overlay layers
// (tendrils, heart, bioluminescent, pulsating spots) and the texture-backed path are deferred.

// `body`: one 18×21×11 box.
const WARDEN_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-9.0, -13.0, -4.0], [18.0, 21.0, 11.0], WARDEN_BODY)];

// The two ribcage planes (`texOffs(90,11)`, the left mirrored); both are 9×21×0.
const WARDEN_RIGHT_RIBCAGE_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.0, -11.0, -0.1], [9.0, 21.0, 0.0], WARDEN_BODY)];
const WARDEN_LEFT_RIBCAGE_CUBES: [ModelCubeDesc; 1] =
    [cube([-7.0, -11.0, -0.1], [9.0, 21.0, 0.0], WARDEN_BODY)];

// `head`: one 16×16×10 box.
const WARDEN_HEAD_CUBES: [ModelCubeDesc; 1] =
    [cube([-8.0, -16.0, -5.0], [16.0, 16.0, 10.0], WARDEN_BODY)];

// The two tendril planes (16×16×0), the warden's iconic glow antennae.
const WARDEN_RIGHT_TENDRIL_CUBES: [ModelCubeDesc; 1] =
    [cube([-16.0, -13.0, 0.0], [16.0, 16.0, 0.0], WARDEN_TENDRIL)];
const WARDEN_LEFT_TENDRIL_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, -13.0, 0.0], [16.0, 16.0, 0.0], WARDEN_TENDRIL)];

// Both arms share one 8×28×8 box.
const WARDEN_ARM_CUBES: [ModelCubeDesc; 1] =
    [cube([-4.0, 0.0, -4.0], [8.0, 28.0, 8.0], WARDEN_BODY)];

// The legs (6×13×6) differ only in X origin.
const WARDEN_RIGHT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.1, 0.0, -3.0], [6.0, 13.0, 6.0], WARDEN_BODY)];
const WARDEN_LEFT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.9, 0.0, -3.0], [6.0, 13.0, 6.0], WARDEN_BODY)];

// `head` children: the two tendrils.
const WARDEN_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    part([-8.0, -12.0, 0.0], &WARDEN_RIGHT_TENDRIL_CUBES, &[]),
    part([8.0, -12.0, 0.0], &WARDEN_LEFT_TENDRIL_CUBES, &[]),
];

// `body` children: the two ribcages, the head, and the two arms.
const WARDEN_BODY_CHILDREN: [ModelPartDesc; 5] = [
    part([-7.0, -2.0, -4.0], &WARDEN_RIGHT_RIBCAGE_CUBES, &[]),
    part([7.0, -2.0, -4.0], &WARDEN_LEFT_RIBCAGE_CUBES, &[]),
    part([0.0, -13.0, 0.0], &WARDEN_HEAD_CUBES, &WARDEN_HEAD_CHILDREN),
    part([-13.0, -13.0, 1.0], &WARDEN_ARM_CUBES, &[]),
    part([13.0, -13.0, 1.0], &WARDEN_ARM_CUBES, &[]),
];

// `bone` children: the body and the two legs.
const WARDEN_BONE_CHILDREN: [ModelPartDesc; 3] = [
    part([0.0, -21.0, 0.0], &WARDEN_BODY_CUBES, &WARDEN_BODY_CHILDREN),
    part([-5.9, -13.0, 0.0], &WARDEN_RIGHT_LEG_CUBES, &[]),
    part([5.9, -13.0, 0.0], &WARDEN_LEFT_LEG_CUBES, &[]),
];

/// Vanilla `WardenModel.createBodyLayer` rest-pose hierarchy, rooted at the `bone` part
/// (`offset(0, 24, 0)`). Ten cubes.
pub(in crate::entity_models) const WARDEN_PARTS: [ModelPartDesc; 1] =
    [part([0.0, 24.0, 0.0], &[], &WARDEN_BONE_CHILDREN)];

/// Child-index path from [`WARDEN_PARTS`] to the `head`: `bone` (`0`) → `body` (child `0`) → `head`
/// (child `2`, after the two ribcages). `WardenModel.animateHeadLookTarget` sets `head.xRot/yRot`
/// from the look angles, and the two tendrils nested under the head inherit the turn. The idle
/// wobble also rolls the body, so the warden emit hand-walks `bone → body → head` using these
/// indices.
pub(in crate::entity_models) const WARDEN_BODY_BONE_CHILD_INDEX: usize = 0;
pub(in crate::entity_models) const WARDEN_HEAD_BODY_CHILD_INDEX: usize = 2;

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
/// `head.xRot += 0.06·sin(s)`, `head.zRot += 0.06·cos(s)` with `s = ageInTicks·0.1`. (The walk pose
/// would add further to `head.xRot/zRot`, but the walk is deferred.) The base `head.zRot` is the
/// bind `0`, so the idle roll lands on `base.rotation[2]`.
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
