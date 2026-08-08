use std::{
    collections::{HashMap, HashSet},
    fmt,
    fs::{File, OpenOptions},
    io::{Read, Write},
    net::Ipv6Addr,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime},
};

use anyhow::{bail, Context, Result};
use bbb_protocol::packets::{ResourcePackPush, ResourcePackResponseAction};
use chrono::{SecondsFormat, Utc};
use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue, CONTENT_LENGTH, USER_AGENT},
};
use serde::ser::{SerializeMap, Serializer};
use sha1::{Digest, Sha1};
use uuid::Uuid;

use crate::types::ServerResourcePackPolicy;

pub const VANILLA_MAX_SERVER_PACK_DOWNLOAD_BYTES: u64 = 250 * 1024 * 1024;
pub const VANILLA_MAX_KEPT_SERVER_PACK_FILES: usize = 20;
static DOWNLOAD_SERIAL: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, PartialEq, Eq)]
pub struct ServerPackDownloadProxy {
    host: String,
    port: u16,
    credentials: Option<ServerPackProxyCredentials>,
}

#[derive(Clone, PartialEq, Eq)]
struct ServerPackProxyCredentials {
    username: String,
    password: String,
}

impl ServerPackDownloadProxy {
    /// Mirrors `Main`'s SOCKS proxy construction. Invalid ports fall back to
    /// `Proxy.NO_PROXY`, and credentials are installed only when both values
    /// are non-empty.
    pub fn from_vanilla_args(
        host: impl Into<String>,
        port: i32,
        username: Option<String>,
        password: Option<String>,
    ) -> Option<Self> {
        let port = u16::try_from(port).ok()?;
        let credentials = match (username, password) {
            (Some(username), Some(password)) if !username.is_empty() && !password.is_empty() => {
                Some(ServerPackProxyCredentials { username, password })
            }
            _ => None,
        };
        Some(Self {
            host: host.into(),
            port,
            credentials,
        })
    }

    fn reqwest_proxy(&self) -> Option<reqwest::Proxy> {
        let host = if self.host.parse::<Ipv6Addr>().is_ok() {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        let mut proxy = reqwest::Proxy::all(format!("socks5h://{host}:{}", self.port)).ok()?;
        if let Some(credentials) = &self.credentials {
            proxy = proxy.basic_auth(&credentials.username, &credentials.password);
        }
        Some(proxy)
    }
}

impl fmt::Debug for ServerPackDownloadProxy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ServerPackDownloadProxy")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("has_credentials", &self.credentials.is_some())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerPackDownloadRequest {
    pub url: String,
    pub expected_sha1: Option<String>,
    pub cache_dir: PathBuf,
    pub username: String,
    pub profile_id: Uuid,
    pub timeout: Option<Duration>,
    pub max_bytes: u64,
    pub proxy: Option<ServerPackDownloadProxy>,
}

impl ServerPackDownloadRequest {
    pub fn new(
        url: impl Into<String>,
        expected_sha1: Option<String>,
        cache_dir: impl Into<PathBuf>,
        username: impl Into<String>,
        profile_id: Uuid,
    ) -> Self {
        Self {
            url: url.into(),
            expected_sha1,
            cache_dir: cache_dir.into(),
            username: username.into(),
            profile_id,
            timeout: None,
            max_bytes: VANILLA_MAX_SERVER_PACK_DOWNLOAD_BYTES,
            proxy: None,
        }
    }

