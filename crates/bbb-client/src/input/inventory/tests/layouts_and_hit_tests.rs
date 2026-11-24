use super::*;

#[test]
fn local_inventory_slot_layouts_match_vanilla_inventory_menu() {
    let slots = local_inventory_slot_layouts();
    assert_eq!(slots.len(), 46);
    assert_eq!(
        slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 154,
            y: 28,
        }
    );
    assert_eq!(
        slots[1],
        InventorySlotLayout {
            slot_id: 1,
            x: 98,
            y: 18,
        }
    );
    assert_eq!(
        slots[5],
        InventorySlotLayout {
            slot_id: 5,
            x: 8,
            y: 8,
        }
    );
    assert_eq!(
        slots[9],
        InventorySlotLayout {
            slot_id: 9,
            x: 8,
            y: 84,
        }
    );
    assert_eq!(
        slots[36],
        InventorySlotLayout {
            slot_id: 36,
            x: 8,
            y: 142,
        }
    );
    assert_eq!(
        slots[45],
        InventorySlotLayout {
            slot_id: 45,
            x: 77,
            y: 62,
        }
    );
}
#[test]
fn local_inventory_hit_test_uses_centered_vanilla_screen_and_hover_margin() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 419.0)), size),
        Some(InventoryClickTarget::Slot(36))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(559.0, 418.0)), size),
        Some(InventoryClickTarget::Slot(36))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(600.0, 300.0)), size),
        Some(InventoryClickTarget::EmptyPanel)
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(551.0, 277.0)), size),
        Some(InventoryClickTarget::Outside)
    );
}

#[test]
fn generic_container_layout_matches_vanilla_chest_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X6_ID,
        title: bbb_protocol::RichTextComponent::plain("Large Chest"),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 176);
    assert_eq!(layout.height, 222);
    assert_eq!(
        layout.background,
        InventoryScreenBackground::Generic9xRows { rows: 6 }
    );
    assert_eq!(layout.slots.len(), 90);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 8,
            y: 18,
        }
    );
    assert_eq!(
        layout.slots[53],
        InventorySlotLayout {
            slot_id: 53,
            x: 152,
            y: 108,
        }
    );
    assert_eq!(
        layout.slots[54],
        InventorySlotLayout {
            slot_id: 54,
            x: 8,
            y: 139,
        }
    );
    assert_eq!(
        layout.slots[89],
        InventorySlotLayout {
            slot_id: 89,
            x: 152,
            y: 197,
        }
    );
}

#[test]
fn every_generated_server_opened_menu_type_has_a_native_layout() {
    assert_eq!(VANILLA_MENU_TYPES.len(), 25);
    for menu_type in VANILLA_MENU_TYPES {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: menu_type.id,
            title: bbb_protocol::RichTextComponent::plain(menu_type.resource_id),
        });

        assert!(
            inventory_screen_layout(&world).is_some(),
            "generated menu {} ({}) has no native layout",
            menu_type.constant_name,
            menu_type.resource_id
        );
    }
}

#[test]
fn generic_3x3_layout_matches_vanilla_dispenser_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Dispenser"),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 176);
    assert_eq!(layout.height, 166);
    assert_eq!(layout.background, InventoryScreenBackground::Generic3x3);
    assert_eq!(layout.slots.len(), 45);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 62,
            y: 17,
        }
    );
    assert_eq!(
        layout.slots[8],
        InventorySlotLayout {
            slot_id: 8,
            x: 98,
            y: 53,
        }
    );
    assert_eq!(
        layout.slots[9],
        InventorySlotLayout {
            slot_id: 9,
            x: 8,
            y: 84,
        }
    );
    assert_eq!(
        layout.slots[44],
        InventorySlotLayout {
            slot_id: 44,
            x: 152,
            y: 142,
        }
    );
}

#[test]
fn crafter_layout_matches_vanilla_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 176);
    assert_eq!(layout.height, 166);
    assert_eq!(layout.background, InventoryScreenBackground::Crafter);
    assert_eq!(layout.slots.len(), 46);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 26,
            y: 17,
        }
    );
    assert_eq!(
        layout.slots[8],
        InventorySlotLayout {
            slot_id: 8,
            x: 62,
            y: 53,
        }
    );
    assert_eq!(
        layout.slots[9],
        InventorySlotLayout {
            slot_id: 9,
            x: 8,
            y: 84,
        }
    );
    assert_eq!(
        layout.slots[44],
        InventorySlotLayout {
            slot_id: 44,
            x: 152,
            y: 142,
        }
    );
    assert_eq!(
        layout.slots[45],
        InventorySlotLayout {
            slot_id: 45,
            x: 134,
            y: 35,
        }
    );
}

#[test]
fn crafting_table_layout_matches_vanilla_crafting_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 176);
    assert_eq!(layout.height, 166);
    assert_eq!(layout.background, InventoryScreenBackground::CraftingTable);
    assert_eq!(layout.slots.len(), 46);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 124,
            y: 35,
        }
    );
    assert_eq!(
        layout.slots[1],
        InventorySlotLayout {
            slot_id: 1,
            x: 30,
            y: 17,
        }
    );
    assert_eq!(
        layout.slots[9],
        InventorySlotLayout {
            slot_id: 9,
            x: 66,
            y: 53,
        }
    );
    assert_eq!(
        layout.slots[10],
        InventorySlotLayout {
            slot_id: 10,
            x: 8,
            y: 84,
        }
    );
    assert_eq!(
        layout.slots[45],
        InventorySlotLayout {
            slot_id: 45,
            x: 152,
            y: 142,
        }
    );
}

