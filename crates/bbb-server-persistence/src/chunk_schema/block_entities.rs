//! The block-entity record list a chunk document carries.

use crate::schema_support::field;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::save::NamedCompound;

/// `chunkData.getList("block_entities").stream().flatMap(ListTag::compoundStream)`
/// (`SerializableChunkData.java:152`); records are checked when promoted.
pub fn parse_block_entity_list(entries: &[NbtSummaryEntry]) -> Vec<Vec<NbtSummaryEntry>> {
    match field(entries, "block_entities") {
        Some(NbtSummaryValue::List(values)) => values
            .iter()
            .filter_map(|value| match value {
                NbtSummaryValue::Compound(entries) => Some(entries.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Replaces a chunk document's `block_entities` list outright, matching
/// `SerializableChunkData.copyOf` (`SerializableChunkData.java:357-363`).
pub fn replace_block_entities(raw: &mut NamedCompound, records: Vec<Vec<NbtSummaryEntry>>) {
    let value = NbtSummaryValue::List(records.into_iter().map(NbtSummaryValue::Compound).collect());
    match raw
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "block_entities")
    {
        Some(entry) => entry.value = value,
        None => raw.entries_mut().push(NbtSummaryEntry {
            name: "block_entities".into(),
            value,
        }),
    }
}
