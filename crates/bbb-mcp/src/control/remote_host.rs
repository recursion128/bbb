//! Renderer-free MCP and line-delimited JSON control servers backed by a
//! [`RemoteControlEndpoint`], shared by `bbb-client` and `bbb-web-host`.

use std::{
    io,
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::{json, Value};
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::{broadcast, Semaphore},
    task::JoinHandle,
    time::timeout,
};

use crate::{
    control::{
        catalog::{catalog, resources, tool_by_name, tool_by_resource_uri, ClientTool},
        ControlRequest, ControlResponse, ControlScreenshot, RemoteControlEndpoint,
    },
    host::{
        CallRefusal, Caller, HostIdentity, McpHost, PendingCall, ResourceChange,
        ResourceDescriptor, Scope, ToolDescriptor,
    },
    http::{read_line, trim_line, LineOutcome},
    jsonrpc::{JsonRpcError, INTERNAL_ERROR},
    server::{
        BindRefusal, McpServer, McpServerConfig, RemoteAuthority, CLIENT_OPERATION_URI_PREFIX,
    },
};

/// Default snapshot-generation poll interval for client resource subscriptions.
pub const CHANGE_WATCH_INTERVAL: Duration = Duration::from_millis(250);

/// Maximum byte length of a single line-delimited JSON control message
/// (bounded to accommodate base64 screenshot payloads without unbounded buffering).
pub const MAX_CONTROL_LINE_BYTES: usize = 16 * 1024 * 1024;

const CHANGE_CHANNEL_CAPACITY: usize = 256;
const MAX_RETAINED_CONTROL_LINE_CAPACITY: usize = 256 * 1024;
const MAX_REMOTE_CONTROL_CONNECTIONS: usize = 16;
const REMOTE_CONTROL_IO_TIMEOUT: Duration = Duration::from_secs(30);
const ACCEPT_ERROR_BACKOFF: Duration = Duration::from_millis(50);

