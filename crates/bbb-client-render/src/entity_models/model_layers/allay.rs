use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `AllayModel` default root height (`PartPose.offset` y), animated by the idle bob.
pub(in crate::entity_models) const ALLAY_ROOT_BASE_Y: f32 = 23.5;
/// Vanilla `AllayModel.setupAnim` wing yaw base `±π/4`: `rightWing.yRot = -π/4 + flapAmount`,
/// `leftWing.yRot = π/4 - flapAmount`.
pub(in crate::entity_models) const ALLAY_WING_Y_ROT_BASE: f32 = std::f32::consts::FRAC_PI_4;
/// Vanilla `AllayModel.setupAnim` wing rest pitch / arm rest roll factor (`0.43633232` rad ≈
/// 25°): `wing.xRot = 0.43633232·(1 - flyingFactor)` and the arm bob centres on `0.43633232`.
pub(in crate::entity_models) const ALLAY_REST_ANGLE: f32 = 0.436_332_32;
/// Vanilla `AllayModel.setupAnim` flying body tilt `flyingFactor·π/4`.
pub(in crate::entity_models) const ALLAY_BODY_FLYING_X_ROT: f32 = std::f32::consts::FRAC_PI_4;
/// Vanilla `AllayModel.setupAnim` held-item arm yaw factor:
/// `rightArm.yRot = 0.27925268·holding`, `leftArm.yRot = -0.27925268·holding`.
pub(in crate::entity_models) const ALLAY_HELD_ITEM_ARM_Y_ROT: f32 = 0.279_252_68;
/// Vanilla `AllayModel.setupAnim` minimum held-item arm pitch while idle:
/// `Mth.lerp(flyingFactor, -π/3, -1.134464)`.
pub(in crate::entity_models) const ALLAY_MIN_HAND_HOLDING_ITEM_X_ROT: f32 =
    -std::f32::consts::FRAC_PI_3;
/// Vanilla `AllayModel.setupAnim` maximum held-item arm pitch while flying.
pub(in crate::entity_models) const ALLAY_MAX_HAND_HOLDING_ITEM_X_ROT: f32 = -1.134_464;

/// Vanilla `AllayModel.setupAnim`: `flyingFactor = min(walkAnimationSpeed / 0.3, 1)`.
pub(in crate::entity_models) fn allay_flying_factor(walk_animation_speed: f32) -> f32 {
    (walk_animation_speed / 0.3).min(1.0)
}

