use super::*;

fn glyphs(advance: f32) -> HudFontGlyphMap {
    let mut map = HudFontGlyphMap::new();
    for (index, ch) in ['😀', '\u{fffd}', 'A', 'B', '_'].into_iter().enumerate() {
        let glyph = HudAsciiGlyph {
            uv: HudUvRect {
                min: [index as f32 / 5.0, 0.0],
                max: [(index + 1) as f32 / 5.0, 1.0],
            },
            width: 2,
            height: 2,
            advance: if ch == '😀' { 12.0 } else { advance },
            ..Default::default()
        };
        map.insert_selected_codepoint_first_wins(
            ch as u32,
            HudAsciiGlyph {
                advance: 40.0,
                ..glyph
            },
            glyph,
            40.0,
        );
    }
    map
}

fn view(text: &str, cursor: usize, selection: usize, display_start: usize) -> HudChatInputView {
    HudChatInputView {
        value: text.encode_utf16().collect(),
        cursor,
        selection,
        display_start,
    }
}

fn text(runs: &[HudStyledTextRun]) -> String {
    runs.iter().map(|r| r.text.as_str()).collect()
}

/// `ChatFormatting` RGB values behind `CommandSuggestions`' styles.
const ARGUMENT_COLORS: [u32; 5] = [0x55FFFF, 0xFFFF55, 0x55FF55, 0xFF55FF, 0xFFAA00];
const LITERAL_COLOR: u32 = 0xAAAAAA;
const UNPARSED_COLOR: u32 = 0xFF5555;

fn ranges(ranges: &[(usize, usize)]) -> Vec<HudCommandInputRange> {
    ranges
        .iter()
        .map(|(start, end)| HudCommandInputRange {
            start: *start,
            end: *end,
        })
        .collect()
}

fn colored(actual: &[HudStyledTextRun]) -> Vec<(&str, Option<u32>)> {
    actual
        .iter()
        .map(|run| (run.text.as_str(), run.color))
        .collect()
}

fn syntax_state(
    text: &str,
    arguments: &[(usize, usize)],
    unread: Option<(usize, usize)>,
) -> HudCommandSuggestions {
    sanitize_hud_command_suggestions(HudCommandSuggestions {
        input_view: Some(view(
            text,
            text.encode_utf16().count(),
            text.encode_utf16().count(),
            0,
        )),
        syntax: Some(HudCommandInputSyntax {
            arguments: ranges(arguments),
            unread: unread.map(|(start, end)| HudCommandInputRange { start, end }),
        }),
        ..Default::default()
    })
}

/// One white 2x2 glyph per character, so a drawn pixel is the run's colour.
fn white_glyphs() -> HudFontGlyphMap {
    let mut map = HudFontGlyphMap::new();
    let chars = ['/', 'A', 'B', 'C', 'D', 'E', 'F', '\u{fffd}'];
    for (index, ch) in chars.into_iter().enumerate() {
        let glyph = HudAsciiGlyph {
            uv: HudUvRect {
                min: [index as f32 / chars.len() as f32, 0.0],
                max: [(index + 1) as f32 / chars.len() as f32, 1.0],
            },
            width: 2,
            height: 2,
            advance: if ch == '\u{fffd}' { 12.0 } else { 6.0 },
            ..HudAsciiGlyph::default()
        };
        map.insert_selected_codepoint_first_wins(
            ch as u32,
            HudAsciiGlyph {
                advance: 40.0,
                ..glyph
            },
            glyph,
            40.0,
        );
    }
    map.insert_selected_codepoint_first_wins(
        ' ' as u32,
        HudAsciiGlyph::empty(6.0),
        HudAsciiGlyph::empty(6.0),
        6.0,
    );
    map
}

#[test]
fn chat_input_command_syntax_colors_arguments_and_cycles_after_five() {
    // "/cmd a1 b2 c3 d4 e5 f6": six parsed arguments cycle the five styles.
    let state = syntax_state(
        "/cmd a1 b2 c3 d4 e5 f6",
        &[(5, 7), (8, 10), (11, 13), (14, 16), (17, 19), (20, 22)],
        None,
    );
    assert_eq!(
        colored(&hud_command_input_runs(&state)),
        vec![
            ("/cmd ", Some(LITERAL_COLOR)),
            ("a1", Some(ARGUMENT_COLORS[0])),
            (" ", Some(LITERAL_COLOR)),
            ("b2", Some(ARGUMENT_COLORS[1])),
            (" ", Some(LITERAL_COLOR)),
            ("c3", Some(ARGUMENT_COLORS[2])),
            (" ", Some(LITERAL_COLOR)),
            ("d4", Some(ARGUMENT_COLORS[3])),
            (" ", Some(LITERAL_COLOR)),
            ("e5", Some(ARGUMENT_COLORS[4])),
            (" ", Some(LITERAL_COLOR)),
            ("f6", Some(ARGUMENT_COLORS[0])),
        ]
    );
    // A command without arguments keeps vanilla's literal grey.
    let literal = syntax_state("/cmd", &[], None);
    assert_eq!(
        colored(&hud_command_input_runs(&literal)),
        vec![("/cmd", Some(LITERAL_COLOR))]
    );
    // Without a parse the input keeps the edit box's own text colour.
    let plain = sanitize_hud_command_suggestions(HudCommandSuggestions {
        input_view: Some(view("/cmd", 4, 4, 0)),
        ..Default::default()
    });
    assert!(hud_command_input_runs(&plain).is_empty());
}

