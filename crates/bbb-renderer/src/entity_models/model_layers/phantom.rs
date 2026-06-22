use super::{ModelCubeDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc};

// The phantom fallback paints its body a dark End-blue teal.
pub(in crate::entity_models) const PHANTOM_TEAL: [f32; 4] = [0.28, 0.42, 0.46, 1.0];

pub(in crate::entity_models) const MODEL_LAYER_PHANTOM: &str = "minecraft:phantom#main";

// Vanilla 26.1 PhantomModel.createBodyLayer rest poses. The body parents the tail chain, the
// two wing chains, and the head; the wings rest with a small +-0.1 zRot dihedral that
// setupAnim overwrites every frame.
pub(in crate::entity_models) const PHANTOM_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [-0.1, 0.0, 0.0],
};
pub(in crate::entity_models) const PHANTOM_TAIL_BASE_POSE: PartPose = PartPose {
    offset: [0.0, -2.0, 1.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const PHANTOM_TAIL_TIP_POSE: PartPose = PartPose {
    offset: [0.0, 0.5, 6.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const PHANTOM_LEFT_WING_BASE_POSE: PartPose = PartPose {
    offset: [2.0, -2.0, -8.0],
    rotation: [0.0, 0.0, 0.1],
};
pub(in crate::entity_models) const PHANTOM_LEFT_WING_TIP_POSE: PartPose = PartPose {
    offset: [6.0, 0.0, 0.0],
    rotation: [0.0, 0.0, 0.1],
};
pub(in crate::entity_models) const PHANTOM_RIGHT_WING_BASE_POSE: PartPose = PartPose {
    offset: [-3.0, -2.0, -8.0],
    rotation: [0.0, 0.0, -0.1],
};
pub(in crate::entity_models) const PHANTOM_RIGHT_WING_TIP_POSE: PartPose = PartPose {
    offset: [-6.0, 0.0, 0.0],
    rotation: [0.0, 0.0, -0.1],
};
pub(in crate::entity_models) const PHANTOM_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 1.0, -7.0],
    rotation: [0.2, 0.0, 0.0],
};

pub(in crate::entity_models) const PHANTOM_BODY_CUBE: ModelCubeDesc = ModelCubeDesc {
    min: [-3.0, -2.0, -8.0],
    size: [5.0, 3.0, 9.0],
    color: PHANTOM_TEAL,
};
pub(in crate::entity_models) const PHANTOM_TAIL_BASE_CUBE: ModelCubeDesc = ModelCubeDesc {
    min: [-2.0, 0.0, 0.0],
    size: [3.0, 2.0, 6.0],
    color: PHANTOM_TEAL,
};
pub(in crate::entity_models) const PHANTOM_TAIL_TIP_CUBE: ModelCubeDesc = ModelCubeDesc {
    min: [-1.0, 0.0, 0.0],
    size: [1.0, 1.0, 6.0],
    color: PHANTOM_TEAL,
};
pub(in crate::entity_models) const PHANTOM_LEFT_WING_BASE_CUBE: ModelCubeDesc = ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [6.0, 2.0, 9.0],
    color: PHANTOM_TEAL,
};
pub(in crate::entity_models) const PHANTOM_LEFT_WING_TIP_CUBE: ModelCubeDesc = ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [13.0, 1.0, 9.0],
    color: PHANTOM_TEAL,
};
pub(in crate::entity_models) const PHANTOM_RIGHT_WING_BASE_CUBE: ModelCubeDesc = ModelCubeDesc {
    min: [-6.0, 0.0, 0.0],
    size: [6.0, 2.0, 9.0],
    color: PHANTOM_TEAL,
};
pub(in crate::entity_models) const PHANTOM_RIGHT_WING_TIP_CUBE: ModelCubeDesc = ModelCubeDesc {
    min: [-13.0, 0.0, 0.0],
    size: [13.0, 1.0, 9.0],
    color: PHANTOM_TEAL,
};
pub(in crate::entity_models) const PHANTOM_HEAD_CUBE: ModelCubeDesc = ModelCubeDesc {
    min: [-4.0, -2.0, -5.0],
    size: [7.0, 3.0, 5.0],
    color: PHANTOM_TEAL,
};

