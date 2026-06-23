use super::{
    bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc, SHULKER_HEAD,
    SHULKER_SHELL,
};

// Vanilla 26.1 `ShulkerModel.createBodyLayer` (atlas 64×64). The mesh root holds three sibling
// parts: the 16×12×16 lid and the 16×8×16 base (both at `offset(0, 24, 0)`), and the 6×6×6 head at
// `offset(0, 12, 0)`. The closed rest pose equals this bind pose — `ShulkerModel.setupAnim` sets the
// lid back to `y = 16 + sin((0.5 + peekAmount) * π) * 8`, which is exactly `24` when `peekAmount = 0`.
// The peek open/close is now driven from the projected `Shulker.getClientPeekAmount` (see
// [`emit_shulker_model`](crate::entity_models::colored::runtime)). The head look (`head.xRot/yRot`)
// stays deferred — its non-standard `(yHeadRot − 180 − yBodyRot)` formula needs the entity-side head
// yaw the native scene does not project. The `ShulkerRenderer.setupRotations` attach-face rotation
// (`attachFace.getOpposite()`, the identity for a floor shulker) and the `bodyRot + 180` body-yaw
// inversion read the entity-side `attachFace`/yaw state, which the native scene does not yet project,
// so the floor rest pose is emitted. The sixteen dye-color variants live on the deferred
// texture-backed path, so the colored debug path renders a purple shell tint plus a yellow head tint.

// `lid`: the 16×12×16 upper shell.
const SHULKER_LID_CUBES: [ModelCubeDesc; 1] =
    [cube([-8.0, -16.0, -8.0], [16.0, 12.0, 16.0], SHULKER_SHELL)];

// `base`: the 16×8×16 lower shell.
const SHULKER_BASE_CUBES: [ModelCubeDesc; 1] =
    [cube([-8.0, -8.0, -8.0], [16.0, 8.0, 16.0], SHULKER_SHELL)];

// `head`: the 6×6×6 yellow head.
const SHULKER_HEAD_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.0, 0.0, -3.0], [6.0, 6.0, 6.0], SHULKER_HEAD)];

pub(in crate::entity_models) const SHULKER_PARTS: [ModelPartDesc; 3] = [
    part([0.0, 24.0, 0.0], &SHULKER_LID_CUBES, &[]),
    part([0.0, 24.0, 0.0], &SHULKER_BASE_CUBES, &[]),
    part([0.0, 12.0, 0.0], &SHULKER_HEAD_CUBES, &[]),
];

/// Vanilla `ShulkerModel.setupAnim` lid pose from the client peek amount (and `ageInTicks`
/// for the open-lid bob). With `bs = (0.5 + peek)·π`:
/// `lid.y = 16 + sin(bs)·8` (plus `sin(ageInTicks·0.1)·0.7` once `bs > π`, i.e. the lid is
/// past half-open) and `lid.yRot = (−1 + sin(bs))⁴ · π · 0.125` once `peek > 0.3` (else `0`).
/// Returns `(lid_y, lid_yrot)`. At `peek = 0` this is `(24, 0)` — the closed/bind pose.
pub(in crate::entity_models) fn shulker_lid_pose(peek: f32, age_in_ticks: f32) -> (f32, f32) {
    let bs = (0.5 + peek) * std::f32::consts::PI;
    let extra = if bs > std::f32::consts::PI {
        (age_in_ticks * 0.1).sin() * 0.7
    } else {
        0.0
    };
    let lid_y = 16.0 + bs.sin() * 8.0 + extra;
    let lid_yrot = if peek > 0.3 {
        let q = -1.0 + bs.sin();
        q * q * q * q * std::f32::consts::PI * 0.125
    } else {
        0.0
    };
    (lid_y, lid_yrot)
}
