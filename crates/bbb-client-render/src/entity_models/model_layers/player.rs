use super::{
    apply_crossbow_charge_pose_for_hand, apply_crossbow_hold_pose,
    apply_crossbow_hold_pose_for_hand, apply_humanoid_arm_bob_named,
    apply_humanoid_attack_animation, apply_humanoid_block_pose, apply_humanoid_bow_pose,
    apply_humanoid_brush_pose, apply_humanoid_crouch_named, apply_humanoid_head_look,
    apply_humanoid_item_hold_pose, apply_humanoid_limb_walk,
    apply_humanoid_no_swing_attack_animation, apply_humanoid_spear_arm_pose,
    apply_humanoid_spear_use_pose, apply_humanoid_spyglass_pose,
    apply_humanoid_stab_attack_animation, apply_humanoid_throw_trident_pose,
    apply_humanoid_toot_horn_pose, humanoid_crouch_head_pose, humanoid_rot_lerp_rad,
};
use crate::entity_models::catalog::PlayerModelPartVisibility;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

fn player_arm_has_spear_pose(instance: &EntityModelInstance, left_arm: bool) -> bool {
    let state = &instance.render_state;
    let main_arm_left = state.main_arm_left;
    (state.kind.player().main_hand_spear_pose && main_arm_left == left_arm)
        || (state.kind.player().off_hand_spear_pose && main_arm_left != left_arm)
        || (state.kind.player().using_spear.is_some()
            && if state.use_item_off_hand {
                main_arm_left != left_arm
            } else {
                main_arm_left == left_arm
            })
}

