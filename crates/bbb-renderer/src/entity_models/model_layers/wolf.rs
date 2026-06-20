use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    PART_POSE_ZERO,
};

pub(in crate::entity_models) const WOLF_GRAY: [f32; 4] = [0.64, 0.66, 0.66, 1.0];

pub(in crate::entity_models) const MODEL_LAYER_WOLF: &str = "minecraft:wolf#main";
pub(in crate::entity_models) const MODEL_LAYER_WOLF_BABY: &str = "minecraft:wolf_baby#main";

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_REAL_HEAD: [TexturedModelCubeDesc; 4] = [
    TexturedModelCubeDesc {
        min: [-2.0, -3.0, -2.0],
        size: [6.0, 6.0, 4.0],
        uv_size: [6.0, 6.0, 4.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-2.0, -5.0, 0.0],
        size: [2.0, 2.0, 1.0],
        uv_size: [2.0, 2.0, 1.0],
        tex: [16.0, 14.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [2.0, -5.0, 0.0],
        size: [2.0, 2.0, 1.0],
        uv_size: [2.0, 2.0, 1.0],
        tex: [16.0, 14.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-0.5, -0.001, -5.0],
        size: [3.0, 3.0, 4.0],
        uv_size: [3.0, 3.0, 4.0],
        tex: [0.0, 10.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_WOLF_TEXTURED_REAL_HEAD,
        children: &[],
    }];

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -2.0, -3.0],
        size: [6.0, 9.0, 6.0],
        uv_size: [6.0, 9.0, 6.0],
        tex: [18.0, 14.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_UPPER_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -3.0, -3.0],
        size: [8.0, 6.0, 7.0],
        uv_size: [8.0, 6.0, 7.0],
        tex: [21.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, -1.0],
        size: [2.0, 8.0, 2.0],
        uv_size: [2.0, 8.0, 2.0],
        tex: [0.0, 18.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, -1.0],
        size: [2.0, 8.0, 2.0],
        uv_size: [2.0, 8.0, 2.0],
        tex: [0.0, 18.0],
        mirror: true,
    }];

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_REAL_TAIL: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, -1.0],
        size: [2.0, 8.0, 2.0],
        uv_size: [2.0, 8.0, 2.0],
        tex: [9.0, 18.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_TAIL_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_WOLF_TEXTURED_REAL_TAIL,
        children: &[],
    }];

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_PARTS: [TexturedModelPartDesc; 8] = [
    TexturedModelPartDesc {
        pose: ADULT_WOLF_PARTS[0].pose,
        cubes: &[],
        children: &ADULT_WOLF_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: ADULT_WOLF_PARTS[1].pose,
        cubes: &ADULT_WOLF_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_WOLF_PARTS[2].pose,
        cubes: &ADULT_WOLF_TEXTURED_UPPER_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_WOLF_PARTS[3].pose,
        cubes: &ADULT_WOLF_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_WOLF_PARTS[4].pose,
        cubes: &ADULT_WOLF_TEXTURED_LEFT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_WOLF_PARTS[5].pose,
        cubes: &ADULT_WOLF_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_WOLF_PARTS[6].pose,
        cubes: &ADULT_WOLF_TEXTURED_LEFT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_WOLF_PARTS[7].pose,
        cubes: &[],
        children: &ADULT_WOLF_TEXTURED_TAIL_CHILDREN,
    },
];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_HEAD: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-3.015, -3.275, -3.025],
        size: [6.05, 5.05, 5.05],
        uv_size: [6.0, 5.0, 5.0],
        tex: [0.0, 12.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-1.5, -0.24, -5.0],
        size: [3.0, 2.0, 2.0],
        uv_size: [3.0, 2.0, 2.0],
        tex: [17.0, 12.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_RIGHT_EAR: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -1.0, -0.5],
        size: [2.0, 2.0, 1.0],
        uv_size: [2.0, 2.0, 1.0],
        tex: [0.0, 5.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_LEFT_EAR: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -1.0, -0.5],
        size: [2.0, 2.0, 1.0],
        uv_size: [2.0, 2.0, 1.0],
        tex: [20.0, 5.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: BABY_WOLF_HEAD_CHILDREN[0].pose,
        cubes: &BABY_WOLF_TEXTURED_RIGHT_EAR,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_WOLF_HEAD_CHILDREN[1].pose,
        cubes: &BABY_WOLF_TEXTURED_LEFT_EAR,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -2.0, -4.0],
        size: [6.0, 4.0, 8.0],
        uv_size: [6.0, 4.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_RIGHT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 3.0, 2.0],
        uv_size: [2.0, 3.0, 2.0],
        tex: [0.0, 22.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_LEFT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 3.0, 2.0],
        uv_size: [2.0, 3.0, 2.0],
        tex: [8.0, 22.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_RIGHT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 3.0, 2.0],
        uv_size: [2.0, 3.0, 2.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_LEFT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 3.0, 2.0],
        uv_size: [2.0, 3.0, 2.0],
        tex: [20.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_TAIL_R1: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -5.7, -1.0],
        size: [2.0, 6.0, 2.0],
        uv_size: [2.0, 6.0, 2.0],
        tex: [22.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_TAIL_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: BABY_WOLF_TAIL_CHILDREN[0].pose,
        cubes: &BABY_WOLF_TEXTURED_TAIL_R1,
        children: &[],
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_PARTS: [TexturedModelPartDesc; 7] = [
    TexturedModelPartDesc {
        pose: BABY_WOLF_PARTS[0].pose,
        cubes: &BABY_WOLF_TEXTURED_HEAD,
        children: &BABY_WOLF_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: BABY_WOLF_PARTS[1].pose,
        cubes: &BABY_WOLF_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_WOLF_PARTS[2].pose,
        cubes: &BABY_WOLF_TEXTURED_RIGHT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_WOLF_PARTS[3].pose,
        cubes: &BABY_WOLF_TEXTURED_LEFT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_WOLF_PARTS[4].pose,
        cubes: &BABY_WOLF_TEXTURED_RIGHT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_WOLF_PARTS[5].pose,
        cubes: &BABY_WOLF_TEXTURED_LEFT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_WOLF_PARTS[6].pose,
        cubes: &[],
        children: &BABY_WOLF_TEXTURED_TAIL_CHILDREN,
    },
];

pub(in crate::entity_models) const ADULT_WOLF_REAL_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-2.0, -3.0, -2.0],
        size: [6.0, 6.0, 4.0],
        color: WOLF_GRAY,
    },
    ModelCubeDesc {
        min: [-2.0, -5.0, 0.0],
        size: [2.0, 2.0, 1.0],
        color: WOLF_GRAY,
    },
    ModelCubeDesc {
        min: [2.0, -5.0, 0.0],
        size: [2.0, 2.0, 1.0],
        color: WOLF_GRAY,
    },
    ModelCubeDesc {
        min: [-0.5, -0.001, -5.0],
        size: [3.0, 3.0, 4.0],
        color: WOLF_GRAY,
    },
];

