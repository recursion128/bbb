use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    DONKEY_GRAY, HORSE_BROWN, PART_POSE_ZERO,
};

pub(in crate::entity_models) const ADULT_HORSE_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.05, -8.05, -17.05],
    size: [10.1, 10.1, 22.1],
    color: HORSE_BROWN,
}];

pub(in crate::entity_models) const ADULT_HORSE_TAIL: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, 0.0],
    size: [3.0, 14.0, 4.0],
    color: HORSE_BROWN,
}];

pub(in crate::entity_models) const ADULT_HORSE_BODY_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -5.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_TAIL,
        children: &[],
    }];

pub(in crate::entity_models) const ADULT_HORSE_NECK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.05, -6.0, -2.0],
    size: [4.0, 12.0, 7.0],
    color: HORSE_BROWN,
}];

pub(in crate::entity_models) const ADULT_HORSE_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -11.0, -2.0],
    size: [6.0, 5.0, 7.0],
    color: HORSE_BROWN,
}];

pub(in crate::entity_models) const ADULT_HORSE_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.551, -12.999, 4.001],
    size: [1.998, 2.998, 0.998],
    color: HORSE_BROWN,
}];

pub(in crate::entity_models) const ADULT_HORSE_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.549, -12.999, 4.001],
    size: [1.998, 2.998, 0.998],
    color: HORSE_BROWN,
}];

pub(in crate::entity_models) const ADULT_HORSE_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_RIGHT_EAR,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_HORSE_MANE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -11.0, 5.01],
    size: [2.0, 16.0, 2.0],
    color: HORSE_BROWN,
}];

pub(in crate::entity_models) const ADULT_HORSE_UPPER_MOUTH: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -11.0, -7.0],
    size: [4.0, 5.0, 5.0],
    color: HORSE_BROWN,
}];

pub(in crate::entity_models) const ADULT_HORSE_HEAD_PARTS_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_HEAD,
        children: &ADULT_HORSE_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_MANE,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_UPPER_MOUTH,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_HORSE_LEFT_HIND_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-3.0, -1.01, -1.0],
        size: [4.0, 11.0, 4.0],
        color: HORSE_BROWN,
    }];

pub(in crate::entity_models) const ADULT_HORSE_RIGHT_HIND_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-1.0, -1.01, -1.0],
        size: [4.0, 11.0, 4.0],
        color: HORSE_BROWN,
    }];

pub(in crate::entity_models) const ADULT_HORSE_LEFT_FRONT_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-3.0, -1.01, -1.9],
        size: [4.0, 11.0, 4.0],
        color: HORSE_BROWN,
    }];

pub(in crate::entity_models) const ADULT_HORSE_RIGHT_FRONT_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-1.0, -1.01, -1.9],
        size: [4.0, 11.0, 4.0],
        color: HORSE_BROWN,
    }];

// Vanilla 26.1 ModelLayers.HORSE:
// AbstractEquineModel.createBodyMesh(CubeDeformation.NONE) with
// LayerDefinitions' MeshTransformer.scaling(1.1F) applied by the emitter.
pub(in crate::entity_models) const ADULT_HORSE_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 11.0, 5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_BODY,
        children: &ADULT_HORSE_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, -12.0],
            rotation: [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_NECK,
        children: &ADULT_HORSE_HEAD_PARTS_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 14.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_LEFT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 14.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_RIGHT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 14.0, -10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_LEFT_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 14.0, -10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_RIGHT_FRONT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_HORSE_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -3.5, -7.0],
    size: [8.0, 7.0, 14.0],
    color: HORSE_BROWN,
}];

pub(in crate::entity_models) const BABY_HORSE_TAIL: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -1.5, -1.0],
    size: [3.0, 3.0, 8.0],
    color: HORSE_BROWN,
}];

pub(in crate::entity_models) const BABY_HORSE_BODY_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -1.0, 7.0],
        rotation: [-0.7418, 0.0, 0.0],
    },
    cubes: &BABY_HORSE_TAIL,
    children: &[],
}];