#[test]
fn crafting_table_layout_offsets_slots_when_recipe_book_is_open() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    world.apply_recipe_book_settings(RecipeBookSettings {
        crafting: RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: RecipeBookTypeSettings::default(),
        blast_furnace: RecipeBookTypeSettings::default(),
        smoker: RecipeBookTypeSettings::default(),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 320);
    assert_eq!(layout.height, 166);
    assert_eq!(recipe_book_main_gui_offset(&world, layout.background), 149);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 273,
            y: 35,
        }
    );
    assert_eq!(
        layout.slots[45],
        InventorySlotLayout {
            slot_id: 45,
            x: 301,
            y: 142,
        }
    );
}

#[test]
fn crafting_table_layout_overlaps_recipe_book_when_screen_is_narrow() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });
    world.apply_recipe_book_settings(RecipeBookSettings {
        crafting: RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: RecipeBookTypeSettings::default(),
        blast_furnace: RecipeBookTypeSettings::default(),
        smoker: RecipeBookTypeSettings::default(),
    });

    let layout = inventory_screen_layout_for_surface(&world, PhysicalSize::new(378, 720)).unwrap();

    assert_eq!(layout.width, 176);
    assert_eq!(layout.height, 166);
    let recipe_book = layout.recipe_book.unwrap();
    assert_eq!(recipe_book.x, 14);
    assert_eq!(recipe_book.main_gui_x_offset, 0);
    assert!(recipe_book.narrow);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 124,
            y: 35,
        }
    );
    assert_eq!(
        layout.slots[45],
        InventorySlotLayout {
            slot_id: 45,
            x: 152,
            y: 142,
        }
    );
}

#[test]
fn enchantment_table_layout_matches_vanilla_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 176);
    assert_eq!(layout.height, 166);
    assert_eq!(
        layout.background,
        InventoryScreenBackground::EnchantmentTable
    );
    assert_eq!(layout.slots.len(), 38);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 15,
            y: 47,
        }
    );
    assert_eq!(
        layout.slots[1],
        InventorySlotLayout {
            slot_id: 1,
            x: 35,
            y: 47,
        }
    );
    assert_eq!(
        layout.slots[2],
        InventorySlotLayout {
            slot_id: 2,
            x: 8,
            y: 84,
        }
    );
    assert_eq!(
        layout.slots[37],
        InventorySlotLayout {
            slot_id: 37,
            x: 152,
            y: 142,
        }
    );
}

#[test]
fn anvil_layout_matches_vanilla_item_combiner_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 176);
    assert_eq!(layout.height, 166);
    assert_eq!(layout.background, InventoryScreenBackground::Anvil);
    assert_eq!(layout.slots.len(), 39);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 27,
            y: 47,
        }
    );
    assert_eq!(
        layout.slots[1],
        InventorySlotLayout {
            slot_id: 1,
            x: 76,
            y: 47,
        }
    );
    assert_eq!(
        layout.slots[2],
        InventorySlotLayout {
            slot_id: 2,
            x: 134,
            y: 47,
        }
    );
    assert_eq!(
        layout.slots[3],
        InventorySlotLayout {
            slot_id: 3,
            x: 8,
            y: 84,
        }
    );
    assert_eq!(
        layout.slots[38],
        InventorySlotLayout {
            slot_id: 38,
            x: 152,
            y: 142,
        }
    );
}

#[test]
fn beacon_layout_matches_vanilla_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 230);
    assert_eq!(layout.height, 219);
    assert_eq!(layout.background, InventoryScreenBackground::Beacon);
    assert_eq!(layout.slots.len(), 37);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 136,
            y: 110,
        }
    );
    assert_eq!(
        layout.slots[1],
        InventorySlotLayout {
            slot_id: 1,
            x: 36,
            y: 137,
        }
    );
    assert_eq!(
        layout.slots[36],
        InventorySlotLayout {
            slot_id: 36,
            x: 180,
            y: 195,
        }
    );
}

#[test]
fn brewing_stand_layout_matches_vanilla_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BREWING_STAND_ID,
        title: bbb_protocol::RichTextComponent::plain("Brewing Stand"),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 176);
    assert_eq!(layout.height, 166);
    assert_eq!(layout.background, InventoryScreenBackground::BrewingStand);
    assert_eq!(layout.slots.len(), 41);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 56,
            y: 51,
        }
    );
    assert_eq!(
        layout.slots[1],
        InventorySlotLayout {
            slot_id: 1,
            x: 79,
            y: 58,
        }
    );
    assert_eq!(
        layout.slots[2],
        InventorySlotLayout {
            slot_id: 2,
            x: 102,
            y: 51,
        }
    );
    assert_eq!(
        layout.slots[3],
        InventorySlotLayout {
            slot_id: 3,
            x: 79,
            y: 17,
        }
    );
    assert_eq!(
        layout.slots[4],
        InventorySlotLayout {
            slot_id: 4,
            x: 17,
            y: 17,
        }
    );
    assert_eq!(
        layout.slots[5],
        InventorySlotLayout {
            slot_id: 5,
            x: 8,
            y: 84,
        }
    );
    assert_eq!(
        layout.slots[40],
        InventorySlotLayout {
            slot_id: 40,
            x: 152,
            y: 142,
        }
    );
}

