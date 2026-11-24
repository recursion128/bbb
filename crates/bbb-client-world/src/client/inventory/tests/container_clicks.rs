use super::*;

#[test]
fn container_player_section_updates_mirror_into_the_backpack() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    // A 9x3 chest menu has 27 container slots, then main 27..54 mapping to
    // backpack 9-35 and hotbar 54..63 mapping to backpack 0-8. Vanilla's
    // menus share the player inventory object, so a server refresh of the
    // trailing section is a backpack update.
    let mut items = vec![ProtocolItemStackSummary::empty(); 63];
    items[27] = item_stack(42, 3);
    items[62] = item_stack(43, 5);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 2,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let backpack = |store: &WorldStore, slot: i32| {
        store
            .inventory()
            .player_slots
            .iter()
            .find(|row| row.slot == slot)
            .map(|row| row.item.clone())
    };
    assert_eq!(backpack(&store, 9), Some(item_stack(42, 3)));
    assert_eq!(backpack(&store, 8), Some(item_stack(43, 5)));

    // A single-slot refresh in the player section mirrors the same way.
    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 3,
        slot: 27,
        item: ProtocolItemStackSummary::empty(),
    });
    assert_eq!(backpack(&store, 9), Some(ProtocolItemStackSummary::empty()));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(44, 2),
    });
    assert_eq!(open_container_slot_item(&store, 27), item_stack(44, 2));

    // Closing the chest and opening the survival inventory must show the
    // mirrored rows in the menu projection.
    store.apply_container_close(ProtocolContainerClose { container_id: 7 });
    assert!(store.open_local_inventory());
    let menu_hotbar_last = store.inventory().inventory_menu_slot_item(44).cloned();
    assert_eq!(menu_hotbar_last, Some(item_stack(43, 5)));
}

#[test]
fn inventory_menu_content_replaces_cached_hotbar_stacks() {
    let mut store = WorldStore::new();

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(10, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 8,
        item: item_stack(18, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(99, 1),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 0,
        state_id: 1,
        items: (0..45)
            .map(|slot| {
                if slot == 36 {
                    item_stack(20, 2)
                } else if slot == 44 {
                    item_stack(28, 2)
                } else {
                    ProtocolItemStackSummary::empty()
                }
            })
            .collect(),
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 0,
        state_id: 2,
        slot: 40,
        item: item_stack(24, 3),
    });

    let hotbar = store.inventory().hotbar_items();
    assert_eq!(hotbar[0], item_stack(20, 2));
    assert_eq!(player_slot_item(&store, 0), item_stack(20, 2));
    assert_eq!(hotbar[4], item_stack(24, 3));
    assert_eq!(hotbar[8], item_stack(28, 2));
    assert_eq!(hotbar[1], ProtocolItemStackSummary::empty());
}

#[test]
fn empty_inventory_menu_slot_updates_clear_player_stacks() {
    let mut inventory = InventoryState {
        player_slots: vec![
            InventorySlot {
                slot: 0,
                item: item_stack(10, 1),
                local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
            },
            InventorySlot {
                slot: 2,
                item: item_stack(12, 3),
                local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
            },
        ],
        ..Default::default()
    };
    inventory.set_menu_slot(
        INVENTORY_MENU_CONTAINER_ID,
        ContainerSlot {
            slot: INVENTORY_MENU_HOTBAR_START,
            item: ProtocolItemStackSummary::empty(),
            local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
        },
    );

    let hotbar = inventory.hotbar_item_states();
    assert_eq!(hotbar[0].unwrap().item, &ProtocolItemStackSummary::empty());
    assert!(hotbar[1].is_none());
    assert_eq!(hotbar[2].unwrap().item, &item_stack(12, 3));
    assert_eq!(
        inventory.hotbar_items()[0],
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn container_zero_content_updates_inventory_menu_without_opening_local_screen() {
    let mut store = WorldStore::new();

    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 7,
        items: vec![ProtocolItemStackSummary::empty(), item_stack(42, 3)],
        carried_item: item_stack(99, 1),
    });

    assert!(!store.local_inventory_is_open());
    assert_eq!(store.open_container_id(), None);
    assert!(store.inventory().open_container.is_none());
    assert_eq!(store.inventory().inventory_menu.container_id, 0);
    assert_eq!(store.inventory().inventory_menu.state_id, 7);
    assert_eq!(
        store.inventory().inventory_menu.slots[1].item,
        item_stack(42, 3)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(99, 1));
}

