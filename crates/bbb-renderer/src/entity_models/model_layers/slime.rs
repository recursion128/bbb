use super::{
    ModelCubeDesc, ModelPartDesc, TexturedModelCubeDesc, TexturedModelPartDesc, PART_POSE_ZERO,
};

pub(in crate::entity_models) const SLIME_GREEN: [f32; 4] = [0.42, 0.82, 0.30, 1.0];
pub(in crate::entity_models) const SLIME_FEATURE_DARK: [f32; 4] = [0.16, 0.28, 0.10, 1.0];
pub(in crate::entity_models) const MAGMA_CUBE_ORANGE: [f32; 4] = [0.92, 0.38, 0.12, 1.0];
pub(in crate::entity_models) const MAGMA_CUBE_CORE: [f32; 4] = [0.98, 0.72, 0.22, 1.0];

pub(in crate::entity_models) const MODEL_LAYER_SLIME: &str = "minecraft:slime#main";
pub(in crate::entity_models) const MODEL_LAYER_SLIME_OUTER: &str = "minecraft:slime#outer";
pub(in crate::entity_models) const MODEL_LAYER_MAGMA_CUBE: &str = "minecraft:magma_cube#main";

pub(in crate::entity_models) const SLIME_INNER_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 17.0, -3.0],
    size: [6.0, 6.0, 6.0],
    color: SLIME_GREEN,
}];

pub(in crate::entity_models) const SLIME_RIGHT_EYE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.25, 18.0, -3.5],
    size: [2.0, 2.0, 2.0],
    color: SLIME_FEATURE_DARK,
}];

pub(in crate::entity_models) const SLIME_LEFT_EYE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [1.25, 18.0, -3.5],
    size: [2.0, 2.0, 2.0],
    color: SLIME_FEATURE_DARK,
}];

pub(in crate::entity_models) const SLIME_MOUTH: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 21.0, -3.5],
    size: [1.0, 1.0, 1.0],
    color: SLIME_FEATURE_DARK,
}];

pub(in crate::entity_models) const SLIME_OUTER_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 16.0, -4.0],
    size: [8.0, 8.0, 8.0],
    color: SLIME_GREEN,
}];

// Vanilla 26.1 ModelLayers.SLIME plus ModelLayers.SLIME_OUTER.
pub(in crate::entity_models) const SLIME_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SLIME_INNER_CUBE,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SLIME_RIGHT_EYE,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SLIME_LEFT_EYE,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SLIME_MOUTH,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SLIME_OUTER_CUBE,
        children: &[],
    },
];

pub(in crate::entity_models) const SLIME_INNER_TEXTURED_CUBE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, 17.0, -3.0],
        size: [6.0, 6.0, 6.0],
        uv_size: [6.0, 6.0, 6.0],
        tex: [0.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SLIME_RIGHT_EYE_TEXTURED_CUBE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.25, 18.0, -3.5],
        size: [2.0, 2.0, 2.0],
        uv_size: [2.0, 2.0, 2.0],
        tex: [32.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SLIME_LEFT_EYE_TEXTURED_CUBE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [1.25, 18.0, -3.5],
        size: [2.0, 2.0, 2.0],
        uv_size: [2.0, 2.0, 2.0],
        tex: [32.0, 4.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SLIME_MOUTH_TEXTURED_CUBE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 21.0, -3.5],
        size: [1.0, 1.0, 1.0],
        uv_size: [1.0, 1.0, 1.0],
        tex: [32.0, 8.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SLIME_OUTER_TEXTURED_CUBE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 16.0, -4.0],
        size: [8.0, 8.0, 8.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

// Vanilla 26.1 ModelLayers.SLIME: SlimeModel.createInnerBodyLayer().
pub(in crate::entity_models) const SLIME_INNER_TEXTURED_PARTS: [TexturedModelPartDesc; 4] = [
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SLIME_INNER_TEXTURED_CUBE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SLIME_RIGHT_EYE_TEXTURED_CUBE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SLIME_LEFT_EYE_TEXTURED_CUBE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SLIME_MOUTH_TEXTURED_CUBE,
        children: &[],
    },
];

// Vanilla 26.1 ModelLayers.SLIME_OUTER: SlimeModel.createOuterBodyLayer().
pub(in crate::entity_models) const SLIME_OUTER_TEXTURED_PARTS: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SLIME_OUTER_TEXTURED_CUBE,
        children: &[],
    }];

pub(in crate::entity_models) const MAGMA_CUBE_SEGMENT_0: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 16.0, -4.0],
    size: [8.0, 1.0, 8.0],
    color: MAGMA_CUBE_ORANGE,
}];

pub(in crate::entity_models) const MAGMA_CUBE_SEGMENT_1: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 17.0, -4.0],
    size: [8.0, 1.0, 8.0],
    color: MAGMA_CUBE_ORANGE,
}];

pub(in crate::entity_models) const MAGMA_CUBE_SEGMENT_2: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 18.0, -4.0],
    size: [8.0, 1.0, 8.0],
    color: MAGMA_CUBE_ORANGE,
}];

