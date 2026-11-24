#[cfg(target_arch = "wasm32")]
use std::sync::mpsc::Sender;
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    time::Duration,
};
#[cfg(not(target_arch = "wasm32"))]
use std::{fs, io::Read, path::Path, thread};

#[cfg(not(target_arch = "wasm32"))]
use anyhow::ensure;
use anyhow::{bail, Context, Result};
#[cfg(not(target_arch = "wasm32"))]
use bbb_protocol::vanilla_profile_texture_cache_hash;
use bbb_protocol::{
    player_names::valid_player_name, vanilla_profile_texture_hash, DynamicTextureFetchRequest,
    DynamicTextureFetchResult, JavaString, ProfileResolutionKey, ProfileResolutionResult,
    ResolvedGameProfile, MAX_PROFILE_TEXTURE_URL_BYTES,
};
use serde::Deserialize;
use uuid::Uuid;

mod json;
use json::SessionProfileResponse;

const MAX_MOJANG_PROFILE_RESPONSE_BYTES: usize = 256 * 1024;
const PROFILE_HTTP_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_PROFILE_CACHE_ENTRIES: usize = 4096;
const NEGATIVE_PROFILE_CACHE_TTL: Duration = Duration::from_secs(60);
#[cfg(not(target_arch = "wasm32"))]
const MAX_NATIVE_CONCURRENT_PROFILE_JOBS: usize = 8;
const DEFAULT_HTTP_429_BACKOFF: Duration = Duration::from_secs(5);
const MAX_HTTP_429_BACKOFF: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy)]
struct CacheTimestamp {
    #[cfg(not(target_arch = "wasm32"))]
    instant: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    millis: f64,
}

impl CacheTimestamp {
    fn now() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {
                instant: std::time::Instant::now(),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self {
                millis: js_sys::Date::now(),
            }
        }
    }

    fn elapsed(self) -> Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.instant.elapsed()
        }
        #[cfg(target_arch = "wasm32")]
        {
            let diff = (js_sys::Date::now() - self.millis).max(0.0);
            Duration::from_millis(diff as u64)
        }
    }
}

#[derive(Debug, Clone)]
struct CachedProfileEntry<T> {
    value: Option<T>,
    cached_at: CacheTimestamp,
}

impl<T> CachedProfileEntry<T> {
    fn new(value: Option<T>) -> Self {
        Self {
            value,
            cached_at: CacheTimestamp::now(),
        }
    }

    fn is_fresh(&self, negative_ttl: Duration) -> bool {
        self.value.is_some() || self.cached_at.elapsed() < negative_ttl
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NameAndId {
    uuid: Uuid,
    name: JavaString,
}

#[derive(Deserialize)]
struct NameToIdResponse {
    id: String,
    #[serde(deserialize_with = "bbb_protocol::JavaString::deserialize_json_string")]
    name: JavaString,
}

#[derive(Debug)]
struct ProfileResolutionCache {
    by_name: HashMap<JavaString, CachedProfileEntry<Uuid>>,
    name_order: VecDeque<JavaString>,
    by_id: HashMap<Uuid, CachedProfileEntry<ResolvedGameProfile>>,
    id_order: VecDeque<Uuid>,
    max_entries: usize,
    negative_ttl: Duration,
}

impl Default for ProfileResolutionCache {
    fn default() -> Self {
        Self::with_limits(MAX_PROFILE_CACHE_ENTRIES, NEGATIVE_PROFILE_CACHE_TTL)
    }
}

impl ProfileResolutionCache {
    fn with_limits(max_entries: usize, negative_ttl: Duration) -> Self {
        Self {
            by_name: HashMap::new(),
            name_order: VecDeque::new(),
            by_id: HashMap::new(),
            id_order: VecDeque::new(),
            max_entries: max_entries.max(1),
            negative_ttl,
        }
    }

    fn cached_name(&self, name: &str) -> Option<Option<Uuid>> {
        let entry = self.by_name.get(&JavaString::from(name))?;
        entry.is_fresh(self.negative_ttl).then_some(entry.value)
    }

    fn insert_name_entry(&mut self, key: JavaString, value: Option<Uuid>) {
        if let Some(existing) = self.by_name.get_mut(&key) {
            *existing = CachedProfileEntry::new(value);
            return;
        }
        while self.by_name.len() >= self.max_entries {
            let Some(oldest) = self.name_order.pop_front() else {
                break;
            };
            self.by_name.remove(&oldest);
        }
        self.name_order.push_back(key.clone());
        self.by_name.insert(key, CachedProfileEntry::new(value));
    }

    fn record_name(&mut self, queried_name: &str, fetched: Option<NameAndId>) -> Option<Uuid> {
        let uuid = fetched.as_ref().map(|entry| entry.uuid);
        self.insert_name_entry(queried_name.into(), uuid);
        if let Some(entry) = fetched {
            self.insert_name_entry(entry.name, Some(entry.uuid));
        }
        uuid
    }

    fn has_id(&self, id: Uuid) -> bool {
        self.by_id
            .get(&id)
            .is_some_and(|entry| entry.is_fresh(self.negative_ttl))
    }

    fn get_id(&self, id: Uuid) -> Option<ResolvedGameProfile> {
        self.by_id
            .get(&id)
            .filter(|entry| entry.is_fresh(self.negative_ttl))
            .and_then(|entry| entry.value.clone())
    }

    fn record_id(&mut self, id: Uuid, profile: Option<ResolvedGameProfile>) {
        if let Some(existing) = self.by_id.get_mut(&id) {
            *existing = CachedProfileEntry::new(profile);
            return;
        }
        while self.by_id.len() >= self.max_entries {
            let Some(oldest) = self.id_order.pop_front() else {
                break;
            };
            self.by_id.remove(&oldest);
        }
        self.id_order.push_back(id);
        self.by_id.insert(id, CachedProfileEntry::new(profile));
    }

    fn try_lookup(&self, key: &ProfileResolutionKey) -> Option<Option<ResolvedGameProfile>> {
        match key {
            ProfileResolutionKey::Uuid(id) => self.has_id(*id).then(|| self.get_id(*id)),
            ProfileResolutionKey::Name(name) => {
                if !is_resolvable_player_name(name) {
                    return Some(None);
                }
                match self.cached_name(name)? {
                    None => Some(None),
                    Some(id) => self.has_id(id).then(|| self.get_id(id)),
                }
            }
        }
    }
}

/// Vanilla `StringUtil.isValidPlayerName` (`StringUtil.java:66-68`) permits
/// empty strings and ASCII punctuation (`ProfileResolver.java:49-50`), so
/// Mojang HTTP lookups additionally reject empty names before URL construction.
fn is_resolvable_player_name(name: &str) -> bool {
    !name.is_empty() && valid_player_name(name)
}

/// Strictly percent-encodes a single URL path segment so characters such as
/// `/`, `?`, `#`, `%`, and `.` cannot perform path traversal or query/fragment
/// injection.
fn encode_strict_path_segment(segment: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(segment.len());
    for &byte in segment.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
    out
}

fn build_mojang_name_lookup_url(api_base_url: &str, name: &str) -> Result<String> {
    if !is_resolvable_player_name(name) {
        bail!("invalid player name for Mojang profile lookup");
    }
    let encoded_name = encode_strict_path_segment(name);
    let base = api_base_url.trim_end_matches('/');
    Ok(format!("{base}/users/profiles/minecraft/{encoded_name}"))
}

#[derive(Debug, Default, Clone)]
struct RateLimitBackoff {
    until: Arc<Mutex<Option<bbb_client_clock::Instant>>>,
}

impl RateLimitBackoff {
    async fn wait_if_needed(&self) {
        let sleep_duration = {
            let guard = self
                .until
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard
                .and_then(|deadline| {
                    deadline.checked_duration_since(bbb_client_clock::Instant::now())
                })
                .map(|d| d.min(MAX_HTTP_429_BACKOFF))
        };
        if let Some(delay) = sleep_duration {
            #[cfg(not(target_arch = "wasm32"))]
            tokio::time::sleep(delay).await;
            #[cfg(target_arch = "wasm32")]
            crate::transport::sleep_duration(delay).await;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn record_429(&self, headers: &reqwest::header::HeaderMap) -> Duration {
        let retry_after = headers
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|val| val.to_str().ok());
        self.record_429_retry_after(retry_after)
    }

    fn record_429_retry_after(&self, retry_after_header: Option<&str>) -> Duration {
        let retry_after = retry_after_header
            .and_then(parse_retry_after_value)
            .unwrap_or(DEFAULT_HTTP_429_BACKOFF);
        let clamped = retry_after.clamp(Duration::from_secs(1), MAX_HTTP_429_BACKOFF);
        let deadline = bbb_client_clock::Instant::now() + clamped;
        let mut guard = self
            .until
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.is_none_or(|existing| deadline > existing) {
            *guard = Some(deadline);
        }
        clamped
    }
}

fn parse_retry_after_value(raw: &str) -> Option<Duration> {
    parse_retry_after_value_at(raw, current_unix_seconds())
}

fn current_unix_seconds() -> i64 {
    bbb_client_clock::system_now()
        .duration_since(bbb_client_clock::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .unwrap_or(0)
}

fn parse_retry_after_value_at(raw: &str, now_unix_secs: i64) -> Option<Duration> {
    let trimmed = raw.trim();
    if let Ok(secs) = trimmed.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    let retry_at = chrono::DateTime::parse_from_rfc2822(trimmed).ok()?;
    let delta_secs = retry_at.timestamp().saturating_sub(now_unix_secs).max(0) as u64;
    Some(Duration::from_secs(delta_secs))
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub(crate) struct MojangProfileHttpClient {
    client: reqwest::Client,
    api_base_url: String,
    session_base_url: String,
    rate_limit: RateLimitBackoff,
}

#[cfg(not(target_arch = "wasm32"))]
impl MojangProfileHttpClient {
    pub(crate) fn new() -> Result<Self> {
        Self::with_base_urls("https://api.mojang.com", "https://sessionserver.mojang.com")
    }

    fn with_base_urls(api_base_url: &str, session_base_url: &str) -> Result<Self> {
        crate::install_http_crypto_provider();
        let api_host = url::Url::parse(api_base_url)
            .ok()
            .and_then(|parsed| parsed.host_str().map(str::to_owned));
        let session_host = url::Url::parse(session_base_url)
            .ok()
            .and_then(|parsed| parsed.host_str().map(str::to_owned));
        let redirect_policy = reqwest::redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() >= 10 {
                attempt.error("too many Mojang profile redirects")
            } else if !matches!(attempt.url().scheme(), "http" | "https") {
                attempt.error("Mojang profile redirect must use HTTP or HTTPS")
            } else if attempt.url().host_str().is_none_or(|host| {
                api_host.as_deref() != Some(host) && session_host.as_deref() != Some(host)
            }) {
                attempt.error("Mojang profile redirect left allowed host")
            } else {
                attempt.follow()
            }
        });
        Ok(Self {
            client: reqwest::Client::builder()
                .no_proxy()
                .redirect(redirect_policy)
                .timeout(PROFILE_HTTP_TIMEOUT)
                .user_agent("bbb-client")
                .build()
                .context("create profile resolver HTTP client")?,
            api_base_url: api_base_url.trim_end_matches('/').to_string(),
            session_base_url: session_base_url.trim_end_matches('/').to_string(),
            rate_limit: RateLimitBackoff::default(),
        })
    }

    async fn get_optional_json<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
    ) -> Result<Option<T>> {
        self.rate_limit.wait_if_needed().await;
        let response = self
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("request HTTP profile {url}"))?;
        let status = response.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let backoff = self.rate_limit.record_429(response.headers());
            bail!(
                "HTTP 429 rate-limited fetching {url} (backing off {}s)",
                backoff.as_secs()
            );
        }
        if status == reqwest::StatusCode::NO_CONTENT || status == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let mut response = response
            .error_for_status()
            .with_context(|| format!("fetch HTTP profile {url}"))?;
        let body = read_bounded_response_bytes(
            &mut response,
            MAX_MOJANG_PROFILE_RESPONSE_BYTES,
            &format!("HTTP profile {url}"),
        )
        .await?;
        serde_json::from_slice(&body)
            .with_context(|| format!("decode HTTP profile JSON from {url}"))
            .map(Some)
    }

