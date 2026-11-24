use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::{animate, animate_walk};
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_data_26_1::baby_axolotl_animation;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// The per-part rotation/offset deltas the adult axolotl's factor-blended sub-animations accumulate
/// before they are added onto the reset bind pose, mirroring vanilla `AdultAxolotlModel.setupAnim`'s
/// `+=` onto the freshly-`resetPose`d parts. Each field is a delta (radians for rotations, model
/// units for `body_y`), summed across all five sub-animations in vanilla order so the mirror-leg copy
/// sees the final accumulated left-leg deltas.
#[derive(Default)]
struct AxolotlPose {
    /// `body` `[xRot, _, zRot]` deltas (the `yRot` head-look is applied separately).
    body: [f32; 3],
    /// `body` y-offset delta (vanilla `this.body.y -= …`).
    body_y: f32,
    head: [f32; 3],
    tail_y: f32,
    top_gills_x: f32,
    left_gills_y: f32,
    right_gills_y: f32,
    left_hind: [f32; 3],
    left_front: [f32; 3],
    right_hind: [f32; 3],
    right_front: [f32; 3],
}

/// Vanilla `AdultAxolotlModel.setupSwimmingAnimation`: the moving-in-water gallop.
fn axolotl_setup_swimming(pose: &mut AxolotlPose, age: f32, x_rot_deg: f32, factor: f32) {
    if factor <= 1.0e-5 {
        return;
    }
    let anim_move_speed = age * 0.33;
    let sine_sway = anim_move_speed.sin();
    let cosine_sway = anim_move_speed.cos();
    let body_sway = 0.13 * sine_sway;
    pose.body[0] += (x_rot_deg.to_radians() + body_sway) * factor;
    pose.head[0] -= body_sway * 1.8 * factor;
    pose.body_y -= 0.45 * cosine_sway * factor;
    pose.top_gills_x += (-0.5 * sine_sway - 0.8) * factor;
    let gill_y_rot = (0.3 * sine_sway + 0.9) * factor;
    pose.left_gills_y += gill_y_rot;
    pose.right_gills_y -= gill_y_rot;
    pose.tail_y += 0.3 * (anim_move_speed * 0.9).cos() * factor;
    pose.left_hind[0] += 1.884_955_8 * factor;
    pose.left_hind[1] += -0.4 * sine_sway * factor;
    pose.left_hind[2] += std::f32::consts::FRAC_PI_2 * factor;
    pose.left_front[0] += 1.884_955_8 * factor;
    pose.left_front[1] += (-0.2 * cosine_sway - 0.1) * factor;
    pose.left_front[2] += std::f32::consts::FRAC_PI_2 * factor;
}

/// Vanilla `AdultAxolotlModel.setupWaterHoveringAnimation`: the still-in-water hover.
fn axolotl_setup_water_hovering(pose: &mut AxolotlPose, age: f32, factor: f32) {
    if factor <= 1.0e-5 {
        return;
    }
    let anim_move_speed = age * 0.075;
    let cosine_sway = anim_move_speed.cos();
    let sine_sway = anim_move_speed.sin() * 0.15;
    let body_x_rot = (-0.15 + 0.075 * cosine_sway) * factor;
    pose.body[0] += body_x_rot;
    pose.body_y -= sine_sway * factor;
    pose.head[0] -= body_x_rot;
    pose.top_gills_x += 0.2 * cosine_sway * factor;
    let gill_y_rot = (-0.3 * cosine_sway - 0.19) * factor;
    pose.left_gills_y += gill_y_rot;
    pose.right_gills_y -= gill_y_rot;
    pose.left_hind[0] += (3.0 * std::f32::consts::PI / 4.0 - cosine_sway * 0.11) * factor;
    pose.left_hind[1] += 0.471_238_94 * factor;
    pose.left_hind[2] += 1.727_876_1 * factor;
    pose.left_front[0] += (std::f32::consts::FRAC_PI_4 - cosine_sway * 0.2) * factor;
    pose.left_front[1] += 2.042_035 * factor;
    pose.tail_y += 0.5 * cosine_sway * factor;
}