#[test]
fn open_local_inventory_builds_container_zero_view_from_player_inventory() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(10, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
        item: item_stack(VANILLA_ITEM_ELYTRA_ID, 1),
    });
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(99, 1),
    });

    assert!(store.open_local_inventory());
    assert!(!store.open_local_inventory());
    assert!(store.local_inventory_is_open());
    assert_eq!(store.open_container_id(), Some(INVENTORY_MENU_CONTAINER_ID));

    let inventory_menu = &store.inventory().inventory_menu;
    assert_eq!(inventory_menu.container_id, INVENTORY_MENU_CONTAINER_ID);
    assert_eq!(
        store.inventory().container_slots(inventory_menu).count(),
        46
    );
    assert_eq!(
        store
            .inventory()
            .container_slot(inventory_menu, INVENTORY_MENU_HOTBAR_START)
            .unwrap()
            .item,
        &item_stack(10, 1)
    );
    assert_eq!(
        store
            .inventory()
            .container_slot(inventory_menu, 6)
            .unwrap()
            .item,
        &item_stack(VANILLA_ITEM_ELYTRA_ID, 1)
    );

    let click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(click.container_id, INVENTORY_MENU_CONTAINER_ID);
    assert_eq!(click.slot_num, INVENTORY_MENU_HOTBAR_START);
    assert_eq!(
        click.carried_item,
        ProtocolHashedStack::Item(ProtocolHashedItemStack {
            item_id: 99,
            count: 1,
            components: ProtocolHashedComponentPatch::default(),
        })
    );
}

#[test]
fn local_item_use_prefers_offhand_only_when_selected_hotbar_slot_is_empty() {
    let mut store = WorldStore::new();

    assert!(!store.local_item_use_prefers_offhand());

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(99, 1),
    });
    assert!(store.local_item_use_prefers_offhand());

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(10, 1),
    });
    assert!(!store.local_item_use_prefers_offhand());

    assert!(store.set_local_selected_hotbar_slot(1));
    assert!(store.local_item_use_prefers_offhand());

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: ProtocolItemStackSummary::empty(),
    });
    assert!(!store.local_item_use_prefers_offhand());
}

#[test]
fn drop_local_selected_hotbar_item_drops_one_and_updates_menu_view() {
    let mut store = WorldStore::new();
    assert!(store.set_local_selected_hotbar_slot(2));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 2,
        item: item_stack(42, 3),
    });

    assert!(store.drop_local_selected_hotbar_item(false));

    assert_eq!(player_slot_item(&store, 2), item_stack(42, 2));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START + 2),
        item_stack(42, 2)
    );
}

#[test]
fn drop_local_selected_hotbar_item_drops_stack_and_reports_empty_slots() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(99, 1),
    });

    assert!(store.drop_local_selected_hotbar_item(true));

    assert_eq!(
        player_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert!(!store.drop_local_selected_hotbar_item(true));
}

#[test]
fn hotbar_item_states_include_local_bundle_selection() {
    let mut store = WorldStore::new();

    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 4,
        item: bundle_item_stack(10, 1, 2),
    });
    assert!(store.apply_local_select_bundle_item(4, 1));

    let hotbar = store.inventory().hotbar_item_states();
    assert_eq!(hotbar[4].unwrap().item, &bundle_item_stack(10, 1, 2));
    assert_eq!(
        hotbar[4].unwrap().local_selected_bundle_item_index(),
        Some(1)
    );

    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 0,
        state_id: 1,
        items: (0..45)
            .map(|slot| {
                if slot == 40 {
                    bundle_item_stack(20, 1, 2)
                } else {
                    ProtocolItemStackSummary::empty()
                }
            })
            .collect(),
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.open_local_inventory());
    assert!(store.apply_local_select_bundle_item(40, 0));

    let hotbar = store.inventory().hotbar_item_states();
    assert_eq!(hotbar[4].unwrap().item, &bundle_item_stack(20, 1, 2));
    assert_eq!(
        hotbar[4].unwrap().local_selected_bundle_item_index(),
        Some(0)
    );
    assert_eq!(
        store.inventory().hotbar_items()[4],
        bundle_item_stack(20, 1, 2)
    );
}

#[test]
fn build_container_click_slot_allows_vanilla_outside_slots() {
    let mut store = WorldStore::new();
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 0,
        state_id: 4,
        items: vec![item_stack(42, 1)],
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.open_local_inventory());

    let outside_click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: -999,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    let carried_click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: -1,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(outside_click.slot_num, -999);
    assert_eq!(carried_click.slot_num, -1);
}

#[test]
fn build_container_click_slot_rejects_missing_container_invalid_slot_and_unhashable_carried_item() {
    let mut store = WorldStore::new();
    assert_eq!(
        store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap_err(),
        ContainerClickBuildError::NoOpenContainer
    );

    open_test_container(&mut store, 7);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: vec![item_stack(42, 3)],
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert_eq!(
        store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: 5,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap_err(),
        ContainerClickBuildError::InvalidSlot(5)
    );

    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack_with_component_summary(99, 1, 10),
    });
    assert_eq!(
        store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap_err(),
        ContainerClickBuildError::UnhashableCarriedItem
    );
}

#[test]
fn apply_local_container_click_slot_hashes_removed_only_component_patch() {
    let mut store = WorldStore::new();
    open_test_container(&mut store, 7);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: vec![ProtocolItemStackSummary::empty()],
        carried_item: item_stack_with_removed_component_summary(
            42,
            3,
            VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        ),
    });

    let place = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        place.changed_slots,
        BTreeMap::from([(
            0,
            hashed_removed_component_item_stack(42, 3, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID)
        )])
    );
    assert_eq!(place.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, 0),
        item_stack_with_removed_component_summary(42, 3, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID)
    );
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_container_click_slot_hashes_integer_component_patches() {
    for (component_type_id, value) in [
        (VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID, 16),
        (VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, 250),
        (VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, 7),
        (VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID, 7),
    ] {
        let mut store = WorldStore::new();
        let patched_stack = item_stack_with_int_component_summary(42, 1, component_type_id, value);
        open_test_container(&mut store, 7);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items: vec![ProtocolItemStackSummary::empty()],
            carried_item: patched_stack.clone(),
        });

        let place = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();

        assert_eq!(
            place.changed_slots,
            BTreeMap::from([(
                0,
                hashed_int_component_item_stack(42, 1, component_type_id, value)
            )])
        );
        assert_eq!(place.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(open_container_slot_item(&store, 0), patched_stack);
    }
}

