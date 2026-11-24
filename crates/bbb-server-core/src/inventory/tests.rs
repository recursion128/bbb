use bbb_facts::container_inputs::ContainerInput;
use bbb_protocol::{
    attribute_modifier_types::{
        VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
        VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
        VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
    },
    attribute_types::{
        VANILLA_ATTRIBUTE_ARMOR_ID, VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
        VANILLA_ATTRIBUTE_ATTACK_SPEED_ID,
    },
    data_component_types::{
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
    },
    equipment_slots::{
        VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID, VANILLA_EQUIPMENT_SLOT_GROUP_HAND_ID,
        VANILLA_EQUIPMENT_SLOT_GROUP_MAINHAND_ID, VANILLA_EQUIPMENT_SLOT_GROUP_OFFHAND_ID,
    },
    item_types::{
        VANILLA_ITEM_DIAMOND_AXE_ID, VANILLA_ITEM_EGG_ID, VANILLA_ITEM_MACE_ID,
        VANILLA_ITEM_MUSIC_DISC_CAT_ID, VANILLA_ITEM_NETHERITE_SPEAR_ID, VANILLA_ITEM_STONE_ID,
        VANILLA_ITEM_WOODEN_SPEAR_ID, VANILLA_ITEM_WOODEN_SWORD_ID,
    },
    packets::AttributeModifierSummary,
    vanilla_item_prototype_component_catalog, NbtSummaryEntry, NbtSummaryValue,
};

use super::item_rules::{is_damaged, is_stackable};
use super::*;

#[test]
fn the_menu_layout_matches_the_vanilla_slot_blocks() {
    // InventoryMenu.java:15-27 declares the blocks; this walks every id it
    // can address and pins where each one lands.
    assert_eq!(
        MenuLayout::PlayerInventory.slot(MENU_RESULT_SLOT),
        Some(MenuSlot::CraftingResult)
    );
    assert_eq!(
        MenuLayout::PlayerInventory.slot(1),
        Some(MenuSlot::CraftingGrid(0))
    );
    assert_eq!(
        MenuLayout::PlayerInventory.slot(4),
        Some(MenuSlot::CraftingGrid(3))
    );
    // `SLOT_IDS` runs head to feet while the inventory numbers the armor
    // feet to head, so the armor block reads backwards.
    assert_eq!(
        MenuLayout::PlayerInventory.slot(5),
        Some(MenuSlot::Inventory(39))
    );
    assert_eq!(
        MenuLayout::PlayerInventory.slot(6),
        Some(MenuSlot::Inventory(38))
    );
    assert_eq!(
        MenuLayout::PlayerInventory.slot(7),
        Some(MenuSlot::Inventory(37))
    );
    assert_eq!(
        MenuLayout::PlayerInventory.slot(8),
        Some(MenuSlot::Inventory(36))
    );
    assert_eq!(
        MenuLayout::PlayerInventory.slot(9),
        Some(MenuSlot::Inventory(9))
    );
    assert_eq!(
        MenuLayout::PlayerInventory.slot(35),
        Some(MenuSlot::Inventory(35))
    );
    // The hotbar is last in the menu and first in the inventory.
    assert_eq!(
        MenuLayout::PlayerInventory.slot(36),
        Some(MenuSlot::Inventory(0))
    );
    assert_eq!(
        MenuLayout::PlayerInventory.slot(44),
        Some(MenuSlot::Inventory(8))
    );
    assert_eq!(
        MenuLayout::PlayerInventory.slot(MENU_SHIELD_SLOT),
        Some(MenuSlot::Inventory(40))
    );
    assert_eq!(MenuLayout::PlayerInventory.slot(MENU_SLOT_COUNT), None);
    assert_eq!(MenuLayout::PlayerInventory.slot(-1), None);
}

/// `Item.canDestroyBlock` (Item.java:169-172) is two independent refusals
/// with different reaches, and this pins both against the third case they
/// have to leave alone.
///
/// The tool refusal is `instabuild`-only, so the same sword that destroys
/// nothing for a creative player destroys blocks for a survival one. The
/// `DebugStickItem` override (DebugStickItem.java:29-35) is not, so it
/// refuses either way. A pickaxe carries a tool whose
/// `can_destroy_blocks_in_creative` is the codec default `true`
/// (`ToolMaterial.applyToolProperties` passes `true`, ToolMaterial.java:48-56),
/// so it is the case that must keep working.
#[test]
fn the_creative_destroy_gate_reads_the_tool_the_stack_carries() {
    let stack = |item_id: i32| ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        ..ItemStackSummary::empty()
    };
    let sword = stack(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_SWORD_ID);
    assert!(!can_destroy_block(&sword, true).unwrap());
    assert!(can_destroy_block(&sword, false).unwrap());

    let trident = stack(bbb_protocol::item_types::VANILLA_ITEM_TRIDENT_ID);
    assert!(!can_destroy_block(&trident, true).unwrap());
    let mace = stack(bbb_protocol::item_types::VANILLA_ITEM_MACE_ID);
    assert!(!can_destroy_block(&mace, true).unwrap());

    let pickaxe = stack(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_PICKAXE_ID);
    assert!(can_destroy_block(&pickaxe, true).unwrap());
    let stone = stack(VANILLA_ITEM_STONE_ID);
    assert!(can_destroy_block(&stone, true).unwrap());
    assert!(can_destroy_block(&ItemStackSummary::empty(), true).unwrap());

    let debug_stick = stack(bbb_protocol::item_types::VANILLA_ITEM_DEBUG_STICK_ID);
    assert!(!can_destroy_block(&debug_stick, true).unwrap());
    assert!(
        !can_destroy_block(&debug_stick, false).unwrap(),
        "the override does not ask about the game type"
    );

    // A patch that removes the component takes the refusal with it
    // (`itemStack.get(TOOL)` is then null).
    let mut disarmed = sword.clone();
    disarmed
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_TOOL_TYPE_ID);
    assert!(can_destroy_block(&disarmed, true).unwrap());
}

#[test]
fn the_state_id_wraps_the_way_the_menu_does() {
    let mut menu = PlayerInventoryMenu::empty();
    assert_eq!(menu.increment_state_id(), 1);
    menu.menu.state_id = 32767;
    assert_eq!(menu.increment_state_id(), 0);
}

fn stone(count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(VANILLA_ITEM_STONE_ID),
        count,
        component_patch: Default::default(),
    }
}

fn stone_with_component(count: i32, name: &str, value: NbtSummaryValue) -> RuntimeItemStack {
    RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::String("minecraft:stone".into()),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(count),
            },
            NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: name.into(),
                    value,
                }]),
            },
        ]),
        test_item_resolver(),
    )
    .unwrap()
}

#[test]
fn merchant_result_removal_takes_the_whole_result() {
    let mut menu = PlayerInventoryMenu::empty();
    menu.container = vec![
        RuntimeItemStack::empty(),
        RuntimeItemStack::empty(),
        RuntimeItemStack::from_summary(stone(3)),
    ];
    let taken = menu.merchant_result_remove(1);
    assert_eq!(taken.count(), 3);
    assert!(menu.container[2].is_empty());
}

/// `Inventory.clearOrCountMatchingItems` (`Inventory.java:180-191`) with
/// `i -> true` and `amountToRemove == -1`.
#[test]
fn clear_all_items_empties_inventory_craft_and_carried() {
    let mut menu = PlayerInventoryMenu::empty();
    menu.inventory[0] = RuntimeItemStack::from_summary(stone(64));
    menu.inventory[5] = RuntimeItemStack::from_summary(stone(3));
    menu.crafting_grid[1] = RuntimeItemStack::from_summary(stone(2));
    menu.menu.carried = RuntimeItemStack::from_summary(stone(7));
    assert_eq!(menu.clear_all_items(None).unwrap(), 76);
    assert_eq!(menu.take_crafting_sync().len(), 1);
    assert!(menu.inventory.iter().all(|item| item.is_empty()));
    assert!(menu.crafting_grid.iter().all(|item| item.is_empty()));
    assert!(menu.menu.carried.is_empty());
    assert_eq!(menu.clear_all_items(None).unwrap(), 0);
}

