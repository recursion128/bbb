use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    STRIDER_LEG, STRIDER_MAROON,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

// Vanilla 26.1 `AdultStriderModel.createBodyLayer` (atlas 64×128). The mesh root parents the
// two legs and the body directly; the six bristles hang under the body. The legs and body are
// repositioned/rotated by `StriderModel.setupAnim` + `AdultStriderModel.customAnimations`, so
// their poses are built per frame from the offset constants and the animation curves below.
pub(in crate::entity_models) const STRIDER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-8.0, -6.0, -8.0],
    size: [16.0, 14.0, 16.0],
    color: STRIDER_MAROON,
}];

pub(in crate::entity_models) const STRIDER_RIGHT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 16.0, 4.0],
    color: STRIDER_LEG,
}];

pub(in crate::entity_models) const STRIDER_LEFT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 16.0, 4.0],
    color: STRIDER_LEG,
}];

// Bristles are zero-thickness `12×0×16` planes. The right bristles are mirrored (box at
// `-12`), the left are not (box at `0`).
pub(in crate::entity_models) const STRIDER_RIGHT_BRISTLE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-12.0, 0.0, 0.0],
    size: [12.0, 0.0, 16.0],
    color: STRIDER_MAROON,
}];

pub(in crate::entity_models) const STRIDER_LEFT_BRISTLE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [12.0, 0.0, 16.0],
    color: STRIDER_MAROON,
}];

/// Adult body base height (`customAnimations` sets `body.y = 2.0` before the bob).
pub(in crate::entity_models) const STRIDER_BODY_BASE_Y: f32 = 2.0;
/// Adult leg base height (`leftLeg.y`/`rightLeg.y = 8.0` before the lift) and leg x offsets.
pub(in crate::entity_models) const STRIDER_LEG_BASE_Y: f32 = 8.0;
pub(in crate::entity_models) const STRIDER_RIGHT_LEG_X: f32 = -4.0;
pub(in crate::entity_models) const STRIDER_LEFT_LEG_X: f32 = 4.0;

// Adult bristle rest poses (offset + rest `zRot`); the flow curve is added to `zRot` per frame.
pub(in crate::entity_models) const STRIDER_RIGHT_TOP_BRISTLE_POSE: PartPose = PartPose {
    offset: [-8.0, -5.0, -8.0],
    rotation: [0.0, 0.0, -0.872_664_63],
};
pub(in crate::entity_models) const STRIDER_RIGHT_MIDDLE_BRISTLE_POSE: PartPose = PartPose {
    offset: [-8.0, -1.0, -8.0],
    rotation: [0.0, 0.0, -1.134_464],
};
pub(in crate::entity_models) const STRIDER_RIGHT_BOTTOM_BRISTLE_POSE: PartPose = PartPose {
    offset: [-8.0, 4.0, -8.0],
    rotation: [0.0, 0.0, -1.221_730_5],
};
pub(in crate::entity_models) const STRIDER_LEFT_TOP_BRISTLE_POSE: PartPose = PartPose {
    offset: [8.0, -6.0, -8.0],
    rotation: [0.0, 0.0, 0.872_664_63],
};
pub(in crate::entity_models) const STRIDER_LEFT_MIDDLE_BRISTLE_POSE: PartPose = PartPose {
    offset: [8.0, -2.0, -8.0],
    rotation: [0.0, 0.0, 1.134_464],
};
pub(in crate::entity_models) const STRIDER_LEFT_BOTTOM_BRISTLE_POSE: PartPose = PartPose {
    offset: [8.0, 3.0, -8.0],
    rotation: [0.0, 0.0, 1.221_730_5],
};

// Vanilla 26.1 `BabyStriderModel.createBodyLayer` (atlas 32×32). Same root layout (legs +
// body, three bristles under the body), smaller geometry, and the bristles flap on `xRot`.
pub(in crate::entity_models) const STRIDER_BABY_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -3.75, -4.0],
    size: [7.0, 7.0, 8.0],
    color: STRIDER_MAROON,
}];

pub(in crate::entity_models) const STRIDER_BABY_RIGHT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 4.0, 2.0],
    color: STRIDER_LEG,
}];

pub(in crate::entity_models) const STRIDER_BABY_LEFT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 4.0, 2.0],
    color: STRIDER_LEG,
}];

