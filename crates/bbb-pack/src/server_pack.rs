use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use zip::ZipArchive;

const CLIENT_RESOURCE_PACK_FORMAT: ServerPackFormat = ServerPackFormat {
    major: 84,
    minor: 0,
};
const LAST_PRE_MINOR_CLIENT_RESOURCE_PACK_FORMAT: u32 = 64;
static STAGING_SERIAL: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerPackArchiveLimits {
    pub max_archive_bytes: u64,
    pub max_entries: usize,
    pub max_path_bytes: usize,
    pub max_file_uncompressed_bytes: u64,
    pub max_total_uncompressed_bytes: u64,
    pub max_compression_ratio: u64,
}

impl Default for ServerPackArchiveLimits {
    fn default() -> Self {
        Self {
            // Vanilla `DownloadedPackSource` caps a server pack download at 250 MiB.
            max_archive_bytes: 250 * 1024 * 1024,
            max_entries: 65_536,
            max_path_bytes: 1_024,
            max_file_uncompressed_bytes: 64 * 1024 * 1024,
            max_total_uncompressed_bytes: 512 * 1024 * 1024,
            max_compression_ratio: 200,
        }
    }
}

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

impl ServerPackFormatRange {
    pub fn supports_current_client(self) -> bool {
        self.min <= CLIENT_RESOURCE_PACK_FORMAT && CLIENT_RESOURCE_PACK_FORMAT <= self.max
    }
}

pub(crate) fn client_resource_pack_format() -> ServerPackFormat {
    CLIENT_RESOURCE_PACK_FORMAT
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerPackMaterialization {
    pub root: PathBuf,
    pub archive_bytes: u64,
    pub entries: usize,
    pub files: usize,
    pub uncompressed_bytes: u64,
    pub supported_formats: ServerPackFormatRange,
}

#[derive(Debug)]
struct ArchiveEntryPlan {
    index: usize,
    relative_path: PathBuf,
    directory: bool,
    uncompressed_bytes: u64,
}

/// Materializes one downloaded server-pack ZIP into a new directory.
///
/// The destination must not already exist. Extraction first validates every entry and declared
/// size, writes into a private sibling directory, validates `pack.mcmeta`, and only then renames
/// the directory into place. A rejected archive therefore never leaves a selectable partial pack.
pub fn materialize_server_pack_archive(
    archive_path: &Path,
    output_dir: &Path,
    limits: ServerPackArchiveLimits,
) -> Result<ServerPackMaterialization> {
    validate_limits(limits)?;
    if output_dir.exists() {
        bail!(
            "server pack output already exists: {}",
            output_dir.display()
        );
    }
    let archive_bytes = std::fs::metadata(archive_path)
        .with_context(|| {
            format!(
                "read server pack archive metadata {}",
                archive_path.display()
            )
        })?
        .len();
    if archive_bytes > limits.max_archive_bytes {
        bail!(
            "server pack archive is {archive_bytes} bytes, exceeding the {} byte limit",
            limits.max_archive_bytes
        );
    }

    let file = File::open(archive_path)
        .with_context(|| format!("open server pack archive {}", archive_path.display()))?;
    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("parse server pack archive {}", archive_path.display()))?;
    let plans = validate_archive_entries(&mut archive, limits)?;
    let uncompressed_bytes = plans.iter().try_fold(0_u64, |total, entry| {
        total
            .checked_add(entry.uncompressed_bytes)
            .context("server pack uncompressed size overflow")
    })?;
    let files = plans.iter().filter(|entry| !entry.directory).count();

    let staging_dir = staging_path(output_dir)?;
    std::fs::create_dir(&staging_dir).with_context(|| {
        format!(
            "create server pack staging directory {}",
            staging_dir.display()
        )
    })?;
    let mut cleanup = StagingCleanup::new(staging_dir.clone());
    extract_archive_entries(&mut archive, &plans, &staging_dir, limits)?;
    let supported_formats = read_client_resource_pack_format_range(&staging_dir)?;
    std::fs::rename(&staging_dir, output_dir).with_context(|| {
        format!(
            "publish server pack staging directory {} as {}",
            staging_dir.display(),
            output_dir.display()
        )
    })?;
    cleanup.disarm();

    Ok(ServerPackMaterialization {
        root: output_dir.to_path_buf(),
        archive_bytes,
        entries: plans.len(),
        files,
        uncompressed_bytes,
        supported_formats,
    })
}

