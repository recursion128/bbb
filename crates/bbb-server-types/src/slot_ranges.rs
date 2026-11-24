//! `SlotRanges` (`SlotRanges.java:16-95`) as the `minecraft:item_slot` lookup,
//! plus `Player.getSlot` / `LivingEntity.getSlot` command-slot projection.

use bbb_facts::equipment_slots::{EquipmentSlot, VANILLA_EQUIPMENT_SLOTS};
use std::collections::HashMap;
use std::sync::OnceLock;

/// `PlayerEnderChestContainer`'s `super(27)`
/// (`PlayerEnderChestContainer.java:16-18`).
pub const ENDER_CHEST_SIZE: usize = 27;

/// `Inventory.INVENTORY_SIZE` (`Inventory.java:30`), the range
/// `Player.getSlot` hands to `inventory.getSlot` (`Player.java:1737-1738`).
const PLAYER_NON_EQUIPMENT_SIZE: i32 = 36;

/// `Player.getSlot`'s slot 499 (`Player.java:1705-1718`).
const CURSOR_SLOT: i32 = 499;

/// The base `Player.getSlot` subtracts for the crafting grid
/// (`Player.java:1720-1721`), and the grid's own size.
const CRAFTING_BASE: i32 = 500;
const CRAFTING_SIZE: i32 = 4;

/// The base `Player.getSlot` subtracts for the ender chest
/// (`Player.java:1741`).
const ENDER_CHEST_BASE: i32 = 200;

/// `EquipmentSlot.getIndex` (`EquipmentSlot.java:53-55`) for the bases
/// `SlotRanges` passes (`SlotRanges.java:27-44`).
const WEAPON_BASE: i32 = 98;
const ARMOR_BASE: i32 = 100;
const BODY_BASE: i32 = 105;
const SADDLE_BASE: i32 = 106;

struct SlotTable {
    by_name: HashMap<String, Vec<i32>>,
    singles: Vec<String>,
}

fn table() -> &'static SlotTable {
    static TABLE: OnceLock<SlotTable> = OnceLock::new();
    TABLE.get_or_init(build_table)
}

fn build_table() -> SlotTable {
    let mut by_name = HashMap::new();
    let mut singles = Vec::new();
    add_single(&mut by_name, &mut singles, "contents", 0);
    add_range(&mut by_name, &mut singles, "container.", 0, 54);
    add_range(&mut by_name, &mut singles, "hotbar.", 0, 9);
    add_range(&mut by_name, &mut singles, "inventory.", 9, 27);
    add_range(&mut by_name, &mut singles, "enderchest.", 200, 27);
    add_range(&mut by_name, &mut singles, "mob.inventory.", 300, 8);
    add_range(&mut by_name, &mut singles, "horse.", 500, 15);
    // `EquipmentSlot.MAINHAND.getIndex(98)` / `OFFHAND.getIndex(98)`
    // (`SlotRanges.java:27-32`).
    add_single(&mut by_name, &mut singles, "weapon", WEAPON_BASE);
    add_single(&mut by_name, &mut singles, "weapon.mainhand", WEAPON_BASE);
    add_single(
        &mut by_name,
        &mut singles,
        "weapon.offhand",
        WEAPON_BASE + 1,
    );
    by_name.insert("weapon.*".to_string(), vec![WEAPON_BASE, WEAPON_BASE + 1]);
    // `HEAD`/`CHEST`/`LEGS`/`FEET`.getIndex(100) (`SlotRanges.java:33-43`).
    add_single(&mut by_name, &mut singles, "armor.head", ARMOR_BASE + 3);
    add_single(&mut by_name, &mut singles, "armor.chest", ARMOR_BASE + 2);
    add_single(&mut by_name, &mut singles, "armor.legs", ARMOR_BASE + 1);
    add_single(&mut by_name, &mut singles, "armor.feet", ARMOR_BASE);
    add_single(&mut by_name, &mut singles, "armor.body", BODY_BASE);
    by_name.insert(
        "armor.*".to_string(),
        vec![
            ARMOR_BASE + 3,
            ARMOR_BASE + 2,
            ARMOR_BASE + 1,
            ARMOR_BASE,
            BODY_BASE,
        ],
    );
    add_single(&mut by_name, &mut singles, "saddle", SADDLE_BASE);
    add_single(&mut by_name, &mut singles, "horse.chest", 499);
    add_single(&mut by_name, &mut singles, "player.cursor", 499);
    add_range(&mut by_name, &mut singles, "player.crafting.", 500, 4);
    SlotTable { by_name, singles }
}

