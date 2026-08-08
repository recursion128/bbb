use anyhow::{bail, Context, Result};
use bbb_pack::{PackRoots, StandardWindowIcon, WindowIconCatalog, WindowIconFamily, WindowIconSet};
#[cfg(target_os = "linux")]
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::{
    raw_window_handle::{HasDisplayHandle, RawDisplayHandle},
    window::Window,
};

const STANDARD_ICON_SIZES: [u32; 5] = [16, 32, 48, 128, 256];

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

    /// `Window.getPlatform` (Window.java:144-158): the GLFW platform identity.
    /// winit has no GLFW "null" backend, so a display handle that is neither
    /// X11 nor Wayland maps to the unknown label; non-Linux targets are named
    /// by `target_os`. bbb supports macOS and Linux, so Vanilla's `win32`
    /// identity has no target that could report it.
    pub(crate) fn platform_identity(self, target_os: &str) -> &'static str {
        match target_os {
            "macos" => "cocoa",
            "linux" => match self {
                Self::Wayland => "wayland",
                Self::X11 => "x11",
                Self::Other => "unknown",
            },
            _ => "unknown",
        }
    }

    /// Whether this backend can enter `Fullscreen::Exclusive`. Wayland cannot
    /// (winit `Window::set_fullscreen` docs: "Wayland does not support
    /// exclusive fullscreen mode and will no-op a request"), so BBB selects
    /// borderless there; winit itself decides on the remaining backends.
    pub(crate) const fn supports_exclusive_fullscreen(self) -> bool {
        !matches!(self, Self::Wayland)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowIconTarget {
    SkipWayland,
    SkipUnsupported,
    X11,
    MacOs,
}

impl WindowIconTarget {
    fn for_platform(target_os: &str, backend: NativeWindowBackend) -> Self {
        match target_os {
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
            Self::X11 => Some(WindowIconFamily::Standard),
            Self::MacOs => Some(WindowIconFamily::MacOs),
            Self::SkipWayland | Self::SkipUnsupported => None,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::SkipWayland => "wayland-skip",
            Self::SkipUnsupported => "unsupported-skip",
            Self::X11 => "x11",
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
            let property = prepare_standard_icons(icons)?;
            apply_x11_icon_property(window, &property.cardinals)?;
            Ok(AppliedWindowIcon {
                resource_count: property.resource_count,
                primary_size: property.primary_size,
                taskbar_size: None,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct X11IconProperty {
    cardinals: Vec<u32>,
    resource_count: usize,
    primary_size: Option<u32>,
}

fn prepare_standard_icons(icons: Vec<StandardWindowIcon>) -> Result<X11IconProperty> {
    let mut requested_sizes = Vec::with_capacity(icons.len());
    let mut cardinals = Vec::new();
    for icon in icons {
        let (requested_size, [width, height], rgba) = icon.into_native_parts();
        let pixel_count = width
            .checked_mul(height)
            .and_then(|count| usize::try_from(count).ok())
            .context("window icon dimensions exceed the host address space")?;
        let expected_bytes = pixel_count
            .checked_mul(4)
            .context("window icon RGBA byte count overflow")?;
        if rgba.len() != expected_bytes {
            bail!(
                "{requested_size}px window icon declares {width}x{height} but contains {} RGBA bytes",
                rgba.len()
            );
        }
        requested_sizes.push(requested_size);
        cardinals.push(width);
        cardinals.push(height);
        cardinals.extend(rgba.chunks_exact(4).map(|pixel| {
            u32::from(pixel[3]) << 24
                | u32::from(pixel[0]) << 16
                | u32::from(pixel[1]) << 8
                | u32::from(pixel[2])
        }));
    }
    if requested_sizes != STANDARD_ICON_SIZES {
        bail!(
            "standard window icon family has sizes {requested_sizes:?}, expected {STANDARD_ICON_SIZES:?}"
        );
    }
    Ok(X11IconProperty {
        cardinals,
        resource_count: requested_sizes.len(),
        primary_size: requested_sizes.last().copied(),
    })
}

#[cfg(target_os = "linux")]
fn apply_x11_icon_property(window: &Window, cardinals: &[u32]) -> Result<()> {
    use std::os::raw::{c_int, c_ulong};

    use x11_dl::xlib;

    let display = match window.display_handle()?.as_raw() {
        RawDisplayHandle::Xlib(handle) => handle
            .display
            .context("X11 display handle has no Xlib display pointer")?
            .as_ptr()
            .cast::<xlib::Display>(),
        other => bail!("X11 icon target has non-Xlib display handle {other:?}"),
    };
    let xwindow = match window.window_handle()?.as_raw() {
        RawWindowHandle::Xlib(handle) => handle.window,
        other => bail!("X11 icon target has non-Xlib window handle {other:?}"),
    };
    let property_len = c_int::try_from(cardinals.len())
        .context("X11 window icon property exceeds the Xlib item-count range")?;
    let native_cardinals = cardinals
        .iter()
        .copied()
        .map(c_ulong::from)
        .collect::<Vec<_>>();
    let xlib = xlib::Xlib::open().map_err(|err| anyhow::anyhow!("load Xlib: {err}"))?;

    // SAFETY: winit owns a live Xlib display/window pair for the X11 backend,
    // the property data remains allocated through both calls, and Xlib's
    // format-32 ABI consumes one native `unsigned long` per CARDINAL.
    unsafe {
        let property = (xlib.XInternAtom)(display, c"_NET_WM_ICON".as_ptr(), 0);
        if property == 0 {
            bail!("XInternAtom returned None for _NET_WM_ICON");
        }
        (xlib.XChangeProperty)(
            display,
            xwindow,
            property,
            xlib::XA_CARDINAL,
            32,
            xlib::PropModeReplace,
            native_cardinals.as_ptr().cast(),
            property_len,
        );
        (xlib.XFlush)(display);
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn apply_x11_icon_property(_window: &Window, _cardinals: &[u32]) -> Result<()> {
    bail!("X11 window icon apply is only available on Linux")
}

/// The AppKit call itself lives in `bbb-platform-macos`, whose closure
/// cross-compiles from Linux; keeping it here would leave it in the one part of
/// the tree no gate on the dev host ever compiles.
fn apply_macos_application_icon(bytes: Vec<u8>) -> Result<()> {
    bbb_platform_macos::set_application_icon(&bytes)
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
            WindowIconTarget::for_platform("macos", NativeWindowBackend::Other).family(),
            Some(WindowIconFamily::MacOs)
        );
        // Windows is a declared non-goal and the build refuses that target, so
        // it must not acquire an icon path by falling into a live arm.
        assert_eq!(
            WindowIconTarget::for_platform("windows", NativeWindowBackend::Other),
            WindowIconTarget::SkipUnsupported
        );
    }

    #[test]
    fn platform_identity_matches_glfw_platform_names() {
        assert_eq!(
            NativeWindowBackend::Wayland.platform_identity("linux"),
            "wayland"
        );
        assert_eq!(NativeWindowBackend::X11.platform_identity("linux"), "x11");
        assert_eq!(
            NativeWindowBackend::Other.platform_identity("linux"),
            "unknown"
        );
        assert_eq!(
            NativeWindowBackend::Other.platform_identity("macos"),
            "cocoa"
        );
        assert_eq!(
            NativeWindowBackend::Other.platform_identity("windows"),
            "unknown"
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
    fn x11_property_preserves_all_five_icons_and_argb_order() {
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
        assert_eq!(prepared.resource_count, STANDARD_ICON_SIZES.len());
        assert_eq!(prepared.primary_size, Some(256));
        let mut cursor = 0;
        for size in STANDARD_ICON_SIZES {
            assert_eq!(&prepared.cardinals[cursor..cursor + 2], &[size, size]);
            assert_eq!(prepared.cardinals[cursor + 2], 0xff01_0203);
            cursor += 2 + usize::try_from(size * size).unwrap();
        }
        assert_eq!(cursor, prepared.cardinals.len());
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
        // Startup revokes the IME rather than allowing it: vanilla's
        // `TextInputManager` only turns it on while a text field has focus.
        assert!(!main.contains("window.set_ime_allowed(true)"));
        let ime = main.find("window.set_ime_allowed(false)").unwrap();
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
