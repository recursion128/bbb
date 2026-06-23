use super::{
    bind_part as part, bind_part_rot as rpart, model_cube as cube, ModelCubeDesc, ModelPartDesc,
    ARMADILLO_SHELL, ARMADILLO_SKIN,
};

// Vanilla 26.1 `AdultArmadilloModel`/`BabyArmadilloModel.createBodyLayer` (atlas 64×64). The mesh
// root parents the body and the four legs directly (no wrapping bone); the body parents the tail
// and the head, and the head parents the head cube and the two ear planes. The armadillo is one of
// the `AgeableMobRenderer` two-model entities: `state.isBaby` (the synced `AgeableMob.DATA_BABY_ID`
// flag) selects the baby body layer, which has its own smaller geometry and a different ear/tail
// topology. The `isHidingInShell` visibility swap is now projected (see
// `ADULT_ARMADILLO_ROLLED_PARTS` / `BABY_ARMADILLO_ROLLED_PARTS`): the synced
// `Armadillo.ArmadilloState.SCARED` shows the shell-ball `cube` and hides the body cubes, tail,
// and hind legs. The clamped head look, the `applyWalk` leg sway, and the roll-out / roll-up /
// peek keyframe transition animations (ROLLING/UNROLLING, gated on the un-synced `inStateTicks`)
// stay deferred, so the non-hiding pose renders at its rest pose. The texture-backed path is
// deferred.

// ----- Adult -----

// `body` (offset (0, 21, 4)): a `CubeDeformation(0.3)` armor shell wrapping the bare 8×8×12 box.
const ADULT_ARMADILLO_BODY_CUBES: [ModelCubeDesc; 2] = [
    cube([-4.3, -7.3, -10.3], [8.6, 8.6, 12.6], ARMADILLO_SHELL),
    cube([-4.0, -7.0, -10.0], [8.0, 8.0, 12.0], ARMADILLO_SHELL),
];

// `tail`: a 1×6×1 plume pitched down by `0.5061` rad.
const ADULT_ARMADILLO_TAIL_CUBES: [ModelCubeDesc; 1] = [cube(
    [-0.5, -0.0865, 0.0933],
    [1.0, 6.0, 1.0],
    ARMADILLO_SKIN,
)];

// `head_cube`: the 3×5×2 snout, pitched up by `-0.3927` rad.
const ADULT_ARMADILLO_HEAD_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.5, -1.0, -1.0], [3.0, 5.0, 2.0], ARMADILLO_SKIN)];

// The two 2×5×0 ear planes (`texOffs(43,10)` / `texOffs(47,10)`).
const ADULT_ARMADILLO_RIGHT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.0, -3.0, 0.0], [2.0, 5.0, 0.0], ARMADILLO_SKIN)];
const ADULT_ARMADILLO_LEFT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, -3.0, 0.0], [2.0, 5.0, 0.0], ARMADILLO_SKIN)];

// The shared 2×3×2 leg box (all four legs reuse it, differing only in pivot).
const ADULT_ARMADILLO_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, -1.0], [2.0, 3.0, 2.0], ARMADILLO_SHELL)];

const ADULT_RIGHT_EAR_CHILDREN: [ModelPartDesc; 1] = [rpart(
    [-0.5, 0.0, -0.6],
    [0.1886, -0.3864, -0.0718],
    &ADULT_ARMADILLO_RIGHT_EAR_CUBES,
    &[],
)];
const ADULT_LEFT_EAR_CHILDREN: [ModelPartDesc; 1] = [rpart(
    [0.5, 1.0, -0.6],
    [0.1886, 0.3864, 0.0718],
    &ADULT_ARMADILLO_LEFT_EAR_CUBES,
    &[],
)];

const ADULT_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    rpart(
        [0.0, 0.0, 0.0],
        [-0.3927, 0.0, 0.0],
        &ADULT_ARMADILLO_HEAD_CUBES,
        &[],
    ),
    part([-1.0, -1.0, 0.0], &[], &ADULT_RIGHT_EAR_CHILDREN),
    part([1.0, -2.0, 0.0], &[], &ADULT_LEFT_EAR_CHILDREN),
];

