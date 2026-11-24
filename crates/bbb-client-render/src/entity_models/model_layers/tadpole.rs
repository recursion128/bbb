use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `TadpoleModel.setupAnim`: `tail.yRot = -amplitude * 0.25 * sin(0.3 * ageInTicks)`, with
/// `amplitude = isInWater ? 1.0 : 1.5` (a beached tadpole thrashes harder). The rest pose has
/// `yRot = 0`, so this is set absolutely. Mirrors [`super::cod_tail_fin_yrot`] with the tadpole's
/// own `0.25` / `0.3` constants.
pub(in crate::entity_models) fn tadpole_tail_yrot(age_in_ticks: f32, in_water: bool) -> f32 {
    let amplitude = if in_water { 1.0 } else { 1.5 };
    -amplitude * 0.25 * (0.3 * age_in_ticks).sin()
}

/// Vanilla `TadpoleModel`.
pub(in crate::entity_models) fn tadpole_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::TADPOLE.bake();
    // Vanilla `TadpoleModel.setupAnim`: the tail fin's yaw sways ([`tadpole_tail_yrot`]); the
    // body box holds still. The sway is an absolute set that collapses to the bind `yRot = 0` at
    // `ageInTicks = 0`, so it applies every frame.
    root.child_mut("tail").pose.rotation[1] = tadpole_tail_yrot(
        instance.render_state.age_in_ticks,
        instance.render_state.in_water,
    );
    root
}
