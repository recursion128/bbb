use super::{ModelCubeDesc, PartPose, STRIDER_LEG, STRIDER_MAROON};

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