const fn phantom_textured_cube(
    min: [f32; 3],
    size: [f32; 3],
    tex: [f32; 2],
    mirror: bool,
) -> TexturedModelCubeDesc {
    TexturedModelCubeDesc {
        min,
        size,
        uv_size: size,
        tex,
        mirror,
    }
}

pub(in crate::entity_models) const PHANTOM_BODY_TEXTURED_CUBE: TexturedModelCubeDesc =
    phantom_textured_cube([-3.0, -2.0, -8.0], [5.0, 3.0, 9.0], [0.0, 8.0], false);
pub(in crate::entity_models) const PHANTOM_TAIL_BASE_TEXTURED_CUBE: TexturedModelCubeDesc =
    phantom_textured_cube([-2.0, 0.0, 0.0], [3.0, 2.0, 6.0], [3.0, 20.0], false);
pub(in crate::entity_models) const PHANTOM_TAIL_TIP_TEXTURED_CUBE: TexturedModelCubeDesc =
    phantom_textured_cube([-1.0, 0.0, 0.0], [1.0, 1.0, 6.0], [4.0, 29.0], false);
pub(in crate::entity_models) const PHANTOM_LEFT_WING_BASE_TEXTURED_CUBE: TexturedModelCubeDesc =
    phantom_textured_cube([0.0, 0.0, 0.0], [6.0, 2.0, 9.0], [23.0, 12.0], false);
pub(in crate::entity_models) const PHANTOM_LEFT_WING_TIP_TEXTURED_CUBE: TexturedModelCubeDesc =
    phantom_textured_cube([0.0, 0.0, 0.0], [13.0, 1.0, 9.0], [16.0, 24.0], false);
// Vanilla mirrors the right-wing texOffs onto the negative-x boxes.
pub(in crate::entity_models) const PHANTOM_RIGHT_WING_BASE_TEXTURED_CUBE: TexturedModelCubeDesc =
    phantom_textured_cube([-6.0, 0.0, 0.0], [6.0, 2.0, 9.0], [23.0, 12.0], true);
pub(in crate::entity_models) const PHANTOM_RIGHT_WING_TIP_TEXTURED_CUBE: TexturedModelCubeDesc =
    phantom_textured_cube([-13.0, 0.0, 0.0], [13.0, 1.0, 9.0], [16.0, 24.0], true);
pub(in crate::entity_models) const PHANTOM_HEAD_TEXTURED_CUBE: TexturedModelCubeDesc =
    phantom_textured_cube([-4.0, -2.0, -5.0], [7.0, 3.0, 5.0], [0.0, 0.0], false);

const PHANTOM_TAIL_TIP_TEXTURED: [TexturedModelPartDesc; 1] = [TexturedModelPartDesc {
    pose: PHANTOM_TAIL_TIP_POSE,
    cubes: &[PHANTOM_TAIL_TIP_TEXTURED_CUBE],
    children: &[],
}];
const PHANTOM_LEFT_WING_TIP_TEXTURED: [TexturedModelPartDesc; 1] = [TexturedModelPartDesc {
    pose: PHANTOM_LEFT_WING_TIP_POSE,
    cubes: &[PHANTOM_LEFT_WING_TIP_TEXTURED_CUBE],
    children: &[],
}];
const PHANTOM_RIGHT_WING_TIP_TEXTURED: [TexturedModelPartDesc; 1] = [TexturedModelPartDesc {
    pose: PHANTOM_RIGHT_WING_TIP_POSE,
    cubes: &[PHANTOM_RIGHT_WING_TIP_TEXTURED_CUBE],
    children: &[],
}];
const PHANTOM_BODY_TEXTURED_CHILDREN: [TexturedModelPartDesc; 4] = [
    TexturedModelPartDesc {
        pose: PHANTOM_TAIL_BASE_POSE,
        cubes: &[PHANTOM_TAIL_BASE_TEXTURED_CUBE],
        children: &PHANTOM_TAIL_TIP_TEXTURED,
    },
    TexturedModelPartDesc {
        pose: PHANTOM_LEFT_WING_BASE_POSE,
        cubes: &[PHANTOM_LEFT_WING_BASE_TEXTURED_CUBE],
        children: &PHANTOM_LEFT_WING_TIP_TEXTURED,
    },
    TexturedModelPartDesc {
        pose: PHANTOM_RIGHT_WING_BASE_POSE,
        cubes: &[PHANTOM_RIGHT_WING_BASE_TEXTURED_CUBE],
        children: &PHANTOM_RIGHT_WING_TIP_TEXTURED,
    },
    TexturedModelPartDesc {
        pose: PHANTOM_HEAD_POSE,
        cubes: &[PHANTOM_HEAD_TEXTURED_CUBE],
        children: &[],
    },
];

