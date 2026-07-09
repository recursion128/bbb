use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};

use serde::{Deserialize, Serialize};

const VANILLA_BLOCK_STATES_JSON: &str = include_str!("../../data/block_states_26_1.json");
static VANILLA_BLOCK_STATES: OnceLock<Arc<Vec<Option<BlockStateInfo>>>> = OnceLock::new();
static VANILLA_BLOCK_REGISTRY_NAMES: OnceLock<Arc<Vec<String>>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct BlockStateRegistry {
    states: Arc<Vec<Option<BlockStateInfo>>>,
    block_registry_names: Arc<Vec<String>>,
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

impl BlockStateRegistry {
    pub fn vanilla_26_1() -> Self {
        let states = VANILLA_BLOCK_STATES
            .get_or_init(|| Arc::new(load_vanilla_block_states()))
            .clone();
        let block_registry_names = VANILLA_BLOCK_REGISTRY_NAMES
            .get_or_init(|| Arc::new(load_vanilla_block_registry_names(&states)))
            .clone();
        Self {
            states,
            block_registry_names,
        }
    }

    pub fn by_id(&self, id: i32) -> Option<&BlockStateInfo> {
        let id = usize::try_from(id).ok()?;
        self.states.get(id)?.as_ref()
    }

    pub fn find_by_name_and_properties(
        &self,
        name: &str,
        properties: &BTreeMap<String, String>,
    ) -> Option<&BlockStateInfo> {
        self.states
            .iter()
            .flatten()
            .find(|state| state.name == name && state.properties == *properties)
    }

    pub fn iter(&self) -> impl Iterator<Item = &BlockStateInfo> {
        self.states.iter().flatten()
    }

    pub fn block_name_by_registry_id(&self, id: i32) -> Option<&str> {
        let id = usize::try_from(id).ok()?;
        self.block_registry_names.get(id).map(String::as_str)
    }

    pub fn block_registry_len(&self) -> usize {
        self.block_registry_names.len()
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

fn load_vanilla_block_registry_names(states: &[Option<BlockStateInfo>]) -> Vec<String> {
    let mut names = Vec::new();
    for state in states.iter().flatten() {
        if names.last() != Some(&state.name) {
            names.push(state.name.clone());
        }
    }
    names
}
