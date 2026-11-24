//! Exact-version playerdata file media.

use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(test)]
use bbb_protocol::encode_named_nbt_summary;
#[cfg(test)]
use bbb_protocol::encode_named_nbt_summary_java_string;
use bbb_protocol::{decode_file_saved_nbt_summary, NbtSummaryValue};
use flate2::{write::GzEncoder, Compression};
use uuid::Uuid;

use super::atomic_file::{io_error, StagedFile};
use super::level_file::{is_fallback_eligible_error, read_optional_gzip_or_raw_nbt_bytes};
use bbb_server_types::save::{
    NamedCompound, SaveError, SaveResult, WorldSaveLimits, WorldSavePaths, JAVA_26_1_DATA_VERSION,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerDataSource {
    Primary,
    Fallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedPlayerData {
    pub source: PlayerDataSource,
    pub document: NamedCompound,
    pub decompressed_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerDataCandidate {
    pub uuid: Uuid,
    pub has_primary: bool,
    pub has_fallback: bool,
}

#[derive(Debug, Clone)]
pub struct PlayerDataStore {
    paths: WorldSavePaths,
    limits: WorldSaveLimits,
}

impl PlayerDataStore {
    pub fn new(paths: WorldSavePaths, limits: WorldSaveLimits) -> SaveResult<Self> {
        Ok(Self {
            paths,
            limits: limits.validate()?,
        })
    }

    pub fn load(&self, uuid: Uuid) -> SaveResult<Option<LoadedPlayerData>> {
        let primary_path = self.player_data_path(uuid);
        let fallback_path = self.player_data_old_path(uuid);
        match read_optional_player_compound(&primary_path, self.limits) {
            Ok(Some((document, decompressed_bytes))) => {
                validate_data_version(&document, &primary_path)?;
                Ok(Some(LoadedPlayerData {
                    source: PlayerDataSource::Primary,
                    document,
                    decompressed_bytes,
                }))
            }
            Ok(None) => match read_optional_player_compound(&fallback_path, self.limits) {
                Ok(Some((document, decompressed_bytes))) => {
                    validate_data_version(&document, &fallback_path)?;
                    Ok(Some(LoadedPlayerData {
                        source: PlayerDataSource::Fallback,
                        document,
                        decompressed_bytes,
                    }))
                }
                Ok(None) => Ok(None),
                Err(error) => Err(error),
            },
            Err(primary) if !is_fallback_eligible_error(&primary) => Err(primary),
            Err(primary) => match read_optional_player_compound(&fallback_path, self.limits) {
                Ok(Some((document, decompressed_bytes))) => {
                    validate_data_version(&document, &fallback_path)?;
                    Ok(Some(LoadedPlayerData {
                        source: PlayerDataSource::Fallback,
                        document,
                        decompressed_bytes,
                    }))
                }
                Ok(None) => Err(primary),
                Err(fallback) if !is_fallback_eligible_error(&fallback) => Err(fallback),
                Err(fallback) => Err(no_usable_player_data_error(
                    &primary_path,
                    primary,
                    fallback,
                )),
            },
        }
    }

    pub fn load_candidate(&self, candidate: PlayerDataCandidate) -> SaveResult<LoadedPlayerData> {
        let loaded = self
            .load(candidate.uuid)?
            .ok_or_else(|| SaveError::InvalidNbt {
                path: Some(self.paths.player_data_dir()),
                detail: format!(
                    "discovered player {} became unavailable during load",
                    candidate.uuid
                ),
            })?;
        let source_was_discovered = match loaded.source {
            PlayerDataSource::Primary => candidate.has_primary,
            PlayerDataSource::Fallback => candidate.has_fallback,
        };
        if !source_was_discovered {
            return Err(SaveError::InvalidNbt {
                path: Some(self.paths.player_data_dir()),
                detail: format!(
                    "player {} load source changed after discovery",
                    candidate.uuid
                ),
            });
        }
        Ok(loaded)
    }

    pub fn discover_candidates(&self) -> SaveResult<Vec<PlayerDataCandidate>> {
        let player_data_dir = self.paths.player_data_dir();
        let entries = match fs::read_dir(&player_data_dir) {
            Ok(entries) => entries,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => {
                return Err(io_error(
                    "reading playerdata directory",
                    &player_data_dir,
                    source,
                ))
            }
        };
        let mut candidates = BTreeMap::new();
        for entry in entries {
            let entry = entry.map_err(|source| {
                io_error(
                    "reading playerdata directory entry",
                    &player_data_dir,
                    source,
                )
            })?;
            let file_type = entry.file_type().map_err(|source| {
                io_error(
                    "reading playerdata directory entry type",
                    &entry.path(),
                    source,
                )
            })?;
            if file_type.is_symlink() {
                return Err(SaveError::InvalidPath {
                    path: entry.path(),
                    detail: "managed playerdata directory must not contain symbolic links",
                });
            }
            let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Some((uuid, is_primary)) = parse_player_data_file_name(&file_name) else {
                continue;
            };
            if !file_type.is_file() {
                return Err(SaveError::InvalidPath {
                    path: entry.path(),
                    detail: "canonical playerdata path must be a regular file",
                });
            }
            let candidate = candidates.entry(uuid).or_insert(PlayerDataCandidate {
                uuid,
                has_primary: false,
                has_fallback: false,
            });
            if is_primary {
                candidate.has_primary = true;
            } else {
                candidate.has_fallback = true;
            }
        }
        let mut candidates: Vec<_> = candidates.into_values().collect();
        candidates.sort_unstable_by(|left, right| left.uuid.as_bytes().cmp(right.uuid.as_bytes()));
        Ok(candidates)
    }

    pub fn save(&self, uuid: Uuid, document: &NamedCompound) -> SaveResult<()> {
        let current = self.player_data_path(uuid);
        validate_data_version(document, &current)?;
        let bytes = super::level_file::encode_saved_compound_nbt(
            &current,
            document,
            self.limits.max_decompressed_nbt_bytes,
        )?;
        let compressed = gzip_encode(&current, &bytes)?;
        check_budget(
            "compressed playerdata NBT",
            compressed.len(),
            self.limits.max_compressed_nbt_bytes,
        )?;
        let fallback = self.player_data_old_path(uuid);
        let staged = StagedFile::write(
            &self.paths.player_data_dir(),
            &uuid.hyphenated().to_string(),
            &compressed,
        )?;
        staged.publish_with_backup(&current, &fallback)
    }

    fn player_data_path(&self, uuid: Uuid) -> PathBuf {
        self.paths
            .player_data_dir()
            .join(format!("{}.dat", uuid.hyphenated()))
    }

    fn player_data_old_path(&self, uuid: Uuid) -> PathBuf {
        self.paths
            .player_data_dir()
            .join(format!("{}.dat_old", uuid.hyphenated()))
    }
}

fn parse_player_data_file_name(file_name: &str) -> Option<(Uuid, bool)> {
    let (uuid_text, is_primary) = if let Some(uuid_text) = file_name.strip_suffix(".dat_old") {
        (uuid_text, false)
    } else {
        let uuid_text = file_name.strip_suffix(".dat")?;
        (uuid_text, true)
    };
    let uuid = Uuid::parse_str(uuid_text).ok()?;
    (uuid.hyphenated().to_string() == uuid_text).then_some((uuid, is_primary))
}

fn read_optional_player_compound(
    path: &Path,
    limits: WorldSaveLimits,
) -> SaveResult<Option<(NamedCompound, usize)>> {
    let Some(bytes) = read_optional_gzip_or_raw_nbt_bytes(
        path,
        limits,
        "compressed playerdata NBT",
        "decompressed playerdata NBT",
    )?
    else {
        return Ok(None);
    };
    let root = decode_file_saved_nbt_summary(&bytes)
        .map_err(|error| SaveError::InvalidNbt {
            path: Some(path.to_path_buf()),
            detail: error.to_string(),
        })?
        .ok_or_else(|| SaveError::InvalidNbt {
            path: Some(path.to_path_buf()),
            detail: "playerdata NBT root must not be TAG_End".to_string(),
        })?;
    let document =
        NamedCompound::try_from_summary("", root).map_err(|error| SaveError::InvalidNbt {
            path: Some(path.to_path_buf()),
            detail: error.to_string(),
        })?;
    Ok(Some((document, bytes.len())))
}

#[cfg(test)]
fn read_optional_gzip_named_compound(
    path: &Path,
    limits: WorldSaveLimits,
) -> SaveResult<Option<(NamedCompound, usize)>> {
    let Some((document, len)) = read_optional_player_compound(path, limits)? else {
        return Ok(None);
    };
    validate_data_version(&document, path)?;
    Ok(Some((document, len)))
}

fn validate_data_version(document: &NamedCompound, path: &Path) -> SaveResult<()> {
    let mut data_version = None;
    for entry in document.entries() {
        if entry.name != "DataVersion" {
            continue;
        }
        let NbtSummaryValue::Int(value) = &entry.value else {
            return Err(SaveError::InvalidNbt {
                path: Some(path.to_path_buf()),
                detail: "playerdata DataVersion must be a TAG_Int".to_string(),
            });
        };
        if data_version.replace(*value).is_some() {
            return Err(SaveError::InvalidNbt {
                path: Some(path.to_path_buf()),
                detail: "playerdata must contain exactly one top-level DataVersion".to_string(),
            });
        }
    }
    let actual = data_version.ok_or_else(|| SaveError::InvalidNbt {
        path: Some(path.to_path_buf()),
        detail: "playerdata is missing top-level DataVersion".to_string(),
    })?;
    if actual != JAVA_26_1_DATA_VERSION {
        return Err(SaveError::UnsupportedDataVersion {
            expected: JAVA_26_1_DATA_VERSION,
            actual,
        });
    }
    Ok(())
}

fn gzip_encode(path: &Path, bytes: &[u8]) -> SaveResult<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(bytes)
        .map_err(|source| io_error("compressing playerdata NBT", path, source))?;
    encoder
        .finish()
        .map_err(|source| io_error("finishing compressed playerdata NBT", path, source))
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

fn no_usable_player_data_error(path: &Path, primary: SaveError, fallback: SaveError) -> SaveError {
    SaveError::InvalidNbt {
        path: Some(path.to_path_buf()),
        detail: format!("neither playerdata primary nor fallback was usable; primary: {primary}; fallback: {fallback}"),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::Path,
        sync::atomic::{AtomicU64, Ordering},
    };

    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use super::*;

    static NEXT_DIR: AtomicU64 = AtomicU64::new(1);

    fn temp_paths(name: &str) -> WorldSavePaths {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        WorldSavePaths::new(
            std::env::temp_dir().join(format!("bbb-playerdata-{name}-{}-{id}", std::process::id())),
        )
        .unwrap()
    }

    fn uuid() -> Uuid {
        Uuid::parse_str("84F561C1-9FFE-3785-8A7B-6A5351516F8C").unwrap()
    }

    fn document(version: i32) -> NamedCompound {
        NamedCompound::new(
            "",
            vec![
                NbtSummaryEntry {
                    name: "DataVersion".into(),
                    value: NbtSummaryValue::Int(version),
                },
                NbtSummaryEntry {
                    name: "UnknownModField".into(),
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "payload".into(),
                        value: NbtSummaryValue::LongArray(vec![7, 11]),
                    }]),
                },
            ],
        )
    }

    fn write_document(paths: &WorldSavePaths, uuid: Uuid, document: &NamedCompound) {
        let path = paths
            .player_data_dir()
            .join(format!("{}.dat", uuid.hyphenated()));
        fs::create_dir_all(paths.player_data_dir()).unwrap();
        let bytes = encode_named_nbt_summary_java_string(
            document.root_name(),
            &document.to_summary(),
            usize::MAX,
        )
        .unwrap();
        fs::write(path, gzip_encode(Path::new("player.dat"), &bytes).unwrap()).unwrap();
    }

    #[test]
    fn player_reader_skips_malformed_root_name_bytes() {
        let paths = temp_paths("raw-root");
        let store = PlayerDataStore::new(paths.clone(), WorldSaveLimits::default()).unwrap();
        let expected = document(JAVA_26_1_DATA_VERSION);
        let mut raw = encode_named_nbt_summary("abc", &expected.to_summary(), 4096).unwrap();
        raw[3..6].copy_from_slice(&[0xff, 0xff, 0xff]);
        fs::create_dir_all(paths.player_data_dir()).unwrap();
        let path = store.player_data_path(uuid());
        fs::write(&path, gzip_encode(&path, &raw).unwrap()).unwrap();
        assert_eq!(store.load(uuid()).unwrap().unwrap().document, expected);
        fs::remove_dir_all(paths.root()).unwrap();
    }

    fn touch_player_file(paths: &WorldSavePaths, file_name: &str) {
        fs::create_dir_all(paths.player_data_dir()).unwrap();
        fs::write(paths.player_data_dir().join(file_name), b"").unwrap();
    }

    #[test]
    fn discovery_treats_missing_and_empty_directories_as_empty_without_creating_them() {
        let paths = temp_paths("discovery-empty");
        let store = PlayerDataStore::new(paths.clone(), WorldSaveLimits::default()).unwrap();

        assert_eq!(store.discover_candidates().unwrap(), Vec::new());
        assert!(!paths.player_data_dir().exists());

        fs::create_dir_all(paths.player_data_dir()).unwrap();
        assert_eq!(store.discover_candidates().unwrap(), Vec::new());
        fs::remove_dir_all(paths.root()).unwrap();
    }

    #[test]
    fn discovery_merges_primary_and_fallback_and_sorts_by_uuid_bytes() {
        let paths = temp_paths("discovery-merge");
        let first = Uuid::parse_str("00000000-0000-0000-0000-0000000000ff").unwrap();
        let second = Uuid::parse_str("00000000-0000-0000-0000-000000000100").unwrap();
        touch_player_file(&paths, &format!("{second}.dat_old"));
        touch_player_file(&paths, &format!("{first}.dat_old"));
        touch_player_file(&paths, &format!("{first}.dat"));
        let store = PlayerDataStore::new(paths.clone(), WorldSaveLimits::default()).unwrap();

        assert_eq!(
            store.discover_candidates().unwrap(),
            vec![
                PlayerDataCandidate {
                    uuid: first,
                    has_primary: true,
                    has_fallback: true,
                },
                PlayerDataCandidate {
                    uuid: second,
                    has_primary: false,
                    has_fallback: true,
                },
            ]
        );
        fs::remove_dir_all(paths.root()).unwrap();
    }

    #[test]
    fn discovery_ignores_noncanonical_temporary_and_non_file_entries() {
        let paths = temp_paths("discovery-ignore");
        let player = uuid();
        let canonical = player.hyphenated().to_string();
        touch_player_file(&paths, &format!("{canonical}.dat"));
        touch_player_file(&paths, &format!("{}.dat", canonical.to_uppercase()));
        touch_player_file(&paths, &format!("{}.dat", player.simple()));
        touch_player_file(&paths, &format!("{canonical}.dat.tmp"));
        touch_player_file(&paths, &format!("{canonical}-temporary.dat"));
        touch_player_file(&paths, &format!("{canonical}_corrupted_2026-08-13.dat"));
        fs::create_dir(paths.player_data_dir().join("ignored-directory")).unwrap();
        let store = PlayerDataStore::new(paths.clone(), WorldSaveLimits::default()).unwrap();

        assert_eq!(
            store.discover_candidates().unwrap(),
            vec![PlayerDataCandidate {
                uuid: player,
                has_primary: true,
                has_fallback: false,
            }]
        );
        fs::remove_dir_all(paths.root()).unwrap();
    }

    #[test]
    fn first_and_second_save_create_vanilla_named_files_backup_and_no_temp() {
        let paths = temp_paths("roundtrip");
        let store = PlayerDataStore::new(paths.clone(), WorldSaveLimits::default()).unwrap();
        let player = uuid();
        store
            .save(player, &document(JAVA_26_1_DATA_VERSION))
            .unwrap();
        store
            .save(player, &document(JAVA_26_1_DATA_VERSION))
            .unwrap();

        let loaded = store.load(player).unwrap().unwrap();
        assert_eq!(loaded.source, PlayerDataSource::Primary);
        assert_eq!(loaded.document, document(JAVA_26_1_DATA_VERSION));
        assert_eq!(
            read_optional_gzip_named_compound(
                &paths
                    .player_data_dir()
                    .join(format!("{}.dat_old", player.hyphenated())),
                WorldSaveLimits::default(),
            )
            .unwrap(),
            Some((
                document(JAVA_26_1_DATA_VERSION),
                encode_named_nbt_summary(
                    "",
                    &document(JAVA_26_1_DATA_VERSION).to_summary(),
                    WorldSaveLimits::default().max_decompressed_nbt_bytes,
                )
                .unwrap()
                .len(),
            ))
        );
        assert_eq!(
            fs::read_dir(paths.player_data_dir())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp"))
                .count(),
            0
        );
        fs::remove_dir_all(paths.root()).unwrap();
    }

    #[test]
    fn corrupt_primary_falls_back_without_repairing_the_evidence() {
        let paths = temp_paths("fallback");
        let store = PlayerDataStore::new(paths.clone(), WorldSaveLimits::default()).unwrap();
        let player = uuid();
        store
            .save(player, &document(JAVA_26_1_DATA_VERSION))
            .unwrap();
        store
            .save(player, &document(JAVA_26_1_DATA_VERSION))
            .unwrap();
        let primary = paths
            .player_data_dir()
            .join(format!("{}.dat", player.hyphenated()));
        fs::write(&primary, b"not-gzip").unwrap();

        let loaded = store.load(player).unwrap().unwrap();
        assert_eq!(loaded.source, PlayerDataSource::Fallback);
        assert_eq!(loaded.document, document(JAVA_26_1_DATA_VERSION));
        assert_eq!(fs::read(primary).unwrap(), b"not-gzip");
        fs::remove_dir_all(paths.root()).unwrap();
    }

    #[test]
    fn missing_both_files_is_not_an_error() {
        let paths = temp_paths("missing");
        let store = PlayerDataStore::new(paths.clone(), WorldSaveLimits::default()).unwrap();
        assert_eq!(store.load(uuid()).unwrap(), None);
    }

    #[test]
    fn other_data_versions_are_refused() {
        for version in [JAVA_26_1_DATA_VERSION - 1, JAVA_26_1_DATA_VERSION + 1] {
            let paths = temp_paths("data-version");
            write_document(&paths, uuid(), &document(version));
            assert!(matches!(
                PlayerDataStore::new(paths.clone(), WorldSaveLimits::default())
                    .unwrap()
                    .load(uuid()),
                Err(SaveError::UnsupportedDataVersion { actual, .. }) if actual == version
            ));
            fs::remove_dir_all(paths.root()).unwrap();
        }
    }

    #[test]
    fn future_primary_data_version_does_not_fall_back_to_old_file() {
        let paths = temp_paths("no-version-fallback");
        let player = uuid();
        let store = PlayerDataStore::new(paths.clone(), WorldSaveLimits::default()).unwrap();
        store
            .save(player, &document(JAVA_26_1_DATA_VERSION))
            .unwrap();
        store
            .save(player, &document(JAVA_26_1_DATA_VERSION))
            .unwrap();
        // Overwrite primary with a valid gzip NBT carrying a newer DataVersion while .dat_old stays 26.1.
        write_document(&paths, player, &document(JAVA_26_1_DATA_VERSION + 1));
        assert!(matches!(
            store.load(player),
            Err(SaveError::UnsupportedDataVersion { actual, .. })
                if actual == JAVA_26_1_DATA_VERSION + 1
        ));
        fs::remove_dir_all(paths.root()).unwrap();
    }

    #[test]
    #[ignore = "requires BBB_TEST_JAVA_26_1_WORLD from an authorized official-server run"]
    fn verified_official_java_26_1_playerdata_matches_frozen_identity() {
        let root = std::env::var_os("BBB_TEST_JAVA_26_1_WORLD")
            .expect("BBB_TEST_JAVA_26_1_WORLD must name an official 26.1 world");
        let paths = WorldSavePaths::new(root).unwrap();
        let player_file = fs::read_dir(paths.player_data_dir())
            .unwrap()
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "dat")
            })
            .expect("official world must contain playerdata")
            .path();
        let player = Uuid::parse_str(player_file.file_stem().unwrap().to_str().unwrap()).unwrap();
        let store = PlayerDataStore::new(paths, WorldSaveLimits::default()).unwrap();

        let loaded = store.load(player).unwrap().unwrap();

        assert_eq!(loaded.source, PlayerDataSource::Primary);
        assert_eq!(loaded.document.root_name(), "");
    }
}
