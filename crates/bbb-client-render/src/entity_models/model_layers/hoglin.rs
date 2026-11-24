use super::{head_look_yaw_pose, hoglin_ear_sway_pose, hoglin_leg_swing_pose};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

pub(in crate::entity_models) const HOGLIN_HEAD_X_ROT: f32 = 0.87266463;

/// The four leg child names, shared by both layers. [`hoglin_leg_swing_pose`] resolves each leg's
/// phase from its bind offset, so the differing front/hind order of the adult and baby layers does
/// not matter — the names only need to be distinct and render in the layer's order.
const HOGLIN_LEG_NAMES: [&str; 4] = [
    "right_front_leg",
    "left_front_leg",
    "right_hind_leg",
    "left_hind_leg",
];

/// Vanilla `HoglinModel`.
pub(in crate::entity_models) fn hoglin_model(
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = if baby {
        ModelLayer::HOGLIN_BABY
    } else {
        ModelLayer::HOGLIN
    }
    .bake();
    let render_state = &instance.render_state;
    let limb_swing = render_state.walk_animation_pos;
    let limb_swing_amount = render_state.walk_animation_speed;
    // Yaw-only head look, the headbutt head ram (always applied — at rest it re-sets the baked down
    // tilt), then sway the two ears (head children). Vanilla overrides the baked ear rest angle to
    // `±2π/9` every frame, so the sway is applied unconditionally (a no-op for the adult layer at
    // rest, an override for the baby layer's wider baked angle).
    let head = root.child_mut("head");
    head.pose = head_look_yaw_pose(head.pose, render_state.head_yaw);
    apply_hoglin_headbutt(head, render_state.kind.hoglin().attack_animation_tick, baby);
    let right_ear = head.child_mut("right_ear");
    right_ear.pose = hoglin_ear_sway_pose(right_ear.pose, false, limb_swing, limb_swing_amount);
    let left_ear = head.child_mut("left_ear");
    left_ear.pose = hoglin_ear_sway_pose(left_ear.pose, true, limb_swing, limb_swing_amount);
    for name in HOGLIN_LEG_NAMES {
        let leg = root.child_mut(name);
        leg.pose = hoglin_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
    }
    root
}

/// Vanilla `HoglinModel.animateHeadbutt` (and `BabyHoglinModel`'s override): the head SET to
/// `lerp(headbuttLerpFactor, 0.87266463, -π/9)`, where `headbuttLerpFactor = 1 - |10 - 2·tick| / 10`
/// ramps `0 → 1 → 0` across the 10-tick attack (peaking at `tick = 5`). At rest (`tick = 0`) the factor
/// is `0`, so the head holds its baked down-tilt `0.87266463`; mid-ram it rises to `-π/9`. The baby
/// additionally lifts `head.y += factor·2.5`. Vanilla SETs the head pitch absolutely, so this supersedes
/// the baked rest tilt and the yaw-only look's preserved `xRot`.
fn apply_hoglin_headbutt(head: &mut ModelPart, attack_animation_tick: i32, baby: bool) {
    use std::f32::consts::PI;
    let factor = 1.0 - (10 - 2 * attack_animation_tick).abs() as f32 / 10.0;
    head.pose.rotation[0] = HOGLIN_HEAD_X_ROT + factor * (-PI / 9.0 - HOGLIN_HEAD_X_ROT);
    if baby {
        head.pose.offset[1] += factor * 2.5;
    }
}
