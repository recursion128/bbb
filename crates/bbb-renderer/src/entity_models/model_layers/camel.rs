use super::{ModelCubeDesc, ModelPartDesc, PartPose, CAMEL_TAN};

pub(in crate::entity_models) const ADULT_CAMEL_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-7.5, -12.0, -23.5],
    size: [15.0, 12.0, 27.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const ADULT_CAMEL_HUMP: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -5.0, -5.5],
    size: [9.0, 5.0, 11.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const ADULT_CAMEL_TAIL: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, 0.0],
    size: [3.0, 14.0, 0.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const ADULT_CAMEL_HEAD: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-3.5, -7.0, -15.0],
        size: [7.0, 8.0, 19.0],
        color: CAMEL_TAN,
    },
    ModelCubeDesc {
        min: [-3.5, -21.0, -15.0],
        size: [7.0, 14.0, 7.0],
        color: CAMEL_TAN,
    },
    ModelCubeDesc {
        min: [-2.5, -21.0, -21.0],
        size: [5.0, 5.0, 6.0],
        color: CAMEL_TAN,
    },
];

pub(in crate::entity_models) const ADULT_CAMEL_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, 0.5, -1.0],
    size: [3.0, 1.0, 2.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const ADULT_CAMEL_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, 0.5, -1.0],
    size: [3.0, 1.0, 2.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const ADULT_CAMEL_LEFT_HIND_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.5, 2.0, -2.5],
        size: [5.0, 21.0, 5.0],
        color: CAMEL_TAN,
    }];

pub(in crate::entity_models) const ADULT_CAMEL_RIGHT_HIND_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.5, 2.0, -2.5],
        size: [5.0, 21.0, 5.0],
        color: CAMEL_TAN,
    }];

pub(in crate::entity_models) const ADULT_CAMEL_LEFT_FRONT_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.5, 2.0, -2.5],
        size: [5.0, 21.0, 5.0],
        color: CAMEL_TAN,
    }];

pub(in crate::entity_models) const ADULT_CAMEL_RIGHT_FRONT_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.5, 2.0, -2.5],
        size: [5.0, 21.0, 5.0],
        color: CAMEL_TAN,
    }];

pub(in crate::entity_models) const ADULT_CAMEL_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, -21.0, -9.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_LEFT_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, -21.0, -9.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_RIGHT_EAR,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_CAMEL_BODY_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -12.0, -10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_HUMP,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -9.0, 3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_TAIL,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -3.0, -19.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_HEAD,
        children: &ADULT_CAMEL_HEAD_CHILDREN,
    },
];

// Vanilla 26.1 ModelLayers.CAMEL: AdultCamelModel.createBodyLayer().
pub(in crate::entity_models) const ADULT_CAMEL_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, 9.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_BODY,
        children: &ADULT_CAMEL_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.9, 1.0, 9.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_LEFT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.9, 1.0, 9.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_RIGHT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.9, 1.0, -10.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_LEFT_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.9, 1.0, -10.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_RIGHT_FRONT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_CAMEL_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -4.0, -8.0],
    size: [9.0, 8.0, 16.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const BABY_CAMEL_TAIL: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -0.5, 0.0],
    size: [3.0, 9.0, 0.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const BABY_CAMEL_HEAD: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-2.5, -3.0, -7.5],
        size: [5.0, 5.0, 7.0],
        color: CAMEL_TAN,
    },
    ModelCubeDesc {
        min: [-2.5, -12.0, -7.5],
        size: [5.0, 9.0, 5.0],
        color: CAMEL_TAN,
    },
    ModelCubeDesc {
        min: [-2.5, -12.0, -10.5],
        size: [5.0, 4.0, 3.0],
        color: CAMEL_TAN,
    },
];

pub(in crate::entity_models) const BABY_CAMEL_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -0.5, -1.0],
    size: [3.0, 1.0, 2.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const BABY_CAMEL_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -0.5, -1.0],
    size: [3.0, 1.0, 2.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const BABY_CAMEL_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -0.5, -1.5],
    size: [3.0, 13.0, 3.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const BABY_CAMEL_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, -11.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_RIGHT_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, -11.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_LEFT_EAR,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_CAMEL_BODY_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -1.5, 8.05],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_TAIL,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 1.0, -7.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_HEAD,
        children: &BABY_CAMEL_HEAD_CHILDREN,
    },
];

// Vanilla 26.1 ModelLayers.CAMEL_BABY: BabyCamelModel.createBodyLayer().
pub(in crate::entity_models) const BABY_CAMEL_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 7.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_BODY,
        children: &BABY_CAMEL_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 11.5, -5.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 11.5, -5.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 11.5, 5.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 11.5, 5.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_LEG,
        children: &[],
    },
];
