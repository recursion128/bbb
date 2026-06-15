use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::resources::{validate_resource_path, PackResourceStack, ResourceLocation};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagCatalog {
    pub registry_path: String,
    pub tags: BTreeMap<String, TagDefinition>,
}

impl TagCatalog {
    pub fn load_resource_stack(stack: &PackResourceStack, registry_path: &str) -> Result<Self> {
        validate_resource_path(registry_path)?;
        let directory = tag_directory(registry_path);
        let resource_stacks = stack.list_data_resource_stacks(&directory, ".json")?;
        let mut builders: BTreeMap<String, Vec<TagEntry>> = BTreeMap::new();

        for (location, resources) in resource_stacks {
            let tag_id = tag_id_from_resource_location(&location, &directory)?;
            for resource in resources {
                let Ok(bytes) = std::fs::read(&resource.path) else {
                    continue;
                };
                let Ok(tag_file) = RawTagFile::from_json_bytes(&bytes).with_context(|| {
                    format!("parse tag {} from {}", tag_id.id(), resource.path.display())
                }) else {
                    continue;
                };
                let entries = builders.entry(tag_id.id()).or_default();
                if tag_file.replace {
                    entries.clear();
                }
                entries.extend(tag_file.values);
            }
        }

        Ok(Self {
            registry_path: registry_path.to_string(),
            tags: build_tags(builders),
        })
    }

    pub fn values(&self, tag_id: &str) -> Option<&[String]> {
        let tag_id = ResourceLocation::parse(tag_id).ok()?.id();
        self.tags.get(&tag_id).map(|tag| tag.values.as_slice())
    }

    pub fn contains(&self, tag_id: &str, element_id: &str) -> bool {
        let Ok(element_id) = ResourceLocation::parse(element_id).map(|location| location.id())
        else {
            return false;
        };
        self.values(tag_id)
            .is_some_and(|values| values.iter().any(|value| value == &element_id))
    }

