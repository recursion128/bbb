use super::*;

#[test]
fn hud_inventory_screen_projects_smithing_error_layer() {
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
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 0,
        value: 1,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/smithing.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/smithing/error"),
                65,
                46,
                28,
                21,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_cartography_table_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(38), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Texture("textures/gui/container/cartography_table.png"),
            0,
            0,
            176,
            166,
            [0.0, 0.0],
            [176.0 / 256.0, 166.0 / 256.0],
        )]
    );
    assert_eq!(screen.hovered_slot_id, Some(38));
    assert_eq!(screen.slots.len(), 39);
    let map = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((map.x, map.y), (15, 15));
    let additional = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((additional.x, additional.y), (15, 52));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
    assert_eq!((result.x, result.y), (145, 39));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 38).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_cartography_table_map_frame() {
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[0] = map_stack(100, 1, 42, None);
    let world = cartography_table_world_with_items(items);

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture(
                    "textures/gui/container/cartography_table.png"
                ),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/cartography_table/map"),
                67,
                13,
                66,
                66,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert!(screen.map_previews.is_empty());
}

#[test]
fn hud_inventory_screen_projects_cartography_table_result_modes() {
    let cases = [
        (
            map_stack(
                100,
                1,
                42,
                Some(bbb_protocol::packets::MapPostProcessingSummary::Scale),
            ),
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/cartography_table/scaled_map"),
                67,
                13,
                66,
                66,
                [0.0, 0.0],
                [1.0, 1.0],
            )],
        ),
        (
            map_stack(100, 2, 42, None),
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Sprite(
                        "container/cartography_table/duplicated_map",
                    ),
                    83,
                    13,
                    50,
                    66,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Sprite(
                        "container/cartography_table/duplicated_map",
                    ),
                    67,
                    29,
                    50,
                    66,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ],
        ),
        (
            map_stack(
                100,
                1,
                42,
                Some(bbb_protocol::packets::MapPostProcessingSummary::Lock),
            ),
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Sprite("container/cartography_table/map"),
                    67,
                    13,
                    66,
                    66,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Sprite("container/cartography_table/locked"),
                    118,
                    60,
                    10,
                    14,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ],
        ),
    ];

    for (result_stack, expected_layers) in cases {
        let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
        items[0] = map_stack(100, 1, 42, None);
        items[2] = result_stack;
        let world = cartography_table_world_with_items(items);

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

        assert_eq!(&screen.background_layers[1..], expected_layers.as_slice());
    }
}

#[test]
fn hud_inventory_screen_projects_cartography_map_previews_with_vanilla_strata() {
    let cases = [
        (
            map_stack(100, 1, 42, None),
            vec![HudInventoryMapPreview {
                map_id: 42,
                x: 71,
                y: 17,
                scale: 0.45,
                after_background_layer: 1,
            }],
        ),
        (
            map_stack(
                100,
                1,
                42,
                Some(bbb_protocol::packets::MapPostProcessingSummary::Scale),
            ),
            vec![HudInventoryMapPreview {
                map_id: 42,
                x: 85,
                y: 31,
                scale: 0.226,
                after_background_layer: 1,
            }],
        ),
        (
            map_stack(100, 2, 42, None),
            vec![
                HudInventoryMapPreview {
                    map_id: 42,
                    x: 86,
                    y: 16,
                    scale: 0.34,
                    after_background_layer: 1,
                },
                HudInventoryMapPreview {
                    map_id: 42,
                    x: 70,
                    y: 32,
                    scale: 0.34,
                    after_background_layer: 2,
                },
            ],
        ),
        (
            map_stack(
                100,
                1,
                42,
                Some(bbb_protocol::packets::MapPostProcessingSummary::Lock),
            ),
            vec![HudInventoryMapPreview {
                map_id: 42,
                x: 71,
                y: 17,
                scale: 0.45,
                after_background_layer: 1,
            }],
        ),
    ];

    for (result_stack, expected_previews) in cases {
        let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
        items[0] = map_stack(100, 1, 42, None);
        items[2] = result_stack;
        let mut world = cartography_table_world_with_items(items);
        apply_cartography_map_state(&mut world, 42, 2, false);

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

        assert_eq!(screen.map_previews, expected_previews);
    }
}

#[test]
fn cartography_presentation_uses_world_item_profiles_without_item_runtime() {
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[0] = map_stack(100, 1, 42, None);
    items[1] = item_stack(200, 1);
    let mut world = cartography_table_world_with_items(items);
    world.set_cartography_additional_items(BTreeMap::from([(
        200,
        CartographyAdditionalItemKind::Paper,
    )]));
    apply_cartography_map_state(&mut world, 42, 2, false);

    let presentation = cartography_table_presentation(&world, None);

    assert_eq!(presentation.mode, Some(CartographyResultMode::Scaled));
    assert!(!presentation.invalid_transform);
    assert_eq!(
        presentation.map_previews,
        vec![HudInventoryMapPreview {
            map_id: 42,
            x: 85,
            y: 31,
            scale: 0.226,
            after_background_layer: 1,
        }]
    );
}

#[test]
fn cartography_presentation_keeps_known_mode_without_missing_map_pixels() {
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[0] = map_stack(100, 1, 42, None);
    items[1] = item_stack(200, 1);
    let mut world = cartography_table_world_with_items(items);
    world.set_cartography_additional_items(BTreeMap::from([(
        200,
        CartographyAdditionalItemKind::Paper,
    )]));

    let presentation = cartography_table_presentation(&world, None);

    assert_eq!(presentation.mode, Some(CartographyResultMode::Scaled));
    assert!(!presentation.invalid_transform);
    assert_eq!(
        presentation.background_layers[1].texture,
        HudInventoryBackgroundTexture::Sprite("container/cartography_table/scaled_map")
    );
    assert!(presentation.map_previews.is_empty());
}

