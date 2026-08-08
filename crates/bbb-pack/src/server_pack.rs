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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPackArchive {
    pub archive_path: PathBuf,
    pub cache_root: PathBuf,
    pub archive_bytes: u64,
    pub entries: usize,
    pub files: usize,
    pub metadata: ClientPackMetadata,
}

/// Opens a file resource pack the same way as Vanilla's `FileResourcesSupplier`: the ZIP remains
/// the canonical backing store and only `pack.mcmeta` is consumed during repository admission.
/// The cache root is reserved for on-demand path adapters and is not created here.
pub fn prepare_pack_archive(archive_path: &Path, cache_root: &Path) -> Result<PreparedPackArchive> {
    let archive_bytes = std::fs::metadata(archive_path)
        .with_context(|| {
            format!(
                "read resource pack archive metadata {}",
                archive_path.display()
            )
        })?
        .len();
    let resources = ZipPackResources::open(archive_path, cache_root)?;
    let metadata_source = resources.entry("pack.mcmeta").with_context(|| {
        format!(
            "resource pack archive {} is missing pack.mcmeta",
            archive_path.display()
        )
    })?;
    let metadata = ClientPackMetadata::parse(&metadata_source.read()?).with_context(|| {
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
    supported_formats: Option<RawMajorRange>,
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

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(untagged)]
enum RawPackFormat {
    Major(u32),
    Full([u32; 2]),
}

impl RawPackFormat {
    fn bottom(self) -> ServerPackFormat {
        match self {
            Self::Major(major) => ServerPackFormat { major, minor: 0 },
            Self::Full([major, minor]) => ServerPackFormat { major, minor },
        }
    }

    fn top(self) -> ServerPackFormat {
        match self {
            Self::Major(major) => ServerPackFormat {
                major,
                minor: u32::MAX,
            },
            Self::Full([major, minor]) => ServerPackFormat { major, minor },
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(untagged)]
enum RawMajorRange {
    Major(u32),
    Range([u32; 2]),
}

impl RawMajorRange {
    fn min(self) -> u32 {
        match self {
            Self::Major(value) => value,
            Self::Range([min, _]) => min,
        }
    }

    fn max(self) -> u32 {
        match self {
            Self::Major(value) => value,
            Self::Range([_, max]) => max,
        }
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
