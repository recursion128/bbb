use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::animate;

use crate::entity_models::model_data_26_1::breeze_animation;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `BreezeModel`, shared by the body, wind and eyes layers (each keeps every bone).
pub(in crate::entity_models) fn breeze_model(
    layer: ModelLayer,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = layer.bake();
    let state = &instance.render_state;
    animate(
        &mut root,
        &breeze_animation::IDLE,
        state.age_in_ticks * 0.05,
        1.0,
    );
    for (definition, seconds) in [
        (&breeze_animation::SHOOT, state.kind.breeze().shoot_seconds),
        (&breeze_animation::SLIDE, state.kind.breeze().slide_seconds),
        (
            &breeze_animation::SLIDE_BACK,
            state.kind.breeze().slide_back_seconds,
        ),
        (
            &breeze_animation::INHALE,
            state.kind.breeze().inhale_seconds,
        ),
        (
            &breeze_animation::JUMP,
            state.kind.breeze().long_jump_seconds,
        ),
    ] {
        if seconds >= 0.0 {
            animate(&mut root, definition, seconds, 1.0);
        }
    }
    root
}
