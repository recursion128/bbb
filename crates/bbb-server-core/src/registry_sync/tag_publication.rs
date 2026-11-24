//! Freeze loaded tag element ids into the raw holder ids published on the wire.
//!
//! `TagNetworkSerialization.serializeToNetwork` asks each frozen registry for the
//! numeric id of every tag holder (`TagNetworkSerialization.java:20-45`). For a
//! synchronized registry that id is the element's position in the corresponding
//! [`PublishedRegistry`]; for a supported static registry it is the position in
//! BBB's generated registration-order catalog. Vanilla includes every STATIC
//! registry (`RegistrySynchronization.java:81-84`); BBB's catalog is still partial.

use std::collections::HashMap;

use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::packets::{PublishedRegistry, RegistryTags, TagNetworkPayload, UpdateTags};

use bbb_server_datapack::dynamic_registry::DatapackGeneration;
use bbb_server_datapack::tag_loader::{
    network_safe_static_registry_elements_with_content_registry, LoadedRegistryTags, TagLoadReport,
};

use super::RegistrySyncError;

/// Pack every defined tag from the network-safe registries of one generation.
///
/// Report registry order, tag order, and tag member order are all wire-visible and
/// are retained exactly. Resource ids are never sorted and a missing holder is
/// always rejected: raw ids belong to the registry that was actually frozen.
pub(super) fn pack_network_tags(
    registries: &[PublishedRegistry],
    generation: &DatapackGeneration,
    tags: &TagLoadReport,
    content_registry: &ContentRegistry,
) -> Result<UpdateTags, RegistrySyncError> {
    let static_registries =
        network_safe_static_registry_elements_with_content_registry(content_registry);
    let mut packed_registries = Vec::with_capacity(tags.registries.len());

    for loaded in &tags.registries {
        if loaded.tags.is_empty() {
            continue;
        }

        let published = unique_published_registry(registries, loaded)?;
        let static_catalog = static_registries
            .iter()
            .find(|(registry_id, _)| *registry_id == loaded.registry_id);
        let holder_ids = match (published, static_catalog) {
            (Some(_), Some(_)) => {
                return Err(sync_error(
                    loaded,
                    "",
                    "registry is present in both synchronized publication and static catalog",
                ));
            }
            (Some(published), None) => holder_ids(
                loaded,
                published.entries.iter().map(|entry| entry.id.as_str()),
            )?,
            (None, Some((_, elements))) => holder_ids(loaded, elements.iter().copied())?,
            // The tag report also carries worldgen-only dynamic registries.
            // Vanilla filters those out through `isNetworkable` before
            // concatenating the synchronized and static layers
            // (`RegistrySynchronization.java:61-68`).
            (None, None) if generation.registry(&loaded.registry_id).is_some() => continue,
            (None, None) => {
                return Err(sync_error(
                    loaded,
                    "",
                    "tag report names an unknown publication registry",
                ));
            }
        };

        let mut packed_tags = Vec::with_capacity(loaded.tags.len());
        for (tag_id, members) in &loaded.tags {
            let mut entries = Vec::with_capacity(members.len());
            for member in members {
                let raw_id = holder_ids.get(member.as_str()).ok_or_else(|| {
                    sync_error(
                        loaded,
                        member,
                        format!("tag {tag_id} names a holder absent from the frozen registry"),
                    )
                })?;
                entries.push(*raw_id);
            }
            packed_tags.push(TagNetworkPayload {
                tag: tag_id.clone(),
                entries,
            });
        }

        packed_registries.push(RegistryTags {
            registry: loaded.registry_id.clone(),
            tags: packed_tags,
        });
    }

    Ok(UpdateTags {
        registries: packed_registries,
    })
}

fn unique_published_registry<'a>(
    registries: &'a [PublishedRegistry],
    loaded: &LoadedRegistryTags,
) -> Result<Option<&'a PublishedRegistry>, RegistrySyncError> {
    let mut matching = registries
        .iter()
        .filter(|registry| registry.registry == loaded.registry_id);
    let first = matching.next();
    if matching.next().is_some() {
        return Err(sync_error(
            loaded,
            "",
            "synchronized publication contains the registry more than once",
        ));
    }
    Ok(first)
}

