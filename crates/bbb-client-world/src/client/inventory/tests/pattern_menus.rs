use super::*;

#[test]
fn apply_local_beacon_pickup_enforces_payment_tag_and_single_item_limit() {
    let mut store = WorldStore::new();
    apply_item_tags(&mut store, vec![(BEACON_PAYMENT_ITEM_TAG, vec![42])]);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize],
        carried_item: item_stack(43, 3),
    });

    let rejected = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_PAYMENT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(rejected.changed_slots, BTreeMap::new());
    assert_eq!(rejected.carried_item, hashed_item_stack(43, 3));
    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        ProtocolItemStackSummary::empty()
    );

    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(42, 3),
    });
    let placed = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_PAYMENT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        placed.changed_slots,
        BTreeMap::from([(BEACON_PAYMENT_SLOT, hashed_item_stack(42, 1))])
    );
    assert_eq!(placed.carried_item, hashed_item_stack(42, 2));
    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(42, 2));
}

#[test]
fn apply_local_enchantment_pickup_enforces_input_and_lapis_slot_rules() {
    let mut store = WorldStore::new();
    store.set_enchantment_lapis_lazuli_item_ids(BTreeSet::from([43]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: vec![ProtocolItemStackSummary::empty(); ENCHANTMENT_TOTAL_SLOT_COUNT as usize],
        carried_item: item_stack(42, 3),
    });

    let input_place = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ENCHANTMENT_INPUT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        input_place.changed_slots,
        BTreeMap::from([(ENCHANTMENT_INPUT_SLOT, hashed_item_stack(42, 1))])
    );
    assert_eq!(input_place.carried_item, hashed_item_stack(42, 2));

    let rejected_lapis = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ENCHANTMENT_LAPIS_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(rejected_lapis.changed_slots, BTreeMap::new());
    assert_eq!(rejected_lapis.carried_item, hashed_item_stack(42, 2));

    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(43, 2),
    });
    let lapis_place = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ENCHANTMENT_LAPIS_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        lapis_place.changed_slots,
        BTreeMap::from([(ENCHANTMENT_LAPIS_SLOT, hashed_item_stack(43, 2))])
    );
    assert_eq!(lapis_place.carried_item, ProtocolHashedStack::Empty);
}

#[test]
fn apply_local_beacon_quick_move_moves_payment_slot_to_player_reverse() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize];
    items[BEACON_PAYMENT_SLOT as usize] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_PAYMENT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (BEACON_PAYMENT_SLOT, ProtocolHashedStack::Empty),
            (BEACON_HOTBAR_END - 1, hashed_item_stack(42, 1)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_HOTBAR_END - 1),
        item_stack(42, 1)
    );
}

#[test]
fn apply_local_beacon_quick_move_routes_single_payment_item_to_payment_slot() {
    let mut store = WorldStore::new();
    apply_item_tags(&mut store, vec![(BEACON_PAYMENT_ITEM_TAG, vec![42])]);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize];
    items[BEACON_PLAYER_MAIN_START as usize] = item_stack(42, 1);
    items[(BEACON_PLAYER_MAIN_START + 1) as usize] = item_stack(42, 2);
    items[BEACON_HOTBAR_START as usize] = item_stack(43, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let tagged_stack_with_multiple_items = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_PLAYER_MAIN_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        tagged_stack_with_multiple_items.changed_slots,
        BTreeMap::from([
            (BEACON_PLAYER_MAIN_START + 1, ProtocolHashedStack::Empty),
            (BEACON_HOTBAR_START + 1, hashed_item_stack(42, 2)),
        ])
    );

    let payment_to_slot = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        payment_to_slot.changed_slots,
        BTreeMap::from([
            (BEACON_PAYMENT_SLOT, hashed_item_stack(42, 1)),
            (BEACON_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
        ])
    );

    let hotbar_to_main = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        hotbar_to_main.changed_slots,
        BTreeMap::from([
            (BEACON_PLAYER_MAIN_START, hashed_item_stack(43, 3)),
            (BEACON_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_PLAYER_MAIN_START),
        item_stack(43, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_HOTBAR_START + 1),
        item_stack(42, 2)
    );
}

