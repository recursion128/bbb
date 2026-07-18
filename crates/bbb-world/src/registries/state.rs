use std::{collections::BTreeMap, sync::Arc};

use bbb_protocol::attribute_types::{
    vanilla_attribute_type, vanilla_attribute_type_for_resource_id,
};
use bbb_protocol::damage_types::{vanilla_damage_type, vanilla_damage_type_for_resource_id};
use bbb_protocol::mob_effect_types::{
    vanilla_mob_effect_type, vanilla_mob_effect_type_for_resource_id,
};
use bbb_protocol::packets::RegistryDataEntry as ProtocolRegistryDataEntry;
use bbb_protocol::potion_types::{vanilla_potion, vanilla_potion_for_resource_id};
use bbb_protocol::sound_event_types::{
    vanilla_sound_event_registry_id, vanilla_sound_event_resource_id,
};
use bbb_protocol::stat_types::{
    vanilla_custom_stat_by_id, vanilla_custom_stat_by_resource_id, vanilla_stat_type_by_id,
    vanilla_stat_type_by_resource_id,
};
use bbb_protocol::world_clock_types::{
    vanilla_world_clock_type_by_id, vanilla_world_clock_type_by_resource_id,
};
use bbb_protocol::{entity_types::vanilla_entity_type, item_types::vanilla_item_type};
use serde::{Deserialize, Serialize};

use crate::chunks::{decode_nbt_payload_summary, NbtPayloadSummary};

