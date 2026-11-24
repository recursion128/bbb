use super::{head_look_at_rest, head_look_pose, PartPose};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `WitherBossModel.setupAnim` breathing sway, driven entirely by the projected
/// `ageInTicks`: `anim = cos(ageInTicks * 0.1)` pitches the ribcage to
/// `(0.065 + 0.05 * anim) * PI`, re-hangs the tail from that new pitch
/// (`tail.setPos(-2, 6.9 + cos(ribcage.xRot) * 10, -0.5 + sin(ribcage.xRot) * 10)`), and pitches the
/// tail to `(0.265 + 0.1 * anim) * PI`. Returns the `(ribcage, tail)` poses; the ribcage keeps its bind offset `(-2, 6.9, -0.5)` and only its `xRot`
/// moves. Because `ageInTicks` advances every frame, the wither never sits perfectly still.
pub(in crate::entity_models) fn wither_breathing_poses(age_in_ticks: f32) -> (PartPose, PartPose) {
    use std::f32::consts::PI;
    let anim = (age_in_ticks * 0.1).cos();
    let ribcage_x_rot = (0.065 + 0.05 * anim) * PI;
    let ribcage = PartPose {
        offset: [-2.0, 6.9, -0.5],
        rotation: [ribcage_x_rot, 0.0, 0.0],
    };
    let tail = PartPose {
        offset: [
            -2.0,
            6.9 + ribcage_x_rot.cos() * 10.0,
            -0.5 + ribcage_x_rot.sin() * 10.0,
        ],
        rotation: [(0.265 + 0.1 * anim) * PI, 0.0, 0.0],
    };
    (ribcage, tail)
}

/// Vanilla `WitherBossModel`.
pub(in crate::entity_models) fn wither_model(
    layer: ModelLayer,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = layer.bake();
    // Vanilla `WitherBossModel.setupAnim`: side heads track the copied wither head rotation
    // arrays, the ribcage and tail breathe with `ageInTicks` ([`wither_breathing_poses`]), then
    // the center head tracks the ordinary look angles.
    let render_state = instance.render_state;
    let body_rot = render_state.body_rot;
    for (name, index) in [("right_head", 0_usize), ("left_head", 1_usize)] {
        let head = root.child_mut(name);
        head.pose.rotation[0] = render_state.kind.wither().x_head_rots[index].to_radians();
        head.pose.rotation[1] =
            (render_state.kind.wither().y_head_rots[index] - body_rot).to_radians();
    }

    let (ribcage_pose, tail_pose) = wither_breathing_poses(instance.render_state.age_in_ticks);
    root.child_mut("ribcage").pose = ribcage_pose;
    root.child_mut("tail").pose = tail_pose;

    let head_yaw = render_state.head_yaw;
    let head_pitch = render_state.head_pitch;
    if !head_look_at_rest(head_yaw, head_pitch) {
        let center_head = root.child_mut("center_head");
        center_head.pose = head_look_pose(center_head.pose, head_yaw, head_pitch);
    }
    root
}