#[test]
fn cartography_presentation_matches_vanilla_invalid_transform_modes() {
    let cases = [
        (
            CartographyAdditionalItemKind::Paper,
            4,
            false,
            CartographyResultMode::Map,
            HudInventoryBackgroundTexture::Sprite("container/cartography_table/map"),
        ),
        (
            CartographyAdditionalItemKind::Paper,
            2,
            true,
            CartographyResultMode::Map,
            HudInventoryBackgroundTexture::Sprite("container/cartography_table/map"),
        ),
        (
            CartographyAdditionalItemKind::GlassPane,
            2,
            true,
            CartographyResultMode::Locked,
            HudInventoryBackgroundTexture::Sprite("container/cartography_table/locked"),
        ),
    ];

    for (kind, scale, locked, expected_mode, last_texture) in cases {
        let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
        items[0] = map_stack(100, 1, 42, None);
        items[1] = item_stack(200, 1);
        let mut world = cartography_table_world_with_items(items);
        world.set_cartography_additional_items(BTreeMap::from([(200, kind)]));
        apply_cartography_map_state(&mut world, 42, scale, locked);

        let presentation = cartography_table_presentation(&world, None);

        assert_eq!(presentation.mode, Some(expected_mode));
        assert!(presentation.invalid_transform);
        assert_eq!(
            presentation.background_layers[1].texture,
            HudInventoryBackgroundTexture::Sprite("container/cartography_table/error")
        );
        assert_eq!(
            presentation.background_layers.last().unwrap().texture,
            last_texture
        );
        assert_eq!(presentation.map_previews.len(), 1);
        assert_eq!(presentation.map_previews[0].after_background_layer, 2);
    }
}

#[test]
fn cartography_presentation_uses_result_when_item_identity_is_unavailable() {
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[0] = map_stack(100, 1, 42, None);
    items[1] = item_stack(999, 1);
    items[2] = map_stack(
        100,
        1,
        42,
        Some(bbb_protocol::packets::MapPostProcessingSummary::Lock),
    );
    let mut world = cartography_table_world_with_items(items);
    apply_cartography_map_state(&mut world, 42, 2, false);

    let presentation = cartography_table_presentation(&world, None);

    assert_eq!(presentation.mode, Some(CartographyResultMode::Locked));
    assert!(!presentation.invalid_transform);
    assert_eq!(presentation.map_previews.len(), 1);
    assert_eq!(presentation.map_previews[0].after_background_layer, 1);
}

#[test]
fn cartography_additional_item_resource_ids_match_vanilla_items() {
    assert_eq!(
        cartography_additional_item_for_resource_id("minecraft:paper"),
        Some(CartographyAdditionalItemKind::Paper)
    );
    assert_eq!(
        cartography_additional_item_for_resource_id("minecraft:map"),
        Some(CartographyAdditionalItemKind::EmptyMap)
    );
    assert_eq!(
        cartography_additional_item_for_resource_id("minecraft:glass_pane"),
        Some(CartographyAdditionalItemKind::GlassPane)
    );
    assert_eq!(
        cartography_additional_item_for_resource_id("minecraft:filled_map"),
        None
    );
}

#[test]
fn hud_inventory_screen_projects_stonecutter_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 38],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(37), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/stonecutter.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/stonecutter/scroller_disabled"),
                119,
                15,
                12,
                15,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(37));
    assert_eq!(screen.slots.len(), 38);
    let input = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((input.x, input.y), (20, 33));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((result.x, result.y), (143, 33));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 37).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_stonecutter_recipe_buttons_and_scroller() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 38];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_update_recipes(bbb_protocol::packets::UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: (0..16)
            .map(|index| stonecutter_recipe_display(42, 100 + index, 1))
            .collect(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 0,
        value: 5,
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            stonecutter_recipe_scroll_row: Some(1),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(
        screen.background_layers[1],
        hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/stonecutter/scroller"),
            119,
            56,
            12,
            15,
            [0.0, 0.0],
            [1.0, 1.0],
        )
    );
    let recipe_layers: Vec<_> = screen
        .background_layers
        .iter()
        .filter(|layer| {
            matches!(
                layer.texture,
                HudInventoryBackgroundTexture::Sprite("container/stonecutter/recipe")
                    | HudInventoryBackgroundTexture::Sprite(
                        "container/stonecutter/recipe_selected"
                    )
            )
        })
        .collect();
    assert_eq!(recipe_layers.len(), 12);
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/stonecutter/recipe_selected"),
            68,
            15,
            16,
            18,
            [0.0, 0.0],
            [1.0, 1.0],
        )));

    let option_stacks = stonecutter_visible_recipe_option_stacks(&world, 1);
    assert_eq!(option_stacks.len(), 12);
    assert_eq!(
        option_stacks[0],
        StonecutterRecipeOptionStack {
            x: 52,
            y: 16,
            stack: item_stack(104, 1),
        }
    );
    assert_eq!(
        option_stacks[11],
        StonecutterRecipeOptionStack {
            x: 100,
            y: 52,
            stack: item_stack(115, 1),
        }
    );
}