pub(in crate::entity_models) const BABY_HORSE_LEFT_HIND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -1.0, -1.5],
    size: [3.0, 9.0, 3.0],
    color: HORSE_BROWN,
}];

pub(in crate::entity_models) const BABY_HORSE_RIGHT_HIND_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-1.5, -1.0, -1.5],
        size: [3.0, 9.0, 3.0],
        color: HORSE_BROWN,
    }];

pub(in crate::entity_models) const BABY_HORSE_LEFT_FRONT_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-1.5, -1.0, -1.5],
        size: [3.0, 9.0, 3.0],
        color: HORSE_BROWN,
    }];

pub(in crate::entity_models) const BABY_HORSE_RIGHT_FRONT_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-1.5, -1.0, -1.5],
        size: [3.0, 9.0, 3.0],
        color: HORSE_BROWN,
    }];

pub(in crate::entity_models) const BABY_HORSE_NECK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -6.0, -2.0],
    size: [4.0, 8.0, 4.0],
    color: HORSE_BROWN,
}];

pub(in crate::entity_models) const BABY_HORSE_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -3.9484, -6.705],
    size: [6.0, 4.0, 9.0],
    color: HORSE_BROWN,
}];

pub(in crate::entity_models) const BABY_HORSE_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.5, -0.8],
    size: [2.0, 3.0, 1.0],
    color: HORSE_BROWN,
}];

pub(in crate::entity_models) const BABY_HORSE_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.5, -0.5],
    size: [2.0, 3.0, 1.0],
    color: HORSE_BROWN,
}];

pub(in crate::entity_models) const BABY_HORSE_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, -4.2484, 1.9451],
            rotation: [0.0, 0.0, 0.2618],
        },
        cubes: &BABY_HORSE_LEFT_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -4.2484, 1.645],
            rotation: [0.0, 0.0, -0.2618],
        },
        cubes: &BABY_HORSE_RIGHT_EAR,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_HORSE_HEAD_PARTS_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -6.0516, -0.2951],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HORSE_HEAD,
        children: &BABY_HORSE_HEAD_CHILDREN,
    }];

// Vanilla 26.1 ModelLayers.HORSE_BABY:
// BabyHorseModel.createBabyMesh(CubeDeformation.NONE), without livingHorseScale.
pub(in crate::entity_models) const BABY_HORSE_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HORSE_BODY,
        children: &BABY_HORSE_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.4, 16.0, 5.4],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HORSE_LEFT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.4, 16.0, 5.4],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HORSE_RIGHT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.4, 16.0, -5.4],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HORSE_LEFT_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.4, 16.0, -5.4],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HORSE_RIGHT_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 10.0, -6.0],
            rotation: [0.6109, 0.0, 0.0],
        },
        cubes: &BABY_HORSE_NECK,
        children: &BABY_HORSE_HEAD_PARTS_CHILDREN,
    },
];

// Textured undead-horse (skeleton / zombie horse) geometry. Vanilla `HorseModel` is shared by the
// living and undead horses, so these trees mirror the colored `ADULT_HORSE_PARTS` / `BABY_HORSE_PARTS`
// 1:1 — identical (deformed) `min`/`size` and poses (the poses are referenced from the colored consts
// to keep the two paths in lockstep) — and only add the per-cube `uv_size`/`tex`/`mirror` from
// `AbstractEquineModel.createBodyMesh` / `BabyHorseModel.createBabyMesh`. `uv_size` is the RAW
// `addBox` size (pre-deformation); the body (deform `0.05`) and ears (deform `−0.001`) therefore carry
// a `uv_size` that differs from their deformed `size`. The horse texture is 64×64.
pub(in crate::entity_models) const ADULT_HORSE_BODY_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-5.05, -8.05, -17.05],
        size: [10.1, 10.1, 22.1],
        uv_size: [10.0, 10.0, 22.0],
        tex: [0.0, 32.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_HORSE_TAIL_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, 0.0, 0.0],
        size: [3.0, 14.0, 4.0],
        uv_size: [3.0, 14.0, 4.0],
        tex: [42.0, 36.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_HORSE_BODY_CHILDREN_TEXTURED: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: ADULT_HORSE_BODY_CHILDREN[0].pose,
        cubes: &ADULT_HORSE_TAIL_TEXTURED,
        children: &[],
    }];

