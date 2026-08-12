//! Vanilla 26.1 window-family decision rules that are platform-independent and
//! repo-owned.
//!
//! winit owns the native window/surface/event delivery and wgpu owns the
//! present path; this module owns the rules BBB must reproduce from
//! `Window.java` and `Minecraft.java`: minimum nonzero window dimensions,
//! GUI-scale calculation/projection, the preferred fullscreen video-mode
//! parse/match/fallback, the selection line width, refresh-rate observation,
//! cursor-change gating and the window title.

use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    monitor::{MonitorHandle, VideoModeHandle},
    window::CursorIcon,
};

/// `Options.MAX_GUI_SCALE_INCLUSIVE` (Options.java:852-859): the accepted
/// upper bound of the stored `guiScale` option. `Window.calculateScale` stops
/// at the cap anyway, so a huge manual cap is equivalent to auto until the
/// framebuffer cannot fit 320x240 at the next scale.
pub(crate) const MAX_GUI_SCALE: u32 = 2_147_483_646;

/// Validates the stored or typed `guiScale` option against Vanilla's accepted
/// range (`Options.guiScale`, Options.java:853-865): `0` (auto) through
/// `MAX_GUI_SCALE` inclusive.
pub(crate) fn parse_gui_scale(value: &str) -> std::result::Result<u32, String> {
    let parsed: u32 = value
        .parse()
        .map_err(|_| format!("GUI scale must be an integer, got {value}"))?;
    if parsed > MAX_GUI_SCALE {
        return Err(format!(
            "GUI scale {parsed} exceeds Vanilla's accepted maximum {MAX_GUI_SCALE}"
        ));
    }
    Ok(parsed)
}

/// `Window.BASE_WIDTH` / `Window.BASE_HEIGHT` (Window.java:35-36): the minimum
/// GUI resolution the scale loop preserves.
pub(crate) const BASE_WIDTH: u32 = 320;
pub(crate) const BASE_HEIGHT: u32 = 240;

/// `Window.allowedWindowMinSize(int)` (Window.java:551): every stored window
/// dimension is clamped to at least one, so a degenerate display size never
/// produces a zero-width window.
pub(crate) fn allowed_window_min_size(size: u32) -> u32 {
    size.max(1)
}

/// `Window.calculateScale` (Window.java:411-426): starts at 1 and increments
/// while the next scale still leaves at least 320x240 GUI pixels; a
/// `max_scale` of 0 (auto) never stops the loop early.
pub(crate) fn calculate_gui_scale(
    max_scale: u32,
    enforce_unicode: bool,
    framebuffer_width: u32,
    framebuffer_height: u32,
) -> u32 {
    let mut scale = 1_u32;
    while scale != max_scale
        && scale < framebuffer_width
        && scale < framebuffer_height
        && framebuffer_width / (scale + 1) >= BASE_WIDTH
        && framebuffer_height / (scale + 1) >= BASE_HEIGHT
    {
        scale += 1;
    }
    if enforce_unicode && scale % 2 != 0 {
        scale += 1;
    }
    scale
}

/// `Window.setGuiScale` (Window.java:431-439): the projected GUI dimensions use
/// ceiling division, so a 1920x1080 framebuffer at scale 3 projects to
/// 640x360 rather than 639x359.
pub(crate) fn gui_scaled_size(
    framebuffer_width: u32,
    framebuffer_height: u32,
    gui_scale: u32,
) -> (u32, u32) {
    (
        framebuffer_width.div_ceil(gui_scale),
        framebuffer_height.div_ceil(gui_scale),
    )
}

/// `Window.getAppropriateLineWidth` (Window.java:543): the block-outline /
/// hitbox line width scales linearly from 2.5 at 1920 wide and never falls
/// below 2.5.
pub(crate) fn appropriate_line_width(framebuffer_width: u32) -> f32 {
    (framebuffer_width as f32 / 1920.0 * 2.5).max(2.5)
}

/// Winit reports monitor refresh rates in millihertz while GLFW's
/// `Window.getRefreshRate` (Window.java:158) returns integer hertz. Rounding
/// half up keeps a 59.999 Hz mode from degrading to 59 when the source is a
/// 60 Hz mode with sub-millihertz error. Saturating addition keeps a
/// degenerate `u32::MAX` millihertz source from overflowing before the
/// division.
pub(crate) fn refresh_rate_hz(refresh_rate_millihertz: u32) -> u32 {
    refresh_rate_millihertz.saturating_add(500) / 1000
}

