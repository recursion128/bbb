use std::collections::{BTreeMap, BTreeSet};

use bbb_registry_codec::registry_codec::RegistryReferenceLookup;

use crate::dynamic_registry::DatapackGeneration;
use crate::frozen_lookup::{validate_frozen_references_with_lookup, FrozenDatapackRegistryLookup};
use crate::registry_data_loader::DatapackLoadError;
use crate::tag_loader::{network_safe_static_registry_elements, TagLoadReport};

#[derive(Debug, Clone)]
pub(crate) struct FrozenRegistryScope {
    bound_tags: std::sync::Arc<BTreeMap<String, BTreeSet<String>>>,
}

impl FrozenRegistryScope {
    pub(crate) fn new(tags: &TagLoadReport) -> Self {
        Self {
            bound_tags: std::sync::Arc::new(
                tags.registries
                    .iter()
                    .map(|registry| {
                        (
                            registry.registry_id.clone(),
                            registry.tags.keys().cloned().collect(),
                        )
                    })
                    .collect(),
            ),
        }
    }

    fn contains_bound_tag(&self, registry: &str, id: &str) -> bool {
        self.bound_tags
            .get(registry)
            .is_some_and(|tags| tags.contains(id))
    }
}

pub(super) fn retain_unreloaded_tags(
    current: &mut TagLoadReport,
    previous: &TagLoadReport,
    generation: &DatapackGeneration,
) {
    // TagLoader.java:152-156 preserves the old lookup without PendingTags.
    let mut registries = std::mem::take(&mut current.registries)
        .into_iter()
        .filter(|registry| !registry.tags.is_empty())
        .map(|registry| (registry.registry_id.clone(), registry))
        .collect::<BTreeMap<_, _>>();
    for registry in &previous.registries {
        registries
            .entry(registry.registry_id.clone())
            .or_insert_with(|| registry.clone());
    }
    for registry_id in generation
        .registries
        .iter()
        .map(|registry| registry.registry_id.as_str())
        .chain(
            network_safe_static_registry_elements()
                .iter()
                .map(|(id, _)| *id),
        )
    {
        if let Some(registry) = registries.remove(registry_id) {
            current.registries.push(registry);
        }
    }
    current.registries.extend(registries.into_values());
}

impl super::DatapackStartup {
    pub fn validate_frozen_registry_references(&self) -> Result<(), DatapackLoadError> {
        let lookup = BoundRegistryLookup {
            current: FrozenDatapackRegistryLookup::from_startup(self),
            scope: &self.registry_scope,
        };
        validate_frozen_references_with_lookup(&self.registries, &lookup)
    }
}

struct BoundRegistryLookup<'a> {
    current: FrozenDatapackRegistryLookup<'a>,
    scope: &'a FrozenRegistryScope,
}

impl RegistryReferenceLookup for BoundRegistryLookup<'_> {
    fn contains_element(&self, registry: &str, id: &str) -> bool {
        self.current.contains_element(registry, id)
    }

    fn contains_tag(&self, registry: &str, id: &str) -> bool {
        // MinecraftServer.java:1512-1516 reloads resources against the frozen access.
        self.scope.contains_bound_tag(registry, id)
    }
}

#[cfg(test)]
mod tests;
