use super::{
    bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc, LLAMA_SPIT_COLOR,
};

// Vanilla 26.1 `LlamaSpitModel.createBodyLayer` (atlas 64×32). The mesh root holds a single `main`
// part at ZERO with seven 2×2×2 boxes (all at `texOffs(0, 0)`) forming a plus/cross: a centre cube
// with one neighbour stepping out along each of the +X/-X, +Y/-Y, +Z/-Z directions. `LlamaSpitModel`
// has no `setupAnim`, so the model is fully static — nothing is deferred on the geometry side.
// `LlamaSpitRenderer` is a plain `EntityRenderer` that orients the spit along its flight
// (`translate(0, 0.15, 0) · Ry(yRot - 90) · Rz(xRot)`), captured by `llama_spit_model_root_transform`.
// Only the texture-backed path is deferred, so the colored debug path renders the cross with one tint.

const LLAMA_SPIT_CUBES: [ModelCubeDesc; 7] = [
    cube([-4.0, 0.0, 0.0], [2.0, 2.0, 2.0], LLAMA_SPIT_COLOR),
    cube([0.0, -4.0, 0.0], [2.0, 2.0, 2.0], LLAMA_SPIT_COLOR),
    cube([0.0, 0.0, -4.0], [2.0, 2.0, 2.0], LLAMA_SPIT_COLOR),
    cube([0.0, 0.0, 0.0], [2.0, 2.0, 2.0], LLAMA_SPIT_COLOR),
    cube([2.0, 0.0, 0.0], [2.0, 2.0, 2.0], LLAMA_SPIT_COLOR),
    cube([0.0, 2.0, 0.0], [2.0, 2.0, 2.0], LLAMA_SPIT_COLOR),
    cube([0.0, 0.0, 2.0], [2.0, 2.0, 2.0], LLAMA_SPIT_COLOR),
];

pub(in crate::entity_models) const LLAMA_SPIT_PARTS: [ModelPartDesc; 1] =
    [part([0.0, 0.0, 0.0], &LLAMA_SPIT_CUBES, &[])];
