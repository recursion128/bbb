use std::collections::BTreeMap;

use bbb_protocol::registry_data_types::{
    synchronized_registry_data_type, VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES,
};

use super::animal_variants::{
    decode_cat_variant_value, decode_farm_animal_variant_value, decode_frog_variant_value,
    decode_wolf_variant_value, decode_zombie_nautilus_variant_value, CAT_VARIANT_REGISTRY,
    CHICKEN_VARIANT_REGISTRY, COW_VARIANT_REGISTRY, FROG_VARIANT_REGISTRY, PIG_VARIANT_REGISTRY,
    WOLF_VARIANT_REGISTRY, ZOMBIE_NAUTILUS_VARIANT_REGISTRY,
};
use super::armor_trims::{
    decode_material as decode_trim_material, decode_pattern as decode_trim_pattern,
    TRIM_MATERIAL_REGISTRY, TRIM_PATTERN_REGISTRY,
};
use super::banner_patterns::{decode_banner_pattern_value, BANNER_PATTERN_REGISTRY};
use super::biomes::decode_biome_client_projection;
use super::chat_types::{decode_chat_type, CHAT_TYPE_REGISTRY};
use super::damage_types::{decode_damage_type_value, DAMAGE_TYPE_REGISTRY};
use super::dimension_types::{decode_dimension_type_client_projection, DIMENSION_TYPE_REGISTRY};
use super::element_value::decode_known_pack_element;
use super::enchantments::{materialize_enchantment_definition, ENCHANTMENT_REGISTRY};
use super::game_tests::{
    decode_test_environment, decode_test_instance, TEST_ENVIRONMENT_REGISTRY,
    TEST_INSTANCE_REGISTRY,
};
use super::instrument_values::{decode_instrument_with_access, INSTRUMENT_REGISTRY};
use super::item_component_initializers::build_registry_dependent_item_defaults;
use super::jukebox_songs::{decode_jukebox_song_with_access, JUKEBOX_SONG_REGISTRY};
use super::painting_variants::{decode_painting_variant, PAINTING_VARIANT_REGISTRY};
use super::sound_variants::{
    decode_sound_variant_definition, validate_sound_variant_references, CAT_SOUND_VARIANT_REGISTRY,
    CHICKEN_SOUND_VARIANT_REGISTRY, COW_SOUND_VARIANT_REGISTRY, PIG_SOUND_VARIANT_REGISTRY,
    WOLF_SOUND_VARIANT_REGISTRY,
};
use super::state::PendingRegistryCollection;
use super::state::RegistryTypedValues;
use super::store::RegistryValidationError;
use super::timelines::{
    decode_timeline_definition, validate_world_clock_value, TimelineDefinition,
    WorldClockDefinition, TIMELINE_REGISTRY, WORLD_CLOCK_REGISTRY,
};
use super::{RegistryContentState, RegistryPacketEntry, RegistrySet, RegistryTagState};

/// One completely prepared replacement for the frozen session registry view.
///
/// Construction owns all fallible work. The live [`RegistrySet`] is swapped
/// only after this value exists, so no element, tag, validator, or future typed
/// projection can leak out of a failed configuration batch.
pub(super) struct RegistryRevisionCandidate {
    registries: RegistrySet,
    item_defaults: BTreeMap<i32, bbb_protocol::packets::DataComponentPatchSummary>,
}

pub(super) struct RegistryCandidateError {
    pub stage: &'static str,
    pub error: RegistryValidationError,
}

