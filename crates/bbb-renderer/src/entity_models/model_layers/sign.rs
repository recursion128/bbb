use std::f32::consts::FRAC_PI_4;

use super::{PartPose, PART_POSE_ZERO, SIGN_WOOD};
use crate::entity_models::catalog::SignModelAttachment;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla bakes one layer per wood type (`ModelLayers.createStandingSignModelName` ->
// `sign/standing/<wood>#main`, `createWallSignModelName` -> `sign/wall/<wood>#main`,
// `createHangingSignModelName` -> `hanging_sign/<wood>/<attachment>#main`), but the mesh is
// identical across woods — only the bound sprite differs — so bbb keys one layer per mesh
// variant, like the chest layers.
pub(in crate::entity_models) const MODEL_LAYER_SIGN_STANDING: &str = "minecraft:sign/standing#main";
pub(in crate::entity_models) const MODEL_LAYER_SIGN_WALL: &str = "minecraft:sign/wall#main";
pub(in crate::entity_models) const MODEL_LAYER_HANGING_SIGN_CEILING: &str =
    "minecraft:hanging_sign/ceiling#main";
pub(in crate::entity_models) const MODEL_LAYER_HANGING_SIGN_CEILING_MIDDLE: &str =
    "minecraft:hanging_sign/ceiling_middle#main";
pub(in crate::entity_models) const MODEL_LAYER_HANGING_SIGN_WALL: &str =
    "minecraft:hanging_sign/wall#main";

const fn sign_cube(min: [f32; 3], size: [f32; 3], tex: [f32; 2]) -> ModelCube {
    ModelCube::new(min, size, SIGN_WOOD, size, tex, false)
}

// Vanilla 26.1 `StandingSignRenderer.createSignLayer` (atlas 64×32): the `sign` board is a
// 24×12×2 box at (-12, -14, -1) texOffs(0, 0); the standing form adds the `stick`, a 2×14×2 box
// at (-1, -2, -1) texOffs(0, 14). The mesh is authored in the vanilla Y-down model space; the
// root transform's `scale(RENDER_SCALE, -RENDER_SCALE, -RENDER_SCALE)` flip
// (`StandingSignRenderer.bodyTransformation`) maps it into the block.
pub(in crate::entity_models) const SIGN_BOARD_CUBE: ModelCube =
    sign_cube([-12.0, -14.0, -1.0], [24.0, 12.0, 2.0], [0.0, 0.0]);
pub(in crate::entity_models) const SIGN_STICK_CUBE: ModelCube =
    sign_cube([-1.0, -2.0, -1.0], [2.0, 14.0, 2.0], [0.0, 14.0]);

// Vanilla 26.1 `HangingSignRenderer.createHangingSignLayer` (atlas 64×32): every attachment has
// the 14×10×2 `board` at (-7, 0, -1) texOffs(0, 12); WALL adds the 16×2×4 `plank` at
// (-8, -6, -2) texOffs(0, 0); WALL and CEILING add the four angled chain planes (3×6×0 at
// texOffs(0, 6) / (6, 6), offset (±5, -6, 0), yRot ∓π/4); CEILING_MIDDLE instead hangs the
// straight 12×6×0 `vChains` plane at (-6, -6, 0) texOffs(14, 6).
pub(in crate::entity_models) const HANGING_SIGN_BOARD_CUBE: ModelCube =
    sign_cube([-7.0, 0.0, -1.0], [14.0, 10.0, 2.0], [0.0, 12.0]);
pub(in crate::entity_models) const HANGING_SIGN_PLANK_CUBE: ModelCube =
    sign_cube([-8.0, -6.0, -2.0], [16.0, 2.0, 4.0], [0.0, 0.0]);
pub(in crate::entity_models) const HANGING_SIGN_CHAIN_1_CUBE: ModelCube =
    sign_cube([-1.5, 0.0, 0.0], [3.0, 6.0, 0.0], [0.0, 6.0]);
pub(in crate::entity_models) const HANGING_SIGN_CHAIN_2_CUBE: ModelCube =
    sign_cube([-1.5, 0.0, 0.0], [3.0, 6.0, 0.0], [6.0, 6.0]);
pub(in crate::entity_models) const HANGING_SIGN_V_CHAINS_CUBE: ModelCube =
    sign_cube([-6.0, -6.0, 0.0], [12.0, 6.0, 0.0], [14.0, 6.0]);

const fn hanging_chain_pose(x: f32, y_rot: f32) -> PartPose {
    PartPose {
        offset: [x, -6.0, 0.0],
        rotation: [0.0, y_rot, 0.0],
    }
}

/// Vanilla 26.1 sign meshes: `StandingSignRenderer.createSignLayer` for the
/// standing/wall plain sign and `HangingSignRenderer.createHangingSignLayer`
/// for the three hanging attachments. No `setupAnim` — the sign model is
/// static; facing/rotation ride the root transform.
pub(in crate::entity_models) struct SignModel {
    root: ModelPart,
}

impl SignModel {
    pub(in crate::entity_models) fn new(attachment: SignModelAttachment) -> Self {
        let mut children: Vec<(&'static str, ModelPart)> = Vec::new();
        match attachment {
            SignModelAttachment::Standing => {
                children.push((
                    "sign",
                    ModelPart::leaf(PART_POSE_ZERO, vec![SIGN_BOARD_CUBE]),
                ));
                children.push((
                    "stick",
                    ModelPart::leaf(PART_POSE_ZERO, vec![SIGN_STICK_CUBE]),
                ));
            }
            SignModelAttachment::Wall => {
                children.push((
                    "sign",
                    ModelPart::leaf(PART_POSE_ZERO, vec![SIGN_BOARD_CUBE]),
                ));
            }
            SignModelAttachment::HangingCeiling
            | SignModelAttachment::HangingCeilingMiddle
            | SignModelAttachment::HangingWall => {
                children.push((
                    "board",
                    ModelPart::leaf(PART_POSE_ZERO, vec![HANGING_SIGN_BOARD_CUBE]),
                ));
                if matches!(attachment, SignModelAttachment::HangingWall) {
                    children.push((
                        "plank",
                        ModelPart::leaf(PART_POSE_ZERO, vec![HANGING_SIGN_PLANK_CUBE]),
                    ));
                }
                if matches!(attachment, SignModelAttachment::HangingCeilingMiddle) {
                    children.push((
                        "vChains",
                        ModelPart::leaf(PART_POSE_ZERO, vec![HANGING_SIGN_V_CHAINS_CUBE]),
                    ));
                } else {
                    children.push((
                        "normalChains",
                        ModelPart::new(
                            PART_POSE_ZERO,
                            Vec::new(),
                            vec![
                                (
                                    "chainL1",
                                    ModelPart::leaf(
                                        hanging_chain_pose(-5.0, -FRAC_PI_4),
                                        vec![HANGING_SIGN_CHAIN_1_CUBE],
                                    ),
                                ),
                                (
                                    "chainL2",
                                    ModelPart::leaf(
                                        hanging_chain_pose(-5.0, FRAC_PI_4),
                                        vec![HANGING_SIGN_CHAIN_2_CUBE],
                                    ),
                                ),
                                (
                                    "chainR1",
                                    ModelPart::leaf(
                                        hanging_chain_pose(5.0, -FRAC_PI_4),
                                        vec![HANGING_SIGN_CHAIN_1_CUBE],
                                    ),
                                ),
                                (
                                    "chainR2",
                                    ModelPart::leaf(
                                        hanging_chain_pose(5.0, FRAC_PI_4),
                                        vec![HANGING_SIGN_CHAIN_2_CUBE],
                                    ),
                                ),
                            ],
                        ),
                    ));
                }
            }
        }
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), children),
        }
    }
}

impl EntityModel for SignModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {
        // Vanilla sign models have no `setupAnim`; the mesh is static.
    }
}
