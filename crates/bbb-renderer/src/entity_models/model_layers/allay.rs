use super::{PartPose, ALLAY_BLUE, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_ALLAY: &str = "minecraft:allay#main";

// Vanilla 26.1 `AllayModel.createBodyLayer` (atlas 32×32). The model root is the `root` part
// at `(0, 23.5, 0)`; `head` and `body` hang under it, and the arms and wings are children of
// `body` (so the body tilt carries them). `CubeDeformation` insets are baked into the cube
// min/size below (`min -= grow`, `size += 2·grow`); the textured `uv_size` keeps the BASE box,
// exactly as vanilla bakes it. Each cube carries both render paths' data: the colored debug
// tint (`ALLAY_BLUE`) and the textured `uv_size` / `texOffs` / `mirror`.
pub(in crate::entity_models) const ALLAY_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-2.5, -5.0, -2.5],
    [5.0, 5.0, 5.0],
    ALLAY_BLUE,
    [5.0, 5.0, 5.0],
    [0.0, 0.0],
    false,
)];

// Body: a plain `texOffs(0, 10)` 3×4×2 box plus the `texOffs(0, 16)` 3×5×2 box inset by
// `CubeDeformation(-0.2)` → min `+0.2`, size `-0.4` per axis (geometry inset, uv_size keeps the
// 3×5×2 base box).
pub(in crate::entity_models) const ALLAY_BODY: [ModelCube; 2] = [
    ModelCube::new(
        [-1.5, 0.0, -1.0],
        [3.0, 4.0, 2.0],
        ALLAY_BLUE,
        [3.0, 4.0, 2.0],
        [0.0, 10.0],
        false,
    ),
    ModelCube::new(
        [-1.3, 0.2, -0.8],
        [2.6, 4.6, 1.6],
        ALLAY_BLUE,
        [3.0, 5.0, 2.0],
        [0.0, 16.0],
        false,
    ),
];

// Arms: 1×4×2 boxes inset by `CubeDeformation(-0.01)` → min `+0.01`, size `-0.02` per axis (the
// uv_size keeps the 1×4×2 base box). The right and left arms differ in their box origin (`-0.75`
// vs `-0.25`) and `texOffs(23, 0)` / `texOffs(23, 6)`.
pub(in crate::entity_models) const ALLAY_RIGHT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-0.74, -0.49, -0.99],
    [0.98, 3.98, 1.98],
    ALLAY_BLUE,
    [1.0, 4.0, 2.0],
    [23.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ALLAY_LEFT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-0.24, -0.49, -0.99],
    [0.98, 3.98, 1.98],
    ALLAY_BLUE,
    [1.0, 4.0, 2.0],
    [23.0, 6.0],
    false,
)];

// Wings: zero-thickness `0×5×8` planes whose box starts at y=1. Both wings share `texOffs(16, 14)`
// and the same geometry and UV — unlike the vex, neither allay wing is mirrored, so a single cube
// const drives both.
pub(in crate::entity_models) const ALLAY_WING: [ModelCube; 1] = [ModelCube::new(
    [0.0, 1.0, 0.0],
    [0.0, 5.0, 8.0],
    ALLAY_BLUE,
    [0.0, 5.0, 8.0],
    [16.0, 14.0],
    false,
)];

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
/// Vanilla `AllayModel.setupAnim` held-item arm yaw factor:
/// `rightArm.yRot = 0.27925268·holding`, `leftArm.yRot = -0.27925268·holding`.
pub(in crate::entity_models) const ALLAY_HELD_ITEM_ARM_Y_ROT: f32 = 0.279_252_68;
/// Vanilla `AllayModel.setupAnim` minimum held-item arm pitch while idle:
/// `Mth.lerp(flyingFactor, -π/3, -1.134464)`.
pub(in crate::entity_models) const ALLAY_MIN_HAND_HOLDING_ITEM_X_ROT: f32 =
    -std::f32::consts::FRAC_PI_3;
/// Vanilla `AllayModel.setupAnim` maximum held-item arm pitch while flying.
pub(in crate::entity_models) const ALLAY_MAX_HAND_HOLDING_ITEM_X_ROT: f32 = -1.134_464;

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

