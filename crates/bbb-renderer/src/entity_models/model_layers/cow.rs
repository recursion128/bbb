use super::{ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc};

pub(in crate::entity_models) const COW_BROWN: [f32; 4] = [0.38, 0.25, 0.18, 1.0];
pub(in crate::entity_models) const COW_COLD_FUR: [f32; 4] = [0.70, 0.66, 0.58, 1.0];

pub(in crate::entity_models) const ADULT_COW_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-4.0, -4.0, -6.0],
        size: [8.0, 8.0, 6.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [-3.0, 1.0, -7.0],
        size: [6.0, 3.0, 1.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [-5.0, -5.0, -5.0],
        size: [1.0, 3.0, 1.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [4.0, -5.0, -5.0],
        size: [1.0, 3.0, 1.0],
        color: COW_BROWN,
    },
];

pub(in crate::entity_models) const WARM_COW_HEAD: [ModelCubeDesc; 6] = [
    ModelCubeDesc {
        min: [-4.0, -4.0, -6.0],
        size: [8.0, 8.0, 6.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [-3.0, 1.0, -7.0],
        size: [6.0, 3.0, 1.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [-8.0, -3.0, -5.0],
        size: [4.0, 2.0, 2.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [-8.0, -5.0, -5.0],
        size: [2.0, 2.0, 2.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [4.0, -3.0, -5.0],
        size: [4.0, 2.0, 2.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [6.0, -5.0, -5.0],
        size: [2.0, 2.0, 2.0],
        color: COW_BROWN,
    },
];

pub(in crate::entity_models) const COLD_COW_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, -4.0, -6.0],
        size: [8.0, 8.0, 6.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [-3.0, 1.0, -7.0],
        size: [6.0, 3.0, 1.0],
        color: COW_BROWN,
    },
];

pub(in crate::entity_models) const COLD_COW_RIGHT_HORN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -4.5, -0.5],
    size: [2.0, 6.0, 2.0],
    color: COW_COLD_FUR,
}];

pub(in crate::entity_models) const COLD_COW_LEFT_HORN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -3.0, -0.5],
    size: [2.0, 6.0, 2.0],
    color: COW_COLD_FUR,
}];

pub(in crate::entity_models) const COLD_COW_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.5, -2.5, -3.5],
            rotation: [1.5708, 0.0, 0.0],
        },
        cubes: &COLD_COW_RIGHT_HORN,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.5, -2.5, -5.0],
            rotation: [1.5708, 0.0, 0.0],
        },
        cubes: &COLD_COW_LEFT_HORN,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_COW_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-6.0, -10.0, -7.0],
        size: [12.0, 18.0, 10.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [-2.0, 2.0, -8.0],
        size: [4.0, 6.0, 1.0],
        color: COW_BROWN,
    },
];

pub(in crate::entity_models) const COLD_COW_BODY: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-6.5, -10.5, -7.5],
        size: [13.0, 19.0, 11.0],
        color: COW_COLD_FUR,
    },
    ModelCubeDesc {
        min: [-6.0, -10.0, -7.0],
        size: [12.0, 18.0, 10.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [-2.0, 2.0, -8.0],
        size: [4.0, 6.0, 1.0],
        color: COW_BROWN,
    },
];

pub(in crate::entity_models) const ADULT_COW_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: COW_BROWN,
}];

// Vanilla 26.1 CowModel.createBodyLayer().
pub(in crate::entity_models) const ADULT_COW_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, -8.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 5.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_COW_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 12.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 12.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 12.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 12.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_LEG,
        children: &[],
    },
];

// Vanilla 26.1 WarmCowModel.createBodyLayer().
pub(in crate::entity_models) const WARM_COW_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, -8.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &WARM_COW_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 5.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_COW_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 12.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 12.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 12.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 12.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_LEG,
        children: &[],
    },
];

// Vanilla 26.1 ColdCowModel.createBodyLayer().
pub(in crate::entity_models) const COLD_COW_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, -8.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &COLD_COW_HEAD,
        children: &COLD_COW_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 5.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &COLD_COW_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 12.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 12.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 12.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 12.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_COW_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-3.0, -4.569, -4.8333],
        size: [6.0, 6.0, 5.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [3.0, -5.569, -3.8333],
        size: [1.0, 2.0, 1.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [-4.0, -5.569, -3.8333],
        size: [1.0, 2.0, 1.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [-2.0, -1.569, -5.8333],
        size: [4.0, 3.0, 1.0],
        color: COW_BROWN,
    },
];

pub(in crate::entity_models) const BABY_COW_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-7.0, -7.0, -1.0],
    size: [8.0, 6.0, 12.0],
    color: COW_BROWN,
}];

pub(in crate::entity_models) const BABY_COW_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, -1.5],
    size: [3.0, 6.0, 3.0],
    color: COW_BROWN,
}];

