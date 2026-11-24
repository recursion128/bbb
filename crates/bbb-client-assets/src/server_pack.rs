use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::zip_pack::ZipPackResources;
use anyhow::{bail, Context, Result};
use serde::Deserialize;

const CLIENT_RESOURCE_PACK_FORMAT: ServerPackFormat = ServerPackFormat {
    major: 84,
    minor: 0,
};
const LAST_PRE_MINOR_CLIENT_RESOURCE_PACK_FORMAT: u32 = 64;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServerPackFormat {
    pub major: u32,
    pub minor: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerPackFormatRange {
    pub min: ServerPackFormat,
    pub max: ServerPackFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientPackMetadata {
    pub supported_formats: ServerPackFormatRange,
    document: Arc<serde_json::Value>,
    source_bytes: u64,
}

impl ClientPackMetadata {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let document: serde_json::Value =
            serde_json::from_slice(bytes).context("parse client resource pack metadata JSON")?;
        let source_bytes = u64::try_from(bytes.len()).context("pack metadata size overflow")?;
        Self::from_document(document, source_bytes)
    }

    fn from_document(document: serde_json::Value, source_bytes: u64) -> Result<Self> {
        let metadata: RawMetadata = serde_json::from_value(document.clone())
            .context("decode client resource pack metadata")?;
        if metadata.pack.description.is_null() {
            bail!("client resource pack metadata description cannot be null");
        }
        Ok(Self {
            supported_formats: metadata.pack.supported_format_range()?,
            document: Arc::new(document),
            source_bytes,
        })
    }

    pub(crate) fn document(&self) -> &serde_json::Value {
        &self.document
    }

    pub(crate) fn source_bytes(&self) -> u64 {
        self.source_bytes
    }
}

impl ServerPackFormatRange {
    pub fn supports_current_client(self) -> bool {
        self.min <= CLIENT_RESOURCE_PACK_FORMAT && CLIENT_RESOURCE_PACK_FORMAT <= self.max
    }
}

pub(crate) fn client_resource_pack_format() -> ServerPackFormat {
    CLIENT_RESOURCE_PACK_FORMAT
}

#[derive(Debug, Clone)]
pub struct PreparedPackArchive {
    pub archive_path: PathBuf,
    pub cache_root: PathBuf,
    pub archive_bytes: u64,
    pub entries: usize,
    pub files: usize,
    pub metadata: ClientPackMetadata,
    pub(crate) resources: ZipPackResources,
}

impl PartialEq for PreparedPackArchive {
    fn eq(&self, other: &Self) -> bool {
        self.archive_path == other.archive_path
            && self.cache_root == other.cache_root
            && self.archive_bytes == other.archive_bytes
            && self.entries == other.entries
            && self.files == other.files
            && self.metadata == other.metadata
    }
}

impl Eq for PreparedPackArchive {}

/// Opens a file resource pack the same way as Vanilla's `FileResourcesSupplier`: the ZIP remains
/// the canonical backing store and only `pack.mcmeta` is consumed during repository admission.
/// The cache root is reserved for on-demand path adapters and is not created here.
pub fn prepare_pack_archive(archive_path: &Path, cache_root: &Path) -> Result<PreparedPackArchive> {
    let archive_bytes =
        crate::virtual_store::resource_file_len(archive_path).with_context(|| {
            format!(
                "read resource pack archive metadata {}",
                archive_path.display()
            )
        })?;
    let resources = ZipPackResources::open(archive_path, cache_root)?;
    let metadata_source = resources.entry("pack.mcmeta").with_context(|| {
        format!(
            "resource pack archive {} is missing pack.mcmeta",
            archive_path.display()
        )
    })?;
    let metadata_bytes = metadata_source.read_bounded(
        crate::resource_budget::MAX_RESOURCE_JSON_FILE_BYTES,
        "resource pack metadata",
        archive_path,
    )?;
    let metadata = ClientPackMetadata::parse(&metadata_bytes).with_context(|| {
        format!(
            "parse resource pack metadata from {}",
            archive_path.display()
        )
    })?;
    Ok(PreparedPackArchive {
        archive_path: archive_path.to_path_buf(),
        cache_root: cache_root.to_path_buf(),
        archive_bytes,
        entries: resources.entry_count(),
        files: resources.file_count(),
        metadata,
        resources,
    })
}

/// Opens an unextracted Vanilla client JAR (`26.1.jar` / `client-26.1.jar`), synthesizing the
/// built-in `[84, 0]` `pack` section when `pack.mcmeta` lives in the external asset index.
pub(crate) fn prepare_vanilla_jar_archive(
    archive_path: &Path,
    cache_root: &Path,
    indexed_assets: Option<&crate::asset_index::VanillaAssetIndex>,
) -> Result<PreparedPackArchive> {
    let archive_bytes =
        crate::virtual_store::resource_file_len(archive_path).with_context(|| {
            format!(
                "read Vanilla client JAR metadata {}",
                archive_path.display()
            )
        })?;
    let resources = ZipPackResources::open(archive_path, cache_root)?;
    let version_entry = resources.entry("version.json").with_context(|| {
        format!(
            "Vanilla client JAR {} is missing version.json",
            archive_path.display()
        )
    })?;
    let version_bytes = version_entry.read_bounded(
        crate::resource_budget::MAX_RESOURCE_JSON_FILE_BYTES,
        "Vanilla JAR version.json",
        archive_path,
    )?;
    let version_doc: serde_json::Value = serde_json::from_slice(&version_bytes)
        .with_context(|| format!("parse version.json in {}", archive_path.display()))?;
    if version_doc.get("id").and_then(serde_json::Value::as_str) != Some(crate::MC_VERSION) {
        bail!(
            "Vanilla client JAR {} is not Minecraft {}",
            archive_path.display(),
            crate::MC_VERSION
        );
    }

    let mut document = if let Some(entry) = resources.entry("pack.mcmeta") {
        let bytes = entry.read_bounded(
            crate::resource_budget::MAX_RESOURCE_JSON_FILE_BYTES,
            "Vanilla JAR pack.mcmeta",
            archive_path,
        )?;
        serde_json::from_slice::<serde_json::Value>(&bytes)
            .unwrap_or_else(|_| serde_json::json!({}))
    } else if let Some(indexed) =
        indexed_assets.and_then(|index| index.root_resource("pack.mcmeta"))
    {
        if indexed.path.is_file()
            && indexed
                .integrity
                .verify(&indexed.path, "pack.mcmeta")
                .is_ok()
        {
            crate::resource_budget::read_standalone_json_bytes_and_value(
                &indexed.path,
                "indexed Vanilla pack.mcmeta",
            )
            .map(|(_, value)| value)
            .unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        }
    } else {
        serde_json::json!({})
    };
    if !document.is_object() {
        document = serde_json::json!({});
    }
    let object = document.as_object_mut().expect("object initialized above");
    if !object.get("pack").is_some_and(serde_json::Value::is_object) {
        object.insert(
            "pack".to_string(),
            serde_json::json!({
                "description": "The default data and assets for Minecraft",
                "min_format": [CLIENT_RESOURCE_PACK_FORMAT.major, CLIENT_RESOURCE_PACK_FORMAT.minor],
                "max_format": [CLIENT_RESOURCE_PACK_FORMAT.major, CLIENT_RESOURCE_PACK_FORMAT.minor],
            }),
        );
    }
    let encoded = serde_json::to_vec(&document).context("encode Vanilla JAR pack metadata")?;
    let source_bytes = u64::try_from(encoded.len()).context("pack metadata size overflow")?;
    let metadata = ClientPackMetadata::from_document(document, source_bytes)?;

    Ok(PreparedPackArchive {
        archive_path: archive_path.to_path_buf(),
        cache_root: cache_root.to_path_buf(),
        archive_bytes,
        entries: resources.entry_count(),
        files: resources.file_count(),
        metadata,
        resources,
    })
}

pub fn read_client_resource_pack_format_range(root: &Path) -> Result<ServerPackFormatRange> {
    Ok(read_client_resource_pack_metadata(root)?.supported_formats)
}

pub fn read_client_resource_pack_metadata(root: &Path) -> Result<ClientPackMetadata> {
    let path = root.join("pack.mcmeta");
    let (bytes, document) = crate::resource_budget::read_standalone_json_bytes_and_value(
        &path,
        "client resource pack metadata",
    )
    .with_context(|| format!("read client resource pack metadata {}", path.display()))?;
    let source_bytes = u64::try_from(bytes.len()).context("pack metadata size overflow")?;
    ClientPackMetadata::from_document(document, source_bytes)
        .with_context(|| format!("parse client resource pack metadata {}", path.display()))
}

#[derive(Debug, Deserialize)]
struct RawMetadata {
    pack: RawPackSection,
}

#[derive(Debug, Deserialize)]
struct RawPackSection {
    description: serde_json::Value,
    pack_format: Option<u32>,
    supported_formats: Option<RawMajorFormatRange>,
    min_format: Option<RawPackFormat>,
    max_format: Option<RawPackFormat>,
}

impl RawPackSection {
    fn supported_format_range(&self) -> Result<ServerPackFormatRange> {
        if self.min_format.is_some() != self.max_format.is_some() {
            bail!("client resource pack metadata must declare both min_format and max_format");
        }
        if let (Some(min), Some(max)) = (self.min_format, self.max_format) {
            let min = min.bottom();
            let max = max.top();
            if min > max {
                bail!("client resource pack metadata min_format exceeds max_format");
            }
            if min.major > LAST_PRE_MINOR_CLIENT_RESOURCE_PACK_FORMAT {
                if self.supported_formats.is_some() {
                    bail!("client resource pack metadata supported_formats is deprecated for this range");
                }
            } else {
                let old_range = self.supported_formats.context(
                    "client resource pack metadata range crossing legacy formats requires supported_formats",
                )?;
                if old_range.min() != min.major
                    || !matches!(old_range.max(), value if value == max.major || value == LAST_PRE_MINOR_CLIENT_RESOURCE_PACK_FORMAT)
                {
                    bail!("client resource pack metadata legacy and full format ranges disagree");
                }
                if self.pack_format.is_none() {
                    bail!("client resource pack metadata legacy range requires pack_format");
                }
            }
            validate_main_format(self.pack_format, min.major, max.major)?;
            return Ok(ServerPackFormatRange { min, max });
        }

        if let Some(supported) = self.supported_formats {
            if supported.min() > supported.max() {
                bail!("client resource pack metadata supported_formats minimum exceeds maximum");
            }
            if supported.max() > LAST_PRE_MINOR_CLIENT_RESOURCE_PACK_FORMAT {
                bail!(
                    "client resource pack metadata newer formats require min_format and max_format"
                );
            }
            validate_main_format(self.pack_format, supported.min(), supported.max())?;
            return Ok(ServerPackFormatRange {
                min: ServerPackFormat {
                    major: supported.min(),
                    minor: 0,
                },
                max: ServerPackFormat {
                    major: supported.max(),
                    minor: u32::MAX,
                },
            });
        }

        let pack_format = self
            .pack_format
            .context("client resource pack metadata is missing format version information")?;
        if pack_format > LAST_PRE_MINOR_CLIENT_RESOURCE_PACK_FORMAT {
            bail!("client resource pack metadata newer pack_format requires min_format and max_format");
        }
        Ok(ServerPackFormatRange {
            min: ServerPackFormat {
                major: pack_format,
                minor: 0,
            },
            max: ServerPackFormat {
                major: pack_format,
                minor: u32::MAX,
            },
        })
    }
}

fn validate_main_format(pack_format: Option<u32>, min: u32, max: u32) -> Result<()> {
    if let Some(pack_format) = pack_format {
        if pack_format < min || pack_format > max {
            bail!("client resource pack metadata pack_format is outside its supported range");
        }
        if min != max && min < 15 {
            bail!("client resource pack metadata multi-version range starts before format 15");
        }
    }
    Ok(())
}

// Vanilla `PackFormat.java:25-31`: accepts either a single major integer or a 1..=256 integer list
// (`[major]` uses the default minor; `[major, minor, ...]` uses the first two entries).
#[derive(Debug, Clone, Copy)]
pub(crate) enum RawPackFormat {
    Major(u32),
    Full(u32, u32),
}

impl<'de> Deserialize<'de> for RawPackFormat {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PackFormatVisitor;

        impl<'de> serde::de::Visitor<'de> for PackFormatVisitor {
            type Value = RawPackFormat;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a non-negative pack format integer or 1..=256 element array")
            }

            fn visit_u64<E: serde::de::Error>(
                self,
                value: u64,
            ) -> std::result::Result<Self::Value, E> {
                u32::try_from(value)
                    .map(RawPackFormat::Major)
                    .map_err(|_| E::custom("pack format integer out of u32 range"))
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let Some(major) = seq.next_element::<u32>()? else {
                    return Err(serde::de::Error::custom(
                        "pack format array must contain at least 1 integer",
                    ));
                };
                let minor = seq.next_element::<u32>()?;
                let mut count = if minor.is_some() { 2usize } else { 1usize };
                while seq.next_element::<u32>()?.is_some() {
                    count += 1;
                    if count > 256 {
                        return Err(serde::de::Error::custom(
                            "pack format array exceeds 256 elements",
                        ));
                    }
                }
                Ok(match minor {
                    Some(minor) => RawPackFormat::Full(major, minor),
                    None => RawPackFormat::Major(major),
                })
            }
        }

        deserializer.deserialize_any(PackFormatVisitor)
    }
}