/// Vanilla `AllayModel.setupAnim` empty-handed arm idle roll.
#[cfg(test)]
pub(in crate::entity_models) fn allay_arm_idle_bob_amount(
    age_in_ticks: f32,
    walk_animation_speed: f32,
) -> f32 {
    allay_arm_z_rot_amount(age_in_ticks, walk_animation_speed, 0.0)
}

/// Vanilla `AllayModel.setupAnim` arm roll:
/// `armIdleBobAmount = 0.43633232 - cos(idleBobSpeed + 3π/2)·π·0.075·(1 - flyingFactor)·(1 -
/// holdingItemFactor)`, with `leftArm.zRot = -amount` and `rightArm.zRot = amount`.
pub(in crate::entity_models) fn allay_arm_z_rot_amount(
    age_in_ticks: f32,
    walk_animation_speed: f32,
    holding_item_progress: f32,
) -> f32 {
    let idle_bob_factor = 1.0 - allay_flying_factor(walk_animation_speed);
    ALLAY_REST_ANGLE
        - (allay_idle_bob_speed(age_in_ticks) + std::f32::consts::PI * 1.5).cos()
            * std::f32::consts::PI
            * 0.075
            * idle_bob_factor
            * (1.0 - holding_item_progress)
}

/// Vanilla `AllayModel.setupAnim` held-item arm pitch:
/// `holdingItemFactor * Mth.lerp(flyingFactor, -π/3, -1.134464)`.
pub(in crate::entity_models) fn allay_arm_holding_x_rot(
    walk_animation_speed: f32,
    holding_item_progress: f32,
) -> f32 {
    let flying_factor = allay_flying_factor(walk_animation_speed);
    holding_item_progress
        * (ALLAY_MIN_HAND_HOLDING_ITEM_X_ROT
            + (ALLAY_MAX_HAND_HOLDING_ITEM_X_ROT - ALLAY_MIN_HAND_HOLDING_ITEM_X_ROT)
                * flying_factor)
}

/// Vanilla `AllayModel.setupAnim` dance phase: `danceSpeed = ageInTicks·8° + walkAnimationSpeed`
/// (radians). The shared cosine of this phase drives the body sway (`16°`), the head Y-tilt
/// (`30°`), and the head Z-tilt (`14°`).
fn allay_dance_speed(age_in_ticks: f32, walk_animation_speed: f32) -> f32 {
    age_in_ticks * 8.0_f32.to_radians() + walk_animation_speed
}