/// Normalize winit's platform pixel encoding to GLFW's RGB component sum.
/// On macOS, winit maps `IO32BitDirectPixels` to 32, while GLFW exposes the
/// same 8-bit RGB mode as red+green+blue = 24. The alpha/storage byte must not
/// prevent Vanilla's `fullscreenResolution` from matching that native mode.
pub(crate) const fn vanilla_total_color_bits(winit_bit_depth: u16) -> u32 {
    #[cfg(target_os = "macos")]
    if winit_bit_depth == 32 {
        return 24;
    }
    winit_bit_depth as u32
}

/// The winit `VideoModeHandle` surface facts `Monitor.getPreferredVidMode`
/// (Monitor.java:45-57) compares: resolution, refresh rate in hertz, and the
/// total colour bits after the platform encoding is normalized to GLFW's
/// `redBits + greenBits + blueBits` convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VideoModeFacts {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) refresh_rate_hz: u32,
    pub(crate) total_bits: u32,
}

impl VideoModeFacts {
    pub(crate) fn from_winit(video_mode: &VideoModeHandle) -> Self {
        Self {
            width: video_mode.size().width,
            height: video_mode.size().height,
            refresh_rate_hz: refresh_rate_hz(video_mode.refresh_rate_millihertz()),
            total_bits: vanilla_total_color_bits(video_mode.bit_depth()),
        }
    }

    /// `VideoMode.equals` (VideoMode.java:88-97): exact match on width, height,
    /// refresh rate and total colour bits.
    pub(crate) fn exactly_matches(self, preferred: PreferredVideoMode) -> bool {
        self.width == preferred.width
            && self.height == preferred.height
            && self.refresh_rate_hz == preferred.refresh_rate_hz
            && self.total_bits == preferred.total_bits
    }
}

/// One stored `fullscreenResolution` line (`VideoMode.read`, VideoMode.java).
/// The accepted form is `(digits)x(digits)[@(digits)[:(digits)]]`; an omitted
/// refresh defaults to 60 and an omitted total colour-bit count defaults to
/// 24. Vanilla splits the total into three equal components (`bits / 3`), so a
/// non-multiple-of-three total rounds down to the component-truncated total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PreferredVideoMode {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) refresh_rate_hz: u32,
    pub(crate) total_bits: u32,
}

impl PreferredVideoMode {
    pub(crate) fn parse(raw: &str) -> Option<Self> {
        let (resolution, suffix) = raw
            .split_once('@')
            .map(|(resolution, suffix)| (resolution, Some(suffix)))
            .unwrap_or((raw, None));
        let (width, height) = resolution.split_once('x')?;
        let width = parse_vanilla_int(width)?;
        let height = parse_vanilla_int(height)?;
        let (refresh_rate_hz, total_bits) = match suffix {
            None => (60, 24),
            Some("") => return None,
            Some(suffix) => match suffix.split_once(':') {
                Some((refresh, bits)) if !refresh.is_empty() && !bits.is_empty() => {
                    (parse_vanilla_int(refresh)?, parse_vanilla_int(bits)?)
                }
                Some(_) => return None,
                None => (parse_vanilla_int(suffix)?, 24),
            },
        };
        Some(Self {
            width,
            height,
            refresh_rate_hz,
            total_bits: (total_bits / 3) * 3,
        })
    }

    /// `VideoMode.write` (VideoMode.java:131-134): the round-trippable line
    /// `widthxheight@refresh:totalBits`.
    #[cfg(test)]
    pub(crate) fn write(self) -> String {
        format!(
            "{}x{}@{}:{}",
            self.width, self.height, self.refresh_rate_hz, self.total_bits
        )
    }
}

/// `Integer.parseInt` bounds: `VideoMode.read` rejects any field outside the
/// signed 32-bit range, and BBB stores these in bounded `u32` types, so
/// `i32::MAX` is the accepted ceiling.
fn parse_vanilla_int(raw: &str) -> Option<u32> {
    let parsed: u32 = raw.parse().ok()?;
    (parsed <= i32::MAX as u32).then_some(parsed)
}

/// The pure decision half of `Monitor.getPreferredVidMode`
/// (Monitor.java:45-57) plus the backend policy: an exact preferred mode wins
/// only when the backend supports exclusive mode; every other path falls back
/// deterministically to borderless on the current monitor (winit has no
/// `current_video_mode` accessor, so the borderless surface at the desktop
/// mode is the supported equivalent of GLFW's current-mode fallback). Wayland
/// cannot enter exclusive mode at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FullscreenChoice {
    Exclusive,
    Borderless,
}

