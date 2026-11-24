use super::*;
use serde::ser::{SerializeSeq, SerializeStruct};

impl<SlotId> InventorySlot<SlotId> {
    pub(crate) fn new(slot: SlotId, item: ProtocolItemStackSummary) -> Self {
        Self {
            slot,
            item,
            local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
        }
    }

    pub(crate) fn set(slots: &mut Vec<Self>, mut update: Self)
    where
        SlotId: Ord,
    {
        update.normalize_selection();
        match slots.binary_search_by(|row| row.slot.cmp(&update.slot)) {
            Ok(index) => slots[index] = update,
            Err(index) => slots.insert(index, update),
        }
    }

    pub(super) fn normalize_selection(&mut self) {
        self.local_selected_bundle_item_index = normalize_local_selected_bundle_item_index(
            self.local_selected_bundle_item_index,
            &self.item,
        );
    }
}

pub type ContainerSlotRef<'a> = InventorySlot<i16, &'a ProtocolItemStackSummary>;

impl ContainerSlotRef<'_> {
    pub(super) fn to_owned(self) -> ContainerSlot {
        ContainerSlot {
            slot: self.slot,
            item: self.item.clone(),
            local_selected_bundle_item_index: self.local_selected_bundle_item_index,
        }
    }
}

pub(super) fn container_slot_count(container: &ContainerState) -> i16 {
    const INVENTORY_MENU_SLOT_COUNT: i16 = INVENTORY_MENU_OFFHAND_SLOT + 1; // 46
    const MAX_UNKNOWN_CONTAINER_SLOTS: i16 = 512;
    if container.container_id == INVENTORY_MENU_CONTAINER_ID {
        return INVENTORY_MENU_SLOT_COUNT;
    }
    if container.menu_type_id == Some(VANILLA_MENU_TYPE_CRAFTER_3X3_ID) {
        return CRAFTER_RESULT_SLOT + 1;
    }
    if let Some(start) = player_section_start(container) {
        return start.saturating_add(GENERIC_CONTAINER_PLAYER_SLOT_COUNT);
    }
    if container.menu_type_id == Some(VANILLA_MENU_TYPE_LECTERN_ID) {
        return 1;
    }
    MAX_UNKNOWN_CONTAINER_SLOTS
}

pub(super) fn player_section_start(container: &ContainerState) -> Option<i16> {
    if let Some(mount) = container.mount {
        if mount.inventory_columns < 0 {
            return None;
        }
        let mount_slots = i16::try_from(mount.inventory_columns.checked_mul(3)?).ok()?;
        let start = MOUNT_INVENTORY_START.checked_add(mount_slots)?;
        start.checked_add(GENERIC_CONTAINER_PLAYER_SLOT_COUNT)?;
        return Some(start);
    }
    let menu = container.menu_type_id?;
    Some(match menu {
        VANILLA_MENU_TYPE_GENERIC_9X1_ID..=VANILLA_MENU_TYPE_GENERIC_9X6_ID => {
            (menu - VANILLA_MENU_TYPE_GENERIC_9X1_ID + 1) as i16
                * GENERIC_CONTAINER_SLOT_COUNT_PER_ROW
        }
        VANILLA_MENU_TYPE_GENERIC_3X3_ID => GENERIC_3X3_CONTAINER_SLOT_COUNT,
        VANILLA_MENU_TYPE_CRAFTER_3X3_ID => CRAFTER_PLAYER_MAIN_START,
        VANILLA_MENU_TYPE_CRAFTING_ID => CRAFTING_MENU_PLAYER_MAIN_START,
        VANILLA_MENU_TYPE_FURNACE_ID
        | VANILLA_MENU_TYPE_BLAST_FURNACE_ID
        | VANILLA_MENU_TYPE_SMOKER_ID => FURNACE_CONTAINER_SLOT_COUNT,
        VANILLA_MENU_TYPE_BREWING_STAND_ID => BREWING_STAND_PLAYER_MAIN_START,
        VANILLA_MENU_TYPE_BEACON_ID => BEACON_PLAYER_MAIN_START,
        VANILLA_MENU_TYPE_ENCHANTMENT_ID => ENCHANTMENT_PLAYER_MAIN_START,
        VANILLA_MENU_TYPE_ANVIL_ID => ANVIL_PLAYER_MAIN_START,
        VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID => CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
        VANILLA_MENU_TYPE_GRINDSTONE_ID => GRINDSTONE_PLAYER_MAIN_START,
        VANILLA_MENU_TYPE_HOPPER_ID => HOPPER_CONTAINER_SLOT_COUNT,
        VANILLA_MENU_TYPE_LOOM_ID => LOOM_PLAYER_MAIN_START,
        VANILLA_MENU_TYPE_MERCHANT_ID => MERCHANT_PLAYER_MAIN_START,
        VANILLA_MENU_TYPE_SHULKER_BOX_ID => SHULKER_BOX_CONTAINER_SLOT_COUNT,
        VANILLA_MENU_TYPE_SMITHING_ID => SMITHING_PLAYER_MAIN_START,
        VANILLA_MENU_TYPE_STONECUTTER_ID => STONECUTTER_PLAYER_MAIN_START,
        _ => return None,
    })
}

