use super::{apply_head_look, apply_quadruped_leg_swing};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::BakedRoot;

/// Vanilla `GoatModel`.
pub(in crate::entity_models) fn goat_model(
    baby: bool,
    left_horn: bool,
    right_horn: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = crate::entity_models::textured::goat_model_layer(baby).bake();
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
    let head = root.child_mut("head");
    head.child_mut("left_horn").visible = left_horn;
    head.child_mut("right_horn").visible = right_horn;
    // Vanilla `GoatModel.setupAnim`: a ramming goat tilts its head down
    // (`if rammingXHeadRot != 0 { head.xRot = rammingXHeadRot }`), overwriting the head-look pitch.
    if render_state.kind.goat().ramming_x_head_rot != 0.0 {
        head.pose.rotation[0] = render_state.kind.goat().ramming_x_head_rot;
    }
    root
}