#[test]
fn chat_input_command_syntax_marks_the_unread_tail_and_returns_to_literal() {
    let state = syntax_state("/cmd ab rest", &[(5, 7)], Some((8, 12)));
    assert_eq!(
        colored(&hud_command_input_runs(&state)),
        vec![
            ("/cmd ", Some(LITERAL_COLOR)),
            ("ab", Some(ARGUMENT_COLORS[0])),
            (" ", Some(LITERAL_COLOR)),
            ("rest", Some(UNPARSED_COLOR)),
        ]
    );
    // `formatText` also appends whatever follows the unread region as literal.
    let trailing = syntax_state("/cmd ab rest", &[(5, 7)], Some((8, 10)));
    assert_eq!(
        colored(&hud_command_input_runs(&trailing)),
        vec![
            ("/cmd ", Some(LITERAL_COLOR)),
            ("ab", Some(ARGUMENT_COLORS[0])),
            (" ", Some(LITERAL_COLOR)),
            ("re", Some(UNPARSED_COLOR)),
            ("st", Some(LITERAL_COLOR)),
        ]
    );
    // A range that cannot be read after the previous one keeps its grey gap:
    // vanilla's `substring(unformattedStart, start)` would throw there.
    let inverted = syntax_state("/cmd ab", &[(5, 7), (2, 4)], None);
    assert_eq!(
        colored(&hud_command_input_runs(&inverted)),
        vec![
            ("/cmd ", Some(LITERAL_COLOR)),
            ("ab", Some(ARGUMENT_COLORS[0])),
        ]
    );
    // Ranges past the value end are clamped to the units that exist.
    let clamped = syntax_state("/cmd ab", &[(5, 99), (6, 8)], Some((6, 99)));
    assert_eq!(
        colored(&hud_command_input_runs(&clamped)),
        vec![
            ("/cmd ", Some(LITERAL_COLOR)),
            ("ab", Some(ARGUMENT_COLORS[0])),
        ]
    );
}

#[test]
fn chat_input_draft_style_outranks_a_parse_the_screen_still_holds() {
    let state = sanitize_hud_command_suggestions(HudCommandSuggestions {
        input_view: Some(view("/cmd ab", 7, 7, 0)),
        input_text: "/cmd ab".into(),
        draft: true,
        syntax: Some(HudCommandInputSyntax {
            arguments: ranges(&[(5, 7)]),
            unread: Some(HudCommandInputRange { start: 1, end: 5 }),
        }),
        ..Default::default()
    });
    let runs = hud_command_input_runs(&state);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "/cmd ab");
    assert!(runs[0].style.italic);
    assert_eq!(runs[0].color, Some(LITERAL_COLOR));
}

