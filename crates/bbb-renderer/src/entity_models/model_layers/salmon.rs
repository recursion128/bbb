use super::{PartPose, PART_POSE_ZERO, SALMON_RED};
use crate::entity_models::catalog::SalmonModelSize;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

use std::f32::consts::FRAC_PI_4;

// Vanilla 26.1 `SalmonModel.createBodyLayer` (atlas 32×32). The body is split into a front and back
// segment (the back sways), each carrying a flat top fin; the back also carries the flat tail fin. The
// side fins are zero-thickness planes. `CubeDeformation.NONE`, so each `uv_size` equals the geometry
// size, and no cube mirrors. Each unified cube carries both render paths' data: the colored debug tint
// (`SALMON_RED`) and the textured `uv_size` / `texOffs` (the right fin keeps its negative
// `texOffs(-4, 0)` U origin).
pub(in crate::entity_models) const SALMON_BODY_FRONT: [ModelCube; 1] = [ModelCube::new(
    [-1.5, -2.5, 0.0],
    [3.0, 5.0, 8.0],
    SALMON_RED,
    [3.0, 5.0, 8.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const SALMON_BODY_BACK: [ModelCube; 1] = [ModelCube::new(
    [-1.5, -2.5, 0.0],
    [3.0, 5.0, 8.0],
    SALMON_RED,
    [3.0, 5.0, 8.0],
    [0.0, 13.0],
    false,
)];

pub(in crate::entity_models) const SALMON_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -2.0, -3.0],
    [2.0, 4.0, 3.0],
    SALMON_RED,
    [2.0, 4.0, 3.0],
    [22.0, 0.0],
    false,
)];

pub(in crate::entity_models) const SALMON_BACK_FIN: [ModelCube; 1] = [ModelCube::new(
    [0.0, -2.5, 0.0],
    [0.0, 5.0, 6.0],
    SALMON_RED,
    [0.0, 5.0, 6.0],
    [20.0, 10.0],
    false,
)];

pub(in crate::entity_models) const SALMON_TOP_FRONT_FIN: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, 0.0],
    [0.0, 2.0, 3.0],
    SALMON_RED,
    [0.0, 2.0, 3.0],
    [2.0, 1.0],
    false,
)];

pub(in crate::entity_models) const SALMON_TOP_BACK_FIN: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, 0.0],
    [0.0, 2.0, 4.0],
    SALMON_RED,
    [0.0, 2.0, 4.0],
    [0.0, 2.0],
    false,
)];

pub(in crate::entity_models) const SALMON_RIGHT_FIN: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, 0.0],
    [2.0, 0.0, 2.0],
    SALMON_RED,
    [2.0, 0.0, 2.0],
    [-4.0, 0.0],
    false,
)];

pub(in crate::entity_models) const SALMON_LEFT_FIN: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, 0.0],
    [2.0, 0.0, 2.0],
    SALMON_RED,
    [2.0, 0.0, 2.0],
    [0.0, 0.0],
    false,
)];

