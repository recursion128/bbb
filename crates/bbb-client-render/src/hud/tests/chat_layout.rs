use super::*;

fn glyphs_for(text: &str) -> HudFontGlyphMap {
    let mut glyphs = HudFontGlyphMap::new();
    let glyph = HudAsciiGlyph {
        width: 5,
        height: 8,
        advance: 6.0,
        ..HudAsciiGlyph::default()
    };
    for ch in text.chars().chain(std::iter::once('?')) {
        glyphs.insert_first_wins(ch, glyph);
    }
    glyphs
}

fn glyph(advance: f32, width: u32, height: u32, bearing_top: f32) -> HudAsciiGlyph {
    HudAsciiGlyph {
        width,
        height,
        advance,
        bearing_top,
        ..HudAsciiGlyph::default()
    }
}

fn chat_fonts(specs: &[(char, HudAsciiGlyph)]) -> HudFontGlyphBook {
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, spec) in specs {
        glyphs.insert_first_wins(*ch, *spec);
    }
    HudFontGlyphBook::from(glyphs)
}

fn chat_interaction(
    start_char: usize,
    end_char: usize,
    target_index: usize,
    clickable: bool,
    insertion: bool,
) -> HudChatInteraction {
    HudChatInteraction {
        start_char,
        end_char,
        target_index,
        clickable,
        insertion,
        tooltip: None,
    }
}

fn chat_lines_from(
    runs: Vec<HudComponentTextRun>,
    interactions: Vec<HudChatInteraction>,
    fonts: &HudFontGlyphBook,
) -> Vec<HudChatLine> {
    let message = HudChatMessage {
        display_sequence: 7,
        added_tick: 0,
        runs,
        interactions,
        tag: None,
    };
    hud_wrap_chat_messages(&[message], fonts, HudChatOptions::default().wrap_width())
}

fn scaled(scale: f64) -> HudChatOptions {
    HudChatOptions {
        scale,
        ..HudChatOptions::default()
    }
}

fn default_chat_geometry() -> HudChatGeometry {
    HudChatGeometry::new(HudChatOptions::default(), PhysicalSize::new(800, 600))
}

fn chat_context(scroll_lines: usize) -> HudChatContext {
    HudChatContext {
        level_generation: 11,
        delivery_revision: 13,
        chat_screen_revision: 19,
        resource_generation: 17,
        resources_loaded: true,
        focused: true,
        scroll: HudChatScroll {
            position: scroll_lines,
            new_message_since_scroll: false,
        },
        ..HudChatContext::default()
    }
}

/// One click at chat line `row` (0 is the bottom line), offset into its
/// text origin, over an 800x600 surface.
fn chat_click(
    lines: &[HudChatLine],
    fonts: &HudFontGlyphBook,
    scroll_lines: usize,
    row: i32,
    local_x: i32,
    local_y: i32,
) -> Option<HudChatHitTargetKind> {
    let origin_y = 560 - row * HUD_CHAT_MESSAGE_HEIGHT - 8;
    hud_chat_hit_target_at(
        lines,
        &chat_context(scroll_lines),
        fonts,
        default_chat_geometry(),
        (f64::from(4 + local_x), f64::from(origin_y + local_y)),
    )
    .map(|hit| hit.kind)
}

fn message_target(kind: Option<HudChatHitTargetKind>) -> Option<(usize, Option<usize>)> {
    match kind {
        Some(HudChatHitTargetKind::Message {
            target_index,
            click_target_index,
            ..
        }) => Some((target_index, click_target_index)),
        _ => None,
    }
}

#[test]
fn rich_chat_wrap_preserves_run_style_and_word_boundary() {
    let glyphs = glyphs_for("alpha beta");
    let bold = HudTextStyle {
        bold: true,
        ..HudTextStyle::default()
    };
    let lines = hud_wrap_chat_runs(
        &[
            HudStyledTextRun::plain("alpha "),
            HudStyledTextRun {
                shadow_color: None,
                text: "beta".to_string(),
                style: bold,
                color: Some(0x55ffff),
            },
        ],
        36,
        &glyphs,
    );
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], vec![HudStyledTextRun::plain("alpha")]);
    assert_eq!(lines[1][0].text, "beta");
    assert_eq!(lines[1][0].style, bold);
    assert_eq!(lines[1][0].color, Some(0x55ffff));
}

#[test]
fn modified_tag_reserves_icon_width_before_wrapping() {
    let text = "a".repeat(52);
    let glyphs = glyphs_for(&text);
    let plain = HudChatMessage {
        display_sequence: 1,
        added_tick: 0,
        runs: vec![HudComponentTextRun::plain(&text)],
        interactions: Vec::new(),
        tag: None,
    };
    let modified = HudChatMessage {
        tag: Some(HudChatTag {
            kind: HudChatTagKind::Modified,
            indicator_color: 6_316_128,
            tooltip_runs: Vec::new(),
        }),
        ..plain.clone()
    };
    let fonts = HudFontGlyphBook::from(glyphs);
    let width = HudChatOptions::default().wrap_width();
    assert_eq!(hud_wrap_chat_messages(&[plain], &fonts, width).len(), 1);
    assert_eq!(hud_wrap_chat_messages(&[modified], &fonts, width).len(), 2);
}

#[test]
fn chat_alpha_matches_vanilla_focus_and_two_hundred_tick_fade() {
    let line = HudChatLine {
        display_sequence: 1,
        added_tick: 10,
        runs: vec![HudComponentTextRun::plain("hello")],
        cached_geometry: None,
        tag: None,
        targets: Vec::new(),
        interactions: Vec::new(),
        end_of_entry: true,
    };
    let mut context = HudChatContext {
        gui_ticks: 200,
        ..HudChatContext::default()
    };
    assert!((hud_chat_line_alpha(&context, &line) - 0.25).abs() < 1.0e-6);
    context.gui_ticks = 210;
    assert_eq!(hud_chat_line_alpha(&context, &line), 0.0);
    context.focused = true;
    assert_eq!(hud_chat_line_alpha(&context, &line), 1.0);
}

