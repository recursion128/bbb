use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

pub use bbb_protocol::block_state_types::VanillaBlockStateIdentity as BlockStateInfo;
use bbb_protocol::{
    block_state_types::{
        vanilla_block_default_state_id, vanilla_block_has_property, vanilla_block_property_kind,
        vanilla_block_property_value_rank, vanilla_block_property_values,
        vanilla_block_state_block_type_id, vanilla_block_state_id_by_name_and_properties,
        vanilla_block_state_identities, vanilla_block_state_identity, vanilla_block_state_name,
        vanilla_block_state_properties, VANILLA_BLOCK_STATE_COUNT,
    },
    block_types::{vanilla_block_type, vanilla_block_type_for_resource_id, VANILLA_BLOCK_TYPES},
    content_registry::ContentRegistry,
};

use crate::terrain::{block_name_has_animate_tick, classify_terrain_material, terrain_fluid_state};
use crate::{TerrainFluidState, TerrainMaterialClass};

/// Client queries over the shared built-in 26.1 block-state identities.
#[derive(Debug, Clone, Copy, Default)]
pub struct BlockStateRegistry;

/// Per-state facts the client derives from names once instead of per probe.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BlockStateTerrainFacts {
    pub(crate) material: TerrainMaterialClass,
    pub(crate) fluid: Option<TerrainFluidState>,
    pub(crate) has_animate_tick: bool,
}

static VANILLA_BLOCK_STATE_TERRAIN: OnceLock<Vec<BlockStateTerrainFacts>> = OnceLock::new();

pub(crate) fn vanilla_block_state_terrain_facts(id: i32) -> Option<BlockStateTerrainFacts> {
    VANILLA_BLOCK_STATE_TERRAIN
        .get_or_init(|| {
            let states = vanilla_block_state_identities();
            let mut facts = Vec::with_capacity(states.len());
            for block_states in states.chunk_by(|left, right| left.name == right.name) {
                let name = block_states[0].name.as_str();
                let has_animate_tick = block_name_has_animate_tick(name);
                let material = classify_terrain_material(Some(name));
                facts.extend(block_states.iter().map(|state| BlockStateTerrainFacts {
                    material,
                    fluid: terrain_fluid_state(Some(name), &state.properties),
                    has_animate_tick,
                }));
            }
            facts
        })
        .get(usize::try_from(id).ok()?)
        .copied()
}

impl BlockStateRegistry {
    pub fn vanilla_26_1() -> Self {
        let _ = vanilla_block_state_terrain_facts(0);
        Self
    }

    pub fn by_id(&self, id: i32) -> Option<&'static BlockStateInfo> {
        vanilla_block_state_identity(id)
    }

    pub fn is_valid_state_id_with_registry(
        &self,
        id: i32,
        content_registry: &ContentRegistry,
    ) -> bool {
        vanilla_block_state_name(id).is_some() || content_registry.block_by_state_id(id).is_some()
    }

    pub fn block_name_by_state_id_with_registry<'a>(
        &self,
        id: i32,
        content_registry: &'a ContentRegistry,
    ) -> Option<&'a str> {
        if let Some(name) = vanilla_block_state_name(id) {
            return Some(name);
        }
        content_registry
            .block_by_state_id(id)
            .map(|(spec, _)| spec.resource_id())
    }

    pub fn block_name_by_registry_id_with_registry<'a>(
        &self,
        id: i32,
        content_registry: &'a ContentRegistry,
    ) -> Option<&'a str> {
        if let Some(name) = self.block_name_by_registry_id(id) {
            return Some(name);
        }
        content_registry
            .block_by_id(id)
            .map(|spec| spec.resource_id())
    }

    pub fn find_by_name_and_properties(
        &self,
        name: &str,
        properties: &BTreeMap<String, String>,
    ) -> Option<&'static BlockStateInfo> {
        self.by_id(vanilla_block_state_id_by_name_and_properties(
            name, properties,
        )?)
    }

    pub fn iter(&self) -> impl Iterator<Item = &'static BlockStateInfo> {
        vanilla_block_state_identities().iter()
    }

    pub fn block_name_by_registry_id(&self, id: i32) -> Option<&'static str> {
        Some(vanilla_block_type(id)?.resource_id)
    }

    pub fn block_registry_id_by_name(&self, name: &str) -> Option<i32> {
        Some(vanilla_block_type_for_resource_id(name)?.id)
    }

    pub fn block_entity_type_id_by_state_id(&self, state_id: i32) -> Option<i32> {
        vanilla_block_type(vanilla_block_state_block_type_id(state_id)?)?.block_entity_type_id
    }

    pub fn block_has_property(&self, name: &str, property: &str) -> bool {
        vanilla_block_has_property(name, property)
    }

    pub fn block_matches_properties(
        &self,
        name: &str,
        properties: &BTreeMap<String, String>,
    ) -> bool {
        vanilla_block_type_for_resource_id(name).is_some_and(|_| {
            properties.iter().all(|(property, value)| {
                vanilla_block_property_values(name, property)
                    .is_some_and(|values| values.contains(&value.as_str()))
            })
        })
    }

    pub fn block_property_values(
        &self,
        name: &str,
    ) -> BTreeMap<&'static str, BTreeSet<&'static str>> {
        let mut properties = BTreeMap::<&str, BTreeSet<&str>>::new();
        let Some(block) = vanilla_block_type_for_resource_id(name) else {
            return properties;
        };
        let state = vanilla_block_default_state_id(block.id)
            .expect("every built-in block has a default state");
        for (property, _) in vanilla_block_state_properties(state) {
            let values = properties.entry(property).or_default();
            for value in vanilla_block_property_values(name, property)
                .expect("a canonical state property has a value domain")
            {
                values.insert(*value);
            }
        }
        properties
    }

    pub fn maximum_numeric_block_property_value(&self, name: &str, property: &str) -> Option<i32> {
        let (first, rest) = vanilla_block_property_values(name, property)?.split_first()?;
        let maximum = first.parse::<i32>().ok()?;
        rest.iter().try_fold(maximum, |maximum, value| {
            Some(maximum.max(value.parse::<i32>().ok()?))
        })
    }

    pub fn compare_block_property_values(
        &self,
        name: &str,
        property: &str,
        left: &str,
        right: &str,
    ) -> Option<Ordering> {
        let values = vanilla_block_property_values(name, property)?;
        if !values.contains(&left) || !values.contains(&right) {
            return None;
        }
        let kind = vanilla_block_property_kind(name, property)?;
        Some(
            vanilla_block_property_value_rank(kind, left)?
                .cmp(&vanilla_block_property_value_rank(kind, right)?),
        )
    }

    pub fn block_registry_len(&self) -> usize {
        VANILLA_BLOCK_TYPES.len()
    }

    pub fn len(&self) -> usize {
        VANILLA_BLOCK_STATE_COUNT as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub fn vanilla_block_state_id(name: &str, properties: &BTreeMap<String, String>) -> Option<i32> {
    vanilla_block_state_id_by_name_and_properties(name, properties)
}

pub fn vanilla_air_block_state_id() -> i32 {
    vanilla_block_state_id("minecraft:air", &BTreeMap::new())
        .expect("embedded vanilla block-state registry contains air")
}
