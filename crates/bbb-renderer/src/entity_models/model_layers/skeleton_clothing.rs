use super::{PartPose, TexturedModelCubeDesc, TexturedModelPartDesc, PART_POSE_ZERO};

pub(in crate::entity_models) const MODEL_LAYER_STRAY_OUTER_LAYER: &str = "minecraft:stray#outer";
pub(in crate::entity_models) const MODEL_LAYER_BOGGED_OUTER_LAYER: &str = "minecraft:bogged#outer";

pub(in crate::entity_models) const STRAY_OUTER_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.25, -8.25, -4.25],
        size: [8.5, 8.5, 8.5],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const STRAY_OUTER_TEXTURED_HAT: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.75, -8.75, -4.75],
        size: [9.5, 9.5, 9.5],
        uv_size: [8.0, 8.0, 8.0],
        tex: [32.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const STRAY_OUTER_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &STRAY_OUTER_TEXTURED_HAT,
        children: &[],
    }];

pub(in crate::entity_models) const STRAY_OUTER_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.25, -0.25, -2.25],
        size: [8.5, 12.5, 4.5],
        uv_size: [8.0, 12.0, 4.0],
        tex: [16.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const STRAY_OUTER_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.25, -2.25, -2.25],
        size: [4.5, 12.5, 4.5],
        uv_size: [4.0, 12.0, 4.0],
        tex: [40.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const STRAY_OUTER_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.25, -2.25, -2.25],
        size: [4.5, 12.5, 4.5],
        uv_size: [4.0, 12.0, 4.0],
        tex: [40.0, 16.0],
        mirror: true,
    }];

pub(in crate::entity_models) const STRAY_OUTER_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.25, -0.25, -2.25],
        size: [4.5, 12.5, 4.5],
        uv_size: [4.0, 12.0, 4.0],
        tex: [0.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const STRAY_OUTER_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.25, -0.25, -2.25],
        size: [4.5, 12.5, 4.5],
        uv_size: [4.0, 12.0, 4.0],
        tex: [0.0, 16.0],
        mirror: true,
    }];

// Vanilla 26.1 ModelLayers.STRAY_OUTER_LAYER:
// HumanoidModel.createMesh(new CubeDeformation(0.25F), 0.0F), 64x32.
pub(in crate::entity_models) const STRAY_OUTER_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &STRAY_OUTER_TEXTURED_HEAD,
        children: &STRAY_OUTER_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &STRAY_OUTER_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &STRAY_OUTER_TEXTURED_RIGHT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &STRAY_OUTER_TEXTURED_LEFT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &STRAY_OUTER_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &STRAY_OUTER_TEXTURED_LEFT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BOGGED_OUTER_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.2, -8.2, -4.2],
        size: [8.4, 8.4, 8.4],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOGGED_OUTER_TEXTURED_HAT: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.7, -8.7, -4.7],
        size: [9.4, 9.4, 9.4],
        uv_size: [8.0, 8.0, 8.0],
        tex: [32.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOGGED_OUTER_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &BOGGED_OUTER_TEXTURED_HAT,
        children: &[],
    }];

pub(in crate::entity_models) const BOGGED_OUTER_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.2, -0.2, -2.2],
        size: [8.4, 12.4, 4.4],
        uv_size: [8.0, 12.0, 4.0],
        tex: [16.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOGGED_OUTER_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.2, -2.2, -2.2],
        size: [4.4, 12.4, 4.4],
        uv_size: [4.0, 12.0, 4.0],
        tex: [40.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOGGED_OUTER_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.2, -2.2, -2.2],
        size: [4.4, 12.4, 4.4],
        uv_size: [4.0, 12.0, 4.0],
        tex: [40.0, 16.0],
        mirror: true,
    }];

pub(in crate::entity_models) const BOGGED_OUTER_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.2, -0.2, -2.2],
        size: [4.4, 12.4, 4.4],
        uv_size: [4.0, 12.0, 4.0],
        tex: [0.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOGGED_OUTER_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.2, -0.2, -2.2],
        size: [4.4, 12.4, 4.4],
        uv_size: [4.0, 12.0, 4.0],
        tex: [0.0, 16.0],
        mirror: true,
    }];

// Vanilla 26.1 ModelLayers.BOGGED_OUTER_LAYER:
// HumanoidModel.createMesh(new CubeDeformation(0.2F), 0.0F), 64x32.
pub(in crate::entity_models) const BOGGED_OUTER_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &BOGGED_OUTER_TEXTURED_HEAD,
        children: &BOGGED_OUTER_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &BOGGED_OUTER_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BOGGED_OUTER_TEXTURED_RIGHT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BOGGED_OUTER_TEXTURED_LEFT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BOGGED_OUTER_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BOGGED_OUTER_TEXTURED_LEFT_LEG,
        children: &[],
    },
];
