use super::*;

pub(super) fn centered_screen_origin(
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
) -> (i64, i64) {
    (
        (i64::from(surface_size.width.max(1)) - i64::from(screen_width)) / 2,
        (i64::from(surface_size.height.max(1)) - i64::from(screen_height)) / 2,
    )
}

pub(super) fn push_dialog_outline_layers(
    layers: &mut Vec<HudInventoryFillLayer>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    let right = x.saturating_add(width);
    let bottom = y.saturating_add(height);
    push_dialog_fill_layer(layers, x, y, right, y.saturating_add(1));
    push_dialog_fill_layer(layers, x, bottom.saturating_sub(1), right, bottom);
    push_dialog_fill_layer(
        layers,
        x,
        y.saturating_add(1),
        x.saturating_add(1),
        bottom.saturating_sub(1),
    );
    push_dialog_fill_layer(
        layers,
        right.saturating_sub(1),
        y.saturating_add(1),
        right,
        bottom.saturating_sub(1),
    );
}

pub(super) fn push_dialog_fill_layer(
    layers: &mut Vec<HudInventoryFillLayer>,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) {
    let width = x0.abs_diff(x1);
    let height = y0.abs_diff(y1);
    if width == 0 || height == 0 {
        return;
    }
    layers.push(HudInventoryFillLayer {
        x: x0.min(x1),
        y: y0.min(y1),
        width,
        height,
        tint: TEXT_COLOR,
        stage: HudInventoryFillStage::Foreground,
    });
}

#[cfg(test)]
pub(super) fn dialog_text_block(message: &DialogPlainMessageState) -> Option<DialogTextBlock> {
    dialog_text_block_with(message, DialogTextContext::approximate(None))
}

pub(super) fn dialog_text_block_with(
    message: &DialogPlainMessageState,
    text_ctx: DialogTextContext<'_>,
) -> Option<DialogTextBlock> {
    let width = u32::try_from(message.width).ok()?;
    let text_width = width
        .saturating_sub(u32::try_from(FOCUSABLE_TEXT_PADDING * 2).unwrap_or_default())
        .max(1);
    let lines = wrapped_text_lines_with(&message.contents, text_width, text_ctx);
    let height = i32::try_from(lines.len())
        .unwrap_or(i32::MAX)
        .saturating_mul(TEXT_LINE_HEIGHT)
        .saturating_add(FOCUSABLE_TEXT_PADDING * 2);
    Some(DialogTextBlock {
        width,
        height,
        lines,
    })
}

