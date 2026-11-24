use super::*;
use bbb_render_types::{visit_legacy_utf16, LegacyTextFormat};
use std::collections::VecDeque;

struct FormattedPart {
    text: Vec<u16>,
    style: HudChatStyledChar,
}

pub fn hud_resolve_formatted_component_text(
    runs: &[HudComponentTextRun],
) -> Vec<HudComponentTextRun> {
    component_runs(resolve_parts(raw_parts(runs, &[])))
}

fn raw_parts(
    runs: &[HudComponentTextRun],
    interactions: &[HudChatInteraction],
) -> VecDeque<FormattedPart> {
    let mut parts = VecDeque::new();
    let mut offset = 0;
    for run in runs {
        let mut part: Option<FormattedPart> = None;
        for ch in run.visual.text.chars() {
            let target_index = interactions
                .iter()
                .find(|target| target.start_char <= offset && offset < target.end_char)
                .map(|target| target.target_index);
            offset += 1;
            if part
                .as_ref()
                .is_some_and(|part| part.style.target_index != target_index)
            {
                parts.push_back(part.take().unwrap());
            }
            let part = part.get_or_insert_with(|| FormattedPart {
                text: Vec::new(),
                style: HudChatStyledChar {
                    ch: '\0',
                    style: run.visual.style,
                    color: run.visual.color,
                    shadow_color: run.visual.shadow_color,
                    font: run.font.clone(),
                    target_index,
                },
            });
            part.text.extend_from_slice(ch.encode_utf16(&mut [0; 2]));
        }
        if let Some(part) = part {
            parts.push_back(part);
        }
    }
    parts
}

pub(in crate::hud) fn hud_wrap_formatted_component_runs_with_interactions(
    runs: &[HudComponentTextRun],
    interactions: &[HudChatInteraction],
    width: u32,
    fonts: &HudFontGlyphBook,
) -> Vec<HudWrappedComponentChatLine> {
    wrap_parts(raw_parts(runs, interactions), width, fonts)
        .into_iter()
        .map(|chars| hud_chat_chars_to_component_line(&chars, interactions))
        .collect()
}

// StringSplitter: split with the root reset, then resolve each resulting part.
pub fn hud_wrap_formatted_component_text(
    runs: &[HudComponentTextRun],
    width: u32,
    fonts: &HudFontGlyphBook,
) -> Vec<Vec<HudComponentTextRun>> {
    wrap_parts(raw_parts(runs, &[]), width, fonts)
        .into_iter()
        .map(component_runs)
        .collect()
}

pub fn hud_resolve_formatted_utf16_text(text: &[u16]) -> Vec<HudStyledTextRun> {
    hud_resolve_formatted_utf16_text_with_style(text, HudTextStyle::default())
}

/// `StringDecomposer.iterateFormatted(text, style)` passes the caller's style as
/// both the current and the reset style, so a `§r` inside the text returns to
/// it. A `Component.withStyle(ITALIC)` line is resolved this way, which no
/// "resolve then set italic" pass can reproduce.
pub fn hud_resolve_formatted_utf16_text_with_style(
    text: &[u16],
    style: HudTextStyle,
) -> Vec<HudStyledTextRun> {
    let part = FormattedPart {
        text: text.to_vec(),
        style: HudChatStyledChar {
            ch: '\0',
            style,
            color: None,
            shadow_color: None,
            font: None,
            target_index: None,
        },
    };
    component_runs(resolve_parts([part]))
        .into_iter()
        .map(|run| run.visual)
        .collect()
}

pub fn hud_formatted_utf16_width(text: &[u16], glyphs: &HudFontGlyphMap) -> i32 {
    let root = HudChatStyledChar {
        ch: '\0',
        style: HudTextStyle::default(),
        color: None,
        shadow_color: None,
        font: None,
        target_index: None,
    };
    let mut width = 0.0_f32;
    visit_legacy_utf16(text, root.clone(), &root, apply_format, |source, style| {
        width += hud_font_glyph(source.ch, glyphs).styled_advance_f32(style.style);
        true
    });
    if glyphs.is_available() {
        width.ceil() as i32
    } else {
        0
    }
}

