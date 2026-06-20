use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc, BOAT_WOOD,
};

pub(in crate::entity_models) const MODEL_LAYER_ACACIA_BOAT: &str = "minecraft:boat/acacia#main";
pub(in crate::entity_models) const MODEL_LAYER_ACACIA_CHEST_BOAT: &str =
    "minecraft:chest_boat/acacia#main";
pub(in crate::entity_models) const MODEL_LAYER_BAMBOO_RAFT: &str = "minecraft:boat/bamboo#main";
pub(in crate::entity_models) const MODEL_LAYER_BAMBOO_CHEST_RAFT: &str =
    "minecraft:chest_boat/bamboo#main";
pub(in crate::entity_models) const MODEL_LAYER_BIRCH_BOAT: &str = "minecraft:boat/birch#main";
pub(in crate::entity_models) const MODEL_LAYER_BIRCH_CHEST_BOAT: &str =
    "minecraft:chest_boat/birch#main";
pub(in crate::entity_models) const MODEL_LAYER_CHERRY_BOAT: &str = "minecraft:boat/cherry#main";
pub(in crate::entity_models) const MODEL_LAYER_CHERRY_CHEST_BOAT: &str =
    "minecraft:chest_boat/cherry#main";
pub(in crate::entity_models) const MODEL_LAYER_DARK_OAK_BOAT: &str = "minecraft:boat/dark_oak#main";
pub(in crate::entity_models) const MODEL_LAYER_DARK_OAK_CHEST_BOAT: &str =
    "minecraft:chest_boat/dark_oak#main";
pub(in crate::entity_models) const MODEL_LAYER_JUNGLE_BOAT: &str = "minecraft:boat/jungle#main";
pub(in crate::entity_models) const MODEL_LAYER_JUNGLE_CHEST_BOAT: &str =
    "minecraft:chest_boat/jungle#main";
pub(in crate::entity_models) const MODEL_LAYER_MANGROVE_BOAT: &str = "minecraft:boat/mangrove#main";
pub(in crate::entity_models) const MODEL_LAYER_MANGROVE_CHEST_BOAT: &str =
    "minecraft:chest_boat/mangrove#main";
pub(in crate::entity_models) const MODEL_LAYER_OAK_BOAT: &str = "minecraft:boat/oak#main";
pub(in crate::entity_models) const MODEL_LAYER_OAK_CHEST_BOAT: &str =
    "minecraft:chest_boat/oak#main";
pub(in crate::entity_models) const MODEL_LAYER_PALE_OAK_BOAT: &str = "minecraft:boat/pale_oak#main";
pub(in crate::entity_models) const MODEL_LAYER_PALE_OAK_CHEST_BOAT: &str =
    "minecraft:chest_boat/pale_oak#main";
pub(in crate::entity_models) const MODEL_LAYER_SPRUCE_BOAT: &str = "minecraft:boat/spruce#main";
pub(in crate::entity_models) const MODEL_LAYER_SPRUCE_CHEST_BOAT: &str =
    "minecraft:chest_boat/spruce#main";

pub(in crate::entity_models) const BOAT_BOTTOM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-14.0, -9.0, -3.0],
    size: [28.0, 16.0, 3.0],
    color: BOAT_WOOD,
}];

pub(in crate::entity_models) const BOAT_BACK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-13.0, -7.0, -1.0],
    size: [18.0, 6.0, 2.0],
    color: BOAT_WOOD,
}];

pub(in crate::entity_models) const BOAT_FRONT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-8.0, -7.0, -1.0],
    size: [16.0, 6.0, 2.0],
    color: BOAT_WOOD,
}];

pub(in crate::entity_models) const BOAT_SIDE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-14.0, -7.0, -1.0],
    size: [28.0, 6.0, 2.0],
    color: BOAT_WOOD,
}];

pub(in crate::entity_models) const BOAT_LEFT_PADDLE: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.0, 0.0, -5.0],
        size: [2.0, 2.0, 18.0],
        color: BOAT_WOOD,
    },
    ModelCubeDesc {
        min: [-1.001, -3.0, 8.0],
        size: [1.0, 6.0, 7.0],
        color: BOAT_WOOD,
    },
];

pub(in crate::entity_models) const BOAT_RIGHT_PADDLE: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.0, 0.0, -5.0],
        size: [2.0, 2.0, 18.0],
        color: BOAT_WOOD,
    },
    ModelCubeDesc {
        min: [0.001, -3.0, 8.0],
        size: [1.0, 6.0, 7.0],
        color: BOAT_WOOD,
    },
];

pub(in crate::entity_models) const BOAT_CHEST_BOTTOM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [12.0, 8.0, 12.0],
    color: BOAT_WOOD,
}];

pub(in crate::entity_models) const BOAT_CHEST_LID: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [12.0, 4.0, 12.0],
    color: BOAT_WOOD,
}];

