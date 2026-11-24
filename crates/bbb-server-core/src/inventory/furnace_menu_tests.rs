use bbb_facts::container_inputs::ContainerInput;
use bbb_protocol::item_types::{
    VANILLA_ITEM_BUCKET_ID, VANILLA_ITEM_COAL_ID, VANILLA_ITEM_RAW_IRON_ID, VANILLA_ITEM_STONE_ID,
    VANILLA_ITEM_WATER_BUCKET_ID,
};
use bbb_protocol::menu_types::vanilla_menu_type;

use std::collections::BTreeSet;
use std::sync::Arc;

use super::*;

fn stone(count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(VANILLA_ITEM_STONE_ID),
        count,
        component_patch: Default::default(),
    }
}

fn stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn furnace_anchor() -> ContainerAnchor {
    ContainerAnchor {
        positions: vec![BlockPos { x: 1, y: 2, z: 3 }],
    }
}

/// The two admission sets `AbstractFurnaceMenu`'s constructor resolves from
/// the recipe manager (`AbstractFurnaceMenu.java:140-146`), reduced to the one
/// input and one fuel these tests need.
fn opened_over_a_furnace(container: Vec<ItemStackSummary>) -> PlayerInventoryMenu {
    let mut menu = PlayerInventoryMenu::empty();
    menu.open_menu(
        crate::inventory::test_item_resolver(),
        None,
        MenuLayout::Furnace {
            kind: FurnaceMenuKind::Furnace,
        },
        furnace_anchor(),
        container
            .into_iter()
            .map(RuntimeItemStack::from_summary)
            .collect(),
        false,
    )
    .expect("the fixture carries no catalog lookups")
    .expect("a furnace is an openable menu");
    menu.attach_furnace_admission(
        Arc::new(BTreeSet::from([VANILLA_ITEM_RAW_IRON_ID])),
        Arc::new(BTreeSet::from([VANILLA_ITEM_COAL_ID])),
    );
    menu
}

/// `AbstractFurnaceMenu`'s constructor (`AbstractFurnaceMenu.java:38-73`):
/// three container slots, then `addStandardInventorySlots` — the three
/// storage rows `addInventoryExtendedSlots` walks as `x + (y + 1) * 9`
/// (`AbstractContainerMenu.java:78-84`), then the hotbar
/// `addInventoryHotbarSlots` walks as `x` (`:72-76`).
#[test]
fn the_furnace_menu_slot_list_is_three_container_slots_then_the_standard_inventory() {
    let layout = MenuLayout::Furnace {
        kind: FurnaceMenuKind::Furnace,
    };
    assert_eq!(layout.slot_count(), 39);
    assert_eq!(layout.container_size(), 3);
    for index in 0..3_usize {
        assert_eq!(
            layout.slot(i16::try_from(index).unwrap()),
            Some(MenuSlot::Container(index)),
        );
    }
    for row in 0..3_usize {
        for column in 0..9_usize {
            let slot = 3 + row * 9 + column;
            assert_eq!(
                layout.slot(i16::try_from(slot).unwrap()),
                Some(MenuSlot::Inventory(column + (row + 1) * 9)),
            );
        }
    }
    for column in 0..9_usize {
        let slot = 3 + 27 + column;
        assert_eq!(
            layout.slot(i16::try_from(slot).unwrap()),
            Some(MenuSlot::Inventory(column)),
        );
    }
    assert_eq!(layout.slot(39), None);
    assert_eq!(layout.slot(-1), None);
}

/// `FurnaceMenu` / `SmokerMenu` / `BlastFurnaceMenu` register one `MenuType`
/// each (`FurnaceMenu.java:10-14`, `SmokerMenu.java:10-14`,
/// `BlastFurnaceMenu.java:10-14`), resolved through the generated
/// `BuiltInRegistries.MENU` catalog rather than written down here.
#[test]
fn the_three_furnace_layouts_name_their_own_menu_types() {
    for (kind, resource_id) in [
        (FurnaceMenuKind::Furnace, "minecraft:furnace"),
        (FurnaceMenuKind::Smoker, "minecraft:smoker"),
        (FurnaceMenuKind::BlastFurnace, "minecraft:blast_furnace"),
    ] {
        assert_eq!(
            MenuLayout::Furnace { kind }
                .menu_type_id()
                .and_then(vanilla_menu_type)
                .map(|entry| entry.resource_id),
            Some(resource_id),
        );
    }
}

