use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Adult body base height (`customAnimations` sets `body.y = 2.0` before the bob).
pub(in crate::entity_models) const STRIDER_BODY_BASE_Y: f32 = 2.0;
/// Adult leg base height (`leftLeg.y`/`rightLeg.y = 8.0` before the lift) and leg x offsets.
pub(in crate::entity_models) const STRIDER_LEG_BASE_Y: f32 = 8.0;

/// Baby body base height (`customAnimations` sets `body.y = 17.25` before the bob).
pub(in crate::entity_models) const STRIDER_BABY_BODY_BASE_Y: f32 = 17.25;
/// Baby leg base height (`leg.y = 20.0` before the lift) and leg x offsets.
pub(in crate::entity_models) const STRIDER_BABY_LEG_BASE_Y: f32 = 20.0;

/// Vanilla `StriderModel.SPEED` (the body sway / leg swing frequency multiplier).
const STRIDER_SPEED: f32 = 1.5;

/// Vanilla `StriderModel.setupAnim`: `animationSpeed = min(walkAnimationSpeed, 0.25)`.
pub(in crate::entity_models) fn strider_animation_speed(walk_animation_speed: f32) -> f32 {
    walk_animation_speed.min(0.25)
}

/// Vanilla `StriderModel.setupAnim` body sway: `body.zRot = 0.1·sin(pos·1.5)·4·speed`.
pub(in crate::entity_models) fn strider_body_z_rot(pos: f32, speed: f32) -> f32 {
    0.1 * (pos * STRIDER_SPEED).sin() * 4.0 * speed
}

/// Vanilla `StriderModel.setupAnim` leg swing: `leg.xRot = sin(pos·0.75 + phase)·2·speed` with
/// `phase = 0` (left) or `π` (right).
pub(in crate::entity_models) fn strider_leg_x_rot(pos: f32, speed: f32, right: bool) -> f32 {
    let phase = if right { std::f32::consts::PI } else { 0.0 };
    (pos * STRIDER_SPEED * 0.5 + phase).sin() * 2.0 * speed
}

/// Vanilla `StriderModel.setupAnim` leg roll: `leg.zRot = (π/18)·cos(pos·0.75 + phase)·speed`
/// with `phase = 0` (left) or `π` (right).
pub(in crate::entity_models) fn strider_leg_z_rot(pos: f32, speed: f32, right: bool) -> f32 {
    let phase = if right { std::f32::consts::PI } else { 0.0 };
    (std::f32::consts::PI / 18.0) * (pos * STRIDER_SPEED * 0.5 + phase).cos() * speed
}

/// Vanilla `customAnimations` body bob: `body.y = base - mul·cos(pos·1.5)·2·speed` (the adult
/// uses `mul = 2`, the baby `mul = 1`).
pub(in crate::entity_models) fn strider_body_y(base: f32, mul: f32, pos: f32, speed: f32) -> f32 {
    base - mul * (pos * STRIDER_SPEED).cos() * 2.0 * speed
}

/// Vanilla `customAnimations` leg lift: `leg.y = base + 2·sin(pos·0.75 + phase)·2·speed` with
/// `phase = 0` (right) or `π` (left) — note this phase is the opposite of the leg swing.
pub(in crate::entity_models) fn strider_leg_y(base: f32, pos: f32, speed: f32, right: bool) -> f32 {
    let phase = if right { 0.0 } else { std::f32::consts::PI };
    base + 2.0 * (pos * STRIDER_SPEED * 0.5 + phase).sin() * 2.0 * speed
}

/// Vanilla `customAnimations` bristle flow: `bristleFlow = cos(pos·1.5 + π)·speed`.
pub(in crate::entity_models) fn strider_bristle_flow(pos: f32, speed: f32) -> f32 {
    (pos * STRIDER_SPEED + std::f32::consts::PI).cos() * speed
}

/// Vanilla `animateBristle` for the top/front bristle: `flow·0.6 + 0.1·sin(age·0.4)`.
pub(in crate::entity_models) fn strider_bristle_top_flow(flow: f32, age_in_ticks: f32) -> f32 {
    flow * 0.6 + 0.1 * (age_in_ticks * 0.4).sin()
}

/// Vanilla `animateBristle` for the middle bristle: `flow·1.2 + 0.1·sin(age·0.2)`.
pub(in crate::entity_models) fn strider_bristle_middle_flow(flow: f32, age_in_ticks: f32) -> f32 {
    flow * 1.2 + 0.1 * (age_in_ticks * 0.2).sin()
}