#[test]
fn container_click_hashes_potion_without_its_stored_hidden_effect() {
    let mut stack = item_stack(42, 1);
    stack.component_patch = ProtocolDataComponentPatchSummary {
        added_type_ids: vec![
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        ],
        potion_contents: bbb_protocol::packets::PotionContentsSummary {
            custom_effects: vec![bbb_protocol::packets::MobEffectInstanceSummary {
                effect_id: bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_HASTE_ID,
                amplifier: 1,
                duration: 200,
                ambient: false,
                show_particles: true,
                show_icon: true,
                hidden_effect: Some(Box::new(bbb_protocol::packets::MobEffectDetailsSummary {
                    amplifier: 0,
                    duration: 40,
                    ambient: false,
                    show_particles: true,
                    show_icon: false,
                    hidden_effect: None,
                })),
            }],
            ..Default::default()
        },
        ..Default::default()
    }
    .into();
    let mut store = WorldStore::new();
    open_test_container(&mut store, 7);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: vec![ProtocolItemStackSummary::empty()],
        carried_item: stack.clone(),
    });
    let click = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    let Some(ProtocolHashedStack::Item(item)) = click.changed_slots.get(&0) else {
        panic!("placed potion");
    };
    assert_eq!(
        item.components.added_components.get(
            &bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID
        ),
        Some(&(0xd688_8158_u32 as i32))
    );
    assert_eq!(open_container_slot_item(&store, 0), stack);
    assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
}

#[test]
fn apply_local_container_click_slot_hashes_multiple_integer_and_removed_component_patches() {
    let mut store = WorldStore::new();
    let mut patched_stack = item_stack(42, 1);
    patched_stack.component_patch.added_type_ids = vec![
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
    ];
    patched_stack.component_patch.max_stack_size = Some(16);
    patched_stack.component_patch.max_damage = Some(250);
    patched_stack.component_patch.damage = Some(7);
    patched_stack.component_patch.removed_type_ids = vec![VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID];
    open_test_container(&mut store, 7);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: vec![ProtocolItemStackSummary::empty()],
        carried_item: patched_stack.clone(),
    });

    let place = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        place.changed_slots,
        BTreeMap::from([(
            0,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 42,
                count: 1,
                components: ProtocolHashedComponentPatch {
                    added_components: BTreeMap::from([
                        (
                            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                            hash_ops_crc32c_int(16)
                        ),
                        (
                            VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                            hash_ops_crc32c_int(250)
                        ),
                        (
                            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                            hash_ops_crc32c_int(7)
                        ),
                    ]),
                    removed_components: BTreeSet::from([VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID]),
                },
            })
        )])
    );
    assert_eq!(place.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(open_container_slot_item(&store, 0), patched_stack);
}

#[test]
fn apply_local_container_click_slot_rejects_incomplete_integer_component_patch() {
    let mut store = WorldStore::new();
    let mut patched_stack = item_stack(42, 1);
    patched_stack
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    open_test_container(&mut store, 7);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: vec![ProtocolItemStackSummary::empty()],
        carried_item: patched_stack,
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnhashableChangedSlot(0))
    );
}

#[test]
fn apply_local_container_click_slot_hashes_profile_component_patch() {
    let mut store = WorldStore::new();
    open_test_container(&mut store, 7);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: vec![profiled_item_stack(42, 1)],
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([(0, ProtocolHashedStack::Empty)])
    );
    let ProtocolHashedStack::Item(carried) = pickup.carried_item else {
        panic!("profiled carried stack must remain an item");
    };
    assert_eq!(carried.item_id, 42);
    assert_eq!(carried.count, 1);
    assert_eq!(
        carried
            .components
            .added_components
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID]
    );
    assert!(carried.components.removed_components.is_empty());
}

