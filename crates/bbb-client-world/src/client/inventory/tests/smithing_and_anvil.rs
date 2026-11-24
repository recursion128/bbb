use super::*;

#[test]
fn apply_local_anvil_input_quick_move_moves_input_slots_to_player_forward() {
    const ANVIL_HOTBAR_START: i16 = 30;

    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 39];
    items[ANVIL_INPUT_SLOT as usize] = item_stack(42, 1);
    items[ANVIL_ADDITIONAL_SLOT as usize] = item_stack(43, 2);
    items[ANVIL_RESULT_SLOT as usize] = item_stack(90, 1);
    items[ANVIL_HOTBAR_START as usize] = item_stack(44, 3);
    items[(ANVIL_HOTBAR_START + 1) as usize] = item_stack(45, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let input_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_INPUT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        input_move.changed_slots,
        BTreeMap::from([
            (ANVIL_INPUT_SLOT, ProtocolHashedStack::Empty),
            (ANVIL_PLAYER_MAIN_START, hashed_item_stack(42, 1)),
        ])
    );

    let additional_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_ADDITIONAL_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        additional_move.changed_slots,
        BTreeMap::from([
            (ANVIL_ADDITIONAL_SLOT, ProtocolHashedStack::Empty),
            (ANVIL_PLAYER_MAIN_START + 1, hashed_item_stack(43, 2)),
        ])
    );

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    let result_move_without_cost = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(result_move_without_cost.changed_slots, BTreeMap::new());
    assert_eq!(
        result_move_without_cost.carried_item,
        ProtocolHashedStack::Empty
    );
    let player_to_input = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        player_to_input.changed_slots,
        BTreeMap::from([
            (ANVIL_INPUT_SLOT, hashed_item_stack(44, 3)),
            (ANVIL_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );

    let player_to_additional = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_HOTBAR_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        player_to_additional.changed_slots,
        BTreeMap::from([
            (ANVIL_ADDITIONAL_SLOT, hashed_item_stack(45, 2)),
            (ANVIL_HOTBAR_START + 1, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_INPUT_SLOT),
        item_stack(44, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_ADDITIONAL_SLOT),
        item_stack(45, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_HOTBAR_START + 1),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_PLAYER_MAIN_START),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_PLAYER_MAIN_START + 1),
        item_stack(43, 2)
    );
}

