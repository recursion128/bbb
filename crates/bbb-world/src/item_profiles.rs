//! Vanilla item and block default profiles.
//!
//! These tables are derived from the vanilla 26.1 item/block registries by
//! the native asset pipeline and pushed into the world store at startup.
//! They are canonical lookup state, not runtime caches.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use bbb_pack::RegistryDependentItemComponentInitializer;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID;
use bbb_protocol::data_component_types::{
    copy_data_component_value_summary, VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
    VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID, VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID,
    VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
    VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
    VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
    VANILLA_DATA_COMPONENT_LOCK_TYPE_ID, VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
    VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
    VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID, VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
};
use bbb_protocol::item_types::{
    VANILLA_ITEM_ABSOLUTE_MAX_STACK_SIZE, VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE,
};
use bbb_protocol::packets::{
    ConsumableSummary, DataComponentPatchSummary, ItemStackSummary as ProtocolItemStackSummary,
};

use crate::{
    ItemAttackRange, ItemEquipmentSlot, ItemFoodProperties, ItemUseEffects, MountArmorSlotKind,
    PublicationShared, WorldBlockDestroyProfile, WorldBlockSoundProfile, WorldItemMiningProfile,
};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ItemProfiles {
    /// Fixed `Items.java` initializer rules. Their holder/tag values are
    /// resolved only by the registry candidate owner below.
    #[serde(default)]
    pub(crate) registry_dependent_component_initializer_rules:
        PublicationShared<BTreeMap<i32, Vec<RegistryDependentItemComponentInitializer>>>,
    /// Persistent codec inputs retained by resource id/tag until a frozen
    /// registry generation can resolve every item prototype.
    #[serde(default)]
    pub(crate) default_component_codec_values:
        PublicationShared<BTreeMap<i32, BTreeMap<i32, serde_json::Value>>>,
    /// Complete prototype patches resolved from the same frozen registry
    /// revision that is currently committed in `WorldStore`.
    #[serde(default, alias = "registry_dependent_item_defaults")]
    pub(crate) default_item_component_patches:
        PublicationShared<BTreeMap<i32, DataComponentPatchSummary>>,
    /// Complete source-derived prototype component membership by item protocol id.
    #[serde(default, with = "publication_i32_map")]
    pub(crate) default_component_type_ids: PublicationShared<BTreeMap<i32, BTreeSet<i32>>>,
    /// Source-derived `DataComponents.DYE` prototype value by item protocol id.
    #[serde(default, with = "publication_i32_map")]
    pub(crate) default_dye_color_ids: PublicationShared<BTreeMap<i32, i32>>,
    /// Source-derived `DataComponents.POTION_DURATION_SCALE` prototype float bits.
    #[serde(default, with = "publication_i32_map")]
    pub(crate) default_potion_duration_scale_bits: PublicationShared<BTreeMap<i32, u32>>,
    #[serde(default, with = "publication_i32_map")]
    pub(crate) default_use_cooldown_seconds_bits: PublicationShared<BTreeMap<i32, u32>>,
    #[serde(default)]
    pub(crate) default_item_max_stack_sizes: PublicationShared<BTreeMap<i32, i32>>,
    #[serde(default)]
    pub(crate) default_item_max_damage: PublicationShared<BTreeMap<i32, i32>>,
    #[serde(default)]
    pub(crate) default_item_crafting_remainders: PublicationShared<BTreeMap<i32, i32>>,
    #[serde(default)]
    pub(crate) default_item_crafting_remainders_known: bool,
    #[serde(default)]
    pub(crate) default_item_equipment_slots: PublicationShared<BTreeMap<i32, ItemEquipmentSlot>>,
    /// Source-derived prototype `Equippable.swappable` membership.
    #[serde(default)]
    pub(crate) default_swappable_equipment_item_ids: PublicationShared<BTreeSet<i32>>,
    /// Swappable prototype `Equippable` items whose allowed-entity set accepts
    /// the player entity type.
    #[serde(default)]
    pub(crate) default_player_swappable_equipment_item_ids: PublicationShared<BTreeSet<i32>>,
    /// Item protocol ids whose source-derived prototype `Equippable` component has
    /// `can_be_sheared=true`.
    #[serde(default)]
    pub(crate) default_shearable_equipment_item_ids: PublicationShared<BTreeSet<i32>>,
    /// Generation-local compact equipment asset ids for prototype
    /// `Equippable.asset_id` values.
    #[serde(default)]
    pub(crate) default_item_equipment_assets:
        PublicationShared<BTreeMap<i32, crate::entities::EquipmentAssetId>>,
    #[serde(default)]
    pub(crate) equipment_asset_generation: u64,
    /// Normalized equipment asset names indexed by
    /// [`EquipmentAssetId`](crate::entities::EquipmentAssetId).
    #[serde(default)]
    pub(crate) equipment_asset_names: PublicationShared<Vec<String>>,
    /// Item protocol id → humanoid armor material (`HumanoidArmorLayer` equipment asset), for
    /// projecting worn armor onto the entity render source.
    #[serde(default)]
    pub(crate) default_item_armor_materials:
        PublicationShared<BTreeMap<i32, crate::entities::ArmorMaterialKind>>,
    /// Item protocol id → normalized equipment asset id for assets with a WINGS layer. The world
    /// combines this pack-derived identity with synchronized trim registries when projecting elytra.
    #[serde(default)]
    pub(crate) default_item_wings_equipment_assets: PublicationShared<BTreeMap<i32, String>>,
    #[serde(default)]
    pub(crate) default_mount_body_armor_kinds: PublicationShared<BTreeMap<i32, MountArmorSlotKind>>,
    #[serde(default)]
    pub(crate) default_llama_body_decor_colors:
        PublicationShared<BTreeMap<i32, crate::entities::LlamaBodyDecorColor>>,
    #[serde(default)]
    pub(crate) default_happy_ghast_harness_colors:
        PublicationShared<BTreeMap<i32, crate::entities::HappyGhastHarnessColor>>,
    #[serde(default)]
    pub(crate) default_nautilus_body_armor_materials:
        PublicationShared<BTreeMap<i32, crate::entities::ArmorMaterialKind>>,
    #[serde(default)]
    pub(crate) default_horse_body_armor_materials:
        PublicationShared<BTreeMap<i32, crate::entities::ArmorMaterialKind>>,
    #[serde(default)]
    pub(crate) default_wolf_body_armor_materials:
        PublicationShared<BTreeMap<i32, crate::entities::ArmorMaterialKind>>,
    #[serde(default)]
    pub(crate) default_item_attack_ranges: PublicationShared<BTreeMap<i32, ItemAttackRange>>,
    #[serde(default)]
    pub(crate) default_item_minimum_attack_charges: PublicationShared<BTreeMap<i32, f32>>,
    #[serde(default)]
    pub(crate) default_item_swing_animation_durations: PublicationShared<BTreeMap<i32, i32>>,
    #[serde(default)]
    pub(crate) default_item_use_effects: PublicationShared<BTreeMap<i32, ItemUseEffects>>,
    #[serde(default)]
    pub(crate) default_item_consumables: PublicationShared<BTreeMap<i32, ConsumableSummary>>,
    #[serde(default)]
    pub(crate) default_item_food_properties: PublicationShared<BTreeMap<i32, ItemFoodProperties>>,
    #[serde(default)]
    pub(crate) default_damageable_item_ids: PublicationShared<BTreeSet<i32>>,
    #[serde(default)]
    pub(crate) default_piercing_weapon_item_ids: PublicationShared<BTreeSet<i32>>,
    #[serde(default)]
    pub(crate) furnace_fuel_item_ids: PublicationShared<BTreeSet<i32>>,
    #[serde(default)]
    pub(crate) furnace_fuel_rules: PublicationShared<Vec<FurnaceFuelRule>>,
    #[serde(default)]
    pub(crate) brewing_potion_item_ids: PublicationShared<BTreeSet<i32>>,
    #[serde(default)]
    pub(crate) brewing_ingredient_item_ids: PublicationShared<BTreeSet<i32>>,
    #[serde(default)]
    pub(crate) enchantment_lapis_lazuli_item_ids: PublicationShared<BTreeSet<i32>>,
    #[serde(default)]
    pub(crate) cartography_additional_item_ids: PublicationShared<BTreeSet<i32>>,
    #[serde(default)]
    pub(crate) freeze_immune_wearable_item_ids: PublicationShared<BTreeSet<i32>>,
    #[serde(default)]
    pub(crate) powder_snow_walkable_foot_item_ids: PublicationShared<BTreeSet<i32>>,
    #[serde(default)]
    pub(crate) default_item_mining_profiles:
        PublicationShared<BTreeMap<i32, WorldItemMiningProfile>>,
    #[serde(default)]
    pub(crate) default_block_destroy_profiles:
        PublicationShared<BTreeMap<String, WorldBlockDestroyProfile>>,
    #[serde(default)]
    pub(crate) default_block_sound_profiles:
        PublicationShared<BTreeMap<String, WorldBlockSoundProfile>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FurnaceFuelRule {
    AddItem(i32),
    AddTag(String),
    RemoveTag(String),
}

mod publication_i32_map {
    use std::collections::BTreeMap;

    use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

    use crate::PublicationShared;

    pub(super) fn serialize<S, V>(
        values: &PublicationShared<BTreeMap<i32, V>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        serializer.collect_map(
            values
                .iter()
                .map(|(item_id, value)| (item_id.to_string(), value)),
        )
    }

    pub(super) fn deserialize<'de, D, V>(
        deserializer: D,
    ) -> Result<PublicationShared<BTreeMap<i32, V>>, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
    {
        BTreeMap::<String, V>::deserialize(deserializer)?
            .into_iter()
            .map(|(item_id, value)| {
                item_id
                    .parse::<i32>()
                    .map(|item_id| (item_id, value))
                    .map_err(D::Error::custom)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()
            .map(Into::into)
    }
}

pub(crate) fn clamp_vanilla_item_max_stack_size(size: i32) -> i32 {
    size.clamp(1, VANILLA_ITEM_ABSOLUTE_MAX_STACK_SIZE)
}

impl ItemProfiles {
    pub(crate) fn set_registry_dependent_component_initializer_rules(
        &mut self,
        rules: BTreeMap<i32, Vec<RegistryDependentItemComponentInitializer>>,
    ) {
        self.registry_dependent_component_initializer_rules = rules
            .into_iter()
            .filter(|(item_id, rules)| *item_id >= 0 && !rules.is_empty())
            .collect::<BTreeMap<_, _>>()
            .into();
        self.default_item_component_patches.clear();
    }

    pub(crate) fn set_default_component_codec_values(
        &mut self,
        values: BTreeMap<i32, BTreeMap<i32, serde_json::Value>>,
    ) {
        self.default_component_codec_values = values
            .into_iter()
            .filter(|(item_id, values)| *item_id >= 0 && !values.is_empty())
            .collect::<BTreeMap<_, _>>()
            .into();
        self.default_item_component_patches.clear();
    }

    pub(crate) fn commit_item_component_defaults(
        &mut self,
        defaults: BTreeMap<i32, DataComponentPatchSummary>,
    ) {
        self.default_item_component_patches = defaults.into();
    }

    pub(crate) fn clear_item_component_defaults(&mut self) {
        self.default_item_component_patches.clear();
    }

    pub(crate) fn set_default_component_type_ids(
        &mut self,
        defaults: BTreeMap<i32, BTreeSet<i32>>,
    ) {
        self.default_component_type_ids = defaults
            .into_iter()
            .filter(|(item_id, _)| *item_id >= 0)
            .collect::<BTreeMap<_, _>>()
            .into();
    }

    pub(crate) fn set_default_dye_color_ids(&mut self, defaults: BTreeMap<i32, i32>) {
        self.default_dye_color_ids = defaults
            .into_iter()
            .filter(|(item_id, dye_color_id)| {
                *item_id >= 0
                    && bbb_protocol::builtin_enum_component_values::VANILLA_DYE_COLOR_VALUES
                        .iter()
                        .any(|value| value.id == *dye_color_id)
            })
            .collect::<BTreeMap<_, _>>()
            .into();
    }

    pub(crate) fn set_default_potion_duration_scale_bits(&mut self, defaults: BTreeMap<i32, u32>) {
        self.default_potion_duration_scale_bits = defaults
            .into_iter()
            .filter(|(item_id, bits)| {
                let value = f32::from_bits(*bits);
                *item_id >= 0 && value.is_finite() && !value.total_cmp(&0.0).is_lt()
            })
            .collect::<BTreeMap<_, _>>()
            .into();
    }

    pub(crate) fn set_default_use_cooldown_seconds_bits(&mut self, defaults: BTreeMap<i32, u32>) {
        self.default_use_cooldown_seconds_bits = defaults
            .into_iter()
            .filter(|(item_id, bits)| {
                let value = f32::from_bits(*bits);
                *item_id >= 0 && value.is_finite() && value > 0.0
            })
            .collect::<BTreeMap<_, _>>()
            .into();
    }

    pub(crate) fn item_stack_has_effective_component(
        &self,
        stack: &ProtocolItemStackSummary,
        component_id: i32,
    ) -> bool {
        if stack
            .component_patch
            .removed_type_ids
            .contains(&component_id)
        {
            return false;
        }
        stack.component_patch.added_type_ids.contains(&component_id)
            || stack.item_id.is_some_and(|item_id| {
                self.default_component_type_ids
                    .get(&item_id)
                    .is_some_and(|defaults| defaults.contains(&component_id))
                    || self
                        .default_item_component_patches
                        .get(&item_id)
                        .is_some_and(|defaults| defaults.added_type_ids.contains(&component_id))
            })
    }

    pub(crate) fn effective_component_patch(
        &self,
        item_id: i32,
        patch: &DataComponentPatchSummary,
    ) -> Option<DataComponentPatchSummary> {
        let prototype_defaults = self.default_item_component_patches.get(&item_id);
        let default_dye_color_id = self.default_dye_color_ids.get(&item_id).copied();
        let default_potion_duration_scale_bits = self
            .default_potion_duration_scale_bits
            .get(&item_id)
            .copied();
        let default_use_cooldown_seconds_bits = self
            .default_use_cooldown_seconds_bits
            .get(&item_id)
            .copied();
        let default_food_properties = self.default_item_food_properties.get(&item_id).copied();
        if prototype_defaults.is_none()
            && default_dye_color_id.is_none()
            && default_potion_duration_scale_bits.is_none()
            && default_use_cooldown_seconds_bits.is_none()
            && default_food_properties.is_none()
        {
            return None;
        }
        let mut effective = patch.clone();
        let mut changed = false;
        if let Some(defaults) = prototype_defaults {
            for component_id in &defaults.added_type_ids {
                if patch.removed_type_ids.contains(component_id)
                    || patch.added_type_ids.contains(component_id)
                {
                    continue;
                }
                copy_persistent_component_value_summary(*component_id, defaults, &mut effective)
                    .expect("validated item prototype component must remain codec-copyable");
                effective.added_type_ids.push(*component_id);
                changed = true;
            }
        }
        let dye_component_id = VANILLA_DATA_COMPONENT_DYE_TYPE_ID;
        if let Some(dye_color_id) = default_dye_color_id {
            if !patch.removed_type_ids.contains(&dye_component_id)
                && !patch.added_type_ids.contains(&dye_component_id)
                && prototype_defaults
                    .is_none_or(|defaults| !defaults.added_type_ids.contains(&dye_component_id))
            {
                effective.added_type_ids.push(dye_component_id);
                effective.dye_color_id = Some(dye_color_id);
                changed = true;
            }
        }
        let duration_scale_component_id = VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID;
        if let Some(bits) = default_potion_duration_scale_bits {
            if !patch
                .removed_type_ids
                .contains(&duration_scale_component_id)
                && !patch.added_type_ids.contains(&duration_scale_component_id)
                && prototype_defaults.is_none_or(|defaults| {
                    !defaults
                        .added_type_ids
                        .contains(&duration_scale_component_id)
                })
            {
                effective.added_type_ids.push(duration_scale_component_id);
                effective.potion_duration_scale_bits = Some(bits);
                changed = true;
            }
        }
        let use_cooldown_component_id = VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID;
        if let Some(bits) = default_use_cooldown_seconds_bits {
            if !patch.removed_type_ids.contains(&use_cooldown_component_id)
                && !patch.added_type_ids.contains(&use_cooldown_component_id)
                && prototype_defaults.is_none_or(|defaults| {
                    !defaults.added_type_ids.contains(&use_cooldown_component_id)
                })
            {
                let seconds = f32::from_bits(bits);
                effective.added_type_ids.push(use_cooldown_component_id);
                effective.use_cooldown_seconds_bits = Some(bits);
                effective.use_cooldown_ticks = Some((seconds * 20.0) as i32);
                effective.use_cooldown_group = None;
                changed = true;
            }
        }
        let food_component_id = VANILLA_DATA_COMPONENT_FOOD_TYPE_ID;
        if let Some(food) = default_food_properties {
            if !patch.removed_type_ids.contains(&food_component_id)
                && !patch.added_type_ids.contains(&food_component_id)
                && prototype_defaults
                    .is_none_or(|defaults| !defaults.added_type_ids.contains(&food_component_id))
            {
                effective.added_type_ids.push(food_component_id);
                effective.food_nutrition = Some(food.nutrition);
                effective.food_saturation_bits = Some(food.saturation_bits);
                effective.food_can_always_eat = food.can_always_eat;
                changed = true;
            }
        }
        if changed {
            effective.added = effective.added_type_ids.len();
            Some(effective)
        } else {
            None
        }
    }

    pub(crate) fn set_default_item_max_stack_sizes(&mut self, max_stack_sizes: BTreeMap<i32, i32>) {
        self.default_item_max_stack_sizes = max_stack_sizes
            .into_iter()
            .filter(|(item_id, size)| *item_id >= 0 && *size > 0)
            .map(|(item_id, size)| (item_id, clamp_vanilla_item_max_stack_size(size)))
            .collect::<BTreeMap<_, _>>()
            .into();
    }

    pub(crate) fn set_default_item_max_damage(&mut self, max_damage: BTreeMap<i32, i32>) {
        self.default_item_max_damage = max_damage
            .into_iter()
            .filter(|(item_id, max_damage)| *item_id >= 0 && *max_damage > 0)
            .collect::<BTreeMap<_, _>>()
            .into();
    }

    pub(crate) fn set_default_item_crafting_remainders(&mut self, remainders: BTreeMap<i32, i32>) {
        self.default_item_crafting_remainders_known = true;
        self.default_item_crafting_remainders = remainders
            .into_iter()
            .filter(|(item_id, remainder_id)| *item_id >= 0 && *remainder_id >= 0)
            .collect::<BTreeMap<_, _>>()
            .into();
    }

    pub(crate) fn item_max_stack_size_for_protocol_id(&self, item_id: i32) -> i32 {
        self.default_item_max_stack_sizes
            .get(&item_id)
            .copied()
            .map(clamp_vanilla_item_max_stack_size)
            .unwrap_or(VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE)
    }

    /// Vanilla `ItemStack.getMaxDamage` reads `DataComponents.MAX_DAMAGE` via
    /// `getOrDefault`, so a damageable item whose patch omits `max_damage`
    /// (the common case: only `damage` is patched) still falls back to the
    /// item's registry default.
    pub(crate) fn item_max_damage_for_protocol_id(&self, item_id: i32) -> Option<i32> {
        self.default_item_max_damage.get(&item_id).copied()
    }

    pub(crate) fn set_furnace_fuel_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.furnace_fuel_item_ids = filter_item_ids(item_ids).into();
        self.furnace_fuel_rules.clear();
    }

    pub(crate) fn set_furnace_fuel_profile(
        &mut self,
        item_ids: BTreeSet<i32>,
        rules: Vec<FurnaceFuelRule>,
    ) {
        self.furnace_fuel_item_ids = filter_item_ids(item_ids).into();
        self.furnace_fuel_rules = rules
            .into_iter()
            .filter(|rule| !matches!(rule, FurnaceFuelRule::AddItem(item_id) if *item_id < 0))
            .collect::<Vec<_>>()
            .into();
    }

    pub(crate) fn refresh_furnace_fuel_item_ids(&mut self, registries: &crate::RegistrySet) {
        if self.furnace_fuel_rules.is_empty() {
            return;
        }

        let mut item_ids = BTreeSet::new();
        for rule in self.furnace_fuel_rules.iter() {
            match rule {
                FurnaceFuelRule::AddItem(item_id) => {
                    item_ids.insert(*item_id);
                }
                FurnaceFuelRule::AddTag(tag) => {
                    if let Some(tag_item_ids) = registries.holder_ids_for_tag("minecraft:item", tag)
                    {
                        item_ids
                            .extend(tag_item_ids.iter().copied().filter(|item_id| *item_id >= 0));
                    }
                }
                FurnaceFuelRule::RemoveTag(tag) => {
                    if let Some(tag_item_ids) = registries.holder_ids_for_tag("minecraft:item", tag)
                    {
                        for item_id in tag_item_ids {
                            item_ids.remove(item_id);
                        }
                    }
                }
            }
        }
        self.furnace_fuel_item_ids = item_ids.into();
    }

    pub(crate) fn set_brewing_potion_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.brewing_potion_item_ids = filter_item_ids(item_ids).into();
    }

    pub(crate) fn set_brewing_ingredient_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.brewing_ingredient_item_ids = filter_item_ids(item_ids).into();
    }

    pub(crate) fn set_enchantment_lapis_lazuli_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.enchantment_lapis_lazuli_item_ids = filter_item_ids(item_ids).into();
    }

    pub(crate) fn set_cartography_additional_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.cartography_additional_item_ids = filter_item_ids(item_ids).into();
    }

    pub(crate) fn set_default_damageable_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.default_damageable_item_ids = filter_item_ids(item_ids).into();
    }

    pub(crate) fn set_freeze_immune_wearable_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.freeze_immune_wearable_item_ids = filter_item_ids(item_ids).into();
    }

    pub(crate) fn set_powder_snow_walkable_foot_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.powder_snow_walkable_foot_item_ids = filter_item_ids(item_ids).into();
    }

    pub(crate) fn set_default_piercing_weapon_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.default_piercing_weapon_item_ids = filter_item_ids(item_ids).into();
    }

    pub(crate) fn set_default_item_attack_ranges(
        &mut self,
        attack_ranges: BTreeMap<i32, ItemAttackRange>,
    ) {
        self.default_item_attack_ranges = filter_item_id_keys(attack_ranges).into();
    }

    pub(crate) fn set_default_item_minimum_attack_charges(&mut self, charges: BTreeMap<i32, f32>) {
        self.default_item_minimum_attack_charges = charges
            .into_iter()
            .filter(|(item_id, charge)| {
                *item_id >= 0 && charge.is_finite() && (0.0..=1.0).contains(charge)
            })
            .collect::<BTreeMap<_, _>>()
            .into();
    }

    pub(crate) fn set_default_item_swing_animation_durations(
        &mut self,
        durations: BTreeMap<i32, i32>,
    ) {
        self.default_item_swing_animation_durations = durations
            .into_iter()
            .filter(|(item_id, duration)| *item_id >= 0 && *duration > 0)
            .collect::<BTreeMap<_, _>>()
            .into();
    }

    pub(crate) fn set_default_item_use_effects(
        &mut self,
        use_effects: BTreeMap<i32, ItemUseEffects>,
    ) {
        self.default_item_use_effects = filter_item_id_keys(use_effects).into();
    }

    pub(crate) fn set_default_item_consumables(
        &mut self,
        consumables: BTreeMap<i32, ConsumableSummary>,
    ) {
        self.default_item_consumables = consumables
            .into_iter()
            .filter(|(item_id, consumable)| {
                *item_id >= 0
                    && consumable.consume_seconds.is_finite()
                    && consumable.consume_seconds >= 0.0
            })
            .collect::<BTreeMap<_, _>>()
            .into();
    }

    pub(crate) fn set_default_item_food_properties(
        &mut self,
        foods: BTreeMap<i32, ItemFoodProperties>,
    ) {
        self.default_item_food_properties = foods
            .into_iter()
            .filter(|(item_id, food)| *item_id >= 0 && food.nutrition >= 0)
            .collect::<BTreeMap<_, _>>()
            .into();
    }

    pub(crate) fn set_default_item_equipment_slots(
        &mut self,
        equipment_slots: BTreeMap<i32, ItemEquipmentSlot>,
    ) {
        self.default_item_equipment_slots = filter_item_id_keys(equipment_slots).into();
    }

    pub(crate) fn set_default_swappable_equipment_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.default_swappable_equipment_item_ids = filter_item_ids(item_ids).into();
    }

    pub(crate) fn set_default_player_swappable_equipment_item_ids(
        &mut self,
        item_ids: BTreeSet<i32>,
    ) {
        self.default_player_swappable_equipment_item_ids = filter_item_ids(item_ids).into();
    }

    pub(crate) fn set_default_shearable_equipment_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.default_shearable_equipment_item_ids = filter_item_ids(item_ids).into();
    }

    pub(crate) fn set_default_item_equipment_assets(
        &mut self,
        generation: u64,
        asset_names: Vec<String>,
        item_assets: BTreeMap<i32, crate::entities::EquipmentAssetId>,
    ) {
        let asset_count = asset_names.len();
        self.default_item_equipment_assets = item_assets
            .into_iter()
            .filter(|(item_id, asset_id)| {
                *item_id >= 0
                    && asset_id.generation == generation
                    && usize::try_from(asset_id.index).is_ok_and(|id| id < asset_count)
            })
            .collect::<BTreeMap<_, _>>()
            .into();
        self.equipment_asset_generation = generation;
        self.equipment_asset_names = asset_names.into();
    }

    pub(crate) fn set_item_armor_materials(
        &mut self,
        armor_materials: BTreeMap<i32, crate::entities::ArmorMaterialKind>,
    ) {
        self.default_item_armor_materials = filter_item_id_keys(armor_materials).into();
    }

    pub(crate) fn set_default_item_wings_equipment_assets(
        &mut self,
        equipment_assets: BTreeMap<i32, String>,
    ) {
        self.default_item_wings_equipment_assets = equipment_assets
            .into_iter()
            .filter(|(item_id, asset_id)| *item_id >= 0 && !asset_id.is_empty())
            .collect::<BTreeMap<_, _>>()
            .into();
    }

    pub(crate) fn set_default_mount_body_armor_kinds(
        &mut self,
        armor_kinds: BTreeMap<i32, MountArmorSlotKind>,
    ) {
        self.default_mount_body_armor_kinds = filter_item_id_keys(armor_kinds).into();
    }

    pub(crate) fn set_default_llama_body_decor_colors(
        &mut self,
        decor_colors: BTreeMap<i32, crate::entities::LlamaBodyDecorColor>,
    ) {
        self.default_llama_body_decor_colors = filter_item_id_keys(decor_colors).into();
    }

    pub(crate) fn set_default_happy_ghast_harness_colors(
        &mut self,
        harness_colors: BTreeMap<i32, crate::entities::HappyGhastHarnessColor>,
    ) {
        self.default_happy_ghast_harness_colors = filter_item_id_keys(harness_colors).into();
    }

    pub(crate) fn set_default_nautilus_body_armor_materials(
        &mut self,
        armor_materials: BTreeMap<i32, crate::entities::ArmorMaterialKind>,
    ) {
        self.default_nautilus_body_armor_materials = filter_item_id_keys(armor_materials).into();
    }

    pub(crate) fn set_default_horse_body_armor_materials(
        &mut self,
        armor_materials: BTreeMap<i32, crate::entities::ArmorMaterialKind>,
    ) {
        self.default_horse_body_armor_materials = filter_item_id_keys(armor_materials).into();
    }

    pub(crate) fn set_default_wolf_body_armor_materials(
        &mut self,
        armor_materials: BTreeMap<i32, crate::entities::ArmorMaterialKind>,
    ) {
        self.default_wolf_body_armor_materials = filter_item_id_keys(armor_materials).into();
    }
}

