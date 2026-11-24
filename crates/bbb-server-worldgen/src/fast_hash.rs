//! Lightweight FxHash aliases for internal worldgen lookup tables.

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

const K: u64 = 0x517c_c1b7_2722_0a95;

#[derive(Default)]
pub(crate) struct FastHasher {
    hash: u64,
}

impl FastHasher {
    #[inline]
    fn add_to_hash(&mut self, value: u64) {
        self.hash = (self.hash.rotate_left(5) ^ value).wrapping_mul(K);
    }
}

impl Hasher for FastHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }

    #[inline]
    fn write(&mut self, mut bytes: &[u8]) {
        while bytes.len() >= 8 {
            let (chunk, rest) = bytes.split_at(8);
            self.add_to_hash(u64::from_ne_bytes(chunk.try_into().unwrap()));
            bytes = rest;
        }
        if bytes.len() >= 4 {
            let (chunk, rest) = bytes.split_at(4);
            self.add_to_hash(u64::from(u32::from_ne_bytes(chunk.try_into().unwrap())));
            bytes = rest;
        }
        if bytes.len() >= 2 {
            let (chunk, rest) = bytes.split_at(2);
            self.add_to_hash(u64::from(u16::from_ne_bytes(chunk.try_into().unwrap())));
            bytes = rest;
        }
        if let Some(&byte) = bytes.first() {
            self.add_to_hash(u64::from(byte));
        }
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.add_to_hash(u64::from(i));
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.add_to_hash(u64::from(i));
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.add_to_hash(u64::from(i));
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.add_to_hash(i);
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add_to_hash(i as u64);
    }
}

pub(crate) type FastBuildHasher = BuildHasherDefault<FastHasher>;
pub(crate) type FastHashMap<K, V> = HashMap<K, V, FastBuildHasher>;
pub(crate) type FastHashSet<K> = HashSet<K, FastBuildHasher>;
