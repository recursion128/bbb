//! CORS-safe HTTP proxies for Mojang player skins and server resource packs with SSRF protection.

use std::{
    error::Error as StdError,
    fmt,
    future::Future,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Once, OnceLock,
    },
    time::Duration,
};

use anyhow::Result;
use bytes::Bytes;
use reqwest::{
    dns::{Addrs, Name, Resolve, Resolving},
    header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE, RETRY_AFTER, USER_AGENT},
    redirect::Policy,
    StatusCode, Url,
};
use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::{OwnedSemaphorePermit, Semaphore},
};
use url::Host;

use crate::http::{write_streaming_ok_headers, HttpRequest, HttpResponse};

const MAX_SKIN_BYTES: usize = 2 * 1024 * 1024;
const MAX_PACK_BYTES: usize = 256 * 1024 * 1024;
const MAX_INITIAL_PROXY_RESERVE_BYTES: usize = 1024 * 1024;
const MAX_INFLIGHT_PROXY_BYTES: usize = 256 * 1024 * 1024;
const MAX_CONCURRENT_SKIN_REQUESTS: usize = 16;
const MAX_CONCURRENT_PACK_REQUESTS: usize = 4;
const MAX_SKIN_REDIRECTS: usize = 3;
const MAX_PACK_REDIRECTS: usize = 4;
const PROXY_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const PROXY_READ_TIMEOUT: Duration = Duration::from_secs(30);
const SKIN_PROXY_TOTAL_TIMEOUT: Duration = Duration::from_secs(30);
const PACK_PROXY_TOTAL_TIMEOUT: Duration = Duration::from_secs(120);
const ALLOWED_SKIN_HOSTS: &[&str] = &[
    "textures.minecraft.net",
    "api.mojang.com",
    "sessionserver.mojang.com",
    "api.minecraftservices.com",
];

struct InFlightByteBudget {
    max_bytes: usize,
    used_bytes: AtomicUsize,
}

impl InFlightByteBudget {
    fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            used_bytes: AtomicUsize::new(0),
        }
    }

    fn guard(self: &Arc<Self>) -> ByteBudgetGuard {
        ByteBudgetGuard {
            budget: Arc::clone(self),
            reserved: 0,
        }
    }
}

pub(crate) struct ByteBudgetGuard {
    budget: Arc<InFlightByteBudget>,
    reserved: usize,
}

impl ByteBudgetGuard {
    fn try_reserve(&mut self, additional: usize) -> bool {
        if additional == 0 {
            return true;
        }
        let ok = self
            .budget
            .used_bytes
            .try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                let next = used.checked_add(additional)?;
                (next <= self.budget.max_bytes).then_some(next)
            })
            .is_ok();
        if ok {
            self.reserved += additional;
        }
        ok
    }

    fn release(&mut self, amount: usize) {
        let to_release = amount.min(self.reserved);
        if to_release > 0 {
            self.reserved -= to_release;
            self.budget
                .used_bytes
                .fetch_sub(to_release, Ordering::Release);
        }
    }
}

impl Drop for ByteBudgetGuard {
    fn drop(&mut self) {
        if self.reserved > 0 {
            self.budget
                .used_bytes
                .fetch_sub(self.reserved, Ordering::Release);
            self.reserved = 0;
        }
    }
}

pub(crate) struct ProxyLimiter {
    skin_concurrency: Arc<Semaphore>,
    pack_concurrency: Arc<Semaphore>,
    inflight_bytes: Arc<InFlightByteBudget>,
    skin_total_timeout: Duration,
    pack_total_timeout: Duration,
}

impl Default for ProxyLimiter {
    fn default() -> Self {
        Self::new(
            MAX_CONCURRENT_SKIN_REQUESTS,
            MAX_CONCURRENT_PACK_REQUESTS,
            MAX_INFLIGHT_PROXY_BYTES,
            SKIN_PROXY_TOTAL_TIMEOUT,
            PACK_PROXY_TOTAL_TIMEOUT,
        )
    }
}

impl ProxyLimiter {
    fn new(
        max_skin_concurrency: usize,
        max_pack_concurrency: usize,
        max_inflight_bytes: usize,
        skin_total_timeout: Duration,
        pack_total_timeout: Duration,
    ) -> Self {
        Self {
            skin_concurrency: Arc::new(Semaphore::new(max_skin_concurrency)),
            pack_concurrency: Arc::new(Semaphore::new(max_pack_concurrency)),
            inflight_bytes: Arc::new(InFlightByteBudget::new(max_inflight_bytes)),
            skin_total_timeout,
            pack_total_timeout,
        }
    }

    fn global() -> &'static Self {
        static GLOBAL: OnceLock<ProxyLimiter> = OnceLock::new();
        GLOBAL.get_or_init(ProxyLimiter::default)
    }

    fn acquire_skin(&self) -> Option<OwnedSemaphorePermit> {
        Arc::clone(&self.skin_concurrency).try_acquire_owned().ok()
    }

    fn acquire_pack(&self) -> Option<OwnedSemaphorePermit> {
        Arc::clone(&self.pack_concurrency).try_acquire_owned().ok()
    }
}

pub(crate) struct ProxyResponse {
    pub(crate) response: HttpResponse,
    _permit: Option<OwnedSemaphorePermit>,
    _budget: Option<ByteBudgetGuard>,
}

impl ProxyResponse {
    fn plain(response: HttpResponse) -> Self {
        Self {
            response,
            _permit: None,
            _budget: None,
        }
    }

    fn guarded(
        response: HttpResponse,
        permit: OwnedSemaphorePermit,
        budget: ByteBudgetGuard,
    ) -> Self {
        Self {
            response,
            _permit: Some(permit),
            _budget: Some(budget),
        }
    }
}

fn busy_response(message: impl Into<String>) -> HttpResponse {
    HttpResponse::text(503, "Service Unavailable", message).with_header("Retry-After", "1")
}

pub fn ensure_rustls_provider() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

#[derive(Debug, Clone, Copy)]
enum HostPolicy {
    SkinAllowlist,
    AnyPublic {
        allow_loopback: bool,
        allow_private: bool,
    },
}

impl HostPolicy {
    fn allow_loopback(self) -> bool {
        match self {
            Self::SkinAllowlist => false,
            Self::AnyPublic { allow_loopback, .. } => allow_loopback,
        }
    }

    fn allow_private(self) -> bool {
        match self {
            Self::SkinAllowlist => false,
            Self::AnyPublic { allow_private, .. } => allow_private,
        }
    }
}

#[derive(Debug)]
enum ProxyError {
    BadRequest(String),
    Forbidden(String),
    NotFound,
    TooManyRequests { retry_after: Option<String> },
    Busy(String),
    Upstream(String),
}

impl fmt::Display for ProxyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadRequest(msg)
            | Self::Forbidden(msg)
            | Self::Busy(msg)
            | Self::Upstream(msg) => f.write_str(msg),
            Self::NotFound => f.write_str("upstream returned HTTP 404 Not Found"),
            Self::TooManyRequests { .. } => {
                f.write_str("upstream returned HTTP 429 Too Many Requests")
            }
        }
    }
}

enum FetchedUpstream {
    Ok { content_type: String, body: Bytes },
    NoContent,
}

#[derive(Debug)]
enum BlockedTargetError {
    DisallowedIp { host: String, ip: IpAddr },
    EmptyResolution { host: String },
}

impl fmt::Display for BlockedTargetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DisallowedIp { host, ip } => {
                write!(f, "resolved address {ip} for {host} is non-public")
            }
            Self::EmptyResolution { host } => {
                write!(f, "host {host} resolved to zero addresses")
            }
        }
    }
}

impl StdError for BlockedTargetError {}

type LookupFuture = Pin<Box<dyn Future<Output = std::io::Result<Vec<SocketAddr>>> + Send>>;

trait HostLookup: Send + Sync + 'static {
    fn lookup(&self, host: String) -> LookupFuture;
}

struct SystemHostLookup;

impl HostLookup for SystemHostLookup {
    fn lookup(&self, host: String) -> LookupFuture {
        Box::pin(async move {
            let iter = tokio::net::lookup_host((host.as_str(), 0)).await?;
            Ok(iter.collect())
        })
    }
}

/// Validates every resolved `SocketAddr` inside `reqwest`'s own DNS step so the
/// HTTP connector dials the exact validated addresses without a second lookup.
struct SsrfSafeDnsResolver<L = SystemHostLookup> {
    allow_loopback: bool,
    allow_private: bool,
    lookup: L,
}

impl SsrfSafeDnsResolver<SystemHostLookup> {
    fn system(allow_loopback: bool, allow_private: bool) -> Self {
        Self {
            allow_loopback,
            allow_private,
            lookup: SystemHostLookup,
        }
    }
}

