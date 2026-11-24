use super::{apply_head_look, limb_swing_at_rest};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::animate_walk;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_data_26_1::fox_baby_animation::FOX_BABY_WALK;
use crate::entity_models::model_layer::BakedRoot;
use std::f32::consts::{FRAC_PI_2, PI};

/// Vanilla `FoxModel.setWalkingPose` + the `AdultFoxModel` override: always run. Tilts the head by the
/// eased `headRollAngle` (`head.zRot`), keeps all four legs visible (un-doing any prior sleep hide on a
/// reused tree), and — for the adult — sweeps the gait. The legs' bind pose carries no roll, so the
/// `head.zRot` is set; the walk swing sets `xRot` (a no-op at rest).
fn fox_set_walking_pose(root: &mut ModelPart, baby: bool, instance: &EntityModelInstance) {
    let state = &instance.render_state;
    root.child_mut("head").pose.rotation[2] = state.kind.fox().head_roll_angle;
    for name in FOX_LEG_NAMES {
        root.child_mut(name).visible = true;
    }
    if baby {
        apply_fox_baby_walk(root, state.walk_animation_pos, state.walk_animation_speed);
    } else {
        apply_fox_adult_leg_swing(root, state.walk_animation_pos, state.walk_animation_speed);
    }
}

/// Vanilla `FoxModel.setCrouchingPose` + the adult/baby override. The base pose pitches the body
/// (`body.xRot += π/30`), lifts the head by `crouchAmount · ageScale`, and wiggles the body yaw and legs
/// by `cos(ageInTicks) · 0.05`; the adult adds `body.y += crouchAmount`, the baby `body.y +=
/// crouchAmount / 6`.
fn fox_set_crouching_pose(root: &mut ModelPart, baby: bool, instance: &EntityModelInstance) {
    let state = &instance.render_state;
    let age_scale = state.age_scale;
    let crouch = state.kind.fox().crouch_amount;
    let wiggle = state.age_in_ticks.cos() * 0.05;

    let head = root.child_mut("head");
    head.pose.offset[1] += crouch * age_scale;

    let body = root.child_mut("body");
    body.pose.rotation[0] += 0.10471976;
    body.pose.rotation[1] = wiggle;
    body.pose.offset[1] += if baby { crouch / 6.0 } else { crouch };

    root.child_mut("right_hind_leg").pose.rotation[2] = wiggle;
    root.child_mut("left_hind_leg").pose.rotation[2] = wiggle;
    root.child_mut("right_front_leg").pose.rotation[2] = wiggle / 2.0;
    root.child_mut("left_front_leg").pose.rotation[2] = wiggle / 2.0;
}

/// Vanilla `FoxModel.setSleepingPose` + the adult/baby override. The base pose hides all four legs; the
/// adult/baby override then folds the body onto its side (`body.zRot = -π/2`), drops the tail and shifts
/// the head. Mirrors how the bee hides its stinger via `ModelPart::visible`. The head's
/// `xRot`/`yRot`/`zRot` set here is overwritten by the `setupAnim` sleeping override afterwards; the
/// `head.x`/`head.y` shifts persist.
fn fox_set_sleeping_pose(root: &mut ModelPart, baby: bool) {
    for name in FOX_LEG_NAMES {
        root.child_mut(name).visible = false;
    }
    if baby {
        let body = root.child_mut("body");
        body.pose.rotation[2] = -FRAC_PI_2;
        body.pose.rotation[0] = -PI / 18.0;
        body.pose.offset[1] += 1.5;
        body.pose.offset[2] -= 1.5;
        body.pose.offset[0] -= 1.5;
        let tail = body.child_mut("tail");
        tail.pose.rotation[0] = -2.1816616;
        tail.pose.offset[0] -= 0.7;
        tail.pose.offset[2] += 0.6;
        tail.pose.offset[1] += 0.9;
        let head = root.child_mut("head");
        head.pose.offset[0] -= 2.0;
        head.pose.offset[1] += 2.8;
        head.pose.offset[2] -= 4.0;
    } else {
        let body = root.child_mut("body");
        body.pose.rotation[2] = -FRAC_PI_2;
        body.pose.offset[1] += 5.0;
        body.child_mut("tail").pose.rotation[0] = -PI * 5.0 / 6.0;
        let head = root.child_mut("head");
        head.pose.offset[0] += 2.0;
        head.pose.offset[1] += 2.99;
    }
}

