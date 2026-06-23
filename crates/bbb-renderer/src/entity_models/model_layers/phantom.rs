use super::{PartPose, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

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

// Vanilla 26.1 PhantomModel.createBodyLayer cubes (texture 64x64, no CubeDeformation). Each unified
// cube carries both render paths' data: the colored debug tint (`PHANTOM_TEAL`) and the textured
// `uv_size` / `texOffs` / `mirror`. `uv_size` equals the geometry size for every cube. Vanilla mirrors
// the right-wing texOffs onto the negative-x boxes.
pub(in crate::entity_models) const PHANTOM_BODY_CUBE: ModelCube = ModelCube::new(
    [-3.0, -2.0, -8.0],
    [5.0, 3.0, 9.0],
    PHANTOM_TEAL,
    [5.0, 3.0, 9.0],
    [0.0, 8.0],
    false,
);
pub(in crate::entity_models) const PHANTOM_TAIL_BASE_CUBE: ModelCube = ModelCube::new(
    [-2.0, 0.0, 0.0],
    [3.0, 2.0, 6.0],
    PHANTOM_TEAL,
    [3.0, 2.0, 6.0],
    [3.0, 20.0],
    false,
);
pub(in crate::entity_models) const PHANTOM_TAIL_TIP_CUBE: ModelCube = ModelCube::new(
    [-1.0, 0.0, 0.0],
    [1.0, 1.0, 6.0],
    PHANTOM_TEAL,
    [1.0, 1.0, 6.0],
    [4.0, 29.0],
    false,
);
pub(in crate::entity_models) const PHANTOM_LEFT_WING_BASE_CUBE: ModelCube = ModelCube::new(
    [0.0, 0.0, 0.0],
    [6.0, 2.0, 9.0],
    PHANTOM_TEAL,
    [6.0, 2.0, 9.0],
    [23.0, 12.0],
    false,
);
pub(in crate::entity_models) const PHANTOM_LEFT_WING_TIP_CUBE: ModelCube = ModelCube::new(
    [0.0, 0.0, 0.0],
    [13.0, 1.0, 9.0],
    PHANTOM_TEAL,
    [13.0, 1.0, 9.0],
    [16.0, 24.0],
    false,
);
pub(in crate::entity_models) const PHANTOM_RIGHT_WING_BASE_CUBE: ModelCube = ModelCube::new(
    [-6.0, 0.0, 0.0],
    [6.0, 2.0, 9.0],
    PHANTOM_TEAL,
    [6.0, 2.0, 9.0],
    [23.0, 12.0],
    true,
);
pub(in crate::entity_models) const PHANTOM_RIGHT_WING_TIP_CUBE: ModelCube = ModelCube::new(
    [-13.0, 0.0, 0.0],
    [13.0, 1.0, 9.0],
    PHANTOM_TEAL,
    [13.0, 1.0, 9.0],
    [16.0, 24.0],
    true,
);
pub(in crate::entity_models) const PHANTOM_HEAD_CUBE: ModelCube = ModelCube::new(
    [-4.0, -2.0, -5.0],
    [7.0, 3.0, 5.0],
    PHANTOM_TEAL,
    [7.0, 3.0, 5.0],
    [0.0, 0.0],
    false,
);

/// Mutable phantom model, mirroring vanilla `PhantomModel`. The unified tree is built once with named
/// children: the `body` parents the `tail_base` (→ `tail_tip`) chain, the `left_wing_base`
/// (→ `left_wing_tip`) and `right_wing_base` (→ `right_wing_tip`) chains, and the `head`. The same
/// posed tree drives the colored fallback, the textured cutout base layer, and the emissive eyes
/// overlay (both passes re-render the same tree). `setup_anim` flaps the wings and tail from `flapTime`
/// (`id*3 + ageInTicks`); the size scale and body pitch live in the root transform.
pub(in crate::entity_models) struct PhantomModel {
    root: ModelPart,
}

impl PhantomModel {
    pub(in crate::entity_models) fn new() -> Self {
        let tail_base = ModelPart::new(
            PHANTOM_TAIL_BASE_POSE,
            vec![PHANTOM_TAIL_BASE_CUBE],
            vec![(
                "tail_tip",
                ModelPart::leaf(PHANTOM_TAIL_TIP_POSE, vec![PHANTOM_TAIL_TIP_CUBE]),
            )],
        );
        let left_wing_base = ModelPart::new(
            PHANTOM_LEFT_WING_BASE_POSE,
            vec![PHANTOM_LEFT_WING_BASE_CUBE],
            vec![(
                "left_wing_tip",
                ModelPart::leaf(PHANTOM_LEFT_WING_TIP_POSE, vec![PHANTOM_LEFT_WING_TIP_CUBE]),
            )],
        );
        let right_wing_base = ModelPart::new(
            PHANTOM_RIGHT_WING_BASE_POSE,
            vec![PHANTOM_RIGHT_WING_BASE_CUBE],
            vec![(
                "right_wing_tip",
                ModelPart::leaf(
                    PHANTOM_RIGHT_WING_TIP_POSE,
                    vec![PHANTOM_RIGHT_WING_TIP_CUBE],
                ),
            )],
        );
        let body = ModelPart::new(
            PHANTOM_BODY_POSE,
            vec![PHANTOM_BODY_CUBE],
            vec![
                ("tail_base", tail_base),
                ("left_wing_base", left_wing_base),
                ("right_wing_base", right_wing_base),
                (
                    "head",
                    ModelPart::leaf(PHANTOM_HEAD_POSE, vec![PHANTOM_HEAD_CUBE]),
                ),
            ],
        );
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![("body", body)]),
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
        // Vanilla `PhantomModel.setupAnim` flap: each wing's base and tip `zRot` is set to
        // ±[`phantom_wing_z_rot`] (left positive, right negated) and each tail segment's `xRot` to
        // [`phantom_tail_x_rot`], overwriting the rest dihedral every frame. The head holds its rest
        // tilt. The flap always advances (`flapTime` tracks `ageInTicks`), so this runs unconditionally.
        let flap = phantom_flap_time(instance.entity_id, instance.render_state.age_in_ticks);
        let wing_z = phantom_wing_z_rot(flap);
        let tail_x = phantom_tail_x_rot(flap);
        let body = self.root.child_mut("body");

        let tail_base = body.child_mut("tail_base");
        tail_base.pose = phantom_tail_pose(PHANTOM_TAIL_BASE_POSE, tail_x);
        tail_base.child_mut("tail_tip").pose = phantom_tail_pose(PHANTOM_TAIL_TIP_POSE, tail_x);

        let left_base = body.child_mut("left_wing_base");
        left_base.pose = phantom_wing_pose(PHANTOM_LEFT_WING_BASE_POSE, wing_z);
        left_base.child_mut("left_wing_tip").pose =
            phantom_wing_pose(PHANTOM_LEFT_WING_TIP_POSE, wing_z);

        let right_base = body.child_mut("right_wing_base");
        right_base.pose = phantom_wing_pose(PHANTOM_RIGHT_WING_BASE_POSE, -wing_z);
        right_base.child_mut("right_wing_tip").pose =
            phantom_wing_pose(PHANTOM_RIGHT_WING_TIP_POSE, -wing_z);
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