impl<L: HostLookup> Resolve for SsrfSafeDnsResolver<L> {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        let allow_loopback = self.allow_loopback;
        let allow_private = self.allow_private;
        let fut = self.lookup.lookup(host.clone());
        Box::pin(async move {
            let addrs = fut.await?;
            if addrs.is_empty() {
                return Err(Box::new(BlockedTargetError::EmptyResolution { host })
                    as Box<dyn StdError + Send + Sync>);
            }
            for addr in &addrs {
                if !is_permitted_ip(addr.ip(), allow_loopback, allow_private) {
                    return Err(Box::new(BlockedTargetError::DisallowedIp {
                        host,
                        ip: addr.ip(),
                    }) as Box<dyn StdError + Send + Sync>);
                }
            }
            let iter: Addrs = Box::new(addrs.into_iter());
            Ok(iter)
        })
    }
}

fn build_proxy_client<R: Resolve + 'static>(resolver: R) -> reqwest::Client {
    ensure_rustls_provider();
    reqwest::Client::builder()
        .no_proxy()
        .redirect(Policy::none())
        .connect_timeout(PROXY_CONNECT_TIMEOUT)
        .read_timeout(PROXY_READ_TIMEOUT)
        .dns_resolver(Arc::new(resolver))
        .build()
        .expect("build shared proxy HTTP client")
}

fn shared_proxy_client(allow_loopback: bool, allow_private: bool) -> &'static reqwest::Client {
    static PUBLIC_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    static LOOPBACK_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    static PRIVATE_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    static LOOPBACK_AND_PRIVATE_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    let slot = match (allow_loopback, allow_private) {
        (false, false) => &PUBLIC_CLIENT,
        (true, false) => &LOOPBACK_CLIENT,
        (false, true) => &PRIVATE_CLIENT,
        (true, true) => &LOOPBACK_AND_PRIVATE_CLIENT,
    };
    slot.get_or_init(|| {
        build_proxy_client(SsrfSafeDnsResolver::system(allow_loopback, allow_private))
    })
}

pub async fn handle_skin_proxy(req: &HttpRequest) -> HttpResponse {
    handle_skin_proxy_with_limiter(req, ProxyLimiter::global())
        .await
        .response
}

pub(crate) async fn handle_skin_proxy_with_limiter(
    req: &HttpRequest,
    limiter: &ProxyLimiter,
) -> ProxyResponse {
    handle_skin_proxy_with_client_and_limiter(req, shared_proxy_client(false, false), limiter).await
}

async fn handle_skin_proxy_with_client_and_limiter(
    req: &HttpRequest,
    client: &reqwest::Client,
    limiter: &ProxyLimiter,
) -> ProxyResponse {
    if !crate::is_allowed_proxy_origin(req) {
        return ProxyResponse::plain(HttpResponse::text(
            403,
            "Forbidden",
            "Cross-origin /proxy/skin request rejected",
        ));
    }
    let Some(raw_url) = req.query_param("url") else {
        return ProxyResponse::plain(HttpResponse::text(
            400,
            "Bad Request",
            "Missing url query parameter",
        ));
    };
    let parsed = match Url::parse(&raw_url) {
        Ok(u) if matches!(u.scheme(), "http" | "https") => u,
        _ => {
            return ProxyResponse::plain(HttpResponse::text(
                400,
                "Bad Request",
                "Invalid skin URL",
            ));
        }
    };
    if parsed.host_str().is_none() {
        return ProxyResponse::plain(HttpResponse::text(400, "Bad Request", "Missing skin host"));
    }

    let Some(permit) = limiter.acquire_skin() else {
        return ProxyResponse::plain(busy_response("Skin proxy concurrency limit reached"));
    };
    let mut budget = limiter.inflight_bytes.guard();

    let fetched = tokio::time::timeout(
        limiter.skin_total_timeout,
        fetch_bounded(
            client,
            parsed,
            HeaderMap::new(),
            MAX_SKIN_BYTES,
            HostPolicy::SkinAllowlist,
            MAX_SKIN_REDIRECTS,
            &mut budget,
        ),
    )
    .await;

    match fetched {
        Ok(Ok(FetchedUpstream::Ok { content_type, body })) => {
            let ct = leak_known_content_type(&content_type, "application/octet-stream");
            ProxyResponse::guarded(
                HttpResponse::ok(ct, body).with_header("Cache-Control", "public, max-age=3600"),
                permit,
                budget,
            )
        }
        Ok(Ok(FetchedUpstream::NoContent)) => ProxyResponse::plain(HttpResponse::no_content()),
        Ok(Err(ProxyError::NotFound)) => ProxyResponse::plain(HttpResponse::text(
            404,
            "Not Found",
            "Upstream skin resource not found",
        )),
        Ok(Err(ProxyError::TooManyRequests { retry_after })) => {
            let mut resp = HttpResponse::text(
                429,
                "Too Many Requests",
                "Upstream skin service rate limited",
            );
            if let Some(ra) = retry_after {
                resp = resp.with_header("Retry-After", ra);
            }
            ProxyResponse::plain(resp)
        }
        Ok(Err(ProxyError::BadRequest(msg))) => {
            ProxyResponse::plain(HttpResponse::text(400, "Bad Request", msg))
        }
        Ok(Err(ProxyError::Forbidden(msg))) => {
            ProxyResponse::plain(HttpResponse::text(403, "Forbidden", msg))
        }
        Ok(Err(ProxyError::Busy(msg))) => ProxyResponse::plain(busy_response(msg)),
        Err(_) => ProxyResponse::plain(HttpResponse::text(
            504,
            "Gateway Timeout",
            "Skin proxy request timed out",
        )),
        Ok(Err(ProxyError::Upstream(msg))) => ProxyResponse::plain(HttpResponse::text(
            502,
            "Bad Gateway",
            format!("Skin proxy error: {msg}"),
        )),
    }
}

pub async fn handle_pack_proxy(
    req: &HttpRequest,
    allow_loopback: bool,
    allow_private: bool,
) -> HttpResponse {
    handle_pack_proxy_with_client_and_limiter(
        req,
        allow_loopback,
        allow_private,
        shared_proxy_client(allow_loopback, allow_private),
        ProxyLimiter::global(),
    )
    .await
    .response
}

fn parse_pack_request(req: &HttpRequest) -> Result<(Url, HeaderMap), HttpResponse> {
    if !crate::is_allowed_proxy_origin(req) {
        return Err(HttpResponse::text(
            403,
            "Forbidden",
            "Cross-origin /proxy/pack request rejected",
        ));
    }
    let Some(raw_url) = req.query_param("url") else {
        return Err(HttpResponse::text(
            400,
            "Bad Request",
            "Missing url query parameter",
        ));
    };
    let parsed = match Url::parse(&raw_url) {
        Ok(u) if matches!(u.scheme(), "http" | "https") => u,
        _ => {
            return Err(HttpResponse::text(
                400,
                "Bad Request",
                "Invalid server pack URL",
            ));
        }
    };
    let username = req
        .query_param("username")
        .unwrap_or_else(|| "bbb-web".to_string());
    let uuid = req
        .query_param("uuid")
        .unwrap_or_else(|| uuid::Uuid::nil().simple().to_string());
    let headers = pack_download_headers(&username, &uuid)
        .map_err(|err| HttpResponse::text(400, "Bad Request", format!("Invalid headers: {err}")))?;
    Ok((parsed, headers))
}

fn map_pack_proxy_error(err: ProxyError) -> HttpResponse {
    match err {
        ProxyError::BadRequest(msg) => HttpResponse::text(400, "Bad Request", msg),
        ProxyError::Forbidden(msg) => HttpResponse::text(
            403,
            "Forbidden",
            format!("Blocked server pack target: {msg}"),
        ),
        ProxyError::Busy(msg) => busy_response(msg),
        ProxyError::NotFound => HttpResponse::text(
            502,
            "Bad Gateway",
            "Server pack proxy error: upstream returned HTTP 404 Not Found",
        ),
        ProxyError::TooManyRequests { .. } => HttpResponse::text(
            502,
            "Bad Gateway",
            "Server pack proxy error: upstream returned HTTP 429 Too Many Requests",
        ),
        ProxyError::Upstream(msg) => HttpResponse::text(
            502,
            "Bad Gateway",
            format!("Server pack proxy error: {msg}"),
        ),
    }
}

