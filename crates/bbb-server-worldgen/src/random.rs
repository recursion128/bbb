//! The worldgen random source: wyrand draws, with every stream seeded by
//! hashing the world seed with the values that identify it (position, step,
//! index, salt, name). Streams are independent of call order, so the same
//! inputs always generate the same world.

use bbb_registry_codec::random_source::RandomSource;

const WY_INCREMENT: u64 = 0xa076_1d64_78bd_642f;
const WY_MIX: u64 = 0xe703_7ed1_a0b4_28db;

/// SplitMix64's finalizer: a bijective avalanche of one word.
const fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Hashes `parts` into one seed; each part is absorbed through `mix64`.
pub fn hash_seed(parts: &[i64]) -> i64 {
    parts.iter().fold(0x9e37_79b9_7f4a_7c15_u64, |hash, part| {
        mix64(hash ^ (*part as u64)).wrapping_add(WY_INCREMENT)
    }) as i64
}

/// FNV-1a over the UTF-8 bytes of a name, for name-keyed streams.
pub fn name_seed(name: &str) -> i64 {
    name.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    }) as i64
}

/// A seed for block-position streams that do not depend on the world seed
/// (structure template palettes and processors).
pub fn position_seed(x: i32, y: i32, z: i32) -> i64 {
    hash_seed(&[i64::from(x), i64::from(y), i64::from(z)])
}

/// The per-chunk stream of a structure or carver (Vanilla
/// `setLargeFeatureSeed`), keyed by the seed and the chunk.
pub fn large_feature_random(seed: i64, chunk_x: i32, chunk_z: i32) -> WorldgenRandom {
    WorldgenRandom::from_parts(&[seed, i64::from(chunk_x), i64::from(chunk_z)])
}

/// Streams at block positions, keyed by the world seed and a name.
#[derive(Clone, Copy, Debug)]
pub struct PositionalRandom {
    seed: i64,
}

impl PositionalRandom {
    pub fn new(world_seed: i64, name: &str) -> Self {
        Self {
            seed: hash_seed(&[world_seed, name_seed(name)]),
        }
    }

    pub fn at(self, x: i32, y: i32, z: i32) -> WorldgenRandom {
        WorldgenRandom::at(self.seed, x, y, z)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldgenRandom {
    state: u64,
}

impl WorldgenRandom {
    pub const fn with_seed(seed: i64) -> Self {
        Self {
            state: mix64(seed as u64),
        }
    }

    /// A stream keyed by the world seed and identifying values.
    pub fn from_parts(parts: &[i64]) -> Self {
        Self::with_seed(hash_seed(parts))
    }

    /// A stream keyed by the world seed and a name.
    pub fn named(seed: i64, name: &str) -> Self {
        Self::from_parts(&[seed, name_seed(name)])
    }

    /// A stream keyed by the world seed and a block position.
    pub fn at(seed: i64, x: i32, y: i32, z: i32) -> Self {
        Self::from_parts(&[seed, i64::from(x), i64::from(y), i64::from(z)])
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(WY_INCREMENT);
        let product = u128::from(self.state) * u128::from(self.state ^ WY_MIX);
        ((product >> 64) as u64) ^ (product as u64)
    }

    /// The top `bits` bits of one draw.
    pub fn next_bits(&mut self, bits: u32) -> u32 {
        (self.next_u64() >> (64 - bits)) as u32
    }

    /// A uniform value in `0..bound` (Lemire's multiply-shift).
    pub fn next_int(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        ((u64::from(self.next_bits(32)) * bound as u64) >> 32) as i32
    }

    pub fn next_long(&mut self) -> i64 {
        self.next_u64() as i64
    }

    pub fn next_boolean(&mut self) -> bool {
        self.next_u64() >> 63 != 0
    }

    pub fn next_float(&mut self) -> f32 {
        self.next_bits(24) as f32 * (1.0 / (1_u32 << 24) as f32)
    }

    pub fn next_double(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1_u64 << 53) as f64)
    }

    /// A standard normal draw (Box-Muller, one value per call).
    pub fn next_gaussian(&mut self) -> f64 {
        let radius = (-2.0 * (1.0 - self.next_double()).ln()).sqrt();
        radius * (std::f64::consts::TAU * self.next_double()).cos()
    }

    pub fn next_int_between_inclusive(&mut self, min: i32, max: i32) -> i32 {
        self.next_int(max - min + 1) + min
    }

    /// A child stream seeded from this one.
    pub fn fork(&mut self) -> Self {
        Self::with_seed(self.next_long())
    }
}

impl RandomSource for WorldgenRandom {
    fn next_int(&mut self, bound: i32) -> i32 {
        Self::next_int(self, bound)
    }

    fn next_long(&mut self) -> i64 {
        Self::next_long(self)
    }

    fn next_boolean(&mut self) -> bool {
        Self::next_boolean(self)
    }

    fn next_float(&mut self) -> f32 {
        Self::next_float(self)
    }

    fn next_double(&mut self) -> f64 {
        Self::next_double(self)
    }

    fn next_gaussian(&mut self) -> f64 {
        Self::next_gaussian(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_stay_in_range_and_cover_it() {
        let mut random = WorldgenRandom::with_seed(1);
        let mut seen = [false; 7];
        for _ in 0..10_000 {
            let value = random.next_int(7);
            seen[value as usize] = true;
            let float = random.next_float();
            assert!((0.0..1.0).contains(&float));
            let double = random.next_double();
            assert!((0.0..1.0).contains(&double));
            assert!(random.next_gaussian().is_finite());
        }
        assert!(seen.iter().all(|seen| *seen));
    }

    #[test]
    fn keyed_streams_are_stable_and_distinct() {
        assert_eq!(
            WorldgenRandom::at(42, 1, 2, 3).next_u64(),
            WorldgenRandom::at(42, 1, 2, 3).next_u64()
        );
        assert_ne!(
            WorldgenRandom::at(42, 1, 2, 3).next_u64(),
            WorldgenRandom::at(42, 3, 2, 1).next_u64()
        );
        assert_ne!(
            WorldgenRandom::named(42, "a").next_u64(),
            WorldgenRandom::named(43, "a").next_u64()
        );
        assert_ne!(hash_seed(&[0, 1]), hash_seed(&[1, 0]));
    }
}
