use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

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

    pub fn get_resource(&self, location: &ResourceLocation) -> Option<PackResource> {
        self.roots.iter().rev().find_map(|root| {
            let path = resource_path(root, location);
            path.is_file().then(|| PackResource {
                location: location.clone(),
                path,
            })
        })
    }

    pub fn get_resource_stack(&self, location: &ResourceLocation) -> Vec<PackResource> {
        self.roots
            .iter()
            .filter_map(|root| {
                let path = resource_path(root, location);
                path.is_file().then(|| PackResource {
                    location: location.clone(),
                    path,
                })
            })
            .collect()
    }

    pub fn list_resources(&self, path_prefix: &str, extension: &str) -> Result<Vec<PackResource>> {
        validate_resource_path_prefix(path_prefix)?;
        let mut resources = BTreeMap::new();
        for root in &self.roots {
            let assets_root = root.join("assets");
            if !assets_root.is_dir() {
                continue;
            }
            for namespace_entry in std::fs::read_dir(&assets_root)
                .with_context(|| format!("read assets directory {}", assets_root.display()))?
            {
                let namespace_entry = namespace_entry
                    .with_context(|| format!("read assets entry in {}", assets_root.display()))?;
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
                    .map_err(|name| anyhow::anyhow!("non-utf8 asset namespace {name:?}"))?;
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
        Ok(resources.into_values().collect())
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

fn resource_path(root: &Path, location: &ResourceLocation) -> PathBuf {
    root.join("assets")
        .join(location.namespace())
        .join(location.path())
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