fn validate_limits(limits: ServerPackArchiveLimits) -> Result<()> {
    if limits.max_archive_bytes == 0
        || limits.max_entries == 0
        || limits.max_path_bytes == 0
        || limits.max_file_uncompressed_bytes == 0
        || limits.max_total_uncompressed_bytes == 0
        || limits.max_compression_ratio == 0
    {
        bail!("server pack archive limits must all be positive");
    }
    Ok(())
}

fn validate_archive_entries(
    archive: &mut ZipArchive<File>,
    limits: ServerPackArchiveLimits,
) -> Result<Vec<ArchiveEntryPlan>> {
    if archive.len() > limits.max_entries {
        bail!(
            "server pack has {} entries, exceeding the {} entry limit",
            archive.len(),
            limits.max_entries
        );
    }

    let mut paths = BTreeMap::<PathBuf, bool>::new();
    let mut plans = Vec::with_capacity(archive.len());
    let mut total_uncompressed_bytes = 0_u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .with_context(|| format!("read server pack ZIP entry {index}"))?;
        let relative_path = validate_archive_entry_path(entry.name_raw(), limits.max_path_bytes)
            .with_context(|| format!("validate server pack ZIP entry {index}"))?;
        let directory = entry.is_dir();
        validate_archive_entry_kind(entry.unix_mode(), directory, &relative_path)?;
        let uncompressed_bytes = entry.size();
        if directory && uncompressed_bytes != 0 {
            bail!(
                "server pack directory entry {} has a nonzero body",
                relative_path.display()
            );
        }
        if uncompressed_bytes > limits.max_file_uncompressed_bytes {
            bail!(
                "server pack entry {} is {uncompressed_bytes} bytes, exceeding the {} byte per-file limit",
                relative_path.display(),
                limits.max_file_uncompressed_bytes
            );
        }
        total_uncompressed_bytes = total_uncompressed_bytes
            .checked_add(uncompressed_bytes)
            .context("server pack uncompressed size overflow")?;
        if total_uncompressed_bytes > limits.max_total_uncompressed_bytes {
            bail!(
                "server pack declares {total_uncompressed_bytes} uncompressed bytes, exceeding the {} byte total limit",
                limits.max_total_uncompressed_bytes
            );
        }
        validate_compression_ratio(
            &relative_path,
            entry.compressed_size(),
            uncompressed_bytes,
            limits.max_compression_ratio,
        )?;
        if let Some(previous_directory) = paths.insert(relative_path.clone(), directory) {
            if !(previous_directory && directory) {
                bail!(
                    "server pack contains duplicate file path {}",
                    relative_path.display()
                );
            }
        }
        plans.push(ArchiveEntryPlan {
            index,
            relative_path,
            directory,
            uncompressed_bytes,
        });
    }
    Ok(plans)
}

