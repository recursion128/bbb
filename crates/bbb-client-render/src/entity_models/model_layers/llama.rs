use super::{apply_head_look, apply_quadruped_leg_swing};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

// Vanilla 26.1 `ModelLayers.LLAMA` / `LLAMA_BABY` (`LlamaRenderer`). The trader llama bakes the same
// `LlamaModel.createBodyLayer` mesh; its distinction is the `LlamaDecorLayer` overlay.

/// Vanilla `LlamaModel` / `BabyLlamaModel`.
pub(in crate::entity_models) fn llama_model(
    layer: ModelLayer,
    has_chest: bool,
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
    root.child_mut("right_chest").visible = has_chest;
    root.child_mut("left_chest").visible = has_chest;
    root
}
