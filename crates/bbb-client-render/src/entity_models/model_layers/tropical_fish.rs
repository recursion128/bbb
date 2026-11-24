use super::{
    TROPICAL_FISH_BETTY_PATTERN_TEXTURE_REF, TROPICAL_FISH_BLOCKFISH_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_BRINELY_PATTERN_TEXTURE_REF, TROPICAL_FISH_CLAYFISH_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_DASHER_PATTERN_TEXTURE_REF, TROPICAL_FISH_FLOPPER_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_GLITTER_PATTERN_TEXTURE_REF, TROPICAL_FISH_KOB_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_SNOOPER_PATTERN_TEXTURE_REF, TROPICAL_FISH_SPOTTY_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_STRIPEY_PATTERN_TEXTURE_REF, TROPICAL_FISH_SUNSTREAK_PATTERN_TEXTURE_REF,
};
use crate::entity_models::catalog::{TropicalFishModelShape, TropicalFishPattern};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};
use crate::entity_models::EntityModelTextureRef;

/// Vanilla `TropicalFishSmallModel`/`TropicalFishLargeModel.setupAnim`: `tail.yRot =
/// -amplitude * 0.45 * sin(0.6 * ageInTicks)`, with `amplitude = isInWater ? 1.0 : 1.5`.
/// This is identical to `CodModel.setupAnim`'s tail sway, so both share
/// [`super::cod_tail_fin_yrot`]; this alias documents the shared formula at the tropical
/// fish call site.
pub(in crate::entity_models) fn tropical_fish_tail_yrot(age_in_ticks: f32, in_water: bool) -> f32 {
    super::cod_tail_fin_yrot(age_in_ticks, in_water)
}

/// Vanilla `TropicalFishRenderer` selects the small (kob) or large (flopper) body layer by
/// `TropicalFish.Pattern.base()`.
pub(in crate::entity_models) fn tropical_fish_model_layer(
    shape: TropicalFishModelShape,
) -> ModelLayer {
    match shape {
        TropicalFishModelShape::Small => ModelLayer::TROPICAL_FISH_SMALL,
        TropicalFishModelShape::Large => ModelLayer::TROPICAL_FISH_LARGE,
    }
}

/// The pattern overlay model-layer key for a body shape.
pub(in crate::entity_models) fn tropical_fish_pattern_model_layer(
    shape: TropicalFishModelShape,
) -> ModelLayer {
    match shape {
        TropicalFishModelShape::Small => ModelLayer::TROPICAL_FISH_SMALL_PATTERN,
        TropicalFishModelShape::Large => ModelLayer::TROPICAL_FISH_LARGE_PATTERN,
    }
}

/// Vanilla `TropicalFishPatternLayer` texture for a pattern (`tropical_{a,b}_pattern_{1..6}`).
pub(in crate::entity_models) fn tropical_fish_pattern_texture_ref(
    pattern: TropicalFishPattern,
) -> EntityModelTextureRef {
    match pattern {
        TropicalFishPattern::Kob => TROPICAL_FISH_KOB_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Sunstreak => TROPICAL_FISH_SUNSTREAK_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Snooper => TROPICAL_FISH_SNOOPER_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Dasher => TROPICAL_FISH_DASHER_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Brinely => TROPICAL_FISH_BRINELY_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Spotty => TROPICAL_FISH_SPOTTY_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Flopper => TROPICAL_FISH_FLOPPER_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Stripey => TROPICAL_FISH_STRIPEY_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Glitter => TROPICAL_FISH_GLITTER_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Blockfish => TROPICAL_FISH_BLOCKFISH_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Betty => TROPICAL_FISH_BETTY_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Clayfish => TROPICAL_FISH_CLAYFISH_PATTERN_TEXTURE_REF,
    }
}

/// Applies the vanilla `TropicalFish{Small,Large}Model.setupAnim` tail sway to a unified tree: the
/// `tail` child's `yRot` is set to [`tropical_fish_tail_yrot`] (absolute, like vanilla); every other
/// part stays at the bind pose. Shared by the base body and the pattern overlay so the overlay sways
/// with the body. The body shape, swim wiggle, and out-of-water flop live in the root transform.
fn apply_tropical_fish_tail_sway(root: &mut ModelPart, instance: &EntityModelInstance) {
    let in_water = instance.render_state.in_water;
    let tail_yrot = tropical_fish_tail_yrot(instance.render_state.age_in_ticks, in_water);
    root.child_mut("tail").pose.rotation[1] = tail_yrot;
}

/// Mutable tropical fish base-body model, mirroring vanilla `TropicalFishSmallModel` /
/// `TropicalFishLargeModel`. The unified tree is built with named children selected by `shape`;
/// `setup_anim` runs the shared [`apply_tropical_fish_tail_sway`]; the base layer is tinted by the
/// base color (`getModelTint`).
pub(in crate::entity_models) fn tropical_fish_model(
    shape: TropicalFishModelShape,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = tropical_fish_model_layer(shape).bake();
    apply_tropical_fish_tail_sway(&mut root, instance);
    root
}

/// Mutable tropical fish pattern-overlay model, mirroring vanilla `TropicalFishPatternLayer`. The
/// overlay is the body mesh inflated by `FISH_PATTERN_DEFORMATION`; `setup_anim` runs the same
/// [`apply_tropical_fish_tail_sway`] so the pattern sways with the body, tinted by the pattern color.
pub(in crate::entity_models) fn tropical_fish_pattern_model(
    shape: TropicalFishModelShape,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = tropical_fish_pattern_model_layer(shape).bake();
    apply_tropical_fish_tail_sway(&mut root, instance);
    root
}
