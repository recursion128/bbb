use super::{
    apply_head_look, apply_iron_golem_arm_events, apply_iron_golem_walk, snow_golem_arm_pose,
    snow_golem_upper_body_pose, snow_golem_upper_body_yrot,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Mutable iron golem model, mirroring vanilla `IronGolemModel`. The unified tree is built with the
/// vanilla child names: `head`, `body`, `right_arm`/`left_arm`, `right_leg`/`left_leg`. `setup_anim`
/// follows the head look ([`apply_head_look`] on `head`) then swings the arms and legs
/// ([`apply_iron_golem_walk`]), then overrides the arms with the attack smash / offer-flower poses
/// ([`apply_iron_golem_arm_events`]) while their event timers are active. The held poppy follows the
/// posed flower arm through the block-attachment pass.
pub(in crate::entity_models) fn iron_golem_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::IRON_GOLEM.bake();
    let render_state = &instance.render_state;
    apply_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
    );
    apply_iron_golem_walk(
        &mut root,
        render_state.walk_animation_pos,
        render_state.walk_animation_speed,
    );
    // Vanilla `IronGolemModel.setupAnim`: the attack smash / offer-flower arm poses override the
    // walk arm swing while their event timers are active (the legs keep the walk swing).
    apply_iron_golem_arm_events(
        &mut root,
        render_state.kind.iron_golem().attack_ticks_remaining,
        render_state.kind.iron_golem().offer_flower_tick,
    );
    root
}

/// Mutable snow golem model, mirroring vanilla `SnowGolemModel`. The unified tree is built with the
/// vanilla child names: `head`, `left_arm`/`right_arm` (stick arms), `upper_body` (the middle snow
/// ball), `lower_body`. `setup_anim` looks the head ([`apply_head_look`] on `head`), twists the upper
/// body by a quarter of the head yaw ([`snow_golem_upper_body_pose`]), and orbits the two arms around
/// that twist ([`snow_golem_arm_pose`]). The arm orbit overwrites the body-layer `x`/`z` even at rest,
/// so the tree is always re-posed.
pub(in crate::entity_models) fn snow_golem_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::SNOW_GOLEM.bake();
    let render_state = &instance.render_state;
    let upper_body_yrot = snow_golem_upper_body_yrot(render_state.head_yaw);
    apply_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
    );
    let upper_body = root.child_mut("upper_body");
    upper_body.pose = snow_golem_upper_body_pose(upper_body.pose, upper_body_yrot);
    let left_arm = root.child_mut("left_arm");
    left_arm.pose = snow_golem_arm_pose(left_arm.pose, upper_body_yrot, false);
    let right_arm = root.child_mut("right_arm");
    right_arm.pose = snow_golem_arm_pose(right_arm.pose, upper_body_yrot, true);
    root
}
