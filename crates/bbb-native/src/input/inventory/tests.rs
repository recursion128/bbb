use super::*;
use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::packets::{
    AddEntity, ContainerButtonClick, ContainerClick, ContainerCloseRequest, ContainerSetContent,
    ContainerSetData, ContainerSlotStateChanged, EntityDataValue, EntityDataValueKind,
    HashedComponentPatch, HashedItemStack, HashedStack, IngredientSummary, ItemCostSummary,
    ItemStackSummary, MerchantOffer, MerchantOffers, MountScreenOpen, OpenScreen, PlayerAbilities,
    PlayerExperience, RecipeBookChangeSettingsCommand, RecipeBookSettings, RecipeBookType,
    RecipeBookTypeSettings, RecipePropertySetSummary, RegistryTags, SelectBundleItem,
    SelectTradeCommand, SetBeacon, SetCursorItem, SetEntityData, SetPlayerInventory,
    SlotDisplaySummary, StonecutterSelectableRecipeSummary, TagNetworkPayload, UpdateRecipes,
    UpdateTags, Vec3d,
};
use uuid::Uuid;

const TEST_AGEABLE_MOB_BABY_DATA_ID: u8 = 16;
const TEST_MOUNT_TAME_FLAGS_DATA_ID: u8 = 18;
const TEST_ABSTRACT_HORSE_TAME_FLAG: i8 = 2;
const TEST_TAMABLE_ANIMAL_TAME_FLAG: i8 = 4;
const TEST_MAX_DAMAGE_COMPONENT_ID: i32 = 2;
const TEST_DAMAGE_COMPONENT_ID: i32 = 3;
const TEST_MAP_ID_COMPONENT_ID: i32 = 41;
const TEST_HASH_OPS_INT_7_HASH: i32 = -1_726_626_450;
const TEST_MAP_ID_7_HASH: i32 = TEST_HASH_OPS_INT_7_HASH;

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
        menu_type_id: 5,
        title: "Large Chest".to_string(),
        title_styled: Vec::new(),
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
fn generic_3x3_layout_matches_vanilla_dispenser_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: 6,
        title: "Dispenser".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: CRAFTER_MENU_TYPE_ID,
        title: "Crafter".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: CRAFTING_MENU_TYPE_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: CRAFTING_MENU_TYPE_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
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
fn enchantment_table_layout_matches_vanilla_menu() {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: ENCHANTMENT_MENU_TYPE_ID,
        title: "Enchanting Table".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: ANVIL_MENU_TYPE_ID,
        title: "Anvil".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: BEACON_MENU_TYPE_ID,
        title: "Beacon".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: BREWING_STAND_MENU_TYPE_ID,
        title: "Brewing Stand".to_string(),
        title_styled: Vec::new(),
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
            BLAST_FURNACE_MENU_TYPE_ID,
            "Blast Furnace",
            InventoryScreenBackground::BlastFurnace,
        ),
        (
            FURNACE_MENU_TYPE_ID,
            "Furnace",
            InventoryScreenBackground::Furnace,
        ),
        (
            SMOKER_MENU_TYPE_ID,
            "Smoker",
            InventoryScreenBackground::Smoker,
        ),
    ] {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id,
            title: title.to_string(),
            title_styled: Vec::new(),
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
        menu_type_id: GRINDSTONE_MENU_TYPE_ID,
        title: "Grindstone".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: 16,
        title: "Hopper".to_string(),
        title_styled: Vec::new(),
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
    world.apply_add_entity(add_entity_with_type(42, 88));
    world.apply_set_entity_data(SetEntityData {
        id: 42,
        values: vec![byte_entity_data(
            TEST_MOUNT_TAME_FLAGS_DATA_ID,
            TEST_TAMABLE_ANIMAL_TAME_FLAG,
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
    world.apply_add_entity(add_entity_with_type(42, 36));
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
    world.apply_add_entity(add_entity_with_type(42, 36));
    world.apply_set_entity_data(SetEntityData {
        id: 42,
        values: vec![byte_entity_data(
            TEST_MOUNT_TAME_FLAGS_DATA_ID,
            TEST_ABSTRACT_HORSE_TAME_FLAG,
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
    world.apply_add_entity(add_entity_with_type(42, 36));
    world.apply_set_entity_data(SetEntityData {
        id: 42,
        values: vec![
            byte_entity_data(TEST_MOUNT_TAME_FLAGS_DATA_ID, TEST_ABSTRACT_HORSE_TAME_FLAG),
            bool_entity_data(TEST_AGEABLE_MOB_BABY_DATA_ID, true),
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
        menu_type_id: LECTERN_MENU_TYPE_ID,
        title: "Lectern".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: 20,
        title: "Shulker Box".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: LOOM_MENU_TYPE_ID,
        title: "Loom".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: MERCHANT_MENU_TYPE_ID,
        title: "Merchant".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: SMITHING_MENU_TYPE_ID,
        title: "Smithing".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: CARTOGRAPHY_TABLE_MENU_TYPE_ID,
        title: "Cartography Table".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: STONECUTTER_MENU_TYPE_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: 5,
        title: "Large Chest".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: 6,
        title: "Dispenser".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: CRAFTER_MENU_TYPE_ID,
        title: "Crafter".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: CRAFTING_MENU_TYPE_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: ENCHANTMENT_MENU_TYPE_ID,
        title: "Enchanting Table".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: ANVIL_MENU_TYPE_ID,
        title: "Anvil".to_string(),
        title_styled: Vec::new(),
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
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: BEACON_MENU_TYPE_ID,
        title: "Beacon".to_string(),
        title_styled: Vec::new(),
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
            effect_id: BEACON_EFFECT_SPEED_ID,
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
            effect_id: BEACON_EFFECT_REGENERATION_ID,
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
        menu_type_id: BREWING_STAND_MENU_TYPE_ID,
        title: "Brewing Stand".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: FURNACE_MENU_TYPE_ID,
        title: "Furnace".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: GRINDSTONE_MENU_TYPE_ID,
        title: "Grindstone".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: 16,
        title: "Hopper".to_string(),
        title_styled: Vec::new(),
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
    world.apply_add_entity(add_entity_with_type(42, 36));
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
    world.apply_add_entity(add_entity_with_type(42, 36));
    world.apply_set_entity_data(SetEntityData {
        id: 42,
        values: vec![byte_entity_data(
            TEST_MOUNT_TAME_FLAGS_DATA_ID,
            TEST_ABSTRACT_HORSE_TAME_FLAG,
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
        menu_type_id: LECTERN_MENU_TYPE_ID,
        title: "Lectern".to_string(),
        title_styled: Vec::new(),
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
        lectern_button_at_position(&world, Some(PhysicalPosition::new(588.0, 422.0)), size),
        Some(LecternClickTarget::MenuButton(LECTERN_BUTTON_PREV_PAGE))
    );
    assert_eq!(
        lectern_button_at_position(&world, Some(PhysicalPosition::new(661.0, 422.0)), size),
        Some(LecternClickTarget::MenuButton(LECTERN_BUTTON_NEXT_PAGE))
    );
    assert_eq!(
        lectern_button_at_position(&world, Some(PhysicalPosition::new(560.0, 464.0)), size),
        Some(LecternClickTarget::Done)
    );
    assert_eq!(
        lectern_button_at_position(&world, Some(PhysicalPosition::new(660.0, 464.0)), size),
        Some(LecternClickTarget::MenuButton(LECTERN_BUTTON_TAKE_BOOK))
    );
}

#[test]
fn shulker_box_hit_test_uses_vanilla_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: 20,
        title: "Shulker Box".to_string(),
        title_styled: Vec::new(),
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
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: LOOM_MENU_TYPE_ID,
        title: "Loom".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 40];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });

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
    let mut input = ClientInputState::new(true);
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
fn loom_pattern_scroll_changes_visible_button_indices() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
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
    let mut input = ClientInputState::new(true);
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
    assert!(input.loom_pattern_scrolling);
    assert_eq!(input.loom_pattern_scroll_row(), 0);

    assert!(handle_inventory_cursor_moved(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(PhysicalPosition::new(674.0, 328.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(input.loom_pattern_scrolling);
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
    assert!(!input.loom_pattern_scrolling);

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
fn merchant_hit_test_uses_vanilla_slots() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: MERCHANT_MENU_TYPE_ID,
        title: "Merchant".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: SMITHING_MENU_TYPE_ID,
        title: "Smithing".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: CARTOGRAPHY_TABLE_MENU_TYPE_ID,
        title: "Cartography Table".to_string(),
        title_styled: Vec::new(),
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
        menu_type_id: STONECUTTER_MENU_TYPE_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
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

#[test]
fn stonecutter_recipe_grid_hit_test_uses_vanilla_first_page_buttons() {
    let size = PhysicalSize::new(1280, 720);
    let mut world = WorldStore::new();
    world.apply_update_recipes(UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: vec![stonecutter_recipe(vec![42])],
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: STONECUTTER_MENU_TYPE_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 38];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });

    assert_eq!(
        stonecutter_recipe_button_at_position(
            &world,
            0,
            Some(PhysicalPosition::new(612.0, 300.0)),
            size
        ),
        Some(0)
    );
    assert_eq!(
        stonecutter_recipe_button_at_position(
            &world,
            0,
            Some(PhysicalPosition::new(660.0, 336.0)),
            size
        ),
        Some(11)
    );
    assert_eq!(
        stonecutter_recipe_button_at_position(
            &world,
            0,
            Some(PhysicalPosition::new(669.0, 300.0)),
            size
        ),
        None
    );
}

#[test]
fn stonecutter_recipe_button_click_queues_container_button_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_recipes(UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: (0..6).map(|_| stonecutter_recipe(vec![42])).collect(),
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: STONECUTTER_MENU_TYPE_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
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
        Some(PhysicalPosition::new(628.0, 318.0)),
        PhysicalSize::new(1280, 720),
    ));

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
fn stonecutter_recipe_button_right_click_matches_vanilla_button_path() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_recipes(UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: (0..6).map(|_| stonecutter_recipe(vec![42])).collect(),
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: STONECUTTER_MENU_TYPE_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
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
        MouseButton::Right,
        ElementState::Pressed,
        Some(PhysicalPosition::new(628.0, 318.0)),
        PhysicalSize::new(1280, 720),
    ));

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
fn stonecutter_mouse_wheel_scrolls_recipe_grid_button_index_by_rows() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_recipes(UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: (0..25).map(|_| stonecutter_recipe(vec![42])).collect(),
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: STONECUTTER_MENU_TYPE_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
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
    assert_eq!(input.stonecutter_recipe_scroll_row, 2);

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
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_recipes(UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: (0..25).map(|_| stonecutter_recipe(vec![42])).collect(),
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: STONECUTTER_MENU_TYPE_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
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
    assert!(input.stonecutter_recipe_scrolling);
    assert_eq!(input.stonecutter_recipe_scroll_row, 0);

    assert!(handle_inventory_cursor_moved(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(PhysicalPosition::new(676.0, 318.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(input.stonecutter_recipe_scrolling);
    assert_eq!(input.stonecutter_recipe_scroll_row, 2);
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
    assert!(!input.stonecutter_recipe_scrolling);

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
    let mut input = ClientInputState::new(true);
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
        menu_type_id: STONECUTTER_MENU_TYPE_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
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
    assert_eq!(input.stonecutter_recipe_scroll_row, 2);

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

    assert_eq!(input.stonecutter_recipe_scroll_row, 0);
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

#[test]
fn stonecutter_recipe_button_click_requires_matching_input_recipe() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_recipes(UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: vec![stonecutter_recipe(vec![42])],
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: STONECUTTER_MENU_TYPE_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
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

    let mut items = vec![ItemStackSummary::empty(); 38];
    items[0] = item_stack(99, 1);
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
        Some(PhysicalPosition::new(612.0, 300.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_button_click_commands_queued, 0);
    assert_eq!(counters.container_click_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn stonecutter_recipe_button_click_ignores_already_selected_recipe() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_recipes(UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: (0..6).map(|_| stonecutter_recipe(vec![42])).collect(),
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: STONECUTTER_MENU_TYPE_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 38];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: STONECUTTER_SELECTED_RECIPE_DATA_ID,
        value: 5,
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(628.0, 318.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_button_click_commands_queued, 0);
    assert_eq!(counters.container_click_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn crafter_empty_grid_click_queues_slot_state_change_and_pickup() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: CRAFTER_MENU_TYPE_ID,
        title: "Crafter".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![ItemStackSummary::empty(); 46],
        carried_item: ItemStackSummary::empty(),
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(586.0, 302.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_slot_state_changed_commands_queued, 1);
    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerSlotStateChanged(ContainerSlotStateChanged {
            slot_id: 0,
            container_id: 7,
            new_state: false,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::Empty,
        })
    );
    assert_eq!(world.open_container_data_value(0), None);
}

#[test]
fn enchantment_table_option_click_queues_button_command_when_cost_is_available() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: ENCHANTMENT_MENU_TYPE_ID,
        title: "Enchanting Table".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![ItemStackSummary::empty(); 38],
        carried_item: ItemStackSummary::empty(),
    });
    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: 2,
        value: 30,
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(620.0, 334.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_button_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerButtonClick(ContainerButtonClick {
            container_id: 7,
            button_id: 2,
        })
    );
    assert_eq!(counters.container_click_commands_queued, 0);
}

#[test]
fn enchantment_table_option_click_ignores_zero_cost_buttons() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: ENCHANTMENT_MENU_TYPE_ID,
        title: "Enchanting Table".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![ItemStackSummary::empty(); 38],
        carried_item: ItemStackSummary::empty(),
    });

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

    assert_eq!(counters.container_button_click_commands_queued, 0);
    assert_eq!(counters.container_click_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn beacon_confirm_click_queues_set_beacon_then_close_when_active() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: BEACON_MENU_TYPE_ID,
        title: "Beacon".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 37];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: BEACON_PRIMARY_EFFECT_DATA_ID,
        value: 5,
    });
    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: BEACON_SECONDARY_EFFECT_DATA_ID,
        value: 8,
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(700.0, 369.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.set_beacon_commands_queued, 1);
    assert_eq!(counters.container_close_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SetBeacon(SetBeacon {
            primary_effect: Some(4),
            secondary_effect: Some(7),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClose(ContainerCloseRequest { container_id: 7 })
    );
    assert!(world.inventory().open_container.is_none());
    assert!(rx.try_recv().is_err());
}

#[test]
fn beacon_effect_clicks_update_local_selection_and_confirm_submits_it() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: BEACON_MENU_TYPE_ID,
        title: "Beacon".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 37];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: BEACON_LEVELS_DATA_ID,
        value: 4,
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(600.0, 333.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert_eq!(input.beacon_effect_selection(), (Some(4), None));
    assert_eq!(counters.set_beacon_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(704.0, 308.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert_eq!(input.beacon_effect_selection(), (Some(4), Some(4)));
    assert_eq!(counters.set_beacon_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(700.0, 369.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.set_beacon_commands_queued, 1);
    assert_eq!(counters.container_close_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SetBeacon(SetBeacon {
            primary_effect: Some(BEACON_EFFECT_STRENGTH_ID),
            secondary_effect: Some(BEACON_EFFECT_STRENGTH_ID),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClose(ContainerCloseRequest { container_id: 7 })
    );
    assert!(world.inventory().open_container.is_none());
    assert!(rx.try_recv().is_err());
}

#[test]
fn beacon_effect_click_ignores_power_buttons_above_current_level() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: BEACON_MENU_TYPE_ID,
        title: "Beacon".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 37];
    items[0] = item_stack(42, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: BEACON_LEVELS_DATA_ID,
        value: 1,
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(600.0, 333.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert_eq!(input.beacon_effect_selection(), (None, None));

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(700.0, 369.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.set_beacon_commands_queued, 0);
    assert_eq!(counters.container_close_commands_queued, 0);
    assert!(world.inventory().open_container.is_some());
    assert!(rx.try_recv().is_err());
}

#[test]
fn beacon_confirm_click_ignores_disabled_button_without_payment() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: BEACON_MENU_TYPE_ID,
        title: "Beacon".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![ItemStackSummary::empty(); 37],
        carried_item: ItemStackSummary::empty(),
    });
    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: BEACON_PRIMARY_EFFECT_DATA_ID,
        value: 5,
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(700.0, 369.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.set_beacon_commands_queued, 0);
    assert_eq!(counters.container_close_commands_queued, 0);
    assert!(world.inventory().open_container.is_some());
    assert!(rx.try_recv().is_err());
}

#[test]
fn beacon_cancel_click_queues_container_close_request() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: BEACON_MENU_TYPE_ID,
        title: "Beacon".to_string(),
        title_styled: Vec::new(),
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(726.0, 369.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_close_commands_queued, 1);
    assert_eq!(counters.set_beacon_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClose(ContainerCloseRequest { container_id: 7 })
    );
    assert!(world.inventory().open_container.is_none());
    assert!(rx.try_recv().is_err());
}

#[test]
fn lectern_page_button_click_queues_container_button_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: LECTERN_MENU_TYPE_ID,
        title: "Lectern".to_string(),
        title_styled: Vec::new(),
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(661.0, 422.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_button_click_commands_queued, 1);
    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerButtonClick(ContainerButtonClick {
            container_id: 7,
            button_id: LECTERN_BUTTON_NEXT_PAGE,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn lectern_done_button_queues_container_close_request() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: LECTERN_MENU_TYPE_ID,
        title: "Lectern".to_string(),
        title_styled: Vec::new(),
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(560.0, 464.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_close_commands_queued, 1);
    assert_eq!(counters.container_button_click_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClose(ContainerCloseRequest { container_id: 7 })
    );
    assert!(world.inventory().open_container.is_none());
    assert!(rx.try_recv().is_err());
}

#[test]
fn lectern_take_book_button_queues_container_button_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: LECTERN_MENU_TYPE_ID,
        title: "Lectern".to_string(),
        title_styled: Vec::new(),
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(660.0, 464.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_button_click_commands_queued, 1);
    assert_eq!(counters.container_close_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerButtonClick(ContainerButtonClick {
            container_id: 7,
            button_id: LECTERN_BUTTON_TAKE_BOOK,
        })
    );
    assert!(world.inventory().open_container.is_some());
    assert!(rx.try_recv().is_err());
}

#[test]
fn merchant_trade_row_click_queues_select_trade_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: MERCHANT_MENU_TYPE_ID,
        title: "Merchant".to_string(),
        title_styled: Vec::new(),
    });
    assert!(world.apply_merchant_offers(merchant_offers(7, 4)));
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[3] = item_stack(45, 5);
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
        Some(PhysicalPosition::new(545.0, 365.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.select_trade_commands_queued, 1);
    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SelectTrade(SelectTradeCommand { item: 3 })
    );
    assert_eq!(
        world
            .inventory()
            .open_container
            .as_ref()
            .and_then(|container| container.merchant_offers.as_ref())
            .map(|offers| offers.local_selected_offer_index),
        Some(3)
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(45, 5));
    assert_eq!(slots[3].item, ItemStackSummary::empty());
    assert!(rx.try_recv().is_err());
}

#[test]
fn merchant_mouse_wheel_scrolls_visible_trade_window() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: MERCHANT_MENU_TYPE_ID,
        title: "Merchant".to_string(),
        title_styled: Vec::new(),
    });
    assert!(world.apply_merchant_offers(merchant_offers(7, 8)));

    assert!(handle_inventory_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, -1.0),
        Some(PhysicalPosition::new(545.0, 365.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.select_trade_commands_queued, 0);
    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(
        world
            .inventory()
            .open_container
            .as_ref()
            .and_then(|container| container.merchant_offers.as_ref())
            .map(|offers| offers.local_scroll_offset),
        Some(1)
    );
    assert!(rx.try_recv().is_err());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(545.0, 365.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.select_trade_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SelectTrade(SelectTradeCommand { item: 4 })
    );
    assert_eq!(
        world
            .inventory()
            .open_container
            .as_ref()
            .and_then(|container| container.merchant_offers.as_ref())
            .map(|offers| offers.local_selected_offer_index),
        Some(4)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn merchant_scroller_drag_updates_visible_trade_window() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: MERCHANT_MENU_TYPE_ID,
        title: "Merchant".to_string(),
        title_styled: Vec::new(),
    });
    assert!(world.apply_merchant_offers(merchant_offers(7, 12)));

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(598.0, 296.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(input.merchant_trade_scrolling);

    assert!(handle_inventory_cursor_moved(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(PhysicalPosition::new(598.0, 420.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(input.merchant_trade_scrolling);
    assert_eq!(counters.select_trade_commands_queued, 0);
    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(
        world
            .inventory()
            .open_container
            .as_ref()
            .and_then(|container| container.merchant_offers.as_ref())
            .map(|offers| offers.local_scroll_offset),
        Some(5)
    );
    assert!(rx.try_recv().is_err());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Released,
        Some(PhysicalPosition::new(598.0, 420.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(!input.merchant_trade_scrolling);

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(545.0, 300.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.select_trade_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SelectTrade(SelectTradeCommand { item: 5 })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn merchant_trade_row_click_ignores_missing_offer() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: MERCHANT_MENU_TYPE_ID,
        title: "Merchant".to_string(),
        title_styled: Vec::new(),
    });
    assert!(world.apply_merchant_offers(merchant_offers(7, 2)));

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(545.0, 365.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.select_trade_commands_queued, 0);
    assert_eq!(counters.container_click_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn generic_container_mouse_click_queues_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: 5,
        title: "Large Chest".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 90];
    items[0] = item_stack(42, 3);
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
        Some(PhysicalPosition::new(560.0, 267.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: [(0, HashedStack::Empty)].into(),
            carried_item: HashedStack::Item(HashedItemStack {
                item_id: 42,
                count: 3,
                components: HashedComponentPatch::default(),
            }),
        })
    );
    assert_eq!(
        world.inventory().open_container.as_ref().unwrap().slots[0].item,
        ItemStackSummary::empty()
    );
}

#[test]
fn mount_horse_mouse_click_queues_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_add_entity(add_entity_with_type(42, 66));
    world.apply_mount_screen_open(MountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });
    let mut items = vec![ItemStackSummary::empty(); 53];
    items[2] = item_stack(42, 3);
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
        Some(PhysicalPosition::new(632.0, 295.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 2,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: [(2, HashedStack::Empty)].into(),
            carried_item: HashedStack::Item(HashedItemStack {
                item_id: 42,
                count: 3,
                components: HashedComponentPatch::default(),
            }),
        })
    );
    assert_eq!(
        world.inventory().open_container.as_ref().unwrap().slots[2].item,
        ItemStackSummary::empty()
    );
}

#[test]
fn mount_horse_shift_click_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_add_entity(add_entity_with_type(42, 66));
    world.apply_mount_screen_open(MountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });
    let mut items = vec![ItemStackSummary::empty(); 53];
    items[2] = item_stack(42, 3);
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
        Some(PhysicalPosition::new(632.0, 295.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 2,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (2, HashedStack::Empty),
                (52, HashedStack::Item(hashed_item(42, 3))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(slots[52].item, item_stack(42, 3));
}

#[test]
fn mount_horse_shift_click_queues_predicted_saddle_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_default_item_equipment_slots(BTreeMap::from([(90, ItemEquipmentSlot::Saddle)]));
    world.apply_add_entity(add_entity_with_type(42, 66));
    world.apply_mount_screen_open(MountScreenOpen {
        container_id: 7,
        inventory_columns: 5,
        entity_id: 42,
    });
    let mut items = vec![ItemStackSummary::empty(); 53];
    items[17] = item_stack(90, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 13,
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
        Some(PhysicalPosition::new(568.0, 369.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 13,
            slot_num: 17,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Item(hashed_item(90, 1))),
                (17, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(90, 1));
    assert_eq!(slots[17].item, ItemStackSummary::empty());
}

#[test]
fn recipe_book_button_click_toggles_local_setting_and_queues_packet() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: CRAFTING_MENU_TYPE_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_recipe_book_settings(RecipeBookSettings {
        crafting: RecipeBookTypeSettings {
            open: false,
            filtering: true,
        },
        furnace: RecipeBookTypeSettings::default(),
        blast_furnace: RecipeBookTypeSettings::default(),
        smoker: RecipeBookTypeSettings::default(),
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(558.0, 312.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert!(world.recipe_book().settings.crafting.open);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RecipeBookChangeSettings(RecipeBookChangeSettingsCommand {
            book_type: RecipeBookType::Crafting,
            open: true,
            filtering: true,
        })
    );

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(635.0, 312.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert!(!world.recipe_book().settings.crafting.open);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RecipeBookChangeSettings(RecipeBookChangeSettingsCommand {
            book_type: RecipeBookType::Crafting,
            open: false,
            filtering: true,
        })
    );
}

#[test]
fn furnace_mouse_click_queues_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: FURNACE_MENU_TYPE_ID,
        title: "Furnace".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 3);
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
        Some(PhysicalPosition::new(616.0, 302.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: [(0, HashedStack::Empty)].into(),
            carried_item: HashedStack::Item(HashedItemStack {
                item_id: 42,
                count: 3,
                components: HashedComponentPatch::default(),
            }),
        })
    );
    assert_eq!(
        world.inventory().open_container.as_ref().unwrap().slots[0].item,
        ItemStackSummary::empty()
    );
}

#[test]
fn furnace_shift_click_queues_quick_move_to_input_slot() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_recipes(UpdateRecipes {
        property_sets: vec![RecipePropertySetSummary {
            key: "minecraft:furnace_input".to_string(),
            item_ids: vec![42],
        }],
        stonecutter_recipes: Vec::new(),
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: FURNACE_MENU_TYPE_ID,
        title: "Furnace".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[3] = item_stack(42, 3);
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
        Some(PhysicalPosition::new(560.0, 361.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 3,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Item(hashed_item(42, 3))),
                (3, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(42, 3));
    assert_eq!(slots[3].item, ItemStackSummary::empty());
}

#[test]
fn generic_container_shift_click_queues_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: 5,
        title: "Large Chest".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 90];
    items[0] = item_stack(42, 3);
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
        Some(PhysicalPosition::new(560.0, 267.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (89, HashedStack::Item(hashed_item(42, 3))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[89].item, item_stack(42, 3));
}

#[test]
fn generic_3x3_shift_click_queues_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: 6,
        title: "Dispenser".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 45];
    items[0] = item_stack(42, 3);
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
        Some(PhysicalPosition::new(614.0, 294.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (44, HashedStack::Item(hashed_item(42, 3))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[44].item, item_stack(42, 3));
}

#[test]
fn crafting_table_shift_click_input_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: CRAFTING_MENU_TYPE_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 46];
    items[1] = item_stack(42, 3);
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
        Some(PhysicalPosition::new(590.0, 302.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 1,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (1, HashedStack::Empty),
                (10, HashedStack::Item(hashed_item(42, 3))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[10].item, item_stack(42, 3));
}

#[test]
fn enchantment_table_shift_click_input_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: ENCHANTMENT_MENU_TYPE_ID,
        title: "Enchanting Table".to_string(),
        title_styled: Vec::new(),
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
        Some(PhysicalPosition::new(575.0, 332.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (37, HashedStack::Item(hashed_item(42, 1))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[37].item, item_stack(42, 1));
}

#[test]
fn enchantment_table_shift_click_player_lapis_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_enchantment_lapis_lazuli_item_ids(BTreeSet::from([43]));
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: ENCHANTMENT_MENU_TYPE_ID,
        title: "Enchanting Table".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 38];
    items[37] = item_stack(43, 3);
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
        Some(PhysicalPosition::new(712.0, 427.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 37,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (1, HashedStack::Item(hashed_item(43, 3))),
                (37, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[1].item, item_stack(43, 3));
    assert_eq!(slots[37].item, ItemStackSummary::empty());
}

#[test]
fn enchantment_table_shift_click_player_item_queues_predicted_input_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_enchantment_lapis_lazuli_item_ids(BTreeSet::from([43]));
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: ENCHANTMENT_MENU_TYPE_ID,
        title: "Enchanting Table".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 38];
    items[37] = item_stack(50, 3);
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
        Some(PhysicalPosition::new(712.0, 427.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 37,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Item(hashed_item(50, 1))),
                (37, HashedStack::Item(hashed_item(50, 2))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(50, 1));
    assert_eq!(slots[37].item, item_stack(50, 2));
}

#[test]
fn non_local_quick_move_with_unhashable_prediction_falls_back_to_server_click() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: CRAFTING_MENU_TYPE_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 46];
    items[1] = bundle_stack(42, 3, 1);
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
        Some(PhysicalPosition::new(590.0, 302.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.select_bundle_item_commands_queued, 1);
    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SelectBundleItem(SelectBundleItem {
            slot_id: 1,
            selected_item_index: -1,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 1,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[1].item, bundle_stack(42, 3, 1));
    assert_eq!(slots[10].item, ItemStackSummary::empty());
}

#[test]
fn crafting_table_shift_click_result_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_default_item_crafting_remainders(BTreeMap::new());
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: CRAFTING_MENU_TYPE_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 13,
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
        Some(PhysicalPosition::new(684.0, 320.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 13,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (1, HashedStack::Empty),
                (45, HashedStack::Item(hashed_item(90, 2))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[45].item, item_stack(90, 2));
}

#[test]
fn crafting_table_left_click_result_slot_queues_predicted_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_default_item_crafting_remainders(BTreeMap::new());
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: CRAFTING_MENU_TYPE_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 13,
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
        Some(PhysicalPosition::new(684.0, 320.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 13,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: [(1, HashedStack::Item(hashed_item(42, 1)))].into(),
            carried_item: HashedStack::Item(hashed_item(90, 1)),
        })
    );
    let inventory = world.inventory();
    let slots = &inventory.open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(90, 1));
    assert_eq!(slots[1].item, item_stack(42, 1));
    assert_eq!(inventory.cursor_item, item_stack(90, 1));
}

#[test]
fn crafting_table_result_slot_with_default_remainder_queues_predicted_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: CRAFTING_MENU_TYPE_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 13,
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
        Some(PhysicalPosition::new(684.0, 320.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 13,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: [
                (0, HashedStack::Empty),
                (1, HashedStack::Item(hashed_item(43, 1))),
            ]
            .into(),
            carried_item: HashedStack::Item(hashed_item(90, 1)),
        })
    );
    let inventory = world.inventory();
    let slots = &inventory.open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, item_stack(43, 1));
    assert_eq!(inventory.cursor_item, item_stack(90, 1));
    assert!(rx.try_recv().is_err());
}

#[test]
fn crafting_table_result_slot_with_recipe_specific_remainder_queues_server_authoritative_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_default_item_crafting_remainders(BTreeMap::new());
    world.set_recipe_specific_crafting_remainder_item_ids(BTreeSet::from([42]));
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: CRAFTING_MENU_TYPE_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 2);
    items[5] = item_stack(43, 2);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 13,
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
        Some(PhysicalPosition::new(684.0, 320.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 13,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::Empty,
        })
    );
    let inventory = world.inventory();
    let slots = &inventory.open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(90, 1));
    assert_eq!(slots[1].item, item_stack(42, 2));
    assert_eq!(slots[5].item, item_stack(43, 2));
    assert_eq!(inventory.cursor_item, ItemStackSummary::empty());
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_shift_click_player_item_queues_predicted_input_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: ANVIL_MENU_TYPE_ID,
        title: "Anvil".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[30] = item_stack(42, 3);
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
        Some(PhysicalPosition::new(560.0, 427.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 30,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Item(hashed_item(42, 3))),
                (30, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(42, 3));
    assert_eq!(slots[30].item, ItemStackSummary::empty());
}

#[test]
fn anvil_shift_click_input_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: ANVIL_MENU_TYPE_ID,
        title: "Anvil".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
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
        Some(PhysicalPosition::new(587.0, 332.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (3, HashedStack::Item(hashed_item(42, 1))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, item_stack(42, 1));
}

#[test]
fn anvil_shift_click_result_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: ANVIL_MENU_TYPE_ID,
        title: "Anvil".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    items[2] = item_stack(90, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: 0,
        value: 1,
    });
    world.apply_player_experience(PlayerExperience {
        progress: 0.0,
        level: 1,
        total: 0,
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(694.0, 332.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 2,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (2, HashedStack::Empty),
                (38, HashedStack::Item(hashed_item(90, 1))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(slots[38].item, item_stack(90, 1));
}

#[test]
fn anvil_left_click_result_slot_queues_predicted_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: ANVIL_MENU_TYPE_ID,
        title: "Anvil".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    items[2] = item_stack(90, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: 0,
        value: 1,
    });
    world.apply_player_experience(PlayerExperience {
        progress: 0.0,
        level: 1,
        total: 0,
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(694.0, 332.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 2,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: [(0, HashedStack::Empty), (2, HashedStack::Empty)].into(),
            carried_item: HashedStack::Item(hashed_item(90, 1)),
        })
    );
    let inventory = world.inventory();
    let slots = &inventory.open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(inventory.cursor_item, item_stack(90, 1));
}

#[test]
fn smithing_shift_click_input_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: SMITHING_MENU_TYPE_ID,
        title: "Smithing".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 40];
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
        Some(PhysicalPosition::new(568.0, 333.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (4, HashedStack::Item(hashed_item(42, 1))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[4].item, item_stack(42, 1));
}

#[test]
fn smithing_shift_click_result_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: SMITHING_MENU_TYPE_ID,
        title: "Smithing".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 40];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[2] = item_stack(44, 1);
    items[3] = item_stack(90, 1);
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
        Some(PhysicalPosition::new(658.0, 333.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 3,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (1, HashedStack::Empty),
                (2, HashedStack::Empty),
                (3, HashedStack::Empty),
                (39, HashedStack::Item(hashed_item(90, 1))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, ItemStackSummary::empty());
    assert_eq!(slots[39].item, item_stack(90, 1));
}

#[test]
fn smithing_left_click_result_slot_queues_predicted_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: SMITHING_MENU_TYPE_ID,
        title: "Smithing".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 40];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[2] = item_stack(44, 1);
    items[3] = item_stack(90, 1);
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
        Some(PhysicalPosition::new(658.0, 333.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 3,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: [
                (0, HashedStack::Empty),
                (1, HashedStack::Empty),
                (2, HashedStack::Empty),
                (3, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Item(hashed_item(90, 1)),
        })
    );
    let inventory = world.inventory();
    let slots = &inventory.open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, ItemStackSummary::empty());
    assert_eq!(inventory.cursor_item, item_stack(90, 1));
}

#[test]
fn smithing_shift_click_player_template_queues_predicted_input_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_recipes(UpdateRecipes {
        property_sets: vec![
            RecipePropertySetSummary {
                key: "minecraft:smithing_template".to_string(),
                item_ids: vec![42],
            },
            RecipePropertySetSummary {
                key: "minecraft:smithing_base".to_string(),
                item_ids: vec![43],
            },
            RecipePropertySetSummary {
                key: "minecraft:smithing_addition".to_string(),
                item_ids: vec![44],
            },
        ],
        stonecutter_recipes: Vec::new(),
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: SMITHING_MENU_TYPE_ID,
        title: "Smithing".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 40];
    items[31] = item_stack(42, 1);
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
        Some(PhysicalPosition::new(568.0, 427.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 31,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Item(hashed_item(42, 1))),
                (31, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(42, 1));
    assert_eq!(slots[31].item, ItemStackSummary::empty());
}

#[test]
fn merchant_shift_click_payment_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: MERCHANT_MENU_TYPE_ID,
        title: "Merchant".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 3);
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
        Some(PhysicalPosition::new(646.0, 322.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (3, HashedStack::Item(hashed_item(42, 3))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, item_stack(42, 3));
}

#[test]
fn merchant_left_click_result_slot_queues_predicted_trade_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: MERCHANT_MENU_TYPE_ID,
        title: "Merchant".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 3);
    items[2] = item_stack(99, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    assert!(world.apply_merchant_offers(merchant_offers(7, 1)));

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(730.0, 322.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 2,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: [(0, HashedStack::Item(hashed_item(42, 2)))].into(),
            carried_item: HashedStack::Item(hashed_item(99, 1)),
        })
    );
    let inventory = world.inventory();
    let slots = &inventory.open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(42, 2));
    assert_eq!(slots[2].item, item_stack(99, 1));
    assert_eq!(inventory.cursor_item, item_stack(99, 1));
}

