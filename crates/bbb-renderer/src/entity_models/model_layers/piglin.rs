use super::{ModelCubeDesc, ModelPartDesc, PartPose, PART_POSE_ZERO};

pub(in crate::entity_models) const PIGLIN_SKIN: [f32; 4] = [0.74, 0.44, 0.36, 1.0];
pub(in crate::entity_models) const PIGLIN_BRUTE_SKIN: [f32; 4] = [0.58, 0.35, 0.29, 1.0];
pub(in crate::entity_models) const ZOMBIFIED_PIGLIN_SKIN: [f32; 4] = [0.46, 0.62, 0.42, 1.0];

pub(in crate::entity_models) const ADULT_PIGLIN_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-5.0, -8.0, -4.0],
        size: [10.0, 8.0, 8.0],
        color: PIGLIN_SKIN,
    },
    ModelCubeDesc {
        min: [-2.0, -4.0, -5.0],
        size: [4.0, 4.0, 1.0],
        color: PIGLIN_SKIN,
    },
    ModelCubeDesc {
        min: [2.0, -2.0, -5.0],
        size: [1.0, 2.0, 1.0],
        color: PIGLIN_SKIN,
    },
    ModelCubeDesc {
        min: [-3.0, -2.0, -5.0],
        size: [1.0, 2.0, 1.0],
        color: PIGLIN_SKIN,
    },
];

pub(in crate::entity_models) const ADULT_PIGLIN_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, -2.0],
    size: [1.0, 5.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const ADULT_PIGLIN_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -2.0],
    size: [1.0, 5.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const ADULT_PIGLIN_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [4.5, -6.0, 0.0],
            rotation: [0.0, 0.0, -std::f32::consts::FRAC_PI_6],
        },
        cubes: &ADULT_PIGLIN_LEFT_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.5, -6.0, 0.0],
            rotation: [0.0, 0.0, std::f32::consts::FRAC_PI_6],
        },
        cubes: &ADULT_PIGLIN_RIGHT_EAR,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_PIGLIN_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const ADULT_PIGLIN_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const ADULT_PIGLIN_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const ADULT_PIGLIN_RIGHT_SLEEVE: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-3.25, -2.25, -2.25],
        size: [4.5, 12.5, 4.5],
        color: PIGLIN_SKIN,
    }];

pub(in crate::entity_models) const ADULT_PIGLIN_LEFT_SLEEVE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.25, -2.25, -2.25],
    size: [4.5, 12.5, 4.5],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const ADULT_PIGLIN_RIGHT_ARM_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_PIGLIN_RIGHT_SLEEVE,
        children: &[],
    }];

pub(in crate::entity_models) const ADULT_PIGLIN_LEFT_ARM_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_PIGLIN_LEFT_SLEEVE,
        children: &[],
    }];

pub(in crate::entity_models) const ADULT_PIGLIN_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const ADULT_PIGLIN_PANTS: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.25, -0.25, -2.25],
    size: [4.5, 12.5, 4.5],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const ADULT_PIGLIN_LEG_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_PIGLIN_PANTS,
        children: &[],
    }];

// Vanilla 26.1 AdultPiglinModel.createBodyLayer().
pub(in crate::entity_models) const ADULT_PIGLIN_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_PIGLIN_HEAD,
        children: &ADULT_PIGLIN_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_PIGLIN_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIGLIN_RIGHT_ARM,
        children: &ADULT_PIGLIN_RIGHT_ARM_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIGLIN_LEFT_ARM,
        children: &ADULT_PIGLIN_LEFT_ARM_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIGLIN_LEG,
        children: &ADULT_PIGLIN_LEG_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIGLIN_LEG,
        children: &ADULT_PIGLIN_LEG_CHILDREN,
    },
];

pub(in crate::entity_models) const BABY_PIGLIN_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -3.0, -1.0],
    size: [6.0, 5.0, 3.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const BABY_PIGLIN_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.5, -3.0, -4.5],
        size: [3.0, 3.0, 1.0],
        color: PIGLIN_SKIN,
    },
    ModelCubeDesc {
        min: [-4.5, -6.0, -3.5],
        size: [9.0, 6.0, 7.0],
        color: PIGLIN_SKIN,
    },
];

pub(in crate::entity_models) const BABY_PIGLIN_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, -3.0, -2.0],
    size: [1.0, 6.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const BABY_PIGLIN_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, -3.0, -2.0],
    size: [1.0, 6.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const BABY_PIGLIN_HAT_CHILD: ModelPartDesc = ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &[],
    children: &[],
};

pub(in crate::entity_models) const BABY_PIGLIN_LEFT_EAR_ROTATED_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 1.75, 0.0],
            rotation: [0.0, 0.0, -0.6109],
        },
        cubes: &BABY_PIGLIN_LEFT_EAR,
        children: &[],
    }];

pub(in crate::entity_models) const BABY_PIGLIN_RIGHT_EAR_ROTATED_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 1.75, 0.0],
            rotation: [0.0, 0.0, 0.6109],
        },
        cubes: &BABY_PIGLIN_RIGHT_EAR,
        children: &[],
    }];

pub(in crate::entity_models) const BABY_PIGLIN_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    BABY_PIGLIN_HAT_CHILD,
    ModelPartDesc {
        pose: PartPose {
            offset: [4.2, -4.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_PIGLIN_LEFT_EAR_ROTATED_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.2, -4.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_PIGLIN_RIGHT_EAR_ROTATED_CHILDREN,
    },
];

pub(in crate::entity_models) const BABY_PIGLIN_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.5],
    size: [2.0, 5.0, 3.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const BABY_PIGLIN_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.5],
    size: [2.0, 5.0, 3.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const BABY_PIGLIN_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, -1.5],
    size: [3.0, 4.0, 3.0],
    color: PIGLIN_SKIN,
}];

// Vanilla 26.1 BabyPiglinModel.createBodyLayer().
pub(in crate::entity_models) const BABY_PIGLIN_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.0, -0.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_HEAD,
        children: &BABY_PIGLIN_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 15.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 15.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.5, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.5, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_LEG,
        children: &[],
    },
];
