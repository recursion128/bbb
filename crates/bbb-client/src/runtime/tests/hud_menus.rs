use super::*;

#[test]
fn hud_inventory_screen_filters_recipe_book_buttons_by_crafting_tab_category() {
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry(20, 2, None, 120),
            shapeless_crafting_recipe_book_entry(21, 3, None, 121),
        ],
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

    let recipe_slots = screen
        .background_layers
        .iter()
        .filter(|layer| {
            matches!(
                layer.texture,
                HudInventoryBackgroundTexture::Sprite("recipe_book/slot_uncraftable")
                    | HudInventoryBackgroundTexture::Sprite("recipe_book/slot_many_uncraftable")
            )
        })
        .count();
    assert_eq!(recipe_slots, 1);
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
fn hud_inventory_screen_filters_recipe_book_buttons_by_search_text() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry(20, 2, None, 0),
            shapeless_crafting_recipe_book_entry(21, 2, None, 1),
            shapeless_crafting_recipe_book_entry(22, 2, None, 2),
        ],
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_search: RecipeBookSearchHudState {
                edit: crate::input::text_field::TextField::with_text("stick", 50),
                focused: true,
            },
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    let recipe_slots = screen
        .background_layers
        .iter()
        .filter(|layer| {
            matches!(
                layer.texture,
                HudInventoryBackgroundTexture::Sprite("recipe_book/slot_uncraftable")
                    | HudInventoryBackgroundTexture::Sprite("recipe_book/slot_many_uncraftable")
            )
        })
        .count();
    assert_eq!(recipe_slots, 1);
    assert!(screen
        .floating_items
        .iter()
        .any(|item| (item.x, item.y) == (15, 35)));
}

#[test]
fn recipe_book_plain_search_uses_result_tooltip_text_only() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry(20, 2, None, 0),
            shapeless_crafting_recipe_book_entry(21, 2, None, 1),
            shapeless_crafting_recipe_book_entry(22, 2, None, 2),
        ],
    });

    assert_eq!(
        crafting_recipe_book_search_recipe_indices(&world, &item_runtime, "wooden"),
        vec![22]
    );
    assert!(
        crafting_recipe_book_search_recipe_indices(&world, &item_runtime, "oak_planks").is_empty()
    );
}

#[test]
fn recipe_book_identifier_search_intersects_namespace_with_path_or_tooltip() {
    let item_runtime = recipe_book_ghost_item_runtime();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry(20, 2, None, 0),
            shapeless_crafting_recipe_book_entry(21, 2, None, 1),
            shapeless_crafting_recipe_book_entry(22, 2, None, 2),
        ],
    });

    assert_eq!(
        crafting_recipe_book_search_recipe_indices(&world, &item_runtime, "minecraft:oak_planks"),
        vec![22]
    );
    assert_eq!(
        crafting_recipe_book_search_recipe_indices(&world, &item_runtime, "minecraft:wooden"),
        vec![22]
    );
    assert!(
        crafting_recipe_book_search_recipe_indices(&world, &item_runtime, "bbb:wooden").is_empty()
    );
    assert!(
        crafting_recipe_book_search_recipe_indices(&world, &item_runtime, "minecraft:2").is_empty()
    );
}

