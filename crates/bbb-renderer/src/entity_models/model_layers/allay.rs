use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    ALLAY_BLUE,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

// Vanilla 26.1 `AllayModel.createBodyLayer` (atlas 32×32). The model root is the `root` part
// at `(0, 23.5, 0)`; `head` and `body` hang under it, and the arms and wings are children of
// `body` (so the body tilt carries them). `CubeDeformation` insets are baked into the cube
// min/size below (`min -= grow`, `size += 2·grow`).
pub(in crate::entity_models) const ALLAY_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, -5.0, -2.5],
    size: [5.0, 5.0, 5.0],
    color: ALLAY_BLUE,
}];

// Body: a plain `texOffs(0, 10)` 3×4×2 box plus the `texOffs(0, 16)` 3×5×2 box inset by
// `CubeDeformation(-0.2)` → min `+0.2`, size `-0.4` per axis.
pub(in crate::entity_models) const ALLAY_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.5, 0.0, -1.0],
        size: [3.0, 4.0, 2.0],
        color: ALLAY_BLUE,
    },
    ModelCubeDesc {
        min: [-1.3, 0.2, -0.8],
        size: [2.6, 4.6, 1.6],
        color: ALLAY_BLUE,
    },
];

// Arms: 1×4×2 boxes inset by `CubeDeformation(-0.01)` → min `+0.01`, size `-0.02` per axis.
pub(in crate::entity_models) const ALLAY_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.74, -0.49, -0.99],
    size: [0.98, 3.98, 1.98],
    color: ALLAY_BLUE,
}];

pub(in crate::entity_models) const ALLAY_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.24, -0.49, -0.99],
    size: [0.98, 3.98, 1.98],
    color: ALLAY_BLUE,
}];

// Wings: zero-thickness `0×5×8` planes (both wings share the same geometry and UV — unlike
// the vex, neither allay wing is mirrored).
pub(in crate::entity_models) const ALLAY_WING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 1.0, 0.0],
    size: [0.0, 5.0, 8.0],
    color: ALLAY_BLUE,
}];