#[test]
fn furnace_like_layouts_match_vanilla_abstract_furnace_menu() {
    for (menu_type_id, title, background) in [
        (
            VANILLA_MENU_TYPE_BLAST_FURNACE_ID,
            "Blast Furnace",
            InventoryScreenBackground::BlastFurnace,
        ),
        (
            VANILLA_MENU_TYPE_FURNACE_ID,
            "Furnace",
            InventoryScreenBackground::Furnace,
        ),
        (
            VANILLA_MENU_TYPE_SMOKER_ID,
            "Smoker",
            InventoryScreenBackground::Smoker,
        ),
    ] {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id,
            title: bbb_protocol::RichTextComponent::plain(title),
        });

        let layout = inventory_screen_layout(&world).unwrap();

        assert_eq!(layout.width, 176);
        assert_eq!(layout.height, 166);
        assert_eq!(layout.background, background);
        assert_eq!(layout.slots.len(), 39);
        assert_eq!(
            layout.slots[0],
            InventorySlotLayout {
                slot_id: 0,
                x: 56,
                y: 17,
            }
        );
        assert_eq!(
            layout.slots[1],
            InventorySlotLayout {
                slot_id: 1,
                x: 56,
                y: 53,
            }
        );
        assert_eq!(
            layout.slots[2],
            InventorySlotLayout {
                slot_id: 2,
                x: 116,
                y: 35,
            }
        );
        assert_eq!(
            layout.slots[3],
            InventorySlotLayout {
                slot_id: 3,
                x: 8,
                y: 84,
            }
        );
        assert_eq!(
            layout.slots[38],
            InventorySlotLayout {
                slot_id: 38,
                x: 152,
                y: 142,
            }
        );
    }
}

#[test]
fn grindstone_layout_matches_vanilla_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 176);
    assert_eq!(layout.height, 166);
    assert_eq!(layout.background, InventoryScreenBackground::Grindstone);
    assert_eq!(layout.slots.len(), 39);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 49,
            y: 19,
        }
    );
    assert_eq!(
        layout.slots[1],
        InventorySlotLayout {
            slot_id: 1,
            x: 49,
            y: 40,
        }
    );
    assert_eq!(
        layout.slots[2],
        InventorySlotLayout {
            slot_id: 2,
            x: 129,
            y: 34,
        }
    );
    assert_eq!(
        layout.slots[3],
        InventorySlotLayout {
            slot_id: 3,
            x: 8,
            y: 84,
        }
    );
    assert_eq!(
        layout.slots[38],
        InventorySlotLayout {
            slot_id: 38,
            x: 152,
            y: 142,
        }
    );
}

#[test]
fn hopper_layout_matches_vanilla_hopper_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_HOPPER_ID,
        title: bbb_protocol::RichTextComponent::plain("Hopper"),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 176);
    assert_eq!(layout.height, 133);
    assert_eq!(layout.background, InventoryScreenBackground::Hopper);
    assert_eq!(layout.slots.len(), 41);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 44,
            y: 20,
        }
    );
    assert_eq!(
        layout.slots[4],
        InventorySlotLayout {
            slot_id: 4,
            x: 116,
            y: 20,
        }
    );
    assert_eq!(
        layout.slots[5],
        InventorySlotLayout {
            slot_id: 5,
            x: 8,
            y: 51,
        }
    );
    assert_eq!(
        layout.slots[40],
        InventorySlotLayout {
            slot_id: 40,
            x: 152,
            y: 109,
        }
    );
}

#[test]
fn mount_horse_layout_matches_vanilla_horse_inventory_menu() {
    let mut world = WorldStore::new();
    world.apply_add_entity(add_entity_with_type(42, 66));
    world.apply_mount_screen_open(MountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 176);
    assert_eq!(layout.height, 166);
    assert_eq!(
        layout.background,
        InventoryScreenBackground::Mount {
            kind: MountInventoryKind::Horse,
            inventory_columns: 5,
        }
    );
    assert_eq!(layout.slots.len(), 53);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 8,
            y: 18,
        }
    );
    assert_eq!(
        layout.slots[1],
        InventorySlotLayout {
            slot_id: 1,
            x: 8,
            y: 36,
        }
    );
    assert_eq!(
        layout.slots[2],
        InventorySlotLayout {
            slot_id: 2,
            x: 80,
            y: 18,
        }
    );
    assert_eq!(
        layout.slots[16],
        InventorySlotLayout {
            slot_id: 16,
            x: 152,
            y: 54,
        }
    );
    assert_eq!(
        layout.slots[17],
        InventorySlotLayout {
            slot_id: 17,
            x: 8,
            y: 84,
        }
    );
    assert_eq!(
        layout.slots[52],
        InventorySlotLayout {
            slot_id: 52,
            x: 152,
            y: 142,
        }
    );
}

