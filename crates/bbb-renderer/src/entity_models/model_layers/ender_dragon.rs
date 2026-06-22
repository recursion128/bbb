use super::{
    bind_part as part, bind_part_rot as rpart, model_cube as cube, ModelCubeDesc, ModelPartDesc,
    DRAGON_BODY, DRAGON_MEMBRANE,
};

// Vanilla 26.1 `EnderDragonModel.createBodyLayer` (atlas 256×256). The mesh root holds the head
// (parenting the jaw), the five neck segments and twelve tail segments (each the shared 10×10×10
// spine box plus its 2×4×6 dorsal scale), and the body (parenting the two wings — each a bone box
// plus a 56×0×56 membrane plane and a wing tip — and the four three-segment legs). The whole
// `EnderDragonModel.setupAnim` is procedural: every neck/tail segment is re-placed from the
// `DragonFlightHistory` path each frame, the wings flap (`flapTime`), the jaw opens, and the root
// gets the `bounce` y / fixed `z = -48` / `xRot` adjustments. All of that is deferred (like the
// guardian's procedural tail), so the model renders at this straight bind layout. `EnderDragonRenderer`
// applies the flight-history yaw, a pitch, a fixed `translate(0, 0, 1)`, and the standard flip /
// y-offset (captured by `ender_dragon_model_root_transform`). The dying dissolve, the emissive eyes
// layer, the crystal-healing beam, and the texture-backed path are deferred, so the colored debug
// path renders the body dark and the wing membranes a lighter tint.

// ----- Head + jaw -----

// `head` (offset (0, 20, -62)): the upper lip, the upper head, and the mirrored scale/nostril pairs.
const DRAGON_HEAD_CUBES: [ModelCubeDesc; 6] = [
    cube([-6.0, -1.0, -24.0], [12.0, 5.0, 16.0], DRAGON_BODY),
    cube([-8.0, -8.0, -10.0], [16.0, 16.0, 16.0], DRAGON_BODY),
    cube([-5.0, -12.0, -4.0], [2.0, 4.0, 6.0], DRAGON_BODY),
    cube([-5.0, -3.0, -22.0], [2.0, 2.0, 4.0], DRAGON_BODY),
    cube([3.0, -12.0, -4.0], [2.0, 4.0, 6.0], DRAGON_BODY),
    cube([3.0, -3.0, -22.0], [2.0, 2.0, 4.0], DRAGON_BODY),
];

// `jaw` (offset (0, 4, -8)): the lower jaw box.
const DRAGON_JAW_CUBES: [ModelCubeDesc; 1] =
    [cube([-6.0, 0.0, -16.0], [12.0, 4.0, 16.0], DRAGON_BODY)];

const DRAGON_HEAD_CHILDREN: [ModelPartDesc; 1] = [part([0.0, 4.0, -8.0], &DRAGON_JAW_CUBES, &[])];

// ----- Shared spine segment (necks and tails) -----

// The 10×10×10 vertebra box plus its 2×4×6 dorsal scale.
const DRAGON_SPINE_CUBES: [ModelCubeDesc; 2] = [
    cube([-5.0, -5.0, -5.0], [10.0, 10.0, 10.0], DRAGON_BODY),
    cube([-1.0, -9.0, -3.0], [2.0, 4.0, 6.0], DRAGON_BODY),
];

// ----- Body + wings + legs -----

// `body` (offset (0, 3, 8)): the 24×24×64 torso plus the three dorsal scales.
const DRAGON_BODY_CUBES: [ModelCubeDesc; 4] = [
    cube([-12.0, 1.0, -16.0], [24.0, 24.0, 64.0], DRAGON_BODY),
    cube([-1.0, -5.0, -10.0], [2.0, 6.0, 12.0], DRAGON_BODY),
    cube([-1.0, -5.0, 10.0], [2.0, 6.0, 12.0], DRAGON_BODY),
    cube([-1.0, -5.0, 30.0], [2.0, 6.0, 12.0], DRAGON_BODY),
];