/// Vanilla `PlayerModel.setupAnim` swim cycle, including its asymmetric
/// `rotLerpRad`/ordinary-lerp arm interpolation and always-active leg kick.
fn apply_player_swim_pose(root: &mut ModelPart, instance: &EntityModelInstance) {
    let state = &instance.render_state;
    let swim_amount = state.swim_amount;
    if swim_amount <= 0.0 {
        return;
    }

    let attack_arm_left = if state.attack_arm_off_hand {
        !state.main_arm_left
    } else {
        state.main_arm_left
    };
    let right_amount = if !player_arm_has_spear_pose(instance, false)
        && (attack_arm_left || state.attack_anim <= 0.0)
    {
        swim_amount
    } else {
        0.0
    };
    let left_amount = if !player_arm_has_spear_pose(instance, true)
        && (!attack_arm_left || state.attack_anim <= 0.0)
    {
        swim_amount
    } else {
        0.0
    };
    let animation_position = state.walk_animation_pos % 26.0;

    if !state.is_using_item {
        if animation_position < 14.0 {
            let arm_update = -65.0 * animation_position + animation_position * animation_position;
            let left = root.child_mut("left_arm");
            left.pose.rotation[0] = humanoid_rot_lerp_rad(left_amount, left.pose.rotation[0], 0.0);
            left.pose.rotation[1] =
                humanoid_rot_lerp_rad(left_amount, left.pose.rotation[1], std::f32::consts::PI);
            left.pose.rotation[2] = humanoid_rot_lerp_rad(
                left_amount,
                left.pose.rotation[2],
                std::f32::consts::PI + 1.870_796_4 * arm_update / (-65.0 * 14.0 + 14.0 * 14.0),
            );
            let right = root.child_mut("right_arm");
            right.pose.rotation[0] += right_amount * (0.0 - right.pose.rotation[0]);
            right.pose.rotation[1] +=
                right_amount * (std::f32::consts::PI - right.pose.rotation[1]);
            right.pose.rotation[2] += right_amount
                * (std::f32::consts::PI
                    - 1.870_796_4 * arm_update / (-65.0 * 14.0 + 14.0 * 14.0)
                    - right.pose.rotation[2]);
        } else if animation_position < 22.0 {
            let phase = (animation_position - 14.0) / 8.0;
            let left = root.child_mut("left_arm");
            left.pose.rotation[0] = humanoid_rot_lerp_rad(
                left_amount,
                left.pose.rotation[0],
                std::f32::consts::FRAC_PI_2 * phase,
            );
            left.pose.rotation[1] =
                humanoid_rot_lerp_rad(left_amount, left.pose.rotation[1], std::f32::consts::PI);
            left.pose.rotation[2] = humanoid_rot_lerp_rad(
                left_amount,
                left.pose.rotation[2],
                5.012_389 - 1.870_796_4 * phase,
            );
            let right = root.child_mut("right_arm");
            right.pose.rotation[0] +=
                right_amount * (std::f32::consts::FRAC_PI_2 * phase - right.pose.rotation[0]);
            right.pose.rotation[1] +=
                right_amount * (std::f32::consts::PI - right.pose.rotation[1]);
            right.pose.rotation[2] +=
                right_amount * (1.270_796_3 + 1.870_796_4 * phase - right.pose.rotation[2]);
        } else if animation_position < 26.0 {
            let phase = (animation_position - 22.0) / 4.0;
            let target_x = std::f32::consts::FRAC_PI_2 * (1.0 - phase);
            let left = root.child_mut("left_arm");
            left.pose.rotation[0] =
                humanoid_rot_lerp_rad(left_amount, left.pose.rotation[0], target_x);
            left.pose.rotation[1] =
                humanoid_rot_lerp_rad(left_amount, left.pose.rotation[1], std::f32::consts::PI);
            left.pose.rotation[2] =
                humanoid_rot_lerp_rad(left_amount, left.pose.rotation[2], std::f32::consts::PI);
            let right = root.child_mut("right_arm");
            right.pose.rotation[0] += right_amount * (target_x - right.pose.rotation[0]);
            right.pose.rotation[1] +=
                right_amount * (std::f32::consts::PI - right.pose.rotation[1]);
            right.pose.rotation[2] +=
                right_amount * (std::f32::consts::PI - right.pose.rotation[2]);
        }
    }

    let left_leg_target = 0.3 * (animation_position * 0.333_333_34 + std::f32::consts::PI).cos();
    let left_leg = root.child_mut("left_leg");
    left_leg.pose.rotation[0] += swim_amount * (left_leg_target - left_leg.pose.rotation[0]);
    let right_leg_target = 0.3 * (animation_position * 0.333_333_34).cos();
    let right_leg = root.child_mut("right_leg");
    right_leg.pose.rotation[0] += swim_amount * (right_leg_target - right_leg.pose.rotation[0]);
}

/// Exact `PlayerModel.bodyParts` order used by vanilla `getRandomBodyPart`.
pub(in crate::entity_models) const PLAYER_STUCK_BODY_PARTS: [&str; 6] = [
    "head",
    "body",
    "left_arm",
    "right_arm",
    "left_leg",
    "right_leg",
];

pub(in crate::entity_models) fn player_model_layer(slim: bool) -> ModelLayer {
    if slim {
        ModelLayer::PLAYER_SLIM
    } else {
        ModelLayer::PLAYER
    }
}

/// Toggles the six skin overlays after the player's pose has been applied.
pub(in crate::entity_models) fn apply_player_part_visibility(
    root: &mut ModelPart,
    parts: PlayerModelPartVisibility,
) {
    let overlays = [
        ("head", "hat", parts.hat),
        ("body", "jacket", parts.jacket),
        ("right_arm", "right_sleeve", parts.right_sleeve),
        ("left_arm", "left_sleeve", parts.left_sleeve),
        ("right_leg", "right_pants", parts.right_pants),
        ("left_leg", "left_pants", parts.left_pants),
    ];
    for (base, overlay, visible) in overlays {
        root.child_mut(base).child_mut(overlay).visible = visible;
    }
}

