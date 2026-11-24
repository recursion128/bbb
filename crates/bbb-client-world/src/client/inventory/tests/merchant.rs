use super::*;

#[test]
fn merchant_offer_scroll_offset_clamps_to_visible_window() {
    let mut store = WorldStore::new();
    assert!(!store.scroll_local_merchant_offers(1));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    assert!(store.apply_merchant_offers(merchant_offers(7, 8)));

    assert!(store.scroll_local_merchant_offers(1));
    assert_eq!(
        store
            .inventory()
            .open_container
            .as_ref()
            .and_then(|container| container.merchant_offers.as_ref())
            .map(|offers| offers.local_scroll_offset),
        Some(1)
    );
    assert!(store.scroll_local_merchant_offers(1));
    assert_eq!(
        store
            .inventory()
            .open_container
            .as_ref()
            .and_then(|container| container.merchant_offers.as_ref())
            .map(|offers| offers.local_scroll_offset),
        Some(1)
    );
    assert!(store.scroll_local_merchant_offers(-1));
    assert_eq!(
        store
            .inventory()
            .open_container
            .as_ref()
            .and_then(|container| container.merchant_offers.as_ref())
            .map(|offers| offers.local_scroll_offset),
        Some(0)
    );
}

#[test]
fn apply_local_merchant_pickup_all_honors_menu_wide_exclusion() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 2);
    items[MERCHANT_PLAYER_MAIN_START as usize] = item_stack(42, 3);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 3,
        items,
        carried_item: item_stack(42, 1),
    });

    let pickup_all = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_PLAYER_MAIN_START + 1,
            button_num: 0,
            input: ProtocolContainerInput::PickupAll,
        })
        .unwrap();

    assert_eq!(pickup_all.changed_slots, BTreeMap::new());
    assert_eq!(pickup_all.carried_item, hashed_item_stack(42, 1));
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        item_stack(42, 2)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START),
        item_stack(42, 3)
    );
}

#[test]
fn apply_local_merchant_quick_move_moves_non_result_slots() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 3);
    items[MERCHANT_PAYMENT_SLOT_2 as usize] = item_stack(43, 1);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    items[MERCHANT_PLAYER_MAIN_START as usize] = item_stack(44, 2);
    items[MERCHANT_HOTBAR_START as usize] = item_stack(45, 4);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::QuickMove
        ))
    );

    let payment_1_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_PAYMENT_SLOT_1,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        payment_1_move.changed_slots,
        BTreeMap::from([
            (MERCHANT_PAYMENT_SLOT_1, ProtocolHashedStack::Empty),
            (MERCHANT_PLAYER_MAIN_START + 1, hashed_item_stack(42, 3)),
        ])
    );

    let payment_2_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_PAYMENT_SLOT_2,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        payment_2_move.changed_slots,
        BTreeMap::from([
            (MERCHANT_PAYMENT_SLOT_2, ProtocolHashedStack::Empty),
            (MERCHANT_PLAYER_MAIN_START + 2, hashed_item_stack(43, 1)),
        ])
    );

    let main_to_hotbar = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        main_to_hotbar.changed_slots,
        BTreeMap::from([
            (MERCHANT_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
            (MERCHANT_HOTBAR_START + 1, hashed_item_stack(44, 2)),
        ])
    );

    let hotbar_to_main = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_HOTBAR_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        hotbar_to_main.changed_slots,
        BTreeMap::from([
            (MERCHANT_PLAYER_MAIN_START, hashed_item_stack(45, 4)),
            (MERCHANT_HOTBAR_START, ProtocolHashedStack::Empty),
        ])
    );

    let click = store
        .build_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(click.changed_slots, BTreeMap::new());
    assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_2),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START),
        item_stack(45, 4)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START + 1),
        item_stack(42, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START + 2),
        item_stack(43, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_HOTBAR_START),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_HOTBAR_START + 1),
        item_stack(44, 2)
    );
}

#[test]
fn apply_local_merchant_result_quick_move_consumes_exact_selected_offer_payments() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 7);
    items[MERCHANT_PAYMENT_SLOT_2 as usize] = item_stack(43, 2);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 14,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: item_cost(42, 5),
            sell: item_stack(90, 1),
            buy_b: Some(item_cost(43, 2)),
            is_out_of_stock: false,
            uses: 1,
            max_uses: 12,
            xp: 8,
            special_price_diff: -1,
            price_multiplier: 0.1,
            demand: 6,
        }],
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    }));

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (MERCHANT_PAYMENT_SLOT_1, ProtocolHashedStack::Empty),
            (MERCHANT_PAYMENT_SLOT_2, ProtocolHashedStack::Empty),
            (MERCHANT_RESULT_SLOT, ProtocolHashedStack::Empty),
            (MERCHANT_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_2),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_HOTBAR_END - 1),
        item_stack(90, 1)
    );
    let offers = store
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .unwrap();
    assert_eq!(offers.offers[0].uses, 2);
    assert!(!offers.offers[0].is_out_of_stock);
}

