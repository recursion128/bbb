use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    RAVAGER_GRAY,
};

pub(in crate::entity_models) const MODEL_LAYER_RAVAGER: &str = "minecraft:ravager#main";

pub(in crate::entity_models) const RAVAGER_NECK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.0, -1.0, -18.0],
    size: [10.0, 10.0, 18.0],
    color: RAVAGER_GRAY,
}];

pub(in crate::entity_models) const RAVAGER_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-8.0, -20.0, -14.0],
        size: [16.0, 20.0, 16.0],
        color: RAVAGER_GRAY,
    },
    ModelCubeDesc {
        min: [-2.0, -6.0, -18.0],
        size: [4.0, 8.0, 4.0],
        color: RAVAGER_GRAY,
    },
];

pub(in crate::entity_models) const RAVAGER_HORN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -14.0, -2.0],
    size: [2.0, 14.0, 4.0],
    color: RAVAGER_GRAY,
}];

pub(in crate::entity_models) const RAVAGER_MOUTH: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-8.0, 0.0, -16.0],
    size: [16.0, 3.0, 16.0],
    color: RAVAGER_GRAY,
}];

pub(in crate::entity_models) const RAVAGER_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-7.0, -10.0, -7.0],
        size: [14.0, 16.0, 20.0],
        color: RAVAGER_GRAY,
    },
    ModelCubeDesc {
        min: [-6.0, 6.0, -7.0],
        size: [12.0, 13.0, 18.0],
        color: RAVAGER_GRAY,
    },
];

pub(in crate::entity_models) const RAVAGER_HIND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -4.0],
    size: [8.0, 37.0, 8.0],
    color: RAVAGER_GRAY,
}];

pub(in crate::entity_models) const RAVAGER_FRONT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -4.0],
    size: [8.0, 37.0, 8.0],
    color: RAVAGER_GRAY,
}];

pub(in crate::entity_models) const RAVAGER_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-10.0, -14.0, -8.0],
            rotation: [1.0995574, 0.0, 0.0],
        },
        cubes: &RAVAGER_HORN,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [8.0, -14.0, -8.0],
            rotation: [1.0995574, 0.0, 0.0],
        },
        cubes: &RAVAGER_HORN,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, 2.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &RAVAGER_MOUTH,
        children: &[],
    },
];

pub(in crate::entity_models) const RAVAGER_NECK_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, 16.0, -17.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &RAVAGER_HEAD,
    children: &RAVAGER_HEAD_CHILDREN,
}];

// Vanilla 26.1 ModelLayers.RAVAGER: RavagerModel.createBodyLayer().
pub(in crate::entity_models) const RAVAGER_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -7.0, 5.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &RAVAGER_NECK,
        children: &RAVAGER_NECK_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 1.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &RAVAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-8.0, -13.0, 18.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &RAVAGER_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [8.0, -13.0, 18.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &RAVAGER_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-8.0, -13.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &RAVAGER_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [8.0, -13.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &RAVAGER_FRONT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const RAVAGER_TEXTURED_NECK: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-5.0, -1.0, -18.0],
        size: [10.0, 10.0, 18.0],
        uv_size: [10.0, 10.0, 18.0],
        tex: [68.0, 73.0],
        mirror: false,
    }];

pub(in crate::entity_models) const RAVAGER_TEXTURED_HEAD: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-8.0, -20.0, -14.0],
        size: [16.0, 20.0, 16.0],
        uv_size: [16.0, 20.0, 16.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-2.0, -6.0, -18.0],
        size: [4.0, 8.0, 4.0],
        uv_size: [4.0, 8.0, 4.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const RAVAGER_TEXTURED_RIGHT_HORN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, -14.0, -2.0],
        size: [2.0, 14.0, 4.0],
        uv_size: [2.0, 14.0, 4.0],
        tex: [74.0, 55.0],
        mirror: false,
    }];

pub(in crate::entity_models) const RAVAGER_TEXTURED_LEFT_HORN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, -14.0, -2.0],
        size: [2.0, 14.0, 4.0],
        uv_size: [2.0, 14.0, 4.0],
        tex: [74.0, 55.0],
        mirror: true,
    }];

pub(in crate::entity_models) const RAVAGER_TEXTURED_MOUTH: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-8.0, 0.0, -16.0],
        size: [16.0, 3.0, 16.0],
        uv_size: [16.0, 3.0, 16.0],
        tex: [0.0, 36.0],
        mirror: false,
    }];

pub(in crate::entity_models) const RAVAGER_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-7.0, -10.0, -7.0],
        size: [14.0, 16.0, 20.0],
        uv_size: [14.0, 16.0, 20.0],
        tex: [0.0, 55.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-6.0, 6.0, -7.0],
        size: [12.0, 13.0, 18.0],
        uv_size: [12.0, 13.0, 18.0],
        tex: [0.0, 91.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const RAVAGER_TEXTURED_RIGHT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 0.0, -4.0],
        size: [8.0, 37.0, 8.0],
        uv_size: [8.0, 37.0, 8.0],
        tex: [96.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const RAVAGER_TEXTURED_LEFT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 0.0, -4.0],
        size: [8.0, 37.0, 8.0],
        uv_size: [8.0, 37.0, 8.0],
        tex: [96.0, 0.0],
        mirror: true,
    }];

pub(in crate::entity_models) const RAVAGER_TEXTURED_RIGHT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 0.0, -4.0],
        size: [8.0, 37.0, 8.0],
        uv_size: [8.0, 37.0, 8.0],
        tex: [64.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const RAVAGER_TEXTURED_LEFT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 0.0, -4.0],
        size: [8.0, 37.0, 8.0],
        uv_size: [8.0, 37.0, 8.0],
        tex: [64.0, 0.0],
        mirror: true,
    }];

pub(in crate::entity_models) const RAVAGER_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 3] = [
    TexturedModelPartDesc {
        pose: RAVAGER_HEAD_CHILDREN[0].pose,
        cubes: &RAVAGER_TEXTURED_RIGHT_HORN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: RAVAGER_HEAD_CHILDREN[1].pose,
        cubes: &RAVAGER_TEXTURED_LEFT_HORN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: RAVAGER_HEAD_CHILDREN[2].pose,
        cubes: &RAVAGER_TEXTURED_MOUTH,
        children: &[],
    },
];

pub(in crate::entity_models) const RAVAGER_TEXTURED_NECK_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: RAVAGER_NECK_CHILDREN[0].pose,
        cubes: &RAVAGER_TEXTURED_HEAD,
        children: &RAVAGER_TEXTURED_HEAD_CHILDREN,
    }];

pub(in crate::entity_models) const RAVAGER_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: RAVAGER_PARTS[0].pose,
        cubes: &RAVAGER_TEXTURED_NECK,
        children: &RAVAGER_TEXTURED_NECK_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: RAVAGER_PARTS[1].pose,
        cubes: &RAVAGER_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: RAVAGER_PARTS[2].pose,
        cubes: &RAVAGER_TEXTURED_RIGHT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: RAVAGER_PARTS[3].pose,
        cubes: &RAVAGER_TEXTURED_LEFT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: RAVAGER_PARTS[4].pose,
        cubes: &RAVAGER_TEXTURED_RIGHT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: RAVAGER_PARTS[5].pose,
        cubes: &RAVAGER_TEXTURED_LEFT_FRONT_LEG,
        children: &[],
    },
];
