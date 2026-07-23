use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context, Result};
use bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID;
use bbb_protocol::builtin_enum_component_values::VANILLA_DYE_COLOR_VALUES;
use bbb_protocol::equipment_slots::{
    VANILLA_EQUIPMENT_SLOT_GROUP_BODY_ID, VANILLA_EQUIPMENT_SLOT_GROUP_CHEST_ID,
    VANILLA_EQUIPMENT_SLOT_GROUP_FEET_ID, VANILLA_EQUIPMENT_SLOT_GROUP_HEAD_ID,
    VANILLA_EQUIPMENT_SLOT_GROUP_LEGS_ID,
};
use serde::{Deserialize, Serialize};

use crate::generated_java_expression::GeneratedJavaExpression;
use crate::{
    block_registry_declarations::load_generated_block_declarations, resources::ResourceLocation,
    tags::TagCatalog, PackRoots,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemMountBodyArmorKind {
    Horse,
    Llama,
    Nautilus,
}

/// One vanilla `DataComponentInitializers` rule whose value depends on the
/// frozen registry/tag generation rather than only on `Items.java` literals.
///
/// The item catalog owns these source rules. `bbb-world` resolves them against
/// its candidate `FrozenRegistryAccess`; this type deliberately contains no
/// holder ids or world-owned state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "component", rename_all = "snake_case")]
pub enum RegistryDependentItemComponentInitializer {
    DamageResistant { tag: String },
    JukeboxPlayable { song: String },
    BlocksAttacks,
    ProvidesBannerPatterns { tag: String },
    ProvidesTrimMaterial { material: String },
    Instrument { instrument: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemMiningRule {
    pub block_names: Vec<String>,
    pub mining_speed_thousandths: Option<u32>,
    pub correct_for_drops: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemMiningProfile {
    pub default_mining_speed_thousandths: u32,
    pub rules: Vec<ItemMiningRule>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemRegistryCatalog {
    resource_ids: Vec<String>,
    protocol_ids: BTreeMap<String, i32>,
    #[serde(default)]
    max_damage: BTreeMap<String, i32>,
    #[serde(default)]
    max_stack_size: BTreeMap<String, i32>,
    #[serde(default)]
    default_item_name_translation_keys: BTreeMap<String, String>,
    #[serde(default)]
    default_component_type_ids: BTreeMap<String, BTreeSet<i32>>,
    #[serde(default)]
    default_dye_color_ids: BTreeMap<String, i32>,
    #[serde(default)]
    registry_dependent_component_initializers:
        BTreeMap<String, Vec<RegistryDependentItemComponentInitializer>>,
    #[serde(default)]
    block_items: BTreeSet<String>,
    #[serde(default)]
    block_item_resource_ids: BTreeMap<String, String>,
    #[serde(default)]
    primary_block_resource_ids: BTreeMap<String, String>,
    #[serde(default)]
    spawn_egg_items: BTreeSet<String>,
    #[serde(default)]
    default_equipment_slots: BTreeMap<String, ItemEquipmentSlot>,
    /// Resource id → humanoid armor equipment-asset name (`ArmorMaterials.<MAT>` →
    /// `EquipmentAssets.<MAT>`, the lowercased material, e.g. `iron` / `chainmail` / `turtle_scute`),
    /// for the `HumanoidArmorLayer` texture path. Only `.humanoidArmor(...)` items appear here.
    #[serde(default)]
    humanoid_armor_assets: BTreeMap<String, String>,
    /// Resource id -> equippable equipment-asset name. Humanoid armor contributes the same asset as
    /// `humanoid_armor_assets`; explicit `Equippable.builder(...).setAsset(EquipmentAssets.X)` items
    /// such as elytra contribute their declared asset.
    #[serde(default)]
    equippable_assets: BTreeMap<String, String>,
    #[serde(default)]
    default_mount_body_armor_kinds: BTreeMap<String, ItemMountBodyArmorKind>,
    /// Resource id -> mount body armor equipment-asset name (`ArmorMaterials.<MAT>` ->
    /// `EquipmentAssets.<MAT>`, lowercased), for horse/nautilus body equipment layers.
    #[serde(default)]
    mount_body_armor_assets: BTreeMap<String, String>,
    #[serde(default)]
    default_piercing_weapon_ids: BTreeSet<String>,
    #[serde(default)]
    default_attack_ranges: BTreeMap<String, ItemAttackRange>,
    #[serde(default)]
    default_swing_animation_durations: BTreeMap<String, i32>,
    #[serde(default)]
    default_kinetic_weapons: BTreeMap<String, ItemKineticWeaponDefaults>,
    #[serde(default)]
    default_attribute_modifiers: BTreeMap<String, Vec<ItemDefaultAttributeModifier>>,
    #[serde(default)]
    default_use_effects: BTreeMap<String, ItemUseEffects>,
    #[serde(default)]
    default_consumables: BTreeMap<String, ItemConsumable>,
    #[serde(default)]
    crafting_remainders: BTreeMap<String, String>,
    #[serde(default)]
    mining_profiles: BTreeMap<String, ItemMiningProfile>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDefaultAttributeModifier {
    pub attribute_key: String,
    pub modifier_id: String,
    pub amount_bits: u64,
    pub operation_id: i32,
    pub slot_id: i32,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ItemUseEffects {
    pub can_sprint: bool,
    pub interact_vibrations: bool,
    pub speed_multiplier: f32,
}

impl PartialEq for ItemUseEffects {
    fn eq(&self, other: &Self) -> bool {
        self.can_sprint == other.can_sprint
            && self.interact_vibrations == other.interact_vibrations
            && self.speed_multiplier.to_bits() == other.speed_multiplier.to_bits()
    }
}

impl Eq for ItemUseEffects {}

/// Default `KineticWeapon` fields currently consumed by client presentation.
/// Damage multipliers, thresholds, and sounds remain outside this projection
/// until their semantic owners consume them.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ItemKineticWeaponDefaults {
    pub delay_ticks: i32,
    pub dismount_duration_ticks: i32,
    pub knockback_duration_ticks: i32,
    pub damage_duration_ticks: i32,
    pub forward_movement: f32,
}

impl PartialEq for ItemKineticWeaponDefaults {
    fn eq(&self, other: &Self) -> bool {
        self.delay_ticks == other.delay_ticks
            && self.dismount_duration_ticks == other.dismount_duration_ticks
            && self.knockback_duration_ticks == other.knockback_duration_ticks
            && self.damage_duration_ticks == other.damage_duration_ticks
            && self.forward_movement.to_bits() == other.forward_movement.to_bits()
    }
}

impl Eq for ItemKineticWeaponDefaults {}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ItemConsumable {
    pub consume_seconds: f32,
    pub animation: ItemUseAnimation,
}

impl PartialEq for ItemConsumable {
    fn eq(&self, other: &Self) -> bool {
        self.consume_seconds.to_bits() == other.consume_seconds.to_bits()
            && self.animation == other.animation
    }
}

impl Eq for ItemConsumable {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemUseAnimation {
    None,
    Eat,
    Drink,
    Block,
    Bow,
    Trident,
    Crossbow,
    Spyglass,
    TootHorn,
    Brush,
    Bundle,
    Spear,
}

impl Default for ItemUseAnimation {
    fn default() -> Self {
        Self::Eat
    }
}

use bbb_protocol::data_component_types as component_id;

const GENERATED_ITEM_DECLARATIONS: &str =
    include_str!("../data/item_registry_declarations_26_1.json");
const ITEM_REGISTRY_CATALOG_OVERRIDE_PATH: &str = ".bbb/item_registry_catalog.json";

#[derive(Debug, Deserialize)]
struct ItemSourceCatalog {
    vanilla_version: String,
    sources: Vec<String>,
    item_id_constants: BTreeMap<String, String>,
    declarations: Vec<ItemSourceDeclaration>,
}

#[derive(Debug, Deserialize)]
struct ItemSourceDeclaration {
    field: String,
    field_type: String,
    declaration_order: usize,
    source_path: String,
    start_byte: usize,
    end_byte: usize,
    initializer: GeneratedJavaExpression,
}

fn insert_unique_resource_mapping(
    mappings: &mut BTreeMap<String, String>,
    key: &str,
    value: &str,
    context: &str,
) -> Result<()> {
    if let Some(previous) = mappings.get(key) {
        if previous != value {
            bail!("{context}: conflicting mapping for {key}: {previous} vs {value}");
        }
        return Ok(());
    }
    mappings.insert(key.to_string(), value.to_string());
    Ok(())
}

impl ItemRegistryCatalog {
    /// Load the checked-in vanilla 26.1 item catalog without reading a local Java source tree.
    pub fn vanilla_26_1() -> Result<Self> {
        Self::from_generated_source_catalog(None)
    }

    pub fn load(roots: &PackRoots) -> Result<Self> {
        let override_path = roots.sources_dir.join(ITEM_REGISTRY_CATALOG_OVERRIDE_PATH);
        if override_path.is_file() {
            return roots
                .resource_stack()
                .read_json_file(&override_path, "item registry catalog override")
                .with_context(|| {
                    format!(
                        "parse item registry catalog override {}",
                        override_path.display()
                    )
                });
        }
        let block_tags = roots.load_tag_catalog("block").ok();
        Self::from_generated_source_catalog(block_tags.as_ref())
    }

    pub(crate) fn from_generated_source_catalog(block_tags: Option<&TagCatalog>) -> Result<Self> {
        let source: ItemSourceCatalog = serde_json::from_str(GENERATED_ITEM_DECLARATIONS)?;
        if source.vanilla_version != crate::MC_VERSION
            || source.sources
                != [
                    "net/minecraft/references/ItemIds.java",
                    "net/minecraft/world/item/Items.java",
                ]
        {
            bail!("unsupported generated item source catalog manifest");
        }
        let mut resource_ids = Vec::new();
        let mut max_damage = BTreeMap::new();
        let mut max_stack_size = BTreeMap::new();
        let mut default_item_name_translation_keys = BTreeMap::new();
        let mut default_component_type_ids = BTreeMap::new();
        let mut default_dye_color_ids = BTreeMap::new();
        let mut registry_dependent_component_initializers = BTreeMap::new();
        let mut block_items = BTreeSet::new();
        let mut block_item_resource_ids = BTreeMap::new();
        let mut primary_block_resource_ids = BTreeMap::new();
        let block_resources_by_constant = load_generated_block_declarations()?
            .declarations
            .into_iter()
            .map(|declaration| (declaration.field, declaration.resource_id))
            .collect::<BTreeMap<_, _>>();
        let mut spawn_egg_items = BTreeSet::new();
        let mut default_equipment_slots = BTreeMap::new();
        let mut humanoid_armor_assets = BTreeMap::new();
        let mut equippable_assets = BTreeMap::new();
        let mut default_mount_body_armor_kinds = BTreeMap::new();
        let mut mount_body_armor_assets = BTreeMap::new();
        let mut default_piercing_weapon_ids = BTreeSet::new();
        let mut default_attack_ranges = BTreeMap::new();
        let mut default_swing_animation_durations = BTreeMap::new();
        let mut default_kinetic_weapons = BTreeMap::new();
        let mut default_attribute_modifiers = BTreeMap::new();
        let mut default_use_effects = BTreeMap::new();
        let mut default_consumables = BTreeMap::new();
        let mut crafting_remainders = BTreeMap::new();
        let mut mining_profiles = BTreeMap::new();
        for (order, declaration) in source.declarations.iter().enumerate() {
            if declaration.declaration_order != order
                || declaration.source_path != "net/minecraft/world/item/Items.java"
                || declaration.start_byte >= declaration.end_byte
            {
                bail!(
                    "invalid generated item declaration order/span for Items.{}",
                    declaration.field
                );
            }
            let kind = declaration.field_type.as_str();
            let field = declaration.field.as_str();
            let expression = &declaration.initializer;
            let source_context = || {
                format!(
                    "{}:{}..{}: resolve Items.{field}",
                    declaration.source_path, declaration.start_byte, declaration.end_byte
                )
            };
            let ids =
                resource_ids_for_declaration(kind, field, expression, &source.item_id_constants)
                    .with_context(source_context)?;
            let root_name = expression.name.as_deref();
            let is_block_item = kind == "WeatheringCopperItems"
                || root_name == Some("registerBlock")
                || expression
                    .call("createBlockItemWithCustomItemName")
                    .is_some();
            let is_spawn_egg_item = root_name == Some("registerSpawnEgg");
            let default_item_name =
                default_item_name_for_declaration(kind, expression).with_context(source_context)?;
            let default_components = default_component_type_ids_for_declaration(expression)
                .with_context(source_context)?;
            let default_dye_color =
                default_dye_color_for_declaration(expression).with_context(source_context)?;
            let registry_component_initializers =
                registry_dependent_component_initializers_for_declaration(expression, &ids)
                    .with_context(source_context)?;
            let stack_size =
                max_stack_size_for_declaration(expression).with_context(source_context)?;
            let equipment_slot =
                equipment_slot_for_declaration(expression).with_context(source_context)?;
            let humanoid_armor_asset =
                humanoid_armor_asset_for_declaration(expression).with_context(source_context)?;
            let wolf_armor_asset =
                wolf_armor_asset_for_declaration(expression).with_context(source_context)?;
            let explicit_equippable_asset = explicit_equippable_asset_for_declaration(expression)
                .with_context(source_context)?;
            let mount_body_armor_kind = mount_body_armor_kind_for_declaration(expression);
            let mount_body_armor_asset =
                mount_body_armor_asset_for_declaration(expression).with_context(source_context)?;
            let default_piercing_weapon = default_piercing_weapon_for_declaration(expression);
            let default_attack_range =
                default_attack_range_for_declaration(expression).with_context(source_context)?;
            let default_swing_animation_duration =
                default_swing_animation_duration_for_declaration(expression)
                    .with_context(source_context)?;
            let default_kinetic_weapon =
                default_kinetic_weapon_for_declaration(expression).with_context(source_context)?;
            let default_attribute_modifier_entries =
                default_attribute_modifiers_for_declaration(expression)
                    .with_context(source_context)?;
            let default_use_effect =
                default_use_effects_for_declaration(expression).with_context(source_context)?;
            let default_consumable =
                default_consumable_for_declaration(expression).with_context(source_context)?;
            let crafting_remainder =
                crafting_remainder_for_declaration(expression, &source.item_id_constants)
                    .with_context(source_context)?;
            let mining_profile = mining_profile_for_declaration(
                expression,
                block_tags,
                &block_resources_by_constant,
            )
            .with_context(source_context)?;
            if let Some(durability) =
                durability_for_declaration(expression).with_context(source_context)?
            {
                for resource_id in &ids {
                    max_damage.insert(resource_id.clone(), durability);
                }
            }
            for resource_id in &ids {
                if is_block_item {
                    block_items.insert(resource_id.clone());
                }
                if is_spawn_egg_item {
                    spawn_egg_items.insert(resource_id.clone());
                }
                max_stack_size.insert(resource_id.clone(), stack_size);
                default_item_name_translation_keys.insert(
                    resource_id.clone(),
                    default_item_name.translation_key(resource_id),
                );
                default_component_type_ids.insert(resource_id.clone(), default_components.clone());
                if let Some(dye_color_id) = default_dye_color {
                    default_dye_color_ids.insert(resource_id.clone(), dye_color_id);
                }
                if !registry_component_initializers.is_empty() {
                    registry_dependent_component_initializers
                        .insert(resource_id.clone(), registry_component_initializers.clone());
                }
                if let Some(equipment_slot) = equipment_slot {
                    default_equipment_slots.insert(resource_id.clone(), equipment_slot);
                }
                if let Some(asset) = &humanoid_armor_asset {
                    humanoid_armor_assets.insert(resource_id.clone(), asset.clone());
                    equippable_assets.insert(resource_id.clone(), asset.clone());
                }
                if let Some(asset) = &wolf_armor_asset {
                    equippable_assets.insert(resource_id.clone(), asset.clone());
                }
                if let Some(asset) = &explicit_equippable_asset {
                    equippable_assets.insert(resource_id.clone(), asset.clone());
                }
                if let Some(kind) = mount_body_armor_kind {
                    default_mount_body_armor_kinds.insert(resource_id.clone(), kind);
                }
                if let Some(asset) = &mount_body_armor_asset {
                    mount_body_armor_assets.insert(resource_id.clone(), asset.clone());
                }
                if default_piercing_weapon {
                    default_piercing_weapon_ids.insert(resource_id.clone());
                }
                if let Some(default_attack_range) = default_attack_range {
                    default_attack_ranges.insert(resource_id.clone(), default_attack_range);
                }
                if let Some(default_swing_animation_duration) = default_swing_animation_duration {
                    default_swing_animation_durations
                        .insert(resource_id.clone(), default_swing_animation_duration);
                }
                if let Some(default_kinetic_weapon) = default_kinetic_weapon {
                    default_kinetic_weapons.insert(resource_id.clone(), default_kinetic_weapon);
                }
                if !default_attribute_modifier_entries.is_empty() {
                    default_attribute_modifiers.insert(
                        resource_id.clone(),
                        default_attribute_modifier_entries.clone(),
                    );
                }
                if let Some(default_use_effect) = default_use_effect {
                    default_use_effects.insert(resource_id.clone(), default_use_effect);
                }
                if let Some(default_consumable) = default_consumable {
                    default_consumables.insert(resource_id.clone(), default_consumable);
                }
                if let Some(crafting_remainder) = &crafting_remainder {
                    crafting_remainders.insert(resource_id.clone(), crafting_remainder.clone());
                }
                if let Some(profile) = &mining_profile {
                    mining_profiles.insert(resource_id.clone(), profile.clone());
                }
            }
            if is_block_item {
                let mut seen_constants = BTreeSet::new();
                let block_constants = expression
                    .field_accesses()
                    .filter_map(|field| field.field_constant("Blocks"))
                    .filter(|constant| seen_constants.insert(*constant))
                    .collect::<Vec<_>>();
                for constant in &block_constants {
                    if ids.len() == 1 {
                        let block_resource_id =
                            block_resources_by_constant.get(*constant).ok_or_else(|| {
                                anyhow::anyhow!(
                                    "{}: unknown Blocks.{constant} block-item reference",
                                    source_context()
                                )
                            })?;
                        insert_unique_resource_mapping(
                            &mut block_item_resource_ids,
                            block_resource_id,
                            &ids[0],
                            &source_context(),
                        )?;
                    }
                }
                if ids.len() == 1 {
                    let Some(primary_constant) = block_constants.first() else {
                        bail!("{}: block item has no Blocks reference", source_context());
                    };
                    let primary = block_resources_by_constant
                        .get(*primary_constant)
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                            "{}: unknown primary Blocks.{primary_constant} block-item reference",
                            source_context()
                        )
                        })?;
                    insert_unique_resource_mapping(
                        &mut primary_block_resource_ids,
                        &ids[0],
                        primary,
                        &source_context(),
                    )?;
                }
                for resource_id in &ids {
                    if ids.len() > 1
                        || block_resources_by_constant
                            .values()
                            .any(|block| block == resource_id)
                    {
                        insert_unique_resource_mapping(
                            &mut block_item_resource_ids,
                            resource_id,
                            resource_id,
                            &source_context(),
                        )?;
                        insert_unique_resource_mapping(
                            &mut primary_block_resource_ids,
                            resource_id,
                            resource_id,
                            &source_context(),
                        )?;
                    }
                }
            }
            resource_ids.extend(ids);
        }

        if resource_ids.is_empty() {
            bail!("Items.java did not contain item registry declarations");
        }

        let mut protocol_ids = BTreeMap::new();
        for (protocol_id, resource_id) in resource_ids.iter().enumerate() {
            if protocol_ids
                .insert(resource_id.clone(), protocol_id as i32)
                .is_some()
            {
                bail!("duplicate item registry id {resource_id}");
            }
        }

        Ok(Self {
            resource_ids,
            protocol_ids,
            max_damage,
            max_stack_size,
            default_item_name_translation_keys,
            default_component_type_ids,
            default_dye_color_ids,
            registry_dependent_component_initializers,
            block_items,
            block_item_resource_ids,
            primary_block_resource_ids,
            spawn_egg_items,
            default_equipment_slots,
            humanoid_armor_assets,
            equippable_assets,
            default_mount_body_armor_kinds,
            mount_body_armor_assets,
            default_piercing_weapon_ids,
            default_attack_ranges,
            default_swing_animation_durations,
            default_kinetic_weapons,
            default_attribute_modifiers,
            default_use_effects,
            default_consumables,
            crafting_remainders,
            mining_profiles,
        })
    }

    pub fn resource_id(&self, protocol_id: i32) -> Option<&str> {
        usize::try_from(protocol_id)
            .ok()
            .and_then(|protocol_id| self.resource_ids.get(protocol_id))
            .map(String::as_str)
    }

    pub fn protocol_id(&self, resource_id: &str) -> Option<i32> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.protocol_ids.get(&resource_id).copied()
    }

    pub fn resource_ids(&self) -> &[String] {
        &self.resource_ids
    }

    pub fn is_block_item(&self, resource_id: &str) -> bool {
        ResourceLocation::parse(resource_id)
            .ok()
            .is_some_and(|id| self.block_items.contains(&id.id()))
    }

    pub fn item_resource_id_for_block(&self, block_resource_id: &str) -> Option<&str> {
        let block_resource_id = ResourceLocation::parse(block_resource_id).ok()?.id();
        self.block_item_resource_ids
            .get(&block_resource_id)
            .map(String::as_str)
    }

    pub fn primary_block_resource_id_for_item(&self, item_resource_id: &str) -> Option<&str> {
        let item_resource_id = ResourceLocation::parse(item_resource_id).ok()?.id();
        self.primary_block_resource_ids
            .get(&item_resource_id)
            .map(String::as_str)
    }

    pub fn is_spawn_egg_item(&self, resource_id: &str) -> bool {
        ResourceLocation::parse(resource_id)
            .ok()
            .is_some_and(|id| self.spawn_egg_items.contains(&id.id()))
    }

    pub fn max_damage(&self, resource_id: &str) -> Option<i32> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.max_damage.get(&resource_id).copied()
    }

    pub fn max_damage_protocol_ids(&self) -> BTreeSet<i32> {
        self.max_damage
            .keys()
            .filter_map(|resource_id| self.protocol_ids.get(resource_id).copied())
            .collect()
    }

    pub fn max_stack_size(&self, resource_id: &str) -> Option<i32> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.max_stack_size.get(&resource_id).copied()
    }

    pub fn default_item_name_translation_key(&self, resource_id: &str) -> Option<&str> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.default_item_name_translation_keys
            .get(&resource_id)
            .map(String::as_str)
    }

    pub fn default_component_type_ids(&self, resource_id: &str) -> Option<&BTreeSet<i32>> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.default_component_type_ids.get(&resource_id)
    }

    pub fn default_component_type_ids_by_protocol_id(&self) -> BTreeMap<i32, BTreeSet<i32>> {
        self.default_component_type_ids
            .iter()
            .filter_map(|(resource_id, type_ids)| {
                self.protocol_ids
                    .get(resource_id)
                    .copied()
                    .map(|item_id| (item_id, type_ids.clone()))
            })
            .collect()
    }

    pub fn default_dye_color_ids_by_protocol_id(&self) -> BTreeMap<i32, i32> {
        self.default_dye_color_ids
            .iter()
            .filter_map(|(resource_id, dye_color_id)| {
                self.protocol_ids
                    .get(resource_id)
                    .copied()
                    .map(|item_id| (item_id, *dye_color_id))
            })
            .collect()
    }

    /// Fixed item protocol ids paired with the source initializer rules that
    /// must be rebuilt against each committed dynamic-registry revision.
    pub fn registry_dependent_component_initializers_by_protocol_id(
        &self,
    ) -> BTreeMap<i32, Vec<RegistryDependentItemComponentInitializer>> {
        self.registry_dependent_component_initializers
            .iter()
            .filter_map(|(resource_id, initializers)| {
                self.protocol_ids
                    .get(resource_id)
                    .copied()
                    .map(|item_id| (item_id, initializers.clone()))
            })
            .collect()
    }

    pub fn equipment_slot(&self, resource_id: &str) -> Option<ItemEquipmentSlot> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.default_equipment_slots.get(&resource_id).copied()
    }

    /// The humanoid armor equipment-asset name for an armor item (e.g. `iron`, `chainmail`,
    /// `turtle_scute`), or `None` for a non-humanoid-armor item.
    pub fn humanoid_armor_asset(&self, resource_id: &str) -> Option<&str> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.humanoid_armor_assets
            .get(&resource_id)
            .map(String::as_str)
    }

    pub fn equippable_asset(&self, resource_id: &str) -> Option<&str> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.equippable_assets.get(&resource_id).map(String::as_str)
    }

    pub fn mount_body_armor_kind(&self, resource_id: &str) -> Option<ItemMountBodyArmorKind> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.default_mount_body_armor_kinds
            .get(&resource_id)
            .copied()
    }

    /// The equipment-asset material name for a horse/nautilus body armor item (e.g. `iron`,
    /// `diamond`, `netherite`), or `None` for llama carpets and non-mount body armor.
    pub fn mount_body_armor_asset(&self, resource_id: &str) -> Option<&str> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.mount_body_armor_assets
            .get(&resource_id)
            .map(String::as_str)
    }

    pub fn mount_body_armor_kinds_by_protocol_id(&self) -> BTreeMap<i32, ItemMountBodyArmorKind> {
        self.default_mount_body_armor_kinds
            .iter()
            .filter_map(|(resource_id, kind)| {
                self.protocol_ids
                    .get(resource_id)
                    .copied()
                    .map(|item_id| (item_id, *kind))
            })
            .collect()
    }

    pub fn default_piercing_weapon(&self, resource_id: &str) -> bool {
        let Some(resource_id) = ResourceLocation::parse(resource_id).ok().map(|id| id.id()) else {
            return false;
        };
        self.default_piercing_weapon_ids.contains(&resource_id)
    }

    pub fn default_piercing_weapon_protocol_ids(&self) -> BTreeSet<i32> {
        self.default_piercing_weapon_ids
            .iter()
            .filter_map(|resource_id| self.protocol_ids.get(resource_id).copied())
            .collect()
    }

    pub fn default_attack_range(&self, resource_id: &str) -> Option<ItemAttackRange> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.default_attack_ranges.get(&resource_id).copied()
    }

    pub fn default_swing_animation_duration(&self, resource_id: &str) -> Option<i32> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.default_swing_animation_durations
            .get(&resource_id)
            .copied()
    }

    pub fn default_kinetic_weapon(&self, resource_id: &str) -> Option<ItemKineticWeaponDefaults> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.default_kinetic_weapons.get(&resource_id).copied()
    }

    pub fn default_attribute_modifiers(
        &self,
        resource_id: &str,
    ) -> Option<&[ItemDefaultAttributeModifier]> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.default_attribute_modifiers
            .get(&resource_id)
            .map(Vec::as_slice)
    }

    pub fn default_use_effects(&self, resource_id: &str) -> Option<ItemUseEffects> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.default_use_effects.get(&resource_id).copied()
    }

    pub fn default_consumable(&self, resource_id: &str) -> Option<ItemConsumable> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.default_consumables.get(&resource_id).copied()
    }

    pub fn crafting_remainder(&self, resource_id: &str) -> Option<&str> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.crafting_remainders
            .get(&resource_id)
            .map(String::as_str)
    }

    pub fn crafting_remainders_by_protocol_id(&self) -> BTreeMap<i32, i32> {
        self.crafting_remainders
            .iter()
            .filter_map(|(resource_id, remainder_id)| {
                let item_id = self.protocol_ids.get(resource_id).copied()?;
                let remainder_item_id = self.protocol_ids.get(remainder_id).copied()?;
                Some((item_id, remainder_item_id))
            })
            .collect()
    }

    pub fn mining_profile(&self, resource_id: &str) -> Option<&ItemMiningProfile> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.mining_profiles.get(&resource_id)
    }

    pub fn len(&self) -> usize {
        self.resource_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.resource_ids.is_empty()
    }
}

