use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceLocation {
    namespace: String,
    path: String,
}

impl ResourceLocation {
    pub fn parse(value: &str) -> Result<Self> {
        let (namespace, path) = value.split_once(':').unwrap_or(("minecraft", value));
        Self::new(namespace, path)
    }

    pub fn new(namespace: impl Into<String>, path: impl Into<String>) -> Result<Self> {
        let namespace = namespace.into();
        let path = path.into();
        validate_resource_namespace(&namespace)?;
        validate_resource_path(&path)?;
        Ok(Self { namespace, path })
    }

    pub fn id(&self) -> String {
        format!("{}:{}", self.namespace, self.path)
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn with_suffix(&self, suffix: &str) -> Result<Self> {
        let path = format!("{}{}", self.path, suffix);
        Self::new(self.namespace.clone(), path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackResource {
    pub location: ResourceLocation,
    pub path: PathBuf,
    pub metadata_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackResourceStack {
    roots: Vec<PathBuf>,
}

impl PackResourceStack {
    pub fn from_roots(roots: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        Self {
            roots: roots.into_iter().map(Into::into).collect(),
        }
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    pub fn namespaces(&self) -> Result<Vec<String>> {
        self.namespaces_in("assets")
    }

    pub fn data_namespaces(&self) -> Result<Vec<String>> {
        self.namespaces_in("data")
    }

    fn namespaces_in(&self, domain: &str) -> Result<Vec<String>> {
        let mut namespaces = BTreeMap::new();
        for pack in self.pack_entries(domain) {
            for content_root in pack.content_roots {
                let domain_root = content_root.join(domain);
                if !domain_root.is_dir() {
                    continue;
                }
                for entry in std::fs::read_dir(&domain_root)
                    .with_context(|| format!("read {domain} directory {}", domain_root.display()))?
                {
                    let entry = entry.with_context(|| {
                        format!("read {domain} entry in {}", domain_root.display())
                    })?;
                    let path = entry.path();
                    if !entry
                        .file_type()
                        .with_context(|| format!("read file type {}", path.display()))?
                        .is_dir()
                    {
                        continue;
                    }
                    let namespace = entry
                        .file_name()
                        .into_string()
                        .map_err(|name| anyhow::anyhow!("non-utf8 {domain} namespace {name:?}"))?;
                    validate_resource_namespace(&namespace)?;
                    namespaces.insert(namespace.clone(), namespace);
                }
            }
        }
        Ok(namespaces.into_values().collect())
    }

    pub fn get_resource(&self, location: &ResourceLocation) -> Option<PackResource> {
        self.get_resource_in("assets", location)
    }

    pub fn get_data_resource(&self, location: &ResourceLocation) -> Option<PackResource> {
        self.get_resource_in("data", location)
    }

    fn get_resource_in(&self, domain: &str, location: &ResourceLocation) -> Option<PackResource> {
        let pack_entries = self.pack_entries(domain);
        for (pack_index, pack) in pack_entries.iter().enumerate().rev() {
            if let Some(mut resource) = pack.get_resource(domain, location) {
                resource.metadata_path =
                    metadata_path_for_resource(domain, location, &pack_entries, pack_index);
                return Some(resource);
            }
            if pack.is_filtered(location) {
                return None;
            }
        }
        None
    }

    pub fn get_resource_stack(&self, location: &ResourceLocation) -> Vec<PackResource> {
        self.get_resource_stack_in("assets", location)
    }

    pub fn get_data_resource_stack(&self, location: &ResourceLocation) -> Vec<PackResource> {
        self.get_resource_stack_in("data", location)
    }

    fn get_resource_stack_in(
        &self,
        domain: &str,
        location: &ResourceLocation,
    ) -> Vec<PackResource> {
        let metadata_location = metadata_location(location).ok();
        let mut resources = Vec::new();
        let mut filter_metadata = false;
        for pack in self.pack_entries(domain).iter().rev() {
            if let Some(mut resource) = pack.get_resource(domain, location) {
                if !filter_metadata {
                    resource.metadata_path = metadata_location
                        .as_ref()
                        .and_then(|location| pack.resource_path(domain, location));
                }
                resources.push(resource);
            }
            if pack.is_filtered(location) {
                break;
            }
            if metadata_location
                .as_ref()
                .is_some_and(|location| pack.is_filtered(location))
            {
                filter_metadata = true;
            }
        }
        resources.reverse();
        resources
    }

    pub fn list_resources(&self, path_prefix: &str, extension: &str) -> Result<Vec<PackResource>> {
        self.list_resources_in("assets", path_prefix, extension)
    }

    pub fn list_data_resources(
        &self,
        path_prefix: &str,
        extension: &str,
    ) -> Result<Vec<PackResource>> {
        self.list_resources_in("data", path_prefix, extension)
    }

    fn list_resources_in(
        &self,
        domain: &str,
        path_prefix: &str,
        extension: &str,
    ) -> Result<Vec<PackResource>> {
        validate_resource_path_prefix(path_prefix)?;
        let pack_entries = self.pack_entries(domain);
        let mut resources: BTreeMap<ResourceLocation, (usize, PackResource)> = BTreeMap::new();
        for (pack_index, pack) in pack_entries.iter().enumerate() {
            if let Some(filter) = &pack.filter {
                resources.retain(|location, _| !filter.is_filtered(location));
            }
            for (location, resource) in pack.list_resources(domain, path_prefix, extension)? {
                resources.insert(location, (pack_index, resource));
            }
        }
        Ok(resources
            .into_iter()
            .map(|(location, (pack_index, mut resource))| {
                resource.metadata_path =
                    metadata_path_for_resource(domain, &location, &pack_entries, pack_index);
                resource
            })
            .collect())
    }

    pub fn list_data_resource_stacks(
        &self,
        path_prefix: &str,
        extension: &str,
    ) -> Result<BTreeMap<ResourceLocation, Vec<PackResource>>> {
        self.list_resource_stacks_in("data", path_prefix, extension)
    }

    fn list_resource_stacks_in(
        &self,
        domain: &str,
        path_prefix: &str,
        extension: &str,
    ) -> Result<BTreeMap<ResourceLocation, Vec<PackResource>>> {
        validate_resource_path_prefix(path_prefix)?;
        let mut resources: BTreeMap<ResourceLocation, Vec<PackResource>> = BTreeMap::new();
        for pack in self.pack_entries(domain) {
            if let Some(filter) = &pack.filter {
                resources.retain(|location, stack| {
                    if filter.is_filtered(location) {
                        return false;
                    }
                    if metadata_location(location)
                        .ok()
                        .as_ref()
                        .is_some_and(|metadata_location| filter.is_filtered(metadata_location))
                    {
                        for resource in stack {
                            resource.metadata_path = None;
                        }
                    }
                    true
                });
            }
            for (location, mut resource) in pack.list_resources(domain, path_prefix, extension)? {
                resource.metadata_path = metadata_location(&location)
                    .ok()
                    .and_then(|metadata_location| pack.resource_path(domain, &metadata_location));
                resources.entry(location).or_default().push(resource);
            }
        }
        Ok(resources)
    }

    fn pack_entries(&self, domain: &str) -> Vec<PackEntry> {
        self.roots
            .iter()
            .cloned()
            .map(|root| PackEntry::new(root, domain))
            .collect()
    }
}

fn collect_resources(
    namespace_dir: &Path,
    dir: &Path,
    namespace: &str,
    extension: &str,
    resources: &mut BTreeMap<ResourceLocation, PackResource>,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("read resource directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("read resource entry in {}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("read file type {}", path.display()))?;
        if file_type.is_dir() {
            collect_resources(namespace_dir, &path, namespace, extension, resources)?;
            continue;
        }
        let resource_path = relative_resource_path(namespace_dir, &path)?;
        if !resource_path.ends_with(extension) {
            continue;
        }
        let location = ResourceLocation::new(namespace, resource_path)?;
        resources.insert(
            location.clone(),
            PackResource {
                location,
                path: path.clone(),
                metadata_path: None,
            },
        );
    }
    Ok(())
}

fn relative_resource_path(root: &Path, path: &Path) -> Result<String> {
    let relative = path
        .strip_prefix(root)
        .with_context(|| format!("strip resource root {}", root.display()))?;
    let mut parts = Vec::new();
    for component in relative.components() {
        let std::path::Component::Normal(part) = component else {
            bail!("invalid resource path component in {}", path.display());
        };
        let part = part
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("non-utf8 resource path {}", path.display()))?;
        parts.push(part);
    }
    Ok(parts.join("/"))
}

fn resource_path_in(root: &Path, domain: &str, location: &ResourceLocation) -> PathBuf {
    root.join(domain)
        .join(location.namespace())
        .join(location.path())
}

#[derive(Debug)]
struct PackEntry {
    content_roots: Vec<PathBuf>,
    filter: Option<PackFilter>,
}

impl PackEntry {
    fn new(root: PathBuf, domain: &str) -> Self {
        let mut content_roots = vec![root.clone()];
        content_roots.extend(
            applicable_overlays(&root, pack_format_for_domain(domain))
                .into_iter()
                .map(|overlay| root.join(overlay)),
        );
        Self {
            filter: pack_filter(&root),
            content_roots,
        }
    }

    fn is_filtered(&self, location: &ResourceLocation) -> bool {
        self.filter
            .as_ref()
            .is_some_and(|filter| filter.is_filtered(location))
    }

    fn get_resource(&self, domain: &str, location: &ResourceLocation) -> Option<PackResource> {
        self.resource_path(domain, location)
            .map(|path| PackResource {
                location: location.clone(),
                path,
                metadata_path: None,
            })
    }

    fn resource_path(&self, domain: &str, location: &ResourceLocation) -> Option<PathBuf> {
        self.content_roots.iter().rev().find_map(|root| {
            let path = resource_path_in(root, domain, location);
            path.is_file().then_some(path)
        })
    }

    fn list_resources(
        &self,
        domain: &str,
        path_prefix: &str,
        extension: &str,
    ) -> Result<BTreeMap<ResourceLocation, PackResource>> {
        let mut resources = BTreeMap::new();
        for root in &self.content_roots {
            let domain_root = root.join(domain);
            if !domain_root.is_dir() {
                continue;
            }
            for namespace_entry in std::fs::read_dir(&domain_root)
                .with_context(|| format!("read {domain} directory {}", domain_root.display()))?
            {
                let namespace_entry = namespace_entry
                    .with_context(|| format!("read {domain} entry in {}", domain_root.display()))?;
                let namespace_dir = namespace_entry.path();
                if !namespace_entry
                    .file_type()
                    .with_context(|| format!("read file type {}", namespace_dir.display()))?
                    .is_dir()
                {
                    continue;
                }
                let namespace = namespace_entry
                    .file_name()
                    .into_string()
                    .map_err(|name| anyhow::anyhow!("non-utf8 {domain} namespace {name:?}"))?;
                validate_resource_namespace(&namespace)?;
                let list_root = namespace_dir.join(path_prefix);
                if !list_root.is_dir() {
                    continue;
                }
                collect_resources(
                    &namespace_dir,
                    &list_root,
                    &namespace,
                    extension,
                    &mut resources,
                )?;
            }
        }
        Ok(resources)
    }
}

fn metadata_location(location: &ResourceLocation) -> Result<ResourceLocation> {
    location.with_suffix(".mcmeta")
}

fn metadata_path_for_resource(
    domain: &str,
    location: &ResourceLocation,
    pack_entries: &[PackEntry],
    resource_pack_index: usize,
) -> Option<PathBuf> {
    let metadata_location = metadata_location(location).ok()?;
    for pack in pack_entries[resource_pack_index..].iter().rev() {
        if let Some(path) = pack.resource_path(domain, &metadata_location) {
            return Some(path);
        }
        if pack.is_filtered(&metadata_location) {
            break;
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PackFormat {
    major: u32,
    minor: u32,
}

const CLIENT_RESOURCE_PACK_FORMAT: PackFormat = PackFormat {
    major: 84,
    minor: 0,
};
const SERVER_DATA_PACK_FORMAT: PackFormat = PackFormat {
    major: 101,
    minor: 1,
};

fn pack_format_for_domain(domain: &str) -> PackFormat {
    match domain {
        "data" => SERVER_DATA_PACK_FORMAT,
        _ => CLIENT_RESOURCE_PACK_FORMAT,
    }
}

#[derive(Debug)]
struct PackFilter {
    blocks: Vec<ResourceFilterPattern>,
}

impl PackFilter {
    fn is_filtered(&self, location: &ResourceLocation) -> bool {
        self.blocks
            .iter()
            .any(|pattern| pattern.is_match(location.namespace(), location.path()))
    }
}

#[derive(Debug)]
struct ResourceFilterPattern {
    namespace: Option<Regex>,
    path: Option<Regex>,
}

impl ResourceFilterPattern {
    fn is_match(&self, namespace: &str, path: &str) -> bool {
        self.namespace
            .as_ref()
            .map_or(true, |pattern| pattern.is_match(namespace))
            && self
                .path
                .as_ref()
                .map_or(true, |pattern| pattern.is_match(path))
    }
}

#[derive(Debug, Deserialize)]
struct RawPackMetadata {
    filter: Option<RawResourceFilterSection>,
    overlays: Option<RawOverlaySection>,
}

#[derive(Debug, Deserialize)]
struct RawOverlaySection {
    entries: Vec<RawOverlayEntry>,
}

#[derive(Debug, Deserialize)]
struct RawOverlayEntry {
    directory: String,
    min_format: Option<RawPackFormat>,
    max_format: Option<RawPackFormat>,
    formats: Option<RawMajorFormatRange>,
}

impl RawOverlayEntry {
    fn applies_to(&self, current: PackFormat) -> bool {
        if !is_valid_overlay_directory(&self.directory) {
            return false;
        }
        if let (Some(min), Some(max)) = (self.min_format, self.max_format) {
            return current >= min.into_pack_format(0) && current <= max.into_pack_format(u32::MAX);
        }
        self.formats
            .is_some_and(|range| range.contains(current.major))
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(untagged)]
enum RawPackFormat {
    Major(u32),
    Full([u32; 2]),
}

impl RawPackFormat {
    fn into_pack_format(self, default_minor: u32) -> PackFormat {
        match self {
            Self::Major(major) => PackFormat {
                major,
                minor: default_minor,
            },
            Self::Full([major, minor]) => PackFormat { major, minor },
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(untagged)]
enum RawMajorFormatRange {
    Single(u32),
    Pair([u32; 2]),
    Object {
        min_inclusive: u32,
        max_inclusive: u32,
    },
}

impl RawMajorFormatRange {
    fn contains(self, value: u32) -> bool {
        let (min, max) = match self {
            Self::Single(value) => (value, value),
            Self::Pair([min, max]) => (min, max),
            Self::Object {
                min_inclusive,
                max_inclusive,
            } => (min_inclusive, max_inclusive),
        };
        min <= value && value <= max
    }
}

#[derive(Debug, Deserialize)]
struct RawResourceFilterSection {
    block: Vec<RawResourceFilterPattern>,
}

#[derive(Debug, Deserialize)]
struct RawResourceFilterPattern {
    namespace: Option<String>,
    path: Option<String>,
}

impl RawResourceFilterPattern {
    fn into_pattern(self) -> Result<ResourceFilterPattern> {
        Ok(ResourceFilterPattern {
            namespace: self
                .namespace
                .map(|pattern| Regex::new(&pattern))
                .transpose()?,
            path: self.path.map(|pattern| Regex::new(&pattern)).transpose()?,
        })
    }
}

fn pack_filter(root: &Path) -> Option<PackFilter> {
    let metadata_path = root.join("pack.mcmeta");
    let bytes = std::fs::read(metadata_path).ok()?;
    let metadata: RawPackMetadata = serde_json::from_slice(&bytes).ok()?;
    let filter = metadata.filter?;
    let blocks = filter
        .block
        .into_iter()
        .map(RawResourceFilterPattern::into_pattern)
        .collect::<Result<Vec<_>>>()
        .ok()?;
    Some(PackFilter { blocks })
}

fn applicable_overlays(root: &Path, current_format: PackFormat) -> Vec<String> {
    let metadata_path = root.join("pack.mcmeta");
    let Ok(bytes) = std::fs::read(metadata_path) else {
        return Vec::new();
    };
    let Ok(metadata) = serde_json::from_slice::<RawPackMetadata>(&bytes) else {
        return Vec::new();
    };
    metadata
        .overlays
        .map(|overlays| {
            overlays
                .entries
                .into_iter()
                .filter(|entry| entry.applies_to(current_format))
                .map(|entry| entry.directory)
                .collect()
        })
        .unwrap_or_default()
}

fn is_valid_overlay_directory(directory: &str) -> bool {
    !directory.is_empty()
        && directory != "."
        && directory != ".."
        && directory
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

pub(crate) fn validate_resource_namespace(namespace: &str) -> Result<()> {
    if namespace.is_empty() || namespace == ".." {
        bail!("invalid resource namespace {namespace:?}");
    }
    if !namespace.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.')
    }) {
        bail!("invalid resource namespace {namespace:?}");
    }
    Ok(())
}

pub(crate) fn validate_resource_path(path: &str) -> Result<()> {
    if path.is_empty()
        || !path.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b'.' | b'/')
        })
        || !path
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
    {
        bail!("invalid resource path {path:?}");
    }
    Ok(())
}

fn validate_resource_path_prefix(path: &str) -> Result<()> {
    if path.is_empty() {
        return Ok(());
    }
    validate_resource_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn resource_location_matches_vanilla_identifier_validation() {
        assert_eq!(
            ResourceLocation::parse("block/stone").unwrap().id(),
            "minecraft:block/stone"
        );
        assert!(ResourceLocation::parse("minecraft:block/stone").is_ok());
        assert!(ResourceLocation::parse("example:path.with-dash_1").is_ok());
        assert!(ResourceLocation::parse("minecraft:Block/Stone").is_err());
        assert!(ResourceLocation::parse("bad namespace:block").is_err());
        assert!(ResourceLocation::parse("..:block").is_err());
        assert!(ResourceLocation::parse("minecraft:../block").is_err());
        assert!(ResourceLocation::parse("minecraft:textures/../block/stone.png").is_err());
        assert!(ResourceLocation::parse("minecraft:textures//block/stone.png").is_err());
    }

    #[test]
    fn resource_stack_resolves_highest_precedence_resource_and_lists_unique_ids() {
        let root = unique_temp_dir("resource-stack");
        let base = root.join("base");
        let overlay = root.join("overlay");
        write_file(
            &base
                .join("assets")
                .join("minecraft")
                .join("textures")
                .join("block")
                .join("stone.png"),
            b"base-stone",
        );
        write_file(
            &overlay
                .join("assets")
                .join("minecraft")
                .join("textures")
                .join("block")
                .join("stone.png"),
            b"overlay-stone",
        );
        write_file(
            &overlay
                .join("assets")
                .join("example")
                .join("textures")
                .join("block")
                .join("gem.png"),
            b"overlay-gem",
        );

        let stack = PackResourceStack::from_roots(vec![base, overlay]);
        assert_eq!(stack.namespaces().unwrap(), vec!["example", "minecraft"]);

        let stone = ResourceLocation::parse("minecraft:textures/block/stone.png").unwrap();
        let resolved = stack.get_resource(&stone).unwrap();
        assert!(resolved
            .path
            .ends_with("overlay/assets/minecraft/textures/block/stone.png"));
        assert_eq!(stack.get_resource_stack(&stone).len(), 2);

        let listed = stack
            .list_resources("textures/block", ".png")
            .unwrap()
            .into_iter()
            .map(|resource| resource.location.id())
            .collect::<Vec<_>>();
        assert_eq!(
            listed,
            vec![
                "example:textures/block/gem.png",
                "minecraft:textures/block/stone.png"
            ]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resource_stack_filter_blocks_lower_priority_assets() {
        let root = unique_temp_dir("resource-filter-assets");
        let base = root.join("base");
        let filter = root.join("filter");
        write_file(
            &base
                .join("assets")
                .join("minecraft")
                .join("textures")
                .join("block")
                .join("stone.png"),
            b"base-stone",
        );
        write_file(
            &filter.join("pack.mcmeta"),
            br#"{
              "filter": {
                "block": [
                  {
                    "namespace": "minecraft",
                    "path": "textures/block/stone\\.png"
                  }
                ]
              }
            }"#,
        );

        let stack = PackResourceStack::from_roots(vec![base, filter]);
        let stone = ResourceLocation::parse("minecraft:textures/block/stone.png").unwrap();

        assert!(stack.get_resource(&stone).is_none());
        assert!(stack.get_resource_stack(&stone).is_empty());
        assert!(stack
            .list_resources("textures/block", ".png")
            .unwrap()
            .is_empty());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resource_stack_filter_keeps_current_pack_resource() {
        let root = unique_temp_dir("resource-filter-current-pack");
        let base = root.join("base");
        let overlay = root.join("overlay");
        let overlay_stone = overlay
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("block")
            .join("stone.png");
        write_file(
            &base
                .join("assets")
                .join("minecraft")
                .join("textures")
                .join("block")
                .join("stone.png"),
            b"base-stone",
        );
        write_file(&overlay_stone, b"overlay-stone");
        write_file(
            &overlay.join("pack.mcmeta"),
            br#"{
              "filter": {
                "block": [
                  {
                    "namespace": "minecraft",
                    "path": "textures/block/stone\\.png"
                  }
                ]
              }
            }"#,
        );

        let stack = PackResourceStack::from_roots(vec![base, overlay]);
        let stone = ResourceLocation::parse("minecraft:textures/block/stone.png").unwrap();
        let resolved = stack.get_resource(&stone).unwrap();
        let resource_stack = stack.get_resource_stack(&stone);

        assert!(resolved.path.ends_with(&overlay_stone));
        assert_eq!(resource_stack.len(), 1);
        assert!(resource_stack[0].path.ends_with(&overlay_stone));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn single_resource_and_list_resources_use_highest_precedence_metadata() {
        let root = unique_temp_dir("resource-metadata-precedence");
        let base = root.join("base");
        let overlay = root.join("overlay");
        let base_stone = base
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("block")
            .join("stone.png");
        let base_metadata = base_stone.with_file_name("stone.png.mcmeta");
        let overlay_metadata = overlay
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("block")
            .join("stone.png.mcmeta");
        write_file(&base_stone, b"base-stone");
        write_file(&base_metadata, b"base-metadata");
        write_file(&overlay_metadata, b"overlay-metadata");

        let stack = PackResourceStack::from_roots([base, overlay]);
        let stone = ResourceLocation::parse("minecraft:textures/block/stone.png").unwrap();
        let resolved = stack.get_resource(&stone).unwrap();
        let listed = stack.list_resources("textures/block", ".png").unwrap();
        let resource_stack = stack.get_resource_stack(&stone);

        assert!(resolved.path.ends_with(&base_stone));
        assert!(resolved
            .metadata_path
            .as_ref()
            .unwrap()
            .ends_with(&overlay_metadata));
        assert_eq!(listed.len(), 1);
        assert!(listed[0]
            .metadata_path
            .as_ref()
            .unwrap()
            .ends_with(&overlay_metadata));
        assert_eq!(resource_stack.len(), 1);
        assert!(resource_stack[0]
            .metadata_path
            .as_ref()
            .unwrap()
            .ends_with(&base_metadata));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn metadata_filter_clears_metadata_without_filtering_resource() {
        let root = unique_temp_dir("resource-metadata-filter");
        let base = root.join("base");
        let filter = root.join("filter");
        let base_stone = base
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("block")
            .join("stone.png");
        write_file(&base_stone, b"base-stone");
        write_file(
            &base_stone.with_file_name("stone.png.mcmeta"),
            b"base-metadata",
        );
        write_file(
            &filter.join("pack.mcmeta"),
            br#"{
              "filter": {
                "block": [
                  {
                    "namespace": "minecraft",
                    "path": "textures/block/stone\\.png\\.mcmeta"
                  }
                ]
              }
            }"#,
        );

        let stack = PackResourceStack::from_roots([base, filter]);
        let stone = ResourceLocation::parse("minecraft:textures/block/stone.png").unwrap();
        let resolved = stack.get_resource(&stone).unwrap();
        let listed = stack.list_resources("textures/block", ".png").unwrap();
        let resource_stack = stack.get_resource_stack(&stone);

        assert!(resolved.path.ends_with(&base_stone));
        assert_eq!(resolved.metadata_path, None);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].metadata_path, None);
        assert_eq!(resource_stack.len(), 1);
        assert_eq!(resource_stack[0].metadata_path, None);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resource_stack_filter_blocks_lower_priority_data_resources() {
        let root = unique_temp_dir("resource-filter-data");
        let base = root.join("base");
        let filter = root.join("filter");
        write_file(
            &base
                .join("data")
                .join("minecraft")
                .join("tags")
                .join("block")
                .join("logs.json"),
            br#"{"values":["minecraft:oak_log"]}"#,
        );
        write_file(
            &filter.join("pack.mcmeta"),
            br#"{
              "filter": {
                "block": [
                  {
                    "namespace": "minecraft",
                    "path": "tags/block/logs\\.json"
                  }
                ]
              }
            }"#,
        );

        let stack = PackResourceStack::from_roots(vec![base, filter]);
        let logs = ResourceLocation::parse("minecraft:tags/block/logs.json").unwrap();

        assert!(stack.get_data_resource(&logs).is_none());
        assert!(stack.get_data_resource_stack(&logs).is_empty());
        assert!(stack
            .list_data_resource_stacks("tags/block", ".json")
            .unwrap()
            .is_empty());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resource_stack_overlay_overrides_primary_within_same_pack() {
        let root = unique_temp_dir("resource-overlay-assets");
        let pack = root.join("pack");
        let primary_stone = pack
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("block")
            .join("stone.png");
        let overlay_stone = pack
            .join("overlay_84")
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("block")
            .join("stone.png");
        write_file(&primary_stone, b"primary-stone");
        write_file(&overlay_stone, b"overlay-stone");
        write_file(
            &pack.join("pack.mcmeta"),
            br#"{
              "overlays": {
                "entries": [
                  {
                    "directory": "overlay_84",
                    "min_format": [84, 0],
                    "max_format": [84, 0]
                  }
                ]
              }
            }"#,
        );

        let stack = PackResourceStack::from_roots([pack]);
        let stone = ResourceLocation::parse("minecraft:textures/block/stone.png").unwrap();
        let resolved = stack.get_resource(&stone).unwrap();
        let resource_stack = stack.get_resource_stack(&stone);
        let listed = stack.list_resources("textures/block", ".png").unwrap();

        assert!(resolved.path.ends_with(&overlay_stone));
        assert_eq!(resource_stack.len(), 1);
        assert!(resource_stack[0].path.ends_with(&overlay_stone));
        assert_eq!(listed.len(), 1);
        assert!(listed[0].path.ends_with(&overlay_stone));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resource_stack_overlays_are_selected_by_pack_type_format() {
        let root = unique_temp_dir("resource-overlay-formats");
        let pack = root.join("pack");
        let primary_texture = pack
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("block")
            .join("stone.png");
        let data_overlay_texture = pack
            .join("data_overlay")
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("block")
            .join("stone.png");
        let primary_tag = pack
            .join("data")
            .join("minecraft")
            .join("tags")
            .join("block")
            .join("logs.json");
        let data_overlay_tag = pack
            .join("data_overlay")
            .join("data")
            .join("minecraft")
            .join("tags")
            .join("block")
            .join("logs.json");
        write_file(&primary_texture, b"primary-texture");
        write_file(&data_overlay_texture, b"data-overlay-texture");
        write_file(&primary_tag, br#"{"values":["minecraft:oak_log"]}"#);
        write_file(&data_overlay_tag, br#"{"values":["minecraft:birch_log"]}"#);
        write_file(
            &pack.join("pack.mcmeta"),
            br#"{
              "overlays": {
                "entries": [
                  {
                    "directory": "data_overlay",
                    "min_format": [101, 1],
                    "max_format": [101, 1]
                  }
                ]
              }
            }"#,
        );

        let stack = PackResourceStack::from_roots([pack]);
        let stone = ResourceLocation::parse("minecraft:textures/block/stone.png").unwrap();
        let logs = ResourceLocation::parse("minecraft:tags/block/logs.json").unwrap();
        let texture = stack.get_resource(&stone).unwrap();
        let tag = stack.get_data_resource(&logs).unwrap();

        assert!(texture.path.ends_with(&primary_texture));
        assert!(tag.path.ends_with(&data_overlay_tag));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resource_stack_ignores_path_traversal_overlay_directories() {
        let root = unique_temp_dir("resource-overlay-traversal");
        let pack = root.join("pack");
        let primary_stone = pack
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("block")
            .join("stone.png");
        let escaped_stone = root
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("block")
            .join("stone.png");
        write_file(&primary_stone, b"primary-stone");
        write_file(&escaped_stone, b"escaped-stone");
        write_file(
            &pack.join("pack.mcmeta"),
            br#"{
              "overlays": {
                "entries": [
                  {
                    "directory": "..",
                    "min_format": [84, 0],
                    "max_format": [84, 0]
                  }
                ]
              }
            }"#,
        );

        let stack = PackResourceStack::from_roots([pack]);
        let stone = ResourceLocation::parse("minecraft:textures/block/stone.png").unwrap();
        let resolved = stack.get_resource(&stone).unwrap();
        let listed = stack.list_resources("textures/block", ".png").unwrap();

        assert!(resolved.path.ends_with(&primary_stone));
        assert_eq!(listed.len(), 1);
        assert!(listed[0].path.ends_with(&primary_stone));

        std::fs::remove_dir_all(root).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        dir.push(format!("bbb-pack-{label}-{nanos}"));
        dir
    }

    fn write_file(path: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
}
