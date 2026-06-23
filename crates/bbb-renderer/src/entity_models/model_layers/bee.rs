use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    BEE_YELLOW,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

use std::f32::consts::PI;

// Vanilla 26.1 `AdultBeeModel.createBodyLayer` (atlas 64×64). The empty `bone` pivot parents the
// body (which carries the stinger and the two antennae), the two wings, and the three leg planes.
// The colored path approximates the striped texture with a single representative yellow.
pub(in crate::entity_models) const BEE_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -4.0, -5.0],
    size: [7.0, 7.0, 10.0],
    color: BEE_YELLOW,
}];

// The stinger is a zero-thickness plane (x size 0).
pub(in crate::entity_models) const BEE_STINGER: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -1.0, 5.0],
    size: [0.0, 1.0, 2.0],
    color: BEE_YELLOW,
}];

pub(in crate::entity_models) const BEE_LEFT_ANTENNA: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [1.5, -2.0, -3.0],
    size: [1.0, 2.0, 3.0],
    color: BEE_YELLOW,
}];

pub(in crate::entity_models) const BEE_RIGHT_ANTENNA: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, -2.0, -3.0],
    size: [1.0, 2.0, 3.0],
    color: BEE_YELLOW,
}];

// The wings are zero-height planes inflated by the vanilla `CubeDeformation(0.001)`, so the colored
// box bakes `min -= 0.001` / `size += 0.002`.
pub(in crate::entity_models) const BEE_RIGHT_WING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-9.001, -0.001, -0.001],
    size: [9.002, 0.002, 6.002],
    color: BEE_YELLOW,
}];

pub(in crate::entity_models) const BEE_LEFT_WING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.001, -0.001, -0.001],
    size: [9.002, 0.002, 6.002],
    color: BEE_YELLOW,
}];

// The three leg pairs are zero-depth planes (z size 0).
pub(in crate::entity_models) const BEE_FRONT_LEGS: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.0, 0.0, 0.0],
    size: [7.0, 2.0, 0.0],
    color: BEE_YELLOW,
}];

pub(in crate::entity_models) const BEE_MIDDLE_LEGS: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.0, 0.0, 0.0],
    size: [7.0, 2.0, 0.0],
    color: BEE_YELLOW,
}];

pub(in crate::entity_models) const BEE_BACK_LEGS: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.0, 0.0, 0.0],
    size: [7.0, 2.0, 0.0],
    color: BEE_YELLOW,
}];

pub(in crate::entity_models) const BEE_BONE_POSE: PartPose = PartPose {
    offset: [0.0, 19.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_STINGER_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_LEFT_ANTENNA_POSE: PartPose = PartPose {
    offset: [0.0, -2.0, -5.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_RIGHT_ANTENNA_POSE: PartPose = PartPose {
    offset: [0.0, -2.0, -5.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_RIGHT_WING_POSE: PartPose = PartPose {
    offset: [-1.5, -4.0, -3.0],
    rotation: [0.0, -0.2618, 0.0],
};
pub(in crate::entity_models) const BEE_LEFT_WING_POSE: PartPose = PartPose {
    offset: [1.5, -4.0, -3.0],
    rotation: [0.0, 0.2618, 0.0],
};
pub(in crate::entity_models) const BEE_FRONT_LEGS_POSE: PartPose = PartPose {
    offset: [1.5, 3.0, -2.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_MIDDLE_LEGS_POSE: PartPose = PartPose {
    offset: [1.5, 3.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_BACK_LEGS_POSE: PartPose = PartPose {
    offset: [1.5, 3.0, 2.0],
    rotation: [0.0, 0.0, 0.0],
};

// Vanilla 26.1 `BabyBeeModel.createBodyLayer` (atlas 32×32). The `bone` pivot itself carries two
// small cubes; there are no antennae, and the wings sit at a different bind rotation.
pub(in crate::entity_models) const BEE_BABY_BONE: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [1.0, -1.6667, -2.1633],
        size: [1.0, 2.0, 2.0],
        color: BEE_YELLOW,
    },
    ModelCubeDesc {
        min: [-2.0, -1.6667, -2.1933],
        size: [1.0, 2.0, 2.0],
        color: BEE_YELLOW,
    },
];

pub(in crate::entity_models) const BEE_BABY_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -2.0, -2.5],
    size: [4.0, 4.0, 5.0],
    color: BEE_YELLOW,
}];

pub(in crate::entity_models) const BEE_BABY_STINGER: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -0.5, 0.0],
    size: [0.0, 1.0, 1.0],
    color: BEE_YELLOW,
}];

pub(in crate::entity_models) const BEE_BABY_RIGHT_WING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 0.0, 0.0],
    size: [3.0, 0.0, 3.0],
    color: BEE_YELLOW,
}];

