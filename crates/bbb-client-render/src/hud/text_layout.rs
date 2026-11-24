use super::*;
use crate::text_mesh::{text_width, TextPen};

mod utf16_widget;
pub(super) use utf16_widget::*;

mod chat_geometry;
pub(super) use chat_geometry::{
    hud_chat_queue_prompt, hud_chat_restricted_prompt, HudChatGeometry, HudChatPrompt,
};
pub use chat_geometry::{HudChatAppendedLines, HudChatOptions, HudChatScroll, HudChatScrollPage};
mod formatted;
pub(super) use formatted::hud_wrap_formatted_component_runs_with_interactions;
pub use formatted::{
    hud_formatted_utf16_line_ranges, hud_formatted_utf16_width,
    hud_resolve_formatted_component_text, hud_resolve_formatted_utf16_text,
    hud_resolve_formatted_utf16_text_with_style, hud_wrap_formatted_component_text,
};

/// Test projection of [`hud_wrap_chat_runs`] over the raw runs: the line-range
/// engine is asserted through plain strings, which no draw site feeds directly.
#[cfg(test)]
pub(super) fn hud_wrap_plain_text(
    text: &str,
    max_width: u32,
    glyphs: &HudFontGlyphMap,
) -> Vec<String> {
    let mut lines = hud_wrap_chat_runs(&hud_raw_string_runs(text), max_width, glyphs)
        .into_iter()
        .map(|line| line.into_iter().map(|run| run.text).collect::<String>())
        .collect::<Vec<_>>();
    if text.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Wraps styled text with the renderer's exact bold-aware font advances while
/// preserving the style and colour of each resulting line.
pub fn hud_wrap_styled_text(
    runs: &[HudStyledTextRun],
    width: u32,
    glyphs: &HudFontGlyphMap,
) -> Vec<Vec<HudStyledTextRun>> {
    hud_wrap_chat_runs(runs, width, glyphs)
}

pub fn hud_wrap_component_text(
    runs: &[HudComponentTextRun],
    width: u32,
    fonts: &HudFontGlyphBook,
) -> Vec<Vec<HudComponentTextRun>> {
    hud_wrap_component_runs_with_interactions(runs, &[], width, fonts)
        .into_iter()
        .map(|line| line.runs)
        .collect()
}

pub fn hud_formatted_text_width(text: &str, glyphs: &HudFontGlyphMap) -> i32 {
    hud_formatted_utf16_width(&text.encode_utf16().collect::<Vec<_>>(), glyphs)
}

pub fn hud_logical_component_text_width_f32(
    runs: &[HudComponentTextRun],
    fonts: &HudFontGlyphBook,
) -> f32 {
    let mut width = 0.0_f32;
    for run in runs {
        for ch in run.visual.text.chars() {
            width += hud_font_glyph(ch, fonts.glyphs(run.font.as_deref()))
                .styled_advance_f32(run.visual.style);
        }
    }
    width
}

/// Truncates one resolved run list at the first character that overflows
/// `max_width`, reading each run's own advance. Every caller measures, clips and
/// draws the same list, so the break point cannot disagree with the glyphs.
pub(super) fn hud_clip_styled_text_runs(
    runs: &[HudStyledTextRun],
    max_width: u32,
    glyphs: &HudFontGlyphMap,
) -> Vec<HudStyledTextRun> {
    let max_width = max_width as f32;
    let mut width = 0.0_f32;
    let mut clipped = Vec::new();
    for run in runs {
        let mut kept = String::new();
        let mut overflowed = false;
        for character in run.text.chars() {
            width += hud_font_glyph(character, glyphs).styled_advance_f32(run.style);
            if width > max_width {
                overflowed = true;
                break;
            }
            kept.push(character);
        }
        if !kept.is_empty() {
            clipped.push(HudStyledTextRun {
                text: kept,
                ..run.clone()
            });
        }
        if overflowed {
            break;
        }
    }
    clipped
}

pub(super) fn hud_multiline_lines(
    text: &str,
    width: u32,
    glyphs: &HudFontGlyphMap,
) -> Vec<HudMultilineLine> {
    hud_plain_text_line_ranges(text, width, glyphs)
        .into_iter()
        .map(|range| HudMultilineLine {
            begin: range.start,
            end: range.end,
        })
        .collect()
}

/// Scalar ranges for plain text editors, including empty input and a trailing newline.
pub fn hud_plain_text_line_ranges(
    text: &str,
    width: u32,
    glyphs: &HudFontGlyphMap,
) -> Vec<std::ops::Range<usize>> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut lines = hud_text_line_ranges(&chars, width, |ch| {
        (
            *ch,
            glyphs.styled_char_advance(*ch, HudTextStyle::default()),
        )
    });
    if chars.is_empty() {
        lines.push(0..0);
    }
    lines
}

pub(super) fn hud_scrolling_text_layout(
    bounds: HudInventoryScrollingText,
    line_width: i32,
    time_seconds: f64,
) -> HudScrollingTextLayout {
    let bounds_width = bounds.width.max(1);
    let text_y = bounds.y.saturating_add(
        (i32::try_from(bounds.height).unwrap_or(i32::MAX) - HUD_WIDGET_TEXT_LINE_HEIGHT) / 2 + 1,
    );
    if line_width <= bounds_width as i32 {
        let center_x = bounds
            .x
            .saturating_add(i32::try_from(bounds_width).unwrap_or(i32::MAX) / 2);
        return HudScrollingTextLayout {
            x: center_x.saturating_sub(line_width / 2),
            y: text_y,
            clipped: false,
        };
    }

    super::mark_hud_frame_varying_used();
    let max_position = line_width - bounds_width as i32;
    let period = (f64::from(max_position) * 0.5).max(3.0);
    let time_seconds = if time_seconds.is_finite() {
        time_seconds
    } else {
        0.0
    };
    let alpha = ((std::f64::consts::FRAC_PI_2
        * (std::f64::consts::TAU * time_seconds / period).cos())
    .sin()
        / 2.0)
        + 0.5;
    let position = (alpha * f64::from(max_position)) as i32;
    HudScrollingTextLayout {
        x: bounds.x.saturating_sub(position),
        y: text_y,
        clipped: true,
    }
}

pub(super) fn hud_text_input_display_start(
    text: &str,
    scroll_to: usize,
    width: i32,
    glyphs: &HudFontGlyphMap,
) -> usize {
    glyphs
        .plain_text_display_start_for_width(text, scroll_to, width)
        .unwrap_or(0)
}

pub(super) fn hud_plain_head_char_len_by_width(
    text: &str,
    start: usize,
    max_chars: usize,
    width: i32,
    glyphs: &HudFontGlyphMap,
) -> usize {
    let mut remaining = width as f32;
    let mut len = 0;
    for ch in text.chars().skip(start).take(max_chars) {
        remaining -= hud_font_glyph(ch, glyphs).styled_advance_f32(Default::default());
        if remaining < 0.0 {
            break;
        }
        len += 1;
    }
    len
}

