use super::*;

#[test]
fn pause_screen_custom_dialog_selects_single_tag_and_opens_frozen_target() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        DIALOG_REGISTRY,
        0,
        vec![crate::RegistryPacketEntry::with_known_pack_data_for_test(
            "example:custom",
            br#"{"type":"minecraft:notice","title":"Internal","external_title":"External"}"#
                .to_vec(),
        )],
    );
    apply_pause_screen_dialog_tag(&mut store, vec![0]);

    let custom = store.pause_screen_custom_dialog().expect("custom dialog");
    assert_eq!(custom.resource_id(), "example:custom");
    assert_eq!(custom.external_title().text, "External");

    let current = &custom.dialog;
    assert_eq!(current.resource_id.as_deref(), Some("example:custom"));
    assert!(matches!(
        current.data,
        Some(DialogDataState::Notice { ref common, .. })
            if common.title.text == "Internal"
    ));
    assert_eq!(store.counters().show_dialog_packets, 0);
}

#[test]
fn pause_screen_custom_dialog_uses_builtin_list_for_multiple_tagged_entries() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        DIALOG_REGISTRY,
        0,
        vec![
            pause_notice_registry_entry("example:first", "First"),
            pause_notice_registry_entry("example:second", "Second"),
            crate::RegistryPacketEntry::with_known_pack_data_for_test(
                CUSTOM_OPTIONS_DIALOG,
                br##"{"type":"minecraft:dialog_list","title":"Custom Options","external_title":"Server Options","dialogs":"#minecraft:pause_screen_additions"}"##
                    .to_vec(),
            ),
        ],
    );
    apply_pause_screen_dialog_tag(&mut store, vec![1, 0]);

    let custom = store.pause_screen_custom_dialog().expect("custom list");
    assert_eq!(custom.resource_id(), CUSTOM_OPTIONS_DIALOG);
    assert_eq!(custom.external_title().text, "Server Options");
    assert!(matches!(
        custom.dialog.data,
        Some(DialogDataState::DialogList {
            ref dialogs, ..
        }) if dialogs
            .entries
            .iter()
            .map(|entry| entry.resource_id.as_deref())
            .eq([Some("example:second"), Some("example:first")])
    ));
}

#[test]
fn pause_screen_custom_dialog_falls_back_only_for_nonempty_server_links() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        DIALOG_REGISTRY,
        0,
        vec![crate::RegistryPacketEntry::with_known_pack_data_for_test(
            SERVER_LINKS_DIALOG,
            br#"{"type":"minecraft:server_links","title":"Links","external_title":"Server Links"}"#
                .to_vec(),
        )],
    );
    apply_pause_screen_dialog_tag(&mut store, Vec::new());
    assert_eq!(store.pause_screen_custom_dialog(), None);

    store.apply_server_links(ServerLinks {
        links: vec![ServerLinkEntry {
            link_type: ServerLinkType::Known(ServerLinkKnownType::Website),
            url: "https://example.invalid/".to_string(),
        }],
    });
    let custom = store.pause_screen_custom_dialog().expect("server links");
    assert_eq!(custom.resource_id(), SERVER_LINKS_DIALOG);
    assert_eq!(custom.external_title().text, "Server Links");
}

#[test]
fn pause_screen_custom_dialog_fails_closed_for_unresolved_or_invalid_targets() {
    let mut invalid = WorldStore::new();
    invalid.record_registry_entries(
        DIALOG_REGISTRY,
        0,
        vec![crate::RegistryPacketEntry::stub("example:stub")],
    );
    apply_pause_screen_dialog_tag(&mut invalid, vec![0]);
    assert_eq!(invalid.pause_screen_custom_dialog(), None);

    let mut unresolved = WorldStore::new();
    unresolved.record_registry_entries(
        DIALOG_REGISTRY,
        0,
        vec![crate::RegistryPacketEntry::with_known_pack_data_for_test(
            SERVER_LINKS_DIALOG,
            br#"{"type":"minecraft:server_links","title":"Links"}"#.to_vec(),
        )],
    );
    unresolved.apply_server_links(ServerLinks {
        links: vec![ServerLinkEntry {
            link_type: ServerLinkType::Known(ServerLinkKnownType::Website),
            url: "https://example.invalid/".to_string(),
        }],
    });
    apply_pause_screen_dialog_tag(&mut unresolved, vec![99]);
    assert_eq!(unresolved.pause_screen_custom_dialog(), None);
}