// The wings: each a 56×8×8 bone plus a 56×0×56 membrane plane, parenting a 56×4×4 tip bone plus its
// own membrane. Left wings extend +X, right wings extend -X (vanilla's mirror is true geometry here).
const DRAGON_LEFT_WING_CUBES: [ModelCubeDesc; 2] = [
    cube([0.0, -4.0, -4.0], [56.0, 8.0, 8.0], DRAGON_BODY),
    cube([0.0, 0.0, 2.0], [56.0, 0.0, 56.0], DRAGON_MEMBRANE),
];
const DRAGON_LEFT_WING_TIP_CUBES: [ModelCubeDesc; 2] = [
    cube([0.0, -2.0, -2.0], [56.0, 4.0, 4.0], DRAGON_BODY),
    cube([0.0, 0.0, 2.0], [56.0, 0.0, 56.0], DRAGON_MEMBRANE),
];
const DRAGON_RIGHT_WING_CUBES: [ModelCubeDesc; 2] = [
    cube([-56.0, -4.0, -4.0], [56.0, 8.0, 8.0], DRAGON_BODY),
    cube([-56.0, 0.0, 2.0], [56.0, 0.0, 56.0], DRAGON_MEMBRANE),
];
const DRAGON_RIGHT_WING_TIP_CUBES: [ModelCubeDesc; 2] = [
    cube([-56.0, -2.0, -2.0], [56.0, 4.0, 4.0], DRAGON_BODY),
    cube([-56.0, 0.0, 2.0], [56.0, 0.0, 56.0], DRAGON_MEMBRANE),
];
const DRAGON_LEFT_WING_CHILDREN: [ModelPartDesc; 1] =
    [part([56.0, 0.0, 0.0], &DRAGON_LEFT_WING_TIP_CUBES, &[])];
const DRAGON_RIGHT_WING_CHILDREN: [ModelPartDesc; 1] =
    [part([-56.0, 0.0, 0.0], &DRAGON_RIGHT_WING_TIP_CUBES, &[])];

// The legs: front and hind, each a leg → leg-tip → foot chain. The cubes are centred, so the left
// and right legs of each pair share geometry and reuse the same child hierarchies (only the body
// pivot X differs).
const DRAGON_FRONT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-4.0, -4.0, -4.0], [8.0, 24.0, 8.0], DRAGON_BODY)];
const DRAGON_FRONT_LEG_TIP_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.0, -1.0, -3.0], [6.0, 24.0, 6.0], DRAGON_BODY)];
const DRAGON_FRONT_FOOT_CUBES: [ModelCubeDesc; 1] =
    [cube([-4.0, 0.0, -12.0], [8.0, 4.0, 16.0], DRAGON_BODY)];
const DRAGON_HIND_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-8.0, -4.0, -8.0], [16.0, 32.0, 16.0], DRAGON_BODY)];
const DRAGON_HIND_LEG_TIP_CUBES: [ModelCubeDesc; 1] =
    [cube([-6.0, -2.0, 0.0], [12.0, 32.0, 12.0], DRAGON_BODY)];
const DRAGON_HIND_FOOT_CUBES: [ModelCubeDesc; 1] =
    [cube([-9.0, 0.0, -20.0], [18.0, 6.0, 24.0], DRAGON_BODY)];

const DRAGON_FRONT_LEG_TIP_CHILDREN: [ModelPartDesc; 1] = [rpart(
    [0.0, 23.0, 0.0],
    [0.75, 0.0, 0.0],
    &DRAGON_FRONT_FOOT_CUBES,
    &[],
)];
const DRAGON_FRONT_LEG_CHILDREN: [ModelPartDesc; 1] = [rpart(
    [0.0, 20.0, -1.0],
    [-0.5, 0.0, 0.0],
    &DRAGON_FRONT_LEG_TIP_CUBES,
    &DRAGON_FRONT_LEG_TIP_CHILDREN,
)];
const DRAGON_HIND_LEG_TIP_CHILDREN: [ModelPartDesc; 1] = [rpart(
    [0.0, 31.0, 4.0],
    [0.75, 0.0, 0.0],
    &DRAGON_HIND_FOOT_CUBES,
    &[],
)];
const DRAGON_HIND_LEG_CHILDREN: [ModelPartDesc; 1] = [rpart(
    [0.0, 32.0, -4.0],
    [0.5, 0.0, 0.0],
    &DRAGON_HIND_LEG_TIP_CUBES,
    &DRAGON_HIND_LEG_TIP_CHILDREN,
)];

