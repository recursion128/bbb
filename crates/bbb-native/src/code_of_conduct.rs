use std::{
    collections::BTreeMap,
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use bbb_net::ConnectionOptions;
use bbb_world::WorldStore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub(crate) struct CodeOfConductAcceptance {
    path: PathBuf,
    store: CodeOfConductAcceptStore,
    connected_server: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct CodeOfConductAcceptStore {
    #[serde(default)]
    accepted_hashes: BTreeMap<String, i32>,
}

impl CodeOfConductAcceptance {
    pub(crate) fn load(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let store = CodeOfConductAcceptStore::load(&path)?;
        Ok(Self {
            path,
            store,
            connected_server: None,
        })
    }

    pub(crate) fn empty(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            store: CodeOfConductAcceptStore::default(),
            connected_server: None,
        }
    }

    pub(crate) fn accepted_hash_for_options(&self, options: &ConnectionOptions) -> Option<i32> {
        self.store.accepted_hash(&server_key(options))
    }

    pub(crate) fn set_connected_server(&mut self, options: &ConnectionOptions) {
        self.connected_server = Some(server_key(options));
    }

    pub(crate) fn persist_current_world_acceptance(&mut self, world: &WorldStore) -> Result<bool> {
        let (Some(server), Some(code_of_conduct)) = (
            self.connected_server.as_deref(),
            world.last_code_of_conduct(),
        ) else {
            return Ok(false);
        };

        self.store.accept(server, code_of_conduct.text_hash);
        self.store.save(&self.path)?;
        Ok(true)
    }

    pub(crate) fn clear_connected_server_acceptance(&mut self) -> Result<bool> {
        let Some(server) = self.connected_server.as_deref() else {
            return Ok(false);
        };

        let removed = self.store.clear(server);
        self.store.save(&self.path)?;
        Ok(removed)
    }

    pub(crate) fn current_world_acceptance_matches(&self, world: &WorldStore) -> bool {
        let (Some(server), Some(code_of_conduct)) = (
            self.connected_server.as_deref(),
            world.last_code_of_conduct(),
        ) else {
            return false;
        };

        self.store.accepted_hash(server) == Some(code_of_conduct.text_hash)
    }
}

impl CodeOfConductAcceptStore {
    fn load(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(raw) => serde_json::from_str(&raw).with_context(|| {
                format!("parse code-of-conduct acceptance store {}", path.display())
            }),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err).with_context(|| {
                format!("read code-of-conduct acceptance store {}", path.display())
            }),
        }
    }

    fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "create code-of-conduct acceptance store directory {}",
                    parent.display()
                )
            })?;
        }
        let raw = serde_json::to_vec_pretty(self)
            .context("serialize code-of-conduct acceptance store")?;
        fs::write(path, raw)
            .with_context(|| format!("write code-of-conduct acceptance store {}", path.display()))
    }

    fn accepted_hash(&self, server: &str) -> Option<i32> {
        self.accepted_hashes.get(server).copied()
    }

    fn accept(&mut self, server: &str, hash: i32) {
        self.accepted_hashes.insert(server.to_string(), hash);
    }

    fn clear(&mut self, server: &str) -> bool {
        self.accepted_hashes.remove(server).is_some()
    }
}

pub(crate) fn default_code_of_conduct_store_path() -> PathBuf {
    if let Some(path) = env::var_os("BBB_CODE_OF_CONDUCT_STORE") {
        return PathBuf::from(path);
    }

    if cfg!(target_os = "windows") {
        if let Some(app_data) = env::var_os("APPDATA") {
            return PathBuf::from(app_data)
                .join("bbb-native")
                .join("code-of-conduct.json");
        }
    }

    if cfg!(target_os = "macos") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("bbb-native")
                .join("code-of-conduct.json");
        }
    }

    if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(config_home)
            .join("bbb-native")
            .join("code-of-conduct.json");
    }

    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("bbb-native")
            .join("code-of-conduct.json");
    }

    PathBuf::from("bbb-code-of-conduct.json")
}

fn server_key(options: &ConnectionOptions) -> String {
    format!("{}:{}", options.host, options.port)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_world::code_of_conduct_text_hash;
    use std::{
        process,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn store_loads_missing_file_as_empty() {
        let path = unique_store_path("missing");
        let store = CodeOfConductAcceptStore::load(&path).unwrap();

        assert_eq!(store, CodeOfConductAcceptStore::default());
    }

    #[test]
    fn store_roundtrips_hashes_without_raw_text() {
        let path = unique_store_path("roundtrip");
        let text = "Keep the server friendly.";
        let hash = code_of_conduct_text_hash(text);
        let mut store = CodeOfConductAcceptStore::default();
        store.accept("example.org:25565", hash);

        store.save(&path).unwrap();
        let raw = fs::read_to_string(&path).unwrap();
        assert!(!raw.contains(text));

        let loaded = CodeOfConductAcceptStore::load(&path).unwrap();
        assert_eq!(loaded.accepted_hash("example.org:25565"), Some(hash));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn acceptance_persists_current_world_code_of_conduct_hash() {
        let path = unique_store_path("acceptance");
        let mut acceptance = CodeOfConductAcceptance::load(&path).unwrap();
        let mut options = ConnectionOptions::offline("example.org:25565", "bbb").unwrap();
        options.accepted_code_of_conduct_hash = None;
        let text = "Keep the server friendly.";
        let mut world = WorldStore::new();
        world.apply_code_of_conduct(text.to_string());

        acceptance.set_connected_server(&options);
        assert!(acceptance.persist_current_world_acceptance(&world).unwrap());

        let loaded = CodeOfConductAcceptance::load(&path).unwrap();
        assert_eq!(
            loaded.accepted_hash_for_options(&options),
            Some(code_of_conduct_text_hash(text))
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn acceptance_clears_connected_server_hash() {
        let path = unique_store_path("clear");
        let mut acceptance = CodeOfConductAcceptance::load(&path).unwrap();
        let options = ConnectionOptions::offline("example.org:25565", "bbb").unwrap();
        let text = "Keep the server friendly.";
        let mut world = WorldStore::new();
        world.apply_code_of_conduct(text.to_string());

        acceptance.set_connected_server(&options);
        assert!(acceptance.persist_current_world_acceptance(&world).unwrap());
        assert!(acceptance.clear_connected_server_acceptance().unwrap());

        let loaded = CodeOfConductAcceptance::load(&path).unwrap();
        assert_eq!(loaded.accepted_hash_for_options(&options), None);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn acceptance_matches_current_world_code_of_conduct_hash() {
        let path = unique_store_path("matches");
        let mut acceptance = CodeOfConductAcceptance::load(&path).unwrap();
        let options = ConnectionOptions::offline("example.org:25565", "bbb").unwrap();
        let text = "Keep the server friendly.";
        let mut world = WorldStore::new();
        world.apply_code_of_conduct(text.to_string());

        acceptance.set_connected_server(&options);
        assert!(!acceptance.current_world_acceptance_matches(&world));
        assert!(acceptance.persist_current_world_acceptance(&world).unwrap());
        assert!(acceptance.current_world_acceptance_matches(&world));

        world.apply_code_of_conduct("Different rules.".to_string());
        assert!(!acceptance.current_world_acceptance_matches(&world));
        let _ = fs::remove_file(path);
    }

    fn unique_store_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "bbb-code-of-conduct-{name}-{}-{nanos}.json",
            process::id()
        ))
    }
}
