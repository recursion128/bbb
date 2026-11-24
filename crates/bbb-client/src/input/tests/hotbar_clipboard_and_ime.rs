use super::*;

#[test]
fn hotbar_preset_activators_save_and_load_with_infinite_materials() {
    let (tx, mut rx) = mpsc::channel(9);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let store_path = unique_input_temp_dir("hotbar-preset").join("hotbar.nbt");
    input.set_hotbar_preset_store_path(store_path.clone());
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    set_player_abilities(&mut world, false, true);
    equip_player_slot(&mut world, 0, 1, 64);

    for code in [KeyCode::KeyC, KeyCode::Digit3] {
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
    assert!(store_path.exists());
    assert_eq!(
        world.action_bar().expect("hotbar saved overlay").content,
        RichTextComponent::plain("Item hotbar saved (restore with X + 3)")
    );
    assert!(rx.try_recv().is_err());
    for code in [KeyCode::KeyC, KeyCode::Digit3] {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Released,
        );
    }

    equip_player_slot(&mut world, 0, 848, 1);
    commands
        .as_ref()
        .unwrap()
        .try_send(NetCommand::SetHeldSlot(0))
        .unwrap();
    for code in [KeyCode::KeyX, KeyCode::Digit3] {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
    }
    assert_eq!(world.inventory().hotbar_items()[0].item_id, Some(848));
    assert_eq!(counters.set_creative_mode_slot_commands_queued, 0);
    assert!(matches!(rx.try_recv(), Ok(NetCommand::SetHeldSlot(0))));
    assert!(rx.try_recv().is_err());
    for state in [ElementState::Released, ElementState::Pressed] {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::Digit3),
            state,
        );
    }
    assert_eq!(world.local_player().selected_hotbar_slot, 0);
    assert_eq!(world.inventory().hotbar_items()[0].item_id, Some(1));
    assert_eq!(world.inventory().hotbar_items()[0].count, 64);
    let mut queued_slots = Vec::new();
    while let Ok(command) = rx.try_recv() {
        match command {
            NetCommand::SetCreativeModeSlot(command) => queued_slots.push(command.slot_num),
            other => panic!("unexpected command queued for preset load: {other:?}"),
        }
    }
    assert_eq!(queued_slots, (36..45).collect::<Vec<i16>>());
    let _ = std::fs::remove_dir_all(store_path.parent().unwrap());
}

#[test]
fn hotbar_preset_activators_fall_through_to_slot_select_without_instabuild() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let store_path = std::env::temp_dir().join(format!(
        "bbb-hotbar-preset-fallthrough-{}-{}.nbt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    input.set_hotbar_preset_store_path(store_path.clone());
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    set_player_abilities(&mut world, false, false);

    for code in [KeyCode::KeyC, KeyCode::Digit5] {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
    }
    assert_eq!(world.local_player().selected_hotbar_slot, 4);
    assert!(world.action_bar().is_none());
    assert!(!store_path.exists());
    assert!(matches!(rx.try_recv(), Ok(NetCommand::SetHeldSlot(4))));
}

#[test]
fn creative_screen_hotbar_keys_swap_hovered_items_even_with_preset_activators() {
    for activator in [KeyCode::KeyC, KeyCode::KeyX] {
        let (tx, mut rx) = mpsc::channel(16);
        let commands = Some(tx);
        let mut input = ClientInputState::container_for_test();
        let store_path = std::env::temp_dir().join(format!(
            "bbb-hotbar-preset-screen-{:?}-{}-{}.nbt",
            activator,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        input.set_hotbar_preset_store_path(store_path.clone());
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
            features: Vec::new(),
        });
        set_player_abilities(&mut world, false, true);
        equip_player_slot(&mut world, 0, 1, 64);
        assert!(world.open_local_inventory_with_operator_tab(false).unwrap());
        world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
            features: vec!["minecraft:vanilla".to_string()],
        });
        input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);

        let surface_size = PhysicalSize::new(1280, 720);
        let slot = bbb_client_render::hud_creative_inventory_slot_layouts(
            bbb_client_render::HudCreativeInventorySlotLayoutKind::Picker,
        )
        .find(|slot| slot.slot == bbb_client_render::HudCreativeInventorySlot::Hotbar(0))
        .expect("creative hotbar slot 0 layout");
        let cursor = PhysicalPosition::new(
            (f64::from(surface_size.width)
                - f64::from(bbb_client_render::HUD_CREATIVE_INVENTORY_WIDTH))
                * 0.5
                + f64::from(slot.x + 8),
            (f64::from(surface_size.height)
                - f64::from(bbb_client_render::HUD_CREATIVE_INVENTORY_HEIGHT))
                * 0.5
                + f64::from(slot.y + 8),
        );
        assert!(handle_inventory_cursor_moved(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            Some(cursor),
            surface_size,
        ));

        for code in [activator, KeyCode::Digit3] {
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
        assert_eq!(world.inventory().hotbar_items()[0].item_id, None);
        assert_eq!(world.inventory().hotbar_items()[2].item_id, Some(1));
        assert_eq!(world.inventory().hotbar_items()[2].count, 64);
        assert_eq!(counters.set_creative_mode_slot_commands_queued, 2);
        assert!(matches!(
            rx.try_recv(),
            Ok(NetCommand::SetCreativeModeSlot(command))
                if command.slot_num == 36 && command.item.item_id.is_none()
        ));
        assert!(matches!(
            rx.try_recv(),
            Ok(NetCommand::SetCreativeModeSlot(command))
                if command.slot_num == 38
                    && command.item.item_id == Some(1)
                    && command.item.count == 64
        ));
        assert!(rx.try_recv().is_err());
        assert!(world.action_bar().is_none());
        assert!(!store_path.exists());
    }
}

