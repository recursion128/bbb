use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID, VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
    VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
    VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID, VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID, VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
    VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID, VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
};
use bbb_protocol::equipment_slots::{
    vanilla_equipment_slot_by_id, EquipmentSlot, EquipmentSlotCatalogEntry,
    VANILLA_EQUIPMENT_SLOTS, VANILLA_PLAYER_INVENTORY_BODY_SLOT,
    VANILLA_PLAYER_INVENTORY_CHEST_SLOT, VANILLA_PLAYER_INVENTORY_FEET_SLOT,
    VANILLA_PLAYER_INVENTORY_HEAD_SLOT, VANILLA_PLAYER_INVENTORY_LEGS_SLOT,
    VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT, VANILLA_PLAYER_INVENTORY_SADDLE_SLOT,
};
use bbb_protocol::item_types::{
    vanilla_item_type, VanillaItemUseOverride, VANILLA_ITEM_BARRIER_ID, VANILLA_ITEM_BUCKET_ID,
    VANILLA_ITEM_COMPASS_ID, VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE, VANILLA_ITEM_LIGHT_ID,
};
use bbb_protocol::menu_types::{
    VANILLA_MENU_TYPE_ANVIL_ID, VANILLA_MENU_TYPE_BEACON_ID, VANILLA_MENU_TYPE_BLAST_FURNACE_ID,
    VANILLA_MENU_TYPE_BREWING_STAND_ID, VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
    VANILLA_MENU_TYPE_CRAFTER_3X3_ID, VANILLA_MENU_TYPE_CRAFTING_ID,
    VANILLA_MENU_TYPE_ENCHANTMENT_ID, VANILLA_MENU_TYPE_FURNACE_ID,
    VANILLA_MENU_TYPE_GENERIC_3X3_ID, VANILLA_MENU_TYPE_GENERIC_9X1_ID,
    VANILLA_MENU_TYPE_GENERIC_9X6_ID, VANILLA_MENU_TYPE_GRINDSTONE_ID, VANILLA_MENU_TYPE_HOPPER_ID,
    VANILLA_MENU_TYPE_LECTERN_ID, VANILLA_MENU_TYPE_LOOM_ID, VANILLA_MENU_TYPE_MERCHANT_ID,
    VANILLA_MENU_TYPE_SHULKER_BOX_ID, VANILLA_MENU_TYPE_SMITHING_ID, VANILLA_MENU_TYPE_SMOKER_ID,
    VANILLA_MENU_TYPE_STONECUTTER_ID,
};
use bbb_protocol::mob_effect_types::{
    VANILLA_MOB_EFFECT_CONDUIT_POWER_ID, VANILLA_MOB_EFFECT_HASTE_ID,
    VANILLA_MOB_EFFECT_MINING_FATIGUE_ID,
};
pub use bbb_protocol::packets::AttackRangeSummary as ItemAttackRange;
#[cfg(test)]
use bbb_protocol::packets::{
    AttackRangeSummary as ProtocolAttackRangeSummary, HashedItemStack as ProtocolHashedItemStack,
};
use bbb_protocol::packets::{
    ContainerClick as ProtocolContainerClick, ContainerClose as ProtocolContainerClose,
    ContainerInput as ProtocolContainerInput, ContainerSetContent as ProtocolContainerSetContent,
    ContainerSetData as ProtocolContainerSetData, ContainerSetSlot as ProtocolContainerSetSlot,
    DataComponentPatchSummary as ProtocolDataComponentPatchSummary,
    EntityDataValue as ProtocolEntityDataValue,
    HashedComponentPatch as ProtocolHashedComponentPatch, HashedStack as ProtocolHashedStack,
    InteractionHand, ItemCostSummary as ProtocolItemCostSummary,
    ItemStackSummary as ProtocolItemStackSummary, MerchantOffer as ProtocolMerchantOffer,
    MerchantOffers as ProtocolMerchantOffers, OpenScreen as ProtocolOpenScreen,
    SetCursorItem as ProtocolSetCursorItem, SetPlayerInventory as ProtocolSetPlayerInventory,
    StonecutterSelectableRecipeSummary as ProtocolStonecutterSelectableRecipeSummary,
    SwingAnimationSummary as ProtocolSwingAnimationSummary,
};
use bbb_protocol::RichTextComponent;
#[cfg(test)]
use bbb_registry_codec::item_hash::{crc32c, hash_ops_crc32c_int};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::entities::{
    vanilla_entity_metadata_abstract_horse_accessors_for_entity_type,
    vanilla_entity_metadata_baby_accessor_for_entity_type,
    vanilla_entity_metadata_tamable_animal_accessors_for_entity_type, EntityStore,
    ATTACK_SWING_DURATION,
};
use crate::item_profiles::ItemProfiles;
use crate::{
    ClientRecipesState, ClientUiState, LocalPlayerAbilitiesState, LocalPlayerExperienceState,
    LocalPlayerState, MountScreenState, RegistrySet, RegistryTagState, WorldCounters, WorldStore,
    BANNER_PATTERN_REGISTRY,
};

mod world_defaults;
mod world_facade;

mod local_item_mutation;
mod menu_state;
mod slot_state;
pub use slot_state::ContainerSlotRef;

mod container_button;
mod crafting;
mod grindstone;
mod merchant;
mod slot_ops;
mod smithing;
mod stonecutter;
mod use_prediction;
pub use use_prediction::{
    LocalInteractionParticleAction, LocalInteractionResult, LocalInteractionSoundAction,
    LocalInteractionSuccess, LocalInteractionSwingSource, LocalItemUsePovFluidMode,
    LocalItemUsePovQuery, LocalItemUsePovResult, LocalItemUsePrediction, LocalLeverAttachFace,
    LocalTargetUsePrediction, LocalUsingItemTickActions,
};
#[cfg(test)]
mod tests;

