use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{resources::ResourceLocation, tags::TagCatalog, PackRoots};

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
    default_equipment_slots: BTreeMap<String, ItemEquipmentSlot>,
    /// Resource id → humanoid armor equipment-asset name (`ArmorMaterials.<MAT>` →
    /// `EquipmentAssets.<MAT>`, the lowercased material, e.g. `iron` / `chainmail` / `turtle_scute`),
    /// for the `HumanoidArmorLayer` texture path. Only `.humanoidArmor(...)` items appear here.
    #[serde(default)]
    humanoid_armor_assets: BTreeMap<String, String>,
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
    default_use_effects: BTreeMap<String, ItemUseEffects>,
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

impl ItemRegistryCatalog {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        let items_java = roots
            .sources_dir
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item")
            .join("Items.java");
        let item_ids_java = roots
            .sources_dir
            .join("net")
            .join("minecraft")
            .join("references")
            .join("ItemIds.java");
        let block_tags = roots.load_tag_catalog("block").ok();
        Self::load_from_java_sources_with_block_tags(items_java, item_ids_java, block_tags.as_ref())
    }

    pub fn load_from_java_sources(
        items_java: impl AsRef<Path>,
        item_ids_java: impl AsRef<Path>,
    ) -> Result<Self> {
        Self::load_from_java_sources_with_block_tags(items_java, item_ids_java, None)
    }

    pub fn load_from_java_sources_with_block_tags(
        items_java: impl AsRef<Path>,
        item_ids_java: impl AsRef<Path>,
        block_tags: Option<&TagCatalog>,
    ) -> Result<Self> {
        let items_java = items_java.as_ref();
        let item_ids_java = item_ids_java.as_ref();
        let items_source = std::fs::read_to_string(items_java)
            .with_context(|| format!("read item registry source {}", items_java.display()))?;
        let item_id_constants = if item_ids_java.is_file() {
            let source = std::fs::read_to_string(item_ids_java)
                .with_context(|| format!("read item id source {}", item_ids_java.display()))?;
            parse_item_id_constants(&source)?
        } else {
            BTreeMap::new()
        };
        Self::from_items_java_source_with_block_tags(&items_source, &item_id_constants, block_tags)
    }

    #[cfg(test)]
    pub(crate) fn from_items_java_source(
        source: &str,
        item_id_constants: &BTreeMap<String, String>,
    ) -> Result<Self> {
        Self::from_items_java_source_with_block_tags(source, item_id_constants, None)
    }

    fn from_items_java_source_with_block_tags(
        source: &str,
        item_id_constants: &BTreeMap<String, String>,
        block_tags: Option<&TagCatalog>,
    ) -> Result<Self> {
        let declaration = Regex::new(
            r#"(?s)public\s+static\s+final\s+(Item|WeatheringCopperItems)\s+([A-Z0-9_]+)\s*=\s*(.*?);"#,
        )?;
        let mut resource_ids = Vec::new();
        let mut max_damage = BTreeMap::new();
        let mut max_stack_size = BTreeMap::new();
        let mut default_equipment_slots = BTreeMap::new();
        let mut humanoid_armor_assets = BTreeMap::new();
        let mut default_mount_body_armor_kinds = BTreeMap::new();
        let mut mount_body_armor_assets = BTreeMap::new();
        let mut default_piercing_weapon_ids = BTreeSet::new();
        let mut default_attack_ranges = BTreeMap::new();
        let mut default_use_effects = BTreeMap::new();
        let mut crafting_remainders = BTreeMap::new();
        let mut mining_profiles = BTreeMap::new();
        for capture in declaration.captures_iter(source) {
            let kind = capture.get(1).unwrap().as_str();
            let field = capture.get(2).unwrap().as_str();
            let expression = capture.get(3).unwrap().as_str();
            let ids = resource_ids_for_declaration(kind, field, expression, item_id_constants)?;
            let stack_size = max_stack_size_for_declaration(expression)?;
            let equipment_slot = equipment_slot_for_declaration(expression)?;
            let humanoid_armor_asset = humanoid_armor_asset_for_declaration(expression)?;
            let mount_body_armor_kind = mount_body_armor_kind_for_declaration(expression);
            let mount_body_armor_asset = mount_body_armor_asset_for_declaration(expression)?;
            let default_piercing_weapon = default_piercing_weapon_for_declaration(expression);
            let default_attack_range = default_attack_range_for_declaration(expression)?;
            let default_use_effect = default_use_effects_for_declaration(expression)?;
            let crafting_remainder =
                crafting_remainder_for_declaration(expression, item_id_constants)?;
            let mining_profile = mining_profile_for_declaration(expression, block_tags)?;
            if let Some(durability) = durability_for_declaration(expression)? {
                for resource_id in &ids {
                    max_damage.insert(resource_id.clone(), durability);
                }
            }
            for resource_id in &ids {
                max_stack_size.insert(resource_id.clone(), stack_size);
                if let Some(equipment_slot) = equipment_slot {
                    default_equipment_slots.insert(resource_id.clone(), equipment_slot);
                }
                if let Some(asset) = &humanoid_armor_asset {
                    humanoid_armor_assets.insert(resource_id.clone(), asset.clone());
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
                if let Some(default_use_effect) = default_use_effect {
                    default_use_effects.insert(resource_id.clone(), default_use_effect);
                }
                if let Some(crafting_remainder) = &crafting_remainder {
                    crafting_remainders.insert(resource_id.clone(), crafting_remainder.clone());
                }
                if let Some(profile) = &mining_profile {
                    mining_profiles.insert(resource_id.clone(), profile.clone());
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
            default_equipment_slots,
            humanoid_armor_assets,
            default_mount_body_armor_kinds,
            mount_body_armor_assets,
            default_piercing_weapon_ids,
            default_attack_ranges,
            default_use_effects,
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

    pub fn default_use_effects(&self, resource_id: &str) -> Option<ItemUseEffects> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.default_use_effects.get(&resource_id).copied()
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
    expression: &str,
    item_id_constants: &BTreeMap<String, String>,
) -> Result<Vec<String>> {
    if kind == "WeatheringCopperItems" {
        let base = required_capture(
            r#"WeatheringCopperItems\.create\(\s*Blocks\.([A-Z0-9_]+)"#,
            expression,
            field,
        )?;
        return weathering_copper_ids(&base.to_ascii_lowercase());
    }

    if expression.trim_start().starts_with("registerBlock") {
        return Ok(vec![minecraft_id(
            &required_capture(
                r#"registerBlock\(\s*Blocks\.([A-Z0-9_]+)"#,
                expression,
                field,
            )?
            .to_ascii_lowercase(),
        )?]);
    }

    if expression.trim_start().starts_with("registerSpawnEgg") {
        let entity_type = required_capture(
            r#"registerSpawnEgg\(\s*EntityType\.([A-Z0-9_]+)"#,
            expression,
            field,
        )?;
        return Ok(vec![minecraft_id(&format!(
            "{}_spawn_egg",
            entity_type.to_ascii_lowercase()
        ))?]);
    }

    if expression.trim_start().starts_with("registerItem") {
        if let Some(name) = optional_capture(r#"registerItem\(\s*"([^"]+)""#, expression)? {
            return Ok(vec![minecraft_id(&name)?]);
        }

        let constant = required_capture(
            r#"registerItem\(\s*ItemIds\.([A-Z0-9_]+)"#,
            expression,
            field,
        )?;
        let name = item_id_constants
            .get(&constant)
            .cloned()
            .unwrap_or_else(|| constant.to_ascii_lowercase());
        return Ok(vec![minecraft_id(&name)?]);
    }

    bail!("unsupported item registry declaration {field}: {expression:?}")
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

fn parse_item_id_constants(source: &str) -> Result<BTreeMap<String, String>> {
    let declaration = Regex::new(
        r#"public\s+static\s+final\s+ResourceKey<Item>\s+([A-Z0-9_]+)\s*=\s*createKey\("([^"]+)"\)"#,
    )?;
    let mut constants = BTreeMap::new();
    for capture in declaration.captures_iter(source) {
        constants.insert(
            capture.get(1).unwrap().as_str().to_string(),
            capture.get(2).unwrap().as_str().to_string(),
        );
    }
    Ok(constants)
}

fn durability_for_declaration(expression: &str) -> Result<Option<i32>> {
    if let Some(durability) = optional_capture(r#"\.durability\(\s*([0-9]+)\s*\)"#, expression)?
        .map(|value| value.parse::<i32>())
        .transpose()?
    {
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

    if let Some(material) =
        optional_capture(r#"\.wolfArmor\(\s*ArmorMaterials\.([A-Z_]+)"#, expression)?
    {
        return Ok(Some(
            armor_material_durability_multiplier(&material)? * armor_type_unit_durability("BODY")?,
        ));
    }

    Ok(None)
}

fn tool_material_for_durability(expression: &str) -> Result<Option<String>> {
    for pattern in [
        r#"\.(?:sword|pickaxe|axe|hoe|shovel|spear)\(\s*ToolMaterial\.([A-Z_]+)"#,
        r#"new\s+(?:PickaxeItem|AxeItem|HoeItem|ShovelItem)\(\s*ToolMaterial\.([A-Z_]+)"#,
    ] {
        if let Some(material) = optional_capture(pattern, expression)? {
            return Ok(Some(material));
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
fn humanoid_armor_asset_for_declaration(expression: &str) -> Result<Option<String>> {
    Ok(humanoid_armor_material_and_type(expression)?
        .map(|(material, _armor_type)| material.to_lowercase()))
}

fn humanoid_armor_material_and_type(expression: &str) -> Result<Option<(String, String)>> {
    let regex =
        Regex::new(r#"\.humanoidArmor\(\s*ArmorMaterials\.([A-Z_]+)\s*,\s*ArmorType\.([A-Z_]+)"#)?;
    Ok(regex.captures(expression).map(|capture| {
        (
            capture.get(1).unwrap().as_str().to_string(),
            capture.get(2).unwrap().as_str().to_string(),
        )
    }))
}

fn mount_body_armor_asset_for_declaration(expression: &str) -> Result<Option<String>> {
    for pattern in [
        r#"\.horseArmor\(\s*ArmorMaterials\.([A-Z_]+)"#,
        r#"\.nautilusArmor\(\s*ArmorMaterials\.([A-Z_]+)"#,
    ] {
        if let Some(material) = optional_capture(pattern, expression)? {
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

fn max_stack_size_for_declaration(expression: &str) -> Result<i32> {
    let stack_size = optional_capture(r#"\.stacksTo\(\s*([0-9]+)\s*\)"#, expression)?
        .map(|value| value.parse())
        .transpose()?;
    if let Some(stack_size) = stack_size {
        return Ok(stack_size);
    }

    if expression.contains(".durability(")
        || expression.contains(".sword(")
        || expression.contains(".pickaxe(")
        || expression.contains(".axe(")
        || expression.contains(".hoe(")
        || expression.contains(".shovel(")
        || expression.contains("AxeItem(")
        || expression.contains("HoeItem(")
        || expression.contains("ShovelItem(")
        || expression.contains(".spear(")
        || expression.contains(".humanoidArmor(")
        || expression.contains(".wolfArmor(")
        || expression.contains(".horseArmor(")
        || expression.contains(".nautilusArmor(")
    {
        return Ok(1);
    }

    Ok(64)
}

fn equipment_slot_for_declaration(expression: &str) -> Result<Option<ItemEquipmentSlot>> {
    if expression.contains("Equippable.saddle()") {
        return Ok(Some(ItemEquipmentSlot::Saddle));
    }
    if expression.contains(".horseArmor(")
        || expression.contains(".nautilusArmor(")
        || expression.contains(".wolfArmor(")
        || expression.contains("Equippable.llamaSwag(")
    {
        return Ok(Some(ItemEquipmentSlot::Body));
    }

    for pattern in [
        r#"Equippable\.builder\(\s*EquipmentSlot\.([A-Z_]+)\s*\)"#,
        r#"\.equippableUnswappable\(\s*EquipmentSlot\.([A-Z_]+)\s*\)"#,
        r#"\.equippable\(\s*EquipmentSlot\.([A-Z_]+)\s*\)"#,
    ] {
        if let Some(slot) = optional_capture(pattern, expression)? {
            return equipment_slot_from_name(&slot).map(Some);
        }
    }

    if let Some(armor_type) = optional_capture(
        r#"(?s)\.humanoidArmor\(\s*.*?,\s*ArmorType\.([A-Z_]+)\s*\)"#,
        expression,
    )? {
        return armor_type_equipment_slot(&armor_type).map(Some);
    }

    Ok(None)
}

fn mount_body_armor_kind_for_declaration(expression: &str) -> Option<ItemMountBodyArmorKind> {
    if expression.contains(".horseArmor(") {
        Some(ItemMountBodyArmorKind::Horse)
    } else if expression.contains("Equippable.llamaSwag(") {
        Some(ItemMountBodyArmorKind::Llama)
    } else if expression.contains(".nautilusArmor(") {
        Some(ItemMountBodyArmorKind::Nautilus)
    } else {
        None
    }
}

fn default_piercing_weapon_for_declaration(expression: &str) -> bool {
    expression.contains(".spear(")
}

fn default_attack_range_for_declaration(expression: &str) -> Result<Option<ItemAttackRange>> {
    if expression.contains(".spear(") {
        return Ok(Some(ItemAttackRange {
            min_reach: 2.0,
            max_reach: 4.5,
            min_creative_reach: 2.0,
            max_creative_reach: 6.5,
            hitbox_margin: 0.125,
            mob_factor: 0.5,
        }));
    }

    let float = r#"-?[0-9]+(?:\.[0-9]+)?F?"#;
    let pattern = format!(
        r#"(?s)\.component\(\s*DataComponents\.ATTACK_RANGE\s*,\s*new\s+AttackRange\(\s*({float})\s*,\s*({float})\s*,\s*({float})\s*,\s*({float})\s*,\s*({float})\s*,\s*({float})\s*\)\s*\)"#
    );
    let regex = Regex::new(&pattern)?;
    let Some(capture) = regex.captures(expression) else {
        return Ok(None);
    };
    Ok(Some(ItemAttackRange {
        min_reach: parse_java_float_literal(capture.get(1).unwrap().as_str())?,
        max_reach: parse_java_float_literal(capture.get(2).unwrap().as_str())?,
        min_creative_reach: parse_java_float_literal(capture.get(3).unwrap().as_str())?,
        max_creative_reach: parse_java_float_literal(capture.get(4).unwrap().as_str())?,
        hitbox_margin: parse_java_float_literal(capture.get(5).unwrap().as_str())?,
        mob_factor: parse_java_float_literal(capture.get(6).unwrap().as_str())?,
    }))
}

fn default_use_effects_for_declaration(expression: &str) -> Result<Option<ItemUseEffects>> {
    if expression.contains(".spear(") {
        return Ok(Some(ItemUseEffects {
            can_sprint: true,
            interact_vibrations: false,
            speed_multiplier: 1.0,
        }));
    }

    let bool = r#"(true|false)"#;
    let float = r#"-?[0-9]+(?:\.[0-9]+)?F?"#;
    let pattern = format!(
        r#"(?s)\.component\(\s*DataComponents\.USE_EFFECTS\s*,\s*new\s+UseEffects\(\s*{bool}\s*,\s*{bool}\s*,\s*({float})\s*\)\s*\)"#
    );
    let regex = Regex::new(&pattern)?;
    let Some(capture) = regex.captures(expression) else {
        return Ok(None);
    };
    Ok(Some(ItemUseEffects {
        can_sprint: parse_java_bool_literal(capture.get(1).unwrap().as_str())?,
        interact_vibrations: parse_java_bool_literal(capture.get(2).unwrap().as_str())?,
        speed_multiplier: parse_java_float_literal(capture.get(3).unwrap().as_str())?,
    }))
}

fn crafting_remainder_for_declaration(
    expression: &str,
    item_id_constants: &BTreeMap<String, String>,
) -> Result<Option<String>> {
    let Some(item_field) =
        optional_capture(r#"\.craftRemainder\(\s*([A-Z0-9_]+)\s*\)"#, expression)?
    else {
        return Ok(None);
    };
    let name = item_id_constants
        .get(&item_field)
        .cloned()
        .unwrap_or_else(|| item_field.to_ascii_lowercase());
    minecraft_id(&name).map(Some)
}

fn mining_profile_for_declaration(
    expression: &str,
    block_tags: Option<&TagCatalog>,
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

    if expression.contains(".sword(") {
        let mut rules = Vec::new();
        push_direct_rule(&mut rules, ["minecraft:cobweb"], Some(15_000), Some(true));
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

    if expression.contains("ShearsItem.createToolProperties()") {
        let mut rules = Vec::new();
        push_direct_rule(&mut rules, ["minecraft:cobweb"], Some(15_000), Some(true));
        push_tag_rule(
            &mut rules,
            block_tags,
            "minecraft:leaves",
            Some(15_000),
            None,
        );
        push_tag_rule(&mut rules, block_tags, "minecraft:wool", Some(5_000), None);
        push_direct_rule(
            &mut rules,
            ["minecraft:vine", "minecraft:glow_lichen"],
            Some(2_000),
            None,
        );
        return Ok(non_empty_mining_profile(rules));
    }

    Ok(None)
}

fn tool_mining_tag_and_material(expression: &str) -> Result<Option<(&'static str, String)>> {
    for (tag_id, patterns) in [
        (
            "minecraft:mineable/pickaxe",
            [
                r#"\.pickaxe\(\s*ToolMaterial\.([A-Z_]+)"#,
                r#"new\s+PickaxeItem\(\s*ToolMaterial\.([A-Z_]+)"#,
            ],
        ),
        (
            "minecraft:mineable/axe",
            [
                r#"\.axe\(\s*ToolMaterial\.([A-Z_]+)"#,
                r#"new\s+AxeItem\(\s*ToolMaterial\.([A-Z_]+)"#,
            ],
        ),
        (
            "minecraft:mineable/hoe",
            [
                r#"\.hoe\(\s*ToolMaterial\.([A-Z_]+)"#,
                r#"new\s+HoeItem\(\s*ToolMaterial\.([A-Z_]+)"#,
            ],
        ),
        (
            "minecraft:mineable/shovel",
            [
                r#"\.shovel\(\s*ToolMaterial\.([A-Z_]+)"#,
                r#"new\s+ShovelItem\(\s*ToolMaterial\.([A-Z_]+)"#,
            ],
        ),
    ] {
        for pattern in patterns {
            if let Some(material) = optional_capture(pattern, expression)? {
                return Ok(Some((tag_id, material)));
            }
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

fn push_direct_rule<const N: usize>(
    rules: &mut Vec<ItemMiningRule>,
    block_names: [&str; N],
    mining_speed_thousandths: Option<u32>,
    correct_for_drops: Option<bool>,
) {
    rules.push(ItemMiningRule {
        block_names: block_names.into_iter().map(str::to_string).collect(),
        mining_speed_thousandths,
        correct_for_drops,
    });
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

fn required_capture(pattern: &str, expression: &str, field: &str) -> Result<String> {
    optional_capture(pattern, expression)?
        .ok_or_else(|| anyhow::anyhow!("unsupported item registry declaration {field}"))
}

fn optional_capture(pattern: &str, expression: &str) -> Result<Option<String>> {
    let regex = Regex::new(pattern)?;
    Ok(regex
        .captures(expression)
        .and_then(|capture| capture.get(1))
        .map(|capture| capture.as_str().to_string()))
}

fn parse_java_float_literal(value: &str) -> Result<f32> {
    value.trim_end_matches('F').parse().map_err(Into::into)
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
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn item_registry_catalog_parses_items_java_registration_order() {
        let mut constants = BTreeMap::new();
        constants.insert("PUMPKIN_SEEDS".to_string(), "pumpkin_seeds".to_string());
        let source = r#"
            public class Items {
               public static final Item AIR = registerBlock(Blocks.AIR, AirItem::new);
               public static final Item DRY_SHORT_GRASS = registerBlock(Blocks.SHORT_DRY_GRASS);
               public static final Item TRIAL_KEY = registerItem("trial_key");
               public static final Item OMINOUS_BOTTLE = registerItem(
                  "ominous_bottle",
                  new Item.Properties().rarity(Rarity.UNCOMMON)
               );
               public static final Item PUMPKIN_SEEDS = registerItem(ItemIds.PUMPKIN_SEEDS, createBlockItemWithCustomItemName(Blocks.PUMPKIN_STEM));
               public static final Item CREEPER_SPAWN_EGG = registerSpawnEgg(EntityType.CREEPER);
               public static final WeatheringCopperItems COPPER_BARS = WeatheringCopperItems.create(Blocks.COPPER_BARS, Items::registerBlock);
               public static final Item ELYTRA = registerItem("elytra", Item::new, new Item.Properties().durability(432));
               public static final Item ENDER_PEARL = registerItem("ender_pearl", EnderpearlItem::new, new Item.Properties().stacksTo(16));
               public static final Item IRON_SWORD = registerItem("iron_sword", new Item.Properties().sword(ToolMaterial.IRON, 3.0F, -2.4F));
            }
        "#;

        let catalog = ItemRegistryCatalog::from_items_java_source(source, &constants).unwrap();

        assert_eq!(catalog.len(), 17);
        assert_eq!(catalog.resource_id(0), Some("minecraft:air"));
        assert_eq!(catalog.resource_id(1), Some("minecraft:short_dry_grass"));
        assert_eq!(catalog.resource_id(2), Some("minecraft:trial_key"));
        assert_eq!(catalog.resource_id(3), Some("minecraft:ominous_bottle"));
        assert_eq!(catalog.resource_id(4), Some("minecraft:pumpkin_seeds"));
        assert_eq!(catalog.resource_id(5), Some("minecraft:creeper_spawn_egg"));
        assert_eq!(catalog.resource_id(6), Some("minecraft:copper_bars"));
        assert_eq!(
            catalog.resource_id(13),
            Some("minecraft:waxed_oxidized_copper_bars")
        );
        assert_eq!(catalog.resource_id(14), Some("minecraft:elytra"));
        assert_eq!(catalog.resource_id(15), Some("minecraft:ender_pearl"));
        assert_eq!(catalog.resource_id(16), Some("minecraft:iron_sword"));
        assert_eq!(catalog.protocol_id("trial_key"), Some(2));
        assert_eq!(catalog.protocol_id("minecraft:ominous_bottle"), Some(3));
        assert_eq!(catalog.protocol_id("minecraft:pumpkin_seeds"), Some(4));
        assert_eq!(catalog.protocol_id("minecraft:missing_item"), None);
        assert_eq!(catalog.max_damage("minecraft:elytra"), Some(432));
        assert_eq!(catalog.max_damage("minecraft:iron_sword"), Some(250));
        assert_eq!(catalog.max_damage("minecraft:trial_key"), None);
        assert_eq!(catalog.max_damage_protocol_ids(), BTreeSet::from([14, 16]));
        assert_eq!(catalog.max_stack_size("minecraft:trial_key"), Some(64));
        assert_eq!(catalog.max_stack_size("minecraft:elytra"), Some(1));
        assert_eq!(catalog.max_stack_size("minecraft:ender_pearl"), Some(16));
        assert_eq!(catalog.max_stack_size("minecraft:iron_sword"), Some(1));
        assert_eq!(catalog.resource_id(-1), None);
    }

    #[test]
    fn item_registry_catalog_parses_default_piercing_weapon_ids() {
        let source = r#"
            public class Items {
               public static final Item AIR = registerBlock(Blocks.AIR, AirItem::new);
               public static final Item WOODEN_SPEAR = registerItem(
                  "wooden_spear",
                  new Item.Properties().spear(ToolMaterial.WOOD, 0.65F, 0.7F, 0.75F, 5.0F, 14.0F, 10.0F, 5.1F, 15.0F, 4.6F)
               );
               public static final Item STONE_SPEAR = registerItem("stone_spear", new Item.Properties().spear(ToolMaterial.STONE, 0.75F, 0.82F, 0.7F, 4.5F, 13.0F, 9.0F, 5.1F, 13.75F, 4.6F));
               public static final Item IRON_SWORD = registerItem("iron_sword", new Item.Properties().sword(ToolMaterial.IRON, 3.0F, -2.4F));
            }
        "#;

        let catalog =
            ItemRegistryCatalog::from_items_java_source(source, &BTreeMap::new()).unwrap();

        assert!(catalog.default_piercing_weapon("wooden_spear"));
        assert!(catalog.default_piercing_weapon("minecraft:stone_spear"));
        assert!(!catalog.default_piercing_weapon("minecraft:iron_sword"));
        assert!(!catalog.default_piercing_weapon("minecraft:air"));
        assert_eq!(
            catalog.default_piercing_weapon_protocol_ids(),
            BTreeSet::from([1, 2])
        );

        let decoded: ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
            "resource_ids": ["minecraft:wooden_spear"],
            "protocol_ids": {"minecraft:wooden_spear": 0}
        }))
        .unwrap();
        assert!(decoded.default_piercing_weapon_protocol_ids().is_empty());
    }

    #[test]
    fn item_registry_catalog_parses_default_attack_range() {
        let source = r#"
            public class Items {
               public static final Item WOODEN_SPEAR = registerItem(
                  "wooden_spear",
                  new Item.Properties()
                     .spear(ToolMaterial.WOOD, 0.65F, 0.7F, 0.75F, 5.0F, 14.0F, 10.0F, 5.1F, 15.0F, 4.6F)
                     .component(DataComponents.ATTACK_RANGE, new AttackRange(2.0F, 4.5F, 2.0F, 6.5F, 0.125F, 0.5F))
               );
               public static final Item IRON_SWORD = registerItem("iron_sword", new Item.Properties().sword(ToolMaterial.IRON, 3.0F, -2.4F));
            }
        "#;

        let catalog =
            ItemRegistryCatalog::from_items_java_source(source, &BTreeMap::new()).unwrap();

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
        assert_eq!(catalog.default_attack_range("minecraft:iron_sword"), None);
    }

    #[test]
    fn item_registry_catalog_parses_default_use_effects() {
        let source = r#"
            public class Items {
               public static final Item WOODEN_SPEAR = registerItem(
                  "wooden_spear",
                  new Item.Properties().spear(ToolMaterial.WOOD, 0.65F, 0.7F, 0.75F, 5.0F, 14.0F, 10.0F, 5.1F, 15.0F, 4.6F)
               );
               public static final Item TEST_DRINK = registerItem(
                  "test_drink",
                  new Item.Properties()
                     .component(DataComponents.USE_EFFECTS, new UseEffects(false, true, 0.5F))
               );
               public static final Item IRON_SWORD = registerItem("iron_sword", new Item.Properties().sword(ToolMaterial.IRON, 3.0F, -2.4F));
            }
        "#;

        let catalog =
            ItemRegistryCatalog::from_items_java_source(source, &BTreeMap::new()).unwrap();

        assert_eq!(
            catalog.default_use_effects("minecraft:wooden_spear"),
            Some(ItemUseEffects {
                can_sprint: true,
                interact_vibrations: false,
                speed_multiplier: 1.0,
            })
        );
        assert_eq!(
            catalog.default_use_effects("test_drink"),
            Some(ItemUseEffects {
                can_sprint: false,
                interact_vibrations: true,
                speed_multiplier: 0.5,
            })
        );
        assert_eq!(catalog.default_use_effects("minecraft:iron_sword"), None);

        let decoded: ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
            "resource_ids": ["minecraft:wooden_spear"],
            "protocol_ids": {"minecraft:wooden_spear": 0}
        }))
        .unwrap();
        assert_eq!(decoded.default_use_effects("minecraft:wooden_spear"), None);
    }

    #[test]
    fn item_registry_catalog_parses_crafting_remainders() {
        let mut constants = BTreeMap::new();
        constants.insert("BUCKET".to_string(), "bucket".to_string());
        let source = r#"
            public class Items {
               public static final Item BUCKET = registerItem(ItemIds.BUCKET, new Item.Properties().stacksTo(16));
               public static final Item WATER_BUCKET = registerItem(
                  "water_bucket",
                  new Item.Properties().craftRemainder(BUCKET).stacksTo(1)
               );
               public static final Item GLASS_BOTTLE = registerItem("glass_bottle", new Item.Properties());
               public static final Item HONEY_BOTTLE = registerItem(
                  "honey_bottle",
                  new Item.Properties().craftRemainder(GLASS_BOTTLE).stacksTo(16)
               );
               public static final Item STONE = registerBlock(Blocks.STONE);
            }
        "#;

        let catalog = ItemRegistryCatalog::from_items_java_source(source, &constants).unwrap();

        assert_eq!(
            catalog.crafting_remainder("minecraft:water_bucket"),
            Some("minecraft:bucket")
        );
        assert_eq!(
            catalog.crafting_remainder("honey_bottle"),
            Some("minecraft:glass_bottle")
        );
        assert_eq!(catalog.crafting_remainder("minecraft:stone"), None);
        assert_eq!(
            catalog.crafting_remainders_by_protocol_id(),
            BTreeMap::from([(1, 0), (3, 2)])
        );

        let decoded: ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
            "resource_ids": ["minecraft:water_bucket", "minecraft:bucket"],
            "protocol_ids": {"minecraft:water_bucket": 0, "minecraft:bucket": 1}
        }))
        .unwrap();
        assert!(decoded.crafting_remainders_by_protocol_id().is_empty());
    }

    #[test]
    fn item_registry_catalog_parses_default_equipment_slots() {
        let source = r#"
            public class Items {
               public static final Item DIAMOND_HELMET = registerItem("diamond_helmet", new Item.Properties().humanoidArmor(ArmorMaterials.DIAMOND, ArmorType.HELMET));
               public static final Item DIAMOND_CHESTPLATE = registerItem(
                  "diamond_chestplate", new Item.Properties().humanoidArmor(ArmorMaterials.DIAMOND, ArmorType.CHESTPLATE)
               );
               public static final Item DIAMOND_LEGGINGS = registerItem("diamond_leggings", new Item.Properties().humanoidArmor(ArmorMaterials.DIAMOND, ArmorType.LEGGINGS));
               public static final Item DIAMOND_BOOTS = registerItem("diamond_boots", new Item.Properties().humanoidArmor(ArmorMaterials.DIAMOND, ArmorType.BOOTS));
               public static final Item BODY_ARMOR = registerItem("body_armor", new Item.Properties().humanoidArmor(ArmorMaterials.LEATHER, ArmorType.BODY));
               public static final Item SADDLE = registerItem("saddle", new Item.Properties().stacksTo(1).component(DataComponents.EQUIPPABLE, Equippable.saddle()));
               public static final Item HORSE_ARMOR = registerItem("horse_armor", new Item.Properties().horseArmor(ArmorMaterials.DIAMOND));
               public static final Item WHITE_CARPET = registerBlock(Blocks.WHITE_CARPET, p -> p.component(DataComponents.EQUIPPABLE, Equippable.llamaSwag(DyeColor.WHITE)));
               public static final Item NAUTILUS_ARMOR = registerItem("nautilus_armor", new Item.Properties().nautilusArmor(ArmorMaterials.IRON));
               public static final Item WOLF_ARMOR = registerItem("wolf_armor", new Item.Properties().wolfArmor(ArmorMaterials.ARMADILLO_SCUTE));
               public static final Item CARVED_PUMPKIN = registerBlock(
                  Blocks.CARVED_PUMPKIN,
                  p -> p.component(
                     DataComponents.EQUIPPABLE,
                     Equippable.builder(EquipmentSlot.HEAD).setSwappable(false).build()
                  )
               );
               public static final Item ELYTRA = registerItem(
                  "elytra",
                  new Item.Properties()
                     .component(
                        DataComponents.EQUIPPABLE,
                        Equippable.builder(EquipmentSlot.CHEST).build()
                     )
               );
               public static final Item SHIELD = registerItem(
                  "shield",
                  ShieldItem::new,
                  new Item.Properties().equippableUnswappable(EquipmentSlot.OFFHAND)
               );
               public static final Item OFFHAND_ITEM = registerItem("offhand_item", new Item.Properties().equippable(EquipmentSlot.OFFHAND));
               public static final Item MAINHAND_ITEM = registerItem("mainhand_item", new Item.Properties().equippable(EquipmentSlot.MAINHAND));
               public static final Item SADDLE_ITEM = registerItem("saddle_item", new Item.Properties().equippable(EquipmentSlot.SADDLE));
               public static final Item STONE = registerBlock(Blocks.STONE);
            }
        "#;

        let catalog =
            ItemRegistryCatalog::from_items_java_source(source, &BTreeMap::new()).unwrap();

        assert_eq!(
            catalog.equipment_slot("minecraft:diamond_helmet"),
            Some(ItemEquipmentSlot::Head)
        );
        assert_eq!(
            catalog.equipment_slot("minecraft:diamond_chestplate"),
            Some(ItemEquipmentSlot::Chest)
        );
        assert_eq!(
            catalog.equipment_slot("minecraft:diamond_leggings"),
            Some(ItemEquipmentSlot::Legs)
        );
        assert_eq!(
            catalog.equipment_slot("minecraft:diamond_boots"),
            Some(ItemEquipmentSlot::Feet)
        );
        assert_eq!(
            catalog.equipment_slot("minecraft:body_armor"),
            Some(ItemEquipmentSlot::Body)
        );
        assert_eq!(catalog.max_damage("minecraft:diamond_helmet"), Some(363));
        assert_eq!(
            catalog.max_damage("minecraft:diamond_chestplate"),
            Some(528)
        );
        assert_eq!(catalog.max_damage("minecraft:body_armor"), Some(80));
        assert_eq!(
            catalog.equipment_slot("minecraft:saddle"),
            Some(ItemEquipmentSlot::Saddle)
        );
        assert_eq!(
            catalog.equipment_slot("minecraft:horse_armor"),
            Some(ItemEquipmentSlot::Body)
        );
        assert_eq!(
            catalog.equipment_slot("minecraft:white_carpet"),
            Some(ItemEquipmentSlot::Body)
        );
        assert_eq!(
            catalog.equipment_slot("minecraft:nautilus_armor"),
            Some(ItemEquipmentSlot::Body)
        );
        assert_eq!(
            catalog.equipment_slot("minecraft:wolf_armor"),
            Some(ItemEquipmentSlot::Body)
        );
        assert_eq!(
            catalog.equipment_slot("minecraft:carved_pumpkin"),
            Some(ItemEquipmentSlot::Head)
        );
        assert_eq!(
            catalog.equipment_slot("minecraft:elytra"),
            Some(ItemEquipmentSlot::Chest)
        );
        assert_eq!(
            catalog.equipment_slot("minecraft:shield"),
            Some(ItemEquipmentSlot::OffHand)
        );
        assert_eq!(
            catalog.equipment_slot("minecraft:offhand_item"),
            Some(ItemEquipmentSlot::OffHand)
        );
        assert_eq!(
            catalog.equipment_slot("minecraft:mainhand_item"),
            Some(ItemEquipmentSlot::MainHand)
        );
        assert_eq!(
            catalog.equipment_slot("minecraft:saddle_item"),
            Some(ItemEquipmentSlot::Saddle)
        );
        assert_eq!(catalog.equipment_slot("minecraft:stone"), None);
        assert_eq!(catalog.equipment_slot("minecraft:missing_item"), None);

        // `humanoidArmor(ArmorMaterials.<MAT>, ...)` items resolve to the lowercased material asset; the
        // non-humanoid armors (horse/wolf/nautilus body armor) and plain items have no humanoid asset.
        assert_eq!(
            catalog.humanoid_armor_asset("minecraft:diamond_helmet"),
            Some("diamond")
        );
        assert_eq!(
            catalog.humanoid_armor_asset("minecraft:diamond_leggings"),
            Some("diamond")
        );
        assert_eq!(
            catalog.humanoid_armor_asset("minecraft:body_armor"),
            Some("leather")
        );
        assert_eq!(catalog.humanoid_armor_asset("minecraft:horse_armor"), None);
        assert_eq!(catalog.humanoid_armor_asset("minecraft:wolf_armor"), None);
        assert_eq!(catalog.humanoid_armor_asset("minecraft:stone"), None);
        assert_eq!(
            catalog.mount_body_armor_kind("minecraft:horse_armor"),
            Some(ItemMountBodyArmorKind::Horse)
        );
        assert_eq!(
            catalog.mount_body_armor_asset("minecraft:horse_armor"),
            Some("diamond")
        );
        assert_eq!(
            catalog.mount_body_armor_kind("minecraft:white_carpet"),
            Some(ItemMountBodyArmorKind::Llama)
        );
        assert_eq!(
            catalog.mount_body_armor_asset("minecraft:white_carpet"),
            None
        );
        assert_eq!(
            catalog.mount_body_armor_kind("minecraft:nautilus_armor"),
            Some(ItemMountBodyArmorKind::Nautilus)
        );
        assert_eq!(
            catalog.mount_body_armor_asset("minecraft:nautilus_armor"),
            Some("iron")
        );
        assert_eq!(catalog.mount_body_armor_kind("minecraft:wolf_armor"), None);
        assert_eq!(catalog.mount_body_armor_asset("minecraft:wolf_armor"), None);

        let mount_body_armor_kinds = catalog.mount_body_armor_kinds_by_protocol_id();
        assert_eq!(
            mount_body_armor_kinds.get(&catalog.protocol_id("minecraft:horse_armor").unwrap()),
            Some(&ItemMountBodyArmorKind::Horse)
        );
        assert_eq!(
            mount_body_armor_kinds.get(&catalog.protocol_id("minecraft:white_carpet").unwrap()),
            Some(&ItemMountBodyArmorKind::Llama)
        );
        assert_eq!(
            mount_body_armor_kinds.get(&catalog.protocol_id("minecraft:nautilus_armor").unwrap()),
            Some(&ItemMountBodyArmorKind::Nautilus)
        );
        assert!(!mount_body_armor_kinds
            .contains_key(&catalog.protocol_id("minecraft:wolf_armor").unwrap()));

        let encoded = serde_json::to_value(&catalog).unwrap();
        assert_eq!(
            encoded["default_equipment_slots"]["minecraft:shield"],
            serde_json::json!("offhand")
        );
        let decoded: ItemRegistryCatalog = serde_json::from_value(encoded).unwrap();
        assert_eq!(
            decoded.equipment_slot("minecraft:mainhand_item"),
            Some(ItemEquipmentSlot::MainHand)
        );
    }

    #[test]
    fn item_registry_catalog_parses_default_mining_profiles() {
        let source = r#"
            public class Items {
               public static final Item STONE_PICKAXE = registerItem("stone_pickaxe", new Item.Properties().pickaxe(ToolMaterial.STONE, 1.0F, -2.8F));
               public static final Item IRON_SHOVEL = registerItem("iron_shovel", p -> new ShovelItem(ToolMaterial.IRON, 1.5F, -3.0F, p));
               public static final Item WOODEN_SWORD = registerItem("wooden_sword", new Item.Properties().sword(ToolMaterial.WOOD, 3.0F, -2.4F));
               public static final Item SHEARS = registerItem(
                  "shears", ShearsItem::new, new Item.Properties().durability(238).component(DataComponents.TOOL, ShearsItem.createToolProperties())
               );
               public static final Item STONE = registerBlock(Blocks.STONE);
            }
        "#;
        let block_tags = test_block_tags();

        let catalog = ItemRegistryCatalog::from_items_java_source_with_block_tags(
            source,
            &BTreeMap::new(),
            Some(&block_tags),
        )
        .unwrap();

        let pickaxe = catalog.mining_profile("minecraft:stone_pickaxe").unwrap();
        assert_eq!(pickaxe.default_mining_speed_thousandths, 1_000);
        assert_eq!(pickaxe.rules.len(), 2);
        assert_eq!(pickaxe.rules[0].correct_for_drops, Some(false));
        assert_eq!(pickaxe.rules[0].mining_speed_thousandths, None);
        assert!(pickaxe.rules[0]
            .block_names
            .contains(&"minecraft:obsidian".to_string()));
        assert_eq!(pickaxe.rules[1].correct_for_drops, Some(true));
        assert_eq!(pickaxe.rules[1].mining_speed_thousandths, Some(4_000));
        assert!(pickaxe.rules[1]
            .block_names
            .contains(&"minecraft:stone".to_string()));

        let shovel = catalog.mining_profile("minecraft:iron_shovel").unwrap();
        assert_eq!(shovel.rules[1].mining_speed_thousandths, Some(6_000));
        assert!(shovel.rules[1]
            .block_names
            .contains(&"minecraft:dirt".to_string()));

        let sword = catalog.mining_profile("minecraft:wooden_sword").unwrap();
        assert_eq!(
            sword.rules[0].block_names,
            vec!["minecraft:cobweb".to_string()]
        );
        assert_eq!(sword.rules[0].mining_speed_thousandths, Some(15_000));
        assert_eq!(sword.rules[0].correct_for_drops, Some(true));
        assert_eq!(sword.rules[1].mining_speed_thousandths, Some(u32::MAX));
        assert!(sword.rules[2]
            .block_names
            .contains(&"minecraft:oak_leaves".to_string()));

        let shears = catalog.mining_profile("minecraft:shears").unwrap();
        assert_eq!(
            shears.rules[0].block_names,
            vec!["minecraft:cobweb".to_string()]
        );
        assert!(shears.rules[1]
            .block_names
            .contains(&"minecraft:oak_leaves".to_string()));
        assert!(shears.rules[2]
            .block_names
            .contains(&"minecraft:white_wool".to_string()));
        assert_eq!(
            shears.rules[3].block_names,
            vec![
                "minecraft:vine".to_string(),
                "minecraft:glow_lichen".to_string()
            ]
        );

        assert!(catalog.mining_profile("minecraft:stone").is_none());
    }

    #[test]
    fn item_registry_catalog_loads_java_sources() {
        let root = unique_temp_dir("item-registry");
        let sources = root.join("sources").join(crate::MC_VERSION);
        let item_dir = sources
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item");
        let references_dir = sources.join("net").join("minecraft").join("references");
        write_file(
            &references_dir.join("ItemIds.java"),
            r#"
                public class ItemIds {
                   public static final ResourceKey<Item> PUMPKIN_SEEDS = createKey("pumpkin_seeds");
                }
            "#,
        );
        write_file(
            &item_dir.join("Items.java"),
            r#"
                public class Items {
                   public static final Item AIR = registerBlock(Blocks.AIR, AirItem::new);
                   public static final Item PUMPKIN_SEEDS = registerItem(ItemIds.PUMPKIN_SEEDS, createBlockItemWithCustomItemName(Blocks.PUMPKIN_STEM));
                }
            "#,
        );

        let catalog = ItemRegistryCatalog::load(
            &PackRoots::from_root(&root).expect("test source root should load"),
        )
        .unwrap();

        assert_eq!(
            catalog.resource_ids(),
            &["minecraft:air", "minecraft:pumpkin_seeds"]
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_item_registry() {
        let catalog = PackRoots::discover()
            .unwrap()
            .load_item_registry_catalog()
            .unwrap();

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
        assert!(catalog.mining_profile("minecraft:shears").is_some());
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
    }

    fn write_file(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
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

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let id = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("bbb-pack-{label}-{nanos}-{id}"))
    }
}
