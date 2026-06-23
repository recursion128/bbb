use super::{
    bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc, WARDEN_BODY,
    WARDEN_TENDRIL,
};

// Vanilla 26.1 `WardenModel.createBodyLayer` (atlas 128×128). The mesh root holds one `bone` part
// at `offset(0, 24, 0)` parenting the body and the two legs; `body` parents the two ribcage
// planes, the head (which parents the two tendril planes), and the two arms. Every
// `WardenModel.setupAnim` animation — the head look, walk, the procedural idle wobble
// (`animateIdlePose`), the tendril sway (`animateTendrils`), and the attack / sonic-boom /
// digging / emerge / roar / sniff keyframe animations — is deferred, so the model renders at this
// rest pose. The four emissive overlay layers (tendrils, heart, bioluminescent, pulsating spots)
// and the texture-backed path are deferred.

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
/// from the look angles, and the two tendrils nested under the head inherit the turn.
pub(in crate::entity_models) const WARDEN_HEAD_PART_PATH: &[usize] = &[0, 0, 2];
