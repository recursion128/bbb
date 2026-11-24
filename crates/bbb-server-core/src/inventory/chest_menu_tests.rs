use bbb_facts::container_inputs::ContainerInput;

use super::*;
use bbb_protocol::item_types::VANILLA_ITEM_STONE_ID;
use bbb_protocol::menu_types::vanilla_menu_type;

fn stone(count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(VANILLA_ITEM_STONE_ID),
        count,
        component_patch: Default::default(),
    }
}

fn chest_anchor() -> ContainerAnchor {
    ContainerAnchor {
        positions: vec![BlockPos { x: 1, y: 2, z: 3 }],
    }
}

fn opened_over_a_chest(container: Vec<ItemStackSummary>) -> PlayerInventoryMenu {
    let mut menu = PlayerInventoryMenu::empty();
    menu.open_menu(
        crate::inventory::test_item_resolver(),
        None,
        MenuLayout::Chest {
            rows: CHEST_MENU_ROWS,
        },
        chest_anchor(),
        container
            .into_iter()
            .map(RuntimeItemStack::from_summary)
            .collect(),
        false,
    )
    .expect("the fixture carries no catalog lookups")
    .expect("a three-row chest is an openable menu");
    menu
}

/// `ChestMenu`'s constructor (`ChestMenu.java:49-59`): `rows * 9` chest
/// slots first, then `addStandardInventorySlots`
/// (AbstractContainerMenu.java:86-91) — the three storage rows
/// `addInventoryExtendedSlots` walks as `x + (y + 1) * 9` (`:78-84`),
/// then the hotbar `addInventoryHotbarSlots` walks as `x` (`:72-76`).
#[test]
fn the_chest_menu_slot_list_is_the_grid_then_the_standard_inventory() {
    for rows in [CHEST_MENU_ROWS, DOUBLE_CHEST_MENU_ROWS] {
        let layout = MenuLayout::Chest { rows };
        let grid = usize::try_from(rows * 9).unwrap();
        assert_eq!(layout.slot_count(), rows * 9 + 36);
        assert_eq!(layout.container_size(), grid);
        for index in 0..grid {
            assert_eq!(
                layout.slot(i16::try_from(index).unwrap()),
                Some(MenuSlot::Container(index)),
            );
        }
        for row in 0..3_usize {
            for column in 0..9_usize {
                let slot = grid + row * 9 + column;
                assert_eq!(
                    layout.slot(i16::try_from(slot).unwrap()),
                    Some(MenuSlot::Inventory(column + (row + 1) * 9)),
                );
            }
        }
        for column in 0..9_usize {
            let slot = grid + 27 + column;
            assert_eq!(
                layout.slot(i16::try_from(slot).unwrap()),
                Some(MenuSlot::Inventory(column)),
            );
        }
        assert_eq!(layout.slot(layout.slot_count()), None);
        assert_eq!(layout.slot(-1), None);
    }
}

/// The two chest row counts are the two `ChestMenu` factories that take a
/// backing `Container` (`ChestMenu.java:41-47`), and their `MenuType`s are
/// resolved through the generated `BuiltInRegistries.MENU` catalog rather
/// than written down here.
#[test]
fn the_chest_layouts_name_the_menu_types_their_factories_do() {
    assert_eq!(
        MenuLayout::Generic3x3
            .menu_type_id()
            .and_then(vanilla_menu_type)
            .map(|entry| entry.resource_id),
        Some("minecraft:generic_3x3"),
    );
    assert_eq!(
        MenuLayout::ShulkerBox
            .menu_type_id()
            .and_then(vanilla_menu_type)
            .map(|entry| entry.resource_id),
        Some("minecraft:shulker_box"),
    );
    assert_eq!(
        MenuLayout::Lectern
            .menu_type_id()
            .and_then(vanilla_menu_type)
            .map(|entry| entry.resource_id),
        Some("minecraft:lectern"),
    );
    assert_eq!(MenuLayout::PlayerInventory.menu_type_id(), None);
    assert_eq!(
        MenuLayout::Chest {
            rows: CHEST_MENU_ROWS
        }
        .menu_type_id()
        .and_then(vanilla_menu_type)
        .map(|entry| entry.resource_id),
        Some("minecraft:generic_9x3"),
    );
    assert_eq!(
        MenuLayout::Chest {
            rows: DOUBLE_CHEST_MENU_ROWS
        }
        .menu_type_id()
        .and_then(vanilla_menu_type)
        .map(|entry| entry.resource_id),
        Some("minecraft:generic_9x6"),
    );
}

