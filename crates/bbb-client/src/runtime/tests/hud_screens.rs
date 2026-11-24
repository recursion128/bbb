use super::*;

#[test]
fn item_frame_models_use_world_trim_material_select_context() {
    let root = unique_runtime_temp_dir("item-frame-trim-material-select");
    write_runtime_trim_material_select_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    record_trim_material_registry(&mut world);
    world.apply_add_entity(test_add_entity(701, 73));
    let mut stack = item_stack(0, 1);
    stack.component_patch.armor_trim_material_id = Some(0);
    assert!(
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 701,
            values: vec![test_item_stack_data(9, stack)],
        })
    );
    let terrain_textures =
        bbb_client_render::scene::terrain_textures::TerrainTextureState::default();
    let fallback = crate::item_frames::item_frame_models(
        &world,
        Some(&item_runtime),
        &terrain_textures,
        &crate::item_models::ItemRegistryKeySet::default(),
    );
    let trim_material_keys = crate::item_models::world_trim_material_keys(&world).unwrap();
    let trimmed = crate::item_frames::item_frame_models(
        &world,
        Some(&item_runtime),
        &terrain_textures,
        &crate::item_models::ItemRegistryKeySet {
            trim_materials: Some(trim_material_keys.clone()),
            ..Default::default()
        },
    );
    let fallback_item_assets = fallback
        .residency
        .assets
        .iter()
        .filter(|asset| asset.atlas == bbb_client_render::ItemModelAtlasKind::Items)
        .collect::<Vec<_>>();
    let trimmed_item_assets = trimmed
        .residency
        .assets
        .iter()
        .filter(|asset| asset.atlas == bbb_client_render::ItemModelAtlasKind::Items)
        .collect::<Vec<_>>();
    assert_eq!(fallback_item_assets.len(), 1);
    assert_eq!(trimmed_item_assets.len(), 1);
    assert_ne!(fallback_item_assets, trimmed_item_assets);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_screen_projects_hovered_item_tooltip_name() {
    let root = unique_runtime_temp_dir("inventory-tooltip");
    write_runtime_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: item_stack(0, 1),
    });
    assert!(world.open_local_inventory());

    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();

    assert_eq!(
        screen.tooltip,
        Some(HudInventoryTooltip {
            slot_id: 36,
            x: 8,
            y: 142,
            style: None,
            lines: vec![tooltip_name_line(
                "Test Combo",
                TOOLTIP_TEST_WHITE,
                0xFF_FF_FF,
                false
            )],
        })
    );

    let mut styled_stack = item_stack(0, 1);
    styled_stack.component_patch.added_type_ids =
        vec![bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID];
    styled_stack.component_patch.tooltip_style = Some("example:rare".to_string());
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: styled_stack,
    });
    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();
    assert_eq!(
        screen
            .tooltip
            .as_ref()
            .and_then(|tooltip| tooltip.style.as_deref()),
        Some("example:rare")
    );

    world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec!["minecraft:trade_rebalance".to_string()],
    });
    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();
    assert_eq!(
        screen.tooltip.as_ref().unwrap().lines,
        vec![
            tooltip_name_line("Test Combo", TOOLTIP_TEST_WHITE, 0xFF_FF_FF, false),
            tooltip_plain_line("Disabled item", TOOLTIP_TEST_RED),
        ]
    );
    world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec!["minecraft:vanilla".to_string()],
    });

    let mut damaged_stack = item_stack(0, 1);
    damaged_stack.component_patch.max_damage = Some(20);
    damaged_stack.component_patch.damage = Some(3);
    damaged_stack.component_patch.added_type_ids = vec![2, 3];
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: damaged_stack,
    });
    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        Some(36),
        InventoryHudLocalState {
            advanced_item_tooltips: true,
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();
    assert_eq!(
        screen.tooltip,
        Some(HudInventoryTooltip {
            slot_id: 36,
            x: 8,
            y: 142,
            style: None,
            lines: vec![
                tooltip_name_line("Test Combo", TOOLTIP_TEST_WHITE, 0xFF_FF_FF, false),
                tooltip_plain_line("Durability: 17 / 20", TOOLTIP_TEST_WHITE),
                tooltip_plain_line("minecraft:test_combo", TOOLTIP_TEST_DARK_GRAY),
                tooltip_plain_line("4 component(s)", TOOLTIP_TEST_DARK_GRAY),
            ],
        })
    );

    let mut custom_stack = item_stack(0, 1);
    custom_stack.component_patch.custom_name = Some("Custom Combo".to_string());
    custom_stack.component_patch.rarity = Some(bbb_protocol::packets::ItemRaritySummary::Rare);
    custom_stack.component_patch.lore = vec!["First lore".to_string(), "Second lore".to_string()];
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: custom_stack,
    });
    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();

    assert_eq!(
        screen.tooltip,
        Some(HudInventoryTooltip {
            slot_id: 36,
            x: 8,
            y: 142,
            style: None,
            lines: vec![
                tooltip_name_line("Custom Combo", TOOLTIP_TEST_AQUA, 0x55_FF_FF, true),
                tooltip_lore_line("First lore"),
                tooltip_lore_line("Second lore"),
            ],
        })
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_tooltip_uses_local_player_base_attributes_for_default_weapon() {
    let root = unique_runtime_temp_dir("inventory-default-attribute-tooltip");
    write_runtime_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = world_with_dimension(0, "minecraft:overworld");
    apply_block_tags(
        &mut world,
        vec![
            ("minecraft:sword_instantly_mines", Vec::new()),
            ("minecraft:sword_efficient", Vec::new()),
        ],
    );
    world.set_default_item_component_codec_values(
        item_runtime.default_component_codec_values_by_protocol_id(),
    );
    world.rebuild_item_component_defaults().unwrap();
    attach_lightmap_local_player(&mut world, 42);
    assert!(
        world.apply_update_attributes(bbb_protocol::packets::UpdateAttributes {
            entity_id: 42,
            attributes: vec![
                bbb_protocol::packets::AttributeSnapshot {
                    attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
                    base: 2.5,
                    modifiers: Vec::new(),
                },
                bbb_protocol::packets::AttributeSnapshot {
                    attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_SPEED_ID,
                    base: 5.0,
                    modifiers: Vec::new(),
                },
            ],
        })
    );
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: item_stack(1, 1),
    });
    assert!(world.open_local_inventory());

    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();
    assert_eq!(
        screen.tooltip,
        Some(HudInventoryTooltip {
            slot_id: 36,
            x: 8,
            y: 142,
            style: None,
            lines: vec![
                tooltip_name_line("Test Sword", TOOLTIP_TEST_WHITE, 0xFF_FF_FF, false),
                tooltip_plain_line("", TOOLTIP_TEST_WHITE),
                tooltip_plain_line("When in Main Hand:", TOOLTIP_TEST_GRAY),
                tooltip_plain_line(" 7.5 Attack Damage", TOOLTIP_TEST_DARK_GREEN),
                tooltip_plain_line(" 2.6 Attack Speed", TOOLTIP_TEST_DARK_GREEN),
            ],
        })
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_tooltip_follows_synced_enchantment_holder_order_and_effects() {
    let root = unique_runtime_temp_dir("inventory-enchantment-attribute-tooltip");
    write_runtime_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    world.record_registry_data(bbb_protocol::packets::RegistryData {
        registry: "minecraft:enchantment".to_string(),
        entries: vec![
            bbb_protocol::packets::RegistryDataEntry {
                id: "example:decoy".to_string(),
                raw_data: None,
            },
            bbb_protocol::packets::RegistryDataEntry {
                id: "example:efficiency".to_string(),
                raw_data: Some(efficiency_enchantment_attribute_nbt()),
            },
        ],
        raw_payload_len: 0,
    });
    world.begin_registry_configuration();
    world.finish_registry_configuration().unwrap();
    let mut stack = item_stack(0, 1);
    stack.component_patch.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
        holder_id: 1,
        level: 1,
    }];
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: stack,
    });
    assert!(world.open_local_inventory());

    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();
    assert_eq!(
        screen.tooltip,
        Some(HudInventoryTooltip {
            slot_id: 36,
            x: 8,
            y: 142,
            style: None,
            lines: vec![
                tooltip_name_line("Test Combo", TOOLTIP_TEST_AQUA, 0x55_FF_FF, false),
                tooltip_enchantment_line("Synced Efficiency", "I", 0x12_34_56),
                tooltip_plain_line("", TOOLTIP_TEST_WHITE),
                tooltip_plain_line("When in Main Hand:", TOOLTIP_TEST_GRAY),
                tooltip_plain_line("+2 Mining Efficiency", TOOLTIP_TEST_BLUE),
            ],
        })
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_tooltip_gates_operator_nbt_warning_on_local_permission() {
    let root = unique_runtime_temp_dir("inventory-operator-nbt-warning");
    write_runtime_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    let command_block = bbb_protocol::packets::ItemStackSummary {
        item_id: Some(2),
        count: 1,
        component_patch: DataComponentPatchSummary {
            block_entity_data_type_id: Some(VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: command_block.clone(),
    });
    assert!(world.open_local_inventory());

    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();
    assert_eq!(screen.tooltip.as_ref().unwrap().lines.len(), 1);

    assert!(
        world.apply_entity_event(bbb_protocol::packets::EntityEvent {
            entity_id: 42,
            event_id: 26,
        })
    );
    let warning_lines = vec![
        tooltip_operator_warning_line("Operator data", true),
        tooltip_operator_warning_line("Can alter commands", false),
        tooltip_operator_warning_line("Use with care", false),
    ];
    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();
    let mut expected_lines = vec![tooltip_name_line(
        "Command Block",
        TOOLTIP_TEST_WHITE,
        0xFF_FF_FF,
        false,
    )];
    expected_lines.extend(warning_lines.clone());
    assert_eq!(screen.tooltip.as_ref().unwrap().lines, expected_lines);

    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: bbb_protocol::packets::ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                tooltip_hide_tooltip: true,
                ..command_block.component_patch.clone().into_inner()
            }
            .into(),
            ..command_block
        },
    });
    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();
    assert_eq!(screen.tooltip.as_ref().unwrap().lines, warning_lines);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_screen_projects_generic_container_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X6_ID,
        title: bbb_protocol::RichTextComponent::plain("Large Chest"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 90],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(89), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 222);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/generic_54.png"),
                0,
                0,
                176,
                125,
                [0.0, 0.0],
                [176.0 / 256.0, 125.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/generic_54.png"),
                0,
                125,
                176,
                96,
                [0.0, 126.0 / 256.0],
                [176.0 / 256.0, 222.0 / 256.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(89));
    assert_eq!(screen.slots.len(), 90);
    let first_container = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((first_container.x, first_container.y), (8, 18));
    let player_inventory = screen.slots.iter().find(|slot| slot.slot_id == 54).unwrap();
    assert_eq!((player_inventory.x, player_inventory.y), (8, 139));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 89).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 197));
}