pub(in crate::entity_models) const BOAT_CHEST_LOCK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [2.0, 4.0, 1.0],
    color: BOAT_WOOD,
}];

pub(in crate::entity_models) const RAFT_BOTTOM: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-14.0, -11.0, -4.0],
        size: [28.0, 20.0, 4.0],
        color: BOAT_WOOD,
    },
    ModelCubeDesc {
        min: [-14.0, -9.0, -8.0],
        size: [28.0, 16.0, 4.0],
        color: BOAT_WOOD,
    },
];

pub(in crate::entity_models) const BOAT_COMMON_PARTS: [ModelPartDesc; 7] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 3.0, 1.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &BOAT_BOTTOM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-15.0, 4.0, 4.0],
            rotation: [0.0, std::f32::consts::PI * 1.5, 0.0],
        },
        cubes: &BOAT_BACK,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [15.0, 4.0, 0.0],
            rotation: [0.0, std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &BOAT_FRONT,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, -9.0],
            rotation: [0.0, std::f32::consts::PI, 0.0],
        },
        cubes: &BOAT_SIDE,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, 9.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BOAT_SIDE,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, -5.0, 9.0],
            rotation: [0.0, 0.0, std::f32::consts::PI / 16.0],
        },
        cubes: &BOAT_LEFT_PADDLE,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, -5.0, -9.0],
            rotation: [0.0, std::f32::consts::PI, std::f32::consts::PI / 16.0],
        },
        cubes: &BOAT_RIGHT_PADDLE,
        children: &[],
    },
];

pub(in crate::entity_models) const BOAT_CHEST_PARTS: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -5.0, -6.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &BOAT_CHEST_BOTTOM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -9.0, -6.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &BOAT_CHEST_LID,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, -6.0, -1.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &BOAT_CHEST_LOCK,
        children: &[],
    },
];

pub(in crate::entity_models) const RAFT_COMMON_PARTS: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.1, 1.0],
            rotation: [1.5708, 0.0, 0.0],
        },
        cubes: &RAFT_BOTTOM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, -4.0, 9.0],
            rotation: [0.0, 0.0, std::f32::consts::PI / 16.0],
        },
        cubes: &BOAT_LEFT_PADDLE,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, -4.0, -9.0],
            rotation: [0.0, std::f32::consts::PI, std::f32::consts::PI / 16.0],
        },
        cubes: &BOAT_RIGHT_PADDLE,
        children: &[],
    },
];

pub(in crate::entity_models) const RAFT_CHEST_PARTS: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -10.1, -6.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &BOAT_CHEST_BOTTOM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -14.1, -6.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &BOAT_CHEST_LID,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, -11.1, -1.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &BOAT_CHEST_LOCK,
        children: &[],
    },
];

