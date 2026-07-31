//! Vanilla-compatible persistent creative hotbar presets.
//!
//! `HotbarManager` owns an uncompressed named-root `hotbar.nbt` file with a
//! `DataVersion` and nine fixed nine-value passthrough lists. Item values are
//! decoded only when a row is requested, after a live registry context exists.

use std::{
    env, fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    process,
    sync::atomic::{AtomicU64, Ordering},
};

use anyhow::{Context, Result};
use bbb_protocol::{
    decode_named_nbt_summary, encode_named_nbt_summary, packets::ItemStackSummary, NbtSummaryEntry,
    NbtSummaryValue, MC_DATA_VERSION,
};
use bbb_world::WorldStore;
use serde::Deserialize;

/// Vanilla `HotbarManager.NUM_HOTBAR_GROUPS`.
pub(crate) const HOTBAR_PRESET_COUNT: usize = 9;
/// Vanilla `Inventory.getSelectionSize()`.
pub(crate) const HOTBAR_PRESET_SLOTS: usize = 9;

const MAX_HOTBAR_NBT_BYTES: usize = 16 * 1024 * 1024;
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub(crate) struct HotbarPresetStore {
    path: PathBuf,
    legacy_json_path: Option<PathBuf>,
    file: Option<HotbarPresetFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HotbarPresetFile {
    hotbars: Vec<Vec<NbtSummaryValue>>,
}

#[derive(Debug, Deserialize)]
struct LegacyHotbarPresetFile {
    #[serde(default)]
    hotbars: Vec<Vec<ItemStackSummary>>,
}

impl Default for HotbarPresetFile {
    fn default() -> Self {
        Self {
            hotbars: vec![empty_preset_nbt(); HOTBAR_PRESET_COUNT],
        }
    }
}

impl HotbarPresetStore {
    /// Construction never touches the filesystem, matching
    /// `HotbarManager.get`'s first-access load.
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        Self {
            legacy_json_path: legacy_json_path_for(&path),
            path,
            file: None,
        }
    }

    pub(crate) fn preset(&mut self, index: usize, world: &WorldStore) -> Vec<ItemStackSummary> {
        let path = self.path.clone();
        self.file_mut(world)
            .hotbars
            .get(index)
            .map(|row| {
                row.iter()
                    .map(|value| match world.persistent_item_stack_from_nbt(value) {
                        Ok(stack) => stack,
                        Err(error) => {
                            tracing::warn!(
                                path = %path.display(),
                                preset = index,
                                %error,
                                "could not parse hotbar item; using an empty stack"
                            );
                            ItemStackSummary::empty()
                        }
                    })
                    .collect()
            })
            .unwrap_or_else(empty_preset)
    }

    pub(crate) fn save_preset(
        &mut self,
        index: usize,
        items: Vec<ItemStackSummary>,
        world: &WorldStore,
    ) -> Result<()> {
        if index >= HOTBAR_PRESET_COUNT {
            anyhow::bail!("hotbar preset index {index} out of range");
        }

        let mut items = items;
        items.resize(HOTBAR_PRESET_SLOTS, ItemStackSummary::empty());
        items.truncate(HOTBAR_PRESET_SLOTS);
        let encoded = items
            .iter()
            .enumerate()
            .map(
                |(slot, stack)| match world.persistent_item_stack_to_nbt(stack) {
                    Ok(value) => value,
                    Err(error) => {
                        tracing::warn!(
                            path = %self.path.display(),
                            preset = index,
                            slot,
                            %error,
                            "could not encode hotbar item; saving an empty stack"
                        );
                        empty_stack_nbt()
                    }
                },
            )
            .collect();

        let path = self.path.clone();
        let file = self.file_mut(world);
        file.hotbars[index] = encoded;
        save_preset_file(&path, file)
    }

    fn file_mut(&mut self, world: &WorldStore) -> &mut HotbarPresetFile {
        if self.file.is_none() {
            self.file = Some(load_preset_file(
                &self.path,
                self.legacy_json_path.as_deref(),
                world,
            ));
        }
        self.file.as_mut().expect("hotbar preset file just loaded")
    }
}

