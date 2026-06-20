use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    PIG_COLD_FUR, PIG_PINK,
};

pub(in crate::entity_models) const MODEL_LAYER_PIG: &str = "minecraft:pig#main";
pub(in crate::entity_models) const MODEL_LAYER_PIG_BABY: &str = "minecraft:pig_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_COLD_PIG: &str = "minecraft:cold_pig#main";

pub(in crate::entity_models) const ADULT_PIG_TEXTURED_HEAD: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-4.0, -4.0, -8.0],
        size: [8.0, 8.0, 8.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-2.0, 0.0, -9.0],
        size: [4.0, 3.0, 1.0],
        uv_size: [4.0, 3.0, 1.0],
        tex: [16.0, 16.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const ADULT_PIG_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-5.0, -10.0, -7.0],
        size: [10.0, 16.0, 8.0],
        uv_size: [10.0, 16.0, 8.0],
        tex: [28.0, 8.0],
        mirror: false,
    }];

pub(in crate::entity_models) const COLD_PIG_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-5.0, -10.0, -7.0],
        size: [10.0, 16.0, 8.0],
        uv_size: [10.0, 16.0, 8.0],
        tex: [28.0, 8.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-5.5, -10.5, -7.5],
        size: [11.0, 17.0, 9.0],
        uv_size: [10.0, 16.0, 8.0],
        tex: [28.0, 32.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const ADULT_PIG_TEXTURED_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 6.0, 4.0],
        uv_size: [4.0, 6.0, 4.0],
        tex: [0.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_PIG_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: ADULT_PIG_PARTS[0].pose,
        cubes: &ADULT_PIG_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_PIG_PARTS[1].pose,
        cubes: &ADULT_PIG_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_PIG_PARTS[2].pose,
        cubes: &ADULT_PIG_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_PIG_PARTS[3].pose,
        cubes: &ADULT_PIG_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_PIG_PARTS[4].pose,
        cubes: &ADULT_PIG_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_PIG_PARTS[5].pose,
        cubes: &ADULT_PIG_TEXTURED_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const COLD_PIG_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: COLD_PIG_PARTS[0].pose,
        cubes: &ADULT_PIG_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: COLD_PIG_PARTS[1].pose,
        cubes: &COLD_PIG_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: COLD_PIG_PARTS[2].pose,
        cubes: &ADULT_PIG_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: COLD_PIG_PARTS[3].pose,
        cubes: &ADULT_PIG_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: COLD_PIG_PARTS[4].pose,
        cubes: &ADULT_PIG_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: COLD_PIG_PARTS[5].pose,
        cubes: &ADULT_PIG_TEXTURED_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_PIG_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.5, -3.0, -4.5],
        size: [7.0, 6.0, 9.0],
        uv_size: [7.0, 6.0, 9.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_PIG_TEXTURED_HEAD: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-3.525, -5.025, -5.025],
        size: [7.05, 6.05, 6.05],
        uv_size: [7.0, 6.0, 6.0],
        tex: [0.0, 15.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-1.515, -1.99, -6.015],
        size: [3.03, 2.03, 1.03],
        uv_size: [3.0, 2.0, 1.0],
        tex: [6.0, 27.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BABY_PIG_TEXTURED_LEFT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 2.0, 2.0],
        uv_size: [2.0, 2.0, 2.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_PIG_TEXTURED_RIGHT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 2.0, 2.0],
        uv_size: [2.0, 2.0, 2.0],
        tex: [23.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_PIG_TEXTURED_LEFT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 2.0, 2.0],
        uv_size: [2.0, 2.0, 2.0],
        tex: [0.0, 4.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_PIG_TEXTURED_RIGHT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 2.0, 2.0],
        uv_size: [2.0, 2.0, 2.0],
        tex: [23.0, 4.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_PIG_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: BABY_PIG_PARTS[0].pose,
        cubes: &BABY_PIG_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_PIG_PARTS[1].pose,
        cubes: &BABY_PIG_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_PIG_PARTS[2].pose,
        cubes: &BABY_PIG_TEXTURED_LEFT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_PIG_PARTS[3].pose,
        cubes: &BABY_PIG_TEXTURED_RIGHT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_PIG_PARTS[4].pose,
        cubes: &BABY_PIG_TEXTURED_LEFT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_PIG_PARTS[5].pose,
        cubes: &BABY_PIG_TEXTURED_RIGHT_HIND_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_PIG_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, -4.0, -8.0],
        size: [8.0, 8.0, 8.0],
        color: PIG_PINK,
    },
    ModelCubeDesc {
        min: [-2.0, 0.0, -9.0],
        size: [4.0, 3.0, 1.0],
        color: PIG_PINK,
    },
];

pub(in crate::entity_models) const ADULT_PIG_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.0, -10.0, -7.0],
    size: [10.0, 16.0, 8.0],
    color: PIG_PINK,
}];

pub(in crate::entity_models) const COLD_PIG_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-5.0, -10.0, -7.0],
        size: [10.0, 16.0, 8.0],
        color: PIG_PINK,
    },
    ModelCubeDesc {
        min: [-5.5, -10.5, -7.5],
        size: [11.0, 17.0, 9.0],
        color: PIG_COLD_FUR,
    },
];

pub(in crate::entity_models) const ADULT_PIG_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 6.0, 4.0],
    color: PIG_PINK,
}];

// Vanilla 26.1 PigModel.createBodyLayer(CubeDeformation.NONE).
pub(in crate::entity_models) const ADULT_PIG_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, -6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIG_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 11.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_PIG_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 18.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIG_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 18.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIG_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 18.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIG_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 18.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIG_LEG,
        children: &[],
    },
];

// Vanilla 26.1 ColdPigModel.createBodyLayer(CubeDeformation.NONE).
pub(in crate::entity_models) const COLD_PIG_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, -6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIG_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 11.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &COLD_PIG_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 18.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIG_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 18.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIG_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 18.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIG_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 18.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIG_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_PIG_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -3.0, -4.5],
    size: [7.0, 6.0, 9.0],
    color: PIG_PINK,
}];

pub(in crate::entity_models) const BABY_PIG_HEAD: [ModelCubeDesc; 2] = [
    // BabyPigModel bakes CubeDeformation into ModelPart.Cube render bounds.
    ModelCubeDesc {
        min: [-3.525, -5.025, -5.025],
        size: [7.05, 6.05, 6.05],
        color: PIG_PINK,
    },
    ModelCubeDesc {
        min: [-1.515, -1.99, -6.015],
        size: [3.03, 2.03, 1.03],
        color: PIG_PINK,
    },
];

pub(in crate::entity_models) const BABY_PIG_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 2.0, 2.0],
    color: PIG_PINK,
}];

// Vanilla 26.1 BabyPigModel.createBodyLayer(CubeDeformation.NONE).
pub(in crate::entity_models) const BABY_PIG_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 19.0, 0.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIG_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 19.0, -2.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIG_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 22.0, -3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIG_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 22.0, -3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIG_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 22.0, 4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIG_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 22.0, 4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIG_LEG,
        children: &[],
    },
];