#[test]
fn a_returning_stack_tops_up_before_it_takes_a_free_slot() {
    // `placeItemBackInInventory` (Inventory.java:307-324) asks
    // `getSlotWithRemainingSpace` first, which tries the held slot and the
    // offhand before scanning — so the fill order is not left to right.
    let mut menu = PlayerInventoryMenu::empty();
    menu.set_selected_slot(4);
    menu.inventory[4] = stone_with_component(
        60,
        "minecraft:rarity",
        NbtSummaryValue::String("common".into()),
    );
    menu.inventory[0] = RuntimeItemStack::from_summary(stone(64));
    let uncommon = stone_with_component(
        1,
        "minecraft:rarity",
        NbtSummaryValue::String("uncommon".into()),
    );
    let removed = stone_with_component(1, "!minecraft:rarity", NbtSummaryValue::Compound(vec![]));
    menu.inventory[1] = uncommon.clone();
    menu.inventory[2] = removed.clone();

    let mut returning = RuntimeItemStack::from_summary(stone(10));
    let filled = menu
        .place_item_back_in_inventory(
            crate::inventory::test_item_resolver(),
            &mut returning,
            false,
        )
        .unwrap();

    assert_eq!(filled, vec![4, 3]);
    assert_eq!(menu.inventory[4].count(), 64);
    assert!(menu.inventory[4]
        .same_item_same_effective_components(
            &RuntimeItemStack::from_summary(stone(1)),
            test_item_resolver(),
        )
        .unwrap());
    assert_eq!(menu.inventory[1], uncommon);
    assert_eq!(menu.inventory[2], removed);
    assert_eq!(menu.inventory[3].count(), 6);
    assert_eq!(returning, RuntimeItemStack::empty());
}

#[test]
fn a_damaged_stack_never_merges_into_another() {
    // `isStackable` is false for a damaged damageable item
    // (ItemStack.java:413-415), so `hasRemainingSpaceForItem` refuses every
    // occupied slot and `add` puts the stack down whole.
    let mut menu = PlayerInventoryMenu::empty();
    let mut damaged = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_STONE_ID),
        count: 1,
        component_patch: Default::default(),
    };
    damaged.component_patch.max_damage = Some(100);
    damaged.component_patch.damage = Some(7);
    damaged.component_patch.added_type_ids.extend([
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
    ]);
    assert!(is_damaged(&damaged).unwrap());
    assert!(!is_stackable(&damaged).unwrap());

    menu.inventory[0] = RuntimeItemStack::from_summary(damaged.clone());
    let mut returning = RuntimeItemStack::from_summary(damaged.clone());
    let filled = menu
        .place_item_back_in_inventory(
            crate::inventory::test_item_resolver(),
            &mut returning,
            false,
        )
        .unwrap();

    assert_eq!(filled, vec![1]);
    assert_eq!(menu.inventory[1], RuntimeItemStack::from_summary(damaged));
    assert_eq!(returning, RuntimeItemStack::empty());
}

#[test]
fn a_full_inventory_hands_back_what_no_slot_would_take() {
    // With no slot left, vanilla drops the remainder as an item entity
    // (Inventory.java:314-317) and stops. Creative changes nothing here —
    // the infinite-materials branch lives inside `add`, which this path
    // never reaches — so both players are handed the same leftover, which
    // is what this world has instead of an item entity.
    let mut full = PlayerInventoryMenu::empty();
    for slot in 0..INVENTORY_SIZE {
        full.inventory[slot] = RuntimeItemStack::from_summary(stone(64));
    }
    for has_infinite_materials in [false, true] {
        let mut leftover = RuntimeItemStack::from_summary(stone(5));
        assert!(full
            .place_item_back_in_inventory(
                crate::inventory::test_item_resolver(),
                &mut leftover,
                has_infinite_materials
            )
            .unwrap()
            .is_empty());
        assert_eq!(
            leftover,
            RuntimeItemStack::from_summary(stone(5)),
            "creative {has_infinite_materials}"
        );
    }
}

#[test]
fn a_slot_with_no_room_swallows_the_stack_only_for_infinite_materials() {
    // `add` (Inventory.java:270-284) reports failure when nothing moved,
    // except for a player whose materials are infinite: there the stack is
    // zeroed and the call succeeds, so the overflow ceases to exist rather
    // than piling up.
    let mut menu = PlayerInventoryMenu::empty();
    menu.inventory[0] = RuntimeItemStack::from_summary(stone(64));

    let mut survival = RuntimeItemStack::from_summary(stone(5));
    assert!(!menu.add(0, &mut survival, false).unwrap());
    assert_eq!(survival, RuntimeItemStack::from_summary(stone(5)));

    let mut creative = RuntimeItemStack::from_summary(stone(5));
    assert!(menu.add(0, &mut creative, true).unwrap());
    assert_eq!(creative, RuntimeItemStack::empty());
    assert_eq!(menu.inventory[0], RuntimeItemStack::from_summary(stone(64)));
}

#[test]
fn closing_the_menu_returns_the_crafting_grid_and_clears_the_result() {
    // `InventoryMenu.removed` (InventoryMenu.java:88-94) clears the result
    // outright and hands the grid back through `clearContainer`.
    let mut menu = PlayerInventoryMenu::empty();
    menu.set_by_player(
        crate::inventory::test_item_resolver(),
        None,
        0,
        RuntimeItemStack::from_summary(stone(1)),
    )
    .unwrap();
    menu.set_by_player(
        crate::inventory::test_item_resolver(),
        None,
        1,
        RuntimeItemStack::from_summary(stone(3)),
    )
    .unwrap();
    menu.set_by_player(
        crate::inventory::test_item_resolver(),
        None,
        3,
        RuntimeItemStack::from_summary(stone(2)),
    )
    .unwrap();

    let filled = menu
        .do_close_container(crate::inventory::test_item_resolver(), true)
        .unwrap()
        .filled;

    // Both grid stacks land in the held slot: the first opens it, and
    // `getSlotWithRemainingSpace` tries that slot first, so the second
    // tops it up rather than opening another.
    assert_eq!(filled, vec![0, 0]);
    assert_eq!(menu.inventory[0], RuntimeItemStack::from_summary(stone(5)));
    assert_eq!(
        menu.inventory[1],
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );
    assert_eq!(menu.menu_items().unwrap()[0], ItemStackSummary::empty());
    assert!(menu.menu_items().unwrap()[1..5]
        .iter()
        .all(|item| *item == ItemStackSummary::empty()));
}

#[test]
fn a_discs_song_comes_from_its_prototype() {
    let disc = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_MUSIC_DISC_CAT_ID),
        count: 1,
        ..ItemStackSummary::empty()
    };
    assert_eq!(
        jukebox_playable_song(
            &RuntimeItemStack::from_summary(disc.clone()),
            test_item_resolver()
        )
        .unwrap()
        .as_deref(),
        Some("minecraft:cat")
    );

    let stone = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_STONE_ID),
        count: 1,
        ..ItemStackSummary::empty()
    };
    assert_eq!(
        jukebox_playable_song(&RuntimeItemStack::from_summary(stone), test_item_resolver())
            .unwrap(),
        None
    );
    assert_eq!(
        jukebox_playable_song(&RuntimeItemStack::empty(), test_item_resolver()).unwrap(),
        None
    );

    let mut removed = disc.clone();
    removed
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID);
    assert_eq!(
        jukebox_playable_song(
            &RuntimeItemStack::from_summary(removed),
            test_item_resolver()
        )
        .unwrap(),
        None
    );
}