struct AbortOnDrop(JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Startup error when binding a remote client MCP or control listener.
#[derive(Debug, Error)]
pub enum RemoteServeError {
    #[error(transparent)]
    Bind(#[from] BindRefusal),
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// Completed client control call packaged for [`McpHost`] encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientMcpOutcome {
    pub operation_id: u64,
    pub action: &'static str,
    pub response: ControlResponse,
    pub image: Option<ControlScreenshot>,
}

/// Shared MCP identity advertised by both local `bbb-client` and remote `bbb-web-host` endpoints.
pub fn client_host_identity() -> HostIdentity {
    HostIdentity {
        name: "bbb-client",
        title: "BBB client control",
        version: env!("CARGO_PKG_VERSION"),
        instructions: Some(
            "Control for a running bbb client. Before in-world movement, aiming, mining, or placing, \
             call input.focus({focused: true, capture_cursor: true}) and verify app.status shows \
             input_focused=true and cursor_captured=true (press escape if a screen is blocking). \
             Perceive state with world.client_local_player, world.crosshair_target, \
             world.probe_blocks, world.probe_surface, world.nearby_entities, \
             world.client_inventory (includes item_name and menu_type), and renderer.screenshot. \
             Aim accurately with input.look_at or input.look, then verify world.crosshair_target. \
             In real-time sessions prefer hold_ticks on input.key/input.mouse_button (auto-releases \
             after N ticks at 20 TPS) and sim.wait_ticks; in --lockstep sessions advance with \
             sim.step. Operate open inventories/crafting/furnaces/merchants with \
             net.container_click_slot, net.place_recipe, net.container_button_click, and \
             net.select_trade.",
        ),
        operation_uri_prefix: CLIENT_OPERATION_URI_PREFIX,
    }
}

/// Encodes a [`ClientMcpOutcome`] into the MCP `tools/call` result shape.
pub fn encode_client_tool_result(outcome: &ClientMcpOutcome) -> Value {
    let meta = json!({ "bbb/operationId": outcome.operation_id });
    match (&outcome.response.result, &outcome.response.error) {
        (Some(result), _) if outcome.response.ok => {
            let mut content = vec![json!({
                "type": "text",
                "text": serde_json::to_string_pretty(result).unwrap_or_default(),
            })];
            if let Some(image) = &outcome.image {
                content.push(json!({
                    "type": "image",
                    "mimeType": image.mime_type,
                    "data": BASE64.encode(&image.bytes),
                }));
            }
            json!({
                "content": content,
                "structuredContent": result,
                "isError": false,
                "_meta": meta,
            })
        }
        (_, error) => json!({
            "content": [{
                "type": "text",
                "text": format!(
                    "{} refused: {}",
                    outcome.action,
                    error.as_deref().unwrap_or("no result")
                ),
            }],
            "isError": true,
            "_meta": meta,
        }),
    }
}

/// Encodes a [`ClientMcpOutcome`] into the `bbb://client/operation/<id>` audit document.
pub fn encode_client_outcome_record(outcome: &ClientMcpOutcome) -> Value {
    json!({
        "operationId": outcome.operation_id,
        "action": outcome.action,
        "ok": outcome.response.ok,
        "result": outcome.response.result,
        "error": outcome.response.error,
    })
}

/// Encodes a [`ClientMcpOutcome`] into the MCP `resources/read` JSON payload.
pub fn encode_client_resource_contents(outcome: &ClientMcpOutcome) -> Result<Value, JsonRpcError> {
    match (&outcome.response.result, &outcome.response.error) {
        (Some(result), _) if outcome.response.ok => Ok(result.clone()),
        (_, error) => Err(JsonRpcError::new(
            INTERNAL_ERROR,
            error.clone().unwrap_or_else(|| "no result".to_string()),
        )),
    }
}

/// Renderer-free [`McpHost`] driving a [`RemoteControlEndpoint`].
#[derive(Clone)]
pub struct RemoteClientHost {
    remote: Arc<dyn RemoteControlEndpoint>,
    next_operation: Arc<AtomicU64>,
    changes: broadcast::Sender<ResourceChange>,
}

impl RemoteClientHost {
    pub fn new(remote: Arc<dyn RemoteControlEndpoint>) -> Self {
        Self {
            remote,
            next_operation: Arc::new(AtomicU64::new(1)),
            changes: broadcast::channel(CHANGE_CHANNEL_CAPACITY).0,
        }
    }

    pub fn publish_changes(&self) {
        for resource in resources() {
            let _ = self.changes.send(ResourceChange { uri: resource.uri });
        }
    }

    pub fn spawn_change_watch(&self, interval: Duration) -> JoinHandle<()> {
        let host = self.clone();
        let mut seen = host.remote.generation();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                let generation = host.remote.generation();
                if generation != seen {
                    seen = generation;
                    host.publish_changes();
                }
            }
        })
    }

    fn answer(&self, tool: &'static ClientTool, params: Value) -> PendingCall<ClientMcpOutcome> {
        let operation_id = self.next_operation.fetch_add(1, Ordering::Relaxed);
        let request = ControlRequest::new(tool.name, params);
        let remote = Arc::clone(&self.remote);
        let confirmed = !tool.read_only;
        let (sender, answer) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let remote_answer = remote.call(request, confirmed).await;
            let _ = sender.send(ClientMcpOutcome {
                operation_id,
                action: tool.name,
                response: remote_answer.response,
                image: remote_answer.image,
            });
        });
        PendingCall {
            operation_id,
            action: tool.name,
            answer,
        }
    }
}

impl std::fmt::Debug for RemoteClientHost {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RemoteClientHost")
            .finish_non_exhaustive()
    }
}

impl McpHost for RemoteClientHost {
    type Scopes = Scope;
    type Outcome = ClientMcpOutcome;

    fn identity() -> HostIdentity {
        client_host_identity()
    }

    fn now(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
            .unwrap_or(0)
    }

    fn tools(&self) -> Vec<ToolDescriptor> {
        catalog().iter().map(ClientTool::descriptor).collect()
    }

