use crate::WorldStore;
use bbb_protocol::packets::{RegistryData, UpdateTags};
use bbb_protocol::registry_data_types::{built_in_registry_type, synchronized_registry_data_type};
use bbb_protocol::registry_holder_consumers::registry_holder_consumer_count;

use super::{
    revision::{materialize_registry_tags, RegistryRevisionCandidate},
    state::PendingRegistryCollection,
    BiomeClientProjection, BiomeProjectionError, RegistryContentState, RegistryPacket,
    RegistryPacketEntry, RegistryTagState, ResolvedRegistryEntry, UnresolvedRegistryTagHolder,
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct RegistryValidationError {
    pub registry: String,
    pub validator: String,
    pub message: String,
}

impl WorldStore {
    /// Starts a fresh server session with only the client built-in fallback.
    ///
    /// Vanilla creates the configuration listener reached from login with
    /// `ClientRegistryLayer.createRegistryAccess()`, while play reconfiguration
    /// carries the current frozen access in `CommonListenerCookie`. Keep those
    /// two entry paths distinct so a transfer cannot inherit the previous
    /// server's holder order when the next server sends no registry contents.
    pub fn reset_registry_session(&mut self) {
        self.reset_creative_inventory_for_new_level();
        self.clear_requested_debug_subscriptions();
        self.clear_debug_subscriber_values();
        let next_session = self.registries.revision.session.saturating_add(1);
        self.registries = super::RegistrySet::vanilla_26_1().into();
        self.registries.revision.session = next_session;
        self.begin_known_pack_configuration();
        self.counters.last_registry_data_registry = None;
        self.counters.last_registry_data_entry_count = 0;
        self.counters.last_update_tags_registry_count = 0;
        self.counters.last_update_tags_total_tag_count = 0;
        self.counters.last_update_tags_total_value_count = 0;
        self.sync_registry_counters();
        self.sync_registry_tag_counters();
        self.items.clear_item_component_defaults();
        self.items.refresh_furnace_fuel_item_ids(&self.registries);
    }

    pub fn begin_registry_configuration(&mut self) {
        self.begin_known_pack_configuration();
        self.registries.pending = Some(PendingRegistryCollection::default());
    }

    pub fn finish_registry_configuration(&mut self) -> Result<(), RegistryValidationError> {
        let Some(pending) = self.registries.pending.take() else {
            return Ok(());
        };
        let candidate = RegistryRevisionCandidate::build(
            &self.registries,
            pending,
            self.selected_known_pack_data
                .as_ref()
                .map(|source| &**source),
            self.known_pack_data_error.as_deref(),
            !self.features.known_packs.selected.is_empty(),
            &self.items.registry_dependent_component_initializer_rules,
            &self.items.default_component_codec_values,
        )
        .map_err(|failure| {
            self.record_apply_error(failure.stage, &failure.error);
            failure.error
        })?;
        let (registries, item_defaults) = candidate.into_parts();
        self.registries = registries.into();
        self.items.commit_item_component_defaults(item_defaults);
        self.sync_registry_counters();
        self.sync_registry_tag_counters();
        self.items.refresh_furnace_fuel_item_ids(&self.registries);
        self.refresh_open_creative_inventory();
        Ok(())
    }

    pub fn record_registry(&mut self, name: impl Into<String>, raw_payload_len: usize) {
        self.record_registry_entries(name, raw_payload_len, Vec::new());
    }

    pub fn record_registry_data(&mut self, update: RegistryData) {
        let collect_contents = synchronized_registry_data_type(&update.registry).is_some();
        let entries = update
            .entries
            .into_iter()
            .map(RegistryPacketEntry::from)
            .collect();
        self.record_registry_entries_with_policy(
            update.registry,
            update.raw_payload_len,
            entries,
            collect_contents,
        );
    }

    pub fn record_registry_entries(
        &mut self,
        name: impl Into<String>,
        raw_payload_len: usize,
        entries: Vec<RegistryPacketEntry>,
    ) {
        self.record_registry_entries_with_policy(name, raw_payload_len, entries, true);
    }

    fn record_registry_entries_with_policy(
        &mut self,
        name: impl Into<String>,
        raw_payload_len: usize,
        entries: Vec<RegistryPacketEntry>,
        collect_contents: bool,
    ) {
        let name = name.into();
        self.counters.last_registry_data_registry = Some(name.clone());
        self.counters.last_registry_data_entry_count = entries.len();
        let packet = RegistryPacket {
            configuration_synchronized: synchronized_registry_data_type(&name).is_some(),
            client_built_in: built_in_registry_type(&name).is_some(),
            holder_consumer_count: registry_holder_consumer_count(&name),
            name,
            raw_payload_len,
            entries,
        };
        if let Some(pending) = self.registries.pending.as_mut() {
            if collect_contents {
                pending
                    .contents
                    .entry(packet.name.clone())
                    .or_insert_with(|| RegistryContentState::new(packet.name.clone()))
                    .append_packet(&packet.entries);
            }
            pending.registries.push(packet);
        } else {
            if collect_contents {
                self.registries
                    .contents
                    .entry(packet.name.clone())
                    .or_insert_with(|| RegistryContentState::new(packet.name.clone()))
                    .append_packet(&packet.entries);
                self.registries.revision.contents =
                    self.registries.revision.contents.saturating_add(1);
            }
            self.registries.registries.push(packet);
            self.sync_registry_counters();
        }
    }

    pub fn registries(&self) -> &super::RegistrySet {
        &self.registries
    }

    pub fn frozen_registry_access(&self) -> super::FrozenRegistryAccess<'_> {
        self.registries.frozen_access()
    }

    pub fn registry_access_revision(&self) -> super::RegistryAccessRevision {
        self.registries.revision
    }

    pub fn registry_content(&self, registry: &str) -> Option<&RegistryContentState> {
        self.registries.contents.get(registry)
    }

    pub fn registry_entry_by_holder_id(
        &self,
        registry: &str,
        holder_id: i32,
    ) -> Option<&RegistryPacketEntry> {
        self.registries.entry_by_holder_id(registry, holder_id)
    }

    pub fn registry_resource_id_by_holder_id(
        &self,
        registry: &str,
        holder_id: i32,
    ) -> Option<&str> {
        self.registries
            .resource_id_by_holder_id(registry, holder_id)
    }

    pub fn registry_biome_client_projection(
        &self,
        holder_id: i32,
    ) -> Result<Option<BiomeClientProjection>, BiomeProjectionError> {
        self.registries.biome_client_projection(holder_id)
    }

    pub fn resolved_registry_entry_by_holder_id(
        &self,
        registry: &str,
        holder_id: i32,
    ) -> Option<ResolvedRegistryEntry<'_>> {
        self.frozen_registry_access()
            .entry_by_holder_id(registry, holder_id)
            .map(|entry| ResolvedRegistryEntry {
                holder_id: entry.holder_id,
                resource_id: entry.resource_id,
                source: entry.source,
            })
    }

    pub fn registry_resource_ids_in_holder_order<'a>(
        &'a self,
        registry: &str,
    ) -> Option<impl ExactSizeIterator<Item = &'a str> + 'a> {
        self.registries.resource_ids_in_holder_order(registry)
    }

    pub fn registry_holder_id_by_resource_id(
        &self,
        registry: &str,
        resource_id: &str,
    ) -> Option<i32> {
        self.registries
            .holder_id_by_resource_id(registry, resource_id)
    }

    pub fn resolved_registry_entry_by_resource_id(
        &self,
        registry: &str,
        resource_id: &str,
    ) -> Option<ResolvedRegistryEntry<'_>> {
        self.frozen_registry_access()
            .entry_by_resource_id(registry, resource_id)
            .map(|entry| ResolvedRegistryEntry {
                holder_id: entry.holder_id,
                resource_id: entry.resource_id,
                source: entry.source,
            })
    }

    pub fn registry_holder_ids_for_tag(&self, registry: &str, tag: &str) -> Option<&[i32]> {
        self.registries.holder_ids_for_tag(registry, tag)
    }

    pub fn registry_unresolved_holder_ids_for_tag(
        &self,
        registry: &str,
        tag: &str,
    ) -> Option<&[i32]> {
        self.registries.unresolved_holder_ids_for_tag(registry, tag)
    }

    pub fn registry_holder_is_in_tag(&self, registry: &str, tag: &str, holder_id: i32) -> bool {
        self.registries.holder_is_in_tag(registry, tag, holder_id)
    }

    pub fn registry_tag_names_containing_holder_id<'a>(
        &'a self,
        registry: &str,
        holder_id: i32,
    ) -> Option<impl Iterator<Item = &'a str> + 'a> {
        self.registries
            .tag_names_containing_holder_id(registry, holder_id)
    }

    /// Matches the tag-search branch used by CreativeModeInventoryScreen.
    ///
    /// Vanilla rebuilds an IdSearchTree after UpdateTags because that tree
    /// captures ItemStack tag identifiers. bbb keeps no duplicate search-tree
    /// cache: the future creative screen can filter its display stacks in their
    /// own order through this live predicate, so every committed tag reload is
    /// visible immediately.
    pub fn creative_item_matches_tag_search(&self, item_holder_id: i32, search_term: &str) -> bool {
        let search_term = search_term.to_lowercase();
        self.registries
            .tag_names_containing_holder_id("minecraft:item", item_holder_id)
            .is_some_and(|tags| {
                tags.into_iter()
                    .any(|tag| resource_id_matches_search(tag, &search_term))
            })
    }

    pub fn registry_resource_ids_for_tag<'a>(
        &'a self,
        registry: &str,
        tag: &str,
    ) -> Result<Option<Vec<&'a str>>, UnresolvedRegistryTagHolder> {
        self.registries.resource_ids_for_tag(registry, tag)
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
        self.counters.registry_entry_payload_bytes = self
            .registries
            .registries
            .iter()
            .flat_map(|registry| registry.entries.iter())
            .map(|entry| entry.raw_data_len)
            .sum();
        self.counters.registry_content_registries_tracked = self.registries.contents.len();
        self.counters.registry_duplicate_entries = self
            .registries
            .contents
            .values()
            .map(RegistryContentState::duplicate_entry_count)
            .sum();
        self.counters.registry_content_packets_tracked = self
            .registries
            .contents
            .values()
            .map(|content| content.packet_count)
            .sum();
        self.counters.registry_content_entries_tracked = self
            .registries
            .contents
            .values()
            .map(|content| content.entries.len())
            .sum();
        self.counters.registry_duplicate_entry_ids_tracked = self
            .registries
            .contents
            .values()
            .map(|content| content.duplicate_entry_ids.len())
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

        let has_registry_updates = !update.registries.is_empty();
        for registry in update.registries {
            let tags = registry
                .tags
                .into_iter()
                .map(|tag| (tag.tag, tag.entries))
                .collect();
            let raw = RegistryTagState {
                registry: registry.registry,
                tags,
            };
            if let Some(pending) = self.registries.pending.as_mut() {
                pending.tags.insert(raw.registry.clone(), raw);
            } else {
                let (resolved, unresolved) = materialize_registry_tags(&self.registries, None, raw);
                let registry = resolved.registry.clone();
                self.registries.tags.insert(registry.clone(), resolved);
                self.registries.unresolved_tag_holders.remove(&registry);
                if !unresolved.is_empty() {
                    self.registries
                        .unresolved_tag_holders
                        .insert(registry, unresolved);
                }
            }
        }

        if self.registries.pending.is_none() {
            if has_registry_updates {
                self.registries.revision.tags = self.registries.revision.tags.saturating_add(1);
            }
            self.sync_registry_tag_counters();
            self.items.refresh_furnace_fuel_item_ids(&self.registries);
            self.refresh_open_creative_inventory();
        }
    }

    fn sync_registry_tag_counters(&mut self) {
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

fn resource_id_matches_search(resource_id: &str, search_term: &str) -> bool {
    let (namespace, path) = resource_id
        .split_once(':')
        .unwrap_or(("minecraft", resource_id));
    match search_term.split_once(':') {
        Some((namespace_search, path_search)) => {
            namespace.contains(namespace_search.trim()) && path.contains(path_search.trim())
        }
        None => path.contains(search_term),
    }
}
