use std::{
    fs::File,
    io::Read,
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};

use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;
use serde_json::Value;

pub(crate) const MAX_RESOURCE_JSON_FILE_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_RESOURCE_TEXT_FILE_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_RESOURCE_JSON_DOCUMENT_NODES: u64 = 262_144;
pub(crate) const MAX_RESOURCE_JSON_DOCUMENT_STRING_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_RESOURCE_LIST_RESULTS: u64 = 65_536;
pub(crate) const MAX_RESOURCE_DIRECTORY_DEPTH: u32 = 64;
pub(crate) const RESOURCE_JSON_NODE_LOGICAL_BYTES: u64 = 64;
pub(crate) const RESOURCE_LIST_ENTRY_LOGICAL_BYTES: u64 = 256;
pub const MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_RESOURCE_DATA_PROCESS_LOGICAL_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResourceDataMemoryObservation {
    pub max_logical_bytes: u64,
    pub charged_logical_bytes: u64,
    pub peak_charged_logical_bytes: u64,
    pub json_source_bytes: u64,
    pub json_documents: u64,
    pub json_nodes: u64,
    pub json_string_bytes: u64,
    pub text_source_bytes: u64,
    pub text_documents: u64,
    pub listing_entries: u64,
    pub derived_logical_bytes: u64,
    pub rejections: u64,
}

#[derive(Debug)]
pub(crate) struct ResourceDataGenerationBudget {
    max_logical_bytes: u64,
    charged_logical_bytes: AtomicU64,
    peak_charged_logical_bytes: AtomicU64,
    json_source_bytes: AtomicU64,
    json_documents: AtomicU64,
    json_nodes: AtomicU64,
    json_string_bytes: AtomicU64,
    text_source_bytes: AtomicU64,
    text_documents: AtomicU64,
    listing_entries: AtomicU64,
    derived_logical_bytes: AtomicU64,
    indexed_assets_charged: AtomicBool,
    rejections: AtomicU64,
}

impl ResourceDataGenerationBudget {
    pub(crate) fn new(max_logical_bytes: u64) -> Arc<Self> {
        Arc::new(Self {
            max_logical_bytes,
            charged_logical_bytes: AtomicU64::new(0),
            peak_charged_logical_bytes: AtomicU64::new(0),
            json_source_bytes: AtomicU64::new(0),
            json_documents: AtomicU64::new(0),
            json_nodes: AtomicU64::new(0),
            json_string_bytes: AtomicU64::new(0),
            text_source_bytes: AtomicU64::new(0),
            text_documents: AtomicU64::new(0),
            listing_entries: AtomicU64::new(0),
            derived_logical_bytes: AtomicU64::new(0),
            indexed_assets_charged: AtomicBool::new(false),
            rejections: AtomicU64::new(0),
        })
    }

    pub(crate) fn reject(&self, message: impl Into<String>) -> anyhow::Error {
        self.rejections.fetch_add(1, Ordering::Relaxed);
        anyhow::anyhow!(message.into())
    }

