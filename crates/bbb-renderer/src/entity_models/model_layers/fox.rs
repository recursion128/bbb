use super::super::keyframe::{
    degree_vec, keyframe, keyframe_animated_pose, keyframe_animated_scale, keyframe_walk_sample,
    pos_vec, sample_bone_offsets_with_scale, scale_vec, AnimationChannel, AnimationDefinition,
    AnimationTarget, BoneAnimation, Keyframe, KeyframeInterpolation,
};
use super::{apply_head_look, limb_swing_at_rest, PartPose, FOX_ORANGE, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};
use std::f32::consts::{FRAC_PI_2, PI};

const LINEAR: KeyframeInterpolation = KeyframeInterpolation::Linear;
const CATMULLROM: KeyframeInterpolation = KeyframeInterpolation::CatmullRom;

const fn rot(keyframes: &'static [Keyframe]) -> AnimationChannel {
    AnimationChannel {
        target: AnimationTarget::Rotation,
        keyframes,
    }
}

const fn pos(keyframes: &'static [Keyframe]) -> AnimationChannel {
    AnimationChannel {
        target: AnimationTarget::Position,
        keyframes,
    }
}

const fn scale_channel(keyframes: &'static [Keyframe]) -> AnimationChannel {
    AnimationChannel {
        target: AnimationTarget::Scale,
        keyframes,
    }
}

// Vanilla 26.1 `AdultFoxModel.createBodyLayer` (atlas 48×32). `FoxModel extends EntityModel`: six root
// parts — the `head` (carrying the two ears and the snout), the pitched `body` (carrying the tail), and
// the four legs (all sharing one fudge-inflated 2×6×2 box built off-center at `+2` X). `FoxModel.setupAnim`
// (with its `AdultFoxModel` overrides) is mirrored end to end off the synced `Fox.DATA_FLAGS_ID` (19) and
// the two eased client accumulators (`getHeadRollAngle` / `getCrouchAmount`): the always-run
// `setWalkingPose` tilts the head (`head.zRot = headRollAngle`), keeps the four legs visible and sweeps
// the adult gait (`cos·1.4·speed`); then one of `setCrouchingPose` / `setSleepingPose` / `setSittingPose`
// (the crouch/sleep/sit branch), the pounce drop, the resting head look, the sleeping head wobble, and the
// faceplant leg twitch. The sleeping pose hides all four legs via `ModelPart::visible`, mirroring how the
// bee hides its stinger.
//
// The baby's `FoxBabyAnimation.FOX_BABY_WALK` keyframe gait is reproduced ([`FOX_BABY_WALK`],
// applied in [`fox_set_walking_pose`] via [`apply_fox_baby_walk`]), so a moving baby scampers with the
// diagonal trot, head lift, leg stretch, and tail cock. Deferred: the `FoxRenderer.setupRotations` body-PITCH flip for
// `isPouncing || isFaceplanted` (a renderer root-transform concern, like the death tip-over); the four
// `Fox.Variant` (red/snow) idle/sleeping textures and the held-item layer (so the colored debug path
// renders one orange tint).

// `head` cubes: the 8×6×6 skull, the two 2×2×1 ears, and the 4×2×3 snout.
pub(in crate::entity_models) const FOX_HEAD_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -2.0, -5.0],
    [8.0, 6.0, 6.0],
    FOX_ORANGE,
    [8.0, 6.0, 6.0],
    [1.0, 5.0],
    false,
)];
pub(in crate::entity_models) const FOX_RIGHT_EAR_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -4.0, -4.0],
    [2.0, 2.0, 1.0],
    FOX_ORANGE,
    [2.0, 2.0, 1.0],
    [8.0, 1.0],
    false,
)];
pub(in crate::entity_models) const FOX_LEFT_EAR_CUBES: [ModelCube; 1] = [ModelCube::new(
    [3.0, -4.0, -4.0],
    [2.0, 2.0, 1.0],
    FOX_ORANGE,
    [2.0, 2.0, 1.0],
    [15.0, 1.0],
    false,
)];
pub(in crate::entity_models) const FOX_NOSE_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 2.01, -8.0],
    [4.0, 2.0, 3.0],
    FOX_ORANGE,
    [4.0, 2.0, 3.0],
    [6.0, 18.0],
    false,
)];