#[test]
fn apply_local_container_click_slot_supports_secondary_pickup_place_and_outside_drop() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 5),
    });
    assert!(store.open_local_inventory());

    let half_pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 1,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        half_pickup.changed_slots,
        BTreeMap::from([(
            INVENTORY_MENU_HOTBAR_START,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 42,
                count: 2,
                components: ProtocolHashedComponentPatch::default(),
            })
        )])
    );
    assert_eq!(
        half_pickup.carried_item,
        ProtocolHashedStack::Item(ProtocolHashedItemStack {
            item_id: 42,
            count: 3,
            components: ProtocolHashedComponentPatch::default(),
        })
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(42, 2)
    );
    assert_eq!(player_slot_item(&store, 0), item_stack(42, 2));
    assert_eq!(store.inventory().cursor_item, item_stack(42, 3));

    let single_place = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 10,
            button_num: 1,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        single_place.changed_slots,
        BTreeMap::from([(
            10,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 42,
                count: 1,
                components: ProtocolHashedComponentPatch::default(),
            })
        )])
    );
    assert_eq!(
        single_place.carried_item,
        ProtocolHashedStack::Item(ProtocolHashedItemStack {
            item_id: 42,
            count: 2,
            components: ProtocolHashedComponentPatch::default(),
        })
    );
    assert_eq!(inventory_menu_slot_item(&store, 10), item_stack(42, 1));
    assert_eq!(player_slot_item(&store, 10), item_stack(42, 1));
    assert_eq!(store.inventory().cursor_item, item_stack(42, 2));

    let drop_one = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: -999,
            button_num: 1,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert!(drop_one.changed_slots.is_empty());
    assert_eq!(
        drop_one.carried_item,
        ProtocolHashedStack::Item(ProtocolHashedItemStack {
            item_id: 42,
            count: 1,
            components: ProtocolHashedComponentPatch::default(),
        })
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 1));

    let drop_remaining = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: -999,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert!(drop_remaining.changed_slots.is_empty());
    assert_eq!(drop_remaining.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_container_click_slot_uses_default_item_max_stack_sizes() {
    let mut store = WorldStore::new();
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
            max_stack_size: Some(16),
            ..Default::default()
        },
    )]));
    let mut damage_only = item_stack(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID, 1);
    damage_only.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID];
    damage_only.component_patch.damage = Some(3);
    assert_eq!(store.items.item_stack_max_stack_size(&damage_only), 64);
    damage_only.component_patch.removed_type_ids =
        vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID];
    assert_eq!(store.items.item_stack_max_stack_size(&damage_only), 1);
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 2),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: item_stack(42, 15),
    });
    assert!(store.open_local_inventory());

    store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    let merge = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        merge.changed_slots,
        BTreeMap::from([(
            INVENTORY_MENU_HOTBAR_START + 1,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 42,
                count: 16,
                components: ProtocolHashedComponentPatch::default(),
            })
        )])
    );
    assert_eq!(
        merge.carried_item,
        ProtocolHashedStack::Item(ProtocolHashedItemStack {
            item_id: 42,
            count: 1,
            components: ProtocolHashedComponentPatch::default(),
        })
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START + 1),
        item_stack(42, 16)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 1));
}

#[test]
fn apply_local_container_click_slot_respects_unstackable_and_local_slot_limits() {
    let mut store = WorldStore::new();
    store.items.commit_item_component_defaults(BTreeMap::from([
        (
            42,
            ProtocolDataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                max_stack_size: Some(1),
                ..Default::default()
            },
        ),
        (
            43,
            ProtocolDataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                max_stack_size: Some(64),
                ..Default::default()
            },
        ),
    ]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: item_stack(42, 1),
    });
    assert!(store.open_local_inventory());

    store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    let blocked_merge = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert!(blocked_merge.changed_slots.is_empty());
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START + 1),
        item_stack(42, 1)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 1));

    store.set_test_equippable_defaults([(
        43,
        crate::item_profiles::test_equippable(bbb_protocol::equipment_slots::EquipmentSlot::Head),
    )]);
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(43, 3),
    });
    let result_slot_place = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert!(result_slot_place.changed_slots.is_empty());
    assert_eq!(store.inventory().cursor_item, item_stack(43, 3));

    let armor_place = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 5,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        armor_place.changed_slots,
        BTreeMap::from([(
            5,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 43,
                count: 1,
                components: ProtocolHashedComponentPatch::default(),
            })
        )])
    );
    assert_eq!(inventory_menu_slot_item(&store, 5), item_stack(43, 1));
    assert_eq!(store.inventory().cursor_item, item_stack(43, 2));
}

#[test]
fn apply_local_container_pickup_all_collects_matching_stacks() {
    let mut store = WorldStore::new();
    store.items.commit_item_component_defaults(BTreeMap::from([
        (
            42,
            ProtocolDataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                max_stack_size: Some(16),
                ..Default::default()
            },
        ),
        (
            43,
            ProtocolDataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                max_stack_size: Some(16),
                ..Default::default()
            },
        ),
    ]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(42, 3),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 10,
        item: item_stack(42, 4),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 11,
        item: item_stack(42, 16),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 12,
        item: item_stack(43, 5),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 14,
        item: item_stack_with_component_summary(42, 6, 7),
    });
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 8),
    });
    assert!(store.open_local_inventory());

    let pickup_all = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 13,
            button_num: 0,
            input: ProtocolContainerInput::PickupAll,
        })
        .unwrap();

    assert_eq!(pickup_all.container_id, INVENTORY_MENU_CONTAINER_ID);
    assert_eq!(pickup_all.input, ProtocolContainerInput::PickupAll);
    assert_eq!(
        pickup_all.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, ProtocolHashedStack::Empty),
            (INVENTORY_MENU_MAIN_START + 1, ProtocolHashedStack::Empty),
            (INVENTORY_MENU_MAIN_START + 2, hashed_item_stack(42, 15)),
        ])
    );
    assert_eq!(pickup_all.carried_item, hashed_item_stack(42, 16));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 1),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 2),
        item_stack(42, 15)
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 3),
        item_stack(43, 5)
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 5),
        item_stack_with_component_summary(42, 6, 7)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 16));
}

