use std::collections::BTreeMap;

use super::{
    property_facts_26_1::vanilla_block_property_kinds, vanilla_block_property_value_rank_utf16,
    vanilla_block_state_block_type_id, VanillaBlockPropertyKind,
};

#[allow(dead_code)]
#[path = "property_strides_26_1.rs"]
mod generated;

#[derive(Clone, Copy)]
struct BlockProperties {
    name: &'static str,
    first: i32,
    offset: usize,
    count: usize,
}

#[derive(Clone, Copy)]
struct PropertyStride {
    name: &'static str,
    stride: i32,
    values: &'static [&'static str],
}

impl BlockProperties {
    fn properties(&self) -> &'static [PropertyStride] {
        &generated::properties()[self.offset..self.offset + self.count]
    }

    fn property(&self, name: &str) -> Option<&'static PropertyStride> {
        self.properties()
            .iter()
            .find(|property| property.name == name)
    }
}

fn block_for_state(state: i32) -> Option<&'static BlockProperties> {
    let owner = vanilla_block_state_block_type_id(state)?;
    generated::blocks().get(owner as usize)
}

/// Returns one canonical property token without materializing the state identity.
pub fn vanilla_block_state_property(state: i32, name: &str) -> Option<&'static str> {
    let block = block_for_state(state)?;
    let property = block.property(name)?;
    let index = ((state - block.first) / property.stride) as usize % property.values.len();
    property.values.get(index).copied()
}

/// Matches a raw property spelling against the value carried by one state.
pub fn vanilla_block_state_property_matches_utf16(state: i32, name: &[u16], value: &[u16]) -> bool {
    let Some(block) = block_for_state(state) else {
        return false;
    };
    let Some((index, property)) = block
        .properties()
        .iter()
        .enumerate()
        .find(|(_, property)| property.name.encode_utf16().eq(name.iter().copied()))
    else {
        return false;
    };
    let carried =
        property.values[((state - block.first) / property.stride) as usize % property.values.len()];
    if carried.encode_utf16().eq(value.iter().copied()) {
        return true;
    }
    let kind = &vanilla_block_property_kinds()[block.offset + index].2;
    matches!(kind, VanillaBlockPropertyKind::Integer { .. })
        && vanilla_block_property_value_rank_utf16(kind, value)
            .is_some_and(|rank| carried.parse::<i32>().ok() == Some(rank))
}

/// Every `(property, value)` token of one state in `StateDefinition` order.
pub fn vanilla_block_state_properties(
    state: i32,
) -> impl Iterator<Item = (&'static str, &'static str)> {
    let block = block_for_state(state);
    block
        .map(BlockProperties::properties)
        .unwrap_or_default()
        .iter()
        .map(move |property| {
            let first = block.map_or(0, |block| block.first);
            let index = ((state - first) / property.stride) as usize % property.values.len();
            (property.name, property.values[index])
        })
}

/// Replaces one canonical property token while preserving all other properties.
pub fn vanilla_block_state_with_property(state: i32, name: &str, value: &str) -> Option<i32> {
    let block = block_for_state(state)?;
    let property = block.property(name)?;
    let new_index = property
        .values
        .iter()
        .position(|candidate| *candidate == value)? as i32;
    let old_index = ((state - block.first) / property.stride) % property.values.len() as i32;
    Some(state + (new_index - old_index) * property.stride)
}

/// Cycles one property in report order while preserving all other properties.
pub fn vanilla_block_state_cycle_property(state: i32, name: &str) -> Option<i32> {
    let block = block_for_state(state)?;
    let property = block.property(name)?;
    let count = property.values.len() as i32;
    let index = ((state - block.first) / property.stride) % count;
    Some(state + (((index + 1) % count) - index) * property.stride)
}

fn block_by_name(name: &str) -> Option<&'static BlockProperties> {
    let block_type = crate::block_types::vanilla_block_type_for_resource_id(name)?;
    generated::blocks()
        .get(block_type.id as usize)
        .filter(|block| block.name == name)
}

