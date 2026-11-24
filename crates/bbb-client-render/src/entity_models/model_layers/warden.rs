use super::super::keyframe::{
    keyframe_animated_pose, keyframe_elapsed_seconds, sample_bone_offsets, AnimationDefinition,
};
use super::PartPose;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_data_26_1::warden_animation::{
    WARDEN_ATTACK, WARDEN_DIG, WARDEN_EMERGE, WARDEN_ROAR, WARDEN_SNIFF, WARDEN_SONIC_BOOM,
};
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `WardenModel.animateIdlePose` body roll: with `s = ageInTicks·0.1`, the body adds
/// `xRot += 0.025·cos(s)` and `zRot += 0.025·sin(s)` onto its bind pose. Always on (no gating
/// state), so every warden sways gently. Mirrors the head roll in [`warden_head_pose`].
pub(in crate::entity_models) fn warden_idle_body_pose(
    base: PartPose,
    age_in_ticks: f32,
) -> PartPose {
    let s = age_in_ticks * 0.1;
    PartPose {
        offset: base.offset,
        rotation: [
            base.rotation[0] + 0.025 * s.cos(),
            base.rotation[1],
            base.rotation[2] + 0.025 * s.sin(),
        ],
    }
}

/// Vanilla `WardenModel` head pose: `animateHeadLookTarget` first sets `head.xRot = xRot·π/180`,
/// `head.yRot = yRot·π/180` (overwriting the bind), then `animateIdlePose` adds the always-on roll
/// `head.xRot += 0.06·sin(s)`, `head.zRot += 0.06·cos(s)` with `s = ageInTicks·0.1`. The walk
/// ([`warden_walk_pose`]) then adds further to `head.xRot/zRot` via [`warden_add_x_z_rot`]. The base
/// `head.zRot` is the bind `0`, so the idle roll lands on `base.rotation[2]`.
pub(in crate::entity_models) fn warden_head_pose(
    base: PartPose,
    head_yaw_deg: f32,
    head_pitch_deg: f32,
    age_in_ticks: f32,
) -> PartPose {
    let s = age_in_ticks * 0.1;
    PartPose {
        offset: base.offset,
        rotation: [
            head_pitch_deg.to_radians() + 0.06 * s.sin(),
            head_yaw_deg.to_radians(),
            base.rotation[2] + 0.06 * s.cos(),
        ],
    }
}

/// The per-bone `xRot`/`zRot` offsets produced by vanilla `WardenModel.animateWalk(walkPos,
/// walkSpeed)`. Every term derives from `speedModifier = min(0.5, 3·walkSpeed)`,
/// `speedModifierWithMin = min(0.35, speedModifier)`, and `adjustedPos = walkPos·0.8662`. The head
/// and body offsets ADD onto the look/idle composition (vanilla uses `+=` on the head and SETs the
/// body, whose bind rotation is zero); the legs and arms are SET from their zero bind rotation.
/// `animateWalk` ends with `resetArmPoses`, which restores the arms' bind position and `yRot`, so
/// only the arm `xRot` moves. Because all of these compose additively onto a zero/known base, the
/// warden emit applies them through [`warden_add_x_z_rot`] after the look/idle pass — addition is
/// commutative, so the vanilla order (look → walk → idle) is preserved.
pub(in crate::entity_models) struct WardenWalkPose {
    pub(in crate::entity_models) head_x_rot: f32,
    pub(in crate::entity_models) head_z_rot: f32,
    pub(in crate::entity_models) body_x_rot: f32,
    pub(in crate::entity_models) body_z_rot: f32,
    pub(in crate::entity_models) left_leg_x_rot: f32,
    pub(in crate::entity_models) right_leg_x_rot: f32,
    pub(in crate::entity_models) left_arm_x_rot: f32,
    pub(in crate::entity_models) right_arm_x_rot: f32,
}

/// Samples vanilla `WardenModel.animateWalk` into a [`WardenWalkPose`]. At `walkSpeed = 0` every
/// term is zero, so a standing warden adds nothing on top of the look/idle pose.
pub(in crate::entity_models) fn warden_walk_pose(walk_pos: f32, walk_speed: f32) -> WardenWalkPose {
    use std::f32::consts::{FRAC_PI_2, PI};
    let speed = (3.0 * walk_speed).min(0.5);
    let speed_with_min = speed.min(0.35);
    let adjusted_pos = walk_pos * 0.8662;
    let cos = adjusted_pos.cos();
    let sin = adjusted_pos.sin();
    WardenWalkPose {
        head_x_rot: 1.2 * (adjusted_pos + FRAC_PI_2).cos() * speed_with_min,
        head_z_rot: 0.3 * sin * speed,
        body_x_rot: 1.0 * cos * speed_with_min,
        body_z_rot: 0.1 * sin * speed,
        left_leg_x_rot: 1.0 * cos * speed,
        right_leg_x_rot: 1.0 * (adjusted_pos + PI).cos() * speed,
        left_arm_x_rot: -(0.8 * cos * speed),
        right_arm_x_rot: -(0.8 * sin * speed),
    }
}

