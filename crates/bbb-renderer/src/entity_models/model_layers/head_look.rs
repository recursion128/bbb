use super::PartPose;
use crate::entity_models::model::ModelPart;

/// Head-part index for standalone single-head body layers whose mesh lists the head first (the iron
/// and snow golems still address their head positionally). The creeper, spider/cave spider, enderman,
/// and wolf now build named-children trees and resolve the head by name, so this is retained only as
/// the reference the head-look unit test asserts the golems' head-first layout against.
#[cfg(test)]
pub(in crate::entity_models) const fn head_first_part_index() -> usize {
    0
}

/// Vanilla `HoglinModel.setupAnim` ear sway for one ear: `rightEar.zRot = -2π/9 -
/// walkAnimationSpeed * sin(walkAnimationPos)`, `leftEar.zRot = +2π/9 + walkAnimationSpeed
/// * sin(walkAnimationPos)`. Vanilla writes the absolute angle from the literal `2π/9`, so
/// this *sets* `zRot = ±(2π/9 + speed * sin(pos))` (right `−`, left `+`) rather than adding
/// onto the layer's rest splay; only `zRot` changes. `left` selects the side. The adult
/// ears already rest at `±2π/9`, so this matches their rest pose; the baby layer rests its
/// ears at a wider angle that vanilla overrides to `±2π/9`, so the baby ears must always be
/// re-posed through this (even standing).
pub(in crate::entity_models) fn hoglin_ear_sway_pose(
    base: PartPose,
    left: bool,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let ear_z = std::f32::consts::PI * 2.0 / 9.0;
    let magnitude = ear_z + walk_animation_speed * walk_animation_pos.sin();
    let z_rot = if left { magnitude } else { -magnitude };
    PartPose {
        offset: base.offset,
        rotation: [base.rotation[0], base.rotation[1], z_rot],
    }
}

/// True when a head has no look turn (head aligned with the body and level), so
/// callers can borrow the static parts unchanged instead of cloning to apply
/// [`head_look_pose`].
pub(in crate::entity_models) fn head_look_at_rest(head_yaw_deg: f32, head_pitch_deg: f32) -> bool {
    head_yaw_deg == 0.0 && head_pitch_deg == 0.0
}

/// True when a yaw-only head has no turn. The yaw-only head models now set their head yaw through
/// `setup_anim` directly (a `+=` that is identity at a level gaze), so this rest check is retained
/// only as the reference the head-look unit test asserts against.
#[cfg(test)]
pub(in crate::entity_models) fn head_yaw_at_rest(head_yaw_deg: f32) -> bool {
    head_yaw_deg == 0.0
}

/// Vanilla yaw-only head look: sets `head.yRot = yRot * π/180` while leaving the
/// base `head.xRot` (and `head.zRot`) untouched. Used by `HoglinModel`, whose
/// `setupAnim` keeps `head.xRot` at the headbutt animation value (the fixed
/// `HOGLIN_HEAD_X_ROT` tilt at rest) instead of following the look pitch.
pub(in crate::entity_models) fn head_look_yaw_pose(base: PartPose, head_yaw_deg: f32) -> PartPose {
    PartPose {
        offset: base.offset,
        rotation: [
            base.rotation[0],
            head_yaw_deg.to_radians(),
            base.rotation[2],
        ],
    }
}

/// True when the limb swing is at rest (`walkAnimationSpeed == 0`), so callers can
/// borrow the static leg parts unchanged instead of cloning to apply
/// [`quadruped_leg_swing_pose`].
pub(in crate::entity_models) fn limb_swing_at_rest(walk_animation_speed: f32) -> bool {
    walk_animation_speed == 0.0
}

/// Vanilla `QuadrupedModel.setupAnim` leg swing for a single leg part: sets
/// `leg.xRot = cos(walkAnimationPos * 0.6662 [+ π]) * 1.4 * walkAnimationSpeed`.
/// Vanilla puts the right-hind and left-front legs in phase (`cos(...)`) and the
/// left-hind and right-front legs a half-cycle out of phase (`cos(... + π)`). That
/// diagonal pairing is exactly the legs whose part offset satisfies `x * z < 0`
/// (right is `x < 0`, hind is `z > 0`), so the phase is resolved from the leg's
/// offset and is correct whatever order a model lists its legs in. The base leg
/// pose carries no `xRot`, so it is set (not accumulated), matching the vanilla
/// assignment.
pub(in crate::entity_models) fn quadruped_leg_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let phase = walk_animation_pos * 0.6662;
    let [x, _, z] = base.offset;
    let angle = if x * z < 0.0 {
        phase
    } else {
        phase + std::f32::consts::PI
    };
    PartPose {
        offset: base.offset,
        rotation: [
            angle.cos() * 1.4 * walk_animation_speed,
            base.rotation[1],
            base.rotation[2],
        ],
    }
}

/// Vanilla shared `setupAnim` head look applied to a model part: sets the head's pitch/yaw from the
/// look angles ([`head_look_pose`]). The quadruped/humanoid families assign `head.xRot`/`head.yRot`
/// unconditionally every frame; the head's bind pose carries no head rotation, so the angles are set.
pub(in crate::entity_models) fn apply_head_look(
    head: &mut ModelPart,
    head_yaw_deg: f32,
    head_pitch_deg: f32,
) {
    head.pose = head_look_pose(head.pose, head_yaw_deg, head_pitch_deg);
}

/// Vanilla `QuadrupedModel.setupAnim` leg swing applied to a model root's four named leg children
/// ([`quadruped_leg_swing_pose`]). Shared by the quadruped family models, which build a unified tree
/// with the vanilla `QuadrupedModel` child names. A no-op while the limbs are at rest
/// (`walkAnimationSpeed == 0`). The swing resolves each leg's phase from its own offset, so the four
/// names may be declared in any order.
pub(in crate::entity_models) fn apply_quadruped_leg_swing(
    root: &mut ModelPart,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) {
    if limb_swing_at_rest(walk_animation_speed) {
        return;
    }
    for name in [
        "right_hind_leg",
        "left_hind_leg",
        "right_front_leg",
        "left_front_leg",
    ] {
        let leg = root.child_mut(name);
        leg.pose = quadruped_leg_swing_pose(leg.pose, walk_animation_pos, walk_animation_speed);
    }
}

/// Vanilla `HumanoidModel.setupAnim` leg swing for a single leg part: sets
/// `leg.xRot = cos(walkAnimationPos * 0.6662 [+ π]) * 1.4 * walkAnimationSpeed`.
/// The right leg (part offset `x < 0`) is in phase (`cos(...)`) and the left leg a
/// half-cycle out of phase (`cos(... + π)`) — the legs swing oppositely, each
/// coordinated against the same-side arm. Both legs sit at `z = 0`, so the phase is
/// resolved from the `x` sign alone (the `QuadrupedModel` `x * z` rule would be
/// ambiguous). The base leg pose carries no `xRot`, so it is set (not accumulated),
/// matching the vanilla assignment. `state.speedValue` is `1.0` for every entity
/// that is not elytra fall-flying (a deferred pose) so it is omitted; the constant
/// `±0.005` leg yaw/roll splay vanilla always applies is ~0.3° and is omitted too.
pub(in crate::entity_models) fn humanoid_leg_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let phase = walk_animation_pos * 0.6662;
    let angle = if base.offset[0] < 0.0 {
        phase
    } else {
        phase + std::f32::consts::PI
    };
    PartPose {
        offset: base.offset,
        rotation: [
            angle.cos() * 1.4 * walk_animation_speed,
            base.rotation[1],
            base.rotation[2],
        ],
    }
}

/// Vanilla `HumanoidModel.setupAnim` walking arm swing for a single arm part: sets
/// `arm.xRot = cos(walkAnimationPos * 0.6662 [+ π]) * 2.0 * walkAnimationSpeed * 0.5`
/// (amplitude `1.0`, shorter than the `1.4` leg swing). The right arm (part offset
/// `x < 0`) is the half-cycle out of phase (`cos(... + π)`) and the left arm in phase —
/// the opposite phasing to the same-side leg, the natural walking counter-swing. The
/// base arm pose carries no `xRot`, so it is set (not accumulated). Vanilla also divides
/// by `state.speedValue` (`1.0` except in the deferred crouch/swim/elytra poses) and
/// layers the `ageInTicks` idle bob ([`humanoid_arm_bob_pose`], applied separately on
/// top) and the held-item/attack/crouch/swim arm poses on top — the latter still
/// deferred because the client does not yet track that state. This helper is also reused
/// by the pillager's separate arms and the enderman (which are not `HumanoidModel` and so
/// do *not* get the idle bob), so the bob is kept out of this swing helper.
pub(in crate::entity_models) fn humanoid_arm_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let phase = walk_animation_pos * 0.6662;
    let angle = if base.offset[0] < 0.0 {
        phase + std::f32::consts::PI
    } else {
        phase
    };
    PartPose {
        offset: base.offset,
        rotation: [
            angle.cos() * 2.0 * walk_animation_speed * 0.5,
            base.rotation[1],
            base.rotation[2],
        ],
    }
}

