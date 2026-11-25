use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs::{File, OpenOptions},
    io::{self, BufReader, Cursor, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock, Weak,
    },
};

use anyhow::{bail, Context, Result};
use crc32fast::Hasher as Crc32Hasher;
use sha1::{Digest, Sha1};

use crate::virtual_store::{
    get_virtual_file, has_virtual_file, insert_virtual_file, is_virtual_path,
};

const LOCAL_FILE_HEADER_SIGNATURE: u32 = 0x0403_4b50;
const CENTRAL_DIRECTORY_HEADER_SIGNATURE: u32 = 0x0201_4b50;
const EOCD_SIGNATURE: u32 = 0x0605_4b50;
const ZIP64_EOCD_LOCATOR_SIGNATURE: u32 = 0x0706_4b50;
const ZIP64_EOCD_SIGNATURE: u32 = 0x0606_4b50;
const LOCAL_FILE_HEADER_FIXED_BYTES: usize = 30;
const CENTRAL_DIRECTORY_FIXED_BYTES: usize = 46;
const EOCD_MIN_BYTES: usize = 22;
const EOCD_MAX_COMMENT_BYTES: usize = 65_535;
const ZIP64_LOCATOR_BYTES: usize = 20;
const ZIP64_EOCD_MIN_BYTES: usize = 56;
const ZIP64_EXTRA_HEADER_ID: u16 = 0x0001;
const MAX_POOLED_ZIP_FILES: usize = 8;
pub(crate) const MAX_ZIP_ENTRY_UNCOMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_ZIP_ARCHIVE_MATERIALIZED_BYTES: u64 = 256 * 1024 * 1024;

static MATERIALIZATION_SERIAL: AtomicU64 = AtomicU64::new(0);

/// Direct single-index ZIP resource reader with a bounded reusable file-handle pool.
///
/// The central directory is parsed once into a single compact `BTreeMap<Box<str>, ZipEntryMeta>`.
/// Duplicate entry names resolve to the last central-directory header (matching JDK `ZipFile`
/// hash-chain semantics) while preserving duplicate counts for admission budgets.
#[derive(Debug, Clone)]
pub(crate) struct ZipPackResources {
    inner: Arc<ZipPackInner>,
}

#[derive(Debug)]
enum ZipBacking {
    Memory(Arc<[u8]>),
    File { len: u64, files: Mutex<Vec<File>> },
}

struct PooledZipFile<'a> {
    pool: &'a Mutex<Vec<File>>,
    file: Option<File>,
}

impl<'a> PooledZipFile<'a> {
    fn checkout(pool: &'a Mutex<Vec<File>>, archive_path: &Path) -> Result<Self> {
        let existing = pool
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pop();
        let file = match existing {
            Some(file) => file,
            None => File::open(archive_path)
                .with_context(|| format!("open resource pack ZIP {}", archive_path.display()))?,
        };
        Ok(Self {
            pool,
            file: Some(file),
        })
    }

    fn file_mut(&mut self) -> &mut File {
        self.file.as_mut().expect("pooled ZIP file is checked out")
    }
}

impl Drop for PooledZipFile<'_> {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let mut pool = self
                .pool
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if pool.len() < MAX_POOLED_ZIP_FILES {
                pool.push(file);
            }
        }
    }
}

#[derive(Debug)]
struct ZipPackInner {
    archive_path: PathBuf,
    cache_root: PathBuf,
    backing: ZipBacking,
    entry_count: usize,
    file_count: usize,
    entries: BTreeMap<Box<str>, ZipEntryMeta>,
    materialized_bytes: AtomicU64,
}

impl ZipPackInner {
    fn reserve_materialized_bytes(
        &self,
        bytes: u64,
        max_archive_bytes: u64,
        name: &str,
    ) -> Result<()> {
        self.materialized_bytes
            .try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(bytes)
                    .filter(|&next| next <= max_archive_bytes)
            })
            .map(|_| ())
            .map_err(|used| {
                anyhow::anyhow!(
                    "materializing ZIP resource {name} from {} ({bytes} bytes) would exceed the {max_archive_bytes} byte archive materialization limit (already materialized {used} bytes)",
                    self.archive_path.display()
                )
            })
    }
}

#[derive(Debug, Clone, Copy)]
struct ZipEntryMeta {
    local_header_offset: u64,
    compressed_size: u64,
    uncompressed_size: u64,
    crc32: u32,
    compression_method: u16,
    encrypted: bool,
    directory: bool,
    central_duplicates: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct ZipEntrySource {
    resources: ZipPackResources,
    name: Box<str>,
    meta: ZipEntryMeta,
    target: PathBuf,
}

impl PartialEq for ZipEntrySource {
    fn eq(&self, other: &Self) -> bool {
        self.resources.inner.archive_path == other.resources.inner.archive_path
            && self.resources.inner.cache_root == other.resources.inner.cache_root
            && self.name == other.name
            && self.meta.local_header_offset == other.meta.local_header_offset
    }
}

impl Eq for ZipEntrySource {}

pub(crate) struct ZipEntryReader<'r, 's> {
    uncompressed_size: u64,
    inner: ZipEntryReaderInner<'r, 's>,
}

type DeflatedFileTake<'s> = io::Take<BufReader<&'s mut File>>;
type DeflatedZipFile<'r, R> =
    zip::read::ZipFile<'r, io::Chain<Cursor<[u8; LOCAL_FILE_HEADER_FIXED_BYTES]>, R>>;

enum ZipEntryReaderInner<'r, 's> {
    StoredSlice {
        remaining: &'s [u8],
        hasher: Crc32Hasher,
        expected_crc32: u32,
        expected_len: u64,
        read_total: u64,
        verified: bool,
    },
    StoredFile {
        reader: DeflatedFileTake<'s>,
        hasher: Crc32Hasher,
        expected_crc32: u32,
        expected_len: u64,
        read_total: u64,
        verified: bool,
    },
    DeflatedSlice(DeflatedZipFile<'r, &'s [u8]>),
    DeflatedFile(DeflatedZipFile<'r, DeflatedFileTake<'s>>),
}

impl ZipEntryReader<'_, '_> {
    pub(crate) fn size(&self) -> u64 {
        self.uncompressed_size
    }
}

impl Read for ZipEntryReader<'_, '_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match &mut self.inner {
            ZipEntryReaderInner::StoredSlice {
                remaining,
                hasher,
                expected_crc32,
                expected_len,
                read_total,
                verified,
            } => {
                if remaining.is_empty() {
                    if !*verified {
                        *verified = true;
                        if *read_total != *expected_len
                            || hasher.clone().finalize() != *expected_crc32
                        {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "ZIP entry CRC32 or length mismatch",
                            ));
                        }
                    }
                    return Ok(0);
                }
                let n = remaining.len().min(buf.len());
                buf[..n].copy_from_slice(&remaining[..n]);
                hasher.update(&remaining[..n]);
                *remaining = &remaining[n..];
                *read_total += n as u64;
                if remaining.is_empty() && !*verified {
                    *verified = true;
                    if *read_total != *expected_len || hasher.clone().finalize() != *expected_crc32
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "ZIP entry CRC32 or length mismatch",
                        ));
                    }
                }
                Ok(n)
            }
            ZipEntryReaderInner::StoredFile {
                reader,
                hasher,
                expected_crc32,
                expected_len,
                read_total,
                verified,
            } => {
                if buf.is_empty() {
                    if *read_total == *expected_len && !*verified {
                        *verified = true;
                        if hasher.clone().finalize() != *expected_crc32 {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "ZIP entry CRC32 or length mismatch",
                            ));
                        }
                    }
                    return Ok(0);
                }
                let n = reader.read(buf)?;
                if n > 0 {
                    hasher.update(&buf[..n]);
                    *read_total += n as u64;
                }
                if (n == 0 || *read_total == *expected_len) && !*verified {
                    *verified = true;
                    if *read_total != *expected_len || hasher.clone().finalize() != *expected_crc32
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "ZIP entry CRC32 or length mismatch",
                        ));
                    }
                }
                Ok(n)
            }
            ZipEntryReaderInner::DeflatedSlice(zf) => zf.read(buf),
            ZipEntryReaderInner::DeflatedFile(zf) => zf.read(buf),
        }
    }
}

