use super::{LLAMA_SPIT_COLOR, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_LLAMA_SPIT: &str = "minecraft:llama_spit#main";

// Vanilla 26.1 `LlamaSpitModel.createBodyLayer` (atlas 64×32). The mesh root holds a single `main`
// part at ZERO with seven 2×2×2 boxes (all at `texOffs(0, 0)`) forming a plus/cross: a centre cube
// with one neighbour stepping out along each of the +X/-X, +Y/-Y, +Z/-Z directions. `LlamaSpitModel`
// has no `setupAnim`, so the model is fully static. `LlamaSpitRenderer` is a plain `EntityRenderer`
// that orients the spit along its flight (`translate(0, 0.15, 0) · Ry(yRot - 90) · Rz(xRot)`),
// captured by `llama_spit_model_root_transform`. Each cube carries the colored debug tint and the
// textured `uv_size` / `texOffs` (every box shares the one `texOffs(0, 0)`, no mirror).

const fn llama_spit_cube(min: [f32; 3]) -> ModelCube {
    ModelCube::new(
        min,
        [2.0, 2.0, 2.0],
        LLAMA_SPIT_COLOR,
        [2.0, 2.0, 2.0],
        [0.0, 0.0],
        false,
    )
}

pub(in crate::entity_models) const LLAMA_SPIT_CUBES: [ModelCube; 7] = [
    llama_spit_cube([-4.0, 0.0, 0.0]),
    llama_spit_cube([0.0, -4.0, 0.0]),
    llama_spit_cube([0.0, 0.0, -4.0]),
    llama_spit_cube([0.0, 0.0, 0.0]),
    llama_spit_cube([2.0, 0.0, 0.0]),
    llama_spit_cube([0.0, 2.0, 0.0]),
    llama_spit_cube([0.0, 0.0, 2.0]),
];

/// Static llama-spit model mirroring vanilla `LlamaSpitModel`: a single `main` part at ZERO holding
/// the seven-box cross, no `setup_anim`. Each cube carries the colored tint and textured UV.
pub(in crate::entity_models) struct LlamaSpitModel {
    root: ModelPart,
}

impl LlamaSpitModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![(
                    "main",
                    ModelPart::leaf(PART_POSE_ZERO, LLAMA_SPIT_CUBES.to_vec()),
                )],
            ),
        }
    }
}

impl EntityModel for LlamaSpitModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {}
}
