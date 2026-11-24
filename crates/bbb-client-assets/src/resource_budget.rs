use std::{
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

use crate::ResourceJsonValue;

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
        self.reserve_with_policy(logical_bytes, label, true)
    }

    fn reserve_optional(&self, logical_bytes: u64, label: &str) -> Result<()> {
        self.reserve_with_policy(logical_bytes, label, false)
    }

    fn reserve_with_policy(
        &self,
        logical_bytes: u64,
        label: &str,
        sticky_rejection: bool,
    ) -> Result<()> {
        let next = self
            .charged_logical_bytes
            .try_update(Ordering::AcqRel, Ordering::Acquire, |charged| {
                charged
                    .checked_add(logical_bytes)
                    .filter(|next| *next <= self.max_logical_bytes)
            })
            .map(|previous| previous + logical_bytes)
            .map_err(|charged| {
                let prefix = if sticky_rejection {
                    "resource data generation"
                } else {
                    "optional resource data"
                };
                let message = format!(
                    "{prefix} would use more than {} logical bytes while loading {label} (currently charged {charged}, requested {logical_bytes})",
                    self.max_logical_bytes
                );
                if sticky_rejection {
                    self.reject(message)
                } else {
                    anyhow::anyhow!(message)
                }
            })?;
        self.peak_charged_logical_bytes
            .fetch_max(next, Ordering::Relaxed);
        Ok(())
    }

    pub(crate) fn read_json<T: DeserializeOwned>(&self, path: &Path, label: &str) -> Result<T> {
        let bytes = self.read_json_source(path, label)?;
        self.deserialize_json_slice(&bytes, path, label)
    }

    fn deserialize_json_slice<T: DeserializeOwned>(
        &self,
        bytes: &[u8],
        display_path: &Path,
        label: &str,
    ) -> Result<T> {
        let full_label = format!("{label} {}", display_path.display());
        let mut counts = JsonStreamCounts::default();
        let mut inner = serde_json::Deserializer::from_slice(bytes);
        let result = T::deserialize(CountingDeserializer {
            inner: &mut inner,
            counts: &mut counts,
        });
        if counts.limit_hit.is_some() {
            self.admit_json_counts(counts.nodes, counts.string_bytes, &full_label)?;
        }
        let value =
            result.with_context(|| format!("decode JSON {label} {}", display_path.display()))?;
        inner
            .end()
            .with_context(|| format!("parse JSON {label} {}", display_path.display()))?;
        self.admit_json_counts(counts.nodes, counts.string_bytes, &full_label)?;
        Ok(value)
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

    pub(crate) fn read_resource_json(&self, path: &Path, label: &str) -> Result<ResourceJsonValue> {
        self.read_resource_json_bytes_and_value(path, label)
            .map(|(_, value)| value)
    }

    pub(crate) fn read_resource_json_bytes_and_value(
        &self,
        path: &Path,
        label: &str,
    ) -> Result<(Vec<u8>, ResourceJsonValue)> {
        let bytes = self.read_json_source(path, label)?;
        let value = ResourceJsonValue::from_slice(&bytes)
            .with_context(|| format!("parse JSON {label} {}", path.display()))?;
        let (nodes, string_bytes) = value.tree_size()?;
        self.admit_json_counts(nodes, string_bytes, &format!("{label} {}", path.display()))?;
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
        let (nodes, string_bytes) = json_tree_size(value)?;
        self.admit_json_counts(nodes, string_bytes, label)
    }

    fn admit_json_counts(&self, nodes: u64, string_bytes: u64, label: &str) -> Result<()> {
        if nodes > MAX_RESOURCE_JSON_DOCUMENT_NODES {
            return Err(self.reject(format!(
                "JSON {label} has {nodes} value nodes, exceeding the {} node limit",
                MAX_RESOURCE_JSON_DOCUMENT_NODES
            )));
        }
        if string_bytes > MAX_RESOURCE_JSON_DOCUMENT_STRING_BYTES {
            return Err(self.reject(format!(
                "JSON {label} has {string_bytes} decoded key/value bytes, exceeding the {} byte limit",
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

    pub(crate) fn read_json_from_reader<T: DeserializeOwned>(
        &self,
        reader: impl Read,
        file_bytes: u64,
        display_path: &Path,
        label: &str,
    ) -> Result<T> {
        let bytes = self.read_json_source_reader(reader, file_bytes, display_path, label)?;
        self.deserialize_json_slice(&bytes, display_path, label)
    }

    pub(crate) fn read_json_bytes_and_value_from_reader(
        &self,
        reader: impl Read,
        file_bytes: u64,
        display_path: &Path,
        label: &str,
    ) -> Result<(Vec<u8>, Value)> {
        let bytes = self.read_json_source_reader(reader, file_bytes, display_path, label)?;
        let value: Value = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse JSON {label} {}", display_path.display()))?;
        self.admit_json_tree(&value, &format!("{label} {}", display_path.display()))?;
        Ok((bytes, value))
    }

    pub(crate) fn read_resource_json_bytes_and_value_from_reader(
        &self,
        reader: impl Read,
        file_bytes: u64,
        display_path: &Path,
        label: &str,
    ) -> Result<(Vec<u8>, ResourceJsonValue)> {
        let bytes = self.read_json_source_reader(reader, file_bytes, display_path, label)?;
        let value = ResourceJsonValue::from_slice(&bytes)
            .with_context(|| format!("parse JSON {label} {}", display_path.display()))?;
        let (nodes, string_bytes) = value.tree_size()?;
        self.admit_json_counts(
            nodes,
            string_bytes,
            &format!("{label} {}", display_path.display()),
        )?;
        Ok((bytes, value))
    }

    fn read_json_source(&self, path: &Path, label: &str) -> Result<Vec<u8>> {
        let (file, file_bytes) = crate::virtual_store::open_resource_file(path)
            .with_context(|| format!("open JSON {label} {}", path.display()))?;
        self.read_json_source_reader(file, file_bytes, path, label)
    }

    fn read_json_source_reader(
        &self,
        reader: impl Read,
        file_bytes: u64,
        path: &Path,
        label: &str,
    ) -> Result<Vec<u8>> {
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

        let capacity = usize::try_from(file_bytes.min(64 * 1024))
            .with_context(|| format!("JSON {label} file size does not fit usize"))?;
        let mut bytes = Vec::with_capacity(capacity);
        reader
            .take(MAX_RESOURCE_JSON_FILE_BYTES + 1)
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
        let (file, file_bytes) = crate::virtual_store::open_resource_file(path)
            .with_context(|| format!("open text {label} {}", path.display()))?;
        self.read_text_from_reader(file, file_bytes, path, label)
    }

    pub(crate) fn read_text_from_reader(
        &self,
        reader: impl Read,
        file_bytes: u64,
        path: &Path,
        label: &str,
    ) -> Result<String> {
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

        let capacity = usize::try_from(file_bytes.min(64 * 1024))
            .with_context(|| format!("text {label} file size does not fit usize"))?;
        let mut bytes = Vec::with_capacity(capacity);
        reader
            .take(MAX_RESOURCE_TEXT_FILE_BYTES + 1)
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

    pub(crate) fn read_optional_binary(
        &self,
        path: &Path,
        maximum: u64,
        label: &str,
    ) -> Result<Vec<u8>> {
        let (file, file_bytes) = crate::virtual_store::open_resource_file(path)
            .with_context(|| format!("open binary {label} {}", path.display()))?;
        self.read_optional_binary_from_reader(file, file_bytes, path, maximum, label)
    }

    pub(crate) fn read_optional_binary_from_reader(
        &self,
        reader: impl Read,
        file_bytes: u64,
        path: &Path,
        maximum: u64,
        label: &str,
    ) -> Result<Vec<u8>> {
        if file_bytes > maximum {
            bail!(
                "binary {label} {} is {file_bytes} bytes, exceeding the {maximum} byte limit",
                path.display()
            );
        }
        self.reserve_optional(file_bytes, label)?;
        self.derived_logical_bytes
            .fetch_add(file_bytes, Ordering::Relaxed);

        let capacity = usize::try_from(file_bytes.min(64 * 1024))
            .with_context(|| format!("binary {label} file size does not fit usize"))?;
        let mut bytes = Vec::with_capacity(capacity);
        reader
            .take(maximum + 1)
            .read_to_end(&mut bytes)
            .with_context(|| format!("read binary {label} {}", path.display()))?;
        if bytes.len() as u64 > maximum {
            bail!(
                "binary {label} {} grew beyond the {maximum} byte limit while being read",
                path.display()
            );
        }
        let actual_bytes = u64::try_from(bytes.len())
            .with_context(|| format!("binary {label} byte length does not fit u64"))?;
        if actual_bytes > file_bytes {
            let growth = actual_bytes - file_bytes;
            self.reserve_optional(growth, label)?;
            self.derived_logical_bytes
                .fetch_add(growth, Ordering::Relaxed);
        }
        Ok(bytes)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JsonLimitHit {
    Nodes,
    StringBytes,
}

#[derive(Debug, Default)]
struct JsonStreamCounts {
    nodes: u64,
    string_bytes: u64,
    limit_hit: Option<JsonLimitHit>,
}

impl JsonStreamCounts {
    fn record_node<E: serde::de::Error>(&mut self) -> std::result::Result<(), E> {
        self.nodes = self.nodes.saturating_add(1);
        if self.nodes > MAX_RESOURCE_JSON_DOCUMENT_NODES {
            self.limit_hit = Some(JsonLimitHit::Nodes);
            return Err(E::custom("JSON document exceeded node limit"));
        }
        Ok(())
    }

    fn record_string_bytes<E: serde::de::Error>(
        &mut self,
        len: usize,
    ) -> std::result::Result<(), E> {
        let len = u64::try_from(len).unwrap_or(u64::MAX);
        self.string_bytes = self.string_bytes.saturating_add(len);
        if self.string_bytes > MAX_RESOURCE_JSON_DOCUMENT_STRING_BYTES {
            self.limit_hit = Some(JsonLimitHit::StringBytes);
            return Err(E::custom("JSON document exceeded string byte limit"));
        }
        Ok(())
    }
}

struct CountingDeserializer<'a, D> {
    inner: D,
    counts: &'a mut JsonStreamCounts,
}

macro_rules! forward_counting_deserialize {
    ($($method:ident),* $(,)?) => {
        $(
            fn $method<V: serde::de::Visitor<'de>>(
                self,
                visitor: V,
            ) -> std::result::Result<V::Value, Self::Error> {
                self.inner.$method(CountingVisitor {
                    inner: visitor,
                    counts: self.counts,
                })
            }
        )*
    };
}

impl<'de, 'a, D: serde::Deserializer<'de>> serde::Deserializer<'de>
    for CountingDeserializer<'a, D>
{
    type Error = D::Error;

    forward_counting_deserialize!(
        deserialize_any,
        deserialize_bool,
        deserialize_i8,
        deserialize_i16,
        deserialize_i32,
        deserialize_i64,
        deserialize_i128,
        deserialize_u8,
        deserialize_u16,
        deserialize_u32,
        deserialize_u64,
        deserialize_u128,
        deserialize_f32,
        deserialize_f64,
        deserialize_char,
        deserialize_str,
        deserialize_string,
        deserialize_bytes,
        deserialize_byte_buf,
        deserialize_option,
        deserialize_unit,
        deserialize_seq,
        deserialize_map,
        deserialize_identifier,
    );

    fn deserialize_unit_struct<V: serde::de::Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        self.inner.deserialize_unit_struct(
            name,
            CountingVisitor {
                inner: visitor,
                counts: self.counts,
            },
        )
    }

    fn deserialize_newtype_struct<V: serde::de::Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        self.inner.deserialize_newtype_struct(
            name,
            CountingVisitor {
                inner: visitor,
                counts: self.counts,
            },
        )
    }

    fn deserialize_tuple<V: serde::de::Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        self.inner.deserialize_tuple(
            len,
            CountingVisitor {
                inner: visitor,
                counts: self.counts,
            },
        )
    }

    fn deserialize_tuple_struct<V: serde::de::Visitor<'de>>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        self.inner.deserialize_tuple_struct(
            name,
            len,
            CountingVisitor {
                inner: visitor,
                counts: self.counts,
            },
        )
    }

    fn deserialize_struct<V: serde::de::Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        self.inner.deserialize_struct(
            name,
            fields,
            CountingVisitor {
                inner: visitor,
                counts: self.counts,
            },
        )
    }

    fn deserialize_enum<V: serde::de::Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        self.inner.deserialize_enum(
            name,
            variants,
            CountingVisitor {
                inner: visitor,
                counts: self.counts,
            },
        )
    }

    fn deserialize_ignored_any<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        // Count every node and string byte inside unknown fields without building a DOM.
        self.inner.deserialize_any(CountingIgnoredVisitor {
            counts: self.counts,
        })?;
        visitor.visit_unit()
    }

    fn is_human_readable(&self) -> bool {
        self.inner.is_human_readable()
    }
}