#[test]
fn show_only_reduced_info_ors_the_reduced_debug_info_option_with_the_player_flag() {
    // Vanilla `Minecraft.showOnlyReducedInfo` is
    // `player != null && player.isReducedDebugInfo() || options.reducedDebugInfo().get()`,
    // so the option alone already restricts `DebugScreenEntryList.rebuildCurrentList`
    // before any player exists. The
    // `KeyboardHandler` debug bindings (/213/307) stay on
    // the player flag alone.
    let mut input = ClientInputState::new(true);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::EntityHitboxes,
        DebugScreenEntryStatus::AlwaysOn,
    );
    let empty = WorldStore::new();
    let plain = world_with_local_player_id(7);
    let flagged = world_with_reduced_debug_info(7);

    assert!(!input.show_only_reduced_info(&empty));
    assert!(!input.show_only_reduced_info(&plain));
    assert!(input.show_only_reduced_info(&flagged));
    assert!(input.debug_entity_hitboxes_visible_for_world(&plain));
    assert!(!input.debug_entity_hitboxes_visible_for_world(&flagged));

    input.set_reduced_debug_info_option(true);

    // The option half needs no player, unlike the synced flag.
    assert!(input.show_only_reduced_info(&empty));
    assert!(input.show_only_reduced_info(&plain));
    assert!(input.show_only_reduced_info(&flagged));
    assert!(!input.debug_entity_hitboxes_visible_for_world(&plain));

    // `KeyboardHandler` reads `player.isReducedDebugInfo()` directly, so the
    // option must not disarm F3+B / F3+G / F3+C.
    assert!(ClientInputState::debug_world_status_toggles_allowed(Some(
        &plain
    )));
    assert!(!ClientInputState::debug_world_status_toggles_allowed(Some(
        &flagged
    )));
}

#[test]
fn the_last_input_type_follows_the_screen_gated_keyboard_and_mouse_events() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let net_commands = None;
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(input.last_input_type(), LastInputType::None);

    open_test_stats_screen(&mut input);
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(input.last_input_type(), LastInputType::KeyboardTab);
    assert!(input.last_input_type().is_keyboard());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        PhysicalKey::Code(KeyCode::ArrowDown),
        ElementState::Released,
    );
    assert_eq!(input.last_input_type(), LastInputType::KeyboardArrow);

    input.note_mouse_button_input_type();
    assert_eq!(input.last_input_type(), LastInputType::Mouse);
}

#[test]
fn opening_a_screen_from_no_screen_resets_the_last_input_type() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let net_commands = None;
    let mut world = world_with_debug_player(false);
    let now = Instant::now();

    open_test_stats_screen(&mut input);
    crate::input::movement::advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &net_commands,
        now,
        0,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(input.last_input_type(), LastInputType::KeyboardTab);

    input.screen.close();
    crate::input::movement::advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &net_commands,
        now,
        0,
    );
    assert_eq!(input.last_input_type(), LastInputType::KeyboardTab);

    open_test_stats_screen(&mut input);
    crate::input::movement::advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &net_commands,
        now,
        0,
    );
    assert_eq!(input.last_input_type(), LastInputType::None);
}

#[test]
fn anvil_rename_clipboard_shortcuts_select_copy_cut_paste_and_emit_only_on_change() {
    let (tx, _rx) = mpsc::channel(16);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "Sharp Pick",
    );
    assert_eq!(input.anvil_rename_text(), "Sharp Pick");
    assert_eq!(counters.rename_item_commands_queued, 1);

    let mut clipboard = MockDebugClipboard::with_text("stale");
    press_key(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        TEXT_EDIT_SHORTCUT_KEY,
    );

    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyA,
        &mut clipboard,
    );
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyC,
        &mut clipboard,
    );
    assert_eq!(input.anvil_rename_text(), "Sharp Pick");
    assert_eq!(clipboard.text.as_deref(), Some("Sharp Pick"));
    assert_eq!(counters.rename_item_commands_queued, 1);

    input.container_mut().anvil_rename.set_cursor(5, None);
    input.container_mut().anvil_rename.set_selection(0, None);
    clipboard.text = Some("Stone".to_string());
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyV,
        &mut clipboard,
    );
    assert_eq!(input.anvil_rename_text(), "Stone Pick");
    assert_eq!(counters.rename_item_commands_queued, 2);

    input.container_mut().anvil_rename.set_cursor(0, None);
    input.container_mut().anvil_rename.set_selection(5, None);
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyX,
        &mut clipboard,
    );
    assert_eq!(input.anvil_rename_text(), " Pick");
    assert_eq!(clipboard.text.as_deref(), Some("Stone"));
    assert_eq!(counters.rename_item_commands_queued, 3);

    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyX,
        &mut clipboard,
    );
    assert_eq!(input.anvil_rename_text(), " Pick");
    assert_eq!(clipboard.text.as_deref(), Some("Stone"));
    assert_eq!(counters.rename_item_commands_queued, 3);

    input.container_mut().anvil_rename.set_cursor(0, None);
    input.container_mut().anvil_rename.set_selection(0, None);
    clipboard.text = Some("§a\nb".to_string());
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyV,
        &mut clipboard,
    );
    assert_eq!(input.anvil_rename_text(), "ab Pick");
    assert_eq!(counters.rename_item_commands_queued, 4);

    clipboard.text = Some("§\u{7f}\n".to_string());
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyV,
        &mut clipboard,
    );
    assert_eq!(input.anvil_rename_text(), "ab Pick");
    assert_eq!(counters.rename_item_commands_queued, 4);

    input.container_mut().anvil_rename.set_cursor(0, None);
    input.container_mut().anvil_rename.set_selection(0, None);
    clipboard.text = Some("x".repeat(80));
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyV,
        &mut clipboard,
    );
    assert_eq!(input.anvil_rename_text().encode_utf16().count(), 50);
    assert_eq!(counters.rename_item_commands_queued, 5);
}