#[test]
fn all_generated_open_screen_menu_types_project_a_native_surface() {
    assert_eq!(VANILLA_MENU_TYPES.len(), 25);
    for menu_type in VANILLA_MENU_TYPES {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: menu_type.id,
            title: bbb_protocol::RichTextComponent::plain(menu_type.resource_id),
        });

        if menu_type.id == VANILLA_MENU_TYPE_LECTERN_ID {
            assert!(hud_inventory_screen(&world, None, None, 0.0).is_none());
            let mut input = ClientInputState::default();
            input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
            let book = hud_book_screen(&input, &world, None)
                .expect("lectern must project its dedicated BookViewScreen surface");
            assert_eq!(book.mode, bbb_client_render::HudBookMode::Lectern);
            assert_eq!(book.page_indicator, "Page 1 of 1");
            continue;
        }

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap_or_else(|| {
            panic!(
                "generated menu {} ({}) has no native presentation",
                menu_type.constant_name, menu_type.resource_id
            )
        });
        if menu_type.id == VANILLA_MENU_TYPE_BEACON_ID {
            assert_eq!(
                screen
                    .text_labels
                    .iter()
                    .map(|label| (
                        label.text.as_str(),
                        label.scrolling.unwrap().x,
                        label.y,
                        label.tint,
                        label.shadow,
                    ))
                    .collect::<Vec<_>>(),
                [
                    (
                        "Primary Power",
                        BEACON_PRIMARY_LABEL_CENTER_X
                            - i32::try_from(CONTAINER_CENTERED_LABEL_WIDTH).unwrap() / 2,
                        BEACON_LABEL_Y,
                        BEACON_LABEL_TEXT_COLOR,
                        false,
                    ),
                    (
                        "Secondary Power",
                        BEACON_SECONDARY_LABEL_CENTER_X
                            - i32::try_from(CONTAINER_CENTERED_LABEL_WIDTH).unwrap() / 2,
                        BEACON_LABEL_Y,
                        BEACON_LABEL_TEXT_COLOR,
                        false,
                    ),
                ],
                "generated Beacon menu label contract"
            );
            continue;
        }
        let title = &screen.text_labels[0];
        let inventory = &screen.text_labels[1];
        let (title_x, title_y, centered) = match menu_type.resource_id {
            "minecraft:generic_3x3"
            | "minecraft:crafter_3x3"
            | "minecraft:blast_furnace"
            | "minecraft:brewing_stand"
            | "minecraft:furnace"
            | "minecraft:smoker" => (-168, 6, true),
            "minecraft:merchant" => (-69, 6, true),
            "minecraft:anvil" => (60, 6, false),
            "minecraft:crafting" => (29, 6, false),
            "minecraft:loom" | "minecraft:cartography_table" => (8, 4, false),
            "minecraft:smithing" => (44, 15, false),
            "minecraft:stonecutter" => (8, 5, false),
            _ => (8, 6, false),
        };
        assert_eq!(
            (title.text.as_str(), title.x, title.y),
            (menu_type.resource_id, title_x, title_y),
            "generated menu {} title contract",
            menu_type.constant_name
        );
        assert_eq!(title.tint, CONTAINER_LABEL_TEXT_COLOR);
        assert!(!title.shadow);
        assert_eq!(
            title.scrolling.is_some(),
            centered,
            "generated menu {} centering contract",
            menu_type.constant_name
        );
        assert_eq!(
            (
                inventory.text.as_str(),
                inventory.x,
                inventory.y,
                inventory.tint,
                inventory.shadow,
            ),
            (
                "Inventory",
                if menu_type.id == VANILLA_MENU_TYPE_MERCHANT_ID {
                    107
                } else {
                    8
                },
                i32::try_from(screen.height).unwrap() - 94,
                CONTAINER_LABEL_TEXT_COLOR,
                false,
            ),
            "generated menu {} inventory-label contract",
            menu_type.constant_name
        );
    }
}

