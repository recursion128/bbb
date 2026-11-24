use super::*;
use crate::hud::{inventory_draws::*, sanitize::*};

fn glyphs() -> HudFontGlyphMap {
    let mut result = HudFontGlyphMap::new();
    for (ch, advance, width) in [
        ('A', 4.0, 2),
        ('B', 4.0, 2),
        ('😀', 10.0, 8),
        ('\u{fffd}', 3.0, 2),
        ('_', 4.0, 2),
        ('\0', 2.0, 0),
        ('\t', 2.0, 0),
        ('1', 4.0, 2),
        ('/', 4.0, 2),
    ] {
        result.insert_first_wins(
            ch,
            HudAsciiGlyph {
                advance,
                width,
                height: 2,
                uv: HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                ..Default::default()
            },
        );
    }
    result
}
fn make_input(cursor: usize, selection: usize, visible: bool) -> HudInventoryTextInputDecoration {
    HudInventoryTextInputDecoration {
        cursor,
        selection,
        scroll_to: 0,
        max_length: 10000,
        cursor_visible: visible,
        cursor_tint: [0.2, 0.4, 0.6, 1.0],
    }
}
fn label(text: &str, input: HudInventoryTextInputDecoration) -> HudInventoryTextLabel {
    HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: 0,
        y: 0,
        width: 40,
        text: text.into(),
        tint: HUD_TINT_WHITE,
        background: None,
        input: Some(input),
        scrolling: None,
        shadow: false,
        font: None,
        runs: Vec::new(),
    }
}
fn area(text: &str, needs: bool, visible: bool) -> HudInventoryTextArea {
    HudInventoryTextArea {
        needs_cursor_position: needs,
        x: 4,
        y: 4,
        width: 22,
        height: 32,
        text: text.into(),
        cursor: 2,
        selection: 2,
        scroll_amount: 0,
        max_length: 10000,
        cursor_visible: visible,
        text_tint: HUD_TINT_WHITE,
        cursor_tint: [0.2, 0.4, 0.6, 1.0],
        counter_tint: [0.6, 0.4, 0.2, 1.0],
        shadow: false,
    }
}

#[test]
fn utf16_widget_cursor_and_selection_split_pair_independently_of_blink() {
    for visible in [false, true] {
        let input = make_input(2, 3, visible);
        let value = utf16_widget_input_layout(&label("A😀B", input), input, &glyphs());
        assert_eq!(value.split_text, Some(("A�".into(), "�B".into())));
        assert_eq!(value.cursor_x, 7.0);
        assert_eq!(value.selection_rect, Some((7.0, 6)));
        assert!(value.cursor_on_screen && value.insert_cursor);
    }
    let input = make_input(1, 2, false);
    let value = utf16_widget_input_layout(&label("A😀B", input), input, &glyphs());
    assert_eq!(value.split_text, Some(("A".into(), "😀B".into())));
    assert_eq!(value.cursor_x, 4.0);
    assert_eq!(value.selection_rect, Some((4.0, 2)));
    let input = make_input(3, 2, false);
    let value = utf16_widget_input_layout(&label("A😀B", input), input, &glyphs());
    assert_eq!(value.selection_rect, Some((6.0, 8)));
}

#[test]
fn utf16_widget_tail_scroll_and_isolated_display_units_keep_exact_geometry() {
    let mut input = make_input(2, 3, false);
    input.scroll_to = 2;
    let mut text = label("A😀B", input);
    text.width = 3;
    let value = utf16_widget_input_layout(&text, input, &glyphs());
    assert_eq!(value.displayed_text, "�");
    assert_eq!(value.split_text, Some(("".into(), "�".into())));
    assert_eq!(value.cursor_x, 0.0);
    let units = [0xd800, 0x41, 0xdc00, 0xd83d, 0xde00];
    let text = String::from_utf16_lossy(&units);
    assert_eq!(text.encode_utf16().count(), units.len());
    let input = make_input(4, 5, false);
    let value = utf16_widget_input_layout(&label(&text, input), input, &glyphs());
    assert_eq!(value.split_text, Some(("�A��".into(), "�".into())));
    assert_eq!(value.cursor_x, 13.0);
    assert_eq!(value.selection_rect, Some((13.0, 6)));
    assert_eq!(utf16_widget_slice(&[0xd83d, 0xde00], 0, 1), "�");
    assert_eq!(utf16_widget_slice(&[0xd83d, 0xde00], 1, 2), "�");
}