// Baby bristles are zero-thickness `7×3×0` planes.
pub(in crate::entity_models) const STRIDER_BABY_BRISTLE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -2.5, 0.0],
    size: [7.0, 3.0, 0.0],
    color: STRIDER_MAROON,
}];

/// Baby body base height (`customAnimations` sets `body.y = 17.25` before the bob).
pub(in crate::entity_models) const STRIDER_BABY_BODY_BASE_Y: f32 = 17.25;
/// Baby leg base height (`leg.y = 20.0` before the lift) and leg x offsets.
pub(in crate::entity_models) const STRIDER_BABY_LEG_BASE_Y: f32 = 20.0;
pub(in crate::entity_models) const STRIDER_BABY_RIGHT_LEG_X: f32 = -1.5;
pub(in crate::entity_models) const STRIDER_BABY_LEFT_LEG_X: f32 = 1.5;

// Baby bristle rest poses (offset only; the flow curve drives `xRot`). `bristle2` is the front
// bristle (the `top` flow), `bristle1` the middle, `bristle0` the back (the `bottom` flow).
pub(in crate::entity_models) const STRIDER_BABY_FRONT_BRISTLE_POSE: PartPose = PartPose {
    offset: [0.0, -4.25, -2.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const STRIDER_BABY_MIDDLE_BRISTLE_POSE: PartPose = PartPose {
    offset: [0.0, -4.25, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const STRIDER_BABY_BACK_BRISTLE_POSE: PartPose = PartPose {
    offset: [0.0, -4.25, 2.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla `StriderModel.SPEED` (the body sway / leg swing frequency multiplier).
const STRIDER_SPEED: f32 = 1.5;

/// Vanilla `StriderModel.setupAnim`: `animationSpeed = min(walkAnimationSpeed, 0.25)`.
pub(in crate::entity_models) fn strider_animation_speed(walk_animation_speed: f32) -> f32 {
    walk_animation_speed.min(0.25)
}

/// Vanilla `StriderModel.setupAnim` body sway: `body.zRot = 0.1·sin(pos·1.5)·4·speed`.
pub(in crate::entity_models) fn strider_body_z_rot(pos: f32, speed: f32) -> f32 {
    0.1 * (pos * STRIDER_SPEED).sin() * 4.0 * speed
}

/// Vanilla `StriderModel.setupAnim` leg swing: `leg.xRot = sin(pos·0.75 + phase)·2·speed` with
/// `phase = 0` (left) or `π` (right).
pub(in crate::entity_models) fn strider_leg_x_rot(pos: f32, speed: f32, right: bool) -> f32 {
    let phase = if right { std::f32::consts::PI } else { 0.0 };
    (pos * STRIDER_SPEED * 0.5 + phase).sin() * 2.0 * speed
}

/// Vanilla `StriderModel.setupAnim` leg roll: `leg.zRot = (π/18)·cos(pos·0.75 + phase)·speed`
/// with `phase = 0` (left) or `π` (right).
pub(in crate::entity_models) fn strider_leg_z_rot(pos: f32, speed: f32, right: bool) -> f32 {
    let phase = if right { std::f32::consts::PI } else { 0.0 };
    (std::f32::consts::PI / 18.0) * (pos * STRIDER_SPEED * 0.5 + phase).cos() * speed
}

/// Vanilla `customAnimations` body bob: `body.y = base - mul·cos(pos·1.5)·2·speed` (the adult
/// uses `mul = 2`, the baby `mul = 1`).
pub(in crate::entity_models) fn strider_body_y(base: f32, mul: f32, pos: f32, speed: f32) -> f32 {
    base - mul * (pos * STRIDER_SPEED).cos() * 2.0 * speed
}

/// Vanilla `customAnimations` leg lift: `leg.y = base + 2·sin(pos·0.75 + phase)·2·speed` with
/// `phase = 0` (right) or `π` (left) — note this phase is the opposite of the leg swing.
pub(in crate::entity_models) fn strider_leg_y(base: f32, pos: f32, speed: f32, right: bool) -> f32 {
    let phase = if right { 0.0 } else { std::f32::consts::PI };
    base + 2.0 * (pos * STRIDER_SPEED * 0.5 + phase).sin() * 2.0 * speed
}

/// Vanilla `customAnimations` bristle flow: `bristleFlow = cos(pos·1.5 + π)·speed`.
pub(in crate::entity_models) fn strider_bristle_flow(pos: f32, speed: f32) -> f32 {
    (pos * STRIDER_SPEED + std::f32::consts::PI).cos() * speed
}

/// Vanilla `animateBristle` for the top/front bristle: `flow·0.6 + 0.1·sin(age·0.4)`.
pub(in crate::entity_models) fn strider_bristle_top_flow(flow: f32, age_in_ticks: f32) -> f32 {
    flow * 0.6 + 0.1 * (age_in_ticks * 0.4).sin()
}

/// Vanilla `animateBristle` for the middle bristle: `flow·1.2 + 0.1·sin(age·0.2)`.
pub(in crate::entity_models) fn strider_bristle_middle_flow(flow: f32, age_in_ticks: f32) -> f32 {
    flow * 1.2 + 0.1 * (age_in_ticks * 0.2).sin()
}

/// Vanilla `animateBristle` for the bottom/back bristle: `flow·1.3 + 0.05·sin(age·-0.4)`.
pub(in crate::entity_models) fn strider_bristle_bottom_flow(flow: f32, age_in_ticks: f32) -> f32 {
    flow * 1.3 + 0.05 * (age_in_ticks * -0.4).sin()
}

// Textured counterparts of the adult strider cubes (atlas 64×128). The right bristles are
// mirrored; each bristle carries its own `texOffs`.
pub(in crate::entity_models) const STRIDER_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-8.0, -6.0, -8.0],
        size: [16.0, 14.0, 16.0],
        uv_size: [16.0, 14.0, 16.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const STRIDER_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 16.0, 4.0],
        uv_size: [4.0, 16.0, 4.0],
        tex: [0.0, 32.0],
        mirror: false,
    }];

pub(in crate::entity_models) const STRIDER_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 16.0, 4.0],
        uv_size: [4.0, 16.0, 4.0],
        tex: [0.0, 55.0],
        mirror: false,
    }];