pub(in crate::entity_models) const BEE_BABY_LEFT_WING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [3.0, 0.0, 3.0],
    color: BEE_YELLOW,
}];

pub(in crate::entity_models) const BEE_BABY_FRONT_LEGS: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, 0.0],
    size: [3.0, 1.0, 0.0],
    color: BEE_YELLOW,
}];

pub(in crate::entity_models) const BEE_BABY_MIDDLE_LEGS: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, 0.0],
    size: [3.0, 1.0, 0.0],
    color: BEE_YELLOW,
}];

pub(in crate::entity_models) const BEE_BABY_BACK_LEGS: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, 0.0],
    size: [3.0, 1.0, 0.0],
    color: BEE_YELLOW,
}];

pub(in crate::entity_models) const BEE_BABY_BONE_POSE: PartPose = PartPose {
    offset: [0.0, 19.6667, -1.8567],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_BABY_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 1.3333, 2.3567],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_BABY_STINGER_POSE: PartPose = PartPose {
    offset: [0.0, 0.5, 2.5],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_BABY_RIGHT_WING_POSE: PartPose = PartPose {
    offset: [-1.0, -0.6667, 0.8567],
    rotation: [0.2182, 0.3491, 0.0],
};
pub(in crate::entity_models) const BEE_BABY_LEFT_WING_POSE: PartPose = PartPose {
    offset: [1.0, -0.6667, 0.8567],
    rotation: [0.2182, -0.3491, 0.0],
};
pub(in crate::entity_models) const BEE_BABY_FRONT_LEGS_POSE: PartPose = PartPose {
    offset: [0.0, 3.3333, 1.8567],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_BABY_MIDDLE_LEGS_POSE: PartPose = PartPose {
    offset: [0.0, 3.3333, 2.8567],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_BABY_BACK_LEGS_POSE: PartPose = PartPose {
    offset: [0.0, 3.3333, 3.8567],
    rotation: [0.0, 0.0, 0.0],
};

// The same geometry with the vanilla `AdultBeeModel.createBodyLayer` texOffs UV coordinates (atlas
// 64×64). The wings keep the BASE box `uv_size` (`[9, 0, 6]`) while the geometry inflates by the
// `CubeDeformation(0.001)`.
pub(in crate::entity_models) const BEE_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.5, -4.0, -5.0],
        size: [7.0, 7.0, 10.0],
        uv_size: [7.0, 7.0, 10.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BEE_TEXTURED_STINGER: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, -1.0, 5.0],
        size: [0.0, 1.0, 2.0],
        uv_size: [0.0, 1.0, 2.0],
        tex: [26.0, 7.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BEE_TEXTURED_LEFT_ANTENNA: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [1.5, -2.0, -3.0],
        size: [1.0, 2.0, 3.0],
        uv_size: [1.0, 2.0, 3.0],
        tex: [2.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BEE_TEXTURED_RIGHT_ANTENNA: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.5, -2.0, -3.0],
        size: [1.0, 2.0, 3.0],
        uv_size: [1.0, 2.0, 3.0],
        tex: [2.0, 3.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BEE_TEXTURED_RIGHT_WING: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-9.001, -0.001, -0.001],
        size: [9.002, 0.002, 6.002],
        uv_size: [9.0, 0.0, 6.0],
        tex: [0.0, 18.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BEE_TEXTURED_LEFT_WING: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-0.001, -0.001, -0.001],
        size: [9.002, 0.002, 6.002],
        uv_size: [9.0, 0.0, 6.0],
        tex: [0.0, 18.0],
        mirror: true,
    }];

pub(in crate::entity_models) const BEE_TEXTURED_FRONT_LEGS: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-5.0, 0.0, 0.0],
        size: [7.0, 2.0, 0.0],
        uv_size: [7.0, 2.0, 0.0],
        tex: [26.0, 1.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BEE_TEXTURED_MIDDLE_LEGS: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-5.0, 0.0, 0.0],
        size: [7.0, 2.0, 0.0],
        uv_size: [7.0, 2.0, 0.0],
        tex: [26.0, 3.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BEE_TEXTURED_BACK_LEGS: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-5.0, 0.0, 0.0],
        size: [7.0, 2.0, 0.0],
        uv_size: [7.0, 2.0, 0.0],
        tex: [26.0, 5.0],
        mirror: false,
    }];

