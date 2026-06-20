use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, GOAT_BEARD, GOAT_HORN, GOAT_WHITE, PART_POSE_ZERO,
};

pub(in crate::entity_models) const ADULT_GOAT_HEAD: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-6.0, -11.0, -10.0],
        size: [3.0, 2.0, 1.0],
        color: GOAT_WHITE,
    },
    ModelCubeDesc {
        min: [2.0, -11.0, -10.0],
        size: [3.0, 2.0, 1.0],
        color: GOAT_WHITE,
    },
    ModelCubeDesc {
        min: [-0.5, -3.0, -14.0],
        size: [0.0, 7.0, 5.0],
        color: GOAT_BEARD,
    },
];

pub(in crate::entity_models) const ADULT_GOAT_LEFT_HORN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.01, -16.0, -10.0],
    size: [2.0, 7.0, 2.0],
    color: GOAT_HORN,
}];

pub(in crate::entity_models) const ADULT_GOAT_RIGHT_HORN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.99, -16.0, -10.0],
    size: [2.0, 7.0, 2.0],
    color: GOAT_HORN,
}];

pub(in crate::entity_models) const ADULT_GOAT_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -4.0, -8.0],
    size: [5.0, 7.0, 10.0],
    color: GOAT_WHITE,
}];

pub(in crate::entity_models) const ADULT_GOAT_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, -17.0, -7.0],
        size: [9.0, 11.0, 16.0],
        color: GOAT_WHITE,
    },
    ModelCubeDesc {
        min: [-5.0, -18.0, -8.0],
        size: [11.0, 14.0, 11.0],
        color: GOAT_WHITE,
    },
];

pub(in crate::entity_models) const ADULT_GOAT_HIND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 4.0, 0.0],
    size: [3.0, 6.0, 3.0],
    color: GOAT_WHITE,
}];

pub(in crate::entity_models) const ADULT_GOAT_FRONT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [3.0, 10.0, 3.0],
    color: GOAT_WHITE,
}];

pub(in crate::entity_models) const ADULT_GOAT_LEFT_HORN_PART: ModelPartDesc = ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_GOAT_LEFT_HORN,
    children: &[],
};

pub(in crate::entity_models) const ADULT_GOAT_RIGHT_HORN_PART: ModelPartDesc = ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_GOAT_RIGHT_HORN,
    children: &[],
};

pub(in crate::entity_models) const ADULT_GOAT_NOSE_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -8.0, -8.0],
        rotation: [0.9599, 0.0, 0.0],
    },
    cubes: &ADULT_GOAT_NOSE,
    children: &[],
};

pub(in crate::entity_models) const ADULT_GOAT_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    ADULT_GOAT_LEFT_HORN_PART,
    ADULT_GOAT_RIGHT_HORN_PART,
    ADULT_GOAT_NOSE_PART,
];

pub(in crate::entity_models) const ADULT_GOAT_HEAD_INDEX: usize = 0;
pub(in crate::entity_models) const ADULT_GOAT_LEFT_HORN_CHILD_INDEX: usize = 0;
pub(in crate::entity_models) const ADULT_GOAT_RIGHT_HORN_CHILD_INDEX: usize = 1;

// Vanilla 26.1 ModelLayers.GOAT: GoatModel.createBodyLayer().
pub(in crate::entity_models) const ADULT_GOAT_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 14.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_GOAT_HEAD,
        children: &ADULT_GOAT_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 24.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_GOAT_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 14.0, 4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_GOAT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 14.0, 4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_GOAT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 14.0, -6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_GOAT_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 14.0, -6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_GOAT_FRONT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_GOAT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -0.5, -1.0],
    size: [2.0, 5.0, 2.0],
    color: GOAT_WHITE,
}];

pub(in crate::entity_models) const BABY_GOAT_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-3.0, -2.3, -4.5],
        size: [6.0, 5.0, 9.0],
        color: GOAT_WHITE,
    },
    ModelCubeDesc {
        min: [-2.5, -2.2, -4.0],
        size: [5.0, 4.0, 8.0],
        color: GOAT_WHITE,
    },
];