/// `FurnaceFuelSlot.mayPlace` (`FurnaceFuelSlot.java:16-18`): a fuel or an
/// empty bucket, and nothing else. This is *not* the block entity's
/// `canPlaceItem` (`AbstractFurnaceBlockEntity.java:304-314`), which the
/// hopper path answers and which refuses a bucket beside a bucket; the menu
/// slot has no such condition, only the one-bucket stack size
/// (`FurnaceFuelSlot.java:22-25`). The result slot refuses everything
/// (`FurnaceResultSlot.mayPlace` `:21-24`) and the input slot is a plain
/// `Slot`.
#[test]
fn a_furnace_fuel_slot_takes_a_fuel_or_an_empty_bucket() {
    let menu = opened_over_a_furnace(vec![ItemStackSummary::empty(); 3]);
    assert!(menu
        .slot_may_place(
            1,
            &RuntimeItemStack::from_summary(stack(VANILLA_ITEM_COAL_ID, 1).clone())
        )
        .unwrap());
    assert!(menu
        .slot_may_place(
            1,
            &RuntimeItemStack::from_summary(stack(VANILLA_ITEM_BUCKET_ID, 1).clone())
        )
        .unwrap());
    assert!(!menu
        .slot_may_place(1, &RuntimeItemStack::from_summary(stone(1).clone()))
        .unwrap());
    assert!(
        !menu
            .slot_may_place(
                1,
                &RuntimeItemStack::from_summary(stack(VANILLA_ITEM_WATER_BUCKET_ID, 1).clone())
            )
            .unwrap(),
        "`isBucket` is the empty bucket alone (`:27-29`)"
    );
    assert!(
        !menu
            .slot_may_place(
                2,
                &RuntimeItemStack::from_summary(stack(VANILLA_ITEM_COAL_ID, 1).clone())
            )
            .unwrap(),
        "the result slot takes nothing"
    );
    assert!(menu
        .slot_may_place(0, &RuntimeItemStack::from_summary(stone(1).clone()))
        .unwrap());

    // `getMaxStackSize(ItemStack)` (`:22-25`) caps a bucket at one, so a
    // second bucket never joins the first in the slot.
    assert_eq!(
        menu.slot_max_stack_size_for(
            1,
            &RuntimeItemStack::from_summary(stack(VANILLA_ITEM_BUCKET_ID, 1).clone())
        )
        .unwrap(),
        1
    );
    assert_eq!(
        menu.slot_max_stack_size_for(
            1,
            &RuntimeItemStack::from_summary(stack(VANILLA_ITEM_COAL_ID, 1).clone())
        )
        .unwrap(),
        max_stack_size(&stack(VANILLA_ITEM_COAL_ID, 1)).unwrap(),
        "a fuel keeps its own stack size"
    );
}

/// `AbstractFurnaceMenu.quickMoveStack` (`AbstractFurnaceMenu.java:92-138`):
/// a player stack a recipe accepts goes to the input slot, a fuel to the
/// fuel slot, and anything else changes player rows; the result slot goes to
/// the player's rows.
#[test]
fn a_quick_move_routes_a_furnace_menu_by_the_two_admission_sets() {
    // Input: menu slot 30 is the first hotbar slot, `Inventory` slot 0.
    let mut menu = opened_over_a_furnace(vec![ItemStackSummary::empty(); 3]);
    menu.inventory[0] = RuntimeItemStack::from_summary(stack(VANILLA_ITEM_RAW_IRON_ID, 64));
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        30,
        0,
        ContainerInput::QuickMove,
        false,
    )
    .unwrap();
    assert_eq!(
        menu.container_items()[0],
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_RAW_IRON_ID, 64))
    );
    assert!(menu.inventory[0].item_id().is_none());

    // Fuel: the same slot id with coal reaches the fuel slot.
    let mut menu = opened_over_a_furnace(vec![ItemStackSummary::empty(); 3]);
    menu.inventory[0] = RuntimeItemStack::from_summary(stack(VANILLA_ITEM_COAL_ID, 64));
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        30,
        0,
        ContainerInput::QuickMove,
        false,
    )
    .unwrap();
    assert_eq!(
        menu.container_items()[1],
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_COAL_ID, 64))
    );

    // Neither: a player stack changes player rows, storage first.
    let mut menu = opened_over_a_furnace(vec![ItemStackSummary::empty(); 3]);
    menu.inventory[0] = RuntimeItemStack::from_summary(stone(64));
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        30,
        0,
        ContainerInput::QuickMove,
        false,
    )
    .unwrap();
    assert_eq!(menu.inventory[9], RuntimeItemStack::from_summary(stone(64)));
    assert!(menu.inventory[0].item_id().is_none());

    // Result: `moveItemStackTo(stack, 3, 39, true)` (`:98`) fills the hotbar
    // before the storage rows, so the last hotbar slot takes it first.
    let mut container = vec![ItemStackSummary::empty(); 3];
    container[2] = stone(64);
    let mut menu = opened_over_a_furnace(container);
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        2,
        0,
        ContainerInput::QuickMove,
        false,
    )
    .unwrap();
    assert!(menu.container_items()[2].item_id().is_none());
    assert_eq!(menu.inventory[8], RuntimeItemStack::from_summary(stone(64)));
}

/// The input and fuel slots are not destinations for a quick move out of
/// themselves (`:100`, `:105-109`): a stack in the input slot changes player
/// rows instead of bouncing between the two container slots.
#[test]
fn a_quick_move_out_of_the_input_slot_changes_player_rows() {
    let mut container = vec![ItemStackSummary::empty(); 3];
    container[0] = stack(VANILLA_ITEM_RAW_IRON_ID, 64);
    let mut menu = opened_over_a_furnace(container);
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        0,
        0,
        ContainerInput::QuickMove,
        false,
    )
    .unwrap();
    assert!(menu.container_items()[0].item_id().is_none());
    // `moveItemStackTo(stack, 3, 39, false)` (`:112`) walks the player rows
    // forwards, so the first storage slot takes it.
    assert_eq!(
        menu.inventory[9],
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_RAW_IRON_ID, 64))
    );
}

