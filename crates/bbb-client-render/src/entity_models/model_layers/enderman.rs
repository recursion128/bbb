use super::{
    apply_head_look, enderman_arm_swing_pose, enderman_carried_arm_pose, enderman_leg_swing_pose,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Mutable enderman model, mirroring vanilla `EndermanModel extends HumanoidModel`. The unified tree
/// is built once with the vanilla HumanoidModel child names. `setup_anim` looks the head, then
/// applies the inherited arm/leg swing plus the arms' always-on idle bob, halved and clamped to
/// `[-0.4, 0.4]` on `xRot` ([`enderman_arm_swing_pose`] on the two arms, [`enderman_leg_swing_pose`]
/// on the two legs); the bob's `zRot` survives the clamp so the arms gently splay.
/// Carrying a block overrides both arms ([`enderman_carried_arm_pose`]); the creepy stare drops the
/// head `y -= 5` and raises its `hat` child `y += 5` (vanilla's `isCreepy` branch), so the outer head
/// layer holds its world position as the inner head opens downward. Both the base and eyes textured
/// passes read this one posed tree.
pub(in crate::entity_models) fn enderman_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::ENDERMAN.bake();
    let render_state = &instance.render_state;
    apply_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
    );
    let limb_swing = render_state.walk_animation_pos;
    let limb_swing_amount = render_state.walk_animation_speed;
    for name in ["right_arm", "left_arm"] {
        let arm = root.child_mut(name);
        arm.pose = enderman_arm_swing_pose(
            arm.pose,
            limb_swing,
            limb_swing_amount,
            render_state.age_in_ticks,
        );
    }
    for name in ["right_leg", "left_leg"] {
        let leg = root.child_mut(name);
        leg.pose = enderman_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
    }
    // Carrying a block overrides the arm swing entirely (held out front).
    if render_state.kind.enderman().carrying {
        for name in ["right_arm", "left_arm"] {
            let arm = root.child_mut(name);
            arm.pose = enderman_carried_arm_pose(arm.pose);
        }
    }
    // The creepy stare drops the head and raises its hat child to keep the outer layer in place.
    if render_state.kind.enderman().creepy {
        let head = root.child_mut("head");
        head.pose.offset[1] -= 5.0;
        let hat = head.child_mut("hat");
        hat.pose.offset[1] += 5.0;
    }
    root
}
