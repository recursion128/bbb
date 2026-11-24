use super::*;

#[test]
fn loom_pattern_scroll_changes_visible_button_indices() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = loom_world_with_banner_and_dye();

    assert!(handle_inventory_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, -2.0),
        Some(PhysicalPosition::new(620.0, 296.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert_eq!(input.loom_pattern_scroll_row(), 2);

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(620.0, 296.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(input.loom_selected_pattern_index(), Some(8));
    assert_eq!(counters.container_button_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerButtonClick(ContainerButtonClick {
            container_id: 7,
            button_id: 8,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn loom_scroller_drag_updates_visible_button_indices() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = loom_world_with_banner_and_dye();

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(674.0, 290.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(input.container_mut().loom_pattern_scrolling);
    assert_eq!(input.loom_pattern_scroll_row(), 0);

    assert!(handle_inventory_cursor_moved(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(PhysicalPosition::new(674.0, 328.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(input.container_mut().loom_pattern_scrolling);
    assert_eq!(input.loom_pattern_scroll_row(), 3);

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Released,
        Some(PhysicalPosition::new(674.0, 328.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(!input.container_mut().loom_pattern_scrolling);

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(620.0, 296.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(input.loom_selected_pattern_index(), Some(12));
    assert_eq!(counters.container_button_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerButtonClick(ContainerButtonClick {
            container_id: 7,
            button_id: 12,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn stonecutter_mouse_wheel_scrolls_recipe_grid_button_index_by_rows() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_recipes(UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: (0..25).map(|_| stonecutter_recipe(vec![42])).collect(),
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ItemStackSummary::empty(); 38];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });

    assert!(handle_inventory_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, -2.0),
        Some(PhysicalPosition::new(612.0, 300.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert_eq!(input.container_mut().stonecutter_recipe_scroll_row, 2);

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(628.0, 336.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_button_click_commands_queued, 1);
    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerButtonClick(ContainerButtonClick {
            container_id: 7,
            button_id: 17,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn stonecutter_scroller_drag_updates_recipe_grid_button_index() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_recipes(UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: (0..25).map(|_| stonecutter_recipe(vec![42])).collect(),
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ItemStackSummary::empty(); 38];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(676.0, 300.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(input.container_mut().stonecutter_recipe_scrolling);
    assert_eq!(input.container_mut().stonecutter_recipe_scroll_row, 0);

    assert!(handle_inventory_cursor_moved(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(PhysicalPosition::new(676.0, 318.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(input.container_mut().stonecutter_recipe_scrolling);
    assert_eq!(input.container_mut().stonecutter_recipe_scroll_row, 2);
    assert_eq!(counters.container_button_click_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Released,
        Some(PhysicalPosition::new(676.0, 318.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(!input.container_mut().stonecutter_recipe_scrolling);

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(628.0, 336.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_button_click_commands_queued, 1);
    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerButtonClick(ContainerButtonClick {
            container_id: 7,
            button_id: 17,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn stonecutter_recipe_scroll_resets_when_input_item_changes() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let mut stonecutter_recipes = Vec::new();
    stonecutter_recipes.extend((0..25).map(|_| stonecutter_recipe(vec![42])));
    stonecutter_recipes.extend((0..25).map(|_| stonecutter_recipe(vec![99])));
    world.apply_update_recipes(UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes,
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![ItemStackSummary::empty(); 38];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    assert!(handle_inventory_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, -2.0),
        Some(PhysicalPosition::new(612.0, 300.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert_eq!(input.container_mut().stonecutter_recipe_scroll_row, 2);

    let mut replacement_items = vec![ItemStackSummary::empty(); 38];
    replacement_items[0] = item_stack(99, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 13,
        items: replacement_items,
        carried_item: ItemStackSummary::empty(),
    });
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(612.0, 300.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(input.container_mut().stonecutter_recipe_scroll_row, 0);
    assert_eq!(counters.container_button_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerButtonClick(ContainerButtonClick {
            container_id: 7,
            button_id: 0,
        })
    );
    assert!(rx.try_recv().is_err());
}