const ADULT_BODY_CHILDREN: [ModelPartDesc; 2] = [
    rpart(
        [0.0, -3.0, 1.0],
        [0.5061, 0.0, 0.0],
        &ADULT_ARMADILLO_TAIL_CUBES,
        &[],
    ),
    part([0.0, -2.0, -11.0], &[], &ADULT_HEAD_CHILDREN),
];

pub(in crate::entity_models) const ADULT_ARMADILLO_PARTS: [ModelPartDesc; 5] = [
    part(
        [0.0, 21.0, 4.0],
        &ADULT_ARMADILLO_BODY_CUBES,
        &ADULT_BODY_CHILDREN,
    ),
    part([-2.0, 21.0, 4.0], &ADULT_ARMADILLO_LEG_CUBES, &[]),
    part([2.0, 21.0, 4.0], &ADULT_ARMADILLO_LEG_CUBES, &[]),
    part([-2.0, 21.0, -4.0], &ADULT_ARMADILLO_LEG_CUBES, &[]),
    part([2.0, 21.0, -4.0], &ADULT_ARMADILLO_LEG_CUBES, &[]),
];

// Adult shell ball `cube` (root child at (0, 24, 0)): a plain 10×10×10 box, no deformation.
const ADULT_ARMADILLO_BALL_CUBES: [ModelCubeDesc; 1] = [cube(
    [-5.0, -10.0, -6.0],
    [10.0, 10.0, 10.0],
    ARMADILLO_SHELL,
)];

// When `isHidingInShell`, `body.skipDraw` hides the body's own cubes but still traverses its
// children — and only the head subtree stays visible (`tail.visible = false`). So the hiding
// body keeps an empty cube list and just its head child.
const ADULT_BODY_CHILDREN_HIDDEN: [ModelPartDesc; 1] =
    [part([0.0, -2.0, -11.0], &[], &ADULT_HEAD_CHILDREN)];

// Vanilla `ArmadilloModel.setupAnim` rolled-up (`isHidingInShell`) pose: the body cubes, the
// tail, and both HIND legs hide; the head (+ ears), both FRONT legs, and the 10×10×10 `cube`
// ball stay drawn → six cubes. (Steady SCARED state only; the ROLLING/UNROLLING keyframe
// scrunch, gated on the un-synced `inStateTicks`, stays deferred.)
pub(in crate::entity_models) const ADULT_ARMADILLO_ROLLED_PARTS: [ModelPartDesc; 4] = [
    part([0.0, 21.0, 4.0], &[], &ADULT_BODY_CHILDREN_HIDDEN),
    part([-2.0, 21.0, -4.0], &ADULT_ARMADILLO_LEG_CUBES, &[]),
    part([2.0, 21.0, -4.0], &ADULT_ARMADILLO_LEG_CUBES, &[]),
    part([0.0, 24.0, 0.0], &ADULT_ARMADILLO_BALL_CUBES, &[]),
];

// ----- Baby -----

// `body` (offset (0, 20, 0.5)): a `CubeDeformation(0.3)` armor shell over the bare 5×4×6 box.
const BABY_ARMADILLO_BODY_CUBES: [ModelCubeDesc; 2] = [
    cube([-2.8, -2.3, -3.8], [5.6, 4.6, 7.6], ARMADILLO_SHELL),
    cube([-2.5, -2.0, -3.0], [5.0, 4.0, 6.0], ARMADILLO_SHELL),
];

// `tail` cube (vanilla names it `right_ear_cube`): a 1×1×4 stub pitched by `-1.0472` rad.
const BABY_ARMADILLO_TAIL_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, -0.5, -2.0], [1.0, 1.0, 4.0], ARMADILLO_SKIN)];

// `head_cube`: the 2×2×4 snout, pitched up by `0.7417649` rad.
const BABY_ARMADILLO_HEAD_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, -2.0, -4.0], [2.0, 2.0, 4.0], ARMADILLO_SKIN)];

