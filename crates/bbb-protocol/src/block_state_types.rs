//! Fixed Vanilla 26.1 block-state registry wire identities.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

use serde::Deserialize;

/// The checked-in Vanilla 26.1 global block-state registry report.
///
/// Protocol decode needs only the contiguous id range. World and pack
/// consumers reuse the same report for names, properties, and model coverage
/// instead of embedding a second registry snapshot.
pub const VANILLA_BLOCK_STATES_26_1_JSON: &str = include_str!("../data/block_states_26_1.json");

/// `Block.BLOCK_STATE_REGISTRY.size()` for Vanilla 26.1.
pub const VANILLA_BLOCK_STATE_COUNT: i32 = 29_873;

static VANILLA_BLOCK_PROPERTIES: OnceLock<BTreeMap<String, BTreeSet<String>>> = OnceLock::new();

pub fn vanilla_block_state_id(id: i32) -> Option<i32> {
    (0..VANILLA_BLOCK_STATE_COUNT).contains(&id).then_some(id)
}

/// Whether the built-in block's 26.1 `StateDefinition` contains `property`.
///
/// The checked-in global-state report is the protocol-owned source for both
/// wire ids and property names. Consumers must still resolve `block` through
/// the generated block-type catalog when they need to distinguish an unknown
/// block from a known block without properties.
pub fn vanilla_block_has_property(block: &str, property: &str) -> bool {
    vanilla_block_properties()
        .get(block)
        .is_some_and(|properties| properties.contains(property))
}

fn vanilla_block_properties() -> &'static BTreeMap<String, BTreeSet<String>> {
    VANILLA_BLOCK_PROPERTIES.get_or_init(|| {
        let report: BlockStateReport = serde_json::from_str(VANILLA_BLOCK_STATES_26_1_JSON)
            .expect("embedded vanilla 26.1 block state registry is valid JSON");
        assert_eq!(
            report.version,
            crate::MC_VERSION,
            "embedded block state registry version must match protocol target"
        );
        let mut blocks = BTreeMap::<String, BTreeSet<String>>::new();
        for (expected_id, state) in report.states.into_iter().enumerate() {
            assert_eq!(
                state.id, expected_id as i32,
                "embedded block state registry must be contiguous"
            );
            let properties = blocks.entry(state.name).or_default();
            properties.extend(state.properties.into_keys());
        }
        blocks
    })
}

#[derive(Debug, Deserialize)]
struct BlockStateReport {
    version: String,
    states: Vec<BlockStateEntry>,
}

#[derive(Debug, Deserialize)]
struct BlockStateEntry {
    id: i32,
    name: String,
    #[serde(default)]
    properties: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_state_registry_report_is_contiguous_and_matches_wire_range() {
        let report: BlockStateReport =
            serde_json::from_str(VANILLA_BLOCK_STATES_26_1_JSON).unwrap();
        assert_eq!(report.version, crate::MC_VERSION);
        assert_eq!(report.states.len(), VANILLA_BLOCK_STATE_COUNT as usize);
        for (id, state) in report.states.iter().enumerate() {
            assert_eq!(state.id, id as i32);
        }
        assert_eq!(vanilla_block_state_id(-1), None);
        assert_eq!(vanilla_block_state_id(0), Some(0));
        assert_eq!(
            vanilla_block_state_id(VANILLA_BLOCK_STATE_COUNT - 1),
            Some(VANILLA_BLOCK_STATE_COUNT - 1)
        );
        assert_eq!(vanilla_block_state_id(VANILLA_BLOCK_STATE_COUNT), None);
    }

    #[test]
    fn block_property_catalog_is_derived_from_the_embedded_state_report() {
        assert!(vanilla_block_has_property("minecraft:oak_log", "axis"));
        assert!(vanilla_block_has_property("minecraft:lever", "powered"));
        assert!(!vanilla_block_has_property("minecraft:stone", "axis"));
        assert!(!vanilla_block_has_property("minecraft:missing", "axis"));
        assert_eq!(
            vanilla_block_properties().len(),
            crate::block_types::VANILLA_BLOCK_TYPES.len()
        );
    }
}