// Root-part poses (vanilla `SalmonModel.createBodyLayer` root order): body front, body back (swayed by
// `setupAnim`), head, right fin (`zRot = -π/4`), left fin (`zRot = π/4`).
pub(in crate::entity_models) const SALMON_BODY_FRONT_POSE: PartPose = PartPose {
    offset: [0.0, 20.0, -7.2],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const SALMON_BODY_BACK_POSE: PartPose = PartPose {
    offset: [0.0, 20.0, 0.8],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const SALMON_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 20.0, -7.2],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const SALMON_RIGHT_FIN_POSE: PartPose = PartPose {
    offset: [-1.5, 21.5, -7.2],
    rotation: [0.0, 0.0, -FRAC_PI_4],
};
pub(in crate::entity_models) const SALMON_LEFT_FIN_POSE: PartPose = PartPose {
    offset: [1.5, 21.5, -7.2],
    rotation: [0.0, 0.0, FRAC_PI_4],
};

// Fin child poses: the forward top fin hangs off the front body; the tail fin and rear top fin hang
// off the back body (so they sway with it).
pub(in crate::entity_models) const SALMON_TOP_FRONT_FIN_POSE: PartPose = PartPose {
    offset: [0.0, -4.5, 5.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const SALMON_BACK_FIN_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 8.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const SALMON_TOP_BACK_FIN_POSE: PartPose = PartPose {
    offset: [0.0, -4.5, -1.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla `SalmonModel`/`SalmonRenderer` swim multipliers: a salmon in water uses
/// `(amplitude 1.0, angle 1.0)`; a beached salmon thrashes harder and faster
/// `(1.3, 1.7)`. Shared by the body-back sway and the renderer body wiggle.
pub(in crate::entity_models) fn salmon_swim_multipliers(in_water: bool) -> (f32, f32) {
    if in_water {
        (1.0, 1.0)
    } else {
        (1.3, 1.7)
    }
}

/// Vanilla `SalmonModel.setupAnim`: `bodyBack.yRot = -amplitude * 0.25 * sin(angle *
/// 0.6 * ageInTicks)`. The rest pose has `yRot = 0`, so this is set absolutely.
pub(in crate::entity_models) fn salmon_body_back_yrot(age_in_ticks: f32, in_water: bool) -> f32 {
    let (amplitude, angle) = salmon_swim_multipliers(in_water);
    -amplitude * 0.25 * (angle * 0.6 * age_in_ticks).sin()
}

// Vanilla 26.1 `ModelLayers.SALMON` / `SALMON_SMALL` / `SALMON_LARGE` (`SalmonRenderer`).
// The geometry is shared; the size layers only differ by a `MeshTransformer.scaling`
// factor, which the renderer folds into the root transform.
pub(in crate::entity_models) const MODEL_LAYER_SALMON: &str = "minecraft:salmon#main";
pub(in crate::entity_models) const MODEL_LAYER_SALMON_SMALL: &str = "minecraft:salmon_small#main";
pub(in crate::entity_models) const MODEL_LAYER_SALMON_LARGE: &str = "minecraft:salmon_large#main";

/// Vanilla `SalmonRenderer` selects the small / medium / large `SalmonModel` layer by
/// `Salmon.Variant`; the medium layer is the unscaled `ModelLayers.SALMON`.
pub(in crate::entity_models) fn salmon_model_layer(size: SalmonModelSize) -> &'static str {
    match size {
        SalmonModelSize::Small => MODEL_LAYER_SALMON_SMALL,
        SalmonModelSize::Medium => MODEL_LAYER_SALMON,
        SalmonModelSize::Large => MODEL_LAYER_SALMON_LARGE,
    }
}

/// Mutable salmon model, mirroring vanilla `SalmonModel`. The unified tree is built once with the
/// vanilla `SalmonModel.createBodyLayer` child names: `body_front` (carrying `top_front_fin`),
/// `body_back` (carrying `back_fin` + `top_back_fin`, swayed by `setupAnim`), `head`, `right_fin`,
/// `left_fin`. The same tree drives both render paths. `setup_anim` sways only the back body segment
/// (which carries the tail and rear top fin); the swim wiggle, out-of-water flop, and
/// small/medium/large mesh scale live in the salmon root transform.
pub(in crate::entity_models) struct SalmonModel {
    root: ModelPart,
}

impl SalmonModel {
    pub(in crate::entity_models) fn new() -> Self {
        let body_front = ModelPart::new(
            SALMON_BODY_FRONT_POSE,
            SALMON_BODY_FRONT.to_vec(),
            vec![(
                "top_front_fin",
                ModelPart::leaf(SALMON_TOP_FRONT_FIN_POSE, SALMON_TOP_FRONT_FIN.to_vec()),
            )],
        );
        let body_back = ModelPart::new(
            SALMON_BODY_BACK_POSE,
            SALMON_BODY_BACK.to_vec(),
            vec![
                (
                    "back_fin",
                    ModelPart::leaf(SALMON_BACK_FIN_POSE, SALMON_BACK_FIN.to_vec()),
                ),
                (
                    "top_back_fin",
                    ModelPart::leaf(SALMON_TOP_BACK_FIN_POSE, SALMON_TOP_BACK_FIN.to_vec()),
                ),
            ],
        );
        let children: Vec<(&'static str, ModelPart)> = vec![
            ("body_front", body_front),
            ("body_back", body_back),
            (
                "head",
                ModelPart::leaf(SALMON_HEAD_POSE, SALMON_HEAD.to_vec()),
            ),
            (
                "right_fin",
                ModelPart::leaf(SALMON_RIGHT_FIN_POSE, SALMON_RIGHT_FIN.to_vec()),
            ),
            (
                "left_fin",
                ModelPart::leaf(SALMON_LEFT_FIN_POSE, SALMON_LEFT_FIN.to_vec()),
            ),
        ];
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), children),
        }
    }
}

impl EntityModel for SalmonModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `SalmonModel.setupAnim`: `bodyBack.yRot = sway` (the back segment carries the tail
        // and rear top fin as children, so they swing with it).
        let body_back_yrot = salmon_body_back_yrot(
            instance.render_state.age_in_ticks,
            instance.render_state.in_water,
        );
        self.root.child_mut("body_back").pose.rotation[1] = body_back_yrot;
    }
}