/// `ServerPlayer.nextContainerCounter` (`ServerPlayer.java:1314-1316`):
/// `containerCounter % 100 + 1`, so the ids run 1..=100 and wrap to 1 —
/// never to zero, which `InventoryMenu` holds (`InventoryMenu.java:49`).
#[test]
fn the_container_counter_never_hands_out_the_inventory_menus_id() {
    let mut menu = PlayerInventoryMenu::empty();
    assert_eq!(menu.container_id(), MENU_CONTAINER_ID);
    let mut seen = Vec::new();
    for _ in 0..101 {
        seen.push(menu.next_container_counter());
    }
    assert_eq!(seen[0], 1);
    assert_eq!(seen[99], 100);
    assert_eq!(seen[100], 1);
    assert!(seen.iter().all(|id| (1..=100).contains(id)));
}

/// `ServerPlayer.openMenu` (`ServerPlayer.java:1319-1342`) followed by
/// `doCloseContainer` (`:1405-1409`): the chest menu becomes the one
/// clicks address, and closing puts `inventoryMenu` back.
#[test]
fn opening_a_chest_suspends_the_inventory_menu_and_closing_puts_it_back() {
    let mut menu = PlayerInventoryMenu::empty();
    menu.crafting_grid[0] = RuntimeItemStack::from_summary(stone(7));
    menu.open_menu(
        crate::inventory::test_item_resolver(),
        None,
        MenuLayout::Chest {
            rows: CHEST_MENU_ROWS,
        },
        chest_anchor(),
        vec![RuntimeItemStack::from_summary(ItemStackSummary::empty()); CHEST_CONTAINER_SIZE],
        false,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        menu.crafting_grid[0],
        RuntimeItemStack::from_summary(stone(7))
    );
    assert!(menu.inventory[0].item_id().is_none());
    assert_eq!(menu.container_id(), 1);
    assert_eq!(
        menu.open_container_anchor(),
        Some(&chest_anchor()),
        "the menu remembers which block entity it was built over"
    );
    // The chest menu addresses 63 slots; the inventory menu's 46th is now
    // past the end.
    assert!(menu.is_valid_slot_index(62));
    assert!(!menu.is_valid_slot_index(63));

    // `AbstractContainerMenu.removed` (:588-596) hands the cursor back,
    // and `ChestMenu.removed` (`ChestMenu.java:100-103`) adds only
    // `stopOpen` — the crafting grid is not returned by a chest close.
    menu.menu.carried = RuntimeItemStack::from_summary(stone(5));
    let closed = menu
        .do_close_container(crate::inventory::test_item_resolver(), false)
        .unwrap();
    assert_eq!(closed.filled, vec![0]);
    assert_eq!(closed.closed_container, Some(chest_anchor()));
    assert_eq!(menu.inventory[0], RuntimeItemStack::from_summary(stone(5)));
    assert_eq!(
        menu.crafting_grid[0],
        RuntimeItemStack::from_summary(stone(7)),
        "`ChestMenu.removed` does not run `InventoryMenu.removed`'s clearContainer"
    );
    assert_eq!(menu.container_id(), MENU_CONTAINER_ID);
    assert!(menu.is_valid_slot_index(45));
    assert!(!menu.is_valid_slot_index(MENU_SLOT_COUNT));
    assert_eq!(menu.open_container_anchor(), None);
    assert!(menu.container_items().is_empty());
}

/// `ChestMenu.quickMoveStack` (`ChestMenu.java:75-97`): a chest slot walks
/// the player's rows *backwards* (`:82`, the `true` argument), so a
/// shift-click out of a chest fills the hotbar before the storage rows.
#[test]
fn a_quick_move_out_of_a_chest_fills_the_hotbar_first() {
    let mut container = vec![ItemStackSummary::empty(); CHEST_CONTAINER_SIZE];
    container[0] = stone(64);
    let mut menu = opened_over_a_chest(container);
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        0,
        0,
        ContainerInput::QuickMove,
        false,
    )
    .unwrap();
    assert_eq!(
        menu.container_items()[0],
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );
    // `moveItemStackTo(stack, 27, 63, true)` walks from 62 down, and slot
    // 62 is the last hotbar slot, i.e. `Inventory` slot 8.
    assert_eq!(menu.inventory[8], RuntimeItemStack::from_summary(stone(64)));
}

