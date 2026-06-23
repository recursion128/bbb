use super::{ModelCubeDesc, PartPose, TexturedModelCubeDesc, TURTLE_GREEN, TURTLE_SHELL};

// Vanilla 26.1 `AdultTurtleModel.createBodyLayer` (atlas 128×64). The head, body (shell +
// belly), and four legs are direct children of the mesh root; the `egg_belly` overlay shell (one
// extra cube at the body pose) is emitted when the synced `hasEgg` state is set, and vanilla then
// drops the whole model `root.y--` by one unit. The legs are repositioned per frame by
// `QuadrupedModel.setupAnim` + `TurtleModel.setupAnim`, so their poses are built from the offset
// constants and the animation curves below.
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

// `egg_belly` (`texOffs(70, 33)`): a thin 9×18×1 overlay shell shown only while `hasEgg`. It
// shares the body's pose ([`TURTLE_BODY_POSE`], offset `[0, 11, -10]`, `Rx(π/2)`).
pub(in crate::entity_models) const TURTLE_EGG_BELLY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, 3.0, -14.0],
    size: [9.0, 18.0, 1.0],
    color: TURTLE_SHELL,
}];

/// Vanilla `AdultTurtleModel.setupAnim` `root.y--`: the model-local one-unit drop applied to the
/// whole turtle while the `egg_belly` is shown.
pub(in crate::entity_models) const TURTLE_EGG_ROOT_DROP_POSE: PartPose = PartPose {
    offset: [0.0, -1.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

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

/// Vanilla `TurtleModel.setupAnim` land leg yaw swing. The hind legs swing
/// `±cos(pos·5)·3·speed`; the front legs swing `±cos(layEgg·pos·5)·8·speed·layEggAmplitude`,
/// where a turtle that `isLayingEgg` sets `layEgg = 4` (the front legs paddle four times faster)
/// and `layEggAmplitude = 2` (twice as wide) to mime digging the nest, while the hind legs are
/// untouched. Both multipliers are `1` when not laying, recovering the plain walk. The sign is
/// negated for the right legs.
pub(in crate::entity_models) fn turtle_land_leg_y_rot(
    pos: f32,
    speed: f32,
    front: bool,
    right: bool,
    laying: bool,
) -> f32 {
    let sign = if right { -1.0 } else { 1.0 };
    if front {
        let lay_egg = if laying { 4.0 } else { 1.0 };
        let lay_amplitude = if laying { 2.0 } else { 1.0 };
        sign * (lay_egg * pos * 5.0).cos() * 8.0 * speed * lay_amplitude
    } else {
        sign * (pos * 5.0).cos() * 3.0 * speed
    }
}

/// Vanilla `TurtleModel.setupAnim` water paddle swing: `swing = cos(pos·0.6662·0.6)·0.5·speed`.
/// The hind legs use it on `xRot` (overriding the quadruped base), the front legs on `zRot`.
pub(in crate::entity_models) fn turtle_water_swing(pos: f32, speed: f32) -> f32 {
    (pos * 0.6662 * 0.6).cos() * 0.5 * speed
}

/// The full per-leg rotation `[xRot, yRot, zRot]` for one turtle leg, composing the
/// `QuadrupedModel` base swing with the `TurtleModel` land/water branch. `front`/`right`
/// identify the leg; `on_land` selects the branch (`!isInWater && onGround`); `laying` applies
/// the egg-laying front-leg amplitude (land branch only, matching vanilla).
pub(in crate::entity_models) fn turtle_leg_rotation(
    pos: f32,
    speed: f32,
    on_land: bool,
    front: bool,
    right: bool,
    laying: bool,
) -> [f32; 3] {
    let base_x = turtle_quadruped_leg_x_rot(pos, speed, front == right);
    if on_land {
        // Land: the quadruped `xRot` swing remains and the turtle adds the `yRot` walk swing.
        [
            base_x,
            turtle_land_leg_y_rot(pos, speed, front, right, laying),
            0.0,
        ]
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

// Textured counterparts of the adult turtle cubes (atlas 128×64). No turtle cube is mirrored.
pub(in crate::entity_models) const TURTLE_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -1.0, -3.0],
        size: [6.0, 5.0, 6.0],
        uv_size: [6.0, 5.0, 6.0],
        tex: [3.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const TURTLE_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-9.5, 3.0, -10.0],
        size: [19.0, 20.0, 6.0],
        uv_size: [19.0, 20.0, 6.0],
        tex: [7.0, 37.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-5.5, 3.0, -13.0],
        size: [11.0, 18.0, 3.0],
        uv_size: [11.0, 18.0, 3.0],
        tex: [31.0, 1.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const TURTLE_TEXTURED_EGG_BELLY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.5, 3.0, -14.0],
        size: [9.0, 18.0, 1.0],
        uv_size: [9.0, 18.0, 1.0],
        tex: [70.0, 33.0],
        mirror: false,
    }];

pub(in crate::entity_models) const TURTLE_TEXTURED_RIGHT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, 0.0],
        size: [4.0, 1.0, 10.0],
        uv_size: [4.0, 1.0, 10.0],
        tex: [1.0, 23.0],
        mirror: false,
    }];

