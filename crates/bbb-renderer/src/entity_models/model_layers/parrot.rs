use super::{
    bind_part as part, bind_part_rot as rpart, model_cube as cube, ModelCubeDesc, ModelPartDesc,
    PARROT_BEAK, PARROT_BODY,
};

// Vanilla 26.1 `ParrotModel.createBodyLayer` (atlas 32×32). The mesh root holds seven sibling parts
// (body, tail, the two wings, head, and the two legs); the head parents the upper-head block, the
// two beak halves, and the crest feather. Most parts carry a baked rest rotation (the wings are
// additionally flipped `yRot = -π`). Every `ParrotModel.setupAnim` motion is deferred — the head
// look, the per-pose `prepare` offsets (the FLYING leg pitch, the SITTING crouch), the leg walk
// swing, the wing flap (`zRot = ±(0.0873 + flapAngle)`), the body/tail/head flap bob, and the PARTY
// dance — so the model renders at this STANDING rest pose. The five `Parrot.Variant` colors live on
// the deferred texture-backed path, so the colored debug path renders one body tint plus a beak
// tint. Parrot uses a plain `MobRenderer` with no transform overrides.

// `body`: the 3×6×3 torso.
const PARROT_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.5, 0.0, -1.5], [3.0, 6.0, 3.0], PARROT_BODY)];

// `tail`: the 3×4×1 plate.
const PARROT_TAIL_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.5, -1.0, -1.0], [3.0, 4.0, 1.0], PARROT_BODY)];

// The shared 1×5×3 wing (both wings reuse it, differing only in pivot X sign).
const PARROT_WING_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, 0.0, -1.5], [1.0, 5.0, 3.0], PARROT_BODY)];

// `head`: the 2×3×2 skull.
const PARROT_HEAD_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, -1.5, -1.0], [2.0, 3.0, 2.0], PARROT_BODY)];

// `head2`: the 2×1×4 upper-head block.
const PARROT_HEAD2_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, -0.5, -2.0], [2.0, 1.0, 4.0], PARROT_BODY)];

// `beak1` / `beak2`: the two 1×2×1 beak halves.
const PARROT_BEAK1_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, -1.0, -0.5], [1.0, 2.0, 1.0], PARROT_BEAK)];
const PARROT_BEAK2_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, 0.0, -0.5], [1.0, 2.0, 1.0], PARROT_BEAK)];

// `feather`: the 0×5×4 crest plane.
const PARROT_FEATHER_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, -4.0, -2.0], [0.0, 5.0, 4.0], PARROT_BODY)];

// The shared 1×2×1 leg (both legs reuse it, differing only in pivot X sign).
const PARROT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, 0.0, -0.5], [1.0, 2.0, 1.0], PARROT_BODY)];

const PARROT_HEAD_CHILDREN: [ModelPartDesc; 4] = [
    part([0.0, -2.0, -1.0], &PARROT_HEAD2_CUBES, &[]),
    part([0.0, -0.5, -1.5], &PARROT_BEAK1_CUBES, &[]),
    part([0.0, -1.75, -2.45], &PARROT_BEAK2_CUBES, &[]),
    rpart(
        [0.0, -2.15, 0.15],
        [-0.2214, 0.0, 0.0],
        &PARROT_FEATHER_CUBES,
        &[],
    ),
];

pub(in crate::entity_models) const PARROT_PARTS: [ModelPartDesc; 7] = [
    rpart(
        [0.0, 16.5, -3.0],
        [0.4937, 0.0, 0.0],
        &PARROT_BODY_CUBES,
        &[],
    ),
    rpart(
        [0.0, 21.07, 1.16],
        [1.015, 0.0, 0.0],
        &PARROT_TAIL_CUBES,
        &[],
    ),
    rpart(
        [1.5, 16.94, -2.76],
        [-0.6981, -std::f32::consts::PI, 0.0],
        &PARROT_WING_CUBES,
        &[],
    ),
    rpart(
        [-1.5, 16.94, -2.76],
        [-0.6981, -std::f32::consts::PI, 0.0],
        &PARROT_WING_CUBES,
        &[],
    ),
    part(
        [0.0, 15.69, -2.76],
        &PARROT_HEAD_CUBES,
        &PARROT_HEAD_CHILDREN,
    ),
    rpart(
        [1.0, 22.0, -1.05],
        [-0.0299, 0.0, 0.0],
        &PARROT_LEG_CUBES,
        &[],
    ),
    rpart(
        [-1.0, 22.0, -1.05],
        [-0.0299, 0.0, 0.0],
        &PARROT_LEG_CUBES,
        &[],
    ),
];