#[test]
fn merchant_shift_click_result_slot_queues_predicted_trade_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: MERCHANT_MENU_TYPE_ID,
        title: "Merchant".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 3);
    items[2] = item_stack(99, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    assert!(world.apply_merchant_offers(merchant_offers(7, 1)));

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(730.0, 322.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 2,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Item(hashed_item(42, 2))),
                (38, HashedStack::Item(hashed_item(99, 1))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(42, 2));
    assert_eq!(slots[2].item, item_stack(99, 1));
    assert_eq!(slots[38].item, item_stack(99, 1));
}

#[test]
fn beacon_shift_click_single_payment_item_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    apply_item_tags(
        &mut world,
        vec![("minecraft:beacon_payment_items", vec![42])],
    );
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: BEACON_MENU_TYPE_ID,
        title: "Beacon".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 37];
    items[1] = item_stack(42, 1);
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
        Some(PhysicalPosition::new(569.0, 396.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 1,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Item(hashed_item(42, 1))),
                (1, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(42, 1));
    assert_eq!(slots[1].item, ItemStackSummary::empty());
}

#[test]
fn brewing_stand_shift_click_potion_item_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_default_item_max_stack_sizes(BTreeMap::from([(42, 64)]));
    world.set_brewing_potion_item_ids(BTreeSet::from([42]));
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: BREWING_STAND_MENU_TYPE_ID,
        title: "Brewing Stand".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 41];
    items[32] = item_stack(42, 3);
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
        Some(PhysicalPosition::new(560.0, 427.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 32,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Item(hashed_item(42, 1))),
                (32, HashedStack::Item(hashed_item(42, 2))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(42, 1));
    assert_eq!(slots[32].item, item_stack(42, 2));
}