#[test]
fn max_stack_size_prefers_the_stack_over_its_prototype() {
    let stone = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_STONE_ID),
        count: 1,
        component_patch: Default::default(),
    };
    assert_eq!(max_stack_size(&stone).unwrap(), 64);

    let mut capped = stone.clone();
    capped.component_patch.max_stack_size = Some(16);
    assert_eq!(max_stack_size(&capped).unwrap(), 16);

    let mut removed = stone.clone();
    removed
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID);
    assert_eq!(max_stack_size(&removed).unwrap(), 1);

    assert_eq!(max_stack_size(&ItemStackSummary::empty()).unwrap(), 1);
}

#[test]
fn minimum_attack_charge_reads_the_effective_patched_component() {
    let stack = |item_id: i32| ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        ..ItemStackSummary::empty()
    };

    let spear = stack(VANILLA_ITEM_WOODEN_SPEAR_ID);
    assert_eq!(minimum_attack_charge(&spear).unwrap(), 1.0);

    let mut patched = stack(VANILLA_ITEM_STONE_ID);
    patched
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID);
    patched.component_patch.minimum_attack_charge_bits = Some(0.75_f32.to_bits());
    assert_eq!(minimum_attack_charge(&patched).unwrap(), 0.75);

    let mut removed = spear;
    removed
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID);
    assert_eq!(minimum_attack_charge(&removed).unwrap(), 0.0);
}

fn one_item(item_id: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        ..ItemStackSummary::empty()
    }
}

fn attribute_modifier(
    attribute_id: i32,
    modifier_id: &str,
    amount: f64,
    operation_id: i32,
    slot_id: i32,
) -> AttributeModifierSummary {
    AttributeModifierSummary {
        attribute_id,
        modifier_id: modifier_id.to_owned(),
        amount_bits: amount.to_bits(),
        operation_id,
        slot_id,
        display_id: 0,
        display_text: None,
        display_text_styled: None,
        display_text_nbt: None,
    }
}

#[test]
fn main_hand_attack_attributes_read_weapon_prototypes() {
    let cases = [
        (VANILLA_ITEM_WOODEN_SWORD_ID, 4.0, 1.599_999_904_632_568_4),
        (VANILLA_ITEM_DIAMOND_AXE_ID, 9.0, 1.0),
        (VANILLA_ITEM_MACE_ID, 6.0, 0.599_999_904_632_568_4),
        (VANILLA_ITEM_WOODEN_SPEAR_ID, 1.0, 1.538_461_565_971_374_5),
    ];
    for (item_id, damage, speed) in cases {
        let actual = main_hand_attack_attributes(&one_item(item_id), 1.0, 4.0).unwrap();
        assert_eq!(actual.damage, damage);
        assert_eq!(actual.speed, speed);
    }
}

#[test]
fn main_hand_attack_attributes_resolve_patch_precedence_slots_and_operations() {
    let mut patched = one_item(VANILLA_ITEM_WOODEN_SWORD_ID);
    patched
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID);
    patched.component_patch.attribute_modifiers = vec![
        attribute_modifier(
            VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
            "bbb:add",
            3.0,
            VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            VANILLA_EQUIPMENT_SLOT_GROUP_MAINHAND_ID,
        ),
        attribute_modifier(
            VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
            "bbb:base",
            0.5,
            VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
            VANILLA_EQUIPMENT_SLOT_GROUP_HAND_ID,
        ),
        attribute_modifier(
            VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
            "bbb:total",
            0.25,
            VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
            VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
        ),
        attribute_modifier(
            VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
            "bbb:offhand",
            100.0,
            VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            VANILLA_EQUIPMENT_SLOT_GROUP_OFFHAND_ID,
        ),
        attribute_modifier(
            VANILLA_ATTRIBUTE_ATTACK_SPEED_ID,
            "bbb:speed",
            -2.0,
            VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            VANILLA_EQUIPMENT_SLOT_GROUP_MAINHAND_ID,
        ),
        attribute_modifier(
            VANILLA_ATTRIBUTE_ARMOR_ID,
            "bbb:armor",
            100.0,
            VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            VANILLA_EQUIPMENT_SLOT_GROUP_MAINHAND_ID,
        ),
    ];

    assert_eq!(
        main_hand_attack_attributes(&patched, 1.0, 4.0).unwrap(),
        MainHandAttackAttributes {
            damage: 7.5,
            speed: 2.0,
        }
    );

    let mut removed = one_item(VANILLA_ITEM_WOODEN_SWORD_ID);
    removed
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID);
    assert_eq!(
        main_hand_attack_attributes(&removed, 1.0, 4.0).unwrap(),
        MainHandAttackAttributes {
            damage: 1.0,
            speed: 4.0,
        }
    );
}

#[test]
fn main_hand_attack_damage_type_resolves_spears_and_patch_precedence() {
    for spear in [
        VANILLA_ITEM_WOODEN_SPEAR_ID,
        VANILLA_ITEM_NETHERITE_SPEAR_ID,
    ] {
        assert_eq!(
            main_hand_attack_damage_type(&one_item(spear)).unwrap(),
            MainHandAttackDamageType::ResourceId("minecraft:spear")
        );
    }

    let mut patched = one_item(VANILLA_ITEM_WOODEN_SWORD_ID);
    patched
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID);
    patched.component_patch.damage_type_id = Some(300);
    assert_eq!(
        main_hand_attack_damage_type(&patched).unwrap(),
        MainHandAttackDamageType::RegistryId(300)
    );

    let mut removed = one_item(VANILLA_ITEM_WOODEN_SPEAR_ID);
    removed
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID);
    assert_eq!(
        main_hand_attack_damage_type(&removed).unwrap(),
        MainHandAttackDamageType::PlayerAttack
    );
}

/// The menu id of the first hotbar slot, which is where the fixture's own
/// stack sits.
const FIRST_HOTBAR: i16 = MENU_USE_ROW_SLOT_START;

fn helmet(count: i32) -> ItemStackSummary {
    let catalog = vanilla_item_prototype_component_catalog().unwrap();
    let item_id = catalog
        .item_resource_ids()
        .iter()
        .position(|id| id == "minecraft:iron_helmet")
        .expect("the generated catalog carries every item");
    ItemStackSummary {
        item_id: Some(i32::try_from(item_id).unwrap()),
        count,
        component_patch: Default::default(),
    }
}

