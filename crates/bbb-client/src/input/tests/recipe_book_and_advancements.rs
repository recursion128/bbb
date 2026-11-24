use super::*;

#[test]
fn recipe_book_page_arrow_click_updates_local_page_state() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    input.container_mut().recipe_book_search_focused = true;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: (0..21)
            .map(|index| recipe_book_shapeless_entry(index, 2, 200 + index))
            .collect(),
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_PAGE_FORWARD_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_PAGE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(input.recipe_book_page_hud_state().crafting, 1);
    assert!(!input.recipe_book_search_hud_state().focused);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_PAGE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(input.recipe_book_page_hud_state().crafting, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn visible_recipe_highlights_queue_once_in_button_and_recipe_order() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    let mut first = recipe_book_shapeless_entry_with_group(42, 2, Some(7), 200);
    first.highlight = true;
    let mut second = recipe_book_shapeless_entry_with_group(43, 2, Some(7), 201);
    second.highlight = true;
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![first, second],
    });

    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 0);

    assert!(world.recipe_book().highlights.is_empty());
    assert_eq!(counters.recipe_book_seen_recipe_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RecipeBookSeenRecipe(bbb_protocol::packets::RecipeBookSeenRecipeCommand {
            recipe: bbb_protocol::packets::RecipeDisplayId { index: 42 },
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RecipeBookSeenRecipe(bbb_protocol::packets::RecipeBookSeenRecipeCommand {
            recipe: bbb_protocol::packets::RecipeDisplayId { index: 43 },
        })
    );
    let presentation = input.recipe_book_presentation_hud_state();
    assert_eq!(presentation.button_animation_ticks[0], 15);
    assert_eq!(presentation.tab_animation_ticks[1], 15);

    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 1);
    assert_eq!(counters.recipe_book_seen_recipe_commands_queued, 2);
    assert!(rx.try_recv().is_err());
    assert_eq!(
        input
            .recipe_book_presentation_hud_state()
            .button_animation_ticks[0],
        14
    );
}

#[test]
fn saturated_seen_recipe_queue_retries_before_local_highlight_removal() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetCommand::RecipeBookSeenRecipe(
        bbb_protocol::packets::RecipeBookSeenRecipeCommand {
            recipe: bbb_protocol::packets::RecipeDisplayId { index: 1 },
        },
    ))
    .unwrap();
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    let mut entry = recipe_book_shapeless_entry(42, 2, 200);
    entry.highlight = true;
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![entry],
    });

    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 0);
    assert!(world.recipe_book().highlights.contains(&42));
    assert_eq!(counters.recipe_book_seen_recipe_commands_queued, 0);
    let _ = rx.try_recv().unwrap();

    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 0);
    assert!(!world.recipe_book().highlights.contains(&42));
    assert_eq!(counters.recipe_book_seen_recipe_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RecipeBookSeenRecipe(bbb_protocol::packets::RecipeBookSeenRecipeCommand {
            recipe: bbb_protocol::packets::RecipeDisplayId { index: 42 },
        })
    );
}

#[test]
fn closed_recipe_book_defers_seen_command_and_pauses_animation() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    let mut entry = recipe_book_shapeless_entry(42, 2, 200);
    entry.highlight = true;
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![entry],
    });
    let mut settings = world.recipe_book().settings;
    settings.crafting.open = false;
    world.apply_recipe_book_settings(settings);

    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 7);
    assert!(world.recipe_book().highlights.contains(&42));
    assert!(rx.try_recv().is_err());

    settings.crafting.open = true;
    world.apply_recipe_book_settings(settings);
    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 0);
    assert!(!world.recipe_book().highlights.contains(&42));
    assert_eq!(
        input
            .recipe_book_presentation_hud_state()
            .button_animation_ticks[0],
        15
    );

    settings.crafting.open = false;
    world.apply_recipe_book_settings(settings);
    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 7);
    assert_eq!(
        input
            .recipe_book_presentation_hud_state()
            .button_animation_ticks[0],
        15
    );

    settings.crafting.open = true;
    world.apply_recipe_book_settings(settings);
    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 7);
    assert_eq!(
        input
            .recipe_book_presentation_hud_state()
            .button_animation_ticks[0],
        8
    );
}

