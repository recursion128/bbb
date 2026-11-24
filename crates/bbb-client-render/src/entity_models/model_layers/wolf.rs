use super::{
    apply_head_look, apply_wolf_sitting_pose, quadruped_leg_swing_pose, wolf_angry_tail_pose,
    wolf_sitting_part_roles, wolf_tail_swing_pose,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// The four leg child names, in `WolfModel.createBodyLayer` order (hind pair then front pair).
/// [`quadruped_leg_swing_pose`] resolves each leg's phase from its bind offset, so the names only need
/// to be distinct and render in the layer's order.
const WOLF_LEG_NAMES: [&str; 4] = [
    "right_hind_leg",
    "left_hind_leg",
    "right_front_leg",
    "left_front_leg",
];

/// Vanilla `WolfRenderState.getBodyRollAngle(offset)`: clamp `(shakeAnim + offset) / 1.8` to `0..1`,
/// then use the paired sine wave to roll body parts while the wolf shakes water off. The head part
/// adds `WolfRenderState.headRollAngle` on top for the interested/begging tilt.
pub(in crate::entity_models) fn wolf_body_roll_angle(shake_anim: f32, offset: f32) -> f32 {
    let progress = ((shake_anim + offset) / 1.8).clamp(0.0, 1.0);
    (progress * std::f32::consts::PI).sin()
        * (progress * std::f32::consts::PI * 11.0).sin()
        * 0.15
        * std::f32::consts::PI
}

fn apply_wolf_water_shake_roll(
    root: &mut ModelPart,
    baby: bool,
    shake_anim: f32,
    head_roll_angle: f32,
) {
    root.child_mut("body").pose.rotation[2] = wolf_body_roll_angle(shake_anim, -0.16);
    if baby {
        root.child_mut("head").pose.rotation[2] =
            head_roll_angle + wolf_body_roll_angle(shake_anim, 0.0);
        root.child_mut("tail").pose.rotation[2] = wolf_body_roll_angle(shake_anim, -0.2);
    } else {
        root.child_mut("head").child_mut("real_head").pose.rotation[2] =
            head_roll_angle + wolf_body_roll_angle(shake_anim, 0.0);
        root.child_mut("upper_body").pose.rotation[2] = wolf_body_roll_angle(shake_anim, -0.08);
        root.child_mut("tail").child_mut("real_tail").pose.rotation[2] =
            wolf_body_roll_angle(shake_anim, -0.2);
    }
}

/// Vanilla `WolfModel`.
pub(in crate::entity_models) fn wolf_model(
    baby: bool,
    angry: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = if baby {
        ModelLayer::WOLF_BABY
    } else {
        ModelLayer::WOLF
    }
    .bake();
    pose_wolf(&mut root, baby, angry, instance);
    root
}

pub(in crate::entity_models) fn wolf_armor_model(
    angry: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = ModelLayer::WOLF_ARMOR.bake();
    let baby = false;
    pose_wolf(&mut root, baby, angry, instance);
    root
}

fn pose_wolf(root: &mut ModelPart, baby: bool, angry: bool, instance: &EntityModelInstance) {
    let render_state = &instance.render_state;

    // The head carries the ears, so the look turns the whole head subtree.
    apply_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
    );

    // A sitting wolf folds (`setSittingPose`) instead of swinging its legs; the tail rotation below
    // then layers onto the sitting offset lift (both helpers preserve the offset). The baby's
    // `setSittingPose` adds a further body tilt.
    if render_state.kind.wolf().sitting {
        for (name, role) in wolf_sitting_part_roles() {
            apply_wolf_sitting_pose(
                &mut root.child_mut(name).pose,
                role,
                baby,
                render_state.age_scale,
            );
        }
    } else {
        for name in WOLF_LEG_NAMES {
            let leg = root.child_mut(name);
            leg.pose = quadruped_leg_swing_pose(
                leg.pose,
                render_state.walk_animation_pos,
                render_state.walk_animation_speed,
            );
        }
    }

    // The tail `xRot` is SET to `tailAngle` (wag `yRot` on top) in both poses; an angry wolf holds
    // the raised constant with no wag.
    let tail = root.child_mut("tail");
    tail.pose = if angry {
        wolf_angry_tail_pose(tail.pose)
    } else {
        wolf_tail_swing_pose(
            tail.pose,
            render_state.kind.wolf().tail_angle,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
        )
    };

    apply_wolf_water_shake_roll(
        root,
        baby,
        render_state.kind.wolf().shake_anim,
        render_state.kind.wolf().head_roll_angle,
    );
}
