//! User-provided local `.zip` resource pack mounting (drag-and-drop, file picker, `?pack=` URL).

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

#[cfg(target_arch = "wasm32")]
use anyhow::Context;
use anyhow::{bail, Result};
use bbb_client_assets::{virtual_store, PackRoots};

#[cfg(target_arch = "wasm32")]
use crate::simulation::commands::LoopWake;
#[cfg(target_arch = "wasm32")]
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[cfg(target_arch = "wasm32")]
use tokio::sync::oneshot;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{closure::Closure, prelude::*, JsCast};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

const WEB_USER_PACKS_ROOT: &str = "/__bbb_mem__/user-packs";
const MAX_WEB_USER_PACK_BYTES: usize = 256 * 1024 * 1024;
static NEXT_WEB_USER_PACK_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) struct WebUserPackRegistry {
    vanilla_roots: PackRoots,
    base_roots: PackRoots,
    mounted: Vec<MountedWebPack>,
    mounted_bytes: usize,
}

pub(super) struct WebUserPackChange {
    pub(super) roots: PackRoots,
    mounted: Option<MountedWebPack>,
}

struct MountedWebPack {
    path: PathBuf,
    len: usize,
}

impl Drop for MountedWebPack {
    fn drop(&mut self) {
        virtual_store::remove_virtual_prefix(&self.path);
    }
}

impl WebUserPackRegistry {
    pub(crate) fn new(vanilla_roots: PackRoots) -> Self {
        Self {
            base_roots: vanilla_roots.clone(),
            vanilla_roots,
            mounted: Vec::new(),
            mounted_bytes: 0,
        }
    }

    pub(super) fn base_roots(&self) -> &PackRoots {
        &self.base_roots
    }

    pub(super) fn remaining_bytes(&self) -> usize {
        MAX_WEB_USER_PACK_BYTES - self.mounted_bytes
    }

    pub(super) fn prepare_mount(
        &self,
        display_name: &str,
        bytes: Vec<u8>,
    ) -> Result<Option<WebUserPackChange>> {
        if bytes.is_empty() {
            bail!("resource pack is empty");
        }
        check_pack_size(bytes.len() as u64, self.remaining_bytes())?;
        if self.mounted.iter().any(|pack| {
            pack.len == bytes.len()
                && virtual_store::get_virtual_file(&pack.path)
                    .is_some_and(|existing| existing.as_ref() == bytes.as_slice())
        }) {
            return Ok(None);
        }

        let seq = NEXT_WEB_USER_PACK_ID.fetch_add(1, Ordering::Relaxed);
        let stem = sanitize_pack_stem(display_name);
        let mounted = MountedWebPack {
            path: PathBuf::from(format!("{WEB_USER_PACKS_ROOT}/{seq}-{stem}.zip")),
            len: bytes.len(),
        };
        virtual_store::insert_virtual_file(&mounted.path, Arc::from(bytes));

        let stack = self
            .base_roots
            .selected_client_packs()
            .ok_or_else(|| anyhow::anyhow!("web roots have no selected client pack stack"))?
            .with_startup_pack(mounted.path.clone())?;
        let roots = self.base_roots.clone().with_selected_client_packs(stack)?;
        Ok(Some(WebUserPackChange {
            roots,
            mounted: Some(mounted),
        }))
    }

    pub(super) fn prepare_clear(&self) -> Option<WebUserPackChange> {
        (!self.mounted.is_empty()).then(|| WebUserPackChange {
            roots: self.vanilla_roots.clone(),
            mounted: None,
        })
    }

    pub(super) fn publish(&mut self, change: WebUserPackChange) {
        self.base_roots = change.roots;
        if let Some(pack) = change.mounted {
            self.mounted_bytes += pack.len;
            self.mounted.push(pack);
        } else {
            self.mounted.clear();
            self.mounted_bytes = 0;
        }
    }
}

fn check_pack_size(total_bytes: u64, remaining_bytes: usize) -> Result<()> {
    if total_bytes > remaining_bytes as u64 {
        bail!("exceeds the 256 MiB local ZIP budget ({remaining_bytes} bytes remaining)");
    }
    Ok(())
}