async fn handle_pack_proxy_with_client_and_limiter(
    req: &HttpRequest,
    allow_loopback: bool,
    allow_private: bool,
    client: &reqwest::Client,
    limiter: &ProxyLimiter,
) -> ProxyResponse {
    let (parsed, headers) = match parse_pack_request(req) {
        Ok(parts) => parts,
        Err(resp) => return ProxyResponse::plain(resp),
    };
    let Some(permit) = limiter.acquire_pack() else {
        return ProxyResponse::plain(busy_response("Server pack proxy concurrency limit reached"));
    };
    let mut budget = limiter.inflight_bytes.guard();

    let fetched = tokio::time::timeout(
        limiter.pack_total_timeout,
        fetch_bounded(
            client,
            parsed,
            headers,
            MAX_PACK_BYTES,
            HostPolicy::AnyPublic {
                allow_loopback,
                allow_private,
            },
            MAX_PACK_REDIRECTS,
            &mut budget,
        ),
    )
    .await;

    match fetched {
        Ok(Ok(FetchedUpstream::Ok { body, .. })) => {
            ProxyResponse::guarded(HttpResponse::ok("application/zip", body), permit, budget)
        }
        Ok(Ok(FetchedUpstream::NoContent)) => ProxyResponse::plain(HttpResponse::text(
            502,
            "Bad Gateway",
            "Server pack proxy error: upstream returned HTTP 204 No Content",
        )),
        Ok(Err(err)) => ProxyResponse::plain(map_pack_proxy_error(err)),
        Err(_) => ProxyResponse::plain(HttpResponse::text(
            504,
            "Gateway Timeout",
            "Server pack proxy request timed out",
        )),
    }
}

/// Streams `/proxy/pack` directly to the client socket with bounded concurrency,
/// total timeout, and per-request byte ceiling so concurrent pack downloads do
/// not buffer 256 MiB each in gateway memory. Returns `Ok(true)` when the HTTP
/// connection remains cleanly reusable for keep-alive.
pub(crate) async fn serve_pack_proxy<W>(
    req: &HttpRequest,
    writer: &mut W,
    keep_alive: bool,
    allow_loopback: bool,
    allow_private: bool,
    limiter: &ProxyLimiter,
) -> Result<bool>
where
    W: AsyncWrite + Unpin,
{
    serve_pack_proxy_with_client(
        req,
        writer,
        keep_alive,
        allow_loopback,
        allow_private,
        shared_proxy_client(allow_loopback, allow_private),
        limiter,
        MAX_PACK_BYTES,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn serve_pack_proxy_with_client<W>(
    req: &HttpRequest,
    writer: &mut W,
    keep_alive: bool,
    allow_loopback: bool,
    allow_private: bool,
    client: &reqwest::Client,
    limiter: &ProxyLimiter,
    max_bytes: usize,
) -> Result<bool>
where
    W: AsyncWrite + Unpin,
{
    let (parsed, headers) = match parse_pack_request(req) {
        Ok(parts) => parts,
        Err(resp) => {
            resp.write_to_with_keep_alive(writer, false, keep_alive)
                .await?;
            return Ok(keep_alive);
        }
    };
    let Some(_permit) = limiter.acquire_pack() else {
        busy_response("Server pack proxy concurrency limit reached")
            .write_to_with_keep_alive(writer, false, keep_alive)
            .await?;
        return Ok(keep_alive);
    };

    let policy = HostPolicy::AnyPublic {
        allow_loopback,
        allow_private,
    };
    let mut headers_sent = false;
    let stream_outcome = tokio::time::timeout(limiter.pack_total_timeout, async {
        let mut resp =
            send_validated_hops(client, parsed, &headers, policy, MAX_PACK_REDIRECTS).await?;
        let status = resp.status();
        if status == StatusCode::NO_CONTENT || !status.is_success() {
            return Err(ProxyError::Upstream(format!(
                "upstream returned HTTP {status}"
            )));
        }
        let expected_len = parse_bounded_content_length(resp.content_length(), max_bytes)?;

        headers_sent = true;
        write_streaming_ok_headers(writer, "application/zip", expected_len, keep_alive)
            .await
            .map_err(|err| ProxyError::Upstream(format!("write pack response headers: {err}")))?;

        let mut written: usize = 0;
        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|err| ProxyError::Upstream(format!("read upstream body chunk: {err}")))?
        {
            if chunk.is_empty() {
                continue;
            }
            let Some(next_written) = written.checked_add(chunk.len()) else {
                return Err(ProxyError::Upstream(format!(
                    "upstream body exceeded {max_bytes} byte limit"
                )));
            };
            if next_written > max_bytes || expected_len.is_some_and(|len| next_written > len) {
                return Err(ProxyError::Upstream(format!(
                    "upstream body exceeded {max_bytes} byte limit"
                )));
            }
            if expected_len.is_some() {
                writer
                    .write_all(&chunk)
                    .await
                    .map_err(|err| ProxyError::Upstream(format!("write pack chunk: {err}")))?;
            } else {
                let prefix = format!("{:x}\r\n", chunk.len());
                writer
                    .write_all(prefix.as_bytes())
                    .await
                    .map_err(|err| ProxyError::Upstream(format!("write chunk size: {err}")))?;
                writer
                    .write_all(&chunk)
                    .await
                    .map_err(|err| ProxyError::Upstream(format!("write pack chunk: {err}")))?;
                writer.write_all(b"\r\n").await.map_err(|err| {
                    ProxyError::Upstream(format!("write chunk terminator: {err}"))
                })?;
            }
            written = next_written;
        }

        if let Some(len) = expected_len {
            if written != len {
                return Err(ProxyError::Upstream(format!(
                    "upstream body truncated: received {written} of {len} bytes"
                )));
            }
        } else {
            writer
                .write_all(b"0\r\n\r\n")
                .await
                .map_err(|err| ProxyError::Upstream(format!("write final chunk: {err}")))?;
        }
        writer
            .flush()
            .await
            .map_err(|err| ProxyError::Upstream(format!("flush pack response: {err}")))?;
        Ok(())
    })
    .await;

    match stream_outcome {
        Ok(Ok(())) => Ok(keep_alive),
        Ok(Err(err)) if !headers_sent => {
            map_pack_proxy_error(err)
                .write_to_with_keep_alive(writer, false, keep_alive)
                .await?;
            Ok(keep_alive)
        }
        Err(_) if !headers_sent => {
            HttpResponse::text(
                504,
                "Gateway Timeout",
                "Server pack proxy request timed out",
            )
            .write_to_with_keep_alive(writer, false, keep_alive)
            .await?;
            Ok(keep_alive)
        }
        Ok(Err(_)) | Err(_) => {
            // Headers were already transmitted; close the connection so the peer
            // sees an incomplete transfer rather than a desynchronized keep-alive stream.
            Ok(false)
        }
    }
}

fn pack_download_headers(username: &str, simple_uuid: &str) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    for (name, value) in [
        ("x-minecraft-username", username.to_string()),
        ("x-minecraft-uuid", simple_uuid.to_string()),
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
            HeaderName::from_bytes(name.as_bytes())?,
            HeaderValue::from_str(&value)?,
        );
    }
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(&format!("Minecraft Java/{}", bbb_protocol::MC_VERSION))?,
    );
    Ok(headers)
}

fn is_allowed_skin_host(host: &str) -> bool {
    ALLOWED_SKIN_HOSTS
        .iter()
        .any(|allowed| host.eq_ignore_ascii_case(allowed))
}

fn validate_hop_url(url: &Url, policy: HostPolicy, is_redirect: bool) -> Result<(), ProxyError> {
    if !matches!(url.scheme(), "http" | "https") {
        let msg = format!("Disallowed URL scheme {}", url.scheme());
        return Err(if is_redirect {
            ProxyError::Forbidden(msg)
        } else {
            ProxyError::BadRequest(msg)
        });
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(ProxyError::Forbidden(
            "Proxy target URLs must not contain credentials".to_string(),
        ));
    }

    let Some(host) = url.host() else {
        return Err(if is_redirect {
            ProxyError::Forbidden("Redirect URL missing host".to_string())
        } else {
            ProxyError::BadRequest("URL missing host".to_string())
        });
    };

    if matches!(policy, HostPolicy::SkinAllowlist) {
        match &host {
            Host::Domain(domain) if is_allowed_skin_host(domain) => {}
            _ => {
                return Err(ProxyError::Forbidden(
                    "Host is not in Mojang skin allowlist".to_string(),
                ));
            }
        }
    }

    // Hyper connects to IP literals directly without invoking `dns::Resolve`, so
    // literal IPv4/IPv6 targets are validated up front on every hop.
    let allow_loopback = policy.allow_loopback();
    let allow_private = policy.allow_private();
    match host {
        Host::Ipv4(v4) => {
            if !is_permitted_ip(IpAddr::V4(v4), allow_loopback, allow_private) {
                return Err(ProxyError::Forbidden(format!(
                    "Target IP {v4} is non-public"
                )));
            }
        }
        Host::Ipv6(v6) => {
            if !is_permitted_ip(IpAddr::V6(v6), allow_loopback, allow_private) {
                return Err(ProxyError::Forbidden(format!(
                    "Target IP {v6} is non-public"
                )));
            }
        }
        Host::Domain(_) => {}
    }

    Ok(())
}

