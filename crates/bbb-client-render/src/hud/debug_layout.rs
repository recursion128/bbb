use super::*;

pub(super) fn hud_debug_options_button_sprite_slot(
    active: bool,
    highlighted: bool,
) -> HudDebugOptionsButtonSpriteSlot {
    if !active {
        HudDebugOptionsButtonSpriteSlot::Disabled
    } else if highlighted {
        HudDebugOptionsButtonSpriteSlot::Highlighted
    } else {
        HudDebugOptionsButtonSpriteSlot::Normal
    }
}

pub(super) fn hud_debug_options_scrollbar_rects(
    surface_size: PhysicalSize<u32>,
    scroll_row: usize,
    total_rows: usize,
) -> Option<(HudDebugOptionsScrollbarRect, HudDebugOptionsScrollbarRect)> {
    let list_height = i32::try_from(surface_size.height)
        .unwrap_or(i32::MAX)
        .saturating_sub(HUD_DEBUG_OPTIONS_HEADER_HEIGHT + HUD_DEBUG_OPTIONS_FOOTER_HEIGHT);
    if list_height <= 8 || total_rows == 0 {
        return None;
    }
    let total_rows_i32 =
        i32::try_from(total_rows).unwrap_or(i32::MAX / HUD_DEBUG_OPTIONS_ROW_HEIGHT);
    let content_height = total_rows_i32.saturating_mul(HUD_DEBUG_OPTIONS_ROW_HEIGHT);
    if content_height <= list_height {
        return None;
    }
    let max_scroll_amount = content_height - list_height;
    let scroll_amount = i32::try_from(scroll_row)
        .unwrap_or(i32::MAX / HUD_DEBUG_OPTIONS_ROW_HEIGHT)
        .saturating_mul(HUD_DEBUG_OPTIONS_ROW_HEIGHT)
        .min(max_scroll_amount);
    let max_scroller_height = (list_height - 8).max(1);
    let min_scroller_height = HUD_DEBUG_OPTIONS_SCROLLBAR_MIN_HEIGHT.min(max_scroller_height);
    let scroller_height = list_height
        .saturating_mul(list_height)
        .checked_div(content_height)
        .unwrap_or(max_scroller_height)
        .clamp(min_scroller_height, max_scroller_height);
    let scrollable_track = (list_height - scroller_height).max(0);
    let scroller_y = HUD_DEBUG_OPTIONS_HEADER_HEIGHT
        + scroll_amount
            .saturating_mul(scrollable_track)
            .checked_div(max_scroll_amount)
            .unwrap_or(0);
    let scrollbar_x = hud_debug_options_content_x(surface_size)
        + HUD_DEBUG_OPTIONS_ROW_WIDTH
        + HUD_DEBUG_OPTIONS_SCROLLBAR_WIDTH
        + 2;

    Some((
        HudDebugOptionsScrollbarRect {
            x: scrollbar_x,
            y: HUD_DEBUG_OPTIONS_HEADER_HEIGHT,
            width: HUD_DEBUG_OPTIONS_SCROLLBAR_WIDTH as u32,
            height: list_height as u32,
        },
        HudDebugOptionsScrollbarRect {
            x: scrollbar_x,
            y: scroller_y,
            width: HUD_DEBUG_OPTIONS_SCROLLBAR_WIDTH as u32,
            height: scroller_height as u32,
        },
    ))
}

pub type DebugOptionsSearchBoxRects = ((i32, i32, u32, u32), (i32, i32, u32, u32));

pub fn hud_debug_options_search_box_rects(
    surface_size: PhysicalSize<u32>,
) -> DebugOptionsSearchBoxRects {
    let x = hud_debug_options_content_x(surface_size) + HUD_DEBUG_OPTIONS_ROW_WIDTH
        - HUD_DEBUG_OPTIONS_SEARCH_WIDTH;
    let y = 6;
    let width = HUD_DEBUG_OPTIONS_SEARCH_WIDTH.max(0) as u32;
    let height = HUD_DEBUG_OPTIONS_SEARCH_HEIGHT.max(0) as u32;
    (
        (x, y, width, height),
        (
            x + 1,
            y + 1,
            width.saturating_sub(2),
            height.saturating_sub(2),
        ),
    )
}