use super::biomes::decode_biome_client_projection;
use super::{BiomeClientProjection, BiomeProjectionError, BlockStateRegistry};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySet {
    pub registries: Vec<RegistryPacket>,
    #[serde(default)]
    pub revision: RegistryAccessRevision,
    #[serde(default)]
    pub contents: BTreeMap<String, RegistryContentState>,
    #[serde(default)]
    pub tags: BTreeMap<String, RegistryTagState>,
    /// Holder ids omitted while materializing network tags against the active
    /// session registry. Vanilla's `NetworkPayload.resolve` drops unknown ids;
    /// retaining them separately keeps that behavior diagnosable without
    /// exposing unresolved holders through the checked tag view.
    #[serde(default)]
    pub unresolved_tag_holders: BTreeMap<String, BTreeMap<String, Vec<i32>>>,
    #[serde(skip)]
    pub block_states: BlockStateRegistry,
    #[serde(skip)]
    pub(super) pending: Option<PendingRegistryCollection>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct PendingRegistryCollection {
    pub registries: Vec<RegistryPacket>,
    pub contents: BTreeMap<String, RegistryContentState>,
    pub tags: BTreeMap<String, RegistryTagState>,
    pub unresolved_tag_holders: BTreeMap<String, BTreeMap<String, Vec<i32>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryPacket {
    pub name: String,
    pub raw_payload_len: usize,
    /// Whether vanilla 26.1 accepts element data for this registry during
    /// configuration RegistryData collection.
    #[serde(default)]
    pub configuration_synchronized: bool,
    /// Whether this registry belongs to vanilla's immutable client static layer.
    #[serde(default)]
    pub client_built_in: bool,
    /// Number of concrete vanilla 26.1 Holder/HolderSet/TagKey codec factory
    /// call sites tied to this registry.
    #[serde(default)]
    pub holder_consumer_count: usize,
    #[serde(default)]
    pub entries: Vec<RegistryPacketEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryContentState {
    pub registry: String,
    pub packet_count: usize,
    #[serde(default)]
    pub entries: Vec<RegistryPacketEntry>,
    #[serde(default)]
    pub duplicate_entry_ids: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryPacketEntry {
    pub id: String,
    #[serde(default)]
    pub has_data: bool,
    #[serde(default)]
    pub raw_data_len: usize,
    #[serde(default)]
    pub nbt: Option<NbtPayloadSummary>,
    #[serde(skip)]
    pub raw_data: Option<Arc<[u8]>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryTagState {
    pub registry: String,
    pub tags: BTreeMap<String, Vec<i32>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedRegistryTagHolder {
    pub registry: String,
    pub tag: String,
    pub holder_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryEntrySource {
    Server,
    VanillaFallback,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryAccessRevision {
    pub session: u64,
    pub contents: u64,
    pub tags: u64,
}

/// Read-only access to the atomically committed registry state.
///
/// Pending configuration packets live outside this view. Holding the borrow
/// therefore pins one committed revision in the same way vanilla hands a
/// frozen `RegistryAccess` to its play packet codecs and consumers.
#[derive(Debug, Clone, Copy)]
pub struct FrozenRegistryAccess<'a> {
    registries: &'a RegistrySet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrozenRegistryEntry<'a> {
    pub holder_id: i32,
    pub resource_id: &'a str,
    pub source: RegistryEntrySource,
    pub payload: RegistryEntryPayload<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryEntryPayload<'a> {
    Server {
        has_data: bool,
        raw_data: Option<&'a [u8]>,
        nbt: Option<&'a NbtPayloadSummary>,
    },
    VanillaFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedRegistryEntry<'a> {
    pub holder_id: i32,
    pub resource_id: &'a str,
    pub source: RegistryEntrySource,
}

impl RegistrySet {
    pub fn vanilla_26_1() -> Self {
        Self {
            registries: Vec::new(),
            revision: RegistryAccessRevision::default(),
            contents: BTreeMap::new(),
            tags: BTreeMap::new(),
            unresolved_tag_holders: BTreeMap::new(),
            block_states: BlockStateRegistry::vanilla_26_1(),
            pending: None,
        }
    }

    pub fn block_state(&self, id: i32) -> Option<&super::BlockStateInfo> {
        self.block_states.by_id(id)
    }

    pub fn frozen_access(&self) -> FrozenRegistryAccess<'_> {
        FrozenRegistryAccess { registries: self }
    }

    pub fn block_name_by_registry_id(&self, id: i32) -> Option<&str> {
        self.block_states.block_name_by_registry_id(id)
    }

    pub fn block_state_id_by_name_and_properties(
        &self,
        name: &str,
        properties: &BTreeMap<String, String>,
    ) -> Option<i32> {
        self.block_states
            .find_by_name_and_properties(name, properties)
            .map(|state| state.id)
    }

    pub fn block_state_count(&self) -> usize {
        self.block_states.len()
    }

    pub fn block_registry_count(&self) -> usize {
        self.block_states.block_registry_len()
    }

    pub fn entry_by_holder_id(
        &self,
        registry: &str,
        holder_id: i32,
    ) -> Option<&RegistryPacketEntry> {
        self.contents.get(registry)?.entry_by_holder_id(holder_id)
    }

    pub fn resource_id_by_holder_id(&self, registry: &str, holder_id: i32) -> Option<&str> {
        self.resolved_entry_by_holder_id(registry, holder_id)
            .map(|entry| entry.resource_id)
    }

    pub fn biome_client_projection(
        &self,
        holder_id: i32,
    ) -> Result<Option<BiomeClientProjection>, BiomeProjectionError> {
        let Some(entry) = self.entry_by_holder_id("minecraft:worldgen/biome", holder_id) else {
            return Ok(None);
        };
        decode_biome_client_projection(entry)
    }

    pub fn resolved_entry_by_holder_id(
        &self,
        registry: &str,
        holder_id: i32,
    ) -> Option<ResolvedRegistryEntry<'_>> {
        if let Some(contents) = self.contents.get(registry) {
            return Some(ResolvedRegistryEntry {
                holder_id,
                resource_id: contents.entry_by_holder_id(holder_id)?.id.as_str(),
                source: RegistryEntrySource::Server,
            });
        }
        Some(ResolvedRegistryEntry {
            holder_id,
            resource_id: self.vanilla_fallback_resource_id(registry, holder_id)?,
            source: RegistryEntrySource::VanillaFallback,
        })
    }

    pub fn resource_ids_in_holder_order<'a>(
        &'a self,
        registry: &str,
    ) -> Option<impl ExactSizeIterator<Item = &'a str> + 'a> {
        Some(
            self.contents
                .get(registry)?
                .entries
                .iter()
                .map(|entry| entry.id.as_str()),
        )
    }

    pub fn holder_id_by_resource_id(&self, registry: &str, resource_id: &str) -> Option<i32> {
        self.resolved_entry_by_resource_id(registry, resource_id)
            .map(|entry| entry.holder_id)
    }

    pub fn resolved_entry_by_resource_id(
        &self,
        registry: &str,
        resource_id: &str,
    ) -> Option<ResolvedRegistryEntry<'_>> {
        if let Some(contents) = self.contents.get(registry) {
            let holder_id = contents.holder_id_by_resource_id(resource_id)?;
            return Some(ResolvedRegistryEntry {
                holder_id,
                resource_id: contents.entry_by_holder_id(holder_id)?.id.as_str(),
                source: RegistryEntrySource::Server,
            });
        }
        let holder_id = self.vanilla_fallback_holder_id(registry, resource_id)?;
        Some(ResolvedRegistryEntry {
            holder_id,
            resource_id: self.vanilla_fallback_resource_id(registry, holder_id)?,
            source: RegistryEntrySource::VanillaFallback,
        })
    }

    pub fn holder_ids_for_tag(&self, registry: &str, tag: &str) -> Option<&[i32]> {
        self.tags.get(registry)?.tags.get(tag).map(Vec::as_slice)
    }

    pub fn unresolved_holder_ids_for_tag(&self, registry: &str, tag: &str) -> Option<&[i32]> {
        self.unresolved_tag_holders
            .get(registry)?
            .get(tag)
            .map(Vec::as_slice)
    }

    pub fn holder_is_in_tag(&self, registry: &str, tag: &str, holder_id: i32) -> bool {
        self.holder_ids_for_tag(registry, tag)
            .is_some_and(|holder_ids| holder_ids.contains(&holder_id))
    }

    pub fn tag_names_containing_holder_id<'a>(
        &'a self,
        registry: &str,
        holder_id: i32,
    ) -> Option<impl Iterator<Item = &'a str> + 'a> {
        Some(
            self.tags
                .get(registry)?
                .tags
                .iter()
                .filter(move |(_, holder_ids)| holder_ids.contains(&holder_id))
                .map(|(tag, _)| tag.as_str()),
        )
    }

    pub fn resource_ids_for_tag<'a>(
        &'a self,
        registry: &str,
        tag: &str,
    ) -> Result<Option<Vec<&'a str>>, UnresolvedRegistryTagHolder> {
        let Some(holder_ids) = self.holder_ids_for_tag(registry, tag) else {
            return Ok(None);
        };
        let mut resource_ids = Vec::with_capacity(holder_ids.len());
        for holder_id in holder_ids {
            let Some(resource_id) = self.resource_id_by_holder_id(registry, *holder_id) else {
                return Err(UnresolvedRegistryTagHolder {
                    registry: registry.to_string(),
                    tag: tag.to_string(),
                    holder_id: *holder_id,
                });
            };
            resource_ids.push(resource_id);
        }
        Ok(Some(resource_ids))
    }

    fn vanilla_fallback_resource_id(&self, registry: &str, holder_id: i32) -> Option<&str> {
        match registry {
            "minecraft:attribute" => Some(vanilla_attribute_type(holder_id)?.resource_id),
            "minecraft:item" => Some(vanilla_item_type(holder_id)?.resource_id),
            "minecraft:block" => self.block_states.block_name_by_registry_id(holder_id),
            "minecraft:entity_type" => Some(vanilla_entity_type(holder_id)?.resource_id),
            "minecraft:damage_type" => Some(vanilla_damage_type(holder_id)?.resource_id),
            "minecraft:mob_effect" => Some(vanilla_mob_effect_type(holder_id)?.resource_id),
            "minecraft:potion" => Some(vanilla_potion(holder_id)?.resource_id),
            "minecraft:sound_event" => vanilla_sound_event_resource_id(holder_id),
            "minecraft:world_clock" => Some(vanilla_world_clock_type_by_id(holder_id)?.resource_id),
            "minecraft:stat_type" => Some(vanilla_stat_type_by_id(holder_id)?.resource_id),
            "minecraft:custom_stat" => Some(vanilla_custom_stat_by_id(holder_id)?.resource_id),
            _ => None,
        }
    }

    fn vanilla_fallback_holder_id(&self, registry: &str, resource_id: &str) -> Option<i32> {
        if !resource_id.starts_with("minecraft:") {
            return None;
        }
        match registry {
            "minecraft:attribute" => Some(vanilla_attribute_type_for_resource_id(resource_id)?.id),
            "minecraft:item" => {
                Some(bbb_protocol::item_types::vanilla_item_type_for_resource_id(resource_id)?.id)
            }
            "minecraft:block" => self.block_states.block_registry_id_by_name(resource_id),
            "minecraft:entity_type" => Some(
                bbb_protocol::entity_types::vanilla_entity_type_for_resource_id(resource_id)?.id,
            ),
            "minecraft:damage_type" => Some(vanilla_damage_type_for_resource_id(resource_id)?.id),
            "minecraft:mob_effect" => {
                Some(vanilla_mob_effect_type_for_resource_id(resource_id)?.id)
            }
            "minecraft:potion" => Some(vanilla_potion_for_resource_id(resource_id)?.id),
            "minecraft:sound_event" => vanilla_sound_event_registry_id(resource_id),
            "minecraft:world_clock" => {
                Some(vanilla_world_clock_type_by_resource_id(resource_id)?.id)
            }
            "minecraft:stat_type" => Some(vanilla_stat_type_by_resource_id(resource_id)?.id),
            "minecraft:custom_stat" => Some(vanilla_custom_stat_by_resource_id(resource_id)?.id),
            _ => None,
        }
    }
}

impl<'a> FrozenRegistryAccess<'a> {
    pub fn revision(self) -> RegistryAccessRevision {
        self.registries.revision
    }

    pub fn entry_by_holder_id(
        self,
        registry: &str,
        holder_id: i32,
    ) -> Option<FrozenRegistryEntry<'a>> {
        if let Some(contents) = self.registries.contents.get(registry) {
            let entry = contents.entry_by_holder_id(holder_id)?;
            return Some(FrozenRegistryEntry {
                holder_id,
                resource_id: entry.id.as_str(),
                source: RegistryEntrySource::Server,
                payload: RegistryEntryPayload::Server {
                    has_data: entry.has_data,
                    raw_data: entry.raw_data(),
                    nbt: entry.nbt.as_ref(),
                },
            });
        }
        let resource_id = self
            .registries
            .vanilla_fallback_resource_id(registry, holder_id)?;
        Some(FrozenRegistryEntry {
            holder_id,
            resource_id,
            source: RegistryEntrySource::VanillaFallback,
            payload: RegistryEntryPayload::VanillaFallback,
        })
    }

    pub fn entry_by_resource_id(
        self,
        registry: &str,
        resource_id: &str,
    ) -> Option<FrozenRegistryEntry<'a>> {
        let holder_id = self
            .registries
            .holder_id_by_resource_id(registry, resource_id)?;
        self.entry_by_holder_id(registry, holder_id)
    }

    pub fn holder_ids_for_tag(self, registry: &str, tag: &str) -> Option<&'a [i32]> {
        self.registries.holder_ids_for_tag(registry, tag)
    }

    pub fn unresolved_holder_ids_for_tag(self, registry: &str, tag: &str) -> Option<&'a [i32]> {
        self.registries.unresolved_holder_ids_for_tag(registry, tag)
    }
}