fn menu_player_slot(container: &ContainerState, slot: i16) -> Option<i32> {
    if container.container_id == INVENTORY_MENU_CONTAINER_ID {
        return inventory_menu_slot_to_inventory_slot(slot);
    }
    let relative = i32::from(slot) - i32::from(player_section_start(container)?);
    match relative {
        0..=26 => Some(relative + 9),
        27..=35 => Some(relative - 27),
        _ => None,
    }
}

impl InventoryState {
    pub fn player_slot(&self, slot: i32) -> Option<&InventorySlot> {
        self.player_slots
            .binary_search_by_key(&slot, |row| row.slot)
            .ok()
            .map(|index| &self.player_slots[index])
    }

    pub(super) fn player_slot_mut(&mut self, slot: i32) -> Option<&mut InventorySlot> {
        let index = self
            .player_slots
            .binary_search_by_key(&slot, |row| row.slot)
            .ok()?;
        Some(&mut self.player_slots[index])
    }

    /// Vanilla `LocalPlayer.getItemBySlot`: local equipment comes from the inventory.
    pub(crate) fn local_player_equipment(
        &self,
        selected_hotbar_slot: u8,
    ) -> impl Iterator<
        Item = (
            &'static EquipmentSlotCatalogEntry,
            &ProtocolItemStackSummary,
        ),
    > + Clone
           + '_ {
        VANILLA_EQUIPMENT_SLOTS.iter().filter_map(move |entry| {
            let slot = local_player_equipment_inventory_slot(selected_hotbar_slot, entry.slot)?;
            self.player_slot(slot).map(|row| (entry, &row.item))
        })
    }

    pub fn container_slot<'a>(
        &'a self,
        container: &'a ContainerState,
        slot: i16,
    ) -> Option<ContainerSlotRef<'a>> {
        if let Some(player) = menu_player_slot(container, slot) {
            return self.player_slot(player).map(|row| ContainerSlotRef {
                slot,
                item: &row.item,
                local_selected_bundle_item_index: row.local_selected_bundle_item_index,
            });
        }
        container
            .slots
            .binary_search_by_key(&slot, |row| row.slot)
            .ok()
            .map(|index| {
                let row = &container.slots[index];
                ContainerSlotRef {
                    slot,
                    item: &row.item,
                    local_selected_bundle_item_index: row.local_selected_bundle_item_index,
                }
            })
    }

    pub fn container_slots<'a>(
        &'a self,
        container: &'a ContainerState,
    ) -> impl Iterator<Item = ContainerSlotRef<'a>> + 'a {
        let range = if container.container_id == INVENTORY_MENU_CONTAINER_ID {
            5..INVENTORY_MENU_OFFHAND_SLOT + 1
        } else {
            player_section_start(container)
                .and_then(|start| {
                    Some(start..start.checked_add(GENERIC_CONTAINER_PLAYER_SLOT_COUNT)?)
                })
                .unwrap_or(0..0)
        };
        let mut player = range
            .filter_map(move |slot| self.container_slot(container, slot))
            .peekable();
        let mut own = container
            .slots
            .iter()
            .map(|row| ContainerSlotRef {
                slot: row.slot,
                item: &row.item,
                local_selected_bundle_item_index: row.local_selected_bundle_item_index,
            })
            .peekable();
        std::iter::from_fn(move || {
            if player
                .peek()
                .is_some_and(|player| own.peek().is_none_or(|own| player.slot < own.slot))
            {
                player.next()
            } else {
                own.next()
            }
        })
    }

    pub(crate) fn set_player_slot(&mut self, slot: InventorySlot) {
        InventorySlot::set(&mut self.player_slots, slot);
    }

    pub(crate) fn set_menu_slot(&mut self, container_id: i32, row: ContainerSlot) {
        if let Some(slot) = menu_player_slot(self.container(container_id).unwrap(), row.slot) {
            self.set_player_slot(InventorySlot {
                slot,
                item: row.item,
                local_selected_bundle_item_index: row.local_selected_bundle_item_index,
            });
        } else {
            ContainerSlot::set(&mut self.container_mut(container_id).slots, row);
        }
    }

    pub(super) fn replace_container_slots(
        &mut self,
        container_id: i32,
        slots: impl IntoIterator<Item = ContainerSlot>,
    ) {
        self.container_mut(container_id).slots.clear();
        for slot in slots {
            self.set_menu_slot(container_id, slot);
        }
    }

    pub(super) fn menu_slot_mut(
        &mut self,
        container_id: i32,
        slot: i16,
    ) -> Option<(&mut ProtocolItemStackSummary, &mut i32)> {
        if let Some(player_slot) = menu_player_slot(self.container(container_id)?, slot) {
            let row = self.player_slot_mut(player_slot)?;
            Some((&mut row.item, &mut row.local_selected_bundle_item_index))
        } else {
            let rows = &mut self.container_mut(container_id).slots;
            let index = rows.binary_search_by_key(&slot, |row| row.slot).ok()?;
            let row = &mut rows[index];
            Some((&mut row.item, &mut row.local_selected_bundle_item_index))
        }
    }

    pub(super) fn container(&self, id: i32) -> Option<&ContainerState> {
        if id == INVENTORY_MENU_CONTAINER_ID {
            Some(&self.inventory_menu)
        } else {
            self.open_container
                .as_ref()
                .filter(|container| container.container_id == id)
        }
    }

    pub(super) fn container_mut(&mut self, id: i32) -> &mut ContainerState {
        if id == INVENTORY_MENU_CONTAINER_ID {
            &mut self.inventory_menu
        } else {
            self.open_container.as_mut().unwrap()
        }
    }

    pub(super) fn container_slot_is_valid(&self, container: &ContainerState, slot: i16) -> bool {
        matches!(slot, -1 | -999) || self.container_slot(container, slot).is_some()
    }
}

