use super::*;
use std::ops::Range;

pub(super) fn text_origin(surface_size: PhysicalSize<u32>) -> (f32, f32) {
    (4.0, surface_size.height as f32 - 12.0)
}

pub(super) fn caret_origin(
    state: &HudCommandSuggestions,
    surface_size: PhysicalSize<u32>,
    glyphs: &HudFontGlyphMap,
) -> Option<(i32, i32)> {
    match state.layout {
        HudCommandSuggestionsLayout::Chat => {
            let view = state.input_view.as_ref()?;
            let origin = text_origin(surface_size);
            let layout = input_layout(
                view,
                &hud_command_input_runs(state),
                glyphs,
                surface_size.width.saturating_sub(4),
            );
            Some(((origin.0 + layout.cursor_x) as i32, origin.1 as i32))
        }
        HudCommandSuggestionsLayout::CommandBlock => {
            let rect = hud_command_block_editor_layout(surface_size).command_input;
            let input = HudInventoryTextInputDecoration {
                cursor: state.cursor,
                selection: state.selection,
                scroll_to: state.cursor,
                max_length: 32_500,
                cursor_visible: state.cursor_visible,
                cursor_tint: HUD_COMMAND_INPUT_TINT,
            };
            let label = HudInventoryTextLabel {
                component_runs: Vec::new(),
                x: 0,
                y: 0,
                width: rect.width.saturating_sub(8),
                text: state.input_text.clone(),
                tint: HUD_COMMAND_INPUT_TINT,
                background: None,
                input: Some(input),
                scrolling: None,
                shadow: true,
                font: None,
                runs: Vec::new(),
            };
            let layout = utf16_widget_input_layout(&label, input, glyphs);
            Some((
                rect.x
                    .saturating_add(4)
                    .saturating_add(layout.cursor_x as i32),
                rect.y.saturating_add(6),
            ))
        }
    }
}

pub(super) fn preedit_projection(
    state: &HudCommandSuggestions,
    surface_size: PhysicalSize<u32>,
    glyphs: &HudFontGlyphMap,
) -> Option<HudImePreeditProjection> {
    let preedit = state.preedit.as_ref()?;
    let origin = caret_origin(state, surface_size, glyphs)?;
    Some(ime_preedit::project(
        preedit,
        origin,
        10,
        surface_size,
        glyphs,
    ))
}

pub(super) fn ime_cursor_area(
    state: &HudCommandSuggestions,
    surface_size: PhysicalSize<u32>,
    glyphs: &HudFontGlyphMap,
) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>)> {
    if let Some(projection) = preedit_projection(state, surface_size, glyphs) {
        return Some((projection.ime_position, projection.ime_size));
    }
    let (x, y) = caret_origin(state, surface_size, glyphs)?;
    Some((PhysicalPosition::new(x, y), PhysicalSize::new(1, 10)))
}

pub(super) struct EditBoxLayout {
    pub(super) head: Vec<HudStyledTextRun>,
    pub(super) tail: Vec<HudStyledTextRun>,
    pub(super) tail_x: f32,
    pub(super) cursor_x: f32,
    pub(super) cursor_on_screen: bool,
    pub(super) insert: bool,
    pub(super) selection: Option<(f32, f32)>,
}

fn sliced_runs(
    value: &[u16],
    runs: &[HudStyledTextRun],
    range: Range<usize>,
) -> Vec<HudStyledTextRun> {
    let mut result = Vec::new();
    let mut offset = 0;
    for run in runs {
        let end = offset + run.text.encode_utf16().count();
        let start = offset.max(range.start);
        let stop = end.min(range.end);
        if start < stop {
            result.push(HudStyledTextRun {
                text: String::from_utf16_lossy(&value[start..stop]),
                ..run.clone()
            });
        }
        offset = end;
        if offset >= range.end {
            break;
        }
    }
    if offset < range.end {
        result.push(HudStyledTextRun::plain(String::from_utf16_lossy(
            &value[offset.max(range.start)..range.end],
        )));
    }
    result
}