fn validate_archive_entry_path(raw_name: &[u8], max_path_bytes: usize) -> Result<PathBuf> {
    if raw_name.is_empty() || raw_name.len() > max_path_bytes {
        bail!("server pack ZIP entry path has invalid length");
    }
    let name = std::str::from_utf8(raw_name).context("server pack ZIP entry path is not UTF-8")?;
    if name.starts_with('/')
        || name.contains('\\')
        || name.chars().any(|character| {
            character.is_control() || matches!(character, ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
    {
        bail!("unsafe server pack ZIP entry path {name:?}");
    }

    let mut path = PathBuf::new();
    let mut saw_component = false;
    let name_without_directory_marker = name.strip_suffix('/').unwrap_or(name);
    for component in name_without_directory_marker.split('/') {
        if component.is_empty() || matches!(component, "." | "..") {
            bail!("unsafe server pack ZIP entry path {name:?}");
        }
        saw_component = true;
        path.push(component);
    }
    if !saw_component {
        bail!("empty server pack ZIP entry path");
    }
    Ok(path)
}

fn validate_archive_entry_kind(unix_mode: Option<u32>, directory: bool, path: &Path) -> Result<()> {
    let Some(mode) = unix_mode else {
        return Ok(());
    };
    let file_type = mode & 0o170_000;
    let expected = if directory { 0o040_000 } else { 0o100_000 };
    if file_type != 0 && file_type != expected {
        bail!(
            "server pack ZIP entry {} is not a regular file or directory",
            path.display()
        );
    }
    Ok(())
}

fn validate_compression_ratio(
    path: &Path,
    compressed_bytes: u64,
    uncompressed_bytes: u64,
    max_ratio: u64,
) -> Result<()> {
    if uncompressed_bytes == 0 {
        return Ok(());
    }
    let allowed = compressed_bytes
        .max(1)
        .checked_mul(max_ratio)
        .unwrap_or(u64::MAX);
    if uncompressed_bytes > allowed {
        bail!(
            "server pack ZIP entry {} exceeds the {max_ratio}:1 compression ratio limit",
            path.display()
        );
    }
    Ok(())
}

fn extract_archive_entries(
    archive: &mut ZipArchive<File>,
    plans: &[ArchiveEntryPlan],
    staging_dir: &Path,
    limits: ServerPackArchiveLimits,
) -> Result<()> {
    let mut buffer = [0_u8; 64 * 1024];
    for plan in plans {
        let output_path = staging_dir.join(&plan.relative_path);
        if plan.directory {
            std::fs::create_dir_all(&output_path).with_context(|| {
                format!("create server pack directory {}", output_path.display())
            })?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("create server pack parent directory {}", parent.display())
            })?;
        }
        let mut input = archive.by_index(plan.index).with_context(|| {
            format!(
                "reopen server pack ZIP entry {}",
                plan.relative_path.display()
            )
        })?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output_path)
            .with_context(|| format!("create server pack file {}", output_path.display()))?;
        let mut written = 0_u64;
        loop {
            let read = input
                .read(&mut buffer)
                .with_context(|| format!("extract server pack file {}", output_path.display()))?;
            if read == 0 {
                break;
            }
            written = written
                .checked_add(read as u64)
                .context("server pack extracted size overflow")?;
            if written > plan.uncompressed_bytes || written > limits.max_file_uncompressed_bytes {
                bail!(
                    "server pack file {} exceeded its validated size during extraction",
                    output_path.display()
                );
            }
            output
                .write_all(&buffer[..read])
                .with_context(|| format!("write server pack file {}", output_path.display()))?;
        }
        if written != plan.uncompressed_bytes {
            bail!(
                "server pack file {} extracted {written} bytes, expected {}",
                output_path.display(),
                plan.uncompressed_bytes
            );
        }
    }
    Ok(())
}

