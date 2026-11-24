use super::*;
use crate::input::Screen;

fn project_stats_screen(
    stats: &mut StatsScreen,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> HudStatsScreen {
    stats_screen::sync_stats_screen_interaction(stats, world, item_runtime, surface_size);
    hud_stats_screen(
        stats,
        world,
        item_runtime,
        terrain_textures,
        keybind_context,
        partial_tick,
        surface_size,
    )
}

#[test]
fn hud_stats_screen_waits_for_response_then_projects_general_item_and_mob_rows() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    let item_runtime = NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
    let stone_block_id = world
        .registry_holder_id_by_resource_id("minecraft:block", "minecraft:stone")
        .unwrap();

    let terrain_textures = TerrainTextureState::default();
    input.set_screen_for_test(Screen::Stats(StatsScreen::default()));
    assert!(input.handle_stats_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(70.0, 223.0)),
        surface,
    ));

    let screen = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    assert_eq!(screen.title, "Stats");
    assert_eq!(screen.loading_text, "Downloading statistics...");
    assert!(screen.is_loading);
    assert!(screen.rows.is_empty());
    assert_eq!(screen.scroll_amount, 0);
    assert_eq!(screen.max_scroll_amount, 0);
    assert_eq!(screen.visible_rows, 0);
    assert!(screen.done_hovered);

    world.apply_award_stats(AwardStats {
        stats: vec![
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_CUSTOM_ID,
                value_id: 6,
                amount: 12_345,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ENTITY_KILLED_ID,
                value_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                amount: 2,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_BLOCK_MINED_ID,
                value_id: stone_block_id,
                amount: 9,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID,
                value_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                amount: 1,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
                value_id: bbb_protocol::item_types::vanilla_item_type_for_resource_id(
                    "minecraft:snowball",
                )
                .unwrap()
                .id,
                amount: 7,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
                value_id: bbb_protocol::item_types::vanilla_item_type_for_resource_id(
                    "minecraft:apple",
                )
                .unwrap()
                .id,
                amount: 2,
            },
        ],
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    let screen = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        winit::dpi::PhysicalSize::new(320, 1_200),
    );
    assert!(!screen.is_loading);
    assert_eq!(screen.total_rows, 77);
    assert_eq!(screen.visible_rows, 77);
    assert!(screen
        .rows
        .iter()
        .any(|row| { row.label == "minecraft:walk_one_cm" && row.value == "123.45 m" }));

    let mut counters = NetCounters::default();
    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(160.0, 12.0)),
        surface,
    ));
    let item_screen = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    assert_eq!(item_screen.active_tab, 1);
    assert!(item_screen.items_tab_enabled);
    assert_eq!(item_screen.row_height, 22);
    assert_eq!(item_screen.item_rows.len(), 3);
    assert!(item_screen.item_rows.iter().any(|row| {
        row.resource_id == "minecraft:snowball"
            && row.values[0].is_none()
            && row.values[1].as_deref() == Some("0")
            && row.values[2].as_deref() == Some("7")
    }));
    assert!(item_screen.item_rows.iter().any(|row| {
        row.resource_id == "minecraft:stone"
            && row.values[0].as_deref() == Some("9")
            && row.values[1].as_deref() == Some("0")
            && row.icon.is_some()
    }));
    assert!(input.handle_stats_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(25.0, 50.0)),
        surface,
    ));
    let hovered_item_screen = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );

    assert_eq!(
        hovered_item_screen
            .tooltip
            .as_ref()
            .map(|tooltip| tooltip.text.as_str()),
        Some(hovered_item_screen.item_rows[0].label.as_str())
    );

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(160.0, 28.0)),
        surface,
    ));
    let item_screen = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    assert_eq!(item_screen.item_sort_column, Some(2));
    assert_eq!(item_screen.item_sort_order, -1);
    assert_eq!(item_screen.item_rows[0].resource_id, "minecraft:snowball");
    assert_eq!(
        item_screen
            .tooltip
            .as_ref()
            .map(|tooltip| tooltip.text.as_str()),
        Some("Crafted")
    );

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(160.0, 28.0)),
        surface,
    ));
    let item_screen = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    assert_eq!(item_screen.item_sort_order, 1);
    assert_eq!(item_screen.item_rows[0].resource_id, "minecraft:stone");

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(160.0, 28.0)),
        surface,
    ));
    let item_screen = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    assert_eq!(item_screen.item_sort_column, None);
    assert_eq!(item_screen.item_sort_order, 0);
    assert_eq!(item_screen.item_rows[0].resource_id, "minecraft:stone");

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(253.0, 12.0)),
        surface,
    ));
    let screen = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    assert_eq!(screen.active_tab, 2);
    assert!(screen.mobs_tab_enabled);
    assert_eq!(screen.row_height, 36);
    assert_eq!(screen.rows.len(), 1);
    assert_eq!(screen.rows[0].label, "minecraft:zombie");
    assert_eq!(screen.rows[0].value, "You killed 2 minecraft:zombie");
    assert_eq!(
        screen.rows[0].secondary_value.as_deref(),
        Some("minecraft:zombie killed you 1 time(s)")
    );
}

#[test]
fn stats_screen_narration_matches_tabs_general_items_and_mobs_widgets() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    let item_runtime = NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
    let terrain_textures = TerrainTextureState::default();
    input.set_screen_for_test(Screen::Stats(StatsScreen::default()));

    let loading = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    assert_eq!(
        stats_screen_narration_frame(
            input.stats_screen().unwrap(),
            &loading,
            Some(&item_runtime),
            surface
        )
        .all_text_for_test(),
        "Stats. Use mouse cursor or Tab button to select element"
    );

    world.apply_award_stats(AwardStats {
        stats: vec![
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_CUSTOM_ID,
                value_id: 6,
                amount: 12_345,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
                value_id: bbb_protocol::item_types::vanilla_item_type_for_resource_id(
                    "minecraft:apple",
                )
                .unwrap()
                .id,
                amount: 2,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ENTITY_KILLED_ID,
                value_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                amount: 2,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID,
                value_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                amount: 1,
            },
        ],
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    let general = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    assert_eq!(
        stats_screen_narration_frame(input.stats_screen().unwrap(), &general, Some(&item_runtime), surface)
            .all_text_for_test(),
        "Stats. General tab. Screen element 1 out of 3. Selected tab 1 out of 3. Press Tab to navigate to the next element. Press Ctrl and Tab to switch between tabs"
    );

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(160.0, 28.0)),
        surface,
    ));
    let general = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    let row = &general.rows[0];
    assert_eq!(
        stats_screen_narration_frame(input.stats_screen().unwrap(), &general, Some(&item_runtime), surface)
            .all_text_for_test(),
        format!(
            "Stats. Selected: {} {}. Screen element 2 out of 3. Selected list row 1 out of {}. Press Tab to navigate to the next element. Press the up and down buttons to move to another entry",
            row.label, row.value, general.total_rows
        )
    );

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(160.0, 12.0)),
        surface,
    ));
    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(160.0, 28.0)),
        surface,
    ));
    let items = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    assert_eq!(items.active_tab, 1);
    assert_eq!(
        stats_screen_narration_frame(input.stats_screen().unwrap(), &items, Some(&item_runtime), surface)
            .all_text_for_test(),
        format!(
            "Stats. Crafted button. Screen element 2 out of 3. Selected list row 1 out of {}. Selected row element 3 out of 6. Crafted. Press Tab to navigate to the next element. Press Enter to activate",
            items.total_rows + 1
        )
    );

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(25.0, 50.0)),
        surface,
    ));
    let items = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    assert_eq!(
        stats_screen_narration_frame(input.stats_screen().unwrap(), &items, Some(&item_runtime), surface)
            .all_text_for_test(),
        format!(
            "Stats. Item: {}. Screen element 2 out of 3. Selected list row 2 out of {}. Press Tab to navigate to the next element",
            items.item_rows[0].label,
            items.total_rows + 1
        )
    );

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(253.0, 12.0)),
        surface,
    ));
    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(160.0, 28.0)),
        surface,
    ));
    let mobs = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    let mob = &mobs.rows[0];
    assert_eq!(
        stats_screen_narration_frame(input.stats_screen().unwrap(), &mobs, Some(&item_runtime), surface)
            .all_text_for_test(),
        format!(
            "Stats. Selected: {}. {}. Screen element 2 out of 3. Press Tab to navigate to the next element. Press the up and down buttons to move to another entry",
            mob.value,
            mob.secondary_value.as_deref().unwrap()
        )
    );

    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    let done = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    assert_eq!(
        stats_screen_narration_frame(input.stats_screen().unwrap(), &done, Some(&item_runtime), surface)
            .all_text_for_test(),
        "Stats. Done button. Screen element 3 out of 3. Press Tab to navigate to the next element. Press Enter to activate"
    );
}

#[test]
fn hud_stats_screen_reports_empty_tab_tooltip_after_response() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.set_screen_for_test(Screen::Stats(StatsScreen::default()));
    world.apply_award_stats(AwardStats { stats: Vec::new() });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    assert!(input.handle_stats_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(160.0, 12.0)),
        surface,
    ));

    let screen = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        None,
        &TerrainTextureState::default(),
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );

    assert!(!screen.items_tab_enabled);
    assert_eq!(
        screen.tooltip.as_ref().map(|tooltip| tooltip.text.as_str()),
        Some("No statistics found")
    );
    assert_eq!(
        stats_screen_narration_frame(input.stats_screen().unwrap(), &screen, None, surface)
            .all_text_for_test(),
        "Stats. Items tab. Screen element 1 out of 3. Selected tab 2 out of 3. \
         No statistics found. Press Tab to navigate to the next element. \
         Press Ctrl and Tab to switch between tabs"
    );
}