impl RawPackFormat {
    pub(crate) fn bottom(self) -> ServerPackFormat {
        match self {
            Self::Major(major) => ServerPackFormat { major, minor: 0 },
            Self::Full(major, minor) => ServerPackFormat { major, minor },
        }
    }

    pub(crate) fn top(self) -> ServerPackFormat {
        match self {
            Self::Major(major) => ServerPackFormat {
                major,
                minor: u32::MAX,
            },
            Self::Full(major, minor) => ServerPackFormat { major, minor },
        }
    }
}

// Vanilla `ExtraCodecs.java:271-290` (`InclusiveRange.codec(Codec.INT)`): single integer,
// 2-element array, or `{"min_inclusive", "max_inclusive"}` object.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(untagged)]
pub(crate) enum RawMajorFormatRange {
    Major(u32),
    Range([u32; 2]),
    Object {
        min_inclusive: u32,
        max_inclusive: u32,
    },
}

impl RawMajorFormatRange {
    pub(crate) fn min(self) -> u32 {
        match self {
            Self::Major(value) => value,
            Self::Range([min, _]) => min,
            Self::Object { min_inclusive, .. } => min_inclusive,
        }
    }

    pub(crate) fn max(self) -> u32 {
        match self {
            Self::Major(value) => value,
            Self::Range([_, max]) => max,
            Self::Object { max_inclusive, .. } => max_inclusive,
        }
    }