#[cfg(test)]
pub(super) fn dialog_text_click_event_at(
    message: &DialogPlainMessageState,
    rectangle: DialogFocusRectangle,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<ComponentClickEvent> {
    dialog_text_click_event_at_with(
        message,
        rectangle,
        cursor_position,
        surface_size,
        DialogTextContext::approximate(None),
    )
}

pub(super) fn dialog_text_click_event_at_with(
    message: &DialogPlainMessageState,
    rectangle: DialogFocusRectangle,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    text_ctx: DialogTextContext<'_>,
) -> Option<ComponentClickEvent> {
    let cursor = cursor_position?;
    if !cursor.x.is_finite() || !cursor.y.is_finite() {
        return None;
    }
    let screen_width = surface_size.width.max(1);
    let screen_height = surface_size.height.max(1);
    let (origin_x, origin_y) = centered_screen_origin(surface_size, screen_width, screen_height);
    let cursor_x = i32::try_from((cursor.x.floor() as i64).saturating_sub(origin_x)).ok()?;
    let cursor_y = i32::try_from((cursor.y.floor() as i64).saturating_sub(origin_y)).ok()?;
    let line_y = cursor_y
        .saturating_sub(rectangle.y)
        .saturating_sub(FOCUSABLE_TEXT_PADDING);
    if line_y < 0 || line_y >= rectangle.height.saturating_sub(FOCUSABLE_TEXT_PADDING * 2) {
        return None;
    }
    let line_index = usize::try_from(line_y / TEXT_LINE_HEIGHT).ok()?;
    let block = dialog_text_block_with(message, text_ctx)?;
    let line = block.lines.get(line_index)?;
    let line_x = rectangle
        .x
        .saturating_add(rectangle.width / 2)
        .saturating_sub(i32::try_from(line.width).unwrap_or(i32::MAX) / 2);
    let x = cursor_x.saturating_sub(line_x);
    if x < 0 {
        return None;
    }
    let x = u32::try_from(x).ok()?;
    line.click_spans
        .iter()
        .find(|span| x >= span.x && x < span.x.saturating_add(span.width))
        .map(|span| span.event.clone())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn dialog_item_block(
    world: &WorldStore,
    item_models: &HudItemModelContext<'_>,
    text_ctx: DialogTextContext<'_>,
    terrain_textures: &TerrainTextureState,
    local_state: &DialogHudLocalState,
    item: &DialogItemStackTemplateState,
    description: Option<&DialogPlainMessageState>,
    show_decorations: bool,
    show_tooltip: bool,
    width: i32,
    height: i32,
    partial_tick: f32,
) -> Option<DialogItemBlock> {
    let item_runtime = text_ctx.item_runtime?;
    let stack =
        item.item_stack_summary_for_protocol_id(item_runtime.item_protocol_id(&item.resource_id)?)?;
    let projection = item_models.project(
        &stack,
        HudItemSlotState {
            shift_down: local_state.shift_down,
            keybind: local_state.keybind_context,
            ..HudItemSlotState::default()
        },
        world.local_player_team_color_rgb(),
    );
    let icon = projection
        .as_ref()
        .and_then(|projection| projection.icon(partial_tick))?;
    Some(DialogItemBlock {
        item: HudInventoryItem {
            x: 0,
            y: 0,
            scale: 1.0,
            scale_y: 1.0,
            icon,
            scissor: None,
            draw_decorations: show_decorations,
            block_model: projection
                .as_ref()
                .and_then(|projection| projection.block_model(terrain_textures)),
        },
        stack,
        widget_width: width,
        widget_height: height,
        description: description.and_then(|desc| dialog_text_block_with(desc, text_ctx)),
        show_tooltip,
    })
}

pub(super) fn push_dialog_text_block(
    labels: &mut Vec<HudInventoryTextLabel>,
    text: DialogTextBlock,
    x: i32,
    y: i32,
) {
    let center_x = x.saturating_add(i32::try_from(text.width).unwrap_or(i32::MAX) / 2);
    let mut line_y = y.saturating_add(FOCUSABLE_TEXT_PADDING);
    for line in text.lines {
        let line_width_i32 = i32::try_from(line.width).unwrap_or(i32::MAX);
        labels.push(HudInventoryTextLabel {
            component_runs: Vec::new(),
            x: center_x.saturating_sub(line_width_i32 / 2),
            y: line_y,
            width: line.width.max(1),
            text: line.text,
            tint: TEXT_COLOR,
            background: None,
            input: None,
            scrolling: None,
            shadow: false,
            font: None,
            runs: line.runs,
        });
        line_y = line_y.saturating_add(TEXT_LINE_HEIGHT);
    }
}

pub(super) fn vertical_center_offset(available_height: i32, child_height: i32) -> i32 {
    available_height
        .saturating_sub(child_height)
        .saturating_add(1)
        / 2
}

#[allow(clippy::too_many_arguments)]
pub(super) fn dialog_widget_contains(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
) -> bool {
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (origin_x, origin_y) = centered_screen_origin(surface_size, screen_width, screen_height);
    let left = origin_x + i64::from(x);
    let top = origin_y + i64::from(y);
    cursor.x >= left as f64
        && cursor.x < (left + i64::from(width)) as f64
        && cursor.y >= top as f64
        && cursor.y < (top + i64::from(height)) as f64
}

#[allow(clippy::too_many_arguments)]
pub(super) fn dialog_item_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    stack: &ItemStackSummary,
    advanced: bool,
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
    block_index: usize,
) -> Option<HudInventoryTooltip> {
    let (cursor_x, cursor_y) = cursor_position?;
    let (origin_x, origin_y) = centered_screen_origin(surface_size, screen_width, screen_height);
    let tooltip = hud_item_tooltip(
        world,
        item_runtime,
        stack,
        advanced,
        world.gameplay().game_type == GameType::Creative.id(),
    )?;
    Some(HudInventoryTooltip {
        slot_id: u16::try_from(block_index).unwrap_or(u16::MAX),
        x: i32::try_from(i64::from(cursor_x) - origin_x).ok()?,
        y: i32::try_from(i64::from(cursor_y) - origin_y).ok()?,
        style: tooltip.style,
        lines: tooltip.lines,
    })
}

pub(super) fn inworld_menu_background_layer(
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
) -> HudInventoryBackgroundLayer {
    let surface_width = surface_size.width.max(1);
    let surface_height = surface_size.height.max(1);
    let (origin_x, origin_y) = centered_screen_origin(surface_size, screen_width, screen_height);
    HudInventoryBackgroundLayer {
        texture: HudInventoryBackgroundTexture::Texture("textures/gui/inworld_menu_background.png"),
        x: i32::try_from(-origin_x).unwrap_or(i32::MIN),
        y: i32::try_from(-origin_y).unwrap_or(i32::MIN),
        width: surface_width,
        height: surface_height,
        uv: HudUvRect {
            min: [0.0, 0.0],
            // Screen.extractMenuBackgroundTexture declares a logical 32x32
            // texture size even though vanilla's PNG is 16x16, so each source
            // texel occupies two GUI pixels before the repeat sampler wraps.
            max: [surface_width as f32 / 32.0, surface_height as f32 / 32.0],
        },
    }
}

pub(super) fn push_centered_plain_text_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    text: &str,
    center_x: i32,
    y: i32,
    tint: [f32; 4],
    glyphs: &bbb_client_render::HudFontGlyphMap,
) {
    let width = glyphs.plain_text_width(text).max(0) as u32;
    labels.push(HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: center_x - i32::try_from(width).unwrap_or(i32::MAX) / 2,
        y,
        width: width.max(1),
        text: text.to_string(),
        tint,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs: Vec::new(),
    });
}