// Vanilla 26.1 BabyCowModel.createBodyLayer().
pub(in crate::entity_models) const BABY_COW_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 13.569, -5.1667],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_COW_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 19.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_COW_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 18.0, -3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 18.0, -3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 18.0, 3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 18.0, 3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_COW_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const MODEL_LAYER_COW: &str = "minecraft:cow#main";
pub(in crate::entity_models) const MODEL_LAYER_COW_BABY: &str = "minecraft:cow_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_WARM_COW: &str = "minecraft:warm_cow#main";
pub(in crate::entity_models) const MODEL_LAYER_WARM_COW_BABY: &str = "minecraft:warm_cow_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_COLD_COW: &str = "minecraft:cold_cow#main";
pub(in crate::entity_models) const MODEL_LAYER_COLD_COW_BABY: &str = "minecraft:cold_cow_baby#main";

pub(in crate::entity_models) const ADULT_COW_TEXTURED_HEAD: [TexturedModelCubeDesc; 4] = [
    TexturedModelCubeDesc {
        min: [-4.0, -4.0, -6.0],
        size: [8.0, 8.0, 6.0],
        uv_size: [8.0, 8.0, 6.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-3.0, 1.0, -7.0],
        size: [6.0, 3.0, 1.0],
        uv_size: [6.0, 3.0, 1.0],
        tex: [1.0, 33.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-5.0, -5.0, -5.0],
        size: [1.0, 3.0, 1.0],
        uv_size: [1.0, 3.0, 1.0],
        tex: [22.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [4.0, -5.0, -5.0],
        size: [1.0, 3.0, 1.0],
        uv_size: [1.0, 3.0, 1.0],
        tex: [22.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const WARM_COW_TEXTURED_HEAD: [TexturedModelCubeDesc; 6] = [
    TexturedModelCubeDesc {
        min: [-4.0, -4.0, -6.0],
        size: [8.0, 8.0, 6.0],
        uv_size: [8.0, 8.0, 6.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-3.0, 1.0, -7.0],
        size: [6.0, 3.0, 1.0],
        uv_size: [6.0, 3.0, 1.0],
        tex: [1.0, 33.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-8.0, -3.0, -5.0],
        size: [4.0, 2.0, 2.0],
        uv_size: [4.0, 2.0, 2.0],
        tex: [27.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-8.0, -5.0, -5.0],
        size: [2.0, 2.0, 2.0],
        uv_size: [2.0, 2.0, 2.0],
        tex: [39.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [4.0, -3.0, -5.0],
        size: [4.0, 2.0, 2.0],
        uv_size: [4.0, 2.0, 2.0],
        tex: [27.0, 0.0],
        mirror: true,
    },
    TexturedModelCubeDesc {
        min: [6.0, -5.0, -5.0],
        size: [2.0, 2.0, 2.0],
        uv_size: [2.0, 2.0, 2.0],
        tex: [39.0, 0.0],
        mirror: true,
    },
];

pub(in crate::entity_models) const COLD_COW_TEXTURED_HEAD: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-4.0, -4.0, -6.0],
        size: [8.0, 8.0, 6.0],
        uv_size: [8.0, 8.0, 6.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-3.0, 1.0, -7.0],
        size: [6.0, 3.0, 1.0],
        uv_size: [6.0, 3.0, 1.0],
        tex: [9.0, 33.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const COLD_COW_TEXTURED_RIGHT_HORN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, -4.5, -0.5],
        size: [2.0, 6.0, 2.0],
        uv_size: [2.0, 6.0, 2.0],
        tex: [0.0, 40.0],
        mirror: false,
    }];

pub(in crate::entity_models) const COLD_COW_TEXTURED_LEFT_HORN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, -3.0, -0.5],
        size: [2.0, 6.0, 2.0],
        uv_size: [2.0, 6.0, 2.0],
        tex: [0.0, 32.0],
        mirror: false,
    }];