/// Vanilla `HumanoidModel.setupAnim` idle arm bob (`AnimationUtils.bobModelPart`),
/// applied to both arms on top of the walk swing (and any pose). The right arm (part
/// offset `x < 0`, vanilla `bobModelPart(rightArm, ageInTicks, 1.0)`) accumulates
/// `+ (cos(ageInTicks * 0.09) * 0.05 + 0.05)` onto `zRot` and `+ sin(ageInTicks * 0.067)
/// * 0.05` onto `xRot`; the left arm (offset `x >= 0`, vanilla scale `-1.0`) subtracts the
/// same. Vanilla skips it only for the `SPYGLASS` arm pose (a held spyglass the client
/// does not track), so it is unconditional here — and because `ageInTicks` advances every
/// frame, the arms never sit perfectly still (there is no static rest fast path). The
/// offset and `yRot` are preserved; `xRot`/`zRot` are accumulated onto whatever the swing
/// (or rest) left, matching vanilla's `+=`.
pub(in crate::entity_models) fn humanoid_arm_bob_pose(
    base: PartPose,
    age_in_ticks: f32,
) -> PartPose {
    let scale = if base.offset[0] < 0.0 { 1.0 } else { -1.0 };
    PartPose {
        offset: base.offset,
        rotation: [
            base.rotation[0] + scale * ((age_in_ticks * 0.067).sin() * 0.05),
            base.rotation[1],
            base.rotation[2] + scale * ((age_in_ticks * 0.09).cos() * 0.05 + 0.05),
        ],
    }
}

/// Vanilla `HumanoidModel.setupAnim` arm + leg walk swing applied to a model root's named children.
/// The legs (`right_leg`/`left_leg`) swing ([`humanoid_leg_swing_pose`]) and the arms
/// (`right_arm`/`left_arm`) swing ([`humanoid_arm_swing_pose`]) only while moving, but the arms ALWAYS
/// carry the continuous idle bob ([`humanoid_arm_bob_pose`]). The swing/bob resolves each limb's phase
/// from its own offset, so the names may be declared in any order (the baby layouts swap head/body but
/// keep the same arm/leg names).
pub(in crate::entity_models) fn apply_humanoid_walk(
    root: &mut ModelPart,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
    age_in_ticks: f32,
) {
    let swinging = !limb_swing_at_rest(walk_animation_speed);
    if swinging {
        for name in ["right_leg", "left_leg"] {
            let leg = root.child_mut(name);
            leg.pose = humanoid_leg_swing_pose(leg.pose, walk_animation_pos, walk_animation_speed);
        }
    }
    for name in ["right_arm", "left_arm"] {
        let arm = root.child_mut(name);
        let mut pose = arm.pose;
        if swinging {
            pose = humanoid_arm_swing_pose(pose, walk_animation_pos, walk_animation_speed);
        }
        arm.pose = humanoid_arm_bob_pose(pose, age_in_ticks);
    }
}

/// Vanilla `HumanoidModel.setupAttackAnimation` (the default WHACK melee swing): driven by the
/// projected `attack_anim` (`LivingEntity.getAttackAnim(partialTick)`, `0..1`), it twists the body,
/// swings both arm anchors around it, then whacks the attacking arm down across the swing. The body
/// `yRot` is `sin(sqrt(attackTime) · 2π) · 0.2` (negated for the off arm); each arm's anchor is
/// repositioned to `x = ∓cos(bodyYRot) · 5 · ageScale`, `z = ±sin(bodyYRot) · 5 · ageScale` and gains
/// `yRot += bodyYRot` (the left arm also `xRot += bodyYRot`); then the attacking arm
/// (`attack_arm_off_hand` → left) gets `xRot -= sin(outQuart(t) · π) · 1.2 + sin(t · π) · -(headXRot -
/// 0.7) · 0.75`, `yRot += bodyYRot · 2`, `zRot += sin(t · π) · -0.4`. `head_pitch_degrees` is the head
/// look pitch (vanilla `head.xRot`); `age_scale` is the model scale (`1.0` adult). A no-op at
/// `attack_anim <= 0`. Runs LAST — after the walk swing / arm pose — accumulating onto their rotations
/// and overwriting the arm anchor offsets. The per-item STAB / NONE swing types are deferred.
pub(in crate::entity_models) fn apply_humanoid_attack_animation(
    root: &mut ModelPart,
    attack_anim: f32,
    attack_arm_off_hand: bool,
    head_pitch_degrees: f32,
    age_scale: f32,
) {
    if attack_anim <= 0.0 {
        return;
    }
    use std::f32::consts::PI;
    let mut body_yrot = (attack_anim.sqrt() * PI * 2.0).sin() * 0.2;
    if attack_arm_off_hand {
        body_yrot = -body_yrot;
    }
    root.child_mut("body").pose.rotation[1] = body_yrot;
    {
        let right = root.child_mut("right_arm");
        right.pose.offset[0] = -body_yrot.cos() * 5.0 * age_scale;
        right.pose.offset[2] = body_yrot.sin() * 5.0 * age_scale;
        right.pose.rotation[1] += body_yrot;
    }
    {
        let left = root.child_mut("left_arm");
        left.pose.offset[0] = body_yrot.cos() * 5.0 * age_scale;
        left.pose.offset[2] = -body_yrot.sin() * 5.0 * age_scale;
        left.pose.rotation[1] += body_yrot;
        left.pose.rotation[0] += body_yrot;
    }
    // WHACK: the attacking arm raises (`outQuart`-eased) and chops down across the swing.
    let swing = 1.0 - (1.0 - attack_anim).powi(4); // Ease.outQuart
    let raise = (swing * PI).sin();
    let head_pitch = head_pitch_degrees.to_radians();
    let head_term = (attack_anim * PI).sin() * -(head_pitch - 0.7) * 0.75;
    let attack_arm = root.child_mut(if attack_arm_off_hand {
        "left_arm"
    } else {
        "right_arm"
    });
    attack_arm.pose.rotation[0] -= raise * 1.2 + head_term;
    attack_arm.pose.rotation[1] += body_yrot * 2.0;
    attack_arm.pose.rotation[2] += (attack_anim * PI).sin() * -0.4;
}

/// Vanilla `HumanoidModel.setupAttackAnimation` `STAB` branch (`SpearAnimations.thirdPersonAttackHand`): a
/// spear lunges instead of chopping. The shared prologue (the `WHACK` body twist + arm-anchor reposition,
/// see [`apply_humanoid_attack_animation`]) runs first; the stab then *undoes* the prologue's
/// `arm.yRot += body.yRot` / `leftArm.xRot += body.yRot` additions (they cancel exactly), leaving the arm
/// rotations free of the twist, and drives the attacking arm's pitch through a lunge:
/// `xRot += (90·prepare − 120·attack + 30·retract)·π/180`, where `prepare = inOutSine(progress(t, 0, 0.05))`,
/// `attack = inQuad(progress(t, 0.05, 0.2))`, `retract = inOutExpo(progress(t, 0.4, 1.0))` and
/// `progress(t, a, b) = clamp((t − a)/(b − a), 0, 1)`. `t = attack_anim`. The per-tick `thirdPersonHandUse`
/// hold sway (`KINETIC_WEAPON`/`ticksUsingItem`) is a separate path and stays deferred.
pub(in crate::entity_models) fn apply_humanoid_stab_attack_animation(
    root: &mut ModelPart,
    attack_anim: f32,
    attack_arm_off_hand: bool,
    age_scale: f32,
) {
    if attack_anim <= 0.0 {
        return;
    }
    use std::f32::consts::PI;
    let mut body_yrot = (attack_anim.sqrt() * PI * 2.0).sin() * 0.2;
    if attack_arm_off_hand {
        body_yrot = -body_yrot;
    }
    root.child_mut("body").pose.rotation[1] = body_yrot;
    // The shared prologue repositions the arm anchors (`x = ∓cos·5`, `z = ±sin·5`). The prologue's
    // `arm.yRot += body.yRot` (both arms) and `leftArm.xRot += body.yRot` additions are immediately undone
    // by `thirdPersonAttackHand`, so they net to zero and are omitted here.
    {
        let right = root.child_mut("right_arm");
        right.pose.offset[0] = -body_yrot.cos() * 5.0 * age_scale;
        right.pose.offset[2] = body_yrot.sin() * 5.0 * age_scale;
    }
    {
        let left = root.child_mut("left_arm");
        left.pose.offset[0] = body_yrot.cos() * 5.0 * age_scale;
        left.pose.offset[2] = -body_yrot.sin() * 5.0 * age_scale;
    }
    let prepare = ease_in_out_sine(progress(attack_anim, 0.0, 0.05));
    let attack = progress(attack_anim, 0.05, 0.2).powi(2); // Ease.inQuad
    let retract = ease_in_out_expo(progress(attack_anim, 0.4, 1.0));
    let attack_arm = root.child_mut(if attack_arm_off_hand {
        "left_arm"
    } else {
        "right_arm"
    });
    attack_arm.pose.rotation[0] += (90.0 * prepare - 120.0 * attack + 30.0 * retract).to_radians();
}

/// Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` `SPYGLASS` use-item arm pose: while a player is
/// using a spyglass, the holding arm raises it to the eye along the head look. The arm pitch is
/// `clamp(head.xRot − 1.9198622 − (crouch ? π/12 : 0), −2.4, 3.3)` and the yaw is `head.yRot ∓ π/12`
/// (right arm `− π/12`, left arm `+ π/12`). Vanilla also SKIPS the idle `bobModelPart` for the spyglass
/// arm, so this resets the arm roll (`zRot`) to its bind — undoing the bob that `apply_humanoid_walk`
/// already applied. Vanilla applies this BEFORE the crouch block, so the crouch `arm.xRot += 0.4` still
/// lands on top; call it before [`apply_humanoid_crouch_named`]. `head_*_degrees` are the head look
/// (vanilla `head.xRot`/`head.yRot`).
pub(in crate::entity_models) fn apply_humanoid_spyglass_pose(
    root: &mut ModelPart,
    head_yaw_degrees: f32,
    head_pitch_degrees: f32,
    off_hand: bool,
    is_crouching: bool,
) {
    use std::f32::consts::PI;
    let head_yaw = head_yaw_degrees.to_radians();
    let head_pitch = head_pitch_degrees.to_radians();
    let crouch = if is_crouching { PI / 12.0 } else { 0.0 };
    let x_rot = (head_pitch - 1.9198622 - crouch).clamp(-2.4, 3.3);
    let (arm_name, y_rot) = if off_hand {
        ("left_arm", head_yaw + PI / 12.0)
    } else {
        ("right_arm", head_yaw - PI / 12.0)
    };
    let arm = root.child_mut(arm_name);
    arm.pose.rotation[0] = x_rot;
    arm.pose.rotation[1] = y_rot;
    arm.pose.rotation[2] = 0.0;
}

/// Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` `TOOT_HORN` use-item arm pose: while a player is
/// tooting a goat horn, the holding arm raises it to the mouth — `xRot = clamp(head.xRot, −1.2, 1.2) −
/// 1.4835298`, `yRot = head.yRot ± π/6` (right arm `− π/6`, left arm `+ π/6`). Unlike `SPYGLASS`, vanilla
/// keeps the idle bob on this arm, so the bob's roll (`zRot`, already applied by `apply_humanoid_walk`)
/// is left in place. Applied before the crouch block so the crouch `arm.xRot += 0.4` still lands on top.
pub(in crate::entity_models) fn apply_humanoid_toot_horn_pose(
    root: &mut ModelPart,
    head_yaw_degrees: f32,
    head_pitch_degrees: f32,
    off_hand: bool,
) {
    use std::f32::consts::PI;
    let head_yaw = head_yaw_degrees.to_radians();
    let head_pitch = head_pitch_degrees.to_radians();
    let x_rot = head_pitch.clamp(-1.2, 1.2) - 1.4835298;
    let (arm_name, y_rot) = if off_hand {
        ("left_arm", head_yaw + PI / 6.0)
    } else {
        ("right_arm", head_yaw - PI / 6.0)
    };
    let arm = root.child_mut(arm_name);
    arm.pose.rotation[0] = x_rot;
    arm.pose.rotation[1] = y_rot;
}

/// Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` `BRUSH` use-item arm pose (also the generic `ITEM`
/// pose with `π/10` instead of `π/5`): while a player is brushing, the holding arm lowers toward the
/// block — `xRot = arm.xRot · 0.5 − π/5`, `yRot = 0`. Unlike the spyglass/horn this READS the arm's
/// current pitch and halves it (the vanilla pose runs before the idle bob, which bbb folds into
/// `apply_humanoid_walk`, so the halved value carries bbb's small bob component — the shared posed-arm
/// bob convention, see the renderer-scene-parity note). Applied before the crouch block.
pub(in crate::entity_models) fn apply_humanoid_brush_pose(root: &mut ModelPart, off_hand: bool) {
    use std::f32::consts::PI;
    let arm_name = if off_hand { "left_arm" } else { "right_arm" };
    let arm = root.child_mut(arm_name);
    arm.pose.rotation[0] = arm.pose.rotation[0] * 0.5 - PI / 5.0;
    arm.pose.rotation[1] = 0.0;
}

/// Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` generic `ITEM` arm pose (the `AvatarRenderer.getArmPose`
/// fallback for a player holding any plain item): the holding arm lowers and halves its swing —
/// `xRot = arm.xRot · 0.5 − π/10`, `yRot = 0`. Like the brush pose this READS the arm's current pitch and
/// halves it (vanilla runs it before the idle bob, which bbb folds into `apply_humanoid_walk`, so the halved
/// value carries bbb's small bob component — the shared posed-arm bob convention, see the
/// renderer-scene-parity note). Applied before the crouch/attack blocks (vanilla `poseArm` precedes
/// `setupAttackAnimation`, and crouch/attack commute as additive offsets).
pub(in crate::entity_models) fn apply_humanoid_item_hold_pose(
    root: &mut ModelPart,
    off_hand: bool,
) {
    use std::f32::consts::PI;
    let arm_name = if off_hand { "left_arm" } else { "right_arm" };
    let arm = root.child_mut(arm_name);
    arm.pose.rotation[0] = arm.pose.rotation[0] * 0.5 - PI / 10.0;
    arm.pose.rotation[1] = 0.0;
}

/// Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` `BLOCK` use-item arm pose (`poseBlockingArm`): while a
/// player is raising a shield (`isUsingItem` + the using hand holds an item whose use-animation is `BLOCK`,
/// i.e. carries `DataComponents.BLOCKS_ATTACKS` — the shield), the holding arm tucks the shield forward along
/// the head look. The arm pitch is `arm.xRot · 0.5 − 0.9424779 + clamp(head.xRot, −4π/9, 0.43633232)` and the
/// yaw is `(right ? −π/6 : π/6) + clamp(head.yRot, −π/6, π/6)` (vanilla's `±30°` in radians). Like the brush
/// pose this READS the arm's current pitch and halves it (the folded-in idle bob rides along — the shared
/// posed-arm bob convention); the bob roll (`zRot`) is kept (vanilla applies the bob after `poseArm` and does
/// NOT skip it for BLOCK). Applied before the crouch block so the crouch `arm.xRot += 0.4` still lands on top.
pub(in crate::entity_models) fn apply_humanoid_block_pose(
    root: &mut ModelPart,
    head_yaw_degrees: f32,
    head_pitch_degrees: f32,
    off_hand: bool,
) {
    use std::f32::consts::PI;
    let head_yaw = head_yaw_degrees.to_radians();
    let head_pitch = head_pitch_degrees.to_radians();
    let x_clamp = head_pitch.clamp(-PI * 4.0 / 9.0, 0.43633232);
    let y_clamp = head_yaw.clamp(-PI / 6.0, PI / 6.0);
    let (arm_name, y_base) = if off_hand {
        ("left_arm", PI / 6.0)
    } else {
        ("right_arm", -PI / 6.0)
    };
    let arm = root.child_mut(arm_name);
    arm.pose.rotation[0] = arm.pose.rotation[0] * 0.5 - 0.9424779 + x_clamp;
    arm.pose.rotation[1] = y_base + y_clamp;
}

/// Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` `THROW_TRIDENT` use-item arm pose: while a player
/// charges a trident throw, the holding arm raises the trident straight overhead — `xRot = arm.xRot · 0.5 −
/// π`, `yRot = 0`. Like the brush/item poses this READS the arm's current pitch and halves it (the folded-in
/// idle bob rides along — the shared posed-arm bob convention). This is the same pose the drowned reaches
/// (`apply_drowned_throw_trident`, hard-coded to the main arm), generalized to either hand for the player's
/// use-item path. Applied before the crouch block so the crouch `arm.xRot += 0.4` still lands on top.
pub(in crate::entity_models) fn apply_humanoid_throw_trident_pose(
    root: &mut ModelPart,
    off_hand: bool,
) {
    use std::f32::consts::PI;
    let arm_name = if off_hand { "left_arm" } else { "right_arm" };
    let arm = root.child_mut(arm_name);
    arm.pose.rotation[0] = arm.pose.rotation[0] * 0.5 - PI;
    arm.pose.rotation[1] = 0.0;
}

/// Vanilla `HumanoidModel.poseRightArm` `BOW_AND_ARROW` use-item arm pose: while a player draws a main-hand
/// bow, BOTH arms raise forward along the head look — `rightArm.xRot = leftArm.xRot = −π/2 + head.xRot`,
/// `rightArm.yRot = −0.1 + head.yRot`, `leftArm.yRot = 0.1 + head.yRot + 0.4`. The pose is two-handed +
/// `affectsOffhandPose`, so vanilla's `poseRightArm` sets both arms and `poseLeftArm` never runs (the
/// projection suppresses the off-hand `ITEM` fallback to match). These are SET (not halved/added), so they
/// overwrite the walk-swing pitch/yaw; the bob roll (`zRot`) is left in place. Only the right-handed
/// main-hand draw is posed; the mirrored off-hand draw stays deferred. Applied before the crouch block.
pub(in crate::entity_models) fn apply_humanoid_bow_pose(
    root: &mut ModelPart,
    head_yaw_degrees: f32,
    head_pitch_degrees: f32,
) {
    use std::f32::consts::PI;
    let head_yaw = head_yaw_degrees.to_radians();
    let head_pitch = head_pitch_degrees.to_radians();
    let right = root.child_mut("right_arm");
    right.pose.rotation[0] = -PI / 2.0 + head_pitch;
    right.pose.rotation[1] = -0.1 + head_yaw;
    let left = root.child_mut("left_arm");
    left.pose.rotation[0] = -PI / 2.0 + head_pitch;
    left.pose.rotation[1] = 0.1 + head_yaw + 0.4;
}

