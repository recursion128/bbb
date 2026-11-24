//! `LegacyRandomSource` — the `java.util.Random` linear congruential generator
//! vanilla still seeds most of its gameplay draws from.
//!
//! Both sides of the product need it and both must produce the same stream from
//! the same seed, so the arithmetic lives here once and the callers own only
//! their seeding policy.

/// `BitRandomSource.DOUBLE_MULTIPLIER`
/// (`net/minecraft/world/level/levelgen/BitRandomSource.java:7`), written out
/// as the exact `2^-53` its `float` literal widens to.
const DOUBLE_MULTIPLIER: f64 = 1.110_223_024_625_156_5e-16;

/// The LCG multiplier, increment and 48-bit state mask (`java.util.Random`).
const MULTIPLIER: u64 = 0x5DEECE66D;
const INCREMENT: u64 = 0xB;
const MASK: u64 = (1_u64 << 48) - 1;

/// A `java.util.Random` state.
///
/// The stored `seed` is the scrambled 48-bit state, which is what
/// `java.util.Random`'s constructor produces and what every draw advances, so a
/// serialized value round-trips into the same position in the stream.
///
/// Gaussian draws keep the Marsaglia polar cache Vanilla stores beside the
/// LCG.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegacyRandomSource {
    seed: u64,
    next_next_gaussian: Option<f64>,
}

