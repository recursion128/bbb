use super::{
    degree_vec, keyframe, pos_vec, AnimationChannel, AnimationDefinition, AnimationTarget,
    BoneAnimation, Keyframe, KeyframeInterpolation, PartPose, BAT_BROWN, PART_POSE_ZERO,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::{
    keyframe_animated_pose, keyframe_elapsed_seconds, sample_bone_offsets,
};
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

use KeyframeInterpolation::Linear;

pub(in crate::entity_models) const MODEL_LAYER_BAT: &str = "minecraft:bat#main";

// Vanilla 26.1 `BatModel.createBodyLayer` (atlas 32×32). The body and head hang under the root;
// the wings (each with a tip) and the feet are children of the body, and the two ears are
// children of the head. The keyframe `BatAnimation.BAT_FLYING` (below) adds per-frame position
// and rotation offsets to these bind poses. Each cube carries both render paths' data: the
// colored debug tint (`BAT_BROWN`) and the textured `uv_size` / `texOffs` / `mirror`. No
// `CubeDeformation`, so each `uv_size` matches its box `size`.
pub(in crate::entity_models) const BAT_BODY: [ModelCube; 1] = [ModelCube::new(
    [-1.5, 0.0, -1.0],
    [3.0, 5.0, 2.0],
    BAT_BROWN,
    [3.0, 5.0, 2.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BAT_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -3.0, -1.0],
    [4.0, 3.0, 2.0],
    BAT_BROWN,
    [4.0, 3.0, 2.0],
    [0.0, 7.0],
    false,
)];

// Ears and wings are zero-thickness planes.
pub(in crate::entity_models) const BAT_RIGHT_EAR: [ModelCube; 1] = [ModelCube::new(
    [-2.5, -4.0, 0.0],
    [3.0, 5.0, 0.0],
    BAT_BROWN,
    [3.0, 5.0, 0.0],
    [1.0, 15.0],
    false,
)];

pub(in crate::entity_models) const BAT_LEFT_EAR: [ModelCube; 1] = [ModelCube::new(
    [-0.1, -3.0, 0.0],
    [3.0, 5.0, 0.0],
    BAT_BROWN,
    [3.0, 5.0, 0.0],
    [8.0, 15.0],
    false,
)];

pub(in crate::entity_models) const BAT_RIGHT_WING: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -2.0, 0.0],
    [2.0, 7.0, 0.0],
    BAT_BROWN,
    [2.0, 7.0, 0.0],
    [12.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BAT_RIGHT_WING_TIP: [ModelCube; 1] = [ModelCube::new(
    [-6.0, -2.0, 0.0],
    [6.0, 8.0, 0.0],
    BAT_BROWN,
    [6.0, 8.0, 0.0],
    [16.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BAT_LEFT_WING: [ModelCube; 1] = [ModelCube::new(
    [0.0, -2.0, 0.0],
    [2.0, 7.0, 0.0],
    BAT_BROWN,
    [2.0, 7.0, 0.0],
    [12.0, 7.0],
    false,
)];

pub(in crate::entity_models) const BAT_LEFT_WING_TIP: [ModelCube; 1] = [ModelCube::new(
    [0.0, -2.0, 0.0],
    [6.0, 8.0, 0.0],
    BAT_BROWN,
    [6.0, 8.0, 0.0],
    [16.0, 8.0],
    false,
)];

pub(in crate::entity_models) const BAT_FEET: [ModelCube; 1] = [ModelCube::new(
    [-1.5, 0.0, 0.0],
    [3.0, 2.0, 0.0],
    BAT_BROWN,
    [3.0, 2.0, 0.0],
    [16.0, 16.0],
    false,
)];

pub(in crate::entity_models) const BAT_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 17.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BAT_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 17.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BAT_RIGHT_EAR_POSE: PartPose = PartPose {
    offset: [-1.5, -2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BAT_LEFT_EAR_POSE: PartPose = PartPose {
    offset: [1.1, -3.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BAT_RIGHT_WING_POSE: PartPose = PartPose {
    offset: [-1.5, 0.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BAT_RIGHT_WING_TIP_POSE: PartPose = PartPose {
    offset: [-2.0, 0.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BAT_LEFT_WING_POSE: PartPose = PartPose {
    offset: [1.5, 0.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BAT_LEFT_WING_TIP_POSE: PartPose = PartPose {
    offset: [2.0, 0.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BAT_FEET_POSE: PartPose = PartPose {
    offset: [0.0, 5.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

// Vanilla 26.1 `BatAnimation.BAT_FLYING` (length 0.5s, looping). Each keyframe list is sorted by
// timestamp; the values are baked through `degreeVec`/`posVec`.
const BAT_FLYING_HEAD_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), Linear),
    keyframe(0.125, degree_vec(20.0, 0.0, 0.0), Linear),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), Linear),
];
const BAT_FLYING_HEAD_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), Linear),
    keyframe(0.125, pos_vec(0.0, 2.0, 0.0), Linear),
    keyframe(0.25, pos_vec(0.0, 1.0, 0.0), Linear),
    keyframe(0.375, pos_vec(0.0, 0.0, 0.0), Linear),
    keyframe(0.4583, pos_vec(0.0, -1.0, 0.0), Linear),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), Linear),
];
const BAT_FLYING_BODY_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(40.0, 0.0, 0.0), Linear),
    keyframe(0.25, degree_vec(52.5, 0.0, 0.0), Linear),
    keyframe(0.5, degree_vec(40.0, 0.0, 0.0), Linear),
];
const BAT_FLYING_BODY_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), Linear),
    keyframe(0.125, pos_vec(0.0, 2.0, 0.0), Linear),
    keyframe(0.25, pos_vec(0.0, 1.0, 0.0), Linear),
    keyframe(0.375, pos_vec(0.0, 0.0, 0.0), Linear),
    keyframe(0.4583, pos_vec(0.0, -1.0, 0.0), Linear),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), Linear),
];
const BAT_FLYING_FEET_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(10.0, 0.0, 0.0), Linear),
    keyframe(0.125, degree_vec(-21.25, 0.0, 0.0), Linear),
    keyframe(0.25, degree_vec(-12.5, 0.0, 0.0), Linear),
    keyframe(0.5, degree_vec(10.0, 0.0, 0.0), Linear),
];
const BAT_FLYING_RIGHT_WING_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 85.0, 0.0), Linear),
    keyframe(0.125, degree_vec(0.0, -55.0, 0.0), Linear),
    keyframe(0.25, degree_vec(0.0, 50.0, 0.0), Linear),
    keyframe(0.375, degree_vec(0.0, 70.0, 0.0), Linear),
    keyframe(0.5, degree_vec(0.0, 85.0, 0.0), Linear),
];
const BAT_FLYING_RIGHT_WING_TIP_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, 10.5, 0.0), Linear),
    keyframe(0.0417, degree_vec(0.0, 65.5, 0.0), Linear),
    keyframe(0.2083, degree_vec(0.0, -135.0, 0.0), Linear),
    keyframe(0.5, degree_vec(0.0, 10.5, 0.0), Linear),
];
const BAT_FLYING_LEFT_WING_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, -85.0, 0.0), Linear),
    keyframe(0.125, degree_vec(0.0, 55.0, 0.0), Linear),
    keyframe(0.25, degree_vec(0.0, -50.0, 0.0), Linear),
    keyframe(0.375, degree_vec(0.0, -70.0, 0.0), Linear),
    keyframe(0.5, degree_vec(0.0, -85.0, 0.0), Linear),
];
const BAT_FLYING_LEFT_WING_TIP_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(0.0, -10.5, 0.0), Linear),
    keyframe(0.0417, degree_vec(0.0, -65.5, 0.0), Linear),
    keyframe(0.2083, degree_vec(0.0, 135.0, 0.0), Linear),
    keyframe(0.5, degree_vec(0.0, -10.5, 0.0), Linear),
];

