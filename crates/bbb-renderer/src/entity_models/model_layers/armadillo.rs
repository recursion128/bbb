use super::super::keyframe::{
    degree_vec, keyframe, pos_vec, AnimationChannel, AnimationDefinition, AnimationTarget,
    BoneAnimation, Keyframe, KeyframeInterpolation,
};
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
// and hind legs. While not hiding, the clamped head look ([`armadillo_clamped_head_look`]) is
// reproduced on the body-nested head pivot, and the `applyWalk` leg sway rocks the body, tail, four
// legs, and head as the armadillo moves (the head walk roll ADDS onto the look). Both the adult
// ([`ARMADILLO_WALK`]) and the baby ([`ARMADILLO_BABY_WALK`], the same bones at slightly different
// timestamps) walks are reproduced. The roll-out / roll-up / peek keyframe transition animations
// (ROLLING/UNROLLING, gated on the un-synced `inStateTicks`) stay deferred. The texture-backed path
// is deferred.

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

/// Vanilla `ArmadilloModel.setupAnim` head look (only while not hiding): the pitch (`xRot`) clamps to
/// [-22.5, 25] and the yaw (`yRot`) to [-32.5, 32.5] degrees before `head.xRot/yRot` are set.
/// Returns the clamped `(yaw, pitch)` in degrees.
pub(in crate::entity_models) fn armadillo_clamped_head_look(
    head_yaw_deg: f32,
    head_pitch_deg: f32,
) -> (f32, f32) {
    (
        head_yaw_deg.clamp(-32.5, 32.5),
        head_pitch_deg.clamp(-22.5, 25.0),
    )
}

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

// ----- `ArmadilloAnimation.ARMADILLO_WALK` (the adult walk; length 1.4583s, looping) -----
//
// `ArmadilloModel.setupAnim` samples it (while not hiding) via
// `applyWalk(walkAnimationPos, walkAnimationSpeed, 16.5, 2.5)`. The `body` channel rolls the trunk
// (a CatmullRom z-sway with a slight y-bob), the four legs swing (rotation + position), the `tail`
// rocks, and the `head` channel adds a small z-roll onto the look the head already tracks. The baby
// (`BabyArmadilloAnimation.ARMADILLO_BABY_WALK`, a different cycle and topology) stays deferred.

const LINEAR: KeyframeInterpolation = KeyframeInterpolation::Linear;
const CATMULLROM: KeyframeInterpolation = KeyframeInterpolation::CatmullRom;