/// `FurnaceResultSlot`'s take bookkeeping (`FurnaceResultSlot.java:19-52`):
/// `remove` collects `removeCount`, `onTake` and `onQuickCraft` run
/// `checkTakeAchievements`, and a hotbar swap takes without calling `remove`.
#[test]
fn a_take_from_the_furnace_result_slot_is_reported_once() {
    use bbb_protocol::item_types::VANILLA_ITEM_IRON_INGOT_ID;

    let mut container = vec![ItemStackSummary::empty(); 3];
    container[2] = stack(VANILLA_ITEM_IRON_INGOT_ID, 1);
    let mut menu = opened_over_a_furnace(container);
    assert_eq!(menu.take_furnace_result_take(), None);
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        2,
        0,
        ContainerInput::Pickup,
        false,
    )
    .unwrap();
    assert_eq!(menu.take_furnace_result_take(), Some((1, 1)));
    assert_eq!(menu.take_furnace_result_take(), None, "the drain is a take");

    // `quickMoveStack`'s `onQuickCraft` (`:44-49`) counts what moved.
    let mut container = vec![ItemStackSummary::empty(); 3];
    container[2] = stack(VANILLA_ITEM_IRON_INGOT_ID, 2);
    let mut menu = opened_over_a_furnace(container);
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        2,
        0,
        ContainerInput::QuickMove,
        false,
    )
    .unwrap();
    assert_eq!(menu.take_furnace_result_take(), Some((1, 2)));

    // A swap into an empty hotbar slot takes without a `remove`, so the count
    // stays zero and the take is still reported.
    let mut container = vec![ItemStackSummary::empty(); 3];
    container[2] = stack(VANILLA_ITEM_IRON_INGOT_ID, 1);
    let mut menu = opened_over_a_furnace(container);
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        2,
        0,
        ContainerInput::Swap,
        false,
    )
    .unwrap();
    assert_eq!(
        menu.inventory[0],
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_IRON_INGOT_ID, 1))
    );
    assert_eq!(menu.take_furnace_result_take(), Some((1, 0)));

    // Closing the menu drops the state with it.
    let mut container = vec![ItemStackSummary::empty(); 3];
    container[2] = stack(VANILLA_ITEM_IRON_INGOT_ID, 1);
    let mut menu = opened_over_a_furnace(container);
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        2,
        0,
        ContainerInput::Pickup,
        false,
    )
    .unwrap();
    menu.do_close_container(crate::inventory::test_item_resolver(), false)
        .unwrap();
    assert_eq!(menu.take_furnace_result_take(), None);
}

/// `ServerPlaceRecipe.java:199` / `AbstractFurnaceMenu.java:193`:
/// `testClearGrid` tests only the furnace's input slot (`slot 0`), not the
/// output slot (`slot 2`), so recipe placement still works with a full player
/// inventory when the output slot holds smelted items and slot 0 is empty.
#[test]
fn furnace_recipe_placement_checks_only_input_slot_when_inventory_is_full() {
    use bbb_protocol::item_types::VANILLA_ITEM_IRON_INGOT_ID;

    let mut container = vec![ItemStackSummary::empty(); 3];
    container[2] = stack(VANILLA_ITEM_IRON_INGOT_ID, 1);
    let mut menu = opened_over_a_furnace(container);
    let datapacks = crate::tests::datapack_config()
        .datapacks
        .expect("embedded vanilla datapacks");
    let item_tags = crate::world::ServerWorld::crafting_item_tags_of(Some(&datapacks)).unwrap();
    menu.reload_crafting_catalog(Arc::clone(&datapacks.recipes), item_tags);
    // Fill all 36 player inventory slots: slot 0 holds raw iron, the rest hold stone.
    menu.set_inventory_slot(
        0,
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_RAW_IRON_ID, 4)),
    );
    for slot in 1..INVENTORY_SIZE {
        menu.set_inventory_slot(slot, RuntimeItemStack::from_summary(stone(64)));
    }

    let (action, dropped) = menu
        .place_recipe(
            crate::inventory::test_item_resolver(),
            None,
            "minecraft:iron_ingot_from_smelting_raw_iron",
            false,
            false,
        )
        .unwrap();
    assert_eq!(
        action,
        crate::inventory::recipe_placement::PostPlaceAction::Nothing
    );
    assert_eq!(dropped.len(), 1);
    assert_eq!(dropped[0].stack.item_id(), Some(VANILLA_ITEM_IRON_INGOT_ID));
    assert_eq!(menu.slot_item(0).item_id(), Some(VANILLA_ITEM_RAW_IRON_ID));
    assert_eq!(menu.slot_item(0).count(), 1);
}
