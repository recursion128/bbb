use super::*;

#[test]
fn hud_inventory_screen_projects_notice_dialog_text_body_and_hovered_action() {
    let mut input = crate::input::ClientInputState::new(true);
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let mut world = WorldStore::new();
    world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:notice")),
                entry("title", NbtSummaryValue::string("Native Notice")),
                entry(
                    "body",
                    NbtSummaryValue::Compound(vec![
                        entry("type", NbtSummaryValue::string("minecraft:plain_message")),
                        entry("contents", NbtSummaryValue::string("AlphaBeta")),
                        entry("width", NbtSummaryValue::Int(30)),
                    ]),
                ),
                entry(
                    "action",
                    NbtSummaryValue::Compound(vec![
                        entry(
                            "label",
                            NbtSummaryValue::Compound(vec![
                                entry("text", NbtSummaryValue::string("Continue")),
                                entry("bold", NbtSummaryValue::Byte(1)),
                                entry("color", NbtSummaryValue::string("#55ffff")),
                            ]),
                        ),
                        entry("width", NbtSummaryValue::Int(100)),
                    ]),
                ),
            ]),
        },
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    let surface_size = winit::dpi::PhysicalSize::new(320, 240);

    let screen = dialog_screen::hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &TerrainTextureState::default(),
        &dialog_screen::DialogHudLocalState {
            cursor_position: Some((160, 220)),
            ..Default::default()
        },
        surface_size,
        0.0,
    )
    .unwrap();

    assert_eq!((screen.width, screen.height), (320, 240));
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/inworld_menu_background.png"),
                0,
                0,
                320,
                240,
                [0.0, 0.0],
                [10.0, 7.5],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("dialog/warning_button"),
                193,
                7,
                20,
                20,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("widget/button_highlighted"),
                110,
                213,
                100,
                20,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(
        screen
            .text_labels
            .iter()
            .map(|label| (label.text.as_str(), label.x, label.y, label.width))
            .collect::<Vec<_>>(),
        vec![("Native Notice", 107, 13, 76), ("Continue", 132, 219, 56),]
    );
    assert_eq!(
        screen
            .scroll_area
            .as_ref()
            .unwrap()
            .text_labels
            .iter()
            .map(|label| (label.text.as_str(), label.x, label.y, label.width))
            .collect::<Vec<_>>(),
        vec![
            ("Alp", 151, 67, 18),
            ("haB", 151, 76, 18),
            ("eta", 151, 85, 18),
        ]
    );
    let action_label = screen.text_labels.last().unwrap();
    assert_eq!(action_label.runs.len(), 1);
    assert!(action_label.runs[0].style.bold);
    assert_eq!(action_label.runs[0].color, Some(0x55_FF_FF));

    let idle = dialog_screen::hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &TerrainTextureState::default(),
        &dialog_screen::DialogHudLocalState {
            cursor_position: Some((0, 0)),
            ..Default::default()
        },
        surface_size,
        0.0,
    )
    .unwrap();
    assert_eq!(
        idle.background_layers[0].texture,
        HudInventoryBackgroundTexture::Texture("textures/gui/inworld_menu_background.png")
    );
    assert_eq!(
        idle.background_layers[1].texture,
        HudInventoryBackgroundTexture::Sprite("dialog/warning_button")
    );
    assert_eq!(
        idle.background_layers[2].texture,
        HudInventoryBackgroundTexture::Sprite("widget/button")
    );
    assert!(input.open_dialog_warning());
    let warning =
        hud_dialog_warning_screen(input.dialog_warning_focused_button(), None, surface_size);
    assert_eq!(warning.text_labels[0].text, "Note about custom screens");
}

