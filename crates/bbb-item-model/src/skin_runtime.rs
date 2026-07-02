use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use bbb_render_types::{
    decode_dynamic_player_skin_png, decode_dynamic_player_texture_png, DynamicPlayerSkinImage,
    DynamicPlayerTextureImage,
};

pub(crate) trait SkinPngFetcher {
    fn fetch_skin_png(&mut self, url: &str) -> Result<Vec<u8>>;
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
    fn fetch_skin_png(&mut self, url: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(url)
            .send()
            .with_context(|| format!("request HTTP player skin {url}"))?
            .error_for_status()
            .with_context(|| format!("fetch HTTP player skin {url}"))?;
        Ok(response
            .bytes()
            .with_context(|| format!("read HTTP player skin {url}"))?
            .to_vec())
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
            let bytes = self.cached_or_fetched_bytes(handle, url, fetcher)?;
            let skin = decode_dynamic_player_skin_png(handle, &bytes)
                .with_context(|| format!("process player skin texture from {url}"))?;
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
    ) -> Result<Vec<u8>> {
        let path = self.player_skin_path(handle);
        if path.is_file() {
            return fs::read(&path)
                .with_context(|| format!("read cached player skin {}", path.display()));
        }

        let bytes = fetcher
            .fetch_skin_png(url)
            .with_context(|| format!("fetch player skin texture from {url}"))?;
        if let Err(err) = write_player_skin_cache(&path, &bytes) {
            tracing::warn!(
                ?err,
                path = %path.display(),
                "failed to cache player skin texture"
            );
        }
        Ok(bytes)
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
            let bytes = self.cached_or_fetched_bytes(kind, handle, url, fetcher)?;
            let texture = decode_dynamic_player_texture_png(handle, &bytes)
                .with_context(|| format!("process player {kind:?} texture from {url}"))?;
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
    ) -> Result<Vec<u8>> {
        let path = self.player_texture_path(kind, handle);
        if path.is_file() {
            return fs::read(&path)
                .with_context(|| format!("read cached player {kind:?} {}", path.display()));
        }

        let bytes = fetcher
            .fetch_skin_png(url)
            .with_context(|| format!("fetch player {kind:?} texture from {url}"))?;
        if let Err(err) = write_player_skin_cache(&path, &bytes) {
            tracing::warn!(
                ?err,
                path = %path.display(),
                kind = ?kind,
                "failed to cache player profile texture"
            );
        }
        Ok(bytes)
    }

    fn player_texture_path(&self, kind: DynamicPlayerTextureKind, handle: u64) -> PathBuf {
        player_texture_cache_path(&self.cache_dir, kind, handle)
    }
}

#[derive(Debug)]
pub(crate) struct AsyncDynamicPlayerSkinRuntime {
    entries: HashMap<String, AsyncDynamicPlayerSkinEntry>,
    request_tx: Sender<DynamicPlayerSkinRequest>,
    result_rx: Receiver<DynamicPlayerSkinResult>,
}

