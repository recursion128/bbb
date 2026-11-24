use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `ShulkerBoxRenderer.ShulkerBoxModel.setupAnim(progress)`
/// `lid.setPos(0, 24 - progress·0.5·16, 0)` — the lid rises
/// half a block at full progress — and `lid.yRot = 270°·progress` in radians (the lid corkscrews
/// while opening). Returns `(lid_y, lid_y_rot)`; at `progress = 0` this is `(24, 0)`, the
/// closed/bind pose.
pub(in crate::entity_models) fn shulker_box_lid_pose(progress: f32) -> (f32, f32) {
    let lid_y = 24.0 - progress * 0.5 * 16.0;
    let lid_y_rot = (270.0 * progress).to_radians();
    (lid_y, lid_y_rot)
}

/// Vanilla `ShulkerBoxModel`.
pub(in crate::entity_models) fn shulker_box_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::SHULKER_BOX.bake();
    let (lid_y, lid_y_rot) = shulker_box_lid_pose(instance.block_entity.shulker_box_progress);
    let lid = root.child_mut("lid");
    lid.pose.offset = [0.0, lid_y, 0.0];
    lid.pose.rotation[1] = lid_y_rot;
    root
}