fn sanitize_pack_stem(name: &str) -> String {
    let trimmed = name.rsplit(['/', '\\']).next().unwrap_or(name).trim();
    let without_ext = trimmed
        .strip_suffix(".zip")
        .or_else(|| trimmed.strip_suffix(".ZIP"))
        .unwrap_or(trimmed);
    let sanitized = without_ext
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    let cleaned = sanitized.trim_matches(['.', '_', '-']);
    if cleaned.is_empty() {
        "pack".to_string()
    } else {
        cleaned.chars().take(64).collect()
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) enum WebPackAction {
    Mount(String, Vec<u8>),
    Clear,
}

#[cfg(target_arch = "wasm32")]
pub(super) struct WebPackRequest {
    pub(super) action: WebPackAction,
    pub(super) completion: oneshot::Sender<Result<bool>>,
}

#[cfg(target_arch = "wasm32")]
pub(super) struct WebPackControls {
    input: web_sys::HtmlInputElement,
    clear: web_sys::HtmlElement,
    status: Option<web_sys::Element>,
    busy: Cell<bool>,
    remaining_bytes: Cell<usize>,
    pending: RefCell<Option<WebPackRequest>>,
}

#[cfg(target_arch = "wasm32")]
impl WebPackControls {
    pub(super) fn take_request(&self) -> Option<WebPackRequest> {
        self.pending.borrow_mut().take()
    }

    pub(super) fn set_remaining_bytes(&self, remaining_bytes: usize) {
        self.remaining_bytes.set(remaining_bytes);
        if let Err(error) = self.clear.toggle_attribute_with_force(
            "disabled",
            self.busy.get() || remaining_bytes == MAX_WEB_USER_PACK_BYTES,
        ) {
            tracing::warn!(?error, "failed to update clear local packs button");
        }
    }

    fn set_status_text(&self, text: &str) {
        if let Some(status) = &self.status {
            status.set_text_content(Some(text));
        }
    }

    fn set_busy(&self, busy: bool) {
        self.busy.set(busy);
        self.input.set_disabled(busy);
        self.set_remaining_bytes(self.remaining_bytes.get());
    }

    async fn run(&self, action: WebPackAction, loop_wake: &LoopWake) -> Result<bool> {
        let (completion, result) = oneshot::channel();
        self.pending
            .replace(Some(WebPackRequest { action, completion }));
        loop_wake.wake();
        result.await.context("local resource pack reload ended")?
    }
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static WEB_PACK_FILE_INPUT: RefCell<Option<web_sys::HtmlInputElement>> = const { RefCell::new(None) };
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn open_web_resource_pack_picker() -> bool {
    WEB_PACK_FILE_INPUT.with(|slot| {
        let Some(input) = slot.borrow().clone() else {
            return false;
        };
        if input.disabled() {
            return false;
        }
        input.click();
        true
    })
}

#[cfg(target_arch = "wasm32")]
#[derive(Default)]
struct PackSelectionSummary {
    mounted: usize,
    duplicates: usize,
    rejected: usize,
    last_error: String,
}

#[cfg(target_arch = "wasm32")]
impl PackSelectionSummary {
    fn record(&mut self, name: &str, outcome: Result<bool>) {
        match outcome {
            Ok(true) => self.mounted += 1,
            Ok(false) => self.duplicates += 1,
            Err(error) => {
                tracing::warn!(pack = %name, %error, "rejected local Web resource pack");
                self.rejected += 1;
                self.last_error = format!(" {name}: {error:#}");
            }
        }
    }

    fn display(&self, status: &web_sys::Element) {
        status.set_text_content(Some(&format!(
            "{} added, {} already active, {} rejected.{}",
            self.mounted, self.duplicates, self.rejected, self.last_error
        )));
    }
}

#[cfg(target_arch = "wasm32")]
fn enqueue_browser_file_list(
    files: web_sys::FileList,
    controls: &Rc<WebPackControls>,
    loop_wake: &Arc<LoopWake>,
) {
    if files.length() == 0 || controls.busy.get() {
        return;
    }
    // Drop-event FileList contents are only accessible during the event.
    let selected: Vec<_> = (0..files.length())
        .filter_map(|index| files.get(index))
        .collect();
    controls.set_busy(true);
    let controls = Rc::clone(controls);
    let loop_wake = Arc::clone(loop_wake);
    wasm_bindgen_futures::spawn_local(async move {
        let mut summary = PackSelectionSummary::default();
        for file in selected {
            let name = file.name();
            controls.set_status_text(&format!("Loading {name}…"));
            let outcome = match browser_file_bytes(&file, controls.remaining_bytes.get()).await {
                Ok(bytes) => {
                    controls
                        .run(WebPackAction::Mount(name.clone(), bytes), &loop_wake)
                        .await
                }
                Err(error) => Err(error),
            };
            summary.record(&name, outcome);
        }
        if let Some(status) = &controls.status {
            summary.display(status);
        }
        controls.set_busy(false);
    });
}

#[cfg(target_arch = "wasm32")]
async fn browser_file_bytes(file: &web_sys::File, remaining_bytes: usize) -> Result<Vec<u8>> {
    if !file.name().to_ascii_lowercase().ends_with(".zip") {
        bail!("select a .zip file");
    }
    check_pack_size(file.size() as u64, remaining_bytes)?;
    read_pack_stream(file.stream(), remaining_bytes).await
}

#[cfg(target_arch = "wasm32")]
pub(super) fn install_web_pack_drop_and_picker(
    document: &web_sys::Document,
    canvas: &web_sys::HtmlCanvasElement,
    loop_wake: Arc<LoopWake>,
    remaining_bytes: usize,
) -> Option<Rc<WebPackControls>> {
    let Some(input_element) = document.get_element_by_id("bbb-pack-picker") else {
        tracing::warn!("missing #bbb-pack-picker element; web resource pack picker disabled");
        return None;
    };
    let Ok(input) = input_element.dyn_into::<web_sys::HtmlInputElement>() else {
        tracing::warn!("#bbb-pack-picker is not an HTML input; web resource pack picker disabled");
        return None;
    };
    let Some(clear_element) = document.get_element_by_id("bbb-pack-clear") else {
        tracing::warn!("missing #bbb-pack-clear element; web resource pack picker disabled");
        return None;
    };
    let Ok(clear) = clear_element.dyn_into::<web_sys::HtmlElement>() else {
        tracing::warn!("#bbb-pack-clear is not an HTML element; web resource pack picker disabled");
        return None;
    };
    let controls = Rc::new(WebPackControls {
        input: input.clone(),
        clear: clear.clone(),
        status: pack_status(document),
        busy: Cell::new(false),
        remaining_bytes: Cell::new(remaining_bytes),
        pending: RefCell::new(None),
    });
    controls.set_remaining_bytes(remaining_bytes);

    let on_dragover =
        Closure::<dyn FnMut(web_sys::DragEvent)>::new(move |ev: web_sys::DragEvent| {
            ev.prevent_default();
            if let Some(dt) = ev.data_transfer() {
                dt.set_drop_effect("copy");
            }
        });
    let _ =
        canvas.add_event_listener_with_callback("dragover", on_dragover.as_ref().unchecked_ref());
    on_dragover.forget();

    let drop_wake = Arc::clone(&loop_wake);
    let drop_controls = Rc::clone(&controls);
    let on_drop = Closure::<dyn FnMut(web_sys::DragEvent)>::new(move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        if let Some(files) = ev.data_transfer().and_then(|dt| dt.files()) {
            enqueue_browser_file_list(files, &drop_controls, &drop_wake);
        }
    });
    let _ = canvas.add_event_listener_with_callback("drop", on_drop.as_ref().unchecked_ref());
    on_drop.forget();

    let picker_input = input.clone();
    let picker_controls = Rc::clone(&controls);
    let picker_wake = Arc::clone(&loop_wake);
    let on_change = Closure::<dyn FnMut()>::new(move || {
        if let Some(files) = picker_input.files() {
            enqueue_browser_file_list(files, &picker_controls, &picker_wake);
        }
        picker_input.set_value("");
    });
    if let Err(error) =
        input.add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref())
    {
        tracing::warn!(?error, "failed to register resource pack picker listener");
    } else {
        on_change.forget();
    }
    WEB_PACK_FILE_INPUT.with(|slot| *slot.borrow_mut() = Some(input));

    let clear_controls = Rc::clone(&controls);
    let on_clear = Closure::<dyn FnMut()>::new(move || {
        clear_controls.set_busy(true);
        clear_controls.set_status_text("Clearing local resource packs…");
        let controls = Rc::clone(&clear_controls);
        let loop_wake = Arc::clone(&loop_wake);
        wasm_bindgen_futures::spawn_local(async move {
            let message = match controls.run(WebPackAction::Clear, &loop_wake).await {
                Ok(_) => "Local resource packs cleared.".to_string(),
                Err(error) => format!("Could not clear local resource packs: {error:#}"),
            };
            controls.set_status_text(&message);
            controls.set_busy(false);
        });
    });
    if let Err(error) =
        clear.add_event_listener_with_callback("click", on_clear.as_ref().unchecked_ref())
    {
        tracing::warn!(?error, "failed to register clear local packs button");
    } else {
        on_clear.forget();
    }

    if let Some(container) = document
        .get_element_by_id("bbb-pack-controls")
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
    {
        container.set_hidden(super::dom_has_pointer_lock());
        let on_pointer_lock_change = Closure::<dyn FnMut()>::new(move || {
            container.set_hidden(super::dom_has_pointer_lock());
        });
        if let Err(error) = document.add_event_listener_with_callback(
            "pointerlockchange",
            on_pointer_lock_change.as_ref().unchecked_ref(),
        ) {
            tracing::warn!(
                ?error,
                "failed to register resource pack controls pointer lock listener"
            );
        } else {
            on_pointer_lock_change.forget();
        }
    } else {
        tracing::warn!("missing #bbb-pack-controls HTML element");
    }

    Some(controls)
}

