use super::apply_head_look;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::{animate, animate_walk};
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_data_26_1::copper_golem_animation;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

fn pose_held_item_arms_if_still(root: &mut ModelPart) {
    // Vanilla `CopperGolemModel.poseHeldItemArmsIfStill`: clamp the arms into the resting held-item pose
    // when either rendered hand is non-empty, after the walk-with-item animation has been sampled.
    let body = root.child_mut("body");
    let right_arm = body.child_mut("right_arm");
    right_arm.pose.rotation[0] = right_arm.pose.rotation[0].min(-0.87266463);
    right_arm.pose.rotation[1] = right_arm.pose.rotation[1].min(-0.1134464);
    right_arm.pose.rotation[2] = right_arm.pose.rotation[2].min(-0.064577185);
    let left_arm = body.child_mut("left_arm");
    left_arm.pose.rotation[0] = left_arm.pose.rotation[0].min(-0.87266463);
    left_arm.pose.rotation[1] = left_arm.pose.rotation[1].max(0.1134464);
    left_arm.pose.rotation[2] = left_arm.pose.rotation[2].max(0.064577185);
}

/// Vanilla `CopperGolemModel`, also drawn for the emissive eyes layer.
pub(in crate::entity_models) fn copper_golem_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::COPPER_GOLEM.bake();
    let state = &instance.render_state;
    apply_head_look(
        root.part_mut("head").unwrap(),
        state.head_yaw,
        state.head_pitch,
    );
    let walk = if state.kind.copper_golem().holding_item {
        &copper_golem_animation::COPPER_GOLEM_WALK_ITEM
    } else {
        &copper_golem_animation::COPPER_GOLEM_WALK
    };
    animate_walk(
        &mut root,
        walk,
        state.walk_animation_pos,
        state.walk_animation_speed,
        2.0,
        2.5,
    );
    if state.kind.copper_golem().holding_item {
        pose_held_item_arms_if_still(&mut root);
    }
    for (definition, seconds) in [
        (
            &copper_golem_animation::COPPER_GOLEM_IDLE,
            state.kind.copper_golem().idle_seconds,
        ),
        (
            &copper_golem_animation::COPPER_GOLEM_CHEST_INTERACTION_NOITEM_GET,
            state.kind.copper_golem().get_item_seconds,
        ),
        (
            &copper_golem_animation::COPPER_GOLEM_CHEST_INTERACTION_NOITEM_NOGET,
            state.kind.copper_golem().get_no_item_seconds,
        ),
        (
            &copper_golem_animation::COPPER_GOLEM_CHEST_INTERACTION_ITEM_DROP,
            state.kind.copper_golem().drop_item_seconds,
        ),
        (
            &copper_golem_animation::COPPER_GOLEM_CHEST_INTERACTION_ITEM_NODROP,
            state.kind.copper_golem().drop_no_item_seconds,
        ),
    ] {
        if seconds >= 0.0 {
            animate(&mut root, definition, seconds, 1.0);
        }
    }
    root
}