#[test]
fn hud_inventory_screen_projects_dialog_item_body_structured_patches() {
    let mut input = crate::input::ClientInputState::new(true);
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let mut world = WorldStore::new();
    world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:notice")),
                entry("title", NbtSummaryValue::string("Item Body")),
                entry(
                    "body",
                    NbtSummaryValue::Compound(vec![
                        entry("type", NbtSummaryValue::string("minecraft:item")),
                        entry(
                            "item",
                            NbtSummaryValue::Compound(vec![
                                entry("id", NbtSummaryValue::string("minecraft:stick")),
                                entry("count", NbtSummaryValue::Int(3)),
                            ]),
                        ),
                        entry(
                            "description",
                            NbtSummaryValue::Compound(vec![
                                entry("contents", NbtSummaryValue::string("Info")),
                                entry("width", NbtSummaryValue::Int(50)),
                            ]),
                        ),
                        entry("width", NbtSummaryValue::Int(32)),
                        entry("height", NbtSummaryValue::Int(48)),
                    ]),
                ),
            ]),
        },
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    let item_runtime = recipe_book_ghost_item_runtime().with_vanilla_item_registry_for_test();
    let terrain_textures = TerrainTextureState::default();
    let surface_size = winit::dpi::PhysicalSize::new(320, 240);

    let screen = dialog_screen::hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        Some(&item_runtime),
        &terrain_textures,
        &dialog_screen::DialogHudLocalState {
            cursor_position: Some((120, 65)),
            ..Default::default()
        },
        surface_size,
        0.0,
    )
    .unwrap();

    assert_eq!(screen.floating_items.len(), 1);
    let item = &screen.floating_items[0];
    assert_eq!((item.x, item.y), (118, 63));
    assert!(item.draw_decorations);
    assert_eq!(
        item.icon
            .count_label
            .as_ref()
            .map(|label| label.text.as_str()),
        Some("3")
    );
    assert!(item.block_model.is_none());
    assert!(screen
        .scroll_area
        .as_ref()
        .unwrap()
        .text_labels
        .iter()
        .any(|label| { label.text == "Info" && (label.x, label.y, label.width) == (165, 83, 24) }));
    let tooltip = screen.tooltip.as_ref().unwrap();
    assert_eq!((tooltip.x, tooltip.y), (120, 65));
    assert_eq!(tooltip.lines[0].text, "Stick");

    assert!(input.focus_dialog_body_item(0));
    let focused = dialog_screen::hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        Some(&item_runtime),
        &terrain_textures,
        &dialog_screen::DialogHudLocalState::default(),
        surface_size,
        0.0,
    )
    .unwrap();
    assert_eq!(
        focused
            .scroll_area
            .as_ref()
            .unwrap()
            .fill_layers
            .iter()
            .map(|layer| {
                (
                    layer.x,
                    layer.y,
                    layer.width,
                    layer.height,
                    layer.tint,
                    layer.stage,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (
                118,
                63,
                32,
                1,
                [1.0, 1.0, 1.0, 1.0],
                HudInventoryFillStage::Foreground
            ),
            (
                118,
                110,
                32,
                1,
                [1.0, 1.0, 1.0, 1.0],
                HudInventoryFillStage::Foreground
            ),
            (
                118,
                64,
                1,
                46,
                [1.0, 1.0, 1.0, 1.0],
                HudInventoryFillStage::Foreground
            ),
            (
                149,
                64,
                1,
                46,
                [1.0, 1.0, 1.0, 1.0],
                HudInventoryFillStage::Foreground
            ),
        ]
    );

    let mut patched_world = WorldStore::new();
    patched_world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:notice")),
                entry("title", NbtSummaryValue::string("Patched")),
                entry(
                    "body",
                    NbtSummaryValue::Compound(vec![
                        entry("type", NbtSummaryValue::string("minecraft:item")),
                        entry(
                            "item",
                            NbtSummaryValue::Compound(vec![
                                entry("id", NbtSummaryValue::string("minecraft:stick")),
                                entry(
                                    "components",
                                    NbtSummaryValue::Compound(vec![
                                        entry(
                                            "minecraft:custom_name",
                                            NbtSummaryValue::string("Named"),
                                        ),
                                        entry(
                                            "minecraft:suspicious_stew_effects",
                                            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(
                                                vec![
                                                    entry(
                                                        "id",
                                                        NbtSummaryValue::string("minecraft:speed"),
                                                    ),
                                                    entry("duration", NbtSummaryValue::Int(160)),
                                                ],
                                            )]),
                                        ),
                                        entry(
                                            "minecraft:food",
                                            NbtSummaryValue::Compound(vec![
                                                entry("nutrition", NbtSummaryValue::Int(4)),
                                                entry(
                                                    "saturation",
                                                    NbtSummaryValue::Float(0.6f32.to_bits()),
                                                ),
                                            ]),
                                        ),
                                        entry(
                                            "minecraft:use_cooldown",
                                            NbtSummaryValue::Compound(vec![entry(
                                                "seconds",
                                                NbtSummaryValue::Float(1.25f32.to_bits()),
                                            )]),
                                        ),
                                        entry(
                                            "minecraft:break_sound",
                                            NbtSummaryValue::string("minecraft:entity.item.break"),
                                        ),
                                        entry(
                                            "minecraft:equippable",
                                            NbtSummaryValue::Compound(vec![entry(
                                                "slot",
                                                NbtSummaryValue::string("head"),
                                            )]),
                                        ),
                                        entry(
                                            "minecraft:blocks_attacks",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:consumable",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:death_protection",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:tool",
                                            NbtSummaryValue::Compound(vec![entry(
                                                "rules",
                                                NbtSummaryValue::List(Vec::new()),
                                            )]),
                                        ),
                                        entry(
                                            "minecraft:potion_contents",
                                            NbtSummaryValue::string("minecraft:water"),
                                        ),
                                        entry(
                                            "minecraft:enchantments",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:stored_enchantments",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:attribute_modifiers",
                                            NbtSummaryValue::List(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:can_break",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:can_place_on",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:writable_book_content",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:written_book_content",
                                            NbtSummaryValue::Compound(vec![
                                                entry("title", NbtSummaryValue::string("")),
                                                entry("author", NbtSummaryValue::string("")),
                                            ]),
                                        ),
                                        entry(
                                            "minecraft:entity_data",
                                            NbtSummaryValue::Compound(vec![entry(
                                                "id",
                                                NbtSummaryValue::string("minecraft:pig"),
                                            )]),
                                        ),
                                        entry(
                                            "minecraft:bucket_entity_data",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:block_entity_data",
                                            NbtSummaryValue::Compound(vec![entry(
                                                "id",
                                                NbtSummaryValue::string("minecraft:chest"),
                                            )]),
                                        ),
                                        entry(
                                            "minecraft:use_remainder",
                                            NbtSummaryValue::string("minecraft:glass_bottle"),
                                        ),
                                        entry(
                                            "minecraft:charged_projectiles",
                                            NbtSummaryValue::List(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:bundle_contents",
                                            NbtSummaryValue::List(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:container",
                                            NbtSummaryValue::List(Vec::new()),
                                        ),
                                        entry("minecraft:bees", NbtSummaryValue::List(Vec::new())),
                                        entry(
                                            "minecraft:map_decorations",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:debug_stick_state",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:recipes",
                                            NbtSummaryValue::List(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:container_loot",
                                            NbtSummaryValue::Compound(vec![entry(
                                                "loot_table",
                                                NbtSummaryValue::string("minecraft:empty"),
                                            )]),
                                        ),
                                        entry(
                                            "minecraft:pot_decorations",
                                            NbtSummaryValue::List(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:lock",
                                            NbtSummaryValue::Compound(Vec::new()),
                                        ),
                                        entry(
                                            "minecraft:instrument",
                                            NbtSummaryValue::Compound(vec![
                                                entry(
                                                    "sound_event",
                                                    NbtSummaryValue::string(
                                                        "minecraft:block.note_block.harp",
                                                    ),
                                                ),
                                                entry("use_duration", NbtSummaryValue::Int(1)),
                                                entry("range", NbtSummaryValue::Int(1)),
                                                entry(
                                                    "description",
                                                    NbtSummaryValue::string("Horn"),
                                                ),
                                            ]),
                                        ),
                                        entry(
                                            "minecraft:provides_trim_material",
                                            NbtSummaryValue::Compound(vec![
                                                entry(
                                                    "asset_name",
                                                    NbtSummaryValue::string("custom"),
                                                ),
                                                entry(
                                                    "description",
                                                    NbtSummaryValue::string("Custom material"),
                                                ),
                                            ]),
                                        ),
                                        entry(
                                            "minecraft:trim",
                                            NbtSummaryValue::Compound(vec![
                                                entry(
                                                    "material",
                                                    NbtSummaryValue::Compound(vec![
                                                        entry(
                                                            "asset_name",
                                                            NbtSummaryValue::string("custom"),
                                                        ),
                                                        entry(
                                                            "description",
                                                            NbtSummaryValue::string(
                                                                "Custom material",
                                                            ),
                                                        ),
                                                    ]),
                                                ),
                                                entry(
                                                    "pattern",
                                                    NbtSummaryValue::Compound(vec![
                                                        entry(
                                                            "asset_id",
                                                            NbtSummaryValue::string(
                                                                "example:pattern",
                                                            ),
                                                        ),
                                                        entry(
                                                            "description",
                                                            NbtSummaryValue::string(
                                                                "Custom pattern",
                                                            ),
                                                        ),
                                                    ]),
                                                ),
                                            ]),
                                        ),
                                    ]),
                                ),
                            ]),
                        ),
                    ]),
                ),
            ]),
        },
    });
    input.apply_screen_requests(&mut patched_world, &mut NetCounters::default(), &None);
    assert_eq!(input.current_dialog().unwrap().semantic_error, None);
    let patched = dialog_screen::hud_dialog_screen(
        &input,
        &patched_world,
        input.current_dialog().unwrap(),
        Some(&item_runtime),
        &terrain_textures,
        &dialog_screen::DialogHudLocalState {
            cursor_position: Some((153, 64)),
            ..Default::default()
        },
        surface_size,
        0.0,
    )
    .unwrap();
    assert_eq!(patched.floating_items.len(), 1);
    assert_eq!(patched.floating_items[0].icon.count_label, None);
    assert_eq!(patched.tooltip.as_ref().unwrap().lines[0].text, "Named");
}

#[test]
fn hud_inventory_screen_resolves_referenced_dialog_in_session_holder_order() {
    let mut input = crate::input::ClientInputState::new(true);
    fn network_notice_dialog(title: &str) -> Vec<u8> {
        fn write_string_tag(payload: &mut Vec<u8>, name: &str, value: &str) {
            payload.push(8);
            payload.extend_from_slice(&(name.len() as u16).to_be_bytes());
            payload.extend_from_slice(name.as_bytes());
            payload.extend_from_slice(&(value.len() as u16).to_be_bytes());
            payload.extend_from_slice(value.as_bytes());
        }

        let mut payload = vec![10];
        write_string_tag(&mut payload, "type", "minecraft:notice");
        write_string_tag(&mut payload, "title", title);
        payload.push(0);
        payload
    }

    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:dialog",
        0,
        vec![
            RegistryPacketEntry::stub("example:decoy"),
            RegistryPacketEntry::with_raw_data(
                "example:reordered_notice",
                network_notice_dialog("Reordered Notice"),
            ),
        ],
    );
    world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Reference { registry_id: 1 },
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);

    assert_eq!(
        input.current_dialog().unwrap().resource_id.as_deref(),
        Some("example:reordered_notice")
    );
    let screen = dialog_screen::hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &TerrainTextureState::default(),
        &dialog_screen::DialogHudLocalState::default(),
        winit::dpi::PhysicalSize::new(320, 240),
        0.0,
    )
    .unwrap();
    assert_eq!(screen.text_labels[0].text, "Reordered Notice");
}