    async fn fetch_id_by_name(&self, name: &str) -> Result<Option<NameAndId>> {
        if !is_resolvable_player_name(name) {
            return Ok(None);
        }
        let url = build_mojang_name_lookup_url(&self.api_base_url, name)?;
        let Some(response) = self.get_optional_json::<NameToIdResponse>(&url).await? else {
            return Ok(None);
        };
        let uuid = Uuid::parse_str(&response.id)
            .with_context(|| format!("parse profile id {}", response.id))?;
        Ok(Some(NameAndId {
            uuid,
            name: response.name,
        }))
    }

    async fn fetch_profile_by_id(&self, id: Uuid) -> Result<Option<ResolvedGameProfile>> {
        let url = format!(
            "{}/session/minecraft/profile/{}?unsigned=false",
            self.session_base_url,
            id.as_simple()
        );
        let Some(response) = self
            .get_optional_json::<SessionProfileResponse>(&url)
            .await?
        else {
            return Ok(None);
        };
        let uuid = Uuid::parse_str(&response.id)
            .with_context(|| format!("parse session profile id {}", response.id))?;
        Ok(Some(ResolvedGameProfile {
            uuid,
            name: response.name,
            properties: response.properties,
        }))
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
async fn resolve_profile_with_cache(
    cache: &mut ProfileResolutionCache,
    client: &MojangProfileHttpClient,
    key: &ProfileResolutionKey,
) -> Result<Option<ResolvedGameProfile>> {
    match key {
        ProfileResolutionKey::Name(name) => {
            if !is_resolvable_player_name(name) {
                return Ok(None);
            }
            let id = match cache.cached_name(name) {
                Some(cached) => cached,
                None => {
                    let fetched = client.fetch_id_by_name(name).await?;
                    cache.record_name(name, fetched)
                }
            };
            match id {
                Some(uuid) => resolve_profile_id_with_cache(cache, client, uuid).await,
                None => Ok(None),
            }
        }
        ProfileResolutionKey::Uuid(uuid) => {
            resolve_profile_id_with_cache(cache, client, *uuid).await
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
async fn resolve_profile_id_with_cache(
    cache: &mut ProfileResolutionCache,
    client: &MojangProfileHttpClient,
    id: Uuid,
) -> Result<Option<ResolvedGameProfile>> {
    if !cache.has_id(id) {
        let fetched = client.fetch_profile_by_id(id).await?;
        cache.record_id(id, fetched);
    }
    Ok(cache.get_id(id))
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub(crate) struct ProfileTextureHttpClient {
    client: reqwest::Client,
    enforce_cdn_host: bool,
    rate_limit: RateLimitBackoff,
}

#[cfg(not(target_arch = "wasm32"))]
impl ProfileTextureHttpClient {
    pub(crate) fn new() -> Result<Self> {
        Self::build(true)
    }

    #[cfg(test)]
    fn new_loopback_for_test() -> Result<Self> {
        Self::build(false)
    }

    fn build(enforce_cdn_host: bool) -> Result<Self> {
        crate::install_http_crypto_provider();
        let redirect_policy = reqwest::redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() >= 10 {
                attempt.error("too many player texture redirects")
            } else if !matches!(attempt.url().scheme(), "http" | "https") {
                attempt.error("player texture redirect must use HTTP or HTTPS")
            } else if enforce_cdn_host && attempt.url().host_str() != Some("textures.minecraft.net")
            {
                attempt.error("player texture redirect left textures.minecraft.net")
            } else if !enforce_cdn_host && attempt.url().host_str() != Some("127.0.0.1") {
                attempt.error("loopback texture redirect left 127.0.0.1")
            } else {
                attempt.follow()
            }
        });
        Ok(Self {
            client: reqwest::Client::builder()
                .no_proxy()
                .redirect(redirect_policy)
                .timeout(PROFILE_HTTP_TIMEOUT)
                .user_agent("bbb-client")
                .build()
                .context("create player skin HTTP client")?,
            enforce_cdn_host,
            rate_limit: RateLimitBackoff::default(),
        })
    }

    pub(crate) async fn fetch_skin_png(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>> {
        ensure!(
            url.len() <= MAX_PROFILE_TEXTURE_URL_BYTES,
            "player texture URL exceeds {MAX_PROFILE_TEXTURE_URL_BYTES} bytes"
        );
        if self.enforce_cdn_host && vanilla_profile_texture_hash(url).is_none() {
            bail!("disallowed player texture URL {url}");
        }
        self.rate_limit.wait_if_needed().await;
        let response = self
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("request HTTP player skin {url}"))?;
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let backoff = self.rate_limit.record_429(response.headers());
            bail!(
                "HTTP 429 rate-limited fetching {url} (backing off {}s)",
                backoff.as_secs()
            );
        }
        if response.status() == reqwest::StatusCode::NO_CONTENT {
            bail!("HTTP 204 No Content fetching player skin {url}");
        }
        let mut response = response
            .error_for_status()
            .with_context(|| format!("fetch HTTP player skin {url}"))?;
        read_bounded_response_bytes(
            &mut response,
            max_bytes,
            &format!("HTTP player texture {url}"),
        )
        .await
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn read_bounded_response_bytes(
    response: &mut reqwest::Response,
    max_bytes: usize,
    label: &str,
) -> Result<Vec<u8>> {
    let max_bytes_u64 = u64::try_from(max_bytes).context("HTTP response byte limit overflow")?;
    let declared_bytes = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .or_else(|| response.content_length());
    if let Some(declared_bytes) = declared_bytes {
        if declared_bytes > max_bytes_u64 {
            bail!("{label} declares {declared_bytes} bytes, exceeding the {max_bytes} byte limit");
        }
    }
    let capacity = declared_bytes
        .and_then(|bytes| usize::try_from(bytes).ok())
        .unwrap_or(0)
        .min(max_bytes);
    let mut bytes = Vec::with_capacity(capacity);
    while let Some(chunk) = response
        .chunk()
        .await
        .with_context(|| format!("read {label} body"))?
    {
        let next_len = bytes
            .len()
            .checked_add(chunk.len())
            .context("HTTP response body size overflow")?;
        if next_len > max_bytes {
            bail!("{label} is {next_len} bytes, exceeding the {max_bytes} byte limit");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(not(target_arch = "wasm32"))]
enum ProfileNetworkCommand {
    ResolveProfile(ProfileResolutionKey),
    FetchTexture(DynamicTextureFetchRequest),
    CacheValidatedTexture { url: String, png_bytes: Vec<u8> },
}

#[cfg(target_arch = "wasm32")]
const MAX_WEB_CACHED_TEXTURE_BYTES: usize = 8 * 1024 * 1024;
#[cfg(target_arch = "wasm32")]
const MAX_WEB_CONCURRENT_PROFILE_JOBS: usize = 4;

#[cfg(target_arch = "wasm32")]
enum WebProfileJob {
    ResolveProfile(ProfileResolutionKey),
    FetchTexture(DynamicTextureFetchRequest),
}

#[cfg(target_arch = "wasm32")]
enum WebProfileWork {
    ResolveProfile(ProfileResolutionKey),
    FetchTexture { url: String, max_bytes: usize },
}

#[cfg(target_arch = "wasm32")]
#[derive(Default)]
struct WebProfileState {
    profiles: ProfileResolutionCache,
    validated_textures: HashMap<String, Vec<u8>>,
    texture_order: VecDeque<String>,
    texture_bytes: usize,
    in_flight_profiles: HashMap<ProfileResolutionKey, Vec<ProfileResolutionKey>>,
    in_flight_textures: HashMap<(String, usize), Vec<DynamicTextureFetchRequest>>,
    profile_rate_limit: RateLimitBackoff,
    texture_rate_limit: RateLimitBackoff,
    in_flight: usize,
    queued: VecDeque<WebProfileWork>,
}

#[cfg(target_arch = "wasm32")]
impl WebProfileState {
    fn try_lookup_profile(
        &self,
        key: &ProfileResolutionKey,
    ) -> Option<Option<ResolvedGameProfile>> {
        self.profiles.try_lookup(key)
    }

    fn cache_texture(&mut self, url: String, png_bytes: Vec<u8>) {
        if png_bytes.len() > MAX_WEB_CACHED_TEXTURE_BYTES {
            return;
        }
        if let Some(prev) = self.validated_textures.remove(&url) {
            self.texture_bytes = self.texture_bytes.saturating_sub(prev.len());
            self.texture_order.retain(|existing| existing != &url);
        }
        while !self.texture_order.is_empty()
            && self.texture_bytes.saturating_add(png_bytes.len()) > MAX_WEB_CACHED_TEXTURE_BYTES
        {
            if let Some(oldest) = self.texture_order.pop_front() {
                if let Some(removed) = self.validated_textures.remove(&oldest) {
                    self.texture_bytes = self.texture_bytes.saturating_sub(removed.len());
                }
            }
        }
        self.texture_bytes += png_bytes.len();
        self.texture_order.push_back(url.clone());
        self.validated_textures.insert(url, png_bytes);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct NativeProfileCoordinator {
    cache: ProfileResolutionCache,
    in_flight_profiles: HashMap<ProfileResolutionKey, Vec<ProfileResolutionKey>>,
    in_flight_textures: HashMap<(String, usize), Vec<DynamicTextureFetchRequest>>,
}

#[cfg(not(target_arch = "wasm32"))]
async fn resolve_native_profile_shared(
    coordinator: &Arc<Mutex<NativeProfileCoordinator>>,
    client: &MojangProfileHttpClient,
    key: &ProfileResolutionKey,
) -> Result<Option<ResolvedGameProfile>> {
    let id = match key {
        ProfileResolutionKey::Uuid(uuid) => *uuid,
        ProfileResolutionKey::Name(name) => {
            if !is_resolvable_player_name(name) {
                return Ok(None);
            }
            let cached_name = {
                let state = coordinator
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.cache.cached_name(name)
            };
            let resolved_id = match cached_name {
                Some(cached) => cached,
                None => {
                    let fetched = client.fetch_id_by_name(name).await?;
                    let mut state = coordinator
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    state.cache.record_name(name, fetched)
                }
            };
            let Some(uuid) = resolved_id else {
                return Ok(None);
            };
            uuid
        }
    };
    {
        let state = coordinator
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.cache.has_id(id) {
            return Ok(state.cache.get_id(id));
        }
    }
    let fetched = client.fetch_profile_by_id(id).await?;
    let mut state = coordinator
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.cache.record_id(id, fetched);
    Ok(state.cache.get_id(id))
}

pub struct ProfileNetworkService {
    #[cfg(not(target_arch = "wasm32"))]
    command_tx: tokio::sync::mpsc::UnboundedSender<ProfileNetworkCommand>,
    #[cfg(target_arch = "wasm32")]
    web_state: Arc<Mutex<WebProfileState>>,
    #[cfg(target_arch = "wasm32")]
    profile_result_tx: Sender<ProfileResolutionResult>,
    profile_result_rx: Receiver<ProfileResolutionResult>,
    #[cfg(target_arch = "wasm32")]
    texture_result_tx: Sender<DynamicTextureFetchResult>,
    texture_result_rx: Receiver<DynamicTextureFetchResult>,
}

impl ProfileNetworkService {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Result<Self> {
        let cache_dir = cache_dir.into();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let profile_client = MojangProfileHttpClient::new()?;
            let texture_client = ProfileTextureHttpClient::new()?;
            Self::with_clients(cache_dir, profile_client, texture_client)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = cache_dir;
            let (profile_result_tx, profile_result_rx) = mpsc::channel();
            let (texture_result_tx, texture_result_rx) = mpsc::channel();
            Ok(Self {
                web_state: Arc::new(Mutex::new(WebProfileState::default())),
                profile_result_tx,
                profile_result_rx,
                texture_result_tx,
                texture_result_rx,
            })
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn with_clients(
        cache_dir: PathBuf,
        profile_client: MojangProfileHttpClient,
        texture_client: ProfileTextureHttpClient,
    ) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("create profile network tokio runtime")?;
        let (command_tx, mut command_rx) =
            tokio::sync::mpsc::unbounded_channel::<ProfileNetworkCommand>();
        let (profile_result_tx, profile_result_rx) = mpsc::channel();
        let (texture_result_tx, texture_result_rx) = mpsc::channel();
        thread::Builder::new()
            .name("bbb-profile-net".to_string())
            .spawn(move || {
                runtime.block_on(async move {
                    let coordinator = Arc::new(Mutex::new(NativeProfileCoordinator::default()));
                    let semaphore = Arc::new(tokio::sync::Semaphore::new(
                        MAX_NATIVE_CONCURRENT_PROFILE_JOBS,
                    ));
                    let cache_dir = Arc::new(cache_dir);
                    while let Some(command) = command_rx.recv().await {
                        match command {
                            ProfileNetworkCommand::ResolveProfile(key) => {
                                let immediate = {
                                    let mut state = coordinator
                                        .lock()
                                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                                    if let Some(resolved) = state.cache.try_lookup(&key) {
                                        Some(resolved)
                                    } else if let Some(waiters) =
                                        state.in_flight_profiles.get_mut(&key)
                                    {
                                        waiters.push(key);
                                        continue;
                                    } else {
                                        state
                                            .in_flight_profiles
                                            .insert(key.clone(), vec![key.clone()]);
                                        None
                                    }
                                };
                                if let Some(resolved) = immediate {
                                    if profile_result_tx
                                        .send(ProfileResolutionResult { key, resolved })
                                        .is_err()
                                    {
                                        break;
                                    }
                                    continue;
                                }
                                let coordinator_ref = Arc::clone(&coordinator);
                                let client = profile_client.clone();
                                let sem = Arc::clone(&semaphore);
                                let result_tx = profile_result_tx.clone();
                                tokio::spawn(async move {
                                    let _permit = match sem.acquire_owned().await {
                                        Ok(permit) => permit,
                                        Err(_) => return,
                                    };
                                    let resolved = resolve_native_profile_shared(
                                        &coordinator_ref,
                                        &client,
                                        &key,
                                    )
                                    .await
                                    .ok()
                                    .flatten();
                                    let waiters = {
                                        let mut state = coordinator_ref
                                            .lock()
                                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                                        state.in_flight_profiles.remove(&key).unwrap_or_default()
                                    };
                                    for waiter_key in waiters {
                                        let _ = result_tx.send(ProfileResolutionResult {
                                            key: waiter_key,
                                            resolved: resolved.clone(),
                                        });
                                    }
                                });
                            }
                            ProfileNetworkCommand::FetchTexture(request) => {
                                let max_bytes = request.kind.max_png_bytes();
                                let dedup_key = (request.url.clone(), max_bytes);
                                {
                                    let mut state = coordinator
                                        .lock()
                                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                                    if let Some(waiters) =
                                        state.in_flight_textures.get_mut(&dedup_key)
                                    {
                                        waiters.push(request);
                                        continue;
                                    }
                                    state
                                        .in_flight_textures
                                        .insert(dedup_key.clone(), vec![request]);
                                }
                                let coordinator_ref = Arc::clone(&coordinator);
                                let client = texture_client.clone();
                                let cache_dir_ref = Arc::clone(&cache_dir);
                                let sem = Arc::clone(&semaphore);
                                let result_tx = texture_result_tx.clone();
                                tokio::spawn(async move {
                                    let _permit = match sem.acquire_owned().await {
                                        Ok(permit) => permit,
                                        Err(_) => return,
                                    };
                                    let outcome = cached_or_fetched_texture_bytes(
                                        &cache_dir_ref,
                                        &client,
                                        &dedup_key.0,
                                        max_bytes,
                                    )
                                    .await;
                                    let (png_bytes, from_network) = match outcome {
                                        Ok((bytes, from_network)) => (Ok(bytes), from_network),
                                        Err(err) => (Err(format!("{err:#}")), false),
                                    };
                                    let waiters = {
                                        let mut state = coordinator_ref
                                            .lock()
                                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                                        state
                                            .in_flight_textures
                                            .remove(&dedup_key)
                                            .unwrap_or_default()
                                    };
                                    for waiter in waiters {
                                        let _ = result_tx.send(DynamicTextureFetchResult {
                                            kind: waiter.kind,
                                            url: waiter.url,
                                            handle: waiter.handle,
                                            png_bytes: png_bytes.clone(),
                                            from_network,
                                        });
                                    }
                                });
                            }
                            ProfileNetworkCommand::CacheValidatedTexture { url, png_bytes } => {
                                let cache_dir_ref = Arc::clone(&cache_dir);
                                tokio::task::spawn_blocking(move || {
                                    if let Err(err) = write_validated_player_texture_cache(
                                        &cache_dir_ref,
                                        &url,
                                        &png_bytes,
                                    ) {
                                        tracing::warn!(
                                            ?err,
                                            %url,
                                            "failed to cache player profile texture"
                                        );
                                    }
                                });
                            }
                        }
                    }
                });
            })
            .context("spawn profile network worker")?;
        Ok(Self {
            command_tx,
            profile_result_rx,
            texture_result_rx,
        })
    }

    pub fn submit_profile_requests(
        &mut self,
        requests: impl IntoIterator<Item = ProfileResolutionKey>,
    ) {
        for key in requests {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = self
                    .command_tx
                    .send(ProfileNetworkCommand::ResolveProfile(key));
            }
            #[cfg(target_arch = "wasm32")]
            {
                let cached = {
                    let state = self
                        .web_state
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    state.try_lookup_profile(&key)
                };
                if let Some(resolved) = cached {
                    let _ = self
                        .profile_result_tx
                        .send(ProfileResolutionResult { key, resolved });
                } else {
                    enqueue_web_profile_job(
                        &self.web_state,
                        WebProfileJob::ResolveProfile(key),
                        &self.profile_result_tx,
                        &self.texture_result_tx,
                    );
                }
            }
        }
    }

    pub fn drain_profile_results(&mut self) -> Vec<ProfileResolutionResult> {
        let mut results = Vec::new();
        while let Ok(result) = self.profile_result_rx.try_recv() {
            results.push(result);
        }
        results
    }

    pub fn submit_texture_requests(
        &mut self,
        requests: impl IntoIterator<Item = DynamicTextureFetchRequest>,
    ) {
        for request in requests {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = self
                    .command_tx
                    .send(ProfileNetworkCommand::FetchTexture(request));
            }
            #[cfg(target_arch = "wasm32")]
            {
                let cached_bytes = {
                    let state = self
                        .web_state
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    state.validated_textures.get(&request.url).cloned()
                };
                if let Some(bytes) = cached_bytes {
                    let _ = self.texture_result_tx.send(DynamicTextureFetchResult {
                        kind: request.kind,
                        url: request.url,
                        handle: request.handle,
                        png_bytes: Ok(bytes),
                        from_network: false,
                    });
                } else {
                    enqueue_web_profile_job(
                        &self.web_state,
                        WebProfileJob::FetchTexture(request),
                        &self.profile_result_tx,
                        &self.texture_result_tx,
                    );
                }
            }
        }
    }

    pub fn drain_texture_results(&mut self) -> Vec<DynamicTextureFetchResult> {
        let mut results = Vec::new();
        while let Ok(result) = self.texture_result_rx.try_recv() {
            results.push(result);
        }
        results
    }

    pub fn cache_validated_textures(
        &mut self,
        validated: impl IntoIterator<Item = (String, Vec<u8>)>,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        for (url, png_bytes) in validated {
            let _ = self
                .command_tx
                .send(ProfileNetworkCommand::CacheValidatedTexture { url, png_bytes });
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mut state = self
                .web_state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for (url, png_bytes) in validated {
                state.cache_texture(url, png_bytes);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn try_read_cached_player_texture(path: &Path, max_bytes: usize) -> Option<Vec<u8>> {
    if !path.is_file() {
        return None;
    }
    match read_bounded_png_file(path, max_bytes, "cached player texture") {
        Ok(bytes) => Some(bytes),
        Err(err) => {
            tracing::warn!(
                ?err,
                path = %path.display(),
                "evicting unreadable or oversized cached player texture before refetching"
            );
            let _ = fs::remove_file(path);
            None
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn cached_or_fetched_texture_bytes(
    cache_dir: &Path,
    client: &ProfileTextureHttpClient,
    url: &str,
    max_bytes: usize,
) -> Result<(Vec<u8>, bool)> {
    let path = vanilla_profile_texture_cache_path(cache_dir, url)?;
    let cached =
        tokio::task::spawn_blocking(move || try_read_cached_player_texture(&path, max_bytes))
            .await
            .context("join cached player texture read")?;
    if let Some(bytes) = cached {
        return Ok((bytes, false));
    }
    let bytes = client
        .fetch_skin_png(url, max_bytes)
        .await
        .with_context(|| format!("fetch player texture from {url}"))?;
    validate_png_body_len(bytes.len(), max_bytes, "fetched player texture")?;
    Ok((bytes, true))
}

#[cfg(target_arch = "wasm32")]
fn enqueue_web_profile_job(
    web_state: &Arc<Mutex<WebProfileState>>,
    job: WebProfileJob,
    profile_tx: &Sender<ProfileResolutionResult>,
    texture_tx: &Sender<DynamicTextureFetchResult>,
) {
    let should_spawn = {
        let mut state = web_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match job {
            WebProfileJob::ResolveProfile(key) => {
                if let Some(waiters) = state.in_flight_profiles.get_mut(&key) {
                    waiters.push(key);
                    return;
                }
                state
                    .in_flight_profiles
                    .insert(key.clone(), vec![key.clone()]);
                state.queued.push_back(WebProfileWork::ResolveProfile(key));
            }
            WebProfileJob::FetchTexture(request) => {
                let max_bytes = request.kind.max_png_bytes();
                let dedup_key = (request.url.clone(), max_bytes);
                if let Some(waiters) = state.in_flight_textures.get_mut(&dedup_key) {
                    waiters.push(request);
                    return;
                }
                let url = dedup_key.0.clone();
                state.in_flight_textures.insert(dedup_key, vec![request]);
                state
                    .queued
                    .push_back(WebProfileWork::FetchTexture { url, max_bytes });
            }
        }
        if state.in_flight < MAX_WEB_CONCURRENT_PROFILE_JOBS {
            state.in_flight += 1;
            true
        } else {
            false
        }
    };
    if should_spawn {
        let state_ref = Arc::clone(web_state);
        let profile_tx = profile_tx.clone();
        let texture_tx = texture_tx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                let next_job = {
                    let mut state = state_ref
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    match state.queued.pop_front() {
                        Some(job) => job,
                        None => {
                            state.in_flight = state.in_flight.saturating_sub(1);
                            break;
                        }
                    }
                };
                match next_job {
                    WebProfileWork::ResolveProfile(key) => {
                        let resolved = resolve_web_profile_cached(&state_ref, &key)
                            .await
                            .ok()
                            .flatten();
                        let waiters = {
                            let mut state = state_ref
                                .lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner());
                            state.in_flight_profiles.remove(&key).unwrap_or_default()
                        };
                        for waiter_key in waiters {
                            let _ = profile_tx.send(ProfileResolutionResult {
                                key: waiter_key,
                                resolved: resolved.clone(),
                            });
                        }
                    }
                    WebProfileWork::FetchTexture { url, max_bytes } => {
                        let dedup_key = (url.clone(), max_bytes);
                        let (cached, rate_limit) = {
                            let state = state_ref
                                .lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner());
                            (
                                state.validated_textures.get(&url).cloned(),
                                state.texture_rate_limit.clone(),
                            )
                        };
                        let (png_bytes, from_network) = match cached {
                            Some(bytes) => (Ok(bytes), false),
                            None => {
                                match fetch_web_dynamic_texture_bytes(&url, max_bytes, &rate_limit)
                                    .await
                                {
                                    Ok(bytes) => (Ok(bytes), true),
                                    Err(err) => (Err(format!("{err:#}")), false),
                                }
                            }
                        };
                        let waiters = {
                            let mut state = state_ref
                                .lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner());
                            state
                                .in_flight_textures
                                .remove(&dedup_key)
                                .unwrap_or_default()
                        };
                        for waiter in waiters {
                            let _ = texture_tx.send(DynamicTextureFetchResult {
                                kind: waiter.kind,
                                url: waiter.url,
                                handle: waiter.handle,
                                png_bytes: png_bytes.clone(),
                                from_network,
                            });
                        }
                    }
                }
            }
        });
    }
}

#[cfg(target_arch = "wasm32")]
async fn resolve_web_profile_cached(
    web_state: &Arc<Mutex<WebProfileState>>,
    key: &ProfileResolutionKey,
) -> Result<Option<ResolvedGameProfile>> {
    let rate_limit = {
        let state = web_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.profile_rate_limit.clone()
    };
    let id = match key {
        ProfileResolutionKey::Uuid(uuid) => *uuid,
        ProfileResolutionKey::Name(name) => {
            if !is_resolvable_player_name(name) {
                return Ok(None);
            }
            let cached_name = {
                let state = web_state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.profiles.cached_name(name)
            };
            let resolved_id = match cached_name {
                Some(cached) => cached,
                None => {
                    let url = build_mojang_name_lookup_url("https://api.mojang.com", name)?;
                    let fetched =
                        match fetch_web_optional_json::<NameToIdResponse>(&url, &rate_limit).await?
                        {
                            Some(resp) => Some(NameAndId {
                                uuid: Uuid::parse_str(&resp.id)
                                    .with_context(|| format!("parse profile id {}", resp.id))?,
                                name: resp.name,
                            }),
                            None => None,
                        };
                    let mut state = web_state
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    state.profiles.record_name(name, fetched)
                }
            };
            let Some(uuid) = resolved_id else {
                return Ok(None);
            };
            uuid
        }
    };

    {
        let state = web_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.profiles.has_id(id) {
            return Ok(state.profiles.get_id(id));
        }
    }

    let url = format!(
        "https://sessionserver.mojang.com/session/minecraft/profile/{}?unsigned=false",
        id.as_simple()
    );
    let fetched = match fetch_web_optional_json::<SessionProfileResponse>(&url, &rate_limit).await?
    {
        Some(response) => {
            let uuid = Uuid::parse_str(&response.id)
                .with_context(|| format!("parse session profile id {}", response.id))?;
            Some(ResolvedGameProfile {
                uuid,
                name: response.name,
                properties: response.properties,
            })
        }
        None => None,
    };
    let mut state = web_state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.profiles.record_id(id, fetched);
    Ok(state.profiles.get_id(id))
}

#[cfg(target_arch = "wasm32")]
async fn fetch_web_optional_json<T: for<'de> Deserialize<'de>>(
    url: &str,
    rate_limit: &RateLimitBackoff,
) -> Result<Option<T>> {
    rate_limit.wait_if_needed().await;
    let encoded: String = js_sys::encode_uri_component(url).into();
    let proxy_url = format!("/proxy/skin?url={encoded}");
    let response = crate::wasm_fetch::fetch_bounded_bytes(
        &proxy_url,
        &format!("HTTP profile {url}"),
        PROFILE_HTTP_TIMEOUT,
        MAX_MOJANG_PROFILE_RESPONSE_BYTES as u64,
        None,
        |_| {},
    )
    .await?;
    if response.status == 429 {
        let backoff = rate_limit.record_429_retry_after(response.retry_after.as_deref());
        bail!(
            "HTTP 429 rate-limited fetching {url} (backing off {}s)",
            backoff.as_secs()
        );
    }
    if response.status == 204 || response.status == 404 {
        return Ok(None);
    }
    if !response.ok {
        bail!("HTTP {} from {url}", response.status);
    }
    serde_json::from_slice(&response.body)
        .with_context(|| format!("decode HTTP profile JSON from {url}"))
        .map(Some)
}

#[cfg(target_arch = "wasm32")]
async fn fetch_web_dynamic_texture_bytes(
    url: &str,
    max_bytes: usize,
    rate_limit: &RateLimitBackoff,
) -> Result<Vec<u8>> {
    if url.len() > MAX_PROFILE_TEXTURE_URL_BYTES || vanilla_profile_texture_hash(url).is_none() {
        bail!("disallowed player texture URL {url}");
    }
    rate_limit.wait_if_needed().await;
    let encoded: String = js_sys::encode_uri_component(url).into();
    let proxy_url = format!("/proxy/skin?url={encoded}");
    let response = crate::wasm_fetch::fetch_bounded_bytes(
        &proxy_url,
        &format!("player texture {url}"),
        PROFILE_HTTP_TIMEOUT,
        max_bytes as u64,
        None,
        |_| {},
    )
    .await?;
    if response.status == 429 {
        let backoff = rate_limit.record_429_retry_after(response.retry_after.as_deref());
        bail!(
            "HTTP 429 rate-limited fetching {url} (backing off {}s)",
            backoff.as_secs()
        );
    }
    if !response.ok || response.status == 204 {
        bail!("HTTP {} from {url}", response.status);
    }
    Ok(response.body)
}

pub fn default_player_skin_cache_dir() -> PathBuf {
    #[cfg(target_arch = "wasm32")]
    {
        PathBuf::from("/__bbb_mem__/player-skins")
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::temp_dir().join("bbb-client-player-skins")
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn player_skin_cache_path(cache_dir: &Path, url: &str) -> Result<PathBuf> {
    vanilla_profile_texture_cache_path(cache_dir, url)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn player_texture_cache_path(cache_dir: &Path, url: &str) -> Result<PathBuf> {
    vanilla_profile_texture_cache_path(cache_dir, url)
}

#[cfg(not(target_arch = "wasm32"))]
fn vanilla_profile_texture_cache_path(cache_dir: &Path, url: &str) -> Result<PathBuf> {
    let texture_hash = vanilla_profile_texture_hash(url)
        .with_context(|| format!("reject non-Vanilla profile texture URL {url}"))?;
    let cache_hash = vanilla_profile_texture_cache_hash(&texture_hash);
    let prefix = cache_hash.get(..2).unwrap_or("xx");
    Ok(cache_dir.join(prefix).join(cache_hash))
}

#[cfg(not(target_arch = "wasm32"))]
fn read_bounded_png_file(path: &Path, max_bytes: usize, label: &str) -> Result<Vec<u8>> {
    let file = fs::File::open(path).with_context(|| format!("open {label} {}", path.display()))?;
    let declared_bytes = file
        .metadata()
        .with_context(|| format!("read {label} metadata {}", path.display()))?
        .len();
    read_bounded_png_body(file, Some(declared_bytes), max_bytes, label)
        .with_context(|| format!("read {label} {}", path.display()))
}

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
fn validate_png_body_len(actual_bytes: usize, max_bytes: usize, label: &str) -> Result<()> {
    if actual_bytes > max_bytes {
        bail!("{label} is {actual_bytes} bytes, exceeding the {max_bytes} byte limit");
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_validated_player_texture_cache(
    cache_dir: &Path,
    url: &str,
    bytes: &[u8],
) -> Result<()> {
    let path = vanilla_profile_texture_cache_path(cache_dir, url)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create player skin cache directory {}", parent.display()))?;
    }
    fs::write(&path, bytes).with_context(|| format!("write player skin cache {}", path.display()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use bbb_protocol::{packets::GameProfilePropertySummary, MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES};
    use std::{
        io::{Cursor, Read, Write},
        net::TcpListener,
        sync::atomic::{AtomicU64, Ordering},
        thread,
    };

    static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);

    const SLIM_TEXTURES_PROPERTY: &str = "eyJ0aW1lc3RhbXAiOjEsInByb2ZpbGVJZCI6IjAxMjM0NTY3ODlhYmNkZWYwMTIzNDU2Nzg5YWJjZGVmIiwicHJvZmlsZU5hbWUiOiJBbGV4IiwidGV4dHVyZXMiOnsiU0tJTiI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9za2luaGFzaCIsIm1ldGFkYXRhIjp7Im1vZGVsIjoic2xpbSJ9fSwiQ0FQRSI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9jYXBlaGFzaCJ9LCJFTFlUUkEiOnsidXJsIjoiaHR0cHM6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZWx5dHJhaGFzaCJ9fX0=";

    #[tokio::test]
    async fn http_profile_client_parses_loopback_name_and_session_profile_and_caches() {
        let uuid = Uuid::from_u128(0x01234567_89ab_cdef_0123_456789abcdef);
        let base_url = spawn_http_routes(vec![
            HttpRoute {
                path: "/users/profiles/minecraft/Alex".to_string(),
                status: 200,
                reason: "OK".to_string(),
                headers: Vec::new(),
                body: format!(r#"{{"id":"{}","name":"Alex"}}"#, uuid.as_simple()).into_bytes(),
            },
            HttpRoute {
                path: format!(
                    "/session/minecraft/profile/{}?unsigned=false",
                    uuid.as_simple()
                ),
                status: 200,
                reason: "OK".to_string(),
                headers: Vec::new(),
                body: format!(
                    r#"{{"id":"{}","name":"Alex","properties":[{{"name":"textures","value":"{}","signature":"sig"}}]}}"#,
                    uuid.as_simple(),
                    SLIM_TEXTURES_PROPERTY
                )
                .into_bytes(),
            },
        ]);
        let client = MojangProfileHttpClient::with_base_urls(&base_url, &base_url).unwrap();
        let mut cache = ProfileResolutionCache::default();

        let first = resolve_profile_with_cache(
            &mut cache,
            &client,
            &ProfileResolutionKey::Name("Alex".to_string()),
        )
        .await
        .unwrap()
        .unwrap();
        let second = resolve_profile_with_cache(
            &mut cache,
            &client,
            &ProfileResolutionKey::Name("Alex".to_string()),
        )
        .await
        .unwrap()
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.uuid, uuid);
        assert_eq!(first.name, "Alex");
        assert_eq!(
            first.properties,
            vec![GameProfilePropertySummary {
                name: "textures".into(),
                value: SLIM_TEXTURES_PROPERTY.to_string().into(),
                signature: Some("sig".into()),
            }]
        );
    }

    #[tokio::test]
    async fn http_profile_client_maps_no_content_and_not_found_to_miss() {
        let uuid = Uuid::from_u128(0x01234567_89ab_cdef_0123_456789abcdef);
        let base_url = spawn_http_routes(vec![
            HttpRoute {
                path: "/users/profiles/minecraft/Missing".to_string(),
                status: 204,
                reason: "No Content".to_string(),
                headers: Vec::new(),
                body: Vec::new(),
            },
            HttpRoute {
                path: format!(
                    "/session/minecraft/profile/{}?unsigned=false",
                    uuid.as_simple()
                ),
                status: 404,
                reason: "Not Found".to_string(),
                headers: Vec::new(),
                body: b"missing".to_vec(),
            },
        ]);
        let client = MojangProfileHttpClient::with_base_urls(&base_url, &base_url).unwrap();

        assert!(client.fetch_id_by_name("Missing").await.unwrap().is_none());
        assert!(client.fetch_profile_by_id(uuid).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn http_skin_client_enforces_host_allowlist_redirects_and_budgets() {
        let cdn_client = ProfileTextureHttpClient::new().unwrap();
        let disallowed = cdn_client
            .fetch_skin_png(
                "https://example.com/skin.png",
                MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES,
            )
            .await
            .unwrap_err()
            .to_string();
        assert!(
            disallowed.contains("disallowed player texture URL"),
            "{disallowed}"
        );

        let body = b"player skin bytes".to_vec();
        let ok_base = spawn_http_routes(vec![HttpRoute {
            path: "/skin.png".to_string(),
            status: 200,
            reason: "OK".to_string(),
            headers: Vec::new(),
            body: body.clone(),
        }]);
        let loopback_client = ProfileTextureHttpClient::new_loopback_for_test().unwrap();
        assert_eq!(
            loopback_client
                .fetch_skin_png(
                    &format!("{ok_base}/skin.png"),
                    MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES
                )
                .await
                .unwrap(),
            body
        );

        let not_found_base = spawn_http_routes(vec![HttpRoute {
            path: "/skin.png".to_string(),
            status: 404,
            reason: "Not Found".to_string(),
            headers: Vec::new(),
            body: b"missing".to_vec(),
        }]);
        let err = loopback_client
            .fetch_skin_png(
                &format!("{not_found_base}/skin.png"),
                MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES,
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("fetch HTTP player skin"));

        let redirect_base = spawn_http_routes(vec![HttpRoute {
            path: "/redirect.png".to_string(),
            status: 302,
            reason: "Found".to_string(),
            headers: vec![(
                "Location".to_string(),
                "https://example.com/evil.png".to_string(),
            )],
            body: Vec::new(),
        }]);
        let redirect_err = format!(
            "{:#}",
            loopback_client
                .fetch_skin_png(
                    &format!("{redirect_base}/redirect.png"),
                    MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES,
                )
                .await
                .unwrap_err()
        );
        assert!(redirect_err.contains("redirect"), "{redirect_err}");
    }

    #[tokio::test]
    async fn disk_cache_reads_before_network_and_rejects_oversized_entries() {
        let root = unique_temp_dir("profile-net-disk-cache");
        let url = "https://textures.minecraft.net/texture/cached-skin";
        let expected = b"cached-png-bytes".to_vec();
        write_validated_player_texture_cache(&root, url, &expected).unwrap();

        let client = ProfileTextureHttpClient::new().unwrap();
        let (bytes, from_network) =
            cached_or_fetched_texture_bytes(&root, &client, url, MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES)
                .await
                .unwrap();
        assert!(!from_network);
        assert_eq!(bytes, expected);

        let oversized_url = "https://textures.minecraft.net/texture/oversized-cache";
        let oversized_path = player_skin_cache_path(&root, oversized_url).unwrap();
        fs::create_dir_all(oversized_path.parent().unwrap()).unwrap();
        let file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&oversized_path)
            .unwrap();
        file.set_len(MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES as u64 + 1)
            .unwrap();
        let err = format!(
            "{:#}",
            read_bounded_png_file(
                &oversized_path,
                MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES,
                "cached player texture"
            )
            .unwrap_err()
        );
        assert!(err.contains("declares 1048577 bytes"), "{err}");
        assert!(
            try_read_cached_player_texture(&oversized_path, MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES)
                .is_none()
        );
        assert!(
            !oversized_path.exists(),
            "corrupt/oversized cached player texture should be evicted before refetching"
        );

        let declared = read_bounded_png_body(Cursor::new(b"ok"), Some(5), 4, "declared fixture")
            .unwrap_err()
            .to_string();
        assert!(declared.contains("declares 5 bytes"), "{declared}");
        let streamed = read_bounded_png_body(Cursor::new(b"12345"), None, 4, "streamed fixture")
            .unwrap_err()
            .to_string();
        assert!(streamed.contains("is 5 bytes"), "{streamed}");

        fs::remove_dir_all(root).unwrap();
    }

    #[derive(Clone)]
    struct HttpRoute {
        path: String,
        status: u16,
        reason: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    }

    fn spawn_http_routes(routes: Vec<HttpRoute>) -> String {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            for _ in 0..routes.len() {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0u8; 2048];
                let len = stream.read(&mut request).unwrap_or(0);
                let path = http_request_path(&request[..len]).unwrap_or("/");
                let route = routes
                    .iter()
                    .find(|route| route.path == path)
                    .cloned()
                    .unwrap_or_else(|| HttpRoute {
                        path: path.to_string(),
                        status: 404,
                        reason: "Not Found".to_string(),
                        headers: Vec::new(),
                        body: Vec::new(),
                    });
                let mut extra_headers = String::new();
                for (name, value) in &route.headers {
                    extra_headers.push_str(&format!("{name}: {value}\r\n"));
                }
                write!(
                    stream,
                    "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n{}Connection: close\r\n\r\n",
                    route.status,
                    route.reason,
                    route.body.len(),
                    extra_headers
                )
                .unwrap();
                stream.write_all(&route.body).unwrap();
            }
        });
        format!("http://{addr}")
    }

    fn http_request_path(request: &[u8]) -> Option<&str> {
        let request = std::str::from_utf8(request).ok()?;
        let first_line = request.lines().next()?;
        first_line.split_whitespace().nth(1)
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let id = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn profile_cache_evicts_over_limit_and_expires_negative_entries() {
        let mut cache = ProfileResolutionCache::with_limits(2, Duration::from_millis(25));
        let u1 = Uuid::from_u128(1);
        let u2 = Uuid::from_u128(2);
        let u3 = Uuid::from_u128(3);
        cache.record_name(
            "alpha",
            Some(NameAndId {
                uuid: u1,
                name: "alpha".into(),
            }),
        );
        cache.record_name(
            "beta",
            Some(NameAndId {
                uuid: u2,
                name: "beta".into(),
            }),
        );
        cache.record_name(
            "gamma",
            Some(NameAndId {
                uuid: u3,
                name: "gamma".into(),
            }),
        );
        assert!(cache.by_name.len() <= 2);
        assert!(cache.cached_name("alpha").is_none());
        assert_eq!(cache.cached_name("gamma"), Some(Some(u3)));

        // Negative entry hits immediately, then expires after TTL.
        cache.record_id(u1, None);
        assert!(cache.has_id(u1));
        assert_eq!(cache.get_id(u1), None);
        thread::sleep(Duration::from_millis(40));
        assert!(!cache.has_id(u1));
    }

    #[tokio::test]
    async fn profile_client_respects_http_429_retry_after_and_service_deduplicates_in_flight() {
        use std::sync::atomic::AtomicUsize;

        let uuid = Uuid::from_u128(0x01234567_89ab_cdef_0123_456789abcdef);
        let base_429 = spawn_http_routes(vec![HttpRoute {
            path: "/users/profiles/minecraft/RateLimited".to_string(),
            status: 429,
            reason: "Too Many Requests".to_string(),
            headers: vec![("Retry-After".to_string(), "1".to_string())],
            body: Vec::new(),
        }]);
        let client_429 = MojangProfileHttpClient::with_base_urls(&base_429, &base_429).unwrap();
        let err = client_429
            .fetch_id_by_name("RateLimited")
            .await
            .expect_err("429 should return rate limit error");
        assert!(err.to_string().contains("429"), "{err:#}");

        // Verify in-flight deduplication: 4 concurrent identical requests trigger only 1 HTTP fetch.
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        let hit_count = Arc::new(AtomicUsize::new(0));
        let hits = Arc::clone(&hit_count);
        let body = format!(
            r#"{{"id":"{}","name":"Alex","properties":[]}}"#,
            uuid.as_simple()
        );
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                hits.fetch_add(1, Ordering::SeqCst);
                let mut req = [0u8; 1024];
                let _ = stream.read(&mut req);
                thread::sleep(Duration::from_millis(60));
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
            }
        });
        let base = format!("http://{addr}");
        let profile_client = MojangProfileHttpClient::with_base_urls(&base, &base).unwrap();
        let texture_client = ProfileTextureHttpClient::new_loopback_for_test().unwrap();
        let temp_dir = unique_temp_dir("bbb-profile-dedup");
        let mut service =
            ProfileNetworkService::with_clients(temp_dir.clone(), profile_client, texture_client)
                .unwrap();

        let key = ProfileResolutionKey::Uuid(uuid);
        service.submit_profile_requests(vec![key.clone(), key.clone(), key.clone(), key.clone()]);

        let mut results = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while results.len() < 4 && std::time::Instant::now() < deadline {
            results.extend(service.drain_profile_results());
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(results.len(), 4);
        assert!(results
            .iter()
            .all(|r| r.resolved.as_ref().is_some_and(|p| p.uuid == uuid)));
        assert_eq!(hit_count.load(Ordering::SeqCst), 1);
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn profile_name_lookup_rejects_empty_and_strictly_encodes_path_segments() {
        assert!(build_mojang_name_lookup_url("https://api.mojang.com", "").is_err());
        assert!(build_mojang_name_lookup_url("https://api.mojang.com", "a b").is_err());
        assert_eq!(
            build_mojang_name_lookup_url("https://api.mojang.com/", "Alex_01").unwrap(),
            "https://api.mojang.com/users/profiles/minecraft/Alex_01"
        );
        assert_eq!(
            build_mojang_name_lookup_url("https://api.mojang.com", "../a?b#c%2f").unwrap(),
            "https://api.mojang.com/users/profiles/minecraft/%2E%2E%2Fa%3Fb%23c%252f"
        );
        assert_eq!(
            build_mojang_name_lookup_url("https://api.mojang.com", ".").unwrap(),
            "https://api.mojang.com/users/profiles/minecraft/%2E"
        );

        let cache = ProfileResolutionCache::default();
        assert_eq!(
            cache.try_lookup(&ProfileResolutionKey::Name(String::new())),
            Some(None)
        );

        let uuid = Uuid::from_u128(0xfeed_face_cafe_babe_0123_4567_89ab_cdef);
        let base_url = spawn_http_routes(vec![HttpRoute {
            path: "/users/profiles/minecraft/%2E%2E%2Fa%3Fb%23c%252f".to_string(),
            status: 200,
            reason: "OK".to_string(),
            headers: Vec::new(),
            body: format!(r#"{{"id":"{}","name":"Encoded"}}"#, uuid.as_simple()).into_bytes(),
        }]);
        let client = MojangProfileHttpClient::with_base_urls(&base_url, &base_url).unwrap();
        // Empty name must return None without consuming the single mock route.
        assert!(client.fetch_id_by_name("").await.unwrap().is_none());
        let resolved = client
            .fetch_id_by_name("../a?b#c%2f")
            .await
            .unwrap()
            .expect("strictly percent-encoded segment must hit exact path");
        assert_eq!(resolved.uuid, uuid);
    }

    #[tokio::test]
    async fn profile_204_negative_caches_without_eof_and_429_supports_http_date_retry_after() {
        // RFC 9110 IMF-fixdate: Wed, 21 Oct 2015 07:28:00 GMT == Unix 1445412480.
        assert_eq!(
            parse_retry_after_value_at("45", 1_445_412_400),
            Some(Duration::from_secs(45))
        );
        assert_eq!(
            parse_retry_after_value_at("Wed, 21 Oct 2015 07:28:00 GMT", 1_445_412_450),
            Some(Duration::from_secs(30))
        );
        assert_eq!(
            parse_retry_after_value_at("Wed, 21 Oct 2015 07:28:00 GMT", 1_445_412_500),
            Some(Duration::from_secs(0))
        );
        assert_eq!(
            parse_retry_after_value_at("not-a-date", 1_445_412_450),
            None
        );

        // Single 204 route: second lookup must hit negative cache without a second HTTP request.
        let base_204 = spawn_http_routes(vec![HttpRoute {
            path: "/users/profiles/minecraft/Ghost".to_string(),
            status: 204,
            reason: "No Content".to_string(),
            headers: Vec::new(),
            body: Vec::new(),
        }]);
        let client = MojangProfileHttpClient::with_base_urls(&base_204, &base_204).unwrap();
        let mut cache = ProfileResolutionCache::default();
        let key = ProfileResolutionKey::Name("Ghost".to_string());
        assert!(resolve_profile_with_cache(&mut cache, &client, &key)
            .await
            .unwrap()
            .is_none());
        assert!(resolve_profile_with_cache(&mut cache, &client, &key)
            .await
            .unwrap()
            .is_none());

        // Texture fetch returning HTTP 204 must fail rather than yielding empty bytes.
        let base_tex_204 = spawn_http_routes(vec![HttpRoute {
            path: "/skin.png".to_string(),
            status: 204,
            reason: "No Content".to_string(),
            headers: Vec::new(),
            body: Vec::new(),
        }]);
        let tex_client = ProfileTextureHttpClient::new_loopback_for_test().unwrap();
        let err = tex_client
            .fetch_skin_png(
                &format!("{base_tex_204}/skin.png"),
                MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES,
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("204"), "{err:#}");
    }
}