struct CountingVisitor<'a, V> {
    inner: V,
    counts: &'a mut JsonStreamCounts,
}

macro_rules! forward_counting_primitive {
    ($($method:ident($ty:ty)),* $(,)?) => {
        $(
            fn $method<E: serde::de::Error>(
                self,
                v: $ty,
            ) -> std::result::Result<Self::Value, E> {
                self.counts.record_node::<E>()?;
                self.inner.$method(v)
            }
        )*
    };
}

impl<'de, 'a, V: serde::de::Visitor<'de>> serde::de::Visitor<'de> for CountingVisitor<'a, V> {
    type Value = V::Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.expecting(formatter)
    }

    forward_counting_primitive!(
        visit_bool(bool),
        visit_i8(i8),
        visit_i16(i16),
        visit_i32(i32),
        visit_i64(i64),
        visit_i128(i128),
        visit_u8(u8),
        visit_u16(u16),
        visit_u32(u32),
        visit_u64(u64),
        visit_u128(u128),
        visit_f32(f32),
        visit_f64(f64),
    );

    fn visit_char<E: serde::de::Error>(self, v: char) -> std::result::Result<Self::Value, E> {
        self.counts.record_node::<E>()?;
        self.counts.record_string_bytes::<E>(v.len_utf8())?;
        self.inner.visit_char(v)
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
        self.counts.record_node::<E>()?;
        self.counts.record_string_bytes::<E>(v.len())?;
        self.inner.visit_str(v)
    }

    fn visit_borrowed_str<E: serde::de::Error>(
        self,
        v: &'de str,
    ) -> std::result::Result<Self::Value, E> {
        self.counts.record_node::<E>()?;
        self.counts.record_string_bytes::<E>(v.len())?;
        self.inner.visit_borrowed_str(v)
    }

    fn visit_string<E: serde::de::Error>(self, v: String) -> std::result::Result<Self::Value, E> {
        self.counts.record_node::<E>()?;
        self.counts.record_string_bytes::<E>(v.len())?;
        self.inner.visit_string(v)
    }

    fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> std::result::Result<Self::Value, E> {
        self.counts.record_node::<E>()?;
        self.inner.visit_bytes(v)
    }

    fn visit_borrowed_bytes<E: serde::de::Error>(
        self,
        v: &'de [u8],
    ) -> std::result::Result<Self::Value, E> {
        self.counts.record_node::<E>()?;
        self.inner.visit_borrowed_bytes(v)
    }

    fn visit_byte_buf<E: serde::de::Error>(
        self,
        v: Vec<u8>,
    ) -> std::result::Result<Self::Value, E> {
        self.counts.record_node::<E>()?;
        self.inner.visit_byte_buf(v)
    }

    fn visit_none<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
        self.counts.record_node::<E>()?;
        self.inner.visit_none()
    }

    fn visit_some<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<Self::Value, D::Error> {
        self.inner.visit_some(CountingDeserializer {
            inner: deserializer,
            counts: self.counts,
        })
    }

    fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
        self.counts.record_node::<E>()?;
        self.inner.visit_unit()
    }

    fn visit_newtype_struct<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<Self::Value, D::Error> {
        self.inner.visit_newtype_struct(CountingDeserializer {
            inner: deserializer,
            counts: self.counts,
        })
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(
        self,
        seq: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        self.counts.record_node::<A::Error>()?;
        self.inner.visit_seq(CountingSeqAccess {
            inner: seq,
            counts: self.counts,
        })
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(
        self,
        map: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        self.counts.record_node::<A::Error>()?;
        self.inner.visit_map(CountingMapAccess {
            inner: map,
            counts: self.counts,
        })
    }

    fn visit_enum<A: serde::de::EnumAccess<'de>>(
        self,
        data: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        self.counts.record_node::<A::Error>()?;
        self.inner.visit_enum(CountingEnumAccess {
            inner: data,
            counts: self.counts,
        })
    }
}