#[test]
fn apply_local_beacon_confirm_consumes_payment_and_updates_effect_data() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize];
    items[BEACON_PAYMENT_SLOT as usize] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert!(store.apply_local_beacon_confirm_effects(4, Some(7)));

    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        store.open_container_data_value(BEACON_PRIMARY_EFFECT_DATA_ID),
        Some(5)
    );
    assert_eq!(
        store.open_container_data_value(BEACON_SECONDARY_EFFECT_DATA_ID),
        Some(8)
    );
}

#[test]
fn apply_local_beacon_confirm_clears_missing_secondary_effect_data() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize];
    items[BEACON_PAYMENT_SLOT as usize] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: BEACON_SECONDARY_EFFECT_DATA_ID,
        value: 8,
    });

    assert!(store.apply_local_beacon_confirm_effects(4, None));

    assert_eq!(
        store.open_container_data_value(BEACON_PRIMARY_EFFECT_DATA_ID),
        Some(5)
    );
    assert_eq!(
        store.open_container_data_value(BEACON_SECONDARY_EFFECT_DATA_ID),
        Some(0)
    );
}

#[test]
fn apply_local_beacon_confirm_requires_beacon_payment() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 17,
        items: vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize],
        carried_item: ProtocolItemStackSummary::empty(),
    });
    store.apply_container_set_data(ProtocolContainerSetData {
        container_id: 7,
        id: BEACON_PRIMARY_EFFECT_DATA_ID,
        value: 5,
    });

    assert!(!store.apply_local_beacon_confirm_effects(7, Some(9)));
    assert_eq!(
        store.open_container_data_value(BEACON_PRIMARY_EFFECT_DATA_ID),
        Some(5)
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        ProtocolItemStackSummary::empty()
    );

    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 8,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ANVIL_TOTAL_SLOT_COUNT as usize];
    items[BEACON_PAYMENT_SLOT as usize] = item_stack(42, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 8,
        state_id: 18,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert!(!store.apply_local_beacon_confirm_effects(4, Some(7)));
    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        item_stack(42, 1)
    );
}

#[test]
fn local_container_button_click_rejects_loom_screen_at_six_pattern_limit() {
    let mut store = WorldStore::new();
    install_loom_banner_pattern_registry(&mut store);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut banner = item_stack(42, 1);
    banner
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID);
    banner.component_patch.banner_pattern_layers =
        vec![bbb_protocol::packets::BannerPatternLayerSummary::default(); 6];
    let mut dye = item_stack(43, 1);
    dye.component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_DYE_TYPE_ID);
    dye.component_patch.dye_color_id = Some(VANILLA_DYE_COLOR_RED_ID);
    let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
    items[LOOM_BANNER_SLOT as usize] = banner;
    items[LOOM_DYE_SLOT as usize] = dye;
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(store.apply_local_container_button_click(0), None);
    assert!(store
        .take_pending_local_interaction_sound_actions()
        .is_empty());
}