#[test]
fn mount_nautilus_layout_uses_equipment_and_player_slots_only() {
    let mut world = WorldStore::new();
    let accessors =
        bbb_client_world::vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        )
        .expect("generated TamableAnimal metadata accessors must resolve");
    world.apply_add_entity(add_entity_with_type(42, VANILLA_ENTITY_TYPE_NAUTILUS_ID));
    world.apply_set_entity_data(SetEntityData {
        id: 42,
        values: vec![byte_entity_data(
            accessors.flags_data_id,
            accessors.tame_mask,
        )],
    });
    world.apply_mount_screen_open(MountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(
        layout.background,
        InventoryScreenBackground::Mount {
            kind: MountInventoryKind::Nautilus,
            inventory_columns: 0,
        }
    );
    assert_eq!(layout.slots.len(), 38);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 8,
            y: 18,
        }
    );
    assert_eq!(
        layout.slots[1],
        InventorySlotLayout {
            slot_id: 1,
            x: 8,
            y: 36,
        }
    );
    assert_eq!(
        layout.slots[2],
        InventorySlotLayout {
            slot_id: 2,
            x: 8,
            y: 84,
        }
    );
    assert_eq!(
        layout.slots[37],
        InventorySlotLayout {
            slot_id: 37,
            x: 152,
            y: 142,
        }
    );
}

#[test]
fn mount_donkey_layout_hides_inactive_equipment_slots() {
    let mut world = WorldStore::new();
    world.apply_add_entity(add_entity_with_type(42, VANILLA_ENTITY_TYPE_DONKEY_ID));
    world.apply_mount_screen_open(MountScreenOpen {
        container_id: 7,
        inventory_columns: 3,
        entity_id: 42,
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert!(!layout.slots.iter().any(|slot| slot.slot_id == 0));
    assert!(!layout.slots.iter().any(|slot| slot.slot_id == 1));
    assert_eq!(layout.slots.len(), 45);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 2,
            x: 80,
            y: 18,
        }
    );
    assert_eq!(
        layout.slots[8],
        InventorySlotLayout {
            slot_id: 10,
            x: 116,
            y: 54,
        }
    );
    assert_eq!(
        layout.slots[9],
        InventorySlotLayout {
            slot_id: 11,
            x: 8,
            y: 84,
        }
    );
}

#[test]
fn mount_tamed_donkey_layout_shows_saddle_but_no_body_slot() {
    let mut world = WorldStore::new();
    let accessors =
        bbb_client_world::vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_DONKEY_ID,
        )
        .expect("generated AbstractHorse metadata accessors must resolve");
    world.apply_add_entity(add_entity_with_type(42, VANILLA_ENTITY_TYPE_DONKEY_ID));
    world.apply_set_entity_data(SetEntityData {
        id: 42,
        values: vec![byte_entity_data(
            accessors.flags_data_id,
            accessors.tame_mask,
        )],
    });
    world.apply_mount_screen_open(MountScreenOpen {
        container_id: 7,
        inventory_columns: 3,
        entity_id: 42,
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 8,
            y: 18,
        }
    );
    assert!(!layout.slots.iter().any(|slot| slot.slot_id == 1));
    assert_eq!(layout.slots.len(), 46);
    assert_eq!(
        layout.slots[1],
        InventorySlotLayout {
            slot_id: 2,
            x: 80,
            y: 18,
        }
    );
}

#[test]
fn mount_baby_tamed_donkey_layout_hides_equipment_slots() {
    let mut world = WorldStore::new();
    let accessors =
        bbb_client_world::vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_DONKEY_ID,
        )
        .expect("generated AbstractHorse metadata accessors must resolve");
    world.apply_add_entity(add_entity_with_type(42, VANILLA_ENTITY_TYPE_DONKEY_ID));
    world.apply_set_entity_data(SetEntityData {
        id: 42,
        values: vec![
            byte_entity_data(accessors.flags_data_id, accessors.tame_mask),
            baby_data(VANILLA_ENTITY_TYPE_DONKEY_ID, true),
        ],
    });
    world.apply_mount_screen_open(MountScreenOpen {
        container_id: 7,
        inventory_columns: 3,
        entity_id: 42,
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert!(!layout.slots.iter().any(|slot| slot.slot_id == 0));
    assert!(!layout.slots.iter().any(|slot| slot.slot_id == 1));
    assert_eq!(layout.slots[0].slot_id, 2);
}

#[test]
fn lectern_layout_matches_vanilla_book_screen() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LECTERN_ID,
        title: bbb_protocol::RichTextComponent::plain("Lectern"),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 192);
    assert_eq!(layout.height, 192);
    assert_eq!(layout.background, InventoryScreenBackground::Lectern);
    assert!(layout.slots.is_empty());
}