#[test]
fn later_recipe_page_retains_highlight_until_selected() {
    let (tx, mut rx) = mpsc::channel(24);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: (0..21)
            .map(|index| {
                let mut entry = recipe_book_shapeless_entry(index, 2, 200 + index);
                entry.highlight = true;
                entry
            })
            .collect(),
    });

    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 0);
    assert_eq!(world.recipe_book().highlights.len(), 1);
    assert!(world.recipe_book().highlights.contains(&20));
    for expected in 0..20 {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::RecipeBookSeenRecipe(bbb_protocol::packets::RecipeBookSeenRecipeCommand {
                recipe: bbb_protocol::packets::RecipeDisplayId { index: expected },
            })
        );
    }

    input.recipe_book_crafting_page = 1;
    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 0);
    assert!(world.recipe_book().highlights.is_empty());
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RecipeBookSeenRecipe(bbb_protocol::packets::RecipeBookSeenRecipeCommand {
            recipe: bbb_protocol::packets::RecipeDisplayId { index: 20 },
        })
    );
}

#[test]
fn nonselected_tab_retains_highlight_until_selected() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 2;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    let mut entry = recipe_book_shapeless_entry(42, 2, 200);
    entry.highlight = true;
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![entry],
    });

    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 0);
    assert!(world.recipe_book().highlights.contains(&42));
    assert!(rx.try_recv().is_err());

    input.recipe_book_crafting_tab_index = 1;
    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 0);
    assert!(!world.recipe_book().highlights.contains(&42));
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::RecipeBookSeenRecipe(_))
    ));
}

#[test]
fn craftable_filter_defers_highlight_until_filter_is_disabled() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    let mut settings = world.recipe_book().settings;
    settings.crafting.filtering = true;
    world.apply_recipe_book_settings(settings);
    let mut entry = recipe_book_shapeless_entry(42, 2, 200);
    entry.highlight = true;
    entry.contents.crafting_requirements = Some(vec![bbb_protocol::packets::IngredientSummary {
        tag: None,
        item_ids: vec![50],
    }]);
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![entry],
    });

    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 0);
    assert!(world.recipe_book().highlights.contains(&42));
    assert_eq!(
        input
            .recipe_book_presentation_hud_state()
            .tab_animation_ticks[1],
        0
    );
    assert!(rx.try_recv().is_err());

    settings.crafting.filtering = false;
    world.apply_recipe_book_settings(settings);
    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 0);
    assert!(!world.recipe_book().highlights.contains(&42));
    let presentation = input.recipe_book_presentation_hud_state();
    assert_eq!(presentation.tab_animation_ticks[1], 15);
    assert_eq!(presentation.button_animation_ticks[0], 15);
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::RecipeBookSeenRecipe(_))
    ));
}

#[test]
fn search_and_remove_paths_preserve_only_still_visible_highlights() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    input
        .container_mut()
        .recipe_book_search
        .set_text("no-match", None);
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    let mut entry = recipe_book_shapeless_entry(42, 2, 200);
    entry.highlight = true;
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![entry],
    });

    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 0);
    assert!(world.recipe_book().highlights.contains(&42));
    assert!(rx.try_recv().is_err());

    world.apply_recipe_book_remove(bbb_protocol::packets::RecipeBookRemove {
        recipe_ids: vec![bbb_protocol::packets::RecipeDisplayId { index: 42 }],
    });
    input.container_mut().recipe_book_search.set_text("", None);
    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 0);
    assert!(world.recipe_book().highlights.is_empty());
    assert!(rx.try_recv().is_err());
}

#[test]
fn recipe_replace_before_visibility_queues_no_seen_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    let mut settings = world.recipe_book().settings;
    settings.crafting.open = false;
    world.apply_recipe_book_settings(settings);
    let mut entry = recipe_book_shapeless_entry(42, 2, 200);
    entry.highlight = true;
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![entry],
    });
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: Vec::new(),
    });
    settings.crafting.open = true;
    world.apply_recipe_book_settings(settings);

    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 0);

    assert!(world.recipe_book().known.is_empty());
    assert!(world.recipe_book().highlights.is_empty());
    assert_eq!(counters.recipe_book_seen_recipe_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn recipe_book_menu_replacement_resets_button_animation() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    let mut entry = recipe_book_shapeless_entry(42, 2, 200);
    entry.highlight = true;
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![entry],
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 0);
    let _ = rx.try_recv().unwrap();
    assert_eq!(
        input
            .recipe_book_presentation_hud_state()
            .button_animation_ticks[0],
        15
    );

    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    sync_recipe_book_presentation(&mut input, &mut world, &mut counters, &commands, None, 0);
    assert_eq!(
        input.recipe_book_presentation_hud_state(),
        RecipeBookPresentationHudState::default()
    );
}

