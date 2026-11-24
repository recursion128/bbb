//! Synchronized registry and tag publication for one frozen server generation.
//!
//! Vanilla answers a client's `ClientboundSelectKnownPacks` reply by packing
//! every registry of `RegistryDataLoader.SYNCHRONIZED_REGISTRIES` into one
//! `ClientboundRegistryDataPacket` each and then sending the tag payload
//! (`SynchronizeRegistriesTask.java:35-44`). Among those twenty-eight, a level's
//! identity depends on `minecraft:dimension_type`
//! (`RegistryDataLoader.java:151`): `ServerPlayer.createCommonSpawnInfo` writes
//! the level's dimension type as a bare index into it
//! (`CommonPlayerSpawnInfo.java:43`, `DimensionType.java:57`,
//! `ByteBufCodecs.java:560-586`).
//!
//! One owner is the point. The index is not a number this server chooses; it is
//! the position the element takes in the registry the official server publishes
//! for the same built-in pack, and a client rebuilds that registry by
//! registering the entries in arrival order (`RegistryLoadTask.java:62-71`).
//! Deriving the order here and indexing into something else would rename every
//! dimension without any code looking wrong.
//!
//! The complete publication assembled here is now wired into configuration:
//! all twenty-eight registry packets are followed by one tag packet built from
//! the same frozen generation and then `FinishConfiguration`. One synchronized registry alone
//! is not a wire state a client accepts: any registry data makes it rebuild
//! *every* synchronized registry from what arrived (`RegistryDataCollector.java:91-99`),
//! so the thirteen `RegistryValidator.nonEmpty()` registries
//! (`RegistryDataLoader.java:138-150`) cannot be sent empty. Registry elements
//! also name tags -- `data/minecraft/dimension_type/overworld.json` declares
//! `timelines: "#minecraft:in_overworld"` -- and Vanilla serializes those tags
//! after the registry packets (`SynchronizeRegistriesTask.java:43`).
//!
//! Biome is an exact synchronized owner. Named, tag, and inline generation
//! holders decode through the configured-carver and placed-feature codecs;
//! freeze proves membership before publication writes the network projection.

use std::sync::{Arc, OnceLock};

use crate::registry_tags::RegistryTagSnapshot;
use bbb_protocol::{
    content_registry::ContentRegistry,
    encode_nbt_summary,
    packets::{encode_configuration_registry_data, encode_configuration_update_tags},
    packets::{PublishedRegistry, RegistryDataEntry, UpdateTags},
    registry_data_types::VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES,
};
use bbb_registry_codec::decode_exact_synchronized_registry_element_with_content_registry;

use bbb_server_datapack::dimension_type::DIMENSION_TYPE_REGISTRY;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
use bbb_server_datapack::tag_loader::TagLoadReport;

mod tag_publication;

const MAX_PUBLISHED_REGISTRY_ELEMENTS: usize = 16 * 1024;

/// Byte budget for one published element's NBT payload.
///
/// The largest built-in dimension type is under two kilobytes; this leaves the
/// element room to grow by an order of magnitude while still refusing a
/// pathological document rather than writing a packet nothing will read.
const MAX_PUBLISHED_ELEMENT_BYTES: usize = 64 * 1024;

/// A registry element this server refuses to publish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrySyncError {
    pub registry_id: String,
    pub element_id: String,
    pub message: String,
}

impl std::fmt::Display for RegistrySyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "registry {} element {}: {}",
            self.registry_id, self.element_id, self.message
        )
    }
}

impl std::error::Error for RegistrySyncError {}

/// The synchronized registries one server publishes, in the order it publishes
/// them.
#[derive(Debug, Clone, PartialEq)]
pub struct SynchronizedRegistries {
    registries: Arc<[PublishedRegistry]>,
    tags: Arc<UpdateTags>,
    registry_tags: Arc<RegistryTagSnapshot>,
    jukebox_songs: Arc<[bbb_registry_codec::registry_codec::JukeboxSongRegistryValue]>,
    cat_sound_variants:
        Arc<[bbb_registry_codec::registry_codec::sound_variants::CatSoundVariantElement]>,
    cow_sound_variants:
        Arc<[bbb_registry_codec::registry_codec::sound_variants::CowSoundVariantElement]>,
    chicken_sound_variants:
        Arc<[bbb_registry_codec::registry_codec::sound_variants::ChickenSoundVariantElement]>,
    pig_sound_variants:
        Arc<[bbb_registry_codec::registry_codec::sound_variants::PigSoundVariantElement]>,
    wolf_sound_variants:
        Arc<[bbb_registry_codec::registry_codec::sound_variants::WolfSoundVariantElement]>,
    /// `[width, height]` by `minecraft:painting_variant` holder id.
    painting_variant_sizes: Arc<[[i32; 2]]>,
}

