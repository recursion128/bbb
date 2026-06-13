use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};

use serde::{Deserialize, Serialize};

use bbb_protocol::packets::{
    RegistryData, RegistryDataEntry as ProtocolRegistryDataEntry, UpdateTags,
};

use crate::WorldStore;

const VANILLA_BLOCK_STATES_JSON: &str = include_str!("../data/block_states_26_1.json");
static VANILLA_BLOCK_STATES: OnceLock<Arc<Vec<Option<BlockStateInfo>>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySet {
    pub registries: Vec<RegistryPacket>,
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
pub struct RegistryPacketEntry {
    pub id: String,
    #[serde(default)]
    pub has_data: bool,
    #[serde(default)]
    pub raw_data_len: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryTagState {
    pub registry: String,
    pub tags: BTreeMap<String, Vec<i32>>,
}

#[derive(Debug, Clone)]
pub struct BlockStateRegistry {
    states: Arc<Vec<Option<BlockStateInfo>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockStateInfo {
    pub id: i32,
    pub name: String,
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct BlockStateReport {
    version: String,
    states: Vec<BlockStateInfo>,
}

impl RegistrySet {
    pub fn vanilla_26_1() -> Self {
        Self {
            registries: Vec::new(),
            tags: BTreeMap::new(),
            block_states: BlockStateRegistry::vanilla_26_1(),
        }
    }

    pub fn block_state(&self, id: i32) -> Option<&BlockStateInfo> {
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
    pub fn with_data_len(id: impl Into<String>, raw_data_len: usize) -> Self {
        Self {
            id: id.into(),
            has_data: true,
            raw_data_len,
        }
    }

    pub fn stub(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            has_data: false,
            raw_data_len: 0,
        }
    }
}

impl From<ProtocolRegistryDataEntry> for RegistryPacketEntry {
    fn from(entry: ProtocolRegistryDataEntry) -> Self {
        let has_data = entry.has_data();
        let raw_data_len = entry.raw_data_len();
        Self {
            id: entry.id,
            has_data,
            raw_data_len,
        }
    }
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

impl BlockStateRegistry {
    pub fn vanilla_26_1() -> Self {
        let states = VANILLA_BLOCK_STATES
            .get_or_init(|| Arc::new(load_vanilla_block_states()))
            .clone();
        Self { states }
    }

    pub fn by_id(&self, id: i32) -> Option<&BlockStateInfo> {
        let id = usize::try_from(id).ok()?;
        self.states.get(id)?.as_ref()
    }

    pub fn len(&self) -> usize {
        self.states.iter().filter(|state| state.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for BlockStateRegistry {
    fn default() -> Self {
        Self::vanilla_26_1()
    }
}

impl WorldStore {
    pub fn record_registry(&mut self, name: impl Into<String>, raw_payload_len: usize) {
        self.record_registry_entries(name, raw_payload_len, Vec::new());
    }

    pub fn record_registry_data(&mut self, update: RegistryData) {
        let entries = update
            .entries
            .into_iter()
            .map(RegistryPacketEntry::from)
            .collect();
        self.record_registry_entries(update.registry, update.raw_payload_len, entries);
    }

    pub fn record_registry_entries(
        &mut self,
        name: impl Into<String>,
        raw_payload_len: usize,
        entries: Vec<RegistryPacketEntry>,
    ) {
        self.registries.registries.push(RegistryPacket {
            name: name.into(),
            raw_payload_len,
            entries,
        });
        self.sync_registry_counters();
    }

    pub fn registries(&self) -> &RegistrySet {
        &self.registries
    }

    fn sync_registry_counters(&mut self) {
        self.counters.registries_seen = self.registries.registries.len();
        self.counters.registry_entries_seen = self
            .registries
            .registries
            .iter()
            .map(RegistryPacket::entry_count)
            .sum();
        self.counters.registry_entries_with_data = self
            .registries
            .registries
            .iter()
            .map(RegistryPacket::entries_with_data)
            .sum();
        self.counters.registry_entry_stubs = self
            .registries
            .registries
            .iter()
            .map(RegistryPacket::stub_entries)
            .sum();
    }

    pub fn apply_update_tags(&mut self, update: UpdateTags) {
        self.counters.update_tags_packets += 1;
        self.counters.last_update_tags_registry_count = update.registries.len();
        self.counters.last_update_tags_total_tag_count = update
            .registries
            .iter()
            .map(|registry| registry.tags.len())
            .sum();
        self.counters.last_update_tags_total_value_count = update
            .registries
            .iter()
            .flat_map(|registry| registry.tags.iter())
            .map(|tag| tag.entries.len())
            .sum();

        for registry in update.registries {
            let tags = registry
                .tags
                .into_iter()
                .map(|tag| (tag.tag, tag.entries))
                .collect();
            self.registries.tags.insert(
                registry.registry.clone(),
                RegistryTagState {
                    registry: registry.registry,
                    tags,
                },
            );
        }

        self.counters.tag_registries_tracked = self.registries.tags.len();
        self.counters.tags_tracked = self
            .registries
            .tags
            .values()
            .map(|registry| registry.tags.len())
            .sum();
        self.counters.tag_entries_tracked = self
            .registries
            .tags
            .values()
            .flat_map(|registry| registry.tags.values())
            .map(Vec::len)
            .sum();
    }

    pub fn registry_tags(&self, registry: &str) -> Option<&RegistryTagState> {
        self.registries.tags.get(registry)
    }
}

fn load_vanilla_block_states() -> Vec<Option<BlockStateInfo>> {
    let report: BlockStateReport = serde_json::from_str(VANILLA_BLOCK_STATES_JSON)
        .expect("embedded vanilla 26.1 block state registry is valid JSON");
    assert_eq!(
        report.version, "26.1",
        "embedded block state registry version must match protocol target"
    );

    let max_id = report
        .states
        .iter()
        .map(|state| state.id)
        .max()
        .expect("embedded block state registry is not empty");
    let mut states = vec![None; usize::try_from(max_id).expect("block state id is positive") + 1];
    for state in report.states {
        let index = usize::try_from(state.id).expect("block state id is positive");
        assert!(
            states[index].is_none(),
            "duplicate block state id {}",
            state.id
        );
        states[index] = Some(state);
    }
    states
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_vanilla_block_state_registry() {
        let registries = RegistrySet::vanilla_26_1();
        assert_eq!(registries.block_state_count(), 29873);
        assert_eq!(registries.block_state(0).unwrap().name, "minecraft:air");
        let grass = registries.block_state(9).unwrap();
        assert_eq!(grass.name, "minecraft:grass_block");
        assert_eq!(grass.properties.get("snowy").unwrap(), "false");
    }

    #[test]
    fn registry_data_entries_are_ordered_and_counted() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            "minecraft:chat_type",
            128,
            vec![
                RegistryPacketEntry::with_data_len("minecraft:chat", 24),
                RegistryPacketEntry::stub("minecraft:raw"),
            ],
        );
        store.record_registry_entries(
            "minecraft:damage_type",
            64,
            vec![RegistryPacketEntry::with_data_len("minecraft:in_fire", 17)],
        );

        let registries = &store.registries().registries;
        assert_eq!(registries.len(), 2);
        assert_eq!(registries[0].name, "minecraft:chat_type");
        assert_eq!(
            registries[0].entries,
            vec![
                RegistryPacketEntry::with_data_len("minecraft:chat", 24),
                RegistryPacketEntry::stub("minecraft:raw"),
            ]
        );
        assert_eq!(registries[1].entries[0].id, "minecraft:in_fire");

        let counters = store.counters();
        assert_eq!(counters.registries_seen, 2);
        assert_eq!(counters.registry_entries_seen, 3);
        assert_eq!(counters.registry_entries_with_data, 2);
        assert_eq!(counters.registry_entry_stubs, 1);
    }

    #[test]
    fn update_tags_replace_network_tag_state() {
        let mut store = WorldStore::new();
        store.apply_update_tags(UpdateTags {
            registries: vec![bbb_protocol::packets::RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![
                    bbb_protocol::packets::TagNetworkPayload {
                        tag: "minecraft:logs".to_string(),
                        entries: vec![5, 6, 7],
                    },
                    bbb_protocol::packets::TagNetworkPayload {
                        tag: "minecraft:planks".to_string(),
                        entries: vec![42],
                    },
                ],
            }],
        });

        let item_tags = store
            .registry_tags("minecraft:item")
            .expect("item registry tags tracked");
        assert_eq!(item_tags.tags["minecraft:logs"], vec![5, 6, 7]);
        assert_eq!(item_tags.tags["minecraft:planks"], vec![42]);
        assert_eq!(store.counters().update_tags_packets, 1);
        assert_eq!(store.counters().tag_registries_tracked, 1);
        assert_eq!(store.counters().tags_tracked, 2);
        assert_eq!(store.counters().tag_entries_tracked, 4);

        store.apply_update_tags(UpdateTags {
            registries: vec![bbb_protocol::packets::RegistryTags {
                registry: "minecraft:block".to_string(),
                tags: vec![bbb_protocol::packets::TagNetworkPayload {
                    tag: "minecraft:mineable/pickaxe".to_string(),
                    entries: vec![100, 101],
                }],
            }],
        });

        assert!(store.registry_tags("minecraft:item").is_some());
        assert_eq!(
            store.registry_tags("minecraft:block").unwrap().tags["minecraft:mineable/pickaxe"],
            vec![100, 101]
        );
        assert_eq!(store.counters().update_tags_packets, 2);
        assert_eq!(store.counters().tag_registries_tracked, 2);
        assert_eq!(store.counters().tags_tracked, 3);
        assert_eq!(store.counters().tag_entries_tracked, 6);

        store.apply_update_tags(UpdateTags {
            registries: vec![bbb_protocol::packets::RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![bbb_protocol::packets::TagNetworkPayload {
                    tag: "minecraft:wool".to_string(),
                    entries: vec![200],
                }],
            }],
        });

        let item_tags = store.registry_tags("minecraft:item").unwrap();
        assert!(item_tags.tags.get("minecraft:logs").is_none());
        assert_eq!(item_tags.tags["minecraft:wool"], vec![200]);
        assert!(store.registry_tags("minecraft:block").is_some());
        assert_eq!(store.counters().update_tags_packets, 3);
        assert_eq!(store.counters().tag_registries_tracked, 2);
        assert_eq!(store.counters().tags_tracked, 2);
        assert_eq!(store.counters().tag_entries_tracked, 3);
        assert_eq!(store.counters().last_update_tags_registry_count, 1);
        assert_eq!(store.counters().last_update_tags_total_tag_count, 1);
        assert_eq!(store.counters().last_update_tags_total_value_count, 1);
    }
}
