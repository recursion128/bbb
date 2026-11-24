//! Reverse WebSocket control tunnel wire types and browser client adapter so
//! `bbb-web-host` exposes the exact `control` and `mcp` surfaces against a live
//! browser tab.
#![deny(dead_code)]

#[cfg(not(target_arch = "wasm32"))]
pub use bbb_mcp::control::{
    ControlTunnelClientMessage, ControlTunnelHostMessage, ControlTunnelScreenshot,
};

#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use crate::control::types::ControlRequest;
#[cfg(target_arch = "wasm32")]
use crate::control::ControlResponse;

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlTunnelScreenshot {
    pub width: u32,
    pub height: u32,
    pub mime_type: String,
    pub data_base64: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlTunnelClientMessage {
    Hello {
        version: String,
        generation: u64,
    },
    Generation {
        generation: u64,
    },
    Response {
        id: u64,
        response: ControlResponse,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        image: Option<ControlTunnelScreenshot>,
    },
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlTunnelHostMessage {
    Request {
        id: u64,
        confirmed: bool,
        request: ControlRequest,
    },
}

#[cfg(target_arch = "wasm32")]
const GENERATION_PUSH_MIN_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);

#[cfg(target_arch = "wasm32")]
pub struct WebControlTunnel {
    socket: web_sys::WebSocket,
    consumer_active: std::rc::Rc<std::cell::Cell<bool>>,
    latest_generation: std::rc::Rc<std::cell::Cell<u64>>,
    last_sent_generation: std::rc::Rc<std::cell::Cell<u64>>,
    last_sent_at: Option<bbb_client_clock::Instant>,
}

#[cfg(target_arch = "wasm32")]
impl WebControlTunnel {
    pub fn connect(
        ws_url: &str,
        control: crate::control::ControlState,
        version: String,
    ) -> anyhow::Result<Self> {
        use std::{cell::Cell, rc::Rc};

        use base64::{engine::general_purpose::STANDARD, Engine as _};
        use serde_json::json;
        use wasm_bindgen::{closure::Closure, JsCast};
        use web_sys::{CloseEvent, MessageEvent, WebSocket};

        use crate::control::{dispatch, dispatch_confirmed, ControlCompletion};

        let socket = WebSocket::new(ws_url)
            .map_err(|err| anyhow::anyhow!("open control WebSocket {ws_url}: {err:?}"))?;

        let initial_generation = control.snapshot.load().generation;
        let consumer_active = Rc::new(Cell::new(false));
        let latest_generation = Rc::new(Cell::new(initial_generation));
        let last_sent_generation = Rc::new(Cell::new(initial_generation));

        let open_socket = socket.clone();
        let open_latest_gen = Rc::clone(&latest_generation);
        let open_sent_gen = Rc::clone(&last_sent_generation);
        let onopen = Closure::<dyn FnMut()>::new(move || {
            let gen = open_latest_gen.get();
            open_sent_gen.set(gen);
            let hello = ControlTunnelClientMessage::Hello {
                version: version.clone(),
                generation: gen,
            };
            if let Ok(text) = serde_json::to_string(&hello) {
                let _ = open_socket.send_with_str(&text);
            }
        });
        socket.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        onopen.forget();

        let msg_socket = socket.clone();
        let msg_active = Rc::clone(&consumer_active);
        let msg_latest_gen = Rc::clone(&latest_generation);
        let msg_sent_gen = Rc::clone(&last_sent_generation);
        let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            let Some(text) = event.data().as_string() else {
                return;
            };
            let Ok(ControlTunnelHostMessage::Request {
                id,
                confirmed,
                request,
            }) = serde_json::from_str::<ControlTunnelHostMessage>(&text)
            else {
                return;
            };

            if !msg_active.replace(true) {
                let latest = msg_latest_gen.get();
                if latest != msg_sent_gen.get() {
                    msg_sent_gen.set(latest);
                    let _ = msg_socket.send_with_str(&format!(
                        "{{\"type\":\"generation\",\"generation\":{latest}}}"
                    ));
                }
            }

            if !confirmed && request.method != "renderer.screenshot" {
                control.publication.record_demand();
                if crate::control::method_reads_snapshot(&request.method)
                    && control.publication.needs_flush()
                {
                    let reply_socket = msg_socket.clone();
                    let reply_control = control.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        reply_control.ensure_fresh_snapshot().await;
                        let response = dispatch(request, &reply_control);
                        let reply = ControlTunnelClientMessage::Response {
                            id,
                            response,
                            image: None,
                        };
                        if let Ok(payload) = serde_json::to_string(&reply) {
                            let _ = reply_socket.send_with_str(&payload);
                        }
                    });
                    return;
                }
                let response = dispatch(request, &control);
                let reply = ControlTunnelClientMessage::Response {
                    id,
                    response,
                    image: None,
                };
                if let Ok(payload) = serde_json::to_string(&reply) {
                    let _ = msg_socket.send_with_str(&payload);
                }
                return;
            }
            control.publication.record_demand();

            let (mut response, completion) = dispatch_confirmed(request, &control);
            let Some(completion) = completion.filter(|_| response.ok) else {
                let reply = ControlTunnelClientMessage::Response {
                    id,
                    response,
                    image: None,
                };
                if let Ok(payload) = serde_json::to_string(&reply) {
                    let _ = msg_socket.send_with_str(&payload);
                }
                return;
            };

            let reply_socket = msg_socket.clone();
            let reply_sent_gen = Rc::clone(&msg_sent_gen);
            wasm_bindgen_futures::spawn_local(async move {
                let mut image_wire = None;
                match completion
                    .await
                    .unwrap_or_else(|_| Err("client stopped before completing the request".into()))
                {
                    Ok(ControlCompletion::Recording(status)) => {
                        response.result = Some(json!({"completed": true, "recording": status}));
                    }
                    Ok(ControlCompletion::Processed { generation }) => {
                        reply_sent_gen.set(generation);
                        if let Some(serde_json::Value::Object(result)) = response.result.as_mut() {
                            result.remove("queued");
                            result.remove("pending");
                            result.insert("processed".into(), json!(true));
                            result.insert("generation".into(), json!(generation));
                        }
                    }
                    Ok(ControlCompletion::Screenshot { path, image }) => {
                        response.result = Some(json!({
                            "completed": true,
                            "path": path,
                            "width": image.width,
                            "height": image.height,
                            "mime_type": image.mime_type,
                        }));
                        image_wire = Some(ControlTunnelScreenshot {
                            width: image.width,
                            height: image.height,
                            mime_type: image.mime_type.to_string(),
                            data_base64: STANDARD.encode(&image.bytes),
                        });
                    }
                    Err(error) => {
                        response = ControlResponse {
                            ok: false,
                            result: None,
                            error: Some(error),
                        };
                    }
                }
                let reply = ControlTunnelClientMessage::Response {
                    id,
                    response,
                    image: image_wire,
                };
                if let Ok(payload) = serde_json::to_string(&reply) {
                    let _ = reply_socket.send_with_str(&payload);
                }
            });
        });
        socket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        let close_active = Rc::clone(&consumer_active);
        let onclose = Closure::<dyn FnMut(CloseEvent)>::new(move |event: CloseEvent| {
            close_active.set(false);
            tracing::debug!(
                code = event.code(),
                reason = %event.reason(),
                "web control tunnel closed"
            );
        });
        socket.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        onclose.forget();

        Ok(Self {
            socket,
            consumer_active,
            latest_generation,
            last_sent_generation,
            last_sent_at: None,
        })
    }

    pub fn publish_generation(&mut self, generation: u64) {
        self.latest_generation.set(generation);
        if !self.consumer_active.get()
            || generation == self.last_sent_generation.get()
            || self.socket.ready_state() != 1
            || self.socket.buffered_amount() > 0
        {
            return;
        }
        let now = bbb_client_clock::Instant::now();
        if !bbb_client_clock::is_virtual() {
            if let Some(prev) = self.last_sent_at {
                if now.duration_since(prev) < GENERATION_PUSH_MIN_INTERVAL {
                    return;
                }
            }
        }
        self.last_sent_generation.set(generation);
        self.last_sent_at = Some(now);
        let _ = self.socket.send_with_str(&format!(
            "{{\"type\":\"generation\",\"generation\":{generation}}}"
        ));
    }
}
