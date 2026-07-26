use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID, VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
    VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID, VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
    VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
    VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID, VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
};
use bbb_protocol::equipment_slots::{
    vanilla_equipment_slot_by_id, EquipmentSlot, VANILLA_EQUIPMENT_SLOTS,
    VANILLA_PLAYER_INVENTORY_BODY_SLOT, VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
    VANILLA_PLAYER_INVENTORY_FEET_SLOT, VANILLA_PLAYER_INVENTORY_HEAD_SLOT,
    VANILLA_PLAYER_INVENTORY_LEGS_SLOT, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
    VANILLA_PLAYER_INVENTORY_SADDLE_SLOT,
};
use bbb_protocol::item_types::{
    vanilla_item_type, VanillaItemUseOverride, VANILLA_ITEM_BUCKET_ID, VANILLA_ITEM_COMPASS_ID,
    VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE, VANILLA_ITEM_GLASS_BOTTLE_ID, VANILLA_ITEM_POTION_ID,
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
use bbb_protocol::packets::{
    AttackRangeSummary as ProtocolAttackRangeSummary, ContainerClick as ProtocolContainerClick,
    ContainerClose as ProtocolContainerClose, ContainerInput as ProtocolContainerInput,
    ContainerSetContent as ProtocolContainerSetContent,
    ContainerSetData as ProtocolContainerSetData, ContainerSetSlot as ProtocolContainerSetSlot,
    DataComponentPatchSummary as ProtocolDataComponentPatchSummary,
    EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind, InteractionHand,
    ItemCostSummary as ProtocolItemCostSummary, ItemStackSummary as ProtocolItemStackSummary,
    LodestoneTargetSummary as ProtocolLodestoneTargetSummary,
    MerchantOffer as ProtocolMerchantOffer, MerchantOffers as ProtocolMerchantOffers,
    OpenScreen as ProtocolOpenScreen, SetCursorItem as ProtocolSetCursorItem,
    SetPlayerInventory as ProtocolSetPlayerInventory,
    StonecutterSelectableRecipeSummary as ProtocolStonecutterSelectableRecipeSummary,
    SwingAnimationSummary as ProtocolSwingAnimationSummary,
};
#[cfg(test)]
use bbb_protocol::packets::{
    HashedComponentPatch as ProtocolHashedComponentPatch,
    HashedItemStack as ProtocolHashedItemStack, HashedStack as ProtocolHashedStack,
};
use bbb_protocol::RichTextComponent;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::entities::{
    vanilla_entity_metadata_abstract_horse_accessors_for_entity_type,
    vanilla_entity_metadata_baby_accessor_for_entity_type,
    vanilla_entity_metadata_tamable_animal_accessors_for_entity_type, EntityStore,
    ATTACK_SWING_DURATION,
};
use crate::item_profiles::{clamp_vanilla_item_max_stack_size, ItemProfiles};
use crate::{
    ClientRecipesState, ClientUiState, LocalPlayerAbilitiesState, LocalPlayerExperienceState,
    LocalPlayerState, MountScreenState, RegistrySet, RegistryTagState, WorldCounters, WorldStore,
    BANNER_PATTERN_REGISTRY,
};

mod container_button;
mod crafting;
mod grindstone;
mod hash_ops;
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
        smithing_input_slot_accepts_stack, smithing_quick_move_requires_server_authority,
    },
    stonecutter::{
        apply_stonecutter_menu_quick_move_to_slots, apply_stonecutter_result_pickup_to_slots,
    },
};
use self::{hash_ops::*, slot_ops::*};

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
const INVENTORY_MENU_CONTAINER_ID: i32 = 0;
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

impl ItemEquipmentSlot {
    fn protocol_slot(self) -> EquipmentSlot {
        match self {
            Self::MainHand => EquipmentSlot::MainHand,
            Self::OffHand => EquipmentSlot::OffHand,
            Self::Feet => EquipmentSlot::Feet,
            Self::Legs => EquipmentSlot::Legs,
            Self::Chest => EquipmentSlot::Chest,
            Self::Head => EquipmentSlot::Head,
            Self::Body => EquipmentSlot::Body,
            Self::Saddle => EquipmentSlot::Saddle,
        }
    }
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemFoodProperties {
    pub nutrition: i32,
    pub saturation_bits: u32,
    pub can_always_eat: bool,
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
    /// Exact trusted title supplied by `ClientboundOpenScreenPacket`.
    pub title: Option<RichTextComponent>,
    #[serde(default)]
    pub mount: Option<MountScreenState>,
    pub state_id: i32,
    pub slots: Vec<ContainerSlot>,
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
    ) -> bool {
        counters.inventory_slot_updates_received += 1;
        let slot_id = packet.slot;
        if !(0..=VANILLA_PLAYER_INVENTORY_SADDLE_SLOT).contains(&slot_id) {
            return false;
        }
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
        true
    }

