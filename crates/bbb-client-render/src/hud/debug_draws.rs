use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_debug_options_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    text_field: Option<&'a HudSpriteGpu>,
    text_field_highlighted: Option<&'a HudSpriteGpu>,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    tooltip_background: Option<&'a HudSpriteGpu>,
    tooltip_frame: Option<&'a HudSpriteGpu>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudDebugOptionsScreen,
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    push_hud_fill_rect(
        vertices,
        commands,
        surface_size,
        0,
        0,
        surface_size.width,
        surface_size.height,
        [0.0, 0.0, 0.0, 0.45],
    );

    push_hud_debug_options_centered_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &hud_coded_string_runs(&screen.title),
        8,
        HUD_TINT_WHITE,
    );
    push_hud_debug_options_centered_wrapped_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &hud_coded_string_runs(&screen.warning),
        34,
        HUD_DEBUG_OPTIONS_ROW_WIDTH.max(0) as u32,
        hud_argb_to_tint(0xFFDF5050),
    );
    push_hud_debug_options_search_box(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &screen.search,
        &screen.search_hint,
        text_field,
        text_field_highlighted,
    );

    let content_x = hud_debug_options_content_x(surface_size);
    for (row_index, row) in screen.rows.iter().enumerate() {
        let y = HUD_DEBUG_OPTIONS_HEADER_HEIGHT
            + i32::try_from(row_index).unwrap_or(i32::MAX) * HUD_DEBUG_OPTIONS_ROW_HEIGHT;
        match row {
            HudDebugOptionsRow::Category { label } => {
                push_hud_debug_options_centered_text_in_width(
                    vertices,
                    commands,
                    white_pixel,
                    font_atlas,
                    glyphs,
                    obfuscated_pool,
                    frame_index,
                    surface_size,
                    &hud_coded_string_runs(label),
                    content_x,
                    HUD_DEBUG_OPTIONS_ROW_WIDTH,
                    y + 5,
                    HUD_TINT_WHITE,
                );
            }
            HudDebugOptionsRow::Entry {
                path,
                status,
                hovered_status,
                focused_status,
                allowed,
            } => {
                push_hud_debug_options_entry(
                    vertices,
                    commands,
                    white_pixel,
                    font_atlas,
                    glyphs,
                    obfuscated_pool,
                    frame_index,
                    surface_size,
                    content_x,
                    y,
                    button_sprites,
                    path,
                    *status,
                    *hovered_status,
                    *focused_status,
                    *allowed,
                    &screen.never_label,
                    &screen.overlay_label,
                    &screen.always_label,
                );
            }
        }
    }

    push_hud_debug_options_scrollbar(
        vertices,
        commands,
        white_pixel,
        surface_size,
        screen.scroll_row,
        screen.total_rows,
    );

    push_hud_debug_options_footer(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        button_sprites,
        screen.default_profile_active,
        screen.default_profile_hovered,
        screen.default_profile_focused,
        &screen.default_profile_label,
        screen.performance_profile_active,
        screen.performance_profile_hovered,
        screen.performance_profile_focused,
        &screen.performance_profile_label,
        screen.done_hovered,
        screen.done_focused,
        &screen.done_label,
    );
    push_hud_debug_options_tooltip(
        vertices,
        commands,
        white_pixel,
        tooltip_background,
        tooltip_frame,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        screen.tooltip.as_ref(),
    );
}