#[test]
fn hud_inventory_screen_hides_unresolved_dialog_instead_of_falling_through() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Reference { registry_id: 99 },
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);

    assert!(dialog_screen::hud_dialog_screen(
        &input,
        &world,
        input.current_dialog().unwrap(),
        None,
        &TerrainTextureState::default(),
        &dialog_screen::DialogHudLocalState::default(),
        winit::dpi::PhysicalSize::new(320, 240),
        0.0,
    )
    .is_none());
}

#[test]
fn hud_inventory_screen_projects_empty_advancements_screen() {
    let world = WorldStore::new();
    let terrain_textures = TerrainTextureState::default();
    let surface_size = winit::dpi::PhysicalSize::new(800, 600);
    let local_state = InventoryHudLocalState {
        cursor_position: Some((400, 580)),
        ..Default::default()
    };
    let (window_x, window_y) = advancements_window_origin_for_surface(surface_size);
    let (done_button_x, done_button_y) = advancements_done_button_origin_for_surface(surface_size);

    let screen = hud_advancements_screen(
        &world,
        None,
        &terrain_textures,
        None,
        0.0,
        surface_size,
        local_state.cursor_position,
        Default::default(),
        0.0,
        &approximate_inventory_hud_fonts(),
    );

    assert_eq!(screen.width, 800);
    assert_eq!(screen.height, 600);
    assert_eq!((window_x, window_y), (274, 230));
    assert_eq!((done_button_x, done_button_y), (300, 574));
    for (x, y, expected) in [
        (299.9, 580.0, false),
        (300.0, 580.0, true),
        (499.9, 580.0, true),
        (500.0, 580.0, false),
        (400.0, 573.9, false),
        (400.0, 574.0, true),
        (400.0, 593.9, true),
        (400.0, 594.0, false),
    ] {
        assert_eq!(
            crate::input::AdvancementsScreen::done_button_contains(
                Some(PhysicalPosition::new(x, y)),
                surface_size,
            ),
            expected,
            "done button hit at ({x}, {y})"
        );
    }
    assert!(screen.slots.is_empty());
    assert!(screen.floating_items.is_empty());
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/advancements/window.png"),
                window_x,
                window_y,
                ADVANCEMENTS_WINDOW_WIDTH,
                ADVANCEMENTS_WINDOW_HEIGHT,
                [0.0, 0.0],
                [
                    ADVANCEMENTS_WINDOW_WIDTH as f32 / 256.0,
                    ADVANCEMENTS_WINDOW_HEIGHT as f32 / 256.0,
                ],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("widget/button_highlighted"),
                done_button_x,
                done_button_y,
                ADVANCEMENTS_DONE_BUTTON_WIDTH,
                ADVANCEMENTS_DONE_BUTTON_HEIGHT,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(
        screen.fill_layers,
        vec![HudInventoryFillLayer {
            x: window_x + ADVANCEMENTS_WINDOW_INSIDE_X,
            y: window_y + ADVANCEMENTS_WINDOW_INSIDE_Y,
            width: ADVANCEMENTS_WINDOW_INSIDE_WIDTH,
            height: ADVANCEMENTS_WINDOW_INSIDE_HEIGHT,
            tint: ADVANCEMENTS_EMPTY_BACKGROUND_TINT,
            stage: HudInventoryFillStage::BeforeGhostItem,
        }]
    );
    assert_eq!(
        screen.text_labels,
        vec![
            HudInventoryTextLabel {
                component_runs: vec![HudComponentTextRun::plain(ADVANCEMENTS_TITLE_TEXT)],
                x: window_x + ADVANCEMENTS_WINDOW_TITLE_X,
                y: window_y + ADVANCEMENTS_WINDOW_TITLE_Y,
                width: hud_ascii_approx_text_width(ADVANCEMENTS_TITLE_TEXT).unwrap(),
                text: ADVANCEMENTS_TITLE_TEXT.to_string(),
                tint: ADVANCEMENTS_TITLE_TEXT_COLOR,
                background: None,
                input: None,
                scrolling: None,
                shadow: false,
                font: None,
                runs: Vec::new(),
            },
            advancements_centered_text_label(
                ADVANCEMENTS_EMPTY_TEXT,
                window_x + ADVANCEMENTS_EMPTY_TEXT_CENTER_X,
                window_y + ADVANCEMENTS_EMPTY_TEXT_Y,
                ADVANCEMENTS_EMPTY_TEXT_COLOR,
            ),
            advancements_centered_text_label(
                ADVANCEMENTS_SAD_TEXT,
                window_x + ADVANCEMENTS_EMPTY_TEXT_CENTER_X,
                window_y + ADVANCEMENTS_SAD_TEXT_Y,
                ADVANCEMENTS_EMPTY_TEXT_COLOR,
            ),
            advancements_centered_text_label(
                ADVANCEMENTS_DONE_TEXT,
                done_button_x + ADVANCEMENTS_DONE_BUTTON_WIDTH as i32 / 2,
                done_button_y + ADVANCEMENTS_DONE_BUTTON_TEXT_Y_OFFSET,
                ADVANCEMENTS_EMPTY_TEXT_COLOR,
            ),
            HudInventoryTextLabel {
                shadow: true,
                ..advancements_centered_text_label(
                    ADVANCEMENTS_TITLE_TEXT,
                    400,
                    12,
                    ADVANCEMENTS_EMPTY_TEXT_COLOR,
                )
            },
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_advancement_root_tabs() {
    let mut world = WorldStore::new();
    let mut selected_root = runtime_displayed_advancement("minecraft:y/root", None);
    selected_root.display.as_mut().unwrap().background =
        Some("minecraft:gui/advancements/backgrounds/stone".to_string());
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![
            selected_root,
            runtime_displayed_advancement("minecraft:a/root", None),
        ],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    assert_eq!(
        world.ensure_advancements_screen_selected_tab(),
        Some("minecraft:y/root".to_string())
    );
    let terrain_textures = TerrainTextureState::default();
    let surface_size = winit::dpi::PhysicalSize::new(800, 600);
    let local_state = InventoryHudLocalState {
        ..Default::default()
    };
    let (window_x, window_y) = advancements_window_origin_for_surface(surface_size);
    let (done_button_x, done_button_y) = advancements_done_button_origin_for_surface(surface_size);

    let screen = hud_advancements_screen(
        &world,
        None,
        &terrain_textures,
        None,
        0.0,
        surface_size,
        local_state.cursor_position,
        Default::default(),
        0.0,
        &approximate_inventory_hud_fonts(),
    );

    assert_eq!(
        screen.background_layers[0],
        hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Texture("textures/gui/advancements/window.png"),
            window_x,
            window_y,
            ADVANCEMENTS_WINDOW_WIDTH,
            ADVANCEMENTS_WINDOW_HEIGHT,
            [0.0, 0.0],
            [
                ADVANCEMENTS_WINDOW_WIDTH as f32 / 256.0,
                ADVANCEMENTS_WINDOW_HEIGHT as f32 / 256.0,
            ],
        )
    );
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let background_tiles: Vec<_> = screen
        .background_layers
        .iter()
        .filter(|layer| {
            layer.texture
                == HudInventoryBackgroundTexture::advancement_background(
                    HudAdvancementBackgroundTexture::Stone,
                )
        })
        .collect();
    assert_eq!(background_tiles.len(), 128);
    assert_eq!(
        *background_tiles[0],
        hud_inventory_background_layer(
            HudInventoryBackgroundTexture::advancement_background(
                HudAdvancementBackgroundTexture::Stone,
            ),
            inside_x,
            inside_y,
            7,
            11,
            [9.0 / 16.0, 5.0 / 16.0],
            [1.0, 1.0],
        )
    );
    assert!(background_tiles.iter().all(|layer| {
        layer.x >= inside_x
            && layer.y >= inside_y
            && layer.x + layer.width as i32 <= inside_x + ADVANCEMENTS_WINDOW_INSIDE_WIDTH as i32
            && layer.y + layer.height as i32 <= inside_y + ADVANCEMENTS_WINDOW_INSIDE_HEIGHT as i32
    }));
    let done_button_layer = hud_inventory_background_layer(
        HudInventoryBackgroundTexture::Sprite("widget/button"),
        done_button_x,
        done_button_y,
        ADVANCEMENTS_DONE_BUTTON_WIDTH,
        ADVANCEMENTS_DONE_BUTTON_HEIGHT,
        [0.0, 0.0],
        [1.0, 1.0],
    );
    let selected_tab_layer = hud_inventory_background_layer(
        HudInventoryBackgroundTexture::Sprite(
            HudAdvancementTabSprite::AboveLeftSelected.sprite_path(),
        ),
        window_x,
        window_y - 28,
        28,
        32,
        [0.0, 0.0],
        [1.0, 1.0],
    );
    let second_tab_layer = hud_inventory_background_layer(
        HudInventoryBackgroundTexture::Sprite(HudAdvancementTabSprite::AboveMiddle.sprite_path()),
        window_x + 32,
        window_y - 28,
        28,
        32,
        [0.0, 0.0],
        [1.0, 1.0],
    );
    let widget_frame_layer = hud_inventory_background_layer(
        HudInventoryBackgroundTexture::Sprite(
            HudAdvancementWidgetFrameSprite::TaskUnobtained.sprite_path(),
        ),
        inside_x + 106,
        inside_y + 43,
        26,
        26,
        [0.0, 0.0],
        [1.0, 1.0],
    );
    assert!(screen.background_layers.contains(&done_button_layer));
    assert!(screen.background_layers.contains(&selected_tab_layer));
    assert!(screen.background_layers.contains(&second_tab_layer));
    assert!(screen.background_layers.contains(&widget_frame_layer));
    let first_background_index = screen
        .background_layers
        .iter()
        .position(|layer| {
            matches!(
                layer.texture,
                HudInventoryBackgroundTexture::Texture(path)
                    if path.starts_with("textures/gui/advancements/backgrounds/")
            )
        })
        .unwrap();
    let widget_index = screen
        .background_layers
        .iter()
        .position(|layer| *layer == widget_frame_layer)
        .unwrap();
    assert!(first_background_index < widget_index);
    assert_eq!(
        screen.background_layers.len(),
        1 + background_tiles.len() + 1 + 2 + 1
    );
    assert!(screen.fill_layers.is_empty());
    assert!(screen.floating_items.is_empty());
    assert_eq!(
        screen.text_labels,
        vec![
            HudInventoryTextLabel {
                component_runs: vec![HudComponentTextRun::plain("minecraft:y/root")],
                x: window_x + ADVANCEMENTS_WINDOW_TITLE_X,
                y: window_y + ADVANCEMENTS_WINDOW_TITLE_Y,
                width: hud_ascii_approx_text_width("minecraft:y/root").unwrap(),
                text: "minecraft:y/root".to_string(),
                tint: ADVANCEMENTS_TITLE_TEXT_COLOR,
                background: None,
                input: None,
                scrolling: None,
                shadow: false,
                font: None,
                runs: Vec::new(),
            },
            advancements_centered_text_label(
                ADVANCEMENTS_DONE_TEXT,
                done_button_x + ADVANCEMENTS_DONE_BUTTON_WIDTH as i32 / 2,
                done_button_y + ADVANCEMENTS_DONE_BUTTON_TEXT_Y_OFFSET,
                ADVANCEMENTS_EMPTY_TEXT_COLOR,
            ),
            HudInventoryTextLabel {
                shadow: true,
                ..advancements_centered_text_label(
                    ADVANCEMENTS_TITLE_TEXT,
                    400,
                    12,
                    ADVANCEMENTS_EMPTY_TEXT_COLOR,
                )
            },
        ]
    );
}

#[test]
fn advancement_widget_connection_layers_match_vanilla_two_pass_lines() {
    let surface_size = winit::dpi::PhysicalSize::new(800, 600);
    let (window_x, window_y) = advancements_window_origin_for_surface(surface_size);
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let widgets = vec![
        advancement_widget_summary_for_test("minecraft:story/root", None, 0, 0),
        advancement_widget_summary_for_test(
            "minecraft:story/mine_stone",
            Some("minecraft:story/root"),
            56,
            27,
        ),
    ];

    let layers = advancements_widget_connection_layers(&widgets, None, window_x, window_y);

    assert_eq!(
        layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Background,
                ),
                inside_x + 88,
                inside_y + 41,
                18,
                1,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Background,
                ),
                inside_x + 88,
                inside_y + 42,
                19,
                1,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Background,
                ),
                inside_x + 88,
                inside_y + 43,
                18,
                1,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Background,
                ),
                inside_x + 104,
                inside_y + 68,
                41,
                1,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Background,
                ),
                inside_x + 104,
                inside_y + 69,
                41,
                1,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Background,
                ),
                inside_x + 104,
                inside_y + 70,
                41,
                1,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Background,
                ),
                inside_x + 104,
                inside_y + 43,
                1,
                26,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Background,
                ),
                inside_x + 106,
                inside_y + 43,
                1,
                26,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Foreground,
                ),
                inside_x + 88,
                inside_y + 42,
                18,
                1,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Foreground,
                ),
                inside_x + 105,
                inside_y + 69,
                40,
                1,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::AdvancementLine(
                    HudAdvancementLineTexture::Foreground,
                ),
                inside_x + 105,
                inside_y + 43,
                1,
                26,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
}

