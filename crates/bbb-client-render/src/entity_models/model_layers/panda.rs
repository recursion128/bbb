use super::{apply_head_look, apply_quadruped_leg_swing};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::BakedRoot;

/// Mutable panda model, mirroring vanilla `PandaModel` / `BabyPandaModel` (both `QuadrupedModel`s). The
/// unified tree is built for the selected `baby` layout with the vanilla child names. `setup_anim` runs
/// the shared `QuadrupedModel` head look ([`apply_head_look`] on `head`) and four-leg swing
/// ([`apply_quadruped_leg_swing`]), then applies every projected panda-specific pose below.
/// Vanilla `PandaModel.setupAnim`'s `isUnhappy` and `isSneezing` branches, applied after the inherited
/// `QuadrupedModel` head look + four-leg swing. An unhappy panda shakes its head (`yRot = zRot =
/// 0.35·sin(0.6·age)`, overwriting the look yaw) and paddles its front legs (`xRot = ∓0.75·sin(0.3·age)`,
/// overwriting the walk swing); a sneezing panda dips its head (`xRot` ramps to `-π/4` over `sneezeTime`
/// 0..14, then holds at `-π/4` for 15..19 — vanilla's `(sneezeTime-15)/5` is integer division, so the
/// "ease back" term is always 0). Both run on the reset bind pose, so the resting (`else`) head `zRot`
/// stays 0 with no action.
fn apply_panda_emotes(
    root: &mut ModelPart,
    unhappy: bool,
    sneezing: bool,
    sneeze_time: i32,
    age_in_ticks: f32,
) {
    use std::f32::consts::PI;
    if unhappy {
        let head_shake = 0.35 * (0.6 * age_in_ticks).sin();
        {
            let head = root.child_mut("head");
            head.pose.rotation[1] = head_shake;
            head.pose.rotation[2] = head_shake;
        }
        let paddle = 0.75 * (0.3 * age_in_ticks).sin();
        root.child_mut("right_front_leg").pose.rotation[0] = -paddle;
        root.child_mut("left_front_leg").pose.rotation[0] = paddle;
    }
    if sneezing {
        let head = root.child_mut("head");
        if sneeze_time < 15 {
            head.pose.rotation[0] = -PI / 4.0 * sneeze_time as f32 / 14.0;
        } else if sneeze_time < 20 {
            head.pose.rotation[0] = -PI / 4.0;
        }
    }
}

fn apply_panda_sitting(
    root: &mut ModelPart,
    baby: bool,
    sit_amount: f32,
    eating: bool,
    scared: bool,
    age_in_ticks: f32,
) {
    if baby {
        animate_baby_panda_sitting(root, sit_amount);
    } else {
        animate_adult_panda_sitting(root, sit_amount);
    }

    if eating {
        let eating_wave = (age_in_ticks * 0.6).sin();
        root.child_mut("head").pose.rotation[0] = std::f32::consts::FRAC_PI_2 + 0.2 * eating_wave;
        root.child_mut("right_front_leg").pose.rotation[0] = -0.4 - 0.2 * eating_wave;
        root.child_mut("left_front_leg").pose.rotation[0] = -0.4 - 0.2 * eating_wave;
    }

    if scared {
        root.child_mut("head").pose.rotation[0] = 2.1707964;
        root.child_mut("right_front_leg").pose.rotation[0] = -0.9;
        root.child_mut("left_front_leg").pose.rotation[0] = -0.9;
    }
}

fn animate_adult_panda_sitting(root: &mut ModelPart, sit_amount: f32) {
    {
        let body = root.child_mut("body");
        body.pose.rotation[0] = rot_lerp_rad(sit_amount, body.pose.rotation[0], 1.7407963);
    }
    {
        let head = root.child_mut("head");
        head.pose.rotation[0] = rot_lerp_rad(
            sit_amount,
            head.pose.rotation[0],
            std::f32::consts::FRAC_PI_2,
        );
    }
    set_panda_sitting_leg_rolls(root);
}

fn animate_baby_panda_sitting(root: &mut ModelPart, sit_amount: f32) {
    {
        let body = root.child_mut("body");
        body.pose.rotation[0] = rot_lerp_rad(
            sit_amount,
            body.pose.rotation[0],
            std::f32::consts::PI / 18.0,
        );
        body.pose.offset[2] = lerp(sit_amount, body.pose.offset[2], -1.5);
    }
    {
        let head = root.child_mut("head");
        head.pose.offset[2] = lerp(sit_amount, head.pose.offset[2], -11.5);
        head.pose.offset[1] = lerp(sit_amount, head.pose.offset[1], 17.5);
    }
    {
        let leg = root.child_mut("right_front_leg");
        leg.pose.offset[2] = lerp(sit_amount, leg.pose.offset[2], -5.0);
    }
    {
        let leg = root.child_mut("left_front_leg");
        leg.pose.offset[2] = lerp(sit_amount, leg.pose.offset[2], -5.0);
    }
    {
        let leg = root.child_mut("right_hind_leg");
        leg.pose.offset[2] = lerp(sit_amount, leg.pose.offset[2], 3.0);
    }
    {
        let leg = root.child_mut("left_hind_leg");
        leg.pose.offset[2] = lerp(sit_amount, leg.pose.offset[2], 3.0);
    }
    set_panda_sitting_leg_rolls(root);
}

