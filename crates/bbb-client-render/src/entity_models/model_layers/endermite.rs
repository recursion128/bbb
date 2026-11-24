use super::PartPose;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// The number of body segments in the endermite body layer.
pub(in crate::entity_models) const ENDERMITE_SEGMENT_COUNT: usize = 4;

/// Vanilla `EndermiteModel.createBodyLayer` body-segment child names, in order `segment0..segment3`.
/// `child_mut` needs `&'static` names, so the procedural body draws its names from this const array.
const ENDERMITE_SEGMENT_NAMES: [&str; ENDERMITE_SEGMENT_COUNT] =
    ["segment0", "segment1", "segment2", "segment3"];

/// Vanilla `EndermiteModel.setupAnim` segment wiggle for segment `index`, driven purely by
/// `ageInTicks` (`super.setupAnim` first resets every part to its rest pose). With `phase =
/// ageInTicks * 0.9 + index * 0.15 * π` and `dist = |index - 2|`, vanilla *sets*
/// `segment.yRot = cos(phase) * π * 0.01 * (1 + dist)` and `segment.x = sin(phase) * π *
/// 0.1 * dist`. The middle segment (`index == 2`, `dist == 0`) never shifts in `x` and only
/// wags `yRot` by the smallest amount; the head and tail (`dist == 2`) swing the most. Only
/// `offset.x` and `rotation.yRot` change; the rest `offset.y`/`offset.z` and the zero
/// `xRot`/`zRot` are preserved. Because `ageInTicks` advances every frame and the rest phase
/// already carries nonzero `cos`/`sin` terms, the endermite never sits at its layer pose.
pub(in crate::entity_models) fn endermite_segment_pose(
    base: PartPose,
    index: usize,
    age_in_ticks: f32,
) -> PartPose {
    use std::f32::consts::PI;
    let phase = age_in_ticks * 0.9 + index as f32 * 0.15 * PI;
    let dist = (index as i32 - 2).abs() as f32;
    let y_rot = phase.cos() * PI * 0.01 * (1.0 + dist);
    let x = phase.sin() * PI * 0.1 * dist;
    PartPose {
        offset: [x, base.offset[1], base.offset[2]],
        rotation: [base.rotation[0], y_rot, base.rotation[2]],
    }
}

/// Vanilla `EndermiteModel`.
pub(in crate::entity_models) fn endermite_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::ENDERMITE.bake();
    let age_in_ticks = instance.render_state.age_in_ticks;
    for (index, &name) in ENDERMITE_SEGMENT_NAMES.iter().enumerate() {
        let segment = root.child_mut(name);
        segment.pose = endermite_segment_pose(segment.pose, index, age_in_ticks);
    }
    root
}
