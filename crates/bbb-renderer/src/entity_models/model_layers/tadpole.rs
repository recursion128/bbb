use super::{
    bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc, TADPOLE_BODY, TADPOLE_TAIL,
};

// Vanilla 26.1 `TadpoleModel.createBodyLayer` (atlas 16×16). The mesh root holds two sibling parts:
// a 3×2×3 body box at `offset(0, 22, -3)` and a 0×2×7 tail fin plane at `offset(0, 22, 0)`. The
// only `TadpoleModel.setupAnim` motion is the tail yaw sway (`tail.yRot = -amplitude * 0.25 *
// sin(0.3 * ageInTicks)`, amplitude `1.0` in water / `1.5` on land), which is deferred, so the
// model renders at this rest pose. Tadpole uses a plain `MobRenderer` with no transform overrides.

// `body`: the 3×2×3 box.
const TADPOLE_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.5, -1.0, 0.0], [3.0, 2.0, 3.0], TADPOLE_BODY)];

// `tail`: the 0×2×7 fin plane.
const TADPOLE_TAIL_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, -1.0, 0.0], [0.0, 2.0, 7.0], TADPOLE_TAIL)];

pub(in crate::entity_models) const TADPOLE_PARTS: [ModelPartDesc; 2] = [
    part([0.0, 22.0, -3.0], &TADPOLE_BODY_CUBES, &[]),
    part([0.0, 22.0, 0.0], &TADPOLE_TAIL_CUBES, &[]),
];