/// Vanilla `AdultAxolotlModel.setupGroundCrawlingAnimation`: the moving-on-ground crawl.
fn axolotl_setup_ground_crawling(pose: &mut AxolotlPose, age: f32, factor: f32) {
    if factor <= 1.0e-5 {
        return;
    }
    let anim_move_speed = age * 0.11;
    let cosine_sway = anim_move_speed.cos();
    let hind_leg_y_rot_sway = (cosine_sway * cosine_sway - 2.0 * cosine_sway) / 5.0;
    let front_leg_y_rot_sway = 0.7 * cosine_sway;
    let head_and_tail_y_rot = 0.09 * cosine_sway * factor;
    pose.head[1] += head_and_tail_y_rot;
    pose.tail_y += head_and_tail_y_rot;
    let gill_angle =
        (0.6 - 0.08 * (cosine_sway * cosine_sway + 2.0 * anim_move_speed.sin())) * factor;
    pose.top_gills_x += gill_angle;
    pose.left_gills_y -= gill_angle;
    pose.right_gills_y += gill_angle;
    let hind_leg_x_rot = 0.942_477_9 * factor;
    let front_leg_x_rot = 1.099_557_4 * factor;
    pose.left_hind[0] += hind_leg_x_rot;
    pose.left_hind[1] += (1.5 - hind_leg_y_rot_sway) * factor;
    pose.left_hind[2] += -0.1 * factor;
    pose.left_front[0] += front_leg_x_rot;
    pose.left_front[1] += (std::f32::consts::FRAC_PI_2 - front_leg_y_rot_sway) * factor;
    pose.right_hind[0] += hind_leg_x_rot;
    pose.right_hind[1] += (-1.0 - hind_leg_y_rot_sway) * factor;
    pose.right_front[0] += front_leg_x_rot;
    pose.right_front[1] += (-std::f32::consts::FRAC_PI_2 - front_leg_y_rot_sway) * factor;
}

/// Vanilla `AdultAxolotlModel.setupLayStillOnGroundAnimation`: the still-on-ground rest.
fn axolotl_setup_lay_still(pose: &mut AxolotlPose, age: f32, factor: f32) {
    if factor <= 1.0e-5 {
        return;
    }
    let anim_move_speed = age * 0.09;
    let sine_sway = anim_move_speed.sin();
    let cosine_sway = anim_move_speed.cos();
    let movement = sine_sway * sine_sway - 2.0 * sine_sway;
    let movement2 = cosine_sway * cosine_sway - 3.0 * sine_sway;
    pose.head[0] += -0.09 * movement * factor;
    pose.head[2] += -0.2 * factor;
    pose.tail_y += (-0.1 + 0.1 * movement) * factor;
    let gill_angle = (0.6 + 0.05 * movement2) * factor;
    pose.top_gills_x += gill_angle;
    pose.left_gills_y -= gill_angle;
    pose.right_gills_y += gill_angle;
    pose.left_hind[0] += 1.1 * factor;
    pose.left_hind[1] += 1.0 * factor;
    pose.left_front[0] += 0.8 * factor;
    pose.left_front[1] += 2.3 * factor;
    pose.left_front[2] -= 0.5 * factor;
}

/// Vanilla `AdultAxolotlModel.setupPlayDeadAnimation`: the limp-on-its-side play-dead pose.
fn axolotl_setup_play_dead(pose: &mut AxolotlPose, factor: f32) {
    if factor <= 1.0e-5 {
        return;
    }
    pose.left_hind[0] += 1.413_716_7 * factor;
    pose.left_hind[1] += 1.099_557_4 * factor;
    pose.left_hind[2] += std::f32::consts::FRAC_PI_4 * factor;
    pose.left_front[0] += std::f32::consts::FRAC_PI_4 * factor;
    pose.left_front[1] += 2.042_035 * factor;
    pose.body[0] += -0.15 * factor;
    pose.body[2] += 0.35 * factor;
}

/// Vanilla `AdultAxolotlModel.applyMirrorLegRotations`: copy the accumulated left-leg rotations onto
/// the right legs (Y/Z mirrored) scaled by the `mirroredLegsFactor`. Runs last, so it reads the final
/// left-leg deltas from every preceding sub-animation.
fn axolotl_apply_mirror_legs(pose: &mut AxolotlPose, factor: f32) {
    if factor <= 1.0e-5 {
        return;
    }
    pose.right_hind[0] += pose.left_hind[0] * factor;
    pose.right_hind[1] += -pose.left_hind[1] * factor;
    pose.right_hind[2] += -pose.left_hind[2] * factor;
    pose.right_front[0] += pose.left_front[0] * factor;
    pose.right_front[1] += -pose.left_front[1] * factor;
    pose.right_front[2] += -pose.left_front[2] * factor;
}

/// Adds an `[xRot, yRot, zRot]` delta onto a part's reset bind rotation.
fn add_rotation(part: &mut ModelPart, delta: [f32; 3]) {
    part.pose.rotation[0] += delta[0];
    part.pose.rotation[1] += delta[1];
    part.pose.rotation[2] += delta[2];
}