// The baby textured geometry (`BabyBeeModel.createBodyLayer`, atlas 32×32). The left wing carries
// the vanilla negative `texOffs(-3, 9)` with a mirrored box.
pub(in crate::entity_models) const BEE_BABY_TEXTURED_BONE: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [1.0, -1.6667, -2.1633],
        size: [1.0, 2.0, 2.0],
        uv_size: [1.0, 2.0, 2.0],
        tex: [6.0, 12.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-2.0, -1.6667, -2.1933],
        size: [1.0, 2.0, 2.0],
        uv_size: [1.0, 2.0, 2.0],
        tex: [0.0, 12.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BEE_BABY_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -2.0, -2.5],
        size: [4.0, 4.0, 5.0],
        uv_size: [4.0, 4.0, 5.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BEE_BABY_TEXTURED_STINGER: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, -0.5, 0.0],
        size: [0.0, 1.0, 1.0],
        uv_size: [0.0, 1.0, 1.0],
        tex: [13.0, 2.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BEE_BABY_TEXTURED_RIGHT_WING: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, 0.0, 0.0],
        size: [3.0, 0.0, 3.0],
        uv_size: [3.0, 0.0, 3.0],
        tex: [3.0, 9.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BEE_BABY_TEXTURED_LEFT_WING: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [3.0, 0.0, 3.0],
        uv_size: [3.0, 0.0, 3.0],
        tex: [-3.0, 9.0],
        mirror: true,
    }];

pub(in crate::entity_models) const BEE_BABY_TEXTURED_FRONT_LEGS: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, 0.0, 0.0],
        size: [3.0, 1.0, 0.0],
        uv_size: [3.0, 1.0, 0.0],
        tex: [13.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BEE_BABY_TEXTURED_MIDDLE_LEGS: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, 0.0, 0.0],
        size: [3.0, 1.0, 0.0],
        uv_size: [3.0, 1.0, 0.0],
        tex: [13.0, 1.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BEE_BABY_TEXTURED_BACK_LEGS: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, 0.0, 0.0],
        size: [3.0, 1.0, 0.0],
        uv_size: [3.0, 1.0, 0.0],
        tex: [13.0, 2.0],
        mirror: false,
    }];

// The flying middle-leg angle (vanilla sets all three legs to `π/4` in the airborne branch before
// the bob overrides the front/back pair).
pub(in crate::entity_models) const BEE_MID_LEG_FLYING_X_ROT: f32 = PI / 4.0;

/// Vanilla `BeeModel.setupAnim` wing flap: `zRot = cos(ageInTicks · 120.32113°) · π · 0.15`. The
/// left wing mirrors this (`leftWing.zRot = -rightWing.zRot`).
pub(in crate::entity_models) fn bee_wing_z_rot(age_in_ticks: f32) -> f32 {
    (age_in_ticks * 120.32113 * (PI / 180.0)).cos() * PI * 0.15
}

/// The shared `bobUpAndDown` speed term: `cos(ageInTicks · 0.18)`.
pub(in crate::entity_models) fn bee_bob_speed(age_in_ticks: f32) -> f32 {
    (age_in_ticks * 0.18).cos()
}

/// `bone.xRot = 0.1 + speed · π · 0.025`.
pub(in crate::entity_models) fn bee_bone_x_rot(age_in_ticks: f32) -> f32 {
    0.1 + bee_bob_speed(age_in_ticks) * PI * 0.025
}

/// The vertical bob added to the bone pivot: `bone.y -= cos(ageInTicks · 0.18) · 0.9`.
pub(in crate::entity_models) fn bee_bone_y_delta(age_in_ticks: f32) -> f32 {
    -bee_bob_speed(age_in_ticks) * 0.9
}

