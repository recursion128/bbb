use super::{ModelCubeDesc, ModelPartDesc, PartPose, SPIDER_DARK};

pub(in crate::entity_models) const SPIDER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -4.0, -8.0],
    size: [8.0, 8.0, 8.0],
    color: SPIDER_DARK,
}];

pub(in crate::entity_models) const SPIDER_BODY_0: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -3.0, -3.0],
    size: [6.0, 6.0, 6.0],
    color: SPIDER_DARK,
}];

pub(in crate::entity_models) const SPIDER_BODY_1: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.0, -4.0, -6.0],
    size: [10.0, 8.0, 12.0],
    color: SPIDER_DARK,
}];

pub(in crate::entity_models) const SPIDER_RIGHT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-15.0, -1.0, -1.0],
    size: [16.0, 2.0, 2.0],
    color: SPIDER_DARK,
}];

pub(in crate::entity_models) const SPIDER_LEFT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -1.0],
    size: [16.0, 2.0, 2.0],
    color: SPIDER_DARK,
}];

// Vanilla 26.1 SpiderModel.createSpiderBodyLayer().
pub(in crate::entity_models) const SPIDER_PARTS: [ModelPartDesc; 11] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.0, -3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SPIDER_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SPIDER_BODY_0,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.0, 9.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SPIDER_BODY_1,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 15.0, 2.0],
            rotation: [
                0.0,
                std::f32::consts::FRAC_PI_4,
                -std::f32::consts::FRAC_PI_4,
            ],
        },
        cubes: &SPIDER_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 15.0, 2.0],
            rotation: [
                0.0,
                -std::f32::consts::FRAC_PI_4,
                std::f32::consts::FRAC_PI_4,
            ],
        },
        cubes: &SPIDER_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 15.0, 1.0],
            rotation: [0.0, std::f32::consts::FRAC_PI_8, -0.58119464],
        },
        cubes: &SPIDER_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 15.0, 1.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_8, 0.58119464],
        },
        cubes: &SPIDER_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 15.0, 0.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_8, -0.58119464],
        },
        cubes: &SPIDER_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 15.0, 0.0],
            rotation: [0.0, std::f32::consts::FRAC_PI_8, 0.58119464],
        },
        cubes: &SPIDER_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 15.0, -1.0],
            rotation: [
                0.0,
                -std::f32::consts::FRAC_PI_4,
                -std::f32::consts::FRAC_PI_4,
            ],
        },
        cubes: &SPIDER_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 15.0, -1.0],
            rotation: [
                0.0,
                std::f32::consts::FRAC_PI_4,
                std::f32::consts::FRAC_PI_4,
            ],
        },
        cubes: &SPIDER_LEFT_LEG,
        children: &[],
    },
];
