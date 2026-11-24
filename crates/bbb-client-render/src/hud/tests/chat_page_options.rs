use super::*;

fn glyph() -> HudAsciiGlyph {
    HudAsciiGlyph {
        width: 5,
        height: 8,
        advance: 6.0,
        bearing_top: 7.0,
        ..HudAsciiGlyph::default()
    }
}

fn fonts_for(text: &str) -> HudFontGlyphBook {
    let mut glyphs = HudFontGlyphMap::new();
    for ch in text.chars().chain(".a".chars()) {
        glyphs.insert_first_wins(ch, glyph());
    }
    HudFontGlyphBook::from(glyphs)
}

fn surface() -> PhysicalSize<u32> {
    PhysicalSize::new(800, 600)
}

fn focused(position: usize) -> HudChatContext {
    HudChatContext {
        focused: true,
        scroll: HudChatScroll {
            position,
            new_message_since_scroll: false,
        },
        ..HudChatContext::default()
    }
}

fn message(display_sequence: u64, text: &str, tag: Option<HudChatTag>) -> HudChatMessage {
    HudChatMessage {
        display_sequence,
        added_tick: 0,
        runs: vec![HudComponentTextRun::plain(text)],
        interactions: vec![HudChatInteraction {
            start_char: 0,
            end_char: 1,
            target_index: 1,
            clickable: true,
            insertion: false,
            tooltip: None,
        }],
        tag,
    }
}

fn modified_tag() -> HudChatTag {
    HudChatTag {
        kind: HudChatTagKind::Modified,
        indicator_color: 6_316_128,
        tooltip_runs: vec![HudStyledTextRun::plain("modified")],
    }
}

fn run_texts(prompt: &HudChatPrompt) -> Vec<&str> {
    prompt
        .runs
        .iter()
        .map(|run| run.visual.text.as_str())
        .collect()
}

#[test]
fn chat_options_sanitize_each_unit_double_to_its_initial_value() {
    let initial = HudChatOptions::default();
    assert_eq!(initial.height_unfocused, 0.4375);
    let invalid = HudChatOptions {
        scale: 1.5,
        width: -0.25,
        height_focused: f64::NAN,
        height_unfocused: 1.0001,
        line_spacing: f64::INFINITY,
        opacity: 2.0,
    };
    assert_eq!(invalid.sanitized(), initial);
    let bounds = HudChatOptions {
        scale: 0.0,
        width: 0.0,
        height_focused: 0.0,
        height_unfocused: 1.0,
        line_spacing: 1.0,
        opacity: 0.0,
    };
    assert_eq!(bounds.sanitized(), bounds);
    let geometry = HudChatGeometry::new(initial, surface());
    assert_eq!(
        (
            geometry.lines_per_page(true),
            geometry.lines_per_page(false)
        ),
        (20, 10)
    );
}