pub(in crate::entity_models) const ADULT_HORSE_NECK_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.05, -6.0, -2.0],
        size: [4.0, 12.0, 7.0],
        uv_size: [4.0, 12.0, 7.0],
        tex: [0.0, 35.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_HORSE_HEAD_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -11.0, -2.0],
        size: [6.0, 5.0, 7.0],
        uv_size: [6.0, 5.0, 7.0],
        tex: [0.0, 13.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_HORSE_EAR_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.551, -12.999, 4.001],
        size: [1.998, 2.998, 0.998],
        uv_size: [2.0, 3.0, 1.0],
        tex: [19.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_HORSE_RIGHT_EAR_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.549, -12.999, 4.001],
        size: [1.998, 2.998, 0.998],
        uv_size: [2.0, 3.0, 1.0],
        tex: [19.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_HORSE_HEAD_CHILDREN_TEXTURED: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_EAR_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_RIGHT_EAR_TEXTURED,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_HORSE_MANE_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -11.0, 5.01],
        size: [2.0, 16.0, 2.0],
        uv_size: [2.0, 16.0, 2.0],
        tex: [56.0, 36.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_HORSE_UPPER_MOUTH_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -11.0, -7.0],
        size: [4.0, 5.0, 5.0],
        uv_size: [4.0, 5.0, 5.0],
        tex: [0.0, 25.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_HORSE_HEAD_PARTS_CHILDREN_TEXTURED:
    [TexturedModelPartDesc; 3] = [
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_HEAD_TEXTURED,
        children: &ADULT_HORSE_HEAD_CHILDREN_TEXTURED,
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_MANE_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_UPPER_MOUTH_TEXTURED,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_HORSE_LEFT_HIND_LEG_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -1.01, -1.0],
        size: [4.0, 11.0, 4.0],
        uv_size: [4.0, 11.0, 4.0],
        tex: [48.0, 21.0],
        mirror: true,
    }];

pub(in crate::entity_models) const ADULT_HORSE_RIGHT_HIND_LEG_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -1.01, -1.0],
        size: [4.0, 11.0, 4.0],
        uv_size: [4.0, 11.0, 4.0],
        tex: [48.0, 21.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_HORSE_LEFT_FRONT_LEG_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -1.01, -1.9],
        size: [4.0, 11.0, 4.0],
        uv_size: [4.0, 11.0, 4.0],
        tex: [48.0, 21.0],
        mirror: true,
    }];

pub(in crate::entity_models) const ADULT_HORSE_RIGHT_FRONT_LEG_TEXTURED: [TexturedModelCubeDesc;
    1] = [TexturedModelCubeDesc {
    min: [-1.0, -1.01, -1.9],
    size: [4.0, 11.0, 4.0],
    uv_size: [4.0, 11.0, 4.0],
    tex: [48.0, 21.0],
    mirror: false,
}];

pub(in crate::entity_models) const ADULT_HORSE_PARTS_TEXTURED: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: ADULT_HORSE_PARTS[0].pose,
        cubes: &ADULT_HORSE_BODY_TEXTURED,
        children: &ADULT_HORSE_BODY_CHILDREN_TEXTURED,
    },
    TexturedModelPartDesc {
        pose: ADULT_HORSE_PARTS[1].pose,
        cubes: &ADULT_HORSE_NECK_TEXTURED,
        children: &ADULT_HORSE_HEAD_PARTS_CHILDREN_TEXTURED,
    },
    TexturedModelPartDesc {
        pose: ADULT_HORSE_PARTS[2].pose,
        cubes: &ADULT_HORSE_LEFT_HIND_LEG_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_HORSE_PARTS[3].pose,
        cubes: &ADULT_HORSE_RIGHT_HIND_LEG_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_HORSE_PARTS[4].pose,
        cubes: &ADULT_HORSE_LEFT_FRONT_LEG_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_HORSE_PARTS[5].pose,
        cubes: &ADULT_HORSE_RIGHT_FRONT_LEG_TEXTURED,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_HORSE_BODY_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -3.5, -7.0],
        size: [8.0, 7.0, 14.0],
        uv_size: [8.0, 7.0, 14.0],
        tex: [0.0, 13.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_HORSE_TAIL_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, -1.5, -1.0],
        size: [3.0, 3.0, 8.0],
        uv_size: [3.0, 3.0, 8.0],
        tex: [24.0, 34.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_HORSE_BODY_CHILDREN_TEXTURED: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: BABY_HORSE_BODY_CHILDREN[0].pose,
        cubes: &BABY_HORSE_TAIL_TEXTURED,
        children: &[],
    }];

