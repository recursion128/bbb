use super::*;

#[test]
fn apply_local_container_pickup_all_excludes_crafting_result_slot() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CRAFTING_MENU_TOTAL_SLOT_COUNT as usize];
    items[CRAFTING_MENU_RESULT_SLOT as usize] = item_stack(42, 1);
    items[CRAFTING_MENU_PLAYER_MAIN_START as usize] = item_stack(42, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items,
        carried_item: item_stack(42, 1),
    });

    let pickup_all = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CRAFTING_MENU_PLAYER_MAIN_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::PickupAll,
        })
        .unwrap();

    assert_eq!(
        pickup_all.changed_slots,
        BTreeMap::from([(CRAFTING_MENU_PLAYER_MAIN_START, ProtocolHashedStack::Empty)])
    );
    assert_eq!(pickup_all.carried_item, hashed_item_stack(42, 3));
    assert_eq!(
        open_container_slot_item(&store, CRAFTING_MENU_RESULT_SLOT),
        item_stack(42, 1)
    );
}

#[test]
fn apply_local_container_quick_craft_right_drag_places_one_per_slot() {
    let mut store = WorldStore::new();
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 5),
    });
    assert!(store.open_local_inventory());

    store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_START,
            QUICKCRAFT_TYPE_GREEDY,
        ))
        .unwrap();
    store
        .apply_local_container_click_slot(quick_craft_request(
            INVENTORY_MENU_MAIN_START,
            QUICKCRAFT_HEADER_CONTINUE,
            QUICKCRAFT_TYPE_GREEDY,
        ))
        .unwrap();
    store
        .apply_local_container_click_slot(quick_craft_request(
            INVENTORY_MENU_MAIN_START + 1,
            QUICKCRAFT_HEADER_CONTINUE,
            QUICKCRAFT_TYPE_GREEDY,
        ))
        .unwrap();
    let finish = store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_END,
            QUICKCRAFT_TYPE_GREEDY,
        ))
        .unwrap();

    assert_eq!(
        finish.changed_slots,
        BTreeMap::from([
            (INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 1)),
            (INVENTORY_MENU_MAIN_START + 1, hashed_item_stack(42, 1)),
        ])
    );
    assert_eq!(finish.carried_item, hashed_item_stack(42, 3));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        item_stack(42, 1)
    );
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 1),
        item_stack(42, 1)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 3));
}

#[test]
fn apply_local_container_quick_craft_single_slot_finish_uses_pickup_semantics() {
    let mut store = WorldStore::new();
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 5),
    });
    assert!(store.open_local_inventory());

    store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_START,
            QUICKCRAFT_TYPE_GREEDY,
        ))
        .unwrap();
    store
        .apply_local_container_click_slot(quick_craft_request(
            INVENTORY_MENU_MAIN_START,
            QUICKCRAFT_HEADER_CONTINUE,
            QUICKCRAFT_TYPE_GREEDY,
        ))
        .unwrap();
    let finish = store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_END,
            QUICKCRAFT_TYPE_GREEDY,
        ))
        .unwrap();

    assert_eq!(
        finish.changed_slots,
        BTreeMap::from([(INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 1))])
    );
    assert_eq!(finish.carried_item, hashed_item_stack(42, 4));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        item_stack(42, 1)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 4));
}