    pub(crate) fn predict_writable_book_pages(&mut self, slot_id: i32, pages: Vec<String>) -> bool {
        let Some(mut item) = self.local_player_inventory_item(slot_id).cloned() else {
            return false;
        };
        let patch = &mut item.component_patch;
        patch.removed_type_ids.retain(|component_id| {
            *component_id != VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID
        });
        if !patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID)
        {
            patch
                .added_type_ids
                .push(VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID);
        }
        patch.added = patch.added_type_ids.len();
        patch.writable_book_page_filters = vec![None; pages.len()];
        patch.writable_book_pages = pages;
        let menu_slot = inventory_slot_to_inventory_menu_slot(slot_id);
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
        counters.container_open_updates_received += 1;
        ui.replace_screen();
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
                mount: existing.mount,
                state_id,
                slots,
                data_values: existing.data_values,
                merchant_offers: existing.merchant_offers,
            };
            return true;
        }

        let total_slots = slots.len() as i16;
        self.write_back_container_player_section(counters, total_slots, &slots);
        let existing = self
            .open_container
            .take()
            .expect("matching open container was checked");
        let merchant_offers = existing.merchant_offers.clone();
        self.open_container = Some(ContainerState {
            container_id,
            menu_type_id: existing.menu_type_id,
            title: existing.title,
            mount: existing.mount,
            state_id,
            slots,
            data_values: existing.data_values,
            merchant_offers,
        });
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

    pub(crate) fn apply_mount_screen_open_container(
        &mut self,
        counters: &mut WorldCounters,
        ui: &mut ClientUiState,
        mount: MountScreenState,
    ) {
        ui.replace_screen();
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
    ) -> bool {
        counters.container_slot_updates_received += 1;
        let is_inventory_menu = packet.container_id == INVENTORY_MENU_CONTAINER_ID;
        let container = if is_inventory_menu {
            self.inventory_menu_container()
        } else if let Some(container) = self
            .open_container
            .as_mut()
            .filter(|container| container.container_id == packet.container_id)
        {
            container
        } else {
            return false;
        };
        container.state_id = packet.state_id;
        let row = ContainerSlot {
            slot: packet.slot,
            item: packet.item,
            local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
        };
        set_container_slot(&mut container.slots, row.clone());
        let total_slots = container
            .slots
            .iter()
            .map(|slot| slot.slot + 1)
            .max()
            .unwrap_or(0);
        if !is_inventory_menu {
            self.write_back_container_player_section(counters, total_slots, &[row]);
        }
        self.update_merchant_offer_count(counters);
        true
    }

    pub(crate) fn apply_container_set_data(
        &mut self,
        counters: &mut WorldCounters,
        packet: ProtocolContainerSetData,
    ) -> bool {
        counters.container_data_updates_received += 1;
        let container = if packet.container_id == INVENTORY_MENU_CONTAINER_ID {
            self.inventory_menu_container()
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
            self.local_quick_craft.reset();
            counters.container_close_updates_applied += 1;
            true
        } else if self.open_container.is_some() {
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

    pub(crate) fn close_for_respawn(&mut self, counters: &mut WorldCounters) -> Option<i32> {
        let server_container_id = self
            .open_container
            .as_ref()
            .map(|container| container.container_id);
        self.local_inventory_open = false;
        self.open_container = None;
        self.local_quick_craft.reset();
        counters.merchant_offers_tracked = 0;
        server_container_id
    }

    pub(crate) fn open_local_inventory(&mut self, ui: &mut ClientUiState) -> bool {
        if self.open_container.is_some() {
            return false;
        }
        ui.replace_screen();
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

    pub(crate) fn recipe_update_listener_context(&self) -> Option<RecipeUpdateListenerContext> {
        if self.local_inventory_open {
            return Some(RecipeUpdateListenerContext {
                container_id: INVENTORY_MENU_CONTAINER_ID,
                layout: RecipeUpdateListenerLayout::Crafting {
                    grid_width: 2,
                    grid_height: 2,
                },
            });
        }
        let container = self.open_container.as_ref()?;
        let layout = match container.menu_type_id? {
            VANILLA_MENU_TYPE_CRAFTING_ID => RecipeUpdateListenerLayout::Crafting {
                grid_width: 3,
                grid_height: 3,
            },
            VANILLA_MENU_TYPE_FURNACE_ID
            | VANILLA_MENU_TYPE_BLAST_FURNACE_ID
            | VANILLA_MENU_TYPE_SMOKER_ID => RecipeUpdateListenerLayout::Furnace {
                fuel_slot_empty: container
                    .slots
                    .iter()
                    .find(|slot| slot.slot == 1)
                    .is_none_or(|slot| item_stack_is_empty(&slot.item)),
            },
            _ => return None,
        };
        Some(RecipeUpdateListenerContext {
            container_id: container.container_id,
            layout,
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
            let active =
                mount_nautilus_can_use_equipment_slots(entity_type_id, &entity.data_values);
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

    /// Applies the client-side half of `BundleContents.Mutable.removeOne`.
    ///
    /// `BundleContents` defaults to empty, so a non-empty value must be an
    /// explicit stack patch. The transient selected index is kept beside the
    /// inventory slot because vanilla's bundle stream codec does not serialize
    /// it. Removing an invalid selection falls back to the first entry and
    /// always clears the selection.
    pub(crate) fn remove_one_local_bundle_item(
        &mut self,
        selected_hotbar_slot: u8,
        hand: InteractionHand,
    ) -> bool {
        let slot_id = match hand {
            InteractionHand::MainHand => {
                if selected_hotbar_slot > 8 {
                    return false;
                }
                i32::from(selected_hotbar_slot)
            }
            InteractionHand::OffHand => VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        };
        let mut slot = self.player_inventory_slot(slot_id);
        if !slot.item.item_id.is_some_and(|item_id| {
            vanilla_item_type(item_id).and_then(|item_type| item_type.use_override)
                == Some(VanillaItemUseOverride::BundleItem)
        }) || slot
            .item
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID)
            || !slot
                .item
                .component_patch
                .added_type_ids
                .contains(&VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID)
        {
            return false;
        }

        let patch = &mut slot.item.component_patch;
        let Some(item_count) = patch.bundle_contents_item_count else {
            return false;
        };
        if item_count == 0
            || item_count != patch.bundle_contents_items.len()
            || patch.bundle_contents_items.is_empty()
        {
            return false;
        }
        let selected = usize::try_from(slot.local_selected_bundle_item_index)
            .ok()
            .filter(|selected| *selected < item_count)
            .unwrap_or(0);
        patch.bundle_contents_items.remove(selected);
        if patch.bundle_contents_items.is_empty() {
            patch
                .added_type_ids
                .retain(|type_id| *type_id != VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID);
            patch.added = patch.added_type_ids.len();
            patch.bundle_contents_item_count = None;
        } else {
            patch.bundle_contents_item_count = Some(patch.bundle_contents_items.len());
        }
        slot.local_selected_bundle_item_index = NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX;
        self.set_player_inventory_slot_and_menu_slot(slot);
        true
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

    pub(crate) fn local_player_has_item_in_tag(
        &self,
        ctx: InventoryCtx<'_>,
        tag: &str,
    ) -> Option<bool> {
        let item_ids = ctx.registries.holder_ids_for_tag("minecraft:item", tag)?;
        Some(
            (0..=VANILLA_PLAYER_INVENTORY_SADDLE_SLOT)
                .filter_map(|slot| self.local_player_inventory_item(slot))
                .filter(|item| item_stack_is_non_empty(item))
                .filter_map(|item| item.item_id)
                .any(|item_id| item_ids.contains(&item_id)),
        )
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

    pub(crate) fn local_selected_main_hand_minimum_attack_charge(
        &self,
        ctx: InventoryCtx<'_>,
    ) -> f32 {
        self.local_item_in_hand(ctx, InteractionHand::MainHand)
            .map_or(0.0, |item| {
                item_stack_minimum_attack_charge(
                    item,
                    &ctx.items.default_item_minimum_attack_charges,
                )
            })
    }

    pub(crate) fn local_item_in_hand_is_enabled(
        &self,
        ctx: InventoryCtx<'_>,
        hand: InteractionHand,
        vanilla_feature_enabled: bool,
    ) -> bool {
        let Some(item) = self.local_item_in_hand(ctx, hand) else {
            return true;
        };
        item.item_id
            .and_then(vanilla_item_type)
            .is_some_and(|_| vanilla_feature_enabled)
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

    pub(crate) fn shrink_local_item_in_hand(
        &mut self,
        ctx: InventoryCtx<'_>,
        counters: &mut WorldCounters,
        hand: InteractionHand,
        amount: i32,
    ) -> bool {
        if amount <= 0 {
            return true;
        }
        let slot_id = match hand {
            InteractionHand::MainHand => {
                let selected_slot = ctx.local_player.selected_hotbar_slot;
                if selected_slot > 8 {
                    return false;
                }
                i32::from(selected_slot)
            }
            InteractionHand::OffHand => VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        };
        let mut slot = self.player_inventory_slot(slot_id);
        if item_stack_is_empty(&slot.item) || slot.item.count < amount {
            return false;
        }
        slot.item.count -= amount;
        normalize_item_stack(&mut slot.item);
        self.set_player_inventory_slot_and_menu_slot(slot);
        self.update_inventory_slot_count(counters);
        true
    }

    pub(crate) fn consume_local_item_in_hand(
        &mut self,
        ctx: InventoryCtx<'_>,
        counters: &mut WorldCounters,
        hand: InteractionHand,
        amount: i32,
    ) -> bool {
        if ctx
            .local_player
            .abilities
            .is_some_and(|abilities| abilities.instabuild)
        {
            return self.local_item_in_hand(ctx, hand).is_some();
        }
        self.shrink_local_item_in_hand(ctx, counters, hand, amount)
    }

    /// Applies the client-observable part of `Mob.usePlayerItem`, including
    /// `DataComponents.USE_REMAINDER`.
    ///
    /// `LocalPlayer` inherits the no-op
    /// `LivingEntity.handleExtraItemsCreatedOnUse`, so a remainder replaces
    /// only a consumed singleton. A remainder created while the original
    /// stack remains nonempty is discarded locally and left to server
    /// inventory correction.
    pub(crate) fn consume_local_mob_player_item(
        &mut self,
        ctx: InventoryCtx<'_>,
        counters: &mut WorldCounters,
        hand: InteractionHand,
        remainder: Option<ProtocolItemStackSummary>,
    ) -> bool {
        let slot_id = match hand {
            InteractionHand::MainHand => {
                let selected_slot = ctx.local_player.selected_hotbar_slot;
                if selected_slot > 8 {
                    return false;
                }
                i32::from(selected_slot)
            }
            InteractionHand::OffHand => VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        };
        let mut slot = self.player_inventory_slot(slot_id);
        if item_stack_is_empty(&slot.item) {
            return false;
        }
        if ctx
            .local_player
            .abilities
            .is_some_and(|abilities| abilities.instabuild)
        {
            return true;
        }

        slot.item.count -= 1;
        if slot.item.count <= 0 {
            slot.item = remainder.unwrap_or_else(ProtocolItemStackSummary::empty);
        }
        normalize_item_stack(&mut slot.item);
        self.set_player_inventory_slot_and_menu_slot(slot);
        self.update_inventory_slot_count(counters);
        true
    }

    pub(crate) fn local_player_equipment_item(
        &self,
        ctx: InventoryCtx<'_>,
        equipment_slot: EquipmentSlot,
    ) -> Option<ProtocolItemStackSummary> {
        let slot_id = local_player_equipment_inventory_slot(ctx, equipment_slot)?;
        Some(self.player_inventory_slot(slot_id).item)
    }

    /// Applies the client-side inventory mutation in
    /// `Equippable.swapWithEquipmentSlot`.
    ///
    /// The returned flag is true only when a stacked survival swap cannot add
    /// the displaced equipment to the inventory and vanilla calls
    /// `Player.drop`, which contributes an extra main-hand swing client-side.
    pub(crate) fn swap_local_item_with_equipment_slot(
        &mut self,
        ctx: InventoryCtx<'_>,
        counters: &mut WorldCounters,
        hand: InteractionHand,
        equipment_slot: EquipmentSlot,
        creative_mode: bool,
    ) -> Option<bool> {
        let hand_slot_id = match hand {
            InteractionHand::MainHand => {
                let selected_slot = ctx.local_player.selected_hotbar_slot;
                (selected_slot <= 8).then_some(i32::from(selected_slot))?
            }
            InteractionHand::OffHand => VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        };
        let equipment_slot_id = local_player_equipment_inventory_slot(ctx, equipment_slot)?;
        if hand_slot_id == equipment_slot_id {
            return None;
        }

        let mut hand_slot = self.player_inventory_slot(hand_slot_id);
        if item_stack_is_empty(&hand_slot.item) {
            return None;
        }
        let mut target_slot = self.player_inventory_slot(equipment_slot_id);
        let displaced = target_slot.item.clone();

        if hand_slot.item.count <= 1 {
            target_slot.item = hand_slot.item.clone();
            hand_slot.item = if creative_mode && item_stack_is_empty(&displaced) {
                hand_slot.item
            } else {
                displaced
            };
            normalize_item_stack(&mut hand_slot.item);
            self.set_player_inventory_slot_and_menu_slot(hand_slot);
            self.set_player_inventory_slot_and_menu_slot(target_slot);
            self.update_inventory_slot_count(counters);
            return Some(false);
        }

        target_slot.item = hand_slot.item.clone();
        target_slot.item.count = 1;
        self.set_player_inventory_slot_and_menu_slot(target_slot);

        let instabuild = ctx
            .local_player
            .abilities
            .is_some_and(|abilities| abilities.instabuild);
        if !instabuild {
            hand_slot.item.count -= 1;
            normalize_item_stack(&mut hand_slot.item);
            self.set_player_inventory_slot_and_menu_slot(hand_slot);
        }

        let added = item_stack_is_empty(&displaced)
            || self.add_local_item_to_player_inventory(ctx, displaced)
            || instabuild;
        self.update_inventory_slot_count(counters);
        Some(!added)
    }

    /// Applies the client-side inventory work in `CompassItem.useOn`.
    ///
    /// The returned flag is true only for the survival/full-inventory branch
    /// where vanilla calls `Player.drop`; on the logical client that call
    /// emits an extra main-hand swing but does not create an item entity.
    pub(crate) fn lock_local_compass_to_lodestone(
        &mut self,
        ctx: InventoryCtx<'_>,
        counters: &mut WorldCounters,
        hand: InteractionHand,
        target: ProtocolLodestoneTargetSummary,
    ) -> Option<bool> {
        let hand_slot_id = match hand {
            InteractionHand::MainHand => {
                let selected_slot = ctx.local_player.selected_hotbar_slot;
                (selected_slot <= 8).then_some(i32::from(selected_slot))?
            }
            InteractionHand::OffHand => VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        };
        let mut hand_slot = self.player_inventory_slot(hand_slot_id);
        if item_stack_is_empty(&hand_slot.item)
            || hand_slot.item.item_id != Some(VANILLA_ITEM_COMPASS_ID)
        {
            return None;
        }

        let mut lodestone_compass = hand_slot.item.clone();
        lodestone_compass.count = 1;
        let patch = &mut lodestone_compass.component_patch;
        patch.removed_type_ids.retain(|component_id| {
            *component_id != VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID
        });
        if !patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID)
        {
            patch
                .added_type_ids
                .push(VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID);
        }
        patch.added = patch.added_type_ids.len();
        patch.lodestone_target = Some(target);
        patch.lodestone_tracked = Some(true);

        let instabuild = ctx
            .local_player
            .abilities
            .is_some_and(|abilities| abilities.instabuild);
        if !instabuild && hand_slot.item.count == 1 {
            hand_slot.item = lodestone_compass;
            self.set_player_inventory_slot_and_menu_slot(hand_slot);
            self.update_inventory_slot_count(counters);
            return Some(false);
        }

        if !instabuild {
            hand_slot.item.count -= 1;
            normalize_item_stack(&mut hand_slot.item);
            self.set_player_inventory_slot_and_menu_slot(hand_slot);
        }

        let added = self.add_local_item_to_player_inventory(ctx, lodestone_compass);

        if !instabuild || added {
            self.update_inventory_slot_count(counters);
        }
        Some(!instabuild && !added)
    }

    /// Applies `ItemUtils.createFilledResult` to one locally held stack.
    ///
    /// The returned flag identifies the survival/full-inventory branch where
    /// vanilla calls `Player.drop`; on the logical client that call contributes
    /// an extra main-hand swing without spawning an item entity.
    pub(crate) fn replace_local_item_with_filled_result(
        &mut self,
        ctx: InventoryCtx<'_>,
        counters: &mut WorldCounters,
        hand: InteractionHand,
        expected_item_id: i32,
        filled: ProtocolItemStackSummary,
    ) -> Option<bool> {
        let hand_slot_id = match hand {
            InteractionHand::MainHand => {
                let selected_slot = ctx.local_player.selected_hotbar_slot;
                (selected_slot <= 8).then_some(i32::from(selected_slot))?
            }
            InteractionHand::OffHand => VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        };
        let mut hand_slot = self.player_inventory_slot(hand_slot_id);
        if item_stack_is_empty(&hand_slot.item)
            || hand_slot.item.item_id != Some(expected_item_id)
            || item_stack_is_empty(&filled)
        {
            return None;
        }

        let instabuild = ctx
            .local_player
            .abilities
            .is_some_and(|abilities| abilities.instabuild);
        if instabuild {
            let already_contains_filled = (0..=VANILLA_PLAYER_INVENTORY_SADDLE_SLOT)
                .map(|slot_id| self.player_inventory_slot(slot_id))
                .any(|slot| {
                    !item_stack_is_empty(&slot.item)
                        && same_item_same_components(&slot.item, &filled)
                });
            if !already_contains_filled && self.add_local_item_to_player_inventory(ctx, filled) {
                self.update_inventory_slot_count(counters);
            }
            return Some(false);
        }

        if hand_slot.item.count == 1 {
            hand_slot.item = filled;
            self.set_player_inventory_slot_and_menu_slot(hand_slot);
            self.update_inventory_slot_count(counters);
            return Some(false);
        }

        hand_slot.item.count -= 1;
        normalize_item_stack(&mut hand_slot.item);
        self.set_player_inventory_slot_and_menu_slot(hand_slot);
        let added = self.add_local_item_to_player_inventory(ctx, filled);
        self.update_inventory_slot_count(counters);
        Some(!added)
    }

    /// Applies `ItemUtils.createFilledResult(..., false)`. Creative mode keeps
    /// the input stack and still attempts to add the filled result, while
    /// survival behavior matches the ordinary helper above. In creative mode,
    /// `Inventory.add` consumes an uninserted result and reports success, so
    /// the returned drop flag is always false.
    pub(crate) fn replace_local_item_with_unlimited_creative_filled_result(
        &mut self,
        ctx: InventoryCtx<'_>,
        counters: &mut WorldCounters,
        hand: InteractionHand,
        expected_item_id: i32,
        filled: ProtocolItemStackSummary,
    ) -> Option<bool> {
        let hand_slot_id = match hand {
            InteractionHand::MainHand => {
                let selected_slot = ctx.local_player.selected_hotbar_slot;
                (selected_slot <= 8).then_some(i32::from(selected_slot))?
            }
            InteractionHand::OffHand => VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        };
        let mut hand_slot = self.player_inventory_slot(hand_slot_id);
        if item_stack_is_empty(&hand_slot.item)
            || hand_slot.item.item_id != Some(expected_item_id)
            || item_stack_is_empty(&filled)
        {
            return None;
        }

        let instabuild = ctx
            .local_player
            .abilities
            .is_some_and(|abilities| abilities.instabuild);
        if instabuild {
            let added = self.add_local_item_to_player_inventory(ctx, filled);
            if added {
                self.update_inventory_slot_count(counters);
            }
            return Some(false);
        }

        if hand_slot.item.count == 1 {
            hand_slot.item = filled;
            self.set_player_inventory_slot_and_menu_slot(hand_slot);
            self.update_inventory_slot_count(counters);
            return Some(false);
        }

        hand_slot.item.count -= 1;
        normalize_item_stack(&mut hand_slot.item);
        self.set_player_inventory_slot_and_menu_slot(hand_slot);
        let added = self.add_local_item_to_player_inventory(ctx, filled);
        self.update_inventory_slot_count(counters);
        Some(!added)
    }

    /// Applies the direct `ItemStack.shrink` + `Inventory.add` sequence used
    /// by `BeehiveBlock` when a glass bottle collects honey.
    ///
    /// Unlike `ItemUtils.createFilledResult`, the input shrinks even for an
    /// instabuild player. A full instabuild inventory silently accepts and
    /// discards the generated stack, matching `Inventory.add`.
    pub(crate) fn replace_local_item_with_forced_filled_result(
        &mut self,
        ctx: InventoryCtx<'_>,
        counters: &mut WorldCounters,
        hand: InteractionHand,
        expected_item_id: i32,
        filled: ProtocolItemStackSummary,
    ) -> Option<bool> {
        let hand_slot_id = match hand {
            InteractionHand::MainHand => {
                let selected_slot = ctx.local_player.selected_hotbar_slot;
                (selected_slot <= 8).then_some(i32::from(selected_slot))?
            }
            InteractionHand::OffHand => VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        };
        let mut hand_slot = self.player_inventory_slot(hand_slot_id);
        if item_stack_is_empty(&hand_slot.item)
            || hand_slot.item.item_id != Some(expected_item_id)
            || item_stack_is_empty(&filled)
        {
            return None;
        }

        if hand_slot.item.count == 1 {
            hand_slot.item = filled;
            self.set_player_inventory_slot_and_menu_slot(hand_slot);
            self.update_inventory_slot_count(counters);
            return Some(false);
        }

        hand_slot.item.count -= 1;
        normalize_item_stack(&mut hand_slot.item);
        self.set_player_inventory_slot_and_menu_slot(hand_slot);
        let added = self.add_local_item_to_player_inventory(ctx, filled);
        self.update_inventory_slot_count(counters);
        let instabuild = ctx
            .local_player
            .abilities
            .is_some_and(|abilities| abilities.instabuild);
        Some(!added && !instabuild)
    }

    pub(crate) fn replace_local_potion_with_glass_bottle(
        &mut self,
        ctx: InventoryCtx<'_>,
        counters: &mut WorldCounters,
        hand: InteractionHand,
    ) -> Option<bool> {
        self.replace_local_item_with_filled_result(
            ctx,
            counters,
            hand,
            VANILLA_ITEM_POTION_ID,
            ProtocolItemStackSummary {
                item_id: Some(VANILLA_ITEM_GLASS_BOTTLE_ID),
                count: 1,
                component_patch: ProtocolDataComponentPatchSummary::default(),
            },
        )
    }

    /// Mirrors `Inventory.add` search order: selected slot, offhand, compatible
    /// non-equipment slots, then empty non-equipment slots. Returns true only
    /// when the complete input stack was inserted.
    fn add_local_item_to_player_inventory(
        &mut self,
        ctx: InventoryCtx<'_>,
        mut item: ProtocolItemStackSummary,
    ) -> bool {
        if item_stack_is_empty(&item) {
            return false;
        }
        let selected_slot = i32::from(ctx.local_player.selected_hotbar_slot.min(8));
        for slot_id in [selected_slot, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT]
            .into_iter()
            .chain(0..36)
        {
            let mut slot = self.player_inventory_slot(slot_id);
            if item_stack_is_empty(&slot.item) || !same_item_same_components(&slot.item, &item) {
                continue;
            }
            let max_stack_size =
                item_stack_max_stack_size(&slot.item, &ctx.items.default_item_max_stack_sizes);
            let moved = item.count.min(max_stack_size - slot.item.count);
            if moved <= 0 {
                continue;
            }
            slot.item.count += moved;
            item.count -= moved;
            self.set_player_inventory_slot_and_menu_slot(slot);
            if item.count == 0 {
                return true;
            }
        }
        for slot_id in 0..36 {
            let slot = self.player_inventory_slot(slot_id);
            if !item_stack_is_empty(&slot.item) {
                continue;
            }
            let mut inserted = item.clone();
            inserted.count = item.count.min(item_stack_max_stack_size(
                &item,
                &ctx.items.default_item_max_stack_sizes,
            ));
            item.count -= inserted.count;
            self.set_player_inventory_slot_and_menu_slot(InventorySlot {
                slot: slot_id,
                item: inserted,
                local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
            });
            if item.count == 0 {
                return true;
            }
        }
        false
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
        ctx: InventoryCtx<'_>,
        request: ContainerClickSlotRequest,
    ) -> Result<ProtocolContainerClick, ContainerClickBuildError> {
        let Some(container) = self.active_container() else {
            return Err(ContainerClickBuildError::NoOpenContainer);
        };
        if !container_click_slot_is_valid(container, request.slot_num) {
            return Err(ContainerClickBuildError::InvalidSlot(request.slot_num));
        }

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

    fn active_container_slot_can_quick_craft(&self, ctx: InventoryCtx<'_>, slot_num: i16) -> bool {
        let Some(container) = self.active_container() else {
            return false;
        };
        let mount_rules = local_mount_slot_rules(
            ctx,
            container.mount,
            &container.slots,
            container
                .mount
                .and_then(|_| self.open_mount_equipment_slot_visibility(ctx)),
        );
        if container.container_id != INVENTORY_MENU_CONTAINER_ID
            && !menu_type_has_known_slot_rules(container.menu_type_id)
            && mount_rules.is_none()
        {
            return false;
        }
        let Some(slot) = container.slots.iter().find(|slot| slot.slot == slot_num) else {
            return false;
        };
        let Some(slot_access) = local_container_slot_access(
            ctx,
            container.container_id,
            container.menu_type_id,
            mount_rules,
            &container.data_values,
            slot_num,
            &slot.item,
            &self.cursor_item,
        ) else {
            return false;
        };
        quick_craft_slot_can_accept(
            &slot.item,
            &self.cursor_item,
            slot_access,
            &ctx.items.default_item_max_stack_sizes,
        )
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
        let mount_rules = local_mount_slot_rules(ctx, mount, &slots_before, mount_equipment_slots);
        let mut slots_after = slots_before.clone();
        let mut merchant_offers_after = merchant_offers.clone();
        let mut cursor_after = self.cursor_item.clone();
        let mut quick_craft_after = self.local_quick_craft.clone();
        let external_offhand_before = (container_id != INVENTORY_MENU_CONTAINER_ID
            && request.input == ProtocolContainerInput::Swap
            && request.button_num == VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT as i8)
            .then(|| {
                self.local_offhand_item()
                    .cloned()
                    .unwrap_or_else(ProtocolItemStackSummary::empty)
            });
        let mut external_offhand_after = external_offhand_before.clone();
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
                ProtocolContainerInput::Pickup => {
                    let slot_access = if request.slot_num >= 0 {
                        let Some(access) = local_container_slot_access(
                            ctx,
                            container_id,
                            menu_type_id,
                            mount_rules,
                            &data_values,
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
                    apply_pickup_click_to_slots(
                        &mut slots_after,
                        &mut cursor_after,
                        request.slot_num,
                        request.button_num,
                        slot_access,
                        &ctx.items.default_item_max_stack_sizes,
                    )
                }
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
                    } else if menu_type_id == Some(VANILLA_MENU_TYPE_CRAFTER_3X3_ID) {
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
                            ctx,
                            &slots_after,
                            request.slot_num,
                            &ctx.items.default_damageable_item_ids,
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
                            ctx,
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
                                &data_values,
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
                        &data_values,
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
                        &mut slots_after,
                        &cursor_after,
                        external_offhand_after
                            .as_mut()
                            .expect("external offhand captured for button 40"),
                        request.slot_num,
                        target_access,
                        ctx.local_player.selected_hotbar_slot,
                        &ctx.items.default_item_max_stack_sizes,
                    )
                }
                ProtocolContainerInput::Swap
                    if (container_id == INVENTORY_MENU_CONTAINER_ID
                        || menu_type_has_known_slot_rules(menu_type_id)
                        || mount_rules.is_some())
                        && swap_button_container_slot(
                            container_id,
                            &slots_after,
                            request.button_num,
                        )
                        .is_some() =>
                {
                    let source_slot_num =
                        swap_button_container_slot(container_id, &slots_after, request.button_num)
                            .expect("guarded swap source slot");
                    let source_item = container_slot_item(&slots_after, source_slot_num)
                        .cloned()
                        .unwrap_or_else(ProtocolItemStackSummary::empty);
                    let Some(target_access) = local_container_slot_access(
                        ctx,
                        container_id,
                        menu_type_id,
                        mount_rules,
                        &data_values,
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
                        &mut slots_after,
                        &cursor_after,
                        request.slot_num,
                        request.button_num,
                        target_access,
                        &ctx.items.default_item_max_stack_sizes,
                    )
                }
                ProtocolContainerInput::QuickCraft
                    if container_id == INVENTORY_MENU_CONTAINER_ID
                        || menu_type_has_known_slot_rules(menu_type_id)
                        || mount_rules.is_some() =>
                {
                    apply_quick_craft_to_slots(
                        ctx,
                        container_id,
                        menu_type_id,
                        mount_rules,
                        &data_values,
                        &mut slots_after,
                        &mut cursor_after,
                        &mut quick_craft_after,
                        request.slot_num,
                        request.button_num,
                        &ctx.items.default_item_max_stack_sizes,
                    )
                }
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
                        &data_values,
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
                        && cursor_after.count
                            < item_stack_max_stack_size(
                                &cursor_after,
                                &ctx.items.default_item_max_stack_sizes,
                            );
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
                                && menu_result_slot_requires_server_authority(
                                    menu_type_id,
                                    slot.slot,
                                    ProtocolContainerInput::PickupAll,
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
        let changed_slots = changed_hashed_slots(ctx, &slots_before, &slots_after)?;
        let carried_item = hashed_stack_from_summary(ctx, &cursor_after)
            .ok_or(ContainerClickBuildError::UnhashableCarriedItem)?;

        let write_back_rows = (container_id != INVENTORY_MENU_CONTAINER_ID)
            .then(|| slots_after.clone())
            .unwrap_or_default();
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
        } else {
            let total_slots = write_back_rows
                .iter()
                .map(|slot| slot.slot + 1)
                .max()
                .unwrap_or(0);
            self.write_back_container_player_section(counters, total_slots, &write_back_rows);
        }
        if external_offhand_after != external_offhand_before {
            self.set_player_inventory_slot_and_menu_slot(InventorySlot {
                slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
                item: external_offhand_after.expect("changed external offhand remains captured"),
                local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
            });
            self.update_inventory_slot_count(counters);
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
        if let Some(item) = self.player_slots.iter().find_map(|slot| {
            (slot.slot == VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT).then_some(&slot.item)
        }) {
            return Some(item);
        }

        self.inventory_menu
            .slots
            .iter()
            .find_map(|slot| (slot.slot == INVENTORY_MENU_OFFHAND_SLOT).then_some(&slot.item))
    }

    pub(crate) fn local_player_has_freeze_immune_wearable(&self, ctx: InventoryCtx<'_>) -> bool {
        [
            VANILLA_PLAYER_INVENTORY_FEET_SLOT,
            VANILLA_PLAYER_INVENTORY_LEGS_SLOT,
            VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
            VANILLA_PLAYER_INVENTORY_HEAD_SLOT,
            VANILLA_PLAYER_INVENTORY_BODY_SLOT,
        ]
        .into_iter()
        .filter_map(|slot| self.local_player_inventory_item(slot))
        .filter(|item| item.count > 0)
        .filter_map(|item| item.item_id)
        .any(|item_id| ctx.items.freeze_immune_wearable_item_ids.contains(&item_id))
    }

    pub(crate) fn local_player_can_walk_on_powder_snow(&self, ctx: InventoryCtx<'_>) -> bool {
        self.local_player_inventory_item(VANILLA_PLAYER_INVENTORY_FEET_SLOT)
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

    /// Vanilla container menus share the player `Inventory` object, so their
    /// trailing 27 main + 9 hotbar slots ARE the backpack. Mirror those rows
    /// back into the player-inventory layer and the survival-menu projection,
    /// which are separate snapshots here.
    fn write_back_container_player_section(
        &mut self,
        counters: &mut WorldCounters,
        total_slots: i16,
        container_slots: &[ContainerSlot],
    ) {
        if total_slots < 36 {
            return;
        }
        let player_start = total_slots - 36;
        let hotbar_start = total_slots - 9;
        for slot in container_slots {
            if slot.slot < player_start || slot.slot >= total_slots {
                continue;
            }
            let inventory_slot = if slot.slot >= hotbar_start {
                i32::from(slot.slot - hotbar_start)
            } else {
                9 + i32::from(slot.slot - player_start)
            };
            set_inventory_slot(
                &mut self.player_slots,
                InventorySlot {
                    slot: inventory_slot,
                    item: slot.item.clone(),
                    local_selected_bundle_item_index: slot.local_selected_bundle_item_index,
                },
            );
            if let Some(menu_slot) = inventory_slot_to_inventory_menu_slot(inventory_slot) {
                set_container_slot(
                    &mut self.inventory_menu.slots,
                    ContainerSlot {
                        slot: menu_slot,
                        item: slot.item.clone(),
                        local_selected_bundle_item_index: slot.local_selected_bundle_item_index,
                    },
                );
            }
        }
        self.update_inventory_slot_count(counters);
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

fn local_player_equipment_inventory_slot(
    ctx: InventoryCtx<'_>,
    equipment_slot: EquipmentSlot,
) -> Option<i32> {
    if equipment_slot == EquipmentSlot::MainHand {
        let selected_slot = ctx.local_player.selected_hotbar_slot;
        return (selected_slot <= 8).then_some(i32::from(selected_slot));
    }
    VANILLA_EQUIPMENT_SLOTS
        .iter()
        .find(|entry| entry.slot == equipment_slot)?
        .player_inventory_slot
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

macro_rules! inventory_ctx {
    ($store:expr) => {
        InventoryCtx {
            items: &$store.items,
            local_player: &$store.local_player,
            entities: &$store.entities,
            recipes: &$store.recipes,
            registries: &$store.registries,
        }
    };
}

/// Facade delegation: the inventory method group lives on
/// [`InventoryState`] (and the item default tables on `ItemProfiles`);
/// `WorldStore` keeps the public signatures and forwards, splitting off
/// cross-domain borrows explicitly.
impl WorldStore {
    pub fn apply_set_player_inventory(&mut self, packet: ProtocolSetPlayerInventory) -> bool {
        let applied = self
            .inventory
            .apply_set_player_inventory(&mut self.counters, packet);
        if applied {
            self.close_invalid_writable_book_screen();
        }
        applied
    }

    pub fn apply_set_cursor_item(&mut self, packet: ProtocolSetCursorItem) {
        self.inventory
            .apply_set_cursor_item(&mut self.counters, packet)
    }

    pub fn apply_open_screen(&mut self, packet: ProtocolOpenScreen) {
        self.inventory
            .apply_open_screen(&mut self.counters, &mut self.client_ui, packet)
    }

    pub fn apply_container_set_content(&mut self, packet: ProtocolContainerSetContent) -> bool {
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

    pub fn apply_container_set_slot(&mut self, packet: ProtocolContainerSetSlot) -> bool {
        self.inventory
            .apply_container_set_slot(&mut self.counters, packet)
    }

    pub fn apply_container_set_data(&mut self, packet: ProtocolContainerSetData) -> bool {
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

    pub(crate) fn close_container_for_respawn(&mut self) -> Option<i32> {
        self.inventory.close_for_respawn(&mut self.counters)
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

    pub(crate) fn recipe_update_listener_context(&self) -> Option<RecipeUpdateListenerContext> {
        self.inventory.recipe_update_listener_context()
    }

    pub fn open_container_data_value(&self, id: i16) -> Option<i16> {
        self.inventory.open_container_data_value(id)
    }

    /// Vanilla `LoomMenu.getSelectablePatterns`: banner and dye inputs gate the
    /// list; an empty pattern slot resolves `BannerPatternTags.NO_ITEM_REQUIRED`,
    /// while a pattern item resolves its effective `PROVIDES_BANNER_PATTERNS`
    /// holder set in current session order. Missing or invalid holder state
    /// fails closed to an empty list.
    pub fn loom_selectable_banner_pattern_ids(&self) -> Option<Vec<i32>> {
        let container = self.inventory.open_container.as_ref()?;
        if container.menu_type_id != Some(VANILLA_MENU_TYPE_LOOM_ID) {
            return None;
        }
        let slot_item = |slot_num| {
            container
                .slots
                .iter()
                .find(|slot| slot.slot == slot_num)
                .map(|slot| &slot.item)
        };
        if slot_item(LOOM_BANNER_SLOT).is_none_or(item_stack_is_empty)
            || slot_item(LOOM_DYE_SLOT).is_none_or(item_stack_is_empty)
        {
            return None;
        }

        let access = self.registries.frozen_access();
        let pattern_stack = slot_item(LOOM_PATTERN_SLOT);
        let resolved = if pattern_stack.is_none_or(item_stack_is_empty) {
            access.holder_set_by_tag(BANNER_PATTERN_REGISTRY, LOOM_NO_ITEM_REQUIRED_PATTERN_TAG)
        } else {
            let pattern_stack = pattern_stack?;
            if !self.item_stack_has_effective_component(
                pattern_stack,
                VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
            ) {
                return Some(Vec::new());
            }
            let explicitly_patched = pattern_stack
                .component_patch
                .added_type_ids
                .contains(&VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID);
            let effective = self.effective_item_stack(pattern_stack);
            let patch = &effective.component_patch;
            match patch.provides_banner_pattern_tag.as_deref() {
                Some(_)
                    if explicitly_patched
                        && !patch.provides_banner_pattern_registry_ids.is_empty() =>
                {
                    return Some(Vec::new());
                }
                Some(tag) => access.holder_set_by_tag(BANNER_PATTERN_REGISTRY, tag),
                None => access.holder_set_by_holder_ids(
                    BANNER_PATTERN_REGISTRY,
                    &patch.provides_banner_pattern_registry_ids,
                ),
            }
        };

        Some(
            resolved
                .map(|set| set.entries.iter().map(|entry| entry.holder_id).collect())
                .unwrap_or_default(),
        )
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

    pub(crate) fn local_player_has_item_in_tag(&self, tag: &str) -> Option<bool> {
        self.inventory
            .local_player_has_item_in_tag(inventory_ctx!(self), tag)
    }

    /// Vanilla `Consumable.canConsume`: FOOD only gates an effective
    /// CONSUMABLE, creative/invulnerable players and `can_always_eat` bypass
    /// the full-food check, and an absent health packet uses FoodData's
    /// initial full level.
    pub fn local_item_in_hand_can_start_consuming(&self, hand: InteractionHand) -> bool {
        let Some(stack) = self.local_item_in_hand(hand) else {
            return true;
        };
        if !self
            .item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID)
            || !self.item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID)
        {
            return true;
        }

        let effective = self.effective_item_stack(stack);
        self.local_player
            .abilities
            .is_some_and(|abilities| abilities.invulnerable)
            || effective.component_patch.food_can_always_eat
            || self
                .local_player
                .health
                .is_some_and(|health| health.food < 20)
    }

    pub fn local_selected_main_hand_has_piercing_weapon(&self) -> bool {
        self.inventory
            .local_selected_main_hand_has_piercing_weapon(inventory_ctx!(self))
    }

    pub fn local_selected_main_hand_attack_range(&self) -> Option<ItemAttackRange> {
        self.inventory
            .local_selected_main_hand_attack_range(inventory_ctx!(self))
    }

    pub fn local_selected_main_hand_minimum_attack_charge(&self) -> f32 {
        self.inventory
            .local_selected_main_hand_minimum_attack_charge(inventory_ctx!(self))
    }

    pub fn local_selected_main_hand_is_enabled(&self) -> bool {
        self.local_item_in_hand_is_enabled(InteractionHand::MainHand)
    }

    pub fn local_item_in_hand_is_enabled(&self, hand: InteractionHand) -> bool {
        self.inventory.local_item_in_hand_is_enabled(
            inventory_ctx!(self),
            hand,
            self.is_feature_enabled("minecraft:vanilla"),
        )
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

    pub fn trigger_local_player_swing(&mut self, hand: InteractionHand) -> bool {
        let Some(id) = self.local_player_id else {
            return false;
        };
        let off_hand = hand == InteractionHand::OffHand;
        let duration = self.entity_held_item_swing_duration(id, off_hand);
        self.entities
            .trigger_client_animation_swing(id, off_hand, duration)
            .is_some()
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

    pub(crate) fn shrink_local_item_in_hand(&mut self, hand: InteractionHand, amount: i32) -> bool {
        self.inventory.shrink_local_item_in_hand(
            inventory_ctx!(self),
            &mut self.counters,
            hand,
            amount,
        )
    }

    pub(crate) fn consume_local_item_in_hand(
        &mut self,
        hand: InteractionHand,
        amount: i32,
    ) -> bool {
        self.inventory.consume_local_item_in_hand(
            inventory_ctx!(self),
            &mut self.counters,
            hand,
            amount,
        )
    }

    pub(crate) fn consume_local_mob_player_item(
        &mut self,
        hand: InteractionHand,
        remainder: Option<ProtocolItemStackSummary>,
    ) -> bool {
        self.inventory.consume_local_mob_player_item(
            inventory_ctx!(self),
            &mut self.counters,
            hand,
            remainder,
        )
    }

    pub(crate) fn local_player_equipment_item(
        &self,
        equipment_slot: EquipmentSlot,
    ) -> Option<ProtocolItemStackSummary> {
        self.inventory
            .local_player_equipment_item(inventory_ctx!(self), equipment_slot)
    }

    pub(crate) fn swap_local_item_with_equipment_slot(
        &mut self,
        hand: InteractionHand,
        equipment_slot: EquipmentSlot,
    ) -> Option<bool> {
        let creative_mode = self.local_player_is_creative();
        self.inventory.swap_local_item_with_equipment_slot(
            inventory_ctx!(self),
            &mut self.counters,
            hand,
            equipment_slot,
            creative_mode,
        )
    }

    pub(crate) fn lock_local_compass_to_lodestone(
        &mut self,
        hand: InteractionHand,
        target: ProtocolLodestoneTargetSummary,
    ) -> Option<bool> {
        self.inventory.lock_local_compass_to_lodestone(
            inventory_ctx!(self),
            &mut self.counters,
            hand,
            target,
        )
    }

    pub(crate) fn replace_local_potion_with_glass_bottle(
        &mut self,
        hand: InteractionHand,
    ) -> Option<bool> {
        self.inventory.replace_local_potion_with_glass_bottle(
            inventory_ctx!(self),
            &mut self.counters,
            hand,
        )
    }

    pub(crate) fn replace_local_item_with_filled_result(
        &mut self,
        hand: InteractionHand,
        expected_item_id: i32,
        filled: ProtocolItemStackSummary,
    ) -> Option<bool> {
        self.inventory.replace_local_item_with_filled_result(
            inventory_ctx!(self),
            &mut self.counters,
            hand,
            expected_item_id,
            filled,
        )
    }

    pub(crate) fn replace_local_item_with_unlimited_creative_filled_result(
        &mut self,
        hand: InteractionHand,
        expected_item_id: i32,
        filled: ProtocolItemStackSummary,
    ) -> Option<bool> {
        self.inventory
            .replace_local_item_with_unlimited_creative_filled_result(
                inventory_ctx!(self),
                &mut self.counters,
                hand,
                expected_item_id,
                filled,
            )
    }

    pub(crate) fn replace_local_item_with_forced_filled_result(
        &mut self,
        hand: InteractionHand,
        expected_item_id: i32,
        filled: ProtocolItemStackSummary,
    ) -> Option<bool> {
        self.inventory.replace_local_item_with_forced_filled_result(
            inventory_ctx!(self),
            &mut self.counters,
            hand,
            expected_item_id,
            filled,
        )
    }

    pub(crate) fn add_local_item_from_block_interaction(
        &mut self,
        item: ProtocolItemStackSummary,
    ) -> bool {
        let added = self
            .inventory
            .add_local_item_to_player_inventory(inventory_ctx!(self), item);
        self.inventory
            .update_inventory_slot_count(&mut self.counters);
        added
    }

    /// Vanilla `LivingEntity.canGlideUsing`: any equipment slot may carry the
    /// usable glider, but the effective EQUIPPABLE slot must match and the
    /// next durability point must not break the stack.
    pub fn local_player_has_usable_glider(&self) -> bool {
        VANILLA_EQUIPMENT_SLOTS.iter().any(|slot| {
            let stack = match slot.slot {
                EquipmentSlot::MainHand => self.local_item_in_hand(InteractionHand::MainHand),
                EquipmentSlot::OffHand => self.local_item_in_hand(InteractionHand::OffHand),
                _ => slot
                    .player_inventory_slot
                    .and_then(|slot| self.inventory.local_player_inventory_item(slot))
                    .filter(|stack| item_stack_is_non_empty(stack)),
            };
            let Some(stack) = stack else {
                return false;
            };
            self.item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID)
                && self.effective_item_equipment_slot(stack) == Some(slot.slot)
                && !self.item_stack_next_damage_will_break(stack)
        })
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

    fn effective_item_equipment_slot(
        &self,
        stack: &ProtocolItemStackSummary,
    ) -> Option<EquipmentSlot> {
        if !self
            .item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
        {
            return None;
        }
        if stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
        {
            return vanilla_equipment_slot_by_id(
                stack.component_patch.equippable.as_ref()?.slot_id,
            )
            .map(|slot| slot.slot);
        }
        stack
            .item_id
            .and_then(|item_id| {
                self.items
                    .default_item_equipment_slots
                    .get(&item_id)
                    .copied()
            })
            .map(ItemEquipmentSlot::protocol_slot)
    }

    fn item_stack_next_damage_will_break(&self, stack: &ProtocolItemStackSummary) -> bool {
        if !self
            .item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID)
            || !self
                .item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID)
            || self.item_stack_has_effective_component(
                stack,
                VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
            )
        {
            return false;
        }

        let max_damage = if stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID)
        {
            stack.component_patch.max_damage
        } else {
            stack
                .item_id
                .and_then(|item_id| self.items.default_item_max_damage.get(&item_id).copied())
        };
        let Some(max_damage) = max_damage.filter(|max_damage| *max_damage > 0) else {
            return false;
        };
        let damage = if stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID)
        {
            stack.component_patch.damage.unwrap_or_default()
        } else {
            0
        }
        .max(0)
        .min(max_damage);
        damage >= max_damage - 1
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

    pub fn set_furnace_fuel_profile(
        &mut self,
        item_ids: BTreeSet<i32>,
        rules: Vec<crate::FurnaceFuelRule>,
    ) {
        self.items.set_furnace_fuel_profile(item_ids, rules)
    }

    pub fn furnace_fuel_item_ids(&self) -> &BTreeSet<i32> {
        &self.items.furnace_fuel_item_ids
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

    pub fn set_default_item_minimum_attack_charges(&mut self, charges: BTreeMap<i32, f32>) {
        self.items.set_default_item_minimum_attack_charges(charges)
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

    pub fn set_default_item_consumables(
        &mut self,
        consumables: BTreeMap<i32, bbb_protocol::packets::ConsumableSummary>,
    ) {
        self.items.set_default_item_consumables(consumables)
    }

    pub fn set_default_item_food_properties(&mut self, foods: BTreeMap<i32, ItemFoodProperties>) {
        self.items.set_default_item_food_properties(foods)
    }

    pub fn set_default_item_equipment_slots(
        &mut self,
        equipment_slots: BTreeMap<i32, ItemEquipmentSlot>,
    ) {
        self.items.set_default_item_equipment_slots(equipment_slots)
    }

    pub fn set_default_swappable_equipment_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items
            .set_default_swappable_equipment_item_ids(item_ids)
    }

    pub fn set_default_player_swappable_equipment_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items
            .set_default_player_swappable_equipment_item_ids(item_ids)
    }

    pub fn set_default_shearable_equipment_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items
            .set_default_shearable_equipment_item_ids(item_ids)
    }

    /// Installs the item id → humanoid armor material table (from the item registry), used to project
    /// worn armor onto entity render sources for the `HumanoidArmorLayer` overlay.
    pub fn set_item_armor_materials(
        &mut self,
        armor_materials: BTreeMap<i32, crate::entities::ArmorMaterialKind>,
    ) {
        self.items.set_item_armor_materials(armor_materials)
    }

    pub fn set_default_item_wings_equipment_assets(
        &mut self,
        equipment_assets: BTreeMap<i32, String>,
    ) {
        self.items
            .set_default_item_wings_equipment_assets(equipment_assets)
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

    pub fn set_default_happy_ghast_harness_colors(
        &mut self,
        harness_colors: BTreeMap<i32, crate::entities::HappyGhastHarnessColor>,
    ) {
        self.items
            .set_default_happy_ghast_harness_colors(harness_colors)
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
        self.inventory
            .build_container_click_slot(inventory_ctx!(self), request)
    }

    pub fn active_container_slot_can_quick_craft(&self, slot_num: i16) -> bool {
        self.inventory
            .active_container_slot_can_quick_craft(inventory_ctx!(self), slot_num)
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
        VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT => Some(INVENTORY_MENU_OFFHAND_SLOT),
        _ => None,
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
