//! The `InventoryMenu` / `ChestMenu` slot numbering and what each slot may hold.

use bbb_protocol::{
    block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SHULKER_BOX_ID,
    block_types::vanilla_block_type,
    equipment_slots::{EquipmentSlot, EquipmentSlotType, VANILLA_EQUIPMENT_SLOTS},
    item_types::vanilla_item_type,
    menu_types::{
        VANILLA_MENU_TYPE_BLAST_FURNACE_ID, VANILLA_MENU_TYPE_BREWING_STAND_ID,
        VANILLA_MENU_TYPE_CRAFTER_3X3_ID, VANILLA_MENU_TYPE_CRAFTING_ID,
        VANILLA_MENU_TYPE_FURNACE_ID, VANILLA_MENU_TYPE_GENERIC_3X3_ID,
        VANILLA_MENU_TYPE_GENERIC_9X3_ID, VANILLA_MENU_TYPE_GENERIC_9X6_ID,
        VANILLA_MENU_TYPE_HOPPER_ID, VANILLA_MENU_TYPE_LECTERN_ID, VANILLA_MENU_TYPE_MERCHANT_ID,
        VANILLA_MENU_TYPE_SHULKER_BOX_ID, VANILLA_MENU_TYPE_SMOKER_ID,
    },
    packets::ItemStackSummary,
};

use bbb_server_types::Result;

use super::{
    FurnaceMenuKind, MenuLayout, MenuSlot, MenuSlotKind, CHEST_MENU_ROWS, DOUBLE_CHEST_MENU_ROWS,
    INVENTORY_SIZE, MENU_ARMOR_SLOT_END, MENU_ARMOR_SLOT_IDS, MENU_ARMOR_SLOT_START,
    MENU_CRAFT_SLOT_END, MENU_CRAFT_SLOT_START, MENU_INV_SLOT_END, MENU_INV_SLOT_START,
    MENU_RESULT_SLOT, MENU_SHIELD_SLOT, MENU_SLOT_COUNT, MENU_USE_ROW_SLOT_END,
    MENU_USE_ROW_SLOT_START, SLOTS_PER_ROW, STANDARD_INVENTORY_SLOT_COUNT,
};

impl MenuLayout {
    /// `AbstractContainerMenu.slots.size()`.
    pub fn slot_count(self) -> i16 {
        match self {
            Self::PlayerInventory => MENU_SLOT_COUNT,
            Self::Chest { rows } => rows * SLOTS_PER_ROW + STANDARD_INVENTORY_SLOT_COUNT,
            Self::Generic3x3 => 9 + STANDARD_INVENTORY_SLOT_COUNT,
            Self::Crafter | Self::CraftingTable => 10 + STANDARD_INVENTORY_SLOT_COUNT,
            Self::Hopper => 5 + STANDARD_INVENTORY_SLOT_COUNT,
            Self::ShulkerBox => CHEST_MENU_ROWS * SLOTS_PER_ROW + STANDARD_INVENTORY_SLOT_COUNT,
            Self::Lectern => 1,
            // `AbstractFurnaceMenu` (`AbstractFurnaceMenu.java:38-73`): three
            // container slots then the standard inventory.
            Self::Furnace { .. } => 3 + STANDARD_INVENTORY_SLOT_COUNT,
            // `BrewingStandMenu` (`BrewingStandMenu.java:41-55`): five
            // container slots then the standard inventory.
            Self::BrewingStand => BREWING_STAND_MENU_SLOTS + STANDARD_INVENTORY_SLOT_COUNT,
            Self::Merchant => 3 + STANDARD_INVENTORY_SLOT_COUNT,
            Self::Utility(kind) => kind.container_size() as i16 + STANDARD_INVENTORY_SLOT_COUNT,
            Self::Mount { columns } => 2 + columns * 3 + STANDARD_INVENTORY_SLOT_COUNT,
        }
    }

    /// `Container.getContainerSize()` of the container the menu was built
    /// over, or zero for a menu that has no backing container of its own.
    pub fn container_size(self) -> usize {
        match self {
            Self::PlayerInventory => 0,
            Self::Chest { rows } => {
                usize::try_from(rows * SLOTS_PER_ROW).expect("row counts are small and positive")
            }
            Self::Generic3x3 => 9,
            Self::Crafter => 9,
            Self::CraftingTable => 0,
            Self::Hopper => 5,
            Self::ShulkerBox => {
                usize::try_from(CHEST_MENU_ROWS * SLOTS_PER_ROW).expect("row counts are small")
            }
            Self::Lectern => 1,
            Self::Furnace { .. } => 3,
            Self::BrewingStand => usize::try_from(BREWING_STAND_MENU_SLOTS).expect("five slots"),
            Self::Merchant => 3,
            Self::Utility(kind) => kind.container_size(),
            Self::Mount { columns } => {
                usize::try_from(2 + columns * 3).expect("column counts are small and positive")
            }
        }
    }

