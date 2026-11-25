use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use bbb_mcp::control::{check_bind, serve_remote, serve_remote_control, RemoteControlEndpoint};
use bbb_web_host::{assets_http::has_vanilla_bundle_input, WebHostConfig, WebHostServer};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "bbb-web-host",
    about = "BBB Web client HTTP/COOP-COEP asset gateway, WebSocket->TCP bridge, and reverse MCP host"
)]
struct Cli {
    /// HTTP & WebSocket listen address for the browser client.
    #[arg(long, default_value = "127.0.0.1:8080")]
    bind: SocketAddr,

    /// Default upstream Minecraft Java 26.1 server (`host:port`).
    #[arg(long, default_value = "127.0.0.1:25565")]
    server: String,

    /// Additional upstream `host:port` servers permitted via `?server=...`.
    #[arg(long = "allow-server")]
    allow_servers: Vec<String>,

    /// Path to Minecraft Java 26.1 assets (prepared `sources/26.1` + `downloads/26.1`, `downloads/26.1` client JAR, or an official `.minecraft` directory).
    #[arg(long)]
    vanilla_root: Option<PathBuf>,

    /// Directory containing built WASM assets (defaults to `$BBB_WEB_OUT_DIR` or `<target-root>/bbb-web-dist`).
    #[arg(long)]
    wasm_dir: Option<PathBuf>,

    /// Disk cache directory for assembled `core-26.1.zip` (defaults to `<target-root>/bbb-web-cache`).
    #[arg(long)]
    cache_dir: Option<PathBuf>,

    /// Override the browser profile's offline player name when `?username=` is omitted.
    #[arg(long, default_value = "")]
    username: String,

    /// Default client language/locale when `?lang=` / `?locale=` is omitted (e.g. `zh_cn`, `en_us`).
    #[arg(long)]
    locale: Option<String>,

    /// Optional default player UUID when `?uuid=` is omitted.
    #[arg(long)]
    uuid: Option<String>,

    /// Optional default render distance when `?render_distance=` is omitted.
    #[arg(long)]
    render_distance: Option<u32>,

    /// Default server resource-pack policy (`prompt`, `accept` or `decline`) when `?server_resource_pack_policy=` is omitted.
    #[arg(long)]
    server_resource_pack_policy: Option<String>,

    /// Allow `/proxy/pack` to fetch server resource packs from loopback addresses (local dev/tests).
    #[arg(long)]
    allow_pack_loopback: bool,

    /// Allow `/proxy/pack` to fetch server resource packs from private LAN ranges (RFC1918 and IPv6 ULA).
    #[arg(long)]
    allow_pack_private: bool,

    /// Optional loopback bind address exposing `bbb-mcp` Streamable HTTP against the active browser tab.
    #[arg(long)]
    mcp_bind: Option<SocketAddr>,

    /// Optional loopback bind address exposing the line-delimited JSON control API against the active browser tab.
    #[arg(long)]
    control: Option<SocketAddr>,
}

fn default_vanilla_root() -> PathBuf {
    if let Some(env_dir) = std::env::var_os("BBB_VANILLA_ROOT").filter(|v| !v.is_empty()) {
        return PathBuf::from(env_dir);
    }
    let mut candidates = Vec::new();
    if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME").filter(|v| !v.is_empty()) {
        candidates.push(PathBuf::from(xdg).join("bbb").join("vanilla"));
    }
    #[cfg(target_os = "windows")]
    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA").filter(|v| !v.is_empty()) {
        candidates.push(PathBuf::from(local_app_data).join("bbb").join("vanilla"));
    }
    #[cfg(target_os = "windows")]
    if let Some(home) = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .filter(|v| !v.is_empty())
    {
        candidates.push(
            PathBuf::from(home)
                .join("AppData")
                .join("Local")
                .join("bbb")
                .join("vanilla"),
        );
    }
    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME").filter(|v| !v.is_empty()) {
        candidates.push(
            PathBuf::from(home)
                .join("Library")
                .join("Caches")
                .join("bbb")
                .join("vanilla"),
        );
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    if let Some(home) = std::env::var_os("HOME").filter(|v| !v.is_empty()) {
        candidates.push(
            PathBuf::from(home)
                .join(".cache")
                .join("bbb")
                .join("vanilla"),
        );
    }
    let default_candidate = candidates
        .first()
        .cloned()
        .unwrap_or_else(|| std::env::temp_dir().join("bbb-vanilla"));
    candidates
        .into_iter()
        .find(|candidate| has_vanilla_bundle_input(candidate))
        .unwrap_or(default_candidate)
}

fn default_target_root_from(env_override: Option<std::ffi::OsString>) -> PathBuf {
    env_override
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            if cfg!(unix) {
                PathBuf::from("/tmp")
            } else {
                std::env::temp_dir()
            }
        })
}

