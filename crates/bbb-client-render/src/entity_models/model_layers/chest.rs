use crate::entity_models::catalog::ChestModelHalf;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};
use std::f32::consts::FRAC_PI_2;

/// Vanilla `ChestModel.setupAnim(open)`: the lid and lock share the final openness.
pub(in crate::entity_models) fn chest_model(half: ChestModelHalf, openness: f32) -> BakedRoot {
    let layer = match half {
        ChestModelHalf::Single => ModelLayer::CHEST,
        ChestModelHalf::Left => ModelLayer::DOUBLE_CHEST_LEFT,
        ChestModelHalf::Right => ModelLayer::DOUBLE_CHEST_RIGHT,
    };
    let mut root = layer.bake();
    let x_rot = -(openness * FRAC_PI_2);
    root.child_mut("lid").pose.rotation[0] = x_rot;
    root.child_mut("lock").pose.rotation[0] = x_rot;
    root
}

/// Vanilla `ChestRenderer.submit` easing over the combined raw openness:
/// `o = 1 - o; o = 1 - o * o * o`.
pub(in crate::entity_models) fn chest_lid_eased_openness(openness: f32) -> f32 {
    let inverted = 1.0 - openness;
    1.0 - inverted * inverted * inverted
}