#[test]
fn recipe_book_recipe_button_click_queues_place_recipe_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    input.container_mut().recipe_book_search_focused = true;
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![recipe_book_shapeless_entry(42, 2, 200)],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert!(!input.recipe_book_search_hud_state().focused);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 42,
            use_max_items: true,
        })
    );
}

#[test]
fn narrow_recipe_book_recipe_button_click_closes_book_and_queues_settings() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    input.container_mut().recipe_book_search_focused = true;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![recipe_book_shapeless_entry(42, 2, 200)],
    });
    let surface_size = PhysicalSize::new(378, 720);
    let origin_x = (378.0 - 176.0) / 2.0;
    let origin_y = (720.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + 14.0 + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert!(!world.recipe_book().settings.crafting.open);
    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 1);
    assert!(!input.recipe_book_search_hud_state().focused);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 42,
            use_max_items: false,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RecipeBookChangeSettings(
            bbb_protocol::packets::RecipeBookChangeSettingsCommand {
                book_type: bbb_protocol::packets::RecipeBookType::Crafting,
                open: false,
                filtering: false,
            }
        )
    );
}

#[test]
fn recipe_book_uncraftable_recipe_button_retry_is_guarded() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![recipe_book_shapeless_entry(42, 2, 200)],
    });
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 7,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
            raw_body: Vec::new(),
            crafting: Some(Box::new(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                    ingredients: Vec::new(),
                    result: recipe_book_slot_display_stack(1, 1),
                    crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
                        raw_payload: vec![0],
                        item_stack: None,
                        tag: None,
                    },
                },
            )),
            furnace: None,
            stonecutter: None,
            smithing: None,
        },
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;
    let recipe_button = Some(PhysicalPosition::new(
        origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
        origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
    ));

    assert!(world.last_ghost_recipe().is_some());
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        recipe_button,
        surface_size,
    ));
    assert!(world.last_ghost_recipe().is_none());
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        recipe_button,
        surface_size,
    ));

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 42,
            use_max_items: false,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn recipe_book_craftable_recipe_button_retry_still_queues() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: test_item_stack(50, 1),
    });
    let mut entry = recipe_book_shapeless_entry(42, 2, 200);
    entry.contents.crafting_requirements = Some(vec![bbb_protocol::packets::IngredientSummary {
        tag: None,
        item_ids: vec![50],
    }]);
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![entry],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;
    let recipe_button = Some(PhysicalPosition::new(
        origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
        origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
    ));

    for _ in 0..2 {
        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            recipe_button,
            surface_size,
        ));
    }

    assert_eq!(counters.place_recipe_commands_queued, 2);
    for _ in 0..2 {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
                container_id: 7,
                recipe_index: 42,
                use_max_items: false,
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn recipe_book_recipe_button_click_uses_search_filtered_collection() {
    let root = unique_input_temp_dir("recipe-book-search");
    write_input_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    input
        .container_mut()
        .recipe_book_search
        .set_text("combo", None);
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            recipe_book_shapeless_entry(42, 2, 200),
            recipe_book_shapeless_entry(43, 2, 0),
        ],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input_with_item_runtime(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(&item_runtime),
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 43,
            use_max_items: false,
        })
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn recipe_book_recipe_button_click_uses_current_multi_recipe_cycle() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_world_time(bbb_protocol::packets::PlayTime {
        game_time: 30,
        clock_updates: Vec::new(),
    });
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            recipe_book_shapeless_entry_with_group(42, 2, Some(7), 200),
            recipe_book_shapeless_entry_with_group(43, 2, Some(7), 201),
        ],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 43,
            use_max_items: false,
        })
    );
}