#[test]
fn shulker_box_layout_matches_vanilla_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SHULKER_BOX_ID,
        title: bbb_protocol::RichTextComponent::plain("Shulker Box"),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 176);
    assert_eq!(layout.height, 167);
    assert_eq!(layout.background, InventoryScreenBackground::ShulkerBox);
    assert_eq!(layout.slots.len(), 63);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 8,
            y: 18,
        }
    );
    assert_eq!(
        layout.slots[26],
        InventorySlotLayout {
            slot_id: 26,
            x: 152,
            y: 54,
        }
    );
    assert_eq!(
        layout.slots[27],
        InventorySlotLayout {
            slot_id: 27,
            x: 8,
            y: 84,
        }
    );
    assert_eq!(
        layout.slots[62],
        InventorySlotLayout {
            slot_id: 62,
            x: 152,
            y: 142,
        }
    );
}

#[test]
fn loom_layout_matches_vanilla_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 176);
    assert_eq!(layout.height, 166);
    assert_eq!(layout.background, InventoryScreenBackground::Loom);
    assert_eq!(layout.slots.len(), 40);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 13,
            y: 26,
        }
    );
    assert_eq!(
        layout.slots[1],
        InventorySlotLayout {
            slot_id: 1,
            x: 33,
            y: 26,
        }
    );
    assert_eq!(
        layout.slots[2],
        InventorySlotLayout {
            slot_id: 2,
            x: 23,
            y: 45,
        }
    );
    assert_eq!(
        layout.slots[3],
        InventorySlotLayout {
            slot_id: 3,
            x: 143,
            y: 57,
        }
    );
    assert_eq!(
        layout.slots[4],
        InventorySlotLayout {
            slot_id: 4,
            x: 8,
            y: 84,
        }
    );
    assert_eq!(
        layout.slots[39],
        InventorySlotLayout {
            slot_id: 39,
            x: 152,
            y: 142,
        }
    );
}

#[test]
fn merchant_layout_matches_vanilla_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 276);
    assert_eq!(layout.height, 166);
    assert_eq!(layout.background, InventoryScreenBackground::Merchant);
    assert_eq!(layout.slots.len(), 39);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 136,
            y: 37,
        }
    );
    assert_eq!(
        layout.slots[1],
        InventorySlotLayout {
            slot_id: 1,
            x: 162,
            y: 37,
        }
    );
    assert_eq!(
        layout.slots[2],
        InventorySlotLayout {
            slot_id: 2,
            x: 220,
            y: 37,
        }
    );
    assert_eq!(
        layout.slots[3],
        InventorySlotLayout {
            slot_id: 3,
            x: 108,
            y: 84,
        }
    );
    assert_eq!(
        layout.slots[38],
        InventorySlotLayout {
            slot_id: 38,
            x: 252,
            y: 142,
        }
    );
}

#[test]
fn smithing_layout_matches_vanilla_item_combiner_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 176);
    assert_eq!(layout.height, 166);
    assert_eq!(layout.background, InventoryScreenBackground::Smithing);
    assert_eq!(layout.slots.len(), 40);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 8,
            y: 48,
        }
    );
    assert_eq!(
        layout.slots[1],
        InventorySlotLayout {
            slot_id: 1,
            x: 26,
            y: 48,
        }
    );
    assert_eq!(
        layout.slots[2],
        InventorySlotLayout {
            slot_id: 2,
            x: 44,
            y: 48,
        }
    );
    assert_eq!(
        layout.slots[3],
        InventorySlotLayout {
            slot_id: 3,
            x: 98,
            y: 48,
        }
    );
    assert_eq!(
        layout.slots[4],
        InventorySlotLayout {
            slot_id: 4,
            x: 8,
            y: 84,
        }
    );
    assert_eq!(
        layout.slots[39],
        InventorySlotLayout {
            slot_id: 39,
            x: 152,
            y: 142,
        }
    );
}

#[test]
fn cartography_table_layout_matches_vanilla_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 176);
    assert_eq!(layout.height, 166);
    assert_eq!(
        layout.background,
        InventoryScreenBackground::CartographyTable
    );
    assert_eq!(layout.slots.len(), 39);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 15,
            y: 15,
        }
    );
    assert_eq!(
        layout.slots[1],
        InventorySlotLayout {
            slot_id: 1,
            x: 15,
            y: 52,
        }
    );
    assert_eq!(
        layout.slots[2],
        InventorySlotLayout {
            slot_id: 2,
            x: 145,
            y: 39,
        }
    );
    assert_eq!(
        layout.slots[3],
        InventorySlotLayout {
            slot_id: 3,
            x: 8,
            y: 84,
        }
    );
    assert_eq!(
        layout.slots[38],
        InventorySlotLayout {
            slot_id: 38,
            x: 152,
            y: 142,
        }
    );
}

#[test]
fn stonecutter_layout_matches_vanilla_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });

    let layout = inventory_screen_layout(&world).unwrap();

    assert_eq!(layout.width, 176);
    assert_eq!(layout.height, 166);
    assert_eq!(layout.background, InventoryScreenBackground::Stonecutter);
    assert_eq!(layout.slots.len(), 38);
    assert_eq!(
        layout.slots[0],
        InventorySlotLayout {
            slot_id: 0,
            x: 20,
            y: 33,
        }
    );
    assert_eq!(
        layout.slots[1],
        InventorySlotLayout {
            slot_id: 1,
            x: 143,
            y: 33,
        }
    );
    assert_eq!(
        layout.slots[2],
        InventorySlotLayout {
            slot_id: 2,
            x: 8,
            y: 84,
        }
    );
    assert_eq!(
        layout.slots[37],
        InventorySlotLayout {
            slot_id: 37,
            x: 152,
            y: 142,
        }
    );
}