/// Adds `x_rot`/`z_rot` onto a pose's `rotation[0]`/`rotation[2]`, leaving `rotation[1]` (yRot) and
/// the offset untouched. The warden's [`warden_walk_pose`] offsets compose this way onto every
/// animated bone (the head/body after their look/idle pass, the legs/arms over their bind pose).
pub(in crate::entity_models) fn warden_add_x_z_rot(
    base: PartPose,
    x_rot: f32,
    z_rot: f32,
) -> PartPose {
    PartPose {
        offset: base.offset,
        rotation: [
            base.rotation[0] + x_rot,
            base.rotation[1],
            base.rotation[2] + z_rot,
        ],
    }
}

/// Vanilla `WardenModel.animateTendrils`: the magnitude of the tendril `xRot` sway,
/// `tendrilXRot = tendrilAnimation · cos(ageInTicks · 2.25) · π · 0.1`. The left tendril takes
/// `+tendrilXRot` and the right takes `-tendrilXRot`. `tendrilAnimation` is the projected `0..=1`
/// pulse (`Warden.getTendrilAnimation`), so a resting warden (`tendrilAnimation = 0`) holds its
/// antennae at bind. Vanilla computes the `cos·π·0.1` factor in double precision before the
/// `(float)` cast, which this mirrors.
pub(in crate::entity_models) fn warden_tendril_x_rot(
    tendril_animation: f32,
    age_in_ticks: f32,
) -> f32 {
    let factor = ((age_in_ticks as f64 * 2.25).cos() * std::f64::consts::PI * 0.1) as f32;
    tendril_animation * factor
}

// ----- `WardenAnimation.WARDEN_ATTACK` (length 0.33333s, NOT looping). The melee swing: the `body`
// rocks and dips forward, the `head` whips down, and the two arms slam (each ROTATION + POSITION).
// All keyframes are CATMULLROM; `posVec` negates the y axis and `degreeVec` converts to radians. -----

// ----- `WardenAnimation.WARDEN_SONIC_BOOM` (length 3.0s, NOT looping). The charge/blast: the `body`
// rears then rocks back, the two `ribcage` planes fan open (yaw ±125°), the `head` cranes up then
// snaps down, and the two arms wind up and thrust (ROTATION + POSITION). All keyframes CATMULLROM. --

// ----- `WardenAnimation.WARDEN_ROAR` (length 4.2s, NOT looping). The threat roar: the `body` rears
// up, the `head` shakes side to side, and the two arms fling wide and shudder (ROTATION + POSITION).
// All keyframes CATMULLROM. -----

// ----- `WardenAnimation.WARDEN_SNIFF` (length 4.16s, NOT looping). The investigative sniff: the
// `body` and `head` turn and dip to scent, and the two arms ease (ROTATION only). All keyframes
// CATMULLROM. -----

// ----- `WardenAnimation.WARDEN_DIG` (length 5.0s, NOT looping). The despawn burrow: the warden
// rocks forward, claws at the ground, and sinks out of sight (every bone ROTATION + POSITION). Most
// keyframes are CATMULLROM; the terminal frame of several channels is LINEAR (a straight slide into
// the ground). `posVec` negates the y axis and `degreeVec` converts to radians. -----

// ----- `WardenAnimation.WARDEN_EMERGE` (length 6.68s, NOT looping). The spawn rise: the warden
// surfaces from 63 units underground (the `body`/`right_leg`/`left_leg` POSITION channels open at
// `posVec(0, -63, 0)`), heaves its torso up, plants its arms, and stands (every bone ROTATION +
// POSITION). All keyframes are CATMULLROM. -----