/// The other half of `:85`: a player slot walks the chest grid forwards,
/// so a shift-click into a chest fills its first free slot.
#[test]
fn a_quick_move_into_a_chest_fills_its_first_slot() {
    let mut menu = opened_over_a_chest(vec![ItemStackSummary::empty(); CHEST_CONTAINER_SIZE]);
    menu.inventory[0] = RuntimeItemStack::from_summary(stone(64));
    // Slot 27+27 is the first hotbar slot of a three-row chest menu.
    menu.clicked(
        crate::inventory::test_item_resolver(),
        None,
        54,
        0,
        ContainerInput::QuickMove,
        false,
    )
    .unwrap();
    assert_eq!(
        menu.container_items()[0],
        RuntimeItemStack::from_summary(stone(64))
    );
    assert_eq!(
        menu.inventory[0],
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );

    for (name, value, merges) in [
        (
            "minecraft:rarity",
            bbb_protocol::NbtSummaryValue::String("common".into()),
            true,
        ),
        (
            "minecraft:rarity",
            bbb_protocol::NbtSummaryValue::String("uncommon".into()),
            false,
        ),
        (
            "!minecraft:rarity",
            bbb_protocol::NbtSummaryValue::Compound(vec![]),
            false,
        ),
    ] {
        use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
        let mut menu = opened_over_a_chest(vec![ItemStackSummary::empty(); CHEST_CONTAINER_SIZE]);
        let stored = RuntimeItemStack::try_read(
            &NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "id".into(),
                    value: NbtSummaryValue::String("minecraft:stone".into()),
                },
                NbtSummaryEntry {
                    name: "count".into(),
                    value: NbtSummaryValue::Int(60),
                },
                NbtSummaryEntry {
                    name: "components".into(),
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: name.into(),
                        value,
                    }]),
                },
            ]),
            crate::inventory::test_item_resolver(),
        )
        .unwrap();
        menu.set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            0,
            stored.clone(),
        )
        .unwrap();
        menu.inventory[0] = RuntimeItemStack::try_read(
            &NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "id".into(),
                    value: NbtSummaryValue::String("minecraft:stone".into()),
                },
                NbtSummaryEntry {
                    name: "count".into(),
                    value: NbtSummaryValue::Int(4),
                },
            ]),
            crate::inventory::test_item_resolver(),
        )
        .unwrap();
        menu.clicked(
            crate::inventory::test_item_resolver(),
            None,
            54,
            0,
            ContainerInput::QuickMove,
            false,
        )
        .unwrap();
        assert!(menu.inventory[0].is_empty());
        assert_eq!(
            menu.container_items()[0].count(),
            if merges { 64 } else { 60 }
        );
        assert!(menu.container_items()[0]
            .same_item_same_effective_components(&stored, crate::inventory::test_item_resolver())
            .unwrap());
        if merges {
            assert!(menu.container_items()[1].is_empty());
        } else {
            assert_eq!(menu.container_items()[1].count(), 4);
            assert!(menu.container_items()[1]
                .same_item_same_effective_components(
                    &RuntimeItemStack::from_summary(stone(1)),
                    crate::inventory::test_item_resolver()
                )
                .unwrap());
        }
    }
}

/// `ShulkerBoxSlot.mayPlace` (`ShulkerBoxSlot.java:12-14`) refuses a nested
/// shulker-box item and still accepts everything else.
#[test]
fn a_shulker_box_slot_refuses_a_nested_shulker_box() {
    let mut menu = PlayerInventoryMenu::empty();
    menu.open_menu(
        crate::inventory::test_item_resolver(),
        None,
        MenuLayout::ShulkerBox,
        chest_anchor(),
        vec![RuntimeItemStack::from_summary(ItemStackSummary::empty()); CHEST_CONTAINER_SIZE],
        false,
    )
    .expect("the fixture carries no catalog lookups")
    .expect("a shulker box is an openable menu");
    menu.menu.carried = RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_SHULKER_BOX_ID),
        count: 1,
        component_patch: Default::default(),
    });
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        0,
        0,
        ContainerInput::Pickup,
        false,
    )
    .unwrap();
    assert!(menu.container_items()[0].item_id().is_none());
    assert_eq!(
        menu.menu.carried.item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_SHULKER_BOX_ID)
    );
    menu.menu.carried = RuntimeItemStack::from_summary(stone(1));
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        0,
        0,
        ContainerInput::Pickup,
        false,
    )
    .unwrap();
    assert_eq!(
        menu.container_items()[0],
        RuntimeItemStack::from_summary(stone(1))
    );
    assert!(menu.menu.carried.item_id().is_none());
}