#[test]
fn recipe_book_search_clipboard_shortcuts_select_copy_cut_paste_and_keep_state() {
    let (tx, _rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::container_for_test();
    input.container_mut().recipe_book_search_focused = true;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    let mut clipboard = MockDebugClipboard::with_text("stale");

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "stone sword",
    );
    assert_eq!(
        input.recipe_book_search_hud_state().edit.text(),
        "stone sword"
    );

    press_key(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        TEXT_EDIT_SHORTCUT_KEY,
    );
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyA,
        &mut clipboard,
    );
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyC,
        &mut clipboard,
    );
    let state = input.recipe_book_search_hud_state();
    assert_eq!(state.edit.text(), "stone sword");
    assert_eq!(state.edit.cursor(), 11);
    assert_eq!(state.edit.selection(), 0);
    assert_eq!(clipboard.text.as_deref(), Some("stone sword"));

    input.container_mut().recipe_book_search.set_cursor(5, None);
    input
        .container_mut()
        .recipe_book_search
        .set_selection(0, None);
    clipboard.text = Some("stick".to_string());
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyV,
        &mut clipboard,
    );
    let state = input.recipe_book_search_hud_state();
    assert_eq!(state.edit.text(), "stick sword");
    assert_eq!(state.edit.cursor(), 5);
    assert_eq!(state.edit.selection(), 5);

    input.container_mut().recipe_book_search.set_cursor(0, None);
    input
        .container_mut()
        .recipe_book_search
        .set_selection(5, None);
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyX,
        &mut clipboard,
    );
    let state = input.recipe_book_search_hud_state();
    assert_eq!(state.edit.text(), " sword");
    assert_eq!(state.edit.cursor(), 0);
    assert_eq!(state.edit.selection(), 0);
    assert_eq!(clipboard.text.as_deref(), Some("stick"));

    input.container_mut().recipe_book_search.set_cursor(0, None);
    input
        .container_mut()
        .recipe_book_search
        .set_selection(0, None);
    clipboard.text = Some("§a\nb".to_string());
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyV,
        &mut clipboard,
    );
    assert_eq!(input.recipe_book_search_hud_state().edit.text(), "ab sword");

    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyA,
        &mut clipboard,
    );
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyX,
        &mut clipboard,
    );
    assert_eq!(input.recipe_book_search_hud_state().edit.text(), "");

    clipboard.text = Some("y".repeat(80));
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyV,
        &mut clipboard,
    );
    let state = input.recipe_book_search_hud_state();
    assert_eq!(state.edit.text().encode_utf16().count(), 50);
    assert_eq!(state.edit.cursor(), 50);
    assert_eq!(state.edit.selection(), 50);
}

#[test]
fn creative_search_clipboard_shortcuts_select_copy_cut_paste_and_keep_filters() {
    let (tx, _rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: Vec::new(),
    });
    set_player_abilities(&mut world, false, true);
    assert!(world.open_local_inventory_with_operator_tab(false).unwrap());
    world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec!["minecraft:vanilla".to_string()],
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);

    press_key(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyT,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "stone");
    let hud = input.creative_inventory_hud_state(&world).unwrap();
    assert_eq!(hud.search.text(), "stone");
    assert!(hud.search_focused);

    let mut clipboard = MockDebugClipboard::with_text("stale");
    press_key(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        TEXT_EDIT_SHORTCUT_KEY,
    );
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyA,
        &mut clipboard,
    );
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyC,
        &mut clipboard,
    );
    let hud = input.creative_inventory_hud_state(&world).unwrap();
    assert_eq!(hud.search.text(), "stone");
    assert_eq!(hud.search.cursor(), 5);
    assert_eq!(hud.search.selection(), 0);
    assert_eq!(clipboard.text.as_deref(), Some("stone"));

    clipboard.text = Some("stick".to_string());
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyV,
        &mut clipboard,
    );
    let hud = input.creative_inventory_hud_state(&world).unwrap();
    assert_eq!(hud.search.text(), "stick");
    assert_eq!(hud.search.cursor(), 5);
    assert_eq!(hud.search.selection(), 5);

    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyA,
        &mut clipboard,
    );
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyX,
        &mut clipboard,
    );
    let hud = input.creative_inventory_hud_state(&world).unwrap();
    assert_eq!(hud.search.text(), "");
    assert_eq!(hud.search.cursor(), 0);
    assert_eq!(hud.search.selection(), 0);
    assert_eq!(clipboard.text.as_deref(), Some("stick"));

    clipboard.text = Some("\n\t".to_string());
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyV,
        &mut clipboard,
    );
    assert_eq!(
        input
            .creative_inventory_hud_state(&world)
            .unwrap()
            .search
            .text(),
        ""
    );

    clipboard.text = Some("z".repeat(80));
    press_key_with_clipboard(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyV,
        &mut clipboard,
    );
    let hud = input.creative_inventory_hud_state(&world).unwrap();
    assert_eq!(hud.search.text().chars().count(), 50);
    assert!(hud.search.text().chars().all(|ch| ch == 'z'));
    assert_eq!(hud.search.cursor(), 50);
    assert_eq!(hud.search.selection(), 50);
}

