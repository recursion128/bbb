use super::*;

#[test]
fn open_book_from_held_written_book_tracks_active_screen() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: written_book_stack(vec!["First", "Second"]),
    });

    store.apply_open_book(ProtocolOpenBook {
        hand: InteractionHand::MainHand,
    });

    let mut book = requested_book(&store).unwrap().clone();
    assert!(book.is_view());
    assert_eq!(book.generation(), 1);
    assert_eq!(book.hand(), InteractionHand::MainHand);
    assert_eq!(
        book.plain_pages_for_filter(false),
        vec!["First".to_string(), "Second".to_string()]
    );
    assert_eq!(book.current_page(), 0);
    assert!(book.turn_page(1));
    assert_eq!(book.current_page(), 1);
    assert!(!book.turn_page(1));
    assert_eq!(book.current_page(), 1);
}

#[test]
fn open_book_from_held_writable_book_uses_raw_pages() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 40,
        item: writable_book_stack(vec!["Draft page"]),
    });

    store.apply_open_book(ProtocolOpenBook {
        hand: InteractionHand::OffHand,
    });

    let book = requested_book(&store).unwrap();
    assert!(book.is_view());
    assert_eq!(book.hand(), InteractionHand::OffHand);
    assert_eq!(
        book.plain_pages_for_filter(false),
        vec!["Draft page".to_string()]
    );
    assert_eq!(book.current_page(), 0);
}

#[test]
fn tracks_client_ui_ghost_recipe_request() {
    let mut store = WorldStore::new();
    assert!(store.open_local_inventory());
    let recipe_display = stonecutter_recipe_display(100, 101, 102);

    store.apply_place_ghost_recipe(ProtocolPlaceGhostRecipe {
        container_id: 0,
        recipe_display: recipe_display.clone(),
    });

    assert_eq!(
        store.last_ghost_recipe(),
        Some(&GhostRecipeState {
            container_id: 0,
            recipe_display_type_id: VANILLA_RECIPE_DISPLAY_TYPE_STONECUTTER_ID,
            recipe_display_type: "stonecutter".to_string(),
            recipe_display_body_len: 7,
            recipe_display: Some(recipe_display),
            slots: vec![GhostRecipeSlotState {
                slot_id: 0,
                items: vec![item_stack(101, 1)],
                is_result: true,
            }],
        })
    );
    assert_eq!(store.counters().ghost_recipe_packets, 1);
    assert_eq!(store.counters().ghost_recipe_updates_applied, 1);
    assert_eq!(store.counters().ghost_recipe_updates_ignored, 0);
}

#[test]
fn clears_client_ui_ghost_recipe_without_packet_counter() {
    let mut store = WorldStore::new();
    assert!(store.open_local_inventory());

    store.apply_place_ghost_recipe(ProtocolPlaceGhostRecipe {
        container_id: 0,
        recipe_display: stonecutter_recipe_display(100, 101, 102),
    });

    assert!(store.clear_ghost_recipe());
    assert_eq!(store.last_ghost_recipe(), None);
    assert!(!store.clear_ghost_recipe());
    assert_eq!(store.counters().ghost_recipe_packets, 1);
}