#[test]
fn apply_local_container_pickup_all_button_one_collects_in_reverse_order() {
    let mut store = WorldStore::new();
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
            max_stack_size: Some(16),
            ..Default::default()
        },
    )]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(42, 4),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 8,
        item: item_stack(42, 4),
    });
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 14),
    });
    assert!(store.open_local_inventory());

    let pickup_all = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START + 1,
            button_num: 1,
            input: ProtocolContainerInput::PickupAll,
        })
        .unwrap();

    assert_eq!(
        pickup_all.changed_slots,
        BTreeMap::from([(INVENTORY_MENU_HOTBAR_END - 1, hashed_item_stack(42, 2))])
    );
    assert_eq!(pickup_all.carried_item, hashed_item_stack(42, 16));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        item_stack(42, 4)
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_END - 1),
        item_stack(42, 2)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 16));
}

#[test]
fn apply_local_container_pickup_all_noops_without_eligible_click() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(42, 3),
    });
    assert!(store.open_local_inventory());

    let empty_cursor = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::PickupAll,
        })
        .unwrap();
    assert!(empty_cursor.changed_slots.is_empty());
    assert_eq!(empty_cursor.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        item_stack(42, 3)
    );

    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 5),
    });
    let clicked_item = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::PickupAll,
        })
        .unwrap();
    assert!(clicked_item.changed_slots.is_empty());
    assert_eq!(clicked_item.carried_item, hashed_item_stack(42, 5));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        item_stack(42, 3)
    );

    for slot_num in [-999, -1] {
        let outside = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num,
                button_num: 0,
                input: ProtocolContainerInput::PickupAll,
            })
            .unwrap();
        assert!(outside.changed_slots.is_empty());
        assert_eq!(outside.carried_item, hashed_item_stack(42, 5));
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
            item_stack(42, 3)
        );
    }
}

#[test]
fn apply_local_container_quick_move_moves_hotbar_to_inventory() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 3),
    });
    assert!(store.open_local_inventory());

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, INVENTORY_MENU_CONTAINER_ID);
    assert_eq!(quick_move.input, ProtocolContainerInput::QuickMove);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 3)),
            (INVENTORY_MENU_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(inventory_menu_slot_item(&store, 9), item_stack(42, 3));
    assert_eq!(
        player_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(player_slot_item(&store, 9), item_stack(42, 3));
}

#[test]
fn apply_local_container_quick_move_moves_inventory_to_hotbar() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(43, 4),
    });
    assert!(store.open_local_inventory());

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START,
            button_num: 1,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, ProtocolHashedStack::Empty),
            (INVENTORY_MENU_HOTBAR_START, hashed_item_stack(43, 4)),
        ])
    );
    assert_eq!(
        inventory_menu_slot_item(&store, 9),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(43, 4)
    );
    assert_eq!(
        player_slot_item(&store, 9),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(player_slot_item(&store, 0), item_stack(43, 4));
}

#[test]
fn apply_local_container_quick_move_merges_then_fills_with_stack_limits() {
    let mut store = WorldStore::new();
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
            max_stack_size: Some(16),
            ..Default::default()
        },
    )]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 2),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(42, 15),
    });
    assert!(store.open_local_inventory());

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 16)),
            (INVENTORY_MENU_MAIN_START + 1, hashed_item_stack(42, 1)),
            (INVENTORY_MENU_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(inventory_menu_slot_item(&store, 9), item_stack(42, 16));
    assert_eq!(inventory_menu_slot_item(&store, 10), item_stack(42, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(player_slot_item(&store, 9), item_stack(42, 16));
    assert_eq!(player_slot_item(&store, 10), item_stack(42, 1));
    assert_eq!(
        player_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_container_quick_move_auto_equips_armor_slot() {
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([(
        42,
        crate::item_profiles::test_equippable(EquipmentSlot::Chest),
    )]);
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    assert!(store.open_local_inventory());

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (6, hashed_item_stack(42, 1)),
            (INVENTORY_MENU_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(inventory_menu_slot_item(&store, 6), item_stack(42, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_CHEST_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        player_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_container_quick_move_auto_equips_offhand_slot() {
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([(
        43,
        crate::item_profiles::test_equippable(EquipmentSlot::OffHand),
    )]);
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(43, 1),
    });
    assert!(store.open_local_inventory());

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, ProtocolHashedStack::Empty),
            (INVENTORY_MENU_OFFHAND_SLOT, hashed_item_stack(43, 1)),
        ])
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_OFFHAND_SLOT),
        item_stack(43, 1)
    );
    assert_eq!(
        player_slot_item(&store, 9),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT),
        item_stack(43, 1)
    );
}

