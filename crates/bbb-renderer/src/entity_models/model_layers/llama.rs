use super::{ModelCubeDesc, ModelPartDesc, PartPose, LLAMA_CREAMY};

pub(in crate::entity_models) const ADULT_LLAMA_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-2.0, -14.0, -10.0],
        size: [4.0, 4.0, 9.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [-4.0, -16.0, -6.0],
        size: [8.0, 18.0, 6.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [-4.0, -19.0, -4.0],
        size: [3.0, 3.0, 2.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [1.0, -19.0, -4.0],
        size: [3.0, 3.0, 2.0],
        color: LLAMA_CREAMY,
    },
];

pub(in crate::entity_models) const ADULT_LLAMA_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-6.0, -10.0, -7.0],
    size: [12.0, 18.0, 10.0],
    color: LLAMA_CREAMY,
}];

pub(in crate::entity_models) const LLAMA_CHEST: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 0.0, 0.0],
    size: [8.0, 8.0, 3.0],
    color: LLAMA_CREAMY,
}];

pub(in crate::entity_models) const ADULT_LLAMA_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 14.0, 4.0],
    color: LLAMA_CREAMY,
}];

pub(in crate::entity_models) const ADULT_LLAMA_RIGHT_CHEST_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [-8.5, 3.0, 3.0],
        rotation: [0.0, std::f32::consts::FRAC_PI_2, 0.0],
    },
    cubes: &LLAMA_CHEST,
    children: &[],
};

pub(in crate::entity_models) const ADULT_LLAMA_LEFT_CHEST_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [5.5, 3.0, 3.0],
        rotation: [0.0, std::f32::consts::FRAC_PI_2, 0.0],
    },
    cubes: &LLAMA_CHEST,
    children: &[],
};

// Vanilla 26.1 ModelLayers.LLAMA / TRADER_LLAMA:
// LlamaModel.createBodyLayer(CubeDeformation.NONE). Chest parts are only visible
// when LlamaRenderState.hasChest is true.
pub(in crate::entity_models) const ADULT_LLAMA_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 7.0, -6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 5.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.5, 10.0, 6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.5, 10.0, 6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.5, 10.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.5, 10.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_LLAMA_PARTS_WITH_CHEST: [ModelPartDesc; 8] = [
    ADULT_LLAMA_PARTS[0],
    ADULT_LLAMA_PARTS[1],
    ADULT_LLAMA_RIGHT_CHEST_PART,
    ADULT_LLAMA_LEFT_CHEST_PART,
    ADULT_LLAMA_PARTS[2],
    ADULT_LLAMA_PARTS[3],
    ADULT_LLAMA_PARTS[4],
    ADULT_LLAMA_PARTS[5],
];

pub(in crate::entity_models) const BABY_LLAMA_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-3.0, -9.0, -4.0],
        size: [6.0, 11.0, 4.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [-1.5, -7.0, -7.0],
        size: [3.0, 3.0, 3.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [0.5, -11.0, -3.0],
        size: [2.0, 2.0, 2.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [-2.5, -11.0, -3.0],
        size: [2.0, 2.0, 2.0],
        color: LLAMA_CREAMY,
    },
];

pub(in crate::entity_models) const BABY_LLAMA_RIGHT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.4, -0.5, -1.5],
    size: [3.0, 8.0, 3.0],
    color: LLAMA_CREAMY,
}];

pub(in crate::entity_models) const BABY_LLAMA_LEFT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.6, -0.5, -1.5],
    size: [3.0, 8.0, 3.0],
    color: LLAMA_CREAMY,
}];

pub(in crate::entity_models) const BABY_LLAMA_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -3.0, -8.5],
    size: [8.0, 6.0, 13.0],
    color: LLAMA_CREAMY,
}];

// Vanilla 26.1 ModelLayers.LLAMA_BABY / TRADER_LLAMA_BABY:
// BabyLlamaModel.createBodyLayer(CubeDeformation.NONE). The layer includes
// chest parts, but LlamaRenderer sets hasChest=false for babies.
pub(in crate::entity_models) const BABY_LLAMA_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 16.5, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 16.5, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 16.5, -3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 16.5, -3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 14.0, 2.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_BODY,
        children: &[],
    },
];
