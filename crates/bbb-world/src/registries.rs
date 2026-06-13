use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};

use serde::{Deserialize, Serialize};

use crate::WorldStore;

const VANILLA_BLOCK_STATES_JSON: &str = include_str!("../data/block_states_26_1.json");
static VANILLA_BLOCK_STATES: OnceLock<Arc<Vec<Option<BlockStateInfo>>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySet {
    pub registries: Vec<RegistryPacket>,
    #[serde(skip)]
    pub block_states: BlockStateRegistry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryPacket {
    pub name: String,
    pub raw_payload_len: usize,
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
        self.registries.registries.push(RegistryPacket {
            name: name.into(),
            raw_payload_len,
        });
        self.counters.registries_seen = self.registries.registries.len();
    }

    pub fn registries(&self) -> &RegistrySet {
        &self.registries
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
}