impl ZipPackResources {
    pub(crate) fn open(archive_path: &Path, cache_root: &Path) -> Result<Self> {
        let (backing, entry_count, file_count, entries) = if is_virtual_path(archive_path) {
            let bytes = get_virtual_file(archive_path).with_context(|| {
                format!("open virtual resource pack ZIP {}", archive_path.display())
            })?;
            let (entry_count, file_count, entries) =
                parse_central_directory_from_slice(archive_path, &bytes)?;
            (ZipBacking::Memory(bytes), entry_count, file_count, entries)
        } else {
            let mut file = File::open(archive_path)
                .with_context(|| format!("open resource pack ZIP {}", archive_path.display()))?;
            let len = file
                .metadata()
                .with_context(|| format!("stat resource pack ZIP {}", archive_path.display()))?
                .len();
            let (entry_count, file_count, entries) =
                parse_central_directory_from_file(archive_path, &mut file, len)?;
            (
                ZipBacking::File {
                    len,
                    files: Mutex::new(vec![file]),
                },
                entry_count,
                file_count,
                entries,
            )
        };

        Ok(Self {
            inner: Arc::new(ZipPackInner {
                archive_path: archive_path.to_path_buf(),
                cache_root: cache_root.to_path_buf(),
                backing,
                entry_count,
                file_count,
                entries,
                materialized_bytes: AtomicU64::new(0),
            }),
        })
    }

    pub(crate) fn entry_count(&self) -> usize {
        self.inner.entry_count
    }

    pub(crate) fn file_count(&self) -> usize {
        self.inner.file_count
    }

    pub(crate) fn entry(&self, name: &str) -> Option<ZipEntrySource> {
        if let Some((key, &meta)) = self.inner.entries.get_key_value(name) {
            return Some(self.source(key.clone(), meta));
        }
        let directory_name = format!("{name}/");
        let (key, &meta) = self.inner.entries.get_key_value(directory_name.as_str())?;
        Some(self.source(key.clone(), meta))
    }

    pub(crate) fn namespaces_under(&self, root_prefix: &str) -> BTreeSet<String> {
        let mut namespaces = BTreeSet::new();
        let mut cursor = root_prefix.to_owned();
        while let Some((name, _)) = self
            .inner
            .entries
            .range::<str, _>((
                std::ops::Bound::Included(cursor.as_str()),
                std::ops::Bound::Unbounded,
            ))
            .next()
        {
            let Some(remainder) = name.strip_prefix(root_prefix) else {
                break;
            };
            if let Some((namespace, _)) = remainder.split_once('/') {
                if !namespace.is_empty() {
                    namespaces.insert(namespace.to_owned());
                }
                cursor.clear();
                cursor.push_str(root_prefix);
                cursor.push_str(namespace);
                // ASCII '0' (0x30) is immediately after '/' (0x2F), skipping the entire subtree.
                cursor.push('0');
            } else {
                if !remainder.is_empty() {
                    namespaces.insert(remainder.to_owned());
                }
                cursor.clear();
                cursor.push_str(name);
                cursor.push('\0');
            }
        }
        namespaces
    }

    pub(crate) fn range_files<'a>(
        &'a self,
        prefix: &'a str,
    ) -> impl Iterator<Item = (&'a str, ZipEntrySource, u32)> + 'a {
        self.inner
            .entries
            .range::<str, _>((
                std::ops::Bound::Included(prefix),
                std::ops::Bound::Unbounded,
            ))
            .take_while(move |(name, _)| name.starts_with(prefix))
            .filter(|(_, meta)| !meta.directory)
            .map(move |(name, &meta)| {
                (
                    &**name,
                    self.source(name.clone(), meta),
                    meta.central_duplicates,
                )
            })
    }

    fn source(&self, name: Box<str>, meta: ZipEntryMeta) -> ZipEntrySource {
        let target = self.inner.cache_root.join(cache_file_name(&name));
        ZipEntrySource {
            resources: self.clone(),
            name,
            meta,
            target,
        }
    }
}

impl ZipEntrySource {
    pub(crate) fn target(&self) -> &Path {
        &self.target
    }

    pub(crate) fn uncompressed_size(&self) -> u64 {
        self.meta.uncompressed_size
    }

