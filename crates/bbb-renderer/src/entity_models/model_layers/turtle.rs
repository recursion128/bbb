use super::{ModelCubeDesc, PartPose, TURTLE_GREEN, TURTLE_SHELL};

// Vanilla 26.1 `AdultTurtleModel.createBodyLayer` (atlas 128×64). The head, body (shell +
// belly), and four legs are direct children of the mesh root; the `egg_belly` overlay shell is
// gated on the deferred `hasEgg` state and is not emitted. The legs are repositioned per frame
// by `QuadrupedModel.setupAnim` + `TurtleModel.setupAnim`, so their poses are built from the
// offset constants and the animation curves below.
pub(in crate::entity_models) const TURTLE_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -1.0, -3.0],
    size: [6.0, 5.0, 6.0],
    color: TURTLE_GREEN,
}];

// Body: the `texOffs(7, 37)` shell box plus the `texOffs(31, 1)` belly box, both under the
// body's `Rx(π/2)` rotation.
pub(in crate::entity_models) const TURTLE_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-9.5, 3.0, -10.0],
        size: [19.0, 20.0, 6.0],
        color: TURTLE_SHELL,
    },
    ModelCubeDesc {
        min: [-5.5, 3.0, -13.0],
        size: [11.0, 18.0, 3.0],
        color: TURTLE_GREEN,
    },
];

pub(in crate::entity_models) const TURTLE_RIGHT_HIND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, 0.0],
    size: [4.0, 1.0, 10.0],
    color: TURTLE_GREEN,
}];

pub(in crate::entity_models) const TURTLE_LEFT_HIND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, 0.0],
    size: [4.0, 1.0, 10.0],
    color: TURTLE_GREEN,
}];

pub(in crate::entity_models) const TURTLE_RIGHT_FRONT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-13.0, 0.0, -2.0],
    size: [13.0, 1.0, 5.0],
    color: TURTLE_GREEN,
}];

pub(in crate::entity_models) const TURTLE_LEFT_FRONT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, -2.0],
    size: [13.0, 1.0, 5.0],
    color: TURTLE_GREEN,
}];

pub(in crate::entity_models) const TURTLE_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 19.0, -10.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const TURTLE_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 11.0, -10.0],
    rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
};

pub(in crate::entity_models) const TURTLE_RIGHT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-3.5, 22.0, 11.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const TURTLE_LEFT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [3.5, 22.0, 11.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const TURTLE_RIGHT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-5.0, 21.0, -4.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const TURTLE_LEFT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [5.0, 21.0, -4.0],
    rotation: [0.0, 0.0, 0.0],
};

// Vanilla 26.1 `BabyTurtleModel.createBodyLayer` (atlas 16×16). Smaller geometry, zero-height
// leg planes, but the same root layout and shared `TurtleModel.setupAnim`.
pub(in crate::entity_models) const TURTLE_BABY_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -1.0, -2.0],
    size: [4.0, 2.0, 4.0],
    color: TURTLE_SHELL,
}];

pub(in crate::entity_models) const TURTLE_BABY_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -2.0, -3.0],
    size: [3.0, 3.0, 3.0],
    color: TURTLE_GREEN,
}];

pub(in crate::entity_models) const TURTLE_BABY_RIGHT_HIND_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.0, 0.0, -0.5],
        size: [2.0, 0.0, 1.0],
        color: TURTLE_GREEN,
    }];

pub(in crate::entity_models) const TURTLE_BABY_LEFT_HIND_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [0.0, 0.0, -0.5],
        size: [2.0, 0.0, 1.0],
        color: TURTLE_GREEN,
    }];

pub(in crate::entity_models) const TURTLE_BABY_RIGHT_FRONT_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.0, 0.0, -0.5],
        size: [2.0, 0.0, 1.0],
        color: TURTLE_GREEN,
    }];

pub(in crate::entity_models) const TURTLE_BABY_LEFT_FRONT_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [0.0, 0.0, -0.5],
        size: [2.0, 0.0, 1.0],
        color: TURTLE_GREEN,
    }];

pub(in crate::entity_models) const TURTLE_BABY_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 22.9, -1.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const TURTLE_BABY_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 22.9, 1.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const TURTLE_BABY_RIGHT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-2.0, 23.9, 2.5],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const TURTLE_BABY_LEFT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [2.0, 23.9, 2.5],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const TURTLE_BABY_RIGHT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-2.0, 23.9, -0.5],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const TURTLE_BABY_LEFT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [2.0, 23.9, -0.5],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla `QuadrupedModel.setupAnim` leg swing: `leg.xRot = cos(pos·0.6662 + phase)·1.4·speed`
/// with the diagonal `phase = π` for the left-hind and right-front legs. This is the base pose
/// that `TurtleModel.setupAnim` then augments (land) or partly overrides (water).
pub(in crate::entity_models) fn turtle_quadruped_leg_x_rot(
    pos: f32,
    speed: f32,
    phase_pi: bool,
) -> f32 {
    let phase = if phase_pi { std::f32::consts::PI } else { 0.0 };
    (pos * 0.6662 + phase).cos() * 1.4 * speed
}

/// Vanilla `TurtleModel.setupAnim` land leg yaw swing: `leg.yRot = ±cos(pos·5)·weight·speed`
/// with `weight = 8` (front) / `3` (hind) and the sign negated for the right legs. The
/// egg-laying `layEgg`/`layEggAmplitude` multipliers (both `1` when not laying) are deferred
/// entity-side state, so this assumes the not-laying pose.
pub(in crate::entity_models) fn turtle_land_leg_y_rot(
    pos: f32,
    speed: f32,
    front: bool,
    right: bool,
) -> f32 {
    let swing = (pos * 5.0).cos();
    let weight = if front { 8.0 } else { 3.0 };
    let sign = if right { -1.0 } else { 1.0 };
    sign * swing * weight * speed
}

/// Vanilla `TurtleModel.setupAnim` water paddle swing: `swing = cos(pos·0.6662·0.6)·0.5·speed`.
/// The hind legs use it on `xRot` (overriding the quadruped base), the front legs on `zRot`.
pub(in crate::entity_models) fn turtle_water_swing(pos: f32, speed: f32) -> f32 {
    (pos * 0.6662 * 0.6).cos() * 0.5 * speed
}

/// The full per-leg rotation `[xRot, yRot, zRot]` for one turtle leg, composing the
/// `QuadrupedModel` base swing with the `TurtleModel` land/water branch. `front`/`right`
/// identify the leg; `on_land` selects the branch (`!isInWater && onGround`).
pub(in crate::entity_models) fn turtle_leg_rotation(
    pos: f32,
    speed: f32,
    on_land: bool,
    front: bool,
    right: bool,
) -> [f32; 3] {
    let base_x = turtle_quadruped_leg_x_rot(pos, speed, front == right);
    if on_land {
        // Land: the quadruped `xRot` swing remains and the turtle adds the `yRot` walk swing.
        [base_x, turtle_land_leg_y_rot(pos, speed, front, right), 0.0]
    } else {
        // Water: the hind legs' `xRot` is replaced by the paddle swing; the front legs keep the
        // quadruped `xRot` and add the paddle swing on `zRot`.
        let swing = turtle_water_swing(pos, speed);
        if front {
            [base_x, 0.0, if right { -swing } else { swing }]
        } else {
            [if right { swing } else { -swing }, 0.0, 0.0]
        }
    }
}
