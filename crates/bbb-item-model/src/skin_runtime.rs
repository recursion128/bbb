use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender, TrySendError},
    thread,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use bbb_render_types::{
    decode_dynamic_player_skin_png, decode_dynamic_player_texture_png, DynamicPlayerSkinImage,
    DynamicPlayerTextureImage, MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES,
    MAX_DYNAMIC_PLAYER_TEXTURE_PNG_BYTES,
};

pub(crate) trait SkinPngFetcher {
    fn fetch_skin_png(&mut self, url: &str, max_bytes: usize) -> Result<Vec<u8>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DynamicPlayerTextureKind {
    Cape,
    Elytra,
}

impl DynamicPlayerTextureKind {
    fn cache_directory(self) -> &'static str {
        match self {
            Self::Cape => "capes",
            Self::Elytra => "elytra",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HttpSkinPngFetcher {
    client: reqwest::blocking::Client,
}

impl HttpSkinPngFetcher {
    pub(crate) fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(15))
                .user_agent("bbb-native")
                .build()
                .context("create player skin HTTP client")?,
        })
    }
}

impl SkinPngFetcher for HttpSkinPngFetcher {
    fn fetch_skin_png(&mut self, url: &str, max_bytes: usize) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(url)
            .send()
            .with_context(|| format!("request HTTP player skin {url}"))?
            .error_for_status()
            .with_context(|| format!("fetch HTTP player skin {url}"))?;
        let content_length = response.content_length();
        read_bounded_png_body(
            response,
            content_length,
            max_bytes,
            &format!("HTTP player texture {url}"),
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DynamicPlayerSkinRuntime {
    cache_dir: PathBuf,
    skins: HashMap<String, DynamicPlayerSkinImage>,
}

impl DynamicPlayerSkinRuntime {
    pub(crate) fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            skins: HashMap::new(),
        }
    }

    pub(crate) fn get_or_fetch_player_skin(
        &mut self,
        handle: u64,
        url: &str,
        fetcher: &mut impl SkinPngFetcher,
    ) -> Result<&DynamicPlayerSkinImage> {
        if !self.skins.contains_key(url) {
            let (bytes, fetched) = self.cached_or_fetched_bytes(handle, url, fetcher)?;
            let skin = decode_dynamic_player_skin_png(handle, &bytes)
                .with_context(|| format!("process player skin texture from {url}"))?;
            if fetched {
                let path = self.player_skin_path(handle);
                if let Err(err) = write_player_skin_cache(&path, &bytes) {
                    tracing::warn!(
                        ?err,
                        path = %path.display(),
                        "failed to cache player skin texture"
                    );
                }
            }
            self.skins.insert(url.to_string(), skin);
        }
        Ok(self
            .skins
            .get(url)
            .expect("player skin inserted before lookup"))
    }

    #[cfg(test)]
    fn cached_skin_count(&self) -> usize {
        self.skins.len()
    }

    fn cached_or_fetched_bytes(
        &self,
        handle: u64,
        url: &str,
        fetcher: &mut impl SkinPngFetcher,
    ) -> Result<(Vec<u8>, bool)> {
        let path = self.player_skin_path(handle);
        if path.is_file() {
            return read_bounded_png_file(
                &path,
                MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES,
                "cached player skin",
            )
            .map(|bytes| (bytes, false));
        }

        let bytes = fetcher
            .fetch_skin_png(url, MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES)
            .with_context(|| format!("fetch player skin texture from {url}"))?;
        validate_png_body_len(
            bytes.len(),
            MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES,
            "fetched player skin",
        )?;
        Ok((bytes, true))
    }

    fn player_skin_path(&self, handle: u64) -> PathBuf {
        player_skin_cache_path(&self.cache_dir, handle)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DynamicPlayerTextureRuntime {
    cache_dir: PathBuf,
    textures: HashMap<(DynamicPlayerTextureKind, String), DynamicPlayerTextureImage>,
}

impl DynamicPlayerTextureRuntime {
    pub(crate) fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            textures: HashMap::new(),
        }
    }

    pub(crate) fn get_or_fetch_player_texture(
        &mut self,
        kind: DynamicPlayerTextureKind,
        handle: u64,
        url: &str,
        fetcher: &mut impl SkinPngFetcher,
    ) -> Result<&DynamicPlayerTextureImage> {
        let key = (kind, url.to_string());
        if !self.textures.contains_key(&key) {
            let (bytes, fetched) = self.cached_or_fetched_bytes(kind, handle, url, fetcher)?;
            let texture = decode_dynamic_player_texture_png(handle, &bytes)
                .with_context(|| format!("process player {kind:?} texture from {url}"))?;
            if fetched {
                let path = self.player_texture_path(kind, handle);
                if let Err(err) = write_player_skin_cache(&path, &bytes) {
                    tracing::warn!(
                        ?err,
                        path = %path.display(),
                        kind = ?kind,
                        "failed to cache player profile texture"
                    );
                }
            }
            self.textures.insert(key.clone(), texture);
        }
        Ok(self
            .textures
            .get(&key)
            .expect("player profile texture inserted before lookup"))
    }

    #[cfg(test)]
    fn cached_texture_count(&self) -> usize {
        self.textures.len()
    }