#[test]
fn stats_screen_visible_item_rows_keep_sort_values_and_scrolled_hover() {
    let mut world = WorldStore::new();
    let item_runtime = NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
    let terrain_textures = TerrainTextureState::default();
    let mut expected = bbb_protocol::item_types::VANILLA_ITEM_TYPES
        .iter()
        .filter(|entry| entry.resource_id != "minecraft:air")
        .take(24)
        .enumerate()
        .map(|(index, entry)| (entry, (index % 7 + 1) as i32))
        .collect::<Vec<_>>();
    world.apply_award_stats(AwardStats {
        stats: expected
            .iter()
            .map(|(entry, amount)| StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
                value_id: entry.id,
                amount: *amount,
            })
            .collect(),
    });
    expected.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| right.0.id.cmp(&left.0.id))
    });
    let mut stats = StatsScreen::default();
    stats.loading = false;
    stats.tab = 1;
    stats.activate_item_column(2);
    let all = project_stats_screen(
        &mut stats,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        winit::dpi::PhysicalSize::new(320, 1_200),
    );
    assert_eq!(
        all.item_rows
            .iter()
            .map(|row| row.resource_id.as_str())
            .collect::<Vec<_>>(),
        expected
            .iter()
            .map(|(entry, _)| entry.resource_id)
            .collect::<Vec<_>>()
    );
    for (row, (_, amount)) in all.item_rows.iter().zip(&expected) {
        assert_eq!(row.values[2].as_deref(), Some(amount.to_string().as_str()));
    }

    let surface = winit::dpi::PhysicalSize::new(320, 160);
    stats.focus = StatsScreenFocus::None;
    stats.scroll_amount = 66.0;
    stats.cursor_position = Some((25, 27));
    stats_screen::sync_stats_screen_interaction(&mut stats, &world, Some(&item_runtime), surface);
    let interaction = stats.clone();
    let visible = hud_stats_screen(
        &stats,
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    assert_eq!(stats, interaction);
    assert_eq!(visible.scroll_row, 1);
    assert_eq!(
        visible.item_rows,
        all.item_rows[1..1 + visible.visible_rows]
    );
    assert_eq!(
        visible
            .tooltip
            .as_ref()
            .map(|tooltip| tooltip.text.as_str()),
        Some(all.item_rows[2].label.as_str())
    );
    assert!(
        stats_screen_narration_frame(&stats, &visible, Some(&item_runtime), surface)
            .all_text_for_test()
            .contains(&all.item_rows[2].label)
    );
}

#[test]
fn command_suggestion_narration_localizes_vanilla_payload_shapes() {
    assert_eq!(
        localized_command_suggestion_narration(
            None,
            &CommandSuggestionNarrationState {
                position: 2,
                total: 4,
                text: "stone".into(),
                tooltip: None,
                tooltip_translation_key: None,
            },
        ),
        "Selected suggestion 2 out of 4: stone"
    );
    assert_eq!(
        localized_command_suggestion_narration(
            None,
            &CommandSuggestionNarrationState {
                position: 3,
                total: 5,
                text: "stick".into(),
                tooltip: Some("minecraft:stick".to_string()),
                tooltip_translation_key: None,
            },
        ),
        "Selected suggestion 3 out of 5: stick (minecraft:stick)"
    );
    assert_eq!(
        format_localized_string_arguments("Selected %s (%s), 100%%", &["value %s", "tip"]),
        "Selected value %s (tip), 100%"
    );

    let root = unique_runtime_temp_dir("command-suggestion-language");
    write_runtime_bow_item_assets(&root);
    write_runtime_json(
        &runtime_assets_dir(&root).join("lang").join("en_us.json"),
        r#"{"entity.minecraft.zombie":"Localized Zombie"}"#,
    );
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    assert_eq!(
        localized_command_suggestion_narration(
            Some(&item_runtime),
            &CommandSuggestionNarrationState {
                position: 1,
                total: 2,
                text: "minecraft:zombie".into(),
                tooltip: Some("minecraft:zombie".to_string()),
                tooltip_translation_key: Some("entity.minecraft.zombie".to_string()),
            },
        ),
        "Selected suggestion 1 out of 2: minecraft:zombie (Localized Zombie)"
    );
    let tooltip_runs = hud_command_suggestion_tooltip_runs(
        CommandSuggestionHudTooltip {
            text: "minecraft:zombie".into(),
            translation_key: Some("entity.minecraft.zombie".to_string()),
            runs: Vec::new(),
            x: 12,
            y: 24,
        },
        Some(&item_runtime),
    );
    assert_eq!(tooltip_runs.len(), 1);
    assert_eq!(tooltip_runs[0].text, "Localized Zombie");
    assert_eq!(tooltip_runs[0].style, HudTextStyle::default());
    assert_eq!(tooltip_runs[0].color, None);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn command_suggestion_narration_speaks_once_per_state_change_and_resets() {
    #[derive(Default)]
    struct RecordingNarrator(Vec<String>);

    impl SystemNarrator for RecordingNarrator {
        fn update_status(&mut self, status: crate::narrator_runtime::NarratorStatus) {
            self.0.push(format!("status:{}", status.id()));
        }

        fn say_system_now(&mut self, message: &str) {
            self.0.push(message.to_string());
        }

        fn say_chat_queued(&mut self, message: &str) {
            self.0.push(message.to_string());
        }

        fn say_system_chat_queued(&mut self, message: &str) {
            self.0.push(message.to_string());
        }

        fn say_system_queued(&mut self, message: &str) {
            self.0.push(message.to_string());
        }
    }

    let narration = CommandSuggestionNarrationState {
        position: 1,
        total: 2,
        text: "stone".into(),
        tooltip: None,
        tooltip_translation_key: None,
    };
    let mut last = None;
    let mut narrator = RecordingNarrator::default();
    narrate_command_suggestion_change(Some(&narration), &mut last, None, Some(&mut narrator));
    narrate_command_suggestion_change(Some(&narration), &mut last, None, Some(&mut narrator));
    narrate_command_suggestion_change(None, &mut last, None, Some(&mut narrator));
    narrate_command_suggestion_change(Some(&narration), &mut last, None, Some(&mut narrator));

    assert_eq!(
        narrator.0,
        [
            "Selected suggestion 1 out of 2: stone",
            "Selected suggestion 1 out of 2: stone"
        ]
    );
}

#[test]
fn player_list_visibility_narration_speaks_only_on_opening_edges() {
    #[derive(Default)]
    struct RecordingNarrator(Vec<String>);

    impl SystemNarrator for RecordingNarrator {
        fn update_status(&mut self, status: crate::narrator_runtime::NarratorStatus) {
            self.0.push(format!("status:{}", status.id()));
        }

        fn say_system_now(&mut self, message: &str) {
            self.0.push(message.to_string());
        }

        fn say_chat_queued(&mut self, _message: &str) {}

        fn say_system_chat_queued(&mut self, _message: &str) {}

        fn say_system_queued(&mut self, _message: &str) {}
    }

    let mut visible = false;
    let mut narrator = RecordingNarrator::default();
    narrate_player_list_visibility_change(false, &mut visible, Some(&mut narrator), || {
        "not spoken".to_string()
    });
    narrate_player_list_visibility_change(true, &mut visible, Some(&mut narrator), || {
        "Online players: Ada".to_string()
    });
    narrate_player_list_visibility_change(true, &mut visible, Some(&mut narrator), || {
        "not repeated".to_string()
    });
    narrate_player_list_visibility_change(false, &mut visible, Some(&mut narrator), || {
        "not spoken".to_string()
    });
    narrate_player_list_visibility_change(true, &mut visible, Some(&mut narrator), || {
        "Online players: Grace".to_string()
    });
    assert_eq!(narrator.0, ["Online players: Ada", "Online players: Grace"]);

    let mut muted_edge = false;
    narrate_player_list_visibility_change(true, &mut muted_edge, None, || "muted".to_string());
    narrate_player_list_visibility_change(true, &mut muted_edge, Some(&mut narrator), || {
        "not late".to_string()
    });
    assert_eq!(narrator.0.len(), 2);
}

#[test]
fn stats_screen_pixel_projection_scrolls_item_header_as_first_entry() {
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    assert_eq!(stats_screen_viewport_height(surface, 14), 183);
    assert_eq!(stats_screen_scroller_height(161, 280, 119), 92);

    assert_eq!(stats_screen_first_data_row(0, 7, 14), 0);
    assert_eq!(stats_screen_first_data_row_offset(0, 0, 7, 14), -5);
    assert_eq!(stats_screen_projected_row_count(20, 161, -5, 14), 12);

    assert_eq!(stats_screen_first_data_row(1, 11, 22), 0);
    assert_eq!(stats_screen_first_data_row_offset(1, 0, 11, 22), 13);
    assert_eq!(stats_screen_first_data_row(1, 30, 22), 0);
    assert_eq!(stats_screen_first_data_row_offset(1, 0, 30, 22), -6);
    assert_eq!(stats_screen_first_data_row(1, 44, 22), 0);
    assert_eq!(stats_screen_first_data_row_offset(1, 0, 44, 22), -20);
    assert_eq!(stats_screen_first_data_row(1, 46, 22), 1);
    assert_eq!(stats_screen_first_data_row_offset(1, 1, 46, 22), 0);
}

#[test]
fn hud_stats_screen_clamps_pixel_scroll_to_projected_content() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    input.set_screen_for_test(Screen::Stats(StatsScreen::default()));
    world.apply_award_stats(AwardStats { stats: Vec::new() });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);

    let screen = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        None,
        &TerrainTextureState::default(),
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    assert_eq!(screen.total_rows, 77);
    assert_eq!(screen.max_scroll_amount, 899);
    assert_eq!(screen.scroller_height, 32);

    assert!(input
        .handle_stats_screen_mouse_wheel(winit::event::MouseScrollDelta::LineDelta(0.0, -1.0),));
    let screen = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        None,
        &TerrainTextureState::default(),
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    assert_eq!(screen.scroll_amount, 7);
    assert_eq!(screen.scroll_row, 0);
    assert_eq!(screen.visible_rows, 14);

    assert!(
        input.handle_stats_screen_mouse_wheel(winit::event::MouseScrollDelta::PixelDelta(
            winit::dpi::PhysicalPosition::new(0.0, -10_000.0,)
        ),)
    );
    let screen = project_stats_screen(
        input.stats_screen_mut().unwrap(),
        &world,
        None,
        &TerrainTextureState::default(),
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    assert_eq!(screen.scroll_amount, 899);
    assert_eq!(screen.scroll_row, 64);
    assert_eq!(screen.rows.len(), 13);
}