    pub fn with_proxy(mut self, proxy: Option<ServerPackDownloadProxy>) -> Self {
        self.proxy = proxy;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerPackDownload {
    pub archive_path: PathBuf,
    pub sha1: String,
    pub bytes: u64,
    pub cache_hit: bool,
}

pub fn initialize_server_pack_download_cache(cache_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(cache_dir)
        .with_context(|| format!("create server pack download cache {}", cache_dir.display()))?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(cache_dir.join("log.json"))
        .with_context(|| {
            format!(
                "open server pack download event log {}",
                cache_dir.join("log.json").display()
            )
        })?;
    vacuum_server_pack_download_cache(cache_dir, VANILLA_MAX_KEPT_SERVER_PACK_FILES);
    Ok(())
}

pub fn append_server_pack_download_log(
    cache_dir: &Path,
    pack_id: Uuid,
    url: &str,
    requested_hash: Option<&str>,
    download: Option<&ServerPackDownload>,
) -> Result<()> {
    let event_time = Utc::now().to_rfc3339_opts(SecondsFormat::AutoSi, true);
    let requested_hash = requested_hash.and_then(parse_expected_sha1);
    let outcome = match download {
        Some(download) => match server_pack_file_info(cache_dir, &download.archive_path) {
            Ok((name, size)) => ServerPackLogOutcome::File { name, size },
            Err(error) => {
                tracing::warn!(%error, "server pack download log could not inspect cache file");
                ServerPackLogOutcome::Error("no_access")
            }
        },
        None => ServerPackLogOutcome::Error("download_failed"),
    };

    let log_path = cache_dir.join("log.json");
    let mut output = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("open server pack download event log {}", log_path.display()))?;
    {
        let mut serializer = serde_json::Serializer::new(&mut output);
        let mut entry = serializer.serialize_map(None).with_context(|| {
            format!(
                "encode server pack download event log {}",
                log_path.display()
            )
        })?;
        entry.serialize_entry("id", &pack_id.to_string())?;
        entry.serialize_entry("url", url)?;
        entry.serialize_entry("time", &event_time)?;
        if let Some(hash) = requested_hash {
            entry.serialize_entry("hash", &hash)?;
        }
        match outcome {
            ServerPackLogOutcome::File { name, size } => {
                entry.serialize_entry("file", &ServerPackLogFile { name: &name, size })?;
            }
            ServerPackLogOutcome::Error(error) => entry.serialize_entry("error", error)?,
        }
        entry.end()?;
    }
    output.write_all(b"\n").with_context(|| {
        format!(
            "append server pack download event log {}",
            log_path.display()
        )
    })?;
    output.flush().with_context(|| {
        format!(
            "flush server pack download event log {}",
            log_path.display()
        )
    })?;
    Ok(())
}

enum ServerPackLogOutcome {
    File { name: String, size: u64 },
    Error(&'static str),
}

#[derive(serde::Serialize)]
struct ServerPackLogFile<'a> {
    name: &'a str,
    size: u64,
}

fn server_pack_file_info(cache_dir: &Path, path: &Path) -> Result<(String, u64)> {
    let size = std::fs::metadata(path)
        .with_context(|| format!("read server pack cache metadata {}", path.display()))?
        .len();
    let relative = path.strip_prefix(cache_dir).with_context(|| {
        format!(
            "server pack cache file {} is outside {}",
            path.display(),
            cache_dir.display()
        )
    })?;
    Ok((relative.to_string_lossy().into_owned(), size))
}

#[derive(Debug)]
struct ServerPackCacheFile {
    path: PathBuf,
    modified: SystemTime,
    removal_priority: usize,
}

fn vacuum_server_pack_download_cache(cache_dir: &Path, max_files: usize) {
    let mut files = match nested_server_pack_cache_files(cache_dir) {
        Ok(files) => files,
        Err(error) => {
            tracing::error!(%error, cache = %cache_dir.display(), "failed to vacuum server pack cache");
            return;
        }
    };
    let remove_count = files.len().saturating_sub(max_files);
    if remove_count == 0 {
        return;
    }

    files.sort_by(|left, right| right.modified.cmp(&left.modified));
    let mut parent_counts = HashMap::<PathBuf, usize>::new();
    for file in &mut files {
        let parent = file
            .path
            .parent()
            .expect("nested server pack cache file has a parent")
            .to_path_buf();
        let count = parent_counts.entry(parent).or_default();
        file.removal_priority = *count;
        *count = count.saturating_add(1);
    }
    files.reverse();
    files.sort_by(|left, right| right.removal_priority.cmp(&left.removal_priority));

    let mut empty_directory_candidates = HashSet::new();
    for file in files.into_iter().take(remove_count) {
        match std::fs::remove_file(&file.path) {
            Ok(()) => {
                if file.removal_priority == 0 {
                    if let Some(parent) = file.path.parent() {
                        empty_directory_candidates.insert(parent.to_path_buf());
                    }
                }
            }
            Err(error) => {
                tracing::warn!(%error, path = %file.path.display(), "failed to delete server pack cache file");
            }
        }
    }
    empty_directory_candidates.remove(cache_dir);
    for directory in empty_directory_candidates {
        match std::fs::remove_dir(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::DirectoryNotEmpty => {}
            Err(error) => {
                tracing::warn!(%error, path = %directory.display(), "failed to delete empty server pack cache directory");
            }
        }
    }
}

fn nested_server_pack_cache_files(cache_dir: &Path) -> Result<Vec<ServerPackCacheFile>> {
    let mut files = Vec::new();
    let mut pending = vec![cache_dir.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)
            .with_context(|| format!("walk server pack cache {}", directory.display()))?
        {
            let entry = entry.with_context(|| {
                format!("read server pack cache entry under {}", directory.display())
            })?;
            let file_type = entry.file_type().with_context(|| {
                format!("inspect server pack cache entry {}", entry.path().display())
            })?;
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file() && directory != cache_dir {
                let modified = entry
                    .metadata()
                    .with_context(|| {
                        format!("read server pack cache metadata {}", entry.path().display())
                    })?
                    .modified()
                    .with_context(|| {
                        format!("read server pack cache mtime {}", entry.path().display())
                    })?;
                files.push(ServerPackCacheFile {
                    path: entry.path(),
                    modified,
                    removal_priority: 0,
                });
            }
        }
    }
    Ok(files)
}

/// Progress callbacks emitted in the same order as Vanilla's
/// `HttpUtil.DownloadProgressListener` contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerPackDownloadProgress {
    RequestStarted,
    DownloadStarted { total_bytes: Option<u64> },
    DownloadedBytes { bytes: u64 },
    RequestFinished { success: bool },
}

/// Downloads one server pack without blocking the connection/event-stream task.
///
/// The blocking HTTP reader enforces the byte limit while streaming and publishes a complete
/// cache file only after SHA-1 verification. A non-40-hex server hash follows vanilla and is
/// treated as absent rather than as a failed hash assertion.
pub async fn download_server_pack(
    request: ServerPackDownloadRequest,
) -> Result<ServerPackDownload> {
    download_server_pack_with_progress(request, |_| {}).await
}

pub async fn download_server_pack_with_progress<F>(
    request: ServerPackDownloadRequest,
    mut progress: F,
) -> Result<ServerPackDownload>
where
    F: FnMut(ServerPackDownloadProgress) + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        progress(ServerPackDownloadProgress::RequestStarted);
        let result = download_server_pack_blocking(request, &mut progress);
        match &result {
            Ok((_, completion_reported)) if !completion_reported => {
                progress(ServerPackDownloadProgress::RequestFinished { success: true });
            }
            Err(_) => {
                progress(ServerPackDownloadProgress::RequestFinished { success: false });
            }
            Ok(_) => {}
        }
        result.map(|(download, _)| download)
    })
    .await
    .context("join server pack download worker")?
}