impl Default for RegistrySet {
    fn default() -> Self {
        Self::vanilla_26_1()
    }
}

impl RegistryPacketEntry {
    pub fn with_raw_data(id: impl Into<String>, raw_data: Vec<u8>) -> Self {
        let nbt = decode_registry_entry_nbt(raw_data.as_slice());
        Self {
            id: id.into(),
            has_data: true,
            raw_data_len: raw_data.len(),
            nbt,
            raw_data: Some(Arc::from(raw_data)),
        }
    }

    pub fn with_data_len(id: impl Into<String>, raw_data_len: usize) -> Self {
        Self {
            id: id.into(),
            has_data: true,
            raw_data_len,
            nbt: None,
            raw_data: None,
        }
    }

    pub fn stub(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            has_data: false,
            raw_data_len: 0,
            nbt: None,
            raw_data: None,
        }
    }

    pub fn raw_data(&self) -> Option<&[u8]> {
        self.raw_data.as_deref()
    }
}

impl RegistryContentState {
    pub fn new(registry: impl Into<String>) -> Self {
        Self {
            registry: registry.into(),
            packet_count: 0,
            entries: Vec::new(),
            duplicate_entry_ids: BTreeMap::new(),
        }
    }

    pub(super) fn append_packet(&mut self, entries: &[RegistryPacketEntry]) {
        self.packet_count += 1;
        for entry in entries {
            if self.entries.iter().any(|existing| existing.id == entry.id) {
                *self
                    .duplicate_entry_ids
                    .entry(entry.id.clone())
                    .or_insert(0) += 1;
            }
            self.entries.push(entry.clone());
        }
    }

