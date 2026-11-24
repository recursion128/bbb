use super::*;

fn editor_pixel_renderer(width: u32, height: u32) -> Option<Renderer> {
    let mut renderer = Renderer::new_offscreen(width, height)?;
    let mut glyphs = HudFontGlyphMap::new();
    for ch in ['A', 'B', '_', '?', ' ', 'c', '\u{a7}'] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                width: 2,
                height: 2,
                advance: 6.0,
                uv: HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                ..Default::default()
            },
        );
    }
    renderer
        .upload_hud_font_atlas(1, 1, &[255; 4], glyphs)
        .unwrap();
    renderer.set_clear_color(crate::ClearColor {
        r: 0.2,
        g: 0.4,
        b: 0.6,
        a: 1.0,
    });
    Some(renderer)
}

fn shared_input_pixels(
    renderer: &Renderer,
    selection: usize,
) -> crate::screenshot::ScreenshotPixels {
    let input = HudInventoryTextInputDecoration {
        cursor: 0,
        selection,
        scroll_to: 0,
        max_length: 10,
        cursor_visible: true,
        cursor_tint: [0.4, 0.5, 0.6, 1.0],
    };
    let label = HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: 0,
        y: 0,
        width: 24,
        text: "AB".into(),
        tint: HUD_TINT_WHITE,
        background: None,
        input: Some(input),
        scrolling: None,
        shadow: false,
        font: None,
        runs: Vec::new(),
    };
    let mut vertices = Vec::new();
    let mut commands = Vec::new();
    push_hud_inventory_text_input_label(
        &mut vertices,
        &mut commands,
        &renderer.hud_white_pixel,
        renderer.pack.hud_font_atlas.as_ref().unwrap(),
        renderer.pack.hud_font_book.glyphs(None),
        &renderer.pack.hud_obfuscated_glyph_pool,
        0,
        PhysicalSize::new(32, 20),
        &label,
        input,
        (4.0, 4.0),
    );
    render_commands(renderer, &vertices, &commands).0
}

#[test]
fn chat_input_gui_editor_shared_input_pixels_invert_glyph_then_cover_with_cursor() {
    let Some(renderer) = editor_pixel_renderer(32, 20) else {
        return;
    };
    let baseline = shared_input_pixels(&renderer, 0);
    let selected = shared_input_pixels(&renderer, 1);
    assert_eq!(baseline.pixel(5, 4), [255, 255, 255, 255]);
    assert_highlight(selected.pixel(5, 4), baseline.pixel(5, 4));
    assert_highlight(selected.pixel(6, 6), baseline.pixel(6, 6));
    assert_eq!(selected.pixel(4, 6), baseline.pixel(4, 6));
    assert_ne!(selected.pixel(4, 6), baseline.pixel(3, 6));
    assert_eq!(selected.pixel(10, 4), baseline.pixel(10, 4));
    assert_eq!(selected.pixel(10, 6), baseline.pixel(10, 6));
}

fn pixel_book(mode: HudBookMode, text: &str) -> HudBookScreen {
    HudBookScreen {
        editor_text: text.encode_utf16().collect(),
        editor_display_start: 0,
        generation: 1,
        page: 0,
        page_count: 1,
        resource_generation: 1,
        resources_loaded: true,
        text_filtering_enabled: false,
        advanced_item_tooltips: false,
        mode,
        runs: vec![HudComponentTextRun::plain(text)],
        interactions: Vec::new(),
        cursor: 0,
        selection: 0,
        cursor_visible: true,
        cursor_position: None,
        page_indicator: String::new(),
        title: String::new(),
        author: String::new(),
        warning: String::new(),
        done: String::new(),
        take_book: String::new(),
        may_build: true,
        sign: String::new(),
        finalize: String::new(),
        cancel: String::new(),
        finalize_enabled: true,
        preedit: None,
    }
}

