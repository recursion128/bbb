use super::{apply_head_look, PartPose, PARROT_BEAK, PARROT_BODY, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_PARROT: &str = "minecraft:parrot#main";

// Vanilla 26.1 `ParrotModel.createBodyLayer` (atlas 32×32). The mesh root holds seven sibling parts
// (body, tail, the two wings, head, and the two legs); the head parents the upper-head block, the
// two beak halves, and the crest feather. Most parts carry a baked rest rotation (the wings are
// additionally flipped `yRot = -π`). `ParrotModel.getPose` is derived in `setup_anim` from the
// projected `parrot_party`, `parrot_sitting`, and `on_ground` render-state fields: PARTY when a
// nearby jukebox is playing, else SITTING when sitting, else FLYING when airborne, else STANDING.
// Each pose mirrors `ParrotModel.prepare(pose)` + the per-`Pose`
// `setupAnim` switch:
//   - PARTY: `prepare` splays the legs around z, then `setupAnim` moves head/body/wings/tail by
//     `cos(age)`/`sin(age)`, zeros head look, rolls the head by `sin(age) * 0.4`, and still applies
//     the wing flap zRot.
//   - SITTING: `prepare` raises every part `y += 1.9`, folds the legs `xRot += π/2`, pitches the tail
//     `xRot += π/6`, and tucks the wings to `zRot = ±0.0873`; `setupAnim` adds nothing (only the head
//     look, which precedes the switch).
//   - STANDING: the leg walk swing, then the shared bob/wing-flap block (vanilla's STANDING case falls
//     through to FLYING/default).
//   - FLYING: `prepare` pitches both legs `xRot += 2π/9`, then the shared bob/wing-flap block (no leg
//     walk swing — FLYING does not fall through STANDING).
// The shared bob/wing-flap block: `bob = flapAngle * 0.3` raises head/tail/body/wings/legs `y += bob`,
// the tail adds its walk swing, and the wings set `zRot = ±(0.0873 + flapAngle)`. `flapAngle` is the
// projected `parrot_flap_angle` (vanilla `(sin(flap) + 1) * flapSpeed`); a grounded parrot has
// `flapSpeed == 0 → flapAngle == 0`, so the wings settle to `zRot = ±0.0873` and the bob vanishes.
// DEFERRED: the ON_SHOULDER pose (needs the shoulder-riding render path) — a shoulder parrot falls
// back to STANDING/FLYING. The five `Parrot.Variant` colors live on the texture-backed path, so the
// colored debug path renders one body tint plus a beak tint. Parrot uses a plain `MobRenderer` with no
// transform overrides.

// `body`: the 3×6×3 torso at texOffs(2, 8).
pub(in crate::entity_models) const PARROT_BODY_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.5, 0.0, -1.5],
    [3.0, 6.0, 3.0],
    PARROT_BODY,
    [3.0, 6.0, 3.0],
    [2.0, 8.0],
    false,
)];

// `tail`: the 3×4×1 plate at texOffs(22, 1).
pub(in crate::entity_models) const PARROT_TAIL_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.5, -1.0, -1.0],
    [3.0, 4.0, 1.0],
    PARROT_BODY,
    [3.0, 4.0, 1.0],
    [22.0, 1.0],
    false,
)];

// The shared 1×5×3 wing at texOffs(19, 8) (both wings reuse it — same texOffs, no mirror — differing
// only in pivot X sign and the `yRot = -π` flip).
pub(in crate::entity_models) const PARROT_WING_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-0.5, 0.0, -1.5],
    [1.0, 5.0, 3.0],
    PARROT_BODY,
    [1.0, 5.0, 3.0],
    [19.0, 8.0],
    false,
)];

// `head`: the 2×3×2 skull at texOffs(2, 2).
pub(in crate::entity_models) const PARROT_HEAD_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -1.5, -1.0],
    [2.0, 3.0, 2.0],
    PARROT_BODY,
    [2.0, 3.0, 2.0],
    [2.0, 2.0],
    false,
)];

// `head2`: the 2×1×4 upper-head block at texOffs(10, 0).
pub(in crate::entity_models) const PARROT_HEAD2_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -0.5, -2.0],
    [2.0, 1.0, 4.0],
    PARROT_BODY,
    [2.0, 1.0, 4.0],
    [10.0, 0.0],
    false,
)];

// `beak1` / `beak2`: the two 1×2×1 beak halves at texOffs(11, 7) and texOffs(16, 7).
pub(in crate::entity_models) const PARROT_BEAK1_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-0.5, -1.0, -0.5],
    [1.0, 2.0, 1.0],
    PARROT_BEAK,
    [1.0, 2.0, 1.0],
    [11.0, 7.0],
    false,
)];
pub(in crate::entity_models) const PARROT_BEAK2_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-0.5, 0.0, -0.5],
    [1.0, 2.0, 1.0],
    PARROT_BEAK,
    [1.0, 2.0, 1.0],
    [16.0, 7.0],
    false,
)];