    fn resources(&self) -> Vec<ResourceDescriptor> {
        resources()
    }

    fn resource(&self, uri: &str) -> Option<ResourceDescriptor> {
        tool_by_resource_uri(uri).and_then(ClientTool::resource)
    }

    fn call_tool(
        &self,
        caller: &Caller<Scope>,
        name: &str,
        arguments: &Value,
    ) -> Result<PendingCall<ClientMcpOutcome>, CallRefusal> {
        let tool = tool_by_name(name).ok_or(CallRefusal::Unknown)?;
        if !tool.read_only && caller.scope < Scope::Manage {
            let operation_id = self.next_operation.fetch_add(1, Ordering::Relaxed);
            return Ok(PendingCall::answered(
                operation_id,
                tool.name,
                ClientMcpOutcome {
                    operation_id,
                    action: tool.name,
                    response: ControlResponse::err(format!(
                        "scope `{}` cannot call `{}`; requires `manage`",
                        caller.scope.name(),
                        tool.name
                    )),
                    image: None,
                },
            ));
        }
        Ok(self.answer(tool, arguments.clone()))
    }

    fn read_resource(
        &self,
        _caller: &Caller<Scope>,
        uri: &str,
    ) -> Result<PendingCall<ClientMcpOutcome>, CallRefusal> {
        let tool = tool_by_resource_uri(uri).ok_or(CallRefusal::Unknown)?;
        Ok(self.answer(tool, Value::Null))
    }

    fn tool_result(&self, outcome: &ClientMcpOutcome) -> Value {
        encode_client_tool_result(outcome)
    }

    fn outcome_record(&self, outcome: &ClientMcpOutcome) -> Value {
        encode_client_outcome_record(outcome)
    }

    fn resource_contents(&self, outcome: &ClientMcpOutcome) -> Result<Value, JsonRpcError> {
        encode_client_resource_contents(outcome)
    }

    fn changes(&self) -> broadcast::Receiver<ResourceChange> {
        self.changes.subscribe()
    }
}

/// Validates that a client MCP endpoint bind address stays on the loopback interface.
pub fn check_bind(addr: SocketAddr) -> Result<(), BindRefusal> {
    crate::check_bind(
        addr,
        &RemoteAuthority::LoopbackOnly,
        &McpServerConfig::<RemoteClientHost>::default().auth,
    )
}

/// Binds and serves the client MCP Streamable HTTP endpoint over `remote` until cancelled.
pub async fn serve_remote(
    addr: SocketAddr,
    remote: Arc<dyn RemoteControlEndpoint>,
) -> Result<(), RemoteServeError> {
    check_bind(addr)?;
    let listener = TcpListener::bind(addr).await?;
    let host = RemoteClientHost::new(remote);
    let watch = host.spawn_change_watch(CHANGE_WATCH_INTERVAL);
    let server: Arc<McpServer<RemoteClientHost>> = McpServer::new(McpServerConfig::default(), host);
    Arc::clone(&server)
        .serve(listener, std::future::pending())
        .await;
    watch.abort();
    Ok(())
}

/// Eagerly validates and binds a loopback MCP listener, returning the bound address and task handle.
pub async fn spawn_remote(
    addr: SocketAddr,
    remote: Arc<dyn RemoteControlEndpoint>,
) -> Result<(SocketAddr, JoinHandle<()>), RemoteServeError> {
    check_bind(addr)?;
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;
    let host = RemoteClientHost::new(remote);
    let watch = AbortOnDrop(host.spawn_change_watch(CHANGE_WATCH_INTERVAL));
    let server: Arc<McpServer<RemoteClientHost>> = McpServer::new(McpServerConfig::default(), host);
    let handle = tokio::spawn(async move {
        let _watch = watch;
        Arc::clone(&server)
            .serve(listener, std::future::pending())
            .await;
    });
    Ok((local_addr, handle))
}