#[test]
fn sign_book_chat_and_debug_options_edit_shortcuts_reject_shift_and_alt() {
    let (tx, _rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut counters = NetCounters::default();

    // Sign editor: Shift and Alt reject copy/cut; the platform shortcut still copies.
    let sign_pos = BlockPos { x: 3, y: 64, z: 3 };
    let mut sign_world = world_with_sign_text(sign_pos, ["", "", "", ""], ["", "", "", ""]);
    sign_world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 3, y: 64, z: 3 },
        is_front_text: true,
    });
    let mut sign_input = ClientInputState::new(true);
    sign_input.apply_screen_requests(&mut sign_world, &mut NetCounters::default(), &None);
    handle_text_input(
        &mut sign_input,
        &mut counters,
        &mut sign_world,
        &commands,
        "copy",
    );
    press_key(
        &mut sign_input,
        &mut counters,
        &mut sign_world,
        &commands,
        TEXT_EDIT_SHORTCUT_KEY,
    );
    press_key(
        &mut sign_input,
        &mut counters,
        &mut sign_world,
        &commands,
        KeyCode::ShiftLeft,
    );
    let mut clipboard = MockDebugClipboard::with_text("keep");
    press_key_with_clipboard(
        &mut sign_input,
        &mut counters,
        &mut sign_world,
        &commands,
        KeyCode::KeyC,
        &mut clipboard,
    );
    assert_eq!(clipboard.text.as_deref(), Some("keep"));
    assert_eq!(sign_input.screen.sign_editor().unwrap().lines[0], "copy");
    press_key(
        &mut sign_input,
        &mut counters,
        &mut sign_world,
        &commands,
        KeyCode::AltLeft,
    );
    press_key_with_clipboard(
        &mut sign_input,
        &mut counters,
        &mut sign_world,
        &commands,
        KeyCode::KeyX,
        &mut clipboard,
    );
    assert_eq!(sign_input.screen.sign_editor().unwrap().lines[0], "copy");
    assert_eq!(clipboard.text.as_deref(), Some("keep"));
    sign_input.shift_left_down = false;
    sign_input.pressed_keys.remove(&KeyCode::AltLeft);
    press_key_with_clipboard(
        &mut sign_input,
        &mut counters,
        &mut sign_world,
        &commands,
        KeyCode::KeyA,
        &mut clipboard,
    );
    press_key_with_clipboard(
        &mut sign_input,
        &mut counters,
        &mut sign_world,
        &commands,
        KeyCode::KeyC,
        &mut clipboard,
    );
    assert_eq!(clipboard.text.as_deref(), Some("copy"));
    clipboard.text = Some("keep".to_string());

    // Book editor: Shift and Alt reject copy/cut; the platform shortcut still copies.
    let mut book_world = WorldStore::new();
    let mut stack = test_item_stack(42, 1);
    stack.component_patch.writable_book_pages = vec!["hello".into()];
    stack.item_id = Some(bbb_protocol::item_types::VANILLA_ITEM_WRITABLE_BOOK_ID);
    book_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let mut book_input = ClientInputState::new(true);
    assert!(book_input.open_writable_book_editor(
        &mut book_world,
        &mut NetCounters::default(),
        &None,
        InteractionHand::MainHand,
    ));
    press_key(
        &mut book_input,
        &mut counters,
        &mut book_world,
        &commands,
        TEXT_EDIT_SHORTCUT_KEY,
    );
    press_key(
        &mut book_input,
        &mut counters,
        &mut book_world,
        &commands,
        KeyCode::ShiftLeft,
    );
    press_key_with_clipboard(
        &mut book_input,
        &mut counters,
        &mut book_world,
        &commands,
        KeyCode::KeyC,
        &mut clipboard,
    );
    assert_eq!(clipboard.text.as_deref(), Some("keep"));
    book_input.shift_left_down = false;
    press_key(
        &mut book_input,
        &mut counters,
        &mut book_world,
        &commands,
        KeyCode::AltLeft,
    );
    press_key_with_clipboard(
        &mut book_input,
        &mut counters,
        &mut book_world,
        &commands,
        KeyCode::KeyA,
        &mut clipboard,
    );
    press_key_with_clipboard(
        &mut book_input,
        &mut counters,
        &mut book_world,
        &commands,
        KeyCode::KeyX,
        &mut clipboard,
    );
    let book = &book_input.book_screen().unwrap().book;
    assert!(book.is_edit());
    assert_eq!(book.plain_pages_for_filter(false)[0], "hello");
    assert_eq!(clipboard.text.as_deref(), Some("keep"));
    book_input.pressed_keys.remove(&KeyCode::AltLeft);
    press_key_with_clipboard(
        &mut book_input,
        &mut counters,
        &mut book_world,
        &commands,
        KeyCode::KeyA,
        &mut clipboard,
    );
    press_key_with_clipboard(
        &mut book_input,
        &mut counters,
        &mut book_world,
        &commands,
        KeyCode::KeyC,
        &mut clipboard,
    );
    assert_eq!(clipboard.text.as_deref(), Some("hello"));
    clipboard.text = Some("keep".to_string());

    // Chat: Alt rejects copy; Shift rejects cut.
    let mut chat_world = world_with_local_player_id(9);
    let mut chat_input = ClientInputState::new(true);
    open_chat_entry(
        &mut chat_input,
        &mut chat_world,
        &mut counters,
        &commands,
        ChatMethod::Message,
        false,
    );
    chat_input.replace_chat_entry_text("hello".to_string());
    set_text_edit_shortcut_down(&mut chat_input, true);
    chat_input.pressed_keys.insert(KeyCode::AltLeft);
    handle_chat_entry_key(
        &mut chat_input,
        &mut counters,
        &mut chat_world,
        &commands,
        KeyCode::KeyC,
        true,
        Some(&mut clipboard),
        &FallbackTextMeasurer,
    );
    assert_eq!(clipboard.text.as_deref(), Some("keep"));
    assert_eq!(chat_input.screen.chat_entry().unwrap().edit.text(), "hello");
    chat_input.pressed_keys.remove(&KeyCode::AltLeft);
    chat_input.shift_left_down = true;
    handle_chat_entry_key(
        &mut chat_input,
        &mut counters,
        &mut chat_world,
        &commands,
        KeyCode::KeyX,
        true,
        Some(&mut clipboard),
        &FallbackTextMeasurer,
    );
    assert_eq!(chat_input.screen.chat_entry().unwrap().edit.text(), "hello");
    assert_eq!(clipboard.text.as_deref(), Some("keep"));

    // Debug-options search: Shift rejects copy; Alt rejects cut.
    let mut debug_input = ClientInputState::new(true);
    debug_input.open_debug_options_screen(&mut Default::default(), &None);
    assert!(debug_input.handle_debug_options_screen_text_input("chunk stats"));
    let mut debug_clipboard = MockDebugClipboard::with_text("keep");
    assert!(debug_input.handle_debug_options_screen_key(
        PhysicalKey::Code(TEXT_EDIT_SHORTCUT_KEY),
        ElementState::Pressed,
    ));
    assert!(debug_input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    ));
    assert!(debug_input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Pressed,
        Some(&mut debug_clipboard),
    ));
    assert_eq!(debug_clipboard.text.as_deref(), Some("keep"));
    let screen = debug_input.screen.debug_options().unwrap();
    assert_eq!(screen.search.text(), "chunk stats");
    assert_eq!(screen.search.cursor(), 11);
    assert_eq!(screen.search.selection(), 11);
    assert!(debug_input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::AltLeft),
        ElementState::Pressed,
    ));
    assert!(debug_input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyX),
        ElementState::Pressed,
        Some(&mut debug_clipboard),
    ));
    let screen = debug_input.screen.debug_options().unwrap();
    assert_eq!(screen.search.text(), "chunk stats");
    assert_eq!(debug_clipboard.text.as_deref(), Some("keep"));
}