#[test]
fn book_screen_dims_world_before_drawing_the_page() {
    let Some(mut renderer) = editor_pixel_renderer(320, 240) else {
        return;
    };
    let background = renderer.render_offscreen_frame().unwrap().pixel(4, 100);
    let mut book = pixel_book(HudBookMode::View, "A");
    book.done = "Done".into();
    renderer.set_hud_book_screen(Some(book));
    let page = renderer.render_offscreen_frame().unwrap();
    let dimmed = page.pixel(4, 100);
    for channel in 0..3 {
        assert!(dimmed[channel] < background[channel] / 2);
    }
}

#[test]
fn chat_input_gui_editor_book_page_pixels_highlight_both_lines_after_cursor() {
    let Some(mut renderer) = editor_pixel_renderer(320, 240) else {
        return;
    };
    renderer
        .upload_gui_texture(
            "textures/gui/book.png",
            256,
            256,
            &[255; 4].repeat(256 * 256),
            crate::StandaloneTextureSampling::default(),
        )
        .unwrap();
    let mut screen = pixel_book(HudBookMode::Edit, "AA\nBB");
    renderer.set_hud_book_screen(Some(screen.clone()));
    let baseline = renderer.render_offscreen_frame().unwrap();
    screen.selection = 4;
    renderer.set_hud_book_screen(Some(screen));
    let selected = renderer.render_offscreen_frame().unwrap();
    for (x, y) in [(100, 32), (100, 41), (99, 36), (102, 36)] {
        assert_highlight(selected.pixel(x, y), baseline.pixel(x, y));
    }
    assert_eq!(baseline.pixel(100, 32), [0, 0, 0, 255]);
    assert_eq!(baseline.pixel(100, 41), [0, 0, 0, 255]);
    assert_eq!(baseline.pixel(99, 36), [0, 0, 0, 255]);
    assert_eq!(selected.pixel(105, 41), baseline.pixel(105, 41));
    assert_eq!(selected.pixel(98, 36), baseline.pixel(98, 36));
    assert_eq!(selected.pixel(100, 50), baseline.pixel(100, 50));
}

#[test]
fn chat_input_gui_editor_book_title_pixels_cover_highlight_with_cursor() {
    let Some(mut renderer) = editor_pixel_renderer(320, 240) else {
        return;
    };
    renderer
        .upload_gui_texture(
            "textures/gui/book.png",
            256,
            256,
            &[255; 4].repeat(256 * 256),
            crate::StandaloneTextureSampling::default(),
        )
        .unwrap();
    let mut screen = pixel_book(HudBookMode::Sign, "AB");
    renderer.set_hud_book_screen(Some(screen.clone()));
    let baseline = renderer.render_offscreen_frame().unwrap();
    screen.selection = 1;
    renderer.set_hud_book_screen(Some(screen));
    let selected = renderer.render_offscreen_frame().unwrap();
    assert_eq!(baseline.pixel(152, 50), [0, 0, 0, 255]);
    assert_highlight(selected.pixel(152, 50), baseline.pixel(152, 50));
    assert_highlight(selected.pixel(153, 53), baseline.pixel(153, 53));
    assert_eq!(baseline.pixel(151, 53), [0, 0, 0, 255]);
    assert_eq!(selected.pixel(151, 53), baseline.pixel(151, 53));
    assert_eq!(selected.pixel(157, 50), baseline.pixel(157, 50));
    assert_eq!(selected.pixel(150, 53), baseline.pixel(150, 53));
}

fn pixel_sign(kind: HudSignEditorKind) -> HudSignEditorScreen {
    HudSignEditorScreen {
        kind,
        sign_preview: None,
        title: "A".into(),
        lines: [
            "AAAAAAAA".into(),
            String::new(),
            String::new(),
            String::new(),
        ],
        line: 0,
        cursor: 0,
        selection: 0,
        cursor_visible: true,
        text_tint: HUD_TINT_WHITE,
        done: "B".into(),
        done_hovered: false,
        preedit: None,
    }
}

