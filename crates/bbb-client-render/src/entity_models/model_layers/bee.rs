use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

use std::f32::consts::PI;

// The flying middle-leg angle (vanilla sets all three legs to `π/4` in the airborne branch before
// the bob overrides the front/back pair).
pub(in crate::entity_models) const BEE_MID_LEG_FLYING_X_ROT: f32 = PI / 4.0;

/// Vanilla `BeeModel.setupAnim` wing flap: `zRot = cos(ageInTicks · 120.32113°) · π · 0.15`. The
/// left wing mirrors this (`leftWing.zRot = -rightWing.zRot`).
pub(in crate::entity_models) fn bee_wing_z_rot(age_in_ticks: f32) -> f32 {
    (age_in_ticks * 120.32113 * (PI / 180.0)).cos() * PI * 0.15
}

/// The shared `bobUpAndDown` speed term: `cos(ageInTicks · 0.18)`.
pub(in crate::entity_models) fn bee_bob_speed(age_in_ticks: f32) -> f32 {
    (age_in_ticks * 0.18).cos()
}

/// `bone.xRot = 0.1 + speed · π · 0.025`.
pub(in crate::entity_models) fn bee_bone_x_rot(age_in_ticks: f32) -> f32 {
    0.1 + bee_bob_speed(age_in_ticks) * PI * 0.025
}

/// The vertical bob added to the bone pivot: `bone.y -= cos(ageInTicks · 0.18) · 0.9`.
pub(in crate::entity_models) fn bee_bone_y_delta(age_in_ticks: f32) -> f32 {
    -bee_bob_speed(age_in_ticks) * 0.9
}

/// `frontLeg.xRot = -speed · π · 0.1 + π/8`.
pub(in crate::entity_models) fn bee_front_leg_x_rot(age_in_ticks: f32) -> f32 {
    -bee_bob_speed(age_in_ticks) * PI * 0.1 + PI / 8.0
}

/// `backLeg.xRot = -speed · π · 0.05 + π/4`.
pub(in crate::entity_models) fn bee_back_leg_x_rot(age_in_ticks: f32) -> f32 {
    -bee_bob_speed(age_in_ticks) * PI * 0.05 + PI / 4.0
}

/// Adult-only antenna bob (`AdultBeeModel.bobUpAndDown`): `antenna.xRot = speed · π · 0.03`.
pub(in crate::entity_models) fn bee_antenna_x_rot(age_in_ticks: f32) -> f32 {
    bee_bob_speed(age_in_ticks) * PI * 0.03
}

/// Applies vanilla `BeeModel.setupAnim` to the unified tree. While airborne the wings flap on
/// `ageInTicks` and (when not angry) `bobUpAndDown` rocks the `bone` pivot, the front/back legs and —
/// on adults — the antennae; all three legs first splay to `π/4`, so the middle leg holds that angle
/// while the bob overrides the front/back pair. On the ground the model rests at its bind pose. The
/// stinger cube is hidden once the bee has stung (`stinger.visible = hasStinger`).
fn apply_bee_anim(root: &mut ModelPart, baby: bool, instance: &EntityModelInstance) {
    let bee_state = *instance.render_state.kind.bee();
    let age = instance.render_state.age_in_ticks;
    let flying = !instance.render_state.on_ground;
    // Vanilla gates `bobUpAndDown` on `!isAngry && !isOnGround`: an angry airborne bee still flaps
    // its wings and splays its legs to `π/4`, but its body, front/back legs and antennae hold still.
    let bob = flying && !bee_state.angry;
    let has_stinger = bee_state.has_stinger;

    // Bone pivot (root child): the airborne bob rocks it forward and lifts/drops it.
    let bone = root.child_mut("bone");
    if bob {
        bone.pose.offset[1] += bee_bone_y_delta(age);
        bone.pose.rotation = [bee_bone_x_rot(age), 0.0, 0.0];
    }

    // Body subtree: the stinger is shown only while carried; the adult antennae bob with the body.
    {
        let body = bone.child_mut("body");
        body.child_mut("stinger").visible = has_stinger;
        if !baby {
            let antenna_x_rot = if bob { bee_antenna_x_rot(age) } else { 0.0 };
            body.child_mut("left_antenna").pose.rotation = [antenna_x_rot, 0.0, 0.0];
            body.child_mut("right_antenna").pose.rotation = [antenna_x_rot, 0.0, 0.0];
        }
    }

    // Wings (bone children): the flap overrides the bind yaw to 0 and drives `zRot`, mirrored on the
    // left, while the bind pitch (0 on adults, `0.2182` on babies) is preserved.
    if flying {
        let wing_z_rot = bee_wing_z_rot(age);
        let right_wing = bone.child_mut("right_wing");
        right_wing.pose.rotation = [right_wing.pose.rotation[0], 0.0, wing_z_rot];
        let left_wing = bone.child_mut("left_wing");
        left_wing.pose.rotation = [left_wing.pose.rotation[0], 0.0, -wing_z_rot];
    }

    // Legs (bone children): airborne, all three splay to `π/4`; the non-angry bob then overrides the
    // front/back pair, while an angry bee holds all three at `π/4`. On the ground they rest at `0`.
    let (front_x, mid_x, back_x) = if flying {
        (
            if bob {
                bee_front_leg_x_rot(age)
            } else {
                BEE_MID_LEG_FLYING_X_ROT
            },
            BEE_MID_LEG_FLYING_X_ROT,
            if bob {
                bee_back_leg_x_rot(age)
            } else {
                BEE_MID_LEG_FLYING_X_ROT
            },
        )
    } else {
        (0.0, 0.0, 0.0)
    };
    bone.child_mut("front_legs").pose.rotation = [front_x, 0.0, 0.0];
    bone.child_mut("middle_legs").pose.rotation = [mid_x, 0.0, 0.0];
    bone.child_mut("back_legs").pose.rotation = [back_x, 0.0, 0.0];

    // Vanilla `BeeModel.setupAnim` applies the barrel roll last: a rolling bee tips its `bone` pivot
    // onto its back by interpolating the (already bob-posed) pitch toward `3.0915928` (≈ π − 0.05).
    let roll = bee_state.roll_amount;
    if roll > 0.0 {
        bone.pose.rotation[0] = rot_lerp_rad(roll, bone.pose.rotation[0], 3.0915928);
    }
}

/// Vanilla `Mth.rotLerpRad(a, from, to)`: lerps `from` toward `to` along the shortest signed angular
/// path (the `to - from` delta is wrapped into `[-π, π)` before scaling by `a`).
fn rot_lerp_rad(a: f32, from: f32, to: f32) -> f32 {
    super::head_look::humanoid_rot_lerp_rad(a, from, to)
}

/// Vanilla `AdultBeeModel` / `BabyBeeModel`.
pub(in crate::entity_models) fn bee_model(baby: bool, instance: &EntityModelInstance) -> BakedRoot {
    let mut root = if baby {
        ModelLayer::BEE_BABY
    } else {
        ModelLayer::BEE
    }
    .bake();
    apply_bee_anim(&mut root, baby, instance);
    root
}
