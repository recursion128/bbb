use bbb_protocol::packets::{
    ContainerClick as ProtocolContainerClick, ContainerClose as ProtocolContainerClose,
    ContainerInput as ProtocolContainerInput, ContainerSetContent as ProtocolContainerSetContent,
    ContainerSetData as ProtocolContainerSetData, ContainerSetSlot as ProtocolContainerSetSlot,
    DataComponentPatchSummary as ProtocolDataComponentPatchSummary,
    HashedComponentPatch as ProtocolHashedComponentPatch,
    HashedItemStack as ProtocolHashedItemStack, HashedStack as ProtocolHashedStack,
    ItemCostSummary as ProtocolItemCostSummary, ItemStackSummary as ProtocolItemStackSummary,
    MerchantOffer as ProtocolMerchantOffer, MerchantOffers as ProtocolMerchantOffers,
    OpenScreen as ProtocolOpenScreen, SetCursorItem as ProtocolSetCursorItem,
    SetPlayerInventory as ProtocolSetPlayerInventory,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::WorldStore;

const VANILLA_MENU_TYPE_MERCHANT_ID: i32 = 19;
const PLAYER_HOTBAR_SIZE: usize = 9;
const PLAYER_CHEST_EQUIPMENT_SLOT: i32 = 38;
const PLAYER_OFFHAND_SLOT: i32 = 40;
const INVENTORY_MENU_CONTAINER_ID: i32 = 0;
const INVENTORY_MENU_HOTBAR_START: i16 = 36;
const INVENTORY_MENU_OFFHAND_SLOT: i16 = 45;
const NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX: i32 = -1;
const VANILLA_ELYTRA_ITEM_ID: i32 = 14;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventorySlot {
    pub slot: i32,
    pub item: ProtocolItemStackSummary,
    #[serde(default = "default_local_selected_bundle_item_index")]
    pub local_selected_bundle_item_index: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerSlot {
    pub slot: i16,
    pub item: ProtocolItemStackSummary,
    #[serde(default = "default_local_selected_bundle_item_index")]
    pub local_selected_bundle_item_index: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotbarItemState {
    pub item: ProtocolItemStackSummary,
    pub local_selected_bundle_item_index: i32,
}

impl Default for HotbarItemState {
    fn default() -> Self {
        Self {
            item: ProtocolItemStackSummary::empty(),
            local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
        }
    }
}

impl HotbarItemState {
    pub fn local_selected_bundle_item_index(&self) -> Option<i32> {
        (self.local_selected_bundle_item_index >= 0)
            .then_some(self.local_selected_bundle_item_index)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerDataValue {
    pub id: i16,
    pub value: i16,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ContainerState {
    pub container_id: i32,
    pub menu_type_id: Option<i32>,
    pub title: Option<String>,
    pub state_id: i32,
    pub slots: Vec<ContainerSlot>,
    pub data_values: Vec<ContainerDataValue>,
    pub merchant_offers: Option<MerchantOffersState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryState {
    pub player_slots: Vec<InventorySlot>,
    pub cursor_item: ProtocolItemStackSummary,
    #[serde(default = "default_inventory_menu")]
    pub inventory_menu: ContainerState,
    #[serde(default)]
    pub local_inventory_open: bool,
    pub open_container: Option<ContainerState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MerchantOffersState {
    pub container_id: i32,
    pub offers: Vec<MerchantOfferState>,
    pub villager_level: i32,
    pub villager_xp: i32,
    pub show_progress: bool,
    pub can_restock: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MerchantOfferState {
    pub buy_a: ProtocolItemCostSummary,
    pub sell: ProtocolItemStackSummary,
    pub buy_b: Option<ProtocolItemCostSummary>,
    pub is_out_of_stock: bool,
    pub uses: i32,
    pub max_uses: i32,
    pub xp: i32,
    pub special_price_diff: i32,
    pub price_multiplier: f32,
    pub demand: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerClickSlotRequest {
    pub slot_num: i16,
    pub button_num: i8,
    pub input: ProtocolContainerInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerClickBuildError {
    NoOpenContainer,
    InvalidSlot(i16),
    UnhashableCarriedItem,
}

impl Default for InventoryState {
    fn default() -> Self {
        Self {
            player_slots: Vec::new(),
            cursor_item: ProtocolItemStackSummary::empty(),
            inventory_menu: default_inventory_menu(),
            local_inventory_open: false,
            open_container: None,
        }
    }
}

impl InventoryState {
    pub fn hotbar_items(&self) -> [ProtocolItemStackSummary; PLAYER_HOTBAR_SIZE] {
        self.hotbar_item_states().map(|state| state.item)
    }

    pub fn hotbar_item_states(&self) -> [HotbarItemState; PLAYER_HOTBAR_SIZE] {
        let mut items = std::array::from_fn(|_| HotbarItemState::default());
        for slot in &self.player_slots {
            let Ok(slot_index) = usize::try_from(slot.slot) else {
                continue;
            };
            if slot_index < PLAYER_HOTBAR_SIZE {
                items[slot_index] = HotbarItemState {
                    item: slot.item.clone(),
                    local_selected_bundle_item_index: slot.local_selected_bundle_item_index,
                };
            }
        }

        for slot in &self.inventory_menu.slots {
            let Some(index) = slot.slot.checked_sub(INVENTORY_MENU_HOTBAR_START) else {
                continue;
            };
            let Ok(slot_index) = usize::try_from(index) else {
                continue;
            };
            if slot_index < PLAYER_HOTBAR_SIZE {
                items[slot_index] = HotbarItemState {
                    item: slot.item.clone(),
                    local_selected_bundle_item_index: slot.local_selected_bundle_item_index,
                };
            }
        }

        items
    }
}

impl WorldStore {
    pub fn apply_set_player_inventory(&mut self, packet: ProtocolSetPlayerInventory) {
        self.counters.inventory_slot_updates_received += 1;
        let slot_id = packet.slot;
        let menu_slot = inventory_slot_to_inventory_menu_slot(slot_id);
        let item = packet.item;
        set_inventory_slot(
            &mut self.inventory.player_slots,
            InventorySlot {
                slot: slot_id,
                item: item.clone(),
                local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
            },
        );
        if let Some(menu_slot) = menu_slot {
            set_container_slot(
                &mut self.inventory.inventory_menu.slots,
                ContainerSlot {
                    slot: menu_slot,
                    item,
                    local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
                },
            );
        }
        self.update_inventory_slot_count();
    }

    pub fn apply_set_cursor_item(&mut self, packet: ProtocolSetCursorItem) {
        self.counters.cursor_item_updates_received += 1;
        self.inventory.cursor_item = packet.item;
    }

    pub fn apply_open_screen(&mut self, packet: ProtocolOpenScreen) {
        self.counters.container_open_updates_received += 1;
        self.inventory.local_inventory_open = false;
        let existing = self
            .inventory
            .open_container
            .take()
            .filter(|container| container.container_id == packet.container_id)
            .unwrap_or_else(|| ContainerState {
                container_id: packet.container_id,
                ..ContainerState::default()
            });
        self.inventory.open_container = Some(ContainerState {
            container_id: packet.container_id,
            menu_type_id: Some(packet.menu_type_id),
            title: Some(packet.title),
            state_id: existing.state_id,
            slots: existing.slots,
            data_values: existing.data_values,
            merchant_offers: existing.merchant_offers,
        });
        self.update_merchant_offer_count();
    }

    pub fn apply_container_set_content(&mut self, packet: ProtocolContainerSetContent) {
        self.counters.container_content_updates_received += 1;
        let ProtocolContainerSetContent {
            container_id,
            state_id,
            items,
            carried_item,
        } = packet;
        self.inventory.cursor_item = carried_item;
        let slots = items
            .into_iter()
            .enumerate()
            .map(|(slot, item)| ContainerSlot {
                slot: slot as i16,
                local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
                item,
            })
            .collect();

        if container_id == INVENTORY_MENU_CONTAINER_ID {
            let existing =
                std::mem::replace(&mut self.inventory.inventory_menu, default_inventory_menu());
            self.inventory.inventory_menu = ContainerState {
                container_id,
                menu_type_id: existing.menu_type_id,
                title: existing.title,
                state_id,
                slots,
                data_values: existing.data_values,
                merchant_offers: existing.merchant_offers,
            };
            return;
        }

        let existing = self
            .inventory
            .open_container
            .take()
            .filter(|container| container.container_id == container_id);
        let merchant_offers = existing
            .as_ref()
            .and_then(|container| container.merchant_offers.clone());
        self.inventory.open_container = Some(ContainerState {
            container_id,
            menu_type_id: existing
                .as_ref()
                .and_then(|container| container.menu_type_id),
            title: existing
                .as_ref()
                .and_then(|container| container.title.clone()),
            state_id,
            slots,
            data_values: existing
                .as_ref()
                .map(|container| container.data_values.clone())
                .unwrap_or_default(),
            merchant_offers,
        });
        self.update_merchant_offer_count();
    }

    pub fn apply_merchant_offers(&mut self, packet: ProtocolMerchantOffers) -> bool {
        self.counters.merchant_offer_packets_received += 1;
        let Some(container) = self.inventory.open_container.as_mut().filter(|container| {
            container.container_id == packet.container_id
                && container.menu_type_id == Some(VANILLA_MENU_TYPE_MERCHANT_ID)
        }) else {
            self.counters.merchant_offer_packets_ignored += 1;
            return false;
        };

        let offer_count = packet.offers.len();
        container.merchant_offers = Some(MerchantOffersState::from_packet(packet));
        self.counters.merchant_offer_packets_applied += 1;
        self.counters.merchant_offers_tracked = offer_count;
        true
    }

    pub fn apply_container_set_slot(&mut self, packet: ProtocolContainerSetSlot) {
        self.counters.container_slot_updates_received += 1;
        let container = if packet.container_id == INVENTORY_MENU_CONTAINER_ID {
            &mut self.inventory.inventory_menu
        } else {
            self.ensure_container(packet.container_id)
        };
        container.state_id = packet.state_id;
        set_container_slot(
            &mut container.slots,
            ContainerSlot {
                slot: packet.slot,
                item: packet.item,
                local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
            },
        );
        self.update_merchant_offer_count();
    }

    pub fn apply_container_set_data(&mut self, packet: ProtocolContainerSetData) {
        self.counters.container_data_updates_received += 1;
        let container = self.ensure_container(packet.container_id);
        if let Some(existing) = container
            .data_values
            .iter_mut()
            .find(|value| value.id == packet.id)
        {
            *existing = ContainerDataValue {
                id: packet.id,
                value: packet.value,
            };
        } else {
            container.data_values.push(ContainerDataValue {
                id: packet.id,
                value: packet.value,
            });
        }
        container.data_values.sort_by_key(|value| value.id);
        self.update_merchant_offer_count();
    }

    pub fn apply_container_close(&mut self, packet: ProtocolContainerClose) -> bool {
        self.counters.container_close_updates_received += 1;
        if packet.container_id == INVENTORY_MENU_CONTAINER_ID {
            if self.inventory.local_inventory_open {
                self.inventory.local_inventory_open = false;
                self.counters.container_close_updates_applied += 1;
                return true;
            }
            self.counters.container_close_updates_ignored += 1;
            return false;
        }

        if self
            .inventory
            .open_container
            .as_ref()
            .is_some_and(|container| container.container_id == packet.container_id)
        {
            self.inventory.open_container = None;
            self.counters.merchant_offers_tracked = 0;
            self.counters.container_close_updates_applied += 1;
            true
        } else {
            self.counters.container_close_updates_ignored += 1;
            false
        }
    }

    pub fn close_local_container(&mut self, container_id: i32) -> bool {
        if container_id == INVENTORY_MENU_CONTAINER_ID {
            if self.inventory.local_inventory_open {
                self.inventory.local_inventory_open = false;
                return true;
            }
            return false;
        }

        if self
            .inventory
            .open_container
            .as_ref()
            .is_some_and(|container| container.container_id == container_id)
        {
            self.inventory.open_container = None;
            self.counters.merchant_offers_tracked = 0;
            true
        } else {
            false
        }
    }

    pub fn open_local_inventory(&mut self) -> bool {
        if self.inventory.open_container.is_some() {
            return false;
        }
        self.sync_inventory_menu_slots_from_player_inventory();
        self.ensure_inventory_menu_slot_shape();
        let was_open = self.inventory.local_inventory_open;
        self.inventory.local_inventory_open = true;
        !was_open
    }

    pub fn local_inventory_is_open(&self) -> bool {
        self.inventory.local_inventory_open
    }

    pub fn open_container_id(&self) -> Option<i32> {
        self.inventory
            .open_container
            .as_ref()
            .map(|container| container.container_id)
            .or_else(|| {
                self.inventory
                    .local_inventory_open
                    .then_some(INVENTORY_MENU_CONTAINER_ID)
            })
    }

    pub fn apply_local_select_bundle_item(
        &mut self,
        slot_id: i32,
        selected_item_index: i32,
    ) -> bool {
        if selected_item_index < NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX {
            return false;
        }

        if let Some(container) = self.inventory.open_container.as_mut() {
            let Ok(slot_id) = i16::try_from(slot_id) else {
                return false;
            };
            let Some(slot) = container.slots.iter_mut().find(|slot| slot.slot == slot_id) else {
                return false;
            };
            return apply_local_selected_bundle_item_index(
                &slot.item,
                &mut slot.local_selected_bundle_item_index,
                selected_item_index,
            );
        }
        if self.inventory.local_inventory_open {
            let Ok(slot_id) = i16::try_from(slot_id) else {
                return false;
            };
            let Some(slot) = self
                .inventory
                .inventory_menu
                .slots
                .iter_mut()
                .find(|slot| slot.slot == slot_id)
            else {
                return false;
            };
            return apply_local_selected_bundle_item_index(
                &slot.item,
                &mut slot.local_selected_bundle_item_index,
                selected_item_index,
            );
        }

        let Some((applied, local_selected_bundle_item_index)) = self
            .inventory
            .player_slots
            .iter_mut()
            .find(|slot| slot.slot == slot_id)
            .map(|slot| {
                let applied = apply_local_selected_bundle_item_index(
                    &slot.item,
                    &mut slot.local_selected_bundle_item_index,
                    selected_item_index,
                );
                (applied, slot.local_selected_bundle_item_index)
            })
        else {
            return false;
        };
        if applied {
            if let Some(menu_slot_id) = inventory_slot_to_inventory_menu_slot(slot_id) {
                if let Some(menu_slot) = self
                    .inventory
                    .inventory_menu
                    .slots
                    .iter_mut()
                    .find(|slot| slot.slot == menu_slot_id)
                {
                    menu_slot.local_selected_bundle_item_index = local_selected_bundle_item_index;
                }
            }
        }
        applied
    }

    pub fn inventory(&self) -> &InventoryState {
        &self.inventory
    }

    pub fn local_item_use_prefers_offhand(&self) -> bool {
        let hotbar_items = self.inventory.hotbar_items();
        let selected_slot = usize::from(self.local_player.selected_hotbar_slot.min(8));
        !item_stack_is_non_empty(&hotbar_items[selected_slot])
            && self
                .local_offhand_item()
                .is_some_and(item_stack_is_non_empty)
    }

    pub fn local_player_has_equipped_elytra(&self) -> bool {
        self.inventory
            .player_slots
            .iter()
            .find(|slot| slot.slot == PLAYER_CHEST_EQUIPMENT_SLOT)
            .is_some_and(|slot| {
                slot.item.item_id == Some(VANILLA_ELYTRA_ITEM_ID)
                    && item_stack_is_non_empty(&slot.item)
            })
    }

    pub fn build_container_click_slot(
        &self,
        request: ContainerClickSlotRequest,
    ) -> Result<ProtocolContainerClick, ContainerClickBuildError> {
        let Some(container) = self.active_container() else {
            return Err(ContainerClickBuildError::NoOpenContainer);
        };
        if !container_click_slot_is_valid(container, request.slot_num) {
            return Err(ContainerClickBuildError::InvalidSlot(request.slot_num));
        }

        let carried_item = hashed_stack_from_summary(&self.inventory.cursor_item)
            .ok_or(ContainerClickBuildError::UnhashableCarriedItem)?;
        Ok(ProtocolContainerClick {
            container_id: container.container_id,
            state_id: container.state_id,
            slot_num: request.slot_num,
            button_num: request.button_num,
            input: request.input,
            changed_slots: BTreeMap::new(),
            carried_item,
        })
    }

    fn local_offhand_item(&self) -> Option<&ProtocolItemStackSummary> {
        if let Some(item) = self
            .inventory
            .player_slots
            .iter()
            .find_map(|slot| (slot.slot == PLAYER_OFFHAND_SLOT).then_some(&slot.item))
        {
            return Some(item);
        }

        self.inventory
            .inventory_menu
            .slots
            .iter()
            .find_map(|slot| (slot.slot == INVENTORY_MENU_OFFHAND_SLOT).then_some(&slot.item))
    }

    fn active_container(&self) -> Option<&ContainerState> {
        self.inventory.open_container.as_ref().or_else(|| {
            self.inventory
                .local_inventory_open
                .then_some(&self.inventory.inventory_menu)
        })
    }

    fn ensure_inventory_menu_slot_shape(&mut self) {
        for slot in 0..=INVENTORY_MENU_OFFHAND_SLOT {
            if self
                .inventory
                .inventory_menu
                .slots
                .iter()
                .all(|existing| existing.slot != slot)
            {
                set_container_slot(
                    &mut self.inventory.inventory_menu.slots,
                    ContainerSlot {
                        slot,
                        item: ProtocolItemStackSummary::empty(),
                        local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
                    },
                );
            }
        }
    }

    fn sync_inventory_menu_slots_from_player_inventory(&mut self) {
        for slot in self.inventory.player_slots.clone() {
            let Some(menu_slot) = inventory_slot_to_inventory_menu_slot(slot.slot) else {
                continue;
            };
            if self
                .inventory
                .inventory_menu
                .slots
                .iter()
                .any(|slot| slot.slot == menu_slot)
            {
                continue;
            }
            set_container_slot(
                &mut self.inventory.inventory_menu.slots,
                ContainerSlot {
                    slot: menu_slot,
                    item: slot.item,
                    local_selected_bundle_item_index: slot.local_selected_bundle_item_index,
                },
            );
        }
    }

    fn inventory_menu_container(&mut self) -> &mut ContainerState {
        if self.inventory.inventory_menu.container_id != INVENTORY_MENU_CONTAINER_ID {
            self.inventory.inventory_menu = default_inventory_menu();
        }
        &mut self.inventory.inventory_menu
    }

    fn ensure_container(&mut self, container_id: i32) -> &mut ContainerState {
        if container_id == INVENTORY_MENU_CONTAINER_ID {
            return self.inventory_menu_container();
        }

        if self
            .inventory
            .open_container
            .as_ref()
            .is_none_or(|container| container.container_id != container_id)
        {
            self.inventory.open_container = Some(ContainerState {
                container_id,
                ..ContainerState::default()
            })
        }
        self.inventory
            .open_container
            .as_mut()
            .expect("container was initialized")
    }

    fn update_inventory_slot_count(&mut self) {
        self.counters.inventory_slots_tracked = self.inventory.player_slots.len();
    }

    fn update_merchant_offer_count(&mut self) {
        self.counters.merchant_offers_tracked = self
            .inventory
            .open_container
            .as_ref()
            .and_then(|container| container.merchant_offers.as_ref())
            .map(|offers| offers.offers.len())
            .unwrap_or(0);
    }
}

fn container_click_slot_is_valid(container: &ContainerState, slot_num: i16) -> bool {
    matches!(slot_num, -1 | -999) || container.slots.iter().any(|slot| slot.slot == slot_num)
}

fn hashed_stack_from_summary(stack: &ProtocolItemStackSummary) -> Option<ProtocolHashedStack> {
    let (Some(item_id), true) = (stack.item_id, stack.count > 0) else {
        return Some(ProtocolHashedStack::Empty);
    };
    if !component_patch_can_be_hashed_from_summary(&stack.component_patch) {
        return None;
    }
    Some(ProtocolHashedStack::Item(ProtocolHashedItemStack {
        item_id,
        count: stack.count,
        components: ProtocolHashedComponentPatch::default(),
    }))
}

fn item_stack_is_non_empty(stack: &ProtocolItemStackSummary) -> bool {
    stack.item_id.is_some() && stack.count > 0
}

fn component_patch_can_be_hashed_from_summary(patch: &ProtocolDataComponentPatchSummary) -> bool {
    patch == &ProtocolDataComponentPatchSummary::default()
}

fn set_inventory_slot(slots: &mut Vec<InventorySlot>, mut update: InventorySlot) {
    update.local_selected_bundle_item_index = normalize_local_selected_bundle_item_index(
        update.local_selected_bundle_item_index,
        &update.item,
    );
    if let Some(existing) = slots.iter_mut().find(|slot| slot.slot == update.slot) {
        *existing = update;
    } else {
        slots.push(update);
    }
    slots.sort_by_key(|slot| slot.slot);
}

fn set_container_slot(slots: &mut Vec<ContainerSlot>, mut update: ContainerSlot) {
    update.local_selected_bundle_item_index = normalize_local_selected_bundle_item_index(
        update.local_selected_bundle_item_index,
        &update.item,
    );
    if let Some(existing) = slots.iter_mut().find(|slot| slot.slot == update.slot) {
        *existing = update;
    } else {
        slots.push(update);
    }
    slots.sort_by_key(|slot| slot.slot);
}

fn default_local_selected_bundle_item_index() -> i32 {
    NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
}

fn default_inventory_menu() -> ContainerState {
    ContainerState {
        container_id: INVENTORY_MENU_CONTAINER_ID,
        ..ContainerState::default()
    }
}

fn inventory_slot_to_inventory_menu_slot(slot: i32) -> Option<i16> {
    match slot {
        0..=8 => Some(INVENTORY_MENU_HOTBAR_START + slot as i16),
        9..=35 => Some(slot as i16),
        36..=39 => Some((44 - slot) as i16),
        PLAYER_OFFHAND_SLOT => Some(INVENTORY_MENU_OFFHAND_SLOT),
        _ => None,
    }
}

fn apply_local_selected_bundle_item_index(
    item: &ProtocolItemStackSummary,
    current_selected_item_index: &mut i32,
    selected_item_index: i32,
) -> bool {
    let Some(bundle_item_count) = item.component_patch.bundle_contents_item_count else {
        return false;
    };

    *current_selected_item_index = if selected_item_index == NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
        || selected_item_index == *current_selected_item_index
        || usize::try_from(selected_item_index)
            .map(|index| index >= bundle_item_count)
            .unwrap_or(true)
    {
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    } else {
        selected_item_index
    };
    true
}

fn normalize_local_selected_bundle_item_index(
    selected_item_index: i32,
    item: &ProtocolItemStackSummary,
) -> i32 {
    let Some(bundle_item_count) = item.component_patch.bundle_contents_item_count else {
        return NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX;
    };
    let Ok(selected_item_index_usize) = usize::try_from(selected_item_index) else {
        return NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX;
    };
    if selected_item_index_usize < bundle_item_count {
        selected_item_index
    } else {
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    }
}

impl MerchantOffersState {
    fn from_packet(packet: ProtocolMerchantOffers) -> Self {
        Self {
            container_id: packet.container_id,
            offers: packet
                .offers
                .into_iter()
                .map(MerchantOfferState::from_packet)
                .collect(),
            villager_level: packet.villager_level,
            villager_xp: packet.villager_xp,
            show_progress: packet.show_progress,
            can_restock: packet.can_restock,
        }
    }
}

impl MerchantOfferState {
    fn from_packet(packet: ProtocolMerchantOffer) -> Self {
        Self {
            buy_a: packet.buy_a,
            sell: packet.sell,
            buy_b: packet.buy_b,
            is_out_of_stock: packet.is_out_of_stock,
            uses: packet.uses,
            max_uses: packet.max_uses,
            xp: packet.xp,
            special_price_diff: packet.special_price_diff,
            price_multiplier: packet.price_multiplier,
            demand: packet.demand,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_player_inventory_and_container_state() {
        let mut store = WorldStore::new();

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 36,
            item: item_stack(42, 1),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 36,
            item: item_stack(43, 2),
        });
        store.apply_set_cursor_item(ProtocolSetCursorItem {
            item: item_stack(99, 1),
        });

        assert_eq!(
            store.inventory().player_slots,
            vec![InventorySlot {
                slot: 36,
                item: item_stack(43, 2),
                local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
            }]
        );
        assert_eq!(store.inventory().cursor_item, item_stack(99, 1));

        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: 2,
            title: "Chest".to_string(),
        });
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![ProtocolItemStackSummary::empty(), item_stack(42, 64)],
            carried_item: ProtocolItemStackSummary::empty(),
        });
        store.apply_container_set_slot(ProtocolContainerSetSlot {
            container_id: 7,
            state_id: 13,
            slot: 1,
            item: item_stack(44, 3),
        });
        store.apply_container_set_data(ProtocolContainerSetData {
            container_id: 7,
            id: 2,
            value: 9,
        });
        store.apply_container_set_data(ProtocolContainerSetData {
            container_id: 7,
            id: 2,
            value: 10,
        });

        let container = store.inventory().open_container.as_ref().unwrap();
        assert_eq!(container.container_id, 7);
        assert_eq!(container.menu_type_id, Some(2));
        assert_eq!(container.title.as_deref(), Some("Chest"));
        assert_eq!(container.state_id, 13);
        assert_eq!(
            container.slots,
            vec![
                ContainerSlot {
                    slot: 0,
                    item: ProtocolItemStackSummary::empty(),
                    local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
                },
                ContainerSlot {
                    slot: 1,
                    item: item_stack(44, 3),
                    local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
                },
            ]
        );
        assert_eq!(
            container.data_values,
            vec![ContainerDataValue { id: 2, value: 10 }]
        );
        assert_eq!(
            store.inventory().cursor_item,
            ProtocolItemStackSummary::empty()
        );

        assert!(store.apply_container_close(ProtocolContainerClose { container_id: 7 }));
        assert!(store.inventory().open_container.is_none());
        assert!(!store.apply_container_close(ProtocolContainerClose { container_id: 99 }));

        assert_eq!(store.counters().inventory_slot_updates_received, 2);
        assert_eq!(store.counters().inventory_slots_tracked, 1);
        assert_eq!(store.counters().cursor_item_updates_received, 1);
        assert_eq!(store.counters().container_open_updates_received, 1);
        assert_eq!(store.counters().container_content_updates_received, 1);
        assert_eq!(store.counters().container_slot_updates_received, 1);
        assert_eq!(store.counters().container_data_updates_received, 2);
        assert_eq!(store.counters().container_close_updates_received, 2);
        assert_eq!(store.counters().container_close_updates_applied, 1);
        assert_eq!(store.counters().container_close_updates_ignored, 1);
    }

    #[test]
    fn hotbar_items_merge_player_inventory_and_inventory_menu_slots() {
        let mut store = WorldStore::new();

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(10, 1),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 8,
            item: item_stack(18, 1),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 9,
            item: item_stack(99, 1),
        });
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 0,
            state_id: 1,
            items: (0..45)
                .map(|slot| {
                    if slot == 36 {
                        item_stack(20, 2)
                    } else if slot == 44 {
                        item_stack(28, 2)
                    } else {
                        ProtocolItemStackSummary::empty()
                    }
                })
                .collect(),
            carried_item: ProtocolItemStackSummary::empty(),
        });
        store.apply_container_set_slot(ProtocolContainerSetSlot {
            container_id: 0,
            state_id: 2,
            slot: 40,
            item: item_stack(24, 3),
        });

        let hotbar = store.inventory().hotbar_items();
        assert_eq!(hotbar[0], item_stack(20, 2));
        assert_eq!(hotbar[4], item_stack(24, 3));
        assert_eq!(hotbar[8], item_stack(28, 2));
        assert_eq!(hotbar[1], ProtocolItemStackSummary::empty());
    }

    #[test]
    fn container_zero_content_updates_inventory_menu_without_opening_local_screen() {
        let mut store = WorldStore::new();

        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: INVENTORY_MENU_CONTAINER_ID,
            state_id: 7,
            items: vec![ProtocolItemStackSummary::empty(), item_stack(42, 3)],
            carried_item: item_stack(99, 1),
        });

        assert!(!store.local_inventory_is_open());
        assert_eq!(store.open_container_id(), None);
        assert!(store.inventory().open_container.is_none());
        assert_eq!(store.inventory().inventory_menu.container_id, 0);
        assert_eq!(store.inventory().inventory_menu.state_id, 7);
        assert_eq!(
            store.inventory().inventory_menu.slots[1].item,
            item_stack(42, 3)
        );
        assert_eq!(store.inventory().cursor_item, item_stack(99, 1));
    }

    #[test]
    fn open_local_inventory_builds_container_zero_view_from_player_inventory() {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(10, 1),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: PLAYER_CHEST_EQUIPMENT_SLOT,
            item: item_stack(VANILLA_ELYTRA_ITEM_ID, 1),
        });
        store.apply_set_cursor_item(ProtocolSetCursorItem {
            item: item_stack(99, 1),
        });

        assert!(store.open_local_inventory());
        assert!(!store.open_local_inventory());
        assert!(store.local_inventory_is_open());
        assert_eq!(store.open_container_id(), Some(INVENTORY_MENU_CONTAINER_ID));

        let inventory_menu = &store.inventory().inventory_menu;
        assert_eq!(inventory_menu.container_id, INVENTORY_MENU_CONTAINER_ID);
        assert_eq!(inventory_menu.slots.len(), 46);
        assert_eq!(
            inventory_menu
                .slots
                .iter()
                .find(|slot| slot.slot == INVENTORY_MENU_HOTBAR_START)
                .unwrap()
                .item,
            item_stack(10, 1)
        );
        assert_eq!(
            inventory_menu
                .slots
                .iter()
                .find(|slot| slot.slot == 6)
                .unwrap()
                .item,
            item_stack(VANILLA_ELYTRA_ITEM_ID, 1)
        );

        let click = store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_HOTBAR_START,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();
        assert_eq!(click.container_id, INVENTORY_MENU_CONTAINER_ID);
        assert_eq!(click.slot_num, INVENTORY_MENU_HOTBAR_START);
        assert_eq!(
            click.carried_item,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 99,
                count: 1,
                components: ProtocolHashedComponentPatch::default(),
            })
        );
    }

    #[test]
    fn local_item_use_prefers_offhand_only_when_selected_hotbar_slot_is_empty() {
        let mut store = WorldStore::new();

        assert!(!store.local_item_use_prefers_offhand());

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: PLAYER_OFFHAND_SLOT,
            item: item_stack(99, 1),
        });
        assert!(store.local_item_use_prefers_offhand());

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(10, 1),
        });
        assert!(!store.local_item_use_prefers_offhand());

        assert!(store.set_local_selected_hotbar_slot(1));
        assert!(store.local_item_use_prefers_offhand());

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: PLAYER_OFFHAND_SLOT,
            item: ProtocolItemStackSummary::empty(),
        });
        assert!(!store.local_item_use_prefers_offhand());
    }

    #[test]
    fn local_item_use_reads_inventory_menu_offhand_slot_when_player_slot_is_absent() {
        let mut store = WorldStore::new();

        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: INVENTORY_MENU_CONTAINER_ID,
            state_id: 1,
            items: (0..46)
                .map(|slot| {
                    if slot == INVENTORY_MENU_OFFHAND_SLOT {
                        item_stack(99, 1)
                    } else {
                        ProtocolItemStackSummary::empty()
                    }
                })
                .collect(),
            carried_item: ProtocolItemStackSummary::empty(),
        });

        assert!(store.local_item_use_prefers_offhand());

        store.apply_container_set_slot(ProtocolContainerSetSlot {
            container_id: INVENTORY_MENU_CONTAINER_ID,
            state_id: 2,
            slot: INVENTORY_MENU_OFFHAND_SLOT,
            item: ProtocolItemStackSummary::empty(),
        });
        assert!(!store.local_item_use_prefers_offhand());
    }

    #[test]
    fn local_player_has_equipped_elytra_true_for_non_empty_elytra_in_chest_slot() {
        let mut store = WorldStore::new();

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: PLAYER_CHEST_EQUIPMENT_SLOT,
            item: item_stack(VANILLA_ELYTRA_ITEM_ID, 1),
        });

        assert!(store.local_player_has_equipped_elytra());
    }

    #[test]
    fn local_player_has_equipped_elytra_false_when_chest_slot_is_missing() {
        let store = WorldStore::new();

        assert!(!store.local_player_has_equipped_elytra());
    }

    #[test]
    fn local_player_has_equipped_elytra_false_when_elytra_is_in_wrong_slot() {
        let mut store = WorldStore::new();

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: PLAYER_CHEST_EQUIPMENT_SLOT - 1,
            item: item_stack(VANILLA_ELYTRA_ITEM_ID, 1),
        });

        assert!(!store.local_player_has_equipped_elytra());
    }

    #[test]
    fn local_player_has_equipped_elytra_false_for_empty_or_count_zero_stack() {
        let mut store = WorldStore::new();

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: PLAYER_CHEST_EQUIPMENT_SLOT,
            item: ProtocolItemStackSummary::empty(),
        });
        assert!(!store.local_player_has_equipped_elytra());

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: PLAYER_CHEST_EQUIPMENT_SLOT,
            item: item_stack(VANILLA_ELYTRA_ITEM_ID, 0),
        });
        assert!(!store.local_player_has_equipped_elytra());
    }

    #[test]
    fn hotbar_item_states_include_local_bundle_selection() {
        let mut store = WorldStore::new();

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 4,
            item: bundle_item_stack(10, 1, 2),
        });
        assert!(store.apply_local_select_bundle_item(4, 1));

        let hotbar = store.inventory().hotbar_item_states();
        assert_eq!(hotbar[4].item, bundle_item_stack(10, 1, 2));
        assert_eq!(hotbar[4].local_selected_bundle_item_index(), Some(1));

        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 0,
            state_id: 1,
            items: (0..45)
                .map(|slot| {
                    if slot == 40 {
                        bundle_item_stack(20, 1, 2)
                    } else {
                        ProtocolItemStackSummary::empty()
                    }
                })
                .collect(),
            carried_item: ProtocolItemStackSummary::empty(),
        });
        assert!(store.open_local_inventory());
        assert!(store.apply_local_select_bundle_item(40, 0));

        let hotbar = store.inventory().hotbar_item_states();
        assert_eq!(hotbar[4].item, bundle_item_stack(20, 1, 2));
        assert_eq!(hotbar[4].local_selected_bundle_item_index(), Some(0));
        assert_eq!(
            store.inventory().hotbar_items()[4],
            bundle_item_stack(20, 1, 2)
        );
    }

    #[test]
    fn local_bundle_selection_tracks_player_inventory_slot() {
        let mut store = WorldStore::new();

        assert!(!store.apply_local_select_bundle_item(4, 0));
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 4,
            item: bundle_item_stack(42, 1, 3),
        });

        assert!(!store.apply_local_select_bundle_item(4, -2));
        assert_eq!(
            player_slot_selection(&store, 4),
            NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
        );

        assert!(store.apply_local_select_bundle_item(4, 1));
        assert_eq!(player_slot_selection(&store, 4), 1);

        assert!(store.apply_local_select_bundle_item(4, -1));
        assert_eq!(
            player_slot_selection(&store, 4),
            NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
        );

        assert!(store.apply_local_select_bundle_item(4, 2));
        assert_eq!(player_slot_selection(&store, 4), 2);

        assert!(store.apply_local_select_bundle_item(4, 2));
        assert_eq!(
            player_slot_selection(&store, 4),
            NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
        );

        assert!(store.apply_local_select_bundle_item(4, 0));
        assert_eq!(player_slot_selection(&store, 4), 0);

        assert!(store.apply_local_select_bundle_item(4, 3));
        assert_eq!(
            player_slot_selection(&store, 4),
            NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
        );

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 5,
            item: item_stack(43, 1),
        });
        assert!(!store.apply_local_select_bundle_item(5, 0));
        assert_eq!(
            player_slot_selection(&store, 5),
            NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
        );
    }

    #[test]
    fn local_bundle_selection_is_cleared_when_player_slot_item_is_replaced() {
        let mut store = WorldStore::new();

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 4,
            item: bundle_item_stack(42, 1, 4),
        });
        assert!(store.apply_local_select_bundle_item(4, 2));

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 4,
            item: bundle_item_stack(43, 1, 3),
        });
        assert_eq!(
            player_slot_selection(&store, 4),
            NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
        );

        assert!(store.apply_local_select_bundle_item(4, 1));
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 4,
            item: bundle_item_stack(44, 1, 2),
        });
        assert_eq!(
            player_slot_selection(&store, 4),
            NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
        );

        assert!(store.apply_local_select_bundle_item(4, 1));
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 4,
            item: item_stack(45, 1),
        });
        assert_eq!(
            player_slot_selection(&store, 4),
            NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
        );
        assert!(!store.apply_local_select_bundle_item(4, 0));
    }

    #[test]
    fn local_bundle_selection_applies_to_open_container_slots() {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 1,
            item: bundle_item_stack(99, 1, 2),
        });
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 1,
            items: vec![item_stack(42, 1), bundle_item_stack(43, 1, 2)],
            carried_item: ProtocolItemStackSummary::empty(),
        });

        assert!(store.apply_local_select_bundle_item(1, 1));
        assert_eq!(container_slot_selection(&store, 1), 1);
        assert_eq!(
            player_slot_selection(&store, 1),
            NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
        );

        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 2,
            items: vec![item_stack(42, 1), bundle_item_stack(43, 1, 2)],
            carried_item: ProtocolItemStackSummary::empty(),
        });
        assert_eq!(
            container_slot_selection(&store, 1),
            NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
        );
        assert!(store.apply_local_select_bundle_item(1, 1));
        assert_eq!(container_slot_selection(&store, 1), 1);

        assert!(!store.apply_local_select_bundle_item(0, 0));
        assert_eq!(
            container_slot_selection(&store, 0),
            NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
        );

        assert!(!store.apply_local_select_bundle_item(99, 0));

        store.apply_container_set_slot(ProtocolContainerSetSlot {
            container_id: 7,
            state_id: 2,
            slot: 1,
            item: bundle_item_stack(44, 1, 1),
        });
        assert_eq!(
            container_slot_selection(&store, 1),
            NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
        );

        assert!(store.apply_local_select_bundle_item(1, 0));
        assert_eq!(container_slot_selection(&store, 1), 0);
    }

    #[test]
    fn local_bundle_selection_fields_default_when_deserializing_old_slots() {
        let player_slot: InventorySlot = serde_json::from_value(serde_json::json!({
            "slot": 4,
            "item": item_stack(42, 1),
        }))
        .unwrap();
        assert_eq!(
            player_slot.local_selected_bundle_item_index,
            NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
        );

        let container_slot: ContainerSlot = serde_json::from_value(serde_json::json!({
            "slot": 4,
            "item": item_stack(42, 1),
        }))
        .unwrap();
        assert_eq!(
            container_slot.local_selected_bundle_item_index,
            NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
        );
    }

    #[test]
    fn local_container_close_does_not_count_clientbound_close_packet() {
        let mut store = WorldStore::new();

        assert!(store.open_local_inventory());
        assert!(store.close_local_container(INVENTORY_MENU_CONTAINER_ID));
        assert!(!store.local_inventory_is_open());
        assert_eq!(store.open_container_id(), None);
        assert_eq!(store.counters().container_close_updates_received, 0);
        assert!(!store.close_local_container(INVENTORY_MENU_CONTAINER_ID));

        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: 2,
            title: "Chest".to_string(),
        });

        assert!(store.close_local_container(7));
        assert!(store.inventory().open_container.is_none());
        assert_eq!(store.counters().container_close_updates_received, 0);
        assert_eq!(store.counters().container_close_updates_applied, 0);
        assert_eq!(store.counters().container_close_updates_ignored, 0);

        assert!(!store.close_local_container(7));
        assert_eq!(store.counters().container_close_updates_received, 0);
        assert_eq!(store.counters().container_close_updates_applied, 0);
        assert_eq!(store.counters().container_close_updates_ignored, 0);
    }

    #[test]
    fn merchant_offers_apply_only_to_matching_open_container() {
        let mut store = WorldStore::new();

        assert!(!store.apply_merchant_offers(merchant_offers(7, 1)));
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: 18,
            title: "Merchant".to_string(),
        });
        assert!(!store.apply_merchant_offers(merchant_offers(7, 1)));
        assert!(store
            .inventory()
            .open_container
            .as_ref()
            .unwrap()
            .merchant_offers
            .is_none());
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
            title: "Merchant".to_string(),
        });
        assert!(!store.apply_merchant_offers(merchant_offers(99, 1)));
        assert!(store.apply_merchant_offers(merchant_offers(7, 2)));

        let container = store.inventory().open_container.as_ref().unwrap();
        let offers = container.merchant_offers.as_ref().unwrap();
        assert_eq!(offers.container_id, 7);
        assert_eq!(offers.offers.len(), 2);
        assert_eq!(offers.villager_level, 3);
        assert_eq!(offers.villager_xp, 120);
        assert!(offers.show_progress);
        assert!(!offers.can_restock);
        assert_eq!(offers.offers[0].buy_a, item_cost(42, 3));
        assert_eq!(offers.offers[0].sell, item_stack(99, 1));

        assert_eq!(store.counters().merchant_offer_packets_received, 4);
        assert_eq!(store.counters().merchant_offer_packets_applied, 1);
        assert_eq!(store.counters().merchant_offer_packets_ignored, 3);
        assert_eq!(store.counters().merchant_offers_tracked, 2);

        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 5,
            items: Vec::new(),
            carried_item: ProtocolItemStackSummary::empty(),
        });
        assert_eq!(store.counters().merchant_offers_tracked, 2);

        assert!(store.apply_container_close(ProtocolContainerClose { container_id: 7 }));
        assert_eq!(store.counters().container_close_updates_applied, 1);
        assert_eq!(store.counters().container_close_updates_ignored, 0);
        assert_eq!(store.counters().merchant_offers_tracked, 0);
    }

    #[test]
    fn build_container_click_slot_uses_open_container_state_and_cursor_item() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: 2,
            title: "Chest".to_string(),
        });
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items: vec![ProtocolItemStackSummary::empty(), item_stack(42, 3)],
            carried_item: item_stack(99, 1),
        });

        let click = store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: 1,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();

        assert_eq!(click.container_id, 7);
        assert_eq!(click.state_id, 13);
        assert_eq!(click.slot_num, 1);
        assert_eq!(click.button_num, 0);
        assert_eq!(click.input, ProtocolContainerInput::Pickup);
        assert!(click.changed_slots.is_empty());
        assert_eq!(
            click.carried_item,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 99,
                count: 1,
                components: ProtocolHashedComponentPatch::default(),
            })
        );
    }

    #[test]
    fn build_container_click_slot_allows_vanilla_outside_slots() {
        let mut store = WorldStore::new();
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 0,
            state_id: 4,
            items: vec![item_stack(42, 1)],
            carried_item: ProtocolItemStackSummary::empty(),
        });
        assert!(store.open_local_inventory());

        let outside_click = store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: -999,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();
        let carried_click = store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: -1,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();

        assert_eq!(outside_click.slot_num, -999);
        assert_eq!(carried_click.slot_num, -1);
    }

    #[test]
    fn build_container_click_slot_rejects_missing_container_invalid_slot_and_unhashable_carried_item(
    ) {
        let mut store = WorldStore::new();
        assert_eq!(
            store
                .build_container_click_slot(ContainerClickSlotRequest {
                    slot_num: 0,
                    button_num: 0,
                    input: ProtocolContainerInput::Pickup,
                })
                .unwrap_err(),
            ContainerClickBuildError::NoOpenContainer
        );

        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items: vec![item_stack(42, 3)],
            carried_item: ProtocolItemStackSummary::empty(),
        });
        assert_eq!(
            store
                .build_container_click_slot(ContainerClickSlotRequest {
                    slot_num: 5,
                    button_num: 0,
                    input: ProtocolContainerInput::Pickup,
                })
                .unwrap_err(),
            ContainerClickBuildError::InvalidSlot(5)
        );

        store.apply_set_cursor_item(ProtocolSetCursorItem {
            item: item_stack_with_component_summary(99, 1, 10),
        });
        assert_eq!(
            store
                .build_container_click_slot(ContainerClickSlotRequest {
                    slot_num: 0,
                    button_num: 0,
                    input: ProtocolContainerInput::Pickup,
                })
                .unwrap_err(),
            ContainerClickBuildError::UnhashableCarriedItem
        );
    }

    fn item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
        ProtocolItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }

    fn item_stack_with_component_summary(
        item_id: i32,
        count: i32,
        component_type_id: i32,
    ) -> ProtocolItemStackSummary {
        let mut stack = item_stack(item_id, count);
        stack.component_patch.added = 1;
        stack.component_patch.added_type_ids = vec![component_type_id];
        stack
    }

    fn bundle_item_stack(
        item_id: i32,
        count: i32,
        bundle_contents_item_count: usize,
    ) -> ProtocolItemStackSummary {
        let mut stack = item_stack(item_id, count);
        stack.component_patch.bundle_contents_item_count = Some(bundle_contents_item_count);
        stack
    }

    fn player_slot_selection(store: &WorldStore, slot: i32) -> i32 {
        store
            .inventory()
            .player_slots
            .iter()
            .find(|state| state.slot == slot)
            .map(|state| state.local_selected_bundle_item_index)
            .unwrap()
    }

    fn container_slot_selection(store: &WorldStore, slot: i16) -> i32 {
        store
            .inventory()
            .open_container
            .as_ref()
            .unwrap()
            .slots
            .iter()
            .find(|state| state.slot == slot)
            .map(|state| state.local_selected_bundle_item_index)
            .unwrap()
    }

    fn merchant_offers(container_id: i32, offer_count: usize) -> ProtocolMerchantOffers {
        ProtocolMerchantOffers {
            container_id,
            offers: (0..offer_count)
                .map(|index| ProtocolMerchantOffer {
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

    fn item_cost(item_id: i32, count: i32) -> ProtocolItemCostSummary {
        ProtocolItemCostSummary {
            item_id,
            count,
            component_predicate: Default::default(),
        }
    }
}
