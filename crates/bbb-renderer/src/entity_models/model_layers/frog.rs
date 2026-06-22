use super::{ModelCubeDesc, ModelPartDesc, PartPose, FROG_BODY, FROG_EYE};

// Vanilla 26.1 `FrogModel.createBodyLayer` (atlas 48×48). The mesh root holds one `root` part at
// `offset(0, 24, 0)` parenting `body` and the two legs; `body` parents the head (with its eye
// chain), the tongue, and the two arms (with their hands). The `croaking_body` cube is omitted
// because `setupAnim` only makes it visible while the croak animation plays. Every keyframe
// animation (jump, croak, tongue, swim/walk, idle-in-water) is deferred, so the model renders at
// this rest pose. The three frog texture variants share this geometry and are deferred with the
// texture-backed path.

const fn cube(min: [f32; 3], size: [f32; 3], color: [f32; 4]) -> ModelCubeDesc {
    ModelCubeDesc { min, size, color }
}

const fn part(
    offset: [f32; 3],
    cubes: &'static [ModelCubeDesc],
    children: &'static [ModelPartDesc],
) -> ModelPartDesc {
    ModelPartDesc {
        pose: PartPose {
            offset,
            rotation: [0.0, 0.0, 0.0],
        },
        cubes,
        children,
    }
}

// `body`: the `texOffs(3,1)` 7×3×9 box plus the `texOffs(23,22)` 7×0×9 underside plane.
const FROG_BODY_CUBES: [ModelCubeDesc; 2] = [
    cube([-3.5, -2.0, -8.0], [7.0, 3.0, 9.0], FROG_BODY),
    cube([-3.5, -1.0, -8.0], [7.0, 0.0, 9.0], FROG_BODY),
];

// `head`: the `texOffs(23,13)` 7×0×9 top plane plus the `texOffs(0,13)` 7×3×9 box.
const FROG_HEAD_CUBES: [ModelCubeDesc; 2] = [
    cube([-3.5, -1.0, -7.0], [7.0, 0.0, 9.0], FROG_BODY),
    cube([-3.5, -2.0, -7.0], [7.0, 3.0, 9.0], FROG_BODY),
];

// Each eye is the same 3×2×3 box (`texOffs(0,0)`/`(0,5)`).
const FROG_EYE_CUBES: [ModelCubeDesc; 1] = [cube([-1.5, -1.0, -1.5], [3.0, 2.0, 3.0], FROG_EYE)];

const FROG_TONGUE_CUBES: [ModelCubeDesc; 1] = [cube([-2.0, 0.0, -7.1], [4.0, 0.0, 7.0], FROG_BODY)];

// Both arms share the 2×3×3 box; the webbed hands are 8×0×8 planes that differ only in Z origin.
const FROG_ARM_CUBES: [ModelCubeDesc; 1] = [cube([-1.0, 0.0, -1.0], [2.0, 3.0, 3.0], FROG_BODY)];
const FROG_LEFT_HAND_CUBES: [ModelCubeDesc; 1] =
    [cube([-4.0, 0.01, -4.0], [8.0, 0.0, 8.0], FROG_BODY)];
const FROG_RIGHT_HAND_CUBES: [ModelCubeDesc; 1] =
    [cube([-4.0, 0.01, -5.0], [8.0, 0.0, 8.0], FROG_BODY)];

// The legs differ only in X origin; both feet share one 8×0×8 plane.
const FROG_LEFT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, -2.0], [3.0, 3.0, 4.0], FROG_BODY)];
const FROG_RIGHT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.0, 0.0, -2.0], [3.0, 3.0, 4.0], FROG_BODY)];
const FROG_FOOT_CUBES: [ModelCubeDesc; 1] = [cube([-4.0, 0.01, -4.0], [8.0, 0.0, 8.0], FROG_BODY)];

// `eyes` (an empty pivot at `offset(-0.5, 0, 2)`) parents the two eyes.
const FROG_EYE_PARTS: [ModelPartDesc; 2] = [
    part([-1.5, -3.0, -6.5], &FROG_EYE_CUBES, &[]),
    part([2.5, -3.0, -6.5], &FROG_EYE_CUBES, &[]),
];
const FROG_HEAD_CHILDREN: [ModelPartDesc; 1] = [part([-0.5, 0.0, 2.0], &[], &FROG_EYE_PARTS)];

const FROG_LEFT_ARM_CHILDREN: [ModelPartDesc; 1] =
    [part([0.0, 3.0, -1.0], &FROG_LEFT_HAND_CUBES, &[])];
const FROG_RIGHT_ARM_CHILDREN: [ModelPartDesc; 1] =
    [part([0.0, 3.0, 0.0], &FROG_RIGHT_HAND_CUBES, &[])];

// `body` children: head (with eyes), tongue, and the two arms (each with its hand).
const FROG_BODY_CHILDREN: [ModelPartDesc; 4] = [
    part([0.0, -2.0, -1.0], &FROG_HEAD_CUBES, &FROG_HEAD_CHILDREN),
    part([0.0, -1.01, 1.0], &FROG_TONGUE_CUBES, &[]),
    part([4.0, -1.0, -6.5], &FROG_ARM_CUBES, &FROG_LEFT_ARM_CHILDREN),
    part(
        [-4.0, -1.0, -6.5],
        &FROG_ARM_CUBES,
        &FROG_RIGHT_ARM_CHILDREN,
    ),
];

const FROG_LEFT_LEG_CHILDREN: [ModelPartDesc; 1] = [part([2.0, 3.0, 0.0], &FROG_FOOT_CUBES, &[])];
const FROG_RIGHT_LEG_CHILDREN: [ModelPartDesc; 1] = [part([-2.0, 3.0, 0.0], &FROG_FOOT_CUBES, &[])];

// `root` (at `offset(0, 24, 0)`) children: body and the two legs (each with its foot).
const FROG_ROOT_CHILDREN: [ModelPartDesc; 3] = [
    part([0.0, -2.0, 4.0], &FROG_BODY_CUBES, &FROG_BODY_CHILDREN),
    part(
        [3.5, -3.0, 4.0],
        &FROG_LEFT_LEG_CUBES,
        &FROG_LEFT_LEG_CHILDREN,
    ),
    part(
        [-3.5, -3.0, 4.0],
        &FROG_RIGHT_LEG_CUBES,
        &FROG_RIGHT_LEG_CHILDREN,
    ),
];

/// Vanilla `FrogModel.createBodyLayer` rest-pose hierarchy, rooted at the `root` part
/// (`offset(0, 24, 0)`). Fifteen visible cubes (the `croaking_body` is hidden at rest).
pub(in crate::entity_models) const FROG_PARTS: [ModelPartDesc; 1] =
    [part([0.0, 24.0, 0.0], &[], &FROG_ROOT_CHILDREN)];
