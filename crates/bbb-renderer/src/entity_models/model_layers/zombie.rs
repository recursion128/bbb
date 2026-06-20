use super::{ModelCubeDesc, ModelPartDesc, PartPose, PART_POSE_ZERO};

pub(in crate::entity_models) const ZOMBIE_GREEN: [f32; 4] = [0.33, 0.62, 0.34, 1.0];
pub(in crate::entity_models) const HUSK_TAN: [f32; 4] = [0.60, 0.50, 0.31, 1.0];
pub(in crate::entity_models) const DROWNED_BLUE: [f32; 4] = [0.23, 0.48, 0.55, 1.0];
pub(in crate::entity_models) const ZOMBIE_VILLAGER_ROBE: [f32; 4] = [0.38, 0.55, 0.34, 1.0];

pub(in crate::entity_models) const ADULT_ZOMBIE_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -4.0],
    size: [8.0, 8.0, 8.0],
    color: ZOMBIE_GREEN,
}];

pub(in crate::entity_models) const ADULT_ZOMBIE_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -8.5, -4.5],
    size: [9.0, 9.0, 9.0],
    color: ZOMBIE_GREEN,
}];

pub(in crate::entity_models) const ADULT_ZOMBIE_HEAD_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_ZOMBIE_HAT,
        children: &[],
    }];

pub(in crate::entity_models) const ADULT_ZOMBIE_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: ZOMBIE_GREEN,
}];

pub(in crate::entity_models) const ADULT_ZOMBIE_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ZOMBIE_GREEN,
}];

pub(in crate::entity_models) const ADULT_ZOMBIE_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ZOMBIE_GREEN,
}];

pub(in crate::entity_models) const ADULT_ZOMBIE_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ZOMBIE_GREEN,
}];

// Vanilla 26.1 ModelLayers.ZOMBIE: HumanoidModel.createMesh(CubeDeformation.NONE, 0.0F).
pub(in crate::entity_models) const ADULT_ZOMBIE_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_ZOMBIE_HEAD,
        children: &ADULT_ZOMBIE_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_ZOMBIE_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_ZOMBIE_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -2.5, -1.0],
    size: [4.0, 5.0, 2.0],
    color: ZOMBIE_GREEN,
}];

pub(in crate::entity_models) const BABY_ZOMBIE_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-3.0, -6.25, -3.0],
        size: [6.0, 6.0, 6.0],
        color: ZOMBIE_GREEN,
    },
    // BabyZombieModel bakes CubeDeformation(0.25F) into ModelPart.Cube bounds.
    ModelCubeDesc {
        min: [-3.25, -6.4, -3.25],
        size: [6.5, 6.5, 6.5],
        color: ZOMBIE_GREEN,
    },
];

pub(in crate::entity_models) const BABY_ZOMBIE_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -0.5, -1.0],
    size: [2.0, 5.0, 2.0],
    color: ZOMBIE_GREEN,
}];

pub(in crate::entity_models) const BABY_ZOMBIE_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 4.0, 2.0],
    color: ZOMBIE_GREEN,
}];

// Vanilla 26.1 BabyZombieModel.createBodyLayer(CubeDeformation.NONE).
pub(in crate::entity_models) const BABY_ZOMBIE_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 17.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.25, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 15.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 15.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, -10.0, -4.0],
        size: [8.0, 10.0, 8.0],
        color: ZOMBIE_VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-1.0, -3.0, -6.0],
        size: [2.0, 4.0, 2.0],
        color: ZOMBIE_VILLAGER_ROBE,
    },
];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_HAT: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-4.5, -10.5, -4.5],
        size: [9.0, 11.0, 9.0],
        color: ZOMBIE_VILLAGER_ROBE,
    }];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_HAT_RIM: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-8.0, -8.0, -6.0],
        size: [16.0, 16.0, 1.0],
        color: ZOMBIE_VILLAGER_ROBE,
    }];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, 0.0, -3.0],
        size: [8.0, 12.0, 6.0],
        color: ZOMBIE_VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-4.05, -0.05, -3.05],
        size: [8.1, 20.1, 6.1],
        color: ZOMBIE_VILLAGER_ROBE,
    },
];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_RIGHT_ARM: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-3.0, -2.0, -2.0],
        size: [4.0, 12.0, 4.0],
        color: ZOMBIE_VILLAGER_ROBE,
    }];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_LEFT_ARM: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-1.0, -2.0, -2.0],
        size: [4.0, 12.0, 4.0],
        color: ZOMBIE_VILLAGER_ROBE,
    }];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 12.0, 4.0],
        color: ZOMBIE_VILLAGER_ROBE,
    }];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_HAT_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 0.0, 0.0],
            rotation: [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_VILLAGER_HAT_RIM,
        children: &[],
    }];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_HEAD_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_ZOMBIE_VILLAGER_HAT,
        children: &ADULT_ZOMBIE_VILLAGER_HAT_CHILDREN,
    }];

// Vanilla 26.1 ZombieVillagerModel.createBodyLayer().
pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_ZOMBIE_VILLAGER_HEAD,
        children: &ADULT_ZOMBIE_VILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_ZOMBIE_VILLAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_VILLAGER_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_VILLAGER_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_VILLAGER_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-2.0, -2.75, -1.5],
        size: [4.0, 5.0, 3.0],
        color: ZOMBIE_VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-2.1, -2.85, -1.6],
        size: [4.2, 6.2, 3.2],
        color: ZOMBIE_VILLAGER_ROBE,
    },
];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_HEAD: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-4.0, -8.0, -3.5],
        size: [8.0, 8.0, 7.0],
        color: ZOMBIE_VILLAGER_ROBE,
    }];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.3, -4.3, -3.8],
    size: [8.6, 8.6, 7.6],
    color: ZOMBIE_VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_HAT_RIM: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-7.0, -0.5, -6.0],
        size: [14.0, 1.0, 12.0],
        color: ZOMBIE_VILLAGER_ROBE,
    }];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_NOSE: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-1.0, -1.0, -0.5],
        size: [2.0, 2.0, 1.0],
        color: ZOMBIE_VILLAGER_ROBE,
    }];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -0.5, -1.0],
    size: [2.0, 5.0, 2.0],
    color: ZOMBIE_VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -0.5, -1.0],
    size: [2.0, 3.0, 2.0],
    color: ZOMBIE_VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -4.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_HAT,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -4.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_HAT_RIM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -1.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_NOSE,
        children: &[],
    },
];

// Vanilla 26.1 BabyZombieVillagerModel.createBodyLayer().
pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.75, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 16.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_HEAD,
        children: &BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 15.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 15.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 21.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 21.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_LEG,
        children: &[],
    },
];
