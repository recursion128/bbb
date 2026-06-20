use super::{ModelCubeDesc, ModelPartDesc, PartPose, PART_POSE_ZERO};

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
