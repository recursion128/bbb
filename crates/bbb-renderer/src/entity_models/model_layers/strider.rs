use super::{PartPose, PART_POSE_ZERO, STRIDER_LEG, STRIDER_MAROON};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 `AdultStriderModel.createBodyLayer` (atlas 64×128). The mesh root parents the
// two legs and the body directly; the six bristles hang under the body. The legs and body are
// repositioned/rotated by `StriderModel.setupAnim` + `AdultStriderModel.customAnimations`, so
// their poses are built per frame from the offset constants and the animation curves below.
// Each cube carries both render paths' data: the colored debug tint (`STRIDER_MAROON` /
// `STRIDER_LEG`) and the textured `uv_size` / `texOffs` / `mirror` (`CubeDeformation.NONE`, so
// `uv_size == size`).
pub(in crate::entity_models) const STRIDER_BODY: [ModelCube; 1] = [ModelCube::new(
    [-8.0, -6.0, -8.0],
    [16.0, 14.0, 16.0],
    STRIDER_MAROON,
    [16.0, 14.0, 16.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const STRIDER_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 16.0, 4.0],
    STRIDER_LEG,
    [4.0, 16.0, 4.0],
    [0.0, 32.0],
    false,
)];

pub(in crate::entity_models) const STRIDER_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 16.0, 4.0],
    STRIDER_LEG,
    [4.0, 16.0, 4.0],
    [0.0, 55.0],
    false,
)];

// Bristles are zero-thickness `12×0×16` planes. The right bristles are mirrored (box at
// `-12`), the left are not (box at `0`); each bristle carries its own `texOffs`, so the three
// right bristles and the three left bristles get distinct cubes.
const fn strider_right_bristle(tex: [f32; 2]) -> ModelCube {
    ModelCube::new(
        [-12.0, 0.0, 0.0],
        [12.0, 0.0, 16.0],
        STRIDER_MAROON,
        [12.0, 0.0, 16.0],
        tex,
        true,
    )
}

const fn strider_left_bristle(tex: [f32; 2]) -> ModelCube {
    ModelCube::new(
        [0.0, 0.0, 0.0],
        [12.0, 0.0, 16.0],
        STRIDER_MAROON,
        [12.0, 0.0, 16.0],
        tex,
        false,
    )
}

pub(in crate::entity_models) const STRIDER_RIGHT_TOP_BRISTLE: [ModelCube; 1] =
    [strider_right_bristle([16.0, 33.0])];
pub(in crate::entity_models) const STRIDER_RIGHT_MIDDLE_BRISTLE: [ModelCube; 1] =
    [strider_right_bristle([16.0, 49.0])];
pub(in crate::entity_models) const STRIDER_RIGHT_BOTTOM_BRISTLE: [ModelCube; 1] =
    [strider_right_bristle([16.0, 65.0])];
pub(in crate::entity_models) const STRIDER_LEFT_TOP_BRISTLE: [ModelCube; 1] =
    [strider_left_bristle([16.0, 33.0])];
pub(in crate::entity_models) const STRIDER_LEFT_MIDDLE_BRISTLE: [ModelCube; 1] =
    [strider_left_bristle([16.0, 49.0])];
pub(in crate::entity_models) const STRIDER_LEFT_BOTTOM_BRISTLE: [ModelCube; 1] =
    [strider_left_bristle([16.0, 65.0])];

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
pub(in crate::entity_models) const STRIDER_BABY_BODY: [ModelCube; 1] = [ModelCube::new(
    [-3.5, -3.75, -4.0],
    [7.0, 7.0, 8.0],
    STRIDER_MAROON,
    [7.0, 7.0, 8.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const STRIDER_BABY_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 4.0, 2.0],
    STRIDER_LEG,
    [2.0, 4.0, 2.0],
    [0.0, 24.0],
    false,
)];

pub(in crate::entity_models) const STRIDER_BABY_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 4.0, 2.0],
    STRIDER_LEG,
    [2.0, 4.0, 2.0],
    [8.0, 24.0],
    false,
)];