impl RegistryRevisionCandidate {
    pub(super) fn build(
        active: &RegistrySet,
        mut pending: PendingRegistryCollection,
        known_pack_source: Option<&bbb_pack::KnownPackDataSource>,
        known_pack_source_error: Option<&str>,
        selected_any_known_pack: bool,
        item_component_initializer_rules: &BTreeMap<
            i32,
            Vec<bbb_pack::RegistryDependentItemComponentInitializer>,
        >,
    ) -> Result<Self, RegistryCandidateError> {
        let replaces_synchronized_contents = !pending.registries.is_empty();
        if replaces_synchronized_contents {
            materialize_empty_synchronized_registries(&mut pending);
        }

        materialize_pending_known_pack_data(
            known_pack_source,
            known_pack_source_error,
            selected_any_known_pack,
            &mut pending,
        )
        .map_err(|error| RegistryCandidateError {
            stage: "registry_configuration_known_packs",
            error,
        })?;
        validate_pending_registry_element_data(&pending).map_err(|error| {
            RegistryCandidateError {
                stage: "registry_configuration_element_data",
                error,
            }
        })?;
        validate_pending_duplicate_keys(&pending).map_err(|error| RegistryCandidateError {
            stage: "registry_configuration_validation",
            error,
        })?;
        materialize_pending_registry_tags(active, &mut pending);

        let mut registries =
            prepare_candidate_registry_set(active, pending, replaces_synchronized_contents);
        if replaces_synchronized_contents {
            validate_declared_registry_validators(&registries).map_err(|error| {
                RegistryCandidateError {
                    stage: "registry_configuration_validation",
                    error,
                }
            })?;
        }
        build_registered_typed_projections(&mut registries).map_err(|error| {
            RegistryCandidateError {
                stage: "registry_configuration_typed_values",
                error,
            }
        })?;

        let item_defaults = build_registry_dependent_item_defaults(
            item_component_initializer_rules,
            registries.frozen_access(),
        )
        .map_err(|error| RegistryCandidateError {
            stage: "registry_configuration_item_components",
            error,
        })?;

        Ok(Self {
            registries,
            item_defaults,
        })
    }

    pub(super) fn into_parts(
        self,
    ) -> (
        RegistrySet,
        BTreeMap<i32, bbb_protocol::packets::DataComponentPatchSummary>,
    ) {
        (self.registries, self.item_defaults)
    }
}

fn prepare_candidate_registry_set(
    active: &RegistrySet,
    mut pending: PendingRegistryCollection,
    replaces_synchronized_contents: bool,
) -> RegistrySet {
    let mut registries = active.clone();
    registries.pending = None;
    let tags_changed = replaces_synchronized_contents || !pending.tags.is_empty();
    if replaces_synchronized_contents {
        for registry in pending.contents.keys() {
            registries.tags.remove(registry);
            registries.unresolved_tag_holders.remove(registry);
        }
        registries.registries = pending.registries;
        registries.contents = pending.contents;
        registries.typed = RegistryTypedValues::default();
        registries.revision.contents = registries.revision.contents.saturating_add(1);
    }
    for registry in pending.tags.keys() {
        registries.unresolved_tag_holders.remove(registry);
    }
    registries.tags.append(&mut pending.tags);
    registries
        .unresolved_tag_holders
        .append(&mut pending.unresolved_tag_holders);
    if tags_changed {
        registries.revision.tags = registries.revision.tags.saturating_add(1);
    }
    registries
}

