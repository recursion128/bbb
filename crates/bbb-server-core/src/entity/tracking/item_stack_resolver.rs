//! One frozen item-stack registry publication, including empty registries.

use crate::registry_tags::RegistryTagSnapshot;
use bbb_protocol::{
    PersistentRegistryMembership, PersistentRegistryResolver, PersistentRegistryTagMembers,
};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct ItemStackRegistryPublication {
    pub(super) holder_ids: BTreeMap<(String, String), i32>,
    pub(super) registry_ids: BTreeSet<String>,
    pub(super) tags: Option<Arc<RegistryTagSnapshot>>,
}

impl ItemStackRegistryPublication {
    pub(super) fn resolver(&self) -> TrackingItemStackResolver<'_> {
        TrackingItemStackResolver {
            holder_ids: &self.holder_ids,
            registry_ids: Some(&self.registry_ids),
            tags: self.tags.as_deref(),
        }
    }
}

/// Frozen registry membership and wire IDs used by entity item-stack publication.
#[derive(Clone, Copy)]
pub struct TrackingItemStackResolver<'a> {
    holder_ids: &'a BTreeMap<(String, String), i32>,
    registry_ids: Option<&'a BTreeSet<String>>,
    tags: Option<&'a RegistryTagSnapshot>,
}

impl<'a> TrackingItemStackResolver<'a> {
    pub const fn new(holder_ids: &'a BTreeMap<(String, String), i32>) -> Self {
        Self {
            holder_ids,
            registry_ids: None,
            tags: None,
        }
    }
}

impl TrackingItemStackResolver<'_> {
    fn owns_registry(&self, registry_id: &str) -> bool {
        match self.registry_ids {
            Some(ids) => ids.contains(registry_id),
            None => self
                .holder_ids
                .keys()
                .any(|(registry, _)| registry == registry_id),
        }
    }
}

impl PersistentRegistryResolver for TrackingItemStackResolver<'_> {
    fn registry_available(&self, registry_id: &str) -> bool {
        self.owns_registry(registry_id)
            || bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_available_26_1(
                registry_id,
            )
    }

    fn resource_membership(
        &self,
        registry_id: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership {
        if self
            .holder_ids
            .contains_key(&(registry_id.to_owned(), resource_id.to_owned()))
        {
            return PersistentRegistryMembership::Present;
        }
        if self.owns_registry(registry_id) {
            return PersistentRegistryMembership::Absent;
        }
        bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_membership_26_1(
            registry_id,
            resource_id,
        )
    }

    fn tag_members(&self, registry_id: &str, tag_id: &str) -> PersistentRegistryTagMembers {
        match self.tags {
            Some(tags) if tags.tag_exists(registry_id, tag_id) => PersistentRegistryTagMembers::Present(
                tags.members(registry_id, tag_id).iter().map(|member| member.to_string()).collect(),
            ),
            Some(_) => PersistentRegistryTagMembers::Absent,
            None => bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1(registry_id, tag_id),
        }
    }
}

impl PersistentItemStackHolderResolver for TrackingItemStackResolver<'_> {
    fn holder_id(&self, registry_id: &str, resource_id: &str) -> Option<i32> {
        self.holder_ids
            .get(&(registry_id.to_owned(), resource_id.to_owned()))
            .copied()
    }

    fn holder_resource_id(&self, registry_id: &str, holder_id: i32) -> Option<String> {
        let mut matches = self
            .holder_ids
            .iter()
            .filter_map(|((registry, resource), id)| {
                (registry == registry_id && *id == holder_id).then_some(resource)
            });
        let resource = matches.next()?;
        matches.next().is_none().then(|| resource.clone())
    }
}

#[cfg(test)]
mod tests;
