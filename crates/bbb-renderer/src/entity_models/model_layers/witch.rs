use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    PART_POSE_ZERO, WITCH_HAT_COLOR, WITCH_ROBE,
};

pub(in crate::entity_models) const MODEL_LAYER_WITCH: &str = "minecraft:witch#main";

pub(in crate::entity_models) const WITCH_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -10.0, -4.0],
    size: [8.0, 10.0, 8.0],
    color: WITCH_ROBE,
}];

pub(in crate::entity_models) const WITCH_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [10.0, 2.0, 10.0],
    color: WITCH_HAT_COLOR,
}];

pub(in crate::entity_models) const WITCH_HAT_2: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [7.0, 4.0, 7.0],
    color: WITCH_HAT_COLOR,
}];

pub(in crate::entity_models) const WITCH_HAT_3: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [4.0, 4.0, 4.0],
    color: WITCH_HAT_COLOR,
}];

pub(in crate::entity_models) const WITCH_HAT_4: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.25, -0.25, -0.25],
    size: [1.5, 2.5, 1.5],
    color: WITCH_HAT_COLOR,
}];

pub(in crate::entity_models) const WITCH_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -6.0],
    size: [2.0, 4.0, 2.0],
    color: WITCH_ROBE,
}];

pub(in crate::entity_models) const WITCH_MOLE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.25, 3.25, -6.5],
    size: [0.5, 0.5, 0.5],
    color: WITCH_ROBE,
}];

pub(in crate::entity_models) const WITCH_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -3.0],
    size: [8.0, 12.0, 6.0],
    color: WITCH_ROBE,
}];

pub(in crate::entity_models) const WITCH_JACKET: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -0.5, -3.5],
    size: [9.0, 21.0, 7.0],
    color: WITCH_ROBE,
}];

pub(in crate::entity_models) const WITCH_ARMS: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-8.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: WITCH_ROBE,
    },
    ModelCubeDesc {
        min: [4.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: WITCH_ROBE,
    },
    ModelCubeDesc {
        min: [-4.0, 2.0, -2.0],
        size: [8.0, 4.0, 4.0],
        color: WITCH_ROBE,
    },
];

pub(in crate::entity_models) const WITCH_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: WITCH_ROBE,
}];

pub(in crate::entity_models) const WITCH_HAT_3_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [1.75, -2.0, 2.0],
        rotation: [-(std::f32::consts::PI / 15.0), 0.0, 0.10471976],
    },
    cubes: &WITCH_HAT_4,
    children: &[],
}];

pub(in crate::entity_models) const WITCH_HAT_2_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [1.75, -4.0, 2.0],
        rotation: [-0.10471976, 0.0, 0.05235988],
    },
    cubes: &WITCH_HAT_3,
    children: &WITCH_HAT_3_CHILDREN,
}];

pub(in crate::entity_models) const WITCH_HAT_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [1.75, -4.0, 2.0],
        rotation: [-0.05235988, 0.0, 0.02617994],
    },
    cubes: &WITCH_HAT_2,
    children: &WITCH_HAT_2_CHILDREN,
}];

pub(in crate::entity_models) const WITCH_NOSE_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &WITCH_MOLE,
    children: &[],
}];

pub(in crate::entity_models) const WITCH_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, -10.03125, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &WITCH_HAT,
        children: &WITCH_HAT_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &WITCH_NOSE,
        children: &WITCH_NOSE_CHILDREN,
    },
];

pub(in crate::entity_models) const WITCH_BODY_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &WITCH_JACKET,
    children: &[],
}];

// Vanilla 26.1 WitchModel.createBodyLayer(), with LayerDefinitions'
// MeshTransformer.scaling(0.9375F) applied by the emitter root transform.
pub(in crate::entity_models) const WITCH_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &WITCH_HEAD,
        children: &WITCH_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &WITCH_BODY,
        children: &WITCH_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 3.0, -1.0],
            rotation: [-0.75, 0.0, 0.0],
        },
        cubes: &WITCH_ARMS,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &WITCH_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &WITCH_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const WITCH_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -10.0, -4.0],
        size: [8.0, 10.0, 8.0],
        uv_size: [8.0, 10.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const WITCH_TEXTURED_HAT: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [10.0, 2.0, 10.0],
        uv_size: [10.0, 2.0, 10.0],
        tex: [0.0, 64.0],
        mirror: false,
    }];

pub(in crate::entity_models) const WITCH_TEXTURED_HAT_2: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [7.0, 4.0, 7.0],
        uv_size: [7.0, 4.0, 7.0],
        tex: [0.0, 76.0],
        mirror: false,
    }];

pub(in crate::entity_models) const WITCH_TEXTURED_HAT_3: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [4.0, 4.0, 4.0],
        uv_size: [4.0, 4.0, 4.0],
        tex: [0.0, 87.0],
        mirror: false,
    }];