#[test]
fn apply_local_container_quick_craft_invalid_type_or_order_resets_without_corruption() {
    let mut store = WorldStore::new();
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 8),
    });
    assert!(store.open_local_inventory());

    let clone_start = store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_START,
            QUICKCRAFT_TYPE_CLONE,
        ))
        .unwrap();
    assert!(clone_start.changed_slots.is_empty());
    assert_eq!(clone_start.carried_item, hashed_item_stack(42, 8));

    let continue_without_start = store
        .apply_local_container_click_slot(quick_craft_request(
            INVENTORY_MENU_MAIN_START,
            QUICKCRAFT_HEADER_CONTINUE,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    assert!(continue_without_start.changed_slots.is_empty());
    assert_eq!(
        continue_without_start.carried_item,
        hashed_item_stack(42, 8)
    );

    store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_START,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    let pickup_while_active = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: INVENTORY_MENU_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert!(pickup_while_active.changed_slots.is_empty());
    assert_eq!(pickup_while_active.carried_item, hashed_item_stack(42, 8));

    let stale_continue = store
        .apply_local_container_click_slot(quick_craft_request(
            INVENTORY_MENU_MAIN_START,
            QUICKCRAFT_HEADER_CONTINUE,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    assert!(stale_continue.changed_slots.is_empty());
    assert_eq!(stale_continue.carried_item, hashed_item_stack(42, 8));
    assert_eq!(
        inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 8));
}

#[test]
fn apply_local_container_quick_craft_predicts_generic_menu() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X2_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![ProtocolItemStackSummary::empty(); 54],
        carried_item: item_stack(42, 8),
    });

    store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_START,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    store
        .apply_local_container_click_slot(quick_craft_request(
            0,
            QUICKCRAFT_HEADER_CONTINUE,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    store
        .apply_local_container_click_slot(quick_craft_request(
            1,
            QUICKCRAFT_HEADER_CONTINUE,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    let finish = store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_END,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();

    assert_eq!(
        finish.changed_slots,
        BTreeMap::from([(0, hashed_item_stack(42, 4)), (1, hashed_item_stack(42, 4)),])
    );
    assert_eq!(finish.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 4));
    assert_eq!(open_container_slot_item(&store, 1), item_stack(42, 4));
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_brewing_stand_quick_craft_only_selects_matching_bottle_slots() {
    let mut store = WorldStore::new();
    store.set_brewing_potion_item_ids(BTreeSet::from([42]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BREWING_STAND_ID,
        title: bbb_protocol::RichTextComponent::plain("Brewing Stand"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![ProtocolItemStackSummary::empty(); BREWING_STAND_TOTAL_SLOT_COUNT as usize],
        carried_item: item_stack(42, 3),
    });

    store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_START,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    for slot_num in [
        BREWING_STAND_BOTTLE_SLOT_START,
        BREWING_STAND_BOTTLE_SLOT_START + 1,
        BREWING_STAND_INGREDIENT_SLOT,
    ] {
        store
            .apply_local_container_click_slot(quick_craft_request(
                slot_num,
                QUICKCRAFT_HEADER_CONTINUE,
                QUICKCRAFT_TYPE_CHARITABLE,
            ))
            .unwrap();
    }
    let finish = store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_END,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();

    assert_eq!(
        finish.changed_slots,
        BTreeMap::from([
            (BREWING_STAND_BOTTLE_SLOT_START, hashed_item_stack(42, 1)),
            (
                BREWING_STAND_BOTTLE_SLOT_START + 1,
                hashed_item_stack(42, 1)
            ),
        ])
    );
    assert_eq!(finish.carried_item, hashed_item_stack(42, 1));
    assert_eq!(
        open_container_slot_item(&store, BREWING_STAND_BOTTLE_SLOT_START),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, BREWING_STAND_BOTTLE_SLOT_START + 1),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, BREWING_STAND_INGREDIENT_SLOT),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_crafter_quick_craft_skips_disabled_grid_slots() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items: vec![ProtocolItemStackSummary::empty(); CRAFTER_TOTAL_SLOT_COUNT as usize],
        carried_item: item_stack(42, 6),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: 1,
        value: 1,
    });

    store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_START,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();
    for slot_num in [0, 1, 2] {
        store
            .apply_local_container_click_slot(quick_craft_request(
                slot_num,
                QUICKCRAFT_HEADER_CONTINUE,
                QUICKCRAFT_TYPE_CHARITABLE,
            ))
            .unwrap();
    }
    let finish = store
        .apply_local_container_click_slot(quick_craft_request(
            -999,
            QUICKCRAFT_HEADER_END,
            QUICKCRAFT_TYPE_CHARITABLE,
        ))
        .unwrap();

    assert_eq!(
        finish.changed_slots,
        BTreeMap::from([(0, hashed_item_stack(42, 3)), (2, hashed_item_stack(42, 3))])
    );
    assert_eq!(finish.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn inventory_menu_crafting_result_quick_move_consumes_inputs_once() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 2);
    items[1] = item_stack(42, 3);
    items[2] = item_stack(43, 3);
    items[4] = item_stack(44, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.open_local_inventory());

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );
    assert_eq!(inventory_menu_slot_item(&store, 0), item_stack(90, 2));
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 3));
    assert_eq!(inventory_menu_slot_item(&store, 2), item_stack(43, 3));
    assert_eq!(inventory_menu_slot_item(&store, 4), item_stack(44, 3));
    assert_eq!(
        inventory_menu_slot_item(&store, 44),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn inventory_menu_crafting_result_quick_move_keeps_unmoved_result_remainder() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.items.commit_item_component_defaults(BTreeMap::from([(
        90,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
            max_stack_size: Some(1),
            ..Default::default()
        },
    )]));
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 2);
    items[1] = item_stack(42, 1);
    items[2] = item_stack(43, 1);
    items[4] = item_stack(44, 1);
    items[9..44].fill(item_stack(99, 1));
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.open_local_inventory());

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    // Vanilla `InventoryMenu.quickMoveStack` (`InventoryMenu.java:155-157`) drops any
    // unmoved crafted remainder and clears slot 0.
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (1, ProtocolHashedStack::Empty),
            (2, ProtocolHashedStack::Empty),
            (4, ProtocolHashedStack::Empty),
            (44, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        inventory_menu_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, 1),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, 2),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        inventory_menu_slot_item(&store, 4),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(inventory_menu_slot_item(&store, 43), item_stack(99, 1));
    assert_eq!(inventory_menu_slot_item(&store, 44), item_stack(90, 1));
    assert_eq!(player_slot_item(&store, 7), item_stack(99, 1));
    assert_eq!(player_slot_item(&store, 8), item_stack(90, 1));
}