/// Vanilla `Mth.clamp(Mth.inverseLerp(t, a, b), 0, 1)`: the normalized `0..1` position of `t` in `[a, b]`.
fn progress(t: f32, a: f32, b: f32) -> f32 {
    ((t - a) / (b - a)).clamp(0.0, 1.0)
}

/// Vanilla `Ease.inOutSine`: `-(cos(π·x) − 1) / 2`.
fn ease_in_out_sine(x: f32) -> f32 {
    -((std::f32::consts::PI * x).cos() - 1.0) / 2.0
}

/// Vanilla `Ease.inOutExpo`: an exponential ease that snaps through the midpoint, with the `x == 0`/`x == 1`
/// endpoints pinned to `0`/`1` (the `pow` would otherwise miss them by a hair).
fn ease_in_out_expo(x: f32) -> f32 {
    if x < 0.5 {
        if x == 0.0 {
            0.0
        } else {
            2.0_f32.powf(20.0 * x - 10.0) / 2.0
        }
    } else if x == 1.0 {
        1.0
    } else {
        (2.0 - 2.0_f32.powf(-20.0 * x + 10.0)) / 2.0
    }
}

/// Vanilla `AnimationUtils.swingWeaponDown(rightArm, leftArm, mainArm = RIGHT, attackTime, ageInTicks)`:
/// the main (right) arm raises overhead (`xRot = -1.8849558` + a `cos(age·0.09)·0.15` idle wobble) and
/// chops down with the attack (`+= sin(t·π)·2.2 - sin((1-(1-t)²)·π)·0.4`), the off (left) arm trails
/// (`xRot = cos(age·0.19)·0.5 += sin(t·π)·1.2 - …·0.4`), both yawing slightly apart (`±π/20`, `zRot = 0`)
/// with the shared idle bob ([`humanoid_arm_bob_pose`]) on top. `t = attack_anim`; at rest (`t = 0`) the
/// weapon holds raised. Shared by every humanoid that swings a melee weapon overhead (the vindicator
/// `ATTACKING` axe, the piglin/brute `ATTACKING_WITH_MELEE_WEAPON`). Left-handed mobs (`mainArm = LEFT`)
/// are not projected, so this always takes the right-handed branch.
pub(in crate::entity_models) fn apply_humanoid_weapon_swing_down(
    root: &mut ModelPart,
    attack_anim: f32,
    age_in_ticks: f32,
) {
    use std::f32::consts::PI;
    let attack2 = (attack_anim * PI).sin();
    let attack = ((1.0 - (1.0 - attack_anim) * (1.0 - attack_anim)) * PI).sin();
    {
        let right = root.child_mut("right_arm");
        right.pose.rotation = [
            -1.8849558 + (age_in_ticks * 0.09).cos() * 0.15 + (attack2 * 2.2 - attack * 0.4),
            PI / 20.0,
            0.0,
        ];
        right.pose = humanoid_arm_bob_pose(right.pose, age_in_ticks);
    }
    {
        let left = root.child_mut("left_arm");
        left.pose.rotation = [
            (age_in_ticks * 0.19).cos() * 0.5 + (attack2 * 1.2 - attack * 0.4),
            -PI / 20.0,
            0.0,
        ];
        left.pose = humanoid_arm_bob_pose(left.pose, age_in_ticks);
    }
}

/// Vanilla `AnimationUtils.animateCrossbowHold(rightArm, leftArm, head, holdingInRightArm = true)`: the
/// right (holding) arm levels the crossbow along the head look (`yRot = -0.3 + head.yRot`,
/// `xRot = -π/2 + head.xRot + 0.1`) while the left (shooting) arm reaches across to the trigger
/// (`yRot = 0.6 + head.yRot`, `xRot = -1.5 + head.xRot`). Vanilla sets these absolutely after the walk
/// swing (which zeroed `zRot`), so the roll is preserved. Shared by every humanoid that levels a crossbow
/// (pillager, piglin). `head_yaw_degrees` / `head_pitch_degrees` are the net head look (vanilla
/// `head.yRot` / `head.xRot`).
pub(in crate::entity_models) fn apply_crossbow_hold_pose(
    root: &mut ModelPart,
    head_yaw_degrees: f32,
    head_pitch_degrees: f32,
) {
    let head_yaw = head_yaw_degrees.to_radians();
    let head_pitch = head_pitch_degrees.to_radians();
    let right = root.child_mut("right_arm");
    right.pose.rotation = [
        -std::f32::consts::FRAC_PI_2 + head_pitch + 0.1,
        -0.3 + head_yaw,
        right.pose.rotation[2],
    ];
    let left = root.child_mut("left_arm");
    left.pose.rotation = [-1.5 + head_pitch, 0.6 + head_yaw, left.pose.rotation[2]];
}

/// Vanilla `AnimationUtils.animateCrossbowCharge(rightArm, leftArm, maxChargeDuration, ticksUsingItem,
/// holdingInRightArm = true)`: the right (holding) arm braces the crossbow (`yRot = -0.8`,
/// `xRot = -0.97079635`) while the left (pulling) arm draws the string back over the charge — its `xRot`
/// lerps `-0.97079635 → -π/2` and its `yRot` lerps `0.4 → 0.85` as `ticksUsingItem / maxChargeDuration`
/// climbs `0 → 1` (clamped). Shared by every humanoid that draws a crossbow (the pillager and the regular
/// piglin). `max_charge_duration` is the vanilla `CrossbowItem.getChargeDuration` ([`CROSSBOW_CHARGE_DURATION_TICKS`],
/// 25 ticks without a Quick Charge enchant — the rare enchant is not projected, a minor draw-speed
/// simplification).
pub(in crate::entity_models) const CROSSBOW_CHARGE_DURATION_TICKS: f32 = 25.0;

pub(in crate::entity_models) fn apply_crossbow_charge_pose(
    root: &mut ModelPart,
    max_charge_duration: f32,
    ticks_using_item: f32,
) {
    const HOLD_X_ROT: f32 = -0.97079635;
    let lerp_alpha = (ticks_using_item.clamp(0.0, max_charge_duration)) / max_charge_duration;
    {
        let right = root.child_mut("right_arm");
        right.pose.rotation[1] = -0.8;
        right.pose.rotation[0] = HOLD_X_ROT;
    }
    let left = root.child_mut("left_arm");
    left.pose.rotation[1] = 0.4 + (0.85 - 0.4) * lerp_alpha;
    left.pose.rotation[0] = HOLD_X_ROT + (-std::f32::consts::FRAC_PI_2 - HOLD_X_ROT) * lerp_alpha;
}

/// Vanilla `HumanoidModel.setupAnim` crouch (`isCrouching`) sneaking pose applied to a humanoid model
/// root by name: the body (`body`) leans forward and drops, the head (`head`) drops with it, the arms
/// (`right_arm`/`left_arm`) tilt forward and ride down, and the legs (`right_leg`/`left_leg`) tuck back.
/// Applied after the walk swing and idle bob, so callers run it last and only while crouching.
pub(in crate::entity_models) fn apply_humanoid_crouch_named(root: &mut ModelPart) {
    let head = root.child_mut("head");
    head.pose = humanoid_crouch_head_pose(head.pose);
    let body = root.child_mut("body");
    body.pose = humanoid_crouch_body_pose(body.pose);
    for name in ["right_arm", "left_arm"] {
        let arm = root.child_mut(name);
        arm.pose = humanoid_crouch_arm_pose(arm.pose);
    }
    for name in ["right_leg", "left_leg"] {
        let leg = root.child_mut(name);
        leg.pose = humanoid_crouch_leg_pose(leg.pose);
    }
}

/// Vanilla `HumanoidModel.setupAnim` leg swing only, applied to a model root's named leg children
/// `right_leg`/`left_leg` ([`humanoid_leg_swing_pose`]). A no-op while at rest. Used by the zombified
/// piglin and the zombie family, whose arms are overridden by a held-out pose instead of swinging.
pub(in crate::entity_models) fn apply_humanoid_leg_swing_named(
    root: &mut ModelPart,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) {
    if limb_swing_at_rest(walk_animation_speed) {
        return;
    }
    for name in ["right_leg", "left_leg"] {
        let leg = root.child_mut(name);
        leg.pose = humanoid_leg_swing_pose(leg.pose, walk_animation_pos, walk_animation_speed);
    }
}

/// Vanilla `ZombieModel.setupAnim` held-out arm override applied to a model root's named arm children
/// `right_arm`/`left_arm` ([`zombie_arm_held_out_pose`]). The zombie family (zombie, husk, drowned,
/// zombie villager) replaces the inherited humanoid arm swing with this aggressive/idle held-out pose,
/// which always re-poses the arms (the idle bob folded in advances every frame). `attack_anim` swings
/// the held-out arms during a melee strike.
pub(in crate::entity_models) fn apply_zombie_arms_held_out_named(
    root: &mut ModelPart,
    aggressive: bool,
    attack_anim: f32,
    age_in_ticks: f32,
) {
    for name in ["right_arm", "left_arm"] {
        let arm = root.child_mut(name);
        arm.pose = zombie_arm_held_out_pose(arm.pose, aggressive, attack_anim, age_in_ticks);
    }
}

