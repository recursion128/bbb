//! Shared `HeightProvider` representation and sampling.

use bbb_registry_codec::random_source::RandomSource;

use super::{mth_random_between_inclusive, WorldGenLevel};

pub use bbb_server_datapack::worldgen::{HeightProvider, VerticalAnchor};

pub fn sample_height_provider<L: WorldGenLevel>(
    provider: &HeightProvider,
    level: &L,
    random: &mut dyn RandomSource,
) -> Option<i32> {
    sample_height_provider_with_bounds(
        provider,
        level.get_min_gen_y(),
        level.get_gen_depth(),
        random,
    )
    .ok()
}

pub fn sample_height_provider_with_bounds(
    provider: &HeightProvider,
    min_y: i32,
    height: i32,
    random: &mut dyn RandomSource,
) -> Result<i32, &'static str> {
    match provider {
        // `ConstantHeight.sample` (`ConstantHeight.java:26-28`).
        HeightProvider::Constant(anchor) => Ok(resolve_anchor_with_bounds(anchor, min_y, height)),
        // `UniformHeight.sample` (`UniformHeight.java:37-48`).
        HeightProvider::Uniform {
            min_inclusive,
            max_inclusive,
        } => {
            let min = resolve_anchor_with_bounds(min_inclusive, min_y, height);
            let max = resolve_anchor_with_bounds(max_inclusive, min_y, height);
            Ok(if min > max {
                min
            } else {
                mth_random_between_inclusive(random, min, max)
            })
        }
        // `BiasedToBottomHeight.sample` (`BiasedToBottomHeight.java:38-46`).
        HeightProvider::BiasedToBottom {
            min_inclusive,
            max_inclusive,
            inner,
        } => {
            let min = resolve_anchor_with_bounds(min_inclusive, min_y, height);
            let max = resolve_anchor_with_bounds(max_inclusive, min_y, height);
            if max - min - inner < 0 {
                Ok(min)
            } else {
                let limit = random.next_int(max - min - inner + 1);
                Ok(random.next_int(limit + inner) + min)
            }
        }
        // `VeryBiasedToBottomHeight.sample` (`VeryBiasedToBottomHeight.java:39-48`).
        HeightProvider::VeryBiasedToBottom {
            min_inclusive,
            max_inclusive,
            inner,
        } => {
            let min = resolve_anchor_with_bounds(min_inclusive, min_y, height);
            let max = resolve_anchor_with_bounds(max_inclusive, min_y, height);
            if max - min - inner < 0 {
                Ok(min)
            } else {
                let upper = mth_next_int(random, min + inner, max);
                let biased_upper = mth_next_int(random, min, upper - 1);
                Ok(mth_next_int(random, min, biased_upper - 1 + inner))
            }
        }
        // `TrapezoidHeight.sample` (`TrapezoidHeight.java:42-57`).
        HeightProvider::Trapezoid {
            min_inclusive,
            max_inclusive,
            plateau,
        } => {
            let min = resolve_anchor_with_bounds(min_inclusive, min_y, height);
            let max = resolve_anchor_with_bounds(max_inclusive, min_y, height);
            if min > max {
                return Ok(min);
            }
            let range = max - min;
            Ok(if *plateau >= range {
                mth_random_between_inclusive(random, min, max)
            } else {
                let plateau_start = (range - plateau) / 2;
                let plateau_end = range - plateau_start;
                min + mth_random_between_inclusive(random, 0, plateau_end)
                    + mth_random_between_inclusive(random, 0, plateau_start)
            })
        }
        // `WeightedListHeight.sample` (`WeightedListHeight.java:20-22`).
        HeightProvider::WeightedList { distribution } => {
            let Some(total_weight) = distribution.iter().try_fold(0_i32, |total, (_, weight)| {
                if *weight < 0 {
                    None
                } else {
                    total.checked_add(*weight)
                }
            }) else {
                return Err("Weighted list has an invalid total weight");
            };
            if total_weight <= 0 {
                return Err("Weighted list must contain at least one entry with non-zero weight");
            }
            let mut selection = random.next_int(total_weight);
            for (entry, weight) in distribution {
                selection -= weight;
                if selection < 0 {
                    return sample_height_provider_with_bounds(entry, min_y, height, random);
                }
            }
            Err("Weighted selection exceeded total weight")
        }
    }
}

/// `VerticalAnchor.resolveY` over `WorldGenerationContext`
/// (`VerticalAnchor.java:48-96`, `WorldGenerationContext.java:10-13`).
fn resolve_anchor_with_bounds(anchor: &VerticalAnchor, min_y: i32, height: i32) -> i32 {
    match *anchor {
        VerticalAnchor::Absolute(y) => y,
        VerticalAnchor::AboveBottom(offset) => min_y + offset,
        VerticalAnchor::BelowTop(offset) => min_y + height - 1 - offset,
    }
}

/// `Mth.nextInt` (`Mth.java:145-147`).
fn mth_next_int(random: &mut dyn RandomSource, min_inclusive: i32, max_inclusive: i32) -> i32 {
    if min_inclusive >= max_inclusive {
        min_inclusive
    } else {
        random.next_int(max_inclusive - min_inclusive + 1) + min_inclusive
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::WorldgenRandom;
    use crate::tests::RecordingLevel;
    use crate::WorldGenLevel;

    #[test]
    fn default_generation_bounds_preserve_level_height_sampling() {
        let level = RecordingLevel::admitting();
        assert_eq!(level.get_min_gen_y(), -64);
        assert_eq!(level.get_gen_depth(), 384);
        let mut random = WorldgenRandom::with_seed(0);
        assert_eq!(
            sample_height_provider(
                &HeightProvider::Constant(VerticalAnchor::BelowTop(0)),
                &level,
                &mut random,
            )
            .unwrap(),
            319
        );
    }
}
