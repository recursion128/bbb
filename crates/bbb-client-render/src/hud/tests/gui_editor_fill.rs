use super::*;

#[test]
fn inventory_item_decorations_keep_fill_colors_and_order() {
    let size = PhysicalSize::new(64, 64);
    let rect = absolute_hud_rect(8.0, 8.0, 16, 16);
    let mut vertices = Vec::new();
    let mut commands = Vec::new();
    push_hud_item_durability_bar(
        &mut vertices,
        &mut commands,
        size,
        rect,
        Some(&HudItemDurabilityBar::new(7, [0.2, 0.8, 0.1])),
    );
    push_hud_item_cooldown(&mut vertices, &mut commands, size, rect, Some(0.5));
    assert_eq!(vertices.len(), 18);
    assert!(commands
        .iter()
        .all(|command| matches!(command, HudDrawCommand::Fill { .. })));
    assert!(vertices[..6]
        .iter()
        .all(|v| v.tint == HUD_ITEM_BAR_BACKGROUND_TINT));
    assert!(vertices[6..12]
        .iter()
        .all(|v| v.tint == [0.2, 0.8, 0.1, 1.0]));
    assert!(vertices[12..]
        .iter()
        .all(|v| v.tint == HUD_ITEM_COOLDOWN_TINT));
    assert_eq!(
        vertices[12].position,
        hud_quad_vertices(
            size,
            hud_item_cooldown_rect(rect, 0.5).unwrap(),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0]
            },
            HUD_ITEM_COOLDOWN_TINT
        )[0]
        .position
    );
}

#[test]
fn inventory_zero_durability_and_absent_cooldown_only_draw_background() {
    let mut vertices = Vec::new();
    let mut commands = Vec::new();
    let size = PhysicalSize::new(64, 64);
    let rect = absolute_hud_rect(0.0, 0.0, 16, 16);
    push_hud_item_durability_bar(
        &mut vertices,
        &mut commands,
        size,
        rect,
        Some(&HudItemDurabilityBar::new(0, [1.0, 0.0, 0.0])),
    );
    push_hud_item_cooldown(&mut vertices, &mut commands, size, rect, None);
    assert_eq!(vertices.len(), 6);
    assert!(matches!(
        commands.as_slice(),
        [HudDrawCommand::Fill { start: 0, end: 6 }]
    ));
}

#[test]
fn inventory_shared_input_highlights_after_glyphs_before_insert_cursor() {
    let Some(mut renderer) = Renderer::new_offscreen(64, 64) else {
        return;
    };
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(
        'A',
        HudAsciiGlyph {
            width: 5,
            height: 8,
            advance: 6.0,
            ..Default::default()
        },
    );
    renderer
        .upload_hud_font_atlas(1, 1, &[255; 4], glyphs)
        .unwrap();
    let input = HudInventoryTextInputDecoration {
        cursor: 0,
        selection: 1,
        scroll_to: 0,
        max_length: 10,
        cursor_visible: true,
        cursor_tint: [0.4, 0.5, 0.6, 1.0],
    };
    let label = HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: 0,
        y: 0,
        width: 40,
        text: "AA".into(),
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
        PhysicalSize::new(64, 64),
        &label,
        input,
        (8.0, 8.0),
    );
    let highlight = commands
        .iter()
        .position(|c| matches!(c, HudDrawCommand::TextHighlight { .. }))
        .unwrap();
    assert!(highlight > 0);
    assert!(commands[..highlight]
        .iter()
        .all(|c| matches!(c, HudDrawCommand::Sprite { .. })));
    let HudDrawCommand::TextHighlight {
        invert_start,
        additive_start,
        end,
    } = commands[highlight]
    else {
        unreachable!()
    };
    assert!(vertices[invert_start as usize..additive_start as usize]
        .iter()
        .all(|v| v.tint == HUD_TINT_WHITE));
    assert!(vertices[additive_start as usize..end as usize]
        .iter()
        .all(|v| v.tint == [0.0, 0.0, 1.0, 1.0]));
    assert!(matches!(
        commands[highlight + 1],
        HudDrawCommand::Fill { .. }
    ));
    assert!(vertices[end as usize..]
        .iter()
        .all(|v| v.tint == input.cursor_tint));
}

#[test]
fn inventory_pause_gradient_uses_fill_and_retains_both_endpoint_tints() {
    let mut vertices = Vec::new();
    let mut commands = Vec::new();
    push_hud_pause_background(&mut vertices, &mut commands, PhysicalSize::new(64, 48));
    assert!(matches!(
        commands.as_slice(),
        [HudDrawCommand::Fill { start: 0, end: 6 }]
    ));
    assert_eq!(vertices[0].tint, HUD_PAUSE_BACKGROUND_TOP_TINT);
    assert_eq!(vertices[2].tint, HUD_PAUSE_BACKGROUND_BOTTOM_TINT);
}
