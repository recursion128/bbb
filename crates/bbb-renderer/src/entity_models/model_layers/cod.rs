use std::f32::consts::FRAC_PI_4;

use super::{PartPose, COD_TAN, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_COD: &str = "minecraft:cod#main";

// Vanilla 26.1 `CodModel.createBodyLayer` (atlas 32×32). All offsets share the `yo = 22` baseline.
// The fins are zero-thickness planes (`right`/`left` flat in Y, `tail`/`top` flat in X). Each cube
// carries both render paths' data: the colored debug tint (`COD_TAN`) and the textured `uv_size` /
// `texOffs` (`CubeDeformation.NONE`, so `uv_size == size`). The top fin's `texOffs(20, -6)` has a
// negative V origin, exactly as vanilla bakes it.
pub(in crate::entity_models) const COD_BODY: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -2.0, 0.0],
    [2.0, 4.0, 7.0],
    COD_TAN,
    [2.0, 4.0, 7.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const COD_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -2.0, -3.0],
    [2.0, 4.0, 3.0],
    COD_TAN,
    [2.0, 4.0, 3.0],
    [11.0, 0.0],
    false,
)];

pub(in crate::entity_models) const COD_NOSE: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -2.0, -1.0],
    [2.0, 3.0, 1.0],
    COD_TAN,
    [2.0, 3.0, 1.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const COD_RIGHT_FIN: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -1.0],
    [2.0, 0.0, 2.0],
    COD_TAN,
    [2.0, 0.0, 2.0],
    [22.0, 1.0],
    false,
)];

pub(in crate::entity_models) const COD_LEFT_FIN: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, -1.0],
    [2.0, 0.0, 2.0],
    COD_TAN,
    [2.0, 0.0, 2.0],
    [22.0, 4.0],
    false,
)];

pub(in crate::entity_models) const COD_TAIL_FIN: [ModelCube; 1] = [ModelCube::new(
    [0.0, -2.0, 0.0],
    [0.0, 4.0, 4.0],
    COD_TAN,
    [0.0, 4.0, 4.0],
    [22.0, 3.0],
    false,
)];

pub(in crate::entity_models) const COD_TOP_FIN: [ModelCube; 1] = [ModelCube::new(
    [0.0, -1.0, -1.0],
    [0.0, 1.0, 6.0],
    COD_TAN,
    [0.0, 1.0, 6.0],
    [20.0, -6.0],
    false,
)];

/// Bind poses for the cod parts, vanilla `CodModel.createBodyLayer` order: body, head, nose, right
/// fin (`zRot = -π/4`), left fin (`zRot = π/4`), tail fin, top fin.
const fn pose(offset: [f32; 3], rotation: [f32; 3]) -> PartPose {
    PartPose { offset, rotation }
}

const COD_BODY_POSE: PartPose = pose([0.0, 22.0, 0.0], [0.0, 0.0, 0.0]);
const COD_HEAD_POSE: PartPose = pose([0.0, 22.0, 0.0], [0.0, 0.0, 0.0]);
const COD_NOSE_POSE: PartPose = pose([0.0, 22.0, -3.0], [0.0, 0.0, 0.0]);
const COD_RIGHT_FIN_POSE: PartPose = pose([-1.0, 23.0, 0.0], [0.0, 0.0, -FRAC_PI_4]);
const COD_LEFT_FIN_POSE: PartPose = pose([1.0, 23.0, 0.0], [0.0, 0.0, FRAC_PI_4]);
const COD_TAIL_FIN_POSE: PartPose = pose([0.0, 22.0, 7.0], [0.0, 0.0, 0.0]);
const COD_TOP_FIN_POSE: PartPose = pose([0.0, 20.0, 0.0], [0.0, 0.0, 0.0]);

/// Vanilla `CodModel.setupAnim`: `tailFin.yRot = -amplitude * 0.45 * sin(0.6 * ageInTicks)`, with
/// `amplitude = isInWater ? 1.0 : 1.5` (a beached cod thrashes harder). The rest pose has
/// `yRot = 0`, so this is set absolutely.
pub(in crate::entity_models) fn cod_tail_fin_yrot(age_in_ticks: f32, in_water: bool) -> f32 {
    let amplitude = if in_water { 1.0 } else { 1.5 };
    -amplitude * 0.45 * (0.6 * age_in_ticks).sin()
}

/// Mutable cod model, mirroring vanilla `CodModel`. The seven parts hang off a synthetic root; each
/// carries both the colored tint and the textured UV, so one tree drives both render paths. The swim
/// wiggle and out-of-water flop live in the cod root transform; `setup_anim` only sways the tail fin.
pub(in crate::entity_models) struct CodModel {
    root: ModelPart,
}

impl CodModel {
    pub(in crate::entity_models) fn new() -> Self {
        let root = ModelPart::new(
            PART_POSE_ZERO,
            Vec::new(),
            vec![
                ("body", ModelPart::leaf(COD_BODY_POSE, COD_BODY.to_vec())),
                ("head", ModelPart::leaf(COD_HEAD_POSE, COD_HEAD.to_vec())),
                ("nose", ModelPart::leaf(COD_NOSE_POSE, COD_NOSE.to_vec())),
                (
                    "right_fin",
                    ModelPart::leaf(COD_RIGHT_FIN_POSE, COD_RIGHT_FIN.to_vec()),
                ),
                (
                    "left_fin",
                    ModelPart::leaf(COD_LEFT_FIN_POSE, COD_LEFT_FIN.to_vec()),
                ),
                (
                    "tail_fin",
                    ModelPart::leaf(COD_TAIL_FIN_POSE, COD_TAIL_FIN.to_vec()),
                ),
                (
                    "top_fin",
                    ModelPart::leaf(COD_TOP_FIN_POSE, COD_TOP_FIN.to_vec()),
                ),
            ],
        );
        Self { root }
    }
}

impl EntityModel for CodModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `CodModel.setupAnim` sways only the tail fin (`yRot`).
        let tail_yrot = cod_tail_fin_yrot(
            instance.render_state.age_in_ticks,
            instance.render_state.in_water,
        );
        self.root.child_mut("tail_fin").pose.rotation[1] = tail_yrot;
    }
}