pub(in crate::entity_models) const ADULT_WOLF_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_WOLF_REAL_HEAD,
    children: &[],
}];

pub(in crate::entity_models) const ADULT_WOLF_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -3.0],
    size: [6.0, 9.0, 6.0],
    color: WOLF_GRAY,
}];

pub(in crate::entity_models) const ADULT_WOLF_UPPER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -3.0, -3.0],
    size: [8.0, 6.0, 7.0],
    color: WOLF_GRAY,
}];

pub(in crate::entity_models) const ADULT_WOLF_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, -1.0],
    size: [2.0, 8.0, 2.0],
    color: WOLF_GRAY,
}];

pub(in crate::entity_models) const ADULT_WOLF_REAL_TAIL: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, -1.0],
    size: [2.0, 8.0, 2.0],
    color: WOLF_GRAY,
}];

pub(in crate::entity_models) const ADULT_WOLF_TAIL_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_WOLF_REAL_TAIL,
    children: &[],
}];

// Vanilla 26.1 AdultWolfModel.createBodyLayer(CubeDeformation.NONE).
pub(in crate::entity_models) const ADULT_WOLF_PARTS: [ModelPartDesc; 8] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 13.5, -7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &ADULT_WOLF_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 14.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_WOLF_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 14.0, -3.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_WOLF_UPPER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 16.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_WOLF_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.5, 16.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_WOLF_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 16.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_WOLF_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.5, 16.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_WOLF_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 12.0, 8.0],
            rotation: [0.62831855, 0.0, 0.0],
        },
        cubes: &[],
        children: &ADULT_WOLF_TAIL_CHILDREN,
    },
];

pub(in crate::entity_models) const BABY_WOLF_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-3.015, -3.275, -3.025],
        size: [6.05, 5.05, 5.05],
        color: WOLF_GRAY,
    },
    ModelCubeDesc {
        min: [-1.5, -0.24, -5.0],
        size: [3.0, 2.0, 2.0],
        color: WOLF_GRAY,
    },
];

pub(in crate::entity_models) const BABY_WOLF_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -0.5],
    size: [2.0, 2.0, 1.0],
    color: WOLF_GRAY,
}];

pub(in crate::entity_models) const BABY_WOLF_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -4.25, -0.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_WOLF_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, -4.25, -0.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_WOLF_EAR,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_WOLF_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -4.0],
    size: [6.0, 4.0, 8.0],
    color: WOLF_GRAY,
}];

pub(in crate::entity_models) const BABY_WOLF_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 3.0, 2.0],
    color: WOLF_GRAY,
}];

pub(in crate::entity_models) const BABY_WOLF_TAIL_R1: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -5.7, -1.0],
    size: [2.0, 6.0, 2.0],
    color: WOLF_GRAY,
}];

pub(in crate::entity_models) const BABY_WOLF_TAIL_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -0.6, 0.2],
        rotation: [-3.1, 0.0, 0.0],
    },
    cubes: &BABY_WOLF_TAIL_R1,
    children: &[],
}];

// Vanilla 26.1 BabyWolfModel.createBodyLayer().
pub(in crate::entity_models) const BABY_WOLF_PARTS: [ModelPartDesc; 7] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.25, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_WOLF_HEAD,
        children: &BABY_WOLF_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 19.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_WOLF_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.5, 21.0, 3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_WOLF_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.5, 21.0, 3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_WOLF_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.5, 21.0, -3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_WOLF_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.5, 21.0, -3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_WOLF_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 19.0, 3.0],
            rotation: [-0.5236, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_WOLF_TAIL_CHILDREN,
    },
];