#[test]
fn stonecutter_buttons_and_tooltips_resolve_all_builtin_slot_displays() {
    let root = unique_runtime_temp_dir("stonecutter-slot-displays");
    write_stonecutter_slot_display_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let world = stonecutter_world_with_builtin_slot_displays(1);

    let option_stacks = stonecutter_visible_recipe_option_stacks(&world, 0);
    assert_eq!(option_stacks.len(), 10);
    let stack_at = |position: i32| {
        let x = STONECUTTER_RECIPE_BUTTON_X
            + position % STONECUTTER_RECIPE_BUTTON_COLUMNS * STONECUTTER_RECIPE_BUTTON_WIDTH;
        let y = STONECUTTER_RECIPE_ITEM_Y
            + position / STONECUTTER_RECIPE_BUTTON_COLUMNS * STONECUTTER_RECIPE_BUTTON_HEIGHT;
        option_stacks
            .iter()
            .find(|option| (option.x, option.y) == (x, y))
            .map(|option| &option.stack)
    };

    assert_eq!(stack_at(0), None);
    for position in 1..=10 {
        assert_eq!(stack_at(position).unwrap().item_id, Some(1));
    }
    assert_eq!(
        stack_at(2)
            .unwrap()
            .component_patch
            .potion_contents
            .potion_id,
        Some(0)
    );
    assert!(world.item_stack_has_effective_component(
        stack_at(3).unwrap(),
        VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID,
    ));
    assert_eq!(stack_at(5).unwrap().count, 3);
    assert_eq!(
        stack_at(7).unwrap().component_patch.dyed_color,
        Some(0xB0_2E_26)
    );
    assert_eq!(
        stack_at(8).unwrap().component_patch.armor_trim_material_id,
        Some(0)
    );
    assert_eq!(
        stack_at(8).unwrap().component_patch.armor_trim_pattern_id,
        Some(0)
    );

    for position in 0..=10 {
        let cursor = (
            STONECUTTER_RECIPE_BUTTON_X
                + position % STONECUTTER_RECIPE_BUTTON_COLUMNS * STONECUTTER_RECIPE_BUTTON_WIDTH,
            STONECUTTER_RECIPE_ITEM_Y
                + position / STONECUTTER_RECIPE_BUTTON_COLUMNS * STONECUTTER_RECIPE_BUTTON_HEIGHT,
        );
        let screen = hud_inventory_screen_with_local_state(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            None,
            InventoryHudLocalState {
                cursor_position: Some(cursor),
                ..InventoryHudLocalState::default()
            },
            0.0,
        )
        .unwrap();
        assert_eq!(screen.floating_items.len(), 10);
        if position == 0 {
            assert_eq!(screen.tooltip, None);
        } else {
            let tooltip = screen.tooltip.unwrap();
            assert_eq!(tooltip.slot_id, u16::MAX);
            assert_eq!((tooltip.x, tooltip.y), cursor);
            assert_eq!(tooltip.lines[0].text, "Test Sword");
        }
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn stonecutter_buttons_and_tooltips_follow_current_tag_and_trim_context() {
    let root = unique_runtime_temp_dir("stonecutter-slot-display-context");
    write_stonecutter_slot_display_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = stonecutter_world_with_builtin_slot_displays(1);

    set_stonecutter_item_tag(&mut world, vec![2]);
    let tag_option = stonecutter_visible_recipe_option_stacks(&world, 0)
        .into_iter()
        .find(|option| (option.x, option.y) == stonecutter_recipe_item_position(6))
        .unwrap();
    assert_eq!(tag_option.stack.item_id, Some(2));
    let tag_tooltip =
        stonecutter_recipe_tooltip_at(&world, &item_runtime, stonecutter_recipe_item_position(6));
    assert_eq!(tag_tooltip.lines[0].text, "Command Block");

    clear_stonecutter_item_tags(&mut world);
    assert!(stonecutter_visible_recipe_option_stacks(&world, 0)
        .into_iter()
        .all(|option| (option.x, option.y) != stonecutter_recipe_item_position(6)));
    let removed_tag_screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some(stonecutter_recipe_item_position(6)),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();
    assert_eq!(removed_tag_screen.tooltip, None);

    let initial_trim_tooltip =
        stonecutter_recipe_tooltip_at(&world, &item_runtime, stonecutter_recipe_item_position(8));
    assert!(
        initial_trim_tooltip
            .lines
            .iter()
            .any(|line| line.text.trim() == "Spiral"),
        "{:#?}",
        initial_trim_tooltip.lines
    );
    assert!(
        initial_trim_tooltip
            .lines
            .iter()
            .any(|line| line.text.trim() == "Copper"),
        "{:#?}",
        initial_trim_tooltip.lines
    );

    world.record_registry_entries(
        TRIM_MATERIAL_REGISTRY,
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:opal",
            stonecutter_trim_material_nbt("opal", "Opal"),
        )],
    );
    world.record_registry_entries(
        TRIM_PATTERN_REGISTRY,
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:wave",
            stonecutter_trim_pattern_nbt("example:wave", "Wave", false),
        )],
    );
    world.set_default_item_component_codec_values(BTreeMap::from([
        (
            2,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_DYE_TYPE_ID, serde_json::json!("red"))]),
        ),
        (
            3,
            BTreeMap::from([(
                VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
                serde_json::json!("example:opal"),
            )]),
        ),
    ]));
    world.begin_registry_configuration();
    world.finish_registry_configuration().unwrap();
    world.apply_update_recipes(stonecutter_builtin_update_recipes(2));

    let trim_option = stonecutter_visible_recipe_option_stacks(&world, 0)
        .into_iter()
        .find(|option| (option.x, option.y) == stonecutter_recipe_item_position(8))
        .unwrap();
    assert_eq!(
        trim_option.stack.component_patch.armor_trim_material_id,
        Some(1)
    );
    assert_eq!(
        trim_option.stack.component_patch.armor_trim_pattern_id,
        Some(1)
    );
    let changed_trim_tooltip =
        stonecutter_recipe_tooltip_at(&world, &item_runtime, stonecutter_recipe_item_position(8));
    assert!(
        changed_trim_tooltip
            .lines
            .iter()
            .any(|line| line.text.trim() == "Wave"),
        "{:#?}",
        changed_trim_tooltip.lines
    );
    assert!(
        changed_trim_tooltip
            .lines
            .iter()
            .any(|line| line.text.trim() == "Opal"),
        "{:#?}",
        changed_trim_tooltip.lines
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_item_count_label_follows_vanilla_stack_count_rule() {
    assert_eq!(
        hud_item_count_label_for_stack(&item_stack(42, 64)),
        Some(HudItemCountLabel::new("64"))
    );
    assert_eq!(hud_item_count_label_for_stack(&item_stack(42, 1)), None);
    assert_eq!(hud_item_count_label_for_stack(&item_stack(42, 0)), None);
    assert_eq!(
        hud_item_count_label_for_stack(&bbb_protocol::packets::ItemStackSummary::empty()),
        None
    );
}

#[test]
fn hud_item_cooldown_progress_uses_world_cooldown_group_state() {
    let mut world = WorldStore::new();
    world.apply_cooldown(bbb_protocol::packets::Cooldown {
        cooldown_group: "minecraft:ender_pearl".to_string(),
        duration: 20,
    });
    world.advance_item_cooldowns(5);
    let mut stack = item_stack(42, 1);
    stack.component_patch.use_cooldown_group = Some("minecraft:ender_pearl".to_string());

    assert_eq!(
        hud_item_cooldown_progress_for_stack(&world, None, &stack, 0.5),
        Some(0.725)
    );
    assert_eq!(
        hud_item_cooldown_progress_for_stack(&world, None, &stack, 1.5),
        Some(0.7)
    );

    stack.component_patch.use_cooldown_group = Some("minecraft:wind_charge".to_string());
    assert_eq!(
        hud_item_cooldown_progress_for_stack(&world, None, &stack, 0.0),
        None
    );
    assert_eq!(
        hud_item_cooldown_progress_for_stack(
            &world,
            None,
            &bbb_protocol::packets::ItemStackSummary::empty(),
            0.0
        ),
        None
    );
}

#[test]
fn item_cooldown_group_requires_runtime_for_default_item_group() {
    let stack = item_stack(42, 1);
    assert_eq!(item_cooldown_group(None, &stack), None);

    let mut explicit_group = stack;
    explicit_group.component_patch.use_cooldown_group = Some("minecraft:custom_group".to_string());
    assert_eq!(
        item_cooldown_group(None, &explicit_group),
        Some("minecraft:custom_group".to_string())
    );
}

#[test]
fn hud_item_durability_bar_follows_vanilla_damage_formula() {
    let world = WorldStore::new();
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &item_stack_with_damage(42, 1, 100, 25, false)),
        Some(HudItemDurabilityBar::new(10, [127.0 / 255.0, 1.0, 0.0]))
    );
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &item_stack_with_damage(42, 1, 100, 100, false)),
        Some(HudItemDurabilityBar::new(0, [1.0, 0.0, 0.0]))
    );
}

