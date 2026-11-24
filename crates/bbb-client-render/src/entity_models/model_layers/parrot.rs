use super::{apply_head_look, PartPose};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

// Vanilla 26.1 `ParrotModel` (32x32 atlas): body, tail, wings, head, legs, with baked rest
// poses and setup_anim switching between PARTY, SITTING, STANDING, FLYING, and ON_SHOULDER.
const PARROT_PART_NAMES: [&str; 7] = [
    "body",
    "tail",
    "left_wing",
    "right_wing",
    "head",
    "left_leg",
    "right_leg",
];

/// The two legs swing with the STANDING walk; `ParrotModel.setupAnim` adds the swing onto their baked
/// pitch.
const PARROT_LEG_NAMES: [&str; 2] = ["left_leg", "right_leg"];

/// Vanilla `ParrotModel`'s wing rest splay literal `0.0873F`. SITTING tucks the wings to `zRot =
/// ±0.0873` (set); STANDING/FLYING set `zRot = ±(0.0873 + flapAngle)` so a grounded parrot
/// (`flapAngle == 0`) holds this same splay.
const PARROT_WING_REST_Z_ROT: f32 = 0.0873;

/// Vanilla `ParrotModel.prepare(FLYING)` leg pitch: both legs tuck back `xRot += 2π/9`.
const PARROT_FLYING_LEG_X_ROT: f32 = std::f32::consts::PI * 2.0 / 9.0;

/// Vanilla `ParrotModel.prepare(SITTING)` applied in place to the model root's seven sibling parts
/// (body, tail, left_wing, right_wing, head, left_leg, right_leg): every part raises `y += 1.9`, the
/// tail pitches `xRot += π/6`, the wings tuck to `zRot = ±0.0873` (set, not added), and the legs fold
/// `xRot += π/2`. The `setupAnim` `SITTING` branch adds nothing more.
fn apply_parrot_sitting_pose(root: &mut ModelPart) {
    const SIT_Y: f32 = 1.9;
    for name in PARROT_PART_NAMES {
        root.child_mut(name).pose.offset[1] += SIT_Y;
    }
    root.child_mut("tail").pose.rotation[0] += std::f32::consts::FRAC_PI_6;
    root.child_mut("left_wing").pose.rotation[2] = -PARROT_WING_REST_Z_ROT; // left wing tuck
    root.child_mut("right_wing").pose.rotation[2] = PARROT_WING_REST_Z_ROT; // right wing tuck
    for name in PARROT_LEG_NAMES {
        root.child_mut(name).pose.rotation[0] += std::f32::consts::FRAC_PI_2;
    }
}

/// Vanilla `ParrotModel.prepare(PARTY)`: the dance pose splays only the two legs around `zRot`;
/// the rest of the motion is applied in the PARTY branch after the head-look assignment.
fn apply_parrot_party_prepare(root: &mut ModelPart) {
    root.child_mut("left_leg").pose.rotation[2] = -std::f32::consts::PI / 9.0;
    root.child_mut("right_leg").pose.rotation[2] = std::f32::consts::PI / 9.0;
}

/// Vanilla `ParrotModel.setupAnim` PARTY branch. The head look is assigned immediately before the
/// switch, but PARTY overwrites `head.xRot/yRot` to zero and rolls `head.zRot = sin(age) * 0.4`.
fn apply_parrot_party_pose(root: &mut ModelPart, age_in_ticks: f32, flap_angle: f32) {
    let x_pos = age_in_ticks.cos();
    let y_pos = age_in_ticks.sin();

    let head = root.child_mut("head");
    head.pose.offset[0] += x_pos;
    head.pose.offset[1] += y_pos;
    head.pose.rotation[0] = 0.0;
    head.pose.rotation[1] = 0.0;
    head.pose.rotation[2] = y_pos * 0.4;

    let body = root.child_mut("body");
    body.pose.offset[0] += x_pos;
    body.pose.offset[1] += y_pos;

    let left_wing = root.child_mut("left_wing");
    left_wing.pose.rotation[2] = -PARROT_WING_REST_Z_ROT - flap_angle;
    left_wing.pose.offset[0] += x_pos;
    left_wing.pose.offset[1] += y_pos;

    let right_wing = root.child_mut("right_wing");
    right_wing.pose.rotation[2] = PARROT_WING_REST_Z_ROT + flap_angle;
    right_wing.pose.offset[0] += x_pos;
    right_wing.pose.offset[1] += y_pos;

    let tail = root.child_mut("tail");
    tail.pose.offset[0] += x_pos;
    tail.pose.offset[1] += y_pos;
}

/// Vanilla `ParrotModel.setupAnim`'s shared STANDING/FLYING/ON_SHOULDER bob + wing-flap block. After
/// the (optional) STANDING leg walk swing, vanilla falls through to the default case:
///   `bob = flapAngle * 0.3`
///   `head.y += bob; tail.xRot += cos(walkPos·0.6662)·0.3·walkSpeed; tail.y += bob; body.y += bob`
///   `leftWing.zRot = -0.0873 - flapAngle; leftWing.y += bob`
///   `rightWing.zRot = 0.0873 + flapAngle; rightWing.y += bob`
///   `leftLeg.y += bob; rightLeg.y += bob`
/// The head look (head rotation) is set before the switch; this only adds the `y` bob and the tail
/// swing/wing `zRot`, so it composes with the look. With `flapAngle == 0` (a grounded parrot) the bob
/// vanishes and the wings settle to `zRot = ±0.0873`.
fn apply_parrot_flap_bob(root: &mut ModelPart, flap_angle: f32, walk_pos: f32, walk_speed: f32) {
    let bob = flap_angle * 0.3;
    // body, head, both wings, both legs raise by the bob; the tail raises by the bob *and* adds its
    // walk swing (the body/head only carry the bob — the head look already set their rotation).
    for name in ["body", "head", "left_leg", "right_leg"] {
        root.child_mut(name).pose.offset[1] += bob;
    }
    let tail = root.child_mut("tail");
    tail.pose = parrot_tail_swing_pose(tail.pose, walk_pos, walk_speed);
    tail.pose.offset[1] += bob;
    // Wings: set the absolute flap angle off the rest splay, then raise by the bob.
    let left_wing = root.child_mut("left_wing");
    left_wing.pose.rotation[2] = -PARROT_WING_REST_Z_ROT - flap_angle;
    left_wing.pose.offset[1] += bob;
    let right_wing = root.child_mut("right_wing");
    right_wing.pose.rotation[2] = PARROT_WING_REST_Z_ROT + flap_angle;
    right_wing.pose.offset[1] += bob;
}

