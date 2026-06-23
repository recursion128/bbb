use super::{
    degree_vec, keyframe, pos_vec, AnimationChannel, AnimationDefinition, AnimationTarget,
    BoneAnimation, Keyframe, KeyframeInterpolation, PartPose, BREEZE_SLATE, PART_POSE_ZERO,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::{
    keyframe_animated_pose, keyframe_elapsed_seconds, sample_bone_offsets,
};
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

use KeyframeInterpolation::{CatmullRom, Linear};

// Vanilla 26.1 `BreezeModel.createBodyLayer` (atlas 32×32): the base body layer retains only the
// `head` (with its emissive `eyes` child) and the three `rods` under the `body` pivot; the swirling
// `wind_body` is a separate translucent layer. The colored path approximates the wind body's
// translucent blue with a single representative slate. Each cube carries both render paths' data:
// the colored debug tint (`BREEZE_SLATE`) and the textured `uv_size` / `texOffs` / `mirror`; no
// `CubeDeformation`, so each `uv_size` matches its box `size`. The head is the `texOffs(4, 24)`
// 10×3×4 jaw plate plus the `texOffs(0, 0)` 8×8×8 head cube.
pub(in crate::entity_models) const BREEZE_HEAD: [ModelCube; 2] = [
    ModelCube::new(
        [-5.0, -5.0, -4.2],
        [10.0, 3.0, 4.0],
        BREEZE_SLATE,
        [10.0, 3.0, 4.0],
        [4.0, 24.0],
        false,
    ),
    ModelCube::new(
        [-4.0, -8.0, -4.0],
        [8.0, 8.0, 8.0],
        BREEZE_SLATE,
        [8.0, 8.0, 8.0],
        [0.0, 0.0],
        false,
    ),
];

// All three rods share the same `texOffs(0, 17)` 2×8×2 box; only their bind pose differs.
pub(in crate::entity_models) const BREEZE_ROD: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -3.0],
    [2.0, 8.0, 2.0],
    BREEZE_SLATE,
    [2.0, 8.0, 2.0],
    [0.0, 17.0],
    false,
)];

pub(in crate::entity_models) const BREEZE_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BREEZE_RODS_POSE: PartPose = PartPose {
    offset: [0.0, 8.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BREEZE_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 4.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BREEZE_ROD_1_POSE: PartPose = PartPose {
    offset: [2.5981, -3.0, 1.5],
    rotation: [-2.7489, -1.0472, 3.1416],
};
pub(in crate::entity_models) const BREEZE_ROD_2_POSE: PartPose = PartPose {
    offset: [-2.5981, -3.0, 1.5],
    rotation: [-2.7489, 1.0472, 3.1416],
};
pub(in crate::entity_models) const BREEZE_ROD_3_POSE: PartPose = PartPose {
    offset: [0.0, -3.0, -3.0],
    rotation: [0.3927, 0.0, 0.0],
};

// Vanilla 26.1 `BreezeAnimation.IDLE` (length 2.0s, looping), restricted to the base body layer's
// bones. The head bobs on a CATMULLROM position spline; the rods spin a full `1080°` of yaw per
// cycle (LINEAR) while bobbing on a LINEAR position spline. The `wind_top` / `wind_mid` channels
// drive the deferred wind layer and are omitted here.
const BREEZE_IDLE_HEAD_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CatmullRom),
    keyframe(1.0, pos_vec(0.0, 1.0, 0.0), CatmullRom),
    keyframe(2.0, pos_vec(0.0, 0.0, 0.0), CatmullRom),
];
const BREEZE_IDLE_RODS_ROT: [Keyframe; 2] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), Linear),
    keyframe(2.0, degree_vec(0.0, 1080.0, 0.0), Linear),
];
const BREEZE_IDLE_RODS_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), Linear),
    keyframe(1.0, pos_vec(0.0, -1.0, 0.0), Linear),
    keyframe(2.0, pos_vec(0.0, 0.0, 0.0), Linear),
];

const BREEZE_IDLE_BONES: [BoneAnimation; 2] = [
    BoneAnimation {
        bone: "head",
        channels: &[AnimationChannel {
            target: AnimationTarget::Position,
            keyframes: &BREEZE_IDLE_HEAD_POS,
        }],
    },
    BoneAnimation {
        bone: "rods",
        channels: &[
            AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &BREEZE_IDLE_RODS_ROT,
            },
            AnimationChannel {
                target: AnimationTarget::Position,
                keyframes: &BREEZE_IDLE_RODS_POS,
            },
        ],
    },
];

pub(in crate::entity_models) const BREEZE_IDLE: AnimationDefinition = AnimationDefinition {
    length_seconds: 2.0,
    looping: true,
    bones: &BREEZE_IDLE_BONES,
};

/// Applies the vanilla `BreezeModel.setupAnim` looping `BreezeAnimation.IDLE` to the unified tree: the
/// `head` bobs on its CATMULLROM position spline and the `rods` pivot spins (1080°/cycle yaw) while
/// bobbing, both sampled from `ageInTicks`. The `body` pivot has no IDLE channel and holds its bind
/// pose. The wind body, emissive eyes, and action animations are deferred entity-side state.
fn apply_breeze_anim(root: &mut ModelPart, instance: &EntityModelInstance) {
    let seconds = keyframe_elapsed_seconds(&BREEZE_IDLE, instance.render_state.age_in_ticks * 0.05);
    let body = root.child_mut("body");
    let (head_pos, _) = sample_bone_offsets(&BREEZE_IDLE, "head", seconds, 1.0);
    body.child_mut("head").pose = keyframe_animated_pose(BREEZE_HEAD_POSE, head_pos, [0.0; 3]);
    let (rods_pos, rods_rot) = sample_bone_offsets(&BREEZE_IDLE, "rods", seconds, 1.0);
    body.child_mut("rods").pose = keyframe_animated_pose(BREEZE_RODS_POSE, rods_pos, rods_rot);
}

/// Mutable breeze model, mirroring vanilla `BreezeModel`'s base body layer. A synthetic root holds the
/// `body` pivot → (`head`, `rods` pivot → the three rods at their fixed bind poses); each cube carries
/// both the colored tint and the textured UV, so one tree drives both render paths. `setup_anim` runs
/// [`apply_breeze_anim`]. The wind body, emissive eyes, and action animations are deferred.
pub(in crate::entity_models) struct BreezeModel {
    root: ModelPart,
}

impl BreezeModel {
    pub(in crate::entity_models) fn new() -> Self {
        let rods = ModelPart::new(
            BREEZE_RODS_POSE,
            Vec::new(),
            vec![
                (
                    "rod_1",
                    ModelPart::leaf(BREEZE_ROD_1_POSE, BREEZE_ROD.to_vec()),
                ),
                (
                    "rod_2",
                    ModelPart::leaf(BREEZE_ROD_2_POSE, BREEZE_ROD.to_vec()),
                ),
                (
                    "rod_3",
                    ModelPart::leaf(BREEZE_ROD_3_POSE, BREEZE_ROD.to_vec()),
                ),
            ],
        );
        let body = ModelPart::new(
            BREEZE_BODY_POSE,
            Vec::new(),
            vec![
                (
                    "head",
                    ModelPart::leaf(BREEZE_HEAD_POSE, BREEZE_HEAD.to_vec()),
                ),
                ("rods", rods),
            ],
        );
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![("body", body)]),
        }
    }
}

impl EntityModel for BreezeModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_breeze_anim(&mut self.root, instance);
    }
}