fn holder_ids<'a>(
    loaded: &LoadedRegistryTags,
    elements: impl Iterator<Item = &'a str>,
) -> Result<HashMap<&'a str, i32>, RegistrySyncError> {
    let mut holder_ids = HashMap::new();
    for (position, element_id) in elements.enumerate() {
        let raw_id = i32::try_from(position).map_err(|_| {
            sync_error(
                loaded,
                element_id,
                "holder position exceeds the protocol's signed 32-bit raw-id range",
            )
        })?;
        if holder_ids.insert(element_id, raw_id).is_some() {
            return Err(sync_error(
                loaded,
                element_id,
                "frozen holder catalog contains a duplicate resource id",
            ));
        }
    }
    Ok(holder_ids)
}

fn sync_error(
    loaded: &LoadedRegistryTags,
    element_id: impl Into<String>,
    message: impl Into<String>,
) -> RegistrySyncError {
    RegistrySyncError {
        registry_id: loaded.registry_id.clone(),
        element_id: element_id.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::packets::{PublishedRegistry, RegistryDataEntry};

    use super::*;

    fn generation() -> DatapackGeneration {
        DatapackGeneration {
            registries: Vec::new(),
            worldgen: Default::default(),
        }
    }

    fn published(registry: &str, ids: &[&str]) -> PublishedRegistry {
        PublishedRegistry {
            registry: registry.to_string(),
            entries: ids
                .iter()
                .map(|id| RegistryDataEntry {
                    id: (*id).to_string(),
                    raw_data: Some(vec![1]),
                })
                .collect(),
        }
    }

    fn loaded(registry_id: &str, tags: &[(&str, &[&str])]) -> LoadedRegistryTags {
        LoadedRegistryTags {
            registry_id: registry_id.to_string(),
            tags: tags
                .iter()
                .map(|(tag, members)| {
                    (
                        (*tag).to_string(),
                        members.iter().map(|member| (*member).to_string()).collect(),
                    )
                })
                .collect::<BTreeMap<_, _>>(),
        }
    }

    #[test]
    fn dynamic_holders_use_published_entry_positions() {
        let registries = [published(
            "minecraft:worldgen/biome",
            &["minecraft:plains", "minecraft:desert", "minecraft:forest"],
        )];
        let tags = TagLoadReport {
            registries: vec![loaded(
                "minecraft:worldgen/biome",
                &[(
                    "minecraft:is_overworld",
                    &["minecraft:forest", "minecraft:plains"],
                )],
            )],
            warnings: Vec::new(),
        };

        let packed = pack_network_tags(
            &registries,
            &generation(),
            &tags,
            ContentRegistry::vanilla(),
        )
        .unwrap();

        assert_eq!(packed.registries[0].tags[0].entries, [2, 0]);
    }

    #[test]
    fn empty_defined_tags_are_published_but_absent_registry_maps_are_not() {
        let registries = [
            published("minecraft:all_empty", &["minecraft:holder"]),
            published("minecraft:mixed", &["minecraft:holder"]),
        ];
        let tags = TagLoadReport {
            registries: vec![
                loaded("minecraft:all_empty", &[("minecraft:empty", &[])]),
                loaded(
                    "minecraft:mixed",
                    &[
                        ("minecraft:empty", &[]),
                        ("minecraft:present", &["minecraft:holder"]),
                    ],
                ),
            ],
            warnings: Vec::new(),
        };

        let packed = pack_network_tags(
            &registries,
            &generation(),
            &tags,
            ContentRegistry::vanilla(),
        )
        .unwrap();

        assert_eq!(packed.registries.len(), 2);
        assert_eq!(packed.registries[0].registry, "minecraft:all_empty");
        assert_eq!(packed.registries[0].tags[0].tag, "minecraft:empty");
        assert!(packed.registries[0].tags[0].entries.is_empty());
        assert_eq!(packed.registries[1].registry, "minecraft:mixed");
        assert_eq!(packed.registries[1].tags.len(), 2);
        assert_eq!(packed.registries[1].tags[0].tag, "minecraft:empty");
        assert!(packed.registries[1].tags[0].entries.is_empty());
        assert_eq!(packed.registries[1].tags[1].tag, "minecraft:present");
    }
}