#[test]
fn apply_local_loom_result_quick_move_consumes_single_banner_and_dye() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
    items[LOOM_BANNER_SLOT as usize] = item_stack(42, 1);
    items[LOOM_DYE_SLOT as usize] = item_stack(43, 1);
    items[LOOM_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (LOOM_BANNER_SLOT, ProtocolHashedStack::Empty),
            (LOOM_DYE_SLOT, ProtocolHashedStack::Empty),
            (LOOM_RESULT_SLOT, ProtocolHashedStack::Empty),
            (LOOM_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, LOOM_BANNER_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_DYE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_HOTBAR_END - 1),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_loom_result_pickup_consumes_single_banner_and_dye_to_cursor() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
    items[LOOM_BANNER_SLOT as usize] = item_stack(42, 1);
    items[LOOM_DYE_SLOT as usize] = item_stack(43, 1);
    items[LOOM_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([
            (LOOM_BANNER_SLOT, ProtocolHashedStack::Empty),
            (LOOM_DYE_SLOT, ProtocolHashedStack::Empty),
            (LOOM_RESULT_SLOT, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        open_container_slot_item(&store, LOOM_BANNER_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_DYE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn apply_local_loom_result_quick_move_keeps_stacked_inputs_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
    items[LOOM_BANNER_SLOT as usize] = item_stack(42, 2);
    items[LOOM_DYE_SLOT as usize] = item_stack(43, 1);
    items[LOOM_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_BANNER_SLOT),
        item_stack(42, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_DYE_SLOT),
        item_stack(43, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_HOTBAR_END - 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_loom_result_pickup_keeps_pattern_item_result_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[2] = item_stack(44, 1);
    items[LOOM_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );
    assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 1));
    assert_eq!(open_container_slot_item(&store, 1), item_stack(43, 1));
    assert_eq!(open_container_slot_item(&store, 2), item_stack(44, 1));
    assert_eq!(
        open_container_slot_item(&store, LOOM_RESULT_SLOT),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_loom_quick_move_moves_input_slots_to_player_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
    items[LOOM_BANNER_SLOT as usize] = item_stack(42, 3);
    items[LOOM_DYE_SLOT as usize] = item_stack(43, 2);
    items[LOOM_PATTERN_SLOT as usize] = item_stack(44, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let banner_to_player = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_BANNER_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        banner_to_player.changed_slots,
        BTreeMap::from([
            (LOOM_BANNER_SLOT, ProtocolHashedStack::Empty),
            (LOOM_PLAYER_MAIN_START, hashed_item_stack(42, 3)),
        ])
    );

    let dye_to_player = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_DYE_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        dye_to_player.changed_slots,
        BTreeMap::from([
            (LOOM_DYE_SLOT, ProtocolHashedStack::Empty),
            (LOOM_PLAYER_MAIN_START + 1, hashed_item_stack(43, 2)),
        ])
    );

    let pattern_to_player = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_PATTERN_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        pattern_to_player.changed_slots,
        BTreeMap::from([
            (LOOM_PATTERN_SLOT, ProtocolHashedStack::Empty),
            (LOOM_PLAYER_MAIN_START + 2, hashed_item_stack(44, 1)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_BANNER_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_DYE_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_PATTERN_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_PLAYER_MAIN_START),
        item_stack(42, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_PLAYER_MAIN_START + 1),
        item_stack(43, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_PLAYER_MAIN_START + 2),
        item_stack(44, 1)
    );
}

#[test]
fn apply_local_loom_quick_move_routes_tagged_items_to_input_slots() {
    let mut store = WorldStore::new();
    store.items.commit_item_component_defaults(BTreeMap::from([
        (
            43,
            ProtocolDataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_DYE_TYPE_ID],
                ..Default::default()
            },
        ),
        (
            44,
            ProtocolDataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID],
                ..Default::default()
            },
        ),
    ]));
    apply_item_tags(
        &mut store,
        vec![
            (LOOM_BANNER_ITEM_TAG, vec![42]),
            (LOOM_DYE_ITEM_TAG, vec![43]),
            (LOOM_PATTERN_ITEM_TAG, vec![44]),
        ],
    );
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
    items[LOOM_PLAYER_MAIN_START as usize] = item_stack(42, 1);
    items[(LOOM_PLAYER_MAIN_START + 1) as usize] = item_stack(43, 2);
    items[(LOOM_PLAYER_MAIN_START + 2) as usize] = item_stack(44, 1);
    items[LOOM_HOTBAR_START as usize] = item_stack(45, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let banner_to_input = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        banner_to_input.changed_slots,
        BTreeMap::from([
            (LOOM_BANNER_SLOT, hashed_item_stack(42, 1)),
            (LOOM_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
        ])
    );

    let dye_to_input = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_PLAYER_MAIN_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        dye_to_input.changed_slots,
        BTreeMap::from([
            (LOOM_DYE_SLOT, hashed_item_stack(43, 2)),
            (LOOM_PLAYER_MAIN_START + 1, ProtocolHashedStack::Empty),
        ])
    );

    let pattern_to_input = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_PLAYER_MAIN_START + 2,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        pattern_to_input.changed_slots,
        BTreeMap::from([
            (LOOM_PATTERN_SLOT, hashed_item_stack(44, 1)),
            (LOOM_PLAYER_MAIN_START + 2, ProtocolHashedStack::Empty),
        ])
    );

    let hotbar_to_main = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: LOOM_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        hotbar_to_main.changed_slots,
        BTreeMap::from([
            (LOOM_PLAYER_MAIN_START, hashed_item_stack(45, 3)),
            (LOOM_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_BANNER_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_DYE_SLOT),
        item_stack(43, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_PATTERN_SLOT),
        item_stack(44, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_PLAYER_MAIN_START),
        item_stack(45, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, LOOM_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_enchantment_quick_move_moves_menu_slots_to_player_reverse() {
    const ENCHANTMENT_TOTAL_SLOT_COUNT: usize = 38;

    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ENCHANTMENT_TOTAL_SLOT_COUNT];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 3);
    items[29] = item_stack(44, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let input_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ENCHANTMENT_INPUT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        input_move.changed_slots,
        BTreeMap::from([
            (ENCHANTMENT_INPUT_SLOT, ProtocolHashedStack::Empty),
            (ENCHANTMENT_HOTBAR_END - 1, hashed_item_stack(42, 1)),
        ])
    );

    let lapis_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ENCHANTMENT_LAPIS_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        lapis_move.changed_slots,
        BTreeMap::from([
            (ENCHANTMENT_LAPIS_SLOT, ProtocolHashedStack::Empty),
            (ENCHANTMENT_HOTBAR_END - 2, hashed_item_stack(43, 3)),
        ])
    );
    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 29,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );
    let click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: 29,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(click.changed_slots, BTreeMap::new());
    assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_LAPIS_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(open_container_slot_item(&store, 29), item_stack(44, 2));
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_HOTBAR_END - 2),
        item_stack(43, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_HOTBAR_END - 1),
        item_stack(42, 1)
    );
}