/// Vanilla `HumanoidModel.setupAnim` crouch (`isCrouching`) head drop: `head.y += 4.2`, so the
/// sneaking head sinks with the lowered body. Applied after the look/swing/bob.
pub(in crate::entity_models) fn humanoid_crouch_head_pose(base: PartPose) -> PartPose {
    PartPose {
        offset: [base.offset[0], base.offset[1] + 4.2, base.offset[2]],
        rotation: base.rotation,
    }
}

/// Vanilla `HumanoidModel.setupAnim` crouch body lean: `body.xRot = 0.5` (set, the standing
/// body has no pitch) and `body.y += 3.2`, so the torso tilts forward and drops.
pub(in crate::entity_models) fn humanoid_crouch_body_pose(base: PartPose) -> PartPose {
    PartPose {
        offset: [base.offset[0], base.offset[1] + 3.2, base.offset[2]],
        rotation: [0.5, base.rotation[1], base.rotation[2]],
    }
}

/// Vanilla `HumanoidModel.setupAnim` crouch arm pose: `arm.xRot += 0.4` (accumulated onto the
/// swing and idle bob) and `arm.y += 3.2`, so the arms tilt forward and ride the lowered body.
pub(in crate::entity_models) fn humanoid_crouch_arm_pose(base: PartPose) -> PartPose {
    PartPose {
        offset: [base.offset[0], base.offset[1] + 3.2, base.offset[2]],
        rotation: [base.rotation[0] + 0.4, base.rotation[1], base.rotation[2]],
    }
}

/// Vanilla `HumanoidModel.setupAnim` crouch leg pose: `leg.z += 4.0`, so the legs tuck back
/// under the lowered body. The offset is shifted; the leg swing rotation is preserved.
pub(in crate::entity_models) fn humanoid_crouch_leg_pose(base: PartPose) -> PartPose {
    PartPose {
        offset: [base.offset[0], base.offset[1], base.offset[2] + 4.0],
        rotation: base.rotation,
    }
}

/// Vanilla `AnimationUtils.animateZombieArms` held-out zombie arms — the iconic forward reach. Each arm
/// drops to `xRot = armDrop`, splays by `yRot` (right arm, part offset `x < 0`, `-0.1`; left arm `+0.1`),
/// zeroes `zRot`, then takes the idle bob (`bobArms` → [`humanoid_arm_bob_pose`]). `armDrop =
/// -π / (aggressive ? 1.5 : 2.25)` — an aggressive mob (`Mob.isAggressive`, `DATA_MOB_FLAGS_ID & 4`,
/// projected as `is_aggressive`) raises its arms higher. A melee strike (`attack_anim` =
/// `getAttackAnim(partialTick) > 0`) swings the held-out arms: `attackYRot = sin(t·π)` lifts both arms'
/// yRot toward center (`yRot = ±(0.1 - attackYRot·0.6)`) and `xRot += attackYRot·1.2 - sin((1-(1-t)²)·π)
/// ·0.4` chops them down then back. `ZombieModel.setupAnim` calls this *after* the inherited
/// `HumanoidModel.setupAnim`, so it overrides the walk arm swing while the legs keep theirs (the held-out
/// values are set absolutely). The body twist / arm-anchor reposition from the inherited
/// `setupAttackAnimation` (and the per-item STAB swing-type skip) stay deferred for the zombie family.
pub(in crate::entity_models) fn zombie_arm_held_out_pose(
    base: PartPose,
    aggressive: bool,
    attack_anim: f32,
    age_in_ticks: f32,
) -> PartPose {
    use std::f32::consts::PI;
    let arm_drop = -PI / if aggressive { 1.5 } else { 2.25 };
    let attack_y_rot = (attack_anim * PI).sin();
    let attack_x_rot = ((1.0 - (1.0 - attack_anim) * (1.0 - attack_anim)) * PI).sin();
    let side = if base.offset[0] < 0.0 { -1.0 } else { 1.0 };
    let held_out = PartPose {
        offset: base.offset,
        rotation: [
            arm_drop + attack_y_rot * 1.2 - attack_x_rot * 0.4,
            side * (0.1 - attack_y_rot * 0.6),
            0.0,
        ],
    };
    humanoid_arm_bob_pose(held_out, age_in_ticks)
}

/// `AbstractPiglinModel.ADULT_EAR_ANGLE_IN_DEGREES`/`BABY_EAR_ANGLE_IN_DEGREES` in radians
/// (`getDefaultEarAngleInDegrees() * π/180`): `30°` for the adult piglin/brute/zombified
/// piglin, `5°` for the babies.
pub(in crate::entity_models) const PIGLIN_ADULT_EAR_ANGLE: f32 =
    30.0 * std::f32::consts::PI / 180.0;
pub(in crate::entity_models) const PIGLIN_BABY_EAR_ANGLE: f32 = 5.0 * std::f32::consts::PI / 180.0;

/// Vanilla `AbstractPiglinModel.setupAnim` ear flap (shared by every piglin/zombified
/// piglin subclass via `super.setupAnim`). The ears sway continuously about `zRot` from a
/// frequency `ageInTicks * 0.1 + walkAnimationPos * 0.5` and an amplitude `0.08 +
/// walkAnimationSpeed * 0.4`, *set* absolutely onto the per-model default ear angle
/// (`default_ear_angle`, [`PIGLIN_ADULT_EAR_ANGLE`]/[`PIGLIN_BABY_EAR_ANGLE`]):
/// `leftEar.zRot = -default - cos(freq * 1.2) * amp`, `rightEar.zRot = default + cos(freq)
/// * amp` — note the left ear's `× 1.2` frequency. The `±0.08` baseline already differs
/// from the layer rest and `ageInTicks` advances every frame, so the ears never sit still;
/// `offset` and `xRot`/`yRot` are preserved.
pub(in crate::entity_models) fn piglin_ear_flap_pose(
    base: PartPose,
    left: bool,
    default_ear_angle: f32,
    age_in_ticks: f32,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let frequency = age_in_ticks * 0.1 + walk_animation_pos * 0.5;
    let amplitude = 0.08 + walk_animation_speed * 0.4;
    let z_rot = if left {
        -default_ear_angle - (frequency * 1.2).cos() * amplitude
    } else {
        default_ear_angle + frequency.cos() * amplitude
    };
    PartPose {
        offset: base.offset,
        rotation: [base.rotation[0], base.rotation[1], z_rot],
    }
}

/// Vanilla `WitchModel.setupAnim` idle nose bob: a per-entity `speed = 0.01 * (entityId %
/// 10)` drives `nose.xRot = sin(ageInTicks * speed) * 4.5°` and `nose.zRot =
/// cos(ageInTicks * speed) * 2.5°` (degrees → radians), *set* absolutely (overriding the
/// layer's zeroed rest) and preserving the offset and `yRot`. Because `cos(0) = 1`, the
/// nose carries a constant `+2.5°` zRot baseline even at `ageInTicks = 0`, and `ageInTicks`
/// advances every frame, so the nose never sits at the layer rest. The `isHoldingItem`
/// drinking pose runs after this helper in `WitchModel.setupAnim`, preserving this bob's
/// `zRot` while replacing the offset and `xRot`.
pub(in crate::entity_models) fn witch_nose_bob_pose(
    base: PartPose,
    age_in_ticks: f32,
    entity_id: i32,
) -> PartPose {
    let speed = 0.01 * (entity_id % 10) as f32;
    let phase = age_in_ticks * speed;
    PartPose {
        offset: base.offset,
        rotation: [
            phase.sin() * 4.5_f32.to_radians(),
            base.rotation[1],
            phase.cos() * 2.5_f32.to_radians(),
        ],
    }
}

/// Vanilla half-amplitude leg swing for a single leg part: `leg.xRot =
/// cos(walkAnimationPos * 0.6662 [+ π]) * 1.4 * walkAnimationSpeed * 0.5`. The
/// `EntityModel` bipeds that are not `HumanoidModel` — `IllagerModel` (non-riding
/// branch), `VillagerModel`, and `WitchModel` — all apply this same extra `0.5`
/// amplitude factor that `HumanoidModel` does not, so they share this pose helper
/// rather than reusing [`humanoid_leg_swing_pose`]. The phase rule is the same: the
/// right leg (part offset `x < 0`) is in phase and the left leg a half-cycle out of
/// phase, both legs sitting at `z = 0`. The base leg pose carries no `xRot`, so it
/// is set (not accumulated). The `IllagerModel` riding sit pose (fixed `-1.4137167`
/// with leg yaw/roll splay) is a separate deferred pose.
pub(in crate::entity_models) fn half_amplitude_leg_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let phase = walk_animation_pos * 0.6662;
    let angle = if base.offset[0] < 0.0 {
        phase
    } else {
        phase + std::f32::consts::PI
    };
    PartPose {
        offset: base.offset,
        rotation: [
            angle.cos() * 1.4 * walk_animation_speed * 0.5,
            base.rotation[1],
            base.rotation[2],
        ],
    }
}