// `feather`: the 0×5×4 crest plane at texOffs(2, 18).
pub(in crate::entity_models) const PARROT_FEATHER_CUBES: [ModelCube; 1] = [ModelCube::new(
    [0.0, -4.0, -2.0],
    [0.0, 5.0, 4.0],
    PARROT_BODY,
    [0.0, 5.0, 4.0],
    [2.0, 18.0],
    false,
)];

// The shared 1×2×1 leg at texOffs(14, 18) (both legs reuse it — same texOffs, no mirror — differing
// only in pivot X sign).
pub(in crate::entity_models) const PARROT_LEG_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-0.5, 0.0, -0.5],
    [1.0, 2.0, 1.0],
    PARROT_BODY,
    [1.0, 2.0, 1.0],
    [14.0, 18.0],
    false,
)];

/// `body` part pose: `PartPose.offsetAndRotation(0, 16.5, -3, 0.4937, 0, 0)`.
pub(in crate::entity_models) const PARROT_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 16.5, -3.0],
    rotation: [0.4937, 0.0, 0.0],
};
/// `tail` part pose: `PartPose.offsetAndRotation(0, 21.07, 1.16, 1.015, 0, 0)`.
pub(in crate::entity_models) const PARROT_TAIL_POSE: PartPose = PartPose {
    offset: [0.0, 21.07, 1.16],
    rotation: [1.015, 0.0, 0.0],
};
/// `left_wing` part pose: `PartPose.offsetAndRotation(1.5, 16.94, -2.76, -0.6981, -π, 0)`.
pub(in crate::entity_models) const PARROT_LEFT_WING_POSE: PartPose = PartPose {
    offset: [1.5, 16.94, -2.76],
    rotation: [-0.6981, -std::f32::consts::PI, 0.0],
};
/// `right_wing` part pose: `PartPose.offsetAndRotation(-1.5, 16.94, -2.76, -0.6981, -π, 0)`.
pub(in crate::entity_models) const PARROT_RIGHT_WING_POSE: PartPose = PartPose {
    offset: [-1.5, 16.94, -2.76],
    rotation: [-0.6981, -std::f32::consts::PI, 0.0],
};
/// `head` part pose: `PartPose.offset(0, 15.69, -2.76)`.
pub(in crate::entity_models) const PARROT_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 15.69, -2.76],
    rotation: [0.0, 0.0, 0.0],
};
/// `head2` part pose: `PartPose.offset(0, -2, -1)`.
pub(in crate::entity_models) const PARROT_HEAD2_POSE: PartPose = PartPose {
    offset: [0.0, -2.0, -1.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `beak1` part pose: `PartPose.offset(0, -0.5, -1.5)`.
pub(in crate::entity_models) const PARROT_BEAK1_POSE: PartPose = PartPose {
    offset: [0.0, -0.5, -1.5],
    rotation: [0.0, 0.0, 0.0],
};
/// `beak2` part pose: `PartPose.offset(0, -1.75, -2.45)`.
pub(in crate::entity_models) const PARROT_BEAK2_POSE: PartPose = PartPose {
    offset: [0.0, -1.75, -2.45],
    rotation: [0.0, 0.0, 0.0],
};
/// `feather` part pose: `PartPose.offsetAndRotation(0, -2.15, 0.15, -0.2214, 0, 0)`.
pub(in crate::entity_models) const PARROT_FEATHER_POSE: PartPose = PartPose {
    offset: [0.0, -2.15, 0.15],
    rotation: [-0.2214, 0.0, 0.0],
};
/// `left_leg` part pose: `PartPose.offsetAndRotation(1, 22, -1.05, -0.0299, 0, 0)`.
pub(in crate::entity_models) const PARROT_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [1.0, 22.0, -1.05],
    rotation: [-0.0299, 0.0, 0.0],
};
/// `right_leg` part pose: `PartPose.offsetAndRotation(-1, 22, -1.05, -0.0299, 0, 0)`.
pub(in crate::entity_models) const PARROT_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-1.0, 22.0, -1.05],
    rotation: [-0.0299, 0.0, 0.0],
};

/// The seven root sibling part names, in vanilla `addOrReplaceChild` order — used by the
/// `prepare(SITTING)` re-pose to raise every part `y += 1.9`.
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