/// The allay `root` part's bind pose (vanilla `(0, 23.5, 0)`); the idle bob animates its `y`.
const ALLAY_ROOT_POSE: PartPose = PartPose {
    offset: [0.0, ALLAY_ROOT_BASE_Y, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Applies the vanilla `AllayModel.setupAnim` pose to the unified tree: the `root` bob, the head
/// look (or the dance head tilt + body sway/spin), the body flying tilt, the held-item arm blend, and
/// the wing flap. Every value is set absolutely each frame, reproducing the hand-walked emit exactly.
fn apply_allay_anim(root: &mut ModelPart, instance: &EntityModelInstance) {
    let age = instance.render_state.age_in_ticks;
    let walk_pos = instance.render_state.walk_animation_pos;
    let walk_speed = instance.render_state.walk_animation_speed;
    let head_pitch = instance.render_state.head_pitch.to_radians();
    let head_yaw = instance.render_state.head_yaw.to_radians();
    let holding_item_progress = instance.render_state.allay_holding_item_progress;
    let arm_x = allay_arm_holding_x_rot(walk_speed, holding_item_progress);
    let arm_z = allay_arm_z_rot_amount(age, walk_speed, holding_item_progress);
    let wing_x = allay_wing_rest_x_rot(walk_speed);
    let flap = allay_wing_flap_amount(age, walk_pos, walk_speed);

    let is_dancing = instance.render_state.allay_dancing;
    let is_spinning = instance.render_state.allay_spinning;
    let spin_progress = instance.render_state.allay_spinning_progress;

    let allay_root = root.child_mut("root");
    allay_root.pose.offset[1] = allay_root_y(age, walk_speed);
    if is_dancing {
        // Vanilla dance pose: the body sways (`root.zRot`) and the head tilts in Y/Z, all
        // cross-faded into a full `4π·spinningProgress` body spin (`root.yRot`) by the spin
        // blend. The head pitch/look is dropped while dancing (vanilla leaves `head.xRot` at
        // its reset bind pose), and `root.yRot` only spins during the `isSpinning` sub-window.
        let dance_cos = allay_dance_speed(age, walk_speed).cos();
        let dance_frequency = dance_cos * 16.0_f32.to_radians();
        let head_tilt_y = dance_cos * 30.0_f32.to_radians();
        let head_tilt_z = dance_cos * 14.0_f32.to_radians();
        let root_y_rot = if is_spinning {
            4.0 * std::f32::consts::PI * spin_progress
        } else {
            0.0
        };
        allay_root.pose.rotation = [0.0, root_y_rot, dance_frequency * (1.0 - spin_progress)];
        allay_root.child_mut("head").pose.rotation = [
            0.0,
            head_tilt_y * (1.0 - spin_progress),
            head_tilt_z * (1.0 - spin_progress),
        ];
    } else {
        allay_root.pose.rotation = [0.0, 0.0, 0.0];
        allay_root.child_mut("head").pose.rotation = [head_pitch, head_yaw, 0.0];
    }

    let body = allay_root.child_mut("body");
    body.pose.rotation = [allay_body_x_rot(walk_speed), 0.0, 0.0];
    body.child_mut("right_arm").pose.rotation = [
        arm_x,
        ALLAY_HELD_ITEM_ARM_Y_ROT * holding_item_progress,
        arm_z,
    ];
    body.child_mut("left_arm").pose.rotation = [
        arm_x,
        -ALLAY_HELD_ITEM_ARM_Y_ROT * holding_item_progress,
        -arm_z,
    ];
    body.child_mut("right_wing").pose.rotation = [wing_x, -ALLAY_WING_Y_ROT_BASE + flap, 0.0];
    body.child_mut("left_wing").pose.rotation = [wing_x, ALLAY_WING_Y_ROT_BASE - flap, 0.0];
}

/// Mutable allay model, mirroring vanilla `AllayModel`. The unified tree is built once with named
/// children: the bobbing `root` pivot → (`head`, `body`), with `body` parenting the right arm, left
/// arm, right wing, and left wing (the emit order, preserved for byte-identical meshes). Each cube
/// carries both the colored tint and the textured UV, so one tree drives both render paths;
/// `setup_anim` runs [`apply_allay_anim`]. The same posed tree drives the colored fallback and the
/// single translucent textured layer.
pub(in crate::entity_models) struct AllayModel {
    root: ModelPart,
}

impl AllayModel {
    pub(in crate::entity_models) fn new() -> Self {
        let body = ModelPart::new(
            ALLAY_BODY_POSE,
            ALLAY_BODY.to_vec(),
            vec![
                (
                    "right_arm",
                    ModelPart::leaf(ALLAY_RIGHT_ARM_POSE, ALLAY_RIGHT_ARM.to_vec()),
                ),
                (
                    "left_arm",
                    ModelPart::leaf(ALLAY_LEFT_ARM_POSE, ALLAY_LEFT_ARM.to_vec()),
                ),
                (
                    "right_wing",
                    ModelPart::leaf(ALLAY_RIGHT_WING_POSE, ALLAY_WING.to_vec()),
                ),
                (
                    "left_wing",
                    ModelPart::leaf(ALLAY_LEFT_WING_POSE, ALLAY_WING.to_vec()),
                ),
            ],
        );
        let allay_root = ModelPart::new(
            ALLAY_ROOT_POSE,
            Vec::new(),
            vec![
                (
                    "head",
                    ModelPart::leaf(ALLAY_HEAD_POSE, ALLAY_HEAD.to_vec()),
                ),
                ("body", body),
            ],
        );
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![("root", allay_root)]),
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
