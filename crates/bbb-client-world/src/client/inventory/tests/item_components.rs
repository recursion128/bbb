use super::*;

#[test]
fn local_item_use_reads_inventory_menu_offhand_slot_when_player_slot_is_absent() {
    let mut store = WorldStore::new();

    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 1,
        items: (0..46)
            .map(|slot| {
                if slot == INVENTORY_MENU_OFFHAND_SLOT {
                    item_stack(99, 1)
                } else {
                    ProtocolItemStackSummary::empty()
                }
            })
            .collect(),
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert!(store.local_item_use_prefers_offhand());

    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 2,
        slot: INVENTORY_MENU_OFFHAND_SLOT,
        item: ProtocolItemStackSummary::empty(),
    });
    assert!(!store.local_item_use_prefers_offhand());
}

#[test]
fn local_selected_main_hand_has_piercing_weapon_true_for_default_item() {
    let mut store = WorldStore::new();
    store.set_default_item_component_codec_values(BTreeMap::from([
        (
            -1,
            BTreeMap::from([(
                VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
                serde_json::json!({}),
            )]),
        ),
        (
            42,
            BTreeMap::from([(
                VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
                serde_json::json!({}),
            )]),
        ),
    ]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });

    assert!(store.local_selected_main_hand_has_piercing_weapon());
    store.set_default_item_component_codec_values(BTreeMap::new());
    assert!(!store.local_selected_main_hand_has_piercing_weapon());
}

#[test]
fn local_selected_main_hand_has_piercing_weapon_true_for_added_component() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack_with_component_summary(
            99,
            1,
            VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
        ),
    });

    assert!(store.local_selected_main_hand_has_piercing_weapon());
}

#[test]
fn local_selected_main_hand_has_piercing_weapon_removed_component_overrides_default_and_added() {
    let mut store = WorldStore::new();
    store.set_default_item_component_codec_values(BTreeMap::from([(
        42,
        BTreeMap::from([(
            VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
            serde_json::json!({}),
        )]),
    )]));
    let mut item =
        item_stack_with_component_summary(42, 1, VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID);
    item.component_patch.removed_type_ids = vec![VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID];
    store.apply_set_player_inventory(ProtocolSetPlayerInventory { slot: 0, item });

    assert!(!store.local_selected_main_hand_has_piercing_weapon());
}

#[test]
fn local_selected_main_hand_has_piercing_weapon_false_for_empty_or_invalid_item() {
    let mut store = WorldStore::new();
    store.set_default_item_component_codec_values(BTreeMap::from([
        (
            -1,
            BTreeMap::from([(
                VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
                serde_json::json!({}),
            )]),
        ),
        (
            42,
            BTreeMap::from([(
                VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
                serde_json::json!({}),
            )]),
        ),
    ]));

    assert!(!store.local_selected_main_hand_has_piercing_weapon());

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 0),
    });
    assert!(!store.local_selected_main_hand_has_piercing_weapon());

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(-1, 1),
    });
    assert!(!store.local_selected_main_hand_has_piercing_weapon());
}

#[test]
fn local_selected_main_hand_has_piercing_weapon_respects_selected_slot() {
    let mut store = WorldStore::new();
    store.set_default_item_component_codec_values(BTreeMap::from([(
        42,
        BTreeMap::from([(
            VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
            serde_json::json!({}),
        )]),
    )]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: item_stack(99, 1),
    });

    assert!(store.local_selected_main_hand_has_piercing_weapon());
    assert!(store.set_local_selected_hotbar_slot(1));
    assert!(!store.local_selected_main_hand_has_piercing_weapon());
}

#[test]
fn local_active_item_attack_range_follows_use_hand_and_spectator_policy() {
    let mut store = WorldStore::new();
    let main_range = item_attack_range(0.0, 3.5);
    let offhand_range = item_attack_range(0.0, 4.5);
    store.items.commit_item_component_defaults(BTreeMap::from([
        (
            42,
            ProtocolDataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID],
                attack_range: Some(main_range),
                ..Default::default()
            },
        ),
        (
            99,
            ProtocolDataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID],
                attack_range: Some(offhand_range),
                ..Default::default()
            },
        ),
    ]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(99, 1),
    });

    assert_eq!(store.local_active_item_attack_range(), Some(main_range));
    store.set_local_using_item_with_hand(true, InteractionHand::OffHand);
    assert_eq!(store.local_active_item_attack_range(), Some(offhand_range));
    store.take_local_using_item();
    assert_eq!(store.local_active_item_attack_range(), Some(main_range));

    store.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: 3,
        param: 3.0,
    });
    assert_eq!(store.local_active_item_attack_range(), None);
}

#[test]
fn local_selected_main_hand_attack_range_respects_empty_invalid_and_selected_slot() {
    let mut store = WorldStore::new();
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID],
            attack_range: Some(item_attack_range(2.0, 4.5)),
            ..Default::default()
        },
    )]));

    assert_eq!(store.local_selected_main_hand_attack_range(), None);

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 0),
    });
    assert_eq!(store.local_selected_main_hand_attack_range(), None);

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(-1, 1),
    });
    assert_eq!(store.local_selected_main_hand_attack_range(), None);

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: item_stack(99, 1),
    });
    assert_eq!(
        store.local_selected_main_hand_attack_range(),
        Some(item_attack_range(2.0, 4.5))
    );
    assert!(store.set_local_selected_hotbar_slot(1));
    assert_eq!(store.local_selected_main_hand_attack_range(), None);
}

