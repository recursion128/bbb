use std::{collections::BTreeMap, path::Path};

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
    #[serde(default)]
    mining_profiles: BTreeMap<String, ItemMiningProfile>,
}

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
        let mut mining_profiles = BTreeMap::new();
        for capture in declaration.captures_iter(source) {
            let kind = capture.get(1).unwrap().as_str();
            let field = capture.get(2).unwrap().as_str();
            let expression = capture.get(3).unwrap().as_str();
            let ids = resource_ids_for_declaration(kind, field, expression, item_id_constants)?;
            let stack_size = max_stack_size_for_declaration(expression)?;
            let equipment_slot = equipment_slot_for_declaration(expression)?;
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

    pub fn max_stack_size(&self, resource_id: &str) -> Option<i32> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.max_stack_size.get(&resource_id).copied()
    }

    pub fn equipment_slot(&self, resource_id: &str) -> Option<ItemEquipmentSlot> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.default_equipment_slots.get(&resource_id).copied()
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
    optional_capture(r#"\.durability\(\s*([0-9]+)\s*\)"#, expression)?
        .map(|value| value.parse().map_err(Into::into))
        .transpose()
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
        assert_eq!(catalog.max_damage("minecraft:trial_key"), None);
        assert_eq!(catalog.max_stack_size("minecraft:trial_key"), Some(64));
        assert_eq!(catalog.max_stack_size("minecraft:elytra"), Some(1));
        assert_eq!(catalog.max_stack_size("minecraft:ender_pearl"), Some(16));
        assert_eq!(catalog.max_stack_size("minecraft:iron_sword"), Some(1));
        assert_eq!(catalog.resource_id(-1), None);
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
               public static final Item BODY_ARMOR = registerItem("body_armor", new Item.Properties().humanoidArmor(ArmorMaterials.TEST, ArmorType.BODY));
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
        assert!(catalog
            .mining_profile("minecraft:diamond_pickaxe")
            .is_some());
        assert!(catalog.mining_profile("minecraft:shears").is_some());
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