/// Vanilla `FoxModel.setSittingPose` + the adult/baby override (the base sets `head.xRot/yRot = 0`). The
/// adult folds the body down and back, lifts the hind legs and pitches the front legs and tail; the baby
/// scales most shifts by `ageScale`.
fn fox_set_sitting_pose(root: &mut ModelPart, baby: bool, age_scale: f32) {
    if baby {
        let body = root.child_mut("body");
        body.pose.rotation[0] = -0.959931;
        body.pose.offset[2] -= 4.5 * age_scale;
        body.pose.offset[1] += 3.0 * age_scale;
        let tail = body.child_mut("tail");
        tail.pose.offset[1] -= 0.6;
        tail.pose.offset[2] -= 2.0 * age_scale;
        tail.pose.rotation[0] = 0.95993114;

        let head = root.child_mut("head");
        head.pose.rotation = [0.0, 0.0, head.pose.rotation[2]];
        head.pose.offset[1] -= 0.75;

        for (name, x_sign) in [("right_front_leg", 1.0), ("left_front_leg", -1.0)] {
            let leg = root.child_mut(name);
            leg.pose.rotation[0] = -PI / 12.0;
            leg.pose.offset[2] -= 1.5;
            leg.pose.offset[0] += 0.01 * x_sign;
        }
        for (name, x_sign) in [("right_hind_leg", 1.0), ("left_hind_leg", -1.0)] {
            let leg = root.child_mut(name);
            leg.pose.offset[2] -= 3.75;
            leg.pose.offset[0] += 0.01 * x_sign;
        }
    } else {
        let body = root.child_mut("body");
        body.pose.rotation[0] = PI / 6.0;
        body.pose.offset[1] -= 7.0;
        body.pose.offset[2] += 3.0;
        let tail = body.child_mut("tail");
        tail.pose.rotation[0] = PI / 4.0;
        tail.pose.offset[2] -= 1.0;

        let head = root.child_mut("head");
        head.pose.rotation = [0.0, 0.0, head.pose.rotation[2]];
        head.pose.offset[1] -= 6.5;
        head.pose.offset[2] += 2.75;

        root.child_mut("right_front_leg").pose.rotation[0] = -PI / 12.0;
        root.child_mut("left_front_leg").pose.rotation[0] = -PI / 12.0;
        for name in ["right_hind_leg", "left_hind_leg"] {
            let leg = root.child_mut(name);
            leg.pose.rotation[0] = -PI * 5.0 / 12.0;
            leg.pose.offset[1] += 4.0;
            leg.pose.offset[2] -= 0.25;
        }
    }
}

/// Vanilla `AdultFoxModel.setPouncingPose`: drops the body and head by `crouchAmount / 2`. The base
/// `FoxModel.setPouncingPose` is empty and the baby does not override it, so this is adult-only.
fn fox_set_pouncing_pose(root: &mut ModelPart, baby: bool, instance: &EntityModelInstance) {
    if baby {
        return;
    }
    let drop = instance.render_state.kind.fox().crouch_amount / 2.0;
    root.child_mut("body").pose.offset[1] -= drop;
    root.child_mut("head").pose.offset[1] -= drop;
}

/// The four leg part names shared by both layouts, in vanilla `addOrReplaceChild` order.
const FOX_LEG_NAMES: [&str; 4] = [
    "right_hind_leg",
    "left_hind_leg",
    "right_front_leg",
    "left_front_leg",
];

/// Vanilla `BabyFoxModel`'s `applyWalk(walkPos, walkSpeed, 1.0, 2.5)` factors: `MAX_WALK_ANIMATION_SPEED`
/// and `WALK_ANIMATION_SCALE_FACTOR`. The speed factor scales `walkPos` into the animation time; the
/// scale factor caps the amplitude (`min(walkSpeed · 2.5, 1.0)`), so a standing baby adds nothing.
const FOX_BABY_WALK_SPEED_FACTOR: f32 = 1.0;
const FOX_BABY_WALK_SCALE_FACTOR: f32 = 2.5;