pub(crate) fn fullscreen_choice(
    preferred: Option<PreferredVideoMode>,
    exact_match_available: bool,
    exclusive_supported: bool,
) -> FullscreenChoice {
    if exclusive_supported && preferred.is_some() && exact_match_available {
        FullscreenChoice::Exclusive
    } else {
        FullscreenChoice::Borderless
    }
}

/// Enumerates `MonitorHandle::video_modes()` and returns the first exact
/// match, mirroring `Monitor.getPreferredVidMode`.
pub(crate) fn find_exact_video_mode<'m>(
    preferred: PreferredVideoMode,
    monitor: Option<&'m MonitorHandle>,
) -> Option<VideoModeHandle> {
    monitor?.video_modes().find_map(|mode| {
        VideoModeFacts::from_winit(&mode)
            .exactly_matches(preferred)
            .then_some(mode)
    })
}

/// `MouseHandler.getScaledXPos/getScaledYPos` (MouseHandler.java:337-342):
/// projects one physical window coordinate onto the logical GUI surface with
/// the exact ratio `x * guiWidth / screenWidth` (and the same for y). Ceiling
/// division makes `gui_width / screen_width` differ from an integer scale at
/// e.g. 1280x720 / scale 3, so this is never replaced by `x / scale`.
pub(crate) fn project_cursor_to_gui(
    position: PhysicalPosition<f64>,
    physical: PhysicalSize<u32>,
    gui: PhysicalSize<u32>,
) -> PhysicalPosition<f64> {
    PhysicalPosition::new(
        position.x * f64::from(gui.width) / f64::from(physical.width.max(1)),
        position.y * f64::from(gui.height) / f64::from(physical.height.max(1)),
    )
}

/// The event-loop cursor cache (`MouseHandler.xpos/ypos`,
/// MouseHandler.java:39-40): raw physical window coordinates, retained
/// verbatim across GUI-projection changes. `MouseHandler.getScaledXPos/
/// getScaledYPos` (MouseHandler.java:337-342) recompute the projection from
/// the *current* physical and logical sizes at every use, so a resize without
/// another `CursorMoved` must never leave a stale logical coordinate behind.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CursorPositionCache {
    physical: Option<PhysicalPosition<f64>>,
}

impl CursorPositionCache {
    /// `MouseHandler.onMove` (MouseHandler.java:266-279): store the raw
    /// physical position; the GUI projection is derived at each use.
    pub(crate) fn on_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        self.physical = Some(position);
    }

    pub(crate) fn on_cursor_left(&mut self) {
        self.physical = None;
    }

    /// `MouseHandler.getScaledXPos/getScaledYPos`: project the raw physical
    /// position onto the current logical GUI surface.
    pub(crate) fn project(
        &self,
        physical: PhysicalSize<u32>,
        gui: PhysicalSize<u32>,
    ) -> Option<PhysicalPosition<f64>> {
        self.physical
            .map(|position| project_cursor_to_gui(position, physical, gui))
    }

    /// The raw cache value, for tests that pin the physical-retention
    /// invariant.
    #[cfg(test)]
    pub(crate) fn physical(&self) -> Option<PhysicalPosition<f64>> {
        self.physical
    }
}

/// Converts a logical-GUI rectangle back into physical window coordinates for
/// native consumers (the IME cursor rectangle). The inverse of
/// [`project_cursor_to_gui`]'s ratio; a physical size of zero (minimized)
/// keeps the rectangle empty rather than dividing by zero.
pub(crate) fn gui_rect_to_physical(
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    gui: PhysicalSize<u32>,
    physical: PhysicalSize<u32>,
) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>)> {
    if physical.width == 0 || physical.height == 0 {
        return None;
    }
    let scale_x = f64::from(physical.width) / f64::from(gui.width.max(1));
    let scale_y = f64::from(physical.height) / f64::from(gui.height.max(1));
    Some((
        PhysicalPosition::new(
            (f64::from(position.x) * scale_x).floor() as i32,
            (f64::from(position.y) * scale_y).floor() as i32,
        ),
        PhysicalSize::new(
            (f64::from(size.width) * scale_x).ceil() as u32,
            (f64::from(size.height) * scale_y).ceil() as u32,
        ),
    ))
}

