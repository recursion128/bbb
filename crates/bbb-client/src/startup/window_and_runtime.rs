use super::*;

/// Vanilla `FramerateLimitTracker.ICONIFIED_WINDOW_LIMIT` / `LONG_AFK_LIMIT`.
const VANILLA_ICONIFIED_FRAMERATE_LIMIT: u32 = 10;
/// Vanilla `FramerateLimitTracker.AFK_LIMIT`.
const VANILLA_SHORT_AFK_FRAMERATE_LIMIT: u32 = 30;
/// Vanilla `FramerateLimitTracker.OUT_OF_LEVEL_MENU_LIMIT`.
const VANILLA_OUT_OF_LEVEL_MENU_FRAMERATE_LIMIT: u32 = 60;
/// Vanilla `FramerateLimitTracker.AFK_THRESHOLD_MS` / `LONG_AFK_THRESHOLD_MS`.
const VANILLA_AFK_THRESHOLD_MS: u128 = 60_000;
const VANILLA_LONG_AFK_THRESHOLD_MS: u128 = 600_000;

/// Vanilla `FramerateLimitTracker.getFramerateLimit`: an iconified window and
/// a long AFK force 10 fps, a short AFK caps at 30, and a menu with no level
/// runs at exactly 60 — even above the configured limit. The window state is
/// checked before the AFK branches, which only fire with the option on `AFK`.
/// bbb has no vanilla `LoadingOverlay` equivalent, so the out-of-level branch
/// only tests the screen set.
pub(crate) fn effective_framerate_limit(
    framerate_limit: u32,
    inactivity_fps_limit: InactivityFpsLimitArg,
    window_iconified: bool,
    afk_millis: u128,
    level_loaded: bool,
    menu_screen_open: bool,
) -> u32 {
    if window_iconified {
        return VANILLA_ICONIFIED_FRAMERATE_LIMIT;
    }
    if inactivity_fps_limit == InactivityFpsLimitArg::Afk {
        if afk_millis > VANILLA_LONG_AFK_THRESHOLD_MS {
            return VANILLA_ICONIFIED_FRAMERATE_LIMIT;
        }
        if afk_millis > VANILLA_AFK_THRESHOLD_MS {
            return framerate_limit.min(VANILLA_SHORT_AFK_FRAMERATE_LIMIT);
        }
    }
    if !level_loaded && menu_screen_open {
        return VANILLA_OUT_OF_LEVEL_MENU_FRAMERATE_LIMIT;
    }
    framerate_limit
}

pub(crate) fn client_framerate_limit_interval(client_framerate_limit: u32) -> Option<Duration> {
    if client_framerate_limit == 0 || client_framerate_limit >= VANILLA_UNLIMITED_FRAMERATE_LIMIT {
        return None;
    }
    Some(Duration::from_nanos(
        (1_000_000_000.0 / f64::from(client_framerate_limit))
            .round()
            .clamp(1.0, u64::MAX as f64) as u64,
    ))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn start_control_api(
    runtime: &Runtime,
    addr: Option<SocketAddr>,
    control: &ControlState,
) {
    let Some(addr) = addr else {
        return;
    };

    let control = control.clone();
    runtime.spawn(async move {
        if let Err(err) = crate::control::serve(addr, control).await {
            tracing::error!(?err, "control API stopped");
        }
    });
}

pub(crate) fn create_event_loop() -> Result<EventLoop<()>> {
    Ok(EventLoop::new()?)
}

#[cfg(any(not(target_arch = "wasm32"), test))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WindowConfig {
    pub(crate) title: String,
    pub(crate) width: i32,
    pub(crate) height: i32,
}

#[cfg(any(not(target_arch = "wasm32"), test))]
impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "bbb-client".to_string(),
            width: 854,
            height: 480,
        }
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
impl WindowConfig {
    pub(crate) fn from_args(args: &Args) -> Self {
        Self {
            width: args.window_width,
            height: args.window_height,
            ..Self::default()
        }
    }

    pub(crate) fn physical_size(&self) -> PhysicalSize<u32> {
        // `Window.allowedWindowMinSize` (Window.java:551) clamps every stored
        // window dimension to at least one, so a degenerate display size can
        // never construct a zero-width native window.
        PhysicalSize::new(
            crate::window_surface::allowed_window_min_size(self.width.max(0) as u32),
            crate::window_surface::allowed_window_min_size(self.height.max(0) as u32),
        )
    }
}

/// `Window.createWindow`/`createGlfwWindow` collapsed into one owner. The
/// GLFW error plumbing around it — `checkGlfwError`'s error-queue drain,
/// `defaultErrorCallback`, and the `setBootErrorCallback`/
/// `setDefaultErrorCallback` install/swap pair — has no equivalent surface:
/// winit and wgpu return typed `Result`s at each call site instead of parking
/// errors in a process-global queue behind an installable callback, so every
/// failure is handled where it occurs and flows into the guarded crash
/// report.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn build_window(
    event_loop: &ActiveEventLoop,
    config: &WindowConfig,
    initial_fullscreen: Option<Fullscreen>,
) -> Result<Window> {
    let attributes = Window::default_attributes()
        .with_title(config.title.clone())
        .with_inner_size(config.physical_size())
        .with_fullscreen(initial_fullscreen);
    event_loop
        .create_window(attributes)
        .context("create native window")
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn vanilla_root_candidates(args: &Args) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for key in ["BBB_VANILLA_ROOT", "BBB_VANILLA_DIR"] {
        if let Some(dir) = std::env::var_os(key).filter(|v| !v.is_empty()) {
            candidates.push(PathBuf::from(dir));
        }
    }
    candidates.push(args.game_dir.clone());
    if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME").filter(|v| !v.is_empty()) {
        candidates.push(PathBuf::from(xdg).join("bbb").join("vanilla"));
    }
    super::platform::append_os_default_vanilla_candidates(&mut candidates);
    candidates
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn auto_discover_vanilla_root(args: &Args) -> Option<PathBuf> {
    if args.probe_server || args.vanilla_root.is_some() {
        return None;
    }
    vanilla_root_candidates(args)
        .into_iter()
        .find(|candidate| bbb_client_assets::PackRoots::has_usable_root(candidate))
}