#[test]
fn generic_container_hit_test_uses_vanilla_screen_height() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X6_ID,
        title: bbb_protocol::RichTextComponent::plain("Large Chest"),
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 267.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(704.0, 446.0)), size),
        Some(InventoryClickTarget::Slot(89))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(600.0, 375.0)), size),
        Some(InventoryClickTarget::EmptyPanel)
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(551.0, 249.0)), size),
        Some(InventoryClickTarget::Outside)
    );
}

#[test]
fn generic_3x3_hit_test_uses_vanilla_dispenser_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Dispenser"),
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(614.0, 294.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(650.0, 330.0)), size),
        Some(InventoryClickTarget::Slot(8))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(704.0, 419.0)), size),
        Some(InventoryClickTarget::Slot(44))
    );
}

#[test]
fn crafter_hit_test_uses_vanilla_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTER_3X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafter"),
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(586.0, 302.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(622.0, 338.0)), size),
        Some(InventoryClickTarget::Slot(8))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(694.0, 320.0)), size),
        Some(InventoryClickTarget::Slot(45))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
        Some(InventoryClickTarget::Slot(44))
    );
}

#[test]
fn crafting_table_hit_test_uses_vanilla_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(684.0, 320.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(590.0, 302.0)), size),
        Some(InventoryClickTarget::Slot(1))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(626.0, 338.0)), size),
        Some(InventoryClickTarget::Slot(9))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
        Some(InventoryClickTarget::Slot(45))
    );
}

#[test]
fn enchantment_table_hit_test_uses_vanilla_slots_and_buttons() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        title: bbb_protocol::RichTextComponent::plain("Enchanting Table"),
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(575.0, 332.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(595.0, 332.0)), size),
        Some(InventoryClickTarget::Slot(1))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
        Some(InventoryClickTarget::Slot(37))
    );
    assert_eq!(
        enchantment_button_at_position(&world, Some(PhysicalPosition::new(620.0, 296.0)), size),
        Some(0)
    );
    assert_eq!(
        enchantment_button_at_position(&world, Some(PhysicalPosition::new(620.0, 334.0)), size),
        Some(2)
    );
}

#[test]
fn anvil_hit_test_uses_vanilla_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(587.0, 332.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(636.0, 332.0)), size),
        Some(InventoryClickTarget::Slot(1))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(694.0, 332.0)), size),
        Some(InventoryClickTarget::Slot(2))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
        Some(InventoryClickTarget::Slot(38))
    );
}

#[test]
fn beacon_hit_test_uses_vanilla_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut input = ClientInputState::container_for_test();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
        title: bbb_protocol::RichTextComponent::plain("Beacon"),
    });
    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: BEACON_LEVELS_DATA_ID,
        value: 4,
    });
    sync_beacon_effect_selection(&mut input, &world);

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(669.0, 369.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(569.0, 396.0)), size),
        Some(InventoryClickTarget::Slot(1))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(713.0, 454.0)), size),
        Some(InventoryClickTarget::Slot(36))
    );
    assert_eq!(
        beacon_button_at_position(
            &input,
            &world,
            Some(PhysicalPosition::new(590.0, 283.0)),
            size
        ),
        Some(BeaconClickTarget::Effect {
            primary: true,
            effect_id: VANILLA_MOB_EFFECT_SPEED_ID,
        })
    );
    assert_eq!(
        beacon_button_at_position(
            &input,
            &world,
            Some(PhysicalPosition::new(680.0, 308.0)),
            size
        ),
        Some(BeaconClickTarget::Effect {
            primary: false,
            effect_id: VANILLA_MOB_EFFECT_REGENERATION_ID,
        })
    );
    assert_eq!(
        beacon_button_at_position(
            &input,
            &world,
            Some(PhysicalPosition::new(704.0, 308.0)),
            size
        ),
        None
    );
    assert_eq!(
        beacon_button_at_position(
            &input,
            &world,
            Some(PhysicalPosition::new(700.0, 369.0)),
            size
        ),
        Some(BeaconClickTarget::Confirm)
    );
    assert_eq!(
        beacon_button_at_position(
            &input,
            &world,
            Some(PhysicalPosition::new(726.0, 369.0)),
            size
        ),
        Some(BeaconClickTarget::Cancel)
    );
}

#[test]
fn brewing_stand_hit_test_uses_vanilla_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_BREWING_STAND_ID,
        title: bbb_protocol::RichTextComponent::plain("Brewing Stand"),
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(577.0, 302.0)), size),
        Some(InventoryClickTarget::Slot(4))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(639.0, 302.0)), size),
        Some(InventoryClickTarget::Slot(3))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(616.0, 336.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(662.0, 336.0)), size),
        Some(InventoryClickTarget::Slot(2))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
        Some(InventoryClickTarget::Slot(40))
    );
}

#[test]
fn furnace_hit_test_uses_vanilla_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
        title: bbb_protocol::RichTextComponent::plain("Furnace"),
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(616.0, 302.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(616.0, 338.0)), size),
        Some(InventoryClickTarget::Slot(1))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(676.0, 320.0)), size),
        Some(InventoryClickTarget::Slot(2))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
        Some(InventoryClickTarget::Slot(38))
    );
}