/// `AbstractContainerMenu.doClick`'s `PICKUP` branch runs unchanged over a
/// container slot, which is the whole point of the generalisation: there
/// is one `doClick`, and the layout decides where a slot writes.
#[test]
fn a_pickup_moves_a_stack_between_a_chest_slot_and_the_cursor() {
    let mut container = vec![ItemStackSummary::empty(); CHEST_CONTAINER_SIZE];
    container[4] = stone(64);
    let mut menu = opened_over_a_chest(container);
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        4,
        0,
        ContainerInput::Pickup,
        false,
    )
    .unwrap();
    assert_eq!(menu.menu.carried, RuntimeItemStack::from_summary(stone(64)));
    assert_eq!(
        menu.container_items()[4],
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        5,
        1,
        ContainerInput::Pickup,
        false,
    )
    .unwrap();
    assert_eq!(
        menu.container_items()[5],
        RuntimeItemStack::from_summary(stone(1))
    );
    assert_eq!(menu.menu.carried, RuntimeItemStack::from_summary(stone(63)));
}

/// `AbstractContainerMenu.sendAllDataToRemote` (:157-177) walks
/// `this.slots`, so the initial content of a chest menu is 63 stacks and
/// carries the chest menu's own container id.
#[test]
fn the_initial_data_of_a_chest_menu_covers_every_one_of_its_slots() {
    let mut container = vec![ItemStackSummary::empty(); CHEST_CONTAINER_SIZE];
    container[26] = stone(1);
    let mut menu = PlayerInventoryMenu::empty();
    menu.inventory[0] = RuntimeItemStack::from_summary(stone(2));
    let (opened, _) = menu
        .open_menu(
            crate::inventory::test_item_resolver(),
            None,
            MenuLayout::Chest {
                rows: CHEST_MENU_ROWS,
            },
            chest_anchor(),
            container
                .into_iter()
                .map(RuntimeItemStack::from_summary)
                .collect(),
            false,
        )
        .unwrap()
        .unwrap();
    let MenuSync::Content {
        container_id,
        state_id,
        items,
        ..
    } = opened.initial
    else {
        panic!("initMenu sends the whole menu");
    };
    assert_eq!(container_id, opened.container_id);
    assert_eq!(state_id, 1);
    assert_eq!(items.len(), 63);
    assert_eq!(items[26], stone(1));
    // Slot 54 is the first hotbar slot, i.e. `Inventory` slot 0.
    assert_eq!(items[54], stone(2));
}

/// `handleSetCreativeModeSlot` (:2013-2015) names
/// `ServerPlayer.inventoryMenu`, so a creative write lands in the player's
/// own menu and its slot numbering even while a chest is open.
#[test]
fn a_creative_write_addresses_the_inventory_menu_while_a_chest_is_open() {
    let mut menu = PlayerInventoryMenu::empty();
    // `initInventoryMenu` (`ServerPlayer.java:545-547`) runs at join, so
    // the inventory menu's remote mirror already agrees before the chest
    // is opened; without it every unsynchronised slot would be reported.
    menu.send_all_data_to_remote().unwrap();
    menu.open_menu(
        crate::inventory::test_item_resolver(),
        None,
        MenuLayout::Chest {
            rows: CHEST_MENU_ROWS,
        },
        chest_anchor(),
        vec![RuntimeItemStack::from_summary(ItemStackSummary::empty()); CHEST_CONTAINER_SIZE],
        false,
    )
    .unwrap()
    .unwrap();
    // 36 is `InventoryMenu`'s first hotbar slot; in the chest menu the
    // same index is a player storage slot, so a write that went through
    // the open menu would land in `Inventory` slot 18 instead of 0.
    let sync = menu
        .creative_set_inventory_menu_slot(
            crate::inventory::test_item_resolver(),
            MENU_USE_ROW_SLOT_START,
            &RuntimeItemStack::from_summary(stone(64).clone()),
        )
        .unwrap();
    assert_eq!(menu.inventory[0], RuntimeItemStack::from_summary(stone(64)));
    assert_eq!(
        menu.inventory[18],
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );
    assert!(
        sync.is_empty(),
        "the write and its mirror move together, so nothing is republished"
    );
    assert_eq!(
        menu.container_id(),
        1,
        "the chest menu is still the open one"
    );
}