pub(in crate::entity_models) const BABY_HORSE_LEFT_HIND_LEG_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, -1.0, -1.5],
        size: [3.0, 9.0, 3.0],
        uv_size: [3.0, 9.0, 3.0],
        tex: [12.0, 46.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_HORSE_RIGHT_HIND_LEG_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, -1.0, -1.5],
        size: [3.0, 9.0, 3.0],
        uv_size: [3.0, 9.0, 3.0],
        tex: [0.0, 46.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_HORSE_LEFT_FRONT_LEG_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, -1.0, -1.5],
        size: [3.0, 9.0, 3.0],
        uv_size: [3.0, 9.0, 3.0],
        tex: [12.0, 34.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_HORSE_RIGHT_FRONT_LEG_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, -1.0, -1.5],
        size: [3.0, 9.0, 3.0],
        uv_size: [3.0, 9.0, 3.0],
        tex: [0.0, 34.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_HORSE_NECK_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -6.0, -2.0],
        size: [4.0, 8.0, 4.0],
        uv_size: [4.0, 8.0, 4.0],
        tex: [30.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_HORSE_HEAD_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -3.9484, -6.705],
        size: [6.0, 4.0, 9.0],
        uv_size: [6.0, 4.0, 9.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_HORSE_LEFT_EAR_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -2.5, -0.8],
        size: [2.0, 3.0, 1.0],
        uv_size: [2.0, 3.0, 1.0],
        tex: [0.0, 4.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_HORSE_RIGHT_EAR_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -2.5, -0.5],
        size: [2.0, 3.0, 1.0],
        uv_size: [2.0, 3.0, 1.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_HORSE_HEAD_CHILDREN_TEXTURED: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: BABY_HORSE_HEAD_CHILDREN[0].pose,
        cubes: &BABY_HORSE_LEFT_EAR_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_HORSE_HEAD_CHILDREN[1].pose,
        cubes: &BABY_HORSE_RIGHT_EAR_TEXTURED,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_HORSE_HEAD_PARTS_CHILDREN_TEXTURED:
    [TexturedModelPartDesc; 1] = [TexturedModelPartDesc {
    pose: BABY_HORSE_HEAD_PARTS_CHILDREN[0].pose,
    cubes: &BABY_HORSE_HEAD_TEXTURED,
    children: &BABY_HORSE_HEAD_CHILDREN_TEXTURED,
}];

pub(in crate::entity_models) const BABY_HORSE_PARTS_TEXTURED: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: BABY_HORSE_PARTS[0].pose,
        cubes: &BABY_HORSE_BODY_TEXTURED,
        children: &BABY_HORSE_BODY_CHILDREN_TEXTURED,
    },
    TexturedModelPartDesc {
        pose: BABY_HORSE_PARTS[1].pose,
        cubes: &BABY_HORSE_LEFT_HIND_LEG_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_HORSE_PARTS[2].pose,
        cubes: &BABY_HORSE_RIGHT_HIND_LEG_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_HORSE_PARTS[3].pose,
        cubes: &BABY_HORSE_LEFT_FRONT_LEG_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_HORSE_PARTS[4].pose,
        cubes: &BABY_HORSE_RIGHT_FRONT_LEG_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_HORSE_PARTS[5].pose,
        cubes: &BABY_HORSE_NECK_TEXTURED,
        children: &BABY_HORSE_HEAD_PARTS_CHILDREN_TEXTURED,
    },
];

