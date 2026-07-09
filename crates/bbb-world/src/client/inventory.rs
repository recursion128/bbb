use bbb_protocol::packets::{
    AttackRangeSummary as ProtocolAttackRangeSummary, ContainerClick as ProtocolContainerClick,
    ContainerClose as ProtocolContainerClose, ContainerInput as ProtocolContainerInput,
    ContainerSetContent as ProtocolContainerSetContent,
    ContainerSetData as ProtocolContainerSetData, ContainerSetSlot as ProtocolContainerSetSlot,
    CraftingRecipeDisplaySummary as ProtocolCraftingRecipeDisplaySummary,
    DataComponentPatchSummary as ProtocolDataComponentPatchSummary,
    EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind,
    HashedComponentPatch as ProtocolHashedComponentPatch,
    HashedItemStack as ProtocolHashedItemStack, HashedStack as ProtocolHashedStack,
    IngredientSummary as ProtocolIngredientSummary, InteractionHand,
    ItemCostSummary as ProtocolItemCostSummary, ItemStackSummary as ProtocolItemStackSummary,
    MerchantOffer as ProtocolMerchantOffer, MerchantOffers as ProtocolMerchantOffers,
    OpenScreen as ProtocolOpenScreen, RecipeDisplayEntry as ProtocolRecipeDisplayEntry,
    SetCursorItem as ProtocolSetCursorItem, SetPlayerInventory as ProtocolSetPlayerInventory,
    SlotDisplaySummary as ProtocolSlotDisplaySummary,
    StonecutterSelectableRecipeSummary as ProtocolStonecutterSelectableRecipeSummary,
    SwingAnimationSummary as ProtocolSwingAnimationSummary,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::entities::{EntityStore, ATTACK_SWING_DURATION};
use crate::item_profiles::{
    clamp_vanilla_item_max_stack_size, ItemProfiles, VANILLA_DEFAULT_MAX_STACK_SIZE,
};
use crate::{
    ClientRecipeBookState, ClientRecipesState, ClientUiState, LocalPlayerAbilitiesState,
    LocalPlayerExperienceState, LocalPlayerState, MountScreenState, RegistrySet, RegistryTagState,
    WorldCounters, WorldStore,
};

mod crafting;
mod grindstone;
mod merchant;
mod slot_ops;
mod smithing;
mod stonecutter;
#[cfg(test)]
mod tests;

use self::slot_ops::*;
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
const VANILLA_DAMAGE_COMPONENT_ID: i32 = 3;
const VANILLA_USE_EFFECTS_COMPONENT_ID: i32 = 5;
const VANILLA_ATTACK_RANGE_COMPONENT_ID: i32 = 30;
const VANILLA_PIERCING_WEAPON_COMPONENT_ID: i32 = 38;
const VANILLA_SWING_ANIMATION_COMPONENT_ID: i32 = 40;
const VANILLA_MAP_ID_COMPONENT_ID: i32 = 46;
const VANILLA_MOB_EFFECT_HASTE_ID: i32 = 2;
const VANILLA_MOB_EFFECT_MINING_FATIGUE_ID: i32 = 3;
const VANILLA_MOB_EFFECT_CONDUIT_POWER_ID: i32 = 28;
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
    /// Styled-run projection of `title` (empty when unknown or plain);
    /// carried alongside the plain text like the packet's `title_styled`.
    #[serde(default)]
    pub title_styled: Vec<bbb_protocol::StyledTextRun>,
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

impl InventoryState {
    pub(crate) fn apply_set_player_inventory(
        &mut self,
        counters: &mut WorldCounters,
        packet: ProtocolSetPlayerInventory,
    ) {
        counters.inventory_slot_updates_received += 1;
        let slot_id = packet.slot;
        let menu_slot = inventory_slot_to_inventory_menu_slot(slot_id);
        let item = packet.item;
        set_inventory_slot(
            &mut self.player_slots,
            InventorySlot {
                slot: slot_id,
                item: item.clone(),
                local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
            },
        );
        if let Some(menu_slot) = menu_slot {
            set_container_slot(
                &mut self.inventory_menu.slots,
                ContainerSlot {
                    slot: menu_slot,
                    item,
                    local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
                },
            );
        }
        self.update_inventory_slot_count(counters);
    }

    pub(crate) fn apply_set_cursor_item(
        &mut self,
        counters: &mut WorldCounters,
        packet: ProtocolSetCursorItem,
    ) {
        counters.cursor_item_updates_received += 1;
        self.cursor_item = packet.item;
    }

    pub(crate) fn apply_open_screen(
        &mut self,
        counters: &mut WorldCounters,
        ui: &mut ClientUiState,
        packet: ProtocolOpenScreen,
    ) {
        counters.container_open_updates_received += 1;
        ui.current_book = None;
        ui.current_advancements_screen = false;
        ui.current_stats_screen = false;
        self.local_inventory_open = false;
        self.local_quick_craft.reset();
        let existing = self
            .open_container
            .take()
            .filter(|container| container.container_id == packet.container_id)
            .unwrap_or_else(|| ContainerState {
                container_id: packet.container_id,
                ..ContainerState::default()
            });
        self.open_container = Some(ContainerState {
            container_id: packet.container_id,
            menu_type_id: Some(packet.menu_type_id),
            title: Some(packet.title),
            title_styled: packet.title_styled,
            mount: None,
            state_id: existing.state_id,
            slots: existing.slots,
            data_values: existing.data_values,
            merchant_offers: existing.merchant_offers,
        });
        self.update_merchant_offer_count(counters);
    }

    pub(crate) fn apply_container_set_content(
        &mut self,
        counters: &mut WorldCounters,
        packet: ProtocolContainerSetContent,
    ) {
        counters.container_content_updates_received += 1;
        let ProtocolContainerSetContent {
            container_id,
            state_id,
            items,
            carried_item,
        } = packet;
        self.cursor_item = carried_item;
        self.local_quick_craft.reset();
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
            let existing = std::mem::replace(&mut self.inventory_menu, default_inventory_menu());
            self.inventory_menu = ContainerState {
                container_id,
                menu_type_id: existing.menu_type_id,
                title: existing.title,
                title_styled: existing.title_styled,
                mount: existing.mount,
                state_id,
                slots,
                data_values: existing.data_values,
                merchant_offers: existing.merchant_offers,
            };
            return;
        }

        let existing = self
            .open_container
            .take()
            .filter(|container| container.container_id == container_id);
        let merchant_offers = existing
            .as_ref()
            .and_then(|container| container.merchant_offers.clone());
        self.open_container = Some(ContainerState {
            container_id,
            menu_type_id: existing
                .as_ref()
                .and_then(|container| container.menu_type_id),
            title: existing
                .as_ref()
                .and_then(|container| container.title.clone()),
            title_styled: existing
                .as_ref()
                .map(|container| container.title_styled.clone())
                .unwrap_or_default(),
            mount: existing.as_ref().and_then(|container| container.mount),
            state_id,
            slots,
            data_values: existing
                .as_ref()
                .map(|container| container.data_values.clone())
                .unwrap_or_default(),
            merchant_offers,
        });
        self.update_merchant_offer_count(counters);
    }

    pub(crate) fn apply_merchant_offers(
        &mut self,
        counters: &mut WorldCounters,
        packet: ProtocolMerchantOffers,
    ) -> bool {
        counters.merchant_offer_packets_received += 1;
        let Some(container) = self.open_container.as_mut().filter(|container| {
            container.container_id == packet.container_id
                && container.menu_type_id == Some(VANILLA_MENU_TYPE_MERCHANT_ID)
        }) else {
            counters.merchant_offer_packets_ignored += 1;
            return false;
        };

        let offer_count = packet.offers.len();
        container.merchant_offers = Some(MerchantOffersState::from_packet(packet));
        counters.merchant_offer_packets_applied += 1;
        counters.merchant_offers_tracked = offer_count;
        true
    }

    pub(crate) fn apply_mount_screen_open_container(
        &mut self,
        counters: &mut WorldCounters,
        ui: &mut ClientUiState,
        mount: MountScreenState,
    ) {
        ui.current_book = None;
        ui.current_advancements_screen = false;
        ui.current_stats_screen = false;
        self.local_inventory_open = false;
        self.local_quick_craft.reset();
        let existing = self
            .open_container
            .take()
            .filter(|container| container.container_id == mount.container_id)
            .unwrap_or_else(|| ContainerState {
                container_id: mount.container_id,
                ..ContainerState::default()
            });
        self.open_container = Some(ContainerState {
            container_id: mount.container_id,
            menu_type_id: None,
            title: existing.title,
            title_styled: existing.title_styled,
            mount: Some(mount),
            state_id: existing.state_id,
            slots: existing.slots,
            data_values: existing.data_values,
            merchant_offers: None,
        });
        self.update_merchant_offer_count(counters);
    }

    pub(crate) fn apply_container_set_slot(
        &mut self,
        counters: &mut WorldCounters,
        packet: ProtocolContainerSetSlot,
    ) {
        counters.container_slot_updates_received += 1;
        let container = if packet.container_id == INVENTORY_MENU_CONTAINER_ID {
            &mut self.inventory_menu
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
        self.update_merchant_offer_count(counters);
    }

    pub(crate) fn apply_container_set_data(
        &mut self,
        counters: &mut WorldCounters,
        packet: ProtocolContainerSetData,
    ) {
        counters.container_data_updates_received += 1;
        let container = self.ensure_container(packet.container_id);
        set_container_data_value(&mut container.data_values, packet.id, packet.value);
        self.update_merchant_offer_count(counters);
    }

    pub(crate) fn apply_container_close(
        &mut self,
        counters: &mut WorldCounters,
        packet: ProtocolContainerClose,
    ) -> bool {
        counters.container_close_updates_received += 1;
        if packet.container_id == INVENTORY_MENU_CONTAINER_ID {
            if self.local_inventory_open {
                self.local_inventory_open = false;
                self.local_quick_craft.reset();
                counters.container_close_updates_applied += 1;
                return true;
            }
            counters.container_close_updates_ignored += 1;
            return false;
        }

        if self
            .open_container
            .as_ref()
            .is_some_and(|container| container.container_id == packet.container_id)
        {
            self.open_container = None;
            self.local_quick_craft.reset();
            counters.merchant_offers_tracked = 0;
            counters.container_close_updates_applied += 1;
            true
        } else {
            counters.container_close_updates_ignored += 1;
            false
        }
    }

    pub(crate) fn close_local_container(
        &mut self,
        counters: &mut WorldCounters,
        container_id: i32,
    ) -> bool {
        if container_id == INVENTORY_MENU_CONTAINER_ID {
            if self.local_inventory_open {
                self.local_inventory_open = false;
                self.local_quick_craft.reset();
                return true;
            }
            return false;
        }

        if self
            .open_container
            .as_ref()
            .is_some_and(|container| container.container_id == container_id)
        {
            self.open_container = None;
            self.local_quick_craft.reset();
            counters.merchant_offers_tracked = 0;
            true
        } else {
            false
        }
    }

    pub(crate) fn open_local_inventory(&mut self, ui: &mut ClientUiState) -> bool {
        if self.open_container.is_some() {
            return false;
        }
        ui.current_book = None;
        ui.current_advancements_screen = false;
        ui.current_stats_screen = false;
        self.sync_inventory_menu_slots_from_player_inventory();
        self.ensure_inventory_menu_slot_shape();
        let was_open = self.local_inventory_open;
        self.local_inventory_open = true;
        self.local_quick_craft.reset();
        !was_open
    }

    pub(crate) fn local_inventory_is_open(&self) -> bool {
        self.local_inventory_open
    }

    pub(crate) fn open_container_id(&self) -> Option<i32> {
        self.open_container
            .as_ref()
            .map(|container| container.container_id)
            .or_else(|| {
                self.local_inventory_open
                    .then_some(INVENTORY_MENU_CONTAINER_ID)
            })
    }

    pub(crate) fn open_container_data_value(&self, id: i16) -> Option<i16> {
        self.open_container
            .as_ref()?
            .data_values
            .iter()
            .find_map(|value| (value.id == id).then_some(value.value))
    }

    pub(crate) fn apply_local_beacon_confirm_effects(
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

    pub(crate) fn open_mount_inventory_kind(
        &self,
        ctx: InventoryCtx<'_>,
    ) -> Option<MountInventoryKind> {
        let mount = self.open_container.as_ref()?.mount?;
        let entity_type_id = ctx.entities.entity_type_id(mount.entity_id)?;
        if crate::entities::is_vanilla_abstract_horse_type(entity_type_id) {
            Some(MountInventoryKind::Horse)
        } else if crate::entities::is_vanilla_abstract_nautilus_type(entity_type_id) {
            Some(MountInventoryKind::Nautilus)
        } else {
            None
        }
    }

    pub(crate) fn open_mount_armor_slot_kind(
        &self,
        ctx: InventoryCtx<'_>,
    ) -> Option<MountArmorSlotKind> {
        self.open_mount_equipment_slot_visibility(ctx)?.body
    }

    pub(crate) fn open_mount_equipment_slot_visibility(
        &self,
        ctx: InventoryCtx<'_>,
    ) -> Option<MountEquipmentSlotVisibility> {
        let mount = self.open_container.as_ref()?.mount?;
        let entity = ctx.entities.get(mount.entity_id)?;
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

    pub(crate) fn apply_local_select_bundle_item(
        &mut self,
        slot_id: i32,
        selected_item_index: i32,
    ) -> bool {
        if selected_item_index < NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX {
            return false;
        }

        if let Some(container) = self.open_container.as_mut() {
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
        if self.local_inventory_open {
            let Ok(slot_id) = i16::try_from(slot_id) else {
                return false;
            };
            let Some(slot) = self
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

    pub(crate) fn local_item_use_prefers_offhand(&self, ctx: InventoryCtx<'_>) -> bool {
        let hotbar_items = self.hotbar_items();
        let selected_slot = usize::from(ctx.local_player.selected_hotbar_slot.min(8));
        !item_stack_is_non_empty(&hotbar_items[selected_slot])
            && self
                .local_offhand_item()
                .is_some_and(item_stack_is_non_empty)
    }

    pub(crate) fn local_item_in_hand_is_non_empty(
        &self,
        ctx: InventoryCtx<'_>,
        hand: InteractionHand,
    ) -> bool {
        self.local_item_in_hand(ctx, hand).is_some()
    }

    pub(crate) fn local_item_in_hand(
        &self,
        ctx: InventoryCtx<'_>,
        hand: InteractionHand,
    ) -> Option<&ProtocolItemStackSummary> {
        match hand {
            InteractionHand::MainHand => {
                let selected_slot = ctx.local_player.selected_hotbar_slot;
                if selected_slot > 8 {
                    return None;
                }
                self.local_player_inventory_item(i32::from(selected_slot))
                    .filter(|item| item_stack_is_non_empty(item))
            }
            InteractionHand::OffHand => self
                .local_offhand_item()
                .filter(|item| item_stack_is_non_empty(item)),
        }
    }

    pub(crate) fn local_selected_main_hand_has_piercing_weapon(
        &self,
        ctx: InventoryCtx<'_>,
    ) -> bool {
        let selected_slot = ctx.local_player.selected_hotbar_slot;
        if selected_slot > 8 {
            return false;
        }

        self.local_player_inventory_item(i32::from(selected_slot))
            .is_some_and(|item| {
                item_stack_has_piercing_weapon(item, &ctx.items.default_piercing_weapon_item_ids)
            })
    }

    pub(crate) fn local_selected_main_hand_attack_range(
        &self,
        ctx: InventoryCtx<'_>,
    ) -> Option<ItemAttackRange> {
        let selected_slot = ctx.local_player.selected_hotbar_slot;
        if selected_slot > 8 {
            return None;
        }

        self.local_player_inventory_item(i32::from(selected_slot))
            .and_then(|item| item_stack_attack_range(item, &ctx.items.default_item_attack_ranges))
    }

    pub(crate) fn local_using_item_use_effects(
        &self,
        ctx: InventoryCtx<'_>,
    ) -> Option<ItemUseEffects> {
        if !ctx.local_player.interaction.using_item {
            return None;
        }

        let item = match ctx.local_player.interaction.using_item_hand {
            Some(InteractionHand::OffHand) => self.local_offhand_item(),
            Some(InteractionHand::MainHand) | None => {
                let selected_slot = ctx.local_player.selected_hotbar_slot;
                if selected_slot > 8 {
                    return None;
                }
                self.local_player_inventory_item(i32::from(selected_slot))
            }
        }?;

        item_stack_use_effects(item, &ctx.items.default_item_use_effects)
    }

    pub(crate) fn local_using_item_item_id(&self, ctx: InventoryCtx<'_>) -> Option<i32> {
        if !ctx.local_player.interaction.using_item {
            return None;
        }
        let hand = ctx
            .local_player
            .interaction
            .using_item_hand
            .unwrap_or(InteractionHand::MainHand);
        self.local_item_in_hand(ctx, hand)
            .and_then(|item| item.item_id.filter(|item_id| *item_id >= 0))
    }

    pub(crate) fn drop_local_selected_hotbar_item(
        &mut self,
        ctx: InventoryCtx<'_>,
        counters: &mut WorldCounters,
        all: bool,
    ) -> bool {
        let selected_slot = ctx.local_player.selected_hotbar_slot;
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
        self.update_inventory_slot_count(counters);
        true
    }

    pub(crate) fn local_player_has_equipped_elytra(&self) -> bool {
        self.player_slots
            .iter()
            .find(|slot| slot.slot == PLAYER_CHEST_EQUIPMENT_SLOT)
            .is_some_and(|slot| {
                slot.item.item_id == Some(VANILLA_ELYTRA_ITEM_ID)
                    && item_stack_is_non_empty(&slot.item)
            })
    }

    pub(crate) fn set_local_merchant_selected_offer(
        &mut self,
        ctx: InventoryCtx<'_>,
        index: i32,
    ) -> bool {
        let Some(container) = self.open_container.as_mut() else {
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
            &ctx.items.default_item_max_stack_sizes,
        );
        true
    }

    pub(crate) fn scroll_local_merchant_offers(&mut self, delta: i32) -> bool {
        let Some(offers) = self
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

    pub(crate) fn build_container_click_slot(
        &self,
        request: ContainerClickSlotRequest,
    ) -> Result<ProtocolContainerClick, ContainerClickBuildError> {
        let Some(container) = self.active_container() else {
            return Err(ContainerClickBuildError::NoOpenContainer);
        };
        if !container_click_slot_is_valid(container, request.slot_num) {
            return Err(ContainerClickBuildError::InvalidSlot(request.slot_num));
        }

        let carried_item = hashed_stack_from_summary(&self.cursor_item)
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

    pub(crate) fn apply_local_container_click_slot(
        &mut self,
        ctx: InventoryCtx<'_>,
        counters: &mut WorldCounters,
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
                    .and_then(|_| self.open_mount_equipment_slot_visibility(ctx)),
                container.merchant_offers.clone(),
            )
        };
        let mut slots_after = slots_before.clone();
        let mut merchant_offers_after = merchant_offers.clone();
        let mut cursor_after = self.cursor_item.clone();
        let mut quick_craft_after = self.local_quick_craft.clone();
        let anvil_result_may_pickup = anvil_result_may_pickup(
            &data_values,
            ctx.local_player.abilities,
            ctx.local_player.experience,
        );
        if menu_result_slot_requires_server_authority(menu_type_id, request.slot_num, request.input)
        {
            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                request.input,
            ));
        }
        if inventory_menu_result_click_requires_server_authority(
            container_id,
            request.slot_num,
            request.input,
            &slots_after,
            ctx.items.default_item_crafting_remainders_known,
            &ctx.items.default_item_crafting_remainders,
            &ctx.items.recipe_specific_crafting_remainder_item_ids,
        ) {
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
                        container_id,
                        &mut slots_after,
                        &mut cursor_after,
                        request.button_num,
                        ctx.items.default_item_crafting_remainders_known,
                        &ctx.items.default_item_crafting_remainders,
                        &ctx.items.recipe_specific_crafting_remainder_item_ids,
                        ctx.local_player.selected_hotbar_slot,
                        &ctx.items.default_item_max_stack_sizes,
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
                        &ctx.items.default_item_max_stack_sizes,
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
                    &ctx.items.default_item_max_stack_sizes,
                ),
                ProtocolContainerInput::Clone => apply_clone_click_to_slots(
                    &slots_after,
                    &mut cursor_after,
                    request.slot_num,
                    ctx.local_player
                        .abilities
                        .is_some_and(|abilities| abilities.instabuild),
                    &ctx.items.default_item_max_stack_sizes,
                ),
                ProtocolContainerInput::QuickMove => {
                    if container_id == INVENTORY_MENU_CONTAINER_ID {
                        if request.slot_num == 0 {
                            apply_inventory_menu_result_quick_move_to_slots(
                                &mut slots_after,
                                &ctx.items.default_item_crafting_remainders,
                                ctx.local_player.selected_hotbar_slot,
                                &ctx.items.default_item_max_stack_sizes,
                            );
                        } else {
                            apply_quick_move_to_slots(
                                container_id,
                                &mut slots_after,
                                request.slot_num,
                                &ctx.items.default_item_equipment_slots,
                                &ctx.items.default_item_max_stack_sizes,
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
                            &ctx.items.default_item_max_stack_sizes,
                        )
                    } else if mount.is_some() {
                        if mount_inventory_quick_move_requires_server_authority(
                            &slots_after,
                            request.slot_num,
                            mount_equipment_slots,
                            &ctx.items.default_item_equipment_slots,
                            &ctx.items.default_mount_body_armor_kinds,
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
                            &ctx.items.default_item_equipment_slots,
                            &ctx.items.default_mount_body_armor_kinds,
                            &ctx.items.default_item_max_stack_sizes,
                        )
                    } else if let Some(container_slot_count) =
                        generic_3x3_container_slot_count(menu_type_id)
                    {
                        apply_generic_container_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            container_slot_count,
                            &ctx.items.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_CRAFTING_ID) {
                        if request.slot_num == CRAFTING_MENU_RESULT_SLOT {
                            if !apply_crafting_menu_result_quick_move_to_slots(
                                container_id,
                                &mut slots_after,
                                ctx.items.default_item_crafting_remainders_known,
                                &ctx.items.default_item_crafting_remainders,
                                &ctx.items.recipe_specific_crafting_remainder_item_ids,
                                ctx.local_player.selected_hotbar_slot,
                                &ctx.items.default_item_max_stack_sizes,
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
                                &ctx.items.default_item_max_stack_sizes,
                            )
                        }
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_CRAFTER_ID) {
                        let disabled_slots = crafter_disabled_slots(&data_values);
                        apply_crafter_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &disabled_slots,
                            &ctx.items.default_item_max_stack_sizes,
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
                            &ctx.items.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_BEACON_ID) {
                        apply_beacon_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            ctx.registries.tags.get("minecraft:item"),
                            &ctx.items.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_ENCHANTMENT_ID) {
                        if enchantment_quick_move_requires_server_authority(
                            &slots_after,
                            request.slot_num,
                            &ctx.items.enchantment_lapis_lazuli_item_ids,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                        apply_enchantment_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &ctx.items.enchantment_lapis_lazuli_item_ids,
                            &ctx.items.default_item_max_stack_sizes,
                        )
                    } else if furnace_family_menu_type(menu_type_id).is_some() {
                        apply_furnace_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            menu_type_id,
                            &ctx.recipes.property_sets,
                            &ctx.items.furnace_fuel_item_ids,
                            &ctx.items.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_BREWING_STAND_ID) {
                        apply_brewing_stand_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            ctx.registries.tags.get("minecraft:item"),
                            &ctx.items.brewing_potion_item_ids,
                            &ctx.items.brewing_ingredient_item_ids,
                            &ctx.items.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_GRINDSTONE_ID) {
                        if grindstone_quick_move_requires_server_authority(
                            &slots_after,
                            request.slot_num,
                            &ctx.items.default_damageable_item_ids,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                        apply_grindstone_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &ctx.items.default_damageable_item_ids,
                            &ctx.items.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_SMITHING_ID) {
                        if smithing_quick_move_requires_server_authority(
                            &slots_after,
                            request.slot_num,
                            &ctx.recipes.property_sets,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                        apply_smithing_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &ctx.recipes.property_sets,
                            &ctx.items.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID) {
                        if cartography_table_quick_move_requires_server_authority(
                            &slots_after,
                            request.slot_num,
                            &ctx.items.cartography_additional_item_ids,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                        apply_cartography_table_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &ctx.items.cartography_additional_item_ids,
                            &ctx.items.default_item_max_stack_sizes,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_LOOM_ID) {
                        apply_loom_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            ctx.registries.tags.get("minecraft:item"),
                            &ctx.items.default_item_max_stack_sizes,
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
                            &ctx.items.default_item_max_stack_sizes,
                        ) {
                            merchant_increment_selected_offer_use(&mut merchant_offers_after);
                        }
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_STONECUTTER_ID) {
                        apply_stonecutter_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &ctx.recipes.stonecutter_recipes,
                            &ctx.items.default_item_max_stack_sizes,
                        )
                    } else if let Some(container_slot_count) =
                        hopper_container_slot_count(menu_type_id)
                    {
                        apply_generic_container_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            container_slot_count,
                            &ctx.items.default_item_max_stack_sizes,
                        )
                    } else if let Some(container_slot_count) =
                        shulker_box_container_slot_count(menu_type_id)
                    {
                        apply_generic_container_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            container_slot_count,
                            &ctx.items.default_item_max_stack_sizes,
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
                        &ctx.items.default_item_max_stack_sizes,
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
                        &ctx.items.default_item_max_stack_sizes,
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
                        &ctx.items.default_item_max_stack_sizes,
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
            if apply_inventory_menu_result_take_side_effects(
                &mut slots_after,
                &ctx.items.default_item_crafting_remainders,
                ctx.local_player.selected_hotbar_slot,
                &ctx.items.default_item_max_stack_sizes,
            )
            .is_none()
            {
                return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                    request.input,
                ));
            }
        }
        if container_id == INVENTORY_MENU_CONTAINER_ID {
            let item_tags = ctx.registries.tags.get("minecraft:item");
            sync_inventory_menu_crafting_result_from_recipe_book(
                &ctx.recipe_book.known,
                item_tags,
                &mut slots_after,
            );
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
        self.cursor_item = cursor_after;
        self.local_quick_craft = quick_craft_after;
        if container_id == INVENTORY_MENU_CONTAINER_ID {
            self.sync_player_inventory_slots_from_inventory_menu(counters);
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
            .player_slots
            .iter()
            .find_map(|slot| (slot.slot == PLAYER_OFFHAND_SLOT).then_some(&slot.item))
        {
            return Some(item);
        }

        self.inventory_menu
            .slots
            .iter()
            .find_map(|slot| (slot.slot == INVENTORY_MENU_OFFHAND_SLOT).then_some(&slot.item))
    }

    pub(crate) fn local_player_has_freeze_immune_wearable(&self, ctx: InventoryCtx<'_>) -> bool {
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
        .any(|item_id| ctx.items.freeze_immune_wearable_item_ids.contains(&item_id))
    }

    pub(crate) fn local_player_can_walk_on_powder_snow(&self, ctx: InventoryCtx<'_>) -> bool {
        self.local_player_inventory_item(PLAYER_FEET_EQUIPMENT_SLOT)
            .filter(|item| item.count > 0)
            .and_then(|item| item.item_id)
            .is_some_and(|item_id| {
                ctx.items
                    .powder_snow_walkable_foot_item_ids
                    .contains(&item_id)
            })
    }

    fn local_player_inventory_item(&self, slot_id: i32) -> Option<&ProtocolItemStackSummary> {
        if let Some(item) = self
            .player_slots
            .iter()
            .find_map(|slot| (slot.slot == slot_id).then_some(&slot.item))
        {
            return Some(item);
        }

        let menu_slot = inventory_slot_to_inventory_menu_slot(slot_id)?;
        self.inventory_menu
            .slots
            .iter()
            .find_map(|slot| (slot.slot == menu_slot).then_some(&slot.item))
    }

    fn player_inventory_slot(&self, slot: i32) -> InventorySlot {
        self.player_slots
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
        set_inventory_slot(&mut self.player_slots, slot.clone());
        if let Some(menu_slot) = menu_slot {
            set_container_slot(
                &mut self.inventory_menu.slots,
                ContainerSlot {
                    slot: menu_slot,
                    item: slot.item,
                    local_selected_bundle_item_index: slot.local_selected_bundle_item_index,
                },
            );
        }
    }

    fn active_container(&self) -> Option<&ContainerState> {
        self.open_container
            .as_ref()
            .or_else(|| self.local_inventory_open.then_some(&self.inventory_menu))
    }

    fn active_container_mut(&mut self) -> Option<&mut ContainerState> {
        if self.open_container.is_some() {
            return self.open_container.as_mut();
        }
        self.local_inventory_open
            .then_some(&mut self.inventory_menu)
    }

    fn ensure_inventory_menu_slot_shape(&mut self) {
        for slot in 0..=INVENTORY_MENU_OFFHAND_SLOT {
            if self
                .inventory_menu
                .slots
                .iter()
                .all(|existing| existing.slot != slot)
            {
                set_container_slot(
                    &mut self.inventory_menu.slots,
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
        for slot in self.player_slots.clone() {
            let Some(menu_slot) = inventory_slot_to_inventory_menu_slot(slot.slot) else {
                continue;
            };
            if self
                .inventory_menu
                .slots
                .iter()
                .any(|slot| slot.slot == menu_slot)
            {
                continue;
            }
            set_container_slot(
                &mut self.inventory_menu.slots,
                ContainerSlot {
                    slot: menu_slot,
                    item: slot.item,
                    local_selected_bundle_item_index: slot.local_selected_bundle_item_index,
                },
            );
        }
    }

    fn sync_player_inventory_slots_from_inventory_menu(&mut self, counters: &mut WorldCounters) {
        for slot in self.inventory_menu.slots.clone() {
            let Some(player_slot) = inventory_menu_slot_to_inventory_slot(slot.slot) else {
                continue;
            };
            set_inventory_slot(
                &mut self.player_slots,
                InventorySlot {
                    slot: player_slot,
                    item: slot.item,
                    local_selected_bundle_item_index: slot.local_selected_bundle_item_index,
                },
            );
        }
        self.update_inventory_slot_count(counters);
    }

    fn inventory_menu_container(&mut self) -> &mut ContainerState {
        if self.inventory_menu.container_id != INVENTORY_MENU_CONTAINER_ID {
            self.inventory_menu = default_inventory_menu();
        }
        &mut self.inventory_menu
    }

    fn ensure_container(&mut self, container_id: i32) -> &mut ContainerState {
        if container_id == INVENTORY_MENU_CONTAINER_ID {
            return self.inventory_menu_container();
        }

        if self
            .open_container
            .as_ref()
            .is_none_or(|container| container.container_id != container_id)
        {
            self.open_container = Some(ContainerState {
                container_id,
                ..ContainerState::default()
            })
        }
        self.open_container
            .as_mut()
            .expect("container was initialized")
    }

    fn update_inventory_slot_count(&mut self, counters: &mut WorldCounters) {
        counters.inventory_slots_tracked = self.player_slots.len();
    }

    fn update_merchant_offer_count(&mut self, counters: &mut WorldCounters) {
        counters.merchant_offers_tracked = self
            .open_container
            .as_ref()
            .and_then(|container| container.merchant_offers.as_ref())
            .map(|offers| offers.offers.len())
            .unwrap_or(0);
    }
}

/// Read-only cross-domain state borrowed from the `WorldStore` facade for a
/// single inventory call. Built field-by-field at the delegation site so the
/// shared borrows split cleanly against `&mut InventoryState`.
#[derive(Clone, Copy)]
pub(crate) struct InventoryCtx<'a> {
    pub(crate) items: &'a ItemProfiles,
    pub(crate) local_player: &'a LocalPlayerState,
    pub(crate) entities: &'a EntityStore,
    pub(crate) recipes: &'a ClientRecipesState,
    pub(crate) recipe_book: &'a ClientRecipeBookState,
    pub(crate) registries: &'a RegistrySet,
}

macro_rules! inventory_ctx {
    ($store:expr) => {
        InventoryCtx {
            items: &$store.items,
            local_player: &$store.local_player,
            entities: &$store.entities,
            recipes: &$store.recipes,
            recipe_book: &$store.recipe_book,
            registries: &$store.registries,
        }
    };
}

/// Facade delegation: the inventory method group lives on
/// [`InventoryState`] (and the item default tables on `ItemProfiles`);
/// `WorldStore` keeps the public signatures and forwards, splitting off
/// cross-domain borrows explicitly.
impl WorldStore {
    pub fn apply_set_player_inventory(&mut self, packet: ProtocolSetPlayerInventory) {
        self.inventory
            .apply_set_player_inventory(&mut self.counters, packet)
    }

    pub fn apply_set_cursor_item(&mut self, packet: ProtocolSetCursorItem) {
        self.inventory
            .apply_set_cursor_item(&mut self.counters, packet)
    }

    pub fn apply_open_screen(&mut self, packet: ProtocolOpenScreen) {
        self.inventory
            .apply_open_screen(&mut self.counters, &mut self.client_ui, packet)
    }

    pub fn apply_container_set_content(&mut self, packet: ProtocolContainerSetContent) {
        self.inventory
            .apply_container_set_content(&mut self.counters, packet)
    }

    pub fn apply_merchant_offers(&mut self, packet: ProtocolMerchantOffers) -> bool {
        self.inventory
            .apply_merchant_offers(&mut self.counters, packet)
    }

    pub(crate) fn apply_mount_screen_open_container(&mut self, mount: MountScreenState) {
        self.inventory.apply_mount_screen_open_container(
            &mut self.counters,
            &mut self.client_ui,
            mount,
        )
    }

    pub fn apply_container_set_slot(&mut self, packet: ProtocolContainerSetSlot) {
        self.inventory
            .apply_container_set_slot(&mut self.counters, packet)
    }

    pub fn apply_container_set_data(&mut self, packet: ProtocolContainerSetData) {
        self.inventory
            .apply_container_set_data(&mut self.counters, packet)
    }

    pub fn apply_container_close(&mut self, packet: ProtocolContainerClose) -> bool {
        self.inventory
            .apply_container_close(&mut self.counters, packet)
    }

    pub fn close_local_container(&mut self, container_id: i32) -> bool {
        self.inventory
            .close_local_container(&mut self.counters, container_id)
    }

    pub fn open_local_inventory(&mut self) -> bool {
        self.inventory.open_local_inventory(&mut self.client_ui)
    }

    pub fn local_inventory_is_open(&self) -> bool {
        self.inventory.local_inventory_is_open()
    }

    pub fn open_container_id(&self) -> Option<i32> {
        self.inventory.open_container_id()
    }

    pub fn open_container_data_value(&self, id: i16) -> Option<i16> {
        self.inventory.open_container_data_value(id)
    }

    pub fn apply_local_beacon_confirm_effects(
        &mut self,
        primary_effect: i32,
        secondary_effect: Option<i32>,
    ) -> bool {
        self.inventory
            .apply_local_beacon_confirm_effects(primary_effect, secondary_effect)
    }

    pub fn open_mount_inventory_kind(&self) -> Option<MountInventoryKind> {
        self.inventory
            .open_mount_inventory_kind(inventory_ctx!(self))
    }

    pub fn open_mount_armor_slot_kind(&self) -> Option<MountArmorSlotKind> {
        self.inventory
            .open_mount_armor_slot_kind(inventory_ctx!(self))
    }

    pub fn open_mount_equipment_slot_visibility(&self) -> Option<MountEquipmentSlotVisibility> {
        self.inventory
            .open_mount_equipment_slot_visibility(inventory_ctx!(self))
    }

    pub fn apply_local_select_bundle_item(
        &mut self,
        slot_id: i32,
        selected_item_index: i32,
    ) -> bool {
        self.inventory
            .apply_local_select_bundle_item(slot_id, selected_item_index)
    }

    pub fn inventory(&self) -> &InventoryState {
        &self.inventory
    }

    pub fn local_item_use_prefers_offhand(&self) -> bool {
        self.inventory
            .local_item_use_prefers_offhand(inventory_ctx!(self))
    }

    pub(crate) fn local_item_in_hand_is_non_empty(&self, hand: InteractionHand) -> bool {
        self.inventory
            .local_item_in_hand_is_non_empty(inventory_ctx!(self), hand)
    }

    /// The non-empty stack currently held in the requested local-player hand.
    ///
    /// Main hand follows the selected hotbar slot, while off hand follows the
    /// player-inventory offhand slot, matching vanilla `LocalPlayer.getItemInHand`.
    pub fn local_item_in_hand(&self, hand: InteractionHand) -> Option<&ProtocolItemStackSummary> {
        self.inventory
            .local_item_in_hand(inventory_ctx!(self), hand)
    }

    pub fn local_selected_main_hand_has_piercing_weapon(&self) -> bool {
        self.inventory
            .local_selected_main_hand_has_piercing_weapon(inventory_ctx!(self))
    }

    pub fn local_selected_main_hand_attack_range(&self) -> Option<ItemAttackRange> {
        self.inventory
            .local_selected_main_hand_attack_range(inventory_ctx!(self))
    }

    pub(crate) fn entity_held_item_swing_duration(&self, id: i32, off_hand: bool) -> i32 {
        self.held_item(id, off_hand)
            .as_ref()
            .map(|item| {
                self.entity_swing_duration_with_effects(
                    id,
                    item_stack_swing_duration(
                        item,
                        &self.items.default_item_swing_animation_durations,
                    ),
                )
            })
            .unwrap_or(ATTACK_SWING_DURATION)
    }

    pub(crate) fn refresh_entity_active_swing_duration(&mut self, id: i32) {
        let Some(off_hand) = self.entities.active_swing_off_hand(id) else {
            return;
        };
        let duration = self.entity_held_item_swing_duration(id, off_hand);
        let _ = self
            .entities
            .refresh_client_animation_swing_duration(id, duration);
    }

    pub(crate) fn local_using_item_use_effects(&self) -> Option<ItemUseEffects> {
        self.inventory
            .local_using_item_use_effects(inventory_ctx!(self))
    }

    pub fn local_using_item_item_id(&self) -> Option<i32> {
        self.inventory
            .local_using_item_item_id(inventory_ctx!(self))
    }

    pub fn drop_local_selected_hotbar_item(&mut self, all: bool) -> bool {
        self.inventory.drop_local_selected_hotbar_item(
            inventory_ctx!(self),
            &mut self.counters,
            all,
        )
    }

    pub fn local_player_has_equipped_elytra(&self) -> bool {
        self.inventory.local_player_has_equipped_elytra()
    }

    pub fn set_default_item_max_stack_sizes(&mut self, max_stack_sizes: BTreeMap<i32, i32>) {
        self.items.set_default_item_max_stack_sizes(max_stack_sizes)
    }

    pub fn set_default_item_max_damage(&mut self, max_damage: BTreeMap<i32, i32>) {
        self.items.set_default_item_max_damage(max_damage)
    }

    pub fn set_default_item_crafting_remainders(&mut self, remainders: BTreeMap<i32, i32>) {
        self.items.set_default_item_crafting_remainders(remainders)
    }

    pub fn set_recipe_specific_crafting_remainder_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items
            .set_recipe_specific_crafting_remainder_item_ids(item_ids)
    }

    pub fn item_max_stack_size_for_protocol_id(&self, item_id: i32) -> i32 {
        self.items.item_max_stack_size_for_protocol_id(item_id)
    }

    pub fn item_max_damage_for_protocol_id(&self, item_id: i32) -> Option<i32> {
        self.items.item_max_damage_for_protocol_id(item_id)
    }

    pub fn set_local_merchant_selected_offer(&mut self, index: i32) -> bool {
        self.inventory
            .set_local_merchant_selected_offer(inventory_ctx!(self), index)
    }

    pub fn scroll_local_merchant_offers(&mut self, delta: i32) -> bool {
        self.inventory.scroll_local_merchant_offers(delta)
    }

    pub fn set_furnace_fuel_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items.set_furnace_fuel_item_ids(item_ids)
    }

    pub fn set_brewing_potion_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items.set_brewing_potion_item_ids(item_ids)
    }

    pub fn set_brewing_ingredient_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items.set_brewing_ingredient_item_ids(item_ids)
    }

    pub fn set_enchantment_lapis_lazuli_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items.set_enchantment_lapis_lazuli_item_ids(item_ids)
    }

    pub fn set_cartography_additional_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items.set_cartography_additional_item_ids(item_ids)
    }

    pub fn set_default_damageable_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items.set_default_damageable_item_ids(item_ids)
    }

    pub fn set_freeze_immune_wearable_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items.set_freeze_immune_wearable_item_ids(item_ids)
    }

    pub fn set_powder_snow_walkable_foot_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items.set_powder_snow_walkable_foot_item_ids(item_ids)
    }

    pub fn set_default_piercing_weapon_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items.set_default_piercing_weapon_item_ids(item_ids)
    }

    pub fn set_default_item_attack_ranges(
        &mut self,
        attack_ranges: BTreeMap<i32, ItemAttackRange>,
    ) {
        self.items.set_default_item_attack_ranges(attack_ranges)
    }

    pub fn set_default_item_swing_animation_durations(&mut self, durations: BTreeMap<i32, i32>) {
        self.items
            .set_default_item_swing_animation_durations(durations)
    }

    fn entity_swing_duration_with_effects(&self, entity_id: i32, duration: i32) -> i32 {
        if let Some(amplifier) = self.entity_dig_speed_amplifier(entity_id) {
            return duration.saturating_sub(amplifier.saturating_add(1));
        }
        if let Some(amplifier) =
            self.entity_mob_effect_amplifier(entity_id, VANILLA_MOB_EFFECT_MINING_FATIGUE_ID)
        {
            return duration.saturating_add(amplifier.saturating_add(1).saturating_mul(2));
        }
        duration
    }

    fn entity_mob_effect_amplifier(&self, entity_id: i32, effect_id: i32) -> Option<i32> {
        self.entity_effect(entity_id, effect_id)
            .map(|effect| effect.amplifier)
    }

    fn entity_dig_speed_amplifier(&self, entity_id: i32) -> Option<i32> {
        [
            VANILLA_MOB_EFFECT_HASTE_ID,
            VANILLA_MOB_EFFECT_CONDUIT_POWER_ID,
        ]
        .into_iter()
        .filter_map(|effect_id| self.entity_mob_effect_amplifier(entity_id, effect_id))
        .max()
    }

    pub fn set_default_item_use_effects(&mut self, use_effects: BTreeMap<i32, ItemUseEffects>) {
        self.items.set_default_item_use_effects(use_effects)
    }

    pub fn set_default_item_equipment_slots(
        &mut self,
        equipment_slots: BTreeMap<i32, ItemEquipmentSlot>,
    ) {
        self.items.set_default_item_equipment_slots(equipment_slots)
    }

    /// Installs the item id → humanoid armor material table (from the item registry), used to project
    /// worn armor onto entity render sources for the `HumanoidArmorLayer` overlay.
    pub fn set_item_armor_materials(
        &mut self,
        armor_materials: BTreeMap<i32, crate::entities::ArmorMaterialKind>,
    ) {
        self.items.set_item_armor_materials(armor_materials)
    }

    pub fn set_default_mount_body_armor_kinds(
        &mut self,
        armor_kinds: BTreeMap<i32, MountArmorSlotKind>,
    ) {
        self.items.set_default_mount_body_armor_kinds(armor_kinds)
    }

    pub fn set_default_llama_body_decor_colors(
        &mut self,
        decor_colors: BTreeMap<i32, crate::entities::LlamaBodyDecorColor>,
    ) {
        self.items.set_default_llama_body_decor_colors(decor_colors)
    }

    pub fn set_default_nautilus_body_armor_materials(
        &mut self,
        armor_materials: BTreeMap<i32, crate::entities::ArmorMaterialKind>,
    ) {
        self.items
            .set_default_nautilus_body_armor_materials(armor_materials)
    }

    pub fn set_default_horse_body_armor_materials(
        &mut self,
        armor_materials: BTreeMap<i32, crate::entities::ArmorMaterialKind>,
    ) {
        self.items
            .set_default_horse_body_armor_materials(armor_materials)
    }

    pub fn set_default_wolf_body_armor_materials(
        &mut self,
        armor_materials: BTreeMap<i32, crate::entities::ArmorMaterialKind>,
    ) {
        self.items
            .set_default_wolf_body_armor_materials(armor_materials)
    }

    pub fn build_container_click_slot(
        &self,
        request: ContainerClickSlotRequest,
    ) -> Result<ProtocolContainerClick, ContainerClickBuildError> {
        self.inventory.build_container_click_slot(request)
    }

    pub fn apply_local_container_click_slot(
        &mut self,
        request: ContainerClickSlotRequest,
    ) -> Result<ProtocolContainerClick, ContainerClickBuildError> {
        self.inventory.apply_local_container_click_slot(
            inventory_ctx!(self),
            &mut self.counters,
            request,
        )
    }

    pub(crate) fn local_player_has_freeze_immune_wearable(&self) -> bool {
        self.inventory
            .local_player_has_freeze_immune_wearable(inventory_ctx!(self))
    }

    pub(crate) fn local_player_can_walk_on_powder_snow(&self) -> bool {
        self.inventory
            .local_player_can_walk_on_powder_snow(inventory_ctx!(self))
    }
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