pub(super) fn hud_debug_options_search_display(
    search_text: &str,
    search_focused: bool,
    search_hint: &str,
) -> (String, Vec<HudStyledTextRun>) {
    if !search_text.is_empty() || search_focused {
        return (search_text.to_string(), Vec::new());
    }
    (
        search_hint.to_string(),
        vec![HudStyledTextRun {
            shadow_color: None,
            text: search_hint.to_string(),
            style: HudTextStyle {
                italic: true,
                ..HudTextStyle::default()
            },
            color: Some(0xAAAAAA),
        }],
    )
}

pub(super) fn hud_debug_options_button_label_tint(
    active: bool,
    text_tint: [f32; 4],
    use_inactive_text_tint: bool,
) -> [f32; 4] {
    if active || !use_inactive_text_tint {
        text_tint
    } else {
        // AbstractWidget.WithInactiveMessage.defaultInactiveMessage.
        hud_argb_to_tint(0xFFA0A0A0)
    }
}

pub(super) fn hud_debug_options_entry_name_run(text: &str, allowed: bool) -> HudStyledTextRun {
    HudStyledTextRun {
        shadow_color: None,
        text: text.to_string(),
        style: HudTextStyle {
            italic: !allowed,
            ..HudTextStyle::default()
        },
        color: None,
    }
}

pub(super) fn hud_debug_options_status_tint(
    status: HudDebugOptionsEntryStatus,
    current: bool,
) -> [f32; 4] {
    if !current {
        return hud_argb_to_tint(0xFFBABABA);
    }
    match status {
        HudDebugOptionsEntryStatus::AlwaysOn => hud_argb_to_tint(0xFFDF5050),
        HudDebugOptionsEntryStatus::InOverlay => hud_argb_to_tint(0xFFFFFF55),
        HudDebugOptionsEntryStatus::Never => HUD_TINT_WHITE,
    }
}

pub fn hud_debug_options_content_x(surface_size: PhysicalSize<u32>) -> i32 {
    i32::try_from(surface_size.width).unwrap_or(i32::MAX) / 2 - HUD_DEBUG_OPTIONS_ROW_WIDTH / 2
}

pub fn hud_debug_options_footer_button_y(surface_size: PhysicalSize<u32>) -> i32 {
    i32::try_from(surface_size.height).unwrap_or(i32::MAX) - HUD_DEBUG_OPTIONS_FOOTER_HEIGHT + 6
}

pub fn hud_debug_options_footer_button_xs(surface_size: PhysicalSize<u32>) -> (i32, i32, i32) {
    let total_width = HUD_DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH * 2
        + HUD_DEBUG_OPTIONS_DONE_BUTTON_WIDTH
        + HUD_DEBUG_OPTIONS_FOOTER_BUTTON_SPACING * 2;
    let default_x = i32::try_from(surface_size.width).unwrap_or(i32::MAX) / 2 - total_width / 2;
    let performance_x = default_x
        + HUD_DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH
        + HUD_DEBUG_OPTIONS_FOOTER_BUTTON_SPACING;
    let done_x = performance_x
        + HUD_DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH
        + HUD_DEBUG_OPTIONS_FOOTER_BUTTON_SPACING;
    (default_x, performance_x, done_x)
}

pub(super) fn hud_debug_crosshair_axis_endpoints(
    surface_size: PhysicalSize<u32>,
    crosshair: HudDebugCrosshair,
) -> [[f32; 2]; 3] {
    let center = [
        surface_size.width.max(1) as f32 * 0.5,
        surface_size.height.max(1) as f32 * 0.5,
    ];
    let length = hud_debug_crosshair_axis_length(surface_size, crosshair);
    hud_debug_crosshair_axis_vectors(crosshair).map(|vector| {
        [
            center[0] + vector[0] * length,
            center[1] + vector[1] * length,
        ]
    })
}

