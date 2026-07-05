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

pub(crate) const VANILLA_DEFAULT_MAX_STACK_SIZE: i32 = 64;
pub(crate) const VANILLA_ABSOLUTE_MAX_STACK_SIZE: i32 = 99;

pub(crate) fn clamp_vanilla_item_max_stack_size(size: i32) -> i32 {
    size.clamp(1, VANILLA_ABSOLUTE_MAX_STACK_SIZE)
}

impl ItemProfiles {
    pub(crate) fn set_default_item_max_stack_sizes(&mut self, max_stack_sizes: BTreeMap<i32, i32>) {
        self.default_item_max_stack_sizes = max_stack_sizes
            .into_iter()
            .filter(|(item_id, size)| *item_id >= 0 && *size > 0)
            .map(|(item_id, size)| (item_id, clamp_vanilla_item_max_stack_size(size)))
            .collect();
    }

    pub(crate) fn set_default_item_max_damage(&mut self, max_damage: BTreeMap<i32, i32>) {
        self.default_item_max_damage = max_damage
            .into_iter()
            .filter(|(item_id, max_damage)| *item_id >= 0 && *max_damage > 0)
            .collect();
    }

    pub(crate) fn set_default_item_crafting_remainders(&mut self, remainders: BTreeMap<i32, i32>) {
        self.default_item_crafting_remainders_known = true;
        self.default_item_crafting_remainders = remainders
            .into_iter()
            .filter(|(item_id, remainder_id)| *item_id >= 0 && *remainder_id >= 0)
            .collect();
    }

    pub(crate) fn set_recipe_specific_crafting_remainder_item_ids(
        &mut self,
        item_ids: BTreeSet<i32>,
    ) {
        self.recipe_specific_crafting_remainder_item_ids = filter_item_ids(item_ids);
    }

    pub(crate) fn item_max_stack_size_for_protocol_id(&self, item_id: i32) -> i32 {
        self.default_item_max_stack_sizes
            .get(&item_id)
            .copied()
            .map(clamp_vanilla_item_max_stack_size)
            .unwrap_or(VANILLA_DEFAULT_MAX_STACK_SIZE)
    }

    /// Vanilla `ItemStack.getMaxDamage` reads `DataComponents.MAX_DAMAGE` via
    /// `getOrDefault`, so a damageable item whose patch omits `max_damage`
    /// (the common case: only `damage` is patched) still falls back to the
    /// item's registry default.
    pub(crate) fn item_max_damage_for_protocol_id(&self, item_id: i32) -> Option<i32> {
        self.default_item_max_damage.get(&item_id).copied()
    }

    pub(crate) fn set_furnace_fuel_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.furnace_fuel_item_ids = filter_item_ids(item_ids);
    }

    pub(crate) fn set_brewing_potion_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.brewing_potion_item_ids = filter_item_ids(item_ids);
    }

    pub(crate) fn set_brewing_ingredient_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.brewing_ingredient_item_ids = filter_item_ids(item_ids);
    }

    pub(crate) fn set_enchantment_lapis_lazuli_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.enchantment_lapis_lazuli_item_ids = filter_item_ids(item_ids);
    }

    pub(crate) fn set_cartography_additional_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.cartography_additional_item_ids = filter_item_ids(item_ids);
    }

    pub(crate) fn set_default_damageable_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.default_damageable_item_ids = filter_item_ids(item_ids);
    }

    pub(crate) fn set_freeze_immune_wearable_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.freeze_immune_wearable_item_ids = filter_item_ids(item_ids);
    }

    pub(crate) fn set_powder_snow_walkable_foot_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.powder_snow_walkable_foot_item_ids = filter_item_ids(item_ids);
    }

    pub(crate) fn set_default_piercing_weapon_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.default_piercing_weapon_item_ids = filter_item_ids(item_ids);
    }

    pub(crate) fn set_default_item_attack_ranges(
        &mut self,
        attack_ranges: BTreeMap<i32, ItemAttackRange>,
    ) {
        self.default_item_attack_ranges = filter_item_id_keys(attack_ranges);
    }

    pub(crate) fn set_default_item_swing_animation_durations(
        &mut self,
        durations: BTreeMap<i32, i32>,
    ) {
        self.default_item_swing_animation_durations = durations
            .into_iter()
            .filter(|(item_id, duration)| *item_id >= 0 && *duration > 0)
            .collect();
    }

    pub(crate) fn set_default_item_use_effects(
        &mut self,
        use_effects: BTreeMap<i32, ItemUseEffects>,
    ) {
        self.default_item_use_effects = filter_item_id_keys(use_effects);
    }

    pub(crate) fn set_default_item_equipment_slots(
        &mut self,
        equipment_slots: BTreeMap<i32, ItemEquipmentSlot>,
    ) {
        self.default_item_equipment_slots = filter_item_id_keys(equipment_slots);
    }

    pub(crate) fn set_item_armor_materials(
        &mut self,
        armor_materials: BTreeMap<i32, crate::entities::ArmorMaterialKind>,
    ) {
        self.default_item_armor_materials = filter_item_id_keys(armor_materials);
    }

    pub(crate) fn set_default_mount_body_armor_kinds(
        &mut self,
        armor_kinds: BTreeMap<i32, MountArmorSlotKind>,
    ) {
        self.default_mount_body_armor_kinds = filter_item_id_keys(armor_kinds);
    }

    pub(crate) fn set_default_llama_body_decor_colors(
        &mut self,
        decor_colors: BTreeMap<i32, crate::entities::LlamaBodyDecorColor>,
    ) {
        self.default_llama_body_decor_colors = filter_item_id_keys(decor_colors);
    }

    pub(crate) fn set_default_nautilus_body_armor_materials(
        &mut self,
        armor_materials: BTreeMap<i32, crate::entities::ArmorMaterialKind>,
    ) {
        self.default_nautilus_body_armor_materials = filter_item_id_keys(armor_materials);
    }

    pub(crate) fn set_default_horse_body_armor_materials(
        &mut self,
        armor_materials: BTreeMap<i32, crate::entities::ArmorMaterialKind>,
    ) {
        self.default_horse_body_armor_materials = filter_item_id_keys(armor_materials);
    }

    pub(crate) fn set_default_wolf_body_armor_materials(
        &mut self,
        armor_materials: BTreeMap<i32, crate::entities::ArmorMaterialKind>,
    ) {
        self.default_wolf_body_armor_materials = filter_item_id_keys(armor_materials);
    }
}

fn filter_item_ids(item_ids: BTreeSet<i32>) -> BTreeSet<i32> {
    item_ids
        .into_iter()
        .filter(|item_id| *item_id >= 0)
        .collect()
}

fn filter_item_id_keys<V>(values: BTreeMap<i32, V>) -> BTreeMap<i32, V> {
    values
        .into_iter()
        .filter(|(item_id, _)| *item_id >= 0)
        .collect()
}
