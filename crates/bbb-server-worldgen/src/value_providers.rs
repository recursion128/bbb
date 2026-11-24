//! `IntProvider.sample` for every `IntProviders.CODEC` family.

use bbb_registry_codec::random_source::RandomSource;

pub use bbb_server_datapack::worldgen::IntProvider;

use super::{mth_clamp_i32, mth_random_between_inclusive};

/// `IntProvider.sample` as a method on the datapack value.
pub trait IntProviderExt {
    fn sample(&self, random: &mut dyn RandomSource) -> i32;
}

impl IntProviderExt for IntProvider {
    fn sample(&self, random: &mut dyn RandomSource) -> i32 {
        sample_int(self, random)
    }
}

pub fn sample_int(provider: &IntProvider, random: &mut dyn RandomSource) -> i32 {
    match provider {
        // `ConstantInt.sample` (`ConstantInt.java:19-21`).
        IntProvider::Constant(value) => *value,
        // `UniformInt.sample` (`UniformInt.java:28-30`).
        IntProvider::Uniform {
            min_inclusive,
            max_inclusive,
        } => mth_random_between_inclusive(random, *min_inclusive, *max_inclusive),
        // `BiasedToBottomInt.sample` (`BiasedToBottomInt.java:28-30`): the
        // inner bound is drawn first.
        IntProvider::BiasedToBottom {
            min_inclusive,
            max_inclusive,
        } => {
            let inner = random.next_int(max_inclusive - min_inclusive + 1) + 1;
            min_inclusive + random.next_int(inner)
        }
        // `ClampedInt.sample` (`ClampedInt.java:30-32`).
        IntProvider::Clamped {
            source,
            min_inclusive,
            max_inclusive,
        } => mth_clamp_i32(sample_int(source, random), *min_inclusive, *max_inclusive),
        // `WeightedListInt.sample` (`WeightedListInt.java:34-36`) through
        // `WeightedList.getRandomOrThrow` (`WeightedList.java:75-82`): one
        // draw in `[0, totalWeight)` walks the cumulative weights
        // (`WeightedList.java:169-178`, `:196-198`).
        IntProvider::WeightedList(distribution) => {
            let total_weight: i32 = distribution.iter().map(|entry| entry.weight).sum();
            let mut selection = random.next_int(total_weight);
            for entry in distribution {
                selection -= entry.weight;
                if selection < 0 {
                    return sample_int(&entry.provider, random);
                }
            }
            unreachable!("weighted selection stays below the total weight")
        }
        // `ClampedNormalInt.sample` (`ClampedNormalInt.java:35-37`) over
        // `Mth.normal` (`Mth.java:672-674`) and `Mth.clamp(float)`; the
        // `(int)` cast truncates toward zero.
        IntProvider::ClampedNormal {
            mean_bits,
            deviation_bits,
            min_inclusive,
            max_inclusive,
        } => {
            let normal = f32::from_bits(*mean_bits)
                + (random.next_gaussian() as f32) * f32::from_bits(*deviation_bits);
            normal.max(*min_inclusive as f32).min(*max_inclusive as f32) as i32
        }
        // `TrapezoidInt.sample` (`TrapezoidInt.java:40-53`).
        IntProvider::Trapezoid {
            min_inclusive: min,
            max_inclusive: max,
            plateau,
        } => {
            if *plateau == 0 && *max == -*min {
                return random.next_int(max + 1) - random.next_int(max + 1);
            }
            let range = max - min;
            if *plateau == range {
                return mth_random_between_inclusive(random, *min, *max);
            }
            let plateau_start = (range - plateau) / 2;
            let plateau_end = range - plateau_start;
            min + mth_random_between_inclusive(random, 0, plateau_end)
                + mth_random_between_inclusive(random, 0, plateau_start)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::random::WorldgenRandom;
    use bbb_registry_codec::WeightedIntProvider;

    use super::{sample_int, IntProvider};

    #[test]
    fn trapezoid_triangle_draws_two_bounded_ints_and_subtracts() {
        let provider = IntProvider::Trapezoid {
            min_inclusive: -7,
            max_inclusive: 7,
            plateau: 0,
        };
        let mut random = WorldgenRandom::with_seed(1);
        let mut expected = WorldgenRandom::with_seed(1);
        let want = expected.next_int(8) - expected.next_int(8);
        assert_eq!(sample_int(&provider, &mut random), want);
    }

    #[test]
    fn weighted_list_walks_cumulative_weights_after_one_total_draw() {
        let provider = IntProvider::WeightedList(vec![
            WeightedIntProvider {
                provider: IntProvider::Constant(10),
                weight: 2,
            },
            WeightedIntProvider {
                provider: IntProvider::Constant(20),
                weight: 1,
            },
        ]);
        let mut random = WorldgenRandom::with_seed(5);
        let mut expected = WorldgenRandom::with_seed(5);
        let want = if expected.next_int(3) < 2 { 10 } else { 20 };
        assert_eq!(sample_int(&provider, &mut random), want);
    }

    #[test]
    fn nested_clamped_weighted_bounds_follow_provider_extrema() {
        let weighted = IntProvider::WeightedList(vec![
            WeightedIntProvider {
                provider: IntProvider::Constant(-8),
                weight: 1,
            },
            WeightedIntProvider {
                provider: IntProvider::Uniform {
                    min_inclusive: 2,
                    max_inclusive: 14,
                },
                weight: 1,
            },
        ]);
        assert_eq!(weighted.min_inclusive(), -8);
        assert_eq!(weighted.max_inclusive(), 14);

        let clamped = IntProvider::Clamped {
            source: Box::new(weighted),
            min_inclusive: 0,
            max_inclusive: 10,
        };
        assert_eq!(clamped.min_inclusive(), 0);
        assert_eq!(clamped.max_inclusive(), 10);
    }
}