/// `frontLeg.xRot = -speed · π · 0.1 + π/8`.
pub(in crate::entity_models) fn bee_front_leg_x_rot(age_in_ticks: f32) -> f32 {
    -bee_bob_speed(age_in_ticks) * PI * 0.1 + PI / 8.0
}

/// `backLeg.xRot = -speed · π · 0.05 + π/4`.
pub(in crate::entity_models) fn bee_back_leg_x_rot(age_in_ticks: f32) -> f32 {
    -bee_bob_speed(age_in_ticks) * PI * 0.05 + PI / 4.0
}

/// Adult-only antenna bob (`AdultBeeModel.bobUpAndDown`): `antenna.xRot = speed · π · 0.03`.
pub(in crate::entity_models) fn bee_antenna_x_rot(age_in_ticks: f32) -> f32 {
    bee_bob_speed(age_in_ticks) * PI * 0.03
}

// Colored adult bee tree (`AdultBeeModel.createBodyLayer`): a single root child, the empty `bone`
// pivot, parenting the body (which carries the stinger and the two antennae), the two wings, and the
// three leg planes — in the vanilla emit order. Zipped with the textured tree by [`BeeModel::new`].
const BEE_PARTS: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: BEE_BONE_POSE,
    cubes: &[],
    children: &[
        ModelPartDesc {
            pose: BEE_BODY_POSE,
            cubes: &BEE_BODY,
            children: &[
                ModelPartDesc {
                    pose: BEE_STINGER_POSE,
                    cubes: &BEE_STINGER,
                    children: &[],
                },
                ModelPartDesc {
                    pose: BEE_LEFT_ANTENNA_POSE,
                    cubes: &BEE_LEFT_ANTENNA,
                    children: &[],
                },
                ModelPartDesc {
                    pose: BEE_RIGHT_ANTENNA_POSE,
                    cubes: &BEE_RIGHT_ANTENNA,
                    children: &[],
                },
            ],
        },
        ModelPartDesc {
            pose: BEE_RIGHT_WING_POSE,
            cubes: &BEE_RIGHT_WING,
            children: &[],
        },
        ModelPartDesc {
            pose: BEE_LEFT_WING_POSE,
            cubes: &BEE_LEFT_WING,
            children: &[],
        },
        ModelPartDesc {
            pose: BEE_FRONT_LEGS_POSE,
            cubes: &BEE_FRONT_LEGS,
            children: &[],
        },
        ModelPartDesc {
            pose: BEE_MIDDLE_LEGS_POSE,
            cubes: &BEE_MIDDLE_LEGS,
            children: &[],
        },
        ModelPartDesc {
            pose: BEE_BACK_LEGS_POSE,
            cubes: &BEE_BACK_LEGS,
            children: &[],
        },
    ],
}];
const BEE_TEXTURED_PARTS: [TexturedModelPartDesc; 1] = [TexturedModelPartDesc {
    pose: BEE_BONE_POSE,
    cubes: &[],
    children: &[
        TexturedModelPartDesc {
            pose: BEE_BODY_POSE,
            cubes: &BEE_TEXTURED_BODY,
            children: &[
                TexturedModelPartDesc {
                    pose: BEE_STINGER_POSE,
                    cubes: &BEE_TEXTURED_STINGER,
                    children: &[],
                },
                TexturedModelPartDesc {
                    pose: BEE_LEFT_ANTENNA_POSE,
                    cubes: &BEE_TEXTURED_LEFT_ANTENNA,
                    children: &[],
                },
                TexturedModelPartDesc {
                    pose: BEE_RIGHT_ANTENNA_POSE,
                    cubes: &BEE_TEXTURED_RIGHT_ANTENNA,
                    children: &[],
                },
            ],
        },
        TexturedModelPartDesc {
            pose: BEE_RIGHT_WING_POSE,
            cubes: &BEE_TEXTURED_RIGHT_WING,
            children: &[],
        },
        TexturedModelPartDesc {
            pose: BEE_LEFT_WING_POSE,
            cubes: &BEE_TEXTURED_LEFT_WING,
            children: &[],
        },
        TexturedModelPartDesc {
            pose: BEE_FRONT_LEGS_POSE,
            cubes: &BEE_TEXTURED_FRONT_LEGS,
            children: &[],
        },
        TexturedModelPartDesc {
            pose: BEE_MIDDLE_LEGS_POSE,
            cubes: &BEE_TEXTURED_MIDDLE_LEGS,
            children: &[],
        },
        TexturedModelPartDesc {
            pose: BEE_BACK_LEGS_POSE,
            cubes: &BEE_TEXTURED_BACK_LEGS,
            children: &[],
        },
    ],
}];