#[test]
fn apply_local_container_quick_move_uses_inventory_fallback_when_equipment_slot_is_occupied() {
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([(
        42,
        crate::item_profiles::test_equippable(EquipmentSlot::Chest),
    )]);
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
        item: item_stack(99, 1),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(42, 1),
    });
    assert!(store.open_local_inventory());

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, ProtocolHashedStack::Empty),
            (INVENTORY_MENU_HOTBAR_START, hashed_item_stack(42, 1)),
        ])
    );
    assert_eq!(inventory_menu_slot_item(&store, 6), item_stack(99, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(42, 1)
    );
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_CHEST_SLOT),
        item_stack(99, 1)
    );
    assert_eq!(
        player_slot_item(&store, 9),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(player_slot_item(&store, 0), item_stack(42, 1));
}

#[test]
fn apply_local_container_result_adds_default_remainder_to_visible_player_inventory() {
    let mut store = WorldStore::new();
    assert!(store.set_local_selected_hotbar_slot(2));
    store.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.open_local_inventory());

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (1, hashed_item_stack(42, 1)),
            (36, hashed_item_stack(43, 1)),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 1));
    assert_eq!(inventory_menu_slot_item(&store, 36), item_stack(43, 1));
    assert_eq!(player_slot_item(&store, 0), item_stack(43, 1));
}

#[test]
fn apply_local_container_quick_move_rejects_non_inventory_menu() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: i32::try_from(VANILLA_MENU_TYPES.len()).unwrap(),
        title: bbb_protocol::RichTextComponent::plain("Unsupported"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![item_stack(42, 3); 9],
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap_err(),
        ContainerClickBuildError::UnsupportedLocalClickInput(ProtocolContainerInput::QuickMove)
    );
    assert_eq!(
        store.inventory().open_container.as_ref().unwrap().slots[0].item,
        item_stack(42, 3)
    );
}

#[test]
fn apply_local_container_throw_drops_one_from_inventory_slot() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 3),
    });
    assert!(store.open_local_inventory());

    let throw = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::Throw,
        })
        .unwrap();

    assert_eq!(throw.container_id, INVENTORY_MENU_CONTAINER_ID);
    assert_eq!(throw.input, ProtocolContainerInput::Throw);
    assert_eq!(
        throw.changed_slots,
        BTreeMap::from([(INVENTORY_MENU_HOTBAR_START, hashed_item_stack(42, 2))])
    );
    assert_eq!(throw.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(42, 2)
    );
    assert_eq!(player_slot_item(&store, 0), item_stack(42, 2));
}

#[test]
fn apply_local_container_throw_drops_full_stack_and_requires_empty_cursor() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 3),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: item_stack(43, 4),
    });
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(99, 1),
    });
    assert!(store.open_local_inventory());

    let blocked = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 1,
            input: ProtocolContainerInput::Throw,
        })
        .unwrap();
    assert!(blocked.changed_slots.is_empty());
    assert_eq!(blocked.carried_item, hashed_item_stack(99, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(42, 3)
    );

    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: ProtocolItemStackSummary::empty(),
    });
    let drop_stack = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START + 1,
            button_num: 1,
            input: ProtocolContainerInput::Throw,
        })
        .unwrap();

    assert_eq!(
        drop_stack.changed_slots,
        BTreeMap::from([(INVENTORY_MENU_HOTBAR_START + 1, ProtocolHashedStack::Empty)])
    );
    assert_eq!(drop_stack.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START + 1),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        player_slot_item(&store, 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_container_swap_exchanges_hovered_slot_with_hotbar_button() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 3),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(43, 2),
    });
    assert!(store.open_local_inventory());

    let swap = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();

    assert_eq!(swap.container_id, INVENTORY_MENU_CONTAINER_ID);
    assert_eq!(swap.input, ProtocolContainerInput::Swap);
    assert_eq!(
        swap.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 3)),
            (INVENTORY_MENU_HOTBAR_START, hashed_item_stack(43, 2)),
        ])
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        item_stack(42, 3)
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(43, 2)
    );
    assert_eq!(player_slot_item(&store, 9), item_stack(42, 3));
    assert_eq!(player_slot_item(&store, 0), item_stack(43, 2));
}

#[test]
fn apply_local_container_swap_moves_hovered_slot_to_empty_offhand() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(44, 5),
    });
    assert!(store.open_local_inventory());

    let swap = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START,
            button_num: 40,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();

    assert_eq!(
        swap.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, ProtocolHashedStack::Empty),
            (INVENTORY_MENU_OFFHAND_SLOT, hashed_item_stack(44, 5)),
        ])
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_OFFHAND_SLOT),
        item_stack(44, 5)
    );
    assert_eq!(
        player_slot_item(&store, 9),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT),
        item_stack(44, 5)
    );
}

#[test]
fn apply_local_container_swap_splits_source_when_target_slot_has_lower_limit() {
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([(
        45,
        crate::item_profiles::test_equippable(bbb_protocol::equipment_slots::EquipmentSlot::Head),
    )]);
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(45, 3),
    });
    assert!(store.open_local_inventory());

    let swap = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 5,
            button_num: 0,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();

    assert_eq!(
        swap.changed_slots,
        BTreeMap::from([
            (5, hashed_item_stack(45, 1)),
            (INVENTORY_MENU_HOTBAR_START, hashed_item_stack(45, 2)),
        ])
    );
    assert_eq!(inventory_menu_slot_item(&store, 5), item_stack(45, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(45, 2)
    );
    assert_eq!(player_slot_item(&store, 39), item_stack(45, 1));
    assert_eq!(player_slot_item(&store, 0), item_stack(45, 2));
}