#[test]
fn absent_or_failing_clipboard_is_a_safe_noop_for_edit_shortcuts() {
    let (tx, _rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut counters = NetCounters::default();

    // Anvil paste with no clipboard object leaves the rename untouched.
    let mut anvil_input = ClientInputState::container_for_test();
    let mut anvil_world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));
    handle_text_input(
        &mut anvil_input,
        &mut counters,
        &mut anvil_world,
        &commands,
        "Pick",
    );
    press_key(
        &mut anvil_input,
        &mut counters,
        &mut anvil_world,
        &commands,
        KeyCode::ControlLeft,
    );
    press_key(
        &mut anvil_input,
        &mut counters,
        &mut anvil_world,
        &commands,
        KeyCode::KeyV,
    );
    assert_eq!(anvil_input.anvil_rename_text(), "Pick");
    assert_eq!(counters.rename_item_commands_queued, 1);

    // Cut with no clipboard object or a failed write preserves the selection.
    anvil_input.container_mut().anvil_rename.set_cursor(4, None);
    anvil_input
        .container_mut()
        .anvil_rename
        .set_selection(0, None);
    press_key(
        &mut anvil_input,
        &mut counters,
        &mut anvil_world,
        &commands,
        KeyCode::KeyX,
    );
    assert_eq!(anvil_input.anvil_rename_text(), "Pick");
    assert_eq!(counters.rename_item_commands_queued, 1);
    let mut rejecting = MockDebugClipboard::with_text("keep");
    rejecting.accepts_text = false;
    press_key_with_clipboard(
        &mut anvil_input,
        &mut counters,
        &mut anvil_world,
        &commands,
        KeyCode::KeyX,
        &mut rejecting,
    );
    assert_eq!(anvil_input.anvil_rename_text(), "Pick");
    assert_eq!(rejecting.text.as_deref(), Some("keep"));
    assert_eq!(counters.rename_item_commands_queued, 1);

    // Recipe-book search paste with an unreadable clipboard leaves state.
    let mut recipe_input = ClientInputState::container_for_test();
    recipe_input.container_mut().recipe_book_search_focused = true;
    let mut recipe_world = open_recipe_book_crafting_table_world();
    handle_text_input(
        &mut recipe_input,
        &mut counters,
        &mut recipe_world,
        &commands,
        "stone",
    );
    press_key(
        &mut recipe_input,
        &mut counters,
        &mut recipe_world,
        &commands,
        KeyCode::ControlLeft,
    );
    let mut unreadable = MockDebugClipboard::with_text("stale");
    unreadable.text = None;
    press_key_with_clipboard(
        &mut recipe_input,
        &mut counters,
        &mut recipe_world,
        &commands,
        KeyCode::KeyV,
        &mut unreadable,
    );
    let state = recipe_input.recipe_book_search_hud_state();
    assert_eq!(state.edit.text(), "stone");
    assert_eq!(state.edit.cursor(), 5);
    assert_eq!(state.edit.selection(), 5);

    // Creative search paste with an unreadable clipboard leaves search text.
    let mut creative_input = ClientInputState::container_for_test();
    let mut creative_world = WorldStore::new();
    creative_world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: Vec::new(),
    });
    set_player_abilities(&mut creative_world, false, true);
    assert!(creative_world
        .open_local_inventory_with_operator_tab(false)
        .unwrap());
    creative_world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec!["minecraft:vanilla".to_string()],
    });
    creative_input.apply_screen_requests(&mut creative_world, &mut NetCounters::default(), &None);
    press_key(
        &mut creative_input,
        &mut counters,
        &mut creative_world,
        &commands,
        KeyCode::KeyT,
    );
    handle_text_input(
        &mut creative_input,
        &mut counters,
        &mut creative_world,
        &commands,
        "stone",
    );
    press_key(
        &mut creative_input,
        &mut counters,
        &mut creative_world,
        &commands,
        KeyCode::ControlLeft,
    );
    press_key_with_clipboard(
        &mut creative_input,
        &mut counters,
        &mut creative_world,
        &commands,
        KeyCode::KeyV,
        &mut unreadable,
    );
    assert_eq!(
        creative_input
            .creative_inventory_hud_state(&creative_world)
            .unwrap()
            .search
            .text(),
        "stone"
    );

    // Chat copy with no clipboard object is a safe no-op.
    let mut chat_world = world_with_local_player_id(9);
    let mut chat_input = ClientInputState::container_for_test();
    open_chat_entry(
        &mut chat_input,
        &mut chat_world,
        &mut counters,
        &commands,
        ChatMethod::Message,
        false,
    );
    chat_input.replace_chat_entry_text("hello".to_string());
    chat_input.control_left_down = true;
    handle_chat_entry_key(
        &mut chat_input,
        &mut counters,
        &mut chat_world,
        &commands,
        KeyCode::KeyC,
        true,
        None,
        &FallbackTextMeasurer,
    );
    assert_eq!(chat_input.screen.chat_entry().unwrap().edit.text(), "hello");
}