/// Vanilla `WardenModel`: `setup_anim` composes the head look, idle wobble, walk swing and tendril
/// sway, then adds the six triggered keyframe one-shots in the Vanilla order.
pub(in crate::entity_models) fn warden_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::WARDEN.bake();
    let warden_state = *instance.render_state.kind.warden();
    // Vanilla `WardenModel.setupAnim` composes four non-keyframe motions: the head look
    // ([`warden_head_pose`]), the always-on idle wobble ([`warden_idle_body_pose`] on the body,
    // folded into the head pose), the walk swing ([`warden_walk_pose`], ADDED via
    // [`warden_add_x_z_rot`] onto the look/idle composition — addition is commutative, preserving
    // the vanilla order), and the tendril sway ([`warden_tendril_x_rot`]). At `walkSpeed = 0` the
    // walk adds nothing; the idle wobble and tendril sway advance every frame off `ageInTicks`.
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let age = instance.render_state.age_in_ticks;
    let walk = warden_walk_pose(
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );
    let tendril_x = warden_tendril_x_rot(warden_state.tendril_animation, age);

    // Vanilla `WardenModel.setupAnim` then applies the six triggered combat keyframe one-shots
    // ADDITIVELY (`KeyframeAnimation.apply` folds `offsetPos`/`offsetRotation` onto the
    // already-posed bones) in the order attack → sonic_boom → dig → emerge → roar → sniff. Each
    // is applied only while its projected elapsed-seconds value is `>= 0`; a non-looping def
    // clamps past its length to the resting final frame (vanilla's "hold the last frame"). The
    // active definitions, paired with their (clamped) sample seconds.
    let combat: [(&AnimationDefinition, f32); 6] = [
        (&WARDEN_ATTACK, warden_state.attack_seconds),
        (&WARDEN_SONIC_BOOM, warden_state.sonic_boom_seconds),
        (&WARDEN_DIG, warden_state.dig_seconds),
        (&WARDEN_EMERGE, warden_state.emerge_seconds),
        (&WARDEN_ROAR, warden_state.roar_seconds),
        (&WARDEN_SNIFF, warden_state.sniff_seconds),
    ];
    // Adds every active combat one-shot's position/rotation offsets onto a part already posed by
    // the look/idle/walk (and, for the head, the tendril pulse).
    let apply_combat = |part: &mut ModelPart, bone: &str| {
        for (definition, seconds) in combat {
            if seconds < 0.0 {
                continue;
            }
            let sample = keyframe_elapsed_seconds(definition, seconds);
            let (position, rotation) = sample_bone_offsets(definition, bone, sample, 1.0);
            part.pose = keyframe_animated_pose(part.pose, position, rotation);
        }
    };

    let bone = root.child_mut("bone");
    {
        let body = bone.child_mut("body");
        body.pose = warden_add_x_z_rot(
            warden_idle_body_pose(body.pose, age),
            walk.body_x_rot,
            walk.body_z_rot,
        );
        apply_combat(body, "body");

        {
            let head = body.child_mut("head");
            head.pose = warden_add_x_z_rot(
                warden_head_pose(head.pose, head_yaw, head_pitch, age),
                walk.head_x_rot,
                walk.head_z_rot,
            );
            apply_combat(head, "head");

            // The two tendrils sway their `xRot` off the pulse (left `+`, right `-`).
            let right = head.child_mut("right_tendril");
            right.pose = warden_add_x_z_rot(right.pose, -tendril_x, 0.0);
            let left = head.child_mut("left_tendril");
            left.pose = warden_add_x_z_rot(left.pose, tendril_x, 0.0);
        }

        // The two ribcages hold at bind for the walk/idle, but the sonic boom fans them open.
        apply_combat(body.child_mut("right_ribcage"), "right_ribcage");
        apply_combat(body.child_mut("left_ribcage"), "left_ribcage");

        // The two arms swing their `xRot` with the walk, then the combat one-shots add on top.
        let right_arm = body.child_mut("right_arm");
        right_arm.pose = warden_add_x_z_rot(right_arm.pose, walk.right_arm_x_rot, 0.0);
        apply_combat(right_arm, "right_arm");
        let left_arm = body.child_mut("left_arm");
        left_arm.pose = warden_add_x_z_rot(left_arm.pose, walk.left_arm_x_rot, 0.0);
        apply_combat(left_arm, "left_arm");
    }

    // The two legs swing their `xRot` with the walk, then the dig/emerge spawn/despawn one-shots
    // add on top (only those two touch the legs — attack/sonic_boom/roar/sniff carry no leg
    // bone, so `apply_combat` adds zero for them).
    let right_leg = bone.child_mut("right_leg");
    right_leg.pose = warden_add_x_z_rot(right_leg.pose, walk.right_leg_x_rot, 0.0);
    apply_combat(right_leg, "right_leg");
    let left_leg = bone.child_mut("left_leg");
    left_leg.pose = warden_add_x_z_rot(left_leg.pose, walk.left_leg_x_rot, 0.0);
    apply_combat(left_leg, "left_leg");
    root
}