#[test]
fn grindstone_shift_click_input_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: GRINDSTONE_MENU_TYPE_ID,
        title: "Grindstone".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
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
        Some(PhysicalPosition::new(609.0, 304.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (3, HashedStack::Item(hashed_item(42, 1))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, item_stack(42, 1));
}

#[test]
fn grindstone_shift_click_player_to_input_queues_server_authoritative_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: GRINDSTONE_MENU_TYPE_ID,
        title: "Grindstone".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[3] = item_stack(42, 3);
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
        Some(PhysicalPosition::new(568.0, 369.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 3,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, item_stack(42, 3));
}

#[test]
fn grindstone_shift_click_default_damageable_player_item_queues_predicted_input_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_default_damageable_item_ids(BTreeSet::from([42]));
    world.set_default_item_max_stack_sizes(BTreeMap::from([(42, 1)]));
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: GRINDSTONE_MENU_TYPE_ID,
        title: "Grindstone".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[3] = item_stack(42, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 13,
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
        Some(PhysicalPosition::new(568.0, 369.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 13,
            slot_num: 3,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Item(hashed_item(42, 1))),
                (3, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(42, 1));
    assert_eq!(slots[3].item, ItemStackSummary::empty());
}

#[test]
fn grindstone_shift_click_player_range_queues_predicted_quick_move_when_inputs_full() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: GRINDSTONE_MENU_TYPE_ID,
        title: "Grindstone".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(10, 1);
    items[1] = item_stack(11, 1);
    items[3] = item_stack(42, 3);
    items[30] = item_stack(43, 4);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 13,
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
        Some(PhysicalPosition::new(568.0, 369.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 13,
            slot_num: 3,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (3, HashedStack::Empty),
                (31, HashedStack::Item(hashed_item(42, 3))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[3].item, ItemStackSummary::empty());
    assert_eq!(slots[31].item, item_stack(42, 3));
}

#[test]
fn grindstone_shift_click_result_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: GRINDSTONE_MENU_TYPE_ID,
        title: "Grindstone".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    items[2] = item_stack(90, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 14,
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
        Some(PhysicalPosition::new(689.0, 319.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 14,
            slot_num: 2,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (2, HashedStack::Empty),
                (38, HashedStack::Item(hashed_item(90, 1))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(slots[38].item, item_stack(90, 1));
}

#[test]
fn grindstone_left_click_result_slot_queues_predicted_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: GRINDSTONE_MENU_TYPE_ID,
        title: "Grindstone".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[2] = item_stack(90, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 14,
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
        Some(PhysicalPosition::new(689.0, 319.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 14,
            slot_num: 2,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: [
                (0, HashedStack::Empty),
                (1, HashedStack::Empty),
                (2, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Item(hashed_item(90, 1)),
        })
    );
    let inventory = world.inventory();
    let slots = &inventory.open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(inventory.cursor_item, item_stack(90, 1));
}

#[test]
fn stonecutter_shift_click_input_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: STONECUTTER_MENU_TYPE_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 38];
    items[0] = item_stack(42, 3);
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
        Some(PhysicalPosition::new(580.0, 318.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (2, HashedStack::Item(hashed_item(42, 3))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[2].item, item_stack(42, 3));
}

#[test]
fn stonecutter_shift_click_valid_recipe_input_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_recipes(UpdateRecipes {
        property_sets: Vec::new(),
        stonecutter_recipes: vec![stonecutter_recipe(vec![42])],
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: STONECUTTER_MENU_TYPE_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 38];
    items[2] = item_stack(42, 3);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 13,
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
        Some(PhysicalPosition::new(568.0, 369.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 13,
            slot_num: 2,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Item(hashed_item(42, 3))),
                (2, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(42, 3));
    assert_eq!(slots[2].item, ItemStackSummary::empty());
}

#[test]
fn stonecutter_left_click_result_slot_queues_predicted_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: STONECUTTER_MENU_TYPE_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 38];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(90, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 14,
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
        Some(PhysicalPosition::new(703.0, 318.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 14,
            slot_num: 1,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: [(0, HashedStack::Empty), (1, HashedStack::Empty)].into(),
            carried_item: HashedStack::Item(hashed_item(90, 1)),
        })
    );
    let inventory = world.inventory();
    let slots = &inventory.open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(inventory.cursor_item, item_stack(90, 1));
}

#[test]
fn stonecutter_left_click_result_slot_with_remaining_input_queues_server_authoritative_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: STONECUTTER_MENU_TYPE_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 38];
    items[0] = item_stack(42, 2);
    items[1] = item_stack(90, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 14,
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
        Some(PhysicalPosition::new(703.0, 318.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 14,
            slot_num: 1,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::Empty,
        })
    );
    let inventory = world.inventory();
    let slots = &inventory.open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(42, 2));
    assert_eq!(slots[1].item, item_stack(90, 1));
    assert_eq!(inventory.cursor_item, ItemStackSummary::empty());
    assert!(rx.try_recv().is_err());
}

#[test]
fn stonecutter_shift_click_result_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: STONECUTTER_MENU_TYPE_ID,
        title: "Stonecutter".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 38];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(90, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 14,
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
        Some(PhysicalPosition::new(703.0, 318.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 14,
            slot_num: 1,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (1, HashedStack::Empty),
                (37, HashedStack::Item(hashed_item(90, 1))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[37].item, item_stack(90, 1));
}

#[test]
fn cartography_table_shift_click_result_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: CARTOGRAPHY_TABLE_MENU_TYPE_ID,
        title: "Cartography Table".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[2] = item_stack(90, 1);
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
        Some(PhysicalPosition::new(705.0, 324.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 2,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (1, HashedStack::Empty),
                (2, HashedStack::Empty),
                (38, HashedStack::Item(hashed_item(90, 1))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(slots[38].item, item_stack(90, 1));
}

#[test]
fn cartography_table_left_click_result_slot_queues_predicted_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: CARTOGRAPHY_TABLE_MENU_TYPE_ID,
        title: "Cartography Table".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[2] = item_stack(90, 1);
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
        Some(PhysicalPosition::new(705.0, 324.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 2,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: [
                (0, HashedStack::Empty),
                (1, HashedStack::Empty),
                (2, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Item(hashed_item(90, 1)),
        })
    );
    let inventory = world.inventory();
    let slots = &inventory.open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(inventory.cursor_item, item_stack(90, 1));
}

#[test]
fn cartography_table_shift_click_input_slots_queue_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: CARTOGRAPHY_TABLE_MENU_TYPE_ID,
        title: "Cartography Table".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 3);
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
        Some(PhysicalPosition::new(575.0, 300.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (3, HashedStack::Item(hashed_item(42, 1))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(575.0, 337.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 1,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (1, HashedStack::Empty),
                (4, HashedStack::Item(hashed_item(43, 3))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, item_stack(42, 1));
    assert_eq!(slots[4].item, item_stack(43, 3));
}

#[test]
fn cartography_table_shift_click_player_additional_item_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_cartography_additional_item_ids(BTreeSet::from([43, 44, 45]));
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: CARTOGRAPHY_TABLE_MENU_TYPE_ID,
        title: "Cartography Table".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[38] = item_stack(43, 2);
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
        Some(PhysicalPosition::new(712.0, 427.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 38,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (1, HashedStack::Item(hashed_item(43, 2))),
                (38, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[1].item, item_stack(43, 2));
    assert_eq!(slots[38].item, ItemStackSummary::empty());
}

#[test]
fn cartography_table_shift_click_player_map_id_item_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: CARTOGRAPHY_TABLE_MENU_TYPE_ID,
        title: "Cartography Table".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    items[3] = map_id_item_stack(42, 1, 7);
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
        Some(PhysicalPosition::new(568.0, 369.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 3,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Item(hashed_map_id_item(42, 1, 7))),
                (3, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, map_id_item_stack(42, 1, 7));
    assert_eq!(slots[3].item, ItemStackSummary::empty());
}

#[test]
fn cartography_table_shift_click_player_map_id_removed_component_item_queues_predicted_quick_move()
{
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: CARTOGRAPHY_TABLE_MENU_TYPE_ID,
        title: "Cartography Table".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 39];
    let mut map_stack = map_id_item_stack(42, 1, 7);
    map_stack
        .component_patch
        .removed_type_ids
        .push(TEST_MAX_DAMAGE_COMPONENT_ID);
    items[3] = map_stack.clone();
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
        Some(PhysicalPosition::new(568.0, 369.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 3,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (
                    0,
                    HashedStack::Item(hashed_map_id_removed_component_item(
                        42,
                        1,
                        7,
                        TEST_MAX_DAMAGE_COMPONENT_ID
                    ))
                ),
                (3, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, map_stack);
    assert_eq!(slots[3].item, ItemStackSummary::empty());
}

#[test]
fn loom_shift_click_input_slots_queue_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: LOOM_MENU_TYPE_ID,
        title: "Loom".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 40];
    items[0] = item_stack(42, 3);
    items[1] = item_stack(43, 2);
    items[2] = item_stack(44, 1);
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
        Some(PhysicalPosition::new(573.0, 311.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (4, HashedStack::Item(hashed_item(42, 3))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(593.0, 311.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 1,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (1, HashedStack::Empty),
                (5, HashedStack::Item(hashed_item(43, 2))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(583.0, 330.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 3);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 2,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (2, HashedStack::Empty),
                (6, HashedStack::Item(hashed_item(44, 1))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[2].item, ItemStackSummary::empty());
    assert_eq!(slots[4].item, item_stack(42, 3));
    assert_eq!(slots[5].item, item_stack(43, 2));
    assert_eq!(slots[6].item, item_stack(44, 1));
}

#[test]
fn loom_left_click_result_slot_queues_predicted_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: LOOM_MENU_TYPE_ID,
        title: "Loom".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 40];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[3] = item_stack(90, 1);
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
        Some(PhysicalPosition::new(703.0, 342.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 3,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: [
                (0, HashedStack::Empty),
                (1, HashedStack::Empty),
                (3, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Item(hashed_item(90, 1)),
        })
    );
    let inventory = world.inventory();
    let slots = &inventory.open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, ItemStackSummary::empty());
    assert_eq!(inventory.cursor_item, item_stack(90, 1));
}

#[test]
fn loom_shift_click_result_slot_queues_predicted_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: LOOM_MENU_TYPE_ID,
        title: "Loom".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 40];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    items[3] = item_stack(90, 1);
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
        Some(PhysicalPosition::new(703.0, 342.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 3,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (1, HashedStack::Empty),
                (3, HashedStack::Empty),
                (39, HashedStack::Item(hashed_item(90, 1))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[1].item, ItemStackSummary::empty());
    assert_eq!(slots[3].item, ItemStackSummary::empty());
    assert_eq!(slots[39].item, item_stack(90, 1));
}

#[test]
fn hopper_shift_click_queues_bidirectional_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: 16,
        title: "Hopper".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 41];
    items[0] = item_stack(42, 3);
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
        Some(PhysicalPosition::new(604.0, 314.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (40, HashedStack::Item(hashed_item(42, 3))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[40].item, item_stack(42, 3));

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(704.0, 403.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 40,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Item(hashed_item(42, 3))),
                (40, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(42, 3));
    assert_eq!(slots[40].item, ItemStackSummary::empty());
}

#[test]
fn shulker_box_shift_click_queues_bidirectional_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: 20,
        title: "Shulker Box".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 63];
    items[0] = item_stack(42, 3);
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
        Some(PhysicalPosition::new(560.0, 303.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Empty),
                (62, HashedStack::Item(hashed_item(42, 3))),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, ItemStackSummary::empty());
    assert_eq!(slots[62].item, item_stack(42, 3));

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(712.0, 427.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 62,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: [
                (0, HashedStack::Item(hashed_item(42, 3))),
                (62, HashedStack::Empty),
            ]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
    let slots = &world.inventory().open_container.as_ref().unwrap().slots;
    assert_eq!(slots[0].item, item_stack(42, 3));
    assert_eq!(slots[62].item, ItemStackSummary::empty());
}

#[test]
fn inventory_mouse_click_queues_container_zero_pickup() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: item_stack(42, 3),
    });
    assert!(world.open_local_inventory());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 0,
            slot_num: 36,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: [(36, HashedStack::Empty)].into(),
            carried_item: HashedStack::Item(HashedItemStack {
                item_id: 42,
                count: 3,
                components: HashedComponentPatch::default(),
            }),
        })
    );
}

#[test]
fn inventory_mouse_click_hashes_integer_component_patch() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: damage_item_stack(42, 1, 7),
    });
    assert!(world.open_local_inventory());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 0,
            slot_num: 36,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: [(36, HashedStack::Empty)].into(),
            carried_item: HashedStack::Item(hashed_int_component_item(
                42,
                1,
                TEST_DAMAGE_COMPONENT_ID,
                7
            )),
        })
    );
}