pub(crate) fn is_permitted_ip(ip: IpAddr, allow_loopback: bool, allow_private: bool) -> bool {
    match ip {
        IpAddr::V4(v4) => is_permitted_ipv4(v4, allow_loopback, allow_private),
        IpAddr::V6(v6) => is_permitted_ipv6(v6, allow_loopback, allow_private),
    }
}

fn is_permitted_ipv4(v4: Ipv4Addr, allow_loopback: bool, allow_private: bool) -> bool {
    if allow_loopback && v4.is_loopback() {
        return true;
    }
    if allow_private && v4.is_private() {
        return true;
    }
    let octets = v4.octets();
    let is_this_network = octets[0] == 0; // 0.0.0.0/8 (RFC 1122)
    let is_cgnat = octets[0] == 100 && (64..=127).contains(&octets[1]); // 100.64.0.0/10 (RFC 6598)
    let is_ietf_protocol = octets[0] == 192 && octets[1] == 0 && octets[2] == 0; // 192.0.0.0/24 (RFC 6890)
    let is_6to4_relay = octets[0] == 192 && octets[1] == 88 && octets[2] == 99; // 192.88.99.0/24 (RFC 7526)
    let is_benchmarking = octets[0] == 198 && (octets[1] == 18 || octets[1] == 19); // 198.18.0.0/15 (RFC 2544)
    let is_reserved_or_broadcast = octets[0] >= 240; // 240.0.0.0/4 + 255.255.255.255

    !is_this_network
        && !v4.is_loopback()
        && !v4.is_private()
        && !is_cgnat
        && !v4.is_link_local()
        && !is_ietf_protocol
        && !v4.is_documentation()
        && !is_6to4_relay
        && !is_benchmarking
        && !v4.is_multicast()
        && !is_reserved_or_broadcast
}

fn is_permitted_ipv6(v6: Ipv6Addr, allow_loopback: bool, allow_private: bool) -> bool {
    if let Some(mapped) = v6.to_ipv4_mapped() {
        return is_permitted_ipv4(mapped, allow_loopback, allow_private);
    }
    if allow_loopback && v6.is_loopback() {
        return true;
    }
    let segments = v6.segments();
    // Unique Local Addresses `fc00::/7` (RFC 4193) are permitted when explicitly opted in.
    if allow_private && (segments[0] & 0xfe00) == 0xfc00 {
        return true;
    }
    // Require IANA Global Unicast `2000::/3`, rejecting unspecified `::`, loopback
    // `::1`, NAT64 `64:ff9b::/96` & `64:ff9b:1::/48`, discard `100::/64`, ULA
    // `fc00::/7`, link-local `fe80::/10`, and multicast `ff00::/8`.
    if (segments[0] & 0xe000) != 0x2000 {
        return false;
    }
    let is_ietf_or_teredo_or_benchmarking = segments[0] == 0x2001 && segments[1] < 0x0200; // 2001::/23
    let is_doc_2001_db8 = segments[0] == 0x2001 && segments[1] == 0x0db8; // 2001:db8::/32
    let is_doc_3fff = segments[0] == 0x3fff && (segments[1] & 0xf000) == 0; // 3fff::/20
    let is_6to4 = segments[0] == 0x2002; // 2002::/16

    !is_ietf_or_teredo_or_benchmarking && !is_doc_2001_db8 && !is_doc_3fff && !is_6to4
}

fn classify_send_error(err: reqwest::Error) -> ProxyError {
    let mut cursor: Option<&(dyn StdError + 'static)> = Some(&err);
    while let Some(source) = cursor {
        if let Some(blocked) = source.downcast_ref::<BlockedTargetError>() {
            return ProxyError::Forbidden(blocked.to_string());
        }
        cursor = source.source();
    }
    ProxyError::Upstream(format!("send upstream HTTP request: {err}"))
}

async fn send_validated_hops(
    client: &reqwest::Client,
    mut current_url: Url,
    headers: &HeaderMap,
    policy: HostPolicy,
    max_redirects: usize,
) -> Result<reqwest::Response, ProxyError> {
    for hop in 0..=max_redirects {
        validate_hop_url(&current_url, policy, hop > 0)?;

        let resp = client
            .get(current_url.clone())
            .headers(headers.clone())
            .send()
            .await
            .map_err(classify_send_error)?;

        if resp.status().is_redirection() {
            if hop == max_redirects {
                return Err(ProxyError::Upstream("too many redirects".to_string()));
            }
            let loc = resp
                .headers()
                .get(reqwest::header::LOCATION)
                .ok_or_else(|| {
                    ProxyError::Upstream("redirect missing Location header".to_string())
                })?
                .to_str()
                .map_err(|_| ProxyError::Upstream("redirect Location is not UTF-8".to_string()))?;
            let next_url = current_url
                .join(loc)
                .map_err(|err| ProxyError::Upstream(format!("resolve redirect URL: {err}")))?;
            current_url = next_url;
            continue;
        }

        return Ok(resp);
    }

    Err(ProxyError::Upstream("redirect loop exhausted".to_string()))
}

fn parse_bounded_content_length(
    content_length: Option<u64>,
    max_bytes: usize,
) -> Result<Option<usize>, ProxyError> {
    let Some(len) = content_length else {
        return Ok(None);
    };
    let len_usize = usize::try_from(len).map_err(|_| {
        ProxyError::Upstream(format!(
            "upstream Content-Length {len} exceeds {max_bytes} byte limit"
        ))
    })?;
    if len_usize > max_bytes {
        return Err(ProxyError::Upstream(format!(
            "upstream Content-Length {len} exceeds {max_bytes} byte limit"
        )));
    }
    Ok(Some(len_usize))
}

fn sanitize_retry_after(value: &HeaderValue) -> Option<String> {
    const MAX_RETRY_AFTER_LEN: usize = 64;
    let raw = value.to_str().ok()?.trim();
    if raw.is_empty() || raw.len() > MAX_RETRY_AFTER_LEN {
        return None;
    }
    // RFC 9110 Section 10.2.3: delay-seconds (non-negative integer) or HTTP-date.
    if raw.bytes().all(|b| b.is_ascii_digit()) {
        return raw.parse::<u64>().ok().map(|_| raw.to_string());
    }
    is_valid_http_date(raw).then(|| raw.to_string())
}

fn is_valid_http_date(raw: &str) -> bool {
    if !raw
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b' ' | b',' | b':' | b'-'))
    {
        return false;
    }
    const WEEKDAYS: &[&str] = &["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let Some(first_word) = raw.split([',', ' ']).next() else {
        return false;
    };
    if !WEEKDAYS.iter().any(|day| first_word.starts_with(day)) {
        return false;
    }
    raw.split(' ').any(|token| {
        let bytes = token.as_bytes();
        bytes.len() == 8
            && bytes[2] == b':'
            && bytes[5] == b':'
            && bytes[0].is_ascii_digit()
            && bytes[1].is_ascii_digit()
            && bytes[3].is_ascii_digit()
            && bytes[4].is_ascii_digit()
            && bytes[6].is_ascii_digit()
            && bytes[7].is_ascii_digit()
    })
}

async fn fetch_bounded(
    client: &reqwest::Client,
    current_url: Url,
    headers: HeaderMap,
    max_bytes: usize,
    policy: HostPolicy,
    max_redirects: usize,
    budget: &mut ByteBudgetGuard,
) -> Result<FetchedUpstream, ProxyError> {
    let mut resp =
        send_validated_hops(client, current_url, &headers, policy, max_redirects).await?;

    match resp.status() {
        StatusCode::NO_CONTENT => return Ok(FetchedUpstream::NoContent),
        StatusCode::NOT_FOUND => return Err(ProxyError::NotFound),
        StatusCode::TOO_MANY_REQUESTS => {
            let retry_after = resp
                .headers()
                .get(RETRY_AFTER)
                .and_then(sanitize_retry_after);
            return Err(ProxyError::TooManyRequests { retry_after });
        }
        status if !status.is_success() => {
            return Err(ProxyError::Upstream(format!(
                "upstream returned HTTP {status}"
            )));
        }
        _ => {}
    }

    let expected_len = parse_bounded_content_length(resp.content_length(), max_bytes)?;

    let content_type = resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let mut out = Vec::new();
    if let Some(len) = expected_len {
        // Cap initial reservation so a lying Content-Length header cannot force
        // a 256 MiB upfront allocation before body bytes actually arrive.
        let initial = len.min(MAX_INITIAL_PROXY_RESERVE_BYTES);
        reserve_proxy_capacity(&mut out, initial, budget)?;
    }
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|err| ProxyError::Upstream(format!("read upstream body chunk: {err}")))?
    {
        append_bounded_chunk(&mut out, &chunk, max_bytes, budget)?;
    }
    Ok(FetchedUpstream::Ok {
        content_type,
        body: Bytes::from(out),
    })
}

