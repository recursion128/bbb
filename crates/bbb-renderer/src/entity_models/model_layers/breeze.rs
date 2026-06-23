use super::{
    degree_vec, keyframe, pos_vec, AnimationChannel, AnimationDefinition, AnimationTarget,
    BoneAnimation, Keyframe, KeyframeInterpolation, ModelCubeDesc, ModelPartDesc, PartPose,
    TexturedModelCubeDesc, TexturedModelPartDesc, BREEZE_SLATE,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::{
    keyframe_animated_pose, keyframe_elapsed_seconds, sample_bone_offsets,
};
use crate::entity_models::model::{EntityModel, ModelPart};

use KeyframeInterpolation::{CatmullRom, Linear};

// Vanilla 26.1 `BreezeModel.createBodyLayer` (atlas 32×32): the base body layer retains only the
// `head` (with its emissive `eyes` child) and the three `rods` under the `body` pivot; the swirling
// `wind_body` is a separate translucent layer. The colored path approximates the wind body's
// translucent blue with a single representative slate.
pub(in crate::entity_models) const BREEZE_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-5.0, -5.0, -4.2],
        size: [10.0, 3.0, 4.0],
        color: BREEZE_SLATE,
    },
    ModelCubeDesc {
        min: [-4.0, -8.0, -4.0],
        size: [8.0, 8.0, 8.0],
        color: BREEZE_SLATE,
    },
];

// All three rods share the same `texOffs(0, 17)` 2×8×2 box; only their bind pose differs.
pub(in crate::entity_models) const BREEZE_ROD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -3.0],
    size: [2.0, 8.0, 2.0],
    color: BREEZE_SLATE,
}];

// The same geometry with the vanilla `BreezeModel.createBaseMesh` texOffs UV coordinates (atlas
// 32×32); no `CubeDeformation`, so each `uv_size` matches its box `size`.
pub(in crate::entity_models) const BREEZE_TEXTURED_HEAD: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-5.0, -5.0, -4.2],
        size: [10.0, 3.0, 4.0],
        uv_size: [10.0, 3.0, 4.0],
        tex: [4.0, 24.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-4.0, -8.0, -4.0],
        size: [8.0, 8.0, 8.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BREEZE_TEXTURED_ROD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -3.0],
        size: [2.0, 8.0, 2.0],
        uv_size: [2.0, 8.0, 2.0],
        tex: [0.0, 17.0],
        mirror: false,
    }];

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

// Colored breeze tree: `body` (the pivot, no cubes) → `head`, `rods` (pivot, no cubes); `rods` → the
// three rods at their fixed bind poses. Mirrors vanilla `BreezeModel`'s base body layer (the swirling
// wind body, the emissive eyes, and the action animations are deferred). Zipped with the textured
// tree by `BreezeModel::new`; the IDLE keyframe sample is applied in `setup_anim`.
const BREEZE_RODS_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: BREEZE_ROD_1_POSE,
        cubes: &BREEZE_ROD,
        children: &[],
    },
    ModelPartDesc {
        pose: BREEZE_ROD_2_POSE,
        cubes: &BREEZE_ROD,
        children: &[],
    },
    ModelPartDesc {
        pose: BREEZE_ROD_3_POSE,
        cubes: &BREEZE_ROD,
        children: &[],
    },
];
const BREEZE_BODY_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: BREEZE_HEAD_POSE,
        cubes: &BREEZE_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: BREEZE_RODS_POSE,
        cubes: &[],
        children: &BREEZE_RODS_CHILDREN,
    },
];
pub(in crate::entity_models) const BREEZE_PARTS: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: BREEZE_BODY_POSE,
    cubes: &[],
    children: &BREEZE_BODY_CHILDREN,
}];

// Textured counterpart of `BREEZE_PARTS` (same hierarchy and bind poses, UV cubes).
const BREEZE_TEXTURED_RODS_CHILDREN: [TexturedModelPartDesc; 3] = [
    TexturedModelPartDesc {
        pose: BREEZE_ROD_1_POSE,
        cubes: &BREEZE_TEXTURED_ROD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BREEZE_ROD_2_POSE,
        cubes: &BREEZE_TEXTURED_ROD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BREEZE_ROD_3_POSE,
        cubes: &BREEZE_TEXTURED_ROD,
        children: &[],
    },
];
const BREEZE_TEXTURED_BODY_CHILDREN: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: BREEZE_HEAD_POSE,
        cubes: &BREEZE_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BREEZE_RODS_POSE,
        cubes: &[],
        children: &BREEZE_TEXTURED_RODS_CHILDREN,
    },
];
pub(in crate::entity_models) const BREEZE_TEXTURED_PARTS: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: BREEZE_BODY_POSE,
        cubes: &[],
        children: &BREEZE_TEXTURED_BODY_CHILDREN,
    }];

/// Applies the vanilla `BreezeModel.setupAnim` looping `BreezeAnimation.IDLE` to the unified tree: the
/// `head` bobs on its CATMULLROM position spline and the `rods` pivot spins (1080°/cycle yaw) while
/// bobbing, both sampled from `ageInTicks`. The `body` pivot has no IDLE channel and holds its bind
/// pose. The wind body, emissive eyes, and action animations are deferred entity-side state.
fn apply_breeze_anim(root: &mut ModelPart, instance: &EntityModelInstance) {
    let seconds = keyframe_elapsed_seconds(&BREEZE_IDLE, instance.render_state.age_in_ticks * 0.05);
    let body = root.child_at_mut(0);
    let (head_pos, _) = sample_bone_offsets(&BREEZE_IDLE, "head", seconds, 1.0);
    body.child_at_mut(0).pose = keyframe_animated_pose(BREEZE_HEAD_POSE, head_pos, [0.0; 3]);
    let (rods_pos, rods_rot) = sample_bone_offsets(&BREEZE_IDLE, "rods", seconds, 1.0);
    body.child_at_mut(1).pose = keyframe_animated_pose(BREEZE_RODS_POSE, rods_pos, rods_rot);
}

/// Mutable breeze model, mirroring vanilla `BreezeModel`'s base body layer. The unified tree is zipped
/// from the `body` → (head, rods → three rods) hierarchy ([`BREEZE_PARTS`] / [`BREEZE_TEXTURED_PARTS`]);
/// `setup_anim` runs [`apply_breeze_anim`]. The same posed tree drives the colored fallback and the
/// single translucent textured layer.
pub(in crate::entity_models) struct BreezeModel {
    root: ModelPart,
}

impl BreezeModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::root_from_descs(&BREEZE_PARTS, &BREEZE_TEXTURED_PARTS),
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