#[test]
fn inventory_result_with_default_remainder_queues_predicted_container_zero_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_default_item_crafting_remainders(BTreeMap::from([(42, 43)]));
    let mut items = vec![ItemStackSummary::empty(); 46];
    items[0] = item_stack(90, 1);
    items[1] = item_stack(42, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 0,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    assert!(world.open_local_inventory());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(714.0, 313.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: [
                (0, HashedStack::Empty),
                (
                    1,
                    HashedStack::Item(HashedItemStack {
                        item_id: 43,
                        count: 1,
                        components: HashedComponentPatch::default(),
                    }),
                ),
            ]
            .into(),
            carried_item: HashedStack::Item(HashedItemStack {
                item_id: 90,
                count: 1,
                components: HashedComponentPatch::default(),
            }),
        })
    );
    let inventory = world.inventory();
    assert_eq!(
        inventory.inventory_menu.slots[0].item,
        ItemStackSummary::empty()
    );
    assert_eq!(inventory.inventory_menu.slots[1].item, item_stack(43, 1));
    assert_eq!(inventory.cursor_item, item_stack(90, 1));
}

#[test]
fn creative_inventory_middle_click_queues_container_zero_clone() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    apply_instabuild_abilities(&mut world);
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: item_stack(42, 3),
    });
    assert!(world.open_local_inventory());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Middle,
        ElementState::Pressed,
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 0,
            slot_num: 36,
            button_num: 2,
            input: ContainerInput::Clone,
            changed_slots: [].into(),
            carried_item: HashedStack::Item(hashed_item(42, 64)),
        })
    );
    assert_eq!(world.inventory().cursor_item, item_stack(42, 64));
}