fn resource_ids_for_declaration(
    kind: &str,
    field: &str,
    expression: &GeneratedJavaExpression,
    item_id_constants: &BTreeMap<String, String>,
) -> Result<Vec<String>> {
    let root_name = expression.name.as_deref();
    let first_argument = expression.arguments.first();
    if kind == "WeatheringCopperItems" {
        let base = first_argument
            .and_then(|argument| argument.field_constant("Blocks"))
            .ok_or_else(|| anyhow::anyhow!("unsupported item registry declaration {field}"))?;
        return weathering_copper_ids(&base.to_ascii_lowercase());
    }

    if root_name == Some("registerBlock") {
        return Ok(vec![minecraft_id(
            &first_argument
                .and_then(|argument| argument.field_constant("Blocks"))
                .ok_or_else(|| anyhow::anyhow!("unsupported item registry declaration {field}"))?
                .to_ascii_lowercase(),
        )?]);
    }

    if root_name == Some("registerSpawnEgg") {
        let entity_type = first_argument
            .and_then(|argument| argument.field_constant("EntityType"))
            .ok_or_else(|| anyhow::anyhow!("unsupported item registry declaration {field}"))?;
        return Ok(vec![minecraft_id(&format!(
            "{}_spawn_egg",
            entity_type.to_ascii_lowercase()
        ))?]);
    }

    if root_name == Some("registerItem") {
        if let Some(name) = first_argument
            .map(GeneratedJavaExpression::string_literal)
            .transpose()?
            .flatten()
        {
            return Ok(vec![minecraft_id(&name)?]);
        }

        let constant = first_argument
            .and_then(|argument| argument.field_constant("ItemIds"))
            .ok_or_else(|| anyhow::anyhow!("unsupported item registry declaration {field}"))?;
        let name = item_id_constants
            .get(constant)
            .cloned()
            .unwrap_or_else(|| constant.to_ascii_lowercase());
        return Ok(vec![minecraft_id(&name)?]);
    }

    bail!(
        "unsupported item registry declaration {field}: {}",
        expression.canonical
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DefaultItemName {
    Prefix(&'static str),
    Fixed(String),
}

impl DefaultItemName {
    fn translation_key(&self, resource_id: &str) -> String {
        match self {
            Self::Prefix(prefix) => description_translation_key(prefix, resource_id),
            Self::Fixed(key) => key.clone(),
        }
    }
}

fn default_item_name_for_declaration(
    kind: &str,
    expression: &GeneratedJavaExpression,
) -> Result<DefaultItemName> {
    if let Some(description_id) = expression
        .call("overrideDescription")
        .and_then(|call| call.arguments.first())
        .map(GeneratedJavaExpression::string_literal)
        .transpose()?
        .flatten()
    {
        return Ok(DefaultItemName::Fixed(description_id));
    }

    if kind == "WeatheringCopperItems" {
        return Ok(DefaultItemName::Prefix("block"));
    }

    if expression.name.as_deref() == Some("registerBlock")
        && expression.call("useItemDescriptionPrefix").is_none()
    {
        return Ok(DefaultItemName::Prefix("block"));
    }

    Ok(DefaultItemName::Prefix("item"))
}

fn description_translation_key(prefix: &str, resource_id: &str) -> String {
    let (namespace, path) = resource_id
        .split_once(':')
        .unwrap_or(("minecraft", resource_id));
    format!("{prefix}.{namespace}.{}", path.replace('/', "."))
}

fn weathering_copper_ids(base: &str) -> Result<Vec<String>> {
    [
        base.to_string(),
        format!("exposed_{base}"),
        format!("weathered_{base}"),
        format!("oxidized_{base}"),
        format!("waxed_{base}"),
        format!("waxed_exposed_{base}"),
        format!("waxed_weathered_{base}"),
        format!("waxed_oxidized_{base}"),
    ]
    .into_iter()
    .map(|id| minecraft_id(&id))
    .collect()
}

fn durability_for_declaration(expression: &GeneratedJavaExpression) -> Result<Option<i32>> {
    if let Some(durability) = call_integer_argument(expression, "durability", 0)? {
        return Ok(Some(durability));
    }

    if let Some(material) = tool_material_for_durability(expression)? {
        return Ok(Some(tool_material_durability(&material)?));
    }

    if let Some((material, armor_type)) = humanoid_armor_material_and_type(expression)? {
        return Ok(Some(
            armor_material_durability_multiplier(&material)?
                * armor_type_unit_durability(&armor_type)?,
        ));
    }

    if let Some(material) = call_field_argument(expression, "wolfArmor", 0, "ArmorMaterials") {
        return Ok(Some(
            armor_material_durability_multiplier(&material)? * armor_type_unit_durability("BODY")?,
        ));
    }

    Ok(None)
}

fn default_component_type_ids_for_declaration(
    expression: &GeneratedJavaExpression,
) -> Result<BTreeSet<i32>> {
    let mut type_ids = component_id::VANILLA_COMMON_ITEM_COMPONENT_TYPE_IDS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    type_ids.insert(component_id::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID);
    type_ids.insert(component_id::VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID);

    if durability_for_declaration(expression)?.is_some() {
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID);
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    }
    if expression.call("food").is_some() {
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_FOOD_TYPE_ID);
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID);
    }
    if expression.call("usingConvertsTo").is_some() {
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID);
    }
    if expression.call("useCooldown").is_some() {
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID);
    }
    if expression.call("fireResistant").is_some() {
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID);
    }
    if expression.call("jukeboxPlayable").is_some() {
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID);
    }
    if expression.name.as_deref() == Some("registerSpawnEgg")
        || expression.call("spawnEgg").is_some()
    {
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID);
    }
    if applies_tool_material_defaults(expression) {
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID);
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID);
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID);
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID);
    }
    if expression.call("spear").is_some() {
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID);
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID);
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID);
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID);
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID);
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID);
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID);
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID);
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID);
    }
    if expression.call("humanoidArmor").is_some() {
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID);
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID);
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID);
    }
    if expression.call("wolfArmor").is_some() {
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID);
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID);
    }
    if expression.call("horseArmor").is_some() || expression.call("nautilusArmor").is_some() {
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID);
    }
    if [
        "equippable",
        "equippableUnswappable",
        "builder",
        "saddle",
        "llamaSwag",
        "harness",
    ]
    .iter()
    .any(|name| expression.call(name).is_some())
    {
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID);
    }
    if expression.call("repairable").is_some() {
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID);
    }
    if expression.call("enchantable").is_some() {
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID);
    }
    if expression.call("trimMaterial").is_some() {
        type_ids.insert(component_id::VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID);
    }

    for name in expression
        .field_accesses()
        .filter_map(|field| field.field_constant("DataComponents"))
    {
        let Some(type_id) = data_component_type_id_from_name(name) else {
            bail!("unknown item data component DataComponents.{name}");
        };
        type_ids.insert(type_id);
    }

    Ok(type_ids)
}