#[test]
fn advancement_widget_scroll_applies_local_delta_and_vanilla_clamp() {
    let widgets = vec![
        advancement_widget_summary_for_test("minecraft:story/root", None, 0, 0),
        advancement_widget_summary_for_test(
            "minecraft:story/tall_child",
            Some("minecraft:story/root"),
            0,
            135,
        ),
    ];

    assert_eq!(advancements_widget_scroll(&widgets, None), Some((103, -25)));
    assert_eq!(
        advancements_widget_scroll(&widgets, Some((64.0, -16.0))),
        Some((103, -41))
    );
    assert_eq!(
        advancements_widget_scroll(&widgets, Some((0.0, -100.0))),
        Some((103, -49))
    );
    assert_eq!(
        advancements_widget_scroll(&widgets, Some((0.0, 100.0))),
        Some((103, 0))
    );
}

#[test]
fn advancement_widget_frame_layer_clips_to_contents_with_uvs() {
    let surface_size = winit::dpi::PhysicalSize::new(800, 600);
    let (window_x, window_y) = advancements_window_origin_for_surface(surface_size);
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;

    let layer = clipped_advancement_widget_frame_layer(
        HudAdvancementWidgetFrameSprite::TaskUnobtained,
        inside_x - 5,
        inside_y + 2,
        inside_x,
        inside_y,
    )
    .unwrap();

    assert_eq!(
        layer,
        hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite(
                HudAdvancementWidgetFrameSprite::TaskUnobtained.sprite_path()
            ),
            inside_x,
            inside_y + 2,
            21,
            26,
            [5.0 / 26.0, 0.0],
            [1.0, 1.0],
        )
    );
}

#[test]
fn advancement_widget_icon_items_keep_partial_fake_item_with_content_scissor() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let world = WorldStore::new();
    let terrain_textures = TerrainTextureState::default();
    let surface_size = winit::dpi::PhysicalSize::new(800, 600);
    let (window_x, window_y) = advancements_window_origin_for_surface(surface_size);
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let widgets = vec![
        advancement_widget_summary_for_test("minecraft:story/root", None, 0, 0),
        advancement_widget_summary_for_test("minecraft:story/far_child", None, 400, 0),
    ];

    let items = advancements_widget_icon_items(
        &world,
        Some(&item_runtime),
        &terrain_textures,
        &widgets,
        Some((81.0, 0.0)),
        window_x,
        window_y,
        ItemModelKeybindContext::default(),
        0.0,
    );

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].x, inside_x - 8);
    assert_eq!(items[0].y, inside_y + 48);
    assert_eq!(
        items[0].scissor,
        Some(HudInventoryItemScissor {
            x: inside_x,
            y: inside_y,
            width: ADVANCEMENTS_WINDOW_INSIDE_WIDTH,
            height: ADVANCEMENTS_WINDOW_INSIDE_HEIGHT,
        })
    );
    assert!(!items[0].draw_decorations);
}