/// Vanilla `AllayModel.setupAnim` wing flap:
/// `flapAmount = cos(ageInTicks·20° + walkAnimationPos)·π·0.15 + walkAnimationSpeed`.
pub(in crate::entity_models) fn allay_wing_flap_amount(
    age_in_ticks: f32,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> f32 {
    let flap_speed = age_in_ticks * 20.0_f32.to_radians() + walk_animation_pos;
    flap_speed.cos() * std::f32::consts::PI * 0.15 + walk_animation_speed
}

/// Vanilla `AllayModel.setupAnim` idle bob phase: `idleBobSpeed = ageInTicks·9°` (radians).
fn allay_idle_bob_speed(age_in_ticks: f32) -> f32 {
    age_in_ticks * 9.0_f32.to_radians()
}

/// Vanilla `AllayModel.setupAnim` wing rest pitch: `wing.xRot = 0.43633232·(1 - flyingFactor)`.
pub(in crate::entity_models) fn allay_wing_rest_x_rot(walk_animation_speed: f32) -> f32 {
    ALLAY_REST_ANGLE * (1.0 - allay_flying_factor(walk_animation_speed))
}

/// Vanilla `AllayModel.setupAnim` body flying tilt: `body.xRot = flyingFactor·π/4`.
pub(in crate::entity_models) fn allay_body_x_rot(walk_animation_speed: f32) -> f32 {
    allay_flying_factor(walk_animation_speed) * ALLAY_BODY_FLYING_X_ROT
}

/// Vanilla `AllayModel.setupAnim` vertical bob: `root.y = 23.5 + cos(idleBobSpeed)·0.25·(1 -
/// flyingFactor)`.
pub(in crate::entity_models) fn allay_root_y(age_in_ticks: f32, walk_animation_speed: f32) -> f32 {
    let idle_bob_factor = 1.0 - allay_flying_factor(walk_animation_speed);
    ALLAY_ROOT_BASE_Y + allay_idle_bob_speed(age_in_ticks).cos() * 0.25 * idle_bob_factor
}

/// Vanilla `AllayModel.setupAnim` empty-handed arm idle roll.
#[cfg(test)]
pub(in crate::entity_models) fn allay_arm_idle_bob_amount(
    age_in_ticks: f32,
    walk_animation_speed: f32,
) -> f32 {
    allay_arm_z_rot_amount(age_in_ticks, walk_animation_speed, 0.0)
}

/// Vanilla `AllayModel.setupAnim` arm roll:
/// `armIdleBobAmount = 0.43633232 - cos(idleBobSpeed + 3π/2)·π·0.075·(1 - flyingFactor)·(1 -
/// holdingItemFactor)`, with `leftArm.zRot = -amount` and `rightArm.zRot = amount`.
pub(in crate::entity_models) fn allay_arm_z_rot_amount(
    age_in_ticks: f32,
    walk_animation_speed: f32,
    holding_item_progress: f32,
) -> f32 {
    let idle_bob_factor = 1.0 - allay_flying_factor(walk_animation_speed);
    ALLAY_REST_ANGLE
        - (allay_idle_bob_speed(age_in_ticks) + std::f32::consts::PI * 1.5).cos()
            * std::f32::consts::PI
            * 0.075
            * idle_bob_factor
            * (1.0 - holding_item_progress)
}

/// Vanilla `AllayModel.setupAnim` held-item arm pitch:
/// `holdingItemFactor * Mth.lerp(flyingFactor, -π/3, -1.134464)`.
pub(in crate::entity_models) fn allay_arm_holding_x_rot(
    walk_animation_speed: f32,
    holding_item_progress: f32,
) -> f32 {
    let flying_factor = allay_flying_factor(walk_animation_speed);
    holding_item_progress
        * (ALLAY_MIN_HAND_HOLDING_ITEM_X_ROT
            + (ALLAY_MAX_HAND_HOLDING_ITEM_X_ROT - ALLAY_MIN_HAND_HOLDING_ITEM_X_ROT)
                * flying_factor)
}

/// Vanilla `AllayModel.setupAnim` dance phase: `danceSpeed = ageInTicks·8° + walkAnimationSpeed`
/// (radians). The shared cosine of this phase drives the body sway (`16°`), the head Y-tilt
/// (`30°`), and the head Z-tilt (`14°`).
fn allay_dance_speed(age_in_ticks: f32, walk_animation_speed: f32) -> f32 {
    age_in_ticks * 8.0_f32.to_radians() + walk_animation_speed
}

/// Applies the vanilla `AllayModel.setupAnim` pose to the unified tree: the `root` bob, the head
/// look (or the dance head tilt + body sway/spin), the body flying tilt, the held-item arm blend, and
/// the wing flap. Every value is set absolutely each frame, reproducing the hand-walked emit exactly.
fn apply_allay_anim(root: &mut ModelPart, instance: &EntityModelInstance) {
    let allay_state = *instance.render_state.kind.allay();
    let age = instance.render_state.age_in_ticks;
    let walk_pos = instance.render_state.walk_animation_pos;
    let walk_speed = instance.render_state.walk_animation_speed;
    let head_pitch = instance.render_state.head_pitch.to_radians();
    let head_yaw = instance.render_state.head_yaw.to_radians();
    let holding_item_progress = allay_state.holding_item_progress;
    let arm_x = allay_arm_holding_x_rot(walk_speed, holding_item_progress);
    let arm_z = allay_arm_z_rot_amount(age, walk_speed, holding_item_progress);
    let wing_x = allay_wing_rest_x_rot(walk_speed);
    let flap = allay_wing_flap_amount(age, walk_pos, walk_speed);

    let is_dancing = allay_state.dancing;
    let is_spinning = allay_state.spinning;
    let spin_progress = allay_state.spinning_progress;

    let allay_root = root.child_mut("root");
    allay_root.pose.offset[1] = allay_root_y(age, walk_speed);
    if is_dancing {
        // Vanilla dance pose: the body sways (`root.zRot`) and the head tilts in Y/Z, all
        // cross-faded into a full `4π·spinningProgress` body spin (`root.yRot`) by the spin
        // blend. The head pitch/look is dropped while dancing (vanilla leaves `head.xRot` at
        // its reset bind pose), and `root.yRot` only spins during the `isSpinning` sub-window.
        let dance_cos = allay_dance_speed(age, walk_speed).cos();
        let dance_frequency = dance_cos * 16.0_f32.to_radians();
        let head_tilt_y = dance_cos * 30.0_f32.to_radians();
        let head_tilt_z = dance_cos * 14.0_f32.to_radians();
        let root_y_rot = if is_spinning {
            4.0 * std::f32::consts::PI * spin_progress
        } else {
            0.0
        };
        allay_root.pose.rotation = [0.0, root_y_rot, dance_frequency * (1.0 - spin_progress)];
        allay_root.child_mut("head").pose.rotation = [
            0.0,
            head_tilt_y * (1.0 - spin_progress),
            head_tilt_z * (1.0 - spin_progress),
        ];
    } else {
        allay_root.pose.rotation = [0.0, 0.0, 0.0];
        allay_root.child_mut("head").pose.rotation = [head_pitch, head_yaw, 0.0];
    }

    let body = allay_root.child_mut("body");
    body.pose.rotation = [allay_body_x_rot(walk_speed), 0.0, 0.0];
    body.child_mut("right_arm").pose.rotation = [
        arm_x,
        ALLAY_HELD_ITEM_ARM_Y_ROT * holding_item_progress,
        arm_z,
    ];
    body.child_mut("left_arm").pose.rotation = [
        arm_x,
        -ALLAY_HELD_ITEM_ARM_Y_ROT * holding_item_progress,
        -arm_z,
    ];
    body.child_mut("right_wing").pose.rotation = [wing_x, -ALLAY_WING_Y_ROT_BASE + flap, 0.0];
    body.child_mut("left_wing").pose.rotation = [wing_x, ALLAY_WING_Y_ROT_BASE - flap, 0.0];
}

/// Vanilla `AllayModel`.
pub(in crate::entity_models) fn allay_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::ALLAY.bake();
    apply_allay_anim(&mut root, instance);
    root
}