fn check_control_bind(addr: SocketAddr) -> io::Result<()> {
    check_bind(addr).map_err(|err| io::Error::new(io::ErrorKind::PermissionDenied, err.to_string()))
}

/// Serves the line-delimited JSON remote control API over `remote`.
pub async fn serve_remote_control(
    addr: SocketAddr,
    remote: Arc<dyn RemoteControlEndpoint>,
) -> io::Result<()> {
    check_control_bind(addr)?;
    let listener = TcpListener::bind(addr).await?;
    let connections = Arc::new(Semaphore::new(MAX_REMOTE_CONTROL_CONNECTIONS));
    loop {
        let stream = match listener.accept().await {
            Ok((stream, _)) => stream,
            Err(_) => {
                tokio::time::sleep(ACCEPT_ERROR_BACKOFF).await;
                continue;
            }
        };
        let Ok(permit) = Arc::clone(&connections).try_acquire_owned() else {
            drop(stream);
            continue;
        };
        let remote = Arc::clone(&remote);
        tokio::spawn(async move {
            let _permit = permit;
            let _ = handle_remote_control_client(stream, remote).await;
        });
    }
}

/// Eagerly binds and spawns the line-delimited JSON remote control API over `remote`.
pub async fn spawn_remote_control(
    addr: SocketAddr,
    remote: Arc<dyn RemoteControlEndpoint>,
) -> io::Result<(SocketAddr, JoinHandle<()>)> {
    check_control_bind(addr)?;
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;
    let connections = Arc::new(Semaphore::new(MAX_REMOTE_CONTROL_CONNECTIONS));
    let handle = tokio::spawn(async move {
        loop {
            let stream = match listener.accept().await {
                Ok((stream, _)) => stream,
                Err(_) => {
                    tokio::time::sleep(ACCEPT_ERROR_BACKOFF).await;
                    continue;
                }
            };
            let Ok(permit) = Arc::clone(&connections).try_acquire_owned() else {
                drop(stream);
                continue;
            };
            let remote = Arc::clone(&remote);
            tokio::spawn(async move {
                let _permit = permit;
                let _ = handle_remote_control_client(stream, remote).await;
            });
        }
    });
    Ok((local_addr, handle))
}

async fn handle_remote_control_client(
    stream: TcpStream,
    remote: Arc<dyn RemoteControlEndpoint>,
) -> io::Result<()> {
    handle_remote_control_stream_with_timeout(
        stream,
        remote,
        MAX_CONTROL_LINE_BYTES,
        REMOTE_CONTROL_IO_TIMEOUT,
    )
    .await
}

#[cfg(test)]
async fn handle_remote_control_stream<S>(
    stream: S,
    remote: Arc<dyn RemoteControlEndpoint>,
    max_line_bytes: usize,
) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    handle_remote_control_stream_with_timeout(
        stream,
        remote,
        max_line_bytes,
        REMOTE_CONTROL_IO_TIMEOUT,
    )
    .await
}

