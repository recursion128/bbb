use super::{
    apply_head_look, apply_humanoid_walk, ModelCubeDesc, ModelPartDesc, PartPose,
    TexturedModelCubeDesc, TexturedModelPartDesc, PART_POSE_ZERO,
};
use super::{parched_head_part_index, skeleton_head_part_index};
use crate::entity_models::catalog::SkeletonModelFamily;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_SKELETON: &str = "minecraft:skeleton#main";
pub(in crate::entity_models) const MODEL_LAYER_STRAY: &str = "minecraft:stray#main";
pub(in crate::entity_models) const MODEL_LAYER_PARCHED: &str = "minecraft:parched#main";
pub(in crate::entity_models) const MODEL_LAYER_WITHER_SKELETON: &str =
    "minecraft:wither_skeleton#main";
pub(in crate::entity_models) const MODEL_LAYER_BOGGED: &str = "minecraft:bogged#main";

pub(in crate::entity_models) const SKELETON_BONE: [f32; 4] = [0.82, 0.82, 0.72, 1.0];
pub(in crate::entity_models) const WITHER_SKELETON_DARK: [f32; 4] = [0.14, 0.14, 0.14, 1.0];
pub(in crate::entity_models) const PARCHED_BONE: [f32; 4] = [0.70, 0.62, 0.48, 1.0];
pub(in crate::entity_models) const BOGGED_BONE: [f32; 4] = [0.53, 0.61, 0.42, 1.0];
pub(in crate::entity_models) const BOGGED_RED_MUSHROOM_COLOR: [f32; 4] = [0.78, 0.15, 0.12, 1.0];
pub(in crate::entity_models) const BOGGED_BROWN_MUSHROOM_COLOR: [f32; 4] = [0.48, 0.31, 0.18, 1.0];

pub(in crate::entity_models) const SKELETON_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -4.0],
    size: [8.0, 8.0, 8.0],
    color: SKELETON_BONE,
}];

pub(in crate::entity_models) const SKELETON_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -8.5, -4.5],
    size: [9.0, 9.0, 9.0],
    color: SKELETON_BONE,
}];

pub(in crate::entity_models) const SKELETON_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &SKELETON_HAT,
    children: &[],
}];

pub(in crate::entity_models) const SKELETON_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: SKELETON_BONE,
}];

pub(in crate::entity_models) const SKELETON_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -1.0],
    size: [2.0, 12.0, 2.0],
    color: SKELETON_BONE,
}];

pub(in crate::entity_models) const SKELETON_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 12.0, 2.0],
    color: SKELETON_BONE,
}];

// Vanilla 26.1 SkeletonModel.createBodyLayer().
pub(in crate::entity_models) const SKELETON_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SKELETON_HEAD,
        children: &SKELETON_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SKELETON_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SKELETON_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SKELETON_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SKELETON_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SKELETON_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const SKELETON_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -8.0, -4.0],
        size: [8.0, 8.0, 8.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SKELETON_TEXTURED_HAT: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.5, -8.5, -4.5],
        size: [9.0, 9.0, 9.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [32.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SKELETON_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SKELETON_TEXTURED_HAT,
        children: &[],
    }];

pub(in crate::entity_models) const SKELETON_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 0.0, -2.0],
        size: [8.0, 12.0, 4.0],
        uv_size: [8.0, 12.0, 4.0],
        tex: [16.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SKELETON_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -2.0, -1.0],
        size: [2.0, 12.0, 2.0],
        uv_size: [2.0, 12.0, 2.0],
        tex: [40.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SKELETON_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -2.0, -1.0],
        size: [2.0, 12.0, 2.0],
        uv_size: [2.0, 12.0, 2.0],
        tex: [40.0, 16.0],
        mirror: true,
    }];

pub(in crate::entity_models) const SKELETON_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 12.0, 2.0],
        uv_size: [2.0, 12.0, 2.0],
        tex: [0.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SKELETON_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 12.0, 2.0],
        uv_size: [2.0, 12.0, 2.0],
        tex: [0.0, 16.0],
        mirror: true,
    }];

pub(in crate::entity_models) const SKELETON_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: SKELETON_PARTS[0].pose,
        cubes: &SKELETON_TEXTURED_HEAD,
        children: &SKELETON_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: SKELETON_PARTS[1].pose,
        cubes: &SKELETON_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SKELETON_PARTS[2].pose,
        cubes: &SKELETON_TEXTURED_RIGHT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SKELETON_PARTS[3].pose,
        cubes: &SKELETON_TEXTURED_LEFT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SKELETON_PARTS[4].pose,
        cubes: &SKELETON_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SKELETON_PARTS[5].pose,
        cubes: &SKELETON_TEXTURED_LEFT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BOGGED_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -4.0],
    size: [8.0, 8.0, 8.0],
    color: BOGGED_BONE,
}];