#[test]
fn a_primary_pickup_takes_a_whole_stack_and_a_secondary_one_takes_half() {
    // `doClick`'s PICKUP branch (:437-467): the primary button takes
    // everything and the secondary takes the rounded-up half.
    let mut menu = PlayerInventoryMenu::fixture();
    assert!(menu
        .clicked(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            FIRST_HOTBAR,
            0,
            ContainerInput::Pickup,
            true
        )
        .unwrap()
        .is_empty());
    assert_eq!(menu.menu.carried, RuntimeItemStack::from_summary(stone(64)));
    assert_eq!(
        *menu.slot_item(FIRST_HOTBAR),
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );

    // Putting it back with the secondary button places exactly one.
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        FIRST_HOTBAR,
        1,
        ContainerInput::Pickup,
        true,
    )
    .unwrap();
    assert_eq!(menu.menu.carried, RuntimeItemStack::from_summary(stone(63)));
    assert_eq!(
        *menu.slot_item(FIRST_HOTBAR),
        RuntimeItemStack::from_summary(stone(1))
    );

    // And taking half of an odd stack rounds up.
    menu.menu.carried = RuntimeItemStack::from_summary(ItemStackSummary::empty());
    menu.set_by_player(
        crate::inventory::test_item_resolver(),
        None,
        FIRST_HOTBAR,
        RuntimeItemStack::from_summary(stone(7)),
    )
    .unwrap();
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        FIRST_HOTBAR,
        1,
        ContainerInput::Pickup,
        true,
    )
    .unwrap();
    assert_eq!(menu.menu.carried, RuntimeItemStack::from_summary(stone(4)));
    assert_eq!(
        *menu.slot_item(FIRST_HOTBAR),
        RuntimeItemStack::from_summary(stone(3))
    );

    for (component, value, merges) in [
        (
            "minecraft:rarity",
            NbtSummaryValue::String("common".into()),
            true,
        ),
        (
            "minecraft:rarity",
            NbtSummaryValue::String("uncommon".into()),
            false,
        ),
        (
            "!minecraft:rarity",
            NbtSummaryValue::Compound(vec![]),
            false,
        ),
    ] {
        let mut menu = PlayerInventoryMenu::empty();
        let held = stone_with_component(7, component, value);
        menu.set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            FIRST_HOTBAR,
            held.clone(),
        )
        .unwrap();
        menu.menu.carried = RuntimeItemStack::from_summary(stone(3));
        menu.clicked(
            test_item_resolver(),
            None,
            FIRST_HOTBAR,
            0,
            ContainerInput::Pickup,
            false,
        )
        .unwrap();
        if merges {
            assert_eq!(menu.slot_item(FIRST_HOTBAR).count(), 10);
            assert!(menu.menu.carried.is_empty());
            assert!(menu
                .slot_item(FIRST_HOTBAR)
                .same_item_same_effective_components(
                    &RuntimeItemStack::from_summary(stone(1)),
                    test_item_resolver(),
                )
                .unwrap());
        } else {
            assert_eq!(menu.slot_item(FIRST_HOTBAR).count(), 3);
            assert_eq!(menu.menu.carried, held);
        }
    }
}

#[test]
fn a_click_outside_every_slot_drops_what_the_cursor_holds() {
    // `doClick` (:406-416): the primary button drops the whole cursor and
    // the secondary one drops a single item off it.
    let mut menu = PlayerInventoryMenu::fixture();
    menu.menu.carried = RuntimeItemStack::from_summary(stone(3));

    let dropped = menu
        .clicked(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            SLOT_CLICKED_OUTSIDE,
            1,
            ContainerInput::Pickup,
            true,
        )
        .unwrap();
    assert_eq!(
        dropped,
        vec![DroppedMenuStack {
            stack: RuntimeItemStack::from_summary(stone(1)),
            thrown_from_hand: true
        }]
    );
    assert_eq!(menu.menu.carried, RuntimeItemStack::from_summary(stone(2)));

    let dropped = menu
        .clicked(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            SLOT_CLICKED_OUTSIDE,
            0,
            ContainerInput::Pickup,
            true,
        )
        .unwrap();
    assert_eq!(
        dropped,
        vec![DroppedMenuStack {
            stack: RuntimeItemStack::from_summary(stone(2)),
            thrown_from_hand: true
        }]
    );
    assert_eq!(
        menu.menu.carried,
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );
}

#[test]
fn only_an_equippable_item_reaches_an_armor_slot() {
    // `ArmorSlot.mayPlace` (ArmorSlot.java:45-46) asks the item's
    // `equippable` component which slot it belongs in. Stone declares
    // none, so it stays on the cursor.
    let mut menu = PlayerInventoryMenu::empty();
    let head = MENU_ARMOR_SLOT_START;
    menu.menu.carried = RuntimeItemStack::from_summary(stone(1));
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        head,
        0,
        ContainerInput::Pickup,
        true,
    )
    .unwrap();
    assert_eq!(menu.menu.carried, RuntimeItemStack::from_summary(stone(1)));
    assert_eq!(
        *menu.slot_item(head),
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );

    menu.menu.carried = RuntimeItemStack::from_summary(helmet(1));
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        head,
        0,
        ContainerInput::Pickup,
        true,
    )
    .unwrap();
    assert_eq!(
        menu.menu.carried,
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );
    assert_eq!(
        *menu.slot_item(head),
        RuntimeItemStack::from_summary(helmet(1))
    );
    // The helmet belongs in the head slot and nowhere else in the armor
    // block, so the chest slot still refuses it.
    let chest = MENU_ARMOR_SLOT_START + 1;
    menu.menu.carried = RuntimeItemStack::from_summary(helmet(1));
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        chest,
        0,
        ContainerInput::Pickup,
        true,
    )
    .unwrap();
    assert_eq!(menu.menu.carried, RuntimeItemStack::from_summary(helmet(1)));
}

#[test]
fn nothing_may_be_placed_into_the_crafting_result() {
    // `ResultSlot.mayPlace` (ResultSlot.java:25-27) refuses every stack.
    let mut menu = PlayerInventoryMenu::empty();
    menu.menu.carried = RuntimeItemStack::from_summary(stone(4));
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        MENU_RESULT_SLOT,
        0,
        ContainerInput::Pickup,
        true,
    )
    .unwrap();
    assert_eq!(menu.menu.carried, RuntimeItemStack::from_summary(stone(4)));
    assert_eq!(
        *menu.slot_item(MENU_RESULT_SLOT),
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );
}

#[test]
fn a_quick_move_from_the_hotbar_lands_in_the_storage_rows() {
    // `InventoryMenu.quickMoveStack` (InventoryMenu.java:132-139) sends a
    // hotbar stack to 9..36 and a storage stack back to the hotbar.
    let mut menu = PlayerInventoryMenu::fixture();
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        FIRST_HOTBAR,
        0,
        ContainerInput::QuickMove,
        true,
    )
    .unwrap();
    assert_eq!(
        *menu.slot_item(FIRST_HOTBAR),
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );
    assert_eq!(
        *menu.slot_item(MENU_INV_SLOT_START),
        RuntimeItemStack::from_summary(stone(64))
    );

    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        MENU_INV_SLOT_START,
        0,
        ContainerInput::QuickMove,
        true,
    )
    .unwrap();
    assert_eq!(
        *menu.slot_item(MENU_INV_SLOT_START),
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );
    assert_eq!(
        *menu.slot_item(FIRST_HOTBAR),
        RuntimeItemStack::from_summary(stone(64))
    );
}

#[test]
fn a_quick_moved_helmet_goes_to_its_own_armor_slot() {
    // The equipment branch (InventoryMenu.java:130-135) wins over the
    // plain hotbar-to-storage move when the armor slot is still empty.
    let mut menu = PlayerInventoryMenu::empty();
    menu.set_by_player(
        crate::inventory::test_item_resolver(),
        None,
        FIRST_HOTBAR,
        RuntimeItemStack::from_summary(helmet(1)),
    )
    .unwrap();
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        FIRST_HOTBAR,
        0,
        ContainerInput::QuickMove,
        true,
    )
    .unwrap();
    assert_eq!(
        *menu.slot_item(MENU_ARMOR_SLOT_START),
        RuntimeItemStack::from_summary(helmet(1))
    );
    assert_eq!(
        *menu.slot_item(FIRST_HOTBAR),
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );
}

#[test]
fn a_swap_exchanges_a_menu_slot_with_the_hotbar_button() {
    // `doClick`'s SWAP branch (:472-511) names an `Inventory` slot by
    // button number, not a menu slot.
    let mut menu = PlayerInventoryMenu::fixture();
    let storage = MENU_INV_SLOT_START;
    menu.set_by_player(
        crate::inventory::test_item_resolver(),
        None,
        storage,
        RuntimeItemStack::from_summary(stone(5)),
    )
    .unwrap();
    // Button 0 is the first hotbar slot, which holds the fixture stack.
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        storage,
        0,
        ContainerInput::Swap,
        true,
    )
    .unwrap();
    assert_eq!(
        *menu.slot_item(storage),
        RuntimeItemStack::from_summary(stone(64))
    );
    assert_eq!(
        *menu.slot_item(FIRST_HOTBAR),
        RuntimeItemStack::from_summary(stone(5))
    );
}