impl SynchronizedRegistries {
    /// The complete configuration publication for one frozen datapack startup.
    ///
    /// The startup's registry generation and tag report are one transaction:
    /// Vanilla encodes registry
    /// holders against the completed registry access and serializes tags from
    /// that same frozen access (`RegistrySynchronization.java:27-84`). Taking
    /// their common [`DatapackStartup`] owner prevents callers from mixing two
    /// reload generations. `None` is the test/fixture built-in path and loads the
    /// complete built-in startup through the same datapack pipeline.
    pub fn load(startup: Option<&DatapackStartup>) -> Result<Self, RegistrySyncError> {
        let content_registry = startup.map_or_else(
            || Arc::clone(ContentRegistry::vanilla()),
            |startup| Arc::new(startup.content_registry().clone()),
        );
        Self::load_with_content_registry(startup, content_registry)
    }

    pub fn load_with_content_registry(
        startup: Option<&DatapackStartup>,
        content_registry: Arc<ContentRegistry>,
    ) -> Result<Self, RegistrySyncError> {
        match startup {
            Some(startup) => Self::from_startup_with_content_registry(startup, content_registry),
            None => {
                let (builtin_ref, startup) = builtin_registry_input()?;
                let mut builtin = builtin_ref.clone();
                let has_custom = !content_registry.blocks().is_empty()
                    || !content_registry.items().is_empty()
                    || !content_registry.entities().is_empty()
                    || !content_registry.particles().is_empty()
                    || !content_registry.sound_events().is_empty();
                if has_custom {
                    builtin.registry_tags = Arc::new(freeze_registry_tags(
                        &startup.tags,
                        Arc::clone(&content_registry),
                    ));
                    let tags = tag_publication::pack_network_tags(
                        &builtin.registries,
                        &startup.registries,
                        &startup.tags,
                        &content_registry,
                    )?;
                    builtin.tags = Arc::new(tags);
                }
                Ok(builtin)
            }
        }
    }

    pub fn content_registry(&self) -> &ContentRegistry {
        self.registry_tags.content_registry()
    }

    pub fn content_registry_arc(&self) -> &Arc<ContentRegistry> {
        self.registry_tags.content_registry_arc()
    }

    /// Rebuilds only the reloadable tag views against this server's frozen
    /// synchronized registries (`MinecraftServer.reloadResources`,
    /// `MinecraftServer.java:1504-1544`).
    pub fn reload_tags(&self, startup: &DatapackStartup) -> Result<Self, RegistrySyncError> {
        for published in self.registries.iter() {
            let loaded = startup
                .registries
                .registry(&published.registry)
                .ok_or_else(|| RegistrySyncError {
                    registry_id: published.registry.clone(),
                    element_id: String::new(),
                    message: "synchronized registry is absent from the frozen generation"
                        .to_string(),
                })?;
            let same_order = loaded.elements.len() == published.entries.len()
                && loaded
                    .elements
                    .iter()
                    .zip(published.entries.iter())
                    .all(|(element, entry)| element.id == entry.id);
            if !same_order {
                return Err(RegistrySyncError {
                    registry_id: published.registry.clone(),
                    element_id: String::new(),
                    message: "reload attempted to mutate frozen synchronized registry entries"
                        .to_string(),
                });
            }
        }
        let content_registry = Arc::clone(self.registry_tags.content_registry_arc());
        let registry_tags = Arc::new(freeze_registry_tags(
            &startup.tags,
            Arc::clone(&content_registry),
        ));
        let tags = tag_publication::pack_network_tags(
            &self.registries,
            &startup.registries,
            &startup.tags,
            &content_registry,
        )?;
        encode_configuration_update_tags(&tags).map_err(|error| RegistrySyncError {
            registry_id: "minecraft:root".to_string(),
            element_id: String::new(),
            message: format!("tag packet is not encodable: {error}"),
        })?;
        Ok(Self {
            registries: Arc::clone(&self.registries),
            tags: Arc::new(tags),
            registry_tags,
            jukebox_songs: Arc::clone(&self.jukebox_songs),
            cat_sound_variants: Arc::clone(&self.cat_sound_variants),
            cow_sound_variants: Arc::clone(&self.cow_sound_variants),
            chicken_sound_variants: Arc::clone(&self.chicken_sound_variants),
            pig_sound_variants: Arc::clone(&self.pig_sound_variants),
            wolf_sound_variants: Arc::clone(&self.wolf_sound_variants),
            painting_variant_sizes: Arc::clone(&self.painting_variant_sizes),
        })
    }

