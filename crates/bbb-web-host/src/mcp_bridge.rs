//! Reverse WebSocket control tunnel endpoint bridging `bbb-web-host` (`--mcp-bind`
//! and `--control`) to the active browser WASM client tab over `/control-ws`.

use std::{
    collections::HashMap,
    path::PathBuf,
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use anyhow::Result;
use bbb_mcp::control::{
    ControlRequest, ControlResponse, ControlTunnelClientMessage, ControlTunnelHostMessage,
    RemoteControlAnswer, RemoteControlEndpoint,
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, oneshot},
};

use crate::{
    http::HttpRequest,
    ws_codec::{upgrade_into_ws, WsMessage},
};

const CONTROL_CALL_TIMEOUT: Duration = Duration::from_secs(20);

struct ActiveBrowserSession {
    session_id: u64,
    tx: mpsc::UnboundedSender<WsMessage>,
    pending: HashMap<u64, oneshot::Sender<RemoteControlAnswer>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

pub struct BrowserControlBridge {
    generation: AtomicU64,
    next_session_id: AtomicU64,
    next_request_id: AtomicU64,
    session: Arc<Mutex<Option<ActiveBrowserSession>>>,
}

impl Default for BrowserControlBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserControlBridge {
    pub fn new() -> Self {
        Self {
            generation: AtomicU64::new(0),
            next_session_id: AtomicU64::new(1),
            next_request_id: AtomicU64::new(1),
            session: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn accept_browser_tunnel(
        self: Arc<Self>,
        req: HttpRequest,
        stream: TcpStream,
        leftover: Vec<u8>,
    ) -> Result<()> {
        let _ = stream.set_nodelay(true);
        let Some(ws) = upgrade_into_ws(&req, stream, leftover).await? else {
            return Ok(());
        };
        let (mut ws_reader, mut ws_writer) = ws.into_parts();
        let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        let session_id = self.next_session_id.fetch_add(1, Ordering::Relaxed);

        {
            let mut guard = self.session.lock().unwrap_or_else(|err| err.into_inner());
            if let Some(prev) = guard.take() {
                if let Some(stop) = prev.shutdown_tx {
                    let _ = stop.send(());
                }
                fail_all_pending(
                    prev.pending,
                    "browser control session superseded by newer tab",
                );
            }
            *guard = Some(ActiveBrowserSession {
                session_id,
                tx: tx.clone(),
                pending: HashMap::new(),
                shutdown_tx: Some(shutdown_tx),
            });
        }

        let writer_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_writer.send(&msg).await.is_err() {
                    break;
                }
            }
        });

        loop {
            let next = tokio::select! {
                _ = &mut shutdown_rx => break,
                res = ws_reader.recv() => res,
            };
            let Ok(Some(msg)) = next else {
                break;
            };
            match msg {
                WsMessage::Text(text) => {
                    if let Ok(parsed) = ControlTunnelClientMessage::from_json_str(&text) {
                        self.handle_client_message(session_id, parsed);
                    }
                }
                WsMessage::Binary(bytes) => {
                    if let Ok(parsed) = ControlTunnelClientMessage::from_json_slice(&bytes) {
                        self.handle_client_message(session_id, parsed);
                    }
                }
                WsMessage::Ping(payload) => {
                    let _ = tx.send(WsMessage::Pong(payload));
                }
                WsMessage::Pong(_) => {}
                WsMessage::Close => break,
            }
        }

        writer_task.abort();
        let mut guard = self.session.lock().unwrap_or_else(|err| err.into_inner());
        if guard.as_ref().is_some_and(|s| s.session_id == session_id) {
            if let Some(ended) = guard.take() {
                fail_all_pending(ended.pending, "browser control WebSocket closed");
            }
        }
        Ok(())
    }

