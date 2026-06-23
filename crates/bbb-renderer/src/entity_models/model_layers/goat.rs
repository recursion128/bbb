use super::{
    apply_head_look, apply_quadruped_leg_swing, ModelCubeDesc, ModelPartDesc, PartPose,
    TexturedModelCubeDesc, TexturedModelPartDesc, GOAT_BEARD, GOAT_HORN, GOAT_WHITE,
    PART_POSE_ZERO,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_GOAT: &str = "minecraft:goat#main";
pub(in crate::entity_models) const MODEL_LAYER_GOAT_BABY: &str = "minecraft:goat_baby#main";

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

pub(in crate::entity_models) const ADULT_GOAT_TEXTURED_HEAD: [TexturedModelCubeDesc; 3] = [
    TexturedModelCubeDesc {
        min: [-6.0, -11.0, -10.0],
        size: [3.0, 2.0, 1.0],
        uv_size: [3.0, 2.0, 1.0],
        tex: [2.0, 61.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [2.0, -11.0, -10.0],
        size: [3.0, 2.0, 1.0],
        uv_size: [3.0, 2.0, 1.0],
        tex: [2.0, 61.0],
        mirror: true,
    },
    TexturedModelCubeDesc {
        min: [-0.5, -3.0, -14.0],
        size: [0.0, 7.0, 5.0],
        uv_size: [0.0, 7.0, 5.0],
        tex: [23.0, 52.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const ADULT_GOAT_TEXTURED_LEFT_HORN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-0.01, -16.0, -10.0],
        size: [2.0, 7.0, 2.0],
        uv_size: [2.0, 7.0, 2.0],
        tex: [12.0, 55.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_GOAT_TEXTURED_RIGHT_HORN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.99, -16.0, -10.0],
        size: [2.0, 7.0, 2.0],
        uv_size: [2.0, 7.0, 2.0],
        tex: [12.0, 55.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_GOAT_TEXTURED_NOSE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -4.0, -8.0],
        size: [5.0, 7.0, 10.0],
        uv_size: [5.0, 7.0, 10.0],
        tex: [34.0, 46.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_GOAT_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-4.0, -17.0, -7.0],
        size: [9.0, 11.0, 16.0],
        uv_size: [9.0, 11.0, 16.0],
        tex: [1.0, 1.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-5.0, -18.0, -8.0],
        size: [11.0, 14.0, 11.0],
        uv_size: [11.0, 14.0, 11.0],
        tex: [0.0, 28.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const ADULT_GOAT_TEXTURED_LEFT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 4.0, 0.0],
        size: [3.0, 6.0, 3.0],
        uv_size: [3.0, 6.0, 3.0],
        tex: [36.0, 29.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_GOAT_TEXTURED_RIGHT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 4.0, 0.0],
        size: [3.0, 6.0, 3.0],
        uv_size: [3.0, 6.0, 3.0],
        tex: [49.0, 29.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_GOAT_TEXTURED_LEFT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [3.0, 10.0, 3.0],
        uv_size: [3.0, 10.0, 3.0],
        tex: [49.0, 2.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_GOAT_TEXTURED_RIGHT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [3.0, 10.0, 3.0],
        uv_size: [3.0, 10.0, 3.0],
        tex: [35.0, 2.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_GOAT_TEXTURED_LEFT_HORN_PART: TexturedModelPartDesc =
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_GOAT_TEXTURED_LEFT_HORN,
        children: &[],
    };

pub(in crate::entity_models) const ADULT_GOAT_TEXTURED_RIGHT_HORN_PART: TexturedModelPartDesc =
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_GOAT_TEXTURED_RIGHT_HORN,
        children: &[],
    };

pub(in crate::entity_models) const ADULT_GOAT_TEXTURED_NOSE_PART: TexturedModelPartDesc =
    TexturedModelPartDesc {
        pose: ADULT_GOAT_NOSE_PART.pose,
        cubes: &ADULT_GOAT_TEXTURED_NOSE,
        children: &[],
    };

pub(in crate::entity_models) const ADULT_GOAT_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 3] = [
    ADULT_GOAT_TEXTURED_LEFT_HORN_PART,
    ADULT_GOAT_TEXTURED_RIGHT_HORN_PART,
    ADULT_GOAT_TEXTURED_NOSE_PART,
];

pub(in crate::entity_models) const ADULT_GOAT_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: ADULT_GOAT_PARTS[0].pose,
        cubes: &ADULT_GOAT_TEXTURED_HEAD,
        children: &ADULT_GOAT_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: ADULT_GOAT_PARTS[1].pose,
        cubes: &ADULT_GOAT_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_GOAT_PARTS[2].pose,
        cubes: &ADULT_GOAT_TEXTURED_LEFT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_GOAT_PARTS[3].pose,
        cubes: &ADULT_GOAT_TEXTURED_RIGHT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_GOAT_PARTS[4].pose,
        cubes: &ADULT_GOAT_TEXTURED_LEFT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_GOAT_PARTS[5].pose,
        cubes: &ADULT_GOAT_TEXTURED_RIGHT_FRONT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_LEFT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -0.5, -1.0],
        size: [2.0, 5.0, 2.0],
        uv_size: [2.0, 5.0, 2.0],
        tex: [29.0, 12.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_RIGHT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -0.5, -1.0],
        size: [2.0, 5.0, 2.0],
        uv_size: [2.0, 5.0, 2.0],
        tex: [21.0, 12.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_RIGHT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -0.5, -1.0],
        size: [2.0, 5.0, 2.0],
        uv_size: [2.0, 5.0, 2.0],
        tex: [21.0, 5.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_LEFT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -0.5, -1.0],
        size: [2.0, 5.0, 2.0],
        uv_size: [2.0, 5.0, 2.0],
        tex: [29.0, 5.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-3.0, -2.3, -4.5],
        size: [6.0, 5.0, 9.0],
        uv_size: [6.0, 5.0, 9.0],
        tex: [0.0, 10.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-2.5, -2.2, -4.0],
        size: [5.0, 4.0, 8.0],
        uv_size: [5.0, 4.0, 8.0],
        tex: [0.0, 24.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -3.8126, -5.1548],
        size: [4.0, 4.0, 6.0],
        uv_size: [4.0, 4.0, 6.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_RIGHT_HORN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, -4.5, 0.0],
        size: [1.0, 2.0, 1.0],
        uv_size: [1.0, 2.0, 1.0],
        tex: [24.0, 0.0],
        mirror: true,
    }];

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_LEFT_HORN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [2.0, -4.5, 0.0],
        size: [1.0, 2.0, 1.0],
        uv_size: [1.0, 2.0, 1.0],
        tex: [24.0, 0.0],
        mirror: true,
    }];

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_RIGHT_EAR: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -0.5, -0.5],
        size: [2.0, 1.0, 1.0],
        uv_size: [2.0, 1.0, 1.0],
        tex: [0.0, 12.0],
        mirror: true,
    }];

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_LEFT_EAR: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, -0.5, -0.5],
        size: [2.0, 1.0, 1.0],
        uv_size: [2.0, 1.0, 1.0],
        tex: [0.0, 12.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_HEAD_MAIN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -2.5, -4.0],
        size: [4.0, 4.0, 6.0],
        uv_size: [4.0, 4.0, 6.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_RIGHT_HORN_PART: TexturedModelPartDesc =
    TexturedModelPartDesc {
        pose: BABY_GOAT_RIGHT_HORN_PART.pose,
        cubes: &BABY_GOAT_TEXTURED_RIGHT_HORN,
        children: &[],
    };

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_LEFT_HORN_PART: TexturedModelPartDesc =
    TexturedModelPartDesc {
        pose: BABY_GOAT_LEFT_HORN_PART.pose,
        cubes: &BABY_GOAT_TEXTURED_LEFT_HORN,
        children: &[],
    };

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_RIGHT_EAR_PART: TexturedModelPartDesc =
    TexturedModelPartDesc {
        pose: BABY_GOAT_RIGHT_EAR_PART.pose,
        cubes: &BABY_GOAT_TEXTURED_RIGHT_EAR,
        children: &[],
    };

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_LEFT_EAR_PART: TexturedModelPartDesc =
    TexturedModelPartDesc {
        pose: BABY_GOAT_LEFT_EAR_PART.pose,
        cubes: &BABY_GOAT_TEXTURED_LEFT_EAR,
        children: &[],
    };

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_HEAD_MAIN_PART: TexturedModelPartDesc =
    TexturedModelPartDesc {
        pose: BABY_GOAT_HEAD_MAIN_PART.pose,
        cubes: &BABY_GOAT_TEXTURED_HEAD_MAIN,
        children: &[],
    };

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 5] = [
    BABY_GOAT_TEXTURED_RIGHT_HORN_PART,
    BABY_GOAT_TEXTURED_LEFT_HORN_PART,
    BABY_GOAT_TEXTURED_RIGHT_EAR_PART,
    BABY_GOAT_TEXTURED_LEFT_EAR_PART,
    BABY_GOAT_TEXTURED_HEAD_MAIN_PART,
];