fn input_layout(
    view: &HudChatInputView,
    runs: &[HudStyledTextRun],
    glyphs: &HudFontGlyphMap,
    width: u32,
) -> EditBoxLayout {
    edit_box_layout(view, runs, glyphs, width, 256, width as f32)
}

pub(super) fn edit_box_layout(
    view: &HudChatInputView,
    runs: &[HudStyledTextRun],
    glyphs: &HudFontGlyphMap,
    width: u32,
    max_length: usize,
    highlight_right: f32,
) -> EditBoxLayout {
    let start = view.display_start.min(view.value.len());
    let len = glyphs
        .plain_utf16_head_len(&view.value[start..], width.min(i32::MAX as u32) as i32)
        .unwrap_or_default();
    let end = start + len;
    let cursor = view.cursor.min(view.value.len());
    let selection = view.selection.min(view.value.len()).clamp(start, end);
    let cursor_on_screen = (start..=end).contains(&cursor);
    let split = if cursor_on_screen { cursor } else { end };
    // EditBox decomposes each cursor half independently, including split pairs.
    let head = sliced_runs(&view.value, runs, start..split);
    let mut tail_x = if len == 0 {
        0.0
    } else {
        hud_logical_font_runs_width(&head, glyphs).unwrap_or_default() as f32 + 1.0
    };
    let insert = cursor < view.value.len() || view.value.len() >= max_length;
    let cursor_x = if !cursor_on_screen {
        if cursor > start {
            width as f32
        } else {
            0.0
        }
    } else if insert {
        tail_x -= 1.0;
        tail_x
    } else {
        tail_x
    };
    let tail = if cursor_on_screen {
        sliced_runs(&view.value, runs, split..end)
    } else {
        Vec::new()
    };
    let highlight = (selection != cursor).then(|| {
        let x = hud_formatted_utf16_width(&view.value[start..selection], glyphs) as f32 - 1.0;
        let a = cursor_x.min(highlight_right);
        let b = x.min(highlight_right);
        (a.min(b), a.max(b))
    });
    EditBoxLayout {
        head,
        tail,
        tail_x,
        cursor_x,
        cursor_on_screen,
        insert,
        selection: highlight,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_chat_input<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    seed: u64,
    surface_size: PhysicalSize<u32>,
    view: &HudChatInputView,
    runs: &[HudStyledTextRun],
    cursor_visible: bool,
    suffix: Option<&str>,
    origin: (f32, f32),
    width: u32,
) {
    let layout = input_layout(view, runs, glyphs, width);
    for (runs, x) in [(&layout.head, 0.0), (&layout.tail, layout.tail_x)] {
        let geometry = hud_logical_styled_text_geometry(
            runs,
            glyphs,
            obfuscated_pool,
            seed,
            (origin.0 + x, origin.1),
            true,
            HUD_COMMAND_INPUT_TINT,
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
    if let Some(suffix) = suffix.filter(|_| !layout.insert) {
        // `EditBox` draws `this.suggestion` through
        // `graphics.text(font, String, …)`, which resolves `FormattedText.of`.
        push_hud_coded_string_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            seed,
            surface_size,
            &hud_coded_string_runs(suffix),
            (origin.0 + layout.cursor_x - 1.0, origin.1),
            HUD_COMMAND_SUFFIX_TINT,
            1.0,
            true,
        );
    }
    if let Some((left, right)) = layout.selection {
        let rect = absolute_hud_rect(origin.0 + left, origin.1 - 1.0, (right - left) as u32, 11);
        push_hud_text_highlight(vertices, commands, surface_size, rect);
    }
    if cursor_visible && layout.cursor_on_screen {
        if layout.insert {
            push_hud_fill(
                vertices,
                commands,
                surface_size,
                absolute_hud_rect(origin.0 + layout.cursor_x, origin.1 - 1.0, 1, 11),
                HUD_COMMAND_INPUT_TINT,
            );
        } else {
            push_hud_raw_string_text(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                seed,
                surface_size,
                "_",
                (origin.0 + layout.cursor_x, origin.1),
                HUD_COMMAND_INPUT_TINT,
                1.0,
                true,
            );
        }
    }
}

#[cfg(test)]
#[path = "tests/chat_input.rs"]
mod tests;