#[test]
fn apply_local_container_clone_copies_slot_stack_to_cursor_at_max_count() {
    let mut store = WorldStore::new();
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
            max_stack_size: Some(16),
            ..Default::default()
        },
    )]));
    apply_player_instabuild(&mut store, true);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![item_stack(42, 3)],
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let clone = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 2,
            input: ProtocolContainerInput::Clone,
        })
        .unwrap();

    assert_eq!(clone.container_id, 7);
    assert_eq!(clone.state_id, 3);
    assert_eq!(clone.input, ProtocolContainerInput::Clone);
    assert!(clone.changed_slots.is_empty());
    assert_eq!(clone.carried_item, hashed_item_stack(42, 16));
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
    assert_eq!(store.inventory().cursor_item, item_stack(42, 16));
}

#[test]
fn apply_local_container_clone_does_not_apply_without_instabuild() {
    let mut store = WorldStore::new();
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
            max_stack_size: Some(16),
            ..Default::default()
        },
    )]));
    apply_player_instabuild(&mut store, false);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![item_stack(42, 3)],
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let blocked = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 2,
            input: ProtocolContainerInput::Clone,
        })
        .unwrap();

    assert!(blocked.changed_slots.is_empty());
    assert_eq!(blocked.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_container_clone_does_not_apply_with_non_empty_cursor() {
    let mut store = WorldStore::new();
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
            max_stack_size: Some(16),
            ..Default::default()
        },
    )]));
    apply_player_instabuild(&mut store, true);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![item_stack(42, 3)],
        carried_item: item_stack(99, 1),
    });

    let blocked = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 2,
            input: ProtocolContainerInput::Clone,
        })
        .unwrap();

    assert!(blocked.changed_slots.is_empty());
    assert_eq!(blocked.carried_item, hashed_item_stack(99, 1));
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
    assert_eq!(store.inventory().cursor_item, item_stack(99, 1));
}

#[test]
fn inventory_menu_result_slot_stacks_to_cursor_and_armor_slot_enforces_rules() {
    let mut store = WorldStore::new();
    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 1,
        slot: 0,
        item: item_stack(42, 4),
    });
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 4),
    });
    assert!(store.open_local_inventory());

    let craft_again = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(store.inventory().cursor_item, item_stack(42, 8));
    assert!(craft_again.changed_slots.contains_key(&0));

    // Non-equippable item cannot be placed into Head armor slot 5.
    let blocked_armor = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 5,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert!(blocked_armor.changed_slots.is_empty());
    assert_eq!(
        inventory_menu_slot_item(&store, 5),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn set_entity_motion_updates_local_player_pose_delta_movement() {
    let mut store = WorldStore::new();
    store.apply_login(&bbb_protocol::packets::PlayLogin {
        player_id: 7,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: bbb_protocol::packets::CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 0,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: false,
    });
    store.set_local_player_pose(crate::LocalPlayerPoseState::default());
    let applied = store.apply_set_entity_motion(bbb_protocol::packets::SetEntityMotion {
        id: 7,
        delta_movement: bbb_protocol::packets::Vec3d {
            x: 0.4,
            y: 0.35,
            z: -0.25,
        },
    });
    assert!(applied);
    let pose = store.local_player_pose().unwrap();
    assert_eq!(pose.delta_movement.x, 0.4);
    assert_eq!(pose.delta_movement.y, 0.35);
    assert_eq!(pose.delta_movement.z, -0.25);
}

#[test]
fn bundle_pickup_overrides_insert_extract_and_reject_shulker_boxes() {
    let mut store = WorldStore::new();
    let bundle = item_stack(bbb_protocol::item_types::VANILLA_ITEM_BUNDLE_ID, 1);
    let stone = item_stack(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID, 5);
    let shulker = item_stack(bbb_protocol::item_types::VANILLA_ITEM_SHULKER_BOX_ID, 1);
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: bundle.clone(),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: shulker.clone(),
    });
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: stone.clone(),
    });
    assert!(store.open_local_inventory());

    // Left-click stone cursor onto bundle slot inserts stone into bundle.
    let insert = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(insert.carried_item, ProtocolHashedStack::Empty);
    let inserted_contents = inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START)
        .component_patch
        .explicit_bundle_contents()
        .unwrap()
        .to_vec();
    assert_eq!(inserted_contents.len(), 1);
    assert_eq!(
        inserted_contents[0].item_id,
        bbb_protocol::item_types::VANILLA_ITEM_STONE_ID
    );
    assert_eq!(inserted_contents[0].count, 5);

    // Right-click empty cursor onto bundle extracts the top item.
    let extract = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START,
            button_num: 1,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        extract.carried_item,
        hashed_item_stack(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID, 5)
    );
    assert!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START)
            .component_patch
            .explicit_bundle_contents()
            .is_none_or(|items| items.is_empty())
    );

    // Shulker box cannot be inserted into a bundle via carried bundle click.
    store.apply_set_cursor_item(ProtocolSetCursorItem { item: bundle });
    let rejected = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_HOTBAR_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert!(rejected.changed_slots.is_empty());
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START + 1),
        shulker
    );
    assert!(store
        .inventory()
        .cursor_item
        .component_patch
        .explicit_bundle_contents()
        .is_none_or(|items| items.is_empty()));
}

