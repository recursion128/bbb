use super::{
    apply_head_look, model_cube as cube, ModelCubeDesc, PartPose, NAUTILUS_BODY, NAUTILUS_SHELL,
    PART_POSE_ZERO,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

// Vanilla 26.1 `NautilusModel.createBodyLayer` (atlas 128×128) — the new rideable nautilus mob.
// `NautilusModel extends EntityModel`: one cubeless `root` pivot parenting the `shell` (the spiral shell,
// three boxes) and the `body` (the trunk plus its 0-thickness rear fin) which in turn parents the three
// mouth boxes. `NautilusModel.setupAnim` steers the `body` by the look — `body.yRot/xRot` set from the
// look yaw/pitch CLAMPED to ±10° — then layers the looping `NautilusAnimation.SWIMMING` keyframe via
// `applyWalk` (always on, the idle baseline `walkAnimationSpeed + 0.2`). The clamped body look is
// reproduced; the SWIMMING keyframe undulation needs the keyframe machinery + an `AnimationState`, so it
// stays deferred (the nautilus renders at this bind pose plus the clamped look). Both the adult
// `createBodyMesh` and the smaller baby `createBabyBodyLayer` are modeled (same `root → shell + body →
// mouths` structure). The variant textures and the saddle / armor / coral layers are deferred, so the
// colored debug path renders a tan shell over a pale body. Colored-only (no textured path yet), so the
// cubes stay [`ModelCubeDesc`] and the tree is assembled from `leaf_colored` (and `colored` for the
// cube-bearing `body`). Nautilus uses a plain `MobRenderer`.

// `shell` cubes: the 14×10×16 dome, the 14×8×20 whorl, and a 14×8×0 rear fin plane.
pub(in crate::entity_models) const NAUTILUS_SHELL_CUBES: [ModelCubeDesc; 3] = [
    cube([-7.0, -10.0, -7.0], [14.0, 10.0, 16.0], NAUTILUS_SHELL),
    cube([-7.0, 0.0, -7.0], [14.0, 8.0, 20.0], NAUTILUS_SHELL),
    cube([-7.0, 0.0, 6.0], [14.0, 8.0, 0.0], NAUTILUS_SHELL),
];

// `body` cubes: the 10×8×14 trunk and a 10×8×0 rear fin plane.
pub(in crate::entity_models) const NAUTILUS_BODY_CUBES: [ModelCubeDesc; 2] = [
    cube([-5.0, -4.51, -3.0], [10.0, 8.0, 14.0], NAUTILUS_BODY),
    cube([-5.0, -4.51, 7.0], [10.0, 8.0, 0.0], NAUTILUS_BODY),
];

// The three mouth boxes. Upper/lower use the vanilla `CubeDeformation(-0.001)` (min += 0.001,
// size -= 0.002); the inner mouth is undeformed.
pub(in crate::entity_models) const NAUTILUS_UPPER_MOUTH_CUBES: [ModelCubeDesc; 1] = [cube(
    [-4.999, -1.999, 0.001],
    [9.998, 3.998, 3.998],
    NAUTILUS_BODY,
)];
pub(in crate::entity_models) const NAUTILUS_INNER_MOUTH_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.0, -2.0, -0.5], [6.0, 4.0, 4.0], NAUTILUS_BODY)];
pub(in crate::entity_models) const NAUTILUS_LOWER_MOUTH_CUBES: [ModelCubeDesc; 1] = [cube(
    [-4.999, -1.979, 0.001],
    [9.998, 3.998, 3.998],
    NAUTILUS_BODY,
)];