#[test]
fn open_screen_title_localizes_rich_component_runs_and_uses_real_glyph_centering() {
    let title =
        bbb_protocol::RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "translate".into(),
                value: NbtSummaryValue::string("missing.container.title"),
            },
            NbtSummaryEntry {
                name: "fallback".into(),
                value: NbtSummaryValue::string("Open %s"),
            },
            NbtSummaryEntry {
                name: "with".into(),
                value: NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "text".into(),
                        value: NbtSummaryValue::string("Chest"),
                    },
                    NbtSummaryEntry {
                        name: "bold".into(),
                        value: NbtSummaryValue::Byte(1),
                    },
                    NbtSummaryEntry {
                        name: "color".into(),
                        value: NbtSummaryValue::string("red"),
                    },
                ])]),
            },
        ]))
        .expect("rich open-screen title");
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_3X3_ID,
        title,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).expect("container HUD");

    assert_eq!(screen.text_labels.len(), 2);
    let title = &screen.text_labels[0];
    assert_eq!(title.text, "Open Chest");
    assert_eq!(title.tint, CONTAINER_LABEL_TEXT_COLOR);
    assert!(!title.shadow);
    assert_eq!(
        title.scrolling,
        Some(HudInventoryScrollingText {
            x: -168,
            y: 5,
            width: 512,
            height: 9,
        })
    );
    assert_eq!(title.runs.len(), 2);
    assert_eq!(title.runs[0], HudStyledTextRun::plain("Open "));
    assert_eq!(title.runs[1].text, "Chest");
    assert!(title.runs[1].style.bold);
    assert_eq!(title.runs[1].color, Some(0xFF_55_55));
    assert_eq!(
        (
            screen.text_labels[1].text.as_str(),
            screen.text_labels[1].x,
            screen.text_labels[1].y,
        ),
        ("Inventory", 8, 72)
    );
}

#[test]
fn hud_inventory_screen_projects_generic_3x3_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Dispenser"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 45],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(44), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Texture("textures/gui/container/dispenser.png"),
            0,
            0,
            176,
            166,
            [0.0, 0.0],
            [176.0 / 256.0, 166.0 / 256.0],
        )]
    );
    assert_eq!(screen.hovered_slot_id, Some(44));
    assert_eq!(screen.slots.len(), 45);
    let first_container = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((first_container.x, first_container.y), (62, 17));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 44).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_crafter_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(45), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/crafter.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/crafter/unpowered_redstone"),
                97,
                35,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(45));
    assert_eq!(screen.slots.len(), 46);
    let first_grid = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((first_grid.x, first_grid.y), (26, 17));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 45).unwrap();
    assert_eq!((result.x, result.y), (134, 35));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 44).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_crafter_state_layers() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 0,
        value: 1,
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 8,
        value: 1,
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 9,
        value: 1,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/crafter.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/crafter/disabled_slot"),
                25,
                16,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/crafter/disabled_slot"),
                61,
                52,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/crafter/powered_redstone"),
                97,
                35,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_crafting_table_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(45), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/crafting_table.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("recipe_book/button"),
                5,
                34,
                20,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(45));
    assert_eq!(screen.slots.len(), 46);
    let result = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((result.x, result.y), (124, 35));
    let first_grid = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((first_grid.x, first_grid.y), (30, 17));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 45).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_highlights_recipe_book_button() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some((5, 34)),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(
        screen.background_layers.last().map(|layer| layer.texture),
        Some(HudInventoryBackgroundTexture::Sprite(
            "recipe_book/button_highlighted"
        ))
    );
}

#[test]
fn hud_inventory_screen_projects_recipe_book_overlay_for_crafting_table() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });

    let screen = hud_inventory_screen(&world, None, Some(45), 0.0).unwrap();

    assert_eq!(screen.width, 320);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/recipe_book.png"),
                0,
                0,
                147,
                166,
                [1.0 / 256.0, 1.0 / 256.0],
                [148.0 / 256.0, 167.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/crafting_table.png"),
                149,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("recipe_book/tab_selected"),
                -32,
                3,
                35,
                27,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("widget/text_field"),
                25,
                13,
                81,
                14,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("recipe_book/button"),
                154,
                34,
                20,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("recipe_book/filter_disabled"),
                110,
                12,
                26,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(45));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((result.x, result.y), (273, 35));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 45).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (301, 142));
}

