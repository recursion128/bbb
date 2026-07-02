//! Vanilla item and block default profiles.
//!
//! These tables are derived from the vanilla 26.1 item/block registries by
//! the native asset pipeline and pushed into the world store at startup.
//! They are canonical lookup state, not runtime caches.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    ItemAttackRange, ItemEquipmentSlot, ItemUseEffects, MountArmorSlotKind,
    WorldBlockDestroyProfile, WorldBlockSoundProfile, WorldItemMiningProfile,
};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ItemProfiles {
    #[serde(default)]
    pub(crate) default_item_max_stack_sizes: BTreeMap<i32, i32>,
    #[serde(default)]
    pub(crate) default_item_max_damage: BTreeMap<i32, i32>,
    #[serde(default)]
    pub(crate) default_item_crafting_remainders: BTreeMap<i32, i32>,
    #[serde(default)]
    pub(crate) default_item_crafting_remainders_known: bool,
    #[serde(default)]
    pub(crate) recipe_specific_crafting_remainder_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) default_item_equipment_slots: BTreeMap<i32, ItemEquipmentSlot>,
    /// Item protocol id → humanoid armor material (`HumanoidArmorLayer` equipment asset), for
    /// projecting worn armor onto the entity render source.
    #[serde(default)]
    pub(crate) default_item_armor_materials: BTreeMap<i32, crate::entities::ArmorMaterialKind>,
    #[serde(default)]
    pub(crate) default_mount_body_armor_kinds: BTreeMap<i32, MountArmorSlotKind>,
    #[serde(default)]
    pub(crate) default_llama_body_decor_colors: BTreeMap<i32, crate::entities::LlamaBodyDecorColor>,
    #[serde(default)]
    pub(crate) default_nautilus_body_armor_materials:
        BTreeMap<i32, crate::entities::ArmorMaterialKind>,
    #[serde(default)]
    pub(crate) default_horse_body_armor_materials:
        BTreeMap<i32, crate::entities::ArmorMaterialKind>,
    #[serde(default)]
    pub(crate) default_wolf_body_armor_materials: BTreeMap<i32, crate::entities::ArmorMaterialKind>,
    #[serde(default)]
    pub(crate) default_item_attack_ranges: BTreeMap<i32, ItemAttackRange>,
    #[serde(default)]
    pub(crate) default_item_swing_animation_durations: BTreeMap<i32, i32>,
    #[serde(default)]
    pub(crate) default_item_use_effects: BTreeMap<i32, ItemUseEffects>,
    #[serde(default)]
    pub(crate) default_damageable_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) default_piercing_weapon_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) furnace_fuel_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) brewing_potion_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) brewing_ingredient_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) enchantment_lapis_lazuli_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) cartography_additional_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) freeze_immune_wearable_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) powder_snow_walkable_foot_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) default_item_mining_profiles: BTreeMap<i32, WorldItemMiningProfile>,
    #[serde(default)]
    pub(crate) default_block_destroy_profiles: BTreeMap<String, WorldBlockDestroyProfile>,
    #[serde(default)]
    pub(crate) default_block_sound_profiles: BTreeMap<String, WorldBlockSoundProfile>,
}