async fn handle_remote_control_stream_with_timeout<S>(
    stream: S,
    remote: Arc<dyn RemoteControlEndpoint>,
    max_line_bytes: usize,
    io_timeout: Duration,
) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);
    let mut line_buf = Vec::new();
    loop {
        let outcome = timeout(
            io_timeout,
            read_line(&mut reader, max_line_bytes, &mut line_buf),
        )
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "control read deadline elapsed"))??;
        match outcome {
            LineOutcome::TooLarge => {
                let message = format!("control request line exceeds {max_line_bytes} bytes");
                let response = ControlResponse::err(&message);
                let _ = timeout(io_timeout, async {
                    writer
                        .write_all(response.to_json_string().as_bytes())
                        .await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await
                })
                .await;
                return Err(io::Error::new(io::ErrorKind::InvalidData, message));
            }
            LineOutcome::Eof if line_buf.is_empty() => return Ok(()),
            LineOutcome::Complete | LineOutcome::Eof => {
                let line = std::str::from_utf8(trim_line(&line_buf)).map_err(|error| {
                    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
                })?;
                let response = match ControlRequest::from_json_str(line) {
                    Ok(request) => remote.call(request, false).await.response,
                    Err(err) => ControlResponse::err(err.to_string()),
                };
                if line_buf.capacity() > MAX_RETAINED_CONTROL_LINE_CAPACITY {
                    line_buf = Vec::new();
                }
                timeout(io_timeout, async {
                    writer
                        .write_all(response.to_json_string().as_bytes())
                        .await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await
                })
                .await
                .map_err(|_| {
                    io::Error::new(io::ErrorKind::TimedOut, "control write deadline elapsed")
                })??;
                if outcome == LineOutcome::Eof {
                    return Ok(());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::RemoteControlAnswer;
    use std::{future::Future, pin::Pin};
    use tokio::io::AsyncReadExt;

    struct StubEndpoint;

    impl RemoteControlEndpoint for StubEndpoint {
        fn generation(&self) -> u64 {
            1
        }

        fn call(
            &self,
            request: ControlRequest,
            _confirmed: bool,
        ) -> Pin<Box<dyn Future<Output = RemoteControlAnswer> + Send>> {
            Box::pin(async move { RemoteControlAnswer::ok(json!({ "echo": request.method })) })
        }
    }

    #[tokio::test]
    async fn remote_control_rejects_oversized_newline_free_stream_without_oom() {
        assert_eq!(MAX_CONTROL_LINE_BYTES, 16 * 1024 * 1024);
        let (mut client, server) = tokio::io::duplex(4096);
        let task = tokio::spawn(handle_remote_control_stream(
            server,
            Arc::new(StubEndpoint),
            64,
        ));
        client.write_all(&[b'x'; 256]).await.unwrap();
        let mut reply = Vec::new();
        client.read_to_end(&mut reply).await.unwrap();
        let response =
            ControlResponse::from_json_str(std::str::from_utf8(&reply).unwrap().trim()).unwrap();
        assert!(!response.ok);
        assert!(response
            .error
            .as_deref()
            .is_some_and(|err| err.contains("exceeds 64 bytes")));
        let result = task.await.unwrap();
        assert!(matches!(
            result,
            Err(err) if err.kind() == io::ErrorKind::InvalidData
        ));
    }

    #[tokio::test]
    async fn remote_control_processes_bounded_lines_and_clean_eof() {
        let (mut client, server) = tokio::io::duplex(4096);
        let task = tokio::spawn(handle_remote_control_stream(
            server,
            Arc::new(StubEndpoint),
            256,
        ));
        client
            .write_all(b"{\"method\":\"client.status\",\"params\":null}\n")
            .await
            .unwrap();
        drop(client.shutdown().await);
        let mut reply = Vec::new();
        client.read_to_end(&mut reply).await.unwrap();
        let response =
            ControlResponse::from_json_str(std::str::from_utf8(&reply).unwrap().trim()).unwrap();
        assert!(response.ok);
        assert_eq!(response.result, Some(json!({ "echo": "client.status" })));
        task.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn remote_control_refuses_non_loopback_bind_and_enforces_scope() {
        let remote_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let err = spawn_remote_control(remote_addr, Arc::new(StubEndpoint))
            .await
            .expect_err("non-loopback remote control bind must be refused");
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);

        let host = RemoteClientHost::new(Arc::new(StubEndpoint));
        let read_caller = Caller {
            principal: "reader".to_string(),
            scope: Scope::Read,
            request_id: "req-1".to_string(),
        };
        let denied = host
            .call_tool(&read_caller, "app.quit", &json!({}))
            .expect("returns execution refusal");
        let outcome = denied.answer.await.expect("answered");
        assert!(!outcome.response.ok);
        assert!(outcome
            .response
            .error
            .as_deref()
            .is_some_and(|msg| msg.contains("requires `manage`")));

        let allowed = host
            .call_tool(&read_caller, "app.status", &json!({}))
            .expect("read-only tool allowed");
        let outcome = allowed.answer.await.expect("answered");
        assert!(outcome.response.ok);
    }
}