const BAT_FLYING_BONES: [BoneAnimation; 7] = [
    BoneAnimation {
        bone: "head",
        channels: &[
            AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &BAT_FLYING_HEAD_ROT,
            },
            AnimationChannel {
                target: AnimationTarget::Position,
                keyframes: &BAT_FLYING_HEAD_POS,
            },
        ],
    },
    BoneAnimation {
        bone: "body",
        channels: &[
            AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &BAT_FLYING_BODY_ROT,
            },
            AnimationChannel {
                target: AnimationTarget::Position,
                keyframes: &BAT_FLYING_BODY_POS,
            },
        ],
    },
    BoneAnimation {
        bone: "feet",
        channels: &[AnimationChannel {
            target: AnimationTarget::Rotation,
            keyframes: &BAT_FLYING_FEET_ROT,
        }],
    },
    BoneAnimation {
        bone: "right_wing",
        channels: &[AnimationChannel {
            target: AnimationTarget::Rotation,
            keyframes: &BAT_FLYING_RIGHT_WING_ROT,
        }],
    },
    BoneAnimation {
        bone: "right_wing_tip",
        channels: &[AnimationChannel {
            target: AnimationTarget::Rotation,
            keyframes: &BAT_FLYING_RIGHT_WING_TIP_ROT,
        }],
    },
    BoneAnimation {
        bone: "left_wing",
        channels: &[AnimationChannel {
            target: AnimationTarget::Rotation,
            keyframes: &BAT_FLYING_LEFT_WING_ROT,
        }],
    },
    BoneAnimation {
        bone: "left_wing_tip",
        channels: &[AnimationChannel {
            target: AnimationTarget::Rotation,
            keyframes: &BAT_FLYING_LEFT_WING_TIP_ROT,
        }],
    },
];

