//! Browser WASM + WebGPU entry point for `bbb-client`.
#![deny(dead_code)]

pub(crate) mod ime;
pub(crate) mod locales;
#[cfg(target_arch = "wasm32")]
pub(crate) mod net_worker;
pub(crate) mod packs;
#[cfg(target_arch = "wasm32")]
mod speech;

#[cfg(target_arch = "wasm32")]
pub use packs::open_web_resource_pack_picker;

#[cfg(target_arch = "wasm32")]
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

#[cfg(target_arch = "wasm32")]
use anyhow::{Context, Result};
#[cfg(target_arch = "wasm32")]
use bbb_client_assets::PackRoots;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{closure::Closure, prelude::*, JsCast};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen_rayon::init_thread_pool;
#[cfg(target_arch = "wasm32")]
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, DeviceId, ElementState, Event, StartCause, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys},
    window::{CursorGrabMode, Window, WindowId},
};

#[cfg(target_arch = "wasm32")]
use crate::{
    audio_runtime::{AudioEventSink, NativeAudioRuntime},
    code_of_conduct::{default_code_of_conduct_store_path, CodeOfConductAcceptance},
    control::{AudioCounters, ControlState, SharedControlRequests, WebControlTunnel},
    event_loop_shared::{drive_session_event, PlatformEventLoopHooks, SessionEventLoopContext},
    input::DebugClipboard,
    narrator_runtime::SystemNarrator,
    network_runtime::NetworkRuntime,
    options::ClientOptionHost,
    resource_pack_runtime::ServerResourcePackRuntime,
    resource_reload::{prepare_server_resource_pack_reload, PreparedServerResourcePackReload},
    server_list::{most_recent_saved_web_server_address, ServerListPersistence},
    session_shared::{self, LoadedCoreClientAssets, SessionCoreInputs},
    simulation::{
        commands::{command_queue, LoopWake, SimulationCommand},
        effects::ResourceReloadEffect,
        StepOutcome,
    },
    sound_subtitle_runtime::SoundSubtitleRuntime,
    startup::{
        self, build_web_args_from_config_and_query, create_event_loop, load_known_packs_manager,
        start_network_web_if_requested, WebBootstrapConfig,
    },
};

#[cfg(target_arch = "wasm32")]
fn browser_offline_username(window: &web_sys::Window) -> Result<String> {
    const KEY: &str = "bbb.offline-player";
    let storage = window.local_storage().ok().flatten();
    if let Some(name) = storage
        .as_ref()
        .and_then(|store| store.get_item(KEY).ok().flatten())
    {
        return Ok(name);
    }
    let crypto = window
        .crypto()
        .map_err(|error| anyhow::anyhow!("browser crypto: {error:?}"))?;
    let uuid = crypto.random_uuid();
    let (_, suffix) = uuid.rsplit_once('-').expect("browser returns a UUID");
    let name = format!("BBB_{suffix}");
    if let Some(store) = storage {
        if let Err(error) = store.set_item(KEY, &name) {
            tracing::warn!(?error, "browser could not persist the offline player name");
        }
    }
    Ok(name)
}

#[cfg(target_arch = "wasm32")]
const CORE_ASSETS_CACHE_NAME: &str = "bbb-core-assets-v1";
#[cfg(target_arch = "wasm32")]
const CORE_BUNDLE_URL: &str = "/assets/core-26.1.zip";
#[cfg(target_arch = "wasm32")]
const ASSET_INDEX_URL: &str = "/assets/asset-index.json";
#[cfg(target_arch = "wasm32")]
const CONFIG_URL: &str = "/config.json";

#[cfg(target_arch = "wasm32")]
struct WebConsoleSubscriber;

#[cfg(target_arch = "wasm32")]
impl tracing::Subscriber for WebConsoleSubscriber {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        *metadata.level() <= tracing::Level::INFO
    }

    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        struct MessageVisitor(String);
        impl tracing::field::Visit for MessageVisitor {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if !self.0.is_empty() {
                    self.0.push(' ');
                }
                if field.name() == "message" {
                    self.0.push_str(&format!("{value:?}"));
                } else {
                    self.0.push_str(&format!("{}={value:?}", field.name()));
                }
            }
        }
        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);
        let formatted = format!("[{}] {}", event.metadata().target(), visitor.0);
        let js_msg = JsValue::from_str(&formatted);
        match *event.metadata().level() {
            tracing::Level::ERROR => web_sys::console::error_1(&js_msg),
            tracing::Level::WARN => web_sys::console::warn_1(&js_msg),
            _ => web_sys::console::log_1(&js_msg),
        }
    }

    fn enter(&self, _span: &tracing::span::Id) {}

    fn exit(&self, _span: &tracing::span::Id) {}
}

#[cfg(target_arch = "wasm32")]
static FATAL_ERROR_LATCHED: AtomicBool = AtomicBool::new(false);

