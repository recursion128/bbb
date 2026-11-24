//! The MCP endpoint of the client, sharing `bbb_mcp` with the dedicated
//! server so an agent manages both the same way.
//!
//! The host projects the control surface: every read method of the control
//! dispatch is a tool of the same name, the readouts that take no argument
//! are also resources under `bbb://client/`, and every write request is a tool
//! whose schema mirrors the control parameters and serde request types.
//!
//! Authentication is loopback-only, like the line-based control server: the
//! default [`bbb_mcp::AuthConfig`] grants a credential-less loopback peer the
//! `manage` scope, and [`check_bind`] refuses any address off the loopback
//! interface. There is no token, OAuth or TLS surface here; an agent that is
//! not on the machine reaches the client through a gateway in front of the
//! loopback bind.

use std::{net::SocketAddr, sync::Arc};

use anyhow::Context;
pub use bbb_mcp::control::{check_bind, CHANGE_WATCH_INTERVAL};
use tokio::net::TcpListener;

use crate::control::ControlState;

#[cfg(test)]
mod catalog;
mod host;

#[cfg(test)]
use bbb_mcp::control::{catalog, resources, CLIENT_OPERATION_URI_PREFIX as OPERATION_URI_PREFIX};
pub use host::ClientHost;

pub type McpServerConfig = bbb_mcp::McpServerConfig<ClientHost>;
pub type McpServer = bbb_mcp::McpServer<ClientHost>;

/// Serves the client's MCP endpoint until the process ends.
pub async fn serve(addr: SocketAddr, control: ControlState) -> anyhow::Result<()> {
    check_bind(addr).with_context(|| format!("bind MCP API on {addr}"))?;
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind MCP API on {addr}"))?;
    let host = ClientHost::new(control);
    let watch = host.spawn_change_watch(CHANGE_WATCH_INTERVAL);
    let server = McpServer::new(McpServerConfig::default(), host);
    tracing::info!(%addr, path = %server.config().endpoint_path, "native MCP API listening");
    Arc::clone(&server)
        .serve(listener, std::future::pending())
        .await;
    watch.abort();
    Ok(())
}

#[cfg(test)]
mod tests;