pub(super) fn push_centered_text_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    text: &DialogTextState,
    text_ctx: DialogTextContext<'_>,
    center_x: i32,
    y: i32,
) {
    push_centered_runs_text_label(
        labels,
        dialog_text_runs_with(text, text_ctx.item_runtime),
        center_x,
        y,
        None,
        text_ctx.glyphs,
    );
}

pub(super) fn push_centered_dialog_control_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    action: DialogControlAction<'_>,
    text_ctx: DialogTextContext<'_>,
    center_x: i32,
    y: i32,
) {
    if let Some(label) = action.styled_label() {
        push_centered_text_label(labels, label, text_ctx, center_x, y);
    } else {
        push_centered_runs_text_label(
            labels,
            vec![HudStyledTextRun::plain(action.label_text())],
            center_x,
            y,
            None,
            text_ctx.glyphs,
        );
    }
}

pub(super) fn push_centered_runs_text_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    runs: Vec<HudStyledTextRun>,
    center_x: i32,
    y: i32,
    width_limit: Option<u32>,
    glyphs: &bbb_client_render::HudFontGlyphMap,
) {
    let text = runs.iter().map(|run| run.text.as_str()).collect::<String>();
    let measured_width = runs_width(&runs, glyphs).max(1);
    let width = width_limit.map_or(measured_width, |limit| measured_width.min(limit.max(1)));
    labels.push(HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: center_x.saturating_sub(i32::try_from(width).unwrap_or(i32::MAX) / 2),
        y,
        width,
        text,
        tint: TEXT_COLOR,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs,
    });
}

pub(super) fn push_scrolling_widget_text_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    runs: Vec<HudStyledTextRun>,
    layout: DialogInputLayout,
    glyphs: &bbb_client_render::HudFontGlyphMap,
) {
    let text = runs.iter().map(|run| run.text.as_str()).collect::<String>();
    let bounds = HudInventoryScrollingText {
        x: layout
            .x
            .saturating_add(i32::try_from(WIDGET_TEXT_MARGIN).unwrap_or(i32::MAX)),
        y: layout.y,
        width: u32::try_from(layout.width)
            .unwrap_or(1)
            .saturating_sub(WIDGET_TEXT_MARGIN.saturating_mul(2))
            .max(1),
        height: u32::try_from(layout.height).unwrap_or(1).max(1),
    };
    let measured_width = runs_width(&runs, glyphs).max(1);
    let width = measured_width.min(bounds.width);
    labels.push(HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: layout
            .x
            .saturating_add(layout.width / 2)
            .saturating_sub(i32::try_from(width).unwrap_or(i32::MAX) / 2),
        y: layout.y.saturating_add(BUTTON_TEXT_Y_OFFSET),
        width,
        text,
        tint: TEXT_COLOR,
        background: None,
        input: None,
        scrolling: Some(bounds),
        shadow: false,
        font: None,
        runs,
    });
}