    fn from_startup_with_content_registry(
        startup: &DatapackStartup,
        content_registry: Arc<ContentRegistry>,
    ) -> Result<Self, RegistrySyncError> {
        let generation = &startup.registries;
        let tags = &startup.tags;
        startup
            .validate_frozen_registry_references()
            .map_err(|error| {
                let first = error
                    .errors
                    .first()
                    .expect("frozen registry codec validation reports at least one error");
                RegistrySyncError {
                    registry_id: first.registry_id.clone(),
                    element_id: first.element_id.clone().unwrap_or_default(),
                    message: error.to_string(),
                }
            })?;
        let lookup = FrozenDatapackRegistryLookup::from_startup(startup)
            .with_content_registry(&content_registry);
        let mut registries = Vec::with_capacity(VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES.len());
        let mut jukebox_songs = Vec::new();
        let mut cat_sound_variants = Vec::new();
        let mut cow_sound_variants = Vec::new();
        let mut chicken_sound_variants = Vec::new();
        let mut pig_sound_variants = Vec::new();
        let mut wolf_sound_variants = Vec::new();
        let mut painting_variant_sizes = Vec::new();
        for data_type in VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {
            let loaded =
                generation
                    .registry(data_type.resource_id)
                    .ok_or_else(|| RegistrySyncError {
                        registry_id: data_type.resource_id.to_string(),
                        element_id: String::new(),
                        message: "synchronized registry is absent from the frozen generation"
                            .to_string(),
                    })?;
            let mut variants = SynchronizedVariantOutputs {
                jukebox_songs: &mut jukebox_songs,
                cat_sound_variants: &mut cat_sound_variants,
                cow_sound_variants: &mut cow_sound_variants,
                chicken_sound_variants: &mut chicken_sound_variants,
                pig_sound_variants: &mut pig_sound_variants,
                wolf_sound_variants: &mut wolf_sound_variants,
                painting_variant_sizes: &mut painting_variant_sizes,
            };
            registries.push(pack_synchronized_registry(
                loaded,
                &lookup,
                &content_registry,
                &mut variants,
            )?);
        }
        let registry_tags = Arc::new(freeze_registry_tags(tags, Arc::clone(&content_registry)));
        let tags =
            tag_publication::pack_network_tags(&registries, generation, tags, &content_registry)?;
        validate_wire_transaction(&registries, &tags)?;
        Ok(Self {
            registries: registries.into(),
            tags: Arc::new(tags),
            registry_tags,
            jukebox_songs: jukebox_songs.into(),
            cat_sound_variants: cat_sound_variants.into(),
            cow_sound_variants: cow_sound_variants.into(),
            chicken_sound_variants: chicken_sound_variants.into(),
            pig_sound_variants: pig_sound_variants.into(),
            wolf_sound_variants: wolf_sound_variants.into(),
            painting_variant_sizes: painting_variant_sizes.into(),
        })
    }

    /// The wire index of `element_id`, which is its position in the entry list
    /// [`Self::published`] builds out of this same value.
    ///
    /// `Registry.getIdOrThrow` has no answer for an element the registry does
    /// not hold, and neither does this: a level naming a dimension type nothing
    /// published cannot be described to a client at all.
    pub fn dimension_type_index(&self, element_id: &str) -> Option<i32> {
        self.registry_element_index(DIMENSION_TYPE_REGISTRY, element_id)
    }

    /// The raw holder id of an element in the exact registry transaction this
    /// server publishes. `MappedRegistry.getId` is registration order
    /// (`MappedRegistry.java:88-118`), which is also the packet entry order.
    pub fn registry_element_index(&self, registry_id: &str, element_id: &str) -> Option<i32> {
        let registry = self
            .registries
            .iter()
            .find(|registry| registry.registry == registry_id)?;
        let position = registry
            .entries
            .iter()
            .position(|entry| entry.id == element_id)?;
        i32::try_from(position).ok()
    }

