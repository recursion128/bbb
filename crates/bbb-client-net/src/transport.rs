//! Transport stream abstraction shared by native TCP sockets and browser
//! binary WebSockets without duplicating the protocol state machine.

#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWrite};

pub trait ClientStream: AsyncRead + AsyncWrite + Unpin + Send {}

impl<T: AsyncRead + AsyncWrite + Unpin + Send> ClientStream for T {}

pub(crate) type BoxedClientStream = Box<dyn ClientStream>;

#[cfg(target_arch = "wasm32")]
pub(crate) fn build_gateway_ws_url(gateway: &str, server_address: &str) -> String {
    let encoded: String = url::form_urlencoded::byte_serialize(server_address.as_bytes()).collect();
    if gateway.contains('?') {
        format!("{gateway}&server={encoded}")
    } else {
        format!("{gateway}?server={encoded}")
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn connect_transport(
    address: &str,
    _websocket_gateway: Option<&str>,
    timeout_duration: Duration,
) -> Result<(BoxedClientStream, Option<(String, u16)>)> {
    connect_tcp(address, timeout_duration, minecraft_srv).await
}

#[cfg(not(target_arch = "wasm32"))]
async fn connect_tcp(
    address: &str,
    timeout_duration: Duration,
    lookup_srv: impl AsyncFnOnce(&str) -> Option<(String, u16)>,
) -> Result<(BoxedClientStream, Option<(String, u16)>)> {
    use anyhow::Context;
    let (host, port) = crate::types::split_host_port(address)?;
    let (stream, redirect) = tokio::time::timeout(timeout_duration, async {
        let redirect = if port == crate::types::DEFAULT_SERVER_PORT
            && host.parse::<std::net::IpAddr>().is_err()
        {
            lookup_srv(&host).await
        } else {
            None
        };
        let target = redirect
            .as_ref()
            .map_or((host.as_str(), port), |(host, port)| (host.as_str(), *port));
        let stream = tokio::net::TcpStream::connect(target).await?;
        Ok::<_, std::io::Error>((stream, redirect))
    })
    .await
    .unwrap_or_else(|elapsed| {
        Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("connect to {address} timed out ({elapsed})"),
        ))
    })
    .with_context(|| format!("connect {address}"))?;
    stream.set_nodelay(true).ok();
    Ok((Box::new(stream), redirect))
}

#[cfg(not(target_arch = "wasm32"))]
fn shared_srv_resolver() -> Option<&'static hickory_resolver::TokioResolver> {
    use hickory_resolver::TokioResolver;
    use std::sync::OnceLock;
    static RESOLVER: OnceLock<Option<TokioResolver>> = OnceLock::new();
    RESOLVER
        .get_or_init(|| {
            let mut builder = TokioResolver::builder_tokio().ok()?;
            builder.options_mut().attempts = 1;
            builder.options_mut().timeout = Duration::from_secs(1);
            builder.build().ok()
        })
        .as_ref()
}

/// Selects one DNS SRV target per RFC 2782 (lowest priority first, then
/// weight-ordered selection within the lowest priority tier), stripping any
/// trailing DNS root dot and rejecting empty or port-0 targets.
#[cfg(not(target_arch = "wasm32"))]
fn select_srv_target(
    records: &[(u16, u16, String, u16)],
    random_draw: u64,
) -> Option<(String, u16)> {
    let mut candidates: Vec<(u16, u16, &str, u16)> = records
        .iter()
        .filter_map(|(priority, weight, target, port)| {
            let host = target.trim_end_matches('.');
            (!host.is_empty() && *port != 0).then_some((*priority, *weight, host, *port))
        })
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let min_priority = candidates
        .iter()
        .map(|(priority, _, _, _)| *priority)
        .min()?;
    candidates.retain(|(priority, _, _, _)| *priority == min_priority);
    // RFC 2782: place weight-0 records first before building cumulative weight sums.
    candidates.sort_by_key(|(_, weight, host, port)| (*weight > 0, *weight, *host, *port));
    let total_weight: u64 = candidates
        .iter()
        .map(|(_, weight, _, _)| u64::from(*weight))
        .sum();
    if total_weight == 0 {
        let idx = (random_draw as usize) % candidates.len();
        let (_, _, host, port) = candidates[idx];
        return Some((host.to_string(), port));
    }
    let pick = random_draw % (total_weight + 1);
    let mut running = 0u64;
    for (_, weight, host, port) in &candidates {
        running += u64::from(*weight);
        if running >= pick {
            return Some(((*host).to_string(), *port));
        }
    }
    let (_, _, host, port) = candidates.last()?;
    Some(((*host).to_string(), *port))
}