pub(in crate::entity_models) const BOGGED_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -8.5, -4.5],
    size: [9.0, 9.0, 9.0],
    color: BOGGED_BONE,
}];

pub(in crate::entity_models) const BOGGED_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: BOGGED_BONE,
}];

pub(in crate::entity_models) const BOGGED_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -1.0],
    size: [2.0, 12.0, 2.0],
    color: BOGGED_BONE,
}];

pub(in crate::entity_models) const BOGGED_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 12.0, 2.0],
    color: BOGGED_BONE,
}];

pub(in crate::entity_models) const BOGGED_RED_MUSHROOM_PLANE: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-3.0, -3.0, 0.0],
        size: [6.0, 4.0, 0.0],
        color: BOGGED_RED_MUSHROOM_COLOR,
    }];

pub(in crate::entity_models) const BOGGED_BROWN_MUSHROOM_PLANE: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-3.0, -3.0, 0.0],
        size: [6.0, 4.0, 0.0],
        color: BOGGED_BROWN_MUSHROOM_COLOR,
    }];

pub(in crate::entity_models) const BOGGED_BROWN_TOP_MUSHROOM_PLANE: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-3.0, -4.0, 0.0],
        size: [6.0, 4.0, 0.0],
        color: BOGGED_BROWN_MUSHROOM_COLOR,
    }];

pub(in crate::entity_models) const BOGGED_HAT_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &BOGGED_HAT,
    children: &[],
}];

pub(in crate::entity_models) const BOGGED_MUSHROOM_CHILDREN: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, -8.0, 3.0],
            rotation: [0.0, std::f32::consts::FRAC_PI_4, 0.0],
        },
        cubes: &BOGGED_RED_MUSHROOM_PLANE,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, -8.0, 3.0],
            rotation: [0.0, std::f32::consts::FRAC_PI_4 * 3.0, 0.0],
        },
        cubes: &BOGGED_RED_MUSHROOM_PLANE,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, -8.0, -3.0],
            rotation: [0.0, std::f32::consts::FRAC_PI_4, 0.0],
        },
        cubes: &BOGGED_BROWN_MUSHROOM_PLANE,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, -8.0, -3.0],
            rotation: [0.0, std::f32::consts::FRAC_PI_4 * 3.0, 0.0],
        },
        cubes: &BOGGED_BROWN_MUSHROOM_PLANE,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -1.0, 4.0],
            rotation: [
                -std::f32::consts::FRAC_PI_2,
                0.0,
                std::f32::consts::FRAC_PI_4,
            ],
        },
        cubes: &BOGGED_BROWN_TOP_MUSHROOM_PLANE,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -1.0, 4.0],
            rotation: [
                -std::f32::consts::FRAC_PI_2,
                0.0,
                std::f32::consts::FRAC_PI_4 * 3.0,
            ],
        },
        cubes: &BOGGED_BROWN_TOP_MUSHROOM_PLANE,
        children: &[],
    },
];

pub(in crate::entity_models) const BOGGED_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &BOGGED_HAT,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &[],
        children: &BOGGED_MUSHROOM_CHILDREN,
    },
];

pub(in crate::entity_models) const BOGGED_TEXTURED_RED_MUSHROOM_PLANE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -3.0, 0.0],
        size: [6.0, 4.0, 0.0],
        uv_size: [6.0, 4.0, 0.0],
        tex: [50.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOGGED_TEXTURED_BROWN_MUSHROOM_PLANE: [TexturedModelCubeDesc;
    1] = [TexturedModelCubeDesc {
    min: [-3.0, -3.0, 0.0],
    size: [6.0, 4.0, 0.0],
    uv_size: [6.0, 4.0, 0.0],
    tex: [50.0, 22.0],
    mirror: false,
}];

pub(in crate::entity_models) const BOGGED_TEXTURED_BROWN_TOP_MUSHROOM_PLANE:
    [TexturedModelCubeDesc; 1] = [TexturedModelCubeDesc {
    min: [-3.0, -4.0, 0.0],
    size: [6.0, 4.0, 0.0],
    uv_size: [6.0, 4.0, 0.0],
    tex: [50.0, 28.0],
    mirror: false,
}];

pub(in crate::entity_models) const BOGGED_TEXTURED_MUSHROOM_CHILDREN: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: BOGGED_MUSHROOM_CHILDREN[0].pose,
        cubes: &BOGGED_TEXTURED_RED_MUSHROOM_PLANE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOGGED_MUSHROOM_CHILDREN[1].pose,
        cubes: &BOGGED_TEXTURED_RED_MUSHROOM_PLANE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOGGED_MUSHROOM_CHILDREN[2].pose,
        cubes: &BOGGED_TEXTURED_BROWN_MUSHROOM_PLANE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOGGED_MUSHROOM_CHILDREN[3].pose,
        cubes: &BOGGED_TEXTURED_BROWN_MUSHROOM_PLANE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOGGED_MUSHROOM_CHILDREN[4].pose,
        cubes: &BOGGED_TEXTURED_BROWN_TOP_MUSHROOM_PLANE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOGGED_MUSHROOM_CHILDREN[5].pose,
        cubes: &BOGGED_TEXTURED_BROWN_TOP_MUSHROOM_PLANE,
        children: &[],
    },
];