    fn handle_client_message(&self, session_id: u64, msg: ControlTunnelClientMessage) {
        match msg {
            ControlTunnelClientMessage::Hello {
                version,
                generation,
            } => {
                let guard = self.session.lock().unwrap_or_else(|err| err.into_inner());
                if guard.as_ref().is_some_and(|s| s.session_id == session_id) {
                    self.generation.store(generation, Ordering::Release);
                    tracing::info!(%version, generation, "browser tab connected to /control-ws");
                }
            }
            ControlTunnelClientMessage::Generation { generation } => {
                let guard = self.session.lock().unwrap_or_else(|err| err.into_inner());
                if guard.as_ref().is_some_and(|s| s.session_id == session_id) {
                    self.generation.store(generation, Ordering::Release);
                }
            }
            ControlTunnelClientMessage::Response {
                id,
                response,
                image,
            } => {
                let tx = {
                    let mut guard = self.session.lock().unwrap_or_else(|err| err.into_inner());
                    guard
                        .as_mut()
                        .filter(|s| s.session_id == session_id)
                        .and_then(|s| s.pending.remove(&id))
                };
                if let Some(tx) = tx {
                    let decoded_image = image.and_then(|shot| shot.decode_lossy());
                    let _ = tx.send(RemoteControlAnswer {
                        response,
                        image: decoded_image,
                    });
                }
            }
        }
    }
    fn call_with_timeout(
        &self,
        request: ControlRequest,
        confirmed: bool,
        timeout: Duration,
    ) -> Pin<Box<dyn std::future::Future<Output = RemoteControlAnswer> + Send>> {
        let requested_screenshot_path = extract_screenshot_path(&request);
        let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let host_msg = ControlTunnelHostMessage::Request {
            id,
            confirmed,
            request,
        };
        let encoded = host_msg.to_json_string();

        let (reply_tx, reply_rx) = oneshot::channel();
        let active_session_id = {
            let mut guard = self.session.lock().unwrap_or_else(|err| err.into_inner());
            if let Some(session) = guard.as_mut() {
                if session.tx.send(WsMessage::Text(encoded)).is_ok() {
                    session.pending.insert(id, reply_tx);
                    Some(session.session_id)
                } else {
                    None
                }
            } else {
                None
            }
        };

        let Some(active_session_id) = active_session_id else {
            return Box::pin(async move {
                error_answer("no active browser client connected on /control-ws".to_string())
            });
        };

        let session = Arc::clone(&self.session);
        Box::pin(async move {
            let mut cleanup = PendingCleanupGuard {
                session,
                session_id: active_session_id,
                request_id: id,
                armed: true,
            };
            let mut answer = match tokio::time::timeout(timeout, reply_rx).await {
                Ok(Ok(ans)) => {
                    cleanup.disarm();
                    ans
                }
                Ok(Err(_)) => {
                    cleanup.disarm();
                    error_answer("browser control tunnel dropped mid-call".to_string())
                }
                Err(_) => {
                    drop(cleanup);
                    error_answer(format!(
                        "browser control request {id} timed out after {}s",
                        timeout.as_secs()
                    ))
                }
            };

            if let (Some(path), Some(img)) = (requested_screenshot_path, answer.image.as_ref()) {
                if answer.response.ok {
                    if let Some(parent) = path.parent() {
                        if !parent.as_os_str().is_empty() {
                            let _ = tokio::fs::create_dir_all(parent).await;
                        }
                    }
                    if let Err(err) = tokio::fs::write(&path, &img.bytes).await {
                        answer.response = ControlResponse {
                            ok: false,
                            result: None,
                            error: Some(format!(
                                "write screenshot to host path {}: {err}",
                                path.display()
                            )),
                        };
                    } else if let Some(serde_json::Value::Object(ref mut map)) =
                        answer.response.result
                    {
                        map.insert(
                            "path".to_string(),
                            serde_json::Value::String(path.display().to_string()),
                        );
                    }
                }
            }

            answer
        })
    }
}

struct PendingCleanupGuard {
    session: Arc<Mutex<Option<ActiveBrowserSession>>>,
    session_id: u64,
    request_id: u64,
    armed: bool,
}

impl PendingCleanupGuard {
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for PendingCleanupGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let mut guard = self.session.lock().unwrap_or_else(|err| err.into_inner());
        if let Some(session) = guard.as_mut().filter(|s| s.session_id == self.session_id) {
            session.pending.remove(&self.request_id);
        }
    }
}

impl RemoteControlEndpoint for BrowserControlBridge {
    fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    fn call(
        &self,
        request: ControlRequest,
        confirmed: bool,
    ) -> Pin<Box<dyn std::future::Future<Output = RemoteControlAnswer> + Send>> {
        self.call_with_timeout(request, confirmed, CONTROL_CALL_TIMEOUT)
    }
}

fn extract_screenshot_path(request: &ControlRequest) -> Option<PathBuf> {
    if request.method != "renderer.screenshot" {
        return None;
    }
    request
        .params
        .get("path")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

fn fail_all_pending(pending: HashMap<u64, oneshot::Sender<RemoteControlAnswer>>, reason: &str) {
    for (_, tx) in pending {
        let _ = tx.send(error_answer(reason.to_string()));
    }
}

fn error_answer(msg: String) -> RemoteControlAnswer {
    RemoteControlAnswer {
        response: ControlResponse {
            ok: false,
            result: None,
            error: Some(msg),
        },
        image: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn timed_out_control_request_removes_pending_entry() {
        let bridge = BrowserControlBridge::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        {
            let mut guard = bridge.session.lock().unwrap();
            *guard = Some(ActiveBrowserSession {
                session_id: 7,
                tx,
                pending: HashMap::new(),
                shutdown_tx: None,
            });
        }

        let answer = bridge
            .call_with_timeout(
                ControlRequest {
                    method: "state.snapshot".to_string(),
                    params: serde_json::json!({}),
                },
                false,
                Duration::from_millis(20),
            )
            .await;

        assert!(!answer.response.ok);
        assert!(
            answer
                .response
                .error
                .as_deref()
                .unwrap_or_default()
                .contains("timed out"),
            "{:?}",
            answer.response
        );
        assert!(rx.try_recv().is_ok());

        let guard = bridge.session.lock().unwrap();
        let pending_len = guard
            .as_ref()
            .map(|s| s.pending.len())
            .unwrap_or(usize::MAX);
        assert_eq!(
            pending_len, 0,
            "timed-out request must be evicted from pending map"
        );
    }

    #[test]
    fn stale_control_session_cannot_mutate_generation() {
        let bridge = BrowserControlBridge::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        {
            let mut guard = bridge.session.lock().unwrap();
            *guard = Some(ActiveBrowserSession {
                session_id: 2,
                tx,
                pending: HashMap::new(),
                shutdown_tx: None,
            });
        }
        bridge.handle_client_message(
            2,
            ControlTunnelClientMessage::Hello {
                version: "v2".to_string(),
                generation: 99,
            },
        );
        assert_eq!(bridge.generation(), 99);

        bridge.handle_client_message(
            1,
            ControlTunnelClientMessage::Hello {
                version: "stale".to_string(),
                generation: 1,
            },
        );
        bridge.handle_client_message(1, ControlTunnelClientMessage::Generation { generation: 2 });
        assert_eq!(
            bridge.generation(),
            99,
            "stale session must not overwrite active generation"
        );
    }
}
