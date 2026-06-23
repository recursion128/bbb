use super::{
    bind_part as part, bind_part_rot as rpart, model_cube as cube, ModelCubeDesc, ModelPartDesc,
    PartPose, WITHER_BODY, WITHER_HEAD,
};

// Vanilla 26.1 `WitherBossModel.createBodyLayer(CubeDeformation.NONE)` (atlas 64×64). The mesh root
// holds six sibling parts: the shoulders bar, the ribcage (its spine plus three rib bars), the
// hanging tail, the center head, and the two side heads. The ribcage and tail carry their baked
// rest rotation; the tail's bind position is `(-2, 6.9 + cos(0.20420352) * 10, -0.5 +
// sin(0.20420352) * 10)`, derived from the ribcage's bind pitch (the `anim = 0` rest of the
// breathing sway below). The center head (part 3) follows the plain head look
// (`centerHead.yRot/xRot = state.yRot/xRot`), reproduced via `head_look_pose`; the ribcage and tail
// breathe with `cos(ageInTicks * 0.1)` via [`wither_breathing_poses`]. The two side heads' target
// tracking is deferred (the `DATA_TARGET_*` head targets are client-tick lerped). The
// `WITHER_ARMOR` invulnerable-shimmer overlay layer (the same mesh re-rendered with
// `INNER_ARMOR_DEFORMATION`) and the texture-backed path are deferred, so the colored debug path
// renders a dark body tint plus a lighter head tint.

// `shoulders`: the 20×3×3 bar.
const WITHER_SHOULDERS_CUBES: [ModelCubeDesc; 1] =
    [cube([-10.0, 3.9, -0.5], [20.0, 3.0, 3.0], WITHER_BODY)];

// `ribcage`: the 3×10×3 spine plus three 11×2×2 rib bars (`texOffs(24,22)`, stacked along Y).
const WITHER_RIBCAGE_CUBES: [ModelCubeDesc; 4] = [
    cube([0.0, 0.0, 0.0], [3.0, 10.0, 3.0], WITHER_BODY),
    cube([-4.0, 1.5, 0.5], [11.0, 2.0, 2.0], WITHER_BODY),
    cube([-4.0, 4.0, 0.5], [11.0, 2.0, 2.0], WITHER_BODY),
    cube([-4.0, 6.5, 0.5], [11.0, 2.0, 2.0], WITHER_BODY),
];

// `tail`: the 3×6×3 hanging spine segment.
const WITHER_TAIL_CUBES: [ModelCubeDesc; 1] = [cube([0.0, 0.0, 0.0], [3.0, 6.0, 3.0], WITHER_BODY)];

// `center_head`: the 8×8×8 skull.
const WITHER_CENTER_HEAD_CUBES: [ModelCubeDesc; 1] =
    [cube([-4.0, -4.0, -4.0], [8.0, 8.0, 8.0], WITHER_HEAD)];

// The shared 6×6×6 side head (both side heads reuse it, differing only in pivot).
const WITHER_SIDE_HEAD_CUBES: [ModelCubeDesc; 1] =
    [cube([-4.0, -4.0, -4.0], [6.0, 6.0, 6.0], WITHER_HEAD)];

pub(in crate::entity_models) const WITHER_PARTS: [ModelPartDesc; 6] = [
    part([0.0, 0.0, 0.0], &WITHER_SHOULDERS_CUBES, &[]),
    rpart(
        [-2.0, 6.9, -0.5],
        [0.20420352, 0.0, 0.0],
        &WITHER_RIBCAGE_CUBES,
        &[],
    ),
    rpart(
        [-2.0, 16.692228, 1.5278729],
        [0.83252203, 0.0, 0.0],
        &WITHER_TAIL_CUBES,
        &[],
    ),
    part([0.0, 0.0, 0.0], &WITHER_CENTER_HEAD_CUBES, &[]),
    part([-8.0, 4.0, 0.0], &WITHER_SIDE_HEAD_CUBES, &[]),
    part([10.0, 4.0, 0.0], &WITHER_SIDE_HEAD_CUBES, &[]),
];

/// Index of the `ribcage` part in [`WITHER_PARTS`]; it breathes via [`wither_breathing_poses`].
pub(in crate::entity_models) const WITHER_RIBCAGE_PART_INDEX: usize = 1;

/// Index of the `tail` part in [`WITHER_PARTS`]; its hang position and pitch breathe with the
/// ribcage via [`wither_breathing_poses`].
pub(in crate::entity_models) const WITHER_TAIL_PART_INDEX: usize = 2;

/// Index of the `center_head` part in [`WITHER_PARTS`] (vanilla `createBodyLayer` order:
/// shoulders, ribcage, tail, center_head, right_head, left_head). It tracks the plain head look.
pub(in crate::entity_models) const WITHER_CENTER_HEAD_PART_INDEX: usize = 3;

/// Vanilla `WitherBossModel.setupAnim` breathing sway, driven entirely by the projected
/// `ageInTicks`: `anim = cos(ageInTicks * 0.1)` pitches the ribcage to
/// `(0.065 + 0.05 * anim) * PI`, re-hangs the tail from that new pitch
/// (`tail.setPos(-2, 6.9 + cos(ribcage.xRot) * 10, -0.5 + sin(ribcage.xRot) * 10)`), and pitches the
/// tail to `(0.265 + 0.1 * anim) * PI`. At `anim = 0` it collapses to the baked [`WITHER_PARTS`]
/// rest poses, so the sway oscillates symmetrically about the layer pose. Returns the
/// `(ribcage, tail)` poses; the ribcage keeps its bind offset `(-2, 6.9, -0.5)` and only its `xRot`
/// moves. Because `ageInTicks` advances every frame, the wither never sits perfectly still.
pub(in crate::entity_models) fn wither_breathing_poses(age_in_ticks: f32) -> (PartPose, PartPose) {
    use std::f32::consts::PI;
    let anim = (age_in_ticks * 0.1).cos();
    let ribcage_x_rot = (0.065 + 0.05 * anim) * PI;
    let ribcage = PartPose {
        offset: [-2.0, 6.9, -0.5],
        rotation: [ribcage_x_rot, 0.0, 0.0],
    };
    let tail = PartPose {
        offset: [
            -2.0,
            6.9 + ribcage_x_rot.cos() * 10.0,
            -0.5 + ribcage_x_rot.sin() * 10.0,
        ],
        rotation: [(0.265 + 0.1 * anim) * PI, 0.0, 0.0],
    };
    (ribcage, tail)
}
