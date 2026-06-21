use super::{ModelCubeDesc, ModelPartDesc, PartPose, PART_POSE_ZERO, VILLAGER_ROBE};

pub(in crate::entity_models) const ADULT_VILLAGER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -10.0, -4.0],
    size: [8.0, 10.0, 8.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const ADULT_VILLAGER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.51, -10.51, -4.51],
    size: [9.02, 11.02, 9.02],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const ADULT_VILLAGER_HAT_RIM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-8.0, -8.0, -6.0],
    size: [16.0, 16.0, 1.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const ADULT_VILLAGER_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -6.0],
    size: [2.0, 4.0, 2.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const ADULT_VILLAGER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -3.0],
    size: [8.0, 12.0, 6.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const ADULT_VILLAGER_JACKET: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -0.5, -3.5],
    size: [9.0, 21.0, 7.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const ADULT_VILLAGER_ARMS: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-8.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [4.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-4.0, 2.0, -2.0],
        size: [8.0, 4.0, 4.0],
        color: VILLAGER_ROBE,
    },
];

pub(in crate::entity_models) const ADULT_VILLAGER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const ADULT_VILLAGER_HAT_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 0.0, 0.0],
            rotation: [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_HAT_RIM,
        children: &[],
    }];

pub(in crate::entity_models) const ADULT_VILLAGER_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_VILLAGER_HAT,
        children: &ADULT_VILLAGER_HAT_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_NOSE,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_VILLAGER_BODY_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_VILLAGER_JACKET,
        children: &[],
    }];

// Vanilla 26.1 VillagerModel.createBodyModel(), with LayerDefinitions'
// MeshTransformer.scaling(0.9375F) applied by the emitter root transform.
pub(in crate::entity_models) const ADULT_VILLAGER_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_VILLAGER_HEAD,
        children: &ADULT_VILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_VILLAGER_BODY,
        children: &ADULT_VILLAGER_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 3.0, -1.0],
            rotation: [-0.75, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_ARMS,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_VILLAGER_RIGHT_HAND: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.0, -2.4925, -1.8401],
        size: [2.0, 4.0, 2.0],
        color: VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [5.0, -2.4925, -1.8401],
        size: [2.0, 4.0, 2.0],
        color: VILLAGER_ROBE,
    },
];

pub(in crate::entity_models) const BABY_VILLAGER_MIDDLE_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -0.9924, -0.9825],
    size: [4.0, 2.0, 2.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_VILLAGER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -0.5, -1.0],
    size: [2.0, 3.0, 2.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_VILLAGER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -3.5],
    size: [8.0, 8.0, 7.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_VILLAGER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.3, -4.3, -3.8],
    size: [8.6, 8.6, 7.6],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_VILLAGER_HAT_RIM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-7.0, -0.5, -6.0],
    size: [14.0, 1.0, 12.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_VILLAGER_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -0.5],
    size: [2.0, 2.0, 1.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_VILLAGER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -2.75, -1.5],
    size: [4.0, 5.0, 3.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_VILLAGER_BB_MAIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.7, -8.2, -1.7],
    size: [4.4, 6.4, 3.4],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_VILLAGER_ARMS_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 1.4025, -0.9599],
            rotation: [-1.0472, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_RIGHT_HAND,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 0.9024, -1.8175],
            rotation: [-1.0472, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_MIDDLE_ARM,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_VILLAGER_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -4.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_HAT,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -4.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_HAT_RIM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_NOSE,
        children: &[],
    },
];

// Vanilla 26.1 BabyVillagerModel.createBodyModel().
pub(in crate::entity_models) const BABY_VILLAGER_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 17.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_VILLAGER_ARMS_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 21.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 21.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 16.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_HEAD,
        children: &BABY_VILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.75, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.5, 24.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_BB_MAIN,
        children: &[],
    },
];