pub(in crate::entity_models) const COLD_COW_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: COLD_COW_HEAD_CHILDREN[0].pose,
        cubes: &COLD_COW_TEXTURED_RIGHT_HORN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: COLD_COW_HEAD_CHILDREN[1].pose,
        cubes: &COLD_COW_TEXTURED_LEFT_HORN,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_COW_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-6.0, -10.0, -7.0],
        size: [12.0, 18.0, 10.0],
        uv_size: [12.0, 18.0, 10.0],
        tex: [18.0, 4.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-2.0, 2.0, -8.0],
        size: [4.0, 6.0, 1.0],
        uv_size: [4.0, 6.0, 1.0],
        tex: [52.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const COLD_COW_TEXTURED_BODY: [TexturedModelCubeDesc; 3] = [
    TexturedModelCubeDesc {
        min: [-6.5, -10.5, -7.5],
        size: [13.0, 19.0, 11.0],
        uv_size: [12.0, 18.0, 10.0],
        tex: [20.0, 32.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-6.0, -10.0, -7.0],
        size: [12.0, 18.0, 10.0],
        uv_size: [12.0, 18.0, 10.0],
        tex: [18.0, 4.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-2.0, 2.0, -8.0],
        size: [4.0, 6.0, 1.0],
        uv_size: [4.0, 6.0, 1.0],
        tex: [52.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const ADULT_COW_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 12.0, 4.0],
        uv_size: [4.0, 12.0, 4.0],
        tex: [0.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_COW_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 12.0, 4.0],
        uv_size: [4.0, 12.0, 4.0],
        tex: [0.0, 16.0],
        mirror: true,
    }];

pub(in crate::entity_models) const ADULT_COW_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: ADULT_COW_PARTS[0].pose,
        cubes: &ADULT_COW_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_COW_PARTS[1].pose,
        cubes: &ADULT_COW_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_COW_PARTS[2].pose,
        cubes: &ADULT_COW_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_COW_PARTS[3].pose,
        cubes: &ADULT_COW_TEXTURED_LEFT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_COW_PARTS[4].pose,
        cubes: &ADULT_COW_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_COW_PARTS[5].pose,
        cubes: &ADULT_COW_TEXTURED_LEFT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const WARM_COW_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: WARM_COW_PARTS[0].pose,
        cubes: &WARM_COW_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: WARM_COW_PARTS[1].pose,
        cubes: &ADULT_COW_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: WARM_COW_PARTS[2].pose,
        cubes: &ADULT_COW_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: WARM_COW_PARTS[3].pose,
        cubes: &ADULT_COW_TEXTURED_LEFT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: WARM_COW_PARTS[4].pose,
        cubes: &ADULT_COW_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: WARM_COW_PARTS[5].pose,
        cubes: &ADULT_COW_TEXTURED_LEFT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const COLD_COW_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: COLD_COW_PARTS[0].pose,
        cubes: &COLD_COW_TEXTURED_HEAD,
        children: &COLD_COW_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: COLD_COW_PARTS[1].pose,
        cubes: &COLD_COW_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: COLD_COW_PARTS[2].pose,
        cubes: &ADULT_COW_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: COLD_COW_PARTS[3].pose,
        cubes: &ADULT_COW_TEXTURED_LEFT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: COLD_COW_PARTS[4].pose,
        cubes: &ADULT_COW_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: COLD_COW_PARTS[5].pose,
        cubes: &ADULT_COW_TEXTURED_LEFT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_COW_TEXTURED_HEAD: [TexturedModelCubeDesc; 4] = [
    TexturedModelCubeDesc {
        min: [-3.0, -4.569, -4.8333],
        size: [6.0, 6.0, 5.0],
        uv_size: [6.0, 6.0, 5.0],
        tex: [0.0, 18.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [3.0, -5.569, -3.8333],
        size: [1.0, 2.0, 1.0],
        uv_size: [1.0, 2.0, 1.0],
        tex: [8.0, 29.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-4.0, -5.569, -3.8333],
        size: [1.0, 2.0, 1.0],
        uv_size: [1.0, 2.0, 1.0],
        tex: [4.0, 29.0],
        mirror: true,
    },
    TexturedModelCubeDesc {
        min: [-2.0, -1.569, -5.8333],
        size: [4.0, 3.0, 1.0],
        uv_size: [4.0, 3.0, 1.0],
        tex: [12.0, 29.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BABY_COW_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-7.0, -7.0, -1.0],
        size: [8.0, 6.0, 12.0],
        uv_size: [8.0, 6.0, 12.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_COW_TEXTURED_RIGHT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, 0.0, -1.5],
        size: [3.0, 6.0, 3.0],
        uv_size: [3.0, 6.0, 3.0],
        tex: [22.0, 18.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_COW_TEXTURED_LEFT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, 0.0, -1.5],
        size: [3.0, 6.0, 3.0],
        uv_size: [3.0, 6.0, 3.0],
        tex: [34.0, 18.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_COW_TEXTURED_RIGHT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, 0.0, -1.5],
        size: [3.0, 6.0, 3.0],
        uv_size: [3.0, 6.0, 3.0],
        tex: [22.0, 27.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_COW_TEXTURED_LEFT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, 0.0, -1.5],
        size: [3.0, 6.0, 3.0],
        uv_size: [3.0, 6.0, 3.0],
        tex: [34.0, 27.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_COW_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: BABY_COW_PARTS[0].pose,
        cubes: &BABY_COW_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_COW_PARTS[1].pose,
        cubes: &BABY_COW_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_COW_PARTS[2].pose,
        cubes: &BABY_COW_TEXTURED_RIGHT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_COW_PARTS[3].pose,
        cubes: &BABY_COW_TEXTURED_LEFT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_COW_PARTS[4].pose,
        cubes: &BABY_COW_TEXTURED_RIGHT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_COW_PARTS[5].pose,
        cubes: &BABY_COW_TEXTURED_LEFT_HIND_LEG,
        children: &[],
    },
];
