use super::{
    model_cube as cube, ModelCubeDesc, PartPose, PART_POSE_ZERO, WARDEN_BODY, WARDEN_TENDRIL,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

// Vanilla 26.1 `WardenModel.createBodyLayer` (atlas 128×128). The mesh root holds one `bone` part
// at `offset(0, 24, 0)` parenting the body and the two legs; `body` parents the two ribcage
// planes, the head (which parents the two tendril planes), and the two arms. Four non-keyframe
// `WardenModel.setupAnim` motions are reproduced ([`warden_head_pose`] / [`warden_idle_body_pose`] /
// [`warden_walk_pose`] / [`warden_tendril_x_rot`]): the head look (`animateHeadLookTarget`), the
// always-on idle wobble (`animateIdlePose`), the walk (`animateWalk`, which swings the head, body,
// legs, and arms off `walkAnimationPos/Speed` and composes additively onto the look/idle pose via
// [`warden_add_x_z_rot`]), and the tendril sway (`animateTendrils`, which swings the two head
// tendrils off the projected `tendrilAnimation` pulse and `ageInTicks`). The attack / sonic-boom /
// digging / emerge / roar / sniff keyframe animations stay deferred. The four emissive overlay
// layers (tendrils, heart, bioluminescent, pulsating spots) and the texture-backed path are deferred.

// `body`: one 18×21×11 box.
pub(in crate::entity_models) const WARDEN_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-9.0, -13.0, -4.0], [18.0, 21.0, 11.0], WARDEN_BODY)];

// The two ribcage planes (`texOffs(90,11)`, the left mirrored); both are 9×21×0.
pub(in crate::entity_models) const WARDEN_RIGHT_RIBCAGE_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.0, -11.0, -0.1], [9.0, 21.0, 0.0], WARDEN_BODY)];
pub(in crate::entity_models) const WARDEN_LEFT_RIBCAGE_CUBES: [ModelCubeDesc; 1] =
    [cube([-7.0, -11.0, -0.1], [9.0, 21.0, 0.0], WARDEN_BODY)];

// `head`: one 16×16×10 box.
pub(in crate::entity_models) const WARDEN_HEAD_CUBES: [ModelCubeDesc; 1] =
    [cube([-8.0, -16.0, -5.0], [16.0, 16.0, 10.0], WARDEN_BODY)];

// The two tendril planes (16×16×0), the warden's iconic glow antennae.
pub(in crate::entity_models) const WARDEN_RIGHT_TENDRIL_CUBES: [ModelCubeDesc; 1] =
    [cube([-16.0, -13.0, 0.0], [16.0, 16.0, 0.0], WARDEN_TENDRIL)];
pub(in crate::entity_models) const WARDEN_LEFT_TENDRIL_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, -13.0, 0.0], [16.0, 16.0, 0.0], WARDEN_TENDRIL)];

// Both arms share one 8×28×8 box.
pub(in crate::entity_models) const WARDEN_ARM_CUBES: [ModelCubeDesc; 1] =
    [cube([-4.0, 0.0, -4.0], [8.0, 28.0, 8.0], WARDEN_BODY)];

// The legs (6×13×6) differ only in X origin.
pub(in crate::entity_models) const WARDEN_RIGHT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.1, 0.0, -3.0], [6.0, 13.0, 6.0], WARDEN_BODY)];
pub(in crate::entity_models) const WARDEN_LEFT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.9, 0.0, -3.0], [6.0, 13.0, 6.0], WARDEN_BODY)];