#[cfg(not(target_arch = "wasm32"))]
async fn minecraft_srv(host: &str) -> Option<(String, u16)> {
    use hickory_resolver::proto::rr::RData;
    use std::time::{SystemTime, UNIX_EPOCH};
    let resolver = shared_srv_resolver()?;
    let lookup = resolver
        .srv_lookup(format!("_minecraft._tcp.{host}"))
        .await
        .ok()?;
    let records: Vec<(u16, u16, String, u16)> = lookup
        .answers()
        .iter()
        .filter_map(|record| match &record.data {
            RData::SRV(srv) => Some((srv.priority, srv.weight, srv.target.to_ascii(), srv.port)),
            _ => None,
        })
        .collect();
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    select_srv_target(&records, seed)
}

#[cfg(any(target_arch = "wasm32", test))]
mod coalesce {
    use std::{
        io,
        pin::Pin,
        sync::{Arc, Mutex},
        task::{Context as TaskContext, Poll, Waker},
    };

    pub(crate) const MAX_WS_OUTBOUND_BUFFER_BYTES: usize = 256 * 1024;
    #[cfg(target_arch = "wasm32")]
    pub(crate) const MAX_WS_SINGLE_FRAME_BYTES: usize = 64 * 1024;

    pub(crate) struct OutboundCoalesceState {
        buf: Vec<u8>,
        closed: bool,
        failed: Option<String>,
        writer_waker: Option<Waker>,
        pump_waker: Option<Waker>,
    }

    #[derive(Clone)]
    pub(crate) struct OutboundCoalesceShared(Arc<Mutex<OutboundCoalesceState>>);

    impl OutboundCoalesceShared {
        pub(crate) fn new() -> Self {
            Self(Arc::new(Mutex::new(OutboundCoalesceState {
                buf: Vec::with_capacity(8192),
                closed: false,
                failed: None,
                writer_waker: None,
                pump_waker: None,
            })))
        }

        fn lock(&self) -> std::sync::MutexGuard<'_, OutboundCoalesceState> {
            self.0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
        }

