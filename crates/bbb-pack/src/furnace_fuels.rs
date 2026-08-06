use std::collections::BTreeSet;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{ItemRegistryCatalog, PackRoots, ResourceLocation, TagCatalog};

const GENERATED_FURNACE_FUEL_DECLARATIONS: &str =
    include_str!("../data/furnace_fuel_declarations_26_1.json");

#[derive(Debug, Deserialize)]
struct FurnaceFuelSourceCatalog {
    vanilla_version: String,
    sources: Vec<String>,
    operations: Vec<FurnaceFuelSourceOperation>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum FurnaceFuelSourceAction {
    Add,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum FurnaceFuelSourceEntryKind {
    Item,
    Block,
    Tag,
}

#[derive(Debug, Deserialize)]
struct FurnaceFuelSourceOperation {
    action: FurnaceFuelSourceAction,
    entry_kind: FurnaceFuelSourceEntryKind,
    constant: String,
    resource_id: String,
    source_path: String,
    start_byte: usize,
    end_byte: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FurnaceFuelCatalog {
    item_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FurnaceFuelSourceRule {
    AddItem(String),
    AddTag(String),
    RemoveTag(String),
}

impl FurnaceFuelCatalog {
    pub fn load(roots: &PackRoots, registry: &ItemRegistryCatalog) -> Result<Self> {
        let item_tags = roots.load_tag_catalog("item")?;
        Self::from_generated_source_catalog(registry, &item_tags)
    }

    fn from_generated_source_catalog(
        registry: &ItemRegistryCatalog,
        item_tags: &TagCatalog,
    ) -> Result<Self> {
        let source = generated_source_catalog()?;
        let mut item_ids = BTreeSet::new();
        for operation in source.operations {
            let source_context = || {
                format!(
                    "{}:{}..{}: resolve {}",
                    operation.source_path,
                    operation.start_byte,
                    operation.end_byte,
                    operation.constant
                )
            };
            validate_source_operation(&operation).with_context(source_context)?;
            let resolved = match operation.entry_kind {
                FurnaceFuelSourceEntryKind::Item | FurnaceFuelSourceEntryKind::Block => {
                    vec![operation.resource_id]
                }
                FurnaceFuelSourceEntryKind::Tag => item_tags
                    .values(&operation.resource_id)
                    .unwrap_or_default()
                    .to_vec(),
            };
            match operation.action {
                FurnaceFuelSourceAction::Add => {
                    for item_id in &resolved {
                        if registry.protocol_id(&item_id).is_some() {
                            item_ids.insert(item_id.clone());
                        }
                    }
                }
                FurnaceFuelSourceAction::Remove => {
                    for item_id in &resolved {
                        item_ids.remove(item_id);
                    }
                }
            }
        }

        if item_ids.is_empty() {
            bail!("FuelValues.java did not contain resolvable furnace fuel items");
        }
        Ok(Self { item_ids })
    }

    /// Returns the ordered `FuelValues.Builder` membership rules without
    /// resolving item tags from a resource pack. Native startup maps direct
    /// item ids once and gives the rules to `bbb-world`, which can then apply
    /// the current session `UpdateTags` state.
    pub fn source_rules() -> Result<Vec<FurnaceFuelSourceRule>> {
        generated_source_catalog()?
            .operations
            .into_iter()
            .map(|operation| {
                validate_source_operation(&operation).with_context(|| {
                    format!(
                        "{}:{}..{}: resolve {}",
                        operation.source_path,
                        operation.start_byte,
                        operation.end_byte,
                        operation.constant
                    )
                })?;
                match (operation.action, operation.entry_kind) {
                    (FurnaceFuelSourceAction::Add, FurnaceFuelSourceEntryKind::Item)
                    | (FurnaceFuelSourceAction::Add, FurnaceFuelSourceEntryKind::Block) => {
                        Ok(FurnaceFuelSourceRule::AddItem(operation.resource_id))
                    }
                    (FurnaceFuelSourceAction::Add, FurnaceFuelSourceEntryKind::Tag) => {
                        Ok(FurnaceFuelSourceRule::AddTag(operation.resource_id))
                    }
                    (FurnaceFuelSourceAction::Remove, FurnaceFuelSourceEntryKind::Tag) => {
                        Ok(FurnaceFuelSourceRule::RemoveTag(operation.resource_id))
                    }
                    (FurnaceFuelSourceAction::Remove, _) => {
                        bail!("generated furnace-fuel remove operation does not reference a tag")
                    }
                }
            })
            .collect()
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

fn generated_source_catalog() -> Result<FurnaceFuelSourceCatalog> {
    let source: FurnaceFuelSourceCatalog =
        serde_json::from_str(GENERATED_FURNACE_FUEL_DECLARATIONS)
            .context("parse generated furnace-fuel declaration catalog")?;
    if source.vanilla_version != crate::MC_VERSION
        || source.sources
            != [
                "net/minecraft/tags/ItemTags.java",
                "net/minecraft/world/level/block/entity/FuelValues.java",
            ]
    {
        bail!("unsupported generated furnace-fuel source catalog manifest");
    }
    Ok(source)
}

fn validate_source_operation(operation: &FurnaceFuelSourceOperation) -> Result<()> {
    if operation.source_path != "net/minecraft/world/level/block/entity/FuelValues.java"
        || operation.start_byte >= operation.end_byte
        || operation.constant.is_empty()
    {
        bail!("invalid generated furnace-fuel source evidence");
    }
    ResourceLocation::parse(&operation.resource_id)
        .context("invalid generated furnace-fuel resource id")?;
    if matches!(operation.action, FurnaceFuelSourceAction::Remove)
        && operation.entry_kind != FurnaceFuelSourceEntryKind::Tag
    {
        bail!("generated furnace-fuel remove operation does not reference a tag");
    }
    Ok(())
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
    fn furnace_fuel_catalog_resolves_generated_items_blocks_tags_and_removals() {
        let root = unique_temp_dir("furnace-fuels");
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
    fn generated_furnace_fuel_operations_are_complete_and_source_backed() {
        let source: FurnaceFuelSourceCatalog =
            serde_json::from_str(GENERATED_FURNACE_FUEL_DECLARATIONS).unwrap();

        assert_eq!(source.operations.len(), 65);
        assert_eq!(
            source
                .operations
                .iter()
                .filter(|operation| matches!(operation.action, FurnaceFuelSourceAction::Add))
                .count(),
            64
        );
        assert_eq!(
            source
                .operations
                .iter()
                .filter(|operation| matches!(operation.action, FurnaceFuelSourceAction::Remove))
                .count(),
            1
        );
        for operation in &source.operations {
            validate_source_operation(operation).unwrap();
        }

        let rules = FurnaceFuelCatalog::source_rules().unwrap();
        assert_eq!(rules.len(), 65);
        assert_eq!(
            rules.first(),
            Some(&FurnaceFuelSourceRule::AddItem(
                "minecraft:lava_bucket".to_string()
            ))
        );
        assert_eq!(
            rules.last(),
            Some(&FurnaceFuelSourceRule::RemoveTag(
                "minecraft:non_flammable_wood".to_string()
            ))
        );
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_furnace_fuel_catalog() {
        let roots = PackRoots::from_environment().unwrap();
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