/// Vanilla `WardenModel.createBodyLayer` rest-pose part poses, rooted at the cubeless `bone` part
/// (`offset(0, 24, 0)`) parenting the `body` and the two legs; `body` parents the two ribcages, the
/// `head` (which parents the two tendrils), and the two arms. Ten cubes.
/// `bone` cubeless-pivot part pose: `PartPose.offset(0, 24, 0)`.
pub(in crate::entity_models) const WARDEN_BONE_POSE: PartPose = PartPose {
    offset: [0.0, 24.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `body` part pose: `PartPose.offset(0, -21, 0)`. The idle wobble and walk swing add onto its bind.
pub(in crate::entity_models) const WARDEN_BODY_POSE: PartPose = PartPose {
    offset: [0.0, -21.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_ribcage` part pose: `PartPose.offset(-7, -2, -4)`.
pub(in crate::entity_models) const WARDEN_RIGHT_RIBCAGE_POSE: PartPose = PartPose {
    offset: [-7.0, -2.0, -4.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_ribcage` part pose: `PartPose.offset(7, -2, -4)`.
pub(in crate::entity_models) const WARDEN_LEFT_RIBCAGE_POSE: PartPose = PartPose {
    offset: [7.0, -2.0, -4.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `head` part pose: `PartPose.offset(0, -13, 0)`. `animateHeadLookTarget` sets `head.xRot/yRot`
/// from the look; the tendrils nested under the head inherit the turn.
pub(in crate::entity_models) const WARDEN_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, -13.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_tendril` part pose: `PartPose.offset(-8, -12, 0)`. Vanilla sets `rightTendril.xRot =
/// -tendrilXRot`.
pub(in crate::entity_models) const WARDEN_RIGHT_TENDRIL_POSE: PartPose = PartPose {
    offset: [-8.0, -12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_tendril` part pose: `PartPose.offset(8, -12, 0)`. Vanilla sets `leftTendril.xRot =
/// +tendrilXRot`.
pub(in crate::entity_models) const WARDEN_LEFT_TENDRIL_POSE: PartPose = PartPose {
    offset: [8.0, -12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_arm` part pose: `PartPose.offset(-13, -13, 1)`. `animateWalk` swings its `xRot`.
pub(in crate::entity_models) const WARDEN_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-13.0, -13.0, 1.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_arm` part pose: `PartPose.offset(13, -13, 1)`. `animateWalk` swings its `xRot`.
pub(in crate::entity_models) const WARDEN_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [13.0, -13.0, 1.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_leg` part pose: `PartPose.offset(-5.9, -13, 0)`. `animateWalk` swings its `xRot`.
pub(in crate::entity_models) const WARDEN_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-5.9, -13.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_leg` part pose: `PartPose.offset(5.9, -13, 0)`. `animateWalk` swings its `xRot`.
pub(in crate::entity_models) const WARDEN_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [5.9, -13.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds the warden's synthetic root parenting the single cubeless `bone` part, which parents the
/// cube-bearing `body` (→ two ribcages / `head` → two tendrils / two arms) and the two legs, in
/// vanilla `addOrReplaceChild` order. The `bone`, `body`, `head`, both tendrils, both arms, and both
/// legs are name-addressed by `setup_anim`, so `bone`, `body`, and `head` carry named children.
fn warden_root() -> ModelPart {
    let head = ModelPart::colored_named(
        WARDEN_HEAD_POSE,
        &WARDEN_HEAD_CUBES,
        vec![
            (
                "right_tendril",
                ModelPart::leaf_colored(WARDEN_RIGHT_TENDRIL_POSE, &WARDEN_RIGHT_TENDRIL_CUBES),
            ),
            (
                "left_tendril",
                ModelPart::leaf_colored(WARDEN_LEFT_TENDRIL_POSE, &WARDEN_LEFT_TENDRIL_CUBES),
            ),
        ],
    );
    let body = ModelPart::colored_named(
        WARDEN_BODY_POSE,
        &WARDEN_BODY_CUBES,
        vec![
            (
                "right_ribcage",
                ModelPart::leaf_colored(WARDEN_RIGHT_RIBCAGE_POSE, &WARDEN_RIGHT_RIBCAGE_CUBES),
            ),
            (
                "left_ribcage",
                ModelPart::leaf_colored(WARDEN_LEFT_RIBCAGE_POSE, &WARDEN_LEFT_RIBCAGE_CUBES),
            ),
            ("head", head),
            (
                "right_arm",
                ModelPart::leaf_colored(WARDEN_RIGHT_ARM_POSE, &WARDEN_ARM_CUBES),
            ),
            (
                "left_arm",
                ModelPart::leaf_colored(WARDEN_LEFT_ARM_POSE, &WARDEN_ARM_CUBES),
            ),
        ],
    );
    let bone = ModelPart::new(
        WARDEN_BONE_POSE,
        Vec::new(),
        vec![
            ("body", body),
            (
                "right_leg",
                ModelPart::leaf_colored(WARDEN_RIGHT_LEG_POSE, &WARDEN_RIGHT_LEG_CUBES),
            ),
            (
                "left_leg",
                ModelPart::leaf_colored(WARDEN_LEFT_LEG_POSE, &WARDEN_LEFT_LEG_CUBES),
            ),
        ],
    );
    ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![("bone", bone)])
}

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

/// Mutable warden model, mirroring vanilla `WardenModel`. The cubeless `bone` root (parenting the
/// body and two legs; `body` parents the ribcages, head, and two arms; `head` parents the two
/// tendrils) hangs off a synthetic root, built from the baked colored geometry as a named-children
/// tree. Colored-only: `setup_anim` reproduces the four non-keyframe motions — the head look, the
/// idle wobble, the walk swing, and the tendril sway (the attack / sonic-boom / dig / emerge / roar
/// keyframes stay deferred).
pub(in crate::entity_models) struct WardenModel {
    root: ModelPart,
}

impl WardenModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: warden_root(),
        }
    }
}

impl EntityModel for WardenModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
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
        let tendril_x = warden_tendril_x_rot(instance.render_state.tendril_animation, age);

        let bone = self.root.child_mut("bone");
        {
            let body = bone.child_mut("body");
            body.pose = warden_add_x_z_rot(
                warden_idle_body_pose(body.pose, age),
                walk.body_x_rot,
                walk.body_z_rot,
            );

            {
                let head = body.child_mut("head");
                head.pose = warden_add_x_z_rot(
                    warden_head_pose(head.pose, head_yaw, head_pitch, age),
                    walk.head_x_rot,
                    walk.head_z_rot,
                );

                // The two tendrils sway their `xRot` off the pulse (left `+`, right `-`).
                let right = head.child_mut("right_tendril");
                right.pose = warden_add_x_z_rot(right.pose, -tendril_x, 0.0);
                let left = head.child_mut("left_tendril");
                left.pose = warden_add_x_z_rot(left.pose, tendril_x, 0.0);
            }

            // The two arms swing their `xRot` with the walk; the ribcages hold.
            let right_arm = body.child_mut("right_arm");
            right_arm.pose = warden_add_x_z_rot(right_arm.pose, walk.right_arm_x_rot, 0.0);
            let left_arm = body.child_mut("left_arm");
            left_arm.pose = warden_add_x_z_rot(left_arm.pose, walk.left_arm_x_rot, 0.0);
        }

        // The two legs swing their `xRot` with the walk.
        let right_leg = bone.child_mut("right_leg");
        right_leg.pose = warden_add_x_z_rot(right_leg.pose, walk.right_leg_x_rot, 0.0);
        let left_leg = bone.child_mut("left_leg");
        left_leg.pose = warden_add_x_z_rot(left_leg.pose, walk.left_leg_x_rot, 0.0);
    }
}
