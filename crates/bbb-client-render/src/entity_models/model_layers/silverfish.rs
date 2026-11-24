use super::PartPose;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// The number of body segments in the silverfish body layer (parts `0..=6`); the three
/// overlay `layer` parts follow at `7..=9`.
pub(in crate::entity_models) const SILVERFISH_SEGMENT_COUNT: usize = 7;

/// The number of overlay `layer` parts in the silverfish body layer.
pub(in crate::entity_models) const SILVERFISH_LAYER_COUNT: usize = 3;

/// Vanilla `SilverfishModel.createBodyLayer` body-segment child names, in order `segment0..segment6`.
/// `child_mut` needs `&'static` names, so the procedural body draws its names from this const array.
const SILVERFISH_SEGMENT_NAMES: [&str; SILVERFISH_SEGMENT_COUNT] = [
    "segment0", "segment1", "segment2", "segment3", "segment4", "segment5", "segment6",
];

/// Vanilla `SilverfishModel.createBodyLayer` overlay `layer` child names, in order `layer0..layer2`.
const SILVERFISH_LAYER_NAMES: [&str; SILVERFISH_LAYER_COUNT] = ["layer0", "layer1", "layer2"];

/// Each overlay `layer` part copies one body segment's animation: `(source_segment,
/// copy_x)`. Vanilla sets `layer0.yRot = segment2.yRot` (x untouched), `layer1.yRot =
/// segment4.yRot` and `layer1.x = segment4.x`, `layer2.yRot = segment1.yRot` and `layer2.x =
/// segment1.x`.
pub(in crate::entity_models) const SILVERFISH_LAYER_RULES: [(usize, bool); SILVERFISH_LAYER_COUNT] =
    [(2, false), (4, true), (1, true)];

/// Vanilla `SilverfishModel.setupAnim` segment wiggle for segment `index`, driven purely by
/// `ageInTicks` (`super.setupAnim` first resets every part to its rest pose). With `phase =
/// ageInTicks * 0.9 + index * 0.15 * π` and `dist = |index - 2|`, vanilla *sets*
/// `segment.yRot = cos(phase) * π * 0.05 * (1 + dist)` and `segment.x = sin(phase) * π *
/// 0.2 * dist` — the same shape as the endermite, but with the larger `0.05`/`0.2` amplitudes of
/// the silverfish's faster scuttle. Only `offset.x` and `rotation.yRot` change; the rest
/// `offset.y`/`offset.z` and the zero `xRot`/`zRot` are preserved. `ageInTicks` advances
/// every frame and the rest phase already carries nonzero `cos`/`sin` terms, so the
/// silverfish never sits at its layer pose.
pub(in crate::entity_models) fn silverfish_segment_pose(
    base: PartPose,
    index: usize,
    age_in_ticks: f32,
) -> PartPose {
    use std::f32::consts::PI;
    let phase = age_in_ticks * 0.9 + index as f32 * 0.15 * PI;
    let dist = (index as i32 - 2).abs() as f32;
    let y_rot = phase.cos() * PI * 0.05 * (1.0 + dist);
    let x = phase.sin() * PI * 0.2 * dist;
    PartPose {
        offset: [x, base.offset[1], base.offset[2]],
        rotation: [base.rotation[0], y_rot, base.rotation[2]],
    }
}

/// Applies a [`SILVERFISH_LAYER_RULES`] copy to one overlay `layer` part: its `yRot` always
/// follows the source segment's animated `yRot`, and its `offset.x` follows the source's `x`
/// only when `copy_x` (layer0 leaves its `x` at the layer rest). The rest `offset.y`/`z` and
/// `xRot`/`zRot` are preserved.
pub(in crate::entity_models) fn silverfish_layer_pose(
    base: PartPose,
    source: PartPose,
    copy_x: bool,
) -> PartPose {
    PartPose {
        offset: [
            if copy_x {
                source.offset[0]
            } else {
                base.offset[0]
            },
            base.offset[1],
            base.offset[2],
        ],
        rotation: [base.rotation[0], source.rotation[1], base.rotation[2]],
    }
}

/// Vanilla `SilverfishModel`.
pub(in crate::entity_models) fn silverfish_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::SILVERFISH.bake();
    let age_in_ticks = instance.render_state.age_in_ticks;
    for (index, &name) in SILVERFISH_SEGMENT_NAMES.iter().enumerate() {
        let segment = root.child_mut(name);
        segment.pose = silverfish_segment_pose(segment.pose, index, age_in_ticks);
    }
    for (&layer_name, &(source, copy_x)) in SILVERFISH_LAYER_NAMES
        .iter()
        .zip(SILVERFISH_LAYER_RULES.iter())
    {
        let source_pose = root.child_mut(SILVERFISH_SEGMENT_NAMES[source]).pose;
        let part = root.child_mut(layer_name);
        part.pose = silverfish_layer_pose(part.pose, source_pose, copy_x);
    }
    root
}