pub(super) fn push_text_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    text: &DialogTextState,
    text_ctx: DialogTextContext<'_>,
    x: i32,
    y: i32,
) {
    let runs = dialog_text_runs_with(text, text_ctx.item_runtime);
    let resolved_text = runs.iter().map(|run| run.text.as_str()).collect::<String>();
    let width = runs_width(&runs, text_ctx.glyphs).max(1);
    labels.push(HudInventoryTextLabel {
        component_runs: Vec::new(),
        x,
        y,
        width,
        text: resolved_text,
        tint: TEXT_COLOR,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs,
    });
}

struct DialogWrappedGlyph {
    ch: char,
    run_style: HudStyledTextRun,
    click_event: Option<ComponentClickEvent>,
    advance: f32,
}

#[cfg(test)]
pub(super) fn wrapped_text_lines(text: &DialogTextState, max_width: u32) -> Vec<DialogTextLine> {
    wrapped_text_lines_with(text, max_width, DialogTextContext::approximate(None))
}

/// Wraps one dialog text value at word boundaries using real font glyph metrics
/// and derives its click spans.
///
/// `MultiLineLabel` splits the localized `Component` with `Style.EMPTY` as the
/// reset and then renders every line through `getVisualOrder`, so `§` codes are
/// consumed before the break position, drawn runs, and glyph-area click spans
/// are decided.
pub(super) fn wrapped_text_lines_with(
    text: &DialogTextState,
    max_width: u32,
    text_ctx: DialogTextContext<'_>,
) -> Vec<DialogTextLine> {
    let source_runs = localized_dialog_text_runs(text, text_ctx.item_runtime);
    let mut glyphs = Vec::new();
    crate::component_text::visit_legacy_component_runs(&source_runs, |ch, style, _part| {
        let run_style = hud_dialog_style_run(style);
        let advance = if ch == '\n' {
            0.0
        } else {
            text_ctx.glyphs.styled_char_advance(ch, run_style.style)
        };
        glyphs.push(DialogWrappedGlyph {
            ch,
            run_style,
            click_event: style.click_event.clone(),
            advance,
        });
    });
    if glyphs.is_empty() {
        return vec![empty_text_line()];
    }

    let max_width = max_width.max(1) as f32;
    let mut lines = Vec::new();
    let mut begin = 0usize;
    let mut trailing_newline = false;
    while begin < glyphs.len() {
        let mut width = 0.0f32;
        let mut had_nonzero = false;
        let mut last_space = None;
        let mut end = glyphs.len();
        for (index, glyph) in glyphs.iter().enumerate().skip(begin) {
            if glyph.ch == '\n' {
                end = index;
                break;
            }
            if glyph.ch == ' ' {
                last_space = Some(index);
            }
            width += glyph.advance;
            if had_nonzero && width > max_width {
                end = last_space.unwrap_or(index);
                break;
            }
            had_nonzero |= glyph.advance != 0.0;
        }
        lines.push(build_dialog_text_line(&glyphs[begin..end]));
        trailing_newline = end < glyphs.len() && glyphs[end].ch == '\n';
        let skip = end < glyphs.len() && matches!(glyphs[end].ch, ' ' | '\n');
        begin = end + usize::from(skip);
    }
    if trailing_newline {
        lines.push(empty_text_line());
    }
    lines
}

fn build_dialog_text_line(glyphs: &[DialogWrappedGlyph]) -> DialogTextLine {
    let mut line = empty_text_line();
    let mut width = 0.0f32;
    let mut active_click: Option<(f32, f32, ComponentClickEvent)> = None;
    for glyph in glyphs {
        let next_width = width + glyph.advance;
        match (&mut active_click, &glyph.click_event) {
            (Some((_, end, event)), Some(next_event)) if *event == *next_event => {
                *end = next_width;
            }
            (slot, next_event) => {
                if let Some((start, end, event)) = slot.take() {
                    push_click_span(&mut line.click_spans, start, end, event);
                }
                if let Some(event) = next_event {
                    *slot = Some((width, next_width, event.clone()));
                }
            }
        }
        line.text.push(glyph.ch);
        if let Some(last) = line.runs.last_mut().filter(|last| {
            last.style == glyph.run_style.style
                && last.color == glyph.run_style.color
                && last.shadow_color == glyph.run_style.shadow_color
        }) {
            last.text.push(glyph.ch);
        } else {
            let mut run = glyph.run_style.clone();
            run.text = glyph.ch.to_string();
            line.runs.push(run);
        }
        width = next_width;
    }
    if let Some((start, end, event)) = active_click {
        push_click_span(&mut line.click_spans, start, end, event);
    }
    line.width = width.ceil().max(0.0) as u32;
    line
}