#[test]
fn hud_inventory_screen_marks_and_filters_craftable_recipe_book_buttons() {
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_container_set_slot(bbb_protocol::packets::ContainerSetSlot {
        container_id: 7,
        state_id: 13,
        slot: 10,
        item: item_stack(50, 1),
    });
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry_with_requirements(
                20,
                2,
                None,
                120,
                vec![vec![50]],
            ),
            shapeless_crafting_recipe_book_entry_with_requirements(
                21,
                2,
                None,
                121,
                vec![vec![51]],
            ),
        ],
    });

    let all_recipes = hud_inventory_screen_with_local_state(
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

    assert!(all_recipes.background_layers.iter().any(|layer| {
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
    assert!(all_recipes.background_layers.iter().any(|layer| {
        *layer
            == hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("recipe_book/slot_uncraftable"),
                36,
                31,
                25,
                25,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }));

    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: true,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });
    let craftable_recipes = hud_inventory_screen_with_local_state(
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

    let recipe_slots = craftable_recipes
        .background_layers
        .iter()
        .filter(|layer| {
            matches!(
                layer.texture,
                HudInventoryBackgroundTexture::Sprite("recipe_book/slot_craftable")
                    | HudInventoryBackgroundTexture::Sprite("recipe_book/slot_uncraftable")
                    | HudInventoryBackgroundTexture::Sprite("recipe_book/slot_many_craftable")
                    | HudInventoryBackgroundTexture::Sprite("recipe_book/slot_many_uncraftable")
            )
        })
        .count();
    assert_eq!(recipe_slots, 1);
    assert!(craftable_recipes.background_layers.iter().any(|layer| {
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
fn hud_inventory_screen_marks_tagged_recipe_book_requirements_craftable() {
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_container_set_slot(bbb_protocol::packets::ContainerSetSlot {
        container_id: 7,
        state_id: 13,
        slot: 10,
        item: item_stack(50, 1),
    });
    apply_item_tags(&mut world, vec![("minecraft:planks", vec![50])]);
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            shapeless_crafting_recipe_book_entry_with_requirement_summaries(
                20,
                2,
                None,
                120,
                vec![bbb_protocol::packets::IngredientSummary {
                    tag: Some("minecraft:planks".to_string()),
                    item_ids: Vec::new(),
                }],
            ),
        ],
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
fn hud_inventory_screen_projects_recipe_book_page_controls_and_current_page() {
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: (0..21)
            .map(|index| shapeless_crafting_recipe_book_entry(index, 2, None, 200 + index))
            .collect(),
    });

    let first_page = hud_inventory_screen_with_local_state(
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

    assert_eq!(
        first_page
            .background_layers
            .iter()
            .filter(|layer| {
                matches!(
                    layer.texture,
                    HudInventoryBackgroundTexture::Sprite("recipe_book/slot_uncraftable")
                        | HudInventoryBackgroundTexture::Sprite(
                            "recipe_book/slot_many_uncraftable"
                        )
                )
            })
            .count(),
        20
    );
    assert!(first_page.background_layers.iter().any(|layer| {
        layer.texture == HudInventoryBackgroundTexture::Sprite("recipe_book/page_forward")
    }));
    assert!(!first_page.background_layers.iter().any(|layer| {
        layer.texture == HudInventoryBackgroundTexture::Sprite("recipe_book/page_backward")
    }));
    assert!(first_page
        .text_labels
        .iter()
        .any(|label| label.text == "1/2"));

    let second_page = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            recipe_book_tabs: RecipeBookTabSelectionHudState {
                crafting: 1,
                ..RecipeBookTabSelectionHudState::default()
            },
            recipe_book_pages: RecipeBookPageHudState {
                crafting: 1,
                ..RecipeBookPageHudState::default()
            },
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(
        second_page
            .background_layers
            .iter()
            .filter(|layer| {
                matches!(
                    layer.texture,
                    HudInventoryBackgroundTexture::Sprite("recipe_book/slot_uncraftable")
                        | HudInventoryBackgroundTexture::Sprite(
                            "recipe_book/slot_many_uncraftable"
                        )
                )
            })
            .count(),
        1
    );
    assert!(second_page.background_layers.iter().any(|layer| {
        layer.texture == HudInventoryBackgroundTexture::Sprite("recipe_book/page_backward")
    }));
    assert!(!second_page.background_layers.iter().any(|layer| {
        layer.texture == HudInventoryBackgroundTexture::Sprite("recipe_book/page_forward")
    }));
    assert!(second_page
        .text_labels
        .iter()
        .any(|label| label.text == "2/2"));
}

#[test]
fn hud_inventory_screen_highlights_recipe_book_filter_button() {
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
            cursor_position: Some((110, 12)),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(
        screen.background_layers.last().map(|layer| layer.texture),
        Some(HudInventoryBackgroundTexture::Sprite(
            "recipe_book/filter_disabled_highlighted"
        ))
    );
}

#[test]
fn hud_inventory_screen_uses_furnace_recipe_book_filter_sprite() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
        title: bbb_protocol::RichTextComponent::plain("Furnace"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        furnace: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: true,
        },
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(screen.width, 320);
    assert_eq!(
        screen.background_layers.last(),
        Some(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("recipe_book/furnace_filter_enabled"),
            110,
            12,
            26,
            16,
            [0.0, 0.0],
            [1.0, 1.0],
        ))
    );
}

#[test]
fn hud_inventory_screen_projects_enchanting_table_layout_and_lapis_slot_layer() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
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
                HudInventoryBackgroundTexture::Texture(
                    "textures/gui/container/enchanting_table.png"
                ),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/slot/lapis_lazuli"),
                35,
                47,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite(
                    "container/enchanting_table/enchantment_slot_disabled"
                ),
                60,
                14,
                108,
                19,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite(
                    "container/enchanting_table/enchantment_slot_disabled"
                ),
                60,
                33,
                108,
                19,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite(
                    "container/enchanting_table/enchantment_slot_disabled"
                ),
                60,
                52,
                108,
                19,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(37));
    assert_eq!(screen.slots.len(), 38);
    let item = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((item.x, item.y), (15, 47));
    let lapis = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((lapis.x, lapis.y), (35, 47));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 37).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_enchanting_table_enabled_option_layers() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 38];
    items[1] = item_stack(42, 2);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 1,
        value: 12,
    });
    world.apply_player_experience(bbb_protocol::packets::PlayerExperience {
        progress: 0.0,
        level: 12,
        total: 0,
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some((61, 34)),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite(
                "container/enchanting_table/enchantment_slot_highlighted"
            ),
            60,
            33,
            108,
            19,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/enchanting_table/level_2"),
            61,
            34,
            16,
            16,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert_eq!(screen.text_labels.len(), 4);
    let name = screen
        .text_labels
        .iter()
        .find(|label| label.font.as_deref() == Some(ENCHANTMENT_ALT_FONT_ID))
        .expect("seeded enchanting-language label");
    assert_eq!(name.x, 80);
    assert_eq!(name.y, 35);
    assert_eq!(name.tint, ENCHANTING_TABLE_NAME_TEXT_HIGHLIGHTED_COLOR);
    assert_eq!(
        screen.text_labels.last(),
        Some(&HudInventoryTextLabel {
            component_runs: Vec::new(),
            x: 154,
            y: 42,
            width: 12,
            text: "12".to_string(),
            tint: ENCHANTING_TABLE_COST_TEXT_ENABLED_COLOR,
            background: None,
            input: None,
            scrolling: None,
            shadow: false,
            font: None,
            runs: Vec::new(),
        })
    );
}

#[test]
fn hud_inventory_screen_projects_enchanting_table_disabled_cost_label() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 38];
    items[1] = item_stack(42, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 2,
        value: 30,
    });
    world.apply_player_experience(bbb_protocol::packets::PlayerExperience {
        progress: 0.0,
        level: 40,
        total: 0,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite(
                "container/enchanting_table/enchantment_slot_disabled"
            ),
            60,
            52,
            108,
            19,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/enchanting_table/level_3_disabled"),
            61,
            53,
            16,
            16,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert_eq!(screen.text_labels.len(), 4);
    let name = screen
        .text_labels
        .iter()
        .find(|label| label.font.as_deref() == Some(ENCHANTMENT_ALT_FONT_ID))
        .expect("seeded enchanting-language label");
    assert_eq!(name.x, 80);
    assert_eq!(name.y, 54);
    assert_eq!(name.tint, ENCHANTING_TABLE_NAME_TEXT_DISABLED_COLOR);
    assert_eq!(
        screen.text_labels.last(),
        Some(&HudInventoryTextLabel {
            component_runs: Vec::new(),
            x: 154,
            y: 61,
            width: 12,
            text: "30".to_string(),
            tint: ENCHANTING_TABLE_COST_TEXT_DISABLED_COLOR,
            background: None,
            input: None,
            scrolling: None,
            shadow: false,
            font: None,
            runs: Vec::new(),
        })
    );
}

#[test]
fn enchanting_gui_book_preview_matches_vanilla_pose_and_animation_projection() {
    let state = EnchantmentBookHudState {
        open: 0.4,
        flip: -0.2,
    };
    let preview = hud_enchantment_book_preview(state);

    assert_eq!(preview.entity.kind, EntityModelKind::EnchantingBook);
    assert_eq!(
        preview.rect,
        HudEntityPreviewRect {
            x: 14,
            y: 14,
            width: 38,
            height: 31,
        }
    );
    assert_eq!(preview.after_background_layer, Some(0));
    assert_eq!(preview.lighting, GuiItemLightingEntry::EntityInUi);
    assert!(preview.depth_isolated);
    assert_eq!(
        preview.entity.render_state.light_coords,
        ENTITY_FULL_BRIGHT_LIGHT_COORDS
    );
    assert!((preview.entity.block_entity.book_open - 0.4).abs() < 1.0e-6);
    assert!((preview.entity.block_entity.book_page_flip_1 - 0.0).abs() < 1.0e-6);
    assert!((preview.entity.block_entity.book_page_flip_2 - 0.58).abs() < 1.0e-6);

    let actual = glam::Mat4::from_translation(Vec3::from_array(preview.translation))
        * glam::Mat4::from_quat(Quat::from_array(preview.rotation))
        * glam::Mat4::from_translation(Vec3::from_array(preview.entity.position))
        * glam::Mat4::from_translation(Vec3::new(0.5, 0.75, 0.5))
        * glam::Mat4::from_rotation_z(80.0_f32.to_radians());
    let closed = 0.6;
    let expected = glam::Mat4::from_translation(Vec3::new(0.0, 0.0375, 0.0))
        * glam::Mat4::from_rotation_y(std::f32::consts::PI)
        * glam::Mat4::from_rotation_x(25.0_f32.to_radians())
        * glam::Mat4::from_translation(Vec3::new(closed * 0.2, closed * 0.1, closed * 0.25))
        * glam::Mat4::from_rotation_y((-(closed * 90.0) - 90.0).to_radians())
        * glam::Mat4::from_rotation_x(std::f32::consts::PI);
    for (actual, expected) in actual
        .to_cols_array()
        .into_iter()
        .zip(expected.to_cols_array())
    {
        assert!((actual - expected).abs() < 1.0e-5);
    }
}

#[test]
fn enchanting_option_tooltip_uses_synced_clue_and_exact_resource_branches() {
    let root = unique_runtime_temp_dir("enchanting-option-tooltip");
    write_runtime_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    world.record_registry_data(bbb_protocol::packets::RegistryData {
        registry: "minecraft:enchantment".to_string(),
        entries: vec![bbb_protocol::packets::RegistryDataEntry {
            id: "example:efficiency".to_string(),
            raw_data: Some(efficiency_enchantment_attribute_nbt()),
        }],
        raw_payload_len: 0,
    });
    world.begin_registry_configuration();
    world.finish_registry_configuration().unwrap();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 38],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    for (id, value) in [(0, 12), (3, 1234), (4, 0), (7, 3)] {
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id,
            value,
        });
    }
    world.apply_player_experience(bbb_protocol::packets::PlayerExperience {
        progress: 0.0,
        level: 20,
        total: 0,
    });
    let local_state = InventoryHudLocalState {
        cursor_position: Some((61, 15)),
        ..InventoryHudLocalState::default()
    };

    let fallback_screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        local_state.clone(),
        0.0,
    )
    .unwrap();
    let fallback_tooltip = fallback_screen
        .tooltip
        .expect("registry definition supplies the clue without an item runtime");
    assert_eq!(
        fallback_tooltip.lines[0].text,
        "enchantment.example.synced_efficiency III . . . ?"
    );
    assert!(fallback_tooltip.lines[0]
        .runs
        .iter()
        .any(|run| run.visual.color == Some(0x12_34_56)));

    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        local_state.clone(),
        0.0,
    )
    .unwrap();
    let tooltip = screen.tooltip.expect("valid synchronized clue tooltip");
    assert_eq!(tooltip.slot_id, u16::MAX);
    assert_eq!((tooltip.x, tooltip.y), (61, 15));
    assert_eq!(tooltip.lines.len(), 4);
    assert_eq!(tooltip.lines[0].text, "Synced Efficiency III . . . ?");
    assert_eq!(tooltip.lines[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert!(tooltip.lines[0]
        .runs
        .iter()
        .any(|run| run.visual.color == Some(0x12_34_56)));
    assert_eq!(tooltip.lines[1].text, "");
    assert_eq!(tooltip.lines[2].text, "1 Lapis Lazuli");
    assert_eq!(tooltip.lines[2].tint, TOOLTIP_TEST_RED);
    assert_eq!(tooltip.lines[3].text, "1 Enchantment Level");
    assert_eq!(tooltip.lines[3].tint, TOOLTIP_TEST_GRAY);

    world.apply_player_experience(bbb_protocol::packets::PlayerExperience {
        progress: 0.0,
        level: 5,
        total: 0,
    });
    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        local_state.clone(),
        0.0,
    )
    .unwrap();
    let tooltip = screen.tooltip.expect("level requirement tooltip");
    assert_eq!(tooltip.lines.len(), 3);
    assert_eq!(tooltip.lines[2].text, "Level Requirement: 12");
    assert_eq!(tooltip.lines[2].tint, TOOLTIP_TEST_RED);

    world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: true,
        flying: true,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        local_state,
        0.0,
    )
    .unwrap();
    assert_eq!(
        screen.tooltip.expect("infinite-material clue").lines.len(),
        1
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn enchanting_option_tooltip_does_not_fabricate_unknown_custom_clue() {
    let mut world = WorldStore::new();
    world.record_registry_data(bbb_protocol::packets::RegistryData {
        registry: "minecraft:enchantment".to_string(),
        entries: vec![bbb_protocol::packets::RegistryDataEntry {
            id: "example:unknown".to_string(),
            raw_data: None,
        }],
        raw_payload_len: 0,
    });
    let options = std::array::from_fn(|index| EnchantmentOptionHudState {
        index,
        cost: if index == 0 { 1 } else { 0 },
        enabled: false,
        highlighted: false,
        tooltip_hovered: index == 0,
        name: String::new(),
        name_max_width: 86,
        clue_holder_id: (index == 0).then_some(0),
        clue_level: (index == 0).then_some(1),
    });

    assert!(hud_enchantment_table_tooltip(&world, None, Some((61, 15)), Some(&options)).is_none());
}

#[test]
fn hud_inventory_screen_projects_anvil_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
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
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/anvil.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/anvil/text_field_disabled"),
                59,
                20,
                110,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(38));
    assert_eq!(screen.slots.len(), 39);
    let input = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((input.x, input.y), (27, 47));
    let additional = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((additional.x, additional.y), (76, 47));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
    assert_eq!((result.x, result.y), (134, 47));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 38).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_beacon_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 37],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(36), 0.0).unwrap();

    assert_eq!(screen.width, 230);
    assert_eq!(screen.height, 219);
    assert_eq!(screen.background_layers.len(), 17);
    assert_eq!(
        screen.background_layers[0],
        hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Texture("textures/gui/container/beacon.png"),
            0,
            0,
            230,
            219,
            [0.0, 0.0],
            [230.0 / 256.0, 219.0 / 256.0],
        )
    );
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/beacon/button_disabled"),
            53,
            22,
            22,
            22,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::MobEffect(VANILLA_MOB_EFFECT_SPEED_ID),
            55,
            24,
            18,
            18,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::MobEffect(VANILLA_MOB_EFFECT_REGENERATION_ID),
            146,
            49,
            18,
            18,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/beacon/button_disabled"),
            164,
            107,
            22,
            22,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/beacon/confirm"),
            166,
            109,
            18,
            18,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/beacon/button"),
            190,
            107,
            22,
            22,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/beacon/cancel"),
            192,
            109,
            18,
            18,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert_eq!(screen.hovered_slot_id, Some(36));
    assert_eq!(screen.slots.len(), 37);
    let payment = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((payment.x, payment.y), (136, 110));
    let first_inventory = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((first_inventory.x, first_inventory.y), (36, 137));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 36).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (180, 195));
}