fn reserve_proxy_capacity(
    out: &mut Vec<u8>,
    target_cap: usize,
    budget: &mut ByteBudgetGuard,
) -> Result<(), ProxyError> {
    if target_cap <= out.capacity() {
        return Ok(());
    }
    let cap_delta = target_cap - out.capacity();
    if !budget.try_reserve(cap_delta) {
        return Err(ProxyError::Busy(
            "Proxy in-flight memory budget exhausted".to_string(),
        ));
    }
    let additional = target_cap - out.len();
    if out.try_reserve_exact(additional).is_err() {
        budget.release(cap_delta);
        return Err(ProxyError::Upstream(format!(
            "failed to allocate {target_cap} bytes for upstream body"
        )));
    }
    Ok(())
}

fn append_bounded_chunk(
    out: &mut Vec<u8>,
    chunk: &[u8],
    max_bytes: usize,
    budget: &mut ByteBudgetGuard,
) -> Result<(), ProxyError> {
    let Some(needed) = out.len().checked_add(chunk.len()) else {
        return Err(ProxyError::Upstream(format!(
            "upstream body exceeded {max_bytes} byte limit"
        )));
    };
    if needed > max_bytes {
        return Err(ProxyError::Upstream(format!(
            "upstream body exceeded {max_bytes} byte limit"
        )));
    }
    if needed > out.capacity() {
        // Cap geometric growth at `max_bytes` so chunked/unknown-length downloads
        // never overshoot into a 2x allocation peak.
        let target_cap = out.capacity().saturating_mul(2).max(needed).min(max_bytes);
        reserve_proxy_capacity(out, target_cap, budget)?;
    }
    out.extend_from_slice(chunk);
    Ok(())
}

