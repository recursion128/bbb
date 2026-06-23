use super::{
    bind_part as part, bind_part_rot as part_rot, model_cube as cube, ModelCubeDesc, ModelPartDesc,
    WIND_CHARGE_CORE, WIND_CHARGE_WIND,
};

// Vanilla 26.1 `WindChargeModel.createBodyLayer` (atlas 64×32). The `bone` root (no cubes) parents
// the `wind` shell and the `wind_charge` core. The `wind` part carries a fixed `yRot = -0.7854`
// (≈ -π/4) bind rotation and two boxes (`texOffs(15, 20)` 8×2×8 + `texOffs(0, 9)` 6×4×6); the
// `wind_charge` core is the `texOffs(0, 0)` 4×4×4 box at ZERO. `setupAnim` counter-rotates the two
// children off `ageInTicks` ([`wind_charge_spin_yrot`]): `wind.yRot = age·16°` (a *set*, overwriting
// the -π/4 bind) and `windCharge.yRot = -age·16°`. The whole model uses `RenderTypes::entityTranslucent`
// and the `WindChargeRenderer` draws the scrolling `breezeWind` texture (a plain `EntityRenderer`, no
// flip/scale), both deferred, so the colored debug path renders the wind shell and core as opaque
// tinted geometry with the counter-spin applied.

const WIND_CHARGE_WIND_CUBES: [ModelCubeDesc; 2] = [
    cube([-4.0, -1.0, -4.0], [8.0, 2.0, 8.0], WIND_CHARGE_WIND),
    cube([-3.0, -2.0, -3.0], [6.0, 4.0, 6.0], WIND_CHARGE_WIND),
];

const WIND_CHARGE_CORE_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.0, -2.0, -2.0], [4.0, 4.0, 4.0], WIND_CHARGE_CORE)];

const WIND_CHARGE_BONE_CHILDREN: [ModelPartDesc; 2] = [
    part_rot(
        [0.0, 0.0, 0.0],
        [0.0, -std::f32::consts::FRAC_PI_4, 0.0],
        &WIND_CHARGE_WIND_CUBES,
        &[],
    ),
    part([0.0, 0.0, 0.0], &WIND_CHARGE_CORE_CUBES, &[]),
];

pub(in crate::entity_models) const WIND_CHARGE_PARTS: [ModelPartDesc; 1] =
    [part([0.0, 0.0, 0.0], &[], &WIND_CHARGE_BONE_CHILDREN)];

/// The `bone`'s two children: the `wind` shell (`0`) spins `+age·16°` and the `wind_charge` core
/// (`1`) counter-spins `-age·16°`.
pub(in crate::entity_models) const WIND_CHARGE_WIND_CHILD_INDEX: usize = 0;
pub(in crate::entity_models) const WIND_CHARGE_CORE_CHILD_INDEX: usize = 1;

/// Vanilla `WindChargeModel.setupAnim` spin magnitude: `age·16°` in radians. The shell takes `+`
/// this (a *set*, overwriting the -π/4 bind) and the core `-` this, so they counter-rotate. `16` is
/// the vanilla `ROTATION_SPEED` constant.
pub(in crate::entity_models) fn wind_charge_spin_yrot(age_in_ticks: f32) -> f32 {
    (age_in_ticks * 16.0).to_radians()
}