    pub(crate) fn with_entry<T>(
        &self,
        f: impl FnOnce(&mut ZipEntryReader<'_, '_>) -> Result<T>,
    ) -> Result<T> {
        let meta = self.meta;
        if meta.encrypted {
            bail!(
                "encrypted ZIP resource {} in {} is unsupported",
                self.name,
                self.resources.inner.archive_path.display()
            );
        }

        match &self.resources.inner.backing {
            ZipBacking::Memory(archive_bytes) => {
                let raw_slice = resolve_slice_payload(
                    archive_bytes,
                    meta,
                    &self.name,
                    &self.resources.inner.archive_path,
                )?;
                match meta.compression_method {
                    0 => {
                        if raw_slice.len() as u64 != meta.uncompressed_size {
                            bail!(
                                "stored ZIP resource {} in {} has mismatched sizes ({} vs {})",
                                self.name,
                                self.resources.inner.archive_path.display(),
                                raw_slice.len(),
                                meta.uncompressed_size
                            );
                        }
                        let mut reader = ZipEntryReader {
                            uncompressed_size: meta.uncompressed_size,
                            inner: ZipEntryReaderInner::StoredSlice {
                                remaining: raw_slice,
                                hasher: Crc32Hasher::new(),
                                expected_crc32: meta.crc32,
                                expected_len: meta.uncompressed_size,
                                read_total: 0,
                                verified: false,
                            },
                        };
                        f(&mut reader)
                    }
                    8 => {
                        let header = build_synthetic_local_header(
                            meta,
                            &self.name,
                            &self.resources.inner.archive_path,
                        )?;
                        let mut chained = Cursor::new(header).chain(raw_slice);
                        let zf = zip::read::read_zipfile_from_stream_with_compressed_size(
                            &mut chained,
                            meta.compressed_size,
                        )
                        .with_context(|| {
                            format!(
                                "open deflated ZIP resource {} from {}",
                                self.name,
                                self.resources.inner.archive_path.display()
                            )
                        })?
                        .context("missing synthetic local ZIP header")?;
                        let mut reader = ZipEntryReader {
                            uncompressed_size: meta.uncompressed_size,
                            inner: ZipEntryReaderInner::DeflatedSlice(zf),
                        };
                        f(&mut reader)
                    }
                    other => bail!(
                        "unsupported ZIP compression method {} for {} in {}",
                        other,
                        self.name,
                        self.resources.inner.archive_path.display()
                    ),
                }
            }
            ZipBacking::File { len, files } => {
                let mut pooled_file =
                    PooledZipFile::checkout(files, &self.resources.inner.archive_path)?;
                seek_file_to_payload(
                    pooled_file.file_mut(),
                    *len,
                    meta,
                    &self.name,
                    &self.resources.inner.archive_path,
                )?;
                let limited = BufReader::with_capacity(
                    meta.compressed_size.min(64 * 1024) as usize,
                    pooled_file.file_mut(),
                )
                .take(meta.compressed_size);
                match meta.compression_method {
                    0 => {
                        if meta.compressed_size != meta.uncompressed_size {
                            bail!(
                                "stored ZIP resource {} in {} has mismatched sizes ({} vs {})",
                                self.name,
                                self.resources.inner.archive_path.display(),
                                meta.compressed_size,
                                meta.uncompressed_size
                            );
                        }
                        let mut reader = ZipEntryReader {
                            uncompressed_size: meta.uncompressed_size,
                            inner: ZipEntryReaderInner::StoredFile {
                                reader: limited,
                                hasher: Crc32Hasher::new(),
                                expected_crc32: meta.crc32,
                                expected_len: meta.uncompressed_size,
                                read_total: 0,
                                verified: false,
                            },
                        };
                        f(&mut reader)
                    }
                    8 => {
                        let header = build_synthetic_local_header(
                            meta,
                            &self.name,
                            &self.resources.inner.archive_path,
                        )?;
                        let mut chained = Cursor::new(header).chain(limited);
                        // Own `chained` on the heap only if `ZipFile` borrows; instead keep
                        // `chained` on the stack right here!
                        let zf = zip::read::read_zipfile_from_stream_with_compressed_size(
                            &mut chained,
                            meta.compressed_size,
                        )
                        .with_context(|| {
                            format!(
                                "open deflated ZIP resource {} from {}",
                                self.name,
                                self.resources.inner.archive_path.display()
                            )
                        })?
                        .context("missing synthetic local ZIP header")?;
                        let mut reader = ZipEntryReader {
                            uncompressed_size: meta.uncompressed_size,
                            inner: ZipEntryReaderInner::DeflatedFile(zf),
                        };
                        f(&mut reader)
                    }
                    other => bail!(
                        "unsupported ZIP compression method {} for {} in {}",
                        other,
                        self.name,
                        self.resources.inner.archive_path.display()
                    ),
                }
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn read(&self) -> Result<Vec<u8>> {
        self.read_bounded(
            MAX_ZIP_ENTRY_UNCOMPRESSED_BYTES,
            "ZIP resource",
            &self.target,
        )
    }

    pub(crate) fn read_bounded(
        &self,
        max_bytes: u64,
        label: &str,
        display_path: &Path,
    ) -> Result<Vec<u8>> {
        self.with_entry(|entry| {
            let size = entry.size();
            if size > max_bytes {
                bail!(
                    "{label} {} is {size} bytes, exceeding the {max_bytes} byte limit",
                    display_path.display()
                );
            }
            let prealloc = usize::try_from(size.min(64 * 1024)).unwrap_or(0);
            let mut bytes = Vec::with_capacity(prealloc);
            entry
                .take(max_bytes.saturating_add(1))
                .read_to_end(&mut bytes)
                .with_context(|| format!("read {label} {}", display_path.display()))?;
            if bytes.len() as u64 > max_bytes {
                bail!(
                    "{label} {} grew past the {max_bytes} byte limit",
                    display_path.display()
                );
            }
            Ok(bytes)
        })
    }

    pub(crate) fn materialize(&self) -> Result<&Path> {
        self.materialize_with_limits(
            MAX_ZIP_ENTRY_UNCOMPRESSED_BYTES,
            MAX_ZIP_ARCHIVE_MATERIALIZED_BYTES,
        )
    }

    pub(crate) fn materialize_with_limits(
        &self,
        max_entry_bytes: u64,
        max_archive_bytes: u64,
    ) -> Result<&Path> {
        if self.meta.uncompressed_size > max_entry_bytes {
            bail!(
                "ZIP resource {} from {} is {} bytes, exceeding the {max_entry_bytes} byte entry materialization limit",
                self.name,
                self.resources.inner.archive_path.display(),
                self.meta.uncompressed_size
            );
        }
        if is_virtual_path(&self.resources.inner.cache_root) {
            if !has_virtual_file(&self.target) {
                let used = self
                    .resources
                    .inner
                    .materialized_bytes
                    .load(Ordering::Acquire);
                let remaining = max_archive_bytes.saturating_sub(used);
                if self.meta.uncompressed_size > remaining
                    || used >= max_archive_bytes && self.meta.uncompressed_size > 0
                {
                    bail!(
                        "materializing ZIP resource {} from {} ({} bytes) would exceed the {max_archive_bytes} byte archive materialization limit (already materialized {used} bytes)",
                        self.name,
                        self.resources.inner.archive_path.display(),
                        self.meta.uncompressed_size
                    );
                }
                let effective_limit = max_entry_bytes.min(remaining);
                let bytes = self.read_bounded(effective_limit, "ZIP resource", &self.target)?;
                self.resources.inner.reserve_materialized_bytes(
                    bytes.len() as u64,
                    max_archive_bytes,
                    &self.name,
                )?;
                insert_virtual_file(self.target.clone(), Arc::from(bytes));
            }
            return Ok(&self.target);
        }
        if self.target.is_file() {
            return Ok(&self.target);
        }
        let used = self
            .resources
            .inner
            .materialized_bytes
            .load(Ordering::Acquire);
        let remaining = max_archive_bytes.saturating_sub(used);
        if self.meta.uncompressed_size > remaining {
            bail!(
                "materializing ZIP resource {} from {} ({} bytes) would exceed the {max_archive_bytes} byte archive materialization limit (already materialized {used} bytes)",
                self.name,
                self.resources.inner.archive_path.display(),
                self.meta.uncompressed_size
            );
        }
        let effective_limit = max_entry_bytes.min(remaining);
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
            let copied = self.with_entry(|entry| {
                let mut limited = entry.take(effective_limit.saturating_add(1));
                std::io::copy(&mut limited, &mut output).with_context(|| {
                    format!(
                        "materialize ZIP resource {} from {}",
                        self.name,
                        self.resources.inner.archive_path.display()
                    )
                })
            })?;
            if copied > effective_limit {
                bail!(
                    "ZIP resource {} from {} grew past the {effective_limit} byte materialization limit",
                    self.name,
                    self.resources.inner.archive_path.display()
                );
            }
            self.resources.inner.reserve_materialized_bytes(
                copied,
                max_archive_bytes,
                &self.name,
            )?;
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

fn resolve_slice_payload<'a>(
    archive_bytes: &'a [u8],
    meta: ZipEntryMeta,
    name: &str,
    archive_path: &Path,
) -> Result<&'a [u8]> {
    let lh_off = usize::try_from(meta.local_header_offset)
        .context("ZIP local header offset exceeds usize")?;
    let hdr_end = lh_off
        .checked_add(LOCAL_FILE_HEADER_FIXED_BYTES)
        .context("ZIP local header offset overflow")?;
    let hdr = archive_bytes.get(lh_off..hdr_end).with_context(|| {
        format!(
            "truncated local header for {} in {}",
            name,
            archive_path.display()
        )
    })?;
    if u32::from_le_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]) != LOCAL_FILE_HEADER_SIGNATURE {
        bail!(
            "invalid local file header signature for {} in {}",
            name,
            archive_path.display()
        );
    }
    let name_len = usize::from(u16::from_le_bytes([hdr[26], hdr[27]]));
    let extra_len = usize::from(u16::from_le_bytes([hdr[28], hdr[29]]));
    let data_start = hdr_end
        .checked_add(name_len)
        .and_then(|v| v.checked_add(extra_len))
        .context("ZIP local variable header overflow")?;
    let comp_len =
        usize::try_from(meta.compressed_size).context("ZIP compressed size exceeds usize")?;
    let data_end = data_start
        .checked_add(comp_len)
        .context("ZIP payload end overflow")?;
    archive_bytes.get(data_start..data_end).with_context(|| {
        format!(
            "truncated ZIP payload for {} in {}",
            name,
            archive_path.display()
        )
    })
}

fn seek_file_to_payload(
    file: &mut File,
    archive_len: u64,
    meta: ZipEntryMeta,
    name: &str,
    archive_path: &Path,
) -> Result<()> {
    let hdr_end = meta
        .local_header_offset
        .checked_add(LOCAL_FILE_HEADER_FIXED_BYTES as u64)
        .context("ZIP local header offset overflow")?;
    if hdr_end > archive_len {
        bail!(
            "truncated local header for {} in {}",
            name,
            archive_path.display()
        );
    }
    file.seek(SeekFrom::Start(meta.local_header_offset))
        .with_context(|| format!("seek local header in {}", archive_path.display()))?;
    let mut hdr = [0u8; LOCAL_FILE_HEADER_FIXED_BYTES];
    file.read_exact(&mut hdr)
        .with_context(|| format!("read local header in {}", archive_path.display()))?;
    if u32::from_le_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]) != LOCAL_FILE_HEADER_SIGNATURE {
        bail!(
            "invalid local file header signature for {} in {}",
            name,
            archive_path.display()
        );
    }
    let name_len = u64::from(u16::from_le_bytes([hdr[26], hdr[27]]));
    let extra_len = u64::from(u16::from_le_bytes([hdr[28], hdr[29]]));
    let data_start = hdr_end
        .checked_add(name_len)
        .and_then(|v| v.checked_add(extra_len))
        .context("ZIP local variable header overflow")?;
    let data_end = data_start
        .checked_add(meta.compressed_size)
        .context("ZIP payload end overflow")?;
    if data_end > archive_len {
        bail!(
            "truncated ZIP payload for {} in {}",
            name,
            archive_path.display()
        );
    }
    file.seek(SeekFrom::Start(data_start))
        .with_context(|| format!("seek ZIP payload in {}", archive_path.display()))?;
    Ok(())
}