#[test]
fn inventory_menu_crafting_result_remains_server_owned_after_input_click() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_recipe_book_add(ProtocolRecipeBookAdd {
        entries: vec![recipe_book_crafting_entry(
            7,
            CraftingRecipeDisplaySummary::Shaped {
                width: 2,
                height: 1,
                ingredients: vec![slot_display_item(42), slot_display_item(43)],
                result: slot_display_item(91),
                crafting_station: slot_display_empty(),
            },
        )],
        replace: true,
    });
    let mut server_result = component_rich_item_stack();
    server_result.count = 1;
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = server_result.clone();
    items[1] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 12,
        items,
        carried_item: item_stack(43, 1),
    });
    assert!(store.open_local_inventory());

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 2,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([(2, hashed_item_stack(43, 1))])
    );
    assert_eq!(pickup.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(inventory_menu_slot_item(&store, 0), server_result);
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 1));
    assert_eq!(inventory_menu_slot_item(&store, 2), item_stack(43, 1));
}

#[test]
fn apply_local_container_result_requires_known_crafting_remainders() {
    let mut store = WorldStore::new();
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.open_local_inventory());

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    assert_eq!(inventory_menu_slot_item(&store, 0), item_stack(90, 1));
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 1));
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_container_result_places_default_remainder_in_crafting_slot() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 1);
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
            (1, hashed_item_stack(43, 1))
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(43, 1));
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn inventory_menu_crafting_result_drops_default_remainder_when_inventory_is_full() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    store.items.commit_item_component_defaults(BTreeMap::from([
        (
            43,
            ProtocolDataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                max_stack_size: Some(1),
                ..Default::default()
            },
        ),
        (
            99,
            ProtocolDataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                max_stack_size: Some(1),
                ..Default::default()
            },
        ),
    ]));
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
    items[9..45].fill(item_stack(99, 1));
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
            (1, hashed_item_stack(42, 1))
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        inventory_menu_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 1));
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn inventory_menu_crafting_result_uses_default_prediction_for_recipe_specific_input() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
    items[5] = item_stack(43, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.open_local_inventory());

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );
    assert_eq!(inventory_menu_slot_item(&store, 0), item_stack(90, 1));
    assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 2));
    assert_eq!(inventory_menu_slot_item(&store, 5), item_stack(43, 2));
    assert_eq!(
        inventory_menu_slot_item(&store, 44),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_crafting_menu_quick_move_moves_grid_to_player_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[1] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.container_id, 7);
    assert_eq!(quick_move.state_id, 12);
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (1, ProtocolHashedStack::Empty),
            (10, hashed_item_stack(42, 3))
        ])
    );
    let slots = &open_container_slots(&store);
    assert_eq!(slots[1].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[10].item, item_stack(42, 3));
}

