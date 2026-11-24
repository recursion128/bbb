use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::{animate, animate_walk};

use crate::entity_models::model_data_26_1::{armadillo_animation, baby_armadillo_animation};
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `ArmadilloModel.setupAnim` head look (only while not hiding): the pitch (`xRot`) clamps to
/// [-22.5, 25] and the yaw (`yRot`) to [-32.5, 32.5] degrees before `head.xRot/yRot` are set.
/// Returns the clamped `(yaw, pitch)` in degrees.
pub(in crate::entity_models) fn armadillo_clamped_head_look(
    head_yaw_deg: f32,
    head_pitch_deg: f32,
) -> (f32, f32) {
    (
        head_yaw_deg.clamp(-32.5, 32.5),
        head_pitch_deg.clamp(-22.5, 25.0),
    )
}

pub(in crate::entity_models) fn armadillo_model(
    baby: bool,
    rolled_up: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = if baby {
        ModelLayer::ARMADILLO_BABY
    } else {
        ModelLayer::ARMADILLO
    }
    .bake();
    let state = &instance.render_state;
    let hiding = rolled_up;
    root.child_mut("body").skip_draw = hiding;
    root.child_mut("left_hind_leg").visible = !hiding;
    root.child_mut("right_hind_leg").visible = !hiding;
    root.part_mut("tail").unwrap().visible = !hiding;
    root.child_mut("cube").visible = hiding;
    let (walk, roll_out, roll_up, peek) = if baby {
        (
            &baby_armadillo_animation::ARMADILLO_BABY_WALK,
            &baby_armadillo_animation::ARMADILLO_BABY_ROLL_OUT,
            &baby_armadillo_animation::ARMADILLO_BABY_ROLL_UP,
            &baby_armadillo_animation::ARMADILLO_BABY_PEEK,
        )
    } else {
        (
            &armadillo_animation::ARMADILLO_WALK,
            &armadillo_animation::ARMADILLO_ROLL_OUT,
            &armadillo_animation::ARMADILLO_ROLL_UP,
            &armadillo_animation::ARMADILLO_PEEK,
        )
    };
    if !hiding {
        let (yaw, pitch) = armadillo_clamped_head_look(state.head_yaw, state.head_pitch);
        let head = root.part_mut("head").unwrap();
        head.pose.rotation[0] = pitch.to_radians();
        head.pose.rotation[1] = yaw.to_radians();
        animate_walk(
            &mut root,
            walk,
            state.walk_animation_pos,
            state.walk_animation_speed,
            16.5,
            2.5,
        );
    }
    for (definition, seconds) in [
        (roll_out, state.kind.armadillo().roll_out_seconds),
        (roll_up, state.kind.armadillo().roll_up_seconds),
        (peek, state.kind.armadillo().peek_seconds),
    ] {
        if seconds >= 0.0 {
            animate(&mut root, definition, seconds, 1.0);
        }
    }
    root
}