#[test]
fn hud_inventory_screen_projects_narrow_recipe_book_over_main_gui() {
    let world = open_recipe_book_crafting_table_world();

    let screen = hud_inventory_screen_with_local_state_for_surface(
        &world,
        None,
        &TerrainTextureState::default(),
        Some(45),
        InventoryHudLocalState::default(),
        winit::dpi::PhysicalSize::new(378, 720),
        0.0,
    )
    .unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen
            .background_layers
            .iter()
            .find(|layer| layer.texture
                == HudInventoryBackgroundTexture::Texture(
                    "textures/gui/container/crafting_table.png"
                ))
            .map(|layer| layer.x),
        Some(0)
    );
    assert_eq!(
        screen
            .background_layers
            .iter()
            .find(|layer| layer.texture
                == HudInventoryBackgroundTexture::Texture("textures/gui/recipe_book.png"))
            .map(|layer| layer.x),
        Some(14)
    );
    assert_eq!(
        screen
            .background_layers
            .iter()
            .find(|layer| layer.texture
                == HudInventoryBackgroundTexture::Sprite("recipe_book/button"))
            .map(|layer| layer.x),
        Some(5)
    );
    assert_eq!(
        screen
            .background_layers
            .iter()
            .find(
                |layer| layer.texture == HudInventoryBackgroundTexture::Sprite("widget/text_field")
            )
            .map(|layer| layer.x),
        Some(39)
    );
    let result = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((result.x, result.y), (124, 35));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 45).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_crafting_table_ghost_recipe_slots() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 7,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShaped,
            raw_body: Vec::new(),
            crafting: Some(Box::new(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shaped {
                    width: 2,
                    height: 1,
                    ingredients: vec![
                        stonecutter_item_display(1),
                        stonecutter_item_stack_display(2, 3),
                    ],
                    result: stonecutter_item_stack_display(4, 2),
                    crafting_station: slot_display_empty(),
                },
            )),
            furnace: None,
            stonecutter: None,
            smithing: None,
        },
    });

    let screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();

    assert_eq!(
        screen
            .fill_layers
            .iter()
            .filter(|layer| layer.stage == HudInventoryFillStage::BeforeGhostItem)
            .copied()
            .collect::<Vec<_>>(),
        vec![
            HudInventoryFillLayer {
                x: 269,
                y: 31,
                width: 24,
                height: 24,
                tint: RECIPE_BOOK_GHOST_PRE_ITEM_TINT,
                stage: HudInventoryFillStage::BeforeGhostItem,
            },
            HudInventoryFillLayer {
                x: 179,
                y: 35,
                width: 16,
                height: 16,
                tint: RECIPE_BOOK_GHOST_PRE_ITEM_TINT,
                stage: HudInventoryFillStage::BeforeGhostItem,
            },
            HudInventoryFillLayer {
                x: 197,
                y: 35,
                width: 16,
                height: 16,
                tint: RECIPE_BOOK_GHOST_PRE_ITEM_TINT,
                stage: HudInventoryFillStage::BeforeGhostItem,
            },
        ]
    );
    assert_eq!(
        screen
            .fill_layers
            .iter()
            .filter(|layer| layer.stage == HudInventoryFillStage::AfterGhostItem)
            .map(|layer| (layer.x, layer.y, layer.width, layer.height, layer.tint))
            .collect::<Vec<_>>(),
        vec![
            (273, 35, 16, 16, RECIPE_BOOK_GHOST_POST_ITEM_TINT),
            (179, 35, 16, 16, RECIPE_BOOK_GHOST_POST_ITEM_TINT),
            (197, 35, 16, 16, RECIPE_BOOK_GHOST_POST_ITEM_TINT),
        ]
    );
    assert_eq!(
        screen
            .ghost_items
            .iter()
            .map(|item| (
                item.x,
                item.y,
                item.draw_decorations,
                item.icon.count_label.clone()
            ))
            .collect::<Vec<_>>(),
        vec![
            (273, 35, true, Some(HudItemCountLabel::new("2"))),
            (179, 35, false, None),
            (197, 35, false, None),
        ]
    );

    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 99,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
            raw_body: Vec::new(),
            crafting: Some(Box::new(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                    ingredients: vec![stonecutter_item_display(1)],
                    result: stonecutter_item_display(4),
                    crafting_station: slot_display_empty(),
                },
            )),
            furnace: None,
            stonecutter: None,
            smithing: None,
        },
    });
    let stale = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();
    assert_eq!(stale.fill_layers, screen.fill_layers);
    assert_eq!(stale.ghost_items, screen.ghost_items);
    assert_eq!(world.counters().ghost_recipe_packets, 2);
    assert_eq!(world.counters().ghost_recipe_updates_applied, 1);
    assert_eq!(world.counters().ghost_recipe_updates_ignored, 1);

    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 8,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 8,
        state_id: 1,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 63],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    let replaced = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();
    assert!(replaced.fill_layers.is_empty());
    assert!(replaced.ghost_items.is_empty());
}

#[test]
fn hud_inventory_screen_projects_furnace_ghost_recipe_slots() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_furnace_world();
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 7,
        recipe_display: furnace_recipe_display(2, 1, 4),
    });

    let screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();

    assert_eq!(
        screen
            .fill_layers
            .iter()
            .filter(|layer| layer.stage == HudInventoryFillStage::BeforeGhostItem)
            .map(|layer| (layer.x, layer.y, layer.width, layer.height))
            .collect::<Vec<_>>(),
        vec![(265, 35, 16, 16), (205, 17, 16, 16), (205, 53, 16, 16)]
    );
    assert_eq!(
        screen
            .ghost_items
            .iter()
            .map(|item| (item.x, item.y, item.draw_decorations))
            .collect::<Vec<_>>(),
        vec![(265, 35, true), (205, 17, false), (205, 53, false)]
    );

    world.apply_container_set_slot(bbb_protocol::packets::ContainerSetSlot {
        container_id: 7,
        state_id: 13,
        slot: 1,
        item: item_stack(1, 1),
    });
    let fuel_occupied = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();
    assert_eq!(
        fuel_occupied
            .ghost_items
            .iter()
            .map(|item| (item.x, item.y, item.draw_decorations))
            .collect::<Vec<_>>(),
        vec![(265, 35, true), (205, 17, false), (205, 53, false)]
    );

    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 7,
        recipe_display: furnace_recipe_display(2, 1, 4),
    });
    let refreshed = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();
    assert_eq!(
        refreshed
            .ghost_items
            .iter()
            .map(|item| (item.x, item.y, item.draw_decorations))
            .collect::<Vec<_>>(),
        vec![(265, 35, true), (205, 17, false)]
    );
}

#[test]
fn hud_inventory_screen_projects_blast_furnace_and_smoker_ghost_recipe_slots() {
    let item_runtime = recipe_book_ghost_item_runtime();
    for menu_type_id in [
        VANILLA_MENU_TYPE_BLAST_FURNACE_ID,
        VANILLA_MENU_TYPE_SMOKER_ID,
    ] {
        let mut world = open_recipe_book_furnace_family_world(menu_type_id);
        world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
            container_id: 7,
            recipe_display: furnace_recipe_display(2, 1, 4),
        });

        let screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();
        assert_eq!(
            screen
                .ghost_items
                .iter()
                .map(|item| (item.x, item.y, item.draw_decorations))
                .collect::<Vec<_>>(),
            vec![(265, 35, true), (205, 17, false), (205, 53, false)]
        );
    }
}

