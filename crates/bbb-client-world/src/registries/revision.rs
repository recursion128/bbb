use std::collections::BTreeMap;

use bbb_protocol::registry_data_types::{
    synchronized_registry_data_type, VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES,
};
use bbb_registry_codec::{
    decode_exact_synchronized_registry_element_with_content_registry,
    registry_codec::{
        direct_leaf::DirectLeafRegistryElement, game_tests::GameTestElementView,
        SynchronizedRegistryElementValue,
    },
    synchronized_registry_element_codec,
};

use super::animal_variants::{
    CHICKEN_VARIANT_REGISTRY, COW_VARIANT_REGISTRY, PIG_VARIANT_REGISTRY,
};
use super::armor_trims::{
    decode_validated_material as decode_trim_material,
    decode_validated_pattern as decode_trim_pattern, TRIM_MATERIAL_REGISTRY, TRIM_PATTERN_REGISTRY,
};
use super::chat_types::{decode_validated_chat_type, CHAT_TYPE_REGISTRY};
use super::dimension_types::{decode_dimension_type_client_projection, DIMENSION_TYPE_REGISTRY};
use super::element_value::decode_known_pack_element;
use super::enchantments::{materialize_enchantment_definition, ENCHANTMENT_REGISTRY};
use super::instrument_values::{decode_validated_instrument, INSTRUMENT_REGISTRY};
use super::item_component_initializers::build_item_component_defaults;
use super::jukebox_songs::{decode_validated_jukebox_song, JUKEBOX_SONG_REGISTRY};
use super::painting_variants::{decode_validated_painting_variant, PAINTING_VARIANT_REGISTRY};
use super::sound_variants::{
    CAT_SOUND_VARIANT_REGISTRY, CHICKEN_SOUND_VARIANT_REGISTRY, COW_SOUND_VARIANT_REGISTRY,
    PIG_SOUND_VARIANT_REGISTRY, WOLF_SOUND_VARIANT_REGISTRY,
};
use super::state::PendingRegistryCollection;
use super::state::RegistryTypedValues;
use super::store::RegistryValidationError;
use super::timelines::{decode_timeline_definition, TIMELINE_REGISTRY, WORLD_CLOCK_REGISTRY};
use super::{
    RegistryContentState, RegistryElementSource, RegistryPacketEntry, RegistrySet, RegistryTagState,
};

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
        known_pack_source: Option<&bbb_client_assets::KnownPackDataSource>,
        known_pack_source_error: Option<&str>,
        selected_any_known_pack: bool,
        item_component_codec_values: &rustc_hash::FxHashMap<
            i32,
            rustc_hash::FxHashMap<i32, serde_json::Value>,
        >,
    ) -> Result<Self, RegistryCandidateError> {
        let replaces_synchronized_contents = !pending.registries.is_empty();
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
        for index in pending.content_packet_indices.drain(..) {
            let packet = &pending.registries[index];
            pending
                .contents
                .entry(packet.name.clone())
                .or_insert_with(|| RegistryContentState::new(&packet.name))
                .append_packet(&packet.entries);
        }
        if replaces_synchronized_contents {
            materialize_empty_synchronized_registries(&mut pending);
        }
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
        registries.typed =
            build_exact_registry_projections(&registries, replaces_synchronized_contents)?;

        let item_defaults = build_item_component_defaults(
            item_component_codec_values,
            Some(registries.frozen_access()),
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

/// Validates exact codecs against the complete generation and retains their
/// structured client projections for publication.
fn build_exact_registry_projections(
    candidate: &RegistrySet,
    require_element_source: bool,
) -> Result<RegistryTypedValues, RegistryCandidateError> {
    let exact_error = |error| RegistryCandidateError {
        stage: "registry_configuration_exact_codecs",
        error,
    };
    let typed_error = |error| RegistryCandidateError {
        stage: "registry_configuration_typed_values",
        error,
    };
    let access = candidate.frozen_access();
    let mut typed = RegistryTypedValues {
        farm_animal_variants: [
            CHICKEN_VARIANT_REGISTRY,
            COW_VARIANT_REGISTRY,
            PIG_VARIANT_REGISTRY,
        ]
        .into_iter()
        .map(|registry| (registry.to_string(), BTreeMap::new()))
        .collect(),
        sound_variants: [
            CAT_SOUND_VARIANT_REGISTRY,
            CHICKEN_SOUND_VARIANT_REGISTRY,
            COW_SOUND_VARIANT_REGISTRY,
            PIG_SOUND_VARIANT_REGISTRY,
            WOLF_SOUND_VARIANT_REGISTRY,
        ]
        .into_iter()
        .map(|registry| (registry.to_string(), BTreeMap::new()))
        .collect(),
        ..RegistryTypedValues::default()
    };
    for data_type in VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {
        for entry in candidate
            .contents
            .get(data_type.resource_id)
            .into_iter()
            .flat_map(|contents| &contents.entries)
        {
            let Some(data) = entry.element_data() else {
                if require_element_source || entry.has_data {
                    return Err(exact_error(RegistryValidationError {
                        registry: data_type.resource_id.to_string(),
                        validator: "RegistryDataLoader.elementCodec".to_string(),
                        message: format!("{} typed payload unavailable", entry.id),
                    }));
                }
                if matches!(
                    data_type.resource_id,
                    DIMENSION_TYPE_REGISTRY
                        | WORLD_CLOCK_REGISTRY
                        | TIMELINE_REGISTRY
                        | CAT_SOUND_VARIANT_REGISTRY
                        | CHICKEN_SOUND_VARIANT_REGISTRY
                        | COW_SOUND_VARIANT_REGISTRY
                        | PIG_SOUND_VARIANT_REGISTRY
                        | WOLF_SOUND_VARIANT_REGISTRY
                ) {
                    required_typed_element_data(entry, data_type.resource_id)
                        .map_err(typed_error)?;
                }
                continue;
            };
            let decoded = decode_exact_synchronized_registry_element_with_content_registry(
                data_type.resource_id,
                data,
                access.content_registry(),
            )
            .map_err(|error| exact_error(registry_codec_entry_error(&entry.id, error)))?;
            decoded
                .validate_references(&access)
                .map_err(|error| exact_error(registry_codec_entry_error(&entry.id, error)))?;
            let projection_error = |message: String| {
                typed_error(RegistryValidationError {
                    registry: data_type.resource_id.to_string(),
                    validator: synchronized_element_codec(data_type.resource_id, data.source)
                        .expect("generated synchronized registry codec")
                        .to_string(),
                    message: format!("{} {message}", entry.id),
                })
            };
            match decoded.into_value() {
                SynchronizedRegistryElementValue::DimensionType(dimension) => {
                    let projection =
                        decode_dimension_type_client_projection(&entry.id, dimension, access)
                            .map_err(|message| {
                                typed_error(RegistryValidationError {
                                    registry: DIMENSION_TYPE_REGISTRY.to_string(),
                                    validator: synchronized_element_codec(
                                        DIMENSION_TYPE_REGISTRY,
                                        data.source,
                                    )
                                    .expect("generated synchronized registry codec")
                                    .to_string(),
                                    message,
                                })
                            })?;
                    typed.dimension_types.insert(entry.id.clone(), projection);
                }
                SynchronizedRegistryElementValue::DirectLeaf(element) => match element {
                    DirectLeafRegistryElement::DamageType(value) => {
                        typed.damage_types.insert(entry.id.clone(), value.effects);
                    }
                    DirectLeafRegistryElement::BannerPattern(value) => {
                        typed.banner_patterns.insert(entry.id.clone(), value);
                    }
                    DirectLeafRegistryElement::WorldClock => {}
                },
                SynchronizedRegistryElementValue::AnimalVariant(element) => {
                    typed.insert_animal_variant(&entry.id, element);
                }
                SynchronizedRegistryElementValue::SoundVariant(element) => {
                    typed
                        .sound_variants
                        .get_mut(data_type.resource_id)
                        .expect("sound-variant registry initialized")
                        .insert(entry.id.clone(), element);
                }
                SynchronizedRegistryElementValue::GameTest(element) => {
                    if matches!(element.view(), GameTestElementView::Instance(_)) {
                        typed.test_instances.insert(entry.id.clone(), element);
                    }
                }
                SynchronizedRegistryElementValue::Canonical(value) => {
                    typed
                        .insert_canonical_projection(
                            data_type.resource_id,
                            &entry.id,
                            value.network_nbt,
                            access,
                        )
                        .map_err(projection_error)?;
                }
            }
        }
    }
    Ok(typed)
}

fn registry_codec_entry_error(
    resource_id: &str,
    error: bbb_registry_codec::RegistryCodecError,
) -> RegistryValidationError {
    RegistryValidationError {
        registry: error.registry,
        validator: error.codec,
        message: format!("{resource_id} {}: {}", error.field, error.reason),
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
        registries.cached_bars_block_types.take();
    }
    registries
}

impl RegistryTypedValues {
    fn insert_canonical_projection(
        &mut self,
        registry: &str,
        resource_id: &str,
        network: bbb_protocol::NbtSummaryValue,
        access: super::FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        match registry {
            TIMELINE_REGISTRY => {
                let value = decode_timeline_definition(resource_id, &network, access)
                    .map_err(|error| format!("{}: {}", error.field, error.reason))?;
                self.timelines.insert(resource_id.to_string(), value);
            }
            "minecraft:worldgen/biome" => {
                self.insert_biome_projection(resource_id, &network, access)?;
            }
            CHAT_TYPE_REGISTRY => {
                let value = decode_validated_chat_type(network);
                self.chat_types.insert(resource_id.to_string(), value);
            }
            ENCHANTMENT_REGISTRY => {
                let value = materialize_enchantment_definition(network, access)?;
                self.enchantments.insert(resource_id.to_string(), value);
            }
            INSTRUMENT_REGISTRY => {
                let value = decode_validated_instrument(network);
                self.instruments.insert(resource_id.to_string(), value);
            }
            JUKEBOX_SONG_REGISTRY => {
                let value = decode_validated_jukebox_song(network);
                self.jukebox_songs.insert(resource_id.to_string(), value);
            }
            PAINTING_VARIANT_REGISTRY => {
                let value = decode_validated_painting_variant(network);
                self.painting_variants
                    .insert(resource_id.to_string(), value);
            }
            TRIM_MATERIAL_REGISTRY => {
                let value = decode_trim_material(network)?;
                self.trim_materials.insert(resource_id.to_string(), value);
            }
            TRIM_PATTERN_REGISTRY => {
                let value = decode_trim_pattern(network);
                self.trim_patterns.insert(resource_id.to_string(), value);
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
pub(super) fn exact_network_value(
    registry: &str,
    data: &super::RegistryElementData,
    access: super::FrozenRegistryAccess<'_>,
) -> Result<bbb_protocol::NbtSummaryValue, String> {
    let decoded = decode_exact_synchronized_registry_element_with_content_registry(
        registry,
        data,
        access.content_registry(),
    )
    .map_err(|error| format!("{}: {}", error.field, error.reason))?;
    decoded
        .encode_network(&access)
        .map_err(|error| format!("{}: {}", error.field, error.reason))
}

fn required_typed_element_data<'a>(
    entry: &'a RegistryPacketEntry,
    registry: &str,
) -> Result<&'a super::RegistryElementData, RegistryValidationError> {
    entry.element_data().ok_or_else(|| RegistryValidationError {
        registry: registry.to_string(),
        validator: "RegistryDataLoader.elementCodec".to_string(),
        message: format!(
            "{} has neither network element data nor selected known-pack data",
            entry.id
        ),
    })
}

/// Names the authoritative codec for the element representation being read.
///
/// This is diagnostic attribution, not a claim that every client projection
/// below implements the complete persistent or network codec. Server
/// publication still requires the shared typed decode/encode owner.
fn synchronized_element_codec(
    registry: &str,
    source: RegistryElementSource,
) -> Result<&'static str, RegistryValidationError> {
    synchronized_registry_element_codec(registry, source).map_err(|error| RegistryValidationError {
        registry: error.registry,
        validator: error.codec,
        message: format!("{}: {}", error.field, error.reason),
    })
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
    source: Option<&bbb_client_assets::KnownPackDataSource>,
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

    for packet in &mut pending.registries {
        for entry in &mut packet.entries {
            materialize_known_pack_entry(source, &packet.name, entry)?;
        }
    }
    Ok(())
}

fn materialize_known_pack_entry(
    source: &bbb_client_assets::KnownPackDataSource,
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

pub(super) const MAX_TAGS_PER_REGISTRY: usize = 8_192;
const MAX_UNRESOLVED_TAG_HOLDERS_PER_TAG: usize = 16;
const MAX_UNRESOLVED_TAG_HOLDERS_PER_REGISTRY: usize = 256;

pub(super) fn registry_is_known_for_tags(
    active: &RegistrySet,
    pending_contents: Option<&BTreeMap<String, RegistryContentState>>,
    registry: &str,
) -> bool {
    pending_contents.is_some_and(|contents| contents.contains_key(registry))
        || active.contents.contains_key(registry)
        || bbb_protocol::registry_data_types::built_in_registry_type(registry).is_some()
        || synchronized_registry_data_type(registry).is_some()
        || super::state::registry_has_exact_vanilla_fallback(registry)
}

fn materialize_pending_registry_tags(
    active: &RegistrySet,
    pending: &mut PendingRegistryCollection,
) {
    let raw_tags = std::mem::take(&mut pending.tags);
    for (_, raw) in raw_tags {
        if !registry_is_known_for_tags(active, Some(&pending.contents), &raw.registry) {
            continue;
        }
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
    let RegistryTagState { registry, tags, .. } = raw;
    let mut resolved_tags = BTreeMap::new();
    let mut unresolved_tags = BTreeMap::new();
    let mut unresolved_registry_total = 0usize;
    const MAX_TAG_HOLDERS_PER_TAG: usize = 65_536;
    for (tag, holder_ids) in tags.into_iter().take(MAX_TAGS_PER_REGISTRY) {
        let mut resolved = Vec::new();
        let mut unresolved = Vec::new();
        let mut seen_unresolved = rustc_hash::FxHashSet::default();
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
                if resolved.len() < MAX_TAG_HOLDERS_PER_TAG {
                    resolved.push(holder_id);
                }
            } else if unresolved.len() < MAX_UNRESOLVED_TAG_HOLDERS_PER_TAG
                && unresolved_registry_total < MAX_UNRESOLVED_TAG_HOLDERS_PER_REGISTRY
                && seen_unresolved.insert(holder_id)
            {
                unresolved.push(holder_id);
                unresolved_registry_total += 1;
            }
        }
        resolved_tags.insert(tag.clone(), resolved);
        if !unresolved.is_empty() {
            unresolved_tags.insert(tag, unresolved);
        }
    }
    (
        RegistryTagState::from_materialized(registry, resolved_tags),
        unresolved_tags,
    )
}
