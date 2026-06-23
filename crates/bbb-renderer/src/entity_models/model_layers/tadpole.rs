use super::{
    bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc, TADPOLE_BODY, TADPOLE_TAIL,
};

// Vanilla 26.1 `TadpoleModel.createBodyLayer` (atlas 16×16). The mesh root holds two sibling parts:
// a 3×2×3 body box at `offset(0, 22, -3)` and a 0×2×7 tail fin plane at `offset(0, 22, 0)`. The
// only `TadpoleModel.setupAnim` motion is the tail yaw sway ([`tadpole_tail_yrot`]), reproduced from
// the projected `age_in_ticks` + `in_water`. Tadpole uses a plain `MobRenderer` with no transform
// overrides.

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

/// The tail fin is [`TADPOLE_PARTS`] index 1; `TadpoleModel.setupAnim` sets its `yRot`.
pub(in crate::entity_models) const TADPOLE_TAIL_PART_INDEX: usize = 1;

/// Vanilla `TadpoleModel.setupAnim`: `tail.yRot = -amplitude * 0.25 * sin(0.3 * ageInTicks)`, with
/// `amplitude = isInWater ? 1.0 : 1.5` (a beached tadpole thrashes harder). The rest pose has
/// `yRot = 0`, so this is set absolutely. Mirrors [`super::cod_tail_fin_yrot`] with the tadpole's
/// own `0.25` / `0.3` constants.
pub(in crate::entity_models) fn tadpole_tail_yrot(age_in_ticks: f32, in_water: bool) -> f32 {
    let amplitude = if in_water { 1.0 } else { 1.5 };
    -amplitude * 0.25 * (0.3 * age_in_ticks).sin()
}