    pub fn len(&self) -> usize {
        self.tags.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagDefinition {
    pub id: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TagEntry {
    id: String,
    tag: bool,
    required: bool,
}

#[derive(Debug, Deserialize)]
struct RawTagFile {
    #[serde(default)]
    replace: bool,
    values: Vec<RawTagEntry>,
}

impl RawTagFile {
    fn from_json_bytes(bytes: &[u8]) -> Result<TagFile> {
        let raw: Self = serde_json::from_slice(bytes)?;
        let values = raw
            .values
            .into_iter()
            .map(RawTagEntry::into_entry)
            .collect::<Result<Vec<_>>>()?;
        Ok(TagFile {
            replace: raw.replace,
            values,
        })
    }
}

#[derive(Debug)]
struct TagFile {
    replace: bool,
    values: Vec<TagEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawTagEntry {
    Id(String),
    Object {
        id: String,
        #[serde(default = "required_by_default")]
        required: bool,
    },
}

impl RawTagEntry {
    fn into_entry(self) -> Result<TagEntry> {
        match self {
            Self::Id(id) => parse_tag_entry_id(&id, true),
            Self::Object { id, required } => parse_tag_entry_id(&id, required),
        }
    }
}

fn required_by_default() -> bool {
    true
}

fn parse_tag_entry_id(id: &str, required: bool) -> Result<TagEntry> {
    let (tag, location) = id
        .strip_prefix('#')
        .map_or((false, id), |location| (true, location));
    let id = ResourceLocation::parse(location)?.id();
    Ok(TagEntry { id, tag, required })
}

fn tag_directory(registry_path: &str) -> String {
    format!("tags/{registry_path}")
}

fn tag_id_from_resource_location(
    location: &ResourceLocation,
    directory: &str,
) -> Result<ResourceLocation> {
    let path = location.path();
    let Some(relative) = path.strip_prefix(directory) else {
        bail!(
            "tag resource {} is outside directory {directory:?}",
            location.id()
        );
    };
    let relative = relative.strip_prefix('/').unwrap_or(relative);
    let Some(tag_path) = relative.strip_suffix(".json") else {
        bail!("tag resource {} does not end with .json", location.id());
    };
    ResourceLocation::new(location.namespace().to_string(), tag_path.to_string())
}

fn build_tags(builders: BTreeMap<String, Vec<TagEntry>>) -> BTreeMap<String, TagDefinition> {
    let mut resolved = BTreeMap::new();
    let mut failed = BTreeSet::new();
    for id in builders.keys() {
        let mut resolving = BTreeSet::new();
        let _ = resolve_tag(id, &builders, &mut resolved, &mut failed, &mut resolving);
    }
    resolved
        .into_iter()
        .map(|(id, values)| {
            let definition = TagDefinition {
                id: id.clone(),
                values,
            };
            (id, definition)
        })
        .collect()
}

fn resolve_tag(
    id: &str,
    builders: &BTreeMap<String, Vec<TagEntry>>,
    resolved: &mut BTreeMap<String, Vec<String>>,
    failed: &mut BTreeSet<String>,
    resolving: &mut BTreeSet<String>,
) -> Option<Vec<String>> {
    if let Some(values) = resolved.get(id) {
        return Some(values.clone());
    }
    if failed.contains(id) || !resolving.insert(id.to_string()) {
        failed.insert(id.to_string());
        return None;
    }

    let Some(entries) = builders.get(id) else {
        failed.insert(id.to_string());
        resolving.remove(id);
        return None;
    };

    let mut values = Vec::new();
    let mut seen = BTreeSet::new();
    for entry in entries {
        if entry.tag {
            match resolve_tag(&entry.id, builders, resolved, failed, resolving) {
                Some(tag_values) => {
                    for value in tag_values {
                        push_unique(&mut values, &mut seen, value);
                    }
                }
                None if entry.required => {
                    failed.insert(id.to_string());
                    resolving.remove(id);
                    return None;
                }
                None => {}
            }
        } else {
            push_unique(&mut values, &mut seen, entry.id.clone());
        }
    }

    resolving.remove(id);
    resolved.insert(id.to_string(), values.clone());
    Some(values)
}

fn push_unique(values: &mut Vec<String>, seen: &mut BTreeSet<String>, value: String) {
    if seen.insert(value.clone()) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests {
    use crate::{PackRoots, MC_VERSION};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn tag_catalog_merges_data_resource_stack_with_replace() {
        let root = unique_temp_dir("tag-stack");
        let base = root.join("sources").join(MC_VERSION);
        let overlay = root.join("overlay");
        write_json(
            &base
                .join("data")
                .join("minecraft")
                .join("tags")
                .join("block")
                .join("logs.json"),
            r#"{
              "values": [
                "minecraft:oak_log",
                "minecraft:spruce_log"
              ]
            }"#,
        );
        write_json(
            &overlay
                .join("data")
                .join("minecraft")
                .join("tags")
                .join("block")
                .join("logs.json"),
            r#"{
              "replace": true,
              "values": [
                "minecraft:birch_log"
              ]
            }"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([overlay]);
        let catalog = roots.load_tag_catalog("block").unwrap();

        assert_eq!(
            catalog.values("minecraft:logs").unwrap(),
            &["minecraft:birch_log".to_string()]
        );
        assert!(catalog.contains("logs", "birch_log"));
        assert!(!catalog.contains("logs", "oak_log"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tag_catalog_resolves_tag_references_and_deduplicates_values() {
        let root = unique_temp_dir("tag-references");
        let data_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("tags")
            .join("item");
        write_json(
            &data_dir.join("logs.json"),
            r#"{
              "values": [
                "minecraft:oak_log",
                "minecraft:oak_log",
                {"id": "minecraft:spruce_log", "required": false}
              ]
            }"#,
        );
        write_json(
            &data_dir.join("fuel.json"),
            r##"{
              "values": [
                "#minecraft:logs",
                {"id": "#minecraft:missing_optional", "required": false},
                "minecraft:coal"
              ]
            }"##,
        );
        write_json(
            &data_dir.join("broken.json"),
            r##"{
              "values": [
                "#minecraft:missing_required"
              ]
            }"##,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_tag_catalog("item")
            .unwrap();

        assert_eq!(
            catalog.values("minecraft:fuel").unwrap(),
            &[
                "minecraft:oak_log".to_string(),
                "minecraft:spruce_log".to_string(),
                "minecraft:coal".to_string()
            ]
        );
        assert!(catalog.values("minecraft:broken").is_none());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tag_catalog_skips_invalid_tag_file_without_creating_empty_tag() {
        let root = unique_temp_dir("tag-invalid");
        let data_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("tags")
            .join("block");
        write_json(&data_dir.join("invalid.json"), "{");
        write_json(
            &data_dir.join("valid.json"),
            r#"{"values":["minecraft:stone"]}"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_tag_catalog("block")
            .unwrap();

        assert!(catalog.values("minecraft:invalid").is_none());
        assert_eq!(
            catalog.values("minecraft:valid").unwrap(),
            &["minecraft:stone".to_string()]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_block_tags() {
        let catalog = PackRoots::discover()
            .unwrap()
            .load_tag_catalog("block")
            .unwrap();

        assert!(catalog.len() > 100);
        assert!(catalog.contains("minecraft:logs", "minecraft:oak_log"));
        assert!(catalog.contains("minecraft:mineable/pickaxe", "minecraft:stone"));
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-pack-{label}-{nanos}"))
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }
}
