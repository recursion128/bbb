const MULTIPLIER: u64 = 25_214_903_917;
const INCREMENT: u64 = 11;
const MASK: u64 = (1_u64 << 48) - 1;

#[derive(Debug, Clone)]
pub(crate) struct LegacyRandom {
    seed: u64,
}

impl LegacyRandom {
    pub(crate) fn new(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ MULTIPLIER) & MASK,
        }
    }

    pub(crate) fn next_i32(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "legacy random bound must be positive");
        if bound & (bound - 1) == 0 {
            return (((bound as i64) * (self.next_bits(31) as i64)) >> 31) as i32;
        }

        loop {
            let sample = self.next_bits(31) as i32;
            let modulo = sample % bound;
            if (sample as i64) - (modulo as i64) + ((bound - 1) as i64) >= 0 {
                return modulo;
            }
        }
    }

    fn next_bits(&mut self, bits: u32) -> u32 {
        self.seed = self.seed.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT) & MASK;
        (self.seed >> (48 - bits)) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_random_matches_java_weight_selection_shape() {
        let mut random = LegacyRandom::new(0);
        assert_eq!(random.next_i32(4), 2);
        assert_eq!(random.next_i32(4), 3);
    }
}