#[test]
fn local_attack_item_enabled_gate_accepts_empty_and_known_vanilla_stacks_only() {
    let mut store = WorldStore::new();
    assert!(store.local_selected_main_hand_is_enabled());

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    assert!(store.local_selected_main_hand_is_enabled());

    store.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec!["minecraft:trade_rebalance".to_string()],
    });
    assert!(!store.local_selected_main_hand_is_enabled());

    store.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec!["minecraft:vanilla".to_string()],
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(99_999, 1),
    });
    assert!(!store.local_selected_main_hand_is_enabled());
}

#[test]
fn local_using_item_use_effects_removed_component_falls_back_to_vanilla_default() {
    let mut store = WorldStore::new();
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID],
            use_effects: Some(ProtocolUseEffectsSummary {
                can_sprint: true,
                interact_vibrations: false,
                speed_multiplier: 1.0,
            }),
            ..Default::default()
        },
    )]));
    let mut item = item_stack(42, 1);
    item.component_patch.removed_type_ids = vec![VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID];
    store.apply_set_player_inventory(ProtocolSetPlayerInventory { slot: 0, item });
    store.set_local_using_item(true);

    assert_eq!(
        store.local_using_item_use_effects(),
        Some(ItemUseEffects::default())
    );
}

#[test]
fn local_using_item_item_id_reads_selected_hand() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 40,
        item: item_stack(99, 1),
    });

    assert_eq!(store.local_using_item_item_id(), None);

    store.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    assert_eq!(store.local_using_item_item_id(), Some(42));

    store.set_local_using_item_with_hand(true, InteractionHand::OffHand);
    assert_eq!(store.local_using_item_item_id(), Some(99));

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 40,
        item: ProtocolItemStackSummary::empty(),
    });
    assert_eq!(store.local_using_item_item_id(), None);
}

#[test]
fn local_bundle_selection_is_cleared_when_player_slot_item_is_replaced() {
    let mut store = WorldStore::new();

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 4,
        item: bundle_item_stack(42, 1, 4),
    });
    assert!(store.apply_local_select_bundle_item(4, 2));

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 4,
        item: bundle_item_stack(43, 1, 3),
    });
    assert_eq!(
        player_slot_selection(&store, 4),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );

    assert!(store.apply_local_select_bundle_item(4, 1));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 4,
        item: bundle_item_stack(44, 1, 2),
    });
    assert_eq!(
        player_slot_selection(&store, 4),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );

    assert!(store.apply_local_select_bundle_item(4, 1));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 4,
        item: item_stack(45, 1),
    });
    assert_eq!(
        player_slot_selection(&store, 4),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );
    assert!(!store.apply_local_select_bundle_item(4, 0));
}

#[test]
fn local_bundle_selection_fields_default_when_deserializing_old_slots() {
    let player_slot: InventorySlot = serde_json::from_value(serde_json::json!({
        "slot": 4,
        "item": item_stack(42, 1),
    }))
    .unwrap();
    assert_eq!(
        player_slot.local_selected_bundle_item_index,
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );

    let container_slot: ContainerSlot = serde_json::from_value(serde_json::json!({
        "slot": 4,
        "item": item_stack(42, 1),
    }))
    .unwrap();
    assert_eq!(
        container_slot.local_selected_bundle_item_index,
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );
}

#[test]
fn camera_overlay_textures_follow_equipped_items_that_declare_their_own_slot() {
    let mut store = WorldStore::new();
    assert!(store.local_player_camera_overlay_textures().is_empty());

    let mut pumpkin = item_stack(bbb_protocol::item_types::VANILLA_ITEM_CARVED_PUMPKIN_ID, 1);
    pumpkin.component_patch.equippable = Some(equippable_with_camera_overlay(
        bbb_protocol::equipment_slots::vanilla_equipment_slot_for_serialized_name("head")
            .unwrap()
            .id,
        Some("minecraft:misc/pumpkinblur"),
    ));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: bbb_protocol::equipment_slots::VANILLA_PLAYER_INVENTORY_HEAD_SLOT,
        item: pumpkin.clone(),
    });
    assert_eq!(
        store.local_player_camera_overlay_textures(),
        vec!["minecraft:textures/misc/pumpkinblur.png".to_string()]
    );

    // `Equippable.slot()` must match the slot the item actually occupies.
    let mut wrong_slot = pumpkin.clone();
    wrong_slot.component_patch.equippable = Some(equippable_with_camera_overlay(
        bbb_protocol::equipment_slots::vanilla_equipment_slot_for_serialized_name("feet")
            .unwrap()
            .id,
        Some("minecraft:misc/pumpkinblur"),
    ));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: bbb_protocol::equipment_slots::VANILLA_PLAYER_INVENTORY_HEAD_SLOT,
        item: wrong_slot,
    });
    assert!(store.local_player_camera_overlay_textures().is_empty());

    let mut without_overlay = pumpkin;
    without_overlay.component_patch.equippable = Some(equippable_with_camera_overlay(
        bbb_protocol::equipment_slots::vanilla_equipment_slot_for_serialized_name("head")
            .unwrap()
            .id,
        None,
    ));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: bbb_protocol::equipment_slots::VANILLA_PLAYER_INVENTORY_HEAD_SLOT,
        item: without_overlay,
    });
    assert!(store.local_player_camera_overlay_textures().is_empty());
}
