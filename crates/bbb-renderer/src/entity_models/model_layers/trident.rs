use super::{PART_POSE_ZERO, TRIDENT_POLE, TRIDENT_SPIKE};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_TRIDENT: &str = "minecraft:trident#main";

// Vanilla 26.1 `TridentModel.createLayer` (atlas 32×32). The mesh root holds the `pole` (a 1×25×1
// shaft), which parents the `base` crossguard and the three spikes (left / middle / right) — all at
// ZERO. `TridentModel` is a `Model<Unit>` with no animation, so the geometry is complete; nothing is
// deferred on the geometry side. `ThrownTridentRenderer` orients the trident along its flight with
// `Ry(yRot - 90)` then `Rz(xRot + 90)` (the `+90` points the upright pole along the flight axis),
// captured by `trident_model_root_transform`. Each unified cube carries both the colored debug tint
// (`TRIDENT_POLE` / `TRIDENT_SPIKE`) and the textured `uv_size` / `texOffs`; the right spike is the
// only mirrored box (it samples the same atlas region as the left spike, flipped). The base texture
// submit is wired; the enchant-foil overlay is recorded as glint submission metadata while GPU glint
// presentation remains deferred.

// `pole`: the 1×25×1 shaft.
pub(in crate::entity_models) const TRIDENT_POLE_CUBE: ModelCube = ModelCube::new(
    [-0.5, 2.0, -0.5],
    [1.0, 25.0, 1.0],
    TRIDENT_POLE,
    [1.0, 25.0, 1.0],
    [0.0, 6.0],
    false,
);

// `base`: the 3×2×1 crossguard.
pub(in crate::entity_models) const TRIDENT_BASE_CUBE: ModelCube = ModelCube::new(
    [-1.5, 0.0, -0.5],
    [3.0, 2.0, 1.0],
    TRIDENT_POLE,
    [3.0, 2.0, 1.0],
    [4.0, 0.0],
    false,
);

// The three 1×4×1 spikes (`left` / `middle` / `right`; the right one samples the left's atlas region
// mirrored).
pub(in crate::entity_models) const TRIDENT_LEFT_SPIKE_CUBE: ModelCube = ModelCube::new(
    [-2.5, -3.0, -0.5],
    [1.0, 4.0, 1.0],
    TRIDENT_SPIKE,
    [1.0, 4.0, 1.0],
    [4.0, 3.0],
    false,
);
pub(in crate::entity_models) const TRIDENT_MIDDLE_SPIKE_CUBE: ModelCube = ModelCube::new(
    [-0.5, -4.0, -0.5],
    [1.0, 4.0, 1.0],
    TRIDENT_SPIKE,
    [1.0, 4.0, 1.0],
    [0.0, 0.0],
    false,
);
pub(in crate::entity_models) const TRIDENT_RIGHT_SPIKE_CUBE: ModelCube = ModelCube::new(
    [1.5, -3.0, -0.5],
    [1.0, 4.0, 1.0],
    TRIDENT_SPIKE,
    [1.0, 4.0, 1.0],
    [4.0, 3.0],
    true,
);

/// Static thrown-trident model mirroring vanilla `TridentModel`: `pole` → {`base`, three spikes}, no
/// `setup_anim`. Each cube carries the colored tint and the textured UV.
pub(in crate::entity_models) struct TridentModel {
    root: ModelPart,
}

impl TridentModel {
    pub(in crate::entity_models) fn new() -> Self {
        let pole = ModelPart::new(
            PART_POSE_ZERO,
            vec![TRIDENT_POLE_CUBE],
            vec![
                (
                    "base",
                    ModelPart::leaf(PART_POSE_ZERO, vec![TRIDENT_BASE_CUBE]),
                ),
                (
                    "left_spike",
                    ModelPart::leaf(PART_POSE_ZERO, vec![TRIDENT_LEFT_SPIKE_CUBE]),
                ),
                (
                    "middle_spike",
                    ModelPart::leaf(PART_POSE_ZERO, vec![TRIDENT_MIDDLE_SPIKE_CUBE]),
                ),
                (
                    "right_spike",
                    ModelPart::leaf(PART_POSE_ZERO, vec![TRIDENT_RIGHT_SPIKE_CUBE]),
                ),
            ],
        );
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![("pole", pole)]),
        }
    }
}

impl EntityModel for TridentModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {}
}