impl LegacyRandomSource {
    /// `new java.util.Random(seed)`: the initial scramble.
    #[inline]
    pub const fn with_seed(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ MULTIPLIER) & MASK,
            next_next_gaussian: None,
        }
    }

    /// The scrambled state, for a caller that has to persist it.
    pub const fn state(&self) -> u64 {
        self.seed
    }

    /// Restores a state produced by [`LegacyRandomSource::state`].
    pub const fn from_state(seed: u64) -> Self {
        Self {
            seed: seed & MASK,
            next_next_gaussian: None,
        }
    }

    /// `java.util.Random.next(bits)`.
    #[inline]
    pub fn next_bits(&mut self, bits: u32) -> u32 {
        self.seed = self.seed.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT) & MASK;
        (self.seed >> (48 - bits)) as u32
    }

    /// `java.util.Random.nextInt(bound)`: the power-of-two fast path and the
    /// rejection loop for every other bound. Non-positive bounds return `0`
    /// rather than panicking so malformed untrusted ranges cannot crash a tick.
    #[inline]
    pub fn next_int(&mut self, bound: i32) -> i32 {
        if bound <= 0 {
            return 0;
        }
        if (bound & (bound - 1)) == 0 {
            return ((i64::from(bound) * i64::from(self.next_bits(31))) >> 31) as i32;
        }
        loop {
            let sample = self.next_bits(31) as i32;
            let modulo = sample % bound;
            if sample.wrapping_sub(modulo).wrapping_add(bound - 1) >= 0 {
                return modulo;
            }
        }
    }

    /// Inclusive draw in `[min_inclusive, max_inclusive]` using 64-bit span
    /// arithmetic so wide signed ranges never overflow `i32`.
    #[inline]
    pub fn next_int_inclusive(&mut self, min_inclusive: i32, max_inclusive: i32) -> i32 {
        if min_inclusive >= max_inclusive {
            return min_inclusive;
        }
        let span = i64::from(max_inclusive) - i64::from(min_inclusive) + 1;
        if let Ok(bound) = i32::try_from(span) {
            min_inclusive.wrapping_add(self.next_int(bound))
        } else {
            let span_u64 = span as u64;
            let offset = (self.next_long() as u64) % span_u64;
            (i64::from(min_inclusive) + offset as i64) as i32
        }
    }

    /// `java.util.Random.nextFloat()`: one 24-bit draw scaled into `[0, 1)`.
    #[inline]
    pub fn next_float(&mut self) -> f32 {
        self.next_bits(24) as f32 / (1u32 << 24) as f32
    }

    /// `java.util.Random.nextLong()`: two 32-bit draws, the first shifted up.
    #[inline]
    pub fn next_long(&mut self) -> i64 {
        let high = i64::from(self.next_bits(32) as i32);
        let low = i64::from(self.next_bits(32) as i32);
        (high << 32).wrapping_add(low)
    }

    /// `java.util.Random.nextDouble()`, which is
    /// `BitRandomSource.nextDouble()`
    /// (`net/minecraft/world/level/levelgen/`): a
    /// 26-bit draw shifted above a 27-bit one, scaled by `DOUBLE_MULTIPLIER`.
    ///
    /// `DOUBLE_MULTIPLIER` is declared `double DOUBLE_MULTIPLIER = 1.110223E-16F`
    /// so the constant is a `float` literal widened
    /// to `double` before the multiply. The nearest `float` to `1.110223E-16`
    /// is exactly `2^-53`, so the widened value is exactly `2^-53` and the
    /// scaling is the same one `java.util.Random` performs.
    #[inline]
    pub fn next_double(&mut self) -> f64 {
        let high = i64::from(self.next_bits(26));
        let low = i64::from(self.next_bits(27));
        ((high << 27) + low) as f64 * DOUBLE_MULTIPLIER
    }

    /// Vanilla `MarsagliaPolarGaussian.nextGaussian`
    #[inline]
    pub fn next_gaussian(&mut self) -> f64 {
        if let Some(next) = self.next_next_gaussian.take() {
            return next;
        }
        loop {
            let x = 2.0 * self.next_double() - 1.0;
            let y = 2.0 * self.next_double() - 1.0;
            let radius_squared = x * x + y * y;
            if radius_squared < 1.0 && radius_squared != 0.0 {
                let multiplier = (-2.0 * radius_squared.ln() / radius_squared).sqrt();
                self.next_next_gaussian = Some(y * multiplier);
                return x * multiplier;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stream_matches_java_util_random_for_a_known_seed() {
        // `new java.util.Random(0).nextLong()` is the canonical published
        // value for this generator, so it pins the scramble, the two 32-bit
        // draws and their signed recombination at once.
        assert_eq!(
            LegacyRandomSource::with_seed(0).next_long(),
            -4_962_768_465_676_381_896
        );
        // `new java.util.Random(42)` continues into the bounded draws.
        let mut random = LegacyRandomSource::with_seed(42);
        assert_eq!(random.next_int(100), 30);
        assert_eq!(random.next_int(100), 63);
        assert_eq!(random.next_long(), -5_843_495_416_241_995_736);
    }

    #[test]
    fn the_double_draw_matches_java_util_random() {
        // `new java.util.Random(0).nextDouble()` is the canonical published
        // value, so it pins the 26-bit and 27-bit draws and the `2^-53` scale
        // at once; the second draw pins that the two draws advance the state
        // rather than one wide one.
        let mut random = LegacyRandomSource::with_seed(0);
        assert_eq!(random.next_double(), 0.730_967_787_376_657);
        assert_eq!(random.next_double(), 0.240_536_415_671_485_87);
    }

    #[test]
    fn a_saved_state_resumes_the_same_stream() {
        let mut random = LegacyRandomSource::with_seed(-1);
        random.next_long();
        let resumed = LegacyRandomSource::from_state(random.state());
        let mut left = random;
        let mut right = resumed;
        assert_eq!(left.next_long(), right.next_long());
        assert_eq!(left.next_int(40), right.next_int(40));
    }

    #[test]
    fn marsaglia_polar_gaussian_is_seed_stable_and_caches_the_pair() {
        let mut first = LegacyRandomSource::with_seed(0);
        let mut second = LegacyRandomSource::with_seed(0);
        let a = first.next_gaussian();
        let b = first.next_gaussian();
        assert_eq!(a, second.next_gaussian());
        assert_eq!(b, second.next_gaussian());
        assert_ne!(a, b);
        // A third draw must consume new `nextDouble` pairs rather than the
        // cached second sample a second time.
        let mut third = LegacyRandomSource::with_seed(0);
        third.next_gaussian();
        third.next_gaussian();
        assert_eq!(first.next_gaussian(), third.next_gaussian());
    }

    #[test]
    fn non_positive_bound_and_wide_inclusive_ranges_do_not_panic() {
        let mut random = LegacyRandomSource::with_seed(7);
        assert_eq!(random.next_int(0), 0);
        assert_eq!(random.next_int(-10), 0);
        assert_eq!(random.next_int(i32::MIN), 0);
        let draw = random.next_int_inclusive(i32::MIN, 1);
        assert!((i32::MIN..=1).contains(&draw));
        assert_eq!(random.next_int_inclusive(5, 3), 5);
    }
}