fn default_dye_color_for_declaration(expression: &GeneratedJavaExpression) -> Result<Option<i32>> {
    let mut dye_color_id = None;
    for call in expression
        .calls()
        .filter(|call| call.name.as_deref() == Some("component"))
    {
        if call
            .arguments
            .first()
            .and_then(|argument| argument.field_constant("DataComponents"))
            != Some("DYE")
        {
            continue;
        }
        let constant_name = call
            .arguments
            .get(1)
            .and_then(|argument| argument.field_constant("DyeColor"))
            .ok_or_else(|| anyhow::anyhow!("DYE component requires a DyeColor constant"))?;
        let value = VANILLA_DYE_COLOR_VALUES
            .iter()
            .find(|value| value.constant_name == constant_name)
            .ok_or_else(|| anyhow::anyhow!("unknown DyeColor.{constant_name}"))?
            .id;
        if dye_color_id.replace(value).is_some() {
            bail!("item declaration contains duplicate DYE components");
        }
    }
    Ok(dye_color_id)
}

fn registry_dependent_component_initializers_for_declaration(
    expression: &GeneratedJavaExpression,
    resource_ids: &[String],
) -> Result<Vec<RegistryDependentItemComponentInitializer>> {
    let mut initializers = Vec::new();
    if expression.call("fireResistant").is_some() {
        initializers.push(RegistryDependentItemComponentInitializer::DamageResistant {
            tag: "minecraft:is_fire".to_string(),
        });
    }
    if let Some(call) = expression.call("jukeboxPlayable") {
        call.arguments
            .first()
            .and_then(|argument| argument.field_constant("JukeboxSongs"))
            .ok_or_else(|| anyhow::anyhow!("jukeboxPlayable requires a JukeboxSongs key"))?;
        let [item_id] = resource_ids else {
            return Err(anyhow::anyhow!(
                "jukeboxPlayable declaration must register exactly one item"
            ));
        };
        let song = item_id
            .strip_prefix("minecraft:music_disc_")
            .ok_or_else(|| anyhow::anyhow!("jukeboxPlayable item has unexpected id {item_id}"))?;
        initializers.push(RegistryDependentItemComponentInitializer::JukeboxPlayable {
            song: format!("minecraft:{song}"),
        });
    }
    if let Some(call) = expression.call("trimMaterial") {
        let material = call
            .arguments
            .first()
            .and_then(|argument| argument.field_constant("TrimMaterials"))
            .ok_or_else(|| anyhow::anyhow!("trimMaterial requires a TrimMaterials key"))?;
        initializers.push(
            RegistryDependentItemComponentInitializer::ProvidesTrimMaterial {
                material: format!("minecraft:{}", material.to_ascii_lowercase()),
            },
        );
    }

    for call in expression
        .calls()
        .filter(|call| call.name.as_deref() == Some("delayedComponent"))
    {
        let Some(component) = call
            .arguments
            .first()
            .and_then(|argument| argument.field_constant("DataComponents"))
        else {
            continue;
        };
        let initializer = match component {
            "DAMAGE_RESISTANT" => RegistryDependentItemComponentInitializer::DamageResistant {
                tag: required_tag_resource_id(call, "DamageTypeTags")?,
            },
            "BLOCKS_ATTACKS" => RegistryDependentItemComponentInitializer::BlocksAttacks,
            "PROVIDES_BANNER_PATTERNS" => {
                let constant = required_field_constant(call, "BannerPatternTags")?;
                let suffix = constant
                    .strip_prefix("PATTERN_ITEM_")
                    .ok_or_else(|| anyhow::anyhow!("unknown banner-pattern item tag {constant}"))?
                    .to_ascii_lowercase();
                RegistryDependentItemComponentInitializer::ProvidesBannerPatterns {
                    tag: format!("minecraft:pattern_item/{suffix}"),
                }
            }
            "INSTRUMENT" => RegistryDependentItemComponentInitializer::Instrument {
                instrument: required_registry_key_resource_id(call, "Instruments")?,
            },
            _ => continue,
        };
        initializers.push(initializer);
    }
    Ok(initializers)
}

