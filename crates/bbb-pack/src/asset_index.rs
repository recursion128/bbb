use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::{ResourceIntegrity, ResourceLocation};

const MAX_VERSION_METADATA_BYTES: u64 = 4 * 1024 * 1024;
const MAX_ASSET_INDEX_BYTES: u64 = 4 * 1024 * 1024;
const MAX_ASSET_INDEX_OBJECTS: usize = 100_000;
const MAX_ASSET_OBJECT_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ASSET_INDEX_TOTAL_OBJECT_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const ASSET_INDEX_ENTRY_LOGICAL_BYTES: u64 = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VanillaAssetIndex {
    index_id: String,
    objects_dir: PathBuf,
    objects: BTreeMap<String, AssetObject>,
    retained_logical_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexedAsset {
    pub(crate) path: PathBuf,
    pub(crate) integrity: ResourceIntegrity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VanillaAssetIndexObservation {
    pub index_id: String,
    pub object_count: usize,
    pub exposed_resource_count: usize,
    pub root_resource_count: usize,
    pub retained_logical_bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct VersionMetadata {
    #[serde(rename = "assetIndex")]
    asset_index: AssetIndexMetadata,
}

#[derive(Debug, Clone, Deserialize)]
struct AssetIndexMetadata {
    id: String,
    sha1: String,
    size: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct AssetIndexDocument {
    objects: BTreeMap<String, AssetObject>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct AssetObject {
    hash: String,
    size: u64,
}

impl VanillaAssetIndex {
    pub(crate) fn discover(vanilla_root: &Path) -> Result<Option<Self>> {
        let downloads = vanilla_root.join("downloads").join(crate::MC_VERSION);
        let version_metadata_path = downloads.join(format!("{}.json", crate::MC_VERSION));
        if !version_metadata_path.is_file() {
            return Ok(None);
        }

        let version: VersionMetadata = read_json_file_limited(
            &version_metadata_path,
            MAX_VERSION_METADATA_BYTES,
            "Vanilla version metadata",
        )?;
        validate_index_id(&version.asset_index.id)?;
        validate_sha1(&version.asset_index.sha1, "asset index SHA-1")?;

        let assets_dir = downloads.join("assets");
        let index_path = assets_dir
            .join("indexes")
            .join(format!("{}.json", version.asset_index.id));
        if version.asset_index.size > MAX_ASSET_INDEX_BYTES {
            bail!(
                "Vanilla asset index {} is {} bytes, exceeding the {} byte limit",
                index_path.display(),
                version.asset_index.size,
                MAX_ASSET_INDEX_BYTES
            );
        }
        verify_file(
            &index_path,
            version.asset_index.size,
            &version.asset_index.sha1,
            "Vanilla asset index",
        )?;
        let document: AssetIndexDocument =
            read_json_file_limited(&index_path, MAX_ASSET_INDEX_BYTES, "Vanilla asset index")?;
        let retained_logical_bytes = validate_asset_index_document(&document)?;

        Ok(Some(Self {
            index_id: version.asset_index.id,
            objects_dir: assets_dir.join("objects"),
            objects: document.objects,
            retained_logical_bytes,
        }))
    }

    pub(crate) fn resource(&self, location: &ResourceLocation) -> Option<IndexedAsset> {
        let key = format!("{}/{}", location.namespace(), location.path());
        self.objects
            .get(&key)
            .map(|object| self.indexed_asset(object))
    }

    pub(crate) fn root_resource(&self, path: &str) -> Option<IndexedAsset> {
        self.objects
            .get(path)
            .map(|object| self.indexed_asset(object))
    }

    pub(crate) fn list_resources<'a>(
        &'a self,
        path_prefix: &'a str,
        extension: &'a str,
    ) -> impl Iterator<Item = (ResourceLocation, IndexedAsset)> + 'a {
        self.objects.iter().filter_map(move |(key, object)| {
            let location = resource_location_for_asset_key(key).ok()?;
            if !matches!(location.namespace(), "minecraft" | "realms")
                || !resource_path_is_below(location.path(), path_prefix)
                || !location.path().ends_with(extension)
            {
                return None;
            }
            Some((location, self.indexed_asset(object)))
        })
    }

    pub(crate) fn observation(&self) -> VanillaAssetIndexObservation {
        let exposed_resource_count = self
            .objects
            .keys()
            .filter_map(|key| resource_location_for_asset_key(key).ok())
            .filter(|location| matches!(location.namespace(), "minecraft" | "realms"))
            .count();
        let root_resource_count = self
            .objects
            .keys()
            .filter(|key| !key.contains('/') || key.starts_with("icons/"))
            .count();
        VanillaAssetIndexObservation {
            index_id: self.index_id.clone(),
            object_count: self.objects.len(),
            exposed_resource_count,
            root_resource_count,
            retained_logical_bytes: self.retained_logical_bytes,
        }
    }

    fn indexed_asset(&self, object: &AssetObject) -> IndexedAsset {
        IndexedAsset {
            path: self.objects_dir.join(&object.hash[..2]).join(&object.hash),
            integrity: ResourceIntegrity {
                size: object.size,
                sha1: object.hash.clone(),
            },
        }
    }
}

fn validate_asset_index_document(document: &AssetIndexDocument) -> Result<u64> {
    validate_asset_index_object_count(document.objects.len())?;
    let mut total_object_bytes = 0_u64;
    let mut retained_logical_bytes = 0_u64;
    for (name, object) in &document.objects {
        validate_asset_key(name)
            .with_context(|| format!("invalid Vanilla asset-index key {name:?}"))?;
        validate_sha1(&object.hash, &format!("asset object {name:?} SHA-1"))?;
        if object.size > MAX_ASSET_OBJECT_BYTES {
            bail!(
                "Vanilla asset object {name:?} is {} bytes, exceeding the {} byte limit",
                object.size,
                MAX_ASSET_OBJECT_BYTES
            );
        }
        total_object_bytes = total_object_bytes
            .checked_add(object.size)
            .context("Vanilla asset-index total object size overflow")?;
        if total_object_bytes > MAX_ASSET_INDEX_TOTAL_OBJECT_BYTES {
            bail!(
                "Vanilla asset index declares {total_object_bytes} object bytes, exceeding the {} byte limit",
                MAX_ASSET_INDEX_TOTAL_OBJECT_BYTES
            );
        }
        let entry_bytes = u64::try_from(name.len())
            .context("Vanilla asset-index key length overflow")?
            .checked_add(u64::try_from(object.hash.len()).expect("SHA-1 length fits u64"))
            .and_then(|bytes| bytes.checked_add(ASSET_INDEX_ENTRY_LOGICAL_BYTES))
            .context("Vanilla asset-index retained byte count overflow")?;
        retained_logical_bytes = retained_logical_bytes
            .checked_add(entry_bytes)
            .context("Vanilla asset-index retained byte count overflow")?;
    }
    Ok(retained_logical_bytes)
}

fn validate_asset_index_object_count(count: usize) -> Result<()> {
    if count > MAX_ASSET_INDEX_OBJECTS {
        bail!(
            "Vanilla asset index has {count} objects, exceeding the {MAX_ASSET_INDEX_OBJECTS} object limit"
        );
    }
    Ok(())
}

fn resource_path_is_below(path: &str, prefix: &str) -> bool {
    prefix.is_empty()
        || path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn read_json_file_limited<T: serde::de::DeserializeOwned>(
    path: &Path,
    maximum: u64,
    label: &str,
) -> Result<T> {
    let file = File::open(path).with_context(|| format!("open {label} {}", path.display()))?;
    let declared = file
        .metadata()
        .with_context(|| format!("read {label} metadata {}", path.display()))?
        .len();
    if declared > maximum {
        bail!(
            "{label} {} is {declared} bytes, exceeding the {maximum} byte limit",
            path.display()
        );
    }
    let mut bytes = Vec::with_capacity(usize::try_from(declared).context("JSON size overflow")?);
    file.take(maximum + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("read {label} {}", path.display()))?;
    if bytes.len() as u64 > maximum {
        bail!(
            "{label} {} grew beyond the {maximum} byte limit while being read",
            path.display()
        );
    }
    serde_json::from_slice(&bytes).with_context(|| format!("parse {label} {}", path.display()))
}

fn verify_file(path: &Path, expected_size: u64, expected_sha1: &str, label: &str) -> Result<()> {
    validate_sha1(expected_sha1, &format!("{label} SHA-1"))?;
    ResourceIntegrity {
        size: expected_size,
        sha1: expected_sha1.to_string(),
    }
    .verify(path, label)
}

fn validate_index_id(id: &str) -> Result<()> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        bail!("invalid Vanilla asset-index id {id:?}");
    }
    Ok(())
}

fn validate_sha1(hash: &str, label: &str) -> Result<()> {
    if hash.len() != 40
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("{label} is not a lowercase 40-digit SHA-1: {hash:?}");
    }
    Ok(())
}

fn resource_location_for_asset_key(key: &str) -> Result<ResourceLocation> {
    let (namespace, path) = key
        .split_once('/')
        .with_context(|| format!("asset key has no namespace separator: {key:?}"))?;
    ResourceLocation::new(namespace, path)
}

fn validate_asset_key(key: &str) -> Result<()> {
    // The launcher index has one root-level default-pack metadata entry. All
    // resource-manager assets are namespace-prefixed and must pass normal
    // `ResourceLocation` validation.
    if key == "pack.mcmeta" {
        return Ok(());
    }
    resource_location_for_asset_key(key).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha1::{Digest, Sha1};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn discovers_and_verifies_indexed_object() {
        let root = fixture_root("valid");
        let object = b"font archive";
        let object_hash = sha1_bytes(object);
        let index = format!(
            r#"{{"objects":{{"minecraft/font/unifont.zip":{{"hash":"{object_hash}","size":{}}}}}}}"#,
            object.len()
        );
        write_fixture(&root, &index, &object_hash, object);

        let catalog = VanillaAssetIndex::discover(&root).unwrap().unwrap();
        let location = ResourceLocation::parse("minecraft:font/unifont.zip").unwrap();
        let resolved = catalog.resource(&location).unwrap();
        resolved
            .integrity
            .verify(&resolved.path, "Vanilla asset")
            .unwrap();
        assert_eq!(std::fs::read(resolved.path).unwrap(), object);
        assert!(catalog
            .resource(&ResourceLocation::parse("minecraft:font/missing.zip").unwrap())
            .is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_wrong_object_size_and_sha1() {
        let root = fixture_root("bad-object");
        let object = b"font archive";
        let object_hash = sha1_bytes(object);
        let index = format!(
            r#"{{"objects":{{"minecraft/font/unifont.zip":{{"hash":"{object_hash}","size":{}}}}}}}"#,
            object.len()
        );
        write_fixture(&root, &index, &object_hash, object);
        let catalog = VanillaAssetIndex::discover(&root).unwrap().unwrap();
        let path = root
            .join("downloads/26.1/assets/objects")
            .join(&object_hash[..2])
            .join(&object_hash);
        std::fs::write(&path, b"changed").unwrap();
        let resolved = catalog
            .resource(&ResourceLocation::parse("minecraft:font/unifont.zip").unwrap())
            .unwrap();
        let error = resolved
            .integrity
            .verify(&resolved.path, "Vanilla asset")
            .unwrap_err()
            .to_string();
        assert!(error.contains("has size"), "{error}");
        std::fs::write(&path, b"font archivf").unwrap();
        let error = resolved
            .integrity
            .verify(&resolved.path, "Vanilla asset")
            .unwrap_err()
            .to_string();
        assert!(error.contains("has SHA-1"), "{error}");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_wrong_index_metadata_and_missing_object() {
        let root = fixture_root("bad-index");
        let object = b"font archive";
        let object_hash = sha1_bytes(object);
        let index = format!(
            r#"{{"objects":{{"minecraft/font/unifont.zip":{{"hash":"{object_hash}","size":{}}}}}}}"#,
            object.len()
        );
        write_fixture(&root, &index, &object_hash, object);
        let metadata_path = root.join("downloads/26.1/26.1.json");
        let index_hash = sha1_bytes(index.as_bytes());

        std::fs::write(
            &metadata_path,
            format!(
                r#"{{"assetIndex":{{"id":"30","sha1":"{index_hash}","size":{}}}}}"#,
                index.len() + 1
            ),
        )
        .unwrap();
        let error = VanillaAssetIndex::discover(&root).unwrap_err().to_string();
        assert!(error.contains("has size"), "{error}");

        std::fs::write(
            &metadata_path,
            format!(
                r#"{{"assetIndex":{{"id":"30","sha1":"{}","size":{}}}}}"#,
                "0".repeat(40),
                index.len()
            ),
        )
        .unwrap();
        let error = VanillaAssetIndex::discover(&root).unwrap_err().to_string();
        assert!(error.contains("has SHA-1"), "{error}");

        std::fs::write(
            &metadata_path,
            format!(
                r#"{{"assetIndex":{{"id":"30","sha1":"{index_hash}","size":{}}}}}"#,
                index.len()
            ),
        )
        .unwrap();
        let catalog = VanillaAssetIndex::discover(&root).unwrap().unwrap();
        let object_path = root
            .join("downloads/26.1/assets/objects")
            .join(&object_hash[..2])
            .join(&object_hash);
        std::fs::remove_file(object_path).unwrap();
        let resolved = catalog
            .resource(&ResourceLocation::parse("minecraft:font/unifont.zip").unwrap())
            .unwrap();
        let error = resolved
            .integrity
            .verify(&resolved.path, "Vanilla asset")
            .unwrap_err()
            .to_string();
        assert!(error.contains("open Vanilla asset"), "{error}");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_path_escape_index_id_and_asset_key() {
        assert!(validate_index_id("../30").is_err());
        assert!(validate_asset_key("minecraft/../escape").is_err());
        assert!(validate_asset_key("minecraft\\font/unifont.zip").is_err());
        assert!(validate_asset_key("minecraft//absolute").is_err());
        assert!(validate_asset_key("Minecraft/font/unifont.zip").is_err());
        assert!(validate_asset_key("minecraft/font/\0.zip").is_err());
        assert!(validate_asset_key("pack.mcmeta").is_ok());
    }

    #[test]
    fn missing_version_metadata_keeps_external_asset_source_optional() {
        let root = fixture_root("missing-metadata");

        assert!(VanillaAssetIndex::discover(&root).unwrap().is_none());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_oversized_index_before_opening_or_hashing_it() {
        let root = fixture_root("oversized-index");
        let downloads = root.join("downloads/26.1");
        std::fs::create_dir_all(&downloads).unwrap();
        std::fs::write(
            downloads.join("26.1.json"),
            format!(
                r#"{{"assetIndex":{{"id":"30","sha1":"{}","size":{}}}}}"#,
                "0".repeat(40),
                MAX_ASSET_INDEX_BYTES + 1
            ),
        )
        .unwrap();

        let error = VanillaAssetIndex::discover(&root).unwrap_err().to_string();
        assert!(
            error.contains("exceeding the 4194304 byte limit"),
            "{error}"
        );
        assert!(!error.contains("open Vanilla asset index"), "{error}");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_unbounded_object_count_and_total_declared_bytes() {
        let error = validate_asset_index_object_count(MAX_ASSET_INDEX_OBJECTS + 1)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("exceeding the 100000 object limit"),
            "{error}"
        );

        let objects = (0..=MAX_ASSET_INDEX_TOTAL_OBJECT_BYTES / MAX_ASSET_OBJECT_BYTES)
            .map(|index| {
                (
                    format!("minecraft/sounds/{index}.ogg"),
                    AssetObject {
                        hash: "0".repeat(40),
                        size: MAX_ASSET_OBJECT_BYTES,
                    },
                )
            })
            .collect();
        let error = validate_asset_index_document(&AssetIndexDocument { objects })
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("exceeding the 17179869184 byte limit"),
            "{error}"
        );
    }

    #[test]
    fn rejects_unbounded_declared_object_size_without_opening_an_object() {
        let root = fixture_root("oversized-object");
        let hash = "0".repeat(40);
        let index = format!(
            r#"{{"objects":{{"minecraft/sounds/huge.ogg":{{"hash":"{hash}","size":{}}}}}}}"#,
            MAX_ASSET_OBJECT_BYTES + 1
        );
        write_fixture(&root, &index, &hash, b"");
        let error = VanillaAssetIndex::discover(&root).unwrap_err().to_string();
        assert!(
            error.contains("exceeding the 536870912 byte limit"),
            "{error}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    fn write_fixture(root: &Path, index: &str, object_hash: &str, object: &[u8]) {
        let downloads = root.join("downloads/26.1");
        let index_path = downloads.join("assets/indexes/30.json");
        let object_path = downloads
            .join("assets/objects")
            .join(&object_hash[..2])
            .join(object_hash);
        std::fs::create_dir_all(index_path.parent().unwrap()).unwrap();
        std::fs::create_dir_all(object_path.parent().unwrap()).unwrap();
        std::fs::write(&index_path, index).unwrap();
        std::fs::write(&object_path, object).unwrap();
        let index_hash = sha1_bytes(index.as_bytes());
        std::fs::write(
            downloads.join("26.1.json"),
            format!(
                r#"{{"assetIndex":{{"id":"30","sha1":"{index_hash}","size":{}}}}}"#,
                index.len()
            ),
        )
        .unwrap();
    }

    fn sha1_bytes(bytes: &[u8]) -> String {
        format!("{:x}", Sha1::digest(bytes))
    }

    fn fixture_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "bbb-pack-asset-index-{}-{label}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
    }
}
