//! `RandomSequences` — the per-key `XoroshiroRandomSource` map behind
//! `/random` and `data/minecraft/random_sequences.dat`. Loot draws from the
//! level's gameplay stream instead; this store keeps the saved sequences intact
//! and answers the command.
//!
//! # How a key becomes a generator
//!
//! `RandomSequences.createSequence` (`RandomSequences.java:49-58`) folds the
//! world seed and the instance salt into one legacy seed, and
//! `RandomSequence.createSequence` (`RandomSequence.java:29-36`) turns that
//! into the 128-bit state:
//!
//! 1. `seed = (include_world_seed ? world_seed : 0) ^ salt` — the `int` salt is
//!    sign-extended to `long` by Java's binary numeric promotion.
//! 2. `RandomSupport.upgradeSeedTo128bitUnmixed(seed)` (`RandomSupport.java:23-27`).
//! 3. when `include_sequence_id`, xor in `RandomSequence.seedForKey`
//!    (`RandomSequence.java:38-40`) = the MD5-128 of the key's `toString()`
//!    (`RandomSupport.java:33-38`).
//! 4. `Seed128bit.mixed()` (`RandomSupport.java:44-56`), then
//!    `XoroshiroRandomSource(Seed128bit)` (`XoroshiroRandomSource.java:20-22`).
//!
//! The three settings default to `salt = 0`, both flags true
//! (`RandomSequences.java:29-32`); only `/random reset` moves them.

use std::collections::BTreeMap;

use bbb_server_types::random_sequences_saved::RandomSequencesSaved;

use bbb_registry_codec::xoroshiro_random::{
    seed_from_hash_of, upgrade_seed_to_128bit_unmixed, XoroshiroRandomSource,
};

/// `RandomSequences` (`RandomSequences.java:16-94`) minus the `SavedData`
/// bookkeeping — see the module docs for what is deliberately absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RandomSequences {
    salt: i32,
    include_world_seed: bool,
    include_sequence_id: bool,
    /// Keyed by the `Identifier.toString()` form the map codec writes; a
    /// `BTreeMap` where vanilla has an `Object2ObjectOpenHashMap` because the
    /// saved document has to come out in one order.
    sequences: BTreeMap<String, XoroshiroRandomSource>,
}

impl Default for RandomSequences {
    /// The field initialisers of `RandomSequences` (`RandomSequences.java:29-32`),
    /// which are also what `computeIfAbsent` builds for a save with no file.
    fn default() -> Self {
        Self {
            salt: 0,
            include_world_seed: true,
            include_sequence_id: true,
            sequences: BTreeMap::new(),
        }
    }
}

impl RandomSequences {
    pub fn from_saved(saved: &RandomSequencesSaved) -> Self {
        Self::from_parts(
            saved.salt,
            saved.include_world_seed,
            saved.include_sequence_id,
            saved
                .sequences
                .iter()
                .map(|(key, &[lo, hi])| (key.clone(), XoroshiroRandomSource::from_state(lo, hi)))
                .collect(),
        )
    }

    pub fn saved(&self) -> RandomSequencesSaved {
        RandomSequencesSaved {
            salt: self.salt,
            include_world_seed: self.include_world_seed,
            include_sequence_id: self.include_sequence_id,
            sequences: self
                .entries()
                .map(|(key, (lo, hi))| (key.to_string(), [lo, hi]))
                .collect(),
        }
    }

    /// `new RandomSequences()` (`RandomSequences.java:34-35`).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The private four-argument constructor `RandomSequences.CODEC` parses
    /// into (`RandomSequences.java:17-25`, `:37-42`).
    #[must_use]
    pub fn from_parts(
        salt: i32,
        include_world_seed: bool,
        include_sequence_id: bool,
        sequences: BTreeMap<String, XoroshiroRandomSource>,
    ) -> Self {
        Self {
            salt,
            include_world_seed,
            include_sequence_id,
            sequences,
        }
    }

    /// `RandomSequences.salt()` (`RandomSequences.java:84-86`).
    #[must_use]
    pub fn salt(&self) -> i32 {
        self.salt
    }

