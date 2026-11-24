use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use sha1::{Digest, Sha1};
use zip::ZipArchive;

use crate::{ResourceIntegrity, MC_VERSION};

const MAX_VERSION_METADATA_BYTES: u64 = 4 * 1024 * 1024;
const MAX_NATIVE_ARCHIVE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_NATIVE_LIBRARY_BYTES: u64 = 16 * 1024 * 1024;
static EXTRACT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Deserialize)]
struct VersionMetadata {
    libraries: Vec<LibraryMetadata>,
}

#[derive(Debug, Deserialize)]
struct LibraryMetadata {
    name: String,
    downloads: LibraryDownloads,
}

#[derive(Debug, Deserialize)]
struct LibraryDownloads {
    artifact: LibraryArtifact,
}

#[derive(Debug, Deserialize)]
struct LibraryArtifact {
    path: String,
    sha1: String,
    size: u64,
}

/// Materializes the manifest-owned LWJGL OpenAL library.
pub fn materialize_verified_openal_library(vanilla_root: impl AsRef<Path>) -> Result<PathBuf> {
    materialize_verified_library(vanilla_root.as_ref(), "openal")
}

fn materialize_verified_library(vanilla_root: &Path, component: &str) -> Result<PathBuf> {
    let downloads = vanilla_root.join("downloads").join(MC_VERSION);
    let downloads_meta = downloads.join(format!("{MC_VERSION}.json"));
    let launcher_meta = vanilla_root
        .join("versions")
        .join(MC_VERSION)
        .join(format!("{MC_VERSION}.json"));
    let (metadata_path, libraries_dir) = if downloads_meta.is_file() {
        (downloads_meta, downloads.join("libraries"))
    } else if launcher_meta.is_file() {
        (launcher_meta, vanilla_root.join("libraries"))
    } else {
        (downloads_meta, downloads.join("libraries"))
    };
    let metadata: VersionMetadata = read_json_limited(&metadata_path)?;
    let (classifier, platform, suffix) = platform_native_artifact()?;
    let native_component = if cfg!(target_os = "windows") && component == "openal" {
        "OpenAL"
    } else {
        component
    };
    let library_name = format!(
        "{}{native_component}.{suffix}",
        std::env::consts::DLL_PREFIX
    );
    let archive_entry = format!("{platform}/org/lwjgl/{component}/{library_name}");
    let coordinate = format!("org.lwjgl:lwjgl-{component}:3.4.1:{classifier}");
    let artifact = metadata
        .libraries
        .iter()
        .find(|library| library.name == coordinate)
        .with_context(|| format!("Vanilla {MC_VERSION} manifest lacks {coordinate}"))?;
    validate_relative_artifact_path(&artifact.downloads.artifact.path)?;
    if artifact.downloads.artifact.size > MAX_NATIVE_ARCHIVE_BYTES {
        bail!(
            "Vanilla native archive declares {} bytes, exceeding {MAX_NATIVE_ARCHIVE_BYTES}",
            artifact.downloads.artifact.size
        );
    }
    let archive_path = libraries_dir.join(&artifact.downloads.artifact.path);
    ResourceIntegrity {
        size: artifact.downloads.artifact.size,
        sha1: artifact.downloads.artifact.sha1.clone(),
    }
    .verify(&archive_path, "Vanilla native archive")?;

    let archive_file = File::open(&archive_path)
        .with_context(|| format!("open Vanilla native archive {}", archive_path.display()))?;
    let mut archive = ZipArchive::new(archive_file).context("parse Vanilla native archive")?;
    let mut entry = archive
        .by_name(&archive_entry)
        .with_context(|| format!("Vanilla native archive lacks {archive_entry}"))?;
    if !entry.is_file() || entry.size() > MAX_NATIVE_LIBRARY_BYTES {
        bail!(
            "Vanilla native entry {archive_entry} is not a bounded regular file ({} bytes)",
            entry.size()
        );
    }
    let mut bytes =
        Vec::with_capacity(usize::try_from(entry.size()).context("native entry size overflow")?);
    entry
        .by_ref()
        .take(MAX_NATIVE_LIBRARY_BYTES + 1)
        .read_to_end(&mut bytes)
        .context("read Vanilla native entry")?;
    if bytes.len() as u64 != entry.size() {
        bail!(
            "Vanilla native entry read {} bytes, expected {}",
            bytes.len(),
            entry.size()
        );
    }
    let content_sha1 = format!("{:x}", Sha1::digest(&bytes));
    let cache_root = std::env::temp_dir().join(format!("bbb-{component}-{MC_VERSION}"));
    prepare_private_cache_directory(&cache_root)?;
    let cache_dir = cache_root.join(&content_sha1);
    prepare_private_cache_directory(&cache_dir)?;
    let output = cache_dir.join(&library_name);
    match fs::symlink_metadata(&output) {
        Ok(metadata) if metadata.file_type().is_file() => {
            verify_materialized(&output, &bytes, &content_sha1)?;
            return Ok(output);
        }
        Ok(_) => bail!(
            "materialized native path {} is not a regular file",
            output.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("inspect materialized native library"),
    }

    let sequence = EXTRACT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = cache_dir.join(format!(
        ".{library_name}.{}.{}.tmp",
        std::process::id(),
        sequence
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o400);
    let mut file = options
        .open(&temporary)
        .with_context(|| format!("create native temporary {}", temporary.display()))?;
    file.write_all(&bytes).context("write native temporary")?;
    file.sync_all().context("sync native temporary")?;
    drop(file);
    match fs::hard_link(&temporary, &output) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            return Err(error).context("publish content-addressed native library");
        }
    }
    fs::remove_file(&temporary)
        .with_context(|| format!("remove native temporary {}", temporary.display()))?;
    verify_materialized(&output, &bytes, &content_sha1)?;
    Ok(output)
}