/// Vanilla `ParrotModel.setupAnim` STANDING leg walk swing for one leg:
/// `leg.xRot += cos(walkAnimationPos·0.6662 [+ π])·1.4·walkAnimationSpeed`. The left leg
/// (`leftLeg`, offset `x > 0`) is in phase and the right (`rightLeg`, `x < 0`) a half-cycle out —
/// the opposite x-sign convention to `QuadrupedModel`/`HumanoidModel`. Unlike those, the swing is
/// ADDED onto the baked leg pitch (`-0.0299`), matching vanilla's `+=`. STANDING only: the SITTING
/// branch breaks before the swing and FLYING/PARTY are not projected.
pub(in crate::entity_models) fn parrot_leg_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let phase = walk_animation_pos * 0.6662;
    let angle = if base.offset[0] > 0.0 {
        phase
    } else {
        phase + std::f32::consts::PI
    };
    PartPose {
        offset: base.offset,
        rotation: [
            base.rotation[0] + angle.cos() * 1.4 * walk_animation_speed,
            base.rotation[1],
            base.rotation[2],
        ],
    }
}

/// Vanilla `ParrotModel.setupAnim` STANDING tail walk swing:
/// `tail.xRot += cos(walkAnimationPos·0.6662)·0.3·walkAnimationSpeed`, added onto the baked tail
/// pitch (`1.015`). Reached through the STANDING fall-through, so the renderer applies it whenever
/// the parrot is not sitting.
pub(in crate::entity_models) fn parrot_tail_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    PartPose {
        offset: base.offset,
        rotation: [
            base.rotation[0] + (walk_animation_pos * 0.6662).cos() * 0.3 * walk_animation_speed,
            base.rotation[1],
            base.rotation[2],
        ],
    }
}

/// Mutable parrot model, mirroring vanilla `ParrotModel`. Its seven named sibling parts hang off a
/// synthetic root. `setup_anim` derives the pose from `parrot_party`/`parrot_sitting`/`parrot_on_shoulder`/
/// `on_ground`, sets the head look, then runs the per-pose `prepare` + `setupAnim` math — the PARTY
/// dance, the SITTING perch, the ON_SHOULDER shared bob/wing block, the STANDING leg walk swing plus
/// bob/wing-flap, or the FLYING leg pitch plus bob/wing-flap.
pub(in crate::entity_models) fn parrot_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::PARROT.bake();
    // Vanilla `ParrotModel.getPose(entity)`: PARTY wins over SITTING, `ParrotOnShoulderLayer`
    // supplies ON_SHOULDER directly, then FLYING is selected from `isFlying() = !onGround()`.
    let render_state = &instance.render_state;
    let party = render_state.kind.parrot().party;
    let sitting = !party && render_state.kind.parrot().sitting;
    let on_shoulder = !party && !sitting && render_state.kind.parrot().on_shoulder;
    let flying = !party && !sitting && !on_shoulder && !render_state.on_ground;

    // Vanilla `prepare(pose)` runs before the head look in `setupAnim`'s body, but it only
    // translates parts or adjusts legs; PARTY later overwrites head x/y look rotations exactly
    // like vanilla's switch branch.
    if party {
        apply_parrot_party_prepare(&mut root);
    } else if sitting {
        apply_parrot_sitting_pose(&mut root);
    } else if flying {
        for name in PARROT_LEG_NAMES {
            root.child_mut(name).pose.rotation[0] += PARROT_FLYING_LEG_X_ROT;
        }
    }

    // Vanilla `ParrotModel.setupAnim`: `head.xRot/yRot` set from the look angles before the
    // per-pose switch. Identity at a neutral gaze, so it applies at every pose.
    apply_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
    );

    // The per-pose switch. SITTING breaks immediately (its pose is entirely in `prepare`).
    // STANDING adds the leg walk swing, then falls through to the shared bob/wing-flap block;
    // FLYING and ON_SHOULDER run only that shared block.
    if party {
        apply_parrot_party_pose(
            &mut root,
            render_state.age_in_ticks,
            render_state.kind.parrot().flap_angle,
        );
        return root;
    }
    if sitting {
        return root;
    }
    let walk_pos = render_state.walk_animation_pos;
    let walk_speed = render_state.walk_animation_speed;
    if !flying && !on_shoulder {
        // STANDING leg walk swing, added onto the baked leg pitch.
        for name in PARROT_LEG_NAMES {
            let leg = root.child_mut(name);
            leg.pose = parrot_leg_swing_pose(leg.pose, walk_pos, walk_speed);
        }
    }
    apply_parrot_flap_bob(
        &mut root,
        render_state.kind.parrot().flap_angle,
        walk_pos,
        walk_speed,
    );
    root
}