pub(super) fn hud_debug_crosshair_axis_vectors(crosshair: HudDebugCrosshair) -> [[f32; 2]; 3] {
    let rotation = glam::Quat::from_rotation_x(crosshair.x_rot_degrees.to_radians())
        * glam::Quat::from_rotation_y(crosshair.y_rot_degrees.to_radians());
    [glam::Vec3::X, glam::Vec3::Y, glam::Vec3::Z].map(|axis| {
        let scaled = glam::Vec3::new(-axis.x, axis.y, -axis.z);
        let rotated = rotation * scaled;
        [rotated.x, -rotated.y]
    })
}

pub(super) fn hud_debug_crosshair_axis_length(
    surface_size: PhysicalSize<u32>,
    crosshair: HudDebugCrosshair,
) -> f32 {
    let gui_scale = crosshair.gui_scale.max(1) as f32;
    let half_height = surface_size.height.max(1) as f32 * 0.5;
    let cot_half_fov = 1.0 / (HUD_DEBUG_CROSSHAIR_FOV_DEGREES.to_radians() * 0.5).tan();
    HUD_DEBUG_CROSSHAIR_SCALE * gui_scale * cot_half_fov * half_height
}

pub(super) fn hud_debug_crosshair_line_corners(
    start: [f32; 2],
    end: [f32; 2],
    width: f32,
) -> Option<[[f32; 2]; 4]> {
    if !start
        .iter()
        .chain(end.iter())
        .all(|coordinate| coordinate.is_finite())
        || !width.is_finite()
        || width <= 0.0
    {
        return None;
    }
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let length = dx.hypot(dy);
    if length <= f32::EPSILON {
        return None;
    }
    let half_width = width * 0.5;
    let normal = [-dy / length * half_width, dx / length * half_width];
    Some([
        [start[0] + normal[0], start[1] + normal[1]],
        [start[0] - normal[0], start[1] - normal[1]],
        [end[0] - normal[0], end[1] - normal[1]],
        [end[0] + normal[0], end[1] + normal[1]],
    ])
}

