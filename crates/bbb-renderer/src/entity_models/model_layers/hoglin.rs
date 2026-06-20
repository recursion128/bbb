use super::{ModelCubeDesc, ModelPartDesc, PartPose, HOGLIN_RED};

pub(in crate::entity_models) const HOGLIN_HEAD_X_ROT: f32 = 0.87266463;
pub(in crate::entity_models) const HOGLIN_EAR_Z_ROT: f32 = std::f32::consts::PI * 2.0 / 9.0;
pub(in crate::entity_models) const BABY_HOGLIN_HEAD_X_ROT: f32 = 0.8727;
pub(in crate::entity_models) const BABY_HOGLIN_EAR_Z_ROT: f32 = 0.8727;

pub(in crate::entity_models) const ADULT_HOGLIN_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-8.0, -7.0, -13.0],
    size: [16.0, 14.0, 26.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_MANE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.001, -0.001, -9.001],
    size: [0.002, 10.002, 19.002],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-7.0, -3.0, -19.0],
    size: [14.0, 6.0, 19.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-6.0, -1.0, -2.0],
    size: [6.0, 1.0, 4.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -1.0, -2.0],
    size: [6.0, 1.0, 4.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_HORN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -11.0, -1.0],
    size: [2.0, 11.0, 2.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_FRONT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 0.0, -3.0],
    size: [6.0, 14.0, 6.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_HIND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, 0.0, -2.5],
    size: [5.0, 11.0, 5.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_BODY_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -14.0, -7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_MANE,
        children: &[],
    }];

pub(in crate::entity_models) const ADULT_HOGLIN_HEAD_CHILDREN: [ModelPartDesc; 4] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-6.0, -2.0, -3.0],
            rotation: [0.0, 0.0, -HOGLIN_EAR_Z_ROT],
        },
        cubes: &ADULT_HOGLIN_RIGHT_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [6.0, -2.0, -3.0],
            rotation: [0.0, 0.0, HOGLIN_EAR_Z_ROT],
        },
        cubes: &ADULT_HOGLIN_LEFT_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-7.0, 2.0, -12.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_HORN,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [7.0, 2.0, -12.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_HORN,
        children: &[],
    },
];

// Vanilla 26.1 ModelLayers.HOGLIN / ZOGLIN: HoglinModel.createBodyLayer().
pub(in crate::entity_models) const ADULT_HOGLIN_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 7.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_BODY,
        children: &ADULT_HOGLIN_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 2.0, -12.0],
            rotation: [HOGLIN_HEAD_X_ROT, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_HEAD,
        children: &ADULT_HOGLIN_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 10.0, -8.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 10.0, -8.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 13.0, 10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 13.0, 10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_HIND_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_HOGLIN_HEAD: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-5.0, -2.2605, -10.547],
        size: [10.0, 4.0, 12.0],
        color: HOGLIN_RED,
    },
    ModelCubeDesc {
        min: [-7.0, -4.0981, -8.4879],
        size: [2.0, 5.0, 2.0],
        color: HOGLIN_RED,
    },
    ModelCubeDesc {
        min: [5.0, -4.0981, -8.4879],
        size: [2.0, 5.0, 2.0],
        color: HOGLIN_RED,
    },
];

pub(in crate::entity_models) const BABY_HOGLIN_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.02, -14.02, -7.02],
        size: [8.04, 8.04, 14.04],
        color: HOGLIN_RED,
    },
    ModelCubeDesc {
        min: [-0.02, -18.02, -8.02],
        size: [0.04, 6.04, 11.04],
        color: HOGLIN_RED,
    },
];

pub(in crate::entity_models) const BABY_HOGLIN_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.1, -0.5, -2.0],
    size: [6.0, 1.0, 4.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const BABY_HOGLIN_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.9, -0.5, -2.0],
    size: [6.0, 1.0, 4.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const BABY_HOGLIN_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, -1.5],
    size: [3.0, 6.0, 3.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const BABY_HOGLIN_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, -1.0, -1.5],
            rotation: [0.0, 0.0, -BABY_HOGLIN_EAR_Z_ROT],
        },
        cubes: &BABY_HOGLIN_RIGHT_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, -1.0, -1.5],
            rotation: [0.0, 0.0, BABY_HOGLIN_EAR_Z_ROT],
        },
        cubes: &BABY_HOGLIN_LEFT_EAR,
        children: &[],
    },
];

// Vanilla 26.1 ModelLayers.HOGLIN_BABY / ZOGLIN_BABY:
// BabyHoglinModel.createBodyLayer().
pub(in crate::entity_models) const BABY_HOGLIN_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 13.0, -7.0],
            rotation: [BABY_HOGLIN_HEAD_X_ROT, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_HEAD,
        children: &BABY_HOGLIN_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 24.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 18.0, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 18.0, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 18.0, -4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 18.0, -4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_LEG,
        children: &[],
    },
];