pub(in crate::entity_models) const WITCH_TEXTURED_HAT_4: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-0.25, -0.25, -0.25],
        size: [1.5, 2.5, 1.5],
        uv_size: [1.0, 2.0, 1.0],
        tex: [0.0, 95.0],
        mirror: false,
    }];

pub(in crate::entity_models) const WITCH_TEXTURED_NOSE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -1.0, -6.0],
        size: [2.0, 4.0, 2.0],
        uv_size: [2.0, 4.0, 2.0],
        tex: [24.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const WITCH_TEXTURED_MOLE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.25, 3.25, -6.5],
        size: [0.5, 0.5, 0.5],
        uv_size: [1.0, 1.0, 1.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const WITCH_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 0.0, -3.0],
        size: [8.0, 12.0, 6.0],
        uv_size: [8.0, 12.0, 6.0],
        tex: [16.0, 20.0],
        mirror: false,
    }];

pub(in crate::entity_models) const WITCH_TEXTURED_JACKET: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.5, -0.5, -3.5],
        size: [9.0, 21.0, 7.0],
        uv_size: [8.0, 20.0, 6.0],
        tex: [0.0, 38.0],
        mirror: false,
    }];

pub(in crate::entity_models) const WITCH_TEXTURED_ARMS: [TexturedModelCubeDesc; 3] = [
    TexturedModelCubeDesc {
        min: [-8.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        uv_size: [4.0, 8.0, 4.0],
        tex: [44.0, 22.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [4.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        uv_size: [4.0, 8.0, 4.0],
        tex: [44.0, 22.0],
        mirror: true,
    },
    TexturedModelCubeDesc {
        min: [-4.0, 2.0, -2.0],
        size: [8.0, 4.0, 4.0],
        uv_size: [8.0, 4.0, 4.0],
        tex: [40.0, 38.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const WITCH_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 12.0, 4.0],
        uv_size: [4.0, 12.0, 4.0],
        tex: [0.0, 22.0],
        mirror: false,
    }];

pub(in crate::entity_models) const WITCH_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 12.0, 4.0],
        uv_size: [4.0, 12.0, 4.0],
        tex: [0.0, 22.0],
        mirror: true,
    }];

pub(in crate::entity_models) const WITCH_TEXTURED_HAT_3_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: WITCH_HAT_3_CHILDREN[0].pose,
        cubes: &WITCH_TEXTURED_HAT_4,
        children: &[],
    }];

pub(in crate::entity_models) const WITCH_TEXTURED_HAT_2_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: WITCH_HAT_2_CHILDREN[0].pose,
        cubes: &WITCH_TEXTURED_HAT_3,
        children: &WITCH_TEXTURED_HAT_3_CHILDREN,
    }];

pub(in crate::entity_models) const WITCH_TEXTURED_HAT_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: WITCH_HAT_CHILDREN[0].pose,
        cubes: &WITCH_TEXTURED_HAT_2,
        children: &WITCH_TEXTURED_HAT_2_CHILDREN,
    }];

pub(in crate::entity_models) const WITCH_TEXTURED_NOSE_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: WITCH_NOSE_CHILDREN[0].pose,
        cubes: &WITCH_TEXTURED_MOLE,
        children: &[],
    }];

pub(in crate::entity_models) const WITCH_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: WITCH_HEAD_CHILDREN[0].pose,
        cubes: &WITCH_TEXTURED_HAT,
        children: &WITCH_TEXTURED_HAT_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: WITCH_HEAD_CHILDREN[1].pose,
        cubes: &WITCH_TEXTURED_NOSE,
        children: &WITCH_TEXTURED_NOSE_CHILDREN,
    },
];

pub(in crate::entity_models) const WITCH_TEXTURED_BODY_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: WITCH_BODY_CHILDREN[0].pose,
        cubes: &WITCH_TEXTURED_JACKET,
        children: &[],
    }];

pub(in crate::entity_models) const WITCH_TEXTURED_PARTS: [TexturedModelPartDesc; 5] = [
    TexturedModelPartDesc {
        pose: WITCH_PARTS[0].pose,
        cubes: &WITCH_TEXTURED_HEAD,
        children: &WITCH_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: WITCH_PARTS[1].pose,
        cubes: &WITCH_TEXTURED_BODY,
        children: &WITCH_TEXTURED_BODY_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: WITCH_PARTS[2].pose,
        cubes: &WITCH_TEXTURED_ARMS,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: WITCH_PARTS[3].pose,
        cubes: &WITCH_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: WITCH_PARTS[4].pose,
        cubes: &WITCH_TEXTURED_LEFT_LEG,
        children: &[],
    },
];
