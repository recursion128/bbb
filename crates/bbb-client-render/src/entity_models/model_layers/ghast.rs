use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

const GHAST_TENTACLE_NAMES: [&str; 9] = [
    "tentacle0",
    "tentacle1",
    "tentacle2",
    "tentacle3",
    "tentacle4",
    "tentacle5",
    "tentacle6",
    "tentacle7",
    "tentacle8",
];

/// Vanilla `GhastModel.animateTentacles`: each tentacle's `xRot = 0.2 * sin(ageInTicks * 0.3
/// + i) + 0.4`, driven purely by `ageInTicks` (never at rest), so the tentacles always wave.
pub(in crate::entity_models) fn ghast_tentacle_x_rot(index: usize, age_in_ticks: f32) -> f32 {
    0.2 * (age_in_ticks * 0.3 + index as f32).sin() + 0.4
}

/// Vanilla `GhastModel`.
pub(in crate::entity_models) fn ghast_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::GHAST.bake();
    let age_in_ticks = instance.render_state.age_in_ticks;
    for (i, &name) in GHAST_TENTACLE_NAMES.iter().enumerate() {
        root.child_mut(name).pose.rotation[0] = ghast_tentacle_x_rot(i, age_in_ticks);
    }
    root
}