#[test]
fn hud_inventory_screen_projects_active_beacon_confirm_button() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 37];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: BEACON_PRIMARY_EFFECT_DATA_ID,
        value: 5,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/beacon/button"),
            164,
            107,
            22,
            22,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(!screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/beacon/button_disabled"),
            164,
            107,
            22,
            22,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
}

#[test]
fn hud_inventory_screen_projects_local_beacon_effect_selection() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 37];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: BEACON_LEVELS_DATA_ID,
        value: 4,
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            beacon_effect_selection: Some((
                Some(VANILLA_MOB_EFFECT_STRENGTH_ID),
                Some(VANILLA_MOB_EFFECT_STRENGTH_ID),
            )),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(screen.background_layers.len(), 19);
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/beacon/button_selected"),
            65,
            72,
            22,
            22,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::MobEffect(VANILLA_MOB_EFFECT_STRENGTH_ID),
            67,
            74,
            18,
            18,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/beacon/button_selected"),
            168,
            47,
            22,
            22,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::MobEffect(VANILLA_MOB_EFFECT_STRENGTH_ID),
            170,
            49,
            18,
            18,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
    assert!(screen
        .background_layers
        .contains(&hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/beacon/button"),
            164,
            107,
            22,
            22,
            [0.0, 0.0],
            [1.0, 1.0],
        )));
}