use self::slot_ops::*;
use self::{
    crafting::{
        apply_crafter_menu_quick_move_to_slots, apply_crafting_menu_quick_move_to_slots,
        apply_crafting_menu_result_pickup_to_slots, apply_crafting_menu_result_quick_move_to_slots,
        apply_crafting_menu_result_take_to_slots, crafter_disabled_slots, CraftingResultTakeTarget,
    },
    grindstone::{
        apply_grindstone_menu_quick_move_to_slots, apply_grindstone_result_to_slots,
        grindstone_quick_move_requires_server_authority,
    },
    merchant::{
        apply_merchant_menu_quick_move_to_slots, apply_merchant_result_to_slots,
        apply_merchant_selected_offer_payment_autofill, merchant_increment_selected_offer_use,
        merchant_max_scroll_offset, merchant_selected_offer_index,
    },
    smithing::{
        apply_smithing_menu_quick_move_to_slots, apply_smithing_result_to_slots,
        smithing_input_slot_accepts_stack, smithing_quick_move_requires_server_authority,
    },
    stonecutter::{apply_stonecutter_menu_quick_move_to_slots, apply_stonecutter_result_to_slots},
};

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
const LOOM_NO_ITEM_REQUIRED_PATTERN_TAG: &str = "minecraft:no_item_required";
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
const FURNACE_INPUT_SLOT: i16 = 0;
const FURNACE_FUEL_SLOT: i16 = 1;
const FURNACE_RESULT_SLOT: i16 = 2;
const FURNACE_CONTAINER_SLOT_COUNT: i16 = 3;
const HOPPER_CONTAINER_SLOT_COUNT: i16 = 5;
const SHULKER_BOX_CONTAINER_SLOT_COUNT: i16 = 27;
const GENERIC_CONTAINER_PLAYER_SLOT_COUNT: i16 = 36;
const PLAYER_HOTBAR_SIZE: usize = 9;
/// `InventoryMenu.CONTAINER_ID` (InventoryMenu.java:14): the player inventory
/// is the one menu that is always open.
pub const INVENTORY_MENU_CONTAINER_ID: i32 = 0;
const INVENTORY_MENU_MAIN_START: i16 = 9;
const INVENTORY_MENU_MAIN_END: i16 = 36;
const INVENTORY_MENU_HOTBAR_START: i16 = 36;
const INVENTORY_MENU_HOTBAR_END: i16 = 45;
const INVENTORY_MENU_OFFHAND_SLOT: i16 = 45;
const NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX: i32 = -1;
const QUICKCRAFT_TYPE_CHARITABLE: i8 = 0;
const QUICKCRAFT_TYPE_GREEDY: i8 = 1;
const QUICKCRAFT_TYPE_CLONE: i8 = 2;
const QUICKCRAFT_HEADER_START: i8 = 0;
const QUICKCRAFT_HEADER_CONTINUE: i8 = 1;
const QUICKCRAFT_HEADER_END: i8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventorySlot<SlotId = i32, Item = ProtocolItemStackSummary> {
    pub slot: SlotId,
    pub item: Item,
    #[serde(default = "default_local_selected_bundle_item_index")]
    pub local_selected_bundle_item_index: i32,
}

pub type ContainerSlot = InventorySlot<i16>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotbarItemState<'a> {
    pub item: &'a ProtocolItemStackSummary,
    pub local_selected_bundle_item_index: i32,
}

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

impl HotbarItemState<'_> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalCrafterSlotStateChange {
    pub container_id: i32,
    pub slot_id: i16,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ContainerState {
    pub container_id: i32,
    pub menu_type_id: Option<i32>,
    /// Exact trusted title supplied by `ClientboundOpenScreenPacket`.
    pub title: Option<RichTextComponent>,
    #[serde(default)]
    pub mount: Option<MountScreenState>,
    pub state_id: i32,
    pub(crate) slots: Vec<ContainerSlot>,
    pub data_values: Vec<ContainerDataValue>,
    pub merchant_offers: Option<MerchantOffersState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecipeUpdateListenerContext {
    pub(crate) container_id: i32,
    pub(crate) layout: RecipeUpdateListenerLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecipeUpdateListenerLayout {
    Crafting { grid_width: i32, grid_height: i32 },
    Furnace { fuel_slot_empty: bool },
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

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(from = "slot_state::InventoryWire")]
pub struct InventoryState {
    pub player_slots: Vec<InventorySlot>,
    pub cursor_item: ProtocolItemStackSummary,
    pub inventory_menu: ContainerState,
    pub local_inventory_open: bool,
    pub open_container: Option<ContainerState>,
    /// Monotonic identity for renderer/input projections of the current
    /// server-opened container.
    pub open_container_revision: u64,
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
            open_container_revision: 0,
            local_quick_craft: LocalQuickCraftState::default(),
        }
    }
}

impl InventoryState {
    /// `AbstractContainerMenu.getStateId` as the client last heard it, which
    /// is what a click packet echoes back so the server can tell whether the
    /// two sides still agree.
    pub fn inventory_menu_state_id(&self) -> i32 {
        self.inventory_menu.state_id
    }

    /// One `InventoryMenu` slot as the client currently shows it.
    pub fn inventory_menu_slot_item(&self, slot: i16) -> Option<&ProtocolItemStackSummary> {
        self.container_slot(&self.inventory_menu, slot)
            .map(|row| row.item)
    }

    pub fn hotbar_items(&self) -> [ProtocolItemStackSummary; PLAYER_HOTBAR_SIZE] {
        self.hotbar_item_states().map(|state| {
            state.map_or_else(ProtocolItemStackSummary::empty, |state| state.item.clone())
        })
    }

    pub(crate) fn selected_hotbar_item(&self, slot: u8) -> Option<&ProtocolItemStackSummary> {
        self.player_slot(slot.min(8).into()).map(|row| &row.item)
    }

    pub fn hotbar_item_states(&self) -> [Option<HotbarItemState<'_>>; PLAYER_HOTBAR_SIZE] {
        let mut items = [None; PLAYER_HOTBAR_SIZE];
        for slot in &self.player_slots {
            let Ok(slot_index) = usize::try_from(slot.slot) else {
                continue;
            };
            if slot_index < PLAYER_HOTBAR_SIZE {
                items[slot_index] = Some(HotbarItemState {
                    item: &slot.item,
                    local_selected_bundle_item_index: slot.local_selected_bundle_item_index,
                });
            }
        }

        items
    }

    /// The `Inventory` offhand stack `Gui.extractItemHotbar` reads
    /// (`Player.getOffhandItem()`), in the same shape as a hotbar slot.
    pub fn offhand_item_state(&self) -> Option<HotbarItemState<'_>> {
        self.local_offhand_slot()
            .map(|(item, local_selected_bundle_item_index)| HotbarItemState {
                item,
                local_selected_bundle_item_index,
            })
    }
}

