use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, ILLAGER_HAT_COLOR, ILLAGER_ROBE, PART_POSE_ZERO,
};

pub(in crate::entity_models) const ILLAGER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -10.0, -4.0],
    size: [8.0, 10.0, 8.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.45, -10.45, -4.45],
    size: [8.9, 12.9, 8.9],
    color: ILLAGER_HAT_COLOR,
}];

pub(in crate::entity_models) const ILLAGER_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -6.0],
    size: [2.0, 4.0, 2.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, 0.0, -3.0],
        size: [8.0, 12.0, 6.0],
        color: ILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-4.5, -0.5, -3.5],
        size: [9.0, 21.0, 7.0],
        color: ILLAGER_ROBE,
    },
];

pub(in crate::entity_models) const ILLAGER_CROSSED_ARMS: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-8.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: ILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-4.0, 2.0, -2.0],
        size: [8.0, 4.0, 4.0],
        color: ILLAGER_ROBE,
    },
];

pub(in crate::entity_models) const ILLAGER_LEFT_SHOULDER: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [4.0, -2.0, -2.0],
    size: [4.0, 8.0, 4.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_NOSE,
    children: &[],
}];

pub(in crate::entity_models) const ILLAGER_HEAD_WITH_HAT_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HAT,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_NOSE,
        children: &[],
    },
];

pub(in crate::entity_models) const ILLAGER_CROSSED_ARM_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_LEFT_SHOULDER,
        children: &[],
    }];

pub(in crate::entity_models) const ILLAGER_CROSSED_ARM_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [0.0, 3.0, -1.0],
        rotation: [-0.75, 0.0, 0.0],
    },
    cubes: &ILLAGER_CROSSED_ARMS,
    children: &ILLAGER_CROSSED_ARM_CHILDREN,
};

pub(in crate::entity_models) const ILLAGER_RIGHT_ARM_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [-5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_RIGHT_ARM,
    children: &[],
};

pub(in crate::entity_models) const ILLAGER_LEFT_ARM_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_LEFT_ARM,
    children: &[],
};

// Vanilla 26.1 IllagerModel.createBodyLayer(), with LayerDefinitions'
// MeshTransformer.scaling(0.9375F) applied by the emitter root transform.
pub(in crate::entity_models) const ILLAGER_SHARED_CROSSED_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HEAD,
        children: &ILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_BODY,
        children: &[],
    },
    ILLAGER_CROSSED_ARM_PART,
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const ILLAGER_SHARED_UNCROSSED_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HEAD,
        children: &ILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ILLAGER_RIGHT_ARM_PART,
    ILLAGER_LEFT_ARM_PART,
];

pub(in crate::entity_models) const ILLAGER_ILLUSIONER_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HEAD,
        children: &ILLAGER_HEAD_WITH_HAT_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_BODY,
        children: &[],
    },
    ILLAGER_CROSSED_ARM_PART,
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
];