pub(super) fn hud_slice_by_chars(text: &str, start: usize, end: usize) -> String {
    text.chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

/// Vanilla `FormattedCharSequence.forward(text, Style.EMPTY)`
/// and `StringSplitter.plainIndexAtWidth`
/// the string *is* the glyph sequence, so a `§`
/// keeps its own glyph and advance. Nothing on this face reads a code, and
/// [`hud_raw_string_width`] is this list's logical-order width.
pub(super) fn hud_raw_string_runs(text: &str) -> [HudStyledTextRun; 1] {
    [HudStyledTextRun::plain(text)]
}

pub(super) fn hud_raw_string_width(text: &str, glyphs: &HudFontGlyphMap) -> i32 {
    hud_logical_font_runs_width(&hud_raw_string_runs(text), glyphs).unwrap_or_default()
}

/// Vanilla `Font.width(String)` over the runs a `FormattedText.of(str)` pass
/// resolved: the same logical-order sum as
/// [`hud_formatted_text_width`], read off the caller's run list so a
/// measurement and a draw cannot disagree about which codes were consumed.
pub fn hud_coded_string_width(runs: &[HudStyledTextRun], glyphs: &HudFontGlyphMap) -> i32 {
    hud_logical_font_runs_width(runs, glyphs).unwrap_or_default()
}

/// `FormattedText.of(str)` for a `String` the caller only has as text. The run
/// list and [`hud_coded_string_width`] share this one resolution pass, so a
/// caller that binds the result uses it for both.
pub fn hud_coded_string_runs(text: &str) -> Vec<HudStyledTextRun> {
    hud_resolve_formatted_utf16_text(&text.encode_utf16().collect::<Vec<_>>())
}

/// `Font.width(String)` for a caller that measures a string it does not bind:
/// `StringSplitter.stringWidth(String)` runs its own `iterateFormatted` pass
/// so this and the draw cannot disagree about
/// which codes were consumed even though each resolves once.
pub(super) fn hud_coded_string_text_width(text: &str, glyphs: &HudFontGlyphMap) -> i32 {
    hud_coded_string_width(&hud_coded_string_runs(text), glyphs)
}

/// `StringDecomposer.iterateFormatted(text, style)` starts from the caller's
/// style and returns to it on `§r`, which is what `Component.withStyle` hands a
/// translated line: the base is not `Style.EMPTY`, so a reset inside the
/// translation restores the base rather than clearing it.
pub(super) fn hud_coded_string_runs_with_style(
    text: &str,
    style: HudTextStyle,
) -> Vec<HudStyledTextRun> {
    hud_resolve_formatted_utf16_text_with_style(&text.encode_utf16().collect::<Vec<_>>(), style)
}

pub(super) fn hud_command_input_runs(state: &HudCommandSuggestions) -> Vec<HudStyledTextRun> {
    if state.draft {
        // ChatScreen installs its draft formatter before
        // CommandSuggestions adds the syntax one, and
        // EditBox uses the first formatter that answers.
        return vec![HudStyledTextRun {
            shadow_color: None,
            text: state.input_text.clone(),
            style: HudTextStyle {
                italic: true,
                ..HudTextStyle::default()
            },
            color: Some(HUD_COMMAND_LITERAL_COLOR),
        }];
    }
    let (Some(syntax), Some(view)) = (state.syntax.as_ref(), state.input_view.as_ref()) else {
        // No parse means no formatter answered: the edit box draws its own
        // text colour.
        return Vec::new();
    };
    hud_command_syntax_runs(&view.value, syntax)
}

/// `CommandSuggestions.formatText` over the whole raw value. Vanilla calls the
/// formatter once per displayed half with that half's absolute offset
/// (431), so ranges resolved against the value slice to the
/// same halves afterwards.
fn hud_command_syntax_runs(value: &[u16], syntax: &HudCommandInputSyntax) -> Vec<HudStyledTextRun> {
    let len = value.len();
    let mut runs = Vec::new();
    let mut unformatted_start = 0usize;
    let mut next_color = -1i32;
    for argument in &syntax.arguments {
        next_color += 1;
        if next_color >= HUD_COMMAND_ARGUMENT_COLORS.len() as i32 {
            next_color = 0;
        }
        // Vanilla only ever clamps the range ends. A non-increasing pair would
        // address text twice, so such a range keeps its grey gap instead.
        if argument.start < unformatted_start {
            continue;
        }
        if argument.start >= len {
            break;
        }
        let end = argument.end.min(len);
        if end <= argument.start {
            continue;
        }
        push_command_syntax_run(
            &mut runs,
            value,
            unformatted_start..argument.start,
            HUD_COMMAND_LITERAL_COLOR,
        );
        push_command_syntax_run(
            &mut runs,
            value,
            argument.start..end,
            HUD_COMMAND_ARGUMENT_COLORS[next_color as usize],
        );
        unformatted_start = end;
    }
    if let Some(unread) = syntax
        .unread
        .filter(|unread| unread.start >= unformatted_start && unread.start < len)
    {
        let end = unread.end.min(len).max(unread.start);
        push_command_syntax_run(
            &mut runs,
            value,
            unformatted_start..unread.start,
            HUD_COMMAND_LITERAL_COLOR,
        );
        push_command_syntax_run(
            &mut runs,
            value,
            unread.start..end,
            HUD_COMMAND_UNPARSED_COLOR,
        );
        unformatted_start = end;
    }
    push_command_syntax_run(
        &mut runs,
        value,
        unformatted_start..len,
        HUD_COMMAND_LITERAL_COLOR,
    );
    runs
}

fn push_command_syntax_run(
    runs: &mut Vec<HudStyledTextRun>,
    value: &[u16],
    range: std::ops::Range<usize>,
    color: u32,
) {
    if range.start >= range.end {
        return;
    }
    runs.push(HudStyledTextRun {
        shadow_color: None,
        text: String::from_utf16_lossy(&value[range]),
        style: HudTextStyle::default(),
        color: Some(color),
    });
}

pub(super) fn hud_prefix_by_chars(text: &str, char_count: usize) -> String {
    text.chars().take(char_count).collect()
}

pub(super) fn hud_chat_line_alpha(context: &HudChatContext, line: &HudChatLine) -> f32 {
    if context.focused {
        return 1.0;
    }
    let age = context.gui_ticks.saturating_sub(line.added_tick) as f64;
    let t = ((1.0 - age / 200.0) * 10.0).clamp(0.0, 1.0);
    (t * t) as f32
}

pub(super) fn hud_text_backdrop_alpha(opacity: f32, text_alpha: f32) -> f32 {
    let background_alpha = hud_argb_alpha_channel(opacity);
    let text_alpha = hud_argb_alpha_channel(text_alpha);
    (background_alpha * text_alpha / 255) as f32 / 255.0
}

pub(super) fn hud_text_backdrop_rect(
    origin: (f32, f32),
    text_width: i32,
    scale: f32,
) -> Option<HudRect> {
    (scale.is_finite() && scale > 0.0).then(|| {
        let left = origin.0 - 2.0 * scale;
        let right = origin.0 + (text_width as f32 + 2.0) * scale;
        HudRect {
            x: left.min(right),
            y: origin.1 - 2.0 * scale,
            width: (right - left).abs().round() as u32,
            height: (13.0 * scale).round() as u32,
        }
    })
}

/// Resolved main-pass colour of a styled run: the run's `Style` colour over
/// the line's base tint (vanilla `StringRenderOutput.getTextColor`), keeping
/// the base alpha.
pub(in crate::hud) fn hud_run_color_tint(run: &HudStyledTextRun, base_tint: [f32; 4]) -> [f32; 4] {
    match run.color {
        Some(rgb) => [
            ((rgb >> 16) & 0xFF) as f32 / 255.0,
            ((rgb >> 8) & 0xFF) as f32 / 255.0,
            (rgb & 0xFF) as f32 / 255.0,
            base_tint[3],
        ],
        None => base_tint,
    }
}

fn hud_text_argb(tint: [f32; 4]) -> u32 {
    let [r, g, b, a] = tint.map(hud_argb_alpha_channel);
    (a << 24) | (r << 16) | (g << 8) | b
}

fn hud_text_tint(argb: u32) -> [f32; 4] {
    [16, 8, 0, 24].map(|shift| ((argb >> shift) & 255) as f32 / 255.0)
}

/// The draw's cached line geometry, carrying the vanilla click areas and
/// prepared bounds so a pointer hit test never lays the line out again.
pub(super) fn hud_chat_line_geometry(
    runs: &[HudComponentTextRun],
    targets: &[Option<usize>],
    fonts: &HudFontGlyphBook,
) -> Option<HudChatLineGeometry> {
    if runs
        .iter()
        .any(|run| run.visual.style.obfuscated || run.visual.shadow_color.is_some())
    {
        return None;
    }
    Some(HudChatLineGeometry {
        geometry: hud_component_text_hit_geometry(runs, targets, fonts, true),
    })
}

/// Negative advances can move later glyphs back inside `width_limit`, so
/// clipping a glyph must not stop the pen walk.
#[allow(clippy::too_many_arguments)]
pub(super) fn hud_styled_text_geometry(
    runs: &[HudStyledTextRun],
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    origin: (f32, f32),
    draw_shadow: bool,
    base_tint: [f32; 4],
    width_limit: Option<u32>,
    scale: f32,
) -> HudStyledTextGeometry {
    hud_resolved_text_geometry(
        runs.iter().map(|visual| HudResolvedTextRun {
            visual,
            glyphs,
            obfuscated_pool,
            font: None,
        }),
        None,
        true,
        glyphs.default_right_to_left(),
        obfuscated_seed,
        origin,
        draw_shadow,
        base_tint,
        width_limit,
        scale,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn hud_logical_styled_text_geometry(
    runs: &[HudStyledTextRun],
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    origin: (f32, f32),
    draw_shadow: bool,
    base_tint: [f32; 4],
    width_limit: Option<u32>,
    scale: f32,
) -> HudStyledTextGeometry {
    // Vanilla's one-line EditBox explicitly supplies
    // `FormattedCharSequence.forward`, so its visible slice must stay aligned
    // with logical cursor and selection offsets even in an RTL language.
    hud_resolved_text_geometry(
        runs.iter().map(|visual| HudResolvedTextRun {
            visual,
            glyphs,
            obfuscated_pool,
            font: None,
        }),
        None,
        false,
        glyphs.default_right_to_left(),
        obfuscated_seed,
        origin,
        draw_shadow,
        base_tint,
        width_limit,
        scale,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn hud_component_text_geometry(
    runs: &[HudComponentTextRun],
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
    origin: (f32, f32),
    draw_shadow: bool,
    base_tint: [f32; 4],
    width_limit: Option<u32>,
    scale: f32,
) -> HudStyledTextGeometry {
    hud_resolved_text_geometry(
        runs.iter().map(|run| HudResolvedTextRun {
            visual: &run.visual,
            glyphs: fonts.glyphs(run.font.as_deref()),
            obfuscated_pool: fonts.obfuscated_pool(run.font.as_deref()),
            font: run.font.as_deref(),
        }),
        None,
        true,
        fonts.default_right_to_left(),
        obfuscated_seed,
        origin,
        draw_shadow,
        base_tint,
        width_limit,
        scale,
    )
}

/// The click/hover geometry of one already-wrapped line in font space: the same
/// pen walk and the same advances the draw submits, plus vanilla's per-glyph
/// `ActiveArea` boxes and `PreparedText.bounds()`. `targets` holds one
/// interaction target per logical character of `runs` and is what tells a glyph
/// area which chat/book interaction it belongs to.
pub(super) fn hud_component_text_hit_geometry(
    runs: &[HudComponentTextRun],
    targets: &[Option<usize>],
    fonts: &HudFontGlyphBook,
    draw_shadow: bool,
) -> HudStyledTextGeometry {
    hud_resolved_text_geometry(
        runs.iter().map(|run| HudResolvedTextRun {
            visual: &run.visual,
            glyphs: fonts.glyphs(run.font.as_deref()),
            obfuscated_pool: fonts.obfuscated_pool(run.font.as_deref()),
            font: run.font.as_deref(),
        }),
        Some(targets),
        true,
        fonts.default_right_to_left(),
        0,
        (0.0, 0.0),
        draw_shadow,
        HUD_TINT_WHITE,
        None,
        1.0,
    )
}

#[allow(clippy::too_many_arguments)]
fn hud_resolved_text_geometry<'a>(
    runs: impl IntoIterator<Item = HudResolvedTextRun<'a>>,
    targets: Option<&[Option<usize>]>,
    apply_visual_order: bool,
    default_right_to_left: bool,
    obfuscated_seed: u64,
    origin: (f32, f32),
    draw_shadow: bool,
    base_tint: [f32; 4],
    width_limit: Option<u32>,
    scale: f32,
) -> HudStyledTextGeometry {
    let mut geometry = HudStyledTextGeometry {
        glyph_quads: Vec::new(),
        effect_rects: Vec::new(),
        text_bounds: None,
        glyph_areas: Vec::new(),
        empty_areas: Vec::new(),
    };
    let collect_areas = targets.is_some();
    // `Font.PreparedTextBuilder.markSize`: drawn glyph extents and
    // underline/strikethrough effects, but never an `EmptyArea`.
    let mut bounds: Option<[f32; 4]> = None;
    let mut mark_bounds = |left: f32, top: f32, right: f32, bottom: f32| {
        bounds = Some(match bounds {
            Some([l, t, r, b]) => [l.min(left), t.min(top), r.max(right), b.max(bottom)],
            None => [left, top, right, bottom],
        });
    };
    // A glyph and its shadow share one obfuscated substitute.
    let mut pen = TextPen::new(0.0, 0.0, obfuscated_seed);
    // The interaction target rides with the character so bidi reordering and
    // Arabic shaping keep it attached to the glyph it came from.
    let mut logical = Vec::<(char, HudSubsequence<(HudResolvedTextRun, Option<usize>)>)>::new();
    let mut char_index = 0usize;
    for resolved in runs {
        for ch in resolved.visual.text.chars() {
            let target = targets.and_then(|targets| targets.get(char_index).copied().flatten());
            logical.push((
                ch,
                HudSubsequence {
                    payload: (resolved, target),
                    key: HudSubsequenceKey {
                        style: resolved.visual.style,
                        color: resolved.visual.color,
                        shadow_color: resolved.visual.shadow_color,
                        font: resolved.font,
                        target,
                    },
                },
            ));
            char_index = char_index.saturating_add(1);
        }
    }
    // `FormattedCharSequence.forward` over the whole visible slice is one part, so the
    // uninterrupted editor line indexes by its own UTF-16 offsets.
    let ordered = if apply_visual_order {
        visual_order_indexed(&logical, default_right_to_left)
    } else {
        let mut offset = 0usize;
        logical
            .into_iter()
            .map(|(ch, subsequence)| {
                let index = offset;
                offset += ch.len_utf16();
                (ch, subsequence, index)
            })
            .collect()
    };
    for (ch, subsequence, local_index) in ordered {
        let (resolved, target) = subsequence.payload;
        let run = resolved.visual;
        let pen_x = pen.x;
        if !collect_areas && run.style.obfuscated && ch != ' ' {
            super::mark_hud_frame_varying_used();
        }
        let glyph = pen.place(
            ch,
            resolved.glyphs,
            resolved.obfuscated_pool,
            run.style,
            hud_text_argb(hud_run_color_tint(run, base_tint)),
            run.shadow_color,
            draw_shadow,
            local_index == 0,
        );
        // Glyph geometry is produced in font pixels relative to `origin`
        // and mapped through `origin + scale * offset`, exactly a vanilla
        // pose scale (the shadow offset scales with the pose too).
        // Bitmap cells include transparent padding beyond the font advance.
        if width_limit.is_none_or(|limit| pen_x + glyph.advance() <= limit as f32) {
            for (quad, _, argb) in glyph.glyph_quads() {
                let quad = HudGlyphQuad {
                    corners: quad
                        .corners
                        .map(|[cx, cy]| [origin.0 + cx * scale, origin.1 + cy * scale]),
                    ..quad
                };
                geometry.glyph_quads.push((quad, hud_text_tint(argb)));
            }
        }
        if collect_areas {
            let area = glyph
                .glyph
                .styled_glyph_area(pen_x, 0.0, run.style, glyph.shadow_argb != 0);
            if area.empty {
                geometry.empty_areas.push((area, target));
            } else {
                mark_bounds(area.left, area.top, area.bounds_right, area.bottom);
                geometry.glyph_areas.push((area, target));
            }
        }
        if width_limit.is_none_or(|limit| pen_x + glyph.advance() <= limit as f32) {
            for (rect, _, argb) in glyph.effect_rects() {
                let rect = HudEffectRect {
                    x0: origin.0 + rect.x0 * scale,
                    y0: origin.1 + rect.y0 * scale,
                    x1: origin.0 + rect.x1 * scale,
                    y1: origin.1 + rect.y1 * scale,
                    ..rect
                };
                if collect_areas {
                    mark_bounds(rect.x0, rect.y0, rect.x1, rect.y1);
                }
                geometry.effect_rects.push((rect, hud_text_tint(argb)));
            }
        }
    }
    geometry.text_bounds = bounds.filter(|[left, top, right, bottom]| left < right && top < bottom);
    geometry
}

pub(super) fn hud_item_count_glyphs<'a>(
    text: &'a str,
    glyphs: &'a [HudDigitGlyph; 10],
) -> impl Iterator<Item = (f32, HudDigitGlyph)> + 'a {
    let mut pen_x = 0.0;
    text.bytes().filter_map(move |digit| {
        let glyph = glyphs[(digit - b'0') as usize];
        let x = pen_x;
        pen_x += glyph.advance;
        (!glyph.is_empty).then_some((x, glyph))
    })
}

pub(super) fn hud_digit_text_width(text: &str, glyphs: &[HudDigitGlyph; 10]) -> Option<i32> {
    if text.is_empty() || !text.bytes().all(|digit| digit.is_ascii_digit()) {
        return None;
    }
    Some(text_width(
        text.bytes()
            .map(|digit| glyphs[(digit - b'0') as usize].advance),
    ))
}

/// Test-only single-style wrappers over [`hud_font_runs_width`], the one
/// width implementation.
#[cfg(test)]
pub(super) fn hud_font_text_width(text: &str, glyphs: &HudFontGlyphMap) -> Option<i32> {
    hud_font_text_width_styled(text, glyphs, HudTextStyle::default())
}

#[cfg(test)]
pub(super) fn hud_font_text_width_styled(
    text: &str,
    glyphs: &HudFontGlyphMap,
    style: HudTextStyle,
) -> Option<i32> {
    hud_font_runs_width(
        &[HudStyledTextRun {
            shadow_color: None,
            text: text.to_string(),
            style,
            color: None,
        }],
        glyphs,
    )
}

/// Vanilla `Font.plainSubstrByWidth` cursor equivalent for a plain search
/// field: include glyphs while the cumulative unstyled advance fits inside
/// the caller's pixel budget.
pub(super) fn hud_plain_text_cursor_for_width(
    text: &str,
    width: i32,
    glyphs: &HudFontGlyphMap,
) -> Option<usize> {
    hud_plain_text_cursor_for_width_from(text, 0, width, glyphs)
}

pub(super) fn hud_plain_text_cursor_for_width_from(
    text: &str,
    display_start: usize,
    width: i32,
    glyphs: &HudFontGlyphMap,
) -> Option<usize> {
    if !glyphs.is_available() {
        return None;
    }
    let text_len = text.chars().count();
    let display_start = display_start.min(text_len);
    let display_len = hud_plain_head_char_len_by_width(
        text,
        display_start,
        text_len.saturating_sub(display_start),
        width,
        glyphs,
    );
    Some(display_start.saturating_add(display_len))
}

pub(super) fn hud_plain_text_display_start_for_width(
    text: &str,
    scroll_to: usize,
    width: i32,
    glyphs: &HudFontGlyphMap,
) -> Option<usize> {
    glyphs
        .is_available()
        .then(|| hud_text_input_display_start(text, scroll_to, width, glyphs))
}

/// Measures a completed formatted line after vanilla language visual ordering.
/// Each visual glyph contributes `GlyphInfo.getAdvance(style.isBold())`; this
/// includes Arabic shaping compaction such as Lam-Alef.
pub(super) fn hud_font_runs_width(
    runs: &[HudStyledTextRun],
    glyphs: &HudFontGlyphMap,
) -> Option<i32> {
    let logical = runs
        .iter()
        .flat_map(|run| run.text.chars().map(move |ch| (ch, run)))
        .collect::<Vec<_>>();
    let width = text_width(
        visual_order(&logical, glyphs.default_right_to_left())
            .into_iter()
            .map(|(ch, run)| hud_font_glyph(ch, glyphs).styled_advance_f32(run.style)),
    );
    glyphs.is_available().then_some(width)
}

/// Measures the unshaped logical buffer used by wrapping, editing cursors, and
/// selections. Vanilla `Font.width(String)` does not apply language ordering.
pub(super) fn hud_logical_font_runs_width(
    runs: &[HudStyledTextRun],
    glyphs: &HudFontGlyphMap,
) -> Option<i32> {
    let width = text_width(runs.iter().flat_map(|run| {
        run.text
            .chars()
            .map(|ch| hud_font_glyph(ch, glyphs).styled_advance_f32(run.style))
    }));
    glyphs.is_available().then_some(width)
}

pub(super) fn hud_component_runs_width(
    runs: &[HudComponentTextRun],
    fonts: &HudFontGlyphBook,
) -> Option<i32> {
    fonts
        .glyphs(None)
        .is_available()
        .then(|| hud_component_text_width(runs, fonts))
}

pub fn hud_component_text_width(runs: &[HudComponentTextRun], fonts: &HudFontGlyphBook) -> i32 {
    let logical = runs
        .iter()
        .flat_map(|run| run.visual.text.chars().map(move |ch| (ch, run)))
        .collect::<Vec<_>>();
    text_width(
        visual_order(&logical, fonts.default_right_to_left())
            .into_iter()
            .map(|(ch, run)| {
                hud_font_glyph(ch, fonts.glyphs(run.font.as_deref()))
                    .styled_advance_f32(run.visual.style)
            }),
    )
}

#[cfg(test)]
pub(super) fn hud_wrap_chat_messages(
    messages: &[HudChatMessage],
    fonts: &HudFontGlyphBook,
    wrap_width: i32,
) -> Vec<HudChatLine> {
    hud_wrap_chat_message_lines(messages, fonts, wrap_width).0
}

/// The trimmed wrapped lines and each message's line count before the trim.
pub(super) fn hud_wrap_chat_message_lines(
    messages: &[HudChatMessage],
    fonts: &HudFontGlyphBook,
    wrap_width: i32,
) -> (Vec<HudChatLine>, Vec<usize>) {
    let mut lines = Vec::with_capacity(messages.len().min(HUD_CHAT_MAX_WRAPPED_LINES));
    let mut message_lines = Vec::with_capacity(messages.len());
    for message in messages {
        let width = if message
            .tag
            .as_ref()
            .is_some_and(|tag| tag.kind == HudChatTagKind::Modified)
        {
            wrap_width.wrapping_sub(HUD_CHAT_MODIFIED_ICON_WIDTH_WITH_GAP as i32)
        } else {
            wrap_width
        };
        let wrapped = hud_wrap_formatted_component_runs_with_interactions(
            &message.runs,
            &message.interactions,
            u32::try_from(width).unwrap_or(0).max(1),
            fonts,
        );
        let wrapped_len = wrapped.len();
        message_lines.push(wrapped_len);
        for (index, wrapped) in wrapped.into_iter().enumerate() {
            lines.push(HudChatLine {
                display_sequence: message.display_sequence,
                added_tick: message.added_tick,
                cached_geometry: hud_chat_line_geometry(&wrapped.runs, &wrapped.targets, fonts),
                runs: wrapped.runs,
                tag: message.tag.clone(),
                targets: wrapped.targets,
                interactions: wrapped.interactions,
                end_of_entry: index + 1 == wrapped_len,
            });
        }
    }
    if lines.len() > HUD_CHAT_MAX_WRAPPED_LINES {
        let excess = lines.len() - HUD_CHAT_MAX_WRAPPED_LINES;
        lines.drain(..excess);
    }
    (lines, message_lines)
}

pub(super) fn hud_wrap_chat_runs(
    runs: &[HudStyledTextRun],
    max_width: u32,
    glyphs: &HudFontGlyphMap,
) -> Vec<Vec<HudStyledTextRun>> {
    hud_wrap_chat_runs_with_interactions(runs, &[], max_width, glyphs)
        .into_iter()
        .map(|line| line.runs)
        .collect()
}

pub(super) fn hud_wrap_chat_runs_with_interactions(
    runs: &[HudStyledTextRun],
    interactions: &[HudChatInteraction],
    max_width: u32,
    glyphs: &HudFontGlyphMap,
) -> Vec<HudWrappedChatLine> {
    let runs = runs
        .iter()
        .cloned()
        .map(|visual| HudComponentTextRun { visual, font: None })
        .collect::<Vec<_>>();
    hud_wrap_component_runs(
        &runs,
        interactions,
        max_width,
        HudChatFontResolver::Default(glyphs),
    )
    .into_iter()
    .map(|line| HudWrappedChatLine {
        runs: line.runs.into_iter().map(|run| run.visual).collect(),
        targets: line.targets,
        interactions: line.interactions,
    })
    .collect()
}

pub(super) fn hud_wrap_component_runs_with_interactions(
    runs: &[HudComponentTextRun],
    interactions: &[HudChatInteraction],
    max_width: u32,
    fonts: &HudFontGlyphBook,
) -> Vec<HudWrappedComponentChatLine> {
    hud_wrap_component_runs(
        runs,
        interactions,
        max_width,
        HudChatFontResolver::Book(fonts),
    )
}

fn hud_wrap_component_runs(
    runs: &[HudComponentTextRun],
    interactions: &[HudChatInteraction],
    max_width: u32,
    fonts: HudChatFontResolver<'_>,
) -> Vec<HudWrappedComponentChatLine> {
    let mut char_index = 0usize;
    let chars = runs
        .iter()
        .flat_map(|run| {
            run.visual.text.chars().map(move |ch| {
                (
                    ch,
                    run.visual.style,
                    run.visual.color,
                    run.visual.shadow_color,
                    run.font.clone(),
                )
            })
        })
        .map(|(ch, style, color, shadow_color, font)| {
            let target_index = interactions
                .iter()
                .find(|interaction| {
                    interaction.start_char <= char_index && char_index < interaction.end_char
                })
                .map(|interaction| interaction.target_index);
            char_index = char_index.saturating_add(1);
            HudChatStyledChar {
                ch,
                style,
                color,
                shadow_color,
                font,
                target_index,
            }
        })
        .collect::<Vec<_>>();
    hud_text_line_ranges(&chars, max_width, |styled| {
        (
            styled.ch,
            hud_font_glyph(styled.ch, fonts.glyphs(styled.font.as_deref()))
                .styled_advance_f32(styled.style),
        )
    })
    .into_iter()
    .map(|range| hud_chat_chars_to_component_line(&chars[range], interactions))
    .collect()
}

// Vanilla StringSplitter: consume only the break delimiter.
fn hud_text_line_ranges<T>(
    chars: &[T],
    max_width: u32,
    glyph: impl Fn(&T) -> (char, f32),
) -> Vec<std::ops::Range<usize>> {
    let max_width = max_width.max(1) as f32;
    let mut lines = Vec::new();
    let mut begin = 0;
    let mut trailing_newline = false;
    while begin < chars.len() {
        let mut width = 0.0;
        let mut had_nonzero = false;
        let mut last_space = None;
        let mut end = chars.len();
        for (index, value) in chars.iter().enumerate().skip(begin) {
            let (ch, advance) = glyph(value);
            if ch == '\n' {
                end = index;
                break;
            }
            if ch == ' ' {
                last_space = Some(index);
            }
            width += advance;
            if had_nonzero && width > max_width {
                end = last_space.unwrap_or(index);
                break;
            }
            had_nonzero |= advance != 0.0;
        }
        lines.push(begin..end);
        trailing_newline = end < chars.len() && glyph(&chars[end]).0 == '\n';
        let skip = end < chars.len() && matches!(glyph(&chars[end]).0, ' ' | '\n');
        begin = end + usize::from(skip);
    }
    if trailing_newline {
        lines.push(chars.len()..chars.len());
    }
    lines
}

fn hud_chat_chars_to_component_line(
    chars: &[HudChatStyledChar],
    source_interactions: &[HudChatInteraction],
) -> HudWrappedComponentChatLine {
    let mut runs = Vec::<HudComponentTextRun>::new();
    for styled in chars {
        if let Some(run) = runs.last_mut().filter(|run| {
            run.visual.style == styled.style
                && run.visual.color == styled.color
                && run.visual.shadow_color == styled.shadow_color
                && run.font == styled.font
        }) {
            run.visual.text.push(styled.ch);
        } else {
            runs.push(HudComponentTextRun {
                visual: HudStyledTextRun {
                    shadow_color: styled.shadow_color,
                    text: styled.ch.to_string(),
                    style: styled.style,
                    color: styled.color,
                },
                font: styled.font.clone(),
            });
        }
    }

    let mut interactions = Vec::<HudChatLineInteraction>::new();
    for styled in chars {
        let Some(target_index) = styled.target_index else {
            continue;
        };
        if interactions
            .iter()
            .any(|interaction: &HudChatLineInteraction| interaction.target_index == target_index)
        {
            continue;
        }
        let Some(source) = source_interactions
            .iter()
            .find(|interaction| interaction.target_index == target_index)
        else {
            continue;
        };
        interactions.push(HudChatLineInteraction {
            target_index,
            clickable: source.clickable,
            insertion: source.insertion,
            tooltip: source.tooltip.clone(),
        });
    }
    // The line's characters map straight onto these targets: the format runs
    // keep every character, so the glyph geometry indexes the same sequence.
    let targets = chars
        .iter()
        .map(|styled| styled.target_index)
        .collect::<Vec<_>>();
    HudWrappedComponentChatLine {
        runs,
        targets,
        interactions,
    }
}

/// `ChatComponent.messageHeight`.
pub(super) const HUD_CHAT_MESSAGE_HEIGHT: i32 = 9;
/// `ChatComponent`'s chat-area bottom margin `screenHeight - 40`
pub(super) const HUD_CHAT_BOTTOM_MARGIN: i32 = 40;

/// The last accepted hit per candidate predicate, exactly the
/// `ActiveTextCollector.ClickableStyleFinder` result set: `hovered` is the
/// target of the last hit area whatever its style
/// (`DrawingFocusedGraphicsAccess.accept`), `clickable` the last hit style with
/// a click event (`includeInsertions(false)`), and `clickable_or_insertion` the
/// last hit style with either (`includeInsertions(true)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HudActiveAreaHits<P> {
    pub(super) hovered: Option<P>,
    pub(super) clickable: Option<P>,
    pub(super) clickable_or_insertion: Option<P>,
}

impl<P> Default for HudActiveAreaHits<P> {
    fn default() -> Self {
        Self {
            hovered: None,
            clickable: None,
            clickable_or_insertion: None,
        }
    }
}

/// Vanilla `ActiveTextCollector.isPointInRectangle` (half-open on both axes).
fn hud_area_contains(local: (f32, f32), area: &HudGlyphArea) -> bool {
    local.0 >= area.left && local.0 < area.right && local.1 >= area.top && local.1 < area.bottom
}

/// `GuiTextRenderState.ensurePrepared` + `ScreenRectangle.containsPoint`: the
/// prepared bounds clip the accepted text to its drawn ink, so a text with no
/// drawable glyph contributes no bounds and can never be hit.
pub(super) fn hud_prepared_bounds_contain(
    bounds: Option<[f32; 4]>,
    origin: (f32, f32),
    click: (i32, i32),
) -> bool {
    let Some([left, top, right, bottom]) = bounds else {
        return false;
    };
    // `PreparedText.bounds()` rounds in font space; the pose translation the
    // click path uses for these surfaces is an integral offset.
    let (x, y) = (click.0 as f32, click.1 as f32);
    x >= left.floor() + origin.0
        && x < right.ceil() + origin.0
        && y >= top.floor() + origin.1
        && y < bottom.ceil() + origin.1
}

/// `ActiveTextCollector.findElementUnderCursor`
/// over one accepted text: every glyph instance first, then every `EmptyArea`,
/// each hit overwriting the previous candidate (last clickable hit wins).
/// `candidate` resolves an area's interaction target into its payload plus
/// whether that target carries a click event and an insertion. Returns whether
/// any area took the pointer, whatever its style.
pub(super) fn hud_active_area_hits<P: Copy>(
    geometry: &HudStyledTextGeometry,
    local: (f32, f32),
    candidate: impl Fn(usize) -> Option<(P, bool, bool)>,
    hits: &mut HudActiveAreaHits<P>,
) -> bool {
    let mut any = false;
    for (area, target) in geometry
        .glyph_areas
        .iter()
        .chain(geometry.empty_areas.iter())
    {
        if !hud_area_contains(local, area) {
            continue;
        }
        any = true;
        let payload = target.and_then(&candidate);
        hits.hovered = payload.map(|(payload, _, _)| payload);
        let Some((payload, clickable, insertion)) = payload else {
            continue;
        };
        if clickable {
            hits.clickable = Some(payload);
        }
        if clickable || insertion {
            hits.clickable_or_insertion = Some(payload);
        }
    }
    any
}

pub(super) fn hud_chat_prompt_contains(
    prompt: &HudChatPrompt,
    geometry: HudChatGeometry,
    text_top: i32,
    pointer: (i32, i32),
) -> bool {
    let mut hits = HudActiveAreaHits::default();
    geometry.active_area_hits(
        &prompt.text,
        text_top,
        pointer,
        |_| Some(((), true, false)),
        &mut hits,
    );
    hits.clickable.is_some()
}

/// The `ShowText` hover of the last prompt accepted under the pointer: the
/// clipped restriction prompt after the queue line (ChatComponent,
/// GuiGraphicsExtractor).
pub(super) fn hud_chat_prompt_tooltip(
    context: &HudChatContext,
    geometry: HudChatGeometry,
    queue: Option<&HudChatPrompt>,
    restricted: Option<&HudChatPrompt>,
    drawn_rows: usize,
) -> Option<HudRichTooltip> {
    let pointer = context.cursor_position.filter(|_| context.focused)?;
    let hovered = |prompt: &HudChatPrompt, text_top| {
        hud_chat_prompt_contains(prompt, geometry, text_top, pointer)
    };
    let text = if restricted.is_some_and(|prompt| {
        prompt.clipped && hovered(prompt, geometry.restricted_text_top(drawn_rows))
    }) {
        context.restricted_text.as_deref()
    } else if queue.is_some_and(|prompt| hovered(prompt, geometry.queue_text_top())) {
        context.queue_tooltip_text.as_deref()
    } else {
        None
    }?;
    Some(HudRichTooltip {
        style: None,
        lines: vec![HudRichTooltipLine {
            runs: hud_coded_string_runs(text)
                .into_iter()
                .map(HudComponentTextRun::from)
                .collect(),
            tint: HUD_TINT_WHITE,
        }],
        advanced_lines: None,
        wrap_text: true,
    })
}

pub(super) fn hud_chat_hit_target_at(
    lines: &[HudChatLine],
    context: &HudChatContext,
    fonts: &HudFontGlyphBook,
    geometry: HudChatGeometry,
    position: (f64, f64),
) -> Option<HudChatHitTarget> {
    if !context.focused || !position.0.is_finite() || !position.1.is_finite() {
        return None;
    }
    // `MouseButtonEvent.x()`/`y()` are already the truncated GUI pixels.
    let pointer = (position.0 as i32, position.1 as i32);
    let target = |kind| HudChatHitTarget {
        level_generation: context.level_generation,
        delivery_revision: context.delivery_revision,
        chat_screen_revision: context.chat_screen_revision,
        resource_generation: context.resource_generation,
        resources_loaded: context.resources_loaded,
        kind,
    };
    let rows = geometry.visible_rows(lines, context);
    // The capture replays the draw order: rows,
    // then the queue line, then the restriction prompt; the last hit wins.
    if let Some(restricted_text) = context.restricted_text.as_deref() {
        let prompt = hud_chat_restricted_prompt(restricted_text, fonts, geometry.max_width);
        if hud_chat_prompt_contains(
            &prompt,
            geometry,
            geometry.restricted_text_top(rows.len()),
            pointer,
        ) {
            return Some(target(HudChatHitTargetKind::Restrictions));
        }
    }
    if let Some(queue_text) = context
        .queue_text
        .as_deref()
        .filter(|_| context.queued_messages > 0)
    {
        let prompt = hud_chat_queue_prompt(queue_text, fonts);
        if hud_chat_prompt_contains(&prompt, geometry, geometry.queue_text_top(), pointer) {
            return Some(target(HudChatHitTargetKind::Queue));
        }
    }
    let hits = hud_chat_active_area_hits(lines, &rows, fonts, geometry, pointer);
    let (line_index, target_index) = hits.clickable_or_insertion?;
    let line = lines.get(line_index)?;
    Some(target(HudChatHitTargetKind::Message {
        display_sequence: line.display_sequence,
        target_index,
        // An unmodified click only ever consumes the target the click-only
        // scan accepted, and that target must belong to the same hit line.
        click_target_index: hits
            .clickable
            .filter(|(click_line, _)| *click_line == line_index)
            .map(|(_, index)| index),
    }))
}

fn hud_chat_line_text<'a>(
    line: &'a HudChatLine,
    fonts: &HudFontGlyphBook,
) -> std::borrow::Cow<'a, HudStyledTextGeometry> {
    match line.cached_geometry.as_ref() {
        Some(cached) => std::borrow::Cow::Borrowed(&cached.geometry),
        None => std::borrow::Cow::Owned(hud_component_text_hit_geometry(
            &line.runs,
            &line.targets,
            fonts,
            true,
        )),
    }
}

