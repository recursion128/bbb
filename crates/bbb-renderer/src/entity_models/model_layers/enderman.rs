use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    ENDERMAN_DARK, PART_POSE_ZERO,
};

pub(in crate::entity_models) const MODEL_LAYER_ENDERMAN: &str = "minecraft:enderman#main";

pub(in crate::entity_models) const ENDERMAN_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -4.0],
    size: [8.0, 8.0, 8.0],
    color: ENDERMAN_DARK,
}];

pub(in crate::entity_models) const ENDERMAN_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -7.5, -3.5],
    size: [7.0, 7.0, 7.0],
    color: ENDERMAN_DARK,
}];

pub(in crate::entity_models) const ENDERMAN_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: ENDERMAN_DARK,
}];

pub(in crate::entity_models) const ENDERMAN_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -1.0],
    size: [2.0, 30.0, 2.0],
    color: ENDERMAN_DARK,
}];

pub(in crate::entity_models) const ENDERMAN_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 30.0, 2.0],
    color: ENDERMAN_DARK,
}];

pub(in crate::entity_models) const ENDERMAN_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ENDERMAN_HAT,
    children: &[],
}];

/// Vanilla `EndermanModel.setupAnim` `isCreepy` branch: the head is dropped `y -= 5` while
/// the hat (this child) is raised `y += 5`, so the outer head layer keeps its world
/// position as the inner head opens downward into the screech pose. Swapped in for
/// [`ENDERMAN_HEAD_CHILDREN`] when the enderman is creepy.
pub(in crate::entity_models) const ENDERMAN_HEAD_CHILDREN_CREEPY: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 5.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_HAT,
        children: &[],
    }];

// Vanilla 26.1 EndermanModel.createBodyLayer().
pub(in crate::entity_models) const ENDERMAN_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_HEAD,
        children: &ENDERMAN_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -14.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, -12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, -12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -5.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, -5.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -8.0, -4.0],
        size: [8.0, 8.0, 8.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_HAT: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.5, -7.5, -3.5],
        size: [7.0, 7.0, 7.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ENDERMAN_TEXTURED_HAT,
        children: &[],
    }];

/// Vanilla `EndermanModel.setupAnim` `isCreepy` hat raise (`y += 5`), the textured
/// counterpart to [`ENDERMAN_HEAD_CHILDREN_CREEPY`].
pub(in crate::entity_models) const ENDERMAN_TEXTURED_HEAD_CHILDREN_CREEPY: [TexturedModelPartDesc;
    1] = [TexturedModelPartDesc {
    pose: PartPose {
        offset: [0.0, 5.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ENDERMAN_TEXTURED_HAT,
    children: &[],
}];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 0.0, -2.0],
        size: [8.0, 12.0, 4.0],
        uv_size: [8.0, 12.0, 4.0],
        tex: [32.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -2.0, -1.0],
        size: [2.0, 30.0, 2.0],
        uv_size: [2.0, 30.0, 2.0],
        tex: [56.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -2.0, -1.0],
        size: [2.0, 30.0, 2.0],
        uv_size: [2.0, 30.0, 2.0],
        tex: [56.0, 0.0],
        mirror: true,
    }];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 30.0, 2.0],
        uv_size: [2.0, 30.0, 2.0],
        tex: [56.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 30.0, 2.0],
        uv_size: [2.0, 30.0, 2.0],
        tex: [56.0, 0.0],
        mirror: true,
    }];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: ENDERMAN_PARTS[0].pose,
        cubes: &ENDERMAN_TEXTURED_HEAD,
        children: &ENDERMAN_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: ENDERMAN_PARTS[1].pose,
        cubes: &ENDERMAN_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ENDERMAN_PARTS[2].pose,
        cubes: &ENDERMAN_TEXTURED_RIGHT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ENDERMAN_PARTS[3].pose,
        cubes: &ENDERMAN_TEXTURED_LEFT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ENDERMAN_PARTS[4].pose,
        cubes: &ENDERMAN_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ENDERMAN_PARTS[5].pose,
        cubes: &ENDERMAN_TEXTURED_LEFT_LEG,
        children: &[],
    },
];