#[test]
fn apply_local_merchant_result_pickup_places_result_on_cursor_and_keeps_payable_remainder() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 7);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: item_cost(42, 3),
            sell: item_stack(90, 1),
            buy_b: None,
            is_out_of_stock: false,
            uses: 1,
            max_uses: 12,
            xp: 8,
            special_price_diff: 0,
            price_multiplier: 0.05,
            demand: 0,
        }],
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    }));

    let pickup = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Pickup,
        })
        .unwrap();
    assert_eq!(
        pickup.changed_slots,
        BTreeMap::from([(MERCHANT_PAYMENT_SLOT_1, hashed_item_stack(42, 4))])
    );
    assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        item_stack(42, 4)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        item_stack(90, 1)
    );
    assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
    let offers = store
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .unwrap();
    assert_eq!(offers.offers[0].uses, 2);
}

#[test]
fn apply_local_merchant_result_secondary_pickup_requires_server_authority() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 3);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: item_cost(42, 3),
            sell: item_stack(90, 1),
            buy_b: None,
            is_out_of_stock: false,
            uses: 1,
            max_uses: 12,
            xp: 8,
            special_price_diff: 0,
            price_multiplier: 0.05,
            demand: 0,
        }],
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    }));

    assert_eq!(
        store.apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 1,
            input: ProtocolContainerInput::Pickup,
        }),
        Err(ContainerClickBuildError::UnsupportedLocalClickInput(
            ProtocolContainerInput::Pickup
        ))
    );
}

#[test]
fn apply_local_merchant_result_quick_move_consumes_overfilled_payment_remainder() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 4);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 15,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: item_cost(42, 3),
            sell: item_stack(90, 1),
            buy_b: None,
            is_out_of_stock: false,
            uses: 1,
            max_uses: 12,
            xp: 8,
            special_price_diff: 0,
            price_multiplier: 0.05,
            demand: 0,
        }],
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    }));

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (MERCHANT_PAYMENT_SLOT_1, hashed_item_stack(42, 1)),
            (MERCHANT_RESULT_SLOT, ProtocolHashedStack::Empty),
            (MERCHANT_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_merchant_result_quick_move_repopulates_result_when_remainder_still_pays() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 7);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: item_cost(42, 3),
            sell: item_stack(90, 1),
            buy_b: None,
            is_out_of_stock: false,
            uses: 1,
            max_uses: 12,
            xp: 8,
            special_price_diff: 0,
            price_multiplier: 0.05,
            demand: 0,
        }],
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    }));

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    // Vanilla `AbstractContainerMenu.doClick` loops `QuickMove` on the merchant result
    // slot while payment satisfies the offer (7 / 3 == 2 trades, leaving 1 in payment).
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (MERCHANT_PAYMENT_SLOT_1, hashed_item_stack(42, 1)),
            (MERCHANT_RESULT_SLOT, ProtocolHashedStack::Empty),
            (MERCHANT_HOTBAR_END - 1, hashed_item_stack(90, 2)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    let offers = store
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .unwrap();
    assert_eq!(offers.offers[0].uses, 3);
}

#[test]
fn apply_local_merchant_result_quick_move_consumes_swapped_payment_slots() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(43, 2);
    items[MERCHANT_PAYMENT_SLOT_2 as usize] = item_stack(42, 4);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 17,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: item_cost(42, 3),
            sell: item_stack(90, 1),
            buy_b: Some(item_cost(43, 2)),
            is_out_of_stock: false,
            uses: 1,
            max_uses: 12,
            xp: 8,
            special_price_diff: 0,
            price_multiplier: 0.05,
            demand: 0,
        }],
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    }));

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (MERCHANT_PAYMENT_SLOT_1, ProtocolHashedStack::Empty),
            (MERCHANT_PAYMENT_SLOT_2, hashed_item_stack(42, 1)),
            (MERCHANT_RESULT_SLOT, ProtocolHashedStack::Empty),
            (MERCHANT_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_2),
        item_stack(42, 1)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
}

