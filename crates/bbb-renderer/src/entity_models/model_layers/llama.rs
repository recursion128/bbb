use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    LLAMA_CREAMY,
};

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

// Vanilla 26.1 `ModelLayers.LLAMA` / `LLAMA_BABY` (`LlamaRenderer`). The trader
// llama bakes the same `LlamaModel.createBodyLayer` mesh under
// `ModelLayers.TRADER_LLAMA[_BABY]`; the only difference is the `LlamaDecorLayer`
// trader overlay, a deferred equipment layer, so the textured base reuses these.
pub(in crate::entity_models) const MODEL_LAYER_LLAMA: &str = "minecraft:llama#main";
pub(in crate::entity_models) const MODEL_LAYER_LLAMA_BABY: &str = "minecraft:llama_baby#main";

// `LlamaModel.createBodyLayer` UVs, atlas 128×64. `CubeDeformation.NONE`, so every
// `uv_size` equals the geometry size. The two ears share `texOffs(17, 0)` and neither
// is mirrored (both vanilla `addBox("ear", …)` calls use the same offset).
pub(in crate::entity_models) const ADULT_LLAMA_TEXTURED_HEAD: [TexturedModelCubeDesc; 4] = [
    TexturedModelCubeDesc {
        min: [-2.0, -14.0, -10.0],
        size: [4.0, 4.0, 9.0],
        uv_size: [4.0, 4.0, 9.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-4.0, -16.0, -6.0],
        size: [8.0, 18.0, 6.0],
        uv_size: [8.0, 18.0, 6.0],
        tex: [0.0, 14.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-4.0, -19.0, -4.0],
        size: [3.0, 3.0, 2.0],
        uv_size: [3.0, 3.0, 2.0],
        tex: [17.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [1.0, -19.0, -4.0],
        size: [3.0, 3.0, 2.0],
        uv_size: [3.0, 3.0, 2.0],
        tex: [17.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const ADULT_LLAMA_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-6.0, -10.0, -7.0],
        size: [12.0, 18.0, 10.0],
        uv_size: [12.0, 18.0, 10.0],
        tex: [29.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_LLAMA_TEXTURED_RIGHT_CHEST: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, 0.0, 0.0],
        size: [8.0, 8.0, 3.0],
        uv_size: [8.0, 8.0, 3.0],
        tex: [45.0, 28.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_LLAMA_TEXTURED_LEFT_CHEST: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, 0.0, 0.0],
        size: [8.0, 8.0, 3.0],
        uv_size: [8.0, 8.0, 3.0],
        tex: [45.0, 41.0],
        mirror: false,
    }];

// All four adult legs share one `CubeListBuilder` (`texOffs(29, 29)`, no mirror).
pub(in crate::entity_models) const ADULT_LLAMA_TEXTURED_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 14.0, 4.0],
        uv_size: [4.0, 14.0, 4.0],
        tex: [29.0, 29.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_LLAMA_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: ADULT_LLAMA_PARTS[0].pose,
        cubes: &ADULT_LLAMA_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_LLAMA_PARTS[1].pose,
        cubes: &ADULT_LLAMA_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_LLAMA_PARTS[2].pose,
        cubes: &ADULT_LLAMA_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_LLAMA_PARTS[3].pose,
        cubes: &ADULT_LLAMA_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_LLAMA_PARTS[4].pose,
        cubes: &ADULT_LLAMA_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_LLAMA_PARTS[5].pose,
        cubes: &ADULT_LLAMA_TEXTURED_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_LLAMA_TEXTURED_PARTS_WITH_CHEST: [TexturedModelPartDesc;
    8] = [
    ADULT_LLAMA_TEXTURED_PARTS[0],
    ADULT_LLAMA_TEXTURED_PARTS[1],
    TexturedModelPartDesc {
        pose: ADULT_LLAMA_RIGHT_CHEST_PART.pose,
        cubes: &ADULT_LLAMA_TEXTURED_RIGHT_CHEST,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_LLAMA_LEFT_CHEST_PART.pose,
        cubes: &ADULT_LLAMA_TEXTURED_LEFT_CHEST,
        children: &[],
    },
    ADULT_LLAMA_TEXTURED_PARTS[2],
    ADULT_LLAMA_TEXTURED_PARTS[3],
    ADULT_LLAMA_TEXTURED_PARTS[4],
    ADULT_LLAMA_TEXTURED_PARTS[5],
];

// `BabyLlamaModel.createBodyLayer` UVs, atlas 64×64. Each leg has its own `texOffs`
// (right/left, hind/front), unlike the adult layer's single shared leg.
pub(in crate::entity_models) const BABY_LLAMA_TEXTURED_HEAD: [TexturedModelCubeDesc; 4] = [
    TexturedModelCubeDesc {
        min: [-3.0, -9.0, -4.0],
        size: [6.0, 11.0, 4.0],
        uv_size: [6.0, 11.0, 4.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-1.5, -7.0, -7.0],
        size: [3.0, 3.0, 3.0],
        uv_size: [3.0, 3.0, 3.0],
        tex: [0.0, 15.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [0.5, -11.0, -3.0],
        size: [2.0, 2.0, 2.0],
        uv_size: [2.0, 2.0, 2.0],
        tex: [20.0, 4.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-2.5, -11.0, -3.0],
        size: [2.0, 2.0, 2.0],
        uv_size: [2.0, 2.0, 2.0],
        tex: [20.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BABY_LLAMA_TEXTURED_RIGHT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.4, -0.5, -1.5],
        size: [3.0, 8.0, 3.0],
        uv_size: [3.0, 8.0, 3.0],
        tex: [0.0, 45.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_LLAMA_TEXTURED_LEFT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.6, -0.5, -1.5],
        size: [3.0, 8.0, 3.0],
        uv_size: [3.0, 8.0, 3.0],
        tex: [12.0, 45.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_LLAMA_TEXTURED_RIGHT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.4, -0.5, -1.5],
        size: [3.0, 8.0, 3.0],
        uv_size: [3.0, 8.0, 3.0],
        tex: [0.0, 34.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_LLAMA_TEXTURED_LEFT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.6, -0.5, -1.5],
        size: [3.0, 8.0, 3.0],
        uv_size: [3.0, 8.0, 3.0],
        tex: [12.0, 34.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_LLAMA_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -3.0, -8.5],
        size: [8.0, 6.0, 13.0],
        uv_size: [8.0, 6.0, 13.0],
        tex: [0.0, 15.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_LLAMA_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: BABY_LLAMA_PARTS[0].pose,
        cubes: &BABY_LLAMA_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_LLAMA_PARTS[1].pose,
        cubes: &BABY_LLAMA_TEXTURED_RIGHT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_LLAMA_PARTS[2].pose,
        cubes: &BABY_LLAMA_TEXTURED_LEFT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_LLAMA_PARTS[3].pose,
        cubes: &BABY_LLAMA_TEXTURED_RIGHT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_LLAMA_PARTS[4].pose,
        cubes: &BABY_LLAMA_TEXTURED_LEFT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_LLAMA_PARTS[5].pose,
        cubes: &BABY_LLAMA_TEXTURED_BODY,
        children: &[],
    },
];
