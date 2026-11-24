use std::collections::{BTreeMap, HashSet};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::resources::{validate_resource_path, PackResourceStack, ResourceLocation};

const MAX_TAG_RESOLUTION_DEPTH: usize = 128;
const MAX_RESOLVED_TAG_VALUES_PER_TAG: usize = 65_536;
const MAX_TOTAL_RESOLVED_TAG_VALUES: usize = 262_144;
const TAG_DEFINITION_LOGICAL_BYTES: u64 = 128;
const TAG_VALUE_LOGICAL_BYTES: u64 = 64;

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
                let raw: RawTagFile = match stack.read_json_resource(&resource, "tag definition") {
                    Ok(raw) => raw,
                    Err(err) => {
                        stack.ensure_resource_data_admitted().map_err(|_| err)?;
                        continue;
                    }
                };
                let Ok(tag_file) = raw.into_tag_file().with_context(|| {
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
            tags: build_tags(&builders, stack)?,
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
    fn into_tag_file(self) -> Result<TagFile> {
        let values = self
            .values
            .into_iter()
            .map(RawTagEntry::into_entry)
            .collect::<Result<Vec<_>>>()?;
        Ok(TagFile {
            replace: self.replace,
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

struct TagResolveFrame<'a> {
    id: &'a str,
    entries: &'a [TagEntry],
    next_index: usize,
    values: Vec<&'a str>,
    seen: HashSet<&'a str>,
}

fn build_tags(
    builders: &BTreeMap<String, Vec<TagEntry>>,
    resource_stack: &PackResourceStack,
) -> Result<BTreeMap<String, TagDefinition>> {
    let mut resolved: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut failed: HashSet<&str> = HashSet::new();
    let mut total_values = 0usize;

    for (root_id, root_entries) in builders {
        let root_id = root_id.as_str();
        if resolved.contains_key(root_id) || failed.contains(root_id) {
            continue;
        }
        let mut frames = vec![TagResolveFrame {
            id: root_id,
            entries: root_entries.as_slice(),
            next_index: 0,
            values: Vec::new(),
            seen: HashSet::new(),
        }];
        let mut in_stack = HashSet::new();
        in_stack.insert(root_id);

        while let Some(frame) = frames.last_mut() {
            if frame.next_index < frame.entries.len() {
                let entry = &frame.entries[frame.next_index];
                if !entry.tag {
                    frame.next_index += 1;
                    push_unique_ref(
                        &mut frame.values,
                        &mut frame.seen,
                        entry.id.as_str(),
                        total_values,
                        resource_stack,
                    )?;
                    continue;
                }
                let target_id = entry.id.as_str();
                if let Some(child_values) = resolved.get(target_id) {
                    frame.next_index += 1;
                    for &value in child_values {
                        push_unique_ref(
                            &mut frame.values,
                            &mut frame.seen,
                            value,
                            total_values,
                            resource_stack,
                        )?;
                    }
                    continue;
                }
                let unresolvable = failed.contains(target_id)
                    || in_stack.contains(target_id)
                    || !builders.contains_key(target_id);
                if unresolvable {
                    if entry.required {
                        let popped = frames.pop().expect("tag resolution frame present");
                        in_stack.remove(popped.id);
                        failed.insert(popped.id);
                    } else {
                        frame.next_index += 1;
                    }
                    continue;
                }
                if frames.len() >= MAX_TAG_RESOLUTION_DEPTH {
                    return Err(resource_stack.reject_resource_data(format!(
                        "tag resolution depth exceeded {MAX_TAG_RESOLUTION_DEPTH} while resolving {target_id}"
                    )));
                }
                let (key, child_entries) = builders
                    .get_key_value(target_id)
                    .expect("checked builders contains target tag");
                let key = key.as_str();
                in_stack.insert(key);
                frames.push(TagResolveFrame {
                    id: key,
                    entries: child_entries.as_slice(),
                    next_index: 0,
                    values: Vec::new(),
                    seen: HashSet::new(),
                });
            } else {
                let popped = frames.pop().expect("tag resolution frame present");
                in_stack.remove(popped.id);
                total_values = total_values
                    .checked_add(popped.values.len())
                    .ok_or_else(|| {
                        resource_stack.reject_resource_data("resolved tag value count overflow")
                    })?;
                resource_stack.admit_resource_collection_count(
                    u64::try_from(total_values).unwrap_or(u64::MAX),
                    MAX_TOTAL_RESOLVED_TAG_VALUES as u64,
                    "resolved tag values",
                )?;
                let derived_bytes = popped
                    .values
                    .iter()
                    .try_fold(TAG_DEFINITION_LOGICAL_BYTES, |acc, value| {
                        acc.checked_add(TAG_VALUE_LOGICAL_BYTES)
                            .and_then(|bytes| bytes.checked_add(value.len() as u64))
                    })
                    .ok_or_else(|| {
                        resource_stack.reject_resource_data("resolved tag logical byte overflow")
                    })?;
                resource_stack
                    .reserve_derived_resource_bytes(derived_bytes, "resolved tag catalog")?;
                resolved.insert(popped.id, popped.values);
            }
        }
    }

    Ok(resolved
        .into_iter()
        .map(|(id, values)| {
            let owned_id = id.to_string();
            let definition = TagDefinition {
                id: owned_id.clone(),
                values: values.into_iter().map(str::to_string).collect(),
            };
            (owned_id, definition)
        })
        .collect())
}

fn push_unique_ref<'a>(
    values: &mut Vec<&'a str>,
    seen: &mut HashSet<&'a str>,
    value: &'a str,
    committed_total: usize,
    resource_stack: &PackResourceStack,
) -> Result<()> {
    if seen.insert(value) {
        let next_tag_len = values.len().saturating_add(1);
        resource_stack.admit_resource_collection_count(
            u64::try_from(next_tag_len).unwrap_or(u64::MAX),
            MAX_RESOLVED_TAG_VALUES_PER_TAG as u64,
            "tag values per tag",
        )?;
        let projected_total = committed_total.saturating_add(next_tag_len);
        resource_stack.admit_resource_collection_count(
            u64::try_from(projected_total).unwrap_or(u64::MAX),
            MAX_TOTAL_RESOLVED_TAG_VALUES as u64,
            "resolved tag values",
        )?;
        values.push(value);
    }
    Ok(())
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
    fn tag_catalog_rejects_deep_tag_chains_iteratively_and_handles_cycles() {
        let root = unique_temp_dir("tag-depth-and-cycle");
        let data_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("data/minecraft/tags/block");

        // Required cycle A -> B -> A fails cleanly without looping.
        write_json(
            &data_dir.join("cycle_a.json"),
            r##"{"values":["#minecraft:cycle_b"]}"##,
        );
        write_json(
            &data_dir.join("cycle_b.json"),
            r##"{"values":["#minecraft:cycle_a"]}"##,
        );
        let cycle_catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_tag_catalog("block")
            .unwrap();
        assert!(cycle_catalog.values("minecraft:cycle_a").is_none());
        assert!(cycle_catalog.values("minecraft:cycle_b").is_none());

        // Chain exceeding MAX_TAG_RESOLUTION_DEPTH is rejected cleanly without stack overflow.
        let depth = super::MAX_TAG_RESOLUTION_DEPTH + 4;
        for i in 0..depth {
            write_json(
                &data_dir.join(format!("chain_{i:04}.json")),
                &format!(r##"{{"values":["#minecraft:chain_{:04}"]}}"##, i + 1),
            );
        }
        write_json(
            &data_dir.join(format!("chain_{depth:04}.json")),
            r#"{"values":["minecraft:stone"]}"#,
        );
        let roots = PackRoots::from_root(&root).unwrap();
        let err = roots.load_tag_catalog("block").unwrap_err();
        assert!(
            format!("{err:#}").contains("tag resolution depth exceeded"),
            "{err:#}"
        );
        assert!(roots.ensure_resource_data_admitted().is_err());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tag_catalog_ignores_tags_injected_by_server_resource_pack() {
        let root = unique_temp_dir("tag-server-isolation");
        let base = root.join("sources").join(MC_VERSION);
        let server_pack_dir = root.join("server_pack");
        write_json(
            &base.join("data/minecraft/tags/block/logs.json"),
            r#"{"values":["minecraft:oak_log"]}"#,
        );
        write_json(
            &server_pack_dir.join("pack.mcmeta"),
            r#"{"pack":{"description":"server","min_format":[84,0],"max_format":[84,0]}}"#,
        );
        write_json(
            &server_pack_dir.join("data/minecraft/tags/block/logs.json"),
            r#"{"replace":true,"values":["minecraft:bedrock"]}"#,
        );

        let base_roots = PackRoots::from_root(&root).unwrap();
        let metadata = crate::read_client_resource_pack_metadata(&server_pack_dir).unwrap();
        let server_pack =
            crate::SelectedClientPack::server("evil", 1, 0, &server_pack_dir, metadata).unwrap();
        let selected = crate::SelectedClientPackStack::from_startup_dirs(&base_roots, [])
            .unwrap()
            .with_server_packs([server_pack])
            .unwrap();
        let roots = base_roots.with_selected_client_packs(selected).unwrap();

        let catalog = roots.load_tag_catalog("block").unwrap();
        assert_eq!(
            catalog.values("minecraft:logs").unwrap(),
            &["minecraft:oak_log".to_string()]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_block_tags() {
        let catalog = PackRoots::from_environment()
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
        std::env::temp_dir().join(format!("bbb-client-assets-{label}-{nanos}"))
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }
}
