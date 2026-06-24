use super::{LEASH_KNOT_COLOR, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 `LeashKnotModel.createBodyLayer` (atlas 32×32): the mesh root holds a single `knot`
// part at ZERO with one 6×8×6 box at texOffs(0, 0). `LeashKnotModel` has no `setupAnim`, so the model
// is fully static. The unified cube carries both render paths' data: the colored debug tint
// (`LEASH_KNOT_COLOR`) and the textured `uv_size` / `texOffs`. `LeashKnotRenderer` applies only the
// model flip (`scale(-1, -1, 1)`), captured by `leash_knot_model_root_transform`. The vanilla texture
// folder/name is `lead_knot/lead_knot`, not `leash_knot`.
pub(in crate::entity_models) const LEASH_KNOT_KNOT_CUBE: ModelCube = ModelCube::new(
    [-3.0, -8.0, -3.0],
    [6.0, 8.0, 6.0],
    LEASH_KNOT_COLOR,
    [6.0, 8.0, 6.0],
    [0.0, 0.0],
    false,
);

/// Static leash-knot model mirroring vanilla `LeashKnotModel`: one `knot` child at ZERO, no
/// `setup_anim`. Each cube carries the colored tint and the textured UV.
pub(in crate::entity_models) struct LeashKnotModel {
    root: ModelPart,
}

impl LeashKnotModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![(
                    "knot",
                    ModelPart::leaf(PART_POSE_ZERO, vec![LEASH_KNOT_KNOT_CUBE]),
                )],
            ),
        }
    }
}

impl EntityModel for LeashKnotModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {}
}
