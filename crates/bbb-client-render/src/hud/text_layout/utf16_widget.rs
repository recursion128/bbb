use super::*;

pub(in crate::hud) fn utf16_widget_counter(text: &str, max_length: usize) -> String {
    format!("{}/{}", text.encode_utf16().count(), max_length)
}

pub(in crate::hud) fn utf16_widget_slice(units: &[u16], begin: usize, end: usize) -> String {
    let begin = begin.min(units.len());
    String::from_utf16_lossy(&units[begin..end.min(units.len()).max(begin)])
}

pub(in crate::hud) fn utf16_widget_lines(
    text: &str,
    width: u32,
    glyphs: &HudFontGlyphMap,
) -> (Vec<u16>, Vec<HudMultilineLine>) {
    let units = text.encode_utf16().collect();
    let mut offsets = vec![0];
    for ch in text.chars() {
        offsets.push(offsets.last().copied().unwrap() + ch.len_utf16());
    }
    // Wrap the complete display first: cursor-local replacement glyphs must
    // not change the line containing the original pair.
    let lines = hud_multiline_lines(text, width, glyphs)
        .into_iter()
        .map(|line| HudMultilineLine {
            begin: offsets[line.begin],
            end: offsets[line.end],
        })
        .collect();
    (units, lines)
}

pub(in crate::hud) fn utf16_widget_input_layout(
    label: &HudInventoryTextLabel,
    input: HudInventoryTextInputDecoration,
    glyphs: &HudFontGlyphMap,
) -> HudInventoryTextInputLayout {
    let units = label.text.encode_utf16().collect::<Vec<_>>();
    let cursor = input.cursor.min(units.len());
    let selection = input.selection.min(units.len());
    let target = input.scroll_to.min(units.len());
    let width = i32::try_from(label.width).unwrap_or(i32::MAX);
    let start = glyphs
        .plain_utf16_scroll_to(&units, 0, target, width)
        .unwrap_or(0);
    let len = glyphs
        .plain_utf16_head_len(&units[start..], width)
        .unwrap_or(0);
    let end = start + len;
    let cursor_on_screen = (start..=end).contains(&cursor);
    let insert_cursor = cursor < units.len() || units.len() >= input.max_length;
    let cursor_x = if cursor < start {
        0.0
    } else if cursor > end {
        label.width as f32
    } else {
        hud_raw_string_width(&utf16_widget_slice(&units, start, cursor), glyphs) as f32
            + match (len > 0, insert_cursor) {
                (true, false) => 1.0,
                (false, true) => -1.0,
                _ => 0.0,
            }
    };
    let selection_rect = (selection != cursor).then(|| {
        let selection_x = hud_raw_string_width(
            &utf16_widget_slice(&units, start, selection.clamp(start, end)),
            glyphs,
        ) as f32
            - 1.0;
        let left = cursor_x.min(selection_x);
        let right = cursor_x.max(selection_x);
        (left, (right - left).ceil().max(0.0) as u32)
    });
    HudInventoryTextInputLayout {
        split_text: cursor_on_screen.then(|| {
            (
                utf16_widget_slice(&units, start, cursor),
                utf16_widget_slice(&units, cursor, end),
            )
        }),
        displayed_text: utf16_widget_slice(&units, start, end),
        cursor_x,
        cursor_on_screen,
        insert_cursor,
        selection_rect,
    }
}

#[cfg(test)]
mod tests;