fn build_synthetic_local_header(
    meta: ZipEntryMeta,
    name: &str,
    archive_path: &Path,
) -> Result<[u8; LOCAL_FILE_HEADER_FIXED_BYTES]> {
    let comp32 = u32::try_from(meta.compressed_size).with_context(|| {
        format!(
            "ZIP resource {} in {} exceeds 4 GiB compressed size",
            name,
            archive_path.display()
        )
    })?;
    let uncomp32 = u32::try_from(meta.uncompressed_size).with_context(|| {
        format!(
            "ZIP resource {} in {} exceeds 4 GiB uncompressed size",
            name,
            archive_path.display()
        )
    })?;
    if comp32 == u32::MAX || uncomp32 == u32::MAX {
        bail!(
            "ZIP resource {} in {} exceeds standard 32-bit entry limit",
            name,
            archive_path.display()
        );
    }
    let mut header = [0u8; LOCAL_FILE_HEADER_FIXED_BYTES];
    header[0..4].copy_from_slice(&LOCAL_FILE_HEADER_SIGNATURE.to_le_bytes());
    header[4..6].copy_from_slice(&20u16.to_le_bytes());
    header[8..10].copy_from_slice(&meta.compression_method.to_le_bytes());
    header[14..18].copy_from_slice(&meta.crc32.to_le_bytes());
    header[18..22].copy_from_slice(&comp32.to_le_bytes());
    header[22..26].copy_from_slice(&uncomp32.to_le_bytes());
    Ok(header)
}