#[test]
fn low_disk_space_toast_uses_ten_second_visible_window_and_square_slides() {
    let start = std::time::Instant::now();
    let mut world = WorldStore::new();
    let mut state = HudRenderRuntime::default();
    assert_eq!(
        hud_system_toast(&mut state, &world, None, start),
        (None, None)
    );
    world.apply_low_disk_space_warning();
    assert!(hud_system_toast(&mut state, &world, None, start)
        .1
        .is_none());
    assert_eq!(
        hud_system_toast(&mut state, &world, None, start)
            .1
            .and_then(|sound| sound.sound.location)
            .as_deref(),
        Some("minecraft:ui.toast.in")
    );
    assert_eq!(
        hud_system_toast(&mut state, &world, None, start + Duration::from_millis(300))
            .0
            .map(|toast| toast.visible_portion),
        Some(0.25)
    );
    assert_eq!(
        hud_system_toast(&mut state, &world, None, start + Duration::from_millis(600))
            .0
            .map(|toast| toast.visible_portion),
        Some(1.0)
    );
    assert_eq!(
        hud_system_toast(
            &mut state,
            &world,
            None,
            start + Duration::from_millis(10_599)
        )
        .0
        .map(|toast| toast.visible_portion),
        Some(1.0)
    );
    assert_eq!(
        hud_system_toast(
            &mut state,
            &world,
            None,
            start + Duration::from_millis(10_600)
        )
        .1
        .and_then(|sound| sound.sound.location)
        .as_deref(),
        Some("minecraft:ui.toast.out")
    );
    assert_eq!(
        hud_system_toast(
            &mut state,
            &world,
            None,
            start + Duration::from_millis(10_900)
        )
        .0
        .map(|toast| toast.visible_portion),
        Some(0.75)
    );
    assert_eq!(
        hud_system_toast(
            &mut state,
            &world,
            None,
            start + Duration::from_millis(11_201)
        )
        .0,
        None
    );

    world.apply_low_disk_space_warning();
    assert!(hud_system_toast(
        &mut state,
        &world,
        None,
        start + Duration::from_millis(12_000)
    )
    .0
    .is_some());
    assert_eq!(
        hud_system_toast(
            &mut state,
            &world,
            None,
            start + Duration::from_millis(12_000)
        )
        .1
        .and_then(|sound| sound.sound.location)
        .as_deref(),
        Some("minecraft:ui.toast.in")
    );
}

#[test]
fn narrator_toggle_projects_enabled_and_disabled_system_toasts() {
    let start = std::time::Instant::now();
    let mut world = WorldStore::new();
    let mut state = HudRenderRuntime::default();

    world.push_narrator_toggle_toast(true, "Narrates Chat");
    let (toast, _) = hud_system_toast(&mut state, &world, None, start);
    assert_eq!(
        toast,
        Some(HudSystemToast {
            title: "Narrator Enabled".to_string(),
            message_lines: vec!["Narrates Chat".to_string()],
            width: 160,
            visible_portion: 0.0,
        })
    );

    world.push_narrator_toggle_toast(false, "Off");
    let (toast, _) = hud_system_toast(&mut state, &world, None, start);
    assert_eq!(
        toast,
        Some(HudSystemToast {
            title: "Narrator Disabled".to_string(),
            message_lines: Vec::new(),
            width: 160,
            visible_portion: 0.0,
        })
    );
    assert_eq!(world.narrator_toggle_toast_count(), 2);
}

#[test]
fn system_toast_title_relocalizes_while_message_lines_and_width_stay_frozen() {
    let first_root = unique_runtime_temp_dir("system-toast-title-first");
    let second_root = unique_runtime_temp_dir("system-toast-title-second");
    for root in [&first_root, &second_root] {
        write_runtime_bow_item_assets(root);
    }
    write_runtime_json(
        &runtime_assets_dir(&first_root)
            .join("lang")
            .join("en_us.json"),
        r#"{
            "chunk.toast.lowDiskSpace": "First language title with fixed construction width",
            "chunk.toast.lowDiskSpace.description": "First frozen message"
        }"#,
    );
    write_runtime_json(
        &runtime_assets_dir(&second_root)
            .join("lang")
            .join("en_us.json"),
        r#"{
            "chunk.toast.lowDiskSpace": "Second language title",
            "chunk.toast.lowDiskSpace.description": "Second reset message that must not resize the existing toast"
        }"#,
    );
    let first_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&first_root).unwrap())
            .unwrap();
    let second_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&second_root).unwrap())
            .unwrap();
    let glyphs = bbb_client_render::HudFontGlyphMap::default();
    let start = Instant::now();
    let mut world = WorldStore::new();
    let mut state = ClientToastRuntime::default();

    world.apply_low_disk_space_warning();
    state.update_with_options(&world, Some(&first_runtime), &glyphs, None, 1.0, start);
    let initial = state.project_systems(Some(&first_runtime)).remove(0).toast;
    assert_eq!(
        initial.title,
        "First language title with fixed construction width"
    );
    assert_eq!(initial.message_lines, ["First frozen message"]);
    let construction_width = initial.width;

    let after_language_reload = state.project_systems(Some(&second_runtime)).remove(0).toast;
    assert_eq!(after_language_reload.title, "Second language title");
    assert_eq!(
        after_language_reload.message_lines,
        ["First frozen message"]
    );
    assert_eq!(after_language_reload.width, construction_width);

    world.apply_low_disk_space_warning();
    state.update_with_options(
        &world,
        Some(&second_runtime),
        &glyphs,
        None,
        1.0,
        start + Duration::from_millis(10),
    );
    let after_reset = state.project_systems(Some(&first_runtime)).remove(0).toast;
    assert_eq!(
        after_reset.title,
        "First language title with fixed construction width"
    );
    assert_eq!(
        after_reset.message_lines,
        ["Second reset message that must not resize the existing toast"]
    );
    assert_eq!(after_reset.width, construction_width);

    std::fs::remove_dir_all(first_root).unwrap();
    std::fs::remove_dir_all(second_root).unwrap();
}

#[test]
fn system_toast_title_arguments_relocalize_without_rebuilding_download_content() {
    let first_root = unique_runtime_temp_dir("system-toast-args-first");
    let second_root = unique_runtime_temp_dir("system-toast-args-second");
    for root in [&first_root, &second_root] {
        write_runtime_bow_item_assets(root);
    }
    write_runtime_json(
        &runtime_assets_dir(&first_root)
            .join("lang")
            .join("en_us.json"),
        r#"{
            "download.pack.title": "Initial pack %s/%s",
            "download.pack.progress.percent": "Initial progress: %s%%"
        }"#,
    );
    write_runtime_json(
        &runtime_assets_dir(&second_root)
            .join("lang")
            .join("en_us.json"),
        r#"{
            "download.pack.title": "Reloaded total %2$s current %1$s",
            "download.pack.progress.percent": "Reloaded progress: %s%%"
        }"#,
    );
    let first_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&first_root).unwrap())
            .unwrap();
    let second_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&second_root).unwrap())
            .unwrap();
    let mut world = WorldStore::new();
    world.set_resource_pack_download_state(Some(bbb_client_world::ResourcePackDownloadState {
        batch_sequence: 3,
        update_sequence: 1,
        current_pack: 2,
        total_packs: 5,
        downloaded_bytes: 50,
        total_bytes: Some(100),
        failed_packs: 0,
        phase: bbb_client_world::ResourcePackDownloadPhase::Downloading,
        toast_action: bbb_client_world::ResourcePackDownloadToastAction::AddOrUpdateProgress,
    }));
    let start = Instant::now();
    let mut state = ClientToastRuntime::default();
    state.update_with_options(
        &world,
        Some(&first_runtime),
        &bbb_client_render::HudFontGlyphMap::default(),
        None,
        1.0,
        start,
    );

    let initial = state.project_systems(Some(&first_runtime)).remove(0).toast;
    assert_eq!(initial.title, "Initial pack 2/5");
    assert_eq!(initial.message_lines, ["Initial progress: 50%"]);
    let after_language_reload = state.project_systems(Some(&second_runtime)).remove(0).toast;
    assert_eq!(after_language_reload.title, "Reloaded total 5 current 2");
    assert_eq!(
        after_language_reload.message_lines,
        ["Initial progress: 50%"]
    );
    assert_eq!(after_language_reload.width, initial.width);

    std::fs::remove_dir_all(first_root).unwrap();
    std::fs::remove_dir_all(second_root).unwrap();
}