struct CountingSeed<'a, S> {
    inner: S,
    counts: &'a mut JsonStreamCounts,
}

impl<'de, 'a, S: serde::de::DeserializeSeed<'de>> serde::de::DeserializeSeed<'de>
    for CountingSeed<'a, S>
{
    type Value = S::Value;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<Self::Value, D::Error> {
        self.inner.deserialize(CountingDeserializer {
            inner: deserializer,
            counts: self.counts,
        })
    }
}

struct KeyCountingSeed<'a, S> {
    inner: S,
    counts: &'a mut JsonStreamCounts,
}

impl<'de, 'a, S: serde::de::DeserializeSeed<'de>> serde::de::DeserializeSeed<'de>
    for KeyCountingSeed<'a, S>
{
    type Value = S::Value;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<Self::Value, D::Error> {
        self.inner.deserialize(KeyCountingDeserializer {
            inner: deserializer,
            counts: self.counts,
        })
    }
}

struct KeyCountingDeserializer<'a, D> {
    inner: D,
    counts: &'a mut JsonStreamCounts,
}

macro_rules! forward_key_deserialize {
    ($($method:ident),* $(,)?) => {
        $(
            fn $method<V: serde::de::Visitor<'de>>(
                self,
                visitor: V,
            ) -> std::result::Result<V::Value, Self::Error> {
                self.inner.$method(KeyCountingVisitor {
                    inner: visitor,
                    counts: self.counts,
                })
            }
        )*
    };
}

