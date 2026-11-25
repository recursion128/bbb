//! Server-side datapack repository: discovery, metadata, and selection ordering.
//!
//! Vanilla anchors (Java 26.1):
//! - `ServerPacksSource.java:24-83` — the built-in `vanilla` pack with hard-coded
//!   `VERSION_METADATA_SECTION` (`minorRange()` of the game format) and the
//!   `feature/*` packs; selection configs: vanilla `(required=false, BOTTOM,
//!   fixed=false)`, feature/discovered `(false, TOP, false)`
//!   (`FolderRepositorySource.java:31`).
//! - `FolderRepositorySource.java:44-89` + `PackDetector.java:21-51` — each
//!   subdirectory and each `*.zip` file of `<world>/datapacks` is a candidate with
//!   id `file/<entry-name>` and `PackSource.WORLD`.
//! - `Pack.java:28-70` (`readMetaAndCreate` / `readPackMetadata`) — a candidate
//!   whose `pack.mcmeta` is missing or unparseable yields no pack.
//! - `MinecraftServer.configurePackRepository`
//!   (`MinecraftServer.java:1552-1608`) — selection: warn-drop missing enabled
//!   packs, auto-add packs whose source `shouldAddAutomatically` when their
//!   requested features are a subset of the world's enabled features, remove
//!   selected packs with missing features, force `vanilla` on empty selection.
//! - `PackRepository.rebuildSelected` (`PackRepository.java:82-96`) and
//!   `Pack.Position.insert` (`Pack.java:141-175`) — final ordering.
//!
//! Out of scope (documented in the slice contract): `enableForcedFeaturePacks`
//! (`MinecraftServer.java:1620-1647`). `FolderPackResources` rejects symlinks
//! and canonical paths escaping the pack root fail-closed.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File};
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use bbb_protocol::JavaString;
use bbb_registry_codec::pack_filter::ResourceFilter;
use bbb_registry_codec::RegistryJsonValue as Value;

use super::pack::{
    DeclaredFormatRange, PackFormat, ServerPackResources, ServerPackSource,
    JAVA_26_1_SERVER_DATA_FORMAT,
};
use super::pack_mcmeta::ServerPackMeta;
use bbb_server_types::save::Java26DataPacks;

/// One available datapack: identity, parsed metadata, and its opened resources
/// (`Pack` + `PackLocationInfo`, `Pack.java:21-41`).
pub struct ServerPack {
    /// `PackLocationInfo.id`: `vanilla`, a feature pack id, or `file/<name>`.
    pub id: String,
    /// `PackSource` (`PackSource.java:10-17`).
    pub source: ServerPackSource,
    /// Parsed `pack.mcmeta` (`Pack.Metadata`, `Pack.java:138-139`).
    pub meta: ServerPackMeta,
    /// Opened `PackType.SERVER_DATA` resources (`PackResources`).
    pub resources: Box<dyn ServerPackResources>,
}

impl fmt::Debug for ServerPack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServerPack")
            .field("id", &self.id)
            .field("source", &self.source)
            .field("meta", &self.meta)
            .finish_non_exhaustive()
    }
}

/// Structured replacement for the vanilla selection log lines
/// (`MinecraftServer.java:1565-1605`, `Pack.java:55-68`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackSelectionNotice {
    /// `"Missing data pack {0}"` (`MinecraftServer.java:1569`).
    MissingEnabled(bbb_protocol::JavaString),
    /// `"Found new data pack {0}, loading it automatically"` (`MinecraftServer.java:1580`).
    AutoAdded(String),
    /// `"Found new data pack {0}, but can't load it due to missing features"` (`MinecraftServer.java:1583-1587`).
    AutoAddBlockedMissingFeatures(String),
    /// `"Pack {0} requires features that are not enabled for this world, disabling pack."`
    /// (`MinecraftServer.java:1592-1596`).
    DisabledMissingFeatures(String),
    /// `"No datapacks selected, forcing vanilla"` (`MinecraftServer.java:1603`).
    ForcedVanilla,
    /// `Pack.readMetaAndCreate` returned no pack (`Pack.java:55-68`,
    /// `FolderRepositorySource.java:54-58`).
    UnavailablePack { id: String, reason: String },
}

/// Result of `ServerPackRepository::select`: the ordered selection plus every
/// notice emitted while computing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackSelection {
    /// Selected pack ids in load order (`PackRepository.getSelectedPacks`).
    pub ordered_ids: Vec<String>,
    /// The `DataPackConfig` Vanilla writes back after applying this selection.
    pub effective_data_packs: Java26DataPacks,
    pub notices: Vec<PackSelectionNotice>,
}

impl PackSelection {
    pub fn data_packs(&self) -> &Java26DataPacks {
        &self.effective_data_packs
    }
}

/// What `/datapack list` can still name after startup drops the open repository.
///
/// `DataPackCommand.listAvailablePacks` / `listEnabledPacks`
/// (`DataPackCommand.java:235-274`) walk `PackRepository.getAvailablePacks` and
/// `getSelectedPacks`. The repository is gone once `DatapackStartup::load`
/// finishes, so this is the catalog those two readers need: id, source,
/// description and requested features.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapackListing {
    pub id: String,
    pub source: ServerPackSource,
    pub description_raw: Value,
    pub requested_features: BTreeSet<String>,
}

impl DatapackListing {
    /// Hover text for `Pack.getChatLink` (`Pack.java:84-86`): a string
    /// description is the literal, `{"text": ...}` keeps that text, and any
    /// other JSON is stringified.
    pub fn description_text(&self) -> JavaString {
        match &self.description_raw {
            Value::String(text) => text.clone(),
            value => match value.get("text") {
                Some(Value::String(text)) => text.clone(),
                _ => serde_json::to_string(value)
                    .expect("retained metadata serializes")
                    .into(),
            },
        }
    }
}

/// Fatal repository-construction failures. Vanilla logs and continues with fewer
/// packs for most I/O problems; only an unusable `datapacks` directory itself is
/// fatal here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackRepositoryError {
    /// `FileUtil.createDirectoriesSafe` / `Files.newDirectoryStream` failing on the
    /// datapacks directory (`FolderRepositorySource.java:49-62`).
    DatapacksDirUnavailable { path: PathBuf, reason: String },
}

impl fmt::Display for PackRepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DatapacksDirUnavailable { path, reason } => {
                write!(f, "failed to list packs in {}: {reason}", path.display())
            }
        }
    }
}

impl std::error::Error for PackRepositoryError {}

/// `PackRepository` for server datapacks (`PackRepository.java:20-129`): the fixed
/// source list is `ServerPacksSource` (vanilla + feature packs) plus one
/// `FolderRepositorySource` over `<world>/datapacks`
/// (`ServerPacksSource.createPackRepository`, `ServerPacksSource.java:72-74`).
pub struct ServerPackRepository {
    /// `PackRepository.available`, a `TreeMap` in vanilla (`PackRepository.java:40`);
    /// `BTreeMap` gives the same id-sorted iteration order.
    available: BTreeMap<String, ServerPack>,
    /// Notices recorded during discovery; prepended to every selection result.
    discovery_notices: Vec<PackSelectionNotice>,
}

impl ServerPackRepository {
    /// Build the repository: hard-coded vanilla pack metadata
    /// (`ServerPacksSource.VERSION_METADATA_SECTION`, `ServerPacksSource.java:25-27`),
    /// metadata-parsed feature packs, and `<world>/datapacks` discovery.
    pub fn new(
        datapacks_dir: &Path,
        vanilla_pack: Box<dyn ServerPackResources>,
        feature_packs: Vec<(String, Box<dyn ServerPackResources>)>,
    ) -> Result<Self, PackRepositoryError> {
        let mut available = BTreeMap::new();
        let mut discovery_notices = Vec::new();

        // `ServerPacksSource.java:25-35`: hard-coded metadata, id `vanilla`,
        // `PackSource.BUILT_IN`. `VERSION_METADATA_SECTION` is
        // `gameFormat.minorRange()` = `[game, (game.major, u32::MAX)]`
        // (`PackFormat.java:87-89`).
        let vanilla_meta = ServerPackMeta {
            description_raw: Value::String("The default data pack".into()),
            declared_range: Some(DeclaredFormatRange {
                min: JAVA_26_1_SERVER_DATA_FORMAT,
                max: PackFormat::new(JAVA_26_1_SERVER_DATA_FORMAT.major, u32::MAX),
            }),
            filter: ResourceFilter::default(),
            requested_features: BTreeSet::new(),
            overlays: Vec::new(),
        };
        available.insert(
            "vanilla".to_string(),
            ServerPack {
                id: "vanilla".to_string(),
                source: ServerPackSource::BuiltIn,
                meta: vanilla_meta,
                resources: vanilla_pack,
            },
        );

        // `ServerPacksSource.createBuiltinPack` (`ServerPacksSource.java:68-70`):
        // feature packs parse their own `pack.mcmeta`.
        for (id, resources) in feature_packs {
            match read_pack_meta(resources.as_ref()) {
                Ok(meta) => {
                    available.insert(
                        id.clone(),
                        ServerPack {
                            id,
                            source: ServerPackSource::Feature,
                            meta,
                            resources,
                        },
                    );
                }
                Err(reason) => {
                    discovery_notices.push(PackSelectionNotice::UnavailablePack { id, reason });
                }
            }
        }

        discover_folder_packs(datapacks_dir, &mut available, &mut discovery_notices)?;

        Ok(Self {
            available,
            discovery_notices,
        })
    }

    /// `PackRepository.getPack` (`PackRepository.java:118-120`).
    pub fn get(&self, id: &str) -> Option<&ServerPack> {
        self.available.get(id)
    }

    /// `PackRepository.getAvailablePacks` in `TreeMap` id order
    /// (`PackRepository.java:40`).
    pub fn listings(&self) -> Vec<DatapackListing> {
        self.available
            .values()
            .map(|pack| DatapackListing {
                id: pack.id.clone(),
                source: pack.source,
                description_raw: pack.meta.description_raw.clone(),
                requested_features: pack.meta.requested_features.clone(),
            })
            .collect()
    }

