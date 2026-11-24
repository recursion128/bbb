use std::f32::consts::{PI, TAU};

use crate::entity_models::catalog::EntityModelKind;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

// Vanilla bakes four banner layers (`ModelLayers.STANDING_BANNER` -> `standing_banner#main`,
// `STANDING_BANNER_FLAG` -> `standing_banner#flag`, `WALL_BANNER` -> `wall_banner#main`,
// `WALL_BANNER_FLAG` -> `wall_banner#flag`); bbb keeps one part tree per attachment and splits
// the frame/flag submits over retained-parts layer passes, like the decorated pot.

/// Vanilla `BannerRenderer.submitBanner`'s frame model parts (`BannerModel`): the standing form's
/// `pole` + `bar`, the wall form's `bar` only (`BannerModel.createBodyLayer(standing)`).
pub(in crate::entity_models) const STANDING_BANNER_FRAME_PARTS: &[&str] = &["pole", "bar"];
pub(in crate::entity_models) const WALL_BANNER_FRAME_PARTS: &[&str] = &["bar"];
/// The `BannerFlagModel` flag part the banner-base and every pattern pass re-submit.
pub(in crate::entity_models) const BANNER_FLAG_PARTS: &[&str] = &["flag"];

/// Vanilla `BannerFlagModel.setupAnim(phase)`:
/// `flag.xRot = (-0.0125F + 0.01F * Mth.cos(2π * phase)) * π`.
pub(in crate::entity_models) fn banner_flag_swing_x_rot(phase: f32) -> f32 {
    (-0.0125 + 0.01 * (TAU * phase).cos()) * PI
}

/// Vanilla `BannerRenderer`'s combined part set per attachment: the `BannerModel` frame
/// (`pole`/`bar`; the wall form has no pole) and the `BannerFlagModel` `flag` as one tree (bbb
/// keeps a single model per instance; the frame/flag/pattern texture split rides the
/// retained-parts layer passes). The only animation is the flag swing —
/// `BannerFlagModel.setupAnim` over `BlockEntityRenderState.banner_flag_phase`.
pub(in crate::entity_models) fn banner_model(
    wall: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let (main, flag) = if wall {
        (ModelLayer::WALL_BANNER, ModelLayer::WALL_BANNER_FLAG)
    } else {
        (
            ModelLayer::STANDING_BANNER,
            ModelLayer::STANDING_BANNER_FLAG,
        )
    };
    let mut root = main.bake_with_children_of(flag);
    if matches!(
        instance.kind,
        EntityModelKind::Banner {
            flag_only: true,
            ..
        }
    ) {
        // `GuiBannerResultRenderer` submits the baked flag part directly
        // and never calls `BannerFlagModel.setupAnim`.
        return root;
    }
    root.child_mut("flag").pose.rotation[0] =
        banner_flag_swing_x_rot(instance.block_entity.banner_flag_phase);
    root
}
