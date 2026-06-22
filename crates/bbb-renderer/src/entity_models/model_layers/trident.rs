use super::{
    bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc, TRIDENT_POLE,
    TRIDENT_SPIKE,
};

// Vanilla 26.1 `TridentModel.createLayer` (atlas 32×32). The mesh root holds the `pole` (a 1×25×1
// shaft), which parents the `base` crossguard and the three spikes (left / middle / right) — all at
// ZERO. `TridentModel` is a `Model<Unit>` with no animation, so the geometry is complete; nothing is
// deferred on the geometry side. `ThrownTridentRenderer` orients the trident along its flight with
// `Ry(yRot - 90)` then `Rz(xRot + 90)` (the `+90` points the upright pole along the flight axis),
// captured by `trident_model_root_transform`. The enchant-foil overlay pass and the texture-backed
// path are deferred, so the colored debug path renders the pole/base in teal and the spikes lighter.

// `pole`: the 1×25×1 shaft.
const TRIDENT_POLE_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, 2.0, -0.5], [1.0, 25.0, 1.0], TRIDENT_POLE)];

// `base`: the 3×2×1 crossguard.
const TRIDENT_BASE_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.5, 0.0, -0.5], [3.0, 2.0, 1.0], TRIDENT_POLE)];

// The three 1×4×1 spikes (`left` / `middle` / `right`; the right one is mirrored on the atlas only).
const TRIDENT_LEFT_SPIKE_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.5, -3.0, -0.5], [1.0, 4.0, 1.0], TRIDENT_SPIKE)];
const TRIDENT_MIDDLE_SPIKE_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, -4.0, -0.5], [1.0, 4.0, 1.0], TRIDENT_SPIKE)];
const TRIDENT_RIGHT_SPIKE_CUBES: [ModelCubeDesc; 1] =
    [cube([1.5, -3.0, -0.5], [1.0, 4.0, 1.0], TRIDENT_SPIKE)];

const TRIDENT_POLE_CHILDREN: [ModelPartDesc; 4] = [
    part([0.0, 0.0, 0.0], &TRIDENT_BASE_CUBES, &[]),
    part([0.0, 0.0, 0.0], &TRIDENT_LEFT_SPIKE_CUBES, &[]),
    part([0.0, 0.0, 0.0], &TRIDENT_MIDDLE_SPIKE_CUBES, &[]),
    part([0.0, 0.0, 0.0], &TRIDENT_RIGHT_SPIKE_CUBES, &[]),
];

pub(in crate::entity_models) const TRIDENT_PARTS: [ModelPartDesc; 1] = [part(
    [0.0, 0.0, 0.0],
    &TRIDENT_POLE_CUBES,
    &TRIDENT_POLE_CHILDREN,
)];
