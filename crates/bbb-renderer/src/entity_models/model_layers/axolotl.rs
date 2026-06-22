use super::{
    bind_part as part, bind_part_rot as rpart, model_cube as cube, ModelCubeDesc, ModelPartDesc,
    AXOLOTL_BODY, AXOLOTL_GILLS,
};

// Vanilla 26.1 `AdultAxolotlModel` (atlas 64×64) / `BabyAxolotlModel` (atlas 32×32)
// `createBodyLayer`. The axolotl is one of the `AgeableMobRenderer` two-model entities: the synced
// `AgeableMob.DATA_BABY_ID` flag selects the baby body layer, which has its own smaller geometry
// and a different leg topology. The adult body parents the head (which parents the three gill
// planes), the four leg planes, and the tail fin; the baby wraps the body under a `root` bone at
// `offset(0, 24, 0)` and parents the legs/tail/head off the body. Every `setupAnim` animation is
// deferred — the body yaw, the swimming / water-hovering / ground-crawling / lay-still procedural
// sways (adult) and the keyframe swim/walk/idle animations (baby), the play-dead pose, and the
// mirror-leg copy. The five color variants (`Axolotl.Variant`) live on the deferred texture-backed
// path, so the colored debug path renders the lucy (pink) body with one body tint and one gill
// tint. This is the non-animated rest pose.

// ----- Adult -----

// `body` (offset (0, 19.5, 5)): the 8×4×10 trunk plus a 0×5×9 dorsal fin plane.
const ADULT_AXOLOTL_BODY_CUBES: [ModelCubeDesc; 2] = [
    cube([-4.0, -2.0, -9.0], [8.0, 4.0, 10.0], AXOLOTL_BODY),
    cube([0.0, -3.0, -8.0], [0.0, 5.0, 9.0], AXOLOTL_BODY),
];

// `head` (offset (0, 0, -9)): the 8×5×5 skull (`CubeDeformation(0.001)` fudge baked in).
const ADULT_AXOLOTL_HEAD_CUBES: [ModelCubeDesc; 1] = [cube(
    [-4.001, -3.001, -5.001],
    [8.002, 5.002, 5.002],
    AXOLOTL_BODY,
)];

// The three gill planes (top 8×3×0, and the two 3×7×0 side frills), all fudge-inflated.
const ADULT_AXOLOTL_TOP_GILLS_CUBES: [ModelCubeDesc; 1] = [cube(
    [-4.001, -3.001, -0.001],
    [8.002, 3.002, 0.002],
    AXOLOTL_GILLS,
)];
const ADULT_AXOLOTL_LEFT_GILLS_CUBES: [ModelCubeDesc; 1] = [cube(
    [-3.001, -5.001, -0.001],
    [3.002, 7.002, 0.002],
    AXOLOTL_GILLS,
)];
const ADULT_AXOLOTL_RIGHT_GILLS_CUBES: [ModelCubeDesc; 1] = [cube(
    [-0.001, -5.001, -0.001],
    [3.002, 7.002, 0.002],
    AXOLOTL_GILLS,
)];

// The 3×5×0 leg planes — the right legs use the `-2` origin, the left legs the `-1` origin.
const ADULT_AXOLOTL_RIGHT_LEG_CUBES: [ModelCubeDesc; 1] = [cube(
    [-2.001, -0.001, -0.001],
    [3.002, 5.002, 0.002],
    AXOLOTL_BODY,
)];
const ADULT_AXOLOTL_LEFT_LEG_CUBES: [ModelCubeDesc; 1] = [cube(
    [-1.001, -0.001, -0.001],
    [3.002, 5.002, 0.002],
    AXOLOTL_BODY,
)];

// `tail` (offset (0, 0, 1)): the 0×5×12 tail fin plane.
const ADULT_AXOLOTL_TAIL_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, -3.0, 0.0], [0.0, 5.0, 12.0], AXOLOTL_BODY)];

const ADULT_AXOLOTL_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    part([0.0, -3.0, -1.0], &ADULT_AXOLOTL_TOP_GILLS_CUBES, &[]),
    part([-4.0, 0.0, -1.0], &ADULT_AXOLOTL_LEFT_GILLS_CUBES, &[]),
    part([4.0, 0.0, -1.0], &ADULT_AXOLOTL_RIGHT_GILLS_CUBES, &[]),
];

const ADULT_AXOLOTL_BODY_CHILDREN: [ModelPartDesc; 6] = [
    part(
        [0.0, 0.0, -9.0],
        &ADULT_AXOLOTL_HEAD_CUBES,
        &ADULT_AXOLOTL_HEAD_CHILDREN,
    ),
    part([-3.5, 1.0, -1.0], &ADULT_AXOLOTL_RIGHT_LEG_CUBES, &[]),
    part([3.5, 1.0, -1.0], &ADULT_AXOLOTL_LEFT_LEG_CUBES, &[]),
    part([-3.5, 1.0, -8.0], &ADULT_AXOLOTL_RIGHT_LEG_CUBES, &[]),
    part([3.5, 1.0, -8.0], &ADULT_AXOLOTL_LEFT_LEG_CUBES, &[]),
    part([0.0, 0.0, 1.0], &ADULT_AXOLOTL_TAIL_CUBES, &[]),
];