fn required_tag_resource_id(expression: &GeneratedJavaExpression, owner: &str) -> Result<String> {
    Ok(format!(
        "minecraft:{}",
        required_field_constant(expression, owner)?.to_ascii_lowercase()
    ))
}

fn required_registry_key_resource_id(
    expression: &GeneratedJavaExpression,
    owner: &str,
) -> Result<String> {
    required_tag_resource_id(expression, owner)
}

fn required_field_constant<'a>(
    expression: &'a GeneratedJavaExpression,
    owner: &str,
) -> Result<&'a str> {
    expression
        .field_accesses()
        .find_map(|field| field.field_constant(owner))
        .ok_or_else(|| anyhow::anyhow!("missing {owner} field reference"))
}

fn applies_tool_material_defaults(expression: &GeneratedJavaExpression) -> bool {
    ["tool", "sword", "pickaxe", "axe", "hoe", "shovel"]
        .iter()
        .any(|name| expression.call(name).is_some())
        || ["PickaxeItem", "AxeItem", "HoeItem", "ShovelItem"]
            .iter()
            .any(|name| {
                expression
                    .creations()
                    .any(|creation| creation.name.as_deref() == Some(name))
            })
}

fn data_component_type_id_from_name(name: &str) -> Option<i32> {
    component_id::VANILLA_DATA_COMPONENT_TYPES
        .iter()
        .find(|entry| entry.constant_name == name)
        .map(|entry| entry.type_id)
}

fn tool_material_for_durability(expression: &GeneratedJavaExpression) -> Result<Option<String>> {
    for name in ["sword", "pickaxe", "axe", "hoe", "shovel", "spear"] {
        if let Some(material) = call_field_argument(expression, name, 0, "ToolMaterial") {
            return Ok(Some(material.to_string()));
        }
    }
    for name in ["PickaxeItem", "AxeItem", "HoeItem", "ShovelItem"] {
        if let Some(material) = creation_field_argument(expression, name, 0, "ToolMaterial") {
            return Ok(Some(material.to_string()));
        }
    }
    Ok(None)
}

fn tool_material_durability(material: &str) -> Result<i32> {
    match material {
        "WOOD" => Ok(59),
        "STONE" => Ok(131),
        "COPPER" => Ok(190),
        "IRON" => Ok(250),
        "DIAMOND" => Ok(1561),
        "GOLD" => Ok(32),
        "NETHERITE" => Ok(2031),
        _ => bail!("unsupported tool material ToolMaterial.{material}"),
    }
}

/// The humanoid armor equipment-asset name for a `.humanoidArmor(ArmorMaterials.<MAT>, ...)`
/// declaration. Vanilla `ArmorMaterials.<MAT>` carries `EquipmentAssets.<MAT>`, whose id is the
/// lowercased material name (`CHAINMAIL` → `chainmail`, `TURTLE_SCUTE` → `turtle_scute`), so the asset
/// is the material captured by [`humanoid_armor_material_and_type`] lowercased.
fn humanoid_armor_asset_for_declaration(
    expression: &GeneratedJavaExpression,
) -> Result<Option<String>> {
    Ok(humanoid_armor_material_and_type(expression)?
        .map(|(material, _armor_type)| material.to_lowercase()))
}

fn explicit_equippable_asset_for_declaration(
    expression: &GeneratedJavaExpression,
) -> Result<Option<String>> {
    Ok(call_field_argument(expression, "setAsset", 0, "EquipmentAssets").map(str::to_lowercase))
}

fn wolf_armor_asset_for_declaration(
    expression: &GeneratedJavaExpression,
) -> Result<Option<String>> {
    Ok(call_field_argument(expression, "wolfArmor", 0, "ArmorMaterials").map(str::to_lowercase))
}

fn humanoid_armor_material_and_type(
    expression: &GeneratedJavaExpression,
) -> Result<Option<(String, String)>> {
    let Some(call) = expression.call("humanoidArmor") else {
        return Ok(None);
    };
    let Some(material) = call
        .arguments
        .first()
        .and_then(|argument| argument.field_constant("ArmorMaterials"))
    else {
        bail!("humanoidArmor declaration lacks ArmorMaterials argument");
    };
    let Some(armor_type) = call
        .arguments
        .get(1)
        .and_then(|argument| argument.field_constant("ArmorType"))
    else {
        bail!("humanoidArmor declaration lacks ArmorType argument");
    };
    Ok(Some((material.to_string(), armor_type.to_string())))
}

fn mount_body_armor_asset_for_declaration(
    expression: &GeneratedJavaExpression,
) -> Result<Option<String>> {
    for name in ["horseArmor", "nautilusArmor"] {
        if let Some(material) = call_field_argument(expression, name, 0, "ArmorMaterials") {
            return Ok(Some(material.to_lowercase()));
        }
    }
    Ok(None)
}

fn armor_material_durability_multiplier(material: &str) -> Result<i32> {
    match material {
        "LEATHER" => Ok(5),
        "COPPER" => Ok(11),
        "CHAINMAIL" | "IRON" => Ok(15),
        "GOLD" => Ok(7),
        "DIAMOND" => Ok(33),
        "TURTLE_SCUTE" => Ok(25),
        "NETHERITE" => Ok(37),
        "ARMADILLO_SCUTE" => Ok(4),
        _ => bail!("unsupported armor material ArmorMaterials.{material}"),
    }
}

fn armor_type_unit_durability(armor_type: &str) -> Result<i32> {
    match armor_type {
        "HELMET" => Ok(11),
        "CHESTPLATE" | "BODY" => Ok(16),
        "LEGGINGS" => Ok(15),
        "BOOTS" => Ok(13),
        _ => bail!("unsupported armor type ArmorType.{armor_type}"),
    }
}

fn max_stack_size_for_declaration(expression: &GeneratedJavaExpression) -> Result<i32> {
    let stack_size = call_integer_argument(expression, "stacksTo", 0)?;
    if let Some(stack_size) = stack_size {
        return Ok(stack_size);
    }

    if [
        "durability",
        "sword",
        "pickaxe",
        "axe",
        "hoe",
        "shovel",
        "spear",
        "humanoidArmor",
        "wolfArmor",
        "horseArmor",
        "nautilusArmor",
    ]
    .iter()
    .any(|name| expression.call(name).is_some())
        || ["AxeItem", "HoeItem", "ShovelItem"].iter().any(|name| {
            expression
                .creations()
                .any(|creation| creation.name.as_deref() == Some(name))
        })
    {
        return Ok(1);
    }

    Ok(64)
}

fn equipment_slot_for_declaration(
    expression: &GeneratedJavaExpression,
) -> Result<Option<ItemEquipmentSlot>> {
    if qualified_call(expression, "saddle", "Equippable").is_some() {
        return Ok(Some(ItemEquipmentSlot::Saddle));
    }
    if expression.call("horseArmor").is_some()
        || expression.call("nautilusArmor").is_some()
        || expression.call("wolfArmor").is_some()
        || qualified_call(expression, "llamaSwag", "Equippable").is_some()
    {
        return Ok(Some(ItemEquipmentSlot::Body));
    }

    for name in ["builder", "equippableUnswappable", "equippable"] {
        if let Some(slot) = call_field_argument(expression, name, 0, "EquipmentSlot") {
            return equipment_slot_from_name(slot).map(Some);
        }
    }

    if let Some((_, armor_type)) = humanoid_armor_material_and_type(expression)? {
        return armor_type_equipment_slot(&armor_type).map(Some);
    }

    Ok(None)
}