pub(in crate::entity_models) const STRIDER_TEXTURED_RIGHT_TOP_BRISTLE: [TexturedModelCubeDesc; 1] =
    [strider_textured_right_bristle([16.0, 33.0])];
pub(in crate::entity_models) const STRIDER_TEXTURED_RIGHT_MIDDLE_BRISTLE: [TexturedModelCubeDesc;
    1] = [strider_textured_right_bristle([16.0, 49.0])];
pub(in crate::entity_models) const STRIDER_TEXTURED_RIGHT_BOTTOM_BRISTLE: [TexturedModelCubeDesc;
    1] = [strider_textured_right_bristle([16.0, 65.0])];
pub(in crate::entity_models) const STRIDER_TEXTURED_LEFT_TOP_BRISTLE: [TexturedModelCubeDesc; 1] =
    [strider_textured_left_bristle([16.0, 33.0])];
pub(in crate::entity_models) const STRIDER_TEXTURED_LEFT_MIDDLE_BRISTLE: [TexturedModelCubeDesc;
    1] = [strider_textured_left_bristle([16.0, 49.0])];
pub(in crate::entity_models) const STRIDER_TEXTURED_LEFT_BOTTOM_BRISTLE: [TexturedModelCubeDesc;
    1] = [strider_textured_left_bristle([16.0, 65.0])];

const fn strider_textured_right_bristle(tex: [f32; 2]) -> TexturedModelCubeDesc {
    TexturedModelCubeDesc {
        min: [-12.0, 0.0, 0.0],
        size: [12.0, 0.0, 16.0],
        uv_size: [12.0, 0.0, 16.0],
        tex,
        mirror: true,
    }
}

const fn strider_textured_left_bristle(tex: [f32; 2]) -> TexturedModelCubeDesc {
    TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [12.0, 0.0, 16.0],
        uv_size: [12.0, 0.0, 16.0],
        tex,
        mirror: false,
    }
}

// Textured counterparts of the baby strider cubes (atlas 32×32).
pub(in crate::entity_models) const STRIDER_BABY_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.5, -3.75, -4.0],
        size: [7.0, 7.0, 8.0],
        uv_size: [7.0, 7.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const STRIDER_BABY_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 4.0, 2.0],
        uv_size: [2.0, 4.0, 2.0],
        tex: [0.0, 24.0],
        mirror: false,
    }];

