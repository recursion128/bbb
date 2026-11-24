use super::*;

#[test]
fn local_bundle_selection_applies_to_open_container_slots() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: bundle_item_stack(99, 1, 2),
    });
    open_test_container(&mut store, 7);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 1,
        items: vec![item_stack(42, 1), bundle_item_stack(43, 1, 2)],
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert!(store.apply_local_select_bundle_item(1, 1));
    assert_eq!(container_slot_selection(&store, 1), 1);
    assert_eq!(
        player_slot_selection(&store, 1),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );

    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 2,
        items: vec![item_stack(42, 1), bundle_item_stack(43, 1, 2)],
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert_eq!(
        container_slot_selection(&store, 1),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );
    assert!(store.apply_local_select_bundle_item(1, 1));
    assert_eq!(container_slot_selection(&store, 1), 1);

    assert!(!store.apply_local_select_bundle_item(0, 0));
    assert_eq!(
        container_slot_selection(&store, 0),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );

    assert!(!store.apply_local_select_bundle_item(99, 0));

    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 2,
        slot: 1,
        item: bundle_item_stack(44, 1, 1),
    });
    assert_eq!(
        container_slot_selection(&store, 1),
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    );

    assert!(store.apply_local_select_bundle_item(1, 0));
    assert_eq!(container_slot_selection(&store, 1), 0);
}

#[test]
fn build_container_click_slot_uses_open_container_state_and_cursor_item() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: vec![ProtocolItemStackSummary::empty(), item_stack(42, 3)],
        carried_item: item_stack(99, 1),
    });

    let click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: 1,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(click.container_id, 7);
    assert_eq!(click.state_id, 13);
    assert_eq!(click.slot_num, 1);
    assert_eq!(click.button_num, 0);
    assert_eq!(click.input, ProtocolContainerInput::Pickup);
    assert!(click.changed_slots.is_empty());
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
fn apply_local_container_pickup_all_predicts_generic_menu() {
    let mut store = WorldStore::new();
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 5),
    });
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 54];
    items[1] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items,
        carried_item: item_stack(42, 5),
    });

    let pickup_all = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::PickupAll,
        })
        .unwrap();

    assert_eq!(
        pickup_all.changed_slots,
        BTreeMap::from([(1, ProtocolHashedStack::Empty)])
    );
    assert_eq!(pickup_all.carried_item, hashed_item_stack(42, 8));
    assert_eq!(
        store.inventory().open_container.as_ref().unwrap().slots[1].item,
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 8));
}

#[test]
fn apply_local_generic_container_quick_move_moves_chest_to_player_reverse() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X6_ID,
        title: bbb_protocol::RichTextComponent::plain("Large Chest"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 90];
    items[0] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 12);
    assert_eq!(quick_move.input, ProtocolContainerInput::QuickMove);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (89, hashed_item_stack(42, 3))
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    let slots = &open_container_slots(&store);
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[89].item, item_stack(42, 3));
}

#[test]
fn apply_local_generic_container_quick_move_merges_player_to_chest_forward() {
    let mut store = WorldStore::new();
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
            max_stack_size: Some(16),
            ..Default::default()
        },
    )]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 54];
    items[0] = item_stack(42, 15);
    items[18] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 18,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 13);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(42, 16)),
            (1, hashed_item_stack(42, 2)),
            (18, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    let slots = &open_container_slots(&store);
    assert_eq!(slots[0].item, item_stack(42, 16));
    assert_eq!(slots[1].item, item_stack(42, 2));
    assert_eq!(slots[18].item, ProtocolItemStackSummary::empty());
}

#[test]
fn apply_local_generic_3x3_quick_move_moves_dispenser_to_player_reverse() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Dispenser"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 45];
    items[0] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 12);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (44, hashed_item_stack(42, 3))
        ])
    );
    let slots = &open_container_slots(&store);
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[44].item, item_stack(42, 3));
}

#[test]
fn apply_local_generic_3x3_quick_move_moves_player_to_dispenser_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Dispenser"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 45];
    items[9] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 9,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 13);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(42, 3)),
            (9, ProtocolHashedStack::Empty)
        ])
    );
    let slots = &open_container_slots(&store);
    assert_eq!(slots[0].item, item_stack(42, 3));
    assert_eq!(slots[9].item, ProtocolItemStackSummary::empty());
}

