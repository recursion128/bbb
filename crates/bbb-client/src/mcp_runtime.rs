//! The client's MCP endpoint, started next to the line-based control API and
//! served over the same [`ControlState`].

use std::net::SocketAddr;

use crate::control::ControlState;
use anyhow::Result;
use tokio::runtime::Runtime;

/// Serves the MCP endpoint on `addr` until the process ends. The bind is
/// refused off the loopback interface, like the control API's authority.
pub async fn serve(addr: SocketAddr, control: ControlState) -> Result<()> {
    crate::mcp::serve(addr, control).await
}

/// Starts the endpoint when `--mcp-bind` was given, mirroring
/// `startup::start_control_api`.
pub(crate) fn start_mcp_api(runtime: &Runtime, addr: Option<SocketAddr>, control: &ControlState) {
    let Some(addr) = addr else {
        return;
    };

    let control = control.clone();
    runtime.spawn(async move {
        if let Err(err) = serve(addr, control).await {
            tracing::error!(?err, "MCP API stopped");
        }
    });
}
