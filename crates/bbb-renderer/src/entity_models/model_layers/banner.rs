use std::f32::consts::{PI, TAU};

use super::{PartPose, BANNER_CLOTH, PART_POSE_ZERO, SIGN_WOOD};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla bakes four banner layers (`ModelLayers.STANDING_BANNER` -> `standing_banner#main`,
// `STANDING_BANNER_FLAG` -> `standing_banner#flag`, `WALL_BANNER` -> `wall_banner#main`,
// `WALL_BANNER_FLAG` -> `wall_banner#flag`); bbb keeps one part tree per attachment and splits
// the frame/flag submits over retained-parts layer passes, like the decorated pot.
pub(in crate::entity_models) const MODEL_LAYER_STANDING_BANNER: &str =
    "minecraft:standing_banner#main";
pub(in crate::entity_models) const MODEL_LAYER_STANDING_BANNER_FLAG: &str =
    "minecraft:standing_banner#flag";
pub(in crate::entity_models) const MODEL_LAYER_WALL_BANNER: &str = "minecraft:wall_banner#main";
pub(in crate::entity_models) const MODEL_LAYER_WALL_BANNER_FLAG: &str =
    "minecraft:wall_banner#flag";

/// Vanilla `BannerRenderer.submitBanner`'s frame model parts (`BannerModel`): the standing form's
/// `pole` + `bar`, the wall form's `bar` only (`BannerModel.createBodyLayer(standing)`).
pub(in crate::entity_models) const STANDING_BANNER_FRAME_PARTS: &[&str] = &["pole", "bar"];
pub(in crate::entity_models) const WALL_BANNER_FRAME_PARTS: &[&str] = &["bar"];
/// The `BannerFlagModel` flag part the banner-base and every pattern pass re-submit.
pub(in crate::entity_models) const BANNER_FLAG_PARTS: &[&str] = &["flag"];

// Vanilla 26.1 `BannerModel.createBodyLayer` (`BannerModel.java:25-37`, atlas 64×64): the
// standing form's `pole` is a 2×42×2 box at (-1, -42, -1) texOffs(44, 0); the `bar` is a 20×2×2
// box texOffs(0, 42) at (-10, -44, -1) standing / (-10, -20.5, 9.5) wall. The mesh is authored
// in the vanilla Y-down model space; the root transform's `scale(⅔, -⅔, -⅔)` flip
// (`BannerRenderer.modelTransformation`) maps it into the block.
pub(in crate::entity_models) const BANNER_POLE_CUBE: ModelCube = ModelCube::new(
    [-1.0, -42.0, -1.0],
    [2.0, 42.0, 2.0],
    SIGN_WOOD,
    [2.0, 42.0, 2.0],
    [44.0, 0.0],
    false,
);
pub(in crate::entity_models) const BANNER_STANDING_BAR_CUBE: ModelCube = ModelCube::new(
    [-10.0, -44.0, -1.0],
    [20.0, 2.0, 2.0],
    SIGN_WOOD,
    [20.0, 2.0, 2.0],
    [0.0, 42.0],
    false,
);
pub(in crate::entity_models) const BANNER_WALL_BAR_CUBE: ModelCube = ModelCube::new(
    [-10.0, -20.5, 9.5],
    [20.0, 2.0, 2.0],
    SIGN_WOOD,
    [20.0, 2.0, 2.0],
    [0.0, 42.0],
    false,
);

// Vanilla 26.1 `BannerFlagModel.createFlagLayer` (`BannerFlagModel.java:22-31`, atlas 64×64): the
// `flag` is a 20×40×1 box at (-10, 0, -2) texOffs(0, 0), pivoted at
// `PartPose.offset(0, -44, 0)` standing / `(0, -20.5, 10.5)` wall so the swing rotates about the
// bar.
pub(in crate::entity_models) const BANNER_FLAG_CUBE: ModelCube = ModelCube::new(
    [-10.0, 0.0, -2.0],
    [20.0, 40.0, 1.0],
    BANNER_CLOTH,
    [20.0, 40.0, 1.0],
    [0.0, 0.0],
    false,
);

/// Vanilla `PartPose.offset(0, -44, 0)` — the standing flag pivot.
pub(in crate::entity_models) const STANDING_BANNER_FLAG_POSE: PartPose = PartPose {
    offset: [0.0, -44.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Vanilla `PartPose.offset(0, -20.5, 10.5)` — the wall flag pivot.
pub(in crate::entity_models) const WALL_BANNER_FLAG_POSE: PartPose = PartPose {
    offset: [0.0, -20.5, 10.5],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla `BannerFlagModel.setupAnim(phase)`:
/// `flag.xRot = (-0.0125F + 0.01F * Mth.cos(2π * phase)) * π`.
pub(in crate::entity_models) fn banner_flag_swing_x_rot(phase: f32) -> f32 {
    (-0.0125 + 0.01 * (TAU * phase).cos()) * PI
}

/// Vanilla `BannerRenderer`'s combined part set per attachment: the `BannerModel` frame
/// (`pole`/`bar`; the wall form has no pole) and the `BannerFlagModel` `flag` as one tree (bbb
/// keeps a single model per instance; the frame/flag/pattern texture split rides the
/// retained-parts layer passes). The only animation is the flag swing —
/// `BannerFlagModel.setupAnim` over `EntityRenderState.banner_flag_phase`.
pub(in crate::entity_models) struct BannerModel {
    root: ModelPart,
}

impl BannerModel {
    pub(in crate::entity_models) fn new(wall: bool) -> Self {
        let mut children: Vec<(&'static str, ModelPart)> = Vec::new();
        if !wall {
            children.push((
                "pole",
                ModelPart::leaf(PART_POSE_ZERO, vec![BANNER_POLE_CUBE]),
            ));
        }
        children.push((
            "bar",
            ModelPart::leaf(
                PART_POSE_ZERO,
                vec![if wall {
                    BANNER_WALL_BAR_CUBE
                } else {
                    BANNER_STANDING_BAR_CUBE
                }],
            ),
        ));
        children.push((
            "flag",
            ModelPart::leaf(
                if wall {
                    WALL_BANNER_FLAG_POSE
                } else {
                    STANDING_BANNER_FLAG_POSE
                },
                vec![BANNER_FLAG_CUBE],
            ),
        ));
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), children),
        }
    }
}

impl EntityModel for BannerModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        self.root.child_mut("flag").pose.rotation[0] =
            banner_flag_swing_x_rot(instance.render_state.banner_flag_phase);
    }
}