#[test]
fn the_narrator_hotkey_cycles_the_status_and_saves_the_enum_id() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let net_commands = None;
    let mut world = world_with_debug_player(false);
    input.set_narrator_hotkey_enabled(true);
    input.set_narrator_available(true);

    press_key(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        KeyCode::ControlLeft,
    );
    press_key(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        KeyCode::KeyB,
    );

    assert_eq!(
        input.take_narrator_status_requests(),
        vec![crate::input::NarratorStatusRequest {
            status: crate::narrator_runtime::NarratorStatus::All,
            schedule_narration: true,
        }]
    );
    assert_eq!(
        input.take_debug_client_option_writes(),
        vec![crate::options::ClientOptionWrite {
            key: "narrator",
            value: crate::options::ClientOptionValue::EnumId(1),
        }]
    );

    for expected in [
        crate::narrator_runtime::NarratorStatus::Chat,
        crate::narrator_runtime::NarratorStatus::System,
        crate::narrator_runtime::NarratorStatus::Off,
    ] {
        press_key(
            &mut input,
            &mut counters,
            &mut world,
            &net_commands,
            KeyCode::KeyB,
        );
        assert_eq!(
            input.take_narrator_status_requests(),
            vec![crate::input::NarratorStatusRequest {
                status: expected,
                schedule_narration: false,
            }]
        );
        input.take_debug_client_option_writes();
    }
}

#[test]
fn the_narrator_hotkey_stays_out_of_text_entry_and_unavailable_narrators() {
    let mut counters = NetCounters::default();
    let net_commands = None;

    let mut without_narrator = ClientInputState::new(true);
    without_narrator.set_narrator_hotkey_enabled(true);
    let mut world = world_with_debug_player(false);
    press_key(
        &mut without_narrator,
        &mut counters,
        &mut world,
        &net_commands,
        KeyCode::ControlLeft,
    );
    press_key(
        &mut without_narrator,
        &mut counters,
        &mut world,
        &net_commands,
        KeyCode::KeyB,
    );
    assert!(without_narrator.take_narrator_status_requests().is_empty());

    let mut hotkey_off = ClientInputState::new(true);
    hotkey_off.set_narrator_available(true);
    press_key(
        &mut hotkey_off,
        &mut counters,
        &mut world,
        &net_commands,
        KeyCode::ControlLeft,
    );
    press_key(
        &mut hotkey_off,
        &mut counters,
        &mut world,
        &net_commands,
        KeyCode::KeyB,
    );
    assert!(hotkey_off.take_narrator_status_requests().is_empty());

    let mut typing = ClientInputState::new(true);
    typing.set_narrator_hotkey_enabled(true);
    typing.set_narrator_available(true);
    open_chat_entry(
        &mut typing,
        &mut world,
        &mut counters,
        &net_commands,
        ChatMethod::Message,
        false,
    );
    press_key(
        &mut typing,
        &mut counters,
        &mut world,
        &net_commands,
        KeyCode::ControlLeft,
    );
    press_key(
        &mut typing,
        &mut counters,
        &mut world,
        &net_commands,
        KeyCode::KeyB,
    );
    assert!(typing.take_narrator_status_requests().is_empty());

    let mut without_control = ClientInputState::new(true);
    without_control.set_narrator_hotkey_enabled(true);
    without_control.set_narrator_available(true);
    press_key(
        &mut without_control,
        &mut counters,
        &mut world,
        &net_commands,
        KeyCode::KeyB,
    );
    assert!(without_control.take_narrator_status_requests().is_empty());
}

#[test]
fn the_fullscreen_key_toggles_the_window_and_saves_the_option() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let net_commands = None;
    let mut world = world_with_debug_player(false);

    press_key(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        KeyCode::F11,
    );
    assert_eq!(input.take_fullscreen_requests(), vec![true]);
    assert_eq!(
        input.take_debug_client_option_writes(),
        vec![crate::options::ClientOptionWrite {
            key: "fullscreen",
            value: crate::options::ClientOptionValue::Boolean(true),
        }]
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        PhysicalKey::Code(KeyCode::F11),
        ElementState::Released,
    );
    assert!(input.take_fullscreen_requests().is_empty());

    press_key(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        KeyCode::F11,
    );
    assert_eq!(input.take_fullscreen_requests(), vec![false]);
    assert_eq!(
        input.take_debug_client_option_writes(),
        vec![crate::options::ClientOptionWrite {
            key: "fullscreen",
            value: crate::options::ClientOptionValue::Boolean(false),
        }]
    );
}

