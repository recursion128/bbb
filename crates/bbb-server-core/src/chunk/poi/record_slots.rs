use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PoiRecordSlots {
    keys: BTreeSet<i16>,
}

impl PoiRecordSlots {
    pub(super) fn new() -> Self {
        Self {
            keys: BTreeSet::new(),
        }
    }

    pub(super) fn insert(&mut self, key: i16) {
        self.keys.insert(key);
    }

    pub(super) fn remove(&mut self, key: i16) {
        self.keys.remove(&key);
    }

    pub(super) fn clear_preserving_capacity(&mut self) {
        self.keys.clear();
    }

    pub(super) fn stream_keys(&self) -> impl Iterator<Item = i16> + '_ {
        self.keys.iter().copied()
    }
}

pub(super) fn relative_key([x, y, z]: [i32; 3]) -> i16 {
    // SectionPos.java:92-96.
    (((x & 15) << 8) | ((z & 15) << 4) | (y & 15)) as i16
}

#[cfg(test)]
mod tests;
