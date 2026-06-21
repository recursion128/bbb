use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    PART_POSE_ZERO, VILLAGER_ROBE,
};

pub(in crate::entity_models) const MODEL_LAYER_VILLAGER: &str = "minecraft:villager#main";
pub(in crate::entity_models) const MODEL_LAYER_VILLAGER_BABY: &str = "minecraft:villager_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_WANDERING_TRADER: &str =
    "minecraft:wandering_trader#main";

pub(in crate::entity_models) const ADULT_VILLAGER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -10.0, -4.0],
    size: [8.0, 10.0, 8.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const ADULT_VILLAGER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.51, -10.51, -4.51],
    size: [9.02, 11.02, 9.02],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const ADULT_VILLAGER_HAT_RIM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-8.0, -8.0, -6.0],
    size: [16.0, 16.0, 1.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const ADULT_VILLAGER_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -6.0],
    size: [2.0, 4.0, 2.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const ADULT_VILLAGER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -3.0],
    size: [8.0, 12.0, 6.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const ADULT_VILLAGER_JACKET: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -0.5, -3.5],
    size: [9.0, 21.0, 7.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const ADULT_VILLAGER_ARMS: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-8.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [4.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-4.0, 2.0, -2.0],
        size: [8.0, 4.0, 4.0],
        color: VILLAGER_ROBE,
    },
];

pub(in crate::entity_models) const ADULT_VILLAGER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const ADULT_VILLAGER_HAT_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 0.0, 0.0],
            rotation: [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_HAT_RIM,
        children: &[],
    }];

pub(in crate::entity_models) const ADULT_VILLAGER_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_VILLAGER_HAT,
        children: &ADULT_VILLAGER_HAT_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_NOSE,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_VILLAGER_BODY_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_VILLAGER_JACKET,
        children: &[],
    }];

// Vanilla 26.1 VillagerModel.createBodyModel(), with LayerDefinitions'
// MeshTransformer.scaling(0.9375F) applied by the emitter root transform.
pub(in crate::entity_models) const ADULT_VILLAGER_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_VILLAGER_HEAD,
        children: &ADULT_VILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_VILLAGER_BODY,
        children: &ADULT_VILLAGER_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 3.0, -1.0],
            rotation: [-0.75, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_ARMS,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_VILLAGER_RIGHT_HAND: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.0, -2.4925, -1.8401],
        size: [2.0, 4.0, 2.0],
        color: VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [5.0, -2.4925, -1.8401],
        size: [2.0, 4.0, 2.0],
        color: VILLAGER_ROBE,
    },
];

pub(in crate::entity_models) const BABY_VILLAGER_MIDDLE_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -0.9924, -0.9825],
    size: [4.0, 2.0, 2.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_VILLAGER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -0.5, -1.0],
    size: [2.0, 3.0, 2.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_VILLAGER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -3.5],
    size: [8.0, 8.0, 7.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_VILLAGER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.3, -4.3, -3.8],
    size: [8.6, 8.6, 7.6],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_VILLAGER_HAT_RIM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-7.0, -0.5, -6.0],
    size: [14.0, 1.0, 12.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_VILLAGER_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -0.5],
    size: [2.0, 2.0, 1.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_VILLAGER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -2.75, -1.5],
    size: [4.0, 5.0, 3.0],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_VILLAGER_BB_MAIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.7, -8.2, -1.7],
    size: [4.4, 6.4, 3.4],
    color: VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_VILLAGER_ARMS_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 1.4025, -0.9599],
            rotation: [-1.0472, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_RIGHT_HAND,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 0.9024, -1.8175],
            rotation: [-1.0472, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_MIDDLE_ARM,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_VILLAGER_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -4.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_HAT,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -4.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_HAT_RIM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_NOSE,
        children: &[],
    },
];

