use super::{
    bind_part_rot as rpart, model_cube as cube, ModelCubeDesc, ModelPartDesc, ARROW_HEAD,
    ARROW_SHAFT,
};

// Vanilla 26.1 `ArrowModel.createBodyLayer` (atlas 32×32). The mesh root holds three sibling planes:
// the `back` arrowhead (a 0×5×5 YZ plane at `offset(-11, 0, 0)`, pitched π/4, `withScale(0.8)`), and
// two crossed fletching planes (`cross_1`/`cross_2`, each a 16×4×0 XY plane pitched π/4 and 3π/4).
// The whole mesh is baked through `mesh.transformed(pose -> pose.scaled(0.9))`; that 0.9 lives in
// `arrow_model_root_transform`, while the `back` part's `withScale(0.8)` is baked into its cube
// (a 0×5×5 box → a 0×4×4 box). `ArrowModel.setupAnim` only adds the impact-shake `root.zRot` wobble
// (`-sin(shake·3)·shake`), which is deferred. `ArrowRenderer` orients the arrow along its flight with
// `Ry(yRot - 90)` then `Rz(xRot)` (no flip). The tipped/spectral textures live on the deferred
// texture-backed path, so the colored debug path renders the shaft cross and the head with two tints.

// `back`: the 0×5×5 arrowhead plane, `withScale(0.8)` baked into the centred YZ box → 0×4×4.
const ARROW_BACK_CUBES: [ModelCubeDesc; 1] = [cube([0.0, -2.0, -2.0], [0.0, 4.0, 4.0], ARROW_HEAD)];

// The shared 16×4×0 cross plane (both fletching planes reuse it, differing only in pitch).
const ARROW_CROSS_CUBES: [ModelCubeDesc; 1] =
    [cube([-12.0, -2.0, 0.0], [16.0, 4.0, 0.0], ARROW_SHAFT)];

pub(in crate::entity_models) const ARROW_PARTS: [ModelPartDesc; 3] = [
    rpart(
        [-11.0, 0.0, 0.0],
        [std::f32::consts::FRAC_PI_4, 0.0, 0.0],
        &ARROW_BACK_CUBES,
        &[],
    ),
    rpart(
        [0.0, 0.0, 0.0],
        [std::f32::consts::FRAC_PI_4, 0.0, 0.0],
        &ARROW_CROSS_CUBES,
        &[],
    ),
    rpart(
        [0.0, 0.0, 0.0],
        [3.0 * std::f32::consts::FRAC_PI_4, 0.0, 0.0],
        &ARROW_CROSS_CUBES,
        &[],
    ),
];