// The two 2×3×0 ear planes (the right one mirrored on the atlas; geometry is identical for colors).
const BABY_ARMADILLO_RIGHT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.8, -2.0, 0.0], [2.0, 3.0, 0.0], ARMADILLO_SKIN)];
const BABY_ARMADILLO_LEFT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.2, -2.0, 0.0], [2.0, 3.0, 0.0], ARMADILLO_SKIN)];

// The shared 2×2×2 leg box.
const BABY_ARMADILLO_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, -1.0], [2.0, 2.0, 2.0], ARMADILLO_SHELL)];

const BABY_TAIL_CHILDREN: [ModelPartDesc; 1] = [rpart(
    [0.0, 1.5, 1.0],
    [-1.0472, 0.0, 0.0],
    &BABY_ARMADILLO_TAIL_CUBES,
    &[],
)];

const BABY_HEAD_CUBE_CHILDREN: [ModelPartDesc; 2] = [
    rpart(
        [-1.0, -2.0, -0.3],
        [-0.4363, -0.1134, 0.0524],
        &BABY_ARMADILLO_RIGHT_EAR_CUBES,
        &[],
    ),
    rpart(
        [1.0, -2.0, -0.3],
        [-0.4363, 0.1134, -0.0524],
        &BABY_ARMADILLO_LEFT_EAR_CUBES,
        &[],
    ),
];

const BABY_HEAD_CHILDREN: [ModelPartDesc; 1] = [rpart(
    [0.0, 0.0, 0.0],
    [0.7417649, 0.0, 0.0],
    &BABY_ARMADILLO_HEAD_CUBES,
    &BABY_HEAD_CUBE_CHILDREN,
)];

const BABY_BODY_CHILDREN: [ModelPartDesc; 2] = [
    part([0.0, 0.0, 3.4], &[], &BABY_TAIL_CHILDREN),
    part([0.0, 0.0, -3.2], &[], &BABY_HEAD_CHILDREN),
];

// The baby front legs carry vanilla's swapped X origins (right at +1.5, left at -1.5).
pub(in crate::entity_models) const BABY_ARMADILLO_PARTS: [ModelPartDesc; 5] = [
    part(
        [0.0, 20.0, 0.5],
        &BABY_ARMADILLO_BODY_CUBES,
        &BABY_BODY_CHILDREN,
    ),
    part([-1.5, 22.0, 2.5], &BABY_ARMADILLO_LEG_CUBES, &[]),
    part([1.5, 22.0, 2.5], &BABY_ARMADILLO_LEG_CUBES, &[]),
    part([1.5, 22.0, -1.5], &BABY_ARMADILLO_LEG_CUBES, &[]),
    part([-1.5, 22.0, -1.5], &BABY_ARMADILLO_LEG_CUBES, &[]),
];

// Baby shell ball `cube` (root child at (0, 20.7, 0.5)): a 6×6×6 box + `CubeDeformation(0.3)` →
// min -3.3, size 6.6.
const BABY_ARMADILLO_BALL_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.3, -3.3, -3.3], [6.6, 6.6, 6.6], ARMADILLO_SHELL)];

// The hiding baby body keeps only its head child (the tail child and body cubes are hidden).
const BABY_BODY_CHILDREN_HIDDEN: [ModelPartDesc; 1] =
    [part([0.0, 0.0, -3.2], &[], &BABY_HEAD_CHILDREN)];

// Baby rolled-up (`isHidingInShell`) pose: same swap as the adult — head (+ ears), both front
// legs, and the 6×6×6 ball stay drawn; the body cubes, tail, and both hind legs hide.
pub(in crate::entity_models) const BABY_ARMADILLO_ROLLED_PARTS: [ModelPartDesc; 4] = [
    part([0.0, 20.0, 0.5], &[], &BABY_BODY_CHILDREN_HIDDEN),
    part([1.5, 22.0, -1.5], &BABY_ARMADILLO_LEG_CUBES, &[]),
    part([-1.5, 22.0, -1.5], &BABY_ARMADILLO_LEG_CUBES, &[]),
    part([0.0, 20.7, 0.5], &BABY_ARMADILLO_BALL_CUBES, &[]),
];