#[test]
fn hud_item_durability_bar_requires_damageable_damaged_stack() {
    let world = WorldStore::new();
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &item_stack_with_damage(42, 1, 100, 0, false)),
        None
    );
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &item_stack_with_damage(42, 1, 100, 25, true)),
        None
    );
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &item_stack_with_damage(42, 0, 100, 25, false)),
        None
    );
    let mut missing_damage = item_stack(42, 1);
    missing_damage.component_patch.max_damage = Some(100);
    missing_damage
        .component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID);
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &missing_damage),
        None
    );

    // No patch `max_damage` and an empty default-item-max-damage table (as
    // when the registry default table hasn't been populated, or the item has
    // no vanilla default): still None, since there is nothing to fall back to.
    let mut missing_max_damage = item_stack(42, 1);
    missing_max_damage.component_patch.damage = Some(25);
    missing_max_damage
        .component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &missing_max_damage),
        None
    );
}

#[test]
fn hud_item_durability_bar_falls_back_to_default_item_max_damage_table() {
    let mut world = WorldStore::new();
    crate::item_registry_test_fixture::install_damageable_item_defaults(&mut world, 42, 100);

    // Vanilla protocol patches usually only carry `damage` for a damaged
    // stack, since `max_damage` is a registry default component. The HUD/
    // inventory durability bar must still show, matching the same
    // width/color formula as an explicit patch `max_damage`.
    let mut default_max_damage_only = item_stack(42, 1);
    default_max_damage_only.component_patch.added_type_ids =
        vec![bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID];
    default_max_damage_only.component_patch.damage = Some(25);
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &default_max_damage_only),
        Some(HudItemDurabilityBar::new(10, [127.0 / 255.0, 1.0, 0.0]))
    );

    // An explicit patch `max_damage` still takes priority over the registry
    // default table (100): using the patched 50 (not 100) yields width 10,
    // not the width 12 that the default-table value would produce.
    let mut explicit_max_damage = item_stack(42, 1);
    explicit_max_damage.component_patch.added_type_ids = vec![
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
    ];
    explicit_max_damage.component_patch.max_damage = Some(50);
    explicit_max_damage.component_patch.damage = Some(10);
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &explicit_max_damage),
        Some(HudItemDurabilityBar::new(
            10,
            vanilla_hsv_to_rgb_unit(0.8 / 3.0, 1.0, 1.0)
        ))
    );

    // `minecraft:unbreakable` suppresses the bar even when the registry
    // default table has a `max_damage` entry for the item.
    let mut unbreakable = item_stack(42, 1);
    unbreakable.component_patch.added_type_ids = vec![
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
    ];
    unbreakable.component_patch.damage = Some(25);
    unbreakable.component_patch.unbreakable = true;
    assert_eq!(
        hud_item_durability_bar_for_stack(&world, &unbreakable),
        None
    );

    // An item with no registry default entry (e.g. a non-damageable item)
    // still falls back to None when the patch omits `max_damage`.
    let mut other_item = item_stack(7, 1);
    other_item.component_patch.damage = Some(25);
    assert_eq!(hud_item_durability_bar_for_stack(&world, &other_item), None);
}

#[test]
fn block_destroy_overlays_include_server_progress_and_keep_highest_per_position() {
    let mut world = WorldStore::new();
    let textures = destroy_stage_test_textures();
    let pos = bbb_protocol::packets::BlockPos { x: 2, y: 3, z: 4 };
    assert!(
        world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
            id: 10,
            pos,
            progress: 2,
        })
    );
    assert!(
        world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
            id: 11,
            pos,
            progress: 7,
        })
    );
    assert!(
        world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
            id: 12,
            pos: bbb_protocol::packets::BlockPos { x: 1, y: 3, z: 4 },
            progress: 3,
        })
    );

    let stages = world.block_destruction_render_stages();
    let overlays = block_destroy_overlays_from_stages(
        &world,
        &textures,
        &stages,
        Some(CameraPose {
            position: [0.0, 0.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }),
    );

    assert_eq!(
        stages,
        vec![
            (BlockPos { x: 1, y: 3, z: 4 }, 3),
            (BlockPos { x: 2, y: 3, z: 4 }, 7)
        ]
    );
    assert_eq!(overlays.len(), 2);
    assert_eq!(overlays[0].pos, [1, 3, 4]);
    assert_eq!(overlays[0].uv, textures.destroy_stage_uv_rect(3).unwrap());
    assert_eq!(overlays[1].pos, [2, 3, 4]);
    assert_eq!(overlays[1].uv, textures.destroy_stage_uv_rect(7).unwrap());
}

#[test]
fn block_destroy_overlays_merge_local_stage_with_server_progress_per_position() {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let pos = BlockPos { x: 0, y: 1, z: 3 };
    set_lightmap_test_block(&mut world, pos, 9);
    assert_eq!(
        world.probe_block(pos).unwrap().block_name.as_deref(),
        Some("minecraft:grass_block")
    );
    world.set_local_destroying_block_hit(pos, bbb_protocol::packets::Direction::North);
    for _ in 0..10 {
        assert_eq!(world.advance_local_destroying_block_tick(), None);
    }
    assert_eq!(
        world.local_player().interaction.destroying_block_stage,
        Some(5)
    );
    assert!(
        world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
            id: 10,
            pos: bbb_protocol::packets::BlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            progress: 2,
        })
    );
    let textures = destroy_stage_test_textures();

    let stages = world.block_destruction_render_stages();
    let overlays = block_destroy_overlays_from_stages(
        &world,
        &textures,
        &stages,
        Some(CameraPose {
            position: [0.0, 0.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }),
    );

    assert_eq!(stages, vec![(pos, 5)]);
    assert_eq!(overlays.len(), 1);
    assert_eq!(overlays[0].pos, [0, 1, 3]);
    assert_eq!(overlays[0].uv, textures.destroy_stage_uv_rect(5).unwrap());
    // The overlay now carries the block's render shape (grass_block has no model in the test
    // textures, so it projects to the full-cube crack).
    assert_eq!(
        overlays[0].shape,
        bbb_client_render::terrain::TerrainRenderShape::Cube
    );
}