#[test]
fn hud_inventory_screen_projects_beacon_labels_without_generic_container_copy() {
    let world = beacon_test_world(false, 0);

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(screen.text_labels.len(), 2);
    assert_eq!(
        screen
            .text_labels
            .iter()
            .map(|label| label.text.as_str())
            .collect::<Vec<_>>(),
        ["Primary Power", "Secondary Power"]
    );
    assert_eq!(
        screen
            .text_labels
            .iter()
            .map(|label| (
                label.scrolling.unwrap().x,
                label.y,
                label.tint,
                label.shadow,
            ))
            .collect::<Vec<_>>(),
        [
            (
                BEACON_PRIMARY_LABEL_CENTER_X
                    - i32::try_from(CONTAINER_CENTERED_LABEL_WIDTH).unwrap() / 2,
                BEACON_LABEL_Y,
                BEACON_LABEL_TEXT_COLOR,
                false,
            ),
            (
                BEACON_SECONDARY_LABEL_CENTER_X
                    - i32::try_from(CONTAINER_CENTERED_LABEL_WIDTH).unwrap() / 2,
                BEACON_LABEL_Y,
                BEACON_LABEL_TEXT_COLOR,
                false,
            ),
        ]
    );
    assert!(screen
        .text_labels
        .iter()
        .all(|label| label.text != "Beacon" && label.text != "Inventory"));
}