pub(in crate::entity_models) const BOAT_TEXTURED_BOTTOM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-14.0, -9.0, -3.0],
        size: [28.0, 16.0, 3.0],
        uv_size: [28.0, 16.0, 3.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOAT_TEXTURED_BACK: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-13.0, -7.0, -1.0],
        size: [18.0, 6.0, 2.0],
        uv_size: [18.0, 6.0, 2.0],
        tex: [0.0, 19.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOAT_TEXTURED_FRONT: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-8.0, -7.0, -1.0],
        size: [16.0, 6.0, 2.0],
        uv_size: [16.0, 6.0, 2.0],
        tex: [0.0, 27.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOAT_TEXTURED_RIGHT_SIDE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-14.0, -7.0, -1.0],
        size: [28.0, 6.0, 2.0],
        uv_size: [28.0, 6.0, 2.0],
        tex: [0.0, 35.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOAT_TEXTURED_LEFT_SIDE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-14.0, -7.0, -1.0],
        size: [28.0, 6.0, 2.0],
        uv_size: [28.0, 6.0, 2.0],
        tex: [0.0, 43.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOAT_TEXTURED_LEFT_PADDLE: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-1.0, 0.0, -5.0],
        size: [2.0, 2.0, 18.0],
        uv_size: [2.0, 2.0, 18.0],
        tex: [62.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-1.001, -3.0, 8.0],
        size: [1.0, 6.0, 7.0],
        uv_size: [1.0, 6.0, 7.0],
        tex: [62.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BOAT_TEXTURED_RIGHT_PADDLE: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-1.0, 0.0, -5.0],
        size: [2.0, 2.0, 18.0],
        uv_size: [2.0, 2.0, 18.0],
        tex: [62.0, 20.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [0.001, -3.0, 8.0],
        size: [1.0, 6.0, 7.0],
        uv_size: [1.0, 6.0, 7.0],
        tex: [62.0, 20.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const RAFT_TEXTURED_BOTTOM: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-14.0, -11.0, -4.0],
        size: [28.0, 20.0, 4.0],
        uv_size: [28.0, 20.0, 4.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-14.0, -9.0, -8.0],
        size: [28.0, 16.0, 4.0],
        uv_size: [28.0, 16.0, 4.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const RAFT_TEXTURED_LEFT_PADDLE: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-1.0, 0.0, -5.0],
        size: [2.0, 2.0, 18.0],
        uv_size: [2.0, 2.0, 18.0],
        tex: [0.0, 24.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-1.001, -3.0, 8.0],
        size: [1.0, 6.0, 7.0],
        uv_size: [1.0, 6.0, 7.0],
        tex: [0.0, 24.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const RAFT_TEXTURED_RIGHT_PADDLE: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-1.0, 0.0, -5.0],
        size: [2.0, 2.0, 18.0],
        uv_size: [2.0, 2.0, 18.0],
        tex: [40.0, 24.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [0.001, -3.0, 8.0],
        size: [1.0, 6.0, 7.0],
        uv_size: [1.0, 6.0, 7.0],
        tex: [40.0, 24.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BOAT_TEXTURED_CHEST_BOTTOM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [12.0, 8.0, 12.0],
        uv_size: [12.0, 8.0, 12.0],
        tex: [0.0, 76.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOAT_TEXTURED_CHEST_LID: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [12.0, 4.0, 12.0],
        uv_size: [12.0, 4.0, 12.0],
        tex: [0.0, 59.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOAT_TEXTURED_CHEST_LOCK: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [2.0, 4.0, 1.0],
        uv_size: [2.0, 4.0, 1.0],
        tex: [0.0, 59.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOAT_TEXTURED_PARTS: [TexturedModelPartDesc; 7] = [
    TexturedModelPartDesc {
        pose: BOAT_COMMON_PARTS[0].pose,
        cubes: &BOAT_TEXTURED_BOTTOM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOAT_COMMON_PARTS[1].pose,
        cubes: &BOAT_TEXTURED_BACK,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOAT_COMMON_PARTS[2].pose,
        cubes: &BOAT_TEXTURED_FRONT,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOAT_COMMON_PARTS[3].pose,
        cubes: &BOAT_TEXTURED_RIGHT_SIDE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOAT_COMMON_PARTS[4].pose,
        cubes: &BOAT_TEXTURED_LEFT_SIDE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOAT_COMMON_PARTS[5].pose,
        cubes: &BOAT_TEXTURED_LEFT_PADDLE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOAT_COMMON_PARTS[6].pose,
        cubes: &BOAT_TEXTURED_RIGHT_PADDLE,
        children: &[],
    },
];

pub(in crate::entity_models) const BOAT_CHEST_TEXTURED_PARTS: [TexturedModelPartDesc; 10] = [
    BOAT_TEXTURED_PARTS[0],
    BOAT_TEXTURED_PARTS[1],
    BOAT_TEXTURED_PARTS[2],
    BOAT_TEXTURED_PARTS[3],
    BOAT_TEXTURED_PARTS[4],
    BOAT_TEXTURED_PARTS[5],
    BOAT_TEXTURED_PARTS[6],
    TexturedModelPartDesc {
        pose: BOAT_CHEST_PARTS[0].pose,
        cubes: &BOAT_TEXTURED_CHEST_BOTTOM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOAT_CHEST_PARTS[1].pose,
        cubes: &BOAT_TEXTURED_CHEST_LID,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOAT_CHEST_PARTS[2].pose,
        cubes: &BOAT_TEXTURED_CHEST_LOCK,
        children: &[],
    },
];

pub(in crate::entity_models) const RAFT_TEXTURED_PARTS: [TexturedModelPartDesc; 3] = [
    TexturedModelPartDesc {
        pose: RAFT_COMMON_PARTS[0].pose,
        cubes: &RAFT_TEXTURED_BOTTOM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: RAFT_COMMON_PARTS[1].pose,
        cubes: &RAFT_TEXTURED_LEFT_PADDLE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: RAFT_COMMON_PARTS[2].pose,
        cubes: &RAFT_TEXTURED_RIGHT_PADDLE,
        children: &[],
    },
];

pub(in crate::entity_models) const RAFT_CHEST_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    RAFT_TEXTURED_PARTS[0],
    RAFT_TEXTURED_PARTS[1],
    RAFT_TEXTURED_PARTS[2],
    TexturedModelPartDesc {
        pose: RAFT_CHEST_PARTS[0].pose,
        cubes: &BOAT_TEXTURED_CHEST_BOTTOM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: RAFT_CHEST_PARTS[1].pose,
        cubes: &BOAT_TEXTURED_CHEST_LID,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: RAFT_CHEST_PARTS[2].pose,
        cubes: &BOAT_TEXTURED_CHEST_LOCK,
        children: &[],
    },
];
