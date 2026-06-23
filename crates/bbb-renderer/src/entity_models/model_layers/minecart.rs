use super::{PartPose, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};
use std::f32::consts::{FRAC_PI_2, PI};

const PI_3_HALVES: f32 = PI * 1.5;

// The colored fallback paints the whole cart a neutral iron grey.
pub(in crate::entity_models) const MINECART_GRAY: [f32; 4] = [0.34, 0.35, 0.37, 1.0];

pub(in crate::entity_models) const MODEL_LAYER_MINECART: &str = "minecraft:minecart#main";

// Vanilla 26.1 MinecartModel.createBodyLayer(): the floor `bottom` panel (`texOffs(0, 10)`,
// a 20x16x2 box laid flat) plus four identical 16x8x2 wall panels (`texOffs(0, 0)`) rotated
// to box in the cart. None are mirrored.
const MINECART_BOTTOM_POSE: PartPose = PartPose {
    offset: [0.0, 4.0, 0.0],
    rotation: [FRAC_PI_2, 0.0, 0.0],
};
const MINECART_FRONT_POSE: PartPose = PartPose {
    offset: [-9.0, 4.0, 0.0],
    rotation: [0.0, PI_3_HALVES, 0.0],
};
const MINECART_BACK_POSE: PartPose = PartPose {
    offset: [9.0, 4.0, 0.0],
    rotation: [0.0, FRAC_PI_2, 0.0],
};
const MINECART_LEFT_POSE: PartPose = PartPose {
    offset: [0.0, 4.0, -7.0],
    rotation: [0.0, PI, 0.0],
};
const MINECART_RIGHT_POSE: PartPose = PartPose {
    offset: [0.0, 4.0, 7.0],
    rotation: [0.0, 0.0, 0.0],
};

// The floor `bottom` panel: `texOffs(0, 10)`, box(-10, -8, -1, 20x16x2). Each cube carries both
// render paths' data: the colored debug tint (`MINECART_GRAY`) and the textured `uv_size` / `texOffs`.
pub(in crate::entity_models) const MINECART_BOTTOM: [ModelCube; 1] = [ModelCube::new(
    [-10.0, -8.0, -1.0],
    [20.0, 16.0, 2.0],
    MINECART_GRAY,
    [20.0, 16.0, 2.0],
    [0.0, 10.0],
    false,
)];

// The four walls share one `texOffs(0, 0)` box(-8, -9, -1, 16x8x2), rotated to face out of each side.
pub(in crate::entity_models) const MINECART_WALL: [ModelCube; 1] = [ModelCube::new(
    [-8.0, -9.0, -1.0],
    [16.0, 8.0, 2.0],
    MINECART_GRAY,
    [16.0, 8.0, 2.0],
    [0.0, 0.0],
    false,
)];

/// Mutable minecart model, mirroring vanilla `MinecartModel`. The unified tree is built once with the
/// vanilla `MinecartModel.createBodyLayer` child names ("bottom" floor panel plus the four boxed-in
/// "front"/"back"/"left"/"right" wall panels). Vanilla `MinecartModel` has no `setupAnim`, so
/// `setup_anim` is a no-op — the cart is a static box rendered at its rest pose under the entity root
/// transform.
pub(in crate::entity_models) struct MinecartModel {
    root: ModelPart,
}

impl MinecartModel {
    pub(in crate::entity_models) fn new() -> Self {
        let children: Vec<(&'static str, ModelPart)> = vec![
            (
                "bottom",
                ModelPart::leaf(MINECART_BOTTOM_POSE, MINECART_BOTTOM.to_vec()),
            ),
            (
                "front",
                ModelPart::leaf(MINECART_FRONT_POSE, MINECART_WALL.to_vec()),
            ),
            (
                "back",
                ModelPart::leaf(MINECART_BACK_POSE, MINECART_WALL.to_vec()),
            ),
            (
                "left",
                ModelPart::leaf(MINECART_LEFT_POSE, MINECART_WALL.to_vec()),
            ),
            (
                "right",
                ModelPart::leaf(MINECART_RIGHT_POSE, MINECART_WALL.to_vec()),
            ),
        ];
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), children),
        }
    }
}

impl EntityModel for MinecartModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {}
}
