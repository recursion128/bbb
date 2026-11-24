use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

use super::{apply_head_look, spider_leg_swing_pose, spider_leg_swing_roles};
use crate::entity_models::instances::EntityModelInstance;

/// Mutable spider model, mirroring vanilla `SpiderModel` (shared by the cave spider, which differs
/// only by its smaller root transform). The unified tree is built once with named children.
/// `setup_anim` looks the head ([`apply_head_look`]) and sweeps/steps the eight legs
/// ([`spider_leg_swing_pose`] at the [`spider_leg_swing_roles`] names). Both the base and eyes
/// textured passes read this one posed tree.
pub(in crate::entity_models) fn spider_model(
    cave: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let layer = if cave {
        ModelLayer::CAVE_SPIDER
    } else {
        ModelLayer::SPIDER
    };
    let mut root = layer.bake();
    let render_state = &instance.render_state;
    apply_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
    );
    let limb_swing = render_state.walk_animation_pos;
    let limb_swing_amount = render_state.walk_animation_speed;
    for (name, phase, side_sign) in spider_leg_swing_roles() {
        let leg = root.child_mut(name);
        leg.pose = spider_leg_swing_pose(leg.pose, phase, side_sign, limb_swing, limb_swing_amount);
    }
    root
}
