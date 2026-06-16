use bbb_protocol::packets::{
    ContainerClose as ProtocolContainerClose, ContainerSetContent as ProtocolContainerSetContent,
    ContainerSetData as ProtocolContainerSetData, ContainerSetSlot as ProtocolContainerSetSlot,
    ItemCostSummary as ProtocolItemCostSummary, ItemStackSummary as ProtocolItemStackSummary,
    MerchantOffer as ProtocolMerchantOffer, MerchantOffers as ProtocolMerchantOffers,
    OpenScreen as ProtocolOpenScreen, SetCursorItem as ProtocolSetCursorItem,
    SetPlayerInventory as ProtocolSetPlayerInventory,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

const VANILLA_MENU_TYPE_MERCHANT_ID: i32 = 19;
const PLAYER_HOTBAR_SIZE: usize = 9;
const INVENTORY_MENU_CONTAINER_ID: i32 = 0;
const INVENTORY_MENU_HOTBAR_START: i16 = 36;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventorySlot {
    pub slot: i32,
    pub item: ProtocolItemStackSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerSlot {
    pub slot: i16,
    pub item: ProtocolItemStackSummary,
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

impl Default for InventoryState {
    fn default() -> Self {
        Self {
            player_slots: Vec::new(),
            cursor_item: ProtocolItemStackSummary::empty(),
            open_container: None,
        }
    }
}

impl InventoryState {
    pub fn hotbar_items(&self) -> [ProtocolItemStackSummary; PLAYER_HOTBAR_SIZE] {
        let mut items = std::array::from_fn(|_| ProtocolItemStackSummary::empty());

        for slot in &self.player_slots {
            let Ok(slot_index) = usize::try_from(slot.slot) else {
                continue;
            };
            if slot_index < PLAYER_HOTBAR_SIZE {
                items[slot_index] = slot.item.clone();
            }
        }

        if let Some(container) = self
            .open_container
            .as_ref()
            .filter(|container| container.container_id == INVENTORY_MENU_CONTAINER_ID)
        {
            for slot in &container.slots {
                let Some(index) = slot.slot.checked_sub(INVENTORY_MENU_HOTBAR_START) else {
                    continue;
                };
                let Ok(slot_index) = usize::try_from(index) else {
                    continue;
                };
                if slot_index < PLAYER_HOTBAR_SIZE {
                    items[slot_index] = slot.item.clone();
                }
            }
        }

        items
    }
}

impl WorldStore {
    pub fn apply_set_player_inventory(&mut self, packet: ProtocolSetPlayerInventory) {
        self.counters.inventory_slot_updates_received += 1;
        set_inventory_slot(
            &mut self.inventory.player_slots,
            InventorySlot {
                slot: packet.slot,
                item: packet.item,
            },
        );
        self.update_inventory_slot_count();
    }

    pub fn apply_set_cursor_item(&mut self, packet: ProtocolSetCursorItem) {
        self.counters.cursor_item_updates_received += 1;
        self.inventory.cursor_item = packet.item;
    }

    pub fn apply_open_screen(&mut self, packet: ProtocolOpenScreen) {
        self.counters.container_open_updates_received += 1;
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
        self.inventory.cursor_item = packet.carried_item;
        let existing = self
            .inventory
            .open_container
            .take()
            .filter(|container| container.container_id == packet.container_id);
        let merchant_offers = existing
            .as_ref()
            .and_then(|container| container.merchant_offers.clone());
        self.inventory.open_container = Some(ContainerState {
            container_id: packet.container_id,
            menu_type_id: existing
                .as_ref()
                .and_then(|container| container.menu_type_id),
            title: existing
                .as_ref()
                .and_then(|container| container.title.clone()),
            state_id: packet.state_id,
            slots: packet
                .items
                .into_iter()
                .enumerate()
                .map(|(slot, item)| ContainerSlot {
                    slot: slot as i16,
                    item,
                })
                .collect(),
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
        let container = self.ensure_container(packet.container_id);
        container.state_id = packet.state_id;
        set_container_slot(
            &mut container.slots,
            ContainerSlot {
                slot: packet.slot,
                item: packet.item,
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

    pub fn inventory(&self) -> &InventoryState {
        &self.inventory
    }

    fn ensure_container(&mut self, container_id: i32) -> &mut ContainerState {
        if self
            .inventory
            .open_container
            .as_ref()
            .is_none_or(|container| container.container_id != container_id)
        {
            self.inventory.open_container = Some(ContainerState {
                container_id,
                ..ContainerState::default()
            });
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

fn set_inventory_slot(slots: &mut Vec<InventorySlot>, update: InventorySlot) {
    if let Some(existing) = slots.iter_mut().find(|slot| slot.slot == update.slot) {
        *existing = update;
    } else {
        slots.push(update);
    }
    slots.sort_by_key(|slot| slot.slot);
}

fn set_container_slot(slots: &mut Vec<ContainerSlot>, update: ContainerSlot) {
    if let Some(existing) = slots.iter_mut().find(|slot| slot.slot == update.slot) {
        *existing = update;
    } else {
        slots.push(update);
    }
    slots.sort_by_key(|slot| slot.slot);
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
                },
                ContainerSlot {
                    slot: 1,
                    item: item_stack(44, 3),
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
    fn local_container_close_does_not_count_clientbound_close_packet() {
        let mut store = WorldStore::new();

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

    fn item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
        ProtocolItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
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