#[test]
fn a_swap_into_an_armor_slot_splits_what_will_not_fit() {
    // An armor slot holds one item (ArmorSlot.java:40-41), so a swap out
    // of a stack leaves the rest in the inventory.
    let mut menu = PlayerInventoryMenu::empty();
    menu.inventory[0] = RuntimeItemStack::from_summary(helmet(3));
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        MENU_ARMOR_SLOT_START,
        0,
        ContainerInput::Swap,
        true,
    )
    .unwrap();
    assert_eq!(
        *menu.slot_item(MENU_ARMOR_SLOT_START),
        RuntimeItemStack::from_summary(helmet(1))
    );
    assert_eq!(menu.inventory[0], RuntimeItemStack::from_summary(helmet(2)));
}

#[test]
fn cloning_a_slot_only_works_for_a_player_with_infinite_materials() {
    // `doClick`'s CLONE branch (:512-518) is gated on
    // `hasInfiniteMaterials` and fills the cursor to the item's maximum.
    let mut menu = PlayerInventoryMenu::empty();
    menu.set_by_player(
        crate::inventory::test_item_resolver(),
        None,
        FIRST_HOTBAR,
        RuntimeItemStack::from_summary(stone(3)),
    )
    .unwrap();

    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        FIRST_HOTBAR,
        0,
        ContainerInput::Clone,
        false,
    )
    .unwrap();
    assert_eq!(
        menu.menu.carried,
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );

    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        FIRST_HOTBAR,
        0,
        ContainerInput::Clone,
        true,
    )
    .unwrap();
    assert_eq!(menu.menu.carried, RuntimeItemStack::from_summary(stone(64)));
    // The slot is untouched: cloning copies rather than takes.
    assert_eq!(
        *menu.slot_item(FIRST_HOTBAR),
        RuntimeItemStack::from_summary(stone(3))
    );
}

#[test]
fn throwing_drops_one_item_or_the_whole_slot() {
    // `doClick`'s THROW branch (:520-544): button 0 throws one item and
    // button 1 empties the slot an item at a time.
    let mut menu = PlayerInventoryMenu::fixture();
    let dropped = menu
        .clicked(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            FIRST_HOTBAR,
            0,
            ContainerInput::Throw,
            true,
        )
        .unwrap();
    assert_eq!(
        dropped,
        vec![DroppedMenuStack {
            stack: RuntimeItemStack::from_summary(stone(1)),
            thrown_from_hand: true
        }]
    );
    assert_eq!(
        *menu.slot_item(FIRST_HOTBAR),
        RuntimeItemStack::from_summary(stone(63))
    );

    let dropped = menu
        .clicked(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            FIRST_HOTBAR,
            1,
            ContainerInput::Throw,
            true,
        )
        .unwrap();
    assert_eq!(
        dropped,
        vec![DroppedMenuStack {
            stack: RuntimeItemStack::from_summary(stone(63)),
            thrown_from_hand: true
        }]
    );
    assert_eq!(
        *menu.slot_item(FIRST_HOTBAR),
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );
}

#[test]
fn a_double_click_gathers_partial_stacks_before_full_ones() {
    // `doClick`'s PICKUP_ALL branch (:546-568) runs two passes so a full
    // stack is only broken up once the partial ones are gone.
    let mut menu = PlayerInventoryMenu::empty();
    menu.set_by_player(
        crate::inventory::test_item_resolver(),
        None,
        MENU_INV_SLOT_START,
        RuntimeItemStack::from_summary(stone(64)),
    )
    .unwrap();
    menu.set_by_player(
        crate::inventory::test_item_resolver(),
        None,
        MENU_INV_SLOT_START + 1,
        stone_with_component(
            10,
            "minecraft:rarity",
            NbtSummaryValue::String("common".into()),
        ),
    )
    .unwrap();
    menu.set_by_player(
        crate::inventory::test_item_resolver(),
        None,
        MENU_INV_SLOT_START + 2,
        RuntimeItemStack::from_summary(stone(20)),
    )
    .unwrap();
    let uncommon = stone_with_component(
        7,
        "minecraft:rarity",
        NbtSummaryValue::String("uncommon".into()),
    );
    let removed = stone_with_component(8, "!minecraft:rarity", NbtSummaryValue::Compound(vec![]));
    menu.set_by_player(
        crate::inventory::test_item_resolver(),
        None,
        MENU_INV_SLOT_START + 3,
        uncommon.clone(),
    )
    .unwrap();
    menu.set_by_player(
        crate::inventory::test_item_resolver(),
        None,
        MENU_INV_SLOT_START + 4,
        removed.clone(),
    )
    .unwrap();
    menu.menu.carried = RuntimeItemStack::from_summary(stone(1));

    menu.clicked(
        test_item_resolver(),
        None,
        MENU_INV_SLOT_START + 5,
        0,
        ContainerInput::PickupAll,
        true,
    )
    .unwrap();

    assert_eq!(menu.menu.carried.count(), 64);
    assert!(menu
        .menu
        .carried
        .same_item_same_effective_components(
            &RuntimeItemStack::from_summary(stone(1)),
            test_item_resolver(),
        )
        .unwrap());
    // The two partial stacks were drained first; the full one gave up the
    // remaining 33.
    assert_eq!(
        *menu.slot_item(MENU_INV_SLOT_START + 1),
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );
    assert_eq!(
        *menu.slot_item(MENU_INV_SLOT_START + 2),
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );
    assert_eq!(
        *menu.slot_item(MENU_INV_SLOT_START),
        RuntimeItemStack::from_summary(stone(31))
    );
    assert_eq!(*menu.slot_item(MENU_INV_SLOT_START + 3), uncommon);
    assert_eq!(*menu.slot_item(MENU_INV_SLOT_START + 4), removed);
    assert_eq!(
        menu.inventory
            .iter()
            .map(RuntimeItemStack::count)
            .sum::<i32>()
            + menu.menu.carried.count(),
        110,
    );
}

#[test]
fn a_drag_spreads_the_cursor_evenly_across_the_slots_it_passed() {
    // The QUICK_CRAFT state machine (:336-400): start, one message per
    // slot, then end. A charitable drag divides the carried stack.
    let mut menu = PlayerInventoryMenu::empty();
    menu.set_by_player(
        crate::inventory::test_item_resolver(),
        None,
        MENU_INV_SLOT_START,
        RuntimeItemStack::from_summary(stone(1)),
    )
    .unwrap();
    menu.set_by_player(
        crate::inventory::test_item_resolver(),
        None,
        MENU_INV_SLOT_START + 1,
        stone_with_component(
            2,
            "minecraft:rarity",
            NbtSummaryValue::String("common".into()),
        ),
    )
    .unwrap();
    let uncommon = stone_with_component(
        7,
        "minecraft:rarity",
        NbtSummaryValue::String("uncommon".into()),
    );
    let removed = stone_with_component(8, "!minecraft:rarity", NbtSummaryValue::Compound(vec![]));
    menu.set_by_player(
        crate::inventory::test_item_resolver(),
        None,
        MENU_INV_SLOT_START + 2,
        uncommon.clone(),
    )
    .unwrap();
    menu.set_by_player(
        crate::inventory::test_item_resolver(),
        None,
        MENU_INV_SLOT_START + 3,
        removed.clone(),
    )
    .unwrap();
    menu.menu.carried = RuntimeItemStack::from_summary(stone(9));
    let start = quickcraft_mask(QUICKCRAFT_HEADER_START, QUICKCRAFT_TYPE_CHARITABLE);
    let add = quickcraft_mask(QUICKCRAFT_HEADER_CONTINUE, QUICKCRAFT_TYPE_CHARITABLE);
    let end = quickcraft_mask(QUICKCRAFT_HEADER_END, QUICKCRAFT_TYPE_CHARITABLE);

    menu.clicked(
        test_item_resolver(),
        None,
        SLOT_CLICKED_OUTSIDE,
        start,
        ContainerInput::QuickCraft,
        true,
    )
    .unwrap();
    for offset in 0..5 {
        menu.clicked(
            test_item_resolver(),
            None,
            MENU_INV_SLOT_START + offset,
            add,
            ContainerInput::QuickCraft,
            true,
        )
        .unwrap();
    }
    menu.clicked(
        test_item_resolver(),
        None,
        SLOT_CLICKED_OUTSIDE,
        end,
        ContainerInput::QuickCraft,
        true,
    )
    .unwrap();

    for (offset, count) in [(0, 4), (1, 5), (4, 3)] {
        let held = menu.slot_item(MENU_INV_SLOT_START + offset);
        assert_eq!(held.count(), count);
        assert!(held
            .same_item_same_effective_components(
                &RuntimeItemStack::from_summary(stone(1)),
                test_item_resolver(),
            )
            .unwrap());
    }
    assert_eq!(*menu.slot_item(MENU_INV_SLOT_START + 2), uncommon);
    assert_eq!(*menu.slot_item(MENU_INV_SLOT_START + 3), removed);
    assert_eq!(
        menu.inventory
            .iter()
            .map(RuntimeItemStack::count)
            .sum::<i32>()
            + menu.menu.carried.count(),
        27,
    );
    assert_eq!(
        menu.menu.carried,
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );
}

