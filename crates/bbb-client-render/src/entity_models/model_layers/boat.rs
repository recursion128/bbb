use crate::entity_models::catalog::BoatModelFamily;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::BakedRoot;
use crate::entity_models::textured::boat_model_layer;

fn boat_paddle_lerp(delta: f32, start: f32, end: f32) -> f32 {
    start + delta.clamp(0.0, 1.0) * (end - start)
}

/// Vanilla `AbstractBoatModel.animatePaddle`: converts `BoatRenderState.rowingTime*`
/// into a paddle `xRot` plus side-specific `yRot`.
pub(in crate::entity_models) fn boat_paddle_rotations(rowing_time: f32, side: usize) -> [f32; 2] {
    let x_rot = boat_paddle_lerp(
        ((-rowing_time).sin() + 1.0) / 2.0,
        -std::f32::consts::PI / 3.0,
        -std::f32::consts::PI / 12.0,
    );
    let y_rot = boat_paddle_lerp(
        ((-rowing_time + 1.0).sin() + 1.0) / 2.0,
        -std::f32::consts::PI / 4.0,
        std::f32::consts::PI / 4.0,
    );
    if side == 1 {
        [x_rot, std::f32::consts::PI - y_rot]
    } else {
        [x_rot, y_rot]
    }
}

/// Vanilla `BoatModel` / `RaftModel` (+ chest variants) with `AbstractBoatModel.animatePaddle`.
pub(in crate::entity_models) fn boat_model(
    family: BoatModelFamily,
    chest: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = boat_model_layer(family, chest).bake();
    let boat_state = *instance.render_state.kind.boat();
    let left = boat_paddle_rotations(boat_state.rowing_time_left, 0);
    let left_paddle = root.child_mut("left_paddle");
    left_paddle.pose.rotation[0] = left[0];
    left_paddle.pose.rotation[1] = left[1];

    let right = boat_paddle_rotations(boat_state.rowing_time_right, 1);
    let right_paddle = root.child_mut("right_paddle");
    right_paddle.pose.rotation[0] = right[0];
    right_paddle.pose.rotation[1] = right[1];
    root
}
