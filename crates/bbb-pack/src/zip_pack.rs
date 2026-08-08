use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use anyhow::{bail, Context, Result};
use sha1::{Digest, Sha1};
use zip::ZipArchive;

const CENTRAL_DIRECTORY_HEADER_SIGNATURE: u32 = 0x0201_4b50;
const CENTRAL_DIRECTORY_FIXED_BYTES: usize = 46;
static MATERIALIZATION_SERIAL: AtomicU64 = AtomicU64::new(0);

/// Repo-native equivalent of Vanilla's shared `FilePackResources` ZIP access.
///
/// The archive stays open for the resource generation. Its central directory is retained in
/// archive order (including duplicate names), while exact lookup resolves the last duplicate in
/// the same way as the JDK `ZipFile` hash-chain head. Bytes are copied into a flat cache only when
/// an existing path-based BBB consumer opens that resource; admitting or listing a pack never
/// extracts the whole archive.
#[derive(Debug, Clone)]
pub(crate) struct ZipPackResources {
    inner: Arc<ZipPackInner>,
}

#[derive(Debug)]
struct ZipPackInner {
    archive_path: PathBuf,
    cache_root: PathBuf,
    entries: Vec<CentralEntry>,
    lookup: HashMap<String, usize>,
    archive: Mutex<ZipArchive<File>>,
}

#[derive(Debug, Clone)]
struct CentralEntry {
    name: String,
    directory: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ZipEntrySource {
    resources: ZipPackResources,
    name: String,
    archive_index: usize,
    target: PathBuf,
}

impl PartialEq for ZipEntrySource {
    fn eq(&self, other: &Self) -> bool {
        self.resources.inner.archive_path == other.resources.inner.archive_path
            && self.resources.inner.cache_root == other.resources.inner.cache_root
            && self.name == other.name
            && self.archive_index == other.archive_index
    }
}

impl Eq for ZipEntrySource {}

impl ZipPackResources {
    pub(crate) fn open(archive_path: &Path, cache_root: &Path) -> Result<Self> {
        let file = File::open(archive_path)
            .with_context(|| format!("open resource pack ZIP {}", archive_path.display()))?;
        let mut archive = ZipArchive::new(file)
            .with_context(|| format!("parse resource pack ZIP {}", archive_path.display()))?;
        let central_start = archive.central_directory_start();

        // `zip` intentionally folds duplicate names into one IndexMap entry. Retain the archive
        // index of the final entry at each central-header offset so the raw central scan below can
        // restore duplicate enumeration without reimplementing ZIP decompression.
        let mut archive_index_by_central_offset = HashMap::with_capacity(archive.len());
        for archive_index in 0..archive.len() {
            let entry = archive
                .by_index(archive_index)
                .with_context(|| format!("inspect resource pack ZIP entry {archive_index}"))?;
            archive_index_by_central_offset.insert(entry.central_header_start(), archive_index);
        }

        let central_entries = read_central_entries(archive_path, central_start)?;
        let mut lookup = HashMap::new();
        let mut entries = Vec::with_capacity(central_entries.len());
        for raw in central_entries {
            // The default JDK ZipFile constructor used by Vanilla decodes names as UTF-8. Reject
            // names it cannot open instead of accepting the zip crate's CP437 fallback.
            let name = String::from_utf8(raw.name).with_context(|| {
                format!(
                    "resource pack ZIP {} has a non-UTF-8 entry name",
                    archive_path.display()
                )
            })?;
            let directory = name.ends_with('/');
            entries.push(CentralEntry {
                name: name.clone(),
                directory,
            });
            if let Some(&archive_index) =
                archive_index_by_central_offset.get(&raw.central_header_start)
            {
                // Central entries are visited in order, so a duplicate overwrites the preceding
                // value exactly as JDK ZipFile's newest hash-chain node does.
                lookup.insert(name, archive_index);
            }
        }

        // Every final name must be represented by the decompressor's retained entry. A mismatch
        // would otherwise make lookup silently select different bytes than Vanilla.
        for entry in &entries {
            if !lookup.contains_key(&entry.name) {
                bail!(
                    "resource pack ZIP {} has a central-directory name the ZIP reader cannot address",
                    archive_path.display()
                );
            }
        }

        Ok(Self {
            inner: Arc::new(ZipPackInner {
                archive_path: archive_path.to_path_buf(),
                cache_root: cache_root.to_path_buf(),
                entries,
                lookup,
                archive: Mutex::new(archive),
            }),
        })
    }

