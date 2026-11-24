use super::apply_head_look;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::animate_walk;

use crate::entity_models::model_data_26_1::nautilus_animation;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

const NAUTILUS_LOOK_CLAMP_DEGREES: f32 = 10.0;

/// Vanilla `NautilusModel`, also used for its saddle, armor and zombie coral layers.
pub(in crate::entity_models) fn nautilus_model(
    layer: ModelLayer,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = layer.bake();
    let has_corals = layer == ModelLayer::ZOMBIE_NAUTILUS_CORAL;
    let render_state = &instance.render_state;
    apply_head_look(
        root.part_mut("body").unwrap(),
        render_state
            .head_yaw
            .clamp(-NAUTILUS_LOOK_CLAMP_DEGREES, NAUTILUS_LOOK_CLAMP_DEGREES),
        render_state
            .head_pitch
            .clamp(-NAUTILUS_LOOK_CLAMP_DEGREES, NAUTILUS_LOOK_CLAMP_DEGREES),
    );
    animate_walk(
        &mut root,
        &nautilus_animation::SWIMMING,
        render_state.walk_animation_pos + render_state.age_in_ticks / 5.0,
        render_state.walk_animation_speed + 0.2,
        2.0,
        3.0,
    );
    if has_corals {
        root.part_mut("corals").unwrap().visible = !render_state.kind.nautilus().body_item_present;
    }
    root
}
