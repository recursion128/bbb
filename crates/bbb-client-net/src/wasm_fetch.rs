//! Shared browser `fetch()` helper with `AbortController` timeouts, `clearTimeout`
//! cleanup on cancellation, and streaming response size bounds.

use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{anyhow, bail, Result};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

static NEXT_ABORT_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static ABORT_CONTROLLERS: RefCell<HashMap<u64, JsValue>> = RefCell::new(HashMap::new());
}

fn remove_controller_on_current_thread(id: u64) {
    let _ = ABORT_CONTROLLERS.try_with(|controllers| {
        controllers.borrow_mut().remove(&id);
    });
}

fn abort_js_controller(controller: &JsValue) {
    if let Ok(abort_fn) = js_sys::Reflect::get(controller, &JsValue::from_str("abort")) {
        if let Ok(func) = abort_fn.dyn_into::<js_sys::Function>() {
            let _ = func.call0(controller);
        }
    }
}

struct AbortState {
    id: u64,
    aborted: AtomicBool,
}

impl Drop for AbortState {
    fn drop(&mut self) {
        remove_controller_on_current_thread(self.id);
    }
}

/// Thread-safe handle to a browser `AbortController` kept in thread-local
/// storage on the thread that created it.
#[derive(Clone)]
pub(crate) struct WasmAbortHandle {
    state: Arc<AbortState>,
}

impl WasmAbortHandle {
    pub(crate) fn new() -> Option<Self> {
        let global = js_sys::global();
        let ctor = js_sys::Reflect::get(&global, &JsValue::from_str("AbortController")).ok()?;
        let ctor_fn = ctor.dyn_into::<js_sys::Function>().ok()?;
        let controller = js_sys::Reflect::construct(&ctor_fn, &js_sys::Array::new()).ok()?;
        let id = NEXT_ABORT_ID.fetch_add(1, Ordering::Relaxed);
        ABORT_CONTROLLERS.with(|controllers| {
            controllers.borrow_mut().insert(id, controller);
        });
        Some(Self {
            state: Arc::new(AbortState {
                id,
                aborted: AtomicBool::new(false),
            }),
        })
    }

    pub(crate) fn is_aborted(&self) -> bool {
        self.state.aborted.load(Ordering::Acquire)
    }

    pub(crate) fn signal(&self) -> Option<JsValue> {
        let controller = ABORT_CONTROLLERS
            .try_with(|controllers| controllers.borrow().get(&self.state.id).cloned())
            .ok()
            .flatten()?;
        if self.is_aborted() {
            abort_js_controller(&controller);
        }
        js_sys::Reflect::get(&controller, &JsValue::from_str("signal"))
            .ok()
            .filter(|val| !val.is_undefined() && !val.is_null())
    }

    pub(crate) fn abort(&self) {
        self.state.aborted.store(true, Ordering::Release);
        let controller = ABORT_CONTROLLERS
            .try_with(|controllers| controllers.borrow().get(&self.state.id).cloned())
            .ok()
            .flatten();
        if let Some(controller) = controller {
            abort_js_controller(&controller);
        }
    }
}

struct AbortRegistryGuard(u64);

impl Drop for AbortRegistryGuard {
    fn drop(&mut self) {
        remove_controller_on_current_thread(self.0);
    }
}

struct AbortTimeoutGuard {
    timer_id: Option<JsValue>,
    _callback: Option<Closure<dyn FnMut()>>,
}

impl AbortTimeoutGuard {
    fn arm(abort: Option<&WasmAbortHandle>, timeout: Duration) -> Self {
        let Some(abort) = abort.cloned() else {
            return Self {
                timer_id: None,
                _callback: None,
            };
        };
        let millis = i32::try_from(timeout.as_millis())
            .unwrap_or(i32::MAX)
            .max(1);
        let callback = Closure::<dyn FnMut()>::new(move || {
            abort.abort();
        });
        let global = js_sys::global();
        let timer_id = js_sys::Reflect::get(&global, &JsValue::from_str("setTimeout"))
            .ok()
            .and_then(|val| val.dyn_into::<js_sys::Function>().ok())
            .and_then(|set_timeout| {
                set_timeout
                    .call2(
                        &global,
                        callback.as_ref().unchecked_ref(),
                        &JsValue::from_f64(f64::from(millis)),
                    )
                    .ok()
            });
        Self {
            timer_id,
            _callback: Some(callback),
        }
    }
}