#[test]
fn hud_inventory_screen_projects_advancement_hover_tooltip() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = WorldStore::new();
    let mut root = runtime_displayed_advancement("minecraft:story/root", None);
    root.requirements = vec![vec!["has_log".into()], vec!["has_planks".into()]];
    let display = root.display.as_mut().unwrap();
    display.title = *styled_boss_component("Getting Wood");
    display.description = *styled_boss_component("Punch a tree");
    display.frame_type = AdvancementFrameType::Challenge;
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![root],
        removed: Vec::new(),
        progress: vec![AdvancementProgressSummary {
            id: "minecraft:story/root".to_string(),
            criteria: vec![
                AdvancementCriterionProgressSummary {
                    name: "has_log".into(),
                    obtained_epoch_millis: Some(1),
                },
                AdvancementCriterionProgressSummary {
                    name: "has_planks".into(),
                    obtained_epoch_millis: None,
                },
            ],
        }],
        show_advancements: false,
    });
    assert_eq!(
        world.ensure_advancements_screen_selected_tab(),
        Some("minecraft:story/root".to_string())
    );
    let terrain_textures = TerrainTextureState::default();
    let surface_size = winit::dpi::PhysicalSize::new(800, 600);
    let (window_x, window_y) = advancements_window_origin_for_surface(surface_size);
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let local_state = InventoryHudLocalState {
        cursor_position: Some((inside_x + 110, inside_y + 50)),
        ..Default::default()
    };

    let screen = hud_advancements_screen(
        &world,
        Some(&item_runtime),
        &terrain_textures,
        None,
        ADVANCEMENTS_HOVER_MAX_FADE,
        surface_size,
        local_state.cursor_position,
        Default::default(),
        0.0,
        &approximate_inventory_hud_fonts(),
    );

    assert!(screen.fill_layers.contains(&HudInventoryFillLayer {
        x: inside_x,
        y: inside_y,
        width: ADVANCEMENTS_WINDOW_INSIDE_WIDTH,
        height: ADVANCEMENTS_WINDOW_INSIDE_HEIGHT,
        tint: ADVANCEMENTS_HOVER_FADE_TINT,
        stage: HudInventoryFillStage::Foreground,
    }));
    assert!(screen.foreground_layers.iter().any(|layer| {
        layer.texture
            == HudInventoryBackgroundTexture::Sprite(
                HudAdvancementHoverBoxSprite::Title.sprite_path(),
            )
    }));
    assert!(screen.foreground_layers.iter().any(|layer| {
        layer.texture
            == HudInventoryBackgroundTexture::Sprite(
                HudAdvancementHoverBoxSprite::Obtained.sprite_path(),
            )
    }));
    assert!(screen.foreground_layers.iter().any(|layer| {
        layer.texture
            == HudInventoryBackgroundTexture::Sprite(
                HudAdvancementHoverBoxSprite::Unobtained.sprite_path(),
            )
    }));
    assert!(screen.foreground_layers.iter().any(|layer| {
        layer.texture
            == HudInventoryBackgroundTexture::Sprite(
                HudAdvancementWidgetFrameSprite::ChallengeUnobtained.sprite_path(),
            )
    }));
    let title = screen
        .text_labels
        .iter()
        .find(|label| label.text == "Getting Wood")
        .unwrap();
    assert!(title.component_runs[0].visual.style.bold);
    assert_eq!(title.component_runs[0].visual.color, Some(0xFF_55_55));
    assert!(screen.text_labels.iter().any(|label| label.text == "1/2"));
    let description = screen
        .text_labels
        .iter()
        .find(|label| label.text == "Punch a tree")
        .unwrap();
    assert_eq!(
        description.tint,
        ADVANCEMENTS_HOVER_CHALLENGE_DESCRIPTION_TEXT_COLOR
    );
    assert!(description.component_runs[0].visual.style.bold);
    assert_eq!(description.component_runs[0].visual.color, Some(0xFF_55_55));
    assert_eq!(screen.foreground_items.len(), 1);
    assert_eq!(screen.foreground_items[0].x, inside_x + 111);
    assert_eq!(screen.foreground_items[0].y, inside_y + 48);
    assert_eq!(screen.foreground_items[0].scissor, None);
    assert!(!screen.foreground_items[0].draw_decorations);
}

#[test]
fn hud_inventory_screen_projects_shulker_box_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SHULKER_BOX_ID,
        title: bbb_protocol::RichTextComponent::plain("Shulker Box"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 63],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(62), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 167);
    assert_eq!(
        screen.background_layers,
        vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Texture("textures/gui/container/shulker_box.png"),
            0,
            0,
            176,
            167,
            [0.0, 0.0],
            [176.0 / 256.0, 167.0 / 256.0],
        )]
    );
    assert_eq!(screen.hovered_slot_id, Some(62));
    assert_eq!(screen.slots.len(), 63);
    let first_container = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((first_container.x, first_container.y), (8, 18));
    let last_container = screen.slots.iter().find(|slot| slot.slot_id == 26).unwrap();
    assert_eq!((last_container.x, last_container.y), (152, 54));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 62).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_loom_layout_and_empty_slot_layers() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 40],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(39), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/loom.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/slot/banner"),
                13,
                26,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/slot/dye"),
                33,
                26,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/slot/banner_pattern"),
                23,
                45,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/loom/scroller_disabled"),
                119,
                13,
                12,
                15,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(39));
    assert_eq!(screen.slots.len(), 40);
    let banner = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((banner.x, banner.y), (13, 26));
    let dye = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((dye.x, dye.y), (33, 26));
    let pattern = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
    assert_eq!((pattern.x, pattern.y), (23, 45));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 3).unwrap();
    assert_eq!((result.x, result.y), (143, 57));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 39).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_loom_pattern_grid_and_scroller() {
    let mut world = WorldStore::new();
    world.record_registry_data(bbb_protocol::packets::RegistryData {
        registry: "minecraft:banner_pattern".to_string(),
        entries: (0..32)
            .map(|index| bbb_protocol::packets::RegistryDataEntry {
                id: format!("example:pattern_{index}"),
                raw_data: None,
            })
            .collect(),
        raw_payload_len: 0,
    });
    world.apply_update_tags(bbb_protocol::packets::UpdateTags {
        registries: vec![bbb_protocol::packets::RegistryTags {
            registry: "minecraft:banner_pattern".to_string(),
            tags: vec![bbb_protocol::packets::TagNetworkPayload {
                tag: "minecraft:no_item_required".to_string(),
                entries: (0..32).collect(),
            }],
        }],
    });
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 40];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            loom_pattern_scroll_row: Some(2),
            loom_selected_pattern_index: Some(10),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(screen.background_layers.len(), 19);
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/loom/scroller"),
            119,
            33,
            12,
            15,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/loom/pattern"),
            60,
            13,
            14,
            14,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/loom/pattern_selected"),
            88,
            13,
            14,
            14,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/loom/pattern"),
            102,
            55,
            14,
            14,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
}