/// A bad root resets the whole grid. A bad/missing row resets only that row;
/// individual item values remain opaque until `preset`, matching
/// `Hotbar.CODEC` plus `Hotbar.load`.
fn load_preset_file(
    path: &Path,
    legacy_json_path: Option<&Path>,
    world: &WorldStore,
) -> HotbarPresetFile {
    match fs::read(path) {
        Ok(raw) => match decode_hotbar_nbt(&raw) {
            Ok(file) => file,
            Err(nbt_error) => match decode_legacy_json(&raw, world) {
                Ok(file) => {
                    tracing::info!(
                        path = %path.display(),
                        "loaded legacy JSON hotbar presets; next save will write Vanilla NBT"
                    );
                    file
                }
                Err(json_error) => {
                    tracing::warn!(
                        path = %path.display(),
                        %nbt_error,
                        %json_error,
                        "failed to parse hotbar preset store; using an empty grid"
                    );
                    HotbarPresetFile::default()
                }
            },
        },
        Err(error) if error.kind() == ErrorKind::NotFound => {
            load_legacy_sibling(legacy_json_path, world)
        }
        Err(error) => {
            tracing::warn!(
                path = %path.display(),
                %error,
                "failed to read hotbar preset store; using an empty grid"
            );
            HotbarPresetFile::default()
        }
    }
}

fn load_legacy_sibling(path: Option<&Path>, world: &WorldStore) -> HotbarPresetFile {
    let Some(path) = path else {
        return HotbarPresetFile::default();
    };
    match fs::read(path) {
        Ok(raw) => match decode_legacy_json(&raw, world) {
            Ok(file) => {
                tracing::info!(
                    path = %path.display(),
                    "loaded legacy JSON hotbar presets; next save will write hotbar.nbt"
                );
                file
            }
            Err(error) => {
                tracing::warn!(
                    path = %path.display(),
                    %error,
                    "failed to parse legacy hotbar presets; using an empty grid"
                );
                HotbarPresetFile::default()
            }
        },
        Err(error) if error.kind() == ErrorKind::NotFound => HotbarPresetFile::default(),
        Err(error) => {
            tracing::warn!(
                path = %path.display(),
                %error,
                "failed to read legacy hotbar presets; using an empty grid"
            );
            HotbarPresetFile::default()
        }
    }
}

fn decode_hotbar_nbt(raw: &[u8]) -> Result<HotbarPresetFile> {
    let (_, root) = decode_named_nbt_summary(raw)
        .context("decode named hotbar NBT")?
        .context("hotbar NBT has no root")?;
    let NbtSummaryValue::Compound(entries) = root else {
        anyhow::bail!("hotbar NBT root is not a compound");
    };

    let hotbars = (0..HOTBAR_PRESET_COUNT)
        .map(|index| {
            let value = entries
                .iter()
                .rev()
                .find(|entry| entry.name == index.to_string())
                .map(|entry| &entry.value);
            match value {
                Some(NbtSummaryValue::List(items)) if items.len() == HOTBAR_PRESET_SLOTS => {
                    items.clone()
                }
                _ => empty_preset_nbt(),
            }
        })
        .collect();
    Ok(HotbarPresetFile { hotbars })
}

fn decode_legacy_json(raw: &[u8], world: &WorldStore) -> Result<HotbarPresetFile> {
    let legacy: LegacyHotbarPresetFile =
        serde_json::from_slice(raw).context("decode legacy JSON hotbar presets")?;
    let mut hotbars = Vec::with_capacity(HOTBAR_PRESET_COUNT);
    for index in 0..HOTBAR_PRESET_COUNT {
        let mut row = legacy.hotbars.get(index).cloned().unwrap_or_default();
        row.resize(HOTBAR_PRESET_SLOTS, ItemStackSummary::empty());
        row.truncate(HOTBAR_PRESET_SLOTS);
        hotbars.push(
            row.iter()
                .enumerate()
                .map(
                    |(slot, stack)| match world.persistent_item_stack_to_nbt(stack) {
                        Ok(value) => value,
                        Err(error) => {
                            tracing::warn!(
                                preset = index,
                                slot,
                                %error,
                                "could not migrate legacy hotbar item; using an empty stack"
                            );
                            empty_stack_nbt()
                        }
                    },
                )
                .collect(),
        );
    }
    Ok(HotbarPresetFile { hotbars })
}