pub(in crate::entity_models) const BOGGED_TEXTURED_HAT_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SKELETON_TEXTURED_HAT,
        children: &[],
    }];

pub(in crate::entity_models) const BOGGED_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SKELETON_TEXTURED_HAT,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &[],
        children: &BOGGED_TEXTURED_MUSHROOM_CHILDREN,
    },
];

pub(in crate::entity_models) const BOGGED_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: BOGGED_PARTS[0].pose,
        cubes: &SKELETON_TEXTURED_HEAD,
        children: &BOGGED_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: BOGGED_PARTS[1].pose,
        cubes: &SKELETON_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOGGED_PARTS[2].pose,
        cubes: &SKELETON_TEXTURED_RIGHT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOGGED_PARTS[3].pose,
        cubes: &SKELETON_TEXTURED_LEFT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOGGED_PARTS[4].pose,
        cubes: &SKELETON_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOGGED_PARTS[5].pose,
        cubes: &SKELETON_TEXTURED_LEFT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BOGGED_SHEARED_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: BOGGED_SHEARED_PARTS[0].pose,
        cubes: &SKELETON_TEXTURED_HEAD,
        children: &BOGGED_TEXTURED_HAT_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: BOGGED_SHEARED_PARTS[1].pose,
        cubes: &SKELETON_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOGGED_SHEARED_PARTS[2].pose,
        cubes: &SKELETON_TEXTURED_RIGHT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOGGED_SHEARED_PARTS[3].pose,
        cubes: &SKELETON_TEXTURED_LEFT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOGGED_SHEARED_PARTS[4].pose,
        cubes: &SKELETON_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOGGED_SHEARED_PARTS[5].pose,
        cubes: &SKELETON_TEXTURED_LEFT_LEG,
        children: &[],
    },
];

// Vanilla 26.1 BoggedModel.createBodyLayer(): HumanoidModel base,
// SkeletonModel.createDefaultSkeletonMesh(root), then head/mushrooms children.
pub(in crate::entity_models) const BOGGED_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &BOGGED_HEAD,
        children: &BOGGED_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &BOGGED_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BOGGED_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BOGGED_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BOGGED_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BOGGED_LEG,
        children: &[],
    },
];

// Vanilla 26.1 BoggedModel.createBodyLayer(), with mushrooms hidden when
// BoggedRenderState.isSheared is true.
pub(in crate::entity_models) const BOGGED_SHEARED_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &BOGGED_HEAD,
        children: &BOGGED_HAT_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &BOGGED_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BOGGED_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BOGGED_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BOGGED_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BOGGED_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const PARCHED_BODY: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-4.0, 0.0, -2.0],
        size: [8.0, 12.0, 4.0],
        color: PARCHED_BONE,
    },
    ModelCubeDesc {
        min: [-4.0, 10.0, -2.0],
        size: [8.0, 1.0, 4.0],
        color: PARCHED_BONE,
    },
    ModelCubeDesc {
        min: [-4.025, -0.025, -2.025],
        size: [8.05, 12.05, 4.05],
        color: PARCHED_BONE,
    },
];

pub(in crate::entity_models) const PARCHED_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, -8.0, -4.0],
        size: [8.0, 8.0, 8.0],
        color: PARCHED_BONE,
    },
    ModelCubeDesc {
        min: [-4.2, -8.2, -4.2],
        size: [8.4, 8.4, 8.4],
        color: PARCHED_BONE,
    },
];

pub(in crate::entity_models) const PARCHED_EMPTY_HAT: [ModelCubeDesc; 0] = [];

pub(in crate::entity_models) const PARCHED_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &PARCHED_EMPTY_HAT,
    children: &[],
}];