#[test]
fn low_disk_space_warning_projects_one_server_driven_system_toast() {
    let start = std::time::Instant::now();
    let mut world = WorldStore::new();
    let mut state = HudRenderRuntime::default();

    assert_eq!(
        hud_system_toast(&mut state, &world, None, start),
        (None, None)
    );

    world.apply_low_disk_space_warning();
    let (toast, sound) = hud_system_toast(&mut state, &world, None, start);
    assert_eq!(
        toast,
        Some(HudSystemToast {
            title: "Low disk space!".to_string(),
            message_lines: vec!["Might not be able to save the world.".to_string()],
            width: 160,
            visible_portion: 0.0,
        })
    );
    assert!(sound.is_none());
    assert_eq!(
        hud_system_toast(&mut state, &world, None, start)
            .1
            .and_then(|sound| sound.sound.location)
            .as_deref(),
        Some("minecraft:ui.toast.in")
    );
    assert_eq!(
        hud_system_toast(&mut state, &world, None, start + Duration::from_millis(600),)
            .0
            .map(|toast| toast.visible_portion),
        Some(1.0)
    );

    world.apply_low_disk_space_warning();
    assert_eq!(
        hud_system_toast(
            &mut state,
            &world,
            None,
            start + Duration::from_millis(10_500),
        )
        .0
        .map(|toast| toast.visible_portion),
        Some(1.0)
    );
    assert_eq!(
        hud_system_toast(
            &mut state,
            &world,
            None,
            start + Duration::from_millis(20_499),
        )
        .0
        .map(|toast| toast.visible_portion),
        Some(1.0)
    );
}

#[test]
fn completed_resource_pack_recovery_projects_pack_load_failure_system_toast() {
    let start = std::time::Instant::now();
    let mut world = WorldStore::new();
    let mut state = HudRenderRuntime::default();

    world.apply_resource_pack_load_failure();
    let (toast, sound) = hud_system_toast(&mut state, &world, None, start);
    assert_eq!(
        toast,
        Some(HudSystemToast {
            title: "Resource reload failed".to_string(),
            message_lines: Vec::new(),
            width: 160,
            visible_portion: 0.0,
        })
    );
    assert!(sound.is_none());
    assert_eq!(
        hud_system_toast(&mut state, &world, None, start)
            .1
            .and_then(|sound| sound.sound.location)
            .as_deref(),
        Some("minecraft:ui.toast.in")
    );
}

#[test]
fn shared_toast_projection_preserves_mixed_family_insertion_order() {
    let mut world = WorldStore::new();
    world.apply_low_disk_space_warning();
    enqueue_runtime_advancement_toasts(&mut world, 1, AdvancementFrameType::Task);
    enqueue_runtime_recipe_toast(&mut world, 1, 1, 2);
    let item_runtime = recipe_book_ghost_item_runtime();
    let start = Instant::now();
    let mut state = ClientToastRuntime::default();
    state.update_with_options(
        &world,
        Some(&item_runtime),
        &bbb_client_render::HudFontGlyphMap::default(),
        None,
        1.0,
        start,
    );

    let projected = state.project_toasts(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        ItemModelKeybindContext::default(),
        0.0,
    );
    assert!(matches!(
        projected.as_slice(),
        [
            HudToast::System(_),
            HudToast::Advancement(_),
            HudToast::Recipe(_)
        ]
    ));
}

#[test]
fn system_add_or_update_keeps_slot_and_construction_width_while_resetting_lifetime() {
    let mut glyphs = bbb_client_render::HudFontGlyphMap::new();
    let glyph = bbb_client_render::HudAsciiGlyph {
        width: 5,
        height: 8,
        advance: 6.0,
        ..bbb_client_render::HudAsciiGlyph::default()
    };
    for ch in ' '..='~' {
        glyphs.insert_first_wins(ch, glyph);
    }
    let mut world = WorldStore::new();
    let mut state = ClientToastRuntime::default();
    let start = Instant::now();
    world.set_resource_pack_download_state(Some(bbb_client_world::ResourcePackDownloadState {
        batch_sequence: 3,
        update_sequence: 1,
        current_pack: 1,
        total_packs: 100,
        downloaded_bytes: 0,
        total_bytes: Some(100),
        failed_packs: 0,
        phase: bbb_client_world::ResourcePackDownloadPhase::Downloading,
        toast_action: bbb_client_world::ResourcePackDownloadToastAction::AddOrUpdateTitle,
    }));
    state.update_with_options(&world, None, &glyphs, None, 1.0, start);
    let initial = state.project_systems(None).remove(0);
    assert_eq!(initial.slot, 0);
    assert!(initial.toast.width > 160);
    state.update_with_options(&world, None, &glyphs, None, 1.0, start);
    state.update_with_options(
        &world,
        None,
        &glyphs,
        None,
        1.0,
        start + Duration::from_millis(600),
    );

    world.set_resource_pack_download_state(Some(bbb_client_world::ResourcePackDownloadState {
        batch_sequence: 3,
        update_sequence: 2,
        current_pack: 100,
        total_packs: 100,
        downloaded_bytes: 100,
        total_bytes: Some(100),
        failed_packs: 1,
        phase: bbb_client_world::ResourcePackDownloadPhase::Failed,
        toast_action: bbb_client_world::ResourcePackDownloadToastAction::AddOrUpdateTitle,
    }));
    state.update_with_options(
        &world,
        None,
        &glyphs,
        None,
        1.0,
        start + Duration::from_millis(4_000),
    );
    let updated = state.project_systems(None).remove(0);
    assert_eq!(updated.slot, 0);
    assert_eq!(updated.toast.width, initial.toast.width);
    assert_eq!(
        updated.toast.title,
        "1 out of 100 pack(s) failed to download"
    );

    state.update_with_options(
        &world,
        None,
        &glyphs,
        None,
        1.0,
        start + Duration::from_millis(8_999),
    );
    assert!(state.visible[0].showing);
    state.update_with_options(
        &world,
        None,
        &glyphs,
        None,
        1.0,
        start + Duration::from_millis(9_000),
    );
    assert!(!state.visible[0].showing);
}

#[test]
fn periodic_add_queues_distinct_toasts_and_display_multiplier_scales_system_lifetime() {
    let start = Instant::now();
    let world = WorldStore::new();
    let glyphs = bbb_client_render::HudFontGlyphMap::default();
    let mut state = ClientToastRuntime::default();
    let notification = |sequence| periodic_notifications::PeriodicNotificationToast {
        title_key: format!("title.{sequence}"),
        message_key: format!("message.{sequence}"),
        elapsed_periods: 0,
        sequence,
    };
    state.update_with_options(&world, None, &glyphs, Some(notification(1)), 2.0, start);
    state.update_with_options(&world, None, &glyphs, Some(notification(2)), 2.0, start);
    assert_eq!(state.visible.len(), 2, "SystemToast.add does not coalesce");
    state.update_with_options(
        &world,
        None,
        &glyphs,
        None,
        2.0,
        start + Duration::from_millis(600),
    );
    state.update_with_options(
        &world,
        None,
        &glyphs,
        None,
        2.0,
        start + Duration::from_millis(10_599),
    );
    assert!(state.visible.iter().all(|toast| toast.showing));
    state.update_with_options(
        &world,
        None,
        &glyphs,
        None,
        2.0,
        start + Duration::from_millis(10_600),
    );
    assert!(state.visible.iter().all(|toast| !toast.showing));
}