/// Vanilla villager-family (`VillagerModel`/`IllagerModel`/`WitchModel`) leg swing applied to a model
/// root's two named leg children `right_leg`/`left_leg` ([`half_amplitude_leg_swing_pose`]). Shared by
/// the villager-family models, which build a unified tree with the vanilla child names. A no-op while
/// the limbs are at rest (`walkAnimationSpeed == 0`). The swing resolves each leg's phase from its own
/// offset, so the two names may be declared in any order.
pub(in crate::entity_models) fn apply_half_amplitude_leg_swing(
    root: &mut ModelPart,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) {
    if limb_swing_at_rest(walk_animation_speed) {
        return;
    }
    for name in ["right_leg", "left_leg"] {
        let leg = root.child_mut(name);
        leg.pose =
            half_amplitude_leg_swing_pose(leg.pose, walk_animation_pos, walk_animation_speed);
    }
}

/// Vanilla `HoglinModel.setupAnim` leg swing for a single leg part: `leg.xRot =
/// cos(walkAnimationPos [+ π]) * 1.2 * walkAnimationSpeed`. `HoglinModel` is a custom
/// `EntityModel` (zoglin shares it) with its own formula — amplitude `1.2` (not the
/// `QuadrupedModel` `1.4`) and no `0.6662` frequency factor. The right-front and
/// left-hind legs are in phase (`cos(pos)`) and the left-front and right-hind a
/// half-cycle out (`cos(pos + π)`); that diagonal pairing is exactly the legs whose
/// part offset satisfies `x * z > 0` (right-front is `x < 0, z < 0`; left-hind is
/// `x > 0, z > 0`), the opposite sign from the `QuadrupedModel` rule. The base leg
/// pose carries no `xRot`, so it is set (not accumulated). The ear sway and the
/// headbutt head tilt are separate deferred animations.
pub(in crate::entity_models) fn hoglin_leg_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let [x, _, z] = base.offset;
    let angle = if x * z > 0.0 {
        walk_animation_pos
    } else {
        walk_animation_pos + std::f32::consts::PI
    };
    PartPose {
        offset: base.offset,
        rotation: [
            angle.cos() * 1.2 * walk_animation_speed,
            base.rotation[1],
            base.rotation[2],
        ],
    }
}

/// Vanilla `EndermanModel.setupAnim` leg swing for a single leg part. `EndermanModel
/// extends HumanoidModel`, so `super.setupAnim` first sets `leg.xRot =
/// cos(walkAnimationPos * 0.6662 [+ π]) * 1.4 * walkAnimationSpeed`, then the enderman
/// halves it (`*= 0.5`) and clamps it to `[-0.4, 0.4]`. The phase rule is the
/// `HumanoidModel` one (the right leg, part offset `x < 0`, in phase; both legs at
/// `z = 0`). The base leg pose carries no `xRot`, so it is set (not accumulated). The
/// legs carry no idle bob (vanilla `bobModelPart` is arms-only); the arm swing+bob
/// halve/clamp, the carried-block arm pose, and the creepy head shift are separate.
pub(in crate::entity_models) fn enderman_leg_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let phase = walk_animation_pos * 0.6662;
    let angle = if base.offset[0] < 0.0 {
        phase
    } else {
        phase + std::f32::consts::PI
    };
    let x_rot = (angle.cos() * 1.4 * walk_animation_speed * 0.5).clamp(-0.4, 0.4);
    PartPose {
        offset: base.offset,
        rotation: [x_rot, base.rotation[1], base.rotation[2]],
    }
}

/// Vanilla `EndermanModel.setupAnim` arm pose for a single arm part. `EndermanModel
/// extends HumanoidModel`, so `super.setupAnim` first sets the inherited arm
/// counter-swing ([`humanoid_arm_swing_pose`]: `arm.xRot = cos(walkAnimationPos *
/// 0.6662 [+ π]) * 2.0 * walkAnimationSpeed * 0.5`, the right arm — part offset `x < 0`
/// — out of phase) AND the always-on idle bob ([`humanoid_arm_bob_pose`]: `xRot +=
/// scale * sin(ageInTicks * 0.067) * 0.05`, `zRot += scale * (cos(ageInTicks * 0.09) *
/// 0.05 + 0.05)`), THEN the enderman halves the accumulated `xRot` (`*= 0.5`) and clamps
/// it to `[-0.4, 0.4]` exactly as it does the legs. The halve/clamp touches only `xRot`
/// in vanilla, so the bob's `zRot` survives the clamp untouched (the arms gently splay).
/// The base arm pose carries no rotation, so the swing/bob set it. When the enderman is
/// carrying a block this pose is overridden entirely by [`enderman_carried_arm_pose`],
/// and the creepy head/hat shift rides the head part separately.
pub(in crate::entity_models) fn enderman_arm_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
    age_in_ticks: f32,
) -> PartPose {
    let swung = humanoid_arm_swing_pose(base, walk_animation_pos, walk_animation_speed);
    let bobbed = humanoid_arm_bob_pose(swung, age_in_ticks);
    let x_rot = (bobbed.rotation[0] * 0.5).clamp(-0.4, 0.4);
    PartPose {
        offset: base.offset,
        rotation: [x_rot, bobbed.rotation[1], bobbed.rotation[2]],
    }
}

/// Vanilla `EndermanModel.setupAnim` carried-block arm pose: when `!carriedBlock.isEmpty()`
/// both arms are *set* (overriding the swing and its clamp) to hold the block out front —
/// `xRot = -0.5` on both, and `zRot = +0.05` on the right arm (part offset `x < 0`) /
/// `-0.05` on the left so they cradle inward. `yRot` and the bind offset are preserved.
pub(in crate::entity_models) fn enderman_carried_arm_pose(base: PartPose) -> PartPose {
    let z_rot = if base.offset[0] < 0.0 { 0.05 } else { -0.05 };
    PartPose {
        offset: base.offset,
        rotation: [-0.5, base.rotation[1], z_rot],
    }
}

/// Vanilla `WolfModel.setupAnim` tail wag for the tail part. In its non-angry branch the
/// wolf sets `tail.yRot = cos(walkAnimationPos * 0.6662) * 1.4 * walkAnimationSpeed` — the
/// same `QuadrupedModel` swing amplitude as the legs, with no phase offset, so the tail
/// sweeps side to side in step with the gait. The caller takes this branch only for a
/// non-angry wolf; an angry one holds its tail straight and raised
/// ([`wolf_angry_tail_pose`]). Vanilla then unconditionally sets `tail.xRot =
/// state.tailAngle` (`Wolf.getTailAngle()`), so the wag *sets* `xRot` to `tail_angle`: the
/// `π/5` default for an untamed wolf (matching the layer's rest droop, leaving a wild wolf
/// unchanged) or the tame/health droop `(0.55 - damageRatio * 0.4) * π`. Only `xRot`/`yRot`
/// are written; the offset and `zRot` are preserved.
pub(in crate::entity_models) fn wolf_tail_swing_pose(
    base: PartPose,
    tail_angle: f32,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let y_rot = (walk_animation_pos * 0.6662).cos() * 1.4 * walk_animation_speed;
    PartPose {
        offset: base.offset,
        rotation: [tail_angle, y_rot, base.rotation[2]],
    }
}

/// Vanilla `WolfModel.setupAnim` angry tail: an angry wolf zeroes the wag (`tail.yRot = 0`)
/// and `getTailAngle()` returns the constant `1.5393804` (≈ 88°), so `tail.xRot` is *set*
/// to that raised angle — overriding the layer's `π/5` wild rest droop. The offset and
/// `zRot` are preserved. Driven by the `isAngry` render state (which the client tracks); the
/// tame/health droop and the sitting/water-shake poses remain deferred.
pub(in crate::entity_models) const WOLF_ANGRY_TAIL_X_ROT: f32 = 1.5393804;

pub(in crate::entity_models) fn wolf_angry_tail_pose(base: PartPose) -> PartPose {
    PartPose {
        offset: base.offset,
        rotation: [WOLF_ANGRY_TAIL_X_ROT, 0.0, base.rotation[2]],
    }
}

/// Which wolf part a `WolfModel.setSittingPose` delta applies to. A sitting wolf tilts its
/// body, tucks the hind legs under it, splays the front legs forward (with a tiny opposite
/// `x` nudge per side), and lifts the tail; the head still follows the look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum WolfSitPart {
    Body,
    HindLeg,
    RightFrontLeg,
    LeftFrontLeg,
    Tail,
}