fn push_hud_debug_options_scrollbar<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    scroll_row: usize,
    total_rows: usize,
) {
    let Some((background_rect, scroller_rect)) =
        hud_debug_options_scrollbar_rects(surface_size, scroll_row, total_rows)
    else {
        return;
    };
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        white_pixel,
        surface_size,
        absolute_hud_rect(
            (background_rect.x) as f32,
            (background_rect.y) as f32,
            background_rect.width,
            background_rect.height,
        ),
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        [0.0, 0.0, 0.0, 0.45],
    );
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        white_pixel,
        surface_size,
        absolute_hud_rect(
            (scroller_rect.x) as f32,
            (scroller_rect.y) as f32,
            scroller_rect.width,
            scroller_rect.height,
        ),
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        [0.55, 0.55, 0.55, 0.92],
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_debug_options_tooltip<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    tooltip_background: Option<&'a HudSpriteGpu>,
    tooltip_frame: Option<&'a HudSpriteGpu>,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    tooltip: Option<&HudDebugOptionsTooltip>,
) {
    let Some(tooltip) = tooltip else {
        return;
    };
    let runs = hud_coded_string_runs(tooltip.text.as_str());
    let Some(text_width) = hud_font_runs_width(&runs, glyphs) else {
        return;
    };
    let Some(text_height) = hud_inventory_tooltip_text_height(1) else {
        return;
    };
    let background_rect = hud_inventory_tooltip_background_hud_rect(
        surface_size,
        surface_size.width,
        surface_size.height,
        tooltip.x,
        tooltip.y,
        text_width,
        text_height,
    );
    match (tooltip_background, tooltip_frame) {
        (Some(background), Some(frame)) => {
            push_hud_gui_sprite(
                vertices,
                commands,
                background,
                surface_size,
                background_rect,
            );
            push_hud_gui_sprite(vertices, commands, frame, surface_size, background_rect);
        }
        _ => {
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                white_pixel,
                surface_size,
                background_rect,
                HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                HUD_TOOLTIP_BACKGROUND_TINT,
            );
        }
    }

    let origin = hud_inventory_tooltip_line_origin(
        surface_size,
        surface_size.width,
        surface_size.height,
        tooltip.x,
        tooltip.y,
        text_width,
        text_height,
        0,
    );
    let geometry = hud_styled_text_geometry(
        &runs,
        glyphs,
        obfuscated_pool,
        frame_index,
        origin,
        true,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    push_hud_styled_text_geometry(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        surface_size,
        &geometry,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_options_entry<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    content_x: i32,
    y: i32,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    path: &str,
    status: HudDebugOptionsEntryStatus,
    hovered_status: Option<HudDebugOptionsEntryStatus>,
    focused_status: Option<HudDebugOptionsEntryStatus>,
    allowed: bool,
    never_label: &str,
    overlay_label: &str,
    always_label: &str,
) {
    let name_tint = if allowed {
        HUD_TINT_WHITE
    } else {
        hud_argb_to_tint(0xFF808080)
    };
    push_hud_debug_options_entry_name(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        path,
        (content_x as f32, (y + 5) as f32),
        name_tint,
        allowed,
    );
    let buttons_start_x =
        content_x + HUD_DEBUG_OPTIONS_ROW_WIDTH - HUD_DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 3;
    for (index, (button_status, label)) in [
        (HudDebugOptionsEntryStatus::Never, never_label),
        (HudDebugOptionsEntryStatus::InOverlay, overlay_label),
        (HudDebugOptionsEntryStatus::AlwaysOn, always_label),
    ]
    .into_iter()
    .enumerate()
    {
        let x = buttons_start_x
            + i32::try_from(index).unwrap_or(i32::MAX) * HUD_DEBUG_OPTIONS_STATUS_BUTTON_WIDTH;
        push_hud_debug_options_button_with_text_policy(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            x,
            y,
            HUD_DEBUG_OPTIONS_STATUS_BUTTON_WIDTH,
            HUD_DEBUG_OPTIONS_STATUS_BUTTON_HEIGHT,
            &hud_coded_string_runs(label),
            status != button_status,
            hovered_status == Some(button_status) || focused_status == Some(button_status),
            hud_debug_options_status_tint(button_status, status == button_status),
            false,
            button_sprites,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_options_footer<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    default_profile_active: bool,
    default_profile_hovered: bool,
    default_profile_focused: bool,
    default_profile_label: &str,
    performance_profile_active: bool,
    performance_profile_hovered: bool,
    performance_profile_focused: bool,
    performance_profile_label: &str,
    done_hovered: bool,
    done_focused: bool,
    done_label: &str,
) {
    let y = hud_debug_options_footer_button_y(surface_size);
    let (default_x, performance_x, done_x) = hud_debug_options_footer_button_xs(surface_size);
    push_hud_debug_options_button(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        default_x,
        y,
        HUD_DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH,
        HUD_DEBUG_OPTIONS_FOOTER_BUTTON_HEIGHT,
        &hud_coded_string_runs(default_profile_label),
        default_profile_active,
        default_profile_hovered || default_profile_focused,
        HUD_TINT_WHITE,
        button_sprites,
    );
    push_hud_debug_options_button(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        performance_x,
        y,
        HUD_DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH,
        HUD_DEBUG_OPTIONS_FOOTER_BUTTON_HEIGHT,
        &hud_coded_string_runs(performance_profile_label),
        performance_profile_active,
        performance_profile_hovered || performance_profile_focused,
        HUD_TINT_WHITE,
        button_sprites,
    );
    push_hud_debug_options_button(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        done_x,
        y,
        HUD_DEBUG_OPTIONS_DONE_BUTTON_WIDTH,
        HUD_DEBUG_OPTIONS_FOOTER_BUTTON_HEIGHT,
        &hud_coded_string_runs(done_label),
        true,
        done_hovered || done_focused,
        HUD_TINT_WHITE,
        button_sprites,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_options_search_box<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    search: &HudTextField,
    search_hint: &str,
    text_field: Option<&'a HudSpriteGpu>,
    text_field_highlighted: Option<&'a HudSpriteGpu>,
) {
    let x = hud_debug_options_content_x(surface_size) + HUD_DEBUG_OPTIONS_ROW_WIDTH
        - HUD_DEBUG_OPTIONS_SEARCH_WIDTH;
    let y = 6;
    let (outer, inner) = hud_debug_options_search_box_rects(surface_size);
    let text_field = if search.focused {
        text_field_highlighted.or(text_field)
    } else {
        text_field
    };
    if let Some(text_field) = text_field {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            text_field,
            surface_size,
            absolute_hud_rect(outer.0 as f32, outer.1 as f32, outer.2, outer.3),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            HUD_TINT_WHITE,
        );
    } else {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            white_pixel,
            surface_size,
            absolute_hud_rect(outer.0 as f32, outer.1 as f32, outer.2, outer.3),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            [0.95, 0.95, 0.95, 1.0],
        );
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            white_pixel,
            surface_size,
            absolute_hud_rect(inner.0 as f32, inner.1 as f32, inner.2, inner.3),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            [0.0, 0.0, 0.0, 0.75],
        );
    }
    if search.text.is_empty() && !search.focused {
        let (_, runs) = hud_debug_options_search_display(&search.text, false, search_hint);
        let draw = HudScreenTextDraw {
            runs: &runs,
            origin: ((x + 4) as f32, (y + 6) as f32),
            scale: 1.0,
            tint: hud_argb_to_tint(0xFFE0E0E0),
            backdrop_opacity: 0.0,
        };
        push_hud_screen_text_draw(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &draw,
        );
        return;
    }
    let input = search.decoration(HUD_TINT_WHITE);
    let label = HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: 0,
        y: 0,
        width: u32::try_from(HUD_DEBUG_OPTIONS_SEARCH_WIDTH - 8).unwrap_or_default(),
        text: search.text.clone(),
        tint: hud_argb_to_tint(0xFFE0E0E0),
        background: None,
        input: Some(input),
        scrolling: None,
        shadow: true,
        font: None,
        runs: Vec::new(),
    };
    push_hud_inventory_text_input_label(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &label,
        input,
        ((x + 4) as f32, (y + 6) as f32),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_debug_options_button<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    label: &[HudStyledTextRun],
    active: bool,
    highlighted: bool,
    text_tint: [f32; 4],
    button_sprites: HudDebugOptionsButtonSprites<'a>,
) {
    push_hud_debug_options_button_with_text_policy(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        x,
        y,
        width,
        height,
        label,
        active,
        highlighted,
        text_tint,
        true,
        button_sprites,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_debug_options_button_with_text_policy<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    label: &[HudStyledTextRun],
    active: bool,
    highlighted: bool,
    text_tint: [f32; 4],
    use_inactive_text_tint: bool,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
) {
    let sprite = button_sprites.get(active, highlighted);
    if let Some(sprite) = sprite {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            sprite,
            surface_size,
            absolute_hud_rect(
                x as f32,
                y as f32,
                width.max(0) as u32,
                height.max(0) as u32,
            ),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            HUD_TINT_WHITE,
        );
    } else {
        let background_tint = if active {
            [0.18, 0.18, 0.18, 0.92]
        } else {
            [0.08, 0.08, 0.08, 0.86]
        };
        if width > 0 && height > 0 {
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                white_pixel,
                surface_size,
                absolute_hud_rect(
                    x as f32,
                    y as f32,
                    width.max(0) as u32,
                    height.max(0) as u32,
                ),
                HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                background_tint,
            );
        }
        if width > 0 {
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                white_pixel,
                surface_size,
                absolute_hud_rect(x as f32, y as f32, width.max(0) as u32, 1),
                HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                [0.8, 0.8, 0.8, 0.75],
            );
        }
    }
    push_hud_debug_options_centered_text_in_width(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        label,
        x,
        width,
        y + (height - 8).max(0) / 2,
        hud_debug_options_button_label_tint(active, text_tint, use_inactive_text_tint),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_debug_options_centered_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    runs: &[HudStyledTextRun],
    y: i32,
    tint: [f32; 4],
) {
    push_hud_debug_options_centered_text_in_width(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        runs,
        0,
        i32::try_from(surface_size.width).unwrap_or(i32::MAX),
        y,
        tint,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_options_centered_wrapped_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    runs: &[HudStyledTextRun],
    y: i32,
    max_width: u32,
    tint: [f32; 4],
) {
    for (line_index, line) in hud_wrap_chat_runs(runs, max_width, glyphs)
        .into_iter()
        .enumerate()
    {
        let width = hud_font_runs_width(&line, glyphs).unwrap_or_default() as f32;
        let draw = HudScreenTextDraw {
            runs: &line,
            origin: (
                surface_size.width as f32 * 0.5 - width * 0.5,
                (y + i32::try_from(line_index).unwrap_or(i32::MAX) * 9) as f32,
            ),
            scale: 1.0,
            tint,
            backdrop_opacity: 0.0,
        };
        push_hud_screen_text_draw(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &draw,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_debug_options_centered_text_in_width<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    runs: &[HudStyledTextRun],
    x: i32,
    width: i32,
    y: i32,
    tint: [f32; 4],
) {
    let text_width = hud_font_runs_width(runs, glyphs).unwrap_or_default() as f32;
    let origin = (
        x as f32 + width.max(0) as f32 * 0.5 - text_width * 0.5,
        y as f32,
    );
    let draw = HudScreenTextDraw {
        runs,
        origin,
        scale: 1.0,
        tint,
        backdrop_opacity: 0.0,
    };
    push_hud_screen_text_draw(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &draw,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_debug_options_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    runs: &[HudStyledTextRun],
    origin: (f32, f32),
    tint: [f32; 4],
) {
    let draw = HudScreenTextDraw {
        runs,
        origin,
        scale: 1.0,
        tint,
        backdrop_opacity: 0.0,
    };
    push_hud_screen_text_draw(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &draw,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_options_entry_name<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    text: &str,
    origin: (f32, f32),
    tint: [f32; 4],
    allowed: bool,
) {
    let runs = [hud_debug_options_entry_name_run(text, allowed)];
    let draw = HudScreenTextDraw {
        runs: &runs,
        origin,
        scale: 1.0,
        tint,
        backdrop_opacity: 0.0,
    };
    push_hud_screen_text_draw(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &draw,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_debug_crosshair<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    crosshair: HudDebugCrosshair,
) {
    let center = [
        surface_size.width.max(1) as f32 * 0.5,
        surface_size.height.max(1) as f32 * 0.5,
    ];
    let length = hud_debug_crosshair_axis_length(surface_size, crosshair);
    let endpoints = hud_debug_crosshair_axis_endpoints(surface_size, crosshair);
    if length <= f32::EPSILON {
        return;
    }

    for endpoint in endpoints {
        push_hud_debug_crosshair_line(
            vertices,
            commands,
            white_pixel,
            surface_size,
            center,
            endpoint,
            HUD_DEBUG_CROSSHAIR_OUTLINE_WIDTH,
            hud_argb_to_tint(HUD_DEBUG_CROSSHAIR_OUTLINE_ARGB),
        );
    }
    for (axis_index, endpoint) in endpoints.into_iter().enumerate() {
        push_hud_debug_crosshair_line(
            vertices,
            commands,
            white_pixel,
            surface_size,
            center,
            endpoint,
            HUD_DEBUG_CROSSHAIR_AXIS_WIDTH,
            hud_argb_to_tint(HUD_DEBUG_CROSSHAIR_AXIS_ARGB[axis_index]),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_crosshair_line<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    start: [f32; 2],
    end: [f32; 2],
    width: f32,
    tint: [f32; 4],
) {
    let Some(corners) = hud_debug_crosshair_line_corners(start, end, width) else {
        return;
    };
    let vertex_start = vertices.len() as u32;
    vertices.extend_from_slice(&hud_styled_quad_vertices(
        surface_size,
        corners,
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        tint,
    ));
    let vertex_end = vertices.len() as u32;
    commands.push(HudDrawCommand::Sprite {
        sprite: white_pixel,
        start: vertex_start,
        end: vertex_end,
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_debug_overlay<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    post_gui_item_commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    item_atlas: Option<&'a HudSpriteGpu>,
    digit_atlas: Option<&'a HudSpriteGpu>,
    digit_glyphs: &[HudDigitGlyph; 10],
    game_mode_switcher_background: Option<&'a HudSpriteGpu>,
    game_mode_switcher_slot: Option<&'a HudSpriteGpu>,
    game_mode_switcher_selection: Option<&'a HudSpriteGpu>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    overlay: &HudDebugOverlay,
) {
    if let Some(switcher) = &overlay.game_mode_switcher {
        push_hud_debug_game_mode_switcher(
            vertices,
            commands,
            white_pixel,
            game_mode_switcher_background,
            game_mode_switcher_slot,
            game_mode_switcher_selection,
            item_atlas,
            digit_atlas,
            digit_glyphs,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            switcher,
            post_gui_item_commands,
        );
    }
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let commands = post_gui_item_commands;
    push_hud_debug_overlay_column_backgrounds(
        vertices,
        commands,
        glyphs,
        surface_size,
        &overlay.left_lines,
        true,
    );
    push_hud_debug_overlay_column_backgrounds(
        vertices,
        commands,
        glyphs,
        surface_size,
        &overlay.right_lines,
        false,
    );
    push_hud_debug_overlay_column_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &overlay.left_lines,
        true,
    );
    push_hud_debug_overlay_column_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &overlay.right_lines,
        false,
    );
    if let Some(chart) = &overlay.fps_chart {
        push_hud_debug_fps_chart(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            chart,
        );
    }
    if let Some(chart) = &overlay.tps_chart {
        push_hud_debug_tps_chart(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            chart,
        );
    }
    if let Some(charts) = &overlay.network_charts {
        push_hud_debug_network_charts(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            charts,
        );
    }
    if let Some(chart) = &overlay.profiler_chart {
        push_hud_debug_profiler_chart(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            chart,
            hud_debug_profiler_bottom_offset(overlay),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_game_mode_switcher<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    background: Option<&'a HudSpriteGpu>,
    slot_sprite: Option<&'a HudSpriteGpu>,
    selection_sprite: Option<&'a HudSpriteGpu>,
    item_atlas: Option<&'a HudSpriteGpu>,
    digit_atlas: Option<&'a HudSpriteGpu>,
    digit_glyphs: &[HudDigitGlyph; 10],
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    switcher: &HudDebugGameModeSwitcher,
    post_gui_item_commands: &mut Vec<HudDrawCommand<'a>>,
) {
    if let (Some(background), Some(rect)) = (
        background,
        hud_debug_game_mode_switcher_rect(
            switcher.background_x,
            switcher.background_y,
            switcher.background_width,
            switcher.background_height,
        ),
    ) {
        push_hud_draw_with_uv(
            vertices,
            commands,
            background,
            surface_size,
            rect,
            hud_debug_game_mode_switcher_background_uv(),
        );
    }

    for slot in &switcher.slots {
        let Some(rect) = hud_debug_game_mode_switcher_rect(slot.x, slot.y, slot.width, slot.height)
        else {
            continue;
        };
        if let Some(slot_sprite) = slot_sprite {
            push_hud_draw(vertices, commands, slot_sprite, surface_size, rect);
        }
        if slot.selected {
            if let Some(selection_sprite) = selection_sprite {
                push_hud_draw(vertices, commands, selection_sprite, surface_size, rect);
            }
        }
        if let (Some(item_atlas), Some(icon_rect), Some(icon)) = (
            item_atlas,
            hud_debug_game_mode_switcher_icon_rect(slot),
            slot.icon.as_ref(),
        ) {
            let renders_as_3d_block = slot.block_model.is_some();
            push_hud_item_icon(
                vertices,
                commands,
                item_atlas,
                digit_atlas,
                digit_glyphs,
                surface_size,
                icon_rect,
                icon,
                !renders_as_3d_block,
                !renders_as_3d_block,
            );
            if renders_as_3d_block {
                push_hud_item_icon(
                    vertices,
                    post_gui_item_commands,
                    item_atlas,
                    digit_atlas,
                    digit_glyphs,
                    surface_size,
                    icon_rect,
                    icon,
                    false,
                    true,
                );
            }
        }
    }

    let Some(font_atlas) = font_atlas else {
        return;
    };
    let center_x = switcher.background_x + HUD_DEBUG_GAME_MODE_SWITCHER_CENTER_X_OFFSET;
    push_hud_debug_game_mode_switcher_centered_text(
        vertices,
        post_gui_item_commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &hud_coded_string_runs(&switcher.title),
        center_x,
        switcher.background_y + HUD_DEBUG_GAME_MODE_SWITCHER_TITLE_Y_OFFSET,
    );
    push_hud_debug_game_mode_switcher_centered_text(
        vertices,
        post_gui_item_commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &switcher.help_runs,
        center_x,
        switcher.background_y + HUD_DEBUG_GAME_MODE_SWITCHER_HELP_Y_OFFSET,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_game_mode_switcher_centered_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    runs: &[HudStyledTextRun],
    center_x: i32,
    y: i32,
) {
    // `centeredText(Font, Component, ...)`
    // measures and draws one visual-order sequence, so the width and the glyphs
    // read the same run list.
    let origin = hud_debug_game_mode_switcher_centered_text_origin(runs, glyphs, center_x, y);
    push_hud_coded_string_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        runs,
        origin,
        HUD_TINT_WHITE,
        1.0,
        true,
    );
}

fn push_hud_debug_overlay_column_backgrounds<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    glyphs: &HudFontGlyphMap,
    surface_size: PhysicalSize<u32>,
    lines: &[String],
    align_left: bool,
) {
    for (line_index, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        let width = hud_coded_string_text_width(line, glyphs);
        let (left, top) =
            hud_debug_overlay_line_origin(surface_size, width, line_index, align_left);
        push_hud_fill(
            vertices,
            commands,
            surface_size,
            absolute_hud_rect(
                (left - 1) as f32,
                (top - 1) as f32,
                width.saturating_add(2).max(0) as u32,
                HUD_DEBUG_OVERLAY_LINE_HEIGHT as u32,
            ),
            HUD_DEBUG_OVERLAY_BACKGROUND_TINT,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_overlay_column_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    lines: &[String],
    align_left: bool,
) {
    for (line_index, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        // `DebugScreenOverlay.extractLines` measures with `font.width(String)`
        // and draws with `graphics.text(font, String, ...)`
        // both code-consuming.
        let width = hud_coded_string_text_width(line, glyphs);
        let (left, top) =
            hud_debug_overlay_line_origin(surface_size, width, line_index, align_left);
        push_hud_coded_string_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &hud_coded_string_runs(line),
            (left as f32, top as f32),
            HUD_DEBUG_OVERLAY_TEXT_TINT,
            1.0,
            false,
        );
    }
}

pub(super) fn push_hud_debug_lightmap_preview<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    surface_size: PhysicalSize<u32>,
) {
    let border = hud_debug_lightmap_preview_border_rect(surface_size);
    push_hud_fill(
        vertices,
        commands,
        surface_size,
        border,
        [0.0, 0.0, 0.0, 1.0],
    );

    let preview = hud_debug_lightmap_preview_rect(surface_size);
    let start = vertices.len() as u32;
    vertices.extend_from_slice(&hud_quad_vertices(
        surface_size,
        preview,
        hud_debug_lightmap_preview_uv(),
        HUD_TINT_WHITE,
    ));
    let end = vertices.len() as u32;
    commands.push(HudDrawCommand::LightmapPreview { start, end });
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_fps_chart<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    chart: &HudDebugFrameTimeChart,
) {
    let width = hud_debug_chart_width(surface_size);
    if width == 0 {
        return;
    }
    let left = 0;
    let bottom = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let top = bottom.saturating_sub(HUD_DEBUG_CHART_HEIGHT);
    push_hud_debug_sample_chart(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        left,
        width,
        &chart.frame_time_nanos,
        hud_debug_fps_chart_sample_height,
        hud_debug_fps_chart_sample_tint,
        hud_debug_fps_chart_display_string,
    );

    push_hud_debug_chart_shaded_label(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        "30 FPS",
        left + 1,
        top + 1,
    );
    push_hud_debug_chart_shaded_label(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        "60 FPS",
        left + 1,
        bottom.saturating_sub(30).saturating_add(1),
    );
    push_hud_debug_chart_horizontal_line(
        vertices,
        commands,
        surface_size,
        left,
        width,
        bottom.saturating_sub(30),
        HUD_TINT_WHITE,
    );
    if let Some(framerate_limit) = chart.configured_framerate_limit {
        push_hud_debug_chart_horizontal_line(
            vertices,
            commands,
            surface_size,
            left,
            width,
            bottom
                .saturating_sub(hud_debug_fps_configured_framerate_height(framerate_limit))
                .saturating_sub(1),
            hud_argb_to_tint(0xFF00FFFF),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_tps_chart<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    chart: &HudDebugTpsChart,
) {
    let width = hud_debug_chart_width(surface_size);
    if width == 0 {
        return;
    }
    let surface_width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let left = surface_width.saturating_sub(i32::try_from(width).unwrap_or(i32::MAX));
    let bottom = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let top = bottom.saturating_sub(HUD_DEBUG_CHART_HEIGHT);
    push_hud_fill_rect(
        vertices,
        commands,
        surface_size,
        left,
        top,
        width,
        HUD_DEBUG_CHART_HEIGHT as u32,
        HUD_DEBUG_OVERLAY_BACKGROUND_TINT,
    );

    let visible_samples = hud_debug_chart_visible_tps_samples(&chart.samples, width);
    for (index, sample) in visible_samples.iter().copied().enumerate() {
        let x = left + i32::try_from(index).unwrap_or(i32::MAX).saturating_add(1);
        let full_height =
            hud_debug_tps_chart_sample_height(sample.full_tick_nanos, chart.milliseconds_per_tick);
        if full_height > 0 {
            push_hud_fill_rect(
                vertices,
                commands,
                surface_size,
                x,
                bottom.saturating_sub(full_height),
                1,
                u32::try_from(full_height).unwrap_or(u32::MAX),
                hud_debug_tps_chart_sample_tint(
                    sample.full_tick_nanos,
                    chart.milliseconds_per_tick,
                ),
            );
        }

        let tick_method_height = hud_debug_tps_chart_sample_height(
            sample.tick_server_method_nanos,
            chart.milliseconds_per_tick,
        );
        if tick_method_height > 0 {
            push_hud_fill_rect(
                vertices,
                commands,
                surface_size,
                x,
                bottom.saturating_sub(tick_method_height),
                1,
                u32::try_from(tick_method_height).unwrap_or(u32::MAX),
                hud_argb_to_tint(0xFF991111),
            );
        }

        let tasks_height = hud_debug_tps_chart_sample_height(
            sample.scheduled_tasks_nanos,
            chart.milliseconds_per_tick,
        );
        if tasks_height > 0 {
            push_hud_fill_rect(
                vertices,
                commands,
                surface_size,
                x,
                bottom
                    .saturating_sub(tick_method_height)
                    .saturating_sub(tasks_height),
                1,
                u32::try_from(tasks_height).unwrap_or(u32::MAX),
                hud_argb_to_tint(0xFFBA995F),
            );
        }

        let other_height = hud_debug_tps_chart_sample_height(
            hud_debug_tps_chart_other_nanos(sample),
            chart.milliseconds_per_tick,
        );
        if other_height > 0 {
            push_hud_fill_rect(
                vertices,
                commands,
                surface_size,
                x,
                bottom
                    .saturating_sub(tick_method_height)
                    .saturating_sub(tasks_height)
                    .saturating_sub(other_height),
                1,
                u32::try_from(other_height).unwrap_or(u32::MAX),
                hud_argb_to_tint(0xFF5F0E8C),
            );
        }
    }

    push_hud_debug_chart_horizontal_line(
        vertices,
        commands,
        surface_size,
        left,
        width,
        top,
        HUD_TINT_WHITE,
    );
    push_hud_debug_chart_horizontal_line(
        vertices,
        commands,
        surface_size,
        left,
        width,
        bottom.saturating_sub(1),
        HUD_TINT_WHITE,
    );
    push_hud_debug_chart_vertical_line(
        vertices,
        commands,
        surface_size,
        left,
        top,
        HUD_DEBUG_CHART_HEIGHT as u32,
        HUD_TINT_WHITE,
    );
    push_hud_debug_chart_vertical_line(
        vertices,
        commands,
        surface_size,
        left + i32::try_from(width.saturating_sub(1)).unwrap_or(i32::MAX),
        top,
        HUD_DEBUG_CHART_HEIGHT as u32,
        HUD_TINT_WHITE,
    );

    if !visible_samples.is_empty() {
        let aggregation_values = visible_samples
            .iter()
            .map(|sample| hud_debug_tps_chart_aggregation_nanos(*sample));
        let min = aggregation_values.clone().min().unwrap_or(0);
        let max = aggregation_values.clone().max().unwrap_or(0);
        let avg = aggregation_values.map(|sample| sample as f64).sum::<f64>()
            / visible_samples.len() as f64;
        push_hud_debug_chart_label(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &format!("{} min", hud_debug_tps_chart_display_string(min as f64)),
            left + 2,
            top.saturating_sub(HUD_DEBUG_CHART_LABEL_HEIGHT),
        );
        let avg_text = format!("{} avg", hud_debug_tps_chart_display_string(avg));
        let avg_width = hud_coded_string_text_width(&avg_text, glyphs);
        push_hud_debug_chart_label(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &avg_text,
            left + i32::try_from(width / 2).unwrap_or(i32::MAX) - avg_width / 2,
            top.saturating_sub(HUD_DEBUG_CHART_LABEL_HEIGHT),
        );
        let max_text = format!("{} max", hud_debug_tps_chart_display_string(max as f64));
        let max_width = hud_coded_string_text_width(&max_text, glyphs);
        push_hud_debug_chart_label(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &max_text,
            left + i32::try_from(width).unwrap_or(i32::MAX) - max_width - 2,
            top.saturating_sub(HUD_DEBUG_CHART_LABEL_HEIGHT),
        );
    }

    push_hud_debug_chart_shaded_label(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &hud_debug_tps_chart_tps_label(chart.milliseconds_per_tick),
        left + 1,
        top + 1,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_network_charts<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    charts: &HudDebugNetworkCharts,
) {
    let width = hud_debug_chart_width(surface_size);
    if width == 0 {
        return;
    }

    if charts.show_bandwidth {
        push_hud_debug_bandwidth_chart(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            0,
            width,
            &charts.bandwidth_bytes_per_tick,
        );
    }

    let surface_width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let left = surface_width.saturating_sub(i32::try_from(width).unwrap_or(i32::MAX));
    push_hud_debug_ping_chart(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        left,
        width,
        &charts.ping_millis,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_ping_chart<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    left: i32,
    width: u32,
    ping_millis: &[u64],
) {
    push_hud_debug_sample_chart(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        left,
        width,
        ping_millis,
        hud_debug_ping_chart_sample_height,
        hud_debug_ping_chart_sample_tint,
        hud_debug_ping_chart_display_string,
    );

    let bottom = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let top = bottom.saturating_sub(HUD_DEBUG_CHART_HEIGHT);
    push_hud_debug_chart_shaded_label(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        "500 ms",
        left + 1,
        top + 1,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_bandwidth_chart<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    left: i32,
    width: u32,
    bandwidth_bytes_per_tick: &[u64],
) {
    push_hud_debug_sample_chart(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        left,
        width,
        bandwidth_bytes_per_tick,
        hud_debug_bandwidth_chart_sample_height,
        hud_debug_bandwidth_chart_sample_tint,
        hud_debug_bandwidth_chart_display_string,
    );

    let bottom = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    push_hud_debug_bandwidth_chart_labeled_line(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        left,
        width,
        bottom,
        64,
    );
    push_hud_debug_bandwidth_chart_labeled_line(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        left,
        width,
        bottom,
        1_024,
    );
    push_hud_debug_bandwidth_chart_labeled_line(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        left,
        width,
        bottom,
        16_384,
    );
    push_hud_debug_chart_shaded_label(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &hud_debug_bandwidth_chart_display_string_internal(1_048_576.0),
        left + 1,
        bottom
            .saturating_sub(hud_debug_bandwidth_chart_sample_height_internal(
                1_048_576.0,
            ))
            .saturating_add(1),
    );
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_bandwidth_chart_labeled_line<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    left: i32,
    width: u32,
    bottom: i32,
    bytes_per_second: u64,
) {
    let y = bottom.saturating_sub(hud_debug_bandwidth_chart_sample_height_internal(
        bytes_per_second as f64,
    ));
    push_hud_debug_chart_shaded_label(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &hud_debug_bandwidth_chart_display_string_internal(bytes_per_second as f64),
        left + 1,
        y + 1,
    );
    push_hud_debug_chart_horizontal_line(
        vertices,
        commands,
        surface_size,
        left,
        width,
        y,
        HUD_TINT_WHITE,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_sample_chart<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    left: i32,
    width: u32,
    samples: &[u64],
    sample_height: impl Fn(u64) -> i32,
    sample_tint: impl Fn(u64) -> [f32; 4],
    display_string: impl Fn(f64) -> String,
) {
    let bottom = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let top = bottom.saturating_sub(HUD_DEBUG_CHART_HEIGHT);
    push_hud_fill_rect(
        vertices,
        commands,
        surface_size,
        left,
        top,
        width,
        HUD_DEBUG_CHART_HEIGHT as u32,
        HUD_DEBUG_OVERLAY_BACKGROUND_TINT,
    );

    let visible_samples = hud_debug_chart_visible_samples(samples, width);
    for (index, sample) in visible_samples.iter().copied().enumerate() {
        let x = left + i32::try_from(index).unwrap_or(i32::MAX).saturating_add(1);
        let height = sample_height(sample);
        if height <= 0 {
            continue;
        }
        push_hud_fill_rect(
            vertices,
            commands,
            surface_size,
            x,
            bottom.saturating_sub(height),
            1,
            u32::try_from(height).unwrap_or(u32::MAX),
            sample_tint(sample),
        );
    }

    push_hud_debug_chart_horizontal_line(
        vertices,
        commands,
        surface_size,
        left,
        width,
        top,
        HUD_TINT_WHITE,
    );
    push_hud_debug_chart_horizontal_line(
        vertices,
        commands,
        surface_size,
        left,
        width,
        bottom.saturating_sub(1),
        HUD_TINT_WHITE,
    );
    push_hud_debug_chart_vertical_line(
        vertices,
        commands,
        surface_size,
        left,
        top,
        HUD_DEBUG_CHART_HEIGHT as u32,
        HUD_TINT_WHITE,
    );
    push_hud_debug_chart_vertical_line(
        vertices,
        commands,
        surface_size,
        left + i32::try_from(width.saturating_sub(1)).unwrap_or(i32::MAX),
        top,
        HUD_DEBUG_CHART_HEIGHT as u32,
        HUD_TINT_WHITE,
    );

    if !visible_samples.is_empty() {
        let min = visible_samples.iter().copied().min().unwrap_or(0);
        let max = visible_samples.iter().copied().max().unwrap_or(0);
        let avg = visible_samples
            .iter()
            .map(|sample| *sample as f64)
            .sum::<f64>()
            / visible_samples.len() as f64;
        push_hud_debug_chart_label(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &format!("{} min", display_string(min as f64)),
            left + 2,
            top.saturating_sub(HUD_DEBUG_CHART_LABEL_HEIGHT),
        );
        let avg_text = format!("{} avg", display_string(avg));
        let avg_width = hud_coded_string_text_width(&avg_text, glyphs);
        push_hud_debug_chart_label(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &avg_text,
            left + i32::try_from(width / 2).unwrap_or(i32::MAX) - avg_width / 2,
            top.saturating_sub(HUD_DEBUG_CHART_LABEL_HEIGHT),
        );
        let max_text = format!("{} max", display_string(max as f64));
        let max_width = hud_coded_string_text_width(&max_text, glyphs);
        push_hud_debug_chart_label(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &max_text,
            left + i32::try_from(width).unwrap_or(i32::MAX) - max_width - 2,
            top.saturating_sub(HUD_DEBUG_CHART_LABEL_HEIGHT),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_profiler_chart<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    chart: &HudDebugProfilerChart,
    bottom_offset: i32,
) {
    let global_percentage = hud_debug_profiler_percentage_text(chart.current_global_percentage);
    let global_percentage_width = hud_coded_string_text_width(&global_percentage, glyphs);
    let zero_prefix_width = hud_coded_string_text_width("[0] ", glyphs);
    let left = i32::try_from(surface_size.width)
        .unwrap_or(i32::MAX)
        .saturating_sub(HUD_DEBUG_PROFILER_WIDTH)
        .saturating_sub(HUD_DEBUG_PROFILER_RIGHT_MARGIN);
    let right = left.saturating_add(HUD_DEBUG_PROFILER_WIDTH);
    let top_text_max_width = right
        .saturating_sub(global_percentage_width)
        .saturating_sub(HUD_DEBUG_PROFILER_MARGIN)
        .saturating_sub(left)
        .saturating_sub(zero_prefix_width)
        .max(0);
    let node_name = hud_debug_profiler_demangle_path(&chart.current_node_name);
    let current_node_lines = hud_debug_profiler_split_node_name(
        &node_name,
        u32::try_from(top_text_max_width).unwrap_or(0),
        u32::try_from(
            top_text_max_width
                .saturating_sub(HUD_DEBUG_PROFILER_TEXT_INDENT)
                .max(0),
        )
        .unwrap_or(0),
        glyphs,
    );
    let layout = hud_debug_profiler_chart_layout(
        surface_size,
        chart.slices.len(),
        current_node_lines.len(),
        bottom_offset,
    );
    let background_left = layout.left.saturating_sub(HUD_DEBUG_PROFILER_MARGIN);
    let background_top = layout
        .current_node_top
        .saturating_sub(HUD_DEBUG_PROFILER_MARGIN);
    let background_right = layout.right.saturating_add(HUD_DEBUG_PROFILER_MARGIN);
    let background_bottom = layout.bottom.saturating_add(HUD_DEBUG_PROFILER_MARGIN);
    if background_right > background_left && background_bottom > background_top {
        push_hud_fill_rect(
            vertices,
            commands,
            surface_size,
            background_left,
            background_top,
            u32::try_from(background_right - background_left).unwrap_or(u32::MAX),
            u32::try_from(background_bottom - background_top).unwrap_or(u32::MAX),
            HUD_DEBUG_OVERLAY_BACKGROUND_TINT,
        );
    }

    push_hud_debug_profiler_pie(
        vertices,
        commands,
        white_pixel,
        surface_size,
        layout.chart_center_x as f32,
        layout.chart_center_y as f32,
        &chart.slices,
    );

    let mut first_line = String::new();
    if node_name != "unspecified" && node_name != "root" {
        first_line.push_str("[0] ");
    }
    first_line.push_str(
        current_node_lines
            .first()
            .map(String::as_str)
            .unwrap_or_default(),
    );
    push_hud_debug_profiler_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &first_line,
        layout.left,
        layout.current_node_top,
        HUD_TINT_WHITE,
    );
    for (index, line) in current_node_lines.iter().enumerate().skip(1) {
        push_hud_debug_profiler_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            line,
            layout
                .left
                .saturating_add(HUD_DEBUG_PROFILER_TEXT_INDENT)
                .saturating_add(zero_prefix_width),
            layout.current_node_top.saturating_add(
                i32::try_from(index).unwrap_or(i32::MAX) * HUD_DEBUG_CHART_LABEL_HEIGHT,
            ),
            HUD_TINT_WHITE,
        );
    }
    push_hud_debug_profiler_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &global_percentage,
        layout.right.saturating_sub(global_percentage_width),
        layout.current_node_top,
        HUD_TINT_WHITE,
    );

    for (index, slice) in chart.slices.iter().enumerate() {
        let text_y = layout.text_start_y.saturating_add(
            i32::try_from(index).unwrap_or(i32::MAX) * HUD_DEBUG_CHART_LABEL_HEIGHT,
        );
        let color = hud_argb_to_tint(hud_debug_profiler_slice_argb(&slice.name));
        let label = if slice.name == "unspecified" {
            format!("[?] {}", slice.name)
        } else {
            format!("[{}] {}", index + 1, slice.name)
        };
        push_hud_debug_profiler_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &label,
            layout.left,
            text_y,
            color,
        );
        let local_percentage = hud_debug_profiler_percentage_text(slice.percentage);
        let local_width = hud_coded_string_text_width(&local_percentage, glyphs);
        push_hud_debug_profiler_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &local_percentage,
            layout.right.saturating_sub(50).saturating_sub(local_width),
            text_y,
            color,
        );
        let global_percentage = hud_debug_profiler_percentage_text(slice.global_percentage);
        let global_width = hud_coded_string_text_width(&global_percentage, glyphs);
        push_hud_debug_profiler_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &global_percentage,
            layout.right.saturating_sub(global_width),
            text_y,
            color,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_profiler_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    text: &str,
    x: i32,
    y: i32,
    tint: [f32; 4],
) {
    // `ProfilerPieChart` draws every label with `graphics.text(font, String, ...)`
    push_hud_coded_string_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &hud_coded_string_runs(text),
        (x as f32, y as f32),
        tint,
        1.0,
        false,
    );
}

fn push_hud_debug_profiler_pie<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    center_x: f32,
    center_y: f32,
    slices: &[HudDebugProfilerSlice],
) {
    let start = vertices.len() as u32;
    let mut total_percentage = 0.0;
    for slice in slices {
        let percentage = slice.percentage.clamp(0.0, 100.0);
        if percentage <= f64::EPSILON {
            continue;
        }
        let steps = hud_debug_profiler_slice_steps(percentage);
        let color = hud_debug_profiler_slice_argb(&slice.name);
        let tint = hud_argb_to_tint(color);
        let shade_tint = hud_argb_to_tint(hud_argb_multiply(color | 0xFF00_0000, 0xFF80_8080));
        for step in (1..=steps).rev() {
            let p0 = hud_debug_profiler_pie_point(
                center_x,
                center_y,
                total_percentage + percentage * step as f64 / steps as f64,
            );
            let p1 = hud_debug_profiler_pie_point(
                center_x,
                center_y,
                total_percentage + percentage * (step - 1) as f64 / steps as f64,
            );
            push_hud_debug_profiler_triangle(
                vertices,
                surface_size,
                [center_x, center_y],
                p0,
                p1,
                tint,
            );
            let mid_y = ((p0[1] - center_y) + (p1[1] - center_y)) * 0.5;
            if mid_y >= 0.0 {
                push_hud_debug_profiler_quad(
                    vertices,
                    surface_size,
                    p0,
                    [p0[0], p0[1] + HUD_DEBUG_PROFILER_THICKNESS],
                    [p1[0], p1[1] + HUD_DEBUG_PROFILER_THICKNESS],
                    p1,
                    shade_tint,
                );
            }
        }
        total_percentage += percentage;
    }
    let end = vertices.len() as u32;
    if end > start {
        commands.push(HudDrawCommand::Sprite {
            sprite: white_pixel,
            start,
            end,
        });
    }
}

fn push_hud_debug_profiler_triangle(
    vertices: &mut Vec<HudVertex>,
    surface_size: PhysicalSize<u32>,
    a: [f32; 2],
    b: [f32; 2],
    c: [f32; 2],
    tint: [f32; 4],
) {
    vertices.push(hud_debug_profiler_vertex(surface_size, a, tint));
    vertices.push(hud_debug_profiler_vertex(surface_size, b, tint));
    vertices.push(hud_debug_profiler_vertex(surface_size, c, tint));
}

fn push_hud_debug_profiler_quad(
    vertices: &mut Vec<HudVertex>,
    surface_size: PhysicalSize<u32>,
    top_left: [f32; 2],
    bottom_left: [f32; 2],
    bottom_right: [f32; 2],
    top_right: [f32; 2],
    tint: [f32; 4],
) {
    let corners = [top_left, bottom_left, bottom_right, top_right];
    vertices.extend_from_slice(&hud_styled_quad_vertices(
        surface_size,
        corners,
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        tint,
    ));
}

fn push_hud_debug_chart_horizontal_line<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    surface_size: PhysicalSize<u32>,
    left: i32,
    width: u32,
    y: i32,
    tint: [f32; 4],
) {
    push_hud_fill_rect(vertices, commands, surface_size, left, y, width, 1, tint);
}

fn push_hud_debug_chart_vertical_line<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    surface_size: PhysicalSize<u32>,
    x: i32,
    top: i32,
    height: u32,
    tint: [f32; 4],
) {
    push_hud_fill_rect(vertices, commands, surface_size, x, top, 1, height, tint);
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_chart_shaded_label<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    text: &str,
    x: i32,
    y: i32,
) {
    let width = hud_coded_string_text_width(text, glyphs)
        .saturating_add(1)
        .max(0) as u32;
    push_hud_fill_rect(
        vertices,
        commands,
        surface_size,
        x,
        y,
        width,
        HUD_DEBUG_CHART_LABEL_HEIGHT as u32,
        HUD_DEBUG_OVERLAY_BACKGROUND_TINT,
    );
    push_hud_debug_chart_label(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        text,
        x + 1,
        y + 1,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_hud_debug_chart_label<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    text: &str,
    x: i32,
    y: i32,
) {
    // `AbstractDebugChart.drawStringWithShade` fills to `font.width(str)` and
    // draws with `graphics.text(font, String, ...)`
    push_hud_coded_string_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &hud_coded_string_runs(text),
        (x as f32, y as f32),
        HUD_DEBUG_OVERLAY_TEXT_TINT,
        1.0,
        false,
    );
}

#[cfg(test)]
#[path = "debug_draws/fill_tests.rs"]
mod fill_tests;