pub(in crate::entity_models) const BABY_GOAT_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -3.8126, -5.1548],
    size: [4.0, 4.0, 6.0],
    color: GOAT_WHITE,
}];

pub(in crate::entity_models) const BABY_GOAT_RIGHT_HORN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -4.5, 0.0],
    size: [1.0, 2.0, 1.0],
    color: GOAT_HORN,
}];

pub(in crate::entity_models) const BABY_GOAT_LEFT_HORN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [2.0, -4.5, 0.0],
    size: [1.0, 2.0, 1.0],
    color: GOAT_HORN,
}];

pub(in crate::entity_models) const BABY_GOAT_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -0.5, -0.5],
    size: [2.0, 1.0, 1.0],
    color: GOAT_WHITE,
}];

pub(in crate::entity_models) const BABY_GOAT_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -0.5, -0.5],
    size: [2.0, 1.0, 1.0],
    color: GOAT_WHITE,
}];

pub(in crate::entity_models) const BABY_GOAT_HEAD_MAIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -2.5, -4.0],
    size: [4.0, 4.0, 6.0],
    color: GOAT_WHITE,
}];

pub(in crate::entity_models) const BABY_GOAT_RIGHT_HORN_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [-1.5, -1.5, -1.0],
        rotation: [-0.3926991, 0.0, 0.0],
    },
    cubes: &BABY_GOAT_RIGHT_HORN,
    children: &[],
};

pub(in crate::entity_models) const BABY_GOAT_LEFT_HORN_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [-1.5, -1.5, -1.0],
        rotation: [-0.3926991, 0.0, 0.0],
    },
    cubes: &BABY_GOAT_LEFT_HORN,
    children: &[],
};

pub(in crate::entity_models) const BABY_GOAT_RIGHT_EAR_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [-1.7, -2.3126, 0.1452],
        rotation: [0.0, -0.5236, 0.0],
    },
    cubes: &BABY_GOAT_RIGHT_EAR,
    children: &[],
};

pub(in crate::entity_models) const BABY_GOAT_LEFT_EAR_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [1.7, -2.3126, 0.1452],
        rotation: [0.0, 0.5236, 0.0],
    },
    cubes: &BABY_GOAT_LEFT_EAR,
    children: &[],
};

pub(in crate::entity_models) const BABY_GOAT_HEAD_MAIN_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -1.3126, -1.1548],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &BABY_GOAT_HEAD_MAIN,
    children: &[],
};

pub(in crate::entity_models) const BABY_GOAT_HEAD_CHILDREN: [ModelPartDesc; 5] = [
    BABY_GOAT_RIGHT_HORN_PART,
    BABY_GOAT_LEFT_HORN_PART,
    BABY_GOAT_RIGHT_EAR_PART,
    BABY_GOAT_LEFT_EAR_PART,
    BABY_GOAT_HEAD_MAIN_PART,
];

pub(in crate::entity_models) const BABY_GOAT_HEAD_INDEX: usize = 5;
pub(in crate::entity_models) const BABY_GOAT_LEFT_HORN_CHILD_INDEX: usize = 1;
pub(in crate::entity_models) const BABY_GOAT_RIGHT_HORN_CHILD_INDEX: usize = 0;

// Vanilla 26.1 ModelLayers.GOAT_BABY: BabyGoatModel.createBodyLayer().
pub(in crate::entity_models) const BABY_GOAT_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [1.5, 19.5, 3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_GOAT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.5, 19.5, 3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_GOAT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.5, 19.5, -2.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_GOAT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.5, 19.5, -2.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_GOAT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 17.8, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_GOAT_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.5, -3.0],
            rotation: [0.4363, 0.0, 0.0],
        },
        cubes: &BABY_GOAT_HEAD,
        children: &BABY_GOAT_HEAD_CHILDREN,
    },
];
