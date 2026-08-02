use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::Arc,
};

use bbb_protocol::attribute_types::{
    vanilla_attribute_type, vanilla_attribute_type_for_resource_id,
};
use bbb_protocol::damage_types::{vanilla_damage_type, vanilla_damage_type_for_resource_id};
use bbb_protocol::data_component_types::{
    vanilla_data_component_predicate_type, vanilla_data_component_type,
    vanilla_data_component_type_for_resource_id, VANILLA_DATA_COMPONENT_PREDICATE_TYPES,
};
use bbb_protocol::debug_subscription_types::{
    vanilla_debug_game_event_type_by_id, VANILLA_DEBUG_GAME_EVENT_TYPES,
};
use bbb_protocol::mob_effect_types::{
    vanilla_mob_effect_type, vanilla_mob_effect_type_for_resource_id,
};
use bbb_protocol::packets::RegistryDataEntry as ProtocolRegistryDataEntry;
use bbb_protocol::particle_types::{
    vanilla_particle_type, vanilla_particle_type_id_for_resource_id,
};
use bbb_protocol::potion_types::{vanilla_potion, vanilla_potion_for_resource_id};
use bbb_protocol::registry_data_types::{VanillaChatTypeIdentity, VanillaFluidIdentity};
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
use bbb_protocol::{
    entity_types::vanilla_entity_type, item_types::vanilla_item_type, NbtSummaryValue,
};
use bbb_render_types::{
    cat_variants::CatModelVariant,
    temperature_animal_variants::{
        ChickenModelVariant, CowModelVariant, FrogModelVariant, PigModelVariant,
    },
    wolf_variants::WolfModelVariant,
    zombie_nautilus_variants::ZombieNautilusModelVariant,
    VillagerModelProfession, VillagerModelType,
};
use serde::{Deserialize, Serialize};

use crate::chunks::{decode_nbt_payload_summary, NbtPayloadSummary};
use crate::painting_variants::{
    vanilla_painting_variant, vanilla_painting_variant_for_resource_id,
};
use crate::{
    CatSoundVariantIdentity, ChickenSoundVariantIdentity, CowSoundVariantIdentity,
    PigSoundVariantIdentity, WolfSoundVariantIdentity,
};