fn locate_eocd_in_tail(
    tail: &[u8],
    tail_abs_start: u64,
    archive_path: &Path,
) -> Result<(u64, usize)> {
    if tail.len() < EOCD_MIN_BYTES {
        bail!(
            "resource pack ZIP {} is shorter than minimal EOCD record",
            archive_path.display()
        );
    }
    for pos in (0..=tail.len() - EOCD_MIN_BYTES).rev() {
        if u32::from_le_bytes([tail[pos], tail[pos + 1], tail[pos + 2], tail[pos + 3]])
            == EOCD_SIGNATURE
        {
            let comment_len = usize::from(u16::from_le_bytes([tail[pos + 20], tail[pos + 21]]));
            if pos + EOCD_MIN_BYTES + comment_len == tail.len() {
                return Ok((tail_abs_start + pos as u64, pos));
            }
        }
    }
    bail!(
        "resource pack ZIP {} is missing a valid end-of-central-directory record",
        archive_path.display()
    )
}

fn parse_central_directory_from_slice(
    archive_path: &Path,
    bytes: &[u8],
) -> Result<(usize, usize, BTreeMap<Box<str>, ZipEntryMeta>)> {
    let len = bytes.len() as u64;
    let max_tail = (EOCD_MIN_BYTES + EOCD_MAX_COMMENT_BYTES + ZIP64_LOCATOR_BYTES) as u64;
    let tail_start = len.saturating_sub(max_tail) as usize;
    let (eocd_abs, eocd_rel) =
        locate_eocd_in_tail(&bytes[tail_start..], tail_start as u64, archive_path)?;
    let eocd = &bytes[tail_start + eocd_rel..tail_start + eocd_rel + EOCD_MIN_BYTES];

    let mut central_size = u64::from(u32::from_le_bytes([eocd[12], eocd[13], eocd[14], eocd[15]]));
    let mut central_offset =
        u64::from(u32::from_le_bytes([eocd[16], eocd[17], eocd[18], eocd[19]]));

    if eocd_abs >= ZIP64_LOCATOR_BYTES as u64 {
        let loc_off = (eocd_abs - ZIP64_LOCATOR_BYTES as u64) as usize;
        let loc = &bytes[loc_off..loc_off + ZIP64_LOCATOR_BYTES];
        if u32::from_le_bytes([loc[0], loc[1], loc[2], loc[3]]) == ZIP64_EOCD_LOCATOR_SIGNATURE {
            let z64_off = u64::from_le_bytes(loc[8..16].try_into().unwrap());
            let z64_usize = usize::try_from(z64_off).context("ZIP64 EOCD offset exceeds usize")?;
            let z64 = bytes
                .get(z64_usize..z64_usize + ZIP64_EOCD_MIN_BYTES)
                .with_context(|| format!("truncated ZIP64 EOCD in {}", archive_path.display()))?;
            if u32::from_le_bytes([z64[0], z64[1], z64[2], z64[3]]) != ZIP64_EOCD_SIGNATURE {
                bail!("invalid ZIP64 EOCD signature in {}", archive_path.display());
            }
            central_size = u64::from_le_bytes(z64[40..48].try_into().unwrap());
            central_offset = u64::from_le_bytes(z64[48..56].try_into().unwrap());
        }
    }

    let cd_end = central_offset
        .checked_add(central_size)
        .context("central directory bounds overflow")?;
    if cd_end > eocd_abs {
        bail!(
            "central directory bounds exceed EOCD in {}",
            archive_path.display()
        );
    }
    let cd_slice = &bytes[central_offset as usize..cd_end as usize];
    parse_central_directory_bytes(archive_path, cd_slice)
}

fn parse_central_directory_from_file(
    archive_path: &Path,
    file: &mut File,
    len: u64,
) -> Result<(usize, usize, BTreeMap<Box<str>, ZipEntryMeta>)> {
    let max_tail = (EOCD_MIN_BYTES + EOCD_MAX_COMMENT_BYTES + ZIP64_LOCATOR_BYTES) as u64;
    let tail_start = len.saturating_sub(max_tail);
    file.seek(SeekFrom::Start(tail_start))
        .with_context(|| format!("seek EOCD in {}", archive_path.display()))?;
    let mut tail = Vec::with_capacity((len - tail_start) as usize);
    file.read_to_end(&mut tail)
        .with_context(|| format!("read EOCD in {}", archive_path.display()))?;
    let (eocd_abs, eocd_rel) = locate_eocd_in_tail(&tail, tail_start, archive_path)?;
    let eocd = &tail[eocd_rel..eocd_rel + EOCD_MIN_BYTES];

    let mut central_size = u64::from(u32::from_le_bytes([eocd[12], eocd[13], eocd[14], eocd[15]]));
    let mut central_offset =
        u64::from(u32::from_le_bytes([eocd[16], eocd[17], eocd[18], eocd[19]]));

    if eocd_abs >= ZIP64_LOCATOR_BYTES as u64 {
        let loc_abs = eocd_abs - ZIP64_LOCATOR_BYTES as u64;
        let mut loc = [0u8; ZIP64_LOCATOR_BYTES];
        file.seek(SeekFrom::Start(loc_abs))?;
        file.read_exact(&mut loc)?;
        if u32::from_le_bytes([loc[0], loc[1], loc[2], loc[3]]) == ZIP64_EOCD_LOCATOR_SIGNATURE {
            let z64_off = u64::from_le_bytes(loc[8..16].try_into().unwrap());
            let mut z64 = [0u8; ZIP64_EOCD_MIN_BYTES];
            file.seek(SeekFrom::Start(z64_off))
                .with_context(|| format!("seek ZIP64 EOCD in {}", archive_path.display()))?;
            file.read_exact(&mut z64)
                .with_context(|| format!("read ZIP64 EOCD in {}", archive_path.display()))?;
            if u32::from_le_bytes([z64[0], z64[1], z64[2], z64[3]]) != ZIP64_EOCD_SIGNATURE {
                bail!("invalid ZIP64 EOCD signature in {}", archive_path.display());
            }
            central_size = u64::from_le_bytes(z64[40..48].try_into().unwrap());
            central_offset = u64::from_le_bytes(z64[48..56].try_into().unwrap());
        }
    }

    let cd_end = central_offset
        .checked_add(central_size)
        .context("central directory bounds overflow")?;
    if cd_end > eocd_abs {
        bail!(
            "central directory bounds exceed EOCD in {}",
            archive_path.display()
        );
    }
    let cd_len = usize::try_from(central_size).context("central directory exceeds usize")?;
    file.seek(SeekFrom::Start(central_offset))
        .with_context(|| format!("seek central directory in {}", archive_path.display()))?;
    let mut cd_bytes = vec![0u8; cd_len];
    file.read_exact(&mut cd_bytes)
        .with_context(|| format!("read central directory in {}", archive_path.display()))?;
    parse_central_directory_bytes(archive_path, &cd_bytes)
}

