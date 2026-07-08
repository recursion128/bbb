use super::{CONDUIT_CYAN, PART_POSE_ZERO};
use crate::entity_models::catalog::ConduitModelPart;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

/// Vanilla `ModelLayers.CONDUIT_*` names registered by `ModelLayers`.
pub(in crate::entity_models) const MODEL_LAYER_CONDUIT_EYE: &str = "minecraft:conduit_eye#main";
pub(in crate::entity_models) const MODEL_LAYER_CONDUIT_WIND: &str = "minecraft:conduit_wind#main";
pub(in crate::entity_models) const MODEL_LAYER_CONDUIT_SHELL: &str = "minecraft:conduit_shell#main";
pub(in crate::entity_models) const MODEL_LAYER_CONDUIT_CAGE: &str = "minecraft:conduit_cage#main";

// Vanilla 26.1 `ConduitRenderer.createEyeLayer`: `texOffs(0, 0).addBox(-4, -4, 0, 8, 8, 0,
// CubeDeformation(0.01F))` on a 16x16 atlas. The deformation gives the zero-depth eye plane a
// visible 0.02-thick sliver while keeping the UV box at 8x8x0.
pub(in crate::entity_models) const CONDUIT_EYE_CUBE: ModelCube = ModelCube::new(
    [-4.01, -4.01, -0.01],
    [8.02, 8.02, 0.02],
    CONDUIT_CYAN,
    [8.0, 8.0, 0.0],
    [0.0, 0.0],
    false,
);

// Vanilla `createWindLayer`: `addBox(-8, -8, -8, 16, 16, 16)` on a 64x32 atlas.
pub(in crate::entity_models) const CONDUIT_WIND_CUBE: ModelCube = ModelCube::new(
    [-8.0, -8.0, -8.0],
    [16.0, 16.0, 16.0],
    CONDUIT_CYAN,
    [16.0, 16.0, 16.0],
    [0.0, 0.0],
    false,
);

// Vanilla `createShellLayer`: `addBox(-3, -3, -3, 6, 6, 6)` on a 32x16 atlas.
pub(in crate::entity_models) const CONDUIT_SHELL_CUBE: ModelCube = ModelCube::new(
    [-3.0, -3.0, -3.0],
    [6.0, 6.0, 6.0],
    CONDUIT_CYAN,
    [6.0, 6.0, 6.0],
    [0.0, 0.0],
    false,
);

// Vanilla `createCageLayer`: `addBox(-4, -4, -4, 8, 8, 8)` on a 32x16 atlas.
pub(in crate::entity_models) const CONDUIT_CAGE_CUBE: ModelCube = ModelCube::new(
    [-4.0, -4.0, -4.0],
    [8.0, 8.0, 8.0],
    CONDUIT_CYAN,
    [8.0, 8.0, 8.0],
    [0.0, 0.0],
    false,
);

pub(in crate::entity_models) struct ConduitModel {
    root: ModelPart,
}

impl ConduitModel {
    pub(in crate::entity_models) fn new(part: ConduitModelPart) -> Self {
        let cube = match part {
            ConduitModelPart::Shell => CONDUIT_SHELL_CUBE,
            ConduitModelPart::Cage => CONDUIT_CAGE_CUBE,
            ConduitModelPart::OuterWind { .. } | ConduitModelPart::InnerWind { .. } => {
                CONDUIT_WIND_CUBE
            }
            ConduitModelPart::Eye { .. } => CONDUIT_EYE_CUBE,
        };
        Self {
            root: ModelPart::leaf(PART_POSE_ZERO, vec![cube]),
        }
    }
}

impl EntityModel for ConduitModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {}
}