// `body`: the 6×11×6 trunk (pitched onto its belly).
pub(in crate::entity_models) const FOX_BODY_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-3.0, 3.999, -3.5],
    [6.0, 11.0, 6.0],
    FOX_ORANGE,
    [6.0, 11.0, 6.0],
    [24.0, 15.0],
    false,
)];

// `tail`: the 4×9×5 brush.
pub(in crate::entity_models) const FOX_TAIL_CUBES: [ModelCube; 1] = [ModelCube::new(
    [2.0, 0.0, -1.0],
    [4.0, 9.0, 5.0],
    FOX_ORANGE,
    [4.0, 9.0, 5.0],
    [30.0, 0.0],
    false,
)];

// The shared leg box (all four reuse its geometry), inflated by the vanilla `CubeDeformation(0.001)`
// fudge (min -= 0.001, size += 0.002) and built off-center at `+2` X. The `uv_size` is the un-inflated
// vanilla `[2, 6, 2]`. Vanilla builds the right and left legs from two different `CubeListBuilder`s with
// different `texOffs` (and `mirror = false`), so the two sides carry distinct `tex_offs` over the same
// geometry: right legs `texOffs(13, 24)`, left legs `texOffs(4, 24)`.
pub(in crate::entity_models) const FOX_LEG_CUBES: [ModelCube; 1] = [ModelCube::new(
    [1.999, 0.499, -1.001],
    [2.002, 6.002, 2.002],
    FOX_ORANGE,
    [2.0, 6.0, 2.0],
    [13.0, 24.0],
    false,
)];
pub(in crate::entity_models) const FOX_LEFT_LEG_CUBES: [ModelCube; 1] = [ModelCube::new(
    [1.999, 0.499, -1.001],
    [2.002, 6.002, 2.002],
    FOX_ORANGE,
    [2.0, 6.0, 2.0],
    [4.0, 24.0],
    false,
)];