#[test]
fn chat_input_command_syntax_geometry_follows_scrolled_offsets_and_split_pairs() {
    let map = white_glyphs();
    // "/😀AB C" is seven UTF-16 units; the parsed argument is units 1..5.
    let state = syntax_state("/\u{1f600}AB C", &[(1, 5)], None);
    let runs = hud_command_input_runs(&state);
    assert_eq!(
        colored(&runs),
        vec![
            ("/", Some(LITERAL_COLOR)),
            ("\u{1f600}AB", Some(ARGUMENT_COLORS[0])),
            (" C", Some(LITERAL_COLOR)),
        ]
    );
    // A cursor inside the pair splits it; both halves keep the argument style
    // because the ranges are units of the raw value.
    let layout = input_layout(&view("/\u{1f600}AB C", 2, 2, 0), &runs, map.non_fishy(), 60);
    assert_eq!(
        colored(&layout.head),
        vec![
            ("/", Some(LITERAL_COLOR)),
            ("\u{fffd}", Some(ARGUMENT_COLORS[0]))
        ]
    );
    assert_eq!(
        colored(&layout.tail),
        vec![
            ("\u{fffd}AB", Some(ARGUMENT_COLORS[0])),
            (" C", Some(LITERAL_COLOR)),
        ]
    );
    let geometry = hud_logical_styled_text_geometry(
        &layout.tail,
        map.non_fishy(),
        &Default::default(),
        0,
        (layout.tail_x, 0.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    assert_eq!(
        geometry
            .glyph_quads
            .iter()
            .map(|quad| quad.1)
            .collect::<Vec<_>>(),
        vec![
            tint(ARGUMENT_COLORS[0]),
            tint(ARGUMENT_COLORS[0]),
            tint(ARGUMENT_COLORS[0]),
            tint(LITERAL_COLOR),
        ]
    );
    // Scrolling drops the argument style exactly where the range ends: the
    // display offset is the value offset vanilla clamps against.
    let layout = input_layout(&view("/\u{1f600}AB C", 7, 7, 2), &runs, map.non_fishy(), 60);
    assert_eq!(
        colored(&layout.head),
        vec![
            ("\u{fffd}AB", Some(ARGUMENT_COLORS[0])),
            (" C", Some(LITERAL_COLOR)),
        ]
    );
    let layout = input_layout(&view("/\u{1f600}AB C", 7, 7, 4), &runs, map.non_fishy(), 60);
    assert_eq!(
        colored(&layout.head),
        vec![("B", Some(ARGUMENT_COLORS[0])), (" C", Some(LITERAL_COLOR)),]
    );
}

/// A run colour replaces the base tint's RGB and keeps its alpha, so the
/// geometry tint of a parsed run is its `ChatFormatting` value.
fn tint(color: u32) -> [f32; 4] {
    let [r, g, b] = opaque_tint(color);
    [r, g, b, 1.0]
}

fn opaque_tint(color: u32) -> [f32; 3] {
    [16, 8, 0].map(|shift| ((color >> shift) & 0xFF) as f32 / 255.0)
}

#[test]
fn chat_input_whole_frame_draws_parsed_argument_colors_and_unread_red() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 60) else {
        return;
    };
    renderer
        .upload_hud_font_atlas(8, 1, &[255; 32], white_glyphs())
        .expect("white font atlas");
    let state = syntax_state("/AB CDE", &[(1, 3)], Some((4, 7)));
    renderer.set_hud_command_suggestions(Some(state));
    let frame = renderer.render_offscreen_frame().expect("chat input frame");
    // Six-pixel advances from the input's text origin at (4, 48); the space at
    // x = 22 has no bitmap and answers with the input background.
    for (x, color) in [
        (4, LITERAL_COLOR),
        (10, ARGUMENT_COLORS[0]),
        (16, ARGUMENT_COLORS[0]),
        (28, UNPARSED_COLOR),
        (34, UNPARSED_COLOR),
        (40, UNPARSED_COLOR),
    ] {
        let pixel = frame.pixel(x, 48);
        let expected = opaque_tint(color).map(|channel| (channel * 255.0).round() as u8);
        assert_eq!(
            [pixel[0], pixel[1], pixel[2]],
            expected,
            "glyph at {x} for {color:#08x}"
        );
    }
}

/// `EditBox` forwards the edit buffer as
/// `FormattedCharSequence.forward(text, Style.EMPTY)`, so a `§` typed into the
/// chat input is a character of the line: it keeps its own glyph and its own
/// advance, and the letter after it is a glyph too.
#[test]
fn chat_input_draws_a_legacy_code_as_two_glyphs_that_keep_their_advances() {
    const CODE_CHARS: [char; 3] = ['\u{a7}', 'c', 'A'];
    let mut glyphs = HudFontGlyphMap::new();
    for (index, ch) in CODE_CHARS.into_iter().enumerate() {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                uv: HudUvRect {
                    min: [index as f32 / 3.0, 0.0],
                    max: [(index + 1) as f32 / 3.0, 1.0],
                },
                width: 2,
                height: 2,
                advance: 6.0,
                ..HudAsciiGlyph::default()
            },
        );
    }
    assert_eq!(hud_raw_string_width("\u{a7}cA", &glyphs), 18);
    // The same text on the `FormattedText.of` face drops the pair and its
    // advance, which is the shape the input line must not fall into.
    assert_eq!(
        hud_coded_string_width(&hud_coded_string_runs("\u{a7}cA"), &glyphs),
        6
    );

    let Some(mut renderer) = Renderer::new_offscreen(320, 60) else {
        return;
    };
    renderer
        .upload_hud_font_atlas(3, 1, &[255; 12], glyphs)
        .expect("white font atlas");
    renderer.set_hud_command_suggestions(Some(sanitize_hud_command_suggestions(
        HudCommandSuggestions {
            input_view: Some(view("\u{a7}cA", 3, 3, 0)),
            cursor_visible: false,
            ..Default::default()
        },
    )));
    let frame = renderer.render_offscreen_frame().expect("chat input frame");
    // Three six-pixel advances from the input's text origin at (4, 48).
    let expected = [224, 224, 224];
    for x in [4, 10, 16] {
        let pixel = frame.pixel(x, 48);
        assert_eq!(
            [pixel[0], pixel[1], pixel[2]],
            expected,
            "literal glyph at {x}"
        );
    }
}

