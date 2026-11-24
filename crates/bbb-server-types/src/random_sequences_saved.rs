//! Persistent random-sequence settings and state pairs.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RandomSequencesSaved {
    pub salt: i32,
    pub include_world_seed: bool,
    pub include_sequence_id: bool,
    pub sequences: BTreeMap<String, [i64; 2]>,
}

impl Default for RandomSequencesSaved {
    fn default() -> Self {
        Self {
            salt: 0,
            include_world_seed: true,
            include_sequence_id: true,
            sequences: BTreeMap::new(),
        }
    }
}
