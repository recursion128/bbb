//! Decoration boundaries at the run level: vanilla keys the one-pixel left
//! expansion on `FormattedCharSink`'s position, the local UTF-16 index inside
//! the glyph's own `FormattedCharSequence` part, not on the
//! first drawn glyph of the line.

use super::*;
use bbb_render_types::{visual_order_indexed, HudSubsequence, HudSubsequenceKey};

// Uniform advances make a pen position identify the drawn glyph exactly.
const ADVANCE: f32 = 1.0;

fn bidi_glyphs() -> HudFontGlyphBook {
    let mut glyphs = HudFontGlyphMap::new();
    for codepoint in [
        'a', 'b', 'c', 'd', 'x', 'א', 'ב', 'ג', '😀', // U+1F600 stays one codepoint
        '\u{633}', '\u{644}', '\u{627}', '\u{645}',
        // The presentation forms and the Lam-Alef ligature ICU shaping selects.
        '\u{feb3}', '\u{fefc}', '\u{fee1}', '\u{fefb}',
    ] {
        glyphs.insert_codepoint_first_wins(
            u32::from(codepoint),
            HudAsciiGlyph {
                advance: ADVANCE,
                width: 1,
                height: 8,
                ..Default::default()
            },
        );
    }
    HudFontGlyphBook::from(glyphs)
}

fn underlined(text: &str, color: u32) -> HudComponentTextRun {
    HudComponentTextRun::from(HudStyledTextRun {
        text: text.to_string(),
        style: HudTextStyle {
            underlined: true,
            ..Default::default()
        },
        color: Some(color),
        shadow_color: None,
    })
}

/// The drawn `(codepoint, local index)` sequence of the same runs the geometry
/// pass walks, keyed on the same vanilla `Style` facts it uses.
fn drawn_local_indices(
    runs: &[HudComponentTextRun],
    targets: &[Option<usize>],
) -> Vec<(char, usize)> {
    let mut logical = Vec::new();
    let mut char_index = 0usize;
    for (run_index, run) in runs.iter().enumerate() {
        for ch in run.visual.text.chars() {
            let target = targets.get(char_index).copied().flatten();
            logical.push((
                ch,
                HudSubsequence {
                    payload: run_index,
                    key: HudSubsequenceKey {
                        style: run.visual.style,
                        color: run.visual.color,
                        shadow_color: run.visual.shadow_color,
                        font: run.font.as_deref(),
                        target,
                    },
                },
            ));
            char_index += 1;
        }
    }
    visual_order_indexed(&logical, false)
        .into_iter()
        .map(|(ch, _, local_index)| (ch, local_index))
        .collect()
}

/// Asserts, per drawn glyph, the local index, the pen, and the underline's left
/// edge, plus that the left edge expands exactly where that index is zero.
fn assert_drawn_glyphs(
    runs: &[HudComponentTextRun],
    targets: &[Option<usize>],
    expected: &[(char, usize, f32, f32)],
) {
    let fonts = bidi_glyphs();
    assert_eq!(
        drawn_local_indices(runs, targets),
        expected
            .iter()
            .map(|(ch, local_index, _, _)| (*ch, *local_index))
            .collect::<Vec<_>>(),
        "drawn order and local index"
    );
    let geometry = hud_component_text_hit_geometry(runs, targets, &fonts, false);
    assert_eq!(geometry.glyph_quads.len(), expected.len(), "drawn glyphs");
    assert_eq!(
        geometry.effect_rects.len(),
        expected.len(),
        "underline bars"
    );
    for (index, (ch, local_index, pen, effect_x0)) in expected.iter().enumerate() {
        assert_eq!(
            geometry.glyph_quads[index].0.corners[0][0], *pen,
            "glyph {index} ({ch:?}) pen"
        );
        let rect = geometry.effect_rects[index].0;
        assert_eq!(rect.x0, *effect_x0, "glyph {index} ({ch:?}) bar left edge");
        assert_eq!(
            rect.x1,
            *pen + ADVANCE,
            "glyph {index} ({ch:?}) bar right edge"
        );
        assert_eq!(
            *effect_x0 == *pen - 1.0,
            *local_index == 0,
            "glyph {index} ({ch:?}) expands left exactly at its part's index 0"
        );
    }
}

#[test]
fn pure_ltr_line_starts_one_subsequence_at_the_first_glyph() {
    let runs = [underlined("ab", 0xffffff)];
    assert_drawn_glyphs(
        &runs,
        &[None, None],
        &[('a', 0, 0.0, -1.0), ('b', 1, 1.0, 1.0)],
    );
}