/// Player pose in inherited order: head look, limb swing, arm poses, attack,
/// crouch, idle arm bob, then the three-phase swim stroke.
pub(in crate::entity_models) fn player_model(
    slim: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = player_model_layer(slim).bake();
    let render_state = &instance.render_state;
    apply_humanoid_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
        render_state.is_fall_flying,
        render_state.swim_amount,
    );
    let head_yaw_degrees = root.child_mut("head").pose.rotation[1].to_degrees();
    let head_pitch_degrees = root.child_mut("head").pose.rotation[0].to_degrees();
    apply_humanoid_limb_walk(
        &mut root,
        render_state.walk_animation_pos,
        render_state.walk_animation_speed,
        render_state.humanoid_speed_value,
    );
    // Vanilla `HumanoidModel.setupAnim` poses the using arm (here SPYGLASS) BEFORE the crouch block,
    // so the crouch `arm.xRot += 0.4` still lands on top of the raised arm.
    if render_state.kind.player().using_spyglass {
        apply_humanoid_spyglass_pose(
            &mut root,
            head_yaw_degrees,
            head_pitch_degrees,
            render_state.use_item_off_hand,
            render_state.is_crouching,
        );
    }
    if render_state.kind.player().tooting_horn {
        apply_humanoid_toot_horn_pose(
            &mut root,
            head_yaw_degrees,
            head_pitch_degrees,
            render_state.use_item_off_hand,
        );
    }
    if render_state.kind.player().brushing {
        apply_humanoid_brush_pose(&mut root, render_state.use_item_off_hand);
    }
    if render_state.kind.player().blocking {
        apply_humanoid_block_pose(
            &mut root,
            head_yaw_degrees,
            head_pitch_degrees,
            render_state.use_item_off_hand,
        );
    }
    if render_state.kind.player().throwing_trident {
        apply_humanoid_throw_trident_pose(&mut root, render_state.use_item_off_hand);
    }
    // Vanilla `HumanoidModel.poseRightArm` / `poseLeftArm` `BOW_AND_ARROW`: drawing a bow raises both
    // arms along the head look, with the brace offset mirrored for off-hand use.
    if render_state.kind.player().drawing_bow {
        apply_humanoid_bow_pose(
            &mut root,
            head_yaw_degrees,
            head_pitch_degrees,
            render_state.use_item_off_hand,
        );
    }
    // Vanilla `HumanoidModel.poseRightArm` / `poseLeftArm` `CROSSBOW_CHARGE`
    // (`AnimationUtils.animateCrossbowCharge`): drawing a crossbow braces the holding arm and pulls the
    // string back with the opposite arm over the draw ticks.
    if render_state.kind.player().charging_crossbow {
        apply_crossbow_charge_pose_for_hand(
            &mut root,
            render_state.crossbow_charge_duration_ticks,
            render_state.crossbow_charge_ticks,
            render_state.use_item_off_hand,
        );
    }
    if let Some(kinetic_weapon) = render_state.kind.player().using_spear {
        apply_humanoid_spear_use_pose(
            &mut root,
            head_yaw_degrees,
            head_pitch_degrees,
            render_state.use_item_off_hand,
            render_state.swim_amount,
            render_state.crossbow_charge_ticks,
            kinetic_weapon,
        );
    }
    // Vanilla `AvatarRenderer.getArmPose` `SPEAR` for a held (non-using) spear uses the same
    // `thirdPersonHandUse` base pose as the use-item path, but with `ticksUsingItem <= 0`, so no kinetic
    // sway/raise branch runs. The native projection has already applied vanilla's pose dispatch
    // suppression (`ArmPose.SPEAR.affectsOffhandPose`) before setting these booleans.
    if render_state.kind.player().off_hand_spear_pose {
        apply_humanoid_spear_arm_pose(
            &mut root,
            head_yaw_degrees,
            head_pitch_degrees,
            true,
            render_state.swim_amount,
        );
    }
    if render_state.kind.player().main_hand_spear_pose {
        apply_humanoid_spear_arm_pose(
            &mut root,
            head_yaw_degrees,
            head_pitch_degrees,
            false,
            render_state.swim_amount,
        );
    }
    // Vanilla `AvatarRenderer.getArmPose` fallback `ITEM`: a player holding a plain main-hand item lowers
    // and halves the arm. It runs as part of `poseRightArm` (before crouch/attack) and only on the main
    // (right) arm, so this is unconditionally the right arm.
    if render_state.kind.player().main_hand_item_pose {
        apply_humanoid_item_hold_pose(&mut root, false);
    }
    // Vanilla `AvatarRenderer.getArmPose(_, OFF_HAND)` fallback `ITEM`, posed by `poseLeftArm` onto the
    // OFF (left) arm. Independent of the main-hand pose (separate arm), so order between them is moot.
    if render_state.kind.player().off_hand_item_pose {
        apply_humanoid_item_hold_pose(&mut root, true);
    }
    // Vanilla `AvatarRenderer.getArmPose` `CROSSBOW_HOLD` (`animateCrossbowHold`): a charged main-hand
    // crossbow levels along the head look, setting BOTH arms. It is two-handed + affectsOffhandPose and
    // vanilla runs `poseRightArm` last for this case, overwriting any off-hand ITEM, so it is applied
    // after the ITEM blocks (and before crouch/attack).
    if render_state.kind.player().crossbow_hold {
        apply_crossbow_hold_pose(&mut root, head_yaw_degrees, head_pitch_degrees);
    }
    // Vanilla's off-hand `CROSSBOW_HOLD` uses `poseLeftArm`, applying the mirrored
    // `AnimationUtils.animateCrossbowHold(..., holdingInRightArm = false)` after the main hand's
    // non-affecting pose.
    if render_state.kind.player().crossbow_hold_off_hand {
        apply_crossbow_hold_pose_for_hand(&mut root, head_yaw_degrees, head_pitch_degrees, true);
    }
    // Vanilla inherited order is arm poses, attack, crouch, idle bob, then
    // the player swim cycle. The attack type selects STAB / NONE / WHACK.
    if render_state.main_hand_swing_is_stab {
        apply_humanoid_stab_attack_animation(
            &mut root,
            render_state.attack_anim,
            render_state.attack_arm_off_hand,
            render_state.age_scale,
        );
    } else if render_state.main_hand_swing_is_none {
        apply_humanoid_no_swing_attack_animation(
            &mut root,
            render_state.attack_anim,
            render_state.attack_arm_off_hand,
            render_state.age_scale,
        );
    } else {
        apply_humanoid_attack_animation(
            &mut root,
            render_state.attack_anim,
            render_state.attack_arm_off_hand,
            head_pitch_degrees,
            render_state.age_scale,
        );
    }
    if render_state.is_crouching {
        apply_humanoid_crouch_named(&mut root);
    }
    let spyglass_right =
        render_state.kind.player().using_spyglass && !render_state.use_item_off_hand;
    let spyglass_left = render_state.kind.player().using_spyglass && render_state.use_item_off_hand;
    apply_humanoid_arm_bob_named(
        &mut root,
        render_state.age_in_ticks,
        !spyglass_right,
        !spyglass_left,
    );
    apply_player_swim_pose(&mut root, instance);
    root
}

/// Vanilla `PlayerEarsModel.createEarsLayer`: a cleared player mesh with only the
/// head part and the two inflated `left_ear` / `right_ear` children.
pub(in crate::entity_models) fn player_ears_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::PLAYER_EARS.bake();
    let render_state = &instance.render_state;
    let head = root.child_mut("head");
    apply_humanoid_head_look(
        head,
        render_state.head_yaw,
        render_state.head_pitch,
        render_state.is_fall_flying,
        render_state.swim_amount,
    );
    if render_state.is_crouching {
        head.pose = humanoid_crouch_head_pose(head.pose);
    }
    root
}