fn set_panda_sitting_leg_rolls(root: &mut ModelPart) {
    root.child_mut("right_front_leg").pose.rotation[2] = -0.27079642;
    root.child_mut("left_front_leg").pose.rotation[2] = 0.27079642;
    root.child_mut("right_hind_leg").pose.rotation[2] = 0.5707964;
    root.child_mut("left_hind_leg").pose.rotation[2] = -0.5707964;
}

fn reset_panda_leg_rolls(root: &mut ModelPart) {
    root.child_mut("right_hind_leg").pose.rotation[2] = 0.0;
    root.child_mut("left_hind_leg").pose.rotation[2] = 0.0;
    root.child_mut("right_front_leg").pose.rotation[2] = 0.0;
    root.child_mut("left_front_leg").pose.rotation[2] = 0.0;
}

fn apply_panda_lie_on_back(root: &mut ModelPart, lie_on_back_amount: f32, age_in_ticks: f32) {
    root.child_mut("right_hind_leg").pose.rotation[0] = -0.6 * (age_in_ticks * 0.15).sin();
    root.child_mut("left_hind_leg").pose.rotation[0] = 0.6 * (age_in_ticks * 0.15).sin();
    root.child_mut("right_front_leg").pose.rotation[0] = 0.3 * (age_in_ticks * 0.25).sin();
    root.child_mut("left_front_leg").pose.rotation[0] = -0.3 * (age_in_ticks * 0.25).sin();
    let head = root.child_mut("head");
    head.pose.rotation[0] = rot_lerp_rad(
        lie_on_back_amount,
        head.pose.rotation[0],
        std::f32::consts::FRAC_PI_2,
    );
}

fn apply_panda_roll(root: &mut ModelPart, roll_amount: f32, age_in_ticks: f32) {
    {
        let head = root.child_mut("head");
        head.pose.rotation[0] = rot_lerp_rad(roll_amount, head.pose.rotation[0], 2.0561945);
    }
    root.child_mut("right_hind_leg").pose.rotation[0] = -0.5 * (age_in_ticks * 0.5).sin();
    root.child_mut("left_hind_leg").pose.rotation[0] = 0.5 * (age_in_ticks * 0.5).sin();
    root.child_mut("right_front_leg").pose.rotation[0] = 0.5 * (age_in_ticks * 0.5).sin();
    root.child_mut("left_front_leg").pose.rotation[0] = -0.5 * (age_in_ticks * 0.5).sin();
}

/// Vanilla `Mth.rotLerpRad(a, from, to)`: lerps along the shortest signed angular path.
fn rot_lerp_rad(a: f32, from: f32, to: f32) -> f32 {
    super::head_look::humanoid_rot_lerp_rad(a, from, to)
}

fn lerp(a: f32, from: f32, to: f32) -> f32 {
    from + a * (to - from)
}

pub(in crate::entity_models) fn panda_model(
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = crate::entity_models::textured::panda_model_layer(baby).bake();
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
    // Vanilla `PandaModel.setupAnim` overrides the head/front legs for the unhappy shake and the
    // sneeze head dip after the inherited quadruped pose.
    apply_panda_emotes(
        &mut root,
        render_state.kind.panda().unhappy,
        render_state.kind.panda().sneezing,
        render_state.kind.panda().sneeze_time,
        render_state.age_in_ticks,
    );
    if render_state.kind.panda().sit_amount > 0.0 {
        apply_panda_sitting(
            &mut root,
            baby,
            render_state.kind.panda().sit_amount,
            render_state.kind.panda().eating,
            render_state.kind.panda().scared,
            render_state.age_in_ticks,
        );
    } else {
        reset_panda_leg_rolls(&mut root);
    }
    if render_state.kind.panda().lie_on_back_amount > 0.0 {
        apply_panda_lie_on_back(
            &mut root,
            render_state.kind.panda().lie_on_back_amount,
            render_state.age_in_ticks,
        );
    }
    if render_state.kind.panda().roll_amount > 0.0 {
        apply_panda_roll(
            &mut root,
            render_state.kind.panda().roll_amount,
            render_state.age_in_ticks,
        );
    }
    root
}
