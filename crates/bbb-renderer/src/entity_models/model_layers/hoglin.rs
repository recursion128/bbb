use super::{
    head_look_yaw_pose, hoglin_ear_sway_pose, hoglin_head_part_index, hoglin_leg_swing_pose,
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    HOGLIN_LEFT_EAR_CHILD_INDEX, HOGLIN_RED, HOGLIN_RIGHT_EAR_CHILD_INDEX,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_HOGLIN: &str = "minecraft:hoglin#main";
pub(in crate::entity_models) const MODEL_LAYER_HOGLIN_BABY: &str = "minecraft:hoglin_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOGLIN: &str = "minecraft:zoglin#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOGLIN_BABY: &str = "minecraft:zoglin_baby#main";

pub(in crate::entity_models) const HOGLIN_HEAD_X_ROT: f32 = 0.87266463;
pub(in crate::entity_models) const HOGLIN_EAR_Z_ROT: f32 = std::f32::consts::PI * 2.0 / 9.0;
pub(in crate::entity_models) const BABY_HOGLIN_HEAD_X_ROT: f32 = 0.8727;
pub(in crate::entity_models) const BABY_HOGLIN_EAR_Z_ROT: f32 = 0.8727;

pub(in crate::entity_models) const ADULT_HOGLIN_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-8.0, -7.0, -13.0],
    size: [16.0, 14.0, 26.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-8.0, -7.0, -13.0],
        size: [16.0, 14.0, 26.0],
        uv_size: [16.0, 14.0, 26.0],
        tex: [1.0, 1.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_HOGLIN_MANE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.001, -0.001, -9.001],
    size: [0.002, 10.002, 19.002],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_TEXTURED_MANE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-0.001, -0.001, -9.001],
        size: [0.002, 10.002, 19.002],
        uv_size: [0.0, 10.0, 19.0],
        tex: [90.0, 33.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_HOGLIN_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-7.0, -3.0, -19.0],
    size: [14.0, 6.0, 19.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-7.0, -3.0, -19.0],
        size: [14.0, 6.0, 19.0],
        uv_size: [14.0, 6.0, 19.0],
        tex: [61.0, 1.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_HOGLIN_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-6.0, -1.0, -2.0],
    size: [6.0, 1.0, 4.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_TEXTURED_RIGHT_EAR: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-6.0, -1.0, -2.0],
        size: [6.0, 1.0, 4.0],
        uv_size: [6.0, 1.0, 4.0],
        tex: [1.0, 1.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_HOGLIN_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -1.0, -2.0],
    size: [6.0, 1.0, 4.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_TEXTURED_LEFT_EAR: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, -1.0, -2.0],
        size: [6.0, 1.0, 4.0],
        uv_size: [6.0, 1.0, 4.0],
        tex: [1.0, 6.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_HOGLIN_HORN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -11.0, -1.0],
    size: [2.0, 11.0, 2.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_TEXTURED_RIGHT_HORN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -11.0, -1.0],
        size: [2.0, 11.0, 2.0],
        uv_size: [2.0, 11.0, 2.0],
        tex: [10.0, 13.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_HOGLIN_TEXTURED_LEFT_HORN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -11.0, -1.0],
        size: [2.0, 11.0, 2.0],
        uv_size: [2.0, 11.0, 2.0],
        tex: [1.0, 13.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_HOGLIN_FRONT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 0.0, -3.0],
    size: [6.0, 14.0, 6.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_TEXTURED_RIGHT_FRONT_LEG: [TexturedModelCubeDesc;
    1] = [TexturedModelCubeDesc {
    min: [-3.0, 0.0, -3.0],
    size: [6.0, 14.0, 6.0],
    uv_size: [6.0, 14.0, 6.0],
    tex: [66.0, 42.0],
    mirror: false,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_TEXTURED_LEFT_FRONT_LEG: [TexturedModelCubeDesc;
    1] = [TexturedModelCubeDesc {
    min: [-3.0, 0.0, -3.0],
    size: [6.0, 14.0, 6.0],
    uv_size: [6.0, 14.0, 6.0],
    tex: [41.0, 42.0],
    mirror: false,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_HIND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, 0.0, -2.5],
    size: [5.0, 11.0, 5.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_TEXTURED_RIGHT_HIND_LEG: [TexturedModelCubeDesc;
    1] = [TexturedModelCubeDesc {
    min: [-2.5, 0.0, -2.5],
    size: [5.0, 11.0, 5.0],
    uv_size: [5.0, 11.0, 5.0],
    tex: [21.0, 45.0],
    mirror: false,
}];

pub(in crate::entity_models) const ADULT_HOGLIN_TEXTURED_LEFT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.5, 0.0, -2.5],
        size: [5.0, 11.0, 5.0],
        uv_size: [5.0, 11.0, 5.0],
        tex: [0.0, 45.0],
        mirror: false,
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

pub(in crate::entity_models) const ADULT_HOGLIN_TEXTURED_BODY_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, -14.0, -7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_TEXTURED_MANE,
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

pub(in crate::entity_models) const ADULT_HOGLIN_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 4] = [
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-6.0, -2.0, -3.0],
            rotation: [0.0, 0.0, -HOGLIN_EAR_Z_ROT],
        },
        cubes: &ADULT_HOGLIN_TEXTURED_RIGHT_EAR,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [6.0, -2.0, -3.0],
            rotation: [0.0, 0.0, HOGLIN_EAR_Z_ROT],
        },
        cubes: &ADULT_HOGLIN_TEXTURED_LEFT_EAR,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-7.0, 2.0, -12.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_TEXTURED_RIGHT_HORN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [7.0, 2.0, -12.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_TEXTURED_LEFT_HORN,
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

pub(in crate::entity_models) const ADULT_HOGLIN_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 7.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_TEXTURED_BODY,
        children: &ADULT_HOGLIN_TEXTURED_BODY_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 2.0, -12.0],
            rotation: [HOGLIN_HEAD_X_ROT, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_TEXTURED_HEAD,
        children: &ADULT_HOGLIN_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 10.0, -8.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_TEXTURED_RIGHT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [4.0, 10.0, -8.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_TEXTURED_LEFT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 13.0, 10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_TEXTURED_RIGHT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [5.0, 13.0, 10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_TEXTURED_LEFT_HIND_LEG,
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

pub(in crate::entity_models) const BABY_HOGLIN_TEXTURED_HEAD: [TexturedModelCubeDesc; 3] = [
    TexturedModelCubeDesc {
        min: [-5.0, -2.2605, -10.547],
        size: [10.0, 4.0, 12.0],
        uv_size: [10.0, 4.0, 12.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-7.0, -4.0981, -8.4879],
        size: [2.0, 5.0, 2.0],
        uv_size: [2.0, 5.0, 2.0],
        tex: [44.0, 29.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [5.0, -4.0981, -8.4879],
        size: [2.0, 5.0, 2.0],
        uv_size: [2.0, 5.0, 2.0],
        tex: [52.0, 29.0],
        mirror: false,
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

pub(in crate::entity_models) const BABY_HOGLIN_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-4.02, -14.02, -7.02],
        size: [8.04, 8.04, 14.04],
        uv_size: [8.0, 8.0, 14.0],
        tex: [0.0, 16.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-0.02, -18.02, -8.02],
        size: [0.04, 6.04, 11.04],
        uv_size: [0.0, 6.0, 11.0],
        tex: [24.0, 39.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BABY_HOGLIN_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.1, -0.5, -2.0],
    size: [6.0, 1.0, 4.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const BABY_HOGLIN_TEXTURED_RIGHT_EAR: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-5.1, -0.5, -2.0],
        size: [6.0, 1.0, 4.0],
        uv_size: [6.0, 1.0, 4.0],
        tex: [32.0, 5.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_HOGLIN_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.9, -0.5, -2.0],
    size: [6.0, 1.0, 4.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const BABY_HOGLIN_TEXTURED_LEFT_EAR: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-0.9, -0.5, -2.0],
        size: [6.0, 1.0, 4.0],
        uv_size: [6.0, 1.0, 4.0],
        tex: [32.0, 0.0],
        mirror: true,
    }];

pub(in crate::entity_models) const BABY_HOGLIN_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, -1.5],
    size: [3.0, 6.0, 3.0],
    color: HOGLIN_RED,
}];

pub(in crate::entity_models) const BABY_HOGLIN_TEXTURED_RIGHT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, 0.0, -1.5],
        size: [3.0, 6.0, 3.0],
        uv_size: [3.0, 6.0, 3.0],
        tex: [0.0, 47.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_HOGLIN_TEXTURED_LEFT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, 0.0, -1.5],
        size: [3.0, 6.0, 3.0],
        uv_size: [3.0, 6.0, 3.0],
        tex: [12.0, 47.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_HOGLIN_TEXTURED_RIGHT_FRONT_LEG: [TexturedModelCubeDesc;
    1] = [TexturedModelCubeDesc {
    min: [-1.5, 0.0, -1.5],
    size: [3.0, 6.0, 3.0],
    uv_size: [3.0, 6.0, 3.0],
    tex: [0.0, 38.0],
    mirror: false,
}];

pub(in crate::entity_models) const BABY_HOGLIN_TEXTURED_LEFT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, 0.0, -1.5],
        size: [3.0, 6.0, 3.0],
        uv_size: [3.0, 6.0, 3.0],
        tex: [12.0, 38.0],
        mirror: false,
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