impl Drop for AbortTimeoutGuard {
    fn drop(&mut self) {
        if let Some(timer_id) = self.timer_id.take() {
            let global = js_sys::global();
            if let Ok(clear) = js_sys::Reflect::get(&global, &JsValue::from_str("clearTimeout")) {
                if let Ok(func) = clear.dyn_into::<js_sys::Function>() {
                    let _ = func.call1(&global, &timer_id);
                }
            }
        }
    }
}

pub(crate) enum WasmFetchEvent {
    Headers { content_length: Option<u64> },
    Chunk { downloaded_bytes: u64 },
}

const MAX_INITIAL_FETCH_RESERVE_BYTES: usize = 1024 * 1024;

pub(crate) struct WasmFetchResponse {
    pub(crate) status: u16,
    pub(crate) ok: bool,
    pub(crate) retry_after: Option<String>,
    pub(crate) body: Vec<u8>,
}

/// Performs a browser `fetch` guarded by an `AbortController` timeout and a
/// strict response byte ceiling enforced both from `Content-Length` and while
/// reading the response stream.
pub(crate) async fn fetch_bounded_bytes(
    url: &str,
    label: &str,
    timeout: Duration,
    max_bytes: u64,
    external_abort: Option<WasmAbortHandle>,
    mut on_event: impl FnMut(WasmFetchEvent),
) -> Result<WasmFetchResponse> {
    let global = js_sys::global();
    let fetch_fn = js_sys::Reflect::get(&global, &JsValue::from_str("fetch"))
        .map_err(|err| anyhow!("lookup global fetch for {label}: {err:?}"))?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| anyhow!("missing global fetch function for {label}"))?;
    let abort = external_abort.or_else(WasmAbortHandle::new);
    let _registry_guard = abort.as_ref().map(|h| AbortRegistryGuard(h.state.id));
    if abort.as_ref().is_some_and(WasmAbortHandle::is_aborted) {
        if let Some(abort) = abort.as_ref() {
            abort.abort();
        }
        bail!("fetch {label} aborted");
    }
    let _timeout_guard = AbortTimeoutGuard::arm(abort.as_ref(), timeout);

    let init = web_sys::RequestInit::new();
    if let Some(signal) = abort.as_ref().and_then(WasmAbortHandle::signal) {
        let _ = js_sys::Reflect::set(&init, &JsValue::from_str("signal"), &signal);
    }

    let fetch_promise: js_sys::Promise = fetch_fn
        .call2(&global, &JsValue::from_str(url), &init)
        .map_err(|err| anyhow!("invoke fetch {label}: {err:?}"))?
        .dyn_into()
        .map_err(|_| anyhow!("fetch {label} did not return a Promise"))?;
    let resp_val = JsFuture::from(fetch_promise)
        .await
        .map_err(|err| anyhow!("fetch {label}: {err:?}"))?;
    if abort.as_ref().is_some_and(WasmAbortHandle::is_aborted) {
        if let Some(abort) = abort.as_ref() {
            abort.abort();
        }
        bail!("fetch {label} aborted");
    }
    let response: web_sys::Response = resp_val
        .dyn_into()
        .map_err(|_| anyhow!("invalid fetch Response for {label}"))?;
    let status = response.status();
    let ok = response.ok();
    let retry_after = response.headers().get("retry-after").ok().flatten();
    let content_length = response
        .headers()
        .get("content-length")
        .ok()
        .flatten()
        .and_then(|val| val.parse::<u64>().ok());
    on_event(WasmFetchEvent::Headers { content_length });

    if !ok || status == 204 || status == 404 {
        return Ok(WasmFetchResponse {
            status,
            ok,
            retry_after,
            body: Vec::new(),
        });
    }

    if let Some(declared) = content_length {
        if declared > max_bytes {
            if let Some(abort) = abort.as_ref() {
                abort.abort();
            }
            bail!(
                "{label} Content-Length is {declared} bytes, exceeding the {max_bytes} byte limit"
            );
        }
    }

    let body = read_bounded_body(
        &response,
        label,
        content_length,
        max_bytes,
        abort.as_ref(),
        &mut |bytes| {
            on_event(WasmFetchEvent::Chunk {
                downloaded_bytes: bytes,
            });
        },
    )
    .await?;
    Ok(WasmFetchResponse {
        status,
        ok,
        retry_after,
        body,
    })
}