/// The phantom body layer as a nested tree (body parents the tail/wing chains and head),
/// used for the layer-pass definition and tests. Emission re-poses the animated descendants
/// by hand, so this static tree carries the rest pose.
pub(in crate::entity_models) const PHANTOM_TEXTURED_PARTS: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PHANTOM_BODY_POSE,
        cubes: &[PHANTOM_BODY_TEXTURED_CUBE],
        children: &PHANTOM_BODY_TEXTURED_CHILDREN,
    }];

/// Vanilla `PhantomRenderer.extractRenderState`: `flapTime = getUniqueFlapTickOffset() +
/// ageInTicks`, where `Phantom.getUniqueFlapTickOffset() = getId() * 3` — a deterministic
/// per-entity phase offset (Java `int` multiply, so it wraps like vanilla) plus the projected
/// `ageInTicks`.
pub(in crate::entity_models) fn phantom_flap_time(entity_id: i32, age_in_ticks: f32) -> f32 {
    entity_id.wrapping_mul(3) as f32 + age_in_ticks
}

/// Vanilla `PhantomModel.setupAnim` flap phase: `anim = flapTime * FLAP_DEGREES_PER_TICK *
/// π/180`, where `FLAP_DEGREES_PER_TICK = 7.448451`. Returned in radians.
fn phantom_flap_anim(flap_time: f32) -> f32 {
    (flap_time * 7.448451).to_radians()
}

/// Vanilla `PhantomModel.setupAnim` left-wing `zRot = cos(anim) * 16°`. The right wing uses
/// the negation. Both the base and tip wing parts share this value (set absolutely, so the
/// rest `±0.1` dihedral is overwritten every frame).
pub(in crate::entity_models) fn phantom_wing_z_rot(flap_time: f32) -> f32 {
    phantom_flap_anim(flap_time).cos() * 16.0_f32.to_radians()
}

/// Vanilla `PhantomModel.setupAnim` tail `xRot = -(5° + cos(2·anim) * 5°)`. Both the base and
/// tip tail parts share this value (set absolutely over the zeroed rest tail pose).
pub(in crate::entity_models) fn phantom_tail_x_rot(flap_time: f32) -> f32 {
    let anim = phantom_flap_anim(flap_time);
    -(5.0 + (anim * 2.0).cos() * 5.0).to_radians()
}

/// Applies the flap `zRot` to a wing part pose, overwriting the rest dihedral while
/// preserving the offset and the zeroed `xRot`/`yRot`.
pub(in crate::entity_models) fn phantom_wing_pose(base: PartPose, z_rot: f32) -> PartPose {
    PartPose {
        offset: base.offset,
        rotation: [base.rotation[0], base.rotation[1], z_rot],
    }
}

/// Applies the flap `xRot` to a tail part pose, preserving the offset and the zeroed
/// `yRot`/`zRot`.
pub(in crate::entity_models) fn phantom_tail_pose(base: PartPose, x_rot: f32) -> PartPose {
    PartPose {
        offset: base.offset,
        rotation: [x_rot, base.rotation[1], base.rotation[2]],
    }
}
