use crate::entity_models::model_layer::{BakedRoot, ModelLayer};
use std::f32::consts::PI;

use crate::item_models::SpecialItemCopperGolemPose;

/// Vanilla `CopperGolemStatueModel.setupAnim`.
pub(in crate::entity_models) fn copper_golem_statue_model(
    pose: SpecialItemCopperGolemPose,
) -> BakedRoot {
    let mut root = match pose {
        SpecialItemCopperGolemPose::Standing => ModelLayer::COPPER_GOLEM,
        SpecialItemCopperGolemPose::Running => ModelLayer::COPPER_GOLEM_RUNNING,
        SpecialItemCopperGolemPose::Sitting => ModelLayer::COPPER_GOLEM_SITTING,
        SpecialItemCopperGolemPose::Star => ModelLayer::COPPER_GOLEM_STAR,
    }
    .bake();
    root.pose.offset[1] = 0.0;
    root.pose.rotation[2] = PI;
    root
}