const ARMADILLO_WALK_BODY_ROT: [Keyframe; 9] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, degree_vec(0.0, 0.0, 4.6), CATMULLROM),
    keyframe(0.2917, degree_vec(0.0, 0.0, 6.81), CATMULLROM),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.7083, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.9583, degree_vec(0.0, 0.0, -4.6), CATMULLROM),
    keyframe(1.0, degree_vec(0.0, 0.0, -6.89), CATMULLROM),
    keyframe(1.25, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.4583, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const ARMADILLO_WALK_BODY_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, pos_vec(0.0, -0.2, 0.0), CATMULLROM),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.7083, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.9583, pos_vec(0.0, -0.2, 0.0), CATMULLROM),
    keyframe(1.25, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.4583, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const ARMADILLO_WALK_TAIL_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(-9.17, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(5.0, 0.0, 0.0), LINEAR),
    keyframe(1.2083, degree_vec(-8.24, 0.0, 0.0), LINEAR),
    keyframe(1.4583, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_WALK_RIGHT_HIND_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(-50.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(1.2917, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, degree_vec(-50.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_WALK_RIGHT_HIND_LEG_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(1.2917, pos_vec(0.0, 1.0, -0.18), LINEAR),
    keyframe(1.4583, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_WALK_LEFT_HIND_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(0.5417, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(0.7083, degree_vec(-50.0, 0.0, 0.0), LINEAR),
    keyframe(0.9583, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2083, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, degree_vec(50.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_WALK_LEFT_HIND_LEG_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 0.0, -0.25), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(0.5417, pos_vec(0.0, 1.0, -0.18), LINEAR),
    keyframe(0.7083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.9583, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(1.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, pos_vec(0.0, 0.0, -0.25), LINEAR),
];
const ARMADILLO_WALK_RIGHT_FRONT_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(0.2917, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(0.5417, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(0.7083, degree_vec(-50.0, 0.0, 0.0), LINEAR),
    keyframe(0.9583, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2083, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, degree_vec(50.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_WALK_RIGHT_FRONT_LEG_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 0.0, -0.25), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(0.5417, pos_vec(0.0, 1.0, -0.18), LINEAR),
    keyframe(0.7083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.9583, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(1.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, pos_vec(0.0, 0.0, -0.25), LINEAR),
];
const ARMADILLO_WALK_LEFT_FRONT_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(-50.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(1.2917, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, degree_vec(-50.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_WALK_LEFT_FRONT_LEG_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(1.2917, pos_vec(0.0, 1.0, -0.18), LINEAR),
    keyframe(1.4583, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_WALK_HEAD_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(0.0, 0.0, -2.5), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.7083, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(0.0, 0.0, 2.5), LINEAR),
    keyframe(1.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, degree_vec(0.0, 0.0, 0.0), LINEAR),
];

const fn rot(keyframes: &'static [Keyframe]) -> AnimationChannel {
    AnimationChannel {
        target: AnimationTarget::Rotation,
        keyframes,
    }
}
const fn pos(keyframes: &'static [Keyframe]) -> AnimationChannel {
    AnimationChannel {
        target: AnimationTarget::Position,
        keyframes,
    }
}

const ARMADILLO_WALK_BODY_CHANNELS: [AnimationChannel; 2] =
    [rot(&ARMADILLO_WALK_BODY_ROT), pos(&ARMADILLO_WALK_BODY_POS)];
const ARMADILLO_WALK_TAIL_CHANNELS: [AnimationChannel; 1] = [rot(&ARMADILLO_WALK_TAIL_ROT)];
const ARMADILLO_WALK_RIGHT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_WALK_RIGHT_HIND_LEG_ROT),
    pos(&ARMADILLO_WALK_RIGHT_HIND_LEG_POS),
];
const ARMADILLO_WALK_LEFT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_WALK_LEFT_HIND_LEG_ROT),
    pos(&ARMADILLO_WALK_LEFT_HIND_LEG_POS),
];
const ARMADILLO_WALK_RIGHT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_WALK_RIGHT_FRONT_LEG_ROT),
    pos(&ARMADILLO_WALK_RIGHT_FRONT_LEG_POS),
];
const ARMADILLO_WALK_LEFT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_WALK_LEFT_FRONT_LEG_ROT),
    pos(&ARMADILLO_WALK_LEFT_FRONT_LEG_POS),
];
const ARMADILLO_WALK_HEAD_CHANNELS: [AnimationChannel; 1] = [rot(&ARMADILLO_WALK_HEAD_ROT)];

const ARMADILLO_WALK_BONES: [BoneAnimation; 7] = [
    BoneAnimation {
        bone: "body",
        channels: &ARMADILLO_WALK_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "tail",
        channels: &ARMADILLO_WALK_TAIL_CHANNELS,
    },
    BoneAnimation {
        bone: "right_hind_leg",
        channels: &ARMADILLO_WALK_RIGHT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_hind_leg",
        channels: &ARMADILLO_WALK_LEFT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_front_leg",
        channels: &ARMADILLO_WALK_RIGHT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_front_leg",
        channels: &ARMADILLO_WALK_LEFT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &ARMADILLO_WALK_HEAD_CHANNELS,
    },
];

/// Vanilla `ArmadilloAnimation.ARMADILLO_WALK`: the looping 1.4583s adult walk cycle, sampled by
/// `ArmadilloModel.setupAnim` (while not hiding) via
/// `applyWalk(walkAnimationPos, walkAnimationSpeed, 16.5, 2.5)`. The `head` channel adds onto the
/// clamped look the head already tracks; the `body` channel uses CatmullRom interpolation.
pub(in crate::entity_models) const ARMADILLO_WALK: AnimationDefinition = AnimationDefinition {
    length_seconds: 1.4583,
    looping: true,
    bones: &ARMADILLO_WALK_BONES,
};

/// Vanilla `ArmadilloModel.applyWalk(..., 16.5F, 2.5F)` factors: `MAX_WALK_ANIMATION_SPEED` drives
/// the sample time and `WALK_ANIMATION_SCALE_FACTOR` the amplitude. The base `ArmadilloModel` passes
/// these for both the adult and the baby walk.
pub(in crate::entity_models) const ARMADILLO_WALK_SPEED_FACTOR: f32 = 16.5;
pub(in crate::entity_models) const ARMADILLO_WALK_SCALE_FACTOR: f32 = 2.5;

// ----- `BabyArmadilloAnimation.ARMADILLO_BABY_WALK` (the baby walk; length 1.4583s, looping) -----
//
// The same seven bones (body / tail / four legs / head) and value structure as the adult walk, with
// slightly different keyframe timestamps. The baby topology differs (the tail parents a stub cube,
// the head cube parents the ears), but the animated bone names line up.

const ARMADILLO_BABY_WALK_BODY_ROT: [Keyframe; 9] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, degree_vec(0.0, 0.0, 4.6), CATMULLROM),
    keyframe(0.3, degree_vec(0.0, 0.0, 6.81), CATMULLROM),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.7, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.95, degree_vec(0.0, 0.0, -4.6), CATMULLROM),
    keyframe(1.0, degree_vec(0.0, 0.0, -6.89), CATMULLROM),
    keyframe(1.25, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.45, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const ARMADILLO_BABY_WALK_BODY_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, pos_vec(0.0, -0.2, 0.0), CATMULLROM),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.7, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.95, pos_vec(0.0, -0.2, 0.0), CATMULLROM),
    keyframe(1.25, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.45, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const ARMADILLO_BABY_WALK_TAIL_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(-9.17, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(5.0, 0.0, 0.0), LINEAR),
    keyframe(1.2, degree_vec(-8.24, 0.0, 0.0), LINEAR),
    keyframe(1.45, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_BABY_WALK_RIGHT_HIND_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(-50.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(1.3, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(1.45, degree_vec(-50.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_BABY_WALK_RIGHT_HIND_LEG_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(1.3, pos_vec(0.0, 1.0, -0.18), LINEAR),
    keyframe(1.45, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_BABY_WALK_LEFT_HIND_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(0.55, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(0.7, degree_vec(-50.0, 0.0, 0.0), LINEAR),
    keyframe(0.95, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.45, degree_vec(50.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_BABY_WALK_LEFT_HIND_LEG_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 0.0, -0.25), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(0.55, pos_vec(0.0, 1.0, -0.18), LINEAR),
    keyframe(0.7, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.95, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(1.2, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.45, pos_vec(0.0, 0.0, -0.25), LINEAR),
];
const ARMADILLO_BABY_WALK_RIGHT_FRONT_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(0.3, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(0.55, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(0.7, degree_vec(-50.0, 0.0, 0.0), LINEAR),
    keyframe(0.95, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.45, degree_vec(50.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_BABY_WALK_RIGHT_FRONT_LEG_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 0.0, -0.25), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(0.55, pos_vec(0.0, 1.0, -0.18), LINEAR),
    keyframe(0.7, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.95, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(1.2, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.45, pos_vec(0.0, 0.0, -0.25), LINEAR),
];
const ARMADILLO_BABY_WALK_LEFT_FRONT_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(-50.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(1.3, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(1.45, degree_vec(-50.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_BABY_WALK_LEFT_FRONT_LEG_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(1.3, pos_vec(0.0, 1.0, -0.18), LINEAR),
    keyframe(1.45, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_BABY_WALK_HEAD_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(0.0, 0.0, -2.5), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.7, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(0.0, 0.0, 2.5), LINEAR),
    keyframe(1.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.45, degree_vec(0.0, 0.0, 0.0), LINEAR),
];

const ARMADILLO_BABY_WALK_BODY_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_BABY_WALK_BODY_ROT),
    pos(&ARMADILLO_BABY_WALK_BODY_POS),
];
const ARMADILLO_BABY_WALK_TAIL_CHANNELS: [AnimationChannel; 1] =
    [rot(&ARMADILLO_BABY_WALK_TAIL_ROT)];
const ARMADILLO_BABY_WALK_RIGHT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_BABY_WALK_RIGHT_HIND_LEG_ROT),
    pos(&ARMADILLO_BABY_WALK_RIGHT_HIND_LEG_POS),
];
const ARMADILLO_BABY_WALK_LEFT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_BABY_WALK_LEFT_HIND_LEG_ROT),
    pos(&ARMADILLO_BABY_WALK_LEFT_HIND_LEG_POS),
];
const ARMADILLO_BABY_WALK_RIGHT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_BABY_WALK_RIGHT_FRONT_LEG_ROT),
    pos(&ARMADILLO_BABY_WALK_RIGHT_FRONT_LEG_POS),
];
const ARMADILLO_BABY_WALK_LEFT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_BABY_WALK_LEFT_FRONT_LEG_ROT),
    pos(&ARMADILLO_BABY_WALK_LEFT_FRONT_LEG_POS),
];
const ARMADILLO_BABY_WALK_HEAD_CHANNELS: [AnimationChannel; 1] =
    [rot(&ARMADILLO_BABY_WALK_HEAD_ROT)];

const ARMADILLO_BABY_WALK_BONES: [BoneAnimation; 7] = [
    BoneAnimation {
        bone: "body",
        channels: &ARMADILLO_BABY_WALK_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "tail",
        channels: &ARMADILLO_BABY_WALK_TAIL_CHANNELS,
    },
    BoneAnimation {
        bone: "right_hind_leg",
        channels: &ARMADILLO_BABY_WALK_RIGHT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_hind_leg",
        channels: &ARMADILLO_BABY_WALK_LEFT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_front_leg",
        channels: &ARMADILLO_BABY_WALK_RIGHT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_front_leg",
        channels: &ARMADILLO_BABY_WALK_LEFT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &ARMADILLO_BABY_WALK_HEAD_CHANNELS,
    },
];

/// Vanilla `BabyArmadilloAnimation.ARMADILLO_BABY_WALK`: the looping 1.4583s baby walk cycle,
/// sampled like the adult via `applyWalk(walkAnimationPos, walkAnimationSpeed, 16.5, 2.5)`. Same
/// seven bones and structure as [`ARMADILLO_WALK`], with slightly different keyframe timestamps.
pub(in crate::entity_models) const ARMADILLO_BABY_WALK: AnimationDefinition = AnimationDefinition {
    length_seconds: 1.4583,
    looping: true,
    bones: &ARMADILLO_BABY_WALK_BONES,
};
