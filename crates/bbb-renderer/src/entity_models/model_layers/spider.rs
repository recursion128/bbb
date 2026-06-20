use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    SPIDER_DARK,
};

pub(in crate::entity_models) const MODEL_LAYER_SPIDER: &str = "minecraft:spider#main";
pub(in crate::entity_models) const MODEL_LAYER_CAVE_SPIDER: &str = "minecraft:cave_spider#main";

pub(in crate::entity_models) const SPIDER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -4.0, -8.0],
    size: [8.0, 8.0, 8.0],
    color: SPIDER_DARK,
}];

pub(in crate::entity_models) const SPIDER_BODY_0: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -3.0, -3.0],
    size: [6.0, 6.0, 6.0],
    color: SPIDER_DARK,
}];

pub(in crate::entity_models) const SPIDER_BODY_1: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.0, -4.0, -6.0],
    size: [10.0, 8.0, 12.0],
    color: SPIDER_DARK,
}];

pub(in crate::entity_models) const SPIDER_RIGHT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-15.0, -1.0, -1.0],
    size: [16.0, 2.0, 2.0],
    color: SPIDER_DARK,
}];

pub(in crate::entity_models) const SPIDER_LEFT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -1.0],
    size: [16.0, 2.0, 2.0],
    color: SPIDER_DARK,
}];

// Vanilla 26.1 SpiderModel.createSpiderBodyLayer().
pub(in crate::entity_models) const SPIDER_PARTS: [ModelPartDesc; 11] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.0, -3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SPIDER_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SPIDER_BODY_0,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.0, 9.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SPIDER_BODY_1,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 15.0, 2.0],
            rotation: [
                0.0,
                std::f32::consts::FRAC_PI_4,
                -std::f32::consts::FRAC_PI_4,
            ],
        },
        cubes: &SPIDER_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 15.0, 2.0],
            rotation: [
                0.0,
                -std::f32::consts::FRAC_PI_4,
                std::f32::consts::FRAC_PI_4,
            ],
        },
        cubes: &SPIDER_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 15.0, 1.0],
            rotation: [0.0, std::f32::consts::FRAC_PI_8, -0.58119464],
        },
        cubes: &SPIDER_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 15.0, 1.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_8, 0.58119464],
        },
        cubes: &SPIDER_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 15.0, 0.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_8, -0.58119464],
        },
        cubes: &SPIDER_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 15.0, 0.0],
            rotation: [0.0, std::f32::consts::FRAC_PI_8, 0.58119464],
        },
        cubes: &SPIDER_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 15.0, -1.0],
            rotation: [
                0.0,
                -std::f32::consts::FRAC_PI_4,
                -std::f32::consts::FRAC_PI_4,
            ],
        },
        cubes: &SPIDER_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 15.0, -1.0],
            rotation: [
                0.0,
                std::f32::consts::FRAC_PI_4,
                std::f32::consts::FRAC_PI_4,
            ],
        },
        cubes: &SPIDER_LEFT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const SPIDER_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -4.0, -8.0],
        size: [8.0, 8.0, 8.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [32.0, 4.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SPIDER_TEXTURED_BODY_0: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -3.0, -3.0],
        size: [6.0, 6.0, 6.0],
        uv_size: [6.0, 6.0, 6.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SPIDER_TEXTURED_BODY_1: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-5.0, -4.0, -6.0],
        size: [10.0, 8.0, 12.0],
        uv_size: [10.0, 8.0, 12.0],
        tex: [0.0, 12.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SPIDER_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-15.0, -1.0, -1.0],
        size: [16.0, 2.0, 2.0],
        uv_size: [16.0, 2.0, 2.0],
        tex: [18.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SPIDER_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -1.0, -1.0],
        size: [16.0, 2.0, 2.0],
        uv_size: [16.0, 2.0, 2.0],
        tex: [18.0, 0.0],
        mirror: true,
    }];

pub(in crate::entity_models) const SPIDER_TEXTURED_PARTS: [TexturedModelPartDesc; 11] = [
    TexturedModelPartDesc {
        pose: SPIDER_PARTS[0].pose,
        cubes: &SPIDER_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SPIDER_PARTS[1].pose,
        cubes: &SPIDER_TEXTURED_BODY_0,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SPIDER_PARTS[2].pose,
        cubes: &SPIDER_TEXTURED_BODY_1,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SPIDER_PARTS[3].pose,
        cubes: &SPIDER_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SPIDER_PARTS[4].pose,
        cubes: &SPIDER_TEXTURED_LEFT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SPIDER_PARTS[5].pose,
        cubes: &SPIDER_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SPIDER_PARTS[6].pose,
        cubes: &SPIDER_TEXTURED_LEFT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SPIDER_PARTS[7].pose,
        cubes: &SPIDER_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SPIDER_PARTS[8].pose,
        cubes: &SPIDER_TEXTURED_LEFT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SPIDER_PARTS[9].pose,
        cubes: &SPIDER_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SPIDER_PARTS[10].pose,
        cubes: &SPIDER_TEXTURED_LEFT_LEG,
        children: &[],
    },
];