    pub(crate) fn entry_count(&self) -> usize {
        self.inner.entries.len()
    }

    pub(crate) fn file_count(&self) -> usize {
        self.inner
            .entries
            .iter()
            .filter(|entry| !entry.directory)
            .count()
    }

    pub(crate) fn entry(&self, name: &str) -> Option<ZipEntrySource> {
        let (resolved_name, archive_index) = self
            .inner
            .lookup
            .get(name)
            .map(|&index| (name.to_owned(), index))
            .or_else(|| {
                let directory_name = format!("{name}/");
                self.inner
                    .lookup
                    .get(&directory_name)
                    .map(|&index| (directory_name, index))
            })?;
        Some(self.source(resolved_name, archive_index))
    }

    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        self.inner.entries.iter().map(|entry| entry.name.as_str())
    }

    fn source(&self, name: String, archive_index: usize) -> ZipEntrySource {
        let target = self.inner.cache_root.join(cache_file_name(&name));
        ZipEntrySource {
            resources: self.clone(),
            name,
            archive_index,
            target,
        }
    }
}

impl ZipEntrySource {
    pub(crate) fn target(&self) -> &Path {
        &self.target
    }

    pub(crate) fn read(&self) -> Result<Vec<u8>> {
        let mut archive = self
            .resources
            .inner
            .archive
            .lock()
            .map_err(|_| anyhow::anyhow!("resource pack ZIP reader lock poisoned"))?;
        let mut entry = archive.by_index(self.archive_index).with_context(|| {
            format!(
                "open ZIP resource {} from {}",
                self.name,
                self.resources.inner.archive_path.display()
            )
        })?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).with_context(|| {
            format!(
                "read ZIP resource {} from {}",
                self.name,
                self.resources.inner.archive_path.display()
            )
        })?;
        Ok(bytes)
    }

    pub(crate) fn materialize(&self) -> Result<&Path> {
        if self.target.is_file() {
            return Ok(&self.target);
        }
        std::fs::create_dir_all(&self.resources.inner.cache_root).with_context(|| {
            format!(
                "create lazy resource pack cache {}",
                self.resources.inner.cache_root.display()
            )
        })?;

        let serial = MATERIALIZATION_SERIAL.fetch_add(1, Ordering::Relaxed);
        let temporary = self
            .resources
            .inner
            .cache_root
            .join(format!(".tmp-{serial:020}"));
        let result = (|| -> Result<()> {
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .with_context(|| format!("create lazy ZIP resource {}", temporary.display()))?;
            let mut archive = self
                .resources
                .inner
                .archive
                .lock()
                .map_err(|_| anyhow::anyhow!("resource pack ZIP reader lock poisoned"))?;
            let mut entry = archive.by_index(self.archive_index).with_context(|| {
                format!(
                    "open ZIP resource {} from {}",
                    self.name,
                    self.resources.inner.archive_path.display()
                )
            })?;
            std::io::copy(&mut entry, &mut output).with_context(|| {
                format!(
                    "materialize ZIP resource {} from {}",
                    self.name,
                    self.resources.inner.archive_path.display()
                )
            })?;
            output
                .flush()
                .with_context(|| format!("flush lazy ZIP resource {}", temporary.display()))?;
            drop(output);
            if self.target.is_file() {
                std::fs::remove_file(&temporary).with_context(|| {
                    format!("remove duplicate lazy ZIP resource {}", temporary.display())
                })?;
            } else {
                std::fs::rename(&temporary, &self.target).with_context(|| {
                    format!(
                        "publish lazy ZIP resource {} as {}",
                        temporary.display(),
                        self.target.display()
                    )
                })?;
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result?;
        Ok(&self.target)
    }
}

#[derive(Debug)]
struct RawCentralEntry {
    central_header_start: u64,
    name: Vec<u8>,
}

fn read_central_entries(path: &Path, central_start: u64) -> Result<Vec<RawCentralEntry>> {
    let mut file = File::open(path)
        .with_context(|| format!("open resource pack central directory {}", path.display()))?;
    file.seek(SeekFrom::Start(central_start))
        .with_context(|| format!("seek resource pack central directory {}", path.display()))?;
    let mut entries = Vec::new();
    loop {
        let central_header_start = file
            .stream_position()
            .with_context(|| format!("locate resource pack central entry {}", path.display()))?;
        let mut signature = [0_u8; 4];
        file.read_exact(&mut signature)
            .with_context(|| format!("read resource pack central signature {}", path.display()))?;
        if u32::from_le_bytes(signature) != CENTRAL_DIRECTORY_HEADER_SIGNATURE {
            break;
        }
        let mut fixed = [0_u8; CENTRAL_DIRECTORY_FIXED_BYTES - 4];
        file.read_exact(&mut fixed)
            .with_context(|| format!("read resource pack central header {}", path.display()))?;
        let name_len = usize::from(u16::from_le_bytes([fixed[24], fixed[25]]));
        let extra_len = u64::from(u16::from_le_bytes([fixed[26], fixed[27]]));
        let comment_len = u64::from(u16::from_le_bytes([fixed[28], fixed[29]]));
        let mut name = vec![0_u8; name_len];
        file.read_exact(&mut name)
            .with_context(|| format!("read resource pack central name {}", path.display()))?;
        let skip = extra_len
            .checked_add(comment_len)
            .context("resource pack central variable fields overflow")?;
        file.seek(SeekFrom::Current(
            i64::try_from(skip).context("resource pack central variable fields are too large")?,
        ))
        .with_context(|| format!("skip resource pack central fields {}", path.display()))?;
        entries.push(RawCentralEntry {
            central_header_start,
            name,
        });
    }
    Ok(entries)
}

fn cache_file_name(name: &str) -> String {
    let mut sha1 = Sha1::new();
    sha1.update(name.as_bytes());
    let digest = format!("{:x}", sha1.finalize());
    let extension = Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| {
            !extension.is_empty()
                && extension.len() <= 16
                && extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
        });
    match extension {
        Some(extension) => format!("{digest}.{extension}"),
        None => digest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, time::SystemTime};
    use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

    #[test]
    fn duplicate_lookup_uses_last_entry_while_listing_keeps_central_order() {
        let root = unique_temp_dir("duplicate-order");
        std::fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("pack.zip");
        let file = File::create(&archive_path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        writer
            .start_file("assets/minecraft/test-a.txt", options)
            .unwrap();
        writer.write_all(b"first").unwrap();
        writer
            .start_file("assets/minecraft/test-b.txt", options)
            .unwrap();
        writer.write_all(b"last").unwrap();
        writer
            .start_file("assets/minecraft/other.txt", options)
            .unwrap();
        writer.write_all(b"other").unwrap();
        writer.finish().unwrap();
        let mut bytes = std::fs::read(&archive_path).unwrap();
        for offset in 0..=bytes.len() - b"test-b".len() {
            if &bytes[offset..offset + b"test-b".len()] == b"test-b" {
                bytes[offset..offset + b"test-a".len()].copy_from_slice(b"test-a");
            }
        }
        std::fs::write(&archive_path, bytes).unwrap();

        let resources = ZipPackResources::open(&archive_path, &root.join("cache")).unwrap();
        assert_eq!(resources.entry_count(), 3);
        assert_eq!(
            resources.names().collect::<Vec<_>>(),
            vec![
                "assets/minecraft/test-a.txt",
                "assets/minecraft/test-a.txt",
                "assets/minecraft/other.txt"
            ]
        );
        assert_eq!(
            resources
                .entry("assets/minecraft/test-a.txt")
                .unwrap()
                .read()
                .unwrap(),
            b"last"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn materialization_is_flat_lazy_and_preserves_useful_extension() {
        let root = unique_temp_dir("lazy");
        std::fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("pack.zip");
        let file = File::create(&archive_path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        writer.start_file("pack.mcmeta", options).unwrap();
        writer.write_all(b"metadata").unwrap();
        writer
            .start_file("assets/minecraft/textures/a.png", options)
            .unwrap();
        writer.write_all(b"png").unwrap();
        writer.finish().unwrap();

        let cache = root.join("cache");
        let resources = ZipPackResources::open(&archive_path, &cache).unwrap();
        assert!(!cache.exists());
        let metadata = resources.entry("pack.mcmeta").unwrap();
        let path = metadata.materialize().unwrap();
        assert_eq!(
            path.extension().and_then(|value| value.to_str()),
            Some("mcmeta")
        );
        assert_eq!(std::fs::read(path).unwrap(), b"metadata");
        assert_eq!(std::fs::read_dir(&cache).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-zip-pack-{label}-{nanos}"))
    }
}