#[cfg(target_arch = "wasm32")]
fn pack_status(document: &web_sys::Document) -> Option<web_sys::Element> {
    document.get_element_by_id("bbb-pack-status")
}

#[cfg(target_arch = "wasm32")]
fn startup_pack_fetch_url(window: &web_sys::Window, raw_url: &str) -> String {
    let Ok(origin) = window.location().origin() else {
        return raw_url.to_string();
    };
    let Ok(parsed) = web_sys::Url::new_with_base(raw_url, &origin) else {
        return raw_url.to_string();
    };
    let protocol = parsed.protocol();
    if (protocol == "http:" || protocol == "https:") && parsed.origin() != origin {
        let encoded: String = js_sys::encode_uri_component(&parsed.href()).into();
        return format!("/proxy/pack?url={encoded}");
    }
    raw_url.to_string()
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn mount_startup_pack_urls(
    window: &web_sys::Window,
    registry: &mut WebUserPackRegistry,
    urls: &[String],
) {
    if urls.is_empty() {
        return;
    }
    let mut summary = PackSelectionSummary::default();
    for url in urls {
        let fetch_url = startup_pack_fetch_url(window, url);
        let outcome = async {
            let response = super::fetch_response(window, &fetch_url).await?;
            let stream = response
                .body()
                .context("resource pack response has no body")?;
            let remaining_bytes = registry.remaining_bytes();
            if let Some(len) = response
                .headers()
                .get("content-length")
                .ok()
                .flatten()
                .and_then(|value| value.parse().ok())
            {
                if let Err(error) = check_pack_size(len, remaining_bytes) {
                    let _ = JsFuture::from(stream.cancel()).await;
                    return Err(error);
                }
            }
            let bytes = read_pack_stream(stream, remaining_bytes).await?;
            if let Some(change) = registry.prepare_mount(url, bytes)? {
                registry.publish(change);
                Ok(true)
            } else {
                Ok(false)
            }
        }
        .await;
        summary.record(url, outcome);
    }
    if let Some(status) = window.document().as_ref().and_then(pack_status) {
        summary.display(&status);
    }
}

#[cfg(target_arch = "wasm32")]
async fn read_pack_stream(
    stream: web_sys::ReadableStream,
    remaining_bytes: usize,
) -> Result<Vec<u8>> {
    let reader = stream
        .get_reader()
        .unchecked_into::<web_sys::ReadableStreamDefaultReader>();
    let mut bytes = Vec::new();
    loop {
        let next = JsFuture::from(reader.read())
            .await
            .map_err(|error| anyhow::anyhow!("read resource pack: {error:?}"))?
            .unchecked_into::<js_sys::IteratorNext>();
        if next.done() {
            return Ok(bytes);
        }
        let chunk = next.value().unchecked_into::<js_sys::Uint8Array>();
        let start = bytes.len();
        let total = start as u64 + u64::from(chunk.length());
        if let Err(error) = check_pack_size(total, remaining_bytes) {
            let _ = JsFuture::from(reader.cancel()).await;
            return Err(error);
        }
        let end = total as usize;
        if end > bytes.capacity() {
            let capacity = end.max(2 * bytes.capacity()).min(remaining_bytes);
            bytes.reserve_exact(capacity - start);
        }
        bytes.resize(end, 0);
        chunk.copy_to(&mut bytes[start..]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

    fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            for &(name, data) in entries {
                zip.start_file(name, opts).unwrap();
                zip.write_all(data).unwrap();
            }
            zip.finish().unwrap();
        }
        cursor.into_inner()
    }

    fn language_pack(translation: &str) -> Vec<u8> {
        make_zip(&[
            (
                "pack.mcmeta",
                br#"{"pack":{"description":"pack","min_format":[84,0],"max_format":[84,0]}}"#,
            ),
            (
                "assets/minecraft/lang/en_us.json",
                format!(r#"{{"menu.singleplayer":"{translation}"}}"#).as_bytes(),
            ),
        ])
    }

    #[test]
    fn web_user_pack_registry_preserves_priority_and_releases_cleared_archives() {
        let vanilla_roots =
            PackRoots::from_web_bundle(Arc::from(language_pack("Vanilla")), None, None).unwrap();
        let mut registry = WebUserPackRegistry::new(vanilla_roots);
        let label = |roots: &PackRoots| {
            roots
                .load_language_catalog("en_us")
                .unwrap()
                .get_or_key("menu.singleplayer")
                .to_owned()
        };

        let first = language_pack("First");
        let second = language_pack("Second");
        let mount = |registry: &mut WebUserPackRegistry, bytes| {
            let change = registry
                .prepare_mount("example.zip", bytes)
                .unwrap()
                .unwrap();
            registry.publish(change);
        };
        mount(&mut registry, first.clone());
        mount(&mut registry, second.clone());

        assert!(registry
            .prepare_mount("renamed.zip", first.clone())
            .unwrap()
            .is_none());
        assert!(registry
            .prepare_mount("corrupt.zip", b"not-a-zip".to_vec())
            .is_err());
        assert_eq!(label(registry.base_roots()), "Second");
        let remaining = registry.remaining_bytes();
        assert_eq!(
            remaining,
            MAX_WEB_USER_PACK_BYTES - first.len() - second.len()
        );
        assert!(check_pack_size(remaining as u64, remaining).is_ok());
        assert!(check_pack_size(remaining as u64 + 1, remaining).is_err());

        let roots = registry.base_roots();
        let archive = roots.resource_pack_dirs()[1].clone();
        let resource =
            bbb_client_assets::ResourceLocation::parse("minecraft:lang/en_us.json").unwrap();
        let entry = roots.resource_stack().get_resource(&resource).unwrap();
        let cache = entry.materialized_path().unwrap().to_path_buf();
        let clear = registry.prepare_clear().unwrap();

        assert_eq!(label(registry.base_roots()), "Second");
        assert!(virtual_store::has_virtual_file(&archive));
        assert!(virtual_store::has_virtual_file(&cache));

        registry.publish(clear);
        assert_eq!(label(registry.base_roots()), "Vanilla");
        assert_eq!(registry.remaining_bytes(), MAX_WEB_USER_PACK_BYTES);
        assert!(!virtual_store::has_virtual_file(&archive));
        assert!(!virtual_store::has_virtual_file(&cache));

        mount(&mut registry, first);
        assert_eq!(label(registry.base_roots()), "First");
    }
}