#[test]
fn hud_inventory_screen_projects_all_builtin_slot_display_ghost_recipe_candidates() {
    let root = unique_runtime_temp_dir("ghost-recipe-slot-displays");
    write_stonecutter_slot_display_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = stonecutter_world_with_builtin_slot_displays(1);
    let displays = world
        .recipes()
        .stonecutter_recipes
        .iter()
        .map(|recipe| recipe.option_display.clone())
        .collect::<Vec<_>>();
    assert_eq!(displays.len(), 11);
    assert!(world.close_local_container(7));
    assert!(world.open_local_inventory());

    for (index, result) in displays.into_iter().enumerate() {
        world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
            container_id: 0,
            recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
                display_type: bbb_protocol::packets::RecipeDisplayType::Stonecutter,
                raw_body: Vec::new(),
                crafting: None,
                furnace: None,
                stonecutter: Some(Box::new(
                    bbb_protocol::packets::StonecutterRecipeDisplaySummary {
                        input: slot_display_empty(),
                        result,
                        crafting_station: slot_display_empty(),
                    },
                )),
                smithing: None,
            },
        });

        let snapshot = world.last_ghost_recipe().unwrap();
        assert_eq!(snapshot.slots.len(), usize::from(index != 0));
        let screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();
        assert_eq!(screen.ghost_items.len(), snapshot.slots.len());
        assert!(screen.ghost_items.iter().all(|item| item.draw_decorations));
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_screen_resolves_tag_ghost_recipe_ingredients() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    apply_item_tags(&mut world, vec![("minecraft:planks", vec![2])]);
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 7,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
            raw_body: Vec::new(),
            crafting: Some(Box::new(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                    ingredients: vec![slot_display_tag("minecraft:planks")],
                    result: stonecutter_item_display(4),
                    crafting_station: slot_display_empty(),
                },
            )),
            furnace: None,
            stonecutter: None,
            smithing: None,
        },
    });

    let screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();

    assert_eq!(
        screen
            .ghost_items
            .iter()
            .map(|item| (
                item.x,
                item.y,
                item.draw_decorations,
                item.icon.count_label.clone()
            ))
            .collect::<Vec<_>>(),
        vec![(273, 35, true, None), (179, 17, false, None)]
    );
}

#[test]
fn hud_inventory_screen_cycles_tag_ghost_recipe_ingredients() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    apply_item_tags(&mut world, vec![("minecraft:planks", vec![1, 2])]);
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 7,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
            raw_body: Vec::new(),
            crafting: Some(Box::new(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                    ingredients: vec![slot_display_tag("minecraft:planks")],
                    result: stonecutter_item_display(4),
                    crafting_station: slot_display_empty(),
                },
            )),
            furnace: None,
            stonecutter: None,
            smithing: None,
        },
    });

    let first_screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();
    let first_icon = first_screen
        .ghost_items
        .iter()
        .find(|item| (item.x, item.y, item.draw_decorations) == (179, 17, false))
        .map(|item| item.icon.clone())
        .expect("first tag ghost ingredient");

    world.apply_world_time(PlayTime {
        game_time: 30,
        clock_updates: Vec::new(),
    });
    let second_screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();
    let second_icon = second_screen
        .ghost_items
        .iter()
        .find(|item| (item.x, item.y, item.draw_decorations) == (179, 17, false))
        .map(|item| item.icon.clone())
        .expect("second tag ghost ingredient");

    assert_ne!(first_icon, second_icon);
}

#[test]
fn hud_inventory_screen_keeps_resolved_tag_ghost_recipe_snapshot() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    apply_item_tags(&mut world, vec![("minecraft:planks", vec![1, 2])]);
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 7,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
            raw_body: Vec::new(),
            crafting: Some(Box::new(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                    ingredients: vec![slot_display_tag("minecraft:planks")],
                    result: stonecutter_item_display(4),
                    crafting_station: slot_display_empty(),
                },
            )),
            furnace: None,
            stonecutter: None,
            smithing: None,
        },
    });

    let ghost_icon = |world: &WorldStore| {
        hud_inventory_screen(world, Some(&item_runtime), None, 0.0)
            .unwrap()
            .ghost_items
            .into_iter()
            .find(|item| (item.x, item.y, item.draw_decorations) == (179, 17, false))
            .map(|item| item.icon)
    };

    let first = ghost_icon(&world).expect("first ordered tag member");

    apply_item_tags(&mut world, vec![("minecraft:planks", vec![2, 1])]);
    let reordered = ghost_icon(&world).expect("snapshotted tag member");
    assert_eq!(reordered, first);

    apply_item_tags(&mut world, Vec::new());
    assert_eq!(ghost_icon(&world), Some(first));
}

#[test]
fn hud_inventory_screen_ghost_recipe_tooltip_wins_only_while_recipe_book_is_visible() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_container_set_slot(bbb_protocol::packets::ContainerSetSlot {
        container_id: 7,
        state_id: 13,
        slot: 1,
        item: item_stack(2, 1),
    });
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 7,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
            raw_body: Vec::new(),
            crafting: Some(Box::new(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                    ingredients: vec![stonecutter_item_display(1)],
                    result: stonecutter_item_display(4),
                    crafting_station: slot_display_empty(),
                },
            )),
            furnace: None,
            stonecutter: None,
            smithing: None,
        },
    });

    let visible = hud_inventory_screen(&world, Some(&item_runtime), Some(1), 0.0).unwrap();
    assert_eq!(
        visible
            .tooltip
            .as_ref()
            .and_then(|tooltip| tooltip.lines.first())
            .map(|line| line.text.as_str()),
        Some("Stick")
    );

    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });
    let hidden = hud_inventory_screen(&world, Some(&item_runtime), Some(1), 0.0).unwrap();
    assert_eq!(
        hidden
            .tooltip
            .as_ref()
            .and_then(|tooltip| tooltip.lines.first())
            .map(|line| line.text.as_str()),
        Some("Wooden Boards")
    );
    assert_eq!(
        hidden
            .ghost_items
            .iter()
            .map(|item| (&item.icon, item.draw_decorations))
            .collect::<Vec<_>>(),
        visible
            .ghost_items
            .iter()
            .map(|item| (&item.icon, item.draw_decorations))
            .collect::<Vec<_>>()
    );
}

#[test]
fn hud_inventory_screen_cycles_composite_ghost_recipe_ingredients() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 7,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
            raw_body: Vec::new(),
            crafting: Some(Box::new(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                    ingredients: vec![slot_display_composite(vec![
                        slot_display_with_remainder(
                            stonecutter_item_display(1),
                            stonecutter_item_display(0),
                        ),
                        stonecutter_item_display(2),
                    ])],
                    result: stonecutter_item_display(4),
                    crafting_station: slot_display_empty(),
                },
            )),
            furnace: None,
            stonecutter: None,
            smithing: None,
        },
    });

    let first_screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();
    let first_icon = first_screen
        .ghost_items
        .iter()
        .find(|item| (item.x, item.y, item.draw_decorations) == (179, 17, false))
        .map(|item| item.icon.clone())
        .expect("first composite ghost ingredient");

    world.apply_world_time(PlayTime {
        game_time: 30,
        clock_updates: Vec::new(),
    });
    let second_screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();
    let second_icon = second_screen
        .ghost_items
        .iter()
        .find(|item| (item.x, item.y, item.draw_decorations) == (179, 17, false))
        .map(|item| item.icon.clone())
        .expect("second composite ghost ingredient");

    assert_ne!(first_icon, second_icon);
}

