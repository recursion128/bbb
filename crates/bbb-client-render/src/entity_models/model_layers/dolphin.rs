use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `DolphinModel.setupAnim` swim wave term: `cos(ageInTicks · 0.3)`.
pub(in crate::entity_models) fn dolphin_wave(age_in_ticks: f32) -> f32 {
    (age_in_ticks * 0.3).cos()
}

/// Vanilla `DolphinModel.setupAnim`.
fn apply_dolphin_anim(root: &mut ModelPart, instance: &EntityModelInstance) {
    let state = &instance.render_state;
    let wave = dolphin_wave(state.age_in_ticks);
    let body = root.child_mut("body");
    body.pose.rotation[0] = state.head_pitch.to_radians();
    body.pose.rotation[1] = state.head_yaw.to_radians();
    if state.kind.dolphin().is_moving {
        body.pose.rotation[0] += -0.05 - 0.05 * wave;
        let tail = body.child_mut("tail");
        tail.pose.rotation[0] = -0.1 * wave;
        tail.child_mut("tail_fin").pose.rotation[0] = -0.2 * wave;
    }
}

/// Vanilla `DolphinModel` / `BabyDolphinModel`.
pub(in crate::entity_models) fn dolphin_model(
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let layer = if baby {
        ModelLayer::DOLPHIN_BABY
    } else {
        ModelLayer::DOLPHIN
    };
    let mut root = layer.bake();
    apply_dolphin_anim(&mut root, instance);
    root
}