fn mount_body_armor_kind_for_declaration(
    expression: &GeneratedJavaExpression,
) -> Option<ItemMountBodyArmorKind> {
    if expression.call("horseArmor").is_some() {
        Some(ItemMountBodyArmorKind::Horse)
    } else if qualified_call(expression, "llamaSwag", "Equippable").is_some() {
        Some(ItemMountBodyArmorKind::Llama)
    } else if expression.call("nautilusArmor").is_some() {
        Some(ItemMountBodyArmorKind::Nautilus)
    } else {
        None
    }
}

fn default_piercing_weapon_for_declaration(expression: &GeneratedJavaExpression) -> bool {
    expression.call("spear").is_some()
}

fn default_attack_range_for_declaration(
    expression: &GeneratedJavaExpression,
) -> Result<Option<ItemAttackRange>> {
    if expression.call("spear").is_some() {
        return Ok(Some(ItemAttackRange {
            min_reach: 2.0,
            max_reach: 4.5,
            min_creative_reach: 2.0,
            max_creative_reach: 6.5,
            hitbox_margin: 0.125,
            mob_factor: 0.5,
        }));
    }

    let Some(value) = component_value(expression, "ATTACK_RANGE") else {
        return Ok(None);
    };
    let Some(creation) = value
        .creations()
        .find(|creation| creation.name.as_deref() == Some("AttackRange"))
    else {
        bail!("ATTACK_RANGE component lacks AttackRange constructor");
    };
    if creation.arguments.len() != 6 {
        bail!("AttackRange constructor must have six arguments");
    }
    Ok(Some(ItemAttackRange {
        min_reach: parse_java_float_literal(&creation.arguments[0].canonical)?,
        max_reach: parse_java_float_literal(&creation.arguments[1].canonical)?,
        min_creative_reach: parse_java_float_literal(&creation.arguments[2].canonical)?,
        max_creative_reach: parse_java_float_literal(&creation.arguments[3].canonical)?,
        hitbox_margin: parse_java_float_literal(&creation.arguments[4].canonical)?,
        mob_factor: parse_java_float_literal(&creation.arguments[5].canonical)?,
    }))
}

fn default_kinetic_weapon_for_declaration(
    expression: &GeneratedJavaExpression,
) -> Result<Option<ItemKineticWeaponDefaults>> {
    let Some(call) = expression.call("spear") else {
        return Ok(None);
    };
    if call.arguments.len() != 10 {
        bail!("spear declaration must have ten arguments");
    }
    let ticks = |index: usize, field: &str| -> Result<i32> {
        let seconds = parse_java_float_literal(&call.arguments[index].canonical)?;
        if !seconds.is_finite() || seconds < 0.0 || seconds * 20.0 > i32::MAX as f32 {
            bail!("spear {field} must be a finite non-negative tick duration");
        }
        Ok((seconds * 20.0) as i32)
    };
    Ok(Some(ItemKineticWeaponDefaults {
        delay_ticks: ticks(3, "delay")?,
        dismount_duration_ticks: ticks(4, "dismount time")?,
        knockback_duration_ticks: ticks(6, "knockback time")?,
        damage_duration_ticks: ticks(8, "damage time")?,
        // Vanilla `Item.Properties.spear` supplies this fixed presentation
        // multiplier to every default KineticWeapon component.
        forward_movement: 0.38,
    }))
}

fn default_swing_animation_duration_for_declaration(
    expression: &GeneratedJavaExpression,
) -> Result<Option<i32>> {
    if let Some(attack_duration) = spear_attack_duration_for_declaration(expression)? {
        return Ok(Some((attack_duration * 20.0) as i32));
    }

    let Some(value) = component_value(expression, "SWING_ANIMATION") else {
        return Ok(None);
    };
    let Some(creation) = value
        .creations()
        .find(|creation| creation.name.as_deref() == Some("SwingAnimation"))
    else {
        bail!("SWING_ANIMATION component lacks SwingAnimation constructor");
    };
    let duration = creation
        .arguments
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("SwingAnimation constructor lacks duration"))?;
    if duration.kind == "decimal_integer_literal" {
        return Ok(Some(duration.canonical.parse()?));
    }
    let numbers = duration
        .children
        .iter()
        .flat_map(|child| child.children.iter().chain(std::iter::once(child)))
        .filter(|child| child.kind == "decimal_floating_point_literal")
        .collect::<Vec<_>>();
    let seconds = numbers
        .iter()
        .find(|number| number.canonical.trim_end_matches('F') != "20.0")
        .ok_or_else(|| {
            anyhow::anyhow!("unsupported SwingAnimation duration {}", duration.canonical)
        })?;
    Ok(Some(
        (parse_java_float_literal(&seconds.canonical)? * 20.0) as i32,
    ))
}

fn default_attribute_modifiers_for_declaration(
    expression: &GeneratedJavaExpression,
) -> Result<Vec<ItemDefaultAttributeModifier>> {
    if let Some((material, armor_type)) = humanoid_armor_material_and_type(expression)? {
        return armor_default_attribute_modifiers(&material, &armor_type);
    }
    for name in ["wolfArmor", "horseArmor", "nautilusArmor"] {
        if let Some(material) = call_field_argument(expression, name, 0, "ArmorMaterials") {
            return armor_default_attribute_modifiers(material, "BODY");
        }
    }
    if call_has_qualified_call_argument(expression, "attributes", 0, "MaceItem", "createAttributes")
    {
        return Ok(attribute_modifier_entries(5.0, f64::from(-3.4_f32)));
    }
    if call_has_qualified_call_argument(
        expression,
        "attributes",
        0,
        "TridentItem",
        "createAttributes",
    ) {
        return Ok(attribute_modifier_entries(8.0, f64::from(-2.9_f32)));
    }

    for name in ["sword", "pickaxe", "axe", "hoe", "shovel"] {
        if let Some(call) = expression.call(name) {
            if let Some(material) = call
                .arguments
                .first()
                .and_then(|argument| argument.field_constant("ToolMaterial"))
            {
                if call.arguments.len() < 3 {
                    bail!("{name} declaration lacks attribute arguments");
                }
                return tool_default_attribute_modifiers(
                    material,
                    parse_java_float_literal(&call.arguments[1].canonical)?,
                    parse_java_float_literal(&call.arguments[2].canonical)?,
                );
            }
        }
    }

    for name in ["PickaxeItem", "AxeItem", "HoeItem", "ShovelItem"] {
        if let Some(creation) = expression
            .creations()
            .find(|creation| creation.name.as_deref() == Some(name))
        {
            if let Some(material) = creation
                .arguments
                .first()
                .and_then(|argument| argument.field_constant("ToolMaterial"))
            {
                if creation.arguments.len() < 3 {
                    bail!("{name} declaration lacks attribute arguments");
                }
                return tool_default_attribute_modifiers(
                    material,
                    parse_java_float_literal(&creation.arguments[1].canonical)?,
                    parse_java_float_literal(&creation.arguments[2].canonical)?,
                );
            }
        }
    }

    if let Some((material, attack_duration)) = spear_material_and_attack_duration(expression)? {
        let attack_damage = f64::from(material_attack_damage_bonus(&material)?);
        let attack_speed = f64::from(1.0_f32 / attack_duration) - 4.0;
        return Ok(attribute_modifier_entries(attack_damage, attack_speed));
    }

    Ok(Vec::new())
}

fn tool_default_attribute_modifiers(
    material: &str,
    attack_damage_baseline: f32,
    attack_speed_baseline: f32,
) -> Result<Vec<ItemDefaultAttributeModifier>> {
    let attack_damage = f64::from(attack_damage_baseline + material_attack_damage_bonus(material)?);
    Ok(attribute_modifier_entries(
        attack_damage,
        f64::from(attack_speed_baseline),
    ))
}

fn armor_default_attribute_modifiers(
    material: &str,
    armor_type: &str,
) -> Result<Vec<ItemDefaultAttributeModifier>> {
    let modifier_id = format!("minecraft:armor.{}", armor_type_name(armor_type)?);
    let slot_id = armor_type_slot_group_id(armor_type)?;
    let mut modifiers = vec![
        ItemDefaultAttributeModifier {
            attribute_key: "minecraft:armor".to_string(),
            modifier_id: modifier_id.clone(),
            amount_bits: f64::from(armor_material_defense(material, armor_type)?).to_bits(),
            operation_id: VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            slot_id,
        },
        ItemDefaultAttributeModifier {
            attribute_key: "minecraft:armor_toughness".to_string(),
            modifier_id: modifier_id.clone(),
            amount_bits: f64::from(armor_material_toughness(material)?).to_bits(),
            operation_id: VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            slot_id,
        },
    ];
    let knockback_resistance = armor_material_knockback_resistance(material)?;
    if knockback_resistance > 0.0 {
        modifiers.push(ItemDefaultAttributeModifier {
            attribute_key: "minecraft:knockback_resistance".to_string(),
            modifier_id,
            amount_bits: f64::from(knockback_resistance).to_bits(),
            operation_id: VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            slot_id,
        });
    }
    Ok(modifiers)
}

fn armor_type_name(armor_type: &str) -> Result<&'static str> {
    match armor_type {
        "HELMET" => Ok("helmet"),
        "CHESTPLATE" => Ok("chestplate"),
        "LEGGINGS" => Ok("leggings"),
        "BOOTS" => Ok("boots"),
        "BODY" => Ok("body"),
        _ => bail!("unsupported armor type ArmorType.{armor_type}"),
    }
}

fn armor_type_slot_group_id(armor_type: &str) -> Result<i32> {
    match armor_type {
        "BOOTS" => Ok(VANILLA_EQUIPMENT_SLOT_GROUP_FEET_ID),
        "LEGGINGS" => Ok(VANILLA_EQUIPMENT_SLOT_GROUP_LEGS_ID),
        "CHESTPLATE" => Ok(VANILLA_EQUIPMENT_SLOT_GROUP_CHEST_ID),
        "HELMET" => Ok(VANILLA_EQUIPMENT_SLOT_GROUP_HEAD_ID),
        "BODY" => Ok(VANILLA_EQUIPMENT_SLOT_GROUP_BODY_ID),
        _ => bail!("unsupported armor type ArmorType.{armor_type}"),
    }
}

fn armor_material_defense(material: &str, armor_type: &str) -> Result<i32> {
    let defenses = match material {
        "LEATHER" => [1, 2, 3, 1, 3],
        "COPPER" => [1, 3, 4, 2, 4],
        "CHAINMAIL" => [1, 4, 5, 2, 4],
        "IRON" => [2, 5, 6, 2, 5],
        "GOLD" => [1, 3, 5, 2, 7],
        "DIAMOND" => [3, 6, 8, 3, 11],
        "TURTLE_SCUTE" => [2, 5, 6, 2, 5],
        "NETHERITE" => [3, 6, 8, 3, 19],
        "ARMADILLO_SCUTE" => [3, 6, 8, 3, 11],
        _ => bail!("unsupported armor material ArmorMaterials.{material}"),
    };
    let index = match armor_type {
        "BOOTS" => 0,
        "LEGGINGS" => 1,
        "CHESTPLATE" => 2,
        "HELMET" => 3,
        "BODY" => 4,
        _ => bail!("unsupported armor type ArmorType.{armor_type}"),
    };
    Ok(defenses[index])
}

fn armor_material_toughness(material: &str) -> Result<f32> {
    match material {
        "DIAMOND" => Ok(2.0),
        "NETHERITE" => Ok(3.0),
        "LEATHER" | "COPPER" | "CHAINMAIL" | "IRON" | "GOLD" | "TURTLE_SCUTE"
        | "ARMADILLO_SCUTE" => Ok(0.0),
        _ => bail!("unsupported armor material ArmorMaterials.{material}"),
    }
}

fn armor_material_knockback_resistance(material: &str) -> Result<f32> {
    match material {
        "NETHERITE" => Ok(0.1),
        "LEATHER" | "COPPER" | "CHAINMAIL" | "IRON" | "GOLD" | "DIAMOND" | "TURTLE_SCUTE"
        | "ARMADILLO_SCUTE" => Ok(0.0),
        _ => bail!("unsupported armor material ArmorMaterials.{material}"),
    }
}