#[test]
fn creative_server_opened_container_middle_click_queues_clone() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    apply_instabuild_abilities(&mut world);
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: GENERIC_CONTAINER_FIRST_MENU_TYPE_ID,
        title: "Chest".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![item_stack(42, 3)],
        carried_item: ItemStackSummary::empty(),
    });

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Middle,
        ElementState::Pressed,
        Some(PhysicalPosition::new(568.0, 320.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 2,
            input: ContainerInput::Clone,
            changed_slots: [].into(),
            carried_item: HashedStack::Item(hashed_item(42, 64)),
        })
    );
    assert_eq!(world.inventory().cursor_item, item_stack(42, 64));
}

#[test]
fn inventory_middle_click_without_instabuild_is_consumed_without_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: item_stack(42, 3),
    });
    assert!(world.open_local_inventory());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Middle,
        ElementState::Pressed,
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(world.inventory().cursor_item, ItemStackSummary::empty());
    assert!(rx.try_recv().is_err());
}

#[test]
fn inventory_double_left_click_queues_container_zero_pickup_all() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_last_click_slot = Some(36);
    input.inventory_last_click_button_num = Some(0);
    input.inventory_last_click_at = Some(Instant::now() - Duration::from_millis(1));
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_cursor_item(SetCursorItem {
        item: item_stack(42, 4),
    });
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 1,
        item: item_stack(42, 3),
    });
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 9,
        item: item_stack(42, 5),
    });
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 2,
        item: item_stack(43, 7),
    });
    assert!(world.open_local_inventory());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(input.inventory_last_click_slot, Some(36));
    assert_eq!(input.inventory_last_click_button_num, Some(0));
    match rx.try_recv().unwrap() {
        NetCommand::ContainerClick(click) => {
            assert_eq!(click.container_id, 0);
            assert_eq!(click.state_id, 0);
            assert_eq!(click.slot_num, 36);
            assert_eq!(click.button_num, 0);
            assert_eq!(click.input, ContainerInput::PickupAll);
            assert_eq!(
                click.changed_slots,
                [(9, HashedStack::Empty), (37, HashedStack::Empty)].into()
            );
            assert_eq!(
                click.carried_item,
                HashedStack::Item(HashedItemStack {
                    item_id: 42,
                    count: 12,
                    components: HashedComponentPatch::default(),
                })
            );
        }
        command => panic!("expected container click command, got {command:?}"),
    }
    assert_eq!(world.inventory().cursor_item, item_stack(42, 12));
    assert_eq!(player_slot_item(&world, 1), ItemStackSummary::empty());
    assert_eq!(player_slot_item(&world, 9), ItemStackSummary::empty());
    assert_eq!(player_slot_item(&world, 2), item_stack(43, 7));
    assert!(rx.try_recv().is_err());
}