#[test]
fn stable_chat_geometry_caches_plain_runs_and_bypasses_obfuscated_runs() {
    let fonts = HudFontGlyphBook::from(glyphs_for("hello"));
    let plain = [HudComponentTextRun::plain("hello")];
    let cached = hud_chat_line_geometry(&plain, &[], &fonts).expect("plain cache");
    assert_eq!(cached.geometry.glyph_quads.len(), 10);
    assert_eq!(cached.geometry.glyph_areas.len(), 5);
    assert_eq!(cached.geometry.text_bounds, Some([0.0, 0.0, 30.0, 9.0]));
    assert!(cached
        .geometry
        .glyph_quads
        .as_chunks::<2>()
        .0
        .iter()
        .all(|quads| quads[0].0.shadow && !quads[1].0.shadow));

    let obfuscated = [HudComponentTextRun {
        visual: HudStyledTextRun {
            shadow_color: None,
            text: "hello".to_string(),
            style: HudTextStyle {
                obfuscated: true,
                ..HudTextStyle::default()
            },
            color: None,
        },
        font: None,
    }];
    assert!(hud_chat_line_geometry(&obfuscated, &[], &fonts).is_none());
}

#[test]
fn chat_interaction_follows_wrapped_glyph_spans() {
    let text = format!("{} beta", "a".repeat(52));
    let glyphs = glyphs_for(&text);
    let message = HudChatMessage {
        display_sequence: 7,
        added_tick: 0,
        runs: vec![HudComponentTextRun::plain(&text)],
        interactions: vec![HudChatInteraction {
            start_char: 53,
            end_char: 57,
            target_index: 3,
            clickable: true,
            insertion: false,
            tooltip: None,
        }],
        tag: None,
    };
    let fonts = HudFontGlyphBook::from(glyphs);
    let lines = hud_wrap_chat_messages(&[message], &fonts, HudChatOptions::default().wrap_width());
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1].interactions.len(), 1);
    assert_eq!(lines[1].interactions[0].target_index, 3);
    // Only the four characters of the interaction carry a target.
    assert_eq!(
        lines[1]
            .targets
            .iter()
            .filter(|target| target.is_some())
            .count(),
        4
    );

    let context = HudChatContext {
        level_generation: 11,
        delivery_revision: 13,
        chat_screen_revision: 19,
        resource_generation: 17,
        resources_loaded: true,
        focused: true,
        ..HudChatContext::default()
    };
    let hit = hud_chat_hit_target_at(
        &lines,
        &context,
        &fonts,
        default_chat_geometry(),
        (5.0, 552.0),
    );
    assert_eq!(
        hit,
        Some(HudChatHitTarget {
            level_generation: 11,
            delivery_revision: 13,
            chat_screen_revision: 19,
            resource_generation: 17,
            resources_loaded: true,
            kind: HudChatHitTargetKind::Message {
                display_sequence: 7,
                target_index: 3,
                click_target_index: Some(3),
            },
        })
    );
}

#[test]
fn chat_interactions_follow_visual_rtl_glyph_spans_after_logical_wrap() {
    let mut glyphs = glyphs_for("אב");
    glyphs.set_default_right_to_left(true);
    let lines = hud_wrap_chat_runs_with_interactions(
        &[HudStyledTextRun::plain("אב")],
        &[
            HudChatInteraction {
                start_char: 0,
                end_char: 1,
                target_index: 1,
                clickable: true,
                insertion: false,
                tooltip: None,
            },
            HudChatInteraction {
                start_char: 1,
                end_char: 2,
                target_index: 2,
                clickable: true,
                insertion: false,
                tooltip: None,
            },
        ],
        100,
        &glyphs,
    );

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].runs, vec![HudStyledTextRun::plain("אב")]);
    // Visual order puts the second logical character first; the glyph areas
    // follow that order while each target keeps its own advance span.
    let geometry = hud_component_text_hit_geometry(
        &[HudComponentTextRun::from(HudStyledTextRun::plain("אב"))],
        &lines[0].targets,
        &HudFontGlyphBook::from(glyphs_for("אב")),
        true,
    );
    assert_eq!(
        geometry
            .glyph_areas
            .iter()
            .map(|(area, target)| (area.left, area.right, *target))
            .collect::<Vec<_>>(),
        vec![(0.0, 6.0, Some(2)), (6.0, 12.0, Some(1))]
    );
}

#[test]
fn hud_chat_custom_font_controls_wrapping_and_interaction_width() {
    let default = glyphs_for("a");
    let mut compact = HudFontGlyphMap::new();
    compact.insert_first_wins(
        'a',
        HudAsciiGlyph {
            width: 1,
            height: 8,
            advance: 2.0,
            ..HudAsciiGlyph::default()
        },
    );
    let fonts = HudFontGlyphBook::from_fonts(
        "minecraft:default",
        BTreeMap::from([
            ("example:compact".to_string(), compact),
            ("minecraft:default".to_string(), default),
        ]),
    );
    let text = "a".repeat(10);
    let lines = hud_wrap_component_runs_with_interactions(
        &[HudComponentTextRun {
            visual: HudStyledTextRun::plain(&text),
            font: Some("example:compact".to_string()),
        }],
        &[HudChatInteraction {
            start_char: 0,
            end_char: text.chars().count(),
            target_index: 7,
            clickable: true,
            insertion: false,
            tooltip: None,
        }],
        20,
        &fonts,
    );

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].runs[0].font.as_deref(), Some("example:compact"));
    assert_eq!(lines[0].interactions[0].target_index, 7);
    let geometry = hud_component_text_hit_geometry(&lines[0].runs, &lines[0].targets, &fonts, true);
    assert_eq!(geometry.glyph_areas.len(), 10);
    assert_eq!(geometry.glyph_areas[9].0.right, 20.0);
}

