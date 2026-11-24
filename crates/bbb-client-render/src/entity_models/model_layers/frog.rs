use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::{animate, animate_walk};

use crate::entity_models::model_data_26_1::frog_animation;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `FrogModel`.
pub(in crate::entity_models) fn frog_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::FROG.bake();
    let state = &instance.render_state;
    for (definition, seconds) in [
        (&frog_animation::FROG_JUMP, state.kind.frog().jump_seconds),
        (&frog_animation::FROG_CROAK, state.kind.frog().croak_seconds),
        (
            &frog_animation::FROG_TONGUE,
            state.kind.frog().tongue_seconds,
        ),
    ] {
        if seconds >= 0.0 {
            animate(&mut root, definition, seconds, 1.0);
        }
    }
    if state.in_water {
        animate_walk(
            &mut root,
            &frog_animation::FROG_SWIM,
            state.walk_animation_pos,
            state.walk_animation_speed,
            1.0,
            2.5,
        );
    } else {
        animate_walk(
            &mut root,
            &frog_animation::FROG_WALK,
            state.walk_animation_pos,
            state.walk_animation_speed,
            1.5,
            2.5,
        );
    }
    if state.kind.frog().swim_idle_seconds >= 0.0 {
        animate(
            &mut root,
            &frog_animation::FROG_IDLE_WATER,
            state.kind.frog().swim_idle_seconds,
            1.0,
        );
    }
    root.part_mut("croaking_body").unwrap().visible = state.kind.frog().croak_seconds >= 0.0;
    root
}
