use std::f32::consts::{FRAC_PI_2, FRAC_PI_3, FRAC_PI_4, FRAC_PI_6, PI};

use crate::entity_models::catalog::EntityModelKind;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// The `AbstractEquineModel` subclass whose overrides drive `setupAnim`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum EquineModelClass {
    /// `HorseModel` and the adult `DonkeyModel` (which also toggles its chests).
    Adult,
    BabyHorse,
    BabyDonkey,
    /// `EquineSaddleModel`: the bridle lines show only while ridden.
    Saddle,
}

/// Vanilla `AbstractEquineModel.setupAnim` over a baked horse, donkey, mule, undead horse, horse
/// armor or equine saddle layer.
pub(in crate::entity_models) fn equine_model(
    layer: ModelLayer,
    class: EquineModelClass,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = layer.bake();
    let state = &instance.render_state;
    let baby_donkey = class == EquineModelClass::BabyDonkey;
    // getLegStandAngle, getLegStandingYOffset, getLegStandingZOffset,
    // getLegStandingXRotOffset, getTailXRotOffset.
    let (leg_stand_angle, leg_standing_y, leg_standing_z, leg_standing_x_rot, tail_x_rot) =
        match class {
            EquineModelClass::BabyHorse => (PI / 12.0, 4.0, 0.0, -FRAC_PI_3, -FRAC_PI_2),
            EquineModelClass::BabyDonkey => (FRAC_PI_3, 1.0, 0.5, 0.0, -FRAC_PI_4),
            _ => (PI / 12.0, 12.0, 4.0, -FRAC_PI_3, 0.0),
        };

    let clamped_y_rot = state.head_yaw.clamp(-20.0, 20.0).to_radians();
    let speed = state.walk_animation_speed;
    let pos = state.walk_animation_pos;
    // `BabyDonkeyModel.setupAnim` forces `state.xRot = -30` before recomputing the head pitch.
    let head_rot_x = if baby_donkey {
        -FRAC_PI_6
    } else {
        let mut head_rot_x = state.head_pitch.to_radians();
        if speed > 0.2 {
            head_rot_x += (pos * 0.8).cos() * 0.15 * speed;
        }
        head_rot_x
    };
    let eating = state.kind.equine().eat_animation;
    let standing = state.kind.equine().stand_animation;
    let i_standing = 1.0 - standing;
    let idle = 1.0 - standing.max(eating);
    let age = state.age_in_ticks;
    let eating_angle = if baby_donkey { FRAC_PI_2 } else { 2.1816616 };

    let head_parts = root.part_mut("head_parts").unwrap();
    head_parts.pose.rotation[0] = standing * (PI / 12.0 + head_rot_x)
        + eating * (eating_angle + age.sin() * 0.05)
        + idle
            * (FRAC_PI_6 + head_rot_x + state.kind.equine().feeding_animation * age.sin() * 0.05);
    head_parts.pose.rotation[1] = standing * clamped_y_rot + idle * clamped_y_rot;
    let offset = &mut head_parts.pose.offset;
    match class {
        EquineModelClass::BabyHorse => {
            offset[1] += lerp(eating, lerp(standing, 0.0, -2.0), 2.0);
            offset[2] = lerp(standing, offset[2], -4.0);
        }
        EquineModelClass::BabyDonkey => {
            offset[1] = lerp(eating, offset[1], -1.2);
            offset[2] = lerp(standing, offset[2], -3.6);
        }
        _ => {
            offset[1] += lerp(eating, lerp(standing, 0.0, -8.0), 7.0);
            offset[2] = lerp(standing, offset[2], -4.0);
        }
    }

    let body = root.part_mut("body").unwrap();
    body.pose.rotation[0] = standing * -FRAC_PI_4 + i_standing * body.pose.rotation[0];

    let water = if state.in_water { 0.2 } else { 1.0 };
    let leg_anim = (water * pos * 0.6662 + PI).cos();
    let leg_x_rot = leg_anim * 0.8 * speed;
    let bob = (age * 0.6 + PI).cos();
    let stand_angle = leg_stand_angle * standing;
    let left_front = root.part_mut("left_front_leg").unwrap();
    left_front.pose.offset[1] -= leg_standing_y * standing;
    left_front.pose.offset[2] += leg_standing_z * standing;
    left_front.pose.rotation[0] = (leg_standing_x_rot + bob) * standing + leg_x_rot * i_standing;
    let [_, front_y, front_z] = left_front.pose.offset;
    let right_front = root.part_mut("right_front_leg").unwrap();
    right_front.pose.offset[1] = front_y;
    right_front.pose.offset[2] = front_z;
    right_front.pose.rotation[0] = (leg_standing_x_rot - bob) * standing - leg_x_rot * i_standing;
    let hind_swing = leg_anim * 0.5 * speed * i_standing;
    let left_hind = root.part_mut("left_hind_leg").unwrap();
    left_hind.pose.rotation[0] = stand_angle - hind_swing;
    if baby_donkey {
        left_hind.pose.offset[1] = lerp(standing, left_hind.pose.offset[1], -0.3);
    }
    let left_hind_y = left_hind.pose.offset[1];
    let right_hind = root.part_mut("right_hind_leg").unwrap();
    right_hind.pose.rotation[0] = stand_angle + hind_swing;
    if baby_donkey {
        right_hind.pose.offset[1] = lerp(standing, left_hind_y, -0.3);
    }

    let tail = root.part_mut("tail").unwrap();
    tail.pose.rotation[0] = tail_x_rot + FRAC_PI_6 + speed * 0.75;
    tail.pose.offset[1] += speed * state.age_scale;
    tail.pose.offset[2] += speed * 2.0 * state.age_scale;
    tail.pose.rotation[1] = if state.kind.equine().animate_tail {
        (age * 0.7).cos()
    } else {
        0.0
    };

    if class == EquineModelClass::Saddle {
        let ridden = state.kind.equine().saddle_ridden;
        root.part_mut("left_saddle_line").unwrap().visible = ridden;
        root.part_mut("right_saddle_line").unwrap().visible = ridden;
    } else if let EntityModelKind::Donkey { has_chest, .. } = instance.kind {
        root.part_mut("left_chest").unwrap().visible = has_chest;
        root.part_mut("right_chest").unwrap().visible = has_chest;
    }
    root
}

fn lerp(delta: f32, start: f32, end: f32) -> f32 {
    start + delta * (end - start)
}
