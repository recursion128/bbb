use crate::entity_models::catalog::SalmonModelSize;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `SalmonModel`/`SalmonRenderer` swim multipliers: a salmon in water uses
/// `(amplitude 1.0, angle 1.0)`; a beached salmon thrashes harder and faster
/// `(1.3, 1.7)`. Shared by the body-back sway and the renderer body wiggle.
pub(in crate::entity_models) fn salmon_swim_multipliers(in_water: bool) -> (f32, f32) {
    if in_water {
        (1.0, 1.0)
    } else {
        (1.3, 1.7)
    }
}

/// Vanilla `SalmonModel.setupAnim`: `bodyBack.yRot = -amplitude * 0.25 * sin(angle *
/// 0.6 * ageInTicks)`. The rest pose has `yRot = 0`, so this is set absolutely.
pub(in crate::entity_models) fn salmon_body_back_yrot(age_in_ticks: f32, in_water: bool) -> f32 {
    let (amplitude, angle) = salmon_swim_multipliers(in_water);
    -amplitude * 0.25 * (angle * 0.6 * age_in_ticks).sin()
}

/// Vanilla `SalmonRenderer` selects the small / medium / large `SalmonModel` layer by
/// `Salmon.Variant`; the medium layer is the unscaled `ModelLayers.SALMON`.
pub(in crate::entity_models) fn salmon_model_layer(size: SalmonModelSize) -> ModelLayer {
    match size {
        SalmonModelSize::Small => ModelLayer::SALMON_SMALL,
        SalmonModelSize::Medium => ModelLayer::SALMON,
        SalmonModelSize::Large => ModelLayer::SALMON_LARGE,
    }
}

/// Vanilla `SalmonModel`.
pub(in crate::entity_models) fn salmon_model(
    size: SalmonModelSize,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = salmon_model_layer(size).bake();
    // Vanilla `SalmonModel.setupAnim`: `bodyBack.yRot = sway` (the back segment carries the tail
    // and rear top fin as children, so they swing with it).
    let body_back_yrot = salmon_body_back_yrot(
        instance.render_state.age_in_ticks,
        instance.render_state.in_water,
    );
    root.child_mut("body_back").pose.rotation[1] = body_back_yrot;
    root
}
