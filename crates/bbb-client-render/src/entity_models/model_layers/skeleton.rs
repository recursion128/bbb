use super::{
    apply_humanoid_arm_bob_named, apply_humanoid_attack_animation, apply_humanoid_head_look,
    apply_humanoid_limb_walk, apply_humanoid_mob_spear_arm_poses, humanoid_arm_bob_pose,
};
use crate::entity_models::catalog::SkeletonModelFamily;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `SkeletonModel`'s bow-aim arm pose: `AbstractSkeletonRenderer.getArmPose` returns
/// `BOW_AND_ARROW` when the skeleton `isAggressive() && getMainHandItem().is(Items.BOW)`, so
/// `HumanoidModel.poseRightArm`'s `BOW_AND_ARROW` case raises both arms forward along the head look,
/// overwriting the idle walk swing. The skeleton is right-handed (bow in the right arm); the off arm is
/// splayed `0.4` outward. `head_yaw_degrees` / `head_pitch_degrees` are the same net head look fed to
/// [`apply_head_look`] (vanilla `head.yRot` / `head.xRot`, radians once converted).
fn apply_skeleton_bow_aim(root: &mut ModelPart, head_yaw_degrees: f32, head_pitch_degrees: f32) {
    let head_yaw = head_yaw_degrees.to_radians();
    let head_pitch = head_pitch_degrees.to_radians();
    let aim_pitch = -std::f32::consts::FRAC_PI_2 + head_pitch;
    let right = root.child_mut("right_arm");
    right.pose.rotation = [aim_pitch, -0.1 + head_yaw, right.pose.rotation[2]];
    let left = root.child_mut("left_arm");
    left.pose.rotation = [aim_pitch, 0.1 + head_yaw + 0.4, left.pose.rotation[2]];
}

/// Whether the skeleton renders the `BOW_AND_ARROW` aim pose this frame (vanilla
/// `AbstractSkeletonRenderer.getArmPose`).
fn skeleton_is_aiming_bow(instance: &EntityModelInstance) -> bool {
    instance.render_state.is_aggressive && instance.render_state.main_hand_holds_bow
}

/// Whether the skeleton renders its custom melee pose this frame (vanilla `SkeletonModel.setupAnim`'s
/// `state.isAggressive && !state.isHoldingBow`): an aggressive skeleton not holding a bow raises and
/// chops its arms instead of aiming.
fn skeleton_is_meleeing(instance: &EntityModelInstance) -> bool {
    instance.render_state.is_aggressive && !instance.render_state.main_hand_holds_bow
}

fn apply_skeleton_humanoid_mob_arm_poses(root: &mut ModelPart, instance: &EntityModelInstance) {
    let render_state = &instance.render_state;
    let main_suppressed = apply_humanoid_mob_spear_arm_poses(
        root,
        render_state.head_yaw,
        render_state.head_pitch,
        render_state.humanoid_mob_main_hand_spear_pose,
        render_state.humanoid_mob_off_hand_spear_pose,
        render_state.swim_amount,
    );
    if !main_suppressed && skeleton_is_aiming_bow(instance) {
        apply_skeleton_bow_aim(root, render_state.head_yaw, render_state.head_pitch);
    }
}

/// Vanilla `SkeletonModel.setupAnim`'s melee arm pose (`isAggressive && !isHoldingBow`): both arms
/// raise forward to `-π/2` and chop with the attack (`attack_anim` = `attackTime`), the right arm
/// yawing in and the left out symmetrically, then take the idle bob ([`humanoid_arm_bob_pose`]). At
/// rest (`attack_anim = 0`) the arms hold the raised melee-ready pose. The shared
/// [`apply_humanoid_attack_animation`] body twist + arm-anchor reposition run first; this overwrites
/// the arm rotations exactly as vanilla overwrites `super.setupAnim`'s.
fn apply_skeleton_melee_arms(root: &mut ModelPart, attack_anim: f32, age_in_ticks: f32) {
    use std::f32::consts::{FRAC_PI_2, PI};
    let attack2 = (attack_anim * PI).sin();
    let attack = ((1.0 - (1.0 - attack_anim) * (1.0 - attack_anim)) * PI).sin();
    let arm_x = -FRAC_PI_2 - (attack2 * 1.2 - attack * 0.4);
    let yaw = 0.1 - attack2 * 0.6;
    for (name, yaw_sign) in [("right_arm", -1.0), ("left_arm", 1.0)] {
        let arm = root.child_mut(name);
        arm.pose.rotation = [arm_x, yaw_sign * yaw, 0.0];
        arm.pose = humanoid_arm_bob_pose(arm.pose, age_in_ticks);
    }
}

