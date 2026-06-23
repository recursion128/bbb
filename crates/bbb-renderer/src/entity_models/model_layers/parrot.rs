use super::{
    apply_head_look, bind_part as part, bind_part_rot as rpart, model_cube as cube, ModelCubeDesc,
    ModelPartDesc, PartPose, PARROT_BEAK, PARROT_BODY,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

// Vanilla 26.1 `ParrotModel.createBodyLayer` (atlas 32×32). The mesh root holds seven sibling parts
// (body, tail, the two wings, head, and the two legs); the head parents the upper-head block, the
// two beak halves, and the crest feather. Most parts carry a baked rest rotation (the wings are
// additionally flipped `yRot = -π`). The SITTING perch pose is now projected (see
// [`parrot_pose_parts`](crate::entity_models::colored::runtime)): `prepare(SITTING)` raises every
// part `y += 1.9`, folds the legs `xRot += π/2`, pitches the tail `xRot += π/6`, and tucks the wings
// to `zRot = ±0.0873`. The remaining `ParrotModel.setupAnim` motion is deferred — the head look, the
// FLYING leg pitch, the leg walk swing, the wing flap (`zRot = ±(0.0873 + flapAngle)`), the
// body/tail/head flap bob, and the PARTY dance — so a non-sitting parrot renders at this STANDING rest
// pose. The five `Parrot.Variant` colors live on the deferred texture-backed path, so the colored
// debug path renders one body tint plus a beak tint. Parrot uses a plain `MobRenderer` with no
// transform overrides.

// `body`: the 3×6×3 torso.
const PARROT_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.5, 0.0, -1.5], [3.0, 6.0, 3.0], PARROT_BODY)];

// `tail`: the 3×4×1 plate.
const PARROT_TAIL_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.5, -1.0, -1.0], [3.0, 4.0, 1.0], PARROT_BODY)];

// The shared 1×5×3 wing (both wings reuse it, differing only in pivot X sign).
const PARROT_WING_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, 0.0, -1.5], [1.0, 5.0, 3.0], PARROT_BODY)];

// `head`: the 2×3×2 skull.
const PARROT_HEAD_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, -1.5, -1.0], [2.0, 3.0, 2.0], PARROT_BODY)];

// `head2`: the 2×1×4 upper-head block.
const PARROT_HEAD2_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, -0.5, -2.0], [2.0, 1.0, 4.0], PARROT_BODY)];

// `beak1` / `beak2`: the two 1×2×1 beak halves.
const PARROT_BEAK1_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, -1.0, -0.5], [1.0, 2.0, 1.0], PARROT_BEAK)];
const PARROT_BEAK2_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, 0.0, -0.5], [1.0, 2.0, 1.0], PARROT_BEAK)];

// `feather`: the 0×5×4 crest plane.
const PARROT_FEATHER_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, -4.0, -2.0], [0.0, 5.0, 4.0], PARROT_BODY)];

// The shared 1×2×1 leg (both legs reuse it, differing only in pivot X sign).
const PARROT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, 0.0, -0.5], [1.0, 2.0, 1.0], PARROT_BODY)];

const PARROT_HEAD_CHILDREN: [ModelPartDesc; 4] = [
    part([0.0, -2.0, -1.0], &PARROT_HEAD2_CUBES, &[]),
    part([0.0, -0.5, -1.5], &PARROT_BEAK1_CUBES, &[]),
    part([0.0, -1.75, -2.45], &PARROT_BEAK2_CUBES, &[]),
    rpart(
        [0.0, -2.15, 0.15],
        [-0.2214, 0.0, 0.0],
        &PARROT_FEATHER_CUBES,
        &[],
    ),
];

pub(in crate::entity_models) const PARROT_PARTS: [ModelPartDesc; 7] = [
    rpart(
        [0.0, 16.5, -3.0],
        [0.4937, 0.0, 0.0],
        &PARROT_BODY_CUBES,
        &[],
    ),
    rpart(
        [0.0, 21.07, 1.16],
        [1.015, 0.0, 0.0],
        &PARROT_TAIL_CUBES,
        &[],
    ),
    rpart(
        [1.5, 16.94, -2.76],
        [-0.6981, -std::f32::consts::PI, 0.0],
        &PARROT_WING_CUBES,
        &[],
    ),
    rpart(
        [-1.5, 16.94, -2.76],
        [-0.6981, -std::f32::consts::PI, 0.0],
        &PARROT_WING_CUBES,
        &[],
    ),
    part(
        [0.0, 15.69, -2.76],
        &PARROT_HEAD_CUBES,
        &PARROT_HEAD_CHILDREN,
    ),
    rpart(
        [1.0, 22.0, -1.05],
        [-0.0299, 0.0, 0.0],
        &PARROT_LEG_CUBES,
        &[],
    ),
    rpart(
        [-1.0, 22.0, -1.05],
        [-0.0299, 0.0, 0.0],
        &PARROT_LEG_CUBES,
        &[],
    ),
];

