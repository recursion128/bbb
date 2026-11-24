use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::{animate, animate_walk};

use crate::entity_models::model_data_26_1::creaking_animation;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `CreakingModel`.
pub(in crate::entity_models) fn creaking_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::CREAKING.bake();
    let state = &instance.render_state;
    let head = root.part_mut("head").unwrap();
    head.pose.rotation[0] = state.head_pitch.to_radians();
    head.pose.rotation[1] = state.head_yaw.to_radians();
    if state.kind.creaking().can_move {
        animate_walk(
            &mut root,
            &creaking_animation::CREAKING_WALK,
            state.walk_animation_pos,
            state.walk_animation_speed,
            1.0,
            1.0,
        );
    }
    for (definition, seconds) in [
        (
            &creaking_animation::CREAKING_ATTACK,
            state.kind.creaking().attack_seconds,
        ),
        (
            &creaking_animation::CREAKING_INVULNERABLE,
            state.kind.creaking().invulnerable_seconds,
        ),
        (
            &creaking_animation::CREAKING_DEATH,
            state.kind.creaking().death_seconds,
        ),
    ] {
        if seconds >= 0.0 {
            animate(&mut root, definition, seconds, 1.0);
        }
    }
    root
}
