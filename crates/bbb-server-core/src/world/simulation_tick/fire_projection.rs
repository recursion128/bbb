use crate::world::LevelId;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_server_datapack::dimension_type::{
    builtin_dimension_type_runtime_facts, dimension_type_runtime_facts,
};
use bbb_server_types::fluid::block_type_ids_in_tag;
#[cfg(test)]
use bbb_server_types::save::Java26BuiltinDimension;

use super::ServerWorld;
use crate::world::spawn_predicate_level::{spawn_dimension_facts_of, SpawnDimensionFacts};

#[derive(Default)]
pub(in crate::world) struct InfiniburnCache {
    state: RefCell<InfiniburnCacheState>,
}

#[derive(Default)]
struct InfiniburnCacheState {
    entries: BTreeMap<LevelId, InfiniburnCacheEntry>,
    #[cfg(test)]
    projections: BTreeMap<LevelId, usize>,
}

struct InfiniburnCacheEntry {
    datapacks: Option<Arc<bbb_server_datapack::startup::DatapackStartup>>,
    element_id: String,
    blocks: Arc<HashSet<i32>>,
    /// The spawn predicates' dimension facts, read from the same dimension type.
    spawn: Option<SpawnDimensionFacts>,
}

impl InfiniburnCacheEntry {
    fn matches(
        &self,
        datapacks: &Option<Arc<bbb_server_datapack::startup::DatapackStartup>>,
        element_id: &str,
    ) -> bool {
        self.element_id == element_id
            && match (&self.datapacks, datapacks) {
                (Some(cached), Some(current)) => Arc::ptr_eq(cached, current),
                (None, None) => true,
                _ => false,
            }
    }
}

impl ServerWorld {
    pub(super) fn level_infiniburn_blocks(&self, dimension: &LevelId) -> Arc<HashSet<i32>> {
        self.with_dimension_projection(dimension, |entry| Arc::clone(&entry.blocks))
            .unwrap_or_default()
    }

    /// `spawn_dimension_facts` for the level's own dimension type, without
    /// decoding the dimension type again every tick.
    pub(in crate::world) fn level_spawn_dimension_facts(
        &self,
        dimension: &LevelId,
        thundering: bool,
    ) -> Option<SpawnDimensionFacts> {
        let mut facts =
            self.with_dimension_projection(dimension, |entry| entry.spawn.clone())??;
        facts.thundering = thundering;
        Some(facts)
    }

    fn with_dimension_projection<R>(
        &self,
        dimension: &LevelId,
        read: impl FnOnce(&InfiniburnCacheEntry) -> R,
    ) -> Option<R> {
        let element_id = self.cold.level_dimension_types.get(dimension).cloned()?;
        let datapacks = self.cold.config.datapacks.clone();
        let mut cache = self.cold.infiniburn_cache.state.borrow_mut();
        if let Some(entry) = cache.entries.get(dimension) {
            if entry.matches(&datapacks, &element_id) {
                return Some(read(entry));
            }
        }
        let facts = match datapacks.as_deref() {
            Some(startup) => dimension_type_runtime_facts(&startup.registries, &element_id).ok(),
            None => builtin_dimension_type_runtime_facts(&element_id).ok(),
        };
        let spawn = facts
            .as_ref()
            .and_then(|facts| spawn_dimension_facts_of(facts, false));
        let blocks = Arc::new(match (datapacks.as_deref(), facts) {
            (Some(startup), Some(facts)) => startup
                .tags
                .registry("minecraft:block")
                .and_then(|registry| registry.tag(&facts.infiniburn_tag))
                .and_then(|members| {
                    members
                        .iter()
                        .map(|member| {
                            vanilla_block_type_for_resource_id(member).map(|entry| entry.id)
                        })
                        .collect::<Option<HashSet<_>>>()
                })
                .unwrap_or_default(),
            (None, Some(facts)) => block_type_ids_in_tag(&facts.infiniburn_tag),
            (_, None) => HashSet::new(),
        });
        #[cfg(test)]
        {
            *cache.projections.entry(*dimension).or_default() += 1;
        }
        let entry = InfiniburnCacheEntry {
            datapacks,
            element_id,
            blocks,
            spawn,
        };
        let answer = read(&entry);
        cache.entries.insert(*dimension, entry);
        Some(answer)
    }

    #[cfg(test)]
    pub(crate) fn set_level_dimension_type_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        element_id: &str,
    ) {
        self.cold
            .level_dimension_types
            .insert(dimension.into(), element_id.to_string());
    }

    #[cfg(test)]
    pub(crate) fn level_infiniburn_blocks_for_test(
        &self,
        dimension: Java26BuiltinDimension,
    ) -> HashSet<i32> {
        (*self.level_infiniburn_blocks(&dimension.into())).clone()
    }
}