#[test]
fn recipe_book_right_click_overlay_queues_clicked_recipe() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            recipe_book_shapeless_entry_with_group(42, 2, Some(7), 200),
            recipe_book_shapeless_entry_with_group(43, 2, Some(7), 201),
        ],
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(
        input.recipe_book_overlay_hud_state(),
        Some(RecipeBookOverlayHudState {
            book_type: bbb_protocol::packets::RecipeBookType::Crafting,
            tab_index: 1,
            page_index: 0,
            button_index: 0,
            x: 11,
            y: 31,
        })
    );
    assert_eq!(counters.place_recipe_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    let overlay = input.recipe_book_overlay_hud_state();
    press_key(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::Escape,
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClose(ContainerCloseRequest { container_id: 7 })
    );
    assert_eq!(input.recipe_book_overlay_hud_state(), None);
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    // Vanilla `RecipeBookComponent.initVisuals` (`RecipeBookComponent.java:115-134`)
    // clears any open multi-recipe overlay when reopening the container screen.
    assert_eq!(input.recipe_book_overlay_hud_state(), None);
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));
    assert_eq!(input.recipe_book_overlay_hud_state(), overlay);

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(origin_x + 41.0, origin_y + 37.0)),
        surface_size,
    ));

    assert_eq!(input.recipe_book_overlay_hud_state(), None);
    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 43,
            use_max_items: false,
        })
    );
}

#[test]
fn recipe_book_overlay_uncraftable_recipe_retry_is_guarded() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            recipe_book_shapeless_entry_with_group(42, 2, Some(7), 200),
            recipe_book_shapeless_entry_with_group(43, 2, Some(7), 201),
        ],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;
    let recipe_button = Some(PhysicalPosition::new(
        origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
        origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
    ));
    let overlay_second_recipe = Some(PhysicalPosition::new(origin_x + 41.0, origin_y + 37.0));

    for _ in 0..2 {
        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            recipe_button,
            surface_size,
        ));
        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            overlay_second_recipe,
            surface_size,
        ));
    }

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 43,
            use_max_items: false,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn furnace_recipe_book_recipe_button_click_queues_place_recipe_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.recipe_book_furnace_tab_index = 1;
    input.container_mut().recipe_book_search_focused = true;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_furnace_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![recipe_book_furnace_entry(84, 4, 200)],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert!(!input.recipe_book_search_hud_state().focused);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 84,
            use_max_items: false,
        })
    );
}

#[test]
fn gameplay_keys_are_consumed_while_unsupported_container_is_open() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.set_local_selected_hotbar_slot(0));
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 9,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);

    for code in [KeyCode::Digit5, KeyCode::KeyQ] {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
    }

    assert_eq!(world.local_player().selected_hotbar_slot, 0);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.container_close_commands_queued, 0);
    assert!(world.inventory().open_container.is_some());
    assert!(rx.try_recv().is_err());
}

