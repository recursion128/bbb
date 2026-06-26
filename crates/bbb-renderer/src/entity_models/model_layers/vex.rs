use super::{PartPose, PART_POSE_ZERO, VEX_GREY};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 `VexModel.createBodyLayer` (atlas 32×32). The model root is the `root` part
// at `(0, -2.5, 0)`; `head` and `body` hang under it, and the arms and wings are children
// of `body` (so the body tilt carries them). `CubeDeformation` insets are baked into the
// cube min/size below (`min -= grow`, `size += 2·grow`); the textured `uv_size` keeps the
// BASE box, exactly as vanilla bakes it. Each cube carries both render paths' data: the
// colored debug tint (`VEX_GREY`) and the textured `uv_size` / `texOffs` / `mirror`.
pub(in crate::entity_models) const VEX_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-2.5, -5.0, -2.5],
    [5.0, 5.0, 5.0],
    VEX_GREY,
    [5.0, 5.0, 5.0],
    [0.0, 0.0],
    false,
)];

// Body: a plain `texOffs(0, 10)` 3×4×2 box plus the `texOffs(0, 16)` 3×5×2 box inset by
// `CubeDeformation(-0.2)` → min `+0.2`, size `-0.4` per axis (geometry inset, uv_size keeps
// the 3×5×2 base box).
pub(in crate::entity_models) const VEX_BODY: [ModelCube; 2] = [
    ModelCube::new(
        [-1.5, 0.0, -1.0],
        [3.0, 4.0, 2.0],
        VEX_GREY,
        [3.0, 4.0, 2.0],
        [0.0, 10.0],
        false,
    ),
    ModelCube::new(
        [-1.3, 1.2, -0.8],
        [2.6, 4.6, 1.6],
        VEX_GREY,
        [3.0, 5.0, 2.0],
        [0.0, 16.0],
        false,
    ),
];

// Arms: 2×4×2 boxes inset by `CubeDeformation(-0.1)` → min `+0.1`, size `-0.2` per axis (the
// uv_size keeps the 2×4×2 base box).
pub(in crate::entity_models) const VEX_RIGHT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.15, -0.4, -0.9],
    [1.8, 3.8, 1.8],
    VEX_GREY,
    [2.0, 4.0, 2.0],
    [23.0, 0.0],
    false,
)];

pub(in crate::entity_models) const VEX_LEFT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-0.65, -0.4, -0.9],
    [1.8, 3.8, 1.8],
    VEX_GREY,
    [2.0, 4.0, 2.0],
    [23.0, 6.0],
    false,
)];

// Wings: zero-thickness `0×5×8` planes. Both share `texOffs(16, 14)`; the left wing's UV is
// mirrored, so the two wings use distinct cubes.
pub(in crate::entity_models) const VEX_LEFT_WING: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, 0.0],
    [0.0, 5.0, 8.0],
    VEX_GREY,
    [0.0, 5.0, 8.0],
    [16.0, 14.0],
    true,
)];

pub(in crate::entity_models) const VEX_RIGHT_WING: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, 0.0],
    [0.0, 5.0, 8.0],
    VEX_GREY,
    [0.0, 5.0, 8.0],
    [16.0, 14.0],
    false,
)];

