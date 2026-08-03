use super::*;
use bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID;
use bbb_protocol::attribute_types::vanilla_attribute_type_for_resource_id;

pub(super) fn default_attribute_modifier_summary(
    modifier: &PackItemDefaultAttributeModifier,
    attribute_keys: Option<&[String]>,
) -> AttributeModifierSummary {
    let attribute_id = attribute_keys
        .and_then(|keys| {
            keys.iter()
                .position(|key| key == &modifier.attribute_key)
                .and_then(|index| i32::try_from(index).ok())
        })
        .or_else(|| {
            vanilla_attribute_type_for_resource_id(&modifier.attribute_key).map(|entry| entry.id)
        })
        .unwrap_or(-1);
    AttributeModifierSummary {
        attribute_id,
        modifier_id: modifier.modifier_id.clone(),
        amount_bits: modifier.amount_bits,
        operation_id: modifier.operation_id,
        slot_id: modifier.slot_id,
        display_id: VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
        display_text: None,
        display_text_styled: None,
        display_text_nbt: None,
    }
}

pub(super) fn protocol_ids_for_resource_ids(
    registry: &ItemRegistryCatalog,
    resource_ids: &[&str],
) -> BTreeSet<i32> {
    resource_ids
        .iter()
        .filter_map(|resource_id| registry.protocol_id(resource_id))
        .collect()
}

pub(super) fn world_item_equipment_slot(slot: PackItemEquipmentSlot) -> WorldItemEquipmentSlot {
    match slot {
        PackItemEquipmentSlot::MainHand => WorldItemEquipmentSlot::MainHand,
        PackItemEquipmentSlot::OffHand => WorldItemEquipmentSlot::OffHand,
        PackItemEquipmentSlot::Feet => WorldItemEquipmentSlot::Feet,
        PackItemEquipmentSlot::Legs => WorldItemEquipmentSlot::Legs,
        PackItemEquipmentSlot::Chest => WorldItemEquipmentSlot::Chest,
        PackItemEquipmentSlot::Head => WorldItemEquipmentSlot::Head,
        PackItemEquipmentSlot::Body => WorldItemEquipmentSlot::Body,
        PackItemEquipmentSlot::Saddle => WorldItemEquipmentSlot::Saddle,
    }
}

pub(super) fn world_mount_armor_slot_kind(
    kind: PackItemMountBodyArmorKind,
) -> WorldMountArmorSlotKind {
    match kind {
        PackItemMountBodyArmorKind::Horse => WorldMountArmorSlotKind::Horse,
        PackItemMountBodyArmorKind::Llama => WorldMountArmorSlotKind::Llama,
        PackItemMountBodyArmorKind::Nautilus => WorldMountArmorSlotKind::Nautilus,
    }
}

pub(super) fn world_item_attack_range(range: PackItemAttackRange) -> WorldItemAttackRange {
    WorldItemAttackRange {
        min_reach: range.min_reach,
        max_reach: range.max_reach,
        min_creative_reach: range.min_creative_reach,
        max_creative_reach: range.max_creative_reach,
        hitbox_margin: range.hitbox_margin,
        mob_factor: range.mob_factor,
    }
}

pub(super) fn world_item_use_effects(effects: PackItemUseEffects) -> WorldItemUseEffects {
    WorldItemUseEffects {
        can_sprint: effects.can_sprint,
        interact_vibrations: effects.interact_vibrations,
        speed_multiplier: effects.speed_multiplier,
    }
}

pub(super) fn world_item_food_properties(food: PackItemFoodProperties) -> WorldItemFoodProperties {
    WorldItemFoodProperties {
        nutrition: food.nutrition,
        saturation_bits: food.saturation_bits,
        can_always_eat: food.can_always_eat,
    }
}

pub(super) fn protocol_item_consumable(consumable: PackItemConsumable) -> ConsumableSummary {
    ConsumableSummary {
        consume_seconds: consumable.consume_seconds,
        animation: protocol_item_use_animation(consumable.animation),
    }
}

