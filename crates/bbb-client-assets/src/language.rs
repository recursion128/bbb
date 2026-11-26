use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context, Result};
use bbb_protocol::JavaString;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    resources::{PackResourceStack, ResourceLocation},
    ResourceJsonValue,
};

pub const DEFAULT_LANGUAGE_CODE: &str = "en_us";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct LanguageCatalog {
    #[serde(serialize_with = "serialize_translations")]
    translations: BTreeMap<JavaString, JavaString>,
    #[serde(skip)]
    display_cache: BTreeMap<String, String>,
    #[serde(default)]
    bidirectional: bool,
    /// Languages declared by the selected pack stack's `language` metadata.
    /// A JSON file alone does not make a language selectable in Vanilla.
    #[serde(default)]
    available_languages: BTreeSet<String>,
}

fn serialize_translations<S: serde::Serializer>(
    translations: &BTreeMap<JavaString, JavaString>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    bbb_protocol::serialize_java_string_map(translations.iter(), serializer)
}

#[derive(Deserialize)]
struct LanguageCatalogSource {
    translations: BTreeMap<JavaString, JavaString>,
    #[serde(default)]
    bidirectional: bool,
    #[serde(default)]
    available_languages: BTreeSet<String>,
}

impl<'de> Deserialize<'de> for LanguageCatalog {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let source = LanguageCatalogSource::deserialize(deserializer)?;
        let display_cache = source
            .translations
            .iter()
            .filter_map(|(key, value)| {
                String::from_utf16(key.units())
                    .ok()
                    .map(|key| (key, value.to_display_string()))
            })
            .collect();
        Ok(Self {
            translations: source.translations,
            display_cache,
            bidirectional: source.bidirectional,
            available_languages: source.available_languages,
        })
    }
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
        self.apply_deprecated_raw(ResourceJsonValue::from_slice(bytes)?)
    }

    pub(crate) fn apply_deprecated_raw(&mut self, raw: ResourceJsonValue) -> Result<()> {
        self.apply_deprecated_info(DeprecatedTranslationsInfo::from_raw(&raw)?)
    }

    pub(crate) fn apply_deprecated_info(&mut self, info: DeprecatedTranslationsInfo) -> Result<()> {
        for key in info.removed {
            self.remove_translation(&key);
        }
        for (from_key, to_key) in info.renamed {
            match self.remove_translation(&from_key) {
                Some(value) => {
                    self.insert_translation(to_key, value);
                }
                None => {
                    self.remove_translation(&to_key);
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
            let raw = match stack.read_resource_json(resource, "language file") {
                Ok(raw) => raw,
                // `ClientLanguage.appendFrom` catches an `IOException`
                // around each resource and keeps walking that stack.
                Err(error) if error.chain().any(|cause| cause.is::<std::io::Error>()) => {
                    continue;
                }
                // Gson syntax/type failures escape `appendFrom` and are
                // caught by `loadFrom`'s namespace-level `Exception`
                // guard, so higher-priority resources in this same stack
                // must not apply after the malformed entry.
                Err(error) => return Err(error),
            };
            self.merge_raw(raw)
                .with_context(|| format!("parse language file {}", resource.path.display()))?;
        }
        Ok(())
    }

    fn merge_json_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        let raw = ResourceJsonValue::from_slice(bytes)?;
        self.merge_raw(raw)
    }

    fn merge_raw(&mut self, raw: ResourceJsonValue) -> Result<()> {
        let entries = raw
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("language file is not an object"))?;
        let unsupported_format = unsupported_format_regex();
        for (key, value) in entries {
            let key: JavaString = key.utf16_units().into();
            let value = translation_value_as_java_string(&key, value)?;
            self.insert_translation(
                key,
                normalize_translation_format(&value, unsupported_format),
            );
        }
        Ok(())
    }

    fn insert_translation(&mut self, key: JavaString, value: JavaString) {
        if let Ok(text) = String::from_utf16(key.units()) {
            self.display_cache.insert(text, value.to_display_string());
        }
        self.translations.insert(key, value);
    }

    fn remove_translation(&mut self, key: &JavaString) -> Option<JavaString> {
        if let Ok(text) = String::from_utf16(key.units()) {
            self.display_cache.remove(&text);
        }
        self.translations.remove(key)
    }

    pub fn get_raw(&self, key: &JavaString) -> Option<&JavaString> {
        self.translations.get(key)
    }

    pub fn get_raw_units(&self, units: &[u16]) -> Option<&JavaString> {
        self.translations.get(units)
    }

    pub fn get_or_key_raw<'a>(&'a self, key: &'a JavaString) -> &'a JavaString {
        self.get_raw(key).unwrap_or(key)
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.display_cache.get(key).map(String::as_str)
    }

    pub fn get_or_key<'a>(&'a self, key: &'a str) -> &'a str {
        self.get_or_default(key, key)
    }

    pub fn get_or_default<'a>(&'a self, key: &str, default_value: &'a str) -> &'a str {
        self.get(key).unwrap_or(default_value)
    }

    pub fn has(&self, key: &str) -> bool {
        self.display_cache.contains_key(key)
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

    pub fn has_available_language(&self, language_code: &str) -> bool {
        self.available_languages.contains(language_code)
    }

    pub(crate) fn set_bidirectional(&mut self, bidirectional: bool) {
        self.bidirectional = bidirectional;
    }

    pub(crate) fn set_available_languages(
        &mut self,
        language_codes: impl IntoIterator<Item = String>,
    ) {
        self.available_languages = language_codes.into_iter().collect();
    }
}

