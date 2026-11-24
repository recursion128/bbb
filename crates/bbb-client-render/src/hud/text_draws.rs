use super::*;

/// Vanilla `FormattedCharSequence.forward(text, Style.EMPTY)`
/// the string is the glyph sequence and a `§` keeps
/// its own glyph and advance. Every input-line caller needs this shape.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_raw_string_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    text: &str,
    origin: (f32, f32),
    tint: [f32; 4],
    scale: f32,
    shadow: bool,
) {
    if text.is_empty() {
        return;
    }
    let runs = [HudStyledTextRun::plain(text.to_string())];
    push_hud_coded_string_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        obfuscated_seed,
        surface_size,
        &runs,
        origin,
        tint,
        scale,
        shadow,
    );
}

/// Draws a `String` face that goes through `FormattedText.of(str)`
/// so `§` codes shape the run styles
/// instead of reaching the glyph pass. The caller resolves the text once
/// (`hud_coded_string_runs`) and feeds this and `hud_coded_string_width` from
/// that same list.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_coded_string_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    runs: &[HudStyledTextRun],
    origin: (f32, f32),
    tint: [f32; 4],
    scale: f32,
    shadow: bool,
) {
    if runs.iter().all(|run| run.text.is_empty()) {
        return;
    }
    let geometry = hud_styled_text_geometry(
        runs,
        glyphs,
        obfuscated_pool,
        obfuscated_seed,
        origin,
        shadow,
        tint,
        None,
        scale,
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
pub(super) fn push_hud_command_suggestion_tooltip<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    tooltip_background: Option<&'a HudSpriteGpu>,
    tooltip_frame: Option<&'a HudSpriteGpu>,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    tooltip: &HudCommandSuggestionTooltip,
) {
    push_hud_rich_tooltip(
        vertices,
        commands,
        white_pixel,
        tooltip_background,
        tooltip_frame,
        font_atlas,
        glyphs,
        obfuscated_pool,
        obfuscated_seed,
        surface_size,
        &tooltip.runs,
        tooltip.x,
        tooltip.y,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_rich_tooltip<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    tooltip_background: Option<&'a HudSpriteGpu>,
    tooltip_frame: Option<&'a HudSpriteGpu>,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    runs: &[HudStyledTextRun],
    x: i32,
    y: i32,
) {
    let tooltip_lines = hud_wrap_chat_runs_with_interactions(runs, &[], 210, glyphs)
        .into_iter()
        .map(|line| line.runs)
        .collect::<Vec<_>>();
    let Some(text_width) = tooltip_lines
        .iter()
        .filter_map(|line| hud_font_runs_width(line, glyphs))
        .max()
    else {
        return;
    };
    let Some(text_height) = hud_inventory_tooltip_text_height(tooltip_lines.len()) else {
        return;
    };
    let background_rect = clip_hud_rect_to_viewport(
        hud_inventory_tooltip_background_hud_rect(
            surface_size,
            surface_size.width,
            surface_size.height,
            x,
            y,
            text_width,
            text_height,
        ),
        surface_size,
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
        _ => push_hud_draw_with_uv_and_tint(
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
        ),
    }
    for (line_index, line) in tooltip_lines.iter().enumerate() {
        let origin = hud_inventory_tooltip_line_origin(
            surface_size,
            surface_size.width,
            surface_size.height,
            x,
            y,
            text_width,
            text_height,
            line_index,
        );
        let geometry = hud_styled_text_geometry(
            line,
            glyphs,
            obfuscated_pool,
            obfuscated_seed,
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
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_component_tooltip<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    tooltip_background: Option<&'a HudSpriteGpu>,
    tooltip_frame: Option<&'a HudSpriteGpu>,
    tooltip_styles: &'a GuiSprites,
    font_atlas: &'a HudSpriteGpu,
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    tooltip: &HudRichTooltip,
    advanced_item_tooltips: bool,
    x: i32,
    y: i32,
) {
    let source_lines = hud_rich_tooltip_lines(tooltip, advanced_item_tooltips);
    let lines = if tooltip.wrap_text {
        let max_width = (surface_size.width / 2).max(200);
        source_lines
            .iter()
            .flat_map(|line| {
                hud_wrap_component_runs_with_interactions(&line.runs, &[], max_width, fonts)
                    .into_iter()
                    .map(|wrapped| HudRichTooltipLine {
                        runs: wrapped.runs,
                        tint: line.tint,
                    })
            })
            .collect::<Vec<_>>()
    } else {
        source_lines
            .iter()
            .map(|line| HudRichTooltipLine {
                runs: hud_resolved_tooltip_runs(&line.runs),
                tint: line.tint,
            })
            .collect::<Vec<_>>()
    };
    push_hud_resolved_tooltip(
        vertices,
        commands,
        white_pixel,
        tooltip_background,
        tooltip_frame,
        tooltip_styles,
        font_atlas,
        fonts,
        obfuscated_seed,
        surface_size,
        surface_size.width,
        surface_size.height,
        x,
        y,
        tooltip.style.as_deref(),
        lines.iter().map(|line| (line.runs.as_slice(), line.tint)),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_resolved_tooltip<'a, 'b>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    tooltip_background: Option<&'a HudSpriteGpu>,
    tooltip_frame: Option<&'a HudSpriteGpu>,
    tooltip_styles: &'a GuiSprites,
    font_atlas: &'a HudSpriteGpu,
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
    x: i32,
    y: i32,
    style: Option<&str>,
    lines: impl Clone + ExactSizeIterator<Item = (&'b [HudComponentTextRun], [f32; 4])>,
) {
    let Some(text_height) = hud_inventory_tooltip_text_height(lines.len()) else {
        return;
    };
    let Some(text_width) = lines
        .clone()
        .filter_map(|(runs, _)| hud_component_runs_width(runs, fonts))
        .max()
    else {
        return;
    };
    let background_rect = clip_hud_rect_to_viewport(
        hud_inventory_tooltip_background_hud_rect(
            surface_size,
            screen_width,
            screen_height,
            x,
            y,
            text_width,
            text_height,
        ),
        surface_size,
    );
    let sprites = match style {
        Some(style) => tooltip_styles.tooltip_style(style),
        None => tooltip_background.zip(tooltip_frame),
    };
    if let Some((background, frame)) = sprites {
        // Vanilla `TooltipRenderUtil.extractTooltipBackground`: default nine-slice
        // `tooltip/background` then `tooltip/frame` over the same padded rect.
        push_hud_gui_sprite(
            vertices,
            commands,
            background,
            surface_size,
            background_rect,
        );
        push_hud_gui_sprite(vertices, commands, frame, surface_size, background_rect);
    } else {
        push_hud_inventory_tooltip_flat_background(
            vertices,
            commands,
            white_pixel,
            surface_size,
            background_rect,
        );
    }
    for (line_index, (runs, tint)) in lines.enumerate() {
        let origin = hud_inventory_tooltip_line_origin(
            surface_size,
            screen_width,
            screen_height,
            x,
            y,
            text_width,
            text_height,
            line_index,
        );
        let geometry = hud_component_text_geometry(
            runs,
            fonts,
            obfuscated_seed,
            origin,
            true,
            tint,
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
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_chat<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    fonts: &HudFontGlyphBook,
    modified_icon: Option<&'a HudSpriteGpu>,
    tooltip_background: Option<&'a HudSpriteGpu>,
    tooltip_frame: Option<&'a HudSpriteGpu>,
    tooltip_styles: &'a GuiSprites,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    geometry: HudChatGeometry,
    lines: &[HudChatLine],
    context: &HudChatContext,
    background_opacity: f32,
) {
    let visible_rows = geometry.visible_rows(lines, context);
    let text_opacity = geometry.text_opacity;
    let queue_prompt = context
        .queue_text
        .as_deref()
        .filter(|_| context.queued_messages > 0)
        .map(|text| hud_chat_queue_prompt(text, fonts));
    let restricted_prompt = context
        .restricted_text
        .as_deref()
        .map(|text| hud_chat_restricted_prompt(text, fonts, geometry.max_width));
    let mut fill = |vertices: &mut Vec<HudVertex>, rect: [i32; 4], tint: [f32; 4]| {
        push_hud_fill_corners(
            vertices,
            commands,
            surface_size,
            geometry.fill_corners(rect),
            tint,
        );
    };

    for &(row, _, alpha) in &visible_rows {
        fill(
            vertices,
            [
                -4,
                geometry.entry_top(row),
                geometry.max_width.wrapping_add(8),
                geometry.entry_bottom(row),
            ],
            [
                0.0,
                0.0,
                0.0,
                hud_argb_black_alpha(alpha * background_opacity),
            ],
        );
    }
    if context.queued_messages > 0 {
        fill(
            vertices,
            [
                -2,
                geometry.chat_bottom,
                geometry.max_width.wrapping_add(4),
                geometry.chat_bottom.wrapping_add(HUD_CHAT_MESSAGE_HEIGHT),
            ],
            [0.0, 0.0, 0.0, hud_argb_black_alpha(background_opacity)],
        );
    }
    let line_above_messages = geometry.line_above_messages(visible_rows.len());
    if restricted_prompt.is_some() {
        fill(
            vertices,
            [
                -2,
                line_above_messages,
                geometry.max_width.wrapping_add(8),
                line_above_messages.wrapping_add(geometry.entry_height),
            ],
            [0.0, 0.0, 0.0, hud_argb_black_alpha(background_opacity)],
        );
    }

    let modified_icons = hud_chat_modified_icons(lines, context, fonts, geometry);
    for &(row, line_index, alpha) in &visible_rows {
        let line = &lines[line_index];
        let text_top = geometry.text_top(row);
        let text_alpha = alpha * text_opacity;
        if let Some(cached) = &line.cached_geometry {
            push_hud_posed_text_geometry(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                surface_size,
                &cached.geometry,
                geometry,
                text_top,
                text_alpha,
            );
        } else {
            push_hud_chat_component_text(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                fonts,
                frame_index,
                surface_size,
                geometry,
                &line.runs,
                text_top,
                text_alpha,
            );
        }
        if let Some(tag) = line.tag.as_ref() {
            let color = tag.indicator_color;
            push_hud_fill_corners(
                vertices,
                commands,
                surface_size,
                geometry.fill_corners([
                    -4,
                    geometry.entry_top(row),
                    -2,
                    geometry.entry_bottom(row),
                ]),
                [
                    ((color >> 16) & 0xff) as f32 / 255.0,
                    ((color >> 8) & 0xff) as f32 / 255.0,
                    (color & 0xff) as f32 / 255.0,
                    hud_argb_black_alpha(text_alpha),
                ],
            );
        }
        if let Some(icon) = modified_icon {
            for (_, rect) in modified_icons
                .iter()
                .filter(|(icon_row, _)| *icon_row == row)
            {
                push_hud_draw_with_corners_uv_and_tint(
                    vertices,
                    commands,
                    icon,
                    surface_size,
                    geometry.fill_corners(*rect),
                    HudUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                    HUD_TINT_WHITE,
                );
            }
        }
    }
    if let Some(prompt) = &queue_prompt {
        push_hud_chat_component_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            fonts,
            frame_index,
            surface_size,
            geometry,
            &prompt.runs,
            geometry.queue_text_top(),
            0.5 * text_opacity,
        );
    }
    if let Some(prompt) = &restricted_prompt {
        push_hud_chat_component_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            fonts,
            frame_index,
            surface_size,
            geometry,
            &prompt.runs,
            geometry.restricted_text_top(visible_rows.len()),
            text_opacity,
        );
    }
    if context.focused {
        for (rect, color) in geometry
            .scrollbar(lines.len(), visible_rows.len(), context.scroll)
            .into_iter()
            .flatten()
        {
            push_hud_fill_corners(
                vertices,
                commands,
                surface_size,
                geometry.fill_corners(rect),
                [
                    ((color >> 16) & 0xff) as f32 / 255.0,
                    ((color >> 8) & 0xff) as f32 / 255.0,
                    (color & 0xff) as f32 / 255.0,
                    (color >> 24) as f32 / 255.0,
                ],
            );
        }
    }
    let Some((cursor_x, cursor_y)) = context.cursor_position.filter(|_| context.focused) else {
        return;
    };
    let position = (f64::from(cursor_x), f64::from(cursor_y));
    // The tag tooltip is deferred during extraction and the hovered text
    // style's only afterwards, so the tag wins (1263-1281).
    if let Some(tag) = hud_chat_tag_at(lines, context, geometry, position, fonts)
        .filter(|tag| !tag.tooltip_runs.is_empty())
    {
        push_hud_rich_tooltip(
            vertices,
            commands,
            white_pixel,
            tooltip_background,
            tooltip_frame,
            font_atlas,
            fonts.glyphs(None),
            fonts.obfuscated_pool(None),
            frame_index,
            surface_size,
            &tag.tooltip_runs,
            cursor_x,
            cursor_y,
        );
        return;
    }
    let prompt_tooltip = hud_chat_prompt_tooltip(
        context,
        geometry,
        queue_prompt.as_ref(),
        restricted_prompt.as_ref(),
        visible_rows.len(),
    );
    let tooltip = prompt_tooltip.as_ref().or_else(|| {
        hud_chat_line_interaction_at(lines, context, fonts, geometry, position)
            .and_then(|interaction| interaction.tooltip.as_ref())
            .filter(|tooltip| {
                !hud_rich_tooltip_lines(tooltip, context.advanced_item_tooltips).is_empty()
            })
    });
    if let Some(tooltip) = tooltip {
        push_hud_component_tooltip(
            vertices,
            commands,
            white_pixel,
            tooltip_background,
            tooltip_frame,
            tooltip_styles,
            font_atlas,
            fonts,
            frame_index,
            surface_size,
            tooltip,
            context.advanced_item_tooltips,
            cursor_x,
            cursor_y,
        );
    }
}

/// A chat text `handleMessage` accepts at `(0, text_top)` under the chat pose
/// with the given opacity, for runs the glyph cache does not hold.
#[allow(clippy::too_many_arguments)]
fn push_hud_chat_component_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    fonts: &HudFontGlyphBook,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    geometry: HudChatGeometry,
    runs: &[HudComponentTextRun],
    text_top: i32,
    alpha: f32,
) {
    push_hud_component_screen_text_draw(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        fonts,
        frame_index,
        surface_size,
        &HudComponentScreenTextDraw {
            runs,
            origin: geometry.screen_point(0.0, text_top as f32).into(),
            scale: geometry.scale,
            tint: [1.0, 1.0, 1.0, alpha],
            backdrop_opacity: 0.0,
        },
    );
}

/// Draws the centered experience-level number with vanilla's 1px black outline
/// (`ContextualBarRenderer.extractExperienceLevel`,
/// ContextualBarRenderer): the level string `Component.translatable
/// ("gui.experience.level", level)` (the `"%s"` lang value, so just the number)
/// centered at `x = (guiWidth - width)/2`, `y = guiHeight - 24 - 9 - 2`, drawn as
/// four black axis-offset copies then the `0x80FF20` green center, all
/// `dropShadow = false`; each outline offset is part of the pen origin.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_experience_level_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    level: i32,
) {
    let runs = [HudStyledTextRun::plain(level.to_string())];
    let width = hud_font_runs_width(&runs, glyphs).unwrap_or(0);
    let (base_x, base_y) = hud_experience_level_text_origin(surface_size, width);
    let geometry = hud_styled_text_geometry(
        &runs,
        glyphs,
        obfuscated_pool,
        frame_index,
        (0.0, 0.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    for (dx, dy, tint) in HUD_EXPERIENCE_LEVEL_PASSES {
        // Match the layout's ARGB byte round-trip before tinting the white glyphs.
        let tint = tint.map(|channel| hud_argb_alpha_channel(channel) as f32 / 255.0);
        push_hud_text_geometry_with(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            surface_size,
            &geometry,
            |x, y| [(base_x + dx) + x, (base_y + dy) + y],
            |_| tint,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_hud_text_backdrop<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    surface_size: PhysicalSize<u32>,
    origin: (f32, f32),
    text_width: i32,
    scale: f32,
    opacity: f32,
    text_alpha: f32,
) {
    let alpha = hud_text_backdrop_alpha(opacity, text_alpha);
    let Some(rect) = hud_text_backdrop_rect(origin, text_width, scale) else {
        return;
    };
    if alpha <= 0.0 {
        return;
    }
    push_hud_fill(
        vertices,
        commands,
        surface_size,
        rect,
        [0.0, 0.0, 0.0, alpha],
    );
}

/// Draws one resolved screen text line through the styled pipeline with the
/// vanilla `textWithBackdrop` pass order:
/// optional accessibility backdrop, then glyphs and decoration effects.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_screen_text_draw<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    draw: &HudScreenTextDraw<'_>,
) {
    push_hud_text_backdrop(
        vertices,
        commands,
        surface_size,
        draw.origin,
        hud_font_runs_width(draw.runs, glyphs).unwrap_or_default(),
        draw.scale,
        draw.backdrop_opacity,
        draw.tint[3],
    );
    let geometry = hud_styled_text_geometry(
        draw.runs,
        glyphs,
        obfuscated_pool,
        frame_index,
        draw.origin,
        true,
        draw.tint,
        None,
        draw.scale,
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
pub(super) fn push_hud_component_screen_text_draw<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    fonts: &HudFontGlyphBook,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    draw: &HudComponentScreenTextDraw<'_>,
) {
    push_hud_text_backdrop(
        vertices,
        commands,
        surface_size,
        draw.origin,
        hud_component_runs_width(draw.runs, fonts).unwrap_or_default(),
        draw.scale,
        draw.backdrop_opacity,
        draw.tint[3],
    );
    let geometry = hud_component_text_geometry(
        draw.runs,
        fonts,
        frame_index,
        draw.origin,
        true,
        draw.tint,
        None,
        draw.scale,
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

pub(super) fn push_hud_styled_text_geometry<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    geometry: &HudStyledTextGeometry,
) {
    push_hud_text_geometry_with(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        surface_size,
        geometry,
        |x, y| [x, y],
        |tint| tint,
    );
}

#[inline]
#[allow(clippy::too_many_arguments)]
fn push_hud_text_geometry_with<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    geometry: &HudStyledTextGeometry,
    pose: impl Fn(f32, f32) -> [f32; 2],
    color: impl Fn([f32; 4]) -> [f32; 4],
) {
    for (quad, tint) in &geometry.glyph_quads {
        let start = vertices.len() as u32;
        vertices.extend_from_slice(&hud_styled_quad_vertices(
            surface_size,
            quad.corners.map(|[x, y]| pose(x, y)),
            quad.uv,
            color(*tint),
        ));
        let end = vertices.len() as u32;
        push_hud_sprite_batch(commands, font_atlas, start, end);
    }
    for (rect, tint) in &geometry.effect_rects {
        let start = vertices.len() as u32;
        vertices.extend_from_slice(&hud_styled_quad_vertices(
            surface_size,
            [
                pose(rect.x0, rect.y0),
                pose(rect.x0, rect.y1),
                pose(rect.x1, rect.y1),
                pose(rect.x1, rect.y0),
            ],
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            color(*tint),
        ));
        let end = vertices.len() as u32;
        push_hud_sprite_batch(commands, white_pixel, start, end);
    }
}

/// Replays a cached chat line's font-space glyphs and effects through the chat
/// pose, the same `(0, text_top)` acceptance the pointer tests invert.
#[allow(clippy::too_many_arguments)]
fn push_hud_posed_text_geometry<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    text: &HudStyledTextGeometry,
    geometry: HudChatGeometry,
    text_top: i32,
    alpha: f32,
) {
    push_hud_text_geometry_with(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        surface_size,
        text,
        |x, y| geometry.screen_point(x, y + text_top as f32),
        |mut tint| {
            tint[3] = hud_argb_alpha_channel(tint[3] * alpha) as f32 / 255.0;
            tint
        },
    );
}

#[cfg(test)]
#[path = "text_draws/fill_tests.rs"]
mod fill_tests;