fn download_server_pack_blocking(
    request: ServerPackDownloadRequest,
    progress: &mut impl FnMut(ServerPackDownloadProgress),
) -> Result<(ServerPackDownload, bool)> {
    if request.max_bytes == 0 {
        bail!("server pack download byte limit must be positive");
    }
    if request.timeout.is_some_and(|timeout| timeout.is_zero()) {
        bail!("server pack download timeout must be positive");
    }
    if !is_allowed_resource_pack_url(&request.url) {
        bail!("server pack URL must be a valid HTTP or HTTPS URL");
    }
    reqwest::Url::parse(&request.url).context("parse server pack URL")?;
    let expected_sha1 = request
        .expected_sha1
        .as_deref()
        .and_then(parse_expected_sha1);
    let requested_target_path = expected_sha1
        .as_deref()
        .map(|hash| cached_archive_path(&request.cache_dir, hash));
    if let Some((expected_sha1, cached_path)) =
        expected_sha1.as_deref().zip(requested_target_path.as_ref())
    {
        if let Ok(bytes) = validate_cached_archive(&cached_path, expected_sha1) {
            progress(ServerPackDownloadProgress::RequestFinished { success: true });
            touch_cached_archive(&cached_path);
            return Ok((
                ServerPackDownload {
                    archive_path: cached_path.clone(),
                    sha1: expected_sha1.to_string(),
                    bytes,
                    cache_hit: true,
                },
                true,
            ));
        }
        delete_cache_entry(&cached_path).with_context(|| {
            format!(
                "remove mismatched server pack cache {}",
                cached_path.display()
            )
        })?;
    }

    // Vanilla uses `Proxy.NO_PROXY` unless `Main --proxyHost` created an
    // explicit SOCKS proxy. Disable reqwest's environment proxy discovery so
    // ambient HTTP_PROXY/HTTPS_PROXY cannot silently change that contract.
    let mut client_builder = Client::builder().no_proxy();
    if let Some(timeout) = request.timeout {
        client_builder = client_builder.timeout(timeout);
    }
    if let Some(proxy) = request
        .proxy
        .as_ref()
        .and_then(ServerPackDownloadProxy::reqwest_proxy)
    {
        client_builder = client_builder.proxy(proxy);
    }
    let client = client_builder
        .build()
        .context("build server pack HTTP client")?;
    let headers = download_headers(&request.username, request.profile_id)?;
    let mut response = client
        .get(&request.url)
        .headers(headers)
        .send()
        .context("request server pack")?
        .error_for_status()
        .context("server pack HTTP status")?;
    let content_length = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    std::fs::create_dir_all(&request.cache_dir).with_context(|| {
        format!(
            "create server pack download cache {}",
            request.cache_dir.display()
        )
    })?;
    progress(ServerPackDownloadProgress::DownloadStarted {
        total_bytes: content_length,
    });
    if let Some(content_length) = content_length {
        if content_length > request.max_bytes {
            bail!(
                "server pack Content-Length is {content_length} bytes, exceeding the {} byte limit",
                request.max_bytes
            );
        }
    }

    let (download_path, mut cleanup) = match requested_target_path {
        Some(path) => (path, None),
        None => {
            let path = allocate_partial_path(&request.cache_dir)?;
            let cleanup = DownloadCleanup::new(path.clone());
            (path, Some(cleanup))
        }
    };
    let mut output_options = OpenOptions::new();
    output_options.write(true);
    if expected_sha1.is_some() {
        // Vanilla passes only `StandardOpenOption.CREATE` here. The requested
        // target is normally absent after `deleteIfExists`, but a file raced
        // into place is overwritten from offset zero without an extra
        // `TRUNCATE_EXISTING` semantic.
        output_options.create(true);
    } else {
        output_options.create_new(true);
    }
    let mut output = output_options
        .open(&download_path)
        .with_context(|| format!("create server pack download {}", download_path.display()))?;

    let mut digest = Sha1::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 8_196];
    loop {
        let read = response
            .read(&mut buffer)
            .context("read server pack response body")?;
        if read == 0 {
            break;
        }
        bytes = bytes
            .checked_add(read as u64)
            .context("server pack download size overflow")?;
        progress(ServerPackDownloadProgress::DownloadedBytes { bytes });
        if bytes > request.max_bytes {
            bail!(
                "server pack response exceeded the {} byte limit",
                request.max_bytes
            );
        }
        digest.update(&buffer[..read]);
        output
            .write_all(&buffer[..read])
            .context("write server pack download cache")?;
    }
    output.flush().context("flush server pack download cache")?;
    drop(output);

    let actual_sha1 = format!("{:x}", digest.finalize());
    if let Some(expected_sha1) = expected_sha1 {
        if expected_sha1 != actual_sha1 {
            bail!("server pack SHA-1 mismatch: expected {expected_sha1}, downloaded {actual_sha1}");
        }
        return Ok((
            ServerPackDownload {
                archive_path: download_path,
                sha1: actual_sha1,
                bytes,
                cache_hit: false,
            },
            false,
        ));
    }

    let archive_path = cached_archive_path(&request.cache_dir, &actual_sha1);
    let cache_hit = publish_cached_archive(&download_path, &archive_path, &actual_sha1)?;
    cleanup
        .as_mut()
        .expect("unhashed download owns a temporary cache file")
        .disarm();
    Ok((
        ServerPackDownload {
            archive_path,
            sha1: actual_sha1,
            bytes,
            cache_hit,
        },
        false,
    ))
}

