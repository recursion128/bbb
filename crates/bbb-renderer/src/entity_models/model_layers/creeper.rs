use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    CREEPER_GREEN,
};

pub(in crate::entity_models) const MODEL_LAYER_CREEPER: &str = "minecraft:creeper#main";

pub(in crate::entity_models) const CREEPER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -4.0],
    size: [8.0, 8.0, 8.0],
    color: CREEPER_GREEN,
}];

pub(in crate::entity_models) const CREEPER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: CREEPER_GREEN,
}];

pub(in crate::entity_models) const CREEPER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 6.0, 4.0],
    color: CREEPER_GREEN,
}];

// Vanilla 26.1 CreeperModel.createBodyLayer(CubeDeformation.NONE).
pub(in crate::entity_models) const CREEPER_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 6.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 6.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 18.0, 4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 18.0, 4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 18.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 18.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const CREEPER_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -8.0, -4.0],
        size: [8.0, 8.0, 8.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const CREEPER_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 0.0, -2.0],
        size: [8.0, 12.0, 4.0],
        uv_size: [8.0, 12.0, 4.0],
        tex: [16.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const CREEPER_TEXTURED_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 6.0, 4.0],
        uv_size: [4.0, 6.0, 4.0],
        tex: [0.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const CREEPER_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: CREEPER_PARTS[0].pose,
        cubes: &CREEPER_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: CREEPER_PARTS[1].pose,
        cubes: &CREEPER_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: CREEPER_PARTS[2].pose,
        cubes: &CREEPER_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: CREEPER_PARTS[3].pose,
        cubes: &CREEPER_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: CREEPER_PARTS[4].pose,
        cubes: &CREEPER_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: CREEPER_PARTS[5].pose,
        cubes: &CREEPER_TEXTURED_LEG,
        children: &[],
    },
];