    /// `MinecraftServer.configurePackRepository`
    /// (`MinecraftServer.java:1552-1608`): compute the selected pack set and its
    /// final load order.
    ///
    /// `allowed_features` is the world's enabled feature set
    /// (`WorldDataConfiguration.enabledFeatures`); forced feature packs
    /// (`enableForcedFeaturePacks`) are out of scope.
    pub fn select(
        &self,
        data_packs: &Java26DataPacks,
        safe_mode: bool,
        allowed_features: &BTreeSet<String>,
    ) -> PackSelection {
        self.select_inner(
            data_packs.enabled(),
            data_packs.disabled(),
            safe_mode,
            allowed_features,
        )
    }

    /// `/reload` keeps the effective disabled set and admits newly available ids
    /// before rebuilding selected order (`ReloadCommand.java:26-37`).
    pub fn select_reload(&self, current: &Java26DataPacks) -> PackSelection {
        let mut selected = Vec::new();
        for id in current.enabled() {
            if let Ok(id) = id.try_to_string() {
                if self.available.contains_key(&id) && !selected.contains(&id) {
                    selected.push(id);
                }
            }
        }
        for id in self.available.keys() {
            if !current.disabled().iter().any(|held| held == id) && !selected.contains(id) {
                selected.push(id.clone());
            }
        }
        self.finish_selection(selected, self.discovery_notices.clone(), true)
    }

    fn select_inner(
        &self,
        enabled: &[bbb_protocol::JavaString],
        disabled: &[bbb_protocol::JavaString],
        safe_mode: bool,
        allowed_features: &BTreeSet<String>,
    ) -> PackSelection {
        let mut notices = self.discovery_notices.clone();

        // `selected` is a `LinkedHashSet` in vanilla (`MinecraftServer.java:1563`):
        // insertion-ordered, de-duplicated.
        let mut selected: Vec<String> = Vec::new();
        if safe_mode {
            // `MinecraftServer.java:1559-1561`: exactly `["vanilla"]`.
            selected.push("vanilla".to_string());
        } else {
            for id in enabled {
                let available = id
                    .try_to_string()
                    .ok()
                    .filter(|id| self.available.contains_key(id));
                if let Some(id) = available {
                    if !selected.contains(&id) {
                        selected.push(id);
                    }
                } else {
                    notices.push(PackSelectionNotice::MissingEnabled(id.clone()));
                }
            }

            for pack in self.available.values() {
                let id = &pack.id;
                if disabled.iter().any(|held| held == id) {
                    continue;
                }
                let is_selected = selected.contains(id);
                if !is_selected && pack.source.should_add_automatically() {
                    if pack.meta.requested_features.is_subset(allowed_features) {
                        notices.push(PackSelectionNotice::AutoAdded(id.clone()));
                        selected.push(id.clone());
                    } else {
                        notices.push(PackSelectionNotice::AutoAddBlockedMissingFeatures(
                            id.clone(),
                        ));
                    }
                }
                // `is_selected` is captured before any auto-add above, exactly like
                // vanilla (`MinecraftServer.java:1577`).
                if is_selected && !pack.meta.requested_features.is_subset(allowed_features) {
                    notices.push(PackSelectionNotice::DisabledMissingFeatures(id.clone()));
                    selected.retain(|s| s != id);
                }
            }

            if selected.is_empty() {
                notices.push(PackSelectionNotice::ForcedVanilla);
                selected.push("vanilla".to_string());
            }
        }

        self.finish_selection(selected, notices, !safe_mode)
    }

    /// `PackRepository.setSelected` (`PackRepository.java:54-56`) over an
    /// explicit id list, which is what `MinecraftServer.reloadResources`
    /// installs for the set `/datapack enable` or `disable` computed
    /// (`MinecraftServer.java:1534-1536`).
    pub fn select_ids(&self, ids: &[String]) -> PackSelection {
        self.finish_selection(ids.to_vec(), self.discovery_notices.clone(), true)
    }

    fn finish_selection(
        &self,
        selected: Vec<String>,
        notices: Vec<PackSelectionNotice>,
        disable_inactive: bool,
    ) -> PackSelection {
        // `PackRepository.rebuildSelected` (`PackRepository.java:82-96`) keeps the
        // given order and drops ids that are not available. Its second pass
        // re-inserts a missing `required` pack at its default position, and no
        // server pack is required -- `ServerPacksSource.java:35-36` and
        // `FolderRepositorySource.java:31` all pass `required = false` -- so the
        // order `/datapack enable first|last|before|after` chose survives the
        // reload that applies it.
        let ordered: Vec<&ServerPack> = selected
            .iter()
            .filter_map(|id| self.available.get(id))
            .collect();

        let ordered_ids = ordered
            .iter()
            .map(|pack| pack.id.clone())
            .collect::<Vec<_>>();
        // `getSelectedPacks` writes enabled in selected order and optionally
        // writes every available inactive id (`MinecraftServer.java:1649-1653`).
        let disabled = if disable_inactive {
            self.available
                .keys()
                .filter(|id| !ordered_ids.contains(id))
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        PackSelection {
            effective_data_packs: Java26DataPacks::new(ordered_ids.clone(), disabled),
            ordered_ids,
            notices,
        }
    }

    /// `PackRepository.openAllSelected` (`PackRepository.java:126-128`): the
    /// selected packs in load order.
    pub fn open_selected<'a>(&'a self, selection: &PackSelection) -> Vec<&'a ServerPack> {
        selection
            .ordered_ids
            .iter()
            .filter_map(|id| self.available.get(id))
            .collect()
    }
}

/// `Pack.Position` (`Pack.java:141-143`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackPosition {
    Top,
    Bottom,
}

/// `PackSelectionConfig` (`PackSelectionConfig.java:5-6`) as the three server
/// sources declare it (`ServerPacksSource.java:35-36`,
/// `FolderRepositorySource.java:31`): no server pack is `required` or
/// `fixedPosition`; vanilla sits BOTTOM, feature and world packs TOP.
pub fn selection_config(source: ServerPackSource) -> (bool, PackPosition) {
    match source {
        ServerPackSource::BuiltIn => (false, PackPosition::Bottom),
        ServerPackSource::Feature | ServerPackSource::World => (false, PackPosition::Top),
    }
}

/// `Pack.Position.insert` (`Pack.java:145-170`) with `reverse = false`, over the
/// `(fixedPosition, defaultPosition)` half of each element's selection config.
/// The fixed-position run scans are mirrored exactly; every server datapack
/// declares `fixedPosition = false`, so in practice BOTTOM lands at index 0 and
/// TOP appends.
pub fn position_insert<T>(
    list: &mut Vec<T>,
    value: T,
    position: PackPosition,
    config: impl Fn(&T) -> (bool, PackPosition),
) -> usize {
    let index = match position {
        // Insert after the leading run of fixed-BOTTOM packs.
        PackPosition::Bottom => {
            let mut index = 0;
            while index < list.len() {
                let (fixed, element_position) = config(&list[index]);
                if !fixed || element_position != position {
                    break;
                }
                index += 1;
            }
            index
        }
        // Insert before the trailing run of fixed-TOP packs.
        PackPosition::Top => {
            let mut index = list.len();
            while index > 0 {
                let (fixed, element_position) = config(&list[index - 1]);
                if !fixed || element_position != position {
                    break;
                }
                index -= 1;
            }
            index
        }
    };
    list.insert(index, value);
    index
}

/// `Pack.readPackMetadata` (`Pack.java:43-70`): `pack.mcmeta` must exist and parse.
fn read_pack_meta(resources: &dyn ServerPackResources) -> Result<ServerPackMeta, String> {
    let bytes = resources
        .read_root("pack.mcmeta")
        .ok_or_else(|| "missing pack.mcmeta".to_string())?;
    ServerPackMeta::parse(&bytes).map_err(|e| e.to_string())
}

/// `FolderRepositorySource.loadPacks` + `PackDetector.detectPackResources`
/// (`FolderRepositorySource.java:49-89`, `PackDetector.java:21-51`): every
/// subdirectory and every `*.zip` file is a `file/<name>` candidate with
/// `PackSource.WORLD`; entries without readable metadata are recorded as
/// unavailable. Entries are processed in name order for determinism (vanilla's
/// `DirectoryStream` order is unspecified; the available map is sorted anyway).
fn discover_folder_packs(
    datapacks_dir: &Path,
    available: &mut BTreeMap<String, ServerPack>,
    notices: &mut Vec<PackSelectionNotice>,
) -> Result<(), PackRepositoryError> {
    std::fs::create_dir_all(datapacks_dir).map_err(|e| {
        PackRepositoryError::DatapacksDirUnavailable {
            path: datapacks_dir.to_path_buf(),
            reason: e.to_string(),
        }
    })?;
    let entries = std::fs::read_dir(datapacks_dir).map_err(|e| {
        PackRepositoryError::DatapacksDirUnavailable {
            path: datapacks_dir.to_path_buf(),
            reason: e.to_string(),
        }
    })?;

    let mut candidates: Vec<(String, PathBuf, bool)> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| PackRepositoryError::DatapacksDirUnavailable {
            path: datapacks_dir.to_path_buf(),
            reason: e.to_string(),
        })?;
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        // `PackDetector.java:41-50`: regular directories and `*.zip` regular files only.
        if file_type.is_dir() {
            candidates.push((name, path, true));
        } else if file_type.is_file() && name.ends_with(".zip") {
            candidates.push((name, path, false));
        }
    }
    candidates.sort();

    for (name, path, is_dir) in candidates {
        let id = format!("file/{name}");
        let resources: Box<dyn ServerPackResources> = if is_dir {
            Box::new(FolderPackResources::new(path))
        } else {
            match ZipPackResources::open(&path) {
                Ok(zip) => Box::new(zip),
                Err(reason) => {
                    notices.push(PackSelectionNotice::UnavailablePack { id, reason });
                    continue;
                }
            }
        };
        match read_pack_meta(resources.as_ref()) {
            Ok(meta) => {
                available.insert(
                    id.clone(),
                    ServerPack {
                        id,
                        source: ServerPackSource::World,
                        meta,
                        resources,
                    },
                );
            }
            Err(reason) => {
                notices.push(PackSelectionNotice::UnavailablePack { id, reason });
            }
        }
    }
    Ok(())
}

/// Maximum `.zip` archive bytes read into memory when opening a zipped datapack.
const MAX_ZIP_ARCHIVE_BYTES: usize = 128 * 1024 * 1024;

