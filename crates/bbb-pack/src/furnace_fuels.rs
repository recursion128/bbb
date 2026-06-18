use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{ItemRegistryCatalog, PackRoots, ResourceLocation, TagCatalog};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FurnaceFuelCatalog {
    item_ids: BTreeSet<String>,
}

impl FurnaceFuelCatalog {
    pub fn load(roots: &PackRoots, registry: &ItemRegistryCatalog) -> Result<Self> {
        let fuel_values_java = roots
            .sources_dir
            .join("net")
            .join("minecraft")
            .join("world")
            .join("level")
            .join("block")
            .join("entity")
            .join("FuelValues.java");
        let item_tags_java = roots
            .sources_dir
            .join("net")
            .join("minecraft")
            .join("tags")
            .join("ItemTags.java");
        let fuel_values_source = std::fs::read_to_string(&fuel_values_java)
            .with_context(|| format!("read furnace fuel source {}", fuel_values_java.display()))?;
        let item_tags_source = std::fs::read_to_string(&item_tags_java)
            .with_context(|| format!("read item tag source {}", item_tags_java.display()))?;
        let item_tag_constants = parse_item_tag_constants(&item_tags_source)?;
        let item_tags = roots.load_tag_catalog("item")?;
        Self::from_java_source(
            &fuel_values_source,
            registry,
            &item_tag_constants,
            &item_tags,
        )
    }

