use super::{
    bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc, SHULKER_HEAD,
    SHULKER_SHELL,
};

// Vanilla 26.1 `ShulkerModel.createBodyLayer` (atlas 64×64). The mesh root holds three sibling
// parts: the 16×12×16 lid and the 16×8×16 base (both at `offset(0, 24, 0)`), and the 6×6×6 head at
// `offset(0, 12, 0)`. The closed rest pose equals this bind pose — `ShulkerModel.setupAnim` sets the
// lid back to `y = 16 + sin((0.5 + peekAmount) * π) * 8`, which is exactly `24` when `peekAmount = 0`
// — so the peek open/close (`lid.setPos` + `lid.yRot` wobble) and the head look (`head.xRot/yRot`)
// are deferred. The `ShulkerRenderer.setupRotations` attach-face rotation (`attachFace.getOpposite()`,
// the identity for a floor shulker) and the `bodyRot + 180` body-yaw inversion read the entity-side
// `attachFace`/yaw state, which the native scene does not yet project, so the floor rest pose is
// emitted. The sixteen dye-color variants live on the deferred texture-backed path, so the colored
// debug path renders a purple shell tint plus a yellow head tint.

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