pub(in crate::entity_models) const PARCHED_RIGHT_ARM: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.0, -2.0, -1.0],
        size: [2.0, 12.0, 2.0],
        color: PARCHED_BONE,
    },
    ModelCubeDesc {
        min: [-1.55, -2.025, -1.5],
        size: [3.0, 12.0, 3.0],
        color: PARCHED_BONE,
    },
];

pub(in crate::entity_models) const PARCHED_LEFT_ARM: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.0, -2.0, -1.0],
        size: [2.0, 12.0, 2.0],
        color: PARCHED_BONE,
    },
    ModelCubeDesc {
        min: [-1.45, -2.025, -1.5],
        size: [3.0, 12.0, 3.0],
        color: PARCHED_BONE,
    },
];

pub(in crate::entity_models) const PARCHED_LEG: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 12.0, 2.0],
        color: PARCHED_BONE,
    },
    ModelCubeDesc {
        min: [-1.5, 0.0, -1.5],
        size: [3.0, 12.0, 3.0],
        color: PARCHED_BONE,
    },
];

// Vanilla 26.1 SkeletonModel.createSingleModelDualBodyLayer().
pub(in crate::entity_models) const PARCHED_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PARCHED_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PARCHED_HEAD,
        children: &PARCHED_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.5, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PARCHED_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.5, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PARCHED_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PARCHED_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PARCHED_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const PARCHED_TEXTURED_BODY: [TexturedModelCubeDesc; 3] = [
    TexturedModelCubeDesc {
        min: [-4.0, 0.0, -2.0],
        size: [8.0, 12.0, 4.0],
        uv_size: [8.0, 12.0, 4.0],
        tex: [16.0, 16.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-4.0, 10.0, -2.0],
        size: [8.0, 1.0, 4.0],
        uv_size: [8.0, 1.0, 4.0],
        tex: [28.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-4.025, -0.025, -2.025],
        size: [8.05, 12.05, 4.05],
        uv_size: [8.0, 12.0, 4.0],
        tex: [16.0, 48.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const PARCHED_TEXTURED_HEAD: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-4.0, -8.0, -4.0],
        size: [8.0, 8.0, 8.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-4.2, -8.2, -4.2],
        size: [8.4, 8.4, 8.4],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 32.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const PARCHED_TEXTURED_EMPTY_HAT: [TexturedModelCubeDesc; 0] = [];

pub(in crate::entity_models) const PARCHED_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PARCHED_TEXTURED_EMPTY_HAT,
        children: &[],
    }];

pub(in crate::entity_models) const PARCHED_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-1.0, -2.0, -1.0],
        size: [2.0, 12.0, 2.0],
        uv_size: [2.0, 12.0, 2.0],
        tex: [40.0, 16.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-1.55, -2.025, -1.5],
        size: [3.0, 12.0, 3.0],
        uv_size: [3.0, 12.0, 3.0],
        tex: [42.0, 33.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const PARCHED_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-1.0, -2.0, -1.0],
        size: [2.0, 12.0, 2.0],
        uv_size: [2.0, 12.0, 2.0],
        tex: [56.0, 16.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-1.45, -2.025, -1.5],
        size: [3.0, 12.0, 3.0],
        uv_size: [3.0, 12.0, 3.0],
        tex: [40.0, 48.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const PARCHED_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 12.0, 2.0],
        uv_size: [2.0, 12.0, 2.0],
        tex: [0.0, 16.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-1.5, 0.0, -1.5],
        size: [3.0, 12.0, 3.0],
        uv_size: [3.0, 12.0, 3.0],
        tex: [0.0, 49.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const PARCHED_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 12.0, 2.0],
        uv_size: [2.0, 12.0, 2.0],
        tex: [0.0, 16.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-1.5, 0.0, -1.5],
        size: [3.0, 12.0, 3.0],
        uv_size: [3.0, 12.0, 3.0],
        tex: [4.0, 49.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const PARCHED_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: PARCHED_PARTS[0].pose,
        cubes: &PARCHED_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PARCHED_PARTS[1].pose,
        cubes: &PARCHED_TEXTURED_HEAD,
        children: &PARCHED_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PARCHED_PARTS[2].pose,
        cubes: &PARCHED_TEXTURED_RIGHT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PARCHED_PARTS[3].pose,
        cubes: &PARCHED_TEXTURED_LEFT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PARCHED_PARTS[4].pose,
        cubes: &PARCHED_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PARCHED_PARTS[5].pose,
        cubes: &PARCHED_TEXTURED_LEFT_LEG,
        children: &[],
    },
];