// Colored baby bee tree (`BabyBeeModel.createBodyLayer`): the `bone` pivot itself carries two small
// cubes, the body has only the stinger (no antennae), and the wings/legs sit at the baby binds.
const BEE_BABY_PARTS: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: BEE_BABY_BONE_POSE,
    cubes: &BEE_BABY_BONE,
    children: &[
        ModelPartDesc {
            pose: BEE_BABY_BODY_POSE,
            cubes: &BEE_BABY_BODY,
            children: &[ModelPartDesc {
                pose: BEE_BABY_STINGER_POSE,
                cubes: &BEE_BABY_STINGER,
                children: &[],
            }],
        },
        ModelPartDesc {
            pose: BEE_BABY_RIGHT_WING_POSE,
            cubes: &BEE_BABY_RIGHT_WING,
            children: &[],
        },
        ModelPartDesc {
            pose: BEE_BABY_LEFT_WING_POSE,
            cubes: &BEE_BABY_LEFT_WING,
            children: &[],
        },
        ModelPartDesc {
            pose: BEE_BABY_FRONT_LEGS_POSE,
            cubes: &BEE_BABY_FRONT_LEGS,
            children: &[],
        },
        ModelPartDesc {
            pose: BEE_BABY_MIDDLE_LEGS_POSE,
            cubes: &BEE_BABY_MIDDLE_LEGS,
            children: &[],
        },
        ModelPartDesc {
            pose: BEE_BABY_BACK_LEGS_POSE,
            cubes: &BEE_BABY_BACK_LEGS,
            children: &[],
        },
    ],
}];
const BEE_BABY_TEXTURED_PARTS: [TexturedModelPartDesc; 1] = [TexturedModelPartDesc {
    pose: BEE_BABY_BONE_POSE,
    cubes: &BEE_BABY_TEXTURED_BONE,
    children: &[
        TexturedModelPartDesc {
            pose: BEE_BABY_BODY_POSE,
            cubes: &BEE_BABY_TEXTURED_BODY,
            children: &[TexturedModelPartDesc {
                pose: BEE_BABY_STINGER_POSE,
                cubes: &BEE_BABY_TEXTURED_STINGER,
                children: &[],
            }],
        },
        TexturedModelPartDesc {
            pose: BEE_BABY_RIGHT_WING_POSE,
            cubes: &BEE_BABY_TEXTURED_RIGHT_WING,
            children: &[],
        },
        TexturedModelPartDesc {
            pose: BEE_BABY_LEFT_WING_POSE,
            cubes: &BEE_BABY_TEXTURED_LEFT_WING,
            children: &[],
        },
        TexturedModelPartDesc {
            pose: BEE_BABY_FRONT_LEGS_POSE,
            cubes: &BEE_BABY_TEXTURED_FRONT_LEGS,
            children: &[],
        },
        TexturedModelPartDesc {
            pose: BEE_BABY_MIDDLE_LEGS_POSE,
            cubes: &BEE_BABY_TEXTURED_MIDDLE_LEGS,
            children: &[],
        },
        TexturedModelPartDesc {
            pose: BEE_BABY_BACK_LEGS_POSE,
            cubes: &BEE_BABY_TEXTURED_BACK_LEGS,
            children: &[],
        },
    ],
}];

/// Selects the colored and textured const trees for an adult or baby bee, zipped into the unified
/// tree by [`BeeModel::new`].
fn bee_part_trees(baby: bool) -> (&'static [ModelPartDesc], &'static [TexturedModelPartDesc]) {
    if baby {
        (&BEE_BABY_PARTS, &BEE_BABY_TEXTURED_PARTS)
    } else {
        (&BEE_PARTS, &BEE_TEXTURED_PARTS)
    }
}