#[derive(Debug)]
pub(crate) struct DeprecatedTranslationsInfo {
    removed: Vec<JavaString>,
    renamed: Vec<(JavaString, JavaString)>,
}

impl DeprecatedTranslationsInfo {
    fn from_raw(raw: &ResourceJsonValue) -> Result<Self> {
        let object = raw
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("deprecated translations is not an object"))?;
        let removed = object
            .get("removed")
            .and_then(ResourceJsonValue::as_array)
            .ok_or_else(|| anyhow::anyhow!("deprecated translations removed is not an array"))?
            .iter()
            .map(|value| {
                value
                    .as_string()
                    .map(|value| value.utf16_units().into())
                    .ok_or_else(|| {
                        anyhow::anyhow!("deprecated translation removed key is not a string")
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        let renamed = object
            .get("renamed")
            .and_then(ResourceJsonValue::as_object)
            .ok_or_else(|| anyhow::anyhow!("deprecated translations renamed is not an object"))?
            .iter()
            .map(|(key, value)| {
                value
                    .as_string()
                    .map(|value| (key.utf16_units().into(), value.utf16_units().into()))
                    .ok_or_else(|| anyhow::anyhow!("deprecated translation target is not a string"))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { removed, renamed })
    }
}

fn language_path(language_code: &str) -> Result<String> {
    ResourceLocation::new("minecraft", format!("lang/{language_code}.json"))
        .map(|location| location.path().to_string())
}

fn translation_value_as_java_string(
    key: &JavaString,
    value: &ResourceJsonValue,
) -> Result<JavaString> {
    match value {
        ResourceJsonValue::String(value) => Ok(value.utf16_units().into()),
        ResourceJsonValue::Bool(value) => Ok(value.to_string().into()),
        ResourceJsonValue::Number(value) => Ok(value.as_str().into()),
        _ => bail!("language entry {key:?} is not a string, number, or boolean"),
    }
}

fn unsupported_format_regex() -> &'static Regex {
    static PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"%([0-9]+\$)?[0-9.]*[df]").expect("language format normalizer regex is valid")
    })
}