pub(in crate::entity_models) const BABY_GOAT_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: BABY_GOAT_PARTS[0].pose,
        cubes: &BABY_GOAT_TEXTURED_LEFT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_GOAT_PARTS[1].pose,
        cubes: &BABY_GOAT_TEXTURED_RIGHT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_GOAT_PARTS[2].pose,
        cubes: &BABY_GOAT_TEXTURED_RIGHT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_GOAT_PARTS[3].pose,
        cubes: &BABY_GOAT_TEXTURED_LEFT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_GOAT_PARTS[4].pose,
        cubes: &BABY_GOAT_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_GOAT_PARTS[5].pose,
        cubes: &BABY_GOAT_TEXTURED_HEAD,
        children: &BABY_GOAT_TEXTURED_HEAD_CHILDREN,
    },
];

/// Per-`baby` goat layout: the head part index, the head's left/right horn child indices, and the
/// four leg part indices (the swing resolves each leg's phase from its offset, so order is free).
struct GoatLayout {
    head_index: usize,
    left_horn_child: usize,
    right_horn_child: usize,
    leg_indices: [usize; 4],
}

const fn goat_layout(baby: bool) -> GoatLayout {
    if baby {
        GoatLayout {
            head_index: BABY_GOAT_HEAD_INDEX,
            left_horn_child: BABY_GOAT_LEFT_HORN_CHILD_INDEX,
            right_horn_child: BABY_GOAT_RIGHT_HORN_CHILD_INDEX,
            leg_indices: [0, 1, 2, 3],
        }
    } else {
        GoatLayout {
            head_index: ADULT_GOAT_HEAD_INDEX,
            left_horn_child: ADULT_GOAT_LEFT_HORN_CHILD_INDEX,
            right_horn_child: ADULT_GOAT_RIGHT_HORN_CHILD_INDEX,
            leg_indices: [2, 3, 4, 5],
        }
    }
}