#[test]
fn hud_inventory_screen_projects_beacon_hover_precedence() {
    let disabled_world = beacon_test_world(false, 0);
    let disabled = beacon_screen_with_local_state(
        &disabled_world,
        None,
        InventoryHudLocalState {
            cursor_position: Some((53, 22)),
            ..InventoryHudLocalState::default()
        },
    );
    assert_eq!(
        beacon_button_texture_at(&disabled, 53, 22),
        Some(HudInventoryBackgroundTexture::Sprite(
            "container/beacon/button_disabled"
        ))
    );
    assert_eq!(disabled.tooltip.as_ref().unwrap().lines[0].text, "Speed");

    let world = beacon_test_world(true, 4);
    let highlighted = beacon_screen_with_local_state(
        &world,
        None,
        InventoryHudLocalState {
            cursor_position: Some((53, 22)),
            ..InventoryHudLocalState::default()
        },
    );
    assert_eq!(
        beacon_button_texture_at(&highlighted, 53, 22),
        Some(HudInventoryBackgroundTexture::Sprite(
            "container/beacon/button_highlighted"
        ))
    );

    let selected = beacon_screen_with_local_state(
        &world,
        None,
        InventoryHudLocalState {
            beacon_effect_selection: Some((Some(VANILLA_MOB_EFFECT_STRENGTH_ID), None)),
            cursor_position: Some((65, 72)),
            ..InventoryHudLocalState::default()
        },
    );
    assert_eq!(
        beacon_button_texture_at(&selected, 65, 72),
        Some(HudInventoryBackgroundTexture::Sprite(
            "container/beacon/button_selected"
        ))
    );

    let disabled_confirm = beacon_screen_with_local_state(
        &disabled_world,
        None,
        InventoryHudLocalState {
            beacon_effect_selection: Some((Some(VANILLA_MOB_EFFECT_SPEED_ID), None)),
            cursor_position: Some((BEACON_CONFIRM_BUTTON_X, BEACON_ACTION_BUTTON_Y)),
            ..InventoryHudLocalState::default()
        },
    );
    assert_eq!(
        beacon_button_texture_at(
            &disabled_confirm,
            BEACON_CONFIRM_BUTTON_X,
            BEACON_ACTION_BUTTON_Y,
        ),
        Some(HudInventoryBackgroundTexture::Sprite(
            "container/beacon/button_disabled"
        ))
    );
    assert_eq!(
        disabled_confirm.tooltip.as_ref().unwrap().lines[0].text,
        "Done"
    );

    let active_confirm = beacon_screen_with_local_state(
        &world,
        None,
        InventoryHudLocalState {
            beacon_effect_selection: Some((Some(VANILLA_MOB_EFFECT_SPEED_ID), None)),
            cursor_position: Some((BEACON_CONFIRM_BUTTON_X, BEACON_ACTION_BUTTON_Y)),
            ..InventoryHudLocalState::default()
        },
    );
    assert_eq!(
        beacon_button_texture_at(
            &active_confirm,
            BEACON_CONFIRM_BUTTON_X,
            BEACON_ACTION_BUTTON_Y,
        ),
        Some(HudInventoryBackgroundTexture::Sprite(
            "container/beacon/button_highlighted"
        ))
    );

    let cancel = beacon_screen_with_local_state(
        &world,
        None,
        InventoryHudLocalState {
            cursor_position: Some((BEACON_CANCEL_BUTTON_X, BEACON_ACTION_BUTTON_Y)),
            ..InventoryHudLocalState::default()
        },
    );
    assert_eq!(
        beacon_button_texture_at(&cancel, BEACON_CANCEL_BUTTON_X, BEACON_ACTION_BUTTON_Y,),
        Some(HudInventoryBackgroundTexture::Sprite(
            "container/beacon/button_highlighted"
        ))
    );
    assert_eq!(cancel.tooltip.as_ref().unwrap().lines[0].text, "Cancel");
}

