use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use bbb_protocol::{
    decode_file_saved_nbt_summary, decode_named_saved_nbt_summary,
    encode_named_nbt_summary_java_string_saved_file, encode_named_nbt_summary_saved_file,
};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};

use super::atomic_file::{io_error, open_file_matches_path, StagedFile};
use bbb_server_types::save::{
    NamedCompound, SaveError, SaveResult, WorldSaveLimits, WorldSavePaths,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelDatSource {
    Primary,
    Fallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedLevelDat {
    pub source: LevelDatSource,
    pub document: NamedCompound,
    pub decompressed_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct LevelDatStore {
    paths: WorldSavePaths,
    limits: WorldSaveLimits,
}

impl LevelDatStore {
    pub fn new(paths: WorldSavePaths, limits: WorldSaveLimits) -> SaveResult<Self> {
        Ok(Self {
            paths,
            limits: limits.validate()?,
        })
    }

    pub fn load(&self) -> SaveResult<LoadedLevelDat> {
        match read_gzip_compound(&self.paths.level_dat(), self.limits) {
            Ok((document, decompressed_bytes)) => Ok(LoadedLevelDat {
                source: LevelDatSource::Primary,
                document,
                decompressed_bytes,
            }),
            Err(primary) if !is_fallback_eligible_error(&primary) => Err(primary),
            Err(primary) => match read_gzip_compound(&self.paths.level_dat_old(), self.limits) {
                Ok((document, decompressed_bytes)) => Ok(LoadedLevelDat {
                    source: LevelDatSource::Fallback,
                    document,
                    decompressed_bytes,
                }),
                Err(fallback) if !is_fallback_eligible_error(&fallback) => Err(fallback),
                Err(fallback) => Err(SaveError::NoUsableLevelData {
                    primary: primary.to_string(),
                    fallback: fallback.to_string(),
                }),
            },
        }
    }

    pub fn save(&self, document: &NamedCompound) -> SaveResult<()> {
        let compressed = encode_gzip_compound(&self.paths.level_dat(), document, self.limits)?;
        let staged = StagedFile::write(self.paths.root(), "level", &compressed)?;
        staged.publish_with_backup(&self.paths.level_dat(), &self.paths.level_dat_old())
    }
}

/// Encodes a named compound into the gzipped NBT byte form used by
/// `level.dat`, playerdata, and `data/*.dat` files, enforcing the
/// decompressed-encode and compressed-byte budgets before any file is staged.
pub fn encode_gzip_named_compound(
    path: &Path,
    document: &NamedCompound,
    limits: WorldSaveLimits,
) -> SaveResult<Vec<u8>> {
    let bytes = encode_named_nbt_summary_java_string_saved_file(
        document.root_name(),
        document.as_summary(),
        limits.max_decompressed_nbt_bytes,
    )
    .map_err(|error| SaveError::InvalidNbt {
        path: Some(path.to_path_buf()),
        detail: error.to_string(),
    })?;
    compress_bounded_nbt(path, &bytes, limits)
}

pub fn encode_gzip_compound(
    path: &Path,
    document: &NamedCompound,
    limits: WorldSaveLimits,
) -> SaveResult<Vec<u8>> {
    let bytes = encode_saved_compound_nbt(path, document, limits.max_decompressed_nbt_bytes)?;
    compress_bounded_nbt(path, &bytes, limits)
}

pub(crate) fn encode_gzip_owned_compound(
    path: &Path,
    document: NamedCompound,
    limits: WorldSaveLimits,
) -> SaveResult<Vec<u8>> {
    encode_gzip_compound(path, &document, limits)
}

pub(crate) fn encode_saved_compound_nbt(
    path: &Path,
    document: &NamedCompound,
    max_decompressed_bytes: usize,
) -> SaveResult<Vec<u8>> {
    encode_named_nbt_summary_saved_file("", document.as_summary(), max_decompressed_bytes).map_err(
        |error| SaveError::InvalidNbt {
            path: Some(path.to_path_buf()),
            detail: error.to_string(),
        },
    )
}

fn compress_bounded_nbt(path: &Path, bytes: &[u8], limits: WorldSaveLimits) -> SaveResult<Vec<u8>> {
    let compressed = gzip_encode(path, bytes)?;
    check_budget(
        "compressed persistent NBT",
        compressed.len(),
        limits.max_compressed_nbt_bytes,
    )?;
    Ok(compressed)
}

pub fn read_gzip_named_compound(
    path: &Path,
    limits: WorldSaveLimits,
) -> SaveResult<(NamedCompound, usize)> {
    let bytes = read_gzip_nbt_bytes(path, limits)?;
    let (root_name, root) = decode_named_saved_nbt_summary(&bytes)
        .map_err(|error| SaveError::InvalidNbt {
            path: Some(path.to_path_buf()),
            detail: error.to_string(),
        })?
        .ok_or_else(|| SaveError::InvalidNbt {
            path: Some(path.to_path_buf()),
            detail: "persistent NBT root must not be TAG_End".to_string(),
        })?;
    let document = NamedCompound::try_from_summary(root_name, root).map_err(|error| {
        SaveError::InvalidNbt {
            path: Some(path.to_path_buf()),
            detail: error.to_string(),
        }
    })?;
    Ok((document, bytes.len()))
}

/// Only missing/corrupt primary data falls back to `*_old`; security/path,
/// budget, and version errors propagate directly.
pub(crate) fn is_fallback_eligible_error(error: &SaveError) -> bool {
    matches!(error, SaveError::Io { .. } | SaveError::InvalidNbt { .. })
}

fn open_optional_regular_file(
    path: &Path,
    operation: &'static str,
    symlink_detail: &'static str,
    changed_detail: &'static str,
) -> SaveResult<Option<File>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(SaveError::InvalidPath {
                path: path.to_path_buf(),
                detail: symlink_detail,
            });
        }
        Ok(_) => {}
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(io_error(operation, path, source)),
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = match options.open(path) {
        Ok(file) => file,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(io_error(operation, path, source)),
    };
    if !open_file_matches_path(&file, path) {
        return Err(SaveError::InvalidPath {
            path: path.to_path_buf(),
            detail: changed_detail,
        });
    }
    Ok(Some(file))
}

pub(crate) fn read_optional_gzip_compound(
    path: &Path,
    limits: WorldSaveLimits,
) -> SaveResult<Option<(NamedCompound, usize)>> {
    let Some(bytes) = read_optional_gzip_or_raw_nbt_bytes(
        path,
        limits,
        "compressed persistent NBT",
        "decompressed persistent NBT",
    )?
    else {
        return Ok(None);
    };
    decode_file_compound(path, &bytes).map(Some)
}

pub(crate) fn read_optional_bounded_file(
    path: &Path,
    operation: &'static str,
    resource: &'static str,
    max_bytes: usize,
) -> SaveResult<Option<Vec<u8>>> {
    let Some(file) = open_optional_regular_file(
        path,
        operation,
        "managed world-save path must be a regular file and not a symbolic link",
        "managed world-save path changed or became a symbolic link while opening",
    )?
    else {
        return Ok(None);
    };
    let len = file
        .metadata()
        .map_err(|source| io_error(operation, path, source))?
        .len();
    if len > max_bytes as u64 {
        return Err(SaveError::BudgetExceeded {
            resource,
            actual: len,
            limit: max_bytes as u64,
        });
    }
    let mut bytes = Vec::with_capacity(len as usize);
    file.take((max_bytes as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| io_error(operation, path, source))?;
    if bytes.len() > max_bytes {
        return Err(SaveError::BudgetExceeded {
            resource,
            actual: bytes.len() as u64,
            limit: max_bytes as u64,
        });
    }
    Ok(Some(bytes))
}

pub fn read_gzip_compound(
    path: &Path,
    limits: WorldSaveLimits,
) -> SaveResult<(NamedCompound, usize)> {
    let bytes = read_gzip_nbt_bytes(path, limits)?;
    decode_file_compound(path, &bytes)
}

fn decode_file_compound(path: &Path, bytes: &[u8]) -> SaveResult<(NamedCompound, usize)> {
    let root = decode_file_saved_nbt_summary(bytes)
        .map_err(|error| SaveError::InvalidNbt {
            path: Some(path.to_path_buf()),
            detail: error.to_string(),
        })?
        .ok_or_else(|| SaveError::InvalidNbt {
            path: Some(path.to_path_buf()),
            detail: "persistent NBT root must not be TAG_End".to_string(),
        })?;
    let document =
        NamedCompound::try_from_summary("", root).map_err(|error| SaveError::InvalidNbt {
            path: Some(path.to_path_buf()),
            detail: error.to_string(),
        })?;
    Ok((document, bytes.len()))
}

fn read_gzip_nbt_bytes(path: &Path, limits: WorldSaveLimits) -> SaveResult<Vec<u8>> {
    read_optional_gzip_or_raw_nbt_bytes(
        path,
        limits,
        "compressed persistent NBT",
        "decompressed persistent NBT",
    )?
    .ok_or_else(|| {
        io_error(
            "opening NBT file",
            path,
            std::io::Error::from(std::io::ErrorKind::NotFound),
        )
    })
}

/// Shared regular-file reader for gzipped or raw persistent NBT (`level.dat`,
/// `data/*.dat`, `players/data/<uuid>.dat`).
pub(crate) fn read_optional_gzip_or_raw_nbt_bytes(
    path: &Path,
    limits: WorldSaveLimits,
    compressed_resource: &'static str,
    decompressed_resource: &'static str,
) -> SaveResult<Option<Vec<u8>>> {
    let Some(file) = open_optional_regular_file(
        path,
        "opening NBT file",
        "persistent NBT path must be a regular file and not a symbolic link",
        "persistent NBT path changed or became a symbolic link while opening",
    )?
    else {
        return Ok(None);
    };
    let file_len = file
        .metadata()
        .map_err(|source| io_error("reading open NBT metadata", path, source))?
        .len();
    let mut magic = Vec::with_capacity(2);
    (&file)
        .take(2)
        .read_to_end(&mut magic)
        .map_err(|source| io_error("reading NBT file", path, source))?;
    // SavedDataStorage.java:109-126: a file without the gzip magic is raw NBT.
    if !magic.starts_with(&[0x1f, 0x8b]) {
        if file_len > limits.max_decompressed_nbt_bytes as u64 {
            return Err(SaveError::BudgetExceeded {
                resource: decompressed_resource,
                actual: file_len,
                limit: limits.max_decompressed_nbt_bytes as u64,
            });
        }
        let mut stored =
            Vec::with_capacity((file_len as usize).min(limits.max_decompressed_nbt_bytes));
        stored.extend_from_slice(&magic);
        let remaining_limit = (limits.max_decompressed_nbt_bytes as u64)
            .saturating_sub(magic.len() as u64)
            .saturating_add(1);
        file.take(remaining_limit)
            .read_to_end(&mut stored)
            .map_err(|source| io_error("reading NBT file", path, source))?;
        if stored.len() > limits.max_decompressed_nbt_bytes {
            return Err(SaveError::BudgetExceeded {
                resource: decompressed_resource,
                actual: stored.len() as u64,
                limit: limits.max_decompressed_nbt_bytes as u64,
            });
        }
        return Ok(Some(stored));
    }
    if file_len > limits.max_compressed_nbt_bytes as u64 {
        return Err(SaveError::BudgetExceeded {
            resource: compressed_resource,
            actual: file_len,
            limit: limits.max_compressed_nbt_bytes as u64,
        });
    }
    let mut stored = Vec::with_capacity((file_len as usize).min(limits.max_compressed_nbt_bytes));
    stored.extend_from_slice(&magic);
    let remaining_limit = (limits.max_compressed_nbt_bytes as u64)
        .saturating_sub(magic.len() as u64)
        .saturating_add(1);
    file.take(remaining_limit)
        .read_to_end(&mut stored)
        .map_err(|source| io_error("reading NBT file", path, source))?;
    if stored.len() > limits.max_compressed_nbt_bytes {
        return Err(SaveError::BudgetExceeded {
            resource: compressed_resource,
            actual: stored.len() as u64,
            limit: limits.max_compressed_nbt_bytes as u64,
        });
    }
    let mut bytes = Vec::new();
    GzDecoder::new(stored.as_slice())
        .take((limits.max_decompressed_nbt_bytes as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| io_error("decompressing NBT file", path, source))?;
    if bytes.len() > limits.max_decompressed_nbt_bytes {
        return Err(SaveError::BudgetExceeded {
            resource: decompressed_resource,
            actual: bytes.len() as u64,
            limit: limits.max_decompressed_nbt_bytes as u64,
        });
    }
    Ok(Some(bytes))
}

fn gzip_encode(path: &Path, bytes: &[u8]) -> SaveResult<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(bytes)
        .map_err(|source| io_error("compressing NBT file", path, source))?;
    encoder
        .finish()
        .map_err(|source| io_error("finishing compressed NBT file", path, source))
}

fn check_budget(resource: &'static str, actual: usize, limit: usize) -> SaveResult<()> {
    if actual > limit {
        return Err(SaveError::BudgetExceeded {
            resource,
            actual: actual as u64,
            limit: limit as u64,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;

    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use super::*;

    static NEXT_DIR: AtomicU64 = AtomicU64::new(1);

    fn temp_paths(name: &str) -> WorldSavePaths {
        let id = NEXT_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        WorldSavePaths::new(
            std::env::temp_dir().join(format!("bbb-level-dat-{name}-{}-{id}", std::process::id())),
        )
        .unwrap()
    }

    fn document(value: i32) -> NamedCompound {
        NamedCompound::new(
            "",
            vec![NbtSummaryEntry {
                name: "Data".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "DataVersion".into(),
                    value: NbtSummaryValue::Int(value),
                }]),
            }],
        )
    }

    #[test]
    fn level_dat_save_load_and_backup_preserve_named_unknown_fields() {
        let paths = temp_paths("roundtrip");
        let store = LevelDatStore::new(paths.clone(), WorldSaveLimits::default()).unwrap();
        store.save(&document(1)).unwrap();
        store.save(&document(2)).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded.source, LevelDatSource::Primary);
        assert_eq!(loaded.document, document(2));
        assert_eq!(
            read_gzip_named_compound(&paths.level_dat_old(), WorldSaveLimits::default())
                .unwrap()
                .0,
            document(1)
        );
        fs::remove_dir_all(paths.root()).unwrap();
    }

    #[test]
    fn encoder_round_trips() {
        let paths = temp_paths("encoder");
        let document = document(7);
        let compressed =
            encode_gzip_named_compound(&paths.level_dat(), &document, WorldSaveLimits::default())
                .unwrap();
        fs::create_dir_all(paths.root()).unwrap();
        fs::write(paths.level_dat(), compressed).unwrap();
        let (loaded, _) =
            read_gzip_named_compound(&paths.level_dat(), WorldSaveLimits::default()).unwrap();
        assert_eq!(loaded, document);
        fs::remove_dir_all(paths.root()).unwrap();
    }

    #[test]
    fn corrupt_primary_uses_level_dat_old_without_partial_repair() {
        let paths = temp_paths("fallback");
        let store = LevelDatStore::new(paths.clone(), WorldSaveLimits::default()).unwrap();
        store.save(&document(1)).unwrap();
        store.save(&document(2)).unwrap();
        fs::write(paths.level_dat(), b"not-gzip").unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded.source, LevelDatSource::Fallback);
        assert_eq!(loaded.document, document(1));
        assert_eq!(fs::read(paths.level_dat()).unwrap(), b"not-gzip");
        fs::remove_dir_all(paths.root()).unwrap();
    }

    #[test]
    fn encoder_supports_saved_file_wide_compounds_and_deep_nesting() {
        let paths = temp_paths("saved-limits");
        let wide_entries: Vec<NbtSummaryEntry> = (0..5000)
            .map(|i| NbtSummaryEntry {
                name: format!("k{i}").into(),
                value: NbtSummaryValue::Int(i),
            })
            .collect();
        let mut nested = NbtSummaryValue::Int(7);
        for _ in 0..78 {
            nested = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "next".into(),
                value: nested,
            }]);
        }
        let mut entries = wide_entries;
        entries.push(NbtSummaryEntry {
            name: "deep".into(),
            value: nested,
        });
        let doc = NamedCompound::new("Data", entries);
        let compressed =
            encode_gzip_named_compound(&paths.level_dat(), &doc, WorldSaveLimits::default())
                .unwrap();
        fs::create_dir_all(paths.root()).unwrap();
        fs::write(paths.level_dat(), compressed).unwrap();
        let (loaded, _) =
            read_gzip_named_compound(&paths.level_dat(), WorldSaveLimits::default()).unwrap();
        assert_eq!(loaded.entries().len(), 5001);
        fs::remove_dir_all(paths.root()).unwrap();
    }

    #[test]
    fn raw_uncompressed_nbt_uses_decompressed_budget_and_budget_exceeded_does_not_fallback() {
        let paths = temp_paths("raw-budget-no-fallback");
        let doc = document(4786);
        let raw_bytes = encode_saved_compound_nbt(&paths.level_dat(), &doc, 4096).unwrap();
        let compressed_bytes =
            encode_gzip_compound(&paths.level_dat(), &doc, WorldSaveLimits::default()).unwrap();
        fs::create_dir_all(paths.root()).unwrap();

        // Raw NBT whose size exceeds max_compressed_nbt_bytes but fits max_decompressed_nbt_bytes.
        let limits = WorldSaveLimits {
            max_compressed_nbt_bytes: raw_bytes.len() - 1,
            max_decompressed_nbt_bytes: raw_bytes.len() + 64,
            ..Default::default()
        };
        fs::write(paths.level_dat(), &raw_bytes).unwrap();
        let loaded = LevelDatStore::new(paths.clone(), limits)
            .unwrap()
            .load()
            .unwrap();
        assert_eq!(loaded.source, LevelDatSource::Primary);
        assert_eq!(loaded.document, doc);

        // BudgetExceeded on primary gzip level.dat must NOT silently fall back to level.dat_old.
        let tight = WorldSaveLimits {
            max_compressed_nbt_bytes: compressed_bytes.len() - 1,
            max_decompressed_nbt_bytes: raw_bytes.len() + 64,
            ..Default::default()
        };
        fs::write(paths.level_dat(), &compressed_bytes).unwrap();
        fs::write(paths.level_dat_old(), &raw_bytes).unwrap();
        assert!(matches!(
            LevelDatStore::new(paths.clone(), tight).unwrap().load(),
            Err(SaveError::BudgetExceeded { .. })
        ));
        fs::remove_dir_all(paths.root()).unwrap();
    }
}
