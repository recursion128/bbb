use super::{ModelCubeDesc, ModelPartDesc, PartPose, PART_POSE_ZERO, WITCH_HAT_COLOR, WITCH_ROBE};

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