        pub(crate) fn poll_write(
            &self,
            cx: &mut TaskContext<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            if buf.is_empty() {
                return Poll::Ready(Ok(0));
            }
            let pump_waker = {
                let mut state = self.lock();
                if let Some(reason) = &state.failed {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        reason.clone(),
                    )));
                }
                if state.closed {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "websocket outbound closed",
                    )));
                }
                if !state.buf.is_empty() && state.buf.len() >= MAX_WS_OUTBOUND_BUFFER_BYTES {
                    state.writer_waker = Some(cx.waker().clone());
                    let pump = state.pump_waker.take();
                    drop(state);
                    if let Some(waker) = pump {
                        waker.wake();
                    }
                    return Poll::Pending;
                }
                state.buf.extend_from_slice(buf);
                state.pump_waker.take()
            };
            if let Some(waker) = pump_waker {
                waker.wake();
            }
            Poll::Ready(Ok(buf.len()))
        }

        pub(crate) fn poll_flush(&self) -> Poll<io::Result<()>> {
            let pump_waker = {
                let mut state = self.lock();
                if let Some(reason) = &state.failed {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        reason.clone(),
                    )));
                }
                (!state.buf.is_empty())
                    .then(|| state.pump_waker.take())
                    .flatten()
            };
            if let Some(waker) = pump_waker {
                waker.wake();
            }
            Poll::Ready(Ok(()))
        }

        pub(crate) fn close(&self) {
            let pump_waker = {
                let mut state = self.lock();
                state.closed = true;
                state.pump_waker.take()
            };
            if let Some(waker) = pump_waker {
                waker.wake();
            }
        }

        pub(crate) fn fail(&self, message: String) {
            let writer_waker = {
                let mut state = self.lock();
                state.closed = true;
                state.failed = Some(message);
                state.writer_waker.take()
            };
            if let Some(waker) = writer_waker {
                waker.wake();
            }
        }

        pub(crate) async fn next_coalesced_batch(&self, out: &mut Vec<u8>) -> bool {
            std::future::poll_fn(|cx| {
                let mut state = self.lock();
                if !state.buf.is_empty() {
                    out.clear();
                    std::mem::swap(out, &mut state.buf);
                    let writer = state.writer_waker.take();
                    drop(state);
                    if let Some(waker) = writer {
                        waker.wake();
                    }
                    return Poll::Ready(true);
                }
                if state.closed {
                    return Poll::Ready(false);
                }
                state.pump_waker = Some(cx.waker().clone());
                let _ = Pin::new(&mut ());
                Poll::Pending
            })
            .await
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::{
        cell::RefCell,
        io,
        pin::Pin,
        rc::Rc,
        task::{Context as TaskContext, Poll},
        time::Duration,
    };

    use anyhow::{anyhow, bail, Result};
    use bytes::Bytes;
    use tokio::{
        io::{AsyncRead, AsyncWrite, ReadBuf},
        sync::{mpsc, oneshot},
    };
    use wasm_bindgen::{closure::Closure, JsCast, JsValue};
    use web_sys::{BinaryType, CloseEvent, MessageEvent, WebSocket};

    use super::{
        build_gateway_ws_url,
        coalesce::{OutboundCoalesceShared, MAX_WS_SINGLE_FRAME_BYTES},
        BoxedClientStream,
    };

    const WS_INBOUND_CHANNEL_CAPACITY: usize = 2048;

    pub(crate) struct WebSocketClientStream {
        inbound_rx: mpsc::Receiver<io::Result<Bytes>>,
        outbound: OutboundCoalesceShared,
        pending_read: Bytes,
        eof: bool,
    }

    impl Drop for WebSocketClientStream {
        fn drop(&mut self) {
            self.outbound.close();
        }
    }

    impl AsyncRead for WebSocketClientStream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut TaskContext<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            if self.eof || buf.remaining() == 0 {
                return Poll::Ready(Ok(()));
            }
            loop {
                if !self.pending_read.is_empty() {
                    let take = self.pending_read.len().min(buf.remaining());
                    let chunk = self.pending_read.split_to(take);
                    buf.put_slice(&chunk);
                    return Poll::Ready(Ok(()));
                }
                match self.inbound_rx.poll_recv(cx) {
                    Poll::Ready(Some(Ok(bytes))) => {
                        if bytes.is_empty() {
                            continue;
                        }
                        self.pending_read = bytes;
                    }
                    Poll::Ready(Some(Err(error))) => return Poll::Ready(Err(error)),
                    Poll::Ready(None) => {
                        self.eof = true;
                        return Poll::Ready(Ok(()));
                    }
                    Poll::Pending => return Poll::Pending,
                }
            }
        }
    }

    impl AsyncWrite for WebSocketClientStream {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut TaskContext<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            self.outbound.poll_write(cx, buf)
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
            self.outbound.poll_flush()
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
            self.outbound.close();
            Poll::Ready(Ok(()))
        }
    }

    fn default_gateway_from_global() -> Result<String> {
        let global = js_sys::global();
        let location = js_sys::Reflect::get(&global, &JsValue::from_str("location"))
            .map_err(|_| anyhow!("missing global location for WebSocket transport"))?;
        if location.is_undefined() || location.is_null() {
            bail!("missing global location for WebSocket transport");
        }
        let protocol = js_sys::Reflect::get(&location, &JsValue::from_str("protocol"))
            .ok()
            .and_then(|val| val.as_string())
            .ok_or_else(|| anyhow!("read location.protocol"))?;
        let host = js_sys::Reflect::get(&location, &JsValue::from_str("host"))
            .ok()
            .and_then(|val| val.as_string())
            .ok_or_else(|| anyhow!("read location.host"))?;
        let ws_scheme = if protocol == "https:" { "wss" } else { "ws" };
        Ok(format!("{ws_scheme}://{host}/ws"))
    }

    struct JsTimeoutGuard {
        id: Option<JsValue>,
    }

    impl Drop for JsTimeoutGuard {
        fn drop(&mut self) {
            if let Some(id) = self.id.take() {
                let global = js_sys::global();
                if let Ok(clear) = js_sys::Reflect::get(&global, &JsValue::from_str("clearTimeout"))
                {
                    if let Ok(func) = clear.dyn_into::<js_sys::Function>() {
                        let _ = func.call1(&global, &id);
                    }
                }
            }
        }
    }

    async fn sleep_ms(millis: i32) {
        let mut guard = JsTimeoutGuard { id: None };
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            let global = js_sys::global();
            if let Ok(set_timeout) = js_sys::Reflect::get(&global, &JsValue::from_str("setTimeout"))
            {
                if let Ok(func) = set_timeout.dyn_into::<js_sys::Function>() {
                    if let Ok(id) = func.call2(
                        &global,
                        &resolve,
                        &JsValue::from_f64(f64::from(millis.max(1))),
                    ) {
                        guard.id = Some(id);
                    }
                }
            }
        });
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
        // Timer fired cleanly; disarm clearTimeout in Drop.
        guard.id = None;
    }

    pub(crate) async fn sleep_duration(duration: Duration) {
        let millis = i32::try_from(duration.as_millis())
            .unwrap_or(i32::MAX)
            .max(1);
        sleep_ms(millis).await;
    }

    struct WsConnectGuard {
        ws: Option<WebSocket>,
    }

    impl WsConnectGuard {
        fn new(ws: WebSocket) -> Self {
            Self { ws: Some(ws) }
        }

        fn get(&self) -> &WebSocket {
            self.ws
                .as_ref()
                .expect("WebSocket connect guard still armed")
        }

        fn disarm(mut self) -> WebSocket {
            self.ws
                .take()
                .expect("WebSocket connect guard already disarmed")
        }
    }

    impl Drop for WsConnectGuard {
        fn drop(&mut self) {
            if let Some(ws) = self.ws.take() {
                ws.set_onopen(None);
                ws.set_onmessage(None);
                ws.set_onerror(None);
                ws.set_onclose(None);
                let _ = ws.close();
            }
        }
    }

    pub(crate) async fn connect_transport(
        address: &str,
        websocket_gateway: Option<&str>,
        timeout_duration: Duration,
    ) -> Result<(BoxedClientStream, Option<(String, u16)>)> {
        let gateway = match websocket_gateway {
            Some(gateway) => gateway.to_string(),
            None => default_gateway_from_global()?,
        };
        let ws_url = build_gateway_ws_url(&gateway, address);
        let ws =
            WebSocket::new(&ws_url).map_err(|err| anyhow!("open WebSocket {ws_url}: {err:?}"))?;
        ws.set_binary_type(BinaryType::Arraybuffer);

        let (open_tx, open_rx) = oneshot::channel::<Result<()>>();
        let open_tx = Rc::new(RefCell::new(Some(open_tx)));
        let (inbound_tx, inbound_rx) =
            mpsc::channel::<io::Result<Bytes>>(WS_INBOUND_CHANNEL_CAPACITY);
        let inbound_tx_cell = Rc::new(RefCell::new(Some(inbound_tx)));

        // Declare JS closures before `ws_guard` so that on any early return or
        // `select!` cancellation `ws_guard` drops first, detaching all handlers
        // and closing the socket before the closures are freed.
        let open_signal = Rc::clone(&open_tx);
        let on_open = Closure::<dyn FnMut()>::new(move || {
            if let Some(tx) = open_signal.borrow_mut().take() {
                let _ = tx.send(Ok(()));
            }
        });

        let msg_tx = Rc::clone(&inbound_tx_cell);
        let ws_for_msg = ws.clone();
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            if let Ok(buffer) = event.data().dyn_into::<js_sys::ArrayBuffer>() {
                let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
                let overflow = {
                    let guard = msg_tx.borrow();
                    match guard.as_ref() {
                        Some(tx) => matches!(
                            tx.try_send(Ok(Bytes::from(bytes))),
                            Err(mpsc::error::TrySendError::Full(_))
                        ),
                        None => false,
                    }
                };
                if overflow {
                    let _ = msg_tx.borrow_mut().take();
                    let _ = ws_for_msg.close_with_code_and_reason(1009, "inbound queue overflow");
                }
            }
        });

        let err_open = Rc::clone(&open_tx);
        let err_inbound = Rc::clone(&inbound_tx_cell);
        let url_for_err = ws_url.clone();
        let on_error = Closure::<dyn FnMut(JsValue)>::new(move |_event: JsValue| {
            if let Some(tx) = err_open.borrow_mut().take() {
                let _ = tx.send(Err(anyhow!("WebSocket connection failed: {url_for_err}")));
            } else if let Some(tx) = err_inbound.borrow_mut().take() {
                let _ = tx.try_send(Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "WebSocket error",
                )));
            }
        });

        let close_open = Rc::clone(&open_tx);
        let close_inbound = Rc::clone(&inbound_tx_cell);
        let on_close = Closure::<dyn FnMut(CloseEvent)>::new(move |event: CloseEvent| {
            if let Some(tx) = close_open.borrow_mut().take() {
                let _ = tx.send(Err(anyhow!(
                    "WebSocket closed before open (code {}, reason {:?})",
                    event.code(),
                    event.reason()
                )));
            }
            let _ = close_inbound.borrow_mut().take();
        });

        let ws_guard = WsConnectGuard::new(ws);
        ws_guard
            .get()
            .set_onopen(Some(on_open.as_ref().unchecked_ref()));
        ws_guard
            .get()
            .set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        ws_guard
            .get()
            .set_onerror(Some(on_error.as_ref().unchecked_ref()));
        ws_guard
            .get()
            .set_onclose(Some(on_close.as_ref().unchecked_ref()));

        let timeout_ms = i32::try_from(timeout_duration.as_millis()).unwrap_or(20_000);
        let open_outcome = tokio::select! {
            res = open_rx => res.map_err(|_| anyhow!("WebSocket open channel dropped"))?,
            _ = sleep_ms(timeout_ms) => {
                return Err(anyhow::Error::new(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("WebSocket connect timed out: {ws_url}"),
                )));
            }
        };
        open_outcome?;
        let ws = ws_guard.disarm();

        let outbound = OutboundCoalesceShared::new();
        let pump_outbound = outbound.clone();
        let pump_inbound = Rc::clone(&inbound_tx_cell);
        wasm_bindgen_futures::spawn_local(async move {
            let _keep_open = on_open;
            let _keep_message = on_message;
            let _keep_error = on_error;
            let _keep_close = on_close;
            let mut batch = Vec::with_capacity(8192);
            'pump: while pump_outbound.next_coalesced_batch(&mut batch).await {
                for chunk in batch.chunks(MAX_WS_SINGLE_FRAME_BYTES) {
                    // `WebSocket.send` rejects `ArrayBufferView` backed by WASM's
                    // `SharedArrayBuffer`; copy the coalesced batch once into an
                    // unshared JS `ArrayBuffer`.
                    let unshared = js_sys::Uint8Array::new_with_length(chunk.len() as u32);
                    unshared.copy_from(chunk);
                    if let Err(err) = ws.send_with_array_buffer(&unshared.buffer()) {
                        let msg = format!("WebSocket send failed: {err:?}");
                        tracing::warn!(?err, "WebSocket binary send failed");
                        pump_outbound.fail(msg.clone());
                        if let Some(tx) = pump_inbound.borrow_mut().take() {
                            let _ =
                                tx.try_send(Err(io::Error::new(io::ErrorKind::BrokenPipe, msg)));
                        }
                        break 'pump;
                    }
                }
            }
            ws.set_onopen(None);
            ws.set_onmessage(None);
            ws.set_onerror(None);
            ws.set_onclose(None);
            let _ = ws.close();
        });

        Ok((
            Box::new(WebSocketClientStream {
                inbound_rx,
                outbound,
                pending_read: Bytes::new(),
                eof: false,
            }),
            None,
        ))
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) use wasm::{connect_transport, sleep_duration};