// Vanilla 26.1 BabyVillagerModel.createBodyModel().
pub(in crate::entity_models) const BABY_VILLAGER_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 17.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_VILLAGER_ARMS_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 21.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 21.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 16.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_HEAD,
        children: &BABY_VILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.75, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.5, 24.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_BB_MAIN,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_VILLAGER_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -10.0, -4.0],
        size: [8.0, 10.0, 8.0],
        uv_size: [8.0, 10.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_VILLAGER_TEXTURED_HAT: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.51, -10.51, -4.51],
        size: [9.02, 11.02, 9.02],
        uv_size: [8.0, 10.0, 8.0],
        tex: [32.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_VILLAGER_TEXTURED_HAT_RIM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-8.0, -8.0, -6.0],
        size: [16.0, 16.0, 1.0],
        uv_size: [16.0, 16.0, 1.0],
        tex: [30.0, 47.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_VILLAGER_TEXTURED_NOSE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -1.0, -6.0],
        size: [2.0, 4.0, 2.0],
        uv_size: [2.0, 4.0, 2.0],
        tex: [24.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_VILLAGER_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 0.0, -3.0],
        size: [8.0, 12.0, 6.0],
        uv_size: [8.0, 12.0, 6.0],
        tex: [16.0, 20.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_VILLAGER_TEXTURED_JACKET: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.5, -0.5, -3.5],
        size: [9.0, 21.0, 7.0],
        uv_size: [8.0, 20.0, 6.0],
        tex: [0.0, 38.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_VILLAGER_TEXTURED_ARMS: [TexturedModelCubeDesc; 3] = [
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

pub(in crate::entity_models) const ADULT_VILLAGER_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 12.0, 4.0],
        uv_size: [4.0, 12.0, 4.0],
        tex: [0.0, 22.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_VILLAGER_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 12.0, 4.0],
        uv_size: [4.0, 12.0, 4.0],
        tex: [0.0, 22.0],
        mirror: true,
    }];

pub(in crate::entity_models) const ADULT_VILLAGER_TEXTURED_HAT_CHILDREN: [TexturedModelPartDesc;
    1] = [TexturedModelPartDesc {
    pose: ADULT_VILLAGER_HAT_CHILDREN[0].pose,
    cubes: &ADULT_VILLAGER_TEXTURED_HAT_RIM,
    children: &[],
}];