impl InventoryState {
    pub(crate) fn apply_set_player_inventory(
        &mut self,
        counters: &mut WorldCounters,
        packet: ProtocolSetPlayerInventory,
    ) -> bool {
        counters.inventory_slot_updates_received += 1;
        let slot_id = packet.slot;
        if !(0..=VANILLA_PLAYER_INVENTORY_SADDLE_SLOT).contains(&slot_id) {
            return false;
        }
        self.set_player_slot(InventorySlot::new(slot_id, packet.item));
        self.update_inventory_slot_count(counters);
        true
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
        ui.replace_screen();
        ui.push_screen_request(crate::ScreenRequest::Container);
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
            mount: None,
            state_id: existing.state_id,
            slots: existing.slots,
            data_values: existing.data_values,
            merchant_offers: existing.merchant_offers,
        });
        let slots = std::mem::take(&mut self.open_container.as_mut().unwrap().slots);
        self.replace_container_slots(packet.container_id, slots);
        self.advance_open_container_revision();
        self.update_merchant_offer_count(counters);
    }

    pub(crate) fn apply_container_set_content(
        &mut self,
        counters: &mut WorldCounters,
        packet: ProtocolContainerSetContent,
    ) -> bool {
        counters.container_content_updates_received += 1;
        let ProtocolContainerSetContent {
            container_id,
            state_id,
            items,
            carried_item,
        } = packet;
        if container_id != INVENTORY_MENU_CONTAINER_ID
            && self
                .open_container
                .as_ref()
                .is_none_or(|container| container.container_id != container_id)
        {
            return false;
        }
        self.cursor_item = carried_item;
        self.local_quick_craft.reset();
        let max_slots = slot_state::container_slot_count(self.container(container_id).unwrap());
        let slots = items
            .into_iter()
            .take(usize::try_from(max_slots).unwrap_or(0))
            .enumerate()
            .map(|(slot, item)| ContainerSlot::new(slot as i16, item));
        self.replace_container_slots(container_id, slots);
        self.container_mut(container_id).state_id = state_id;
        self.update_inventory_slot_count(counters);
        if container_id == INVENTORY_MENU_CONTAINER_ID {
            return true;
        }

        self.advance_open_container_revision();
        self.update_merchant_offer_count(counters);
        true
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

    pub(crate) fn apply_container_set_slot(
        &mut self,
        counters: &mut WorldCounters,
        packet: ProtocolContainerSetSlot,
    ) -> bool {
        counters.container_slot_updates_received += 1;
        let container_id = packet.container_id;
        let Some(container) = self.container(container_id) else {
            return false;
        };
        let slot_count = slot_state::container_slot_count(container);
        if packet.slot < 0 || packet.slot >= slot_count {
            return false;
        }
        self.container_mut(container_id).state_id = packet.state_id;
        self.set_menu_slot(container_id, ContainerSlot::new(packet.slot, packet.item));
        self.update_inventory_slot_count(counters);
        if container_id != INVENTORY_MENU_CONTAINER_ID {
            self.advance_open_container_revision();
        }
        self.update_merchant_offer_count(counters);
        true
    }

    pub(crate) fn apply_container_set_data(
        &mut self,
        counters: &mut WorldCounters,
        packet: ProtocolContainerSetData,
    ) -> bool {
        const MAX_CONTAINER_DATA_SLOTS: i16 = 16;
        counters.container_data_updates_received += 1;
        if packet.id < 0 || packet.id >= MAX_CONTAINER_DATA_SLOTS {
            return false;
        }
        let container = if packet.container_id == INVENTORY_MENU_CONTAINER_ID {
            &mut self.inventory_menu
        } else if let Some(container) = self
            .open_container
            .as_mut()
            .filter(|container| container.container_id == packet.container_id)
        {
            container
        } else {
            return false;
        };
        set_container_data_value(&mut container.data_values, packet.id, packet.value);
        if packet.container_id != INVENTORY_MENU_CONTAINER_ID {
            self.advance_open_container_revision();
        }
        self.update_merchant_offer_count(counters);
        true
    }

    pub(crate) fn apply_container_close(
        &mut self,
        counters: &mut WorldCounters,
        _packet: ProtocolContainerClose,
    ) -> bool {
        counters.container_close_updates_received += 1;
        if self.local_inventory_open {
            self.local_inventory_open = false;
            self.cursor_item = ProtocolItemStackSummary::empty();
            self.local_quick_craft.reset();
            counters.container_close_updates_applied += 1;
            true
        } else if self.open_container.is_some() {
            self.open_container = None;
            self.cursor_item = ProtocolItemStackSummary::empty();
            self.advance_open_container_revision();
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
                self.cursor_item = ProtocolItemStackSummary::empty();
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
            self.cursor_item = ProtocolItemStackSummary::empty();
            self.advance_open_container_revision();
            self.local_quick_craft.reset();
            counters.merchant_offers_tracked = 0;
            true
        } else {
            false
        }
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
        let Some(container_id) = self
            .open_container
            .as_ref()
            .filter(|container| container.menu_type_id == Some(VANILLA_MENU_TYPE_BEACON_ID))
            .map(|container| container.container_id)
        else {
            return false;
        };
        let Some((payment, selection)) = self.menu_slot_mut(container_id, BEACON_PAYMENT_SLOT)
        else {
            return false;
        };
        if item_stack_is_empty(payment) {
            return false;
        }
        payment.count -= 1;
        normalize_item_stack(payment);
        *selection = normalize_local_selected_bundle_item_index(*selection, payment);
        let container = self.container_mut(container_id);

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

    pub(crate) fn apply_local_select_bundle_item(
        &mut self,
        slot_id: i32,
        selected_item_index: i32,
    ) -> bool {
        if selected_item_index < NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX {
            return false;
        }

        let pair = if let Some(container) = self.active_container() {
            let Ok(slot_id) = i16::try_from(slot_id) else {
                return false;
            };
            let container_id = container.container_id;
            self.menu_slot_mut(container_id, slot_id)
        } else {
            self.player_slots
                .iter_mut()
                .find(|row| row.slot == slot_id)
                .map(|row| (&mut row.item, &mut row.local_selected_bundle_item_index))
        };
        let Some((item, selection)) = pair else {
            return false;
        };
        apply_local_selected_bundle_item_index(item, selection, selected_item_index)
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
        let Some(selected) = self.player_slot_mut(selected_slot) else {
            return false;
        };
        if item_stack_is_empty(&selected.item) {
            return false;
        }

        if all {
            selected.item = ProtocolItemStackSummary::empty();
        } else {
            selected.item.count -= 1;
            normalize_item_stack(&mut selected.item);
        }

        selected.normalize_selection();
        self.update_inventory_slot_count(counters);
        true
    }

    pub(crate) fn set_local_merchant_selected_offer(
        &mut self,
        ctx: InventoryCtx<'_>,
        index: i32,
    ) -> bool {
        let Some(container) = self.open_container.as_ref() else {
            return false;
        };
        let Some(offers) = container.merchant_offers.as_ref() else {
            return false;
        };
        let Ok(index_usize) = usize::try_from(index) else {
            return false;
        };
        let Some(offer) = offers.offers.get(index_usize) else {
            return false;
        };
        let container_id = container.container_id;
        let buy_a = offer.buy_a.clone();
        let buy_b = offer.buy_b.clone();
        apply_merchant_selected_offer_payment_autofill(
            self,
            container_id,
            &buy_a,
            buy_b.as_ref(),
            ctx.items,
        );
        self.open_container
            .as_mut()
            .unwrap()
            .merchant_offers
            .as_mut()
            .unwrap()
            .local_selected_offer_index = index;
        true
    }

    fn container_for_click(
        &self,
        slot_num: i16,
    ) -> Result<&ContainerState, ContainerClickBuildError> {
        let Some(container) = self.active_container() else {
            return Err(ContainerClickBuildError::NoOpenContainer);
        };
        if !self.container_slot_is_valid(container, slot_num) {
            return Err(ContainerClickBuildError::InvalidSlot(slot_num));
        }
        Ok(container)
    }

    pub(crate) fn build_container_click_slot(
        &self,
        ctx: InventoryCtx<'_>,
        request: ContainerClickSlotRequest,
    ) -> Result<ProtocolContainerClick, ContainerClickBuildError> {
        let container = self.container_for_click(request.slot_num)?;
        let carried_item = hashed_stack_from_summary(ctx, &self.cursor_item)
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

    pub(crate) fn predict_container_click_slot(
        &mut self,
        ctx: InventoryCtx<'_>,
        request: ContainerClickSlotRequest,
        emit_packet: bool,
    ) -> Result<
        (
            Option<ProtocolContainerClick>,
            Option<crate::LocalInteractionSoundAction>,
        ),
        ContainerClickBuildError,
    > {
        let container = self.container_for_click(request.slot_num)?;
        let container_id = container.container_id;
        let state_id = container.state_id;
        let make_packet = |changed_slots, carried_item| ProtocolContainerClick {
            container_id,
            state_id,
            slot_num: request.slot_num,
            button_num: request.button_num,
            input: request.input,
            changed_slots,
            carried_item,
        };
        let menu_type_id = container.menu_type_id;
        let mount = container.mount;
        let data_values = container.data_values.as_slice();
        let mount_equipment_slots =
            mount.and_then(|_| self.open_mount_equipment_slot_visibility(ctx));
        let mount_rules = local_mount_slot_rules(ctx, container, mount_equipment_slots);
        let mut quick_craft_after = self.local_quick_craft.clone();
        let mut click_sound_action: Option<crate::LocalInteractionSoundAction> = None;
        if request.input == ProtocolContainerInput::QuickCraft {
            if !(container_id == INVENTORY_MENU_CONTAINER_ID
                || menu_type_has_known_slot_rules(menu_type_id)
                || mount_rules.is_some())
            {
                return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                    request.input,
                ));
            }
            if !advance_quick_craft(
                ctx,
                container,
                mount_rules,
                self.container_slot(container, request.slot_num),
                &self.cursor_item,
                &mut quick_craft_after,
                request.button_num,
            ) {
                let packet = if emit_packet {
                    let carried_item = hashed_stack_from_summary(ctx, &self.cursor_item)
                        .ok_or(ContainerClickBuildError::UnhashableCarriedItem)?;
                    Some(make_packet(BTreeMap::new(), carried_item))
                } else {
                    None
                };
                self.local_quick_craft = quick_craft_after;
                return Ok((packet, None));
            }
        }
        let mut slots_after = self
            .container_slots(container)
            .map(ContainerSlotRef::to_owned)
            .collect::<Vec<_>>();
        let player_slots = PlayerInventoryAddSlots::for_container(container);
        let mut cursor_after = self.cursor_item.clone();
        let mut merchant_offers_after = container.merchant_offers.clone();
        let external_offhand_before = (container_id != INVENTORY_MENU_CONTAINER_ID
            && (request.input == ProtocolContainerInput::Swap
                || (menu_type_id == Some(VANILLA_MENU_TYPE_CRAFTING_ID)
                    && request.slot_num == CRAFTING_MENU_RESULT_SLOT
                    && matches!(
                        request.input,
                        ProtocolContainerInput::Pickup
                            | ProtocolContainerInput::QuickMove
                            | ProtocolContainerInput::Swap
                            | ProtocolContainerInput::Throw
                    ))))
        .then(|| {
            self.local_offhand_item()
                .cloned()
                .unwrap_or_else(ProtocolItemStackSummary::empty)
        });
        let mut external_offhand_after = external_offhand_before.clone();
        let anvil_result_may_pickup = anvil_result_may_pickup(
            data_values,
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
                    if is_interactive_menu_result_slot(menu_type_id, request.slot_num)
                        && is_bundle_item(&cursor_after) =>
                {
                    return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                        ProtocolContainerInput::Pickup,
                    ));
                }
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
                        external_offhand_after.as_mut(),
                        ctx.local_player.selected_hotbar_slot,
                        ctx.items,
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
                    if !apply_merchant_result_to_slots(
                        &mut slots_after,
                        ResultTakeDestination::Cursor(&mut cursor_after, request.button_num),
                        selected_offer.as_ref(),
                        ctx.items,
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
                    if !apply_stonecutter_result_to_slots(
                        &mut slots_after,
                        ResultTakeDestination::Cursor(&mut cursor_after, request.button_num),
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
                    if !apply_loom_result_to_slots(
                        &mut slots_after,
                        ResultTakeDestination::Cursor(&mut cursor_after, request.button_num),
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
                    if !apply_anvil_result_to_slots(
                        &mut slots_after,
                        ResultTakeDestination::Cursor(&mut cursor_after, request.button_num),
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
                    if !apply_grindstone_result_to_slots(
                        &mut slots_after,
                        ResultTakeDestination::Cursor(&mut cursor_after, request.button_num),
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
                    if !apply_smithing_result_to_slots(
                        &mut slots_after,
                        ResultTakeDestination::Cursor(&mut cursor_after, request.button_num),
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
                    if !apply_cartography_table_result_to_slots(
                        &mut slots_after,
                        ResultTakeDestination::Cursor(&mut cursor_after, request.button_num),
                    ) {
                        return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                            ProtocolContainerInput::Pickup,
                        ));
                    }
                }
                ProtocolContainerInput::Pickup => {
                    let slot_access = if request.slot_num >= 0 {
                        let Some(access) = local_container_slot_access(
                            ctx,
                            container_id,
                            menu_type_id,
                            mount_rules,
                            data_values,
                            request.slot_num,
                            container_slot_item(&slots_after, request.slot_num)
                                .unwrap_or(&ProtocolItemStackSummary::empty()),
                            &cursor_after,
                        ) else {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::Pickup,
                            ));
                        };
                        Some(access)
                    } else {
                        None
                    };
                    let bundle_handled = if let Some(access) = slot_access {
                        if let Some(slot) = slots_after
                            .iter_mut()
                            .find(|slot| slot.slot == request.slot_num)
                        {
                            let slot_num = slot.slot;
                            let slot_item_snapshot = slot.item.clone();
                            let Some((handled, bundle_sound)) = try_apply_bundle_pickup_override(
                                slot,
                                &mut cursor_after,
                                request.button_num,
                                access,
                                ctx.items,
                                |stack_to_place| {
                                    local_container_slot_access(
                                        ctx,
                                        container_id,
                                        menu_type_id,
                                        mount_rules,
                                        data_values,
                                        slot_num,
                                        &slot_item_snapshot,
                                        stack_to_place,
                                    )
                                },
                            ) else {
                                return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                    ProtocolContainerInput::Pickup,
                                ));
                            };
                            if let Some(sound) = bundle_sound {
                                click_sound_action = Some(sound.into_action());
                            }
                            handled
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    if !bundle_handled {
                        apply_pickup_click_to_slots(
                            &mut slots_after,
                            &mut cursor_after,
                            request.slot_num,
                            request.button_num,
                            slot_access,
                            ctx.items,
                        );
                    }
                }
                ProtocolContainerInput::Clone => apply_clone_click_to_slots(
                    &slots_after,
                    &mut cursor_after,
                    request.slot_num,
                    ctx.local_player
                        .abilities
                        .is_some_and(|abilities| abilities.instabuild),
                    ctx.items,
                ),
                ProtocolContainerInput::QuickMove => {
                    // Vanilla `AbstractContainerMenu.doClick` (`AbstractContainerMenu.java:418-421`):
                    // abort `QuickMove` if the clicked slot refuses `mayPickup` (e.g. Curse of Binding).
                    let quick_move_may_pickup = if request.slot_num >= 0 {
                        if let Some(clicked_item) =
                            container_slot_item(&slots_after, request.slot_num)
                                .filter(|item| item_stack_is_non_empty(item))
                        {
                            let has_known_rules = container_id == INVENTORY_MENU_CONTAINER_ID
                                || menu_type_has_known_slot_rules(menu_type_id)
                                || mount_rules.is_some();
                            match local_container_slot_access(
                                ctx,
                                container_id,
                                menu_type_id,
                                mount_rules,
                                data_values,
                                request.slot_num,
                                clicked_item,
                                clicked_item,
                            ) {
                                Some(access) => access.may_pickup,
                                None if has_known_rules => {
                                    return Err(
                                        ContainerClickBuildError::UnsupportedLocalClickInput(
                                            ProtocolContainerInput::QuickMove,
                                        ),
                                    );
                                }
                                None => true,
                            }
                        } else {
                            true
                        }
                    } else {
                        true
                    };
                    if !quick_move_may_pickup {
                        // Blocked by `Slot.mayPickup` — no-op click.
                    } else if container_id == INVENTORY_MENU_CONTAINER_ID {
                        if request.slot_num == 0 {
                            if !apply_inventory_menu_result_quick_move_to_slots(
                                &mut slots_after,
                                &ctx.items.default_item_crafting_remainders,
                                ctx.local_player.selected_hotbar_slot,
                                ctx.items,
                            ) {
                                return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                    ProtocolContainerInput::QuickMove,
                                ));
                            }
                        } else {
                            apply_quick_move_to_slots(
                                container_id,
                                &mut slots_after,
                                request.slot_num,
                                ctx.items,
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
                            ctx.items,
                        )
                    } else if mount.is_some() {
                        if mount_inventory_quick_move_requires_server_authority(
                            slot_state::player_section_start(container).unwrap(),
                            &slots_after,
                            request.slot_num,
                            ctx,
                            mount_rules,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                        apply_mount_inventory_quick_move_to_slots(
                            container_id,
                            slot_state::player_section_start(container).unwrap(),
                            &mut slots_after,
                            request.slot_num,
                            ctx,
                            mount_rules,
                        )
                    } else if let Some(container_slot_count) =
                        generic_3x3_container_slot_count(menu_type_id)
                    {
                        apply_generic_container_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            container_slot_count,
                            ctx.items,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_CRAFTING_ID) {
                        if request.slot_num == CRAFTING_MENU_RESULT_SLOT {
                            if !apply_crafting_menu_result_quick_move_to_slots(
                                container_id,
                                &mut slots_after,
                                ctx.items.default_item_crafting_remainders_known,
                                &ctx.items.default_item_crafting_remainders,
                                external_offhand_after.as_mut(),
                                ctx.local_player.selected_hotbar_slot,
                                ctx.items,
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
                                ctx.items,
                            )
                        }
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_CRAFTER_3X3_ID) {
                        let disabled_slots = crafter_disabled_slots(data_values);
                        apply_crafter_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &disabled_slots,
                            ctx.items,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_ANVIL_ID) {
                        apply_anvil_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            anvil_result_may_pickup,
                            ctx.items,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_BEACON_ID) {
                        apply_beacon_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            ctx.registries.tags.get("minecraft:item"),
                            ctx.items,
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
                            ctx.items,
                        )
                    } else if furnace_family_menu_type(menu_type_id).is_some() {
                        apply_furnace_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            menu_type_id,
                            &ctx.recipes.property_sets,
                            &ctx.items.furnace_fuel_item_ids,
                            ctx.items,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_BREWING_STAND_ID) {
                        apply_brewing_stand_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            ctx.registries.tags.get("minecraft:item"),
                            &ctx.items.brewing_potion_item_ids,
                            &ctx.items.brewing_ingredient_item_ids,
                            ctx.items,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_GRINDSTONE_ID) {
                        if grindstone_quick_move_requires_server_authority(
                            ctx,
                            &slots_after,
                            request.slot_num,
                            ctx.items,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                        apply_grindstone_menu_quick_move_to_slots(
                            ctx,
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            ctx.items,
                        )
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_SMITHING_ID) {
                        if smithing_quick_move_requires_server_authority(
                            &slots_after,
                            request.slot_num,
                            &ctx.recipes.property_sets,
                        ) || !apply_smithing_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &ctx.recipes.property_sets,
                            ctx.items,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID) {
                        if cartography_table_quick_move_requires_server_authority(
                            ctx,
                            &slots_after,
                            request.slot_num,
                            &ctx.items.cartography_additional_items,
                        ) || !apply_cartography_table_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &ctx.items.cartography_additional_items,
                            ctx.items,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_LOOM_ID) {
                        if !apply_loom_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            ctx.registries.tags.get("minecraft:item"),
                            ctx.items,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::QuickMove,
                            ));
                        }
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_MERCHANT_ID) {
                        if request.slot_num == MERCHANT_RESULT_SLOT {
                            if let Some(taken_item_id) =
                                container_slot_item(&slots_after, MERCHANT_RESULT_SLOT)
                                    .filter(|item| item_stack_is_non_empty(item))
                                    .and_then(|item| item.item_id)
                            {
                                // Vanilla `AbstractContainerMenu.doClick` (`AbstractContainerMenu.java:423-427`)
                                // repeats `quickMoveStack` while the result slot keeps producing the same item.
                                let mut traded_any = false;
                                loop {
                                    let selected_offer =
                                        merchant_offers_after.as_ref().and_then(|offers| {
                                            merchant_selected_offer_index(offers)
                                                .ok()
                                                .and_then(|index| offers.offers.get(index))
                                                .cloned()
                                        });
                                    let Some(offer) = selected_offer else {
                                        if !traded_any {
                                            return Err(
                                                ContainerClickBuildError::UnsupportedLocalClickInput(
                                                    ProtocolContainerInput::QuickMove,
                                                ),
                                            );
                                        }
                                        break;
                                    };
                                    if !apply_merchant_menu_quick_move_to_slots(
                                        container_id,
                                        &mut slots_after,
                                        request.slot_num,
                                        Some(&offer),
                                        ctx.items,
                                    ) {
                                        if !traded_any
                                            && merchant::merchant_offer_result_take_requires_server_authority(
                                                &offer, ctx.items,
                                            )
                                        {
                                            return Err(
                                                ContainerClickBuildError::UnsupportedLocalClickInput(
                                                    ProtocolContainerInput::QuickMove,
                                                ),
                                            );
                                        }
                                        break;
                                    }
                                    traded_any = true;
                                    merchant_increment_selected_offer_use(
                                        &mut merchant_offers_after,
                                    );
                                    if !container_slot_item(&slots_after, MERCHANT_RESULT_SLOT)
                                        .is_some_and(|item| {
                                            item_stack_is_non_empty(item)
                                                && item.item_id == Some(taken_item_id)
                                        })
                                    {
                                        break;
                                    }
                                }
                            }
                        } else {
                            let _ = apply_merchant_menu_quick_move_to_slots(
                                container_id,
                                &mut slots_after,
                                request.slot_num,
                                None,
                                ctx.items,
                            );
                        }
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_STONECUTTER_ID) {
                        apply_stonecutter_menu_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            &ctx.recipes.stonecutter_recipes,
                            ctx.items,
                        )
                    } else if let Some(container_slot_count) =
                        hopper_container_slot_count(menu_type_id)
                    {
                        apply_generic_container_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            container_slot_count,
                            ctx.items,
                        )
                    } else if let Some(container_slot_count) =
                        shulker_box_container_slot_count(menu_type_id)
                    {
                        apply_shulker_box_quick_move_to_slots(
                            container_id,
                            &mut slots_after,
                            request.slot_num,
                            container_slot_count,
                            ctx.items,
                        )
                    } else {
                        return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                            ProtocolContainerInput::QuickMove,
                        ));
                    }
                }
                ProtocolContainerInput::Throw
                    if is_interactive_menu_result_slot(menu_type_id, request.slot_num) =>
                {
                    if item_stack_is_empty(&cursor_after)
                        && matches!(request.button_num, 0 | 1)
                        && container_slot_item(&slots_after, request.slot_num)
                            .is_some_and(item_stack_is_non_empty)
                    {
                        let taken_item_id = container_slot_item(&slots_after, request.slot_num)
                            .and_then(|item| item.item_id);
                        if !apply_interactive_menu_result_throw(
                            container_id,
                            menu_type_id,
                            &mut slots_after,
                            &mut merchant_offers_after,
                            anvil_result_may_pickup,
                            external_offhand_after.as_mut(),
                            ctx,
                        ) {
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::Throw,
                            ));
                        }
                        if request.button_num == 1
                            && menu_type_id == Some(VANILLA_MENU_TYPE_CRAFTING_ID)
                            && !non_empty_slot_nums(
                                &slots_after,
                                CRAFTING_MENU_CRAFT_SLOT_START,
                                CRAFTING_MENU_CRAFT_SLOT_END,
                            )
                            .is_empty()
                        {
                            // Vanilla `AbstractContainerMenu.doClick` loops `THROW` button 1 while
                            // the server-side recipe repopulates the result slot; if ingredients
                            // remain in the grid, keep the click server-authoritative.
                            return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                ProtocolContainerInput::Throw,
                            ));
                        }
                        if request.button_num == 1
                            && menu_type_id == Some(VANILLA_MENU_TYPE_MERCHANT_ID)
                        {
                            // Vanilla `AbstractContainerMenu.doClick` loops `THROW` button 1 while
                            // `ItemStack.isSameItem(slot.getItem(), itemStack)` remains true.
                            while container_slot_item(&slots_after, MERCHANT_RESULT_SLOT)
                                .is_some_and(|item| {
                                    item_stack_is_non_empty(item) && item.item_id == taken_item_id
                                })
                            {
                                if !apply_interactive_menu_result_throw(
                                    container_id,
                                    menu_type_id,
                                    &mut slots_after,
                                    &mut merchant_offers_after,
                                    anvil_result_may_pickup,
                                    external_offhand_after.as_mut(),
                                    ctx,
                                ) {
                                    break;
                                }
                            }
                        }
                    }
                }
                ProtocolContainerInput::Throw
                    if container_id == INVENTORY_MENU_CONTAINER_ID
                        || menu_type_has_known_slot_rules(menu_type_id)
                        || mount_rules.is_some() =>
                {
                    let slot_access = slots_after
                        .iter()
                        .find(|slot| slot.slot == request.slot_num)
                        .and_then(|slot| {
                            local_container_slot_access(
                                ctx,
                                container_id,
                                menu_type_id,
                                mount_rules,
                                data_values,
                                request.slot_num,
                                &slot.item,
                                &slot.item,
                            )
                        });
                    apply_throw_click_to_slots(
                        &mut slots_after,
                        &cursor_after,
                        request.slot_num,
                        request.button_num,
                        slot_access,
                    )
                }
                ProtocolContainerInput::Swap
                    if is_interactive_menu_result_slot(menu_type_id, request.slot_num)
                        && (request.button_num == VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT as i8
                            || player_slots
                                .and_then(|slots| slots.swap_slot(request.button_num))
                                .is_some()) =>
                {
                    if container_slot_item(&slots_after, request.slot_num)
                        .is_some_and(item_stack_is_non_empty)
                    {
                        if request.button_num == VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT as i8 {
                            let offhand = external_offhand_after
                                .as_mut()
                                .expect("external offhand captured for button 40");
                            if item_stack_is_empty(offhand)
                                && !apply_interactive_menu_result_external_offhand_swap(
                                    container_id,
                                    menu_type_id,
                                    &mut slots_after,
                                    offhand,
                                    &mut merchant_offers_after,
                                    anvil_result_may_pickup,
                                    ctx,
                                )
                            {
                                return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                    ProtocolContainerInput::Swap,
                                ));
                            }
                        } else {
                            let source_slot_num = player_slots
                                .and_then(|slots| slots.swap_slot(request.button_num))
                                .expect("guarded swap source slot");
                            if container_slot_item(&slots_after, source_slot_num)
                                .is_some_and(item_stack_is_empty)
                                && !apply_interactive_menu_result_slot_swap(
                                    container_id,
                                    menu_type_id,
                                    &mut slots_after,
                                    source_slot_num,
                                    &mut merchant_offers_after,
                                    anvil_result_may_pickup,
                                    external_offhand_after.as_mut(),
                                    ctx,
                                )
                            {
                                return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                                    ProtocolContainerInput::Swap,
                                ));
                            }
                        }
                    }
                }
                ProtocolContainerInput::Swap
                    if container_id != INVENTORY_MENU_CONTAINER_ID
                        && request.button_num == VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT as i8
                        && (menu_type_has_known_slot_rules(menu_type_id)
                            || mount_rules.is_some()) =>
                {
                    let source_item = external_offhand_after
                        .as_ref()
                        .cloned()
                        .expect("external offhand captured for button 40");
                    let Some(target_access) = local_container_slot_access(
                        ctx,
                        container_id,
                        menu_type_id,
                        mount_rules,
                        data_values,
                        request.slot_num,
                        container_slot_item(&slots_after, request.slot_num)
                            .unwrap_or(&ProtocolItemStackSummary::empty()),
                        &source_item,
                    ) else {
                        return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                            ProtocolContainerInput::Swap,
                        ));
                    };
                    apply_external_offhand_swap_to_slots(
                        container_id,
                        player_slots,
                        &mut slots_after,
                        external_offhand_after
                            .as_mut()
                            .expect("external offhand captured for button 40"),
                        request.slot_num,
                        target_access,
                        ctx.local_player.selected_hotbar_slot,
                        ctx.items,
                    )
                }
                ProtocolContainerInput::Swap
                    if (container_id == INVENTORY_MENU_CONTAINER_ID
                        || menu_type_has_known_slot_rules(menu_type_id)
                        || mount_rules.is_some())
                        && player_slots
                            .and_then(|slots| slots.swap_slot(request.button_num))
                            .is_some() =>
                {
                    let source_slot_num = player_slots
                        .and_then(|slots| slots.swap_slot(request.button_num))
                        .expect("guarded swap source slot");
                    let source_item = container_slot_item(&slots_after, source_slot_num)
                        .cloned()
                        .unwrap_or_else(ProtocolItemStackSummary::empty);
                    let Some(target_access) = local_container_slot_access(
                        ctx,
                        container_id,
                        menu_type_id,
                        mount_rules,
                        data_values,
                        request.slot_num,
                        container_slot_item(&slots_after, request.slot_num)
                            .unwrap_or(&ProtocolItemStackSummary::empty()),
                        &source_item,
                    ) else {
                        return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                            ProtocolContainerInput::Swap,
                        ));
                    };
                    apply_swap_click_to_slots(
                        container_id,
                        player_slots.expect("guarded swap player section"),
                        &mut slots_after,
                        request.slot_num,
                        request.button_num,
                        target_access,
                        external_offhand_after.as_mut(),
                        ctx.local_player.selected_hotbar_slot,
                        ctx.items,
                    )
                }
                ProtocolContainerInput::QuickCraft => finish_quick_craft(
                    ctx,
                    container_id,
                    menu_type_id,
                    mount_rules,
                    data_values,
                    &mut slots_after,
                    &mut cursor_after,
                    &mut quick_craft_after,
                    ctx.items,
                ),
                ProtocolContainerInput::PickupAll
                    if container_id == INVENTORY_MENU_CONTAINER_ID
                        || menu_type_has_known_slot_rules(menu_type_id)
                        || mount_rules.is_some() =>
                {
                    let Some(slot_access) = local_container_slot_accesses(
                        ctx,
                        container_id,
                        menu_type_id,
                        mount_rules,
                        data_values,
                        &slots_after,
                        &cursor_after,
                    ) else {
                        return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                            ProtocolContainerInput::PickupAll,
                        ));
                    };
                    let pickup_all_will_scan = !item_stack_is_empty(&cursor_after)
                        && container_slot_item(&slots_after, request.slot_num).is_some_and(
                            |clicked| {
                                item_stack_is_empty(clicked)
                                    || !slot_access
                                        .get(&request.slot_num)
                                        .is_some_and(|access| access.may_pickup)
                            },
                        )
                        && cursor_after.count < ctx.items.item_stack_max_stack_size(&cursor_after);
                    if pickup_all_will_scan
                        && slots_after.iter().any(|slot| {
                            !item_stack_is_empty(&slot.item)
                                && same_item_same_components(&slot.item, &cursor_after)
                                && slot_access
                                    .get(&slot.slot)
                                    .is_some_and(|access| access.may_pickup)
                                && menu_slot_can_take_item_for_pick_all(
                                    container_id,
                                    menu_type_id,
                                    slot.slot,
                                )
                                && menu_result_slot_pickup_all_scan_requires_server_authority(
                                    menu_type_id,
                                    slot.slot,
                                )
                        })
                    {
                        return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                            ProtocolContainerInput::PickupAll,
                        ));
                    }
                    apply_pickup_all_to_slots(
                        &mut slots_after,
                        &mut cursor_after,
                        request.slot_num,
                        request.button_num,
                        &slot_access,
                        container_id,
                        menu_type_id,
                        ctx.items,
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
            && inventory_menu_result_was_taken(
                self.container_slot(container, 0).map(|row| row.item),
                &slots_after,
            )
        {
            apply_inventory_menu_result_take_side_effects(
                &mut slots_after,
                &ctx.items.default_item_crafting_remainders,
                ctx.local_player.selected_hotbar_slot,
                ctx.items,
            );
            if request.input == ProtocolContainerInput::Throw
                && request.button_num == 1
                && !inventory_menu_non_empty_crafting_slot_nums(&slots_after).is_empty()
            {
                return Err(ContainerClickBuildError::UnsupportedLocalClickInput(
                    ProtocolContainerInput::Throw,
                ));
            }
        }
        let packet = if emit_packet {
            let changed_slots =
                changed_hashed_slots(ctx, self.container_slots(container), &slots_after)?;
            let carried_item = hashed_stack_from_summary(ctx, &cursor_after)
                .ok_or(ContainerClickBuildError::UnhashableCarriedItem)?;
            Some(make_packet(changed_slots, carried_item))
        } else {
            None
        };

        self.replace_container_slots(container_id, slots_after);
        if menu_type_id == Some(VANILLA_MENU_TYPE_MERCHANT_ID) {
            self.container_mut(container_id).merchant_offers = merchant_offers_after;
        }
        self.cursor_item = cursor_after;
        self.local_quick_craft = quick_craft_after;
        if external_offhand_after != external_offhand_before {
            self.set_player_slot(InventorySlot::new(
                VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
                external_offhand_after.expect("changed external offhand remains captured"),
            ));
        }

        Ok((packet, click_sound_action))
    }
}

