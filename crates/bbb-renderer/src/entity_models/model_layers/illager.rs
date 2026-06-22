use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    ILLAGER_HAT_COLOR, ILLAGER_ROBE, PART_POSE_ZERO,
};

pub(in crate::entity_models) const MODEL_LAYER_EVOKER: &str = "minecraft:evoker#main";
pub(in crate::entity_models) const MODEL_LAYER_ILLUSIONER: &str = "minecraft:illusioner#main";
pub(in crate::entity_models) const MODEL_LAYER_PILLAGER: &str = "minecraft:pillager#main";
pub(in crate::entity_models) const MODEL_LAYER_VINDICATOR: &str = "minecraft:vindicator#main";

pub(in crate::entity_models) const ILLAGER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -10.0, -4.0],
    size: [8.0, 10.0, 8.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.45, -10.45, -4.45],
    size: [8.9, 12.9, 8.9],
    color: ILLAGER_HAT_COLOR,
}];

pub(in crate::entity_models) const ILLAGER_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -6.0],
    size: [2.0, 4.0, 2.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, 0.0, -3.0],
        size: [8.0, 12.0, 6.0],
        color: ILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-4.5, -0.5, -3.5],
        size: [9.0, 21.0, 7.0],
        color: ILLAGER_ROBE,
    },
];

pub(in crate::entity_models) const ILLAGER_CROSSED_ARMS: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-8.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: ILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-4.0, 2.0, -2.0],
        size: [8.0, 4.0, 4.0],
        color: ILLAGER_ROBE,
    },
];

pub(in crate::entity_models) const ILLAGER_LEFT_SHOULDER: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [4.0, -2.0, -2.0],
    size: [4.0, 8.0, 4.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_NOSE,
    children: &[],
}];

pub(in crate::entity_models) const ILLAGER_HEAD_WITH_HAT_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HAT,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_NOSE,
        children: &[],
    },
];

pub(in crate::entity_models) const ILLAGER_CROSSED_ARM_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_LEFT_SHOULDER,
        children: &[],
    }];

pub(in crate::entity_models) const ILLAGER_CROSSED_ARM_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [0.0, 3.0, -1.0],
        rotation: [-0.75, 0.0, 0.0],
    },
    cubes: &ILLAGER_CROSSED_ARMS,
    children: &ILLAGER_CROSSED_ARM_CHILDREN,
};

pub(in crate::entity_models) const ILLAGER_RIGHT_ARM_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [-5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_RIGHT_ARM,
    children: &[],
};

pub(in crate::entity_models) const ILLAGER_LEFT_ARM_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_LEFT_ARM,
    children: &[],
};

// Vanilla 26.1 IllagerModel.createBodyLayer(), with LayerDefinitions'
// MeshTransformer.scaling(0.9375F) applied by the emitter root transform.
pub(in crate::entity_models) const ILLAGER_SHARED_CROSSED_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HEAD,
        children: &ILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_BODY,
        children: &[],
    },
    ILLAGER_CROSSED_ARM_PART,
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const ILLAGER_SHARED_UNCROSSED_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HEAD,
        children: &ILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ILLAGER_RIGHT_ARM_PART,
    ILLAGER_LEFT_ARM_PART,
];

pub(in crate::entity_models) const ILLAGER_ILLUSIONER_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HEAD,
        children: &ILLAGER_HEAD_WITH_HAT_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_BODY,
        children: &[],
    },
    ILLAGER_CROSSED_ARM_PART,
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
];

// Textured `IllagerModel.createBodyLayer` (64x64 UVs). The deformed cubes (the hat, the body's
// robe overlay) inflate their geometry but keep the base box as `uv_size`, exactly like
// `CubeDeformation` in vanilla `addBox`. The geometry (min/size) matches the colored cubes above,
// so both render paths share the same mesh.
const fn illager_textured_cube(
    min: [f32; 3],
    size: [f32; 3],
    uv_size: [f32; 3],
    tex: [f32; 2],
    mirror: bool,
) -> TexturedModelCubeDesc {
    TexturedModelCubeDesc {
        min,
        size,
        uv_size,
        tex,
        mirror,
    }
}

const fn illager_textured_part(
    offset: [f32; 3],
    cubes: &'static [TexturedModelCubeDesc],
    children: &'static [TexturedModelPartDesc],
) -> TexturedModelPartDesc {
    TexturedModelPartDesc {
        pose: PartPose {
            offset,
            rotation: [0.0, 0.0, 0.0],
        },
        cubes,
        children,
    }
}