    /// The `MenuType` this menu is opened with, or `None` for `InventoryMenu`,
    /// whose constructor passes null (`InventoryMenu.java:49`) and whose
    /// `getType` therefore throws (AbstractContainerMenu.java:97-103).
    pub fn menu_type_id(self) -> Option<i32> {
        match self {
            Self::PlayerInventory => None,
            Self::Chest { rows } if rows == CHEST_MENU_ROWS => {
                Some(VANILLA_MENU_TYPE_GENERIC_9X3_ID)
            }
            Self::Chest { rows } if rows == DOUBLE_CHEST_MENU_ROWS => {
                Some(VANILLA_MENU_TYPE_GENERIC_9X6_ID)
            }
            // `ChestMenu` has a factory per row count
            // (`ChestMenu.java:17-39`), but only the two above take a backing
            // `Container` (`:41-47`), so no other row count can be opened over
            // a block entity.
            Self::Chest { .. } => None,
            Self::Generic3x3 => Some(VANILLA_MENU_TYPE_GENERIC_3X3_ID),
            Self::Crafter => Some(VANILLA_MENU_TYPE_CRAFTER_3X3_ID),
            Self::CraftingTable => Some(VANILLA_MENU_TYPE_CRAFTING_ID),
            Self::Hopper => Some(VANILLA_MENU_TYPE_HOPPER_ID),
            Self::ShulkerBox => Some(VANILLA_MENU_TYPE_SHULKER_BOX_ID),
            Self::Lectern => Some(VANILLA_MENU_TYPE_LECTERN_ID),
            Self::Furnace { kind } => Some(kind.menu_type_id()),
            Self::BrewingStand => Some(VANILLA_MENU_TYPE_BREWING_STAND_ID),
            Self::Merchant => Some(VANILLA_MENU_TYPE_MERCHANT_ID),
            Self::Utility(kind) => Some(kind.menu_type_id()),
            Self::Mount { .. } => None,
        }
    }

    /// Where a slot id writes, or `None` for an id this menu does not have.
    pub fn slot(self, slot: i16) -> Option<MenuSlot> {
        match self {
            Self::PlayerInventory => player_inventory_menu_slot(slot),
            Self::Chest { rows } => chest_menu_slot(rows, slot),
            Self::Generic3x3 => chest_menu_slot(1, slot),
            Self::Crafter => {
                if slot == 45 {
                    Some(MenuSlot::CrafterPreview)
                } else {
                    container_menu_slot(9, slot)
                }
            }
            // `CraftingMenu.RESULT_SLOT` .. `CRAFT_SLOT_END` (`CraftingMenu.java:22-25`).
            Self::CraftingTable => match slot {
                0 => Some(MenuSlot::CraftingResult),
                1..=9 => Some(MenuSlot::CraftingGrid(usize::try_from(slot - 1).ok()?)),
                _ => container_menu_slot(10, slot),
            },
            Self::Hopper => container_menu_slot(5, slot),
            Self::ShulkerBox => chest_menu_slot(CHEST_MENU_ROWS, slot),
            Self::Lectern => (slot == 0).then_some(MenuSlot::Container(0)),
            Self::Furnace { .. } => furnace_menu_slot(slot),
            Self::BrewingStand => brewing_stand_menu_slot(slot),
            Self::Merchant => container_menu_slot(3, slot),
            Self::Utility(kind) => container_menu_slot(kind.container_size() as i16, slot),
            Self::Mount { columns } => container_menu_slot(2 + columns * 3, slot),
        }
    }