fn prepare_private_cache_directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("create native cache {}", path.display()))?;
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("inspect native cache {}", path.display()))?;
    if !metadata.file_type().is_dir() {
        bail!("native cache {} is not a real directory", path.display());
    }
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .with_context(|| format!("restrict native cache {}", path.display()))?;
    Ok(())
}

fn read_json_limited(path: &Path) -> Result<VersionMetadata> {
    let file = File::open(path)
        .with_context(|| format!("open Vanilla version metadata {}", path.display()))?;
    let size = file.metadata()?.len();
    if size > MAX_VERSION_METADATA_BYTES {
        bail!("Vanilla version metadata is {size} bytes, exceeding {MAX_VERSION_METADATA_BYTES}");
    }
    serde_json::from_reader(file.take(MAX_VERSION_METADATA_BYTES + 1))
        .with_context(|| format!("parse Vanilla version metadata {}", path.display()))
}

fn validate_relative_artifact_path(path: &str) -> Result<()> {
    let path = Path::new(path);
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("invalid Vanilla library artifact path {}", path.display());
    }
    Ok(())
}

fn verify_materialized(path: &Path, expected: &[u8], expected_sha1: &str) -> Result<()> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        bail!(
            "materialized native library {} is not a regular file",
            path.display()
        );
    }
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o400))
        .with_context(|| format!("restrict materialized native library {}", path.display()))?;
    let actual = fs::read(path)
        .with_context(|| format!("read materialized native library {}", path.display()))?;
    if actual.len() != expected.len() || format!("{:x}", Sha1::digest(&actual)) != expected_sha1 {
        bail!(
            "materialized native library {} does not match the verified archive entry",
            path.display()
        );
    }
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn platform_native_artifact() -> Result<(&'static str, &'static str, &'static str)> {
    Ok(("natives-linux", "linux/x64", "so"))
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn platform_native_artifact() -> Result<(&'static str, &'static str, &'static str)> {
    Ok(("natives-macos-arm64", "macos/arm64", "dylib"))
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
fn platform_native_artifact() -> Result<(&'static str, &'static str, &'static str)> {
    Ok(("natives-macos", "macos/x64", "dylib"))
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
fn platform_native_artifact() -> Result<(&'static str, &'static str, &'static str)> {
    Ok(("natives-windows", "windows/x64", "dll"))
}

#[cfg(not(any(
    all(target_os = "windows", target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "macos", target_arch = "x86_64")
)))]
fn platform_native_artifact() -> Result<(&'static str, &'static str, &'static str)> {
    bail!(
        "verified Vanilla native extraction supports Windows/Linux x86_64 and macOS aarch64/x86_64"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_paths_reject_absolute_and_parent_components() {
        assert!(validate_relative_artifact_path("org/lwjgl/native.jar").is_ok());
        assert!(validate_relative_artifact_path("/native.jar").is_err());
        assert!(validate_relative_artifact_path("org/../native.jar").is_err());
        assert!(validate_relative_artifact_path("").is_err());
    }

    #[test]
    fn platform_native_artifact_matches_the_native_library_family() {
        let (classifier, platform, suffix) = platform_native_artifact().unwrap();
        #[cfg(target_os = "windows")]
        {
            assert_eq!(classifier, "natives-windows");
            assert_eq!(platform, "windows/x64");
            assert_eq!(suffix, "dll");
        }
        #[cfg(target_os = "linux")]
        {
            assert_eq!(classifier, "natives-linux");
            assert_eq!(platform, "linux/x64");
            assert_eq!(suffix, "so");
        }
        #[cfg(target_os = "macos")]
        {
            assert!(classifier.starts_with("natives-macos"));
            assert!(platform.starts_with("macos/"));
            assert_eq!(suffix, "dylib");
        }
    }

    #[test]
    #[ignore = "requires BBB_VANILLA_ROOT from scripts/vanilla path 26.1 --kind root"]
    fn verified_workspace_materializes_manifest_owned_openal() {
        let root = std::env::var_os("BBB_VANILLA_ROOT")
            .expect("BBB_VANILLA_ROOT must name the verified Vanilla workspace");
        let path = materialize_verified_openal_library(root).unwrap();
        assert!(path.is_file());
        assert_eq!(
            path.file_name().unwrap(),
            if cfg!(target_os = "windows") {
                "OpenAL.dll"
            } else if cfg!(target_os = "macos") {
                "libopenal.dylib"
            } else {
                "libopenal.so"
            }
        );
    }
}