pub(crate) fn copy_persistent_component_value_summary(
    type_id: i32,
    source: &DataComponentPatchSummary,
    target: &mut DataComponentPatchSummary,
) -> bbb_protocol::codec::Result<()> {
    let codec_projection = crate::component_select_codec_projection(source);
    copy_data_component_value_summary(type_id, &codec_projection, target)?;

    // A named holder set is encoded as its tag on the network. World state
    // additionally owns the members resolved for the current registry
    // generation, so restore that derived projection after descriptor copy.
    match type_id {
        VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID => {
            target.can_place_on = source.can_place_on.clone();
            target.can_place_on_persistent = source.can_place_on_persistent.clone();
        }
        VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID => {
            target.can_break = source.can_break.clone();
            target.can_break_persistent = source.can_break_persistent.clone();
        }
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID => {
            target.consumable_effects = source.consumable_effects.clone();
        }
        VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID => {
            target.use_remainder = source.use_remainder.clone();
        }
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID => {
            target.damage_resistant_type_registry_ids =
                source.damage_resistant_type_registry_ids.clone();
        }
        VANILLA_DATA_COMPONENT_TOOL_TYPE_ID => {
            target.tool = source.tool.clone();
        }
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID => {
            target.equippable = source.equippable.clone();
        }
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID => {
            target.repairable_item_registry_ids = source.repairable_item_registry_ids.clone();
        }
        VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID => {
            target.death_protection_effects = source.death_protection_effects.clone();
        }
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID => {
            target.blocks_attacks = source.blocks_attacks.clone();
        }
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID => {
            target.charged_projectiles_items = source.charged_projectiles_items.clone();
        }
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID => {
            target.bundle_contents_items = source.bundle_contents_items.clone();
        }
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID => {
            target.provides_banner_pattern_registry_ids =
                source.provides_banner_pattern_registry_ids.clone();
        }
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID => {
            target.container_items = source.container_items.clone();
            target.container_slots = source.container_slots.clone();
        }
        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID => {
            target.lock_predicate = source.lock_predicate.clone();
        }
        _ => {}
    }
    Ok(())
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