fn attribute_modifier_entries(
    attack_damage: f64,
    attack_speed: f64,
) -> Vec<ItemDefaultAttributeModifier> {
    vec![
        ItemDefaultAttributeModifier {
            attribute_key: "minecraft:attack_damage".to_string(),
            modifier_id: "minecraft:base_attack_damage".to_string(),
            amount_bits: attack_damage.to_bits(),
            operation_id: VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            slot_id: 1,
        },
        ItemDefaultAttributeModifier {
            attribute_key: "minecraft:attack_speed".to_string(),
            modifier_id: "minecraft:base_attack_speed".to_string(),
            amount_bits: attack_speed.to_bits(),
            operation_id: VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            slot_id: 1,
        },
    ]
}

fn spear_attack_duration_for_declaration(
    expression: &GeneratedJavaExpression,
) -> Result<Option<f32>> {
    let Some(call) = expression.call("spear") else {
        return Ok(None);
    };
    let argument = call
        .arguments
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("spear declaration lacks attack duration"))?;
    Ok(Some(parse_java_float_literal(&argument.canonical)?))
}

fn spear_material_and_attack_duration(
    expression: &GeneratedJavaExpression,
) -> Result<Option<(String, f32)>> {
    let Some(call) = expression.call("spear") else {
        return Ok(None);
    };
    let material = call
        .arguments
        .first()
        .and_then(|argument| argument.field_constant("ToolMaterial"))
        .ok_or_else(|| anyhow::anyhow!("spear declaration lacks ToolMaterial"))?;
    let duration = call
        .arguments
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("spear declaration lacks attack duration"))?;
    Ok(Some((
        material.to_string(),
        parse_java_float_literal(&duration.canonical)?,
    )))
}

fn material_attack_damage_bonus(material: &str) -> Result<f32> {
    match material {
        "WOOD" | "GOLD" => Ok(0.0),
        "STONE" | "COPPER" => Ok(1.0),
        "IRON" => Ok(2.0),
        "DIAMOND" => Ok(3.0),
        "NETHERITE" => Ok(4.0),
        _ => bail!("unsupported tool material ToolMaterial.{material}"),
    }
}

fn default_use_effects_for_declaration(
    expression: &GeneratedJavaExpression,
) -> Result<Option<ItemUseEffects>> {
    if expression.call("spear").is_some() {
        return Ok(Some(ItemUseEffects {
            can_sprint: true,
            interact_vibrations: false,
            speed_multiplier: 1.0,
        }));
    }

    let Some(value) = component_value(expression, "USE_EFFECTS") else {
        return Ok(None);
    };
    let Some(creation) = value
        .creations()
        .find(|creation| creation.name.as_deref() == Some("UseEffects"))
    else {
        bail!("USE_EFFECTS component lacks UseEffects constructor");
    };
    if creation.arguments.len() != 3 {
        bail!("UseEffects constructor must have three arguments");
    }
    Ok(Some(ItemUseEffects {
        can_sprint: parse_java_bool_literal(&creation.arguments[0].canonical)?,
        interact_vibrations: parse_java_bool_literal(&creation.arguments[1].canonical)?,
        speed_multiplier: parse_java_float_literal(&creation.arguments[2].canonical)?,
    }))
}

fn default_consumable_for_declaration(
    expression: &GeneratedJavaExpression,
) -> Result<Option<ItemConsumable>> {
    if let Some(value) = component_value(expression, "CONSUMABLE") {
        if let Some(name) = value.field_constant("Consumables") {
            return consumable_for_name(name).map(Some);
        }
    }
    if let Some(name) = expression
        .call("food")
        .and_then(|call| call.arguments.get(1))
        .and_then(|argument| argument.field_constant("Consumables"))
    {
        return consumable_for_name(name).map(Some);
    }
    if expression.call("food").is_some() {
        return Ok(Some(ItemConsumable {
            consume_seconds: 1.6,
            animation: ItemUseAnimation::Eat,
        }));
    }
    Ok(None)
}

fn consumable_for_name(name: &str) -> Result<ItemConsumable> {
    let consumable = match name {
        "DEFAULT_FOOD"
        | "CHICKEN"
        | "CHORUS_FRUIT"
        | "ENCHANTED_GOLDEN_APPLE"
        | "GOLDEN_APPLE"
        | "POISONOUS_POTATO"
        | "PUFFERFISH"
        | "ROTTEN_FLESH"
        | "SPIDER_EYE" => ItemConsumable {
            consume_seconds: 1.6,
            animation: ItemUseAnimation::Eat,
        },
        "DRIED_KELP" => ItemConsumable {
            consume_seconds: 0.8,
            animation: ItemUseAnimation::Eat,
        },
        "DEFAULT_DRINK" | "MILK_BUCKET" | "OMINOUS_BOTTLE" => ItemConsumable {
            consume_seconds: 1.6,
            animation: ItemUseAnimation::Drink,
        },
        "HONEY_BOTTLE" => ItemConsumable {
            consume_seconds: 2.0,
            animation: ItemUseAnimation::Drink,
        },
        other => bail!("unsupported Consumables.{other} item declaration"),
    };
    Ok(consumable)
}

fn crafting_remainder_for_declaration(
    expression: &GeneratedJavaExpression,
    item_id_constants: &BTreeMap<String, String>,
) -> Result<Option<String>> {
    let Some(item_field) = expression
        .call("craftRemainder")
        .and_then(|call| call.arguments.first())
        .map(|argument| argument.canonical.as_str())
    else {
        return Ok(None);
    };
    let name = item_id_constants
        .get(item_field)
        .cloned()
        .unwrap_or_else(|| item_field.to_ascii_lowercase());
    minecraft_id(&name).map(Some)
}

fn mining_profile_for_declaration(
    expression: &GeneratedJavaExpression,
    block_tags: Option<&TagCatalog>,
    block_resources_by_constant: &BTreeMap<String, String>,
) -> Result<Option<ItemMiningProfile>> {
    let Some(block_tags) = block_tags else {
        return Ok(None);
    };

    if let Some((tag_id, material)) = tool_mining_tag_and_material(expression)? {
        let mut rules = Vec::new();
        push_tag_rule(
            &mut rules,
            block_tags,
            material_incorrect_for_drops_tag(&material)?,
            None,
            Some(false),
        );
        push_tag_rule(
            &mut rules,
            block_tags,
            tag_id,
            Some(material_speed_thousandths(&material)?),
            Some(true),
        );
        return Ok(non_empty_mining_profile(rules));
    }

    if expression.call("sword").is_some() {
        let mut rules = Vec::new();
        push_direct_block_rule(
            &mut rules,
            block_resources_by_constant,
            ["COBWEB"],
            Some(15_000),
            Some(true),
        )?;
        push_tag_rule(
            &mut rules,
            block_tags,
            "minecraft:sword_instantly_mines",
            Some(u32::MAX),
            None,
        );
        push_tag_rule(
            &mut rules,
            block_tags,
            "minecraft:sword_efficient",
            Some(1_500),
            None,
        );
        return Ok(non_empty_mining_profile(rules));
    }

    if qualified_call(expression, "createToolProperties", "ShearsItem").is_some() {
        let mut rules = Vec::new();
        push_direct_block_rule(
            &mut rules,
            block_resources_by_constant,
            ["COBWEB"],
            Some(15_000),
            Some(true),
        )?;
        push_tag_rule(
            &mut rules,
            block_tags,
            "minecraft:leaves",
            Some(15_000),
            None,
        );
        push_tag_rule(&mut rules, block_tags, "minecraft:wool", Some(5_000), None);
        push_direct_block_rule(
            &mut rules,
            block_resources_by_constant,
            ["VINE", "GLOW_LICHEN"],
            Some(2_000),
            None,
        )?;
        return Ok(non_empty_mining_profile(rules));
    }

    Ok(None)
}

fn tool_mining_tag_and_material(
    expression: &GeneratedJavaExpression,
) -> Result<Option<(&'static str, String)>> {
    for (tag_id, call_name, creation_name) in [
        ("minecraft:mineable/pickaxe", "pickaxe", "PickaxeItem"),
        ("minecraft:mineable/axe", "axe", "AxeItem"),
        ("minecraft:mineable/hoe", "hoe", "HoeItem"),
        ("minecraft:mineable/shovel", "shovel", "ShovelItem"),
    ] {
        if let Some(material) = call_field_argument(expression, call_name, 0, "ToolMaterial") {
            return Ok(Some((tag_id, material.to_string())));
        }
        if let Some(material) =
            creation_field_argument(expression, creation_name, 0, "ToolMaterial")
        {
            return Ok(Some((tag_id, material.to_string())));
        }
    }
    Ok(None)
}

fn material_speed_thousandths(material: &str) -> Result<u32> {
    match material {
        "WOOD" => Ok(2_000),
        "STONE" => Ok(4_000),
        "COPPER" => Ok(5_000),
        "IRON" => Ok(6_000),
        "DIAMOND" => Ok(8_000),
        "GOLD" => Ok(12_000),
        "NETHERITE" => Ok(9_000),
        _ => bail!("unsupported tool material ToolMaterial.{material}"),
    }
}

fn material_incorrect_for_drops_tag(material: &str) -> Result<&'static str> {
    match material {
        "WOOD" => Ok("minecraft:incorrect_for_wooden_tool"),
        "STONE" => Ok("minecraft:incorrect_for_stone_tool"),
        "COPPER" => Ok("minecraft:incorrect_for_copper_tool"),
        "IRON" => Ok("minecraft:incorrect_for_iron_tool"),
        "DIAMOND" => Ok("minecraft:incorrect_for_diamond_tool"),
        "GOLD" => Ok("minecraft:incorrect_for_gold_tool"),
        "NETHERITE" => Ok("minecraft:incorrect_for_netherite_tool"),
        _ => bail!("unsupported tool material ToolMaterial.{material}"),
    }
}

fn push_tag_rule(
    rules: &mut Vec<ItemMiningRule>,
    block_tags: &TagCatalog,
    tag_id: &str,
    mining_speed_thousandths: Option<u32>,
    correct_for_drops: Option<bool>,
) {
    let Some(block_names) = block_tags.values(tag_id) else {
        return;
    };
    if block_names.is_empty() {
        return;
    }
    rules.push(ItemMiningRule {
        block_names: block_names.to_vec(),
        mining_speed_thousandths,
        correct_for_drops,
    });
}

fn push_direct_block_rule<const N: usize>(
    rules: &mut Vec<ItemMiningRule>,
    block_resources_by_constant: &BTreeMap<String, String>,
    block_constants: [&str; N],
    mining_speed_thousandths: Option<u32>,
    correct_for_drops: Option<bool>,
) -> Result<()> {
    let block_names = block_constants
        .into_iter()
        .map(|constant| {
            block_resources_by_constant
                .get(constant)
                .cloned()
                .with_context(|| format!("unknown generated Blocks.{constant} mining rule"))
        })
        .collect::<Result<Vec<_>>>()?;
    rules.push(ItemMiningRule {
        block_names,
        mining_speed_thousandths,
        correct_for_drops,
    });
    Ok(())
}

fn non_empty_mining_profile(rules: Vec<ItemMiningRule>) -> Option<ItemMiningProfile> {
    (!rules.is_empty()).then_some(ItemMiningProfile {
        default_mining_speed_thousandths: 1_000,
        rules,
    })
}

