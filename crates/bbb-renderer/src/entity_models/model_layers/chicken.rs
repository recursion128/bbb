use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    CHICKEN_BEAK, CHICKEN_LEG, CHICKEN_RED, CHICKEN_WHITE, CHICKEN_WING, PART_POSE_ZERO,
};

pub(in crate::entity_models) const ADULT_CHICKEN_BEAK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -4.0, -4.0],
    size: [4.0, 2.0, 2.0],
    color: CHICKEN_BEAK,
}];

pub(in crate::entity_models) const ADULT_CHICKEN_RED_THING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -3.0],
    size: [2.0, 2.0, 2.0],
    color: CHICKEN_RED,
}];

pub(in crate::entity_models) const ADULT_CHICKEN_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_CHICKEN_BEAK,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_CHICKEN_RED_THING,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_CHICKEN_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -6.0, -2.0],
    size: [4.0, 6.0, 3.0],
    color: CHICKEN_WHITE,
}];

pub(in crate::entity_models) const ADULT_CHICKEN_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -4.0, -3.0],
    size: [6.0, 8.0, 6.0],
    color: CHICKEN_WHITE,
}];

pub(in crate::entity_models) const COLD_CHICKEN_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-2.0, -6.0, -2.0],
        size: [4.0, 6.0, 3.0],
        color: CHICKEN_WHITE,
    },
    ModelCubeDesc {
        min: [-3.0, -7.0, -2.015],
        size: [6.0, 3.0, 4.0],
        color: CHICKEN_WING,
    },
];

pub(in crate::entity_models) const COLD_CHICKEN_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-3.0, -4.0, -3.0],
        size: [6.0, 8.0, 6.0],
        color: CHICKEN_WHITE,
    },
    ModelCubeDesc {
        min: [0.0, 3.0, -1.0],
        size: [0.0, 3.0, 5.0],
        color: CHICKEN_WING,
    },
];

pub(in crate::entity_models) const ADULT_CHICKEN_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -3.0],
    size: [3.0, 5.0, 3.0],
    color: CHICKEN_LEG,
}];

pub(in crate::entity_models) const ADULT_CHICKEN_RIGHT_WING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, -3.0],
    size: [1.0, 4.0, 6.0],
    color: CHICKEN_WING,
}];

pub(in crate::entity_models) const ADULT_CHICKEN_LEFT_WING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -3.0],
    size: [1.0, 4.0, 6.0],
    color: CHICKEN_WING,
}];

pub(in crate::entity_models) const ADULT_CHICKEN_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_HEAD,
        children: &ADULT_CHICKEN_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 16.0, 0.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 19.0, 1.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 19.0, 1.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_RIGHT_WING,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_LEFT_WING,
        children: &[],
    },
];

pub(in crate::entity_models) const COLD_CHICKEN_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &COLD_CHICKEN_HEAD,
        children: &ADULT_CHICKEN_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 16.0, 0.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &COLD_CHICKEN_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 19.0, 1.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 19.0, 1.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_RIGHT_WING,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_LEFT_WING,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_CHICKEN_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-2.0, -2.25, -0.75],
        size: [4.0, 4.0, 4.0],
        color: CHICKEN_WHITE,
    },
    ModelCubeDesc {
        min: [-1.0, -0.25, -1.75],
        size: [2.0, 1.0, 1.0],
        color: CHICKEN_BEAK,
    },
];

pub(in crate::entity_models) const BABY_CHICKEN_LEFT_LEG: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-0.5, 0.0, 0.0],
        size: [1.0, 2.0, 0.0],
        color: CHICKEN_LEG,
    },
    ModelCubeDesc {
        min: [-0.5, 2.0, -1.0],
        size: [1.0, 0.0, 1.0],
        color: CHICKEN_LEG,
    },
];

pub(in crate::entity_models) const BABY_CHICKEN_RIGHT_LEG: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-0.5, 0.0, 0.0],
        size: [1.0, 2.0, 0.0],
        color: CHICKEN_LEG,
    },
    ModelCubeDesc {
        min: [-0.5, 2.0, -1.0],
        size: [1.0, 0.0, 1.0],
        color: CHICKEN_LEG,
    },
];

pub(in crate::entity_models) const BABY_CHICKEN_RIGHT_WING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, -1.0],
    size: [1.0, 0.0, 2.0],
    color: CHICKEN_WING,
}];

pub(in crate::entity_models) const BABY_CHICKEN_LEFT_WING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [1.0, 0.0, 2.0],
    color: CHICKEN_WING,
}];

pub(in crate::entity_models) const BABY_CHICKEN_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 20.25, -1.25],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CHICKEN_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 22.0, 0.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CHICKEN_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 22.0, 0.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CHICKEN_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CHICKEN_RIGHT_WING,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CHICKEN_LEFT_WING,
        children: &[],
    },
];

/// The two leg part indices in the chicken body layers. The adult and cold layers
/// list head and body at `0`/`1` then the legs at `[2, 3]`; the baby layer has no head
/// (its beak is baked into the body cube), so the body is at `0` and the legs at
/// `[1, 2]`. [`super::humanoid_leg_swing_pose`] resolves each leg's phase from its `x`
/// sign, so only the slot positions differ.
pub(in crate::entity_models) fn chicken_leg_part_indices(baby: bool) -> [usize; 2] {
    if baby {
        [1, 2]
    } else {
        [2, 3]
    }
}