#[test]
fn hud_inventory_screen_projects_local_inventory_ghost_recipe_result_without_big_slot_fill() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = WorldStore::new();
    world.open_local_inventory();
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 0,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
            raw_body: Vec::new(),
            crafting: Some(Box::new(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                    ingredients: vec![stonecutter_item_display(1)],
                    result: stonecutter_item_display(4),
                    crafting_station: slot_display_empty(),
                },
            )),
            furnace: None,
            stonecutter: None,
            smithing: None,
        },
    });

    let screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();

    assert_eq!(
        screen
            .fill_layers
            .iter()
            .filter(|layer| layer.stage == HudInventoryFillStage::BeforeGhostItem)
            .map(|layer| (layer.x, layer.y, layer.width, layer.height))
            .collect::<Vec<_>>(),
        vec![(154, 28, 16, 16), (98, 18, 16, 16)]
    );
    assert_eq!(
        screen
            .ghost_items
            .iter()
            .map(|item| (item.x, item.y, item.draw_decorations))
            .collect::<Vec<_>>(),
        vec![(154, 28, true), (98, 18, false)]
    );
}

#[test]
fn hud_inventory_screen_projects_recipe_book_search_box_text() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });
    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_search: RecipeBookSearchHudState {
                edit: {
                    let mut edit = crate::input::text_field::TextField::with_text("axe", 50);
                    edit.set_cursor(2, None);
                    edit.set_selection(1, None);
                    edit
                },
                focused: true,
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("widget/text_field_highlighted"),
                25,
                13,
                81,
                14,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
    let label = screen.text_labels.last().unwrap();
    assert_eq!(label.x, 29);
    assert_eq!(label.y, 16);
    assert_eq!(label.width, 73);
    assert_eq!(label.text, "axe");
    assert_eq!(label.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(label.background, None);
    assert!(!label.shadow);
    assert!(label.runs.is_empty());
    let input = label.input.unwrap();
    assert_eq!(input.cursor, 2);
    assert_eq!(input.selection, 1);
    assert_eq!(input.scroll_to, 1);
    assert_eq!(input.max_length, 50);
    assert_eq!(input.cursor_tint, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn hud_inventory_screen_projects_selected_recipe_book_tab() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![shapeless_crafting_recipe_book_entry(20, 2, None, 120)],
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("recipe_book/tab_selected"),
                -32,
                30,
                35,
                27,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
}

#[test]
fn hud_inventory_screen_animates_highlighted_recipe_book_tab() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    let mut entry = shapeless_crafting_recipe_book_entry(20, 2, None, 120);
    entry.flags |= 0b10;
    entry.highlight = true;
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![entry],
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            recipe_book_presentation: RecipeBookPresentationHudState {
                tab_animation_ticks: [0, 8, 0, 0, 0],
                ..RecipeBookPresentationHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.5,
    )
    .unwrap();

    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("recipe_book/tab_selected"),
                -32,
                29,
                35,
                30,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));

    let primary_icon = screen
        .floating_items
        .iter()
        .find(|item| item.x == -29)
        .expect("selected recipe book tab primary icon");
    assert_eq!(primary_icon.y, 34);
    assert_eq!(primary_icon.scale, 1.0);
    assert!((primary_icon.scale_y - 1.1).abs() <= 0.000001);

    let secondary_icon = screen
        .floating_items
        .iter()
        .find(|item| item.x == -18)
        .expect("selected recipe book tab secondary icon");
    assert_eq!(secondary_icon.y, 34);
    assert_eq!(secondary_icon.scale, 1.0);
    assert!((secondary_icon.scale_y - 1.1).abs() <= 0.000001);
}

#[test]
fn hud_inventory_screen_projects_crafting_recipe_book_buttons() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![bbb_protocol::packets::RecipeBookAddEntry {
            contents: bbb_protocol::packets::RecipeDisplayEntry {
                id: bbb_protocol::packets::RecipeDisplayId { index: 42 },
                display: bbb_protocol::packets::RecipeDisplaySummary {
                    display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
                    raw_body: Vec::new(),
                    crafting: Some(Box::new(
                        bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                            ingredients: Vec::new(),
                            result: bbb_protocol::packets::SlotDisplaySummary {
                                display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
                                raw_payload: Vec::new(),
                                item_stack: Some(item_stack(99, 1)),
                                tag: None,
                            },
                            crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                                display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
                                raw_payload: Vec::new(),
                                item_stack: None,
                                tag: None,
                            },
                        },
                    )),
                    furnace: None,
                    stonecutter: None,
                    smithing: None,
                },
                group: None,
                category_id: VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_MISC_ID,
                crafting_requirements: None,
            },
            flags: 0,
            notification: false,
            highlight: false,
        }],
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("recipe_book/slot_uncraftable"),
                11,
                31,
                25,
                25,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
}

#[test]
fn hud_inventory_screen_keeps_recipe_button_animation_after_highlight_removal() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    let mut entry = shapeless_crafting_recipe_book_entry(20, 2, None, 1);
    let bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless { result, .. } =
        entry.contents.display.crafting.as_deref_mut().unwrap()
    else {
        unreachable!()
    };
    result.item_stack.as_mut().unwrap().count = 4;
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![entry],
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            recipe_book_presentation: RecipeBookPresentationHudState {
                button_animation_ticks: [
                    8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ],
                ..RecipeBookPresentationHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.5,
    )
    .unwrap();

    assert!(world.recipe_book().highlights.is_empty());
    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("recipe_book/slot_uncraftable"),
                10,
                30,
                28,
                28,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
    let item = screen
        .floating_items
        .iter()
        .find(|item| item.x == 15 && item.y == 34)
        .expect("animated recipe result");
    assert!(!item.draw_decorations);
    assert_eq!(item.icon.count_label.as_ref().unwrap().text, "4");
    let hint = screen
        .text_labels
        .iter()
        .find(|label| label.text == "Search...")
        .unwrap();
    assert!(hint.input.is_none());
    assert!(hint.shadow);
    assert_eq!(hint.runs.len(), 1);
    assert_eq!(hint.runs[0].color, Some(0xAA_AA_AA));
    assert!(hint.runs[0].style.italic);
    let focused = hud_recipe_book_search_text_label(
        &world,
        Some(&item_runtime),
        InventoryScreenBackground::CraftingTable,
        &RecipeBookSearchHudState {
            focused: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(focused.text.is_empty());
    assert!(focused.input.is_some());
    assert!(focused.runs.is_empty());
    assert!((item.scale - 1.1).abs() <= 0.000001);
    assert!((item.scale_y - 1.1).abs() <= 0.000001);
}

#[test]
fn hud_inventory_screen_projects_recipe_button_and_control_tooltips() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry(20, 2, Some(7), 1),
            shapeless_crafting_recipe_book_entry(21, 2, Some(7), 1),
        ],
    });
    let local_state = InventoryHudLocalState {
        recipe_book_tabs: RecipeBookTabSelectionHudState {
            crafting: 1,
            ..RecipeBookTabSelectionHudState::default()
        },
        cursor_position: Some((15, 35)),
        ..InventoryHudLocalState::default()
    };

    let recipe = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        local_state.clone(),
        0.0,
    )
    .unwrap()
    .tooltip
    .expect("recipe tooltip");
    assert_eq!(recipe.slot_id, u16::MAX);
    assert_eq!(
        recipe.lines.last().map(|line| line.text.as_str()),
        Some("Right Click for More")
    );

    let overlay = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_overlay: Some(RecipeBookOverlayHudState {
                book_type: RecipeBookType::Crafting,
                tab_index: 1,
                page_index: 0,
                button_index: 0,
                x: 11,
                y: 31,
            }),
            ..local_state.clone()
        },
        0.0,
    )
    .unwrap();
    assert!(overlay.tooltip.is_none());

    let filter = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some((
                RECIPE_BOOK_FILTER_BUTTON_X + 1,
                RECIPE_BOOK_FILTER_BUTTON_Y + 1,
            )),
            ..local_state
        },
        0.0,
    )
    .unwrap()
    .tooltip
    .expect("filter tooltip");
    assert_eq!(filter.lines[0].text, "Showing All");
}

