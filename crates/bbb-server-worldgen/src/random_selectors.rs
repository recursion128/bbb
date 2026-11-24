//! Vanilla selector features that pick one nested placed feature:
//! `RandomSelectorFeature` (`Feature.java:135-137`),
//! `SimpleRandomSelectorFeature` (`Feature.java:138-140`) and
//! `RandomBooleanSelectorFeature` (`Feature.java:141-143`).

use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_datapack::worldgen::{PlacedFeatureHolder, PlacedFeatureSet, RandomSelectorConfig};

use super::placed_feature::place_placed_feature_holder;
use super::WorldGenLevel;

/// Vanilla `RandomSelectorFeature.place`.
pub fn place_random_selector<L: WorldGenLevel>(
    config: &RandomSelectorConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    for (feature, chance) in &config.features {
        if random.next_float() < *chance {
            return Some(place_placed_feature_holder(feature, level, random, origin));
        }
    }
    Some(place_placed_feature_holder(
        &config.default,
        level,
        random,
        origin,
    ))
}

/// Vanilla `SimpleRandomSelectorFeature.place`;
/// a `#tag` set has no level seam to expand it.
pub fn place_simple_random_selector<L: WorldGenLevel>(
    features: &PlacedFeatureSet,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let PlacedFeatureSet::Direct(features) = features else {
        return None;
    };
    let index = random.next_int(features.len() as i32);
    Some(place_placed_feature_holder(
        &features[index as usize],
        level,
        random,
        origin,
    ))
}

/// Vanilla `RandomBooleanSelectorFeature.place`.
pub fn place_random_boolean_selector<L: WorldGenLevel>(
    feature_true: &PlacedFeatureHolder,
    feature_false: &PlacedFeatureHolder,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let feature = if random.next_boolean() {
        feature_true
    } else {
        feature_false
    };
    Some(place_placed_feature_holder(feature, level, random, origin))
}
