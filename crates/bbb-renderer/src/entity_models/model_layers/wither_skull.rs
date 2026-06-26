use super::{PART_POSE_ZERO, WITHER_SKULL_GRAY};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 `WitherSkullRenderer.createSkullLayer` (atlas 64×64): one `head` part at ZERO with a
// single 8×8×8 box (`addBox(-4, -8, -4, 8, 8, 8)` at `texOffs(0, 35)`). `SkullModel.setupAnim` turns
// the head by the projectile's flight `yRot`/`xRot`; since the single part sits at ZERO that facing is
// folded into `wither_skull_model_root_transform` (together with the renderer's `scale(-1, -1, 1)`
// flip). `WitherSkullRenderer` is a plain `EntityRenderer`; the `wither.png` texture is wired here,
// while the `wither_invulnerable.png` `isDangerous` swap stays deferred. The cube carries the colored
// debug tint and the textured `uv_size` / `texOffs(0, 35)`.

pub(in crate::entity_models) const WITHER_SKULL_CUBE: ModelCube = ModelCube::new(
    [-4.0, -8.0, -4.0],
    [8.0, 8.0, 8.0],
    WITHER_SKULL_GRAY,
    [8.0, 8.0, 8.0],
    [0.0, 35.0],
    false,
);

// Vanilla `SkullModel.createMobHeadLayer` used by `CustomHeadLayer` for skeleton / wither-skeleton /
// zombie / creeper skull equipment: one `head` cube at `texOffs(0, 0)` on a 64x32 texture.
pub(in crate::entity_models) const CUSTOM_HEAD_SKULL_CUBE: ModelCube = ModelCube::new(
    [-4.0, -8.0, -4.0],
    [8.0, 8.0, 8.0],
    WITHER_SKULL_GRAY,
    [8.0, 8.0, 8.0],
    [0.0, 0.0],
    false,
);

/// Static wither-skull model mirroring vanilla `SkullModel` at its ZERO rest pose: a single `head`
/// part holding the 8×8×8 skull box (the flight facing lives in the root transform), no `setup_anim`.
pub(in crate::entity_models) struct WitherSkullModel {
    root: ModelPart,
}

impl WitherSkullModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![(
                    "head",
                    ModelPart::leaf(PART_POSE_ZERO, vec![WITHER_SKULL_CUBE]),
                )],
            ),
        }
    }
}

impl EntityModel for WitherSkullModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {}
}

/// Static mob-head model for `CustomHeadLayer` skull equipment. The host model already supplied the
/// posed head transform, so this skull tree remains at its baked ZERO pose.
pub(in crate::entity_models) struct CustomHeadSkullModel {
    root: ModelPart,
}

impl CustomHeadSkullModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![(
                    "head",
                    ModelPart::leaf(PART_POSE_ZERO, vec![CUSTOM_HEAD_SKULL_CUBE]),
                )],
            ),
        }
    }
}

impl EntityModel for CustomHeadSkullModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {}
}