fn download_headers(username: &str, profile_id: Uuid) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    for (name, value) in [
        ("x-minecraft-username", username.to_string()),
        ("x-minecraft-uuid", profile_id.simple().to_string()),
        ("x-minecraft-version", bbb_protocol::MC_VERSION.to_string()),
        (
            "x-minecraft-version-id",
            bbb_protocol::MC_VERSION.to_string(),
        ),
        (
            "x-minecraft-pack-format",
            bbb_protocol::MC_RESOURCE_PACK_FORMAT.to_vanilla_string(),
        ),
    ] {
        headers.insert(
            reqwest::header::HeaderName::from_bytes(name.as_bytes())?,
            HeaderValue::from_str(&value)
                .with_context(|| format!("encode server pack request header {name}"))?,
        );
    }
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(&format!("Minecraft Java/{}", bbb_protocol::MC_VERSION))?,
    );
    Ok(headers)
}

fn parse_expected_sha1(value: &str) -> Option<String> {
    (value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| value.to_ascii_lowercase())
}

fn cached_archive_path(cache_dir: &Path, sha1: &str) -> PathBuf {
    cache_dir.join(sha1)
}

fn delete_cache_entry(path: &Path) -> std::io::Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if metadata.file_type().is_dir() {
        std::fs::remove_dir(path)
    } else {
        std::fs::remove_file(path)
    }
}

fn touch_cached_archive(path: &Path) {
    if let Ok(file) = File::open(path) {
        let _ = file.set_modified(std::time::SystemTime::now());
    }
}
fn validate_cached_archive(path: &Path, expected_sha1: &str) -> Result<u64> {
    let mut file =
        File::open(path).with_context(|| format!("open cached server pack {}", path.display()))?;
    let mut digest = Sha1::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("read cached server pack {}", path.display()))?;
        if read == 0 {
            break;
        }
        bytes = bytes
            .checked_add(read as u64)
            .context("cached server pack size overflow")?;
        digest.update(&buffer[..read]);
    }
    let actual_sha1 = format!("{:x}", digest.finalize());
    if actual_sha1 != expected_sha1 {
        bail!("cached server pack SHA-1 mismatch: expected {expected_sha1}, found {actual_sha1}");
    }
    Ok(bytes)
}

fn publish_cached_archive(partial_path: &Path, archive_path: &Path, sha1: &str) -> Result<bool> {
    if archive_path.exists() {
        if validate_cached_archive(archive_path, sha1).is_ok() {
            touch_cached_archive(archive_path);
            std::fs::remove_file(partial_path).with_context(|| {
                format!(
                    "remove duplicate server pack download {}",
                    partial_path.display()
                )
            })?;
            return Ok(true);
        }
        std::fs::remove_file(archive_path).with_context(|| {
            format!(
                "remove corrupt cached server pack {}",
                archive_path.display()
            )
        })?;
    }
    match std::fs::hard_link(partial_path, archive_path) {
        Ok(()) => {
            std::fs::remove_file(partial_path).with_context(|| {
                format!(
                    "remove published server pack staging file {}",
                    partial_path.display()
                )
            })?;
            Ok(false)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            validate_cached_archive(archive_path, sha1)?;
            std::fs::remove_file(partial_path).with_context(|| {
                format!(
                    "remove raced server pack download {}",
                    partial_path.display()
                )
            })?;
            Ok(true)
        }
        Err(error) => Err(error)
            .with_context(|| format!("publish server pack cache {}", archive_path.display())),
    }
}

fn allocate_partial_path(cache_dir: &Path) -> Result<PathBuf> {
    for _ in 0..32 {
        let serial = DOWNLOAD_SERIAL.fetch_add(1, Ordering::Relaxed);
        let path = cache_dir.join(format!(".download-{}-{serial}.partial", std::process::id()));
        if !path.exists() {
            return Ok(path);
        }
    }
    bail!("could not allocate a unique server pack download path")
}

struct DownloadCleanup {
    path: Option<PathBuf>,
}

impl DownloadCleanup {
    fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    fn disarm(&mut self) {
        self.path = None;
    }
}

impl Drop for DownloadCleanup {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = std::fs::remove_file(path);
        }
    }
}

pub(crate) fn response_action_for_push(update: &ResourcePackPush) -> ResourcePackResponseAction {
    response_action_for_push_with_policy(update, ServerResourcePackPolicy::Decline)
}

pub(crate) fn response_action_for_push_with_policy(
    update: &ResourcePackPush,
    policy: ServerResourcePackPolicy,
) -> ResourcePackResponseAction {
    if is_allowed_resource_pack_url(&update.url) {
        match policy {
            ServerResourcePackPolicy::Accept => ResourcePackResponseAction::Accepted,
            ServerResourcePackPolicy::Decline => ResourcePackResponseAction::Declined,
        }
    } else {
        ResourcePackResponseAction::InvalidUrl
    }
}

