use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::animate;

use crate::entity_models::model_data_26_1::bat_animation::{BAT_FLYING, BAT_RESTING};
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `BatModel`.
pub(in crate::entity_models) fn bat_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::BAT.bake();
    let state = &instance.render_state;
    if state.kind.bat().resting {
        root.child_mut("head").pose.rotation[1] = state.head_yaw.to_radians();
        animate(&mut root, &BAT_RESTING, 0.0, 1.0);
    } else if state.kind.bat().flying_seconds >= 0.0 {
        animate(&mut root, &BAT_FLYING, state.kind.bat().flying_seconds, 1.0);
    }
    root
}
