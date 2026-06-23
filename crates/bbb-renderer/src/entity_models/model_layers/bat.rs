use super::{
    degree_vec, keyframe, pos_vec, AnimationChannel, AnimationDefinition, AnimationTarget,
    BoneAnimation, Keyframe, KeyframeInterpolation, ModelCubeDesc, ModelPartDesc, PartPose,
    TexturedModelCubeDesc, TexturedModelPartDesc, BAT_BROWN,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::{
    keyframe_animated_pose, keyframe_elapsed_seconds, sample_bone_offsets,
};
use crate::entity_models::model::{EntityModel, ModelPart};

use KeyframeInterpolation::Linear;

// Vanilla 26.1 `BatModel.createBodyLayer` (atlas 32×32). The body and head hang under the root;
// the wings (each with a tip) and the feet are children of the body, and the two ears are
// children of the head. The keyframe `BatAnimation.BAT_FLYING` (below) adds per-frame position
// and rotation offsets to these bind poses.
pub(in crate::entity_models) const BAT_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, -1.0],
    size: [3.0, 5.0, 2.0],
    color: BAT_BROWN,
}];

pub(in crate::entity_models) const BAT_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -3.0, -1.0],
    size: [4.0, 3.0, 2.0],
    color: BAT_BROWN,
}];

// Ears and wings are zero-thickness planes.
pub(in crate::entity_models) const BAT_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, -4.0, 0.0],
    size: [3.0, 5.0, 0.0],
    color: BAT_BROWN,
}];

pub(in crate::entity_models) const BAT_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.1, -3.0, 0.0],
    size: [3.0, 5.0, 0.0],
    color: BAT_BROWN,
}];

pub(in crate::entity_models) const BAT_RIGHT_WING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -2.0, 0.0],
    size: [2.0, 7.0, 0.0],
    color: BAT_BROWN,
}];

pub(in crate::entity_models) const BAT_RIGHT_WING_TIP: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-6.0, -2.0, 0.0],
    size: [6.0, 8.0, 0.0],
    color: BAT_BROWN,
}];

pub(in crate::entity_models) const BAT_LEFT_WING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -2.0, 0.0],
    size: [2.0, 7.0, 0.0],
    color: BAT_BROWN,
}];

pub(in crate::entity_models) const BAT_LEFT_WING_TIP: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -2.0, 0.0],
    size: [6.0, 8.0, 0.0],
    color: BAT_BROWN,
}];

pub(in crate::entity_models) const BAT_FEET: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, 0.0],
    size: [3.0, 2.0, 0.0],
    color: BAT_BROWN,
}];

// The same geometry with the vanilla `BatModel.createBodyLayer` texOffs UV coordinates (atlas
// 32×32). No `CubeDeformation`, so each `uv_size` matches its box `size`.
pub(in crate::entity_models) const BAT_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, 0.0, -1.0],
        size: [3.0, 5.0, 2.0],
        uv_size: [3.0, 5.0, 2.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BAT_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -3.0, -1.0],
        size: [4.0, 3.0, 2.0],
        uv_size: [4.0, 3.0, 2.0],
        tex: [0.0, 7.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BAT_TEXTURED_RIGHT_EAR: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.5, -4.0, 0.0],
        size: [3.0, 5.0, 0.0],
        uv_size: [3.0, 5.0, 0.0],
        tex: [1.0, 15.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BAT_TEXTURED_LEFT_EAR: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-0.1, -3.0, 0.0],
        size: [3.0, 5.0, 0.0],
        uv_size: [3.0, 5.0, 0.0],
        tex: [8.0, 15.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BAT_TEXTURED_RIGHT_WING: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -2.0, 0.0],
        size: [2.0, 7.0, 0.0],
        uv_size: [2.0, 7.0, 0.0],
        tex: [12.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BAT_TEXTURED_RIGHT_WING_TIP: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-6.0, -2.0, 0.0],
        size: [6.0, 8.0, 0.0],
        uv_size: [6.0, 8.0, 0.0],
        tex: [16.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BAT_TEXTURED_LEFT_WING: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, -2.0, 0.0],
        size: [2.0, 7.0, 0.0],
        uv_size: [2.0, 7.0, 0.0],
        tex: [12.0, 7.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BAT_TEXTURED_LEFT_WING_TIP: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, -2.0, 0.0],
        size: [6.0, 8.0, 0.0],
        uv_size: [6.0, 8.0, 0.0],
        tex: [16.0, 8.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BAT_TEXTURED_FEET: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, 0.0, 0.0],
        size: [3.0, 2.0, 0.0],
        uv_size: [3.0, 2.0, 0.0],
        tex: [16.0, 16.0],
        mirror: false,
    }];

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