pub(in crate::entity_models) const STRIDER_BABY_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 4.0, 2.0],
        uv_size: [2.0, 4.0, 2.0],
        tex: [8.0, 24.0],
        mirror: false,
    }];

pub(in crate::entity_models) const STRIDER_BABY_TEXTURED_FRONT_BRISTLE: [TexturedModelCubeDesc; 1] =
    [strider_baby_textured_bristle([0.0, 15.0])];
pub(in crate::entity_models) const STRIDER_BABY_TEXTURED_MIDDLE_BRISTLE: [TexturedModelCubeDesc;
    1] = [strider_baby_textured_bristle([0.0, 18.0])];
pub(in crate::entity_models) const STRIDER_BABY_TEXTURED_BACK_BRISTLE: [TexturedModelCubeDesc; 1] =
    [strider_baby_textured_bristle([0.0, 21.0])];

const fn strider_baby_textured_bristle(tex: [f32; 2]) -> TexturedModelCubeDesc {
    TexturedModelCubeDesc {
        min: [-3.5, -2.5, 0.0],
        size: [7.0, 3.0, 0.0],
        uv_size: [7.0, 3.0, 0.0],
        tex,
        mirror: false,
    }
}

// The strider legs and body carry per-frame offsets/rotations (set absolutely in `setup_anim`); these
// are their bind poses (the rest offsets, which `strider_leg_y`/`strider_body_y` return at speed 0).
const STRIDER_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [STRIDER_RIGHT_LEG_X, STRIDER_LEG_BASE_Y, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const STRIDER_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [STRIDER_LEFT_LEG_X, STRIDER_LEG_BASE_Y, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const STRIDER_BODY_POSE: PartPose = PartPose {
    offset: [0.0, STRIDER_BODY_BASE_Y, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const STRIDER_BABY_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [STRIDER_BABY_RIGHT_LEG_X, STRIDER_BABY_LEG_BASE_Y, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const STRIDER_BABY_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [STRIDER_BABY_LEFT_LEG_X, STRIDER_BABY_LEG_BASE_Y, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const STRIDER_BABY_BODY_POSE: PartPose = PartPose {
    offset: [0.0, STRIDER_BABY_BODY_BASE_Y, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

// Colored adult strider tree: right leg, left leg, body (with its six bristles), in the emit order.
// Mirrors vanilla `AdultStriderModel.createBodyLayer`. The three right bristles share one colored cube
// (their textured UVs differ); same for the left.
const STRIDER_BODY_BRISTLE_CHILDREN: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: STRIDER_RIGHT_TOP_BRISTLE_POSE,
        cubes: &STRIDER_RIGHT_BRISTLE,
        children: &[],
    },
    ModelPartDesc {
        pose: STRIDER_RIGHT_MIDDLE_BRISTLE_POSE,
        cubes: &STRIDER_RIGHT_BRISTLE,
        children: &[],
    },
    ModelPartDesc {
        pose: STRIDER_RIGHT_BOTTOM_BRISTLE_POSE,
        cubes: &STRIDER_RIGHT_BRISTLE,
        children: &[],
    },
    ModelPartDesc {
        pose: STRIDER_LEFT_TOP_BRISTLE_POSE,
        cubes: &STRIDER_LEFT_BRISTLE,
        children: &[],
    },
    ModelPartDesc {
        pose: STRIDER_LEFT_MIDDLE_BRISTLE_POSE,
        cubes: &STRIDER_LEFT_BRISTLE,
        children: &[],
    },
    ModelPartDesc {
        pose: STRIDER_LEFT_BOTTOM_BRISTLE_POSE,
        cubes: &STRIDER_LEFT_BRISTLE,
        children: &[],
    },
];
const STRIDER_PARTS: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: STRIDER_RIGHT_LEG_POSE,
        cubes: &STRIDER_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: STRIDER_LEFT_LEG_POSE,
        cubes: &STRIDER_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: STRIDER_BODY_POSE,
        cubes: &STRIDER_BODY,
        children: &STRIDER_BODY_BRISTLE_CHILDREN,
    },
];
const STRIDER_TEXTURED_BODY_BRISTLE_CHILDREN: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: STRIDER_RIGHT_TOP_BRISTLE_POSE,
        cubes: &STRIDER_TEXTURED_RIGHT_TOP_BRISTLE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: STRIDER_RIGHT_MIDDLE_BRISTLE_POSE,
        cubes: &STRIDER_TEXTURED_RIGHT_MIDDLE_BRISTLE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: STRIDER_RIGHT_BOTTOM_BRISTLE_POSE,
        cubes: &STRIDER_TEXTURED_RIGHT_BOTTOM_BRISTLE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: STRIDER_LEFT_TOP_BRISTLE_POSE,
        cubes: &STRIDER_TEXTURED_LEFT_TOP_BRISTLE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: STRIDER_LEFT_MIDDLE_BRISTLE_POSE,
        cubes: &STRIDER_TEXTURED_LEFT_MIDDLE_BRISTLE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: STRIDER_LEFT_BOTTOM_BRISTLE_POSE,
        cubes: &STRIDER_TEXTURED_LEFT_BOTTOM_BRISTLE,
        children: &[],
    },
];
const STRIDER_TEXTURED_PARTS: [TexturedModelPartDesc; 3] = [
    TexturedModelPartDesc {
        pose: STRIDER_RIGHT_LEG_POSE,
        cubes: &STRIDER_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: STRIDER_LEFT_LEG_POSE,
        cubes: &STRIDER_TEXTURED_LEFT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: STRIDER_BODY_POSE,
        cubes: &STRIDER_TEXTURED_BODY,
        children: &STRIDER_TEXTURED_BODY_BRISTLE_CHILDREN,
    },
];

// Colored baby strider tree: right leg, left leg, body (with its three bristles, which flap on `xRot`).
// Mirrors vanilla `BabyStriderModel.createBodyLayer`.
const STRIDER_BABY_BODY_BRISTLE_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: STRIDER_BABY_FRONT_BRISTLE_POSE,
        cubes: &STRIDER_BABY_BRISTLE,
        children: &[],
    },
    ModelPartDesc {
        pose: STRIDER_BABY_MIDDLE_BRISTLE_POSE,
        cubes: &STRIDER_BABY_BRISTLE,
        children: &[],
    },
    ModelPartDesc {
        pose: STRIDER_BABY_BACK_BRISTLE_POSE,
        cubes: &STRIDER_BABY_BRISTLE,
        children: &[],
    },
];
const STRIDER_BABY_PARTS: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: STRIDER_BABY_RIGHT_LEG_POSE,
        cubes: &STRIDER_BABY_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: STRIDER_BABY_LEFT_LEG_POSE,
        cubes: &STRIDER_BABY_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: STRIDER_BABY_BODY_POSE,
        cubes: &STRIDER_BABY_BODY,
        children: &STRIDER_BABY_BODY_BRISTLE_CHILDREN,
    },
];
const STRIDER_BABY_TEXTURED_BODY_BRISTLE_CHILDREN: [TexturedModelPartDesc; 3] = [
    TexturedModelPartDesc {
        pose: STRIDER_BABY_FRONT_BRISTLE_POSE,
        cubes: &STRIDER_BABY_TEXTURED_FRONT_BRISTLE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: STRIDER_BABY_MIDDLE_BRISTLE_POSE,
        cubes: &STRIDER_BABY_TEXTURED_MIDDLE_BRISTLE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: STRIDER_BABY_BACK_BRISTLE_POSE,
        cubes: &STRIDER_BABY_TEXTURED_BACK_BRISTLE,
        children: &[],
    },
];
const STRIDER_BABY_TEXTURED_PARTS: [TexturedModelPartDesc; 3] = [
    TexturedModelPartDesc {
        pose: STRIDER_BABY_RIGHT_LEG_POSE,
        cubes: &STRIDER_BABY_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: STRIDER_BABY_LEFT_LEG_POSE,
        cubes: &STRIDER_BABY_TEXTURED_LEFT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: STRIDER_BABY_BODY_POSE,
        cubes: &STRIDER_BABY_TEXTURED_BODY,
        children: &STRIDER_BABY_TEXTURED_BODY_BRISTLE_CHILDREN,
    },
];