pub(in crate::entity_models) const ADULT_AXOLOTL_PARTS: [ModelPartDesc; 1] = [part(
    [0.0, 19.5, 5.0],
    &ADULT_AXOLOTL_BODY_CUBES,
    &ADULT_AXOLOTL_BODY_CHILDREN,
)];

// ----- Baby -----

// `body` (offset (0, -1.25, 1.75) under the root bone): the 4×2×6 trunk plus a 0×3×5 dorsal fin.
const BABY_AXOLOTL_BODY_CUBES: [ModelCubeDesc; 2] = [
    cube([-2.0, -0.75, -2.75], [4.0, 2.0, 6.0], AXOLOTL_BODY),
    cube([0.0, -1.75, -2.75], [0.0, 3.0, 5.0], AXOLOTL_BODY),
];

// The 3×0×1 horizontal leg planes (the right hind leg is a doubly-rotated pivot/cube pair).
const BABY_AXOLOTL_RIGHT_FRONT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.0, 0.0, -0.5], [3.0, 0.0, 1.0], AXOLOTL_BODY)];
const BABY_AXOLOTL_RIGHT_HIND_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, 0.0, -0.5], [3.0, 0.0, 1.0], AXOLOTL_BODY)];
const BABY_AXOLOTL_LEFT_FRONT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, 0.0, -0.5], [3.0, 0.0, 1.0], AXOLOTL_BODY)];
const BABY_AXOLOTL_LEFT_HIND_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, 0.0, -0.5], [3.0, 0.0, 1.0], AXOLOTL_BODY)];

// `tail` (offset (0, -0.25, 3.25)): the 0×3×8 tail fin plane.
const BABY_AXOLOTL_TAIL_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, -1.5, -1.0], [0.0, 3.0, 8.0], AXOLOTL_BODY)];

// `head` (offset (0, 0.25, -2.75)): the 6×3×4 skull.
const BABY_AXOLOTL_HEAD_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.0, -2.0, -4.0], [6.0, 3.0, 4.0], AXOLOTL_BODY)];

// The three gill planes (two 3×5×0 side frills and the 6×3×0 top frill).
const BABY_AXOLOTL_LEFT_GILLS_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, -3.5, 0.0], [3.0, 5.0, 0.0], AXOLOTL_GILLS)];
const BABY_AXOLOTL_RIGHT_GILLS_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.0, -3.5, 0.0], [3.0, 5.0, 0.0], AXOLOTL_GILLS)];
const BABY_AXOLOTL_TOP_GILLS_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.0, -3.0, 0.0], [6.0, 3.0, 0.0], AXOLOTL_GILLS)];

// `right_hind_leg` is a bare pivot rotated `(yRot, zRot) = (π/2, π/2)`; its `right_leg_r1` child
// carries the cube under a further `(xRot, zRot) = (-π/2, π/2)` rotation.
const BABY_AXOLOTL_RIGHT_HIND_LEG_CHILDREN: [ModelPartDesc; 1] = [rpart(
    [0.0, 0.0, 0.0],
    [-1.5708, 0.0, 1.5708],
    &BABY_AXOLOTL_RIGHT_HIND_LEG_CUBES,
    &[],
)];

const BABY_AXOLOTL_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    part([3.0, -0.5, -2.0], &BABY_AXOLOTL_LEFT_GILLS_CUBES, &[]),
    part([-3.0, -0.5, -2.0], &BABY_AXOLOTL_RIGHT_GILLS_CUBES, &[]),
    part([0.0, -2.0, -2.0], &BABY_AXOLOTL_TOP_GILLS_CUBES, &[]),
];

const BABY_AXOLOTL_BODY_CHILDREN: [ModelPartDesc; 6] = [
    part(
        [-2.0, 0.25, -1.25],
        &BABY_AXOLOTL_RIGHT_FRONT_LEG_CUBES,
        &[],
    ),
    rpart(
        [-2.0, 0.25, 1.75],
        [0.0, 1.5708, 1.5708],
        &[],
        &BABY_AXOLOTL_RIGHT_HIND_LEG_CHILDREN,
    ),
    part([2.0, 0.25, -1.25], &BABY_AXOLOTL_LEFT_FRONT_LEG_CUBES, &[]),
    part([2.0, 0.25, 1.75], &BABY_AXOLOTL_LEFT_HIND_LEG_CUBES, &[]),
    part([0.0, -0.25, 3.25], &BABY_AXOLOTL_TAIL_CUBES, &[]),
    part(
        [0.0, 0.25, -2.75],
        &BABY_AXOLOTL_HEAD_CUBES,
        &BABY_AXOLOTL_HEAD_CHILDREN,
    ),
];

const BABY_AXOLOTL_ROOT_CHILDREN: [ModelPartDesc; 1] = [part(
    [0.0, -1.25, 1.75],
    &BABY_AXOLOTL_BODY_CUBES,
    &BABY_AXOLOTL_BODY_CHILDREN,
)];

pub(in crate::entity_models) const BABY_AXOLOTL_PARTS: [ModelPartDesc; 1] =
    [part([0.0, 24.0, 0.0], &[], &BABY_AXOLOTL_ROOT_CHILDREN)];