#[test]
fn inventory_double_click_requires_left_button_and_vanilla_threshold() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_last_click_slot = Some(36);
    input.inventory_last_click_button_num = Some(1);
    input.inventory_last_click_at = Some(Instant::now() - Duration::from_millis(1));
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_cursor_item(SetCursorItem {
        item: item_stack(42, 4),
    });
    assert!(world.open_local_inventory());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(rx.try_recv().is_err());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Released,
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));

    match rx.try_recv().unwrap() {
        NetCommand::ContainerClick(click) => {
            assert_eq!(click.slot_num, 36);
            assert_eq!(click.button_num, 1);
            assert_eq!(click.input, ContainerInput::Pickup);
        }
        command => panic!("expected container click command, got {command:?}"),
    }

    input.inventory_last_click_slot = Some(37);
    input.inventory_last_click_button_num = Some(0);
    input.inventory_last_click_at = Some(Instant::now() - VANILLA_DOUBLE_CLICK_THRESHOLD);

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(580.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(rx.try_recv().is_err());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Released,
        Some(PhysicalPosition::new(580.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));

    match rx.try_recv().unwrap() {
        NetCommand::ContainerClick(click) => {
            assert_eq!(click.slot_num, 37);
            assert_eq!(click.button_num, 0);
            assert_eq!(click.input, ContainerInput::Pickup);
        }
        command => panic!("expected container click command, got {command:?}"),
    }
    assert_eq!(counters.container_click_commands_queued, 2);
    assert!(rx.try_recv().is_err());
}