#[test]
fn server_pack_download_progress_projects_vanilla_system_toast_and_forced_hide() {
    let start = std::time::Instant::now();
    let mut world = WorldStore::new();
    let mut state = HudRenderRuntime::default();
    world.set_resource_pack_download_state(Some(bbb_client_world::ResourcePackDownloadState {
        batch_sequence: 7,
        update_sequence: 1,
        current_pack: 1,
        total_packs: 2,
        downloaded_bytes: 512,
        total_bytes: Some(1_024),
        failed_packs: 0,
        phase: bbb_client_world::ResourcePackDownloadPhase::Downloading,
        toast_action: bbb_client_world::ResourcePackDownloadToastAction::AddOrUpdateProgress,
    }));

    let (toasts, sound) = hud_system_toasts(&mut state, &world, None, start);
    assert_eq!(
        toasts,
        vec![bbb_client_render::HudSystemToastInstance {
            toast: HudSystemToast {
                title: "Downloading resource pack 1/2".to_string(),
                message_lines: vec!["Progress: 50%".to_string()],
                width: 160,
                visible_portion: 0.0,
            },
            slot: 0,
        }]
    );
    assert!(sound.is_none());
    let (_, sound) = hud_system_toasts(&mut state, &world, None, start + Duration::from_millis(16));
    assert_eq!(
        sound.and_then(|sound| sound.sound.location).as_deref(),
        Some("minecraft:ui.toast.in")
    );

    world.set_resource_pack_download_state(Some(bbb_client_world::ResourcePackDownloadState {
        batch_sequence: 7,
        update_sequence: 2,
        current_pack: 2,
        total_packs: 2,
        downloaded_bytes: 512,
        total_bytes: Some(1_024),
        failed_packs: 0,
        phase: bbb_client_world::ResourcePackDownloadPhase::Downloading,
        toast_action:
            bbb_client_world::ResourcePackDownloadToastAction::AddOrUpdateTitlePreserveMessage,
    }));
    let (toasts, _) =
        hud_system_toasts(&mut state, &world, None, start + Duration::from_millis(500));
    assert_eq!(toasts[0].toast.title, "Downloading resource pack 2/2");
    assert_eq!(toasts[0].toast.message_lines, ["Progress: 50%"]);

    world.set_resource_pack_download_state(Some(bbb_client_world::ResourcePackDownloadState {
        batch_sequence: 7,
        update_sequence: 3,
        current_pack: 2,
        total_packs: 2,
        downloaded_bytes: 0,
        total_bytes: None,
        failed_packs: 1,
        phase: bbb_client_world::ResourcePackDownloadPhase::Failed,
        toast_action: bbb_client_world::ResourcePackDownloadToastAction::AddOrUpdateTitle,
    }));
    let (toasts, _) =
        hud_system_toasts(&mut state, &world, None, start + Duration::from_millis(700));
    assert_eq!(
        toasts[0].toast.title,
        "1 out of 2 pack(s) failed to download"
    );

    world.set_resource_pack_download_state(Some(bbb_client_world::ResourcePackDownloadState {
        batch_sequence: 8,
        update_sequence: 4,
        current_pack: 1,
        total_packs: 1,
        downloaded_bytes: 2_048,
        total_bytes: None,
        failed_packs: 0,
        phase: bbb_client_world::ResourcePackDownloadPhase::Downloading,
        toast_action: bbb_client_world::ResourcePackDownloadToastAction::AddOrUpdateProgress,
    }));
    let (toasts, _) = hud_system_toasts(
        &mut state,
        &world,
        None,
        start + Duration::from_millis(1_000),
    );
    assert_eq!(toasts.len(), 2);
    assert_eq!(toasts[0].slot, 0);
    assert_eq!(
        toasts[0].toast.title,
        "1 out of 2 pack(s) failed to download"
    );
    assert_eq!(toasts[1].slot, 1);
    assert_eq!(
        toasts[1].toast.message_lines.join(""),
        "Progress: 2.0 KB (total size unknown)"
    );
    let (_, sound) = hud_system_toasts(
        &mut state,
        &world,
        None,
        start + Duration::from_millis(1_016),
    );
    assert_eq!(
        sound.and_then(|sound| sound.sound.location).as_deref(),
        Some("minecraft:ui.toast.in")
    );
    let _ = hud_system_toasts(
        &mut state,
        &world,
        None,
        start + Duration::from_millis(1_616),
    );

    world.set_resource_pack_download_state(Some(bbb_client_world::ResourcePackDownloadState {
        batch_sequence: 8,
        update_sequence: 5,
        current_pack: 1,
        total_packs: 1,
        downloaded_bytes: 2_048,
        total_bytes: None,
        failed_packs: 0,
        phase: bbb_client_world::ResourcePackDownloadPhase::Succeeded,
        toast_action: bbb_client_world::ResourcePackDownloadToastAction::ForceHide,
    }));
    let (_, sound) = hud_system_toasts(
        &mut state,
        &world,
        None,
        start + Duration::from_millis(1_620),
    );
    assert_eq!(
        sound.and_then(|sound| sound.sound.location),
        Some("minecraft:ui.toast.out".to_string())
    );

    world.reset_resource_pack_session();
    world.set_resource_pack_download_state(Some(bbb_client_world::ResourcePackDownloadState {
        batch_sequence: 7,
        update_sequence: 1,
        current_pack: 1,
        total_packs: 1,
        downloaded_bytes: 64,
        total_bytes: None,
        failed_packs: 0,
        phase: bbb_client_world::ResourcePackDownloadPhase::Downloading,
        toast_action: bbb_client_world::ResourcePackDownloadToastAction::AddOrUpdateProgress,
    }));
    let (toasts, sound) = hud_system_toasts(
        &mut state,
        &world,
        None,
        start + Duration::from_millis(2_000),
    );
    assert_eq!(toasts.len(), 3);
    assert_eq!(toasts[2].slot, 2);
    assert_eq!(toasts[2].toast.title, "Downloading resource pack 1/1");
    assert!(sound.is_none());
}

#[test]
fn advancement_toast_runtime_admits_five_fifo_slots_and_matches_vanilla_timing() {
    let mut world = WorldStore::new();
    enqueue_runtime_advancement_toasts(&mut world, 6, AdvancementFrameType::Challenge);
    let start = Instant::now();
    let mut state = ClientToastRuntime::default();

    let admission_sounds = state.update(&world, start);
    assert_eq!(state.visible.len(), 5);
    assert_eq!(state.queued.len(), 1);
    assert_eq!(
        state
            .visible
            .iter()
            .map(|toast| toast.slot)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4]
    );
    assert_eq!(
        state.visible[0]
            .toast
            .advancement_event()
            .expect("advancement event")
            .advancement()
            .expect("advancement toast")
            .0,
        "test:toast_0"
    );
    assert_eq!(
        state.queued[0]
            .advancement_event()
            .expect("advancement event")
            .advancement()
            .expect("advancement toast")
            .0,
        "test:toast_5"
    );
    assert_eq!(
        sound_locations(&admission_sounds),
        vec!["minecraft:ui.toast.challenge_complete"]
    );

    let transition_sounds = state.update(&world, start);
    assert_eq!(
        sound_locations(&transition_sounds),
        vec!["minecraft:ui.toast.in"]
    );
    assert!(state
        .visible
        .iter()
        .all(|toast| toast.visible_portion == 0.0));

    assert!(state
        .update(&world, start + Duration::from_millis(300))
        .is_empty());
    assert!(state
        .visible
        .iter()
        .all(|toast| (toast.visible_portion - 0.25).abs() < 1.0e-6));
    state.update(&world, start + Duration::from_millis(600));
    assert!(state
        .visible
        .iter()
        .all(|toast| toast.visible_portion == 1.0 && toast.fully_visible_ms == 0));

    assert!(state
        .update(&world, start + Duration::from_millis(5_599))
        .is_empty());
    assert_eq!(state.visible[0].fully_visible_ms, 4_999);
    assert_eq!(
        sound_locations(&state.update(&world, start + Duration::from_millis(5_600))),
        vec!["minecraft:ui.toast.out"]
    );
    state.update(&world, start + Duration::from_millis(5_900));
    assert!((state.visible[0].visible_portion - 0.75).abs() < 1.0e-6);

    let next_admission = state.update(&world, start + Duration::from_millis(6_201));
    assert_eq!(state.visible.len(), 1);
    assert_eq!(
        state.visible[0]
            .toast
            .advancement_event()
            .expect("advancement event")
            .advancement()
            .expect("advancement toast")
            .0,
        "test:toast_5"
    );
    assert_eq!(state.visible[0].slot, 0);
    assert_eq!(
        sound_locations(&next_admission),
        vec!["minecraft:ui.toast.challenge_complete"]
    );
}

#[test]
fn tutorial_toast_uses_contiguous_slots_allows_later_small_toast_bypass_and_smooths_progress() {
    let mut world = runtime_tutorial_world();
    enqueue_runtime_advancement_toasts(&mut world, 4, AdvancementFrameType::Task);
    world.advance_tutorial_ticks(1);
    world.advance_tutorial_ticks(100);
    enqueue_runtime_recipe_toast(&mut world, 1, 1, 2);

    let mut glyphs = bbb_client_render::HudFontGlyphMap::new();
    let glyph = bbb_client_render::HudAsciiGlyph {
        width: 5,
        height: 8,
        advance: 6.0,
        ..bbb_client_render::HudAsciiGlyph::default()
    };
    for ch in ' '..='~' {
        glyphs.insert_first_wins(ch, glyph);
    }
    let start = Instant::now();
    let mut state = ClientToastRuntime::default();
    state.update_with_resources(&world, None, &glyphs, start);

    assert_eq!(state.visible.len(), 5);
    assert!(matches!(state.visible[4].toast, ClientToast::Recipe(_)));
    assert_eq!(state.visible[4].slot, 4);
    let ClientToast::Tutorial(queued_tutorial) = &state.queued[0] else {
        panic!("two-slot tutorial remains queued behind four occupied slots");
    };
    assert_eq!(queued_tutorial.lines.occupied_slots(), 2);
    assert!(
        queued_tutorial
            .lines
            .as_slice()
            .iter()
            .flatten()
            .any(|run| run.style.bold),
        "Vanilla Tutorial.key placeholders remain bold through wrapping"
    );
    assert!(queued_tutorial
        .lines
        .as_slice()
        .first()
        .expect("title line")
        .iter()
        .all(|run| run.color == Some(0x50_0050)));

    let mut tutorial_only = runtime_tutorial_world();
    tutorial_only.advance_tutorial_ticks(1);
    tutorial_only.advance_tutorial_ticks(100);
    let mut tutorial_runtime = ClientToastRuntime::default();
    tutorial_runtime.update_with_resources(&tutorial_only, None, &glyphs, start);
    tutorial_runtime.update_with_resources(&tutorial_only, None, &glyphs, start);
    tutorial_runtime.update_with_resources(
        &tutorial_only,
        None,
        &glyphs,
        start + Duration::from_millis(600),
    );
    for _ in 0..20 {
        tutorial_only.tutorial_on_input(true, false, false, false, false);
        tutorial_only.advance_tutorial_ticks(1);
    }
    tutorial_runtime.update_with_resources(
        &tutorial_only,
        None,
        &glyphs,
        start + Duration::from_millis(650),
    );
    let projected = tutorial_runtime.project_tutorials();
    assert_eq!(projected.len(), 1);
    assert!((projected[0].progress.unwrap() - 0.25).abs() < 1.0e-6);

    tutorial_only.set_tutorial_step(bbb_client_world::TutorialStep::None);
    assert_eq!(
        sound_locations(&tutorial_runtime.update_with_resources(
            &tutorial_only,
            None,
            &glyphs,
            start + Duration::from_millis(700),
        )),
        vec!["minecraft:ui.toast.out"]
    );
}

