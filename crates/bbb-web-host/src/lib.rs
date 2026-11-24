//! Same-origin HTTP asset host, COOP/COEP cross-origin isolation gateway,
//! binary WebSocket <-> Minecraft TCP bridge, CORS skin/pack proxy, and reverse
//! MCP/control tunnel for the BBB WASM + WebGPU client.

pub mod assets_http;
pub mod http;
pub mod mcp_bridge;
pub mod proxy;
pub mod ws_bridge;
pub mod ws_codec;

use std::{
    collections::HashSet,
    future::Future,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use tokio::net::{TcpListener, TcpStream};

use crate::{
    assets_http::{AssetBuildOptions, PreparedWebAssets},
    http::{HttpConnReader, HttpMethod, HttpRequest, HttpResponse},
    mcp_bridge::BrowserControlBridge,
};

const INITIAL_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const KEEP_ALIVE_IDLE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct WebHostConfig {
    pub bind: SocketAddr,
    pub default_server: String,
    pub allow_servers: Vec<String>,
    pub vanilla_root: PathBuf,
    pub wasm_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub default_username: String,
    pub default_locale: Option<String>,
    pub default_uuid: Option<String>,
    pub default_render_distance: Option<u32>,
    pub server_resource_pack_policy: Option<String>,
    pub allow_pack_loopback: bool,
    pub allow_pack_private: bool,
    pub control_enabled: bool,
}

struct SharedGatewayState {
    default_server: String,
    allowed_servers: HashSet<String>,
    allow_pack_loopback: bool,
    allow_pack_private: bool,
    control_enabled: bool,
    assets: PreparedWebAssets,
    control_bridge: Arc<BrowserControlBridge>,
    proxy_limiter: Arc<proxy::ProxyLimiter>,
}

pub struct WebHostServer {
    listener: TcpListener,
    local_addr: SocketAddr,
    state: Arc<SharedGatewayState>,
}

impl WebHostServer {
    pub async fn bind(config: WebHostConfig) -> Result<Self> {
        proxy::ensure_rustls_provider();
        let assets = PreparedWebAssets::build_or_load(AssetBuildOptions {
            vanilla_root: config.vanilla_root,
            wasm_dir: config.wasm_dir,
            cache_dir: config.cache_dir,
            default_server: config.default_server.clone(),
            default_username: config.default_username,
            default_locale: config.default_locale,
            default_uuid: config.default_uuid,
            default_render_distance: config.default_render_distance,
            server_resource_pack_policy: config.server_resource_pack_policy,
            control_enabled: config.control_enabled,
        })?;

        let listener = TcpListener::bind(config.bind)
            .await
            .with_context(|| format!("bind bbb-web-host on {}", config.bind))?;
        let local_addr = listener.local_addr().context("query local bind address")?;

        let mut allowed_servers = HashSet::new();
        allowed_servers.insert(config.default_server.trim().to_ascii_lowercase());
        for extra in config.allow_servers {
            let trimmed = extra.trim().to_ascii_lowercase();
            if !trimmed.is_empty() {
                allowed_servers.insert(trimmed);
            }
        }

        let state = Arc::new(SharedGatewayState {
            default_server: config.default_server,
            allowed_servers,
            allow_pack_loopback: config.allow_pack_loopback,
            allow_pack_private: config.allow_pack_private,
            control_enabled: config.control_enabled,
            assets,
            control_bridge: Arc::new(BrowserControlBridge::new()),
            proxy_limiter: Arc::new(proxy::ProxyLimiter::default()),
        });

        Ok(Self {
            listener,
            local_addr,
            state,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn assets(&self) -> &PreparedWebAssets {
        &self.state.assets
    }

    pub fn control_bridge(&self) -> Arc<BrowserControlBridge> {
        Arc::clone(&self.state.control_bridge)
    }

    pub async fn serve(self, shutdown: impl Future<Output = ()>) -> Result<()> {
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                _ = &mut shutdown => {
                    break;
                }
                accepted = self.listener.accept() => {
                    let (stream, _) = accepted.context("accept HTTP/WS client")?;
                    let state = Arc::clone(&self.state);
                    tokio::spawn(async move {
                        if let Err(err) = handle_connection(stream, state).await {
                            tracing::debug!(?err, "web-host connection closed with error");
                        }
                    });
                }
            }
        }
        Ok(())
    }
}

async fn handle_connection(stream: TcpStream, state: Arc<SharedGatewayState>) -> Result<()> {
    let _ = stream.set_nodelay(true);
    let (read_half, mut write_half) = tokio::io::split(stream);
    let mut http_reader = HttpConnReader::new(read_half);
    let mut first_request = true;

    loop {
        let timeout = if first_request {
            first_request = false;
            INITIAL_REQUEST_TIMEOUT
        } else {
            KEEP_ALIVE_IDLE_TIMEOUT
        };
        let next_req =
            match tokio::time::timeout(timeout, http_reader.next_request(&mut write_half)).await {
                Ok(res) => res?,
                Err(_) => break,
            };
        let Some(req) = next_req else {
            break;
        };

        if req.path == "/ws" {
            if !is_allowed_ws_origin(&req) {
                let resp =
                    HttpResponse::text(403, "Forbidden", "Cross-origin /ws connection rejected");
                resp.write_to_with_keep_alive(&mut write_half, false, false)
                    .await?;
                break;
            }
            let (read_half, leftover) = http_reader.into_inner_and_leftover();
            let stream = read_half.unsplit(write_half);
            return ws_bridge::handle_ws_relay(
                req,
                stream,
                leftover,
                &state.default_server,
                &state.allowed_servers,
            )
            .await;
        }

        if req.path == "/control-ws" {
            if !state.control_enabled {
                let resp = HttpResponse::text(
                    404,
                    "Not Found",
                    "Browser control tunnel is disabled on this bbb-web-host instance",
                );
                resp.write_to_with_keep_alive(&mut write_half, false, false)
                    .await?;
                break;
            }
            if !is_allowed_ws_origin(&req) {
                let resp = HttpResponse::text(
                    403,
                    "Forbidden",
                    "Cross-origin /control-ws connection rejected",
                );
                resp.write_to_with_keep_alive(&mut write_half, false, false)
                    .await?;
                break;
            }
            let (read_half, leftover) = http_reader.into_inner_and_leftover();
            let stream = read_half.unsplit(write_half);
            return Arc::clone(&state.control_bridge)
                .accept_browser_tunnel(req, stream, leftover)
                .await;
        }

        let head_only = req.method == HttpMethod::Head;
        if req.method != HttpMethod::Get && !head_only {
            let resp =
                HttpResponse::text(405, "Method Not Allowed", "Only GET and HEAD are supported");
            resp.write_to_with_keep_alive(&mut write_half, false, false)
                .await?;
            break;
        }

        let keep_alive = req.keep_alive;
        match req.path.as_str() {
            "/proxy/skin" | "/proxy/pack" if head_only => {
                let resp = HttpResponse::text(
                    405,
                    "Method Not Allowed",
                    "Only GET is supported for proxy endpoints",
                )
                .with_header("Allow", "GET");
                resp.write_to_with_keep_alive(&mut write_half, head_only, keep_alive)
                    .await?;
                if !keep_alive {
                    break;
                }
            }
            "/proxy/skin" => {
                let proxy_resp =
                    proxy::handle_skin_proxy_with_limiter(&req, &state.proxy_limiter).await;
                proxy_resp
                    .response
                    .write_to_with_keep_alive(&mut write_half, false, keep_alive)
                    .await?;
                drop(proxy_resp);
                if !keep_alive {
                    break;
                }
            }
            "/proxy/pack" => {
                let keep_alive_ok = proxy::serve_pack_proxy(
                    &req,
                    &mut write_half,
                    keep_alive,
                    state.allow_pack_loopback,
                    state.allow_pack_private,
                    &state.proxy_limiter,
                )
                .await?;
                if !keep_alive_ok {
                    break;
                }
            }
            _ => {
                let resp = state.assets.serve_async(&req).await;
                resp.write_to_with_keep_alive(&mut write_half, head_only, keep_alive)
                    .await?;
                if !keep_alive {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn is_loopback_host(host: &url::Host<&str>) -> bool {
    match host {
        url::Host::Domain(domain) => domain.eq_ignore_ascii_case("localhost"),
        url::Host::Ipv4(v4) => *v4 == Ipv4Addr::LOCALHOST,
        url::Host::Ipv6(v6) => *v6 == Ipv6Addr::LOCALHOST,
    }
}

fn parse_host_authority(scheme: &str, raw_host: &str) -> Option<(url::Host<String>, u16)> {
    let trimmed = raw_host.trim();
    if trimmed.is_empty()
        || trimmed
            .bytes()
            .any(|b| matches!(b, b'/' | b'\\' | b'@' | b'?' | b'#' | b' ' | b'\t'))
    {
        return None;
    }
    let parsed = url::Url::parse(&format!("{scheme}://{trimmed}")).ok()?;
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.path() != "/"
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return None;
    }
    let host = parsed.host()?.to_owned();
    let port = parsed.port_or_known_default()?;
    Some((host, port))
}

fn hosts_match(a: &url::Host<&str>, b: &url::Host<&str>) -> bool {
    match (a, b) {
        (url::Host::Domain(lhs), url::Host::Domain(rhs)) => lhs.eq_ignore_ascii_case(rhs),
        (url::Host::Ipv4(lhs), url::Host::Ipv4(rhs)) => lhs == rhs,
        (url::Host::Ipv6(lhs), url::Host::Ipv6(rhs)) => lhs == rhs,
        _ => false,
    }
}

pub(crate) fn is_allowed_ws_origin(req: &HttpRequest) -> bool {
    let mut origins = req.headers.values("origin");
    let Some(origin) = origins.next() else {
        // Non-browser local callers may omit Origin; browsers always attach it on WS upgrades.
        return true;
    };
    if origins.next().is_some() {
        return false;
    }
    let Ok(parsed) = url::Url::parse(origin.trim()) else {
        return false;
    };
    if !matches!(parsed.scheme(), "http" | "https")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.path() != "/"
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return false;
    }
    let (Some(origin_host), Some(origin_port)) = (parsed.host(), parsed.port_or_known_default())
    else {
        return false;
    };
    let mut hosts = req.headers.values("host");
    let Some(host_header) = hosts.next() else {
        return false;
    };
    if hosts.next().is_some() {
        return false;
    }
    let Some((req_host, req_port)) = parse_host_authority(parsed.scheme(), host_header) else {
        return false;
    };
    if origin_port != req_port {
        return false;
    }
    let req_host_ref = match &req_host {
        url::Host::Domain(d) => url::Host::Domain(d.as_str()),
        url::Host::Ipv4(v4) => url::Host::Ipv4(*v4),
        url::Host::Ipv6(v6) => url::Host::Ipv6(*v6),
    };
    if is_loopback_host(&origin_host) && is_loopback_host(&req_host_ref) {
        return true;
    }
    hosts_match(&origin_host, &req_host_ref)
}

pub(crate) fn is_allowed_proxy_origin(req: &HttpRequest) -> bool {
    if !is_allowed_ws_origin(req) {
        return false;
    }
    // Browser same-origin GETs normally omit Origin and attach Sec-Fetch-Site.
    for fetch_site in req.headers.values("sec-fetch-site") {
        let trimmed = fetch_site.trim();
        if !trimmed.eq_ignore_ascii_case("same-origin") && !trimmed.eq_ignore_ascii_case("none") {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncWriteExt;

    use super::*;
    use crate::http::read_http_request;

    async fn parse_req(raw: &str) -> HttpRequest {
        let (mut client, mut server) = tokio::io::duplex(4096);
        client.write_all(raw.as_bytes()).await.unwrap();
        drop(client);
        read_http_request(&mut server).await.unwrap().unwrap().0
    }

    #[tokio::test]
    async fn ws_origin_requires_matching_port_and_loopback_on_both_sides() {
        // Matching-port loopback aliases are allowed.
        for (host, origin) in [
            ("127.0.0.1:8080", "http://127.0.0.1:8080"),
            ("127.0.0.1:8080", "http://localhost:8080"),
            ("127.0.0.1:8080", "http://[::1]:8080"),
            ("[::1]:8080", "http://localhost:8080"),
            ("localhost", "http://127.0.0.1:80"),
            ("127.0.0.1:443", "https://localhost"),
            ("example.com:8443", "https://EXAMPLE.COM:8443"),
            ("example.com", "https://example.com:443"),
        ] {
            let req = parse_req(&format!(
                "GET /ws HTTP/1.1\r\nHost: {host}\r\nOrigin: {origin}\r\n\r\n"
            ))
            .await;
            assert!(
                is_allowed_ws_origin(&req),
                "expected allowed for Host={host} Origin={origin}"
            );
        }

        // Cross-port loopback, remote-host + loopback-origin, scheme-default port mismatches,
        // credentials, and malformed values must be rejected.
        for (host, origin) in [
            ("127.0.0.1:8080", "http://localhost:9090"),
            ("127.0.0.1:8080", "http://127.0.0.1"),
            ("127.0.0.1:8080", "http://[::1]:8081"),
            ("remote.example:8080", "http://localhost:8080"),
            ("remote.example:8080", "http://127.0.0.1:8080"),
            ("remote.example", "http://localhost"),
            ("127.0.0.1", "https://localhost:80"),
            ("127.0.0.1:80", "https://localhost"),
            ("127.0.0.1:443", "http://localhost"),
            ("127.0.0.1:8080", "http://127.0.0.2:8080"),
            ("127.0.0.1:8080", "http://user@127.0.0.1:8080"),
            ("evil@127.0.0.1:8080", "http://127.0.0.1:8080"),
            ("127.0.0.1:8080", "http://127.0.0.1:8080/extra"),
            ("127.0.0.1:8080", "null"),
        ] {
            let req = parse_req(&format!(
                "GET /ws HTTP/1.1\r\nHost: {host}\r\nOrigin: {origin}\r\n\r\n"
            ))
            .await;
            assert!(
                !is_allowed_ws_origin(&req),
                "expected rejected for Host={host} Origin={origin}"
            );
        }

        // Duplicate Origin headers must be rejected.
        let dup_origin = parse_req(
            "GET /ws HTTP/1.1\r\nHost: 127.0.0.1:8080\r\nOrigin: http://127.0.0.1:8080\r\nOrigin: https://evil.example\r\n\r\n",
        )
        .await;
        assert!(!is_allowed_ws_origin(&dup_origin));
    }

    #[tokio::test]
    async fn proxy_origin_enforces_origin_and_sec_fetch_site() {
        let same_origin_get = parse_req(
            "GET /proxy/skin?url=x HTTP/1.1\r\nHost: 127.0.0.1:8080\r\nSec-Fetch-Site: same-origin\r\n\r\n",
        )
        .await;
        assert!(is_allowed_proxy_origin(&same_origin_get));

        let nav_none = parse_req(
            "GET /proxy/skin?url=x HTTP/1.1\r\nHost: 127.0.0.1:8080\r\nSec-Fetch-Site: none\r\n\r\n",
        )
        .await;
        assert!(is_allowed_proxy_origin(&nav_none));

        let absent_both =
            parse_req("GET /proxy/skin?url=x HTTP/1.1\r\nHost: 127.0.0.1:8080\r\n\r\n").await;
        assert!(is_allowed_proxy_origin(&absent_both));

        let cross_site = parse_req(
            "GET /proxy/pack?url=x HTTP/1.1\r\nHost: 127.0.0.1:8080\r\nSec-Fetch-Site: cross-site\r\n\r\n",
        )
        .await;
        assert!(!is_allowed_proxy_origin(&cross_site));

        let same_site = parse_req(
            "GET /proxy/pack?url=x HTTP/1.1\r\nHost: 127.0.0.1:8080\r\nSec-Fetch-Site: same-site\r\n\r\n",
        )
        .await;
        assert!(!is_allowed_proxy_origin(&same_site));

        let bad_origin = parse_req(
            "GET /proxy/skin?url=x HTTP/1.1\r\nHost: 127.0.0.1:8080\r\nOrigin: http://localhost:9999\r\nSec-Fetch-Site: same-origin\r\n\r\n",
        )
        .await;
        assert!(!is_allowed_proxy_origin(&bad_origin));
    }
}