fn apply_interactive_menu_result_throw(
    container_id: i32,
    menu_type_id: Option<i32>,
    slots: &mut [ContainerSlot],
    merchant_offers: &mut Option<MerchantOffersState>,
    anvil_result_may_pickup: bool,
    external_offhand: Option<&mut ProtocolItemStackSummary>,
    ctx: InventoryCtx<'_>,
) -> bool {
    if menu_type_id == Some(VANILLA_MENU_TYPE_CRAFTING_ID) {
        return apply_crafting_menu_result_take_to_slots(
            container_id,
            slots,
            CraftingResultTakeTarget::Drop,
            ctx.items.default_item_crafting_remainders_known,
            &ctx.items.default_item_crafting_remainders,
            external_offhand,
            ctx.local_player.selected_hotbar_slot,
            ctx.items,
        );
    }
    apply_non_crafting_menu_result_take(
        menu_type_id,
        slots,
        ResultTakeDestination::Drop,
        merchant_offers,
        anvil_result_may_pickup,
        ctx.items,
    )
}

#[allow(clippy::too_many_arguments)]
fn apply_interactive_menu_result_slot_swap(
    container_id: i32,
    menu_type_id: Option<i32>,
    slots: &mut [ContainerSlot],
    dest_slot_num: i16,
    merchant_offers: &mut Option<MerchantOffersState>,
    anvil_result_may_pickup: bool,
    external_offhand: Option<&mut ProtocolItemStackSummary>,
    ctx: InventoryCtx<'_>,
) -> bool {
    if menu_type_id == Some(VANILLA_MENU_TYPE_CRAFTING_ID) {
        return apply_crafting_menu_result_take_to_slots(
            container_id,
            slots,
            CraftingResultTakeTarget::Slot(dest_slot_num),
            ctx.items.default_item_crafting_remainders_known,
            &ctx.items.default_item_crafting_remainders,
            external_offhand,
            ctx.local_player.selected_hotbar_slot,
            ctx.items,
        );
    }
    apply_non_crafting_menu_result_take(
        menu_type_id,
        slots,
        ResultTakeDestination::Slot(dest_slot_num),
        merchant_offers,
        anvil_result_may_pickup,
        ctx.items,
    )
}