fn normalize_translation_format(value: &JavaString, unsupported_format: &Regex) -> JavaString {
    let mut output = Vec::with_capacity(value.units().len());
    let mut run = String::new();
    let flush = |run: &mut String, output: &mut Vec<u16>| {
        output.extend(
            unsupported_format
                .replace_all(run, |captures: &regex::Captures<'_>| {
                    format!(
                        "%{}s",
                        captures.get(1).map_or("", |capture| capture.as_str())
                    )
                })
                .encode_utf16(),
        );
        run.clear();
    };
    for decoded in char::decode_utf16(value.units().iter().copied()) {
        match decoded {
            Ok(character) => run.push(character),
            Err(error) => {
                flush(&mut run, &mut output);
                output.push(error.unpaired_surrogate());
            }
        }
    }
    flush(&mut run, &mut output);
    output.into()
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
            r#"{
              "chat.type.text": "<%s> %s",
              "debug.value": "%1$d / %2$.2f",
              "plain.percent": "100%% ready",
              "unicode.digit": "%٣d %1$d %٣$f %1.٢f",
              "primitive.number": 42,
              "primitive.boolean": true
            }"#
            .as_bytes(),
        )
        .unwrap();

        assert_eq!(catalog.get("chat.type.text"), Some("<%s> %s"));
        assert_eq!(catalog.get("debug.value"), Some("%1$s / %2$s"));
        assert_eq!(catalog.get("plain.percent"), Some("100%% ready"));
        assert_eq!(catalog.get("unicode.digit"), Some("%٣d %1$s %٣$f %1.٢f"));
        assert_eq!(catalog.get("primitive.number"), Some("42"));
        assert_eq!(catalog.get("primitive.boolean"), Some("true"));
        assert_eq!(catalog.get_or_key("missing.key"), "missing.key");
        assert_eq!(
            catalog.get_or_default("missing.key", "explicit fallback"),
            "explicit fallback"
        );
    }

    #[test]
    fn language_numeric_values_keep_source_spelling_through_rename_and_serde() {
        let mut catalog = LanguageCatalog::from_json_bytes(
            br#"{"sci":1e2,"upper":1E+02,"negative_zero":-0,"small":0.0000010,"precise":9007199254740993,"large":18446744073709551616,"huge":1e400,"literal":"1e2"}"#,
        )
        .unwrap();
        for (key, expected) in [
            ("sci", "1e2"),
            ("upper", "1E+02"),
            ("negative_zero", "-0"),
            ("small", "0.0000010"),
            ("precise", "9007199254740993"),
            ("large", "18446744073709551616"),
            ("huge", "1e400"),
            ("literal", "1e2"),
        ] {
            assert_eq!(catalog.get(key), Some(expected));
            assert_eq!(
                catalog.get_raw(&key.into()),
                Some(&JavaString::from(expected))
            );
        }
        catalog
            .apply_deprecated_json_bytes(br#"{"removed":[],"renamed":{"huge":"renamed"}}"#)
            .unwrap();
        assert_eq!(catalog.get("huge"), None);
        assert_eq!(catalog.get("renamed"), Some("1e400"));
        let serialized = serde_json::to_value(&catalog).unwrap();
        assert_eq!(serialized["translations"]["renamed"], "1e400");
        assert_eq!(
            serde_json::from_value::<LanguageCatalog>(serialized).unwrap(),
            catalog
        );
    }

    #[test]
    fn language_catalog_serde_preserves_raw_translation_keys_and_values() {
        let catalog = LanguageCatalog::from_json_bytes(
            br#"{"\ud800":"\ud801","\ud801":"\ud800","\ufffd":"replacement","plain":"\\uD800"}"#,
        )
        .unwrap();
        let serialized = serde_json::to_string(&catalog).unwrap();
        let restored: LanguageCatalog = serde_json::from_str(&serialized).unwrap();
        assert_eq!(restored, catalog);
        assert_eq!(restored.len(), 4);
        assert_eq!(
            restored.get_raw_units(&[0xd800]),
            Some(&JavaString::from(vec![0xd801]))
        );
        assert_eq!(
            restored.get_raw_units(&[0xd801]),
            Some(&JavaString::from(vec![0xd800]))
        );
        assert_eq!(restored.get("�"), Some("replacement"));
        let projected = bbb_protocol::to_display_json_value(&catalog).unwrap();
        assert!(projected["translations"].is_object());
        assert_eq!(projected["translations"]["�"], "replacement");
        assert_eq!(projected["translations"]["plain"], "\\uD800");
    }

    #[test]
    fn deprecated_renames_follow_source_order_and_duplicate_last_write() {
        let mut catalog =
            LanguageCatalog::from_json_bytes(br#"{"z":"Z","a":"A","b":"B","x":"X"}"#).unwrap();
        catalog
            .apply_deprecated_json_bytes(br#"{"removed":[],"renamed":{"z":"x","a":"b","z":"a"}}"#)
            .unwrap();
        assert_eq!(catalog.get("z"), None);
        assert_eq!(catalog.get("a"), None);
        assert_eq!(catalog.get("b"), Some("Z"));
        assert_eq!(catalog.get("x"), Some("X"));
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
    fn language_pack_priority_accepts_large_exponent_translation() {
        let root = unique_temp_dir("language-number-priority");
        let base = root.join("base");
        let overlay = root.join("overlay");
        write_json(
            &base.join("assets/minecraft/lang/en_us.json"),
            r#"{"numeric.translation":"base","unchanged":"base"}"#,
        );
        write_json(
            &overlay.join("assets/minecraft/lang/en_us.json"),
            r#"{"numeric.translation":1e400,"other":"overlay"}"#,
        );

        let stack = PackResourceStack::from_roots([base, overlay]);
        let catalog =
            LanguageCatalog::load_resource_stack(&stack, &[DEFAULT_LANGUAGE_CODE]).unwrap();
        assert_eq!(catalog.get("numeric.translation"), Some("1e400"));
        assert_eq!(catalog.get("other"), Some("overlay"));
        assert_eq!(catalog.get("unchanged"), Some("base"));
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
    fn malformed_language_json_stops_only_its_namespace_resource_stack() {
        let root = unique_temp_dir("language-bad-middle-resource");
        let base = root.join("base");
        let malformed = root.join("malformed");
        let higher = root.join("higher");
        write_json(
            &base.join("assets/minecraft/lang/en_us.json"),
            r#"{"stack.value":"base","\ud800":"raw base"}"#,
        );
        write_json(&malformed.join("assets/minecraft/lang/en_us.json"), "{");
        write_json(
            &higher.join("assets/minecraft/lang/en_us.json"),
            r#"{"stack.value":"higher","\ud800":"raw higher"}"#,
        );
        write_json(
            &higher.join("assets/custom/lang/en_us.json"),
            r#"{"custom.value":"still loaded"}"#,
        );

        let stack = PackResourceStack::from_roots([base, malformed, higher]);
        let catalog =
            LanguageCatalog::load_resource_stack(&stack, &[DEFAULT_LANGUAGE_CODE]).unwrap();

        assert_eq!(catalog.get("stack.value"), Some("base"));
        assert_eq!(
            catalog.get_raw(&vec![0xd800].into()),
            Some(&JavaString::from("raw base"))
        );
        assert_eq!(catalog.get("custom.value"), Some("still loaded"));

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
        assert!(fallback_catalog.has_available_language(DEFAULT_LANGUAGE_CODE));
        assert!(!fallback_catalog.has_available_language("pirate"));

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
        assert!(selected_catalog.has_available_language("pirate"));

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
        let roots = PackRoots::from_environment().unwrap();
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
        let roots = PackRoots::from_environment().unwrap();
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
        std::env::temp_dir().join(format!("bbb-client-assets-{label}-{nanos}"))
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
            let hash = crate::hex_lower(&Sha1::digest(bytes));
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
        let index_hash = crate::hex_lower(&Sha1::digest(index.as_bytes()));
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
