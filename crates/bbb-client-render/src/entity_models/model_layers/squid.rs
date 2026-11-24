use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

use crate::entity_models::instances::EntityModelInstance;

/// Vanilla `SquidModel.createBodyLayer` tentacle child names, in ring order `tentacle0..tentacle7`.
/// `child_mut` needs `&'static` names, so the procedural ring draws its names from this const array.
const SQUID_TENTACLE_NAMES: [&str; 8] = [
    "tentacle0",
    "tentacle1",
    "tentacle2",
    "tentacle3",
    "tentacle4",
    "tentacle5",
    "tentacle6",
    "tentacle7",
];

/// Vanilla `SquidModel`.
pub(in crate::entity_models) fn squid_model(
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    // The glow squid layers share these meshes.
    let layer = if baby {
        ModelLayer::SQUID_BABY
    } else {
        ModelLayer::SQUID
    };
    let mut root = layer.bake();
    // Vanilla `SquidModel.setupAnim` sets `tentacle.xRot = tentacleAngle` on every tentacle, while
    // the body and the tentacles' fixed yaw layout stay at the bind pose. The angle is `0` at rest,
    // so a still squid is identical to its bind tree.
    let tentacle_angle = instance.render_state.kind.squid().tentacle_angle;
    for &name in &SQUID_TENTACLE_NAMES {
        root.child_mut(name).pose.rotation[0] = tentacle_angle;
    }
    root
}