#[test]
fn hud_inventory_screen_projects_recipe_book_page_tooltips() {
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: (0..21)
            .map(|index| shapeless_crafting_recipe_book_entry(index, 2, None, 1))
            .collect(),
    });
    let base = InventoryHudLocalState {
        recipe_book_tabs: RecipeBookTabSelectionHudState {
            crafting: 1,
            ..RecipeBookTabSelectionHudState::default()
        },
        cursor_position: Some((
            RECIPE_BOOK_PAGE_FORWARD_BUTTON_X + 1,
            RECIPE_BOOK_PAGE_BUTTON_Y + 1,
        )),
        ..InventoryHudLocalState::default()
    };

    let next = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        base.clone(),
        0.0,
    )
    .unwrap()
    .tooltip
    .expect("next page tooltip");
    assert_eq!(next.lines[0].text, "Next Page");

    let previous = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_pages: RecipeBookPageHudState {
                crafting: 1,
                ..RecipeBookPageHudState::default()
            },
            cursor_position: Some((
                RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X + 1,
                RECIPE_BOOK_PAGE_BUTTON_Y + 1,
            )),
            ..base
        },
        0.0,
    )
    .unwrap()
    .tooltip
    .expect("previous page tooltip");
    assert_eq!(previous.lines[0].text, "Previous Page");
}

#[test]
fn recipe_book_narration_describes_multi_recipe_button_and_usage() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry(20, 2, Some(7), 1),
            shapeless_crafting_recipe_book_entry(21, 2, Some(7), 1),
        ],
    });
    let mut input = ClientInputState::new(true);
    input.update_screen_cursor_position(Some(PhysicalPosition::new(15.0, 35.0)));

    let narration = recipe_book_narration_frame(
        &input,
        &world,
        Some(&item_runtime),
        PhysicalSize::new(1280, 720),
        0.0,
    )
    .unwrap()
    .all_text_for_test();

    assert_eq!(
        narration,
        "Recipe for Stick. Left click to activate. Right click to show more recipes"
    );
}

#[test]
fn focused_recipe_book_search_has_terminal_narration_priority() {
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![shapeless_crafting_recipe_book_entry(20, 2, None, 1)],
    });
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let commands = None;
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    let surface = PhysicalSize::new(1280, 720);
    let origin_x = (1280.0 - 320.0) / 2.0;
    let origin_y = (720.0 - 166.0) / 2.0;
    assert!(crate::input::handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_SEARCH_BOX_X + 1),
            origin_y + f64::from(RECIPE_BOOK_SEARCH_BOX_Y + 1),
        )),
        surface,
    ));
    input.update_screen_cursor_position(Some(PhysicalPosition::new(15.0, 35.0)));

    let narration = recipe_book_narration_frame(&input, &world, None, surface, 0.0)
        .unwrap()
        .all_text_for_test();

    assert!(narration.starts_with("Search Items edit box:"));
    assert!(!narration.contains("Recipe for"));
}

#[test]
fn recipe_book_narration_describes_filter_and_page_controls() {
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: (0..21)
            .map(|index| shapeless_crafting_recipe_book_entry(index, 2, None, 1))
            .collect(),
    });
    let surface = PhysicalSize::new(1280, 720);
    let mut input = ClientInputState::new(true);
    input.update_screen_cursor_position(Some(PhysicalPosition::new(
        f64::from(RECIPE_BOOK_FILTER_BUTTON_X + 1),
        f64::from(RECIPE_BOOK_FILTER_BUTTON_Y + 1),
    )));
    assert_eq!(
        recipe_book_narration_frame(&input, &world, None, surface, 0.0)
            .unwrap()
            .all_text_for_test(),
        "Showing All button. Left click to activate"
    );

    input.update_screen_cursor_position(Some(PhysicalPosition::new(
        f64::from(RECIPE_BOOK_PAGE_FORWARD_BUTTON_X + 1),
        f64::from(RECIPE_BOOK_PAGE_BUTTON_Y + 1),
    )));
    assert_eq!(
        recipe_book_narration_frame(&input, &world, None, surface, 0.0)
            .unwrap()
            .all_text_for_test(),
        "Next Page button. Left click to activate"
    );
}

#[test]
fn hud_inventory_screen_projects_furnace_recipe_book_buttons() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_furnace_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![furnace_recipe_book_entry(42, 4, None, 1)],
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                furnace: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(screen.width, 320);
    assert_eq!(
        (
            screen.text_labels[0].text.as_str(),
            screen.text_labels[0].x,
            screen.text_labels[0].scrolling.map(|scrolling| scrolling.x),
        ),
        ("Furnace", -19, Some(-19))
    );
    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("recipe_book/slot_uncraftable"),
                11,
                31,
                25,
                25,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
    assert!(screen
        .floating_items
        .iter()
        .any(|item| (item.x, item.y) == (15, 35)));
}

#[test]
fn hud_inventory_screen_counts_furnace_slots_for_recipe_book_craftability() {
    let mut world = open_recipe_book_furnace_world();
    world.apply_container_set_slot(bbb_protocol::packets::ContainerSetSlot {
        container_id: 7,
        state_id: 13,
        slot: 1,
        item: item_stack(2, 1),
    });
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![furnace_recipe_book_entry_with_requirements(
            42,
            4,
            None,
            1,
            vec![vec![2]],
        )],
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                furnace: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("recipe_book/slot_craftable"),
                11,
                31,
                25,
                25,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
}

#[test]
fn hud_inventory_screen_draws_same_result_recipe_book_multi_recipe_offset_icons() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry(20, 2, Some(7), 1),
            shapeless_crafting_recipe_book_entry(21, 2, Some(7), 1),
        ],
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("recipe_book/slot_many_uncraftable"),
                11,
                31,
                25,
                25,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
    let item_positions: Vec<(i32, i32)> = screen
        .floating_items
        .iter()
        .map(|item| (item.x, item.y))
        .collect();
    assert!(item_positions.contains(&(16, 36)));
    assert!(item_positions.contains(&(14, 34)));
    assert!(!item_positions.contains(&(15, 35)));
}