#[test]
fn apply_local_crafting_menu_quick_move_moves_player_to_grid_then_between_player_ranges() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[10] = item_stack(42, 3);
    items[37] = item_stack(43, 4);
    for (offset, item) in items[1..10].iter_mut().enumerate() {
        *item = item_stack(91 + offset as i32, 1);
    }
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let main_to_hotbar = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 10,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        main_to_hotbar.changed_slots,
        BTreeMap::from([
            (10, ProtocolHashedStack::Empty),
            (38, hashed_item_stack(42, 3))
        ])
    );

    let hotbar_to_main = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 37,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        hotbar_to_main.changed_slots,
        BTreeMap::from([
            (10, hashed_item_stack(43, 4)),
            (37, ProtocolHashedStack::Empty)
        ])
    );
    let slots = &open_container_slots(&store);
    assert_eq!(slots[10].item, item_stack(43, 4));
    assert_eq!(slots[37].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[38].item, item_stack(42, 3));
}

#[test]
fn crafting_result_quick_move_consumes_single_inputs() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 1);
    items[5] = item_stack(43, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    let request = ContainerClickSlotRequest {
        slot_num: 0,
        button_num: 0,
        input: ProtocolContainerInput::QuickMove,
    };

    let quick_move = store.apply_local_container_click_slot(request).unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (1, ProtocolHashedStack::Empty),
            (5, ProtocolHashedStack::Empty),
            (45, hashed_item_stack(90, 1)),
        ])
    );
    let slots = &open_container_slots(&store);
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[1].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[5].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[45].item, item_stack(90, 1));
}

#[test]
fn crafting_result_pickup_leaves_result_empty_when_inputs_remain() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
    items[5] = item_stack(43, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
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
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (1, hashed_item_stack(42, 1)),
            (5, hashed_item_stack(43, 1)),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    let slots = &open_container_slots(&store);
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[1].item, item_stack(42, 1));
    assert_eq!(slots[5].item, item_stack(43, 1));
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn crafting_result_pickup_consumes_single_inputs_to_cursor() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 1);
    items[5] = item_stack(43, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
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
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (1, ProtocolHashedStack::Empty),
            (5, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    let slots = &open_container_slots(&store);
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[1].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[5].item, ProtocolItemStackSummary::empty());
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn crafting_result_secondary_pickup_takes_full_result_stack() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 3);
    items[1] = item_stack(42, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    // Vanilla `ResultSlot.remove(int)` always removes the entire result stack on right-click.
    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 1,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (1, hashed_item_stack(42, 1))
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 3));
    let slots = &open_container_slots(&store);
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[1].item, item_stack(42, 1));
    assert_eq!(store.inventory().cursor_item, item_stack(90, 3));
}