impl<'de, 'a, D: serde::Deserializer<'de>> serde::Deserializer<'de>
    for KeyCountingDeserializer<'a, D>
{
    type Error = D::Error;

    forward_key_deserialize!(
        deserialize_any,
        deserialize_bool,
        deserialize_i8,
        deserialize_i16,
        deserialize_i32,
        deserialize_i64,
        deserialize_i128,
        deserialize_u8,
        deserialize_u16,
        deserialize_u32,
        deserialize_u64,
        deserialize_u128,
        deserialize_f32,
        deserialize_f64,
        deserialize_char,
        deserialize_str,
        deserialize_string,
        deserialize_bytes,
        deserialize_byte_buf,
        deserialize_option,
        deserialize_unit,
        deserialize_seq,
        deserialize_map,
        deserialize_identifier,
    );

    fn deserialize_ignored_any<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        self.inner.deserialize_str(CountingIgnoredKeyVisitor {
            counts: self.counts,
        })?;
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: serde::de::Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        self.inner.deserialize_unit_struct(
            name,
            KeyCountingVisitor {
                inner: visitor,
                counts: self.counts,
            },
        )
    }

    fn deserialize_newtype_struct<V: serde::de::Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        self.inner.deserialize_newtype_struct(
            name,
            KeyCountingVisitor {
                inner: visitor,
                counts: self.counts,
            },
        )
    }

    fn deserialize_tuple<V: serde::de::Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        self.inner.deserialize_tuple(
            len,
            KeyCountingVisitor {
                inner: visitor,
                counts: self.counts,
            },
        )
    }

    fn deserialize_tuple_struct<V: serde::de::Visitor<'de>>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        self.inner.deserialize_tuple_struct(
            name,
            len,
            KeyCountingVisitor {
                inner: visitor,
                counts: self.counts,
            },
        )
    }

    fn deserialize_struct<V: serde::de::Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        self.inner.deserialize_struct(
            name,
            fields,
            KeyCountingVisitor {
                inner: visitor,
                counts: self.counts,
            },
        )
    }

    fn deserialize_enum<V: serde::de::Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        self.inner.deserialize_enum(
            name,
            variants,
            KeyCountingVisitor {
                inner: visitor,
                counts: self.counts,
            },
        )
    }

    fn is_human_readable(&self) -> bool {
        self.inner.is_human_readable()
    }
}

fn decimal_len_u128(mut v: u128) -> usize {
    let mut len = 1;
    while v >= 10 {
        v /= 10;
        len += 1;
    }
    len
}

fn decimal_len_i128(v: i128) -> usize {
    let sign = usize::from(v < 0);
    sign + decimal_len_u128(v.unsigned_abs())
}

struct KeyCountingVisitor<'a, V> {
    inner: V,
    counts: &'a mut JsonStreamCounts,
}

macro_rules! forward_signed_key_primitive {
    ($($method:ident($ty:ty)),* $(,)?) => {
        $(
            fn $method<E: serde::de::Error>(
                self,
                v: $ty,
            ) -> std::result::Result<Self::Value, E> {
                self.counts.record_string_bytes::<E>(decimal_len_i128(i128::from(v)))?;
                self.inner.$method(v)
            }
        )*
    };
}

macro_rules! forward_unsigned_key_primitive {
    ($($method:ident($ty:ty)),* $(,)?) => {
        $(
            fn $method<E: serde::de::Error>(
                self,
                v: $ty,
            ) -> std::result::Result<Self::Value, E> {
                self.counts.record_string_bytes::<E>(decimal_len_u128(u128::from(v)))?;
                self.inner.$method(v)
            }
        )*
    };
}