fn parse_central_directory_bytes(
    archive_path: &Path,
    cd: &[u8],
) -> Result<(usize, usize, BTreeMap<Box<str>, ZipEntryMeta>)> {
    let mut pos = 0usize;
    let mut entry_count = 0usize;
    let mut file_count = 0usize;
    let mut entries: BTreeMap<Box<str>, ZipEntryMeta> = BTreeMap::new();

    while pos < cd.len() {
        let hdr = cd
            .get(pos..pos + CENTRAL_DIRECTORY_FIXED_BYTES)
            .with_context(|| {
                format!(
                    "truncated central directory header in {}",
                    archive_path.display()
                )
            })?;
        let sig = u32::from_le_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]);
        if sig != CENTRAL_DIRECTORY_HEADER_SIGNATURE {
            bail!(
                "invalid central directory header signature {sig:#x} in {}",
                archive_path.display()
            );
        }
        let flags = u16::from_le_bytes([hdr[8], hdr[9]]);
        let compression_method = u16::from_le_bytes([hdr[10], hdr[11]]);
        let crc32 = u32::from_le_bytes([hdr[16], hdr[17], hdr[18], hdr[19]]);
        let comp32 = u32::from_le_bytes([hdr[20], hdr[21], hdr[22], hdr[23]]);
        let uncomp32 = u32::from_le_bytes([hdr[24], hdr[25], hdr[26], hdr[27]]);
        let name_len = usize::from(u16::from_le_bytes([hdr[28], hdr[29]]));
        let extra_len = usize::from(u16::from_le_bytes([hdr[30], hdr[31]]));
        let comment_len = usize::from(u16::from_le_bytes([hdr[32], hdr[33]]));
        let lhoff32 = u32::from_le_bytes([hdr[42], hdr[43], hdr[44], hdr[45]]);

        let name_start = pos + CENTRAL_DIRECTORY_FIXED_BYTES;
        let name_end = name_start
            .checked_add(name_len)
            .context("central directory name overflow")?;
        let extra_end = name_end
            .checked_add(extra_len)
            .context("central directory extra overflow")?;
        let next_pos = extra_end
            .checked_add(comment_len)
            .context("central directory comment overflow")?;
        if next_pos > cd.len() {
            bail!(
                "truncated central directory variable fields in {}",
                archive_path.display()
            );
        }

        let name_str = std::str::from_utf8(&cd[name_start..name_end]).with_context(|| {
            format!(
                "resource pack ZIP {} has a non-UTF-8 entry name",
                archive_path.display()
            )
        })?;

        let mut uncompressed_size = u64::from(uncomp32);
        let mut compressed_size = u64::from(comp32);
        let mut local_header_offset = u64::from(lhoff32);

        if uncomp32 == u32::MAX || comp32 == u32::MAX || lhoff32 == u32::MAX {
            parse_zip64_extra(
                &cd[name_end..extra_end],
                uncomp32 == u32::MAX,
                comp32 == u32::MAX,
                lhoff32 == u32::MAX,
                &mut uncompressed_size,
                &mut compressed_size,
                &mut local_header_offset,
            )?;
        }

        let directory = name_str.ends_with('/');
        entry_count += 1;
        if !directory {
            file_count += 1;
        }

        let new_meta = ZipEntryMeta {
            local_header_offset,
            compressed_size,
            uncompressed_size,
            crc32,
            compression_method,
            encrypted: (flags & 1) != 0,
            directory,
            central_duplicates: 1,
        };

        if let Some(existing) = entries.get_mut(name_str) {
            let count = existing.central_duplicates.saturating_add(1);
            *existing = ZipEntryMeta {
                central_duplicates: count,
                ..new_meta
            };
        } else {
            entries.insert(name_str.into(), new_meta);
        }

        pos = next_pos;
    }

    Ok((entry_count, file_count, entries))
}

fn parse_zip64_extra(
    extra: &[u8],
    need_uncomp: bool,
    need_comp: bool,
    need_offset: bool,
    uncompressed_size: &mut u64,
    compressed_size: &mut u64,
    local_header_offset: &mut u64,
) -> Result<()> {
    let mut p = 0usize;
    while p + 4 <= extra.len() {
        let id = u16::from_le_bytes([extra[p], extra[p + 1]]);
        let len = usize::from(u16::from_le_bytes([extra[p + 2], extra[p + 3]]));
        let body_start = p + 4;
        let Some(body_end) = body_start.checked_add(len) else {
            break;
        };
        let Some(body) = extra.get(body_start..body_end) else {
            break;
        };
        if id == ZIP64_EXTRA_HEADER_ID {
            let mut q = 0usize;
            if need_uncomp {
                let s = body
                    .get(q..q + 8)
                    .context("truncated ZIP64 uncompressed size")?;
                *uncompressed_size = u64::from_le_bytes(s.try_into().unwrap());
                q += 8;
            }
            if need_comp {
                let s = body
                    .get(q..q + 8)
                    .context("truncated ZIP64 compressed size")?;
                *compressed_size = u64::from_le_bytes(s.try_into().unwrap());
                q += 8;
            }
            if need_offset {
                let s = body
                    .get(q..q + 8)
                    .context("truncated ZIP64 local header offset")?;
                *local_header_offset = u64::from_le_bytes(s.try_into().unwrap());
            }
            return Ok(());
        }
        p = body_end;
    }
    Ok(())
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

#[cfg(not(target_arch = "wasm32"))]
type LazyVirtualLock<T> = std::sync::RwLock<T>;

#[cfg(target_arch = "wasm32")]
type LazyVirtualLock<T> = bbb_render_types::wasm_sync::SpinRwLock<T>;

#[derive(Debug, Clone)]
struct LazyZipEntrySource {
    resources: Weak<ZipPackInner>,
    name: Box<str>,
    meta: ZipEntryMeta,
    target: PathBuf,
}

impl LazyZipEntrySource {
    fn from_source(source: &ZipEntrySource) -> Self {
        Self {
            resources: Arc::downgrade(&source.resources.inner),
            name: source.name.clone(),
            meta: source.meta,
            target: source.target.clone(),
        }
    }

    fn upgrade(&self) -> Option<ZipEntrySource> {
        let inner = self.resources.upgrade()?;
        Some(ZipEntrySource {
            resources: ZipPackResources { inner },
            name: self.name.clone(),
            meta: self.meta,
            target: self.target.clone(),
        })
    }
}

fn normalize_lazy_key(path: &Path) -> PathBuf {
    crate::virtual_store::normalize_virtual_path(path).unwrap_or_else(|| path.to_path_buf())
}

fn lazy_virtual_store() -> &'static LazyVirtualLock<HashMap<PathBuf, LazyZipEntrySource>> {
    static LAZY: OnceLock<LazyVirtualLock<HashMap<PathBuf, LazyZipEntrySource>>> = OnceLock::new();
    LAZY.get_or_init(|| LazyVirtualLock::new(HashMap::new()))
}