pub(in crate::entity_models) const BAT_FLYING: AnimationDefinition = AnimationDefinition {
    length_seconds: 0.5,
    looping: true,
    bones: &BAT_FLYING_BONES,
};

// Vanilla 26.1 `BatAnimation.BAT_RESTING` (length 0.5s, looping). Every channel has a single
// keyframe at t = 0, so the animation is a static hanging pose: the head and body flip 180°
// about X (and shift `+0.5` y) so the bat hangs upside down, and the wings fold inward.
const BAT_RESTING_HEAD_ROT: [Keyframe; 1] = [keyframe(0.0, degree_vec(180.0, 0.0, 0.0), Linear)];
const BAT_RESTING_HEAD_POS: [Keyframe; 1] = [keyframe(0.0, pos_vec(0.0, 0.5, 0.0), Linear)];
const BAT_RESTING_BODY_ROT: [Keyframe; 1] = [keyframe(0.0, degree_vec(180.0, 0.0, 0.0), Linear)];
const BAT_RESTING_BODY_POS: [Keyframe; 1] = [keyframe(0.0, pos_vec(0.0, 0.5, 0.0), Linear)];
const BAT_RESTING_FEET_ROT: [Keyframe; 1] = [keyframe(0.0, degree_vec(0.0, 0.0, 0.0), Linear)];
const BAT_RESTING_RIGHT_WING_ROT: [Keyframe; 1] =
    [keyframe(0.0, degree_vec(0.0, -10.0, 0.0), Linear)];
const BAT_RESTING_RIGHT_WING_POS: [Keyframe; 1] = [keyframe(0.0, pos_vec(0.0, 0.0, 1.0), Linear)];
const BAT_RESTING_RIGHT_WING_TIP_ROT: [Keyframe; 1] =
    [keyframe(0.0, degree_vec(0.0, -120.0, 0.0), Linear)];
const BAT_RESTING_LEFT_WING_ROT: [Keyframe; 1] =
    [keyframe(0.0, degree_vec(0.0, 10.0, 0.0), Linear)];
const BAT_RESTING_LEFT_WING_POS: [Keyframe; 1] = [keyframe(0.0, pos_vec(0.0, 0.0, 1.0), Linear)];
const BAT_RESTING_LEFT_WING_TIP_ROT: [Keyframe; 1] =
    [keyframe(0.0, degree_vec(0.0, 120.0, 0.0), Linear)];

const BAT_RESTING_BONES: [BoneAnimation; 7] = [
    BoneAnimation {
        bone: "head",
        channels: &[
            AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &BAT_RESTING_HEAD_ROT,
            },
            AnimationChannel {
                target: AnimationTarget::Position,
                keyframes: &BAT_RESTING_HEAD_POS,
            },
        ],
    },
    BoneAnimation {
        bone: "body",
        channels: &[
            AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &BAT_RESTING_BODY_ROT,
            },
            AnimationChannel {
                target: AnimationTarget::Position,
                keyframes: &BAT_RESTING_BODY_POS,
            },
        ],
    },
    BoneAnimation {
        bone: "feet",
        channels: &[AnimationChannel {
            target: AnimationTarget::Rotation,
            keyframes: &BAT_RESTING_FEET_ROT,
        }],
    },
    BoneAnimation {
        bone: "right_wing",
        channels: &[
            AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &BAT_RESTING_RIGHT_WING_ROT,
            },
            AnimationChannel {
                target: AnimationTarget::Position,
                keyframes: &BAT_RESTING_RIGHT_WING_POS,
            },
        ],
    },
    BoneAnimation {
        bone: "right_wing_tip",
        channels: &[AnimationChannel {
            target: AnimationTarget::Rotation,
            keyframes: &BAT_RESTING_RIGHT_WING_TIP_ROT,
        }],
    },
    BoneAnimation {
        bone: "left_wing",
        channels: &[
            AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &BAT_RESTING_LEFT_WING_ROT,
            },
            AnimationChannel {
                target: AnimationTarget::Position,
                keyframes: &BAT_RESTING_LEFT_WING_POS,
            },
        ],
    },
    BoneAnimation {
        bone: "left_wing_tip",
        channels: &[AnimationChannel {
            target: AnimationTarget::Rotation,
            keyframes: &BAT_RESTING_LEFT_WING_TIP_ROT,
        }],
    },
];

pub(in crate::entity_models) const BAT_RESTING: AnimationDefinition = AnimationDefinition {
    length_seconds: 0.5,
    looping: true,
    bones: &BAT_RESTING_BONES,
};