impl<'de, 'a, V: serde::de::Visitor<'de>> serde::de::Visitor<'de> for KeyCountingVisitor<'a, V> {
    type Value = V::Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.expecting(formatter)
    }

    fn visit_bool<E: serde::de::Error>(self, v: bool) -> std::result::Result<Self::Value, E> {
        self.counts
            .record_string_bytes::<E>(if v { 4 } else { 5 })?;
        self.inner.visit_bool(v)
    }

    forward_signed_key_primitive!(
        visit_i8(i8),
        visit_i16(i16),
        visit_i32(i32),
        visit_i64(i64),
        visit_i128(i128),
    );

    forward_unsigned_key_primitive!(
        visit_u8(u8),
        visit_u16(u16),
        visit_u32(u32),
        visit_u64(u64),
        visit_u128(u128),
    );

    fn visit_f32<E: serde::de::Error>(self, v: f32) -> std::result::Result<Self::Value, E> {
        self.counts.record_string_bytes::<E>(8)?;
        self.inner.visit_f32(v)
    }

    fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
        self.counts.record_string_bytes::<E>(8)?;
        self.inner.visit_f64(v)
    }

    fn visit_char<E: serde::de::Error>(self, v: char) -> std::result::Result<Self::Value, E> {
        self.counts.record_string_bytes::<E>(v.len_utf8())?;
        self.inner.visit_char(v)
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
        self.counts.record_string_bytes::<E>(v.len())?;
        self.inner.visit_str(v)
    }

    fn visit_borrowed_str<E: serde::de::Error>(
        self,
        v: &'de str,
    ) -> std::result::Result<Self::Value, E> {
        self.counts.record_string_bytes::<E>(v.len())?;
        self.inner.visit_borrowed_str(v)
    }

    fn visit_string<E: serde::de::Error>(self, v: String) -> std::result::Result<Self::Value, E> {
        self.counts.record_string_bytes::<E>(v.len())?;
        self.inner.visit_string(v)
    }

    fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> std::result::Result<Self::Value, E> {
        self.counts.record_string_bytes::<E>(v.len())?;
        self.inner.visit_bytes(v)
    }

    fn visit_borrowed_bytes<E: serde::de::Error>(
        self,
        v: &'de [u8],
    ) -> std::result::Result<Self::Value, E> {
        self.counts.record_string_bytes::<E>(v.len())?;
        self.inner.visit_borrowed_bytes(v)
    }

    fn visit_byte_buf<E: serde::de::Error>(
        self,
        v: Vec<u8>,
    ) -> std::result::Result<Self::Value, E> {
        self.counts.record_string_bytes::<E>(v.len())?;
        self.inner.visit_byte_buf(v)
    }

    fn visit_none<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
        self.inner.visit_none()
    }

    fn visit_some<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<Self::Value, D::Error> {
        self.inner.visit_some(KeyCountingDeserializer {
            inner: deserializer,
            counts: self.counts,
        })
    }

    fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
        self.inner.visit_unit()
    }

    fn visit_newtype_struct<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<Self::Value, D::Error> {
        self.inner.visit_newtype_struct(KeyCountingDeserializer {
            inner: deserializer,
            counts: self.counts,
        })
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(
        self,
        seq: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        self.inner.visit_seq(CountingSeqAccess {
            inner: seq,
            counts: self.counts,
        })
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(
        self,
        map: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        self.inner.visit_map(CountingMapAccess {
            inner: map,
            counts: self.counts,
        })
    }

    fn visit_enum<A: serde::de::EnumAccess<'de>>(
        self,
        data: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        self.inner.visit_enum(CountingEnumAccess {
            inner: data,
            counts: self.counts,
        })
    }
}

struct CountingSeqAccess<'a, A> {
    inner: A,
    counts: &'a mut JsonStreamCounts,
}

impl<'de, 'a, A: serde::de::SeqAccess<'de>> serde::de::SeqAccess<'de> for CountingSeqAccess<'a, A> {
    type Error = A::Error;

    fn next_element_seed<T: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> std::result::Result<Option<T::Value>, Self::Error> {
        self.inner.next_element_seed(CountingSeed {
            inner: seed,
            counts: self.counts,
        })
    }

    fn size_hint(&self) -> Option<usize> {
        self.inner.size_hint()
    }
}

struct CountingMapAccess<'a, A> {
    inner: A,
    counts: &'a mut JsonStreamCounts,
}

impl<'de, 'a, A: serde::de::MapAccess<'de>> serde::de::MapAccess<'de> for CountingMapAccess<'a, A> {
    type Error = A::Error;

    fn next_key_seed<K: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> std::result::Result<Option<K::Value>, Self::Error> {
        self.inner.next_key_seed(KeyCountingSeed {
            inner: seed,
            counts: self.counts,
        })
    }

    fn next_value_seed<V: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        self.inner.next_value_seed(CountingSeed {
            inner: seed,
            counts: self.counts,
        })
    }

    fn size_hint(&self) -> Option<usize> {
        self.inner.size_hint()
    }
}

struct CountingEnumAccess<'a, A> {
    inner: A,
    counts: &'a mut JsonStreamCounts,
}

impl<'de, 'a, A: serde::de::EnumAccess<'de>> serde::de::EnumAccess<'de>
    for CountingEnumAccess<'a, A>
{
    type Error = A::Error;
    type Variant = CountingVariantAccess<'a, A::Variant>;

    fn variant_seed<V: serde::de::DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> std::result::Result<(V::Value, Self::Variant), Self::Error> {
        let (value, variant) = self.inner.variant_seed(KeyCountingSeed {
            inner: seed,
            counts: self.counts,
        })?;
        Ok((
            value,
            CountingVariantAccess {
                inner: variant,
                counts: self.counts,
            },
        ))
    }
}

struct CountingVariantAccess<'a, A> {
    inner: A,
    counts: &'a mut JsonStreamCounts,
}

