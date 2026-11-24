use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `SpinAttackEffectModel`.
pub(in crate::entity_models) fn spin_attack_effect_model(
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = ModelLayer::SPIN_ATTACK.bake();
    for (index, name) in ["box0", "box1"].into_iter().enumerate() {
        let angle = instance.render_state.age_in_ticks * -(45.0 + (index as f32 + 1.0) * 5.0);
        root.child_mut(name).pose.rotation[1] = angle.to_radians();
    }
    root
}