pub(in crate::entity_models) const TURTLE_TEXTURED_LEFT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, 0.0],
        size: [4.0, 1.0, 10.0],
        uv_size: [4.0, 1.0, 10.0],
        tex: [1.0, 12.0],
        mirror: false,
    }];

pub(in crate::entity_models) const TURTLE_TEXTURED_RIGHT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-13.0, 0.0, -2.0],
        size: [13.0, 1.0, 5.0],
        uv_size: [13.0, 1.0, 5.0],
        tex: [27.0, 30.0],
        mirror: false,
    }];

pub(in crate::entity_models) const TURTLE_TEXTURED_LEFT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, -2.0],
        size: [13.0, 1.0, 5.0],
        uv_size: [13.0, 1.0, 5.0],
        tex: [27.0, 24.0],
        mirror: false,
    }];

// Textured counterparts of the baby turtle cubes (atlas 16×16). The hind-leg planes use the
// vanilla negative `texOffs(-1, …)` exactly as `BabyTurtleModel` bakes them.
pub(in crate::entity_models) const TURTLE_BABY_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -1.0, -2.0],
        size: [4.0, 2.0, 4.0],
        uv_size: [4.0, 2.0, 4.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const TURTLE_BABY_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, -2.0, -3.0],
        size: [3.0, 3.0, 3.0],
        uv_size: [3.0, 3.0, 3.0],
        tex: [0.0, 6.0],
        mirror: false,
    }];

pub(in crate::entity_models) const TURTLE_BABY_TEXTURED_RIGHT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -0.5],
        size: [2.0, 0.0, 1.0],
        uv_size: [2.0, 0.0, 1.0],
        tex: [-1.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const TURTLE_BABY_TEXTURED_LEFT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, -0.5],
        size: [2.0, 0.0, 1.0],
        uv_size: [2.0, 0.0, 1.0],
        tex: [-1.0, 1.0],
        mirror: false,
    }];

pub(in crate::entity_models) const TURTLE_BABY_TEXTURED_RIGHT_FRONT_LEG: [TexturedModelCubeDesc;
    1] = [TexturedModelCubeDesc {
    min: [-2.0, 0.0, -0.5],
    size: [2.0, 0.0, 1.0],
    uv_size: [2.0, 0.0, 1.0],
    tex: [8.0, 6.0],
    mirror: false,
}];

pub(in crate::entity_models) const TURTLE_BABY_TEXTURED_LEFT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, -0.5],
        size: [2.0, 0.0, 1.0],
        uv_size: [2.0, 0.0, 1.0],
        tex: [8.0, 7.0],
        mirror: false,
    }];