#[test]
fn block_destroy_overlays_skip_missing_destroy_stage_textures() {
    let mut world = WorldStore::new();
    let textures = TerrainTextureState::default();
    assert!(
        world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
            id: 10,
            pos: bbb_protocol::packets::BlockPos { x: 2, y: 3, z: 4 },
            progress: 2,
        })
    );

    let stages = world.block_destruction_render_stages();
    assert_eq!(stages, vec![(BlockPos { x: 2, y: 3, z: 4 }, 2)]);
    assert!(block_destroy_overlays_from_stages(
        &world,
        &textures,
        &stages,
        Some(CameraPose {
            position: [0.0, 0.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }),
    )
    .is_empty());
}

#[test]
fn block_destroy_overlays_use_camera_eye_and_vanilla_thirty_two_block_center_bound() {
    let world = WorldStore::new();
    let textures = destroy_stage_test_textures();
    let stages = [
        (BlockPos { x: 31, y: 0, z: 0 }, 1),
        (BlockPos { x: 32, y: 0, z: 0 }, 2),
        (BlockPos { x: 0, y: 33, z: 0 }, 3),
    ];
    let camera = Some(CameraPose {
        position: [-0.5, -1.12, 0.5],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 1.62,
    });

    let overlays = block_destroy_overlays_from_stages(&world, &textures, &stages, camera);

    assert_eq!(overlays.len(), 1);
    assert_eq!(overlays[0].pos, [31, 0, 0]);
    assert!(block_destroy_overlays_from_stages(&world, &textures, &stages, None).is_empty());
}

#[test]
fn block_selection_projection_uses_current_material_catalog_surface_width_and_style() {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.insert_decoded_chunk(empty_lightmap_test_chunk(world.dimension()));
    let pos = BlockPos { x: 0, y: 1, z: 3 };
    set_lightmap_test_block(&mut world, pos, 9);
    assert_eq!(
        world.probe_block(pos).unwrap().block_name.as_deref(),
        Some("minecraft:grass_block")
    );
    let selection = || BlockSelectionProjection {
        pos,
        boxes: vec![bbb_client_render::SelectionBox {
            min: [0.0, 1.0, 3.0],
            max: [1.0, 2.0, 4.0],
        }],
    };

    let solid = block_selection_render_state(
        &world,
        &TerrainTextureState::default(),
        selection(),
        1920,
        false,
    );
    assert_eq!(solid.phase, BlockSelectionPhase::BeforeTranslucent);
    assert_eq!(solid.line_width, 2.5);
    assert!(!solid.high_contrast);

    let root = unique_runtime_temp_dir("block-selection-material-phase");
    let assets = runtime_assets_dir(&root);
    write_runtime_json(
        &assets.join("blockstates").join("grass_block.json"),
        r#"{"variants":{"":{"model":"minecraft:block/grass_block"}}}"#,
    );
    write_runtime_json(
        &assets.join("models").join("block").join("grass_block.json"),
        r##"{
            "textures":{"particle":"#all","all":{"sprite":"minecraft:block/glass","force_translucent":true}},
            "elements":[{"from":[0,0,0],"to":[16,16,16],"faces":{
                "down":{"texture":"#all"},"up":{"texture":"#all"},
                "north":{"texture":"#all"},"south":{"texture":"#all"},
                "west":{"texture":"#all"},"east":{"texture":"#all"}
            }}]
        }"##,
    );
    let models = bbb_client_assets::PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let atlas = bbb_client_assets::AtlasPacker::new(16, 1)
        .unwrap()
        .stitch(&[bbb_client_assets::SpriteImage::new(
            "minecraft:block/glass",
            1,
            1,
            vec![255, 255, 255, 255],
        )
        .unwrap()])
        .unwrap();
    let textures = TerrainTextureState::from_layout(&atlas.layout, Some(models), None, None);
    let translucent = block_selection_render_state(&world, &textures, selection(), 3840, true);
    assert_eq!(translucent.phase, BlockSelectionPhase::AfterTranslucent);
    assert_eq!(translucent.line_width, 5.0);
    assert!(translucent.high_contrast);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_entity_crumbling_acceptance_matches_vanilla_model_renderers() {
    let accepted = [
        bbb_client_render::EntityModelKind::Chest {
            texture: bbb_client_render::ChestModelTexture::Normal,
            half: bbb_client_render::ChestModelHalf::Single,
        },
        bbb_client_render::EntityModelKind::Sign {
            wood: bbb_client_render::SignModelWood::Oak,
            attachment: bbb_client_render::SignModelAttachment::Standing,
        },
        bbb_client_render::EntityModelKind::Bed {
            color: bbb_client_render::EntityDyeColor::Red,
            part: bbb_client_render::BedModelPart::Head,
        },
        bbb_client_render::EntityModelKind::Bell,
        bbb_client_render::EntityModelKind::ShulkerBox {
            color: None,
            facing: bbb_client_render::EntityAttachmentFace::Up,
        },
        bbb_client_render::EntityModelKind::Banner {
            wall: false,
            flag_only: false,
            base_color: bbb_client_render::EntityDyeColor::White,
            layers: [None; 16],
        },
        bbb_client_render::EntityModelKind::EnchantingBook,
        bbb_client_render::EntityModelKind::LecternBook,
        bbb_client_render::EntityModelKind::SkullBlock {
            skull: bbb_client_render::EntityCustomHeadSkull::Skeleton,
            attachment: bbb_client_render::SkullBlockModelAttachment::Ground,
        },
        bbb_client_render::EntityModelKind::Conduit {
            part: bbb_client_render::ConduitModelPart::Shell,
        },
        bbb_client_render::EntityModelKind::Conduit {
            part: bbb_client_render::ConduitModelPart::Cage,
        },
    ];
    assert!(accepted
        .into_iter()
        .all(block_entity_model_accepts_crumbling));

    let rejected = [
        bbb_client_render::EntityModelKind::DecoratedPot {
            back: None,
            left: None,
            right: None,
            front: None,
        },
        bbb_client_render::EntityModelKind::EndPortalBlock {
            kind: bbb_client_render::EndPortalModelKind::EndPortal,
            faces: bbb_client_render::EndPortalModelFaces::Y_AXIS,
        },
        bbb_client_render::EntityModelKind::Conduit {
            part: bbb_client_render::ConduitModelPart::OuterWind { phase: 1 },
        },
        bbb_client_render::EntityModelKind::Conduit {
            part: bbb_client_render::ConduitModelPart::InnerWind { vertical: false },
        },
        bbb_client_render::EntityModelKind::Conduit {
            part: bbb_client_render::ConduitModelPart::Eye { open: true },
        },
    ];
    assert!(rejected
        .into_iter()
        .all(|kind| !block_entity_model_accepts_crumbling(kind)));
}

