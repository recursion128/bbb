//! Non-bundled Vanilla language object resolution and SHA-1 verified mounting for Web.

use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use anyhow::{bail, Result};
use bbb_client_assets::PackRoots;
#[cfg(test)]
use {anyhow::Context, serde::Deserialize};

#[cfg(test)]
const VANILLA_LANG_KEY_PREFIX: &str = "minecraft/lang/";
#[cfg(test)]
const VANILLA_LANG_KEY_SUFFIX: &str = ".json";
const WEB_VANILLA_OBJECTS_ROOT: &str = "/__bbb_mem__/vanilla/objects";
const MAX_LOCALE_JSON_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WebLocaleObjectEntry {
    pub(crate) hash: String,
    pub(crate) size: u64,
}

impl WebLocaleObjectEntry {
    pub(crate) fn virtual_path(&self) -> PathBuf {
        PathBuf::from(format!(
            "{WEB_VANILLA_OBJECTS_ROOT}/{}/{}",
            &self.hash[..2],
            self.hash
        ))
    }

    pub(crate) fn http_path(&self) -> String {
        format!("/assets/objects/{}/{}", &self.hash[..2], self.hash)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct WebLocaleCatalog {
    entries: BTreeMap<String, WebLocaleObjectEntry>,
}

#[cfg(test)]
#[derive(Deserialize)]
struct RawAssetIndexDocument {
    #[serde(default)]
    objects: BTreeMap<String, RawAssetObject>,
}

#[cfg(test)]
#[derive(Deserialize)]
struct RawAssetObject {
    hash: String,
    size: u64,
}

impl WebLocaleCatalog {
    pub(crate) fn from_pack_roots(roots: &PackRoots) -> Self {
        let mut entries = BTreeMap::new();
        for (code, hash, size) in roots.indexed_vanilla_lang_objects() {
            if size <= MAX_LOCALE_JSON_BYTES {
                entries.insert(code, WebLocaleObjectEntry { hash, size });
            }
        }
        Self { entries }
    }

    #[cfg(test)]
    pub(crate) fn from_asset_index_json(json_bytes: &[u8]) -> Result<Self> {
        let parsed: RawAssetIndexDocument =
            serde_json::from_slice(json_bytes).context("parse web asset-index.json")?;
        let mut entries = BTreeMap::new();
        for (logical_key, obj) in parsed.objects {
            let Some(rest) = logical_key.strip_prefix(VANILLA_LANG_KEY_PREFIX) else {
                continue;
            };
            let Some(code) = rest.strip_suffix(VANILLA_LANG_KEY_SUFFIX) else {
                continue;
            };
            if code.is_empty() || code.contains('/') || obj.size > MAX_LOCALE_JSON_BYTES {
                continue;
            }
            let hash = obj.hash.trim().to_ascii_lowercase();
            if hash.len() != 40 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
                continue;
            }
            entries.insert(
                code.to_ascii_lowercase(),
                WebLocaleObjectEntry {
                    hash,
                    size: obj.size,
                },
            );
        }
        Ok(Self { entries })
    }

    pub(crate) fn entry_for_locale(&self, locale: &str) -> Option<&WebLocaleObjectEntry> {
        let code = locale.trim().to_ascii_lowercase();
        if code.is_empty() || code == "en_us" {
            return None;
        }
        self.entries.get(&code)
    }
}

pub(crate) fn verify_and_mount_locale_object(
    entry: &WebLocaleObjectEntry,
    bytes: Vec<u8>,
) -> Result<PathBuf> {
    let actual_size = bytes.len() as u64;
    if actual_size != entry.size {
        bail!(
            "locale asset {} byte size mismatch (expected {}, got {})",
            entry.hash,
            entry.size,
            actual_size
        );
    }
    let actual_sha1 = bbb_client_net::sha1_hex_lower(&bytes);
    if actual_sha1 != entry.hash {
        bail!(
            "locale asset SHA-1 mismatch (expected {}, got {})",
            entry.hash,
            actual_sha1
        );
    }
    let virtual_path = entry.virtual_path();
    bbb_client_assets::virtual_store::insert_virtual_file(&virtual_path, Arc::from(bytes));
    Ok(virtual_path)
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn ensure_locale_mounted(
    window: &web_sys::Window,
    catalog: &WebLocaleCatalog,
    locale: &str,
) -> Result<bool> {
    let Some(entry) = catalog.entry_for_locale(locale) else {
        return Ok(false);
    };
    let virtual_path = entry.virtual_path();
    if bbb_client_assets::virtual_store::has_virtual_file(&virtual_path) {
        return Ok(false);
    }
    let url = entry.http_path();
    let resp = super::fetch_response(window, &url).await?;
    let bytes = super::response_bytes(&resp, &url).await?;
    verify_and_mount_locale_object(entry, bytes)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

    #[test]
    fn locale_catalog_resolves_non_bundled_locales_and_verifies_sha1_plus_rtl() {
        let ar_payload = br#"{"menu.singleplayer":"\u0644\u0639\u0628 \u0641\u0631\u062f\u064a"}"#;
        let ar_hash = bbb_client_net::sha1_hex_lower(ar_payload);
        let ja_payload = br#"{"menu.singleplayer":"\u30b7\u30f3\u30b0\u30eb\u30d7\u30ec\u30a4"}"#;
        let ja_hash = bbb_client_net::sha1_hex_lower(ja_payload);

        let index_json = format!(
            r#"{{
                "objects": {{
                    "minecraft/lang/ar_sa.json": {{"hash": "{ar_hash}", "size": {ar_len}}},
                    "minecraft/lang/ja_jp.json": {{"hash": "{ja_hash}", "size": {ja_len}}},
                    "minecraft/sounds/ambient/cave.ogg": {{"hash": "0123456789abcdef0123456789abcdef01234567", "size": 99}}
                }}
            }}"#,
            ar_len = ar_payload.len(),
            ja_len = ja_payload.len(),
        );

        let catalog = WebLocaleCatalog::from_asset_index_json(index_json.as_bytes()).unwrap();
        assert!(catalog.entry_for_locale("en_us").is_none());
        assert!(catalog.entry_for_locale("EN_US").is_none());

        let ar_entry = catalog.entry_for_locale("AR_SA").expect("ar_sa entry");
        assert_eq!(
            ar_entry.http_path(),
            format!("/assets/objects/{}/{}", &ar_hash[..2], ar_hash)
        );
        assert!(verify_and_mount_locale_object(ar_entry, b"tampered".to_vec()).is_err());

        let mut wrong_hash_same_len = ar_payload.to_vec();
        wrong_hash_same_len[0] = b' ';
        assert!(verify_and_mount_locale_object(ar_entry, wrong_hash_same_len).is_err());

        let mounted = verify_and_mount_locale_object(ar_entry, ar_payload.to_vec()).unwrap();
        assert!(bbb_client_assets::virtual_store::has_virtual_file(&mounted));

        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            zip.start_file("pack.mcmeta", opts).unwrap();
            zip.write_all(
                br#"{
                    "pack": {"description": "core", "min_format": [84, 0], "max_format": [84, 0]},
                    "language": {
                        "en_us": {"name": "English", "region": "US", "bidirectional": false},
                        "ar_sa": {"name": "Arabic", "region": "SA", "bidirectional": true},
                        "ja_jp": {"name": "Japanese", "region": "JP", "bidirectional": false}
                    }
                }"#,
            )
            .unwrap();
            zip.start_file("assets/minecraft/lang/en_us.json", opts)
                .unwrap();
            zip.write_all(br#"{"menu.singleplayer":"Singleplayer","menu.quit":"Quit"}"#)
                .unwrap();
            zip.finish().unwrap();
        }

        let roots = bbb_client_assets::PackRoots::from_web_bundle(
            Arc::from(cursor.into_inner()),
            Some("test-web-locales"),
            Some(index_json.as_bytes()),
        )
        .unwrap();
        let roots_catalog = WebLocaleCatalog::from_pack_roots(&roots);
        assert_eq!(roots_catalog.entry_for_locale("ar_sa"), Some(ar_entry));
        assert!(roots_catalog.entry_for_locale("en_us").is_none());
        let lang = roots.load_client_language_catalog("ar_sa").unwrap();
        assert!(lang.is_bidirectional());
        assert_eq!(lang.get_or_key("menu.singleplayer"), "لعب فردي");
        assert_eq!(lang.get_or_key("menu.quit"), "Quit");
    }
}
