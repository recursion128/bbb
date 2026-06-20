use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    PART_POSE_ZERO, PLAYER_BLUE,
};

pub(in crate::entity_models) const MODEL_LAYER_PLAYER: &str = "minecraft:player#main";
pub(in crate::entity_models) const MODEL_LAYER_PLAYER_SLIM: &str = "minecraft:player_slim#main";

pub(in crate::entity_models) const PLAYER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -4.0],
    size: [8.0, 8.0, 8.0],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -8.5, -4.5],
    size: [9.0, 9.0, 9.0],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &PLAYER_HAT,
    children: &[],
}];

pub(in crate::entity_models) const PLAYER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_JACKET: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.25, -0.25, -2.25],
    size: [8.5, 12.5, 4.5],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_BODY_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &PLAYER_JACKET,
    children: &[],
}];

pub(in crate::entity_models) const PLAYER_WIDE_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_WIDE_RIGHT_SLEEVE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.25, -2.25, -2.25],
    size: [4.5, 12.5, 4.5],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_WIDE_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_WIDE_LEFT_SLEEVE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.25, -2.25, -2.25],
    size: [4.5, 12.5, 4.5],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_SLIM_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -2.0, -2.0],
    size: [3.0, 12.0, 4.0],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_SLIM_RIGHT_SLEEVE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.25, -2.25, -2.25],
    size: [3.5, 12.5, 4.5],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_SLIM_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -2.0],
    size: [3.0, 12.0, 4.0],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_SLIM_LEFT_SLEEVE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.25, -2.25, -2.25],
    size: [3.5, 12.5, 4.5],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_PANTS: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.25, -0.25, -2.25],
    size: [4.5, 12.5, 4.5],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_RIGHT_PANTS_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_PANTS,
        children: &[],
    }];

pub(in crate::entity_models) const PLAYER_LEFT_PANTS_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_PANTS,
        children: &[],
    }];

pub(in crate::entity_models) const PLAYER_WIDE_RIGHT_ARM_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_WIDE_RIGHT_SLEEVE,
        children: &[],
    }];

pub(in crate::entity_models) const PLAYER_WIDE_LEFT_ARM_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_WIDE_LEFT_SLEEVE,
        children: &[],
    }];

pub(in crate::entity_models) const PLAYER_SLIM_RIGHT_ARM_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_SLIM_RIGHT_SLEEVE,
        children: &[],
    }];

pub(in crate::entity_models) const PLAYER_SLIM_LEFT_ARM_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_SLIM_LEFT_SLEEVE,
        children: &[],
    }];

// Vanilla 26.1 ModelLayers.PLAYER / PLAYER_SLIM:
// PlayerModel.createMesh(CubeDeformation.NONE, slim).
pub(in crate::entity_models) const PLAYER_WIDE_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_HEAD,
        children: &PLAYER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_BODY,
        children: &PLAYER_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PLAYER_WIDE_RIGHT_ARM,
        children: &PLAYER_WIDE_RIGHT_ARM_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PLAYER_WIDE_LEFT_ARM,
        children: &PLAYER_WIDE_LEFT_ARM_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PLAYER_LEG,
        children: &PLAYER_RIGHT_PANTS_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PLAYER_LEG,
        children: &PLAYER_LEFT_PANTS_CHILDREN,
    },
];

pub(in crate::entity_models) const PLAYER_SLIM_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_HEAD,
        children: &PLAYER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_BODY,
        children: &PLAYER_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PLAYER_SLIM_RIGHT_ARM,
        children: &PLAYER_SLIM_RIGHT_ARM_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PLAYER_SLIM_LEFT_ARM,
        children: &PLAYER_SLIM_LEFT_ARM_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PLAYER_LEG,
        children: &PLAYER_RIGHT_PANTS_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PLAYER_LEG,
        children: &PLAYER_LEFT_PANTS_CHILDREN,
    },
];

pub(in crate::entity_models) const PLAYER_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -8.0, -4.0],
        size: [8.0, 8.0, 8.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_HAT: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.5, -8.5, -4.5],
        size: [9.0, 9.0, 9.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [32.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_TEXTURED_HAT,
        children: &[],
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 0.0, -2.0],
        size: [8.0, 12.0, 4.0],
        uv_size: [8.0, 12.0, 4.0],
        tex: [16.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_JACKET: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.25, -0.25, -2.25],
        size: [8.5, 12.5, 4.5],
        uv_size: [8.0, 12.0, 4.0],
        tex: [16.0, 32.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_BODY_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_TEXTURED_JACKET,
        children: &[],
    }];

