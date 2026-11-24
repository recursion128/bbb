use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `ElytraModel`.
pub(in crate::entity_models) fn elytra_model(
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let layer = if baby {
        ModelLayer::ELYTRA_BABY
    } else {
        ModelLayer::ELYTRA
    };
    let mut root = layer.bake();
    let state = &instance.render_state;
    let y = if state.is_crouching { 3.0 } else { 0.0 };
    let left = root.child_mut("left_wing");
    left.pose.offset[1] = y;
    left.pose.rotation[0] = state.elytra_rot_x;
    left.pose.rotation[1] = state.elytra_rot_y;
    left.pose.rotation[2] = state.elytra_rot_z;

    let right = root.child_mut("right_wing");
    right.pose.offset[1] = y;
    right.pose.rotation[0] = state.elytra_rot_x;
    right.pose.rotation[1] = -state.elytra_rot_y;
    right.pose.rotation[2] = -state.elytra_rot_z;
    root
}