#[cfg(test)]
mod tests {
    use std::{pin::Pin, task::Poll};

    use super::coalesce::{OutboundCoalesceShared, MAX_WS_OUTBOUND_BUFFER_BYTES};
    use tokio::io::{AsyncWrite, AsyncWriteExt};

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn default_port_srv_redirect_reaches_target_and_sets_login_handshake() {
        use crate::{connection::RawConnection, session::ClientProtocolSession};
        use bbb_protocol::packets::{decode_client_intention, ClientIntent};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let direct = format!("127.0.0.1:{port}");
        for address in [
            "play.example.invalid",
            "play.example.invalid:25565",
            &direct,
        ] {
            let (stream, redirect) =
                super::connect_tcp(address, super::Duration::from_secs(1), async |host| {
                    assert_eq!(host, "play.example.invalid");
                    Some(("127.0.0.1".into(), port))
                })
                .await
                .unwrap();
            let (host, initial_port) = crate::types::split_host_port(address).unwrap();
            let conn = RawConnection::from_boxed_stream(stream, redirect, None);
            let mut session = ClientProtocolSession::new(conn);
            session
                .start_login(
                    &host,
                    initial_port,
                    ClientIntent::Login,
                    "alice",
                    uuid::Uuid::nil(),
                )
                .await
                .unwrap();
            session.conn.flush().await.unwrap();
            let mut peer = RawConnection::from_stream(listener.accept().await.unwrap().0, None);
            let (id, payload) = peer.read_packet().await.unwrap();
            let handshake = decode_client_intention(id, &payload).unwrap();
            assert_eq!(handshake.host, "127.0.0.1");
            assert_eq!(handshake.port, port);
        }
    }

