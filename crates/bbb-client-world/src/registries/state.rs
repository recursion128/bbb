use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::{Arc, OnceLock},
};

use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::packets::RegistryDataEntry as ProtocolRegistryDataEntry;
use bbb_protocol::registry_data_types::built_in_registry_type;
use bbb_protocol::static_registry_elements::vanilla_static_registry_elements;
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::registry_codec::{
    game_tests::{GameTestElementView, GameTestRegistryElement, TestInstanceView},
    sound_variants::SoundVariantElement,
    RegistryReferenceLookup, RegistryReferenceMembership,
};
use serde::{Deserialize, Serialize};

use crate::chunks::{decode_nbt_payload_summary, NbtPayloadSummary};

use super::dimension_types::DimensionTypeClientProjection;
#[cfg(test)]
use super::element_value::decode_known_pack_element;
use super::element_value::decode_network_element;
use super::enchantments::MaterializedEnchantmentDefinition;
use super::instrument_values::InstrumentPresentation;
use super::jukebox_songs::JukeboxSongPresentation;
use super::loader_order;
use super::painting_variants::PaintingVariantPresentation;
use super::timelines::TimelineDefinition;
use super::{
    BannerPatternElement, CatVariantPresentation, ChatTypeDecoration, DamageEffects,
    FarmAnimalVariantPresentation, FrogVariantPresentation, TrimMaterialPresentation,
    TrimPatternPresentation, WolfVariantPresentation, ZombieNautilusVariantPresentation,
};
use super::{
    BiomeClientProjection, BlockStateRegistry, RegistryElementData, RegistryElementDecodeError,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SessionContentRegistry(pub(crate) Arc<ContentRegistry>);

impl Default for SessionContentRegistry {
    fn default() -> Self {
        Self(Arc::clone(ContentRegistry::vanilla()))
    }
}

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
    pub(crate) content_registry: SessionContentRegistry,
    /// Typed values prepared against the same holder/tag generation as
    /// `contents`. This is replaced only by `RegistryRevisionCandidate`.
    #[serde(skip)]
    pub(super) typed: RegistryTypedValues,
    #[serde(skip)]
    pub(super) cached_bars_block_types: OnceLock<Arc<[i32]>>,
    #[serde(skip)]
    pub(super) pending: Option<PendingRegistryCollection>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct RegistryTypedValues {
    pub biomes: BTreeMap<String, BiomeClientProjection>,
    pub chat_types: BTreeMap<String, (ChatTypeDecoration, ChatTypeDecoration)>,
    pub damage_types: BTreeMap<String, DamageEffects>,
    pub dimension_types: BTreeMap<String, DimensionTypeClientProjection>,
    pub enchantments: BTreeMap<String, MaterializedEnchantmentDefinition>,
    pub instruments: BTreeMap<String, InstrumentPresentation>,
    pub jukebox_songs: BTreeMap<String, JukeboxSongPresentation>,
    pub painting_variants: BTreeMap<String, PaintingVariantPresentation>,
    pub trim_materials: BTreeMap<String, TrimMaterialPresentation>,
    pub trim_patterns: BTreeMap<String, TrimPatternPresentation>,
    pub banner_patterns: BTreeMap<String, BannerPatternElement>,
    pub farm_animal_variants: BTreeMap<String, BTreeMap<String, FarmAnimalVariantPresentation>>,
    pub cat_variants: BTreeMap<String, CatVariantPresentation>,
    pub frog_variants: BTreeMap<String, FrogVariantPresentation>,
    pub wolf_variants: BTreeMap<String, WolfVariantPresentation>,
    pub zombie_nautilus_variants: BTreeMap<String, ZombieNautilusVariantPresentation>,
    pub sound_variants: BTreeMap<String, BTreeMap<String, SoundVariantElement>>,
    pub test_instances: BTreeMap<String, GameTestRegistryElement>,
    pub timelines: BTreeMap<String, TimelineDefinition>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct PendingRegistryCollection {
    pub registries: Vec<RegistryPacket>,
    pub content_packet_indices: Vec<usize>,
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
    #[serde(default)]
    pub entries: Vec<RegistryPacketEntry>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct RegistryResourceIndex(pub(super) rustc_hash::FxHashMap<String, Option<i32>>);

impl PartialEq for RegistryResourceIndex {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for RegistryResourceIndex {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryContentState {
    pub registry: String,
    pub packet_count: usize,
    #[serde(default)]
    pub entries: Vec<RegistryPacketEntry>,
    #[serde(default)]
    pub duplicate_entry_ids: BTreeMap<String, usize>,
    #[serde(skip)]
    by_resource_id: RegistryResourceIndex,
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
    /// Direct-codec JSON supplied by a locally selected SERVER_DATA known pack
    /// when the server omits this entry's network payload.
    #[serde(skip)]
    pub known_pack_data: Option<Arc<[u8]>>,
    /// Source-neutral typed value materialized from either the network NBT or
    /// selected known-pack JSON before the registry revision is committed.
    #[serde(skip)]
    pub(crate) element_data: Option<Arc<RegistryElementData>>,
    #[serde(skip)]
    pub(crate) element_data_error: Option<RegistryElementDecodeError>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct RegistryTagIndexes {
    pub(super) populated: bool,
    pub(super) holder_sets: rustc_hash::FxHashMap<String, rustc_hash::FxHashSet<i32>>,
    pub(super) tags_by_holder: rustc_hash::FxHashMap<i32, Vec<String>>,
}

impl PartialEq for RegistryTagIndexes {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for RegistryTagIndexes {}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryTagState {
    pub registry: String,
    pub tags: BTreeMap<String, Vec<i32>>,
    #[serde(skip)]
    pub(super) indexes: RegistryTagIndexes,
}

impl RegistryTagState {
    pub(crate) fn from_materialized(registry: String, tags: BTreeMap<String, Vec<i32>>) -> Self {
        let mut holder_sets: rustc_hash::FxHashMap<String, rustc_hash::FxHashSet<i32>> =
            rustc_hash::FxHashMap::with_capacity_and_hasher(tags.len(), Default::default());
        let mut tags_by_holder: rustc_hash::FxHashMap<i32, Vec<String>> =
            rustc_hash::FxHashMap::default();
        for (tag, holder_ids) in &tags {
            let mut set = rustc_hash::FxHashSet::with_capacity_and_hasher(
                holder_ids.len(),
                Default::default(),
            );
            for &holder_id in holder_ids {
                if set.insert(holder_id) {
                    tags_by_holder
                        .entry(holder_id)
                        .or_default()
                        .push(tag.clone());
                }
            }
            holder_sets.insert(tag.clone(), set);
        }
        Self {
            registry,
            tags,
            indexes: RegistryTagIndexes {
                populated: true,
                holder_sets,
                tags_by_holder,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedRegistryTagHolder {
    pub registry: String,
    pub tag: String,
    pub holder_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryEntrySource {
    Server,
    VanillaFallback,
}

pub use bbb_registry_codec::RegistryAccessRevision;

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
pub enum FrozenRegistryMembership {
    Present,
    Absent,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrozenRegistryEntry<'a> {
    pub holder_id: i32,
    pub resource_id: &'a str,
    pub source: RegistryEntrySource,
    pub payload: RegistryEntryPayload<'a>,
    pub element_data: Option<&'a RegistryElementData>,
    pub element_data_error: Option<&'a RegistryElementDecodeError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryEntryPayload<'a> {
    Server {
        has_data: bool,
        raw_data: Option<&'a [u8]>,
        nbt: Option<&'a NbtPayloadSummary>,
        known_pack_data: Option<&'a [u8]>,
    },
    VanillaFallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenRegistryHolderSet<'a> {
    pub kind: FrozenRegistryHolderSetKind<'a>,
    pub entries: Vec<FrozenRegistryEntry<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryHolderSetIdentity {
    Direct { resource_ids: Vec<String> },
    Named { tag: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrozenRegistryHolderSetKind<'a> {
    Direct,
    Named {
        tag: &'a str,
        unresolved_holder_ids: &'a [i32],
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryResolutionError {
    MissingHolderId {
        registry: String,
        holder_id: i32,
    },
    MissingResourceId {
        registry: String,
        resource_id: String,
    },
    MissingTag {
        registry: String,
        tag: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistentHolderDecodeError {
    InvalidShape { registry: String },
    InvalidResourceId { registry: String, value: String },
    Resolution(RegistryResolutionError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistentHolderSetDecodeError {
    InvalidShape { registry: String },
    InvalidTag { registry: String, value: String },
    InvalidResourceId { registry: String, value: String },
    NonStringEntry { registry: String, index: usize },
    Resolution(RegistryResolutionError),
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
            content_registry: SessionContentRegistry::default(),
            typed: RegistryTypedValues::default(),
            cached_bars_block_types: OnceLock::new(),
            pending: None,
        }
    }

    pub fn block_state(&self, id: i32) -> Option<&'static super::BlockStateInfo> {
        self.block_states.by_id(id)
    }

    pub fn is_valid_block_state_id(&self, id: i32) -> bool {
        self.block_states
            .is_valid_state_id_with_registry(id, &self.content_registry.0)
    }

    pub fn block_state_name(&self, id: i32) -> Option<&str> {
        self.block_states
            .block_name_by_state_id_with_registry(id, &self.content_registry.0)
    }

    pub fn block_entity_type_id_for_block_state(&self, id: i32) -> Option<i32> {
        self.block_states.block_entity_type_id_by_state_id(id)
    }

    pub fn frozen_access(&self) -> FrozenRegistryAccess<'_> {
        FrozenRegistryAccess { registries: self }
    }

    pub fn block_name_by_registry_id(&self, id: i32) -> Option<&str> {
        self.block_states
            .block_name_by_registry_id_with_registry(id, &self.content_registry.0)
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

    pub(crate) fn maximum_numeric_block_property_value(
        &self,
        name: &str,
        property: &str,
    ) -> Option<i32> {
        self.block_states
            .maximum_numeric_block_property_value(name, property)
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

    pub fn biome_client_projection(&self, holder_id: i32) -> Option<&BiomeClientProjection> {
        let entry = self.entry_by_holder_id("minecraft:worldgen/biome", holder_id)?;
        self.typed.biomes.get(&entry.id)
    }

    pub(crate) fn dimension_type_client_projection(
        &self,
        holder_id: i32,
    ) -> Option<&DimensionTypeClientProjection> {
        let entry = self.entry_by_holder_id(super::DIMENSION_TYPE_REGISTRY, holder_id)?;
        self.typed.dimension_types.get(&entry.id)
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
    ) -> Option<impl ExactSizeIterator<Item = &'a str> + Clone + 'a> {
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

    pub(crate) fn bars_block_types(&self) -> Arc<[i32]> {
        Arc::clone(self.cached_bars_block_types.get_or_init(|| {
            self.holder_ids_for_tag("minecraft:block", "minecraft:bars")
                .map(Arc::from)
                .unwrap_or_else(|| Arc::from([]))
        }))
    }

    pub fn unresolved_holder_ids_for_tag(&self, registry: &str, tag: &str) -> Option<&[i32]> {
        self.unresolved_tag_holders
            .get(registry)?
            .get(tag)
            .map(Vec::as_slice)
    }

    pub fn holder_is_in_tag(&self, registry: &str, tag: &str, holder_id: i32) -> bool {
        let Some(tag_state) = self.tags.get(registry) else {
            return false;
        };
        if tag_state.indexes.populated {
            return tag_state
                .indexes
                .holder_sets
                .get(tag)
                .is_some_and(|set| set.contains(&holder_id));
        }
        tag_state
            .tags
            .get(tag)
            .is_some_and(|holder_ids| holder_ids.contains(&holder_id))
    }

    pub fn tag_names_containing_holder_id<'a>(
        &'a self,
        registry: &str,
        holder_id: i32,
    ) -> Option<impl Iterator<Item = &'a str> + 'a> {
        let tag_state = self.tags.get(registry)?;
        if tag_state.indexes.populated {
            let slice = tag_state
                .indexes
                .tags_by_holder
                .get(&holder_id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            Some(TagNamesContainingHolderIter::Indexed(slice.iter()))
        } else {
            Some(TagNamesContainingHolderIter::Fallback {
                iter: tag_state.tags.iter(),
                holder_id,
            })
        }
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

    // RegistryDataLoader.java:83-129; RegistryDumpReport.java:37-45.
    fn vanilla_fallback_resource_id(&self, registry: &str, holder_id: i32) -> Option<&str> {
        if loader_order::is_loader_ordered_registry(registry) {
            return loader_order::loader_ordered_resource_id(registry, holder_id);
        }
        let index = usize::try_from(holder_id).ok()?;
        if let Some(id) = vanilla_static_registry_elements(registry)
            .and_then(|elements| elements.get(index).copied())
        {
            return Some(id);
        }
        self.mod_registry_resource_id(registry, holder_id)
    }

    fn vanilla_fallback_holder_id(&self, registry: &str, resource_id: &str) -> Option<i32> {
        if loader_order::is_loader_ordered_registry(registry) {
            return loader_order::loader_ordered_holder_id(registry, resource_id);
        }
        if let Some(index) = vanilla_static_registry_elements(registry)
            .and_then(|elements| elements.iter().position(|entry| *entry == resource_id))
        {
            return i32::try_from(index).ok();
        }
        self.mod_registry_holder_id(registry, resource_id)
    }

    fn mod_registry_resource_id(&self, registry: &str, holder_id: i32) -> Option<&str> {
        let reg = &self.content_registry.0;
        if reg.is_vanilla_only() {
            return None;
        }
        match registry {
            "minecraft:entity_type" => reg.entity_by_id(holder_id).map(|e| e.resource_id()),
            "minecraft:item" => reg.item_by_id(holder_id).map(|e| e.resource_id()),
            "minecraft:block" => reg.block_by_id(holder_id).map(|e| e.resource_id()),
            "minecraft:sound_event" => reg.sound_event_by_id(holder_id).map(|e| e.resource_id()),
            "minecraft:particle_type" => reg.particle_by_id(holder_id).map(|e| e.resource_id()),
            _ => None,
        }
    }

    fn mod_registry_holder_id(&self, registry: &str, resource_id: &str) -> Option<i32> {
        let reg = &self.content_registry.0;
        if reg.is_vanilla_only() {
            return None;
        }
        match registry {
            "minecraft:entity_type" => reg.entity_by_resource_id(resource_id).map(|e| e.id),
            "minecraft:item" => reg.item_by_resource_id(resource_id).map(|e| e.id),
            "minecraft:block" => reg.block_by_resource_id(resource_id).map(|e| e.block_id),
            "minecraft:sound_event" => reg.sound_event_by_resource_id(resource_id).map(|e| e.id),
            "minecraft:particle_type" => reg.particle_by_resource_id(resource_id).map(|e| e.id),
            _ => None,
        }
    }
}

impl<'a> FrozenRegistryAccess<'a> {
    pub(crate) fn content_registry(self) -> &'a ContentRegistry {
        &self.registries.content_registry.0
    }

    pub fn revision(self) -> RegistryAccessRevision {
        self.registries.revision
    }

    pub(super) fn typed_chat_type(
        self,
        resource_id: &str,
    ) -> Option<&'a (ChatTypeDecoration, ChatTypeDecoration)> {
        self.registries.typed.chat_types.get(resource_id)
    }

    pub(crate) fn typed_test_instance(self, resource_id: &str) -> Option<TestInstanceView<'a>> {
        let element = self.registries.typed.test_instances.get(resource_id)?;
        let GameTestElementView::Instance(instance) = element.view() else {
            unreachable!("published test instance registry value");
        };
        Some(instance)
    }

    pub fn has_server_contents(self, registry: &str) -> bool {
        self.registries.contents.contains_key(registry)
    }

    pub fn resource_membership(
        self,
        registry: &str,
        resource_id: &str,
    ) -> FrozenRegistryMembership {
        if let Some(contents) = self.registries.contents.get(registry) {
            return if contents.holder_id_by_resource_id(resource_id).is_some() {
                FrozenRegistryMembership::Present
            } else {
                FrozenRegistryMembership::Absent
            };
        }
        let mut packets = self
            .registries
            .registries
            .iter()
            .filter(|packet| packet.name == registry)
            .peekable();
        if packets.peek().is_some() {
            return if packets
                .any(|packet| packet.entries.iter().any(|entry| entry.id == resource_id))
            {
                FrozenRegistryMembership::Present
            } else {
                FrozenRegistryMembership::Absent
            };
        }
        if self
            .registries
            .vanilla_fallback_holder_id(registry, resource_id)
            .is_some()
        {
            FrozenRegistryMembership::Present
        } else if registry_has_exact_vanilla_fallback(registry) {
            FrozenRegistryMembership::Absent
        } else {
            FrozenRegistryMembership::Unavailable
        }
    }

    pub fn tag_membership(self, registry: &str, tag: &str) -> FrozenRegistryMembership {
        match self.registries.tags.get(registry) {
            Some(tags) if tags.tags.contains_key(tag) => FrozenRegistryMembership::Present,
            Some(_) => FrozenRegistryMembership::Absent,
            None => FrozenRegistryMembership::Unavailable,
        }
    }

    pub fn tag_ids(self, registry: &str) -> Option<Vec<&'a str>> {
        Some(
            self.registries
                .tags
                .get(registry)?
                .tags
                .keys()
                .map(String::as_str)
                .collect(),
        )
    }

    pub fn resource_ids_for_tag(self, registry: &str, tag: &str) -> Option<Vec<&'a str>> {
        self.registries
            .resource_ids_for_tag(registry, tag)
            .ok()
            .flatten()
    }

    pub fn block_property_values(self, resource_id: &str) -> BTreeMap<&'a str, BTreeSet<&'a str>> {
        self.registries
            .block_states
            .block_property_values(resource_id)
    }

    pub fn block_properties_membership(
        self,
        resource_id: &str,
        properties: &BTreeMap<String, String>,
    ) -> FrozenRegistryMembership {
        match self.resource_membership("minecraft:block", resource_id) {
            FrozenRegistryMembership::Present => {}
            membership => return membership,
        }
        if self
            .registries
            .block_states
            .block_registry_id_by_name(resource_id)
            .is_none()
        {
            return FrozenRegistryMembership::Unavailable;
        }
        if self
            .registries
            .block_states
            .block_matches_properties(resource_id, properties)
        {
            FrozenRegistryMembership::Present
        } else {
            FrozenRegistryMembership::Absent
        }
    }

    pub fn resource_ids_in_holder_order(self, registry: &str) -> Option<Vec<&'a str>> {
        if let Some(contents) = self.registries.contents.get(registry) {
            return Some(
                contents
                    .entries
                    .iter()
                    .map(|entry| entry.id.as_str())
                    .collect(),
            );
        }
        let mut packets = self
            .registries
            .registries
            .iter()
            .filter(|packet| packet.name == registry)
            .peekable();
        if packets.peek().is_some() {
            return Some(
                packets
                    .flat_map(|packet| packet.entries.iter().map(|entry| entry.id.as_str()))
                    .collect(),
            );
        }
        if !registry_has_exact_vanilla_fallback(registry) {
            return None;
        }
        Some(
            (0..)
                .map_while(|holder_id| {
                    self.registries
                        .vanilla_fallback_resource_id(registry, holder_id)
                })
                .collect(),
        )
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
                    known_pack_data: entry.known_pack_data(),
                },
                element_data: entry.element_data(),
                element_data_error: entry.element_data_error.as_ref(),
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
            element_data: None,
            element_data_error: None,
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

    pub fn require_entry_by_holder_id(
        self,
        registry: &str,
        holder_id: i32,
    ) -> Result<FrozenRegistryEntry<'a>, RegistryResolutionError> {
        self.entry_by_holder_id(registry, holder_id).ok_or_else(|| {
            RegistryResolutionError::MissingHolderId {
                registry: registry.to_string(),
                holder_id,
            }
        })
    }

    pub fn require_entry_by_resource_id(
        self,
        registry: &str,
        resource_id: &str,
    ) -> Result<FrozenRegistryEntry<'a>, RegistryResolutionError> {
        self.entry_by_resource_id(registry, resource_id)
            .ok_or_else(|| RegistryResolutionError::MissingResourceId {
                registry: registry.to_string(),
                resource_id: resource_id.to_string(),
            })
    }

    pub fn holder_set_by_holder_ids(
        self,
        registry: &str,
        holder_ids: &[i32],
    ) -> Result<FrozenRegistryHolderSet<'a>, RegistryResolutionError> {
        let entries = holder_ids
            .iter()
            .map(|holder_id| self.require_entry_by_holder_id(registry, *holder_id))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(FrozenRegistryHolderSet {
            kind: FrozenRegistryHolderSetKind::Direct,
            entries,
        })
    }

    pub fn holder_set_by_resource_ids(
        self,
        registry: &str,
        resource_ids: &[String],
    ) -> Result<FrozenRegistryHolderSet<'a>, RegistryResolutionError> {
        let entries = resource_ids
            .iter()
            .map(|resource_id| self.require_entry_by_resource_id(registry, resource_id))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(FrozenRegistryHolderSet {
            kind: FrozenRegistryHolderSetKind::Direct,
            entries,
        })
    }

    pub fn holder_set_by_tag(
        self,
        registry: &str,
        tag: &str,
    ) -> Result<FrozenRegistryHolderSet<'a>, RegistryResolutionError> {
        let (stored_registry, tags) =
            self.registries
                .tags
                .get_key_value(registry)
                .ok_or_else(|| RegistryResolutionError::MissingTag {
                    registry: registry.to_string(),
                    tag: tag.to_string(),
                })?;
        let (stored_tag, holder_ids) =
            tags.tags
                .get_key_value(tag)
                .ok_or_else(|| RegistryResolutionError::MissingTag {
                    registry: registry.to_string(),
                    tag: tag.to_string(),
                })?;
        let entries = self
            .holder_set_by_holder_ids(stored_registry, holder_ids)?
            .entries;
        Ok(FrozenRegistryHolderSet {
            kind: FrozenRegistryHolderSetKind::Named {
                tag: stored_tag,
                unresolved_holder_ids: self
                    .unresolved_holder_ids_for_tag(stored_registry, stored_tag)
                    .unwrap_or_default(),
            },
            entries,
        })
    }

    pub fn persistent_holder_from_nbt(
        self,
        registry: &str,
        value: &NbtSummaryValue,
    ) -> Result<FrozenRegistryEntry<'a>, PersistentHolderDecodeError> {
        let NbtSummaryValue::String(raw) = value else {
            return Err(PersistentHolderDecodeError::InvalidShape {
                registry: registry.to_string(),
            });
        };
        let resource_id = raw
            .with_utf8(super::normalize_codec_identifier)
            .flatten()
            .ok_or_else(|| PersistentHolderDecodeError::InvalidResourceId {
                registry: registry.to_string(),
                value: raw.to_display_string(),
            })?;
        self.require_entry_by_resource_id(registry, &resource_id)
            .map_err(PersistentHolderDecodeError::Resolution)
    }

    pub fn persistent_holder_set_from_nbt(
        self,
        registry: &str,
        value: &NbtSummaryValue,
    ) -> Result<FrozenRegistryHolderSet<'a>, PersistentHolderSetDecodeError> {
        match value {
            NbtSummaryValue::String(raw) if raw.units().starts_with(&[u16::from(b'#')]) => {
                let tag = raw
                    .with_utf8(|raw| super::normalize_codec_identifier(&raw[1..]))
                    .flatten()
                    .ok_or_else(|| PersistentHolderSetDecodeError::InvalidTag {
                        registry: registry.to_string(),
                        value: raw.to_display_string(),
                    })?;
                self.holder_set_by_tag(registry, &tag)
                    .map_err(PersistentHolderSetDecodeError::Resolution)
            }
            NbtSummaryValue::String(_) => {
                let entry = self
                    .persistent_holder_from_nbt(registry, value)
                    .map_err(PersistentHolderSetDecodeError::from)?;
                Ok(FrozenRegistryHolderSet {
                    kind: FrozenRegistryHolderSetKind::Direct,
                    entries: vec![entry],
                })
            }
            NbtSummaryValue::List(values) => {
                let resource_ids = values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        let NbtSummaryValue::String(raw) = value else {
                            return Err(PersistentHolderSetDecodeError::NonStringEntry {
                                registry: registry.to_string(),
                                index,
                            });
                        };
                        raw.with_utf8(super::normalize_codec_identifier)
                            .flatten()
                            .ok_or_else(|| PersistentHolderSetDecodeError::InvalidResourceId {
                                registry: registry.to_string(),
                                value: raw.to_display_string(),
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                self.holder_set_by_resource_ids(registry, &resource_ids)
                    .map_err(PersistentHolderSetDecodeError::Resolution)
            }
            _ => Err(PersistentHolderSetDecodeError::InvalidShape {
                registry: registry.to_string(),
            }),
        }
    }

    pub fn holder_set_by_identity(
        self,
        registry: &str,
        identity: &RegistryHolderSetIdentity,
    ) -> Result<FrozenRegistryHolderSet<'a>, RegistryResolutionError> {
        match identity {
            RegistryHolderSetIdentity::Direct { resource_ids } => {
                self.holder_set_by_resource_ids(registry, resource_ids)
            }
            RegistryHolderSetIdentity::Named { tag } => self.holder_set_by_tag(registry, tag),
        }
    }

    pub fn holder_ids_for_tag(self, registry: &str, tag: &str) -> Option<&'a [i32]> {
        self.registries.holder_ids_for_tag(registry, tag)
    }

    pub fn unresolved_holder_ids_for_tag(self, registry: &str, tag: &str) -> Option<&'a [i32]> {
        self.registries.unresolved_holder_ids_for_tag(registry, tag)
    }
}

impl RegistryReferenceLookup for FrozenRegistryAccess<'_> {
    fn registry_available(&self, registry: &str) -> bool {
        if built_in_registry_type(registry).is_some() {
            registry_has_exact_vanilla_fallback(registry)
        } else {
            self.registries.contents.contains_key(registry)
                || self
                    .registries
                    .registries
                    .iter()
                    .any(|packet| packet.name == registry)
                || registry_has_exact_vanilla_fallback(registry)
        }
    }

    fn element_membership(&self, registry: &str, resource_id: &str) -> RegistryReferenceMembership {
        if !RegistryReferenceLookup::registry_available(self, registry) {
            RegistryReferenceMembership::Unavailable
        } else if self.contains_element(registry, resource_id) {
            RegistryReferenceMembership::Present
        } else {
            RegistryReferenceMembership::Absent
        }
    }

    fn tag_membership(&self, registry: &str, resource_id: &str) -> RegistryReferenceMembership {
        match FrozenRegistryAccess::tag_membership(*self, registry, resource_id) {
            FrozenRegistryMembership::Present => RegistryReferenceMembership::Present,
            FrozenRegistryMembership::Absent => RegistryReferenceMembership::Absent,
            FrozenRegistryMembership::Unavailable => RegistryReferenceMembership::Unavailable,
        }
    }

    fn contains_element(&self, registry: &str, resource_id: &str) -> bool {
        // RegistryDataLoader creates candidate element owners only for its
        // data-loaded registries. BuiltInRegistries remain loading context:
        // configuration may replace their tags, but a RegistryData packet
        // cannot add a fixed-registry element such as a sound event.
        if built_in_registry_type(registry).is_some() {
            return self
                .registries
                .vanilla_fallback_holder_id(registry, resource_id)
                .is_some();
        }
        matches!(
            self.resource_membership(registry, resource_id),
            FrozenRegistryMembership::Present
        )
    }

    fn contains_tag(&self, registry: &str, resource_id: &str) -> bool {
        matches!(
            FrozenRegistryAccess::tag_membership(*self, registry, resource_id),
            FrozenRegistryMembership::Present
        )
    }
}

impl bbb_protocol::PersistentRegistryResolver for FrozenRegistryAccess<'_> {
    fn registry_available(&self, registry: &str) -> bool {
        RegistryReferenceLookup::registry_available(self, registry)
    }

    fn resource_membership(
        &self,
        registry: &str,
        resource_id: &str,
    ) -> bbb_protocol::PersistentRegistryMembership {
        RegistryReferenceLookup::element_membership(self, registry, resource_id)
    }

    fn tag_members(
        &self,
        registry: &str,
        tag_id: &str,
    ) -> bbb_protocol::PersistentRegistryTagMembers {
        match FrozenRegistryAccess::tag_membership(*self, registry, tag_id) {
            FrozenRegistryMembership::Present => {
                match self.resource_ids_for_tag(registry, tag_id) {
                    Some(members) => bbb_protocol::PersistentRegistryTagMembers::Present(
                        members.into_iter().map(str::to_owned).collect(),
                    ),
                    None => bbb_protocol::PersistentRegistryTagMembers::Unavailable,
                }
            }
            FrozenRegistryMembership::Absent => bbb_protocol::PersistentRegistryTagMembers::Absent,
            FrozenRegistryMembership::Unavailable => {
                bbb_protocol::PersistentRegistryTagMembers::Unavailable
            }
        }
    }
}

impl bbb_registry_codec::PersistentItemStackHolderResolver for FrozenRegistryAccess<'_> {
    fn holder_id(&self, registry: &str, resource_id: &str) -> Option<i32> {
        self.entry_by_resource_id(registry, resource_id)
            .map(|entry| entry.holder_id)
    }

    fn holder_resource_id(&self, registry: &str, holder_id: i32) -> Option<String> {
        self.entry_by_holder_id(registry, holder_id)
            .map(|entry| entry.resource_id.to_owned())
    }

    fn tag_holder_ids(&self, registry: &str, tag: &str) -> Result<Vec<i32>, String> {
        self.holder_set_by_tag(registry, tag)
            .map_err(|error| error.to_string())
            .map(|set| {
                set.entries
                    .into_iter()
                    .map(|entry| entry.holder_id)
                    .collect()
            })
    }

    fn content_registry(&self) -> &ContentRegistry {
        FrozenRegistryAccess::content_registry(*self)
    }
}

impl bbb_protocol::PersistentRegistryResolver for RegistrySet {
    fn registry_available(&self, registry: &str) -> bool {
        bbb_protocol::PersistentRegistryResolver::registry_available(
            &self.frozen_access(),
            registry,
        )
    }

    fn resource_membership(
        &self,
        registry: &str,
        resource_id: &str,
    ) -> bbb_protocol::PersistentRegistryMembership {
        bbb_protocol::PersistentRegistryResolver::resource_membership(
            &self.frozen_access(),
            registry,
            resource_id,
        )
    }

    fn tag_members(
        &self,
        registry: &str,
        tag_id: &str,
    ) -> bbb_protocol::PersistentRegistryTagMembers {
        bbb_protocol::PersistentRegistryResolver::tag_members(
            &self.frozen_access(),
            registry,
            tag_id,
        )
    }
}

impl bbb_registry_codec::PersistentItemStackHolderResolver for RegistrySet {
    fn holder_id(&self, registry: &str, resource_id: &str) -> Option<i32> {
        self.frozen_access().holder_id(registry, resource_id)
    }

    fn holder_resource_id(&self, registry: &str, holder_id: i32) -> Option<String> {
        self.frozen_access().holder_resource_id(registry, holder_id)
    }

    fn tag_holder_ids(&self, registry: &str, tag: &str) -> Result<Vec<i32>, String> {
        self.frozen_access().tag_holder_ids(registry, tag)
    }

    fn content_registry(&self) -> &ContentRegistry {
        &self.content_registry.0
    }
}

fn registry_has_static_vanilla_fallback(registry: &str) -> bool {
    vanilla_static_registry_elements(registry).is_some()
}

impl FrozenRegistryHolderSet<'_> {
    pub fn identity(&self) -> RegistryHolderSetIdentity {
        match self.kind {
            FrozenRegistryHolderSetKind::Direct => RegistryHolderSetIdentity::Direct {
                resource_ids: self
                    .entries
                    .iter()
                    .map(|entry| entry.resource_id.to_string())
                    .collect(),
            },
            FrozenRegistryHolderSetKind::Named { tag, .. } => RegistryHolderSetIdentity::Named {
                tag: tag.to_string(),
            },
        }
    }
}

pub(super) fn registry_has_exact_vanilla_fallback(registry: &str) -> bool {
    registry_has_static_vanilla_fallback(registry)
        || loader_order::loader_ordered_registry_available(registry)
}

impl fmt::Display for RegistryResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHolderId {
                registry,
                holder_id,
            } => write!(formatter, "missing {registry} holder {holder_id}"),
            Self::MissingResourceId {
                registry,
                resource_id,
            } => write!(formatter, "missing {registry} resource {resource_id}"),
            Self::MissingTag { registry, tag } => {
                write!(formatter, "missing {registry} tag #{tag}")
            }
        }
    }
}

