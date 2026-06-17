use std::{collections::BTreeMap, path::Path};

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{resources::ResourceLocation, PackRoots};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemRegistryCatalog {
    resource_ids: Vec<String>,
    protocol_ids: BTreeMap<String, i32>,
    #[serde(default)]
    max_damage: BTreeMap<String, i32>,
    #[serde(default)]
    max_stack_size: BTreeMap<String, i32>,
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
        Self::load_from_java_sources(items_java, item_ids_java)
    }

    pub fn load_from_java_sources(
        items_java: impl AsRef<Path>,
        item_ids_java: impl AsRef<Path>,
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
        Self::from_items_java_source(&items_source, &item_id_constants)
    }

    fn from_items_java_source(
        source: &str,
        item_id_constants: &BTreeMap<String, String>,
    ) -> Result<Self> {
        let declaration = Regex::new(
            r#"(?s)public\s+static\s+final\s+(Item|WeatheringCopperItems)\s+([A-Z0-9_]+)\s*=\s*(.*?);"#,
        )?;
        let mut resource_ids = Vec::new();
        let mut max_damage = BTreeMap::new();
        let mut max_stack_size = BTreeMap::new();
        for capture in declaration.captures_iter(source) {
            let kind = capture.get(1).unwrap().as_str();
            let field = capture.get(2).unwrap().as_str();
            let expression = capture.get(3).unwrap().as_str();
            let ids = resource_ids_for_declaration(kind, field, expression, item_id_constants)?;
            let stack_size = max_stack_size_for_declaration(expression)?;
            if let Some(durability) = durability_for_declaration(expression)? {
                for resource_id in &ids {
                    max_damage.insert(resource_id.clone(), durability);
                }
            }
            for resource_id in &ids {
                max_stack_size.insert(resource_id.clone(), stack_size);
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
    }

    fn write_file(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
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