    pub(crate) fn bounds(self) -> (u32, u32) {
        (self.min(), self.max())
    }

    pub(crate) fn contains(self, value: u32) -> bool {
        let (min, max) = self.bounds();
        min <= value && value <= max
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::File,
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };
    use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

    const PACK_METADATA: &str = r#"{
      "pack": {
        "description": "bbb fixture",
        "min_format": [84, 0],
        "max_format": [84, 0]
      }
    }"#;

    #[test]
    fn prepares_archive_provider_without_eager_extraction() {
        let root = unique_temp_dir("direct-provider");
        let archive = root.join("pack.zip");
        write_zip(
            &archive,
            &[
                ("pack.mcmeta", PACK_METADATA.as_bytes()),
                ("assets/minecraft/lang/en_us.json", br#"{"hello":"world"}"#),
            ],
            CompressionMethod::Deflated,
        );
        let cache = root.join("lazy-cache");

        let prepared = prepare_pack_archive(&archive, &cache).unwrap();

        assert_eq!(prepared.archive_path, archive);
        assert_eq!(prepared.cache_root, cache);
        assert_eq!(prepared.entries, 2);
        assert_eq!(prepared.files, 2);
        assert!(prepared
            .metadata
            .supported_formats
            .supports_current_client());
        assert!(prepared.archive_path.is_file());
        assert!(!prepared.cache_root.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parses_single_element_pack_format_arrays_and_inclusive_range_objects() {
        let single_array = ClientPackMetadata::parse(
            br#"{
              "pack": {
                "description": "single array",
                "min_format": [84],
                "max_format": [84]
              }
            }"#,
        )
        .unwrap();
        assert_eq!(
            single_array.supported_formats,
            ServerPackFormatRange {
                min: ServerPackFormat {
                    major: 84,
                    minor: 0
                },
                max: ServerPackFormat {
                    major: 84,
                    minor: u32::MAX,
                },
            }
        );
        assert!(single_array.supported_formats.supports_current_client());

        let legacy_object_range = ClientPackMetadata::parse(
            br#"{
              "pack": {
                "description": "legacy inclusive range object",
                "pack_format": 34,
                "supported_formats": {"min_inclusive": 34, "max_inclusive": 64},
                "min_format": [34],
                "max_format": [84, 0]
              }
            }"#,
        )
        .unwrap();
        assert!(legacy_object_range
            .supported_formats
            .supports_current_client());
        assert_eq!(
            legacy_object_range.supported_formats.min,
            ServerPackFormat {
                major: 34,
                minor: 0
            }
        );
        assert_eq!(
            legacy_object_range.supported_formats.max,
            ServerPackFormat {
                major: 84,
                minor: 0
            }
        );

        assert!(ClientPackMetadata::parse(
            br#"{
              "pack": {
                "description": "empty format array",
                "min_format": [],
                "max_format": [84]
              }
            }"#
        )
        .is_err());
    }

    fn write_zip(path: &Path, entries: &[(&str, &[u8])], compression: CompressionMethod) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(compression);
        for (name, contents) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "bbb-server-pack-{label}-{}-{nanos}",
            std::process::id()
        ))
    }
}