    /// Reverse lookup within the same immutable publication transaction.
    pub fn registry_element_id(&self, registry_id: &str, index: i32) -> Option<&str> {
        let index = usize::try_from(index).ok()?;
        self.registries
            .iter()
            .find(|registry| registry.registry == registry_id)?
            .entries
            .get(index)
            .map(|entry| entry.id.as_str())
    }

    pub fn registry_len(&self, registry_id: &str) -> Option<usize> {
        self.registries
            .iter()
            .find(|registry| registry.registry == registry_id)
            .map(|registry| registry.entries.len())
    }

    pub(crate) fn cat_sound_variant(
        &self,
        holder_id: i32,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::CatSoundVariantElement> {
        self.cat_sound_variants
            .get(usize::try_from(holder_id).ok()?)
    }

    pub(crate) fn cow_sound_variant(
        &self,
        resource_id: &str,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::CowSoundVariantElement> {
        let index = self.registry_element_index("minecraft:cow_sound_variant", resource_id)?;
        self.cow_sound_variants.get(usize::try_from(index).ok()?)
    }

    pub(crate) fn chicken_sound_variant(
        &self,
        resource_id: &str,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::ChickenSoundVariantElement>
    {
        let index = self.registry_element_index("minecraft:chicken_sound_variant", resource_id)?;
        self.chicken_sound_variants
            .get(usize::try_from(index).ok()?)
    }

    pub(crate) fn pig_sound_variant(
        &self,
        resource_id: &str,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::PigSoundVariantElement> {
        let index = self.registry_element_index("minecraft:pig_sound_variant", resource_id)?;
        self.pig_sound_variants.get(usize::try_from(index).ok()?)
    }

    pub(crate) fn wolf_sound_variant(
        &self,
        resource_id: &str,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::WolfSoundVariantElement> {
        let index = self.registry_element_index("minecraft:wolf_sound_variant", resource_id)?;
        self.wolf_sound_variants.get(usize::try_from(index).ok()?)
    }

    pub(crate) fn painting_variant_size(&self, resource_id: &str) -> Option<[i32; 2]> {
        let index = self.registry_element_index("minecraft:painting_variant", resource_id)?;
        self.painting_variant_sizes
            .get(usize::try_from(index).ok()?)
            .copied()
    }

    pub(crate) fn jukebox_song(
        &self,
        holder_id: i32,
    ) -> Option<bbb_registry_codec::registry_codec::JukeboxSongRegistryValue> {
        self.jukebox_songs
            .get(usize::try_from(holder_id).ok()?)
            .copied()
    }

    pub fn require_registry_element_index(
        &self,
        registry_id: &str,
        element_id: &str,
    ) -> Result<i32, RegistrySyncError> {
        self.registry_element_index(registry_id, element_id)
            .ok_or_else(|| RegistrySyncError {
                registry_id: registry_id.to_string(),
                element_id: element_id.to_string(),
                message: "element is absent from the published transaction".to_string(),
            })
    }

    /// The `minecraft:dimension_type` registry as a `ClientboundRegistryDataPacket`
    /// would carry it, in `RegistryDataLoader.SYNCHRONIZED_REGISTRIES` order
    /// (`RegistryDataLoader.java:133-162`).
    ///
    pub fn published(&self) -> &[PublishedRegistry] {
        &self.registries
    }

    /// Shared ownership for connection snapshots; cloning this does not copy
    /// the encoded registry payloads.
    pub fn published_shared(&self) -> Arc<[PublishedRegistry]> {
        Arc::clone(&self.registries)
    }

    /// The one `ClientboundUpdateTagsPacket` following [`Self::published`].
    pub fn published_tags(&self) -> &UpdateTags {
        &self.tags
    }

    /// Shared ownership for connection snapshots; cloning this does not copy
    /// the complete tag transaction.
    pub fn published_tags_shared(&self) -> Arc<UpdateTags> {
        Arc::clone(&self.tags)
    }

    /// The level-scoped lookup frozen with this publication transaction.
    pub fn registry_tags(&self) -> &RegistryTagSnapshot {
        &self.registry_tags
    }

    /// Clones the transaction's owner without copying its member sets.
    pub fn registry_tags_shared(&self) -> Arc<RegistryTagSnapshot> {
        Arc::clone(&self.registry_tags)
    }

    /// The element ids of the published `minecraft:dimension_type` registry, in
    /// index order.
    pub fn dimension_type_ids(&self) -> impl Iterator<Item = &str> {
        self.registries
            .iter()
            .find(|registry| registry.registry == DIMENSION_TYPE_REGISTRY)
            .expect("the synchronized catalog contains dimension_type")
            .entries
            .iter()
            .map(|entry| entry.id.as_str())
    }
}

fn freeze_registry_tags(
    tags: &TagLoadReport,
    content_registry: Arc<ContentRegistry>,
) -> RegistryTagSnapshot {
    RegistryTagSnapshot::new_with_content_registry(
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

fn validate_wire_transaction(
    registries: &[PublishedRegistry],
    tags: &UpdateTags,
) -> Result<(), RegistrySyncError> {
    for registry in registries {
        encode_configuration_registry_data(&registry.registry, &registry.entries).map_err(
            |error| RegistrySyncError {
                registry_id: registry.registry.clone(),
                element_id: String::new(),
                message: format!("registry packet is not encodable: {error}"),
            },
        )?;
    }
    encode_configuration_update_tags(tags).map_err(|error| RegistrySyncError {
        registry_id: "minecraft:root".to_string(),
        element_id: String::new(),
        message: format!("tag packet is not encodable: {error}"),
    })?;
    Ok(())
}

struct SynchronizedVariantOutputs<'a> {
    jukebox_songs: &'a mut Vec<bbb_registry_codec::registry_codec::JukeboxSongRegistryValue>,
    cat_sound_variants:
        &'a mut Vec<bbb_registry_codec::registry_codec::sound_variants::CatSoundVariantElement>,
    cow_sound_variants:
        &'a mut Vec<bbb_registry_codec::registry_codec::sound_variants::CowSoundVariantElement>,
    chicken_sound_variants:
        &'a mut Vec<bbb_registry_codec::registry_codec::sound_variants::ChickenSoundVariantElement>,
    pig_sound_variants:
        &'a mut Vec<bbb_registry_codec::registry_codec::sound_variants::PigSoundVariantElement>,
    wolf_sound_variants:
        &'a mut Vec<bbb_registry_codec::registry_codec::sound_variants::WolfSoundVariantElement>,
    painting_variant_sizes: &'a mut Vec<[i32; 2]>,
}

fn pack_synchronized_registry(
    loaded: &bbb_server_datapack::dynamic_registry::LoadedDynamicRegistry,
    lookup: &FrozenDatapackRegistryLookup<'_>,
    content_registry: &ContentRegistry,
    variants: &mut SynchronizedVariantOutputs<'_>,
) -> Result<PublishedRegistry, RegistrySyncError> {
    let registry_id = &loaded.registry_id;
    let mut entries = Vec::with_capacity(loaded.elements.len());
    for element in &loaded.elements {
        // Vanilla encodes the bound registry value, not a second JSON parse
        // (`RegistrySynchronization.java:37-70`). Freeze already proved every
        // retained placeholder, so publication writes that NBT. Elements without
        // a retained projection decode here and check membership before encoding.
        let projected_network;
        let network = if let Some(retained) = &element.codec_value {
            &retained.network_nbt
        } else {
            let decoded = element
                .decoded_direct_source()
                .map_err(|error| RegistrySyncError {
                    registry_id: registry_id.clone(),
                    element_id: element.id.clone(),
                    message: error.to_string(),
                })?;
            let value = decode_exact_synchronized_registry_element_with_content_registry(
                registry_id,
                &decoded,
                content_registry,
            )
            .map_err(|error| RegistrySyncError {
                registry_id: registry_id.clone(),
                element_id: element.id.clone(),
                message: error.to_string(),
            })?;
            projected_network =
                value
                    .encode_network(lookup)
                    .map_err(|error| RegistrySyncError {
                        registry_id: registry_id.clone(),
                        element_id: element.id.clone(),
                        message: error.to_string(),
                    })?;
            &projected_network
        };
        if matches!(
            registry_id.as_str(),
            "minecraft:cat_sound_variant"
                | "minecraft:cow_sound_variant"
                | "minecraft:chicken_sound_variant"
                | "minecraft:pig_sound_variant"
                | "minecraft:wolf_sound_variant"
        ) {
            use bbb_registry_codec::registry_codec::{
                sound_variants::{decode_sound_variant_element, SoundVariantElement},
                RegistryElementSource, RegistryElementValueRef,
            };
            let decoded = decode_sound_variant_element(
                registry_id,
                RegistryElementSource::NetworkNbt,
                RegistryElementValueRef::Nbt(network),
            )
            .map_err(|error| RegistrySyncError {
                registry_id: registry_id.clone(),
                element_id: element.id.clone(),
                message: error.to_string(),
            })?;
            match decoded {
                SoundVariantElement::Cat(variant) => variants.cat_sound_variants.push(variant),
                SoundVariantElement::Cow(variant) => variants.cow_sound_variants.push(variant),
                SoundVariantElement::Chicken(variant) => {
                    variants.chicken_sound_variants.push(variant)
                }
                SoundVariantElement::Pig(variant) => variants.pig_sound_variants.push(variant),
                SoundVariantElement::Wolf(variant) => variants.wolf_sound_variants.push(variant),
            }
        }
        if registry_id == "minecraft:painting_variant" {
            variants
                .painting_variant_sizes
                .push(
                    painting_variant_size(network).ok_or_else(|| RegistrySyncError {
                        registry_id: registry_id.clone(),
                        element_id: element.id.clone(),
                        message: "validated painting variant has no size".into(),
                    })?,
                );
        }
        if registry_id == "minecraft:jukebox_song" {
            variants.jukebox_songs.push(
                bbb_registry_codec::registry_codec::jukebox_song_registry_value(network)
                    .ok_or_else(|| RegistrySyncError {
                        registry_id: registry_id.clone(),
                        element_id: element.id.clone(),
                        message: "validated jukebox song has no playback fields".into(),
                    })?,
            );
        }
        let raw_data =
            encode_nbt_summary(network, MAX_PUBLISHED_ELEMENT_BYTES).map_err(|error| {
                RegistrySyncError {
                    registry_id: registry_id.clone(),
                    element_id: element.id.clone(),
                    message: format!("element does not fit a published payload: {error}"),
                }
            })?;
        entries.push(RegistryDataEntry {
            id: element.id.clone(),
            raw_data: Some(raw_data),
        });
    }
    if entries.len() > MAX_PUBLISHED_REGISTRY_ELEMENTS {
        return Err(RegistrySyncError {
            registry_id: registry_id.clone(),
            element_id: String::new(),
            message: format!(
                "registry has {} entries, maximum is {MAX_PUBLISHED_REGISTRY_ELEMENTS}",
                entries.len()
            ),
        });
    }
    Ok(PublishedRegistry {
        registry: registry_id.clone(),
        entries,
    })
}

/// `PaintingVariant.DIRECT_CODEC` (`PaintingVariant.java:18-25`).
fn painting_variant_size(network: &bbb_protocol::NbtSummaryValue) -> Option<[i32; 2]> {
    let bbb_protocol::NbtSummaryValue::Compound(entries) = network else {
        return None;
    };
    let int = |name: &str| match entries.iter().find(|entry| entry.name == name)?.value {
        bbb_protocol::NbtSummaryValue::Int(value) => Some(value),
        _ => None,
    };
    Some([int("width")?, int("height")?])
}

fn builtin_registry_input(
) -> Result<&'static (SynchronizedRegistries, Arc<DatapackStartup>), RegistrySyncError> {
    static BUILTIN: OnceLock<
        Result<(SynchronizedRegistries, Arc<DatapackStartup>), RegistrySyncError>,
    > = OnceLock::new();
    BUILTIN
        .get_or_init(|| {
            let startup = DatapackStartup::load(&DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: DatapackLoadLimits::default(),
            })
            .map_err(|error| RegistrySyncError {
                registry_id: "minecraft:root".to_string(),
                element_id: String::new(),
                message: format!("built-in datapack startup failed: {error}"),
            })?;
            let registries = SynchronizedRegistries::from_startup_with_content_registry(
                &startup,
                Arc::clone(ContentRegistry::vanilla()),
            )?;
            Ok((registries, Arc::new(startup)))
        })
        .as_ref()
        .map_err(Clone::clone)
}

pub(crate) fn builtin_registry_tags() -> Arc<RegistryTagSnapshot> {
    builtin_registry_input()
        .expect("the built-in datapack generation loads")
        .0
        .registry_tags_shared()
}

pub(crate) fn builtin_datapack_startup_shared() -> Arc<DatapackStartup> {
    Arc::clone(
        &builtin_registry_input()
            .expect("the live world already proved the built-in registry publication")
            .1,
    )
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod reload_scope_tests;

#[cfg(test)]
mod static_catalog_publication_tests;