#[test]
fn utf16_widget_complete_pair_wrap_precedes_local_cursor_projection() {
    let (_, lines) = utf16_widget_lines("A😀B\n�", 14, &glyphs());
    assert_eq!(
        lines
            .iter()
            .map(|line| line.begin..line.end)
            .collect::<Vec<_>>(),
        vec![0..3, 3..4, 5..6]
    );
    let (units, lines) = utf16_widget_lines("A😀B", 14, &glyphs());
    assert_eq!(utf16_widget_slice(&units, lines[0].begin, 2), "A�");
    assert_eq!(utf16_widget_slice(&units, 2, lines[0].end), "�");
    assert_eq!(
        utf16_widget_slice(&units, lines[1].begin, lines[1].end),
        "B"
    );
}

#[test]
fn utf16_widget_sanitize_preserves_full_controls_counter_and_legal_maximum() {
    let text = format!("A\0\t😀�{}", "B".repeat(3000));
    let input = make_input(5, 6, true);
    let value = sanitize_hud_inventory_text_label(label(&text, input)).unwrap();
    assert_eq!(value.text, text);
    assert_eq!(value.input.unwrap().max_length, 10000);
    let layout = utf16_widget_input_layout(&value, value.input.unwrap(), &glyphs());
    assert_eq!(layout.cursor_x, 18.0);
    assert_eq!(layout.selection_rect, Some((18.0, 2)));
    let area = sanitize_hud_inventory_text_area(area(&text, true, false)).unwrap();
    assert_eq!(area.text, text);
    assert_eq!(area.max_length, 10000);
    assert_eq!(
        utf16_widget_counter(&area.text, area.max_length),
        "3006/10000"
    );
}

fn renderer() -> Option<Renderer> {
    let mut renderer = Renderer::new_offscreen(64, 64)?;
    renderer
        .upload_hud_font_atlas(1, 1, &[255; 4], glyphs())
        .unwrap();
    Some(renderer)
}
fn pixels(
    renderer: &Renderer,
    vertices: &[HudVertex],
    commands: &[HudDrawCommand<'_>],
) -> crate::screenshot::ScreenshotPixels {
    use wgpu::util::DeviceExt;
    let buffer = renderer
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("utf16 widget vertices"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
    let crate::frame_flight::RenderSurface::Offscreen(target) = &renderer.surface else {
        unreachable!()
    };
    let view = target.create_view(&Default::default());
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("utf16 widget pixels"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
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
        renderer.draw_hud_commands(&mut pass, &buffer, commands);
    }
    let pending = renderer
        .prepare_screenshot_copy(&mut encoder, target)
        .unwrap();
    renderer.queue.submit([encoder.finish()]);
    renderer.read_screenshot_pixels(pending).unwrap()
}
fn input_pixels(
    renderer: &Renderer,
    visible: bool,
    selection: usize,
) -> crate::screenshot::ScreenshotPixels {
    let input = make_input(2, selection, visible);
    let label = label("A😀B", input);
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
        PhysicalSize::new(64, 64),
        &label,
        input,
        (4.0, 4.0),
    );
    pixels(renderer, &vertices, &commands)
}
#[test]
fn utf16_widget_single_line_pixels_split_even_when_blink_is_off() {
    let Some(renderer) = renderer() else {
        return;
    };
    let split = input_pixels(&renderer, false, 2);
    assert_eq!(split.pixel(10, 4), [0, 0, 0, 255]);
    assert_eq!(split.pixel(11, 4), [255; 4]);
    let cursor = input_pixels(&renderer, true, 2);
    assert_eq!(cursor.pixel(11, 8), [51, 102, 153, 255]);
    let selected = input_pixels(&renderer, false, 3);
    assert_ne!(selected.pixel(12, 4), split.pixel(12, 4));
    assert_eq!(selected.pixel(17, 4), split.pixel(17, 4));
}
fn area_draws<'a>(
    renderer: &'a Renderer,
    area: &HudInventoryTextArea,
) -> (Vec<HudVertex>, Vec<HudDrawCommand<'a>>) {
    let screen = HudInventoryScreen {
        width: 64,
        height: 64,
        background_layers: Vec::new(),
        foreground_layers: Vec::new(),
        fill_layers: Vec::new(),
        slots: Vec::new(),
        floating_items: Vec::new(),
        foreground_items: Vec::new(),
        ghost_items: Vec::new(),
        entity_previews: Vec::new(),
        map_previews: Vec::new(),
        banner_pattern_previews: Vec::new(),
        text_labels: Vec::new(),
        scroll_area: None,
        preedit_overlay: None,
        hovered_slot_id: None,
        tooltip: None,
    };
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
#[test]
fn utf16_widget_multiline_pixels_follow_blink_and_ime_without_rewrapping_pair() {
    let Some(renderer) = renderer() else {
        return;
    };
    for (needs, visible, split) in [
        (false, false, false),
        (true, false, true),
        (false, true, true),
        (true, true, true),
    ] {
        let area = area("A😀B", needs, visible);
        let (vertices, commands) = area_draws(&renderer, &area);
        let result = pixels(&renderer, &vertices, &commands);
        assert_eq!(
            result.pixel(14, 8),
            if split { [0, 0, 0, 255] } else { [255; 4] }
        );
        assert_eq!(result.pixel(8, 17), [255; 4]);
        assert_eq!(
            commands
                .iter()
                .filter(|command| matches!(command, HudDrawCommand::Fill { .. }))
                .count(),
            usize::from(visible)
        );
        if visible {
            assert_eq!(result.pixel(15, 12), [51, 102, 153, 255]);
        }
    }
}

#[test]
fn utf16_widget_controls_keep_unit_cursor_position_in_pixels_after_sanitize() {
    let Some(renderer) = renderer() else {
        return;
    };
    let input = make_input(4, 4, true);
    let label = sanitize_hud_inventory_text_label(label("A\0\t😀B", input)).unwrap();
    assert_eq!(label.text.encode_utf16().count(), 6);
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
        PhysicalSize::new(64, 64),
        &label,
        label.input.unwrap(),
        (4.0, 4.0),
    );
    let result = pixels(&renderer, &vertices, &commands);
    assert_eq!(result.pixel(15, 8), [51, 102, 153, 255]);
    assert_eq!(result.pixel(18, 4), [255; 4]);
}

