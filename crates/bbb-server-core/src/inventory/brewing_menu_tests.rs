use bbb_facts::container_inputs::ContainerInput;
use bbb_protocol::item_types::{
    VANILLA_ITEM_BLAZE_POWDER_ID, VANILLA_ITEM_DIRT_ID, VANILLA_ITEM_GLASS_BOTTLE_ID,
    VANILLA_ITEM_NETHER_WART_ID, VANILLA_ITEM_POTION_ID, VANILLA_ITEM_STONE_ID,
};
use bbb_protocol::menu_types::vanilla_menu_type;

use std::collections::BTreeSet;
use std::sync::Arc;

use super::*;

fn stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn brewing_anchor() -> ContainerAnchor {
    ContainerAnchor {
        positions: vec![BlockPos { x: 1, y: 2, z: 3 }],
    }
}

/// The `minecraft:brewing_fuel` members `BrewingStandMenu`'s fuel slot admits
/// (`BrewingStandMenu.java:135-137`), attached from the live item tags.
fn opened_over_a_brewing_stand(container: Vec<ItemStackSummary>) -> PlayerInventoryMenu {
    let mut menu = PlayerInventoryMenu::empty();
    menu.open_menu(
        crate::inventory::test_item_resolver(),
        None,
        MenuLayout::BrewingStand,
        brewing_anchor(),
        container
            .into_iter()
            .map(RuntimeItemStack::from_summary)
            .collect(),
        false,
    )
    .expect("the fixture carries no catalog lookups")
    .expect("a brewing stand is an openable menu");
    menu.attach_brewing_fuel(Arc::new(BTreeSet::from([VANILLA_ITEM_BLAZE_POWDER_ID])));
    menu
}

/// `BrewingStandMenu`'s constructor (`BrewingStandMenu.java:41-55`): three
/// bottle slots, the ingredient slot and the fuel slot, then
/// `addStandardInventorySlots`.
#[test]
fn the_brewing_menu_slot_list_is_five_container_slots_then_the_standard_inventory() {
    let layout = MenuLayout::BrewingStand;
    assert_eq!(layout.slot_count(), 41);
    assert_eq!(layout.container_size(), 5);
    for index in 0..5_usize {
        assert_eq!(
            layout.slot(i16::try_from(index).unwrap()),
            Some(MenuSlot::Container(index)),
        );
    }
    for row in 0..3_usize {
        for column in 0..9_usize {
            let slot = 5 + row * 9 + column;
            assert_eq!(
                layout.slot(i16::try_from(slot).unwrap()),
                Some(MenuSlot::Inventory(column + (row + 1) * 9)),
            );
        }
    }
    for column in 0..9_usize {
        let slot = 5 + 27 + column;
        assert_eq!(
            layout.slot(i16::try_from(slot).unwrap()),
            Some(MenuSlot::Inventory(column)),
        );
    }
    assert_eq!(layout.slot(41), None);
    assert_eq!(layout.slot(-1), None);
}

/// `MenuType.BREWING_STAND` (`BrewingStandMenu.java:42`), resolved through the
/// generated `BuiltInRegistries.MENU` catalog rather than written down here.
#[test]
fn the_brewing_menu_names_its_own_menu_type() {
    assert_eq!(
        MenuLayout::BrewingStand
            .menu_type_id()
            .and_then(vanilla_menu_type)
            .map(|entry| entry.resource_id),
        Some("minecraft:brewing_stand"),
    );
}

