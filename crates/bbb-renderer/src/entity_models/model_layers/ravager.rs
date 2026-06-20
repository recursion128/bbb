use super::{ModelCubeDesc, ModelPartDesc, PartPose, RAVAGER_GRAY};

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
