use super::*;
use crate::FrozenRegistryMembership;
use bbb_protocol::static_registry_elements::{
    vanilla_static_registry_elements, VANILLA_STATIC_REGISTRY_ELEMENTS,
};
use bbb_registry_codec::registry_codec::RegistryReferenceMembership;

fn tags(registry: &str, tag: &str, entries: Vec<i32>) -> UpdateTags {
    UpdateTags {
        registries: vec![RegistryTags {
            registry: registry.into(),
            tags: vec![TagNetworkPayload {
                tag: tag.into(),
                entries,
            }],
        }],
    }
}

#[test]
fn all_static_registries_resolve_tag_raw_ids_in_catalog_order_and_preserve_duplicates() {
    assert_eq!(VANILLA_STATIC_REGISTRY_ELEMENTS.len(), 95);
    assert_eq!(
        VANILLA_STATIC_REGISTRY_ELEMENTS
            .iter()
            .map(|(_, elements)| elements.len())
            .sum::<usize>(),
        6812
    );
    let mut store = WorldStore::new();
    store.apply_update_tags(UpdateTags {
        registries: VANILLA_STATIC_REGISTRY_ELEMENTS
            .iter()
            .map(|&(registry, elements)| {
                assert!(!elements.is_empty(), "{registry}");
                let middle = (elements.len() / 2) as i32;
                RegistryTags {
                    registry: registry.into(),
                    tags: vec![
                        TagNetworkPayload {
                            tag: "test:ordered".into(),
                            entries: vec![0, middle, elements.len() as i32 - 1, middle],
                        },
                        TagNetworkPayload {
                            tag: "test:negative".into(),
                            entries: vec![-1],
                        },
                        TagNetworkPayload {
                            tag: "test:overflow".into(),
                            entries: vec![elements.len() as i32],
                        },
                    ],
                }
            })
            .collect(),
    });
    let access = store.frozen_registry_access();
    for &(registry, elements) in VANILLA_STATIC_REGISTRY_ELEMENTS {
        let middle = elements.len() / 2;
        let expected = vec![
            elements[0],
            elements[middle],
            elements[elements.len() - 1],
            elements[middle],
        ];
        assert_eq!(
            store
                .registry_resource_ids_for_tag(registry, "test:ordered")
                .unwrap(),
            Some(expected.clone()),
            "{registry}"
        );
        assert_eq!(
            access.resource_ids_for_tag(registry, "test:ordered"),
            Some(expected),
            "{registry}"
        );
        for index in [0, middle, elements.len() - 1] {
            assert_eq!(
                store.registry_resource_id_by_holder_id(registry, index as i32),
                Some(elements[index]),
                "{registry}:{index}"
            );
            assert_eq!(
                store.registry_holder_id_by_resource_id(registry, elements[index]),
                Some(index as i32),
                "{registry}:{}",
                elements[index]
            );
            assert_eq!(
                access.resource_membership(registry, elements[index]),
                FrozenRegistryMembership::Present
            );
            assert_eq!(
                access.element_membership(registry, elements[index]),
                RegistryReferenceMembership::Present
            );
        }
        for (name, bad) in [
            ("test:negative", -1),
            ("test:overflow", elements.len() as i32),
        ] {
            assert_eq!(store.registry_resource_id_by_holder_id(registry, bad), None);
            assert_eq!(
                store.registry_unresolved_holder_ids_for_tag(registry, name),
                Some([bad].as_slice())
            );
            assert_eq!(access.resource_ids_for_tag(registry, name), Some(vec![]));
            assert_eq!(
                access.tag_membership(registry, name),
                FrozenRegistryMembership::Present
            );
        }
        assert_eq!(
            access.resource_membership(registry, "example:forged"),
            FrozenRegistryMembership::Absent
        );
        assert_eq!(
            access.element_membership(registry, "example:forged"),
            RegistryReferenceMembership::Absent
        );
        assert_eq!(
            access.tag_membership(registry, "test:missing"),
            FrozenRegistryMembership::Absent
        );
    }
}

#[test]
fn new_static_families_are_available_and_unknown_registry_remains_unavailable() {
    let store = WorldStore::new();
    let access = store.frozen_registry_access();
    for registry in [
        "minecraft:memory_module_type",
        "minecraft:block_type",
        "minecraft:test_function",
    ] {
        let elements = vanilla_static_registry_elements(registry).unwrap();
        assert!(access.registry_available(registry));
        assert!(access.contains_element(registry, elements[0]));
        assert!(!access.contains_element(registry, "example:forged"));
        assert_eq!(
            access.element_membership(registry, "example:forged"),
            RegistryReferenceMembership::Absent
        );
        assert_eq!(
            access.tag_membership(registry, "test:missing"),
            FrozenRegistryMembership::Unavailable
        );
    }
    assert!(vanilla_static_registry_elements("example:unpublished").is_none());
    assert!(!access.registry_available("example:unpublished"));
    assert_eq!(
        access.resource_membership("example:unpublished", "minecraft:stone"),
        FrozenRegistryMembership::Unavailable
    );
    assert_eq!(
        access.element_membership("example:unpublished", "minecraft:stone"),
        RegistryReferenceMembership::Unavailable
    );
}