/// The glyph-area walk over every visible chat line, in vanilla visit order:
/// `ChatComponent.forEachLine` accepts the top line
/// first and the bottom line last, so a hit in an overlapping line region
/// belongs to the lower line. `captureClickableText` replays the draw path
/// so every line is tested through the chat pose.
fn hud_chat_active_area_hits(
    lines: &[HudChatLine],
    rows: &[(usize, usize, f32)],
    fonts: &HudFontGlyphBook,
    geometry: HudChatGeometry,
    pointer: (i32, i32),
) -> HudActiveAreaHits<(usize, usize)> {
    let mut hits = HudActiveAreaHits::default();
    for &(row, line_index, _) in rows {
        let line = &lines[line_index];
        geometry.active_area_hits(
            &hud_chat_line_text(line, fonts),
            geometry.text_top(row),
            pointer,
            |target_index| {
                line.interactions
                    .iter()
                    .find(|interaction| interaction.target_index == target_index)
                    .map(|interaction| {
                        (
                            (line_index, target_index),
                            interaction.clickable,
                            interaction.insertion,
                        )
                    })
            },
            &mut hits,
        );
    }
    hits
}

/// The tooltip hover of the focused chat: the style under the cursor
/// (`ChatComponent.DrawingFocusedGraphicsAccess`), independent of whether it is
/// clickable.
pub(super) fn hud_chat_line_interaction_at<'a>(
    lines: &'a [HudChatLine],
    context: &HudChatContext,
    fonts: &HudFontGlyphBook,
    geometry: HudChatGeometry,
    position: (f64, f64),
) -> Option<&'a HudChatLineInteraction> {
    if !context.focused || !position.0.is_finite() || !position.1.is_finite() {
        return None;
    }
    let hits = hud_chat_active_area_hits(
        lines,
        &geometry.visible_rows(lines, context),
        fonts,
        geometry,
        (position.0 as i32, position.1 as i32),
    );
    let (line_index, target_index) = hits.hovered?;
    lines
        .get(line_index)?
        .interactions
        .iter()
        .find(|interaction| interaction.target_index == target_index)
}