    fn cached_or_fetched_bytes(
        &self,
        kind: DynamicPlayerTextureKind,
        handle: u64,
        url: &str,
        fetcher: &mut impl SkinPngFetcher,
    ) -> Result<(Vec<u8>, bool)> {
        let path = self.player_texture_path(kind, handle);
        if path.is_file() {
            return read_bounded_png_file(
                &path,
                MAX_DYNAMIC_PLAYER_TEXTURE_PNG_BYTES,
                &format!("cached player {kind:?} texture"),
            )
            .map(|bytes| (bytes, false));
        }

        let bytes = fetcher
            .fetch_skin_png(url, MAX_DYNAMIC_PLAYER_TEXTURE_PNG_BYTES)
            .with_context(|| format!("fetch player {kind:?} texture from {url}"))?;
        validate_png_body_len(
            bytes.len(),
            MAX_DYNAMIC_PLAYER_TEXTURE_PNG_BYTES,
            &format!("fetched player {kind:?} texture"),
        )?;
        Ok((bytes, true))
    }

    fn player_texture_path(&self, kind: DynamicPlayerTextureKind, handle: u64) -> PathBuf {
        player_texture_cache_path(&self.cache_dir, kind, handle)
    }
}

const DEFAULT_DYNAMIC_TEXTURE_MAX_ENTRIES: usize = 2_048;
const DEFAULT_DYNAMIC_TEXTURE_PENDING_CAPACITY: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DynamicTextureRequestKind {
    Skin,
    Cape,
    Elytra,
}