#[test]
fn chat_input_gui_editor_sign_pixels_highlight_cursor_and_preserve_fractional_boundary() {
    let Some(mut renderer) = editor_pixel_renderer(320, 240) else {
        return;
    };
    for (kind, glyph_pixel, cursor_pixel, above_pixel, edge_pixel) in [
        (
            HudSignEditorKind::Standing {
                wood: SignModelWood::Oak,
                attachment: SignModelAttachment::Standing,
            },
            (138, 71),
            (137, 75),
            (137, 69),
            (142, 75),
        ),
        (
            HudSignEditorKind::Hanging {
                wood: SignModelWood::Oak,
            },
            (137, 108),
            (136, 112),
            (136, 106),
            (142, 112),
        ),
    ] {
        let mut screen = pixel_sign(kind);
        renderer.set_hud_sign_editor_screen(Some(screen.clone()));
        assert!(
            renderer.hud_sign_editor_screen.is_some(),
            "fixture passes production sanitization"
        );
        let baseline = renderer.render_offscreen_frame().unwrap();
        screen.selection = 1;
        renderer.set_hud_sign_editor_screen(Some(screen));
        let selected = renderer.render_offscreen_frame().unwrap();
        for (x, y) in [glyph_pixel, cursor_pixel] {
            assert_eq!(
                baseline.pixel(x, y),
                [255, 255, 255, 255],
                "sign fixture at {x},{y}; glyph={glyph_pixel:?} cursor={cursor_pixel:?}"
            );
            assert_highlight(selected.pixel(x, y), baseline.pixel(x, y));
        }
        for (x, y) in [above_pixel, edge_pixel, (160, 120)] {
            assert_eq!(selected.pixel(x, y), baseline.pixel(x, y));
        }
        assert_eq!(
            baseline.pixel(above_pixel.0, above_pixel.1),
            [255, 255, 255, 255]
        );
    }
}

#[test]
fn multiline_book_pixels_extend_newlines_and_empty_rows_and_clip_widget_edge() {
    let Some(mut renderer) = editor_pixel_renderer(320, 240) else {
        return;
    };
    renderer
        .upload_gui_texture(
            "textures/gui/book.png",
            256,
            256,
            &[255; 4].repeat(256 * 256),
            crate::StandaloneTextureSampling::default(),
        )
        .unwrap();
    let mut screen = pixel_book(HudBookMode::Edit, "AA\n\nBB");
    screen.cursor_visible = false;
    screen.cursor = 2;
    screen.selection = 2;
    renderer.set_hud_book_screen(Some(screen.clone()));
    let baseline = renderer.render_offscreen_frame().unwrap();
    screen.selection = 5;
    renderer.set_hud_book_screen(Some(screen.clone()));
    let selected = renderer.render_offscreen_frame().unwrap();
    for (x, y) in [(111, 36), (215, 36), (99, 45), (215, 45), (100, 50)] {
        assert_highlight(selected.pixel(x, y), baseline.pixel(x, y));
    }
    for (x, y) in [
        (110, 36),
        (216, 36),
        (216, 45),
        (98, 45),
        (105, 54),
        (100, 59),
    ] {
        assert_eq!(selected.pixel(x, y), baseline.pixel(x, y));
    }
    screen.cursor = 5;
    screen.selection = 2;
    renderer.set_hud_book_screen(Some(screen));
    let reversed = renderer.render_offscreen_frame().unwrap();
    for y in 29..62 {
        for x in 95..220 {
            assert_eq!(reversed.pixel(x, y), selected.pixel(x, y));
        }
    }
    let shaders = crate::core_shaders::AdaptedShaderPair {
        vertex: "#version 450\nlayout(location=0) in vec2 Position; layout(location=2) in vec4 Color; layout(location=0) out vec4 vertexColor; void main() { gl_Position=vec4(Position,0.0,1.0); vertexColor=Color; }".into(),
        fragment: "#version 450\nlayout(location=0) in vec4 vertexColor; layout(location=0) out vec4 fragColor; void main() { fragColor=vertexColor*vec4(1.0,0.0,0.0,1.0); }".into(),
    };
    renderer.pipelines.hud_gui = create_hud_gui_pipelines(
        &renderer.device,
        renderer.config.format,
        renderer.gui_projection_extent(),
        Some(&shaders),
    )
    .unwrap();
    let overridden = renderer.render_offscreen_frame().unwrap();
    let before = baseline.pixel(215, 45);
    assert_eq!(
        overridden.pixel(215, 45),
        [255 - before[0], before[1], before[2], 255]
    );
    assert_eq!(overridden.pixel(216, 45), baseline.pixel(216, 45));
}