impl std::error::Error for RegistryResolutionError {}

impl fmt::Display for PersistentHolderDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidShape { registry } => write!(
                formatter,
                "{registry} persistent holder is not a resource id"
            ),
            Self::InvalidResourceId { registry, value } => write!(
                formatter,
                "invalid {registry} persistent holder resource id {value}"
            ),
            Self::Resolution(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for PersistentHolderDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Resolution(error) => Some(error),
            _ => None,
        }
    }
}

impl From<PersistentHolderDecodeError> for PersistentHolderSetDecodeError {
    fn from(error: PersistentHolderDecodeError) -> Self {
        match error {
            PersistentHolderDecodeError::InvalidShape { registry } => {
                Self::InvalidShape { registry }
            }
            PersistentHolderDecodeError::InvalidResourceId { registry, value } => {
                Self::InvalidResourceId { registry, value }
            }
            PersistentHolderDecodeError::Resolution(error) => Self::Resolution(error),
        }
    }
}

impl fmt::Display for PersistentHolderSetDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidShape { registry } => write!(
                formatter,
                "{registry} persistent holder set is neither a tag, resource id, nor list"
            ),
            Self::InvalidTag { registry, value } => {
                write!(
                    formatter,
                    "invalid {registry} persistent holder-set tag {value}"
                )
            }
            Self::InvalidResourceId { registry, value } => write!(
                formatter,
                "invalid {registry} persistent holder resource id {value}"
            ),
            Self::NonStringEntry { registry, index } => write!(
                formatter,
                "{registry} persistent holder-set entry {index} is not a resource id"
            ),
            Self::Resolution(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for PersistentHolderSetDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Resolution(error) => Some(error),
            _ => None,
        }
    }
}

