use super::ghast_tentacle_x_rot;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `HappyGhastModel.createBodyLayer` tentacle child names, in `tentacle0..tentacle8` order;
/// `child_mut` needs `&'static` names, so the procedural ring draws its names from this const array.
const HAPPY_GHAST_TENTACLE_NAMES: [&str; 9] = [
    "tentacle0",
    "tentacle1",
    "tentacle2",
    "tentacle3",
    "tentacle4",
    "tentacle5",
    "tentacle6",
    "tentacle7",
    "tentacle8",
];

/// Vanilla `HappyGhastModel`.
pub(in crate::entity_models) fn happy_ghast_model(
    layer: ModelLayer,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = layer.bake();
    let age_in_ticks = instance.render_state.age_in_ticks;
    let body = root.child_mut("body");
    if instance.render_state.kind.happy_ghast().body_item {
        body.scale = [0.9375; 3];
    }
    for (i, &name) in HAPPY_GHAST_TENTACLE_NAMES.iter().enumerate() {
        body.child_mut(name).pose.rotation[0] = ghast_tentacle_x_rot(i, age_in_ticks);
    }
    root
}

/// Vanilla `HappyGhastHarnessModel`.
pub(in crate::entity_models) fn happy_ghast_harness_model(
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let layer = if baby {
        ModelLayer::HAPPY_GHAST_BABY_HARNESS
    } else {
        ModelLayer::HAPPY_GHAST_HARNESS
    };
    let mut root = layer.bake();
    let goggles = root.child_mut("goggles");
    if instance.render_state.kind.happy_ghast().ridden {
        goggles.pose.rotation[0] = 0.0;
        goggles.pose.offset[1] = 14.0;
    } else {
        #[allow(clippy::approx_constant)]
        {
            goggles.pose.rotation[0] = -0.7854;
        }
        goggles.pose.offset[1] = 9.0;
    }
    root
}