#[test]
fn tracks_code_of_conduct_text() {
    let mut store = WorldStore::new();
    let text = "Keep the server friendly.";

    store.apply_code_of_conduct(text.to_string());

    assert_eq!(
        store.last_code_of_conduct(),
        Some(&CodeOfConductState {
            text: text.to_string(),
            text_hash: code_of_conduct_text_hash(text),
        })
    );
    assert_eq!(store.counters().code_of_conduct_packets, 1);
    assert_eq!(store.counters().last_code_of_conduct_len, text.len());
}

#[test]
fn code_of_conduct_hash_matches_java_string_hash_code() {
    assert_eq!(code_of_conduct_text_hash(""), 0);
    assert_eq!(code_of_conduct_text_hash("abc"), 96354);
    assert_eq!(code_of_conduct_text_hash("Aa"), 2112);
    assert_eq!(code_of_conduct_text_hash("BB"), 2112);
    assert_eq!(code_of_conduct_text_hash("😀"), 1_772_899);
}

#[test]
fn known_pack_dialog_item_block_state_keeps_json_property_order() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        DIALOG_REGISTRY,
        0,
        vec![crate::RegistryPacketEntry::with_known_pack_data_for_test(
            "example:raw_block_state",
            br#"{"type":"minecraft:notice","title":"Raw","body":{"type":"minecraft:item","item":{"id":"minecraft:stone","components":{"minecraft:block_state":{"b":"one","a":"two","?":"question","\ufffd":"replacement"}}}}}"#.to_vec(),
        )],
    );
    store.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 0 },
    });
    let dialog = requested_dialog(&store);
    assert_eq!(dialog.semantic_error, None);
    let Some(DialogDataState::Notice { common, .. }) = &dialog.data else {
        panic!("expected resolved notice dialog: {dialog:?}");
    };
    let crate::DialogBodyState::Item { item, .. } = &common.body_data[0] else {
        panic!("expected item body");
    };
    let stack = item
        .item_stack_summary_for_protocol_id(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:stone")
                .unwrap()
                .id,
        )
        .unwrap();
    let patch = &stack.component_patch;
    let names = patch
        .block_state_properties
        .iter()
        .map(|(name, _)| name.units().to_vec())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            vec![b'b' as u16],
            vec![b'a' as u16],
            vec![b'?' as u16],
            vec![0xfffd]
        ]
    );

    let bytes = bbb_protocol::encode_nbt_summary(dialog.dialog.as_ref().unwrap(), 1 << 20).unwrap();
    let mut network = WorldStore::new();
    network.record_registry_entries(
        DIALOG_REGISTRY,
        0,
        vec![crate::RegistryPacketEntry::with_raw_data(
            "example:raw_block_state",
            bytes,
        )],
    );
    network.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 0 },
    });
    let dialog = requested_dialog(&network);
    assert_eq!(dialog.semantic_error, None);
    let Some(DialogDataState::Notice { common, .. }) = &dialog.data else {
        panic!("expected notice")
    };
    let crate::DialogBodyState::Item { item, .. } = &common.body_data[0] else {
        panic!("expected item")
    };
    let stack = item
        .item_stack_summary_for_protocol_id(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:stone")
                .unwrap()
                .id,
        )
        .unwrap();
    let patch = &stack.component_patch;
    let names = patch
        .block_state_properties
        .iter()
        .map(|(name, _)| name.units().to_vec())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            vec![b'b' as u16],
            vec![b'a' as u16],
            vec![b'?' as u16],
            vec![0xfffd]
        ]
    );
    assert_eq!(
        patch
            .block_state_properties
            .get_str("a")
            .unwrap()
            .to_jdk_string(),
        "two"
    );
}