/// Vanilla `AdultFoxModel.setWalkingPose` leg swing: each leg's `xRot = cos(pos·0.6662 [+ π]) · 1.4 ·
/// speed`. The diagonal pairing — back-right & front-left in phase, back-left & front-right a half-cycle
/// out — is keyed by leg NAME because the fox builds all four legs at the same negative pivot X (`+2`
/// off-center), so the `QuadrupedModel` `x·z` sign rule can't resolve the phase. The base leg pose
/// carries no `xRot`, so it is set (not accumulated). A no-op while at rest (`walkAnimationSpeed == 0`),
/// matching the static leg pose. The baby uses the `FOX_BABY_WALK` keyframe gait instead
/// ([`apply_fox_baby_walk`]), so this swing is adult-only.
fn apply_fox_adult_leg_swing(
    root: &mut ModelPart,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) {
    if limb_swing_at_rest(walk_animation_speed) {
        return;
    }
    let phase = walk_animation_pos * 0.6662;
    // back-right (`right_hind_leg`) & front-left (`left_front_leg`) are in phase; the other diagonal is
    // half a cycle out, exactly the `QuadrupedModel` diagonal gait.
    for (name, phase_offset) in [
        ("right_hind_leg", 0.0),
        ("left_front_leg", 0.0),
        ("left_hind_leg", PI),
        ("right_front_leg", PI),
    ] {
        root.child_mut(name).pose.rotation[0] =
            (phase + phase_offset).cos() * 1.4 * walk_animation_speed;
    }
}

fn apply_fox_baby_walk(root: &mut ModelPart, walk_animation_pos: f32, walk_animation_speed: f32) {
    if limb_swing_at_rest(walk_animation_speed) {
        return;
    }
    animate_walk(
        root,
        &FOX_BABY_WALK,
        walk_animation_pos,
        walk_animation_speed,
        FOX_BABY_WALK_SPEED_FACTOR,
        FOX_BABY_WALK_SCALE_FACTOR,
    );
}

/// Vanilla `FoxModel`.
pub(in crate::entity_models) fn fox_model(baby: bool, instance: &EntityModelInstance) -> BakedRoot {
    let mut root = crate::entity_models::textured::fox_model_layer(baby).bake();
    let state = &instance.render_state;

    // Vanilla `FoxModel.setupAnim`: `setWalkingPose` always runs (head tilt, legs visible, adult gait).
    fox_set_walking_pose(&mut root, baby, instance);

    // Then exactly one of the crouch / sleep / sit poses (crouch wins, then sleep, then sit).
    if state.kind.fox().is_crouching {
        fox_set_crouching_pose(&mut root, baby, instance);
    } else if state.kind.fox().is_sleeping {
        fox_set_sleeping_pose(&mut root, baby);
    } else if state.kind.fox().is_sitting {
        fox_set_sitting_pose(&mut root, baby, instance.render_state.age_scale);
    }

    // The pounce drop layers on top of whichever pose ran.
    if state.kind.fox().is_pouncing {
        fox_set_pouncing_pose(&mut root, baby, instance);
    }

    // Resting head look: only while not sleeping / faceplanted / crouching (those own the head).
    if !state.kind.fox().is_sleeping
        && !state.kind.fox().is_faceplanted
        && !state.kind.fox().is_crouching
    {
        apply_head_look(root.child_mut("head"), state.head_yaw, state.head_pitch);
    }

    // Sleeping head override: a fixed yaw with a slow `ageInTicks` roll wobble.
    if state.kind.fox().is_sleeping {
        let head = root.child_mut("head");
        head.pose.rotation[0] = 0.0;
        head.pose.rotation[1] = -PI * 2.0 / 3.0;
        head.pose.rotation[2] = (state.age_in_ticks * 0.027).cos() / 22.0;
    }

    // Faceplant leg twitch: a small `ageInTicks`-driven cosine flap, the diagonals out of phase. The
    // model is rebuilt each frame, so vanilla's persistent `legMotionPos` accumulator is derived from
    // `ageInTicks` here (the same `+= 0.67`/tick cadence, phase-continuous across frames).
    if state.kind.fox().is_faceplanted {
        let leg_motion_pos = state.age_in_ticks * 0.67;
        let twitch = (leg_motion_pos * 0.4662).cos() * 0.1;
        let twitch_offset = (leg_motion_pos * 0.4662 + PI).cos() * 0.1;
        root.child_mut("right_hind_leg").pose.rotation[0] = twitch;
        root.child_mut("left_hind_leg").pose.rotation[0] = twitch_offset;
        root.child_mut("right_front_leg").pose.rotation[0] = twitch_offset;
        root.child_mut("left_front_leg").pose.rotation[0] = twitch;
    }
    root
}