pub(in crate::entity_models) const VEX_ROOT_POSE: PartPose = PartPose {
    offset: [0.0, -2.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const VEX_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 20.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const VEX_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 20.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const VEX_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-1.75, 0.25, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const VEX_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [1.75, 0.25, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const VEX_LEFT_WING_POSE: PartPose = PartPose {
    offset: [0.5, 1.0, 1.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const VEX_RIGHT_WING_POSE: PartPose = PartPose {
    offset: [-0.5, 1.0, 1.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla `VexModel.setupAnim` non-charging body tilt: `body.xRot = π/20`.
pub(in crate::entity_models) const VEX_BODY_X_ROT: f32 = std::f32::consts::PI / 20.0;
/// Vanilla `VexModel.setupAnim` arm rest roll: `rightArm.zRot = π/5 + bob`,
/// `leftArm.zRot = -(π/5 + bob)`.
pub(in crate::entity_models) const VEX_ARM_REST_Z_ROT: f32 = std::f32::consts::PI / 5.0;
/// Vanilla `VexModel.setupAnim` wing pitch/roll: both wings use
/// `xRot = zRot = 0.47123888` (the left wing's roll is negated).
pub(in crate::entity_models) const VEX_WING_X_ROT: f32 = 0.471_238_88;
pub(in crate::entity_models) const VEX_WING_Z_ROT: f32 = 0.471_238_88;

/// Vanilla `VexModel.setArmsCharging` (both hands empty) arm pitch: `xRot = -1.2217305` for
/// both arms while the vex is charging an attack.
pub(in crate::entity_models) const VEX_ARM_CHARGING_X_ROT: f32 = -1.221_730_5;
/// Vanilla `VexModel.setArmsCharging` held-item arm pitch: `xRot = π*7/6` for each hand whose
/// `ItemStackRenderState` is non-empty while charging.
pub(in crate::entity_models) const VEX_ARM_CHARGING_ITEM_X_ROT: f32 =
    std::f32::consts::PI * 7.0 / 6.0;
/// Charging arm yaw magnitude: `rightArm.yRot = π/12`, `leftArm.yRot = -π/12`.
pub(in crate::entity_models) const VEX_ARM_CHARGING_Y_ROT: f32 = std::f32::consts::PI / 12.0;
/// Charging arm roll base: `rightArm.zRot = -0.47123888 - bob`, `leftArm.zRot = 0.47123888 +
/// bob` (`bob` is the shared `vex_moving_arm_z_bob`).
pub(in crate::entity_models) const VEX_ARM_CHARGING_Z_ROT: f32 = 0.471_238_88;

/// Vanilla `VexModel.setupAnim` arm bob: `movingArmZBob = cos(ageInTicks · 5.5°) · 0.1`,
/// added to the right arm's `zRot` and subtracted from the left's.
pub(in crate::entity_models) fn vex_moving_arm_z_bob(age_in_ticks: f32) -> f32 {
    (age_in_ticks * 5.5_f32.to_radians()).cos() * 0.1
}

/// Vanilla `VexModel.setupAnim` wing flap: `leftWing.yRot = 1.0995574 +
/// cos(ageInTicks · 45.836624°) · 16.2°`; the right wing mirrors it (`-leftWing.yRot`).
pub(in crate::entity_models) fn vex_left_wing_y_rot(age_in_ticks: f32) -> f32 {
    1.099_557_4 + (age_in_ticks * 45.836_624_f32.to_radians()).cos() * 16.2_f32.to_radians()
}

fn vex_charging_arm_rotations(instance: &EntityModelInstance, bob: f32) -> ([f32; 3], [f32; 3]) {
    let right_has_item = instance.render_state.vex_right_hand_item_non_empty;
    let left_has_item = instance.render_state.vex_left_hand_item_non_empty;
    if !right_has_item && !left_has_item {
        return (
            [
                VEX_ARM_CHARGING_X_ROT,
                VEX_ARM_CHARGING_Y_ROT,
                -VEX_ARM_CHARGING_Z_ROT - bob,
            ],
            [
                VEX_ARM_CHARGING_X_ROT,
                -VEX_ARM_CHARGING_Y_ROT,
                VEX_ARM_CHARGING_Z_ROT + bob,
            ],
        );
    }

    let right_arm_rot = if right_has_item {
        [
            VEX_ARM_CHARGING_ITEM_X_ROT,
            VEX_ARM_CHARGING_Y_ROT,
            -VEX_ARM_CHARGING_Z_ROT - bob,
        ]
    } else {
        [0.0, 0.0, VEX_ARM_REST_Z_ROT + bob]
    };
    let left_arm_rot = if left_has_item {
        [
            VEX_ARM_CHARGING_ITEM_X_ROT,
            -VEX_ARM_CHARGING_Y_ROT,
            VEX_ARM_CHARGING_Z_ROT + bob,
        ]
    } else {
        [0.0, 0.0, -(VEX_ARM_REST_Z_ROT + bob)]
    };
    (right_arm_rot, left_arm_rot)
}

/// Applies the vanilla `VexModel.setupAnim` pose to the unified tree: the head look, the body charging
/// level / idle tilt, the arm charging raise / idle hold (both with the shared `vex_moving_arm_z_bob`),
/// and the wing flap. The charging arm branch follows vanilla `setArmsCharging`: both empty hands use
/// the two-arm lunge, while item-bearing hands pitch to `π*7/6` and empty hands keep the rest roll.
/// Every value is set absolutely, reproducing the hand-walked emit exactly.
fn apply_vex_anim(root: &mut ModelPart, instance: &EntityModelInstance) {
    let age = instance.render_state.age_in_ticks;
    let charging = instance.render_state.vex_charging;
    let head_pitch = instance.render_state.head_pitch.to_radians();
    let head_yaw = instance.render_state.head_yaw.to_radians();
    let bob = vex_moving_arm_z_bob(age);
    let left_wing_yrot = vex_left_wing_y_rot(age);
    let (right_arm_rot, left_arm_rot) = if charging {
        vex_charging_arm_rotations(instance, bob)
    } else {
        (
            [0.0, 0.0, VEX_ARM_REST_Z_ROT + bob],
            [0.0, 0.0, -(VEX_ARM_REST_Z_ROT + bob)],
        )
    };

    let vex_root = root.child_mut("root");
    vex_root.child_mut("head").pose.rotation = [head_pitch, head_yaw, 0.0];

    let body = vex_root.child_mut("body");
    body.pose.rotation = [if charging { 0.0 } else { VEX_BODY_X_ROT }, 0.0, 0.0];
    body.child_mut("right_arm").pose.rotation = right_arm_rot;
    body.child_mut("left_arm").pose.rotation = left_arm_rot;
    body.child_mut("left_wing").pose.rotation = [VEX_WING_X_ROT, left_wing_yrot, -VEX_WING_Z_ROT];
    body.child_mut("right_wing").pose.rotation = [VEX_WING_X_ROT, -left_wing_yrot, VEX_WING_Z_ROT];
}

/// Mutable vex model, mirroring vanilla `VexModel`. The unified tree is built once with named children:
/// the static `root` pivot → (`head`, `body`), with `body` parenting the right arm, left arm, left wing,
/// and right wing (the emit order, preserved for byte-identical meshes). Each cube carries both the
/// colored tint and the textured UV, so one tree drives both render paths; `setup_anim` runs
/// [`apply_vex_anim`] (head look, charging body/arms, wing flap). The same posed tree drives the
/// colored fallback and the single translucent textured layer (the full-bright block light is deferred
/// lighting).
pub(in crate::entity_models) struct VexModel {
    root: ModelPart,
}

impl VexModel {
    pub(in crate::entity_models) fn new() -> Self {
        let body = ModelPart::new(
            VEX_BODY_POSE,
            VEX_BODY.to_vec(),
            vec![
                (
                    "right_arm",
                    ModelPart::leaf(VEX_RIGHT_ARM_POSE, VEX_RIGHT_ARM.to_vec()),
                ),
                (
                    "left_arm",
                    ModelPart::leaf(VEX_LEFT_ARM_POSE, VEX_LEFT_ARM.to_vec()),
                ),
                (
                    "left_wing",
                    ModelPart::leaf(VEX_LEFT_WING_POSE, VEX_LEFT_WING.to_vec()),
                ),
                (
                    "right_wing",
                    ModelPart::leaf(VEX_RIGHT_WING_POSE, VEX_RIGHT_WING.to_vec()),
                ),
            ],
        );
        let vex_root = ModelPart::new(
            VEX_ROOT_POSE,
            Vec::new(),
            vec![
                ("head", ModelPart::leaf(VEX_HEAD_POSE, VEX_HEAD.to_vec())),
                ("body", body),
            ],
        );
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![("root", vex_root)]),
        }
    }
}

impl EntityModel for VexModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_vex_anim(&mut self.root, instance);
    }
}