fn equipment_slot_from_name(name: &str) -> Result<ItemEquipmentSlot> {
    match name {
        "MAINHAND" => Ok(ItemEquipmentSlot::MainHand),
        "OFFHAND" => Ok(ItemEquipmentSlot::OffHand),
        "FEET" => Ok(ItemEquipmentSlot::Feet),
        "LEGS" => Ok(ItemEquipmentSlot::Legs),
        "CHEST" => Ok(ItemEquipmentSlot::Chest),
        "HEAD" => Ok(ItemEquipmentSlot::Head),
        "BODY" => Ok(ItemEquipmentSlot::Body),
        "SADDLE" => Ok(ItemEquipmentSlot::Saddle),
        _ => bail!("unsupported item equipment slot EquipmentSlot.{name}"),
    }
}

fn armor_type_equipment_slot(name: &str) -> Result<ItemEquipmentSlot> {
    match name {
        "HELMET" => Ok(ItemEquipmentSlot::Head),
        "CHESTPLATE" => Ok(ItemEquipmentSlot::Chest),
        "LEGGINGS" => Ok(ItemEquipmentSlot::Legs),
        "BOOTS" => Ok(ItemEquipmentSlot::Feet),
        "BODY" => Ok(ItemEquipmentSlot::Body),
        _ => bail!("unsupported item armor equipment slot ArmorType.{name}"),
    }
}

fn qualified_call<'a>(
    expression: &'a GeneratedJavaExpression,
    name: &str,
    qualifier: &str,
) -> Option<&'a GeneratedJavaExpression> {
    expression.calls().find(|call| {
        call.name.as_deref() == Some(name)
            && call
                .object
                .as_deref()
                .map(|object| object.canonical.as_str())
                == Some(qualifier)
    })
}

fn call_field_argument<'a>(
    expression: &'a GeneratedJavaExpression,
    call_name: &str,
    argument_index: usize,
    owner: &str,
) -> Option<&'a str> {
    expression
        .calls()
        .find(|call| {
            call.name.as_deref() == Some(call_name)
                && call
                    .arguments
                    .get(argument_index)
                    .and_then(|argument| argument.field_constant(owner))
                    .is_some()
        })
        .and_then(|call| call.arguments[argument_index].field_constant(owner))
}

fn creation_field_argument<'a>(
    expression: &'a GeneratedJavaExpression,
    creation_name: &str,
    argument_index: usize,
    owner: &str,
) -> Option<&'a str> {
    expression
        .creations()
        .find(|creation| {
            creation.name.as_deref() == Some(creation_name)
                && creation
                    .arguments
                    .get(argument_index)
                    .and_then(|argument| argument.field_constant(owner))
                    .is_some()
        })
        .and_then(|creation| creation.arguments[argument_index].field_constant(owner))
}

fn call_integer_argument(
    expression: &GeneratedJavaExpression,
    call_name: &str,
    argument_index: usize,
) -> Result<Option<i32>> {
    expression
        .call(call_name)
        .and_then(|call| call.arguments.get(argument_index))
        .map(|argument| argument.canonical.parse())
        .transpose()
        .map_err(Into::into)
}

fn component_value<'a>(
    expression: &'a GeneratedJavaExpression,
    component_name: &str,
) -> Option<&'a GeneratedJavaExpression> {
    expression
        .calls()
        .find(|call| {
            call.name.as_deref() == Some("component")
                && call
                    .arguments
                    .first()
                    .and_then(|argument| argument.field_constant("DataComponents"))
                    == Some(component_name)
        })
        .and_then(|call| call.arguments.get(1))
}

fn call_has_qualified_call_argument(
    expression: &GeneratedJavaExpression,
    call_name: &str,
    argument_index: usize,
    qualifier: &str,
    argument_call_name: &str,
) -> bool {
    expression.calls().any(|call| {
        call.name.as_deref() == Some(call_name)
            && call.arguments.get(argument_index).is_some_and(|argument| {
                argument.name.as_deref() == Some(argument_call_name)
                    && argument
                        .object
                        .as_deref()
                        .map(|object| object.canonical.as_str())
                        == Some(qualifier)
            })
    })
}

fn parse_java_float_literal(value: &str) -> Result<f32> {
    value
        .chars()
        .filter(|character| !character.is_ascii_whitespace() && *character != '_')
        .collect::<String>()
        .trim_end_matches('F')
        .parse()
        .map_err(Into::into)
}

fn parse_java_bool_literal(value: &str) -> Result<bool> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        other => bail!("invalid Java bool literal {other:?}"),
    }
}