#[derive(Deserialize)]
pub(super) struct InventoryWire {
    player_slots: Vec<InventorySlot>,
    cursor_item: ProtocolItemStackSummary,
    #[serde(default = "default_inventory_menu")]
    inventory_menu: ContainerState,
    #[serde(default)]
    local_inventory_open: bool,
    open_container: Option<ContainerState>,
    #[serde(default)]
    open_container_revision: u64,
    #[serde(default)]
    local_quick_craft: LocalQuickCraftState,
}

impl From<InventoryWire> for InventoryState {
    fn from(wire: InventoryWire) -> Self {
        let mut inventory = Self {
            player_slots: wire.player_slots,
            cursor_item: wire.cursor_item,
            inventory_menu: wire.inventory_menu,
            local_inventory_open: wire.local_inventory_open,
            open_container: wire.open_container,
            open_container_revision: wire.open_container_revision,
            local_quick_craft: wire.local_quick_craft,
        };
        let container_ids = [
            inventory
                .open_container
                .as_ref()
                .map(|container| container.container_id),
            Some(INVENTORY_MENU_CONTAINER_ID),
        ];
        for id in container_ids.into_iter().flatten() {
            let rows = std::mem::take(&mut inventory.container_mut(id).slots);
            for row in rows {
                if menu_player_slot(inventory.container(id).unwrap(), row.slot)
                    .is_some_and(|slot| inventory.player_slot(slot).is_some())
                {
                    continue;
                }
                inventory.set_menu_slot(id, row);
            }
        }
        inventory
    }
}

struct ContainerView<'a>(&'a InventoryState, &'a ContainerState);
struct SlotsView<'a>(&'a InventoryState, &'a ContainerState);

impl Serialize for SlotsView<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(None)?;
        for row in self.0.container_slots(self.1) {
            seq.serialize_element(&row)?;
        }
        seq.end()
    }
}

impl Serialize for ContainerView<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let container = self.1;
        let mut state = serializer.serialize_struct("ContainerState", 8)?;
        state.serialize_field("container_id", &container.container_id)?;
        state.serialize_field("menu_type_id", &container.menu_type_id)?;
        state.serialize_field("title", &container.title)?;
        state.serialize_field("mount", &container.mount)?;
        state.serialize_field("state_id", &container.state_id)?;
        state.serialize_field("slots", &SlotsView(self.0, container))?;
        state.serialize_field("data_values", &container.data_values)?;
        state.serialize_field("merchant_offers", &container.merchant_offers)?;
        state.end()
    }
}

impl Serialize for InventoryState {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("InventoryState", 6)?;
        state.serialize_field("player_slots", &self.player_slots)?;
        state.serialize_field("cursor_item", &self.cursor_item)?;
        state.serialize_field("inventory_menu", &ContainerView(self, &self.inventory_menu))?;
        state.serialize_field("local_inventory_open", &self.local_inventory_open)?;
        state.serialize_field(
            "open_container",
            &self
                .open_container
                .as_ref()
                .map(|container| ContainerView(self, container)),
        )?;
        state.serialize_field("open_container_revision", &self.open_container_revision)?;
        state.end()
    }
}
