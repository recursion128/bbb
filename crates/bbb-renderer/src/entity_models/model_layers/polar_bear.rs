use super::{ModelCubeDesc, ModelPartDesc, PartPose, POLAR_BEAR_WHITE};

pub(in crate::entity_models) const ADULT_POLAR_BEAR_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-3.5, -3.0, -3.0],
        size: [7.0, 7.0, 7.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [-2.5, 1.0, -6.0],
        size: [5.0, 3.0, 3.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [-4.5, -4.0, -1.0],
        size: [2.0, 2.0, 1.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [2.5, -4.0, -1.0],
        size: [2.0, 2.0, 1.0],
        color: POLAR_BEAR_WHITE,
    },
];

pub(in crate::entity_models) const ADULT_POLAR_BEAR_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-5.0, -13.0, -7.0],
        size: [14.0, 14.0, 11.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [-4.0, -25.0, -7.0],
        size: [12.0, 12.0, 10.0],
        color: POLAR_BEAR_WHITE,
    },
];

pub(in crate::entity_models) const ADULT_POLAR_BEAR_HIND_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 10.0, 8.0],
        color: POLAR_BEAR_WHITE,
    }];

pub(in crate::entity_models) const ADULT_POLAR_BEAR_FRONT_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 10.0, 6.0],
        color: POLAR_BEAR_WHITE,
    }];

// Vanilla 26.1 ModelLayers.POLAR_BEAR: PolarBearModel.createBodyLayer()
// with LayerDefinition MeshTransformer.scaling(1.2F) applied at emission.
pub(in crate::entity_models) const ADULT_POLAR_BEAR_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 10.0, -16.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 9.0, 12.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.5, 14.0, 6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.5, 14.0, 6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.5, 14.0, -8.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.5, 14.0, -8.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_FRONT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_POLAR_BEAR_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -3.5, -6.0],
    size: [8.0, 7.0, 12.0],
    color: POLAR_BEAR_WHITE,
}];

pub(in crate::entity_models) const BABY_POLAR_BEAR_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-3.0, -2.625, -4.25],
        size: [6.0, 5.0, 4.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [-2.0, 0.375, -6.25],
        size: [4.0, 2.0, 2.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [-4.0, -3.625, -2.75],
        size: [2.0, 2.0, 1.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [2.0, -3.625, -2.75],
        size: [2.0, 2.0, 1.0],
        color: POLAR_BEAR_WHITE,
    },
];

pub(in crate::entity_models) const BABY_POLAR_BEAR_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -0.5, -1.5],
    size: [3.0, 3.0, 3.0],
    color: POLAR_BEAR_WHITE,
}];

// Vanilla 26.1 ModelLayers.POLAR_BEAR_BABY: BabyPolarBearModel.createBodyLayer().
pub(in crate::entity_models) const BABY_POLAR_BEAR_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 17.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.625, -5.75],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 21.5, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 21.5, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 21.5, -4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 21.5, -4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_LEG,
        children: &[],
    },
];
