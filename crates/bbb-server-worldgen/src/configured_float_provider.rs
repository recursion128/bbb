//! `FloatProvider.sample` for every `FloatProviders.CODEC` family.

use bbb_registry_codec::random_source::RandomSource;

use bbb_server_datapack::worldgen::FloatProvider;

use super::{clamped_normal_float_sample, mth_random_between_f32};

pub(super) fn sample_float_provider(
    provider: &FloatProvider,
    random: &mut dyn RandomSource,
) -> f32 {
    match *provider {
        FloatProvider::Constant(value) => value,
        FloatProvider::Uniform {
            min_inclusive,
            max_exclusive,
        } => mth_random_between_f32(random, min_inclusive, max_exclusive),
        FloatProvider::ClampedNormal {
            mean,
            deviation,
            min,
            max,
        } => clamped_normal_float_sample(random, mean, deviation, min, max),
        // The two draws and their order are the codec owner's sampling contract
        // (`TrapezoidFloat.java:35-40`).
        FloatProvider::Trapezoid { min, max, plateau } => {
            let range = max - min;
            let plateau_start = (range - plateau) / 2.0_f32;
            let plateau_end = range - plateau_start;
            min + random.next_float() * plateau_end + random.next_float() * plateau_start
        }
    }
}