impl AsyncDynamicPlayerSkinRuntime {
    pub(crate) fn new(
        cache_dir: impl Into<PathBuf>,
        fetcher: impl SkinPngFetcher + Send + 'static,
    ) -> Self {
        let (request_tx, request_rx) = mpsc::channel::<DynamicPlayerSkinRequest>();
        let (result_tx, result_rx) = mpsc::channel::<DynamicPlayerSkinResult>();
        let cache_dir = cache_dir.into();
        thread::spawn(move || {
            let mut runtime = DynamicPlayerSkinRuntime::new(cache_dir);
            let mut fetcher = fetcher;
            for request in request_rx {
                let result = runtime
                    .get_or_fetch_player_skin(request.handle, &request.url, &mut fetcher)
                    .cloned()
                    .map_err(|err| err.to_string());
                if result_tx
                    .send(DynamicPlayerSkinResult {
                        url: request.url,
                        result,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            entries: HashMap::new(),
            request_tx,
            result_rx,
        }
    }

    pub(crate) fn queue(&mut self, handle: u64, url: &str) {
        self.drain_results();
        if self.entries.contains_key(url) {
            return;
        }

        self.entries
            .insert(url.to_string(), AsyncDynamicPlayerSkinEntry::Pending);
        let request = DynamicPlayerSkinRequest {
            handle,
            url: url.to_string(),
        };
        if self.request_tx.send(request).is_err() {
            self.entries
                .insert(url.to_string(), AsyncDynamicPlayerSkinEntry::Failed);
        }
    }

    pub(crate) fn drain_results(&mut self) -> Vec<AsyncDynamicPlayerSkinResult> {
        let mut drained = Vec::new();
        while let Ok(result) = self.result_rx.try_recv() {
            let entry = match &result.result {
                Ok(skin) => AsyncDynamicPlayerSkinEntry::Downloaded(skin.clone()),
                Err(_) => AsyncDynamicPlayerSkinEntry::Failed,
            };
            self.entries.insert(result.url.clone(), entry);
            drained.push(AsyncDynamicPlayerSkinResult {
                url: result.url,
                skin: result.result.ok(),
            });
        }
        drained
    }

    #[cfg(test)]
    pub(crate) fn downloaded_skin_count(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| matches!(entry, AsyncDynamicPlayerSkinEntry::Downloaded(_)))
            .count()
    }
}

#[derive(Debug)]
pub(crate) struct AsyncDynamicPlayerTextureRuntime {
    entries: HashMap<(DynamicPlayerTextureKind, String), AsyncDynamicPlayerTextureEntry>,
    request_tx: Sender<DynamicPlayerTextureRequest>,
    result_rx: Receiver<DynamicPlayerTextureResult>,
}

impl AsyncDynamicPlayerTextureRuntime {
    pub(crate) fn new(
        cache_dir: impl Into<PathBuf>,
        fetcher: impl SkinPngFetcher + Send + 'static,
    ) -> Self {
        let (request_tx, request_rx) = mpsc::channel::<DynamicPlayerTextureRequest>();
        let (result_tx, result_rx) = mpsc::channel::<DynamicPlayerTextureResult>();
        let cache_dir = cache_dir.into();
        thread::spawn(move || {
            let mut runtime = DynamicPlayerTextureRuntime::new(cache_dir);
            let mut fetcher = fetcher;
            for request in request_rx {
                let result = runtime
                    .get_or_fetch_player_texture(
                        request.kind,
                        request.handle,
                        &request.url,
                        &mut fetcher,
                    )
                    .cloned()
                    .map_err(|err| err.to_string());
                if result_tx
                    .send(DynamicPlayerTextureResult {
                        kind: request.kind,
                        url: request.url,
                        result,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            entries: HashMap::new(),
            request_tx,
            result_rx,
        }
    }

    pub(crate) fn queue(&mut self, kind: DynamicPlayerTextureKind, handle: u64, url: &str) {
        self.drain_results();
        let key = (kind, url.to_string());
        if self.entries.contains_key(&key) {
            return;
        }

        self.entries
            .insert(key, AsyncDynamicPlayerTextureEntry::Pending);
        let request = DynamicPlayerTextureRequest {
            kind,
            handle,
            url: url.to_string(),
        };
        if self.request_tx.send(request).is_err() {
            self.entries.insert(
                (kind, url.to_string()),
                AsyncDynamicPlayerTextureEntry::Failed,
            );
        }
    }

    pub(crate) fn drain_results(&mut self) -> Vec<AsyncDynamicPlayerTextureResult> {
        let mut drained = Vec::new();
        while let Ok(result) = self.result_rx.try_recv() {
            let entry = match &result.result {
                Ok(texture) => AsyncDynamicPlayerTextureEntry::Downloaded(texture.clone()),
                Err(_) => AsyncDynamicPlayerTextureEntry::Failed,
            };
            self.entries
                .insert((result.kind, result.url.clone()), entry);
            drained.push(AsyncDynamicPlayerTextureResult {
                kind: result.kind,
                url: result.url,
                texture: result.result.ok(),
            });
        }
        drained
    }

    #[cfg(test)]
    pub(crate) fn downloaded_texture_count(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| matches!(entry, AsyncDynamicPlayerTextureEntry::Downloaded(_)))
            .count()
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
struct DynamicPlayerSkinRequest {
    handle: u64,
    url: String,
}

#[derive(Debug, Clone)]
struct DynamicPlayerTextureRequest {
    kind: DynamicPlayerTextureKind,
    handle: u64,
    url: String,
}

#[derive(Debug)]
struct DynamicPlayerSkinResult {
    url: String,
    result: std::result::Result<DynamicPlayerSkinImage, String>,
}

#[derive(Debug)]
struct DynamicPlayerTextureResult {
    kind: DynamicPlayerTextureKind,
    url: String,
    result: std::result::Result<DynamicPlayerTextureImage, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AsyncDynamicPlayerSkinEntry {
    Pending,
    Downloaded(DynamicPlayerSkinImage),
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AsyncDynamicPlayerTextureEntry {
    Pending,
    Downloaded(DynamicPlayerTextureImage),
    Failed,
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
        let mut runtime = AsyncDynamicPlayerSkinRuntime::new(
            &root,
            AsyncStaticSkinFetcher::new(
                rgba_png(64, 64, |x, y| [x as u8, y as u8, 19, 255]),
                calls.clone(),
            ),
        );
        let url = "https://textures.minecraft.net/texture/async-ready";

        runtime.queue(0x55, url);
        let results = drain_until_async_skin_result(&mut runtime);
        runtime.queue(0x55, url);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, url);
        let skin = results[0].skin.as_ref().unwrap();
        assert_eq!(skin.handle, 0x55);
        assert_eq!(skin.rgba.len(), 64 * 64 * 4);
        assert_eq!(runtime.downloaded_skin_count(), 1);
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn async_dynamic_skin_runtime_caches_failed_download_or_decode() {
        let root = unique_temp_dir("async-dynamic-skin-runtime-failed");
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = AsyncDynamicPlayerSkinRuntime::new(
            &root,
            AsyncStaticSkinFetcher::new(b"not a png".to_vec(), calls.clone()),
        );
        let url = "https://textures.minecraft.net/texture/async-failed";

        runtime.queue(0x66, url);
        let results = drain_until_async_skin_result(&mut runtime);
        runtime.queue(0x66, url);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, url);
        assert_eq!(results[0].skin, None);
        assert_eq!(runtime.downloaded_skin_count(), 0);
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn async_dynamic_texture_runtime_downloads_processes_and_reuses_result() {
        let root = unique_temp_dir("async-dynamic-texture-runtime-ready");
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = AsyncDynamicPlayerTextureRuntime::new(
            &root,
            AsyncStaticSkinFetcher::new(
                rgba_png(64, 32, |x, y| [x as u8, y as u8, 71, 255]),
                calls.clone(),
            ),
        );
        let url = "https://textures.minecraft.net/texture/async-cape";

        runtime.queue(DynamicPlayerTextureKind::Cape, 0x77, url);
        let results = drain_until_async_texture_result(&mut runtime);
        runtime.queue(DynamicPlayerTextureKind::Cape, 0x77, url);

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
        let mut runtime = AsyncDynamicPlayerTextureRuntime::new(
            &root,
            AsyncStaticSkinFetcher::new(b"not a png".to_vec(), calls.clone()),
        );
        let url = "https://textures.minecraft.net/texture/async-cape-failed";

        runtime.queue(DynamicPlayerTextureKind::Cape, 0x78, url);
        let results = drain_until_async_texture_result(&mut runtime);
        runtime.queue(DynamicPlayerTextureKind::Cape, 0x78, url);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, DynamicPlayerTextureKind::Cape);
        assert_eq!(results[0].url, url);
        assert_eq!(results[0].texture, None);
        assert_eq!(runtime.downloaded_texture_count(), 0);
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn http_skin_fetcher_reads_successful_response_bytes() {
        let body = b"player skin bytes".to_vec();
        let url = spawn_http_response(200, "OK", body.clone());
        let mut fetcher = HttpSkinPngFetcher::new().unwrap();

        assert_eq!(fetcher.fetch_skin_png(&url).unwrap(), body);
    }

    #[test]
    fn http_skin_fetcher_rejects_non_success_status() {
        let url = spawn_http_response(404, "Not Found", b"missing".to_vec());
        let mut fetcher = HttpSkinPngFetcher::new().unwrap();

        let err = fetcher.fetch_skin_png(&url).unwrap_err();

        assert!(err.to_string().contains("fetch HTTP player skin"));
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
        fn fetch_skin_png(&mut self, _url: &str) -> Result<Vec<u8>> {
            self.calls += 1;
            Ok(self.bytes.clone())
        }
    }

    struct AsyncStaticSkinFetcher {
        bytes: Vec<u8>,
        calls: Arc<AtomicUsize>,
    }

    impl AsyncStaticSkinFetcher {
        fn new(bytes: Vec<u8>, calls: Arc<AtomicUsize>) -> Self {
            Self { bytes, calls }
        }
    }

    impl SkinPngFetcher for AsyncStaticSkinFetcher {
        fn fetch_skin_png(&mut self, _url: &str) -> Result<Vec<u8>> {
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
        runtime: &mut AsyncDynamicPlayerSkinRuntime,
    ) -> Vec<AsyncDynamicPlayerSkinResult> {
        for _ in 0..100 {
            let results = runtime.drain_results();
            if !results.is_empty() {
                return results;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out waiting for async skin result");
    }

    fn drain_until_async_texture_result(
        runtime: &mut AsyncDynamicPlayerTextureRuntime,
    ) -> Vec<AsyncDynamicPlayerTextureResult> {
        for _ in 0..100 {
            let results = runtime.drain_results();
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