    fn reserve(&self, logical_bytes: u64, label: &str) -> Result<()> {
        let next = self
            .charged_logical_bytes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |charged| {
                charged
                    .checked_add(logical_bytes)
                    .filter(|next| *next <= self.max_logical_bytes)
            })
            .map(|previous| previous + logical_bytes)
            .map_err(|charged| {
                self.reject(format!(
                    "resource data generation would use more than {} logical bytes while loading {label} (currently charged {charged}, requested {logical_bytes})",
                    self.max_logical_bytes
                ))
            })?;
        self.peak_charged_logical_bytes
            .fetch_max(next, Ordering::Relaxed);
        Ok(())
    }

    pub(crate) fn read_json<T: DeserializeOwned>(&self, path: &Path, label: &str) -> Result<T> {
        let value = self.read_json_value(path, label)?;
        serde_json::from_value(value)
            .with_context(|| format!("decode JSON {label} {}", path.display()))
    }

    pub(crate) fn read_json_value(&self, path: &Path, label: &str) -> Result<Value> {
        self.read_json_bytes_and_value(path, label)
            .map(|(_, value)| value)
    }

    pub(crate) fn read_json_bytes_and_value(
        &self,
        path: &Path,
        label: &str,
    ) -> Result<(Vec<u8>, Value)> {
        let bytes = self.read_json_source(path, label)?;
        let value: Value = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse JSON {label} {}", path.display()))?;
        self.admit_json_tree(&value, &format!("{label} {}", path.display()))?;
        Ok((bytes, value))
    }

    pub(crate) fn admit_preparsed_json(
        &self,
        source_bytes: u64,
        value: &Value,
        label: &str,
    ) -> Result<()> {
        if source_bytes > MAX_RESOURCE_JSON_FILE_BYTES {
            return Err(self.reject(format!(
                "JSON {label} is {source_bytes} bytes, exceeding the {} byte limit",
                MAX_RESOURCE_JSON_FILE_BYTES
            )));
        }
        self.reserve(source_bytes, label)?;
        self.json_source_bytes
            .fetch_add(source_bytes, Ordering::Relaxed);
        self.admit_json_tree(value, label)
    }

    fn admit_json_tree(&self, value: &Value, label: &str) -> Result<()> {
        let (nodes, string_bytes) = json_tree_size(&value)?;
        if nodes > MAX_RESOURCE_JSON_DOCUMENT_NODES {
            return Err(self.reject(format!(
                "JSON {label} has {nodes} value nodes, exceeding the {} node limit",
                MAX_RESOURCE_JSON_DOCUMENT_NODES
            )));
        }
        if string_bytes > MAX_RESOURCE_JSON_DOCUMENT_STRING_BYTES {
            return Err(self.reject(format!(
                "JSON {label} has {string_bytes} decoded UTF-8 key/value bytes, exceeding the {} byte limit",
                MAX_RESOURCE_JSON_DOCUMENT_STRING_BYTES
            )));
        }
        let tree_bytes = nodes
            .checked_mul(RESOURCE_JSON_NODE_LOGICAL_BYTES)
            .and_then(|bytes| bytes.checked_add(string_bytes))
            .ok_or_else(|| self.reject(format!("JSON {label} logical tree size overflow")))?;
        self.reserve(tree_bytes, label)?;
        self.json_documents.fetch_add(1, Ordering::Relaxed);
        self.json_nodes.fetch_add(nodes, Ordering::Relaxed);
        self.json_string_bytes
            .fetch_add(string_bytes, Ordering::Relaxed);
        Ok(())
    }

    fn read_json_source(&self, path: &Path, label: &str) -> Result<Vec<u8>> {
        let file =
            File::open(path).with_context(|| format!("open JSON {label} {}", path.display()))?;
        let file_bytes = file
            .metadata()
            .with_context(|| format!("read JSON metadata for {label} {}", path.display()))?
            .len();
        if file_bytes > MAX_RESOURCE_JSON_FILE_BYTES {
            return Err(self.reject(format!(
                "JSON {label} {} is {file_bytes} bytes, exceeding the {} byte limit",
                path.display(),
                MAX_RESOURCE_JSON_FILE_BYTES
            )));
        }
        self.reserve(file_bytes, label)?;
        self.json_source_bytes
            .fetch_add(file_bytes, Ordering::Relaxed);

        let capacity = usize::try_from(file_bytes)
            .with_context(|| format!("JSON {label} file size does not fit usize"))?;
        let mut bytes = Vec::with_capacity(capacity);
        file.take(MAX_RESOURCE_JSON_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .with_context(|| format!("read JSON {label} {}", path.display()))?;
        if bytes.len() as u64 > MAX_RESOURCE_JSON_FILE_BYTES {
            return Err(self.reject(format!(
                "JSON {label} {} grew beyond the {} byte limit while being read",
                path.display(),
                MAX_RESOURCE_JSON_FILE_BYTES
            )));
        }
        let actual_bytes = u64::try_from(bytes.len())
            .with_context(|| format!("JSON {label} byte length does not fit u64"))?;
        if actual_bytes > file_bytes {
            let growth = actual_bytes - file_bytes;
            self.reserve(growth, label)?;
            self.json_source_bytes.fetch_add(growth, Ordering::Relaxed);
        }
        Ok(bytes)
    }

    pub(crate) fn read_text(&self, path: &Path, label: &str) -> Result<String> {
        let file =
            File::open(path).with_context(|| format!("open text {label} {}", path.display()))?;
        let file_bytes = file
            .metadata()
            .with_context(|| format!("read text metadata for {label} {}", path.display()))?
            .len();
        if file_bytes > MAX_RESOURCE_TEXT_FILE_BYTES {
            return Err(self.reject(format!(
                "text {label} {} is {file_bytes} bytes, exceeding the {} byte limit",
                path.display(),
                MAX_RESOURCE_TEXT_FILE_BYTES
            )));
        }
        self.reserve(file_bytes, label)?;
        self.text_source_bytes
            .fetch_add(file_bytes, Ordering::Relaxed);

        let capacity = usize::try_from(file_bytes)
            .with_context(|| format!("text {label} file size does not fit usize"))?;
        let mut bytes = Vec::with_capacity(capacity);
        file.take(MAX_RESOURCE_TEXT_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .with_context(|| format!("read text {label} {}", path.display()))?;
        if bytes.len() as u64 > MAX_RESOURCE_TEXT_FILE_BYTES {
            return Err(self.reject(format!(
                "text {label} {} grew beyond the {} byte limit while being read",
                path.display(),
                MAX_RESOURCE_TEXT_FILE_BYTES
            )));
        }
        let actual_bytes = u64::try_from(bytes.len())
            .with_context(|| format!("text {label} byte length does not fit u64"))?;
        if actual_bytes > file_bytes {
            let growth = actual_bytes - file_bytes;
            self.reserve(growth, label)?;
            self.text_source_bytes.fetch_add(growth, Ordering::Relaxed);
        }
        let text = String::from_utf8(bytes)
            .with_context(|| format!("decode UTF-8 text {label} {}", path.display()))?;
        self.text_documents.fetch_add(1, Ordering::Relaxed);
        Ok(text)
    }

    pub(crate) fn reserve_listing_entry(&self, path_bytes: u64, label: &str) -> Result<()> {
        let logical_bytes = RESOURCE_LIST_ENTRY_LOGICAL_BYTES
            .checked_add(path_bytes)
            .ok_or_else(|| self.reject("resource listing logical size overflow"))?;
        self.reserve(logical_bytes, label)?;
        self.listing_entries.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub(crate) fn reserve_derived(&self, logical_bytes: u64, label: &str) -> Result<()> {
        self.reserve(logical_bytes, label)?;
        self.derived_logical_bytes
            .fetch_add(logical_bytes, Ordering::Relaxed);
        Ok(())
    }

    pub(crate) fn reserve_indexed_assets_once(&self, logical_bytes: u64) -> Result<()> {
        if self.indexed_assets_charged.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        if let Err(error) = self.reserve_derived(logical_bytes, "retained Vanilla asset index") {
            self.indexed_assets_charged.store(false, Ordering::Release);
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn ensure_admitted(&self) -> Result<()> {
        let rejections = self.rejections.load(Ordering::Acquire);
        if rejections != 0 {
            bail!(
                "resource data generation was rejected by {rejections} bounded admission check(s)"
            );
        }
        Ok(())
    }

    pub(crate) fn observation(&self) -> ResourceDataMemoryObservation {
        ResourceDataMemoryObservation {
            max_logical_bytes: self.max_logical_bytes,
            charged_logical_bytes: self.charged_logical_bytes.load(Ordering::Acquire),
            peak_charged_logical_bytes: self.peak_charged_logical_bytes.load(Ordering::Relaxed),
            json_source_bytes: self.json_source_bytes.load(Ordering::Relaxed),
            json_documents: self.json_documents.load(Ordering::Relaxed),
            json_nodes: self.json_nodes.load(Ordering::Relaxed),
            json_string_bytes: self.json_string_bytes.load(Ordering::Relaxed),
            text_source_bytes: self.text_source_bytes.load(Ordering::Relaxed),
            text_documents: self.text_documents.load(Ordering::Relaxed),
            listing_entries: self.listing_entries.load(Ordering::Relaxed),
            derived_logical_bytes: self.derived_logical_bytes.load(Ordering::Relaxed),
            rejections: self.rejections.load(Ordering::Acquire),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ResourceListingAdmission {
    results: u64,
    generation: Arc<ResourceDataGenerationBudget>,
}

impl ResourceListingAdmission {
    pub(crate) fn new(generation: Arc<ResourceDataGenerationBudget>) -> Self {
        Self {
            results: 0,
            generation,
        }
    }

    pub(crate) fn admit(&mut self, path: &str, label: &str) -> Result<()> {
        let next = self
            .results
            .checked_add(1)
            .ok_or_else(|| self.generation.reject("resource listing count overflow"))?;
        if next > MAX_RESOURCE_LIST_RESULTS {
            return Err(self.generation.reject(format!(
                "resource listing {label} has more than {MAX_RESOURCE_LIST_RESULTS} results"
            )));
        }
        let path_bytes = u64::try_from(path.len()).map_err(|_| {
            self.generation
                .reject("resource listing path length overflow")
        })?;
        self.generation.reserve_listing_entry(path_bytes, label)?;
        self.results = next;
        Ok(())
    }

    pub(crate) fn validate_depth(&self, depth: u32, label: &str) -> Result<()> {
        if depth > MAX_RESOURCE_DIRECTORY_DEPTH {
            return Err(self.generation.reject(format!(
                "resource listing {label} exceeds the {MAX_RESOURCE_DIRECTORY_DEPTH} directory level limit"
            )));
        }
        Ok(())
    }
}

fn json_tree_size(root: &Value) -> Result<(u64, u64)> {
    let mut nodes = 0_u64;
    let mut string_bytes = 0_u64;
    let mut pending = vec![root];
    while let Some(value) = pending.pop() {
        nodes = nodes
            .checked_add(1)
            .context("JSON value node count overflow")?;
        match value {
            Value::String(value) => {
                string_bytes = string_bytes
                    .checked_add(u64::try_from(value.len()).context("JSON string size overflow")?)
                    .context("JSON decoded string byte count overflow")?;
            }
            Value::Array(values) => pending.extend(values),
            Value::Object(values) => {
                for (key, value) in values {
                    string_bytes = string_bytes
                        .checked_add(u64::try_from(key.len()).context("JSON key size overflow")?)
                        .context("JSON decoded key byte count overflow")?;
                    pending.push(value);
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }
    Ok((nodes, string_bytes))
}

pub(crate) fn read_standalone_json<T: DeserializeOwned>(path: &Path, label: &str) -> Result<T> {
    ResourceDataGenerationBudget::new(MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES)
        .read_json(path, label)
}

pub(crate) fn read_standalone_json_bytes_and_value(
    path: &Path,
    label: &str,
) -> Result<(Vec<u8>, Value)> {
    ResourceDataGenerationBudget::new(MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES)
        .read_json_bytes_and_value(path, label)
}

#[cfg(test)]
mod tests {
    use std::{
        fs::OpenOptions,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn rejects_oversized_json_before_payload_allocation_and_sticks() {
        let path = temp_path("oversized.json");
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .unwrap();
        file.set_len(MAX_RESOURCE_JSON_FILE_BYTES + 1).unwrap();
        let budget = ResourceDataGenerationBudget::new(u64::MAX);

        let error = budget
            .read_json_value(&path, "oversized fixture")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("exceeding the 16777216 byte limit"),
            "{error}"
        );
        assert_eq!(budget.observation().charged_logical_bytes, 0);
        assert!(budget.ensure_admitted().is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn clone_scopes_share_generation_charge() {
        let path = temp_path("shared.json");
        std::fs::write(&path, br#"{"key":"value"}"#).unwrap();
        let budget = ResourceDataGenerationBudget::new(200);

        let _: Value = budget.read_json(&path, "first").unwrap();
        let error = budget
            .read_json_value(&path, "second")
            .unwrap_err()
            .to_string();
        assert!(error.contains("resource data generation"), "{error}");
        assert!(budget.ensure_admitted().is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn observes_json_tree_and_listing_components() {
        let path = temp_path("observed.json");
        std::fs::write(&path, br#"{"key":["value",1]}"#).unwrap();
        let budget = ResourceDataGenerationBudget::new(4096);
        let _: Value = budget.read_json(&path, "observed").unwrap();
        budget
            .reserve_listing_entry(12, "observed listing")
            .unwrap();

        let observation = budget.observation();
        assert_eq!(observation.json_source_bytes, 19);
        assert_eq!(observation.json_documents, 1);
        assert_eq!(observation.json_nodes, 4);
        assert_eq!(observation.json_string_bytes, 8);
        assert_eq!(observation.listing_entries, 1);
        assert_eq!(observation.rejections, 0);
        std::fs::remove_file(path).unwrap();
    }

    fn temp_path(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-resource-data-{nonce}-{label}"))
    }

    #[test]
    fn listing_depth_and_result_rejections_are_sticky() {
        let budget = ResourceDataGenerationBudget::new(u64::MAX);
        let admission = ResourceListingAdmission::new(Arc::clone(&budget));
        admission
            .validate_depth(MAX_RESOURCE_DIRECTORY_DEPTH, "boundary")
            .unwrap();
        assert!(admission
            .validate_depth(MAX_RESOURCE_DIRECTORY_DEPTH + 1, "too deep")
            .is_err());

        let budget = ResourceDataGenerationBudget::new(u64::MAX);
        let mut admission = ResourceListingAdmission {
            results: MAX_RESOURCE_LIST_RESULTS,
            generation: Arc::clone(&budget),
        };
        assert!(admission.admit("overflow", "too wide").is_err());
        assert_eq!(budget.observation().rejections, 1);
        assert!(budget.ensure_admitted().is_err());
    }

    #[test]
    fn production_resource_json_consumers_do_not_bypass_bounded_file_reader() {
        let sources = [
            ("atlas_sources.rs", include_str!("atlas_sources.rs")),
            ("block_models.rs", include_str!("block_models.rs")),
            ("colors.rs", include_str!("colors.rs")),
            ("equipment_assets.rs", include_str!("equipment_assets.rs")),
            ("item_models.rs", include_str!("item_models.rs")),
            ("item_registry.rs", include_str!("item_registry.rs")),
            ("known_packs.rs", include_str!("known_packs.rs")),
            ("language.rs", include_str!("language.rs")),
            ("metadata.rs", include_str!("metadata.rs")),
            (
                "particle_definitions.rs",
                include_str!("particle_definitions.rs"),
            ),
            ("roots.rs", include_str!("roots.rs")),
            ("server_pack.rs", include_str!("server_pack.rs")),
            ("sounds.rs", include_str!("sounds.rs")),
            ("sprites.rs", include_str!("sprites.rs")),
            ("tags.rs", include_str!("tags.rs")),
            ("waypoint_styles.rs", include_str!("waypoint_styles.rs")),
            (
                "bbb-item-model/font/providers.rs",
                include_str!("../../bbb-item-model/src/font/providers.rs"),
            ),
        ];
        for (name, source) in sources {
            let production = source.split("#[cfg(test)]").next().unwrap_or(source);
            assert!(
                !production.contains("std::fs::read(")
                    && !production.contains("std::fs::read_to_string(")
                    && !production.contains("fs::read(")
                    && !production.contains("fs::read_to_string("),
                "{name} bypasses the generation-owned bounded JSON reader"
            );
        }
    }
}