    pub fn duplicate_entry_count(&self) -> usize {
        self.duplicate_entry_ids.values().sum()
    }

    pub fn entry_by_holder_id(&self, holder_id: i32) -> Option<&RegistryPacketEntry> {
        usize::try_from(holder_id)
            .ok()
            .and_then(|index| self.entries.get(index))
    }

    pub fn holder_id_by_resource_id(&self, resource_id: &str) -> Option<i32> {
        let mut matches = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.id == resource_id);
        let (index, _) = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        i32::try_from(index).ok()
    }
}

impl From<ProtocolRegistryDataEntry> for RegistryPacketEntry {
    fn from(entry: ProtocolRegistryDataEntry) -> Self {
        let ProtocolRegistryDataEntry { id, raw_data } = entry;
        let raw_data_len = raw_data.as_ref().map_or(0, Vec::len);
        let nbt = raw_data.as_deref().and_then(decode_registry_entry_nbt);
        Self {
            id,
            has_data: raw_data.is_some(),
            raw_data_len,
            nbt,
            raw_data: raw_data.map(Arc::from),
        }
    }
}

fn decode_registry_entry_nbt(raw_data: &[u8]) -> Option<NbtPayloadSummary> {
    decode_nbt_payload_summary(raw_data).ok().flatten()
}

impl RegistryPacket {
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn entries_with_data(&self) -> usize {
        self.entries.iter().filter(|entry| entry.has_data).count()
    }

    pub fn stub_entries(&self) -> usize {
        self.entries.len() - self.entries_with_data()
    }
}