// StringSplitter: ranges retain raw UTF-16 indices and carry split style.
pub fn hud_formatted_utf16_line_ranges(
    text: &[u16],
    width: u32,
    glyphs: &HudFontGlyphMap,
) -> Vec<std::ops::Range<usize>> {
    let root = HudChatStyledChar {
        ch: '\0',
        style: HudTextStyle::default(),
        color: None,
        shadow_color: None,
        font: None,
        target_index: None,
    };
    let mut style = root.clone();
    let mut start = 0;
    let mut lines = Vec::new();
    while start < text.len() {
        let mut advance = 0.0;
        let mut had_nonzero = false;
        let mut space = None;
        let mut split = None;
        visit_legacy_utf16(
            &text[start..],
            style.clone(),
            &root,
            apply_format,
            |source, style| {
                let position = start + source.utf16_index;
                if source.ch == '\n' {
                    split = Some((position, style.clone()));
                    return false;
                }
                if source.ch == ' ' {
                    space = Some((position, style.clone()));
                }
                let next = hud_font_glyph(source.ch, glyphs).styled_advance_f32(style.style);
                advance += next;
                if had_nonzero && advance > width.max(1) as f32 {
                    split = Some(space.clone().unwrap_or((position, style.clone())));
                    return false;
                }
                had_nonzero |= next != 0.0;
                true
            },
        );
        let Some((position, split_style)) = split else {
            lines.push(start..text.len());
            break;
        };
        lines.push(start..position);
        start = position + usize::from(matches!(text[position], 10 | 32));
        style = split_style;
    }
    if text.is_empty() || text.last() == Some(&10) {
        lines.push(text.len()..text.len());
    }
    lines
}

fn wrap_parts(
    mut parts: VecDeque<FormattedPart>,
    width: u32,
    fonts: &HudFontGlyphBook,
) -> Vec<Vec<HudChatStyledChar>> {
    let root = HudChatStyledChar {
        ch: '\0',
        style: HudTextStyle::default(),
        color: None,
        shadow_color: None,
        font: None,
        target_index: None,
    };
    let mut lines = Vec::new();
    let mut trailing_newline = false;
    while !parts.is_empty() {
        let mut advance = 0.0;
        let mut had_nonzero = false;
        let mut offset = 0;
        let mut space = None;
        let mut split = None;
        for part in &parts {
            let complete = visit_legacy_utf16(
                &part.text,
                part.style.clone(),
                &root,
                apply_format,
                |source, style| {
                    let position = offset + source.utf16_index;
                    if source.ch == '\n' {
                        split = Some((position, style.clone()));
                        return false;
                    }
                    if source.ch == ' ' {
                        space = Some((position, style.clone()));
                    }
                    let next = hud_font_glyph(source.ch, fonts.glyphs(style.font.as_deref()))
                        .styled_advance_f32(style.style);
                    advance += next;
                    if had_nonzero && advance > width.max(1) as f32 {
                        split = Some(space.clone().unwrap_or((position, style.clone())));
                        return false;
                    }
                    had_nonzero |= next != 0.0;
                    true
                },
            );
            if !complete {
                break;
            }
            offset += part.text.len();
        }
        let Some((position, style)) = split else {
            lines.push(resolve_parts(parts.drain(..)));
            trailing_newline = false;
            break;
        };
        let unit = parts
            .iter()
            .flat_map(|part| &part.text)
            .nth(position)
            .copied()
            .unwrap();
        trailing_newline = unit == 10;
        let skip = usize::from(matches!(unit, 10 | 32));
        lines.push(resolve_parts(split_parts(
            &mut parts, position, skip, style,
        )));
    }
    if trailing_newline {
        lines.push(Vec::new());
    }
    lines
}

// StringSplitter: preserve raw codes and assign split style only to the remaining part.
fn split_parts(
    parts: &mut VecDeque<FormattedPart>,
    mut position: usize,
    skip: usize,
    style: HudChatStyledChar,
) -> Vec<FormattedPart> {
    let mut line = Vec::new();
    while let Some(mut part) = parts.pop_front() {
        if position > part.text.len() {
            position -= part.text.len();
            line.push(part);
            continue;
        }
        if position != 0 {
            line.push(FormattedPart {
                text: part.text[..position].to_vec(),
                style: part.style.clone(),
            });
        }
        position += skip;
        loop {
            if position <= part.text.len() {
                if position < part.text.len() {
                    part.text.drain(..position);
                    part.style = style;
                    parts.push_front(part);
                }
                return line;
            }
            position -= part.text.len();
            let Some(next) = parts.pop_front() else {
                return line;
            };
            part = next;
        }
    }
    line
}