#[test]
fn crafting_result_places_default_remainder_in_crafting_slot() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
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
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (1, hashed_item_stack(43, 1))
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    let slots = &open_container_slots(&store);
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[1].item, item_stack(43, 1));
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn crafting_result_inserts_incompatible_default_remainder_into_player_inventory() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
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
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (1, hashed_item_stack(42, 1)),
            (37, hashed_item_stack(43, 1)),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    let slots = &open_container_slots(&store);
    assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
    assert_eq!(slots[1].item, item_stack(42, 1));
    assert_eq!(slots[37].item, item_stack(43, 1));
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn crafting_result_uses_default_prediction_for_recipe_specific_input() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
    items[5] = item_stack(43, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );
    let slots = &open_container_slots(&store);
    assert_eq!(slots[0].item, item_stack(90, 1));
    assert_eq!(slots[1].item, item_stack(42, 2));
    assert_eq!(slots[5].item, item_stack(43, 2));
    assert_eq!(slots[45].item, ProtocolItemStackSummary::empty());
}

#[test]
fn crafting_result_merges_incompatible_default_remainder_into_external_offhand() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(43, 2),
    });
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
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
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (1, hashed_item_stack(42, 1)),
        ])
    );
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT),
        item_stack(43, 3)
    );
}

#[test]
fn crafting_result_drops_incompatible_default_remainder_when_inventory_is_full() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    store.items.commit_item_component_defaults(BTreeMap::from([
        (
            43,
            ProtocolDataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                max_stack_size: Some(1),
                ..Default::default()
            },
        ),
        (
            99,
            ProtocolDataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                max_stack_size: Some(1),
                ..Default::default()
            },
        ),
    ]));
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(99, 1),
    });
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
    items[10..46].fill(item_stack(99, 1));
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
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
        BTreeMap::from([
            (0, ProtocolHashedStack::Empty),
            (1, hashed_item_stack(42, 1)),
        ])
    );
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT),
        item_stack(99, 1)
    );
}

#[test]
fn apply_local_crafter_quick_move_moves_grid_to_player_backwards() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
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

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (2, ProtocolHashedStack::Empty),
            (44, hashed_item_stack(42, 3))
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, 2),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(open_container_slot_item(&store, 44), item_stack(42, 3));
}

#[test]
fn apply_local_crafter_quick_move_skips_disabled_grid_slots() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[9] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: 0,
        value: 1,
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 9,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (1, hashed_item_stack(42, 3)),
            (9, ProtocolHashedStack::Empty)
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(open_container_slot_item(&store, 1), item_stack(42, 3));
    assert_eq!(
        open_container_slot_item(&store, 9),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_crafter_result_slot_requires_server_authority() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 46];
    items[CRAFTER_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    for input in [
        ProtocolContainerInput::Pickup,
        ProtocolContainerInput::QuickMove,
    ] {
        assert_eq!(
            store.apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: CRAFTER_RESULT_SLOT,
                button_num: 0,
                input,
            }),
            Err(ContainerClickBuildError::UnsupportedLocalClickInput(input))
        );
    }
    let click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: CRAFTER_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(click.changed_slots, BTreeMap::new());
    assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, CRAFTER_RESULT_SLOT),
        item_stack(90, 1)
    );
    // `NonInteractiveResultSlot` returns `mayPickup == false` and `mayPlace == false`,
    // so Swap, Throw, and PickupAll on the crafter result slot are deterministic no-ops.
    for (input, button_num) in [
        (ProtocolContainerInput::Swap, 0),
        (ProtocolContainerInput::Throw, 0),
        (ProtocolContainerInput::PickupAll, 0),
    ] {
        let click = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: CRAFTER_RESULT_SLOT,
                button_num,
                input,
            })
            .unwrap();
        assert_eq!(click.changed_slots, BTreeMap::new());
        assert_eq!(
            open_container_slot_item(&store, CRAFTER_RESULT_SLOT),
            item_stack(90, 1)
        );
    }
}

