//! The gameplay random source: one wyrand stream per level.
//!
//! A clone is another handle to the same state, so the entity store, mob AI
//! and players draw from their level's stream without threading a borrow
//! through every call. The world runs on one thread; the atomic only makes the
//! handle `Send`.

use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::Arc;

use bbb_registry_codec::random_source::RandomSource;

const WY_INCREMENT: u64 = 0xa076_1d64_78bd_642f;
const WY_MIX: u64 = 0xe703_7ed1_a0b4_28db;

#[derive(Clone, Debug)]
pub struct GameRandom(Arc<AtomicU64>);

impl GameRandom {
    /// A fresh stream; equal seeds give equal streams.
    pub fn with_seed(seed: i64) -> Self {
        // SplitMix64's finalizer, so nearby seeds start far apart.
        let mut z = seed as u64;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        Self(Arc::new(AtomicU64::new(z ^ (z >> 31))))
    }

    /// Restarts every handle of this stream as `with_seed(seed)` would.
    #[cfg(test)]
    pub(crate) fn reseed(&self, seed: i64) {
        self.0.store(Self::with_seed(seed).0.load(Relaxed), Relaxed);
    }

    pub fn next_u64(&self) -> u64 {
        let state = self.0.load(Relaxed).wrapping_add(WY_INCREMENT);
        self.0.store(state, Relaxed);
        let product = u128::from(state) * u128::from(state ^ WY_MIX);
        ((product >> 64) as u64) ^ (product as u64)
    }

    /// A uniform value in `0..bound` (Lemire's multiply-shift).
    pub fn next_int(&self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        (((self.next_u64() >> 32) * bound as u64) >> 32) as i32
    }

    /// A uniform value in `min..=max`; `min` when the range is empty.
    pub fn next_int_between_inclusive(&self, min: i32, max: i32) -> i32 {
        if max <= min {
            return min;
        }
        let span = (i64::from(max) - i64::from(min) + 1) as u64;
        (i64::from(min) + (((self.next_u64() >> 32) * span) >> 32) as i64) as i32
    }

    pub fn next_long(&self) -> i64 {
        self.next_u64() as i64
    }

    pub fn next_boolean(&self) -> bool {
        self.next_u64() >> 63 != 0
    }

    /// A uniform value in `[0, 1)`.
    pub fn next_float(&self) -> f32 {
        (self.next_u64() >> 40) as f32 * (1.0 / (1_u32 << 24) as f32)
    }

    /// A uniform value in `[0, 1)`.
    pub fn next_double(&self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1_u64 << 53) as f64)
    }

    /// A standard normal draw (Box-Muller).
    pub fn next_gaussian(&self) -> f64 {
        let radius = (-2.0 * (1.0 - self.next_double()).ln()).sqrt();
        radius * (std::f64::consts::TAU * self.next_double()).cos()
    }

    /// `RandomSource.triangle`: `mean` plus a triangular offset in
    /// `(-spread, spread)`.
    pub fn triangle(&self, mean: f64, spread: f64) -> f64 {
        mean + spread * (self.next_double() - self.next_double())
    }
}

/// For the datapack providers that sample through the shared trait.
impl RandomSource for &GameRandom {
    fn next_int(&mut self, bound: i32) -> i32 {
        GameRandom::next_int(self, bound)
    }

    fn next_long(&mut self) -> i64 {
        GameRandom::next_long(self)
    }

    fn next_boolean(&mut self) -> bool {
        GameRandom::next_boolean(self)
    }

    fn next_float(&mut self) -> f32 {
        GameRandom::next_float(self)
    }

    fn next_double(&mut self) -> f64 {
        GameRandom::next_double(self)
    }

    fn next_gaussian(&mut self) -> f64 {
        GameRandom::next_gaussian(self)
    }
}

#[cfg(test)]
mod tests {
    use super::GameRandom;

    #[test]
    fn draws_stay_in_range_and_cover_it() {
        let random = GameRandom::with_seed(1);
        let mut seen = [false; 7];
        for _ in 0..10_000 {
            seen[random.next_int(7) as usize] = true;
            assert!((0.0..1.0).contains(&random.next_float()));
            assert!((0.0..1.0).contains(&random.next_double()));
            assert!((-3..=3).contains(&random.next_int_between_inclusive(-3, 3)));
            assert!(random.next_gaussian().is_finite());
        }
        assert!(seen.iter().all(|seen| *seen));
    }

    #[test]
    fn shared_handles_advance_one_stream() {
        let random = GameRandom::with_seed(5);
        let shared = random.clone();
        let replay = GameRandom::with_seed(5);
        assert_eq!(random.next_u64(), replay.next_u64());
        assert_eq!(shared.next_u64(), replay.next_u64());
    }
}
