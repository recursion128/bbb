//! Registry facts and transactional preparation of generation-bound world resources.

use crate::world::LevelId;
mod native_entity_input;
pub(super) use native_entity_input::EntityNbtReadSnapshot;

use std::collections::BTreeMap;
use std::sync::Arc;

use super::{environment_attributes, natural_spawn, ServerWorld};
use bbb_server_types::{Result, ServerWorldError};

use bbb_protocol::{
    content_registry::ContentRegistry, PersistentRegistryMembership, PersistentRegistryResolver,
    PersistentRegistryTagMembers,
};
use bbb_registry_codec::{
    registry_codec::direct_leaf::decode_damage_type, synchronized_registry_element_codec,
    RegistryElementSource, RegistryElementValueRef,
};
use bbb_server_datapack::damage::{damage_type_registry, DamageTypeFacts, DAMAGE_TYPE_REGISTRY};

fn item_stack_registry_ids(
    registries: &crate::registry_sync::SynchronizedRegistries,
) -> (
    std::collections::BTreeSet<String>,
    BTreeMap<(String, String), i32>,
) {
    let registry_ids = registries
        .published()
        .iter()
        .map(|registry| registry.registry.clone())
        .collect();
    let holder_ids = registries
        .published()
        .iter()
        .flat_map(|registry| {
            registry
                .entries
                .iter()
                .enumerate()
                .filter_map(|(index, entry)| {
                    i32::try_from(index)
                        .ok()
                        .map(|index| ((registry.registry.clone(), entry.id.clone()), index))
                })
        })
        .collect();
    (registry_ids, holder_ids)
}

pub(super) fn new_item_stack_tracker(
    registries: &crate::registry_sync::SynchronizedRegistries,
) -> crate::entity::EntityTracker {
    let (registry_ids, holder_ids) = item_stack_registry_ids(registries);
    crate::entity::EntityTracker::new_with_item_stack_registry_view(
        registry_ids,
        holder_ids,
        registries.registry_tags_shared(),
    )
}

/// The `minecraft:damage_type` registry this world resolves damage sources
/// against.
///
/// The loaded datapack generation is the authority when the runtime wired one
/// in: `LoadedDynamicRegistry` already holds the elements in the sorted
/// `listResources` order the registry ids are handed out in
/// (`dynamic_registry.rs`), and the tags come out of the same generation rather
/// than from a second copy of the tag files. A world built without that surface
/// — the ephemeral flat fixture the tests and the MCP world use — reads the
/// built-in pack directly, which is the same pack the loader would have read.
pub(super) fn load_damage_type_registry(
    datapacks: Option<&bbb_server_datapack::startup::DatapackStartup>,
) -> std::sync::Arc<Vec<DamageTypeFacts>> {
    let loaded = datapacks
        .and_then(|datapacks| {
            let registry = datapacks.registries.registry(DAMAGE_TYPE_REGISTRY)?;
            let codec = synchronized_registry_element_codec(
                DAMAGE_TYPE_REGISTRY,
                RegistryElementSource::NetworkNbt,
            )
            .expect("damage type codec is registered");
            let elements = registry.elements.iter().map(|element| {
                let nbt = &element
                    .codec_value
                    .as_ref()
                    .expect("damage type canonical NBT is required")
                    .network_nbt;
                (
                    element.id.clone(),
                    decode_damage_type(RegistryElementValueRef::Nbt(nbt), codec)
                        .expect("validated damage type decodes"),
                )
            });
            Some(damage_type_registry(elements, |tag_id| {
                datapacks
                    .tags
                    .registry(DAMAGE_TYPE_REGISTRY)
                    .and_then(|tags| tags.tag(tag_id))
                    .map(<[String]>::to_vec)
                    .unwrap_or_default()
            }))
        })
        .unwrap_or_else(|| bbb_server_datapack::damage::builtin_damage_type_registry().to_vec());
    std::sync::Arc::new(loaded)
}

/// The registry generation persistent ItemStack components resolve against at
/// runtime. Dynamic registries and their rebuilt tags come from the installed
/// datapack generation; fixed registries fall back to the verified built-in
/// 26.1 facts used by the save validator.
/// The registry publication live item stacks resolve against on both sides
/// of the save boundary; owned so a save request can carry it.
pub struct PlayerItemStackResolver {
    datapacks: Option<Arc<bbb_server_datapack::startup::DatapackStartup>>,
    registries: crate::registry_sync::SynchronizedRegistries,
}

impl std::fmt::Debug for PlayerItemStackResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PlayerItemStackResolver")
    }
}

impl PersistentRegistryResolver for PlayerItemStackResolver {
    fn registry_available(&self, registry_id: &str) -> bool {
        self.datapack_registries().registry_available(registry_id)
    }

