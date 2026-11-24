use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

use crate::entity_models::instances::EntityModelInstance;

/// Vanilla `CodModel.setupAnim`: `tailFin.yRot = -amplitude * 0.45 * sin(0.6 * ageInTicks)`, with
/// `amplitude = isInWater ? 1.0 : 1.5` (a beached cod thrashes harder). The rest pose has
/// `yRot = 0`, so this is set absolutely.
pub(in crate::entity_models) fn cod_tail_fin_yrot(age_in_ticks: f32, in_water: bool) -> f32 {
    let amplitude = if in_water { 1.0 } else { 1.5 };
    -amplitude * 0.45 * (0.6 * age_in_ticks).sin()
}

/// Vanilla `CodModel`.
pub(in crate::entity_models) fn cod_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::COD.bake();
    // Vanilla `CodModel.setupAnim` sways only the tail fin (`yRot`).
    let tail_yrot = cod_tail_fin_yrot(
        instance.render_state.age_in_ticks,
        instance.render_state.in_water,
    );
    root.child_mut("tail_fin").pose.rotation[1] = tail_yrot;
    root
}