/// `BrewingStandMenu`'s three slot kinds: `PotionSlot.mayPlaceItem`
/// (`:184-186`) takes the three potion items and an empty bottle,
/// `IngredientsSlot.mayPlace` (`:153-156`) takes what `PotionBrewing`
/// answers as an ingredient, and `FuelSlot.mayPlaceItem` (`:135-137`) takes
/// only the `minecraft:brewing_fuel` tag.
#[test]
fn a_brewing_menu_admits_potions_ingredients_and_its_fuel_tag() {
    let menu = opened_over_a_brewing_stand(vec![ItemStackSummary::empty(); 5]);
    assert!(menu
        .slot_may_place(
            0,
            &RuntimeItemStack::from_summary(stack(VANILLA_ITEM_POTION_ID, 1).clone())
        )
        .unwrap());
    assert!(menu
        .slot_may_place(
            1,
            &RuntimeItemStack::from_summary(stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 1).clone())
        )
        .unwrap());
    assert!(!menu
        .slot_may_place(
            2,
            &RuntimeItemStack::from_summary(stack(VANILLA_ITEM_STONE_ID, 1).clone())
        )
        .unwrap());
    assert!(menu
        .slot_may_place(
            3,
            &RuntimeItemStack::from_summary(stack(VANILLA_ITEM_NETHER_WART_ID, 1).clone())
        )
        .unwrap());
    assert!(
        menu.slot_may_place(
            3,
            &RuntimeItemStack::from_summary(stack(VANILLA_ITEM_STONE_ID, 1).clone())
        )
        .unwrap(),
        "stone is an `addStartMix` ingredient (`PotionBrewing.java:146`)"
    );
    assert!(!menu
        .slot_may_place(
            3,
            &RuntimeItemStack::from_summary(stack(VANILLA_ITEM_POTION_ID, 1).clone())
        )
        .unwrap());
    assert!(menu
        .slot_may_place(
            4,
            &RuntimeItemStack::from_summary(stack(VANILLA_ITEM_BLAZE_POWDER_ID, 1).clone())
        )
        .unwrap());
    assert!(!menu
        .slot_may_place(
            4,
            &RuntimeItemStack::from_summary(stack(VANILLA_ITEM_STONE_ID, 1).clone())
        )
        .unwrap());
    // `PotionSlot.getMaxStackSize` (`:169-172`) is one for every item.
    assert_eq!(
        menu.slot_max_stack_size_for(
            0,
            &RuntimeItemStack::from_summary(stack(VANILLA_ITEM_POTION_ID, 1).clone())
        )
        .unwrap(),
        1
    );
    assert_eq!(
        menu.slot_max_stack_size_for(
            4,
            &RuntimeItemStack::from_summary(stack(VANILLA_ITEM_BLAZE_POWDER_ID, 1).clone())
        )
        .unwrap(),
        max_stack_size(&stack(VANILLA_ITEM_BLAZE_POWDER_ID, 1)).unwrap()
    );
}

/// `BrewingStandMenu.quickMoveStack` (`BrewingStandMenu.java:63-115`): a
/// player stack goes to the fuel slot when the tag admits it, then to the
/// ingredient slot, then to a bottle slot, and otherwise changes player rows.
#[test]
fn a_quick_move_routes_a_brewing_menu_by_the_admission_sets() {
    // Fuel: menu slot 32 is the first hotbar slot, `Inventory` slot 0.
    let mut menu = opened_over_a_brewing_stand(vec![ItemStackSummary::empty(); 5]);
    menu.inventory[0] = RuntimeItemStack::from_summary(stack(VANILLA_ITEM_BLAZE_POWDER_ID, 64));
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        32,
        0,
        ContainerInput::QuickMove,
        false,
    )
    .unwrap();
    assert_eq!(
        menu.container_items()[4],
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_BLAZE_POWDER_ID, 64))
    );
    assert!(menu.inventory[0].item_id().is_none());

    // Ingredient: the same slot id with nether wart reaches slot 3.
    let mut menu = opened_over_a_brewing_stand(vec![ItemStackSummary::empty(); 5]);
    menu.inventory[0] = RuntimeItemStack::from_summary(stack(VANILLA_ITEM_NETHER_WART_ID, 64));
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        32,
        0,
        ContainerInput::QuickMove,
        false,
    )
    .unwrap();
    assert_eq!(
        menu.container_items()[3],
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_NETHER_WART_ID, 64))
    );

    // Bottle: a water potion reaches the first bottle slot.
    let mut menu = opened_over_a_brewing_stand(vec![ItemStackSummary::empty(); 5]);
    menu.inventory[0] = RuntimeItemStack::from_summary(stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 1));
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        32,
        0,
        ContainerInput::QuickMove,
        false,
    )
    .unwrap();
    assert_eq!(
        menu.container_items()[0],
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 1))
    );

    // Neither: a player stack changes player rows, storage first. Dirt is not
    // an ingredient, a potion or brewing fuel, where stone is an
    // `addStartMix` ingredient.
    let mut menu = opened_over_a_brewing_stand(vec![ItemStackSummary::empty(); 5]);
    menu.inventory[0] = RuntimeItemStack::from_summary(stack(VANILLA_ITEM_DIRT_ID, 64));
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        32,
        0,
        ContainerInput::QuickMove,
        false,
    )
    .unwrap();
    assert_eq!(
        menu.inventory[9],
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_DIRT_ID, 64))
    );
    assert!(menu.inventory[0].item_id().is_none());

    // A container slot goes to the player's rows walked backwards, so the
    // last hotbar slot takes it first.
    let mut container = vec![ItemStackSummary::empty(); 5];
    container[0] = stack(VANILLA_ITEM_POTION_ID, 1);
    let mut menu = opened_over_a_brewing_stand(container);
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
    assert_eq!(
        menu.inventory[8],
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_POTION_ID, 1))
    );
}