// Colored bat tree: `head` (→ right/left ear) and `body` (→ feet, right wing → tip, left wing → tip)
// hang under the root, in the emit order (preserved for byte-identical meshes). Mirrors vanilla
// `BatModel.createBodyLayer`. Zipped with the textured tree by `BatModel::new`; the keyframe sample is
// applied in `setup_anim`.
const BAT_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: BAT_RIGHT_EAR_POSE,
        cubes: &BAT_RIGHT_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: BAT_LEFT_EAR_POSE,
        cubes: &BAT_LEFT_EAR,
        children: &[],
    },
];
const BAT_RIGHT_WING_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: BAT_RIGHT_WING_TIP_POSE,
    cubes: &BAT_RIGHT_WING_TIP,
    children: &[],
}];
const BAT_LEFT_WING_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: BAT_LEFT_WING_TIP_POSE,
    cubes: &BAT_LEFT_WING_TIP,
    children: &[],
}];
const BAT_BODY_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: BAT_FEET_POSE,
        cubes: &BAT_FEET,
        children: &[],
    },
    ModelPartDesc {
        pose: BAT_RIGHT_WING_POSE,
        cubes: &BAT_RIGHT_WING,
        children: &BAT_RIGHT_WING_CHILDREN,
    },
    ModelPartDesc {
        pose: BAT_LEFT_WING_POSE,
        cubes: &BAT_LEFT_WING,
        children: &BAT_LEFT_WING_CHILDREN,
    },
];
pub(in crate::entity_models) const BAT_PARTS: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: BAT_HEAD_POSE,
        cubes: &BAT_HEAD,
        children: &BAT_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: BAT_BODY_POSE,
        cubes: &BAT_BODY,
        children: &BAT_BODY_CHILDREN,
    },
];

// Textured counterpart of `BAT_PARTS` (same hierarchy and bind poses, UV cubes).
const BAT_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: BAT_RIGHT_EAR_POSE,
        cubes: &BAT_TEXTURED_RIGHT_EAR,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BAT_LEFT_EAR_POSE,
        cubes: &BAT_TEXTURED_LEFT_EAR,
        children: &[],
    },
];
const BAT_TEXTURED_RIGHT_WING_CHILDREN: [TexturedModelPartDesc; 1] = [TexturedModelPartDesc {
    pose: BAT_RIGHT_WING_TIP_POSE,
    cubes: &BAT_TEXTURED_RIGHT_WING_TIP,
    children: &[],
}];
const BAT_TEXTURED_LEFT_WING_CHILDREN: [TexturedModelPartDesc; 1] = [TexturedModelPartDesc {
    pose: BAT_LEFT_WING_TIP_POSE,
    cubes: &BAT_TEXTURED_LEFT_WING_TIP,
    children: &[],
}];
const BAT_TEXTURED_BODY_CHILDREN: [TexturedModelPartDesc; 3] = [
    TexturedModelPartDesc {
        pose: BAT_FEET_POSE,
        cubes: &BAT_TEXTURED_FEET,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BAT_RIGHT_WING_POSE,
        cubes: &BAT_TEXTURED_RIGHT_WING,
        children: &BAT_TEXTURED_RIGHT_WING_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: BAT_LEFT_WING_POSE,
        cubes: &BAT_TEXTURED_LEFT_WING,
        children: &BAT_TEXTURED_LEFT_WING_CHILDREN,
    },
];
pub(in crate::entity_models) const BAT_TEXTURED_PARTS: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: BAT_HEAD_POSE,
        cubes: &BAT_TEXTURED_HEAD,
        children: &BAT_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: BAT_BODY_POSE,
        cubes: &BAT_TEXTURED_BODY,
        children: &BAT_TEXTURED_BODY_CHILDREN,
    },
];

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

    let head = root.child_at_mut(0);
    head.pose = keyframe_animated_pose(
        BAT_HEAD_POSE,
        head_pos,
        [head_rot[0], head_rot[1] + head_look_yaw, head_rot[2]],
    );

    let body = root.child_at_mut(1);
    body.pose = keyframe_animated_pose(BAT_BODY_POSE, body_pos, body_rot);
    body.child_at_mut(0).pose = keyframe_animated_pose(BAT_FEET_POSE, [0.0; 3], feet_rot);

    let right_wing = body.child_at_mut(1);
    right_wing.pose = keyframe_animated_pose(BAT_RIGHT_WING_POSE, right_wing_pos, right_wing_rot);
    right_wing.child_at_mut(0).pose =
        keyframe_animated_pose(BAT_RIGHT_WING_TIP_POSE, [0.0; 3], right_tip_rot);

    let left_wing = body.child_at_mut(2);
    left_wing.pose = keyframe_animated_pose(BAT_LEFT_WING_POSE, left_wing_pos, left_wing_rot);
    left_wing.child_at_mut(0).pose =
        keyframe_animated_pose(BAT_LEFT_WING_TIP_POSE, [0.0; 3], left_tip_rot);
}

/// Mutable bat model, mirroring vanilla `BatModel`. The unified tree is zipped from the head → ears and
/// body → (feet, wings → tips) hierarchy ([`BAT_PARTS`] / [`BAT_TEXTURED_PARTS`]); `setup_anim` runs
/// [`apply_bat_anim`]. The same posed tree drives the colored fallback and the cutout textured layer.
pub(in crate::entity_models) struct BatModel {
    root: ModelPart,
}

impl BatModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::root_from_descs(&BAT_PARTS, &BAT_TEXTURED_PARTS),
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