    /// The behaviour the `Slot` at this index has.
    pub fn slot_kind(self, slot: i16) -> MenuSlotKind {
        match self {
            Self::PlayerInventory => player_inventory_menu_slot_kind(slot),
            Self::CraftingTable if slot == 0 => MenuSlotKind::CraftingResult,
            Self::CraftingTable => MenuSlotKind::Plain,
            Self::Crafter => match slot {
                0..=8 => MenuSlotKind::CrafterInput,
                45 => MenuSlotKind::CrafterPreview,
                _ => MenuSlotKind::Plain,
            },
            // `addChestGrid` builds a plain `Slot` (`ChestMenu.java:64`) and
            // `addStandardInventorySlots` builds plain slots too
            // (AbstractContainerMenu.java:72-84), so a chest menu has no
            // slot with an override anywhere in it.
            Self::Chest { .. } | Self::Generic3x3 | Self::Hopper => MenuSlotKind::Plain,
            Self::ShulkerBox => {
                let grid = CHEST_MENU_ROWS * SLOTS_PER_ROW;
                if slot >= 0 && slot < grid {
                    MenuSlotKind::ShulkerBox
                } else {
                    MenuSlotKind::Plain
                }
            }
            // `LecternMenu`'s slot is a plain `Slot` (`LecternMenu.java:28-34`);
            // `canPlaceItem` on the lectern container is false (`:100-102`).
            Self::Lectern => MenuSlotKind::LecternBook,
            // `AbstractFurnaceMenu` (`AbstractFurnaceMenu.java:68-70`): the
            // input slot, the fuel slot and the result slot, then plain
            // standard-inventory slots.
            Self::Furnace { .. } => match slot {
                0 => MenuSlotKind::FurnaceInput,
                1 => MenuSlotKind::FurnaceFuel,
                2 => MenuSlotKind::FurnaceResult,
                _ => MenuSlotKind::Plain,
            },
            // `BrewingStandMenu` (`BrewingStandMenu.java:48-52`): three
            // `PotionSlot`s, the ingredient slot and the fuel slot.
            Self::BrewingStand => match slot {
                0..=2 => MenuSlotKind::BrewingPotion,
                3 => MenuSlotKind::BrewingIngredient,
                4 => MenuSlotKind::BrewingFuel,
                _ => MenuSlotKind::Plain,
            },
            Self::Merchant => match slot {
                2 => MenuSlotKind::MerchantResult,
                _ => MenuSlotKind::Plain,
            },
            Self::Utility(kind) if kind.result_slot() == Some(slot) => MenuSlotKind::UtilityResult,
            Self::Utility(kind) if (0..kind.container_size() as i16).contains(&slot) => {
                MenuSlotKind::UtilityInput
            }
            Self::Utility(_) => MenuSlotKind::Plain,
            Self::Mount { .. } => match slot {
                0 => MenuSlotKind::MountEquipment(EquipmentSlot::Saddle),
                1 => MenuSlotKind::MountEquipment(EquipmentSlot::Body),
                _ => MenuSlotKind::Plain,
            },
        }
    }

    /// `AbstractContainerMenu.canTakeItemForPickAll` (:584-586), whose base
    /// answer is true and whose one override here is `InventoryMenu`'s
    /// (InventoryMenu.java:164-166).
    pub fn can_take_item_for_pick_all(self, slot: i16) -> bool {
        match self {
            // `CraftingMenu.canTakeItemForPickAll` (`CraftingMenu.java:156-158`).
            Self::PlayerInventory | Self::CraftingTable => {
                self.slot_kind(slot) != MenuSlotKind::CraftingResult
            }
            Self::Chest { .. }
            | Self::Generic3x3
            | Self::Crafter
            | Self::Hopper
            | Self::ShulkerBox
            | Self::Lectern
            | Self::Furnace { .. }
            | Self::BrewingStand
            | Self::Mount { .. } => true,
            // `MerchantMenu.canTakeItemForPickAll` (`MerchantMenu.java:96-98`).
            Self::Merchant => false,
            Self::Utility(kind) => {
                !(kind.result_excluded_from_pick_all() && kind.result_slot() == Some(slot))
            }
        }
    }
}

impl FurnaceMenuKind {
    /// The `MenuType` each subclass registers (`FurnaceMenu.java:10-14`,
    /// `SmokerMenu.java:10-14`, `BlastFurnaceMenu.java:10-14`).
    #[must_use]
    pub fn menu_type_id(self) -> i32 {
        match self {
            Self::Furnace => VANILLA_MENU_TYPE_FURNACE_ID,
            Self::Smoker => VANILLA_MENU_TYPE_SMOKER_ID,
            Self::BlastFurnace => VANILLA_MENU_TYPE_BLAST_FURNACE_ID,
        }
    }
}

/// `BrewingStandMenu`'s five container slots (`BrewingStandMenu.java:23-27`).
const BREWING_STAND_MENU_SLOTS: i16 = 5;