impl From<DynamicPlayerTextureKind> for DynamicTextureRequestKind {
    fn from(kind: DynamicPlayerTextureKind) -> Self {
        match kind {
            DynamicPlayerTextureKind::Cape => Self::Cape,
            DynamicPlayerTextureKind::Elytra => Self::Elytra,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DynamicTextureKey {
    resource_generation: u64,
    session_generation: u64,
    kind: DynamicTextureRequestKind,
    identity: String,
    handle: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DownloadedDynamicTexture {
    Skin(DynamicPlayerSkinImage),
    Profile(DynamicPlayerTextureImage),
}

impl DownloadedDynamicTexture {
    fn logical_bytes(&self) -> usize {
        match self {
            Self::Skin(image) => image.rgba.len(),
            Self::Profile(image) => image.rgba.len(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DynamicTextureCacheObservation {
    pub resource_generation: u64,
    pub session_generation: u64,
    pub accepting_requests: bool,
    pub entries: usize,
    pub pending: usize,
    pub ready: usize,
    pub failed: usize,
    pub stale_results: u64,
    pub queue_full: u64,
    pub resident_bytes: usize,
    pub peak_resident_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AsyncDynamicTextureState {
    Pending,
    Ready(DownloadedDynamicTexture),
    Failed,
}

#[derive(Debug, Clone)]
struct AsyncDynamicTextureEntry {
    state: AsyncDynamicTextureState,
    last_used: u64,
    delivery_pending: bool,
}

/// One bounded owner for every remote profile texture request. Exact keys keep
/// resource generation, semantic kind, full URL identity and render handle;
/// the handle hash is never used as proof of equality.
#[derive(Debug)]
pub(crate) struct AsyncDynamicTextureRuntime {
    resource_generation: u64,
    session_generation: u64,
    accepting_requests: bool,
    entries: HashMap<DynamicTextureKey, AsyncDynamicTextureEntry>,
    request_tx: SyncSender<DynamicTextureRequest>,
    result_rx: Receiver<DynamicTextureResult>,
    sequence: u64,
    max_entries: usize,
    pending_entries: usize,
    ready_entries: usize,
    failed_entries: usize,
    stale_results: u64,
    queue_full: u64,
    resident_bytes: usize,
    peak_resident_bytes: usize,
}

impl AsyncDynamicTextureRuntime {
    pub(crate) fn new(
        resource_generation: u64,
        cache_dir: impl Into<PathBuf>,
        fetcher: impl SkinPngFetcher + Send + 'static,
    ) -> Self {
        Self::with_limits(
            resource_generation,
            cache_dir,
            fetcher,
            DEFAULT_DYNAMIC_TEXTURE_MAX_ENTRIES,
            DEFAULT_DYNAMIC_TEXTURE_PENDING_CAPACITY,
        )
    }

    fn with_limits(
        resource_generation: u64,
        cache_dir: impl Into<PathBuf>,
        fetcher: impl SkinPngFetcher + Send + 'static,
        max_entries: usize,
        pending_capacity: usize,
    ) -> Self {
        let (request_tx, request_rx) =
            mpsc::sync_channel::<DynamicTextureRequest>(pending_capacity);
        let (result_tx, result_rx) = mpsc::channel::<DynamicTextureResult>();
        let cache_dir = cache_dir.into();
        thread::spawn(move || {
            let mut skins = DynamicPlayerSkinRuntime::new(&cache_dir);
            let mut profiles = DynamicPlayerTextureRuntime::new(cache_dir);
            let mut fetcher = fetcher;
            for request in request_rx {
                let result = match request.key.kind {
                    DynamicTextureRequestKind::Skin => skins
                        .get_or_fetch_player_skin(
                            request.key.handle,
                            &request.key.identity,
                            &mut fetcher,
                        )
                        .cloned()
                        .map(DownloadedDynamicTexture::Skin),
                    DynamicTextureRequestKind::Cape | DynamicTextureRequestKind::Elytra => {
                        let kind = request
                            .key
                            .kind
                            .profile_kind()
                            .expect("profile texture request kind");
                        profiles
                            .get_or_fetch_player_texture(
                                kind,
                                request.key.handle,
                                &request.key.identity,
                                &mut fetcher,
                            )
                            .cloned()
                            .map(DownloadedDynamicTexture::Profile)
                    }
                }
                .map_err(|error| format!("{error:#}"));
                // The bounded outer owner holds every Ready image. Keep the
                // worker's legacy synchronous caches transient so eviction
                // from that owner also releases decoded CPU pixels; the disk
                // cache remains available for a later re-admission.
                skins.skins.clear();
                profiles.textures.clear();
                if result_tx
                    .send(DynamicTextureResult {
                        key: request.key,
                        result,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            resource_generation,
            session_generation: 0,
            accepting_requests: true,
            entries: HashMap::new(),
            request_tx,
            result_rx,
            sequence: 0,
            max_entries: max_entries.max(1),
            pending_entries: 0,
            ready_entries: 0,
            failed_entries: 0,
            stale_results: 0,
            queue_full: 0,
            resident_bytes: 0,
            peak_resident_bytes: 0,
        }
    }

    pub(crate) fn queue_skin(&mut self, handle: u64, url: &str) -> bool {
        self.queue(DynamicTextureRequestKind::Skin, handle, url)
    }

    pub(crate) fn queue_profile(
        &mut self,
        kind: DynamicPlayerTextureKind,
        handle: u64,
        url: &str,
    ) -> bool {
        self.queue(kind.into(), handle, url)
    }

    fn queue(&mut self, kind: DynamicTextureRequestKind, handle: u64, identity: &str) -> bool {
        self.drain_worker_results();
        if !self.accepting_requests {
            return false;
        }
        self.sequence = self.sequence.saturating_add(1);
        let key = DynamicTextureKey {
            resource_generation: self.resource_generation,
            session_generation: self.session_generation,
            kind,
            identity: identity.to_owned(),
            handle,
        };
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_used = self.sequence;
            return !matches!(entry.state, AsyncDynamicTextureState::Failed);
        }
        if self.entries.len() >= self.max_entries {
            let evicted = self
                .entries
                .iter()
                .filter(|(_, entry)| !matches!(entry.state, AsyncDynamicTextureState::Pending))
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| key.clone());
            let Some(evicted) = evicted else {
                self.queue_full = self.queue_full.saturating_add(1);
                return false;
            };
            self.remove_entry(&evicted);
        }
        self.entries.insert(
            key.clone(),
            AsyncDynamicTextureEntry {
                state: AsyncDynamicTextureState::Pending,
                last_used: self.sequence,
                delivery_pending: false,
            },
        );
        self.pending_entries = self.pending_entries.saturating_add(1);
        match self
            .request_tx
            .try_send(DynamicTextureRequest { key: key.clone() })
        {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                self.queue_full = self.queue_full.saturating_add(1);
                self.remove_entry(&key);
                false
            }
            Err(TrySendError::Disconnected(_)) => {
                self.queue_full = self.queue_full.saturating_add(1);
                let entry = self
                    .entries
                    .get_mut(&key)
                    .expect("queued dynamic texture entry");
                entry.state = AsyncDynamicTextureState::Failed;
                self.pending_entries = self.pending_entries.saturating_sub(1);
                self.failed_entries = self.failed_entries.saturating_add(1);
                false
            }
        }
    }

    pub(crate) fn drain_skin_results(&mut self) -> Vec<AsyncDynamicPlayerSkinResult> {
        self.drain_worker_results();
        let mut drained = self
            .entries
            .iter_mut()
            .filter_map(|(key, entry)| {
                if key.kind != DynamicTextureRequestKind::Skin || !entry.delivery_pending {
                    return None;
                }
                entry.delivery_pending = false;
                let skin = match &entry.state {
                    AsyncDynamicTextureState::Ready(DownloadedDynamicTexture::Skin(skin)) => {
                        Some(skin.clone())
                    }
                    AsyncDynamicTextureState::Failed => None,
                    AsyncDynamicTextureState::Pending
                    | AsyncDynamicTextureState::Ready(DownloadedDynamicTexture::Profile(_)) => {
                        return None;
                    }
                };
                Some((
                    entry.last_used,
                    AsyncDynamicPlayerSkinResult {
                        url: key.identity.clone(),
                        skin,
                    },
                ))
            })
            .collect::<Vec<_>>();
        drained.sort_by_key(|(sequence, _)| *sequence);
        drained.into_iter().map(|(_, result)| result).collect()
    }

    pub(crate) fn drain_profile_results(&mut self) -> Vec<AsyncDynamicPlayerTextureResult> {
        self.drain_worker_results();
        let mut drained = self
            .entries
            .iter_mut()
            .filter_map(|(key, entry)| {
                let kind = key.kind.profile_kind()?;
                if !entry.delivery_pending {
                    return None;
                }
                entry.delivery_pending = false;
                let texture = match &entry.state {
                    AsyncDynamicTextureState::Ready(DownloadedDynamicTexture::Profile(texture)) => {
                        Some(texture.clone())
                    }
                    AsyncDynamicTextureState::Failed => None,
                    AsyncDynamicTextureState::Pending
                    | AsyncDynamicTextureState::Ready(DownloadedDynamicTexture::Skin(_)) => {
                        return None;
                    }
                };
                Some((
                    entry.last_used,
                    AsyncDynamicPlayerTextureResult {
                        kind,
                        url: key.identity.clone(),
                        texture,
                    },
                ))
            })
            .collect::<Vec<_>>();
        drained.sort_by_key(|(sequence, _)| *sequence);
        drained.into_iter().map(|(_, result)| result).collect()
    }

    fn drain_worker_results(&mut self) {
        while let Ok(result) = self.result_rx.try_recv() {
            let Some(entry) = self.entries.get_mut(&result.key) else {
                self.stale_results = self.stale_results.saturating_add(1);
                continue;
            };
            if !matches!(entry.state, AsyncDynamicTextureState::Pending)
                || result.key.resource_generation != self.resource_generation
                || result.key.session_generation != self.session_generation
            {
                self.stale_results = self.stale_results.saturating_add(1);
                continue;
            }
            entry.state = match &result.result {
                Ok(texture) => AsyncDynamicTextureState::Ready(texture.clone()),
                Err(_) => AsyncDynamicTextureState::Failed,
            };
            entry.delivery_pending = true;
            self.pending_entries = self.pending_entries.saturating_sub(1);
            match &result.result {
                Ok(texture) => {
                    self.ready_entries = self.ready_entries.saturating_add(1);
                    self.resident_bytes =
                        self.resident_bytes.saturating_add(texture.logical_bytes());
                    self.peak_resident_bytes = self.peak_resident_bytes.max(self.resident_bytes);
                }
                Err(_) => {
                    self.failed_entries = self.failed_entries.saturating_add(1);
                }
            }
        }
    }

    pub(crate) fn reset_session_generation(
        &mut self,
        session_generation: u64,
        accepting_requests: bool,
    ) {
        self.session_generation = session_generation;
        self.accepting_requests = accepting_requests;
        self.entries.clear();
        self.pending_entries = 0;
        self.ready_entries = 0;
        self.failed_entries = 0;
        self.resident_bytes = 0;
    }

    pub(crate) fn observation(&mut self) -> DynamicTextureCacheObservation {
        self.drain_worker_results();
        DynamicTextureCacheObservation {
            resource_generation: self.resource_generation,
            session_generation: self.session_generation,
            accepting_requests: self.accepting_requests,
            entries: self.entries.len(),
            pending: self.pending_entries,
            ready: self.ready_entries,
            failed: self.failed_entries,
            stale_results: self.stale_results,
            queue_full: self.queue_full,
            resident_bytes: self.resident_bytes,
            peak_resident_bytes: self.peak_resident_bytes,
        }
    }

    fn remove_entry(&mut self, key: &DynamicTextureKey) {
        let Some(entry) = self.entries.remove(key) else {
            return;
        };
        match entry.state {
            AsyncDynamicTextureState::Pending => {
                self.pending_entries = self.pending_entries.saturating_sub(1);
            }
            AsyncDynamicTextureState::Ready(texture) => {
                self.ready_entries = self.ready_entries.saturating_sub(1);
                self.resident_bytes = self.resident_bytes.saturating_sub(texture.logical_bytes());
            }
            AsyncDynamicTextureState::Failed => {
                self.failed_entries = self.failed_entries.saturating_sub(1);
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn downloaded_skin_count(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| {
                matches!(
                    entry.state,
                    AsyncDynamicTextureState::Ready(DownloadedDynamicTexture::Skin(_))
                )
            })
            .count()
    }

    #[cfg(test)]
    pub(crate) fn downloaded_texture_count(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| {
                matches!(
                    entry.state,
                    AsyncDynamicTextureState::Ready(DownloadedDynamicTexture::Profile(_))
                )
            })
            .count()
    }

    #[cfg(test)]
    fn stale_results(&self) -> u64 {
        self.stale_results
    }

    #[cfg(test)]
    fn queue_full(&self) -> u64 {
        self.queue_full
    }
}

impl DynamicTextureRequestKind {
    const fn profile_kind(self) -> Option<DynamicPlayerTextureKind> {
        match self {
            Self::Skin => None,
            Self::Cape => Some(DynamicPlayerTextureKind::Cape),
            Self::Elytra => Some(DynamicPlayerTextureKind::Elytra),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AsyncDynamicPlayerSkinResult {
    pub(crate) url: String,
    pub(crate) skin: Option<DynamicPlayerSkinImage>,
}

#[derive(Debug, Clone)]
pub(crate) struct AsyncDynamicPlayerTextureResult {
    pub(crate) kind: DynamicPlayerTextureKind,
    pub(crate) url: String,
    pub(crate) texture: Option<DynamicPlayerTextureImage>,
}

#[derive(Debug, Clone)]
struct DynamicTextureRequest {
    key: DynamicTextureKey,
}

#[derive(Debug)]
struct DynamicTextureResult {
    key: DynamicTextureKey,
    result: std::result::Result<DownloadedDynamicTexture, String>,
}

pub fn default_player_skin_cache_dir() -> PathBuf {
    std::env::temp_dir().join("bbb-native-player-skins")
}

pub(crate) fn player_skin_cache_path(cache_dir: &Path, handle: u64) -> PathBuf {
    cache_dir
        .join("skins")
        .join(format!("{:02x}", handle & 0xff))
        .join(format!("{handle:016x}.png"))
}

pub(crate) fn player_texture_cache_path(
    cache_dir: &Path,
    kind: DynamicPlayerTextureKind,
    handle: u64,
) -> PathBuf {
    cache_dir
        .join(kind.cache_directory())
        .join(format!("{:02x}", handle & 0xff))
        .join(format!("{handle:016x}.png"))
}

fn read_bounded_png_file(path: &Path, max_bytes: usize, label: &str) -> Result<Vec<u8>> {
    let file = fs::File::open(path).with_context(|| format!("open {label} {}", path.display()))?;
    let declared_bytes = file
        .metadata()
        .with_context(|| format!("read {label} metadata {}", path.display()))?
        .len();
    read_bounded_png_body(file, Some(declared_bytes), max_bytes, label)
        .with_context(|| format!("read {label} {}", path.display()))
}

fn read_bounded_png_body(
    reader: impl Read,
    declared_bytes: Option<u64>,
    max_bytes: usize,
    label: &str,
) -> Result<Vec<u8>> {
    let max_bytes_u64 = u64::try_from(max_bytes).context("dynamic texture byte limit overflow")?;
    if let Some(declared_bytes) = declared_bytes {
        if declared_bytes > max_bytes_u64 {
            bail!("{label} declares {declared_bytes} bytes, exceeding the {max_bytes} byte limit");
        }
    }
    let read_limit = max_bytes_u64
        .checked_add(1)
        .context("dynamic texture read limit overflow")?;
    let capacity = declared_bytes
        .and_then(|bytes| usize::try_from(bytes).ok())
        .unwrap_or(0)
        .min(max_bytes);
    let mut bytes = Vec::with_capacity(capacity);
    reader
        .take(read_limit)
        .read_to_end(&mut bytes)
        .with_context(|| format!("read {label} body"))?;
    validate_png_body_len(bytes.len(), max_bytes, label)?;
    Ok(bytes)
}

fn validate_png_body_len(actual_bytes: usize, max_bytes: usize, label: &str) -> Result<()> {
    if actual_bytes > max_bytes {
        bail!("{label} is {actual_bytes} bytes, exceeding the {max_bytes} byte limit");
    }
    Ok(())
}

fn write_player_skin_cache(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create player skin cache directory {}", parent.display()))?;
    }
    fs::write(path, bytes).with_context(|| format!("write player skin cache {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Cursor, Read, Write},
        net::TcpListener,
        sync::{
            atomic::{AtomicU64, AtomicUsize, Ordering},
            Arc,
        },
        thread,
        time::Duration,
    };

    static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn dynamic_skin_runtime_fetches_processes_writes_and_reuses_memory_cache() {
        let root = unique_temp_dir("dynamic-skin-runtime-memory");
        let mut runtime = DynamicPlayerSkinRuntime::new(&root);
        let png = rgba_png(64, 64, |x, y| [x as u8, y as u8, 7, 255]);
        let mut fetcher = StaticSkinFetcher::new(png);

        let first = runtime
            .get_or_fetch_player_skin(
                0xabc,
                "https://textures.minecraft.net/texture/skin",
                &mut fetcher,
            )
            .unwrap()
            .clone();
        let second = runtime
            .get_or_fetch_player_skin(
                0xabc,
                "https://textures.minecraft.net/texture/skin",
                &mut fetcher,
            )
            .unwrap()
            .clone();

        assert_eq!(first, second);
        assert_eq!(first.handle, 0xabc);
        assert_eq!(first.rgba.len(), 64 * 64 * 4);
        assert_eq!(fetcher.calls, 1);
        assert_eq!(runtime.cached_skin_count(), 1);
        assert!(player_skin_cache_path(&root, 0xabc).is_file());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dynamic_skin_runtime_reads_disk_cache_before_fetching() {
        let root = unique_temp_dir("dynamic-skin-runtime-disk");
        let handle = 0x1234;
        let path = player_skin_cache_path(&root, handle);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, rgba_png(64, 32, |x, y| [x as u8, y as u8, 11, 255])).unwrap();

        let mut runtime = DynamicPlayerSkinRuntime::new(&root);
        let mut fetcher = StaticSkinFetcher::new(Vec::new());
        let skin = runtime
            .get_or_fetch_player_skin(
                handle,
                "https://textures.minecraft.net/texture/cached-skin",
                &mut fetcher,
            )
            .unwrap();

        assert_eq!(skin.handle, handle);
        assert_eq!(skin.rgba.len(), 64 * 64 * 4);
        assert_eq!(fetcher.calls, 0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dynamic_skin_runtime_does_not_cache_failed_fetch_or_decode() {
        let root = unique_temp_dir("dynamic-skin-runtime-failed");
        let mut runtime = DynamicPlayerSkinRuntime::new(&root);
        let mut fetcher = StaticSkinFetcher::new(b"not a png".to_vec());

        let err = runtime
            .get_or_fetch_player_skin(
                7,
                "https://textures.minecraft.net/texture/bad",
                &mut fetcher,
            )
            .unwrap_err();

        assert!(err.to_string().contains("process player skin texture"));
        assert_eq!(fetcher.calls, 1);
        assert_eq!(runtime.cached_skin_count(), 0);
        assert!(!player_skin_cache_path(&root, 7).exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dynamic_texture_runtime_fetches_processes_writes_and_reuses_memory_cache() {
        let root = unique_temp_dir("dynamic-texture-runtime-memory");
        let mut runtime = DynamicPlayerTextureRuntime::new(&root);
        let png = rgba_png(64, 32, |x, y| [x as u8, y as u8, 29, 255]);
        let mut fetcher = StaticSkinFetcher::new(png);
        let url = "https://textures.minecraft.net/texture/cape";

        let first = runtime
            .get_or_fetch_player_texture(DynamicPlayerTextureKind::Cape, 0xcafe, url, &mut fetcher)
            .unwrap()
            .clone();
        let second = runtime
            .get_or_fetch_player_texture(DynamicPlayerTextureKind::Cape, 0xcafe, url, &mut fetcher)
            .unwrap()
            .clone();

        assert_eq!(first, second);
        assert_eq!(first.handle, 0xcafe);
        assert_eq!(first.size, [64, 32]);
        assert_eq!(first.rgba.len(), 64 * 32 * 4);
        assert_eq!(fetcher.calls, 1);
        assert_eq!(runtime.cached_texture_count(), 1);
        assert!(player_texture_cache_path(&root, DynamicPlayerTextureKind::Cape, 0xcafe).is_file());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dynamic_texture_runtime_keeps_cape_and_elytra_cache_paths_separate() {
        let root = unique_temp_dir("dynamic-texture-runtime-kinds");
        let handle = 0x1234;
        let cape_path = player_texture_cache_path(&root, DynamicPlayerTextureKind::Cape, handle);
        let elytra_path =
            player_texture_cache_path(&root, DynamicPlayerTextureKind::Elytra, handle);
        fs::create_dir_all(cape_path.parent().unwrap()).unwrap();
        fs::create_dir_all(elytra_path.parent().unwrap()).unwrap();
        fs::write(&cape_path, rgba_png(64, 32, |_, _| [1, 2, 3, 4])).unwrap();
        fs::write(&elytra_path, rgba_png(32, 32, |_, _| [5, 6, 7, 8])).unwrap();

        let mut runtime = DynamicPlayerTextureRuntime::new(&root);
        let mut fetcher = StaticSkinFetcher::new(Vec::new());
        let cape = runtime
            .get_or_fetch_player_texture(
                DynamicPlayerTextureKind::Cape,
                handle,
                "https://textures.minecraft.net/texture/shared",
                &mut fetcher,
            )
            .unwrap()
            .clone();
        let elytra = runtime
            .get_or_fetch_player_texture(
                DynamicPlayerTextureKind::Elytra,
                handle,
                "https://textures.minecraft.net/texture/shared",
                &mut fetcher,
            )
            .unwrap()
            .clone();

        assert_eq!(cape.size, [64, 32]);
        assert_eq!(elytra.size, [32, 32]);
        assert_eq!(fetcher.calls, 0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn async_dynamic_skin_runtime_downloads_processes_and_reuses_result() {
        let root = unique_temp_dir("async-dynamic-skin-runtime-ready");
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = AsyncDynamicTextureRuntime::new(
            0,
            &root,
            AsyncStaticSkinFetcher::new(
                rgba_png(64, 64, |x, y| [x as u8, y as u8, 19, 255]),
                calls.clone(),
            ),
        );
        let url = "https://textures.minecraft.net/texture/async-ready";

        runtime.queue_skin(0x55, url);
        let results = drain_until_async_skin_result(&mut runtime);
        runtime.queue_skin(0x55, url);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, url);
        let skin = results[0].skin.as_ref().unwrap();
        assert_eq!(skin.handle, 0x55);
        assert_eq!(skin.rgba.len(), 64 * 64 * 4);
        assert_eq!(runtime.downloaded_skin_count(), 1);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        let observation = runtime.observation();
        assert_eq!(
            (observation.pending, observation.ready, observation.failed),
            (0, 1, 0)
        );
        assert_eq!(observation.resident_bytes, 64 * 64 * 4);
        assert_eq!(observation.peak_resident_bytes, 64 * 64 * 4);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn async_dynamic_skin_runtime_caches_failed_download_or_decode() {
        let root = unique_temp_dir("async-dynamic-skin-runtime-failed");
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = AsyncDynamicTextureRuntime::new(
            0,
            &root,
            AsyncStaticSkinFetcher::new(b"not a png".to_vec(), calls.clone()),
        );
        let url = "https://textures.minecraft.net/texture/async-failed";

        runtime.queue_skin(0x66, url);
        let results = drain_until_async_skin_result(&mut runtime);
        runtime.queue_skin(0x66, url);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, url);
        assert_eq!(results[0].skin, None);
        assert_eq!(runtime.downloaded_skin_count(), 0);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        let observation = runtime.observation();
        assert_eq!(
            (observation.pending, observation.ready, observation.failed),
            (0, 0, 1)
        );
        assert_eq!(observation.resident_bytes, 0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn async_dynamic_texture_runtime_downloads_processes_and_reuses_result() {
        let root = unique_temp_dir("async-dynamic-texture-runtime-ready");
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = AsyncDynamicTextureRuntime::new(
            0,
            &root,
            AsyncStaticSkinFetcher::new(
                rgba_png(64, 32, |x, y| [x as u8, y as u8, 71, 255]),
                calls.clone(),
            ),
        );
        let url = "https://textures.minecraft.net/texture/async-cape";

        runtime.queue_profile(DynamicPlayerTextureKind::Cape, 0x77, url);
        let results = drain_until_async_texture_result(&mut runtime);
        runtime.queue_profile(DynamicPlayerTextureKind::Cape, 0x77, url);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, DynamicPlayerTextureKind::Cape);
        assert_eq!(results[0].url, url);
        let texture = results[0].texture.as_ref().unwrap();
        assert_eq!(texture.handle, 0x77);
        assert_eq!(texture.size, [64, 32]);
        assert_eq!(runtime.downloaded_texture_count(), 1);
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn async_dynamic_texture_runtime_caches_failed_download_or_decode() {
        let root = unique_temp_dir("async-dynamic-texture-runtime-failed");
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = AsyncDynamicTextureRuntime::new(
            0,
            &root,
            AsyncStaticSkinFetcher::new(b"not a png".to_vec(), calls.clone()),
        );
        let url = "https://textures.minecraft.net/texture/async-cape-failed";

        runtime.queue_profile(DynamicPlayerTextureKind::Cape, 0x78, url);
        let results = drain_until_async_texture_result(&mut runtime);
        runtime.queue_profile(DynamicPlayerTextureKind::Cape, 0x78, url);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, DynamicPlayerTextureKind::Cape);
        assert_eq!(results[0].url, url);
        assert_eq!(results[0].texture, None);
        assert_eq!(runtime.downloaded_texture_count(), 0);
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unified_dynamic_texture_runtime_discards_previous_generation_result() {
        let root = unique_temp_dir("async-dynamic-texture-stale-generation");
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = AsyncDynamicTextureRuntime::new(
            7,
            &root,
            AsyncStaticSkinFetcher::new(
                rgba_png(64, 64, |x, y| [x as u8, y as u8, 93, 255]),
                calls.clone(),
            ),
        );
        let url = "https://textures.minecraft.net/texture/stale-generation";

        assert!(runtime.queue_skin(0x91, url));
        for _ in 0..100 {
            if calls.load(Ordering::Relaxed) != 0 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        runtime.reset_session_generation(8, true);
        for _ in 0..100 {
            assert!(runtime.drain_skin_results().is_empty());
            if runtime.stale_results() == 1 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(runtime.stale_results(), 1);
        assert_eq!(runtime.downloaded_skin_count(), 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unified_dynamic_texture_runtime_bounds_all_pending_entries() {
        let root = unique_temp_dir("async-dynamic-texture-bounded");
        let calls = Arc::new(AtomicUsize::new(0));
        let (release_tx, release_rx) = mpsc::channel();
        let mut runtime = AsyncDynamicTextureRuntime::with_limits(
            3,
            &root,
            BlockingSkinFetcher::new(
                rgba_png(64, 64, |x, y| [x as u8, y as u8, 17, 255]),
                calls.clone(),
                release_rx,
            ),
            1,
            1,
        );

        assert!(runtime.queue_skin(1, "https://textures.minecraft.net/texture/one"));
        for _ in 0..100 {
            if calls.load(Ordering::Relaxed) == 1 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(!runtime.queue_profile(
            DynamicPlayerTextureKind::Cape,
            2,
            "https://textures.minecraft.net/texture/two",
        ));
        assert_eq!(runtime.entries.len(), 1);
        assert_eq!(runtime.queue_full(), 1);
        release_tx.send(()).unwrap();
        assert_eq!(drain_until_async_skin_result(&mut runtime).len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unified_dynamic_texture_observation_tracks_ready_eviction_without_byte_growth() {
        let root = unique_temp_dir("async-dynamic-texture-observed-eviction");
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = AsyncDynamicTextureRuntime::with_limits(
            5,
            &root,
            AsyncStaticSkinFetcher::new(
                rgba_png(64, 64, |x, y| [x as u8, y as u8, 31, 255]),
                calls.clone(),
            ),
            1,
            1,
        );

        assert!(runtime.queue_skin(1, "https://textures.minecraft.net/texture/one"));
        assert_eq!(drain_until_async_skin_result(&mut runtime).len(), 1);
        assert!(runtime.queue_skin(2, "https://textures.minecraft.net/texture/two"));
        assert_eq!(drain_until_async_skin_result(&mut runtime).len(), 1);

        let observation = runtime.observation();
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        assert_eq!(observation.entries, 1);
        assert_eq!(
            (observation.pending, observation.ready, observation.failed),
            (0, 1, 0)
        );
        assert_eq!(observation.resident_bytes, 64 * 64 * 4);
        assert_eq!(observation.peak_resident_bytes, 64 * 64 * 4);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn http_skin_fetcher_reads_successful_response_bytes() {
        let body = b"player skin bytes".to_vec();
        let url = spawn_http_response(200, "OK", body.clone());
        let mut fetcher = HttpSkinPngFetcher::new().unwrap();

        assert_eq!(
            fetcher
                .fetch_skin_png(&url, MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES)
                .unwrap(),
            body
        );
    }

    #[test]
    fn http_skin_fetcher_rejects_non_success_status() {
        let url = spawn_http_response(404, "Not Found", b"missing".to_vec());
        let mut fetcher = HttpSkinPngFetcher::new().unwrap();

        let err = fetcher
            .fetch_skin_png(&url, MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES)
            .unwrap_err();

        assert!(err.to_string().contains("fetch HTTP player skin"));
    }

    #[test]
    fn bounded_dynamic_texture_body_rejects_declared_and_streamed_overflow() {
        let declared = read_bounded_png_body(Cursor::new(b"ok"), Some(5), 4, "declared fixture")
            .unwrap_err()
            .to_string();
        assert!(declared.contains("declares 5 bytes"), "{declared}");

        let streamed = read_bounded_png_body(Cursor::new(b"12345"), None, 4, "streamed fixture")
            .unwrap_err()
            .to_string();
        assert!(streamed.contains("is 5 bytes"), "{streamed}");
    }

    #[test]
    fn oversized_cached_skin_is_rejected_before_body_allocation_or_fetch() {
        let root = unique_temp_dir("dynamic-skin-runtime-oversized-cache");
        let handle = 0xdead;
        let path = player_skin_cache_path(&root, handle);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .unwrap();
        file.set_len(MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES as u64 + 1)
            .unwrap();
        let mut runtime = DynamicPlayerSkinRuntime::new(&root);
        let mut fetcher = StaticSkinFetcher::new(Vec::new());

        let err = format!(
            "{:#}",
            runtime
                .get_or_fetch_player_skin(
                    handle,
                    "https://textures.minecraft.net/texture/oversized-cache",
                    &mut fetcher,
                )
                .unwrap_err()
        );

        assert!(err.contains("declares 1048577 bytes"), "{err}");
        assert_eq!(fetcher.calls, 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn oversized_fetched_skin_is_not_written_to_cache() {
        let root = unique_temp_dir("dynamic-skin-runtime-oversized-fetch");
        let handle = 0xbeef;
        let mut runtime = DynamicPlayerSkinRuntime::new(&root);
        let mut fetcher = StaticSkinFetcher::new(vec![0_u8; MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES + 1]);

        let err = runtime
            .get_or_fetch_player_skin(
                handle,
                "https://textures.minecraft.net/texture/oversized-fetch",
                &mut fetcher,
            )
            .unwrap_err()
            .to_string();

        assert!(err.contains("exceeding the 1048576 byte limit"), "{err}");
        assert_eq!(fetcher.calls, 1);
        assert!(!player_skin_cache_path(&root, handle).exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dynamic_texture_transport_does_not_restore_unbounded_body_reads() {
        let production = include_str!("skin_runtime.rs")
            .split("#[cfg(test)]\nmod tests")
            .next()
            .expect("dynamic texture production source");
        assert!(!production.contains(".bytes()"));
        assert!(!production.contains("fs::read("));
        assert!(production.contains(".take(read_limit)"));
    }

    struct StaticSkinFetcher {
        bytes: Vec<u8>,
        calls: usize,
    }

    impl StaticSkinFetcher {
        fn new(bytes: Vec<u8>) -> Self {
            Self { bytes, calls: 0 }
        }
    }

    impl SkinPngFetcher for StaticSkinFetcher {
        fn fetch_skin_png(&mut self, _url: &str, _max_bytes: usize) -> Result<Vec<u8>> {
            self.calls += 1;
            Ok(self.bytes.clone())
        }
    }

    struct AsyncStaticSkinFetcher {
        bytes: Vec<u8>,
        calls: Arc<AtomicUsize>,
    }

    struct BlockingSkinFetcher {
        bytes: Vec<u8>,
        calls: Arc<AtomicUsize>,
        release_rx: mpsc::Receiver<()>,
    }

    impl BlockingSkinFetcher {
        fn new(bytes: Vec<u8>, calls: Arc<AtomicUsize>, release_rx: mpsc::Receiver<()>) -> Self {
            Self {
                bytes,
                calls,
                release_rx,
            }
        }
    }

    impl SkinPngFetcher for BlockingSkinFetcher {
        fn fetch_skin_png(&mut self, _url: &str, _max_bytes: usize) -> Result<Vec<u8>> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.release_rx.recv().context("release test skin fetch")?;
            Ok(self.bytes.clone())
        }
    }

    impl AsyncStaticSkinFetcher {
        fn new(bytes: Vec<u8>, calls: Arc<AtomicUsize>) -> Self {
            Self { bytes, calls }
        }
    }

    impl SkinPngFetcher for AsyncStaticSkinFetcher {
        fn fetch_skin_png(&mut self, _url: &str, _max_bytes: usize) -> Result<Vec<u8>> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(self.bytes.clone())
        }
    }

    fn rgba_png(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let mut image = image::RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                image.put_pixel(x, y, image::Rgba(pixel(x, y)));
            }
        }
        let mut cursor = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .unwrap();
        cursor.into_inner()
    }

    fn drain_until_async_skin_result(
        runtime: &mut AsyncDynamicTextureRuntime,
    ) -> Vec<AsyncDynamicPlayerSkinResult> {
        for _ in 0..100 {
            let results = runtime.drain_skin_results();
            if !results.is_empty() {
                return results;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out waiting for async skin result");
    }

    fn drain_until_async_texture_result(
        runtime: &mut AsyncDynamicTextureRuntime,
    ) -> Vec<AsyncDynamicPlayerTextureResult> {
        for _ in 0..100 {
            let results = runtime.drain_profile_results();
            if !results.is_empty() {
                return results;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out waiting for async player texture result");
    }

    fn spawn_http_response(status: u16, reason: &str, body: Vec<u8>) -> String {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        let reason = reason.to_string();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 1024];
            let _ = stream.read(&mut request);
            write!(
                stream,
                "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        });
        format!("http://{addr}/skin.png")
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let id = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }
}