    struct TestCoalesceWriter(OutboundCoalesceShared);

    impl AsyncWrite for TestCoalesceWriter {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            self.0.poll_write(cx, buf)
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            self.0.poll_flush()
        }

        fn poll_shutdown(
            self: Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            self.0.close();
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn outbound_coalescer_merges_multiple_writes_and_applies_backpressure() {
        let shared = OutboundCoalesceShared::new();
        let mut writer = TestCoalesceWriter(shared.clone());

        writer.write_all(b"frame-1|").await.unwrap();
        writer.write_all(b"frame-2|").await.unwrap();
        writer.write_all(b"frame-3").await.unwrap();

        let mut batch = Vec::new();
        assert!(shared.next_coalesced_batch(&mut batch).await);
        assert_eq!(batch, b"frame-1|frame-2|frame-3");

        // Fill up to the bounded cap and verify a further write waits until the pump drains.
        let large = vec![0xAB; MAX_WS_OUTBOUND_BUFFER_BYTES];
        writer.write_all(&large).await.unwrap();
        let pump_shared = shared.clone();
        let drain_handle = tokio::spawn(async move {
            let mut first = Vec::new();
            assert!(pump_shared.next_coalesced_batch(&mut first).await);
            assert_eq!(first.len(), MAX_WS_OUTBOUND_BUFFER_BYTES);
            let mut second = Vec::new();
            assert!(pump_shared.next_coalesced_batch(&mut second).await);
            assert_eq!(second, b"after-drain");
            assert!(!pump_shared.next_coalesced_batch(&mut second).await);
        });

        writer.write_all(b"after-drain").await.unwrap();
        writer.shutdown().await.unwrap();
        drain_handle.await.unwrap();

        let failed_shared = OutboundCoalesceShared::new();
        let mut failed_writer = TestCoalesceWriter(failed_shared.clone());
        failed_shared.fail("send failed".to_string());
        let err = failed_writer.write_all(b"x").await.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn srv_selection_orders_by_priority_and_weight_and_strips_trailing_dot() {
        let records = vec![
            (20, 100, "backup.example.com.".to_string(), 25566),
            (10, 0, "zero.example.com.".to_string(), 25567),
            (10, 10, "primary-a.example.com.".to_string(), 25568),
            (10, 30, "primary-b.example.com.".to_string(), 25569),
            (1, 50, ".".to_string(), 25570),
            (1, 50, "port-zero.example.com.".to_string(), 0),
        ];
        // Root "." and port 0 must be ignored, so priority 10 wins over priority 20.
        assert_eq!(
            super::select_srv_target(&records, 0),
            Some(("zero.example.com".to_string(), 25567))
        );
        assert_eq!(
            super::select_srv_target(&records, 1),
            Some(("primary-a.example.com".to_string(), 25568))
        );
        assert_eq!(
            super::select_srv_target(&records, 10),
            Some(("primary-a.example.com".to_string(), 25568))
        );
        assert_eq!(
            super::select_srv_target(&records, 11),
            Some(("primary-b.example.com".to_string(), 25569))
        );
        assert_eq!(
            super::select_srv_target(&records, 40),
            Some(("primary-b.example.com".to_string(), 25569))
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn connect_tcp_skips_srv_for_ip_literals() {
        let result = super::connect_tcp(
            "127.0.0.1:25565",
            super::Duration::from_millis(50),
            async |_| -> Option<(String, u16)> {
                panic!("SRV lookup must not be invoked for raw IP literals");
            },
        )
        .await;
        // Connection to closed localhost:25565 may fail, but must not panic in SRV lookup.
        let _ = result;
    }
}