#[test]
fn published_contents_and_empty_dynamic_registry_prevent_catalog_reuse_and_fixed_injection() {
    let registry = "minecraft:memory_module_type";
    let builtin = vanilla_static_registry_elements(registry).unwrap()[0];
    let mut store = WorldStore::new();
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:damage_type", 0),
        Some("minecraft:arrow")
    );
    store.record_registry_entries(
        registry,
        0,
        vec![RegistryPacketEntry::stub("example:injected")],
    );
    store.apply_update_tags(tags(registry, "test:published", vec![0, 1]));
    assert_eq!(
        store.registry_resource_id_by_holder_id(registry, 0),
        Some("example:injected")
    );
    assert_eq!(store.registry_resource_id_by_holder_id(registry, 1), None);
    assert_eq!(
        store.registry_holder_id_by_resource_id(registry, builtin),
        None
    );
    assert_eq!(
        store.registry_unresolved_holder_ids_for_tag(registry, "test:published"),
        Some([1].as_slice())
    );
    let access = store.frozen_registry_access();
    assert_eq!(
        access.resource_ids_for_tag(registry, "test:published"),
        Some(vec!["example:injected"])
    );
    assert!(access.contains_element(registry, builtin));
    assert!(!access.contains_element(registry, "example:injected"));
    assert_eq!(
        access.element_membership(registry, "example:injected"),
        RegistryReferenceMembership::Absent
    );
    store.record_registry_entries("minecraft:damage_type", 0, Vec::new());
    store.apply_update_tags(tags(
        "minecraft:damage_type",
        "test:empty_publication",
        vec![0],
    ));
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:damage_type", 0),
        None
    );
    assert_eq!(
        store.registry_holder_id_by_resource_id("minecraft:damage_type", "minecraft:arrow"),
        None
    );
    assert_eq!(
        store
            .frozen_registry_access()
            .resource_membership("minecraft:damage_type", "minecraft:arrow"),
        FrozenRegistryMembership::Absent
    );
    assert_eq!(
        store
            .frozen_registry_access()
            .resource_ids_for_tag("minecraft:damage_type", "test:empty_publication"),
        Some(vec![])
    );
}

#[test]
fn static_tags_commit_atomically_and_failed_dynamic_configuration_keeps_active_catalog_view() {
    let registry = "minecraft:memory_module_type";
    let elements = vanilla_static_registry_elements(registry).unwrap();
    let last = elements.len() as i32 - 1;
    let mut store = WorldStore::new();
    store.apply_update_tags(tags(registry, "test:active", vec![0]));
    let first_revision = store.registry_access_revision();
    store.begin_registry_configuration();
    store.apply_update_tags(tags(registry, "test:committed", vec![last, 0, last]));
    assert_eq!(
        store
            .frozen_registry_access()
            .resource_ids_for_tag(registry, "test:active"),
        Some(vec![elements[0]])
    );
    assert_eq!(
        store
            .frozen_registry_access()
            .resource_ids_for_tag(registry, "test:committed"),
        None
    );
    assert_eq!(store.registry_access_revision(), first_revision);
    store.finish_registry_configuration().unwrap();
    let expected = vec![
        elements[last as usize],
        elements[0],
        elements[last as usize],
    ];
    assert_eq!(
        store
            .frozen_registry_access()
            .resource_ids_for_tag(registry, "test:committed"),
        Some(expected.clone())
    );
    assert_eq!(
        store
            .frozen_registry_access()
            .tag_membership(registry, "test:active"),
        FrozenRegistryMembership::Absent
    );
    let committed_revision = store.registry_access_revision();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, Some("minecraft:wolf_variant"));
    store.record_registry_data(RegistryData {
        registry: "minecraft:wolf_variant".into(),
        entries: vec![],
        raw_payload_len: 0,
    });
    store.apply_update_tags(tags(registry, "test:rejected", vec![0]));
    let error = store.finish_registry_configuration().unwrap_err();
    assert_eq!(error.registry, "minecraft:wolf_variant");
    assert_eq!(error.validator, "RegistryValidator.nonEmpty ()");
    assert_eq!(store.registry_access_revision(), committed_revision);
    assert_eq!(
        store
            .frozen_registry_access()
            .resource_ids_for_tag(registry, "test:committed"),
        Some(expected)
    );
    assert_eq!(
        store
            .frozen_registry_access()
            .resource_ids_for_tag(registry, "test:rejected"),
        None
    );
    assert_eq!(
        store.registry_resource_id_by_holder_id(registry, last),
        Some(elements[last as usize])
    );
}