/// `BrewingStandMenu`'s slot list (`BrewingStandMenu.java:48-54`): the three
/// bottle slots, the ingredient slot and the fuel slot, then
/// `addStandardInventorySlots`.
pub fn brewing_stand_menu_slot(slot: i16) -> Option<MenuSlot> {
    if slot < 0 {
        return None;
    }
    if slot < BREWING_STAND_MENU_SLOTS {
        return Some(MenuSlot::Container(
            usize::try_from(slot).expect("the range is non-negative"),
        ));
    }
    let standard = slot - BREWING_STAND_MENU_SLOTS;
    if standard < 3 * SLOTS_PER_ROW {
        return Some(MenuSlot::Inventory(
            usize::try_from(standard + SLOTS_PER_ROW).expect("the range is non-negative"),
        ));
    }
    if standard < STANDARD_INVENTORY_SLOT_COUNT {
        return Some(MenuSlot::Inventory(
            usize::try_from(standard - 3 * SLOTS_PER_ROW).expect("the range is non-negative"),
        ));
    }
    None
}

/// `AbstractFurnaceMenu`'s slot list (`AbstractFurnaceMenu.java:38-73`): three
/// container slots then `addStandardInventorySlots`.
pub fn furnace_menu_slot(slot: i16) -> Option<MenuSlot> {
    if slot < 0 {
        return None;
    }
    if slot < 3 {
        return Some(MenuSlot::Container(
            usize::try_from(slot).expect("the range is non-negative"),
        ));
    }
    let standard = slot - 3;
    if standard < 3 * SLOTS_PER_ROW {
        return Some(MenuSlot::Inventory(
            usize::try_from(standard + SLOTS_PER_ROW).expect("the range is non-negative"),
        ));
    }
    if standard < STANDARD_INVENTORY_SLOT_COUNT {
        return Some(MenuSlot::Inventory(
            usize::try_from(standard - 3 * SLOTS_PER_ROW).expect("the range is non-negative"),
        ));
    }
    None
}

/// `ChestMenu`'s slot list (`ChestMenu.java:56-58`): the chest grid, then the
/// player's three storage rows, then the player's hotbar.
pub fn chest_menu_slot(rows: i16, slot: i16) -> Option<MenuSlot> {
    container_menu_slot(rows * SLOTS_PER_ROW, slot)
}

fn container_menu_slot(grid: i16, slot: i16) -> Option<MenuSlot> {
    if slot < 0 {
        return None;
    }
    if slot < grid {
        return Some(MenuSlot::Container(
            usize::try_from(slot).expect("the range is non-negative"),
        ));
    }
    let standard = slot - grid;
    if standard < 3 * SLOTS_PER_ROW {
        // `addInventoryExtendedSlots` (AbstractContainerMenu.java:78-84) walks
        // `x + (y + 1) * 9`, i.e. the inventory slots above the hotbar.
        return Some(MenuSlot::Inventory(
            usize::try_from(standard + SLOTS_PER_ROW).expect("the range is non-negative"),
        ));
    }
    if standard < STANDARD_INVENTORY_SLOT_COUNT {
        // `addInventoryHotbarSlots` (:72-76) walks the inventory's first nine.
        return Some(MenuSlot::Inventory(
            usize::try_from(standard - 3 * SLOTS_PER_ROW).expect("the range is non-negative"),
        ));
    }
    None
}

/// Where an `InventoryMenu` slot id writes, or `None` for an id the menu does
/// not have.
pub fn player_inventory_menu_slot(slot: i16) -> Option<MenuSlot> {
    if slot == MENU_RESULT_SLOT {
        return Some(MenuSlot::CraftingResult);
    }
    if (MENU_CRAFT_SLOT_START..MENU_CRAFT_SLOT_END).contains(&slot) {
        return Some(MenuSlot::CraftingGrid(
            usize::try_from(slot - MENU_CRAFT_SLOT_START).expect("the range is non-negative"),
        ));
    }
    if (MENU_ARMOR_SLOT_START..MENU_ARMOR_SLOT_END).contains(&slot) {
        let armor =
            usize::try_from(slot - MENU_ARMOR_SLOT_START).expect("the range is non-negative");
        return Some(MenuSlot::Inventory(player_inventory_slot(
            MENU_ARMOR_SLOT_IDS[armor],
        )));
    }
    if (MENU_INV_SLOT_START..MENU_INV_SLOT_END).contains(&slot) {
        // `addInventoryExtendedSlots` (AbstractContainerMenu.java:78-84) walks
        // the three rows above the hotbar, which are the inventory slots that
        // share their own numbering.
        return Some(MenuSlot::Inventory(
            usize::try_from(slot).expect("the range is non-negative"),
        ));
    }
    if (MENU_USE_ROW_SLOT_START..MENU_USE_ROW_SLOT_END).contains(&slot) {
        // `addInventoryHotbarSlots` (AbstractContainerMenu.java:72-76) puts
        // the inventory's *first* nine slots last in the menu.
        return Some(MenuSlot::Inventory(
            usize::try_from(slot - MENU_USE_ROW_SLOT_START).expect("the range is non-negative"),
        ));
    }
    if slot == MENU_SHIELD_SLOT {
        return Some(MenuSlot::Inventory(player_inventory_slot(
            EquipmentSlot::OffHand,
        )));
    }
    None
}

