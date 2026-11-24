use super::{apply_head_look, apply_quadruped_leg_swing};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `CreeperModel`.
pub(in crate::entity_models) fn creeper_model(
    layer: ModelLayer,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = layer.bake();
    let render_state = &instance.render_state;
    apply_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
    );
    apply_quadruped_leg_swing(
        &mut root,
        render_state.walk_animation_pos,
        render_state.walk_animation_speed,
    );
    root
}