// SubStringSource: visual text resets to each part's own style before bidi ordering.
fn resolve_parts(parts: impl IntoIterator<Item = FormattedPart>) -> Vec<HudChatStyledChar> {
    let mut chars = Vec::new();
    for part in parts {
        visit_legacy_utf16(
            &part.text,
            part.style.clone(),
            &part.style,
            apply_format,
            |source, style| {
                let mut value = style.clone();
                value.ch = source.ch;
                chars.push(value);
                true
            },
        );
    }
    chars
}

fn component_runs(chars: Vec<HudChatStyledChar>) -> Vec<HudComponentTextRun> {
    let mut runs: Vec<HudComponentTextRun> = Vec::new();
    for value in chars {
        if let Some(last) = runs.last_mut().filter(|run| {
            run.visual.style == value.style
                && run.visual.color == value.color
                && run.visual.shadow_color == value.shadow_color
                && run.font == value.font
        }) {
            last.visual.text.push(value.ch);
        } else {
            runs.push(HudComponentTextRun {
                visual: HudStyledTextRun {
                    text: value.ch.to_string(),
                    style: value.style,
                    color: value.color,
                    shadow_color: value.shadow_color,
                },
                font: value.font,
            });
        }
    }
    runs
}

fn apply_format(run: &mut HudChatStyledChar, format: LegacyTextFormat) {
    match format {
        LegacyTextFormat::Color(color) => {
            run.color = Some(color);
            run.style = HudTextStyle::default();
        }
        LegacyTextFormat::Bold => run.style.bold = true,
        LegacyTextFormat::Italic => run.style.italic = true,
        LegacyTextFormat::Underlined => run.style.underlined = true,
        LegacyTextFormat::Strikethrough => run.style.strikethrough = true,
        LegacyTextFormat::Obfuscated => run.style.obfuscated = true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fonts() -> HudFontGlyphBook {
        let map = |advance| {
            let mut map = HudFontGlyphMap::new();
            for ch in ['A', 'B', 'a', 'X', '�', ' '] {
                map.insert_first_wins(
                    ch,
                    HudAsciiGlyph {
                        advance,
                        ..Default::default()
                    },
                );
            }
            map
        };
        HudFontGlyphBook::from_fonts(
            "minecraft:default",
            BTreeMap::from([
                ("minecraft:default".into(), map(1.0)),
                ("test:wide".into(), map(2.0)),
            ]),
        )
    }

    fn text(lines: &[Vec<HudComponentTextRun>]) -> Vec<String> {
        lines
            .iter()
            .map(|line| line.iter().map(|run| run.visual.text.as_str()).collect())
            .collect()
    }

    #[test]
    fn formatted_utf16_editor_ranges_keep_codes_surrogates_and_trailing_empty_line() {
        let fonts = fonts();
        let glyphs = fonts.glyphs(None);
        let text: Vec<_> = "A§😀B\n".encode_utf16().collect();
        assert_eq!(
            hud_formatted_utf16_line_ranges(&text, 1, glyphs),
            [0..3, 3..4, 4..5, 6..6]
        );
        assert_eq!(
            hud_formatted_utf16_line_ranges(&[], 1, glyphs),
            std::slice::from_ref(&(0..0))
        );
        let text: Vec<_> = "§aAB§".encode_utf16().collect();
        assert_eq!(
            hud_formatted_utf16_line_ranges(&text, 1, glyphs),
            [0..3, 3..5]
        );
    }

    #[test]
    fn formatted_utf16_editor_ranges_carry_style_and_reset_at_root() {
        let fonts = fonts();
        let glyphs = fonts.glyphs(None);
        let text: Vec<_> = "§lAB§rAB".encode_utf16().collect();
        assert_eq!(
            hud_formatted_utf16_line_ranges(&text, 2, glyphs),
            [0..3, 3..6, 6..8]
        );
    }

    #[test]
    fn formatted_chat_reset_restores_part_target_but_split_root_clears_it() {
        let runs = [HudComponentTextRun::plain("A§rB")];
        let targets = [HudChatInteraction {
            start_char: 0,
            end_char: 4,
            target_index: 65536,
            clickable: false,
            insertion: true,
            tooltip: None,
        }];
        let whole =
            hud_wrap_formatted_component_runs_with_interactions(&runs, &targets, 2, &fonts());
        assert_eq!(whole[0].interactions[0].target_index, 65536);
        assert_eq!(whole[0].targets, vec![Some(65536), Some(65536)]);
        let split =
            hud_wrap_formatted_component_runs_with_interactions(&runs, &targets, 1, &fonts());
        assert_eq!(split.len(), 2);
        assert_eq!(split[0].interactions[0].target_index, 65536);
        assert!(split[1].interactions.is_empty());
        assert_eq!(split[1].targets, vec![None]);
        assert_eq!(split[1].runs[0].visual.text, "B");
    }

    #[test]
    fn formatted_chat_uses_source_ranges_after_codes_and_keeps_part_boundaries() {
        let runs = [
            HudComponentTextRun::plain("§"),
            HudComponentTextRun::plain("aX"),
            HudComponentTextRun::plain("§aB"),
        ];
        let targets = [HudChatInteraction {
            start_char: 3,
            end_char: 6,
            target_index: 9,
            clickable: true,
            insertion: false,
            tooltip: None,
        }];
        let lines =
            hud_wrap_formatted_component_runs_with_interactions(&runs, &targets, 10, &fonts());
        assert_eq!(
            lines[0]
                .runs
                .iter()
                .map(|run| run.visual.text.as_str())
                .collect::<String>(),
            "aXB"
        );
        assert_eq!(lines[0].interactions.len(), 1);
        assert_eq!(lines[0].targets, vec![None, None, Some(9)]);
        assert_eq!(lines[0].interactions[0].target_index, 9);
        let message = HudChatMessage {
            display_sequence: 42,
            added_tick: 0,
            runs: runs.to_vec(),
            interactions: targets.to_vec(),
            tag: None,
        };
        let cached =
            hud_wrap_chat_messages(&[message], &fonts(), HudChatOptions::default().wrap_width());
        assert_eq!(cached[0].display_sequence, 42);
        assert_eq!(
            cached[0]
                .runs
                .iter()
                .map(|run| run.visual.text.as_str())
                .collect::<String>(),
            "aXB"
        );
        assert_eq!(cached[0].interactions[0].target_index, 9);
        assert_eq!(cached[0].targets, vec![None, None, Some(9)]);
    }

    #[test]
    fn formatted_wrap_distinguishes_split_reset_from_visual_part_reset() {
        let mut run = HudComponentTextRun::plain("A§rB");
        run.font = Some("test:wide".into());
        let unbroken = hud_wrap_formatted_component_text(&[run.clone()], 3, &fonts());
        assert_eq!(text(&unbroken), ["AB"]);
        assert_eq!(unbroken[0][0].font.as_deref(), Some("test:wide"));
        let broken = hud_wrap_formatted_component_text(&[run], 2, &fonts());
        assert_eq!(text(&broken), ["A", "B"]);
        assert_eq!(broken[0][0].font.as_deref(), Some("test:wide"));
        assert_eq!(broken[1][0].font, None);
    }

    #[test]
    fn formatted_wrap_preserves_parts_and_orphan_tail_at_break() {
        let lines = hud_wrap_formatted_component_text(
            &[
                HudComponentTextRun::plain("§"),
                HudComponentTextRun::plain("aX"),
            ],
            10,
            &fonts(),
        );
        assert_eq!(text(&lines), ["aX"]);
        assert_eq!(lines[0][0].visual.color, None);
        let lines =
            hud_wrap_formatted_component_text(&[HudComponentTextRun::plain("A§😀B")], 1, &fonts());
        assert_eq!(text(&lines), ["A", "�", "B"]);
    }

    #[test]
    fn formatted_wrap_keeps_style_across_newline_and_trailing_empty_line() {
        let lines = hud_wrap_formatted_component_text(
            &[HudComponentTextRun::plain("§aA\nB\n")],
            10,
            &fonts(),
        );
        assert_eq!(text(&lines), ["A", "B", ""]);
        assert_eq!(lines[0][0].visual.color, Some(0x55ff55));
        assert_eq!(lines[1][0].visual.color, Some(0x55ff55));
        assert!(hud_wrap_formatted_component_text(&[], 1, &fonts()).is_empty());
    }
}
