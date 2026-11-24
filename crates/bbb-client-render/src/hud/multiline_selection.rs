use std::ops::Range;

use super::{absolute_hud_rect, HudRect};

// AbstractTextAreaWidget includes lines touching either viewport edge.
pub(super) fn multiline_line_visible(y: f32, viewport_y: (f32, f32)) -> bool {
    y + 9.0 >= viewport_y.0 && y <= viewport_y.1
}

// MultiLineEditBox extends continuing selections beyond the text width.
pub(super) fn multiline_selection_rect(
    line: Range<usize>,
    selection: Range<usize>,
    origin: (f32, f32),
    continuation_width: i32,
    viewport_y: (f32, f32),
    mut measure: impl FnMut(usize, usize) -> i32,
) -> Option<HudRect> {
    if selection.start >= selection.end
        || selection.start > line.end
        || line.start > selection.end
        || !multiline_line_visible(origin.1, viewport_y)
    {
        return None;
    }
    let start = measure(line.start, selection.start.max(line.start));
    let end = if selection.end > line.end {
        continuation_width
    } else {
        measure(line.start, selection.end)
    };
    (start != end).then(|| {
        absolute_hud_rect(
            origin.0 + start.min(end) as f32,
            origin.1,
            start.abs_diff(end),
            9,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(line: Range<usize>, selection: Range<usize>) -> Option<HudRect> {
        multiline_selection_rect(line, selection, (4.0, 4.0), 36, (0.0, 20.0), |a, b| {
            ((b - a) * 6) as i32
        })
    }

    #[test]
    fn multiline_selection_continues_through_newline_and_empty_lines_to_widget_edge() {
        for (line, selection, x, width) in [
            (0..2, 1..5, 10.0, 30),
            (0..2, 2..3, 16.0, 24),
            (3..3, 2..5, 4.0, 36),
            (4..6, 2..5, 4.0, 6),
        ] {
            let selected = rect(line, selection).unwrap();
            assert_eq!(
                (selected.x, selected.y, selected.width, selected.height),
                (x, 4.0, width, 9)
            );
        }
        assert!(rect(0..2, 3..5).is_none());
        assert!(rect(4..6, 0..3).is_none());
        assert!(rect(4..6, 0..4).is_none());
        assert!(rect(0..2, 1..1).is_none());
    }

    #[test]
    fn multiline_selection_preserves_zero_and_negative_measured_widths() {
        assert!(
            multiline_selection_rect(0..2, 0..1, (4.0, 4.0), 36, (0.0, 20.0), |_, _| 0).is_none()
        );
        let selected = multiline_selection_rect(0..2, 0..1, (4.0, 4.0), 36, (0.0, 20.0), |a, b| {
            -6 * (b - a) as i32
        })
        .unwrap();
        assert_eq!((selected.x, selected.width), (-2.0, 6));
    }

    #[test]
    fn multiline_selection_uses_scrolled_line_box_before_glyph_overhang() {
        for (y, visible) in [(-10.0, false), (-9.0, true), (20.0, true), (21.0, false)] {
            assert_eq!(multiline_line_visible(y, (0.0, 20.0)), visible);
            assert_eq!(
                multiline_selection_rect(0..2, 0..1, (4.0, y), 36, (0.0, 20.0), |a, b| {
                    ((b - a) * 6) as i32
                })
                .is_some(),
                visible
            );
        }
    }
}
