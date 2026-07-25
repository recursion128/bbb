use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, OnceLock},
};

use bbb_protocol::{
    block_state_types::{vanilla_block_has_property, VANILLA_BLOCK_STATES_26_1_JSON},
    block_types::{vanilla_block_type, vanilla_block_type_for_resource_id, VANILLA_BLOCK_TYPES},
};
use serde::{Deserialize, Serialize};

static VANILLA_BLOCK_STATES: OnceLock<Arc<Vec<Option<BlockStateInfo>>>> = OnceLock::new();
static VANILLA_AIR_BLOCK_STATE_ID: OnceLock<i32> = OnceLock::new();

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
        Some(vanilla_block_type(id)?.resource_id)
    }

    pub fn block_registry_id_by_name(&self, name: &str) -> Option<i32> {
        Some(vanilla_block_type_for_resource_id(name)?.id)
    }

    pub fn block_entity_type_id_by_state_id(&self, state_id: i32) -> Option<i32> {
        let state = self.by_id(state_id)?;
        vanilla_block_type_for_resource_id(&state.name)?.block_entity_type_id
    }

    pub fn block_has_property(&self, name: &str, property: &str) -> bool {
        vanilla_block_has_property(name, property)
    }

    pub fn block_matches_properties(
        &self,
        name: &str,
        properties: &BTreeMap<String, String>,
    ) -> bool {
        self.states.iter().flatten().any(|state| {
            state.name == name
                && properties.iter().all(|(property, value)| {
                    state
                        .properties
                        .get(property)
                        .is_some_and(|actual| actual == value)
                })
        })
    }

    pub fn block_property_values<'a>(&'a self, name: &str) -> BTreeMap<&'a str, BTreeSet<&'a str>> {
        let mut properties = BTreeMap::<&str, BTreeSet<&str>>::new();
        for state in self
            .states
            .iter()
            .flatten()
            .filter(|state| state.name == name)
        {
            for (property, value) in &state.properties {
                properties
                    .entry(property.as_str())
                    .or_default()
                    .insert(value.as_str());
            }
        }
        properties
    }

    pub(crate) fn maximum_numeric_block_property_value(
        &self,
        name: &str,
        property: &str,
    ) -> Option<i32> {
        self.block_property_values(name)
            .get(property)?
            .iter()
            .map(|value| value.parse::<i32>())
            .collect::<Result<Vec<_>, _>>()
            .ok()?
            .into_iter()
            .max()
    }

    pub(crate) fn compare_block_property_values(
        &self,
        name: &str,
        property: &str,
        left: &str,
        right: &str,
    ) -> Option<Ordering> {
        let mut values = Vec::new();
        for state in self
            .states
            .iter()
            .flatten()
            .filter(|state| state.name == name)
        {
            let value = state.properties.get(property)?;
            if !values.contains(&value.as_str()) {
                values.push(value.as_str());
            }
        }
        if !values.contains(&left) || !values.contains(&right) {
            return None;
        }

        if values.iter().all(|value| value.parse::<i32>().is_ok()) {
            return Some(left.parse::<i32>().ok()?.cmp(&right.parse::<i32>().ok()?));
        }
        if values
            .iter()
            .all(|value| matches!(*value, "false" | "true"))
        {
            return Some((left == "true").cmp(&(right == "true")));
        }

        const DIRECTION_ORDER: &[&str] = &["down", "up", "north", "south", "west", "east"];
        const AXIS_ORDER: &[&str] = &["x", "y", "z"];
        let comparable_order = match property {
            "facing" | "vertical_direction"
                if values.iter().all(|value| DIRECTION_ORDER.contains(value)) =>
            {
                DIRECTION_ORDER
            }
            "axis" if values.iter().all(|value| AXIS_ORDER.contains(value)) => AXIS_ORDER,
            _ => values.as_slice(),
        };
        Some(
            comparable_order
                .iter()
                .position(|value| *value == left)?
                .cmp(&comparable_order.iter().position(|value| *value == right)?),
        )
    }

    pub fn block_registry_len(&self) -> usize {
        VANILLA_BLOCK_TYPES.len()
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

pub fn vanilla_block_state_id(name: &str, properties: &BTreeMap<String, String>) -> Option<i32> {
    BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties(name, properties)
        .map(|state| state.id)
}

pub fn vanilla_air_block_state_id() -> i32 {
    *VANILLA_AIR_BLOCK_STATE_ID.get_or_init(|| {
        vanilla_block_state_id("minecraft:air", &BTreeMap::new())
            .expect("embedded vanilla block-state registry contains air")
    })
}

fn load_vanilla_block_states() -> Vec<Option<BlockStateInfo>> {
    let report: BlockStateReport = serde_json::from_str(VANILLA_BLOCK_STATES_26_1_JSON)
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
    let state_registry_names = load_vanilla_block_registry_names(&states);
    let protocol_registry_names = VANILLA_BLOCK_TYPES
        .iter()
        .map(|entry| entry.resource_id)
        .collect::<Vec<_>>();
    assert_eq!(
        state_registry_names, protocol_registry_names,
        "embedded block-state registry order must match generated protocol block registry"
    );
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