#[test]
fn a_greedy_drag_leaves_one_item_per_slot() {
    // `getQuickCraftPlaceCount` (:735-742) places one per slot for the
    // greedy type, so the rest stays on the cursor.
    let mut menu = PlayerInventoryMenu::empty();
    menu.menu.carried = RuntimeItemStack::from_summary(stone(9));
    let start = quickcraft_mask(QUICKCRAFT_HEADER_START, QUICKCRAFT_TYPE_GREEDY);
    let add = quickcraft_mask(QUICKCRAFT_HEADER_CONTINUE, QUICKCRAFT_TYPE_GREEDY);
    let end = quickcraft_mask(QUICKCRAFT_HEADER_END, QUICKCRAFT_TYPE_GREEDY);

    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        SLOT_CLICKED_OUTSIDE,
        start,
        ContainerInput::QuickCraft,
        true,
    )
    .unwrap();
    for offset in 0..2 {
        menu.clicked(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            MENU_INV_SLOT_START + offset,
            add,
            ContainerInput::QuickCraft,
            true,
        )
        .unwrap();
    }
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        SLOT_CLICKED_OUTSIDE,
        end,
        ContainerInput::QuickCraft,
        true,
    )
    .unwrap();

    assert_eq!(
        *menu.slot_item(MENU_INV_SLOT_START),
        RuntimeItemStack::from_summary(stone(1))
    );
    assert_eq!(
        *menu.slot_item(MENU_INV_SLOT_START + 1),
        RuntimeItemStack::from_summary(stone(1))
    );
    assert_eq!(menu.menu.carried, RuntimeItemStack::from_summary(stone(7)));
}

#[test]
fn a_cloning_drag_needs_infinite_materials() {
    // `isValidQuickcraftType` (:710-716) only accepts the cloning drag
    // from a player whose materials are infinite; anyone else has the
    // drag reset before it starts.
    let start = quickcraft_mask(QUICKCRAFT_HEADER_START, QUICKCRAFT_TYPE_CLONE);
    let mut survival = PlayerInventoryMenu::empty();
    survival.menu.carried = RuntimeItemStack::from_summary(stone(9));
    survival
        .clicked(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            SLOT_CLICKED_OUTSIDE,
            start,
            ContainerInput::QuickCraft,
            false,
        )
        .unwrap();
    assert_eq!(survival.menu.quickcraft_status, 0);

    let mut creative = PlayerInventoryMenu::empty();
    creative.menu.carried = RuntimeItemStack::from_summary(stone(9));
    creative
        .clicked(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            SLOT_CLICKED_OUTSIDE,
            start,
            ContainerInput::QuickCraft,
            true,
        )
        .unwrap();
    assert_eq!(creative.menu.quickcraft_status, QUICKCRAFT_HEADER_CONTINUE);
}

#[test]
fn a_drag_over_one_slot_becomes_a_plain_click_of_the_drag_type() {
    // `doClick` (:361-365) replays a one-slot drag as a PICKUP whose
    // button is the drag type, so a charitable drag places the whole
    // cursor and a greedy one places a single item.
    let mut menu = PlayerInventoryMenu::empty();
    menu.menu.carried = RuntimeItemStack::from_summary(stone(9));
    let start = quickcraft_mask(QUICKCRAFT_HEADER_START, QUICKCRAFT_TYPE_GREEDY);
    let add = quickcraft_mask(QUICKCRAFT_HEADER_CONTINUE, QUICKCRAFT_TYPE_GREEDY);
    let end = quickcraft_mask(QUICKCRAFT_HEADER_END, QUICKCRAFT_TYPE_GREEDY);
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        SLOT_CLICKED_OUTSIDE,
        start,
        ContainerInput::QuickCraft,
        true,
    )
    .unwrap();
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        MENU_INV_SLOT_START,
        add,
        ContainerInput::QuickCraft,
        true,
    )
    .unwrap();
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        SLOT_CLICKED_OUTSIDE,
        end,
        ContainerInput::QuickCraft,
        true,
    )
    .unwrap();

    assert_eq!(
        *menu.slot_item(MENU_INV_SLOT_START),
        RuntimeItemStack::from_summary(stone(1))
    );
    assert_eq!(menu.menu.carried, RuntimeItemStack::from_summary(stone(8)));
}

#[test]
fn a_click_of_another_kind_cancels_a_drag_in_progress() {
    // `doClick` (:401-403): any non-drag click while a drag is open only
    // resets it.
    let mut menu = PlayerInventoryMenu::empty();
    menu.menu.carried = RuntimeItemStack::from_summary(stone(9));
    let start = quickcraft_mask(QUICKCRAFT_HEADER_START, QUICKCRAFT_TYPE_CHARITABLE);
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        SLOT_CLICKED_OUTSIDE,
        start,
        ContainerInput::QuickCraft,
        true,
    )
    .unwrap();
    assert_eq!(menu.menu.quickcraft_status, QUICKCRAFT_HEADER_CONTINUE);

    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        MENU_INV_SLOT_START,
        0,
        ContainerInput::Pickup,
        true,
    )
    .unwrap();
    assert_eq!(menu.menu.quickcraft_status, 0);
    // The cursor is untouched: the cancelling click did nothing else.
    assert_eq!(menu.menu.carried, RuntimeItemStack::from_summary(stone(9)));
    assert_eq!(
        *menu.slot_item(MENU_INV_SLOT_START),
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );
}

#[test]
fn a_client_that_predicted_the_result_is_told_nothing() {
    // `synchronizeSlotToRemote` (:249-260) sends only what the client's
    // own copy no longer matches, and `setRemoteSlotUnsafe` (:288-294)
    // is how the click packet's prediction gets there.
    let mut menu = PlayerInventoryMenu::fixture();
    menu.send_all_data_to_remote().unwrap();
    assert!(menu
        .broadcast_changes(crate::inventory::test_item_resolver(), None)
        .unwrap()
        .is_empty());

    menu.suppress_remote_updates();
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        FIRST_HOTBAR,
        0,
        ContainerInput::Pickup,
        true,
    )
    .unwrap();
    menu.set_remote_slot_unsafe(FIRST_HOTBAR, &HashedStack::Empty);
    menu.set_remote_carried(&hashed(&stone(64)));
    menu.resume_remote_updates();

    assert!(menu
        .broadcast_changes(crate::inventory::test_item_resolver(), None)
        .unwrap()
        .is_empty());
}