#[test]
fn book_editor_pixels_consume_raw_utf16_without_display_runs() {
    let Some(mut renderer) = editor_pixel_renderer(320, 240) else {
        return;
    };
    renderer
        .upload_gui_texture(
            "textures/gui/book.png",
            256,
            256,
            &[255; 4].repeat(256 * 256),
            crate::StandaloneTextureSampling::default(),
        )
        .unwrap();
    let mut screen = pixel_book(HudBookMode::Edit, "");
    screen.editor_text = "§aA".encode_utf16().collect();
    screen.runs.clear();
    screen.cursor_visible = false;
    renderer.set_hud_book_screen(Some(screen));
    let frame = renderer.render_offscreen_frame().unwrap();
    assert_eq!(frame.pixel(100, 32), [85, 255, 85, 255]);
}

#[test]
fn book_sign_pixels_center_the_raw_scrolled_window() {
    let Some(mut renderer) = editor_pixel_renderer(320, 240) else {
        return;
    };
    renderer
        .upload_gui_texture(
            "textures/gui/book.png",
            256,
            256,
            &[255; 4].repeat(256 * 256),
            crate::StandaloneTextureSampling::default(),
        )
        .unwrap();
    let mut screen = pixel_book(HudBookMode::Sign, "AB");
    screen.editor_display_start = 1;
    screen.cursor_visible = false;
    renderer.set_hud_book_screen(Some(screen));
    let frame = renderer.render_offscreen_frame().unwrap();
    assert_eq!(frame.pixel(154, 50), [0, 0, 0, 255]);
    assert_eq!(frame.pixel(151, 50), [255, 255, 255, 255]);
}

/// `AbstractSignEditScreen` preloads each line as
/// `Component::getString`, so the sign editor buffer carries legacy codes and
/// `extractSignText` measures and draws it through the
/// `FormattedText.of` face: the pair is consumed and its colour reaches the
/// glyphs, unlike the chat input line's raw `FormattedCharSequence.forward`.
#[test]
fn chat_input_gui_editor_sign_pixels_consume_legacy_codes_in_the_line() {
    let Some(mut renderer) = editor_pixel_renderer(320, 240) else {
        return;
    };
    let mut screen = pixel_sign(HudSignEditorKind::Standing {
        wood: SignModelWood::Oak,
        attachment: SignModelAttachment::Standing,
    });
    screen.lines[0] = "\u{a7}cAA".into();
    screen.cursor = 4;
    screen.selection = 4;
    screen.cursor_visible = false;
    renderer.set_hud_sign_editor_screen(Some(screen));
    let frame = renderer
        .render_offscreen_frame()
        .expect("sign editor frame");
    let columns = (0..320)
        .filter(|x| (0..240).any(|y| frame.pixel(*x, y) == [255, 85, 85, 255]))
        .collect::<Vec<_>>();
    // Two glyphs of two pixels each, at the centering the resolved 12px width
    // asks for; the raw string would have drawn four glyph columns instead.
    assert_eq!(columns, vec![154, 155, 160, 161]);
    assert_eq!(
        hud_coded_string_runs("\u{a7}cAA"),
        vec![HudStyledTextRun {
            text: "AA".to_string(),
            color: Some(0xFF_5555),
            ..HudStyledTextRun::default()
        }],
        "the pair is consumed and sets the colour of the glyphs it precedes"
    );

    let mut glyphs = HudFontGlyphMap::new();
    for ch in ['A', '\u{a7}', 'c'] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                width: 2,
                height: 2,
                advance: 6.0,
                uv: HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                ..HudAsciiGlyph::default()
            },
        );
    }
    // The measured width is the advance of the two drawn glyphs; the raw face
    // charges the `§` and `c` glyphs it would draw as well.
    assert_eq!(
        hud_coded_string_width(&hud_coded_string_runs("\u{a7}cAA"), &glyphs),
        12
    );
    assert_eq!(hud_raw_string_width("\u{a7}cAA", &glyphs), 24);
}