/// `Window.selectCursor` (Window.java:535-540): unless `allowCursorChanges` is
/// enabled the effective cursor is always `Default`; identical native cursor
/// changes are skipped. Returns `Some` only when the native cursor must
/// change.
pub(crate) fn select_cursor(
    allow_cursor_changes: bool,
    requested: CursorIcon,
    current: CursorIcon,
) -> Option<CursorIcon> {
    let effective = if allow_cursor_changes {
        requested
    } else {
        CursorIcon::Default
    };
    (effective != current).then_some(effective)
}

/// `Minecraft.createTitle` (Minecraft.java:806-828) for the external client:
/// the client reports as modified (`Minecraft*`) and appends
/// ` - Multiplayer` (`title.multiplayer.other`) while the ordinary external
/// multiplayer connection is live. The singleplayer, LAN and Realms branches
/// are out of scope for this client.
pub(crate) fn create_title(version: &str, connected_to_external_multiplayer: bool) -> String {
    let mut title = format!("Minecraft* {version}");
    if connected_to_external_multiplayer {
        title.push_str(" - Multiplayer");
    }
    title
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_window_min_size_clamps_both_dimensions_to_one() {
        assert_eq!(allowed_window_min_size(1280), 1280);
        assert_eq!(allowed_window_min_size(1), 1);
        assert_eq!(allowed_window_min_size(0), 1);
    }

    #[test]
    fn calculate_gui_scale_matches_vanilla_progression() {
        // Manual max scale stops the loop at the cap.
        assert_eq!(calculate_gui_scale(2, false, 1920, 1080), 2);
        // Auto (0) grows while the next scale still fits 320x240.
        assert_eq!(calculate_gui_scale(0, false, 1920, 1080), 4);
        assert_eq!(calculate_gui_scale(0, false, 1280, 720), 3);
        assert_eq!(calculate_gui_scale(0, false, 640, 480), 2);
        // A framebuffer too small for scale 2 keeps scale 1.
        assert_eq!(calculate_gui_scale(0, false, 639, 480), 1);
        assert_eq!(calculate_gui_scale(0, false, 640, 479), 1);
    }

    #[test]
    fn calculate_gui_scale_enforce_unicode_rounds_odd_scale_up() {
        assert_eq!(calculate_gui_scale(0, true, 1920, 1080), 4);
        assert_eq!(calculate_gui_scale(3, true, 1920, 1080), 4);
        assert_eq!(calculate_gui_scale(3, false, 1920, 1080), 3);
        assert_eq!(calculate_gui_scale(0, true, 1280, 720), 4);
    }

    #[test]
    fn gui_scaled_size_uses_ceiling_division() {
        assert_eq!(gui_scaled_size(1920, 1080, 3), (640, 360));
        assert_eq!(gui_scaled_size(1920, 1080, 4), (480, 270));
        assert_eq!(gui_scaled_size(1280, 720, 3), (427, 240));
        assert_eq!(gui_scaled_size(640, 480, 2), (320, 240));
    }

    #[test]
    fn appropriate_line_width_scales_from_1920_and_never_drops_below_2_5() {
        assert_eq!(appropriate_line_width(1920), 2.5);
        assert_eq!(appropriate_line_width(3840), 5.0);
        assert_eq!(appropriate_line_width(960), 2.5);
        assert_eq!(appropriate_line_width(0), 2.5);
    }

    #[test]
    fn refresh_rate_hz_rounds_millihertz_to_glfw_hertz() {
        assert_eq!(refresh_rate_hz(60_000), 60);
        assert_eq!(refresh_rate_hz(59_999), 60);
        assert_eq!(refresh_rate_hz(59_500), 60);
        assert_eq!(refresh_rate_hz(59_499), 59);
        assert_eq!(refresh_rate_hz(144_000), 144);
        // Saturating addition: `u32::MAX + 500` must not wrap.
        assert_eq!(refresh_rate_hz(u32::MAX), 4_294_967);
    }

    #[test]
    fn parse_gui_scale_accepts_vanilla_range_and_rejects_above_the_bound() {
        assert_eq!(parse_gui_scale("0"), Ok(0));
        assert_eq!(parse_gui_scale("4"), Ok(4));
        assert_eq!(parse_gui_scale("2147483646"), Ok(2_147_483_646));
        assert!(parse_gui_scale("2147483647").is_err());
        assert!(parse_gui_scale("-1").is_err());
        assert!(parse_gui_scale("auto").is_err());
        assert!(parse_gui_scale("").is_err());
    }

    #[test]
    fn preferred_video_mode_parses_valid_forms_with_vanilla_defaults() {
        assert_eq!(
            PreferredVideoMode::parse("1920x1080"),
            Some(PreferredVideoMode {
                width: 1920,
                height: 1080,
                refresh_rate_hz: 60,
                total_bits: 24,
            })
        );
        assert_eq!(
            PreferredVideoMode::parse("1920x1080@144"),
            Some(PreferredVideoMode {
                width: 1920,
                height: 1080,
                refresh_rate_hz: 144,
                total_bits: 24,
            })
        );
        assert_eq!(
            PreferredVideoMode::parse("1920x1080@144:24"),
            Some(PreferredVideoMode {
                width: 1920,
                height: 1080,
                refresh_rate_hz: 144,
                total_bits: 24,
            })
        );
        assert_eq!(
            PreferredVideoMode::parse("3840x2160@60:32"),
            Some(PreferredVideoMode {
                width: 3840,
                height: 2160,
                refresh_rate_hz: 60,
                // `bits / 3` truncates per component, so a stored 32 becomes
                // 10+10+10 = 30 total bits (VideoMode.read).
                total_bits: 30,
            })
        );
        // Vanilla truncates per-component bits (`bits / 3`), so 25 becomes 24.
        assert_eq!(
            PreferredVideoMode::parse("1280x720@120:25"),
            Some(PreferredVideoMode {
                width: 1280,
                height: 720,
                refresh_rate_hz: 120,
                total_bits: 24,
            })
        );
    }

    #[test]
    fn preferred_video_mode_rejects_invalid_and_out_of_bounds_forms() {
        for invalid in [
            "",
            "1920",
            "x1080",
            "1920x",
            "1920x1080@",
            "1920x1080@144:",
            "1920x1080@:24",
            "1920x1080@144:24:8",
            "1920x1080@abc",
            "1920x1080@-60",
            "1920x1080@2147483648",
            "1920x1080@60:2147483648",
        ] {
            assert_eq!(PreferredVideoMode::parse(invalid), None, "{invalid}");
        }
    }

    #[test]
    fn preferred_video_mode_writes_the_round_trippable_line() {
        let mode = PreferredVideoMode::parse("1920x1080@144:24").unwrap();
        assert_eq!(mode.write(), "1920x1080@144:24");
        assert_eq!(PreferredVideoMode::parse(&mode.write()), Some(mode));
    }

    #[test]
    fn video_mode_facts_exact_match_includes_refresh_and_total_bits() {
        let preferred = PreferredVideoMode {
            width: 1920,
            height: 1080,
            refresh_rate_hz: 144,
            total_bits: 24,
        };
        let exact = VideoModeFacts {
            width: 1920,
            height: 1080,
            refresh_rate_hz: 144,
            total_bits: 24,
        };
        assert!(exact.exactly_matches(preferred));
        assert!(!VideoModeFacts {
            refresh_rate_hz: 60,
            ..exact
        }
        .exactly_matches(preferred));
        assert!(!VideoModeFacts {
            total_bits: 32,
            ..exact
        }
        .exactly_matches(preferred));
        assert!(!VideoModeFacts {
            width: 1280,
            ..exact
        }
        .exactly_matches(preferred));
        assert!(!VideoModeFacts {
            height: 720,
            ..exact
        }
        .exactly_matches(preferred));
    }

    #[test]
    fn platform_video_mode_depth_uses_vanilla_glfw_rgb_bits() {
        #[cfg(target_os = "macos")]
        {
            assert_eq!(vanilla_total_color_bits(32), 24);
            assert_eq!(vanilla_total_color_bits(30), 30);
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(vanilla_total_color_bits(32), 32);
            assert_eq!(vanilla_total_color_bits(24), 24);
        }
    }

    #[test]
    fn fullscreen_choice_selects_exclusive_only_with_exact_mode_and_backend_support() {
        let preferred = PreferredVideoMode::parse("1920x1080@144:24");
        assert_eq!(
            fullscreen_choice(preferred, true, true),
            FullscreenChoice::Exclusive
        );
        // Unavailable mode falls back to borderless.
        assert_eq!(
            fullscreen_choice(preferred, false, true),
            FullscreenChoice::Borderless
        );
        // Wayland has no exclusive mode regardless of the stored preference.
        assert_eq!(
            fullscreen_choice(preferred, true, false),
            FullscreenChoice::Borderless
        );
        // No stored preference means borderless.
        assert_eq!(
            fullscreen_choice(None, true, true),
            FullscreenChoice::Borderless
        );
    }

    #[test]
    fn cursor_projection_uses_the_vanilla_ratio_not_integer_division() {
        // 1920x1080 at auto scale 4: physical center maps to logical center.
        let gui = PhysicalSize::new(480, 270);
        let physical = PhysicalSize::new(1920, 1080);
        assert_eq!(
            project_cursor_to_gui(PhysicalPosition::new(960.0, 540.0), physical, gui),
            PhysicalPosition::new(240.0, 135.0)
        );
        // 1280x720 at scale 3 projects 427x240; ceiling division makes the
        // ratio 427/1280, not 1/3.
        let gui = PhysicalSize::new(427, 240);
        let physical = PhysicalSize::new(1280, 720);
        assert_eq!(
            project_cursor_to_gui(PhysicalPosition::new(640.0, 360.0), physical, gui),
            PhysicalPosition::new(213.5, 120.0)
        );
        assert_eq!(
            project_cursor_to_gui(PhysicalPosition::new(1280.0, 720.0), physical, gui),
            PhysicalPosition::new(427.0, 240.0)
        );
    }

    #[test]
    fn cursor_cache_stays_physical_and_reprojects_after_gui_projection_changes() {
        let mut cache = CursorPositionCache::default();
        assert_eq!(
            cache.project(PhysicalSize::new(1920, 1080), PhysicalSize::new(480, 270)),
            None
        );

        cache.on_cursor_moved(PhysicalPosition::new(960.0, 540.0));
        // GUI projection A: scale 4 on a 1920x1080 framebuffer.
        assert_eq!(
            cache.project(PhysicalSize::new(1920, 1080), PhysicalSize::new(480, 270)),
            Some(PhysicalPosition::new(240.0, 135.0))
        );
        // The physical/logical sizes change without another CursorMoved; the
        // next click must use the new exact ratio (1280x720 at scale 3
        // projects 427x240), like `MouseHandler.getScaledXPos` at each use.
        assert_eq!(
            cache.project(PhysicalSize::new(1280, 720), PhysicalSize::new(427, 240)),
            Some(PhysicalPosition::new(320.25, 180.0))
        );
        // The raw cache never takes the projected value.
        assert_eq!(cache.physical(), Some(PhysicalPosition::new(960.0, 540.0)));

        cache.on_cursor_left();
        assert_eq!(cache.physical(), None);
    }

    #[test]
    fn gui_rect_to_physical_converts_back_at_the_platform_boundary() {
        let gui = PhysicalSize::new(480, 270);
        let physical = PhysicalSize::new(1920, 1080);
        assert_eq!(
            gui_rect_to_physical(
                PhysicalPosition::new(120, 40),
                PhysicalSize::new(16, 16),
                gui,
                physical,
            ),
            Some((PhysicalPosition::new(480, 160), PhysicalSize::new(64, 64),))
        );
        // A minimized (zero-size) surface keeps no native rectangle.
        assert_eq!(
            gui_rect_to_physical(
                PhysicalPosition::new(0, 0),
                PhysicalSize::new(16, 16),
                gui,
                PhysicalSize::new(0, 0),
            ),
            None
        );
    }

    #[test]
    fn select_cursor_forces_default_unless_allowed_and_skips_redundant_changes() {
        assert_eq!(
            select_cursor(true, CursorIcon::Pointer, CursorIcon::Default),
            Some(CursorIcon::Pointer)
        );
        assert_eq!(
            select_cursor(true, CursorIcon::Pointer, CursorIcon::Pointer),
            None
        );
        assert_eq!(
            select_cursor(false, CursorIcon::Pointer, CursorIcon::Default),
            None
        );
        assert_eq!(
            select_cursor(false, CursorIcon::Pointer, CursorIcon::Pointer),
            Some(CursorIcon::Default)
        );
    }

    #[test]
    fn create_title_marks_modified_client_and_appends_multiplayer_while_connected() {
        assert_eq!(
            create_title(bbb_protocol::MC_VERSION, false),
            format!("Minecraft* {}", bbb_protocol::MC_VERSION)
        );
        assert_eq!(
            create_title(bbb_protocol::MC_VERSION, true),
            format!("Minecraft* {} - Multiplayer", bbb_protocol::MC_VERSION)
        );
    }
}
