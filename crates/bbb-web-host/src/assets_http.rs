//! Same-origin static WASM asset delivery, `core-26.1.zip` builder/cache, and lazy `.ogg` serving.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs::{self, File},
    io::{BufReader, Cursor, Read, Write},
    path::{Component, Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::SystemTime,
};

use anyhow::{bail, Context, Result};
use bbb_protocol::{registry_data_types::VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES, MC_VERSION};
use bytes::Bytes;
use flate2::{write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

use crate::http::{HttpRequest, HttpResponse};

const EMBEDDED_INDEX_HTML: &str = include_str!("../web/index.html");
const BUNDLED_LANG_KEYS: &[&str] = &[
    "minecraft/lang/en_us.json",
    "minecraft/lang/zh_cn.json",
    "minecraft/lang/zh_tw.json",
    "minecraft/lang/zh_hk.json",
];

#[derive(Debug, Deserialize)]
struct VersionManifest {
    #[serde(rename = "assetIndex")]
    asset_index: AssetIndexDescriptor,
}

#[derive(Debug, Deserialize)]
struct AssetIndexDescriptor {
    id: String,
    #[serde(default)]
    sha1: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AssetIndexDocument {
    objects: BTreeMap<String, AssetObjectRef>,
}

#[derive(Debug, Deserialize)]
struct AssetObjectRef {
    hash: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BootstrapConfigJson {
    pub server: String,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_distance: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_resource_pack_policy: Option<String>,
    pub control_enabled: bool,
    pub core_bundle_etag: String,
    pub asset_index_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}

#[derive(Clone)]
struct CachedPkgAsset {
    modified: Option<SystemTime>,
    len: u64,
    etag: String,
    gzip_etag: Option<String>,
    raw: Bytes,
    gzip: Option<Bytes>,
}

#[derive(Clone)]
pub struct PreparedWebAssets {
    wasm_dir: PathBuf,
    objects_dir: PathBuf,
    asset_index_id: String,
    asset_index_bytes: Arc<[u8]>,
    core_bundle_bytes: Arc<[u8]>,
    core_bundle_etag: String,
    config_json: Arc<[u8]>,
    pkg_cache: Arc<Mutex<HashMap<PathBuf, CachedPkgAsset>>>,
}

pub struct AssetBuildOptions {
    pub vanilla_root: PathBuf,
    pub wasm_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub default_server: String,
    pub default_username: String,
    pub default_locale: Option<String>,
    pub default_uuid: Option<String>,
    pub default_render_distance: Option<u32>,
    pub server_resource_pack_policy: Option<String>,
    pub control_enabled: bool,
}

enum VanillaBundleInput {
    ExtractedDir(PathBuf),
    ClientJar(PathBuf),
}

struct ResolvedVanillaPaths {
    input: VanillaBundleInput,
    manifest_path: PathBuf,
    assets_dir: PathBuf,
}

pub fn has_vanilla_bundle_input(vanilla_root: &Path) -> bool {
    resolve_vanilla_paths(vanilla_root).is_ok()
}

fn resolve_vanilla_paths(vanilla_root: &Path) -> Result<ResolvedVanillaPaths> {
    let sources_dir = vanilla_root.join("sources").join(MC_VERSION);
    let downloads_dir = vanilla_root.join("downloads").join(MC_VERSION);
    let downloads_manifest = downloads_dir.join(format!("{MC_VERSION}.json"));
    if sources_dir.is_dir() && downloads_manifest.is_file() {
        return Ok(ResolvedVanillaPaths {
            input: VanillaBundleInput::ExtractedDir(sources_dir),
            manifest_path: downloads_manifest,
            assets_dir: downloads_dir.join("assets"),
        });
    }

    for jar_name in [
        format!("client-{MC_VERSION}.jar"),
        "client.jar".to_string(),
        format!("{MC_VERSION}.jar"),
    ] {
        let jar_path = downloads_dir.join(&jar_name);
        if jar_path.is_file() && downloads_manifest.is_file() {
            return Ok(ResolvedVanillaPaths {
                input: VanillaBundleInput::ClientJar(jar_path),
                manifest_path: downloads_manifest,
                assets_dir: downloads_dir.join("assets"),
            });
        }
    }

    let launcher_version_dir = vanilla_root.join("versions").join(MC_VERSION);
    let launcher_manifest = launcher_version_dir.join(format!("{MC_VERSION}.json"));
    for jar_name in [format!("{MC_VERSION}.jar"), "client.jar".to_string()] {
        let jar_path = launcher_version_dir.join(&jar_name);
        if jar_path.is_file() && launcher_manifest.is_file() {
            return Ok(ResolvedVanillaPaths {
                input: VanillaBundleInput::ClientJar(jar_path),
                manifest_path: launcher_manifest,
                assets_dir: vanilla_root.join("assets"),
            });
        }
    }

    bail!(
        "missing Vanilla {MC_VERSION} sources or client jar under {} (expected `sources/{MC_VERSION}` + `downloads/{MC_VERSION}`, `downloads/{MC_VERSION}/client-{MC_VERSION}.jar`, or `.minecraft/versions/{MC_VERSION}/{MC_VERSION}.jar`)",
        vanilla_root.display()
    );
}

impl PreparedWebAssets {
    pub fn build_or_load(opts: AssetBuildOptions) -> Result<Self> {
        let policy = opts
            .server_resource_pack_policy
            .map(|s| s.trim().to_ascii_lowercase());
        if let Some(value) = &policy {
            if !matches!(value.as_str(), "prompt" | "accept" | "decline") {
                bail!("invalid server_resource_pack_policy '{value}' (expected 'prompt', 'accept' or 'decline')");
            }
        }

        let resolved = resolve_vanilla_paths(&opts.vanilla_root)?;
        let objects_dir = resolved.assets_dir.join("objects");
        let manifest_bytes = fs::read(&resolved.manifest_path).with_context(|| {
            format!("read Vanilla manifest {}", resolved.manifest_path.display())
        })?;
        let manifest: VersionManifest =
            serde_json::from_slice(&manifest_bytes).with_context(|| {
                format!(
                    "parse Vanilla manifest {}",
                    resolved.manifest_path.display()
                )
            })?;
        let asset_index_id = manifest.asset_index.id;
        let index_path = resolved
            .assets_dir
            .join("indexes")
            .join(format!("{asset_index_id}.json"));
        let asset_index_bytes: Arc<[u8]> = fs::read(&index_path)
            .with_context(|| format!("read Vanilla asset index {}", index_path.display()))?
            .into();

        fs::create_dir_all(&opts.cache_dir)
            .with_context(|| format!("create cache dir {}", opts.cache_dir.display()))?;

        let cache_fingerprint = compute_bundle_input_fingerprint(
            &resolved.input,
            &manifest_bytes,
            &asset_index_bytes,
            manifest.asset_index.sha1.as_deref(),
        )?;
        let cached_zip_path = opts
            .cache_dir
            .join(format!("core-{MC_VERSION}-{cache_fingerprint}.zip"));

        let core_bundle_bytes: Arc<[u8]> = if cached_zip_path.is_file() {
            fs::read(&cached_zip_path)
                .with_context(|| format!("read cached core bundle {}", cached_zip_path.display()))?
                .into()
        } else {
            tracing::info!(
                cache_path = %cached_zip_path.display(),
                "assembling core-{MC_VERSION}.zip for first run (subsequent starts reuse disk cache)"
            );
            let built = build_core_bundle_zip(&resolved.input, &objects_dir, &asset_index_bytes)
                .context("assemble core-26.1.zip")?;
            static NEXT_TMP_ID: AtomicU64 = AtomicU64::new(1);
            let tmp_path = opts.cache_dir.join(format!(
                "core-{MC_VERSION}-{}-{}.tmp",
                std::process::id(),
                NEXT_TMP_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::write(&tmp_path, &built)
                .with_context(|| format!("write temporary core bundle {}", tmp_path.display()))?;
            if let Err(err) = fs::rename(&tmp_path, &cached_zip_path) {
                let _ = fs::remove_file(&tmp_path);
                // On Windows `fs::rename` returns `AlreadyExists` / `PermissionDenied`
                // when another process installed the same fingerprinted bundle first.
                if !cached_zip_path.is_file() {
                    return Err(err).with_context(|| {
                        format!(
                            "install cached core bundle {} -> {}",
                            tmp_path.display(),
                            cached_zip_path.display()
                        )
                    });
                }
            }
            built.into()
        };

        let mut sha1 = Sha1::new();
        sha1.update(&core_bundle_bytes);
        let core_bundle_etag = format!("\"{}\"", hex_lower(&sha1.finalize()));

        let lang = opts
            .default_locale
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_ascii_lowercase);
        let config = BootstrapConfigJson {
            server: opts.default_server,
            username: opts.default_username,
            uuid: opts.default_uuid,
            render_distance: opts.default_render_distance,
            server_resource_pack_policy: policy,
            control_enabled: opts.control_enabled,
            core_bundle_etag: core_bundle_etag.clone(),
            asset_index_id: asset_index_id.clone(),
            lang,
        };
        let config_json: Arc<[u8]> = serde_json::to_vec(&config)
            .context("serialize bootstrap /config.json")?
            .into();

        Ok(Self {
            wasm_dir: opts.wasm_dir,
            objects_dir,
            asset_index_id,
            asset_index_bytes,
            core_bundle_bytes,
            core_bundle_etag,
            config_json,
            pkg_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn asset_index_id(&self) -> &str {
        &self.asset_index_id
    }

    pub fn core_bundle_bytes(&self) -> &[u8] {
        &self.core_bundle_bytes
    }

    pub fn core_bundle_etag(&self) -> &str {
        &self.core_bundle_etag
    }

    pub fn serve(&self, req: &HttpRequest) -> HttpResponse {
        if let Some(resp) = self.serve_in_memory(req) {
            return resp;
        }
        match req.path.as_str() {
            "/" | "/index.html" => serve_index_html_from(&self.wasm_dir),
            path if path.starts_with("/assets/objects/") => {
                match resolve_asset_object_path(&self.objects_dir, path) {
                    Ok(disk_path) => read_asset_object(&disk_path),
                    Err(resp) => resp,
                }
            }
            path if path.starts_with("/pkg/") => {
                let rel = &path[5..];
                let Some(clean_rel) = sanitize_relative_path(rel) else {
                    return HttpResponse::text(400, "Bad Request", "Invalid /pkg path");
                };
                serve_wasm_pkg_file_sync(
                    &self.wasm_dir,
                    &self.pkg_cache,
                    rel,
                    &clean_rel,
                    req.header("if-none-match"),
                    accepts_gzip(req),
                )
            }
            _ => HttpResponse::text(404, "Not Found", format!("Route not found: {}", req.path)),
        }
    }

    /// Async entry point used by the HTTP connection handler so disk I/O and
    /// on-demand WASM gzip compression run on Tokio's blocking pool rather than
    /// stalling async runtime worker threads.
    pub async fn serve_async(&self, req: &HttpRequest) -> HttpResponse {
        if let Some(resp) = self.serve_in_memory(req) {
            return resp;
        }
        match req.path.as_str() {
            "/" | "/index.html" => {
                let wasm_dir = self.wasm_dir.clone();
                run_blocking_asset_task(move || serve_index_html_from(&wasm_dir)).await
            }
            path if path.starts_with("/assets/objects/") => {
                match resolve_asset_object_path(&self.objects_dir, path) {
                    Ok(disk_path) => {
                        run_blocking_asset_task(move || read_asset_object(&disk_path)).await
                    }
                    Err(resp) => resp,
                }
            }
            path if path.starts_with("/pkg/") => {
                let rel = &path[5..];
                let Some(clean_rel) = sanitize_relative_path(rel) else {
                    return HttpResponse::text(400, "Bad Request", "Invalid /pkg path");
                };
                let wasm_dir = self.wasm_dir.clone();
                let pkg_cache = Arc::clone(&self.pkg_cache);
                let rel = rel.to_owned();
                let if_none_match = req.header("if-none-match").map(ToOwned::to_owned);
                let client_accepts_gzip = accepts_gzip(req);
                run_blocking_asset_task(move || {
                    serve_wasm_pkg_file_sync(
                        &wasm_dir,
                        &pkg_cache,
                        &rel,
                        &clean_rel,
                        if_none_match.as_deref(),
                        client_accepts_gzip,
                    )
                })
                .await
            }
            _ => HttpResponse::text(404, "Not Found", format!("Route not found: {}", req.path)),
        }
    }

    fn serve_in_memory(&self, req: &HttpRequest) -> Option<HttpResponse> {
        match req.path.as_str() {
            "/favicon.ico" => Some(
                HttpResponse::ok("image/x-icon", Bytes::new())
                    .with_header("Cache-Control", "public, max-age=86400"),
            ),
            "/config.json" => Some(
                HttpResponse::ok(
                    "application/json; charset=utf-8",
                    Bytes::from_owner(Arc::clone(&self.config_json)),
                )
                .with_header("Cache-Control", "no-store"),
            ),
            "/assets/core-26.1.zip" => {
                if req
                    .header("if-none-match")
                    .is_some_and(|val| etag_matches(val, &self.core_bundle_etag))
                {
                    return Some(HttpResponse::not_modified(&self.core_bundle_etag));
                }
                Some(
                    HttpResponse::ok(
                        "application/zip",
                        Bytes::from_owner(Arc::clone(&self.core_bundle_bytes)),
                    )
                    .with_header("ETag", self.core_bundle_etag.clone())
                    .with_header("Cache-Control", "public, max-age=0, must-revalidate"),
                )
            }
            "/assets/asset-index.json" => Some(
                HttpResponse::ok(
                    "application/json; charset=utf-8",
                    Bytes::from_owner(Arc::clone(&self.asset_index_bytes)),
                )
                .with_header("Cache-Control", "public, max-age=3600"),
            ),
            _ => None,
        }
    }
}

async fn run_blocking_asset_task(
    f: impl FnOnce() -> HttpResponse + Send + 'static,
) -> HttpResponse {
    match tokio::task::spawn_blocking(f).await {
        Ok(resp) => resp,
        Err(err) => {
            tracing::error!(?err, "blocking asset task failed");
            HttpResponse::text(500, "Internal Server Error", "Asset task failed")
        }
    }
}

fn serve_index_html_from(wasm_dir: &Path) -> HttpResponse {
    let custom_index = wasm_dir.join("index.html");
    let body = match fs::read(&custom_index) {
        Ok(bytes) => Bytes::from(bytes),
        Err(_) => Bytes::from_static(EMBEDDED_INDEX_HTML.as_bytes()),
    };
    HttpResponse::ok("text/html; charset=utf-8", body).with_header("Cache-Control", "no-store")
}

fn serve_wasm_pkg_file_sync(
    wasm_dir: &Path,
    pkg_cache: &Mutex<HashMap<PathBuf, CachedPkgAsset>>,
    rel: &str,
    clean_rel: &Path,
    if_none_match: Option<&str>,
    client_accepts_gzip: bool,
) -> HttpResponse {
    let candidate_in_pkg = wasm_dir.join("pkg").join(clean_rel);
    let candidate_direct = wasm_dir.join(clean_rel);
    let (target, meta) = if let Ok(m) = fs::metadata(&candidate_in_pkg).and_then(|m| {
        m.is_file()
            .then_some(m)
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))
    }) {
        (candidate_in_pkg, m)
    } else if let Ok(m) = fs::metadata(&candidate_direct).and_then(|m| {
        m.is_file()
            .then_some(m)
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))
    }) {
        (candidate_direct, m)
    } else {
        return HttpResponse::text(
            404,
            "Not Found",
            format!(
                "WASM artifact '/pkg/{rel}' not found under {} (run `cargo build-web` or pass `--wasm-dir`)",
                wasm_dir.display()
            ),
        );
    };

    let ct = content_type_for_path(&target);
    let modified = meta.modified().ok();
    let file_len = meta.len();

    let cached = {
        let guard = pkg_cache.lock().unwrap_or_else(|err| err.into_inner());
        guard
            .get(&target)
            .filter(|entry| entry.len == file_len && entry.modified == modified)
            .cloned()
    };

    let entry = match cached {
        Some(hit) => hit,
        None => {
            let Ok(vec_bytes) = fs::read(&target) else {
                return HttpResponse::text(404, "Not Found", "Failed to read WASM artifact");
            };
            let mut sha1 = Sha1::new();
            sha1.update(&vec_bytes);
            let hash = hex_lower(&sha1.finalize());
            let etag = format!("\"{hash}\"");
            let gzip = if should_gzip_content_type(ct) {
                gzip_compress_bytes(&vec_bytes)
            } else {
                None
            };
            let gzip_etag = gzip.as_ref().map(|_| format!("\"{hash}-gzip\""));
            let built = CachedPkgAsset {
                modified,
                len: vec_bytes.len() as u64,
                etag,
                gzip_etag,
                raw: Bytes::from(vec_bytes),
                gzip,
            };
            let mut guard = pkg_cache.lock().unwrap_or_else(|err| err.into_inner());
            guard.insert(target, built.clone());
            built
        }
    };

    if client_accepts_gzip {
        if let (Some(gz_body), Some(gz_etag)) = (entry.gzip, entry.gzip_etag) {
            if if_none_match.is_some_and(|val| etag_matches(val, &gz_etag)) {
                return HttpResponse::not_modified(&gz_etag).with_header("Vary", "Accept-Encoding");
            }
            return HttpResponse::ok(ct, gz_body)
                .with_header("Content-Encoding", "gzip")
                .with_header("ETag", gz_etag)
                .with_header("Vary", "Accept-Encoding")
                .with_header("Cache-Control", "public, max-age=0, must-revalidate");
        }
    }

    if if_none_match.is_some_and(|val| etag_matches(val, &entry.etag)) {
        return HttpResponse::not_modified(&entry.etag).with_header("Vary", "Accept-Encoding");
    }

    HttpResponse::ok(ct, entry.raw)
        .with_header("ETag", entry.etag)
        .with_header("Vary", "Accept-Encoding")
        .with_header("Cache-Control", "public, max-age=0, must-revalidate")
}

fn resolve_asset_object_path(objects_dir: &Path, path: &str) -> Result<PathBuf, HttpResponse> {
    let rest = &path["/assets/objects/".len()..];
    let Some((prefix, hash)) = rest.split_once('/') else {
        return Err(HttpResponse::text(
            400,
            "Bad Request",
            "Malformed object path",
        ));
    };
    if !is_lower_hex_len(prefix, 2) || !is_lower_hex_len(hash, 40) || !hash.starts_with(prefix) {
        return Err(HttpResponse::text(
            400,
            "Bad Request",
            "Invalid SHA-1 object path",
        ));
    }
    Ok(objects_dir.join(prefix).join(hash))
}

fn read_asset_object(disk_path: &Path) -> HttpResponse {
    match fs::read(disk_path) {
        Ok(bytes) => HttpResponse::ok("application/octet-stream", bytes)
            .with_header("Cache-Control", "public, max-age=31536000, immutable"),
        Err(_) => HttpResponse::text(404, "Not Found", "Asset object not found"),
    }
}

fn accepts_gzip(req: &HttpRequest) -> bool {
    for value in req.headers.values("accept-encoding") {
        for part in value.split(',') {
            let mut segments = part.split(';').map(str::trim);
            let Some(coding) = segments.next() else {
                continue;
            };
            if coding.eq_ignore_ascii_case("gzip") {
                let disabled = segments.any(|param| {
                    param
                        .strip_prefix("q=")
                        .or_else(|| param.strip_prefix("Q="))
                        .and_then(|q| q.trim().parse::<f32>().ok())
                        .is_some_and(|q| q <= 0.0)
                });
                if !disabled {
                    return true;
                }
            }
        }
    }
    false
}

fn should_gzip_content_type(ct: &str) -> bool {
    ct.starts_with("application/wasm")
        || ct.starts_with("text/")
        || ct.starts_with("application/json")
}

fn gzip_compress_bytes(raw: &[u8]) -> Option<Bytes> {
    if raw.is_empty() {
        return None;
    }
    let mut encoder = GzEncoder::new(
        Vec::with_capacity(raw.len() / 2 + 64),
        Compression::default(),
    );
    encoder.write_all(raw).ok()?;
    let gz = encoder.finish().ok()?;
    Some(Bytes::from(gz))
}

fn etag_matches(header_val: &str, expected_etag: &str) -> bool {
    header_val
        .split(',')
        .map(str::trim)
        .any(|candidate| candidate == "*" || candidate.trim_start_matches("W/") == expected_etag)
}

fn is_lower_hex_len(s: &str, len: usize) -> bool {
    s.len() == len
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn sanitize_relative_path(raw: &str) -> Option<PathBuf> {
    if raw.is_empty() || raw.contains('\\') {
        return None;
    }
    let mut out = PathBuf::new();
    for comp in Path::new(raw).components() {
        match comp {
            Component::Normal(seg) => out.push(seg),
            _ => return None,
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

fn content_type_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("html") => "text/html; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("png") => "image/png",
        _ => "application/octet-stream",
    }
}

fn compute_bundle_input_fingerprint(
    input: &VanillaBundleInput,
    manifest_bytes: &[u8],
    asset_index_bytes: &[u8],
    index_sha1: Option<&str>,
) -> Result<String> {
    let mut hasher = Sha1::new();
    hasher.update(b"bbb-core-web-bundle-v3:");
    hasher.update(manifest_bytes);
    hasher.update(asset_index_bytes);
    if let Some(sha1) = index_sha1 {
        hasher.update(sha1.as_bytes());
    }
    match input {
        VanillaBundleInput::ExtractedDir(sources_dir) => {
            let version_path = sources_dir.join("version.json");
            if let Ok(version_bytes) = fs::read(&version_path) {
                hasher.update(&version_bytes);
            }
        }
        VanillaBundleInput::ClientJar(jar_path) => {
            let file = File::open(jar_path)
                .with_context(|| format!("open client jar {}", jar_path.display()))?;
            let mut archive = ZipArchive::new(BufReader::new(file))
                .with_context(|| format!("read client jar {}", jar_path.display()))?;
            if let Ok(mut entry) = archive.by_name("version.json") {
                let mut version_bytes = Vec::new();
                entry
                    .read_to_end(&mut version_bytes)
                    .context("read version.json from client jar")?;
                hasher.update(&version_bytes);
            };
        }
    }
    Ok(hex_lower(&hasher.finalize())[..16].to_string())
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn build_core_bundle_zip(
    input: &VanillaBundleInput,
    objects_dir: &Path,
    asset_index_bytes: &[u8],
) -> Result<Vec<u8>> {
    let index_doc: AssetIndexDocument =
        serde_json::from_slice(asset_index_bytes).context("parse asset index JSON")?;

    let mut jar_archive = match input {
        VanillaBundleInput::ExtractedDir(_) => None,
        VanillaBundleInput::ClientJar(jar_path) => {
            let file = File::open(jar_path)
                .with_context(|| format!("open client jar {}", jar_path.display()))?;
            Some(
                ZipArchive::new(BufReader::new(file))
                    .with_context(|| format!("read client jar {}", jar_path.display()))?,
            )
        }
    };

    let source_pack_mcmeta = match (input, jar_archive.as_mut()) {
        (VanillaBundleInput::ExtractedDir(sources_dir), _) => {
            fs::read(sources_dir.join("pack.mcmeta")).ok()
        }
        (_, Some(archive)) => archive.by_name("pack.mcmeta").ok().and_then(|mut entry| {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).ok().map(|_| buf)
        }),
        _ => None,
    };

    let mut written_entries = BTreeSet::<String>::new();
    let mut cursor = Cursor::new(Vec::with_capacity(24 * 1024 * 1024));
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let deflate_opts = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);
        let stored_opts = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .unix_permissions(0o644);

        let pack_mcmeta_bytes =
            synthesize_pack_mcmeta(source_pack_mcmeta.as_deref(), objects_dir, &index_doc)?;
        write_zip_entry(
            &mut zip,
            &mut written_entries,
            "pack.mcmeta",
            &pack_mcmeta_bytes,
            deflate_opts,
        )?;

        // External asset index entries override `client.jar` stubs (such as empty
        // `assets/minecraft/font/include/unifont.json`), matching `PackEntry::resource_file`.
        for (key, obj) in &index_doc.objects {
            if !should_bundle_indexed_object(key) {
                continue;
            }
            let zip_entry_name = indexed_key_to_zip_entry(key);
            if written_entries.contains(&zip_entry_name) {
                continue;
            }
            if obj.hash.len() < 2 {
                continue;
            }
            let obj_path = objects_dir.join(&obj.hash[..2]).join(&obj.hash);
            if !obj_path.is_file() {
                continue;
            }
            let bytes = fs::read(&obj_path)
                .with_context(|| format!("read indexed asset {} ({})", key, obj_path.display()))?;
            let opts = if is_already_compressed(&zip_entry_name) {
                stored_opts
            } else {
                deflate_opts
            };
            write_zip_entry(
                &mut zip,
                &mut written_entries,
                &zip_entry_name,
                &bytes,
                opts,
            )?;
        }

        match (input, jar_archive.as_mut()) {
            (VanillaBundleInput::ExtractedDir(sources_dir), _) => {
                for root_file in ["version.json", "pack.png"] {
                    if written_entries.contains(root_file) {
                        continue;
                    }
                    let candidate = sources_dir.join(root_file);
                    if candidate.is_file() {
                        let bytes = fs::read(&candidate)
                            .with_context(|| format!("read {}", candidate.display()))?;
                        let opts = if root_file.ends_with(".png") {
                            stored_opts
                        } else {
                            deflate_opts
                        };
                        write_zip_entry(&mut zip, &mut written_entries, root_file, &bytes, opts)?;
                    }
                }

                for tree in ["assets", "data"] {
                    let root = sources_dir.join(tree);
                    if root.is_dir() {
                        append_directory_recursive(
                            &mut zip,
                            &mut written_entries,
                            sources_dir,
                            &root,
                            deflate_opts,
                            stored_opts,
                        )?;
                    }
                }
            }
            (_, Some(archive)) => {
                let mut jar_names = Vec::new();
                for name in archive.file_names() {
                    let name = name.context("decode jar entry name")?;
                    if !name.ends_with('/') && should_bundle_source_entry(&name) {
                        jar_names.push(name.into_owned());
                    }
                }
                jar_names.sort();
                jar_names.dedup();
                for name in jar_names {
                    if written_entries.contains(&name) {
                        continue;
                    }
                    let mut entry = archive
                        .by_name(&name)
                        .with_context(|| format!("read jar entry {name}"))?;
                    let mut bytes = Vec::with_capacity(entry.size() as usize);
                    entry
                        .read_to_end(&mut bytes)
                        .with_context(|| format!("decode jar entry {name}"))?;
                    let opts = if is_already_compressed(&name) {
                        stored_opts
                    } else {
                        deflate_opts
                    };
                    write_zip_entry(&mut zip, &mut written_entries, &name, &bytes, opts)?;
                }
            }
            _ => unreachable!(),
        }

        zip.finish().context("finalize core-26.1.zip")?;
    }
    Ok(cursor.into_inner())
}

fn synthesize_pack_mcmeta(
    source_pack_mcmeta: Option<&[u8]>,
    objects_dir: &Path,
    index_doc: &AssetIndexDocument,
) -> Result<Vec<u8>> {
    let mut doc = serde_json::Map::new();

    if let Some(bytes) = source_pack_mcmeta {
        if let Ok(serde_json::Value::Object(existing)) = serde_json::from_slice(bytes) {
            doc.extend(existing);
        }
    }
    if let Some(obj) = index_doc.objects.get("pack.mcmeta") {
        if obj.hash.len() >= 2 {
            let obj_path = objects_dir.join(&obj.hash[..2]).join(&obj.hash);
            if let Ok(bytes) = fs::read(&obj_path) {
                if let Ok(serde_json::Value::Object(existing)) = serde_json::from_slice(&bytes) {
                    doc.extend(existing);
                }
            }
        }
    }

    if !doc.contains_key("pack") {
        let major = bbb_protocol::MC_RESOURCE_PACK_FORMAT.major;
        let minor = bbb_protocol::MC_RESOURCE_PACK_FORMAT.minor;
        doc.insert(
            "pack".to_string(),
            serde_json::json!({
                "description": format!("Default Minecraft {MC_VERSION} Assets"),
                "min_format": [major, minor],
                "max_format": [major, minor],
            }),
        );
    }

    serde_json::to_vec(&serde_json::Value::Object(doc)).context("encode synthesized pack.mcmeta")
}

fn is_omitted_builtin_resourcepack(rel_after_minecraft: &str) -> bool {
    let Some(rest) = rel_after_minecraft.strip_prefix("resourcepacks/") else {
        return false;
    };
    rest == "programmer_art.zip"
        || rest == "high_contrast.zip"
        || rest.starts_with("programmer_art/")
        || rest.starts_with("high_contrast/")
}

fn is_synchronized_registry_json(rest: &str) -> bool {
    if !rest.ends_with(".json") {
        return false;
    }
    VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES.iter().any(|reg| {
        reg.resource_id
            .split_once(':')
            .is_some_and(|(_, reg_path)| {
                rest.starts_with(reg_path) && rest.as_bytes().get(reg_path.len()) == Some(&b'/')
            })
    })
}

fn should_bundle_data_entry(after_data: &str) -> bool {
    let Some((namespace, rest)) = after_data.split_once('/') else {
        return false;
    };
    if is_synchronized_registry_json(rest) {
        return true;
    }
    // Client startup catalogs (item models, furnace fuels, freeze-immune wearables) load tag JSONs from PackRoots.
    if rest.starts_with("tags/") && rest.ends_with(".json") {
        return true;
    }
    if namespace == "minecraft" {
        if let Some(dp_rest) = rest.strip_prefix("datapacks/") {
            if let Some((pack_id, tail)) = dp_rest.split_once('/') {
                if !pack_id.is_empty() {
                    if tail == "pack.mcmeta" {
                        return true;
                    }
                    if let Some(dp_data) = tail.strip_prefix("data/") {
                        if let Some((_, elem_rest)) = dp_data.split_once('/') {
                            return is_synchronized_registry_json(elem_rest);
                        }
                    }
                }
            }
        }
    }
    false
}

fn should_bundle_source_entry(zip_name: &str) -> bool {
    if zip_name == "version.json" || zip_name == "pack.png" {
        return true;
    }
    if let Some(after_assets) = zip_name.strip_prefix("assets/") {
        if let Some(mc_rest) = after_assets.strip_prefix("minecraft/") {
            if is_omitted_builtin_resourcepack(mc_rest) {
                return false;
            }
        }
        return true;
    }
    if let Some(after_data) = zip_name.strip_prefix("data/") {
        return should_bundle_data_entry(after_data);
    }
    false
}

fn should_traverse_source_dir(rel_dir: &str) -> bool {
    if rel_dir == "assets" || rel_dir.starts_with("assets/") {
        if let Some(mc_rest) = rel_dir.strip_prefix("assets/minecraft/") {
            return !is_omitted_builtin_resourcepack(mc_rest)
                && mc_rest != "resourcepacks/programmer_art"
                && mc_rest != "resourcepacks/high_contrast";
        }
        return true;
    }
    let Some(after_data) = rel_dir.strip_prefix("data") else {
        return false;
    };
    let after_data = after_data.strip_prefix('/').unwrap_or(after_data);
    if after_data.is_empty() {
        return true;
    }
    let Some((namespace, rest)) = after_data.split_once('/') else {
        return true;
    };
    if rest == "tags" || rest.starts_with("tags/") {
        return true;
    }
    if namespace == "minecraft" && (rest == "datapacks" || rest.starts_with("datapacks/")) {
        return true;
    }
    VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES.iter().any(|reg| {
        reg.resource_id
            .split_once(':')
            .is_some_and(|(_, reg_path)| {
                reg_path == rest
                    || (reg_path.starts_with(rest)
                        && reg_path.as_bytes().get(rest.len()) == Some(&b'/'))
                    || (rest.starts_with(reg_path)
                        && rest.as_bytes().get(reg_path.len()) == Some(&b'/'))
            })
    })
}

fn should_bundle_indexed_object(key: &str) -> bool {
    if key.ends_with(".ogg") || key == "pack.mcmeta" {
        return false;
    }
    if let Some(mc_rest) = key.strip_prefix("minecraft/") {
        if is_omitted_builtin_resourcepack(mc_rest) {
            return false;
        }
    }
    if key == "pack.png"
        || key == "minecraft/sounds.json"
        || key.starts_with("icons/")
        || key.starts_with("minecraft/font/")
        || key.starts_with("minecraft/textures/")
        || key.starts_with("minecraft/resourcepacks/")
    {
        return true;
    }
    BUNDLED_LANG_KEYS.contains(&key)
}

fn indexed_key_to_zip_entry(key: &str) -> String {
    if key == "pack.png" || key.starts_with("icons/") {
        key.to_string()
    } else {
        format!("assets/{key}")
    }
}

fn is_already_compressed(name: &str) -> bool {
    name.ends_with(".png") || name.ends_with(".zip") || name.ends_with(".ogg")
}

fn append_directory_recursive<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    written_entries: &mut BTreeSet<String>,
    base_dir: &Path,
    current_dir: &Path,
    deflate_opts: SimpleFileOptions,
    stored_opts: SimpleFileOptions,
) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(current_dir)
        .with_context(|| format!("read directory {}", current_dir.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("iterate directory {}", current_dir.display()))?;
    entries.sort_by_key(fs::DirEntry::file_name);

    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("stat {}", path.display()))?;
        let rel = path
            .strip_prefix(base_dir)
            .with_context(|| format!("strip prefix {}", path.display()))?;
        let zip_name = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        if file_type.is_dir() {
            if should_traverse_source_dir(&zip_name) {
                append_directory_recursive(
                    zip,
                    written_entries,
                    base_dir,
                    &path,
                    deflate_opts,
                    stored_opts,
                )?;
            }
        } else if file_type.is_file() {
            if written_entries.contains(&zip_name) || !should_bundle_source_entry(&zip_name) {
                continue;
            }
            let bytes = fs::read(&path).with_context(|| format!("read {}", path.display()))?;
            let opts = if is_already_compressed(&zip_name) {
                stored_opts
            } else {
                deflate_opts
            };
            write_zip_entry(zip, written_entries, &zip_name, &bytes, opts)?;
        }
    }
    Ok(())
}

fn write_zip_entry<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    written_entries: &mut BTreeSet<String>,
    name: &str,
    bytes: &[u8],
    opts: SimpleFileOptions,
) -> Result<()> {
    if !written_entries.insert(name.to_string()) {
        return Ok(());
    }
    zip.start_file(name, opts)
        .with_context(|| format!("start zip entry {name}"))?;
    zip.write_all(bytes)
        .with_context(|| format!("write zip entry {name}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::gzip_compress_bytes;
    use flate2::read::GzDecoder;
    use std::io::Read;

    #[test]
    fn gzip_compress_bytes_round_trips_and_skips_empty_payloads() {
        assert!(gzip_compress_bytes(b"").is_none());

        let original = b"console.log('bbb web client');".repeat(64);
        let compressed = gzip_compress_bytes(&original).expect("non-empty gzip payload");
        assert_eq!(&compressed[..2], &[0x1f, 0x8b]);
        assert!(compressed.len() < original.len());

        let mut decoded = Vec::new();
        GzDecoder::new(compressed.as_ref())
            .read_to_end(&mut decoded)
            .expect("decode gzip payload");
        assert_eq!(decoded, original);
    }
}