#[test]
fn hud_inventory_screen_projects_loom_result_banner_pip() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut result = item_stack(bbb_protocol::item_types::VANILLA_ITEM_RED_BANNER_ID, 1);
    result
        .component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID);
    result.component_patch.banner_pattern_layers =
        vec![bbb_protocol::packets::BannerPatternLayerSummary {
            registry_id: None,
            asset_id: Some("minecraft:stripe_top".to_string()),
            translation_key: Some("block.minecraft.banner.stripe_top".into()),
            color_id: VANILLA_DYE_COLOR_RED_ID,
        }];
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 40];
    items[3] = result;
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(screen.entity_previews.len(), 1);
    let preview = &screen.entity_previews[0];
    assert_eq!(preview.lighting, GuiItemLightingEntry::ItemsFlat);
    assert_eq!(
        preview.rect,
        HudEntityPreviewRect {
            x: 141,
            y: 8,
            width: 20,
            height: 40,
        }
    );
    assert_eq!(preview.translation, [0.0, 0.25, 0.0]);
    assert_eq!(preview.rotation, [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(preview.scale, 16.0);
    assert!(preview.depth_isolated);
    assert_eq!(preview.after_background_layer, Some(4));
    let EntityModelKind::Banner {
        wall,
        flag_only,
        base_color,
        layers,
    } = preview.entity.kind
    else {
        panic!("Loom result should use the banner flag model");
    };
    assert!(!wall);
    assert!(flag_only);
    assert_eq!(base_color, bbb_client_render::EntityDyeColor::Red);
    assert_eq!(
        layers[0],
        Some(bbb_client_render::BannerPatternLayer {
            texture: bbb_client_render::BannerPatternTexture::Builtin(
                bbb_client_render::BannerPatternKind::StripeTop,
            ),
            color: bbb_client_render::EntityDyeColor::Red,
        })
    );
}

#[test]
fn hud_inventory_screen_uses_the_loom_component_holder_set_size() {
    let mut world = WorldStore::new();
    world.record_registry_data(bbb_protocol::packets::RegistryData {
        registry: "minecraft:banner_pattern".to_string(),
        entries: (0..6)
            .map(|index| bbb_protocol::packets::RegistryDataEntry {
                id: format!("example:pattern_{index}"),
                raw_data: None,
            })
            .collect(),
        raw_payload_len: 0,
    });
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut pattern_stack = item_stack(44, 1);
    pattern_stack
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID);
    pattern_stack
        .component_patch
        .provides_banner_pattern_registry_ids = vec![5, 2];
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 40];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[2] = pattern_stack;
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            loom_selected_pattern_index: Some(1),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();
    let pattern_buttons = screen
        .background_layers
        .iter()
        .filter(|layer| {
            matches!(
                layer.texture,
                HudInventoryBackgroundTexture::Sprite("container/loom/pattern")
                    | HudInventoryBackgroundTexture::Sprite("container/loom/pattern_selected")
                    | HudInventoryBackgroundTexture::Sprite("container/loom/pattern_highlighted")
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(pattern_buttons.len(), 2);
    assert_eq!(
        pattern_buttons[0].texture,
        HudInventoryBackgroundTexture::Sprite("container/loom/pattern")
    );
    assert_eq!(
        pattern_buttons[1].texture,
        HudInventoryBackgroundTexture::Sprite("container/loom/pattern_selected")
    );
    assert!(screen.background_layers.iter().any(|layer| layer.texture
        == HudInventoryBackgroundTexture::Sprite("container/loom/scroller_disabled")));
}

#[test]
fn hud_inventory_screen_projects_merchant_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(38), 0.0).unwrap();

    assert_eq!(screen.width, 276);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Texture("textures/gui/container/villager.png"),
            0,
            0,
            276,
            166,
            [0.0, 0.0],
            [276.0 / 512.0, 166.0 / 256.0],
        )]
    );
    assert_eq!(screen.hovered_slot_id, Some(38));
    assert_eq!(screen.slots.len(), 39);
    assert!(screen.floating_items.is_empty());
    assert_eq!(
        screen
            .text_labels
            .iter()
            .map(|label| label.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Merchant", "Inventory", "Trades"]
    );
    assert_eq!(
        screen.text_labels[0].scrolling,
        Some(HudInventoryScrollingText {
            x: -69,
            y: 5,
            width: 512,
            height: 9,
        })
    );
    assert_eq!(
        screen.text_labels[2].scrolling,
        Some(HudInventoryScrollingText {
            x: -203,
            y: 5,
            width: 512,
            height: 9,
        })
    );
    let payment_a = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((payment_a.x, payment_a.y), (136, 37));
    let payment_b = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((payment_b.x, payment_b.y), (162, 37));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
    assert_eq!((result.x, result.y), (220, 37));
    let first_inventory = screen.slots.iter().find(|slot| slot.slot_id == 3).unwrap();
    assert_eq!((first_inventory.x, first_inventory.y), (108, 84));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 38).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (252, 142));
}

#[test]
fn hud_inventory_screen_projects_merchant_trade_layers() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    assert!(world.apply_merchant_offers(merchant_offers(7, 8, Some(2))));
    assert!(world.set_local_merchant_selected_offer(2));
    assert!(world.scroll_local_merchant_offers(1));

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        Some(38),
        InventoryHudLocalState {
            cursor_position: Some((6, 19)),
            merchant_focused_trade_row: Some(1),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(screen.width, 276);
    assert_eq!(screen.height, 166);
    assert!(screen.floating_items.is_empty());
    assert_eq!(screen.text_labels[0].text, "Merchant - Journeyman");
    assert_eq!(
        screen.background_layers[0],
        hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Texture("textures/gui/container/villager.png"),
            0,
            0,
            276,
            166,
            [0.0, 0.0],
            [276.0 / 512.0, 166.0 / 256.0],
        )
    );
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("widget/button_highlighted"),
            5,
            18,
            88,
            20,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("widget/button_highlighted"),
            5,
            38,
            88,
            20,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("widget/button"),
            5,
            58,
            88,
            20,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/villager/trade_arrow"),
            60,
            22,
            10,
            9,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/villager/trade_arrow_out_of_stock"),
            60,
            42,
            10,
            9,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .foreground_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/villager/discount_strikethrough"),
            17,
            31,
            9,
            2,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/villager/scroller"),
            94,
            131,
            6,
            27,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/villager/out_of_stock"),
            182,
            35,
            28,
            21,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/villager/experience_bar_background"),
            136,
            16,
            102,
            5,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/villager/experience_bar_current"),
            136,
            16,
            63,
            5,
            [0.0, 0.0],
            [63.0 / 102.0, 1.0],
        )));
}

#[test]
fn hud_inventory_screen_projects_merchant_future_xp_result() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[2] = item_stack(101, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    assert!(world.apply_merchant_offers(merchant_offers(7, 8, None)));
    assert!(world.set_local_merchant_selected_offer(2));

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/villager/experience_bar_result"),
            199,
            16,
            10,
            5,
            [63.0 / 102.0, 0.0],
            [73.0 / 102.0, 1.0],
        )));
}