/// The first tag indicator or modified icon under the pointer, in draw order
/// the first deferred tooltip wins
pub(super) fn hud_chat_tag_at<'a>(
    lines: &'a [HudChatLine],
    context: &HudChatContext,
    geometry: HudChatGeometry,
    position: (f64, f64),
    fonts: &HudFontGlyphBook,
) -> Option<&'a HudChatTag> {
    if !context.focused || !position.0.is_finite() || !position.1.is_finite() {
        return None;
    }
    let pointer = (position.0.floor() as i32, position.1.floor() as i32);
    geometry
        .visible_rows(lines, context)
        .into_iter()
        .find_map(|(row, line_index, _)| {
            let line = &lines[line_index];
            let tag = line.tag.as_ref()?;
            let indicator = [-4, geometry.entry_top(row), -2, geometry.entry_bottom(row)];
            if geometry.local_rect_contains(pointer, indicator) {
                return Some(tag);
            }
            let icon = hud_chat_modified_icon_rect(line, geometry.text_top(row), fonts)?;
            geometry
                .local_rect_contains(pointer, hud_chat_icon_hover_rect(icon))
                .then_some(tag)
        })
}

/// `handleTagIcon`'s hover box runs one pixel below the sprite, to the text
/// bottom.
fn hud_chat_icon_hover_rect([left, top, right, bottom]: [i32; 4]) -> [i32; 4] {
    [left, top, right, bottom.wrapping_add(1)]
}