#[test]
fn a_client_that_predicted_wrong_is_corrected_slot_by_slot() {
    let mut menu = PlayerInventoryMenu::fixture();
    menu.send_all_data_to_remote().unwrap();

    menu.suppress_remote_updates();
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        FIRST_HOTBAR,
        0,
        ContainerInput::Pickup,
        true,
    )
    .unwrap();
    // The client thinks it only picked up half.
    menu.set_remote_slot_unsafe(FIRST_HOTBAR, &hashed(&stone(32)));
    menu.set_remote_carried(&hashed(&stone(32)));
    menu.resume_remote_updates();

    let sync = menu
        .broadcast_changes(crate::inventory::test_item_resolver(), None)
        .unwrap();
    assert_eq!(
        sync,
        vec![
            MenuSync::Slot {
                container_id: MENU_CONTAINER_ID,
                state_id: 2,
                slot: FIRST_HOTBAR,
                item: ItemStackSummary::empty(),
            },
            MenuSync::Cursor { item: stone(64) },
        ]
    );
    // A second broadcast with nothing new says nothing.
    assert!(menu
        .broadcast_changes(crate::inventory::test_item_resolver(), None)
        .unwrap()
        .is_empty());
}

#[test]
fn closing_the_menu_hands_the_cursor_back_too() {
    // `AbstractContainerMenu.removed` (:589-598) runs before the crafting
    // grid is cleared, so a carried stack is not lost on close.
    let mut menu = PlayerInventoryMenu::empty();
    menu.menu.carried = RuntimeItemStack::from_summary(stone(5));
    let filled = menu
        .do_close_container(crate::inventory::test_item_resolver(), true)
        .unwrap()
        .filled;
    assert_eq!(filled, vec![0]);
    assert_eq!(menu.inventory[0], RuntimeItemStack::from_summary(stone(5)));
    assert_eq!(
        menu.menu.carried,
        RuntimeItemStack::from_summary(ItemStackSummary::empty())
    );
}

#[test]
fn a_click_that_names_no_slot_changes_nothing() {
    // `isValidSlotIndex` (:119-121) admits -1 and -999, and `doClick`
    // returns for a negative index outside the outside-click sentinel.
    let inventory_menu = PlayerInventoryMenu::empty();
    assert!(inventory_menu.is_valid_slot_index(-1));
    assert!(inventory_menu.is_valid_slot_index(SLOT_CLICKED_OUTSIDE));
    assert!(inventory_menu.is_valid_slot_index(MENU_SLOT_COUNT - 1));
    assert!(!inventory_menu.is_valid_slot_index(MENU_SLOT_COUNT));

    let mut menu = PlayerInventoryMenu::fixture();
    let before = menu.clone();
    menu.clicked(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        -1,
        0,
        ContainerInput::Pickup,
        true,
    )
    .unwrap();
    assert_eq!(menu, before);
}

fn quickcraft_mask(header: i32, kind: i32) -> i8 {
    i8::try_from(header & 3 | (kind & 3) << 2).expect("the mask is four bits")
}

fn hashed(item: &ItemStackSummary) -> HashedStack {
    HashedStack::Item(bbb_protocol::packets::HashedItemStack {
        item_id: item.item_id.expect("a hashed stack names an item"),
        count: item.count,
        components: Default::default(),
    })
}

fn stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn test_recipe_set(
    id: &str,
    body: serde_json::Value,
) -> std::sync::Arc<bbb_server_datapack::recipe::RecipeSet> {
    let bytes = serde_json::to_vec(&body).unwrap();
    let source = bbb_registry_codec::decode_direct_registry_json(&bytes).unwrap();
    let startup = crate::registry_sync::builtin_datapack_startup_shared();
    let lookup =
        bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup::from_startup(&startup);
    std::sync::Arc::new(bbb_server_datapack::recipe::RecipeSet {
        recipes: vec![bbb_server_datapack::recipe::RecipeElement {
            id: id.to_string(),
            source_pack: "test".to_string(),
            recipe: bbb_server_datapack::recipe::parse_recipe_source_with_lookup(
                source.root(),
                &lookup,
            )
            .expect("test recipe"),
        }],
        warnings: Vec::new(),
    })
}

#[test]
fn player_grid_slots_changed_writes_the_result_slot() {
    use bbb_protocol::item_types::{
        VANILLA_ITEM_FLINT_AND_STEEL_ID, VANILLA_ITEM_FLINT_ID, VANILLA_ITEM_IRON_INGOT_ID,
    };
    let mut menu = PlayerInventoryMenu::empty();
    menu.install_crafting_catalog(
        crate::inventory::test_item_resolver(),
        None,
        test_recipe_set(
            "minecraft:flint_and_steel",
            serde_json::json!({
                "type": "minecraft:crafting_shapeless",
                "ingredients": ["minecraft:iron_ingot", "minecraft:flint"],
                "result": { "id": "minecraft:flint_and_steel" }
            }),
        ),
        bbb_server_datapack::recipe::ItemTagIndex::new(),
    )
    .unwrap();
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_IRON_INGOT_ID, 1))
        )
        .unwrap());
    assert!(menu.slot_item(0).is_empty());
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            2,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_FLINT_ID, 1))
        )
        .unwrap());
    assert_eq!(
        menu.slot_item(0).item_id(),
        Some(VANILLA_ITEM_FLINT_AND_STEEL_ID)
    );
    assert_eq!(menu.slot_item(0).count(), 1);
}

#[test]
fn result_on_take_consumes_ingredients_places_remainders_and_counts_crafted() {
    use bbb_protocol::item_types::{
        VANILLA_ITEM_BUCKET_ID, VANILLA_ITEM_MILK_BUCKET_ID, VANILLA_ITEM_STICK_ID,
        VANILLA_ITEM_WHEAT_ID,
    };
    let mut menu = PlayerInventoryMenu::empty();
    menu.install_crafting_catalog(
        crate::inventory::test_item_resolver(),
        None,
        test_recipe_set(
            "test:milk_wheat",
            serde_json::json!({
                "type": "minecraft:crafting_shapeless",
                "ingredients": ["minecraft:milk_bucket", "minecraft:wheat"],
                "result": { "id": "minecraft:stick" }
            }),
        ),
        bbb_server_datapack::recipe::ItemTagIndex::new(),
    )
    .unwrap();
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_MILK_BUCKET_ID, 1))
        )
        .unwrap());
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            2,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_WHEAT_ID, 1))
        )
        .unwrap());
    assert_eq!(menu.slot_item(0).item_id(), Some(VANILLA_ITEM_STICK_ID));
    let taken = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            1,
            64,
            false,
        )
        .expect("take result");
    assert_eq!(taken.item_id(), Some(VANILLA_ITEM_STICK_ID));
    assert_eq!(menu.crafted_count(VANILLA_ITEM_STICK_ID), 1);
    assert!(menu.slot_item(2).is_empty());
    let remainders: Vec<_> = (1..=4)
        .map(|slot| menu.slot_item(slot).into_owned())
        .filter(|item| !item.is_empty())
        .collect();
    assert_eq!(remainders.len(), 1);
    assert_eq!(remainders[0].item_id(), Some(VANILLA_ITEM_BUCKET_ID));
    assert_eq!(remainders[0].count(), 1);
}

#[test]
fn crafting_a_recipe_records_its_unlock() {
    use bbb_protocol::item_types::{VANILLA_ITEM_FLINT_ID, VANILLA_ITEM_IRON_INGOT_ID};
    let mut menu = PlayerInventoryMenu::empty();
    menu.install_crafting_catalog(
        crate::inventory::test_item_resolver(),
        None,
        test_recipe_set(
            "minecraft:flint_and_steel",
            serde_json::json!({
                "type": "minecraft:crafting_shapeless",
                "ingredients": ["minecraft:iron_ingot", "minecraft:flint"],
                "result": { "id": "minecraft:flint_and_steel" }
            }),
        ),
        bbb_server_datapack::recipe::ItemTagIndex::new(),
    )
    .unwrap();
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_IRON_INGOT_ID, 1))
        )
        .unwrap());
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            2,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_FLINT_ID, 1))
        )
        .unwrap());
    let _ = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            1,
            64,
            false,
        )
        .expect("take result");
    assert_eq!(
        menu.take_crafted_recipes()
            .into_iter()
            .filter(|craft| craft.unlock)
            .map(|craft| craft.recipe_id)
            .collect::<Vec<_>>(),
        ["minecraft:flint_and_steel"]
    );
}

