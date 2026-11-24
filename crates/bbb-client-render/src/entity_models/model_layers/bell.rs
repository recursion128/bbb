use crate::entity_models::model_layer::{BakedRoot, ModelLayer};
use std::f32::consts::PI;

use crate::entity_models::catalog::BellShakeDirection;
use crate::entity_models::instances::EntityModelInstance;

/// Vanilla `BellModel.setupAnim` swing angle:
/// `Mth.sin(ticks / π) / (4 + ticks / 3)` — a decaying oscillation over the shake
/// counter (`BellRenderState.ticks = blockEntity.ticks + partialTicks`).
pub(in crate::entity_models) fn bell_shake_base_rotation(ticks: f32) -> f32 {
    (ticks / PI).sin() / (4.0 + ticks / 3.0)
}

/// Vanilla `BellModel`.
pub(in crate::entity_models) fn bell_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::BELL.bake();
    let mut x_rot = 0.0;
    let mut z_rot = 0.0;
    if let Some(direction) = instance.block_entity.bell_shake_direction {
        let base_rot = bell_shake_base_rotation(instance.block_entity.bell_ticks);
        match direction {
            BellShakeDirection::North => x_rot = -base_rot,
            BellShakeDirection::South => x_rot = base_rot,
            BellShakeDirection::East => z_rot = -base_rot,
            BellShakeDirection::West => z_rot = base_rot,
            BellShakeDirection::Down | BellShakeDirection::Up => {}
        }
    }
    let body = root.child_mut("bell_body");
    body.pose.rotation[0] = x_rot;
    body.pose.rotation[2] = z_rot;
    root
}
