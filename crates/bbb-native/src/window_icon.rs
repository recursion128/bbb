use anyhow::{bail, Context, Result};
use bbb_pack::{PackRoots, StandardWindowIcon, WindowIconCatalog, WindowIconFamily, WindowIconSet};
use winit::{
    raw_window_handle::{HasDisplayHandle, RawDisplayHandle},
    window::{Icon, Window},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeWindowBackend {
    Wayland,
    X11,
    Other,
}

impl NativeWindowBackend {
    pub(crate) fn detect(window: &Window) -> Result<Self> {
        Ok(match window.display_handle()?.as_raw() {
            RawDisplayHandle::Wayland(_) => Self::Wayland,
            RawDisplayHandle::Xlib(_) | RawDisplayHandle::Xcb(_) => Self::X11,
            _ => Self::Other,
        })
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Wayland => "wayland",
            Self::X11 => "x11",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowIconTarget {
    SkipWayland,
    SkipUnsupported,
    X11,
    Windows,
    MacOs,
}

impl WindowIconTarget {
    fn for_platform(target_os: &str, backend: NativeWindowBackend) -> Self {
        match target_os {
            "windows" => Self::Windows,
            "macos" => Self::MacOs,
            "linux" => match backend {
                NativeWindowBackend::Wayland => Self::SkipWayland,
                NativeWindowBackend::X11 => Self::X11,
                NativeWindowBackend::Other => Self::SkipUnsupported,
            },
            _ => Self::SkipUnsupported,
        }
    }

    const fn family(self) -> Option<WindowIconFamily> {
        match self {
            Self::X11 | Self::Windows => Some(WindowIconFamily::Standard),
            Self::MacOs => Some(WindowIconFamily::MacOs),
            Self::SkipWayland | Self::SkipUnsupported => None,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::SkipWayland => "wayland-skip",
            Self::SkipUnsupported => "unsupported-skip",
            Self::X11 => "x11",
            Self::Windows => "windows",
            Self::MacOs => "macos",
        }
    }
}

pub(crate) fn install_startup_window_icon(
    window: &Window,
    roots: Option<&PackRoots>,
    backend: NativeWindowBackend,
) {
    let set = WindowIconSet::for_stable(bbb_protocol::MC_STABLE);
    let target = WindowIconTarget::for_platform(std::env::consts::OS, backend);
    if target.family().is_none() {
        tracing::info!(
            window_icon_status = "skipped",
            window_icon_target = target.label(),
            window_icon_object_reads = 0_u8,
            "native window icon not required by platform backend"
        );
        return;
    }

    let result = (|| {
        let catalog = load_catalog_for_target(target, |family| {
            let roots = roots.context("Vanilla pack roots are unavailable")?;
            roots.load_window_icon_catalog(set, family)
        })?
        .context("window icon target unexpectedly skipped catalog preparation")?;
        apply_catalog(window, target, catalog)
    })();
    match result {
        Ok(applied) => tracing::info!(
            window_icon_status = "applied",
            window_icon_target = target.label(),
            window_icon_set = ?set,
            window_icon_resources = applied.resource_count,
            window_icon_primary_size = ?applied.primary_size,
            window_icon_taskbar_size = ?applied.taskbar_size,
            "native window icon applied"
        ),
        Err(err) => tracing::warn!(
            ?err,
            window_icon_status = "fallback",
            window_icon_target = target.label(),
            window_icon_set = ?set,
            "continuing with platform default window icon"
        ),
    }
}

fn load_catalog_for_target(
    target: WindowIconTarget,
    load: impl FnOnce(WindowIconFamily) -> Result<WindowIconCatalog>,
) -> Result<Option<WindowIconCatalog>> {
    target.family().map(load).transpose()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AppliedWindowIcon {
    resource_count: usize,
    primary_size: Option<u32>,
    taskbar_size: Option<u32>,
}

fn apply_catalog(
    window: &Window,
    target: WindowIconTarget,
    catalog: WindowIconCatalog,
) -> Result<AppliedWindowIcon> {
    match (target, catalog) {
        (WindowIconTarget::X11, WindowIconCatalog::Standard(icons)) => {
            let mut icons = prepare_standard_icons(icons)?;
            let resource_count = icons.len();
            let primary = icons
                .remove(&256)
                .context("prepared window icon family is missing 256px icon")?;
            window.set_window_icon(Some(primary));
            Ok(AppliedWindowIcon {
                resource_count,
                primary_size: Some(256),
                taskbar_size: None,
            })
        }
        (WindowIconTarget::Windows, WindowIconCatalog::Standard(icons)) => {
            let mut icons = prepare_standard_icons(icons)?;
            let resource_count = icons.len();
            let primary = icons
                .remove(&32)
                .context("prepared window icon family is missing 32px icon")?;
            let taskbar = icons
                .remove(&256)
                .context("prepared window icon family is missing 256px icon")?;
            window.set_window_icon(Some(primary));
            apply_windows_taskbar_icon(window, taskbar);
            Ok(AppliedWindowIcon {
                resource_count,
                primary_size: Some(32),
                taskbar_size: Some(256),
            })
        }
        (WindowIconTarget::MacOs, WindowIconCatalog::MacOs(icon)) => {
            apply_macos_application_icon(icon.into_bytes())?;
            Ok(AppliedWindowIcon {
                resource_count: 1,
                primary_size: None,
                taskbar_size: None,
            })
        }
        (WindowIconTarget::SkipWayland | WindowIconTarget::SkipUnsupported, _) => {
            bail!("skipped window icon target reached platform apply")
        }
        (target, _) => bail!(
            "window icon catalog family does not match target {}",
            target.label()
        ),
    }
}

fn prepare_standard_icons(
    icons: Vec<StandardWindowIcon>,
) -> Result<std::collections::BTreeMap<u32, Icon>> {
    let mut prepared = std::collections::BTreeMap::new();
    for icon in icons {
        let (requested_size, [width, height], rgba) = icon.into_native_parts();
        let native = Icon::from_rgba(rgba, width, height)
            .with_context(|| format!("prepare {requested_size}px native window icon"))?;
        if prepared.insert(requested_size, native).is_some() {
            bail!("duplicate {requested_size}px native window icon");
        }
    }
    Ok(prepared)
}

#[cfg(target_os = "windows")]
fn apply_windows_taskbar_icon(window: &Window, icon: Icon) {
    use winit::platform::windows::WindowExtWindows;
    window.set_taskbar_icon(Some(icon));
}

#[cfg(not(target_os = "windows"))]
fn apply_windows_taskbar_icon(_window: &Window, _icon: Icon) {}

#[cfg(target_os = "macos")]
fn apply_macos_application_icon(bytes: Vec<u8>) -> Result<()> {
    use objc2::{AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    let marker =
        MainThreadMarker::new().context("window icon apply is not on the macOS main thread")?;
    let data = NSData::with_bytes(&bytes);
    let image = NSImage::initWithData(NSImage::alloc(), &data)
        .context("AppKit rejected the Vanilla ICNS window icon")?;
    let application = NSApplication::sharedApplication(marker);
    // SAFETY: AppKit documents a non-null NSImage for this setter; `image` is retained for the
    // duration of the call and AppKit retains the application icon as needed.
    unsafe { application.setApplicationIconImage(Some(&image)) };
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn apply_macos_application_icon(_bytes: Vec<u8>) -> Result<()> {
    bail!("macOS application icon apply is unavailable on this target")
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use image::{Rgba, RgbaImage};

    use super::*;

    const STANDARD_ICON_SIZES: [u32; 5] = [16, 32, 48, 128, 256];

    #[test]
    fn platform_target_preserves_vanilla_family_and_wayland_skip() {
        assert_eq!(
            WindowIconTarget::for_platform("linux", NativeWindowBackend::Wayland),
            WindowIconTarget::SkipWayland
        );
        assert_eq!(
            WindowIconTarget::for_platform("linux", NativeWindowBackend::X11).family(),
            Some(WindowIconFamily::Standard)
        );
        assert_eq!(
            WindowIconTarget::for_platform("windows", NativeWindowBackend::Other),
            WindowIconTarget::Windows
        );
        assert_eq!(
            WindowIconTarget::for_platform("macos", NativeWindowBackend::Other).family(),
            Some(WindowIconFamily::MacOs)
        );
    }

    #[test]
    fn skipped_target_does_not_invoke_catalog_loader() {
        let calls = AtomicUsize::new(0);
        let target = WindowIconTarget::for_platform("linux", NativeWindowBackend::Wayland);
        let catalog = load_catalog_for_target(target, |_| {
            calls.fetch_add(1, Ordering::Relaxed);
            bail!("loader must not run")
        })
        .unwrap();
        assert!(catalog.is_none());
        assert_eq!(calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn standard_catalog_converts_all_five_icons_before_platform_selection() {
        let fixture = Fixture::new("prepare");
        fixture.write_standard_icons();
        let WindowIconCatalog::Standard(icons) = fixture
            .roots()
            .load_window_icon_catalog(WindowIconSet::Release, WindowIconFamily::Standard)
            .unwrap()
        else {
            panic!("expected standard window icons");
        };
        let prepared = prepare_standard_icons(icons).unwrap();
        assert_eq!(
            prepared.keys().copied().collect::<Vec<_>>(),
            STANDARD_ICON_SIZES
        );
    }

    #[test]
    fn production_source_has_one_ordered_startup_call_and_no_icon_path_resolver() {
        let main = include_str!("main.rs");
        let owner = include_str!("window_icon.rs");
        assert_eq!(main.matches("install_startup_window_icon(").count(), 1);
        let catalog_call = ["load_window_icon_", "catalog"].concat();
        let production_owner = owner.split("#[cfg(test)]").next().unwrap();
        assert_eq!(production_owner.matches(&catalog_call).count(), 1);
        let build = main.find("let window = build_window(").unwrap();
        let install = main.find("install_startup_window_icon(").unwrap();
        let ime = main.find("window.set_ime_allowed(true)").unwrap();
        let input = main
            .find("ClientInputState::new(window.has_focus())")
            .unwrap();
        assert!(build < install && install < ime && ime < input);
        assert!(main.contains("window_backend = %window_backend.label()"));
        assert!(!main.contains("icons/icon_"));
        assert!(!main.contains("minecraft.icns"));
    }

    struct Fixture {
        root: PathBuf,
        sources: PathBuf,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "bbb-native-window-icon-{}-{label}-{nonce}",
                std::process::id()
            ));
            let sources = root.join("sources/26.1");
            std::fs::create_dir_all(&sources).unwrap();
            Self { root, sources }
        }

        fn roots(&self) -> PackRoots {
            PackRoots::from_paths(
                self.root.clone(),
                self.sources.clone(),
                self.sources.join("assets/minecraft"),
            )
        }

        fn write_standard_icons(&self) {
            for size in STANDARD_ICON_SIZES {
                let path = self.sources.join(format!("icons/icon_{size}x{size}.png"));
                write_png(&path, size, size);
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn write_png(path: &Path, width: u32, height: u32) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        RgbaImage::from_pixel(width, height, Rgba([1, 2, 3, 255]))
            .save(path)
            .unwrap();
    }
}
