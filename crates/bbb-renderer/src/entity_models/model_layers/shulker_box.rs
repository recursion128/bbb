use super::{PART_POSE_ZERO, SHULKER_BASE_CUBES, SHULKER_LID_CUBES, SHULKER_SHELL_POSE};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

/// Vanilla `ModelLayers.SHULKER_BOX` (`register("shulker_box")`,
/// `ModelLayers.java:217`), baked from `ShulkerModel.createBoxLayer`.
pub(in crate::entity_models) const MODEL_LAYER_SHULKER_BOX: &str = "minecraft:shulker_box#main";

// Vanilla 26.1 `ShulkerModel.createBoxLayer` (`ShulkerModel.java:45-48`, atlas 64×64): the shulker
// mob's `createShellMesh` — the 16×12×16 `lid` box at (-8, -16, -8) texOffs(0, 0) and the 16×8×16
// `base` box at (-8, -8, -8) texOffs(0, 28), both pivoting at `offset(0, 24, 0)` — without the
// mob-only `head`. The cube and pose consts are shared with the shulker mob's body layer
// (`model_layers/shulker.rs`), which bakes the identical shell.

/// Vanilla `ShulkerBoxRenderer.ShulkerBoxModel.setupAnim(progress)`
/// (`ShulkerBoxRenderer.java:141-145`): `lid.setPos(0, 24 - progress·0.5·16, 0)` — the lid rises
/// half a block at full progress — and `lid.yRot = 270°·progress` in radians (the lid corkscrews
/// while opening). Returns `(lid_y, lid_y_rot)`; at `progress = 0` this is `(24, 0)`, the
/// closed/bind pose.
pub(in crate::entity_models) fn shulker_box_lid_pose(progress: f32) -> (f32, f32) {
    let lid_y = 24.0 - progress * 0.5 * 16.0;
    let lid_y_rot = (270.0 * progress).to_radians();
    (lid_y, lid_y_rot)
}

/// Vanilla `ShulkerBoxRenderer.ShulkerBoxModel`: the box shell (`lid` + `base` root children).
/// `setup_anim` transcribes `ShulkerBoxModel.setupAnim` over the projected
/// `shulker_box_progress` (`ShulkerBoxBlockEntity.getProgress(partialTicks)`).
pub(in crate::entity_models) struct ShulkerBoxModel {
    root: ModelPart,
}

impl ShulkerBoxModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![
                    (
                        "lid",
                        ModelPart::leaf(SHULKER_SHELL_POSE, SHULKER_LID_CUBES.to_vec()),
                    ),
                    (
                        "base",
                        ModelPart::leaf(SHULKER_SHELL_POSE, SHULKER_BASE_CUBES.to_vec()),
                    ),
                ],
            ),
        }
    }
}

impl EntityModel for ShulkerBoxModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let (lid_y, lid_y_rot) = shulker_box_lid_pose(instance.render_state.shulker_box_progress);
        let lid = self.root.child_mut("lid");
        lid.pose.offset = [0.0, lid_y, 0.0];
        lid.pose.rotation[1] = lid_y_rot;
    }
}
