use super::*;

fn multiline_area_renderer() -> Option<Renderer> {
    let mut renderer = Renderer::new_offscreen(64, 64)?;
    let mut glyphs = HudFontGlyphMap::new();
    for ch in ['A', 'B', '_', '?'] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                width: 2,
                height: 2,
                advance: 6.0,
                ..Default::default()
            },
        );
    }
    renderer
        .upload_hud_font_atlas(1, 1, &[255; 4], glyphs)
        .unwrap();
    Some(renderer)
}

fn multiline_area(text: &str) -> HudInventoryTextArea {
    HudInventoryTextArea {
        needs_cursor_position: false,
        x: 4,
        y: 4,
        width: 40,
        height: 36,
        text: text.into(),
        cursor: 0,
        selection: 0,
        scroll_amount: 0,
        max_length: text.encode_utf16().count(),
        cursor_visible: true,
        text_tint: HUD_TINT_WHITE,
        cursor_tint: [0.2, 0.4, 0.6, 1.0],
        counter_tint: [0.6, 0.4, 0.2, 1.0],
        shadow: false,
    }
}

fn multiline_area_draws<'a>(
    renderer: &'a Renderer,
    area: &HudInventoryTextArea,
) -> (Vec<HudVertex>, Vec<HudDrawCommand<'a>>) {
    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.width = 64;
    screen.height = 64;
    let mut vertices = Vec::new();
    let mut commands = Vec::new();
    push_hud_inventory_text_areas(
        &mut vertices,
        &mut commands,
        &renderer.hud_white_pixel,
        renderer.pack.hud_font_atlas.as_ref(),
        renderer.pack.hud_font_book.glyphs(None),
        &renderer.pack.hud_obfuscated_glyph_pool,
        0,
        &renderer.pack.gui_sprites,
        PhysicalSize::new(64, 64),
        &screen,
        std::slice::from_ref(area),
        HudInventoryItemScissor {
            x: 0,
            y: 0,
            width: 64,
            height: 64,
        },
    );
    (vertices, commands)
}

fn assert_multiline_quad(vertices: &[HudVertex], start: u32, rect: HudRect) {
    let expected = hud_quad_vertices(
        PhysicalSize::new(64, 64),
        rect,
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        HUD_TINT_WHITE,
    );
    for (actual, expected) in vertices[start as usize..][..6].iter().zip(expected) {
        assert_eq!(actual.position, expected.position);
    }
}

#[test]
fn multiline_inventory_selection_extends_newline_and_empty_line_after_cursor_and_text() {
    let Some(renderer) = multiline_area_renderer() else {
        return;
    };
    let mut area = multiline_area("A\n\nB");
    area.cursor = 1;
    area.selection = 4;
    let (vertices, commands) = multiline_area_draws(&renderer, &area);
    let cursor = commands
        .iter()
        .position(|command| matches!(command, HudDrawCommand::Fill { .. }))
        .unwrap();
    assert!(matches!(
        commands[cursor - 1],
        HudDrawCommand::Sprite { .. }
    ));
    assert!(matches!(
        commands[cursor + 1],
        HudDrawCommand::Sprite { .. }
    ));
    let highlights = commands
        .iter()
        .enumerate()
        .filter_map(|(index, command)| match command {
            HudDrawCommand::TextHighlight { invert_start, .. } => Some((index, *invert_start)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(highlights.len(), 3);
    assert_eq!(highlights[0].0, cursor + 2);
    for ((_, start), rect) in highlights.into_iter().zip([
        absolute_hud_rect(14.0, 8.0, 30, 9),
        absolute_hud_rect(8.0, 17.0, 36, 9),
        absolute_hud_rect(8.0, 26.0, 6, 9),
    ]) {
        assert_multiline_quad(&vertices, start, rect);
    }
}

#[test]
fn multiline_inventory_full_length_uses_append_cursor_on_last_visible_line() {
    let Some(renderer) = multiline_area_renderer() else {
        return;
    };
    let mut area = multiline_area("A\nB\nA");
    area.height = 10;
    area.cursor = area.text.chars().count();
    area.selection = area.cursor;
    let (vertices, _) = multiline_area_draws(&renderer, &area);
    let cursor = vertices
        .iter()
        .position(|vertex| vertex.tint == area.cursor_tint)
        .unwrap();
    assert_multiline_quad(&vertices, cursor as u32, absolute_hud_rect(14.0, 8.0, 2, 2));
    assert_eq!(
        vertices.iter().filter(|v| v.tint == area.text_tint).count(),
        6
    );
}

#[test]
fn multiline_inventory_scrolls_once_and_keeps_visible_wrapped_boundary_cursor() {
    let Some(renderer) = multiline_area_renderer() else {
        return;
    };
    let mut area = multiline_area("AAAAAA");
    area.width = 20;
    area.height = 10;
    area.scroll_amount = 14;
    area.cursor = 2;
    area.selection = 2;
    let (vertices, commands) = multiline_area_draws(&renderer, &area);
    let cursor = commands
        .iter()
        .find_map(|command| match command {
            HudDrawCommand::Fill { start, .. }
                if vertices[*start as usize].tint == area.cursor_tint =>
            {
                Some(*start)
            }
            _ => None,
        })
        .unwrap();
    assert_multiline_quad(&vertices, cursor, absolute_hud_rect(8.0, 2.0, 1, 11));
    assert_eq!(
        vertices.iter().filter(|v| v.tint == area.text_tint).count(),
        24
    );
}

#[test]
fn multiline_inventory_pixels_clip_continuing_empty_rows_after_scroll_and_parent_clip() {
    let Some(mut renderer) = multiline_area_renderer() else {
        return;
    };
    let mut screen = hud_inventory_screen_with_entity_previews(Vec::new());
    screen.width = 64;
    screen.height = 64;
    for (height, scroll, parent_width, y, right) in [(36, 0, 64, 20, 43), (18, 9, 40, 11, 40)] {
        let mut area = multiline_area("A\n\nB");
        area.height = height;
        area.scroll_amount = scroll;
        area.cursor_visible = false;
        area.cursor = 1;
        area.selection = 1;
        screen.scroll_area = Some(HudInventoryScrollArea {
            scissor: HudInventoryItemScissor {
                x: 0,
                y: 0,
                width: parent_width,
                height: 64,
            },
            background_layers: Vec::new(),
            fill_layers: Vec::new(),
            text_labels: Vec::new(),
            text_areas: vec![area],
            scrollbar: None,
        });
        renderer.set_hud_inventory_screen(Some(screen.clone()));
        assert!(renderer.hud_inventory_screen.is_some());
        let baseline = renderer.render_offscreen_frame().unwrap();
        screen.scroll_area.as_mut().unwrap().text_areas[0].selection = 4;
        renderer.set_hud_inventory_screen(Some(screen.clone()));
        let selected = renderer.render_offscreen_frame().unwrap();
        for x in [8, right - 1] {
            let before = baseline.pixel(x, y);
            assert_eq!(
                selected.pixel(x, y),
                [255 - before[0], 255 - before[1], 255, 255]
            );
        }
        for x in [7, right] {
            assert_eq!(selected.pixel(x, y), baseline.pixel(x, y));
        }
    }
}