use super::biomes::decode_biome_client_projection;
use super::dimension_types::{
    dimension_type_client_projection_from_registry, DimensionTypeClientProjection,
};
#[cfg(test)]
use super::element_value::decode_known_pack_element;
use super::element_value::decode_network_element;
use super::enchantments::MaterializedEnchantmentDefinition;
use super::game_tests::{GameTestInstanceDefinition, TestEnvironmentDefinition};
use super::instrument_values::InstrumentPresentation;
use super::jukebox_songs::JukeboxSongPresentation;
use super::painting_variants::PaintingVariantPresentation;
use super::sound_variants::RegistrySoundVariant;
use super::timelines::{TimelineDefinition, WorldClockDefinition};
use super::{
    BannerPatternPresentation, CatVariantPresentation, ChatTypeDecoration, DamageTypePresentation,
    FarmAnimalVariantPresentation, FrogVariantPresentation, TrimMaterialPresentation,
    TrimPatternPresentation, WolfVariantPresentation, ZombieNautilusVariantPresentation,
};
use super::{
    BiomeClientProjection, BiomeProjectionError, BlockStateRegistry, RegistryElementData,
    RegistryElementDecodeError,
};

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
    /// Typed values prepared against the same holder/tag generation as
    /// `contents`. This is replaced only by `RegistryRevisionCandidate`.
    #[serde(skip)]
    pub(super) typed: RegistryTypedValues,
    #[serde(skip)]
    pub(super) pending: Option<PendingRegistryCollection>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct RegistryTypedValues {
    pub biomes: BTreeMap<String, BiomeClientProjection>,
    pub chat_types: BTreeMap<String, (ChatTypeDecoration, ChatTypeDecoration)>,
    pub damage_types: BTreeMap<String, DamageTypePresentation>,
    pub dimension_types: BTreeMap<String, DimensionTypeClientProjection>,
    pub enchantments: BTreeMap<String, MaterializedEnchantmentDefinition>,
    pub instruments: BTreeMap<String, InstrumentPresentation>,
    pub jukebox_songs: BTreeMap<String, JukeboxSongPresentation>,
    pub painting_variants: BTreeMap<String, PaintingVariantPresentation>,
    pub trim_materials: BTreeMap<String, TrimMaterialPresentation>,
    pub trim_patterns: BTreeMap<String, TrimPatternPresentation>,
    pub banner_patterns: BTreeMap<String, BannerPatternPresentation>,
    pub farm_animal_variants: BTreeMap<String, BTreeMap<String, FarmAnimalVariantPresentation>>,
    pub cat_variants: BTreeMap<String, CatVariantPresentation>,
    pub frog_variants: BTreeMap<String, FrogVariantPresentation>,
    pub wolf_variants: BTreeMap<String, WolfVariantPresentation>,
    pub zombie_nautilus_variants: BTreeMap<String, ZombieNautilusVariantPresentation>,
    pub sound_variants: BTreeMap<String, BTreeMap<String, RegistrySoundVariant>>,
    pub test_environments: BTreeMap<String, TestEnvironmentDefinition>,
    pub test_instances: BTreeMap<String, GameTestInstanceDefinition>,
    pub timelines: BTreeMap<String, TimelineDefinition>,
    pub world_clocks: BTreeMap<String, WorldClockDefinition>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
pub(crate) enum PersistentHolderOrInline<'a> {
    Named(FrozenRegistryEntry<'a>),
    Direct(&'a NbtSummaryValue),
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
            typed: RegistryTypedValues::default(),
            pending: None,
        }
    }

    pub fn block_state(&self, id: i32) -> Option<&super::BlockStateInfo> {
        self.block_states.by_id(id)
    }

    pub fn block_entity_type_id_for_block_state(&self, id: i32) -> Option<i32> {
        self.block_states.block_entity_type_id_by_state_id(id)
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

    pub fn biome_client_projection(
        &self,
        holder_id: i32,
    ) -> Result<Option<BiomeClientProjection>, BiomeProjectionError> {
        let Some(entry) = self.entry_by_holder_id("minecraft:worldgen/biome", holder_id) else {
            return Ok(None);
        };
        if let Some(projection) = self.typed.biomes.get(&entry.id) {
            return Ok(Some(projection.clone()));
        }
        decode_biome_client_projection(entry, self.frozen_access())
    }

    pub(crate) fn dimension_type_client_projection(
        &self,
        holder_id: i32,
    ) -> Option<DimensionTypeClientProjection> {
        self.dimension_type_client_projection_ref(holder_id)
            .map(Cow::into_owned)
    }

    pub(crate) fn dimension_type_client_projection_ref(
        &self,
        holder_id: i32,
    ) -> Option<Cow<'_, DimensionTypeClientProjection>> {
        let entry = self.entry_by_holder_id(super::DIMENSION_TYPE_REGISTRY, holder_id)?;
        self.typed
            .dimension_types
            .get(&entry.id)
            .map(Cow::Borrowed)
            .or_else(|| {
                // Direct insertion is retained for focused fixtures. Real
                // configuration revisions always take the predecoded branch.
                dimension_type_client_projection_from_registry(Some(entry), self.frozen_access())
                    .map(Cow::Owned)
            })
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
            "minecraft:chat_type" => {
                Some(VanillaChatTypeIdentity::for_vanilla_registry_id(holder_id)?.resource_id())
            }
            "minecraft:item" => Some(vanilla_item_type(holder_id)?.resource_id),
            "minecraft:block" => self.block_states.block_name_by_registry_id(holder_id),
            "minecraft:fluid" => {
                Some(VanillaFluidIdentity::for_vanilla_registry_id(holder_id)?.resource_id())
            }
            "minecraft:entity_type" => Some(vanilla_entity_type(holder_id)?.resource_id),
            "minecraft:damage_type" => Some(vanilla_damage_type(holder_id)?.resource_id),
            "minecraft:game_event" => {
                Some(vanilla_debug_game_event_type_by_id(holder_id)?.resource_id)
            }
            "minecraft:mob_effect" => Some(vanilla_mob_effect_type(holder_id)?.resource_id),
            "minecraft:potion" => Some(vanilla_potion(holder_id)?.resource_id),
            "minecraft:sound_event" => vanilla_sound_event_resource_id(holder_id),
            "minecraft:world_clock" => Some(vanilla_world_clock_type_by_id(holder_id)?.resource_id),
            "minecraft:stat_type" => Some(vanilla_stat_type_by_id(holder_id)?.resource_id),
            "minecraft:custom_stat" => Some(vanilla_custom_stat_by_id(holder_id)?.resource_id),
            "minecraft:particle_type" => Some(vanilla_particle_type(holder_id)?.resource_id),
            "minecraft:data_component_type" => {
                Some(vanilla_data_component_type(holder_id)?.resource_id)
            }
            "minecraft:data_component_predicate_type" => {
                Some(vanilla_data_component_predicate_type(holder_id)?.resource_id)
            }
            "minecraft:cat_variant" => {
                Some(CatModelVariant::for_vanilla_registry_id(holder_id)?.resource_id())
            }
            "minecraft:cat_sound_variant" => {
                Some(CatSoundVariantIdentity::for_vanilla_registry_id(holder_id)?.resource_id())
            }
            "minecraft:cow_variant" => {
                Some(CowModelVariant::for_vanilla_registry_id(holder_id)?.resource_id())
            }
            "minecraft:cow_sound_variant" => {
                Some(CowSoundVariantIdentity::for_vanilla_registry_id(holder_id)?.resource_id())
            }
            "minecraft:wolf_variant" => {
                Some(WolfModelVariant::for_vanilla_registry_id(holder_id)?.resource_id())
            }
            "minecraft:wolf_sound_variant" => {
                Some(WolfSoundVariantIdentity::for_vanilla_registry_id(holder_id)?.resource_id())
            }
            "minecraft:frog_variant" => {
                Some(FrogModelVariant::for_vanilla_registry_id(holder_id)?.resource_id())
            }
            "minecraft:pig_variant" => {
                Some(PigModelVariant::for_vanilla_registry_id(holder_id)?.resource_id())
            }
            "minecraft:pig_sound_variant" => {
                Some(PigSoundVariantIdentity::for_vanilla_registry_id(holder_id)?.resource_id())
            }
            "minecraft:chicken_variant" => {
                Some(ChickenModelVariant::for_vanilla_registry_id(holder_id)?.resource_id())
            }
            "minecraft:chicken_sound_variant" => {
                Some(ChickenSoundVariantIdentity::for_vanilla_registry_id(holder_id)?.resource_id())
            }
            "minecraft:zombie_nautilus_variant" => {
                Some(ZombieNautilusModelVariant::for_vanilla_registry_id(holder_id)?.resource_id())
            }
            "minecraft:painting_variant" => Some(vanilla_painting_variant(holder_id)?.resource_id),
            "minecraft:villager_type" => {
                Some(VillagerModelType::for_vanilla_registry_id(holder_id)?.resource_id())
            }
            "minecraft:villager_profession" => {
                Some(VillagerModelProfession::for_vanilla_registry_id(holder_id)?.resource_id())
            }
            _ => None,
        }
    }

    fn vanilla_fallback_holder_id(&self, registry: &str, resource_id: &str) -> Option<i32> {
        if !resource_id.starts_with("minecraft:") {
            return None;
        }
        match registry {
            "minecraft:attribute" => Some(vanilla_attribute_type_for_resource_id(resource_id)?.id),
            "minecraft:chat_type" => {
                Some(VanillaChatTypeIdentity::for_resource_id(resource_id)?.vanilla_registry_id())
            }
            "minecraft:item" => {
                Some(bbb_protocol::item_types::vanilla_item_type_for_resource_id(resource_id)?.id)
            }
            "minecraft:block" => self.block_states.block_registry_id_by_name(resource_id),
            "minecraft:fluid" => {
                Some(VanillaFluidIdentity::for_resource_id(resource_id)?.vanilla_registry_id())
            }
            "minecraft:entity_type" => Some(
                bbb_protocol::entity_types::vanilla_entity_type_for_resource_id(resource_id)?.id,
            ),
            "minecraft:damage_type" => Some(vanilla_damage_type_for_resource_id(resource_id)?.id),
            "minecraft:game_event" => VANILLA_DEBUG_GAME_EVENT_TYPES
                .iter()
                .find(|entry| entry.resource_id == resource_id)
                .map(|entry| entry.holder_id),
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
            "minecraft:particle_type" => vanilla_particle_type_id_for_resource_id(resource_id),
            "minecraft:data_component_type" => {
                Some(vanilla_data_component_type_for_resource_id(resource_id)?.type_id)
            }
            "minecraft:data_component_predicate_type" => VANILLA_DATA_COMPONENT_PREDICATE_TYPES
                .iter()
                .find(|entry| entry.resource_id == resource_id)
                .map(|entry| entry.type_id),
            "minecraft:cat_variant" => Some(CatModelVariant::for_resource_id(resource_id)? as i32),
            "minecraft:cat_sound_variant" => {
                Some(CatSoundVariantIdentity::for_resource_id(resource_id)? as i32)
            }
            "minecraft:cow_variant" => Some(CowModelVariant::for_resource_id(resource_id)? as i32),
            "minecraft:cow_sound_variant" => {
                Some(CowSoundVariantIdentity::for_resource_id(resource_id)? as i32)
            }
            "minecraft:wolf_variant" => {
                Some(WolfModelVariant::for_resource_id(resource_id)? as i32)
            }
            "minecraft:wolf_sound_variant" => {
                Some(WolfSoundVariantIdentity::for_resource_id(resource_id)? as i32)
            }
            "minecraft:frog_variant" => {
                Some(FrogModelVariant::for_resource_id(resource_id)? as i32)
            }
            "minecraft:pig_variant" => Some(PigModelVariant::for_resource_id(resource_id)? as i32),
            "minecraft:pig_sound_variant" => {
                Some(PigSoundVariantIdentity::for_resource_id(resource_id)? as i32)
            }
            "minecraft:chicken_variant" => {
                Some(ChickenModelVariant::for_resource_id(resource_id)? as i32)
            }
            "minecraft:chicken_sound_variant" => {
                Some(ChickenSoundVariantIdentity::for_resource_id(resource_id)? as i32)
            }
            "minecraft:zombie_nautilus_variant" => {
                Some(ZombieNautilusModelVariant::for_resource_id(resource_id)? as i32)
            }
            "minecraft:painting_variant" => {
                Some(vanilla_painting_variant_for_resource_id(resource_id)?.id)
            }
            "minecraft:villager_type" => Some(
                VillagerModelType::for_resource_id(resource_id)?.vanilla_registry_index() as i32,
            ),
            "minecraft:villager_profession" => Some(
                VillagerModelProfession::for_resource_id(resource_id)?.vanilla_registry_index()
                    as i32,
            ),
            _ => None,
        }
    }
}

