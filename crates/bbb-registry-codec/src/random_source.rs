//! The draw interface shared by world generation and gameplay sources.

use crate::legacy_random::LegacyRandomSource;

pub trait RandomSource {
    fn next_int(&mut self, bound: i32) -> i32;
    fn next_long(&mut self) -> i64;
    fn next_boolean(&mut self) -> bool;
    fn next_float(&mut self) -> f32;
    fn next_double(&mut self) -> f64;
    fn next_gaussian(&mut self) -> f64;
}

impl RandomSource for LegacyRandomSource {
    fn next_int(&mut self, bound: i32) -> i32 {
        Self::next_int(self, bound)
    }

    fn next_long(&mut self) -> i64 {
        Self::next_long(self)
    }

    fn next_boolean(&mut self) -> bool {
        self.next_bits(1) != 0
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