fn apply_interactive_menu_result_external_offhand_swap(
    container_id: i32,
    menu_type_id: Option<i32>,
    slots: &mut [ContainerSlot],
    offhand: &mut ProtocolItemStackSummary,
    merchant_offers: &mut Option<MerchantOffersState>,
    anvil_result_may_pickup: bool,
    ctx: InventoryCtx<'_>,
) -> bool {
    if menu_type_id == Some(VANILLA_MENU_TYPE_CRAFTING_ID) {
        return apply_crafting_menu_result_take_to_slots(
            container_id,
            slots,
            CraftingResultTakeTarget::ExternalOffhand,
            ctx.items.default_item_crafting_remainders_known,
            &ctx.items.default_item_crafting_remainders,
            Some(offhand),
            ctx.local_player.selected_hotbar_slot,
            ctx.items,
        );
    }
    apply_non_crafting_menu_result_take(
        menu_type_id,
        slots,
        ResultTakeDestination::Cursor(offhand, 0),
        merchant_offers,
        anvil_result_may_pickup,
        ctx.items,
    )
}

fn apply_non_crafting_menu_result_take(
    menu_type_id: Option<i32>,
    slots: &mut [ContainerSlot],
    destination: ResultTakeDestination<'_>,
    merchant_offers: &mut Option<MerchantOffersState>,
    anvil_result_may_pickup: bool,
    profiles: &ItemProfiles,
) -> bool {
    match menu_type_id {
        Some(VANILLA_MENU_TYPE_MERCHANT_ID) => {
            let selected_offer = merchant_offers.as_ref().and_then(|offers| {
                merchant_selected_offer_index(offers)
                    .ok()
                    .and_then(|index| offers.offers.get(index))
                    .cloned()
            });
            if !apply_merchant_result_to_slots(
                slots,
                destination,
                selected_offer.as_ref(),
                profiles,
            ) {
                return false;
            }
            merchant_increment_selected_offer_use(merchant_offers);
            true
        }
        Some(VANILLA_MENU_TYPE_STONECUTTER_ID) => {
            apply_stonecutter_result_to_slots(slots, destination)
        }
        Some(VANILLA_MENU_TYPE_LOOM_ID) => apply_loom_result_to_slots(slots, destination),
        Some(VANILLA_MENU_TYPE_ANVIL_ID) => {
            apply_anvil_result_to_slots(slots, destination, anvil_result_may_pickup)
        }
        Some(VANILLA_MENU_TYPE_GRINDSTONE_ID) => {
            apply_grindstone_result_to_slots(slots, destination)
        }
        Some(VANILLA_MENU_TYPE_SMITHING_ID) => apply_smithing_result_to_slots(slots, destination),
        Some(VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID) => {
            apply_cartography_table_result_to_slots(slots, destination)
        }
        _ => false,
    }
}

