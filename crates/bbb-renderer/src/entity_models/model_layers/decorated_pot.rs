use std::f32::consts::{FRAC_PI_2, PI};

use super::{PartPose, MODEL_CUBE_FACE_NORTH, PART_POSE_ZERO, POT_TERRACOTTA};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

/// Vanilla `ModelLayers.DECORATED_POT_BASE` / `DECORATED_POT_SIDES`
/// (`register("decorated_pot_base")` / `register("decorated_pot_sides")`), baked from
/// `DecoratedPotRenderer.createBaseLayer` / `createSidesLayer`.
pub(in crate::entity_models) const MODEL_LAYER_DECORATED_POT_BASE: &str =
    "minecraft:decorated_pot_base#main";
pub(in crate::entity_models) const MODEL_LAYER_DECORATED_POT_SIDES: &str =
    "minecraft:decorated_pot_sides#main";

// Vanilla 26.1 `DecoratedPotRenderer.createBaseLayer` (`DecoratedPotRenderer.java:83-101`, atlas
// 32×32): `neck` holds two boxes — `texOffs(0, 0).addBox(4, 17, 4, 8, 3, 8, deflate(-0.1))` and
// `texOffs(0, 5).addBox(5, 20, 5, 6, 1, 6, inflate(0.2))` — at
// `offsetAndRotation(0, 37, 16, π, 0, 0)` (the π X flip turns the neck upside down onto the pot
// top). A `CubeDeformation g` grows each box by `g` on every side (`min - g`, `size + 2g`) while
// the UV box keeps the undeformed dims. `top` and `bottom` share one
// `texOffs(-14, 13).addBox(0, 0, 0, 14, 0, 14)` zero-height plane at `offset(1, 16, 1)` /
// `offset(1, 0, 1)` (the negative U offset lands the plane's down/up faces at u = 0..28).
pub(in crate::entity_models) const DECORATED_POT_NECK_OUTER_CUBE: ModelCube = ModelCube::new(
    [4.1, 17.1, 4.1],
    [7.8, 2.8, 7.8],
    POT_TERRACOTTA,
    [8.0, 3.0, 8.0],
    [0.0, 0.0],
    false,
);
pub(in crate::entity_models) const DECORATED_POT_NECK_TOP_CUBE: ModelCube = ModelCube::new(
    [4.8, 19.8, 4.8],
    [6.4, 1.4, 6.4],
    POT_TERRACOTTA,
    [6.0, 1.0, 6.0],
    [0.0, 5.0],
    false,
);
pub(in crate::entity_models) const DECORATED_POT_TOP_BOTTOM_CUBE: ModelCube = ModelCube::new(
    [0.0, 0.0, 0.0],
    [14.0, 0.0, 14.0],
    POT_TERRACOTTA,
    [14.0, 0.0, 14.0],
    [-14.0, 13.0],
    false,
);

/// Vanilla `PartPose.offsetAndRotation(0, 37, 16, π, 0, 0)` — the neck pivot.
pub(in crate::entity_models) const DECORATED_POT_NECK_POSE: PartPose = PartPose {
    offset: [0.0, 37.0, 16.0],
    rotation: [PI, 0.0, 0.0],
};
/// Vanilla `PartPose.offsetAndRotation(1, 16, 1, 0, 0, 0)` — the top plane.
pub(in crate::entity_models) const DECORATED_POT_TOP_POSE: PartPose = PartPose {
    offset: [1.0, 16.0, 1.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Vanilla `PartPose.offsetAndRotation(1, 0, 1, 0, 0, 0)` — the bottom plane.
pub(in crate::entity_models) const DECORATED_POT_BOTTOM_POSE: PartPose = PartPose {
    offset: [1.0, 0.0, 1.0],
    rotation: [0.0, 0.0, 0.0],
};

// Vanilla `DecoratedPotRenderer.createSidesLayer` (`DecoratedPotRenderer.java:103-112`, atlas
// 16×16): one `texOffs(1, 0).addBox(0, 0, 0, 14, 16, 0, EnumSet.of(Direction.NORTH))` zero-depth
// plane shared by all four sides — only the NORTH face bakes a polygon — posed per side:
// `back(15, 16, 1, 0, 0, π)`, `left(1, 16, 1, 0, -π/2, π)`, `right(15, 16, 15, 0, π/2, π)`,
// `front(1, 16, 15, π, 0, 0)`.
pub(in crate::entity_models) const DECORATED_POT_SIDE_CUBE: ModelCube = ModelCube::new(
    [0.0, 0.0, 0.0],
    [14.0, 16.0, 0.0],
    POT_TERRACOTTA,
    [14.0, 16.0, 0.0],
    [1.0, 0.0],
    false,
)
.with_visible_faces(MODEL_CUBE_FACE_NORTH);

pub(in crate::entity_models) const DECORATED_POT_BACK_POSE: PartPose = PartPose {
    offset: [15.0, 16.0, 1.0],
    rotation: [0.0, 0.0, PI],
};
pub(in crate::entity_models) const DECORATED_POT_LEFT_POSE: PartPose = PartPose {
    offset: [1.0, 16.0, 1.0],
    rotation: [0.0, -FRAC_PI_2, PI],
};
pub(in crate::entity_models) const DECORATED_POT_RIGHT_POSE: PartPose = PartPose {
    offset: [15.0, 16.0, 15.0],
    rotation: [0.0, FRAC_PI_2, PI],
};
pub(in crate::entity_models) const DECORATED_POT_FRONT_POSE: PartPose = PartPose {
    offset: [1.0, 16.0, 15.0],
    rotation: [PI, 0.0, 0.0],
};

/// Vanilla `DecoratedPotRenderer`'s combined part set: the base layer's `neck`/`top`/`bottom` and
/// the sides layer's `back`/`left`/`right`/`front` as one tree (bbb keeps a single model per
/// instance; the per-layer texture split rides the retained-parts layer passes). The pot has no
/// part animation — the wobble is a root-transform rotation
/// (`DecoratedPotRenderer.submit`'s `rotateAround` chain).
pub(in crate::entity_models) struct DecoratedPotModel {
    root: ModelPart,
}

impl DecoratedPotModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![
                    (
                        "neck",
                        ModelPart::leaf(
                            DECORATED_POT_NECK_POSE,
                            vec![DECORATED_POT_NECK_OUTER_CUBE, DECORATED_POT_NECK_TOP_CUBE],
                        ),
                    ),
                    (
                        "top",
                        ModelPart::leaf(
                            DECORATED_POT_TOP_POSE,
                            vec![DECORATED_POT_TOP_BOTTOM_CUBE],
                        ),
                    ),
                    (
                        "bottom",
                        ModelPart::leaf(
                            DECORATED_POT_BOTTOM_POSE,
                            vec![DECORATED_POT_TOP_BOTTOM_CUBE],
                        ),
                    ),
                    (
                        "back",
                        ModelPart::leaf(DECORATED_POT_BACK_POSE, vec![DECORATED_POT_SIDE_CUBE]),
                    ),
                    (
                        "left",
                        ModelPart::leaf(DECORATED_POT_LEFT_POSE, vec![DECORATED_POT_SIDE_CUBE]),
                    ),
                    (
                        "right",
                        ModelPart::leaf(DECORATED_POT_RIGHT_POSE, vec![DECORATED_POT_SIDE_CUBE]),
                    ),
                    (
                        "front",
                        ModelPart::leaf(DECORATED_POT_FRONT_POSE, vec![DECORATED_POT_SIDE_CUBE]),
                    ),
                ],
            ),
        }
    }
}

impl EntityModel for DecoratedPotModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {}
}