pub(in crate::entity_models) const PLAYER_WIDE_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -2.0, -2.0],
        size: [4.0, 12.0, 4.0],
        uv_size: [4.0, 12.0, 4.0],
        tex: [40.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_WIDE_TEXTURED_RIGHT_SLEEVE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.25, -2.25, -2.25],
        size: [4.5, 12.5, 4.5],
        uv_size: [4.0, 12.0, 4.0],
        tex: [40.0, 32.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_WIDE_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -2.0, -2.0],
        size: [4.0, 12.0, 4.0],
        uv_size: [4.0, 12.0, 4.0],
        tex: [32.0, 48.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_WIDE_TEXTURED_LEFT_SLEEVE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.25, -2.25, -2.25],
        size: [4.5, 12.5, 4.5],
        uv_size: [4.0, 12.0, 4.0],
        tex: [48.0, 48.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_SLIM_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -2.0, -2.0],
        size: [3.0, 12.0, 4.0],
        uv_size: [3.0, 12.0, 4.0],
        tex: [40.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_SLIM_TEXTURED_RIGHT_SLEEVE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.25, -2.25, -2.25],
        size: [3.5, 12.5, 4.5],
        uv_size: [3.0, 12.0, 4.0],
        tex: [40.0, 32.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_SLIM_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -2.0, -2.0],
        size: [3.0, 12.0, 4.0],
        uv_size: [3.0, 12.0, 4.0],
        tex: [32.0, 48.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_SLIM_TEXTURED_LEFT_SLEEVE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.25, -2.25, -2.25],
        size: [3.5, 12.5, 4.5],
        uv_size: [3.0, 12.0, 4.0],
        tex: [48.0, 48.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 12.0, 4.0],
        uv_size: [4.0, 12.0, 4.0],
        tex: [0.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 12.0, 4.0],
        uv_size: [4.0, 12.0, 4.0],
        tex: [16.0, 48.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_RIGHT_PANTS: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.25, -0.25, -2.25],
        size: [4.5, 12.5, 4.5],
        uv_size: [4.0, 12.0, 4.0],
        tex: [0.0, 32.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_LEFT_PANTS: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.25, -0.25, -2.25],
        size: [4.5, 12.5, 4.5],
        uv_size: [4.0, 12.0, 4.0],
        tex: [0.0, 48.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_RIGHT_PANTS_CHILDREN: [TexturedModelPartDesc;
    1] = [TexturedModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &PLAYER_TEXTURED_RIGHT_PANTS,
    children: &[],
}];

pub(in crate::entity_models) const PLAYER_TEXTURED_LEFT_PANTS_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_TEXTURED_LEFT_PANTS,
        children: &[],
    }];

pub(in crate::entity_models) const PLAYER_WIDE_TEXTURED_RIGHT_ARM_CHILDREN:
    [TexturedModelPartDesc; 1] = [TexturedModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &PLAYER_WIDE_TEXTURED_RIGHT_SLEEVE,
    children: &[],
}];

pub(in crate::entity_models) const PLAYER_WIDE_TEXTURED_LEFT_ARM_CHILDREN: [TexturedModelPartDesc;
    1] = [TexturedModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &PLAYER_WIDE_TEXTURED_LEFT_SLEEVE,
    children: &[],
}];

pub(in crate::entity_models) const PLAYER_SLIM_TEXTURED_RIGHT_ARM_CHILDREN:
    [TexturedModelPartDesc; 1] = [TexturedModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &PLAYER_SLIM_TEXTURED_RIGHT_SLEEVE,
    children: &[],
}];

pub(in crate::entity_models) const PLAYER_SLIM_TEXTURED_LEFT_ARM_CHILDREN: [TexturedModelPartDesc;
    1] = [TexturedModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &PLAYER_SLIM_TEXTURED_LEFT_SLEEVE,
    children: &[],
}];

pub(in crate::entity_models) const PLAYER_WIDE_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: PLAYER_WIDE_PARTS[0].pose,
        cubes: &PLAYER_TEXTURED_HEAD,
        children: &PLAYER_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_WIDE_PARTS[1].pose,
        cubes: &PLAYER_TEXTURED_BODY,
        children: &PLAYER_TEXTURED_BODY_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_WIDE_PARTS[2].pose,
        cubes: &PLAYER_WIDE_TEXTURED_RIGHT_ARM,
        children: &PLAYER_WIDE_TEXTURED_RIGHT_ARM_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_WIDE_PARTS[3].pose,
        cubes: &PLAYER_WIDE_TEXTURED_LEFT_ARM,
        children: &PLAYER_WIDE_TEXTURED_LEFT_ARM_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_WIDE_PARTS[4].pose,
        cubes: &PLAYER_TEXTURED_RIGHT_LEG,
        children: &PLAYER_TEXTURED_RIGHT_PANTS_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_WIDE_PARTS[5].pose,
        cubes: &PLAYER_TEXTURED_LEFT_LEG,
        children: &PLAYER_TEXTURED_LEFT_PANTS_CHILDREN,
    },
];

pub(in crate::entity_models) const PLAYER_SLIM_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: PLAYER_SLIM_PARTS[0].pose,
        cubes: &PLAYER_TEXTURED_HEAD,
        children: &PLAYER_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_SLIM_PARTS[1].pose,
        cubes: &PLAYER_TEXTURED_BODY,
        children: &PLAYER_TEXTURED_BODY_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_SLIM_PARTS[2].pose,
        cubes: &PLAYER_SLIM_TEXTURED_RIGHT_ARM,
        children: &PLAYER_SLIM_TEXTURED_RIGHT_ARM_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_SLIM_PARTS[3].pose,
        cubes: &PLAYER_SLIM_TEXTURED_LEFT_ARM,
        children: &PLAYER_SLIM_TEXTURED_LEFT_ARM_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_SLIM_PARTS[4].pose,
        cubes: &PLAYER_TEXTURED_RIGHT_LEG,
        children: &PLAYER_TEXTURED_RIGHT_PANTS_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_SLIM_PARTS[5].pose,
        cubes: &PLAYER_TEXTURED_LEFT_LEG,
        children: &PLAYER_TEXTURED_LEFT_PANTS_CHILDREN,
    },
];