#[test]
fn text_input_is_inactive_during_gameplay_and_active_once_chat_takes_focus() {
    // `TextInputManager` keeps the IME off
    // unless a text field has focus.  With an input method installed this is
    // load-bearing rather than tidy: an IME grabs every letter key it is
    // offered, so an always-on client cannot even open chat with `T`.
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let net_commands = None;
    let mut world = world_with_debug_player(false);

    assert!(!input.text_input_is_active(&world));

    press_key(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        KeyCode::KeyT,
    );
    assert!(input.chat_entry_is_active());
    assert!(input.text_input_is_active(&world));

    press_key(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        KeyCode::Escape,
    );
    assert!(!input.chat_entry_is_active());
    assert!(!input.text_input_is_active(&world));
}

#[test]
fn text_input_needs_the_window_focused_like_the_vanilla_tick_does() {
    // `TextInputManager.tick` only revokes the IME while the window is
    // focused, and an unfocused window has no keyboard to compose with.
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let net_commands = None;
    let mut world = world_with_debug_player(false);

    press_key(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        KeyCode::KeyT,
    );
    assert!(input.text_input_is_active(&world));

    input.focused = false;
    assert!(!input.text_input_is_active(&world));
}

#[test]
fn closing_recipe_book_screen_revokes_stale_search_text_input_focus() {
    // Closing the menu disables IME; reopening resumes the saved search.
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let commands = None;
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    input.container_mut().recipe_book_search_focused = true;
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "stone");

    assert!(input.text_input_is_active(&world));
    assert!(input.recipe_book_search_receipt(&world).0);
    press_key(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::Escape,
    );
    assert!(!world.local_inventory_is_open());
    assert!(!input.text_input_is_active(&world));
    assert!(!input.recipe_book_search_receipt(&world).0);

    press_key(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        KeyCode::KeyE,
    );
    // Vanilla `RecipeBookComponent.initVisuals` (`RecipeBookComponent.java:115-134`)
    // persists the search box text across screen closes while clearing focus.
    assert!(!input.text_input_is_active(&world));
    assert_eq!(input.recipe_book_search_hud_state().edit.text(), "stone");
    input.container_mut().recipe_book_search_focused = true;
    assert!(input.text_input_is_active(&world));
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "brick");
    assert_eq!(
        input.recipe_book_search_hud_state().edit.text(),
        "stonebrick"
    );
}

#[test]
fn the_global_press_keys_ignore_screen_state_and_key_releases() {
    // `KeyboardHandler.keyPress` answers these
    // two on any press, before a screen sees the event, and returns without
    // recording the key.  The owner therefore consults neither a screen nor
    // anything a screen could set.
    let mut input = ClientInputState::new(true);

    assert!(input.handle_global_key_press(PhysicalKey::Code(KeyCode::F11), ElementState::Pressed));
    assert_eq!(input.take_fullscreen_requests(), vec![true]);
    assert!(input.handle_global_key_press(PhysicalKey::Code(KeyCode::F2), ElementState::Pressed));
    assert_eq!(input.take_screenshot_requests(), 1);

    // OS key repeats while the key is still held are swallowed without
    // triggering duplicate toggles/screenshots.
    assert!(input.handle_global_key_press(PhysicalKey::Code(KeyCode::F2), ElementState::Pressed));
    assert_eq!(input.take_screenshot_requests(), 0);

    // A release is not a press and an unbound key is not one of the two.
    // Focus is deliberately not rechecked: Vanilla's callback validates only
    // its window handle, while the platform owns ordinary focus filtering.
    assert!(!input.handle_global_key_press(PhysicalKey::Code(KeyCode::F11), ElementState::Released));
    assert!(!input.handle_global_key_press(PhysicalKey::Code(KeyCode::F2), ElementState::Released));
    assert!(!input.handle_global_key_press(PhysicalKey::Code(KeyCode::KeyW), ElementState::Pressed));
    input.focused = false;
    assert!(input.handle_global_key_press(PhysicalKey::Code(KeyCode::F2), ElementState::Pressed));
    assert_eq!(input.take_screenshot_requests(), 1);
}

#[test]
fn the_screenshot_key_requests_one_grab_per_press_and_announces_the_saved_name() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let net_commands = None;
    let mut world = world_with_debug_player(false);

    press_key(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        KeyCode::F2,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        PhysicalKey::Code(KeyCode::F2),
        ElementState::Released,
    );
    press_key(
        &mut input,
        &mut counters,
        &mut world,
        &net_commands,
        KeyCode::F2,
    );
    assert_eq!(input.take_screenshot_requests(), 2);
    assert_eq!(input.take_screenshot_requests(), 0);

    ClientInputState::push_screenshot_saved_message(
        Some(&mut world),
        "2026-08-05_11.22.33.png",
        None,
    );
    let messages = &world.client_chat().messages;
    let saved = messages.last().expect("screenshot chat message");
    assert_eq!(
        saved
            .styled_content
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        "Saved screenshot as 2026-08-05_11.22.33.png"
    );
    assert_eq!(
        saved.styled_content.last().map(|run| run.style.underlined),
        Some(Some(true))
    );

    ClientInputState::push_screenshot_saved_message(Some(&mut world), "", Some("disk full"));
    let messages = &world.client_chat().messages;
    assert_eq!(
        messages
            .last()
            .expect("screenshot failure message")
            .styled_content
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        "Couldn't save screenshot: disk full"
    );
}