#[test]
fn inventory_left_drag_queues_quick_craft_sequence() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_cursor_item(SetCursorItem {
        item: item_stack(42, 8),
    });
    assert!(world.open_local_inventory());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(rx.try_recv().is_err());
    assert!(handle_inventory_cursor_moved(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(handle_inventory_cursor_moved(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(PhysicalPosition::new(580.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Released,
        Some(PhysicalPosition::new(580.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 4);
    assert_quick_craft_click(
        &mut rx,
        -999,
        0,
        [].into(),
        HashedStack::Item(hashed_item(42, 8)),
    );
    assert_quick_craft_click(
        &mut rx,
        36,
        1,
        [].into(),
        HashedStack::Item(hashed_item(42, 8)),
    );
    assert_quick_craft_click(
        &mut rx,
        37,
        1,
        [].into(),
        HashedStack::Item(hashed_item(42, 8)),
    );
    assert_quick_craft_click(
        &mut rx,
        -999,
        2,
        [
            (36, HashedStack::Item(hashed_item(42, 4))),
            (37, HashedStack::Item(hashed_item(42, 4))),
        ]
        .into(),
        HashedStack::Empty,
    );
    assert!(rx.try_recv().is_err());
    assert_eq!(world.inventory().cursor_item, ItemStackSummary::empty());
    assert_eq!(player_slot_item(&world, 0), item_stack(42, 4));
    assert_eq!(player_slot_item(&world, 1), item_stack(42, 4));
}

#[test]
fn inventory_right_drag_queues_quick_craft_one_per_slot() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_cursor_item(SetCursorItem {
        item: item_stack(42, 8),
    });
    assert!(world.open_local_inventory());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(handle_inventory_cursor_moved(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(handle_inventory_cursor_moved(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(PhysicalPosition::new(580.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Released,
        Some(PhysicalPosition::new(580.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 4);
    assert_quick_craft_click(
        &mut rx,
        -999,
        4,
        [].into(),
        HashedStack::Item(hashed_item(42, 8)),
    );
    assert_quick_craft_click(
        &mut rx,
        36,
        5,
        [].into(),
        HashedStack::Item(hashed_item(42, 8)),
    );
    assert_quick_craft_click(
        &mut rx,
        37,
        5,
        [].into(),
        HashedStack::Item(hashed_item(42, 8)),
    );
    assert_quick_craft_click(
        &mut rx,
        -999,
        6,
        [
            (36, HashedStack::Item(hashed_item(42, 1))),
            (37, HashedStack::Item(hashed_item(42, 1))),
        ]
        .into(),
        HashedStack::Item(hashed_item(42, 6)),
    );
    assert!(rx.try_recv().is_err());
    assert_eq!(world.inventory().cursor_item, item_stack(42, 6));
    assert_eq!(player_slot_item(&world, 0), item_stack(42, 1));
    assert_eq!(player_slot_item(&world, 1), item_stack(42, 1));
}

#[test]
fn inventory_quick_craft_without_slots_falls_back_to_pickup_on_release() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_cursor_item(SetCursorItem {
        item: item_stack(42, 3),
    });
    assert!(world.open_local_inventory());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(rx.try_recv().is_err());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Released,
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    match rx.try_recv().unwrap() {
        NetCommand::ContainerClick(click) => {
            assert_eq!(click.slot_num, 36);
            assert_eq!(click.button_num, 0);
            assert_eq!(click.input, ContainerInput::Pickup);
        }
        command => panic!("expected container click command, got {command:?}"),
    }
    assert_eq!(world.inventory().cursor_item, ItemStackSummary::empty());
    assert_eq!(player_slot_item(&world, 0), item_stack(42, 3));
    assert!(rx.try_recv().is_err());
}

#[test]
fn inventory_quick_craft_mismatched_release_button_cancels_drag() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_cursor_item(SetCursorItem {
        item: item_stack(42, 3),
    });
    assert!(world.open_local_inventory());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));
    assert!(handle_inventory_cursor_moved(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Released,
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(world.inventory().cursor_item, item_stack(42, 3));
    assert_eq!(player_slot_item(&world, 0), ItemStackSummary::empty());
    assert!(rx.try_recv().is_err());
}

#[test]
fn shift_inventory_slot_click_queues_container_zero_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: item_stack(42, 3),
    });
    assert!(world.open_local_inventory());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    match rx.try_recv().unwrap() {
        NetCommand::ContainerClick(click) => {
            assert_eq!(click.container_id, 0);
            assert_eq!(click.state_id, 0);
            assert_eq!(click.slot_num, 36);
            assert_eq!(click.button_num, 0);
            assert_eq!(click.input, ContainerInput::QuickMove);
        }
        command => panic!("expected container click command, got {command:?}"),
    }
}

#[test]
fn shift_server_opened_bundle_slot_click_clears_selection_before_quick_move() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: GENERIC_CONTAINER_FIRST_MENU_TYPE_ID,
        title: "Chest".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bundle_stack(42, 1, 3)],
        carried_item: ItemStackSummary::empty(),
    });
    assert!(world.apply_local_select_bundle_item(0, 1));

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(568.0, 320.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.select_bundle_item_commands_queued, 1);
    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SelectBundleItem(SelectBundleItem {
            slot_id: 0,
            selected_item_index: -1,
        })
    );
    match rx.try_recv().unwrap() {
        NetCommand::ContainerClick(click) => {
            assert_eq!(click.container_id, 7);
            assert_eq!(click.state_id, 12);
            assert_eq!(click.slot_num, 0);
            assert_eq!(click.button_num, 0);
            assert_eq!(click.input, ContainerInput::QuickMove);
            assert_eq!(click.changed_slots, [].into());
            assert_eq!(click.carried_item, HashedStack::Empty);
        }
        command => panic!("expected container click command, got {command:?}"),
    }
    assert_eq!(open_container_slot_bundle_selection(&world, 0), Some(-1));
    assert!(rx.try_recv().is_err());
}

#[test]
fn shift_inventory_outside_click_queues_pickup_not_quick_move() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_cursor_item(SetCursorItem {
        item: item_stack(42, 3),
    });
    assert!(world.open_local_inventory());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(551.0, 277.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 0,
            slot_num: -999,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: [].into(),
            carried_item: HashedStack::Empty,
        })
    );
}