#[test]
fn chat_input_raw_view_sanitize_retains_units_and_overrides_derived_text() {
    let mut value = vec![65; 255];
    value.extend([0xd83d, 0xde00]);
    let state = sanitize_hud_command_suggestions(HudCommandSuggestions {
        input_text: "conflict".into(),
        cursor: 0,
        selection: 0,
        input_view: Some(HudChatInputView {
            value,
            cursor: usize::MAX,
            selection: 255,
            display_start: 255,
        }),
        ..Default::default()
    });
    let raw = state.input_view.unwrap();
    assert_eq!(raw.value.len(), 256);
    assert_eq!(raw.value[255], 0xd83d);
    assert_eq!(
        (raw.cursor, raw.selection, raw.display_start),
        (256, 255, 255)
    );
    assert_eq!(state.input_text, format!("{}\u{fffd}", "A".repeat(255)));
    assert_eq!((state.cursor, state.selection), (256, 255));
    let ordinary = sanitize_hud_command_suggestions(HudCommandSuggestions {
        input_text: "B".into(),
        input_view: Some(view("A", 1, 1, 0)),
        layout: HudCommandSuggestionsLayout::CommandBlock,
        ..Default::default()
    });
    assert!(ordinary.input_view.is_none());
    assert_eq!(ordinary.input_text, "B");
}