    fn from_java_source(
        source: &str,
        registry: &ItemRegistryCatalog,
        item_tag_constants: &BTreeMap<String, String>,
        item_tags: &TagCatalog,
    ) -> Result<Self> {
        let operation = Regex::new(r#"\.(add|remove)\(\s*(Items|Blocks|ItemTags)\.([A-Z0-9_]+)"#)?;
        let mut item_ids = BTreeSet::new();
        for capture in operation.captures_iter(source) {
            let action = capture.get(1).unwrap().as_str();
            let namespace = capture.get(2).unwrap().as_str();
            let constant = capture.get(3).unwrap().as_str();
            let resolved = resolve_fuel_entry(namespace, constant, item_tag_constants, item_tags)?;
            match action {
                "add" => {
                    for item_id in resolved {
                        if registry.protocol_id(&item_id).is_some() {
                            item_ids.insert(item_id);
                        }
                    }
                }
                "remove" => {
                    for item_id in resolved {
                        item_ids.remove(&item_id);
                    }
                }
                _ => unreachable!("fuel operation regex only matches add/remove"),
            }
        }

        if item_ids.is_empty() {
            bail!("FuelValues.java did not contain resolvable furnace fuel items");
        }
        Ok(Self { item_ids })
    }

    pub fn item_ids(&self) -> &BTreeSet<String> {
        &self.item_ids
    }

    pub fn protocol_ids(&self, registry: &ItemRegistryCatalog) -> BTreeSet<i32> {
        self.item_ids
            .iter()
            .filter_map(|item_id| registry.protocol_id(item_id))
            .collect()
    }
}

fn resolve_fuel_entry(
    namespace: &str,
    constant: &str,
    item_tag_constants: &BTreeMap<String, String>,
    item_tags: &TagCatalog,
) -> Result<Vec<String>> {
    match namespace {
        "Items" | "Blocks" => Ok(vec![minecraft_id(&constant.to_ascii_lowercase())?]),
        "ItemTags" => {
            let tag_id = match item_tag_constants.get(constant) {
                Some(tag_id) => tag_id.clone(),
                None => minecraft_id(&constant.to_ascii_lowercase())?,
            };
            Ok(item_tags.values(&tag_id).unwrap_or_default().to_vec())
        }
        _ => bail!("unsupported furnace fuel entry namespace {namespace}"),
    }
}

fn parse_item_tag_constants(source: &str) -> Result<BTreeMap<String, String>> {
    let declaration = Regex::new(
        r#"public\s+static\s+final\s+TagKey<Item>\s+([A-Z0-9_]+)\s*=\s*bind\("([^"]+)"\)"#,
    )?;
    let mut constants = BTreeMap::new();
    for capture in declaration.captures_iter(source) {
        let constant = capture.get(1).unwrap().as_str().to_string();
        let id = minecraft_id(capture.get(2).unwrap().as_str())?;
        constants.insert(constant, id);
    }
    Ok(constants)
}

fn minecraft_id(path: &str) -> Result<String> {
    ResourceLocation::new("minecraft", path).map(|location| location.id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn furnace_fuel_catalog_resolves_java_builder_items_blocks_tags_and_removals() {
        let root = unique_temp_dir("furnace-fuels");
        write_java_sources(&root);
        write_item_tags(&root);
        let roots = PackRoots::from_root(&root).unwrap();
        let registry = ItemRegistryCatalog::load(&roots).unwrap();

        let catalog = FurnaceFuelCatalog::load(&roots, &registry).unwrap();

        assert!(catalog.item_ids().contains("minecraft:coal"));
        assert!(catalog.item_ids().contains("minecraft:coal_block"));
        assert!(catalog.item_ids().contains("minecraft:oak_log"));
        assert!(!catalog.item_ids().contains("minecraft:crimson_stem"));

        let protocol_ids = catalog.protocol_ids(&registry);
        assert!(protocol_ids.contains(&registry.protocol_id("minecraft:coal").unwrap()));
        assert!(protocol_ids.contains(&registry.protocol_id("minecraft:coal_block").unwrap()));
        assert!(protocol_ids.contains(&registry.protocol_id("minecraft:oak_log").unwrap()));
        assert!(!protocol_ids.contains(&registry.protocol_id("minecraft:crimson_stem").unwrap()));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_furnace_fuel_catalog() {
        let roots = PackRoots::discover().unwrap();
        let registry = ItemRegistryCatalog::load(&roots).unwrap();

        let catalog = FurnaceFuelCatalog::load(&roots, &registry).unwrap();
        let protocol_ids = catalog.protocol_ids(&registry);

        assert!(catalog.item_ids().contains("minecraft:lava_bucket"));
        assert!(catalog.item_ids().contains("minecraft:coal"));
        assert!(catalog.item_ids().contains("minecraft:oak_log"));
        assert!(!catalog.item_ids().contains("minecraft:crimson_stem"));
        assert!(protocol_ids.contains(&registry.protocol_id("minecraft:coal").unwrap()));
        assert!(protocol_ids.len() > 50);
    }

    fn write_java_sources(root: &Path) {
        write_file(
            &root
                .join("sources")
                .join(crate::MC_VERSION)
                .join("net")
                .join("minecraft")
                .join("world")
                .join("item")
                .join("Items.java"),
            r#"public class Items {
                public static final Item COAL = registerItem("coal");
                public static final Item COAL_BLOCK = registerItem("coal_block");
                public static final Item OAK_LOG = registerItem("oak_log");
                public static final Item CRIMSON_STEM = registerItem("crimson_stem");
            }"#,
        );
        write_file(
            &root
                .join("sources")
                .join(crate::MC_VERSION)
                .join("net")
                .join("minecraft")
                .join("tags")
                .join("ItemTags.java"),
            r#"public final class ItemTags {
                public static final TagKey<Item> LOGS = bind("logs");
                public static final TagKey<Item> NON_FLAMMABLE_WOOD = bind("non_flammable_wood");
            }"#,
        );
        write_file(
            &root
                .join("sources")
                .join(crate::MC_VERSION)
                .join("net")
                .join("minecraft")
                .join("world")
                .join("level")
                .join("block")
                .join("entity")
                .join("FuelValues.java"),
            r#"public class FuelValues {
                public static FuelValues vanillaBurnTimes() {
                    return new FuelValues.Builder()
                        .add(Items.COAL, 1600)
                        .add(Blocks.COAL_BLOCK, 16000)
                        .add(ItemTags.LOGS, 300)
                        .remove(ItemTags.NON_FLAMMABLE_WOOD)
                        .build();
                }
            }"#,
        );
    }

    fn write_item_tags(root: &Path) {
        let tag_dir = root
            .join("sources")
            .join(crate::MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("tags")
            .join("item");
        write_file(
            &tag_dir.join("logs.json"),
            r#"{
              "values": [
                "minecraft:oak_log",
                "minecraft:crimson_stem"
              ]
            }"#,
        );
        write_file(
            &tag_dir.join("non_flammable_wood.json"),
            r#"{
              "values": [
                "minecraft:crimson_stem"
              ]
            }"#,
        );
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-pack-{name}-{nanos}"))
    }
}