#[test]
fn grindstone_hit_test_uses_vanilla_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
        title: bbb_protocol::RichTextComponent::plain("Grindstone"),
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(609.0, 304.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(609.0, 325.0)), size),
        Some(InventoryClickTarget::Slot(1))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(689.0, 319.0)), size),
        Some(InventoryClickTarget::Slot(2))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
        Some(InventoryClickTarget::Slot(38))
    );
}

#[test]
fn hopper_hit_test_uses_vanilla_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_HOPPER_ID,
        title: bbb_protocol::RichTextComponent::plain("Hopper"),
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(604.0, 314.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(676.0, 314.0)), size),
        Some(InventoryClickTarget::Slot(4))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(704.0, 403.0)), size),
        Some(InventoryClickTarget::Slot(40))
    );
}

#[test]
fn mount_horse_hit_test_uses_vanilla_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_add_entity(add_entity_with_type(42, 66));
    world.apply_mount_screen_open(MountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 295.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(704.0, 331.0)), size),
        Some(InventoryClickTarget::Slot(16))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
        Some(InventoryClickTarget::Slot(52))
    );
}

#[test]
fn mount_donkey_hit_test_ignores_inactive_equipment_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_add_entity(add_entity_with_type(42, VANILLA_ENTITY_TYPE_DONKEY_ID));
    world.apply_mount_screen_open(MountScreenOpen {
        container_id: 7,
        inventory_columns: 3,
        entity_id: 42,
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 295.0)), size),
        Some(InventoryClickTarget::EmptyPanel)
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 313.0)), size),
        Some(InventoryClickTarget::EmptyPanel)
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(632.0, 295.0)), size),
        Some(InventoryClickTarget::Slot(2))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 361.0)), size),
        Some(InventoryClickTarget::Slot(11))
    );
}

#[test]
fn mount_tamed_donkey_hit_test_uses_active_saddle_slot_only() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    let accessors =
        bbb_client_world::vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_DONKEY_ID,
        )
        .expect("generated AbstractHorse metadata accessors must resolve");
    world.apply_add_entity(add_entity_with_type(42, VANILLA_ENTITY_TYPE_DONKEY_ID));
    world.apply_set_entity_data(SetEntityData {
        id: 42,
        values: vec![byte_entity_data(
            accessors.flags_data_id,
            accessors.tame_mask,
        )],
    });
    world.apply_mount_screen_open(MountScreenOpen {
        container_id: 7,
        inventory_columns: 3,
        entity_id: 42,
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 295.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 313.0)), size),
        Some(InventoryClickTarget::EmptyPanel)
    );
}

#[test]
fn lectern_hit_test_uses_book_screen_and_page_buttons() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LECTERN_ID,
        title: bbb_protocol::RichTextComponent::plain("Lectern"),
    });
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![lectern_writable_book(3)],
        carried_item: ItemStackSummary::empty(),
    });
    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: 0,
        value: 1,
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(545.0, 265.0)), size),
        Some(InventoryClickTarget::EmptyPanel)
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(543.0, 264.0)), size),
        Some(InventoryClickTarget::Outside)
    );
    assert_eq!(
        lectern_button_at_position(&world, Some(PhysicalPosition::new(588.0, 160.0)), size),
        Some(LecternClickTarget::MenuButton(LECTERN_BUTTON_PREV_PAGE))
    );
    assert_eq!(
        lectern_button_at_position(&world, Some(PhysicalPosition::new(661.0, 160.0)), size),
        Some(LecternClickTarget::MenuButton(LECTERN_BUTTON_NEXT_PAGE))
    );
    assert_eq!(
        lectern_button_at_position(&world, Some(PhysicalPosition::new(560.0, 197.0)), size),
        Some(LecternClickTarget::Done)
    );
    assert_eq!(
        lectern_button_at_position(&world, Some(PhysicalPosition::new(660.0, 197.0)), size),
        Some(LecternClickTarget::MenuButton(LECTERN_BUTTON_TAKE_BOOK))
    );
    assert_eq!(
        inventory_screen_cursor_position(&world, Some(PhysicalPosition::new(661.0, 160.0)), size),
        Some((661, 160)),
        "HudBookScreen hover coordinates stay surface-relative"
    );
    assert_eq!(
        lectern_button_at_position(&world, Some(PhysicalPosition::new(588.0, 422.0)), size),
        None,
        "the former vertically centered hit position is stale"
    );

    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: 0,
        value: 0,
    });
    assert_eq!(
        lectern_button_at_position(&world, Some(PhysicalPosition::new(588.0, 160.0)), size),
        None
    );
    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: 0,
        value: 2,
    });
    assert_eq!(
        lectern_button_at_position(&world, Some(PhysicalPosition::new(661.0, 160.0)), size),
        None
    );

    world.apply_game_event(GameEvent {
        event_id: VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID,
        param: VANILLA_GAME_TYPE_ADVENTURE_ID as f32,
    });
    assert_eq!(
        lectern_button_at_position(&world, Some(PhysicalPosition::new(660.0, 197.0)), size),
        Some(LecternClickTarget::Done),
        "adventure uses BookViewScreen's single full-width Done button"
    );
}