/// Streaming entry read cap for `FolderPackResources` and `ZipPackResources`.
/// Set above `DatapackLoadLimits::default().max_element_bytes` (8 MiB) so
/// moderately oversized elements still produce attributed loader errors while
/// zip-bombs and giant files stop before allocating unbounded memory.
const MAX_PACK_RESOURCE_BYTES: usize = 16 * 1024 * 1024;

/// Maximum directory nesting depth scanned inside a folder or zip datapack.
const MAX_PACK_DIR_DEPTH: usize = 64;

/// Maximum number of files listed from a folder pack or entries accepted from a zip pack.
const MAX_PACK_FILES: usize = 65_536;

/// Cumulative decompressed byte budget shared across all reads of a single zip pack
/// (including its overlays) to stop multi-entry zip bombs.
const MAX_ZIP_DECOMPRESSED_BYTES: usize = 256 * 1024 * 1024;

/// Whether a pack-relative path stays inside the pack root. Fail-closed: absolute
/// paths, `..` components, `.` components, backslashes, and Windows prefixes are
/// all rejected.
fn path_is_enclosed(path: &Path) -> bool {
    !path.as_os_str().is_empty() && path.components().all(|c| matches!(c, Component::Normal(_)))
}

fn is_single_segment(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && path_is_enclosed(Path::new(name))
}

fn is_enclosed_relative(path: &str) -> bool {
    !path.is_empty() && !path.contains('\\') && path_is_enclosed(Path::new(path))
}

/// Streams at most `max_bytes + 1` bytes from `reader` without pre-allocating
/// from untrusted metadata; returns `None` when the stream exceeds `max_bytes`.
fn read_bounded(reader: impl Read, max_bytes: usize) -> Option<Vec<u8>> {
    let mut data = Vec::new();
    reader
        .take((max_bytes as u64).saturating_add(1))
        .read_to_end(&mut data)
        .ok()?;
    (data.len() <= max_bytes).then_some(data)
}

/// `PathPackResources` over a datapack folder: `pack.mcmeta` at the root and
/// `PackType.SERVER_DATA` content under `data/`.
struct FolderPackResources {
    root: PathBuf,
    canonical_root: Option<PathBuf>,
}

impl FolderPackResources {
    fn new(root: PathBuf) -> Self {
        let canonical_root = fs::symlink_metadata(&root)
            .ok()
            .filter(|meta| !meta.file_type().is_symlink() && meta.is_dir())
            .and_then(|_| fs::canonicalize(&root).ok());
        Self {
            root,
            canonical_root,
        }
    }

    fn resolve_enclosed(&self, parts: &[&str]) -> Option<PathBuf> {
        let canonical_root = self.canonical_root.as_ref()?;
        let mut current = self.root.clone();
        for part in parts {
            if !is_enclosed_relative(part) {
                return None;
            }
            for component in Path::new(part).components() {
                let Component::Normal(segment) = component else {
                    return None;
                };
                current.push(segment);
                let meta = fs::symlink_metadata(&current).ok()?;
                if meta.file_type().is_symlink() {
                    return None;
                }
            }
        }
        let canonical = fs::canonicalize(&current).ok()?;
        canonical.starts_with(canonical_root).then_some(current)
    }

    fn open_regular_file(&self, parts: &[&str]) -> Option<File> {
        let full = self.resolve_enclosed(parts)?;
        let meta = fs::symlink_metadata(&full).ok()?;
        if meta.file_type().is_symlink() || !meta.is_file() {
            return None;
        }
        let file = File::open(&full).ok()?;
        let open_meta = file.metadata().ok()?;
        open_meta.is_file().then_some(file)
    }

    fn read_with_limit(&self, namespace: &str, path: &str, max_bytes: usize) -> Option<Vec<u8>> {
        if !is_single_segment(namespace) || !is_enclosed_relative(path) {
            return None;
        }
        let file = self.open_regular_file(&["data", namespace, path])?;
        if file.metadata().ok()?.len() > max_bytes as u64 {
            return None;
        }
        read_bounded(file, max_bytes)
    }
}

impl ServerPackResources for FolderPackResources {
    fn provided_namespaces(&self) -> Vec<String> {
        let Some(canonical_root) = self.canonical_root.as_ref() else {
            return Vec::new();
        };
        let Some(data) = self.resolve_enclosed(&["data"]) else {
            return Vec::new();
        };
        let Ok(entries) = fs::read_dir(&data) else {
            return Vec::new();
        };
        let mut namespaces: Vec<String> = entries
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_type()
                    .is_ok_and(|ft| !ft.is_symlink() && ft.is_dir())
                    && fs::canonicalize(entry.path())
                        .is_ok_and(|canonical| canonical.starts_with(canonical_root))
            })
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| is_single_segment(name))
            .collect();
        namespaces.sort();
        namespaces.dedup();
        namespaces
    }

    fn list_paths(&self, namespace: &str, dir: &str) -> Vec<String> {
        let Some(canonical_root) = self.canonical_root.as_ref() else {
            return Vec::new();
        };
        if !is_single_segment(namespace) || (!dir.is_empty() && !is_enclosed_relative(dir)) {
            return Vec::new();
        }
        // Paths stay namespace-relative: strip at the namespace root so the
        // directory prefix is kept.
        let Some(namespace_root) = self.resolve_enclosed(&["data", namespace]) else {
            return Vec::new();
        };
        let Some(start) = (if dir.is_empty() {
            Some(namespace_root.clone())
        } else {
            self.resolve_enclosed(&["data", namespace, dir])
        }) else {
            return Vec::new();
        };
        let mut paths = Vec::new();
        collect_files(canonical_root, &namespace_root, &start, &mut paths);
        paths.sort();
        paths
    }

    fn contains(&self, namespace: &str, path: &str) -> bool {
        if !is_single_segment(namespace) || !is_enclosed_relative(path) {
            return false;
        }
        self.resolve_enclosed(&["data", namespace, path])
            .is_some_and(|full| {
                fs::symlink_metadata(full)
                    .is_ok_and(|meta| !meta.file_type().is_symlink() && meta.is_file())
            })
    }

    fn read(&self, namespace: &str, path: &str) -> Option<Vec<u8>> {
        self.read_with_limit(namespace, path, MAX_PACK_RESOURCE_BYTES)
    }

    fn read_root(&self, path: &str) -> Option<Vec<u8>> {
        if !is_enclosed_relative(path) {
            return None;
        }
        let file = self.open_regular_file(&[path])?;
        if file.metadata().ok()?.len() > MAX_PACK_RESOURCE_BYTES as u64 {
            return None;
        }
        read_bounded(file, MAX_PACK_RESOURCE_BYTES)
    }

    fn open_overlay(&self, directory: &str) -> Option<Box<dyn ServerPackResources>> {
        if !is_single_segment(directory) {
            return None;
        }
        let overlay_root = self.resolve_enclosed(&[directory])?;
        let meta = fs::symlink_metadata(&overlay_root).ok()?;
        (!meta.file_type().is_symlink() && meta.is_dir())
            .then(|| Box::new(Self::new(overlay_root)) as Box<dyn ServerPackResources>)
    }
}

/// Recursive regular-file listing with pack-relative POSIX paths. Symlinked entries,
/// canonical escapes, directories deeper than `max_depth`, and files beyond `max_files`
/// are skipped fail-closed.
fn collect_files(canonical_root: &Path, base: &Path, dir: &Path, out: &mut Vec<String>) {
    collect_files_bounded(
        canonical_root,
        base,
        dir,
        0,
        MAX_PACK_DIR_DEPTH,
        MAX_PACK_FILES,
        out,
    );
}

