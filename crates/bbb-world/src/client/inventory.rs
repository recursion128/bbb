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

use crate::{
    LocalPlayerAbilitiesState, LocalPlayerExperienceState, MountScreenState, RegistryTagState,
    WorldStore,
};

mod crafting;
mod grindstone;
mod merchant;
mod smithing;
mod stonecutter;
#[cfg(test)]
mod tests;

use self::{
    crafting::{
        apply_crafter_menu_quick_move_to_slots, apply_crafting_menu_quick_move_to_slots,
        apply_crafting_menu_result_pickup_to_slots, apply_crafting_menu_result_quick_move_to_slots,
        crafter_disabled_slots,
    },
    grindstone::{
        apply_grindstone_menu_quick_move_to_slots, apply_grindstone_result_pickup_to_slots,
        grindstone_quick_move_requires_server_authority,
    },
    merchant::{
        apply_merchant_menu_quick_move_to_slots, apply_merchant_result_pickup_to_slots,
        apply_merchant_selected_offer_payment_autofill_to_slots,
        merchant_increment_selected_offer_use, merchant_max_scroll_offset,
        merchant_selected_offer_index,
    },
    smithing::{
        apply_smithing_menu_quick_move_to_slots, apply_smithing_result_pickup_to_slots,
        smithing_quick_move_requires_server_authority,
    },
    stonecutter::{
        apply_stonecutter_menu_quick_move_to_slots, apply_stonecutter_result_pickup_to_slots,
    },
};

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
const BEACON_PRIMARY_EFFECT_DATA_ID: i16 = 1;
const BEACON_SECONDARY_EFFECT_DATA_ID: i16 = 2;
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
const ANVIL_COST_DATA_ID: i16 = 0;
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
const MOUNT_SADDLE_SLOT: i16 = 0;
const MOUNT_BODY_ARMOR_SLOT: i16 = 1;
const MOUNT_INVENTORY_START: i16 = 2;
const MOUNT_PLAYER_SLOT_COUNT: i16 = 36;
const MOUNT_PLAYER_MAIN_SLOT_COUNT: i16 = 27;
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
const SMITHING_PLAYER_MAIN_END: i16 = 31;
const SMITHING_HOTBAR_START: i16 = 31;
const SMITHING_HOTBAR_END: i16 = 40;
const SMITHING_TOTAL_SLOT_COUNT: i16 = 40;
const SMITHING_TEMPLATE_PROPERTY_SET: &str = "minecraft:smithing_template";
const SMITHING_BASE_PROPERTY_SET: &str = "minecraft:smithing_base";
const SMITHING_ADDITION_PROPERTY_SET: &str = "minecraft:smithing_addition";
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
const VANILLA_MAX_DAMAGE_COMPONENT_ID: i32 = 2;
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
        set_container_data_value(&mut container.data_values, packet.id, packet.value);
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

    pub fn apply_local_beacon_confirm_effects(
        &mut self,
        primary_effect: i32,
        secondary_effect: Option<i32>,
    ) -> bool {
        let Some(primary_value) = beacon_effect_data_value(Some(primary_effect)) else {
            return false;
        };
        let Some(secondary_value) = beacon_effect_data_value(secondary_effect) else {
            return false;
        };
        let Some(container) = self
            .inventory
            .open_container
            .as_mut()
            .filter(|container| container.menu_type_id == Some(VANILLA_MENU_TYPE_BEACON_ID))
        else {
            return false;
        };

        {
            let Some(payment_slot) = container
                .slots
                .iter_mut()
                .find(|slot| slot.slot == BEACON_PAYMENT_SLOT)
            else {
                return false;
            };
            if item_stack_is_empty(&payment_slot.item) {
                return false;
            }

            payment_slot.item.count -= 1;
            normalize_item_stack(&mut payment_slot.item);
            normalize_container_slot_selection(payment_slot);
        }

        set_container_data_value(
            &mut container.data_values,
            BEACON_PRIMARY_EFFECT_DATA_ID,
            primary_value,
        );
        set_container_data_value(
            &mut container.data_values,
            BEACON_SECONDARY_EFFECT_DATA_ID,
            secondary_value,
        );
        true
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

    pub fn set_default_item_crafting_remainders(&mut self, remainders: BTreeMap<i32, i32>) {
        self.default_item_crafting_remainders_known = true;
        self.default_item_crafting_remainders = remainders
            .into_iter()
            .filter(|(item_id, remainder_id)| *item_id >= 0 && *remainder_id >= 0)
            .collect();
    }

    pub fn set_recipe_specific_crafting_remainder_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.recipe_specific_crafting_remainder_item_ids = item_ids
            .into_iter()
            .filter(|item_id| *item_id >= 0)
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
        let Some(container) = self.inventory.open_container.as_mut() else {
            return false;
        };
        let Some(offers) = container.merchant_offers.as_mut() else {
            return false;
        };
        let Ok(index_usize) = usize::try_from(index) else {
            return false;
        };
        if index_usize >= offers.offers.len() {
            return false;
        }
        let offer = offers.offers[index_usize].clone();
        offers.local_selected_offer_index = index;
        apply_merchant_selected_offer_payment_autofill_to_slots(
            container.container_id,
            &mut container.slots,
            &offer,
            &self.default_item_max_stack_sizes,
        );
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

    pub fn set_default_damageable_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.default_damageable_item_ids = item_ids
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

    pub fn set_default_mount_body_armor_kinds(
        &mut self,
        armor_kinds: BTreeMap<i32, MountArmorSlotKind>,
    ) {
        self.default_mount_body_armor_kinds = armor_kinds
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
        let (
            container_id,
            state_id,
            menu_type_id,
            mount,
            slots_before,
            data_values,
            mount_equipment_slots,
            merchant_offers,
        ) = {
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
                container.mount,
                container.slots.clone(),
                container.data_values.clone(),
                container
                    .mount
                    .and_then(|_| self.open_mount_equipment_slot_visibility()),
                container.merchant_offers.clone(),
            )
        };
        let mut slots_after = slots_before.clone();
        let mut merchant_offers_after = merchant_offers.clone();
        let mut cursor_after = self.inventory.cursor_item.clone();
        let mut quick_craft_after = self.inventory.local_quick_craft.clone();
        let anvil_result_may_pickup = anvil_result_may_pickup(
            &data_values,
            self.local_player.abilities,
            self.local_player.experience,
        );
        if menu_result_slot_requires_server_authority(menu_type_id, request.slot_num, request.input)
        {
            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                request.input,
            ));
        }
        if request.input != ProtocolContainerInput::QuickCraft && quick_craft_after.is_active() {
            quick_craft_after.reset();
        } else {
            match request.input {
                ProtocolContainerInput::Pickup
                    if menu_type_id == Some(VANILLA_MENU_TYPE_CRAFTING_ID)
                        && request.slot_num == CRAFTING_MENU_RESULT_SLOT =>
                {
                    if !apply_crafting_menu_result_pickup_to_slots(
                        &mut slots_after,
                        &mut cursor_after,
                        request.button_num,
                        self.default_item_crafting_remainders_known,
                        &self.default_item_crafting_remainders,
                        &self.recipe_specific_crafting_remainder_item_ids,
                    ) {
                        return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                            ProtocolContainerInput::Pickup,
                        ));
                    }
                }
                ProtocolContainerInput::Pickup
                    if menu_type_id == Some(VANILLA_MENU_TYPE_MERCHANT_ID)
                        && request.slot_num == MERCHANT_RESULT_SLOT =>
                {
                    let selected_offer = merchant_offers_after.as_ref().and_then(|offers| {
                        merchant_selected_offer_index(offers)
                            .ok()
                            .and_then(|index| offers.offers.get(index))
                            .cloned()
                    });
                    if !apply_merchant_result_pickup_to_slots(
                        &mut slots_after,
                        &mut cursor_after,
                        request.button_num,
                        selected_offer.as_ref(),
                        &self.default_item_max_stack_sizes,
                    ) {
                        return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                            ProtocolContainerInput::Pickup,
                        ));
                    }
                    merchant_increment_selected_offer_use(&mut merchant_offers_after);
                }
                ProtocolContainerInput::Pickup
                    if menu_type_id == Some(VANILLA_MENU_TYPE_STONECUTTER_ID)
                        && request.slot_num == STONECUTTER_RESULT_SLOT =>
                {
                    if !apply_stonecutter_result_pickup_to_slots(
                        &mut slots_after,
                        &mut cursor_after,
                        request.button_num,
                    ) {
                        return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                            ProtocolContainerInput::Pickup,
                        ));
                    }
                }
                ProtocolContainerInput::Pickup
                    if menu_type_id == Some(VANILLA_MENU_TYPE_LOOM_ID)
                        && request.slot_num == LOOM_RESULT_SLOT =>
                {
                    if !apply_loom_result_pickup_to_slots(
                        &mut slots_after,
                        &mut cursor_after,
                        request.button_num,
                    ) {
                        return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                            ProtocolContainerInput::Pickup,
                        ));
                    }
                }
                ProtocolContainerInput::Pickup
                    if menu_type_id == Some(VANILLA_MENU_TYPE_ANVIL_ID)
                        && request.slot_num == ANVIL_RESULT_SLOT =>
                {
                    if !apply_anvil_result_pickup_to_slots(
                        &mut slots_after,
                        &mut cursor_after,
                        request.button_num,
                        anvil_result_may_pickup,
                    ) {
                        return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                            ProtocolContainerInput::Pickup,
                        ));
                    }
                }
                ProtocolContainerInput::Pickup
                    if menu_type_id == Some(VANILLA_MENU_TYPE_GRINDSTONE_ID)
                        && request.slot_num == GRINDSTONE_RESULT_SLOT =>
                {
                    if !apply_grindstone_result_pickup_to_slots(
                        &mut slots_after,
                        &mut cursor_after,
                        request.button_num,
                    ) {
                        return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                            ProtocolContainerInput::Pickup,
                        ));
                    }
                }
                ProtocolContainerInput::Pickup
                    if menu_type_id == Some(VANILLA_MENU_TYPE_SMITHING_ID)
                        && request.slot_num == SMITHING_RESULT_SLOT =>
                {
                    if !apply_smithing_result_pickup_to_slots(
                        &mut slots_after,
                        &mut cursor_after,
                        request.button_num,
                    ) {
                        return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                            ProtocolContainerInput::Pickup,
                        ));
                    }
                }
                ProtocolContainerInput::Pickup
                    if menu_type_id == Some(VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID)
                        && request.slot_num == CARTOGRAPHY_TABLE_RESULT_SLOT =>
                {
                    if !apply_cartography_table_result_pickup_to_slots(
                        &mut slots_after,
                        &mut cursor_after,
                        request.button_num,
                    ) {
                        return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                            ProtocolContainerInput::Pickup,
                        ));
                    }
                }
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
                    } else if mount.is_some() {
                        if mount_inventory_quick_move_requires_server_authority(
                            &slots_after,
                            request.slot_num,
                            mount_equipment_slots,
                            &self.default_item_equipment_slots,
                            &self.default_mount_body_armor_kinds,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                        apply_mount_inventory_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            mount_equipment_slots,
                            &self.default_item_equipment_slots,
                            &self.default_mount_body_armor_kinds,
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
                        if request.slot_num == CRAFTING_MENU_RESULT_SLOT {
                            if !apply_crafting_menu_result_quick_move_to_slots(
                                container_id,
                                &mut slots_after,
                                self.default_item_crafting_remainders_known,
                                &self.default_item_crafting_remainders,
                                &self.recipe_specific_crafting_remainder_item_ids,
                                &self.default_item_max_stack_sizes,
                            ) {
                                return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                    ProtocolContainerInput::QuickMove,
                                ));
                            }
                        } else {
                            apply_crafting_menu_quick_move_to_slots(
                                container_id,
                                &mut slots_after,
                                request.slot_num,
                                &self.default_item_max_stack_sizes,
                            )
                        }
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
                            anvil_result_may_pickup,
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
                            &self.default_damageable_item_ids,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                        apply_grindstone_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &self.default_damageable_item_ids,
                            &self.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_SMITHING_ID) {
                        if smithing_quick_move_requires_server_authority(
                            &slots_after,
                            request.slot_num,
                            &self.recipes.property_sets,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                        apply_smithing_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &self.recipes.property_sets,
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
                        let selected_offer = merchant_offers_after.as_ref().and_then(|offers| {
                            merchant_selected_offer_index(offers)
                                .ok()
                                .and_then(|index| offers.offers.get(index))
                                .cloned()
                        });
                        if apply_merchant_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            selected_offer.as_ref(),
                            &self.default_item_max_stack_sizes,
                        ) {
                            merchant_increment_selected_offer_use(&mut merchant_offers_after);
                        }
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
            if menu_type_id == Some(VANILLA_MENU_TYPE_MERCHANT_ID) {
                container.merchant_offers = merchant_offers_after;
            }
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
    let components = hashed_component_patch_from_summary(&stack.component_patch)?;
    Some(ProtocolHashedStack::Item(ProtocolHashedItemStack {
        item_id,
        count: stack.count,
        components,
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

fn hashed_component_patch_from_summary(
    patch: &ProtocolDataComponentPatchSummary,
) -> Option<ProtocolHashedComponentPatch> {
    if patch == &ProtocolDataComponentPatchSummary::default() {
        return Some(ProtocolHashedComponentPatch::default());
    }

    let map_id = patch.map_id?;
    let mut expected = ProtocolDataComponentPatchSummary::default();
    expected.added = 1;
    expected.added_type_ids.push(VANILLA_MAP_ID_COMPONENT_ID);
    expected.map_id = Some(map_id);
    if patch != &expected {
        return None;
    }

    Some(ProtocolHashedComponentPatch {
        added_components: BTreeMap::from([(
            VANILLA_MAP_ID_COMPONENT_ID,
            hash_ops_crc32c_int(map_id),
        )]),
        removed_components: BTreeSet::new(),
    })
}

fn component_patch_can_be_hashed_from_summary(patch: &ProtocolDataComponentPatchSummary) -> bool {
    hashed_component_patch_from_summary(patch).is_some()
}

fn hash_ops_crc32c_int(value: i32) -> i32 {
    let mut bytes = [0u8; 5];
    bytes[0] = 8;
    bytes[1..].copy_from_slice(&value.to_le_bytes());
    crc32c(&bytes) as i32
}

fn crc32c(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0x82f6_3b78 & mask);
        }
    }
    !crc
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
    non_empty_slot_nums(slots, 1, 5)
}

fn non_empty_slot_nums(slots: &[ContainerSlot], start_slot: i16, end_slot: i16) -> Vec<i16> {
    (start_slot..end_slot)
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

fn item_stack_has_default_crafting_remainder(
    stack: &ProtocolItemStackSummary,
    default_item_crafting_remainders: &BTreeMap<i32, i32>,
) -> bool {
    stack
        .item_id
        .is_some_and(|item_id| default_item_crafting_remainders.contains_key(&item_id))
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

fn menu_result_slot_requires_server_authority(
    menu_type_id: Option<i32>,
    slot_num: i16,
    input: ProtocolContainerInput,
) -> bool {
    if !matches!(
        input,
        ProtocolContainerInput::Pickup | ProtocolContainerInput::QuickMove
    ) {
        return false;
    }
    if matches!(
        (menu_type_id, slot_num, input),
        (
            Some(VANILLA_MENU_TYPE_ANVIL_ID),
            ANVIL_RESULT_SLOT,
            ProtocolContainerInput::QuickMove
        ) | (
            Some(VANILLA_MENU_TYPE_ANVIL_ID),
            ANVIL_RESULT_SLOT,
            ProtocolContainerInput::Pickup
        ) | (
            Some(VANILLA_MENU_TYPE_CRAFTING_ID),
            CRAFTING_MENU_RESULT_SLOT,
            ProtocolContainerInput::QuickMove
        ) | (
            Some(VANILLA_MENU_TYPE_CRAFTING_ID),
            CRAFTING_MENU_RESULT_SLOT,
            ProtocolContainerInput::Pickup
        ) | (
            Some(VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID),
            CARTOGRAPHY_TABLE_RESULT_SLOT,
            ProtocolContainerInput::QuickMove
        ) | (
            Some(VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID),
            CARTOGRAPHY_TABLE_RESULT_SLOT,
            ProtocolContainerInput::Pickup
        ) | (
            Some(VANILLA_MENU_TYPE_STONECUTTER_ID),
            STONECUTTER_RESULT_SLOT,
            ProtocolContainerInput::QuickMove
        ) | (
            Some(VANILLA_MENU_TYPE_STONECUTTER_ID),
            STONECUTTER_RESULT_SLOT,
            ProtocolContainerInput::Pickup
        ) | (
            Some(VANILLA_MENU_TYPE_GRINDSTONE_ID),
            GRINDSTONE_RESULT_SLOT,
            ProtocolContainerInput::QuickMove
        ) | (
            Some(VANILLA_MENU_TYPE_GRINDSTONE_ID),
            GRINDSTONE_RESULT_SLOT,
            ProtocolContainerInput::Pickup
        ) | (
            Some(VANILLA_MENU_TYPE_LOOM_ID),
            LOOM_RESULT_SLOT,
            ProtocolContainerInput::QuickMove
        ) | (
            Some(VANILLA_MENU_TYPE_LOOM_ID),
            LOOM_RESULT_SLOT,
            ProtocolContainerInput::Pickup
        ) | (
            Some(VANILLA_MENU_TYPE_SMITHING_ID),
            SMITHING_RESULT_SLOT,
            ProtocolContainerInput::QuickMove
        ) | (
            Some(VANILLA_MENU_TYPE_SMITHING_ID),
            SMITHING_RESULT_SLOT,
            ProtocolContainerInput::Pickup
        ) | (
            Some(VANILLA_MENU_TYPE_MERCHANT_ID),
            MERCHANT_RESULT_SLOT,
            ProtocolContainerInput::QuickMove
        ) | (
            Some(VANILLA_MENU_TYPE_MERCHANT_ID),
            MERCHANT_RESULT_SLOT,
            ProtocolContainerInput::Pickup
        )
    ) {
        return false;
    }
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

fn apply_mount_inventory_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    mount_equipment_slots: Option<MountEquipmentSlotVisibility>,
    default_item_equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
    default_mount_body_armor_kinds: &BTreeMap<i32, MountArmorSlotKind>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }
    let Some(player_start) = mount_inventory_player_start_slot(slots) else {
        return;
    };
    let player_end = player_start + MOUNT_PLAYER_SLOT_COUNT;
    if !(0..player_end).contains(&slot_num) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let mut moving = source_item.clone();
    let mut changed = false;
    if slot_num < player_start {
        changed = move_item_stack_to_slots(
            container_id,
            slots,
            source_index,
            &mut moving,
            player_start,
            player_end,
            true,
            default_item_max_stack_sizes,
        );
    } else {
        if let Some(target_slot) = mount_equipment_quick_move_target(
            &source_item,
            slots,
            mount_equipment_slots,
            default_item_equipment_slots,
            default_mount_body_armor_kinds,
        ) {
            changed = move_item_stack_to_slots(
                container_id,
                slots,
                source_index,
                &mut moving,
                target_slot,
                target_slot + 1,
                false,
                default_item_max_stack_sizes,
            );
        }

        if !changed && player_start > MOUNT_INVENTORY_START {
            changed = move_item_stack_to_slots(
                container_id,
                slots,
                source_index,
                &mut moving,
                MOUNT_INVENTORY_START,
                player_start,
                false,
                default_item_max_stack_sizes,
            );
        }

        if !changed {
            let player_main_end = player_start + MOUNT_PLAYER_MAIN_SLOT_COUNT;
            let target = if (player_main_end..player_end).contains(&slot_num) {
                Some((player_start, player_main_end))
            } else if (player_start..player_main_end).contains(&slot_num) {
                Some((player_main_end, player_end))
            } else {
                None
            };
            if let Some((start_slot, end_slot)) = target {
                changed = move_item_stack_to_slots(
                    container_id,
                    slots,
                    source_index,
                    &mut moving,
                    start_slot,
                    end_slot,
                    false,
                    default_item_max_stack_sizes,
                );
            }
        }
    }

    if changed {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

fn mount_inventory_quick_move_requires_server_authority(
    slots: &[ContainerSlot],
    slot_num: i16,
    mount_equipment_slots: Option<MountEquipmentSlotVisibility>,
    default_item_equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
    default_mount_body_armor_kinds: &BTreeMap<i32, MountArmorSlotKind>,
) -> bool {
    let Some(player_start) = mount_inventory_player_start_slot(slots) else {
        return false;
    };
    let player_end = player_start + MOUNT_PLAYER_SLOT_COUNT;
    if !(player_start..player_end).contains(&slot_num) {
        return false;
    }
    let Some(source) = slots.iter().find(|slot| slot.slot == slot_num) else {
        return false;
    };
    if item_stack_is_empty(&source.item) {
        return false;
    }
    if default_item_equipment_slots.is_empty() || mount_equipment_slots.is_none() {
        return true;
    }
    if item_stack_has_component_patch(&source.item) {
        return true;
    }

    if let Some(item_id) = source.item.item_id {
        if default_item_equipment_slots
            .get(&item_id)
            .is_some_and(|slot| *slot == ItemEquipmentSlot::Body)
            && !default_mount_body_armor_kinds.contains_key(&item_id)
        {
            return true;
        }
    }

    false
}

fn mount_inventory_player_start_slot(slots: &[ContainerSlot]) -> Option<i16> {
    let slot_count = i16::try_from(slots.len()).ok()?;
    (slot_count >= MOUNT_INVENTORY_START + MOUNT_PLAYER_SLOT_COUNT)
        .then_some(slot_count - MOUNT_PLAYER_SLOT_COUNT)
}

fn mount_equipment_quick_move_target(
    stack: &ProtocolItemStackSummary,
    slots: &[ContainerSlot],
    mount_equipment_slots: Option<MountEquipmentSlotVisibility>,
    default_item_equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
    default_mount_body_armor_kinds: &BTreeMap<i32, MountArmorSlotKind>,
) -> Option<i16> {
    let item_id = stack.item_id?;
    let visibility = mount_equipment_slots?;
    match default_item_equipment_slots.get(&item_id).copied()? {
        ItemEquipmentSlot::Body
            if visibility
                .body
                .zip(default_mount_body_armor_kinds.get(&item_id).copied())
                .is_some_and(|(slot_kind, item_kind)| slot_kind == item_kind)
                && !inventory_menu_slot_has_item(slots, MOUNT_BODY_ARMOR_SLOT) =>
        {
            Some(MOUNT_BODY_ARMOR_SLOT)
        }
        ItemEquipmentSlot::Saddle
            if visibility.saddle && !inventory_menu_slot_has_item(slots, MOUNT_SADDLE_SLOT) =>
        {
            Some(MOUNT_SADDLE_SLOT)
        }
        _ => None,
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

fn anvil_quick_move_requires_server_authority(_slots: &[ContainerSlot], slot_num: i16) -> bool {
    if !(0..ANVIL_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return false;
    }
    if slot_num == ANVIL_RESULT_SLOT {
        return false;
    }
    if matches!(slot_num, ANVIL_INPUT_SLOT | ANVIL_ADDITIONAL_SLOT) {
        return false;
    }
    false
}

fn apply_anvil_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    result_may_pickup: bool,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..ANVIL_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    if slot_num == ANVIL_RESULT_SLOT {
        apply_anvil_result_quick_move_to_slots(
            container_id,
            slots,
            result_may_pickup,
            default_item_max_stack_sizes,
        );
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
        ANVIL_INPUT_SLOT | ANVIL_ADDITIONAL_SLOT => {
            Some((ANVIL_PLAYER_MAIN_START, ANVIL_HOTBAR_END, false))
        }
        slot if (ANVIL_PLAYER_MAIN_START..ANVIL_HOTBAR_END).contains(&slot) => {
            Some((ANVIL_INPUT_SLOT, ANVIL_RESULT_SLOT, false))
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

fn apply_anvil_result_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    result_may_pickup: bool,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !result_may_pickup {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == ANVIL_RESULT_SLOT) else {
        return;
    };
    let Some(input_index) = slots.iter().position(|slot| slot.slot == ANVIL_INPUT_SLOT) else {
        return;
    };
    let Some(additional_index) = slots
        .iter()
        .position(|slot| slot.slot == ANVIL_ADDITIONAL_SLOT)
    else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item)
        || item_stack_is_empty(&slots[input_index].item)
        || slots[input_index].item.count != 1
        || !item_stack_is_empty(&slots[additional_index].item)
    {
        return;
    }

    let mut trial = slots.to_vec();
    let mut moving = slots[source_index].item.clone();
    if !move_item_stack_to_slots(
        container_id,
        &mut trial,
        source_index,
        &mut moving,
        ANVIL_PLAYER_MAIN_START,
        ANVIL_HOTBAR_END,
        true,
        default_item_max_stack_sizes,
    ) || !item_stack_is_empty(&moving)
    {
        return;
    }

    trial[input_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut trial[input_index]);
    trial[source_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut trial[source_index]);
    slots.clone_from_slice(&trial);
}

fn apply_anvil_result_pickup_to_slots(
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    button_num: i8,
    result_may_pickup: bool,
) -> bool {
    if !result_may_pickup || button_num != 0 || !item_stack_is_empty(cursor) {
        return false;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == ANVIL_RESULT_SLOT) else {
        return false;
    };
    let Some(input_index) = slots.iter().position(|slot| slot.slot == ANVIL_INPUT_SLOT) else {
        return false;
    };
    let Some(additional_index) = slots
        .iter()
        .position(|slot| slot.slot == ANVIL_ADDITIONAL_SLOT)
    else {
        return false;
    };
    if item_stack_is_empty(&slots[source_index].item)
        || item_stack_is_empty(&slots[input_index].item)
        || slots[input_index].item.count != 1
        || !item_stack_is_empty(&slots[additional_index].item)
    {
        return false;
    }

    *cursor = slots[source_index].item.clone();
    slots[input_index].item = ProtocolItemStackSummary::empty();
    slots[source_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut slots[input_index]);
    normalize_container_slot_selection(&mut slots[source_index]);
    true
}

fn anvil_result_may_pickup(
    data_values: &[ContainerDataValue],
    abilities: Option<LocalPlayerAbilitiesState>,
    experience: Option<LocalPlayerExperienceState>,
) -> bool {
    let Some(cost) = data_values
        .iter()
        .find_map(|value| (value.id == ANVIL_COST_DATA_ID).then_some(value.value))
    else {
        return false;
    };
    cost > 0
        && (abilities.is_some_and(|abilities| abilities.instabuild)
            || experience.is_some_and(|experience| experience.level >= i32::from(cost)))
}

fn set_container_data_value(data_values: &mut Vec<ContainerDataValue>, id: i16, value: i16) {
    if let Some(existing) = data_values.iter_mut().find(|value| value.id == id) {
        *existing = ContainerDataValue { id, value };
    } else {
        data_values.push(ContainerDataValue { id, value });
    }
    data_values.sort_by_key(|value| value.id);
}

fn beacon_effect_data_value(effect_id: Option<i32>) -> Option<i16> {
    match effect_id {
        Some(effect_id) if effect_id >= 0 => effect_id
            .checked_add(1)
            .and_then(|value| i16::try_from(value).ok()),
        Some(_) => None,
        None => Some(0),
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
    false
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
        slot if !enchantment_lapis_lazuli_item_ids.is_empty()
            && (ENCHANTMENT_PLAYER_MAIN_START..ENCHANTMENT_HOTBAR_END).contains(&slot)
            && item_stack_item_id_in_set(&source_item, enchantment_lapis_lazuli_item_ids) =>
        {
            Some((ENCHANTMENT_LAPIS_SLOT, ENCHANTMENT_PLAYER_MAIN_START, true))
        }
        _ => None,
    };
    let Some((start_slot, end_slot, backwards)) = target else {
        if enchantment_lapis_lazuli_item_ids.is_empty()
            || !(ENCHANTMENT_PLAYER_MAIN_START..ENCHANTMENT_HOTBAR_END).contains(&slot_num)
            || inventory_menu_slot_has_item(slots, ENCHANTMENT_INPUT_SLOT)
        {
            return;
        }
        let Some(input_index) = slots
            .iter()
            .position(|slot| slot.slot == ENCHANTMENT_INPUT_SLOT)
        else {
            return;
        };
        let mut moving = source_item;
        move_stack_count(&mut moving, &mut slots[input_index].item, 1);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
        normalize_container_slot_selection(&mut slots[input_index]);
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
        return false;
    }
    let Some(source_item) = container_slot_item(slots, slot_num) else {
        return false;
    };
    if item_stack_is_empty(source_item) {
        return false;
    }
    if item_stack_has_map_id(source_item) {
        return hashed_component_patch_from_summary(&source_item.component_patch).is_none();
    }
    if cartography_additional_item_ids.is_empty() {
        return true;
    }
    false
}

fn apply_cartography_table_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    cartography_additional_item_ids: &BTreeSet<i32>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    if slot_num == CARTOGRAPHY_TABLE_RESULT_SLOT {
        apply_cartography_table_result_quick_move_to_slots(
            container_id,
            slots,
            default_item_max_stack_sizes,
        );
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
                Some((
                    CARTOGRAPHY_TABLE_MAP_SLOT,
                    CARTOGRAPHY_TABLE_ADDITIONAL_SLOT,
                    false,
                ))
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

fn apply_cartography_table_result_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    let Some(source_index) = slots
        .iter()
        .position(|slot| slot.slot == CARTOGRAPHY_TABLE_RESULT_SLOT)
    else {
        return;
    };
    let Some(map_index) = slots
        .iter()
        .position(|slot| slot.slot == CARTOGRAPHY_TABLE_MAP_SLOT)
    else {
        return;
    };
    let Some(additional_index) = slots
        .iter()
        .position(|slot| slot.slot == CARTOGRAPHY_TABLE_ADDITIONAL_SLOT)
    else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item)
        || item_stack_is_empty(&slots[map_index].item)
        || item_stack_is_empty(&slots[additional_index].item)
        || slots[map_index].item.count != 1
        || slots[additional_index].item.count != 1
    {
        return;
    }

    let mut trial = slots.to_vec();
    let mut moving = slots[source_index].item.clone();
    if !move_item_stack_to_slots(
        container_id,
        &mut trial,
        source_index,
        &mut moving,
        CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
        CARTOGRAPHY_TABLE_HOTBAR_END,
        true,
        default_item_max_stack_sizes,
    ) || !item_stack_is_empty(&moving)
    {
        return;
    }

    trial[map_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut trial[map_index]);
    trial[additional_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut trial[additional_index]);
    trial[source_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut trial[source_index]);
    slots.clone_from_slice(&trial);
}

fn apply_cartography_table_result_pickup_to_slots(
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    button_num: i8,
) -> bool {
    if button_num != 0 || !item_stack_is_empty(cursor) {
        return false;
    }
    let Some(source_index) = slots
        .iter()
        .position(|slot| slot.slot == CARTOGRAPHY_TABLE_RESULT_SLOT)
    else {
        return false;
    };
    let Some(map_index) = slots
        .iter()
        .position(|slot| slot.slot == CARTOGRAPHY_TABLE_MAP_SLOT)
    else {
        return false;
    };
    let Some(additional_index) = slots
        .iter()
        .position(|slot| slot.slot == CARTOGRAPHY_TABLE_ADDITIONAL_SLOT)
    else {
        return false;
    };
    if item_stack_is_empty(&slots[source_index].item)
        || item_stack_is_empty(&slots[map_index].item)
        || item_stack_is_empty(&slots[additional_index].item)
        || slots[map_index].item.count != 1
        || slots[additional_index].item.count != 1
    {
        return false;
    }

    *cursor = slots[source_index].item.clone();
    slots[map_index].item = ProtocolItemStackSummary::empty();
    slots[additional_index].item = ProtocolItemStackSummary::empty();
    slots[source_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut slots[map_index]);
    normalize_container_slot_selection(&mut slots[additional_index]);
    normalize_container_slot_selection(&mut slots[source_index]);
    true
}

fn apply_loom_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    item_tags: Option<&RegistryTagState>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..LOOM_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    if slot_num == LOOM_RESULT_SLOT {
        apply_loom_result_quick_move_to_slots(container_id, slots, default_item_max_stack_sizes);
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

fn apply_loom_result_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    let Some(source_index) = slots.iter().position(|slot| slot.slot == LOOM_RESULT_SLOT) else {
        return;
    };
    let Some(banner_index) = slots.iter().position(|slot| slot.slot == LOOM_BANNER_SLOT) else {
        return;
    };
    let Some(dye_index) = slots.iter().position(|slot| slot.slot == LOOM_DYE_SLOT) else {
        return;
    };
    let Some(pattern_index) = slots.iter().position(|slot| slot.slot == LOOM_PATTERN_SLOT) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item)
        || item_stack_is_empty(&slots[banner_index].item)
        || item_stack_is_empty(&slots[dye_index].item)
        || slots[banner_index].item.count != 1
        || slots[dye_index].item.count != 1
        || !item_stack_is_empty(&slots[pattern_index].item)
    {
        return;
    }

    let mut trial = slots.to_vec();
    let mut moving = slots[source_index].item.clone();
    if !move_item_stack_to_slots(
        container_id,
        &mut trial,
        source_index,
        &mut moving,
        LOOM_PLAYER_MAIN_START,
        LOOM_HOTBAR_END,
        true,
        default_item_max_stack_sizes,
    ) || !item_stack_is_empty(&moving)
    {
        return;
    }

    trial[banner_index].item.count -= 1;
    normalize_item_stack(&mut trial[banner_index].item);
    normalize_container_slot_selection(&mut trial[banner_index]);
    trial[dye_index].item.count -= 1;
    normalize_item_stack(&mut trial[dye_index].item);
    normalize_container_slot_selection(&mut trial[dye_index]);
    trial[source_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut trial[source_index]);
    slots.clone_from_slice(&trial);
}

fn apply_loom_result_pickup_to_slots(
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    button_num: i8,
) -> bool {
    if button_num != 0 || !item_stack_is_empty(cursor) {
        return false;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == LOOM_RESULT_SLOT) else {
        return false;
    };
    let Some(banner_index) = slots.iter().position(|slot| slot.slot == LOOM_BANNER_SLOT) else {
        return false;
    };
    let Some(dye_index) = slots.iter().position(|slot| slot.slot == LOOM_DYE_SLOT) else {
        return false;
    };
    let Some(pattern_index) = slots.iter().position(|slot| slot.slot == LOOM_PATTERN_SLOT) else {
        return false;
    };
    if item_stack_is_empty(&slots[source_index].item)
        || item_stack_is_empty(&slots[banner_index].item)
        || item_stack_is_empty(&slots[dye_index].item)
        || slots[banner_index].item.count != 1
        || slots[dye_index].item.count != 1
        || !item_stack_is_empty(&slots[pattern_index].item)
    {
        return false;
    }

    *cursor = slots[source_index].item.clone();
    slots[banner_index].item = ProtocolItemStackSummary::empty();
    slots[dye_index].item = ProtocolItemStackSummary::empty();
    slots[source_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut slots[banner_index]);
    normalize_container_slot_selection(&mut slots[dye_index]);
    normalize_container_slot_selection(&mut slots[source_index]);
    true
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

fn item_stack_has_component_patch(stack: &ProtocolItemStackSummary) -> bool {
    stack.component_patch != Default::default()
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