/// Dispatches the typed codecs that already participate in the atomic
/// candidate. Later family phases extend this match rather than adding lazy
/// validation at consumer call sites.
fn build_registered_typed_projections(
    candidate: &mut RegistrySet,
) -> Result<(), RegistryValidationError> {
    let world_clocks = candidate
        .contents
        .get(WORLD_CLOCK_REGISTRY)
        .into_iter()
        .flat_map(|contents| &contents.entries)
        .map(|entry| {
            let data = required_typed_element_data(
                entry,
                WORLD_CLOCK_REGISTRY,
                "WorldClock.DIRECT_CODEC",
            )?;
            validate_world_clock_value(&entry.id, &data.value).map_err(|error| {
                RegistryValidationError {
                    registry: WORLD_CLOCK_REGISTRY.to_string(),
                    validator: "WorldClock.DIRECT_CODEC".to_string(),
                    message: error.to_string(),
                }
            })?;
            Ok((
                entry.id.clone(),
                WorldClockDefinition {
                    resource_id: entry.id.clone(),
                    source: super::RegistryEntrySource::Server,
                    element_source: Some(data.source),
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>, RegistryValidationError>>()?;
    candidate.typed.world_clocks = world_clocks;

    let timelines = candidate
        .contents
        .get(TIMELINE_REGISTRY)
        .into_iter()
        .flat_map(|contents| &contents.entries)
        .map(|entry| {
            let data =
                required_typed_element_data(entry, TIMELINE_REGISTRY, "Timeline.NETWORK_CODEC")?;
            decode_timeline_definition(
                &entry.id,
                data.source,
                &data.value,
                candidate.frozen_access(),
            )
            .map(|definition| (entry.id.clone(), definition))
            .map_err(|error| RegistryValidationError {
                registry: TIMELINE_REGISTRY.to_string(),
                validator: "Timeline.NETWORK_CODEC".to_string(),
                message: error.to_string(),
            })
        })
        .collect::<Result<BTreeMap<_, _>, RegistryValidationError>>()?;
    validate_timeline_time_markers(&timelines)?;
    candidate.typed.timelines = timelines;

    let dimension_types = candidate
        .contents
        .get(DIMENSION_TYPE_REGISTRY)
        .into_iter()
        .flat_map(|contents| &contents.entries)
        .map(|entry| {
            let data = required_typed_element_data(
                entry,
                DIMENSION_TYPE_REGISTRY,
                "DimensionType.CLIENT_PROJECTION",
            )?;
            decode_dimension_type_client_projection(
                &entry.id,
                &data.value,
                candidate.frozen_access(),
            )
            .map(|projection| (entry.id.clone(), projection))
            .map_err(|message| RegistryValidationError {
                registry: DIMENSION_TYPE_REGISTRY.to_string(),
                validator: "DimensionType.CLIENT_PROJECTION".to_string(),
                message,
            })
        })
        .collect::<Result<BTreeMap<_, _>, RegistryValidationError>>()?;
    candidate.typed.dimension_types = dimension_types;

    candidate.typed.biomes = materialize_typed_registry(
        candidate,
        "minecraft:worldgen/biome",
        "Biome.NETWORK_CODEC",
        |entry, _, access| {
            let projection = decode_biome_client_projection(entry, access)
                .map_err(|error| format!("{}: {}", error.field, error.reason))?
                .ok_or_else(|| "<root>: typed payload unavailable".to_string())?;
            projection
                .environment_attribute_modifiers
                .validate_references(access)?;
            Ok(projection)
        },
    )?;
    candidate.typed.chat_types = materialize_typed_registry(
        candidate,
        CHAT_TYPE_REGISTRY,
        "ChatType.DIRECT_CODEC",
        |_, data, _| {
            decode_chat_type(&data.value)
                .ok_or_else(|| "<root>: malformed ChatType.DIRECT_CODEC value".to_string())
        },
    )?;
    candidate.typed.damage_types = materialize_typed_registry(
        candidate,
        DAMAGE_TYPE_REGISTRY,
        "DamageType.DIRECT_CODEC",
        |_, data, _| decode_damage_type_value(&data.value).map_err(|error| error.to_string()),
    )?;
    candidate.typed.enchantments = materialize_typed_registry(
        candidate,
        ENCHANTMENT_REGISTRY,
        "Enchantment.DIRECT_CODEC",
        |_, data, access| materialize_enchantment_definition(&data.value, access),
    )?;
    candidate.typed.instruments = materialize_typed_registry(
        candidate,
        INSTRUMENT_REGISTRY,
        "Instrument.DIRECT_CODEC",
        |_, data, access| {
            decode_instrument_with_access(&data.value, data.source, Some(access))
                .map_err(|error| format!("{}: {}", error.field, error.reason))
        },
    )?;
    candidate.typed.jukebox_songs = materialize_typed_registry(
        candidate,
        JUKEBOX_SONG_REGISTRY,
        "JukeboxSong.DIRECT_CODEC",
        |_, data, access| {
            decode_jukebox_song_with_access(&data.value, Some(access))
                .map_err(|error| format!("{}: {}", error.field, error.reason))
        },
    )?;
    candidate.typed.painting_variants = materialize_typed_registry(
        candidate,
        PAINTING_VARIANT_REGISTRY,
        "PaintingVariant.DIRECT_CODEC",
        |_, data, _| decode_painting_variant(&data.value).map_err(|error| error.to_string()),
    )?;
    candidate.typed.trim_materials = materialize_typed_registry(
        candidate,
        TRIM_MATERIAL_REGISTRY,
        "TrimMaterial.DIRECT_CODEC",
        |_, data, _| {
            decode_trim_material(&data.value)
                .map_err(|error| format!("{}: {}", error.field, error.reason))
        },
    )?;
    candidate.typed.trim_patterns = materialize_typed_registry(
        candidate,
        TRIM_PATTERN_REGISTRY,
        "TrimPattern.DIRECT_CODEC",
        |_, data, _| {
            decode_trim_pattern(&data.value)
                .map_err(|error| format!("{}: {}", error.field, error.reason))
        },
    )?;
    candidate.typed.banner_patterns = materialize_typed_registry(
        candidate,
        BANNER_PATTERN_REGISTRY,
        "BannerPattern.DIRECT_CODEC",
        |_, data, _| {
            decode_banner_pattern_value(&data.value)
                .map_err(|error| format!("{}: {}", error.field, error.reason))
        },
    )?;
    for registry in [
        CHICKEN_VARIANT_REGISTRY,
        COW_VARIANT_REGISTRY,
        PIG_VARIANT_REGISTRY,
    ] {
        let variants = materialize_typed_registry(
            candidate,
            registry,
            "FarmAnimalVariant.NETWORK_CODEC",
            |_, data, _| {
                decode_farm_animal_variant_value(registry, &data.value)
                    .map_err(|error| format!("{}: {}", error.field, error.reason))
            },
        )?;
        candidate
            .typed
            .farm_animal_variants
            .insert(registry.to_string(), variants);
    }
    candidate.typed.cat_variants = materialize_typed_registry(
        candidate,
        CAT_VARIANT_REGISTRY,
        "CatVariant.NETWORK_CODEC",
        |_, data, _| {
            decode_cat_variant_value(&data.value)
                .map_err(|error| format!("{}: {}", error.field, error.reason))
        },
    )?;
    candidate.typed.frog_variants = materialize_typed_registry(
        candidate,
        FROG_VARIANT_REGISTRY,
        "FrogVariant.NETWORK_CODEC",
        |_, data, _| {
            decode_frog_variant_value(&data.value)
                .map_err(|error| format!("{}: {}", error.field, error.reason))
        },
    )?;
    candidate.typed.wolf_variants = materialize_typed_registry(
        candidate,
        WOLF_VARIANT_REGISTRY,
        "WolfVariant.NETWORK_CODEC",
        |_, data, _| {
            decode_wolf_variant_value(&data.value)
                .map_err(|error| format!("{}: {}", error.field, error.reason))
        },
    )?;
    candidate.typed.zombie_nautilus_variants = materialize_typed_registry(
        candidate,
        ZOMBIE_NAUTILUS_VARIANT_REGISTRY,
        "ZombieNautilusVariant.NETWORK_CODEC",
        |_, data, _| {
            decode_zombie_nautilus_variant_value(&data.value)
                .map_err(|error| format!("{}: {}", error.field, error.reason))
        },
    )?;

    for (registry, codec) in [
        (CAT_SOUND_VARIANT_REGISTRY, "CatSoundVariant.DIRECT_CODEC"),
        (
            CHICKEN_SOUND_VARIANT_REGISTRY,
            "ChickenSoundVariant.DIRECT_CODEC",
        ),
        (COW_SOUND_VARIANT_REGISTRY, "CowSoundVariant.DIRECT_CODEC"),
        (PIG_SOUND_VARIANT_REGISTRY, "PigSoundVariant.DIRECT_CODEC"),
        (WOLF_SOUND_VARIANT_REGISTRY, "WolfSoundVariant.DIRECT_CODEC"),
    ] {
        let variants = candidate
            .contents
            .get(registry)
            .into_iter()
            .flat_map(|contents| &contents.entries)
            .map(|entry| {
                let data = required_typed_element_data(entry, registry, codec)?;
                let variant = decode_sound_variant_definition(registry, data.source, &data.value)
                    .map_err(|error| RegistryValidationError {
                    registry: registry.to_string(),
                    validator: codec.to_string(),
                    message: format!("{} {}: {}", entry.id, error.field, error.reason),
                })?;
                validate_sound_variant_references(&variant, candidate.frozen_access()).map_err(
                    |message| RegistryValidationError {
                        registry: registry.to_string(),
                        validator: codec.to_string(),
                        message: format!("{} {message}", entry.id),
                    },
                )?;
                Ok((entry.id.clone(), variant))
            })
            .collect::<Result<BTreeMap<_, _>, RegistryValidationError>>()?;
        candidate
            .typed
            .sound_variants
            .insert(registry.to_string(), variants);
    }

    let test_environments = candidate
        .contents
        .get(TEST_ENVIRONMENT_REGISTRY)
        .into_iter()
        .flat_map(|contents| &contents.entries)
        .map(|entry| {
            let codec = "TestEnvironmentDefinition.DIRECT_CODEC";
            let data = required_typed_element_data(entry, TEST_ENVIRONMENT_REGISTRY, codec)?;
            decode_test_environment(
                &entry.id,
                data.source,
                &data.value,
                data.known_pack_json.as_ref(),
                candidate.frozen_access(),
                0,
                "<root>",
            )
            .map(|definition| (entry.id.clone(), definition))
            .map_err(|error| RegistryValidationError {
                registry: TEST_ENVIRONMENT_REGISTRY.to_string(),
                validator: codec.to_string(),
                message: error.to_string(),
            })
        })
        .collect::<Result<BTreeMap<_, _>, RegistryValidationError>>()?;
    candidate.typed.test_environments = test_environments;
    candidate.typed.test_instances = materialize_typed_registry(
        candidate,
        TEST_INSTANCE_REGISTRY,
        "TestInstance.DIRECT_CODEC",
        |entry, data, access| {
            decode_test_instance(
                &entry.id,
                data.source,
                &data.value,
                data.known_pack_json.as_ref(),
                access,
            )
            .map_err(|error| error.to_string())
        },
    )?;
    Ok(())
}

fn materialize_typed_registry<T>(
    candidate: &RegistrySet,
    registry: &str,
    validator: &str,
    decode: impl Fn(
        &RegistryPacketEntry,
        &super::RegistryElementData,
        super::FrozenRegistryAccess<'_>,
    ) -> Result<T, String>,
) -> Result<BTreeMap<String, T>, RegistryValidationError> {
    candidate
        .contents
        .get(registry)
        .into_iter()
        .flat_map(|contents| &contents.entries)
        .filter_map(|entry| {
            let Some(data) = entry.element_data() else {
                return entry.has_data.then(|| {
                    Err(RegistryValidationError {
                        registry: registry.to_string(),
                        validator: validator.to_string(),
                        message: format!("{} typed payload unavailable", entry.id),
                    })
                });
            };
            decode(entry, data, candidate.frozen_access())
                .map(|value| (entry.id.clone(), value))
                .map_err(|message| RegistryValidationError {
                    registry: registry.to_string(),
                    validator: validator.to_string(),
                    message: format!("{} {message}", entry.id),
                })
                .into()
        })
        .collect()
}

fn required_typed_element_data<'a>(
    entry: &'a RegistryPacketEntry,
    registry: &str,
    validator: &str,
) -> Result<&'a super::RegistryElementData, RegistryValidationError> {
    entry.element_data().ok_or_else(|| RegistryValidationError {
        registry: registry.to_string(),
        validator: validator.to_string(),
        message: format!(
            "{} has neither network element data nor selected known-pack data",
            entry.id
        ),
    })
}

fn validate_timeline_time_markers(
    timelines: &BTreeMap<String, TimelineDefinition>,
) -> Result<(), RegistryValidationError> {
    let mut markers_by_clock = BTreeMap::<(&str, &str), &str>::new();
    for timeline in timelines.values() {
        for marker in &timeline.time_markers {
            if let Some(previous) = markers_by_clock.insert(
                (&timeline.clock.resource_id, &marker.resource_id),
                &timeline.resource_id,
            ) {
                return Err(RegistryValidationError {
                    registry: TIMELINE_REGISTRY.to_string(),
                    validator: "Timeline.validateRegistry".to_string(),
                    message: format!(
                        "time marker {} is defined by both {previous} and {} for clock {}",
                        marker.resource_id, timeline.resource_id, timeline.clock.resource_id
                    ),
                });
            }
        }
    }
    Ok(())
}

/// `RegistryDataCollector` creates every synchronized registry load task once
/// it has received any contents packet. A registry omitted from that batch is
/// therefore an authoritative empty registry, not permission to reuse the
/// previous session or vanilla bootstrap order.
fn materialize_empty_synchronized_registries(pending: &mut PendingRegistryCollection) {
    for data_type in VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {
        pending
            .contents
            .entry(data_type.resource_id.to_string())
            .or_insert_with(|| RegistryContentState::new(data_type.resource_id));
    }
}

fn materialize_pending_known_pack_data(
    source: Option<&bbb_pack::KnownPackDataSource>,
    source_error: Option<&str>,
    selected_any: bool,
    pending: &mut PendingRegistryCollection,
) -> Result<(), RegistryValidationError> {
    if let Some(message) = source_error {
        return Err(RegistryValidationError {
            registry: "minecraft:root".to_string(),
            validator: "KnownPacksManager.createResourceManager".to_string(),
            message: message.to_string(),
        });
    }
    if !selected_any {
        return Ok(());
    }
    let Some(source) = source else {
        return Err(RegistryValidationError {
            registry: "minecraft:root".to_string(),
            validator: "KnownPacksManager.createResourceManager".to_string(),
            message: "selected known packs have no local SERVER_DATA source".to_string(),
        });
    };

    for (registry, contents) in &mut pending.contents {
        for entry in &mut contents.entries {
            materialize_known_pack_entry(source, registry, entry)?;
        }
    }
    for packet in &mut pending.registries {
        for entry in &mut packet.entries {
            materialize_known_pack_entry(source, &packet.name, entry)?;
        }
    }
    Ok(())
}

fn materialize_known_pack_entry(
    source: &bbb_pack::KnownPackDataSource,
    registry: &str,
    entry: &mut RegistryPacketEntry,
) -> Result<(), RegistryValidationError> {
    if entry.has_data || entry.known_pack_data.is_some() {
        return Ok(());
    }
    let Some(data) = source.registry_element_json_arc(registry, &entry.id) else {
        return Err(RegistryValidationError {
            registry: registry.to_string(),
            validator: "KnownPacksManager.createResourceManager".to_string(),
            message: format!(
                "selected known packs do not provide registry element {registry}/{}",
                entry.id
            ),
        });
    };
    let element_data =
        decode_known_pack_element(&data).map_err(|error| RegistryValidationError {
            registry: registry.to_string(),
            validator: "RegistryDataLoader.elementCodec".to_string(),
            message: format!(
                "failed to decode known-pack registry element {registry}/{}: {error}",
                entry.id
            ),
        })?;
    entry.known_pack_data = Some(data);
    entry.element_data = Some(std::sync::Arc::new(element_data));
    entry.element_data_error = None;
    Ok(())
}

fn validate_pending_registry_element_data(
    pending: &PendingRegistryCollection,
) -> Result<(), RegistryValidationError> {
    for (registry, contents) in &pending.contents {
        for entry in &contents.entries {
            if let Some(error) = &entry.element_data_error {
                return Err(RegistryValidationError {
                    registry: registry.clone(),
                    validator: "RegistryDataLoader.elementCodec".to_string(),
                    message: format!(
                        "failed to decode network registry element {registry}/{}: {error}",
                        entry.id
                    ),
                });
            }
        }
    }
    Ok(())
}

fn validate_pending_duplicate_keys(
    pending: &PendingRegistryCollection,
) -> Result<(), RegistryValidationError> {
    for (registry, contents) in &pending.contents {
        if contents.duplicate_entry_count() > 0 {
            let duplicate_ids = contents
                .duplicate_entry_ids
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            return Err(RegistryValidationError {
                registry: registry.clone(),
                validator: "MappedRegistry.register".to_string(),
                message: format!("Adding duplicate keys to registry {registry}: {duplicate_ids}"),
            });
        }
    }
    Ok(())
}

fn validate_declared_registry_validators(
    candidate: &RegistrySet,
) -> Result<(), RegistryValidationError> {
    for (registry, contents) in &candidate.contents {
        let Some(validator) =
            synchronized_registry_data_type(registry).and_then(|data| data.validator)
        else {
            continue;
        };
        match validator {
            "RegistryValidator.nonEmpty ()" if contents.entries.is_empty() => {
                return Err(RegistryValidationError {
                    registry: registry.clone(),
                    validator: validator.to_string(),
                    message: format!("Registry must be non-empty: {registry}"),
                });
            }
            "RegistryValidator.nonEmpty ()" => {}
            _ => {
                return Err(RegistryValidationError {
                    registry: registry.clone(),
                    validator: validator.to_string(),
                    message: format!(
                        "unknown synchronized registry validator {validator}: {registry}"
                    ),
                });
            }
        }
    }
    Ok(())
}

fn materialize_pending_registry_tags(
    active: &RegistrySet,
    pending: &mut PendingRegistryCollection,
) {
    let raw_tags = std::mem::take(&mut pending.tags);
    for (_, raw) in raw_tags {
        let (resolved, unresolved) =
            materialize_registry_tags(active, Some(&pending.contents), raw);
        let registry = resolved.registry.clone();
        pending.tags.insert(registry.clone(), resolved);
        if !unresolved.is_empty() {
            pending.unresolved_tag_holders.insert(registry, unresolved);
        }
    }
}

pub(super) fn materialize_registry_tags(
    active: &RegistrySet,
    pending_contents: Option<&BTreeMap<String, RegistryContentState>>,
    raw: RegistryTagState,
) -> (RegistryTagState, BTreeMap<String, Vec<i32>>) {
    let RegistryTagState { registry, tags } = raw;
    let mut resolved_tags = BTreeMap::new();
    let mut unresolved_tags = BTreeMap::new();
    for (tag, holder_ids) in tags {
        let mut resolved = Vec::with_capacity(holder_ids.len());
        let mut unresolved = Vec::new();
        for holder_id in holder_ids {
            let present = pending_contents
                .and_then(|contents| contents.get(&registry))
                .map_or_else(
                    || {
                        active
                            .resolved_entry_by_holder_id(&registry, holder_id)
                            .is_some()
                    },
                    |contents| contents.entry_by_holder_id(holder_id).is_some(),
                );
            if present {
                resolved.push(holder_id);
            } else {
                unresolved.push(holder_id);
            }
        }
        resolved_tags.insert(tag.clone(), resolved);
        if !unresolved.is_empty() {
            unresolved_tags.insert(tag, unresolved);
        }
    }
    (
        RegistryTagState {
            registry,
            tags: resolved_tags,
        },
        unresolved_tags,
    )
}
