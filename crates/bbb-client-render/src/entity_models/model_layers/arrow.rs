use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `ArrowModel`.
pub(in crate::entity_models) fn arrow_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::ARROW.bake();
    root.pose.rotation[2] += arrow_shake_z_rot(instance.render_state.kind.arrow().shake);
    root
}

/// Vanilla `ArrowModel.setupAnim`: `pow = -sin(shake * 3) * shake`, then
/// `root.zRot += pow * π / 180`.
pub(in crate::entity_models) fn arrow_shake_z_rot(shake: f32) -> f32 {
    if shake > 0.0 {
        -f32::sin(shake * 3.0) * shake * std::f32::consts::PI / 180.0
    } else {
        0.0
    }
}