#[test]
fn social_tutorial_toast_freezes_localized_key_content_and_hides_strictly_after_8000ms() {
    let first_root = unique_runtime_temp_dir("social-tutorial-first");
    let second_root = unique_runtime_temp_dir("social-tutorial-second");
    for root in [&first_root, &second_root] {
        write_runtime_bow_item_assets(root);
    }
    write_runtime_json(
        &runtime_assets_dir(&first_root)
            .join("lang")
            .join("en_us.json"),
        r#"{
            "tutorial.socialInteractions.title": "First Social",
            "tutorial.socialInteractions.description": "Tap %s first",
            "key.keyboard.p": "First P"
        }"#,
    );
    write_runtime_json(
        &runtime_assets_dir(&second_root)
            .join("lang")
            .join("en_us.json"),
        r#"{
            "tutorial.socialInteractions.title": "Reloaded Social",
            "tutorial.socialInteractions.description": "Use %s after reload",
            "key.keyboard.p": "Reloaded P"
        }"#,
    );
    let first_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&first_root).unwrap())
            .unwrap();
    let second_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&second_root).unwrap())
            .unwrap();
    let mut world = runtime_tutorial_world();
    assert!(world.show_social_interactions_tutorial_if_first_join());

    let mut glyphs = bbb_client_render::HudFontGlyphMap::new();
    let glyph = bbb_client_render::HudAsciiGlyph {
        width: 5,
        height: 8,
        advance: 6.0,
        ..bbb_client_render::HudAsciiGlyph::default()
    };
    for ch in ' '..='~' {
        glyphs.insert_first_wins(ch, glyph);
    }

    let start = Instant::now();
    let mut runtime = ClientToastRuntime::default();
    runtime.update_with_resources(&world, Some(&first_runtime), &glyphs, start);
    runtime.update_with_resources(&world, Some(&second_runtime), &glyphs, start);
    runtime.update_with_resources(
        &world,
        Some(&second_runtime),
        &glyphs,
        start + Duration::from_millis(600),
    );

    let tutorial = runtime
        .visible
        .iter()
        .find(|instance| matches!(instance.toast, ClientToast::Tutorial(_)))
        .expect("social tutorial is admitted");
    let ClientToast::Tutorial(tutorial_state) = &tutorial.toast else {
        unreachable!()
    };
    assert_eq!(
        tutorial_state.token,
        bbb_client_world::ClientTutorialToastToken::SocialInteractions
    );
    assert_eq!(tutorial_state.time_to_display_ms, 8_000);
    let lines = tutorial_state.lines.as_slice();
    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[0]
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        "First Social"
    );
    assert_eq!(
        lines[1]
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        "Tap First P first"
    );
    assert!(lines[1]
        .iter()
        .any(|run| run.text == "First P" && run.style.bold));

    runtime.update_with_resources(
        &world,
        Some(&second_runtime),
        &glyphs,
        start + Duration::from_millis(8_600),
    );
    let projected = runtime.project_tutorials();
    assert_eq!(projected.len(), 1);
    assert_eq!(
        projected[0].icon,
        bbb_client_render::HudTutorialToastIcon::SocialInteractions
    );
    assert_eq!(projected[0].progress, Some(1.0));
    assert!(runtime.visible.iter().any(|instance| {
        matches!(
            &instance.toast,
            ClientToast::Tutorial(tutorial)
                if tutorial.token == bbb_client_world::ClientTutorialToastToken::SocialInteractions
                    && instance.showing
        )
    }));

    runtime.update_with_resources(
        &world,
        Some(&second_runtime),
        &glyphs,
        start + Duration::from_millis(8_601),
    );
    assert!(runtime.visible.iter().any(|instance| {
        matches!(
            &instance.toast,
            ClientToast::Tutorial(tutorial)
                if tutorial.token == bbb_client_world::ClientTutorialToastToken::SocialInteractions
                    && !instance.showing
        )
    }));

    std::fs::remove_dir_all(first_root).unwrap();
    std::fs::remove_dir_all(second_root).unwrap();
}

#[test]
fn first_multiplayer_tutorial_queues_the_joined_sentinel_once() {
    let mut world = runtime_tutorial_world();
    let mut platform = crate::simulation::effects::PlatformEffects::default();
    let information = bbb_protocol::packets::ClientInformation::default();
    advance_first_multiplayer_server_tutorial(&mut world, &information, &mut platform);
    let effects = platform.take();
    let [crate::simulation::effects::PlatformEffect::SaveClientOptions { writes, broadcast }] =
        effects.as_slice()
    else {
        panic!("expected options save: {effects:?}");
    };
    assert_eq!(
        writes,
        &[crate::options::ClientOptionWrite {
            key: "joinedFirstServer",
            value: crate::options::ClientOptionValue::Boolean(true),
        }]
    );
    assert_eq!(broadcast, &information);

    advance_first_multiplayer_server_tutorial(&mut world, &information, &mut platform);
    assert!(platform.take().is_empty());
}

#[test]
fn social_interactions_hide_mutates_the_same_visible_tutorial_before_screen_open() {
    let mut world = runtime_tutorial_world();
    assert!(world.show_social_interactions_tutorial_if_first_join());
    let start = Instant::now();
    let mut runtime = ClientToastRuntime::default();
    runtime.update(&world, start);
    runtime.update(&world, start);

    assert!(world.hide_social_interactions_tutorial());
    runtime.update(&world, start + Duration::from_millis(1));

    assert!(runtime.visible.iter().any(|instance| {
        matches!(
            &instance.toast,
            ClientToast::Tutorial(tutorial)
                if tutorial.token == bbb_client_world::ClientTutorialToastToken::SocialInteractions
                    && tutorial.force_hide
                    && !instance.showing
        )
    }));
}

#[test]
fn advancement_toast_runtime_counts_retained_window_lag_and_consumes_once() {
    let mut world = WorldStore::new();
    enqueue_runtime_advancement_toasts(
        &mut world,
        bbb_client_world::MAX_CLIENT_TOAST_EVENTS + 1,
        AdvancementFrameType::Task,
    );
    let start = Instant::now();
    let mut state = ClientToastRuntime::default();
    state.update(&world, start);
    assert_eq!(state.dropped_events, 1);
    assert_eq!(
        state.next_sequence,
        u64::try_from(bbb_client_world::MAX_CLIENT_TOAST_EVENTS + 1).unwrap()
    );
    assert_eq!(state.visible.len(), 5);
    assert_eq!(
        state.queued.len(),
        bbb_client_world::MAX_CLIENT_TOAST_EVENTS - bbb_client_render::HUD_TOAST_SLOTS
    );

    state.update(&world, start);
    assert_eq!(state.dropped_events, 1);
    assert_eq!(state.visible.len(), 5);
    assert_eq!(
        state.queued.len(),
        bbb_client_world::MAX_CLIENT_TOAST_EVENTS - bbb_client_render::HUD_TOAST_SLOTS
    );
}

#[test]
fn toast_runtime_clears_visible_and_queued_state_on_disconnect() {
    let mut world = WorldStore::new();
    enqueue_runtime_advancement_toasts(&mut world, 6, AdvancementFrameType::Goal);
    let mut state = ClientToastRuntime::default();
    state.update(&world, Instant::now());
    assert_eq!(state.visible.len(), 5);
    assert_eq!(state.queued.len(), 1);

    world.clear_client_level();
    assert!(state.update(&world, Instant::now()).is_empty());
    assert!(state.visible.is_empty());
    assert!(state.queued.is_empty());
    assert_eq!(state.next_sequence, 0);
}

#[test]
fn toast_runtime_survives_respawn_without_duplicate_admission() {
    let mut world = WorldStore::new();
    enqueue_runtime_advancement_toasts(&mut world, 1, AdvancementFrameType::Goal);
    let start = Instant::now();
    let mut state = ClientToastRuntime::default();
    state.update(&world, start);
    assert_eq!(state.visible.len(), 1);
    let event_sequence = state.next_sequence;

    world.apply_respawn(&bbb_protocol::packets::Respawn {
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 0,
            game_type: 0,
            previous_game_type: 0,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        data_to_keep: 0,
    });
    state.update(&world, start);
    assert_eq!(state.visible.len(), 1);
    assert!(state.queued.is_empty());
    assert_eq!(state.next_sequence, event_sequence);
}

