use std::{collections::BTreeMap, sync::Arc};

use bbb_protocol::packets::RegistryDataEntry as ProtocolRegistryDataEntry;
use serde::{Deserialize, Serialize};

use crate::chunks::{decode_nbt_payload_summary, NbtPayloadSummary};

use super::BlockStateRegistry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySet {
    pub registries: Vec<RegistryPacket>,
    #[serde(default)]
    pub contents: BTreeMap<String, RegistryContentState>,
    #[serde(default)]
    pub tags: BTreeMap<String, RegistryTagState>,
    #[serde(skip)]
    pub block_states: BlockStateRegistry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryPacket {
    pub name: String,
    pub raw_payload_len: usize,
    #[serde(default)]
    pub entries: Vec<RegistryPacketEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryContentState {
    pub registry: String,
    pub packet_count: usize,
    #[serde(default)]
    pub entries: Vec<RegistryPacketEntry>,
    #[serde(default)]
    pub duplicate_entry_ids: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryPacketEntry {
    pub id: String,
    #[serde(default)]
    pub has_data: bool,
    #[serde(default)]
    pub raw_data_len: usize,
    #[serde(default)]
    pub nbt: Option<NbtPayloadSummary>,
    #[serde(skip)]
    pub raw_data: Option<Arc<[u8]>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryTagState {
    pub registry: String,
    pub tags: BTreeMap<String, Vec<i32>>,
}

impl RegistrySet {
    pub fn vanilla_26_1() -> Self {
        Self {
            registries: Vec::new(),
            contents: BTreeMap::new(),
            tags: BTreeMap::new(),
            block_states: BlockStateRegistry::vanilla_26_1(),
        }
    }

    pub fn block_state(&self, id: i32) -> Option<&super::BlockStateInfo> {
        self.block_states.by_id(id)
    }

    pub fn block_state_count(&self) -> usize {
        self.block_states.len()
    }
}

impl Default for RegistrySet {
    fn default() -> Self {
        Self::vanilla_26_1()
    }
}

impl RegistryPacketEntry {
    pub fn with_raw_data(id: impl Into<String>, raw_data: Vec<u8>) -> Self {
        let nbt = decode_registry_entry_nbt(raw_data.as_slice());
        Self {
            id: id.into(),
            has_data: true,
            raw_data_len: raw_data.len(),
            nbt,
            raw_data: Some(Arc::from(raw_data)),
        }
    }

    pub fn with_data_len(id: impl Into<String>, raw_data_len: usize) -> Self {
        Self {
            id: id.into(),
            has_data: true,
            raw_data_len,
            nbt: None,
            raw_data: None,
        }
    }

    pub fn stub(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            has_data: false,
            raw_data_len: 0,
            nbt: None,
            raw_data: None,
        }
    }

    pub fn raw_data(&self) -> Option<&[u8]> {
        self.raw_data.as_deref()
    }
}

impl RegistryContentState {
    pub fn new(registry: impl Into<String>) -> Self {
        Self {
            registry: registry.into(),
            packet_count: 0,
            entries: Vec::new(),
            duplicate_entry_ids: BTreeMap::new(),
        }
    }

    pub(super) fn append_packet(&mut self, entries: &[RegistryPacketEntry]) {
        self.packet_count += 1;
        for entry in entries {
            if self.entries.iter().any(|existing| existing.id == entry.id) {
                *self
                    .duplicate_entry_ids
                    .entry(entry.id.clone())
                    .or_insert(0) += 1;
            }
            self.entries.push(entry.clone());
        }
    }

    pub fn duplicate_entry_count(&self) -> usize {
        self.duplicate_entry_ids.values().sum()
    }
}

impl From<ProtocolRegistryDataEntry> for RegistryPacketEntry {
    fn from(entry: ProtocolRegistryDataEntry) -> Self {
        let ProtocolRegistryDataEntry { id, raw_data } = entry;
        let raw_data_len = raw_data.as_ref().map_or(0, Vec::len);
        let nbt = raw_data.as_deref().and_then(decode_registry_entry_nbt);
        Self {
            id,
            has_data: raw_data.is_some(),
            raw_data_len,
            nbt,
            raw_data: raw_data.map(Arc::from),
        }
    }
}

fn decode_registry_entry_nbt(raw_data: &[u8]) -> Option<NbtPayloadSummary> {
    decode_nbt_payload_summary(raw_data).ok().flatten()
}

impl RegistryPacket {
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn entries_with_data(&self) -> usize {
        self.entries.iter().filter(|entry| entry.has_data).count()
    }

    pub fn stub_entries(&self) -> usize {
        self.entries.len() - self.entries_with_data()
    }
}