/// `BlockItem.canFitInsideContainerItems` (`BlockItem.java:194-196`).
pub fn item_places_a_shulker_box(item: &ItemStackSummary) -> bool {
    let Some(item_id) = item.item_id else {
        return false;
    };
    let Some(behavior) = vanilla_item_type(item_id).and_then(|entry| entry.block_item_behavior)
    else {
        return false;
    };
    vanilla_block_type(behavior.block_type_id).and_then(|block| block.block_entity_type_id)
        == Some(VANILLA_BLOCK_ENTITY_TYPE_SHULKER_BOX_ID)
}

pub fn player_inventory_menu_slot_kind(slot: i16) -> MenuSlotKind {
    if slot == MENU_RESULT_SLOT {
        return MenuSlotKind::CraftingResult;
    }
    if (MENU_ARMOR_SLOT_START..MENU_ARMOR_SLOT_END).contains(&slot) {
        let armor =
            usize::try_from(slot - MENU_ARMOR_SLOT_START).expect("the range is non-negative");
        return MenuSlotKind::Armor(MENU_ARMOR_SLOT_IDS[armor]);
    }
    MenuSlotKind::Plain
}

/// `LivingEntity.isEquippableInSlot` (LivingEntity.java:3821-3826) for a
/// player and one of the four armor slots.
///
/// A stack with no `equippable` component is only equippable in the main hand,
/// so it never fits an armor slot. `canUseSlot` is unconditionally true for a
/// player (LivingEntity.java:2240-2242), and no `head`/`chest`/`legs`/`feet`
/// entry in the generated prototype catalog carries `allowed_entities`, so
/// `canBeEquippedBy` cannot refuse a player either.
pub fn is_equippable_in_slot(item: &ItemStackSummary, slot: EquipmentSlot) -> Result<bool> {
    bbb_server_types::item_stack::rules::is_equippable_in_slot(item, slot)
}

/// The name `EquipmentSlot.CODEC` writes, which is what the `equippable`
/// component's `slot` field carries.
pub fn equipment_slot_serialized_name(slot: EquipmentSlot) -> &'static str {
    bbb_server_types::item_stack::rules::equipment_slot_serialized_name(slot)
}

pub use bbb_server_types::slot_ranges::player_inventory_slot;

/// One past the last addressable `Inventory` slot. The equipment slots sit
/// densely above the storage slots, so the whole inventory is one flat range.
pub fn addressable_slot_count() -> usize {
    VANILLA_EQUIPMENT_SLOTS
        .iter()
        .filter_map(|entry| entry.player_inventory_slot)
        .map(|slot| usize::try_from(slot).expect("generated inventory slots are non-negative") + 1)
        .max()
        .unwrap_or(INVENTORY_SIZE)
        .max(INVENTORY_SIZE)
}

/// `LivingEntity.getEquipmentSlotForItem` (LivingEntity.java:3816-3819) for a
/// player, whose `canUseSlot` never refuses.
pub fn equipment_slot_for_item(item: &ItemStackSummary) -> Result<EquipmentSlot> {
    bbb_server_types::item_stack::rules::equipment_slot_for_item(item)
}

/// The armor menu slot an equipment slot occupies, `8 - eqSlot.getIndex()`
/// (InventoryMenu.java:130-135), or `None` for anything but humanoid armor.
pub fn armor_menu_slot(slot: EquipmentSlot) -> Option<i16> {
    let entry = VANILLA_EQUIPMENT_SLOTS
        .iter()
        .find(|entry| entry.slot == slot)
        .expect("the generated catalog carries every equipment slot");
    if entry.slot_type != EquipmentSlotType::HumanoidArmor {
        return None;
    }
    let index = i16::try_from(entry.index).expect("generated slot indexes are small");
    Some(MENU_ARMOR_SLOT_END - 1 - index)
}
