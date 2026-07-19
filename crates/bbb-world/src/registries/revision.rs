use std::collections::BTreeMap;

use bbb_protocol::registry_data_types::{
    synchronized_registry_data_type, VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES,
};

use super::dimension_types::{decode_dimension_type_client_projection, DIMENSION_TYPE_REGISTRY};
use super::element_value::decode_known_pack_element;
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
            build_registered_typed_projections(&mut registries).map_err(|error| {
                RegistryCandidateError {
                    stage: "registry_configuration_typed_values",
                    error,
                }
            })?;
            validate_declared_registry_validators(&registries).map_err(|error| {
                RegistryCandidateError {
                    stage: "registry_configuration_validation",
                    error,
                }
            })?;
        }

        Ok(Self { registries })
    }

    pub(super) fn into_registry_set(self) -> RegistrySet {
        self.registries
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
    Ok(())
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
    if entry.has_data {
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