/// Expectations follow `ChatComponent, 413-437` and were checked
/// against the same expressions run on the 26.1 JDK (double `getWidth`/
/// `getHeight`, `(int)` line height, `Math.round` text offset, float opacity).
#[test]
fn chat_options_derive_the_page_from_vanilla_width_height_spacing_and_opacity() {
    struct Expected {
        wrap_width: i32,
        max_width: i32,
        chat_bottom: i32,
        entry_height: i32,
        text_top: i32,
        lines_per_page: (usize, usize),
        text_alpha: u32,
        queue_alpha: u32,
        drawn_rows: usize,
        restricted_text_top: i32,
    }
    for (options, expected) in [
        (
            HudChatOptions {
                scale: 1.0,
                width: 0.3,
                height_focused: 0.25,
                height_unfocused: 0.1,
                line_spacing: 0.5,
                opacity: 0.5,
            },
            Expected {
                wrap_width: 124,
                max_width: 124,
                chat_bottom: 560,
                entry_height: 13,
                text_top: 550,
                lines_per_page: (4, 2),
                text_alpha: 140,
                queue_alpha: 70,
                drawn_rows: 2,
                restricted_text_top: 523,
            },
        ),
        (
            HudChatOptions {
                scale: 0.75,
                width: 0.0,
                height_focused: 1.0,
                height_unfocused: 0.0,
                line_spacing: 0.125,
                opacity: 0.7,
            },
            Expected {
                wrap_width: 53,
                max_width: 54,
                chat_bottom: 746,
                entry_height: 10,
                text_top: 737,
                lines_per_page: (18, 2),
                text_alpha: 186,
                queue_alpha: 93,
                drawn_rows: 0,
                restricted_text_top: 736,
            },
        ),
        (
            HudChatOptions {
                scale: 1.0,
                width: 1.0,
                height_focused: 0.9,
                height_unfocused: 0.4375,
                line_spacing: 0.375,
                opacity: 0.0,
            },
            Expected {
                wrap_width: 320,
                max_width: 320,
                chat_bottom: 560,
                entry_height: 12,
                text_top: 550,
                lines_per_page: (13, 7),
                text_alpha: 25,
                queue_alpha: 12,
                drawn_rows: 1,
                restricted_text_top: 537,
            },
        ),
        (
            HudChatOptions {
                line_spacing: 1.0,
                ..HudChatOptions::default()
            },
            Expected {
                wrap_width: 320,
                max_width: 320,
                chat_bottom: 560,
                entry_height: 18,
                text_top: 548,
                lines_per_page: (10, 5),
                text_alpha: 255,
                queue_alpha: 127,
                drawn_rows: 0,
                restricted_text_top: 547,
            },
        ),
    ] {
        let geometry = HudChatGeometry::new(options, surface());
        let label = format!("{options:?}");
        assert_eq!(options.wrap_width(), expected.wrap_width, "{label}");
        assert_eq!(geometry.max_width, expected.max_width, "{label}");
        assert_eq!(geometry.chat_bottom, expected.chat_bottom, "{label}");
        assert_eq!(geometry.entry_height, expected.entry_height, "{label}");
        assert_eq!(geometry.entry_bottom(0), expected.chat_bottom, "{label}");
        assert_eq!(
            geometry.entry_top(0),
            expected.chat_bottom - expected.entry_height,
            "{label}"
        );
        assert_eq!(geometry.entry_bottom(1), geometry.entry_top(0), "{label}");
        assert_eq!(geometry.text_top(0), expected.text_top, "{label}");
        assert_eq!(
            geometry.text_top(1),
            expected.text_top - expected.entry_height,
            "{label}"
        );
        assert_eq!(
            (
                geometry.lines_per_page(true),
                geometry.lines_per_page(false)
            ),
            expected.lines_per_page,
            "{label}"
        );
        assert_eq!(
            hud_argb_alpha_channel(geometry.text_opacity),
            expected.text_alpha,
            "{label}"
        );
        assert_eq!(
            hud_argb_alpha_channel(0.5 * geometry.text_opacity),
            expected.queue_alpha,
            "{label}"
        );
        assert_eq!(
            geometry.restricted_text_top(expected.drawn_rows),
            expected.restricted_text_top,
            "{label}"
        );
        assert_eq!(
            geometry.queue_text_top(),
            expected.chat_bottom + 1,
            "{label}"
        );
    }
}