pub(in crate::entity_models) const ADULT_VILLAGER_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc;
    2] = [
    TexturedModelPartDesc {
        pose: ADULT_VILLAGER_HEAD_CHILDREN[0].pose,
        cubes: &ADULT_VILLAGER_TEXTURED_HAT,
        children: &ADULT_VILLAGER_TEXTURED_HAT_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: ADULT_VILLAGER_HEAD_CHILDREN[1].pose,
        cubes: &ADULT_VILLAGER_TEXTURED_NOSE,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_VILLAGER_TEXTURED_BODY_CHILDREN: [TexturedModelPartDesc;
    1] = [TexturedModelPartDesc {
    pose: ADULT_VILLAGER_BODY_CHILDREN[0].pose,
    cubes: &ADULT_VILLAGER_TEXTURED_JACKET,
    children: &[],
}];

pub(in crate::entity_models) const ADULT_VILLAGER_TEXTURED_PARTS: [TexturedModelPartDesc; 5] = [
    TexturedModelPartDesc {
        pose: ADULT_VILLAGER_PARTS[0].pose,
        cubes: &ADULT_VILLAGER_TEXTURED_HEAD,
        children: &ADULT_VILLAGER_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: ADULT_VILLAGER_PARTS[1].pose,
        cubes: &ADULT_VILLAGER_TEXTURED_BODY,
        children: &ADULT_VILLAGER_TEXTURED_BODY_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: ADULT_VILLAGER_PARTS[2].pose,
        cubes: &ADULT_VILLAGER_TEXTURED_ARMS,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_VILLAGER_PARTS[3].pose,
        cubes: &ADULT_VILLAGER_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_VILLAGER_PARTS[4].pose,
        cubes: &ADULT_VILLAGER_TEXTURED_LEFT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_VILLAGER_TEXTURED_RIGHT_HAND: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-1.0, -2.4925, -1.8401],
        size: [2.0, 4.0, 2.0],
        uv_size: [2.0, 4.0, 2.0],
        tex: [36.0, 15.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [5.0, -2.4925, -1.8401],
        size: [2.0, 4.0, 2.0],
        uv_size: [2.0, 4.0, 2.0],
        tex: [16.0, 15.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BABY_VILLAGER_TEXTURED_MIDDLE_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -0.9924, -0.9825],
        size: [4.0, 2.0, 2.0],
        uv_size: [4.0, 2.0, 2.0],
        tex: [24.0, 17.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_VILLAGER_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -0.5, -1.0],
        size: [2.0, 3.0, 2.0],
        uv_size: [2.0, 3.0, 2.0],
        tex: [8.0, 23.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_VILLAGER_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -0.5, -1.0],
        size: [2.0, 3.0, 2.0],
        uv_size: [2.0, 3.0, 2.0],
        tex: [0.0, 23.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_VILLAGER_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -8.0, -3.5],
        size: [8.0, 8.0, 7.0],
        uv_size: [8.0, 8.0, 7.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_VILLAGER_TEXTURED_HAT: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.3, -4.3, -3.8],
        size: [8.6, 8.6, 7.6],
        uv_size: [8.0, 8.0, 7.0],
        tex: [0.0, 30.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_VILLAGER_TEXTURED_HAT_RIM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-7.0, -0.5, -6.0],
        size: [14.0, 1.0, 12.0],
        uv_size: [14.0, 1.0, 12.0],
        tex: [0.0, 45.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_VILLAGER_TEXTURED_NOSE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -0.5],
        size: [2.0, 2.0, 1.0],
        uv_size: [2.0, 2.0, 1.0],
        tex: [23.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_VILLAGER_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -2.75, -1.5],
        size: [4.0, 5.0, 3.0],
        uv_size: [4.0, 5.0, 3.0],
        tex: [0.0, 15.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_VILLAGER_TEXTURED_BB_MAIN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.7, -8.2, -1.7],
        size: [4.4, 6.4, 3.4],
        uv_size: [4.0, 6.0, 3.0],
        tex: [16.0, 21.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_VILLAGER_TEXTURED_ARMS_CHILDREN: [TexturedModelPartDesc;
    2] = [
    TexturedModelPartDesc {
        pose: BABY_VILLAGER_ARMS_CHILDREN[0].pose,
        cubes: &BABY_VILLAGER_TEXTURED_RIGHT_HAND,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_VILLAGER_ARMS_CHILDREN[1].pose,
        cubes: &BABY_VILLAGER_TEXTURED_MIDDLE_ARM,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_VILLAGER_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc;
    3] = [
    TexturedModelPartDesc {
        pose: BABY_VILLAGER_HEAD_CHILDREN[0].pose,
        cubes: &BABY_VILLAGER_TEXTURED_HAT,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_VILLAGER_HEAD_CHILDREN[1].pose,
        cubes: &BABY_VILLAGER_TEXTURED_HAT_RIM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_VILLAGER_HEAD_CHILDREN[2].pose,
        cubes: &BABY_VILLAGER_TEXTURED_NOSE,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_VILLAGER_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: BABY_VILLAGER_PARTS[0].pose,
        cubes: &[],
        children: &BABY_VILLAGER_TEXTURED_ARMS_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: BABY_VILLAGER_PARTS[1].pose,
        cubes: &BABY_VILLAGER_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_VILLAGER_PARTS[2].pose,
        cubes: &BABY_VILLAGER_TEXTURED_LEFT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_VILLAGER_PARTS[3].pose,
        cubes: &BABY_VILLAGER_TEXTURED_HEAD,
        children: &BABY_VILLAGER_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: BABY_VILLAGER_PARTS[4].pose,
        cubes: &BABY_VILLAGER_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_VILLAGER_PARTS[5].pose,
        cubes: &BABY_VILLAGER_TEXTURED_BB_MAIN,
        children: &[],
    },
];
