use super::{ModelCubeDesc, PartPose, BEE_YELLOW};

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