/// The modified-tag sprite box `[left, top, right, bottom]` in chat-local space
/// for any line of a modified message, `handleTagIcon`'s
/// `top = bottom - height - 1` over the text bottom
/// (704-707).
pub(super) fn hud_chat_modified_icon_rect(
    line: &HudChatLine,
    text_top: i32,
    fonts: &HudFontGlyphBook,
) -> Option<[i32; 4]> {
    const ICON_SIZE: i32 = 9;
    if line
        .tag
        .as_ref()
        .is_none_or(|tag| tag.kind != HudChatTagKind::Modified)
    {
        return None;
    }
    let left = hud_component_runs_width(&line.runs, fonts)
        .unwrap_or_default()
        .wrapping_add(4);
    let top = text_top
        .wrapping_add(HUD_CHAT_MESSAGE_HEIGHT)
        .wrapping_sub(ICON_SIZE + 1);
    Some([
        left,
        top,
        left.wrapping_add(ICON_SIZE),
        top.wrapping_add(ICON_SIZE),
    ])
}

/// The modified icons the focused access draws, as `(row, sprite box)`: a
/// row's own icon while the pointer is over its hover box, and a message's
/// end-of-entry icon while the pointer is over any glyph area of that message's
/// drawn rows. The background access draws none
/// (631-632, 704-715).
pub(super) fn hud_chat_modified_icons(
    lines: &[HudChatLine],
    context: &HudChatContext,
    fonts: &HudFontGlyphBook,
    geometry: HudChatGeometry,
) -> Vec<(usize, [i32; 4])> {
    let Some(pointer) = context.cursor_position.filter(|_| context.focused) else {
        return Vec::new();
    };
    let mut icons = Vec::new();
    let mut hovered_message = false;
    for (row, line_index, _) in geometry.visible_rows(lines, context) {
        let line = &lines[line_index];
        let text_top = geometry.text_top(row);
        hovered_message |= geometry.active_area_hits(
            &hud_chat_line_text(line, fonts),
            text_top,
            pointer,
            |_| None::<((), bool, bool)>,
            &mut HudActiveAreaHits::default(),
        );
        let force = line.end_of_entry && std::mem::take(&mut hovered_message);
        if let Some(icon) = hud_chat_modified_icon_rect(line, text_top, fonts) {
            if force || geometry.local_rect_contains(pointer, hud_chat_icon_hover_rect(icon)) {
                icons.push((row, icon));
            }
        }
    }
    icons
}

