use super::{ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

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

// Colored counterpart of `PHANTOM_TEXTURED_PARTS`: the same body → (tail/wing chains, head)
// hierarchy, carrying the baked teal cubes. Zipped with the textured tree by `PhantomModel::new`.
const PHANTOM_TAIL_TIP_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PHANTOM_TAIL_TIP_POSE,
    cubes: &[PHANTOM_TAIL_TIP_CUBE],
    children: &[],
}];
const PHANTOM_LEFT_WING_TIP_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PHANTOM_LEFT_WING_TIP_POSE,
    cubes: &[PHANTOM_LEFT_WING_TIP_CUBE],
    children: &[],
}];
const PHANTOM_RIGHT_WING_TIP_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PHANTOM_RIGHT_WING_TIP_POSE,
    cubes: &[PHANTOM_RIGHT_WING_TIP_CUBE],
    children: &[],
}];
const PHANTOM_BODY_CHILDREN: [ModelPartDesc; 4] = [
    ModelPartDesc {
        pose: PHANTOM_TAIL_BASE_POSE,
        cubes: &[PHANTOM_TAIL_BASE_CUBE],
        children: &PHANTOM_TAIL_TIP_CHILDREN,
    },
    ModelPartDesc {
        pose: PHANTOM_LEFT_WING_BASE_POSE,
        cubes: &[PHANTOM_LEFT_WING_BASE_CUBE],
        children: &PHANTOM_LEFT_WING_TIP_CHILDREN,
    },
    ModelPartDesc {
        pose: PHANTOM_RIGHT_WING_BASE_POSE,
        cubes: &[PHANTOM_RIGHT_WING_BASE_CUBE],
        children: &PHANTOM_RIGHT_WING_TIP_CHILDREN,
    },
    ModelPartDesc {
        pose: PHANTOM_HEAD_POSE,
        cubes: &[PHANTOM_HEAD_CUBE],
        children: &[],
    },
];
pub(in crate::entity_models) const PHANTOM_PARTS: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PHANTOM_BODY_POSE,
    cubes: &[PHANTOM_BODY_CUBE],
    children: &PHANTOM_BODY_CHILDREN,
}];

/// The phantom body's children, in `PHANTOM_BODY_CHILDREN` order: the tail chain, the two wing
/// chains, then the head. The wings and tail are the animated chains ([`apply_phantom_flap`]).
const PHANTOM_TAIL_CHILD_INDEX: usize = 0;
const PHANTOM_LEFT_WING_CHILD_INDEX: usize = 1;
const PHANTOM_RIGHT_WING_CHILD_INDEX: usize = 2;

/// Applies the vanilla `PhantomModel.setupAnim` flap to the unified tree: each wing's base and tip
/// `zRot` is set to ±[`phantom_wing_z_rot`] (left positive, right negated) and each tail segment's
/// `xRot` to [`phantom_tail_x_rot`], overwriting the rest dihedral every frame. The head holds its
/// rest tilt. The flap always advances (`flapTime` tracks `ageInTicks`), so this runs unconditionally.
fn apply_phantom_flap(root: &mut ModelPart, flap_time: f32) {
    let wing_z = phantom_wing_z_rot(flap_time);
    let tail_x = phantom_tail_x_rot(flap_time);
    let body = root.child_at_mut(0);

    let tail_base = body.child_at_mut(PHANTOM_TAIL_CHILD_INDEX);
    tail_base.pose = phantom_tail_pose(PHANTOM_TAIL_BASE_POSE, tail_x);
    tail_base.child_at_mut(0).pose = phantom_tail_pose(PHANTOM_TAIL_TIP_POSE, tail_x);

    let left_base = body.child_at_mut(PHANTOM_LEFT_WING_CHILD_INDEX);
    left_base.pose = phantom_wing_pose(PHANTOM_LEFT_WING_BASE_POSE, wing_z);
    left_base.child_at_mut(0).pose = phantom_wing_pose(PHANTOM_LEFT_WING_TIP_POSE, wing_z);

    let right_base = body.child_at_mut(PHANTOM_RIGHT_WING_CHILD_INDEX);
    right_base.pose = phantom_wing_pose(PHANTOM_RIGHT_WING_BASE_POSE, -wing_z);
    right_base.child_at_mut(0).pose = phantom_wing_pose(PHANTOM_RIGHT_WING_TIP_POSE, -wing_z);
}

/// Mutable phantom model, mirroring vanilla `PhantomModel`. The unified tree is zipped from the body →
/// (tail chain, two wing chains, head) hierarchy ([`PHANTOM_PARTS`] / [`PHANTOM_TEXTURED_PARTS`]);
/// `setup_anim` runs [`apply_phantom_flap`] from `flapTime` (`id*3 + ageInTicks`). The same posed tree
/// drives the colored fallback, the textured cutout base layer, and the emissive eyes overlay (both
/// passes re-render the same tree). The size scale and body pitch live in the root transform.
pub(in crate::entity_models) struct PhantomModel {
    root: ModelPart,
}

impl PhantomModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::root_from_descs(&PHANTOM_PARTS, &PHANTOM_TEXTURED_PARTS),
        }
    }
}

impl EntityModel for PhantomModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let flap = phantom_flap_time(instance.entity_id, instance.render_state.age_in_ticks);
        apply_phantom_flap(&mut self.root, flap);
    }
}

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