fn push_click_span(
    spans: &mut Vec<DialogTextClickSpan>,
    start: f32,
    end: f32,
    event: ComponentClickEvent,
) {
    let x = start.floor().max(0.0) as u32;
    let end_x = end.ceil().max(0.0) as u32;
    spans.push(DialogTextClickSpan {
        x,
        width: end_x.saturating_sub(x),
        event,
    });
}

pub(super) fn plain_dialog_text_lines_with(
    text: &str,
    max_width: u32,
    glyphs: &bbb_client_render::HudFontGlyphMap,
) -> Vec<DialogTextLine> {
    wrapped_text_lines_with(
        &DialogTextState {
            text: text.to_string(),
            runs: Vec::new(),
            component: NbtSummaryValue::string(text),
        },
        max_width,
        DialogTextContext {
            item_runtime: None,
            glyphs,
        },
    )
}

#[cfg(test)]
pub(super) fn dialog_warning_message_lines(surface_size: PhysicalSize<u32>) -> Vec<DialogTextLine> {
    dialog_warning_message_lines_with(surface_size, ApproximateDialogTextMeasurer.glyphs())
}

pub(super) fn dialog_warning_message_lines_with(
    surface_size: PhysicalSize<u32>,
    glyphs: &bbb_client_render::HudFontGlyphMap,
) -> Vec<DialogTextLine> {
    let message_width = surface_size
        .width
        .max(1)
        .saturating_sub(CONFIRM_MESSAGE_HORIZONTAL_PADDING)
        .max(1);
    plain_dialog_text_lines_with(DIALOG_WARNING_CONTENTS, message_width, glyphs)
        .into_iter()
        .take(CONFIRM_MESSAGE_MAX_ROWS)
        .collect()
}

pub(super) fn empty_text_line() -> DialogTextLine {
    DialogTextLine {
        text: String::new(),
        width: 0,
        runs: Vec::new(),
        click_spans: Vec::new(),
    }
}

/// Canonical single owner for resolving any `DialogTextState` into localized
/// styled runs before legacy `§` formatting, glyph measurement, and wrapping.
pub(super) fn localized_dialog_text_runs(
    text: &DialogTextState,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<StyledTextRun> {
    bbb_client_assets::localized_component_runs(
        &text.component,
        &ComponentStyle::default(),
        &crate::component_text::ClientTextLookup(item_runtime),
    )
}

/// Vanilla widget and label text is a `Component` drawn through the visual
/// order (`AbstractStringWidget.renderString`), so the codes shape the runs the
/// label width and the drawn glyphs share.
pub(super) fn dialog_text_runs_with(
    text: &DialogTextState,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudStyledTextRun> {
    crate::component_text::legacy_resolved_runs(&localized_dialog_text_runs(text, item_runtime))
        .iter()
        .map(hud_dialog_text_run)
        .collect()
}

#[cfg(test)]
pub(super) fn dialog_text_runs(text: &DialogTextState) -> Vec<HudStyledTextRun> {
    dialog_text_runs_with(text, None)
}

pub(super) fn hud_dialog_text_run(run: &StyledTextRun) -> HudStyledTextRun {
    let mut visual = hud_dialog_style_run(&run.style);
    visual.text = run.text.clone();
    visual
}

pub(super) fn hud_dialog_style_run(style: &ComponentStyle) -> HudStyledTextRun {
    HudStyledTextRun {
        shadow_color: style.shadow_color,
        text: String::new(),
        style: crate::component_text::hud_text_style(style),
        color: style.color,
    }
}

pub(super) fn runs_width(
    runs: &[HudStyledTextRun],
    glyphs: &bbb_client_render::HudFontGlyphMap,
) -> u32 {
    runs.iter()
        .fold(0.0f32, |width, run| {
            run.text.chars().fold(width, |width, ch| {
                width + glyphs.styled_char_advance(ch, run.style)
            })
        })
        .ceil()
        .max(0.0) as u32
}

#[cfg(test)]
pub(super) fn runs_approx_width(runs: &[HudStyledTextRun]) -> u32 {
    runs_width(runs, ApproximateDialogTextMeasurer.glyphs())
}
