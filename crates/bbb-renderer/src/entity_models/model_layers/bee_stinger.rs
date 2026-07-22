use super::{PartPose, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 `BeeStingerModel.createBodyLayer` (atlas 16×16): two copies of one
// 2×1×0 XY plane at the root, pitched π/4 and 3π/4 around X to form a crossed stinger.
pub(in crate::entity_models) const BEE_STINGER_CROSS_CUBE: ModelCube = ModelCube::new(
    [-1.0, -0.5, 0.0],
    [2.0, 1.0, 0.0],
    [1.0; 4],
    [2.0, 1.0, 0.0],
    [0.0, 0.0],
    false,
);

const BEE_STINGER_CROSS_1_POSE: PartPose = PartPose {
    offset: [0.0; 3],
    rotation: [std::f32::consts::FRAC_PI_4, 0.0, 0.0],
};
const BEE_STINGER_CROSS_2_POSE: PartPose = PartPose {
    offset: [0.0; 3],
    rotation: [3.0 * std::f32::consts::FRAC_PI_4, 0.0, 0.0],
};

/// Static crossed-plane model used only by `BeeStingerLayer`.
pub(in crate::entity_models) struct BeeStingerModel {
    root: ModelPart,
}

impl BeeStingerModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![
                    (
                        "cross_1",
                        ModelPart::leaf(BEE_STINGER_CROSS_1_POSE, vec![BEE_STINGER_CROSS_CUBE]),
                    ),
                    (
                        "cross_2",
                        ModelPart::leaf(BEE_STINGER_CROSS_2_POSE, vec![BEE_STINGER_CROSS_CUBE]),
                    ),
                ],
            ),
        }
    }
}

impl EntityModel for BeeStingerModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {}
}