#[test]
fn inventory_outside_click_with_empty_cursor_is_consumed_without_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(551.0, 277.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.container_click_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn inventory_mouse_wheel_routes_bundle_selection_for_hovered_container_zero_slot() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: bundle_stack(42, 1, 3),
    });
    assert!(world.open_local_inventory());

    assert!(handle_inventory_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, 1.0),
        Some(PhysicalPosition::new(560.0, 419.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.select_bundle_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SelectBundleItem(SelectBundleItem {
            slot_id: 36,
            selected_item_index: 2,
        })
    );
}

#[test]
fn server_opened_container_mouse_wheel_routes_bundle_selection_for_hovered_slot() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: GENERIC_CONTAINER_FIRST_MENU_TYPE_ID,
        title: "Chest".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![bundle_stack(42, 1, 3)],
        carried_item: ItemStackSummary::empty(),
    });

    assert!(handle_inventory_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, 1.0),
        Some(PhysicalPosition::new(568.0, 320.0)),
        PhysicalSize::new(1280, 720),
    ));

    assert_eq!(counters.select_bundle_item_commands_queued, 1);
    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(open_container_slot_bundle_selection(&world, 0), Some(2));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SelectBundleItem(SelectBundleItem {
            slot_id: 0,
            selected_item_index: 2,
        })
    );
    assert!(rx.try_recv().is_err());
}

fn stonecutter_recipe(item_ids: Vec<i32>) -> StonecutterSelectableRecipeSummary {
    StonecutterSelectableRecipeSummary {
        input: IngredientSummary {
            tag: None,
            item_ids,
        },
        option_display: SlotDisplaySummary {
            display_type_id: 0,
            raw_payload: Vec::new(),
            item_stack: None,
        },
    }
}

fn apply_item_tags(world: &mut WorldStore, tags: Vec<(&str, Vec<i32>)>) {
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: tags
                .into_iter()
                .map(|(tag, entries)| TagNetworkPayload {
                    tag: tag.to_string(),
                    entries,
                })
                .collect(),
        }],
    });
}

fn loom_world_with_banner_and_dye() -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: LOOM_MENU_TYPE_ID,
        title: "Loom".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ItemStackSummary::empty(); 40];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    world
}

fn add_entity_with_type(id: i32, entity_type_id: i32) -> AddEntity {
    AddEntity {
        id,
        uuid: Uuid::from_u128(id as u128),
        entity_type_id,
        position: Vec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        delta_movement: Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    }
}

fn byte_entity_data(data_id: u8, value: i8) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: 0,
        value: EntityDataValueKind::Byte(value),
    }
}

fn bool_entity_data(data_id: u8, value: bool) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: 8,
        value: EntityDataValueKind::Boolean(value),
    }
}

fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn map_id_item_stack(item_id: i32, count: i32, map_id: i32) -> ItemStackSummary {
    let mut item = item_stack(item_id, count);
    item.component_patch.added = 1;
    item.component_patch
        .added_type_ids
        .push(TEST_MAP_ID_COMPONENT_ID);
    item.component_patch.map_id = Some(map_id);
    item
}

fn damage_item_stack(item_id: i32, count: i32, damage: i32) -> ItemStackSummary {
    let mut item = item_stack(item_id, count);
    item.component_patch.added = 1;
    item.component_patch
        .added_type_ids
        .push(TEST_DAMAGE_COMPONENT_ID);
    item.component_patch.damage = Some(damage);
    item
}

fn apply_instabuild_abilities(world: &mut WorldStore) {
    world.apply_player_abilities(PlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
}

fn merchant_offers(container_id: i32, offer_count: usize) -> MerchantOffers {
    MerchantOffers {
        container_id,
        offers: (0..offer_count)
            .map(|index| MerchantOffer {
                buy_a: item_cost(42 + index as i32, 3),
                sell: item_stack(99 + index as i32, 1),
                buy_b: None,
                is_out_of_stock: false,
                uses: 1,
                max_uses: 12,
                xp: 8,
                special_price_diff: -2,
                price_multiplier: 0.05,
                demand: 6,
            })
            .collect(),
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    }
}

fn item_cost(item_id: i32, count: i32) -> ItemCostSummary {
    ItemCostSummary {
        item_id,
        count,
        component_predicate: Default::default(),
    }
}

fn bundle_stack(item_id: i32, count: i32, item_count: usize) -> ItemStackSummary {
    let mut stack = item_stack(item_id, count);
    stack.component_patch.bundle_contents_item_count = Some(item_count);
    stack
}

fn hashed_item(item_id: i32, count: i32) -> HashedItemStack {
    HashedItemStack {
        item_id,
        count,
        components: HashedComponentPatch::default(),
    }
}

fn hashed_int_component_item(
    item_id: i32,
    count: i32,
    component_type_id: i32,
    value: i32,
) -> HashedItemStack {
    let value_hash = match value {
        7 => TEST_HASH_OPS_INT_7_HASH,
        other => panic!("missing test HashOps int hash for {other}"),
    };
    HashedItemStack {
        item_id,
        count,
        components: HashedComponentPatch {
            added_components: BTreeMap::from([(component_type_id, value_hash)]),
            removed_components: BTreeSet::new(),
        },
    }
}

fn hashed_map_id_item(item_id: i32, count: i32, map_id: i32) -> HashedItemStack {
    let map_id_hash = match map_id {
        7 => TEST_MAP_ID_7_HASH,
        other => panic!("missing test HashOps map_id hash for {other}"),
    };
    HashedItemStack {
        item_id,
        count,
        components: HashedComponentPatch {
            added_components: BTreeMap::from([(TEST_MAP_ID_COMPONENT_ID, map_id_hash)]),
            removed_components: BTreeSet::new(),
        },
    }
}

fn hashed_map_id_removed_component_item(
    item_id: i32,
    count: i32,
    map_id: i32,
    component_type_id: i32,
) -> HashedItemStack {
    let map_id_hash = match map_id {
        7 => TEST_MAP_ID_7_HASH,
        other => panic!("missing test HashOps map_id hash for {other}"),
    };
    HashedItemStack {
        item_id,
        count,
        components: HashedComponentPatch {
            added_components: BTreeMap::from([(TEST_MAP_ID_COMPONENT_ID, map_id_hash)]),
            removed_components: BTreeSet::from([component_type_id]),
        },
    }
}

fn assert_quick_craft_click(
    rx: &mut mpsc::Receiver<NetCommand>,
    slot_num: i16,
    button_num: i8,
    changed_slots: BTreeMap<i16, HashedStack>,
    carried_item: HashedStack,
) {
    match rx.try_recv().unwrap() {
        NetCommand::ContainerClick(click) => {
            assert_eq!(click.container_id, 0);
            assert_eq!(click.state_id, 0);
            assert_eq!(click.slot_num, slot_num);
            assert_eq!(click.button_num, button_num);
            assert_eq!(click.input, ContainerInput::QuickCraft);
            assert_eq!(click.changed_slots, changed_slots);
            assert_eq!(click.carried_item, carried_item);
        }
        command => panic!("expected quick craft container click command, got {command:?}"),
    }
}

fn player_slot_item(world: &WorldStore, slot: i32) -> ItemStackSummary {
    world
        .inventory()
        .player_slots
        .iter()
        .find(|state| state.slot == slot)
        .map(|state| state.item.clone())
        .unwrap_or_else(ItemStackSummary::empty)
}

fn open_container_slot_bundle_selection(world: &WorldStore, slot: i16) -> Option<i32> {
    world
        .inventory()
        .open_container
        .as_ref()?
        .slots
        .iter()
        .find(|state| state.slot == slot)
        .map(|state| state.local_selected_bundle_item_index)
}