#[test]
fn the_mouse_turn_delta_follows_the_vanilla_sensitivity_curve() {
    let world = world_with_debug_player(false);
    let mut input = ClientInputState::new(true);
    input.mouse_delta_x = 100.0;
    input.mouse_delta_y = -50.0;

    // The 0.5 default gives (0.5 * 0.6 + 0.2)^3 * 8 == 1.0.
    assert_eq!(input.mouse_turn_delta(&world), (100.0, -50.0));

    input.set_mouse_sensitivity(1.0);
    let (x, y) = input.mouse_turn_delta(&world);
    assert!((x - 409.6).abs() < 1e-9, "{x}");
    assert!((y + 204.8).abs() < 1e-9, "{y}");

    input.set_mouse_sensitivity(0.0);
    let (x, y) = input.mouse_turn_delta(&world);
    assert!((x - 6.4).abs() < 1e-9, "{x}");
    assert!((y + 3.2).abs() < 1e-9, "{y}");

    input.set_mouse_sensitivity(0.5);
    input.set_invert_mouse(true, true);
    assert_eq!(input.mouse_turn_delta(&world), (-100.0, 50.0));
}

#[test]
fn smooth_camera_turn_smooths_both_axes_and_freezes_while_ungrabbed() {
    let world = world_with_debug_player(false);
    let mut input = ClientInputState::new(true);
    input.set_mouse_look_live(true);
    input.smooth_camera = true;
    // Default 0.5 sensitivity gives sens == 1.0, so the smooth chain sees the
    // raw deltas and `mousea` directly.
    input.mouse_delta_x = 8.0;
    input.mouse_delta_y = 8.0;
    let t0 = Instant::now();
    // Vanilla's first frame has a zero wall-clock gap: the accumulator stores
    // the target but returns no turn yet.
    input.prepare_mouse_turn(&world, t0);
    assert_eq!(input.mouse_turn_delta(&world), (0.0, 0.0));

    input.mouse_delta_x = 0.0;
    input.mouse_delta_y = 0.0;
    input.prepare_mouse_turn(&world, t0 + Duration::from_millis(500));
    assert_eq!(input.mouse_turn_delta(&world), (3.0, 3.0));

    // An ungrabbed frame neither advances nor resets the accumulators
    // (vanilla only calls `turnPlayer` while the mouse is grabbed).
    input.set_mouse_look_live(false);
    input.prepare_mouse_turn(&world, t0 + Duration::from_millis(1000));
    assert_eq!(input.mouse_turn_delta(&world), (0.0, 0.0));

    input.set_mouse_look_live(true);
    input.prepare_mouse_turn(&world, t0 + Duration::from_millis(1500));
    assert_eq!(input.mouse_turn_delta(&world), (2.5, 2.5));

    // The non-smooth branch resets the accumulators, so re-enabling starts
    // from an empty chain.
    input.smooth_camera = false;
    input.prepare_mouse_turn(&world, t0 + Duration::from_millis(2000));
    assert_eq!(input.mouse_turn_delta(&world), (0.0, 0.0));
    input.smooth_camera = true;
    input.prepare_mouse_turn(&world, t0 + Duration::from_millis(2500));
    assert_eq!(input.mouse_turn_delta(&world), (0.0, 0.0));
}

#[test]
fn the_smooth_camera_key_toggles_the_transient_option() {
    let (tx, _rx) = mpsc::channel(6);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);

    // Vanilla ships `key.smoothCamera` unbound; binding it is the only way to
    // reach the toggle.
    let mut options = crate::options::ClientOptions::defaults(Default::default());
    options.set_dynamic_raw_for_test("key_key.smoothCamera", "key.keyboard.o");
    input.set_key_bindings(crate::options::key_bindings::KeyBindings::from_options(
        &options,
    ));

    for expected in [true, false] {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::KeyO),
            ElementState::Pressed,
        );
        assert_eq!(input.smooth_camera, expected);
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::KeyO),
            ElementState::Released,
        );
    }
}

#[test]
fn the_scroll_delta_follows_the_discrete_and_wheel_sensitivity_options() {
    let mut input = ClientInputState::new(true);
    let raw = MouseScrollDelta::LineDelta(-2.5, 4.0);
    let raw_pixels = MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(-20.0, 40.0));

    assert_eq!(
        input.scale_scroll_delta(raw),
        MouseScrollDelta::LineDelta(-2.5, 4.0)
    );
    assert_eq!(
        input.scale_scroll_delta(raw_pixels),
        MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(-20.0, 40.0))
    );

    input.set_mouse_scroll_options(true, 1.0);
    assert_eq!(
        input.scale_scroll_delta(raw),
        MouseScrollDelta::LineDelta(-1.0, 1.0)
    );
    assert_eq!(
        input.scale_scroll_delta(raw_pixels),
        MouseScrollDelta::LineDelta(-1.0, 1.0)
    );

    input.set_mouse_scroll_options(false, 2.5);
    assert_eq!(
        input.scale_scroll_delta(raw),
        MouseScrollDelta::LineDelta(-6.25, 10.0)
    );
    assert_eq!(
        input.scale_scroll_delta(raw_pixels),
        MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(-50.0, 100.0))
    );

    input.set_mouse_scroll_options(true, 0.25);
    assert_eq!(
        input.scale_scroll_delta(MouseScrollDelta::LineDelta(0.0, -8.0)),
        MouseScrollDelta::LineDelta(0.0, -0.25)
    );
}

#[test]
fn command_block_editor_activates_ime_text_input() {
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut input = ClientInputState::new(true);
    assert!(!input.text_input_is_active(&world));

    input.open_command_block_editor(
        bbb_client_world::CommandBlockEditorState {
            pos: BlockPos { x: 1, y: 64, z: 2 },
            data: bbb_client_world::CommandBlockEntityDataState::default(),
            mode: CommandBlockMode::Redstone,
            conditional: false,
        },
        &mut world,
        &mut counters,
        &None,
    );
    assert!(input.text_input_is_active(&world));
}
