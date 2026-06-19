use bbb_protocol::packets::{
    AttackRangeSummary as ProtocolAttackRangeSummary, ContainerClick as ProtocolContainerClick,
    ContainerClose as ProtocolContainerClose, ContainerInput as ProtocolContainerInput,
    ContainerSetContent as ProtocolContainerSetContent,
    ContainerSetData as ProtocolContainerSetData, ContainerSetSlot as ProtocolContainerSetSlot,
    DataComponentPatchSummary as ProtocolDataComponentPatchSummary,
    EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind,
    HashedComponentPatch as ProtocolHashedComponentPatch,
    HashedItemStack as ProtocolHashedItemStack, HashedStack as ProtocolHashedStack,
    InteractionHand, ItemCostSummary as ProtocolItemCostSummary,
    ItemStackSummary as ProtocolItemStackSummary, MerchantOffer as ProtocolMerchantOffer,
    MerchantOffers as ProtocolMerchantOffers, OpenScreen as ProtocolOpenScreen,
    SetCursorItem as ProtocolSetCursorItem, SetPlayerInventory as ProtocolSetPlayerInventory,
    StonecutterSelectableRecipeSummary as ProtocolStonecutterSelectableRecipeSummary,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{MountScreenState, RegistryTagState, WorldStore};

const VANILLA_MENU_TYPE_MERCHANT_ID: i32 = 19;
const VANILLA_MENU_TYPE_SHULKER_BOX_ID: i32 = 20;
const VANILLA_MENU_TYPE_GENERIC_9X1_ID: i32 = 0;
const VANILLA_MENU_TYPE_GENERIC_9X6_ID: i32 = 5;
const VANILLA_MENU_TYPE_GENERIC_3X3_ID: i32 = 6;
const VANILLA_MENU_TYPE_CRAFTER_ID: i32 = 7;
const VANILLA_MENU_TYPE_ANVIL_ID: i32 = 8;
const VANILLA_MENU_TYPE_BEACON_ID: i32 = 9;
const VANILLA_MENU_TYPE_BLAST_FURNACE_ID: i32 = 10;
const VANILLA_MENU_TYPE_BREWING_STAND_ID: i32 = 11;
const VANILLA_MENU_TYPE_CRAFTING_ID: i32 = 12;
const VANILLA_MENU_TYPE_ENCHANTMENT_ID: i32 = 13;
const VANILLA_MENU_TYPE_FURNACE_ID: i32 = 14;
const VANILLA_MENU_TYPE_GRINDSTONE_ID: i32 = 15;
const VANILLA_MENU_TYPE_HOPPER_ID: i32 = 16;
const VANILLA_MENU_TYPE_LOOM_ID: i32 = 18;
const VANILLA_MENU_TYPE_SMITHING_ID: i32 = 21;
const VANILLA_MENU_TYPE_SMOKER_ID: i32 = 22;
const VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID: i32 = 23;
const VANILLA_MENU_TYPE_STONECUTTER_ID: i32 = 24;
const GENERIC_CONTAINER_SLOT_COUNT_PER_ROW: i16 = 9;
const GENERIC_3X3_CONTAINER_SLOT_COUNT: i16 = 9;
const CRAFTER_GRID_SLOT_COUNT: i16 = 9;
const CRAFTER_PLAYER_MAIN_START: i16 = 9;
const CRAFTER_HOTBAR_END: i16 = 45;
const CRAFTER_RESULT_SLOT: i16 = 45;
const CRAFTER_TOTAL_SLOT_COUNT: i16 = 46;
const CRAFTING_MENU_RESULT_SLOT: i16 = 0;
const CRAFTING_MENU_CRAFT_SLOT_START: i16 = 1;
const CRAFTING_MENU_CRAFT_SLOT_END: i16 = 10;
const CRAFTING_MENU_PLAYER_MAIN_START: i16 = 10;
const CRAFTING_MENU_PLAYER_MAIN_END: i16 = 37;
const CRAFTING_MENU_HOTBAR_START: i16 = 37;
const CRAFTING_MENU_HOTBAR_END: i16 = 46;
const CRAFTING_MENU_TOTAL_SLOT_COUNT: i16 = 46;
const STONECUTTER_INPUT_SLOT: i16 = 0;
const STONECUTTER_RESULT_SLOT: i16 = 1;
const STONECUTTER_PLAYER_MAIN_START: i16 = 2;
const STONECUTTER_PLAYER_MAIN_END: i16 = 29;
const STONECUTTER_HOTBAR_START: i16 = 29;
const STONECUTTER_HOTBAR_END: i16 = 38;
const STONECUTTER_TOTAL_SLOT_COUNT: i16 = 38;
const GRINDSTONE_INPUT_SLOT: i16 = 0;
const GRINDSTONE_ADDITIONAL_SLOT: i16 = 1;
const GRINDSTONE_RESULT_SLOT: i16 = 2;
const GRINDSTONE_PLAYER_MAIN_START: i16 = 3;
const GRINDSTONE_PLAYER_MAIN_END: i16 = 30;
const GRINDSTONE_HOTBAR_START: i16 = 30;
const GRINDSTONE_HOTBAR_END: i16 = 39;
const GRINDSTONE_TOTAL_SLOT_COUNT: i16 = 39;
const BREWING_STAND_BOTTLE_SLOT_START: i16 = 0;
const BREWING_STAND_BOTTLE_SLOT_END: i16 = 3;
const BREWING_STAND_INGREDIENT_SLOT: i16 = 3;
const BREWING_STAND_FUEL_SLOT: i16 = 4;
const BREWING_STAND_PLAYER_MAIN_START: i16 = 5;
const BREWING_STAND_PLAYER_MAIN_END: i16 = 32;
const BREWING_STAND_HOTBAR_START: i16 = 32;
const BREWING_STAND_HOTBAR_END: i16 = 41;
const BREWING_STAND_TOTAL_SLOT_COUNT: i16 = 41;
const BREWING_STAND_FUEL_ITEM_TAG: &str = "minecraft:brewing_fuel";
const BEACON_PAYMENT_SLOT: i16 = 0;
const BEACON_PLAYER_MAIN_START: i16 = 1;
const BEACON_PLAYER_MAIN_END: i16 = 28;
const BEACON_HOTBAR_START: i16 = 28;
const BEACON_HOTBAR_END: i16 = 37;
const BEACON_TOTAL_SLOT_COUNT: i16 = 37;
const BEACON_PAYMENT_ITEM_TAG: &str = "minecraft:beacon_payment_items";
const ENCHANTMENT_INPUT_SLOT: i16 = 0;
const ENCHANTMENT_LAPIS_SLOT: i16 = 1;
const ENCHANTMENT_PLAYER_MAIN_START: i16 = 2;
const ENCHANTMENT_HOTBAR_END: i16 = 38;
const ENCHANTMENT_TOTAL_SLOT_COUNT: i16 = 38;
const ANVIL_INPUT_SLOT: i16 = 0;
const ANVIL_ADDITIONAL_SLOT: i16 = 1;
const ANVIL_RESULT_SLOT: i16 = 2;
const ANVIL_PLAYER_MAIN_START: i16 = 3;
const ANVIL_HOTBAR_END: i16 = 39;
const ANVIL_TOTAL_SLOT_COUNT: i16 = 39;
const CARTOGRAPHY_TABLE_MAP_SLOT: i16 = 0;
const CARTOGRAPHY_TABLE_ADDITIONAL_SLOT: i16 = 1;
const CARTOGRAPHY_TABLE_RESULT_SLOT: i16 = 2;
const CARTOGRAPHY_TABLE_PLAYER_MAIN_START: i16 = 3;
const CARTOGRAPHY_TABLE_PLAYER_MAIN_END: i16 = 30;
const CARTOGRAPHY_TABLE_HOTBAR_START: i16 = 30;
const CARTOGRAPHY_TABLE_HOTBAR_END: i16 = 39;
const CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT: i16 = 39;
const LOOM_BANNER_SLOT: i16 = 0;
const LOOM_DYE_SLOT: i16 = 1;
const LOOM_PATTERN_SLOT: i16 = 2;
const LOOM_RESULT_SLOT: i16 = 3;
const LOOM_PLAYER_MAIN_START: i16 = 4;
const LOOM_PLAYER_MAIN_END: i16 = 31;
const LOOM_HOTBAR_START: i16 = 31;
const LOOM_HOTBAR_END: i16 = 40;
const LOOM_TOTAL_SLOT_COUNT: i16 = 40;
const LOOM_BANNER_ITEM_TAG: &str = "minecraft:banners";
const LOOM_DYE_ITEM_TAG: &str = "minecraft:loom_dyes";
const LOOM_PATTERN_ITEM_TAG: &str = "minecraft:loom_patterns";
const MERCHANT_PAYMENT_SLOT_1: i16 = 0;
const MERCHANT_PAYMENT_SLOT_2: i16 = 1;
const MERCHANT_RESULT_SLOT: i16 = 2;
const MERCHANT_PLAYER_MAIN_START: i16 = 3;
const MERCHANT_PLAYER_MAIN_END: i16 = 30;
const MERCHANT_HOTBAR_START: i16 = 30;
const MERCHANT_HOTBAR_END: i16 = 39;
const MERCHANT_TOTAL_SLOT_COUNT: i16 = 39;
const MERCHANT_VISIBLE_OFFER_COUNT: usize = 7;
const SMITHING_TEMPLATE_SLOT: i16 = 0;
const SMITHING_BASE_SLOT: i16 = 1;
const SMITHING_ADDITIONAL_SLOT: i16 = 2;
const SMITHING_RESULT_SLOT: i16 = 3;
const SMITHING_PLAYER_MAIN_START: i16 = 4;
const SMITHING_HOTBAR_END: i16 = 40;
const SMITHING_TOTAL_SLOT_COUNT: i16 = 40;
const FURNACE_CONTAINER_SLOT_COUNT: i16 = 3;
const HOPPER_CONTAINER_SLOT_COUNT: i16 = 5;
const SHULKER_BOX_CONTAINER_SLOT_COUNT: i16 = 27;
const GENERIC_CONTAINER_PLAYER_SLOT_COUNT: i16 = 36;
const PLAYER_HOTBAR_SIZE: usize = 9;
const PLAYER_FEET_EQUIPMENT_SLOT: i32 = 36;
const PLAYER_LEGS_EQUIPMENT_SLOT: i32 = 37;
const PLAYER_CHEST_EQUIPMENT_SLOT: i32 = 38;
const PLAYER_HEAD_EQUIPMENT_SLOT: i32 = 39;
const PLAYER_OFFHAND_SLOT: i32 = 40;
const PLAYER_BODY_EQUIPMENT_SLOT: i32 = 41;
const INVENTORY_MENU_CONTAINER_ID: i32 = 0;
const INVENTORY_MENU_MAIN_START: i16 = 9;
const INVENTORY_MENU_MAIN_END: i16 = 36;
const INVENTORY_MENU_HOTBAR_START: i16 = 36;
const INVENTORY_MENU_HOTBAR_END: i16 = 45;
const INVENTORY_MENU_OFFHAND_SLOT: i16 = 45;
const VANILLA_MAX_STACK_SIZE_COMPONENT_ID: i32 = 1;
const VANILLA_USE_EFFECTS_COMPONENT_ID: i32 = 5;
const VANILLA_ATTACK_RANGE_COMPONENT_ID: i32 = 30;
const VANILLA_PIERCING_WEAPON_COMPONENT_ID: i32 = 38;
const VANILLA_MAP_ID_COMPONENT_ID: i32 = 41;
const VANILLA_DEFAULT_MAX_STACK_SIZE: i32 = 64;
const VANILLA_ABSOLUTE_MAX_STACK_SIZE: i32 = 99;
const NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX: i32 = -1;
const VANILLA_ELYTRA_ITEM_ID: i32 = 14;
const QUICKCRAFT_TYPE_CHARITABLE: i8 = 0;
const QUICKCRAFT_TYPE_GREEDY: i8 = 1;
const QUICKCRAFT_TYPE_CLONE: i8 = 2;
const QUICKCRAFT_HEADER_START: i8 = 0;
const QUICKCRAFT_HEADER_CONTINUE: i8 = 1;
const QUICKCRAFT_HEADER_END: i8 = 2;
const VANILLA_AGEABLE_MOB_BABY_DATA_ID: u8 = 16;
const VANILLA_MOUNT_TAME_FLAGS_DATA_ID: u8 = 18;
const VANILLA_ABSTRACT_HORSE_TAME_FLAG: i8 = 2;
const VANILLA_TAMABLE_ANIMAL_TAME_FLAG: i8 = 4;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemEquipmentSlot {
    MainHand,
    OffHand,
    Feet,
    Legs,
    Chest,
    Head,
    Body,
    Saddle,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ItemAttackRange {
    pub min_reach: f32,
    pub max_reach: f32,
    pub min_creative_reach: f32,
    pub max_creative_reach: f32,
    pub hitbox_margin: f32,
    pub mob_factor: f32,
}

impl PartialEq for ItemAttackRange {
    fn eq(&self, other: &Self) -> bool {
        self.min_reach.to_bits() == other.min_reach.to_bits()
            && self.max_reach.to_bits() == other.max_reach.to_bits()
            && self.min_creative_reach.to_bits() == other.min_creative_reach.to_bits()
            && self.max_creative_reach.to_bits() == other.max_creative_reach.to_bits()
            && self.hitbox_margin.to_bits() == other.hitbox_margin.to_bits()
            && self.mob_factor.to_bits() == other.mob_factor.to_bits()
    }
}

impl Eq for ItemAttackRange {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ItemUseEffects {
    pub can_sprint: bool,
    pub interact_vibrations: bool,
    pub speed_multiplier: f32,
}

impl Default for ItemUseEffects {
    fn default() -> Self {
        Self {
            can_sprint: false,
            interact_vibrations: true,
            speed_multiplier: 0.2,
        }
    }
}

impl PartialEq for ItemUseEffects {
    fn eq(&self, other: &Self) -> bool {
        self.can_sprint == other.can_sprint
            && self.interact_vibrations == other.interact_vibrations
            && self.speed_multiplier.to_bits() == other.speed_multiplier.to_bits()
    }
}

impl Eq for ItemUseEffects {}

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
    #[serde(default)]
    pub mount: Option<MountScreenState>,
    pub state_id: i32,
    pub slots: Vec<ContainerSlot>,
    pub data_values: Vec<ContainerDataValue>,
    pub merchant_offers: Option<MerchantOffersState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MountInventoryKind {
    Horse,
    Nautilus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MountArmorSlotKind {
    Horse,
    Llama,
    Nautilus,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountEquipmentSlotVisibility {
    pub saddle: bool,
    pub body: Option<MountArmorSlotKind>,
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
    #[serde(default, skip_serializing)]
    local_quick_craft: LocalQuickCraftState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MerchantOffersState {
    pub container_id: i32,
    pub offers: Vec<MerchantOfferState>,
    pub villager_level: i32,
    pub villager_xp: i32,
    pub show_progress: bool,
    pub can_restock: bool,
    #[serde(default)]
    pub local_selected_offer_index: i32,
    #[serde(default)]
    pub local_scroll_offset: i32,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LocalQuickCraftState {
    status: i8,
    quickcraft_type: i8,
    slots: Vec<i16>,
}

impl Default for LocalQuickCraftState {
    fn default() -> Self {
        Self {
            status: QUICKCRAFT_HEADER_START,
            quickcraft_type: -1,
            slots: Vec::new(),
        }
    }
}

impl LocalQuickCraftState {
    fn reset(&mut self) {
        self.status = QUICKCRAFT_HEADER_START;
        self.quickcraft_type = -1;
        self.slots.clear();
    }

    fn is_active(&self) -> bool {
        self.status != QUICKCRAFT_HEADER_START || !self.slots.is_empty()
    }
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
    UnsupportedLocalClickInput(ProtocolContainerInput),
    UnhashableCarriedItem,
    UnhashableChangedSlot(i16),
}

impl Default for InventoryState {
    fn default() -> Self {
        Self {
            player_slots: Vec::new(),
            cursor_item: ProtocolItemStackSummary::empty(),
            inventory_menu: default_inventory_menu(),
            local_inventory_open: false,
            open_container: None,
            local_quick_craft: LocalQuickCraftState::default(),
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
        self.inventory.local_quick_craft.reset();
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
            mount: None,
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
        self.inventory.local_quick_craft.reset();
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
                mount: existing.mount,
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
            mount: existing.as_ref().and_then(|container| container.mount),
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

    pub(crate) fn apply_mount_screen_open_container(&mut self, mount: MountScreenState) {
        self.inventory.local_inventory_open = false;
        self.inventory.local_quick_craft.reset();
        let existing = self
            .inventory
            .open_container
            .take()
            .filter(|container| container.container_id == mount.container_id)
            .unwrap_or_else(|| ContainerState {
                container_id: mount.container_id,
                ..ContainerState::default()
            });
        self.inventory.open_container = Some(ContainerState {
            container_id: mount.container_id,
            menu_type_id: None,
            title: existing.title,
            mount: Some(mount),
            state_id: existing.state_id,
            slots: existing.slots,
            data_values: existing.data_values,
            merchant_offers: None,
        });
        self.update_merchant_offer_count();
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
                self.inventory.local_quick_craft.reset();
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
            self.inventory.local_quick_craft.reset();
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
                self.inventory.local_quick_craft.reset();
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
            self.inventory.local_quick_craft.reset();
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
        self.inventory.local_quick_craft.reset();
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

    pub fn open_container_data_value(&self, id: i16) -> Option<i16> {
        self.inventory
            .open_container
            .as_ref()?
            .data_values
            .iter()
            .find_map(|value| (value.id == id).then_some(value.value))
    }

    pub fn open_mount_inventory_kind(&self) -> Option<MountInventoryKind> {
        let mount = self.inventory.open_container.as_ref()?.mount?;
        let entity_type_id = self.entities.entity_type_id(mount.entity_id)?;
        if crate::entities::is_vanilla_abstract_horse_type(entity_type_id) {
            Some(MountInventoryKind::Horse)
        } else if crate::entities::is_vanilla_abstract_nautilus_type(entity_type_id) {
            Some(MountInventoryKind::Nautilus)
        } else {
            None
        }
    }

    pub fn open_mount_armor_slot_kind(&self) -> Option<MountArmorSlotKind> {
        self.open_mount_equipment_slot_visibility()?.body
    }

    pub fn open_mount_equipment_slot_visibility(&self) -> Option<MountEquipmentSlotVisibility> {
        let mount = self.inventory.open_container.as_ref()?.mount?;
        let entity = self.probe_entity(mount.entity_id)?;
        let entity_type_id = entity.entity_type_id;
        if crate::entities::is_vanilla_abstract_nautilus_type(entity_type_id) {
            let active = mount_nautilus_can_use_equipment_slots(&entity.data_values);
            Some(MountEquipmentSlotVisibility {
                saddle: active,
                body: active.then_some(MountArmorSlotKind::Nautilus),
            })
        } else if crate::entities::is_vanilla_abstract_horse_type(entity_type_id) {
            Some(MountEquipmentSlotVisibility {
                saddle: mount_horse_saddle_slot_is_active(entity_type_id, &entity.data_values),
                body: mount_horse_body_slot_kind(entity_type_id),
            })
        } else {
            None
        }
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

    pub(crate) fn local_item_in_hand_is_non_empty(&self, hand: InteractionHand) -> bool {
        match hand {
            InteractionHand::MainHand => {
                let selected_slot = self.local_player.selected_hotbar_slot;
                if selected_slot > 8 {
                    return false;
                }
                self.local_player_inventory_item(i32::from(selected_slot))
                    .is_some_and(item_stack_is_non_empty)
            }
            InteractionHand::OffHand => self
                .local_offhand_item()
                .is_some_and(item_stack_is_non_empty),
        }
    }

    pub fn local_selected_main_hand_has_piercing_weapon(&self) -> bool {
        let selected_slot = self.local_player.selected_hotbar_slot;
        if selected_slot > 8 {
            return false;
        }

        self.local_player_inventory_item(i32::from(selected_slot))
            .is_some_and(|item| {
                item_stack_has_piercing_weapon(item, &self.default_piercing_weapon_item_ids)
            })
    }

    pub fn local_selected_main_hand_attack_range(&self) -> Option<ItemAttackRange> {
        let selected_slot = self.local_player.selected_hotbar_slot;
        if selected_slot > 8 {
            return None;
        }

        self.local_player_inventory_item(i32::from(selected_slot))
            .and_then(|item| item_stack_attack_range(item, &self.default_item_attack_ranges))
    }

    pub(crate) fn local_using_item_use_effects(&self) -> Option<ItemUseEffects> {
        if !self.local_player.interaction.using_item {
            return None;
        }

        let item = match self.local_player.interaction.using_item_hand {
            Some(InteractionHand::OffHand) => self.local_offhand_item(),
            Some(InteractionHand::MainHand) | None => {
                let selected_slot = self.local_player.selected_hotbar_slot;
                if selected_slot > 8 {
                    return None;
                }
                self.local_player_inventory_item(i32::from(selected_slot))
            }
        }?;

        item_stack_use_effects(item, &self.default_item_use_effects)
    }

    pub fn drop_local_selected_hotbar_item(&mut self, all: bool) -> bool {
        let selected_slot = self.local_player.selected_hotbar_slot;
        if selected_slot > 8 {
            return false;
        }

        let selected_slot = i32::from(selected_slot);
        let mut selected = self.player_inventory_slot(selected_slot);
        if item_stack_is_empty(&selected.item) {
            return false;
        }

        if all {
            selected.item = ProtocolItemStackSummary::empty();
        } else {
            selected.item.count -= 1;
            normalize_item_stack(&mut selected.item);
        }

        self.set_player_inventory_slot_and_menu_slot(selected);
        self.update_inventory_slot_count();
        true
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

    pub fn set_default_item_max_stack_sizes(&mut self, max_stack_sizes: BTreeMap<i32, i32>) {
        self.default_item_max_stack_sizes = max_stack_sizes
            .into_iter()
            .filter(|(item_id, size)| *item_id >= 0 && *size > 0)
            .map(|(item_id, size)| (item_id, clamp_vanilla_item_max_stack_size(size)))
            .collect();
    }

    pub fn item_max_stack_size_for_protocol_id(&self, item_id: i32) -> i32 {
        self.default_item_max_stack_sizes
            .get(&item_id)
            .copied()
            .map(clamp_vanilla_item_max_stack_size)
            .unwrap_or(VANILLA_DEFAULT_MAX_STACK_SIZE)
    }

    pub fn set_local_merchant_selected_offer(&mut self, index: i32) -> bool {
        let Some(offers) = self
            .inventory
            .open_container
            .as_mut()
            .and_then(|container| container.merchant_offers.as_mut())
        else {
            return false;
        };
        let Ok(index_usize) = usize::try_from(index) else {
            return false;
        };
        if index_usize >= offers.offers.len() {
            return false;
        }
        offers.local_selected_offer_index = index;
        true
    }

    pub fn scroll_local_merchant_offers(&mut self, delta: i32) -> bool {
        let Some(offers) = self
            .inventory
            .open_container
            .as_mut()
            .and_then(|container| container.merchant_offers.as_mut())
        else {
            return false;
        };
        let max_scroll_offset = merchant_max_scroll_offset(offers.offers.len());
        if max_scroll_offset <= 0 {
            return false;
        }
        offers.local_scroll_offset =
            (offers.local_scroll_offset + delta).clamp(0, max_scroll_offset);
        true
    }

    pub fn set_furnace_fuel_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.furnace_fuel_item_ids = item_ids
            .into_iter()
            .filter(|item_id| *item_id >= 0)
            .collect();
    }

    pub fn set_brewing_potion_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.brewing_potion_item_ids = item_ids
            .into_iter()
            .filter(|item_id| *item_id >= 0)
            .collect();
    }

    pub fn set_brewing_ingredient_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.brewing_ingredient_item_ids = item_ids
            .into_iter()
            .filter(|item_id| *item_id >= 0)
            .collect();
    }

    pub fn set_enchantment_lapis_lazuli_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.enchantment_lapis_lazuli_item_ids = item_ids
            .into_iter()
            .filter(|item_id| *item_id >= 0)
            .collect();
    }

    pub fn set_cartography_additional_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.cartography_additional_item_ids = item_ids
            .into_iter()
            .filter(|item_id| *item_id >= 0)
            .collect();
    }

    pub fn set_freeze_immune_wearable_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.freeze_immune_wearable_item_ids = item_ids
            .into_iter()
            .filter(|item_id| *item_id >= 0)
            .collect();
    }

    pub fn set_powder_snow_walkable_foot_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.powder_snow_walkable_foot_item_ids = item_ids
            .into_iter()
            .filter(|item_id| *item_id >= 0)
            .collect();
    }

    pub fn set_default_piercing_weapon_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.default_piercing_weapon_item_ids = item_ids
            .into_iter()
            .filter(|item_id| *item_id >= 0)
            .collect();
    }

    pub fn set_default_item_attack_ranges(
        &mut self,
        attack_ranges: BTreeMap<i32, ItemAttackRange>,
    ) {
        self.default_item_attack_ranges = attack_ranges
            .into_iter()
            .filter(|(item_id, _)| *item_id >= 0)
            .collect();
    }

    pub fn set_default_item_use_effects(&mut self, use_effects: BTreeMap<i32, ItemUseEffects>) {
        self.default_item_use_effects = use_effects
            .into_iter()
            .filter(|(item_id, _)| *item_id >= 0)
            .collect();
    }

    pub fn set_default_item_equipment_slots(
        &mut self,
        equipment_slots: BTreeMap<i32, ItemEquipmentSlot>,
    ) {
        self.default_item_equipment_slots = equipment_slots
            .into_iter()
            .filter(|(item_id, _)| *item_id >= 0)
            .collect();
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

    pub fn apply_local_container_click_slot(
        &mut self,
        request: ContainerClickSlotRequest,
    ) -> Result<ProtocolContainerClick, ContainerClickBuildError> {
        let (container_id, state_id, menu_type_id, slots_before, data_values) = {
            let Some(container) = self.active_container() else {
                return Err(ContainerClickBuildError::NoOpenContainer);
            };
            if !container_click_slot_is_valid(container, request.slot_num) {
                return Err(ContainerClickBuildError::InvalidSlot(request.slot_num));
            }
            (
                container.container_id,
                container.state_id,
                container.menu_type_id,
                container.slots.clone(),
                container.data_values.clone(),
            )
        };
        let mut slots_after = slots_before.clone();
        let mut cursor_after = self.inventory.cursor_item.clone();
        let mut quick_craft_after = self.inventory.local_quick_craft.clone();
        if menu_result_slot_requires_server_authority(menu_type_id, request.slot_num)
            && matches!(
                request.input,
                ProtocolContainerInput::Pickup | ProtocolContainerInput::QuickMove
            )
        {
            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                request.input,
            ));
        }
        if request.input != ProtocolContainerInput::QuickCraft && quick_craft_after.is_active() {
            quick_craft_after.reset();
        } else {
            match request.input {
                ProtocolContainerInput::Pickup => apply_pickup_click_to_slots(
                    container_id,
                    &mut slots_after,
                    &mut cursor_after,
                    request.slot_num,
                    request.button_num,
                    &self.default_item_max_stack_sizes,
                ),
                ProtocolContainerInput::Clone => apply_clone_click_to_slots(
                    &slots_after,
                    &mut cursor_after,
                    request.slot_num,
                    self.local_player
                        .abilities
                        .is_some_and(|abilities| abilities.instabuild),
                    &self.default_item_max_stack_sizes,
                ),
                ProtocolContainerInput::QuickMove => {
                    if container_id == INVENTORY_MENU_CONTAINER_ID {
                        if request.slot_num == 0 {
                            apply_inventory_menu_result_quick_move_to_slots(
                                &mut slots_after,
                                &self.default_item_max_stack_sizes,
                            );
                        } else {
                            apply_quick_move_to_slots(
                                container_id,
                                &mut slots_after,
                                request.slot_num,
                                &self.default_item_equipment_slots,
                                &self.default_item_max_stack_sizes,
                            )
                        }
                    } else if let Some(container_slot_count) =
                        generic_9x_container_slot_count(menu_type_id)
                    {
                        apply_generic_container_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            container_slot_count,
                            &self.default_item_max_stack_sizes,
                        )
                    } else if let Some(container_slot_count) =
                        generic_3x3_container_slot_count(menu_type_id)
                    {
                        apply_generic_container_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            container_slot_count,
                            &self.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_CRAFTING_ID) {
                        apply_crafting_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &self.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_CRAFTER_ID) {
                        let disabled_slots = crafter_disabled_slots(&data_values);
                        apply_crafter_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &disabled_slots,
                            &self.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_ANVIL_ID) {
                        if anvil_quick_move_requires_server_authority(
                            &slots_after,
                            request.slot_num,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                        apply_anvil_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &self.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_BEACON_ID) {
                        apply_beacon_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            self.registry_tags("minecraft:item"),
                            &self.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_ENCHANTMENT_ID) {
                        if enchantment_quick_move_requires_server_authority(
                            &slots_after,
                            request.slot_num,
                            &self.enchantment_lapis_lazuli_item_ids,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                        apply_enchantment_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &self.enchantment_lapis_lazuli_item_ids,
                            &self.default_item_max_stack_sizes,
                        )
                    } else if furnace_family_menu_type(menu_type_id).is_some() {
                        apply_furnace_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            menu_type_id,
                            &self.recipes.property_sets,
                            &self.furnace_fuel_item_ids,
                            &self.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_BREWING_STAND_ID) {
                        apply_brewing_stand_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            self.registry_tags("minecraft:item"),
                            &self.brewing_potion_item_ids,
                            &self.brewing_ingredient_item_ids,
                            &self.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_GRINDSTONE_ID) {
                        if grindstone_quick_move_requires_server_authority(
                            &slots_after,
                            request.slot_num,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                        apply_grindstone_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &self.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_SMITHING_ID) {
                        if smithing_quick_move_requires_server_authority(
                            &slots_after,
                            request.slot_num,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                        apply_smithing_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &self.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID) {
                        if cartography_table_quick_move_requires_server_authority(
                            &slots_after,
                            request.slot_num,
                            &self.cartography_additional_item_ids,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                        apply_cartography_table_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &self.cartography_additional_item_ids,
                            &self.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_LOOM_ID) {
                        apply_loom_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            self.registry_tags("minecraft:item"),
                            &self.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_MERCHANT_ID) {
                        if merchant_quick_move_requires_server_authority(request.slot_num) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                        apply_merchant_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &self.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_STONECUTTER_ID) {
                        apply_stonecutter_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &self.recipes.stonecutter_recipes,
                            &self.default_item_max_stack_sizes,
                        )
                    } else if let Some(container_slot_count) =
                        hopper_container_slot_count(menu_type_id)
                    {
                        apply_generic_container_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            container_slot_count,
                            &self.default_item_max_stack_sizes,
                        )
                    } else if let Some(container_slot_count) =
                        shulker_box_container_slot_count(menu_type_id)
                    {
                        apply_generic_container_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            container_slot_count,
                            &self.default_item_max_stack_sizes,
                        )
                    } else {
                        return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                            ProtocolContainerInput::QuickMove,
                        ));
                    }
                }
                ProtocolContainerInput::Throw if container_id == INVENTORY_MENU_CONTAINER_ID => {
                    apply_throw_click_to_slots(
                        &mut slots_after,
                        &cursor_after,
                        request.slot_num,
                        request.button_num,
                    )
                }
                ProtocolContainerInput::Swap if container_id == INVENTORY_MENU_CONTAINER_ID => {
                    apply_swap_click_to_slots(
                        container_id,
                        &mut slots_after,
                        &cursor_after,
                        request.slot_num,
                        request.button_num,
                        &self.default_item_max_stack_sizes,
                    )
                }
                ProtocolContainerInput::QuickCraft
                    if container_id == INVENTORY_MENU_CONTAINER_ID =>
                {
                    apply_quick_craft_to_slots(
                        container_id,
                        &mut slots_after,
                        &mut cursor_after,
                        &mut quick_craft_after,
                        request.slot_num,
                        request.button_num,
                        &self.default_item_max_stack_sizes,
                    )
                }
                ProtocolContainerInput::PickupAll
                    if container_id == INVENTORY_MENU_CONTAINER_ID =>
                {
                    apply_pickup_all_to_slots(
                        &mut slots_after,
                        &mut cursor_after,
                        request.slot_num,
                        request.button_num,
                        &self.default_item_max_stack_sizes,
                    )
                }
                input => {
                    return Err(ContainerClickBuildError::UnsupportedLocalClickInput(input));
                }
            }
        }
        if container_id == INVENTORY_MENU_CONTAINER_ID
            && request.slot_num == 0
            && request.input != ProtocolContainerInput::QuickMove
            && inventory_menu_result_was_taken(&slots_before, &slots_after)
        {
            apply_inventory_menu_result_take_side_effects(&mut slots_after);
        }
        let changed_slots = changed_hashed_slots(&slots_before, &slots_after)?;
        let carried_item = hashed_stack_from_summary(&cursor_after)
            .ok_or(ContainerClickBuildError::UnhashableCarriedItem)?;

        {
            let container = self
                .active_container_mut()
                .expect("active container still exists");
            container.slots = slots_after;
        }
        self.inventory.cursor_item = cursor_after;
        self.inventory.local_quick_craft = quick_craft_after;
        if container_id == INVENTORY_MENU_CONTAINER_ID {
            self.sync_player_inventory_slots_from_inventory_menu();
        }

        Ok(ProtocolContainerClick {
            container_id,
            state_id,
            slot_num: request.slot_num,
            button_num: request.button_num,
            input: request.input,
            changed_slots,
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

    pub(crate) fn local_player_has_freeze_immune_wearable(&self) -> bool {
        [
            PLAYER_FEET_EQUIPMENT_SLOT,
            PLAYER_LEGS_EQUIPMENT_SLOT,
            PLAYER_CHEST_EQUIPMENT_SLOT,
            PLAYER_HEAD_EQUIPMENT_SLOT,
            PLAYER_BODY_EQUIPMENT_SLOT,
        ]
        .into_iter()
        .filter_map(|slot| self.local_player_inventory_item(slot))
        .filter(|item| item.count > 0)
        .filter_map(|item| item.item_id)
        .any(|item_id| self.freeze_immune_wearable_item_ids.contains(&item_id))
    }

    pub(crate) fn local_player_can_walk_on_powder_snow(&self) -> bool {
        self.local_player_inventory_item(PLAYER_FEET_EQUIPMENT_SLOT)
            .filter(|item| item.count > 0)
            .and_then(|item| item.item_id)
            .is_some_and(|item_id| self.powder_snow_walkable_foot_item_ids.contains(&item_id))
    }

    fn local_player_inventory_item(&self, slot_id: i32) -> Option<&ProtocolItemStackSummary> {
        if let Some(item) = self
            .inventory
            .player_slots
            .iter()
            .find_map(|slot| (slot.slot == slot_id).then_some(&slot.item))
        {
            return Some(item);
        }

        let menu_slot = inventory_slot_to_inventory_menu_slot(slot_id)?;
        self.inventory
            .inventory_menu
            .slots
            .iter()
            .find_map(|slot| (slot.slot == menu_slot).then_some(&slot.item))
    }

    fn player_inventory_slot(&self, slot: i32) -> InventorySlot {
        self.inventory
            .player_slots
            .iter()
            .find(|existing| existing.slot == slot)
            .cloned()
            .unwrap_or(InventorySlot {
                slot,
                item: ProtocolItemStackSummary::empty(),
                local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
            })
    }

    fn set_player_inventory_slot_and_menu_slot(&mut self, slot: InventorySlot) {
        let menu_slot = inventory_slot_to_inventory_menu_slot(slot.slot);
        set_inventory_slot(&mut self.inventory.player_slots, slot.clone());
        if let Some(menu_slot) = menu_slot {
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

    fn active_container(&self) -> Option<&ContainerState> {
        self.inventory.open_container.as_ref().or_else(|| {
            self.inventory
                .local_inventory_open
                .then_some(&self.inventory.inventory_menu)
        })
    }

    fn active_container_mut(&mut self) -> Option<&mut ContainerState> {
        if self.inventory.open_container.is_some() {
            return self.inventory.open_container.as_mut();
        }
        self.inventory
            .local_inventory_open
            .then_some(&mut self.inventory.inventory_menu)
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

    fn sync_player_inventory_slots_from_inventory_menu(&mut self) {
        for slot in self.inventory.inventory_menu.slots.clone() {
            let Some(player_slot) = inventory_menu_slot_to_inventory_slot(slot.slot) else {
                continue;
            };
            set_inventory_slot(
                &mut self.inventory.player_slots,
                InventorySlot {
                    slot: player_slot,
                    item: slot.item,
                    local_selected_bundle_item_index: slot.local_selected_bundle_item_index,
                },
            );
        }
        self.update_inventory_slot_count();
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

fn mount_horse_saddle_slot_is_active(
    entity_type_id: i32,
    data_values: &[ProtocolEntityDataValue],
) -> bool {
    if !crate::entities::is_vanilla_can_equip_saddle_type(entity_type_id) {
        return false;
    }
    if crate::entities::is_vanilla_horse_slot_always_active_type(entity_type_id) {
        return true;
    }
    !mount_entity_is_ageable_baby(data_values)
        && (entity_data_byte(data_values, VANILLA_MOUNT_TAME_FLAGS_DATA_ID, 0)
            & VANILLA_ABSTRACT_HORSE_TAME_FLAG)
            != 0
}

fn mount_horse_body_slot_kind(entity_type_id: i32) -> Option<MountArmorSlotKind> {
    if crate::entities::is_vanilla_llama_type(entity_type_id) {
        Some(MountArmorSlotKind::Llama)
    } else if crate::entities::is_vanilla_can_wear_horse_armor_type(entity_type_id) {
        Some(MountArmorSlotKind::Horse)
    } else {
        None
    }
}

fn mount_nautilus_can_use_equipment_slots(data_values: &[ProtocolEntityDataValue]) -> bool {
    !mount_entity_is_ageable_baby(data_values)
        && (entity_data_byte(data_values, VANILLA_MOUNT_TAME_FLAGS_DATA_ID, 0)
            & VANILLA_TAMABLE_ANIMAL_TAME_FLAG)
            != 0
}

fn mount_entity_is_ageable_baby(data_values: &[ProtocolEntityDataValue]) -> bool {
    entity_data_bool(data_values, VANILLA_AGEABLE_MOB_BABY_DATA_ID, false)
}

fn entity_data_bool(data_values: &[ProtocolEntityDataValue], data_id: u8, fallback: bool) -> bool {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Boolean(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(fallback)
}

fn entity_data_byte(data_values: &[ProtocolEntityDataValue], data_id: u8, fallback: i8) -> i8 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Byte(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(fallback)
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

fn item_stack_has_piercing_weapon(
    stack: &ProtocolItemStackSummary,
    default_piercing_weapon_item_ids: &BTreeSet<i32>,
) -> bool {
    if item_stack_is_empty(stack)
        || stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_PIERCING_WEAPON_COMPONENT_ID)
    {
        return false;
    }

    let Some(item_id) = stack.item_id.filter(|item_id| *item_id >= 0) else {
        return false;
    };

    default_piercing_weapon_item_ids.contains(&item_id)
        || stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_PIERCING_WEAPON_COMPONENT_ID)
}

fn item_stack_has_map_id(stack: &ProtocolItemStackSummary) -> bool {
    if item_stack_is_empty(stack)
        || stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_MAP_ID_COMPONENT_ID)
    {
        return false;
    }

    stack.component_patch.map_id.is_some()
        || stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_MAP_ID_COMPONENT_ID)
}

fn item_stack_attack_range(
    stack: &ProtocolItemStackSummary,
    default_item_attack_ranges: &BTreeMap<i32, ItemAttackRange>,
) -> Option<ItemAttackRange> {
    if item_stack_is_empty(stack)
        || stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_ATTACK_RANGE_COMPONENT_ID)
    {
        return None;
    }

    if let Some(attack_range) = stack.component_patch.attack_range {
        return Some(item_attack_range_from_protocol(attack_range));
    }

    let item_id = stack.item_id.filter(|item_id| *item_id >= 0)?;
    default_item_attack_ranges.get(&item_id).copied()
}

fn item_attack_range_from_protocol(attack_range: ProtocolAttackRangeSummary) -> ItemAttackRange {
    ItemAttackRange {
        min_reach: attack_range.min_reach,
        max_reach: attack_range.max_reach,
        min_creative_reach: attack_range.min_creative_reach,
        max_creative_reach: attack_range.max_creative_reach,
        hitbox_margin: attack_range.hitbox_margin,
        mob_factor: attack_range.mob_factor,
    }
}

fn item_stack_use_effects(
    stack: &ProtocolItemStackSummary,
    default_item_use_effects: &BTreeMap<i32, ItemUseEffects>,
) -> Option<ItemUseEffects> {
    if item_stack_is_empty(stack) {
        return None;
    }

    if stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_USE_EFFECTS_COMPONENT_ID)
    {
        return Some(ItemUseEffects::default());
    }

    if let Some(effects) = stack.component_patch.use_effects {
        return Some(ItemUseEffects {
            can_sprint: effects.can_sprint,
            interact_vibrations: effects.interact_vibrations,
            speed_multiplier: effects.speed_multiplier,
        });
    }

    let default_effects = stack
        .item_id
        .filter(|item_id| *item_id >= 0)
        .and_then(|item_id| default_item_use_effects.get(&item_id).copied())
        .unwrap_or_default();
    Some(default_effects)
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

fn inventory_menu_slot_to_inventory_slot(slot: i16) -> Option<i32> {
    match slot {
        INVENTORY_MENU_HOTBAR_START..=44 => Some(i32::from(slot - INVENTORY_MENU_HOTBAR_START)),
        9..=35 => Some(i32::from(slot)),
        5..=8 => Some(i32::from(44 - slot)),
        INVENTORY_MENU_OFFHAND_SLOT => Some(PLAYER_OFFHAND_SLOT),
        _ => None,
    }
}

fn changed_hashed_slots(
    before: &[ContainerSlot],
    after: &[ContainerSlot],
) -> Result<BTreeMap<i16, ProtocolHashedStack>, ContainerClickBuildError> {
    let mut changed = BTreeMap::new();
    for slot in after {
        if before
            .iter()
            .find(|before| before.slot == slot.slot)
            .is_some_and(|before| before.item == slot.item)
        {
            continue;
        }
        let hashed = hashed_stack_from_summary(&slot.item)
            .ok_or(ContainerClickBuildError::UnhashableChangedSlot(slot.slot))?;
        changed.insert(slot.slot, hashed);
    }
    Ok(changed)
}

fn inventory_menu_result_was_taken(before: &[ContainerSlot], after: &[ContainerSlot]) -> bool {
    let Some(before_result) = container_slot_item(before, 0) else {
        return false;
    };
    if item_stack_is_empty(before_result) {
        return false;
    }

    let Some(after_result) = container_slot_item(after, 0) else {
        return true;
    };
    if item_stack_is_empty(after_result) {
        return true;
    }
    !same_item_same_components(before_result, after_result)
        || after_result.count < before_result.count
}

fn apply_inventory_menu_result_take_side_effects(slots: &mut [ContainerSlot]) {
    let input_slot_nums = inventory_menu_non_empty_crafting_slot_nums(slots);
    apply_inventory_menu_result_take_side_effects_for_slots(slots, &input_slot_nums);
}

fn inventory_menu_non_empty_crafting_slot_nums(slots: &[ContainerSlot]) -> Vec<i16> {
    (1..=4)
        .filter(|slot_num| {
            slots
                .iter()
                .find(|slot| slot.slot == *slot_num)
                .is_some_and(|slot| item_stack_is_non_empty(&slot.item))
        })
        .collect()
}

fn inventory_menu_inputs_can_take_result(slots: &[ContainerSlot], input_slot_nums: &[i16]) -> bool {
    !input_slot_nums.is_empty()
        && input_slot_nums.iter().all(|slot_num| {
            slots
                .iter()
                .find(|slot| slot.slot == *slot_num)
                .is_some_and(|slot| item_stack_is_non_empty(&slot.item))
        })
}

fn apply_inventory_menu_result_take_side_effects_for_slots(
    slots: &mut [ContainerSlot],
    input_slot_nums: &[i16],
) {
    for slot_num in input_slot_nums {
        let Some(slot) = slots.iter_mut().find(|slot| slot.slot == *slot_num) else {
            continue;
        };
        if item_stack_is_empty(&slot.item) {
            continue;
        }
        slot.item.count -= 1;
        normalize_item_stack(&mut slot.item);
        normalize_container_slot_selection(slot);
    }
}

fn container_slot_item(
    slots: &[ContainerSlot],
    slot_num: i16,
) -> Option<&ProtocolItemStackSummary> {
    slots
        .iter()
        .find(|slot| slot.slot == slot_num)
        .map(|slot| &slot.item)
}

fn apply_pickup_click_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    slot_num: i16,
    button_num: i8,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    match slot_num {
        -999 => {
            apply_outside_pickup_click(cursor, button_num);
        }
        slot_num if slot_num >= 0 => {
            let Some(slot) = slots.iter_mut().find(|slot| slot.slot == slot_num) else {
                return;
            };
            apply_slot_pickup_click(
                container_id,
                slot_num,
                &mut slot.item,
                cursor,
                button_num,
                default_item_max_stack_sizes,
            );
            slot.local_selected_bundle_item_index = normalize_local_selected_bundle_item_index(
                slot.local_selected_bundle_item_index,
                &slot.item,
            );
        }
        _ => {}
    }
}

fn apply_quick_craft_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    quick_craft: &mut LocalQuickCraftState,
    slot_num: i16,
    button_num: i8,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if container_id != INVENTORY_MENU_CONTAINER_ID {
        return;
    }

    let expected_status = quick_craft.status;
    quick_craft.status = quickcraft_header(button_num);
    if (expected_status != QUICKCRAFT_HEADER_CONTINUE
        || quick_craft.status != QUICKCRAFT_HEADER_END)
        && expected_status != quick_craft.status
    {
        quick_craft.reset();
        return;
    }
    if item_stack_is_empty(cursor) {
        quick_craft.reset();
        return;
    }

    match quick_craft.status {
        QUICKCRAFT_HEADER_START => {
            quick_craft.quickcraft_type = quickcraft_type(button_num);
            if local_survival_quickcraft_type_is_valid(quick_craft.quickcraft_type) {
                quick_craft.status = QUICKCRAFT_HEADER_CONTINUE;
                quick_craft.slots.clear();
            } else {
                quick_craft.reset();
            }
        }
        QUICKCRAFT_HEADER_CONTINUE => {
            let Some(slot) = slots.iter().find(|slot| slot.slot == slot_num) else {
                return;
            };
            if quick_craft_slot_can_accept(
                container_id,
                slot_num,
                &slot.item,
                cursor,
                default_item_max_stack_sizes,
            ) && cursor.count > quick_craft.slots.len() as i32
                && !quick_craft.slots.contains(&slot_num)
            {
                quick_craft.slots.push(slot_num);
            }
        }
        QUICKCRAFT_HEADER_END => {
            finish_quick_craft(
                container_id,
                slots,
                cursor,
                quick_craft,
                default_item_max_stack_sizes,
            );
        }
        _ => quick_craft.reset(),
    }
}

fn apply_clone_click_to_slots(
    slots: &[ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    slot_num: i16,
    instabuild: bool,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !instabuild || !item_stack_is_empty(cursor) || slot_num < 0 {
        return;
    }
    let Some(slot_item) = container_slot_item(slots, slot_num) else {
        return;
    };
    if item_stack_is_empty(slot_item) {
        return;
    }

    *cursor = slot_item.clone();
    cursor.count = item_stack_max_stack_size(slot_item, default_item_max_stack_sizes);
    normalize_item_stack(cursor);
}

fn finish_quick_craft(
    container_id: i32,
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    quick_craft: &mut LocalQuickCraftState,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    let selected_slots = quick_craft.slots.clone();
    if selected_slots.is_empty() {
        quick_craft.reset();
        return;
    }

    let quickcraft_type = quick_craft.quickcraft_type;
    if selected_slots.len() == 1 {
        quick_craft.reset();
        apply_pickup_click_to_slots(
            container_id,
            slots,
            cursor,
            selected_slots[0],
            quickcraft_type,
            default_item_max_stack_sizes,
        );
        return;
    }
    if !local_survival_quickcraft_type_is_valid(quickcraft_type) {
        quick_craft.reset();
        return;
    }

    let source = cursor.clone();
    if item_stack_is_empty(&source) {
        quick_craft.reset();
        return;
    }

    let slot_count = selected_slots.len() as i32;
    let mut remaining = cursor.count;
    for selected_slot in selected_slots {
        if cursor.count < slot_count {
            continue;
        }
        let Some(slot_index) = slots.iter().position(|slot| slot.slot == selected_slot) else {
            continue;
        };
        if !quick_craft_slot_can_accept(
            container_id,
            selected_slot,
            &slots[slot_index].item,
            cursor,
            default_item_max_stack_sizes,
        ) {
            continue;
        }

        let carry = if item_stack_is_empty(&slots[slot_index].item) {
            0
        } else {
            slots[slot_index].item.count
        };
        let max_size = item_stack_max_stack_size(&source, default_item_max_stack_sizes).min(
            container_slot_max_stack_size(
                container_id,
                selected_slot,
                &source,
                default_item_max_stack_sizes,
            ),
        );
        let place_count = quickcraft_place_count(
            slot_count,
            quickcraft_type,
            &source,
            default_item_max_stack_sizes,
        );
        let new_count = (place_count + carry).min(max_size);
        remaining -= new_count - carry;

        let mut replacement = source.clone();
        replacement.count = new_count;
        normalize_item_stack(&mut replacement);
        slots[slot_index].item = replacement;
        normalize_container_slot_selection(&mut slots[slot_index]);
    }

    cursor.count = remaining;
    normalize_item_stack(cursor);
    quick_craft.reset();
}

fn quick_craft_slot_can_accept(
    container_id: i32,
    slot_num: i16,
    slot_item: &ProtocolItemStackSummary,
    cursor: &ProtocolItemStackSummary,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> bool {
    if container_slot_max_stack_size(container_id, slot_num, cursor, default_item_max_stack_sizes)
        <= 0
    {
        return false;
    }
    if item_stack_is_empty(slot_item) {
        return true;
    }
    same_item_same_components(slot_item, cursor)
        && slot_item.count <= item_stack_max_stack_size(cursor, default_item_max_stack_sizes)
}

fn quickcraft_place_count(
    slot_count: i32,
    quickcraft_type: i8,
    source: &ProtocolItemStackSummary,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> i32 {
    match quickcraft_type {
        QUICKCRAFT_TYPE_CHARITABLE => source.count / slot_count,
        QUICKCRAFT_TYPE_GREEDY => 1,
        QUICKCRAFT_TYPE_CLONE => item_stack_max_stack_size(source, default_item_max_stack_sizes),
        _ => source.count,
    }
}

fn local_survival_quickcraft_type_is_valid(quickcraft_type: i8) -> bool {
    matches!(
        quickcraft_type,
        QUICKCRAFT_TYPE_CHARITABLE | QUICKCRAFT_TYPE_GREEDY
    )
}

fn quickcraft_header(mask: i8) -> i8 {
    mask & 3
}

fn quickcraft_type(mask: i8) -> i8 {
    (mask >> 2) & 3
}

#[cfg(test)]
fn quickcraft_mask(header: i8, quickcraft_type: i8) -> i8 {
    (header & 3) | ((quickcraft_type & 3) << 2)
}

fn apply_throw_click_to_slots(
    slots: &mut [ContainerSlot],
    cursor: &ProtocolItemStackSummary,
    slot_num: i16,
    button_num: i8,
) {
    if !item_stack_is_empty(cursor) || slot_num < 0 || !matches!(button_num, 0 | 1) {
        return;
    }
    let Some(slot) = slots.iter_mut().find(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slot.item) {
        return;
    }
    if button_num == 0 {
        slot.item.count -= 1;
        normalize_item_stack(&mut slot.item);
    } else {
        slot.item = ProtocolItemStackSummary::empty();
    }
    normalize_container_slot_selection(slot);
}

fn apply_pickup_all_to_slots(
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    slot_num: i16,
    button_num: i8,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if slot_num < 0 || item_stack_is_empty(cursor) {
        return;
    }
    let Some(clicked_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if !item_stack_is_empty(&slots[clicked_index].item) {
        return;
    }

    let max_stack_size = item_stack_max_stack_size(cursor, default_item_max_stack_sizes);
    if cursor.count >= max_stack_size {
        return;
    }

    let mut indices = (0..slots.len()).collect::<Vec<_>>();
    if button_num != 0 {
        indices.reverse();
    }

    for pass in 0..2 {
        for index in indices.iter().copied() {
            if cursor.count >= max_stack_size {
                return;
            }
            let slot = &mut slots[index];
            if item_stack_is_empty(&slot.item) || !same_item_same_components(&slot.item, cursor) {
                continue;
            }
            if pass == 0
                && slot.item.count
                    == item_stack_max_stack_size(&slot.item, default_item_max_stack_sizes)
            {
                continue;
            }

            let moved = slot.item.count.min(max_stack_size - cursor.count);
            if moved <= 0 {
                continue;
            }
            slot.item.count -= moved;
            cursor.count += moved;
            normalize_item_stack(&mut slot.item);
            normalize_container_slot_selection(slot);
        }
    }
}

fn apply_swap_click_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    cursor: &ProtocolItemStackSummary,
    slot_num: i16,
    button_num: i8,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if container_id != INVENTORY_MENU_CONTAINER_ID || !item_stack_is_empty(cursor) || slot_num < 0 {
        return;
    }
    let Some(source_slot_num) = swap_button_inventory_menu_slot(button_num) else {
        return;
    };
    if source_slot_num == slot_num {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == source_slot_num) else {
        return;
    };
    let Some(target_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };

    let source_item = slots[source_index].item.clone();
    let target_item = slots[target_index].item.clone();
    if item_stack_is_empty(&source_item) && item_stack_is_empty(&target_item) {
        return;
    }

    if item_stack_is_empty(&source_item) {
        if container_slot_max_stack_size(
            container_id,
            source_slot_num,
            &target_item,
            default_item_max_stack_sizes,
        ) <= 0
        {
            return;
        }
        slots[source_index].item = target_item;
        slots[target_index].item = ProtocolItemStackSummary::empty();
        normalize_container_slot_selection(&mut slots[source_index]);
        normalize_container_slot_selection(&mut slots[target_index]);
        return;
    }

    let target_max_stack_size = container_slot_max_stack_size(
        container_id,
        slot_num,
        &source_item,
        default_item_max_stack_sizes,
    );
    if target_max_stack_size <= 0 {
        return;
    }

    if item_stack_is_empty(&target_item) {
        move_between_container_slots(slots, source_index, target_index, target_max_stack_size);
        return;
    }

    let source_max_stack_size = container_slot_max_stack_size(
        container_id,
        source_slot_num,
        &target_item,
        default_item_max_stack_sizes,
    );
    if source_max_stack_size <= 0 {
        return;
    }

    if source_item.count <= target_max_stack_size && target_item.count <= source_max_stack_size {
        slots[source_index].item = target_item;
        slots[target_index].item = source_item;
        normalize_container_slot_selection(&mut slots[source_index]);
        normalize_container_slot_selection(&mut slots[target_index]);
        return;
    }

    let moved = source_item.count.min(target_max_stack_size);
    let mut target_replacement = source_item.clone();
    target_replacement.count = moved;
    let mut source_remainder = source_item;
    source_remainder.count -= moved;
    normalize_item_stack(&mut source_remainder);
    slots[source_index].item = source_remainder;
    slots[target_index].item = target_replacement;
    normalize_container_slot_selection(&mut slots[source_index]);
    normalize_container_slot_selection(&mut slots[target_index]);

    let mut displaced = target_item;
    move_item_stack_to_slots(
        container_id,
        slots,
        target_index,
        &mut displaced,
        INVENTORY_MENU_MAIN_START,
        INVENTORY_MENU_HOTBAR_END,
        false,
        default_item_max_stack_sizes,
    );
}

fn move_between_container_slots(
    slots: &mut [ContainerSlot],
    source_index: usize,
    target_index: usize,
    max_count: i32,
) {
    let mut source = slots[source_index].item.clone();
    let amount = source.count.min(max_count);
    let mut target = source.clone();
    target.count = amount;
    source.count -= amount;
    normalize_item_stack(&mut source);
    slots[source_index].item = source;
    slots[target_index].item = target;
    normalize_container_slot_selection(&mut slots[source_index]);
    normalize_container_slot_selection(&mut slots[target_index]);
}

fn swap_button_inventory_menu_slot(button_num: i8) -> Option<i16> {
    match button_num {
        0..=8 => Some(INVENTORY_MENU_HOTBAR_START + i16::from(button_num)),
        40 => Some(INVENTORY_MENU_OFFHAND_SLOT),
        _ => None,
    }
}

fn generic_9x_container_slot_count(menu_type_id: Option<i32>) -> Option<i16> {
    let menu_type_id = menu_type_id?;
    (VANILLA_MENU_TYPE_GENERIC_9X1_ID..=VANILLA_MENU_TYPE_GENERIC_9X6_ID)
        .contains(&menu_type_id)
        .then_some((menu_type_id - VANILLA_MENU_TYPE_GENERIC_9X1_ID + 1) as i16)
        .map(|rows| rows * GENERIC_CONTAINER_SLOT_COUNT_PER_ROW)
}

fn generic_3x3_container_slot_count(menu_type_id: Option<i32>) -> Option<i16> {
    (menu_type_id == Some(VANILLA_MENU_TYPE_GENERIC_3X3_ID))
        .then_some(GENERIC_3X3_CONTAINER_SLOT_COUNT)
}

fn hopper_container_slot_count(menu_type_id: Option<i32>) -> Option<i16> {
    (menu_type_id == Some(VANILLA_MENU_TYPE_HOPPER_ID)).then_some(HOPPER_CONTAINER_SLOT_COUNT)
}

fn shulker_box_container_slot_count(menu_type_id: Option<i32>) -> Option<i16> {
    (menu_type_id == Some(VANILLA_MENU_TYPE_SHULKER_BOX_ID))
        .then_some(SHULKER_BOX_CONTAINER_SLOT_COUNT)
}

fn menu_result_slot_requires_server_authority(menu_type_id: Option<i32>, slot_num: i16) -> bool {
    matches!(
        (menu_type_id, slot_num),
        (Some(VANILLA_MENU_TYPE_ANVIL_ID), ANVIL_RESULT_SLOT)
            | (
                Some(VANILLA_MENU_TYPE_CRAFTING_ID),
                CRAFTING_MENU_RESULT_SLOT
            )
            | (
                Some(VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID),
                CARTOGRAPHY_TABLE_RESULT_SLOT
            )
            | (Some(VANILLA_MENU_TYPE_CRAFTER_ID), CRAFTER_RESULT_SLOT)
            | (Some(VANILLA_MENU_TYPE_LOOM_ID), LOOM_RESULT_SLOT)
            | (
                Some(VANILLA_MENU_TYPE_GRINDSTONE_ID),
                GRINDSTONE_RESULT_SLOT
            )
            | (Some(VANILLA_MENU_TYPE_SMITHING_ID), SMITHING_RESULT_SLOT)
            | (Some(VANILLA_MENU_TYPE_MERCHANT_ID), MERCHANT_RESULT_SLOT)
            | (
                Some(VANILLA_MENU_TYPE_STONECUTTER_ID),
                STONECUTTER_RESULT_SLOT
            )
    )
}

fn furnace_family_menu_type(menu_type_id: Option<i32>) -> Option<i32> {
    let menu_type_id = menu_type_id?;
    matches!(
        menu_type_id,
        VANILLA_MENU_TYPE_BLAST_FURNACE_ID
            | VANILLA_MENU_TYPE_FURNACE_ID
            | VANILLA_MENU_TYPE_SMOKER_ID
    )
    .then_some(menu_type_id)
}

fn apply_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    default_item_equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if container_id != INVENTORY_MENU_CONTAINER_ID || slot_num < 0 {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }
    let source_item = slots[source_index].item.clone();
    let Some((start_slot, end_slot, backwards)) = inventory_menu_quick_move_target_range(
        slot_num,
        &source_item,
        slots,
        default_item_equipment_slots,
    ) else {
        return;
    };

    let mut moving = source_item;
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

fn apply_inventory_menu_result_quick_move_to_slots(
    slots: &mut [ContainerSlot],
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    let Some(result_index) = slots.iter().position(|slot| slot.slot == 0) else {
        return;
    };
    if item_stack_is_empty(&slots[result_index].item) {
        return;
    }

    let result_template = slots[result_index].item.clone();
    let input_slot_nums = inventory_menu_non_empty_crafting_slot_nums(slots);
    if input_slot_nums.is_empty() {
        let mut moving = result_template;
        if move_item_stack_to_slots(
            INVENTORY_MENU_CONTAINER_ID,
            slots,
            result_index,
            &mut moving,
            INVENTORY_MENU_MAIN_START,
            INVENTORY_MENU_HOTBAR_END,
            true,
            default_item_max_stack_sizes,
        ) {
            normalize_item_stack(&mut moving);
            slots[result_index].item = moving;
            normalize_container_slot_selection(&mut slots[result_index]);
        }
        return;
    }

    let max_crafts = input_slot_nums
        .iter()
        .filter_map(|slot_num| {
            slots
                .iter()
                .find(|slot| slot.slot == *slot_num)
                .map(|slot| slot.item.count)
        })
        .min()
        .unwrap_or(0)
        .max(0);

    for _ in 0..max_crafts {
        let result_still_same = container_slot_item(slots, 0).is_some_and(|item| {
            item_stack_is_non_empty(item) && same_item_same_components(item, &result_template)
        });
        if !result_still_same || !inventory_menu_inputs_can_take_result(slots, &input_slot_nums) {
            break;
        }

        let mut candidate_slots = slots.to_vec();
        candidate_slots[result_index].item = result_template.clone();
        let mut moving = result_template.clone();
        if !move_item_stack_to_slots(
            INVENTORY_MENU_CONTAINER_ID,
            &mut candidate_slots,
            result_index,
            &mut moving,
            INVENTORY_MENU_MAIN_START,
            INVENTORY_MENU_HOTBAR_END,
            true,
            default_item_max_stack_sizes,
        ) {
            break;
        }

        candidate_slots[result_index].item = ProtocolItemStackSummary::empty();
        normalize_container_slot_selection(&mut candidate_slots[result_index]);
        apply_inventory_menu_result_take_side_effects_for_slots(
            &mut candidate_slots,
            &input_slot_nums,
        );
        if inventory_menu_inputs_can_take_result(&candidate_slots, &input_slot_nums) {
            candidate_slots[result_index].item = result_template.clone();
            normalize_container_slot_selection(&mut candidate_slots[result_index]);
        }
        slots.clone_from_slice(&candidate_slots);
    }
}

fn apply_furnace_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    menu_type_id: Option<i32>,
    recipe_property_sets: &BTreeMap<String, Vec<i32>>,
    furnace_fuel_item_ids: &BTreeSet<i32>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if slot_num < 0
        || slot_num >= FURNACE_CONTAINER_SLOT_COUNT + GENERIC_CONTAINER_PLAYER_SLOT_COUNT
    {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let target = match slot_num {
        2 => Some((3, 39, true)),
        0 | 1 => Some((3, 39, false)),
        3..=38 if furnace_can_smelt(menu_type_id, &source_item, recipe_property_sets) => {
            Some((0, 1, false))
        }
        3..=38 if furnace_is_fuel(&source_item, furnace_fuel_item_ids) => Some((1, 2, false)),
        3..=29 => Some((30, 39, false)),
        30..=38 => Some((3, 30, false)),
        _ => None,
    };
    let Some((start_slot, end_slot, backwards)) = target else {
        return;
    };

    let mut moving = source_item;
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

fn apply_generic_container_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    container_slot_count: i16,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if slot_num < 0 {
        return;
    }
    let total_slot_count = container_slot_count + GENERIC_CONTAINER_PLAYER_SLOT_COUNT;
    if slot_num >= total_slot_count {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }
    let (start_slot, end_slot, backwards) = if slot_num < container_slot_count {
        (container_slot_count, total_slot_count, true)
    } else {
        (0, container_slot_count, false)
    };

    let mut moving = slots[source_index].item.clone();
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

fn apply_crafting_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if slot_num <= CRAFTING_MENU_RESULT_SLOT || slot_num >= CRAFTING_MENU_TOTAL_SLOT_COUNT {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let mut moving = slots[source_index].item.clone();
    let moved = if (CRAFTING_MENU_PLAYER_MAIN_START..CRAFTING_MENU_HOTBAR_END).contains(&slot_num) {
        move_item_stack_to_slots(
            container_id,
            slots,
            source_index,
            &mut moving,
            CRAFTING_MENU_CRAFT_SLOT_START,
            CRAFTING_MENU_CRAFT_SLOT_END,
            false,
            default_item_max_stack_sizes,
        ) || if (CRAFTING_MENU_PLAYER_MAIN_START..CRAFTING_MENU_PLAYER_MAIN_END).contains(&slot_num)
        {
            move_item_stack_to_slots(
                container_id,
                slots,
                source_index,
                &mut moving,
                CRAFTING_MENU_HOTBAR_START,
                CRAFTING_MENU_HOTBAR_END,
                false,
                default_item_max_stack_sizes,
            )
        } else {
            move_item_stack_to_slots(
                container_id,
                slots,
                source_index,
                &mut moving,
                CRAFTING_MENU_PLAYER_MAIN_START,
                CRAFTING_MENU_PLAYER_MAIN_END,
                false,
                default_item_max_stack_sizes,
            )
        }
    } else {
        move_item_stack_to_slots(
            container_id,
            slots,
            source_index,
            &mut moving,
            CRAFTING_MENU_PLAYER_MAIN_START,
            CRAFTING_MENU_HOTBAR_END,
            false,
            default_item_max_stack_sizes,
        )
    };

    if moved {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

fn apply_crafter_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    disabled_slots: &BTreeSet<i16>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..CRAFTER_TOTAL_SLOT_COUNT).contains(&slot_num) || slot_num == CRAFTER_RESULT_SLOT {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let (start_slot, end_slot, backwards) = if slot_num < CRAFTER_GRID_SLOT_COUNT {
        (CRAFTER_PLAYER_MAIN_START, CRAFTER_HOTBAR_END, true)
    } else {
        (0, CRAFTER_GRID_SLOT_COUNT, false)
    };

    let mut moving = source_item;
    if move_item_stack_to_slots_where(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        |slot| !disabled_slots.contains(&slot),
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

fn crafter_disabled_slots(data_values: &[ContainerDataValue]) -> BTreeSet<i16> {
    data_values
        .iter()
        .filter_map(|value| {
            ((0..CRAFTER_GRID_SLOT_COUNT).contains(&value.id) && value.value == 1)
                .then_some(value.id)
        })
        .collect()
}

fn anvil_quick_move_requires_server_authority(slots: &[ContainerSlot], slot_num: i16) -> bool {
    if !(0..ANVIL_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return false;
    }
    if slot_num == ANVIL_RESULT_SLOT {
        return true;
    }
    if matches!(slot_num, ANVIL_INPUT_SLOT | ANVIL_ADDITIONAL_SLOT) {
        return false;
    }
    inventory_menu_slot_has_item(slots, slot_num)
}

fn apply_anvil_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !matches!(slot_num, ANVIL_INPUT_SLOT | ANVIL_ADDITIONAL_SLOT) {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let mut moving = slots[source_index].item.clone();
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        ANVIL_PLAYER_MAIN_START,
        ANVIL_HOTBAR_END,
        false,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

fn apply_beacon_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    item_tags: Option<&RegistryTagState>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..BEACON_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let target = match slot_num {
        BEACON_PAYMENT_SLOT => Some((BEACON_PLAYER_MAIN_START, BEACON_HOTBAR_END, true)),
        slot if (BEACON_PLAYER_MAIN_START..BEACON_HOTBAR_END).contains(&slot)
            && !inventory_menu_slot_has_item(slots, BEACON_PAYMENT_SLOT)
            && source_item.count == 1
            && item_stack_in_item_tag(&source_item, item_tags, BEACON_PAYMENT_ITEM_TAG) =>
        {
            Some((BEACON_PAYMENT_SLOT, BEACON_PAYMENT_SLOT + 1, false))
        }
        slot if (BEACON_PLAYER_MAIN_START..BEACON_PLAYER_MAIN_END).contains(&slot) => {
            Some((BEACON_HOTBAR_START, BEACON_HOTBAR_END, false))
        }
        slot if (BEACON_HOTBAR_START..BEACON_HOTBAR_END).contains(&slot) => {
            Some((BEACON_PLAYER_MAIN_START, BEACON_PLAYER_MAIN_END, false))
        }
        _ => None,
    };
    let Some((start_slot, end_slot, backwards)) = target else {
        return;
    };

    let mut moving = source_item;
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

fn enchantment_quick_move_requires_server_authority(
    slots: &[ContainerSlot],
    slot_num: i16,
    enchantment_lapis_lazuli_item_ids: &BTreeSet<i32>,
) -> bool {
    if !(0..ENCHANTMENT_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return false;
    }
    if matches!(slot_num, ENCHANTMENT_INPUT_SLOT | ENCHANTMENT_LAPIS_SLOT) {
        return false;
    }
    let Some(source_item) = container_slot_item(slots, slot_num) else {
        return false;
    };
    if item_stack_is_empty(source_item) {
        return false;
    }
    if enchantment_lapis_lazuli_item_ids.is_empty() {
        return true;
    }
    !item_stack_item_id_in_set(source_item, enchantment_lapis_lazuli_item_ids)
}

fn apply_enchantment_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    enchantment_lapis_lazuli_item_ids: &BTreeSet<i32>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..ENCHANTMENT_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let target = match slot_num {
        ENCHANTMENT_INPUT_SLOT | ENCHANTMENT_LAPIS_SLOT => {
            Some((ENCHANTMENT_PLAYER_MAIN_START, ENCHANTMENT_HOTBAR_END, true))
        }
        slot if (ENCHANTMENT_PLAYER_MAIN_START..ENCHANTMENT_HOTBAR_END).contains(&slot)
            && item_stack_item_id_in_set(&source_item, enchantment_lapis_lazuli_item_ids) =>
        {
            Some((ENCHANTMENT_LAPIS_SLOT, ENCHANTMENT_PLAYER_MAIN_START, true))
        }
        _ => None,
    };
    let Some((start_slot, end_slot, backwards)) = target else {
        return;
    };

    let mut moving = source_item;
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

fn apply_brewing_stand_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    item_tags: Option<&RegistryTagState>,
    brewing_potion_item_ids: &BTreeSet<i32>,
    brewing_ingredient_item_ids: &BTreeSet<i32>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..BREWING_STAND_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let mut moving = source_item.clone();
    let moved = if (BREWING_STAND_BOTTLE_SLOT_START..=BREWING_STAND_FUEL_SLOT).contains(&slot_num) {
        move_item_stack_to_slots(
            container_id,
            slots,
            source_index,
            &mut moving,
            BREWING_STAND_PLAYER_MAIN_START,
            BREWING_STAND_HOTBAR_END,
            true,
            default_item_max_stack_sizes,
        )
    } else if (BREWING_STAND_PLAYER_MAIN_START..BREWING_STAND_HOTBAR_END).contains(&slot_num) {
        let is_ingredient = item_stack_item_id_in_set(&source_item, brewing_ingredient_item_ids);
        if item_stack_in_item_tag(&source_item, item_tags, BREWING_STAND_FUEL_ITEM_TAG) {
            let fuel_moved = move_item_stack_to_slots(
                container_id,
                slots,
                source_index,
                &mut moving,
                BREWING_STAND_FUEL_SLOT,
                BREWING_STAND_FUEL_SLOT + 1,
                false,
                default_item_max_stack_sizes,
            );
            fuel_moved
                || (is_ingredient
                    && move_item_stack_to_slots(
                        container_id,
                        slots,
                        source_index,
                        &mut moving,
                        BREWING_STAND_INGREDIENT_SLOT,
                        BREWING_STAND_INGREDIENT_SLOT + 1,
                        false,
                        default_item_max_stack_sizes,
                    ))
        } else if is_ingredient {
            move_item_stack_to_slots(
                container_id,
                slots,
                source_index,
                &mut moving,
                BREWING_STAND_INGREDIENT_SLOT,
                BREWING_STAND_INGREDIENT_SLOT + 1,
                false,
                default_item_max_stack_sizes,
            )
        } else if item_stack_item_id_in_set(&source_item, brewing_potion_item_ids) {
            move_item_stack_to_slots_where_with_limit(
                container_id,
                slots,
                source_index,
                &mut moving,
                BREWING_STAND_BOTTLE_SLOT_START,
                BREWING_STAND_BOTTLE_SLOT_END,
                false,
                |_| true,
                brewing_stand_slot_max_stack_size,
                default_item_max_stack_sizes,
            )
        } else if (BREWING_STAND_PLAYER_MAIN_START..BREWING_STAND_PLAYER_MAIN_END)
            .contains(&slot_num)
        {
            move_item_stack_to_slots(
                container_id,
                slots,
                source_index,
                &mut moving,
                BREWING_STAND_HOTBAR_START,
                BREWING_STAND_HOTBAR_END,
                false,
                default_item_max_stack_sizes,
            )
        } else {
            move_item_stack_to_slots(
                container_id,
                slots,
                source_index,
                &mut moving,
                BREWING_STAND_PLAYER_MAIN_START,
                BREWING_STAND_PLAYER_MAIN_END,
                false,
                default_item_max_stack_sizes,
            )
        }
    } else {
        false
    };

    if moved {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

fn brewing_stand_slot_max_stack_size(
    slot_num: i16,
    _stack: &ProtocolItemStackSummary,
    base_max_stack_size: i32,
) -> i32 {
    if (BREWING_STAND_BOTTLE_SLOT_START..BREWING_STAND_BOTTLE_SLOT_END).contains(&slot_num) {
        base_max_stack_size.min(1)
    } else {
        base_max_stack_size
    }
}

fn apply_grindstone_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..GRINDSTONE_TOTAL_SLOT_COUNT).contains(&slot_num) || slot_num == GRINDSTONE_RESULT_SLOT {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let target = match slot_num {
        GRINDSTONE_INPUT_SLOT | GRINDSTONE_ADDITIONAL_SLOT => {
            Some((GRINDSTONE_PLAYER_MAIN_START, GRINDSTONE_HOTBAR_END, false))
        }
        slot if (GRINDSTONE_PLAYER_MAIN_START..GRINDSTONE_PLAYER_MAIN_END).contains(&slot) => {
            Some((GRINDSTONE_HOTBAR_START, GRINDSTONE_HOTBAR_END, false))
        }
        slot if (GRINDSTONE_HOTBAR_START..GRINDSTONE_HOTBAR_END).contains(&slot) => Some((
            GRINDSTONE_PLAYER_MAIN_START,
            GRINDSTONE_PLAYER_MAIN_END,
            false,
        )),
        _ => None,
    };
    let Some((start_slot, end_slot, backwards)) = target else {
        return;
    };

    let mut moving = source_item;
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

fn grindstone_quick_move_requires_server_authority(slots: &[ContainerSlot], slot_num: i16) -> bool {
    if !(0..GRINDSTONE_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return false;
    }
    if slot_num == GRINDSTONE_RESULT_SLOT {
        return true;
    }
    if matches!(slot_num, GRINDSTONE_INPUT_SLOT | GRINDSTONE_ADDITIONAL_SLOT) {
        return false;
    }
    if !(GRINDSTONE_PLAYER_MAIN_START..GRINDSTONE_HOTBAR_END).contains(&slot_num) {
        return false;
    }
    let inputs_full = inventory_menu_slot_has_item(slots, GRINDSTONE_INPUT_SLOT)
        && inventory_menu_slot_has_item(slots, GRINDSTONE_ADDITIONAL_SLOT);
    !inputs_full
}

fn smithing_quick_move_requires_server_authority(slots: &[ContainerSlot], slot_num: i16) -> bool {
    if !(0..SMITHING_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return false;
    }
    if slot_num == SMITHING_RESULT_SLOT {
        return true;
    }
    if matches!(
        slot_num,
        SMITHING_TEMPLATE_SLOT | SMITHING_BASE_SLOT | SMITHING_ADDITIONAL_SLOT
    ) {
        return false;
    }
    inventory_menu_slot_has_item(slots, slot_num)
}

fn apply_smithing_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !matches!(
        slot_num,
        SMITHING_TEMPLATE_SLOT | SMITHING_BASE_SLOT | SMITHING_ADDITIONAL_SLOT
    ) {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let mut moving = slots[source_index].item.clone();
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        SMITHING_PLAYER_MAIN_START,
        SMITHING_HOTBAR_END,
        false,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

fn cartography_table_quick_move_requires_server_authority(
    slots: &[ContainerSlot],
    slot_num: i16,
    cartography_additional_item_ids: &BTreeSet<i32>,
) -> bool {
    if !(0..CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return false;
    }
    if matches!(
        slot_num,
        CARTOGRAPHY_TABLE_MAP_SLOT | CARTOGRAPHY_TABLE_ADDITIONAL_SLOT
    ) {
        return false;
    }
    if slot_num == CARTOGRAPHY_TABLE_RESULT_SLOT {
        return inventory_menu_slot_has_item(slots, slot_num);
    }
    let Some(source_item) = container_slot_item(slots, slot_num) else {
        return false;
    };
    if item_stack_is_empty(source_item) {
        return false;
    }
    if cartography_additional_item_ids.is_empty() {
        return true;
    }
    item_stack_has_map_id(source_item)
}

fn apply_cartography_table_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    cartography_additional_item_ids: &BTreeSet<i32>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT).contains(&slot_num)
        || slot_num == CARTOGRAPHY_TABLE_RESULT_SLOT
    {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let target = match slot_num {
        CARTOGRAPHY_TABLE_MAP_SLOT | CARTOGRAPHY_TABLE_ADDITIONAL_SLOT => Some((
            CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
            CARTOGRAPHY_TABLE_HOTBAR_END,
            false,
        )),
        slot if (CARTOGRAPHY_TABLE_PLAYER_MAIN_START..CARTOGRAPHY_TABLE_HOTBAR_END)
            .contains(&slot) =>
        {
            if item_stack_has_map_id(&source_item) {
                None
            } else if item_stack_item_id_in_set(&source_item, cartography_additional_item_ids) {
                Some((
                    CARTOGRAPHY_TABLE_ADDITIONAL_SLOT,
                    CARTOGRAPHY_TABLE_RESULT_SLOT,
                    false,
                ))
            } else if (CARTOGRAPHY_TABLE_PLAYER_MAIN_START..CARTOGRAPHY_TABLE_PLAYER_MAIN_END)
                .contains(&slot)
            {
                Some((
                    CARTOGRAPHY_TABLE_HOTBAR_START,
                    CARTOGRAPHY_TABLE_HOTBAR_END,
                    false,
                ))
            } else {
                Some((
                    CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
                    CARTOGRAPHY_TABLE_PLAYER_MAIN_END,
                    false,
                ))
            }
        }
        _ => None,
    };
    let Some((start_slot, end_slot, backwards)) = target else {
        return;
    };

    let mut moving = source_item;
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

fn apply_loom_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    item_tags: Option<&RegistryTagState>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..LOOM_TOTAL_SLOT_COUNT).contains(&slot_num) || slot_num == LOOM_RESULT_SLOT {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let target = match slot_num {
        LOOM_BANNER_SLOT | LOOM_DYE_SLOT | LOOM_PATTERN_SLOT => {
            Some((LOOM_PLAYER_MAIN_START, LOOM_HOTBAR_END, false))
        }
        slot if (LOOM_PLAYER_MAIN_START..LOOM_HOTBAR_END).contains(&slot) => {
            if item_stack_in_item_tag(&source_item, item_tags, LOOM_BANNER_ITEM_TAG) {
                Some((LOOM_BANNER_SLOT, LOOM_DYE_SLOT, false))
            } else if item_stack_in_item_tag(&source_item, item_tags, LOOM_DYE_ITEM_TAG) {
                Some((LOOM_DYE_SLOT, LOOM_PATTERN_SLOT, false))
            } else if item_stack_in_item_tag(&source_item, item_tags, LOOM_PATTERN_ITEM_TAG) {
                Some((LOOM_PATTERN_SLOT, LOOM_RESULT_SLOT, false))
            } else if (LOOM_PLAYER_MAIN_START..LOOM_PLAYER_MAIN_END).contains(&slot) {
                Some((LOOM_HOTBAR_START, LOOM_HOTBAR_END, false))
            } else {
                Some((LOOM_PLAYER_MAIN_START, LOOM_PLAYER_MAIN_END, false))
            }
        }
        _ => None,
    };
    let Some((start_slot, end_slot, backwards)) = target else {
        return;
    };

    let mut moving = source_item;
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

fn merchant_quick_move_requires_server_authority(slot_num: i16) -> bool {
    (0..MERCHANT_TOTAL_SLOT_COUNT).contains(&slot_num) && slot_num == MERCHANT_RESULT_SLOT
}

fn apply_merchant_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..MERCHANT_TOTAL_SLOT_COUNT).contains(&slot_num) || slot_num == MERCHANT_RESULT_SLOT {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let target = match slot_num {
        MERCHANT_PAYMENT_SLOT_1 | MERCHANT_PAYMENT_SLOT_2 => {
            Some((MERCHANT_PLAYER_MAIN_START, MERCHANT_HOTBAR_END, false))
        }
        slot if (MERCHANT_PLAYER_MAIN_START..MERCHANT_PLAYER_MAIN_END).contains(&slot) => {
            Some((MERCHANT_HOTBAR_START, MERCHANT_HOTBAR_END, false))
        }
        slot if (MERCHANT_HOTBAR_START..MERCHANT_HOTBAR_END).contains(&slot) => {
            Some((MERCHANT_PLAYER_MAIN_START, MERCHANT_PLAYER_MAIN_END, false))
        }
        _ => None,
    };
    let Some((start_slot, end_slot, backwards)) = target else {
        return;
    };

    let mut moving = source_item;
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

fn apply_stonecutter_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    stonecutter_recipes: &[ProtocolStonecutterSelectableRecipeSummary],
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..STONECUTTER_TOTAL_SLOT_COUNT).contains(&slot_num) || slot_num == STONECUTTER_RESULT_SLOT
    {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let target = match slot_num {
        STONECUTTER_INPUT_SLOT => {
            Some((STONECUTTER_PLAYER_MAIN_START, STONECUTTER_HOTBAR_END, false))
        }
        slot if (STONECUTTER_PLAYER_MAIN_START..STONECUTTER_HOTBAR_END).contains(&slot)
            && stonecutter_accepts_input(&source_item, stonecutter_recipes) =>
        {
            Some((STONECUTTER_INPUT_SLOT, STONECUTTER_RESULT_SLOT, false))
        }
        slot if (STONECUTTER_PLAYER_MAIN_START..STONECUTTER_PLAYER_MAIN_END).contains(&slot) => {
            Some((STONECUTTER_HOTBAR_START, STONECUTTER_HOTBAR_END, false))
        }
        slot if (STONECUTTER_HOTBAR_START..STONECUTTER_HOTBAR_END).contains(&slot) => Some((
            STONECUTTER_PLAYER_MAIN_START,
            STONECUTTER_PLAYER_MAIN_END,
            false,
        )),
        _ => None,
    };
    let Some((start_slot, end_slot, backwards)) = target else {
        return;
    };

    let mut moving = source_item;
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

fn stonecutter_accepts_input(
    stack: &ProtocolItemStackSummary,
    stonecutter_recipes: &[ProtocolStonecutterSelectableRecipeSummary],
) -> bool {
    let Some(item_id) = stack.item_id else {
        return false;
    };
    stonecutter_recipes
        .iter()
        .any(|recipe| recipe.input.item_ids.contains(&item_id))
}

fn furnace_can_smelt(
    menu_type_id: Option<i32>,
    stack: &ProtocolItemStackSummary,
    recipe_property_sets: &BTreeMap<String, Vec<i32>>,
) -> bool {
    let Some(item_id) = stack.item_id else {
        return false;
    };
    let Some(property_set) = furnace_input_property_set(menu_type_id) else {
        return false;
    };
    recipe_property_sets
        .get(property_set)
        .is_some_and(|items| items.contains(&item_id))
}

fn furnace_input_property_set(menu_type_id: Option<i32>) -> Option<&'static str> {
    match menu_type_id {
        Some(VANILLA_MENU_TYPE_BLAST_FURNACE_ID) => Some("minecraft:blast_furnace_input"),
        Some(VANILLA_MENU_TYPE_FURNACE_ID) => Some("minecraft:furnace_input"),
        Some(VANILLA_MENU_TYPE_SMOKER_ID) => Some("minecraft:smoker_input"),
        _ => None,
    }
}

fn furnace_is_fuel(
    stack: &ProtocolItemStackSummary,
    furnace_fuel_item_ids: &BTreeSet<i32>,
) -> bool {
    stack
        .item_id
        .is_some_and(|item_id| furnace_fuel_item_ids.contains(&item_id))
}

fn item_stack_in_item_tag(
    stack: &ProtocolItemStackSummary,
    item_tags: Option<&RegistryTagState>,
    tag: &str,
) -> bool {
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_tags
        .and_then(|registry| registry.tags.get(tag))
        .is_some_and(|entries| entries.contains(&item_id))
}

fn item_stack_item_id_in_set(stack: &ProtocolItemStackSummary, item_ids: &BTreeSet<i32>) -> bool {
    stack
        .item_id
        .is_some_and(|item_id| item_ids.contains(&item_id))
}

fn inventory_menu_quick_move_target_range(
    slot_num: i16,
    source_item: &ProtocolItemStackSummary,
    slots: &[ContainerSlot],
    default_item_equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
) -> Option<(i16, i16, bool)> {
    match slot_num {
        0 => Some((INVENTORY_MENU_MAIN_START, INVENTORY_MENU_HOTBAR_END, true)),
        1..=8 => Some((INVENTORY_MENU_MAIN_START, INVENTORY_MENU_HOTBAR_END, false)),
        INVENTORY_MENU_MAIN_START..=35 => inventory_menu_equipment_quick_move_target(
            source_item,
            slots,
            default_item_equipment_slots,
        )
        .or(Some((
            INVENTORY_MENU_HOTBAR_START,
            INVENTORY_MENU_HOTBAR_END,
            false,
        ))),
        INVENTORY_MENU_HOTBAR_START..=44 => inventory_menu_equipment_quick_move_target(
            source_item,
            slots,
            default_item_equipment_slots,
        )
        .or(Some((
            INVENTORY_MENU_MAIN_START,
            INVENTORY_MENU_MAIN_END,
            false,
        ))),
        INVENTORY_MENU_OFFHAND_SLOT => inventory_menu_equipment_quick_move_target(
            source_item,
            slots,
            default_item_equipment_slots,
        )
        .or(Some((
            INVENTORY_MENU_MAIN_START,
            INVENTORY_MENU_HOTBAR_END,
            false,
        ))),
        _ => None,
    }
}

fn inventory_menu_equipment_quick_move_target(
    source_item: &ProtocolItemStackSummary,
    slots: &[ContainerSlot],
    default_item_equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
) -> Option<(i16, i16, bool)> {
    let item_id = source_item.item_id?;
    let target_slot =
        inventory_menu_equipment_slot(default_item_equipment_slots.get(&item_id).copied()?)?;
    if inventory_menu_slot_has_item(slots, target_slot) {
        return None;
    }
    Some((target_slot, target_slot + 1, false))
}

fn inventory_menu_equipment_slot(equipment_slot: ItemEquipmentSlot) -> Option<i16> {
    match equipment_slot {
        ItemEquipmentSlot::Head => Some(5),
        ItemEquipmentSlot::Chest => Some(6),
        ItemEquipmentSlot::Legs => Some(7),
        ItemEquipmentSlot::Feet => Some(8),
        ItemEquipmentSlot::OffHand => Some(INVENTORY_MENU_OFFHAND_SLOT),
        ItemEquipmentSlot::MainHand | ItemEquipmentSlot::Body | ItemEquipmentSlot::Saddle => None,
    }
}

fn inventory_menu_slot_has_item(slots: &[ContainerSlot], slot_num: i16) -> bool {
    slots
        .iter()
        .find(|slot| slot.slot == slot_num)
        .is_some_and(|slot| item_stack_is_non_empty(&slot.item))
}

fn move_item_stack_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    source_index: usize,
    moving: &mut ProtocolItemStackSummary,
    start_slot: i16,
    end_slot: i16,
    backwards: bool,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> bool {
    move_item_stack_to_slots_where(
        container_id,
        slots,
        source_index,
        moving,
        start_slot,
        end_slot,
        backwards,
        |_| true,
        default_item_max_stack_sizes,
    )
}

fn move_item_stack_to_slots_where(
    container_id: i32,
    slots: &mut [ContainerSlot],
    source_index: usize,
    moving: &mut ProtocolItemStackSummary,
    start_slot: i16,
    end_slot: i16,
    backwards: bool,
    may_use_slot: impl FnMut(i16) -> bool,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> bool {
    move_item_stack_to_slots_where_with_limit(
        container_id,
        slots,
        source_index,
        moving,
        start_slot,
        end_slot,
        backwards,
        may_use_slot,
        |_, _, max_stack_size| max_stack_size,
        default_item_max_stack_sizes,
    )
}

fn move_item_stack_to_slots_where_with_limit(
    container_id: i32,
    slots: &mut [ContainerSlot],
    source_index: usize,
    moving: &mut ProtocolItemStackSummary,
    start_slot: i16,
    end_slot: i16,
    backwards: bool,
    mut may_use_slot: impl FnMut(i16) -> bool,
    mut slot_max_stack_size: impl FnMut(i16, &ProtocolItemStackSummary, i32) -> i32,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> bool {
    let mut changed = false;
    if item_stack_max_stack_size(moving, default_item_max_stack_sizes) > 1 {
        for dest_slot in quick_move_slot_ids(start_slot, end_slot, backwards) {
            if !may_use_slot(dest_slot) {
                continue;
            }
            if item_stack_is_empty(moving) {
                break;
            }
            let Some(dest_index) = slots.iter().position(|slot| slot.slot == dest_slot) else {
                continue;
            };
            if dest_index == source_index {
                continue;
            }
            let slot = &mut slots[dest_index];
            if item_stack_is_empty(&slot.item) || !same_item_same_components(moving, &slot.item) {
                continue;
            }
            let base_max_stack_size = container_slot_max_stack_size(
                container_id,
                dest_slot,
                &slot.item,
                default_item_max_stack_sizes,
            );
            let max_stack_size =
                slot_max_stack_size(dest_slot, &slot.item, base_max_stack_size).max(0);
            let moved = moving.count.min((max_stack_size - slot.item.count).max(0));
            if moved <= 0 {
                continue;
            }
            slot.item.count += moved;
            moving.count -= moved;
            normalize_item_stack(moving);
            normalize_container_slot_selection(slot);
            changed = true;
        }
    }

    if !item_stack_is_empty(moving) {
        for dest_slot in quick_move_slot_ids(start_slot, end_slot, backwards) {
            if !may_use_slot(dest_slot) {
                continue;
            }
            let Some(dest_index) = slots.iter().position(|slot| slot.slot == dest_slot) else {
                continue;
            };
            if dest_index == source_index || !item_stack_is_empty(&slots[dest_index].item) {
                continue;
            }
            let base_max_stack_size = container_slot_max_stack_size(
                container_id,
                dest_slot,
                moving,
                default_item_max_stack_sizes,
            );
            let max_stack_size = slot_max_stack_size(dest_slot, moving, base_max_stack_size).max(0);
            let amount = moving.count.min(max_stack_size);
            if amount <= 0 {
                continue;
            }
            let slot = &mut slots[dest_index];
            move_stack_count(moving, &mut slot.item, amount);
            normalize_container_slot_selection(slot);
            changed = true;
            break;
        }
    }

    changed
}

fn quick_move_slot_ids(start_slot: i16, end_slot: i16, backwards: bool) -> Vec<i16> {
    if backwards {
        (start_slot..end_slot).rev().collect()
    } else {
        (start_slot..end_slot).collect()
    }
}

fn normalize_container_slot_selection(slot: &mut ContainerSlot) {
    slot.local_selected_bundle_item_index = normalize_local_selected_bundle_item_index(
        slot.local_selected_bundle_item_index,
        &slot.item,
    );
}

fn apply_outside_pickup_click(cursor: &mut ProtocolItemStackSummary, button_num: i8) {
    if item_stack_is_empty(cursor) {
        return;
    }
    if button_num == 0 {
        *cursor = ProtocolItemStackSummary::empty();
    } else if button_num == 1 {
        cursor.count -= 1;
        normalize_item_stack(cursor);
    }
}

fn apply_slot_pickup_click(
    container_id: i32,
    slot_num: i16,
    slot: &mut ProtocolItemStackSummary,
    cursor: &mut ProtocolItemStackSummary,
    button_num: i8,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !matches!(button_num, 0 | 1) {
        return;
    }

    if item_stack_is_empty(slot) {
        if item_stack_is_empty(cursor) {
            return;
        }
        let amount = if button_num == 0 { cursor.count } else { 1 };
        let amount = amount.min(container_slot_max_stack_size(
            container_id,
            slot_num,
            cursor,
            default_item_max_stack_sizes,
        ));
        move_stack_count(cursor, slot, amount);
        return;
    }

    if item_stack_is_empty(cursor) {
        let amount = if button_num == 0 {
            slot.count
        } else {
            (slot.count + 1) / 2
        };
        move_stack_count(slot, cursor, amount);
        return;
    }

    if same_item_same_components(slot, cursor) {
        let amount = if button_num == 0 { cursor.count } else { 1 };
        let max_stack_size = container_slot_max_stack_size(
            container_id,
            slot_num,
            slot,
            default_item_max_stack_sizes,
        );
        let moved = amount.min((max_stack_size - slot.count).max(0));
        if moved > 0 {
            slot.count += moved;
            cursor.count -= moved;
            normalize_item_stack(cursor);
        }
    } else if cursor.count
        <= container_slot_max_stack_size(
            container_id,
            slot_num,
            cursor,
            default_item_max_stack_sizes,
        )
    {
        std::mem::swap(slot, cursor);
    }
}

fn move_stack_count(
    source: &mut ProtocolItemStackSummary,
    target: &mut ProtocolItemStackSummary,
    amount: i32,
) {
    let moved = amount.min(source.count).max(0);
    if moved <= 0 {
        return;
    }
    *target = source.clone();
    target.count = moved;
    source.count -= moved;
    normalize_item_stack(source);
}

fn normalize_item_stack(stack: &mut ProtocolItemStackSummary) {
    if item_stack_is_empty(stack) {
        *stack = ProtocolItemStackSummary::empty();
    }
}

fn item_stack_is_empty(stack: &ProtocolItemStackSummary) -> bool {
    stack.item_id.is_none() || stack.count <= 0
}

fn same_item_same_components(
    left: &ProtocolItemStackSummary,
    right: &ProtocolItemStackSummary,
) -> bool {
    left.item_id == right.item_id && left.component_patch == right.component_patch
}

fn container_slot_max_stack_size(
    container_id: i32,
    slot_num: i16,
    stack: &ProtocolItemStackSummary,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> i32 {
    let item_max_stack_size = item_stack_max_stack_size(stack, default_item_max_stack_sizes);
    let slot_max_stack_size = if container_id == INVENTORY_MENU_CONTAINER_ID {
        match slot_num {
            0 => 0,
            5..=8 => 1,
            _ => VANILLA_DEFAULT_MAX_STACK_SIZE,
        }
    } else {
        VANILLA_DEFAULT_MAX_STACK_SIZE
    };
    item_max_stack_size.min(slot_max_stack_size)
}

fn item_stack_max_stack_size(
    stack: &ProtocolItemStackSummary,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> i32 {
    if item_stack_is_empty(stack) {
        return 0;
    }
    if let Some(size) = stack.component_patch.max_stack_size {
        return clamp_vanilla_item_max_stack_size(size);
    }
    if stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_MAX_STACK_SIZE_COMPONENT_ID)
    {
        return 1;
    }
    if stack.component_patch.max_damage.is_some() || stack.component_patch.damage.is_some() {
        return 1;
    }
    stack
        .item_id
        .and_then(|item_id| default_item_max_stack_sizes.get(&item_id).copied())
        .map(clamp_vanilla_item_max_stack_size)
        .unwrap_or(VANILLA_DEFAULT_MAX_STACK_SIZE)
}

fn clamp_vanilla_item_max_stack_size(size: i32) -> i32 {
    size.clamp(1, VANILLA_ABSOLUTE_MAX_STACK_SIZE)
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
            local_selected_offer_index: 0,
            local_scroll_offset: 0,
        }
    }
}

fn merchant_max_scroll_offset(offer_count: usize) -> i32 {
    offer_count
        .saturating_sub(MERCHANT_VISIBLE_OFFER_COUNT)
        .try_into()
        .unwrap_or(i32::MAX)
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
    use crate::entities::{
        VANILLA_ENTITY_TYPE_DONKEY_ID, VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_LLAMA_ID,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    };
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, EntityDataValue as ProtocolEntityDataValue,
        EntityDataValueKind, IngredientSummary, MountScreenOpen as ProtocolMountScreenOpen,
        PlayerAbilities as ProtocolPlayerAbilities, RecipePropertySetSummary, RegistryTags,
        SetEntityData as ProtocolSetEntityData, SlotDisplaySummary,
        StonecutterSelectableRecipeSummary, TagNetworkPayload,
        UpdateRecipes as ProtocolUpdateRecipes, UpdateTags as ProtocolUpdateTags,
        UseEffectsSummary as ProtocolUseEffectsSummary, Vec3d as ProtocolVec3d,
    };
    use uuid::Uuid;

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
        assert_eq!(store.open_container_data_value(2), Some(10));
        assert_eq!(store.open_container_data_value(3), None);
        assert_eq!(
            store.inventory().cursor_item,
            ProtocolItemStackSummary::empty()
        );

        assert!(store.apply_container_close(ProtocolContainerClose { container_id: 7 }));
        assert!(store.inventory().open_container.is_none());
        assert_eq!(store.open_container_data_value(2), None);
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
    fn mount_equipment_visibility_follows_vanilla_horse_type_tags() {
        assert_eq!(
            mount_visibility_for_entity(VANILLA_ENTITY_TYPE_HORSE_ID, Vec::new()),
            Some(MountEquipmentSlotVisibility {
                saddle: true,
                body: Some(MountArmorSlotKind::Horse),
            })
        );
        assert_eq!(
            mount_visibility_for_entity(VANILLA_ENTITY_TYPE_LLAMA_ID, Vec::new()),
            Some(MountEquipmentSlotVisibility {
                saddle: false,
                body: Some(MountArmorSlotKind::Llama),
            })
        );
        assert_eq!(
            mount_visibility_for_entity(VANILLA_ENTITY_TYPE_DONKEY_ID, Vec::new()),
            Some(MountEquipmentSlotVisibility {
                saddle: false,
                body: None,
            })
        );
    }

    #[test]
    fn mount_equipment_visibility_uses_tame_and_baby_entity_data() {
        assert_eq!(
            mount_visibility_for_entity(
                VANILLA_ENTITY_TYPE_DONKEY_ID,
                vec![protocol_byte_data(
                    VANILLA_MOUNT_TAME_FLAGS_DATA_ID,
                    VANILLA_ABSTRACT_HORSE_TAME_FLAG,
                )],
            ),
            Some(MountEquipmentSlotVisibility {
                saddle: true,
                body: None,
            })
        );
        assert_eq!(
            mount_visibility_for_entity(
                VANILLA_ENTITY_TYPE_DONKEY_ID,
                vec![
                    protocol_bool_data(VANILLA_AGEABLE_MOB_BABY_DATA_ID, true),
                    protocol_byte_data(
                        VANILLA_MOUNT_TAME_FLAGS_DATA_ID,
                        VANILLA_ABSTRACT_HORSE_TAME_FLAG,
                    ),
                ],
            ),
            Some(MountEquipmentSlotVisibility {
                saddle: false,
                body: None,
            })
        );
        assert_eq!(
            mount_visibility_for_entity(
                VANILLA_ENTITY_TYPE_NAUTILUS_ID,
                vec![protocol_byte_data(
                    VANILLA_MOUNT_TAME_FLAGS_DATA_ID,
                    VANILLA_TAMABLE_ANIMAL_TAME_FLAG,
                )],
            ),
            Some(MountEquipmentSlotVisibility {
                saddle: true,
                body: Some(MountArmorSlotKind::Nautilus),
            })
        );
        assert_eq!(
            mount_visibility_for_entity(VANILLA_ENTITY_TYPE_NAUTILUS_ID, Vec::new()),
            Some(MountEquipmentSlotVisibility {
                saddle: false,
                body: None,
            })
        );
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
    fn local_selected_main_hand_has_piercing_weapon_true_for_default_item() {
        let mut store = WorldStore::new();
        store.set_default_piercing_weapon_item_ids(BTreeSet::from([-1, 42]));
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });

        assert!(store.local_selected_main_hand_has_piercing_weapon());
    }

    #[test]
    fn local_selected_main_hand_has_piercing_weapon_true_for_added_component() {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack_with_component_summary(99, 1, VANILLA_PIERCING_WEAPON_COMPONENT_ID),
        });

        assert!(store.local_selected_main_hand_has_piercing_weapon());
    }

    #[test]
    fn local_selected_main_hand_has_piercing_weapon_removed_component_overrides_default_and_added()
    {
        let mut store = WorldStore::new();
        store.set_default_piercing_weapon_item_ids(BTreeSet::from([42]));
        let mut item =
            item_stack_with_component_summary(42, 1, VANILLA_PIERCING_WEAPON_COMPONENT_ID);
        item.component_patch.removed_type_ids = vec![VANILLA_PIERCING_WEAPON_COMPONENT_ID];
        store.apply_set_player_inventory(ProtocolSetPlayerInventory { slot: 0, item });

        assert!(!store.local_selected_main_hand_has_piercing_weapon());
    }

    #[test]
    fn local_selected_main_hand_has_piercing_weapon_false_for_empty_or_invalid_item() {
        let mut store = WorldStore::new();
        store.set_default_piercing_weapon_item_ids(BTreeSet::from([-1, 42]));

        assert!(!store.local_selected_main_hand_has_piercing_weapon());

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 0),
        });
        assert!(!store.local_selected_main_hand_has_piercing_weapon());

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(-1, 1),
        });
        assert!(!store.local_selected_main_hand_has_piercing_weapon());
    }

    #[test]
    fn local_selected_main_hand_has_piercing_weapon_respects_selected_slot() {
        let mut store = WorldStore::new();
        store.set_default_piercing_weapon_item_ids(BTreeSet::from([42]));
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 1,
            item: item_stack(99, 1),
        });

        assert!(store.local_selected_main_hand_has_piercing_weapon());
        assert!(store.set_local_selected_hotbar_slot(1));
        assert!(!store.local_selected_main_hand_has_piercing_weapon());
    }

    #[test]
    fn local_selected_main_hand_attack_range_reads_default_and_patch_components() {
        let mut store = WorldStore::new();
        let default_range = item_attack_range(2.0, 4.5);
        let patch_range = ProtocolAttackRangeSummary {
            min_reach: 1.0,
            max_reach: 2.25,
            min_creative_reach: 1.0,
            max_creative_reach: 3.0,
            hitbox_margin: 0.25,
            mob_factor: 0.75,
        };
        store.set_default_item_attack_ranges(BTreeMap::from([
            (-1, default_range),
            (42, default_range),
        ]));
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });

        assert_eq!(
            store.local_selected_main_hand_attack_range(),
            Some(default_range)
        );

        let mut patched = item_stack(99, 1);
        patched.component_patch.added = 1;
        patched.component_patch.added_type_ids = vec![VANILLA_ATTACK_RANGE_COMPONENT_ID];
        patched.component_patch.attack_range = Some(patch_range);
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: patched,
        });

        assert_eq!(
            store.local_selected_main_hand_attack_range(),
            Some(ItemAttackRange {
                min_reach: 1.0,
                max_reach: 2.25,
                min_creative_reach: 1.0,
                max_creative_reach: 3.0,
                hitbox_margin: 0.25,
                mob_factor: 0.75,
            })
        );
    }

    #[test]
    fn local_selected_main_hand_attack_range_removed_component_overrides_default_and_added() {
        let mut store = WorldStore::new();
        store.set_default_item_attack_ranges(BTreeMap::from([(42, item_attack_range(2.0, 4.5))]));
        let mut item = item_stack(42, 1);
        item.component_patch.added = 1;
        item.component_patch.added_type_ids = vec![VANILLA_ATTACK_RANGE_COMPONENT_ID];
        item.component_patch.attack_range = Some(ProtocolAttackRangeSummary {
            min_reach: 1.0,
            max_reach: 2.25,
            min_creative_reach: 1.0,
            max_creative_reach: 3.0,
            hitbox_margin: 0.25,
            mob_factor: 0.75,
        });
        item.component_patch.removed_type_ids = vec![VANILLA_ATTACK_RANGE_COMPONENT_ID];
        store.apply_set_player_inventory(ProtocolSetPlayerInventory { slot: 0, item });

        assert_eq!(store.local_selected_main_hand_attack_range(), None);
    }

    #[test]
    fn local_selected_main_hand_attack_range_respects_empty_invalid_and_selected_slot() {
        let mut store = WorldStore::new();
        store.set_default_item_attack_ranges(BTreeMap::from([
            (-1, item_attack_range(2.0, 4.5)),
            (42, item_attack_range(2.0, 4.5)),
        ]));

        assert_eq!(store.local_selected_main_hand_attack_range(), None);

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 0),
        });
        assert_eq!(store.local_selected_main_hand_attack_range(), None);

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(-1, 1),
        });
        assert_eq!(store.local_selected_main_hand_attack_range(), None);

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 1,
            item: item_stack(99, 1),
        });
        assert_eq!(
            store.local_selected_main_hand_attack_range(),
            Some(item_attack_range(2.0, 4.5))
        );
        assert!(store.set_local_selected_hotbar_slot(1));
        assert_eq!(store.local_selected_main_hand_attack_range(), None);
    }

    #[test]
    fn local_using_item_use_effects_reads_default_and_patch_components() {
        let mut store = WorldStore::new();
        let default_effects = ItemUseEffects {
            can_sprint: true,
            interact_vibrations: false,
            speed_multiplier: 1.0,
        };
        store.set_default_item_use_effects(BTreeMap::from([
            (-1, default_effects),
            (42, default_effects),
        ]));
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });
        store.set_local_using_item(true);

        assert_eq!(store.local_using_item_use_effects(), Some(default_effects));

        let mut patched = item_stack(99, 1);
        patched.component_patch.added = 1;
        patched.component_patch.added_type_ids = vec![VANILLA_USE_EFFECTS_COMPONENT_ID];
        patched.component_patch.use_effects = Some(ProtocolUseEffectsSummary {
            can_sprint: false,
            interact_vibrations: true,
            speed_multiplier: 0.5,
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: patched,
        });

        assert_eq!(
            store.local_using_item_use_effects(),
            Some(ItemUseEffects {
                can_sprint: false,
                interact_vibrations: true,
                speed_multiplier: 0.5,
            })
        );
    }

    #[test]
    fn local_using_item_use_effects_removed_component_falls_back_to_vanilla_default() {
        let mut store = WorldStore::new();
        store.set_default_item_use_effects(BTreeMap::from([(
            42,
            ItemUseEffects {
                can_sprint: true,
                interact_vibrations: false,
                speed_multiplier: 1.0,
            },
        )]));
        let mut item = item_stack(42, 1);
        item.component_patch.removed_type_ids = vec![VANILLA_USE_EFFECTS_COMPONENT_ID];
        store.apply_set_player_inventory(ProtocolSetPlayerInventory { slot: 0, item });
        store.set_local_using_item(true);

        assert_eq!(
            store.local_using_item_use_effects(),
            Some(ItemUseEffects::default())
        );
    }

    #[test]
    fn drop_local_selected_hotbar_item_drops_one_and_updates_menu_view() {
        let mut store = WorldStore::new();
        assert!(store.set_local_selected_hotbar_slot(2));
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 2,
            item: item_stack(42, 3),
        });

        assert!(store.drop_local_selected_hotbar_item(false));

        assert_eq!(player_slot_item(&store, 2), item_stack(42, 2));
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START + 2),
            item_stack(42, 2)
        );
    }

    #[test]
    fn drop_local_selected_hotbar_item_drops_stack_and_reports_empty_slots() {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(99, 1),
        });

        assert!(store.drop_local_selected_hotbar_item(true));

        assert_eq!(
            player_slot_item(&store, 0),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
            ProtocolItemStackSummary::empty()
        );
        assert!(!store.drop_local_selected_hotbar_item(true));
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
        assert_eq!(offers.local_selected_offer_index, 0);
        assert_eq!(offers.local_scroll_offset, 0);
        assert_eq!(offers.offers[0].buy_a, item_cost(42, 3));
        assert_eq!(offers.offers[0].sell, item_stack(99, 1));
        assert!(store.set_local_merchant_selected_offer(1));
        assert!(!store.set_local_merchant_selected_offer(2));
        assert!(!store.set_local_merchant_selected_offer(-1));
        assert_eq!(
            store
                .inventory()
                .open_container
                .as_ref()
                .and_then(|container| container.merchant_offers.as_ref())
                .map(|offers| offers.local_selected_offer_index),
            Some(1)
        );

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
    fn merchant_offer_scroll_offset_clamps_to_visible_window() {
        let mut store = WorldStore::new();
        assert!(!store.scroll_local_merchant_offers(1));
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
            title: "Merchant".to_string(),
        });
        assert!(store.apply_merchant_offers(merchant_offers(7, 8)));

        assert!(store.scroll_local_merchant_offers(1));
        assert_eq!(
            store
                .inventory()
                .open_container
                .as_ref()
                .and_then(|container| container.merchant_offers.as_ref())
                .map(|offers| offers.local_scroll_offset),
            Some(1)
        );
        assert!(store.scroll_local_merchant_offers(1));
        assert_eq!(
            store
                .inventory()
                .open_container
                .as_ref()
                .and_then(|container| container.merchant_offers.as_ref())
                .map(|offers| offers.local_scroll_offset),
            Some(1)
        );
        assert!(store.scroll_local_merchant_offers(-1));
        assert_eq!(
            store
                .inventory()
                .open_container
                .as_ref()
                .and_then(|container| container.merchant_offers.as_ref())
                .map(|offers| offers.local_scroll_offset),
            Some(0)
        );
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

    #[test]
    fn apply_local_container_click_slot_picks_up_and_places_stack() {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 3),
        });
        assert!(store.open_local_inventory());

        let pickup = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_HOTBAR_START,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();

        assert_eq!(pickup.container_id, INVENTORY_MENU_CONTAINER_ID);
        assert_eq!(pickup.slot_num, INVENTORY_MENU_HOTBAR_START);
        assert_eq!(
            pickup.changed_slots,
            BTreeMap::from([(INVENTORY_MENU_HOTBAR_START, ProtocolHashedStack::Empty)])
        );
        assert_eq!(
            pickup.carried_item,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 42,
                count: 3,
                components: ProtocolHashedComponentPatch::default(),
            })
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            player_slot_item(&store, 0),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(store.inventory().cursor_item, item_stack(42, 3));

        let place = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 10,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();

        assert_eq!(
            place.changed_slots,
            BTreeMap::from([(
                10,
                ProtocolHashedStack::Item(ProtocolHashedItemStack {
                    item_id: 42,
                    count: 3,
                    components: ProtocolHashedComponentPatch::default(),
                })
            )])
        );
        assert_eq!(place.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(inventory_menu_slot_item(&store, 10), item_stack(42, 3));
        assert_eq!(player_slot_item(&store, 10), item_stack(42, 3));
        assert_eq!(
            store.inventory().cursor_item,
            ProtocolItemStackSummary::empty()
        );
    }

    #[test]
    fn apply_local_container_click_slot_supports_secondary_pickup_place_and_outside_drop() {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 5),
        });
        assert!(store.open_local_inventory());

        let half_pickup = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_HOTBAR_START,
                button_num: 1,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();

        assert_eq!(
            half_pickup.changed_slots,
            BTreeMap::from([(
                INVENTORY_MENU_HOTBAR_START,
                ProtocolHashedStack::Item(ProtocolHashedItemStack {
                    item_id: 42,
                    count: 2,
                    components: ProtocolHashedComponentPatch::default(),
                })
            )])
        );
        assert_eq!(
            half_pickup.carried_item,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 42,
                count: 3,
                components: ProtocolHashedComponentPatch::default(),
            })
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
            item_stack(42, 2)
        );
        assert_eq!(player_slot_item(&store, 0), item_stack(42, 2));
        assert_eq!(store.inventory().cursor_item, item_stack(42, 3));

        let single_place = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 10,
                button_num: 1,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();

        assert_eq!(
            single_place.changed_slots,
            BTreeMap::from([(
                10,
                ProtocolHashedStack::Item(ProtocolHashedItemStack {
                    item_id: 42,
                    count: 1,
                    components: ProtocolHashedComponentPatch::default(),
                })
            )])
        );
        assert_eq!(
            single_place.carried_item,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 42,
                count: 2,
                components: ProtocolHashedComponentPatch::default(),
            })
        );
        assert_eq!(inventory_menu_slot_item(&store, 10), item_stack(42, 1));
        assert_eq!(player_slot_item(&store, 10), item_stack(42, 1));
        assert_eq!(store.inventory().cursor_item, item_stack(42, 2));

        let drop_one = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: -999,
                button_num: 1,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();
        assert!(drop_one.changed_slots.is_empty());
        assert_eq!(
            drop_one.carried_item,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 42,
                count: 1,
                components: ProtocolHashedComponentPatch::default(),
            })
        );
        assert_eq!(store.inventory().cursor_item, item_stack(42, 1));

        let drop_remaining = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: -999,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();
        assert!(drop_remaining.changed_slots.is_empty());
        assert_eq!(drop_remaining.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            store.inventory().cursor_item,
            ProtocolItemStackSummary::empty()
        );
    }

    #[test]
    fn apply_local_container_click_slot_uses_default_item_max_stack_sizes() {
        let mut store = WorldStore::new();
        store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 16)]));
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 2),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 1,
            item: item_stack(42, 15),
        });
        assert!(store.open_local_inventory());

        store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_HOTBAR_START,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();
        let merge = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_HOTBAR_START + 1,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();

        assert_eq!(
            merge.changed_slots,
            BTreeMap::from([(
                INVENTORY_MENU_HOTBAR_START + 1,
                ProtocolHashedStack::Item(ProtocolHashedItemStack {
                    item_id: 42,
                    count: 16,
                    components: ProtocolHashedComponentPatch::default(),
                })
            )])
        );
        assert_eq!(
            merge.carried_item,
            ProtocolHashedStack::Item(ProtocolHashedItemStack {
                item_id: 42,
                count: 1,
                components: ProtocolHashedComponentPatch::default(),
            })
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START + 1),
            item_stack(42, 16)
        );
        assert_eq!(store.inventory().cursor_item, item_stack(42, 1));
    }

    #[test]
    fn apply_local_container_click_slot_respects_unstackable_and_local_slot_limits() {
        let mut store = WorldStore::new();
        store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 1), (43, 64)]));
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 1,
            item: item_stack(42, 1),
        });
        assert!(store.open_local_inventory());

        store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_HOTBAR_START,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();
        let blocked_merge = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_HOTBAR_START + 1,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();
        assert!(blocked_merge.changed_slots.is_empty());
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START + 1),
            item_stack(42, 1)
        );
        assert_eq!(store.inventory().cursor_item, item_stack(42, 1));

        store.apply_set_cursor_item(ProtocolSetCursorItem {
            item: item_stack(43, 3),
        });
        let result_slot_place = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();
        assert!(result_slot_place.changed_slots.is_empty());
        assert_eq!(store.inventory().cursor_item, item_stack(43, 3));

        let armor_place = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 5,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();
        assert_eq!(
            armor_place.changed_slots,
            BTreeMap::from([(
                5,
                ProtocolHashedStack::Item(ProtocolHashedItemStack {
                    item_id: 43,
                    count: 1,
                    components: ProtocolHashedComponentPatch::default(),
                })
            )])
        );
        assert_eq!(inventory_menu_slot_item(&store, 5), item_stack(43, 1));
        assert_eq!(store.inventory().cursor_item, item_stack(43, 2));
    }

    #[test]
    fn apply_local_container_pickup_all_collects_matching_stacks() {
        let mut store = WorldStore::new();
        store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 16), (43, 16)]));
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 9,
            item: item_stack(42, 3),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 10,
            item: item_stack(42, 4),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 11,
            item: item_stack(42, 16),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 12,
            item: item_stack(43, 5),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 14,
            item: item_stack_with_component_summary(42, 6, 7),
        });
        store.apply_set_cursor_item(ProtocolSetCursorItem {
            item: item_stack(42, 8),
        });
        assert!(store.open_local_inventory());

        let pickup_all = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 13,
                button_num: 0,
                input: ProtocolContainerInput::PickupAll,
            })
            .unwrap();

        assert_eq!(pickup_all.container_id, INVENTORY_MENU_CONTAINER_ID);
        assert_eq!(pickup_all.input, ProtocolContainerInput::PickupAll);
        assert_eq!(
            pickup_all.changed_slots,
            BTreeMap::from([
                (INVENTORY_MENU_MAIN_START, ProtocolHashedStack::Empty),
                (INVENTORY_MENU_MAIN_START + 1, ProtocolHashedStack::Empty),
                (INVENTORY_MENU_MAIN_START + 2, hashed_item_stack(42, 15)),
            ])
        );
        assert_eq!(pickup_all.carried_item, hashed_item_stack(42, 16));
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 1),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 2),
            item_stack(42, 15)
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 3),
            item_stack(43, 5)
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 5),
            item_stack_with_component_summary(42, 6, 7)
        );
        assert_eq!(store.inventory().cursor_item, item_stack(42, 16));
    }

    #[test]
    fn apply_local_container_pickup_all_button_one_collects_in_reverse_order() {
        let mut store = WorldStore::new();
        store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 16)]));
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 9,
            item: item_stack(42, 4),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 8,
            item: item_stack(42, 4),
        });
        store.apply_set_cursor_item(ProtocolSetCursorItem {
            item: item_stack(42, 14),
        });
        assert!(store.open_local_inventory());

        let pickup_all = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_MAIN_START + 1,
                button_num: 1,
                input: ProtocolContainerInput::PickupAll,
            })
            .unwrap();

        assert_eq!(
            pickup_all.changed_slots,
            BTreeMap::from([(INVENTORY_MENU_HOTBAR_END - 1, hashed_item_stack(42, 2))])
        );
        assert_eq!(pickup_all.carried_item, hashed_item_stack(42, 16));
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
            item_stack(42, 4)
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_END - 1),
            item_stack(42, 2)
        );
        assert_eq!(store.inventory().cursor_item, item_stack(42, 16));
    }

    #[test]
    fn apply_local_container_pickup_all_noops_without_eligible_click() {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 9,
            item: item_stack(42, 3),
        });
        assert!(store.open_local_inventory());

        let empty_cursor = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_MAIN_START + 1,
                button_num: 0,
                input: ProtocolContainerInput::PickupAll,
            })
            .unwrap();
        assert!(empty_cursor.changed_slots.is_empty());
        assert_eq!(empty_cursor.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
            item_stack(42, 3)
        );

        store.apply_set_cursor_item(ProtocolSetCursorItem {
            item: item_stack(42, 5),
        });
        let clicked_item = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_MAIN_START,
                button_num: 0,
                input: ProtocolContainerInput::PickupAll,
            })
            .unwrap();
        assert!(clicked_item.changed_slots.is_empty());
        assert_eq!(clicked_item.carried_item, hashed_item_stack(42, 5));
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
            item_stack(42, 3)
        );

        for slot_num in [-999, -1] {
            let outside = store
                .apply_local_container_click_slot(ContainerClickSlotRequest {
                    slot_num,
                    button_num: 0,
                    input: ProtocolContainerInput::PickupAll,
                })
                .unwrap();
            assert!(outside.changed_slots.is_empty());
            assert_eq!(outside.carried_item, hashed_item_stack(42, 5));
            assert_eq!(
                inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
                item_stack(42, 3)
            );
        }
    }

    #[test]
    fn apply_local_container_pickup_all_rejects_non_inventory_menu() {
        let mut store = WorldStore::new();
        store.apply_set_cursor_item(ProtocolSetCursorItem {
            item: item_stack(42, 5),
        });
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: 1,
            title: "Chest".to_string(),
        });
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 3,
            items: vec![ProtocolItemStackSummary::empty(), item_stack(42, 3)],
            carried_item: item_stack(42, 5),
        });

        assert_eq!(
            store
                .apply_local_container_click_slot(ContainerClickSlotRequest {
                    slot_num: 0,
                    button_num: 0,
                    input: ProtocolContainerInput::PickupAll,
                })
                .unwrap_err(),
            ContainerClickBuildError::UnsupportedLocalClickInput(ProtocolContainerInput::PickupAll)
        );
        assert_eq!(
            store.inventory().open_container.as_ref().unwrap().slots[1].item,
            item_stack(42, 3)
        );
        assert_eq!(store.inventory().cursor_item, item_stack(42, 5));
    }

    #[test]
    fn apply_local_container_quick_craft_left_drag_distributes_evenly() {
        let mut store = WorldStore::new();
        store.apply_set_cursor_item(ProtocolSetCursorItem {
            item: item_stack(42, 8),
        });
        assert!(store.open_local_inventory());

        let start = store
            .apply_local_container_click_slot(quick_craft_request(
                -999,
                QUICKCRAFT_HEADER_START,
                QUICKCRAFT_TYPE_CHARITABLE,
            ))
            .unwrap();
        assert!(start.changed_slots.is_empty());
        assert_eq!(start.carried_item, hashed_item_stack(42, 8));

        let add_first = store
            .apply_local_container_click_slot(quick_craft_request(
                INVENTORY_MENU_MAIN_START,
                QUICKCRAFT_HEADER_CONTINUE,
                QUICKCRAFT_TYPE_CHARITABLE,
            ))
            .unwrap();
        assert!(add_first.changed_slots.is_empty());
        assert_eq!(add_first.carried_item, hashed_item_stack(42, 8));

        let add_second = store
            .apply_local_container_click_slot(quick_craft_request(
                INVENTORY_MENU_MAIN_START + 1,
                QUICKCRAFT_HEADER_CONTINUE,
                QUICKCRAFT_TYPE_CHARITABLE,
            ))
            .unwrap();
        assert!(add_second.changed_slots.is_empty());
        assert_eq!(add_second.carried_item, hashed_item_stack(42, 8));

        let finish = store
            .apply_local_container_click_slot(quick_craft_request(
                -999,
                QUICKCRAFT_HEADER_END,
                QUICKCRAFT_TYPE_CHARITABLE,
            ))
            .unwrap();

        assert_eq!(finish.input, ProtocolContainerInput::QuickCraft);
        assert_eq!(
            finish.changed_slots,
            BTreeMap::from([
                (INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 4)),
                (INVENTORY_MENU_MAIN_START + 1, hashed_item_stack(42, 4)),
            ])
        );
        assert_eq!(finish.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
            item_stack(42, 4)
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 1),
            item_stack(42, 4)
        );
        assert_eq!(player_slot_item(&store, 9), item_stack(42, 4));
        assert_eq!(player_slot_item(&store, 10), item_stack(42, 4));
        assert_eq!(
            store.inventory().cursor_item,
            ProtocolItemStackSummary::empty()
        );
    }

    #[test]
    fn apply_local_container_quick_craft_right_drag_places_one_per_slot() {
        let mut store = WorldStore::new();
        store.apply_set_cursor_item(ProtocolSetCursorItem {
            item: item_stack(42, 5),
        });
        assert!(store.open_local_inventory());

        store
            .apply_local_container_click_slot(quick_craft_request(
                -999,
                QUICKCRAFT_HEADER_START,
                QUICKCRAFT_TYPE_GREEDY,
            ))
            .unwrap();
        store
            .apply_local_container_click_slot(quick_craft_request(
                INVENTORY_MENU_MAIN_START,
                QUICKCRAFT_HEADER_CONTINUE,
                QUICKCRAFT_TYPE_GREEDY,
            ))
            .unwrap();
        store
            .apply_local_container_click_slot(quick_craft_request(
                INVENTORY_MENU_MAIN_START + 1,
                QUICKCRAFT_HEADER_CONTINUE,
                QUICKCRAFT_TYPE_GREEDY,
            ))
            .unwrap();
        let finish = store
            .apply_local_container_click_slot(quick_craft_request(
                -999,
                QUICKCRAFT_HEADER_END,
                QUICKCRAFT_TYPE_GREEDY,
            ))
            .unwrap();

        assert_eq!(
            finish.changed_slots,
            BTreeMap::from([
                (INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 1)),
                (INVENTORY_MENU_MAIN_START + 1, hashed_item_stack(42, 1)),
            ])
        );
        assert_eq!(finish.carried_item, hashed_item_stack(42, 3));
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
            item_stack(42, 1)
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START + 1),
            item_stack(42, 1)
        );
        assert_eq!(store.inventory().cursor_item, item_stack(42, 3));
    }

    #[test]
    fn apply_local_container_quick_craft_single_slot_finish_uses_pickup_semantics() {
        let mut store = WorldStore::new();
        store.apply_set_cursor_item(ProtocolSetCursorItem {
            item: item_stack(42, 5),
        });
        assert!(store.open_local_inventory());

        store
            .apply_local_container_click_slot(quick_craft_request(
                -999,
                QUICKCRAFT_HEADER_START,
                QUICKCRAFT_TYPE_GREEDY,
            ))
            .unwrap();
        store
            .apply_local_container_click_slot(quick_craft_request(
                INVENTORY_MENU_MAIN_START,
                QUICKCRAFT_HEADER_CONTINUE,
                QUICKCRAFT_TYPE_GREEDY,
            ))
            .unwrap();
        let finish = store
            .apply_local_container_click_slot(quick_craft_request(
                -999,
                QUICKCRAFT_HEADER_END,
                QUICKCRAFT_TYPE_GREEDY,
            ))
            .unwrap();

        assert_eq!(
            finish.changed_slots,
            BTreeMap::from([(INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 1))])
        );
        assert_eq!(finish.carried_item, hashed_item_stack(42, 4));
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
            item_stack(42, 1)
        );
        assert_eq!(store.inventory().cursor_item, item_stack(42, 4));
    }

    #[test]
    fn apply_local_container_quick_craft_invalid_type_or_order_resets_without_corruption() {
        let mut store = WorldStore::new();
        store.apply_set_cursor_item(ProtocolSetCursorItem {
            item: item_stack(42, 8),
        });
        assert!(store.open_local_inventory());

        let clone_start = store
            .apply_local_container_click_slot(quick_craft_request(
                -999,
                QUICKCRAFT_HEADER_START,
                QUICKCRAFT_TYPE_CLONE,
            ))
            .unwrap();
        assert!(clone_start.changed_slots.is_empty());
        assert_eq!(clone_start.carried_item, hashed_item_stack(42, 8));

        let continue_without_start = store
            .apply_local_container_click_slot(quick_craft_request(
                INVENTORY_MENU_MAIN_START,
                QUICKCRAFT_HEADER_CONTINUE,
                QUICKCRAFT_TYPE_CHARITABLE,
            ))
            .unwrap();
        assert!(continue_without_start.changed_slots.is_empty());
        assert_eq!(
            continue_without_start.carried_item,
            hashed_item_stack(42, 8)
        );

        store
            .apply_local_container_click_slot(quick_craft_request(
                -999,
                QUICKCRAFT_HEADER_START,
                QUICKCRAFT_TYPE_CHARITABLE,
            ))
            .unwrap();
        let pickup_while_active = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_MAIN_START,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();
        assert!(pickup_while_active.changed_slots.is_empty());
        assert_eq!(pickup_while_active.carried_item, hashed_item_stack(42, 8));

        let stale_continue = store
            .apply_local_container_click_slot(quick_craft_request(
                INVENTORY_MENU_MAIN_START,
                QUICKCRAFT_HEADER_CONTINUE,
                QUICKCRAFT_TYPE_CHARITABLE,
            ))
            .unwrap();
        assert!(stale_continue.changed_slots.is_empty());
        assert_eq!(stale_continue.carried_item, hashed_item_stack(42, 8));
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(store.inventory().cursor_item, item_stack(42, 8));
    }

    #[test]
    fn apply_local_container_quick_craft_rejects_non_inventory_menu() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: 1,
            title: "Chest".to_string(),
        });
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 3,
            items: vec![ProtocolItemStackSummary::empty()],
            carried_item: item_stack(42, 8),
        });

        assert_eq!(
            store
                .apply_local_container_click_slot(quick_craft_request(
                    0,
                    QUICKCRAFT_HEADER_START,
                    QUICKCRAFT_TYPE_CHARITABLE,
                ))
                .unwrap_err(),
            ContainerClickBuildError::UnsupportedLocalClickInput(
                ProtocolContainerInput::QuickCraft
            )
        );
        assert_eq!(
            store.inventory().open_container.as_ref().unwrap().slots[0].item,
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(store.inventory().cursor_item, item_stack(42, 8));
    }

    #[test]
    fn apply_local_container_quick_move_moves_hotbar_to_inventory() {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 3),
        });
        assert!(store.open_local_inventory());

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_HOTBAR_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(quick_move.container_id, INVENTORY_MENU_CONTAINER_ID);
        assert_eq!(quick_move.input, ProtocolContainerInput::QuickMove);
        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 3)),
                (INVENTORY_MENU_HOTBAR_START, ProtocolHashedStack::Empty),
            ])
        );
        assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(inventory_menu_slot_item(&store, 9), item_stack(42, 3));
        assert_eq!(
            player_slot_item(&store, 0),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(player_slot_item(&store, 9), item_stack(42, 3));
    }

    #[test]
    fn apply_local_container_quick_move_moves_inventory_to_hotbar() {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 9,
            item: item_stack(43, 4),
        });
        assert!(store.open_local_inventory());

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_MAIN_START,
                button_num: 1,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (INVENTORY_MENU_MAIN_START, ProtocolHashedStack::Empty),
                (INVENTORY_MENU_HOTBAR_START, hashed_item_stack(43, 4)),
            ])
        );
        assert_eq!(
            inventory_menu_slot_item(&store, 9),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
            item_stack(43, 4)
        );
        assert_eq!(
            player_slot_item(&store, 9),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(player_slot_item(&store, 0), item_stack(43, 4));
    }

    #[test]
    fn apply_local_container_quick_move_merges_then_fills_with_stack_limits() {
        let mut store = WorldStore::new();
        store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 16)]));
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 2),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 9,
            item: item_stack(42, 15),
        });
        assert!(store.open_local_inventory());

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_HOTBAR_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 16)),
                (INVENTORY_MENU_MAIN_START + 1, hashed_item_stack(42, 1)),
                (INVENTORY_MENU_HOTBAR_START, ProtocolHashedStack::Empty),
            ])
        );
        assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(inventory_menu_slot_item(&store, 9), item_stack(42, 16));
        assert_eq!(inventory_menu_slot_item(&store, 10), item_stack(42, 1));
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(player_slot_item(&store, 9), item_stack(42, 16));
        assert_eq!(player_slot_item(&store, 10), item_stack(42, 1));
        assert_eq!(
            player_slot_item(&store, 0),
            ProtocolItemStackSummary::empty()
        );
    }

    #[test]
    fn apply_local_container_quick_move_auto_equips_armor_slot() {
        let mut store = WorldStore::new();
        store.set_default_item_equipment_slots(BTreeMap::from([(42, ItemEquipmentSlot::Chest)]));
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });
        assert!(store.open_local_inventory());

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_HOTBAR_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (6, hashed_item_stack(42, 1)),
                (INVENTORY_MENU_HOTBAR_START, ProtocolHashedStack::Empty),
            ])
        );
        assert_eq!(inventory_menu_slot_item(&store, 6), item_stack(42, 1));
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            player_slot_item(&store, PLAYER_CHEST_EQUIPMENT_SLOT),
            item_stack(42, 1)
        );
        assert_eq!(
            player_slot_item(&store, 0),
            ProtocolItemStackSummary::empty()
        );
    }

    #[test]
    fn apply_local_container_quick_move_auto_equips_offhand_slot() {
        let mut store = WorldStore::new();
        store.set_default_item_equipment_slots(BTreeMap::from([(43, ItemEquipmentSlot::OffHand)]));
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 9,
            item: item_stack(43, 1),
        });
        assert!(store.open_local_inventory());

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_MAIN_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (INVENTORY_MENU_MAIN_START, ProtocolHashedStack::Empty),
                (INVENTORY_MENU_OFFHAND_SLOT, hashed_item_stack(43, 1)),
            ])
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_OFFHAND_SLOT),
            item_stack(43, 1)
        );
        assert_eq!(
            player_slot_item(&store, 9),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            player_slot_item(&store, PLAYER_OFFHAND_SLOT),
            item_stack(43, 1)
        );
    }

    #[test]
    fn apply_local_container_quick_move_uses_inventory_fallback_when_equipment_slot_is_occupied() {
        let mut store = WorldStore::new();
        store.set_default_item_equipment_slots(BTreeMap::from([(42, ItemEquipmentSlot::Chest)]));
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: PLAYER_CHEST_EQUIPMENT_SLOT,
            item: item_stack(99, 1),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 9,
            item: item_stack(42, 1),
        });
        assert!(store.open_local_inventory());

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_MAIN_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (INVENTORY_MENU_MAIN_START, ProtocolHashedStack::Empty),
                (INVENTORY_MENU_HOTBAR_START, hashed_item_stack(42, 1)),
            ])
        );
        assert_eq!(inventory_menu_slot_item(&store, 6), item_stack(99, 1));
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
            item_stack(42, 1)
        );
        assert_eq!(
            player_slot_item(&store, PLAYER_CHEST_EQUIPMENT_SLOT),
            item_stack(99, 1)
        );
        assert_eq!(
            player_slot_item(&store, 9),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(player_slot_item(&store, 0), item_stack(42, 1));
    }

    #[test]
    fn apply_local_container_pickup_result_consumes_crafting_inputs_once() {
        let mut store = WorldStore::new();
        let mut items = vec![ProtocolItemStackSummary::empty(); 46];
        items[0] = item_stack(90, 1);
        items[1] = item_stack(42, 2);
        items[2] = item_stack(43, 1);
        items[4] = item_stack(44, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: INVENTORY_MENU_CONTAINER_ID,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });
        assert!(store.open_local_inventory());

        let pickup = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();

        assert_eq!(
            pickup.changed_slots,
            BTreeMap::from([
                (0, ProtocolHashedStack::Empty),
                (1, hashed_item_stack(42, 1)),
                (2, ProtocolHashedStack::Empty),
                (4, hashed_item_stack(44, 2)),
            ])
        );
        assert_eq!(pickup.carried_item, hashed_item_stack(90, 1));
        assert_eq!(
            inventory_menu_slot_item(&store, 0),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(inventory_menu_slot_item(&store, 1), item_stack(42, 1));
        assert_eq!(
            inventory_menu_slot_item(&store, 2),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(inventory_menu_slot_item(&store, 4), item_stack(44, 2));
        assert_eq!(store.inventory().cursor_item, item_stack(90, 1));
    }

    #[test]
    fn apply_local_container_quick_move_result_repeats_while_inputs_remain() {
        let mut store = WorldStore::new();
        let mut items = vec![ProtocolItemStackSummary::empty(); 46];
        items[0] = item_stack(90, 2);
        items[1] = item_stack(42, 3);
        items[2] = item_stack(43, 3);
        items[4] = item_stack(44, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: INVENTORY_MENU_CONTAINER_ID,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });
        assert!(store.open_local_inventory());

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (0, ProtocolHashedStack::Empty),
                (1, ProtocolHashedStack::Empty),
                (2, ProtocolHashedStack::Empty),
                (4, ProtocolHashedStack::Empty),
                (44, hashed_item_stack(90, 6)),
            ])
        );
        assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            inventory_menu_slot_item(&store, 0),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            inventory_menu_slot_item(&store, 1),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            inventory_menu_slot_item(&store, 2),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            inventory_menu_slot_item(&store, 4),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(inventory_menu_slot_item(&store, 44), item_stack(90, 6));
        assert_eq!(player_slot_item(&store, 8), item_stack(90, 6));
    }

    #[test]
    fn apply_local_container_quick_move_result_consumes_after_partial_transfer() {
        let mut store = WorldStore::new();
        store.set_default_item_max_stack_sizes(BTreeMap::from([(90, 1)]));
        let mut items = vec![ProtocolItemStackSummary::empty(); 46];
        items[0] = item_stack(90, 2);
        items[1] = item_stack(42, 2);
        items[2] = item_stack(43, 2);
        items[4] = item_stack(44, 2);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: INVENTORY_MENU_CONTAINER_ID,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });
        assert!(store.open_local_inventory());

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (0, ProtocolHashedStack::Empty),
                (1, ProtocolHashedStack::Empty),
                (2, ProtocolHashedStack::Empty),
                (4, ProtocolHashedStack::Empty),
                (43, hashed_item_stack(90, 1)),
                (44, hashed_item_stack(90, 1)),
            ])
        );
        assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            inventory_menu_slot_item(&store, 0),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            inventory_menu_slot_item(&store, 1),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            inventory_menu_slot_item(&store, 2),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            inventory_menu_slot_item(&store, 4),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(inventory_menu_slot_item(&store, 43), item_stack(90, 1));
        assert_eq!(inventory_menu_slot_item(&store, 44), item_stack(90, 1));
        assert_eq!(player_slot_item(&store, 7), item_stack(90, 1));
        assert_eq!(player_slot_item(&store, 8), item_stack(90, 1));
    }

    #[test]
    fn apply_local_generic_container_quick_move_moves_chest_to_player_reverse() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: 5,
            title: "Large Chest".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 90];
        items[0] = item_stack(42, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(quick_move.container_id, 7);
        assert_eq!(quick_move.state_id, 12);
        assert_eq!(quick_move.input, ProtocolContainerInput::QuickMove);
        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (0, ProtocolHashedStack::Empty),
                (89, hashed_item_stack(42, 3))
            ])
        );
        assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
        let slots = &store.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
        assert_eq!(slots[89].item, item_stack(42, 3));
    }

    #[test]
    fn apply_local_generic_container_quick_move_merges_player_to_chest_forward() {
        let mut store = WorldStore::new();
        store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 16)]));
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: 1,
            title: "Chest".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 54];
        items[0] = item_stack(42, 15);
        items[18] = item_stack(42, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 18,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(quick_move.container_id, 7);
        assert_eq!(quick_move.state_id, 13);
        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (0, hashed_item_stack(42, 16)),
                (1, hashed_item_stack(42, 2)),
                (18, ProtocolHashedStack::Empty),
            ])
        );
        assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
        let slots = &store.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(42, 16));
        assert_eq!(slots[1].item, item_stack(42, 2));
        assert_eq!(slots[18].item, ProtocolItemStackSummary::empty());
    }

    #[test]
    fn apply_local_generic_3x3_quick_move_moves_dispenser_to_player_reverse() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_GENERIC_3X3_ID,
            title: "Dispenser".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 45];
        items[0] = item_stack(42, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(quick_move.container_id, 7);
        assert_eq!(quick_move.state_id, 12);
        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (0, ProtocolHashedStack::Empty),
                (44, hashed_item_stack(42, 3))
            ])
        );
        let slots = &store.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
        assert_eq!(slots[44].item, item_stack(42, 3));
    }

    #[test]
    fn apply_local_generic_3x3_quick_move_moves_player_to_dispenser_forward() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_GENERIC_3X3_ID,
            title: "Dispenser".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 45];
        items[9] = item_stack(42, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 9,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(quick_move.container_id, 7);
        assert_eq!(quick_move.state_id, 13);
        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (0, hashed_item_stack(42, 3)),
                (9, ProtocolHashedStack::Empty)
            ])
        );
        let slots = &store.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(42, 3));
        assert_eq!(slots[9].item, ProtocolItemStackSummary::empty());
    }

    #[test]
    fn apply_local_crafting_menu_quick_move_moves_grid_to_player_forward() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
            title: "Crafting".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 46];
        items[1] = item_stack(42, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 1,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(quick_move.container_id, 7);
        assert_eq!(quick_move.state_id, 12);
        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (1, ProtocolHashedStack::Empty),
                (10, hashed_item_stack(42, 3))
            ])
        );
        let slots = &store.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[1].item, ProtocolItemStackSummary::empty());
        assert_eq!(slots[10].item, item_stack(42, 3));
    }

    #[test]
    fn apply_local_crafting_menu_quick_move_moves_player_to_grid_then_between_player_ranges() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
            title: "Crafting".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 46];
        items[10] = item_stack(42, 3);
        items[37] = item_stack(43, 4);
        for slot in 1..10 {
            items[slot] = item_stack(90 + slot as i32, 1);
        }
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let main_to_hotbar = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 10,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            main_to_hotbar.changed_slots,
            BTreeMap::from([
                (10, ProtocolHashedStack::Empty),
                (38, hashed_item_stack(42, 3))
            ])
        );

        let hotbar_to_main = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 37,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            hotbar_to_main.changed_slots,
            BTreeMap::from([
                (10, hashed_item_stack(43, 4)),
                (37, ProtocolHashedStack::Empty)
            ])
        );
        let slots = &store.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[10].item, item_stack(43, 4));
        assert_eq!(slots[37].item, ProtocolItemStackSummary::empty());
        assert_eq!(slots[38].item, item_stack(42, 3));
    }

    #[test]
    fn apply_local_crafting_menu_result_quick_move_requires_server_authority() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
            title: "Crafting".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 46];
        items[0] = item_stack(90, 1);
        items[1] = item_stack(42, 1);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 14,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });
        let request = ContainerClickSlotRequest {
            slot_num: 0,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        };

        assert_eq!(
            store.apply_local_container_click_slot(request),
            Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                ProtocolContainerInput::QuickMove
            ))
        );
        let click = store.build_container_click_slot(request).unwrap();
        assert_eq!(click.changed_slots, BTreeMap::new());
        assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
        let slots = &store.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(90, 1));
        assert_eq!(slots[1].item, item_stack(42, 1));
    }

    #[test]
    fn apply_local_crafter_quick_move_moves_grid_to_player_backwards() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_CRAFTER_ID,
            title: "Crafter".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 46];
        items[2] = item_stack(42, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 2,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (2, ProtocolHashedStack::Empty),
                (44, hashed_item_stack(42, 3))
            ])
        );
        assert_eq!(
            open_container_slot_item(&store, 2),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(open_container_slot_item(&store, 44), item_stack(42, 3));
    }

    #[test]
    fn apply_local_crafter_quick_move_skips_disabled_grid_slots() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_CRAFTER_ID,
            title: "Crafter".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 46];
        items[9] = item_stack(42, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });
        store.apply_container_set_data(ProtocolContainerSetData {
            container_id: 7,
            id: 0,
            value: 1,
        });

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 9,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (1, hashed_item_stack(42, 3)),
                (9, ProtocolHashedStack::Empty)
            ])
        );
        assert_eq!(
            open_container_slot_item(&store, 0),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(open_container_slot_item(&store, 1), item_stack(42, 3));
        assert_eq!(
            open_container_slot_item(&store, 9),
            ProtocolItemStackSummary::empty()
        );
    }

    #[test]
    fn apply_local_crafter_result_slot_requires_server_authority() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_CRAFTER_ID,
            title: "Crafter".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 46];
        items[CRAFTER_RESULT_SLOT as usize] = item_stack(90, 1);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 14,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        for input in [
            ProtocolContainerInput::Pickup,
            ProtocolContainerInput::QuickMove,
        ] {
            assert_eq!(
                store.apply_local_container_click_slot(ContainerClickSlotRequest {
                    slot_num: CRAFTER_RESULT_SLOT,
                    button_num: 0,
                    input,
                }),
                Err(ContainerClickBuildError::UnsupportedLocalClickInput(input))
            );
        }
        let click = store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: CRAFTER_RESULT_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::Pickup,
            })
            .unwrap();
        assert_eq!(click.changed_slots, BTreeMap::new());
        assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            open_container_slot_item(&store, CRAFTER_RESULT_SLOT),
            item_stack(90, 1)
        );
    }

    #[test]
    fn apply_local_anvil_input_quick_move_moves_input_slots_to_player_forward() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
            title: "Anvil".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 39];
        items[ANVIL_INPUT_SLOT as usize] = item_stack(42, 1);
        items[ANVIL_ADDITIONAL_SLOT as usize] = item_stack(43, 2);
        items[ANVIL_RESULT_SLOT as usize] = item_stack(90, 1);
        items[30] = item_stack(44, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let input_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: ANVIL_INPUT_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            input_move.changed_slots,
            BTreeMap::from([
                (ANVIL_INPUT_SLOT, ProtocolHashedStack::Empty),
                (ANVIL_PLAYER_MAIN_START, hashed_item_stack(42, 1)),
            ])
        );

        let additional_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: ANVIL_ADDITIONAL_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            additional_move.changed_slots,
            BTreeMap::from([
                (ANVIL_ADDITIONAL_SLOT, ProtocolHashedStack::Empty),
                (ANVIL_PLAYER_MAIN_START + 1, hashed_item_stack(43, 2)),
            ])
        );

        for input in [
            ProtocolContainerInput::Pickup,
            ProtocolContainerInput::QuickMove,
        ] {
            assert_eq!(
                store.apply_local_container_click_slot(ContainerClickSlotRequest {
                    slot_num: ANVIL_RESULT_SLOT,
                    button_num: 0,
                    input,
                }),
                Err(ContainerClickBuildError::UnsupportedLocalClickInput(input))
            );
        }
        assert_eq!(
            store.apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 30,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            }),
            Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                ProtocolContainerInput::QuickMove
            ))
        );
        let click = store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: 30,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(click.changed_slots, BTreeMap::new());
        assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            open_container_slot_item(&store, ANVIL_INPUT_SLOT),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, ANVIL_ADDITIONAL_SLOT),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, ANVIL_RESULT_SLOT),
            item_stack(90, 1)
        );
        assert_eq!(open_container_slot_item(&store, 30), item_stack(44, 3));
        assert_eq!(
            open_container_slot_item(&store, ANVIL_PLAYER_MAIN_START),
            item_stack(42, 1)
        );
        assert_eq!(
            open_container_slot_item(&store, ANVIL_PLAYER_MAIN_START + 1),
            item_stack(43, 2)
        );
    }

    #[test]
    fn apply_local_beacon_quick_move_moves_payment_slot_to_player_reverse() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
            title: "Beacon".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize];
        items[BEACON_PAYMENT_SLOT as usize] = item_stack(42, 1);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: BEACON_PAYMENT_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (BEACON_PAYMENT_SLOT, ProtocolHashedStack::Empty),
                (BEACON_HOTBAR_END - 1, hashed_item_stack(42, 1)),
            ])
        );
        assert_eq!(
            open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, BEACON_HOTBAR_END - 1),
            item_stack(42, 1)
        );
    }

    #[test]
    fn apply_local_beacon_quick_move_routes_single_payment_item_to_payment_slot() {
        let mut store = WorldStore::new();
        apply_item_tags(&mut store, vec![(BEACON_PAYMENT_ITEM_TAG, vec![42])]);
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_BEACON_ID,
            title: "Beacon".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); BEACON_TOTAL_SLOT_COUNT as usize];
        items[BEACON_PLAYER_MAIN_START as usize] = item_stack(42, 1);
        items[(BEACON_PLAYER_MAIN_START + 1) as usize] = item_stack(42, 2);
        items[BEACON_HOTBAR_START as usize] = item_stack(43, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 14,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let tagged_stack_with_multiple_items = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: BEACON_PLAYER_MAIN_START + 1,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            tagged_stack_with_multiple_items.changed_slots,
            BTreeMap::from([
                (BEACON_PLAYER_MAIN_START + 1, ProtocolHashedStack::Empty),
                (BEACON_HOTBAR_START + 1, hashed_item_stack(42, 2)),
            ])
        );

        let payment_to_slot = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: BEACON_PLAYER_MAIN_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            payment_to_slot.changed_slots,
            BTreeMap::from([
                (BEACON_PAYMENT_SLOT, hashed_item_stack(42, 1)),
                (BEACON_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
            ])
        );

        let hotbar_to_main = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: BEACON_HOTBAR_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            hotbar_to_main.changed_slots,
            BTreeMap::from([
                (BEACON_PLAYER_MAIN_START, hashed_item_stack(43, 3)),
                (BEACON_HOTBAR_START, ProtocolHashedStack::Empty),
            ])
        );
        assert_eq!(
            open_container_slot_item(&store, BEACON_PAYMENT_SLOT),
            item_stack(42, 1)
        );
        assert_eq!(
            open_container_slot_item(&store, BEACON_PLAYER_MAIN_START),
            item_stack(43, 3)
        );
        assert_eq!(
            open_container_slot_item(&store, BEACON_HOTBAR_START),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, BEACON_HOTBAR_START + 1),
            item_stack(42, 2)
        );
    }

    #[test]
    fn apply_local_loom_result_pickup_and_quick_move_require_server_authority() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
            title: "Loom".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
        items[0] = item_stack(42, 1);
        items[1] = item_stack(43, 1);
        items[2] = item_stack(44, 1);
        items[LOOM_RESULT_SLOT as usize] = item_stack(90, 1);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        for input in [
            ProtocolContainerInput::Pickup,
            ProtocolContainerInput::QuickMove,
        ] {
            assert_eq!(
                store.apply_local_container_click_slot(ContainerClickSlotRequest {
                    slot_num: LOOM_RESULT_SLOT,
                    button_num: 0,
                    input,
                }),
                Err(ContainerClickBuildError::UnsupportedLocalClickInput(input))
            );
        }
        let click = store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: LOOM_RESULT_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(click.changed_slots, BTreeMap::new());
        assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 1));
        assert_eq!(open_container_slot_item(&store, 1), item_stack(43, 1));
        assert_eq!(open_container_slot_item(&store, 2), item_stack(44, 1));
        assert_eq!(
            open_container_slot_item(&store, LOOM_RESULT_SLOT),
            item_stack(90, 1)
        );
    }

    #[test]
    fn apply_local_loom_quick_move_moves_input_slots_to_player_forward() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
            title: "Loom".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
        items[LOOM_BANNER_SLOT as usize] = item_stack(42, 3);
        items[LOOM_DYE_SLOT as usize] = item_stack(43, 2);
        items[LOOM_PATTERN_SLOT as usize] = item_stack(44, 1);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let banner_to_player = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: LOOM_BANNER_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            banner_to_player.changed_slots,
            BTreeMap::from([
                (LOOM_BANNER_SLOT, ProtocolHashedStack::Empty),
                (LOOM_PLAYER_MAIN_START, hashed_item_stack(42, 3)),
            ])
        );

        let dye_to_player = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: LOOM_DYE_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            dye_to_player.changed_slots,
            BTreeMap::from([
                (LOOM_DYE_SLOT, ProtocolHashedStack::Empty),
                (LOOM_PLAYER_MAIN_START + 1, hashed_item_stack(43, 2)),
            ])
        );

        let pattern_to_player = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: LOOM_PATTERN_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            pattern_to_player.changed_slots,
            BTreeMap::from([
                (LOOM_PATTERN_SLOT, ProtocolHashedStack::Empty),
                (LOOM_PLAYER_MAIN_START + 2, hashed_item_stack(44, 1)),
            ])
        );
        assert_eq!(
            open_container_slot_item(&store, LOOM_BANNER_SLOT),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, LOOM_DYE_SLOT),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, LOOM_PATTERN_SLOT),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, LOOM_PLAYER_MAIN_START),
            item_stack(42, 3)
        );
        assert_eq!(
            open_container_slot_item(&store, LOOM_PLAYER_MAIN_START + 1),
            item_stack(43, 2)
        );
        assert_eq!(
            open_container_slot_item(&store, LOOM_PLAYER_MAIN_START + 2),
            item_stack(44, 1)
        );
    }

    #[test]
    fn apply_local_loom_quick_move_routes_tagged_items_to_input_slots() {
        let mut store = WorldStore::new();
        apply_item_tags(
            &mut store,
            vec![
                (LOOM_BANNER_ITEM_TAG, vec![42]),
                (LOOM_DYE_ITEM_TAG, vec![43]),
                (LOOM_PATTERN_ITEM_TAG, vec![44]),
            ],
        );
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
            title: "Loom".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); LOOM_TOTAL_SLOT_COUNT as usize];
        items[LOOM_PLAYER_MAIN_START as usize] = item_stack(42, 1);
        items[(LOOM_PLAYER_MAIN_START + 1) as usize] = item_stack(43, 2);
        items[(LOOM_PLAYER_MAIN_START + 2) as usize] = item_stack(44, 1);
        items[LOOM_HOTBAR_START as usize] = item_stack(45, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 14,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let banner_to_input = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: LOOM_PLAYER_MAIN_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            banner_to_input.changed_slots,
            BTreeMap::from([
                (LOOM_BANNER_SLOT, hashed_item_stack(42, 1)),
                (LOOM_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
            ])
        );

        let dye_to_input = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: LOOM_PLAYER_MAIN_START + 1,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            dye_to_input.changed_slots,
            BTreeMap::from([
                (LOOM_DYE_SLOT, hashed_item_stack(43, 2)),
                (LOOM_PLAYER_MAIN_START + 1, ProtocolHashedStack::Empty),
            ])
        );

        let pattern_to_input = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: LOOM_PLAYER_MAIN_START + 2,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            pattern_to_input.changed_slots,
            BTreeMap::from([
                (LOOM_PATTERN_SLOT, hashed_item_stack(44, 1)),
                (LOOM_PLAYER_MAIN_START + 2, ProtocolHashedStack::Empty),
            ])
        );

        let hotbar_to_main = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: LOOM_HOTBAR_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            hotbar_to_main.changed_slots,
            BTreeMap::from([
                (LOOM_PLAYER_MAIN_START, hashed_item_stack(45, 3)),
                (LOOM_HOTBAR_START, ProtocolHashedStack::Empty),
            ])
        );
        assert_eq!(
            open_container_slot_item(&store, LOOM_BANNER_SLOT),
            item_stack(42, 1)
        );
        assert_eq!(
            open_container_slot_item(&store, LOOM_DYE_SLOT),
            item_stack(43, 2)
        );
        assert_eq!(
            open_container_slot_item(&store, LOOM_PATTERN_SLOT),
            item_stack(44, 1)
        );
        assert_eq!(
            open_container_slot_item(&store, LOOM_PLAYER_MAIN_START),
            item_stack(45, 3)
        );
        assert_eq!(
            open_container_slot_item(&store, LOOM_HOTBAR_START),
            ProtocolItemStackSummary::empty()
        );
    }

    #[test]
    fn apply_local_merchant_quick_move_moves_non_result_slots() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
            title: "Merchant".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); MERCHANT_TOTAL_SLOT_COUNT as usize];
        items[MERCHANT_PAYMENT_SLOT_1 as usize] = item_stack(42, 3);
        items[MERCHANT_PAYMENT_SLOT_2 as usize] = item_stack(43, 1);
        items[MERCHANT_RESULT_SLOT as usize] = item_stack(90, 1);
        items[MERCHANT_PLAYER_MAIN_START as usize] = item_stack(44, 2);
        items[MERCHANT_HOTBAR_START as usize] = item_stack(45, 4);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        for input in [
            ProtocolContainerInput::Pickup,
            ProtocolContainerInput::QuickMove,
        ] {
            assert_eq!(
                store.apply_local_container_click_slot(ContainerClickSlotRequest {
                    slot_num: MERCHANT_RESULT_SLOT,
                    button_num: 0,
                    input,
                }),
                Err(ContainerClickBuildError::UnsupportedLocalClickInput(input))
            );
        }

        let payment_1_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: MERCHANT_PAYMENT_SLOT_1,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            payment_1_move.changed_slots,
            BTreeMap::from([
                (MERCHANT_PAYMENT_SLOT_1, ProtocolHashedStack::Empty),
                (MERCHANT_PLAYER_MAIN_START + 1, hashed_item_stack(42, 3)),
            ])
        );

        let payment_2_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: MERCHANT_PAYMENT_SLOT_2,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            payment_2_move.changed_slots,
            BTreeMap::from([
                (MERCHANT_PAYMENT_SLOT_2, ProtocolHashedStack::Empty),
                (MERCHANT_PLAYER_MAIN_START + 2, hashed_item_stack(43, 1)),
            ])
        );

        let main_to_hotbar = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: MERCHANT_PLAYER_MAIN_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            main_to_hotbar.changed_slots,
            BTreeMap::from([
                (MERCHANT_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
                (MERCHANT_HOTBAR_START + 1, hashed_item_stack(44, 2)),
            ])
        );

        let hotbar_to_main = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: MERCHANT_HOTBAR_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            hotbar_to_main.changed_slots,
            BTreeMap::from([
                (MERCHANT_PLAYER_MAIN_START, hashed_item_stack(45, 4)),
                (MERCHANT_HOTBAR_START, ProtocolHashedStack::Empty),
            ])
        );

        let click = store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: MERCHANT_RESULT_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(click.changed_slots, BTreeMap::new());
        assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_1),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, MERCHANT_PAYMENT_SLOT_2),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, MERCHANT_RESULT_SLOT),
            item_stack(90, 1)
        );
        assert_eq!(
            open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START),
            item_stack(45, 4)
        );
        assert_eq!(
            open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START + 1),
            item_stack(42, 3)
        );
        assert_eq!(
            open_container_slot_item(&store, MERCHANT_PLAYER_MAIN_START + 2),
            item_stack(43, 1)
        );
        assert_eq!(
            open_container_slot_item(&store, MERCHANT_HOTBAR_START),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, MERCHANT_HOTBAR_START + 1),
            item_stack(44, 2)
        );
    }

    #[test]
    fn apply_local_enchantment_quick_move_moves_menu_slots_to_player_reverse() {
        const ENCHANTMENT_TOTAL_SLOT_COUNT: usize = 38;

        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
            title: "Enchanting Table".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); ENCHANTMENT_TOTAL_SLOT_COUNT];
        items[0] = item_stack(42, 1);
        items[1] = item_stack(43, 3);
        items[29] = item_stack(44, 2);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let input_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: ENCHANTMENT_INPUT_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            input_move.changed_slots,
            BTreeMap::from([
                (ENCHANTMENT_INPUT_SLOT, ProtocolHashedStack::Empty),
                (ENCHANTMENT_HOTBAR_END - 1, hashed_item_stack(42, 1)),
            ])
        );

        let lapis_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: ENCHANTMENT_LAPIS_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            lapis_move.changed_slots,
            BTreeMap::from([
                (ENCHANTMENT_LAPIS_SLOT, ProtocolHashedStack::Empty),
                (ENCHANTMENT_HOTBAR_END - 2, hashed_item_stack(43, 3)),
            ])
        );
        assert_eq!(
            store.apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 29,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            }),
            Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                ProtocolContainerInput::QuickMove
            ))
        );
        let click = store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: 29,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(click.changed_slots, BTreeMap::new());
        assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            open_container_slot_item(&store, ENCHANTMENT_INPUT_SLOT),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, ENCHANTMENT_LAPIS_SLOT),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(open_container_slot_item(&store, 29), item_stack(44, 2));
        assert_eq!(
            open_container_slot_item(&store, ENCHANTMENT_HOTBAR_END - 2),
            item_stack(43, 3)
        );
        assert_eq!(
            open_container_slot_item(&store, ENCHANTMENT_HOTBAR_END - 1),
            item_stack(42, 1)
        );
    }

    #[test]
    fn apply_local_enchantment_quick_move_routes_lapis_to_lapis_slot() {
        const ENCHANTMENT_HOTBAR_START: i16 = 29;

        let mut store = WorldStore::new();
        store.set_enchantment_lapis_lazuli_item_ids(BTreeSet::from([43]));
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
            title: "Enchanting Table".to_string(),
        });
        let mut items =
            vec![ProtocolItemStackSummary::empty(); ENCHANTMENT_TOTAL_SLOT_COUNT as usize];
        items[ENCHANTMENT_HOTBAR_START as usize] = item_stack(43, 3);
        items[(ENCHANTMENT_HOTBAR_START + 1) as usize] = item_stack(50, 1);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let lapis_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: ENCHANTMENT_HOTBAR_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            lapis_move.changed_slots,
            BTreeMap::from([
                (ENCHANTMENT_LAPIS_SLOT, hashed_item_stack(43, 3)),
                (ENCHANTMENT_HOTBAR_START, ProtocolHashedStack::Empty),
            ])
        );
        assert_eq!(
            store.apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: ENCHANTMENT_HOTBAR_START + 1,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            }),
            Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                ProtocolContainerInput::QuickMove
            ))
        );
        assert_eq!(
            open_container_slot_item(&store, ENCHANTMENT_LAPIS_SLOT),
            item_stack(43, 3)
        );
        assert_eq!(
            open_container_slot_item(&store, ENCHANTMENT_HOTBAR_START),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, ENCHANTMENT_HOTBAR_START + 1),
            item_stack(50, 1)
        );
    }

    #[test]
    fn apply_local_smithing_input_quick_move_moves_input_slots_to_player_forward() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
            title: "Smithing".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); SMITHING_TOTAL_SLOT_COUNT as usize];
        items[SMITHING_TEMPLATE_SLOT as usize] = item_stack(42, 1);
        items[SMITHING_BASE_SLOT as usize] = item_stack(43, 2);
        items[SMITHING_ADDITIONAL_SLOT as usize] = item_stack(44, 3);
        items[SMITHING_RESULT_SLOT as usize] = item_stack(90, 1);
        items[31] = item_stack(45, 4);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let template_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: SMITHING_TEMPLATE_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            template_move.changed_slots,
            BTreeMap::from([
                (SMITHING_TEMPLATE_SLOT, ProtocolHashedStack::Empty),
                (SMITHING_PLAYER_MAIN_START, hashed_item_stack(42, 1)),
            ])
        );

        let base_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: SMITHING_BASE_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            base_move.changed_slots,
            BTreeMap::from([
                (SMITHING_BASE_SLOT, ProtocolHashedStack::Empty),
                (SMITHING_PLAYER_MAIN_START + 1, hashed_item_stack(43, 2)),
            ])
        );

        let additional_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: SMITHING_ADDITIONAL_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            additional_move.changed_slots,
            BTreeMap::from([
                (SMITHING_ADDITIONAL_SLOT, ProtocolHashedStack::Empty),
                (SMITHING_PLAYER_MAIN_START + 2, hashed_item_stack(44, 3)),
            ])
        );

        for input in [
            ProtocolContainerInput::Pickup,
            ProtocolContainerInput::QuickMove,
        ] {
            assert_eq!(
                store.apply_local_container_click_slot(ContainerClickSlotRequest {
                    slot_num: SMITHING_RESULT_SLOT,
                    button_num: 0,
                    input,
                }),
                Err(ContainerClickBuildError::UnsupportedLocalClickInput(input))
            );
        }
        assert_eq!(
            store.apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 31,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            }),
            Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                ProtocolContainerInput::QuickMove
            ))
        );
        let click = store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: 31,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(click.changed_slots, BTreeMap::new());
        assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            open_container_slot_item(&store, SMITHING_TEMPLATE_SLOT),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, SMITHING_BASE_SLOT),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, SMITHING_ADDITIONAL_SLOT),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, SMITHING_RESULT_SLOT),
            item_stack(90, 1)
        );
        assert_eq!(open_container_slot_item(&store, 31), item_stack(45, 4));
        assert_eq!(
            open_container_slot_item(&store, SMITHING_PLAYER_MAIN_START),
            item_stack(42, 1)
        );
        assert_eq!(
            open_container_slot_item(&store, SMITHING_PLAYER_MAIN_START + 1),
            item_stack(43, 2)
        );
        assert_eq!(
            open_container_slot_item(&store, SMITHING_PLAYER_MAIN_START + 2),
            item_stack(44, 3)
        );
    }

    #[test]
    fn apply_local_cartography_table_result_and_player_quick_move_require_server_authority() {
        const CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT: usize = 39;

        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
            title: "Cartography Table".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT];
        items[0] = item_stack(42, 1);
        items[1] = item_stack(43, 1);
        items[CARTOGRAPHY_TABLE_RESULT_SLOT as usize] = item_stack(90, 1);
        items[30] = item_stack(44, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        for input in [
            ProtocolContainerInput::Pickup,
            ProtocolContainerInput::QuickMove,
        ] {
            assert_eq!(
                store.apply_local_container_click_slot(ContainerClickSlotRequest {
                    slot_num: CARTOGRAPHY_TABLE_RESULT_SLOT,
                    button_num: 0,
                    input,
                }),
                Err(ContainerClickBuildError::UnsupportedLocalClickInput(input))
            );
        }
        assert_eq!(
            store.apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 30,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            }),
            Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                ProtocolContainerInput::QuickMove
            ))
        );
        let click = store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: 30,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(click.changed_slots, BTreeMap::new());
        assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 1));
        assert_eq!(open_container_slot_item(&store, 1), item_stack(43, 1));
        assert_eq!(
            open_container_slot_item(&store, CARTOGRAPHY_TABLE_RESULT_SLOT),
            item_stack(90, 1)
        );
        assert_eq!(open_container_slot_item(&store, 30), item_stack(44, 3));
    }

    #[test]
    fn apply_local_cartography_table_quick_move_moves_input_slots_to_player_forward() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
            title: "Cartography Table".to_string(),
        });
        let mut items =
            vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
        items[CARTOGRAPHY_TABLE_MAP_SLOT as usize] = item_stack(42, 1);
        items[CARTOGRAPHY_TABLE_ADDITIONAL_SLOT as usize] = item_stack(43, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let map_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: CARTOGRAPHY_TABLE_MAP_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            map_move.changed_slots,
            BTreeMap::from([
                (CARTOGRAPHY_TABLE_MAP_SLOT, ProtocolHashedStack::Empty),
                (
                    CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
                    hashed_item_stack(42, 1)
                ),
            ])
        );
        assert_eq!(map_move.carried_item, ProtocolHashedStack::Empty);

        let additional_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: CARTOGRAPHY_TABLE_ADDITIONAL_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            additional_move.changed_slots,
            BTreeMap::from([
                (
                    CARTOGRAPHY_TABLE_ADDITIONAL_SLOT,
                    ProtocolHashedStack::Empty
                ),
                (
                    CARTOGRAPHY_TABLE_PLAYER_MAIN_START + 1,
                    hashed_item_stack(43, 3)
                ),
            ])
        );
        assert_eq!(additional_move.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            open_container_slot_item(&store, CARTOGRAPHY_TABLE_MAP_SLOT),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, CARTOGRAPHY_TABLE_ADDITIONAL_SLOT),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, CARTOGRAPHY_TABLE_PLAYER_MAIN_START),
            item_stack(42, 1)
        );
        assert_eq!(
            open_container_slot_item(&store, CARTOGRAPHY_TABLE_PLAYER_MAIN_START + 1),
            item_stack(43, 3)
        );
    }

    #[test]
    fn apply_local_cartography_table_quick_move_routes_additional_and_player_ranges() {
        let mut store = WorldStore::new();
        store.set_cartography_additional_item_ids(BTreeSet::from([43, 44, 45]));
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
            title: "Cartography Table".to_string(),
        });
        let mut items =
            vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
        items[CARTOGRAPHY_TABLE_PLAYER_MAIN_START as usize] = item_stack(43, 2);
        items[(CARTOGRAPHY_TABLE_PLAYER_MAIN_START + 1) as usize] = item_stack(50, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let additional_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            additional_move.changed_slots,
            BTreeMap::from([
                (CARTOGRAPHY_TABLE_ADDITIONAL_SLOT, hashed_item_stack(43, 2)),
                (
                    CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
                    ProtocolHashedStack::Empty
                ),
            ])
        );

        let main_to_hotbar = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: CARTOGRAPHY_TABLE_PLAYER_MAIN_START + 1,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            main_to_hotbar.changed_slots,
            BTreeMap::from([
                (
                    CARTOGRAPHY_TABLE_PLAYER_MAIN_START + 1,
                    ProtocolHashedStack::Empty
                ),
                (CARTOGRAPHY_TABLE_HOTBAR_START, hashed_item_stack(50, 3)),
            ])
        );

        let hotbar_to_main = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: CARTOGRAPHY_TABLE_HOTBAR_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            hotbar_to_main.changed_slots,
            BTreeMap::from([
                (
                    CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
                    hashed_item_stack(50, 3)
                ),
                (CARTOGRAPHY_TABLE_HOTBAR_START, ProtocolHashedStack::Empty),
            ])
        );
        assert_eq!(
            open_container_slot_item(&store, CARTOGRAPHY_TABLE_ADDITIONAL_SLOT),
            item_stack(43, 2)
        );
        assert_eq!(
            open_container_slot_item(&store, CARTOGRAPHY_TABLE_PLAYER_MAIN_START),
            item_stack(50, 3)
        );
        assert_eq!(
            open_container_slot_item(&store, CARTOGRAPHY_TABLE_HOTBAR_START),
            ProtocolItemStackSummary::empty()
        );
    }

    #[test]
    fn apply_local_cartography_table_player_map_id_quick_move_requires_server_authority() {
        let mut store = WorldStore::new();
        store.set_cartography_additional_item_ids(BTreeSet::from([43, 44, 45]));
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
            title: "Cartography Table".to_string(),
        });
        let mut items =
            vec![ProtocolItemStackSummary::empty(); CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT as usize];
        items[CARTOGRAPHY_TABLE_PLAYER_MAIN_START as usize] = map_id_item_stack(42, 1, 7);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        assert_eq!(
            store.apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            }),
            Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                ProtocolContainerInput::QuickMove
            ))
        );
        assert_eq!(
            open_container_slot_item(&store, CARTOGRAPHY_TABLE_PLAYER_MAIN_START),
            map_id_item_stack(42, 1, 7)
        );
    }

    #[test]
    fn apply_local_stonecutter_quick_move_moves_input_slot_to_player_forward() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
            title: "Stonecutter".to_string(),
        });
        let mut items =
            vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
        items[STONECUTTER_INPUT_SLOT as usize] = item_stack(42, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: STONECUTTER_INPUT_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (STONECUTTER_INPUT_SLOT, ProtocolHashedStack::Empty),
                (STONECUTTER_PLAYER_MAIN_START, hashed_item_stack(42, 3)),
            ])
        );
        assert_eq!(
            open_container_slot_item(&store, STONECUTTER_INPUT_SLOT),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, STONECUTTER_PLAYER_MAIN_START),
            item_stack(42, 3)
        );
    }

    #[test]
    fn apply_local_stonecutter_quick_move_moves_player_main_and_hotbar_ranges() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
            title: "Stonecutter".to_string(),
        });
        let mut items =
            vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
        items[STONECUTTER_PLAYER_MAIN_START as usize] = item_stack(42, 3);
        items[STONECUTTER_HOTBAR_START as usize] = item_stack(43, 4);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let main_to_hotbar = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: STONECUTTER_PLAYER_MAIN_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            main_to_hotbar.changed_slots,
            BTreeMap::from([
                (STONECUTTER_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
                (STONECUTTER_HOTBAR_START + 1, hashed_item_stack(42, 3)),
            ])
        );

        let hotbar_to_main = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: STONECUTTER_HOTBAR_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            hotbar_to_main.changed_slots,
            BTreeMap::from([
                (STONECUTTER_PLAYER_MAIN_START, hashed_item_stack(43, 4)),
                (STONECUTTER_HOTBAR_START, ProtocolHashedStack::Empty),
            ])
        );

        assert_eq!(
            open_container_slot_item(&store, STONECUTTER_PLAYER_MAIN_START),
            item_stack(43, 4)
        );
        assert_eq!(
            open_container_slot_item(&store, STONECUTTER_HOTBAR_START),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, STONECUTTER_HOTBAR_START + 1),
            item_stack(42, 3)
        );
    }

    #[test]
    fn apply_local_stonecutter_quick_move_routes_valid_recipe_input_to_input_slot() {
        let mut store = WorldStore::new();
        store.apply_update_recipes(update_stonecutter_recipes(vec![stonecutter_recipe(vec![
            42, 43,
        ])]));
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
            title: "Stonecutter".to_string(),
        });
        let mut items =
            vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
        items[STONECUTTER_PLAYER_MAIN_START as usize] = item_stack(42, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 14,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: STONECUTTER_PLAYER_MAIN_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (STONECUTTER_INPUT_SLOT, hashed_item_stack(42, 3)),
                (STONECUTTER_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
            ])
        );
        assert_eq!(
            open_container_slot_item(&store, STONECUTTER_INPUT_SLOT),
            item_stack(42, 3)
        );
        assert_eq!(
            open_container_slot_item(&store, STONECUTTER_PLAYER_MAIN_START),
            ProtocolItemStackSummary::empty()
        );
    }

    #[test]
    fn apply_local_stonecutter_result_pickup_and_quick_move_require_server_authority() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_STONECUTTER_ID,
            title: "Stonecutter".to_string(),
        });
        let mut items =
            vec![ProtocolItemStackSummary::empty(); STONECUTTER_TOTAL_SLOT_COUNT as usize];
        items[STONECUTTER_INPUT_SLOT as usize] = item_stack(42, 1);
        items[STONECUTTER_RESULT_SLOT as usize] = item_stack(90, 1);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 15,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        for input in [
            ProtocolContainerInput::Pickup,
            ProtocolContainerInput::QuickMove,
        ] {
            assert_eq!(
                store.apply_local_container_click_slot(ContainerClickSlotRequest {
                    slot_num: STONECUTTER_RESULT_SLOT,
                    button_num: 0,
                    input,
                }),
                Err(ContainerClickBuildError::UnsupportedLocalClickInput(input))
            );
        }
        let click = store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: STONECUTTER_RESULT_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(click.changed_slots, BTreeMap::new());
        assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            open_container_slot_item(&store, STONECUTTER_INPUT_SLOT),
            item_stack(42, 1)
        );
        assert_eq!(
            open_container_slot_item(&store, STONECUTTER_RESULT_SLOT),
            item_stack(90, 1)
        );
    }

    #[test]
    fn apply_local_brewing_stand_quick_move_moves_brewing_slots_to_player_reverse() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_BREWING_STAND_ID,
            title: "Brewing Stand".to_string(),
        });
        let mut items =
            vec![ProtocolItemStackSummary::empty(); BREWING_STAND_TOTAL_SLOT_COUNT as usize];
        items[BREWING_STAND_BOTTLE_SLOT_START as usize] = item_stack(42, 1);
        items[BREWING_STAND_INGREDIENT_SLOT as usize] = item_stack(43, 2);
        items[BREWING_STAND_FUEL_SLOT as usize] = item_stack(44, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let bottle_to_player = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: BREWING_STAND_BOTTLE_SLOT_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            bottle_to_player.changed_slots,
            BTreeMap::from([
                (BREWING_STAND_BOTTLE_SLOT_START, ProtocolHashedStack::Empty),
                (BREWING_STAND_HOTBAR_END - 1, hashed_item_stack(42, 1)),
            ])
        );

        let ingredient_to_player = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: BREWING_STAND_INGREDIENT_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            ingredient_to_player.changed_slots,
            BTreeMap::from([
                (BREWING_STAND_INGREDIENT_SLOT, ProtocolHashedStack::Empty),
                (BREWING_STAND_HOTBAR_END - 2, hashed_item_stack(43, 2)),
            ])
        );

        let fuel_to_player = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: BREWING_STAND_FUEL_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            fuel_to_player.changed_slots,
            BTreeMap::from([
                (BREWING_STAND_FUEL_SLOT, ProtocolHashedStack::Empty),
                (BREWING_STAND_HOTBAR_END - 3, hashed_item_stack(44, 3)),
            ])
        );
    }

    #[test]
    fn apply_local_brewing_stand_quick_move_routes_player_items_to_brewing_slots() {
        let mut store = WorldStore::new();
        store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 64)]));
        store.set_brewing_potion_item_ids(BTreeSet::from([42]));
        store.set_brewing_ingredient_item_ids(BTreeSet::from([43]));
        apply_item_tags(&mut store, vec![(BREWING_STAND_FUEL_ITEM_TAG, vec![44])]);
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_BREWING_STAND_ID,
            title: "Brewing Stand".to_string(),
        });
        let mut items =
            vec![ProtocolItemStackSummary::empty(); BREWING_STAND_TOTAL_SLOT_COUNT as usize];
        items[BREWING_STAND_PLAYER_MAIN_START as usize] = item_stack(42, 3);
        items[(BREWING_STAND_PLAYER_MAIN_START + 1) as usize] = item_stack(43, 2);
        items[(BREWING_STAND_PLAYER_MAIN_START + 2) as usize] = item_stack(44, 5);
        items[BREWING_STAND_HOTBAR_START as usize] = item_stack(45, 4);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let potion_to_bottle_slot = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: BREWING_STAND_PLAYER_MAIN_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            potion_to_bottle_slot.changed_slots,
            BTreeMap::from([
                (BREWING_STAND_BOTTLE_SLOT_START, hashed_item_stack(42, 1)),
                (BREWING_STAND_PLAYER_MAIN_START, hashed_item_stack(42, 2)),
            ])
        );

        let ingredient_to_slot = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: BREWING_STAND_PLAYER_MAIN_START + 1,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            ingredient_to_slot.changed_slots,
            BTreeMap::from([
                (BREWING_STAND_INGREDIENT_SLOT, hashed_item_stack(43, 2)),
                (
                    BREWING_STAND_PLAYER_MAIN_START + 1,
                    ProtocolHashedStack::Empty
                ),
            ])
        );

        let fuel_to_slot = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: BREWING_STAND_PLAYER_MAIN_START + 2,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            fuel_to_slot.changed_slots,
            BTreeMap::from([
                (BREWING_STAND_FUEL_SLOT, hashed_item_stack(44, 5)),
                (
                    BREWING_STAND_PLAYER_MAIN_START + 2,
                    ProtocolHashedStack::Empty
                ),
            ])
        );

        let hotbar_to_main = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: BREWING_STAND_HOTBAR_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            hotbar_to_main.changed_slots,
            BTreeMap::from([
                (
                    BREWING_STAND_PLAYER_MAIN_START + 1,
                    hashed_item_stack(45, 4)
                ),
                (BREWING_STAND_HOTBAR_START, ProtocolHashedStack::Empty),
            ])
        );
        assert_eq!(
            open_container_slot_item(&store, BREWING_STAND_BOTTLE_SLOT_START),
            item_stack(42, 1)
        );
        assert_eq!(
            open_container_slot_item(&store, BREWING_STAND_PLAYER_MAIN_START),
            item_stack(42, 2)
        );
        assert_eq!(
            open_container_slot_item(&store, BREWING_STAND_INGREDIENT_SLOT),
            item_stack(43, 2)
        );
        assert_eq!(
            open_container_slot_item(&store, BREWING_STAND_FUEL_SLOT),
            item_stack(44, 5)
        );
        assert_eq!(
            open_container_slot_item(&store, BREWING_STAND_HOTBAR_START),
            ProtocolItemStackSummary::empty()
        );
    }

    #[test]
    fn apply_local_grindstone_input_quick_move_moves_inputs_to_player_forward() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
            title: "Grindstone".to_string(),
        });
        let mut items =
            vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
        items[GRINDSTONE_INPUT_SLOT as usize] = item_stack(42, 1);
        items[GRINDSTONE_ADDITIONAL_SLOT as usize] = item_stack(43, 2);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let input_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: GRINDSTONE_INPUT_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            input_move.changed_slots,
            BTreeMap::from([
                (GRINDSTONE_INPUT_SLOT, ProtocolHashedStack::Empty),
                (GRINDSTONE_PLAYER_MAIN_START, hashed_item_stack(42, 1)),
            ])
        );

        let additional_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: GRINDSTONE_ADDITIONAL_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            additional_move.changed_slots,
            BTreeMap::from([
                (GRINDSTONE_ADDITIONAL_SLOT, ProtocolHashedStack::Empty),
                (GRINDSTONE_PLAYER_MAIN_START + 1, hashed_item_stack(43, 2)),
            ])
        );
        assert_eq!(
            open_container_slot_item(&store, GRINDSTONE_INPUT_SLOT),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, GRINDSTONE_ADDITIONAL_SLOT),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, GRINDSTONE_PLAYER_MAIN_START),
            item_stack(42, 1)
        );
        assert_eq!(
            open_container_slot_item(&store, GRINDSTONE_PLAYER_MAIN_START + 1),
            item_stack(43, 2)
        );
    }

    #[test]
    fn apply_local_grindstone_player_to_input_quick_move_requires_server_authority() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
            title: "Grindstone".to_string(),
        });
        let mut items =
            vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
        items[GRINDSTONE_PLAYER_MAIN_START as usize] = item_stack(42, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let request = ContainerClickSlotRequest {
            slot_num: GRINDSTONE_PLAYER_MAIN_START,
            button_num: 0,
            input: ProtocolContainerInput::QuickMove,
        };
        assert_eq!(
            store.apply_local_container_click_slot(request),
            Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                ProtocolContainerInput::QuickMove
            ))
        );
        let click = store.build_container_click_slot(request).unwrap();
        assert_eq!(click.changed_slots, BTreeMap::new());
        assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            open_container_slot_item(&store, GRINDSTONE_INPUT_SLOT),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, GRINDSTONE_PLAYER_MAIN_START),
            item_stack(42, 3)
        );
    }

    #[test]
    fn apply_local_grindstone_quick_move_moves_player_ranges_when_inputs_full() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
            title: "Grindstone".to_string(),
        });
        let mut items =
            vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
        items[GRINDSTONE_INPUT_SLOT as usize] = item_stack(10, 1);
        items[GRINDSTONE_ADDITIONAL_SLOT as usize] = item_stack(11, 1);
        items[GRINDSTONE_PLAYER_MAIN_START as usize] = item_stack(42, 3);
        items[GRINDSTONE_HOTBAR_START as usize] = item_stack(43, 4);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 14,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let main_to_hotbar = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: GRINDSTONE_PLAYER_MAIN_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            main_to_hotbar.changed_slots,
            BTreeMap::from([
                (GRINDSTONE_PLAYER_MAIN_START, ProtocolHashedStack::Empty),
                (GRINDSTONE_HOTBAR_START + 1, hashed_item_stack(42, 3)),
            ])
        );

        let hotbar_to_main = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: GRINDSTONE_HOTBAR_START,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(
            hotbar_to_main.changed_slots,
            BTreeMap::from([
                (GRINDSTONE_PLAYER_MAIN_START, hashed_item_stack(43, 4)),
                (GRINDSTONE_HOTBAR_START, ProtocolHashedStack::Empty),
            ])
        );
        assert_eq!(
            open_container_slot_item(&store, GRINDSTONE_PLAYER_MAIN_START),
            item_stack(43, 4)
        );
        assert_eq!(
            open_container_slot_item(&store, GRINDSTONE_HOTBAR_START),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, GRINDSTONE_HOTBAR_START + 1),
            item_stack(42, 3)
        );
    }

    #[test]
    fn apply_local_grindstone_result_pickup_and_quick_move_require_server_authority() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_GRINDSTONE_ID,
            title: "Grindstone".to_string(),
        });
        let mut items =
            vec![ProtocolItemStackSummary::empty(); GRINDSTONE_TOTAL_SLOT_COUNT as usize];
        items[GRINDSTONE_INPUT_SLOT as usize] = item_stack(42, 1);
        items[GRINDSTONE_RESULT_SLOT as usize] = item_stack(90, 1);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 16,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        for input in [
            ProtocolContainerInput::Pickup,
            ProtocolContainerInput::QuickMove,
        ] {
            assert_eq!(
                store.apply_local_container_click_slot(ContainerClickSlotRequest {
                    slot_num: GRINDSTONE_RESULT_SLOT,
                    button_num: 0,
                    input,
                }),
                Err(ContainerClickBuildError::UnsupportedLocalClickInput(input))
            );
        }
        let click = store
            .build_container_click_slot(ContainerClickSlotRequest {
                slot_num: GRINDSTONE_RESULT_SLOT,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        assert_eq!(click.changed_slots, BTreeMap::new());
        assert_eq!(click.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            open_container_slot_item(&store, GRINDSTONE_INPUT_SLOT),
            item_stack(42, 1)
        );
        assert_eq!(
            open_container_slot_item(&store, GRINDSTONE_RESULT_SLOT),
            item_stack(90, 1)
        );
    }

    #[test]
    fn apply_local_hopper_quick_move_moves_hopper_to_player_reverse() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_HOPPER_ID,
            title: "Hopper".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 41];
        items[0] = item_stack(42, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(quick_move.container_id, 7);
        assert_eq!(quick_move.state_id, 12);
        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (0, ProtocolHashedStack::Empty),
                (40, hashed_item_stack(42, 3))
            ])
        );
        let slots = &store.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
        assert_eq!(slots[40].item, item_stack(42, 3));
    }

    #[test]
    fn apply_local_hopper_quick_move_moves_player_to_hopper_forward() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_HOPPER_ID,
            title: "Hopper".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 41];
        items[5] = item_stack(42, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 5,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(quick_move.container_id, 7);
        assert_eq!(quick_move.state_id, 13);
        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (0, hashed_item_stack(42, 3)),
                (5, ProtocolHashedStack::Empty)
            ])
        );
        let slots = &store.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(42, 3));
        assert_eq!(slots[5].item, ProtocolItemStackSummary::empty());
    }

    #[test]
    fn apply_local_shulker_box_quick_move_moves_shulker_to_player_reverse() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_SHULKER_BOX_ID,
            title: "Shulker Box".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 63];
        items[0] = item_stack(42, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(quick_move.container_id, 7);
        assert_eq!(quick_move.state_id, 12);
        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (0, ProtocolHashedStack::Empty),
                (62, hashed_item_stack(42, 3))
            ])
        );
        let slots = &store.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, ProtocolItemStackSummary::empty());
        assert_eq!(slots[62].item, item_stack(42, 3));
    }

    #[test]
    fn apply_local_shulker_box_quick_move_moves_player_to_shulker_forward() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_SHULKER_BOX_ID,
            title: "Shulker Box".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 63];
        items[27] = item_stack(42, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 13,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 27,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(quick_move.container_id, 7);
        assert_eq!(quick_move.state_id, 13);
        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (0, hashed_item_stack(42, 3)),
                (27, ProtocolHashedStack::Empty)
            ])
        );
        let slots = &store.inventory().open_container.as_ref().unwrap().slots;
        assert_eq!(slots[0].item, item_stack(42, 3));
        assert_eq!(slots[27].item, ProtocolItemStackSummary::empty());
    }

    #[test]
    fn apply_local_furnace_quick_move_moves_result_to_player_reverse() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
            title: "Furnace".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 39];
        items[2] = item_stack(42, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 2,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(quick_move.container_id, 7);
        assert_eq!(quick_move.state_id, 12);
        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (2, ProtocolHashedStack::Empty),
                (38, hashed_item_stack(42, 3)),
            ])
        );
        assert_eq!(quick_move.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            open_container_slot_item(&store, 2),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(open_container_slot_item(&store, 38), item_stack(42, 3));
    }

    #[test]
    fn apply_local_furnace_quick_move_routes_input_and_fuel_to_furnace_slots() {
        let mut store = WorldStore::new();
        store.apply_update_recipes(update_recipes(vec![("minecraft:furnace_input", vec![42])]));
        store.set_furnace_fuel_item_ids(BTreeSet::from([43]));
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
            title: "Furnace".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 39];
        items[3] = item_stack(42, 3);
        items[30] = item_stack(43, 2);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let input_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 3,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        let fuel_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 30,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            input_move.changed_slots,
            BTreeMap::from([
                (0, hashed_item_stack(42, 3)),
                (3, ProtocolHashedStack::Empty),
            ])
        );
        assert_eq!(
            fuel_move.changed_slots,
            BTreeMap::from([
                (1, hashed_item_stack(43, 2)),
                (30, ProtocolHashedStack::Empty),
            ])
        );
        assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
        assert_eq!(open_container_slot_item(&store, 1), item_stack(43, 2));
        assert_eq!(
            open_container_slot_item(&store, 3),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            open_container_slot_item(&store, 30),
            ProtocolItemStackSummary::empty()
        );
    }

    #[test]
    fn apply_local_furnace_quick_move_uses_menu_specific_input_property_set() {
        let mut store = WorldStore::new();
        store.apply_update_recipes(update_recipes(vec![("minecraft:furnace_input", vec![42])]));
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_BLAST_FURNACE_ID,
            title: "Blast Furnace".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 39];
        items[3] = item_stack(42, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let fallback_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 3,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            fallback_move.changed_slots,
            BTreeMap::from([
                (3, ProtocolHashedStack::Empty),
                (30, hashed_item_stack(42, 3)),
            ])
        );
        assert_eq!(
            open_container_slot_item(&store, 0),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(open_container_slot_item(&store, 30), item_stack(42, 3));

        store.apply_update_recipes(update_recipes(vec![(
            "minecraft:blast_furnace_input",
            vec![42],
        )]));
        store.apply_container_set_slot(ProtocolContainerSetSlot {
            container_id: 7,
            state_id: 13,
            slot: 30,
            item: item_stack(42, 3),
        });

        let blast_input_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 30,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            blast_input_move.changed_slots,
            BTreeMap::from([
                (0, hashed_item_stack(42, 3)),
                (30, ProtocolHashedStack::Empty),
            ])
        );
        assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
        assert_eq!(
            open_container_slot_item(&store, 30),
            ProtocolItemStackSummary::empty()
        );
    }

    #[test]
    fn apply_local_furnace_quick_move_prioritizes_input_over_fuel() {
        let mut store = WorldStore::new();
        store.apply_update_recipes(update_recipes(vec![("minecraft:smoker_input", vec![42])]));
        store.set_furnace_fuel_item_ids(BTreeSet::from([42]));
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_SMOKER_ID,
            title: "Smoker".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 39];
        items[30] = item_stack(42, 3);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let quick_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 30,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            quick_move.changed_slots,
            BTreeMap::from([
                (0, hashed_item_stack(42, 3)),
                (30, ProtocolHashedStack::Empty),
            ])
        );
        assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
        assert_eq!(
            open_container_slot_item(&store, 1),
            ProtocolItemStackSummary::empty()
        );
    }

    #[test]
    fn apply_local_furnace_quick_move_moves_input_and_fuel_slots_to_player_forward() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
            title: "Furnace".to_string(),
        });
        let mut items = vec![ProtocolItemStackSummary::empty(); 39];
        items[0] = item_stack(42, 3);
        items[1] = item_stack(43, 2);
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let input_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();
        let fuel_move = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 1,
                button_num: 0,
                input: ProtocolContainerInput::QuickMove,
            })
            .unwrap();

        assert_eq!(
            input_move.changed_slots,
            BTreeMap::from([
                (0, ProtocolHashedStack::Empty),
                (3, hashed_item_stack(42, 3)),
            ])
        );
        assert_eq!(
            fuel_move.changed_slots,
            BTreeMap::from([
                (1, ProtocolHashedStack::Empty),
                (4, hashed_item_stack(43, 2)),
            ])
        );
        assert_eq!(open_container_slot_item(&store, 3), item_stack(42, 3));
        assert_eq!(open_container_slot_item(&store, 4), item_stack(43, 2));
    }

    #[test]
    fn apply_local_container_quick_move_rejects_non_inventory_menu() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: 99,
            title: "Unsupported".to_string(),
        });
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 3,
            items: vec![item_stack(42, 3); 9],
            carried_item: ProtocolItemStackSummary::empty(),
        });

        assert_eq!(
            store
                .apply_local_container_click_slot(ContainerClickSlotRequest {
                    slot_num: 0,
                    button_num: 0,
                    input: ProtocolContainerInput::QuickMove,
                })
                .unwrap_err(),
            ContainerClickBuildError::UnsupportedLocalClickInput(ProtocolContainerInput::QuickMove)
        );
        assert_eq!(
            store.inventory().open_container.as_ref().unwrap().slots[0].item,
            item_stack(42, 3)
        );
    }

    #[test]
    fn apply_local_container_throw_drops_one_from_inventory_slot() {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 3),
        });
        assert!(store.open_local_inventory());

        let throw = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_HOTBAR_START,
                button_num: 0,
                input: ProtocolContainerInput::Throw,
            })
            .unwrap();

        assert_eq!(throw.container_id, INVENTORY_MENU_CONTAINER_ID);
        assert_eq!(throw.input, ProtocolContainerInput::Throw);
        assert_eq!(
            throw.changed_slots,
            BTreeMap::from([(INVENTORY_MENU_HOTBAR_START, hashed_item_stack(42, 2))])
        );
        assert_eq!(throw.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
            item_stack(42, 2)
        );
        assert_eq!(player_slot_item(&store, 0), item_stack(42, 2));
    }

    #[test]
    fn apply_local_container_throw_drops_full_stack_and_requires_empty_cursor() {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 3),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 1,
            item: item_stack(43, 4),
        });
        store.apply_set_cursor_item(ProtocolSetCursorItem {
            item: item_stack(99, 1),
        });
        assert!(store.open_local_inventory());

        let blocked = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_HOTBAR_START,
                button_num: 1,
                input: ProtocolContainerInput::Throw,
            })
            .unwrap();
        assert!(blocked.changed_slots.is_empty());
        assert_eq!(blocked.carried_item, hashed_item_stack(99, 1));
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
            item_stack(42, 3)
        );

        store.apply_set_cursor_item(ProtocolSetCursorItem {
            item: ProtocolItemStackSummary::empty(),
        });
        let drop_stack = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_HOTBAR_START + 1,
                button_num: 1,
                input: ProtocolContainerInput::Throw,
            })
            .unwrap();

        assert_eq!(
            drop_stack.changed_slots,
            BTreeMap::from([(INVENTORY_MENU_HOTBAR_START + 1, ProtocolHashedStack::Empty)])
        );
        assert_eq!(drop_stack.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START + 1),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            player_slot_item(&store, 1),
            ProtocolItemStackSummary::empty()
        );
    }

    #[test]
    fn apply_local_container_throw_rejects_non_inventory_menu() {
        let mut store = WorldStore::new();
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: 1,
            title: "Chest".to_string(),
        });
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 3,
            items: vec![item_stack(42, 3)],
            carried_item: ProtocolItemStackSummary::empty(),
        });

        assert_eq!(
            store
                .apply_local_container_click_slot(ContainerClickSlotRequest {
                    slot_num: 0,
                    button_num: 0,
                    input: ProtocolContainerInput::Throw,
                })
                .unwrap_err(),
            ContainerClickBuildError::UnsupportedLocalClickInput(ProtocolContainerInput::Throw)
        );
        assert_eq!(
            store.inventory().open_container.as_ref().unwrap().slots[0].item,
            item_stack(42, 3)
        );
    }

    #[test]
    fn apply_local_container_swap_exchanges_hovered_slot_with_hotbar_button() {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 3),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 9,
            item: item_stack(43, 2),
        });
        assert!(store.open_local_inventory());

        let swap = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_MAIN_START,
                button_num: 0,
                input: ProtocolContainerInput::Swap,
            })
            .unwrap();

        assert_eq!(swap.container_id, INVENTORY_MENU_CONTAINER_ID);
        assert_eq!(swap.input, ProtocolContainerInput::Swap);
        assert_eq!(
            swap.changed_slots,
            BTreeMap::from([
                (INVENTORY_MENU_MAIN_START, hashed_item_stack(42, 3)),
                (INVENTORY_MENU_HOTBAR_START, hashed_item_stack(43, 2)),
            ])
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
            item_stack(42, 3)
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
            item_stack(43, 2)
        );
        assert_eq!(player_slot_item(&store, 9), item_stack(42, 3));
        assert_eq!(player_slot_item(&store, 0), item_stack(43, 2));
    }

    #[test]
    fn apply_local_container_swap_moves_hovered_slot_to_empty_offhand() {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 9,
            item: item_stack(44, 5),
        });
        assert!(store.open_local_inventory());

        let swap = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_MAIN_START,
                button_num: 40,
                input: ProtocolContainerInput::Swap,
            })
            .unwrap();

        assert_eq!(
            swap.changed_slots,
            BTreeMap::from([
                (INVENTORY_MENU_MAIN_START, ProtocolHashedStack::Empty),
                (INVENTORY_MENU_OFFHAND_SLOT, hashed_item_stack(44, 5)),
            ])
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_OFFHAND_SLOT),
            item_stack(44, 5)
        );
        assert_eq!(
            player_slot_item(&store, 9),
            ProtocolItemStackSummary::empty()
        );
        assert_eq!(
            player_slot_item(&store, PLAYER_OFFHAND_SLOT),
            item_stack(44, 5)
        );
    }

    #[test]
    fn apply_local_container_swap_splits_source_when_target_slot_has_lower_limit() {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(45, 3),
        });
        assert!(store.open_local_inventory());

        let swap = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 5,
                button_num: 0,
                input: ProtocolContainerInput::Swap,
            })
            .unwrap();

        assert_eq!(
            swap.changed_slots,
            BTreeMap::from([
                (5, hashed_item_stack(45, 1)),
                (INVENTORY_MENU_HOTBAR_START, hashed_item_stack(45, 2)),
            ])
        );
        assert_eq!(inventory_menu_slot_item(&store, 5), item_stack(45, 1));
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
            item_stack(45, 2)
        );
        assert_eq!(player_slot_item(&store, 39), item_stack(45, 1));
        assert_eq!(player_slot_item(&store, 0), item_stack(45, 2));
    }

    #[test]
    fn apply_local_container_swap_requires_empty_cursor_and_inventory_menu() {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 3),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 9,
            item: item_stack(43, 2),
        });
        store.apply_set_cursor_item(ProtocolSetCursorItem {
            item: item_stack(99, 1),
        });
        assert!(store.open_local_inventory());

        let blocked = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: INVENTORY_MENU_MAIN_START,
                button_num: 0,
                input: ProtocolContainerInput::Swap,
            })
            .unwrap();
        assert!(blocked.changed_slots.is_empty());
        assert_eq!(blocked.carried_item, hashed_item_stack(99, 1));
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_MAIN_START),
            item_stack(43, 2)
        );
        assert_eq!(
            inventory_menu_slot_item(&store, INVENTORY_MENU_HOTBAR_START),
            item_stack(42, 3)
        );

        store.close_local_container(INVENTORY_MENU_CONTAINER_ID);
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: 1,
            title: "Chest".to_string(),
        });
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 3,
            items: vec![item_stack(42, 3)],
            carried_item: ProtocolItemStackSummary::empty(),
        });
        assert_eq!(
            store
                .apply_local_container_click_slot(ContainerClickSlotRequest {
                    slot_num: 0,
                    button_num: 0,
                    input: ProtocolContainerInput::Swap,
                })
                .unwrap_err(),
            ContainerClickBuildError::UnsupportedLocalClickInput(ProtocolContainerInput::Swap)
        );
    }

    #[test]
    fn apply_local_container_clone_copies_slot_stack_to_cursor_at_max_count() {
        let mut store = WorldStore::new();
        store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 16)]));
        apply_player_instabuild(&mut store, true);
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: 1,
            title: "Chest".to_string(),
        });
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 3,
            items: vec![item_stack(42, 3)],
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let clone = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 2,
                input: ProtocolContainerInput::Clone,
            })
            .unwrap();

        assert_eq!(clone.container_id, 7);
        assert_eq!(clone.state_id, 3);
        assert_eq!(clone.input, ProtocolContainerInput::Clone);
        assert!(clone.changed_slots.is_empty());
        assert_eq!(clone.carried_item, hashed_item_stack(42, 16));
        assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
        assert_eq!(store.inventory().cursor_item, item_stack(42, 16));
    }

    #[test]
    fn apply_local_container_clone_does_not_apply_without_instabuild() {
        let mut store = WorldStore::new();
        store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 16)]));
        apply_player_instabuild(&mut store, false);
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: 1,
            title: "Chest".to_string(),
        });
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 3,
            items: vec![item_stack(42, 3)],
            carried_item: ProtocolItemStackSummary::empty(),
        });

        let blocked = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 2,
                input: ProtocolContainerInput::Clone,
            })
            .unwrap();

        assert!(blocked.changed_slots.is_empty());
        assert_eq!(blocked.carried_item, ProtocolHashedStack::Empty);
        assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
        assert_eq!(
            store.inventory().cursor_item,
            ProtocolItemStackSummary::empty()
        );
    }

    #[test]
    fn apply_local_container_clone_does_not_apply_with_non_empty_cursor() {
        let mut store = WorldStore::new();
        store.set_default_item_max_stack_sizes(BTreeMap::from([(42, 16)]));
        apply_player_instabuild(&mut store, true);
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: 1,
            title: "Chest".to_string(),
        });
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 3,
            items: vec![item_stack(42, 3)],
            carried_item: item_stack(99, 1),
        });

        let blocked = store
            .apply_local_container_click_slot(ContainerClickSlotRequest {
                slot_num: 0,
                button_num: 2,
                input: ProtocolContainerInput::Clone,
            })
            .unwrap();

        assert!(blocked.changed_slots.is_empty());
        assert_eq!(blocked.carried_item, hashed_item_stack(99, 1));
        assert_eq!(open_container_slot_item(&store, 0), item_stack(42, 3));
        assert_eq!(store.inventory().cursor_item, item_stack(99, 1));
    }

    fn mount_visibility_for_entity(
        entity_type_id: i32,
        data_values: Vec<ProtocolEntityDataValue>,
    ) -> Option<MountEquipmentSlotVisibility> {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity_with_type(42, entity_type_id));
        if !data_values.is_empty() {
            assert!(store.apply_set_entity_data(ProtocolSetEntityData {
                id: 42,
                values: data_values,
            }));
        }
        store.apply_mount_screen_open(ProtocolMountScreenOpen {
            container_id: 7,
            inventory_columns: 3,
            entity_id: 42,
        });
        store.open_mount_equipment_slot_visibility()
    }

    fn protocol_add_entity_with_type(id: i32, entity_type_id: i32) -> ProtocolAddEntity {
        ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id,
            position: ProtocolVec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d {
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

    fn protocol_byte_data(data_id: u8, value: i8) -> ProtocolEntityDataValue {
        ProtocolEntityDataValue {
            data_id,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(value),
        }
    }

    fn protocol_bool_data(data_id: u8, value: bool) -> ProtocolEntityDataValue {
        ProtocolEntityDataValue {
            data_id,
            serializer_id: 8,
            value: EntityDataValueKind::Boolean(value),
        }
    }

    fn item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
        ProtocolItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }

    fn map_id_item_stack(item_id: i32, count: i32, map_id: i32) -> ProtocolItemStackSummary {
        let mut item = item_stack(item_id, count);
        item.component_patch.added = 1;
        item.component_patch
            .added_type_ids
            .push(VANILLA_MAP_ID_COMPONENT_ID);
        item.component_patch.map_id = Some(map_id);
        item
    }

    fn item_attack_range(min_reach: f32, max_reach: f32) -> ItemAttackRange {
        ItemAttackRange {
            min_reach,
            max_reach,
            min_creative_reach: min_reach,
            max_creative_reach: max_reach,
            hitbox_margin: 0.125,
            mob_factor: 1.0,
        }
    }

    fn hashed_item_stack(item_id: i32, count: i32) -> ProtocolHashedStack {
        ProtocolHashedStack::Item(ProtocolHashedItemStack {
            item_id,
            count,
            components: ProtocolHashedComponentPatch::default(),
        })
    }

    fn apply_player_instabuild(store: &mut WorldStore, instabuild: bool) {
        store.apply_player_abilities(ProtocolPlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: instabuild,
            instabuild,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
    }

    fn quick_craft_request(
        slot_num: i16,
        header: i8,
        quickcraft_type: i8,
    ) -> ContainerClickSlotRequest {
        ContainerClickSlotRequest {
            slot_num,
            button_num: quickcraft_mask(header, quickcraft_type),
            input: ProtocolContainerInput::QuickCraft,
        }
    }

    fn update_recipes(property_sets: Vec<(&str, Vec<i32>)>) -> ProtocolUpdateRecipes {
        ProtocolUpdateRecipes {
            property_sets: property_sets
                .into_iter()
                .map(|(key, item_ids)| RecipePropertySetSummary {
                    key: key.to_string(),
                    item_ids,
                })
                .collect(),
            stonecutter_recipes: Vec::new(),
        }
    }

    fn apply_item_tags(store: &mut WorldStore, tags: Vec<(&str, Vec<i32>)>) {
        store.apply_update_tags(ProtocolUpdateTags {
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

    fn update_stonecutter_recipes(
        stonecutter_recipes: Vec<StonecutterSelectableRecipeSummary>,
    ) -> ProtocolUpdateRecipes {
        ProtocolUpdateRecipes {
            property_sets: Vec::new(),
            stonecutter_recipes,
        }
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
            },
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

    fn player_slot_item(store: &WorldStore, slot: i32) -> ProtocolItemStackSummary {
        store
            .inventory()
            .player_slots
            .iter()
            .find(|state| state.slot == slot)
            .map(|state| state.item.clone())
            .unwrap_or_else(ProtocolItemStackSummary::empty)
    }

    fn inventory_menu_slot_item(store: &WorldStore, slot: i16) -> ProtocolItemStackSummary {
        store
            .inventory()
            .inventory_menu
            .slots
            .iter()
            .find(|state| state.slot == slot)
            .map(|state| state.item.clone())
            .unwrap_or_else(ProtocolItemStackSummary::empty)
    }

    fn open_container_slot_item(store: &WorldStore, slot: i16) -> ProtocolItemStackSummary {
        store
            .inventory()
            .open_container
            .as_ref()
            .unwrap()
            .slots
            .iter()
            .find(|state| state.slot == slot)
            .map(|state| state.item.clone())
            .unwrap_or_else(ProtocolItemStackSummary::empty)
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