#[test]
fn hud_inventory_screen_projects_localized_beacon_tooltips_and_upgrade_visibility() {
    let root = unique_runtime_temp_dir("beacon-presentation");
    write_runtime_beacon_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let world = beacon_test_world(true, 4);

    let primary = beacon_screen_with_local_state(
        &world,
        Some(&item_runtime),
        InventoryHudLocalState {
            cursor_position: Some((53, 22)),
            ..InventoryHudLocalState::default()
        },
    );
    assert_eq!(
        primary
            .text_labels
            .iter()
            .map(|label| label.text.as_str())
            .collect::<Vec<_>>(),
        ["Main Power", "Auxiliary Power"]
    );
    assert_eq!(primary.tooltip.as_ref().unwrap().lines[0].text, "Swift");
    assert_eq!(primary.floating_items.len(), 5);
    assert_eq!(
        primary
            .floating_items
            .iter()
            .map(|item| (item.x, item.y))
            .collect::<Vec<_>>(),
        [(20, 109), (41, 109), (63, 109), (86, 109), (108, 109)]
    );
    assert!(primary.floating_items.iter().all(|item| {
        !item.draw_decorations && item.block_model.is_none() && item.icon.layers.len() == 1
    }));

    let upgrade = beacon_screen_with_local_state(
        &world,
        Some(&item_runtime),
        InventoryHudLocalState {
            beacon_effect_selection: Some((Some(VANILLA_MOB_EFFECT_STRENGTH_ID), None)),
            cursor_position: Some((168, 47)),
            ..InventoryHudLocalState::default()
        },
    );
    assert_eq!(
        upgrade.tooltip.as_ref().unwrap(),
        &HudInventoryTooltip {
            slot_id: u16::MAX,
            x: 168,
            y: 47,
            style: None,
            lines: vec![HudInventoryTooltipLine {
                text: "Might II".to_string(),
                tint: [1.0, 1.0, 1.0, 1.0],
                runs: Vec::new(),
            }],
        }
    );
    assert_eq!(upgrade.background_layers.len(), 19);

    for (cursor, text) in [
        ((BEACON_CONFIRM_BUTTON_X, BEACON_ACTION_BUTTON_Y), "Apply"),
        ((BEACON_CANCEL_BUTTON_X, BEACON_ACTION_BUTTON_Y), "Abort"),
    ] {
        let screen = beacon_screen_with_local_state(
            &world,
            Some(&item_runtime),
            InventoryHudLocalState {
                beacon_effect_selection: Some((Some(VANILLA_MOB_EFFECT_SPEED_ID), None)),
                cursor_position: Some(cursor),
                ..InventoryHudLocalState::default()
            },
        );
        assert_eq!(screen.tooltip.as_ref().unwrap().lines[0].text, text);
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_screen_omits_unresolved_beacon_examples_and_invalid_upgrade() {
    let world = beacon_test_world(true, 4);

    let screen = beacon_screen_with_local_state(
        &world,
        None,
        InventoryHudLocalState {
            beacon_effect_selection: Some((Some(i32::MAX), None)),
            cursor_position: Some((168, 47)),
            ..InventoryHudLocalState::default()
        },
    );

    assert!(screen.floating_items.is_empty());
    assert_eq!(screen.background_layers.len(), 17);
    assert_eq!(screen.tooltip, None);
    assert_eq!(
        beacon_button_texture_at(&screen, BEACON_CONFIRM_BUTTON_X, BEACON_ACTION_BUTTON_Y,),
        Some(HudInventoryBackgroundTexture::Sprite(
            "container/beacon/button_disabled"
        ))
    );
}

#[test]
fn hud_inventory_screen_projects_anvil_text_field_and_error_layers() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/anvil.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/anvil/text_field"),
                59,
                20,
                110,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/anvil/error"),
                99,
                45,
                28,
                21,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_anvil_rename_text_label() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
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
            anvil_rename_text: Some("Sharp Pick".to_string()),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(screen.text_labels.len(), 3);
    assert_eq!(
        screen.text_labels.last(),
        Some(&HudInventoryTextLabel {
            component_runs: Vec::new(),
            x: 62,
            y: 24,
            width: 103,
            text: "Sharp Pick".to_string(),
            tint: ANVIL_RENAME_TEXT_COLOR,
            background: None,
            input: None,
            scrolling: None,
            shadow: false,
            font: None,
            runs: Vec::new(),
        })
    );
}

#[test]
fn hud_inventory_screen_projects_anvil_cost_label() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    items[2] = item_stack(42, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: ANVIL_COST_DATA_ID,
        value: 7,
    });
    world.apply_player_experience(bbb_protocol::packets::PlayerExperience {
        progress: 0.0,
        level: 8,
        total: 0,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(screen.text_labels.len(), 3);
    assert_eq!(
        screen.text_labels.last(),
        Some(&HudInventoryTextLabel {
            component_runs: Vec::new(),
            x: 126,
            y: ANVIL_COST_LABEL_Y,
            width: 40,
            text: "Cost: 7".to_string(),
            tint: ANVIL_COST_TEXT_COLOR,
            background: Some(HudInventoryTextBackground {
                x: 124,
                y: ANVIL_COST_BACKGROUND_Y,
                width: 44,
                height: ANVIL_COST_BACKGROUND_HEIGHT,
                tint: ANVIL_COST_BACKGROUND_TINT,
            }),
            input: None,
            scrolling: None,
            shadow: false,
            font: None,
            runs: Vec::new(),
        })
    );
}

#[test]
fn hud_inventory_screen_projects_anvil_too_expensive_label() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: ANVIL_COST_DATA_ID,
        value: ANVIL_TOO_EXPENSIVE_LEVEL_COST,
    });
    world.apply_player_experience(bbb_protocol::packets::PlayerExperience {
        progress: 0.0,
        level: 100,
        total: 0,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(screen.text_labels.len(), 3);
    assert_eq!(
        screen.text_labels.last(),
        Some(&HudInventoryTextLabel {
            component_runs: Vec::new(),
            x: 84,
            y: ANVIL_COST_LABEL_Y,
            width: 82,
            text: "Too Expensive!".to_string(),
            tint: ANVIL_COST_ERROR_TEXT_COLOR,
            background: Some(HudInventoryTextBackground {
                x: 82,
                y: ANVIL_COST_BACKGROUND_Y,
                width: 86,
                height: ANVIL_COST_BACKGROUND_HEIGHT,
                tint: ANVIL_COST_BACKGROUND_TINT,
            }),
            input: None,
            scrolling: None,
            shadow: false,
            font: None,
            runs: Vec::new(),
        })
    );
}

#[test]
fn hud_inventory_screen_projects_brewing_stand_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BREWING_STAND_ID,
        title: bbb_protocol::RichTextComponent::plain("Brewing Stand"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 41],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(40), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Texture("textures/gui/container/brewing_stand.png"),
            0,
            0,
            176,
            166,
            [0.0, 0.0],
            [176.0 / 256.0, 166.0 / 256.0],
        )]
    );
    assert_eq!(screen.hovered_slot_id, Some(40));
    assert_eq!(screen.slots.len(), 41);
    let bottle = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((bottle.x, bottle.y), (56, 51));
    let ingredient = screen.slots.iter().find(|slot| slot.slot_id == 3).unwrap();
    assert_eq!((ingredient.x, ingredient.y), (79, 17));
    let fuel = screen.slots.iter().find(|slot| slot.slot_id == 4).unwrap();
    assert_eq!((fuel.x, fuel.y), (17, 17));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 40).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_brewing_stand_progress_layers() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BREWING_STAND_ID,
        title: bbb_protocol::RichTextComponent::plain("Brewing Stand"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 41],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 0,
        value: 200,
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 1,
        value: 10,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/brewing_stand.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/brewing_stand/fuel_length"),
                60,
                44,
                9,
                4,
                [0.0, 0.0],
                [9.0 / 18.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/brewing_stand/brew_progress"),
                97,
                16,
                9,
                14,
                [0.0, 0.0],
                [1.0, 14.0 / 28.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/brewing_stand/bubbles"),
                63,
                23,
                12,
                20,
                [0.0, 9.0 / 29.0],
                [1.0, 1.0],
            ),
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_furnace_like_layouts() {
    for (menu_type_id, title, texture) in [
        (
            10,
            "Blast Furnace",
            HudInventoryBackgroundTexture::Texture("textures/gui/container/blast_furnace.png"),
        ),
        (
            14,
            "Furnace",
            HudInventoryBackgroundTexture::Texture("textures/gui/container/furnace.png"),
        ),
        (
            22,
            "Smoker",
            HudInventoryBackgroundTexture::Texture("textures/gui/container/smoker.png"),
        ),
    ] {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id,
            title: bbb_protocol::RichTextComponent::plain(title),
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
            vec![
                hud_inventory_background_layer(
                    texture,
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Sprite("recipe_book/button"),
                    20,
                    34,
                    20,
                    18,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ]
        );
        assert_eq!(screen.hovered_slot_id, Some(38));
        assert_eq!(screen.slots.len(), 39);
        let ingredient = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((ingredient.x, ingredient.y), (56, 17));
        let fuel = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
        assert_eq!((fuel.x, fuel.y), (56, 53));
        let result = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
        assert_eq!((result.x, result.y), (116, 35));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 38).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (152, 142));
    }
}