#[cfg(target_arch = "wasm32")]
thread_local! {
    static DOM_POINTER_WAS_LOCKED: Cell<bool> = const { Cell::new(false) };
    static EXTERNAL_POINTER_UNLOCKED: Cell<bool> = const { Cell::new(false) };
    static POINTER_LOCK_ACQUIRED: Cell<bool> = const { Cell::new(false) };
    static POINTER_LOCK_FAILED: Cell<bool> = const { Cell::new(false) };
    static WEB_SHORTCUT_MODIFIER_ACTIVE: Cell<bool> = const { Cell::new(false) };
    static WEB_PASTE_DEFERRED: Cell<bool> = const { Cell::new(false) };
    static WEB_PASTE_HANDLED_BY_DOM: Cell<bool> = const { Cell::new(false) };
    static PENDING_PASTE_TEXT: RefCell<Option<String>> = const { RefCell::new(None) };
    static CACHED_CLIPBOARD_TEXT: RefCell<Option<String>> = const { RefCell::new(None) };
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn notify_intentional_pointer_unlock() {
    DOM_POINTER_WAS_LOCKED.with(|c| c.set(false));
    EXTERNAL_POINTER_UNLOCKED.with(|c| c.set(false));
    POINTER_LOCK_ACQUIRED.with(|c| c.set(false));
    POINTER_LOCK_FAILED.with(|c| c.set(false));
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn shortcut_modifier_active() -> bool {
    WEB_SHORTCUT_MODIFIER_ACTIVE.with(Cell::get)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn paste_deferred_to_dom_event() -> bool {
    WEB_PASTE_DEFERRED.with(Cell::get)
}

#[cfg(target_arch = "wasm32")]
pub(super) fn set_shortcut_modifier_state(is_shortcut: bool, is_paste: bool) {
    WEB_SHORTCUT_MODIFIER_ACTIVE.with(|c| c.set(is_shortcut));
    WEB_PASTE_DEFERRED.with(|c| c.set(is_paste));
    if is_paste {
        WEB_PASTE_HANDLED_BY_DOM.with(|c| c.set(false));
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn on_dom_keyup_paste_check(code: &str, shortcut_active: bool, wake: &LoopWake) {
    WEB_SHORTCUT_MODIFIER_ACTIVE.with(|c| c.set(shortcut_active));
    if code == "KeyV" {
        let was_deferred = WEB_PASTE_DEFERRED.with(|c| c.replace(false));
        let handled = WEB_PASTE_HANDLED_BY_DOM.with(|c| c.replace(false));
        if was_deferred && !handled {
            if let Some(cached) =
                CACHED_CLIPBOARD_TEXT.with(|slot| slot.borrow().clone().filter(|s| !s.is_empty()))
            {
                PENDING_PASTE_TEXT.with(|slot| *slot.borrow_mut() = Some(cached));
                wake.wake();
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn dom_has_pointer_lock() -> bool {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.pointer_lock_element())
        .is_some()
}

#[cfg(target_arch = "wasm32")]
pub(super) fn document_has_focus() -> bool {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return true;
    };
    js_sys::Reflect::get(&document, &JsValue::from_str("hasFocus"))
        .ok()
        .and_then(|val| val.dyn_into::<js_sys::Function>().ok())
        .and_then(|func| func.call0(&document).ok())
        .and_then(|val| val.as_bool())
        .unwrap_or(true)
}

#[cfg(target_arch = "wasm32")]
pub(super) fn js_bool_prop(target: &JsValue, key: &str) -> bool {
    js_sys::Reflect::get(target, &JsValue::from_str(key))
        .ok()
        .and_then(|val| val.as_bool())
        .unwrap_or(false)
}

#[cfg(target_arch = "wasm32")]
pub(super) fn js_string_prop(target: &JsValue, key: &str) -> Option<String> {
    js_sys::Reflect::get(target, &JsValue::from_str(key))
        .ok()
        .and_then(|val| val.as_string())
}

#[cfg(target_arch = "wasm32")]
fn extract_paste_event_text(ev: &web_sys::Event) -> Option<String> {
    let clipboard_data = js_sys::Reflect::get(ev, &JsValue::from_str("clipboardData")).ok()?;
    if clipboard_data.is_null() || clipboard_data.is_undefined() {
        return None;
    }
    let get_data = js_sys::Reflect::get(&clipboard_data, &JsValue::from_str("getData"))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    get_data
        .call1(&clipboard_data, &JsValue::from_str("text/plain"))
        .ok()?
        .as_string()
}

#[cfg(target_arch = "wasm32")]
fn install_web_diagnostics() {
    let _ = tracing::subscriber::set_global_default(WebConsoleSubscriber);
    std::panic::set_hook(Box::new(|info| {
        let stack = js_sys::Reflect::get(&js_sys::Error::new(""), &JsValue::from_str("stack"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        let message = format!("BBB Web client panicked: {info}");
        if stack.is_empty() {
            web_sys::console::error_1(&JsValue::from_str(&message));
        } else {
            web_sys::console::error_1(&JsValue::from_str(&format!("{message}\n{stack}")));
        }
        set_dom_status(&message, true);
    }));
}

#[cfg(target_arch = "wasm32")]
fn set_dom_status(message: &str, is_error: bool) {
    if is_error {
        if FATAL_ERROR_LATCHED.swap(true, Ordering::AcqRel) {
            return;
        }
    } else if FATAL_ERROR_LATCHED.load(Ordering::Acquire) {
        return;
    }
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    if is_error {
        let _ = js_sys::Reflect::set(
            &window,
            &JsValue::from_str("__bbb_fatal_latched"),
            &JsValue::TRUE,
        );
    }
    if let Some(status) = document.get_element_by_id("bbb-status") {
        status.set_text_content(Some(message));
        if is_error {
            if let Ok(html) = status.dyn_into::<web_sys::HtmlElement>() {
                let _ = html.style().set_property("color", "#ff6b6b");
            }
        }
    }
    if is_error {
        if let Some(overlay) = document.get_element_by_id("bbb-loading") {
            if let Ok(html) = overlay.dyn_into::<web_sys::HtmlElement>() {
                let _ = html.style().set_property("display", "flex");
                let _ = html.style().set_property("pointer-events", "auto");
            }
        }
        if let Some(actions) = document.get_element_by_id("bbb-error-actions") {
            let _ = actions.remove_attribute("hidden");
        }
        if let Some(dismiss) = document.get_element_by_id("bbb-dismiss-btn") {
            let _ = dismiss.set_attribute("hidden", "");
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn hide_dom_loading_overlay() {
    if FATAL_ERROR_LATCHED.load(Ordering::Acquire) {
        return;
    }
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let _ = js_sys::Reflect::set(&window, &JsValue::from_str("__bbb_booted"), &JsValue::TRUE);
    if let Some(status) = document.get_element_by_id("bbb-status") {
        status.set_text_content(Some(""));
    }
    if let Some(actions) = document.get_element_by_id("bbb-error-actions") {
        let _ = actions.set_attribute("hidden", "");
    }
    if let Some(overlay) = document.get_element_by_id("bbb-loading") {
        if let Ok(html) = overlay.dyn_into::<web_sys::HtmlElement>() {
            let _ = html.style().set_property("display", "none");
            let _ = html.style().set_property("pointer-events", "none");
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn sleep_ms(window: &web_sys::Window, millis: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis);
    });
    let _ = JsFuture::from(promise).await;
}

#[cfg(target_arch = "wasm32")]
async fn yield_to_browser_paint(window: &web_sys::Window) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let _ = window.request_animation_frame(&resolve);
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 32);
    });
    let _ = JsFuture::from(promise).await;
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn reconnect_to_server(server: &str) {
    let reconnect = || {
        let location = web_sys::window()
            .expect("client runs in the browser Window")
            .location();
        let url = web_sys::Url::new(&location.href()?)?;
        let params = url.search_params();
        if params.get("server").as_deref() == Some(server) {
            location.reload()
        } else {
            params.set("server", server);
            location.replace(&url.href())
        }
    };
    if let Err(error) = reconnect() {
        tracing::warn!(?error, "failed to reload Web client for reconnection");
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Default)]
struct WebDebugClipboard;

#[cfg(target_arch = "wasm32")]
impl WebDebugClipboard {
    fn refresh_from_browser(&self) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let clipboard = window.navigator().clipboard();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(val) = JsFuture::from(clipboard.read_text()).await {
                if let Some(text) = val.as_string() {
                    CACHED_CLIPBOARD_TEXT.with(|slot| *slot.borrow_mut() = Some(text));
                }
            }
        });
    }
}

#[cfg(target_arch = "wasm32")]
impl DebugClipboard for WebDebugClipboard {
    fn set_debug_clipboard_text(&mut self, text: &str) -> bool {
        CACHED_CLIPBOARD_TEXT.with(|slot| *slot.borrow_mut() = Some(text.to_string()));
        if let Some(window) = web_sys::window() {
            let promise = window.navigator().clipboard().write_text(text);
            wasm_bindgen_futures::spawn_local(async move {
                let _ = JsFuture::from(promise).await;
            });
        }
        true
    }

    fn get_debug_clipboard_text(&mut self) -> Option<String> {
        CACHED_CLIPBOARD_TEXT.with(|slot| slot.borrow().clone())
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn fetch_response(
    window: &web_sys::Window,
    url: &str,
) -> Result<web_sys::Response> {
    let resp_val = JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|err| anyhow::anyhow!("fetch {url} failed: {err:?}"))?;
    let resp: web_sys::Response = resp_val
        .dyn_into()
        .map_err(|_| anyhow::anyhow!("fetch {url}: non-Response value"))?;
    if !resp.ok() {
        anyhow::bail!("fetch {url}: HTTP {}", resp.status());
    }
    Ok(resp)
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn response_bytes(resp: &web_sys::Response, url: &str) -> Result<Vec<u8>> {
    let buf_promise = resp
        .array_buffer()
        .map_err(|err| anyhow::anyhow!("read body {url}: {err:?}"))?;
    let buf = JsFuture::from(buf_promise)
        .await
        .map_err(|err| anyhow::anyhow!("await body {url}: {err:?}"))?;
    Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

#[cfg(target_arch = "wasm32")]
async fn response_text(resp: &web_sys::Response, url: &str) -> Result<String> {
    let text_promise = resp
        .text()
        .map_err(|err| anyhow::anyhow!("read text {url}: {err:?}"))?;
    let val = JsFuture::from(text_promise)
        .await
        .map_err(|err| anyhow::anyhow!("await text {url}: {err:?}"))?;
    val.as_string()
        .ok_or_else(|| anyhow::anyhow!("non-string body from {url}"))
}

#[cfg(target_arch = "wasm32")]
async fn probe_core_bundle_cache(
    window: &web_sys::Window,
) -> (Option<web_sys::Cache>, Option<web_sys::Response>) {
    let cache: Option<web_sys::Cache> = match window.caches() {
        Ok(storage) => JsFuture::from(storage.open(CORE_ASSETS_CACHE_NAME))
            .await
            .ok()
            .and_then(|val| val.dyn_into::<web_sys::Cache>().ok()),
        Err(_) => None,
    };
    let matched = match &cache {
        Some(cache_ref) => JsFuture::from(cache_ref.match_with_str(CORE_BUNDLE_URL))
            .await
            .ok()
            .filter(|v| !v.is_undefined() && !v.is_null())
            .and_then(|v| v.dyn_into::<web_sys::Response>().ok()),
        None => None,
    };
    (cache, matched)
}

#[cfg(any(target_arch = "wasm32", test))]
fn format_core_bundle_progress(received_bytes: u64, total_bytes: Option<u64>) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    let received_mib = received_bytes as f64 / MIB;
    match total_bytes.filter(|&total| total > 0) {
        Some(total) => {
            let total_mib = total as f64 / MIB;
            let pct = ((received_bytes.saturating_mul(100)) / total).min(100);
            format!(
                "Downloading Minecraft 26.1 core assets ({received_mib:.1} / {total_mib:.1} MiB, {pct}%)..."
            )
        }
        None => format!("Downloading Minecraft 26.1 core assets ({received_mib:.1} MiB)..."),
    }
}

#[cfg(target_arch = "wasm32")]
async fn stream_response_bytes_with_progress(
    resp: &web_sys::Response,
    url: &str,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<Vec<u8>> {
    let total_bytes = resp
        .headers()
        .get("content-length")
        .ok()
        .flatten()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|&len| len > 0);
    on_progress(0, total_bytes);
    let Some(stream) = resp.body() else {
        let bytes = response_bytes(resp, url).await?;
        let total = u64::try_from(bytes.len()).ok();
        on_progress(bytes.len() as u64, total_bytes.or(total));
        return Ok(bytes);
    };
    let reader = stream
        .get_reader()
        .unchecked_into::<web_sys::ReadableStreamDefaultReader>();
    let initial_cap = total_bytes
        .and_then(|len| usize::try_from(len).ok())
        .unwrap_or(0)
        .min(64 * 1024 * 1024);
    let mut bytes = Vec::with_capacity(initial_cap);
    let mut last_reported_bytes = 0u64;
    let mut last_reported_pct = total_bytes.map(|_| 0u64);
    loop {
        let next = JsFuture::from(reader.read())
            .await
            .map_err(|err| anyhow::anyhow!("read stream {url}: {err:?}"))?
            .unchecked_into::<js_sys::IteratorNext>();
        if next.done() {
            let len = bytes.len() as u64;
            on_progress(len, total_bytes.or(Some(len)));
            return Ok(bytes);
        }
        let chunk = next.value().unchecked_into::<js_sys::Uint8Array>();
        let chunk_len = usize::try_from(chunk.length()).unwrap_or(0);
        if chunk_len > 0 {
            let start = bytes.len();
            bytes.resize(start + chunk_len, 0);
            chunk.copy_to(&mut bytes[start..]);
            let received = bytes.len() as u64;
            let pct = total_bytes.map(|total| ((received.saturating_mul(100)) / total).min(100));
            if received.saturating_sub(last_reported_bytes) >= 262_144 || pct != last_reported_pct {
                last_reported_bytes = received;
                last_reported_pct = pct;
                on_progress(received, total_bytes);
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn fetch_core_bundle_cached(
    window: &web_sys::Window,
    probed: (Option<web_sys::Cache>, Option<web_sys::Response>),
    expected_etag: Option<&str>,
) -> Result<Vec<u8>> {
    let (cache, matched) = probed;
    if let Some(cached_resp) = matched {
        let cached_etag = cached_resp.headers().get("etag").ok().flatten();
        let etag_matches = match (expected_etag, cached_etag.as_deref()) {
            (Some(exp), Some(got)) => exp.trim_matches('"') == got.trim_matches('"'),
            (None, _) => true,
            (Some(_), None) => false,
        };
        if etag_matches {
            if let Ok(bytes) = response_bytes(&cached_resp, CORE_BUNDLE_URL).await {
                return Ok(bytes);
            }
        }
    }

    let resp = fetch_response(window, CORE_BUNDLE_URL).await?;
    if let Some(cache_ref) = cache {
        if let Ok(cloned) = resp.clone() {
            wasm_bindgen_futures::spawn_local(async move {
                let _ = JsFuture::from(cache_ref.put_with_str(CORE_BUNDLE_URL, &cloned)).await;
            });
        }
    }
    stream_response_bytes_with_progress(&resp, CORE_BUNDLE_URL, |received, total| {
        set_dom_status(&format_core_bundle_progress(received, total), false);
    })
    .await
}

#[cfg(target_arch = "wasm32")]
fn websocket_base_origin(window: &web_sys::Window) -> Result<(String, String)> {
    let location = window.location();
    let protocol = location
        .protocol()
        .map_err(|err| anyhow::anyhow!("read location.protocol: {err:?}"))?;
    let host = location
        .host()
        .map_err(|err| anyhow::anyhow!("read location.host: {err:?}"))?;
    let ws_scheme = if protocol == "https:" { "wss" } else { "ws" };
    Ok((ws_scheme.to_string(), host))
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn trigger_browser_blob_download(name: &str, mime_type: &str, bytes: &[u8]) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let u8_array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::of1(&u8_array);
    let bag = web_sys::BlobPropertyBag::new();
    bag.set_type(mime_type);
    let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &bag) else {
        return;
    };
    let Ok(object_url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return;
    };
    if let Ok(el) = document.create_element("a") {
        if let Ok(anchor) = el.dyn_into::<web_sys::HtmlAnchorElement>() {
            anchor.set_href(&object_url);
            anchor.set_download(if name.is_empty() {
                "download.bin"
            } else {
                name
            });
            anchor.click();
        }
    }
    // Defer object URL revocation so Firefox/Safari finish initiating the download.
    wasm_bindgen_futures::spawn_local(async move {
        sleep_ms(&window, 1000).await;
        let _ = web_sys::Url::revoke_object_url(&object_url);
    });
}

#[cfg(target_arch = "wasm32")]
fn trigger_browser_png_download(name: &str, png_bytes: &[u8]) {
    let file_name = if name.is_empty() {
        "screenshot.png"
    } else {
        name
    };
    trigger_browser_blob_download(file_name, "image/png", png_bytes);
}

#[cfg(target_arch = "wasm32")]
type WebEventHandler = Box<dyn FnMut(Event<()>, &ActiveEventLoop)>;

#[cfg(target_arch = "wasm32")]
struct WebApplication {
    bootstrap: Option<Box<dyn FnOnce(&ActiveEventLoop, Rc<RefCell<Option<WebEventHandler>>>)>>,
    handler: Rc<RefCell<Option<WebEventHandler>>>,
}

#[cfg(target_arch = "wasm32")]
impl ApplicationHandler<()> for WebApplication {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if FATAL_ERROR_LATCHED.load(Ordering::Relaxed) {
            event_loop.exit();
            return;
        }
        if let Ok(mut slot) = self.handler.try_borrow_mut() {
            if let Some(handler) = slot.as_mut() {
                handler(Event::NewEvents(cause), event_loop);
            }
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(bootstrap) = self.bootstrap.take() {
            bootstrap(event_loop, Rc::clone(&self.handler));
        }
        if let Ok(mut slot) = self.handler.try_borrow_mut() {
            if let Some(handler) = slot.as_mut() {
                handler(Event::Resumed, event_loop);
            }
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ()) {
        if FATAL_ERROR_LATCHED.load(Ordering::Relaxed) {
            event_loop.exit();
            return;
        }
        if let Ok(mut slot) = self.handler.try_borrow_mut() {
            if let Some(handler) = slot.as_mut() {
                handler(Event::UserEvent(event), event_loop);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if FATAL_ERROR_LATCHED.load(Ordering::Relaxed) {
            event_loop.exit();
            return;
        }
        if let Ok(mut slot) = self.handler.try_borrow_mut() {
            if let Some(handler) = slot.as_mut() {
                handler(Event::WindowEvent { window_id, event }, event_loop);
            }
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if FATAL_ERROR_LATCHED.load(Ordering::Relaxed) {
            event_loop.exit();
            return;
        }
        if let Ok(mut slot) = self.handler.try_borrow_mut() {
            if let Some(handler) = slot.as_mut() {
                handler(Event::DeviceEvent { device_id, event }, event_loop);
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if FATAL_ERROR_LATCHED.load(Ordering::Relaxed) {
            event_loop.exit();
            return;
        }
        if let Ok(mut slot) = self.handler.try_borrow_mut() {
            if let Some(handler) = slot.as_mut() {
                handler(Event::AboutToWait, event_loop);
            }
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        if let Ok(mut slot) = self.handler.try_borrow_mut() {
            if let Some(handler) = slot.as_mut() {
                handler(Event::Suspended, event_loop);
            }
        }
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        if let Ok(mut slot) = self.handler.try_borrow_mut() {
            if let Some(mut handler) = slot.take() {
                handler(Event::LoopExiting, event_loop);
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn web_worker_init_payload() -> js_sys::Array {
    let arr = js_sys::Array::new();
    arr.push(&wasm_bindgen::module());
    arr.push(&wasm_bindgen::memory());
    arr
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run_web_worker_relay() {
    crate::terrain_upload::wasm_dispatch::run_wasm_worker_relay_loop();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn init_web_net_worker() {
    net_worker::init_worker_scope();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn start_web_client() -> std::result::Result<(), JsValue> {
    install_web_diagnostics();
    run_web_client_inner().await.map_err(|err| {
        let message = format!("{err:#}");
        tracing::error!(error = %message, "BBB Web client startup failed");
        set_dom_status(&format!("Startup error: {message}"), true);
        JsValue::from_str(&message)
    })
}

#[cfg(target_arch = "wasm32")]
async fn run_web_client_inner() -> Result<()> {
    let browser_window = web_sys::window().context("missing browser Window")?;
    let document = browser_window
        .document()
        .context("missing browser Document")?;
    let canvas_el = document
        .get_element_by_id("bbb-canvas")
        .context("missing #bbb-canvas element")?;
    let canvas: web_sys::HtmlCanvasElement = canvas_el
        .dyn_into()
        .map_err(|_| anyhow::anyhow!("#bbb-canvas is not an HtmlCanvasElement"))?;

    // Kick off independent startup I/O (CacheStorage lookup, asset-index fetch,
    // and dedicated Network Web Worker initialization) immediately alongside `/config.json`.
    let (cache_probe_tx, cache_probe_rx) = tokio::sync::oneshot::channel();
    {
        let win = browser_window.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let _ = cache_probe_tx.send(probe_core_bundle_cache(&win).await);
        });
    }
    let (index_bytes_tx, index_bytes_rx) = tokio::sync::oneshot::channel();
    {
        let win = browser_window.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let res = match fetch_response(&win, ASSET_INDEX_URL).await {
                Ok(resp) => response_bytes(&resp, ASSET_INDEX_URL).await,
                Err(err) => Err(err),
            };
            let _ = index_bytes_tx.send(res);
        });
    }
    let (net_worker_tx, net_worker_rx) = tokio::sync::oneshot::channel();
    {
        let win = browser_window.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let _ = net_worker_tx.send(net_worker::prewarm_net_worker(&win).await);
        });
    }

    set_dom_status("Loading gateway configuration...", false);
    let mut config_loaded = false;
    let mut config: WebBootstrapConfig = match fetch_response(&browser_window, CONFIG_URL).await {
        Ok(resp) => {
            let text = response_text(&resp, CONFIG_URL).await?;
            config_loaded = true;
            serde_json::from_str(&text).context("parse /config.json")?
        }
        Err(err) => {
            tracing::warn!(%err, "continuing with default WebBootstrapConfig");
            WebBootstrapConfig::default()
        }
    };
    if !config_loaded {
        if let Some(saved_server) = most_recent_saved_web_server_address() {
            config.server = saved_server;
        }
    }

    let search = browser_window.location().search().unwrap_or_default();
    let browser_locale = browser_window.navigator().language();
    let option_host = ClientOptionHost {
        locale_is_japanese: crate::options::locale_is_japanese(browser_locale.as_deref()),
        platform_is_windows: crate::platform::HOST_IS_WINDOWS,
    };
    let (mut args, parsed_bootstrap, client_options) =
        build_web_args_from_config_and_query(&config, &search, option_host)?;
    if args.username.is_empty() {
        args.username = browser_offline_username(&browser_window)?;
    }
    if parsed_bootstrap.software_gpu {
        bbb_client_render::set_force_software_adapter(true);
    }
    if parsed_bootstrap.lockstep {
        bbb_client_clock::enable_virtual();
    }
    let control_enabled = parsed_bootstrap.control_enabled;

    let (ws_scheme, host) = websocket_base_origin(&browser_window)?;
    let ws_gateway_url = format!("{ws_scheme}://{host}/ws");
    let control_ws_url = format!("{ws_scheme}://{host}/control-ws");

    let event_loop = create_event_loop()?;
    let loop_wake = Arc::new(LoopWake::new(event_loop.create_proxy()));
    let (command_queue, command_inbox) = command_queue(Arc::clone(&loop_wake));
    let version_banner = format!(
        "bbb-client-web {} / protocol {}",
        bbb_protocol::MC_VERSION,
        bbb_protocol::PROTOCOL_VERSION
    );
    let control = ControlState::new(
        version_banner.clone(),
        SharedControlRequests::new(Arc::new(command_queue.clone())),
    );
    let snapshot = control.snapshot.clone();
    let publication_demand = control.publication.clone();

    let web_control_tunnel = if control_enabled {
        match WebControlTunnel::connect(&control_ws_url, control.clone(), version_banner) {
            Ok(tunnel) => Some(tunnel),
            Err(err) => {
                tracing::warn!(?err, "failed to open reverse control tunnel");
                None
            }
        }
    } else {
        None
    };

    let startup_pack_urls = parsed_bootstrap.pack_urls;
    let app = WebApplication {
        handler: Rc::new(RefCell::new(None)),
        bootstrap: Some(Box::new(move |target, handler_slot| {
            let dpr = browser_window.device_pixel_ratio().max(1.0);
            let css_w = browser_window
                .inner_width()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(1280.0);
            let css_h = browser_window
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(720.0);
            let init_w = ((css_w * dpr).round() as u32).max(1);
            let init_h = ((css_h * dpr).round() as u32).max(1);
            canvas.set_width(init_w);
            canvas.set_height(init_h);

            let attributes = Window::default_attributes()
                .with_title("bbb-client")
                .with_canvas(Some(canvas.clone()))
                .with_prevent_default(false);
            let window = match target.create_window(attributes) {
                Ok(window) => window,
                Err(err) => {
                    set_dom_status(&format!("Failed to create canvas window: {err}"), true);
                    return;
                }
            };
            window.set_ime_allowed(false);

            ime::install_web_ime_bridge(
                &document,
                &canvas,
                command_queue.clone(),
                Arc::clone(&loop_wake),
            );
            let debug_clipboard = WebDebugClipboard;
            let focus_clipboard = debug_clipboard.clone();
            let on_focus = Closure::<dyn FnMut()>::new(move || {
                focus_clipboard.refresh_from_browser();
            });
            let _ = browser_window
                .add_event_listener_with_callback("focus", on_focus.as_ref().unchecked_ref());
            on_focus.forget();

            let on_blur = Closure::<dyn FnMut()>::new(move || {
                WEB_SHORTCUT_MODIFIER_ACTIVE.with(|c| c.set(false));
                WEB_PASTE_DEFERRED.with(|c| c.set(false));
                WEB_PASTE_HANDLED_BY_DOM.with(|c| c.set(false));
            });
            let _ = browser_window
                .add_event_listener_with_callback("blur", on_blur.as_ref().unchecked_ref());
            on_blur.forget();

            let on_canvas_keydown = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
                move |ev: web_sys::KeyboardEvent| {
                    let ctrl = ev.ctrl_key();
                    let meta = ev.meta_key();
                    let alt = ev.alt_key();
                    let shift = ev.shift_key();
                    let key = ev.key();
                    let code = ev.code();
                    if ime::is_browser_devtools_shortcut(
                        Some(key.as_str()),
                        Some(code.as_str()),
                        ctrl,
                        shift,
                        alt,
                        meta,
                    ) {
                        return;
                    }
                    let is_shortcut = (ctrl || meta) && !alt;
                    let is_paste = is_shortcut && !shift && code == "KeyV";
                    set_shortcut_modifier_state(is_shortcut, is_paste);
                    if !is_paste {
                        ev.prevent_default();
                    }
                },
            );
            let _ = canvas.add_event_listener_with_callback_and_bool(
                "keydown",
                on_canvas_keydown.as_ref().unchecked_ref(),
                true,
            );
            on_canvas_keydown.forget();

            let keyup_wake = Arc::clone(&loop_wake);
            let on_canvas_keyup = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
                move |ev: web_sys::KeyboardEvent| {
                    let ctrl = ev.ctrl_key();
                    let meta = ev.meta_key();
                    let alt = ev.alt_key();
                    let shift = ev.shift_key();
                    let key = ev.key();
                    let code = ev.code();
                    if ime::is_browser_devtools_shortcut(
                        Some(key.as_str()),
                        Some(code.as_str()),
                        ctrl,
                        shift,
                        alt,
                        meta,
                    ) {
                        return;
                    }
                    on_dom_keyup_paste_check(&code, (ctrl || meta) && !alt, &keyup_wake);
                    ev.prevent_default();
                },
            );
            let _ = canvas.add_event_listener_with_callback_and_bool(
                "keyup",
                on_canvas_keyup.as_ref().unchecked_ref(),
                true,
            );
            on_canvas_keyup.forget();

            let paste_wake = Arc::clone(&loop_wake);
            let on_paste = Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
                ev.prevent_default();
                WEB_PASTE_HANDLED_BY_DOM.with(|c| c.set(true));
                let text = extract_paste_event_text(&ev)
                    .or_else(|| CACHED_CLIPBOARD_TEXT.with(|slot| slot.borrow().clone()));
                if let Some(text) = text.filter(|s| !s.is_empty()) {
                    CACHED_CLIPBOARD_TEXT.with(|slot| *slot.borrow_mut() = Some(text.clone()));
                    PENDING_PASTE_TEXT.with(|slot| *slot.borrow_mut() = Some(text));
                    paste_wake.wake();
                }
            });
            let _ = document
                .add_event_listener_with_callback("paste", on_paste.as_ref().unchecked_ref());
            on_paste.forget();

            DOM_POINTER_WAS_LOCKED.with(|c| c.set(dom_has_pointer_lock()));
            let pointer_lock_wake = Arc::clone(&loop_wake);
            let on_pointer_lock_change = Closure::<dyn FnMut()>::new(move || {
                if dom_has_pointer_lock() {
                    DOM_POINTER_WAS_LOCKED.with(|c| c.set(true));
                    EXTERNAL_POINTER_UNLOCKED.with(|c| c.set(false));
                    POINTER_LOCK_ACQUIRED.with(|c| c.set(true));
                    POINTER_LOCK_FAILED.with(|c| c.set(false));
                    pointer_lock_wake.wake();
                } else if DOM_POINTER_WAS_LOCKED.with(|c| c.replace(false)) {
                    EXTERNAL_POINTER_UNLOCKED.with(|c| c.set(true));
                    POINTER_LOCK_ACQUIRED.with(|c| c.set(false));
                    pointer_lock_wake.wake();
                }
            });
            let _ = document.add_event_listener_with_callback(
                "pointerlockchange",
                on_pointer_lock_change.as_ref().unchecked_ref(),
            );
            on_pointer_lock_change.forget();

            let pointer_lock_fail_wake = Arc::clone(&loop_wake);
            let on_pointer_lock_error = Closure::<dyn FnMut()>::new(move || {
                if !dom_has_pointer_lock() {
                    POINTER_LOCK_ACQUIRED.with(|c| c.set(false));
                    POINTER_LOCK_FAILED.with(|c| c.set(true));
                    pointer_lock_fail_wake.wake();
                }
            });
            let _ = document.add_event_listener_with_callback(
                "pointerlockerror",
                on_pointer_lock_error.as_ref().unchecked_ref(),
            );
            let _ = document.add_event_listener_with_callback(
                "bbb-pointerlock-failed",
                on_pointer_lock_error.as_ref().unchecked_ref(),
            );
            on_pointer_lock_error.forget();

            wasm_bindgen_futures::spawn_local(async move {
                set_dom_status(
                    "Downloading Minecraft 26.1 core assets & initializing WebGPU...",
                    false,
                );
                let client_locale = args.client_locale.clone();
                let gpu_init_fut =
                    bbb_client_render::Renderer::new_with_vsync_and_terrain_filtering_for_fabulous(
                        &window,
                        args.client_vsync,
                        args.terrain_texture_filtering.into(),
                        args.fabulous_texture_filtering,
                    );
                let win_for_assets = browser_window.clone();
                let expected_etag = config.core_bundle_etag.clone();
                let asset_index_id = config.asset_index_id.clone();
                let locale_for_fetch = client_locale.clone();
                let assets_init_fut = async move {
                    let probed = cache_probe_rx.await.unwrap_or((None, None));
                    let (bundle_res, index_res, _) = tokio::join!(
                        fetch_core_bundle_cached(&win_for_assets, probed, expected_etag.as_deref(),),
                        index_bytes_rx,
                        net_worker_rx,
                    );
                    let core_zip_bytes = bundle_res?;
                    let asset_index_bytes =
                        index_res.map_err(|_| anyhow::anyhow!("asset index channel closed"))??;
                    let vanilla_roots = PackRoots::from_web_bundle(
                        core_zip_bytes.into(),
                        Some(asset_index_id.as_str()),
                        Some(asset_index_bytes.as_slice()),
                    )
                    .context("build PackRoots from web bundle")?;
                    drop(asset_index_bytes);
                    let locale_catalog =
                        Rc::new(locales::WebLocaleCatalog::from_pack_roots(&vanilla_roots));
                    let mut user_pack_registry = packs::WebUserPackRegistry::new(vanilla_roots);
                    packs::mount_startup_pack_urls(
                        &win_for_assets,
                        &mut user_pack_registry,
                        &startup_pack_urls,
                    )
                    .await;
                    let pack_roots = Some(user_pack_registry.base_roots().clone());
                    if locale_catalog.entry_for_locale(&locale_for_fetch).is_some() {
                        if let Err(err) = locales::ensure_locale_mounted(
                            &win_for_assets,
                            &locale_catalog,
                            &locale_for_fetch,
                        )
                        .await
                        {
                            tracing::warn!(
                                locale = %locale_for_fetch,
                                %err,
                                "failed to fetch non-bundled Web locale; retaining built-in en_us"
                            );
                        }
                    }
                    Ok::<_, anyhow::Error>((pack_roots, user_pack_registry, locale_catalog))
                };

                let (renderer_res, assets_res) = tokio::join!(gpu_init_fut, assets_init_fut);
                let mut renderer = match renderer_res {
                    Ok(renderer) => renderer,
                    Err(err) => {
                        set_dom_status(&format!("WebGPU initialization failed: {err:#}"), true);
                        return;
                    }
                };
                let (pack_roots, user_pack_registry, locale_catalog) = match assets_res {
                    Ok(loaded) => loaded,
                    Err(err) => {
                        set_dom_status(&format!("Startup error: {err:#}"), true);
                        return;
                    }
                };
                let base_pack_roots = pack_roots.clone();
                let known_packs_manager = load_known_packs_manager(pack_roots.as_ref());

                let (mut world, mut net_counters) =
                    session_shared::build_initial_world_and_net_counters(
                        &args,
                        pack_roots.as_ref(),
                        &client_locale,
                    );
                world.set_known_packs_manager(known_packs_manager.clone());
                let mut code_of_conduct_acceptance =
                    CodeOfConductAcceptance::load(default_code_of_conduct_store_path())
                        .unwrap_or_else(|_| {
                            CodeOfConductAcceptance::empty(default_code_of_conduct_store_path())
                        });
                let server_list = Some(ServerListPersistence::new_web(
                    "Minecraft Server",
                    &args.server,
                    args.server_resource_pack_policy.is_none(),
                ));
                let client_identity = startup::client_identity_from_args(&args);
                let mut network_runtime = match start_network_web_if_requested(
                    &args,
                    &client_identity,
                    ws_gateway_url,
                    &mut code_of_conduct_acceptance,
                    known_packs_manager,
                    &loop_wake,
                ) {
                    Ok(rt) => rt,
                    Err(err) => {
                        set_dom_status(&format!("Startup error: {err:#}"), true);
                        return;
                    }
                };
                let (net_events, net_commands) = network_runtime.take_channels();
                world.set_local_player_profile_id(client_identity.profile_id);
                let server_resource_pack_runtime = ServerResourcePackRuntime::new_web(
                    client_identity.username.clone(),
                    client_identity.profile_id,
                    startup::server_resource_pack_proxy_from_args(&args),
                );

                set_dom_status("Initializing Web Audio & game assets...", false);
                yield_to_browser_paint(&browser_window).await;
                // Offload CPU/Rayon-parallel startup asset decoding onto the Rayon
                // Web Worker pool via the lock-free relay so the browser Window
                // main thread never hits `memory.atomic.wait32` and overlaps Web
                // Audio setup with asset decoding.
                let core_assets_rx = if crate::terrain_upload::wasm_dispatch::is_relay_active() {
                    bbb_client_render::set_web_png_encode_dispatcher(
                        crate::terrain_upload::wasm_dispatch::spawn_from_main_thread,
                    );
                    let (core_assets_tx, core_assets_rx) = bbb_render_types::wasm_sync::wait_slot();
                    let worker_particle_status: bbb_protocol::packets::ClientParticleStatus =
                        args.client_particles.into();
                    let worker_font_options =
                        bbb_client_render::item_model::font::providers::FontOptions {
                            uniform: args.client_force_unicode_font,
                            japanese_variants: args.client_japanese_glyph_variants,
                        };
                    let worker_roots = pack_roots.clone();
                    let worker_locale = client_locale.clone();
                    let worker_max_texture_dimension_2d = renderer.max_texture_dimension_2d();
                    crate::terrain_upload::wasm_dispatch::spawn_from_main_thread(move || {
                        let assets = session_shared::load_core_client_assets_detached(
                            worker_particle_status,
                            worker_font_options,
                            worker_roots.as_ref(),
                            &worker_locale,
                            Some(worker_max_texture_dimension_2d),
                            false,
                        );
                        if assets.is_ok() {
                            if let Some(roots) = worker_roots.as_ref() {
                                let _ = roots.load_block_texture_images_shared();
                            }
                        }
                        core_assets_tx.complete(assets);
                    });
                    Some(core_assets_rx)
                } else {
                    None
                };
                let (audio_runtime, audio_status) = match pack_roots.as_ref() {
                    Some(roots) => match NativeAudioRuntime::load_web(
                        roots,
                        args.client_directional_audio,
                        args.client_audio_volumes,
                    ) {
                        Ok(runtime) => {
                            let status = runtime.counters();
                            (Some(runtime), status)
                        }
                        Err(err) => {
                            tracing::warn!(?err, "continuing without Web Audio runtime");
                            (None, AudioCounters::disabled(err.to_string()))
                        }
                    },
                    None => (
                        None,
                        AudioCounters::disabled("pack roots unavailable for Web Audio"),
                    ),
                };
                world.set_available_command_sounds(
                    audio_runtime
                        .as_ref()
                        .map(NativeAudioRuntime::available_sound_ids)
                        .unwrap_or_default(),
                );

                let narrator = speech::WebSpeechNarrator::try_new(
                    &browser_window,
                    args.client_narrator.into(),
                    args.narrator_voice_volume,
                );
                let sound_subtitle_runtime = SoundSubtitleRuntime::default();
                let loaded_core = match match core_assets_rx {
                    Some(rx) => rx
                        .await
                        .map_err(|err| anyhow::anyhow!("web asset worker canceled: {err}"))
                        .and_then(|assets| assets)
                        .and_then(|assets| {
                            session_shared::apply_loaded_core_client_assets_to_world(
                                &assets, &mut world,
                            )?;
                            Ok(assets)
                        }),
                    None => session_shared::load_core_client_assets(
                        &args,
                        pack_roots.as_ref(),
                        &client_locale,
                        &mut world,
                        Some(renderer.max_texture_dimension_2d()),
                        false,
                    ),
                } {
                    Ok(assets) => assets,
                    Err(err) => {
                        set_dom_status(&format!("Startup error: {err:#}"), true);
                        return;
                    }
                };
                let LoadedCoreClientAssets {
                    client_particle_status,
                    mut particle_runtime,
                    particle_render_runtime,
                    font_options,
                    mut font_atlas,
                    mut item_runtime,
                } = loaded_core;

                let observed_size = {
                    let s = window.inner_size();
                    if s.width > 0 && s.height > 0 {
                        s
                    } else {
                        PhysicalSize::new(init_w, init_h)
                    }
                };
                if observed_size != renderer.physical_surface_size() {
                    if let Err(err) = renderer.resize(observed_size) {
                        set_dom_status(&format!("WebGPU surface resize failed: {err:#}"), true);
                        return;
                    }
                }

                set_dom_status("Uploading GPU atlases...", false);
                yield_to_browser_paint(&browser_window).await;
                let uploaded_gpu = match session_shared::configure_and_upload_initial_gpu_assets(
                    &mut renderer,
                    &args,
                    &pack_roots,
                    &mut font_atlas,
                    &mut particle_runtime,
                    &particle_render_runtime,
                    &mut item_runtime,
                    &client_locale,
                    &mut net_counters,
                    &command_queue,
                    false,
                ) {
                    Ok(uploaded) => uploaded,
                    Err(err) => {
                        set_dom_status(&format!("GPU atlas upload failed: {err:#}"), true);
                        return;
                    }
                };

                let input =
                    session_shared::build_configured_input_state(&args, true, narrator.is_some());
                let (core, state) = match session_shared::build_simulation_core_and_render_state(
                    &args,
                    pack_roots.as_ref(),
                    &mut renderer,
                    SessionCoreInputs {
                        net_events,
                        net_commands,
                        server_resource_packs: server_resource_pack_runtime,
                        input,
                        world,
                        net_counters,
                        client_particle_status,
                        particle_runtime,
                        item_runtime,
                        uploaded_gpu,
                        code_of_conduct: code_of_conduct_acceptance,
                        server_list,
                        audio_status,
                        control_views_enabled: control_enabled,
                        publication_demand: publication_demand.clone(),
                    },
                ) {
                    Ok(built) => built,
                    Err(err) => {
                        set_dom_status(&format!("Invalid client information: {err:#}"), true);
                        return;
                    }
                };

                if !network_runtime.activate() {
                    tracing::warn!("web network runtime ended before initial activation");
                }

                let pack_controls = packs::install_web_pack_drop_and_picker(
                    &document,
                    &canvas,
                    Arc::clone(&loop_wake),
                    user_pack_registry.remaining_bytes(),
                );
                hide_dom_loading_overlay();
                window.request_redraw();
                loop_wake.wake();

                let mut ctx = SessionEventLoopContext {
                    window,
                    renderer,
                    core,
                    state,
                    args,
                    command_queue,
                    command_inbox,
                    snapshot,
                    client_locale,
                    client_options,
                    base_pack_roots,
                    pack_roots,
                    audio_runtime,
                    sound_subtitle_runtime,
                    particle_render_runtime,
                    font_options,
                    client_particle_status,
                };
                let mut hooks = WebEventLoopHooks {
                    network_runtime,
                    debug_clipboard,
                    narrator,
                    user_pack_registry,
                    pack_controls,
                    pending_user_pack_reload: None,
                    locale_catalog,
                    browser_window,
                    loop_wake,
                    web_control_tunnel,
                    pending_resource_reload: None,
                };

                *handler_slot.borrow_mut() = Some(Box::new(
                    move |event: Event<()>, target: &ActiveEventLoop| {
                        drive_session_event(&mut ctx, &mut hooks, event, target);
                    },
                ));
            });
        })),
    };

    event_loop.spawn_app(app);
    Ok(())
}

#[cfg(target_arch = "wasm32")]
struct WebEventLoopHooks {
    network_runtime: NetworkRuntime,
    debug_clipboard: WebDebugClipboard,
    narrator: Option<speech::WebSpeechNarrator>,
    user_pack_registry: packs::WebUserPackRegistry,
    pack_controls: Option<Rc<packs::WebPackControls>>,
    pending_user_pack_reload: Option<PendingWebUserPackReload>,
    locale_catalog: Rc<locales::WebLocaleCatalog>,
    browser_window: web_sys::Window,
    loop_wake: Arc<LoopWake>,
    web_control_tunnel: Option<WebControlTunnel>,
    pending_resource_reload: Option<
        crate::terrain_upload::wasm_dispatch::LockFreeReceiver<PreparedServerResourcePackReload>,
    >,
}

#[cfg(target_arch = "wasm32")]
struct PendingWebUserPackReload {
    change: packs::WebUserPackChange,
    completion: tokio::sync::oneshot::Sender<Result<bool>>,
    generation: Option<u64>,
}

/// Whether any screen (including Chat and GameModeSwitcher) is currently open
/// on the Web client. `blocks_world_input()` intentionally returns `false` for
/// Chat/GameModeSwitcher, so Web pointer-lock and DOM paste gating must check
/// full screen presence (`wants_cursor() || command_entry_is_active()`).
#[cfg(any(target_arch = "wasm32", test))]
fn web_screen_is_open(input: &crate::input::ClientInputState) -> bool {
    input.wants_cursor() || input.command_entry_is_active()
}

#[cfg(target_arch = "wasm32")]
impl PlatformEventLoopHooks for WebEventLoopHooks {
    fn shutdown_platform(&mut self, _ctx: &mut SessionEventLoopContext) {
        self.network_runtime.signal_close_web();
    }

    fn on_before_event(&mut self, ctx: &mut SessionEventLoopContext, event: &Event<()>) {
        if matches!(
            event,
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { .. }
                    | WindowEvent::MouseInput { .. }
                    | WindowEvent::MouseWheel { .. },
                ..
            }
        ) {
            if let Some(audio) = ctx.audio_runtime.as_mut() {
                audio.resume_web_context();
            }
        }
    }

    fn intercept_window_event(
        &mut self,
        ctx: &mut SessionEventLoopContext,
        event: &WindowEvent,
    ) -> bool {
        if matches!(event, WindowEvent::Focused(false)) && ime::ime_proxy_owns_focus() {
            return true;
        }
        if matches!(
            event,
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                ..
            }
        ) && ctx.core.world.level_info().is_some()
            && !web_screen_is_open(&ctx.core.input)
            && !bbb_client_clock::is_virtual()
            && !dom_has_pointer_lock()
        {
            ctx.window.set_ime_allowed(false);
            ime::set_web_ime_allowed(false);
            ctx.core.input_runtime.set_cursor_captured(true);
            // Defer cursor hiding until `pointerlockchange` confirms lock,
            // avoiding an invisible-cursor soft-lock during Chrome's Escape cooldown
            // while still allowing gameplay keys when Pointer Lock is pending or unavailable.
            let _ = ctx.window.set_cursor_grab(CursorGrabMode::Locked);
            return true;
        }
        false
    }

    fn resolve_surface_resize(
        &mut self,
        _ctx: &SessionEventLoopContext,
        size: PhysicalSize<u32>,
    ) -> Option<PhysicalSize<u32>> {
        (size.width != 0 && size.height != 0).then_some(size)
    }

    fn on_resize_error(
        &mut self,
        _ctx: &SessionEventLoopContext,
        _old_size: PhysicalSize<u32>,
        _new_size: PhysicalSize<u32>,
        error: anyhow::Error,
    ) {
        set_dom_status(&format!("Renderer resize error: {error:#}"), true);
    }

    fn clipboard_and_narrator_mut(
        &mut self,
    ) -> (&mut dyn DebugClipboard, Option<&mut dyn SystemNarrator>) {
        (
            &mut self.debug_clipboard,
            self.narrator.as_mut().map(|n| n as &mut dyn SystemNarrator),
        )
    }

    fn on_occluded(&mut self, ctx: &mut SessionEventLoopContext, occluded: bool) {
        ctx.state.redraw_requested = false;
        if !occluded {
            ctx.window.request_redraw();
            ctx.state.redraw_requested = true;
        }
    }

    fn render_frame(
        &mut self,
        ctx: &mut SessionEventLoopContext,
        target: &ActiveEventLoop,
    ) -> Option<Duration> {
        if let Some(pending) = ctx.state.pending_control_screenshots.pop_front() {
            let queue_for_shot = ctx.command_queue.clone();
            let announce = pending.announce;
            let dl_name = pending
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            ctx.renderer.request_web_screenshot(move |result| {
                if announce {
                    if let Ok(img) = &result {
                        trigger_browser_png_download(&dl_name, &img.bytes);
                    }
                }
                pending.complete(result, &queue_for_shot);
            });
        }
        let started_at = bbb_client_clock::Instant::now();
        if let Err(err) = ctx.renderer.render_with_capture(None, false) {
            set_dom_status(&format!("WebGPU render failed: {err:#}"), true);
            target.exit();
            return None;
        }
        Some(started_at.elapsed())
    }

    fn allow_mouse_motion(&self) -> bool {
        DOM_POINTER_WAS_LOCKED.with(Cell::get)
    }

    fn collect_pre_step_commands(
        &mut self,
        ctx: &mut SessionEventLoopContext,
        commands: &mut Vec<SimulationCommand>,
        queued: &mut u64,
    ) {
        if let Some(request) = self
            .pack_controls
            .as_ref()
            .and_then(|controls| controls.take_request())
        {
            let change = match request.action {
                packs::WebPackAction::Mount(name, bytes) => {
                    self.user_pack_registry.prepare_mount(&name, bytes)
                }
                packs::WebPackAction::Clear => Ok(self.user_pack_registry.prepare_clear()),
            };
            match change {
                Ok(Some(change)) => {
                    self.pending_user_pack_reload = Some(PendingWebUserPackReload {
                        change,
                        completion: request.completion,
                        generation: None,
                    });
                    ctx.core.server_resource_packs.request_local_pack_reload();
                }
                Ok(None) => {
                    let _ = request.completion.send(Ok(false));
                }
                Err(error) => {
                    let _ = request.completion.send(Err(error));
                }
            }
        }
        if let Some(prepared) = self
            .pending_resource_reload
            .as_mut()
            .and_then(|rx| rx.try_recv().ok())
        {
            self.pending_resource_reload = None;
            commands.extend(ctx.commit_prepared_reload(prepared));
            *queued += 2;
        }
        if POINTER_LOCK_ACQUIRED.with(|c| c.replace(false)) && dom_has_pointer_lock() {
            if !web_screen_is_open(&ctx.core.input) {
                ctx.core.input_runtime.set_cursor_captured(true);
                ctx.window.set_cursor_visible(false);
            } else {
                notify_intentional_pointer_unlock();
                let _ = ctx
                    .window
                    .set_cursor_grab(winit::window::CursorGrabMode::None);
                ctx.window.set_cursor_visible(true);
            }
        }
        if POINTER_LOCK_FAILED.with(|c| c.replace(false)) && !dom_has_pointer_lock() {
            ctx.window.set_cursor_visible(true);
        }
        if EXTERNAL_POINTER_UNLOCKED.with(|c| c.replace(false))
            && ctx.core.input_runtime.cursor_captured()
        {
            ctx.core.input_runtime.set_cursor_captured(false);
            ctx.window.set_cursor_visible(true);
            if document_has_focus() && !web_screen_is_open(&ctx.core.input) {
                commands.push(SimulationCommand::Key {
                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                    state: ElementState::Pressed,
                    clipboard_text: None,
                });
                commands.push(SimulationCommand::Key {
                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                    state: ElementState::Released,
                    clipboard_text: None,
                });
            } else if !document_has_focus() {
                commands.push(SimulationCommand::FocusChanged(false));
            }
        }
        if let Some(paste_text) = PENDING_PASTE_TEXT.with(|slot| slot.borrow_mut().take()) {
            if web_screen_is_open(&ctx.core.input) {
                let synth_ctrl = !ctx.core.input.text_edit_shortcut_down();
                if synth_ctrl {
                    commands.push(SimulationCommand::Key {
                        physical_key: PhysicalKey::Code(KeyCode::ControlLeft),
                        state: ElementState::Pressed,
                        clipboard_text: None,
                    });
                }
                commands.push(SimulationCommand::Key {
                    physical_key: PhysicalKey::Code(KeyCode::KeyV),
                    state: ElementState::Pressed,
                    clipboard_text: Some(paste_text),
                });
                commands.push(SimulationCommand::Key {
                    physical_key: PhysicalKey::Code(KeyCode::KeyV),
                    state: ElementState::Released,
                    clipboard_text: None,
                });
                if synth_ctrl {
                    commands.push(SimulationCommand::Key {
                        physical_key: PhysicalKey::Code(KeyCode::ControlLeft),
                        state: ElementState::Released,
                        clipboard_text: None,
                    });
                }
            }
        }
    }

    fn step_simulation(
        &mut self,
        ctx: &mut SessionEventLoopContext,
        _target: &ActiveEventLoop,
        now: bbb_client_clock::Instant,
        commands: Vec<SimulationCommand>,
    ) -> Option<Option<StepOutcome>> {
        let outcome = ctx.core.step(
            now,
            commands,
            ctx.audio_runtime
                .as_mut()
                .map(|audio| audio as &mut dyn AudioEventSink),
        );
        let diagnostics = &ctx.core.net_counters.client_resources;
        if let Some(generation) = diagnostics.last_reload_generation {
            if self
                .pending_user_pack_reload
                .as_ref()
                .and_then(|reload| reload.generation)
                == Some(generation)
            {
                if let Some(reload) = self.pending_user_pack_reload.take() {
                    let result = if let Some(error) = &diagnostics.last_reload_error {
                        Err(anyhow::anyhow!("{error}"))
                    } else {
                        ctx.base_pack_roots = Some(reload.change.roots.clone());
                        self.user_pack_registry.publish(reload.change);
                        if let Some(controls) = &self.pack_controls {
                            controls.set_remaining_bytes(self.user_pack_registry.remaining_bytes());
                        }
                        Ok(true)
                    };
                    let _ = reload.completion.send(result);
                }
            }
        }
        Some(outcome)
    }

    fn dispatch_resource_reload(
        &mut self,
        ctx: &mut SessionEventLoopContext,
        reload: ResourceReloadEffect,
    ) {
        let needs_locale_fetch = self
            .locale_catalog
            .entry_for_locale(&ctx.client_locale)
            .is_some_and(|entry| {
                !bbb_client_assets::virtual_store::has_virtual_file(&entry.virtual_path())
            });
        let synchronous = bbb_client_clock::is_virtual() && !needs_locale_fetch;
        if synchronous && !reload.request.local_pack_change {
            ctx.apply_virtual_clock_reload(reload);
            return;
        }
        let base = if reload.request.local_pack_change {
            let pending = self
                .pending_user_pack_reload
                .as_mut()
                .expect("local pack reload queued");
            pending.generation = Some(reload.request.generation);
            Some(pending.change.roots.clone())
        } else {
            ctx.base_pack_roots.clone()
        };
        let active = ctx.pack_roots.clone();
        let has_audio = ctx.audio_runtime.is_some();
        let reload_locale = ctx.client_locale.clone();
        let font_options = ctx.font_options;
        let max_texture_dimension_2d = ctx.renderer.max_texture_dimension_2d();
        let client_particle_status = ctx.client_particle_status;
        let username = ctx.args.username.clone();
        let prepare = {
            let reload_locale = reload_locale.clone();
            move || {
                prepare_server_resource_pack_reload(
                    reload,
                    base,
                    active,
                    has_audio,
                    reload_locale,
                    font_options,
                    max_texture_dimension_2d,
                    client_particle_status,
                    username,
                )
            }
        };
        if synchronous {
            for command in ctx.commit_prepared_reload(prepare()) {
                ctx.command_queue.push(command);
            }
            return;
        }
        let (tx, rx) = crate::terrain_upload::wasm_dispatch::lock_free_channel();
        self.pending_resource_reload = Some(rx);
        if needs_locale_fetch {
            let win_for_locale = self.browser_window.clone();
            let catalog_for_locale = Rc::clone(&self.locale_catalog);
            let wake_for_reload = Arc::clone(&self.loop_wake);
            wasm_bindgen_futures::spawn_local(async move {
                if let Err(err) = locales::ensure_locale_mounted(
                    &win_for_locale,
                    &catalog_for_locale,
                    &reload_locale,
                )
                .await
                {
                    tracing::warn!(
                        locale = %reload_locale,
                        %err,
                        "failed fetching runtime Web locale"
                    );
                }
                crate::terrain_upload::wasm_dispatch::spawn_from_main_thread(move || {
                    let _ = tx.send(prepare());
                    wake_for_reload.wake();
                });
            });
        } else {
            let wake_for_reload = Arc::clone(&self.loop_wake);
            crate::terrain_upload::wasm_dispatch::spawn_from_main_thread(move || {
                let _ = tx.send(prepare());
                wake_for_reload.wake();
            });
        }
    }

    fn on_published_generation(&mut self, generation: u64) {
        if let Some(tunnel) = self.web_control_tunnel.as_mut() {
            tunnel.publish_generation(generation);
        }
    }

    fn before_post_step_schedule(&mut self, _ctx: &mut SessionEventLoopContext) {
        ime::ensure_web_ime_focus_if_allowed();
    }

    fn schedule_policy(&self, _window: &Window) -> (bool, bool, Option<Duration>) {
        let hidden = web_sys::window()
            .and_then(|win| win.document())
            .is_some_and(|doc| doc.hidden());
        (
            hidden,
            false,
            self.pending_resource_reload
                .is_some()
                .then_some(Duration::from_millis(16)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{format_core_bundle_progress, web_screen_is_open};
    use crate::input::ClientInputState;

    #[test]
    fn core_bundle_progress_formats_mib_and_percentage() {
        let known = format_core_bundle_progress(7 * 1024 * 1024, Some(14 * 1024 * 1024));
        assert_eq!(
            known,
            "Downloading Minecraft 26.1 core assets (7.0 / 14.0 MiB, 50%)..."
        );

        let unknown = format_core_bundle_progress(3 * 1024 * 1024, None);
        assert_eq!(
            unknown,
            "Downloading Minecraft 26.1 core assets (3.0 MiB)..."
        );
    }

    #[test]
    fn web_screen_is_open_includes_chat_entry() {
        let mut world = bbb_client_world::WorldStore::new();
        let mut counters = crate::control::NetCounters::default();
        let mut input = ClientInputState::new(true);
        assert!(!web_screen_is_open(&input));
        crate::input::handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &None,
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyT),
            winit::event::ElementState::Pressed,
        );
        assert!(input.command_entry_is_active());
        assert!(!input.blocks_world_input());
        assert!(web_screen_is_open(&input));
    }
}