/// Applies the vanilla `BatModel.setupAnim` to the unified tree: the looping `BatAnimation.BAT_FLYING`
/// wing flap / body bob (sampled from `ageInTicks`), or, while `isResting`, the `BAT_RESTING` hanging
/// pose with the head turned by the look yaw (`applyHeadRotation`, additive to the head's `yRot`). The
/// ears hold their bind poses. The exact animation start tick is deferred, imperceptible for a
/// continuous flap.
fn apply_bat_anim(root: &mut ModelPart, instance: &EntityModelInstance) {
    let resting = instance.render_state.bat_resting;
    let animation = if resting { &BAT_RESTING } else { &BAT_FLYING };
    let head_look_yaw = if resting {
        instance.render_state.head_yaw.to_radians()
    } else {
        0.0
    };
    let seconds = keyframe_elapsed_seconds(animation, instance.render_state.age_in_ticks * 0.05);
    let sample = |bone: &str| sample_bone_offsets(animation, bone, seconds, 1.0);

    let (head_pos, head_rot) = sample("head");
    let (body_pos, body_rot) = sample("body");
    let (_, feet_rot) = sample("feet");
    let (right_wing_pos, right_wing_rot) = sample("right_wing");
    let (_, right_tip_rot) = sample("right_wing_tip");
    let (left_wing_pos, left_wing_rot) = sample("left_wing");
    let (_, left_tip_rot) = sample("left_wing_tip");

    let head = root.child_mut("head");
    head.pose = keyframe_animated_pose(
        BAT_HEAD_POSE,
        head_pos,
        [head_rot[0], head_rot[1] + head_look_yaw, head_rot[2]],
    );

    let body = root.child_mut("body");
    body.pose = keyframe_animated_pose(BAT_BODY_POSE, body_pos, body_rot);
    body.child_mut("feet").pose = keyframe_animated_pose(BAT_FEET_POSE, [0.0; 3], feet_rot);

    let right_wing = body.child_mut("right_wing");
    right_wing.pose = keyframe_animated_pose(BAT_RIGHT_WING_POSE, right_wing_pos, right_wing_rot);
    right_wing.child_mut("right_wing_tip").pose =
        keyframe_animated_pose(BAT_RIGHT_WING_TIP_POSE, [0.0; 3], right_tip_rot);

    let left_wing = body.child_mut("left_wing");
    left_wing.pose = keyframe_animated_pose(BAT_LEFT_WING_POSE, left_wing_pos, left_wing_rot);
    left_wing.child_mut("left_wing_tip").pose =
        keyframe_animated_pose(BAT_LEFT_WING_TIP_POSE, [0.0; 3], left_tip_rot);
}

/// Mutable bat model, mirroring vanilla `BatModel`. The unified tree is built once with named children:
/// a synthetic root parenting `head` (→ `right_ear`, `left_ear`) and `body` (→ `feet`, `right_wing` →
/// `right_wing_tip`, `left_wing` → `left_wing_tip`), in the emit order (preserved for byte-identical
/// meshes). Each cube carries both the colored tint and the textured UV, so one tree drives both render
/// paths; `setup_anim` runs [`apply_bat_anim`]. The same posed tree drives the colored fallback and the
/// cutout textured layer.
pub(in crate::entity_models) struct BatModel {
    root: ModelPart,
}

impl BatModel {
    pub(in crate::entity_models) fn new() -> Self {
        let head = ModelPart::new(
            BAT_HEAD_POSE,
            BAT_HEAD.to_vec(),
            vec![
                (
                    "right_ear",
                    ModelPart::leaf(BAT_RIGHT_EAR_POSE, BAT_RIGHT_EAR.to_vec()),
                ),
                (
                    "left_ear",
                    ModelPart::leaf(BAT_LEFT_EAR_POSE, BAT_LEFT_EAR.to_vec()),
                ),
            ],
        );
        let right_wing = ModelPart::new(
            BAT_RIGHT_WING_POSE,
            BAT_RIGHT_WING.to_vec(),
            vec![(
                "right_wing_tip",
                ModelPart::leaf(BAT_RIGHT_WING_TIP_POSE, BAT_RIGHT_WING_TIP.to_vec()),
            )],
        );
        let left_wing = ModelPart::new(
            BAT_LEFT_WING_POSE,
            BAT_LEFT_WING.to_vec(),
            vec![(
                "left_wing_tip",
                ModelPart::leaf(BAT_LEFT_WING_TIP_POSE, BAT_LEFT_WING_TIP.to_vec()),
            )],
        );
        let body = ModelPart::new(
            BAT_BODY_POSE,
            BAT_BODY.to_vec(),
            vec![
                ("feet", ModelPart::leaf(BAT_FEET_POSE, BAT_FEET.to_vec())),
                ("right_wing", right_wing),
                ("left_wing", left_wing),
            ],
        );
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![("head", head), ("body", body)],
            ),
        }
    }
}

impl EntityModel for BatModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_bat_anim(&mut self.root, instance);
    }
}