const ILLAGER_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] = [illager_textured_cube(
    [-4.0, -10.0, -4.0],
    [8.0, 10.0, 8.0],
    [8.0, 10.0, 8.0],
    [0.0, 0.0],
    false,
)];
const ILLAGER_TEXTURED_HAT: [TexturedModelCubeDesc; 1] = [illager_textured_cube(
    [-4.45, -10.45, -4.45],
    [8.9, 12.9, 8.9],
    [8.0, 12.0, 8.0],
    [32.0, 0.0],
    false,
)];
const ILLAGER_TEXTURED_NOSE: [TexturedModelCubeDesc; 1] = [illager_textured_cube(
    [-1.0, -1.0, -6.0],
    [2.0, 4.0, 2.0],
    [2.0, 4.0, 2.0],
    [24.0, 0.0],
    false,
)];
const ILLAGER_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    illager_textured_cube(
        [-4.0, 0.0, -3.0],
        [8.0, 12.0, 6.0],
        [8.0, 12.0, 6.0],
        [16.0, 20.0],
        false,
    ),
    illager_textured_cube(
        [-4.5, -0.5, -3.5],
        [9.0, 21.0, 7.0],
        [8.0, 20.0, 6.0],
        [0.0, 38.0],
        false,
    ),
];
const ILLAGER_TEXTURED_CROSSED_ARMS: [TexturedModelCubeDesc; 2] = [
    illager_textured_cube(
        [-8.0, -2.0, -2.0],
        [4.0, 8.0, 4.0],
        [4.0, 8.0, 4.0],
        [44.0, 22.0],
        false,
    ),
    illager_textured_cube(
        [-4.0, 2.0, -2.0],
        [8.0, 4.0, 4.0],
        [8.0, 4.0, 4.0],
        [40.0, 38.0],
        false,
    ),
];
const ILLAGER_TEXTURED_LEFT_SHOULDER: [TexturedModelCubeDesc; 1] = [illager_textured_cube(
    [4.0, -2.0, -2.0],
    [4.0, 8.0, 4.0],
    [4.0, 8.0, 4.0],
    [44.0, 22.0],
    true,
)];
const ILLAGER_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] = [illager_textured_cube(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [0.0, 22.0],
    false,
)];
const ILLAGER_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] = [illager_textured_cube(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [0.0, 22.0],
    true,
)];
const ILLAGER_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] = [illager_textured_cube(
    [-3.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [40.0, 46.0],
    false,
)];
const ILLAGER_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] = [illager_textured_cube(
    [-1.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [40.0, 46.0],
    true,
)];

const ILLAGER_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 1] = [illager_textured_part(
    [0.0, -2.0, 0.0],
    &ILLAGER_TEXTURED_NOSE,
    &[],
)];
const ILLAGER_TEXTURED_HEAD_WITH_HAT_CHILDREN: [TexturedModelPartDesc; 2] = [
    illager_textured_part([0.0, 0.0, 0.0], &ILLAGER_TEXTURED_HAT, &[]),
    illager_textured_part([0.0, -2.0, 0.0], &ILLAGER_TEXTURED_NOSE, &[]),
];
const ILLAGER_TEXTURED_CROSSED_ARM_CHILDREN: [TexturedModelPartDesc; 1] = [illager_textured_part(
    [0.0, 0.0, 0.0],
    &ILLAGER_TEXTURED_LEFT_SHOULDER,
    &[],
)];
const ILLAGER_TEXTURED_CROSSED_ARM_PART: TexturedModelPartDesc = TexturedModelPartDesc {
    pose: PartPose {
        offset: [0.0, 3.0, -1.0],
        rotation: [-0.75, 0.0, 0.0],
    },
    cubes: &ILLAGER_TEXTURED_CROSSED_ARMS,
    children: &ILLAGER_TEXTURED_CROSSED_ARM_CHILDREN,
};
const ILLAGER_TEXTURED_RIGHT_ARM_PART: TexturedModelPartDesc = TexturedModelPartDesc {
    pose: PartPose {
        offset: [-5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_TEXTURED_RIGHT_ARM,
    children: &[],
};
const ILLAGER_TEXTURED_LEFT_ARM_PART: TexturedModelPartDesc = TexturedModelPartDesc {
    pose: PartPose {
        offset: [5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_TEXTURED_LEFT_ARM,
    children: &[],
};

pub(in crate::entity_models) const ILLAGER_TEXTURED_CROSSED_PARTS: [TexturedModelPartDesc; 5] = [
    illager_textured_part(
        [0.0, 0.0, 0.0],
        &ILLAGER_TEXTURED_HEAD,
        &ILLAGER_TEXTURED_HEAD_CHILDREN,
    ),
    illager_textured_part([0.0, 0.0, 0.0], &ILLAGER_TEXTURED_BODY, &[]),
    ILLAGER_TEXTURED_CROSSED_ARM_PART,
    illager_textured_part([-2.0, 12.0, 0.0], &ILLAGER_TEXTURED_RIGHT_LEG, &[]),
    illager_textured_part([2.0, 12.0, 0.0], &ILLAGER_TEXTURED_LEFT_LEG, &[]),
];

pub(in crate::entity_models) const ILLAGER_TEXTURED_ILLUSIONER_PARTS: [TexturedModelPartDesc; 5] = [
    illager_textured_part(
        [0.0, 0.0, 0.0],
        &ILLAGER_TEXTURED_HEAD,
        &ILLAGER_TEXTURED_HEAD_WITH_HAT_CHILDREN,
    ),
    illager_textured_part([0.0, 0.0, 0.0], &ILLAGER_TEXTURED_BODY, &[]),
    ILLAGER_TEXTURED_CROSSED_ARM_PART,
    illager_textured_part([-2.0, 12.0, 0.0], &ILLAGER_TEXTURED_RIGHT_LEG, &[]),
    illager_textured_part([2.0, 12.0, 0.0], &ILLAGER_TEXTURED_LEFT_LEG, &[]),
];

pub(in crate::entity_models) const ILLAGER_TEXTURED_UNCROSSED_PARTS: [TexturedModelPartDesc; 6] = [
    illager_textured_part(
        [0.0, 0.0, 0.0],
        &ILLAGER_TEXTURED_HEAD,
        &ILLAGER_TEXTURED_HEAD_CHILDREN,
    ),
    illager_textured_part([0.0, 0.0, 0.0], &ILLAGER_TEXTURED_BODY, &[]),
    illager_textured_part([-2.0, 12.0, 0.0], &ILLAGER_TEXTURED_RIGHT_LEG, &[]),
    illager_textured_part([2.0, 12.0, 0.0], &ILLAGER_TEXTURED_LEFT_LEG, &[]),
    ILLAGER_TEXTURED_RIGHT_ARM_PART,
    ILLAGER_TEXTURED_LEFT_ARM_PART,
];
