use super::{PartPose, PART_POSE_ZERO, TADPOLE_BODY, TADPOLE_TAIL};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 `TadpoleModel.createBodyLayer` (atlas 16×16). The mesh root holds two named sibling
// parts: a 3×2×3 `body` box at `offset(0, 22, -3)` and a 0×2×7 `tail` fin plane at `offset(0, 22, 0)`.
// The only `TadpoleModel.setupAnim` motion is the tail yaw sway ([`tadpole_tail_yrot`]), reproduced
// from the projected `age_in_ticks` + `in_water`. Colored-only (no textured path yet), so the cubes
// stay [`ModelCubeDesc`] and the tree is assembled from `leaf_colored`. Tadpole uses a plain
// `MobRenderer` with no transform overrides.

// `body`: the 3×2×3 box at texOffs(0, 0). Each unified cube carries the colored tint and the textured
// `uv_size` / `texOffs`.
pub(in crate::entity_models) const TADPOLE_BODY_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.5, -1.0, 0.0],
    [3.0, 2.0, 3.0],
    TADPOLE_BODY,
    [3.0, 2.0, 3.0],
    [0.0, 0.0],
    false,
)];

// `tail`: the 0×2×7 fin plane, also at texOffs(0, 0).
pub(in crate::entity_models) const TADPOLE_TAIL_CUBES: [ModelCube; 1] = [ModelCube::new(
    [0.0, -1.0, 0.0],
    [0.0, 2.0, 7.0],
    TADPOLE_TAIL,
    [0.0, 2.0, 7.0],
    [0.0, 0.0],
    false,
)];

/// `body` part pose: `PartPose.offset(0, 22, -3)`.
pub(in crate::entity_models) const TADPOLE_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 22.0, -3.0],
    rotation: [0.0, 0.0, 0.0],
};

/// `tail` part pose: `PartPose.offset(0, 22, 0)`.
pub(in crate::entity_models) const TADPOLE_TAIL_POSE: PartPose = PartPose {
    offset: [0.0, 22.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla `TadpoleModel.setupAnim`: `tail.yRot = -amplitude * 0.25 * sin(0.3 * ageInTicks)`, with
/// `amplitude = isInWater ? 1.0 : 1.5` (a beached tadpole thrashes harder). The rest pose has
/// `yRot = 0`, so this is set absolutely. Mirrors [`super::cod_tail_fin_yrot`] with the tadpole's
/// own `0.25` / `0.3` constants.
pub(in crate::entity_models) fn tadpole_tail_yrot(age_in_ticks: f32, in_water: bool) -> f32 {
    let amplitude = if in_water { 1.0 } else { 1.5 };
    -amplitude * 0.25 * (0.3 * age_in_ticks).sin()
}

/// Mutable tadpole model, mirroring vanilla `TadpoleModel`. Its two named sibling parts (`body`,
/// `tail`) hang off a synthetic root, each built from the baked colored geometry. Colored-only (no
/// textured path yet): `setup_anim` sways only the tail fin's yaw via `child_mut("tail")`.
pub(in crate::entity_models) struct TadpoleModel {
    root: ModelPart,
}

impl TadpoleModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![
                    (
                        "body",
                        ModelPart::leaf(TADPOLE_BODY_POSE, TADPOLE_BODY_CUBES.to_vec()),
                    ),
                    (
                        "tail",
                        ModelPart::leaf(TADPOLE_TAIL_POSE, TADPOLE_TAIL_CUBES.to_vec()),
                    ),
                ],
            ),
        }
    }
}

impl EntityModel for TadpoleModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `TadpoleModel.setupAnim`: the tail fin's yaw sways ([`tadpole_tail_yrot`]); the
        // body box holds still. The sway is an absolute set that collapses to the bind `yRot = 0` at
        // `ageInTicks = 0`, so it applies every frame.
        self.root.child_mut("tail").pose.rotation[1] = tadpole_tail_yrot(
            instance.render_state.age_in_ticks,
            instance.render_state.in_water,
        );
    }
}