/// Applies vanilla `BeeModel.setupAnim` to the unified tree. While airborne the wings flap on
/// `ageInTicks` and (when not angry) `bobUpAndDown` rocks the `bone` pivot, the front/back legs and —
/// on adults — the antennae; all three legs first splay to `π/4`, so the middle leg holds that angle
/// while the bob overrides the front/back pair. On the ground the model rests at its bind pose. The
/// stinger cube is hidden once the bee has stung (`stinger.visible = hasStinger`).
fn apply_bee_anim(root: &mut ModelPart, baby: bool, instance: &EntityModelInstance) {
    let age = instance.render_state.age_in_ticks;
    let flying = !instance.render_state.on_ground;
    // Vanilla gates `bobUpAndDown` on `!isAngry && !isOnGround`: an angry airborne bee still flaps
    // its wings and splays its legs to `π/4`, but its body, front/back legs and antennae hold still.
    let bob = flying && !instance.render_state.bee_angry;
    let has_stinger = instance.render_state.bee_has_stinger;

    // Bone pivot (root child): the airborne bob rocks it forward and lifts/drops it.
    let bone = root.child_at_mut(0);
    if bob {
        bone.pose.offset[1] += bee_bone_y_delta(age);
        bone.pose.rotation = [bee_bone_x_rot(age), 0.0, 0.0];
    }

    // Body subtree: the stinger is shown only while carried; the adult antennae bob with the body.
    {
        let body = bone.child_at_mut(0);
        body.child_at_mut(0).visible = has_stinger;
        if !baby {
            let antenna_x_rot = if bob { bee_antenna_x_rot(age) } else { 0.0 };
            body.child_at_mut(1).pose.rotation = [antenna_x_rot, 0.0, 0.0];
            body.child_at_mut(2).pose.rotation = [antenna_x_rot, 0.0, 0.0];
        }
    }

    // Wings (bone children): the flap overrides the bind yaw to 0 and drives `zRot`, mirrored on the
    // left, while the bind pitch (0 on adults, `0.2182` on babies) is preserved.
    if flying {
        let wing_z_rot = bee_wing_z_rot(age);
        let right_wing = bone.child_at_mut(1);
        right_wing.pose.rotation = [right_wing.pose.rotation[0], 0.0, wing_z_rot];
        let left_wing = bone.child_at_mut(2);
        left_wing.pose.rotation = [left_wing.pose.rotation[0], 0.0, -wing_z_rot];
    }

    // Legs (bone children): airborne, all three splay to `π/4`; the non-angry bob then overrides the
    // front/back pair, while an angry bee holds all three at `π/4`. On the ground they rest at `0`.
    let (front_x, mid_x, back_x) = if flying {
        (
            if bob {
                bee_front_leg_x_rot(age)
            } else {
                BEE_MID_LEG_FLYING_X_ROT
            },
            BEE_MID_LEG_FLYING_X_ROT,
            if bob {
                bee_back_leg_x_rot(age)
            } else {
                BEE_MID_LEG_FLYING_X_ROT
            },
        )
    } else {
        (0.0, 0.0, 0.0)
    };
    bone.child_at_mut(3).pose.rotation = [front_x, 0.0, 0.0];
    bone.child_at_mut(4).pose.rotation = [mid_x, 0.0, 0.0];
    bone.child_at_mut(5).pose.rotation = [back_x, 0.0, 0.0];
}

/// Mutable bee model, mirroring vanilla `AdultBeeModel` / `BabyBeeModel`. The unified tree is zipped
/// from the const trees selected by `baby` ([`bee_part_trees`]); `setup_anim` runs [`apply_bee_anim`].
/// The same posed tree drives the colored fallback and the cutout textured layer; the adult/baby
/// texture and the rolled-up fall pose (`rollAmount`) live outside the model.
pub(in crate::entity_models) struct BeeModel {
    root: ModelPart,
    baby: bool,
}

impl BeeModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        let (colored, textured) = bee_part_trees(baby);
        Self {
            root: ModelPart::root_from_descs(colored, textured),
            baby,
        }
    }
}

impl EntityModel for BeeModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_bee_anim(&mut self.root, self.baby, instance);
    }
}