pub(super) fn protocol_item_use_animation(
    animation: PackItemUseAnimation,
) -> ItemUseAnimationSummary {
    match animation {
        PackItemUseAnimation::None => ItemUseAnimationSummary::None,
        PackItemUseAnimation::Eat => ItemUseAnimationSummary::Eat,
        PackItemUseAnimation::Drink => ItemUseAnimationSummary::Drink,
        PackItemUseAnimation::Block => ItemUseAnimationSummary::Block,
        PackItemUseAnimation::Bow => ItemUseAnimationSummary::Bow,
        PackItemUseAnimation::Trident => ItemUseAnimationSummary::Trident,
        PackItemUseAnimation::Crossbow => ItemUseAnimationSummary::Crossbow,
        PackItemUseAnimation::Spyglass => ItemUseAnimationSummary::Spyglass,
        PackItemUseAnimation::TootHorn => ItemUseAnimationSummary::TootHorn,
        PackItemUseAnimation::Brush => ItemUseAnimationSummary::Brush,
        PackItemUseAnimation::Bundle => ItemUseAnimationSummary::Bundle,
        PackItemUseAnimation::Spear => ItemUseAnimationSummary::Spear,
    }
}

pub(super) fn world_item_mining_rule(rule: &PackItemMiningRule) -> WorldItemMiningRule {
    WorldItemMiningRule {
        block_names: rule.block_names.clone(),
        mining_speed_thousandths: rule.mining_speed_thousandths,
        correct_for_drops: rule.correct_for_drops,
    }
}

impl NativeItemRuntime {
    pub fn default_component_type_ids_by_protocol_id(
        &self,
    ) -> BTreeMap<i32, std::collections::BTreeSet<i32>> {
        self.registry
            .as_ref()
            .map(ItemRegistryCatalog::default_component_type_ids_by_protocol_id)
            .unwrap_or_default()
    }

    pub fn default_component_codec_values_by_protocol_id(
        &self,
    ) -> BTreeMap<i32, BTreeMap<i32, serde_json::Value>> {
        self.registry
            .as_ref()
            .map(ItemRegistryCatalog::default_component_codec_values_by_protocol_id)
            .unwrap_or_default()
    }

    pub fn default_dye_color_ids_by_protocol_id(&self) -> BTreeMap<i32, i32> {
        self.registry
            .as_ref()
            .map(ItemRegistryCatalog::default_dye_color_ids_by_protocol_id)
            .unwrap_or_default()
    }

    pub fn default_potion_duration_scale_bits_by_protocol_id(&self) -> BTreeMap<i32, u32> {
        self.registry
            .as_ref()
            .map(ItemRegistryCatalog::default_potion_duration_scale_bits_by_protocol_id)
            .unwrap_or_default()
    }

    pub fn default_use_cooldown_seconds_bits_by_protocol_id(&self) -> BTreeMap<i32, u32> {
        self.registry
            .as_ref()
            .map(ItemRegistryCatalog::default_use_cooldown_seconds_bits_by_protocol_id)
            .unwrap_or_default()
    }

    pub fn registry_dependent_component_initializers_by_protocol_id(
        &self,
    ) -> BTreeMap<i32, Vec<bbb_pack::RegistryDependentItemComponentInitializer>> {
        self.registry
            .as_ref()
            .map(ItemRegistryCatalog::registry_dependent_component_initializers_by_protocol_id)
            .unwrap_or_default()
    }

    pub fn map_background_textures(&self) -> &[FirstPersonMapBackgroundTexture] {
        &self.map_background_textures
    }

    pub fn map_decoration_textures(&self) -> &[ItemFrameMapDecorationTexture] {
        &self.map_decoration_textures
    }

    pub fn map_text_glyphs(&self) -> Option<&HudFontGlyphMap> {
        self.map_text_fonts.as_ref().map(|fonts| fonts.glyphs(None))
    }