/// Selects the unified goat part-tree pair (colored + textured) for `baby`.
fn goat_part_trees(baby: bool) -> (&'static [ModelPartDesc], &'static [TexturedModelPartDesc]) {
    if baby {
        (&BABY_GOAT_PARTS, &BABY_GOAT_TEXTURED_PARTS)
    } else {
        (&ADULT_GOAT_PARTS, &ADULT_GOAT_TEXTURED_PARTS)
    }
}

/// Mutable goat model, mirroring vanilla `GoatModel` (a `QuadrupedModel`). The unified tree is zipped
/// from the baked colored and textured trees for the selected `baby` layout ([`goat_part_trees`]).
/// `setup_anim` looks the head ([`apply_head_look`]), swings the four legs ([`apply_quadruped_leg_swing`]),
/// and toggles each horn (a head child) via the [`ModelPart::visible`] flag from the `left_horn`/
/// `right_horn` flags — vanilla hides the screaming-goat-only horns a polled goat lacks. The ramming
/// head tilt is a deferred event animation.
pub(in crate::entity_models) struct GoatModel {
    root: ModelPart,
    baby: bool,
    left_horn: bool,
    right_horn: bool,
}

impl GoatModel {
    pub(in crate::entity_models) fn new(baby: bool, left_horn: bool, right_horn: bool) -> Self {
        let (colored, textured) = goat_part_trees(baby);
        Self {
            root: ModelPart::root_from_descs(colored, textured),
            baby,
            left_horn,
            right_horn,
        }
    }
}

impl EntityModel for GoatModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;
        let layout = goat_layout(self.baby);
        apply_head_look(
            self.root.child_at_mut(layout.head_index),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        apply_quadruped_leg_swing(
            &mut self.root,
            layout.leg_indices,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
        );
        let head = self.root.child_at_mut(layout.head_index);
        head.child_at_mut(layout.left_horn_child).visible = self.left_horn;
        head.child_at_mut(layout.right_horn_child).visible = self.right_horn;
    }
}