impl<'de, 'a, A: serde::de::VariantAccess<'de>> serde::de::VariantAccess<'de>
    for CountingVariantAccess<'a, A>
{
    type Error = A::Error;

    fn unit_variant(self) -> std::result::Result<(), Self::Error> {
        self.inner.unit_variant()
    }

    fn newtype_variant_seed<T: serde::de::DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> std::result::Result<T::Value, Self::Error> {
        self.inner.newtype_variant_seed(CountingSeed {
            inner: seed,
            counts: self.counts,
        })
    }

    fn tuple_variant<V: serde::de::Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        self.inner.tuple_variant(
            len,
            CountingVisitor {
                inner: visitor,
                counts: self.counts,
            },
        )
    }

    fn struct_variant<V: serde::de::Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        self.inner.struct_variant(
            fields,
            CountingVisitor {
                inner: visitor,
                counts: self.counts,
            },
        )
    }
}

struct CountingIgnoredSeed<'a>(&'a mut JsonStreamCounts);

impl<'de, 'a> serde::de::DeserializeSeed<'de> for CountingIgnoredSeed<'a> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<(), D::Error> {
        deserializer.deserialize_any(CountingIgnoredVisitor { counts: self.0 })
    }
}

struct CountingIgnoredKeySeed<'a>(&'a mut JsonStreamCounts);

impl<'de, 'a> serde::de::DeserializeSeed<'de> for CountingIgnoredKeySeed<'a> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<(), D::Error> {
        deserializer.deserialize_str(CountingIgnoredKeyVisitor { counts: self.0 })
    }
}

struct CountingIgnoredKeyVisitor<'a> {
    counts: &'a mut JsonStreamCounts,
}

impl<'de> serde::de::Visitor<'de> for CountingIgnoredKeyVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON object key")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<(), E> {
        self.counts.record_string_bytes::<E>(v.len())
    }

    fn visit_borrowed_str<E: serde::de::Error>(self, v: &'de str) -> std::result::Result<(), E> {
        self.counts.record_string_bytes::<E>(v.len())
    }

    fn visit_string<E: serde::de::Error>(self, v: String) -> std::result::Result<(), E> {
        self.counts.record_string_bytes::<E>(v.len())
    }
}

struct CountingIgnoredVisitor<'a> {
    counts: &'a mut JsonStreamCounts,
}

