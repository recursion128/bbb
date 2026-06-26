use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use bbb_renderer::{decode_dynamic_player_skin_png, DynamicPlayerSkinImage};

pub(crate) trait SkinPngFetcher {
    fn fetch_skin_png(&mut self, url: &str) -> Result<Vec<u8>>;
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

pub(crate) fn player_skin_cache_path(cache_dir: &Path, handle: u64) -> PathBuf {
    cache_dir
        .join("skins")
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
        sync::atomic::{AtomicU64, Ordering},
        thread,
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
