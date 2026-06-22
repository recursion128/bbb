use super::{
    bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc, LEASH_KNOT_COLOR,
};

// Vanilla 26.1 `LeashKnotModel.createBodyLayer` (atlas 32×32). The mesh root holds a single `knot`
// part at ZERO with one 6×8×6 box. `LeashKnotModel` has no `setupAnim`, so the model is fully
// static — nothing is deferred on the geometry side. `LeashKnotRenderer` is a plain `EntityRenderer`
// that applies only the model flip (`scale(-1, -1, 1)`), captured by `leash_knot_model_root_transform`.
// Only the texture-backed path is deferred, so the colored debug path renders the knot with one tint.

const LEASH_KNOT_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.0, -8.0, -3.0], [6.0, 8.0, 6.0], LEASH_KNOT_COLOR)];

pub(in crate::entity_models) const LEASH_KNOT_PARTS: [ModelPartDesc; 1] =
    [part([0.0, 0.0, 0.0], &LEASH_KNOT_CUBES, &[])];