fn default_target_root() -> PathBuf {
    default_target_root_from(std::env::var_os("BBB_TARGET_ROOT"))
}

fn default_wasm_dir() -> PathBuf {
    if let Some(env_dir) = std::env::var_os("BBB_WEB_OUT_DIR").filter(|v| !v.is_empty()) {
        return PathBuf::from(env_dir);
    }
    default_target_root().join("bbb-web-dist")
}

fn default_cache_dir() -> PathBuf {
    default_target_root().join("bbb-web-cache")
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    if let Some(mcp_addr) = cli.mcp_bind {
        check_bind(mcp_addr).with_context(|| format!("invalid --mcp-bind {mcp_addr}"))?;
    }
    if let Some(control_addr) = cli.control {
        check_bind(control_addr).with_context(|| format!("invalid --control {control_addr}"))?;
    }

    let vanilla_root = cli.vanilla_root.unwrap_or_else(default_vanilla_root);
    let wasm_dir = cli.wasm_dir.unwrap_or_else(default_wasm_dir);
    let cache_dir = cli.cache_dir.unwrap_or_else(default_cache_dir);
    let control_enabled = cli.mcp_bind.is_some() || cli.control.is_some();

    if !wasm_dir.join("pkg").join("bbb_client.js").is_file()
        && !wasm_dir.join("bbb_client.js").is_file()
    {
        tracing::warn!(
            wasm_dir = %wasm_dir.display(),
            "WASM bundle `bbb_client.js` not found under --wasm-dir; run `cargo build-web` before opening the browser"
        );
    }

    let server = WebHostServer::bind(WebHostConfig {
        bind: cli.bind,
        default_server: cli.server.clone(),
        allow_servers: cli.allow_servers,
        vanilla_root,
        wasm_dir,
        cache_dir,
        default_username: cli.username,
        default_locale: cli.locale,
        default_uuid: cli.uuid,
        default_render_distance: cli.render_distance,
        server_resource_pack_policy: cli.server_resource_pack_policy,
        allow_pack_loopback: cli.allow_pack_loopback,
        allow_pack_private: cli.allow_pack_private,
        control_enabled,
    })
    .await?;

    let mut shutdown = ShutdownSignals::install().context("install shutdown signal handlers")?;

    let addr = server.local_addr();
    let bridge: Arc<dyn RemoteControlEndpoint> = server.control_bridge();

    if let Some(control_addr) = cli.control {
        let remote = Arc::clone(&bridge);
        tokio::spawn(async move {
            if let Err(err) = serve_remote_control(control_addr, remote).await {
                tracing::error!(?err, %control_addr, "remote control API terminated");
            }
        });
    }

    if let Some(mcp_addr) = cli.mcp_bind {
        let remote = Arc::clone(&bridge);
        tokio::spawn(async move {
            if let Err(err) = serve_remote(mcp_addr, remote).await {
                tracing::error!(?err, %mcp_addr, "remote MCP API terminated");
            }
        });
    }

    tracing::info!(
        %addr,
        upstream = %cli.server,
        etag = %server.assets().core_bundle_etag(),
        bundle_bytes = server.assets().core_bundle_bytes().len(),
        "bbb-web-host listening (open http://{addr}/ in a WebGPU browser)"
    );

    server
        .serve(async move {
            let signal = shutdown.recv().await;
            tracing::info!(signal, "bbb-web-host shutting down");
        })
        .await
}

#[cfg(unix)]
struct ShutdownSignals {
    interrupt: tokio::signal::unix::Signal,
    terminate: tokio::signal::unix::Signal,
}

#[cfg(unix)]
impl ShutdownSignals {
    fn install() -> std::io::Result<Self> {
        use tokio::signal::unix::{signal, SignalKind};

        Ok(Self {
            interrupt: signal(SignalKind::interrupt())?,
            terminate: signal(SignalKind::terminate())?,
        })
    }

    async fn recv(&mut self) -> &'static str {
        tokio::select! {
            _ = self.interrupt.recv() => "SIGINT",
            _ = self.terminate.recv() => "SIGTERM",
        }
    }
}

#[cfg(not(unix))]
struct ShutdownSignals;

#[cfg(not(unix))]
impl ShutdownSignals {
    fn install() -> std::io::Result<Self> {
        Ok(Self)
    }

    async fn recv(&mut self) -> &'static str {
        let _ = tokio::signal::ctrl_c().await;
        "SIGINT"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn default_target_root_honors_override_and_platform_default() {
        assert_eq!(
            default_target_root_from(Some(OsString::from("/custom/root"))),
            PathBuf::from("/custom/root")
        );
        let default_root = default_target_root_from(Some(OsString::new()));
        if cfg!(unix) {
            assert_eq!(default_root, PathBuf::from("/tmp"));
        } else {
            assert_eq!(default_root, std::env::temp_dir());
        }
    }
}