pub(super) fn hud_debug_game_mode_switcher_rect(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Option<HudRect> {
    let width = u32::try_from(width).ok().filter(|width| *width > 0)?;
    let height = u32::try_from(height).ok().filter(|height| *height > 0)?;
    Some(absolute_hud_rect(x as f32, y as f32, width, height))
}

pub(super) fn hud_debug_game_mode_switcher_icon_rect(
    slot: &HudDebugGameModeSwitcherSlot,
) -> Option<HudRect> {
    hud_debug_game_mode_switcher_rect(slot.x + 5, slot.y + 5, 16, 16)
}

pub(super) fn hud_debug_game_mode_switcher_background_uv() -> HudUvRect {
    HudUvRect {
        min: [0.0, 0.0],
        max: [
            HUD_DEBUG_GAME_MODE_SWITCHER_BACKGROUND_U_WIDTH
                / HUD_DEBUG_GAME_MODE_SWITCHER_TEXTURE_WIDTH,
            HUD_DEBUG_GAME_MODE_SWITCHER_BACKGROUND_V_HEIGHT
                / HUD_DEBUG_GAME_MODE_SWITCHER_TEXTURE_HEIGHT,
        ],
    }
}

pub(super) fn hud_debug_game_mode_switcher_centered_text_origin(
    runs: &[HudStyledTextRun],
    glyphs: &HudFontGlyphMap,
    center_x: i32,
    y: i32,
) -> (f32, f32) {
    let width = hud_coded_string_width(runs, glyphs);
    (center_x.saturating_sub(width / 2) as f32, y as f32)
}

pub(super) fn hud_debug_profiler_bottom_offset(overlay: &HudDebugOverlay) -> i32 {
    if overlay.fps_chart.is_some() || overlay.network_charts.is_some() {
        HUD_DEBUG_CHART_HEIGHT + HUD_DEBUG_CHART_LABEL_HEIGHT
    } else {
        HUD_DEBUG_PROFILER_BOTTOM_OFFSET
    }
}

pub(super) fn hud_debug_profiler_chart_layout(
    surface_size: PhysicalSize<u32>,
    slice_count: usize,
    current_node_line_count: usize,
    bottom_offset: i32,
) -> HudDebugProfilerChartLayout {
    let chart_center_x = i32::try_from(surface_size.width)
        .unwrap_or(i32::MAX)
        .saturating_sub(HUD_DEBUG_PROFILER_HALF_WIDTH)
        .saturating_sub(HUD_DEBUG_PROFILER_RIGHT_MARGIN);
    let left = chart_center_x.saturating_sub(HUD_DEBUG_PROFILER_HALF_WIDTH);
    let right = chart_center_x.saturating_add(HUD_DEBUG_PROFILER_HALF_WIDTH);
    let bottom = i32::try_from(surface_size.height)
        .unwrap_or(i32::MAX)
        .saturating_sub(bottom_offset)
        .saturating_sub(HUD_DEBUG_PROFILER_MARGIN);
    let text_start_y = bottom.saturating_sub(
        i32::try_from(slice_count)
            .unwrap_or(i32::MAX)
            .saturating_mul(HUD_DEBUG_CHART_LABEL_HEIGHT),
    );
    let chart_center_y = text_start_y
        .saturating_sub(HUD_DEBUG_PROFILER_HALF_HEIGHT)
        .saturating_sub(HUD_DEBUG_PROFILER_MARGIN);
    let current_node_top = chart_center_y
        .saturating_sub(HUD_DEBUG_PROFILER_HALF_HEIGHT)
        .saturating_sub(
            i32::try_from(current_node_line_count.saturating_sub(1))
                .unwrap_or(i32::MAX)
                .saturating_mul(HUD_DEBUG_CHART_LABEL_HEIGHT),
        );
    HudDebugProfilerChartLayout {
        left,
        right,
        chart_center_x,
        chart_center_y,
        text_start_y,
        current_node_top,
        bottom,
    }
}

pub(super) fn hud_debug_profiler_pie_point(
    center_x: f32,
    center_y: f32,
    percentage: f64,
) -> [f32; 2] {
    let dir = (percentage * std::f64::consts::TAU / 100.0) as f32;
    [
        center_x + dir.sin() * HUD_DEBUG_PROFILER_RADIUS,
        center_y + dir.cos() * HUD_DEBUG_PROFILER_RADIUS * HUD_DEBUG_PROFILER_VERTICAL_RADIUS_SCALE,
    ]
}

pub(super) fn hud_debug_profiler_vertex(
    surface_size: PhysicalSize<u32>,
    [x, y]: [f32; 2],
    tint: [f32; 4],
) -> HudVertex {
    let width = surface_size.width.max(1) as f32;
    let height = surface_size.height.max(1) as f32;
    HudVertex {
        position: [x / width * 2.0 - 1.0, 1.0 - y / height * 2.0],
        uv: [0.0, 0.0],
        tint,
        local_uv: [0.0, 0.0],
    }
}

pub(super) fn hud_debug_profiler_split_node_name(
    node_name: &str,
    first_line_max_width: u32,
    max_width: u32,
    glyphs: &HudFontGlyphMap,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    for (name_index, current_name) in node_name.split('.').enumerate() {
        let current_name_with_period = if name_index == 0 {
            current_name.to_string()
        } else {
            format!(".{current_name}")
        };
        let new_line = format!("{current_line}{current_name_with_period}");
        let limit = if lines.is_empty() {
            first_line_max_width
        } else {
            max_width
        };
        if glyphs.plain_text_width_f32(&new_line) > limit as f32 {
            if current_line.is_empty() {
                lines.push(current_name_with_period);
            } else {
                lines.push(std::mem::take(&mut current_line));
                current_line = current_name_with_period;
            }
        } else {
            current_line = new_line;
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

pub(super) fn hud_debug_profiler_demangle_path(path: &str) -> String {
    path.replace('\u{001e}', ".")
}

pub(super) fn hud_debug_profiler_percentage_text(percentage: f64) -> String {
    format!("{:.2}%", percentage.clamp(0.0, 100.0))
}

pub(super) fn hud_debug_profiler_slice_steps(percentage: f64) -> usize {
    ((percentage.clamp(0.0, 100.0) / 4.0).floor() as usize).saturating_add(1)
}

pub(super) fn hud_debug_profiler_slice_argb(name: &str) -> u32 {
    let hash = name.encode_utf16().fold(0i32, |hash, code_unit| {
        hash.wrapping_mul(31).wrapping_add(i32::from(code_unit))
    });
    ((hash as u32) & 0x00AA_AAAA).wrapping_add(0xFF44_4444)
}

pub(super) fn hud_debug_chart_width(surface_size: PhysicalSize<u32>) -> u32 {
    (HUD_DEBUG_CHART_SAMPLE_CAPACITY as u32 + 2).min(surface_size.width / 2)
}

pub(super) fn hud_debug_chart_visible_samples(samples: &[u64], width: u32) -> &[u64] {
    let sample_start = HUD_DEBUG_CHART_SAMPLE_CAPACITY
        .saturating_sub(usize::try_from(width.saturating_sub(2)).unwrap_or(usize::MAX));
    if sample_start < samples.len() {
        &samples[sample_start..]
    } else {
        &[]
    }
}

pub(super) fn hud_debug_chart_visible_tps_samples(
    samples: &[HudDebugTpsSample],
    width: u32,
) -> &[HudDebugTpsSample] {
    let sample_start = HUD_DEBUG_CHART_SAMPLE_CAPACITY
        .saturating_sub(usize::try_from(width.saturating_sub(2)).unwrap_or(usize::MAX));
    if sample_start < samples.len() {
        &samples[sample_start..]
    } else {
        &[]
    }
}

pub(super) fn hud_debug_fps_chart_display_string(nanos: f64) -> String {
    format!("{} ms", hud_debug_fps_chart_millis(nanos).round() as i64)
}

pub(super) fn hud_debug_fps_chart_sample_height(nanos: u64) -> i32 {
    hud_debug_fps_chart_sample_height_f64(nanos as f64)
}

fn hud_debug_fps_chart_sample_height_f64(nanos: f64) -> i32 {
    (hud_debug_fps_chart_millis(nanos) * HUD_DEBUG_CHART_HEIGHT as f64
        / HUD_DEBUG_FPS_CHART_TOP_VALUE_MS)
        .round()
        .clamp(0.0, i32::MAX as f64) as i32
}

pub(super) fn hud_debug_fps_configured_framerate_height(framerate_limit: u32) -> i32 {
    hud_debug_fps_chart_sample_height_f64(1.0E9 / f64::from(framerate_limit))
}

pub(super) fn hud_debug_fps_chart_sample_tint(nanos: u64) -> [f32; 4] {
    let millis = hud_debug_fps_chart_millis(nanos as f64);
    hud_argb_to_tint(hud_debug_chart_sample_argb(
        millis, 0.0, 0xFF00FF00, 28.0, 0xFFFFFF00, 56.0, 0xFFFF0000,
    ))
}

fn hud_debug_fps_chart_millis(nanos: f64) -> f64 {
    nanos / 1_000_000.0
}

pub(super) fn hud_debug_tps_chart_display_string(nanos: f64) -> String {
    format!("{} ms", hud_debug_fps_chart_millis(nanos).round() as i64)
}

pub(super) fn hud_debug_tps_chart_sample_height(nanos: u64, milliseconds_per_tick: f32) -> i32 {
    let mspt = f64::from(milliseconds_per_tick.max(f32::EPSILON));
    (hud_debug_fps_chart_millis(nanos as f64) * HUD_DEBUG_CHART_HEIGHT as f64 / mspt)
        .round()
        .clamp(0.0, i32::MAX as f64) as i32
}

pub(super) fn hud_debug_tps_chart_sample_tint(nanos: u64, milliseconds_per_tick: f32) -> [f32; 4] {
    let mspt = f64::from(milliseconds_per_tick.max(f32::EPSILON));
    let millis = hud_debug_fps_chart_millis(nanos as f64);
    hud_argb_to_tint(hud_debug_chart_sample_argb(
        millis,
        mspt,
        0xFF00FF00,
        mspt * 1.125,
        0xFFFFFF00,
        mspt * 1.25,
        0xFFFF0000,
    ))
}

pub(super) fn hud_debug_tps_chart_tps_label(milliseconds_per_tick: f32) -> String {
    format!(
        "{:.1} TPS",
        1000.0 / milliseconds_per_tick.max(f32::EPSILON)
    )
}

pub(super) fn hud_debug_tps_chart_aggregation_nanos(sample: HudDebugTpsSample) -> u64 {
    sample.full_tick_nanos.saturating_sub(sample.idle_nanos)
}

pub(super) fn hud_debug_tps_chart_other_nanos(sample: HudDebugTpsSample) -> u64 {
    sample
        .full_tick_nanos
        .saturating_sub(sample.idle_nanos)
        .saturating_sub(sample.tick_server_method_nanos)
        .saturating_sub(sample.scheduled_tasks_nanos)
}

pub(super) fn hud_debug_ping_chart_display_string(millis: f64) -> String {
    format!("{} ms", millis.round() as i64)
}

pub(super) fn hud_debug_ping_chart_sample_height(millis: u64) -> i32 {
    (millis as f64 * HUD_DEBUG_CHART_HEIGHT as f64 / 500.0)
        .round()
        .clamp(0.0, i32::MAX as f64) as i32
}

pub(super) fn hud_debug_ping_chart_sample_tint(millis: u64) -> [f32; 4] {
    hud_argb_to_tint(hud_debug_chart_sample_argb(
        millis as f64,
        0.0,
        0xFF00FF00,
        250.0,
        0xFFFFFF00,
        500.0,
        0xFFFF0000,
    ))
}

pub(super) fn hud_debug_bandwidth_chart_display_string(bytes_per_tick: f64) -> String {
    hud_debug_bandwidth_chart_display_string_internal(bytes_per_tick * 20.0)
}

pub(super) fn hud_debug_bandwidth_chart_display_string_internal(bytes_per_second: f64) -> String {
    if bytes_per_second >= 1_048_576.0 {
        format!("{:.1} MiB/s", bytes_per_second / 1_048_576.0)
    } else if bytes_per_second >= 1_024.0 {
        format!("{:.1} KiB/s", bytes_per_second / 1_024.0)
    } else {
        format!("{} B/s", bytes_per_second.floor() as u64)
    }
}

pub(super) fn hud_debug_bandwidth_chart_sample_height(bytes_per_tick: u64) -> i32 {
    hud_debug_bandwidth_chart_sample_height_internal(bytes_per_tick as f64 * 20.0)
}

pub(super) fn hud_debug_bandwidth_chart_sample_height_internal(bytes_per_second: f64) -> i32 {
    ((bytes_per_second + 1.0).ln() * HUD_DEBUG_CHART_HEIGHT as f64 / 1_048_576.0_f64.ln())
        .round()
        .clamp(0.0, i32::MAX as f64) as i32
}

pub(super) fn hud_debug_bandwidth_chart_sample_tint(bytes_per_tick: u64) -> [f32; 4] {
    hud_argb_to_tint(hud_debug_chart_sample_argb(
        bytes_per_tick as f64 * 20.0,
        0.0,
        0xFF00FFFF,
        8_192.0,
        0xFFA0A0FF,
        10_485_760.0,
        0xFFFF0000,
    ))
}

fn hud_debug_chart_sample_argb(
    sample: f64,
    min: f64,
    min_color: u32,
    mid: f64,
    mid_color: u32,
    max: f64,
    max_color: u32,
) -> u32 {
    let sample = sample.clamp(min, max);
    if sample < mid {
        hud_argb_lerp((sample - min) / (mid - min), min_color, mid_color)
    } else {
        hud_argb_lerp((sample - mid) / (max - mid), mid_color, max_color)
    }
}

pub(super) fn hud_debug_overlay_line_origin(
    surface_size: PhysicalSize<u32>,
    width: i32,
    line_index: usize,
    align_left: bool,
) -> (i32, i32) {
    let left = if align_left {
        HUD_DEBUG_OVERLAY_MARGIN_X
    } else {
        i32::try_from(surface_size.width)
            .unwrap_or(i32::MAX)
            .saturating_sub(HUD_DEBUG_OVERLAY_MARGIN_X)
            .saturating_sub(width)
    };
    let top = HUD_DEBUG_OVERLAY_MARGIN_Y.saturating_add(
        i32::try_from(line_index)
            .unwrap_or(i32::MAX)
            .saturating_mul(HUD_DEBUG_OVERLAY_LINE_HEIGHT),
    );
    (left, top)
}

pub(super) fn hud_debug_lightmap_preview_rect(surface_size: PhysicalSize<u32>) -> HudRect {
    let (x, y) = hud_debug_lightmap_preview_origin(surface_size);
    absolute_hud_rect(
        x as f32,
        y as f32,
        HUD_DEBUG_LIGHTMAP_PREVIEW_SIZE,
        HUD_DEBUG_LIGHTMAP_PREVIEW_SIZE,
    )
}

pub(super) fn hud_debug_lightmap_preview_border_rect(surface_size: PhysicalSize<u32>) -> HudRect {
    let (x, y) = hud_debug_lightmap_preview_origin(surface_size);
    absolute_hud_rect(
        (x - HUD_DEBUG_LIGHTMAP_PREVIEW_BORDER) as f32,
        (y - HUD_DEBUG_LIGHTMAP_PREVIEW_BORDER) as f32,
        HUD_DEBUG_LIGHTMAP_PREVIEW_SIZE + (HUD_DEBUG_LIGHTMAP_PREVIEW_BORDER as u32 * 2),
        HUD_DEBUG_LIGHTMAP_PREVIEW_SIZE + (HUD_DEBUG_LIGHTMAP_PREVIEW_BORDER as u32 * 2),
    )
}

fn hud_debug_lightmap_preview_origin(surface_size: PhysicalSize<u32>) -> (i32, i32) {
    let x = i32::try_from(surface_size.width)
        .unwrap_or(i32::MAX)
        .saturating_sub(HUD_DEBUG_LIGHTMAP_PREVIEW_SIZE as i32)
        .saturating_sub(HUD_DEBUG_LIGHTMAP_PREVIEW_MARGIN);
    let y = i32::try_from(surface_size.height)
        .unwrap_or(i32::MAX)
        .saturating_sub(HUD_DEBUG_LIGHTMAP_PREVIEW_SIZE as i32)
        .saturating_sub(HUD_DEBUG_LIGHTMAP_PREVIEW_MARGIN);
    (x, y)
}

pub(super) fn hud_debug_lightmap_preview_uv() -> HudUvRect {
    HudUvRect {
        min: [0.0, 1.0],
        max: [1.0, 0.0],
    }
}
