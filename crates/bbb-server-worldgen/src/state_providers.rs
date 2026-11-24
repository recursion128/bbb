//! `BlockStateProvider.getState` sampling, drawing from the placement random
//! source in Vanilla's order.

use crate::random::WorldgenRandom;
use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::random_source::RandomSource;

pub use bbb_server_datapack::worldgen::{BlockStateProvider, NoiseParams, WeightedList};

use super::block_predicates::block_predicate_test;
use super::noise::NormalNoise;
use super::value_providers::IntProviderExt;
use super::{block_has_property, block_set_property, WorldGenLevel};

/// The provider noise, seeded once from the provider seed; both dual-noise
/// samplers use that seed with independent sources.
fn sample_noise(params: &NoiseParams, pos: BlockPos) -> f64 {
    let noise = params.noise.get_or_init(|| {
        let mut random = WorldgenRandom::with_seed(params.seed);
        NormalNoise::create(&mut random, params.first_octave, &params.amplitudes)
            .expect("decoded noise parameters have an octave")
    });
    let scale = f64::from(params.scale);
    noise.get_value(
        f64::from(pos.x) * scale,
        f64::from(pos.y) * scale,
        f64::from(pos.z) * scale,
    )
}

pub trait BlockStateProviderExt {
    fn get_state<L: WorldGenLevel>(
        &self,
        level: &L,
        random: &mut dyn RandomSource,
        pos: BlockPos,
    ) -> Option<i32>;

    fn get_optional_state<L: WorldGenLevel>(
        &self,
        level: &L,
        random: &mut dyn RandomSource,
        pos: BlockPos,
    ) -> Option<Option<i32>>;
}

impl BlockStateProviderExt for BlockStateProvider {
    /// `BlockStateProvider.getState(level, random, pos)`.
    fn get_state<L: WorldGenLevel>(
        &self,
        level: &L,
        random: &mut dyn RandomSource,
        pos: BlockPos,
    ) -> Option<i32> {
        match self {
            // `SimpleStateProvider.getState` (`SimpleStateProvider.java:23-25`).
            Self::Simple(state) => Some(*state),
            // `WeightedStateProvider.getState`
            // (`WeightedStateProvider.java:35-37`).
            Self::Weighted(list) => Some(*list.get_random(random)),
            // `NoiseThresholdProvider.getState`
            // (`NoiseThresholdProvider.java:59-65`).
            Self::NoiseThreshold {
                noise,
                threshold,
                high_chance,
                default_state,
                low_states,
                high_states,
            } => {
                if sample_noise(noise, pos) < f64::from(*threshold) {
                    random_state(low_states, random)
                } else if random.next_float() < *high_chance {
                    random_state(high_states, random)
                } else {
                    Some(*default_state)
                }
            }
            // `NoiseProvider.getState` (`NoiseProvider.java:38-49`).
            Self::Noise { noise, states } => state_for_noise(states, sample_noise(noise, pos)),
            // `DualNoiseProvider.getState` (`DualNoiseProvider.java:56-69`).
            Self::DualNoise {
                variety,
                slow_noise,
                noise,
                states,
            } => {
                let local_variety = clamped_map(
                    sample_noise(slow_noise, pos),
                    -1.0,
                    1.0,
                    f64::from(variety.0),
                    f64::from(variety.1 + 1),
                ) as i32;
                let mut possible_states = Vec::with_capacity(local_variety as usize);
                for index in 0..local_variety {
                    let offset = BlockPos {
                        x: pos.x.wrapping_add(index * 54_545),
                        y: pos.y,
                        z: pos.z.wrapping_add(index * 34_234),
                    };
                    possible_states
                        .push(state_for_noise(states, sample_noise(slow_noise, offset))?);
                }
                state_for_noise(&possible_states, sample_noise(noise, pos))
            }
            // `RandomizedIntStateProvider.getState`
            // (`RandomizedIntStateProvider.java:58-70`): a state without the
            // named integer property is returned untouched and draws nothing
            // (`findProperty`, `RandomizedIntStateProvider.java:72-80`).
            Self::RandomizedInt {
                source,
                property,
                values,
            } => {
                let unmodified = source.get_state(level, random, pos)?;
                if !state_has_integer_property(unmodified, property) {
                    return Some(unmodified);
                }
                let value = values.sample(random);
                Some(block_set_property(unmodified, property, &value.to_string()))
            }
            // `RotatedBlockProvider.getState` (`RotatedBlockProvider.java:30-33`):
            // `Direction.Axis.getRandom` (`Direction.java:519-521`) over
            // `Axis.VALUES` (`Direction.java:482`), then `trySetValue(AXIS)`.
            Self::Rotated(default_state) => {
                let axis = ["x", "y", "z"][random.next_int(3) as usize];
                Some(try_set_property(*default_state, "axis", axis))
            }
            // `RuleBasedStateProvider.getState`
            // (`RuleBasedStateProvider.java:51-54`).
            Self::RuleBased { .. } => Some(
                self.get_optional_state(level, random, pos)?
                    .unwrap_or_else(|| level.get_block(pos)),
            ),
        }
    }

    /// `BlockStateProvider.getOptionalState` (`BlockStateProvider.java:29-31`)
    /// is `getState` for every family but `RuleBasedStateProvider`
    /// (`RuleBasedStateProvider.java:57-65`).
    fn get_optional_state<L: WorldGenLevel>(
        &self,
        level: &L,
        random: &mut dyn RandomSource,
        pos: BlockPos,
    ) -> Option<Option<i32>> {
        let Self::RuleBased { fallback, rules } = self else {
            return self.get_state(level, random, pos).map(Some);
        };
        for (if_true, then) in rules {
            if block_predicate_test(if_true, level, pos)? {
                return then.get_state(level, random, pos).map(Some);
            }
        }
        match fallback {
            Some(fallback) => fallback.get_state(level, random, pos).map(Some),
            None => Some(None),
        }
    }
}

/// `RandomizedIntStateProvider.findProperty`
/// (`RandomizedIntStateProvider.java:72-80`): a property of the state with
/// that name that is an `IntegerProperty`. The generated state catalog keeps
/// property values as strings; an integer-valued property is one whose value
/// parses as an int, which is what every `IntegerProperty` writes.
fn state_has_integer_property(state: i32, property: &str) -> bool {
    vanilla_block_state_identity(state).is_some_and(|identity| {
        identity
            .properties
            .get(property)
            .is_some_and(|value| value.parse::<i32>().is_ok())
    })
}

fn random_state(states: &[i32], random: &mut dyn RandomSource) -> Option<i32> {
    if states.is_empty() {
        return None;
    }
    let bound = i32::try_from(states.len()).ok()?;
    states.get(random.next_int(bound) as usize).copied()
}

fn state_for_noise(states: &[i32], noise: f64) -> Option<i32> {
    let placement = ((1.0 + noise) / 2.0).clamp(0.0, 0.9999);
    states
        .get((placement * states.len() as f64) as usize)
        .copied()
}

fn clamped_map(value: f64, from_min: f64, from_max: f64, to_min: f64, to_max: f64) -> f64 {
    // `Mth.clampedMap(double)` (`Mth.java:636-638`).
    let factor = ((value - from_min) / (from_max - from_min)).clamp(0.0, 1.0);
    to_min + factor * (to_max - to_min)
}

/// `StateHolder.trySetValue` (`StateHolder.java:113-116`): set the property
/// when the state carries it, otherwise answer the state unchanged.
pub fn try_set_property(state: i32, name: &str, value: &str) -> i32 {
    if block_has_property(state, name) {
        block_set_property(state, name, value)
    } else {
        state
    }
}