#[test]
fn apply_local_anvil_result_quick_move_consumes_single_input_when_cost_allows() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ANVIL_TOTAL_SLOT_COUNT as usize];
    items[ANVIL_INPUT_SLOT as usize] = item_stack(42, 1);
    items[ANVIL_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: ANVIL_COST_DATA_ID,
        value: 1,
    });
    store.apply_player_experience(ProtocolPlayerExperience {
        progress: 0.0,
        level: 1,
        total: 0,
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (ANVIL_INPUT_SLOT, ProtocolHashedStack::Empty),
            (ANVIL_RESULT_SLOT, ProtocolHashedStack::Empty),
            (ANVIL_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, ANVIL_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_HOTBAR_END - 1),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_anvil_result_pickup_consumes_single_input_to_cursor_when_cost_allows() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ANVIL_TOTAL_SLOT_COUNT as usize];
    items[ANVIL_INPUT_SLOT as usize] = item_stack(42, 1);
    items[ANVIL_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: ANVIL_COST_DATA_ID,
        value: 1,
    });
    store.apply_player_experience(ProtocolPlayerExperience {
        progress: 0.0,
        level: 1,
        total: 0,
    });

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([
            (ANVIL_INPUT_SLOT, ProtocolHashedStack::Empty),
            (ANVIL_RESULT_SLOT, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        open_container_slot_item(&store, ANVIL_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn apply_local_anvil_result_quick_move_keeps_low_experience_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ANVIL_TOTAL_SLOT_COUNT as usize];
    items[ANVIL_INPUT_SLOT as usize] = item_stack(42, 1);
    items[ANVIL_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: ANVIL_COST_DATA_ID,
        value: 2,
    });
    store.apply_player_experience(ProtocolPlayerExperience {
        progress: 0.0,
        level: 1,
        total: 0,
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(quick_move.changed_slots, BTreeMap::new());
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, ANVIL_INPUT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_HOTBAR_END - 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_anvil_result_pickup_keeps_low_experience_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ANVIL_TOTAL_SLOT_COUNT as usize];
    items[ANVIL_INPUT_SLOT as usize] = item_stack(42, 1);
    items[ANVIL_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: ANVIL_COST_DATA_ID,
        value: 2,
    });
    store.apply_player_experience(ProtocolPlayerExperience {
        progress: 0.0,
        level: 1,
        total: 0,
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_INPUT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_anvil_result_quick_move_keeps_material_input_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ANVIL_TOTAL_SLOT_COUNT as usize];
    items[ANVIL_INPUT_SLOT as usize] = item_stack(42, 1);
    items[ANVIL_ADDITIONAL_SLOT as usize] = item_stack(43, 1);
    items[ANVIL_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: ANVIL_COST_DATA_ID,
        value: 1,
    });
    store.apply_player_experience(ProtocolPlayerExperience {
        progress: 0.0,
        level: 1,
        total: 0,
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(quick_move.changed_slots, BTreeMap::new());
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, ANVIL_INPUT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_ADDITIONAL_SLOT),
        item_stack(43, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_HOTBAR_END - 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_smithing_input_quick_move_moves_input_slots_to_player_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); SMITHING_TOTAL_SLOT_COUNT as usize];
    items[SMITHING_TEMPLATE_SLOT as usize] = item_stack(42, 1);
    items[SMITHING_BASE_SLOT as usize] = item_stack(43, 2);
    items[SMITHING_ADDITIONAL_SLOT as usize] = item_stack(44, 3);
    items[SMITHING_RESULT_SLOT as usize] = item_stack(90, 1);
    items[31] = item_stack(45, 4);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let template_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_TEMPLATE_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        template_move.changed_slots,
        BTreeMap::from([
            (SMITHING_TEMPLATE_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_PLAYER_MAIN_START, hashed_item_stack(42, 1)),
        ])
    );

    let base_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_BASE_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        base_move.changed_slots,
        BTreeMap::from([
            (SMITHING_BASE_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_PLAYER_MAIN_START + 1, hashed_item_stack(43, 2)),
        ])
    );

    let additional_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_ADDITIONAL_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        additional_move.changed_slots,
        BTreeMap::from([
            (SMITHING_ADDITIONAL_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_PLAYER_MAIN_START + 2, hashed_item_stack(44, 3)),
        ])
    );

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 31,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );
    let click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: 31,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(click.changed_slots, BTreeMap::new());
    assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, SMITHING_TEMPLATE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_BASE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(open_container_slot_item(&store, 31), item_stack(45, 4));
    assert_eq!(
        open_container_slot_item(&store, SMITHING_PLAYER_MAIN_START),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_PLAYER_MAIN_START + 1),
        item_stack(43, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_PLAYER_MAIN_START + 2),
        item_stack(44, 3)
    );
}