#[test]
fn escape_key_without_open_container_does_not_queue_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_close_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_key_opens_local_screen_without_seen_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![input_advancement("minecraft:hidden/root", None)],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );

    assert!(input.advancements_screen_is_open());
    assert_eq!(world.selected_advancements_tab(), None);
    assert_eq!(counters.advancements_seen_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_key_selects_first_root_tab_and_queues_opened_tab() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![
            input_advancement("minecraft:z/root", None),
            input_displayed_advancement("minecraft:y/root", None),
            input_displayed_advancement("minecraft:a/root", None),
        ],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );

    assert!(input.advancements_screen_is_open());
    assert_eq!(world.selected_advancements_tab(), Some("minecraft:y/root"));
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::OpenedTab {
            tab: "minecraft:y/root".to_string()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_key_is_consumed_while_container_is_open() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 9,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );

    assert!(!input.advancements_screen_is_open());
    assert!(world.inventory().open_container.is_some());
    assert_eq!(counters.advancements_seen_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_key_is_consumed_while_dialog_is_open() {
    let mut input = ClientInputState::new(true);
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );

    assert!(!input.advancements_screen_is_open());
    assert!(input.current_dialog().is_some());
    assert_eq!(counters.advancements_seen_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn escape_key_closes_advancements_screen_and_queues_seen_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    open_test_advancements_screen(&mut input);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!input.advancements_screen_is_open());
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::ClosedScreen)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_key_closes_advancements_screen_and_queues_seen_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    open_test_advancements_screen(&mut input);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );

    assert!(!input.advancements_screen_is_open());
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::ClosedScreen)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_done_button_click_closes_screen_and_queues_seen_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    open_test_advancements_screen(&mut input);

    let handled = input.handle_advancements_screen_mouse_input(
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(400.0, 580.0)),
        PhysicalSize::new(800, 600),
    );

    assert!(handled);
    assert!(!input.advancements_screen_is_open());
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::ClosedScreen)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_tab_click_selects_tab_and_queues_opened_tab() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![
            input_displayed_advancement("minecraft:y/root", None),
            input_displayed_advancement("minecraft:a/root", None),
        ],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    open_test_advancements_screen(&mut input);
    assert_eq!(
        world.ensure_advancements_screen_selected_tab(),
        Some("minecraft:y/root".to_string())
    );

    let handled = input.handle_advancements_screen_mouse_input(
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(316.0, 212.0)),
        PhysicalSize::new(800, 600),
    );

    assert!(handled);
    assert!(input.advancements_screen_is_open());
    assert_eq!(world.selected_advancements_tab(), Some("minecraft:a/root"));
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::OpenedTab {
            tab: "minecraft:a/root".to_string()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_mouse_wheel_scrolls_selected_tab_locally() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![input_displayed_advancement("minecraft:story/root", None)],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    open_test_advancements_screen(&mut input);
    assert_eq!(
        world.ensure_advancements_screen_selected_tab(),
        Some("minecraft:story/root".to_string())
    );

    let handled = input
        .handle_advancements_screen_mouse_wheel(&world, MouseScrollDelta::LineDelta(0.0, -1.0));

    assert!(handled);
    assert_eq!(
        input
            .advancements_screen()
            .unwrap()
            .scroll_delta(Some("minecraft:story/root")),
        Some((0.0, -16.0))
    );
}

#[test]
fn gameplay_mouse_wheel_is_consumed_while_advancements_screen_is_open() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.set_local_selected_hotbar_slot(0));
    open_test_advancements_screen(&mut input);

    handle_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, 1.0),
    );

    assert_eq!(world.local_player().selected_hotbar_slot, 0);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_left_drag_scrolls_after_first_drag_event_until_release() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![input_displayed_advancement("minecraft:story/root", None)],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    open_test_advancements_screen(&mut input);
    assert_eq!(
        world.ensure_advancements_screen_selected_tab(),
        Some("minecraft:story/root".to_string())
    );

    assert!(input.handle_advancements_screen_mouse_input(
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        None,
        PhysicalSize::new(800, 600),
    ));
    assert!(input.handle_advancements_screen_cursor_moved(
        &world,
        Some(PhysicalPosition::new(100.0, 100.0)),
        Some(PhysicalPosition::new(110.0, 120.0)),
    ));
    assert_eq!(input.inventory_cursor_position(), Some((110, 120)));
    assert_eq!(input.inventory_hovered_slot(), None);
    assert_eq!(
        input
            .advancements_screen()
            .unwrap()
            .scroll_delta(Some("minecraft:story/root")),
        None
    );

    assert!(input.handle_advancements_screen_cursor_moved(
        &world,
        Some(PhysicalPosition::new(110.0, 120.0)),
        Some(PhysicalPosition::new(115.0, 116.0)),
    ));
    assert_eq!(
        input
            .advancements_screen()
            .unwrap()
            .scroll_delta(Some("minecraft:story/root")),
        Some((5.0, -4.0))
    );

    assert!(input.handle_advancements_screen_mouse_input(
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Released,
        None,
        PhysicalSize::new(800, 600),
    ));
    assert!(input.handle_advancements_screen_cursor_moved(
        &world,
        Some(PhysicalPosition::new(115.0, 116.0)),
        Some(PhysicalPosition::new(120.0, 150.0)),
    ));
    assert_eq!(
        input
            .advancements_screen()
            .unwrap()
            .scroll_delta(Some("minecraft:story/root")),
        Some((5.0, -4.0))
    );
    assert_eq!(counters.advancements_seen_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}
