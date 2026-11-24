use super::PartPose;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla pufferfish fin wiggle for the right fin: `right.zRot = -0.2 + 0.4 *
/// sin(ageInTicks * 0.2)`. The left fin is the negation. Shared verbatim by
/// `PufferfishSmallModel`/`MidModel`/`BigModel.setupAnim` (the small model wiggles its
/// pectoral fins, the mid/big their blue fins), set absolutely over the zeroed rest pose.
pub(in crate::entity_models) fn pufferfish_right_fin_z_rot(age_in_ticks: f32) -> f32 {
    -0.2 + 0.4 * (age_in_ticks * 0.2).sin()
}

/// Applies the fin wiggle `zRot` to a fin part pose, preserving the offset and the zeroed
/// `xRot`/`yRot`.
pub(in crate::entity_models) fn pufferfish_fin_pose(base: PartPose, z_rot: f32) -> PartPose {
    PartPose {
        offset: base.offset,
        rotation: [base.rotation[0], base.rotation[1], z_rot],
    }
}

/// Vanilla `PufferfishSmallModel` / `PufferfishMidModel` / `PufferfishBigModel`, chosen by puff
/// state like `PufferfishRenderer.submit` (any state `>= 2` is big).
pub(in crate::entity_models) fn pufferfish_model(
    puff_state: i32,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let (layer, fins) = match puff_state {
        0 => (ModelLayer::PUFFERFISH_SMALL, ["right_fin", "left_fin"]),
        1 => (
            ModelLayer::PUFFERFISH_MEDIUM,
            ["right_blue_fin", "left_blue_fin"],
        ),
        _ => (
            ModelLayer::PUFFERFISH_BIG,
            ["right_blue_fin", "left_blue_fin"],
        ),
    };
    let mut root = layer.bake();
    // Vanilla `setupAnim` wiggles the two fins on `ageInTicks`: the right fin takes `zRot`, the
    // left its negation, set absolutely over the zeroed rest `zRot` (offset / `xRot` / `yRot`
    // preserved).
    let fin_z = pufferfish_right_fin_z_rot(instance.render_state.age_in_ticks);
    let right = root.child_mut(fins[0]);
    right.pose = pufferfish_fin_pose(right.pose, fin_z);
    let left = root.child_mut(fins[1]);
    left.pose = pufferfish_fin_pose(left.pose, -fin_z);
    root
}