#[test]
fn hud_chat_accumulates_unihex_half_pixel_bold_advance_before_rounding() {
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(
        '中',
        HudAsciiGlyph {
            width: 16,
            height: 16,
            advance: 9.0,
            render_scale: 0.5,
            bold_offset: 0.5,
            shadow_offset: 0.5,
            ..HudAsciiGlyph::default()
        },
    );
    let runs = [HudComponentTextRun {
        visual: HudStyledTextRun {
            shadow_color: None,
            text: "中中".to_string(),
            style: HudTextStyle {
                bold: true,
                ..HudTextStyle::default()
            },
            color: None,
        },
        font: None,
    }];
    let fonts = HudFontGlyphBook::from(glyphs);
    let lines = hud_wrap_component_runs_with_interactions(
        &runs,
        &[HudChatInteraction {
            start_char: 0,
            end_char: 2,
            target_index: 9,
            clickable: true,
            insertion: false,
            tooltip: None,
        }],
        19,
        &fonts,
    );

    assert_eq!(lines.len(), 1);
    // Advances stay fractional in the accepted areas: the second glyph
    // starts at 9.5 and accepts no click at 19, and bold widens each box by
    // its extra thickness on the left. Only the prepared bounds round.
    let geometry = hud_component_text_hit_geometry(&lines[0].runs, &lines[0].targets, &fonts, true);
    assert_eq!(
        geometry
            .glyph_areas
            .iter()
            .map(|(area, target)| (area.left, area.right, *target))
            .collect::<Vec<_>>(),
        vec![(-0.1, 9.5, Some(9)), (9.4, 19.0, Some(9))]
    );
    assert_eq!(geometry.text_bounds, Some([-0.1, -0.1, 18.1, 8.6]));
}

#[test]
fn hud_chat_unavailable_font_falls_back_to_default_glyphs() {
    let fonts = HudFontGlyphBook::from(glyphs_for("a"));
    let text = "a".repeat(10);
    let lines = hud_wrap_component_runs_with_interactions(
        &[HudComponentTextRun {
            visual: HudStyledTextRun::plain(&text),
            font: Some("example:missing".to_string()),
        }],
        &[HudChatInteraction {
            start_char: 0,
            end_char: text.chars().count(),
            target_index: 3,
            clickable: true,
            insertion: false,
            tooltip: None,
        }],
        20,
        &fonts,
    );

    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0].runs[0].visual.text, "aaa");
    assert_eq!(lines[0].interactions[0].target_index, 3);
    assert_eq!(lines[0].targets, vec![Some(3); 3]);
}

#[test]
fn hud_chat_space_provider_glyph_advances_without_a_bitmap() {
    let mut default = glyphs_for("?");
    // `EmptyGlyph`: no bitmap at all, unlike a zero-width bitmap glyph.
    default.insert_first_wins('\u{200c}', HudAsciiGlyph::empty(9.0));
    let fonts = HudFontGlyphBook::from(default);
    let lines = hud_wrap_component_runs_with_interactions(
        &[HudComponentTextRun::plain("\u{200c}")],
        &[HudChatInteraction {
            start_char: 0,
            end_char: 1,
            target_index: 5,
            clickable: true,
            insertion: false,
            tooltip: None,
        }],
        20,
        &fonts,
    );

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].interactions[0].target_index, 5);
    // A bitmap-less glyph is an `EmptyArea`: it accepts clicks but never
    // contributes prepared bounds.
    let geometry = hud_component_text_hit_geometry(&lines[0].runs, &lines[0].targets, &fonts, true);
    assert!(geometry.glyph_areas.is_empty());
    assert_eq!(geometry.empty_areas.len(), 1);
    assert_eq!(geometry.empty_areas[0].0.left, 0.0);
    assert_eq!(geometry.empty_areas[0].0.right, 9.0);
    assert_eq!(geometry.empty_areas[0].0.bottom, 9.0);
    assert_eq!(geometry.text_bounds, None);
}

#[test]
fn hud_chat_entity_hover_lines_require_advanced_tooltips() {
    let tooltip = HudRichTooltip {
        style: None,
        lines: Vec::new(),
        advanced_lines: Some(vec![HudRichTooltipLine::plain("Type: minecraft:pig")]),
        wrap_text: false,
    };

    assert!(hud_rich_tooltip_lines(&tooltip, false).is_empty());
    assert_eq!(
        hud_rich_tooltip_lines(&tooltip, true)[0].runs[0]
            .visual
            .text,
        "Type: minecraft:pig"
    );
}