pub(in crate::entity_models) const ADULT_DONKEY_CHEST: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 8.0, 3.0],
    color: DONKEY_GRAY,
}];

pub(in crate::entity_models) const ADULT_DONKEY_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -7.0, 0.0],
    size: [2.0, 7.0, 1.0],
    color: DONKEY_GRAY,
}];

pub(in crate::entity_models) const ADULT_DONKEY_BODY_CHILDREN_WITH_CHEST: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -5.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_TAIL,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [6.0, -8.0, 0.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &ADULT_DONKEY_CHEST,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-6.0, -8.0, 0.0],
            rotation: [0.0, std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &ADULT_DONKEY_CHEST,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_DONKEY_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [1.25, -10.0, 4.0],
            rotation: [0.2617994, 0.0, 0.2617994],
        },
        cubes: &ADULT_DONKEY_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.25, -10.0, 4.0],
            rotation: [0.2617994, 0.0, -0.2617994],
        },
        cubes: &ADULT_DONKEY_EAR,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_DONKEY_HEAD_PARTS_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_HEAD,
        children: &ADULT_DONKEY_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_MANE,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_UPPER_MOUTH,
        children: &[],
    },
];

// Vanilla 26.1 ModelLayers.DONKEY and ModelLayers.MULE:
// AbstractEquineModel.createBodyMesh(CubeDeformation.NONE), DonkeyModel.modifyMesh(),
// then MeshTransformer.scaling(0.87F or 0.92F) applied by the emitter.
pub(in crate::entity_models) const ADULT_DONKEY_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 11.0, 5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_BODY,
        children: &ADULT_HORSE_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, -12.0],
            rotation: [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_NECK,
        children: &ADULT_DONKEY_HEAD_PARTS_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 14.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_LEFT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 14.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_RIGHT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 14.0, -10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_LEFT_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 14.0, -10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_RIGHT_FRONT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_DONKEY_PARTS_WITH_CHEST: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 11.0, 5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_BODY,
        children: &ADULT_DONKEY_BODY_CHILDREN_WITH_CHEST,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, -12.0],
            rotation: [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_NECK,
        children: &ADULT_DONKEY_HEAD_PARTS_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 14.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_LEFT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 14.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_RIGHT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 14.0, -10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_LEFT_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 14.0, -10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_RIGHT_FRONT_LEG,
        children: &[],
    },
];