/// Selects the colored and textured const trees for an adult or baby strider, zipped into the unified
/// tree by [`StriderModel::new`].
fn strider_part_trees(baby: bool) -> (&'static [ModelPartDesc], &'static [TexturedModelPartDesc]) {
    if baby {
        (&STRIDER_BABY_PARTS, &STRIDER_BABY_TEXTURED_PARTS)
    } else {
        (&STRIDER_PARTS, &STRIDER_TEXTURED_PARTS)
    }
}

/// Applies the vanilla `StriderModel.setupAnim` + `{Adult,Baby}StriderModel.customAnimations` to the
/// unified tree: the legs swing (`xRot`) / roll (`zRot`) / lift (`y`) in opposition, the body tracks
/// the look and sways (`zRot`) / bobs (`y`), and the bristles flow with the walk plus an `ageInTicks`
/// ripple — the adult's six bristles on `zRot`, the baby's three on `xRot`. The ridden pose and the
/// saddle are deferred entity-side state.
fn apply_strider_anim(root: &mut ModelPart, baby: bool, instance: &EntityModelInstance) {
    let age = instance.render_state.age_in_ticks;
    let pos = instance.render_state.walk_animation_pos;
    let speed = strider_animation_speed(instance.render_state.walk_animation_speed);
    let head_pitch = instance.render_state.head_pitch.to_radians();
    let head_yaw = instance.render_state.head_yaw.to_radians();
    let (leg_base_y, body_base_y, body_bob_mul, right_leg_x, left_leg_x) = if baby {
        (
            STRIDER_BABY_LEG_BASE_Y,
            STRIDER_BABY_BODY_BASE_Y,
            1.0,
            STRIDER_BABY_RIGHT_LEG_X,
            STRIDER_BABY_LEFT_LEG_X,
        )
    } else {
        (
            STRIDER_LEG_BASE_Y,
            STRIDER_BODY_BASE_Y,
            2.0,
            STRIDER_RIGHT_LEG_X,
            STRIDER_LEFT_LEG_X,
        )
    };

    let right_leg = root.child_at_mut(0);
    right_leg.pose.offset = [
        right_leg_x,
        strider_leg_y(leg_base_y, pos, speed, true),
        0.0,
    ];
    right_leg.pose.rotation = [
        strider_leg_x_rot(pos, speed, true),
        0.0,
        strider_leg_z_rot(pos, speed, true),
    ];

    let left_leg = root.child_at_mut(1);
    left_leg.pose.offset = [
        left_leg_x,
        strider_leg_y(leg_base_y, pos, speed, false),
        0.0,
    ];
    left_leg.pose.rotation = [
        strider_leg_x_rot(pos, speed, false),
        0.0,
        strider_leg_z_rot(pos, speed, false),
    ];

    let body = root.child_at_mut(2);
    body.pose.offset = [
        0.0,
        strider_body_y(body_base_y, body_bob_mul, pos, speed),
        0.0,
    ];
    body.pose.rotation = [head_pitch, head_yaw, strider_body_z_rot(pos, speed)];

    let flow = strider_bristle_flow(pos, speed);
    let top = strider_bristle_top_flow(flow, age);
    let middle = strider_bristle_middle_flow(flow, age);
    let bottom = strider_bristle_bottom_flow(flow, age);
    if baby {
        // The three baby bristles flap on `xRot` (no rest roll), in [front, middle, back] order.
        for (index, add) in [top, middle, bottom].into_iter().enumerate() {
            body.child_at_mut(index).pose.rotation[0] += add;
        }
    } else {
        // The six adult bristles flow on `zRot`: right top/middle/bottom then left top/middle/bottom.
        for (index, add) in [top, middle, bottom, top, middle, bottom]
            .into_iter()
            .enumerate()
        {
            body.child_at_mut(index).pose.rotation[2] += add;
        }
    }
}

/// Mutable strider model, mirroring vanilla `AdultStriderModel` / `BabyStriderModel`. The unified tree
/// is zipped from the const trees selected by `baby` ([`strider_part_trees`]); `setup_anim` runs
/// [`apply_strider_anim`]. The same posed tree drives the colored fallback and the cutout textured
/// layer; the cold/suffocating texture and the saddle layer live outside the model.
pub(in crate::entity_models) struct StriderModel {
    root: ModelPart,
    baby: bool,
}

impl StriderModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        let (colored, textured) = strider_part_trees(baby);
        Self {
            root: ModelPart::root_from_descs(colored, textured),
            baby,
        }
    }
}

impl EntityModel for StriderModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_strider_anim(&mut self.root, self.baby, instance);
    }
}