pub(in crate::entity_models) const MODEL_LAYER_CHICKEN: &str = "minecraft:chicken#main";
pub(in crate::entity_models) const MODEL_LAYER_CHICKEN_BABY: &str = "minecraft:chicken_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_COLD_CHICKEN: &str = "minecraft:cold_chicken#main";

pub(in crate::entity_models) const ADULT_CHICKEN_TEXTURED_BEAK: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -4.0, -4.0],
        size: [4.0, 2.0, 2.0],
        uv_size: [4.0, 2.0, 2.0],
        tex: [14.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_CHICKEN_TEXTURED_RED_THING: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -2.0, -3.0],
        size: [2.0, 2.0, 2.0],
        uv_size: [2.0, 2.0, 2.0],
        tex: [14.0, 4.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_CHICKEN_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc;
    2] = [
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_CHICKEN_TEXTURED_BEAK,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_CHICKEN_TEXTURED_RED_THING,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_CHICKEN_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -6.0, -2.0],
        size: [4.0, 6.0, 3.0],
        uv_size: [4.0, 6.0, 3.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_CHICKEN_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -4.0, -3.0],
        size: [6.0, 8.0, 6.0],
        uv_size: [6.0, 8.0, 6.0],
        tex: [0.0, 9.0],
        mirror: false,
    }];

pub(in crate::entity_models) const COLD_CHICKEN_TEXTURED_HEAD: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-2.0, -6.0, -2.0],
        size: [4.0, 6.0, 3.0],
        uv_size: [4.0, 6.0, 3.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-3.0, -7.0, -2.015],
        size: [6.0, 3.0, 4.0],
        uv_size: [6.0, 3.0, 4.0],
        tex: [44.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const COLD_CHICKEN_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-3.0, -4.0, -3.0],
        size: [6.0, 8.0, 6.0],
        uv_size: [6.0, 8.0, 6.0],
        tex: [0.0, 9.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [0.0, 3.0, -1.0],
        size: [0.0, 3.0, 5.0],
        uv_size: [0.0, 3.0, 5.0],
        tex: [38.0, 9.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const ADULT_CHICKEN_TEXTURED_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -3.0],
        size: [3.0, 5.0, 3.0],
        uv_size: [3.0, 5.0, 3.0],
        tex: [26.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_CHICKEN_TEXTURED_RIGHT_WING: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, -3.0],
        size: [1.0, 4.0, 6.0],
        uv_size: [1.0, 4.0, 6.0],
        tex: [24.0, 13.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_CHICKEN_TEXTURED_LEFT_WING: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -3.0],
        size: [1.0, 4.0, 6.0],
        uv_size: [1.0, 4.0, 6.0],
        tex: [24.0, 13.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_CHICKEN_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_TEXTURED_HEAD,
        children: &ADULT_CHICKEN_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 16.0, 0.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 19.0, 1.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [1.0, 19.0, 1.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_TEXTURED_RIGHT_WING,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [4.0, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_TEXTURED_LEFT_WING,
        children: &[],
    },
];

pub(in crate::entity_models) const COLD_CHICKEN_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &COLD_CHICKEN_TEXTURED_HEAD,
        children: &ADULT_CHICKEN_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 16.0, 0.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &COLD_CHICKEN_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 19.0, 1.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [1.0, 19.0, 1.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_TEXTURED_RIGHT_WING,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [4.0, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_TEXTURED_LEFT_WING,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_CHICKEN_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-2.0, -2.25, -0.75],
        size: [4.0, 4.0, 4.0],
        uv_size: [4.0, 4.0, 4.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-1.0, -0.25, -1.75],
        size: [2.0, 1.0, 1.0],
        uv_size: [2.0, 1.0, 1.0],
        tex: [10.0, 8.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BABY_CHICKEN_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-0.5, 0.0, 0.0],
        size: [1.0, 2.0, 0.0],
        uv_size: [1.0, 2.0, 0.0],
        tex: [2.0, 2.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-0.5, 2.0, -1.0],
        size: [1.0, 0.0, 1.0],
        uv_size: [1.0, 0.0, 1.0],
        tex: [0.0, 1.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BABY_CHICKEN_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-0.5, 0.0, 0.0],
        size: [1.0, 2.0, 0.0],
        uv_size: [1.0, 2.0, 0.0],
        tex: [0.0, 2.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-0.5, 2.0, -1.0],
        size: [1.0, 0.0, 1.0],
        uv_size: [1.0, 0.0, 1.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BABY_CHICKEN_TEXTURED_RIGHT_WING: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, -1.0],
        size: [1.0, 0.0, 2.0],
        uv_size: [1.0, 0.0, 2.0],
        tex: [6.0, 8.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_CHICKEN_TEXTURED_LEFT_WING: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [1.0, 0.0, 2.0],
        uv_size: [1.0, 0.0, 2.0],
        tex: [4.0, 8.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_CHICKEN_TEXTURED_PARTS: [TexturedModelPartDesc; 5] = [
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 20.25, -1.25],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CHICKEN_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [1.0, 22.0, 0.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CHICKEN_TEXTURED_LEFT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 22.0, 0.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CHICKEN_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [2.0, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CHICKEN_TEXTURED_RIGHT_WING,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CHICKEN_TEXTURED_LEFT_WING,
        children: &[],
    },
];