pub(in crate::entity_models) const ALLAY_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, -3.99, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const ALLAY_BODY_POSE: PartPose = PartPose {
    offset: [0.0, -4.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const ALLAY_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-1.75, 0.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const ALLAY_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [1.75, 0.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const ALLAY_RIGHT_WING_POSE: PartPose = PartPose {
    offset: [-0.5, 0.0, 0.6],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const ALLAY_LEFT_WING_POSE: PartPose = PartPose {
    offset: [0.5, 0.0, 0.6],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla `AllayModel` default root height (`PartPose.offset` y), animated by the idle bob.
pub(in crate::entity_models) const ALLAY_ROOT_BASE_Y: f32 = 23.5;
/// Vanilla `AllayModel.setupAnim` wing yaw base `±π/4`: `rightWing.yRot = -π/4 + flapAmount`,
/// `leftWing.yRot = π/4 - flapAmount`.
pub(in crate::entity_models) const ALLAY_WING_Y_ROT_BASE: f32 = std::f32::consts::FRAC_PI_4;
/// Vanilla `AllayModel.setupAnim` wing rest pitch / arm rest roll factor (`0.43633232` rad ≈
/// 25°): `wing.xRot = 0.43633232·(1 - flyingFactor)` and the arm bob centres on `0.43633232`.
pub(in crate::entity_models) const ALLAY_REST_ANGLE: f32 = 0.436_332_32;
/// Vanilla `AllayModel.setupAnim` flying body tilt `flyingFactor·π/4`.
pub(in crate::entity_models) const ALLAY_BODY_FLYING_X_ROT: f32 = std::f32::consts::FRAC_PI_4;

/// Vanilla `AllayModel.setupAnim`: `flyingFactor = min(walkAnimationSpeed / 0.3, 1)`.
pub(in crate::entity_models) fn allay_flying_factor(walk_animation_speed: f32) -> f32 {
    (walk_animation_speed / 0.3).min(1.0)
}

/// Vanilla `AllayModel.setupAnim` wing flap:
/// `flapAmount = cos(ageInTicks·20° + walkAnimationPos)·π·0.15 + walkAnimationSpeed`.
pub(in crate::entity_models) fn allay_wing_flap_amount(
    age_in_ticks: f32,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> f32 {
    let flap_speed = age_in_ticks * 20.0_f32.to_radians() + walk_animation_pos;
    flap_speed.cos() * std::f32::consts::PI * 0.15 + walk_animation_speed
}

/// Vanilla `AllayModel.setupAnim` idle bob phase: `idleBobSpeed = ageInTicks·9°` (radians).
fn allay_idle_bob_speed(age_in_ticks: f32) -> f32 {
    age_in_ticks * 9.0_f32.to_radians()
}

/// Vanilla `AllayModel.setupAnim` wing rest pitch: `wing.xRot = 0.43633232·(1 - flyingFactor)`.
pub(in crate::entity_models) fn allay_wing_rest_x_rot(walk_animation_speed: f32) -> f32 {
    ALLAY_REST_ANGLE * (1.0 - allay_flying_factor(walk_animation_speed))
}

/// Vanilla `AllayModel.setupAnim` body flying tilt: `body.xRot = flyingFactor·π/4`.
pub(in crate::entity_models) fn allay_body_x_rot(walk_animation_speed: f32) -> f32 {
    allay_flying_factor(walk_animation_speed) * ALLAY_BODY_FLYING_X_ROT
}

/// Vanilla `AllayModel.setupAnim` vertical bob: `root.y = 23.5 + cos(idleBobSpeed)·0.25·(1 -
/// flyingFactor)`.
pub(in crate::entity_models) fn allay_root_y(age_in_ticks: f32, walk_animation_speed: f32) -> f32 {
    let idle_bob_factor = 1.0 - allay_flying_factor(walk_animation_speed);
    ALLAY_ROOT_BASE_Y + allay_idle_bob_speed(age_in_ticks).cos() * 0.25 * idle_bob_factor
}

/// Vanilla `AllayModel.setupAnim` non-holding arm idle roll: `armIdleBobAmount = 0.43633232 -
/// cos(idleBobSpeed + 3π/2)·π·0.075·(1 - flyingFactor)`, with `leftArm.zRot = -amount` and
/// `rightArm.zRot = amount`. The held-item factor (which would scale this to zero and add the
/// `±0.27925268` arm yaw) is deferred entity-side state, so this assumes `holdingItem = 0`.
pub(in crate::entity_models) fn allay_arm_idle_bob_amount(
    age_in_ticks: f32,
    walk_animation_speed: f32,
) -> f32 {
    let idle_bob_factor = 1.0 - allay_flying_factor(walk_animation_speed);
    ALLAY_REST_ANGLE
        - (allay_idle_bob_speed(age_in_ticks) + std::f32::consts::PI * 1.5).cos()
            * std::f32::consts::PI
            * 0.075
            * idle_bob_factor
}

// Textured counterparts of the allay cubes (atlas 32×32). `CubeDeformation` inflates the
// geometry (min/size) but the `uv_size` keeps the BASE box, exactly as vanilla bakes it.
pub(in crate::entity_models) const ALLAY_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.5, -5.0, -2.5],
        size: [5.0, 5.0, 5.0],
        uv_size: [5.0, 5.0, 5.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ALLAY_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-1.5, 0.0, -1.0],
        size: [3.0, 4.0, 2.0],
        uv_size: [3.0, 4.0, 2.0],
        tex: [0.0, 10.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        // `CubeDeformation(-0.2)`: geometry inset, but uv_size keeps the 3×5×2 base box.
        min: [-1.3, 0.2, -0.8],
        size: [2.6, 4.6, 1.6],
        uv_size: [3.0, 5.0, 2.0],
        tex: [0.0, 16.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const ALLAY_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-0.74, -0.49, -0.99],
        size: [0.98, 3.98, 1.98],
        uv_size: [1.0, 4.0, 2.0],
        tex: [23.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ALLAY_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-0.24, -0.49, -0.99],
        size: [0.98, 3.98, 1.98],
        uv_size: [1.0, 4.0, 2.0],
        tex: [23.0, 6.0],
        mirror: false,
    }];

// Both wings share `texOffs(16, 14)`; neither wing's UV is mirrored.
pub(in crate::entity_models) const ALLAY_TEXTURED_WING: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 1.0, 0.0],
        size: [0.0, 5.0, 8.0],
        uv_size: [0.0, 5.0, 8.0],
        tex: [16.0, 14.0],
        mirror: false,
    }];

/// The allay `root` part's bind pose (vanilla `(0, 23.5, 0)`); the idle bob animates its `y`.
const ALLAY_ROOT_POSE: PartPose = PartPose {
    offset: [0.0, ALLAY_ROOT_BASE_Y, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

// Colored allay tree: `root` (the bobbing pivot, no cubes) → `head`, `body`; `body` → the two arms
// and the two wings. Mirrors vanilla `AllayModel.createBodyLayer`. Zipped with the textured tree by
// `AllayModel::new`; the dynamic poses are applied in `setup_anim`.
const ALLAY_BODY_CHILDREN: [ModelPartDesc; 4] = [
    ModelPartDesc {
        pose: ALLAY_RIGHT_ARM_POSE,
        cubes: &ALLAY_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: ALLAY_LEFT_ARM_POSE,
        cubes: &ALLAY_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: ALLAY_RIGHT_WING_POSE,
        cubes: &ALLAY_WING,
        children: &[],
    },
    ModelPartDesc {
        pose: ALLAY_LEFT_WING_POSE,
        cubes: &ALLAY_WING,
        children: &[],
    },
];
const ALLAY_ROOT_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: ALLAY_HEAD_POSE,
        cubes: &ALLAY_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: ALLAY_BODY_POSE,
        cubes: &ALLAY_BODY,
        children: &ALLAY_BODY_CHILDREN,
    },
];
pub(in crate::entity_models) const ALLAY_PARTS: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: ALLAY_ROOT_POSE,
    cubes: &[],
    children: &ALLAY_ROOT_CHILDREN,
}];