#[test]
fn block_entity_crumbling_uses_shared_highest_stage_and_clears_ineligible_models() {
    let textures = destroy_stage_test_textures();
    let pos = BlockPos { x: 2, y: 3, z: 4 };
    let stages = vec![(pos, 7)];
    let stale = Some(bbb_client_render::BlockEntityCrumbling {
        origin: [99.0; 3],
        uv: textures.destroy_stage_uv_rect(1).unwrap(),
    });
    let mut instances = vec![
        bbb_client_render::EntityModelInstance::chest(
            -1,
            [2.0, 3.0, 4.0],
            0.0,
            bbb_client_render::ChestModelTexture::Normal,
            bbb_client_render::ChestModelHalf::Single,
        ),
        bbb_client_render::EntityModelInstance::conduit(
            -1,
            [2.0, 3.0, 4.0],
            bbb_client_render::ConduitModelPart::Shell,
        ),
        bbb_client_render::EntityModelInstance::conduit(
            -1,
            [2.0, 3.0, 4.0],
            bbb_client_render::ConduitModelPart::Cage,
        ),
        bbb_client_render::EntityModelInstance::conduit(
            -1,
            [2.0, 3.0, 4.0],
            bbb_client_render::ConduitModelPart::OuterWind { phase: 1 },
        )
        .with_block_entity(|s| s.block_entity_crumbling = stale),
        bbb_client_render::EntityModelInstance::decorated_pot(
            -1,
            [2.0, 3.0, 4.0],
            0.0,
            None,
            None,
            None,
            None,
        )
        .with_block_entity(|s| s.block_entity_crumbling = stale),
        bbb_client_render::EntityModelInstance::bell(-1, [5.0, 6.0, 7.0])
            .with_block_entity(|s| s.block_entity_crumbling = stale),
    ];

    apply_block_entity_crumbling(&mut instances, &stages, &textures);

    let expected = Some(bbb_client_render::BlockEntityCrumbling {
        origin: [2.0, 3.0, 4.0],
        uv: textures.destroy_stage_uv_rect(7).unwrap(),
    });
    assert_eq!(instances[0].block_entity.block_entity_crumbling, expected);
    assert_eq!(instances[1].block_entity.block_entity_crumbling, expected);
    assert_eq!(instances[2].block_entity.block_entity_crumbling, expected);
    for instance in &instances[3..] {
        assert_eq!(instance.block_entity.block_entity_crumbling, None);
    }
}

#[test]
fn block_destroy_render_ticks_respect_frozen_world_ticking_state() {
    let mut world = WorldStore::new();
    assert!(
        world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
            id: 10,
            pos: bbb_protocol::packets::BlockPos { x: 2, y: 3, z: 4 },
            progress: 2,
        })
    );
    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 20.0,
        frozen: true,
    });

    let running_ticks = world.consume_running_render_ticks(420);
    assert_eq!(
        world.advance_block_destruction_render_ticks(running_ticks),
        0
    );
    assert_eq!(world.block_destructions().len(), 1);

    world.apply_ticking_step(bbb_protocol::packets::TickingStep { tick_steps: 420 });
    let running_ticks = world.consume_running_render_ticks(420);
    assert_eq!(
        world.advance_block_destruction_render_ticks(running_ticks),
        1
    );
    assert!(world.block_destructions().is_empty());
    assert_eq!(world.ticking().frozen_ticks_to_run, 0);
}

#[test]
fn entity_client_animations_advance_at_vanilla_tick_interval() {
    let start = Instant::now();
    let mut ticks = ClientAnimationTickState::default();
    let mut world = WorldStore::new();
    world.apply_add_entity(test_add_entity(123, 104));
    assert!(
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 123,
            values: vec![test_bool_data(18, true)],
        })
    );

    assert_eq!(
        world.probe_entity_pick_bounds(123),
        Some(bbb_client_world::EntityPickBoundsState::from_base_size(
            1.4, 1.4, 0.0
        ))
    );

    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start),
        0
    );
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start + Duration::from_millis(49),),
        0
    );
    assert_eq!(
        world.probe_entity_pick_bounds(123),
        Some(bbb_client_world::EntityPickBoundsState::from_base_size(
            1.4, 1.4, 0.0
        ))
    );

    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start + Duration::from_millis(50),),
        1
    );
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start + Duration::from_millis(50),),
        0
    );
    assert_eq!(
        world.probe_entity_pick_bounds(123),
        Some(bbb_client_world::EntityPickBoundsState::from_base_size(
            1.4, 1.4, 0.0
        ))
    );

    assert_eq!(
        advance_entity_client_animations(
            &mut world,
            &mut ticks,
            start + Duration::from_millis(100),
        ),
        1
    );
    assert_eq!(
        world.probe_entity_pick_bounds(123),
        Some(bbb_client_world::EntityPickBoundsState::from_base_size(
            1.4,
            1.4 * (1.0 + 1.0 / 6.0),
            0.0,
        ))
    );

    assert_eq!(
        advance_entity_client_animations(
            &mut world,
            &mut ticks,
            start + Duration::from_millis(350),
        ),
        5
    );
    assert_eq!(
        world.probe_entity_pick_bounds(123),
        Some(bbb_client_world::EntityPickBoundsState::from_base_size(
            1.4, 2.8, 0.0
        ))
    );
}

