use super::*;

#[test]
fn mount_equipment_visibility_follows_vanilla_horse_type_tags() {
    assert_eq!(
        mount_visibility_for_entity(VANILLA_ENTITY_TYPE_HORSE_ID, Vec::new()),
        Some(MountEquipmentSlotVisibility {
            saddle: true,
            body: Some(MountArmorSlotKind::Horse),
        })
    );
    assert_eq!(
        mount_visibility_for_entity(VANILLA_ENTITY_TYPE_LLAMA_ID, Vec::new()),
        Some(MountEquipmentSlotVisibility {
            saddle: false,
            body: Some(MountArmorSlotKind::Llama),
        })
    );
    assert_eq!(
        mount_visibility_for_entity(VANILLA_ENTITY_TYPE_DONKEY_ID, Vec::new()),
        Some(MountEquipmentSlotVisibility {
            saddle: false,
            body: None,
        })
    );
}

#[test]
fn apply_local_mount_quick_move_routes_non_equipment_player_item_to_mount_inventory() {
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([(
        90,
        crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
    )]);
    store.apply_add_entity(protocol_add_entity_with_type(
        42,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_mount_screen_open(ProtocolMountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 53];
    items[17] = item_stack(43, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 17,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (MOUNT_INVENTORY_START, hashed_item_stack(43, 3)),
            (17, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, MOUNT_INVENTORY_START),
        item_stack(43, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, 17),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_mount_quick_move_routes_saddle_to_equipment_slot() {
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([(
        90,
        crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
    )]);
    store.apply_add_entity(protocol_add_entity_with_type(
        42,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_mount_screen_open(ProtocolMountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 53];
    items[17] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 17,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(90, 1)),
            (17, ProtocolHashedStack::Empty)
        ])
    );
    assert_eq!(open_container_slot_item(&store, 0), item_stack(90, 1));
    assert_eq!(
        open_container_slot_item(&store, 17),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_mount_quick_move_routes_matching_body_armor_to_equipment_slot() {
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([
        (
            91,
            bbb_protocol::packets::EquippableSummary {
                allowed_entities: Some(bbb_protocol::packets::HolderSetSummary {
                    registry_ids: vec![VANILLA_ENTITY_TYPE_HORSE_ID],
                    tag: None,
                }),
                ..crate::item_profiles::test_equippable(EquipmentSlot::Body)
            },
        ),
        (
            92,
            bbb_protocol::packets::EquippableSummary {
                allowed_entities: Some(bbb_protocol::packets::HolderSetSummary {
                    registry_ids: vec![bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LLAMA_ID],
                    tag: None,
                }),
                ..crate::item_profiles::test_equippable(EquipmentSlot::Body)
            },
        ),
    ]);
    store.apply_add_entity(protocol_add_entity_with_type(
        42,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_mount_screen_open(ProtocolMountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 53];
    items[17] = item_stack(91, 1);
    items[18] = item_stack(92, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let horse_armor = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 17,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        horse_armor.changed_slots,
        BTreeMap::from([
            (1, hashed_item_stack(91, 1)),
            (17, ProtocolHashedStack::Empty)
        ])
    );

    let llama_armor = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 18,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        llama_armor.changed_slots,
        BTreeMap::from([
            (MOUNT_INVENTORY_START, hashed_item_stack(92, 1)),
            (18, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(open_container_slot_item(&store, 1), item_stack(91, 1));
    assert_eq!(
        open_container_slot_item(&store, MOUNT_INVENTORY_START),
        item_stack(92, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, 17),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, 18),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_mount_quick_move_keeps_component_patched_player_item_server_authoritative() {
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([(
        90,
        crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
    )]);
    store.apply_add_entity(protocol_add_entity_with_type(
        42,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_mount_screen_open(ProtocolMountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });
    let mut patched = item_stack(90, 1);
    patched.component_patch.added_type_ids = vec![1];
    let mut items = vec![ProtocolItemStackSummary::empty(); 53];
    items[17] = patched.clone();
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let err = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 17,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap_err();

    assert_eq!(
        err,
        ContainerClickBuildError::UnsupportedLocalClickInput(ProtocolContainerInput::QuickMove)
    );
    assert_eq!(open_container_slot_item(&store, 17), patched);
}

#[test]
fn apply_local_mount_quick_move_keeps_missing_allowed_entity_tag_server_authoritative() {
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([(
        93,
        bbb_protocol::packets::EquippableSummary {
            allowed_entities: Some(bbb_protocol::packets::HolderSetSummary {
                registry_ids: vec![],
                tag: Some("example:missing".to_string()),
            }),
            ..crate::item_profiles::test_equippable(EquipmentSlot::Body)
        },
    )]);
    store.apply_add_entity(protocol_add_entity_with_type(
        42,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_mount_screen_open(ProtocolMountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 53];
    items[17] = item_stack(93, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 17,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let err = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 17,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap_err();

    assert_eq!(
        err,
        ContainerClickBuildError::UnsupportedLocalClickInput(ProtocolContainerInput::QuickMove)
    );
    assert_eq!(open_container_slot_item(&store, 17), item_stack(93, 1));
}

#[test]
fn apply_local_mount_pickup_honors_equipment_and_storage_slot_rules() {
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([(
        90,
        crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
    )]);
    store.set_test_equippable_defaults([
        (
            91,
            bbb_protocol::packets::EquippableSummary {
                allowed_entities: Some(bbb_protocol::packets::HolderSetSummary {
                    registry_ids: vec![VANILLA_ENTITY_TYPE_HORSE_ID],
                    tag: None,
                }),
                ..crate::item_profiles::test_equippable(EquipmentSlot::Body)
            },
        ),
        (
            92,
            bbb_protocol::packets::EquippableSummary {
                allowed_entities: Some(bbb_protocol::packets::HolderSetSummary {
                    registry_ids: vec![bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LLAMA_ID],
                    tag: None,
                }),
                ..crate::item_profiles::test_equippable(EquipmentSlot::Body)
            },
        ),
    ]);
    store.apply_add_entity(protocol_add_entity_with_type(
        42,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_mount_screen_open(ProtocolMountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 18,
        items: vec![ProtocolItemStackSummary::empty(); 53],
        carried_item: item_stack(90, 3),
    });

    let saddle = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MOUNT_SADDLE_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        saddle.changed_slots,
        BTreeMap::from([(MOUNT_SADDLE_SLOT, hashed_item_stack(90, 1))])
    );
    assert_eq!(saddle.carried_item, hashed_item_stack(90, 2));

    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(92, 1),
    });
    let mismatched_body = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MOUNT_BODY_ARMOR_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert!(mismatched_body.changed_slots.is_empty());
    assert_eq!(mismatched_body.carried_item, hashed_item_stack(92, 1));

    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(91, 1),
    });
    let matching_body = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MOUNT_BODY_ARMOR_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        matching_body.changed_slots,
        BTreeMap::from([(MOUNT_BODY_ARMOR_SLOT, hashed_item_stack(91, 1))])
    );

    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(43, 3),
    });
    let storage = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MOUNT_INVENTORY_START,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        storage.changed_slots,
        BTreeMap::from([(MOUNT_INVENTORY_START, hashed_item_stack(43, 3))])
    );
    assert_eq!(storage.carried_item, ProtocolHashedStack::Empty);
}

#[test]
fn apply_local_mount_equipment_pickup_falls_back_for_enchanted_stack() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        42,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_mount_screen_open(ProtocolMountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });
    let mut enchanted = item_stack(90, 1);
    enchanted.component_patch.added_type_ids =
        vec![bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID];
    enchanted.component_patch.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
        holder_id: 0,
        level: 1,
    }];
    let mut items = vec![ProtocolItemStackSummary::empty(); 53];
    items[MOUNT_SADDLE_SLOT as usize] = enchanted.clone();
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 19,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MOUNT_SADDLE_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    assert_eq!(
        open_container_slot_item(&store, MOUNT_SADDLE_SLOT),
        enchanted
    );
}

#[test]
fn apply_local_mount_equipment_pickup_allows_non_binding_enchantment_and_blocks_binding_curse() {
    let mut store = WorldStore::new();
    store.record_registry_data(bbb_protocol::packets::RegistryData {
        registry: "minecraft:enchantment".to_string(),
        entries: vec![
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:protection".to_string(),
                raw_data: None,
            },
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:binding_curse".to_string(),
                raw_data: None,
            },
        ],
        raw_payload_len: 0,
    });
    store.apply_add_entity(protocol_add_entity_with_type(
        42,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_mount_screen_open(ProtocolMountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });
    let mut protection = item_stack(90, 1);
    protection.component_patch.added_type_ids =
        vec![bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID];
    protection.component_patch.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
        holder_id: 0,
        level: 1,
    }];
    let mut binding = item_stack(91, 1);
    binding.component_patch.added_type_ids =
        vec![bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID];
    binding.component_patch.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
        holder_id: 1,
        level: 1,
    }];
    let mut items = vec![ProtocolItemStackSummary::empty(); 53];
    items[MOUNT_SADDLE_SLOT as usize] = protection.clone();
    items[MOUNT_BODY_ARMOR_SLOT as usize] = binding.clone();
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 21,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    // Protection does not prevent armor change: pickup predicts locally.
    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MOUNT_SADDLE_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        pickup.changed_slots.get(&MOUNT_SADDLE_SLOT),
        Some(&ProtocolHashedStack::Empty)
    );
    assert_eq!(store.inventory().cursor_item, protection);

    // Binding Curse blocks pickup in Survival.
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: ProtocolItemStackSummary::empty(),
    });
    let blocked = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MOUNT_BODY_ARMOR_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert!(blocked.changed_slots.is_empty());
    assert_eq!(
        open_container_slot_item(&store, MOUNT_BODY_ARMOR_SLOT),
        binding
    );
}