#[test]
fn chat_width_option_rewraps_messages_and_reserves_the_modified_icon() {
    let text = "a".repeat(106);
    let fonts = fonts_for("");
    let narrow = HudChatOptions {
        width: 0.3,
        ..HudChatOptions::default()
    };
    let line_lengths = |tag| {
        hud_wrap_chat_messages(&[message(1, &text, tag)], &fonts, narrow.wrap_width())
            .iter()
            .map(|line| {
                line.runs
                    .iter()
                    .map(|run| run.visual.text.len())
                    .sum::<usize>()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(line_lengths(None), vec![20, 20, 20, 20, 20, 6]);
    assert_eq!(
        line_lengths(Some(modified_tag())),
        vec![18, 18, 18, 18, 18, 16]
    );
}

#[test]
fn chat_page_rows_and_scroll_clamp_to_the_option_lines_per_page() {
    let options = HudChatOptions {
        width: 0.3,
        height_focused: 0.25,
        height_unfocused: 0.1,
        line_spacing: 0.5,
        ..HudChatOptions::default()
    };
    let geometry = HudChatGeometry::new(options, surface());
    let fonts = fonts_for("");
    let messages = (1..=10)
        .map(|display_sequence| message(display_sequence, "a", None))
        .collect::<Vec<_>>();
    let lines = hud_wrap_chat_messages(&messages, &fonts, options.wrap_width());
    let rows = |context: &HudChatContext| {
        geometry
            .visible_rows(&lines, context)
            .into_iter()
            .map(|(row, line_index, _)| (row, line_index))
            .collect::<Vec<_>>()
    };
    assert_eq!(rows(&focused(0)), [(3, 6), (2, 7), (1, 8), (0, 9)]);
    assert_eq!(rows(&focused(6)), [(3, 0), (2, 1), (1, 2), (0, 3)]);
    assert_eq!(rows(&HudChatContext::default()), [(1, 8), (0, 9)]);

    let page = geometry.scroll_page(lines.len());
    assert_eq!(
        page,
        HudChatScrollPage {
            lines: 10,
            lines_per_page: 4
        }
    );
    // `scrollChat` clamps at every step to `lines - linesPerPage`, then to 0.
    for (position, dir, expected) in [
        (0, 7, 6),
        (6, 1, 6),
        (5, 1, 6),
        (6, -7, 0),
        (2, 3, 5),
        (3, -3, 0),
        (4, i32::MAX, 0),
    ] {
        assert_eq!(page.scroll(position, dir), expected, "{position} {dir}");
    }
    let short = HudChatScrollPage {
        lines: 3,
        lines_per_page: 4,
    };
    assert_eq!(short.scroll(0, 7), 0);
    assert_eq!(short.scroll(2, -1), 0);
}

#[test]
fn focused_scrollbar_follows_the_vanilla_integer_thumb() {
    let options = HudChatOptions {
        height_focused: 0.25,
        line_spacing: 0.125,
        ..HudChatOptions::default()
    };
    let geometry = HudChatGeometry::new(options, surface());
    let thumb = |rect: [i32; 4], alpha: u32, rgb: u32| {
        Some([
            (rect, alpha << 24 | rgb),
            (
                [rect[0] + 1, rect[1], rect[2], rect[3]],
                alpha << 24 | 0xCC_CCCC,
            ),
        ])
    };
    let at = |position| HudChatScroll {
        position,
        new_message_since_scroll: false,
    };
    let (plain, new_message) = (0x33_33AA, 0xCC_3333);
    for (total, drawn_rows, scroll, expected) in [
        (10, 6, at(0), thumb([324, 524, 326, 560], 96, plain)),
        (10, 6, at(4), thumb([324, 500, 326, 536], 96, plain)),
        (7, 6, at(1), thumb([324, 501, 326, 552], 96, plain)),
        (6, 6, at(0), None),
        (0, 0, at(0), None),
        // ChatComponent: only the left column takes the new-message colour.
        (
            10,
            6,
            HudChatScroll {
                position: 4,
                new_message_since_scroll: true,
            },
            thumb([324, 500, 326, 536], 96, new_message),
        ),
    ] {
        assert_eq!(
            geometry.scrollbar(total, drawn_rows, scroll),
            expected,
            "{total} {drawn_rows} {scroll:?}"
        );
    }
    // `y > 0` only once the thumb offset passes the chat bottom.
    let short = HudChatGeometry::new(options, PhysicalSize::new(800, 41));
    assert_eq!(
        short.scrollbar(10, 6, at(4)),
        thumb([324, -59, 326, -23], 170, plain)
    );
    let half = HudChatGeometry::new(
        HudChatOptions {
            scale: 0.5,
            ..options
        },
        surface(),
    );
    assert_eq!(
        half.scrollbar(10, 6, at(0)),
        thumb([644, 1084, 646, 1120], 96, plain)
    );
}

#[test]
fn new_lines_scroll_a_scrolled_page_one_line_at_a_time() {
    let scroll = |position, new_message_since_scroll| HudChatScroll {
        position,
        new_message_since_scroll,
    };
    let appended = |prior_lines, message_lines: &[usize]| HudChatAppendedLines {
        prior_lines,
        message_lines: message_lines.to_vec(),
    };
    // ChatComponent: each line scrolls once, clamped against the
    // lines before it, and the list is trimmed to 100 after every message.
    for (from, lines, expected) in [
        (scroll(3, false), appended(10, &[2]), scroll(5, true)),
        (scroll(0, false), appended(10, &[2]), scroll(0, false)),
        // At the top clamp the first line keeps `10 - 4`, the second reaches `11 - 4`.
        (scroll(6, false), appended(10, &[2]), scroll(7, true)),
        (scroll(95, false), appended(99, &[3, 2]), scroll(97, true)),
        // A position past a page taller than the list, or a list rebuilt from
        // empty, lands on the bottom and clears the flag.
        (scroll(2, true), appended(3, &[1]), scroll(0, false)),
        (scroll(5, true), appended(0, &[1, 1]), scroll(0, false)),
    ] {
        assert_eq!(from.followed(4, &lines), expected, "{from:?} {lines:?}");
    }
    let page = HudChatScrollPage {
        lines: 20,
        lines_per_page: 4,
    };
    // ChatComponent: only reaching the bottom clears the flag.
    assert_eq!(scroll(5, true).scrolled(page, -2), scroll(3, true));
    assert_eq!(scroll(5, true).scrolled(page, -7), scroll(0, false));
    assert_eq!(scroll(5, true).scrolled(page, 20), scroll(16, true));
}

#[test]
fn replaced_messages_report_the_lines_vanilla_appends() {
    let messages = |sequences: std::ops::RangeInclusive<u64>| {
        sequences
            .map(|display_sequence| message(display_sequence, "a", None))
            .collect::<Vec<_>>()
    };
    let appended = |prior_lines, message_lines: &[usize]| {
        Some(HudChatAppendedLines {
            prior_lines,
            message_lines: message_lines.to_vec(),
        })
    };
    let replaced = |previous: &[HudChatMessage],
                    lines,
                    next: &[HudChatMessage],
                    counts: &[usize],
                    refreshed| {
        HudChatAppendedLines::replaced(previous, lines, next, counts, refreshed)
    };
    let (three, five) = (messages(1..=3), messages(1..=5));
    assert_eq!(
        replaced(&three, 4, &five, &[1, 2, 1, 2, 1], false),
        appended(4, &[2, 1])
    );
    assert_eq!(
        replaced(&[], 0, &three, &[1, 1, 1], false),
        appended(0, &[1, 1, 1])
    );
    assert_eq!(replaced(&three, 4, &three, &[1, 2, 1], false), None);
    assert_eq!(replaced(&three, 4, &[], &[], false), None);
    // A refresh re-adds every entry to an emptied list.
    assert_eq!(
        replaced(&three, 4, &three, &[1, 2, 1], true),
        appended(0, &[1, 2, 1])
    );
    // Entries cleared before the new one leave nothing to follow...
    assert_eq!(
        replaced(&three, 4, &messages(4..=4), &[2], false),
        appended(0, &[2])
    );
    // ...but entries dropped by the 100-entry history cap still had their lines.
    assert_eq!(
        replaced(
            &messages(1..=100),
            100,
            &messages(2..=101),
            &[1; 100],
            false
        ),
        appended(100, &[1])
    );

    let first = HudChatAppendedLines {
        prior_lines: 4,
        message_lines: vec![2],
    };
    assert_eq!(
        Some(first.clone().then(HudChatAppendedLines {
            prior_lines: 6,
            message_lines: vec![1],
        })),
        appended(4, &[2, 1])
    );
    assert_eq!(
        Some(first.then(HudChatAppendedLines {
            prior_lines: 0,
            message_lines: vec![3],
        })),
        appended(0, &[3])
    );
}

#[test]
fn modified_icon_shows_only_in_focused_chat_for_the_hovered_message_or_its_own_box() {
    let fonts = fonts_for("");
    let geometry = HudChatGeometry::new(HudChatOptions::default(), surface());
    let messages = [
        message(1, "a", None),
        message(2, &"a".repeat(60), Some(modified_tag())),
    ];
    let lines = hud_wrap_chat_messages(&messages, &fonts, HudChatOptions::default().wrap_width());
    assert_eq!(lines.len(), 3);
    let end_icon = (0, [64, 551, 73, 560]);
    let first_icon = (1, [304, 542, 313, 551]);
    let icons = |context: HudChatContext, pointer: Option<(i32, i32)>| {
        let context = HudChatContext {
            cursor_position: pointer,
            ..context
        };
        hud_chat_modified_icons(&lines, &context, &fonts, geometry)
    };
    for (pointer, expected) in [
        // Any glyph of either line of the modified message forces the
        // end-of-entry icon; the first line's own icon stays hidden.
        ((6, 547), vec![end_icon]),
        ((6, 556), vec![end_icon]),
        ((6, 538), vec![]),
        // Each line's hover box shows that line's icon, one pixel taller than
        // the sprite.
        ((310, 545), vec![first_icon]),
        ((70, 560), vec![end_icon]),
        ((70, 561), vec![]),
        ((200, 520), vec![]),
    ] {
        assert_eq!(icons(focused(0), Some(pointer)), expected, "{pointer:?}");
    }
    assert!(icons(focused(0), None).is_empty());
    assert!(icons(HudChatContext::default(), Some((6, 547))).is_empty());
    // Scrolled past the end-of-entry line, the hovered message has no drawn
    // end line to force, and the first line's box moves down one row.
    assert!(icons(focused(1), Some((6, 556))).is_empty());
    assert_eq!(
        icons(focused(1), Some((310, 555))),
        vec![(0, [304, 551, 313, 560])]
    );
    let tag = hud_chat_tag_at(&lines, &focused(0), geometry, (310.0, 545.0), &fonts);
    assert_eq!(tag.map(|tag| tag.kind), Some(HudChatTagKind::Modified));
    assert!(hud_chat_tag_at(
        &lines,
        &HudChatContext::default(),
        geometry,
        (310.0, 545.0),
        &fonts
    )
    .is_none());
}

#[test]
fn restriction_prompt_clips_with_a_plain_ellipsis_and_hits_only_its_glyphs() {
    let text = "Chat is restricted";
    let fonts = fonts_for(text);
    let full = hud_chat_restricted_prompt(text, &fonts, 320);
    assert!(!full.clipped);
    assert_eq!(run_texts(&full), [text]);
    assert_eq!(full.runs[0].visual.color, Some(0xFF_5555));
    assert!(full.runs[0].visual.style.underlined);

    // `substrByWidth(40 - width("..."))` keeps glyphs while the budget stays
    // non-negative.
    let clipped = hud_chat_restricted_prompt(text, &fonts, 40);
    assert!(clipped.clipped);
    assert_eq!(run_texts(&clipped), ["Cha", "..."]);
    assert_eq!(clipped.runs[0].visual.color, Some(0xFF_5555));
    assert!(clipped.runs[0].visual.style.underlined);
    assert_eq!(clipped.runs[1].visual.color, None);
    assert!(!clipped.runs[1].visual.style.underlined);
    assert_eq!(
        run_texts(&hud_chat_restricted_prompt("aaaaaa", &fonts, 36)),
        ["aaaaaa"]
    );
    assert_eq!(
        run_texts(&hud_chat_restricted_prompt("aaaaaa", &fonts, 35)),
        ["aa", "..."]
    );
    assert_eq!(
        run_texts(&hud_chat_restricted_prompt("aaaaaa", &fonts, 10)),
        ["..."]
    );

    let narrow = HudChatOptions {
        width: 0.0,
        ..HudChatOptions::default()
    };
    let restricted = |scroll| HudChatContext {
        restricted_text: Some(text.to_string()),
        ..focused(scroll)
    };
    let hit = |lines: &[HudChatLine], options, pointer: (i32, i32)| {
        hud_chat_hit_target_at(
            lines,
            &restricted(0),
            &fonts,
            HudChatGeometry::new(options, surface()),
            (f64::from(pointer.0), f64::from(pointer.1)),
        )
        .map(|hit| hit.kind)
    };
    let restrictions = Some(HudChatHitTargetKind::Restrictions);
    // The prompt text top is `lineAboveMessagesY`, 551 with no rows.
    assert_eq!(hit(&[], narrow, (6, 555)), restrictions);
    assert_eq!(hit(&[], narrow, (21, 555)), restrictions);
    assert_eq!(hit(&[], narrow, (23, 555)), None);
    assert_eq!(hit(&[], narrow, (60, 555)), None);
    assert_eq!(hit(&[], HudChatOptions::default(), (60, 555)), restrictions);

    // Line spacing moves the prompt text below `lineAboveMessagesY`:
    // 524 + 18 - 12 - 1 over one 18-pixel row.
    let spaced = HudChatOptions {
        line_spacing: 1.0,
        ..narrow
    };
    let lines = hud_wrap_chat_messages(&[message(9, "a", None)], &fonts, spaced.wrap_width());
    assert_eq!(hit(&lines, spaced, (6, 533)), restrictions);
    assert_eq!(hit(&lines, spaced, (6, 527)), None);
    assert_eq!(
        hit(&lines, spaced, (6, 552)),
        Some(HudChatHitTargetKind::Message {
            display_sequence: 9,
            target_index: 1,
            click_target_index: Some(1),
        })
    );
}

#[test]
fn startup_chat_page_options_draw_rows_opacity_scrollbar_and_hovered_icon() {
    let Some(mut renderer) = Renderer::new_offscreen(400, 240) else {
        return;
    };
    renderer.set_clear_color(crate::ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    let white = HudAsciiGlyph {
        uv: HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        ..glyph()
    };
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins('a', white);
    renderer
        .upload_hud_font_atlas(1, 1, &[255; 4], HudFontGlyphBook::from(glyphs))
        .expect("font atlas");
    renderer
        .add_test_gui_sprite("icon/chat_modified", 1, 1, &[255, 0, 0, 255])
        .expect("modified icon");
    renderer.set_hud_chat_options(HudChatOptions {
        width: 0.3,
        height_focused: 0.25,
        line_spacing: 0.5,
        opacity: 0.5,
        ..HudChatOptions::default()
    });
    renderer.set_hud_chat_messages(
        (1..=6)
            .map(|display_sequence| {
                message(
                    display_sequence,
                    "a",
                    (display_sequence == 6).then(modified_tag),
                )
            })
            .collect(),
    );
    assert_eq!(
        renderer.hud_chat_scroll_page(),
        HudChatScrollPage {
            lines: 6,
            lines_per_page: 4
        }
    );
    let frame = |renderer: &mut Renderer, context: HudChatContext| {
        renderer.set_hud_chat_context(context);
        renderer.render_offscreen_frame().expect("chat frame")
    };
    let clear = [0, 0, 255, 255];
    let red = |pixel: [u8; 4]| pixel[0];

    // Four 13-pixel rows over a 200-pixel chat bottom, 136 pixels wide; row 1
    // text starts at (4, 177). The thumb spans local y 166..200 at x 132..134.
    let page = frame(&mut renderer, focused(0));
    assert_ne!(page.pixel(130, 150), clear);
    assert_eq!(page.pixel(137, 195), clear);
    assert_eq!(page.pixel(130, 147), clear);
    assert!(
        (130..=150).contains(&red(page.pixel(4, 177))),
        "{:?}",
        page.pixel(4, 177)
    );
    assert_eq!(red(page.pixel(132, 155)), 0);
    assert!(
        (10..=30).contains(&red(page.pixel(132, 195))),
        "{:?}",
        page.pixel(132, 195)
    );
    assert!(
        (75..=100).contains(&red(page.pixel(133, 195))),
        "{:?}",
        page.pixel(133, 195)
    );
    assert!(red(page.pixel(16, 193)) < 30);

    let hovered = frame(
        &mut renderer,
        HudChatContext {
            cursor_position: Some((5, 194)),
            ..focused(0)
        },
    );
    assert!(
        red(hovered.pixel(16, 193)) > 230,
        "{:?}",
        hovered.pixel(16, 193)
    );
    assert_eq!(
        renderer
            .hud_chat_hit_target_at((5.0, 194.0))
            .map(|hit| hit.kind),
        Some(HudChatHitTargetKind::Message {
            display_sequence: 6,
            target_index: 1,
            click_target_index: Some(1),
        })
    );

    let scrolled = frame(&mut renderer, focused(2));
    assert_eq!(red(scrolled.pixel(132, 195)), 0);
    assert!((10..=30).contains(&red(scrolled.pixel(132, 170))));
    let new_message = frame(
        &mut renderer,
        HudChatContext {
            scroll: HudChatScroll {
                position: 2,
                new_message_since_scroll: true,
            },
            ..focused(2)
        },
    );
    assert!(
        (65..=90).contains(&red(new_message.pixel(132, 170))),
        "{:?}",
        new_message.pixel(132, 170)
    );

    // The queue line's two glyphs sit at local y 201; hovering one of them
    // defers the queue tooltip at the pointer, the rest of its fill does not.
    let queued = |pointer| HudChatContext {
        cursor_position: Some(pointer),
        queued_messages: 1,
        queue_text: Some("aa".to_string()),
        queue_tooltip_text: Some("aaa".to_string()),
        ..focused(0)
    };
    let off_glyphs = frame(&mut renderer, queued((40, 205)));
    let on_glyph = frame(&mut renderer, queued((5, 205)));
    assert_eq!(
        renderer
            .hud_chat_hit_target_at((5.0, 205.0))
            .map(|hit| hit.kind),
        Some(HudChatHitTargetKind::Queue)
    );
    assert_ne!(on_glyph.pixel(30, 196), off_glyphs.pixel(30, 196));
    assert_eq!(on_glyph.pixel(60, 196), off_glyphs.pixel(60, 196));

    // Unfocused: six rows of the 90-pixel page, no scrollbar and no icon.
    let background = frame(
        &mut renderer,
        HudChatContext {
            cursor_position: Some((5, 194)),
            ..HudChatContext::default()
        },
    );
    assert_ne!(background.pixel(120, 130), clear);
    assert_eq!(page.pixel(120, 130), clear);
    assert_eq!(red(background.pixel(132, 195)), 0);
    assert!(red(background.pixel(16, 193)) < 30);

    // The first replacement filled an empty list; the next appends one line
    // after the six drawn above.
    let report = |prior_lines, message_lines: Vec<usize>| {
        Some(HudChatAppendedLines {
            prior_lines,
            message_lines,
        })
    };
    assert_eq!(
        renderer.take_hud_chat_appended_lines(),
        report(0, vec![1; 6])
    );
    assert_eq!(renderer.take_hud_chat_appended_lines(), None);
    renderer.set_hud_chat_messages(
        (1..=7)
            .map(|display_sequence| message(display_sequence, "a", None))
            .collect(),
    );
    assert_eq!(renderer.take_hud_chat_appended_lines(), report(6, vec![1]));
    renderer.refresh_hud_chat_messages(
        (1..=7)
            .map(|display_sequence| message(display_sequence, "a", None))
            .collect(),
    );
    assert_eq!(
        renderer.take_hud_chat_appended_lines(),
        report(0, vec![1; 7])
    );
}