#[test]
fn entity_client_tick_cadence_tracks_normal_world_tick_rate() {
    let start = Instant::now();
    let mut ticks = ClientAnimationTickState::default();
    let mut world = WorldStore::new();
    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 10.0,
        frozen: false,
    });

    assert_eq!(
        world_client_tick_interval(&world),
        Duration::from_millis(100)
    );
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start),
        0
    );
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start + Duration::from_millis(99),),
        0
    );
    assert!((ticks.entity_partial_tick() - 0.99).abs() < 0.000_001);
    assert_eq!(
        advance_entity_client_animations(
            &mut world,
            &mut ticks,
            start + Duration::from_millis(100),
        ),
        1
    );
    assert_eq!(ticks.entity_partial_tick(), 0.0);

    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 40.0,
        frozen: false,
    });
    assert_eq!(
        world_client_tick_interval(&world),
        Duration::from_millis(50)
    );
}

#[test]
fn native_pause_screens_never_freeze_external_client_or_running_level_ticks() {
    for show_menu in [false, true] {
        let start = Instant::now();
        let mut ticks = ClientAnimationTickState::default();
        let mut world = WorldStore::new();
        world.apply_add_entity(test_add_entity(123, 104));
        let mut input = ClientInputState::new(true);
        if show_menu {
            input.open_debug_pause_screen_with_menu(&world, &mut Default::default(), &None);
        } else {
            input.open_debug_pause_screen_without_menu_for_test();
        }

        assert!(!native_pause_freezes_client_ticks(&input, &world));
        assert_eq!(
            advance_native_client_ticks(&input, &mut world, &mut ticks, start),
            0
        );
        assert_eq!(
            advance_native_client_ticks(
                &input,
                &mut world,
                &mut ticks,
                start + Duration::from_millis(50),
            ),
            1
        );
        assert_eq!(ticks.last_running_level_ticks, 1);
        assert_eq!(
            world.probe_entity(123).unwrap().client_animations.age_ticks,
            1
        );
    }
}

#[test]
fn entity_client_tick_cadence_preserves_residual_across_rate_changes_and_freeze() {
    let start = Instant::now();
    let mut ticks = ClientAnimationTickState::default();
    let mut world = WorldStore::new();

    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start),
        0
    );
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start + Duration::from_millis(25),),
        0
    );
    assert_eq!(ticks.entity_partial_tick(), 0.5);

    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 10.0,
        frozen: false,
    });
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start + Duration::from_millis(75),),
        1
    );
    assert_eq!(ticks.entity_partial_tick(), 0.0);

    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 1.0,
        frozen: true,
    });
    assert_eq!(
        world_client_tick_interval(&world),
        Duration::from_millis(50)
    );
    assert_eq!(
        advance_entity_client_animations(
            &mut world,
            &mut ticks,
            start + Duration::from_millis(125),
        ),
        1
    );
}

#[test]
fn entity_client_tick_batch_shares_the_frozen_step_budget_with_world_time() {
    let start = Instant::now();
    let mut ticks = ClientAnimationTickState::default();
    let mut world = WorldStore::new();
    world.apply_add_entity(test_add_entity(123, 104));
    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 20.0,
        frozen: true,
    });
    world.apply_ticking_step(bbb_protocol::packets::TickingStep { tick_steps: 1 });

    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start),
        0
    );
    let advanced_ticks = advance_entity_client_animations(
        &mut world,
        &mut ticks,
        start + Duration::from_millis(151),
    );
    assert_eq!(advanced_ticks, 3);
    assert_eq!(
        world.probe_entity(123).unwrap().client_animations.age_ticks,
        1
    );
    assert_eq!(ticks.last_running_level_ticks, 1);
    assert_eq!(world.ticking().frozen_ticks_to_run, 0);
}

#[test]
fn entity_client_tick_catch_up_runs_ten_ticks_and_drops_the_rest() {
    let start = Instant::now();
    let mut ticks = ClientAnimationTickState::default();
    let mut world = WorldStore::new();

    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, start),
        0
    );
    let delayed = start + Duration::from_millis(1_060);
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, delayed),
        VANILLA_MAX_CLIENT_TICKS_PER_STEP
    );
    assert_eq!(
        advance_entity_client_animations(&mut world, &mut ticks, delayed),
        0
    );
    assert!((ticks.entity_partial_tick() - 0.2).abs() < 1e-4);
}

#[test]
fn client_tick_end_commands_follow_movement_and_require_active_level() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(LocalPlayerPoseState {
        position: Vec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        ..LocalPlayerPoseState::default()
    });
    let mut counters = NetCounters::default();

    advance_player_input_for_client_ticks(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Instant::now(),
        1,
    );
    queue_client_tick_end_commands(&world, &commands, 2);

    assert!(matches!(rx.try_recv().unwrap(), NetCommand::MovePlayer(_)));
    assert_eq!(rx.try_recv().unwrap(), NetCommand::ClientTickEnd);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::ClientTickEnd);

    world.clear_client_level();
    queue_client_tick_end_commands(&world, &commands, 1);
    assert!(matches!(
        rx.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
}

#[test]
fn the_render_distance_consumers_see_is_clamped_by_the_server_radius() {
    use crate::options::derived::effective_render_distance;

    let mut world = WorldStore::new();
    assert_eq!(world.chunk_cache_radius(), None);
    // Before the server says anything the configured distance stands.
    assert_eq!(
        effective_render_distance(16, world.chunk_cache_radius()),
        16
    );

    world.apply_set_chunk_cache_radius(bbb_protocol::packets::SetChunkCacheRadius { radius: 8 });

    // `handleSetChunkCacheRadius` is what feeds `setServerRenderDistance`, and
    // every consumer downstream of `update_renderer_from_publication` reads the
    // clamped value rather than the configured one.
    assert_eq!(world.chunk_cache_radius(), Some(8));
    assert_eq!(effective_render_distance(16, world.chunk_cache_radius()), 8);
    assert_eq!(effective_render_distance(4, world.chunk_cache_radius()), 4);
}

#[test]
fn offhand_hotbar_slot_follows_the_stack_and_the_opposite_main_arm() {
    let mut world = WorldStore::new();
    assert_eq!(hud_offhand_slot_side(&world), None);

    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: bbb_protocol::equipment_slots::VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
    });
    assert_eq!(hud_offhand_slot_side(&world), Some(true));

    world.apply_login(&PlayLogin {
        player_id: 7,
        ..offhand_test_login()
    });
    world.apply_add_entity(bbb_protocol::packets::AddEntity {
        id: 7,
        uuid: uuid::Uuid::from_u128(7),
        entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: bbb_protocol::packets::Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        delta_movement: bbb_protocol::packets::Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: bbb_protocol::equipment_slots::VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
    });
    assert_eq!(hud_offhand_slot_side(&world), Some(true));

    assert!(
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 7,
            values: vec![test_humanoid_arm_data(
                bbb_client_world::VANILLA_AVATAR_MAIN_HAND_DATA_ID,
                bbb_client_world::VANILLA_HUMANOID_ARM_LEFT_ID,
            )],
        })
    );
    assert_eq!(hud_offhand_slot_side(&world), Some(false));

    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: bbb_protocol::equipment_slots::VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    assert_eq!(hud_offhand_slot_side(&world), None);
}