#[cfg(not(target_arch = "wasm32"))]
fn read_lazy_virtual_store(
) -> std::sync::RwLockReadGuard<'static, HashMap<PathBuf, LazyZipEntrySource>> {
    lazy_virtual_store()
        .read()
        .expect("lazy virtual ZIP store read lock poisoned")
}

#[cfg(target_arch = "wasm32")]
fn read_lazy_virtual_store(
) -> bbb_render_types::wasm_sync::SpinReadGuard<'static, HashMap<PathBuf, LazyZipEntrySource>> {
    lazy_virtual_store().read()
}

#[cfg(not(target_arch = "wasm32"))]
fn write_lazy_virtual_store(
) -> std::sync::RwLockWriteGuard<'static, HashMap<PathBuf, LazyZipEntrySource>> {
    lazy_virtual_store()
        .write()
        .expect("lazy virtual ZIP store write lock poisoned")
}

#[cfg(target_arch = "wasm32")]
fn write_lazy_virtual_store(
) -> bbb_render_types::wasm_sync::SpinWriteGuard<'static, HashMap<PathBuf, LazyZipEntrySource>> {
    lazy_virtual_store().write()
}

/// Sweep expired `Weak` archive references only once the table reaches this
/// watermark, avoiding an O(N) scan per registration while never discarding
/// live entries.
const LAZY_VIRTUAL_SWEEP_WATERMARK: usize = 8_192;

pub(crate) fn register_lazy_virtual_entries(entries: Vec<(PathBuf, ZipEntrySource)>) {
    if entries.is_empty() {
        return;
    }
    let prepared: Vec<(PathBuf, LazyZipEntrySource)> = entries
        .into_iter()
        .map(|(path, source)| {
            (
                normalize_lazy_key(&path),
                LazyZipEntrySource::from_source(&source),
            )
        })
        .collect();
    let mut guard = write_lazy_virtual_store();
    if guard.len().saturating_add(prepared.len()) >= LAZY_VIRTUAL_SWEEP_WATERMARK {
        guard.retain(|_, entry| entry.resources.strong_count() > 0);
    }
    guard.reserve(prepared.len());
    for (key, entry) in prepared {
        guard.insert(key, entry);
    }
}

pub(crate) fn remove_lazy_virtual_prefix(prefix: &Path) {
    let norm = normalize_lazy_key(prefix);
    let mut guard = write_lazy_virtual_store();
    guard.retain(|key, entry| !key.starts_with(&norm) && entry.resources.strong_count() > 0);
}