// Baby bristles are zero-thickness `7×3×0` planes; each carries its own `texOffs`.
const fn strider_baby_bristle(tex: [f32; 2]) -> ModelCube {
    ModelCube::new(
        [-3.5, -2.5, 0.0],
        [7.0, 3.0, 0.0],
        STRIDER_MAROON,
        [7.0, 3.0, 0.0],
        tex,
        false,
    )
}

pub(in crate::entity_models) const STRIDER_BABY_FRONT_BRISTLE: [ModelCube; 1] =
    [strider_baby_bristle([0.0, 15.0])];
pub(in crate::entity_models) const STRIDER_BABY_MIDDLE_BRISTLE: [ModelCube; 1] =
    [strider_baby_bristle([0.0, 18.0])];
pub(in crate::entity_models) const STRIDER_BABY_BACK_BRISTLE: [ModelCube; 1] =
    [strider_baby_bristle([0.0, 21.0])];

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

/// Builds the unified adult strider tree: the synthetic root parents the right leg, left leg, and
/// body (the emit order), with the body parenting its six bristles (right top/middle/bottom then left
/// top/middle/bottom). Mirrors vanilla `AdultStriderModel.createBodyLayer`. Each cube carries both the
/// colored tint and the textured UV, so one tree drives both render paths.
fn strider_adult_root() -> ModelPart {
    let body = ModelPart::new(
        STRIDER_BODY_POSE,
        STRIDER_BODY.to_vec(),
        vec![
            (
                "right_top_bristle",
                ModelPart::leaf(
                    STRIDER_RIGHT_TOP_BRISTLE_POSE,
                    STRIDER_RIGHT_TOP_BRISTLE.to_vec(),
                ),
            ),
            (
                "right_middle_bristle",
                ModelPart::leaf(
                    STRIDER_RIGHT_MIDDLE_BRISTLE_POSE,
                    STRIDER_RIGHT_MIDDLE_BRISTLE.to_vec(),
                ),
            ),
            (
                "right_bottom_bristle",
                ModelPart::leaf(
                    STRIDER_RIGHT_BOTTOM_BRISTLE_POSE,
                    STRIDER_RIGHT_BOTTOM_BRISTLE.to_vec(),
                ),
            ),
            (
                "left_top_bristle",
                ModelPart::leaf(
                    STRIDER_LEFT_TOP_BRISTLE_POSE,
                    STRIDER_LEFT_TOP_BRISTLE.to_vec(),
                ),
            ),
            (
                "left_middle_bristle",
                ModelPart::leaf(
                    STRIDER_LEFT_MIDDLE_BRISTLE_POSE,
                    STRIDER_LEFT_MIDDLE_BRISTLE.to_vec(),
                ),
            ),
            (
                "left_bottom_bristle",
                ModelPart::leaf(
                    STRIDER_LEFT_BOTTOM_BRISTLE_POSE,
                    STRIDER_LEFT_BOTTOM_BRISTLE.to_vec(),
                ),
            ),
        ],
    );
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            (
                "right_leg",
                ModelPart::leaf(STRIDER_RIGHT_LEG_POSE, STRIDER_RIGHT_LEG.to_vec()),
            ),
            (
                "left_leg",
                ModelPart::leaf(STRIDER_LEFT_LEG_POSE, STRIDER_LEFT_LEG.to_vec()),
            ),
            ("body", body),
        ],
    )
}

