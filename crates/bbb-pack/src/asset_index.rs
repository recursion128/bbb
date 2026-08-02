use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Take},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use sha1::{Digest, Sha1};

use crate::ResourceLocation;

const MAX_VERSION_METADATA_BYTES: u64 = 4 * 1024 * 1024;
const MAX_ASSET_INDEX_BYTES: u64 = 4 * 1024 * 1024;
const MAX_ASSET_INDEX_OBJECTS: usize = 100_000;

#[derive(Debug, Clone)]
pub(crate) struct VanillaAssetIndex {
    index_id: String,
    objects_dir: PathBuf,
    objects: BTreeMap<String, AssetObject>,
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

#[derive(Debug, Clone, Deserialize)]
struct AssetObject {
    hash: String,
    size: u64,
}

impl VanillaAssetIndex {
    pub(crate) fn discover(mc_code_root: &Path) -> Result<Option<Self>> {
        let downloads = mc_code_root.join("downloads").join(crate::MC_VERSION);
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
        verify_file(
            &index_path,
            version.asset_index.size,
            &version.asset_index.sha1,
            "Vanilla asset index",
        )?;
        if version.asset_index.size > MAX_ASSET_INDEX_BYTES {
            bail!(
                "Vanilla asset index {} is {} bytes, exceeding the {} byte limit",
                index_path.display(),
                version.asset_index.size,
                MAX_ASSET_INDEX_BYTES
            );
        }
        let document: AssetIndexDocument =
            read_json_file_limited(&index_path, MAX_ASSET_INDEX_BYTES, "Vanilla asset index")?;
        if document.objects.len() > MAX_ASSET_INDEX_OBJECTS {
            bail!(
                "Vanilla asset index has {} objects, exceeding the {} object limit",
                document.objects.len(),
                MAX_ASSET_INDEX_OBJECTS
            );
        }
        for (name, object) in &document.objects {
            validate_asset_key(name)
                .with_context(|| format!("invalid Vanilla asset-index key {name:?}"))?;
            validate_sha1(&object.hash, &format!("asset object {name:?} SHA-1"))?;
        }

        Ok(Some(Self {
            index_id: version.asset_index.id,
            objects_dir: assets_dir.join("objects"),
            objects: document.objects,
        }))
    }

    pub(crate) fn resolve(&self, location: &ResourceLocation) -> Result<Option<PathBuf>> {
        let key = format!("{}/{}", location.namespace(), location.path());
        let Some(object) = self.objects.get(&key) else {
            return Ok(None);
        };
        let object_path = self.objects_dir.join(&object.hash[..2]).join(&object.hash);
        verify_file(
            &object_path,
            object.size,
            &object.hash,
            &format!("Vanilla asset {key:?} from index {}", self.index_id),
        )?;
        Ok(Some(object_path))
    }
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
    let file = File::open(path).with_context(|| format!("open {label} {}", path.display()))?;
    let actual_size = file
        .metadata()
        .with_context(|| format!("read {label} metadata {}", path.display()))?
        .len();
    if actual_size != expected_size {
        bail!(
            "{label} {} has size {actual_size}, expected {expected_size}",
            path.display()
        );
    }
    let actual_sha1 = hash_reader(file.take(expected_size + 1), label)?;
    if actual_sha1 != expected_sha1 {
        bail!(
            "{label} {} has SHA-1 {actual_sha1}, expected {expected_sha1}",
            path.display()
        );
    }
    Ok(())
}

fn hash_reader(mut reader: Take<File>, label: &str) -> Result<String> {
    let mut sha1 = Sha1::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .with_context(|| format!("hash {label}"))?;
        if count == 0 {
            break;
        }
        sha1.update(&buffer[..count]);
    }
    Ok(format!("{:x}", sha1.finalize()))
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
        let resolved = catalog.resolve(&location).unwrap().unwrap();
        assert_eq!(std::fs::read(resolved).unwrap(), object);
        assert!(catalog
            .resolve(&ResourceLocation::parse("minecraft:font/missing.zip").unwrap())
            .unwrap()
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
        let error = catalog
            .resolve(&ResourceLocation::parse("minecraft:font/unifont.zip").unwrap())
            .unwrap_err()
            .to_string();
        assert!(error.contains("has size"), "{error}");
        std::fs::write(&path, b"font archivf").unwrap();
        let error = catalog
            .resolve(&ResourceLocation::parse("minecraft:font/unifont.zip").unwrap())
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
        let error = catalog
            .resolve(&ResourceLocation::parse("minecraft:font/unifont.zip").unwrap())
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
        assert!(validate_asset_key("pack.mcmeta").is_ok());
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
