use std::collections::{BTreeMap, HashMap};

use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Serialize,
};

use crate::JavaString;

#[cfg(test)]
mod tests;

/// One property map in source order. Equality remains Map.equals.
#[derive(Debug, Clone, Default)]
pub struct BlockStateProperties(Vec<(JavaString, JavaString)>);

impl BlockStateProperties {
    /// Keeps first-insertion order; a repeated key overwrites the earlier value.
    pub fn from_entries(entries: impl IntoIterator<Item = (JavaString, JavaString)>) -> Self {
        let mut visible: HashMap<JavaString, (usize, JavaString)> = HashMap::new();
        for (key, value) in entries {
            if let Some((_, previous)) = visible.get_mut(&key) {
                *previous = value;
            } else {
                let ordinal = visible.len();
                visible.insert(key, (ordinal, value));
            }
        }
        let mut result = visible
            .into_iter()
            .map(|(key, (ordinal, value))| (ordinal, key, value))
            .collect::<Vec<_>>();
        result.sort_unstable_by_key(|(ordinal, _, _)| *ordinal);
        Self(
            result
                .into_iter()
                .map(|(_, key, value)| (key, value))
                .collect(),
        )
    }

    pub fn insert(&mut self, key: JavaString, value: JavaString) -> Option<JavaString> {
        if let Some((_, old)) = self.0.iter_mut().find(|(candidate, _)| candidate == &key) {
            return Some(std::mem::replace(old, value));
        }
        self.0.push((key, value));
        None
    }

    pub fn get(&self, key: &JavaString) -> Option<&JavaString> {
        self.get_units(key.units())
    }

    pub fn get_units(&self, units: &[u16]) -> Option<&JavaString> {
        self.0
            .iter()
            .find(|(key, _)| key.units() == units)
            .map(|(_, value)| value)
    }

    pub fn get_str(&self, text: &str) -> Option<&JavaString> {
        self.0
            .iter()
            .find(|(key, _)| key == text)
            .map(|(_, value)| value)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&JavaString, &JavaString)> {
        self.0.iter().map(|(key, value)| (key, value))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

impl PartialEq for BlockStateProperties {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len()
            && self
                .iter()
                .all(|(key, value)| other.get(key) == Some(value))
    }
}

impl Eq for BlockStateProperties {}

impl FromIterator<(JavaString, JavaString)> for BlockStateProperties {
    fn from_iter<T: IntoIterator<Item = (JavaString, JavaString)>>(iter: T) -> Self {
        Self::from_entries(iter)
    }
}

impl From<BTreeMap<JavaString, JavaString>> for BlockStateProperties {
    fn from(value: BTreeMap<JavaString, JavaString>) -> Self {
        value.into_iter().collect()
    }
}

impl<const N: usize> From<[(JavaString, JavaString); N]> for BlockStateProperties {
    fn from(value: [(JavaString, JavaString); N]) -> Self {
        Self::from_entries(value)
    }
}

impl IntoIterator for BlockStateProperties {
    type Item = (JavaString, JavaString);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a BlockStateProperties {
    type Item = (&'a JavaString, &'a JavaString);
    type IntoIter = std::iter::Map<
        std::slice::Iter<'a, (JavaString, JavaString)>,
        fn(&(JavaString, JavaString)) -> (&JavaString, &JavaString),
    >;

    fn into_iter(self) -> Self::IntoIter {
        fn pair(value: &(JavaString, JavaString)) -> (&JavaString, &JavaString) {
            (&value.0, &value.1)
        }
        self.0.iter().map(pair)
    }
}

impl Serialize for BlockStateProperties {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        crate::serialize_java_string_map(self.into_iter(), serializer)
    }
}

impl<'de> Deserialize<'de> for BlockStateProperties {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Entries;
        impl<'de> Visitor<'de> for Entries {
            type Value = BlockStateProperties;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a property object")
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry::<JavaString, JavaString>()? {
                    entries.push(entry);
                }
                Ok(BlockStateProperties::from_entries(entries))
            }
        }
        deserializer.deserialize_map(Entries)
    }
}
