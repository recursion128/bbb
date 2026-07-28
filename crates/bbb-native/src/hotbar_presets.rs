//! Persistent creative hotbar presets.
//!
//! Vanilla `HotbarManager` keeps nine nine-slot hotbar groups in
//! `hotbar.nbt`, loading the file lazily on first access and rewriting it
//! after every save activation. BBB persists the same nine-by-nine grid as
//! JSON-serialized [`ItemStackSummary`] rows instead of vanilla's
//! registry-coupled `ItemStack` NBT codec; the interchange-format gap is
//! recorded in `docs/unsupported-features.md`.

use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use bbb_protocol::packets::ItemStackSummary;
use serde::{Deserialize, Serialize};

/// Vanilla `HotbarManager.NUM_HOTBAR_GROUPS`.
pub(crate) const HOTBAR_PRESET_COUNT: usize = 9;
/// Vanilla `Inventory.getSelectionSize()`.
pub(crate) const HOTBAR_PRESET_SLOTS: usize = 9;

#[derive(Debug, Clone)]
pub(crate) struct HotbarPresetStore {
    path: PathBuf,
    file: Option<HotbarPresetFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct HotbarPresetFile {
    #[serde(default)]
    hotbars: Vec<Vec<ItemStackSummary>>,
}

impl Default for HotbarPresetFile {
    fn default() -> Self {
        Self {
            hotbars: vec![empty_preset(); HOTBAR_PRESET_COUNT],
        }
    }
}

impl HotbarPresetStore {
    /// Vanilla `HotbarManager` defers reading the file until the first
    /// `get(...)`, so construction never touches the filesystem.
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            file: None,
        }
    }

    pub(crate) fn preset(&mut self, index: usize) -> Vec<ItemStackSummary> {
        self.file_mut()
            .hotbars
            .get(index)
            .cloned()
            .unwrap_or_else(empty_preset)
    }

    pub(crate) fn save_preset(&mut self, index: usize, items: Vec<ItemStackSummary>) -> Result<()> {
        if index >= HOTBAR_PRESET_COUNT {
            anyhow::bail!("hotbar preset index {index} out of range");
        }
        let mut preset = items;
        preset.resize(HOTBAR_PRESET_SLOTS, ItemStackSummary::empty());
        preset.truncate(HOTBAR_PRESET_SLOTS);
        let file = self.file_mut();
        if file.hotbars.len() < HOTBAR_PRESET_COUNT {
            file.hotbars.resize(HOTBAR_PRESET_COUNT, empty_preset());
        }
        file.hotbars[index] = preset;
        let raw = serde_json::to_vec_pretty(file).context("serialize hotbar preset store")?;
        if let Some(parent) = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).with_context(|| {
                format!("create hotbar preset store directory {}", parent.display())
            })?;
        }
        fs::write(&self.path, raw)
            .with_context(|| format!("write hotbar preset store {}", self.path.display()))
    }

    fn file_mut(&mut self) -> &mut HotbarPresetFile {
        if self.file.is_none() {
            self.file = Some(load_preset_file(&self.path));
        }
        self.file.as_mut().expect("hotbar preset file just loaded")
    }
}

/// Vanilla `HotbarManager.load` logs a parse failure and keeps the empty
/// in-memory grid rather than propagating the error.
fn load_preset_file(path: &Path) -> HotbarPresetFile {
    match fs::read_to_string(path) {
        Ok(raw) => match serde_json::from_str(&raw) {
            Ok(file) => file,
            Err(err) => {
                tracing::warn!(
                    "failed to parse hotbar preset store {}: {err}",
                    path.display()
                );
                HotbarPresetFile::default()
            }
        },
        Err(err) if err.kind() == ErrorKind::NotFound => HotbarPresetFile::default(),
        Err(err) => {
            tracing::warn!(
                "failed to read hotbar preset store {}: {err}",
                path.display()
            );
            HotbarPresetFile::default()
        }
    }
}

fn empty_preset() -> Vec<ItemStackSummary> {
    vec![ItemStackSummary::empty(); HOTBAR_PRESET_SLOTS]
}

pub(crate) fn default_hotbar_preset_store_path() -> PathBuf {
    if let Some(path) = env::var_os("BBB_HOTBAR_PRESET_STORE") {
        return PathBuf::from(path);
    }

    if cfg!(target_os = "windows") {
        if let Some(app_data) = env::var_os("APPDATA") {
            return PathBuf::from(app_data)
                .join("bbb-native")
                .join("hotbar-presets.json");
        }
    }

    if cfg!(target_os = "macos") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("bbb-native")
                .join("hotbar-presets.json");
        }
    }

    if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(config_home)
            .join("bbb-native")
            .join("hotbar-presets.json");
    }

    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("bbb-native")
            .join("hotbar-presets.json");
    }

    PathBuf::from("bbb-hotbar-presets.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        process,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn unique_store_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "bbb-hotbar-presets-{name}-{}-{nanos}.json",
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
        let mut store = HotbarPresetStore::new(unique_store_path("missing"));
        for index in 0..HOTBAR_PRESET_COUNT {
            assert_eq!(store.preset(index), vec![ItemStackSummary::empty(); 9]);
        }
    }

    #[test]
    fn save_preset_roundtrips_through_disk() {
        let path = unique_store_path("roundtrip");
        let mut items = vec![ItemStackSummary::empty(); HOTBAR_PRESET_SLOTS];
        items[0] = stack(1, 64);
        items[8] = stack(848, 1);

        let mut store = HotbarPresetStore::new(&path);
        store.save_preset(3, items.clone()).unwrap();

        let mut reloaded = HotbarPresetStore::new(&path);
        assert_eq!(reloaded.preset(3), items);
        assert_eq!(reloaded.preset(0), vec![ItemStackSummary::empty(); 9]);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn corrupt_file_falls_back_to_empty_grid() {
        let path = unique_store_path("corrupt");
        fs::write(&path, b"{not json").unwrap();

        let mut store = HotbarPresetStore::new(&path);
        assert_eq!(store.preset(0), vec![ItemStackSummary::empty(); 9]);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn save_preset_rejects_out_of_range_index() {
        let mut store = HotbarPresetStore::new(unique_store_path("range"));
        assert!(store
            .save_preset(HOTBAR_PRESET_COUNT, vec![stack(1, 1)])
            .is_err());
    }

    #[test]
    fn short_preset_rows_are_padded_to_nine_slots() {
        let path = unique_store_path("padding");
        let mut store = HotbarPresetStore::new(&path);
        store.save_preset(0, vec![stack(1, 2)]).unwrap();

        let preset = store.preset(0);
        assert_eq!(preset.len(), HOTBAR_PRESET_SLOTS);
        assert_eq!(preset[0], stack(1, 2));
        assert_eq!(preset[1], ItemStackSummary::empty());
        let _ = fs::remove_file(path);
    }
}