pub(super) fn hud_font_glyph(ch: char, glyphs: &HudFontGlyphMap) -> HudAsciiGlyph {
    glyphs.resolve(ch)
}

#[cfg(test)]
mod bidi_effect_tests;

#[cfg(test)]
mod fractional_tests;

#[cfg(test)]
mod wrapping_tests {
    use super::*;

    fn glyphs() -> HudFontGlyphMap {
        let mut glyphs = HudFontGlyphMap::new();
        for (ch, advance) in [
            ('a', 2.5),
            ('b', 2.5),
            (' ', 1.0),
            ('\t', 1.0),
            ('\r', 1.0),
            ('\u{a0}', 1.0),
            ('z', 0.0),
            ('n', -2.0),
            ('w', 8.0),
        ] {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    advance,
                    ..Default::default()
                },
            );
        }
        glyphs
    }

    fn wrapped(text: &str, width: u32) -> Vec<String> {
        hud_wrap_styled_text(
            &[HudStyledTextRun {
                text: text.into(),
                ..Default::default()
            }],
            width,
            &glyphs(),
        )
        .into_iter()
        .map(|runs| runs.into_iter().map(|run| run.text).collect())
        .collect()
    }

    #[test]
    fn component_wrap_preserves_whitespace_and_consumes_only_one_ascii_delimiter() {
        assert_eq!(wrapped("  a  ", 100), ["  a  "]);
        assert_eq!(wrapped("a  b", 4), ["a ", "b"]);
        assert_eq!(wrapped("a\tb", 4), ["a\t", "b"]);
        assert_eq!(wrapped("a\u{a0}b", 4), ["a\u{a0}", "b"]);
        assert_eq!(wrapped("a\rb", 4), ["a\r", "b"]);
        assert_eq!(wrapped(" a \n\nb\n", 100), [" a ", "", "b", ""]);
        assert!(wrapped("", 4).is_empty());
        assert_eq!(wrapped("a\n\n", 100), ["a", "", ""]);
        assert_eq!(wrapped("a ", 3), ["a"]);
    }

    #[test]
    fn component_wrap_uses_first_nonzero_advance_and_fractional_signed_widths() {
        assert_eq!(wrapped("zzwa", 1), ["zzw", "a"]);
        assert_eq!(wrapped("ab", 5), ["ab"]);
        assert_eq!(wrapped("nab", 3), ["nab"]);
        assert_eq!(wrapped("nwa", 3), ["n", "w", "a"]);
        assert_eq!(wrapped("  ", 0), [" "]);
        assert_eq!(wrapped("a wba", 5), ["a", "w", "ba"]);
    }

    #[test]
    fn component_text_public_metrics_preserve_fractional_mixed_font_bold_advances() {
        let mut custom = HudFontGlyphMap::new();
        custom.insert_first_wins(
            'b',
            HudAsciiGlyph {
                advance: 3.125,
                bold_offset: 0.5,
                ..Default::default()
            },
        );
        let fonts = HudFontGlyphBook::from_fonts(
            "minecraft:default",
            std::collections::BTreeMap::from([
                ("minecraft:default".into(), glyphs()),
                ("test:custom".into(), custom),
            ]),
        );
        let runs = vec![
            HudComponentTextRun::plain("a"),
            HudComponentTextRun {
                font: Some("test:custom".into()),
                visual: HudStyledTextRun {
                    text: "bb".into(),
                    style: HudTextStyle {
                        bold: true,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            },
        ];
        let first_width = hud_logical_component_text_width_f32(&runs[..1], &fonts);
        assert_eq!(
            hud_logical_component_text_width_f32(&runs, &fonts),
            first_width + 7.25
        );
        assert_eq!(
            hud_component_text_width(&runs, &fonts),
            (first_width + 7.25).ceil() as i32
        );
        let lines = hud_wrap_component_text(&runs, 7, &fonts);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0][1].visual.text, "b");
        assert_eq!(lines[1][0].font.as_deref(), Some("test:custom"));
        assert!(lines[1][0].visual.style.bold);
        assert_eq!(
            hud_logical_component_text_width_f32(&lines[1], &fonts),
            3.625
        );
        assert_eq!(hud_component_text_width(&lines[1], &fonts), 4);
        assert!(hud_wrap_component_text(&[], 6, &fonts).is_empty());
    }

    #[test]
    fn component_wrap_keeps_style_and_target_across_delimiter_and_hard_breaks() {
        let runs = [
            HudStyledTextRun {
                text: "a ".into(),
                color: Some(0xff0000),
                ..Default::default()
            },
            HudStyledTextRun {
                text: "wba".into(),
                color: Some(0x00ff00),
                shadow_color: Some(0),
                ..Default::default()
            },
        ];
        let interactions = [HudChatInteraction {
            start_char: 2,
            end_char: 5,
            target_index: 65536,
            clickable: false,
            insertion: true,
            tooltip: None,
        }];
        let lines = hud_wrap_chat_runs_with_interactions(&runs, &interactions, 5, &glyphs());
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].runs[0].text, "a");
        assert_eq!(lines[0].runs[0].color, Some(0xff0000));
        assert!(lines[0].interactions.is_empty());
        for (line, text) in lines[1..].iter().zip(["w", "ba"]) {
            assert_eq!(line.runs[0].text, text);
            assert_eq!(line.runs[0].color, Some(0x00ff00));
            assert_eq!(line.runs[0].shadow_color, Some(0));
            assert_eq!(line.interactions[0].target_index, 65536);
        }
    }

    #[test]
    fn component_wrap_uses_each_font_and_bold_advance_across_run_boundaries() {
        let mut custom = HudFontGlyphMap::new();
        custom.insert_first_wins(
            'b',
            HudAsciiGlyph {
                advance: 4.0,
                bold_offset: 1.0,
                ..Default::default()
            },
        );
        let fonts = HudFontGlyphBook::from_fonts(
            "minecraft:default",
            std::collections::BTreeMap::from([
                ("minecraft:default".into(), glyphs()),
                ("test:wide".into(), custom),
            ]),
        );
        let runs = [
            HudComponentTextRun::from(HudStyledTextRun::plain("a ")),
            HudComponentTextRun {
                font: Some("test:wide".into()),
                visual: HudStyledTextRun {
                    text: "bb".into(),
                    style: HudTextStyle {
                        bold: true,
                        ..Default::default()
                    },
                    color: Some(0x00ff00),
                    ..Default::default()
                },
            },
        ];
        let targets = [HudChatInteraction {
            start_char: 2,
            end_char: 4,
            target_index: 9,
            clickable: true,
            insertion: false,
            tooltip: None,
        }];
        let lines = hud_wrap_component_runs_with_interactions(&runs, &targets, 6, &fonts);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].runs[0].visual.text, "a");
        for line in &lines[1..] {
            assert_eq!(line.runs[0].visual.text, "b");
            assert_eq!(line.runs[0].font.as_deref(), Some("test:wide"));
            assert!(line.runs[0].visual.style.bold);
            assert_eq!(line.interactions.len(), 1);
            assert_eq!(line.interactions[0].target_index, 9);
            assert_eq!(line.targets, vec![Some(9)]);
            let geometry = hud_component_text_hit_geometry(&line.runs, &line.targets, &fonts, true);
            assert_eq!(geometry.glyph_areas[0].0.right, 5.0);
        }
    }

    #[test]
    fn plain_and_multiline_wrap_share_boundaries_without_losing_editor_empty_line() {
        let glyphs = glyphs();
        assert_eq!(hud_wrap_plain_text("zzwa\n", 1, &glyphs), ["zzw", "a", ""]);
        let ranges = hud_multiline_lines("zzwa\n", 1, &glyphs);
        assert_eq!(
            ranges
                .iter()
                .map(|line| (line.begin, line.end))
                .collect::<Vec<_>>(),
            [(0, 3), (3, 4), (5, 5)]
        );
        assert_eq!(hud_multiline_lines("", 1, &glyphs).len(), 1);
        assert_eq!(hud_wrap_plain_text("a wba", 5, &glyphs), ["a", "w", "ba"]);
    }
}