fn minecraft_id(path: &str) -> Result<String> {
    ResourceLocation::new("minecraft", path).map(|location| location.id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_component_constant_names_delegate_to_generated_catalog() {
        assert_eq!(component_id::VANILLA_DATA_COMPONENT_TYPES.len(), 110);
        for entry in component_id::VANILLA_DATA_COMPONENT_TYPES {
            assert_eq!(
                data_component_type_id_from_name(entry.constant_name),
                Some(entry.type_id),
                "{}",
                entry.constant_name
            );
        }
        assert_eq!(data_component_type_id_from_name("UNKNOWN_COMPONENT"), None);
        assert_eq!(data_component_type_id_from_name("pig_sound_variant"), None);
    }

    #[test]
    fn item_registry_does_not_restore_component_id_const_aliases() {
        let source = include_str!("item_registry.rs");
        let aliases = source
            .lines()
            .filter(|line| line.trim_start().starts_with("const ") && line.contains("COMPONENT_ID"))
            .collect::<Vec<_>>();
        assert!(
            aliases.is_empty(),
            "item registry restored component id aliases:\n{}",
            aliases.join("\n")
        );
    }

    #[test]
    fn generated_common_item_components_apply_to_full_vanilla_item_catalog() {
        let catalog = ItemRegistryCatalog::from_generated_source_catalog(None).unwrap();
        for item in bbb_protocol::item_types::VANILLA_ITEM_TYPES {
            let actual = catalog
                .default_component_type_ids(item.resource_id)
                .unwrap_or_else(|| panic!("missing {} default components", item.resource_id));
            assert!(
                component_id::VANILLA_COMMON_ITEM_COMPONENT_TYPE_IDS
                    .iter()
                    .all(|type_id| actual.contains(type_id)),
                "{} omitted a generated common item component",
                item.resource_id
            );
        }
    }

    #[test]
    fn extracts_all_registry_dependent_vanilla_item_initializers() {
        let catalog = ItemRegistryCatalog::from_generated_source_catalog(None).unwrap();
        let initializers = catalog.registry_dependent_component_initializers_by_protocol_id();
        assert_eq!(initializers.len(), 60);
        assert_eq!(initializers.values().map(Vec::len).sum::<usize>(), 61);

        let item = |resource_id| catalog.protocol_id(resource_id).unwrap();
        assert_eq!(
            initializers[&item("minecraft:nether_star")],
            vec![RegistryDependentItemComponentInitializer::DamageResistant {
                tag: "minecraft:is_explosion".to_string(),
            }]
        );
        assert_eq!(
            initializers[&item("minecraft:music_disc_13")],
            vec![RegistryDependentItemComponentInitializer::JukeboxPlayable {
                song: "minecraft:13".to_string(),
            }]
        );
        assert_eq!(
            initializers[&item("minecraft:shield")],
            vec![RegistryDependentItemComponentInitializer::BlocksAttacks]
        );
        assert_eq!(
            initializers[&item("minecraft:field_masoned_banner_pattern")],
            vec![
                RegistryDependentItemComponentInitializer::ProvidesBannerPatterns {
                    tag: "minecraft:pattern_item/field_masoned".to_string(),
                }
            ]
        );
        assert_eq!(
            initializers[&item("minecraft:goat_horn")],
            vec![RegistryDependentItemComponentInitializer::Instrument {
                instrument: "minecraft:ponder_goat_horn".to_string(),
            }]
        );
        assert_eq!(
            initializers[&item("minecraft:diamond")],
            vec![
                RegistryDependentItemComponentInitializer::ProvidesTrimMaterial {
                    material: "minecraft:diamond".to_string(),
                }
            ]
        );
        assert_eq!(
            initializers[&item("minecraft:lapis_lazuli")],
            vec![
                RegistryDependentItemComponentInitializer::ProvidesTrimMaterial {
                    material: "minecraft:lapis".to_string(),
                }
            ]
        );
        assert_eq!(
            initializers[&item("minecraft:netherite_ingot")],
            vec![
                RegistryDependentItemComponentInitializer::DamageResistant {
                    tag: "minecraft:is_fire".to_string(),
                },
                RegistryDependentItemComponentInitializer::ProvidesTrimMaterial {
                    material: "minecraft:netherite".to_string(),
                },
            ]
        );
    }

    #[test]
    fn extracts_complete_component_membership_and_dye_values_by_protocol_id() {
        let catalog = ItemRegistryCatalog::from_generated_source_catalog(None).unwrap();
        let components = catalog.default_component_type_ids_by_protocol_id();
        let dyes = catalog.default_dye_color_ids_by_protocol_id();
        let item = |resource_id| catalog.protocol_id(resource_id).unwrap();

        assert_eq!(components.len(), 1506);
        assert_eq!(dyes.len(), 16);
        assert!(components[&item("minecraft:white_dye")]
            .contains(&component_id::VANILLA_DATA_COMPONENT_DYE_TYPE_ID));
        assert!(components[&item("minecraft:diamond")]
            .contains(&component_id::VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID));
        assert_eq!(
            dyes[&item("minecraft:white_dye")],
            bbb_protocol::builtin_enum_component_values::VANILLA_DYE_COLOR_WHITE_ID
        );
        assert_eq!(
            dyes[&item("minecraft:red_dye")],
            bbb_protocol::builtin_enum_component_values::VANILLA_DYE_COLOR_RED_ID
        );
    }

    #[test]
    fn loads_checked_in_vanilla_item_registry() {
        let block_tags = test_block_tags();
        let catalog =
            ItemRegistryCatalog::from_generated_source_catalog(Some(&block_tags)).unwrap();

        assert_eq!(catalog.len(), 1506);
        assert_eq!(catalog.resource_id(0), Some("minecraft:air"));
        assert_eq!(catalog.resource_id(1), Some("minecraft:stone"));
        assert_eq!(catalog.resource_id(1505), Some("minecraft:ominous_bottle"));
        assert_eq!(catalog.protocol_id("ominous_bottle"), Some(1505));
        assert_eq!(catalog.protocol_id("minecraft:short_dry_grass"), Some(209));
        assert_eq!(catalog.protocol_id("minecraft:dry_short_grass"), None);
        assert_eq!(catalog.max_damage("minecraft:elytra"), Some(432));
        assert_eq!(catalog.max_damage("minecraft:stone"), None);
        assert_eq!(catalog.max_stack_size("minecraft:stone"), Some(64));
        assert_eq!(catalog.max_stack_size("minecraft:ender_pearl"), Some(16));
        assert_eq!(catalog.max_stack_size("minecraft:diamond_sword"), Some(1));
        assert_eq!(catalog.max_stack_size("minecraft:diamond_shovel"), Some(1));
        for (resource_id, expected) in [
            ("minecraft:wooden_spear", (15, 100, 200, 300)),
            ("minecraft:stone_spear", (14, 90, 180, 275)),
            ("minecraft:copper_spear", (13, 80, 165, 250)),
            ("minecraft:iron_spear", (12, 50, 135, 225)),
            ("minecraft:golden_spear", (14, 70, 170, 275)),
            ("minecraft:diamond_spear", (10, 60, 130, 200)),
            ("minecraft:netherite_spear", (8, 50, 110, 175)),
        ] {
            let defaults = catalog
                .default_kinetic_weapon(resource_id)
                .unwrap_or_else(|| panic!("missing {resource_id} kinetic defaults"));
            assert_eq!(
                (
                    defaults.delay_ticks,
                    defaults.dismount_duration_ticks,
                    defaults.knockback_duration_ticks,
                    defaults.damage_duration_ticks,
                ),
                expected,
                "{resource_id}"
            );
            assert_eq!(defaults.forward_movement.to_bits(), 0.38f32.to_bits());
        }
        assert_eq!(
            catalog.default_kinetic_weapon("minecraft:diamond_sword"),
            None
        );
        assert_eq!(
            catalog.default_attack_range("minecraft:wooden_spear"),
            Some(ItemAttackRange {
                min_reach: 2.0,
                max_reach: 4.5,
                min_creative_reach: 2.0,
                max_creative_reach: 6.5,
                hitbox_margin: 0.125,
                mob_factor: 0.5,
            })
        );
        assert_eq!(
            catalog.default_attack_range("minecraft:diamond_sword"),
            None
        );
        assert_eq!(
            catalog.default_use_effects("minecraft:wooden_spear"),
            Some(ItemUseEffects {
                can_sprint: true,
                interact_vibrations: false,
                speed_multiplier: 1.0,
            })
        );
        assert_eq!(catalog.default_use_effects("minecraft:diamond_sword"), None);
        assert!(catalog
            .mining_profile("minecraft:diamond_pickaxe")
            .is_some());
        let rule = |block_names: &[&str], speed, correct_for_drops| ItemMiningRule {
            block_names: block_names.iter().map(|name| (*name).to_string()).collect(),
            mining_speed_thousandths: speed,
            correct_for_drops,
        };
        assert_eq!(
            catalog
                .mining_profile("minecraft:diamond_sword")
                .unwrap()
                .rules,
            vec![
                rule(&["minecraft:cobweb"], Some(15_000), Some(true)),
                rule(&["minecraft:bamboo_sapling"], Some(u32::MAX), None,),
                rule(&["minecraft:oak_leaves"], Some(1_500), None),
            ]
        );
        assert_eq!(
            catalog.mining_profile("minecraft:shears").unwrap().rules,
            vec![
                rule(&["minecraft:cobweb"], Some(15_000), Some(true)),
                rule(&["minecraft:oak_leaves"], Some(15_000), None),
                rule(&["minecraft:white_wool"], Some(5_000), None),
                rule(
                    &["minecraft:vine", "minecraft:glow_lichen"],
                    Some(2_000),
                    None,
                ),
            ]
        );
        let default_piercing_weapon_ids: Vec<_> = catalog
            .default_piercing_weapon_protocol_ids()
            .into_iter()
            .filter_map(|protocol_id| catalog.resource_id(protocol_id))
            .collect();
        assert_eq!(
            default_piercing_weapon_ids,
            vec![
                "minecraft:wooden_spear",
                "minecraft:stone_spear",
                "minecraft:copper_spear",
                "minecraft:iron_spear",
                "minecraft:golden_spear",
                "minecraft:diamond_spear",
                "minecraft:netherite_spear",
            ]
        );
        assert_eq!(
            catalog.equipment_slot("minecraft:diamond_boots"),
            Some(ItemEquipmentSlot::Feet)
        );
        assert_eq!(
            catalog.equipment_slot("minecraft:elytra"),
            Some(ItemEquipmentSlot::Chest)
        );
        assert_eq!(
            catalog.equipment_slot("minecraft:shield"),
            Some(ItemEquipmentSlot::OffHand)
        );
        assert_eq!(catalog.equipment_slot("minecraft:stone"), None);

        let component_ids = |resource_id| catalog.default_component_type_ids(resource_id).unwrap();
        assert_eq!(component_ids("minecraft:stick").len(), 12);
        assert!(component_ids("minecraft:elytra")
            .contains(&component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID));
        assert!(component_ids("minecraft:apple")
            .contains(&component_id::VANILLA_DATA_COMPONENT_FOOD_TYPE_ID));
        assert!(component_ids("minecraft:iron_sword")
            .contains(&component_id::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID));
        assert!(component_ids("minecraft:wooden_spear")
            .contains(&component_id::VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID));
        assert!(component_ids("minecraft:creeper_spawn_egg")
            .contains(&component_id::VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID));

        assert_eq!(
            catalog.default_consumable("minecraft:apple"),
            Some(ItemConsumable {
                consume_seconds: 1.6,
                animation: ItemUseAnimation::Eat,
            })
        );
        assert_eq!(
            catalog.default_consumable("minecraft:dried_kelp"),
            Some(ItemConsumable {
                consume_seconds: 0.8,
                animation: ItemUseAnimation::Eat,
            })
        );
        assert_eq!(
            catalog.default_consumable("minecraft:milk_bucket"),
            Some(ItemConsumable {
                consume_seconds: 1.6,
                animation: ItemUseAnimation::Drink,
            })
        );
        assert_eq!(
            catalog.crafting_remainder("minecraft:water_bucket"),
            Some("minecraft:bucket")
        );
        assert_eq!(
            catalog.crafting_remainder("minecraft:honey_bottle"),
            Some("minecraft:glass_bottle")
        );
        assert_eq!(
            catalog.humanoid_armor_asset("minecraft:diamond_helmet"),
            Some("diamond")
        );
        assert_eq!(catalog.equippable_asset("minecraft:elytra"), Some("elytra"));
        assert_eq!(
            catalog.mount_body_armor_kind("minecraft:diamond_horse_armor"),
            Some(ItemMountBodyArmorKind::Horse)
        );
        assert_eq!(
            catalog.mount_body_armor_asset("minecraft:diamond_horse_armor"),
            Some("diamond")
        );
        assert_eq!(
            catalog.mount_body_armor_kind("minecraft:iron_nautilus_armor"),
            Some(ItemMountBodyArmorKind::Nautilus)
        );

        let modifier_amounts = |resource_id| {
            catalog
                .default_attribute_modifiers(resource_id)
                .unwrap()
                .iter()
                .map(|modifier| f64::from_bits(modifier.amount_bits))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            modifier_amounts("minecraft:iron_sword"),
            vec![5.0, f64::from(-2.4_f32)]
        );
        assert_eq!(modifier_amounts("minecraft:iron_shovel"), vec![3.5, -3.0]);
        assert_eq!(
            modifier_amounts("minecraft:mace"),
            vec![5.0, f64::from(-3.4_f32)]
        );
        assert_eq!(
            modifier_amounts("minecraft:trident"),
            vec![8.0, f64::from(-2.9_f32)]
        );
    }

    #[test]
    fn checked_in_item_catalog_matches_generated_protocol_catalog() {
        let source: ItemSourceCatalog = serde_json::from_str(GENERATED_ITEM_DECLARATIONS).unwrap();
        assert_eq!(source.declarations.len(), 1485);
        assert_eq!(
            source
                .declarations
                .iter()
                .filter(|declaration| declaration.field_type == "WeatheringCopperItems")
                .count(),
            3
        );
        let catalog = ItemRegistryCatalog::from_generated_source_catalog(None).unwrap();
        assert_eq!(
            catalog.len(),
            bbb_protocol::item_types::VANILLA_ITEM_TYPES.len()
        );
        for (id, (resource_id, generated)) in catalog
            .resource_ids()
            .iter()
            .zip(bbb_protocol::item_types::VANILLA_ITEM_TYPES)
            .enumerate()
        {
            assert_eq!(generated.id, id as i32);
            assert_eq!(generated.resource_id, resource_id);
        }
        assert_eq!(
            catalog.primary_block_resource_ids.len(),
            catalog.block_items.len(),
            "every generated BlockItem must have one primary block"
        );
        for (block, item) in &catalog.block_item_resource_ids {
            assert!(
                catalog.protocol_id(item).is_some(),
                "unknown mapped item {item}"
            );
            assert!(
                catalog.block_items.contains(item),
                "mapped item {item} for {block} is not classified as a BlockItem"
            );
        }
        assert_eq!(
            catalog.item_resource_id_for_block("minecraft:stone"),
            Some("minecraft:stone")
        );
        assert_eq!(
            catalog.item_resource_id_for_block("minecraft:redstone_wire"),
            Some("minecraft:redstone")
        );
        assert_eq!(
            catalog.item_resource_id_for_block("minecraft:redstone_wall_torch"),
            Some("minecraft:redstone_torch")
        );
        assert_eq!(
            catalog.primary_block_resource_id_for_item("minecraft:redstone_torch"),
            Some("minecraft:redstone_torch")
        );
        assert_eq!(
            catalog.primary_block_resource_id_for_item("minecraft:redstone"),
            Some("minecraft:redstone_wire")
        );
        assert_eq!(
            catalog.item_resource_id_for_block("minecraft:weathered_copper_bars"),
            Some("minecraft:weathered_copper_bars")
        );
        assert_eq!(catalog.item_resource_id_for_block("minecraft:water"), None);
    }

    #[test]
    fn direct_mining_blocks_delegate_to_generated_block_catalog() {
        let production = include_str!("item_registry.rs")
            .split("#[cfg(test)]\nmod tests")
            .next()
            .unwrap();
        for resource_id in [
            "minecraft:cobweb",
            "minecraft:vine",
            "minecraft:glow_lichen",
        ] {
            assert!(
                !production.contains(&format!("\"{resource_id}\"")),
                "item registry restored direct mining block literal {resource_id}"
            );
        }
    }

    #[test]
    fn ordinary_catalog_load_does_not_require_local_vanilla_sources() {
        let missing = std::path::PathBuf::from("/bbb-test/missing-vanilla-source-root");
        let roots = PackRoots::from_paths(
            missing.clone(),
            missing.join("sources/26.1"),
            missing.join("sources/26.1/assets/minecraft"),
            None,
        );

        let catalog = ItemRegistryCatalog::load(&roots).unwrap();

        assert_eq!(catalog.len(), 1506);
        assert_eq!(catalog.resource_id(0), Some("minecraft:air"));
    }

    fn test_block_tags() -> TagCatalog {
        use crate::tags::TagDefinition;

        TagCatalog {
            registry_path: "block".to_string(),
            tags: BTreeMap::from([
                (
                    "minecraft:incorrect_for_stone_tool".to_string(),
                    TagDefinition {
                        id: "minecraft:incorrect_for_stone_tool".to_string(),
                        values: vec!["minecraft:obsidian".to_string()],
                    },
                ),
                (
                    "minecraft:incorrect_for_iron_tool".to_string(),
                    TagDefinition {
                        id: "minecraft:incorrect_for_iron_tool".to_string(),
                        values: vec!["minecraft:ancient_debris".to_string()],
                    },
                ),
                (
                    "minecraft:mineable/pickaxe".to_string(),
                    TagDefinition {
                        id: "minecraft:mineable/pickaxe".to_string(),
                        values: vec![
                            "minecraft:stone".to_string(),
                            "minecraft:obsidian".to_string(),
                        ],
                    },
                ),
                (
                    "minecraft:mineable/shovel".to_string(),
                    TagDefinition {
                        id: "minecraft:mineable/shovel".to_string(),
                        values: vec!["minecraft:dirt".to_string()],
                    },
                ),
                (
                    "minecraft:sword_instantly_mines".to_string(),
                    TagDefinition {
                        id: "minecraft:sword_instantly_mines".to_string(),
                        values: vec!["minecraft:bamboo_sapling".to_string()],
                    },
                ),
                (
                    "minecraft:sword_efficient".to_string(),
                    TagDefinition {
                        id: "minecraft:sword_efficient".to_string(),
                        values: vec!["minecraft:oak_leaves".to_string()],
                    },
                ),
                (
                    "minecraft:leaves".to_string(),
                    TagDefinition {
                        id: "minecraft:leaves".to_string(),
                        values: vec!["minecraft:oak_leaves".to_string()],
                    },
                ),
                (
                    "minecraft:wool".to_string(),
                    TagDefinition {
                        id: "minecraft:wool".to_string(),
                        values: vec!["minecraft:white_wool".to_string()],
                    },
                ),
            ]),
        }
    }
}