/// `root` pivot pose: `PartPose.offset(0, 29, -6)`.
pub(in crate::entity_models) const NAUTILUS_ROOT_POSE: PartPose = PartPose {
    offset: [0.0, 29.0, -6.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `shell` part pose: `PartPose.offset(0, -13, 5)`.
pub(in crate::entity_models) const NAUTILUS_SHELL_POSE: PartPose = PartPose {
    offset: [0.0, -13.0, 5.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `body` part pose: `PartPose.offset(0, -8.5, 12.3)`.
pub(in crate::entity_models) const NAUTILUS_BODY_POSE: PartPose = PartPose {
    offset: [0.0, -8.5, 12.3],
    rotation: [0.0, 0.0, 0.0],
};
/// `upper_mouth` part pose: `PartPose.offset(0, -2.51, 7)`.
pub(in crate::entity_models) const NAUTILUS_UPPER_MOUTH_POSE: PartPose = PartPose {
    offset: [0.0, -2.51, 7.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `inner_mouth` part pose: `PartPose.offset(0, -0.51, 7.5)`.
pub(in crate::entity_models) const NAUTILUS_INNER_MOUTH_POSE: PartPose = PartPose {
    offset: [0.0, -0.51, 7.5],
    rotation: [0.0, 0.0, 0.0],
};
/// `lower_mouth` part pose: `PartPose.offset(0, 1.49, 7)`.
pub(in crate::entity_models) const NAUTILUS_LOWER_MOUTH_POSE: PartPose = PartPose {
    offset: [0.0, 1.49, 7.0],
    rotation: [0.0, 0.0, 0.0],
};

// Vanilla 26.1 `NautilusModel.createBabyBodyLayer` (atlas 64×64): the same `root → shell + body → three
// mouths` hierarchy as the adult, scaled down to the hatchling proportions.

// Baby `shell` cubes: the 7×4×7 dome, the 7×4×9 whorl, and a 7×4×0 rear fin plane.
pub(in crate::entity_models) const BABY_NAUTILUS_SHELL_CUBES: [ModelCubeDesc; 3] = [
    cube([-6.0, -4.0, -1.0], [7.0, 4.0, 7.0], NAUTILUS_SHELL),
    cube([-6.0, 0.0, -1.0], [7.0, 4.0, 9.0], NAUTILUS_SHELL),
    cube([-6.0, 0.0, 5.0], [7.0, 4.0, 0.0], NAUTILUS_SHELL),
];

// Baby `body` cubes: the 5×4×7 trunk and a 5×4×0 rear fin plane.
pub(in crate::entity_models) const BABY_NAUTILUS_BODY_CUBES: [ModelCubeDesc; 2] = [
    cube([-2.5, -3.01, -1.0], [5.0, 4.0, 7.0], NAUTILUS_BODY),
    cube([-2.5, -3.01, 4.1], [5.0, 4.0, 0.0], NAUTILUS_BODY),
];

// The three baby mouth boxes. Upper/lower use the vanilla `CubeDeformation(-0.001)` (min += 0.001,
// size -= 0.002, on the same 5×2×2 box); the inner mouth is an undeformed 3×2×2.
pub(in crate::entity_models) const BABY_NAUTILUS_UPPER_MOUTH_CUBES: [ModelCubeDesc; 1] = [cube(
    [-2.499, -0.999, 0.001],
    [4.998, 1.998, 1.998],
    NAUTILUS_BODY,
)];
pub(in crate::entity_models) const BABY_NAUTILUS_INNER_MOUTH_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.5, -1.0, -1.0], [3.0, 2.0, 2.0], NAUTILUS_BODY)];
pub(in crate::entity_models) const BABY_NAUTILUS_LOWER_MOUTH_CUBES: [ModelCubeDesc; 1] = [cube(
    [-2.499, -0.999, 0.001],
    [4.998, 1.998, 1.998],
    NAUTILUS_BODY,
)];

/// Baby `root` pivot pose: `PartPose.offset(-0.5, 28, -0.5)`.
pub(in crate::entity_models) const BABY_NAUTILUS_ROOT_POSE: PartPose = PartPose {
    offset: [-0.5, 28.0, -0.5],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `shell` part pose: `PartPose.offset(3, -8, -2)`.
pub(in crate::entity_models) const BABY_NAUTILUS_SHELL_POSE: PartPose = PartPose {
    offset: [3.0, -8.0, -2.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `body` part pose: `PartPose.offset(0.5, -5, 3)`.
pub(in crate::entity_models) const BABY_NAUTILUS_BODY_POSE: PartPose = PartPose {
    offset: [0.5, -5.0, 3.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `upper_mouth` part pose: `PartPose.offset(0, -2.01, 3.9)`.
pub(in crate::entity_models) const BABY_NAUTILUS_UPPER_MOUTH_POSE: PartPose = PartPose {
    offset: [0.0, -2.01, 3.9],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `inner_mouth` part pose: `PartPose.offset(0, -1.01, 4.9)`.
pub(in crate::entity_models) const BABY_NAUTILUS_INNER_MOUTH_POSE: PartPose = PartPose {
    offset: [0.0, -1.01, 4.9],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `lower_mouth` part pose: `PartPose.offset(0, -0.01, 3.9)`.
pub(in crate::entity_models) const BABY_NAUTILUS_LOWER_MOUTH_POSE: PartPose = PartPose {
    offset: [0.0, -0.01, 3.9],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds the cubeless `root` pivot parenting the named `shell` and `body` for the adult or baby
/// geometry. The cube-bearing `body` carries its three mouth boxes as index-named children (only the
/// `body` itself is steered by name in [`NautilusModel::setup_anim`]; the mouths ride along). Both the
/// adult and baby share the identical `root → shell + body → mouths` structure.
fn nautilus_root(baby: bool) -> ModelPart {
    if baby {
        let body = ModelPart::colored(
            BABY_NAUTILUS_BODY_POSE,
            &BABY_NAUTILUS_BODY_CUBES,
            vec![
                ModelPart::leaf_colored(
                    BABY_NAUTILUS_UPPER_MOUTH_POSE,
                    &BABY_NAUTILUS_UPPER_MOUTH_CUBES,
                ),
                ModelPart::leaf_colored(
                    BABY_NAUTILUS_INNER_MOUTH_POSE,
                    &BABY_NAUTILUS_INNER_MOUTH_CUBES,
                ),
                ModelPart::leaf_colored(
                    BABY_NAUTILUS_LOWER_MOUTH_POSE,
                    &BABY_NAUTILUS_LOWER_MOUTH_CUBES,
                ),
            ],
        );
        ModelPart::new(
            BABY_NAUTILUS_ROOT_POSE,
            Vec::new(),
            vec![
                (
                    "shell",
                    ModelPart::leaf_colored(BABY_NAUTILUS_SHELL_POSE, &BABY_NAUTILUS_SHELL_CUBES),
                ),
                ("body", body),
            ],
        )
    } else {
        let body = ModelPart::colored(
            NAUTILUS_BODY_POSE,
            &NAUTILUS_BODY_CUBES,
            vec![
                ModelPart::leaf_colored(NAUTILUS_UPPER_MOUTH_POSE, &NAUTILUS_UPPER_MOUTH_CUBES),
                ModelPart::leaf_colored(NAUTILUS_INNER_MOUTH_POSE, &NAUTILUS_INNER_MOUTH_CUBES),
                ModelPart::leaf_colored(NAUTILUS_LOWER_MOUTH_POSE, &NAUTILUS_LOWER_MOUTH_CUBES),
            ],
        );
        ModelPart::new(
            NAUTILUS_ROOT_POSE,
            Vec::new(),
            vec![
                (
                    "shell",
                    ModelPart::leaf_colored(NAUTILUS_SHELL_POSE, &NAUTILUS_SHELL_CUBES),
                ),
                ("body", body),
            ],
        )
    }
}

/// Vanilla `NautilusModel.applyBodyRotation` clamps the look yaw/pitch to ±10° before steering the body.
const NAUTILUS_LOOK_CLAMP_DEGREES: f32 = 10.0;

/// Mutable nautilus model, mirroring vanilla `NautilusModel` (`baby` selecting the adult
/// `createBodyMesh` or the smaller `createBabyBodyLayer` geometry). The named hierarchy
/// (`root → shell + body → upper_mouth/inner_mouth/lower_mouth`) hangs off a synthetic root, built
/// from the baked colored geometry. Colored-only: `setup_anim` steers the body by the clamped look
/// ([`apply_head_look`] on the `body` part, with the yaw/pitch pre-clamped to ±10°) via `child_mut`;
/// the SWIMMING keyframe undulation and every other pose stay deferred.
pub(in crate::entity_models) struct NautilusModel {
    root: ModelPart,
}

impl NautilusModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![("root", nautilus_root(baby))],
            ),
        }
    }
}

impl EntityModel for NautilusModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `NautilusModel.applyBodyRotation`: the body (root → body) turns to the look, clamped
        // to ±10° on each axis. `apply_head_look` sets `xRot`/`yRot` from the (pre-clamped) degrees.
        let render_state = &instance.render_state;
        let body = self.root.child_mut("root").child_mut("body");
        apply_head_look(
            body,
            render_state
                .head_yaw
                .clamp(-NAUTILUS_LOOK_CLAMP_DEGREES, NAUTILUS_LOOK_CLAMP_DEGREES),
            render_state
                .head_pitch
                .clamp(-NAUTILUS_LOOK_CLAMP_DEGREES, NAUTILUS_LOOK_CLAMP_DEGREES),
        );
    }
}
