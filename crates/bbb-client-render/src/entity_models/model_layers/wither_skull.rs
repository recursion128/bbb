use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};
use std::f32::consts::PI;

/// Dragon skull model for `CustomHeadLayer` skull equipment. Vanilla `DragonHeadModel` uses the
/// ender-dragon head and jaw cubes, a scaled `head` part, and opens the jaw from
/// `SkullModelBase.State.animationPos`.
pub(in crate::entity_models) fn custom_head_dragon_skull_model(
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = ModelLayer::DRAGON_SKULL.bake();
    let animation_pos = instance.render_state.worn_head_animation_pos;
    root.child_mut("head").child_mut("jaw").pose.rotation[0] =
        ((animation_pos * PI * 0.2).sin() + 1.0) * 0.2;
    root
}

/// Piglin skull model for `CustomHeadLayer` skull equipment. Vanilla `PiglinHeadModel` reuses
/// `AbstractPiglinModel.addHead(CubeDeformation.NONE)`: adult piglin head cubes plus two ear
/// children, then drives only the ears from `SkullModelBase.State.animationPos`.
pub(in crate::entity_models) fn custom_head_piglin_skull_model(
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = ModelLayer::PIGLIN_HEAD.bake();
    let animation_pos = instance.render_state.worn_head_animation_pos;
    let head = root.child_mut("head");
    head.child_mut("left_ear").pose.rotation[2] =
        -((animation_pos * PI * 0.2 * 1.2).cos() + 2.5) * 0.2;
    head.child_mut("right_ear").pose.rotation[2] = ((animation_pos * PI * 0.2).cos() + 2.5) * 0.2;
    root
}
