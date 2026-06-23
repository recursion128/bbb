use super::{ModelCubeDesc, PartPose, TexturedModelCubeDesc, VEX_GREY};

// Vanilla 26.1 `VexModel.createBodyLayer` (atlas 32×32). The model root is the `root` part
// at `(0, -2.5, 0)`; `head` and `body` hang under it, and the arms and wings are children
// of `body` (so the body tilt carries them). `CubeDeformation` insets are baked into the
// cube min/size below (`min -= grow`, `size += 2·grow`).
pub(in crate::entity_models) const VEX_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, -5.0, -2.5],
    size: [5.0, 5.0, 5.0],
    color: VEX_GREY,
}];

// Body: a plain `texOffs(0, 10)` 3×4×2 box plus the `texOffs(0, 16)` 3×5×2 box inset by
// `CubeDeformation(-0.2)` → min `+0.2`, size `-0.4` per axis.
pub(in crate::entity_models) const VEX_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.5, 0.0, -1.0],
        size: [3.0, 4.0, 2.0],
        color: VEX_GREY,
    },
    ModelCubeDesc {
        min: [-1.3, 1.2, -0.8],
        size: [2.6, 4.6, 1.6],
        color: VEX_GREY,
    },
];

// Arms: 2×4×2 boxes inset by `CubeDeformation(-0.1)` → min `+0.1`, size `-0.2` per axis.
pub(in crate::entity_models) const VEX_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.15, -0.4, -0.9],
    size: [1.8, 3.8, 1.8],
    color: VEX_GREY,
}];

pub(in crate::entity_models) const VEX_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.65, -0.4, -0.9],
    size: [1.8, 3.8, 1.8],
    color: VEX_GREY,
}];

// Wings: zero-thickness `0×5×8` planes (the left wing only differs by mirrored UV).
pub(in crate::entity_models) const VEX_WING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [0.0, 5.0, 8.0],
    color: VEX_GREY,
}];

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

// Textured counterparts of the vex cubes (atlas 32×32). `CubeDeformation` inflates the
// geometry (min/size) but the `uv_size` keeps the BASE box, exactly as vanilla bakes it.
pub(in crate::entity_models) const VEX_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.5, -5.0, -2.5],
        size: [5.0, 5.0, 5.0],
        uv_size: [5.0, 5.0, 5.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const VEX_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-1.5, 0.0, -1.0],
        size: [3.0, 4.0, 2.0],
        uv_size: [3.0, 4.0, 2.0],
        tex: [0.0, 10.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        // `CubeDeformation(-0.2)`: geometry inset, but uv_size keeps the 3×5×2 base box.
        min: [-1.3, 1.2, -0.8],
        size: [2.6, 4.6, 1.6],
        uv_size: [3.0, 5.0, 2.0],
        tex: [0.0, 16.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const VEX_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.15, -0.4, -0.9],
        size: [1.8, 3.8, 1.8],
        uv_size: [2.0, 4.0, 2.0],
        tex: [23.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const VEX_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-0.65, -0.4, -0.9],
        size: [1.8, 3.8, 1.8],
        uv_size: [2.0, 4.0, 2.0],
        tex: [23.0, 6.0],
        mirror: false,
    }];

// Both wings share `texOffs(16, 14)`; the left wing's UV is mirrored.
pub(in crate::entity_models) const VEX_TEXTURED_LEFT_WING: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [0.0, 5.0, 8.0],
        uv_size: [0.0, 5.0, 8.0],
        tex: [16.0, 14.0],
        mirror: true,
    }];

pub(in crate::entity_models) const VEX_TEXTURED_RIGHT_WING: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [0.0, 5.0, 8.0],
        uv_size: [0.0, 5.0, 8.0],
        tex: [16.0, 14.0],
        mirror: false,
    }];