#[test]
fn apply_local_crafting_result_swap_throw_and_pickup_all_follow_vanilla() {
    let mut store = WorldStore::new();
    store.set_default_item_crafting_remainders(BTreeMap::new());
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CRAFTING_MENU_TOTAL_SLOT_COUNT as usize];
    items[CRAFTING_MENU_RESULT_SLOT as usize] = item_stack(90, 4);
    items[CRAFTING_MENU_CRAFT_SLOT_START as usize] = item_stack(42, 2);
    items[CRAFTING_MENU_HOTBAR_START as usize] = item_stack(77, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 5,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    // Swap with a non-empty hotbar slot is a deterministic no-op (`mayPlace == false`).
    let blocked_swap = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CRAFTING_MENU_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();
    assert!(blocked_swap.changed_slots.is_empty());
    assert_eq!(
        open_container_slot_item(&store, CRAFTING_MENU_RESULT_SLOT),
        item_stack(90, 4)
    );

    // Swap with an empty hotbar slot takes the result and decrements grid inputs.
    let hotbar_swap = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CRAFTING_MENU_RESULT_SLOT,
            button_num: 1,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();
    assert_eq!(
        hotbar_swap.changed_slots,
        BTreeMap::from([
            (CRAFTING_MENU_RESULT_SLOT, ProtocolHashedStack::Empty),
            (CRAFTING_MENU_CRAFT_SLOT_START, hashed_item_stack(42, 1)),
            (CRAFTING_MENU_HOTBAR_START + 1, hashed_item_stack(90, 4)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, CRAFTING_MENU_HOTBAR_START + 1),
        item_stack(90, 4)
    );

    // Repopulate result with 2 remaining inputs: button-1 Throw stays server-authoritative
    // because the grid remains non-empty and Vanilla loops recipe refills, while button-0
    // Throw drops one craft deterministically.
    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 6,
        slot: CRAFTING_MENU_CRAFT_SLOT_START,
        item: item_stack(42, 2),
    });
    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 7,
        slot: CRAFTING_MENU_RESULT_SLOT,
        item: item_stack(90, 4),
    });
    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CRAFTING_MENU_RESULT_SLOT,
            button_num: 1,
            input: ProtocolContainerInput::Throw,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Throw
        ))
    );
    let throw_once = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CRAFTING_MENU_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Throw,
        })
        .unwrap();
    assert_eq!(
        throw_once.changed_slots,
        BTreeMap::from([
            (CRAFTING_MENU_RESULT_SLOT, ProtocolHashedStack::Empty),
            (CRAFTING_MENU_CRAFT_SLOT_START, hashed_item_stack(42, 1)),
        ])
    );

    // Swap with empty external offhand (button 40) transfers result to player offhand.
    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 8,
        slot: CRAFTING_MENU_RESULT_SLOT,
        item: item_stack(90, 4),
    });
    let offhand_swap = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CRAFTING_MENU_RESULT_SLOT,
            button_num: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT as i8,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();
    assert_eq!(
        offhand_swap.changed_slots,
        BTreeMap::from([
            (CRAFTING_MENU_RESULT_SLOT, ProtocolHashedStack::Empty),
            (CRAFTING_MENU_CRAFT_SLOT_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        player_slot_item(&store, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT),
        item_stack(90, 4)
    );

    // PickupAll clicking directly on an empty crafting result slot collects matching stacks.
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(90, 1),
    });
    let pickup_all = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CRAFTING_MENU_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::PickupAll,
        })
        .unwrap();
    assert_eq!(
        pickup_all.changed_slots,
        BTreeMap::from([(CRAFTING_MENU_HOTBAR_START + 1, ProtocolHashedStack::Empty)])
    );
    assert_eq!(pickup_all.carried_item, hashed_item_stack(90, 5));
}
