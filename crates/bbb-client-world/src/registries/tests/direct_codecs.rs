use super::*;

#[test]
fn persistent_holder_decoder_uses_resource_identity_and_typed_failures() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:test",
        0,
        vec![
            RegistryPacketEntry::stub("example:second"),
            RegistryPacketEntry::stub("example:first"),
        ],
    );
    let access = store.frozen_registry_access();

    let entry = access
        .persistent_holder_from_nbt("minecraft:test", &NbtSummaryValue::string("example:first"))
        .unwrap();
    assert_eq!((entry.holder_id, entry.resource_id), (1, "example:first"));
    let game_event = access
        .persistent_holder_from_nbt(
            "minecraft:game_event",
            &NbtSummaryValue::string("minecraft:block_change"),
        )
        .unwrap();
    assert_eq!(
        (
            game_event.holder_id,
            game_event.resource_id,
            game_event.source
        ),
        (
            2,
            "minecraft:block_change",
            crate::RegistryEntrySource::VanillaFallback
        )
    );

    for (value, expected) in [
        (
            NbtSummaryValue::string("example:missing"),
            "missing minecraft:test resource example:missing",
        ),
        (
            NbtSummaryValue::string("not a resource"),
            "invalid minecraft:test persistent holder resource id not a resource",
        ),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::string("example:first")]),
            "minecraft:test persistent holder is not a resource id",
        ),
    ] {
        assert_eq!(
            access
                .persistent_holder_from_nbt("minecraft:test", &value)
                .unwrap_err()
                .to_string(),
            expected
        );
    }
}

#[test]
fn persistent_holder_set_decoder_uses_resource_order_tags_and_typed_failures() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:test",
        0,
        vec![
            RegistryPacketEntry::stub("example:second"),
            RegistryPacketEntry::stub("example:first"),
        ],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:test".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "example:ordered".to_string(),
                entries: vec![1, 0],
            }],
        }],
    });
    let access = store.frozen_registry_access();

    let direct = access
        .persistent_holder_set_from_nbt(
            "minecraft:test",
            &NbtSummaryValue::List(vec![
                NbtSummaryValue::string("example:first"),
                NbtSummaryValue::string("example:second"),
            ]),
        )
        .unwrap();
    assert_eq!(direct.kind, FrozenRegistryHolderSetKind::Direct);
    assert_eq!(
        direct
            .entries
            .iter()
            .map(|entry| (entry.holder_id, entry.resource_id))
            .collect::<Vec<_>>(),
        vec![(1, "example:first"), (0, "example:second")]
    );

    let named = access
        .persistent_holder_set_from_nbt(
            "minecraft:test",
            &NbtSummaryValue::string("#example:ordered"),
        )
        .unwrap();
    assert!(matches!(
        named.kind,
        FrozenRegistryHolderSetKind::Named {
            tag: "example:ordered",
            ..
        }
    ));
    assert_eq!(
        named
            .entries
            .iter()
            .map(|entry| entry.resource_id)
            .collect::<Vec<_>>(),
        vec!["example:first", "example:second"]
    );

    for (value, expected) in [
        (
            NbtSummaryValue::string("#example:missing"),
            "missing minecraft:test tag #example:missing",
        ),
        (
            NbtSummaryValue::string("example:missing"),
            "missing minecraft:test resource example:missing",
        ),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::Int(0)]),
            "minecraft:test persistent holder-set entry 0 is not a resource id",
        ),
        (
            NbtSummaryValue::Int(0),
            "minecraft:test persistent holder set is neither a tag, resource id, nor list",
        ),
    ] {
        assert_eq!(
            access
                .persistent_holder_set_from_nbt("minecraft:test", &value)
                .unwrap_err()
                .to_string(),
            expected
        );
    }
}

#[test]
fn source_dependent_json_reaches_its_registry_typed_codec() {
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:timeline",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:legacy_null",
            br#"{"unknown":{"nested":null}}"#.to_vec(),
        )],
    );

    let error = store.finish_registry_configuration().unwrap_err();
    assert_eq!(error.registry, "minecraft:timeline");
    assert_eq!(error.validator, "Timeline.NETWORK_CODEC");

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:cow_variant",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:source_aware",
            br#"{
                "asset_id":"example:adult",
                "baby_asset_id":"example:baby",
                "unknown":{"nested":null}
            }"#
            .to_vec(),
        )],
    );
    store.finish_registry_configuration().unwrap();
    assert_eq!(
        store
            .synchronized_farm_animal_variant_presentation("minecraft:cow_variant", 0)
            .unwrap()
            .adult_asset_id,
        "example:adult"
    );
}

#[test]
fn dialog_direct_codec_does_not_reapply_the_legacy_whatwg_url_validator() {
    let registry = "minecraft:dialog";
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        registry,
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:scoped_uri",
            serde_json::to_vec(&serde_json::json!({
                "type": "notice",
                "title": "Scoped URI",
                "action": {
                    "label": "Open",
                    "action": {
                        "type": "open_url",
                        "url": "http://[fe80::1%eth0]/"
                    }
                }
            }))
            .unwrap(),
        )],
    );

    store.finish_registry_configuration().unwrap();

    assert_eq!(
        store.registry_resource_id_by_holder_id(registry, 0),
        Some("example:scoped_uri")
    );
}

#[test]
fn dialog_direct_codec_accepts_same_generation_registry_cycles() {
    let registry = "minecraft:dialog";
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        registry,
        0,
        vec![
            RegistryPacketEntry::with_known_pack_data_for_test(
                "example:self",
                minimal_notice_dialog_json(dialog_reference_title("example:self")),
            ),
            RegistryPacketEntry::with_known_pack_data_for_test(
                "example:first",
                minimal_notice_dialog_json(dialog_reference_title("example:second")),
            ),
            RegistryPacketEntry::with_known_pack_data_for_test(
                "example:second",
                minimal_notice_dialog_json(dialog_reference_title("example:first")),
            ),
            RegistryPacketEntry::with_known_pack_data_for_test(
                "example:book",
                minimal_notice_dialog_json(written_book_dialog_reference_title("example:book")),
            ),
        ],
    );

    store.finish_registry_configuration().unwrap();

    assert_eq!(
        store.registry_resource_id_by_holder_id(registry, 3),
        Some("example:book")
    );
}

#[test]
fn dialog_direct_codec_validates_same_generation_named_tags_without_projection() {
    let registry = "minecraft:dialog";
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        registry,
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:menu",
            serde_json::to_vec(&serde_json::json!({
                "type": "minecraft:dialog_list",
                "title": "Menu",
                "dialogs": "#example:self_menu",
            }))
            .unwrap(),
        )],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: registry.to_string(),
            tags: vec![TagNetworkPayload {
                tag: "example:self_menu".to_string(),
                entries: vec![0],
            }],
        }],
    });

    store.finish_registry_configuration().unwrap();

    assert_eq!(
        store.registry_tags(registry).unwrap().tags["example:self_menu"],
        vec![0]
    );
}

#[test]
fn dialog_direct_codec_rejects_missing_reference() {
    let registry = "minecraft:dialog";
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        registry,
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:missing_reference",
            minimal_notice_dialog_json(dialog_reference_title("example:missing")),
        )],
    );

    let missing = store.finish_registry_configuration().unwrap_err();
    assert_eq!(missing.registry, registry);
    assert_eq!(missing.validator, "Dialog.DIRECT_CODEC");
    assert!(missing.message.contains("example:missing"), "{missing:?}");
}