/// Vanilla `AdultAxolotlModel` / `BabyAxolotlModel`.
pub(in crate::entity_models) fn axolotl_model(
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = if baby {
        ModelLayer::AXOLOTL_BABY
    } else {
        ModelLayer::AXOLOTL
    }
    .bake();
    if baby {
        let state = &instance.render_state;
        if state.kind.axolotl().baby_walk_seconds >= 0.0 {
            animate_walk(
                &mut root,
                &baby_axolotl_animation::AXOLOTL_WALK_FLOOR,
                state.walk_animation_pos,
                state.walk_animation_speed,
                15.0,
                30.0,
            );
        }
        // 26.1 applies `walkUnderwaterAnimation` with `walkAnimationState`
        // so the dedicated underwater-walk state has no consumer.
        for (definition, seconds) in [
            (
                &baby_axolotl_animation::BABY_AXOLOTL_SWIM,
                state.kind.axolotl().baby_swim_seconds,
            ),
            (
                &baby_axolotl_animation::WALK_FLOOR_UNDERWATER,
                state.kind.axolotl().baby_walk_seconds,
            ),
            (
                &baby_axolotl_animation::BABY_AXOLOTL_IDLE_FLOOR,
                state.kind.axolotl().baby_idle_on_ground_seconds,
            ),
            (
                &baby_axolotl_animation::IDLE_UNDERWATER,
                state.kind.axolotl().baby_idle_underwater_seconds,
            ),
            (
                &baby_axolotl_animation::IDLE_FLOOR_UNDERWATER,
                state.kind.axolotl().baby_idle_underwater_on_ground_seconds,
            ),
            (
                &baby_axolotl_animation::BABY_AXOLOTL_PLAY_DEAD,
                state.kind.axolotl().baby_play_dead_seconds,
            ),
        ] {
            if seconds >= 0.0 {
                animate(&mut root, definition, seconds, 1.0);
            }
        }
        return root;
    }
    let state = &instance.render_state;
    let age = state.age_in_ticks;
    let playing_dead = state.kind.axolotl().playing_dead_factor;
    let in_water = state.kind.axolotl().in_water_factor;
    let on_ground = state.kind.axolotl().on_ground_factor;
    let moving = state.kind.axolotl().moving_factor;
    let not_moving = 1.0 - moving;
    // Vanilla `AdultAxolotlModel.setupAnim`: `mirroredLegsFactor = 1 - min(onGround, moving)`.
    let mirrored_legs = 1.0 - on_ground.min(moving);

    // Accumulate every sub-animation's `+=` deltas (vanilla order) before touching the tree, so
    // the borrow checker is happy and the mirror-leg copy sees the final left-leg totals.
    let mut pose = AxolotlPose::default();
    axolotl_setup_swimming(&mut pose, age, state.head_pitch, moving.min(in_water));
    axolotl_setup_water_hovering(&mut pose, age, not_moving.min(in_water));
    axolotl_setup_ground_crawling(&mut pose, age, moving.min(on_ground));
    axolotl_setup_lay_still(&mut pose, age, not_moving.min(on_ground));
    axolotl_setup_play_dead(&mut pose, playing_dead);
    axolotl_apply_mirror_legs(&mut pose, mirrored_legs);

    let head_yaw = state.head_yaw.to_radians();
    let body = root.child_mut("body");
    body.pose.rotation[0] += pose.body[0];
    body.pose.rotation[1] += head_yaw;
    body.pose.rotation[2] += pose.body[2];
    body.pose.offset[1] += pose.body_y;
    {
        let head = body.child_mut("head");
        head.pose.rotation[0] += pose.head[0];
        head.pose.rotation[1] += pose.head[1];
        head.pose.rotation[2] += pose.head[2];
        head.child_mut("top_gills").pose.rotation[0] += pose.top_gills_x;
        head.child_mut("left_gills").pose.rotation[1] += pose.left_gills_y;
        head.child_mut("right_gills").pose.rotation[1] += pose.right_gills_y;
    }
    add_rotation(body.child_mut("right_front_leg"), pose.right_front);
    add_rotation(body.child_mut("left_front_leg"), pose.left_front);
    add_rotation(body.child_mut("right_hind_leg"), pose.right_hind);
    add_rotation(body.child_mut("left_hind_leg"), pose.left_hind);
    body.child_mut("tail").pose.rotation[1] += pose.tail_y;
    root
}