impl Default for RegistrySet {
    fn default() -> Self {
        Self::vanilla_26_1()
    }
}

impl RegistryPacketEntry {
    pub fn with_raw_data(id: impl Into<String>, raw_data: Vec<u8>) -> Self {
        let (nbt, element_data, element_data_error) = match decode_network_element(&raw_data) {
            Ok(data) => (
                Some(NbtPayloadSummary {
                    root_type: raw_data[0],
                    byte_len: raw_data.len(),
                }),
                Some(Arc::new(data)),
                None,
            ),
            Err(error) => (
                decode_nbt_payload_summary(&raw_data).ok().flatten(),
                None,
                Some(error),
            ),
        };
        Self {
            id: id.into(),
            has_data: true,
            raw_data_len: raw_data.len(),
            nbt,
            raw_data: Some(Arc::from(raw_data)),
            known_pack_data: None,
            element_data,
            element_data_error,
        }
    }

    pub fn with_data_len(id: impl Into<String>, raw_data_len: usize) -> Self {
        Self {
            id: id.into(),
            has_data: true,
            raw_data_len,
            nbt: None,
            raw_data: None,
            known_pack_data: None,
            element_data: None,
            element_data_error: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_known_pack_data_for_test(
        id: impl Into<String>,
        known_pack_data: Vec<u8>,
    ) -> Self {
        let (element_data, element_data_error) = match decode_known_pack_element(&known_pack_data) {
            Ok(data) => (Some(Arc::new(data)), None),
            Err(error) => (None, Some(error)),
        };
        Self {
            id: id.into(),
            has_data: false,
            raw_data_len: 0,
            nbt: None,
            raw_data: None,
            known_pack_data: Some(Arc::from(known_pack_data)),
            element_data,
            element_data_error,
        }
    }

    pub fn stub(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            has_data: false,
            raw_data_len: 0,
            nbt: None,
            raw_data: None,
            known_pack_data: None,
            element_data: None,
            element_data_error: None,
        }
    }

    pub fn raw_data(&self) -> Option<&[u8]> {
        self.raw_data.as_deref()
    }

    pub fn known_pack_data(&self) -> Option<&[u8]> {
        self.known_pack_data.as_deref()
    }

    pub fn element_data(&self) -> Option<&RegistryElementData> {
        self.element_data.as_deref()
    }
}

enum TagNamesContainingHolderIter<'a> {
    Indexed(std::slice::Iter<'a, String>),
    Fallback {
        iter: std::collections::btree_map::Iter<'a, String, Vec<i32>>,
        holder_id: i32,
    },
}

impl<'a> Iterator for TagNamesContainingHolderIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Indexed(iter) => iter.next().map(String::as_str),
            Self::Fallback { iter, holder_id } => {
                for (tag, holder_ids) in iter.by_ref() {
                    if holder_ids.contains(holder_id) {
                        return Some(tag.as_str());
                    }
                }
                None
            }
        }
    }
}