pub(in crate::entity_models) const MAGMA_CUBE_SEGMENT_3: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 19.0, -4.0],
    size: [8.0, 1.0, 8.0],
    color: MAGMA_CUBE_ORANGE,
}];

pub(in crate::entity_models) const MAGMA_CUBE_SEGMENT_4: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 20.0, -4.0],
    size: [8.0, 1.0, 8.0],
    color: MAGMA_CUBE_ORANGE,
}];

pub(in crate::entity_models) const MAGMA_CUBE_SEGMENT_5: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 21.0, -4.0],
    size: [8.0, 1.0, 8.0],
    color: MAGMA_CUBE_ORANGE,
}];

pub(in crate::entity_models) const MAGMA_CUBE_SEGMENT_6: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 22.0, -4.0],
    size: [8.0, 1.0, 8.0],
    color: MAGMA_CUBE_ORANGE,
}];

pub(in crate::entity_models) const MAGMA_CUBE_SEGMENT_7: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 23.0, -4.0],
    size: [8.0, 1.0, 8.0],
    color: MAGMA_CUBE_ORANGE,
}];

pub(in crate::entity_models) const MAGMA_CUBE_INSIDE_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 18.0, -2.0],
    size: [4.0, 4.0, 4.0],
    color: MAGMA_CUBE_CORE,
}];

// Vanilla 26.1 MagmaCubeModel.createBodyLayer().
pub(in crate::entity_models) const MAGMA_CUBE_PARTS: [ModelPartDesc; 9] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_SEGMENT_0,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_SEGMENT_1,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_SEGMENT_2,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_SEGMENT_3,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_SEGMENT_4,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_SEGMENT_5,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_SEGMENT_6,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_SEGMENT_7,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_INSIDE_CUBE,
        children: &[],
    },
];

pub(in crate::entity_models) const MAGMA_CUBE_TEXTURED_SEGMENT_0: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 16.0, -4.0],
        size: [8.0, 1.0, 8.0],
        uv_size: [8.0, 1.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const MAGMA_CUBE_TEXTURED_SEGMENT_1: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 17.0, -4.0],
        size: [8.0, 1.0, 8.0],
        uv_size: [8.0, 1.0, 8.0],
        tex: [0.0, 9.0],
        mirror: false,
    }];

pub(in crate::entity_models) const MAGMA_CUBE_TEXTURED_SEGMENT_2: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 18.0, -4.0],
        size: [8.0, 1.0, 8.0],
        uv_size: [8.0, 1.0, 8.0],
        tex: [0.0, 18.0],
        mirror: false,
    }];

pub(in crate::entity_models) const MAGMA_CUBE_TEXTURED_SEGMENT_3: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 19.0, -4.0],
        size: [8.0, 1.0, 8.0],
        uv_size: [8.0, 1.0, 8.0],
        tex: [0.0, 27.0],
        mirror: false,
    }];

pub(in crate::entity_models) const MAGMA_CUBE_TEXTURED_SEGMENT_4: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 20.0, -4.0],
        size: [8.0, 1.0, 8.0],
        uv_size: [8.0, 1.0, 8.0],
        tex: [32.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const MAGMA_CUBE_TEXTURED_SEGMENT_5: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 21.0, -4.0],
        size: [8.0, 1.0, 8.0],
        uv_size: [8.0, 1.0, 8.0],
        tex: [32.0, 9.0],
        mirror: false,
    }];

pub(in crate::entity_models) const MAGMA_CUBE_TEXTURED_SEGMENT_6: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 22.0, -4.0],
        size: [8.0, 1.0, 8.0],
        uv_size: [8.0, 1.0, 8.0],
        tex: [32.0, 18.0],
        mirror: false,
    }];

pub(in crate::entity_models) const MAGMA_CUBE_TEXTURED_SEGMENT_7: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 23.0, -4.0],
        size: [8.0, 1.0, 8.0],
        uv_size: [8.0, 1.0, 8.0],
        tex: [32.0, 27.0],
        mirror: false,
    }];

pub(in crate::entity_models) const MAGMA_CUBE_INSIDE_TEXTURED_CUBE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 18.0, -2.0],
        size: [4.0, 4.0, 4.0],
        uv_size: [4.0, 4.0, 4.0],
        tex: [24.0, 40.0],
        mirror: false,
    }];

// Vanilla 26.1 ModelLayers.MAGMA_CUBE: MagmaCubeModel.createBodyLayer().
pub(in crate::entity_models) const MAGMA_CUBE_TEXTURED_PARTS: [TexturedModelPartDesc; 9] = [
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_TEXTURED_SEGMENT_0,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_TEXTURED_SEGMENT_1,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_TEXTURED_SEGMENT_2,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_TEXTURED_SEGMENT_3,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_TEXTURED_SEGMENT_4,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_TEXTURED_SEGMENT_5,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_TEXTURED_SEGMENT_6,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_TEXTURED_SEGMENT_7,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_INSIDE_TEXTURED_CUBE,
        children: &[],
    },
];