impl<'a> FrozenRegistryAccess<'a> {
    pub fn revision(self) -> RegistryAccessRevision {
        self.registries.revision
    }

    pub(super) fn typed_chat_type(
        self,
        resource_id: &str,
    ) -> Option<&'a (ChatTypeDecoration, ChatTypeDecoration)> {
        self.registries.typed.chat_types.get(resource_id)
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
        let resource_id = super::normalize_codec_identifier(raw).ok_or_else(|| {
            PersistentHolderDecodeError::InvalidResourceId {
                registry: registry.to_string(),
                value: raw.clone(),
            }
        })?;
        self.require_entry_by_resource_id(registry, &resource_id)
            .map_err(PersistentHolderDecodeError::Resolution)
    }

    pub(crate) fn persistent_holder_or_inline_from_nbt(
        self,
        registry: &str,
        value: &'a NbtSummaryValue,
    ) -> Result<PersistentHolderOrInline<'a>, PersistentHolderDecodeError> {
        match value {
            NbtSummaryValue::String(_) => self
                .persistent_holder_from_nbt(registry, value)
                .map(PersistentHolderOrInline::Named),
            _ => Ok(PersistentHolderOrInline::Direct(value)),
        }
    }

    pub fn persistent_holder_set_from_nbt(
        self,
        registry: &str,
        value: &NbtSummaryValue,
    ) -> Result<FrozenRegistryHolderSet<'a>, PersistentHolderSetDecodeError> {
        match value {
            NbtSummaryValue::String(raw) if raw.starts_with('#') => {
                let tag = super::normalize_codec_identifier(&raw[1..]).ok_or_else(|| {
                    PersistentHolderSetDecodeError::InvalidTag {
                        registry: registry.to_string(),
                        value: raw.clone(),
                    }
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
                        super::normalize_codec_identifier(raw).ok_or_else(|| {
                            PersistentHolderSetDecodeError::InvalidResourceId {
                                registry: registry.to_string(),
                                value: raw.clone(),
                            }
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

fn registry_has_exact_vanilla_fallback(registry: &str) -> bool {
    matches!(
        registry,
        "minecraft:attribute"
            | "minecraft:chat_type"
            | "minecraft:item"
            | "minecraft:block"
            | "minecraft:fluid"
            | "minecraft:entity_type"
            | "minecraft:damage_type"
            | "minecraft:game_event"
            | "minecraft:mob_effect"
            | "minecraft:potion"
            | "minecraft:sound_event"
            | "minecraft:world_clock"
            | "minecraft:stat_type"
            | "minecraft:custom_stat"
            | "minecraft:particle_type"
            | "minecraft:data_component_type"
            | "minecraft:data_component_predicate_type"
            | "minecraft:cat_variant"
            | "minecraft:cat_sound_variant"
            | "minecraft:cow_variant"
            | "minecraft:cow_sound_variant"
            | "minecraft:wolf_variant"
            | "minecraft:wolf_sound_variant"
            | "minecraft:frog_variant"
            | "minecraft:pig_variant"
            | "minecraft:pig_sound_variant"
            | "minecraft:chicken_variant"
            | "minecraft:chicken_sound_variant"
            | "minecraft:zombie_nautilus_variant"
            | "minecraft:painting_variant"
            | "minecraft:villager_type"
            | "minecraft:villager_profession"
    )
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
        let nbt = decode_registry_entry_nbt(raw_data.as_slice());
        let (element_data, element_data_error) = decoded_network_element(raw_data.as_slice());
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
        let (element_data, element_data_error) = raw_data
            .as_deref()
            .map(decoded_network_element)
            .unwrap_or_default();
        Self {
            id,
            has_data: raw_data.is_some(),
            raw_data_len,
            nbt,
            raw_data: raw_data.map(Arc::from),
            known_pack_data: None,
            element_data,
            element_data_error,
        }
    }
}

fn decoded_network_element(
    raw_data: &[u8],
) -> (
    Option<Arc<RegistryElementData>>,
    Option<RegistryElementDecodeError>,
) {
    match decode_network_element(raw_data) {
        Ok(data) => (Some(Arc::new(data)), None),
        Err(error) => (None, Some(error)),
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