const DRAGON_BODY_CHILDREN: [ModelPartDesc; 6] = [
    part(
        [12.0, 2.0, -6.0],
        &DRAGON_LEFT_WING_CUBES,
        &DRAGON_LEFT_WING_CHILDREN,
    ),
    rpart(
        [12.0, 17.0, -6.0],
        [1.3, 0.0, 0.0],
        &DRAGON_FRONT_LEG_CUBES,
        &DRAGON_FRONT_LEG_CHILDREN,
    ),
    rpart(
        [16.0, 13.0, 34.0],
        [1.0, 0.0, 0.0],
        &DRAGON_HIND_LEG_CUBES,
        &DRAGON_HIND_LEG_CHILDREN,
    ),
    part(
        [-12.0, 2.0, -6.0],
        &DRAGON_RIGHT_WING_CUBES,
        &DRAGON_RIGHT_WING_CHILDREN,
    ),
    rpart(
        [-12.0, 17.0, -6.0],
        [1.3, 0.0, 0.0],
        &DRAGON_FRONT_LEG_CUBES,
        &DRAGON_FRONT_LEG_CHILDREN,
    ),
    rpart(
        [-16.0, 13.0, 34.0],
        [1.0, 0.0, 0.0],
        &DRAGON_HIND_LEG_CUBES,
        &DRAGON_HIND_LEG_CHILDREN,
    ),
];

// The mesh root: head, the five neck segments (`offset(0, 20, -12 - i·10)`), the twelve tail
// segments (`offset(0, 10, 60 + i·10)`), and the body.
pub(in crate::entity_models) const ENDER_DRAGON_PARTS: [ModelPartDesc; 19] = [
    part(
        [0.0, 20.0, -62.0],
        &DRAGON_HEAD_CUBES,
        &DRAGON_HEAD_CHILDREN,
    ),
    part([0.0, 20.0, -12.0], &DRAGON_SPINE_CUBES, &[]),
    part([0.0, 20.0, -22.0], &DRAGON_SPINE_CUBES, &[]),
    part([0.0, 20.0, -32.0], &DRAGON_SPINE_CUBES, &[]),
    part([0.0, 20.0, -42.0], &DRAGON_SPINE_CUBES, &[]),
    part([0.0, 20.0, -52.0], &DRAGON_SPINE_CUBES, &[]),
    part([0.0, 10.0, 60.0], &DRAGON_SPINE_CUBES, &[]),
    part([0.0, 10.0, 70.0], &DRAGON_SPINE_CUBES, &[]),
    part([0.0, 10.0, 80.0], &DRAGON_SPINE_CUBES, &[]),
    part([0.0, 10.0, 90.0], &DRAGON_SPINE_CUBES, &[]),
    part([0.0, 10.0, 100.0], &DRAGON_SPINE_CUBES, &[]),
    part([0.0, 10.0, 110.0], &DRAGON_SPINE_CUBES, &[]),
    part([0.0, 10.0, 120.0], &DRAGON_SPINE_CUBES, &[]),
    part([0.0, 10.0, 130.0], &DRAGON_SPINE_CUBES, &[]),
    part([0.0, 10.0, 140.0], &DRAGON_SPINE_CUBES, &[]),
    part([0.0, 10.0, 150.0], &DRAGON_SPINE_CUBES, &[]),
    part([0.0, 10.0, 160.0], &DRAGON_SPINE_CUBES, &[]),
    part([0.0, 10.0, 170.0], &DRAGON_SPINE_CUBES, &[]),
    part([0.0, 3.0, 8.0], &DRAGON_BODY_CUBES, &DRAGON_BODY_CHILDREN),
];