async fn read_bounded_body(
    response: &web_sys::Response,
    label: &str,
    content_length: Option<u64>,
    max_bytes: u64,
    abort: Option<&WasmAbortHandle>,
    on_chunk: &mut impl FnMut(u64),
) -> Result<Vec<u8>> {
    let max_usize = usize::try_from(max_bytes).unwrap_or(usize::MAX);
    if let Ok(body_stream) = js_sys::Reflect::get(response, &JsValue::from_str("body")) {
        if !body_stream.is_undefined() && !body_stream.is_null() {
            if let Ok(get_reader) =
                js_sys::Reflect::get(&body_stream, &JsValue::from_str("getReader"))
            {
                if let Ok(get_reader_fn) = get_reader.dyn_into::<js_sys::Function>() {
                    if let Ok(reader) = get_reader_fn.call0(&body_stream) {
                        return read_stream_bounded(
                            &reader,
                            label,
                            content_length,
                            max_bytes,
                            max_usize,
                            abort,
                            on_chunk,
                        )
                        .await;
                    }
                }
            }
        }
    }

    let buf_val = JsFuture::from(
        response
            .array_buffer()
            .map_err(|err| anyhow!("read {label} arrayBuffer: {err:?}"))?,
    )
    .await
    .map_err(|err| anyhow!("await {label} arrayBuffer: {err:?}"))?;
    let array = js_sys::Uint8Array::new(&buf_val);
    let len = u64::from(array.length());
    if len > max_bytes {
        bail!("{label} response exceeded the {max_bytes} byte limit ({len} bytes)");
    }
    let bytes = array.to_vec();
    on_chunk(len);
    Ok(bytes)
}

async fn read_stream_bounded(
    reader: &JsValue,
    label: &str,
    content_length: Option<u64>,
    max_bytes: u64,
    max_usize: usize,
    abort: Option<&WasmAbortHandle>,
    on_chunk: &mut impl FnMut(u64),
) -> Result<Vec<u8>> {
    let read_fn = js_sys::Reflect::get(reader, &JsValue::from_str("read"))
        .map_err(|err| anyhow!("read stream getter for {label}: {err:?}"))?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| anyhow!("invalid stream reader.read for {label}"))?;
    let cancel_fn = js_sys::Reflect::get(reader, &JsValue::from_str("cancel"))
        .ok()
        .and_then(|val| val.dyn_into::<js_sys::Function>().ok());

    let initial_capacity = content_length
        .map(|len| usize::try_from(len.min(max_bytes)).unwrap_or(max_usize))
        .unwrap_or(0)
        .min(max_usize)
        .min(MAX_INITIAL_FETCH_RESERVE_BYTES);
    let mut out = Vec::new();
    out.try_reserve_exact(initial_capacity)
        .map_err(|_| anyhow!("failed to reserve {initial_capacity} initial bytes for {label}"))?;
    loop {
        if abort.is_some_and(WasmAbortHandle::is_aborted) {
            if let Some(cancel) = cancel_fn.as_ref() {
                let _ = cancel.call0(reader);
            }
            if let Some(abort) = abort {
                abort.abort();
            }
            bail!("fetch {label} aborted");
        }
        let promise_val = read_fn
            .call0(reader)
            .map_err(|err| anyhow!("call reader.read for {label}: {err:?}"))?;
        let promise = promise_val
            .dyn_into::<js_sys::Promise>()
            .map_err(|_| anyhow!("reader.read did not return Promise for {label}"))?;
        let chunk_obj = JsFuture::from(promise)
            .await
            .map_err(|err| anyhow!("await stream chunk for {label}: {err:?}"))?;
        let done = js_sys::Reflect::get(&chunk_obj, &JsValue::from_str("done"))
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if done {
            break;
        }
        let value = js_sys::Reflect::get(&chunk_obj, &JsValue::from_str("value"))
            .map_err(|err| anyhow!("read chunk value for {label}: {err:?}"))?;
        if value.is_undefined() || value.is_null() {
            continue;
        }
        let chunk = js_sys::Uint8Array::new(&value);
        let chunk_len = chunk.length() as usize;
        let next_len = out
            .len()
            .checked_add(chunk_len)
            .ok_or_else(|| anyhow!("{label} response byte count overflow"))?;
        if next_len > max_usize || next_len as u64 > max_bytes {
            if let Some(cancel) = cancel_fn.as_ref() {
                let _ = cancel.call0(reader);
            }
            if let Some(abort) = abort {
                abort.abort();
            }
            bail!("{label} response exceeded the {max_bytes} byte limit");
        }
        if next_len > out.capacity() {
            let target_cap = out
                .capacity()
                .saturating_mul(2)
                .max(next_len)
                .min(max_usize);
            out.try_reserve_exact(target_cap.saturating_sub(out.len()))
                .map_err(|_| anyhow!("failed to reserve {target_cap} bytes for {label}"))?;
        }
        let start = out.len();
        out.resize(next_len, 0);
        chunk.copy_to(&mut out[start..]);
        on_chunk(out.len() as u64);
    }
    Ok(out)
}