/// Builds the unified baby strider tree: the synthetic root parents the right leg, left leg, and body
/// (the emit order), with the body parenting its three bristles (front, middle, back), which flap on
/// `xRot`. Mirrors vanilla `BabyStriderModel.createBodyLayer`.
fn strider_baby_root() -> ModelPart {
    let body = ModelPart::new(
        STRIDER_BABY_BODY_POSE,
        STRIDER_BABY_BODY.to_vec(),
        vec![
            (
                "front_bristle",
                ModelPart::leaf(
                    STRIDER_BABY_FRONT_BRISTLE_POSE,
                    STRIDER_BABY_FRONT_BRISTLE.to_vec(),
                ),
            ),
            (
                "middle_bristle",
                ModelPart::leaf(
                    STRIDER_BABY_MIDDLE_BRISTLE_POSE,
                    STRIDER_BABY_MIDDLE_BRISTLE.to_vec(),
                ),
            ),
            (
                "back_bristle",
                ModelPart::leaf(
                    STRIDER_BABY_BACK_BRISTLE_POSE,
                    STRIDER_BABY_BACK_BRISTLE.to_vec(),
                ),
            ),
        ],
    );
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            (
                "right_leg",
                ModelPart::leaf(STRIDER_BABY_RIGHT_LEG_POSE, STRIDER_BABY_RIGHT_LEG.to_vec()),
            ),
            (
                "left_leg",
                ModelPart::leaf(STRIDER_BABY_LEFT_LEG_POSE, STRIDER_BABY_LEFT_LEG.to_vec()),
            ),
            ("body", body),
        ],
    )
}

/// Applies the vanilla `StriderModel.setupAnim` + `{Adult,Baby}StriderModel.customAnimations` to the
/// unified tree: the legs swing (`xRot`) / roll (`zRot`) / lift (`y`) in opposition, the body tracks
/// the look and sways (`zRot`) / bobs (`y`), and the bristles flow with the walk plus an `ageInTicks`
/// ripple — the adult's six bristles on `zRot`, the baby's three on `xRot`. Ridden striders zero body
/// look pitch/yaw, matching `StriderRenderState.isRidden`.
fn apply_strider_anim(root: &mut ModelPart, baby: bool, instance: &EntityModelInstance) {
    let age = instance.render_state.age_in_ticks;
    let pos = instance.render_state.walk_animation_pos;
    let speed = strider_animation_speed(instance.render_state.walk_animation_speed);
    let (head_pitch, head_yaw) = if instance.render_state.strider_ridden {
        (0.0, 0.0)
    } else {
        (
            instance.render_state.head_pitch.to_radians(),
            instance.render_state.head_yaw.to_radians(),
        )
    };
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

    let right_leg = root.child_mut("right_leg");
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

    let left_leg = root.child_mut("left_leg");
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

    let body = root.child_mut("body");
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
        // The three baby bristles flap on `xRot` (no rest roll), in [front, middle, back] order. The
        // `+=` is relative to each bristle's bind pose.
        body.child_mut("front_bristle").pose.rotation[0] += top;
        body.child_mut("middle_bristle").pose.rotation[0] += middle;
        body.child_mut("back_bristle").pose.rotation[0] += bottom;
    } else {
        // The six adult bristles flow on `zRot`: right top/middle/bottom then left top/middle/bottom.
        // Each `+=` adds to the bristle's non-zero rest `zRot`.
        body.child_mut("right_top_bristle").pose.rotation[2] += top;
        body.child_mut("right_middle_bristle").pose.rotation[2] += middle;
        body.child_mut("right_bottom_bristle").pose.rotation[2] += bottom;
        body.child_mut("left_top_bristle").pose.rotation[2] += top;
        body.child_mut("left_middle_bristle").pose.rotation[2] += middle;
        body.child_mut("left_bottom_bristle").pose.rotation[2] += bottom;
    }
}

/// Mutable strider model, mirroring vanilla `AdultStriderModel` / `BabyStriderModel`. The unified tree
/// is built once with named children selected by `baby` ([`strider_adult_root`] /
/// [`strider_baby_root`]): the synthetic root → (`right_leg`, `left_leg`, `body`), with `body`
/// parenting the bristles. Each cube carries both the colored tint and the textured UV, so one tree
/// drives both render paths; `setup_anim` runs [`apply_strider_anim`]. The same posed tree drives the
/// colored fallback, the base cutout textured layer, and the adult saddle equipment layer; the
/// cold/suffocating texture swap lives outside the model.
pub(in crate::entity_models) struct StriderModel {
    root: ModelPart,
    baby: bool,
}

impl StriderModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        let root = if baby {
            strider_baby_root()
        } else {
            strider_adult_root()
        };
        Self { root, baby }
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