fn save_preset_file(path: &Path, file: &HotbarPresetFile) -> Result<()> {
    let mut entries = Vec::with_capacity(HOTBAR_PRESET_COUNT + 1);
    entries.push(NbtSummaryEntry {
        name: "DataVersion".to_string(),
        value: NbtSummaryValue::Int(MC_DATA_VERSION),
    });
    entries.extend(
        file.hotbars
            .iter()
            .take(HOTBAR_PRESET_COUNT)
            .enumerate()
            .map(|(index, row)| NbtSummaryEntry {
                name: index.to_string(),
                value: NbtSummaryValue::List(row.clone()),
            }),
    );
    let raw = encode_named_nbt_summary(
        "",
        &NbtSummaryValue::Compound(entries),
        MAX_HOTBAR_NBT_BYTES,
    )
    .context("encode hotbar NBT")?;

    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("create hotbar preset directory {}", parent.display()))?;
    }
    atomic_write(path, &raw)
}

fn atomic_write(path: &Path, raw: &[u8]) -> Result<()> {
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("hotbar.nbt");
    let temporary = path.with_file_name(format!(".{file_name}.tmp-{}-{sequence}", process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .with_context(|| format!("create temporary hotbar file {}", temporary.display()))?;
    let prepare_result = (|| -> Result<()> {
        file.write_all(raw)
            .with_context(|| format!("write temporary hotbar file {}", temporary.display()))?;
        file.sync_all()
            .with_context(|| format!("sync temporary hotbar file {}", temporary.display()))
    })();
    drop(file);
    if let Err(error) = prepare_result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    let replace_result = fs::rename(&temporary, path).with_context(|| {
        format!(
            "replace hotbar preset store {} from {}",
            path.display(),
            temporary.display()
        )
    });
    if replace_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    replace_result
}

fn empty_stack_nbt() -> NbtSummaryValue {
    NbtSummaryValue::Compound(Vec::new())
}

fn empty_preset_nbt() -> Vec<NbtSummaryValue> {
    vec![empty_stack_nbt(); HOTBAR_PRESET_SLOTS]
}

fn empty_preset() -> Vec<ItemStackSummary> {
    vec![ItemStackSummary::empty(); HOTBAR_PRESET_SLOTS]
}

fn legacy_json_path_for(path: &Path) -> Option<PathBuf> {
    let file_name = path.file_name()?.to_str()?;
    match file_name {
        "hotbar.nbt" => Some(path.with_file_name("hotbar-presets.json")),
        "bbb-hotbar.nbt" => Some(path.with_file_name("bbb-hotbar-presets.json")),
        _ => None,
    }
}

pub(crate) fn default_hotbar_preset_store_path() -> PathBuf {
    if let Some(path) = env::var_os("BBB_HOTBAR_PRESET_STORE") {
        return PathBuf::from(path);
    }

    if cfg!(target_os = "windows") {
        if let Some(app_data) = env::var_os("APPDATA") {
            return PathBuf::from(app_data)
                .join("bbb-native")
                .join("hotbar.nbt");
        }
    }

    if cfg!(target_os = "macos") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("bbb-native")
                .join("hotbar.nbt");
        }
    }

    if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(config_home)
            .join("bbb-native")
            .join("hotbar.nbt");
    }

    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("bbb-native")
            .join("hotbar.nbt");
    }

    PathBuf::from("bbb-hotbar.nbt")
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_store_path(name: &str, extension: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "bbb-hotbar-presets-{name}-{}-{nanos}.{extension}",
            process::id()
        ))
    }

    fn stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            ..ItemStackSummary::empty()
        }
    }

    #[test]
    fn missing_file_yields_empty_presets() {
        let world = WorldStore::new();
        let mut store = HotbarPresetStore::new(unique_store_path("missing", "nbt"));
        for index in 0..HOTBAR_PRESET_COUNT {
            assert_eq!(store.preset(index, &world), empty_preset());
        }
    }

    #[test]
    fn save_preset_writes_vanilla_named_nbt_and_roundtrips_components() {
        let world = WorldStore::new();
        let path = unique_store_path("roundtrip", "nbt");
        let mut items = empty_preset();
        items[0] = stack(1, 64);
        items[8] = ItemStackSummary {
            item_id: Some(848),
            count: 1,
            component_patch: bbb_protocol::packets::DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID],
                custom_name: Some("Preset".to_string()),
                custom_name_styled: Some(vec![bbb_protocol::StyledTextRun {
                    text: "Preset".to_string(),
                    style: bbb_protocol::ComponentStyle::default(),
                }]),
                custom_name_nbt: Some(NbtSummaryValue::String("Preset".to_string())),
                ..Default::default()
            },
        };

        let mut store = HotbarPresetStore::new(&path);
        store.save_preset(3, items.clone(), &world).unwrap();
        store.save_preset(4, vec![stack(1, 2)], &world).unwrap();

        let raw = fs::read(&path).unwrap();
        let (root_name, root) = decode_named_nbt_summary(&raw).unwrap().unwrap();
        assert_eq!(root_name, "");
        let NbtSummaryValue::Compound(entries) = root else {
            panic!("hotbar root is not a compound");
        };
        assert!(entries.iter().any(|entry| {
            entry.name == "DataVersion" && entry.value == NbtSummaryValue::Int(MC_DATA_VERSION)
        }));

        let mut reloaded = HotbarPresetStore::new(&path);
        assert_eq!(reloaded.preset(3, &world), items);
        assert_eq!(reloaded.preset(4, &world)[0], stack(1, 2));
        assert_eq!(reloaded.preset(0, &world), empty_preset());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn invalid_root_falls_back_to_empty_grid_without_legacy_resurrection() {
        let world = WorldStore::new();
        let path = unique_store_path("corrupt", "nbt");
        fs::write(&path, b"not nbt or json").unwrap();
        let legacy = path.with_file_name("hotbar-presets.json");
        fs::write(
            &legacy,
            serde_json::to_vec(&serde_json::json!({
                "hotbars": [[stack(1, 1)]]
            }))
            .unwrap(),
        )
        .unwrap();

        let mut store = HotbarPresetStore {
            path: path.clone(),
            legacy_json_path: Some(legacy.clone()),
            file: None,
        };
        assert_eq!(store.preset(0, &world), empty_preset());
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(legacy);
    }

    #[test]
    fn invalid_row_is_empty_and_invalid_cell_is_locally_empty() {
        let world = WorldStore::new();
        let path = unique_store_path("recovery", "nbt");
        let mut entries = vec![NbtSummaryEntry {
            name: "DataVersion".to_string(),
            value: NbtSummaryValue::Int(MC_DATA_VERSION),
        }];
        entries.push(NbtSummaryEntry {
            name: "0".to_string(),
            value: NbtSummaryValue::List(vec![empty_stack_nbt()]),
        });
        let mut row = empty_preset_nbt();
        row[4] = NbtSummaryValue::Int(7);
        entries.push(NbtSummaryEntry {
            name: "1".to_string(),
            value: NbtSummaryValue::List(row),
        });
        let raw = encode_named_nbt_summary(
            "",
            &NbtSummaryValue::Compound(entries),
            MAX_HOTBAR_NBT_BYTES,
        )
        .unwrap();
        fs::write(&path, raw).unwrap();

        let mut store = HotbarPresetStore::new(&path);
        assert_eq!(store.preset(0, &world), empty_preset());
        assert_eq!(store.preset(1, &world), empty_preset());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn missing_nbt_migrates_legacy_json_without_deleting_it() {
        let world = WorldStore::new();
        let directory = unique_store_path("migration", "dir");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("hotbar.nbt");
        let legacy = directory.join("hotbar-presets.json");
        let legacy_items = vec![vec![stack(1, 2)]];
        fs::write(
            &legacy,
            serde_json::to_vec(&serde_json::json!({ "hotbars": legacy_items })).unwrap(),
        )
        .unwrap();

        let mut store = HotbarPresetStore::new(&path);
        let loaded = store.preset(0, &world);
        assert_eq!(loaded[0], stack(1, 2));
        assert_eq!(loaded.len(), HOTBAR_PRESET_SLOTS);
        assert!(!path.exists());

        store.save_preset(0, loaded, &world).unwrap();
        assert!(path.exists());
        assert!(legacy.exists());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn save_preset_rejects_out_of_range_index() {
        let world = WorldStore::new();
        let mut store = HotbarPresetStore::new(unique_store_path("range", "nbt"));
        assert!(store
            .save_preset(HOTBAR_PRESET_COUNT, vec![stack(1, 1)], &world)
            .is_err());
    }

    #[test]
    fn short_preset_rows_are_padded_to_nine_slots() {
        let world = WorldStore::new();
        let path = unique_store_path("padding", "nbt");
        let mut store = HotbarPresetStore::new(&path);
        store.save_preset(0, vec![stack(1, 2)], &world).unwrap();

        let preset = store.preset(0, &world);
        assert_eq!(preset.len(), HOTBAR_PRESET_SLOTS);
        assert_eq!(preset[0], stack(1, 2));
        assert_eq!(preset[1], ItemStackSummary::empty());
        let _ = fs::remove_file(path);
    }
}