#[test]
fn shulker_box_menu_rejects_nesting_shulker_boxes_on_pickup_and_quick_move() {
    let mut store = WorldStore::new();
    let shulker = item_stack(bbb_protocol::item_types::VANILLA_ITEM_SHULKER_BOX_ID, 1);
    let stone = item_stack(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID, 4);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 9,
        menu_type_id: bbb_protocol::menu_types::VANILLA_MENU_TYPE_SHULKER_BOX_ID,
        title: bbb_protocol::RichTextComponent::plain("Shulker Box"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 63];
    items[27] = shulker.clone();
    items[28] = stone.clone();
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 9,
        state_id: 4,
        items,
        carried_item: shulker.clone(),
    });

    // Direct pickup placement into shulker box slot 0 is rejected.
    let blocked_pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert!(blocked_pickup.changed_slots.is_empty());
    assert_eq!(
        open_container_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, shulker);

    // Quick-move of shulker box from player inventory slot 27 skips shulker slots 0..27.
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: ProtocolItemStackSummary::empty(),
    });
    let _ = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 27,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        open_container_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );

    // Quick-move of normal stone into shulker box slot 0 succeeds.
    let moved_stone = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 28,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert!(moved_stone.changed_slots.contains_key(&0));
    assert_eq!(open_container_slot_item(&store, 0), stone);
}

#[test]
fn quick_craft_clone_fills_each_slot_to_max_stack_in_creative() {
    let mut store = WorldStore::new();
    store.apply_player_abilities(ProtocolPlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID, 1),
    });
    assert!(store.open_local_inventory());

    for (slot_num, button_num) in [
        (-999, 8),
        (INVENTORY_MENU_MAIN_START, 9),
        (INVENTORY_MENU_MAIN_START + 1, 9),
        (-999, 10),
    ] {
        store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num,
                button_num,
                input: ProtocolContainerInput::QuickCraft,
            })
            .unwrap();
    }

    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        item_stack(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID, 64)
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 1),
        item_stack(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID, 64)
    );
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn carried_bundle_refuses_partial_take_from_furnace_result_slot_but_takes_full_stack() {
    fn bundle_template(
        item_id: i32,
        count: i32,
    ) -> bbb_protocol::packets::ItemStackTemplateSummary {
        bbb_protocol::packets::ItemStackTemplateSummary {
            item_id,
            count,
            component_patch: bbb_protocol::packets::DataComponentPatchSummary::default().into(),
        }
    }

    let mut store = WorldStore::new();
    let iron = item_stack(bbb_protocol::item_types::VANILLA_ITEM_IRON_INGOT_ID, 10);
    let stick_60 = bundle_template(bbb_protocol::item_types::VANILLA_ITEM_STICK_ID, 60);
    let mut partial_bundle = item_stack(bbb_protocol::item_types::VANILLA_ITEM_BUNDLE_ID, 1);
    let mut partial_patch = (*partial_bundle.component_patch).clone();
    partial_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID);
    partial_patch.bundle_contents_items = vec![stick_60.clone()];
    partial_bundle.component_patch = partial_patch.into();

    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 14,
        menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
        title: bbb_protocol::RichTextComponent::plain("Furnace"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 39];
    items[2] = iron.clone();
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 14,
        state_id: 1,
        items,
        carried_item: partial_bundle.clone(),
    });

    // Only 4/64 capacity remains, so clicking 10 iron ingots in FurnaceResultSlot
    // (`allowModification == false`) moves 0 items per Vanilla `Slot.tryRemove`.
    let partial_click = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 2,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert!(partial_click.changed_slots.is_empty());
    assert_eq!(open_container_slot_item(&store, 2), iron);
    assert_eq!(
        store
            .inventory()
            .cursor_item
            .component_patch
            .bundle_contents_items,
        vec![stick_60]
    );

    // With 10/64 capacity free (54 sticks already inside), taking the full stack succeeds.
    let stick_54 = bundle_template(bbb_protocol::item_types::VANILLA_ITEM_STICK_ID, 54);
    let mut roomy_bundle = item_stack(bbb_protocol::item_types::VANILLA_ITEM_BUNDLE_ID, 1);
    let mut roomy_patch = (*roomy_bundle.component_patch).clone();
    roomy_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID);
    roomy_patch.bundle_contents_items = vec![stick_54.clone()];
    roomy_bundle.component_patch = roomy_patch.into();
    store.apply_set_cursor_item(ProtocolSetCursorItem { item: roomy_bundle });

    let full_click = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 2,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        full_click.changed_slots.get(&2),
        Some(&ProtocolHashedStack::Empty)
    );
    assert_eq!(
        open_container_slot_item(&store, 2),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        store
            .inventory()
            .cursor_item
            .component_patch
            .bundle_contents_items,
        vec![
            bundle_template(bbb_protocol::item_types::VANILLA_ITEM_IRON_INGOT_ID, 10),
            stick_54,
        ]
    );
}