fn leak_known_content_type(ct: &str, default_mime: &'static str) -> &'static str {
    let lower = ct.to_ascii_lowercase();
    if lower.starts_with("image/png") {
        "image/png"
    } else if lower.starts_with("application/json") {
        "application/json; charset=utf-8"
    } else if lower.starts_with("application/zip") {
        "application/zip"
    } else {
        default_mime
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use super::*;
    use crate::http::read_http_request;

    async fn handle_skin_proxy_with_client(
        req: &HttpRequest,
        client: &reqwest::Client,
    ) -> HttpResponse {
        handle_skin_proxy_with_client_and_limiter(req, client, &ProxyLimiter::default())
            .await
            .response
    }

    async fn handle_pack_proxy_with_client(
        req: &HttpRequest,
        allow_loopback: bool,
        allow_private: bool,
        client: &reqwest::Client,
    ) -> HttpResponse {
        handle_pack_proxy_with_client_and_limiter(
            req,
            allow_loopback,
            allow_private,
            client,
            &ProxyLimiter::default(),
        )
        .await
        .response
    }

    async fn parse_get(target: &str) -> HttpRequest {
        let (mut client, mut server) = tokio::io::duplex(4096);
        let wire = format!("GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n");
        client.write_all(wire.as_bytes()).await.expect("write");
        drop(client);
        read_http_request(&mut server)
            .await
            .expect("parse")
            .expect("request")
            .0
    }

    struct MapLookup {
        entries: HashMap<String, Vec<SocketAddr>>,
        calls: Arc<AtomicUsize>,
    }

    impl HostLookup for MapLookup {
        fn lookup(&self, host: String) -> LookupFuture {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let found = self
                .entries
                .get(&host.to_ascii_lowercase())
                .cloned()
                .unwrap_or_default();
            Box::pin(async move { Ok(found) })
        }
    }

    #[test]
    fn is_permitted_ip_rejects_cgnat_and_special_ranges() {
        // CGNAT 100.64.0.0/10 must be rejected even when loopback or private is enabled,
        // while adjacent public addresses outside /10 remain permitted.
        for blocked_cgnat in [
            "100.64.0.0",
            "100.64.0.1",
            "100.100.100.100",
            "100.127.255.255",
        ] {
            let ip: IpAddr = blocked_cgnat.parse().unwrap();
            assert!(
                !is_permitted_ip(ip, false, false),
                "{blocked_cgnat} must be rejected"
            );
            assert!(
                !is_permitted_ip(ip, true, false),
                "{blocked_cgnat} must be rejected even with allow_loopback"
            );
            assert!(
                !is_permitted_ip(ip, false, true),
                "{blocked_cgnat} must be rejected even with allow_private"
            );
        }
        assert!(is_permitted_ip(
            "100.63.255.255".parse().unwrap(),
            false,
            false
        ));
        assert!(is_permitted_ip(
            "100.128.0.0".parse().unwrap(),
            false,
            false
        ));
        assert!(is_permitted_ip(
            "93.184.216.34".parse().unwrap(),
            false,
            false
        ));

        // Special non-global IPv4 ranges.
        for special_v4 in [
            "0.0.0.0",
            "0.1.2.3",
            "10.0.0.1",
            "127.0.0.1",
            "169.254.169.254",
            "172.16.0.1",
            "172.31.255.255",
            "192.0.0.1",
            "192.0.2.1",
            "192.88.99.1",
            "192.168.1.1",
            "198.18.0.1",
            "198.19.255.255",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "240.0.0.1",
            "255.255.255.255",
        ] {
            let ip: IpAddr = special_v4.parse().unwrap();
            assert!(
                !is_permitted_ip(ip, false, false),
                "{special_v4} must be rejected"
            );
        }
        assert!(is_permitted_ip("127.0.0.1".parse().unwrap(), true, false));

        // Special non-global IPv6 ranges (including IPv4-mapped CGNAT, NAT64, 6to4, Teredo, ULA).
        for special_v6 in [
            "::",
            "::1",
            "::ffff:100.64.0.1",
            "::ffff:127.0.0.1",
            "64:ff9b::6440:1",
            "100::1",
            "2001::1",
            "2001:2::1",
            "2001:db8::1",
            "2002:7f00:1::1",
            "3fff::1",
            "fc00::1",
            "fd12:3456::1",
            "fe80::1",
            "ff02::1",
        ] {
            let ip: IpAddr = special_v6.parse().unwrap();
            assert!(
                !is_permitted_ip(ip, false, false),
                "{special_v6} must be rejected"
            );
        }
        assert!(is_permitted_ip("::1".parse().unwrap(), true, false));
        assert!(is_permitted_ip(
            "::ffff:127.0.0.1".parse().unwrap(),
            true,
            false
        ));
        assert!(!is_permitted_ip(
            "::ffff:100.64.0.1".parse().unwrap(),
            true,
            false
        ));
        assert!(is_permitted_ip(
            "2606:4700:4700::1111".parse().unwrap(),
            false,
            false
        ));
        assert!(is_permitted_ip(
            "2001:4860:4860::8888".parse().unwrap(),
            false,
            false
        ));
    }

    #[test]
    fn allow_pack_private_permits_rfc1918_and_ula_while_blocking_other_non_public() {
        for private_addr in [
            "10.0.0.1",
            "10.255.255.254",
            "172.16.0.1",
            "172.31.255.254",
            "192.168.0.1",
            "192.168.255.254",
            "fc00::1",
            "fd12:3456::1",
            "fdff:ffff::1",
            "::ffff:10.0.0.1",
            "::ffff:172.16.0.1",
            "::ffff:192.168.1.1",
        ] {
            let ip: IpAddr = private_addr.parse().unwrap();
            assert!(
                !is_permitted_ip(ip, false, false),
                "{private_addr} must be rejected by default"
            );
            assert!(
                !is_permitted_ip(ip, true, false),
                "{private_addr} must remain rejected when only allow_loopback is enabled"
            );
            assert!(
                is_permitted_ip(ip, false, true),
                "{private_addr} must be permitted when allow_private is enabled"
            );
            assert!(
                is_permitted_ip(ip, true, true),
                "{private_addr} must be permitted when both flags are enabled"
            );
        }

        // Loopback stays blocked under `allow_private` unless `allow_loopback` is also set.
        for loopback_addr in ["127.0.0.1", "127.255.255.254", "::1", "::ffff:127.0.0.1"] {
            let ip: IpAddr = loopback_addr.parse().unwrap();
            assert!(
                !is_permitted_ip(ip, false, true),
                "{loopback_addr} must stay blocked without allow_loopback"
            );
            assert!(
                is_permitted_ip(ip, true, true),
                "{loopback_addr} must be permitted with both allow_loopback and allow_private"
            );
        }

        // Link-local/cloud metadata, CGNAT, multicast, unspecified, and reserved ranges
        // remain blocked even when both `allow_loopback` and `allow_private` are enabled.
        for always_blocked in [
            "0.0.0.0",
            "0.1.2.3",
            "100.64.0.1",
            "100.127.255.255",
            "169.254.0.1",
            "169.254.169.254",
            "192.0.2.1",
            "198.18.0.1",
            "224.0.0.1",
            "240.0.0.1",
            "255.255.255.255",
            "::",
            "::ffff:100.64.0.1",
            "::ffff:169.254.169.254",
            "fe80::1",
            "fec0::1",
            "ff02::1",
            "2001:db8::1",
        ] {
            let ip: IpAddr = always_blocked.parse().unwrap();
            assert!(
                !is_permitted_ip(ip, false, true),
                "{always_blocked} must stay blocked with allow_private"
            );
            assert!(
                !is_permitted_ip(ip, true, true),
                "{always_blocked} must stay blocked even with allow_loopback + allow_private"
            );
        }

        // Skin proxy policy must never relax for private or loopback targets.
        for target in [
            "http://192.168.1.10/skin.png",
            "http://10.0.0.5/skin.png",
            "http://[fd12:3456::1]/skin.png",
            "http://127.0.0.1/skin.png",
        ] {
            let url = Url::parse(target).unwrap();
            assert!(matches!(
                validate_hop_url(&url, HostPolicy::SkinAllowlist, false),
                Err(ProxyError::Forbidden(_))
            ));
        }

        let pack_private_policy = HostPolicy::AnyPublic {
            allow_loopback: false,
            allow_private: true,
        };
        assert!(validate_hop_url(
            &Url::parse("http://192.168.1.10/pack.zip").unwrap(),
            pack_private_policy,
            false
        )
        .is_ok());
        assert!(validate_hop_url(
            &Url::parse("http://[fd12:3456::1]/pack.zip").unwrap(),
            pack_private_policy,
            false
        )
        .is_ok());
        for blocked_url in [
            "http://127.0.0.1/pack.zip",
            "http://169.254.169.254/latest/meta-data",
            "http://[fe80::1]/pack.zip",
            "http://100.64.0.1/pack.zip",
        ] {
            assert!(matches!(
                validate_hop_url(
                    &Url::parse(blocked_url).unwrap(),
                    pack_private_policy,
                    false
                ),
                Err(ProxyError::Forbidden(_))
            ));
        }
    }

    #[tokio::test]
    async fn skin_proxy_rejects_redirect_host_escape_and_allows_mojang_redirect() {
        let escaped_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let escaped_addr = escaped_listener.local_addr().unwrap();
        let escaped_hits = Arc::new(AtomicUsize::new(0));
        let escaped_hits_bg = Arc::clone(&escaped_hits);
        let escaped_task = tokio::spawn(async move {
            if escaped_listener.accept().await.is_ok() {
                escaped_hits_bg.fetch_add(1, Ordering::SeqCst);
            }
        });

        let mojang_hop1 = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mojang_hop1_addr = mojang_hop1.local_addr().unwrap();
        let mojang_hop1_task = tokio::spawn(async move {
            let (mut sock, _) = mojang_hop1.accept().await.unwrap();
            let mut scratch = [0u8; 1024];
            let _ = sock.read(&mut scratch).await;
            let body = br#"{"id":"abc"}"#;
            let reply = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n",
                body.len()
            );
            sock.write_all(reply.as_bytes()).await.unwrap();
            sock.write_all(body).await.unwrap();
        });

        let hop0 = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let hop0_addr = hop0.local_addr().unwrap();
        let hop0_task = tokio::spawn(async move {
            for idx in 0..3 {
                let (mut sock, _) = hop0.accept().await.unwrap();
                let mut scratch = [0u8; 1024];
                let _ = sock.read(&mut scratch).await;
                let location = match idx {
                    0 => format!("http://evil.example:{}/steal.png", escaped_addr.port()),
                    1 => format!("http://{escaped_addr}/loopback.png"),
                    _ => format!(
                        "http://sessionserver.mojang.com:{}/session/minecraft/profile/abc",
                        mojang_hop1_addr.port()
                    ),
                };
                let reply = format!(
                    "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\n\r\n"
                );
                sock.write_all(reply.as_bytes()).await.unwrap();
            }
        });

        ensure_rustls_provider();
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .timeout(Duration::from_secs(5))
            .dns_resolver(Arc::new(SsrfSafeDnsResolver::system(false, false)))
            .resolve_to_addrs("textures.minecraft.net", &[hop0_addr])
            .resolve_to_addrs("sessionserver.mojang.com", &[mojang_hop1_addr])
            .resolve_to_addrs("evil.example", &[escaped_addr])
            .build()
            .unwrap();

        let target_url = format!(
            "http://textures.minecraft.net:{}/textures/skin/abc",
            hop0_addr.port()
        );
        let encoded: String = url::form_urlencoded::byte_serialize(target_url.as_bytes()).collect();

        // 1. Redirect to `evil.example` must be blocked with 403 before connecting.
        let req1 = parse_get(&format!("/proxy/skin?url={encoded}")).await;
        let resp1 = handle_skin_proxy_with_client(&req1, &client).await;
        assert_eq!(resp1.status, 403);

        // 2. Redirect to `127.0.0.1` must also be blocked with 403 before connecting.
        let req2 = parse_get(&format!("/proxy/skin?url={encoded}")).await;
        let resp2 = handle_skin_proxy_with_client(&req2, &client).await;
        assert_eq!(resp2.status, 403);

        // 3. Redirect to `sessionserver.mojang.com` remains within `ALLOWED_SKIN_HOSTS` and succeeds.
        let req3 = parse_get(&format!("/proxy/skin?url={encoded}")).await;
        let resp3 = handle_skin_proxy_with_client(&req3, &client).await;
        assert_eq!(resp3.status, 200);
        assert_eq!(&resp3.body[..], br#"{"id":"abc"}"#);

        assert_eq!(
            escaped_hits.load(Ordering::SeqCst),
            0,
            "escaped redirect target must never receive a socket connection"
        );

        hop0_task.await.unwrap();
        mojang_hop1_task.await.unwrap();
        escaped_task.abort();
    }

    #[tokio::test]
    async fn dns_resolver_rejects_cgnat_and_mixed_records_in_single_lookup() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut entries = HashMap::new();
        entries.insert(
            "cgnat.pack.example".to_string(),
            vec!["100.64.10.20:0".parse().unwrap()],
        );
        entries.insert(
            "mixed.pack.example".to_string(),
            vec![
                "93.184.216.34:0".parse().unwrap(),
                "100.64.10.21:0".parse().unwrap(),
            ],
        );
        entries.insert(
            "lan.pack.example".to_string(),
            vec!["192.168.1.50:0".parse().unwrap()],
        );
        let client = build_proxy_client(SsrfSafeDnsResolver {
            allow_loopback: false,
            allow_private: false,
            lookup: MapLookup {
                entries,
                calls: Arc::clone(&calls),
            },
        });

        let req_cgnat =
            parse_get("/proxy/pack?url=http%3A%2F%2Fcgnat.pack.example%2Fpack.zip").await;
        let resp_cgnat = handle_pack_proxy_with_client(&req_cgnat, false, false, &client).await;
        assert_eq!(resp_cgnat.status, 403);

        let req_mixed =
            parse_get("/proxy/pack?url=http%3A%2F%2Fmixed.pack.example%2Fpack.zip").await;
        let resp_mixed = handle_pack_proxy_with_client(&req_mixed, false, false, &client).await;
        assert_eq!(resp_mixed.status, 403);

        let req_lan = parse_get("/proxy/pack?url=http%3A%2F%2Flan.pack.example%2Fpack.zip").await;
        let resp_lan = handle_pack_proxy_with_client(&req_lan, false, false, &client).await;
        assert_eq!(resp_lan.status, 403);
        assert_eq!(calls.load(Ordering::SeqCst), 3);

        // Literal CGNAT and RFC1918 IP URLs are rejected before consulting DNS.
        let req_literal = parse_get("/proxy/pack?url=http%3A%2F%2F100.64.0.5%2Fpack.zip").await;
        let resp_literal = handle_pack_proxy_with_client(&req_literal, false, false, &client).await;
        assert_eq!(resp_literal.status, 403);
        let req_lan_literal =
            parse_get("/proxy/pack?url=http%3A%2F%2F192.168.1.50%2Fpack.zip").await;
        let resp_lan_literal =
            handle_pack_proxy_with_client(&req_lan_literal, false, false, &client).await;
        assert_eq!(resp_lan_literal.status, 403);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn dns_resolver_respects_allow_private_and_keeps_metadata_and_loopback_blocked() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut entries = HashMap::new();
        entries.insert(
            "lan.pack.example".to_string(),
            vec!["192.168.1.50:0".parse().unwrap()],
        );
        entries.insert(
            "ula.pack.example".to_string(),
            vec!["[fd12:3456::1]:0".parse().unwrap()],
        );
        entries.insert(
            "metadata.pack.example".to_string(),
            vec!["169.254.169.254:0".parse().unwrap()],
        );
        entries.insert(
            "loopback.pack.example".to_string(),
            vec!["127.0.0.1:0".parse().unwrap()],
        );

        let resolver = SsrfSafeDnsResolver {
            allow_loopback: false,
            allow_private: true,
            lookup: MapLookup {
                entries: entries.clone(),
                calls: Arc::clone(&calls),
            },
        };
        let lan_addrs: Vec<SocketAddr> = resolver
            .resolve("lan.pack.example".parse().unwrap())
            .await
            .expect("LAN RFC1918 address should resolve when allow_private=true")
            .collect();
        assert_eq!(lan_addrs, vec!["192.168.1.50:0".parse().unwrap()]);
        let ula_addrs: Vec<SocketAddr> = resolver
            .resolve("ula.pack.example".parse().unwrap())
            .await
            .expect("IPv6 ULA address should resolve when allow_private=true")
            .collect();
        assert_eq!(ula_addrs, vec!["[fd12:3456::1]:0".parse().unwrap()]);
        assert!(resolver
            .resolve("metadata.pack.example".parse().unwrap())
            .await
            .is_err());
        assert!(resolver
            .resolve("loopback.pack.example".parse().unwrap())
            .await
            .is_err());

        // End-to-end through `handle_pack_proxy_with_client`: metadata and loopback
        // still return 403 even with `allow_private = true`.
        let client = build_proxy_client(SsrfSafeDnsResolver {
            allow_loopback: false,
            allow_private: true,
            lookup: MapLookup { entries, calls },
        });
        let req_meta =
            parse_get("/proxy/pack?url=http%3A%2F%2Fmetadata.pack.example%2Fpack.zip").await;
        let resp_meta = handle_pack_proxy_with_client(&req_meta, false, true, &client).await;
        assert_eq!(resp_meta.status, 403);

        let req_loop =
            parse_get("/proxy/pack?url=http%3A%2F%2Floopback.pack.example%2Fpack.zip").await;
        let resp_loop = handle_pack_proxy_with_client(&req_loop, false, true, &client).await;
        assert_eq!(resp_loop.status, 403);
    }

    #[tokio::test]
    async fn fetch_bounded_caps_capacity_and_enforces_limits_for_fixed_and_chunked_bodies() {
        let budget_pool = Arc::new(InFlightByteBudget::new(1024));
        let mut budget = budget_pool.guard();
        let mut out = Vec::new();
        for chunk_len in [30usize, 30, 30, 10] {
            append_bounded_chunk(&mut out, &vec![b'a'; chunk_len], 100, &mut budget).unwrap();
            assert!(
                out.capacity() <= 100,
                "chunked growth capacity {} must never overshoot max_bytes=100",
                out.capacity()
            );
        }
        assert_eq!(out.len(), 100);
        assert!(matches!(
            append_bounded_chunk(&mut out, b"x", 100, &mut budget),
            Err(ProxyError::Upstream(_))
        ));
        drop(budget);
        assert_eq!(budget_pool.used_bytes.load(Ordering::Acquire), 0);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            for idx in 0..4 {
                let (mut sock, _) = listener.accept().await.unwrap();
                let mut scratch = [0u8; 1024];
                let _ = sock.read(&mut scratch).await;
                match idx {
                    0 => {
                        let body = vec![b'z'; 64];
                        let head = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Length: {}\r\n\r\n",
                            body.len()
                        );
                        sock.write_all(head.as_bytes()).await.unwrap();
                        sock.write_all(&body).await.unwrap();
                    }
                    1 => {
                        let head = "HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Length: 200\r\n\r\n";
                        sock.write_all(head.as_bytes()).await.unwrap();
                    }
                    2 => {
                        let head = "HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nTransfer-Encoding: chunked\r\n\r\n";
                        sock.write_all(head.as_bytes()).await.unwrap();
                        sock.write_all(b"28\r\n").await.unwrap();
                        sock.write_all(&[b'c'; 40]).await.unwrap();
                        sock.write_all(b"\r\n3c\r\n").await.unwrap();
                        sock.write_all(&[b'd'; 60]).await.unwrap();
                        sock.write_all(b"\r\n0\r\n\r\n").await.unwrap();
                    }
                    _ => {
                        let head = "HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nTransfer-Encoding: chunked\r\n\r\n";
                        sock.write_all(head.as_bytes()).await.unwrap();
                        sock.write_all(b"50\r\n").await.unwrap();
                        sock.write_all(&[b'e'; 80]).await.unwrap();
                        sock.write_all(b"\r\n50\r\n").await.unwrap();
                        let _ = sock.write_all(&[b'f'; 80]).await;
                        let _ = sock.write_all(b"\r\n0\r\n\r\n").await;
                    }
                }
            }
        });

        let client = build_proxy_client(SsrfSafeDnsResolver::system(true, false));
        let url = Url::parse(&format!("http://{addr}/pack.zip")).unwrap();
        let policy = HostPolicy::AnyPublic {
            allow_loopback: true,
            allow_private: false,
        };

        let mut g1 = budget_pool.guard();
        let FetchedUpstream::Ok { body: fixed_ok, .. } = fetch_bounded(
            &client,
            url.clone(),
            HeaderMap::new(),
            128,
            policy,
            0,
            &mut g1,
        )
        .await
        .unwrap() else {
            panic!("expected Ok body");
        };
        assert_eq!(fixed_ok.len(), 64);
        drop(g1);

        let mut g2 = budget_pool.guard();
        let fixed_over = fetch_bounded(
            &client,
            url.clone(),
            HeaderMap::new(),
            128,
            policy,
            0,
            &mut g2,
        )
        .await;
        assert!(matches!(fixed_over, Err(ProxyError::Upstream(_))));
        drop(g2);

        let mut g3 = budget_pool.guard();
        let FetchedUpstream::Ok {
            body: chunked_ok, ..
        } = fetch_bounded(
            &client,
            url.clone(),
            HeaderMap::new(),
            128,
            policy,
            0,
            &mut g3,
        )
        .await
        .unwrap()
        else {
            panic!("expected Ok chunked body");
        };
        assert_eq!(chunked_ok.len(), 100);
        assert_eq!(&chunked_ok[..40], &vec![b'c'; 40][..]);
        assert_eq!(&chunked_ok[40..], &vec![b'd'; 60][..]);
        drop(g3);

        let mut g4 = budget_pool.guard();
        let chunked_over =
            fetch_bounded(&client, url, HeaderMap::new(), 128, policy, 0, &mut g4).await;
        assert!(matches!(chunked_over, Err(ProxyError::Upstream(_))));
        drop(g4);
        assert_eq!(budget_pool.used_bytes.load(Ordering::Acquire), 0);

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn skin_proxy_preserves_204_404_and_429_with_sanitized_retry_after() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            let replies: &[&str] = &[
                "HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 404 Not Found\r\nConnection: close\r\nContent-Length: 9\r\n\r\nnot found",
                "HTTP/1.1 429 Too Many Requests\r\nConnection: close\r\nRetry-After: 45\r\nContent-Length: 4\r\n\r\nslow",
                "HTTP/1.1 429 Too Many Requests\r\nConnection: close\r\nRetry-After: Wed, 21 Oct 2015 07:28:00 GMT\r\nContent-Length: 4\r\n\r\nslow",
                "HTTP/1.1 429 Too Many Requests\r\nConnection: close\r\nRetry-After: invalid-value\r\nContent-Length: 4\r\n\r\nslow",
                "HTTP/1.1 500 Internal Server Error\r\nConnection: close\r\nContent-Length: 4\r\n\r\nboom",
            ];
            for reply in replies {
                let (mut sock, _) = listener.accept().await.unwrap();
                let mut scratch = [0u8; 1024];
                let _ = sock.read(&mut scratch).await;
                sock.write_all(reply.as_bytes()).await.unwrap();
            }
        });

        ensure_rustls_provider();
        let client = reqwest::Client::builder()
            .no_proxy()
            .pool_max_idle_per_host(0)
            .redirect(Policy::none())
            .timeout(Duration::from_secs(5))
            .dns_resolver(Arc::new(SsrfSafeDnsResolver::system(false, false)))
            .resolve_to_addrs("api.mojang.com", &[addr])
            .build()
            .unwrap();

        let target_url = format!(
            "http://api.mojang.com:{}/users/profiles/minecraft/Notch",
            addr.port()
        );
        let encoded: String = url::form_urlencoded::byte_serialize(target_url.as_bytes()).collect();
        let req = parse_get(&format!("/proxy/skin?url={encoded}")).await;

        // 204 No Content must stay 204 and serialize without Content-Length/Content-Type.
        let r_204 = handle_skin_proxy_with_client(&req, &client).await;
        assert_eq!(r_204.status, 204);
        assert!(r_204.body.is_empty());
        let mut wire = Vec::new();
        r_204
            .write_to_with_keep_alive(&mut wire, false, true)
            .await
            .unwrap();
        let wire_str = String::from_utf8(wire).unwrap();
        assert!(wire_str.starts_with("HTTP/1.1 204 No Content\r\n"));
        assert!(!wire_str.to_ascii_lowercase().contains("content-length:"));
        assert!(!wire_str.to_ascii_lowercase().contains("content-type:"));

        // 404 Not Found passes through.
        let r_404 = handle_skin_proxy_with_client(&req, &client).await;
        assert_eq!(r_404.status, 404);

        // 429 Too Many Requests passes through with sanitized delay-seconds Retry-After.
        let r_429_secs = handle_skin_proxy_with_client(&req, &client).await;
        assert_eq!(r_429_secs.status, 429);
        assert!(r_429_secs
            .extra_headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("retry-after") && v == "45"));

        // 429 Too Many Requests passes through with sanitized HTTP-date Retry-After.
        let r_429_date = handle_skin_proxy_with_client(&req, &client).await;
        assert_eq!(r_429_date.status, 429);
        assert!(r_429_date.extra_headers.iter().any(|(k, v)| {
            k.eq_ignore_ascii_case("retry-after") && v == "Wed, 21 Oct 2015 07:28:00 GMT"
        }));

        // 429 Too Many Requests drops invalid Retry-After values.
        let r_429_bad = handle_skin_proxy_with_client(&req, &client).await;
        assert_eq!(r_429_bad.status, 429);
        assert!(r_429_bad
            .extra_headers
            .iter()
            .all(|(k, _)| !k.eq_ignore_ascii_case("retry-after")));

        // 500 still maps to 502 Bad Gateway.
        let r_500 = handle_skin_proxy_with_client(&req, &client).await;
        assert_eq!(r_500.status, 502);

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn proxy_limiter_enforces_concurrency_timeout_and_inflight_byte_budget() {
        let slow_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let slow_addr = slow_listener.local_addr().unwrap();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
        let slow_task = tokio::spawn(async move {
            let (mut sock, _) = slow_listener.accept().await.unwrap();
            let mut scratch = [0u8; 1024];
            let _ = sock.read(&mut scratch).await;
            let _ = release_rx.await;
            let _ = sock
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                .await;
        });

        let client = build_proxy_client(SsrfSafeDnsResolver::system(true, false));
        let limiter = Arc::new(ProxyLimiter::new(
            1,
            1,
            64,
            Duration::from_millis(120),
            Duration::from_millis(120),
        ));

        // 1. Concurrency saturation: hold the single pack permit, verify 2nd request gets 503.
        let held_permit = limiter.acquire_pack().expect("initial pack permit");
        let pack_url = format!("http://{slow_addr}/pack.zip");
        let encoded_pack: String =
            url::form_urlencoded::byte_serialize(pack_url.as_bytes()).collect();
        let pack_req = parse_get(&format!("/proxy/pack?url={encoded_pack}")).await;
        let busy_resp =
            handle_pack_proxy_with_client_and_limiter(&pack_req, true, false, &client, &limiter)
                .await;
        assert_eq!(busy_resp.response.status, 503);
        assert!(busy_resp
            .response
            .extra_headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("retry-after") && v == "1"));
        drop(held_permit);

        // 2. Total timeout: slow upstream exceeds 120ms budget and returns 504.
        let timed_out =
            handle_pack_proxy_with_client_and_limiter(&pack_req, true, false, &client, &limiter)
                .await;
        assert_eq!(timed_out.response.status, 504);
        let _ = release_tx.send(());
        slow_task.await.unwrap();

        // 3. Aggregate in-flight byte budget exhaustion returns 503 and releases on drop.
        let fast_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let fast_addr = fast_listener.local_addr().unwrap();
        let fast_task = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut sock, _) = fast_listener.accept().await.unwrap();
                let mut scratch = [0u8; 1024];
                let _ = sock.read(&mut scratch).await;
                let body = vec![b'p'; 48];
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Length: {}\r\n\r\n",
                    body.len()
                );
                sock.write_all(head.as_bytes()).await.unwrap();
                sock.write_all(&body).await.unwrap();
            }
        });

        let budget_limiter =
            ProxyLimiter::new(2, 2, 64, Duration::from_secs(5), Duration::from_secs(5));
        let fast_url = format!("http://{fast_addr}/pack.zip");
        let encoded_fast: String =
            url::form_urlencoded::byte_serialize(fast_url.as_bytes()).collect();
        let fast_req = parse_get(&format!("/proxy/pack?url={encoded_fast}")).await;
        let first = handle_pack_proxy_with_client_and_limiter(
            &fast_req,
            true,
            false,
            &client,
            &budget_limiter,
        )
        .await;
        assert_eq!(first.response.status, 200);
        assert_eq!(first.response.body.len(), 48);

        // While `first` holds 48 of 64 aggregate bytes, a second 48-byte pack must be rejected with 503.
        let second = handle_pack_proxy_with_client_and_limiter(
            &fast_req,
            true,
            false,
            &client,
            &budget_limiter,
        )
        .await;
        assert_eq!(second.response.status, 503);
        drop(first);
        assert_eq!(
            budget_limiter
                .inflight_bytes
                .used_bytes
                .load(Ordering::Acquire),
            0
        );

        fast_task.await.unwrap();
    }

    #[tokio::test]
    async fn serve_pack_proxy_streams_fixed_chunked_and_aborts_oversize_stream() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            for idx in 0..3 {
                let (mut sock, _) = listener.accept().await.unwrap();
                let mut scratch = [0u8; 1024];
                let _ = sock.read(&mut scratch).await;
                match idx {
                    0 => {
                        sock.write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Length: 10\r\n\r\n0123456789",
                        )
                        .await
                        .unwrap();
                    }
                    1 => {
                        sock.write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nabcd\r\n3\r\nefg\r\n0\r\n\r\n",
                        )
                        .await
                        .unwrap();
                    }
                    _ => {
                        sock.write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nTransfer-Encoding: chunked\r\n\r\n50\r\n",
                        )
                        .await
                        .unwrap();
                        let _ = sock.write_all(&[b'x'; 80]).await;
                        let _ = sock.write_all(b"\r\n50\r\n").await;
                        let _ = sock.write_all(&[b'y'; 80]).await;
                        let _ = sock.write_all(b"\r\n0\r\n\r\n").await;
                    }
                }
            }
        });

        let client = build_proxy_client(SsrfSafeDnsResolver::system(true, false));
        let limiter = ProxyLimiter::default();
        let url = format!("http://{addr}/pack.zip");
        let encoded: String = url::form_urlencoded::byte_serialize(url.as_bytes()).collect();
        let req = parse_get(&format!("/proxy/pack?url={encoded}")).await;

        let mut out_fixed = Vec::new();
        let ka_fixed = serve_pack_proxy_with_client(
            &req,
            &mut out_fixed,
            true,
            true,
            false,
            &client,
            &limiter,
            128,
        )
        .await
        .unwrap();
        assert!(ka_fixed);
        let fixed_str = String::from_utf8(out_fixed).unwrap();
        assert!(fixed_str.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(fixed_str.contains("Content-Length: 10\r\n"));
        assert!(fixed_str.ends_with("\r\n\r\n0123456789"));

        let mut out_chunked = Vec::new();
        let ka_chunked = serve_pack_proxy_with_client(
            &req,
            &mut out_chunked,
            true,
            true,
            false,
            &client,
            &limiter,
            128,
        )
        .await
        .unwrap();
        assert!(ka_chunked);
        let chunked_str = String::from_utf8(out_chunked).unwrap();
        assert!(chunked_str.contains("Transfer-Encoding: chunked\r\n"));
        assert!(chunked_str.ends_with("0\r\n\r\n"));

        // Oversize chunked stream aborts without terminal 0-chunk and closes keep-alive.
        let mut out_over = Vec::new();
        let ka_over = serve_pack_proxy_with_client(
            &req,
            &mut out_over,
            true,
            true,
            false,
            &client,
            &limiter,
            128,
        )
        .await
        .unwrap();
        assert!(!ka_over);
        let over_str = String::from_utf8(out_over).unwrap();
        assert!(!over_str.ends_with("0\r\n\r\n"));

        server_task.await.unwrap();
    }

    #[test]
    fn skin_allowlist_includes_minecraft_services_publickeys() {
        let url = Url::parse("https://api.minecraftservices.com/publickeys").unwrap();
        assert!(validate_hop_url(&url, HostPolicy::SkinAllowlist, false).is_ok());
    }
}