fn add_single(
    by_name: &mut HashMap<String, Vec<i32>>,
    singles: &mut Vec<String>,
    name: &str,
    id: i32,
) {
    by_name.insert(name.to_string(), vec![id]);
    singles.push(name.to_string());
}

/// `addSlotRange` (`SlotRanges.java:68-78`): `prefix + i` then `prefix + "*"`.
fn add_range(
    by_name: &mut HashMap<String, Vec<i32>>,
    singles: &mut Vec<String>,
    prefix: &str,
    offset: i32,
    size: i32,
) {
    let mut all = Vec::with_capacity(usize::try_from(size).unwrap_or(0));
    for i in 0..size {
        let id = offset + i;
        add_single(by_name, singles, &format!("{prefix}{i}"), id);
        all.push(id);
    }
    by_name.insert(format!("{prefix}*"), all);
}

/// `SlotArgument.parse` (`SlotArgument.java:35-45`) after
/// `ParserUtils.readWhile` (`:36`) has taken the name.
pub fn parse_single_slot(name: &str) -> Result<i32, SlotNameError> {
    match table().by_name.get(name) {
        None => Err(SlotNameError::Unknown),
        Some(ids) if ids.len() != 1 => Err(SlotNameError::OnlySingleAllowed),
        Some(ids) => Ok(ids[0]),
    }
}

/// `SlotRanges.singleSlotNames` (`SlotRanges.java:92-94`).
pub fn single_slot_names() -> &'static [String] {
    &table().singles
}

pub fn all_slot_names() -> impl Iterator<Item = &'static str> {
    table().by_name.keys().map(String::as_str)
}

pub fn slot_range(name: &str) -> Option<&'static [i32]> {
    table().by_name.get(name).map(Vec::as_slice)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotNameError {
    Unknown,
    OnlySingleAllowed,
}

#[must_use]
pub fn equipment_slot_for_command_slot(slot: i32) -> Option<EquipmentSlot> {
    Some(match slot {
        98 => EquipmentSlot::MainHand,
        99 => EquipmentSlot::OffHand,
        100 => EquipmentSlot::Feet,
        101 => EquipmentSlot::Legs,
        102 => EquipmentSlot::Chest,
        103 => EquipmentSlot::Head,
        105 => EquipmentSlot::Body,
        106 => EquipmentSlot::Saddle,
        _ => return None,
    })
}

/// The `Inventory` slot an equipment slot occupies, from the generated
/// `EQUIPMENT_SLOT_MAPPING` projection.
#[must_use]
pub fn player_inventory_slot(slot: EquipmentSlot) -> usize {
    let entry = VANILLA_EQUIPMENT_SLOTS
        .iter()
        .find(|entry| entry.slot == slot)
        .expect("the generated catalog carries every equipment slot");
    usize::try_from(
        entry
            .player_inventory_slot
            .expect("every equipment slot the inventory menu shows has an inventory slot"),
    )
    .expect("generated inventory slots are non-negative")
}

/// Which store `Player.getSlot` (`Player.java:1704-1743`) reaches for a
/// command slot on this server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSlot {
    /// `containerMenu.getCarried()` (`Player.java:1708`, `:1713`).
    Cursor,
    /// `inventoryMenu.getCraftSlots()` by grid index (`:1726`, `:1731`).
    Crafting(usize),
    /// `enderChestInventory.getSlot(enderSlot)` (`:1742`) by container index.
    EnderChest(usize),
    /// `inventory.getSlot(slot)` (`:1737-1738`), or the equipment slot
    /// `LivingEntity.getSlot` answers for (`LivingEntity.java:3855-3858`), as
    /// an `Inventory` index.
    Inventory(usize),
}

/// The store and index `Player.getSlot` reaches, in its own order: cursor,
/// then crafting, then the inventory, then the ender chest, then the
/// equipment arm `super.getSlot` reaches.
#[must_use]
pub fn player_slot(slot: i32, selected: u8) -> Option<PlayerSlot> {
    if slot == CURSOR_SLOT {
        return Some(PlayerSlot::Cursor);
    }
    let craft_slot = slot - CRAFTING_BASE;
    if (0..CRAFTING_SIZE).contains(&craft_slot) {
        return usize::try_from(craft_slot).ok().map(PlayerSlot::Crafting);
    }
    if (0..PLAYER_NON_EQUIPMENT_SIZE).contains(&slot) {
        return usize::try_from(slot).ok().map(PlayerSlot::Inventory);
    }
    let ender_slot = slot - ENDER_CHEST_BASE;
    if (0..ENDER_CHEST_SIZE as i32).contains(&ender_slot) {
        return usize::try_from(ender_slot).ok().map(PlayerSlot::EnderChest);
    }
    player_inventory_index(slot, selected).map(PlayerSlot::Inventory)
}

