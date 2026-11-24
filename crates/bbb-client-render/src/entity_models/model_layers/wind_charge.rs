use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

use crate::entity_models::instances::EntityModelInstance;

/// Vanilla `WindChargeModel.setupAnim` spin magnitude: `age·16°` in radians. The shell takes `+`
/// this (a *set*, overwriting the -π/4 bind) and the core `-` this, so they counter-rotate. `16` is
/// the vanilla `ROTATION_SPEED` constant.
pub(in crate::entity_models) fn wind_charge_spin_yrot(age_in_ticks: f32) -> f32 {
    (age_in_ticks * 16.0).to_radians()
}

/// Vanilla `WindChargeModel`.
pub(in crate::entity_models) fn wind_charge_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::WIND_CHARGE.bake();
    // Vanilla `WindChargeModel.setupAnim`: the `wind` shell spins `+age·16°` and the core
    // `-age·16°` ([`wind_charge_spin_yrot`]). Both are absolute sets — the shell's overwrites its
    // -π/4 bind — so they apply every frame (the spin is `0` at `ageInTicks = 0`).
    let spin = wind_charge_spin_yrot(instance.render_state.age_in_ticks);
    let bone = root.child_mut("bone");
    bone.child_mut("wind").pose.rotation[1] = spin;
    bone.child_mut("wind_charge").pose.rotation[1] = -spin;
    root
}