/// Maps each sitting-pose part to its child name in the wolf body layer (the adult and baby trees
/// name the same parts identically — the baby drops only the mane `upper_body`, which never sits).
/// `WolfModel.setupAnim` folds each of these by `child_mut(name)`.
pub(in crate::entity_models) const fn wolf_sitting_part_roles() -> [(&'static str, WolfSitPart); 6]
{
    [
        ("body", WolfSitPart::Body),
        ("right_hind_leg", WolfSitPart::HindLeg),
        ("left_hind_leg", WolfSitPart::HindLeg),
        ("right_front_leg", WolfSitPart::RightFrontLeg),
        ("left_front_leg", WolfSitPart::LeftFrontLeg),
        ("tail", WolfSitPart::Tail),
    ]
}

/// Front-leg sitting `xRot` (`WolfModel.setSittingPose`, the literal `5.811947`, ≈ 333°).
pub(in crate::entity_models) const WOLF_SIT_FRONT_LEG_X_ROT: f32 = 5.811947;

/// Applies the vanilla `WolfModel.setSittingPose` delta to one wolf part pose. The
/// translation terms scale by `ageScale` (`1.0` adult / `0.5` baby); the rotations are SET
/// absolutely. The baby (`BabyWolfModel.setSittingPose`) tilts the body a further `−π/2`
/// after `super.setSittingPose`. The [`WolfSitPart::Tail`] delta only lifts the tail
/// offset — its `xRot`/`yRot` (`tailAngle`/wag) are applied by the normal tail pose, which
/// preserves the offset.
pub(in crate::entity_models) fn apply_wolf_sitting_pose(
    pose: &mut PartPose,
    part: WolfSitPart,
    baby: bool,
) {
    let age_scale = if baby { 0.5 } else { 1.0 };
    match part {
        WolfSitPart::Body => {
            pose.offset[1] += 4.0 * age_scale;
            pose.offset[2] -= 2.0 * age_scale;
            pose.rotation[0] = if baby {
                std::f32::consts::FRAC_PI_4 - std::f32::consts::FRAC_PI_2
            } else {
                std::f32::consts::FRAC_PI_4
            };
        }
        WolfSitPart::HindLeg => {
            pose.offset[1] += 6.7 * age_scale;
            pose.offset[2] -= 5.0 * age_scale;
            pose.rotation[0] = std::f32::consts::PI * 1.5;
        }
        WolfSitPart::RightFrontLeg => {
            pose.rotation[0] = WOLF_SIT_FRONT_LEG_X_ROT;
            pose.offset[0] += 0.01 * age_scale;
            pose.offset[1] += 1.0 * age_scale;
        }
        WolfSitPart::LeftFrontLeg => {
            pose.rotation[0] = WOLF_SIT_FRONT_LEG_X_ROT;
            pose.offset[0] -= 0.01 * age_scale;
            pose.offset[1] += 1.0 * age_scale;
        }
        WolfSitPart::Tail => {
            pose.offset[1] += 9.0 * age_scale;
            pose.offset[2] -= 2.0 * age_scale;
        }
    }
}

/// Vanilla `RavagerModel.setupAnim` leg swing for a single leg part: `leg.xRot =
/// cos(walkAnimationPos * 0.6662 [+ π]) * 0.4 * walkAnimationSpeed`. `RavagerModel`
/// is a custom `EntityModel`, but the leg swing follows the `QuadrupedModel` phase
/// (the right-hind/left-front pair in phase, resolved from `x * z < 0`) with a
/// shorter `0.4` amplitude (vanilla `legRot = 0.4 * walkAnimationSpeed`, no `1.4`
/// factor). The base leg pose carries no `xRot`, so it is set (not accumulated). The
/// neck/mouth attack/stun/roar poses are separate deferred event animations.
pub(in crate::entity_models) fn ravager_leg_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let phase = walk_animation_pos * 0.6662;
    let [x, _, z] = base.offset;
    let angle = if x * z < 0.0 {
        phase
    } else {
        phase + std::f32::consts::PI
    };
    PartPose {
        offset: base.offset,
        rotation: [
            angle.cos() * 0.4 * walk_animation_speed,
            base.rotation[1],
            base.rotation[2],
        ],
    }
}

/// Vanilla `Mth.triangleWave(index, period)`: a triangle wave in `[-1, 1]`,
/// `(|index % period - period/2| - period/4) / (period/4)`.
pub(in crate::entity_models) fn triangle_wave(index: f32, period: f32) -> f32 {
    ((index % period - period * 0.5).abs() - period * 0.25) / (period * 0.25)
}

/// The four swinging limb parts of the iron golem, for [`iron_golem_walk_pose`].
pub(in crate::entity_models) enum IronGolemWalkPart {
    RightArm,
    LeftArm,
    RightLeg,
    LeftLeg,
}

/// Vanilla `IronGolemModel.setupAnim` walking swing for one limb part, driven by
/// `Mth.triangleWave(walkAnimationPos, 13)`. Legs: `xRot = ±1.5 * triangleWave *
/// speed`. Arms (the default branch, when not attacking and not offering a flower):
/// `xRot = (-0.2 ± 1.5 * triangleWave) * speed`. The base pose carries no `xRot`, so
/// it is set (not accumulated). The attack swing and the offer-flower arm pose are
/// separate deferred event animations driven by server-authoritative timers.
pub(in crate::entity_models) fn iron_golem_walk_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
    part: IronGolemWalkPart,
) -> PartPose {
    let wave = triangle_wave(walk_animation_pos, 13.0);
    let x_rot = match part {
        IronGolemWalkPart::RightLeg => -1.5 * wave * walk_animation_speed,
        IronGolemWalkPart::LeftLeg => 1.5 * wave * walk_animation_speed,
        IronGolemWalkPart::RightArm => (-0.2 + 1.5 * wave) * walk_animation_speed,
        IronGolemWalkPart::LeftArm => (-0.2 - 1.5 * wave) * walk_animation_speed,
    };
    PartPose {
        offset: base.offset,
        rotation: [x_rot, base.rotation[1], base.rotation[2]],
    }
}

/// Vanilla `IronGolemModel.setupAnim` walk swing applied to a model root's named arm/leg children
/// `right_arm`/`left_arm`/`right_leg`/`left_leg` ([`iron_golem_walk_pose`]). The arms sit at part
/// offset `x = 0`, so each limb's walk role is fixed by name rather than offset sign. A no-op while
/// the limbs are at rest (`walkAnimationSpeed == 0`).
pub(in crate::entity_models) fn apply_iron_golem_walk(
    root: &mut ModelPart,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) {
    if limb_swing_at_rest(walk_animation_speed) {
        return;
    }
    for (name, part) in [
        ("right_arm", IronGolemWalkPart::RightArm),
        ("left_arm", IronGolemWalkPart::LeftArm),
        ("right_leg", IronGolemWalkPart::RightLeg),
        ("left_leg", IronGolemWalkPart::LeftLeg),
    ] {
        let limb = root.child_mut(name);
        limb.pose = iron_golem_walk_pose(limb.pose, walk_animation_pos, walk_animation_speed, part);
    }
}

/// Vanilla `IronGolemModel.setupAnim`'s arm event poses, applied AFTER [`apply_iron_golem_walk`] so
/// they override the walk arm swing (the legs keep it). While `attackTicksRemaining > 0` both arms
/// raise into the two-fisted smash (`xRot = -2 + 1.5·triangleWave(tick, 10)`); else while
/// `offerFlowerTick > 0` the right arm holds a poppy out (`xRot = -0.8 + 0.025·triangleWave(tick, 70)`)
/// and the left arm drops flat (`xRot = 0`). Only `xRot` is set (the walk pose preserves `yRot`/`zRot`),
/// matching vanilla's per-branch assignment. A no-op when neither timer is active.
pub(in crate::entity_models) fn apply_iron_golem_arm_events(
    root: &mut ModelPart,
    attack_ticks_remaining: f32,
    offer_flower_tick: i32,
) {
    if attack_ticks_remaining > 0.0 {
        let x_rot = -2.0 + 1.5 * triangle_wave(attack_ticks_remaining, 10.0);
        root.child_mut("right_arm").pose.rotation[0] = x_rot;
        root.child_mut("left_arm").pose.rotation[0] = x_rot;
    } else if offer_flower_tick > 0 {
        root.child_mut("right_arm").pose.rotation[0] =
            -0.8 + 0.025 * triangle_wave(offer_flower_tick as f32, 70.0);
        root.child_mut("left_arm").pose.rotation[0] = 0.0;
    }
}

/// Vanilla `SpiderModel.setupAnim` walking swing for one leg part. With
/// `animationPos = walkAnimationPos * 0.6662`, vanilla computes a horizontal `swing`
/// `-(cos(animationPos * 2 + phase) * 0.4) * walkAnimationSpeed` accumulated onto
/// `yRot`, and a vertical `step` `|sin(animationPos + phase) * 0.4| *
/// walkAnimationSpeed` accumulated onto `zRot`; the right legs add both (`+=`), the
/// left legs subtract both (`-=`). `phase` is the per-leg-pair offset (`0`, `π`,
/// `π/2`, `3π/2` for hind, middle-hind, middle-front, front), and `side_sign` is
/// `+1` for the right legs and `-1` for the left. Both terms are accumulated onto the
/// leg's resting `yRot`/`zRot` (the splay set in the body layer); `xRot` is untouched.
pub(in crate::entity_models) fn spider_leg_swing_pose(
    base: PartPose,
    phase: f32,
    side_sign: f32,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let animation_pos = walk_animation_pos * 0.6662;
    let swing = -((animation_pos * 2.0 + phase).cos() * 0.4) * walk_animation_speed;
    let step = (animation_pos + phase).sin().abs() * 0.4 * walk_animation_speed;
    PartPose {
        offset: base.offset,
        rotation: [
            base.rotation[0],
            base.rotation[1] + side_sign * swing,
            base.rotation[2] + side_sign * step,
        ],
    }
}

