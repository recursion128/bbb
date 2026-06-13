use bbb_protocol::packets::{RegistryData, UpdateTags};

use crate::WorldStore;

use super::{RegistryContentState, RegistryPacket, RegistryPacketEntry, RegistryTagState};

impl WorldStore {
    pub fn record_registry(&mut self, name: impl Into<String>, raw_payload_len: usize) {
        self.record_registry_entries(name, raw_payload_len, Vec::new());
    }

    pub fn record_registry_data(&mut self, update: RegistryData) {
        let entries = update
            .entries
            .into_iter()
            .map(RegistryPacketEntry::from)
            .collect();
        self.record_registry_entries(update.registry, update.raw_payload_len, entries);
    }

    pub fn record_registry_entries(
        &mut self,
        name: impl Into<String>,
        raw_payload_len: usize,
        entries: Vec<RegistryPacketEntry>,
    ) {
        let name = name.into();
        self.counters.last_registry_data_registry = Some(name.clone());
        self.counters.last_registry_data_entry_count = entries.len();
        self.registries
            .contents
            .entry(name.clone())
            .or_insert_with(|| RegistryContentState::new(name.clone()))
            .append_packet(&entries);
        self.registries.registries.push(RegistryPacket {
            name,
            raw_payload_len,
            entries,
        });
        self.sync_registry_counters();
    }

    pub fn registries(&self) -> &super::RegistrySet {
        &self.registries
    }

    pub fn registry_content(&self, registry: &str) -> Option<&RegistryContentState> {
        self.registries.contents.get(registry)
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

        for registry in update.registries {
            let tags = registry
                .tags
                .into_iter()
                .map(|tag| (tag.tag, tag.entries))
                .collect();
            self.registries.tags.insert(
                registry.registry.clone(),
                RegistryTagState {
                    registry: registry.registry,
                    tags,
                },
            );
        }

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