fn local_player_hand_inventory_slot(
    selected_hotbar_slot: u8,
    hand: InteractionHand,
) -> Option<i32> {
    match hand {
        InteractionHand::MainHand => {
            (selected_hotbar_slot <= 8).then_some(i32::from(selected_hotbar_slot))
        }
        InteractionHand::OffHand => Some(VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT),
    }
}

fn local_player_equipment_inventory_slot(
    selected_hotbar_slot: u8,
    equipment_slot: EquipmentSlot,
) -> Option<i32> {
    if equipment_slot == EquipmentSlot::MainHand {
        return local_player_hand_inventory_slot(selected_hotbar_slot, InteractionHand::MainHand);
    }
    VANILLA_EQUIPMENT_SLOTS
        .iter()
        .find(|entry| entry.slot == equipment_slot)?
        .player_inventory_slot
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CreativeFilledResult {
    AddIfMissing,
    AlwaysAdd,
    ConsumeInput,
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
    pub(crate) registries: &'a RegistrySet,
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

fn inventory_menu_slot_to_inventory_slot(slot: i16) -> Option<i32> {
    match slot {
        INVENTORY_MENU_HOTBAR_START..=44 => Some(i32::from(slot - INVENTORY_MENU_HOTBAR_START)),
        9..=35 => Some(i32::from(slot)),
        5..=8 => Some(i32::from(44 - slot)),
        INVENTORY_MENU_OFFHAND_SLOT => Some(VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT),
        _ => None,
    }
}

impl bbb_registry_codec::item_hash::ItemHashLookup for RegistrySet {
    fn item_resource_id(&self, item_id: i32) -> Option<String> {
        self.resource_id_by_holder_id("minecraft:item", item_id)
            .map(str::to_owned)
    }

    fn resolve_holder_resource_id(&self, registry_id: &str, holder_id: i32) -> Option<String> {
        self.resource_id_by_holder_id(registry_id, holder_id)
            .map(str::to_owned)
    }
}

fn hashed_stack_from_summary(
    ctx: InventoryCtx<'_>,
    stack: &ProtocolItemStackSummary,
) -> Option<ProtocolHashedStack> {
    bbb_registry_codec::item_hash::hashed_stack_from_summary(ctx.registries, stack)
}

fn hashed_component_patch_from_summary(
    ctx: InventoryCtx<'_>,
    patch: &ProtocolDataComponentPatchSummary,
) -> Option<ProtocolHashedComponentPatch> {
    bbb_registry_codec::item_hash::hashed_component_patch_from_summary(ctx.registries, patch)
}

fn component_patch_can_be_hashed_from_summary(
    ctx: InventoryCtx<'_>,
    patch: &ProtocolDataComponentPatchSummary,
) -> bool {
    hashed_component_patch_from_summary(ctx, patch).is_some()
}

fn changed_hashed_slots<'a>(
    ctx: InventoryCtx<'_>,
    before: impl Iterator<Item = ContainerSlotRef<'a>>,
    after: &[ContainerSlot],
) -> Result<BTreeMap<i16, ProtocolHashedStack>, ContainerClickBuildError> {
    let mut changed = BTreeMap::new();
    // Prediction preserves slot IDs and row order in a clone of before.
    for (before, slot) in before.zip(after) {
        if before.item == &slot.item {
            continue;
        }
        let hashed = hashed_stack_from_summary(ctx, &slot.item)
            .ok_or(ContainerClickBuildError::UnhashableChangedSlot(slot.slot))?;
        changed.insert(slot.slot, hashed);
    }
    Ok(changed)
}