pub(super) fn block_has_property(name: &str, property: &str) -> bool {
    block_by_name(name).is_some_and(|block| block.property(property).is_some())
}

pub(super) fn block_property_values(name: &str, property: &str) -> Option<&'static [&'static str]> {
    Some(block_by_name(name)?.property(property)?.values)
}

pub(super) fn block_property_kind(
    name: &str,
    property: &str,
) -> Option<&'static VanillaBlockPropertyKind> {
    let block = block_by_name(name)?;
    let index = block
        .properties()
        .iter()
        .position(|entry| entry.name == property)?;
    Some(&vanilla_block_property_kinds()[block.offset + index].2)
}

pub(super) fn block_property_count(name: &str) -> Option<usize> {
    Some(block_by_name(name)?.count)
}

pub(super) fn state_by_properties(
    name: &str,
    properties: &BTreeMap<String, String>,
) -> Option<i32> {
    let block = block_by_name(name)?;
    if properties.len() != block.count {
        return None;
    }
    first_state_matching_block_properties(block, properties)
}

pub(super) fn first_state_matching_properties(
    name: &str,
    properties: &BTreeMap<String, String>,
) -> Option<i32> {
    let block = block_by_name(name)?;
    if !properties.keys().all(|key| block.property(key).is_some()) {
        return None;
    }
    first_state_matching_block_properties(block, properties)
}

fn first_state_matching_block_properties(
    block: &BlockProperties,
    properties: &BTreeMap<String, String>,
) -> Option<i32> {
    let mut state = block.first;
    for property in block.properties() {
        if let Some(value) = properties.get(property.name) {
            let index = property
                .values
                .iter()
                .position(|candidate| *candidate == value)?;
            state += index as i32 * property.stride;
        }
    }
    Some(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_state_types::test_block_state_report;
    use std::collections::HashMap;

    #[test]
    fn every_property_transition_preserves_report_wire_identity() {
        let report = test_block_state_report();
        let oracle: HashMap<_, _> = report
            .states
            .iter()
            .map(|state| ((state.name.as_str(), state.properties.clone()), state.id))
            .collect();
        for state in &report.states {
            let block = block_for_state(state.id).unwrap();
            assert_eq!(block.name, state.name);
            assert_eq!(
                crate::block_state_types::vanilla_block_property_count(&state.name),
                Some(state.properties.len())
            );
            assert_eq!(
                vanilla_block_state_properties(state.id)
                    .map(|(name, value)| (name.to_owned(), value.to_owned()))
                    .collect::<std::collections::BTreeMap<_, _>>(),
                state.properties
            );
            for (name, value) in &state.properties {
                assert_eq!(
                    vanilla_block_state_property(state.id, name),
                    Some(value.as_str())
                );
                let mut properties = state.properties.clone();
                let values =
                    crate::block_state_types::vanilla_block_property_values(&state.name, name)
                        .unwrap();
                let index = values
                    .iter()
                    .position(|candidate| *candidate == value.as_str())
                    .unwrap();
                properties.insert(name.clone(), values[(index + 1) % values.len()].to_owned());
                assert_eq!(
                    vanilla_block_state_cycle_property(state.id, name),
                    oracle
                        .get(&(state.name.as_str(), properties.clone()))
                        .copied()
                );
                for next in block.property(name).unwrap().values {
                    properties.insert(name.clone(), (*next).to_string());
                    assert_eq!(
                        vanilla_block_state_with_property(state.id, name, next),
                        oracle
                            .get(&(state.name.as_str(), properties.clone()))
                            .copied()
                    );
                }
            }
        }
        assert_eq!(vanilla_block_state_property(-1, "age"), None);
        assert_eq!(vanilla_block_state_with_property(29_873, "age", "0"), None);
        let wheat = generated::blocks()
            .iter()
            .find(|block| block.name == "minecraft:wheat")
            .unwrap();
        assert_eq!(
            vanilla_block_state_with_property(wheat.first, "age", "+0"),
            None
        );
        assert_eq!(
            vanilla_block_state_with_property(wheat.first, "missing", "0"),
            None
        );
    }
}