#[test]
fn apply_local_smithing_result_quick_move_consumes_single_inputs() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); SMITHING_TOTAL_SLOT_COUNT as usize];
    items[SMITHING_TEMPLATE_SLOT as usize] = item_stack(42, 1);
    items[SMITHING_BASE_SLOT as usize] = item_stack(43, 1);
    items[SMITHING_ADDITIONAL_SLOT as usize] = item_stack(44, 1);
    items[SMITHING_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (SMITHING_TEMPLATE_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_BASE_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_ADDITIONAL_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_RESULT_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, SMITHING_TEMPLATE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_BASE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_HOTBAR_END - 1),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_smithing_result_pickup_consumes_single_inputs_to_cursor() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); SMITHING_TOTAL_SLOT_COUNT as usize];
    items[SMITHING_TEMPLATE_SLOT as usize] = item_stack(42, 1);
    items[SMITHING_BASE_SLOT as usize] = item_stack(43, 1);
    items[SMITHING_ADDITIONAL_SLOT as usize] = item_stack(44, 1);
    items[SMITHING_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([
            (SMITHING_TEMPLATE_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_BASE_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_ADDITIONAL_SLOT, ProtocolHashedStack::Empty),
            (SMITHING_RESULT_SLOT, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        open_container_slot_item(&store, SMITHING_TEMPLATE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_BASE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn apply_local_smithing_result_quick_move_keeps_stacked_inputs_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); SMITHING_TOTAL_SLOT_COUNT as usize];
    items[SMITHING_TEMPLATE_SLOT as usize] = item_stack(42, 2);
    items[SMITHING_BASE_SLOT as usize] = item_stack(43, 1);
    items[SMITHING_ADDITIONAL_SLOT as usize] = item_stack(44, 1);
    items[SMITHING_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_TEMPLATE_SLOT),
        item_stack(42, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_BASE_SLOT),
        item_stack(43, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_ADDITIONAL_SLOT),
        item_stack(44, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_HOTBAR_END - 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_smithing_result_pickup_keeps_stacked_inputs_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); SMITHING_TOTAL_SLOT_COUNT as usize];
    items[SMITHING_TEMPLATE_SLOT as usize] = item_stack(42, 2);
    items[SMITHING_BASE_SLOT as usize] = item_stack(43, 1);
    items[SMITHING_ADDITIONAL_SLOT as usize] = item_stack(44, 1);
    items[SMITHING_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_TEMPLATE_SLOT),
        item_stack(42, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_smithing_quick_move_routes_property_set_items_to_inputs_and_player_ranges() {
    let mut store = WorldStore::new();
    store.apply_update_recipes(update_recipes(vec![
        (SMITHING_TEMPLATE_PROPERTY_SET, vec![42]),
        (SMITHING_BASE_PROPERTY_SET, vec![43]),
        (SMITHING_ADDITION_PROPERTY_SET, vec![44]),
    ]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); SMITHING_TOTAL_SLOT_COUNT as usize];
    items[SMITHING_PLAYER_MAIN_START as usize] = item_stack(50, 2);
    items[SMITHING_HOTBAR_START as usize] = item_stack(42, 1);
    items[(SMITHING_HOTBAR_START + 1) as usize] = item_stack(43, 2);
    items[(SMITHING_HOTBAR_START + 2) as usize] = item_stack(44, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let template_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        template_move.changed_slots,
        BTreeMap::from([
            (SMITHING_TEMPLATE_SLOT, hashed_item_stack(42, 1)),
            (SMITHING_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );

    let base_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_HOTBAR_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        base_move.changed_slots,
        BTreeMap::from([
            (SMITHING_BASE_SLOT, hashed_item_stack(43, 2)),
            (SMITHING_HOTBAR_START + 1, ProtocolHashedStack::Empty),
        ])
    );

    let additional_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_HOTBAR_START + 2,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        additional_move.changed_slots,
        BTreeMap::from([
            (SMITHING_ADDITIONAL_SLOT, hashed_item_stack(44, 3)),
            (SMITHING_HOTBAR_START + 2, ProtocolHashedStack::Empty),
        ])
    );

    let range_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: SMITHING_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        range_move.changed_slots,
        BTreeMap::from([
            (SMITHING_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
            (SMITHING_HOTBAR_START, hashed_item_stack(50, 2)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_TEMPLATE_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_BASE_SLOT),
        item_stack(43, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_ADDITIONAL_SLOT),
        item_stack(44, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, SMITHING_HOTBAR_START),
        item_stack(50, 2)
    );
}

#[test]
fn apply_local_grindstone_input_quick_move_moves_inputs_to_player_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
    items[GRINDSTONE_INPUT_SLOT as usize] = item_stack(42, 1);
    items[GRINDSTONE_ADDITIONAL_SLOT as usize] = item_stack(43, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let input_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_INPUT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        input_move.changed_slots,
        BTreeMap::from([
            (GRINDSTONE_INPUT_SLOT, ProtocolHashedStack::Empty),
            (GRINDSTONE_PLAYER_MAIN_START, hashed_item_stack(42, 1)),
        ])
    );

    let additional_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_ADDITIONAL_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        additional_move.changed_slots,
        BTreeMap::from([
            (GRINDSTONE_ADDITIONAL_SLOT, ProtocolHashedStack::Empty),
            (GRINDSTONE_PLAYER_MAIN_START + 1, hashed_item_stack(43, 2)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_PLAYER_MAIN_START),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_PLAYER_MAIN_START + 1),
        item_stack(43, 2)
    );
}

#[test]
fn apply_local_grindstone_player_to_input_quick_move_requires_server_authority() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
    items[GRINDSTONE_PLAYER_MAIN_START as usize] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let request = ContainerClickSlotRequest {
        slot_num: GRINDSTONE_PLAYER_MAIN_START,
        button_num: 0,
        input: ProtocolContainerInput::QuickMove,
    };
    assert_eq!(
        store.apply_local_container_click_slot(request),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );
    let click = store.build_container_click_slot(request).unwrap();
    assert_eq!(click.changed_slots, BTreeMap::new());
    assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_PLAYER_MAIN_START),
        item_stack(42, 3)
    );
}

#[test]
fn apply_local_grindstone_quick_move_routes_default_damageable_items_to_inputs() {
    let mut store = WorldStore::new();
    store.items.commit_item_component_defaults(BTreeMap::from([
        (
            42,
            ProtocolDataComponentPatchSummary {
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                ],
                max_stack_size: Some(1),
                max_damage: Some(64),
                damage: Some(0),
                ..Default::default()
            },
        ),
        (
            43,
            ProtocolDataComponentPatchSummary {
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                ],
                max_stack_size: Some(1),
                max_damage: Some(64),
                damage: Some(0),
                ..Default::default()
            },
        ),
    ]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
    items[GRINDSTONE_PLAYER_MAIN_START as usize] = item_stack(42, 1);
    items[(GRINDSTONE_PLAYER_MAIN_START + 1) as usize] = item_stack(43, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let first_input = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        first_input.changed_slots,
        BTreeMap::from([
            (GRINDSTONE_INPUT_SLOT, hashed_item_stack(42, 1)),
            (GRINDSTONE_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
        ])
    );

    let second_input = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_PLAYER_MAIN_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        second_input.changed_slots,
        BTreeMap::from([
            (GRINDSTONE_ADDITIONAL_SLOT, hashed_item_stack(43, 1)),
            (GRINDSTONE_PLAYER_MAIN_START + 1, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_INPUT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_ADDITIONAL_SLOT),
        item_stack(43, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_PLAYER_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_PLAYER_MAIN_START + 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_grindstone_quick_move_moves_player_ranges_when_inputs_full() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
    items[GRINDSTONE_INPUT_SLOT as usize] = item_stack(10, 1);
    items[GRINDSTONE_ADDITIONAL_SLOT as usize] = item_stack(11, 1);
    items[GRINDSTONE_PLAYER_MAIN_START as usize] = item_stack(42, 3);
    items[GRINDSTONE_HOTBAR_START as usize] = item_stack(43, 4);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let main_to_hotbar = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        main_to_hotbar.changed_slots,
        BTreeMap::from([
            (GRINDSTONE_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
            (GRINDSTONE_HOTBAR_START + 1, hashed_item_stack(42, 3)),
        ])
    );

    let hotbar_to_main = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        hotbar_to_main.changed_slots,
        BTreeMap::from([
            (GRINDSTONE_PLAYER_MAIN_START, hashed_item_stack(43, 4)),
            (GRINDSTONE_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_PLAYER_MAIN_START),
        item_stack(43, 4)
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_HOTBAR_START + 1),
        item_stack(42, 3)
    );
}

#[test]
fn apply_local_grindstone_result_pickup_consumes_inputs_to_cursor() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
    items[GRINDSTONE_INPUT_SLOT as usize] = item_stack(42, 1);
    items[GRINDSTONE_ADDITIONAL_SLOT as usize] = item_stack(43, 1);
    items[GRINDSTONE_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([
            (GRINDSTONE_INPUT_SLOT, ProtocolHashedStack::Empty),
            (GRINDSTONE_ADDITIONAL_SLOT, ProtocolHashedStack::Empty),
            (GRINDSTONE_RESULT_SLOT, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn apply_local_grindstone_result_secondary_pickup_requires_server_authority() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
    items[GRINDSTONE_INPUT_SLOT as usize] = item_stack(42, 1);
    items[GRINDSTONE_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_RESULT_SLOT,
            button_num: 1,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_INPUT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_RESULT_SLOT),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_grindstone_result_quick_move_predicts() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
    items[GRINDSTONE_INPUT_SLOT as usize] = item_stack(42, 1);
    items[GRINDSTONE_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (GRINDSTONE_INPUT_SLOT, ProtocolHashedStack::Empty),
            (GRINDSTONE_RESULT_SLOT, ProtocolHashedStack::Empty),
            (GRINDSTONE_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_HOTBAR_END - 1),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_grindstone_result_quick_move_keeps_blocked_transfer_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
    items[GRINDSTONE_INPUT_SLOT as usize] = item_stack(42, 1);
    items[GRINDSTONE_RESULT_SLOT as usize] = item_stack(90, 1);
    for slot in GRINDSTONE_PLAYER_MAIN_START..GRINDSTONE_HOTBAR_END {
        items[slot as usize] = item_stack(91, 64);
    }
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: GRINDSTONE_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.changed_slots, BTreeMap::new());
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_INPUT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, GRINDSTONE_RESULT_SLOT),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_anvil_throw_predicts_inputs_and_keeps_result_actions_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ANVIL_TOTAL_SLOT_COUNT as usize];
    items[ANVIL_INPUT_SLOT as usize] = item_stack(42, 3);
    items[ANVIL_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let throw_input = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_INPUT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Throw,
        })
        .unwrap();
    assert_eq!(
        throw_input.changed_slots,
        BTreeMap::from([(ANVIL_INPUT_SLOT, hashed_item_stack(42, 2))])
    );

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Throw,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Throw
        ))
    );
    assert_eq!(
        open_container_slot_item(&store, ANVIL_RESULT_SLOT),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_anvil_result_swap_throw_and_pickup_all_guard_follow_vanilla() {
    const ANVIL_HOTBAR_START: i16 = 30;
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ANVIL_TOTAL_SLOT_COUNT as usize];
    items[ANVIL_INPUT_SLOT as usize] = item_stack(42, 1);
    items[ANVIL_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 4,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: ANVIL_COST_DATA_ID,
        value: 1,
    });
    store.apply_player_experience(ProtocolPlayerExperience {
        progress: 0.0,
        level: 2,
        total: 0,
    });

    // Swap into empty hotbar slot 0 consumes the single input and places result in hotbar 0.
    let swap = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();
    assert_eq!(
        swap.changed_slots,
        BTreeMap::from([
            (ANVIL_INPUT_SLOT, ProtocolHashedStack::Empty),
            (ANVIL_RESULT_SLOT, ProtocolHashedStack::Empty),
            (ANVIL_HOTBAR_START, hashed_item_stack(90, 1)),
        ])
    );

    // Reload anvil and verify single-input Throw succeeds when XP covers cost.
    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 5,
        slot: ANVIL_INPUT_SLOT,
        item: item_stack(42, 1),
    });
    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 6,
        slot: ANVIL_RESULT_SLOT,
        item: item_stack(90, 1),
    });
    let throw = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Throw,
        })
        .unwrap();
    assert_eq!(
        throw.changed_slots,
        BTreeMap::from([
            (ANVIL_INPUT_SLOT, ProtocolHashedStack::Empty),
            (ANVIL_RESULT_SLOT, ProtocolHashedStack::Empty),
        ])
    );

    // PickupAll scanning over a matching takeable Anvil result stays server-authoritative
    // because Vanilla `AnvilMenu` inherits `canTakeItemForPickAll == true`.
    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 7,
        slot: ANVIL_INPUT_SLOT,
        item: item_stack(42, 1),
    });
    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 8,
        slot: ANVIL_RESULT_SLOT,
        item: item_stack(90, 1),
    });
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(90, 1),
    });
    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ANVIL_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::PickupAll,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::PickupAll
        ))
    );
}