// Textured counterpart of `ALLAY_PARTS` (same hierarchy and bind poses, UV cubes).
const ALLAY_TEXTURED_BODY_CHILDREN: [TexturedModelPartDesc; 4] = [
    TexturedModelPartDesc {
        pose: ALLAY_RIGHT_ARM_POSE,
        cubes: &ALLAY_TEXTURED_RIGHT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ALLAY_LEFT_ARM_POSE,
        cubes: &ALLAY_TEXTURED_LEFT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ALLAY_RIGHT_WING_POSE,
        cubes: &ALLAY_TEXTURED_WING,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ALLAY_LEFT_WING_POSE,
        cubes: &ALLAY_TEXTURED_WING,
        children: &[],
    },
];
const ALLAY_TEXTURED_ROOT_CHILDREN: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: ALLAY_HEAD_POSE,
        cubes: &ALLAY_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ALLAY_BODY_POSE,
        cubes: &ALLAY_TEXTURED_BODY,
        children: &ALLAY_TEXTURED_BODY_CHILDREN,
    },
];
pub(in crate::entity_models) const ALLAY_TEXTURED_PARTS: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: ALLAY_ROOT_POSE,
        cubes: &[],
        children: &ALLAY_TEXTURED_ROOT_CHILDREN,
    }];

/// Applies the vanilla `AllayModel.setupAnim` idle/flying pose to the unified tree: the `root` bob,
/// the head look, the body flying tilt, the arm idle bob, and the wing flap. The dance pose and the
/// held-item arms are deferred entity-side state, so this is the non-dancing, empty-handed pose. Every
/// value is set absolutely each frame, reproducing the hand-walked emit exactly.
fn apply_allay_anim(root: &mut ModelPart, instance: &EntityModelInstance) {
    let age = instance.render_state.age_in_ticks;
    let walk_pos = instance.render_state.walk_animation_pos;
    let walk_speed = instance.render_state.walk_animation_speed;
    let head_pitch = instance.render_state.head_pitch.to_radians();
    let head_yaw = instance.render_state.head_yaw.to_radians();
    let arm_bob = allay_arm_idle_bob_amount(age, walk_speed);
    let wing_x = allay_wing_rest_x_rot(walk_speed);
    let flap = allay_wing_flap_amount(age, walk_pos, walk_speed);

    let allay_root = root.child_at_mut(0);
    allay_root.pose.offset[1] = allay_root_y(age, walk_speed);
    allay_root.child_at_mut(0).pose.rotation = [head_pitch, head_yaw, 0.0];

    let body = allay_root.child_at_mut(1);
    body.pose.rotation = [allay_body_x_rot(walk_speed), 0.0, 0.0];
    body.child_at_mut(0).pose.rotation = [0.0, 0.0, arm_bob];
    body.child_at_mut(1).pose.rotation = [0.0, 0.0, -arm_bob];
    body.child_at_mut(2).pose.rotation = [wing_x, -ALLAY_WING_Y_ROT_BASE + flap, 0.0];
    body.child_at_mut(3).pose.rotation = [wing_x, ALLAY_WING_Y_ROT_BASE - flap, 0.0];
}

/// Mutable allay model, mirroring vanilla `AllayModel`. The unified tree is zipped from the `root` →
/// (head, body → arms/wings) hierarchy ([`ALLAY_PARTS`] / [`ALLAY_TEXTURED_PARTS`]); `setup_anim` runs
/// [`apply_allay_anim`]. The same posed tree drives the colored fallback and the single translucent
/// textured layer.
pub(in crate::entity_models) struct AllayModel {
    root: ModelPart,
}

impl AllayModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::root_from_descs(&ALLAY_PARTS, &ALLAY_TEXTURED_PARTS),
        }
    }
}

impl EntityModel for AllayModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_allay_anim(&mut self.root, instance);
    }
}