#[test]
fn hud_inventory_screen_projects_furnace_progress_layers() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
        title: bbb_protocol::RichTextComponent::plain("Furnace"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 0,
        value: 50,
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 1,
        value: 200,
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 2,
        value: 25,
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 3,
        value: 100,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/furnace.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/furnace/lit_progress"),
                56,
                45,
                14,
                5,
                [0.0, 9.0 / 14.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/furnace/burn_progress"),
                79,
                34,
                6,
                16,
                [0.0, 0.0],
                [6.0 / 24.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("recipe_book/button"),
                20,
                34,
                20,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_grindstone_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
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
            HudInventoryBackgroundTexture::Texture("textures/gui/container/grindstone.png"),
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
    let input = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((input.x, input.y), (49, 19));
    let additional = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((additional.x, additional.y), (49, 40));
    let result = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
    assert_eq!((result.x, result.y), (129, 34));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 38).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_grindstone_error_layer() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/grindstone.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/grindstone/error"),
                92,
                31,
                28,
                21,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
}

#[test]
fn hud_inventory_screen_projects_hopper_layout() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_HOPPER_ID,
        title: bbb_protocol::RichTextComponent::plain("Hopper"),
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 41],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });

    let screen = hud_inventory_screen(&world, None, Some(40), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 133);
    assert_eq!(
        screen.background_layers,
        vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Texture("textures/gui/container/hopper.png"),
            0,
            0,
            176,
            133,
            [0.0, 0.0],
            [176.0 / 256.0, 133.0 / 256.0],
        )]
    );
    assert_eq!(screen.hovered_slot_id, Some(40));
    assert_eq!(screen.slots.len(), 41);
    let first_container = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((first_container.x, first_container.y), (44, 20));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 40).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 109));
}

#[test]
fn hud_inventory_screen_projects_mount_horse_layout() {
    let mut world = WorldStore::new();
    world.apply_add_entity(test_add_entity(42, 66));
    world.apply_mount_screen_open(bbb_protocol::packets::MountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });

    let screen = hud_inventory_screen(&world, None, Some(16), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/horse.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/slot"),
                7,
                17,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/slot/saddle"),
                8,
                18,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/slot"),
                7,
                35,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/slot/horse_armor"),
                8,
                36,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/horse/chest_slots"),
                79,
                17,
                90,
                54,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(16));
    assert_eq!(screen.slots.len(), 53);
    let saddle = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
    assert_eq!((saddle.x, saddle.y), (8, 18));
    let last_mount_slot = screen.slots.iter().find(|slot| slot.slot_id == 16).unwrap();
    assert_eq!((last_mount_slot.x, last_mount_slot.y), (152, 54));
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 52).unwrap();
    assert_eq!((hotbar.x, hotbar.y), (152, 142));
}

#[test]
fn hud_inventory_screen_projects_mount_entity_preview() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    world.apply_add_entity(test_add_entity(77, 66));
    world.apply_mount_screen_open(bbb_protocol::packets::MountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 77,
    });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some((5, 90)),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(screen.entity_previews.len(), 1);
    let preview = &screen.entity_previews[0];
    assert_eq!(preview.entity.entity_id, 77);
    assert_eq!(preview.entity.position, [0.0; 3]);
    assert_eq!(preview.lighting, GuiItemLightingEntry::EntityInUi);
    assert_eq!(
        preview.rect,
        HudEntityPreviewRect {
            x: 26,
            y: 18,
            width: 52,
            height: 52,
        }
    );
    assert_eq!(preview.scissor, None);
    assert_eq!(preview.scale, 17.0);
    assert!(preview.depth_isolated);
    let bounds = world.probe_entity_pick_bounds(77).unwrap();
    assert_close3(
        preview.translation,
        [0.0, (bounds.max[1] - bounds.min[1]) / 2.0 + 0.25, 0.0],
    );

    let x_angle = ((52.0_f32 - 5.0) / 40.0).atan();
    let y_angle = ((44.0_f32 - 90.0) / 40.0).atan();
    let expected_yaw = x_angle * 20.0;
    let expected_pitch = y_angle * 20.0;
    let expected_camera = quaternion_x(expected_pitch.to_radians());
    assert!((preview.entity.render_state.body_rot - (180.0 + expected_yaw)).abs() < 1.0e-6);
    assert!((preview.entity.render_state.head_yaw - expected_yaw).abs() < 1.0e-6);
    assert!((preview.entity.render_state.head_pitch + expected_pitch).abs() < 1.0e-6);
    assert_close4(
        preview.rotation,
        quaternion_mul([0.0, 0.0, 1.0, 0.0], expected_camera),
    );
    assert_eq!(preview.override_camera_rotation, Some(expected_camera));
    assert_eq!(
        preview.entity.render_state.light_coords,
        ENTITY_FULL_BRIGHT_LIGHT_COORDS
    );
    assert_eq!(preview.entity.render_state.outline_color, 0);
    assert!(!preview.entity.render_state.appears_glowing);
}