#[test]
fn chat_input_layout_decomposes_each_utf16_cursor_half_and_keeps_styles() {
    let map = glyphs(6.0);
    let raw = view("😀AB", 1, 1, 0);
    let runs = [
        HudStyledTextRun {
            text: "😀".into(),
            color: Some(0xff0000),
            ..Default::default()
        },
        HudStyledTextRun {
            text: "AB".into(),
            color: Some(0x00ff00),
            ..Default::default()
        },
    ];
    let layout = input_layout(&raw, &runs, map.non_fishy(), 18);
    assert_eq!(text(&layout.head), "\u{fffd}");
    assert_eq!(text(&layout.tail), "\u{fffd}A");
    assert_eq!(layout.head[0].color, Some(0xff0000));
    assert_eq!(layout.tail[0].color, Some(0xff0000));
    assert_eq!(layout.tail[1].color, Some(0x00ff00));
    assert_eq!((layout.cursor_x, layout.tail_x), (6.0, 6.0));
    let geometry = hud_logical_styled_text_geometry(
        &layout.tail,
        map.non_fishy(),
        &Default::default(),
        0,
        (layout.tail_x, 0.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    assert_eq!(geometry.glyph_quads[0].1, [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(geometry.glyph_quads[1].1, [0.0, 1.0, 0.0, 1.0]);
    assert_eq!(geometry.glyph_quads[1].0.corners[0][0], 12.0);
    let historical = input_layout(&view("😀AB", 0, 0, 1), &[], map.non_fishy(), 18);
    assert_eq!(text(&historical.head), "\u{fffd}AB");
    assert!(!historical.cursor_on_screen);
    assert_eq!(historical.cursor_x, 0.0);
}

#[test]
fn chat_input_fractional_width_selection_and_empty_insert_follow_edit_box_order() {
    let map = glyphs(1.25);
    let layout = input_layout(&view("AAA", 2, 0, 0), &[], map.non_fishy(), 18);
    assert_eq!((layout.tail_x, layout.cursor_x), (3.0, 3.0));
    assert_eq!(layout.selection, Some((-1.0, 3.0)));
    let append = input_layout(&view("AA", 2, 2, 0), &[], map.non_fishy(), 18);
    assert_eq!(append.cursor_x, 4.0);
    assert!(!append.insert);
    let empty = input_layout(&view("A", 0, 0, 0), &[], map.non_fishy(), 0);
    assert_eq!(empty.cursor_x, -1.0);
    assert!(empty.insert && empty.cursor_on_screen);
}

#[test]
fn chat_input_whole_frame_uses_raw_history_split_pairs_and_cursor_layer_order() {
    let Some(mut renderer) = Renderer::new_offscreen(22, 60) else {
        return;
    };
    renderer
        .upload_hud_font_atlas(
            5,
            1,
            &[
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255, 255, 0, 255, 255,
            ],
            glyphs(6.0),
        )
        .expect("font atlas");
    let mut state = HudCommandSuggestions {
        input_view: Some(view("😀AB", 4, 4, 1)),
        ..Default::default()
    };
    renderer.set_hud_command_suggestions(Some(state.clone()));
    let frame = renderer
        .render_offscreen_frame()
        .expect("historical viewport");
    assert_green(frame.pixel(4, 48));
    assert_blue(frame.pixel(10, 48));
    assert_yellow(frame.pixel(16, 48));
    state.input_view = Some(view("😀AB", 0, 0, 1));
    renderer.set_hud_command_suggestions(Some(state.clone()));
    let frame = renderer
        .render_offscreen_frame()
        .expect("offscreen cursor preserves viewport");
    assert_green(frame.pixel(4, 48));
    assert_yellow(frame.pixel(16, 48));

    state.input_view = Some(view("😀AB", 1, 1, 0));
    state.cursor_visible = true;
    state.suffix = Some("B".into());
    renderer.set_hud_command_suggestions(Some(state.clone()));
    let frame = renderer
        .render_offscreen_frame()
        .expect("split surrogate cursor");
    assert_green(frame.pixel(4, 48));
    assert_green(frame.pixel(11, 48));
    assert_blue(frame.pixel(16, 48));
    assert_gray(frame.pixel(10, 48));
    let selection_background = frame.pixel(12, 48);
    state.input_view = Some(view("😀AB", 1, 4, 0));
    renderer.set_hud_command_suggestions(Some(state.clone()));
    let frame = renderer
        .render_offscreen_frame()
        .expect("selection before cursor");
    assert_highlight(frame.pixel(12, 48), selection_background);
    assert_gray(frame.pixel(10, 48));

    state.input_view = Some(view("A", 1, 1, 0));
    renderer.set_hud_command_suggestions(Some(state));
    let frame = renderer
        .render_offscreen_frame()
        .expect("append suffix and cursor");
    assert_blue(frame.pixel(4, 48));
    assert_yellow(frame.pixel(10, 48));
    let cursor = frame.pixel(11, 48);
    assert!(
        cursor[0] > 180 && cursor[1] < 40 && cursor[2] > 180,
        "append cursor: {cursor:?}"
    );
}

fn assert_green(p: [u8; 4]) {
    assert!(p[1] > 180 && p[0] < 40 && p[2] < 40, "green {p:?}");
}
fn assert_blue(p: [u8; 4]) {
    assert!(p[2] > 180 && p[0] < 40 && p[1] < 40, "blue {p:?}");
}
fn assert_yellow(p: [u8; 4]) {
    assert!(p[0] > 70 && p[1] > 70 && p[2] < 40, "yellow {p:?}");
}
fn assert_gray(p: [u8; 4]) {
    assert!(p[0] > 180 && p[1] > 180 && p[2] > 180, "gray {p:?}");
}

fn assert_highlight(actual: [u8; 4], before: [u8; 4]) {
    let expected = [255 - before[0], 255 - before[1], 255, 255];
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            actual.abs_diff(expected) <= 1,
            "highlight channel {actual}, expected {expected}"
        );
    }
}

#[test]
fn chat_input_gpu_keeps_formatted_colors_on_both_sides_of_a_split_pair() {
    use wgpu::util::DeviceExt;
    let Some(mut renderer) = Renderer::new_offscreen(32, 20) else {
        return;
    };
    renderer
        .upload_hud_font_atlas(5, 1, &[255; 20], glyphs(6.0))
        .expect("white font atlas");
    let runs = [
        HudStyledTextRun {
            text: "😀".into(),
            color: Some(0xff0000),
            ..Default::default()
        },
        HudStyledTextRun {
            text: "AB".into(),
            color: Some(0x00ff00),
            ..Default::default()
        },
    ];
    let mut vertices = Vec::new();
    let mut commands = Vec::new();
    push_chat_input(
        &mut vertices,
        &mut commands,
        &renderer.hud_white_pixel,
        renderer.pack.hud_font_atlas.as_ref().unwrap(),
        renderer.pack.hud_font_book.non_fishy_glyphs(None),
        &renderer.pack.hud_obfuscated_glyph_pool,
        0,
        PhysicalSize::new(32, 20),
        &view("😀AB", 1, 1, 0),
        &runs,
        false,
        None,
        (4.0, 4.0),
        18,
    );
    let buffer = renderer
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chat input styled vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
    let crate::frame_flight::RenderSurface::Offscreen(target) = &renderer.surface else {
        unreachable!()
    };
    let target_view = target.create_view(&Default::default());
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("chat input styled draw"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&renderer.pipelines.hud);
        pass.set_vertex_buffer(0, buffer.slice(..));
        for command in commands {
            let HudDrawCommand::Sprite { sprite, start, end } = command else {
                panic!("text sprite")
            };
            pass.set_bind_group(0, &sprite.gpu.bind_group, &[]);
            pass.draw(start..end, 0..1);
        }
    }
    let pending = renderer
        .prepare_screenshot_copy(&mut encoder, target)
        .expect("copy");
    renderer.queue.submit([encoder.finish()]);
    let pixels = renderer.read_screenshot_pixels(pending).expect("readback");
    for x in [4, 10] {
        let p = pixels.pixel(x, 4);
        assert!(
            p[0] > 180 && p[1] < 40 && p[2] < 40,
            "styled replacement: {p:?}"
        );
    }
    assert_green(pixels.pixel(16, 4));
}

#[test]
fn chat_input_whole_frame_keeps_popup_and_command_block_on_ordinary_metrics() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer
        .upload_hud_font_atlas(
            5,
            1,
            &[
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255, 255, 0, 255, 255,
            ],
            glyphs(1.25),
        )
        .expect("fractional font atlas");
    let mut state = HudCommandSuggestions {
        input_view: Some(view("AAA", 2, 2, 0)),
        cursor_visible: true,
        list_x: 20,
        list_y: 40,
        list_width: 81,
        list_height: 12,
        entries: vec![HudCommandSuggestionEntry {
            runs: vec![HudStyledTextRun::plain("AA")],
            selected: false,
        }],
        ..Default::default()
    };
    renderer.set_hud_command_suggestions(Some(state.clone()));
    let frame = renderer
        .render_offscreen_frame()
        .expect("fractional chat and ordinary popup");
    assert_gray(frame.pixel(7, 228));
    assert_blue(frame.pixel(8, 228));
    assert_eq!(frame.pixel(21, 42), [0, 0, 170, 255]);
    assert_eq!(frame.pixel(61, 42), [0, 0, 170, 255]);
    state.layout = HudCommandSuggestionsLayout::CommandBlock;
    state.input_text = "AA".into();
    state.cursor = 2;
    state.selection = 2;
    state.cursor_visible = false;
    renderer.set_hud_command_suggestions(Some(state));
    let frame = renderer
        .render_offscreen_frame()
        .expect("ordinary command block ignores raw chat view");
    let layout = hud_command_block_editor_layout(PhysicalSize::new(320, 240));
    let x = (layout.command_input.x + 4) as u32;
    let y = (layout.command_input.y + 6) as u32;
    assert_blue(frame.pixel(x, y));
    assert_blue(frame.pixel(x + 40, y));
}