    /// `RandomSequences.includeWorldSeed()` (`RandomSequences.java:88-90`).
    #[must_use]
    pub fn include_world_seed(&self) -> bool {
        self.include_world_seed
    }

    /// `RandomSequences.includeSequenceId()` (`RandomSequences.java:92-94`).
    #[must_use]
    pub fn include_sequence_id(&self) -> bool {
        self.include_sequence_id
    }

    /// Every live sequence with the generator state its codec writes
    /// (`RandomSequence.java:12-14`).
    pub fn entries(&self) -> impl Iterator<Item = (&str, (i64, i64))> {
        self.sequences
            .iter()
            .map(|(key, source)| (key.as_str(), source.state()))
    }

    /// `RandomSequences.get(Identifier, long)` (`RandomSequences.java:44-47`):
    /// the sequence for this key, created on first use and then only advanced.
    ///
    /// `world_seed` is what `MinecraftServer.getRandomSequence` passes,
    /// `worldGenSettings.options().seed()` (`MinecraftServer.java:1758-1760`).
    pub fn get(&mut self, key: &str, world_seed: i64) -> &mut XoroshiroRandomSource {
        if !self.sequences.contains_key(key) {
            // `RandomSequences.createSequence` (`RandomSequences.java:49-58`)
            // followed by `RandomSequence.createSequence`
            // (`RandomSequence.java:29-36`).
            let created = create_sequence(
                key,
                world_seed,
                self.salt,
                self.include_world_seed,
                self.include_sequence_id,
            );
            self.sequences.insert(key.to_string(), created);
        }
        self.sequences
            .get_mut(key)
            .expect("the sequence was just created if it was missing")
    }

    /// `RandomSequences.setSeedDefaults` (`RandomSequences.java:64-68`).
    pub fn set_seed_defaults(
        &mut self,
        salt: i32,
        include_world_seed: bool,
        include_sequence_id: bool,
    ) {
        self.salt = salt;
        self.include_world_seed = include_world_seed;
        self.include_sequence_id = include_sequence_id;
    }

    /// `RandomSequences.clear` (`RandomSequences.java:70-74`): what it removed
    /// is what `/random reset *` reports.
    pub fn clear(&mut self) -> usize {
        let count = self.sequences.len();
        self.sequences.clear();
        count
    }

    /// `RandomSequences.reset(Identifier, long)` (`RandomSequences.java:76-78`):
    /// the sequence starts over from the current defaults, live or not.
    pub fn reset(&mut self, key: &str, world_seed: i64) {
        self.reset_with(
            key,
            world_seed,
            self.salt,
            self.include_world_seed,
            self.include_sequence_id,
        );
    }

    /// The five-argument `RandomSequences.reset` (`RandomSequences.java:80-82`),
    /// which seeds from an explicit trio without moving the defaults.
    pub fn reset_with(
        &mut self,
        key: &str,
        world_seed: i64,
        salt: i32,
        include_world_seed: bool,
        include_sequence_id: bool,
    ) {
        let created = create_sequence(
            key,
            world_seed,
            salt,
            include_world_seed,
            include_sequence_id,
        );
        self.sequences.insert(key.to_string(), created);
    }
}

/// The generator one sequence key starts at — see the module docs for the
/// four steps.
#[must_use]
pub fn create_sequence(
    key: &str,
    world_seed: i64,
    salt: i32,
    include_world_seed: bool,
    include_sequence_id: bool,
) -> XoroshiroRandomSource {
    // `(includeWorldSeed ? worldSeed : 0L) ^ salt`: the `int` salt widens to
    // `long` before the xor, so a negative salt sets the whole high half.
    let seed = if include_world_seed { world_seed } else { 0 } ^ i64::from(salt);
    let mut seed_128 = upgrade_seed_to_128bit_unmixed(seed);
    if include_sequence_id {
        let key_seed = seed_from_hash_of(key);
        seed_128 = seed_128.xor(key_seed.seed_lo, key_seed.seed_hi);
    }
    XoroshiroRandomSource::from_seed_128(seed_128.mixed())
}

#[cfg(test)]
mod tests;
