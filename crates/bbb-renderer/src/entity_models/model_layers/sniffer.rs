use super::{
    bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc, SNIFFER_BROWN,
    SNIFFER_NOSE,
};

// Vanilla 26.1 `SnifferModel.createBodyLayer` (atlas 192×192). The mesh root holds one `bone`
// part at `offset(0, 5, 0)` parenting the body and the six legs; `body` parents the head, which
// parents the two ears, the nose, and the lower beak. `setupAnim` sets `head.xRot/yRot` from the
// plain look (reproduced through the projected look angles, the head's ear/nose/beak children
// inheriting the turn); the search/walk (`applyWalk`) and the dig / long-sniff / stand-up / happy /
// scenting keyframe animations are deferred, so the rest of the model renders at this rest pose.
// The texture-backed path is deferred.

// `body`: the 25×29×40 trunk, a 25×24×40 inner block inflated by `CubeDeformation(0.5)` (geometry
// `min -= 0.5`, `size += 1`), and the 25×0×40 belly plane.
const SNIFFER_BODY_CUBES: [ModelCubeDesc; 3] = [
    cube([-12.5, -14.0, -20.0], [25.0, 29.0, 40.0], SNIFFER_BROWN),
    cube([-13.0, -14.5, -20.5], [26.0, 25.0, 41.0], SNIFFER_BROWN),
    cube([-12.5, 12.0, -20.0], [25.0, 0.0, 40.0], SNIFFER_BROWN),
];

// `head`: the 13×18×11 skull plus a 13×0×11 top plane.
const SNIFFER_HEAD_CUBES: [ModelCubeDesc; 2] = [
    cube([-6.5, -7.5, -11.5], [13.0, 18.0, 11.0], SNIFFER_BROWN),
    cube([-6.5, 7.5, -11.5], [13.0, 0.0, 11.0], SNIFFER_BROWN),
];

const SNIFFER_LEFT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, 0.0, -3.0], [1.0, 19.0, 7.0], SNIFFER_BROWN)];
const SNIFFER_RIGHT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, -3.0], [1.0, 19.0, 7.0], SNIFFER_BROWN)];

// The 13×2×9 nose pad (the sniffer's distinctive snout) and the 13×12×9 lower beak / jaw.
const SNIFFER_NOSE_CUBES: [ModelCubeDesc; 1] =
    [cube([-6.5, -2.0, -9.0], [13.0, 2.0, 9.0], SNIFFER_NOSE)];
const SNIFFER_LOWER_BEAK_CUBES: [ModelCubeDesc; 1] =
    [cube([-6.5, -7.0, -8.0], [13.0, 12.0, 9.0], SNIFFER_BROWN)];

// All six legs share one 7×10×8 box.
const SNIFFER_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.5, -1.0, -4.0], [7.0, 10.0, 8.0], SNIFFER_BROWN)];

// `head` children: the two ears, the nose, and the lower beak.
const SNIFFER_HEAD_CHILDREN: [ModelPartDesc; 4] = [
    part([6.51, -7.5, -4.51], &SNIFFER_LEFT_EAR_CUBES, &[]),
    part([-6.51, -7.5, -4.51], &SNIFFER_RIGHT_EAR_CUBES, &[]),
    part([0.0, -4.5, -11.5], &SNIFFER_NOSE_CUBES, &[]),
    part([0.0, 2.5, -12.5], &SNIFFER_LOWER_BEAK_CUBES, &[]),
];

// `body` (at `offset(0, 0, 0)`) parents the head.
const SNIFFER_BODY_CHILDREN: [ModelPartDesc; 1] = [part(
    [0.0, 6.5, -19.48],
    &SNIFFER_HEAD_CUBES,
    &SNIFFER_HEAD_CHILDREN,
)];

// `bone` children: the body and the six legs (right/left × front/mid/hind).
const SNIFFER_BONE_CHILDREN: [ModelPartDesc; 7] = [
    part([0.0, 0.0, 0.0], &SNIFFER_BODY_CUBES, &SNIFFER_BODY_CHILDREN),
    part([-7.5, 10.0, -15.0], &SNIFFER_LEG_CUBES, &[]),
    part([-7.5, 10.0, 0.0], &SNIFFER_LEG_CUBES, &[]),
    part([-7.5, 10.0, 15.0], &SNIFFER_LEG_CUBES, &[]),
    part([7.5, 10.0, -15.0], &SNIFFER_LEG_CUBES, &[]),
    part([7.5, 10.0, 0.0], &SNIFFER_LEG_CUBES, &[]),
    part([7.5, 10.0, 15.0], &SNIFFER_LEG_CUBES, &[]),
];

/// Vanilla `SnifferModel.createBodyLayer` rest-pose hierarchy, rooted at the `bone` part
/// (`offset(0, 5, 0)`). Fifteen cubes.
pub(in crate::entity_models) const SNIFFER_PARTS: [ModelPartDesc; 1] =
    [part([0.0, 5.0, 0.0], &[], &SNIFFER_BONE_CHILDREN)];

/// Child-index path from [`SNIFFER_PARTS`] to the `head` part: bone (`0`) → `body` (child `0`) →
/// `head` (child `0`). Used to apply the plain head look to the nested head (its ear/nose/beak
/// children inherit the turn).
pub(in crate::entity_models) const SNIFFER_HEAD_PART_PATH: &[usize] = &[0, 0, 0];