fn render_commands(
    renderer: &Renderer,
    vertices: &[HudVertex],
    commands: &[HudDrawCommand<'_>],
) -> (crate::screenshot::ScreenshotPixels, (u64, u64)) {
    render_commands_over(
        renderer,
        vertices,
        commands,
        wgpu::Color {
            r: 0.2,
            g: 0.4,
            b: 0.6,
            a: 1.0,
        },
    )
}

fn render_commands_over(
    renderer: &Renderer,
    vertices: &[HudVertex],
    commands: &[HudDrawCommand<'_>],
    clear: wgpu::Color,
) -> (crate::screenshot::ScreenshotPixels, (u64, u64)) {
    use wgpu::util::DeviceExt;
    let buffer = renderer
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chat highlight vertices"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
    let crate::frame_flight::RenderSurface::Offscreen(target) = &renderer.surface else {
        unreachable!()
    };
    let target_view = target.create_view(&Default::default());
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    let counts = {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("chat highlight draw"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        renderer.draw_hud_commands(&mut pass, &buffer, commands)
    };
    let pending = renderer
        .prepare_screenshot_copy(&mut encoder, target)
        .unwrap();
    renderer.queue.submit([encoder.finish()]);
    (renderer.read_screenshot_pixels(pending).unwrap(), counts)
}

fn input_commands<'a>(
    renderer: &'a Renderer,
    raw: &HudChatInputView,
    cursor: bool,
) -> (Vec<HudVertex>, Vec<HudDrawCommand<'a>>) {
    let mut vertices = Vec::new();
    let mut commands = Vec::new();
    let runs = [
        HudStyledTextRun {
            text: "A".into(),
            color: Some(0xffffff),
            ..Default::default()
        },
        HudStyledTextRun {
            text: "B".into(),
            color: Some(0x336699),
            ..Default::default()
        },
    ];
    push_chat_input(
        &mut vertices,
        &mut commands,
        &renderer.hud_white_pixel,
        renderer.pack.hud_font_atlas.as_ref().unwrap(),
        renderer.pack.hud_font_book.non_fishy_glyphs(None),
        &renderer.pack.hud_obfuscated_glyph_pool,
        0,
        PhysicalSize::new(32, 20),
        raw,
        &runs,
        cursor,
        None,
        (4.0, 4.0),
        24,
    );
    push_hud_draw_with_uv_and_tint(
        &mut vertices,
        &mut commands,
        &renderer.hud_white_pixel,
        PhysicalSize::new(32, 20),
        absolute_hud_rect(20.0, 4.0, 2, 2),
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        [1.0, 0.0, 0.0, 1.0],
    );
    (vertices, commands)
}

#[test]
fn chat_input_gpu_highlight_inverts_white_color_and_background_before_cursor_and_sprite() {
    let Some(mut renderer) = Renderer::new_offscreen(32, 20) else {
        return;
    };
    renderer
        .upload_hud_font_atlas(5, 1, &[255; 20], glyphs(6.0))
        .unwrap();
    let (vertices, commands) = input_commands(&renderer, &view("AB", 0, 0, 0), false);
    let (baseline, _) = render_commands(&renderer, &vertices, &commands);
    for (cursor, selection) in [(0, 2), (2, 0), (1, 2)] {
        let (vertices, commands) =
            input_commands(&renderer, &view("AB", cursor, selection, 0), false);
        let highlight = commands
            .iter()
            .position(|command| matches!(command, HudDrawCommand::TextHighlight { .. }))
            .unwrap();
        let HudDrawCommand::TextHighlight {
            invert_start,
            additive_start,
            end,
        } = commands[highlight]
        else {
            unreachable!()
        };
        assert_eq!(
            (additive_start - invert_start, end - additive_start),
            (6, 6)
        );
        assert!(vertices[invert_start as usize..additive_start as usize]
            .iter()
            .all(|vertex| vertex.tint == [1.0; 4]));
        assert!(vertices[additive_start as usize..end as usize]
            .iter()
            .all(|vertex| vertex.tint == [0.0, 0.0, 1.0, 1.0]));
        let (frame, counts) = render_commands(&renderer, &vertices, &commands);
        for x in [10, 12] {
            assert_highlight(frame.pixel(x, 4), baseline.pixel(x, 4));
        }
        if cursor != 1 {
            assert_highlight(frame.pixel(4, 4), baseline.pixel(4, 4));
        } else {
            assert_eq!(frame.pixel(4, 4), baseline.pixel(4, 4));
        }
        assert_eq!(frame.pixel(20, 4), [255, 0, 0, 255]);
        assert_eq!(counts, (commands.len() as u64 + 1, 4));
    }
    let (vertices, commands) = input_commands(&renderer, &view("AB", 0, 2, 0), true);
    let (frame, counts) = render_commands(&renderer, &vertices, &commands);
    assert_gray(frame.pixel(4, 4));
    assert_highlight(frame.pixel(10, 4), baseline.pixel(10, 4));
    assert_eq!(frame.pixel(20, 4), [255, 0, 0, 255]);
    assert_eq!(counts, (commands.len() as u64 + 1, 5));
}

fn highlight_quad(alpha: f32) -> Vec<HudVertex> {
    let mut vertices = Vec::new();
    for tint in [[1.0, 1.0, 1.0, alpha], [0.0, 0.0, 1.0, alpha]] {
        vertices.extend_from_slice(&hud_quad_vertices(
            PhysicalSize::new(32, 20),
            absolute_hud_rect(2.0, 2.0, 20, 10),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            tint,
        ));
    }
    vertices
}

#[test]
fn chat_input_gpu_highlight_empty_scissor_suppresses_both_passes_and_recovers() {
    let Some(renderer) = Renderer::new_offscreen(32, 20) else {
        return;
    };
    let mut vertices = highlight_quad(1.0);
    let mut commands = vec![
        HudDrawCommand::Scissor {
            x: 40,
            y: 0,
            width: 2,
            height: 2,
        },
        HudDrawCommand::TextHighlight {
            invert_start: 0,
            additive_start: 6,
            end: 12,
        },
        HudDrawCommand::Sprite {
            sprite: &renderer.hud_white_pixel,
            start: 0,
            end: 6,
        },
        HudDrawCommand::Scissor {
            x: 3,
            y: 3,
            width: 3,
            height: 3,
        },
        HudDrawCommand::TextHighlight {
            invert_start: 0,
            additive_start: 6,
            end: 12,
        },
        HudDrawCommand::Scissor {
            x: 0,
            y: 0,
            width: 32,
            height: 20,
        },
    ];
    push_hud_draw_with_uv_and_tint(
        &mut vertices,
        &mut commands,
        &renderer.hud_white_pixel,
        PhysicalSize::new(32, 20),
        absolute_hud_rect(20.0, 4.0, 2, 2),
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        [1.0, 0.0, 0.0, 1.0],
    );
    let (frame, counts) = render_commands(&renderer, &vertices, &commands);
    assert_eq!(counts, (3, 3));
    assert_eq!(frame.pixel(2, 3), [51, 102, 153, 255]);
    assert_highlight(frame.pixel(3, 3), [51, 102, 153, 255]);
    assert_eq!(frame.pixel(6, 3), [51, 102, 153, 255]);
    assert_eq!(frame.pixel(20, 4), [255, 0, 0, 255]);
}

#[test]
fn chat_input_gpu_highlight_uses_selected_fragment_and_discards_zero_alpha() {
    let Some(mut renderer) = Renderer::new_offscreen(32, 20) else {
        return;
    };
    let commands = [HudDrawCommand::TextHighlight {
        invert_start: 0,
        additive_start: 6,
        end: 12,
    }];
    let (frame, counts) = render_commands(&renderer, &highlight_quad(0.0), &commands);
    assert_eq!(counts, (2, 2));
    assert_eq!(frame.pixel(4, 4), [51, 102, 153, 255]);
    let shaders = crate::core_shaders::AdaptedShaderPair {
        vertex: "#version 450\nlayout(location=0) in vec2 Position; layout(location=2) in vec4 Color; layout(location=0) out vec4 vertexColor; void main() { gl_Position=vec4(Position,0.0,1.0); vertexColor=Color; }".into(),
        fragment: "#version 450\nlayout(location=0) in vec4 vertexColor; layout(location=0) out vec4 fragColor; void main() { if (vertexColor.a==0.0) discard; fragColor=vertexColor*vec4(1.0,0.0,0.0,1.0); }".into(),
    };
    renderer.pipelines.hud_gui = create_hud_gui_pipelines(
        &renderer.device,
        renderer.config.format,
        renderer.gui_projection_extent(),
        Some(&shaders),
    )
    .unwrap();
    let (frame, counts) = render_commands(&renderer, &highlight_quad(1.0), &commands);
    assert_eq!(counts, (2, 2));
    assert_eq!(frame.pixel(4, 4), [204, 102, 153, 255]);
}

#[test]
fn chat_input_gpu_highlight_updates_selected_vertex_gui_size_without_recreating_pipelines() {
    let Some(mut renderer) = Renderer::new_offscreen(32, 20) else {
        return;
    };
    let shaders = crate::core_shaders::AdaptedShaderPair {
        vertex: r#"#version 450
layout(std140,set=0,binding=0) uniform BbbGuiParameters { vec4 BbbGuiSize; };
layout(location=0) in vec2 Position;
layout(location=2) in vec4 Color;
layout(location=0) out vec4 vertexColor;
void main() {
    gl_Position=vec4(Position+vec2(2.0/BbbGuiSize.x,-2.0/BbbGuiSize.y),0.0,1.0);
    vertexColor=Color;
}"#.into(),
        fragment: "#version 450\nlayout(location=0) in vec4 vertexColor; layout(location=0) out vec4 fragColor; void main() { fragColor=vertexColor; }".into(),
    };
    renderer.pipelines.hud_gui = create_hud_gui_pipelines(
        &renderer.device,
        renderer.config.format,
        renderer.gui_projection_extent(),
        Some(&shaders),
    )
    .unwrap();
    let commands = [HudDrawCommand::TextHighlight {
        invert_start: 0,
        additive_start: 6,
        end: 12,
    }];
    for (size, shift) in [
        (PhysicalSize::new(32, 20), 1),
        (PhysicalSize::new(16, 10), 2),
    ] {
        renderer.set_gui_viewport(size, shift);
        let (frame, counts) = render_commands(&renderer, &highlight_quad(1.0), &commands);
        assert_eq!(counts, (2, 2));
        assert_eq!(frame.pixel(1 + shift, 2 + shift), [51, 102, 153, 255]);
        assert_eq!(frame.pixel(2 + shift, 1 + shift), [51, 102, 153, 255]);
        assert_highlight(frame.pixel(2 + shift, 2 + shift), [51, 102, 153, 255]);
    }
}

#[test]
fn chat_input_gpu_highlight_replaces_transparent_destination_alpha() {
    let Some(renderer) = Renderer::new_offscreen(32, 20) else {
        return;
    };
    let commands = [HudDrawCommand::TextHighlight {
        invert_start: 0,
        additive_start: 6,
        end: 12,
    }];
    let (frame, _) = render_commands_over(
        &renderer,
        &highlight_quad(1.0),
        &commands,
        wgpu::Color {
            r: 0.2,
            g: 0.4,
            b: 0.6,
            a: 0.0,
        },
    );
    assert_eq!(frame.pixel(0, 0), [51, 102, 153, 0]);
    assert_highlight(frame.pixel(4, 4), [51, 102, 153, 0]);
}

#[path = "gui_fill.rs"]
mod gui_fill;

#[path = "gui_editor_pixels.rs"]
mod gui_editor_pixels;

#[test]
fn book_sign_edit_box_layout_uses_raw_halves_and_fifteen_unit_insert_limit() {
    let map = glyphs(6.0);
    let raw = view("😀", 1, 1, 0);
    let layout = edit_box_layout(&raw, &[], &map, 114, 15, 114.0);
    assert_eq!(layout.head[0].text, "�");
    assert_eq!(layout.tail[0].text, "�");
    assert!(layout.insert);
    let full = view(&"A".repeat(15), 15, 15, 0);
    assert!(edit_box_layout(&full, &[], &map, 114, 15, 114.0).insert);
    let short = view("A", 1, 1, 0);
    assert!(!edit_box_layout(&short, &[], &map, 114, 15, 114.0).insert);
}

#[test]
fn chat_preedit_follows_scrolled_caret_and_shares_edge_layout_with_native_area() {
    let surface = PhysicalSize::new(100, 70);
    let map = glyphs(6.0);
    let glyphs = map.non_fishy();
    let text = "A".repeat(15);
    let mut state = HudCommandSuggestions {
        input_text: text.clone(),
        input_view: Some(view(&text, 1, 1, 0)),
        preedit: Some(HudImePreedit {
            text: "AB".into(),
            focused_start: 0,
            focused_end: 2,
            caret: Some(1),
            caret_visible: true,
        }),
        ..Default::default()
    };
    let start = chat_input::preedit_projection(&state, surface, glyphs).unwrap();
    assert_eq!((start.overlay.text_x, start.overlay.text_y), (10, 36));
    state.input_view = Some(view(&text, 15, 15, 0));
    let edge = chat_input::preedit_projection(&state, surface, glyphs).unwrap();
    assert_eq!((edge.overlay.text_x, edge.overlay.text_y), (88, 36));
    assert_eq!(
        (
            edge.overlay.background_x,
            edge.overlay.background_y,
            edge.overlay.background_width,
            edge.overlay.background_height
        ),
        (83, 31, 22, 19)
    );
    assert_eq!(edge.ime_position, PhysicalPosition::new(86, 34));
    assert_eq!(edge.ime_size, PhysicalSize::new(16, 36));
    state.input_view = Some(view(&text, 15, 15, 10));
    let scrolled = chat_input::preedit_projection(&state, surface, glyphs).unwrap();
    assert_eq!((scrolled.overlay.text_x, scrolled.overlay.text_y), (35, 36));
}