    fn resource_membership(
        &self,
        registry_id: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership {
        self.datapack_registries()
            .resource_membership(registry_id, resource_id)
    }

    fn tag_members(&self, registry_id: &str, tag_id: &str) -> PersistentRegistryTagMembers {
        self.datapack_registries().tag_members(registry_id, tag_id)
    }

    fn tag_contains(&self, registry_id: &str, tag_id: &str, resource_id: &str) -> Option<bool> {
        self.registry_available(registry_id).then(|| {
            self.registries
                .registry_tags()
                .contains(registry_id, tag_id, resource_id)
        })
    }
}

impl bbb_registry_codec::PersistentItemStackHolderResolver for PlayerItemStackResolver {
    fn cat_sound_variant(
        &self,
        holder_id: i32,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::CatSoundVariantElement> {
        self.registries.cat_sound_variant(holder_id)
    }

    fn cow_sound_variant(
        &self,
        resource_id: &str,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::CowSoundVariantElement> {
        self.registries.cow_sound_variant(resource_id)
    }

    fn chicken_sound_variant(
        &self,
        resource_id: &str,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::ChickenSoundVariantElement>
    {
        self.registries.chicken_sound_variant(resource_id)
    }

    fn pig_sound_variant(
        &self,
        resource_id: &str,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::PigSoundVariantElement> {
        self.registries.pig_sound_variant(resource_id)
    }

    fn wolf_sound_variant(
        &self,
        resource_id: &str,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::WolfSoundVariantElement> {
        self.registries.wolf_sound_variant(resource_id)
    }

    fn wolf_sound_variant_count(&self) -> Option<usize> {
        self.registries.registry_len("minecraft:wolf_sound_variant")
    }

    fn painting_variant_size(&self, resource_id: &str) -> Option<[i32; 2]> {
        self.registries.painting_variant_size(resource_id)
    }

    fn jukebox_song(
        &self,
        holder_id: i32,
    ) -> Option<bbb_registry_codec::registry_codec::JukeboxSongRegistryValue> {
        self.registries.jukebox_song(holder_id)
    }

    fn holder_id(&self, registry_id: &str, resource_id: &str) -> Option<i32> {
        self.registries
            .registry_element_index(registry_id, resource_id)
    }

    fn holder_resource_id(&self, registry_id: &str, holder_id: i32) -> Option<String> {
        self.registries
            .registry_element_id(registry_id, holder_id)
            .map(str::to_owned)
    }

    fn content_registry(&self) -> &ContentRegistry {
        self.registries.content_registry()
    }
}

impl PlayerItemStackResolver {
    pub(in crate::world) fn new(
        datapacks: Option<&Arc<bbb_server_datapack::startup::DatapackStartup>>,
        registries: &crate::registry_sync::SynchronizedRegistries,
    ) -> Self {
        Self {
            datapacks: datapacks.cloned(),
            registries: registries.clone(),
        }
    }

    #[must_use]
    pub fn as_dyn(&self) -> &dyn bbb_registry_codec::PersistentItemStackHolderResolver {
        self
    }

    fn datapack_registries(&self) -> RuntimePersistentRegistryResolver<'_> {
        RuntimePersistentRegistryResolver {
            datapacks: self.datapacks.as_deref(),
            content_registry: self.registries.content_registry(),
        }
    }
}

/// The built-in registry publication, for projections a test builds by hand.
#[cfg(test)]
pub(crate) fn item_stack_resolver_for_test() -> &'static PlayerItemStackResolver {
    static RESOLVER: std::sync::OnceLock<PlayerItemStackResolver> = std::sync::OnceLock::new();
    RESOLVER.get_or_init(|| PlayerItemStackResolver {
        datapacks: None,
        registries: crate::registry_sync::SynchronizedRegistries::load(None)
            .expect("the built-in registry publication loads"),
    })
}

impl ServerWorld {
    pub(crate) fn item_stack_resolver(&self) -> PlayerItemStackResolver {
        PlayerItemStackResolver::new(self.cold.config.datapacks.as_ref(), &self.registries)
    }
}

pub(super) struct RuntimePersistentRegistryResolver<'a> {
    pub(super) datapacks: Option<&'a bbb_server_datapack::startup::DatapackStartup>,
    pub(super) content_registry: &'a ContentRegistry,
}

impl PersistentRegistryResolver for RuntimePersistentRegistryResolver<'_> {
    fn registry_available(&self, registry_id: &str) -> bool {
        self.datapacks
            .and_then(|datapacks| datapacks.registries.registry(registry_id))
            .is_some()
            || bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_available_26_1(
                registry_id,
            )
    }

    fn resource_membership(
        &self,
        registry_id: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership {
        if let Some(registry) = self
            .datapacks
            .and_then(|datapacks| datapacks.registries.registry(registry_id))
        {
            return if registry
                .elements
                .iter()
                .any(|element| element.id == resource_id)
            {
                PersistentRegistryMembership::Present
            } else {
                PersistentRegistryMembership::Absent
            };
        }
        let vanilla =
            bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_membership_26_1(
                registry_id,
                resource_id,
            );
        if vanilla == PersistentRegistryMembership::Present {
            return vanilla;
        }
        let custom_present = match registry_id {
            "minecraft:block" => self
                .content_registry
                .block_by_resource_id(resource_id)
                .is_some(),
            "minecraft:item" => self
                .content_registry
                .item_by_resource_id(resource_id)
                .is_some(),
            "minecraft:entity_type" => self
                .content_registry
                .entity_by_resource_id(resource_id)
                .is_some(),
            "minecraft:particle_type" => self
                .content_registry
                .particle_by_resource_id(resource_id)
                .is_some(),
            "minecraft:sound_event" => self
                .content_registry
                .sound_event_by_resource_id(resource_id)
                .is_some(),
            _ => false,
        };
        if custom_present {
            PersistentRegistryMembership::Present
        } else {
            vanilla
        }
    }

    fn tag_members(&self, registry_id: &str, tag_id: &str) -> PersistentRegistryTagMembers {
        if let Some(datapacks) = self.datapacks {
            if let Some(tags) = datapacks.tags.registry(registry_id) {
                return tags
                    .tag(tag_id)
                    .map_or(PersistentRegistryTagMembers::Absent, |members| {
                        PersistentRegistryTagMembers::Present(members.to_vec())
                    });
            }
            if self.registry_available(registry_id) {
                return PersistentRegistryTagMembers::Absent;
            }
        }
        bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1(
            registry_id,
            tag_id,
        )
    }
}

impl bbb_registry_codec::registry_codec::RegistryReferenceLookup
    for RuntimePersistentRegistryResolver<'_>
{
    fn contains_element(&self, registry: &str, id: &str) -> bool {
        self.resource_membership(registry, id) == PersistentRegistryMembership::Present
    }

    fn contains_tag(&self, registry: &str, id: &str) -> bool {
        matches!(
            self.tag_members(registry, id),
            PersistentRegistryTagMembers::Present(_)
        )
    }

    fn registry_available(&self, registry: &str) -> bool {
        PersistentRegistryResolver::registry_available(self, registry)
    }
}

pub(super) fn registry_tag_contains(
    resolver: &dyn PersistentRegistryResolver,
    registry_id: &str,
    tag_id: &str,
    resource_id: &str,
) -> bool {
    match resolver.tag_members(registry_id, tag_id) {
        PersistentRegistryTagMembers::Present(members) => {
            members.iter().any(|member| member == resource_id)
        }
        PersistentRegistryTagMembers::Absent => false,
        // A runtime generation should always answer for the registries this
        // server owns. If it cannot, refusing a damage source is safer than
        // silently dropping an immunity.
        PersistentRegistryTagMembers::Unavailable => true,
    }
}

pub(crate) struct PreparedNoiseReloadableResources {
    levels: BTreeMap<LevelId, crate::chunk::noise_generator::NoiseReloadableResources>,
    mob_spawns: Option<natural_spawn::MobSpawnCatalog>,
    dimension_sky_lights: environment_attributes::DimensionSkyLights,
}

impl ServerWorld {
    pub(super) fn install_item_stack_registry_view(
        &mut self,
        registries: &crate::registry_sync::SynchronizedRegistries,
    ) {
        let (registry_ids, holder_ids) = item_stack_registry_ids(registries);
        let tags = registries.registry_tags_shared();
        for level in self.levels.values_mut() {
            level.entity_tracker.replace_item_stack_registry_view(
                registry_ids.clone(),
                holder_ids.clone(),
                tags.clone(),
            );
        }
    }

    pub(crate) fn prepare_noise_reloadable_resources(
        &self,
        startup: &Arc<bbb_server_datapack::startup::DatapackStartup>,
    ) -> Result<PreparedNoiseReloadableResources> {
        let mob_spawns = natural_spawn::load_mob_spawns(Some(startup))?;
        let dimension_sky_lights = environment_attributes::load_sky_lights(Some(startup))?;
        let mut replacements = BTreeMap::new();
        for (dimension, level) in self.levels.iter() {
            let replacement = level
                .chunks
                .prepare_noise_reloadable_resources(startup)
                .map_err(|error| ServerWorldError::InvalidConfiguration(error.to_string()))?;
            if let Some(replacement) = replacement {
                replacements.insert(dimension, replacement);
            }
        }
        Ok(PreparedNoiseReloadableResources {
            levels: replacements,
            mob_spawns,
            dimension_sky_lights,
        })
    }

    pub(crate) fn install_noise_reloadable_resources(
        &mut self,
        replacements: PreparedNoiseReloadableResources,
    ) {
        let PreparedNoiseReloadableResources {
            levels,
            mob_spawns,
            dimension_sky_lights,
        } = replacements;
        for (dimension, replacement) in levels {
            self.level_mut_by_id(&dimension)
                .chunks
                .install_noise_reloadable_resources(replacement);
        }
        self.cold.mob_spawns = mob_spawns;
        self.cold.dimension_sky_lights = dimension_sky_lights;
    }
}
