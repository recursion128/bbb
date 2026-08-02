use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::resources::{PackResourceStack, ResourceLocation};

pub const DEFAULT_LANGUAGE_CODE: &str = "en_us";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageCatalog {
    pub translations: BTreeMap<String, String>,
    #[serde(default)]
    bidirectional: bool,
}

impl LanguageCatalog {
    pub fn load_resource_stack(
        stack: &PackResourceStack,
        language_codes: &[impl AsRef<str>],
    ) -> Result<Self> {
        let mut catalog = Self::default();
        let namespaces = stack.namespaces()?;
        for language_code in language_codes {
            let language_code = language_code.as_ref();
            let language_path = language_path(language_code)?;
            for namespace in &namespaces {
                let location = ResourceLocation::new(namespace.clone(), language_path.clone())?;
                // Vanilla skips the namespace/language entry if one resource in that stack fails.
                if catalog
                    .append_resource_stack(stack, &stack.get_resource_stack(&location))
                    .is_err()
                {
                    continue;
                }
            }
        }
        Ok(catalog)
    }

    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self> {
        let mut catalog = Self::default();
        catalog.merge_json_bytes(bytes)?;
        Ok(catalog)
    }

    pub fn apply_deprecated_json_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        let info: DeprecatedTranslationsInfo = serde_json::from_slice(bytes)?;
        self.apply_deprecated_info(info)
    }

    pub(crate) fn apply_deprecated_info(&mut self, info: DeprecatedTranslationsInfo) -> Result<()> {
        for key in info.removed {
            self.translations.remove(&key);
        }
        for (from_key, to_key) in info.renamed {
            match self.translations.remove(&from_key) {
                Some(value) => {
                    self.translations.insert(to_key, value);
                }
                None => {
                    self.translations.remove(&to_key);
                }
            }
        }
        Ok(())
    }

    fn append_resource_stack(
        &mut self,
        stack: &PackResourceStack,
        resources: &[crate::resources::PackResource],
    ) -> Result<()> {
        for resource in resources {
            let Ok(raw): Result<BTreeMap<String, serde_json::Value>> =
                stack.read_json_resource(resource, "language file")
            else {
                continue;
            };
            self.merge_raw(raw)
                .with_context(|| format!("parse language file {}", resource.path.display()))?;
        }
        Ok(())
    }

    fn merge_json_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        let raw: BTreeMap<String, serde_json::Value> = serde_json::from_slice(bytes)?;
        self.merge_raw(raw)
    }

    fn merge_raw(&mut self, raw: BTreeMap<String, serde_json::Value>) -> Result<()> {
        let unsupported_format = unsupported_format_regex()?;
        for (key, value) in raw {
            let value = translation_value_as_string(&key, value)?;
            self.translations.insert(
                key,
                normalize_translation_format(&value, &unsupported_format),
            );
        }
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.translations.get(key).map(String::as_str)
    }

    pub fn get_or_key<'a>(&'a self, key: &'a str) -> &'a str {
        self.get(key).unwrap_or(key)
    }

    pub fn has(&self, key: &str) -> bool {
        self.translations.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.translations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.translations.is_empty()
    }

    pub fn is_bidirectional(&self) -> bool {
        self.bidirectional
    }

    pub(crate) fn set_bidirectional(&mut self, bidirectional: bool) {
        self.bidirectional = bidirectional;
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeprecatedTranslationsInfo {
    removed: Vec<String>,
    renamed: BTreeMap<String, String>,
}

fn language_path(language_code: &str) -> Result<String> {
    ResourceLocation::new("minecraft", format!("lang/{language_code}.json"))
        .map(|location| location.path().to_string())
}

fn translation_value_as_string(key: &str, value: serde_json::Value) -> Result<String> {
    match value {
        serde_json::Value::String(value) => Ok(value),
        serde_json::Value::Bool(value) => Ok(value.to_string()),
        serde_json::Value::Number(value) => Ok(value.to_string()),
        _ => bail!("language entry {key:?} is not a string, number, or boolean"),
    }
}

fn unsupported_format_regex() -> Result<Regex> {
    Regex::new(r"%(\d+\$)?[\d.]*[df]").context("compile language format normalizer")
}

fn normalize_translation_format(value: &str, unsupported_format: &Regex) -> String {
    unsupported_format
        .replace_all(value, |captures: &regex::Captures<'_>| {
            format!(
                "%{}s",
                captures.get(1).map_or("", |capture| capture.as_str())
            )
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PackRoots, MC_VERSION};
    use sha1::{Digest, Sha1};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn language_catalog_parses_json_and_normalizes_numeric_formats() {
        let catalog = LanguageCatalog::from_json_bytes(
            br#"{
              "chat.type.text": "<%s> %s",
              "debug.value": "%1$d / %2$.2f",
              "plain.percent": "100%% ready",
              "primitive.number": 42,
              "primitive.boolean": true
            }"#,
        )
        .unwrap();

        assert_eq!(catalog.get("chat.type.text"), Some("<%s> %s"));
        assert_eq!(catalog.get("debug.value"), Some("%1$s / %2$s"));
        assert_eq!(catalog.get("plain.percent"), Some("100%% ready"));
        assert_eq!(catalog.get("primitive.number"), Some("42"));
        assert_eq!(catalog.get("primitive.boolean"), Some("true"));
        assert_eq!(catalog.get_or_key("missing.key"), "missing.key");
    }

    #[test]
    fn language_catalog_applies_deprecated_translation_info() {
        let mut catalog = LanguageCatalog::from_json_bytes(
            br#"{
              "old.key": "Old Value",
              "removed.key": "Removed Value",
              "new.key": "Existing Value",
              "missing.target": "Stale Value"
            }"#,
        )
        .unwrap();

        catalog
            .apply_deprecated_json_bytes(
                br#"{
                  "removed": ["removed.key"],
                  "renamed": {
                    "old.key": "new.key",
                    "missing.key": "missing.target"
                  }
                }"#,
            )
            .unwrap();

        assert!(!catalog.has("old.key"));
        assert!(!catalog.has("removed.key"));
        assert_eq!(catalog.get("new.key"), Some("Old Value"));
        assert!(!catalog.has("missing.target"));
    }

    #[test]
    fn language_catalog_resource_stack_merges_namespaces_and_pack_precedence() {
        let root = unique_temp_dir("language-stack");
        let base = root.join("sources").join(MC_VERSION);
        let overlay = root.join("overlay");
        write_json(
            &base
                .join("assets")
                .join("minecraft")
                .join("lang")
                .join("en_us.json"),
            r#"{
              "menu.singleplayer": "Singleplayer",
              "menu.multiplayer": "Multiplayer"
            }"#,
        );
        write_json(
            &overlay
                .join("assets")
                .join("minecraft")
                .join("lang")
                .join("en_us.json"),
            r#"{
              "menu.singleplayer": "Solo",
              "menu.options": "Options"
            }"#,
        );
        write_json(
            &overlay
                .join("assets")
                .join("custom")
                .join("lang")
                .join("en_us.json"),
            r#"{
              "custom.title": "Custom Pack"
            }"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([overlay]);
        let catalog =
            LanguageCatalog::load_resource_stack(&roots.resource_stack(), &[DEFAULT_LANGUAGE_CODE])
                .unwrap();

        assert_eq!(catalog.get("menu.singleplayer"), Some("Solo"));
        assert_eq!(catalog.get("menu.multiplayer"), Some("Multiplayer"));
        assert_eq!(catalog.get("menu.options"), Some("Options"));
        assert_eq!(catalog.get("custom.title"), Some("Custom Pack"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn language_catalog_skips_bad_pack_resource_and_keeps_other_namespaces() {
        let root = unique_temp_dir("language-bad-resource");
        let base = root.join("sources").join(MC_VERSION);
        let overlay = root.join("overlay");
        write_json(
            &base
                .join("assets")
                .join("minecraft")
                .join("lang")
                .join("en_us.json"),
            r#"{
              "menu.play": "Play"
            }"#,
        );
        write_json(
            &overlay
                .join("assets")
                .join("minecraft")
                .join("lang")
                .join("en_us.json"),
            "{",
        );
        write_json(
            &overlay
                .join("assets")
                .join("custom")
                .join("lang")
                .join("en_us.json"),
            r#"{
              "custom.title": "Custom Pack"
            }"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([overlay]);
        let catalog =
            LanguageCatalog::load_resource_stack(&roots.resource_stack(), &[DEFAULT_LANGUAGE_CODE])
                .unwrap();

        assert_eq!(catalog.get("menu.play"), Some("Play"));
        assert_eq!(catalog.get("custom.title"), Some("Custom Pack"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pack_roots_language_catalog_loads_default_then_selected_language() {
        let root = unique_temp_dir("language-pack-roots");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_json(
            &assets_dir.join("lang").join("en_us.json"),
            r#"{
              "options.language": "Language",
              "menu.play": "Play"
            }"#,
        );
        write_json(
            &assets_dir.join("lang").join("pirate.json"),
            r#"{
              "menu.play": "Sail"
            }"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let catalog = roots.load_language_catalog("pirate").unwrap();

        assert_eq!(catalog.get("options.language"), Some("Language"));
        assert_eq!(catalog.get("menu.play"), Some("Sail"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pack_roots_client_language_catalog_uses_pack_metadata_for_selected_language() {
        let root = unique_temp_dir("language-metadata-aware");
        let sources_dir = root.join("sources").join(MC_VERSION);
        let assets_dir = sources_dir.join("assets").join("minecraft");
        write_json(
            &assets_dir.join("lang").join("en_us.json"),
            r#"{
              "menu.play": "Play"
            }"#,
        );
        write_json(
            &assets_dir.join("lang").join("pirate.json"),
            r#"{
              "menu.play": "Sail"
            }"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let fallback_catalog = roots.load_client_language_catalog("pirate").unwrap();
        assert_eq!(fallback_catalog.get("menu.play"), Some("Play"));
        assert!(!fallback_catalog.is_bidirectional());

        write_json(
            &sources_dir.join("pack.mcmeta"),
            r#"{
              "language": {
                "pirate": {
                  "region": "Seven Seas",
                  "name": "Pirate Speak",
                  "bidirectional": true
                }
              }
            }"#,
        );

        let selected_catalog = roots.load_client_language_catalog("pirate").unwrap();
        assert_eq!(selected_catalog.get("menu.play"), Some("Sail"));
        assert!(selected_catalog.is_bidirectional());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn indexed_selected_locale_honors_pack_override_and_filter() {
        let root = unique_temp_dir("indexed-language-precedence");
        let assets = root.join("sources/26.1/assets/minecraft");
        write_json(
            &assets.join("lang/en_us.json"),
            r#"{"options.language":"Language...","menu.play":"Play"}"#,
        );
        write_indexed_language_fixture(&root);

        let roots = PackRoots::from_root(&root).unwrap();
        let indexed = roots.load_client_language_catalog("zh_cn").unwrap();
        assert_eq!(indexed.get("options.language"), Some("语言…"));
        assert_eq!(indexed.get("menu.play"), Some("开始游戏"));

        let override_pack = root.join("packs/override");
        write_json(
            &override_pack.join("assets/minecraft/lang/zh_cn.json"),
            r#"{"menu.play":"覆盖"}"#,
        );
        let overridden = roots
            .clone()
            .with_resource_pack_dirs([override_pack])
            .load_client_language_catalog("zh_cn")
            .unwrap();
        assert_eq!(overridden.get("options.language"), Some("语言…"));
        assert_eq!(overridden.get("menu.play"), Some("覆盖"));

        let filter_pack = root.join("packs/filter");
        write_json(
            &filter_pack.join("pack.mcmeta"),
            r#"{"filter":{"block":[{"namespace":"minecraft","path":"lang/zh_cn\\.json"}]}}"#,
        );
        let filtered = roots
            .with_resource_pack_dirs([filter_pack])
            .load_client_language_catalog("zh_cn")
            .unwrap();
        assert_eq!(filtered.get("options.language"), Some("Language..."));
        assert_eq!(filtered.get("menu.play"), Some("Play"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pack_roots_language_catalog_applies_default_deprecated_translations() {
        let root = unique_temp_dir("language-deprecated");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_json(
            &assets_dir.join("lang").join("en_us.json"),
            r#"{
              "old.key": "Old Value",
              "removed.key": "Removed Value",
              "new.key": "Existing Value"
            }"#,
        );
        write_json(
            &assets_dir.join("lang").join("deprecated.json"),
            r#"{
              "removed": ["removed.key"],
              "renamed": {
                "old.key": "new.key"
              }
            }"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let catalog = roots.load_language_catalog(DEFAULT_LANGUAGE_CODE).unwrap();

        assert!(!catalog.has("old.key"));
        assert!(!catalog.has("removed.key"));
        assert_eq!(catalog.get("new.key"), Some("Old Value"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_language_catalog() {
        let roots = PackRoots::discover().unwrap();
        let catalog = roots.load_language_catalog(DEFAULT_LANGUAGE_CODE).unwrap();

        assert!(catalog.len() > 1_000);
        assert_eq!(catalog.get("menu.singleplayer"), Some("Singleplayer"));
        assert_eq!(
            catalog.get_or_key("missing.translation.key"),
            "missing.translation.key"
        );
    }

    #[test]
    #[ignore = "requires the local Vanilla 26.1 asset object store"]
    fn loads_real_indexed_zh_cn_through_the_default_resource_pack() {
        let roots = PackRoots::discover().unwrap();
        let stack = roots.resource_stack();
        let observation = stack
            .vanilla_asset_index_observation()
            .expect("local Vanilla index must be mounted");
        assert_eq!(observation.index_id, "30");
        assert_eq!(observation.object_count, 4_749);

        let catalog = roots.load_client_language_catalog("zh_cn").unwrap();
        assert_eq!(catalog.get("options.language"), Some("语言…"));
        assert_eq!(catalog.get("menu.multiplayer"), Some("多人游戏"));
        assert_eq!(catalog.get("block.minecraft.stone"), Some("石头"));
        assert!(!catalog.is_bidirectional());
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-pack-{label}-{nanos}"))
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn write_indexed_language_fixture(root: &Path) {
        let mut objects = serde_json::Map::new();
        for (name, contents) in [
            (
                "minecraft/lang/zh_cn.json",
                r#"{"options.language":"语言…","menu.play":"开始游戏"}"#,
            ),
            (
                "pack.mcmeta",
                r#"{"language":{"zh_cn":{"region":"CN","name":"简体中文"}}}"#,
            ),
        ] {
            let bytes = contents.as_bytes();
            let hash = format!("{:x}", Sha1::digest(bytes));
            objects.insert(
                name.to_string(),
                serde_json::json!({"hash": hash, "size": bytes.len()}),
            );
            let path = root
                .join("downloads/26.1/assets/objects")
                .join(&hash[..2])
                .join(&hash);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
        let index = serde_json::to_string(&serde_json::json!({"objects": objects})).unwrap();
        let index_hash = format!("{:x}", Sha1::digest(index.as_bytes()));
        write_json(&root.join("downloads/26.1/assets/indexes/30.json"), &index);
        write_json(
            &root.join("downloads/26.1/26.1.json"),
            &format!(
                r#"{{"assetIndex":{{"id":"30","sha1":"{index_hash}","size":{}}}}}"#,
                index.len()
            ),
        );
    }
}