#[test]
fn pure_rtl_line_starts_one_subsequence_at_the_last_drawn_glyph() {
    let runs = [underlined("אבג", 0xffffff)];
    // The odd run is drawn backwards, so its index 0 glyph closes the bar.
    assert_drawn_glyphs(
        &runs,
        &[None, None, None],
        &[('ג', 2, 0.0, 0.0), ('ב', 1, 1.0, 1.0), ('א', 0, 2.0, 1.0)],
    );
}

#[test]
fn cross_style_line_expands_at_each_subsequence_start() {
    let runs = [underlined("ab", 0xff0000), underlined("אב", 0x0000ff)];
    // Two parts: the forward run resets at `a`, the odd run at its own `א`.
    assert_drawn_glyphs(
        &runs,
        &[None; 4],
        &[
            ('a', 0, 0.0, -1.0),
            ('b', 1, 1.0, 1.0),
            ('ב', 1, 2.0, 2.0),
            ('א', 0, 3.0, 2.0),
        ],
    );
}

#[test]
fn interaction_only_style_change_starts_a_new_subsequence() {
    // Identical glyphs and identical visible style: only the click target
    // changes, which is part of vanilla `Style` equality.
    let runs = [underlined("ab", 0xffffff), underlined("ab", 0xffffff)];
    let targets = [Some(0), Some(0), Some(1), Some(1)];
    assert_drawn_glyphs(
        &runs,
        &targets,
        &[
            ('a', 0, 0.0, -1.0),
            ('b', 1, 1.0, 1.0),
            ('a', 0, 2.0, 1.0),
            ('b', 1, 3.0, 3.0),
        ],
    );
    // The hit areas keep the same drawn order, so a click cannot disagree
    // with the glyph it covers.
    let fonts = bidi_glyphs();
    let geometry = hud_component_text_hit_geometry(&runs, &targets, &fonts, false);
    assert_eq!(
        geometry
            .glyph_areas
            .iter()
            .map(|(area, target)| (area.left, *target))
            .collect::<Vec<_>>(),
        [
            (0.0, Some(0)),
            (1.0, Some(0)),
            (2.0, Some(1)),
            (3.0, Some(1))
        ]
    );
}

#[test]
fn supplementary_pair_keeps_its_utf16_offsets_in_both_directions() {
    let ltr = [underlined("a😀b", 0xffffff)];
    assert_drawn_glyphs(
        &ltr,
        &[None; 3],
        &[('a', 0, 0.0, -1.0), ('😀', 1, 1.0, 1.0), ('b', 3, 2.0, 2.0)],
    );
    let rtl = [underlined("א😀ב", 0xffffff)];
    // The pair is reordered as one codepoint and keeps its first unit's offset.
    assert_drawn_glyphs(
        &rtl,
        &[None; 3],
        &[('ב', 3, 0.0, 0.0), ('😀', 1, 1.0, 1.0), ('א', 0, 2.0, 1.0)],
    );
}

#[test]
fn arabic_shaping_indexes_the_shaped_units() {
    let shaped = [underlined("سلام", 0xffffff)];
    // Four logical units shape into three drawn glyphs, so the indices are the
    // shaped-string offsets vanilla reorders.
    assert_drawn_glyphs(
        &shaped,
        &[None; 4],
        &[
            ('\u{fee1}', 2, 0.0, 0.0),
            ('\u{fefc}', 1, 1.0, 1.0),
            ('\u{feb3}', 0, 2.0, 1.0),
        ],
    );
    // Lam-Alef compaction plus an LTR tail: one run each, so both drawn glyphs
    // are their own part's index 0.
    let ligature = [underlined("لاx", 0xffffff)];
    assert_drawn_glyphs(
        &ligature,
        &[None; 3],
        &[('x', 0, 0.0, -1.0), ('\u{fefb}', 0, 1.0, 0.0)],
    );
}

#[test]
fn style_equal_component_parts_stay_one_subsequence() {
    // Two component parts that resolve to one vanilla `Style` are one
    // `FormattedCharSequence` part, so the text they happen to carry must not
    // restart the index.
    let runs = [underlined("ab", 0xffffff), underlined("cd", 0xffffff)];
    assert_drawn_glyphs(
        &runs,
        &[None; 4],
        &[
            ('a', 0, 0.0, -1.0),
            ('b', 1, 1.0, 1.0),
            ('c', 2, 2.0, 2.0),
            ('d', 3, 3.0, 3.0),
        ],
    );
}