impl RegistryContentState {
    pub fn new(registry: impl Into<String>) -> Self {
        Self {
            registry: registry.into(),
            packet_count: 0,
            entries: Vec::new(),
            duplicate_entry_ids: BTreeMap::new(),
            by_resource_id: RegistryResourceIndex::default(),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_entries(
        registry: impl Into<String>,
        entries: Vec<RegistryPacketEntry>,
    ) -> Self {
        let mut state = Self::new(registry);
        state.append_packet(&entries);
        state
    }

    fn ensure_resource_index(&mut self) {
        if self.by_resource_id.0.is_empty() && !self.entries.is_empty() {
            self.by_resource_id.0.reserve(self.entries.len());
            for (index, entry) in self.entries.iter().enumerate() {
                let holder_id = i32::try_from(index).ok();
                self.by_resource_id
                    .0
                    .entry(entry.id.clone())
                    .and_modify(|slot| *slot = None)
                    .or_insert(holder_id);
            }
        }
    }

    pub(super) fn append_packet(&mut self, entries: &[RegistryPacketEntry]) {
        const MAX_ENTRIES_PER_REGISTRY: usize = 65_536;
        self.packet_count += 1;
        self.ensure_resource_index();
        let remaining_capacity = MAX_ENTRIES_PER_REGISTRY.saturating_sub(self.entries.len());
        let accepted = &entries[..entries.len().min(remaining_capacity)];
        self.entries.reserve(accepted.len());
        self.by_resource_id.0.reserve(accepted.len());
        for entry in accepted {
            let holder_id = i32::try_from(self.entries.len()).ok();
            match self.by_resource_id.0.entry(entry.id.clone()) {
                std::collections::hash_map::Entry::Occupied(mut occupied) => {
                    occupied.insert(None);
                    *self
                        .duplicate_entry_ids
                        .entry(entry.id.clone())
                        .or_insert(0) += 1;
                }
                std::collections::hash_map::Entry::Vacant(vacant) => {
                    vacant.insert(holder_id);
                }
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
        if !self.by_resource_id.0.is_empty() || self.entries.is_empty() {
            return self.by_resource_id.0.get(resource_id).copied().flatten();
        }
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
        match entry.raw_data {
            Some(raw_data) => Self::with_raw_data(entry.id, raw_data),
            None => Self::stub(entry.id),
        }
    }
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