#[test]
fn pickup_and_quick_move_from_result_run_on_take() {
    use bbb_protocol::item_types::{
        VANILLA_ITEM_FLINT_AND_STEEL_ID, VANILLA_ITEM_FLINT_ID, VANILLA_ITEM_IRON_INGOT_ID,
    };
    let recipes = test_recipe_set(
        "minecraft:flint_and_steel",
        serde_json::json!({
            "type": "minecraft:crafting_shapeless",
            "ingredients": ["minecraft:iron_ingot", "minecraft:flint"],
            "result": { "id": "minecraft:flint_and_steel" }
        }),
    );
    let tags = bbb_server_datapack::recipe::ItemTagIndex::new();

    let mut pickup = PlayerInventoryMenu::empty();
    pickup
        .install_crafting_catalog(
            crate::inventory::test_item_resolver(),
            None,
            recipes.clone(),
            tags.clone(),
        )
        .unwrap();
    assert!(pickup
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_IRON_INGOT_ID, 1))
        )
        .unwrap());
    assert!(pickup
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            2,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_FLINT_ID, 1))
        )
        .unwrap());
    pickup
        .clicked(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            MENU_RESULT_SLOT,
            0,
            ContainerInput::Pickup,
            false,
        )
        .unwrap();
    assert_eq!(
        pickup.menu.carried.item_id(),
        Some(VANILLA_ITEM_FLINT_AND_STEEL_ID)
    );
    assert!(pickup.slot_item(1).is_empty());
    assert!(pickup.slot_item(2).is_empty());
    assert_eq!(pickup.crafted_count(VANILLA_ITEM_FLINT_AND_STEEL_ID), 1);
    assert_eq!(
        pickup
            .take_crafted_recipes()
            .into_iter()
            .filter(|craft| craft.unlock)
            .map(|craft| craft.recipe_id)
            .collect::<Vec<_>>(),
        ["minecraft:flint_and_steel"]
    );

    let mut quick = PlayerInventoryMenu::empty();
    quick
        .install_crafting_catalog(crate::inventory::test_item_resolver(), None, recipes, tags)
        .unwrap();
    assert!(quick
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_IRON_INGOT_ID, 1))
        )
        .unwrap());
    assert!(quick
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            2,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_FLINT_ID, 1))
        )
        .unwrap());
    quick
        .clicked(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            MENU_RESULT_SLOT,
            0,
            ContainerInput::QuickMove,
            false,
        )
        .unwrap();
    assert!(quick.slot_item(1).is_empty());
    assert!(quick.slot_item(2).is_empty());
    assert!(
        quick
            .inventory
            .iter()
            .any(|item| item.item_id() == Some(VANILLA_ITEM_FLINT_AND_STEEL_ID)),
        "quick-move from the result must land in player storage"
    );
    assert_eq!(quick.crafted_count(VANILLA_ITEM_FLINT_AND_STEEL_ID), 1);
}

#[test]
fn slots_changed_sends_the_crafting_result_slot() {
    use bbb_protocol::item_types::{
        VANILLA_ITEM_FLINT_AND_STEEL_ID, VANILLA_ITEM_FLINT_ID, VANILLA_ITEM_IRON_INGOT_ID,
    };
    let mut menu = PlayerInventoryMenu::empty();
    menu.install_crafting_catalog(
        crate::inventory::test_item_resolver(),
        None,
        test_recipe_set(
            "minecraft:flint_and_steel",
            serde_json::json!({
                "type": "minecraft:crafting_shapeless",
                "ingredients": ["minecraft:iron_ingot", "minecraft:flint"],
                "result": { "id": "minecraft:flint_and_steel" }
            }),
        ),
        bbb_server_datapack::recipe::ItemTagIndex::new(),
    )
    .unwrap();
    let _ = menu.send_all_data_to_remote().unwrap();
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_IRON_INGOT_ID, 1))
        )
        .unwrap());
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            2,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_FLINT_ID, 1))
        )
        .unwrap());
    let sync = menu.take_crafting_sync();
    assert!(
        sync.iter().any(|packet| matches!(
            packet,
            MenuSync::Slot {
                slot: MENU_RESULT_SLOT,
                item,
                ..
            } if item.item_id == Some(VANILLA_ITEM_FLINT_AND_STEEL_ID)
        )),
        "result rematch must be a ContainerSetSlot-shaped MenuSync::Slot: {sync:?}"
    );
}

/// `AbstractContainerMenu.getRedstoneSignalFromContainer`
/// (`AbstractContainerMenu.java:751-767`) divides each slot by
/// `container.getMaxStackSize(itemStack)` (`Container.java:38-40`), which is
/// the *stack's* own ceiling wherever it is under 99. A full chest therefore
/// reads 15, and the flat container ceiling would read 10.
#[test]
fn the_container_fill_fraction_divides_by_each_stack_own_ceiling() {
    const CHEST_SIZE: usize = 27;
    let full = |item_id, count| vec![stack(item_id, count); CHEST_SIZE];

    // 27 * (64/64) / 27 = 1.0; floor(1.0 * 14) + 1.
    assert_eq!(
        redstone_signal_from_container(&full(VANILLA_ITEM_STONE_ID, 64)).unwrap(),
        15
    );
    // A 16-stack item reaches the same 1.0 at a quarter of the item count.
    assert_eq!(
        redstone_signal_from_container(&full(VANILLA_ITEM_EGG_ID, 16)).unwrap(),
        15
    );
    // 14 * (64/64) / 27 = 0.5185...; floor(7.259) + 1.
    let mut half = vec![ItemStackSummary::empty(); CHEST_SIZE];
    half[..14].fill(stack(VANILLA_ITEM_STONE_ID, 64));
    assert_eq!(redstone_signal_from_container(&half).unwrap(), 8);

    // One stack of anything is the "not empty" floor of 1.
    let mut one = vec![ItemStackSummary::empty(); CHEST_SIZE];
    one[0] = stack(VANILLA_ITEM_STONE_ID, 64);
    assert_eq!(redstone_signal_from_container(&one).unwrap(), 1);
    let mut single = vec![ItemStackSummary::empty(); CHEST_SIZE];
    single[0] = stack(VANILLA_ITEM_STONE_ID, 1);
    assert_eq!(redstone_signal_from_container(&single).unwrap(), 1);

    // `0 / 0` is `NaN`, which fails both the floor and the `alpha > 0` test.
    assert_eq!(
        redstone_signal_from_container(&vec![ItemStackSummary::empty(); CHEST_SIZE]).unwrap(),
        0
    );
    assert_eq!(
        redstone_signal_from_container(&[] as &[ItemStackSummary]).unwrap(),
        0
    );
}

/// `Container.getMaxStackSize(ItemStack)` is a `min`, so an item whose own
/// `max_stack_size` component exceeds the container ceiling is capped by it.
#[test]
fn the_container_ceiling_caps_a_stack_that_declares_a_larger_one() {
    let mut oversized = stack(VANILLA_ITEM_STONE_ID, 1);
    oversized.component_patch.max_stack_size = Some(CONTAINER_MAX_STACK_SIZE + 1);
    assert_eq!(
        container_max_stack_size(&oversized).unwrap(),
        CONTAINER_MAX_STACK_SIZE
    );
    let mut narrowed = stack(VANILLA_ITEM_STONE_ID, 1);
    narrowed.component_patch.max_stack_size = Some(16);
    assert_eq!(container_max_stack_size(&narrowed).unwrap(), 16);
}