#[test]
fn tooltip_line_legacy_codes_style_runs_width_and_drawn_glyphs() {
    let fonts = chat_fonts(&[
        ('A', glyph(6.0, 5, 8, 0.0)),
        ('B', glyph(6.0, 5, 8, 0.0)),
        ('C', glyph(6.0, 5, 8, 0.0)),
        ('D', glyph(6.0, 5, 8, 0.0)),
        // An unresolved line would draw these as glyphs; the resolved one must not.
        ('§', glyph(6.0, 5, 8, 0.0)),
        ('a', glyph(6.0, 5, 8, 0.0)),
        ('r', glyph(6.0, 5, 8, 0.0)),
        ('?', glyph(6.0, 5, 8, 0.0)),
    ]);
    // The shape the item and hover tooltip projections hand the renderer: one
    // run per Component part, codes still in the text.
    let source = [HudComponentTextRun::plain("§aAB§rC§?D§")];

    let resolved = hud_resolved_tooltip_runs(&source);
    assert_eq!(
        resolved
            .iter()
            .map(|run| run.visual.text.as_str())
            .collect::<String>(),
        "ABCD"
    );
    // `§a` colours both glyphs; `§r` returns to the part's style.
    assert_eq!(
        resolved
            .iter()
            .map(|run| run.visual.color)
            .collect::<Vec<_>>(),
        [Some(0x55_ff_55), None]
    );

    // The tooltip width and the glyph pass read the resolved line: four glyphs
    // at six pixels, none of them a code character.
    assert_eq!(hud_component_text_width(&resolved, &fonts), 24);
    let geometry = hud_component_text_geometry(
        &resolved,
        &fonts,
        0,
        (0.0, 0.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    assert_eq!(geometry.glyph_quads.len(), 4);
    // Draw order puts the fourth glyph of the resolved line on pen 18.
    let last = geometry.glyph_quads.last().unwrap().0;
    assert_eq!(
        last.corners
            .iter()
            .map(|corner| corner[0])
            .fold(f32::MAX, f32::min),
        18.0
    );
    assert_eq!(
        last.corners
            .iter()
            .map(|corner| corner[0])
            .fold(f32::MIN, f32::max),
        23.0
    );
}

#[test]
fn queue_hit_is_focused_revision_bound_and_limited_to_its_glyphs() {
    let text = "[+1]";
    let fonts = chat_fonts(
        &text
            .chars()
            .map(|ch| (ch, glyph(6.0, 5, 8, 7.0)))
            .collect::<Vec<_>>(),
    );
    let context = HudChatContext {
        level_generation: 2,
        delivery_revision: 5,
        focused: true,
        queued_messages: 1,
        queue_text: Some(text.to_string()),
        ..HudChatContext::default()
    };
    let hit = |context: &HudChatContext, x: f64, y: f64| {
        hud_chat_hit_target_at(&[], context, &fonts, default_chat_geometry(), (x, y))
            .map(|hit| hit.kind)
    };
    // The queue text sits at `chatBottom + 1`; its four glyphs span local x 0..24.
    assert_eq!(hit(&context, 5.0, 565.0), Some(HudChatHitTargetKind::Queue));
    assert_eq!(
        hit(&context, 27.0, 565.0),
        Some(HudChatHitTargetKind::Queue)
    );
    // The queue fill runs to `maxWidth + 4`, but `captureClickableText` only
    // tests the accepted text.
    assert_eq!(hit(&context, 29.0, 565.0), None);
    assert_eq!(hit(&context, 300.0, 565.0), None);
    let without_text = HudChatContext {
        queue_text: None,
        ..context.clone()
    };
    assert_eq!(hit(&without_text, 5.0, 565.0), None);
    let unfocused = HudChatContext {
        focused: false,
        ..context
    };
    assert_eq!(hit(&unfocused, 5.0, 565.0), None);
}

#[test]
fn queue_tooltip_shows_over_the_queue_glyphs_of_focused_chat() {
    let text = "[+1]";
    let fonts = chat_fonts(
        &text
            .chars()
            .map(|ch| (ch, glyph(6.0, 5, 8, 7.0)))
            .collect::<Vec<_>>(),
    );
    let geometry = default_chat_geometry();
    let queue = hud_chat_queue_prompt(text, &fonts);
    let context = |pointer| HudChatContext {
        focused: true,
        cursor_position: Some(pointer),
        queued_messages: 1,
        queue_text: Some(text.to_string()),
        queue_tooltip_text: Some("Click to display next message".to_string()),
        ..HudChatContext::default()
    };
    let tooltip_text = |context: &HudChatContext| {
        hud_chat_prompt_tooltip(context, geometry, Some(&queue), None, 0).map(|tooltip| {
            assert!(tooltip.wrap_text);
            assert_eq!(tooltip.lines.len(), 1);
            assert_eq!(tooltip.lines[0].tint, HUD_TINT_WHITE);
            tooltip.lines[0]
                .runs
                .iter()
                .map(|run| run.visual.text.as_str())
                .collect::<String>()
        })
    };
    // ChatComponent: the queue style's `ShowText` hover, found by the
    // same glyph test as the expand click.
    let shown = Some("Click to display next message".to_string());
    assert_eq!(tooltip_text(&context((5, 565))), shown);
    assert_eq!(tooltip_text(&context((27, 565))), shown);
    assert_eq!(tooltip_text(&context((29, 565))), None);
    assert_eq!(tooltip_text(&context((5, 555))), None);
    assert_eq!(
        tooltip_text(&HudChatContext {
            focused: false,
            ..context((5, 565))
        }),
        None
    );
    assert_eq!(
        tooltip_text(&HudChatContext {
            cursor_position: None,
            ..context((5, 565))
        }),
        None
    );
    assert_eq!(
        tooltip_text(&HudChatContext {
            queue_tooltip_text: None,
            ..context((5, 565))
        }),
        None
    );
    assert_eq!(
        hud_chat_prompt_tooltip(&context((5, 565)), geometry, None, None, 0),
        None
    );
}

#[test]
fn restricted_chat_prompt_and_done_button_are_revision_bound_targets() {
    let text = "Chat is restricted";
    let fonts = HudFontGlyphBook::from(glyphs_for(text));
    let context = HudChatContext {
        level_generation: 2,
        delivery_revision: 5,
        chat_screen_revision: 7,
        focused: true,
        restricted_text: Some(text.to_string()),
        ..HudChatContext::default()
    };
    let prompt =
        hud_chat_hit_target_at(&[], &context, &fonts, default_chat_geometry(), (5.0, 552.0))
            .expect("restricted prompt");
    assert_eq!(prompt.kind, HudChatHitTargetKind::Restrictions);
    assert_eq!(prompt.chat_screen_revision, 7);

    let screen = sanitize_hud_chat_restrictions_screen(HudChatRestrictionsScreen {
        generation: 11,
        title: "Restrictions".to_string(),
        reason: HudStyledTextRun::plain("Client settings restriction"),
        permissions: (0..4)
            .map(|index| HudStyledTextRun::plain(format!("permission {index}")))
            .collect(),
        done: "Done".to_string(),
        cursor_position: None,
    })
    .expect("bounded restriction screen");
    assert_eq!(screen.permissions.len(), 4);
    assert!(hud_chat_restrictions_done_contains(
        PhysicalSize::new(800, 600),
        (400.0, 580.0),
    ));
}

#[test]
fn chat_click_uses_glyph_areas_and_rejects_gaps_and_blank_lines() {
    let fonts = chat_fonts(&[
        ('a', glyph(6.0, 5, 8, 7.0)),
        (' ', HudAsciiGlyph::empty(4.0)),
    ]);
    // "a a": the space itself carries no interaction.
    let lines = chat_lines_from(
        vec![HudComponentTextRun::plain("a a")],
        vec![
            chat_interaction(0, 1, 1, true, false),
            chat_interaction(2, 3, 3, true, false),
        ],
        &fonts,
    );
    assert_eq!(lines.len(), 1);
    // Inside the first glyph, well within its drawn box.
    assert_eq!(
        message_target(chat_click(&lines, &fonts, 0, 0, 1, 4)),
        Some((1, Some(1)))
    );
    // The bitmap-less space accepts clicks as an `EmptyArea`, but this one
    // is an interaction gap: the style under the cursor is not targetable.
    assert_eq!(message_target(chat_click(&lines, &fonts, 0, 0, 7, 4)), None);
    // The second glyph starts at the bold-aware pen 10.
    assert_eq!(
        message_target(chat_click(&lines, &fonts, 0, 0, 11, 4)),
        Some((3, Some(3)))
    );
    assert_eq!(message_target(chat_click(&lines, &fonts, 0, 0, 9, 4)), None);
    // Prepared bounds are two-dimensional: the row pitch is nine pixels but
    // the ink is not, so the row padding rejects the click.
    assert_eq!(
        message_target(chat_click(&lines, &fonts, 0, 0, 1, -1)),
        None
    );
    assert_eq!(message_target(chat_click(&lines, &fonts, 0, 0, 1, 9)), None);
    assert_eq!(
        message_target(chat_click(&lines, &fonts, 0, 0, 100, 4)),
        None
    );

    // A line whose glyphs all come from `EmptyGlyph` prepares no bounds at
    // all, so no click anywhere in the row can target it.
    let blank = chat_lines_from(
        vec![HudComponentTextRun::plain("  ")],
        vec![chat_interaction(0, 2, 4, true, false)],
        &fonts,
    );
    for local_x in 0..12 {
        for local_y in -2..11 {
            assert_eq!(
                message_target(chat_click(&blank, &fonts, 0, 0, local_x, local_y)),
                None,
                "blank line accepted a click at ({local_x}, {local_y})"
            );
        }
    }
}

#[test]
fn chat_click_keeps_fractional_pens_and_ignores_the_rounded_interval() {
    let fonts = chat_fonts(&[
        ('w', glyph(8.0, 8, 8, 7.0)),
        ('n', glyph(-2.0, 4, 8, 7.0)),
        ('a', glyph(2.5, 4, 8, 7.0)),
    ]);
    // "wna": the negative advance moves the last glyph back inside the
    // first glyph's span, where the later area must win.
    let lines = chat_lines_from(
        vec![HudComponentTextRun::plain("wna")],
        vec![
            chat_interaction(0, 1, 1, true, false),
            chat_interaction(1, 2, 2, true, false),
            chat_interaction(2, 3, 3, true, false),
        ],
        &fonts,
    );
    let geometry = hud_component_text_hit_geometry(&lines[0].runs, &lines[0].targets, &fonts, true);
    assert_eq!(
        geometry
            .glyph_areas
            .iter()
            .map(|(area, target)| (area.left, area.right, *target))
            .collect::<Vec<_>>(),
        // 'n' spans [8, 6): the half-open test can never accept it.
        vec![
            (0.0, 8.0, Some(1)),
            (8.0, 6.0, Some(2)),
            (6.0, 8.5, Some(3)),
        ]
    );
    assert_eq!(
        message_target(chat_click(&lines, &fonts, 0, 0, 7, 4)),
        Some((3, Some(3)))
    );
    // Fractional advance, not the rounded 9: local 8 is the last accepted
    // column of the 2.5 wide glyph.
    assert_eq!(
        message_target(chat_click(&lines, &fonts, 0, 0, 8, 4)),
        Some((3, Some(3)))
    );
    assert_eq!(message_target(chat_click(&lines, &fonts, 0, 0, 9, 4)), None);
}

#[test]
fn chat_click_area_reflects_italic_bold_and_shadow_extents() {
    // Italic shear reaches back two pixels from the pen.
    let italic = chat_fonts(&[('i', glyph(8.0, 4, 12, 7.0))]);
    let mut italic_runs = vec![HudComponentTextRun::plain("i")];
    italic_runs[0].visual.style.italic = true;
    let lines = chat_lines_from(
        italic_runs,
        vec![chat_interaction(0, 1, 1, true, false)],
        &italic,
    );
    let geometry =
        hud_component_text_hit_geometry(&lines[0].runs, &lines[0].targets, &italic, true);
    assert_eq!(
        (
            geometry.glyph_areas[0].0.left,
            geometry.glyph_areas[0].0.bottom
        ),
        (-2.0, 13.0)
    );
    // One pixel left of the pen is inside the sheared box and its bounds.
    assert_eq!(
        message_target(chat_click(&lines, &italic, 0, 0, -1, 4)),
        Some((1, Some(1)))
    );
    let upright = chat_fonts(&[('i', glyph(8.0, 4, 12, 7.0))]);
    let upright_lines = chat_lines_from(
        vec![HudComponentTextRun::plain("i")],
        vec![chat_interaction(0, 1, 1, true, false)],
        &upright,
    );
    assert_eq!(
        message_target(chat_click(&upright_lines, &upright, 0, 0, -1, 4)),
        None
    );

    // Bold expands every edge by `extraThickness` and keeps a one pixel
    // wider advance, so the ninth row is inside the box.
    let bold_fonts = chat_fonts(&[('b', glyph(6.0, 5, 8, 7.0))]);
    let mut bold_runs = vec![HudComponentTextRun::plain("b")];
    bold_runs[0].visual.style.bold = true;
    let bold_lines = chat_lines_from(
        bold_runs,
        vec![chat_interaction(0, 1, 1, true, false)],
        &bold_fonts,
    );
    let bold_geometry = hud_component_text_hit_geometry(
        &bold_lines[0].runs,
        &bold_lines[0].targets,
        &bold_fonts,
        true,
    );
    assert_eq!(
        (
            bold_geometry.glyph_areas[0].0.left,
            bold_geometry.glyph_areas[0].0.right,
            bold_geometry.glyph_areas[0].0.bottom,
        ),
        (-0.1, 7.0, 9.1)
    );
    assert_eq!(
        message_target(chat_click(&bold_lines, &bold_fonts, 0, 0, 0, 9)),
        Some((1, Some(1)))
    );
    let plain_lines = chat_lines_from(
        vec![HudComponentTextRun::plain("b")],
        vec![chat_interaction(0, 1, 1, true, false)],
        &bold_fonts,
    );
    assert_eq!(
        message_target(chat_click(&plain_lines, &bold_fonts, 0, 0, 0, 9)),
        None
    );

    // An explicit zero shadow color drops the shadow extent the click
    // collector would otherwise always add (`dropShadow = true`).
    let shadowless = chat_lines_from(
        vec![HudComponentTextRun {
            visual: HudStyledTextRun {
                text: "c".to_string(),
                shadow_color: Some(0),
                ..HudStyledTextRun::default()
            },
            font: None,
        }],
        vec![chat_interaction(0, 1, 1, true, false)],
        &chat_fonts(&[('c', glyph(6.0, 5, 8, 7.0))]),
    );
    let shadowless_fonts = chat_fonts(&[('c', glyph(6.0, 5, 8, 7.0))]);
    assert_eq!(
        message_target(chat_click(&shadowless, &shadowless_fonts, 0, 0, 0, 8)),
        None
    );
    let shadowed = chat_lines_from(
        vec![HudComponentTextRun::plain("c")],
        vec![chat_interaction(0, 1, 1, true, false)],
        &shadowless_fonts,
    );
    assert_eq!(
        message_target(chat_click(&shadowed, &shadowless_fonts, 0, 0, 0, 8)),
        Some((1, Some(1)))
    );
}

#[test]
fn chat_click_visits_empty_areas_after_every_glyph() {
    let fonts = chat_fonts(&[
        (' ', HudAsciiGlyph::empty(4.0)),
        ('i', glyph(8.0, 4, 12, 7.0)),
    ]);
    let mut runs = vec![HudComponentTextRun::plain(" i")];
    runs[0].visual.style.italic = true;
    let lines = chat_lines_from(
        runs,
        vec![
            chat_interaction(0, 1, 1, true, false),
            chat_interaction(1, 2, 2, true, false),
        ],
        &fonts,
    );
    // The italic glyph at pen 4 covers [2, 12); the leading space's
    // `EmptyArea` covers [0, 4). Both contain local x = 3, and vanilla
    // visits every glyph instance before any empty area, so the space wins
    // even though it comes first in the text.
    assert_eq!(
        message_target(chat_click(&lines, &fonts, 0, 0, 3, 4)),
        Some((1, Some(1)))
    );
    // The empty area alone is not enough: prepared bounds come from drawn
    // glyphs only, so the leading columns outside the glyph reject.
    assert_eq!(message_target(chat_click(&lines, &fonts, 0, 0, 1, 4)), None);
}

#[test]
fn chat_click_clips_to_the_visible_page_and_drawn_rows() {
    let short = chat_fonts(&[('a', glyph(6.0, 5, 4, 7.0))]);
    let lines = (0..21)
        .flat_map(|index| {
            chat_lines_from(
                vec![HudComponentTextRun::plain("a")],
                vec![chat_interaction(0, 1, index, true, false)],
                &short,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 21);
    // Row 19 is the twentieth visible line; row 20 is scrolled out.
    assert_eq!(
        message_target(chat_click(&lines, &short, 0, 19, 1, 2)),
        Some((1, Some(1)))
    );
    assert_eq!(
        message_target(chat_click(&lines, &short, 0, 20, 1, 2)),
        None
    );
    // Row 0 is the newest line, row 1 the one above it.
    assert_eq!(
        message_target(chat_click(&lines, &short, 0, 1, 1, 2)),
        Some((19, Some(19)))
    );
    // The glyph is four pixels tall: the padding between two rows belongs
    // to neither line's prepared bounds.
    assert_eq!(message_target(chat_click(&lines, &short, 0, 0, 1, 6)), None);
    // Scrolling moves the page instead of the rows.
    assert_eq!(
        message_target(chat_click(&lines, &short, 3, 0, 1, 2)),
        Some((17, Some(17)))
    );
}

#[test]
fn chat_click_separates_click_only_and_insertion_only_targets() {
    let fonts = chat_fonts(&[
        ('a', glyph(6.0, 5, 8, 7.0)),
        ('b', glyph(6.0, 5, 8, 7.0)),
        ('c', glyph(6.0, 5, 8, 7.0)),
    ]);
    let lines = chat_lines_from(
        vec![HudComponentTextRun::plain("abc")],
        vec![
            chat_interaction(0, 1, 1, true, false),
            chat_interaction(1, 2, 2, false, true),
            chat_interaction(2, 3, 3, false, false),
        ],
        &fonts,
    );
    // `includeInsertions(false)`: only a style with a click event answers.
    assert_eq!(
        message_target(chat_click(&lines, &fonts, 0, 0, 1, 4)),
        Some((1, Some(1)))
    );
    // `includeInsertions(true)`: the insertion-only style is found, but the
    // unmodified scan has nothing for it.
    assert_eq!(
        message_target(chat_click(&lines, &fonts, 0, 0, 7, 4)),
        Some((2, None))
    );
    // A hover-only style is targetable by neither scan.
    assert_eq!(
        message_target(chat_click(&lines, &fonts, 0, 0, 13, 4)),
        None
    );
    // The tooltip hover keeps the style under the cursor whatever it does.
    assert_eq!(
        hud_chat_line_interaction_at(
            &lines,
            &chat_context(0),
            &fonts,
            default_chat_geometry(),
            (11.0, 556.0),
        )
        .map(|interaction| interaction.target_index),
        Some(2)
    );
    assert_eq!(
        hud_chat_line_interaction_at(
            &lines,
            &chat_context(0),
            &fonts,
            default_chat_geometry(),
            (17.0, 556.0),
        )
        .map(|interaction| interaction.target_index),
        Some(3)
    );
}

fn message_sequence(kind: Option<HudChatHitTargetKind>) -> Option<u64> {
    match kind {
        Some(HudChatHitTargetKind::Message {
            display_sequence, ..
        }) => Some(display_sequence),
        _ => None,
    }
}

#[test]
fn chat_scale_sanitizes_like_unit_double_and_wraps_at_the_floored_width() {
    for (scale, sanitized) in [
        (0.5, 0.5),
        (0.0, 0.0),
        (1.0, 1.0),
        (1.5, 1.0),
        (-0.25, 1.0),
        (f64::NAN, 1.0),
    ] {
        assert_eq!(
            scaled(scale).sanitized().scale.to_bits(),
            f64::to_bits(sanitized)
        );
    }
    // The wrap floors a double quotient while the drawn page ceils a float one.
    for (scale, wrap_width, drawn_width) in [
        (1.0, 320, 320),
        (0.5, 640, 640),
        (0.75, 426, 427),
        (0.3, 1066, 1067),
        (0.0, i32::MAX, i32::MAX),
    ] {
        assert_eq!(scaled(scale).wrap_width(), wrap_width);
        assert_eq!(
            HudChatGeometry::new(scaled(scale), PhysicalSize::new(800, 600)).max_width,
            drawn_width
        );
    }

    let text = "a".repeat(106);
    let fonts = HudFontGlyphBook::from(glyphs_for(&text));
    let message = |tag| HudChatMessage {
        display_sequence: 1,
        added_tick: 0,
        runs: vec![HudComponentTextRun::plain(&text)],
        interactions: Vec::new(),
        tag,
    };
    let line_lengths = |message, scale| {
        hud_wrap_chat_messages(&[message], &fonts, scaled(scale).wrap_width())
            .iter()
            .map(|line| {
                line.runs
                    .iter()
                    .map(|run| run.visual.text.chars().count())
                    .sum::<usize>()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(line_lengths(message(None), 1.0), vec![53, 53]);
    assert_eq!(line_lengths(message(None), 0.5), vec![106]);
    let modified = HudChatTag {
        kind: HudChatTagKind::Modified,
        indicator_color: 6_316_128,
        tooltip_runs: Vec::new(),
    };
    assert_eq!(line_lengths(message(Some(modified)), 0.5), vec![104, 2]);
}

/// Every expectation is the output of the 26.1 client's `ScreenRectangle`
/// and the bundled JOML `Matrix3x2f` driven with the chat pose over a
/// 600-pixel GUI: a 17x9 prepared text one pixel above the row-0 text top.
#[test]
fn chat_scale_pose_matches_the_capture_path_float_rounding() {
    let surface = PhysicalSize::new(800, 600);
    let bounds = Some([0.0, -1.0, 17.0, 8.0]);
    let pointers = [(2, 556), (3, 559), (9, 552), (10, 559), (5, 557), (6, 575)];
    for (scale, max_width, chat_bottom, background, contains) in [
        (
            1.0,
            320,
            560,
            [[0.0, 551.0], [332.0, 560.0]],
            [false, false, true, true, true, false],
        ),
        (
            0.5,
            640,
            1120,
            [[0.0, 555.5], [326.0, 560.0]],
            [true, true, false, true, true, false],
        ),
        (
            0.3,
            1067,
            1866,
            [[0.0, 557.100_04], [323.7, 559.800_05]],
            [false, true, false, false, true, false],
        ),
        (
            0.75,
            427,
            746,
            [[0.0, 552.75], [329.25, 559.5]],
            [false, false, true, false, true, false],
        ),
        (
            0.1,
            3200,
            5600,
            [[0.0, 559.100_04], [321.2, 560.0]],
            [false; 6],
        ),
        (
            0.0,
            i32::MAX,
            i32::MAX,
            [[0.0, 0.0], [0.0, 0.0]],
            [false; 6],
        ),
    ] {
        let geometry = HudChatGeometry::new(scaled(scale), surface);
        assert_eq!(
            (geometry.max_width, geometry.chat_bottom),
            (max_width, chat_bottom)
        );
        let entry_bottom = geometry.entry_bottom(0);
        let corners = geometry.fill_corners([
            -4,
            entry_bottom.wrapping_sub(9),
            max_width.wrapping_add(8),
            entry_bottom,
        ]);
        assert_eq!([corners[0], corners[2]], background, "scale {scale}");
        for (pointer, expected) in pointers.into_iter().zip(contains) {
            assert_eq!(
                geometry.text_bounds_contain(bounds, geometry.text_top(0), pointer),
                expected,
                "scale {scale} pointer {pointer:?}"
            );
        }
    }
    let local = |scale, pointer| HudChatGeometry::new(scaled(scale), surface).local_point(pointer);
    assert_eq!(local(0.3, (2, 556)), [2.666_666_5, 1_853.333_3]);
    assert_eq!(local(0.3, (10, 559)), [29.333_332, 1_863.333_3]);
    assert_eq!(local(0.75, (2, 556)), [-1.333_333_3, 741.333_4]);
    assert_eq!(local(0.1, (2, 556)), [15.999_998, 5_559.999_5]);
    assert!(local(0.0, (2, 556)).iter().all(|value| value.is_nan()));
}

#[test]
fn scaled_chat_pointer_targets_follow_the_scaled_rows_through_scroll() {
    let fonts = chat_fonts(&[('a', glyph(6.0, 5, 8, 7.0))]);
    let tag = HudChatTag {
        kind: HudChatTagKind::System,
        indicator_color: 0xA8_A8A8,
        tooltip_runs: vec![HudStyledTextRun::plain("system")],
    };
    let messages = (1..=3)
        .map(|display_sequence| HudChatMessage {
            display_sequence,
            added_tick: 0,
            runs: vec![HudComponentTextRun::plain("a")],
            interactions: vec![chat_interaction(0, 1, 1, true, false)],
            tag: (display_sequence == 3).then(|| tag.clone()),
        })
        .collect::<Vec<_>>();
    let lines = hud_wrap_chat_messages(&messages, &fonts, scaled(0.5).wrap_width());
    let half = HudChatGeometry::new(scaled(0.5), PhysicalSize::new(800, 600));
    let unscaled = default_chat_geometry();
    let message = |display_sequence| {
        Some(HudChatHitTargetKind::Message {
            display_sequence,
            target_index: 1,
            click_target_index: Some(1),
        })
    };
    let (queue, restrictions) = (
        Some(HudChatHitTargetKind::Queue),
        Some(HudChatHitTargetKind::Restrictions),
    );
    let queued = HudChatContext {
        queued_messages: 1,
        queue_text: Some("a".to_string()),
        ..chat_context(0)
    };
    let restricted = |scroll| HudChatContext {
        restricted_text: Some("aaa".to_string()),
        ..chat_context(scroll)
    };
    for (context, geometry, pointer, expected) in [
        // Half-scale rows sit 4.5 GUI pixels apart from a text origin of
        // (2, 556); the pointer inverts to local pixels for the glyph test.
        (chat_context(0), half, (3, 558), message(3)),
        (chat_context(0), half, (4, 558), message(3)),
        (chat_context(0), half, (5, 558), None),
        (chat_context(0), half, (1, 558), None),
        (chat_context(0), half, (3, 553), message(2)),
        (chat_context(0), half, (3, 548), message(1)),
        // Scrolling moves every line down one scaled row.
        (chat_context(1), half, (3, 558), message(2)),
        (chat_context(1), half, (3, 553), message(1)),
        (chat_context(1), half, (3, 548), None),
        (chat_context(0), unscaled, (3, 558), None),
        (chat_context(0), unscaled, (5, 556), message(3)),
        // The queue text top is local `chatBottom + 1`, GUI y 560.5 at half scale.
        (queued.clone(), half, (3, 562), queue),
        (queued.clone(), half, (4, 563), queue),
        (queued.clone(), half, (5, 562), None),
        (queued.clone(), half, (323, 562), None),
        (queued.clone(), half, (3, 566), None),
        (queued, unscaled, (6, 565), queue),
        // The restriction prompt sits one scaled row above the drawn rows.
        (restricted(0), half, (2, 543), restrictions),
        (restricted(0), half, (2, 547), message(1)),
        (restricted(1), half, (2, 547), restrictions),
    ] {
        let position = (f64::from(pointer.0), f64::from(pointer.1));
        let hit = hud_chat_hit_target_at(&lines, &context, &fonts, geometry, position);
        assert_eq!(
            hit.map(|hit| hit.kind),
            expected,
            "pointer {pointer:?} scroll {} scale {}",
            context.scroll.position,
            geometry.scale
        );
    }
    for (geometry, expected) in [(half, Some(1)), (unscaled, None)] {
        let hovered =
            hud_chat_line_interaction_at(&lines, &chat_context(0), &fonts, geometry, (3.0, 558.0));
        assert_eq!(
            hovered.map(|interaction| interaction.target_index),
            expected
        );
    }

    // The tag indicator spans local x -4..-2, one screen pixel at half scale.
    for (scroll, geometry, x, expected) in [
        (0, half, 0.0, Some(HudChatTagKind::System)),
        (0, half, 1.0, None),
        (0, unscaled, 1.0, Some(HudChatTagKind::System)),
        (1, half, 0.0, None),
    ] {
        let tag = hud_chat_tag_at(&lines, &chat_context(scroll), geometry, (x, 557.0), &fonts);
        assert_eq!(tag.map(|tag| tag.kind), expected, "x {x} scroll {scroll}");
    }
}

#[test]
fn startup_chat_scale_rewraps_and_draws_the_rows_its_pointer_hits() {
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
        ..glyph(6.0, 5, 8, 7.0)
    };
    renderer
        .upload_hud_font_atlas(1, 1, &[255; 4], chat_fonts(&[('a', white)]))
        .expect("font atlas");
    let message = |display_sequence, text: &str| HudChatMessage {
        display_sequence,
        added_tick: 0,
        runs: vec![HudComponentTextRun::plain(text)],
        interactions: vec![chat_interaction(0, 1, 1, true, false)],
        tag: None,
    };
    renderer.set_hud_chat_messages(vec![message(1, "a"), message(2, &"a".repeat(60))]);
    assert_eq!(renderer.hud_chat_lines.len(), 3);
    renderer.set_hud_chat_options(scaled(0.5));
    assert_eq!(renderer.hud_chat_lines.len(), 2);

    let clear = [0, 0, 255, 255];
    let is_glyph = |pixel: [u8; 4]| pixel[0] > 230 && pixel[1] > 230;
    let is_background = |pixel: [u8; 4]| pixel[0] < 30 && (40..230).contains(&pixel[2]);
    let sequence_at = |renderer: &Renderer, x: f64, y: f64| {
        message_sequence(renderer.hud_chat_hit_target_at((x, y)).map(|hit| hit.kind))
    };

    // Half scale over a 200-pixel chat bottom: row 0 spans y 195.5..200 with
    // its glyph at (2, 196), row 1 spans 191..195.5, the page is 326 wide.
    renderer.set_hud_chat_context(chat_context(0));
    let frame = renderer
        .render_offscreen_frame()
        .expect("scaled chat frame");
    assert!(is_glyph(frame.pixel(3, 197)));
    assert!(is_glyph(frame.pixel(3, 193)));
    assert!(is_background(frame.pixel(250, 197)));
    assert!(is_background(frame.pixel(324, 197)));
    assert!(is_background(frame.pixel(250, 193)));
    assert_eq!(frame.pixel(328, 197), clear);
    assert_eq!(frame.pixel(250, 189), clear);
    assert_eq!(sequence_at(&renderer, 3.0, 197.0), Some(2));
    assert_eq!(sequence_at(&renderer, 3.0, 193.0), Some(1));

    renderer.set_hud_chat_context(chat_context(1));
    let scrolled = renderer
        .render_offscreen_frame()
        .expect("scrolled chat frame");
    assert!(is_glyph(scrolled.pixel(3, 197)));
    assert_eq!(scrolled.pixel(250, 193), clear);
    assert_eq!(sequence_at(&renderer, 3.0, 197.0), Some(1));
    assert_eq!(sequence_at(&renderer, 3.0, 193.0), None);

    renderer.set_hud_chat_options(scaled(1.0));
    assert_eq!(renderer.hud_chat_lines.len(), 3);
    renderer.set_hud_chat_context(chat_context(0));
    let unscaled = renderer
        .render_offscreen_frame()
        .expect("unscaled chat frame");
    assert!(is_background(unscaled.pixel(3, 197)));
    assert!(is_glyph(unscaled.pixel(5, 197)));
    assert!(is_background(unscaled.pixel(328, 197)));
    assert!(is_background(unscaled.pixel(326, 189)));
    // Unscaled, the 60-glyph message wraps and its clickable first glyph moves
    // to row 1 at (4, 183).
    assert_eq!(sequence_at(&renderer, 3.0, 188.0), None);
    assert_eq!(sequence_at(&renderer, 5.0, 188.0), Some(2));
    assert_eq!(sequence_at(&renderer, 5.0, 197.0), None);
}
