use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use bbb_protocol::block_types::VANILLA_BLOCK_TYPES;
use bbb_protocol::item_types::{
    vanilla_item_type, VanillaItemUseOverride, VANILLA_ITEM_ALTERNATIVE_BLOCKS,
    VANILLA_ITEM_CRAFTING_REMAINDERS, VANILLA_ITEM_SPECIAL_BLOCKS, VANILLA_ITEM_TYPES,
};
use serde::{Deserialize, Serialize};

use crate::{resources::ResourceLocation, PackRoots};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemRegistryCatalog {
    resource_ids: Vec<String>,
    protocol_ids: BTreeMap<String, i32>,
    /// Exact persistent-codec values from the shared Vanilla item prototype catalog.
    #[serde(default)]
    default_component_codec_values: BTreeMap<String, BTreeMap<i32, serde_json::Value>>,
    #[serde(default)]
    block_items: BTreeSet<String>,
    #[serde(default)]
    block_item_resource_ids: BTreeMap<String, String>,
    #[serde(default)]
    primary_block_resource_ids: BTreeMap<String, String>,
    #[serde(default)]
    spawn_egg_items: BTreeSet<String>,
    #[serde(default)]
    crafting_remainders: BTreeMap<String, String>,
}

const ITEM_REGISTRY_CATALOG_OVERRIDE_PATH: &str = ".bbb/item_registry_catalog.json";

impl ItemRegistryCatalog {
    /// Load the checked-in vanilla 26.1 item catalog without reading a local Java source tree.
    pub fn vanilla_26_1() -> Result<Self> {
        Self::from_generated_source_catalog()
    }

    pub fn load(roots: &PackRoots) -> Result<Self> {
        let override_path = roots.sources_dir.join(ITEM_REGISTRY_CATALOG_OVERRIDE_PATH);
        if override_path.is_file() {
            return roots
                .resource_stack()
                .read_json_file(&override_path, "item registry catalog override")
                .with_context(|| {
                    format!(
                        "parse item registry catalog override {}",
                        override_path.display()
                    )
                });
        }
        Self::from_generated_source_catalog()
    }

    pub(crate) fn from_generated_source_catalog() -> Result<Self> {
        let prototypes = bbb_protocol::vanilla_item_prototype_component_catalog()?;
        let mut catalog = Self::default();
        for (item, components) in VANILLA_ITEM_TYPES
            .iter()
            .zip(prototypes.component_codec_values_by_item_id())
        {
            let id = item.resource_id;
            catalog.resource_ids.push(id.to_owned());
            catalog.protocol_ids.insert(id.to_owned(), item.id);
            catalog
                .default_component_codec_values
                .insert(id.to_owned(), components.clone());
            if let Some(behavior) = item.block_item_behavior {
                catalog.block_items.insert(id.to_owned());
                catalog.primary_block_resource_ids.insert(
                    id.to_owned(),
                    block_resource_id(behavior.block_type_id)?.to_owned(),
                );
                for block in
                    std::iter::once(behavior.block_type_id).chain(behavior.wall_block_type_id)
                {
                    catalog
                        .block_item_resource_ids
                        .insert(block_resource_id(block)?.to_owned(), id.to_owned());
                }
            }
            if item.use_override == Some(VanillaItemUseOverride::SpawnEggItem) {
                catalog.spawn_egg_items.insert(id.to_owned());
            }
        }
        for &(item, remainder) in VANILLA_ITEM_CRAFTING_REMAINDERS {
            catalog.crafting_remainders.insert(
                item_resource_id(item)?.to_owned(),
                item_resource_id(remainder)?.to_owned(),
            );
        }
        for &(item, block) in VANILLA_ITEM_SPECIAL_BLOCKS {
            let item = item_resource_id(item)?.to_owned();
            let block = block_resource_id(block)?.to_owned();
            catalog.block_items.insert(item.clone());
            catalog
                .primary_block_resource_ids
                .insert(item.clone(), block.clone());
            catalog.block_item_resource_ids.insert(block, item);
        }
        for &(item, block) in VANILLA_ITEM_ALTERNATIVE_BLOCKS {
            catalog.block_item_resource_ids.insert(
                block_resource_id(block)?.to_owned(),
                item_resource_id(item)?.to_owned(),
            );
        }
        Ok(catalog)
    }

    pub fn resource_id(&self, protocol_id: i32) -> Option<&str> {
        usize::try_from(protocol_id)
            .ok()
            .and_then(|protocol_id| self.resource_ids.get(protocol_id))
            .map(String::as_str)
    }

    pub fn protocol_id(&self, resource_id: &str) -> Option<i32> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.protocol_ids.get(&resource_id).copied()
    }

    pub fn resource_ids(&self) -> &[String] {
        &self.resource_ids
    }

    pub fn is_block_item(&self, resource_id: &str) -> bool {
        ResourceLocation::parse(resource_id)
            .ok()
            .is_some_and(|id| self.block_items.contains(&id.id()))
    }

    pub fn item_resource_id_for_block(&self, block_resource_id: &str) -> Option<&str> {
        let block_resource_id = ResourceLocation::parse(block_resource_id).ok()?.id();
        self.block_item_resource_ids
            .get(&block_resource_id)
            .map(String::as_str)
    }

    pub fn primary_block_resource_id_for_item(&self, item_resource_id: &str) -> Option<&str> {
        let item_resource_id = ResourceLocation::parse(item_resource_id).ok()?.id();
        self.primary_block_resource_ids
            .get(&item_resource_id)
            .map(String::as_str)
    }

    pub fn is_spawn_egg_item(&self, resource_id: &str) -> bool {
        ResourceLocation::parse(resource_id)
            .ok()
            .is_some_and(|id| self.spawn_egg_items.contains(&id.id()))
    }

    pub fn default_component_codec_values(
        &self,
        resource_id: &str,
    ) -> Option<&BTreeMap<i32, serde_json::Value>> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.default_component_codec_values.get(&resource_id)
    }

    pub fn default_component_codec_values_by_protocol_id(
        &self,
    ) -> BTreeMap<i32, BTreeMap<i32, serde_json::Value>> {
        self.default_component_codec_values
            .iter()
            .filter_map(|(resource_id, values)| {
                self.protocol_ids
                    .get(resource_id)
                    .copied()
                    .map(|item_id| (item_id, values.clone()))
            })
            .collect()
    }

    pub fn crafting_remainder(&self, resource_id: &str) -> Option<&str> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.crafting_remainders
            .get(&resource_id)
            .map(String::as_str)
    }

    pub fn crafting_remainders_by_protocol_id(&self) -> BTreeMap<i32, i32> {
        self.crafting_remainders
            .iter()
            .filter_map(|(resource_id, remainder_id)| {
                let item_id = self.protocol_ids.get(resource_id).copied()?;
                let remainder_item_id = self.protocol_ids.get(remainder_id).copied()?;
                Some((item_id, remainder_item_id))
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.resource_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.resource_ids.is_empty()
    }
}

fn block_resource_id(id: i32) -> Result<&'static str> {
    VANILLA_BLOCK_TYPES
        .get(id as usize)
        .map(|block| block.resource_id)
        .context("unknown item prototype block")
}

fn item_resource_id(id: i32) -> Result<&'static str> {
    vanilla_item_type(id)
        .map(|item| item.resource_id)
        .context("unknown item prototype item")
}

#[cfg(test)]
mod tests;
