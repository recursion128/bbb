//! Immutable registry-tag membership for one datapack generation.

use std::sync::{Arc, OnceLock};

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::content_registry::ContentRegistry;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::block_getter::BlockTags;

#[derive(Debug, Clone, PartialEq)]
pub struct RegistryTagSnapshot {
    registries: FxHashMap<String, FxHashMap<String, TagContents>>,
    content_registry: Arc<ContentRegistry>,
}

impl Default for RegistryTagSnapshot {
    fn default() -> Self {
        Self {
            registries: FxHashMap::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TagContents {
    ordered: Box<[Arc<str>]>,
    membership: FxHashSet<Arc<str>>,
    block_membership: Option<Box<[u64]>>,
}

impl RegistryTagSnapshot {
    /// A later entry for the same registry and tag replaces an earlier one.
    pub fn new(tags: impl IntoIterator<Item = (String, String, Vec<String>)>) -> Self {
        Self::new_with_content_registry(tags, Arc::clone(ContentRegistry::vanilla()))
    }

    pub fn new_with_content_registry(
        tags: impl IntoIterator<Item = (String, String, Vec<String>)>,
        content_registry: Arc<ContentRegistry>,
    ) -> Self {
        let mut registries: FxHashMap<String, FxHashMap<String, TagContents>> =
            FxHashMap::default();
        let total_blocks = content_registry.total_block_count();
        for (registry_id, tag_id, members) in tags {
            // TagLoader.java:76-86 keeps the first occurrence in a LinkedHashSet.
            let mut membership = FxHashSet::default();
            let mut ordered = Vec::new();
            for member in members {
                let member: Arc<str> = Arc::from(member);
                if membership.insert(Arc::clone(&member)) {
                    ordered.push(member);
                }
            }
            let block_membership = (registry_id == "minecraft:block").then(|| {
                let mut bits = vec![0; total_blocks.div_ceil(64)];
                for member in &ordered {
                    let block_id =
                        bbb_protocol::block_types::vanilla_block_type_for_resource_id(member)
                            .map(|block| block.id)
                            .or_else(|| {
                                content_registry
                                    .block_by_resource_id(member)
                                    .map(|block| block.block_id)
                            });
                    if let Some(id) = block_id.and_then(|id| usize::try_from(id).ok()) {
                        if let Some(word) = bits.get_mut(id / 64) {
                            *word |= 1 << (id % 64);
                        }
                    }
                }
                bits.into_boxed_slice()
            });
            registries.entry(registry_id).or_default().insert(
                tag_id,
                TagContents {
                    ordered: ordered.into_boxed_slice(),
                    membership,
                    block_membership,
                },
            );
        }
        Self {
            registries,
            content_registry,
        }
    }

    /// Builds an immutable O(1) tag membership snapshot from a loaded datapack
    /// tag report.
    pub fn from_tag_load_report(tags: &bbb_server_datapack::tag_loader::TagLoadReport) -> Self {
        Self::from_tag_load_report_with_content_registry(
            tags,
            Arc::clone(ContentRegistry::vanilla()),
        )
    }

    pub fn from_tag_load_report_with_content_registry(
        tags: &bbb_server_datapack::tag_loader::TagLoadReport,
        content_registry: Arc<ContentRegistry>,
    ) -> Self {
        Self::new_with_content_registry(
            tags.registries.iter().flat_map(|registry| {
                registry.tags.iter().map(|(tag_id, members)| {
                    (
                        registry.registry_id.clone(),
                        tag_id.clone(),
                        members.clone(),
                    )
                })
            }),
            content_registry,
        )
    }

    pub fn content_registry(&self) -> &ContentRegistry {
        &self.content_registry
    }

    pub fn content_registry_arc(&self) -> &Arc<ContentRegistry> {
        &self.content_registry
    }

    /// The base Vanilla 26.1 generation, for callers with no level or datapack.
    pub fn builtin() -> &'static Self {
        static BUILTIN: OnceLock<Arc<RegistryTagSnapshot>> = OnceLock::new();
        BUILTIN.get_or_init(crate::registry_sync::builtin_registry_tags)
    }

    fn get(&self, registry_id: &str, tag_id: &str) -> Option<&TagContents> {
        self.registries.get(registry_id)?.get(tag_id)
    }

    pub fn contains(&self, registry_id: &str, tag_id: &str, element_id: &str) -> bool {
        self.get(registry_id, tag_id)
            .is_some_and(|contents| contents.membership.contains(element_id))
    }

    pub fn contains_block(&self, tag_id: &str, block_type_id: i32) -> bool {
        let Ok(id) = usize::try_from(block_type_id) else {
            return false;
        };
        self.get("minecraft:block", tag_id)
            .and_then(|contents| contents.block_membership.as_ref())
            .and_then(|bits| bits.get(id / 64))
            .is_some_and(|word| word & (1 << (id % 64)) != 0)
    }

    pub fn block_state_in(&self, tag_id: &str, block_state_id: i32) -> bool {
        let block_type_id = vanilla_block_state_block_type_id(block_state_id).or_else(|| {
            self.content_registry
                .block_by_state_id(block_state_id)
                .map(|(block, _)| block.block_id)
        });
        block_type_id.is_some_and(|block_type_id| self.contains_block(tag_id, block_type_id))
    }

    pub(crate) fn tag_ids(&self, registry_id: &str) -> impl Iterator<Item = &str> {
        self.registries
            .get(registry_id)
            .into_iter()
            .flat_map(|tags| tags.keys().map(String::as_str))
    }

    pub fn tag_exists(&self, registry_id: &str, tag_id: &str) -> bool {
        self.get(registry_id, tag_id).is_some()
    }

    pub fn members(&self, registry_id: &str, tag_id: &str) -> &[Arc<str>] {
        self.get(registry_id, tag_id)
            .map_or(&[], |contents| &contents.ordered)
    }
}

impl BlockTags for RegistryTagSnapshot {
    fn contains(&self, registry_id: &str, tag_id: &str, element_id: &str) -> bool {
        RegistryTagSnapshot::contains(self, registry_id, tag_id, element_id)
    }

    fn content_registry(&self) -> &ContentRegistry {
        &self.content_registry
    }

    fn block_in_tag(&self, block_state_id: i32, tag_id: &str) -> bool {
        self.block_state_in(tag_id, block_state_id)
    }
}

impl bbb_server_types::block_state_parse::BlockTagLookup for RegistryTagSnapshot {
    fn block_tag_members(&self, tag_id: &str) -> Option<Vec<i32>> {
        self.tag_exists("minecraft:block", tag_id).then(|| {
            self.members("minecraft:block", tag_id)
                .iter()
                .filter_map(|member| {
                    bbb_protocol::block_types::vanilla_block_type_for_resource_id(member)
                        .map(|block| block.id)
                        .or_else(|| {
                            self.content_registry
                                .block_by_resource_id(member)
                                .map(|block| block.block_id)
                        })
                })
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK: &str = "minecraft:block";
    const SAND: &str = "minecraft:sand";

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn missing_tag_is_empty() {
        let snapshot = RegistryTagSnapshot::new(Vec::new());

        assert!(!snapshot.tag_exists(BLOCK, SAND));
        assert!(snapshot.members(BLOCK, SAND).is_empty());
        assert!(!snapshot.contains(BLOCK, SAND, "a"));
    }

    #[test]
    fn members_keep_first_seen_order_and_deduplicate() {
        let snapshot = RegistryTagSnapshot::new([(
            BLOCK.to_owned(),
            SAND.to_owned(),
            strings(&["c", "a", "c", "b", "a"]),
        )]);

        assert!(snapshot.tag_exists(BLOCK, SAND));
        assert!(snapshot.contains(BLOCK, SAND, "a"));
        assert!(!snapshot.contains(BLOCK, SAND, "d"));
        assert_eq!(
            snapshot
                .members(BLOCK, SAND)
                .iter()
                .map(|member| member.as_ref())
                .collect::<Vec<_>>(),
            vec!["c", "a", "b"]
        );

        use bbb_protocol::block_types::{VANILLA_BLOCK_SAND_ID, VANILLA_BLOCK_STONE_ID};
        let original = RegistryTagSnapshot::new([(
            BLOCK.to_owned(),
            SAND.to_owned(),
            strings(&["minecraft:sand", "minecraft:sand"]),
        )]);
        let replaced = RegistryTagSnapshot::new([(
            BLOCK.to_owned(),
            SAND.to_owned(),
            strings(&["minecraft:stone"]),
        )]);
        assert!(original.contains_block(SAND, VANILLA_BLOCK_SAND_ID));
        assert!(!original.contains_block(SAND, VANILLA_BLOCK_STONE_ID));
        assert!(!replaced.contains_block(SAND, VANILLA_BLOCK_SAND_ID));
        assert!(replaced.contains_block(SAND, VANILLA_BLOCK_STONE_ID));
        assert!(!replaced.contains_block("minecraft:missing", VANILLA_BLOCK_STONE_ID));
        assert!(!replaced.contains_block(SAND, -1));
    }
}