/// Vanilla `AdultFoxModel.createBodyLayer` rest-pose hierarchy (`addOrReplaceChild` order): `head`
/// (with ears + snout), `body` (pitched `π/2`, with the tail), then the right-hind / left-hind /
/// right-front / left-front legs. Ten cubes.
/// `head` part pose: `PartPose.offset(-1, 16.5, -3)`.
pub(in crate::entity_models) const FOX_HEAD_POSE: PartPose = PartPose {
    offset: [-1.0, 16.5, -3.0],
    rotation: [0.0, 0.0, 0.0],
};
/// The two ears and the snout all sit at the `PartPose.ZERO` head origin.
pub(in crate::entity_models) const FOX_HEAD_CHILD_POSE: PartPose = PART_POSE_ZERO;
/// `body` part pose: `PartPose.offsetAndRotation(0, 16, -6, π/2, 0, 0)`.
pub(in crate::entity_models) const FOX_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 16.0, -6.0],
    rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
};
/// `tail` part pose: `PartPose.offsetAndRotation(-4, 15, -1, -0.05235988, 0, 0)`.
pub(in crate::entity_models) const FOX_TAIL_POSE: PartPose = PartPose {
    offset: [-4.0, 15.0, -1.0],
    rotation: [-0.05235988, 0.0, 0.0],
};
/// `right_hind_leg` part pose: `PartPose.offset(-5, 17.5, 7)`.
pub(in crate::entity_models) const FOX_RIGHT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-5.0, 17.5, 7.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_hind_leg` part pose: `PartPose.offset(-1, 17.5, 7)`.
pub(in crate::entity_models) const FOX_LEFT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-1.0, 17.5, 7.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_front_leg` part pose: `PartPose.offset(-5, 17.5, 0)`.
pub(in crate::entity_models) const FOX_RIGHT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-5.0, 17.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_front_leg` part pose: `PartPose.offset(-1, 17.5, 0)`.
pub(in crate::entity_models) const FOX_LEFT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-1.0, 17.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds the adult fox's six named root parts under a synthetic root: the cube-bearing `head`
/// (parenting `right_ear`/`left_ear`/`nose`), the pitched `body` (parenting the `tail`), and the four
/// legs, in the vanilla `addOrReplaceChild` order.
fn fox_root() -> ModelPart {
    let head = ModelPart::new(
        FOX_HEAD_POSE,
        FOX_HEAD_CUBES.to_vec(),
        vec![
            (
                "right_ear",
                ModelPart::leaf(FOX_HEAD_CHILD_POSE, FOX_RIGHT_EAR_CUBES.to_vec()),
            ),
            (
                "left_ear",
                ModelPart::leaf(FOX_HEAD_CHILD_POSE, FOX_LEFT_EAR_CUBES.to_vec()),
            ),
            (
                "nose",
                ModelPart::leaf(FOX_HEAD_CHILD_POSE, FOX_NOSE_CUBES.to_vec()),
            ),
        ],
    );
    let body = ModelPart::new(
        FOX_BODY_POSE,
        FOX_BODY_CUBES.to_vec(),
        vec![(
            "tail",
            ModelPart::leaf(FOX_TAIL_POSE, FOX_TAIL_CUBES.to_vec()),
        )],
    );
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            ("head", head),
            ("body", body),
            (
                "right_hind_leg",
                ModelPart::leaf(FOX_RIGHT_HIND_LEG_POSE, FOX_LEG_CUBES.to_vec()),
            ),
            (
                "left_hind_leg",
                ModelPart::leaf(FOX_LEFT_HIND_LEG_POSE, FOX_LEFT_LEG_CUBES.to_vec()),
            ),
            (
                "right_front_leg",
                ModelPart::leaf(FOX_RIGHT_FRONT_LEG_POSE, FOX_LEG_CUBES.to_vec()),
            ),
            (
                "left_front_leg",
                ModelPart::leaf(FOX_LEFT_FRONT_LEG_POSE, FOX_LEFT_LEG_CUBES.to_vec()),
            ),
        ],
    )
}

// Vanilla `BabyFoxModel.createBodyLayer` (atlas 32×32). Flatter layout than the adult — the head bakes
// the ears and snout as cubes (no child parts), the body has no `π/2` pitch, and the root child order is
// head / four legs / body (the body still parenting the tail).

// `head` cubes: the 6×5×5 skull, the 2×2×2 snout, and the two 2×2×1 ears.
pub(in crate::entity_models) const BABY_FOX_HEAD_CUBES: [ModelCube; 4] = [
    ModelCube::new(
        [-3.0, -2.125, -5.125],
        [6.0, 5.0, 5.0],
        FOX_ORANGE,
        [6.0, 5.0, 5.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-1.0, 0.875, -7.125],
        [2.0, 2.0, 2.0],
        FOX_ORANGE,
        [2.0, 2.0, 2.0],
        [18.0, 20.0],
        false,
    ),
    ModelCube::new(
        [-3.0, -4.125, -4.125],
        [2.0, 2.0, 1.0],
        FOX_ORANGE,
        [2.0, 2.0, 1.0],
        [22.0, 8.0],
        false,
    ),
    ModelCube::new(
        [1.0, -4.125, -4.125],
        [2.0, 2.0, 1.0],
        FOX_ORANGE,
        [2.0, 2.0, 1.0],
        [22.0, 11.0],
        false,
    ),
];

// The shared 2×2×2 baby leg box. Vanilla builds the right and left legs from two different
// `CubeListBuilder`s (`mirror = false`), so the two sides carry distinct `tex_offs` over the same
// geometry: right legs `texOffs(22, 4)`, left legs `texOffs(22, 0)`.
pub(in crate::entity_models) const BABY_FOX_LEG_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 2.0, 2.0],
    FOX_ORANGE,
    [2.0, 2.0, 2.0],
    [22.0, 4.0],
    false,
)];
pub(in crate::entity_models) const BABY_FOX_LEFT_LEG_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 2.0, 2.0],
    FOX_ORANGE,
    [2.0, 2.0, 2.0],
    [22.0, 0.0],
    false,
)];

// `body`: the 5×4×6 trunk.
pub(in crate::entity_models) const BABY_FOX_BODY_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-2.5, -2.0, -3.0],
    [5.0, 4.0, 6.0],
    FOX_ORANGE,
    [5.0, 4.0, 6.0],
    [0.0, 10.0],
    false,
)];

// `tail`: the 3×3×6 brush.
pub(in crate::entity_models) const BABY_FOX_TAIL_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.5, -1.48, -1.0],
    [3.0, 3.0, 6.0],
    FOX_ORANGE,
    [3.0, 3.0, 6.0],
    [0.0, 20.0],
    false,
)];

/// Vanilla `BabyFoxModel.createBodyLayer` rest-pose hierarchy (`addOrReplaceChild` order): `head`
/// (ears + snout baked in), the right-hind / left-hind / right-front / left-front legs, then the
/// `body` (with the tail). Ten cubes.
/// Baby `head` part pose: `PartPose.offset(0, 18.125, 0.125)`.
pub(in crate::entity_models) const BABY_FOX_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 18.125, 0.125],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `right_hind_leg` part pose: `PartPose.offset(-1.5, 22, 4)`.
pub(in crate::entity_models) const BABY_FOX_RIGHT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-1.5, 22.0, 4.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `left_hind_leg` part pose: `PartPose.offset(1.5, 22, 4)`.
pub(in crate::entity_models) const BABY_FOX_LEFT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [1.5, 22.0, 4.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `right_front_leg` part pose: `PartPose.offset(-1.5, 22, 0)`.
pub(in crate::entity_models) const BABY_FOX_RIGHT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-1.5, 22.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `left_front_leg` part pose: `PartPose.offset(1.5, 22, 0)`.
pub(in crate::entity_models) const BABY_FOX_LEFT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [1.5, 22.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `body` part pose: `PartPose.offset(0, 20, 2)`.
pub(in crate::entity_models) const BABY_FOX_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 20.0, 2.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `tail` part pose: `PartPose.offset(0, -0.5, 3)` (no bind pitch on the baby).
pub(in crate::entity_models) const BABY_FOX_TAIL_POSE: PartPose = PartPose {
    offset: [0.0, -0.5, 3.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds the baby fox's six named root parts under a synthetic root: the `head` (ears + snout baked
/// in as cubes), the four legs, then the `body` (parenting the `tail`), in the vanilla
/// `addOrReplaceChild` order.
fn baby_fox_root() -> ModelPart {
    let body = ModelPart::new(
        BABY_FOX_BODY_POSE,
        BABY_FOX_BODY_CUBES.to_vec(),
        vec![(
            "tail",
            ModelPart::leaf(BABY_FOX_TAIL_POSE, BABY_FOX_TAIL_CUBES.to_vec()),
        )],
    );
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            (
                "head",
                ModelPart::leaf(BABY_FOX_HEAD_POSE, BABY_FOX_HEAD_CUBES.to_vec()),
            ),
            (
                "right_hind_leg",
                ModelPart::leaf(BABY_FOX_RIGHT_HIND_LEG_POSE, BABY_FOX_LEG_CUBES.to_vec()),
            ),
            (
                "left_hind_leg",
                ModelPart::leaf(
                    BABY_FOX_LEFT_HIND_LEG_POSE,
                    BABY_FOX_LEFT_LEG_CUBES.to_vec(),
                ),
            ),
            (
                "right_front_leg",
                ModelPart::leaf(BABY_FOX_RIGHT_FRONT_LEG_POSE, BABY_FOX_LEG_CUBES.to_vec()),
            ),
            (
                "left_front_leg",
                ModelPart::leaf(
                    BABY_FOX_LEFT_FRONT_LEG_POSE,
                    BABY_FOX_LEFT_LEG_CUBES.to_vec(),
                ),
            ),
            ("body", body),
        ],
    )
}

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

/// Vanilla `FoxModel.setWalkingPose` + the `AdultFoxModel` override: always run. Tilts the head by the
/// eased `headRollAngle` (`head.zRot`), keeps all four legs visible (un-doing any prior sleep hide on a
/// reused tree), and — for the adult — sweeps the gait. The legs' bind pose carries no roll, so the
/// `head.zRot` is set; the walk swing sets `xRot` (a no-op at rest).
fn fox_set_walking_pose(root: &mut ModelPart, baby: bool, instance: &EntityModelInstance) {
    let state = &instance.render_state;
    root.child_mut("head").pose.rotation[2] = state.fox_head_roll_angle;
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
    let age_scale = fox_age_scale(baby);
    let crouch = state.fox_crouch_amount;
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
fn fox_set_sitting_pose(root: &mut ModelPart, baby: bool) {
    let age_scale = fox_age_scale(baby);
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
    let drop = instance.render_state.fox_crouch_amount / 2.0;
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

/// Vanilla `state.ageScale` for the fox = `LivingEntity.getAgeScale()` (`0.5` baby / `1.0` adult). The
/// fox does NOT override `getAgeScale`; `Fox.BABY_SCALE = 0.6` only scales the bounding box, not the
/// model. `setCrouchingPose` multiplies the head lift by this.
fn fox_age_scale(baby: bool) -> f32 {
    if baby {
        0.5
    } else {
        1.0
    }
}

/// Vanilla `BabyFoxModel`'s `applyWalk(walkPos, walkSpeed, 1.0, 2.5)` factors: `MAX_WALK_ANIMATION_SPEED`
/// and `WALK_ANIMATION_SCALE_FACTOR`. The speed factor scales `walkPos` into the animation time; the
/// scale factor caps the amplitude (`min(walkSpeed · 2.5, 1.0)`), so a standing baby adds nothing.
const FOX_BABY_WALK_SPEED_FACTOR: f32 = 1.0;
const FOX_BABY_WALK_SCALE_FACTOR: f32 = 2.5;

// ----- `FoxBabyAnimation.FOX_BABY_WALK` (length 0.5s, LOOPING). The baby's scampering gait: the four
// legs kick ±35° in a diagonal trot (back-right & front-left in phase, the other diagonal a half-cycle
// out), each leg also held forward/up (POSITION) and stretched 1.15× on y (SCALE); the head lifts
// (POSITION) and the tail cocks back -2.5°. `posVec` negates y, `degreeVec` converts to radians, and
// `scaleVec` offsets from `1.0`. Applied via `applyWalk`, so every value is scaled by the limb-swing
// amplitude (zero at rest). The `body`/`head` ROTATION channels are vanilla no-ops (single zero keyframe),
// transcribed faithfully. -----
const FOX_BABY_WALK_BODY_ROT: [Keyframe; 1] = [keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR)];
const FOX_BABY_WALK_BODY_POS: [Keyframe; 1] = [keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR)];
const FOX_BABY_WALK_HEAD_ROT: [Keyframe; 1] = [keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR)];
const FOX_BABY_WALK_HEAD_POS: [Keyframe; 1] = [keyframe(0.0, pos_vec(0.0, -1.025, 0.0), LINEAR)];
const FOX_BABY_WALK_RIGHT_HIND_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(-35.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, degree_vec(35.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(-35.0, 0.0, 0.0), CATMULLROM),
];
const FOX_BABY_WALK_RIGHT_HIND_LEG_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.05, 0.6, -0.02), LINEAR),
    keyframe(0.25, pos_vec(0.05, 0.6, -0.02), LINEAR),
    keyframe(0.5, pos_vec(0.05, 0.6, -0.02), LINEAR),
];
const FOX_BABY_WALK_RIGHT_HIND_LEG_SCALE: [Keyframe; 1] =
    [keyframe(0.0, scale_vec(1.0, 1.15, 1.0), LINEAR)];
const FOX_BABY_WALK_LEFT_HIND_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(35.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, degree_vec(-35.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(35.0, 0.0, 0.0), CATMULLROM),
];
const FOX_BABY_WALK_LEFT_HIND_LEG_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(-0.05, 0.6, -0.02), LINEAR),
    keyframe(0.25, pos_vec(-0.05, 0.6, -0.02), LINEAR),
    keyframe(0.5, pos_vec(-0.05, 0.6, -0.02), LINEAR),
];
const FOX_BABY_WALK_LEFT_HIND_LEG_SCALE: [Keyframe; 1] =
    [keyframe(0.0, scale_vec(1.0, 1.15, 1.0), LINEAR)];
const FOX_BABY_WALK_RIGHT_FRONT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(35.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, degree_vec(-35.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(35.0, 0.0, 0.0), CATMULLROM),
];
const FOX_BABY_WALK_RIGHT_FRONT_LEG_POS: [Keyframe; 1] =
    [keyframe(0.0, pos_vec(0.05, 0.6, -0.4), LINEAR)];
const FOX_BABY_WALK_RIGHT_FRONT_LEG_SCALE: [Keyframe; 1] =
    [keyframe(0.0, scale_vec(1.0, 1.15, 1.0), LINEAR)];
const FOX_BABY_WALK_LEFT_FRONT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(-35.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, degree_vec(35.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(-35.0, 0.0, 0.0), CATMULLROM),
];
const FOX_BABY_WALK_LEFT_FRONT_LEG_POS: [Keyframe; 1] =
    [keyframe(0.0, pos_vec(-0.05, 0.6, -0.4), LINEAR)];
const FOX_BABY_WALK_LEFT_FRONT_LEG_SCALE: [Keyframe; 1] =
    [keyframe(0.0, scale_vec(1.0, 1.15, 1.0), LINEAR)];
const FOX_BABY_WALK_TAIL_ROT: [Keyframe; 1] =
    [keyframe(0.0, degree_vec(-2.5, 0.0, 0.0), CATMULLROM)];
const FOX_BABY_WALK_TAIL_POS: [Keyframe; 1] = [keyframe(0.0, pos_vec(0.0, -0.05, 0.0), CATMULLROM)];

const FOX_BABY_WALK_BODY_CHANNELS: [AnimationChannel; 2] =
    [rot(&FOX_BABY_WALK_BODY_ROT), pos(&FOX_BABY_WALK_BODY_POS)];
const FOX_BABY_WALK_HEAD_CHANNELS: [AnimationChannel; 2] =
    [rot(&FOX_BABY_WALK_HEAD_ROT), pos(&FOX_BABY_WALK_HEAD_POS)];
const FOX_BABY_WALK_RIGHT_HIND_LEG_CHANNELS: [AnimationChannel; 3] = [
    rot(&FOX_BABY_WALK_RIGHT_HIND_LEG_ROT),
    pos(&FOX_BABY_WALK_RIGHT_HIND_LEG_POS),
    scale_channel(&FOX_BABY_WALK_RIGHT_HIND_LEG_SCALE),
];
const FOX_BABY_WALK_LEFT_HIND_LEG_CHANNELS: [AnimationChannel; 3] = [
    rot(&FOX_BABY_WALK_LEFT_HIND_LEG_ROT),
    pos(&FOX_BABY_WALK_LEFT_HIND_LEG_POS),
    scale_channel(&FOX_BABY_WALK_LEFT_HIND_LEG_SCALE),
];
const FOX_BABY_WALK_RIGHT_FRONT_LEG_CHANNELS: [AnimationChannel; 3] = [
    rot(&FOX_BABY_WALK_RIGHT_FRONT_LEG_ROT),
    pos(&FOX_BABY_WALK_RIGHT_FRONT_LEG_POS),
    scale_channel(&FOX_BABY_WALK_RIGHT_FRONT_LEG_SCALE),
];
const FOX_BABY_WALK_LEFT_FRONT_LEG_CHANNELS: [AnimationChannel; 3] = [
    rot(&FOX_BABY_WALK_LEFT_FRONT_LEG_ROT),
    pos(&FOX_BABY_WALK_LEFT_FRONT_LEG_POS),
    scale_channel(&FOX_BABY_WALK_LEFT_FRONT_LEG_SCALE),
];
const FOX_BABY_WALK_TAIL_CHANNELS: [AnimationChannel; 2] =
    [rot(&FOX_BABY_WALK_TAIL_ROT), pos(&FOX_BABY_WALK_TAIL_POS)];
const FOX_BABY_WALK_BONES: [BoneAnimation; 7] = [
    BoneAnimation {
        bone: "body",
        channels: &FOX_BABY_WALK_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &FOX_BABY_WALK_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_hind_leg",
        channels: &FOX_BABY_WALK_RIGHT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_hind_leg",
        channels: &FOX_BABY_WALK_LEFT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_front_leg",
        channels: &FOX_BABY_WALK_RIGHT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_front_leg",
        channels: &FOX_BABY_WALK_LEFT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "tail",
        channels: &FOX_BABY_WALK_TAIL_CHANNELS,
    },
];
/// Vanilla `FoxBabyAnimation.FOX_BABY_WALK`: the 0.5s LOOPING baby gait, applied by
/// `BabyFoxModel.setWalkingPose` via `babyWalkAnimation.applyWalk(walkPos, walkSpeed, 1.0, 2.5)`. The
/// renderer drives it off the projected `walk_animation_pos/speed` through [`keyframe_walk_sample`].
pub(in crate::entity_models) const FOX_BABY_WALK: AnimationDefinition = AnimationDefinition {
    length_seconds: 0.5,
    looping: true,
    bones: &FOX_BABY_WALK_BONES,
};

/// Vanilla `BabyFoxModel.setWalkingPose`: after the base `setWalkingPose` (head tilt, legs visible), it
/// applies `FOX_BABY_WALK` via `applyWalk`. The keyframe-walk sample turns the limb-swing position into
/// the loop time and the limb-swing speed into the amplitude (zero at rest), then each bone's
/// position/rotation/scale offsets fold onto its bind pose (`scale` SETs from the `[1, 1, 1]` base, the
/// legs stretching 1.15× on y). A no-op while standing. Mirrors the adult [`apply_fox_adult_leg_swing`]
/// but as a keyframe gait over all seven animated bones (the four legs, the head lift, the tail cock).
fn apply_fox_baby_walk(root: &mut ModelPart, walk_animation_pos: f32, walk_animation_speed: f32) {
    if limb_swing_at_rest(walk_animation_speed) {
        return;
    }
    let (seconds, scale) = keyframe_walk_sample(
        &FOX_BABY_WALK,
        walk_animation_pos,
        walk_animation_speed,
        FOX_BABY_WALK_SPEED_FACTOR,
        FOX_BABY_WALK_SCALE_FACTOR,
    );
    let apply = |part: &mut ModelPart, bone: &str| {
        let (position, rotation, scale_offset) =
            sample_bone_offsets_with_scale(&FOX_BABY_WALK, bone, seconds, scale);
        part.pose = keyframe_animated_pose(part.pose, position, rotation);
        part.scale = keyframe_animated_scale(scale_offset);
    };
    apply(root.child_mut("head"), "head");
    for name in FOX_LEG_NAMES {
        apply(root.child_mut(name), name);
    }
    let body = root.child_mut("body");
    apply(body, "body");
    apply(body.child_mut("tail"), "tail");
}

/// Mutable fox model, mirroring vanilla `AdultFoxModel` / `BabyFoxModel`. The named root parts hang off
/// a synthetic root, built from the baked colored geometry for the selected `baby` layout. The unified
/// tree drives both render paths; `setup_anim` mirrors `FoxModel.setupAnim` faithfully (the renderer
/// pounce/faceplant body pitch stays deferred).
pub(in crate::entity_models) struct FoxModel {
    root: ModelPart,
    baby: bool,
}

impl FoxModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        Self {
            root: if baby { baby_fox_root() } else { fox_root() },
            baby,
        }
    }
}

impl EntityModel for FoxModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let state = &instance.render_state;
        let baby = self.baby;

        // Vanilla `FoxModel.setupAnim`: `setWalkingPose` always runs (head tilt, legs visible, adult gait).
        fox_set_walking_pose(&mut self.root, baby, instance);

        // Then exactly one of the crouch / sleep / sit poses (crouch wins, then sleep, then sit).
        if state.fox_is_crouching {
            fox_set_crouching_pose(&mut self.root, baby, instance);
        } else if state.fox_is_sleeping {
            fox_set_sleeping_pose(&mut self.root, baby);
        } else if state.fox_is_sitting {
            fox_set_sitting_pose(&mut self.root, baby);
        }

        // The pounce drop layers on top of whichever pose ran.
        if state.fox_is_pouncing {
            fox_set_pouncing_pose(&mut self.root, baby, instance);
        }

        // Resting head look: only while not sleeping / faceplanted / crouching (those own the head).
        if !state.fox_is_sleeping && !state.fox_is_faceplanted && !state.fox_is_crouching {
            apply_head_look(
                self.root.child_mut("head"),
                state.head_yaw,
                state.head_pitch,
            );
        }

        // Sleeping head override: a fixed yaw with a slow `ageInTicks` roll wobble.
        if state.fox_is_sleeping {
            let head = self.root.child_mut("head");
            head.pose.rotation[0] = 0.0;
            head.pose.rotation[1] = -PI * 2.0 / 3.0;
            head.pose.rotation[2] = (state.age_in_ticks * 0.027).cos() / 22.0;
        }

        // Faceplant leg twitch: a small `ageInTicks`-driven cosine flap, the diagonals out of phase. The
        // model is rebuilt each frame, so vanilla's persistent `legMotionPos` accumulator is derived from
        // `ageInTicks` here (the same `+= 0.67`/tick cadence, phase-continuous across frames).
        if state.fox_is_faceplanted {
            let leg_motion_pos = state.age_in_ticks * 0.67;
            let twitch = (leg_motion_pos * 0.4662).cos() * 0.1;
            let twitch_offset = (leg_motion_pos * 0.4662 + PI).cos() * 0.1;
            self.root.child_mut("right_hind_leg").pose.rotation[0] = twitch;
            self.root.child_mut("left_hind_leg").pose.rotation[0] = twitch_offset;
            self.root.child_mut("right_front_leg").pose.rotation[0] = twitch_offset;
            self.root.child_mut("left_front_leg").pose.rotation[0] = twitch;
        }
    }
}