#[test]
fn hud_inventory_screen_projects_merchant_component_cost_decorations_and_tooltips() {
    let root = unique_runtime_temp_dir("merchant-trade-presentation");
    write_runtime_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });
    let mut packet = merchant_offers(7, 1, None);
    let offer = &mut packet.offers[0];
    offer.buy_a = item_cost(1, 10);
    offer.buy_a.component_predicate.components.added_type_ids = vec![
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
    ];
    offer.buy_a.component_predicate.components.custom_name = Some("Predicate Cost".to_string());
    offer.buy_a.component_predicate.components.item_model =
        Some("minecraft:test_combo".to_string());
    offer.buy_b = Some(item_cost(0, 2));
    offer.sell = item_stack(0, 3);
    offer
        .sell
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID);
    offer.sell.component_patch.custom_name = Some("Trade Result".to_string());
    offer.sell.component_patch.max_damage = Some(20);
    offer.sell.component_patch.damage = Some(5);
    offer.special_price_diff = -2;
    offer.price_multiplier = 0.0;
    offer.demand = 0;
    let result = offer.sell.clone();
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[2] = result;
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    assert!(world.apply_merchant_offers(packet.clone()));

    let screen_at = |cursor_position| {
        hud_inventory_screen_with_local_state(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            None,
            InventoryHudLocalState {
                cursor_position: Some(cursor_position),
                ..InventoryHudLocalState::default()
            },
            0.0,
        )
        .unwrap()
    };
    let cost_a_screen = screen_at((10, 19));

    assert_eq!(cost_a_screen.floating_items.len(), 5);
    assert_eq!(
        cost_a_screen
            .floating_items
            .iter()
            .take(3)
            .map(|item| (item.x, item.y, item.draw_decorations))
            .collect::<Vec<_>>(),
        vec![(10, 19, false), (40, 19, true), (73, 19, true)]
    );
    assert!(!cost_a_screen.floating_items[0].icon.layers.is_empty());
    assert_eq!(
        cost_a_screen.floating_items[1]
            .icon
            .count_label
            .as_ref()
            .map(|label| label.text.as_str()),
        Some("2")
    );
    assert_eq!(
        cost_a_screen.floating_items[2]
            .icon
            .count_label
            .as_ref()
            .map(|label| label.text.as_str()),
        Some("3")
    );
    assert!(cost_a_screen.floating_items[2]
        .icon
        .durability_bar
        .is_some());
    assert!(cost_a_screen.foreground_items.is_empty());
    assert_eq!(
        cost_a_screen
            .floating_items
            .iter()
            .skip(3)
            .map(|item| {
                (
                    item.x,
                    item.y,
                    item.icon
                        .count_label
                        .as_ref()
                        .map(|label| label.text.as_str()),
                    item.icon.layers.is_empty(),
                    item.draw_decorations,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (10, 19, Some("10"), true, true),
            (24, 19, Some("8"), true, true),
        ]
    );
    assert_eq!(
        cost_a_screen.tooltip.as_ref().unwrap().lines[0].text,
        "Predicate Cost"
    );
    assert_eq!(
        screen_at((40, 19)).tooltip.unwrap().lines[0].text,
        "Test Combo"
    );
    assert_eq!(
        screen_at((73, 19)).tooltip.unwrap().lines[0].text,
        "Trade Result"
    );
    packet.offers[0].is_out_of_stock = true;
    packet.offers[0].uses = packet.offers[0].max_uses;
    assert!(world.apply_merchant_offers(packet));
    let deprecated = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some((186, 35)),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();
    assert!(!deprecated.background_layers.iter().any(|layer| {
        layer.texture
            == HudInventoryBackgroundTexture::Sprite("container/villager/experience_bar_result")
    }));
    let deprecated = deprecated.tooltip.unwrap();
    assert_eq!(
        (deprecated.slot_id, deprecated.x, deprecated.y),
        (u16::MAX, 186, 35)
    );
    assert_eq!(
        deprecated.lines,
        vec![HudInventoryTooltipLine {
            text: "Villagers restock up to two times per day.".to_string(),
            tint: TOOLTIP_TEST_WHITE,
            runs: Vec::new(),
        }]
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn merchant_offer_cost_a_stack_uses_vanilla_modified_cost_count() {
    let mut world = WorldStore::new();
    crate::item_registry_test_fixture::install_max_stack_size_default(&mut world, 42, 16);
    let mut buy_a = item_cost(42, 10);
    buy_a.component_predicate.components.added_type_ids = vec![
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
    ];
    buy_a.component_predicate.components.max_stack_size = Some(12);
    buy_a.component_predicate.components.item_model = Some("minecraft:emerald".to_string());
    let offer = bbb_client_world::MerchantOfferState {
        buy_a,
        sell: item_stack(99, 1),
        buy_b: None,
        is_out_of_stock: false,
        uses: 0,
        max_uses: 12,
        xp: 8,
        special_price_diff: 5,
        price_multiplier: 0.5,
        demand: 2,
    };

    let stack = merchant_offer_cost_a_stack(&world, &offer);
    assert_eq!(stack.item_id, Some(42));
    assert_eq!(stack.count, 12);
    assert_eq!(
        stack.component_patch.added_type_ids,
        vec![
            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
            VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
        ]
    );
    assert_eq!(stack.component_patch.max_stack_size, Some(12));
    assert_eq!(
        stack.component_patch.item_model.as_deref(),
        Some("minecraft:emerald")
    );
}

#[test]
fn hud_inventory_screen_projects_smithing_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 40],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(39), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Texture("textures/gui/container/smithing.png"),
            0,
            0,
            176,
            166,
            [0.0, 0.0],
            [176.0 / 256.0, 166.0 / 256.0],
        )]
    );
    assert_eq!(screen.hovered_slot_id, Some(39));
    assert_eq!(screen.slots.len(), 40);
    let template = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((template.x, template.y), (8, 48));
    let base = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((base.x, base.y), (26, 48));
    let additional = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
    assert_eq!((additional.x, additional.y), (44, 48));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 3).unwrap();
    assert_eq!((result.x, result.y), (98, 48));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 39).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_smithing_armor_stand_preview() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 40],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(screen.entity_previews.len(), 1);
    let preview = &screen.entity_previews[0];
    assert_eq!(preview.entity.entity_id, -1);
    assert_eq!(preview.entity.position, [0.0; 3]);
    match preview.entity.kind {
        bbb_client_render::EntityModelKind::ArmorStand {
            small,
            marker,
            show_arms,
            show_base_plate,
            pose,
        } => {
            assert!(!small);
            assert!(!marker);
            assert!(show_arms);
            assert!(!show_base_plate);
            assert_eq!(pose, DEFAULT_ARMOR_STAND_MODEL_POSE);
        }
        other => panic!("expected smithing armor stand preview, got {other:?}"),
    }
    assert_eq!(preview.lighting, GuiItemLightingEntry::EntityInUi);
    assert_eq!(
        preview.rect,
        HudEntityPreviewRect {
            x: 121,
            y: 20,
            width: 40,
            height: 60,
        }
    );
    assert_eq!(preview.scissor, None);
    assert_close3(preview.translation, [0.0, 1.0, 0.0]);
    assert_close4(
        preview.rotation,
        quaternion_mul(quaternion_x(0.43633232), quaternion_z(std::f32::consts::PI)),
    );
    assert_eq!(preview.override_camera_rotation, None);
    assert_eq!(preview.scale, 25.0);
    assert!(preview.depth_isolated);
    assert!((preview.entity.render_state.body_rot - 210.0).abs() < 1.0e-6);
    assert_eq!(preview.entity.render_state.head_yaw, 0.0);
    assert_eq!(preview.entity.render_state.head_pitch, 25.0);
    assert_eq!(
        preview.entity.render_state.light_coords,
        ENTITY_FULL_BRIGHT_LIGHT_COORDS
    );
    assert_eq!(preview.entity.render_state.outline_color, 0);
    assert!(!preview.entity.render_state.appears_glowing);
}