/// The eight spider leg child names paired with their `(phase, side_sign)` swing roles.
/// `SpiderModel.createSpiderBodyLayer` lists the legs after head/body0/body1 in right/left pairs from
/// hind to front: hind (phase `0`), middle-hind (phase `π`), middle-front (phase `π/2`), front (phase
/// `3π/2`). Right legs swing `+`, left legs swing `-`. The names match the tree the spider builds, so
/// `setup_anim` addresses each leg by `child_mut(name)`.
pub(in crate::entity_models) fn spider_leg_swing_roles() -> [(&'static str, f32, f32); 8] {
    use std::f32::consts::{FRAC_PI_2, PI};
    [
        ("right_hind_leg", 0.0, 1.0),
        ("left_hind_leg", 0.0, -1.0),
        ("right_middle_hind_leg", PI, 1.0),
        ("left_middle_hind_leg", PI, -1.0),
        ("right_middle_front_leg", FRAC_PI_2, 1.0),
        ("left_middle_front_leg", FRAC_PI_2, -1.0),
        ("right_front_leg", PI * 1.5, 1.0),
        ("left_front_leg", PI * 1.5, -1.0),
    ]
}

/// Vanilla `AbstractEquineModel.setupAnim` walking leg swing (the non-standing branch).
/// With `legAnim = cos(waterMultiplier * walkAnimationPos * 0.6662 + π) * walkAnimationSpeed`,
/// the front legs swing `±0.8 * legAnim` and the hind legs `±0.5 * legAnim` — a horse-specific
/// gait (front amplitude `0.8`, hind `0.5`) rather than the uniform `1.4`
/// `QuadrupedModel` swing. The signs are front-left `+0.8`, front-right `-0.8`,
/// hind-left `-0.5`, hind-right `+0.5`: the front legs have `z < 0` and the left legs
/// `x > 0`, so the sign is `+` when `(x > 0) == (z < 0)`. The base leg pose carries no
/// `xRot`, so it is set (not accumulated). Vanilla scales the swing frequency by
/// `waterMultiplier = isInWater ? 0.2 : 1.0` (a slower paddle in water), which the renderer
/// reproduces from the projected `in_water`. The standing/eating/feeding poses are still
/// deferred (they depend on state the client does not yet track). The head look/bob is
/// applied separately by [`equine_head_look_pose`] and the tail walk lift by
/// [`equine_tail_swing_pose`].
pub(in crate::entity_models) fn equine_leg_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
    in_water: bool,
) -> PartPose {
    let water_multiplier = if in_water { 0.2 } else { 1.0 };
    let leg_anim = (water_multiplier * walk_animation_pos * 0.6662 + std::f32::consts::PI).cos()
        * walk_animation_speed;
    let [x, _, z] = base.offset;
    let front = z < 0.0;
    let amplitude = if front { 0.8 } else { 0.5 };
    let sign = if (x > 0.0) == front { 1.0 } else { -1.0 };
    PartPose {
        offset: base.offset,
        rotation: [
            sign * amplitude * leg_anim,
            base.rotation[1],
            base.rotation[2],
        ],
    }
}

/// Vanilla `AbstractEquineModel.setupAnim` head (`head_parts`) look, in its default
/// (non-standing, non-eating, non-feeding) branch — the branch a free-standing horse
/// always takes. The net head yaw is clamped to `±20°` (a horse turns its head less than
/// the body) and applied as `head_parts.yRot = clamp(yRot, -20, 20) * π/180`; the head
/// pitch is added onto the layer's `π/6` neck tilt as `head_parts.xRot = π/6 + xRot *
/// π/180`, with a walk-driven bob `+= cos(walkAnimationPos * 0.8) * 0.15 *
/// walkAnimationSpeed` folded in when `walkAnimationSpeed > 0.2`. The rest `head_parts`
/// xRot is exactly that `π/6` tilt, so at a level head and no fast gait the pose equals
/// the rest pose. `HorseModel`/`BabyHorseModel` and the adult `DonkeyModel`/mule take this
/// unchanged; the baby donkey/mule (which forces `xRot = -30°`) and the ridden/stand/eat/feed
/// poses are deferred (the in-water gait frequency is applied by [`equine_leg_swing_pose`]).
/// The tail walk lift is applied by [`equine_tail_swing_pose`]; only its `ageInTicks`-driven
/// `yRot` wag stays deferred.
pub(in crate::entity_models) fn equine_head_look_pose(
    base: PartPose,
    head_yaw_deg: f32,
    head_pitch_deg: f32,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let clamped_yaw = head_yaw_deg.clamp(-20.0, 20.0);
    let mut head_rot_x = head_pitch_deg.to_radians();
    if walk_animation_speed > 0.2 {
        head_rot_x += (walk_animation_pos * 0.8).cos() * 0.15 * walk_animation_speed;
    }
    PartPose {
        offset: base.offset,
        rotation: [
            std::f32::consts::FRAC_PI_6 + head_rot_x,
            clamped_yaw.to_radians(),
            base.rotation[2],
        ],
    }
}

/// Vanilla `AbstractEquineModel.setupAnim` tail walk animation (the default branch). The
/// tail's `xRot` is *set* to `getTailXRotOffset() + π/6 + walkAnimationSpeed * 0.75`, so a
/// running equine lifts its tail. The per-model `getTailXRotOffset` (`0` for the adult
/// horse/donkey/mule, `−π/2` for the baby horse) also overrides the baby layer's wider
/// rest angle: vanilla runs `setupAnim` every frame, so a standing baby horse renders its
/// tail at `−π/2 + π/6 = −1.0472`, not the layer's `−0.7418`. The tail base also
/// translates `y += walkAnimationSpeed * ageScale` and `z += walkAnimationSpeed * 2 *
/// ageScale`, where `ageScale` is `getAgeScale()` (`1.0` for adults, `0.5` for babies).
/// The `tail.yRot` wag (`cos(ageInTicks * 0.7)` under `animateTail`) needs `ageInTicks` the
/// client does not track and is deferred, so `yRot`/`zRot` are preserved here.
pub(in crate::entity_models) fn equine_tail_swing_pose(
    base: PartPose,
    tail_x_rot_offset: f32,
    walk_animation_speed: f32,
    age_scale: f32,
) -> PartPose {
    PartPose {
        offset: [
            base.offset[0],
            base.offset[1] + walk_animation_speed * age_scale,
            base.offset[2] + walk_animation_speed * 2.0 * age_scale,
        ],
        rotation: [
            tail_x_rot_offset + std::f32::consts::FRAC_PI_6 + walk_animation_speed * 0.75,
            base.rotation[1],
            base.rotation[2],
        ],
    }
}

/// Vanilla `SnowGolemModel.setupAnim` upper-body twist: the middle snow ball turns a
/// quarter of the head yaw, `upperBody.yRot = headYaw * π/180 * 0.25`. The base upper
/// body carries no rotation, so the twist is set (not accumulated); `xRot`/`zRot` and
/// the offset are preserved. The returned value is also the arm orbit angle for
/// [`snow_golem_arm_pose`].
pub(in crate::entity_models) fn snow_golem_upper_body_yrot(head_yaw_deg: f32) -> f32 {
    head_yaw_deg.to_radians() * 0.25
}

/// Applies the [`snow_golem_upper_body_yrot`] twist to the upper-body part pose.
pub(in crate::entity_models) fn snow_golem_upper_body_pose(
    base: PartPose,
    upper_body_yrot: f32,
) -> PartPose {
    PartPose {
        offset: base.offset,
        rotation: [base.rotation[0], upper_body_yrot, base.rotation[2]],
    }
}

/// Vanilla `SnowGolemModel.setupAnim` arm orbit. The two stick arms ride the twisting
/// upper body: `leftArm.yRot = upperBodyYRot`, `leftArm.x = cos(upperBodyYRot) * 5`,
/// `leftArm.z = -sin(upperBodyYRot) * 5`; the right arm adds `π` to the yaw and negates
/// both `x` and `z`. The arm `y` offset and the drooping `zRot` (`±1.0` from the body
/// layer) are preserved; the base `x`/`z` offsets are overwritten by the orbit even at
/// rest (so a forward-facing snow golem still pulls its arms to `z = 0`).
pub(in crate::entity_models) fn snow_golem_arm_pose(
    base: PartPose,
    upper_body_yrot: f32,
    right: bool,
) -> PartPose {
    let (sin, cos) = upper_body_yrot.sin_cos();
    let (x, z, y_rot) = if right {
        (
            -cos * 5.0,
            sin * 5.0,
            upper_body_yrot + std::f32::consts::PI,
        )
    } else {
        (cos * 5.0, -sin * 5.0, upper_body_yrot)
    };
    PartPose {
        offset: [x, base.offset[1], z],
        rotation: [base.rotation[0], y_rot, base.rotation[2]],
    }
}

/// Vanilla head look shared by `QuadrupedModel.setupAnim` and
/// `HumanoidModel.setupAnim`: `head.xRot = xRot * π/180` and `head.yRot = yRot *
/// π/180`, where `xRot` is the head pitch and `yRot` is the net head yaw
/// (`Mth.wrapDegrees(headRot - bodyRot)`). The base head pose carries no
/// rotation, so the look angles are set (not accumulated), matching the vanilla
/// assignments. Pig, cow, and the zombie family extend these base models without
/// overriding the head animation, so this is their full head pose.
pub(in crate::entity_models) fn head_look_pose(
    base: PartPose,
    head_yaw_deg: f32,
    head_pitch_deg: f32,
) -> PartPose {
    PartPose {
        offset: base.offset,
        rotation: [
            head_pitch_deg.to_radians(),
            head_yaw_deg.to_radians(),
            base.rotation[2],
        ],
    }
}