/// Vanilla `animateBristle` for the bottom/back bristle: `flow·1.3 + 0.05·sin(age·-0.4)`.
pub(in crate::entity_models) fn strider_bristle_bottom_flow(flow: f32, age_in_ticks: f32) -> f32 {
    flow * 1.3 + 0.05 * (age_in_ticks * -0.4).sin()
}

/// Applies the vanilla `StriderModel.setupAnim` + `{Adult,Baby}StriderModel.customAnimations` to the
/// unified tree: the legs swing (`xRot`) / roll (`zRot`) / lift (`y`) in opposition, the body tracks
/// the look and sways (`zRot`) / bobs (`y`), and the bristles flow with the walk plus an `ageInTicks`
/// ripple — the adult's six bristles on `zRot`, the baby's three on `xRot`. Ridden striders zero body
/// look pitch/yaw, matching `StriderRenderState.isRidden`.
fn apply_strider_anim(root: &mut ModelPart, baby: bool, instance: &EntityModelInstance) {
    let age = instance.render_state.age_in_ticks;
    let pos = instance.render_state.walk_animation_pos;
    let speed = strider_animation_speed(instance.render_state.walk_animation_speed);
    let (head_pitch, head_yaw) = if instance.render_state.kind.strider().ridden {
        (0.0, 0.0)
    } else {
        (
            instance.render_state.head_pitch.to_radians(),
            instance.render_state.head_yaw.to_radians(),
        )
    };
    let (leg_base_y, body_base_y, body_bob_mul) = if baby {
        (STRIDER_BABY_LEG_BASE_Y, STRIDER_BABY_BODY_BASE_Y, 1.0)
    } else {
        (STRIDER_LEG_BASE_Y, STRIDER_BODY_BASE_Y, 2.0)
    };

    let right_leg = root.child_mut("right_leg");
    right_leg.pose.offset[1] = strider_leg_y(leg_base_y, pos, speed, true);
    right_leg.pose.rotation = [
        strider_leg_x_rot(pos, speed, true),
        0.0,
        strider_leg_z_rot(pos, speed, true),
    ];

    let left_leg = root.child_mut("left_leg");
    left_leg.pose.offset[1] = strider_leg_y(leg_base_y, pos, speed, false);
    left_leg.pose.rotation = [
        strider_leg_x_rot(pos, speed, false),
        0.0,
        strider_leg_z_rot(pos, speed, false),
    ];

    let body = root.child_mut("body");
    body.pose.offset[1] = strider_body_y(body_base_y, body_bob_mul, pos, speed);
    body.pose.rotation = [head_pitch, head_yaw, strider_body_z_rot(pos, speed)];

    let flow = strider_bristle_flow(pos, speed);
    let top = strider_bristle_top_flow(flow, age);
    let middle = strider_bristle_middle_flow(flow, age);
    let bottom = strider_bristle_bottom_flow(flow, age);
    if baby {
        // The three baby bristles flap on `xRot` (no rest roll), in [front, middle, back] order. The
        // `+=` is relative to each bristle's bind pose.
        body.child_mut("bristle2").pose.rotation[0] += top;
        body.child_mut("bristle1").pose.rotation[0] += middle;
        body.child_mut("bristle0").pose.rotation[0] += bottom;
    } else {
        // The six adult bristles flow on `zRot`: right top/middle/bottom then left top/middle/bottom.
        // Each `+=` adds to the bristle's non-zero rest `zRot`.
        body.child_mut("right_top_bristle").pose.rotation[2] += top;
        body.child_mut("right_middle_bristle").pose.rotation[2] += middle;
        body.child_mut("right_bottom_bristle").pose.rotation[2] += bottom;
        body.child_mut("left_top_bristle").pose.rotation[2] += top;
        body.child_mut("left_middle_bristle").pose.rotation[2] += middle;
        body.child_mut("left_bottom_bristle").pose.rotation[2] += bottom;
    }
}

/// Vanilla `AdultStriderModel` / `BabyStriderModel`, also drawn for the saddle layer.
pub(in crate::entity_models) fn strider_model(
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let layer = if baby {
        ModelLayer::STRIDER_BABY
    } else {
        ModelLayer::STRIDER
    };
    let mut root = layer.bake();
    apply_strider_anim(&mut root, baby, instance);
    root
}