    pub fn map_text_fonts(&self) -> Option<&HudFontGlyphBook> {
        self.map_text_fonts.as_ref()
    }

    pub fn item_max_stack_sizes_by_protocol_id(&self) -> BTreeMap<i32, i32> {
        let mut sizes = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return sizes;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(size) = registry.max_stack_size(resource_id) else {
                continue;
            };
            sizes.insert(protocol_id as i32, size);
        }
        sizes
    }

    pub fn item_max_damage_by_protocol_id(&self) -> BTreeMap<i32, i32> {
        let mut max_damage = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return max_damage;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(damage) = registry.max_damage(resource_id) else {
                continue;
            };
            max_damage.insert(protocol_id as i32, damage);
        }
        max_damage
    }

    pub fn item_max_damage_count(&self) -> usize {
        self.item_max_damage_by_protocol_id().len()
    }

    pub fn item_crafting_remainders_by_protocol_id(&self) -> BTreeMap<i32, i32> {
        self.registry
            .as_ref()
            .map(ItemRegistryCatalog::crafting_remainders_by_protocol_id)
            .unwrap_or_default()
    }

    pub fn item_crafting_remainder_count(&self) -> usize {
        self.item_crafting_remainders_by_protocol_id().len()
    }

    pub fn item_equipment_slots_by_protocol_id(&self) -> BTreeMap<i32, WorldItemEquipmentSlot> {
        let mut slots = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return slots;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            if let Some(slot) = self.item_equipment_slot_for_resource_id(registry, resource_id) {
                slots.insert(protocol_id as i32, slot);
            }
        }
        slots
    }

    pub fn item_equipment_slot(&self, protocol_id: i32) -> Option<WorldItemEquipmentSlot> {
        let registry = self.registry.as_ref()?;
        let resource_id = registry.resource_id(protocol_id)?;
        self.item_equipment_slot_for_resource_id(registry, resource_id)
    }

    pub(super) fn item_equipment_slot_for_resource_id(
        &self,
        registry: &ItemRegistryCatalog,
        resource_id: &str,
    ) -> Option<WorldItemEquipmentSlot> {
        Some(world_item_equipment_slot(
            registry.equipment_slot(resource_id)?,
        ))
    }

    pub fn item_equipment_slot_count(&self) -> usize {
        self.item_equipment_slots_by_protocol_id().len()
    }

    pub fn default_shearable_equipment_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.registry
            .as_ref()
            .map(ItemRegistryCatalog::default_shearable_equipment_protocol_ids)
            .unwrap_or_default()
    }

    pub fn default_swappable_equipment_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.registry
            .as_ref()
            .map(ItemRegistryCatalog::default_swappable_equipment_protocol_ids)
            .unwrap_or_default()
    }

    pub fn default_player_swappable_equipment_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.registry
            .as_ref()
            .map(ItemRegistryCatalog::default_player_swappable_equipment_protocol_ids)
            .unwrap_or_default()
    }

    /// Pack catalog used to build the generation-matched renderer equipment
    /// texture catalog during startup and atomic resource reload.
    pub fn equipment_asset_catalog(&self) -> &EquipmentAssetCatalog {
        &self.equipment_assets
    }

    /// Resolves an asset resource id to its generation-local compact identity.
    pub fn equipment_asset_id(&self, asset_id: &str) -> Option<EntityEquipmentAssetId> {
        self.equipment_assets
            .asset_index(asset_id)
            .map(|index| EntityEquipmentAssetId::new(self.resource_generation, index))
    }

    /// Normalized asset names in the same order as compact identities.
    pub fn equipment_asset_names(&self) -> Vec<String> {
        self.equipment_assets
            .indexed_assets()
            .map(|(_index, asset_id, _asset)| asset_id.to_owned())
            .collect()
    }

    /// Item protocol id → generation-local equipment asset identity.
    pub fn item_equipment_asset_ids_by_protocol_id(&self) -> BTreeMap<i32, EntityEquipmentAssetId> {
        let mut assets = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return assets;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(asset_id) = registry
                .equippable_asset(resource_id)
                .and_then(|asset_id| self.equipment_asset_id(asset_id))
            else {
                continue;
            };
            assets.insert(protocol_id as i32, asset_id);
        }
        assets
    }

    /// One item's generation-local equipment asset identity. The layer type is
    /// deliberately resolved only by the renderer-private catalog.
    pub fn item_equipment_asset_id(&self, protocol_id: i32) -> Option<EntityEquipmentAssetId> {
        let registry = self.registry.as_ref()?;
        let resource_id = registry.resource_id(protocol_id)?;
        let asset_id = registry.equippable_asset(resource_id)?;
        self.equipment_asset_id(asset_id)
    }

    pub fn mount_body_armor_kinds_by_protocol_id(&self) -> BTreeMap<i32, WorldMountArmorSlotKind> {
        self.registry
            .as_ref()
            .map(|registry| {
                registry
                    .mount_body_armor_kinds_by_protocol_id()
                    .into_iter()
                    .map(|(item_id, kind)| (item_id, world_mount_armor_slot_kind(kind)))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn mount_body_armor_kind_count(&self) -> usize {
        self.mount_body_armor_kinds_by_protocol_id().len()
    }

    pub fn default_piercing_weapon_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.registry
            .as_ref()
            .map(ItemRegistryCatalog::default_piercing_weapon_protocol_ids)
            .unwrap_or_default()
    }

    pub fn default_piercing_weapon_item_count(&self) -> usize {
        self.default_piercing_weapon_item_ids_by_protocol_id().len()
    }

    pub fn default_damageable_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.registry
            .as_ref()
            .map(ItemRegistryCatalog::max_damage_protocol_ids)
            .unwrap_or_default()
    }

    pub fn default_damageable_item_count(&self) -> usize {
        self.default_damageable_item_ids_by_protocol_id().len()
    }

    pub fn item_attack_ranges_by_protocol_id(&self) -> BTreeMap<i32, WorldItemAttackRange> {
        let mut ranges = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return ranges;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(range) = registry.default_attack_range(resource_id) else {
                continue;
            };
            ranges.insert(protocol_id as i32, world_item_attack_range(range));
        }
        ranges
    }

    pub fn item_attack_range_count(&self) -> usize {
        self.item_attack_ranges_by_protocol_id().len()
    }

    pub fn item_minimum_attack_charges_by_protocol_id(&self) -> BTreeMap<i32, f32> {
        let mut charges = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return charges;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(charge) = registry.default_minimum_attack_charge(resource_id) else {
                continue;
            };
            charges.insert(protocol_id as i32, charge);
        }
        charges
    }

    pub fn item_minimum_attack_charge_count(&self) -> usize {
        self.item_minimum_attack_charges_by_protocol_id().len()
    }

    pub fn item_swing_animations_by_protocol_id(
        &self,
    ) -> BTreeMap<i32, bbb_protocol::packets::SwingAnimationSummary> {
        let mut animations = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return animations;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(animation) = registry.default_swing_animation(resource_id) else {
                continue;
            };
            animations.insert(
                protocol_id as i32,
                bbb_protocol::packets::SwingAnimationSummary {
                    animation_type: animation.animation_type,
                    duration: animation.duration,
                },
            );
        }
        animations
    }

    pub fn item_default_swing_animation(
        &self,
        resource_id: &str,
    ) -> Option<bbb_protocol::packets::SwingAnimationSummary> {
        let animation = self
            .registry
            .as_ref()?
            .default_swing_animation(resource_id)?;
        Some(bbb_protocol::packets::SwingAnimationSummary {
            animation_type: animation.animation_type,
            duration: animation.duration,
        })
    }

    pub fn item_swing_animation_count(&self) -> usize {
        self.item_swing_animations_by_protocol_id().len()
    }

    pub fn item_use_effects_by_protocol_id(&self) -> BTreeMap<i32, WorldItemUseEffects> {
        let mut use_effects = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return use_effects;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(effects) = registry.default_use_effects(resource_id) else {
                continue;
            };
            use_effects.insert(protocol_id as i32, world_item_use_effects(effects));
        }
        use_effects
    }

    pub fn item_use_effect_count(&self) -> usize {
        self.item_use_effects_by_protocol_id().len()
    }

    pub fn item_food_properties_by_protocol_id(&self) -> BTreeMap<i32, WorldItemFoodProperties> {
        let mut foods = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return foods;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(food) = registry.default_food_properties(resource_id) else {
                continue;
            };
            foods.insert(protocol_id as i32, world_item_food_properties(food));
        }
        foods
    }

    pub fn item_food_property_count(&self) -> usize {
        self.item_food_properties_by_protocol_id().len()
    }

    pub fn item_default_consumable(&self, item_id: i32) -> Option<ConsumableSummary> {
        let registry = self.registry.as_ref()?;
        let resource_id = registry
            .resource_ids()
            .get(usize::try_from(item_id).ok()?)?;
        registry
            .default_consumable(resource_id)
            .map(protocol_item_consumable)
    }

    pub fn item_default_consumables_by_protocol_id(&self) -> BTreeMap<i32, ConsumableSummary> {
        let mut consumables = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return consumables;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(consumable) = registry.default_consumable(resource_id) else {
                continue;
            };
            consumables.insert(protocol_id as i32, protocol_item_consumable(consumable));
        }
        consumables
    }

    pub fn furnace_fuel_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.furnace_fuel_item_ids.clone()
    }

    pub fn furnace_fuel_item_count(&self) -> usize {
        self.furnace_fuel_item_ids.len()
    }

    pub fn brewing_potion_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.registry
            .as_ref()
            .map(|registry| protocol_ids_for_resource_ids(registry, BREWING_POTION_ITEM_IDS))
            .unwrap_or_default()
    }

    pub fn brewing_potion_item_count(&self) -> usize {
        self.brewing_potion_item_ids_by_protocol_id().len()
    }

    pub fn brewing_ingredient_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.registry
            .as_ref()
            .map(|registry| protocol_ids_for_resource_ids(registry, BREWING_INGREDIENT_ITEM_IDS))
            .unwrap_or_default()
    }

    pub fn brewing_ingredient_item_count(&self) -> usize {
        self.brewing_ingredient_item_ids_by_protocol_id().len()
    }

    pub fn enchantment_lapis_lazuli_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.registry
            .as_ref()
            .map(|registry| {
                protocol_ids_for_resource_ids(registry, ENCHANTMENT_LAPIS_LAZULI_ITEM_IDS)
            })
            .unwrap_or_default()
    }

    pub fn enchantment_lapis_lazuli_item_count(&self) -> usize {
        self.enchantment_lapis_lazuli_item_ids_by_protocol_id()
            .len()
    }

    pub fn cartography_additional_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        let Some(registry) = &self.registry else {
            return BTreeSet::new();
        };
        let item_ids = protocol_ids_for_resource_ids(registry, CARTOGRAPHY_ADDITIONAL_ITEM_IDS);
        if item_ids.len() == CARTOGRAPHY_ADDITIONAL_ITEM_IDS.len() {
            item_ids
        } else {
            BTreeSet::new()
        }
    }

    pub fn cartography_additional_item_count(&self) -> usize {
        self.cartography_additional_item_ids_by_protocol_id().len()
    }

    pub fn freeze_immune_wearable_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.freeze_immune_wearable_item_ids.clone()
    }

    pub fn freeze_immune_wearable_item_count(&self) -> usize {
        self.freeze_immune_wearable_item_ids.len()
    }

    pub fn powder_snow_walkable_foot_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.powder_snow_walkable_foot_item_ids.clone()
    }

    pub fn powder_snow_walkable_foot_item_count(&self) -> usize {
        self.powder_snow_walkable_foot_item_ids.len()
    }

    pub fn resolved_model_count(&self) -> usize {
        self.resolved_model_count
    }

    pub fn missing_model_count(&self) -> usize {
        self.missing_model_ids.len()
    }

    pub fn missing_texture_count(&self) -> usize {
        self.missing_texture_ids.len()
    }

    pub fn texture_count(&self) -> usize {
        self.textures.texture_count()
    }

    pub fn icon_texture_count(&self) -> usize {
        self.item_icon_models.len()
    }

    pub fn atlas_size(&self) -> (u32, u32) {
        self.textures.atlas_size()
    }

    pub fn atlas_rgba(&self) -> &[u8] {
        self.textures.atlas_rgba()
    }

    pub fn has_texture_animation(&self) -> bool {
        self.textures.has_animation()
    }

    pub fn texture_animation_atlas_patches(
        &self,
        previous_tick: u64,
        tick: u64,
    ) -> Result<Option<Vec<bbb_pack::AtlasMipPatch>>> {
        self.textures.animation_atlas_patches(previous_tick, tick)
    }

    pub fn atlas_sprite_uvs(&self) -> Vec<ItemAtlasSpriteUv> {
        self.textures.sprite_uvs()
    }

    pub fn texture_index(&self, texture_id: &str) -> u32 {
        self.textures.texture_index(texture_id)
    }

    pub fn item_resource_id_for_protocol_id(&self, protocol_id: i32) -> Option<&str> {
        self.registry.as_ref()?.resource_id(protocol_id)
    }

    pub(super) fn default_max_stack_size_for_protocol_id(&self, protocol_id: i32) -> i32 {
        self.registry
            .as_ref()
            .and_then(|registry| {
                registry
                    .resource_id(protocol_id)
                    .and_then(|resource_id| registry.max_stack_size(resource_id))
            })
            .unwrap_or(64)
            .clamp(1, 99)
    }

    pub(super) fn default_max_damage_for_protocol_id(&self, protocol_id: i32) -> Option<i32> {
        self.registry.as_ref().and_then(|registry| {
            registry
                .resource_id(protocol_id)
                .and_then(|resource_id| registry.max_damage(resource_id))
        })
    }

    pub(super) fn default_component_type_ids_for_resource_id(
        &self,
        resource_id: &str,
    ) -> Option<&BTreeSet<i32>> {
        self.registry
            .as_ref()
            .and_then(|registry| registry.default_component_type_ids(resource_id))
    }

    pub(super) fn default_potion_duration_scale_for_resource_id(
        &self,
        resource_id: &str,
    ) -> Option<f32> {
        self.registry
            .as_ref()
            .and_then(|registry| registry.default_potion_duration_scale(resource_id))
    }

    pub(super) fn default_item_name_translation_key_for_resource_id(
        &self,
        resource_id: &str,
    ) -> String {
        self.registry
            .as_ref()
            .and_then(|registry| registry.default_item_name_translation_key(resource_id))
            .map(str::to_string)
            .unwrap_or_else(|| default_item_name_translation_key(resource_id))
    }

    pub(super) fn default_attribute_modifiers_for_resource_id(
        &self,
        resource_id: &str,
        attribute_keys: Option<&[String]>,
    ) -> Vec<AttributeModifierSummary> {
        self.registry
            .as_ref()
            .and_then(|registry| registry.default_attribute_modifiers(resource_id))
            .map(|modifiers| {
                modifiers
                    .iter()
                    .map(|modifier| default_attribute_modifier_summary(modifier, attribute_keys))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(super) fn default_attribute_modifiers_for_protocol_id(
        &self,
        protocol_id: i32,
        attribute_keys: Option<&[String]>,
    ) -> Vec<AttributeModifierSummary> {
        let Some(resource_id) = self
            .registry
            .as_ref()
            .and_then(|registry| registry.resource_id(protocol_id))
        else {
            return Vec::new();
        };
        self.default_attribute_modifiers_for_resource_id(resource_id, attribute_keys)
    }
}
