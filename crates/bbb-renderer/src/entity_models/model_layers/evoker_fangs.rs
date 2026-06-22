use super::{
    bind_part as part, bind_part_rot as rpart, model_cube as cube, ModelCubeDesc, ModelPartDesc,
    EVOKER_FANGS_BASE, EVOKER_FANGS_JAW,
};

// Vanilla 26.1 `EvokerFangsModel.createBodyLayer` (atlas 64×32). The mesh root holds the base block
// at `offset(-5, 24, -5)`, which parents the two jaws (a shared 4×14×8 box). The bind-pose jaw
// rotations are exactly the `setupAnim` closed-jaw rest at `biteProgress = 0`: `upperJaw.zRot =
// π - 0.35π = 0.65π = 2.042035` and `lowerJaw.zRot = π + 0.35π = 1.35π = 4.2411504` (the lower jaw
// also carries `yRot = π`). Every `EvokerFangsModel.setupAnim` motion is deferred — the jaw bite
// open/close, the `base.y` drop, and the root emerge scale / `root.y = 24 - 20·preScale` — so the
// model renders at this closed-jaw rest pose. `EvokerFangsRenderer` is a plain `EntityRenderer` that
// applies the standard flip and `-1.501` y-offset but a distinct `Ry(90 - yRot)` yaw (captured by
// `evoker_fangs_model_root_transform`). The texture-backed path is deferred, so the colored debug
// path renders a grey base and lighter-bone jaws.

// `base`: the 10×12×10 block.
const EVOKER_FANGS_BASE_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, 0.0, 0.0], [10.0, 12.0, 10.0], EVOKER_FANGS_BASE)];

// The shared 4×14×8 jaw box (both jaws reuse it, differing only in pivot and rotation).
const EVOKER_FANGS_JAW_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, 0.0, 0.0], [4.0, 14.0, 8.0], EVOKER_FANGS_JAW)];

const EVOKER_FANGS_BASE_CHILDREN: [ModelPartDesc; 2] = [
    rpart(
        [6.5, 0.0, 1.0],
        [0.0, 0.0, 2.042035],
        &EVOKER_FANGS_JAW_CUBES,
        &[],
    ),
    rpart(
        [3.5, 0.0, 9.0],
        [0.0, std::f32::consts::PI, 4.2411504],
        &EVOKER_FANGS_JAW_CUBES,
        &[],
    ),
];

pub(in crate::entity_models) const EVOKER_FANGS_PARTS: [ModelPartDesc; 1] = [part(
    [-5.0, 24.0, -5.0],
    &EVOKER_FANGS_BASE_CUBES,
    &EVOKER_FANGS_BASE_CHILDREN,
)];