// Textured adult donkey / mule (skeleton-equine geometry). Vanilla `DonkeyModel` is
// `AbstractEquineModel.createBodyMesh(NONE)` + `modifyMesh` (which REPLACES the horse ears with bigger
// donkey ears `texOffs(0,12)` and adds the two `texOffs(26,21)` chest boxes to the body), on the 64×64
// `donkey.png` / `mule.png`. The textured trees reuse the horse textured cubes 1:1 for body / neck /
// head / mane / upper-mouth / legs / tail (shared `createBodyMesh`), add only the donkey ear and chest
// cubes, and reference the colored donkey poses for lockstep.
pub(in crate::entity_models) const ADULT_DONKEY_EAR_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -7.0, 0.0],
        size: [2.0, 7.0, 1.0],
        uv_size: [2.0, 7.0, 1.0],
        tex: [0.0, 12.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_DONKEY_CHEST_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 0.0, -2.0],
        size: [8.0, 8.0, 3.0],
        uv_size: [8.0, 8.0, 3.0],
        tex: [26.0, 21.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_DONKEY_HEAD_CHILDREN_TEXTURED: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: ADULT_DONKEY_HEAD_CHILDREN[0].pose,
        cubes: &ADULT_DONKEY_EAR_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_DONKEY_HEAD_CHILDREN[1].pose,
        cubes: &ADULT_DONKEY_EAR_TEXTURED,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_DONKEY_HEAD_PARTS_CHILDREN_TEXTURED:
    [TexturedModelPartDesc; 3] = [
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_HEAD_TEXTURED,
        children: &ADULT_DONKEY_HEAD_CHILDREN_TEXTURED,
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_MANE_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_UPPER_MOUTH_TEXTURED,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_DONKEY_BODY_CHILDREN_WITH_CHEST_TEXTURED:
    [TexturedModelPartDesc; 3] = [
    TexturedModelPartDesc {
        pose: ADULT_DONKEY_BODY_CHILDREN_WITH_CHEST[0].pose,
        cubes: &ADULT_HORSE_TAIL_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_DONKEY_BODY_CHILDREN_WITH_CHEST[1].pose,
        cubes: &ADULT_DONKEY_CHEST_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_DONKEY_BODY_CHILDREN_WITH_CHEST[2].pose,
        cubes: &ADULT_DONKEY_CHEST_TEXTURED,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_DONKEY_PARTS_TEXTURED: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: ADULT_DONKEY_PARTS[0].pose,
        cubes: &ADULT_HORSE_BODY_TEXTURED,
        children: &ADULT_HORSE_BODY_CHILDREN_TEXTURED,
    },
    TexturedModelPartDesc {
        pose: ADULT_DONKEY_PARTS[1].pose,
        cubes: &ADULT_HORSE_NECK_TEXTURED,
        children: &ADULT_DONKEY_HEAD_PARTS_CHILDREN_TEXTURED,
    },
    TexturedModelPartDesc {
        pose: ADULT_DONKEY_PARTS[2].pose,
        cubes: &ADULT_HORSE_LEFT_HIND_LEG_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_DONKEY_PARTS[3].pose,
        cubes: &ADULT_HORSE_RIGHT_HIND_LEG_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_DONKEY_PARTS[4].pose,
        cubes: &ADULT_HORSE_LEFT_FRONT_LEG_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_DONKEY_PARTS[5].pose,
        cubes: &ADULT_HORSE_RIGHT_FRONT_LEG_TEXTURED,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_DONKEY_PARTS_WITH_CHEST_TEXTURED: [TexturedModelPartDesc;
    6] = [
    TexturedModelPartDesc {
        pose: ADULT_DONKEY_PARTS_WITH_CHEST[0].pose,
        cubes: &ADULT_HORSE_BODY_TEXTURED,
        children: &ADULT_DONKEY_BODY_CHILDREN_WITH_CHEST_TEXTURED,
    },
    TexturedModelPartDesc {
        pose: ADULT_DONKEY_PARTS_WITH_CHEST[1].pose,
        cubes: &ADULT_HORSE_NECK_TEXTURED,
        children: &ADULT_DONKEY_HEAD_PARTS_CHILDREN_TEXTURED,
    },
    TexturedModelPartDesc {
        pose: ADULT_DONKEY_PARTS_WITH_CHEST[2].pose,
        cubes: &ADULT_HORSE_LEFT_HIND_LEG_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_DONKEY_PARTS_WITH_CHEST[3].pose,
        cubes: &ADULT_HORSE_RIGHT_HIND_LEG_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_DONKEY_PARTS_WITH_CHEST[4].pose,
        cubes: &ADULT_HORSE_LEFT_FRONT_LEG_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_DONKEY_PARTS_WITH_CHEST[5].pose,
        cubes: &ADULT_HORSE_RIGHT_FRONT_LEG_TEXTURED,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_DONKEY_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.0, -3.0, -7.0],
    size: [8.0, 6.0, 14.0],
    color: DONKEY_GRAY,
}];

pub(in crate::entity_models) const BABY_DONKEY_TAIL_R1: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, -1.0, -0.5],
    size: [3.0, 3.0, 8.0],
    color: DONKEY_GRAY,
}];

pub(in crate::entity_models) const BABY_DONKEY_TAIL_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 0.0, 0.0],
            rotation: [-0.7418, 0.0, 0.0],
        },
        cubes: &BABY_DONKEY_TAIL_R1,
        children: &[],
    }];

pub(in crate::entity_models) const BABY_DONKEY_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, -1.5, -1.5],
    size: [3.0, 8.0, 3.0],
    color: DONKEY_GRAY,
}];

pub(in crate::entity_models) const BABY_DONKEY_NECK_R1: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -6.0, -3.0],
    size: [4.0, 8.0, 4.0],
    color: DONKEY_GRAY,
}];

