use super::apply_head_look;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::animate;

use crate::entity_models::model_data_26_1::{baby_rabbit_animation, rabbit_animation};
use crate::entity_models::model_layer::BakedRoot;

/// Vanilla `AdultRabbitModel` / `BabyRabbitModel`.
pub(in crate::entity_models) fn rabbit_model(
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = crate::entity_models::textured::rabbit_model_layer(baby).bake();
    // bbb never starts the idle-head-tilt AnimationState, so the look always applies.
    let state = &instance.render_state;
    apply_head_look(
        root.child_mut("body").child_mut("head"),
        state.head_yaw,
        state.head_pitch,
    );
    if state.kind.rabbit().hop_seconds >= 0.0 {
        let hop = if baby {
            &baby_rabbit_animation::HOP
        } else {
            &rabbit_animation::HOP
        };
        animate(&mut root, hop, state.kind.rabbit().hop_seconds, 1.0);
    }
    root
}