#[test]
fn apply_local_hopper_quick_move_moves_hopper_to_player_reverse() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_HOPPER_ID,
        title: bbb_protocol::RichTextComponent::plain("Hopper"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 41];
    items[0] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 12);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (40, hashed_item_stack(42, 3))
        ])
    );
    let slots = &open_container_slots(&store);
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[40].item, item_stack(42, 3));
}

#[test]
fn apply_local_hopper_quick_move_moves_player_to_hopper_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_HOPPER_ID,
        title: bbb_protocol::RichTextComponent::plain("Hopper"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 41];
    items[5] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 5,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 13);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(42, 3)),
            (5, ProtocolHashedStack::Empty)
        ])
    );
    let slots = &open_container_slots(&store);
    assert_eq!(slots[0].item, item_stack(42, 3));
    assert_eq!(slots[5].item, ProtocolItemStackSummary::empty());
}

#[test]
fn apply_local_shulker_box_quick_move_moves_shulker_to_player_reverse() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SHULKER_BOX_ID,
        title: bbb_protocol::RichTextComponent::plain("Shulker Box"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 63];
    items[0] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 12);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (62, hashed_item_stack(42, 3))
        ])
    );
    let slots = &open_container_slots(&store);
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[62].item, item_stack(42, 3));
}

#[test]
fn apply_local_shulker_box_quick_move_moves_player_to_shulker_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SHULKER_BOX_ID,
        title: bbb_protocol::RichTextComponent::plain("Shulker Box"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 63];
    items[27] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 27,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 13);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(42, 3)),
            (27, ProtocolHashedStack::Empty)
        ])
    );
    let slots = &open_container_slots(&store);
    assert_eq!(slots[0].item, item_stack(42, 3));
    assert_eq!(slots[27].item, ProtocolItemStackSummary::empty());
}

#[test]
fn apply_local_furnace_quick_move_moves_result_to_player_reverse() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
        title: bbb_protocol::RichTextComponent::plain("Furnace"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 39];
    items[2] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 2,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 12);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (2, ProtocolHashedStack::Empty),
            (38, hashed_item_stack(42, 3)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, 2),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(open_container_slot_item(&store, 38), item_stack(42, 3));
}

#[test]
fn apply_local_furnace_quick_move_routes_input_and_fuel_to_furnace_slots() {
    let mut store = WorldStore::new();
    store.apply_update_recipes(update_recipes(vec![("minecraft:furnace_input", vec![42])]));
    store.set_furnace_fuel_item_ids(BTreeSet::from([43]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
        title: bbb_protocol::RichTextComponent::plain("Furnace"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 39];
    items[3] = item_stack(42, 3);
    items[30] = item_stack(43, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let input_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 3,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    let fuel_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 30,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        input_move.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(42, 3)),
            (3, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        fuel_move.changed_slots,
        BTreeMap::from([
            (1, hashed_item_stack(43, 2)),
            (30, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
    assert_eq!(open_container_slot_item(&store, 1), item_stack(43, 2));
    assert_eq!(
        open_container_slot_item(&store, 3),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, 30),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_furnace_quick_move_uses_menu_specific_input_property_set() {
    let mut store = WorldStore::new();
    store.apply_update_recipes(update_recipes(vec![("minecraft:furnace_input", vec![42])]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BLAST_FURNACE_ID,
        title: bbb_protocol::RichTextComponent::plain("Blast Furnace"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 39];
    items[3] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let fallback_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 3,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        fallback_move.changed_slots,
        BTreeMap::from([
            (3, ProtocolHashedStack::Empty),
            (30, hashed_item_stack(42, 3)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(open_container_slot_item(&store, 30), item_stack(42, 3));

    store.apply_update_recipes(update_recipes(vec![(
        "minecraft:blast_furnace_input",
        vec![42],
    )]));
    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 13,
        slot: 30,
        item: item_stack(42, 3),
    });

    let blast_input_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 30,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        blast_input_move.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(42, 3)),
            (30, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
    assert_eq!(
        open_container_slot_item(&store, 30),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_furnace_quick_move_prioritizes_input_over_fuel() {
    let mut store = WorldStore::new();
    store.apply_update_recipes(update_recipes(vec![("minecraft:smoker_input", vec![42])]));
    store.set_furnace_fuel_item_ids(BTreeSet::from([42]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMOKER_ID,
        title: bbb_protocol::RichTextComponent::plain("Smoker"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 39];
    items[30] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 30,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(42, 3)),
            (30, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
    assert_eq!(
        open_container_slot_item(&store, 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_furnace_quick_move_moves_input_and_fuel_slots_to_player_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
        title: bbb_protocol::RichTextComponent::plain("Furnace"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 3);
    items[1] = item_stack(43, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let input_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    let fuel_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        input_move.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (3, hashed_item_stack(42, 3)),
        ])
    );
    assert_eq!(
        fuel_move.changed_slots,
        BTreeMap::from([
            (1, ProtocolHashedStack::Empty),
            (4, hashed_item_stack(43, 2)),
        ])
    );
    assert_eq!(open_container_slot_item(&store, 3), item_stack(42, 3));
    assert_eq!(open_container_slot_item(&store, 4), item_stack(43, 2));
}

#[test]
fn apply_local_container_throw_predicts_generic_menu() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 54];
    items[0] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let throw = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Throw,
        })
        .unwrap();

    assert_eq!(
        throw.changed_slots,
        BTreeMap::from([(0, hashed_item_stack(42, 2))])
    );
    assert_eq!(throw.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 2));
}

#[test]
fn apply_local_container_swap_requires_empty_cursor_and_predicts_generic_menu() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 3),
    });
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: item_stack(43, 2),
    });
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(99, 1),
    });
    assert!(store.open_local_inventory());

    let swapped = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();
    assert_eq!(
        swapped.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 3)),
            (INVENTORY_MENU_HOTBAR_START, hashed_item_stack(43, 2)),
        ])
    );
    assert_eq!(swapped.carried_item, hashed_item_stack(99, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        item_stack(42, 3)
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
        item_stack(43, 2)
    );

    store.close_local_container(INVENTORY_MENU_CONTAINER_ID);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 54];
    items[0] = item_stack(42, 3);
    items[45] = item_stack(43, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let swap = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();

    assert_eq!(
        swap.changed_slots,
        BTreeMap::from([
            (0, hashed_item_stack(43, 2)),
            (45, hashed_item_stack(42, 3)),
        ])
    );
    assert_eq!(open_container_slot_item(&store, 0), item_stack(43, 2));
    assert_eq!(open_container_slot_item(&store, 45), item_stack(42, 3));
    assert_eq!(player_slot_item(&store, 0), item_stack(42, 3));
}