pub fn read_client_resource_pack_format_range(root: &Path) -> Result<ServerPackFormatRange> {
    let path = root.join("pack.mcmeta");
    let bytes = std::fs::read(&path)
        .with_context(|| format!("read client resource pack metadata {}", path.display()))?;
    let metadata: RawMetadata = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse client resource pack metadata {}", path.display()))?;
    if metadata.pack.description.is_null() {
        bail!("client resource pack metadata description cannot be null");
    }
    metadata.pack.supported_format_range()
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

fn staging_path(output_dir: &Path) -> Result<PathBuf> {
    let parent = output_dir.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)
        .with_context(|| format!("create server pack cache directory {}", parent.display()))?;
    let file_name = output_dir
        .file_name()
        .context("server pack output directory has no final component")?
        .to_string_lossy();
    for _ in 0..32 {
        let serial = STAGING_SERIAL.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            ".{file_name}.partial-{}-{serial}",
            std::process::id()
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    bail!("could not allocate a unique server pack staging directory")
}

struct StagingCleanup {
    path: Option<PathBuf>,
}

impl StagingCleanup {
    fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    fn disarm(&mut self) {
        self.path = None;
    }
}

impl Drop for StagingCleanup {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
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
    fn materializes_valid_pack_into_atomic_directory() {
        let root = unique_temp_dir("valid");
        let archive = root.join("pack.zip");
        write_zip(
            &archive,
            &[
                ("pack.mcmeta", PACK_METADATA.as_bytes()),
                ("assets/minecraft/lang/en_us.json", br#"{"hello":"world"}"#),
            ],
            CompressionMethod::Deflated,
        );
        let output = root.join("materialized");

        let result =
            materialize_server_pack_archive(&archive, &output, ServerPackArchiveLimits::default())
                .unwrap();

        assert_eq!(result.root, output);
        assert_eq!(result.files, 2);
        assert_eq!(result.entries, 2);
        assert!(result.supported_formats.supports_current_client());
        assert_eq!(
            std::fs::read_to_string(output.join("assets/minecraft/lang/en_us.json")).unwrap(),
            r#"{"hello":"world"}"#
        );
        assert!(!root.read_dir().unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("partial")));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_parent_traversal_and_removes_staging_directory() {
        let root = unique_temp_dir("traversal");
        let archive = root.join("pack.zip");
        write_zip(
            &archive,
            &[
                ("pack.mcmeta", PACK_METADATA.as_bytes()),
                ("../escaped.txt", b"escape"),
            ],
            CompressionMethod::Stored,
        );
        let output = root.join("materialized");

        let error =
            materialize_server_pack_archive(&archive, &output, ServerPackArchiveLimits::default())
                .unwrap_err();

        assert!(
            format!("{error:#}").contains("unsafe server pack ZIP entry path"),
            "unexpected error: {error:#}"
        );
        assert!(!output.exists());
        assert!(!root.join("escaped.txt").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_zip_bomb_ratio_before_extraction() {
        let root = unique_temp_dir("ratio");
        let archive = root.join("pack.zip");
        let repeated = vec![0_u8; 32 * 1024];
        write_zip(
            &archive,
            &[
                ("pack.mcmeta", PACK_METADATA.as_bytes()),
                ("assets/minecraft/lang/repeated.json", &repeated),
            ],
            CompressionMethod::Deflated,
        );
        let output = root.join("materialized");
        let limits = ServerPackArchiveLimits {
            max_compression_ratio: 2,
            ..ServerPackArchiveLimits::default()
        };

        let error = materialize_server_pack_archive(&archive, &output, limits).unwrap_err();

        assert!(error.to_string().contains("compression ratio limit"));
        assert!(!output.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_missing_or_invalid_pack_metadata() {
        let root = unique_temp_dir("metadata");
        let missing_archive = root.join("missing.zip");
        write_zip(
            &missing_archive,
            &[("assets/minecraft/lang/en_us.json", b"{}")],
            CompressionMethod::Stored,
        );
        let missing_output = root.join("missing-output");
        let missing = materialize_server_pack_archive(
            &missing_archive,
            &missing_output,
            ServerPackArchiveLimits::default(),
        )
        .unwrap_err();
        assert!(missing
            .to_string()
            .contains("read client resource pack metadata"));
        assert!(!missing_output.exists());

        let invalid_archive = root.join("invalid.zip");
        write_zip(
            &invalid_archive,
            &[(
                "pack.mcmeta",
                br#"{"pack":{"description":"bad","pack_format":84}}"#,
            )],
            CompressionMethod::Stored,
        );
        let invalid_output = root.join("invalid-output");
        let invalid = materialize_server_pack_archive(
            &invalid_archive,
            &invalid_output,
            ServerPackArchiveLimits::default(),
        )
        .unwrap_err();
        assert!(invalid
            .to_string()
            .contains("newer pack_format requires min_format and max_format"));
        assert!(!invalid_output.exists());
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