impl<'de> serde::de::Visitor<'de> for CountingIgnoredVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("any valid JSON value")
    }

    fn visit_bool<E: serde::de::Error>(self, _: bool) -> std::result::Result<(), E> {
        self.counts.record_node::<E>()
    }

    fn visit_i64<E: serde::de::Error>(self, _: i64) -> std::result::Result<(), E> {
        self.counts.record_node::<E>()
    }

    fn visit_u64<E: serde::de::Error>(self, _: u64) -> std::result::Result<(), E> {
        self.counts.record_node::<E>()
    }

    fn visit_f64<E: serde::de::Error>(self, _: f64) -> std::result::Result<(), E> {
        self.counts.record_node::<E>()
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<(), E> {
        self.counts.record_node::<E>()?;
        self.counts.record_string_bytes::<E>(v.len())
    }

    fn visit_borrowed_str<E: serde::de::Error>(self, v: &'de str) -> std::result::Result<(), E> {
        self.counts.record_node::<E>()?;
        self.counts.record_string_bytes::<E>(v.len())
    }

    fn visit_string<E: serde::de::Error>(self, v: String) -> std::result::Result<(), E> {
        self.counts.record_node::<E>()?;
        self.counts.record_string_bytes::<E>(v.len())
    }

    fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<(), E> {
        self.counts.record_node::<E>()
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(
        self,
        mut seq: A,
    ) -> std::result::Result<(), A::Error> {
        self.counts.record_node::<A::Error>()?;
        while seq
            .next_element_seed(CountingIgnoredSeed(self.counts))?
            .is_some()
        {}
        Ok(())
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(
        self,
        mut map: A,
    ) -> std::result::Result<(), A::Error> {
        self.counts.record_node::<A::Error>()?;
        while map
            .next_key_seed(CountingIgnoredKeySeed(self.counts))?
            .is_some()
        {
            map.next_value_seed(CountingIgnoredSeed(self.counts))?;
        }
        Ok(())
    }
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

    #[test]
    fn single_pass_read_json_counts_typed_and_ignored_fields_and_enforces_node_limit() {
        #[derive(Debug, serde::Deserialize)]
        struct Small {
            known: u32,
        }

        let path = temp_path("single-pass-ignored.json");
        std::fs::write(&path, br#"{"known":7,"ignored":["abc",{"nested":true}]}"#).unwrap();
        let budget = ResourceDataGenerationBudget::new(4096);
        let parsed: Small = budget.read_json(&path, "typed").unwrap();
        assert_eq!(parsed.known, 7);
        let obs = budget.observation();
        // Nodes: root object + 7 + array + "abc" + nested object + true = 6.
        // String bytes: "known"(5) + "ignored"(7) + "abc"(3) + "nested"(6) = 21.
        assert_eq!(obs.json_documents, 1);
        assert_eq!(obs.json_nodes, 6);
        assert_eq!(obs.json_string_bytes, 21);
        assert_eq!(obs.rejections, 0);

        let bomb = format!(
            r#"{{"known":1,"ignored":[{}0]}}"#,
            "0,".repeat(MAX_RESOURCE_JSON_DOCUMENT_NODES as usize - 1)
        );
        std::fs::write(&path, bomb.as_bytes()).unwrap();
        let strict = ResourceDataGenerationBudget::new(u64::MAX);
        let err = strict
            .read_json::<Small>(&path, "ignored bomb")
            .unwrap_err()
            .to_string();
        assert!(err.contains("node limit"), "{err}");
        assert_eq!(strict.observation().rejections, 1);
        assert!(strict.ensure_admitted().is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn single_pass_read_json_supports_numeric_and_bool_map_keys() {
        use std::collections::BTreeMap;

        #[derive(Debug, serde::Deserialize)]
        struct WithIntMap {
            by_id: BTreeMap<i32, Value>,
            by_flag: BTreeMap<bool, u32>,
        }

        let path = temp_path("single-pass-numeric-keys.json");
        std::fs::write(
            &path,
            br#"{"by_id":{"-12":1,"432":"elytra"},"by_flag":{"true":7,"false":0}}"#,
        )
        .unwrap();
        let budget = ResourceDataGenerationBudget::new(4096);
        let parsed: WithIntMap = budget.read_json(&path, "numeric map keys").unwrap();
        assert_eq!(parsed.by_id.get(&-12), Some(&Value::from(1)));
        assert_eq!(parsed.by_id.get(&432), Some(&Value::from("elytra")));
        assert_eq!(parsed.by_flag.get(&true), Some(&7));
        assert_eq!(parsed.by_flag.get(&false), Some(&0));
        let obs = budget.observation();
        // String bytes: "by_id"(5)+"-12"(3)+"432"(3)+"elytra"(6)+"by_flag"(7)+"true"(4)+"false"(5)=33.
        assert_eq!(obs.json_string_bytes, 33);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn single_pass_read_json_counts_ignored_map_keys() {
        struct IgnoredEntries;

        impl<'de> serde::Deserialize<'de> for IgnoredEntries {
            fn deserialize<D: serde::Deserializer<'de>>(
                deserializer: D,
            ) -> std::result::Result<Self, D::Error> {
                struct Visitor;
                impl<'de> serde::de::Visitor<'de> for Visitor {
                    type Value = IgnoredEntries;
                    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.write_str("a JSON object")
                    }
                    fn visit_map<A: serde::de::MapAccess<'de>>(
                        self,
                        mut map: A,
                    ) -> std::result::Result<Self::Value, A::Error> {
                        while map
                            .next_entry::<serde::de::IgnoredAny, serde::de::IgnoredAny>()?
                            .is_some()
                        {}
                        Ok(IgnoredEntries)
                    }
                }
                deserializer.deserialize_map(Visitor)
            }
        }

        let path = temp_path("single-pass-ignored-keys.json");
        std::fs::write(&path, br#"{"alpha":1,"beta_key":"val"}"#).unwrap();
        let budget = ResourceDataGenerationBudget::new(4096);
        let _: IgnoredEntries = budget.read_json(&path, "ignored map keys").unwrap();
        let obs = budget.observation();
        // Nodes: root object + 1 + "val" = 3.
        // String bytes: "alpha"(5) + "beta_key"(8) + "val"(3) = 16.
        assert_eq!(obs.json_nodes, 3);
        assert_eq!(obs.json_string_bytes, 16);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn resource_json_reader_charges_wtf8_keys_values_and_keeps_json_budget() {
        use crate::{PackResourceStack, ResourceLocation};

        let root = temp_path("resource-json");
        let path = root.join("assets/minecraft/font/test.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let source = br#"{"\uD800":["\uDFFF","\uD800\uDC00",1]}"#;
        std::fs::write(&path, source).unwrap();
        let stack = PackResourceStack::from_roots([root.clone()]);
        let resource = stack
            .get_resource(&ResourceLocation::parse("minecraft:font/test.json").unwrap())
            .unwrap();
        let value = stack.read_resource_json(&resource, "font fixture").unwrap();
        assert_eq!(
            value.as_object().unwrap()[0]
                .0
                .codepoints()
                .collect::<Vec<_>>(),
            [0xd800]
        );
        let observation = stack.resource_data_memory_observation();
        assert_eq!(observation.json_source_bytes, source.len() as u64);
        assert_eq!(observation.json_documents, 1);
        assert_eq!(observation.json_nodes, 5);
        assert_eq!(observation.json_string_bytes, 11);
        assert_eq!(
            observation.charged_logical_bytes,
            source.len() as u64 + 5 * RESOURCE_JSON_NODE_LOGICAL_BYTES + 11
        );
        assert_eq!(observation.text_documents, 0);
        assert_eq!(observation.text_source_bytes, 0);
        assert_eq!(observation.rejections, 0);
        assert!(stack
            .read_json_resource::<Value>(&resource, "strict UTF-8")
            .is_err());

        let budget = ResourceDataGenerationBudget::new(
            source.len() as u64 + 5 * RESOURCE_JSON_NODE_LOGICAL_BYTES + 10 - 1,
        );
        assert!(budget
            .read_resource_json(&path, "over generation budget")
            .is_err());
        assert_eq!(budget.observation().json_source_bytes, source.len() as u64);
        assert_eq!(budget.observation().json_documents, 0);
        assert_eq!(budget.observation().rejections, 1);
        assert!(budget.ensure_admitted().is_err());
        std::fs::write(&path, b"null").unwrap();
        budget
            .read_resource_json(&path, "small subsequent resource")
            .unwrap();
        assert!(budget.ensure_admitted().is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resource_json_bytes_reader_retains_source_and_charges_tree_once() {
        let path = temp_path("resource-json-bytes");
        let source = br#"{"\uD800":["\uDFFF","\uD800\uDC00",1]}"#;
        std::fs::write(&path, source).unwrap();
        let expected_charge = source.len() as u64 + 5 * RESOURCE_JSON_NODE_LOGICAL_BYTES + 11;
        let budget = ResourceDataGenerationBudget::new(expected_charge);
        let (bytes, value) = budget
            .read_resource_json_bytes_and_value(&path, "bytes")
            .unwrap();
        assert_eq!(bytes, source);
        assert_eq!(value.as_object().unwrap()[0].0.utf16_units(), [0xd800]);
        let observation = budget.observation();
        assert_eq!(observation.json_source_bytes, source.len() as u64);
        assert_eq!(observation.json_documents, 1);
        assert_eq!(observation.json_nodes, 5);
        assert_eq!(observation.json_string_bytes, 11);
        assert_eq!(observation.charged_logical_bytes, expected_charge);
        assert_eq!(observation.rejections, 0);
        assert!(budget.ensure_admitted().is_ok());

        let over_budget = ResourceDataGenerationBudget::new(expected_charge - 1);
        assert!(over_budget
            .read_resource_json_bytes_and_value(&path, "generation limit")
            .is_err());
        assert_eq!(
            over_budget.observation().json_source_bytes,
            source.len() as u64
        );
        assert_eq!(over_budget.observation().json_documents, 0);
        assert_eq!(over_budget.observation().rejections, 1);
        assert!(over_budget.ensure_admitted().is_err());
        std::fs::write(&path, b"null").unwrap();
        over_budget
            .read_resource_json_bytes_and_value(&path, "later small value")
            .unwrap();
        assert!(over_budget.ensure_admitted().is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn resource_json_bytes_reader_enforces_file_and_node_limits() {
        let path = temp_path("resource-json-bytes-limits");
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .unwrap();
        file.set_len(MAX_RESOURCE_JSON_FILE_BYTES + 1).unwrap();
        let budget = ResourceDataGenerationBudget::new(u64::MAX);
        assert!(budget
            .read_resource_json_bytes_and_value(&path, "source limit")
            .is_err());
        assert_eq!(budget.observation().charged_logical_bytes, 0);
        assert_eq!(budget.observation().rejections, 1);
        assert!(budget.ensure_admitted().is_err());

        let source = format!(
            "[{}0]",
            "0,".repeat(MAX_RESOURCE_JSON_DOCUMENT_NODES as usize - 1)
        );
        std::fs::write(&path, source.as_bytes()).unwrap();
        let budget = ResourceDataGenerationBudget::new(u64::MAX);
        let error = budget
            .read_resource_json_bytes_and_value(&path, "node limit")
            .unwrap_err()
            .to_string();
        assert!(error.contains("node limit"), "{error}");
        assert_eq!(budget.observation().json_source_bytes, source.len() as u64);
        assert_eq!(budget.observation().json_documents, 0);
        assert_eq!(budget.observation().rejections, 1);
        assert!(budget.ensure_admitted().is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn resource_json_resource_reader_checks_integrity_before_charging() {
        use crate::{PackResourceStack, ResourceIntegrity, ResourceLocation};

        let root = temp_path("resource-json-integrity");
        let path = root.join("assets/minecraft/font/test.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{}").unwrap();
        let stack = PackResourceStack::from_roots([root.clone()]);
        let mut resource = stack
            .get_resource(&ResourceLocation::parse("minecraft:font/test.json").unwrap())
            .unwrap();
        resource.integrity = Some(ResourceIntegrity {
            size: 3,
            sha1: String::new(),
        });
        assert!(stack.read_resource_json(&resource, "integrity").is_err());
        assert_eq!(
            stack.resource_data_memory_observation().json_source_bytes,
            0
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resource_json_file_and_node_rejections_are_sticky() {
        let path = temp_path("resource-json-limits");
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .unwrap();
        file.set_len(MAX_RESOURCE_JSON_FILE_BYTES + 1).unwrap();
        let budget = ResourceDataGenerationBudget::new(u64::MAX);
        assert!(budget.read_resource_json(&path, "source limit").is_err());
        assert_eq!(budget.observation().charged_logical_bytes, 0);
        assert_eq!(budget.observation().rejections, 1);
        assert!(budget.ensure_admitted().is_err());

        let source = format!(
            "[{}0]",
            "0,".repeat(MAX_RESOURCE_JSON_DOCUMENT_NODES as usize - 1)
        );
        std::fs::write(&path, source.as_bytes()).unwrap();
        let budget = ResourceDataGenerationBudget::new(u64::MAX);
        let error = budget
            .read_resource_json(&path, "node limit")
            .unwrap_err()
            .to_string();
        assert!(error.contains("node limit"), "{error}");
        assert_eq!(budget.observation().json_source_bytes, source.len() as u64);
        assert_eq!(budget.observation().json_documents, 0);
        assert_eq!(budget.observation().rejections, 1);
        assert!(budget.ensure_admitted().is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn resource_json_shared_decoded_string_limit_remains_sticky() {
        let budget = ResourceDataGenerationBudget::new(u64::MAX);
        budget
            .admit_json_counts(1, MAX_RESOURCE_JSON_DOCUMENT_STRING_BYTES, "boundary")
            .unwrap();
        let error = budget
            .admit_json_counts(
                1,
                MAX_RESOURCE_JSON_DOCUMENT_STRING_BYTES + 1,
                "over string limit",
            )
            .unwrap_err()
            .to_string();
        assert!(error.contains("decoded key/value bytes"), "{error}");
        assert_eq!(budget.observation().json_documents, 1);
        assert_eq!(budget.observation().rejections, 1);
        assert!(budget.ensure_admitted().is_err());
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
}