#[test]
fn apply_local_enchantment_quick_move_routes_player_items_to_lapis_and_input_slots() {
    const ENCHANTMENT_HOTBAR_START: i16 = 29;

    let mut store = WorldStore::new();
    store.set_enchantment_lapis_lazuli_item_ids(BTreeSet::from([43]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); ENCHANTMENT_TOTAL_SLOT_COUNT as usize];
    items[ENCHANTMENT_HOTBAR_START as usize] = item_stack(43, 3);
    items[(ENCHANTMENT_HOTBAR_START + 1) as usize] = item_stack(50, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let lapis_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ENCHANTMENT_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        lapis_move.changed_slots,
        BTreeMap::from([
            (ENCHANTMENT_LAPIS_SLOT, hashed_item_stack(43, 3)),
            (ENCHANTMENT_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    let input_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ENCHANTMENT_HOTBAR_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        input_move.changed_slots,
        BTreeMap::from([
            (ENCHANTMENT_INPUT_SLOT, hashed_item_stack(50, 1)),
            (ENCHANTMENT_HOTBAR_START + 1, hashed_item_stack(50, 1)),
        ])
    );
    let occupied_input_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: ENCHANTMENT_HOTBAR_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(occupied_input_move.changed_slots, BTreeMap::new());
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_LAPIS_SLOT),
        item_stack(43, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_INPUT_SLOT),
        item_stack(50, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, ENCHANTMENT_HOTBAR_START + 1),
        item_stack(50, 1)
    );
}

#[test]
fn apply_local_cartography_table_result_pickup_consumes_single_inputs_to_cursor() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[CARTOGRAPHY_TABLE_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([
            (CARTOGRAPHY_TABLE_MAP_SLOT, ProtocolHashedStack::Empty),
            (
                CARTOGRAPHY_TABLE_ADDITIONAL_SLOT,
                ProtocolHashedStack::Empty
            ),
            (CARTOGRAPHY_TABLE_RESULT_SLOT, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_MAP_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn apply_local_cartography_table_result_quick_move_predicts_single_input_consumption() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[CARTOGRAPHY_TABLE_RESULT_SLOT as usize] = item_stack(90, 1);
    items[30] = item_stack(44, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let result_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        result_move.changed_slots,
        BTreeMap::from([
            (CARTOGRAPHY_TABLE_MAP_SLOT, ProtocolHashedStack::Empty),
            (
                CARTOGRAPHY_TABLE_ADDITIONAL_SLOT,
                ProtocolHashedStack::Empty
            ),
            (CARTOGRAPHY_TABLE_RESULT_SLOT, ProtocolHashedStack::Empty),
            (CARTOGRAPHY_TABLE_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(result_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: 30,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );
    let click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: 30,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(click.changed_slots, BTreeMap::new());
    assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_MAP_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(open_container_slot_item(&store, 30), item_stack(44, 3));
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_HOTBAR_END - 1),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_cartography_table_result_quick_move_keeps_stacked_inputs_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
    items[CARTOGRAPHY_TABLE_MAP_SLOT as usize] = item_stack(42, 2);
    items[CARTOGRAPHY_TABLE_ADDITIONAL_SLOT as usize] = item_stack(43, 1);
    items[CARTOGRAPHY_TABLE_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_MAP_SLOT),
        item_stack(42, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_ADDITIONAL_SLOT),
        item_stack(43, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_HOTBAR_END - 1),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_cartography_table_quick_move_moves_input_slots_to_player_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
    items[CARTOGRAPHY_TABLE_MAP_SLOT as usize] = item_stack(42, 1);
    items[CARTOGRAPHY_TABLE_ADDITIONAL_SLOT as usize] = item_stack(43, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let map_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_MAP_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        map_move.changed_slots,
        BTreeMap::from([
            (CARTOGRAPHY_TABLE_MAP_SLOT, ProtocolHashedStack::Empty),
            (
                CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
                hashed_item_stack(42, 1)
            ),
        ])
    );
    assert_eq!(map_move.carried_item, ProtocolHashedStack::Empty);

    let additional_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_ADDITIONAL_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        additional_move.changed_slots,
        BTreeMap::from([
            (
                CARTOGRAPHY_TABLE_ADDITIONAL_SLOT,
                ProtocolHashedStack::Empty
            ),
            (
                CARTOGRAPHY_TABLE_PLAYER_MAIN_START + 1,
                hashed_item_stack(43, 3)
            ),
        ])
    );
    assert_eq!(additional_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_MAP_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_ADDITIONAL_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_PLAYER_MAIN_START),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_PLAYER_MAIN_START + 1),
        item_stack(43, 3)
    );
}

#[test]
fn apply_local_cartography_table_quick_move_routes_additional_and_player_ranges() {
    let mut store = WorldStore::new();
    store.set_cartography_additional_items(BTreeMap::from([
        (43, crate::CartographyAdditionalItemKind::Paper),
        (44, crate::CartographyAdditionalItemKind::EmptyMap),
        (45, crate::CartographyAdditionalItemKind::GlassPane),
    ]));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
    items[CARTOGRAPHY_TABLE_PLAYER_MAIN_START as usize] = item_stack(43, 2);
    items[(CARTOGRAPHY_TABLE_PLAYER_MAIN_START + 1) as usize] = item_stack(50, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let additional_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        additional_move.changed_slots,
        BTreeMap::from([
            (CARTOGRAPHY_TABLE_ADDITIONAL_SLOT, hashed_item_stack(43, 2)),
            (
                CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
                ProtocolHashedStack::Empty
            ),
        ])
    );

    let main_to_hotbar = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_PLAYER_MAIN_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        main_to_hotbar.changed_slots,
        BTreeMap::from([
            (
                CARTOGRAPHY_TABLE_PLAYER_MAIN_START + 1,
                ProtocolHashedStack::Empty
            ),
            (CARTOGRAPHY_TABLE_HOTBAR_START, hashed_item_stack(50, 3)),
        ])
    );

    let hotbar_to_main = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        hotbar_to_main.changed_slots,
        BTreeMap::from([
            (
                CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
                hashed_item_stack(50, 3)
            ),
            (CARTOGRAPHY_TABLE_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_ADDITIONAL_SLOT),
        item_stack(43, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_PLAYER_MAIN_START),
        item_stack(50, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_cartography_table_map_id_with_removed_component_quick_move_routes_to_map_slot() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
    let mut map_stack = map_id_item_stack(42, 1, 7);
    map_stack
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID);
    items[CARTOGRAPHY_TABLE_PLAYER_MAIN_START as usize] = map_stack.clone();
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let map_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        map_move.changed_slots,
        BTreeMap::from([
            (
                CARTOGRAPHY_TABLE_MAP_SLOT,
                hashed_map_id_removed_component_item_stack(
                    42,
                    1,
                    7,
                    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID
                )
            ),
            (
                CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
                ProtocolHashedStack::Empty
            ),
        ])
    );
    assert_eq!(map_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_MAP_SLOT),
        map_stack
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_PLAYER_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn cartography_table_map_id_with_use_effects_component_predicts() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
    let mut map_stack = map_id_item_stack(42, 1, 7);
    map_stack
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID);
    map_stack.component_patch.use_effects = Some(ProtocolUseEffectsSummary {
        can_sprint: true,
        interact_vibrations: true,
        speed_multiplier: 1.0,
    });
    items[CARTOGRAPHY_TABLE_PLAYER_MAIN_START as usize] = map_stack.clone();
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let click = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        click.changed_slots,
        BTreeMap::from([
            (
                CARTOGRAPHY_TABLE_MAP_SLOT,
                ProtocolHashedStack::Item(ProtocolHashedItemStack {
                    item_id: 42,
                    count: 1,
                    components: ProtocolHashedComponentPatch {
                        added_components: BTreeMap::from([
                            (VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID, 1_133_294_169,),
                            (
                                VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
                                hash_ops_crc32c_int(7),
                            ),
                        ]),
                        removed_components: BTreeSet::new(),
                    },
                }),
            ),
            (
                CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
                ProtocolHashedStack::Empty,
            ),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_MAP_SLOT),
        map_stack
    );
    assert_eq!(
        open_container_slot_item(&store, CARTOGRAPHY_TABLE_PLAYER_MAIN_START),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_stonecutter_quick_move_moves_input_slot_to_player_forward() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
    items[STONECUTTER_INPUT_SLOT as usize] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: STONECUTTER_INPUT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (STONECUTTER_INPUT_SLOT, ProtocolHashedStack::Empty),
            (STONECUTTER_PLAYER_MAIN_START, hashed_item_stack(42, 3)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_PLAYER_MAIN_START),
        item_stack(42, 3)
    );
}

#[test]
fn apply_local_stonecutter_quick_move_moves_player_main_and_hotbar_ranges() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
    items[STONECUTTER_PLAYER_MAIN_START as usize] = item_stack(42, 3);
    items[STONECUTTER_HOTBAR_START as usize] = item_stack(43, 4);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let main_to_hotbar = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: STONECUTTER_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        main_to_hotbar.changed_slots,
        BTreeMap::from([
            (STONECUTTER_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
            (STONECUTTER_HOTBAR_START + 1, hashed_item_stack(42, 3)),
        ])
    );

    let hotbar_to_main = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: STONECUTTER_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        hotbar_to_main.changed_slots,
        BTreeMap::from([
            (STONECUTTER_PLAYER_MAIN_START, hashed_item_stack(43, 4)),
            (STONECUTTER_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );

    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_PLAYER_MAIN_START),
        item_stack(43, 4)
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_HOTBAR_START + 1),
        item_stack(42, 3)
    );
}

#[test]
fn apply_local_stonecutter_result_pickup_consumes_single_input_to_cursor() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
    items[STONECUTTER_INPUT_SLOT as usize] = item_stack(42, 1);
    items[STONECUTTER_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: STONECUTTER_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();

    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([
            (STONECUTTER_INPUT_SLOT, ProtocolHashedStack::Empty),
            (STONECUTTER_RESULT_SLOT, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
}

#[test]
fn apply_local_stonecutter_result_quick_move_predicts_single_input_consumption() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
    items[STONECUTTER_INPUT_SLOT as usize] = item_stack(42, 1);
    items[STONECUTTER_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: STONECUTTER_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (STONECUTTER_INPUT_SLOT, ProtocolHashedStack::Empty),
            (STONECUTTER_RESULT_SLOT, ProtocolHashedStack::Empty),
            (STONECUTTER_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_INPUT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_HOTBAR_END - 1),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_stonecutter_result_pickup_keeps_remaining_input_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
    items[STONECUTTER_INPUT_SLOT as usize] = item_stack(42, 2);
    items[STONECUTTER_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: STONECUTTER_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_INPUT_SLOT),
        item_stack(42, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        store.inventory().cursor_item,
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_stonecutter_result_quick_move_keeps_blocked_transfer_server_authoritative() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
    items[STONECUTTER_INPUT_SLOT as usize] = item_stack(42, 1);
    items[STONECUTTER_RESULT_SLOT as usize] = item_stack(90, 1);
    for slot in STONECUTTER_PLAYER_MAIN_START..STONECUTTER_HOTBAR_END {
        items[slot as usize] = item_stack(91, 64);
    }
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: STONECUTTER_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();

    assert_eq!(quick_move.changed_slots, BTreeMap::new());
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_INPUT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, STONECUTTER_RESULT_SLOT),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_brewing_stand_quick_move_moves_brewing_slots_to_player_reverse() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BREWING_STAND_ID,
        title: bbb_protocol::RichTextComponent::plain("Brewing Stand"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); BREWING_STAND_TOTAL_SLOT_COUNT as usize];
    items[BREWING_STAND_BOTTLE_SLOT_START as usize] = item_stack(42, 1);
    items[BREWING_STAND_INGREDIENT_SLOT as usize] = item_stack(43, 2);
    items[BREWING_STAND_FUEL_SLOT as usize] = item_stack(44, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let bottle_to_player = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BREWING_STAND_BOTTLE_SLOT_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        bottle_to_player.changed_slots,
        BTreeMap::from([
            (BREWING_STAND_BOTTLE_SLOT_START, ProtocolHashedStack::Empty),
            (BREWING_STAND_HOTBAR_END - 1, hashed_item_stack(42, 1)),
        ])
    );

    let ingredient_to_player = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BREWING_STAND_INGREDIENT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        ingredient_to_player.changed_slots,
        BTreeMap::from([
            (BREWING_STAND_INGREDIENT_SLOT, ProtocolHashedStack::Empty),
            (BREWING_STAND_HOTBAR_END - 2, hashed_item_stack(43, 2)),
        ])
    );

    let fuel_to_player = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BREWING_STAND_FUEL_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        fuel_to_player.changed_slots,
        BTreeMap::from([
            (BREWING_STAND_FUEL_SLOT, ProtocolHashedStack::Empty),
            (BREWING_STAND_HOTBAR_END - 3, hashed_item_stack(44, 3)),
        ])
    );
}

#[test]
fn apply_local_brewing_stand_quick_move_routes_player_items_to_brewing_slots() {
    let mut store = WorldStore::new();
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
            max_stack_size: Some(64),
            ..Default::default()
        },
    )]));
    store.set_brewing_potion_item_ids(BTreeSet::from([42]));
    store.set_brewing_ingredient_item_ids(BTreeSet::from([43]));
    apply_item_tags(&mut store, vec![(BREWING_STAND_FUEL_ITEM_TAG, vec![44])]);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BREWING_STAND_ID,
        title: bbb_protocol::RichTextComponent::plain("Brewing Stand"),
    });
    let mut items =
        vec![ProtocolItemStackSummary::empty(); BREWING_STAND_TOTAL_SLOT_COUNT as usize];
    items[BREWING_STAND_PLAYER_MAIN_START as usize] = item_stack(42, 3);
    items[(BREWING_STAND_PLAYER_MAIN_START + 1) as usize] = item_stack(43, 2);
    items[(BREWING_STAND_PLAYER_MAIN_START + 2) as usize] = item_stack(44, 5);
    items[BREWING_STAND_HOTBAR_START as usize] = item_stack(45, 4);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let potion_to_bottle_slot = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BREWING_STAND_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        potion_to_bottle_slot.changed_slots,
        BTreeMap::from([
            (BREWING_STAND_BOTTLE_SLOT_START, hashed_item_stack(42, 1)),
            (BREWING_STAND_PLAYER_MAIN_START, hashed_item_stack(42, 2)),
        ])
    );

    let ingredient_to_slot = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BREWING_STAND_PLAYER_MAIN_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        ingredient_to_slot.changed_slots,
        BTreeMap::from([
            (BREWING_STAND_INGREDIENT_SLOT, hashed_item_stack(43, 2)),
            (
                BREWING_STAND_PLAYER_MAIN_START + 1,
                ProtocolHashedStack::Empty
            ),
        ])
    );

    let fuel_to_slot = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BREWING_STAND_PLAYER_MAIN_START + 2,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        fuel_to_slot.changed_slots,
        BTreeMap::from([
            (BREWING_STAND_FUEL_SLOT, hashed_item_stack(44, 5)),
            (
                BREWING_STAND_PLAYER_MAIN_START + 2,
                ProtocolHashedStack::Empty
            ),
        ])
    );

    let hotbar_to_main = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BREWING_STAND_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        hotbar_to_main.changed_slots,
        BTreeMap::from([
            (
                BREWING_STAND_PLAYER_MAIN_START + 1,
                hashed_item_stack(45, 4)
            ),
            (BREWING_STAND_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, BREWING_STAND_BOTTLE_SLOT_START),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, BREWING_STAND_PLAYER_MAIN_START),
        item_stack(42, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, BREWING_STAND_INGREDIENT_SLOT),
        item_stack(43, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, BREWING_STAND_FUEL_SLOT),
        item_stack(44, 5)
    );
    assert_eq!(
        open_container_slot_item(&store, BREWING_STAND_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_beacon_swap_enforces_payment_tag_and_single_item_limit() {
    let mut store = WorldStore::new();
    apply_item_tags(&mut store, vec![(BEACON_PAYMENT_ITEM_TAG, vec![42])]);
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize];
    let hotbar_slot = BEACON_HOTBAR_START;
    items[hotbar_slot as usize] = item_stack(43, 2);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    let rejected = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_PAYMENT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();
    assert_eq!(rejected.changed_slots, BTreeMap::new());

    store.apply_container_set_slot(ProtocolContainerSetSlot {
        container_id: 7,
        state_id: 4,
        slot: hotbar_slot,
        item: item_stack(42, 3),
    });
    let accepted = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: BEACON_PAYMENT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();

    assert_eq!(
        accepted.changed_slots,
        BTreeMap::from([
            (BEACON_PAYMENT_SLOT, hashed_item_stack(42, 1)),
            (hotbar_slot, hashed_item_stack(42, 2)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, hotbar_slot),
        item_stack(42, 2)
    );
}