#[test]
fn hud_inventory_screen_projects_mount_nautilus_slot_placeholders() {
    let mut world = WorldStore::new();
    world.apply_add_entity(test_add_entity(42, 88));
    world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
        id: 42,
        values: vec![test_byte_data(18, 4)],
    });
    world.apply_mount_screen_open(bbb_protocol::packets::MountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });

    let screen = hud_inventory_screen(&world, None, Some(1), 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/nautilus.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/slot"),
                7,
                17,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/slot/saddle"),
                8,
                18,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/slot"),
                7,
                35,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/slot/nautilus_armor_inventory"),
                8,
                36,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.slots.len(), 38);
    let armor = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
    assert_eq!((armor.x, armor.y), (8, 36));
}

#[test]
fn hud_inventory_screen_hides_inactive_mount_equipment_slot_layers() {
    let mut world = WorldStore::new();
    world.apply_add_entity(test_add_entity(42, 36));
    world.apply_mount_screen_open(bbb_protocol::packets::MountScreenOpen {
        container_id: 7,
        inventory_columns: 3,
        entity_id: 42,
    });

    let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/horse.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/horse/chest_slots"),
                79,
                17,
                54,
                54,
                [0.0, 0.0],
                [0.6, 1.0],
            ),
        ]
    );
    assert!(screen.slots.iter().all(|slot| slot.slot_id != 0));
    assert!(screen.slots.iter().all(|slot| slot.slot_id != 1));
    assert_eq!(screen.slots[0].slot_id, 2);
}

#[test]
fn hud_inventory_screen_uses_mount_llama_armor_slot_placeholder() {
    let mut world = WorldStore::new();
    world.apply_add_entity(test_add_entity(42, 78));
    world.apply_mount_screen_open(bbb_protocol::packets::MountScreenOpen {
        container_id: 7,
        inventory_columns: 3,
        entity_id: 42,
    });

    let screen = hud_inventory_screen(&world, None, Some(1), 0.0).unwrap();

    assert!(screen.background_layers.iter().any(|layer| layer.texture
        == HudInventoryBackgroundTexture::Sprite("container/slot/llama_armor")
        && (layer.x, layer.y, layer.width, layer.height) == (8, 36, 16, 16)));
}

#[test]
fn dedicated_hud_book_screen_replaces_empty_lectern_inventory_projection() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LECTERN_ID,
        title: bbb_protocol::RichTextComponent::plain("Lectern"),
    });

    assert!(hud_inventory_screen(&world, None, None, 0.0).is_none());
    let mut input = ClientInputState::default();
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    let screen = hud_book_screen(&input, &world, None).unwrap();
    assert_eq!(screen.mode, bbb_client_render::HudBookMode::Lectern);
    assert_eq!(screen.page, 0);
    assert_eq!(screen.page_count, 1);
    assert_eq!(screen.page_indicator, "Page 1 of 1");
    assert!(screen.runs.is_empty());
    assert!(screen.may_build);
}

#[test]
fn dedicated_hud_book_screen_projects_lectern_current_page_text() {
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LECTERN_ID,
        title: bbb_protocol::RichTextComponent::plain("Lectern"),
    });
    let mut book = item_stack(42, 1);
    book.component_patch.written_book = Some(bbb_protocol::packets::WrittenBookContentSummary {
        title: "Guide".into(),
        title_filter: None,
        author: "Alex".into(),
        generation: 0,
        pages: vec![
            "First page".to_string(),
            "Second page\nLine two".to_string(),
        ],
        page_filters: vec![None, None],
        resolved: true,
    });
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![book],
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
        container_id: 7,
        id: 0,
        value: 1,
    });

    let mut input = ClientInputState::default();
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    let screen = hud_book_screen(&input, &world, None).unwrap();

    assert_eq!(screen.mode, bbb_client_render::HudBookMode::Lectern);
    assert_eq!(screen.page, 1);
    assert_eq!(screen.page_count, 2);
    assert_eq!(screen.page_indicator, "Page 2 of 2");
    assert_eq!(screen.runs.len(), 1);
    assert_eq!(screen.runs[0].visual.text, "Second page\nLine two");
}

#[test]
fn dedicated_hud_book_screen_replaces_inventory_projection() {
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: written_book_stack(vec!["First page", "Second page"]),
    });
    world.apply_open_book(OpenBook {
        hand: InteractionHand::MainHand,
    });
    let mut input = ClientInputState::default();
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);

    assert!(hud_inventory_screen(&world, None, None, 0.0).is_none());
    let screen = hud_book_screen(&input, &world, None).unwrap();
    assert_eq!(screen.mode, bbb_client_render::HudBookMode::View);
    assert_eq!(screen.page, 0);
    assert_eq!(screen.page_count, 2);
    assert_eq!(screen.page_indicator, "Page 1 of 2");
    assert_eq!(
        screen.runs,
        vec![HudComponentTextRun {
            visual: HudStyledTextRun {
                shadow_color: Some(0),
                text: "First page".to_string(),
                style: HudTextStyle::default(),
                color: Some(0),
            },
            font: None,
        }]
    );
}