fn is_allowed_resource_pack_url(url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{Arc, Mutex},
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn resource_pack_response_follows_vanilla_url_scheme_gate() {
        let valid = ResourcePackPush {
            id: Uuid::nil(),
            url: "https://example.invalid/pack.zip".to_string(),
            hash: "".to_string(),
            required: false,
            prompt: None,
        };
        assert_eq!(
            response_action_for_push_with_policy(&valid, ServerResourcePackPolicy::Accept),
            ResourcePackResponseAction::Accepted
        );
        assert_eq!(
            response_action_for_push_with_policy(&valid, ServerResourcePackPolicy::Decline),
            ResourcePackResponseAction::Declined
        );
        assert!(is_allowed_resource_pack_url(&valid.url));
        assert!(is_allowed_resource_pack_url(
            "http://example.invalid/pack.zip"
        ));
        assert!(is_allowed_resource_pack_url(
            "https://example.invalid/pack with space.zip"
        ));
        assert!(!is_allowed_resource_pack_url(
            "ftp://example.invalid/pack.zip"
        ));
        assert!(!is_allowed_resource_pack_url("https://bad host/pack.zip"));
        assert!(!is_allowed_resource_pack_url("not-a-url"));
    }

    #[test]
    fn server_pack_hash_gate_accepts_only_vanilla_sha1_shape() {
        assert_eq!(
            parse_expected_sha1("ABCDEF0123456789ABCDEF0123456789ABCDEF01").as_deref(),
            Some("abcdef0123456789abcdef0123456789abcdef01")
        );
        assert_eq!(parse_expected_sha1("abcdef"), None);
        assert_eq!(
            parse_expected_sha1("gggggggggggggggggggggggggggggggggggggggg"),
            None
        );
    }

    #[tokio::test]
    async fn server_pack_download_verifies_hash_headers_and_reuses_cache() {
        let body = b"server-pack-archive".to_vec();
        let expected_sha1 = sha1_hex(&body);
        let cache_dir = unique_temp_dir("success");
        let (url, server) = serve_once(body.clone(), true);
        let request = ServerPackDownloadRequest::new(
            &url,
            Some(expected_sha1.clone()),
            &cache_dir,
            "bbb-client",
            Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678),
        );

        let downloaded = download_server_pack(request.clone()).await.unwrap();
        let request_headers = server.join().unwrap();

        assert_eq!(downloaded.sha1, expected_sha1);
        assert_eq!(downloaded.bytes, body.len() as u64);
        assert!(!downloaded.cache_hit);
        assert_eq!(
            downloaded.archive_path.file_name().unwrap(),
            expected_sha1.as_str()
        );
        assert_eq!(std::fs::read(&downloaded.archive_path).unwrap(), body);
        assert!(request_headers.contains("x-minecraft-username: bbb-client"));
        assert!(request_headers.contains("x-minecraft-version: 26.1"));
        assert!(request_headers.contains("x-minecraft-pack-format: 84.0"));
        assert!(request_headers.contains("user-agent: Minecraft Java/26.1"));

        File::options()
            .write(true)
            .open(&downloaded.archive_path)
            .unwrap()
            .set_modified(std::time::SystemTime::UNIX_EPOCH)
            .unwrap();
        let cached = download_server_pack(request).await.unwrap();
        assert!(cached.cache_hit);
        assert_eq!(cached.archive_path, downloaded.archive_path);
        assert!(
            std::fs::metadata(&cached.archive_path)
                .unwrap()
                .modified()
                .unwrap()
                > std::time::SystemTime::UNIX_EPOCH
        );
        std::fs::remove_dir_all(cache_dir).unwrap();
    }

    #[test]
    fn server_pack_cache_initialization_vacuums_old_duplicates_and_preserves_log() {
        let cache_dir = unique_temp_dir("cache-vacuum");
        initialize_server_pack_download_cache(&cache_dir).unwrap();
        let first = cache_dir.join(Uuid::from_u128(1).to_string());
        let second = cache_dir.join(Uuid::from_u128(2).to_string());
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        let first_new = cache_file_at(&first, "first-new", 4);
        let second_new = cache_file_at(&second, "second-new", 3);
        let second_old = cache_file_at(&second, "second-old", 2);
        let first_old = cache_file_at(&first, "first-old", 1);

        vacuum_server_pack_download_cache(&cache_dir, 2);

        assert!(cache_dir.join("log.json").is_file());
        assert!(first_new.is_file());
        assert!(second_new.is_file());
        assert!(!first_old.exists());
        assert!(!second_old.exists());
        std::fs::remove_dir_all(cache_dir).unwrap();
    }

    #[test]
    fn server_pack_download_log_records_vanilla_file_and_error_shapes() {
        let cache_dir = unique_temp_dir("download-log");
        initialize_server_pack_download_cache(&cache_dir).unwrap();
        let pack_id = Uuid::from_u128(7);
        let pack_dir = cache_dir.join(pack_id.to_string());
        std::fs::create_dir_all(&pack_dir).unwrap();
        let archive_path = pack_dir.join("abcdef0123456789abcdef0123456789abcdef01");
        std::fs::write(&archive_path, b"pack").unwrap();
        let download = ServerPackDownload {
            archive_path: archive_path.clone(),
            sha1: "abcdef0123456789abcdef0123456789abcdef01".to_string(),
            bytes: 4,
            cache_hit: false,
        };

        append_server_pack_download_log(
            &cache_dir,
            pack_id,
            "https://example.invalid/pack.zip",
            Some("ABCDEF0123456789ABCDEF0123456789ABCDEF01"),
            Some(&download),
        )
        .unwrap();
        append_server_pack_download_log(
            &cache_dir,
            Uuid::from_u128(8),
            "https://example.invalid/failed.zip",
            Some("invalid"),
            None,
        )
        .unwrap();
        std::fs::remove_file(&archive_path).unwrap();
        append_server_pack_download_log(
            &cache_dir,
            pack_id,
            "https://example.invalid/missing.zip",
            None,
            Some(&download),
        )
        .unwrap();

        let raw = std::fs::read_to_string(cache_dir.join("log.json")).unwrap();
        let lines = raw.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("{\"id\":"));
        let entries = lines
            .iter()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(entries[0]["id"], pack_id.to_string());
        assert_eq!(
            entries[0]["hash"],
            "abcdef0123456789abcdef0123456789abcdef01"
        );
        assert_eq!(
            entries[0]["file"]["name"],
            format!("{}/abcdef0123456789abcdef0123456789abcdef01", pack_id)
        );
        assert_eq!(entries[0]["file"]["size"], 4);
        assert_eq!(entries[1]["error"], "download_failed");
        assert!(entries[1].get("hash").is_none());
        assert_eq!(entries[2]["error"], "no_access");
        std::fs::remove_dir_all(cache_dir).unwrap();
    }

    #[tokio::test]
    async fn server_pack_download_uses_vanilla_socks_proxy_and_redacts_credentials() {
        let body = b"server-pack-through-socks".to_vec();
        let cache_dir = unique_temp_dir("socks-proxy");
        let (proxy_host, proxy_port, server) = serve_socks5_once(body.clone());
        let proxy = ServerPackDownloadProxy::from_vanilla_args(
            proxy_host,
            i32::from(proxy_port),
            Some("proxy-user".to_string()),
            Some("proxy-password".to_string()),
        )
        .unwrap();
        let debug = format!("{proxy:?}");
        assert!(debug.contains("has_credentials: true"));
        assert!(!debug.contains("proxy-user"));
        assert!(!debug.contains("proxy-password"));

        let request = ServerPackDownloadRequest::new(
            "http://pack.example.invalid/lifecycle.zip",
            None,
            &cache_dir,
            "bbb-client",
            Uuid::nil(),
        )
        .with_proxy(Some(proxy));
        let downloaded = download_server_pack(request).await.unwrap();
        let receipt = server.join().unwrap();

        assert_eq!(std::fs::read(downloaded.archive_path).unwrap(), body);
        assert_eq!(receipt.username, "proxy-user");
        assert_eq!(receipt.password, "proxy-password");
        assert_eq!(receipt.target_host, "pack.example.invalid");
        assert_eq!(receipt.target_port, 80);
        assert!(receipt
            .request
            .starts_with("GET /lifecycle.zip HTTP/1.1\r\n"));
        std::fs::remove_dir_all(cache_dir).unwrap();
    }

    #[test]
    fn vanilla_proxy_arguments_fall_back_and_require_both_credentials() {
        assert!(
            ServerPackDownloadProxy::from_vanilla_args("proxy.example", -1, None, None,).is_none()
        );
        assert!(
            ServerPackDownloadProxy::from_vanilla_args("proxy.example", 65536, None, None,)
                .is_none()
        );
        let proxy = ServerPackDownloadProxy::from_vanilla_args(
            "proxy.example",
            8080,
            Some("user".to_string()),
            Some(String::new()),
        )
        .unwrap();
        assert!(format!("{proxy:?}").contains("has_credentials: false"));
    }

    #[tokio::test]
    async fn server_pack_download_reports_vanilla_progress_order_for_network_and_cache() {
        let body = vec![b'p'; 20_000];
        let expected_sha1 = sha1_hex(&body);
        let cache_dir = unique_temp_dir("progress");
        let (url, server) = serve_once(body.clone(), true);
        let request = ServerPackDownloadRequest::new(
            url,
            Some(expected_sha1),
            &cache_dir,
            "bbb-client",
            Uuid::nil(),
        );
        let progress = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&progress);
        let downloaded = download_server_pack_with_progress(request.clone(), move |event| {
            captured.lock().unwrap().push(event);
        })
        .await
        .unwrap();
        server.join().unwrap();

        {
            let progress = progress.lock().unwrap();
            assert_eq!(progress[0], ServerPackDownloadProgress::RequestStarted);
            assert_eq!(
                progress[1],
                ServerPackDownloadProgress::DownloadStarted {
                    total_bytes: Some(body.len() as u64),
                }
            );
            assert_eq!(
                progress.last(),
                Some(&ServerPackDownloadProgress::RequestFinished { success: true })
            );
            let downloaded_bytes = progress
                .iter()
                .filter_map(|event| match event {
                    ServerPackDownloadProgress::DownloadedBytes { bytes } => Some(*bytes),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(downloaded_bytes.last(), Some(&(body.len() as u64)));
            let mut previous = 0;
            for bytes in downloaded_bytes {
                assert!(bytes > previous);
                assert!(bytes - previous <= 8_196);
                previous = bytes;
            }
        }

        File::options()
            .write(true)
            .open(&downloaded.archive_path)
            .unwrap()
            .set_modified(SystemTime::UNIX_EPOCH)
            .unwrap();
        let cached_progress = Arc::new(Mutex::new(Vec::new()));
        let mtime_at_finish = Arc::new(Mutex::new(None));
        let captured = Arc::clone(&cached_progress);
        let captured_mtime = Arc::clone(&mtime_at_finish);
        let cached_path = downloaded.archive_path.clone();
        let cached = download_server_pack_with_progress(request, move |event| {
            if event == (ServerPackDownloadProgress::RequestFinished { success: true }) {
                *captured_mtime.lock().unwrap() =
                    Some(std::fs::metadata(&cached_path).unwrap().modified().unwrap());
            }
            captured.lock().unwrap().push(event);
        })
        .await
        .unwrap();
        assert!(cached.cache_hit);
        assert_eq!(cached.archive_path, downloaded.archive_path);
        assert_eq!(
            *mtime_at_finish.lock().unwrap(),
            Some(SystemTime::UNIX_EPOCH)
        );
        assert!(
            std::fs::metadata(&cached.archive_path)
                .unwrap()
                .modified()
                .unwrap()
                > SystemTime::UNIX_EPOCH
        );
        assert_eq!(
            *cached_progress.lock().unwrap(),
            vec![
                ServerPackDownloadProgress::RequestStarted,
                ServerPackDownloadProgress::RequestFinished { success: true },
            ]
        );
        std::fs::remove_dir_all(cache_dir).unwrap();
    }

    #[tokio::test]
    async fn requested_hash_mismatch_preserves_vanilla_target_for_next_validation() {
        let cache_dir = unique_temp_dir("hash-mismatch");
        let body = b"wrong contents".to_vec();
        let (url, server) = serve_once(body.clone(), true);
        let expected_sha1 = "0000000000000000000000000000000000000000";
        let request = ServerPackDownloadRequest::new(
            url,
            Some(expected_sha1.to_string()),
            &cache_dir,
            "bbb-client",
            Uuid::nil(),
        );

        let error = download_server_pack(request).await.unwrap_err();
        server.join().unwrap();

        assert!(format!("{error:#}").contains("SHA-1 mismatch"));
        assert_eq!(std::fs::read(cache_dir.join(expected_sha1)).unwrap(), body);
        assert_eq!(cache_dir.read_dir().unwrap().count(), 1);
        std::fs::remove_dir_all(cache_dir).unwrap();
    }

    #[tokio::test]
    async fn requested_hash_target_race_preserves_vanilla_non_truncating_create() {
        let body = b"requested-body".to_vec();
        let expected_sha1 = sha1_hex(&body);
        let cache_dir = unique_temp_dir("requested-target-race");
        let (url, server) = serve_once(body.clone(), true);
        let request = ServerPackDownloadRequest::new(
            url,
            Some(expected_sha1.clone()),
            &cache_dir,
            "bbb-client",
            Uuid::nil(),
        );
        let raced_target = cache_dir.join(&expected_sha1);
        let callback_target = raced_target.clone();

        let downloaded = download_server_pack_with_progress(request, move |progress| {
            if matches!(progress, ServerPackDownloadProgress::DownloadStarted { .. }) {
                let mut raced_contents = body.clone();
                raced_contents.extend_from_slice(b"-raced-trailer");
                std::fs::write(&callback_target, raced_contents).unwrap();
            }
        })
        .await
        .unwrap();
        server.join().unwrap();

        assert_eq!(downloaded.archive_path, raced_target);
        assert_eq!(downloaded.sha1, expected_sha1);
        assert_eq!(
            std::fs::read(downloaded.archive_path).unwrap(),
            b"requested-body-raced-trailer"
        );
        std::fs::remove_dir_all(cache_dir).unwrap();
    }

    #[tokio::test]
    async fn server_pack_download_removes_bad_requested_cache_before_network_failure() {
        let cache_dir = unique_temp_dir("bad-requested-cache");
        let expected_sha1 = "0000000000000000000000000000000000000000";
        let cached_path = cache_dir.join(expected_sha1);
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(&cached_path, b"corrupt").unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let request = ServerPackDownloadRequest::new(
            format!("http://{address}/unreachable.zip"),
            Some(expected_sha1.to_string()),
            &cache_dir,
            "bbb-client",
            Uuid::nil(),
        );

        download_server_pack(request).await.unwrap_err();

        assert!(!cached_path.exists());
        assert!(cache_dir.read_dir().unwrap().next().is_none());
        std::fs::remove_dir_all(cache_dir).unwrap();
    }

    #[tokio::test]
    async fn unhashed_download_touches_an_existing_content_addressed_file() {
        let body = b"same-unhashed-server-pack".to_vec();
        let cache_dir = unique_temp_dir("unhashed-touch");
        let (first_url, first_server) = serve_once(body.clone(), true);
        let first_request =
            ServerPackDownloadRequest::new(first_url, None, &cache_dir, "bbb-client", Uuid::nil());
        assert_eq!(first_request.timeout, None);
        let first = download_server_pack(first_request).await.unwrap();
        first_server.join().unwrap();
        File::options()
            .write(true)
            .open(&first.archive_path)
            .unwrap()
            .set_modified(SystemTime::UNIX_EPOCH)
            .unwrap();

        let (second_url, second_server) = serve_once(body, true);
        let second = download_server_pack(ServerPackDownloadRequest::new(
            second_url,
            None,
            &cache_dir,
            "bbb-client",
            Uuid::nil(),
        ))
        .await
        .unwrap();
        second_server.join().unwrap();

        assert!(second.cache_hit);
        assert_eq!(second.archive_path, first.archive_path);
        assert!(
            std::fs::metadata(second.archive_path)
                .unwrap()
                .modified()
                .unwrap()
                > SystemTime::UNIX_EPOCH
        );
        std::fs::remove_dir_all(cache_dir).unwrap();
    }

    #[tokio::test]
    async fn matching_requested_cache_bypasses_the_network_download_size_limit() {
        let body = b"preseeded-cache";
        let expected_sha1 = sha1_hex(body);
        let cache_dir = unique_temp_dir("preseeded-over-limit");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(cache_dir.join(&expected_sha1), body).unwrap();
        let mut request = ServerPackDownloadRequest::new(
            "https://example.invalid/not-requested.zip",
            Some(expected_sha1),
            &cache_dir,
            "bbb-client",
            Uuid::nil(),
        );
        request.max_bytes = 4;

        let cached = download_server_pack(request).await.unwrap();

        assert!(cached.cache_hit);
        assert_eq!(cached.bytes, body.len() as u64);
        std::fs::remove_dir_all(cache_dir).unwrap();
    }

    #[tokio::test]
    async fn server_pack_download_enforces_stream_limit_without_content_length() {
        let cache_dir = unique_temp_dir("stream-limit");
        let (url, server) = serve_once(b"eight123".to_vec(), false);
        let mut request =
            ServerPackDownloadRequest::new(url, None, &cache_dir, "bbb-client", Uuid::nil());
        request.max_bytes = 4;

        let progress = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&progress);
        let error = download_server_pack_with_progress(request, move |event| {
            captured.lock().unwrap().push(event);
        })
        .await
        .unwrap_err();
        server.join().unwrap();

        assert!(format!("{error:#}").contains("exceeded the 4 byte limit"));
        assert_eq!(
            *progress.lock().unwrap(),
            vec![
                ServerPackDownloadProgress::RequestStarted,
                ServerPackDownloadProgress::DownloadStarted { total_bytes: None },
                ServerPackDownloadProgress::DownloadedBytes { bytes: 8 },
                ServerPackDownloadProgress::RequestFinished { success: false },
            ]
        );
        assert!(cache_dir.read_dir().unwrap().next().is_none());
        std::fs::remove_dir_all(cache_dir).unwrap();
    }

    fn serve_once(
        body: Vec<u8>,
        include_content_length: bool,
    ) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
            }
            let content_length = include_content_length
                .then(|| format!("Content-Length: {}\r\n", body.len()))
                .unwrap_or_default();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\n{content_length}Connection: close\r\n\r\n"
            )
            .unwrap();
            stream.write_all(&body).unwrap();
            String::from_utf8(request).unwrap()
        });
        (format!("http://{address}/pack.zip"), server)
    }

    #[derive(Debug, PartialEq, Eq)]
    struct SocksReceipt {
        username: String,
        password: String,
        target_host: String,
        target_port: u16,
        request: String,
    }

    fn serve_socks5_once(body: Vec<u8>) -> (String, u16, thread::JoinHandle<SocksReceipt>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();

            let mut greeting = [0_u8; 2];
            stream.read_exact(&mut greeting).unwrap();
            assert_eq!(greeting[0], 5);
            let mut methods = vec![0_u8; usize::from(greeting[1])];
            stream.read_exact(&mut methods).unwrap();
            assert!(methods.contains(&2));
            stream.write_all(&[5, 2]).unwrap();

            let mut auth = [0_u8; 2];
            stream.read_exact(&mut auth).unwrap();
            assert_eq!(auth[0], 1);
            let mut username = vec![0_u8; usize::from(auth[1])];
            stream.read_exact(&mut username).unwrap();
            let mut password_len = [0_u8; 1];
            stream.read_exact(&mut password_len).unwrap();
            let mut password = vec![0_u8; usize::from(password_len[0])];
            stream.read_exact(&mut password).unwrap();
            stream.write_all(&[1, 0]).unwrap();

            let mut connect = [0_u8; 4];
            stream.read_exact(&mut connect).unwrap();
            assert_eq!(&connect[..3], &[5, 1, 0]);
            let target_host = match connect[3] {
                1 => {
                    let mut address = [0_u8; 4];
                    stream.read_exact(&mut address).unwrap();
                    std::net::Ipv4Addr::from(address).to_string()
                }
                3 => {
                    let mut length = [0_u8; 1];
                    stream.read_exact(&mut length).unwrap();
                    let mut host = vec![0_u8; usize::from(length[0])];
                    stream.read_exact(&mut host).unwrap();
                    String::from_utf8(host).unwrap()
                }
                4 => {
                    let mut address = [0_u8; 16];
                    stream.read_exact(&mut address).unwrap();
                    std::net::Ipv6Addr::from(address).to_string()
                }
                address_type => panic!("unexpected SOCKS address type {address_type}"),
            };
            let mut target_port = [0_u8; 2];
            stream.read_exact(&mut target_port).unwrap();
            stream.write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 0]).unwrap();

            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
            }
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();

            SocksReceipt {
                username: String::from_utf8(username).unwrap(),
                password: String::from_utf8(password).unwrap(),
                target_host,
                target_port: u16::from_be_bytes(target_port),
                request: String::from_utf8(request).unwrap(),
            }
        });
        (address.ip().to_string(), address.port(), server)
    }

    fn sha1_hex(bytes: &[u8]) -> String {
        let mut digest = Sha1::new();
        digest.update(bytes);
        format!("{:x}", digest.finalize())
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "bbb-server-pack-download-{label}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn cache_file_at(directory: &Path, name: &str, modified_seconds: u64) -> PathBuf {
        let path = directory.join(name);
        std::fs::write(&path, name.as_bytes()).unwrap();
        File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(modified_seconds))
            .unwrap();
        path
    }
}
