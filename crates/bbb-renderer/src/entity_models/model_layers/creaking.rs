use super::{bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc, CREAKING_BARK};

// Vanilla 26.1 `CreakingModel.createBodyLayer` (atlas 64×64). The mesh root holds one `root` part
// at `offset(0, 24, 0)` parenting `upper_body` and the two legs; `upper_body` (an empty pivot)
// parents the head (with its two antler/branch planes), the body, and the two arms. Every
// `setupAnim` animation — the head look, walk (`applyWalk`), attack, invulnerable, and death
// keyframe animations — is deferred, so the model renders at this rest pose (fittingly, the
// creaking freezes into a statue while observed). The emissive eyes layer
// (`createEyesLayer`, the `head` part only) and the texture-backed path are also deferred.

// `head`: the 6×10×6 skull, the 6×3×6 brow, and two 9×14×0 antler/branch planes.
const CREAKING_HEAD_CUBES: [ModelCubeDesc; 4] = [
    cube([-3.0, -10.0, -3.0], [6.0, 10.0, 6.0], CREAKING_BARK),
    cube([-3.0, -13.0, -3.0], [6.0, 3.0, 6.0], CREAKING_BARK),
    cube([3.0, -13.0, 0.0], [9.0, 14.0, 0.0], CREAKING_BARK),
    cube([-12.0, -14.0, 0.0], [9.0, 14.0, 0.0], CREAKING_BARK),
];

// `body`: the 6×13×5 trunk plus the 6×7×5 upper block.
const CREAKING_BODY_CUBES: [ModelCubeDesc; 2] = [
    cube([0.0, -3.0, -3.0], [6.0, 13.0, 5.0], CREAKING_BARK),
    cube([-6.0, -4.0, -3.0], [6.0, 7.0, 5.0], CREAKING_BARK),
];

// `right_arm`: a 3×21×3 limb plus a 3×4×3 hand.
const CREAKING_RIGHT_ARM_CUBES: [ModelCubeDesc; 2] = [
    cube([-2.0, -1.5, -1.5], [3.0, 21.0, 3.0], CREAKING_BARK),
    cube([-2.0, 19.5, -1.5], [3.0, 4.0, 3.0], CREAKING_BARK),
];

// `left_arm`: a 3×16×3 limb with a 3×4×3 shoulder block and a 3×4×3 hand.
const CREAKING_LEFT_ARM_CUBES: [ModelCubeDesc; 3] = [
    cube([0.0, -1.0, -1.5], [3.0, 16.0, 3.0], CREAKING_BARK),
    cube([0.0, -5.0, -1.5], [3.0, 4.0, 3.0], CREAKING_BARK),
    cube([0.0, 15.0, -1.5], [3.0, 4.0, 3.0], CREAKING_BARK),
];

// `left_leg`: a 3×16×3 limb plus a 5×0×9 foot plane.
const CREAKING_LEFT_LEG_CUBES: [ModelCubeDesc; 2] = [
    cube([-1.5, 0.0, -1.5], [3.0, 16.0, 3.0], CREAKING_BARK),
    cube([-1.5, 15.7, -4.5], [5.0, 0.0, 9.0], CREAKING_BARK),
];

// `right_leg`: a 3×19×3 limb, a 5×0×9 foot plane, and a 3×3×3 hip block.
const CREAKING_RIGHT_LEG_CUBES: [ModelCubeDesc; 3] = [
    cube([-3.0, -1.5, -1.5], [3.0, 19.0, 3.0], CREAKING_BARK),
    cube([-5.0, 17.2, -4.5], [5.0, 0.0, 9.0], CREAKING_BARK),
    cube([-3.0, -4.5, -1.5], [3.0, 3.0, 3.0], CREAKING_BARK),
];

// `upper_body` children: head, body, and the two arms.
const CREAKING_UPPER_BODY_CHILDREN: [ModelPartDesc; 4] = [
    part([-3.0, -11.0, 0.0], &CREAKING_HEAD_CUBES, &[]),
    part([0.0, -7.0, 1.0], &CREAKING_BODY_CUBES, &[]),
    part([-7.0, -9.5, 1.5], &CREAKING_RIGHT_ARM_CUBES, &[]),
    part([6.0, -9.0, 0.5], &CREAKING_LEFT_ARM_CUBES, &[]),
];

// `root` children: the `upper_body` pivot and the two legs.
const CREAKING_ROOT_CHILDREN: [ModelPartDesc; 3] = [
    part([-1.0, -19.0, 0.0], &[], &CREAKING_UPPER_BODY_CHILDREN),
    part([1.5, -16.0, 0.5], &CREAKING_LEFT_LEG_CUBES, &[]),
    part([-1.0, -17.5, 0.5], &CREAKING_RIGHT_LEG_CUBES, &[]),
];

/// Vanilla `CreakingModel.createBodyLayer` rest-pose hierarchy, rooted at the `root` part
/// (`offset(0, 24, 0)`). Sixteen cubes.
pub(in crate::entity_models) const CREAKING_PARTS: [ModelPartDesc; 1] =
    [part([0.0, 24.0, 0.0], &[], &CREAKING_ROOT_CHILDREN)];