#[test]
fn the_hotbar_attack_indicator_only_charges_for_the_hotbar_option() {
    let charging = Some(bbb_client_world::LocalPlayerCrosshairAttackIndicatorState {
        strength_scale: 0.25,
        full: false,
    });
    assert_eq!(
        hud_hotbar_attack_indicator(true, charging, false),
        HudHotbarAttackIndicator::Progress {
            strength_scale: 0.25,
            main_arm_left: false,
        }
    );
    assert_eq!(
        hud_hotbar_attack_indicator(true, charging, true),
        HudHotbarAttackIndicator::Progress {
            strength_scale: 0.25,
            main_arm_left: true,
        }
    );
    assert_eq!(
        hud_hotbar_attack_indicator(false, charging, false),
        HudHotbarAttackIndicator::None
    );

    let full = Some(bbb_client_world::LocalPlayerCrosshairAttackIndicatorState {
        strength_scale: 1.0,
        full: true,
    });
    assert_eq!(
        hud_hotbar_attack_indicator(true, full, false),
        HudHotbarAttackIndicator::None
    );
    assert_eq!(
        hud_hotbar_attack_indicator(true, None, false),
        HudHotbarAttackIndicator::None
    );
}

#[test]
fn camera_overlays_split_first_person_equippables_from_the_freeze_outline() {
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id: 7,
        ..offhand_test_login()
    });
    world.apply_add_entity(bbb_protocol::packets::AddEntity {
        id: 7,
        uuid: uuid::Uuid::from_u128(7),
        entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: bbb_protocol::packets::Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        delta_movement: bbb_protocol::packets::Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(hud_camera_overlays(&world, false, true, false).is_empty());

    let mut pumpkin = item_stack(bbb_protocol::item_types::VANILLA_ITEM_CARVED_PUMPKIN_ID, 1);
    pumpkin.component_patch.equippable = Some(bbb_protocol::packets::EquippableSummary {
        slot_id: bbb_protocol::equipment_slots::vanilla_equipment_slot_for_serialized_name("head")
            .unwrap()
            .id,
        equip_sound: bbb_protocol::packets::SoundEventSummary {
            registry_id: None,
            sound_id: Some("minecraft:item.armor.equip_generic".to_string()),
            fixed_range_bits: None,
        },
        asset_id: None,
        camera_overlay: Some("minecraft:misc/pumpkinblur".to_string()),
        allowed_entities: None,
        dispensable: true,
        swappable: true,
        damage_on_hurt: true,
        equip_on_interact: false,
        can_be_sheared: false,
        shearing_sound: bbb_protocol::packets::SoundEventSummary {
            registry_id: None,
            sound_id: Some("minecraft:item.shears.snip".to_string()),
            fixed_range_bits: None,
        },
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: bbb_protocol::equipment_slots::VANILLA_PLAYER_INVENTORY_HEAD_SLOT,
        item: pumpkin,
    });

    let overlays = hud_camera_overlays(&world, false, true, false);
    assert_eq!(overlays.len(), 1);
    assert_eq!(
        overlays[0].texture_id,
        "minecraft:textures/misc/pumpkinblur.png"
    );
    assert_eq!(overlays[0].alpha, 1.0);
    assert!(hud_camera_overlays(&world, false, false, false).is_empty());
    assert!(hud_camera_overlays(&world, false, true, true).is_empty());
    assert!(hud_camera_overlays(&world, true, true, false).is_empty());

    let accessors = bbb_client_world::vanilla_entity_metadata_entity_accessors().unwrap();
    assert!(
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 7,
            values: vec![bbb_protocol::packets::EntityDataValue {
                data_id: accessors.ticks_frozen_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: bbb_protocol::packets::EntityDataValueKind::Int(70),
            }],
        })
    );
    let frozen = hud_camera_overlays(&world, false, false, false);
    assert_eq!(frozen.len(), 1);
    assert_eq!(
        frozen[0].texture_id,
        "minecraft:textures/misc/powder_snow_outline.png"
    );
    assert_eq!(frozen[0].alpha, 0.5);
    assert_eq!(hud_camera_overlays(&world, false, true, false).len(), 2);
    assert!(hud_camera_overlays(&world, true, true, false).is_empty());
}

#[test]
fn the_portal_overlay_is_absent_without_an_active_portal_effect_or_with_hide_gui() {
    let world = WorldStore::new();
    let textures = TerrainTextureState::default();
    assert_eq!(world.local_player_portal_effect_intensity(1.0), 0.0);
    assert!(hud_portal_overlay(&world, &textures, false, 0.5).is_none());
    assert!(hud_portal_overlay(&world, &textures, true, 0.5).is_none());
}

#[test]
fn the_confusion_overlay_needs_nausea_and_a_reduced_screen_effect_scale() {
    let mut world = world_with_dimension(0, "minecraft:overworld");
    attach_lightmap_local_player(&mut world, 42);
    assert!(hud_confusion_overlay(&world, false, 0.0, 1.0).is_none());

    assert!(world.apply_update_mob_effect(mob_effect(42, VANILLA_MOB_EFFECT_NAUSEA_ID, 600,)));
    // `screenEffectScale` at its `1.0` default suppresses the overlay entirely,
    // and `hideGui` drops every camera overlay.
    assert!(hud_confusion_overlay(&world, false, 1.0, 1.0).is_none());
    assert!(hud_confusion_overlay(&world, true, 0.0, 1.0).is_none());

    assert_eq!(
        hud_confusion_overlay(&world, false, 0.0, 1.0)
            .unwrap()
            .strength,
        1.0
    );
    assert_eq!(
        hud_confusion_overlay(&world, false, 0.5, 1.0)
            .unwrap()
            .strength,
        0.5
    );
}