pub(in crate::entity_models) const BABY_DONKEY_HEAD_R1: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -3.6, -8.4],
    size: [6.0, 4.0, 9.0],
    color: DONKEY_GRAY,
}];

pub(in crate::entity_models) const BABY_DONKEY_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -6.5, -0.3],
    size: [2.0, 7.0, 1.0],
    color: DONKEY_GRAY,
}];

pub(in crate::entity_models) const BABY_DONKEY_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -1.0, 1.0],
            rotation: [0.3927, 0.0, 0.0],
        },
        cubes: &BABY_DONKEY_HEAD_R1,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, -3.5, -1.0],
            rotation: [0.48, 0.0, 0.48],
        },
        cubes: &BABY_DONKEY_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -3.5, -1.0],
            rotation: [0.48, 0.0, -0.48],
        },
        cubes: &BABY_DONKEY_EAR,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_DONKEY_HEAD_PARTS_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 0.0, 0.0],
            rotation: [0.3927, 0.0, 0.0],
        },
        cubes: &BABY_DONKEY_NECK_R1,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -5.0, -3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_DONKEY_HEAD_CHILDREN,
    },
];

pub(in crate::entity_models) const BABY_DONKEY_BODY_CHILDREN: [ModelPartDesc; 8] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -1.5, 6.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_DONKEY_TAIL_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.25, 3.5, 5.25],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_DONKEY_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.4, 3.5, 5.4],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_DONKEY_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.4, 3.5, -5.3],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_DONKEY_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.4, 3.5, -5.4],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_DONKEY_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -3.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_DONKEY_HEAD_PARTS_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 10.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 10.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &[],
    },
];

// Vanilla 26.1 ModelLayers.DONKEY_BABY and ModelLayers.MULE_BABY:
// BabyDonkeyModel.createBabyLayer(); both families share geometry and differ by texture.
pub(in crate::entity_models) const BABY_DONKEY_PARTS: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [1.0, 14.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &BABY_DONKEY_BODY,
    children: &BABY_DONKEY_BODY_CHILDREN,
}];

// Textured baby donkey / mule. Vanilla `BabyDonkeyModel.createBabyLayer()` (64×64) is a DISTINCT
// re-parented mesh (legs / head_parts / tail nested UNDER the body), NOT the horse cubes, and its
// `setupAnim` forces `xRot = −30°` — so it emits STATIC on both paths (the equine gait / head look /
// tail lift is deferred). The four legs and two ears carry DIFFERENT texOffs (and the right ear is
// mirrored), so each gets its own textured cube. The tree mirrors the colored `BABY_DONKEY_*`
// structure 1:1 (identical `min`/`size`, poses referenced from the colored consts). Donkey/mule share
// the geometry and differ only by texture.
pub(in crate::entity_models) const BABY_DONKEY_BODY_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-5.0, -3.0, -7.0],
        size: [8.0, 6.0, 14.0],
        uv_size: [8.0, 6.0, 14.0],
        tex: [0.0, 13.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_DONKEY_TAIL_R1_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.5, -1.0, -0.5],
        size: [3.0, 3.0, 8.0],
        uv_size: [3.0, 3.0, 8.0],
        tex: [24.0, 33.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_DONKEY_TAIL_CHILDREN_TEXTURED: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: BABY_DONKEY_TAIL_CHILDREN[0].pose,
        cubes: &BABY_DONKEY_TAIL_R1_TEXTURED,
        children: &[],
    }];