/// The `head` is the fifth sibling in the part tree (body, tail, left_wing, right_wing, head,
/// left_leg, right_leg). `ParrotModel.setupAnim` sets `head.xRot/yRot` from the look angles before
/// the per-pose switch, so the head look applies at every pose the renderer projects (STANDING and
/// SITTING); only the un-projected PARTY pose would overwrite it.
pub(in crate::entity_models) const PARROT_HEAD_PART_INDEX: usize = 4;

/// Vanilla `ParrotModel.prepare(SITTING)` applied in place to the model root's seven sibling parts
/// (part order body, tail, left_wing, right_wing, head, left_leg, right_leg): every part raises
/// `y += 1.9`, the tail pitches `xRot += π/6`, the wings tuck to `zRot = ±0.0873` (set, not added),
/// and the legs fold `xRot += π/2`. The `setupAnim` `SITTING` branch adds nothing more.
fn apply_parrot_sitting_pose(root: &mut ModelPart) {
    const SIT_Y: f32 = 1.9;
    const WING_TUCK_Z_ROT: f32 = 0.0873;
    for index in 0..PARROT_PARTS.len() {
        root.child_at_mut(index).pose.offset[1] += SIT_Y;
    }
    root.child_at_mut(PARROT_TAIL_PART_INDEX).pose.rotation[0] += std::f32::consts::FRAC_PI_6;
    root.child_at_mut(2).pose.rotation[2] = -WING_TUCK_Z_ROT; // left wing tuck
    root.child_at_mut(3).pose.rotation[2] = WING_TUCK_Z_ROT; // right wing tuck
    for index in PARROT_LEG_PART_INDICES {
        root.child_at_mut(index).pose.rotation[0] += std::f32::consts::FRAC_PI_2;
    }
}

/// The `tail` is the second sibling and the two legs are the sixth/seventh; `ParrotModel.setupAnim`
/// adds the STANDING walk swing onto their baked pitch.
pub(in crate::entity_models) const PARROT_TAIL_PART_INDEX: usize = 1;
pub(in crate::entity_models) const PARROT_LEG_PART_INDICES: [usize; 2] = [5, 6];

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

/// Mutable parrot model, mirroring vanilla `ParrotModel`. Its seven sibling parts hang off a
/// synthetic root, each built from the baked [`PARROT_PARTS`] geometry. Colored-only (no textured
/// path yet): `setup_anim` applies the head look at every projected pose, then either the
/// `prepare(SITTING)` perch re-pose or the STANDING tail/leg walk swing.
pub(in crate::entity_models) struct ParrotModel {
    root: ModelPart,
}

impl ParrotModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::root_from_colored_descs(&PARROT_PARTS),
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
        // Vanilla `ParrotModel.setupAnim` sets the head look first (applied at every projected pose),
        // then runs the per-pose switch: SITTING re-poses the whole tree via `prepare(SITTING)` and
        // stops, while STANDING adds the tail/leg walk swing onto the baked pitch. The wing flap and
        // body/head bob (the un-projected `flapAngle`), the FLYING leg pitch, and the PARTY dance
        // stay deferred. Both the head look (identity at a neutral gaze) and the swing (identity at
        // rest) collapse to the bind pose, so they apply unconditionally.
        let sitting = instance.render_state.parrot_sitting;
        if sitting {
            apply_parrot_sitting_pose(&mut self.root);
        }
        apply_head_look(
            self.root.child_at_mut(PARROT_HEAD_PART_INDEX),
            instance.render_state.head_yaw,
            instance.render_state.head_pitch,
        );
        if !sitting {
            let walk_pos = instance.render_state.walk_animation_pos;
            let walk_speed = instance.render_state.walk_animation_speed;
            let tail = self.root.child_at_mut(PARROT_TAIL_PART_INDEX);
            tail.pose = parrot_tail_swing_pose(tail.pose, walk_pos, walk_speed);
            for index in PARROT_LEG_PART_INDICES {
                let leg = self.root.child_at_mut(index);
                leg.pose = parrot_leg_swing_pose(leg.pose, walk_pos, walk_speed);
            }
        }
    }
}
