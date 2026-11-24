use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `ShulkerModel.setupAnim` lid pose from the client peek amount (and `ageInTicks`
/// for the open-lid bob). With `bs = (0.5 + peek)·π`:
/// `lid.y = 16 + sin(bs)·8` (plus `sin(ageInTicks·0.1)·0.7` once `bs > π`, i.e. the lid is
/// past half-open) and `lid.yRot = (−1 + sin(bs))⁴ · π · 0.125` once `peek > 0.3` (else `0`).
/// Returns `(lid_y, lid_yrot)`. At `peek = 0` this is `(24, 0)` — the closed/bind pose.
pub(in crate::entity_models) fn shulker_lid_pose(peek: f32, age_in_ticks: f32) -> (f32, f32) {
    let bs = (0.5 + peek) * std::f32::consts::PI;
    let extra = if bs > std::f32::consts::PI {
        (age_in_ticks * 0.1).sin() * 0.7
    } else {
        0.0
    };
    let lid_y = 16.0 + bs.sin() * 8.0 + extra;
    let lid_yrot = if peek > 0.3 {
        let q = -1.0 + bs.sin();
        q * q * q * q * std::f32::consts::PI * 0.125
    } else {
        0.0
    };
    (lid_y, lid_yrot)
}

/// Vanilla `ShulkerModel`.
pub(in crate::entity_models) fn shulker_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::SHULKER.bake();
    // Vanilla `ShulkerModel.setupAnim`: the lid opens with the synced peek ([`shulker_lid_pose`])
    // and the head tracks the look angles, while the base holds still. All three are absolute
    // sets — at `peek = 0` the lid returns to its `y = 24` bind offset, and the head yaw carries
    // the vanilla `−180` cancel (so it never equals the bind yaw), so they apply every frame.
    let (lid_y, lid_yrot) = shulker_lid_pose(
        instance.render_state.kind.shulker().peek,
        instance.render_state.age_in_ticks,
    );
    let lid = root.child_mut("lid");
    lid.pose.offset[1] = lid_y;
    lid.pose.rotation[1] = lid_yrot;

    let head = root.child_mut("head");
    head.pose.rotation[0] = instance.render_state.head_pitch.to_radians();
    head.pose.rotation[1] = (instance.render_state.head_yaw - 180.0).to_radians();
    root
}