pub(in crate::entity_models) const BABY_DONKEY_LEFT_HIND_LEG_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.5, -1.5, -1.5],
        size: [3.0, 8.0, 3.0],
        uv_size: [3.0, 8.0, 3.0],
        tex: [12.0, 44.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_DONKEY_RIGHT_HIND_LEG_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.5, -1.5, -1.5],
        size: [3.0, 8.0, 3.0],
        uv_size: [3.0, 8.0, 3.0],
        tex: [0.0, 44.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_DONKEY_LEFT_FRONT_LEG_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.5, -1.5, -1.5],
        size: [3.0, 8.0, 3.0],
        uv_size: [3.0, 8.0, 3.0],
        tex: [12.0, 33.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_DONKEY_RIGHT_FRONT_LEG_TEXTURED: [TexturedModelCubeDesc;
    1] = [TexturedModelCubeDesc {
    min: [-2.5, -1.5, -1.5],
    size: [3.0, 8.0, 3.0],
    uv_size: [3.0, 8.0, 3.0],
    tex: [0.0, 33.0],
    mirror: false,
}];

pub(in crate::entity_models) const BABY_DONKEY_NECK_R1_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -6.0, -3.0],
        size: [4.0, 8.0, 4.0],
        uv_size: [4.0, 8.0, 4.0],
        tex: [30.0, 9.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_DONKEY_HEAD_R1_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -3.6, -8.4],
        size: [6.0, 4.0, 9.0],
        uv_size: [6.0, 4.0, 9.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_DONKEY_LEFT_EAR_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -6.5, -0.3],
        size: [2.0, 7.0, 1.0],
        uv_size: [2.0, 7.0, 1.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_DONKEY_RIGHT_EAR_TEXTURED: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -6.5, -0.3],
        size: [2.0, 7.0, 1.0],
        uv_size: [2.0, 7.0, 1.0],
        tex: [22.0, 0.0],
        mirror: true,
    }];

pub(in crate::entity_models) const BABY_DONKEY_HEAD_CHILDREN_TEXTURED: [TexturedModelPartDesc; 3] = [
    TexturedModelPartDesc {
        pose: BABY_DONKEY_HEAD_CHILDREN[0].pose,
        cubes: &BABY_DONKEY_HEAD_R1_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_DONKEY_HEAD_CHILDREN[1].pose,
        cubes: &BABY_DONKEY_LEFT_EAR_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_DONKEY_HEAD_CHILDREN[2].pose,
        cubes: &BABY_DONKEY_RIGHT_EAR_TEXTURED,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_DONKEY_HEAD_PARTS_CHILDREN_TEXTURED:
    [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: BABY_DONKEY_HEAD_PARTS_CHILDREN[0].pose,
        cubes: &BABY_DONKEY_NECK_R1_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_DONKEY_HEAD_PARTS_CHILDREN[1].pose,
        cubes: &[],
        children: &BABY_DONKEY_HEAD_CHILDREN_TEXTURED,
    },
];

pub(in crate::entity_models) const BABY_DONKEY_BODY_CHILDREN_TEXTURED: [TexturedModelPartDesc; 8] = [
    TexturedModelPartDesc {
        pose: BABY_DONKEY_BODY_CHILDREN[0].pose,
        cubes: &[],
        children: &BABY_DONKEY_TAIL_CHILDREN_TEXTURED,
    },
    TexturedModelPartDesc {
        pose: BABY_DONKEY_BODY_CHILDREN[1].pose,
        cubes: &BABY_DONKEY_LEFT_HIND_LEG_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_DONKEY_BODY_CHILDREN[2].pose,
        cubes: &BABY_DONKEY_RIGHT_HIND_LEG_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_DONKEY_BODY_CHILDREN[3].pose,
        cubes: &BABY_DONKEY_LEFT_FRONT_LEG_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_DONKEY_BODY_CHILDREN[4].pose,
        cubes: &BABY_DONKEY_RIGHT_FRONT_LEG_TEXTURED,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_DONKEY_BODY_CHILDREN[5].pose,
        cubes: &[],
        children: &BABY_DONKEY_HEAD_PARTS_CHILDREN_TEXTURED,
    },
    TexturedModelPartDesc {
        pose: BABY_DONKEY_BODY_CHILDREN[6].pose,
        cubes: &[],
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_DONKEY_BODY_CHILDREN[7].pose,
        cubes: &[],
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_DONKEY_PARTS_TEXTURED: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: BABY_DONKEY_PARTS[0].pose,
        cubes: &BABY_DONKEY_BODY_TEXTURED,
        children: &BABY_DONKEY_BODY_CHILDREN_TEXTURED,
    }];