#[test]
fn hud_inventory_screen_projects_smithing_result_equipment_preview() {
    const HELMET_ID: i32 = 0;
    const CHESTPLATE_ID: i32 = 1;
    const LEGGINGS_ID: i32 = 2;
    const BOOTS_ID: i32 = 3;
    const ELYTRA_ID: i32 = 4;
    const SKELETON_SKULL_ID: i32 = 5;
    const CARVED_PUMPKIN_ID: i32 = 6;
    const IRON_SWORD_ID: i32 = 7;

    let runtime = smithing_preview_item_runtime();

    let mut enchanted_helmet = item_stack(HELMET_ID, 1);
    enchanted_helmet.component_patch.dyed_color = Some(0x12_34_56);
    enchanted_helmet.component_patch.enchantments.push(
        bbb_protocol::packets::ItemEnchantmentSummary {
            holder_id: 12,
            level: 1,
        },
    );
    let helmet_preview = smithing_preview_for_result_stack(enchanted_helmet, &runtime);
    let helmet_state = &helmet_preview.entity.render_state;
    assert!(helmet_state.head_equipment_asset.is_some());
    assert_eq!(
        helmet_state.head_equipment_asset,
        runtime.equipment_asset_id("minecraft:diamond"),
        "the preview publishes the generation-matched equipment asset"
    );
    assert_eq!(helmet_state.head_armor_dye, Some(0x12_34_56));
    assert!(helmet_state.head_armor_foil);
    assert_eq!(helmet_state.custom_head_skull, None);

    let chest_preview = smithing_preview_for_result_stack(item_stack(CHESTPLATE_ID, 1), &runtime);
    let chest_state = &chest_preview.entity.render_state;
    assert!(chest_state.chest_equipment_asset.is_some());
    assert_eq!(
        chest_state.chest_equipment_asset,
        runtime.equipment_asset_id("minecraft:diamond")
    );
    assert_eq!(
        chest_state.chest_wings_equipment_asset,
        runtime.equipment_asset_id("minecraft:diamond")
    );

    let legs_preview = smithing_preview_for_result_stack(item_stack(LEGGINGS_ID, 1), &runtime);
    assert_eq!(
        legs_preview.entity.render_state.legs_equipment_asset,
        runtime.equipment_asset_id("minecraft:diamond")
    );

    let feet_preview = smithing_preview_for_result_stack(item_stack(BOOTS_ID, 1), &runtime);
    assert_eq!(
        feet_preview.entity.render_state.feet_equipment_asset,
        runtime.equipment_asset_id("minecraft:diamond")
    );

    let elytra_preview = smithing_preview_for_result_stack(item_stack(ELYTRA_ID, 1), &runtime);
    let elytra_state = &elytra_preview.entity.render_state;
    assert!(elytra_state.chest_wings_equipment_asset.is_some());
    assert_eq!(
        elytra_state.chest_wings_equipment_asset,
        runtime.equipment_asset_id("minecraft:elytra"),
        "the elytra preview reaches the wings layer through the same asset"
    );
    assert_eq!(
        elytra_state.chest_equipment_asset,
        runtime.equipment_asset_id("minecraft:elytra")
    );
    assert!(!elytra_state.chest_armor_foil);

    let skull_preview =
        smithing_preview_for_result_stack(item_stack(SKELETON_SKULL_ID, 1), &runtime);
    let skull_state = &skull_preview.entity.render_state;
    assert_eq!(skull_state.head_equipment_asset, None);
    assert!(!skull_state.head_armor_foil);
    assert_eq!(
        skull_state.custom_head_skull,
        Some(bbb_client_render::EntityCustomHeadSkull::Skeleton)
    );
    assert!(skull_preview.item_layers.is_empty());

    let pumpkin_preview =
        smithing_preview_for_result_stack(item_stack(CARVED_PUMPKIN_ID, 1), &runtime);
    let pumpkin_state = &pumpkin_preview.entity.render_state;
    assert_eq!(pumpkin_state.head_equipment_asset, None);
    assert_eq!(pumpkin_state.custom_head_skull, None);
    assert_eq!(
        pumpkin_preview.item_layers,
        vec![HudEntityPreviewItemLayer {
            slot: HudEntityPreviewItemSlot::Head,
            display_context: HudEntityPreviewItemDisplayContext::Head,
            item_id: CARVED_PUMPKIN_ID,
            count: 1,
            foil: false,
            light_coords: ENTITY_FULL_BRIGHT_LIGHT_COORDS,
            overlay: ITEM_MODEL_NO_OVERLAY,
            order: 0,
            submit_sequence: 2,
        }]
    );

    let mut removed_equipment = item_stack(HELMET_ID, 1);
    removed_equipment.component_patch.removed_type_ids =
        vec![bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID];
    let removed_preview = smithing_preview_for_result_stack(removed_equipment, &runtime);
    assert_eq!(
        removed_preview.entity.render_state.head_equipment_asset,
        None
    );
    assert_eq!(
        removed_preview.item_layers[0].slot,
        HudEntityPreviewItemSlot::LeftHand
    );

    let mut defaults = WorldStore::new();
    crate::resource_reload::apply_item_runtime_world_defaults(&runtime, &mut defaults).unwrap();
    defaults.rebuild_item_component_defaults().unwrap();
    let mut overridden_equipment = item_stack(HELMET_ID, 1);
    overridden_equipment.component_patch.added_type_ids =
        vec![bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID];
    overridden_equipment.component_patch.equippable = defaults
        .effective_item_stack(&item_stack(ELYTRA_ID, 1))
        .component_patch
        .equippable
        .clone();
    let overridden_preview = smithing_preview_for_result_stack(overridden_equipment, &runtime);
    assert_eq!(
        overridden_preview.entity.render_state.head_equipment_asset,
        None
    );
    assert_eq!(
        overridden_preview
            .entity
            .render_state
            .chest_wings_equipment_asset,
        runtime.equipment_asset_id("minecraft:elytra")
    );

    let mut sword = item_stack(IRON_SWORD_ID, 1);
    sword.component_patch.enchantment_glint_override = Some(true);
    let sword_preview = smithing_preview_for_result_stack(sword, &runtime);
    assert_eq!(
        sword_preview.item_layers,
        vec![HudEntityPreviewItemLayer {
            slot: HudEntityPreviewItemSlot::LeftHand,
            display_context: HudEntityPreviewItemDisplayContext::ThirdPersonLeftHand,
            item_id: IRON_SWORD_ID,
            count: 1,
            foil: true,
            light_coords: ENTITY_FULL_BRIGHT_LIGHT_COORDS,
            overlay: ITEM_MODEL_NO_OVERLAY,
            order: 0,
            submit_sequence: 1,
        }]
    );
}

#[test]
fn advancement_projection_reflows_mixed_font_title_when_prepared_fonts_change() {
    use bbb_client_render::{HudAsciiGlyph, HudFontGlyphMap};
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
    let mut world = WorldStore::new();
    let mut root = runtime_displayed_advancement("test:fonts", None);
    root.display.as_mut().unwrap().title =
        RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".into(),
                value: NbtSummaryValue::String("AA".into()),
            },
            NbtSummaryEntry {
                name: "font".into(),
                value: NbtSummaryValue::String("test:wide".into()),
            },
        ]))
        .unwrap();
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![root],
        removed: vec![],
        progress: vec![],
        show_advancements: false,
    });
    world.ensure_advancements_screen_selected_tab();
    let size = winit::dpi::PhysicalSize::new(800, 600);
    let (x, y) = advancements_window_origin_for_surface(size);
    let project = |advance| {
        let mut custom = HudFontGlyphMap::new();
        custom.insert_first_wins('A', HudAsciiGlyph::empty(advance));
        let mut default = HudFontGlyphMap::new();
        default.set_missing_glyph(HudAsciiGlyph::empty(2.0));
        let fonts = HudFontGlyphBook::from_fonts(
            "minecraft:default",
            BTreeMap::from([
                ("minecraft:default".into(), default),
                ("test:wide".into(), custom),
            ]),
        );
        hud_advancements_screen(
            &world,
            None,
            &TerrainTextureState::default(),
            None,
            0.0,
            size,
            Some((
                x + ADVANCEMENTS_WINDOW_INSIDE_X + 110,
                y + ADVANCEMENTS_WINDOW_INSIDE_Y + 50,
            )),
            Default::default(),
            0.0,
            &fonts,
        )
    };
    let wide = project(100.25);
    let wrapped = wide
        .text_labels
        .iter()
        .filter(|label| label.text == "A")
        .collect::<Vec<_>>();
    assert_eq!(wrapped.len(), 2);
    assert!(wrapped
        .iter()
        .all(|label| label.width == 101
            && label.component_runs[0].font.as_deref() == Some("test:wide")));
    let narrow = project(20.25);
    assert!(!narrow.text_labels.iter().any(|label| label.text == "A"));
    let title = narrow
        .text_labels
        .iter()
        .find(|label| label.text == "AA" && label.tint == ADVANCEMENTS_HOVER_TITLE_TEXT_COLOR)
        .unwrap();
    assert_eq!(title.width, 41);
    assert_eq!(title.component_runs[0].font.as_deref(), Some("test:wide"));
}