#[test]
fn apply_local_open_container_swap_uses_external_offhand_without_hashing_it() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(44, 2),
    });
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 54];
    items[0] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 4,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let swap = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT as i8,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();

    assert_eq!(
        swap.changed_slots,
        BTreeMap::from([(0, hashed_item_stack(44, 2))])
    );
    assert_eq!(open_container_slot_item(&store, 0), item_stack(44, 2));
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT),
        item_stack(42, 3)
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_OFFHAND_SLOT),
        item_stack(42, 3)
    );
}

#[test]
fn apply_local_open_container_offhand_swap_returns_displaced_overflow_to_inventory() {
    let mut store = WorldStore::new();
    apply_item_tags(&mut store, vec![(BEACON_PAYMENT_ITEM_TAG, vec![90])]);
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(90, 3),
    });
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize];
    items[BEACON_PAYMENT_SLOT as usize] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 5,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let swap = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_PAYMENT_SLOT,
            button_num: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT as i8,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();

    assert_eq!(
        swap.changed_slots,
        BTreeMap::from([
            (BEACON_PAYMENT_SLOT, hashed_item_stack(90, 1)),
            (BEACON_HOTBAR_START, hashed_item_stack(42, 1)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_HOTBAR_START),
        item_stack(42, 1)
    );
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT),
        item_stack(90, 2)
    );
}

#[test]
fn open_container_revision_tracks_only_accepted_server_container_mutations() {
    let mut store = WorldStore::new();
    assert_eq!(store.inventory().open_container_revision, 0);

    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 5,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X1_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    assert_eq!(store.inventory().open_container_revision, 1);

    assert!(!store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 6,
        id: 0,
        value: 1,
    }));
    assert_eq!(store.inventory().open_container_revision, 1);

    assert!(
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 5,
            state_id: 2,
            items: vec![item_stack(42, 1)],
            carried_item: ProtocolItemStackSummary::empty(),
        })
    );
    assert_eq!(store.inventory().open_container_revision, 2);

    assert!(store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 5,
        state_id: 3,
        slot: 0,
        item: item_stack(43, 1),
    }));
    assert_eq!(store.inventory().open_container_revision, 3);

    assert!(store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 5,
        id: 0,
        value: 1,
    }));
    assert_eq!(store.inventory().open_container_revision, 4);

    assert!(store.apply_container_close(ProtocolContainerClose { container_id: 99 }));
    assert_eq!(store.inventory().open_container_revision, 5);
}