#[test]
fn recipe_toast_runtime_coalesces_resets_lifetime_cycles_and_revives() {
    let mut world = WorldStore::new();
    enqueue_runtime_recipe_toast(&mut world, 1, 1, 2);
    let start = Instant::now();
    let mut state = ClientToastRuntime::default();

    assert!(state.update(&world, start).is_empty());
    assert_eq!(state.visible.len(), 1);
    assert!(matches!(state.visible[0].toast, ClientToast::Recipe(_)));
    assert_eq!(
        sound_locations(&state.update(&world, start)),
        vec!["minecraft:ui.toast.in"]
    );
    state.update(&world, start + Duration::from_millis(600));
    state.update(&world, start + Duration::from_millis(4_600));

    enqueue_runtime_recipe_toast(&mut world, 2, 3, 4);
    assert!(state
        .update(&world, start + Duration::from_millis(4_600))
        .is_empty());
    let ClientToast::Recipe(recipe) = &state.visible[0].toast else {
        panic!("recipe toast");
    };
    assert_eq!(recipe.items.len(), 2);
    assert_eq!(recipe.last_changed_ms, 4_000);
    assert_eq!(
        recipe
            .displayed_item()
            .and_then(|item| item.unlocked_item.item_id),
        Some(4)
    );
    let projected = state.project_recipes(
        &world,
        None,
        &TerrainTextureState::default(),
        ItemModelKeybindContext::default(),
        0.0,
    );
    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].title, "New Recipe(s) Unlocked!");
    assert_eq!(projected[0].description, "Check your recipe book");
    assert_eq!(projected[0].slot, 0);
    assert!(state
        .update(&world, start + Duration::from_millis(9_599))
        .is_empty());
    assert_eq!(
        sound_locations(&state.update(&world, start + Duration::from_millis(9_600))),
        vec!["minecraft:ui.toast.out"]
    );

    enqueue_runtime_recipe_toast(&mut world, 3, 5, 6);
    assert_eq!(
        sound_locations(&state.update(&world, start + Duration::from_millis(9_700))),
        vec!["minecraft:ui.toast.in"]
    );
    assert_eq!(state.visible.len(), 1);
    let ClientToast::Recipe(recipe) = &state.visible[0].toast else {
        panic!("recipe toast");
    };
    assert_eq!(recipe.items.len(), 3);
    assert_eq!(recipe.last_changed_ms, 9_100);
}

#[test]
fn recipe_toast_runtime_coalesces_while_queued_and_keeps_global_fifo_order() {
    let mut world = WorldStore::new();
    enqueue_runtime_advancement_toasts(&mut world, 5, AdvancementFrameType::Task);
    let mut state = ClientToastRuntime::default();
    state.update(&world, Instant::now());
    assert_eq!(state.visible.len(), 5);

    enqueue_runtime_recipe_toast(&mut world, 1, 1, 2);
    enqueue_runtime_recipe_toast(&mut world, 2, 3, 4);
    enqueue_runtime_advancement_toasts(&mut world, 1, AdvancementFrameType::Goal);
    state.update(&world, Instant::now());

    assert_eq!(state.queued.len(), 2);
    let ClientToast::Recipe(recipe) = &state.queued[0] else {
        panic!("recipe toast stays first in FIFO");
    };
    assert_eq!(recipe.items.len(), 2);
    assert!(state.queued[1].advancement_event().is_some());
}

#[test]
fn advancement_toast_and_screen_preserve_rich_title_runs() {
    let rich_title =
        bbb_protocol::RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".into(),
                value: NbtSummaryValue::string("Styled toast"),
            },
            NbtSummaryEntry {
                name: "bold".into(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "color".into(),
                value: NbtSummaryValue::string("red"),
            },
        ]))
        .unwrap();
    let mut world = WorldStore::new();
    let mut advancement = runtime_displayed_advancement("test:styled", None);
    let display = advancement.display.as_mut().unwrap();
    display.title = rich_title.clone();
    display.show_toast = true;
    advancement.requirements = vec![vec!["done".into()]];
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![advancement],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    world.apply_update_advancements(UpdateAdvancements {
        reset: false,
        added: Vec::new(),
        removed: Vec::new(),
        progress: vec![AdvancementProgressSummary {
            id: "test:styled".to_string(),
            criteria: vec![AdvancementCriterionProgressSummary {
                name: "done".into(),
                obtained_epoch_millis: Some(1),
            }],
        }],
        show_advancements: true,
    });

    let mut runtime = ClientToastRuntime::default();
    runtime.update(&world, Instant::now());
    let item_runtime = recipe_book_ghost_item_runtime();
    let projected = runtime.project_advancements(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        ItemModelKeybindContext::default(),
        0.0,
    );
    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].title_runs[0].text, "Styled toast");
    assert!(projected[0].title_runs[0].style.bold);
    assert_eq!(projected[0].title_runs[0].color, Some(0xFF_55_55));

    assert_eq!(
        world.ensure_advancements_screen_selected_tab(),
        Some("test:styled".to_string())
    );
    let screen = hud_advancements_screen(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        0.0,
        winit::dpi::PhysicalSize::new(800, 600),
        None,
        ItemModelKeybindContext::default(),
        0.0,
        &approximate_inventory_hud_fonts(),
    );
    let title = screen
        .text_labels
        .iter()
        .find(|label| label.text == "Styled toast")
        .unwrap();
    assert!(title.component_runs[0].visual.style.bold);
    assert_eq!(title.component_runs[0].visual.color, Some(0xFF_55_55));
}

#[test]
fn mixed_system_toasts_share_fifo_and_multiline_contiguous_slots() {
    let start = std::time::Instant::now();
    let mut world = world_with_custom_dimension_environment();
    assert_eq!(world.insecure_chat_warning_count(), 1);
    world.apply_low_disk_space_warning();

    let mut glyphs = bbb_client_render::HudFontGlyphMap::new();
    let glyph = bbb_client_render::HudAsciiGlyph {
        width: 5,
        height: 8,
        advance: 6.0,
        ..bbb_client_render::HudAsciiGlyph::default()
    };
    for ch in ' '..='~' {
        glyphs.insert_first_wins(ch, glyph);
    }
    let mut state = ClientToastRuntime::default();
    let admission = state.update_with_options(&world, None, &glyphs, None, 1.0, start);
    assert!(admission.is_empty());
    assert_eq!(
        state
            .visible
            .iter()
            .map(|toast| toast.slot)
            .collect::<Vec<_>>(),
        [0, 2]
    );
    assert_eq!(
        state
            .visible
            .iter()
            .map(|toast| toast.toast.occupied_slot_count())
            .collect::<Vec<_>>(),
        [2, 1]
    );
    assert_eq!(
        state
            .project_systems(None)
            .iter()
            .map(|toast| toast.toast.title.as_str())
            .collect::<Vec<_>>(),
        ["Chat messages can't be verified", "Low disk space!"]
    );
    assert_eq!(
        sound_locations(&state.update_with_options(&world, None, &glyphs, None, 1.0, start)),
        ["minecraft:ui.toast.in"]
    );
    state.update_with_options(
        &world,
        None,
        &glyphs,
        None,
        1.0,
        start + Duration::from_millis(600),
    );
    assert_eq!(
        sound_locations(&state.update_with_options(
            &world,
            None,
            &glyphs,
            None,
            1.0,
            start + Duration::from_millis(10_600),
        )),
        ["minecraft:ui.toast.out"]
    );
}

#[test]
fn vanilla_stat_formatters_preserve_thresholds_and_grouping() {
    assert_eq!(format_vanilla_stat_value("DEFAULT", 1_234_567), "1,234,567");
    assert_eq!(format_vanilla_stat_value("DIVIDE_BY_TEN", 123), "12.30");
    assert_eq!(format_vanilla_stat_value("DISTANCE", 50), "50 cm");
    assert_eq!(format_vanilla_stat_value("DISTANCE", 51), "0.51 m");
    assert_eq!(format_vanilla_stat_value("DISTANCE", 50_001), "0.50 km");
    assert_eq!(format_vanilla_stat_value("TIME", 20), "1.0 s");
    assert_eq!(format_vanilla_stat_value("TIME", 601), "0.50 min");
}

#[test]
fn hud_debug_options_screen_projects_visible_rows_and_suppresses_pause() {
    let mut input = ClientInputState::new(true);
    let world = WorldStore::new();
    input.open_debug_pause_screen_without_menu_for_test();
    input.open_debug_options_screen(&mut Default::default(), &None);
    input.set_debug_screen_entry_status(
        DebugScreenEntryId::Biome,
        crate::debug_entries::DebugScreenEntryStatus::AlwaysOn,
    );

    assert!(hud_pause_screen(
        &input,
        &WorldStore::new(),
        None,
        winit::dpi::PhysicalSize::new(420, 240)
    )
    .is_none());
    let screen = hud_debug_options_screen(
        &input,
        &world,
        None,
        winit::dpi::PhysicalSize::new(420, 240),
    )
    .expect("debug options screen");

    assert_eq!(screen.title, "Debug Options");
    assert_eq!(screen.total_rows, 47);
    assert_eq!(screen.visible_rows, 7);
    assert!(screen.default_profile_active);
    assert_eq!(
        screen.rows[0],
        HudDebugOptionsRow::Category {
            label: "Debug Screen Text".to_string()
        }
    );
    assert_eq!(
        screen.rows[1],
        HudDebugOptionsRow::Entry {
            path: "biome".to_string(),
            status: HudDebugOptionsEntryStatus::AlwaysOn,
            hovered_status: None,
            focused_status: None,
            allowed: true,
        }
    );
}

