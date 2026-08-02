use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::{de::DeserializeOwned, Deserialize};
use sha1::{Digest, Sha1};

use crate::{
    asset_index::{IndexedAsset, VanillaAssetIndex},
    image::{
        PngDecodeBudget, ResourceImageGenerationBudget, ResourceImageMemoryObservation,
        MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES,
    },
    resource_budget::{
        ResourceDataGenerationBudget, ResourceDataMemoryObservation, ResourceListingAdmission,
        MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES,
    },
    ClientPackSource, SelectedClientPack,
};

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
    pub pack_id: String,
    pub location: ResourceLocation,
    pub path: PathBuf,
    pub metadata_path: Option<PathBuf>,
    pub integrity: Option<ResourceIntegrity>,
    pub metadata_integrity: Option<ResourceIntegrity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackRootResource {
    pub pack_id: String,
    pub path: PathBuf,
    pub integrity: Option<ResourceIntegrity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceIntegrity {
    pub size: u64,
    pub sha1: String,
}

impl ResourceIntegrity {
    pub fn verify(&self, path: &Path, label: &str) -> Result<()> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("open {label} {}", path.display()))?;
        let actual_size = file
            .metadata()
            .with_context(|| format!("read {label} metadata {}", path.display()))?
            .len();
        if actual_size != self.size {
            bail!(
                "{label} {} has size {actual_size}, expected {}",
                path.display(),
                self.size
            );
        }
        let maximum = self
            .size
            .checked_add(1)
            .context("resource integrity read limit overflow")?;
        let mut reader = std::io::Read::take(file, maximum);
        let mut sha1 = Sha1::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let count = std::io::Read::read(&mut reader, &mut buffer)
                .with_context(|| format!("hash {label}"))?;
            if count == 0 {
                break;
            }
            sha1.update(&buffer[..count]);
        }
        let actual_sha1 = format!("{:x}", sha1.finalize());
        if actual_sha1 != self.sha1 {
            bail!(
                "{label} {} has SHA-1 {actual_sha1}, expected {}",
                path.display(),
                self.sha1
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PackResourceStack {
    roots: Arc<Vec<PathBuf>>,
    entries: Arc<Vec<PackEntry>>,
    start: usize,
    image_budget: Arc<ResourceImageGenerationBudget>,
    data_budget: Arc<ResourceDataGenerationBudget>,
}

impl PackResourceStack {
    pub fn from_roots(roots: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        let roots = roots.into_iter().map(Into::into).collect::<Vec<_>>();
        let image_budget =
            ResourceImageGenerationBudget::new(MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES);
        let data_budget =
            ResourceDataGenerationBudget::new(MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES);
        let pack_ids = (0..roots.len())
            .map(|index| format!("unmanaged/{index}"))
            .collect::<Vec<_>>();
        let entries = roots
            .iter()
            .cloned()
            .zip(pack_ids)
            .map(|(root, pack_id)| {
                PackEntry::new_unmanaged(pack_id, root, Arc::clone(&data_budget))
            })
            .collect();
        Self::from_parts(roots, entries, image_budget, data_budget)
    }

    pub(crate) fn from_roots_with_budgets(
        roots: impl IntoIterator<Item = impl Into<PathBuf>>,
        image_budget: Arc<ResourceImageGenerationBudget>,
        data_budget: Arc<ResourceDataGenerationBudget>,
    ) -> Self {
        let roots = roots.into_iter().map(Into::into).collect::<Vec<_>>();
        let pack_ids = (0..roots.len())
            .map(|index| format!("unmanaged/{index}"))
            .collect::<Vec<_>>();
        let entries = roots
            .iter()
            .cloned()
            .zip(pack_ids)
            .map(|(root, pack_id)| {
                PackEntry::new_unmanaged(pack_id, root, Arc::clone(&data_budget))
            })
            .collect();
        Self::from_parts(roots, entries, image_budget, data_budget)
    }

    pub(crate) fn from_roots_with_data_budget(
        roots: impl IntoIterator<Item = impl Into<PathBuf>>,
        data_budget: Arc<ResourceDataGenerationBudget>,
    ) -> Self {
        Self::from_roots_with_budgets(
            roots,
            ResourceImageGenerationBudget::new(MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES),
            data_budget,
        )
    }

    fn from_parts(
        roots: Vec<PathBuf>,
        entries: Vec<PackEntry>,
        image_budget: Arc<ResourceImageGenerationBudget>,
        data_budget: Arc<ResourceDataGenerationBudget>,
    ) -> Self {
        Self {
            roots: Arc::new(roots),
            entries: Arc::new(entries),
            start: 0,
            image_budget,
            data_budget,
        }
    }

    pub fn prepare_selected_client_packs(packs: &[SelectedClientPack]) -> Result<Self> {
        Self::prepare_selected_client_packs_with_budgets(
            packs,
            ResourceImageGenerationBudget::new(MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES),
            ResourceDataGenerationBudget::new(MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES),
        )
    }

    pub(crate) fn prepare_selected_client_packs_with_budgets(
        packs: &[SelectedClientPack],
        image_budget: Arc<ResourceImageGenerationBudget>,
        data_budget: Arc<ResourceDataGenerationBudget>,
    ) -> Result<Self> {
        let roots = packs
            .iter()
            .map(|pack| pack.root.clone())
            .collect::<Vec<_>>();
        let entries = packs
            .iter()
            .map(|pack| {
                PackEntry::prepare(pack, Arc::clone(&data_budget))
                    .with_context(|| format!("prepare selected client pack {}", pack.id))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self::from_parts(roots, entries, image_budget, data_budget))
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.roots[self.start..]
    }

    pub(crate) fn without_first(&self) -> Self {
        Self {
            roots: Arc::clone(&self.roots),
            entries: Arc::clone(&self.entries),
            start: (self.start + 1).min(self.entries.len()),
            image_budget: Arc::clone(&self.image_budget),
            data_budget: Arc::clone(&self.data_budget),
        }
    }

    pub(crate) fn single_image_png_decode_budget(&self) -> PngDecodeBudget {
        PngDecodeBudget::single_image_in(Arc::clone(&self.image_budget))
    }

    pub(crate) fn atlas_png_decode_budget(&self) -> PngDecodeBudget {
        PngDecodeBudget::atlas_in(Arc::clone(&self.image_budget))
    }

    pub(crate) fn image_budget(&self) -> Arc<ResourceImageGenerationBudget> {
        Arc::clone(&self.image_budget)
    }

    pub(crate) fn data_budget(&self) -> Arc<ResourceDataGenerationBudget> {
        Arc::clone(&self.data_budget)
    }

    pub fn resource_image_memory_observation(&self) -> ResourceImageMemoryObservation {
        self.image_budget.observation()
    }

    pub fn resource_data_memory_observation(&self) -> ResourceDataMemoryObservation {
        self.data_budget.observation()
    }

    pub fn read_json_resource<T: DeserializeOwned>(
        &self,
        resource: &PackResource,
        label: &str,
    ) -> Result<T> {
        self.validate_resource(resource, label)?;
        self.read_json_file(&resource.path, label)
    }

    pub fn read_json_file<T: DeserializeOwned>(&self, path: &Path, label: &str) -> Result<T> {
        self.data_budget.read_json(path, label)
    }

    pub fn read_json_value_file(&self, path: &Path, label: &str) -> Result<serde_json::Value> {
        self.data_budget.read_json_value(path, label)
    }

    pub fn read_text_resource(&self, resource: &PackResource, label: &str) -> Result<String> {
        self.validate_resource(resource, label)?;
        self.read_text_file(&resource.path, label)
    }

    pub fn validate_resource(&self, resource: &PackResource, label: &str) -> Result<()> {
        if let Some(integrity) = &resource.integrity {
            integrity.verify(&resource.path, label)?;
        }
        Ok(())
    }

    pub fn validate_resource_metadata(&self, resource: &PackResource, label: &str) -> Result<()> {
        if let (Some(path), Some(integrity)) =
            (&resource.metadata_path, &resource.metadata_integrity)
        {
            integrity.verify(path, label)?;
        }
        Ok(())
    }

    pub fn root_resource_stack(&self, path: &str) -> Result<Vec<PackRootResource>> {
        validate_resource_path(path)?;
        Ok(self
            .pack_entries()
            .iter()
            .filter_map(|pack| pack.root_resource(path))
            .collect())
    }

    pub fn read_json_root_resource<T: DeserializeOwned>(
        &self,
        resource: &PackRootResource,
        label: &str,
    ) -> Result<T> {
        if let Some(integrity) = &resource.integrity {
            integrity.verify(&resource.path, label)?;
        }
        self.read_json_file(&resource.path, label)
    }

    pub fn vanilla_asset_index_observation(&self) -> Option<crate::VanillaAssetIndexObservation> {
        self.pack_entries().iter().find_map(|pack| {
            pack.indexed_assets
                .as_ref()
                .map(|index| index.observation())
        })
    }

    pub fn read_text_file(&self, path: &Path, label: &str) -> Result<String> {
        self.data_budget.read_text(path, label)
    }

    pub(crate) fn read_json_bytes_and_value(
        &self,
        path: &Path,
        label: &str,
    ) -> Result<(Vec<u8>, serde_json::Value)> {
        self.data_budget.read_json_bytes_and_value(path, label)
    }

    pub fn reserve_derived_resource_bytes(&self, logical_bytes: u64, label: &str) -> Result<()> {
        self.data_budget.reserve_derived(logical_bytes, label)
    }

    pub fn admit_resource_collection_count(
        &self,
        count: u64,
        maximum: u64,
        label: &str,
    ) -> Result<()> {
        if count > maximum {
            return Err(self.data_budget.reject(format!(
                "resource collection {label} has {count} entries, exceeding the {maximum} entry limit"
            )));
        }
        Ok(())
    }

    pub fn ensure_resource_data_admitted(&self) -> Result<()> {
        self.data_budget.ensure_admitted()
    }

    pub fn namespaces(&self) -> Result<Vec<String>> {
        self.namespaces_in("assets")
    }

    pub fn data_namespaces(&self) -> Result<Vec<String>> {
        self.namespaces_in("data")
    }

    fn namespaces_in(&self, domain: &str) -> Result<Vec<String>> {
        let mut namespaces = BTreeMap::new();
        let mut admission = ResourceListingAdmission::new(Arc::clone(&self.data_budget));
        for pack in self.pack_entries() {
            if pack.vanilla_namespaces {
                for namespace in ["minecraft", "realms"] {
                    if !namespaces.contains_key(namespace) {
                        admission.admit(namespace, &format!("{domain} namespaces"))?;
                    }
                    namespaces.insert(namespace.to_string(), namespace.to_string());
                }
            }
            for content_root in pack.content_roots(domain) {
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
                    if validate_resource_namespace(&namespace).is_err() {
                        continue;
                    }
                    if !namespaces.contains_key(&namespace) {
                        admission.admit(&namespace, &format!("{domain} namespaces"))?;
                    }
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
        let pack_entries = self.pack_entries();
        for (pack_index, pack) in pack_entries.iter().enumerate().rev() {
            if let Some(mut resource) = pack.get_resource(domain, location) {
                if let Some(metadata) =
                    metadata_for_resource(domain, location, pack_entries, pack_index)
                {
                    resource.metadata_path = Some(metadata.path);
                    resource.metadata_integrity = metadata.integrity;
                }
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

    pub(crate) fn filters_lower_priority_resource(&self, location: &ResourceLocation) -> bool {
        self.filters_lower_priority_resource_in("assets", location)
    }

    pub fn get_data_resource_stack(&self, location: &ResourceLocation) -> Vec<PackResource> {
        self.get_resource_stack_in("data", location)
    }

    fn filters_lower_priority_resource_in(
        &self,
        _domain: &str,
        location: &ResourceLocation,
    ) -> bool {
        self.pack_entries()
            .iter()
            .rev()
            .any(|pack| pack.is_filtered(location))
    }

    fn get_resource_stack_in(
        &self,
        domain: &str,
        location: &ResourceLocation,
    ) -> Vec<PackResource> {
        let metadata_location = metadata_location(location).ok();
        let mut resources = Vec::new();
        let mut filter_metadata = false;
        for pack in self.pack_entries().iter().rev() {
            if let Some(mut resource) = pack.get_resource(domain, location) {
                if !filter_metadata {
                    if let Some(metadata) = metadata_location
                        .as_ref()
                        .and_then(|location| pack.resource_file(domain, location))
                    {
                        resource.metadata_path = Some(metadata.path);
                        resource.metadata_integrity = metadata.integrity;
                    }
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

    pub fn list_resource_stacks(
        &self,
        path_prefix: &str,
        extension: &str,
    ) -> Result<BTreeMap<ResourceLocation, Vec<PackResource>>> {
        self.list_resource_stacks_in("assets", path_prefix, extension)
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
        let pack_entries = self.pack_entries();
        let mut resources: BTreeMap<ResourceLocation, (usize, PackResource)> = BTreeMap::new();
        let mut admission = ResourceListingAdmission::new(Arc::clone(&self.data_budget));
        for (pack_index, pack) in pack_entries.iter().enumerate() {
            if let Some(filter) = &pack.filter {
                resources.retain(|location, _| !filter.is_filtered(location));
            }
            for (location, resource) in
                pack.list_resources(domain, path_prefix, extension, &mut admission)?
            {
                resources.insert(location, (pack_index, resource));
            }
        }
        Ok(resources
            .into_iter()
            .map(|(location, (pack_index, mut resource))| {
                if let Some(metadata) =
                    metadata_for_resource(domain, &location, pack_entries, pack_index)
                {
                    resource.metadata_path = Some(metadata.path);
                    resource.metadata_integrity = metadata.integrity;
                }
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
        let mut admission = ResourceListingAdmission::new(Arc::clone(&self.data_budget));
        for pack in self.pack_entries() {
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
                            resource.metadata_integrity = None;
                        }
                    }
                    true
                });
            }
            for (location, mut resource) in
                pack.list_resources(domain, path_prefix, extension, &mut admission)?
            {
                if let Some(metadata) = metadata_location(&location)
                    .ok()
                    .and_then(|metadata_location| pack.resource_file(domain, &metadata_location))
                {
                    resource.metadata_path = Some(metadata.path);
                    resource.metadata_integrity = metadata.integrity;
                }
                resources.entry(location).or_default().push(resource);
            }
        }
        Ok(resources)
    }

    fn pack_entries(&self) -> &[PackEntry] {
        &self.entries[self.start..]
    }
}

impl Default for PackResourceStack {
    fn default() -> Self {
        Self::from_roots(std::iter::empty::<PathBuf>())
    }
}

fn collect_resources(
    pack_id: &str,
    namespace_dir: &Path,
    dir: &Path,
    namespace: &str,
    extension: &str,
    resources: &mut BTreeMap<ResourceLocation, PackResource>,
    admission: &mut ResourceListingAdmission,
    depth: u32,
) -> Result<()> {
    admission.validate_depth(depth, &format!("{namespace}:{extension}"))?;
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("read resource directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("read resource entry in {}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("read file type {}", path.display()))?;
        if file_type.is_dir() {
            collect_resources(
                pack_id,
                namespace_dir,
                &path,
                namespace,
                extension,
                resources,
                admission,
                depth + 1,
            )?;
            continue;
        }
        let resource_path = relative_resource_path(namespace_dir, &path)?;
        if !resource_path.ends_with(extension) {
            continue;
        }
        let Ok(location) = ResourceLocation::new(namespace, resource_path) else {
            continue;
        };
        admission.admit(&location.id(), &format!("{namespace}:{extension}"))?;
        resources.insert(
            location.clone(),
            PackResource {
                pack_id: pack_id.to_string(),
                location,
                path: path.clone(),
                metadata_path: None,
                integrity: None,
                metadata_integrity: None,
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
    pack_id: String,
    root: PathBuf,
    asset_content_roots: Vec<PathBuf>,
    data_content_roots: Vec<PathBuf>,
    filter: Option<PackFilter>,
    indexed_assets: Option<Arc<VanillaAssetIndex>>,
    vanilla_namespaces: bool,
}

struct ResourceFile {
    path: PathBuf,
    integrity: Option<ResourceIntegrity>,
}

impl PackEntry {
    fn new_unmanaged(
        pack_id: String,
        root: PathBuf,
        data_budget: Arc<ResourceDataGenerationBudget>,
    ) -> Self {
        let metadata = read_pack_resource_metadata(&root, &data_budget)
            .ok()
            .flatten();
        Self::from_metadata(pack_id, root, metadata, false, None, false)
            .expect("lenient unmanaged pack metadata cannot fail")
    }

    fn prepare(
        pack: &SelectedClientPack,
        data_budget: Arc<ResourceDataGenerationBudget>,
    ) -> Result<Self> {
        let metadata_required = matches!(
            pack.source,
            ClientPackSource::Startup { .. } | ClientPackSource::Server { .. }
        );
        let metadata = if matches!(pack.source, ClientPackSource::Unmanaged) {
            read_pack_resource_metadata(&pack.root, &data_budget)
                .ok()
                .flatten()
        } else {
            pack.metadata
                .as_ref()
                .map(|metadata| {
                    serde_json::from_value::<RawPackMetadata>(metadata.document().clone())
                        .context("decode client pack resource metadata sections")
                })
                .transpose()?
        };
        if metadata_required && metadata.is_none() {
            bail!("missing pack.mcmeta in {}", pack.root.display());
        }
        if let Some(index) = &pack.indexed_vanilla_assets {
            data_budget.reserve_indexed_assets_once(index.observation().retained_logical_bytes)?;
        }
        Self::from_metadata(
            pack.id.clone(),
            pack.root.clone(),
            metadata,
            !matches!(pack.source, ClientPackSource::Unmanaged),
            pack.indexed_vanilla_assets.as_ref().map(Arc::clone),
            matches!(pack.source, ClientPackSource::Vanilla),
        )
    }

    fn from_metadata(
        pack_id: String,
        root: PathBuf,
        metadata: Option<RawPackMetadata>,
        strict: bool,
        indexed_assets: Option<Arc<VanillaAssetIndex>>,
        vanilla_namespaces: bool,
    ) -> Result<Self> {
        let filter = match metadata
            .as_ref()
            .and_then(|metadata| metadata.filter.as_ref())
        {
            Some(filter) => PackFilter::prepare(filter, strict)?,
            None => None,
        };
        let asset_content_roots = content_roots(
            &root,
            metadata.as_ref(),
            CLIENT_RESOURCE_PACK_FORMAT,
            strict,
        )?;
        let data_content_roots =
            content_roots(&root, metadata.as_ref(), SERVER_DATA_PACK_FORMAT, strict)?;
        Ok(Self {
            pack_id,
            root,
            asset_content_roots,
            data_content_roots,
            filter,
            indexed_assets,
            vanilla_namespaces,
        })
    }

    fn content_roots(&self, domain: &str) -> &[PathBuf] {
        match domain {
            "data" => &self.data_content_roots,
            _ => &self.asset_content_roots,
        }
    }

    fn is_filtered(&self, location: &ResourceLocation) -> bool {
        self.filter
            .as_ref()
            .is_some_and(|filter| filter.is_filtered(location))
    }

    fn get_resource(&self, domain: &str, location: &ResourceLocation) -> Option<PackResource> {
        self.resource_file(domain, location)
            .map(|file| PackResource {
                pack_id: self.pack_id.clone(),
                location: location.clone(),
                path: file.path,
                metadata_path: None,
                integrity: file.integrity,
                metadata_integrity: None,
            })
    }

    fn resource_file(&self, domain: &str, location: &ResourceLocation) -> Option<ResourceFile> {
        if self.vanilla_namespaces && !matches!(location.namespace(), "minecraft" | "realms") {
            return None;
        }
        if domain == "assets" {
            if let Some(indexed) = self
                .indexed_assets
                .as_ref()
                .and_then(|index| index.resource(location))
            {
                return Some(resource_file_from_indexed(indexed));
            }
        }
        self.content_roots(domain).iter().rev().find_map(|root| {
            let path = resource_path_in(root, domain, location);
            path.is_file().then_some(ResourceFile {
                path,
                integrity: None,
            })
        })
    }

    fn root_resource(&self, path: &str) -> Option<PackRootResource> {
        if let Some(indexed) = self
            .indexed_assets
            .as_ref()
            .and_then(|index| index.root_resource(path))
        {
            return Some(PackRootResource {
                pack_id: self.pack_id.clone(),
                path: indexed.path,
                integrity: Some(indexed.integrity),
            });
        }
        let path = self.root.join(path);
        path.is_file().then_some(PackRootResource {
            pack_id: self.pack_id.clone(),
            path,
            integrity: None,
        })
    }

    fn list_resources(
        &self,
        domain: &str,
        path_prefix: &str,
        extension: &str,
        admission: &mut ResourceListingAdmission,
    ) -> Result<BTreeMap<ResourceLocation, PackResource>> {
        let mut resources = BTreeMap::new();
        for root in self.content_roots(domain) {
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
                if validate_resource_namespace(&namespace).is_err() {
                    continue;
                }
                if self.vanilla_namespaces && !matches!(namespace.as_str(), "minecraft" | "realms")
                {
                    continue;
                }
                let list_root = namespace_dir.join(path_prefix);
                if !list_root.is_dir() {
                    continue;
                }
                collect_resources(
                    &self.pack_id,
                    &namespace_dir,
                    &list_root,
                    &namespace,
                    extension,
                    &mut resources,
                    admission,
                    1,
                )?;
            }
        }
        if domain == "assets" {
            if let Some(index) = &self.indexed_assets {
                for (location, indexed) in index.list_resources(path_prefix, extension) {
                    admission.admit(&location.id(), "Vanilla indexed resources")?;
                    resources.insert(
                        location.clone(),
                        PackResource {
                            pack_id: self.pack_id.clone(),
                            location,
                            path: indexed.path,
                            metadata_path: None,
                            integrity: Some(indexed.integrity),
                            metadata_integrity: None,
                        },
                    );
                }
            }
        }
        Ok(resources)
    }
}

fn resource_file_from_indexed(indexed: IndexedAsset) -> ResourceFile {
    ResourceFile {
        path: indexed.path,
        integrity: Some(indexed.integrity),
    }
}

fn metadata_location(location: &ResourceLocation) -> Result<ResourceLocation> {
    location.with_suffix(".mcmeta")
}

fn metadata_for_resource(
    domain: &str,
    location: &ResourceLocation,
    pack_entries: &[PackEntry],
    resource_pack_index: usize,
) -> Option<ResourceFile> {
    let metadata_location = metadata_location(location).ok()?;
    for pack in pack_entries[resource_pack_index..].iter().rev() {
        if let Some(file) = pack.resource_file(domain, &metadata_location) {
            return Some(file);
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

#[derive(Debug)]
struct PackFilter {
    blocks: Vec<ResourceFilterPattern>,
}

impl PackFilter {
    fn prepare(raw: &RawResourceFilterSection, strict: bool) -> Result<Option<Self>> {
        let blocks = raw
            .block
            .iter()
            .map(RawResourceFilterPattern::to_pattern)
            .collect::<Result<Vec<_>>>();
        match blocks {
            Ok(blocks) => Ok(Some(Self { blocks })),
            Err(error) if strict => Err(error).context("compile client pack resource filter"),
            Err(_) => Ok(None),
        }
    }

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
    fn applies_to(&self, current: PackFormat, strict: bool) -> Result<bool> {
        if !is_valid_overlay_directory(&self.directory) {
            if strict {
                bail!("invalid client pack overlay directory {:?}", self.directory);
            }
            return Ok(false);
        }
        if self.min_format.is_some() != self.max_format.is_some() {
            if strict {
                bail!("client pack overlay must declare both min_format and max_format");
            }
            return Ok(false);
        }
        if let (Some(min), Some(max)) = (self.min_format, self.max_format) {
            let min = min.into_pack_format(0);
            let max = max.into_pack_format(u32::MAX);
            if min > max {
                if strict {
                    bail!("client pack overlay min_format exceeds max_format");
                }
                return Ok(false);
            }
            return Ok(current >= min && current <= max);
        }
        let Some(formats) = self.formats else {
            if strict {
                bail!("client pack overlay is missing its format range");
            }
            return Ok(false);
        };
        let (min, max) = formats.bounds();
        if min > max {
            if strict {
                bail!("client pack overlay format minimum exceeds maximum");
            }
            return Ok(false);
        }
        Ok(formats.contains(current.major))
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
    fn bounds(self) -> (u32, u32) {
        match self {
            Self::Single(value) => (value, value),
            Self::Pair([min, max]) => (min, max),
            Self::Object {
                min_inclusive,
                max_inclusive,
            } => (min_inclusive, max_inclusive),
        }
    }

    fn contains(self, value: u32) -> bool {
        let (min, max) = self.bounds();
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
    fn to_pattern(&self) -> Result<ResourceFilterPattern> {
        Ok(ResourceFilterPattern {
            namespace: self
                .namespace
                .as_ref()
                .map(|pattern| Regex::new(pattern))
                .transpose()?,
            path: self
                .path
                .as_ref()
                .map(|pattern| Regex::new(pattern))
                .transpose()?,
        })
    }
}

fn read_pack_resource_metadata(
    root: &Path,
    data_budget: &ResourceDataGenerationBudget,
) -> Result<Option<RawPackMetadata>> {
    let metadata_path = root.join("pack.mcmeta");
    if !metadata_path.is_file() {
        return Ok(None);
    }
    data_budget
        .read_json(&metadata_path, "client pack metadata")
        .with_context(|| format!("parse client pack metadata {}", metadata_path.display()))
        .map(Some)
}

fn content_roots(
    root: &Path,
    metadata: Option<&RawPackMetadata>,
    current_format: PackFormat,
    strict: bool,
) -> Result<Vec<PathBuf>> {
    let mut roots = vec![root.to_path_buf()];
    let overlays = metadata
        .and_then(|metadata| metadata.overlays.as_ref())
        .map(|overlays| overlays.entries.as_slice())
        .unwrap_or_default();
    for overlay in overlays {
        if overlay.applies_to(current_format, strict)? {
            roots.push(root.join(&overlay.directory));
        }
    }
    Ok(roots)
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
    use crate::resource_budget::MAX_RESOURCE_DIRECTORY_DEPTH;
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
    fn recursive_resource_listing_rejects_more_than_sixty_four_levels() {
        let root = unique_temp_dir("resource-list-depth");
        let mut dir = root.join("assets/minecraft/test");
        for _ in 0..MAX_RESOURCE_DIRECTORY_DEPTH {
            dir.push("a");
        }
        write_file(&dir.join("value.json"), b"{}");
        let data_budget =
            ResourceDataGenerationBudget::new(MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES);
        let stack = PackResourceStack::from_roots_with_budgets(
            [root.clone()],
            ResourceImageGenerationBudget::new(MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES),
            Arc::clone(&data_budget),
        );

        let error = stack
            .list_resources("test", ".json")
            .unwrap_err()
            .to_string();
        assert!(error.contains("64 directory level limit"), "{error}");
        assert!(stack.ensure_resource_data_admitted().is_err());
        std::fs::remove_dir_all(root).unwrap();
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
    fn resource_stack_skips_invalid_namespace_directories_when_listing() {
        let root = unique_temp_dir("resource-invalid-namespace");
        let pack = root.join("pack");
        write_file(
            &pack
                .join("assets")
                .join("BadNamespace")
                .join("textures")
                .join("block")
                .join("bad.png"),
            b"bad",
        );
        write_file(
            &pack
                .join("assets")
                .join("minecraft")
                .join("textures")
                .join("block")
                .join("stone.png"),
            b"stone",
        );

        let stack = PackResourceStack::from_roots([pack]);
        let listed = stack
            .list_resources("textures/block", ".png")
            .unwrap()
            .into_iter()
            .map(|resource| resource.location.id())
            .collect::<Vec<_>>();

        assert_eq!(stack.namespaces().unwrap(), vec!["minecraft"]);
        assert_eq!(listed, vec!["minecraft:textures/block/stone.png"]);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resource_stack_skips_invalid_resource_paths_when_listing() {
        let root = unique_temp_dir("resource-invalid-path");
        let pack = root.join("pack");
        write_file(
            &pack
                .join("assets")
                .join("minecraft")
                .join("textures")
                .join("block")
                .join("bad name.png"),
            b"bad",
        );
        write_file(
            &pack
                .join("assets")
                .join("minecraft")
                .join("textures")
                .join("block")
                .join("stone.png"),
            b"stone",
        );

        let stack = PackResourceStack::from_roots([pack]);
        let listed = stack
            .list_resources("textures/block", ".png")
            .unwrap()
            .into_iter()
            .map(|resource| resource.location.id())
            .collect::<Vec<_>>();

        assert_eq!(listed, vec!["minecraft:textures/block/stone.png"]);

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