pub(crate) fn read_lazy_virtual_entry_bounded(
    path: &Path,
    max_bytes: u64,
    label: &str,
) -> Option<Result<Vec<u8>>> {
    let key = normalize_lazy_key(path);
    let upgraded = {
        let guard = read_lazy_virtual_store();
        guard.get(&key)?.upgrade()
    };
    if let Some(source) = upgraded {
        return Some(source.read_bounded(max_bytes, label, path));
    }
    let mut guard = write_lazy_virtual_store();
    if guard
        .get(&key)
        .is_some_and(|entry| entry.resources.strong_count() == 0)
    {
        guard.remove(&key);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Write, time::SystemTime};
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
        let listed = resources
            .range_files("assets/minecraft/")
            .map(|(name, _, duplicates)| (name, duplicates))
            .collect::<Vec<_>>();
        assert_eq!(
            listed,
            vec![
                ("assets/minecraft/other.txt", 1),
                ("assets/minecraft/test-a.txt", 2),
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
        drop(metadata);
        drop(resources);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn file_backing_reuses_bounded_handle_pool_across_stored_and_deflated_reads() {
        let root = unique_temp_dir("handle-pool");
        std::fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("pack.zip");
        let file = File::create(&archive_path).unwrap();
        let mut writer = ZipWriter::new(file);
        writer
            .start_file(
                "assets/minecraft/stored.txt",
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(b"stored-payload").unwrap();
        writer
            .start_file(
                "assets/minecraft/deflated.txt",
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
        writer
            .write_all(b"deflated-payload-repeated-repeated")
            .unwrap();
        writer.finish().unwrap();

        let resources = ZipPackResources::open(&archive_path, &root.join("cache")).unwrap();
        let ZipBacking::File { files, .. } = &resources.inner.backing else {
            panic!("expected file backing");
        };
        assert_eq!(files.lock().unwrap().len(), 1);

        for _ in 0..4 {
            assert_eq!(
                resources
                    .entry("assets/minecraft/stored.txt")
                    .unwrap()
                    .read()
                    .unwrap(),
                b"stored-payload"
            );
            assert_eq!(
                resources
                    .entry("assets/minecraft/deflated.txt")
                    .unwrap()
                    .read()
                    .unwrap(),
                b"deflated-payload-repeated-repeated"
            );
            assert_eq!(files.lock().unwrap().len(), 1);
        }

        // Concurrent checkouts beyond MAX_POOLED_ZIP_FILES do not grow the pool past its cap.
        let mut guards = Vec::new();
        for _ in 0..MAX_POOLED_ZIP_FILES + 2 {
            guards.push(PooledZipFile::checkout(files, &archive_path).unwrap());
        }
        assert_eq!(files.lock().unwrap().len(), 0);
        drop(guards);
        assert_eq!(files.lock().unwrap().len(), MAX_POOLED_ZIP_FILES);

        drop(resources);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stored_file_read_exact_verifies_crc32_without_trailing_zero_read() {
        let root = unique_temp_dir("stored-crc");
        std::fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("corrupt-stored.zip");
        let payload = b"stored-crc-check";
        {
            let file = File::create(&archive_path).unwrap();
            let mut writer = ZipWriter::new(file);
            writer
                .start_file(
                    "assets/minecraft/stored.txt",
                    SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
                )
                .unwrap();
            writer.write_all(payload).unwrap();
            writer.finish().unwrap();
        }
        // Corrupt the stored payload bytes in place without updating the ZIP CRC32.
        let mut bytes = std::fs::read(&archive_path).unwrap();
        let pos = bytes
            .windows(payload.len())
            .position(|window| window == payload)
            .expect("stored payload present in ZIP");
        bytes[pos] ^= 0xFF;
        std::fs::write(&archive_path, &bytes).unwrap();

        let resources = ZipPackResources::open(&archive_path, &root.join("cache")).unwrap();
        let entry = resources.entry("assets/minecraft/stored.txt").unwrap();
        let err = entry
            .with_entry(|reader| {
                let mut out = vec![0u8; payload.len()];
                reader.read_exact(&mut out)?;
                Ok(())
            })
            .unwrap_err();
        assert!(err.to_string().contains("CRC32"), "{err}");

        drop(entry);
        drop(resources);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lazy_virtual_batch_registration_sweeps_dead_entries_without_clearing_live_entries() {
        let root = unique_temp_dir("lazy-virtual-batch");
        std::fs::create_dir_all(&root).unwrap();
        let make_zip = |name: &str, payload: &[u8]| {
            let archive_path = root.join(name);
            let file = File::create(&archive_path).unwrap();
            let mut writer = ZipWriter::new(file);
            writer
                .start_file(
                    "assets/minecraft/sounds/a.ogg",
                    SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
                )
                .unwrap();
            writer.write_all(payload).unwrap();
            writer.finish().unwrap();
            archive_path
        };
        let live_zip = make_zip("live.zip", b"live-ogg");
        let dead_zip = make_zip("dead.zip", b"dead-ogg");

        let live_resources = ZipPackResources::open(&live_zip, &root.join("cache-live")).unwrap();
        let live_source = live_resources
            .entry("assets/minecraft/sounds/a.ogg")
            .unwrap();
        let prefix = PathBuf::from(format!(
            "bbb-lazy-virtual-test-{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let dead_key = normalize_lazy_key(&prefix.join("dead/0.ogg"));
        {
            let dead_resources =
                ZipPackResources::open(&dead_zip, &root.join("cache-dead")).unwrap();
            let dead_source = dead_resources
                .entry("assets/minecraft/sounds/a.ogg")
                .unwrap();
            register_lazy_virtual_entries(vec![(prefix.join("dead/0.ogg"), dead_source)]);
        }

        // Register more than 16,384 live entries in one batch: dead weak entries
        // are swept once above the watermark, and live entries are never wiped.
        const LIVE_COUNT: usize = 16_500;
        let batch = (0..LIVE_COUNT)
            .map(|index| {
                (
                    prefix.join(format!("live/{index}.ogg")),
                    live_source.clone(),
                )
            })
            .collect();
        register_lazy_virtual_entries(batch);

        {
            let guard = read_lazy_virtual_store();
            assert!(
                !guard.contains_key(&dead_key),
                "expired weak entries should be swept above the watermark"
            );
        }
        assert_eq!(
            read_lazy_virtual_entry_bounded(&prefix.join("live/0.ogg"), 1024, "test")
                .expect("first live entry must remain registered")
                .unwrap(),
            b"live-ogg"
        );
        assert_eq!(
            read_lazy_virtual_entry_bounded(
                &prefix.join(format!("live/{}.ogg", LIVE_COUNT - 1)),
                1024,
                "test"
            )
            .expect("last live entry must remain registered")
            .unwrap(),
            b"live-ogg"
        );

        remove_lazy_virtual_prefix(&prefix);
        drop(live_source);
        drop(live_resources);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn materialize_enforces_per_entry_and_cumulative_decompression_limits() {
        let root = unique_temp_dir("materialize-limits");
        std::fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("pack.zip");
        {
            let file = File::create(&archive_path).unwrap();
            let mut writer = ZipWriter::new(file);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            for name in ["assets/minecraft/a.ogg", "assets/minecraft/b.ogg"] {
                writer.start_file(name, opts).unwrap();
                writer.write_all(&vec![0u8; 4096]).unwrap();
            }
            writer.finish().unwrap();
        }

        let cache = root.join("cache");
        let resources = ZipPackResources::open(&archive_path, &cache).unwrap();
        let a = resources.entry("assets/minecraft/a.ogg").unwrap();
        let b = resources.entry("assets/minecraft/b.ogg").unwrap();

        // Entry larger than per-entry limit is rejected without leaving files in cache.
        let err = a.materialize_with_limits(2048, 8192).unwrap_err();
        assert!(
            format!("{err:#}").contains("entry materialization limit"),
            "{err:#}"
        );
        assert!(!a.target().exists());

        // Cumulative archive cap allows first 4096-byte entry, then rejects the second.
        let path_a = a.materialize_with_limits(4096, 6000).unwrap();
        assert_eq!(std::fs::metadata(path_a).unwrap().len(), 4096);
        let err = b.materialize_with_limits(4096, 6000).unwrap_err();
        assert!(
            format!("{err:#}").contains("archive materialization limit"),
            "{err:#}"
        );
        assert!(!b.target().exists());

        drop(a);
        drop(b);
        drop(resources);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn materialize_stops_forged_header_deflate_bomb_without_publishing_temp_file() {
        let root = unique_temp_dir("forged-header-bomb");
        std::fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("bomb.zip");
        let real_payload = vec![0x41u8; 8192];
        {
            let file = File::create(&archive_path).unwrap();
            let mut writer = ZipWriter::new(file);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            writer
                .start_file("assets/minecraft/sounds/bomb.ogg", opts)
                .unwrap();
            writer.write_all(&real_payload).unwrap();
            writer.finish().unwrap();
        }
        // Patch the central directory uncompressed_size from 8192 to 16 so the header lies.
        let mut bytes = std::fs::read(&archive_path).unwrap();
        let cd_pos = bytes
            .windows(4)
            .rposition(|w| {
                u32::from_le_bytes(w.try_into().unwrap()) == CENTRAL_DIRECTORY_HEADER_SIGNATURE
            })
            .expect("central directory header present");
        bytes[cd_pos + 24..cd_pos + 28].copy_from_slice(&16u32.to_le_bytes());
        std::fs::write(&archive_path, &bytes).unwrap();

        let cache = root.join("cache");
        let resources = ZipPackResources::open(&archive_path, &cache).unwrap();
        let bomb = resources.entry("assets/minecraft/sounds/bomb.ogg").unwrap();
        let err = bomb.materialize_with_limits(1024, 4096).unwrap_err();
        let message = format!("{err:#}");
        assert!(
            message.contains("larger than its declared uncompressed size")
                || message.contains("grew past the 1024 byte materialization limit"),
            "{message}"
        );
        assert!(!bomb.target().exists());
        assert_eq!(std::fs::read_dir(&cache).unwrap().count(), 0);

        drop(bomb);
        drop(resources);
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