/// Builds the parrot's seven named sibling root parts under a synthetic root, in the vanilla
/// `addOrReplaceChild` order. The cube-bearing `head` parents `head2`, the two beak halves, and the
/// crest feather (index-named, never addressed by name in `setup_anim`).
fn parrot_root() -> ModelPart {
    let head = ModelPart::new(
        PARROT_HEAD_POSE,
        PARROT_HEAD_CUBES.to_vec(),
        vec![
            (
                "0",
                ModelPart::leaf(PARROT_HEAD2_POSE, PARROT_HEAD2_CUBES.to_vec()),
            ),
            (
                "1",
                ModelPart::leaf(PARROT_BEAK1_POSE, PARROT_BEAK1_CUBES.to_vec()),
            ),
            (
                "2",
                ModelPart::leaf(PARROT_BEAK2_POSE, PARROT_BEAK2_CUBES.to_vec()),
            ),
            (
                "3",
                ModelPart::leaf(PARROT_FEATHER_POSE, PARROT_FEATHER_CUBES.to_vec()),
            ),
        ],
    );
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            (
                "body",
                ModelPart::leaf(PARROT_BODY_POSE, PARROT_BODY_CUBES.to_vec()),
            ),
            (
                "tail",
                ModelPart::leaf(PARROT_TAIL_POSE, PARROT_TAIL_CUBES.to_vec()),
            ),
            (
                "left_wing",
                ModelPart::leaf(PARROT_LEFT_WING_POSE, PARROT_WING_CUBES.to_vec()),
            ),
            (
                "right_wing",
                ModelPart::leaf(PARROT_RIGHT_WING_POSE, PARROT_WING_CUBES.to_vec()),
            ),
            ("head", head),
            (
                "left_leg",
                ModelPart::leaf(PARROT_LEFT_LEG_POSE, PARROT_LEG_CUBES.to_vec()),
            ),
            (
                "right_leg",
                ModelPart::leaf(PARROT_RIGHT_LEG_POSE, PARROT_LEG_CUBES.to_vec()),
            ),
        ],
    )
}

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
/// synthetic root, each built from the baked geometry (carrying both the colored tint and the textured
/// UV). `setup_anim` derives the pose from `parrot_party`/`parrot_sitting`/`parrot_on_shoulder`/
/// `on_ground`, sets the head look, then runs the per-pose `prepare` + `setupAnim` math — the PARTY
/// dance, the SITTING perch, the ON_SHOULDER shared bob/wing block, the STANDING leg walk swing plus
/// bob/wing-flap, or the FLYING leg pitch plus bob/wing-flap.
pub(in crate::entity_models) struct ParrotModel {
    root: ModelPart,
}

impl ParrotModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: parrot_root(),
        }
    }
}

impl EntityModel for ParrotModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `ParrotModel.getPose(entity)`: PARTY wins over SITTING, `ParrotOnShoulderLayer`
        // supplies ON_SHOULDER directly, then FLYING is selected from `isFlying() = !onGround()`.
        let render_state = &instance.render_state;
        let party = render_state.parrot_party;
        let sitting = !party && render_state.parrot_sitting;
        let on_shoulder = !party && !sitting && render_state.parrot_on_shoulder;
        let flying = !party && !sitting && !on_shoulder && !render_state.on_ground;

        // Vanilla `prepare(pose)` runs before the head look in `setupAnim`'s body, but it only
        // translates parts or adjusts legs; PARTY later overwrites head x/y look rotations exactly
        // like vanilla's switch branch.
        if party {
            apply_parrot_party_prepare(&mut self.root);
        } else if sitting {
            apply_parrot_sitting_pose(&mut self.root);
        } else if flying {
            for name in PARROT_LEG_NAMES {
                self.root.child_mut(name).pose.rotation[0] += PARROT_FLYING_LEG_X_ROT;
            }
        }

        // Vanilla `ParrotModel.setupAnim`: `head.xRot/yRot` set from the look angles before the
        // per-pose switch. Identity at a neutral gaze, so it applies at every pose.
        apply_head_look(
            self.root.child_mut("head"),
            render_state.head_yaw,
            render_state.head_pitch,
        );

        // The per-pose switch. SITTING breaks immediately (its pose is entirely in `prepare`).
        // STANDING adds the leg walk swing, then falls through to the shared bob/wing-flap block;
        // FLYING and ON_SHOULDER run only that shared block.
        if party {
            apply_parrot_party_pose(
                &mut self.root,
                render_state.age_in_ticks,
                render_state.parrot_flap_angle,
            );
            return;
        }
        if sitting {
            return;
        }
        let walk_pos = render_state.walk_animation_pos;
        let walk_speed = render_state.walk_animation_speed;
        if !flying && !on_shoulder {
            // STANDING leg walk swing, added onto the baked leg pitch.
            for name in PARROT_LEG_NAMES {
                let leg = self.root.child_mut(name);
                leg.pose = parrot_leg_swing_pose(leg.pose, walk_pos, walk_speed);
            }
        }
        apply_parrot_flap_bob(
            &mut self.root,
            render_state.parrot_flap_angle,
            walk_pos,
            walk_speed,
        );
    }
}