/// The `Inventory` index `Player.getSlot` / `LivingEntity.getSlot` reaches for
/// a player, when this server has that slot.
#[must_use]
pub fn player_inventory_index(slot: i32, selected: u8) -> Option<usize> {
    if (0..PLAYER_NON_EQUIPMENT_SIZE).contains(&slot) {
        return usize::try_from(slot).ok();
    }
    match equipment_slot_for_command_slot(slot)? {
        EquipmentSlot::MainHand => Some(usize::from(selected)),
        other => Some(player_inventory_slot(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_ranges_matches_the_vanilla_table() {
        assert_eq!(parse_single_slot("contents"), Ok(0));
        assert_eq!(parse_single_slot("container.5"), Ok(5));
        assert_eq!(parse_single_slot("hotbar.0"), Ok(0));
        assert_eq!(parse_single_slot("inventory.0"), Ok(9));
        assert_eq!(parse_single_slot("weapon"), Ok(98));
        assert_eq!(parse_single_slot("weapon.mainhand"), Ok(98));
        assert_eq!(parse_single_slot("weapon.offhand"), Ok(99));
        assert_eq!(parse_single_slot("armor.head"), Ok(103));
        assert_eq!(parse_single_slot("armor.chest"), Ok(102));
        assert_eq!(parse_single_slot("armor.legs"), Ok(101));
        assert_eq!(parse_single_slot("armor.feet"), Ok(100));
        assert_eq!(parse_single_slot("armor.body"), Ok(105));
        assert_eq!(parse_single_slot("saddle"), Ok(106));
        assert_eq!(parse_single_slot("player.cursor"), Ok(499));
        assert_eq!(parse_single_slot("player.crafting.0"), Ok(500));
        assert_eq!(parse_single_slot("enderchest.0"), Ok(200));
        assert_eq!(
            parse_single_slot("container.*"),
            Err(SlotNameError::OnlySingleAllowed)
        );
        assert_eq!(
            parse_single_slot("weapon.*"),
            Err(SlotNameError::OnlySingleAllowed)
        );
        assert_eq!(parse_single_slot("not_a_slot"), Err(SlotNameError::Unknown));
        assert!(single_slot_names().iter().any(|name| name == "container.5"));
        assert!(single_slot_names().iter().all(|name| !name.ends_with('*')));
    }

    #[test]
    fn command_slots_map_onto_player_inventory_indexes() {
        assert_eq!(player_inventory_index(0, 3), Some(0));
        assert_eq!(player_inventory_index(35, 3), Some(35));
        assert_eq!(player_inventory_index(98, 3), Some(3));
        assert_eq!(player_inventory_index(99, 3), Some(40));
        assert_eq!(player_inventory_index(102, 3), Some(38));
        assert_eq!(player_inventory_index(499, 3), None);
        assert_eq!(player_inventory_index(200, 3), None);
        assert_eq!(player_inventory_index(500, 3), None);
    }

    /// `Player.getSlot` (`Player.java:1704-1743`) in its own order.
    #[test]
    fn command_slots_name_the_store_player_get_slot_reaches() {
        assert_eq!(player_slot(499, 3), Some(PlayerSlot::Cursor));
        assert_eq!(player_slot(500, 3), Some(PlayerSlot::Crafting(0)));
        assert_eq!(player_slot(503, 3), Some(PlayerSlot::Crafting(3)));
        assert_eq!(player_slot(504, 3), None);
        assert_eq!(player_slot(0, 3), Some(PlayerSlot::Inventory(0)));
        assert_eq!(player_slot(98, 3), Some(PlayerSlot::Inventory(3)));
        assert_eq!(player_slot(105, 3), Some(PlayerSlot::Inventory(41)));
        assert_eq!(player_slot(200, 3), Some(PlayerSlot::EnderChest(0)));
        assert_eq!(player_slot(226, 3), Some(PlayerSlot::EnderChest(26)));
        assert_eq!(player_slot(227, 3), None);
        assert_ne!(player_slot(0, 3), Some(PlayerSlot::EnderChest(0)));
    }
}