/// Mutable skeleton model, mirroring vanilla `SkeletonModel` (the base `HumanoidModel`) and its
/// stray / parched / bogged / wither-skeleton variants. The unified tree is built for the selected
/// family ([`skeleton_tree`]) with the vanilla child names. `setup_anim` runs the shared
/// `HumanoidModel.setupAnim` (head look + arm/leg walk swing), the `BOW_AND_ARROW` aim pose when the
/// skeleton is aggressive and holding a bow, the shared melee-swing body twist
/// ([`apply_humanoid_attack_animation`]), and — for an aggressive skeleton not holding a bow — the custom
/// raised-and-chopping melee arm pose ([`apply_skeleton_melee_arms`]). The wither dark tint / root
/// transform and the stray / bogged clothing overlay ([`SkeletonClothingModel`]) are applied at the call
/// site.
pub(in crate::entity_models) fn skeleton_model(
    family: Option<SkeletonModelFamily>,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = crate::entity_models::textured::skeleton_model_layer(family).bake();
    let sheared = matches!(family, Some(SkeletonModelFamily::Bogged { sheared: true }));
    let render_state = &instance.render_state;
    apply_humanoid_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
        render_state.is_fall_flying,
        render_state.swim_amount,
    );
    apply_humanoid_limb_walk(
        &mut root,
        render_state.walk_animation_pos,
        render_state.walk_animation_speed,
        render_state.humanoid_speed_value,
    );
    apply_skeleton_humanoid_mob_arm_poses(&mut root, instance);
    if sheared {
        // Vanilla `BoggedModel.setupAnim`: `mushrooms.visible = !state.isSheared`.
        root.part_mut("mushrooms").unwrap().visible = false;
    }
    // Vanilla `HumanoidModel.setupAttackAnimation` (in `super.setupAnim`): the body twists for a
    // swinging skeleton (a no-op when not mid-swing).
    apply_humanoid_attack_animation(
        &mut root,
        render_state.attack_anim,
        render_state.attack_arm_off_hand,
        render_state.head_pitch,
        render_state.age_scale,
    );
    apply_humanoid_arm_bob_named(&mut root, render_state.age_in_ticks, true, true);
    // A melee (aggressive, no bow) skeleton raises and chops its arms (vanilla `SkeletonModel.setupAnim`).
    if skeleton_is_meleeing(instance) {
        apply_skeleton_melee_arms(
            &mut root,
            render_state.attack_anim,
            render_state.age_in_ticks,
        );
    }
    root
}

/// Mutable textured-only skeleton clothing overlay (the stray frost layer / bogged mushroom layer): an
/// inflated `HumanoidModel`-shaped overlay built as a named-children tree (`stray_clothing_root` /
/// `bogged_clothing_root`) and posed by the SAME shared `HumanoidModel.setupAnim` as the base body, so
/// the overlay tracks the limbs.
pub(in crate::entity_models) fn skeleton_clothing_model(
    family: Option<SkeletonModelFamily>,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let layer = match family {
        Some(SkeletonModelFamily::Bogged { .. }) => ModelLayer::BOGGED_OUTER,
        _ => ModelLayer::STRAY_OUTER,
    };
    let mut root = layer.bake();
    let render_state = &instance.render_state;
    apply_humanoid_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
        render_state.is_fall_flying,
        render_state.swim_amount,
    );
    apply_humanoid_limb_walk(
        &mut root,
        render_state.walk_animation_pos,
        render_state.walk_animation_speed,
        render_state.humanoid_speed_value,
    );
    // The clothing overlay tracks the body arms, so it aims / melees with them (vanilla poses the
    // overlay with the same `SkeletonModel.setupAnim`).
    apply_skeleton_humanoid_mob_arm_poses(&mut root, instance);
    apply_humanoid_attack_animation(
        &mut root,
        render_state.attack_anim,
        render_state.attack_arm_off_hand,
        render_state.head_pitch,
        render_state.age_scale,
    );
    apply_humanoid_arm_bob_named(&mut root, render_state.age_in_ticks, true, true);
    if skeleton_is_meleeing(instance) {
        apply_skeleton_melee_arms(
            &mut root,
            render_state.attack_anim,
            render_state.age_in_ticks,
        );
    }
    root
}
