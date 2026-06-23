use std::f32::consts::FRAC_PI_4;

use super::{
    model_cube as cube, ModelCubeDesc, PartPose, PART_POSE_ZERO, WIND_CHARGE_CORE, WIND_CHARGE_WIND,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

// Vanilla 26.1 `WindChargeModel.createBodyLayer` (atlas 64×32). The `bone` root (no cubes) parents
// the `wind` shell and the `wind_charge` core. The `wind` part carries a fixed `yRot = -0.7854`
// (≈ -π/4) bind rotation and two boxes (`texOffs(15, 20)` 8×2×8 + `texOffs(0, 9)` 6×4×6); the
// `wind_charge` core is the `texOffs(0, 0)` 4×4×4 box at ZERO. `setupAnim` counter-rotates the two
// children off `ageInTicks` ([`wind_charge_spin_yrot`]): `wind.yRot = age·16°` (a *set*, overwriting
// the -π/4 bind) and `windCharge.yRot = -age·16°`. The whole model uses `RenderTypes::entityTranslucent`
// and the `WindChargeRenderer` draws the scrolling `breezeWind` texture (a plain `EntityRenderer`, no
// flip/scale), both deferred, so the colored debug path renders the wind shell and core as opaque
// tinted geometry with the counter-spin applied.

pub(in crate::entity_models) const WIND_CHARGE_WIND_CUBES: [ModelCubeDesc; 2] = [
    cube([-4.0, -1.0, -4.0], [8.0, 2.0, 8.0], WIND_CHARGE_WIND),
    cube([-3.0, -2.0, -3.0], [6.0, 4.0, 6.0], WIND_CHARGE_WIND),
];

pub(in crate::entity_models) const WIND_CHARGE_CORE_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.0, -2.0, -2.0], [4.0, 4.0, 4.0], WIND_CHARGE_CORE)];

/// The `wind` shell's fixed `PartPose.offsetAndRotation(0, 0, 0, 0, -π/4, 0)` bind rotation; the
/// `bone` root and `wind_charge` core both sit at `PartPose.ZERO`.
pub(in crate::entity_models) const WIND_CHARGE_WIND_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [0.0, -FRAC_PI_4, 0.0],
};

/// Vanilla `WindChargeModel.setupAnim` spin magnitude: `age·16°` in radians. The shell takes `+`
/// this (a *set*, overwriting the -π/4 bind) and the core `-` this, so they counter-rotate. `16` is
/// the vanilla `ROTATION_SPEED` constant.
pub(in crate::entity_models) fn wind_charge_spin_yrot(age_in_ticks: f32) -> f32 {
    (age_in_ticks * 16.0).to_radians()
}

/// Mutable wind charge model, mirroring vanilla `WindChargeModel`. The cubeless `bone` root parents
/// the named `wind` shell and `wind_charge` core, built from the baked colored geometry. Colored-only
/// (the scrolling translucent texture is deferred): `setup_anim` counter-spins the two children off
/// `ageInTicks` via `child_mut`.
pub(in crate::entity_models) struct WindChargeModel {
    root: ModelPart,
}

impl WindChargeModel {
    pub(in crate::entity_models) fn new() -> Self {
        let bone = ModelPart::new(
            PART_POSE_ZERO,
            Vec::new(),
            vec![
                (
                    "wind",
                    ModelPart::leaf_colored(WIND_CHARGE_WIND_POSE, &WIND_CHARGE_WIND_CUBES),
                ),
                (
                    "wind_charge",
                    ModelPart::leaf_colored(PART_POSE_ZERO, &WIND_CHARGE_CORE_CUBES),
                ),
            ],
        );
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![("bone", bone)]),
        }
    }
}

impl EntityModel for WindChargeModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `WindChargeModel.setupAnim`: the `wind` shell spins `+age·16°` and the core
        // `-age·16°` ([`wind_charge_spin_yrot`]). Both are absolute sets — the shell's overwrites its
        // -π/4 bind — so they apply every frame (the spin is `0` at `ageInTicks = 0`).
        let spin = wind_charge_spin_yrot(instance.render_state.age_in_ticks);
        let bone = self.root.child_mut("bone");
        bone.child_mut("wind").pose.rotation[1] = spin;
        bone.child_mut("wind_charge").pose.rotation[1] = -spin;
    }
}
