use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `VexModel.setupAnim` non-charging body tilt: `body.xRot = π/20`.
pub(in crate::entity_models) const VEX_BODY_X_ROT: f32 = std::f32::consts::PI / 20.0;
/// Vanilla `VexModel.setupAnim` arm rest roll: `rightArm.zRot = π/5 + bob`,
/// `leftArm.zRot = -(π/5 + bob)`.
pub(in crate::entity_models) const VEX_ARM_REST_Z_ROT: f32 = std::f32::consts::PI / 5.0;
/// Vanilla `VexModel.setupAnim` wing pitch/roll: both wings use
/// `xRot = zRot = 0.47123888` (the left wing's roll is negated).
pub(in crate::entity_models) const VEX_WING_X_ROT: f32 = 0.471_238_88;
pub(in crate::entity_models) const VEX_WING_Z_ROT: f32 = 0.471_238_88;

/// Vanilla `VexModel.setArmsCharging` (both hands empty) arm pitch: `xRot = -1.2217305` for
/// both arms while the vex is charging an attack.
pub(in crate::entity_models) const VEX_ARM_CHARGING_X_ROT: f32 = -1.221_730_5;
/// Vanilla `VexModel.setArmsCharging` held-item arm pitch: `xRot = π*7/6` for each hand whose
/// `ItemStackRenderState` is non-empty while charging.
pub(in crate::entity_models) const VEX_ARM_CHARGING_ITEM_X_ROT: f32 =
    std::f32::consts::PI * 7.0 / 6.0;
/// Charging arm yaw magnitude: `rightArm.yRot = π/12`, `leftArm.yRot = -π/12`.
pub(in crate::entity_models) const VEX_ARM_CHARGING_Y_ROT: f32 = std::f32::consts::PI / 12.0;
/// Charging arm roll base: `rightArm.zRot = -0.47123888 - bob`, `leftArm.zRot = 0.47123888 +
/// bob` (`bob` is the shared `vex_moving_arm_z_bob`).
pub(in crate::entity_models) const VEX_ARM_CHARGING_Z_ROT: f32 = 0.471_238_88;

/// Vanilla `VexModel.setupAnim` arm bob: `movingArmZBob = cos(ageInTicks · 5.5°) · 0.1`,
/// added to the right arm's `zRot` and subtracted from the left's.
pub(in crate::entity_models) fn vex_moving_arm_z_bob(age_in_ticks: f32) -> f32 {
    (age_in_ticks * 5.5_f32.to_radians()).cos() * 0.1
}

/// Vanilla `VexModel.setupAnim` wing flap: `leftWing.yRot = 1.0995574 +
/// cos(ageInTicks · 45.836624°) · 16.2°`; the right wing mirrors it (`-leftWing.yRot`).
pub(in crate::entity_models) fn vex_left_wing_y_rot(age_in_ticks: f32) -> f32 {
    1.099_557_4 + (age_in_ticks * 45.836_624_f32.to_radians()).cos() * 16.2_f32.to_radians()
}

fn vex_charging_arm_rotations(instance: &EntityModelInstance, bob: f32) -> ([f32; 3], [f32; 3]) {
    let vex_state = *instance.render_state.kind.vex();
    let right_has_item = vex_state.right_hand_item_non_empty;
    let left_has_item = vex_state.left_hand_item_non_empty;
    if !right_has_item && !left_has_item {
        return (
            [
                VEX_ARM_CHARGING_X_ROT,
                VEX_ARM_CHARGING_Y_ROT,
                -VEX_ARM_CHARGING_Z_ROT - bob,
            ],
            [
                VEX_ARM_CHARGING_X_ROT,
                -VEX_ARM_CHARGING_Y_ROT,
                VEX_ARM_CHARGING_Z_ROT + bob,
            ],
        );
    }

    let right_arm_rot = if right_has_item {
        [
            VEX_ARM_CHARGING_ITEM_X_ROT,
            VEX_ARM_CHARGING_Y_ROT,
            -VEX_ARM_CHARGING_Z_ROT - bob,
        ]
    } else {
        [0.0, 0.0, VEX_ARM_REST_Z_ROT + bob]
    };
    let left_arm_rot = if left_has_item {
        [
            VEX_ARM_CHARGING_ITEM_X_ROT,
            -VEX_ARM_CHARGING_Y_ROT,
            VEX_ARM_CHARGING_Z_ROT + bob,
        ]
    } else {
        [0.0, 0.0, -(VEX_ARM_REST_Z_ROT + bob)]
    };
    (right_arm_rot, left_arm_rot)
}

/// Applies the vanilla `VexModel.setupAnim` pose to the unified tree: the head look, the body charging
/// level / idle tilt, the arm charging raise / idle hold (both with the shared `vex_moving_arm_z_bob`),
/// and the wing flap. The charging arm branch follows vanilla `setArmsCharging`: both empty hands use
/// the two-arm lunge, while item-bearing hands pitch to `π*7/6` and empty hands keep the rest roll.
/// Every value is set absolutely, reproducing the hand-walked emit exactly.
fn apply_vex_anim(root: &mut ModelPart, instance: &EntityModelInstance) {
    let age = instance.render_state.age_in_ticks;
    let charging = instance.render_state.kind.vex().charging;
    let head_pitch = instance.render_state.head_pitch.to_radians();
    let head_yaw = instance.render_state.head_yaw.to_radians();
    let bob = vex_moving_arm_z_bob(age);
    let left_wing_yrot = vex_left_wing_y_rot(age);
    let (right_arm_rot, left_arm_rot) = if charging {
        vex_charging_arm_rotations(instance, bob)
    } else {
        (
            [0.0, 0.0, VEX_ARM_REST_Z_ROT + bob],
            [0.0, 0.0, -(VEX_ARM_REST_Z_ROT + bob)],
        )
    };

    let vex_root = root.child_mut("root");
    vex_root.child_mut("head").pose.rotation = [head_pitch, head_yaw, 0.0];

    let body = vex_root.child_mut("body");
    body.pose.rotation = [if charging { 0.0 } else { VEX_BODY_X_ROT }, 0.0, 0.0];
    body.child_mut("right_arm").pose.rotation = right_arm_rot;
    body.child_mut("left_arm").pose.rotation = left_arm_rot;
    body.child_mut("left_wing").pose.rotation = [VEX_WING_X_ROT, left_wing_yrot, -VEX_WING_Z_ROT];
    body.child_mut("right_wing").pose.rotation = [VEX_WING_X_ROT, -left_wing_yrot, VEX_WING_Z_ROT];
}

/// Mutable vex model, mirroring vanilla `VexModel`. The unified tree is built once with named children:
/// the static `root` pivot → (`head`, `body`), with `body` parenting the right arm, left arm, left wing,
/// and right wing. `setup_anim` runs [`apply_vex_anim`] (head look, charging body/arms, wing flap);
/// the native entity scene supplies VexRenderer's full-bright block-light override through
/// submitted light coords.
pub(in crate::entity_models) fn vex_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::VEX.bake();
    apply_vex_anim(&mut root, instance);
    root
}
