use super::{PART_POSE_ZERO, SHULKER_BULLET_COLOR};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_SHULKER_BULLET: &str =
    "minecraft:shulker_bullet#main";

// Vanilla 26.1 `ShulkerBulletModel.createBodyLayer` (atlas 64×32). The mesh root holds a single
// `main` part at ZERO with three interlocking slabs (one flat in each of the XY / YZ / XZ planes):
// `texOffs(0, 0)` 8×8×2, `texOffs(0, 10)` 2×8×8, and `texOffs(20, 0)` 8×2×8. `setupAnim` only sets
// `main.yRot`/`main.xRot` from the bullet's facing (reproduced through `body_rot` / `head_pitch` in
// `shulker_bullet_model_root_transform`), so the geometry is complete. `ShulkerBulletRenderer.submit`
// is reproduced in that transform: `translate(0, 0.15, 0)`, the `ageInTicks`-driven tumble, then
// `scale(-0.5, -0.5, 0.5)`. The textured path also re-submits this same posed model as the vanilla
// translucent 1.5× outer shell. Each cube carries the colored tint and the textured UV.

pub(in crate::entity_models) const SHULKER_BULLET_CUBES: [ModelCube; 3] = [
    ModelCube::new(
        [-4.0, -4.0, -1.0],
        [8.0, 8.0, 2.0],
        SHULKER_BULLET_COLOR,
        [8.0, 8.0, 2.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-1.0, -4.0, -4.0],
        [2.0, 8.0, 8.0],
        SHULKER_BULLET_COLOR,
        [2.0, 8.0, 8.0],
        [0.0, 10.0],
        false,
    ),
    ModelCube::new(
        [-4.0, -1.0, -4.0],
        [8.0, 2.0, 8.0],
        SHULKER_BULLET_COLOR,
        [8.0, 2.0, 8.0],
        [20.0, 0.0],
        false,
    ),
];

/// Static shulker-bullet model mirroring vanilla `ShulkerBulletModel`: a single `main` part at ZERO
/// holding the three interlocking slabs (the facing lives in the root transform), no `setup_anim`.
pub(in crate::entity_models) struct ShulkerBulletModel {
    root: ModelPart,
}

impl ShulkerBulletModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![(
                    "main",
                    ModelPart::leaf(PART_POSE_ZERO, SHULKER_BULLET_CUBES.to_vec()),
                )],
            ),
        }
    }
}

impl EntityModel for ShulkerBulletModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {}
}