#[test]
fn hud_inventory_screen_cycles_recipe_book_multi_recipe_icon() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry(20, 2, Some(7), 1),
            shapeless_crafting_recipe_book_entry(21, 2, Some(7), 2),
        ],
    });
    let local_state = InventoryHudLocalState {
        recipe_book_tabs: RecipeBookTabSelectionHudState {
            crafting: 1,
            ..RecipeBookTabSelectionHudState::default()
        },
        ..InventoryHudLocalState::default()
    };

    let first_screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        local_state.clone(),
        0.0,
    )
    .unwrap();
    let first_icon = first_screen
        .floating_items
        .iter()
        .find(|item| (item.x, item.y) == (15, 35))
        .map(|item| item.icon.clone())
        .expect("first multi-recipe icon");

    world.apply_world_time(PlayTime {
        game_time: 30,
        clock_updates: Vec::new(),
    });
    let second_screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        local_state,
        0.0,
    )
    .unwrap();
    let second_icon = second_screen
        .floating_items
        .iter()
        .find(|item| (item.x, item.y) == (15, 35))
        .map(|item| item.icon.clone())
        .expect("second multi-recipe icon");

    assert_ne!(first_icon, second_icon);
}

#[test]
fn hud_inventory_screen_projects_recipe_book_overlay_picker() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry_with_display_ingredients(
                20,
                2,
                Some(7),
                1,
                vec![stonecutter_item_display(1)],
            ),
            shapeless_crafting_recipe_book_entry_with_display_ingredients(
                21,
                2,
                Some(7),
                2,
                vec![stonecutter_item_display(2)],
            ),
        ],
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            recipe_book_overlay: Some(RecipeBookOverlayHudState {
                book_type: bbb_protocol::packets::RecipeBookType::Crafting,
                tab_index: 1,
                page_index: 0,
                button_index: 0,
                x: 11,
                y: 31,
            }),
            cursor_position: Some((41, 37)),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("recipe_book/overlay_recipe"),
                11,
                31,
                58,
                33,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("recipe_book/crafting_overlay_disabled"),
                15,
                36,
                24,
                24,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
    assert!(screen.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite(
                    "recipe_book/crafting_overlay_disabled_highlighted",
                ),
                40,
                36,
                24,
                24,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));
    let all_item_positions: Vec<(i32, i32, f32)> = screen
        .floating_items
        .iter()
        .map(|item| (item.x, item.y, item.scale))
        .collect();
    let item_positions: Vec<(i32, i32)> = screen
        .floating_items
        .iter()
        .filter(|item| (item.scale - RECIPE_BOOK_OVERLAY_ITEM_SCALE).abs() < 1e-6)
        .map(|item| {
            assert!(!item.draw_decorations);
            (item.x, item.y)
        })
        .collect();
    assert!(
        item_positions.contains(&(17, 38)),
        "overlay item positions: {item_positions:?}; all positions: {all_item_positions:?}"
    );
    assert!(
        item_positions.contains(&(42, 38)),
        "overlay item positions: {item_positions:?}; all positions: {all_item_positions:?}"
    );
}

#[test]
fn hud_inventory_screen_cycles_recipe_book_overlay_tag_ingredient_icon() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    apply_item_tags(&mut world, vec![("minecraft:planks", vec![1, 2])]);
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry_with_display_ingredients(
                20,
                2,
                Some(7),
                1,
                vec![slot_display_tag("minecraft:planks")],
            ),
            shapeless_crafting_recipe_book_entry_with_display_ingredients(
                21,
                2,
                Some(7),
                2,
                vec![slot_display_tag("minecraft:planks")],
            ),
        ],
    });
    let local_state = InventoryHudLocalState {
        recipe_book_tabs: RecipeBookTabSelectionHudState {
            crafting: 1,
            ..RecipeBookTabSelectionHudState::default()
        },
        recipe_book_overlay: Some(RecipeBookOverlayHudState {
            book_type: bbb_protocol::packets::RecipeBookType::Crafting,
            tab_index: 1,
            page_index: 0,
            button_index: 0,
            x: 11,
            y: 31,
        }),
        ..InventoryHudLocalState::default()
    };

    let first_screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        local_state.clone(),
        0.0,
    )
    .unwrap();
    let first_icon = first_screen
        .floating_items
        .iter()
        .find(|item| (item.x, item.y) == (17, 38))
        .map(|item| item.icon.clone())
        .expect("first tag overlay ingredient");

    world.apply_world_time(PlayTime {
        game_time: 30,
        clock_updates: Vec::new(),
    });
    let second_screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        local_state,
        0.0,
    )
    .unwrap();
    let second_icon = second_screen
        .floating_items
        .iter()
        .find(|item| (item.x, item.y) == (17, 38))
        .map(|item| item.icon.clone())
        .expect("second tag overlay ingredient");

    assert_ne!(first_icon, second_icon);
}

#[test]
fn hud_inventory_screen_cycles_composite_recipe_book_overlay_ingredient_icon() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry_with_display_ingredients(
                20,
                2,
                Some(7),
                1,
                vec![slot_display_composite(vec![
                    slot_display_with_remainder(
                        stonecutter_item_display(1),
                        stonecutter_item_display(0),
                    ),
                    stonecutter_item_display(2),
                ])],
            ),
            shapeless_crafting_recipe_book_entry_with_display_ingredients(
                21,
                2,
                Some(7),
                2,
                vec![stonecutter_item_display(2)],
            ),
        ],
    });
    let local_state = InventoryHudLocalState {
        recipe_book_tabs: RecipeBookTabSelectionHudState {
            crafting: 1,
            ..RecipeBookTabSelectionHudState::default()
        },
        recipe_book_overlay: Some(RecipeBookOverlayHudState {
            book_type: bbb_protocol::packets::RecipeBookType::Crafting,
            tab_index: 1,
            page_index: 0,
            button_index: 0,
            x: 11,
            y: 31,
        }),
        ..InventoryHudLocalState::default()
    };

    let first_screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        local_state.clone(),
        0.0,
    )
    .unwrap();
    let first_icon = first_screen
        .floating_items
        .iter()
        .find(|item| (item.x, item.y) == (17, 38))
        .map(|item| item.icon.clone())
        .expect("first composite overlay ingredient");

    world.apply_world_time(PlayTime {
        game_time: 30,
        clock_updates: Vec::new(),
    });
    let second_screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        local_state,
        0.0,
    )
    .unwrap();
    let second_icon = second_screen
        .floating_items
        .iter()
        .find(|item| (item.x, item.y) == (17, 38))
        .map(|item| item.icon.clone())
        .expect("second composite overlay ingredient");

    assert_ne!(first_icon, second_icon);
}