#[test]
fn shulker_box_hit_test_uses_vanilla_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SHULKER_BOX_ID,
        title: bbb_protocol::RichTextComponent::plain("Shulker Box"),
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(560.0, 303.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 339.0)), size),
        Some(InventoryClickTarget::Slot(26))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
        Some(InventoryClickTarget::Slot(62))
    );
}

#[test]
fn loom_hit_test_uses_vanilla_slots() {
    let size = PhysicalSize::new(1280, 720);
    let world = loom_world_with_banner_and_dye();

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(573.0, 311.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(593.0, 311.0)), size),
        Some(InventoryClickTarget::Slot(1))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(583.0, 330.0)), size),
        Some(InventoryClickTarget::Slot(2))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(703.0, 342.0)), size),
        Some(InventoryClickTarget::Slot(3))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
        Some(InventoryClickTarget::Slot(39))
    );
    assert_eq!(
        loom_click_target_at_position(&world, 0, Some(PhysicalPosition::new(620.0, 296.0)), size),
        Some(LoomClickTarget::Pattern(0))
    );
    assert_eq!(
        loom_click_target_at_position(&world, 0, Some(PhysicalPosition::new(661.0, 339.0)), size),
        Some(LoomClickTarget::Pattern(15))
    );
    assert!(loom_scroller_at_position(
        &world,
        Some(PhysicalPosition::new(674.0, 300.0)),
        size
    ));
}

#[test]
fn loom_pattern_click_queues_container_button_command() {
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
        Some(PhysicalPosition::new(632.0, 310.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(input.loom_selected_pattern_index(), Some(5));
    assert_eq!(
        world.open_container_data_value(LOOM_SELECTED_PATTERN_DATA_ID),
        Some(5)
    );
    assert_eq!(
        world.take_pending_local_interaction_sound_actions(),
        vec![bbb_client_world::LocalInteractionSoundAction::LoomSelectPattern]
    );
    assert_eq!(counters.container_button_click_commands_queued, 1);
    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerButtonClick(ContainerButtonClick {
            container_id: 7,
            button_id: 5,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn loom_pattern_click_targets_use_the_component_holder_set_size() {
    let mut world = loom_world_with_banner_and_dye();
    let mut pattern_stack = item_stack(44, 1);
    pattern_stack
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID);
    pattern_stack
        .component_patch
        .provides_banner_pattern_registry_ids = vec![5, 2];
    let mut items = vec![ItemStackSummary::empty(); 40];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[2] = pattern_stack;
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 13,
        items,
        carried_item: ItemStackSummary::empty(),
    });

    assert_eq!(loom_selectable_pattern_count(&world), Some(2));
    assert_eq!(
        loom_click_target_at_position(
            &world,
            0,
            Some(PhysicalPosition::new(632.0, 296.0)),
            PhysicalSize::new(1280, 720),
        ),
        Some(LoomClickTarget::Pattern(1))
    );
    assert_eq!(
        loom_click_target_at_position(
            &world,
            0,
            Some(PhysicalPosition::new(646.0, 296.0)),
            PhysicalSize::new(1280, 720),
        ),
        None
    );
}

#[test]
fn merchant_hit_test_uses_vanilla_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: bbb_protocol::RichTextComponent::plain("Merchant"),
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(646.0, 322.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(672.0, 322.0)), size),
        Some(InventoryClickTarget::Slot(1))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(730.0, 322.0)), size),
        Some(InventoryClickTarget::Slot(2))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(618.0, 369.0)), size),
        Some(InventoryClickTarget::Slot(3))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(762.0, 427.0)), size),
        Some(InventoryClickTarget::Slot(38))
    );
}

#[test]
fn smithing_hit_test_uses_vanilla_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: bbb_protocol::RichTextComponent::plain("Smithing"),
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(568.0, 333.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(586.0, 333.0)), size),
        Some(InventoryClickTarget::Slot(1))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(604.0, 333.0)), size),
        Some(InventoryClickTarget::Slot(2))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(658.0, 333.0)), size),
        Some(InventoryClickTarget::Slot(3))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
        Some(InventoryClickTarget::Slot(39))
    );
}

#[test]
fn cartography_table_hit_test_uses_vanilla_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        title: bbb_protocol::RichTextComponent::plain("Cartography Table"),
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(575.0, 300.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(575.0, 337.0)), size),
        Some(InventoryClickTarget::Slot(1))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(705.0, 324.0)), size),
        Some(InventoryClickTarget::Slot(2))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
        Some(InventoryClickTarget::Slot(38))
    );
}

#[test]
fn stonecutter_hit_test_uses_vanilla_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
        title: bbb_protocol::RichTextComponent::plain("Stonecutter"),
    });

    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(580.0, 318.0)), size),
        Some(InventoryClickTarget::Slot(0))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(703.0, 318.0)), size),
        Some(InventoryClickTarget::Slot(1))
    );
    assert_eq!(
        inventory_screen_click_target(&world, Some(PhysicalPosition::new(712.0, 427.0)), size),
        Some(InventoryClickTarget::Slot(37))
    );
}