#[test]
fn apply_local_merchant_result_quick_move_clears_result_when_offer_runs_out_of_stock() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 7);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 18,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: item_cost(42, 3),
            sell: item_stack(90, 1),
            buy_b: None,
            is_out_of_stock: false,
            uses: 11,
            max_uses: 12,
            xp: 8,
            special_price_diff: 0,
            price_multiplier: 0.05,
            demand: 0,
        }],
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    }));

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots,
        BTreeMap::from([
            (MERCHANT_PAYMENT_SLOT_1, hashed_item_stack(42, 4)),
            (MERCHANT_RESULT_SLOT, ProtocolHashedStack::Empty),
            (MERCHANT_HOTBAR_END - 1, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        item_stack(42, 4)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    let offers = store
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .unwrap();
    assert_eq!(offers.offers[0].uses, 12);
    assert!(offers.offers[0].is_out_of_stock);
}

#[test]
fn apply_local_merchant_result_quick_move_rejects_component_predicate_cost() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 3);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    let mut cost = item_cost(42, 3);
    cost.component_predicate.components.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID];
    cost.component_predicate.components.item_model = Some("minecraft:emerald".to_string());
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: cost,
            sell: item_stack(90, 1),
            buy_b: None,
            is_out_of_stock: false,
            uses: 1,
            max_uses: 12,
            xp: 8,
            special_price_diff: 0,
            price_multiplier: 0.05,
            demand: 0,
        }],
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    }));

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(quick_move.changed_slots, BTreeMap::new());
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        item_stack(42, 3)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_merchant_result_quick_move_accepts_matching_component_predicate_cost() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut matching_payment = item_stack(42, 3);
    matching_payment.component_patch.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID];
    matching_payment.component_patch.item_model = Some("minecraft:emerald".to_string());
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = matching_payment;
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 16,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    let mut cost = item_cost(42, 3);
    cost.component_predicate.components.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID];
    cost.component_predicate.components.item_model = Some("minecraft:emerald".to_string());
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: cost,
            sell: item_stack(90, 1),
            buy_b: None,
            is_out_of_stock: false,
            uses: 1,
            max_uses: 12,
            xp: 8,
            special_price_diff: 0,
            price_multiplier: 0.05,
            demand: 0,
        }],
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    }));

    let quick_move = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        })
        .unwrap();
    assert_eq!(
        quick_move.changed_slots.get(&MERCHANT_PAYMENT_SLOT_1),
        Some(&ProtocolHashedStack::Empty)
    );
    assert_eq!(
        quick_move.changed_slots.get(&MERCHANT_RESULT_SLOT),
        Some(&ProtocolHashedStack::Empty)
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_HOTBAR_END - 1),
        item_stack(90, 1)
    );
}

#[test]
fn apply_local_merchant_result_swap_and_throw_predict_trades() {
    let mut store = WorldStore::new();
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
    items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 9);
    items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
    store.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 17,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    assert!(store.apply_merchant_offers(ProtocolMerchantOffers {
        container_id: 7,
        offers: vec![ProtocolMerchantOffer {
            buy_a: item_cost(42, 3),
            sell: item_stack(90, 1),
            buy_b: None,
            is_out_of_stock: false,
            uses: 0,
            max_uses: 12,
            xp: 8,
            special_price_diff: 0,
            price_multiplier: 0.05,
            demand: 0,
        }],
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    }));

    // Swap into empty hotbar slot 0 takes one trade and keeps the result refreshed.
    let swap = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 0,
            input: ProtocolContainerInput::Swap,
        })
        .unwrap();
    assert_eq!(
        swap.changed_slots,
        BTreeMap::from([
            (MERCHANT_PAYMENT_SLOT_1, hashed_item_stack(42, 6)),
            (MERCHANT_HOTBAR_START, hashed_item_stack(90, 1)),
        ])
    );
    assert_eq!(
        open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
        item_stack(90, 1)
    );

    // Button-1 Throw (Ctrl+Q) loops remaining affordable trades (2 more -> 0 payment left).
    let throw_all = store
        .apply_local_container_click_slot(ContainerClickSlotRequest {
            slot_num: MERCHANT_RESULT_SLOT,
            button_num: 1,
            input: ProtocolContainerInput::Throw,
        })
        .unwrap();
    assert_eq!(
        throw_all.changed_slots,
        BTreeMap::from([
            (MERCHANT_PAYMENT_SLOT_1, ProtocolHashedStack::Empty),
            (MERCHANT_RESULT_SLOT, ProtocolHashedStack::Empty),
        ])
    );
    let offers = store
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .unwrap();
    assert_eq!(offers.offers[0].uses, 3);
}