#[test]
fn hud_debug_options_screen_projects_not_allowed_tooltip_under_reduced_debug_info() {
    let mut input = ClientInputState::new(true);
    let world =
        world_with_dimension_height_and_reduced_debug_info(0, "minecraft:overworld", 384, true);
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    input.open_debug_options_screen(&mut Default::default(), &None);
    assert!(input.handle_debug_options_screen_text_input("biome"));
    let tooltip_x = 37;
    let tooltip_y = 83;
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(tooltip_x),
            f64::from(tooltip_y)
        )),
        surface,
    ));

    let screen =
        hud_debug_options_screen(&input, &world, None, surface).expect("debug options screen");

    assert_eq!(
        screen.tooltip.as_ref().map(|tooltip| tooltip.text.as_str()),
        Some("Not visible when debug info is reduced")
    );
    assert_eq!(
        screen
            .tooltip
            .as_ref()
            .map(|tooltip| (tooltip.x, tooltip.y)),
        Some((tooltip_x, tooltip_y))
    );
    assert_eq!(
        screen.rows[1],
        HudDebugOptionsRow::Entry {
            path: "biome".to_string(),
            status: HudDebugOptionsEntryStatus::Never,
            hovered_status: None,
            focused_status: None,
            allowed: false,
        }
    );
}

#[test]
fn debug_options_narration_matches_search_list_and_status_button_semantics() {
    let mut input = ClientInputState::new(true);
    let world = WorldStore::new();
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    input.open_debug_options_screen(&mut Default::default(), &None);

    let screen =
        hud_debug_options_screen(&input, &world, None, surface).expect("debug options screen");
    let initial =
        debug_options_screen_narration_frame(&input, &screen, None, surface).all_text_for_test();
    assert!(initial.contains("Debug Options"));
    assert!(initial.contains("Screen element 1 out of 4"));
    assert!(initial.contains("Search... edit box:"));
    assert!(initial.contains("Press Tab to navigate to the next element"));

    assert!(input
        .handle_debug_options_screen_key(PhysicalKey::Code(KeyCode::Tab), ElementState::Pressed,));
    let screen =
        hud_debug_options_screen(&input, &world, None, surface).expect("debug options screen");
    let focused =
        debug_options_screen_narration_frame(&input, &screen, None, surface).all_text_for_test();
    assert!(focused.contains("Screen element 2 out of 4"));
    assert!(focused.contains("Selected list row 2 out of 47"));
    assert!(focused.contains("Selected row element 2 out of 3"));
    assert!(focused.contains("biome: Currently off: In Overlay"));
    assert!(focused.contains("Press Enter to activate"));

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(328.0, 82.0)),
        surface,
    ));
    let screen =
        hud_debug_options_screen(&input, &world, None, surface).expect("debug options screen");
    let same_row_hover =
        debug_options_screen_narration_frame(&input, &screen, None, surface).all_text_for_test();
    assert!(same_row_hover.contains("Selected row element 2 out of 3"));
    assert!(same_row_hover.contains("biome: Currently off: In Overlay"));
    assert!(!same_row_hover.contains("Currently off: Always"));

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(36.0, 62.0)),
        surface,
    ));
    let screen =
        hud_debug_options_screen(&input, &world, None, surface).expect("debug options screen");
    let category_hover =
        debug_options_screen_narration_frame(&input, &screen, None, surface).all_text_for_test();
    assert!(category_hover.contains("Selected list row 1 out of 47"));
    assert!(category_hover.contains("Debug Screen Text"));
    assert!(!category_hover.contains("Selected row element"));

    assert!(input.handle_debug_options_screen_cursor_moved(None, surface));
    assert!(
        input.handle_debug_options_screen_key(
            PhysicalKey::Code(KeyCode::Enter),
            ElementState::Pressed,
        )
    );
    let screen =
        hud_debug_options_screen(&input, &world, None, surface).expect("debug options screen");
    let selected =
        debug_options_screen_narration_frame(&input, &screen, None, surface).all_text_for_test();
    assert!(selected.contains("biome: Currently only in debug overlay: In Overlay"));
    assert!(!selected.contains("Press Enter to activate"));

    assert!(input.handle_debug_options_screen_mouse_input(
        MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(181.0, 214.0)),
        surface,
        false,
    ));
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(36.0, 62.0)),
        surface,
    ));
    let screen =
        hud_debug_options_screen(&input, &world, None, surface).expect("debug options screen");
    let inactive_profile_list_hover =
        debug_options_screen_narration_frame(&input, &screen, None, surface).all_text_for_test();
    assert!(inactive_profile_list_hover.contains("Debug Screen Text"));
    assert!(!inactive_profile_list_hover.contains("Press Tab to navigate"));
    assert!(!inactive_profile_list_hover.contains("Use mouse cursor"));

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(53.0, 214.0)),
        surface,
    ));
    let screen =
        hud_debug_options_screen(&input, &world, None, surface).expect("debug options screen");
    let inactive_profile_button_hover =
        debug_options_screen_narration_frame(&input, &screen, None, surface).all_text_for_test();
    assert!(inactive_profile_button_hover.contains("Default profile button"));
    assert!(inactive_profile_button_hover.contains("Left click to activate"));
    assert!(!inactive_profile_button_hover.contains("Press Enter to activate"));
}

#[test]
fn book_screen_open_releases_held_movement() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: written_book_stack(vec!["First page"]),
    });
    world.apply_open_book(OpenBook {
        hand: InteractionHand::MainHand,
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    release_input_if_screen_opened(false, &mut input, &mut world, &mut counters, &commands);

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(bbb_protocol::packets::PlayerInput::default())
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn renderer_camera_pose_follows_active_camera_entity() {
    let mut world = WorldStore::new();
    world.set_local_player_pose(local_player_pose([10.0, 64.0, -5.0], 90.0, -10.0));
    world.apply_add_entity(bbb_protocol::packets::AddEntity {
        id: 123,
        uuid: uuid::Uuid::from_u128(123),
        entity_type_id: VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        position: bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        delta_movement: bbb_protocol::packets::Vec3d::default(),
        x_rot: -15.0,
        y_rot: 30.0,
        y_head_rot: 30.0,
        data: 0,
    });

    assert_eq!(
        camera_pose_from_world(&world),
        Some(CameraPose {
            position: [10.0, 64.0, -5.0],
            y_rot: 90.0,
            x_rot: -10.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        })
    );

    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 123 }));
    assert_eq!(
        camera_pose_from_world(&world),
        Some(CameraPose {
            position: [1.0, 2.0, 3.0],
            y_rot: 30.0,
            x_rot: -15.0,
            eye_height: 0.2751,
        })
    );

    assert_eq!(
        world.apply_remove_entities(bbb_protocol::packets::RemoveEntities {
            entity_ids: vec![123],
        }),
        1
    );
    assert_eq!(
        camera_pose_from_world(&world),
        Some(CameraPose {
            position: [10.0, 64.0, -5.0],
            y_rot: 90.0,
            x_rot: -10.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        })
    );
}

#[test]
fn advancement_shadow_survives_wrap_and_hover_projection() {
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, RichTextComponent};
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let component = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
        entry("text", NbtSummaryValue::String("A".into())),
        entry("shadow_color", NbtSummaryValue::Int(0x8012_3456_u32 as i32)),
        entry(
            "extra",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Compound(vec![
                    entry("text", NbtSummaryValue::String("B".into())),
                    entry("shadow_color", NbtSummaryValue::Int(0)),
                ]),
                NbtSummaryValue::String("C".into()),
            ]),
        ),
    ]))
    .unwrap();
    let line =
        advancement_wrap_text_lines(&component, 100, &approximate_inventory_hud_fonts(), None);
    assert_eq!(
        line[0]
            .runs
            .iter()
            .map(|run| run.visual.shadow_color)
            .collect::<Vec<_>>(),
        [Some(0x8012_3456), Some(0), Some(0x8012_3456)]
    );
    let wrapped =
        advancement_wrap_text_lines(&component, 1, &approximate_inventory_hud_fonts(), None);
    assert_eq!(wrapped.len(), 3);
    assert_eq!(
        wrapped
            .iter()
            .map(|line| line.runs[0].visual.shadow_color)
            .collect::<Vec<_>>(),
        [Some(0x8012_3456), Some(0), Some(0x8012_3456)]
    );
    let hover = component_hover::hud_component_run(component.styled()[1].clone());
    assert_eq!(hover.visual.shadow_color, Some(0));
}

#[test]
fn hud_stats_screen_restored_after_dialog_clear_projects_stats_received_under_dialog() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let surface = winit::dpi::PhysicalSize::new(320, 240);
    let item_runtime = NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
    let terrain_textures = TerrainTextureState::default();

    input.set_screen_for_test(Screen::Stats(StatsScreen::default()));
    world.apply_show_dialog(bbb_protocol::packets::ShowDialog {
        dialog: bbb_protocol::packets::DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "type".into(),
                    value: NbtSummaryValue::String("minecraft:notice".into()),
                },
                NbtSummaryEntry {
                    name: "title".into(),
                    value: NbtSummaryValue::String("Notice".into()),
                },
            ]),
        },
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    assert!(input.dialog_screen_is_open());

    world.apply_award_stats(AwardStats {
        stats: vec![StatUpdate {
            stat_type_id: VANILLA_STAT_TYPE_CUSTOM_ID,
            value_id: 6,
            amount: 42,
        }],
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);

    world.apply_clear_dialog();
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    assert!(!input.dialog_screen_is_open());

    let screen = project_stats_screen(
        input.stats_screen_mut().expect("restored stats screen"),
        &world,
        Some(&item_runtime),
        &terrain_textures,
        ItemModelKeybindContext::default(),
        0.0,
        surface,
    );
    assert!(!screen.is_loading);
    assert!(!screen.rows.is_empty());
}