fn collect_files_bounded(
    canonical_root: &Path,
    base: &Path,
    dir: &Path,
    depth: usize,
    max_depth: usize,
    max_files: usize,
    out: &mut Vec<String>,
) {
    if depth > max_depth || out.len() >= max_files {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        if out.len() >= max_files {
            break;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if !fs::canonicalize(&path).is_ok_and(|canonical| canonical.starts_with(canonical_root)) {
            continue;
        }
        if file_type.is_dir() {
            if depth < max_depth {
                collect_files_bounded(
                    canonical_root,
                    base,
                    &path,
                    depth + 1,
                    max_depth,
                    max_files,
                    out,
                );
            }
        } else if file_type.is_file() {
            if let Ok(relative) = path.strip_prefix(base) {
                out.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
}

/// `FilePackResources` over a `*.zip` datapack: `pack.mcmeta` at the zip root and
/// `PackType.SERVER_DATA` content under the internal `data/` directory.
///
/// Central-directory metadata is parsed once in [`Self::open`] and shared via `Arc`
/// across reads and active overlay views. Fail-closed (`zip-slip`): a zip containing
/// any entry with an absolute or `..` path is rejected as a whole at open time.
struct ZipPackResources {
    archive: zip::ZipArchive<Cursor<Arc<[u8]>>>,
    /// Sorted file entry names (directory entries excluded), validated at open.
    entries: Arc<[String]>,
    /// Either `""` for the primary pack or `"<overlay>/"` for an active overlay.
    prefix: String,
    /// Cumulative decompressed bytes read from this zip pack and all its overlays.
    decompressed_bytes: Arc<AtomicUsize>,
    max_decompressed_bytes: usize,
}

impl ZipPackResources {
    fn open(path: &Path) -> Result<Self, String> {
        let meta = fs::symlink_metadata(path)
            .map_err(|e| format!("failed to stat {}: {e}", path.display()))?;
        if meta.file_type().is_symlink() || !meta.is_file() {
            return Err(format!(
                "zip pack {} must be a regular non-symlink file",
                path.display()
            ));
        }
        if meta.len() > MAX_ZIP_ARCHIVE_BYTES as u64 {
            return Err(format!(
                "zip pack {} exceeds {MAX_ZIP_ARCHIVE_BYTES} byte archive budget",
                path.display()
            ));
        }
        let file =
            File::open(path).map_err(|e| format!("failed to open {}: {e}", path.display()))?;
        let bytes = read_bounded(file, MAX_ZIP_ARCHIVE_BYTES).ok_or_else(|| {
            format!(
                "zip pack {} exceeds {MAX_ZIP_ARCHIVE_BYTES} byte archive budget",
                path.display()
            )
        })?;
        Self::from_bytes(Arc::from(bytes))
            .map_err(|e| format!("failed to open zip {}: {e}", path.display()))
    }

    fn from_bytes(bytes: Arc<[u8]>) -> Result<Self, String> {
        Self::from_bytes_with_limits(bytes, MAX_PACK_FILES, MAX_ZIP_DECOMPRESSED_BYTES)
    }

    fn from_bytes_with_limits(
        bytes: Arc<[u8]>,
        max_entries: usize,
        max_decompressed_bytes: usize,
    ) -> Result<Self, String> {
        let archive = zip::ZipArchive::new(Cursor::new(bytes))
            .map_err(|e| format!("invalid zip archive: {e}"))?;
        if archive.len() > max_entries {
            return Err(format!(
                "zip archive has {} entries, exceeding the {max_entries} entry budget",
                archive.len()
            ));
        }
        let mut entries = Vec::with_capacity(archive.len());
        for index in 0..archive.len() {
            let Some(name) = archive.name_for_index(index) else {
                continue;
            };
            let name = name.map_err(|e| format!("invalid zip entry name at index {index}: {e}"))?;
            let trimmed = name.trim_end_matches('/');
            // zip names always use `/`; reject escapes and excessive path nesting before any use.
            if name.contains('\\') || (!trimmed.is_empty() && !path_is_enclosed(Path::new(trimmed)))
            {
                return Err(format!("zip entry {name:?} escapes the pack root"));
            }
            if !trimmed.is_empty() && trimmed.split('/').count() > MAX_PACK_DIR_DEPTH + 1 {
                return Err(format!(
                    "zip entry {name:?} exceeds the {MAX_PACK_DIR_DEPTH} directory depth budget"
                ));
            }
            if !name.ends_with('/') {
                entries.push(name.to_string());
            }
        }
        entries.sort();
        entries.dedup();
        Ok(Self {
            archive,
            entries: entries.into(),
            prefix: String::new(),
            decompressed_bytes: Arc::new(AtomicUsize::new(0)),
            max_decompressed_bytes,
        })
    }

    fn prefixed(&self, relative: &str) -> String {
        if self.prefix.is_empty() {
            relative.to_string()
        } else {
            format!("{}{relative}", self.prefix)
        }
    }

    fn read_entry(&self, name: &str) -> Option<Vec<u8>> {
        self.read_entry_with_limit(name, MAX_PACK_RESOURCE_BYTES)
    }

    fn read_entry_with_limit(&self, name: &str, max_bytes: usize) -> Option<Vec<u8>> {
        let used = self.decompressed_bytes.load(Ordering::Relaxed);
        if used >= self.max_decompressed_bytes {
            return None;
        }
        let remaining = self.max_decompressed_bytes - used;
        let cap = max_bytes.min(remaining);
        let mut archive = self.archive.clone();
        let entry = archive.by_name(name).ok()?;
        let data = read_bounded(entry, cap)?;
        let len = data.len();
        self.decompressed_bytes
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                let next = current.checked_add(len)?;
                (next <= self.max_decompressed_bytes).then_some(next)
            })
            .ok()?;
        Some(data)
    }
}

impl ServerPackResources for ZipPackResources {
    fn provided_namespaces(&self) -> Vec<String> {
        let data_prefix = self.prefixed("data/");
        let start = self
            .entries
            .partition_point(|name| name.as_str() < data_prefix.as_str());
        let namespaces: BTreeSet<&str> = self.entries[start..]
            .iter()
            .take_while(|name| name.starts_with(&data_prefix))
            .filter_map(|name| name.strip_prefix(&data_prefix))
            .filter_map(|rest| rest.split('/').next())
            .filter(|ns| is_single_segment(ns))
            .collect();
        namespaces.into_iter().map(str::to_string).collect()
    }

    fn list_paths(&self, namespace: &str, dir: &str) -> Vec<String> {
        if !is_single_segment(namespace) || (!dir.is_empty() && !is_enclosed_relative(dir)) {
            return Vec::new();
        }
        let strip = self.prefixed(&format!("data/{namespace}/"));
        let prefix = if dir.is_empty() {
            strip.clone()
        } else {
            format!("{strip}{dir}/")
        };
        let start = self
            .entries
            .partition_point(|name| name.as_str() < prefix.as_str());
        self.entries[start..]
            .iter()
            .take_while(|name| name.starts_with(&prefix))
            .filter_map(|name| name.strip_prefix(&strip).map(str::to_string))
            .collect()
    }

    fn contains(&self, namespace: &str, path: &str) -> bool {
        if !is_single_segment(namespace) || !is_enclosed_relative(path) {
            return false;
        }
        let name = self.prefixed(&format!("data/{namespace}/{path}"));
        self.entries.binary_search(&name).is_ok()
    }

    fn read(&self, namespace: &str, path: &str) -> Option<Vec<u8>> {
        if !is_single_segment(namespace) || !is_enclosed_relative(path) {
            return None;
        }
        let name = self.prefixed(&format!("data/{namespace}/{path}"));
        self.read_entry(&name)
    }

    fn read_root(&self, path: &str) -> Option<Vec<u8>> {
        if !is_enclosed_relative(path) {
            return None;
        }
        let name = self.prefixed(path);
        self.read_entry(&name)
    }

    fn open_overlay(&self, directory: &str) -> Option<Box<dyn ServerPackResources>> {
        if !is_single_segment(directory) {
            return None;
        }
        Some(Box::new(Self {
            archive: self.archive.clone(),
            entries: Arc::clone(&self.entries),
            prefix: format!("{directory}/"),
            decompressed_bytes: Arc::clone(&self.decompressed_bytes),
            max_decompressed_bytes: self.max_decompressed_bytes,
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    /// In-memory `ServerPackResources` mock.
    #[derive(Default)]
    struct MockResources {
        root_files: BTreeMap<String, Vec<u8>>,
        namespaces: Vec<String>,
    }

    impl MockResources {
        fn with_mcmeta(mcmeta: &str) -> Self {
            Self {
                root_files: BTreeMap::from([(
                    "pack.mcmeta".to_string(),
                    mcmeta.as_bytes().to_vec(),
                )]),
                namespaces: Vec::new(),
            }
        }
    }

    impl ServerPackResources for MockResources {
        fn provided_namespaces(&self) -> Vec<String> {
            self.namespaces.clone()
        }

        fn list_paths(&self, _namespace: &str, _dir: &str) -> Vec<String> {
            Vec::new()
        }

        fn read(&self, _namespace: &str, _path: &str) -> Option<Vec<u8>> {
            None
        }

        fn read_root(&self, path: &str) -> Option<Vec<u8>> {
            self.root_files.get(path).cloned()
        }
    }

    fn mock_vanilla() -> Box<dyn ServerPackResources> {
        Box::new(MockResources::default())
    }

    fn mcmeta_with_features(features: &[&str]) -> String {
        let flags: Vec<String> = features.iter().map(|f| format!("\"{f}\"")).collect();
        format!(
            "{{\"pack\": {{\"description\": \"d\", \"min_format\": 101, \"max_format\": 101}}, \"features\": {{\"enabled\": [{}]}}}}",
            flags.join(", ")
        )
    }

    const VALID_MCMETA: &str =
        "{\"pack\": {\"description\": \"d\", \"min_format\": 101, \"max_format\": 101}}";

    /// Minimal unique temp directory without extra dev-dependencies.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            static COUNTER: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "bbb-client-pack-repo-test-{}-{tag}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut cursor);
            let options = zip::write::SimpleFileOptions::default();
            for (name, data) in entries {
                writer.start_file(*name, options).unwrap();
                writer.write_all(data).unwrap();
            }
            writer.finish().unwrap();
        }
        cursor.into_inner()
    }

    fn allowed_vanilla() -> BTreeSet<String> {
        BTreeSet::from(["minecraft:vanilla".to_string()])
    }

    fn repository(
        dir: &Path,
        feature_packs: Vec<(String, Box<dyn ServerPackResources>)>,
    ) -> ServerPackRepository {
        ServerPackRepository::new(dir, mock_vanilla(), feature_packs).unwrap()
    }

    // --- construction and discovery ---

    #[test]
    fn vanilla_pack_has_hard_coded_metadata() {
        let temp = TempDir::new("vanilla-meta");
        let repo = repository(&temp.0, Vec::new());
        let vanilla = repo.get("vanilla").unwrap();
        assert_eq!(vanilla.source, ServerPackSource::BuiltIn);
        assert_eq!(
            vanilla.meta.declared_range,
            Some(DeclaredFormatRange {
                min: JAVA_26_1_SERVER_DATA_FORMAT,
                max: PackFormat::new(101, u32::MAX)
            })
        );
        assert!(vanilla.meta.compatibility().is_compatible());
        assert!(vanilla.meta.requested_features.is_empty());
        assert!(vanilla.meta.overlays.is_empty());
    }

    #[test]
    fn feature_pack_metadata_comes_from_its_mcmeta() {
        let temp = TempDir::new("feature-meta");
        let feature: Box<dyn ServerPackResources> =
            Box::new(MockResources::with_mcmeta(&mcmeta_with_features(&[
                "trade_rebalance",
            ])));
        let repo = repository(&temp.0, vec![("trade_rebalance".to_string(), feature)]);
        let pack = repo.get("trade_rebalance").unwrap();
        assert_eq!(pack.source, ServerPackSource::Feature);
        assert_eq!(
            pack.meta.requested_features,
            BTreeSet::from(["minecraft:trade_rebalance".to_string()])
        );
    }

    #[test]
    fn discovers_folder_and_zip_packs() {
        let temp = TempDir::new("discover");
        let datapacks = temp.0.join("datapacks");
        let dir_pack = datapacks.join("my_pack");
        std::fs::create_dir_all(dir_pack.join("data/minecraft/recipes")).unwrap();
        std::fs::write(dir_pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        std::fs::write(dir_pack.join("data/minecraft/recipes/a.json"), b"{}").unwrap();

        let zip_bytes = build_zip(&[
            ("pack.mcmeta", VALID_MCMETA.as_bytes()),
            ("data/minecraft/functions/run.mcfunction", b"say hi"),
        ]);
        std::fs::write(datapacks.join("zipped.zip"), zip_bytes).unwrap();

        // Non-pack entries are ignored (`PackDetector.java:46-50`).
        std::fs::write(datapacks.join("notes.txt"), b"hi").unwrap();

        let repo = repository(&datapacks, Vec::new());

        let folder = repo.get("file/my_pack").unwrap();
        assert_eq!(folder.source, ServerPackSource::World);
        assert_eq!(
            folder.resources.provided_namespaces(),
            vec!["minecraft".to_string()]
        );
        assert_eq!(
            folder.resources.list_paths("minecraft", "recipes"),
            vec!["recipes/a.json".to_string()]
        );
        assert_eq!(
            folder
                .resources
                .read("minecraft", "recipes/a.json")
                .unwrap(),
            b"{}"
        );

        let zipped = repo.get("file/zipped.zip").unwrap();
        assert_eq!(
            zipped.resources.provided_namespaces(),
            vec!["minecraft".to_string()]
        );
        assert_eq!(
            zipped.resources.list_paths("minecraft", "functions"),
            vec!["functions/run.mcfunction".to_string()]
        );
        assert_eq!(
            zipped
                .resources
                .read("minecraft", "functions/run.mcfunction")
                .unwrap(),
            b"say hi"
        );

        assert!(repo.get("file/notes.txt").is_none());
    }

    // --- selection (MinecraftServer.configurePackRepository) ---

    #[test]
    fn safe_mode_selects_exactly_vanilla() {
        let temp = TempDir::new("safe-mode");
        let datapacks = temp.0.join("datapacks");
        let world_pack = datapacks.join("extra");
        std::fs::create_dir_all(&world_pack).unwrap();
        std::fs::write(world_pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();

        let repo = repository(&datapacks, Vec::new());
        let selection = repo.select_inner(&["file/extra".into()], &[], true, &allowed_vanilla());
        assert_eq!(selection.ordered_ids, vec!["vanilla".to_string()]);
        assert_eq!(
            selection.data_packs(),
            &Java26DataPacks::new(vec!["vanilla".to_string()], Vec::new())
        );
    }

    #[test]
    fn missing_enabled_pack_is_dropped_with_notice() {
        let temp = TempDir::new("missing");
        let repo = repository(&temp.0, Vec::new());
        let selection = repo.select_inner(
            &["vanilla".into(), "file/gone".into()],
            &[],
            false,
            &allowed_vanilla(),
        );
        assert_eq!(selection.ordered_ids, vec!["vanilla".to_string()]);
        assert!(selection
            .notices
            .contains(&PackSelectionNotice::MissingEnabled("file/gone".into())));
    }

    #[test]
    fn world_pack_is_auto_added() {
        let temp = TempDir::new("auto-add");
        let datapacks = temp.0.join("datapacks");
        let world_pack = datapacks.join("extra");
        std::fs::create_dir_all(&world_pack).unwrap();
        std::fs::write(world_pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();

        let repo = repository(&datapacks, Vec::new());
        let selection = repo.select_inner(&["vanilla".into()], &[], false, &allowed_vanilla());
        assert_eq!(
            selection.ordered_ids,
            vec!["vanilla".to_string(), "file/extra".to_string()]
        );
        assert_eq!(selection.data_packs().enabled(), selection.ordered_ids);
        assert!(selection.data_packs().disabled().is_empty());
        assert!(selection
            .notices
            .contains(&PackSelectionNotice::AutoAdded("file/extra".to_string())));
    }

    #[test]
    fn world_pack_with_missing_features_is_not_auto_added() {
        let temp = TempDir::new("auto-block");
        let datapacks = temp.0.join("datapacks");
        let world_pack = datapacks.join("experimental");
        std::fs::create_dir_all(&world_pack).unwrap();
        std::fs::write(
            world_pack.join("pack.mcmeta"),
            mcmeta_with_features(&["trade_rebalance"]),
        )
        .unwrap();

        let repo = repository(&datapacks, Vec::new());
        let selection = repo.select_inner(&["vanilla".into()], &[], false, &allowed_vanilla());
        assert_eq!(selection.ordered_ids, vec!["vanilla".to_string()]);
        assert_eq!(
            selection.data_packs().disabled(),
            ["file/experimental".to_string()]
        );
        assert!(selection
            .notices
            .contains(&PackSelectionNotice::AutoAddBlockedMissingFeatures(
                "file/experimental".to_string()
            )));
    }

    #[test]
    fn feature_pack_is_never_auto_added() {
        // `PackSource.FEATURE.shouldAddAutomatically` is false even when the
        // requested features are allowed.
        let temp = TempDir::new("feature-noauto");
        let feature: Box<dyn ServerPackResources> =
            Box::new(MockResources::with_mcmeta(VALID_MCMETA));
        let repo = repository(&temp.0, vec![("some_feature".to_string(), feature)]);
        let selection = repo.select_inner(&["vanilla".into()], &[], false, &allowed_vanilla());
        assert_eq!(selection.ordered_ids, vec!["vanilla".to_string()]);
        assert_eq!(
            selection.data_packs().disabled(),
            ["some_feature".to_string()]
        );
    }

    #[test]
    fn selected_pack_with_missing_features_is_removed() {
        let temp = TempDir::new("selected-missing");
        let feature: Box<dyn ServerPackResources> =
            Box::new(MockResources::with_mcmeta(&mcmeta_with_features(&[
                "trade_rebalance",
            ])));
        let repo = repository(&temp.0, vec![("trade_rebalance".to_string(), feature)]);
        let selection = repo.select_inner(
            &["vanilla".into(), "trade_rebalance".into()],
            &[],
            false,
            &allowed_vanilla(),
        );
        assert_eq!(selection.ordered_ids, vec!["vanilla".to_string()]);
        assert!(selection
            .notices
            .contains(&PackSelectionNotice::DisabledMissingFeatures(
                "trade_rebalance".to_string()
            )));
    }

    #[test]
    fn empty_selection_forces_vanilla() {
        let temp = TempDir::new("forced");
        let repo = repository(&temp.0, Vec::new());
        let selection = repo.select_inner(&[], &["vanilla".into()], false, &allowed_vanilla());
        assert_eq!(selection.ordered_ids, vec!["vanilla".to_string()]);
        assert!(selection
            .notices
            .contains(&PackSelectionNotice::ForcedVanilla));
    }

    #[test]
    fn disabled_world_pack_is_not_auto_added() {
        let temp = TempDir::new("disabled");
        let datapacks = temp.0.join("datapacks");
        let world_pack = datapacks.join("extra");
        std::fs::create_dir_all(&world_pack).unwrap();
        std::fs::write(world_pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();

        let repo = repository(&datapacks, Vec::new());
        let selection = repo.select_inner(
            &["vanilla".into()],
            &["file/extra".into()],
            false,
            &allowed_vanilla(),
        );
        assert_eq!(selection.ordered_ids, vec!["vanilla".to_string()]);
        assert_eq!(
            selection.data_packs().disabled(),
            ["file/extra".to_string()]
        );
    }

    #[test]
    fn reload_appends_new_available_packs_but_keeps_effective_disabled_ids() {
        let temp = TempDir::new("reload-selection");
        let datapacks = temp.0.join("datapacks");
        for name in ["blocked", "new"] {
            let pack = datapacks.join(name);
            std::fs::create_dir_all(&pack).unwrap();
            std::fs::write(pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        }
        let repo = repository(&datapacks, Vec::new());
        let current = Java26DataPacks::new(
            vec!["vanilla".to_string(), "file/missing".to_string()],
            vec!["file/blocked".to_string()],
        );

        let selection = repo.select_reload(&current);
        assert_eq!(
            selection.ordered_ids,
            ["vanilla".to_string(), "file/new".to_string()]
        );
        assert_eq!(selection.data_packs().enabled(), selection.ordered_ids);
        assert_eq!(
            selection.data_packs().disabled(),
            ["file/blocked".to_string()]
        );
    }

    #[test]
    fn a_selection_keeps_the_order_it_was_given() {
        let temp = TempDir::new("ordering");
        let datapacks = temp.0.join("datapacks");
        for name in ["aaa", "zzz"] {
            let dir = datapacks.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        }
        let repo = repository(&datapacks, Vec::new());
        // `rebuildSelected` (`PackRepository.java:82-96`) reorders nothing: only a
        // missing `required` pack is position-inserted, and no server pack is
        // required, so vanilla stays where the enabled list put it.
        let selection = repo.select_inner(
            &["file/aaa".into(), "vanilla".into(), "file/zzz".into()],
            &[],
            false,
            &allowed_vanilla(),
        );
        assert_eq!(
            selection.ordered_ids,
            vec![
                "file/aaa".to_string(),
                "vanilla".to_string(),
                "file/zzz".to_string()
            ]
        );
        let opened = repo.open_selected(&selection);
        assert_eq!(
            opened.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            vec!["file/aaa", "vanilla", "file/zzz"]
        );
    }

    #[test]
    fn select_ids_takes_the_named_set_and_disables_the_rest() {
        let temp = TempDir::new("select-ids");
        let datapacks = temp.0.join("datapacks");
        for name in ["aaa", "zzz"] {
            let dir = datapacks.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        }
        let repo = repository(&datapacks, Vec::new());
        // `setSelected` neither re-discovers nor auto-adds: an unnamed available
        // pack lands in `Disabled`, and an id that is not available is dropped.
        let selection = repo.select_ids(&[
            "file/zzz".to_string(),
            "vanilla".to_string(),
            "file/gone".to_string(),
        ]);
        assert_eq!(
            selection.ordered_ids,
            ["file/zzz".to_string(), "vanilla".to_string()]
        );
        assert_eq!(selection.data_packs().enabled(), selection.ordered_ids);
        assert_eq!(selection.data_packs().disabled(), ["file/aaa".to_string()]);
    }

    #[test]
    fn position_insert_puts_a_bottom_pack_first_and_a_top_pack_last() {
        // `Pack.Position.insert` (`Pack.java:145-170`) is what the bare
        // `/datapack enable <name>` arm applies (`DataPackCommand.java:107`).
        let mut list = vec![ServerPackSource::World, ServerPackSource::Feature];
        let (_, bottom) = selection_config(ServerPackSource::BuiltIn);
        assert_eq!(
            position_insert(&mut list, ServerPackSource::BuiltIn, bottom, |source| {
                selection_config(*source)
            }),
            0
        );
        let (_, top) = selection_config(ServerPackSource::World);
        assert_eq!(
            position_insert(&mut list, ServerPackSource::World, top, |source| {
                selection_config(*source)
            }),
            3
        );
        assert_eq!(
            list,
            [
                ServerPackSource::BuiltIn,
                ServerPackSource::World,
                ServerPackSource::Feature,
                ServerPackSource::World,
            ]
        );
    }

    /// End-to-end D1 loop: the embedded vanilla pack + real feature packs + a world
    /// datapack discovered from a temp `<world>/datapacks` dir drive the overlay
    /// merge and the registry load (`WorldLoader` pipeline equivalents).
    #[test]
    fn repository_drives_embedded_vanilla_generation_load() {
        use crate::registry_data_loader::{load_dynamic_registries, DatapackLoadLimits};
        use crate::resource_overlay::{MergedServerDataView, PackOverlayInput};
        use crate::vanilla_snapshot::VanillaDatapackSnapshot;
        use std::sync::Arc;

        let world = TempDir::new("e2e");
        let datapacks = world.0.join("datapacks");
        let override_pack = datapacks.join("override");
        std::fs::create_dir_all(override_pack.join("data/minecraft/damage_type")).unwrap();
        std::fs::create_dir_all(override_pack.join("data/minecraft/tags/damage_type")).unwrap();
        std::fs::write(override_pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        std::fs::write(
            override_pack.join("data/minecraft/damage_type/cactus.json"),
            "{\"message_id\":\"cactus\",\"scaling\":\"never\",\"exhaustion\":0.0}",
        )
        .unwrap();
        // D2: the world pack appends to a vanilla tag (accumulation, no replace).
        std::fs::write(
            override_pack.join("data/minecraft/tags/damage_type/is_explosion.json"),
            "{\"values\":[\"minecraft:cactus\"]}",
        )
        .unwrap();

        let snapshot = Arc::new(VanillaDatapackSnapshot::embedded().unwrap());
        let feature_packs: Vec<(String, Box<dyn ServerPackResources>)> = snapshot
            .feature_pack_ids()
            .into_iter()
            .map(|id| {
                (
                    id.clone(),
                    Box::new(snapshot.feature_pack(&id).unwrap()) as Box<dyn ServerPackResources>,
                )
            })
            .collect();
        let repo =
            ServerPackRepository::new(&datapacks, Box::new(snapshot.vanilla_pack()), feature_packs)
                .unwrap();
        assert!(repo.get("vanilla").is_some());
        assert!(repo.get("trade_rebalance").is_some());
        assert!(repo.get("file/override").is_some());

        let data_packs = Java26DataPacks::new(vec!["vanilla".to_string()], Vec::new());
        let selection = repo.select(&data_packs, false, &allowed_vanilla());
        // The world pack auto-adds (`PackSource.WORLD`); feature packs do not
        // (`PackSource.java:10-17`).
        assert_eq!(
            selection.ordered_ids,
            vec!["vanilla".to_string(), "file/override".to_string()]
        );
        assert!(selection
            .notices
            .contains(&PackSelectionNotice::AutoAdded("file/override".to_string())));

        let opened = repo.open_selected(&selection);
        let inputs: Vec<PackOverlayInput<'_>> = opened
            .iter()
            .map(|pack| PackOverlayInput {
                pack_id: &pack.id,
                resources: pack.resources.as_ref(),
                filter: Some(&pack.meta.filter),
            })
            .collect();
        let view = MergedServerDataView::merge(&inputs);
        let generation =
            load_dynamic_registries(&view, &inputs, DatapackLoadLimits::default()).unwrap();

        let damage_type = generation.registry("minecraft:damage_type").unwrap();
        let cactus = damage_type
            .elements
            .iter()
            .find(|e| e.id == "minecraft:cactus")
            .unwrap();
        assert_eq!(cactus.source_pack, "file/override");
        assert_eq!(
            cactus
                .json()
                .unwrap()
                .get("scaling")
                .unwrap()
                .utf8_string()
                .as_deref(),
            Some("never")
        );

        // D2: tags accumulate across the stacked pack view and nested `#` references
        // resolve through the dependency-ordered build (`TagLoader.java:48-116`).
        let tags = crate::tag_loader::load_tags_for_generation(
            &inputs,
            &generation,
            DatapackLoadLimits::default(),
        );
        assert_eq!(tags.warnings, Vec::new());
        let is_end = tags
            .registry("minecraft:worldgen/biome")
            .and_then(|registry| registry.tag("minecraft:is_end"))
            .unwrap();
        assert_eq!(
            is_end,
            [
                "minecraft:the_end",
                "minecraft:end_highlands",
                "minecraft:end_midlands",
                "minecraft:small_end_islands",
                "minecraft:end_barrens"
            ]
        );
        let damage_tags = tags.registry("minecraft:damage_type").unwrap();
        assert_eq!(
            damage_tags.tag("minecraft:is_explosion").unwrap(),
            [
                "minecraft:fireworks",
                "minecraft:explosion",
                "minecraft:player_explosion",
                "minecraft:bad_respawn_point",
                // Appended by the world datapack after the vanilla entries.
                "minecraft:cactus"
            ]
        );
        assert_eq!(
            damage_tags
                .tag("minecraft:always_hurts_ender_dragons")
                .unwrap(),
            damage_tags.tag("minecraft:is_explosion").unwrap()
        );

        // Safe mode forces the vanilla-only selection (`MinecraftServer.java:1559-1561`).
        let safe = repo.select(&data_packs, true, &allowed_vanilla());
        assert_eq!(safe.ordered_ids, vec!["vanilla".to_string()]);
    }

    /// End-to-end D3 loop: the embedded vanilla pack plus a world datapack drive the
    /// three loot registries (`ReloadableServerRegistries` + `scanDirectory`
    /// equivalents): built-in tables decode clean, the world pack overrides by
    /// winner-take-all, supplies predicate/item_modifier content the built-in pack
    /// lacks, and an unknown mod codec id is rejected warn-only.
    #[test]
    fn repository_drives_embedded_vanilla_loot_load() {
        use crate::loot::{load_loot_registries, LootElementValue};
        use crate::registry_data_loader::DatapackLoadLimits;
        use crate::resource_overlay::{MergedServerDataView, PackOverlayInput};
        use crate::vanilla_snapshot::VanillaDatapackSnapshot;
        use std::sync::Arc;

        let world = TempDir::new("e2e-loot");
        let datapacks = world.0.join("datapacks");
        let pack = datapacks.join("extras");
        std::fs::create_dir_all(pack.join("data/minecraft/loot_table/entities")).unwrap();
        std::fs::create_dir_all(pack.join("data/minecraft/predicate")).unwrap();
        std::fs::create_dir_all(pack.join("data/minecraft/item_modifier")).unwrap();
        std::fs::write(pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        std::fs::write(
            pack.join("data/minecraft/loot_table/entities/zombie.json"),
            "{\"type\":\"minecraft:entity\",\"pools\":[{\"rolls\":1,\"entries\":[{\"type\":\"minecraft:empty\"}]}]}",
        )
        .unwrap();
        std::fs::write(
            pack.join("data/minecraft/predicate/is_extra.json"),
            "{\"condition\":\"minecraft:survives_explosion\"}",
        )
        .unwrap();
        std::fs::write(
            pack.join("data/minecraft/item_modifier/bump.json"),
            "{\"function\":\"minecraft:set_count\",\"count\":2}",
        )
        .unwrap();
        // Unknown mod codec ids are rejected; the element is dropped warn-only.
        std::fs::write(
            pack.join("data/minecraft/loot_table/modded.json"),
            "{\"pools\":[{\"rolls\":1,\"entries\":[{\"type\":\"modded:entry\"}]}]}",
        )
        .unwrap();

        let snapshot = Arc::new(VanillaDatapackSnapshot::embedded().unwrap());
        let repo =
            ServerPackRepository::new(&datapacks, Box::new(snapshot.vanilla_pack()), Vec::new())
                .unwrap();
        let data_packs = Java26DataPacks::new(vec!["vanilla".to_string()], Vec::new());
        let selection = repo.select(&data_packs, false, &allowed_vanilla());
        let opened = repo.open_selected(&selection);
        let inputs: Vec<PackOverlayInput<'_>> = opened
            .iter()
            .map(|pack| PackOverlayInput {
                pack_id: &pack.id,
                resources: pack.resources.as_ref(),
                filter: Some(&pack.meta.filter),
            })
            .collect();
        let view = MergedServerDataView::merge(&inputs);
        let loot = load_loot_registries(&view, &inputs, DatapackLoadLimits::default(), None);

        // The 1326 built-in tables decode clean; the override replaces zombie
        // winner-take-all and the modded table is dropped with one attributed
        // warning, so the total stays at the built-in count.
        let tables = loot.registry("minecraft:loot_table").unwrap();
        assert_eq!(tables.elements.len(), 1326);
        assert_eq!(loot.warnings.len(), 1);
        assert_eq!(
            loot.warnings[0].element_id.as_deref(),
            Some("minecraft:modded")
        );
        assert!(loot.warnings[0].message.contains("unknown entry type"));
        assert_eq!(loot.tag_warnings, Vec::new());
        let zombie = tables
            .elements
            .iter()
            .find(|element| element.id == "minecraft:entities/zombie")
            .unwrap();
        assert_eq!(zombie.source_pack, "file/extras");
        let LootElementValue::Table(table) = &zombie.value else {
            panic!("zombie must decode as a table");
        };
        assert_eq!(table.param_set, "minecraft:entity");
        assert_eq!(
            table.pools[0].entries[0].kind,
            crate::loot::LootEntryKind::Empty
        );
        // A built-in table keeps its vanilla winner and structure.
        let skeleton = tables
            .elements
            .iter()
            .find(|element| element.id == "minecraft:entities/skeleton")
            .unwrap();
        assert_eq!(skeleton.source_pack, "vanilla");

        // Predicate and item modifier registries load the world-pack content the
        // built-in pack does not ship.
        let predicates = loot.registry("minecraft:predicate").unwrap();
        assert_eq!(predicates.elements.len(), 1);
        let LootElementValue::Predicate(condition) = &predicates.elements[0].value else {
            panic!("is_extra must decode as a predicate");
        };
        assert_eq!(**condition, crate::loot::LootCondition::SurvivesExplosion);
        let modifiers = loot.registry("minecraft:item_modifier").unwrap();
        assert_eq!(modifiers.elements.len(), 1);
        let LootElementValue::Modifier(function) = &modifiers.elements[0].value else {
            panic!("bump must decode as an item modifier");
        };
        assert!(matches!(
            function.kind,
            crate::loot::LootFunctionKind::SetCount { .. }
        ));
        // The built-in pack ships no tags for the loot registries.
        assert!(tables.tags.is_none());
        assert!(predicates.tags.is_none());
    }

    /// End-to-end D4 loop: the embedded vanilla pack plus a world datapack drive the
    /// recipe, advancement and function surfaces (`RecipeManager`,
    /// `ServerAdvancementManager` + `AdvancementTree`, `ServerFunctionLibrary`
    /// equivalents): built-in content decodes clean, the world pack overrides by
    /// winner-take-all, unknown codec ids are rejected warn-only, orphan
    /// advancements are tree-pruned, and functions load with `tags/function`.
    #[test]
    fn repository_drives_embedded_vanilla_recipe_advancement_function_load() {
        use crate::advancement::load_advancements;
        use crate::frozen_lookup::FrozenDatapackRegistryLookup;
        use crate::function::load_function_library;
        use crate::recipe::load_recipes_with_lookup;
        use crate::registry_data_loader::DatapackLoadLimits;
        use crate::resource_overlay::{MergedServerDataView, PackOverlayInput};
        use crate::startup::{DatapackStartup, DatapackStartupOptions};
        use crate::vanilla_snapshot::VanillaDatapackSnapshot;
        use std::sync::Arc;

        let world = TempDir::new("e2e-d4");
        let datapacks = world.0.join("datapacks");
        let pack = datapacks.join("tweaks");
        std::fs::create_dir_all(pack.join("data/minecraft/recipe")).unwrap();
        std::fs::create_dir_all(pack.join("data/minecraft/advancement/adventure")).unwrap();
        std::fs::create_dir_all(pack.join("data/minecraft/function")).unwrap();
        std::fs::create_dir_all(pack.join("data/minecraft/tags/function")).unwrap();
        std::fs::write(pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        // Winner-take-all override of a built-in recipe.
        std::fs::write(
            pack.join("data/minecraft/recipe/crafting_table.json"),
            r#"{"type":"minecraft:crafting_shapeless","ingredients":["minecraft:oak_planks"],"result":{"id":"minecraft:crafting_table"}}"#,
        )
        .unwrap();
        // Unknown serializer ids are rejected; the element is dropped warn-only.
        std::fs::write(
            pack.join("data/minecraft/recipe/modded.json"),
            "{\"type\":\"modded:machine\"}",
        )
        .unwrap();
        // An advancement whose parent never loads is tree-pruned warn-only.
        std::fs::write(
            pack.join("data/minecraft/advancement/adventure/orphan.json"),
            "{\"parent\":\"minecraft:adventure/nope\",\"criteria\":{\"x\":{\"trigger\":\"minecraft:impossible\"}}}",
        )
        .unwrap();
        // Unknown trigger ids are rejected warn-only.
        std::fs::write(
            pack.join("data/minecraft/advancement/adventure/bad_trigger.json"),
            "{\"criteria\":{\"x\":{\"trigger\":\"modded:trigger\"}}}",
        )
        .unwrap();
        std::fs::write(
            pack.join("data/minecraft/function/hello.mcfunction"),
            "# comment\nsay hi \\\nthere\n",
        )
        .unwrap();
        // A `/`-prefixed function line is rejected warn-only.
        std::fs::write(
            pack.join("data/minecraft/function/bad.mcfunction"),
            "/say nope\n",
        )
        .unwrap();
        std::fs::write(
            pack.join("data/minecraft/tags/function/tick.json"),
            "{\"values\":[\"minecraft:hello\"]}",
        )
        .unwrap();

        let snapshot = Arc::new(VanillaDatapackSnapshot::embedded().unwrap());
        let repo =
            ServerPackRepository::new(&datapacks, Box::new(snapshot.vanilla_pack()), Vec::new())
                .unwrap();
        let data_packs = Java26DataPacks::new(vec!["vanilla".to_string()], Vec::new());
        let selection = repo.select(&data_packs, false, &allowed_vanilla());
        let opened = repo.open_selected(&selection);
        let inputs: Vec<PackOverlayInput<'_>> = opened
            .iter()
            .map(|pack| PackOverlayInput {
                pack_id: &pack.id,
                resources: pack.resources.as_ref(),
                filter: Some(&pack.meta.filter),
            })
            .collect();
        let view = MergedServerDataView::merge(&inputs);
        let limits = DatapackLoadLimits::default();

        // The 1515 built-in recipes decode clean against the frozen built-in
        // publication their result defaults name; the override replaces
        // crafting_table winner-take-all and the modded recipe drops with one
        // attributed warning, so the total stays at the built-in count.
        let publication = DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits,
        })
        .unwrap();
        let lookup = FrozenDatapackRegistryLookup::from_startup(&publication);
        let recipes = load_recipes_with_lookup(&view, &inputs, limits, &lookup);
        assert_eq!(recipes.recipes.len(), 1515);
        assert_eq!(recipes.warnings.len(), 1);
        assert_eq!(recipes.warnings[0].element_id, "minecraft:modded");
        assert!(recipes.warnings[0]
            .message
            .contains("unknown recipe serializer type modded:machine"));
        let crafting_table = recipes.recipe("minecraft:crafting_table").unwrap();
        assert_eq!(crafting_table.source_pack, "file/tweaks");
        assert_eq!(
            crafting_table.recipe.type_id,
            "minecraft:crafting_shapeless"
        );
        let acacia = recipes.recipe("minecraft:acacia_planks").unwrap();
        assert_eq!(acacia.source_pack, "vanilla");
        assert_eq!(acacia.recipe.type_id, "minecraft:crafting_shapeless");

        // The 1617 built-in advancements decode clean and form a complete tree; the
        // bad trigger and the orphan drop with one attributed warning each.
        let advancements = load_advancements(&view, &inputs, limits, Some(&lookup));
        assert_eq!(advancements.advancements.len(), 1617);
        assert_eq!(advancements.warnings.len(), 2);
        assert!(advancements
            .warnings
            .iter()
            .any(
                |warning| warning.element_id == "minecraft:adventure/bad_trigger"
                    && warning
                        .message
                        .contains("unknown criterion trigger modded:trigger")
            ));
        assert!(advancements
            .warnings
            .iter()
            .any(|warning| warning.element_id == "minecraft:adventure/orphan"
                && warning.message.contains("orphan advancement dropped")));
        let root = advancements
            .advancement("minecraft:adventure/root")
            .unwrap();
        assert_eq!(root.source_pack, "vanilla");
        assert!(root.advancement.parent.is_none());
        let shoot_arrow = advancements
            .advancement("minecraft:adventure/shoot_arrow")
            .unwrap();
        assert_eq!(
            shoot_arrow.advancement.parent.as_deref(),
            Some("minecraft:adventure/kill_a_mob")
        );

        // Functions: the world pack supplies the whole `function/` tree (the
        // built-in pack ships none); the bad file drops warn-only and
        // `tags/function` holds entries that bind against the loaded ids.
        let library = load_function_library(&view, &inputs, limits);
        assert_eq!(library.functions.len(), 1);
        assert_eq!(library.warnings.len(), 1);
        assert_eq!(library.warnings[0].element_id, "minecraft:bad");
        assert!(library.warnings[0]
            .message
            .contains("do not use a preceding forwards slash"));
        let hello = library.function("minecraft:hello").unwrap();
        assert_eq!(hello.source.lines.len(), 1);
        assert_eq!(
            hello.source.lines[0].line.as_ref(),
            "say hi there".encode_utf16().collect::<Vec<_>>()
        );
        let entries = library.tags.as_ref().unwrap();
        let loaded_ids: Vec<&str> = library
            .functions
            .iter()
            .map(|function| function.id.as_str())
            .collect();
        let tags =
            crate::tag_loader::build_registry_tags(entries, &loaded_ids, &mut Vec::new()).unwrap();
        assert_eq!(tags.tag("minecraft:tick").unwrap(), ["minecraft:hello"]);
        assert_eq!(library.tag_warnings, Vec::new());
    }

    /// End-to-end D5 loop: the embedded vanilla pack plus a world datapack drive the
    /// worldgen dispatch-envelope validation and the structure template surface.
    /// Every dispatched built-in worldgen element passes the 26.1 whitelists
    /// fail-closed clean; an unknown mod codec id aborts the load with attribution;
    /// the 1202 built-in structure templates decode through the bounded NBT
    /// envelope, and a malformed world-pack template drops warn-only.
    #[test]
    fn repository_drives_embedded_vanilla_worldgen_and_structure_load() {
        use crate::registry_data_loader::{load_dynamic_registries, DatapackLoadLimits};
        use crate::resource_overlay::{MergedServerDataView, PackOverlayInput};
        use crate::structure_template::load_structure_templates;
        use crate::vanilla_snapshot::VanillaDatapackSnapshot;
        use std::sync::Arc;

        let world = TempDir::new("e2e-d5");
        let datapacks = world.0.join("datapacks");
        let pack = datapacks.join("worldgen");
        std::fs::create_dir_all(pack.join("data/minecraft/dimension")).unwrap();
        std::fs::create_dir_all(pack.join("data/minecraft/structure")).unwrap();
        std::fs::write(pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        // A custom dimension with a noise generator envelope passes validation.
        std::fs::write(
            pack.join("data/minecraft/dimension/sky.json"),
            "{\"type\":\"minecraft:overworld\",\"generator\":{\"type\":\"minecraft:noise\",\"biome_source\":{\"type\":\"minecraft:multi_noise\"}}}",
        )
        .unwrap();
        // Not gzip NBT at all: dropped warn-only.
        std::fs::write(pack.join("data/minecraft/structure/broken.nbt"), "not nbt").unwrap();

        let snapshot = Arc::new(VanillaDatapackSnapshot::embedded().unwrap());
        let repo =
            ServerPackRepository::new(&datapacks, Box::new(snapshot.vanilla_pack()), Vec::new())
                .unwrap();
        let data_packs = Java26DataPacks::new(vec!["vanilla".to_string()], Vec::new());
        let selection = repo.select(&data_packs, false, &allowed_vanilla());
        let opened = repo.open_selected(&selection);
        let inputs: Vec<PackOverlayInput<'_>> = opened
            .iter()
            .map(|pack| PackOverlayInput {
                pack_id: &pack.id,
                resources: pack.resources.as_ref(),
                filter: Some(&pack.meta.filter),
            })
            .collect();
        let view = MergedServerDataView::merge(&inputs);
        let limits = DatapackLoadLimits::default();

        // Every dispatched built-in worldgen element (221 configured features, 258
        // placed features, 34 structures, 40 processor lists, 188 template pools,
        // 35 density functions, 7 noise settings, 4 carvers) passes the 26.1
        // dispatch whitelists; the custom dimension joins the LEVEL_STEM registry.
        let generation = load_dynamic_registries(&view, &inputs, limits).unwrap();
        let dimension = generation.registry("minecraft:dimension").unwrap();
        assert_eq!(dimension.elements.len(), 1);
        assert_eq!(dimension.elements[0].id, "minecraft:sky");
        assert_eq!(dimension.elements[0].source_pack, "file/worldgen");

        // Structure templates: the 1202 built-in `.nbt` files decode through the
        // bounded envelope reader; the malformed world-pack file drops warn-only.
        let templates = load_structure_templates(&view, &inputs, limits);
        assert_eq!(templates.templates.len(), 1202);
        assert_eq!(templates.warnings.len(), 1);
        assert_eq!(templates.warnings[0].element_id, "minecraft:broken");
        assert_eq!(templates.warnings[0].pack, "file/worldgen");
        let empty = templates.template("minecraft:empty").unwrap();
        assert_eq!(empty.source_pack, "vanilla");
        assert_eq!(empty.template.size, [1, 1, 1]);

        // An unknown mod codec id in a dispatched registry fails the whole load
        // closed with attribution (`RegistryDataLoader` loadingErrors equivalent).
        let bad = datapacks.join("modded");
        std::fs::create_dir_all(bad.join("data/minecraft/worldgen/configured_feature")).unwrap();
        std::fs::write(bad.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        std::fs::write(
            bad.join("data/minecraft/worldgen/configured_feature/machine.json"),
            "{\"type\":\"modded:machine\",\"config\":{}}",
        )
        .unwrap();
        let repo2 =
            ServerPackRepository::new(&datapacks, Box::new(snapshot.vanilla_pack()), Vec::new())
                .unwrap();
        let selection2 = repo2.select(&data_packs, false, &allowed_vanilla());
        let opened2 = repo2.open_selected(&selection2);
        let inputs2: Vec<PackOverlayInput<'_>> = opened2
            .iter()
            .map(|pack| PackOverlayInput {
                pack_id: &pack.id,
                resources: pack.resources.as_ref(),
                filter: Some(&pack.meta.filter),
            })
            .collect();
        let view2 = MergedServerDataView::merge(&inputs2);
        let error = load_dynamic_registries(&view2, &inputs2, limits).unwrap_err();
        assert_eq!(error.errors.len(), 1, "{error}");
        assert_eq!(
            error.errors[0].registry_id,
            "minecraft:worldgen/configured_feature"
        );
        assert_eq!(
            error.errors[0].element_id.as_deref(),
            Some("minecraft:machine")
        );
        assert!(error.errors[0].message.contains("modded:machine"));
    }

    #[test]
    fn zip_pack_caches_directory_streams_bounds_and_rejects_traversal() {
        let temp = TempDir::new("zip-security");
        let zip_path = temp.0.join("pack.zip");
        let zip_bytes = build_zip(&[
            ("pack.mcmeta", VALID_MCMETA.as_bytes()),
            ("data/minecraft/recipes/small.json", b"{}"),
            ("data/minecraft/recipes/large.json", b"0123456789abcdef"),
            ("overlay/data/minecraft/recipes/small.json", b"{\"v\":2}"),
        ]);
        std::fs::write(&zip_path, &zip_bytes).unwrap();

        let opened = ZipPackResources::open(&zip_path).unwrap();
        assert_eq!(opened.entries.len(), 4);
        assert!(opened.contains("minecraft", "recipes/small.json"));
        assert!(!opened.contains("minecraft", "../escape.json"));
        assert_eq!(
            opened
                .read_entry_with_limit("data/minecraft/recipes/small.json", 8)
                .as_deref(),
            Some(&b"{}"[..])
        );
        assert_eq!(
            opened.read_entry_with_limit("data/minecraft/recipes/large.json", 8),
            None
        );

        let overlay = opened.open_overlay("overlay").unwrap();
        assert_eq!(
            overlay.read("minecraft", "recipes/small.json").as_deref(),
            Some(&b"{\"v\":2}"[..])
        );
        assert!(opened.open_overlay("..").is_none());

        // Deleting the file after open proves reads use the archive cached at open time.
        std::fs::remove_file(&zip_path).unwrap();
        assert_eq!(
            opened.read("minecraft", "recipes/small.json").as_deref(),
            Some(&b"{}"[..])
        );

        // Zip-slip entry fails open.
        let bad_zip = build_zip(&[
            ("pack.mcmeta", VALID_MCMETA.as_bytes()),
            ("../outside.txt", b"nope"),
        ]);
        std::fs::write(&zip_path, bad_zip).unwrap();
        assert!(ZipPackResources::open(&zip_path).is_err());
    }

    #[test]
    fn folder_pack_streams_bounds_and_rejects_traversal_and_symlinks() {
        let temp = TempDir::new("folder-security");
        let datapacks = temp.0.join("datapacks");
        let pack = datapacks.join("safe");
        let outside = temp.0.join("outside");
        std::fs::create_dir_all(pack.join("data/minecraft/recipes")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        std::fs::write(pack.join("data/minecraft/recipes/ok.json"), b"{}").unwrap();
        std::fs::write(
            pack.join("data/minecraft/recipes/too_big.json"),
            b"0123456789abcdef",
        )
        .unwrap();
        std::fs::write(outside.join("secret.json"), b"{\"secret\":true}").unwrap();

        let resources = FolderPackResources::new(pack.clone());
        assert_eq!(
            resources
                .read_with_limit("minecraft", "recipes/ok.json", 8)
                .as_deref(),
            Some(&b"{}"[..])
        );
        assert_eq!(
            resources.read_with_limit("minecraft", "recipes/too_big.json", 8),
            None
        );
        assert!(!resources.contains("minecraft", "../pack.mcmeta"));
        assert_eq!(resources.read("minecraft", "../pack.mcmeta"), None);
        assert_eq!(resources.read("..", "secret.json"), None);
        assert!(resources.open_overlay("..").is_none());

        // Test symlink rejection on hosts that allow unprivileged symlink creation.
        let link_file = pack.join("data/minecraft/recipes/link.json");
        let linked = {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(outside.join("secret.json"), &link_file).is_ok()
            }
            #[cfg(windows)]
            {
                std::os::windows::fs::symlink_file(outside.join("secret.json"), &link_file).is_ok()
            }
        };
        if linked {
            assert!(!resources.contains("minecraft", "recipes/link.json"));
            assert_eq!(resources.read("minecraft", "recipes/link.json"), None);
            assert_eq!(
                resources.list_paths("minecraft", "recipes"),
                vec![
                    "recipes/ok.json".to_string(),
                    "recipes/too_big.json".to_string()
                ]
            );
        }

        let linked_pack = datapacks.join("linked_pack");
        let dir_linked = {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&outside, &linked_pack).is_ok()
            }
            #[cfg(windows)]
            {
                std::os::windows::fs::symlink_dir(&outside, &linked_pack).is_ok()
            }
        };
        if dir_linked {
            let repo = repository(&datapacks, Vec::new());
            assert!(repo.get("file/linked_pack").is_none());
        }
    }

    #[test]
    fn folder_pack_enforces_dir_depth_and_file_count_budgets() {
        let temp = TempDir::new("folder-budgets");
        let pack = temp.0.join("bounded");
        let base = pack.join("data/minecraft");
        std::fs::create_dir_all(base.join("a/b/c")).unwrap();
        std::fs::write(base.join("root.json"), b"{}").unwrap();
        std::fs::write(base.join("a/d1.json"), b"{}").unwrap();
        std::fs::write(base.join("a/b/d2.json"), b"{}").unwrap();
        std::fs::write(base.join("a/b/c/d3.json"), b"{}").unwrap();

        let canonical_root = fs::canonicalize(&pack).unwrap();
        let mut depth_limited = Vec::new();
        collect_files_bounded(&canonical_root, &base, &base, 0, 2, 64, &mut depth_limited);
        depth_limited.sort();
        assert_eq!(depth_limited, vec!["a/b/d2.json", "a/d1.json", "root.json"]);

        let mut count_limited = Vec::new();
        collect_files_bounded(&canonical_root, &base, &base, 0, 64, 2, &mut count_limited);
        assert_eq!(count_limited.len(), 2);
    }

    #[test]
    fn zip_pack_enforces_entry_count_depth_and_cumulative_decompressed_budget() {
        let many_entries: Vec<(String, &[u8])> = (0..5)
            .map(|i| (format!("data/minecraft/recipes/r{i}.json"), &b"{}"[..]))
            .collect();
        let refs: Vec<(&str, &[u8])> = many_entries.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        let zip_bytes = Arc::<[u8]>::from(build_zip(&refs));
        assert!(ZipPackResources::from_bytes_with_limits(Arc::clone(&zip_bytes), 4, 1024).is_err());

        let deep_path = format!(
            "data/minecraft/{}/leaf.json",
            vec!["d"; MAX_PACK_DIR_DEPTH].join("/")
        );
        let deep_zip = Arc::<[u8]>::from(build_zip(&[(deep_path.as_str(), b"{}")]));
        assert!(ZipPackResources::from_bytes(deep_zip).is_err());

        let bomb_zip = Arc::<[u8]>::from(build_zip(&[
            ("data/minecraft/recipes/a.json", b"aaaaaaaaaa"),
            ("data/minecraft/recipes/b.json", b"bbbbbbbbbb"),
            ("overlay/data/minecraft/recipes/c.json", b"cccccccccc"),
        ]));
        let opened = ZipPackResources::from_bytes_with_limits(bomb_zip, 16, 25).unwrap();
        assert_eq!(
            opened.read("minecraft", "recipes/a.json").as_deref(),
            Some(&b"aaaaaaaaaa"[..])
        );
        assert_eq!(
            opened.read("minecraft", "recipes/b.json").as_deref(),
            Some(&b"bbbbbbbbbb"[..])
        );
        // Only 5 bytes remain in the cumulative pack budget; overlay shares the same counter.
        let overlay = opened.open_overlay("overlay").unwrap();
        assert_eq!(overlay.read("minecraft", "recipes/c.json"), None);
    }
}