/// Selects the base colored + textured trees for a skeleton family. Every family lists the head, body,
/// right/left arm, right/left leg at the `HumanoidModel` indices (head first, except parched — see
/// [`skeleton_family_head_index`]). The wither-skeleton reuses the plain skeleton mesh (its dark tint
/// and root transform are applied at the call site); the stray / bogged clothing is a separate
/// textured-only overlay ([`SkeletonClothingModel`]).
fn skeleton_part_trees(
    family: Option<SkeletonModelFamily>,
) -> (&'static [ModelPartDesc], &'static [TexturedModelPartDesc]) {
    match family {
        None | Some(SkeletonModelFamily::Stray) | Some(SkeletonModelFamily::WitherSkeleton) => {
            (&SKELETON_PARTS, &SKELETON_TEXTURED_PARTS)
        }
        Some(SkeletonModelFamily::Parched) => (&PARCHED_PARTS, &PARCHED_TEXTURED_PARTS),
        Some(SkeletonModelFamily::Bogged { sheared: false }) => {
            (&BOGGED_PARTS, &BOGGED_TEXTURED_PARTS)
        }
        Some(SkeletonModelFamily::Bogged { sheared: true }) => {
            (&BOGGED_SHEARED_PARTS, &BOGGED_SHEARED_TEXTURED_PARTS)
        }
    }
}

/// The head-part index for a skeleton family: the parched body layer lists the body first (head at 1),
/// every other family lists the head first (0).
pub(in crate::entity_models) fn skeleton_family_head_index(
    family: Option<SkeletonModelFamily>,
) -> usize {
    match family {
        Some(SkeletonModelFamily::Parched) => parched_head_part_index(),
        _ => skeleton_head_part_index(),
    }
}

/// Applies the shared vanilla `HumanoidModel.setupAnim` head look + arm/leg walk swing to a
/// skeleton-family tree (the base body OR a clothing overlay), so one animator drives both render paths
/// and both textured passes. `SkeletonModel extends HumanoidModel` and only overrides the arms in its
/// deferred melee/bow branches, so the default state is the inherited humanoid walk (legs `[4, 5]`,
/// arms `[2, 3]`).
fn apply_skeleton_anim(root: &mut ModelPart, head_index: usize, instance: &EntityModelInstance) {
    let render_state = &instance.render_state;
    apply_head_look(
        root.child_at_mut(head_index),
        render_state.head_yaw,
        render_state.head_pitch,
    );
    apply_humanoid_walk(
        root,
        render_state.walk_animation_pos,
        render_state.walk_animation_speed,
        render_state.age_in_ticks,
    );
}

/// Mutable skeleton model, mirroring vanilla `SkeletonModel` (the base `HumanoidModel`) and its
/// stray / parched / bogged / wither-skeleton variants. The unified tree is zipped from the baked
/// colored and textured trees selected by family ([`skeleton_part_trees`]): the head, body, right/left
/// arm, right/left leg. `setup_anim` runs the shared [`apply_skeleton_anim`]. The bow-aiming arm pose
/// is deferred; the wither dark tint / root transform and the stray / bogged clothing overlay
/// ([`SkeletonClothingModel`]) are applied at the call site.
pub(in crate::entity_models) struct SkeletonModel {
    root: ModelPart,
    head_index: usize,
}

impl SkeletonModel {
    pub(in crate::entity_models) fn new(family: Option<SkeletonModelFamily>) -> Self {
        let (colored, textured) = skeleton_part_trees(family);
        Self {
            root: ModelPart::root_from_descs(colored, textured),
            head_index: skeleton_family_head_index(family),
        }
    }
}

impl EntityModel for SkeletonModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_skeleton_anim(&mut self.root, self.head_index, instance);
    }
}

/// Mutable textured-only skeleton clothing overlay (the stray frost layer / bogged mushroom layer): an
/// inflated overlay built from its `&'static` textured parts and posed by the SAME
/// [`apply_skeleton_anim`] as the base body, so the overlay tracks the limbs. It has no colored variant
/// (the colored debug path omits the clothing), so only [`ModelPart::render_textured`] is ever called.
pub(in crate::entity_models) struct SkeletonClothingModel {
    root: ModelPart,
    head_index: usize,
}

impl SkeletonClothingModel {
    pub(in crate::entity_models) fn new(
        parts: &'static [TexturedModelPartDesc],
        head_index: usize,
    ) -> Self {
        Self {
            root: ModelPart::root_from_textured_descs(parts),
            head_index,
        }
    }
}

impl EntityModel for SkeletonClothingModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_skeleton_anim(&mut self.root, self.head_index, instance);
    }
}
