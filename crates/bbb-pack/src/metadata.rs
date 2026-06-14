use std::collections::BTreeMap;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{language::DEFAULT_LANGUAGE_CODE, resources::PackResourceStack};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackMetadataCatalog {
    pub languages: BTreeMap<String, LanguageInfo>,
}

impl PackMetadataCatalog {
    pub fn load_resource_stack(stack: &PackResourceStack) -> Self {
        let mut catalog = Self::default();
        catalog.languages.insert(
            DEFAULT_LANGUAGE_CODE.to_string(),
            LanguageInfo::default_english(),
        );

        for root in stack.roots() {
            let metadata_path = root.join("pack.mcmeta");
            if !metadata_path.is_file() {
                continue;
            }
            let Ok(bytes) = std::fs::read(&metadata_path) else {
                continue;
            };
            let Ok(metadata) = RawPackMetadata::from_json_bytes(&bytes) else {
                continue;
            };
            for (code, info) in metadata.languages {
                catalog.languages.entry(code).or_insert(info);
            }
        }

        catalog
    }

    pub fn language(&self, code: &str) -> Option<&LanguageInfo> {
        self.languages.get(code)
    }

    pub fn language_stack(&self, selected_code: &str) -> Vec<String> {
        let mut stack = vec![DEFAULT_LANGUAGE_CODE.to_string()];
        if selected_code != DEFAULT_LANGUAGE_CODE && self.languages.contains_key(selected_code) {
            stack.push(selected_code.to_string());
        }
        stack
    }

    pub fn selected_bidirectional(&self, selected_code: &str) -> bool {
        if selected_code == DEFAULT_LANGUAGE_CODE {
            return false;
        }
        self.languages
            .get(selected_code)
            .is_some_and(|info| info.bidirectional)
    }

    pub fn len(&self) -> usize {
        self.languages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.languages.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub region: String,
    pub name: String,
    pub bidirectional: bool,
}

impl LanguageInfo {
    pub fn default_english() -> Self {
        Self {
            region: "US".to_string(),
            name: "English".to_string(),
            bidirectional: false,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawPackMetadata {
    #[serde(default)]
    language: BTreeMap<String, RawLanguageInfo>,
}

impl RawPackMetadata {
    fn from_json_bytes(bytes: &[u8]) -> Result<ParsedPackMetadata> {
        let raw: Self = serde_json::from_slice(bytes)?;
        let mut languages = BTreeMap::new();
        for (code, info) in raw.language {
            validate_language_code(&code)?;
            languages.insert(code, info.into_language_info()?);
        }
        Ok(ParsedPackMetadata { languages })
    }
}

struct ParsedPackMetadata {
    languages: BTreeMap<String, LanguageInfo>,
}

#[derive(Debug, Deserialize)]
struct RawLanguageInfo {
    region: String,
    name: String,
    #[serde(default)]
    bidirectional: bool,
}

impl RawLanguageInfo {
    fn into_language_info(self) -> Result<LanguageInfo> {
        if self.region.is_empty() {
            bail!("language region is empty");
        }
        if self.name.is_empty() {
            bail!("language name is empty");
        }
        Ok(LanguageInfo {
            region: self.region,
            name: self.name,
            bidirectional: self.bidirectional,
        })
    }
}

fn validate_language_code(code: &str) -> Result<()> {
    if code.is_empty() || code.len() > 16 {
        bail!("invalid language code {code:?}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{LanguageInfo, PackMetadataCatalog, PackResourceStack, DEFAULT_LANGUAGE_CODE};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn pack_metadata_catalog_loads_default_language_and_pack_languages() {
        let root = unique_temp_dir("metadata-languages");
        let base = root.join("base");
        let overlay = root.join("overlay");
        write_json(
            &base.join("pack.mcmeta"),
            r#"{
              "language": {
                "pirate": {
                  "region": "Seven Seas",
                  "name": "Pirate Speak"
                },
                "ar_sa": {
                  "region": "SA",
                  "name": "Arabic",
                  "bidirectional": true
                }
              }
            }"#,
        );
        write_json(
            &overlay.join("pack.mcmeta"),
            r#"{
              "language": {
                "pirate": {
                  "region": "Overlay",
                  "name": "Should Not Replace"
                },
                "tok": {
                  "region": "Tok",
                  "name": "Toki Pona"
                }
              }
            }"#,
        );

        let catalog = PackMetadataCatalog::load_resource_stack(&PackResourceStack::from_roots([
            base, overlay,
        ]));

        assert_eq!(
            catalog.language(DEFAULT_LANGUAGE_CODE),
            Some(&LanguageInfo::default_english())
        );
        assert_eq!(catalog.language("pirate").unwrap().region, "Seven Seas");
        assert!(catalog.language("ar_sa").unwrap().bidirectional);
        assert_eq!(catalog.language("tok").unwrap().name, "Toki Pona");
        assert_eq!(
            catalog.language_stack("pirate"),
            vec!["en_us".to_string(), "pirate".to_string()]
        );
        assert_eq!(catalog.language_stack("missing"), vec!["en_us".to_string()]);
        assert!(catalog.selected_bidirectional("ar_sa"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pack_metadata_catalog_skips_invalid_language_metadata_pack() {
        let root = unique_temp_dir("metadata-invalid-language");
        let bad = root.join("bad");
        let good = root.join("good");
        write_json(
            &bad.join("pack.mcmeta"),
            r#"{
              "language": {
                "toolong_language_code": {
                  "region": "Bad",
                  "name": "Bad"
                },
                "valid": {
                  "region": "Valid",
                  "name": "Skipped With Pack"
                }
              }
            }"#,
        );
        write_json(
            &good.join("pack.mcmeta"),
            r#"{
              "language": {
                "valid": {
                  "region": "Good",
                  "name": "Loaded"
                }
              }
            }"#,
        );

        let catalog =
            PackMetadataCatalog::load_resource_stack(&PackResourceStack::from_roots([bad, good]));

        assert!(catalog.language("toolong_language_code").is_none());
        assert_eq!(catalog.language("valid").unwrap().region, "Good");

        std::fs::remove_dir_all(root).unwrap();
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
}