pub(in crate::entity_models) const BABY_HOGLIN_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-5.0, -1.0, -1.5],
            rotation: [0.0, 0.0, -BABY_HOGLIN_EAR_Z_ROT],
        },
        cubes: &BABY_HOGLIN_TEXTURED_RIGHT_EAR,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [5.0, -1.0, -1.5],
            rotation: [0.0, 0.0, BABY_HOGLIN_EAR_Z_ROT],
        },
        cubes: &BABY_HOGLIN_TEXTURED_LEFT_EAR,
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

pub(in crate::entity_models) const BABY_HOGLIN_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 13.0, -7.0],
            rotation: [BABY_HOGLIN_HEAD_X_ROT, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_TEXTURED_HEAD,
        children: &BABY_HOGLIN_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 24.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 18.0, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_TEXTURED_RIGHT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [2.5, 18.0, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_TEXTURED_LEFT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 18.0, -4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_TEXTURED_RIGHT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [2.5, 18.0, -4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_TEXTURED_LEFT_FRONT_LEG,
        children: &[],
    },
];

/// The four leg part indices in the hoglin/zoglin body layers (the head and body occupy `0`/`1` in
/// either order). [`hoglin_leg_swing_pose`] resolves each leg's phase from its offset, so the
/// differing head/body order of the adult and baby layers does not matter.
const HOGLIN_LEG_PART_INDICES: [usize; 4] = [2, 3, 4, 5];

/// Selects the colored ([`ADULT_HOGLIN_PARTS`]/[`BABY_HOGLIN_PARTS`]) and textured
/// ([`ADULT_HOGLIN_TEXTURED_PARTS`]/[`BABY_HOGLIN_TEXTURED_PARTS`]) const trees for a hoglin by
/// `baby`, zipped into the unified tree by [`HoglinModel::new`].
pub(in crate::entity_models) fn hoglin_part_trees(
    baby: bool,
) -> (&'static [ModelPartDesc], &'static [TexturedModelPartDesc]) {
    if baby {
        (&BABY_HOGLIN_PARTS, &BABY_HOGLIN_TEXTURED_PARTS)
    } else {
        (&ADULT_HOGLIN_PARTS, &ADULT_HOGLIN_TEXTURED_PARTS)
    }
}

/// Mutable hoglin model, mirroring vanilla `HoglinModel` (shared by the zoglin). The unified tree is
/// zipped from the colored and textured const trees selected by `baby` ([`hoglin_part_trees`]).
/// `setup_anim` runs the yaw-only head look ([`head_look_yaw_pose`]), sways the two ears — head
/// children at [`HOGLIN_RIGHT_EAR_CHILD_INDEX`]/[`HOGLIN_LEFT_EAR_CHILD_INDEX`]
/// ([`hoglin_ear_sway_pose`], whose `±2π/9` rest also overrides the baby layer's wider baked angle),
/// and swings the four legs ([`hoglin_leg_swing_pose`]). The family recolor/texture and root scale
/// are supplied by the caller; the headbutt head tilt defers.
pub(in crate::entity_models) struct HoglinModel {
    root: ModelPart,
    baby: bool,
}

impl HoglinModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        let (colored, textured) = hoglin_part_trees(baby);
        Self {
            root: ModelPart::root_from_descs(colored, textured),
            baby,
        }
    }
}

impl EntityModel for HoglinModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;
        let limb_swing = render_state.walk_animation_pos;
        let limb_swing_amount = render_state.walk_animation_speed;
        // Yaw-only head look, then sway the two ears (head children). Vanilla overrides the baked ear
        // rest angle to `±2π/9` every frame, so the sway is applied unconditionally (a no-op for the
        // adult layer at rest, an override for the baby layer's wider baked angle).
        let head = self.root.child_at_mut(hoglin_head_part_index(self.baby));
        head.pose = head_look_yaw_pose(head.pose, render_state.head_yaw);
        let right_ear = head.child_at_mut(HOGLIN_RIGHT_EAR_CHILD_INDEX);
        right_ear.pose = hoglin_ear_sway_pose(right_ear.pose, false, limb_swing, limb_swing_amount);
        let left_ear = head.child_at_mut(HOGLIN_LEFT_EAR_CHILD_INDEX);
        left_ear.pose = hoglin_ear_sway_pose(left_ear.pose, true, limb_swing, limb_swing_amount);
        for index in HOGLIN_LEG_PART_INDICES {
            let leg = self.root.child_at_mut(index);
            leg.pose = hoglin_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
        }
    }
}