#[test]
fn utf16_widget_empty_append_and_full_limit_use_edit_box_cursor_offsets() {
    let mut input = make_input(0, 0, false);
    let value = utf16_widget_input_layout(&label("", input), input, &glyphs());
    assert_eq!(value.cursor_x, 0.0);
    assert!(!value.insert_cursor);
    input.max_length = 0;
    let value = utf16_widget_input_layout(&label("", input), input, &glyphs());
    assert_eq!(value.cursor_x, -1.0);
    assert!(value.insert_cursor);
    let mut input = make_input(1, 1, false);
    let value = utf16_widget_input_layout(&label("A", input), input, &glyphs());
    assert_eq!(value.cursor_x, 5.0);
    assert!(!value.insert_cursor);
    input.max_length = 1;
    let value = utf16_widget_input_layout(&label("A", input), input, &glyphs());
    assert_eq!(value.cursor_x, 4.0);
    assert!(value.insert_cursor);
    let mut narrow = glyphs();
    narrow.insert_first_wins('x', HudAsciiGlyph::empty(1.0));
    let input = make_input(0, 1, false);
    let value = utf16_widget_input_layout(&label("x", input), input, &narrow);
    assert_eq!(value.selection_rect, Some((0.0, 0)));
}

#[test]
fn utf16_widget_multiline_selection_measures_each_unit_prefix_independently() {
    let Some(renderer) = renderer() else {
        return;
    };
    let baseline_area = area("A😀B", false, false);
    let (vertices, commands) = area_draws(&renderer, &baseline_area);
    let baseline = pixels(&renderer, &vertices, &commands);
    for (cursor, selection, left, right) in [(1, 2, 12, 15), (2, 3, 15, 22)] {
        let mut selected_area = baseline_area.clone();
        selected_area.cursor = cursor;
        selected_area.selection = selection;
        let (vertices, commands) = area_draws(&renderer, &selected_area);
        let selected = pixels(&renderer, &vertices, &commands);
        assert_ne!(selected.pixel(left, 8), baseline.pixel(left, 8));
        assert_eq!(selected.pixel(left - 1, 8), baseline.pixel(left - 1, 8));
        assert_eq!(selected.pixel(right, 8), baseline.pixel(right, 8));
    }
}
