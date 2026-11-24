use super::*;

pub(super) fn project(
    preedit: &HudImePreedit,
    input: (i32, i32),
    input_height: i32,
    surface_size: PhysicalSize<u32>,
    glyphs: &HudFontGlyphMap,
) -> HudImePreeditProjection {
    let (input_left, input_top) = input;
    // `IMEPreeditOverlay`: `font.width(preEditText)` is the Component face.
    let preedit_width = hud_coded_string_width(&hud_coded_string_runs(&preedit.text), glyphs);
    let surface_width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let surface_height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let mut preedit_left = input_left;
    let mut preedit_right = preedit_left.saturating_add(preedit_width);
    if preedit_right > surface_width {
        preedit_left = surface_width.saturating_sub(preedit_width);
        preedit_right = preedit_left.saturating_add(preedit_width);
    }
    let input_bottom = input_top.saturating_add(input_height);
    let mut preedit_bottom = input_bottom
        .saturating_add(HUD_IME_PREEDIT_INPUT_SEPARATION)
        .saturating_add(HUD_IME_PREEDIT_TEXT_HEIGHT);
    if preedit_bottom > surface_height {
        preedit_bottom = input_top
            .saturating_sub(HUD_IME_PREEDIT_INPUT_SEPARATION)
            .saturating_sub(HUD_IME_PREEDIT_TEXT_HEIGHT);
    }
    let preedit_top = preedit_bottom.saturating_sub(HUD_IME_PREEDIT_TEXT_HEIGHT);
    let hot_left = preedit_left
        .min(input_left)
        .saturating_sub(HUD_IME_PREEDIT_HOT_AREA_MARGIN);
    let hot_top = preedit_top
        .min(input_top)
        .saturating_sub(HUD_IME_PREEDIT_HOT_AREA_MARGIN);
    let hot_right = preedit_right.saturating_add(HUD_IME_PREEDIT_HOT_AREA_MARGIN);
    let hot_bottom = preedit_bottom
        .max(input_bottom)
        .saturating_add(HUD_IME_PREEDIT_HOT_AREA_MARGIN);
    HudImePreeditProjection {
        overlay: HudInventoryPreeditOverlay {
            text: preedit.text.clone(),
            focused_start: preedit.focused_start,
            focused_end: preedit.focused_end,
            caret: preedit.caret,
            caret_visible: preedit.caret_visible,
            text_x: preedit_left,
            text_y: preedit_top,
            background_x: preedit_left.saturating_sub(HUD_IME_PREEDIT_BACKGROUND_OFFSET),
            background_y: preedit_top.saturating_sub(HUD_IME_PREEDIT_BACKGROUND_OFFSET),
            background_width: u32::try_from(
                preedit_width.saturating_add(HUD_IME_PREEDIT_BACKGROUND_OFFSET * 2),
            )
            .unwrap_or(u32::MAX),
            background_height: u32::try_from(
                HUD_IME_PREEDIT_TEXT_HEIGHT + HUD_IME_PREEDIT_BACKGROUND_OFFSET * 2,
            )
            .unwrap_or(u32::MAX),
        },
        ime_position: PhysicalPosition::new(hot_left, hot_top),
        ime_size: PhysicalSize::new(
            u32::try_from(hot_right.saturating_sub(hot_left).max(1)).unwrap_or(u32::MAX),
            u32::try_from(hot_bottom.saturating_sub(hot_top).max(1)).unwrap_or(u32::MAX),
        ),
    }
}

pub(super) fn text_field_caret_origin(
    field: &HudTextField,
    origin: (i32, i32),
    inner_width: u32,
    glyphs: &HudFontGlyphMap,
) -> (i32, i32) {
    let input = field.decoration(HUD_TINT_WHITE);
    let label = HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: 0,
        y: 0,
        width: inner_width,
        text: field.text.clone(),
        tint: HUD_TINT_WHITE,
        background: None,
        input: Some(input),
        scrolling: None,
        shadow: false,
        font: None,
        runs: Vec::new(),
    };
    let layout = utf16_widget_input_layout(&label, input, glyphs);
    (origin.0.saturating_add(layout.cursor_x as i32), origin.1)
}

pub(super) fn project_text_field(
    field: &HudTextField,
    origin: (i32, i32),
    inner_width: u32,
    input_height: i32,
    surface_size: PhysicalSize<u32>,
    glyphs: &HudFontGlyphMap,
) -> Option<HudImePreeditProjection> {
    let preedit = field.preedit.as_ref()?;
    let caret = text_field_caret_origin(field, origin, inner_width, glyphs);
    Some(project(preedit, caret, input_height, surface_size, glyphs))
}

/// Projects an inventory-anchored `HudTextField` preedit into a `HudInventoryPreeditOverlay`.
#[allow(clippy::too_many_arguments)]
pub fn hud_text_field_preedit_overlay(
    field: &HudTextField,
    screen_width: u32,
    screen_height: u32,
    label_x: i32,
    label_y: i32,
    inner_width: u32,
    input_height: i32,
    surface_size: PhysicalSize<u32>,
    glyphs: &HudFontGlyphMap,
) -> Option<HudInventoryPreeditOverlay> {
    let (origin_x, origin_y) = hud_inventory_text_label_origin(
        surface_size,
        screen_width,
        screen_height,
        label_x,
        label_y,
    );
    project_text_field(
        field,
        (origin_x as i32, origin_y as i32),
        inner_width,
        input_height,
        surface_size,
        glyphs,
    )
    .map(|projection| projection.overlay)
}

/// Caret or preedit cursor area for an inventory-anchored `HudTextField`.
#[allow(clippy::too_many_arguments)]
pub fn hud_inventory_text_field_ime_cursor_area(
    field: &HudTextField,
    screen_width: u32,
    screen_height: u32,
    label_x: i32,
    label_y: i32,
    inner_width: u32,
    input_height: i32,
    surface_size: PhysicalSize<u32>,
    glyphs: &HudFontGlyphMap,
) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
    let (origin_x, origin_y) = hud_inventory_text_label_origin(
        surface_size,
        screen_width,
        screen_height,
        label_x,
        label_y,
    );
    let origin = (origin_x as i32, origin_y as i32);
    if let Some(projection) = project_text_field(
        field,
        origin,
        inner_width,
        input_height,
        surface_size,
        glyphs,
    ) {
        return (projection.ime_position, projection.ime_size);
    }
    let (caret_x, caret_y) = text_field_caret_origin(field, origin, inner_width, glyphs);
    (
        PhysicalPosition::new(caret_x, caret_y),
        PhysicalSize::new(1, u32::try_from(input_height.max(1)).unwrap_or(10)),
    )
}

fn active_text_field_and_geometry(
    renderer: &Renderer,
    surface_size: PhysicalSize<u32>,
) -> Option<(&HudTextField, (i32, i32), u32, i32)> {
    if let Some(screen) = &renderer.hud_structure_block_editor_screen {
        let layout = hud_structure_block_editor_layout(surface_size);
        for (field, rect) in screen.fields.iter().zip(layout.fields.iter()) {
            if field.focused {
                return Some((
                    field,
                    (rect.x.saturating_add(4), rect.y.saturating_add(6)),
                    rect.width.saturating_sub(8),
                    10,
                ));
            }
        }
    }
    if let Some(screen) = &renderer.hud_jigsaw_block_editor_screen {
        let layout = hud_jigsaw_block_editor_layout(surface_size);
        for (field, rect) in screen.fields.iter().zip(layout.fields.iter()) {
            if field.focused {
                return Some((
                    field,
                    (rect.x.saturating_add(4), rect.y.saturating_add(6)),
                    rect.width.saturating_sub(8),
                    10,
                ));
            }
        }
    }
    if let Some(screen) = &renderer.hud_test_block_editor_screen {
        if screen.message_visible && screen.message.focused {
            let rect = hud_test_block_editor_layout(surface_size).message;
            return Some((
                &screen.message,
                (rect.x.saturating_add(4), rect.y.saturating_add(6)),
                rect.width.saturating_sub(8),
                10,
            ));
        }
    }
    if let Some(screen) = &renderer.hud_test_instance_block_editor_screen {
        let layout = hud_test_instance_block_editor_layout(surface_size);
        if screen.id.focused {
            return Some((
                &screen.id,
                (layout.id.x.saturating_add(4), layout.id.y.saturating_add(6)),
                layout.id.width.saturating_sub(8),
                10,
            ));
        }
        for (field, rect) in screen.size.iter().zip(layout.size.iter()) {
            if field.focused {
                return Some((
                    field,
                    (rect.x.saturating_add(4), rect.y.saturating_add(6)),
                    rect.width.saturating_sub(8),
                    10,
                ));
            }
        }
    }
    if let Some(HudSocialInteractionsScreen::Main(screen)) =
        &renderer.hud_social_interactions_screen
    {
        if screen.focus == HudSocialInteractionsFocus::Search {
            let layout = hud_social_interactions_layout(
                surface_size,
                screen.rows.len(),
                screen.scroll_amount,
                screen.page == HudSocialInteractionsPage::Blocked,
            );
            return Some((
                &screen.search,
                (
                    layout.search.x.saturating_add(4),
                    layout.search.y.saturating_add(4),
                ),
                u32::try_from(layout.search.width.saturating_sub(8)).unwrap_or_default(),
                10,
            ));
        }
    }
    if let Some(screen) = &renderer.hud_game_rules_screen {
        if screen.discard_confirmation.is_none() {
            let layout = hud_game_rules_layout(surface_size);
            if screen.search.focused {
                return Some((
                    &screen.search,
                    (
                        layout.search.x.saturating_add(4),
                        layout
                            .search
                            .y
                            .saturating_add((layout.search.height - 8) / 2),
                    ),
                    u32::try_from(layout.search.width.saturating_sub(8)).unwrap_or_default(),
                    10,
                ));
            }
            for (index, row) in screen.rows.iter().enumerate() {
                if let HudGameRulesRow::Integer { input, .. } = row {
                    if input.focused {
                        let rect = layout.value_rect(index, screen.scroll_amount);
                        return Some((
                            input,
                            (
                                rect.x.saturating_add(4),
                                rect.y.saturating_add((rect.height - 8) / 2),
                            ),
                            u32::try_from(rect.width.saturating_sub(8)).unwrap_or_default(),
                            10,
                        ));
                    }
                }
            }
        }
    }
    if let Some(screen) = &renderer.hud_debug_options_screen {
        if screen.search.focused {
            let (outer, _) = hud_debug_options_search_box_rects(surface_size);
            return Some((
                &screen.search,
                (outer.0.saturating_add(4), outer.1.saturating_add(6)),
                outer.2.saturating_sub(8),
                10,
            ));
        }
    }
    None
}

pub(super) fn active_screen_preedit_projection(
    renderer: &Renderer,
    surface_size: PhysicalSize<u32>,
) -> Option<HudImePreeditProjection> {
    let glyphs = &renderer.pack.hud_font_glyphs;
    if let Some(screen) = &renderer.hud_book_screen {
        if let Some(projection) = book::preedit_projection(screen, surface_size, glyphs) {
            return Some(projection);
        }
    }
    if let Some((field, origin, inner_width, input_height)) =
        active_text_field_and_geometry(renderer, surface_size)
    {
        return project_text_field(
            field,
            origin,
            inner_width,
            input_height,
            surface_size,
            glyphs,
        );
    }
    None
}

pub(super) fn active_screen_ime_cursor_area(
    renderer: &Renderer,
    surface_size: PhysicalSize<u32>,
) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>)> {
    let glyphs = &renderer.pack.hud_font_glyphs;
    if let Some(screen) = &renderer.hud_book_screen {
        if let Some(area) = book::ime_cursor_area(screen, surface_size, glyphs) {
            return Some(area);
        }
    }
    if let Some((field, origin, inner_width, input_height)) =
        active_text_field_and_geometry(renderer, surface_size)
    {
        if let Some(projection) = project_text_field(
            field,
            origin,
            inner_width,
            input_height,
            surface_size,
            glyphs,
        ) {
            return Some((projection.ime_position, projection.ime_size));
        }
        let (x, y) = text_field_caret_origin(field, origin, inner_width, glyphs);
        return Some((
            PhysicalPosition::new(x, y),
            PhysicalSize::new(1, u32::try_from(input_height.max(1)).unwrap_or(10)),
        ));
    }
    if let Some(screen) = &renderer.hud_inventory_screen {
        if let Some(overlay) = &screen.preedit_overlay {
            let hot_left = overlay
                .background_x
                .saturating_sub(HUD_IME_PREEDIT_HOT_AREA_MARGIN);
            let hot_top = overlay
                .background_y
                .saturating_sub(HUD_IME_PREEDIT_HOT_AREA_MARGIN);
            let hot_width = overlay
                .background_width
                .saturating_add(u32::try_from(HUD_IME_PREEDIT_HOT_AREA_MARGIN * 2).unwrap_or(0))
                .max(1);
            let hot_height = overlay
                .background_height
                .saturating_add(u32::try_from(HUD_IME_PREEDIT_HOT_AREA_MARGIN * 2).unwrap_or(0))
                .max(1);
            return Some((
                PhysicalPosition::new(hot_left, hot_top),
                PhysicalSize::new(hot_width, hot_height),
            ));
        }
        for label in &screen.text_labels {
            if let Some(input) = label.input {
                let (origin_x, origin_y) = hud_inventory_text_label_origin(
                    surface_size,
                    screen.width,
                    screen.height,
                    label.x,
                    label.y,
                );
                let layout = utf16_widget_input_layout(label, input, glyphs);
                return Some((
                    PhysicalPosition::new((origin_x + layout.cursor_x) as i32, origin_y as i32),
                    PhysicalSize::new(1, 10),
                ));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_glyphs() -> HudFontGlyphMap {
        let mut glyphs = HudFontGlyphMap::new();
        for ch in ' '..='~' {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    width: 5,
                    height: 8,
                    advance: 6.0,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        glyphs
    }

    #[test]
    fn command_block_and_text_field_preedit_project_near_caret() {
        let glyphs = test_glyphs();
        let surface = PhysicalSize::new(640, 360);
        let preedit = HudImePreedit {
            text: "nihao".to_string(),
            caret: Some(5),
            focused_start: 0,
            focused_end: 5,
            caret_visible: true,
        };

        let command_block = HudCommandSuggestions {
            preedit: Some(preedit.clone()),
            input_text: "say ".to_string(),
            input_view: None,
            draft: false,
            syntax: None,
            cursor: 4,
            selection: 4,
            cursor_visible: true,
            suffix: None,
            list_x: 0,
            list_y: 0,
            list_width: 0,
            list_height: 0,
            entries: Vec::new(),
            has_previous: false,
            has_next: false,
            tooltip: None,
            layout: HudCommandSuggestionsLayout::CommandBlock,
        };
        let sanitized = sanitize_hud_command_suggestions(command_block);
        assert!(
            sanitized.preedit.is_some(),
            "command block sanitizer must preserve IME preedit"
        );
        let cb_projection = chat_input::preedit_projection(&sanitized, surface, &glyphs)
            .expect("command block preedit projection");
        let cb_rect = hud_command_block_editor_layout(surface).command_input;
        assert!(cb_projection.overlay.text_x >= cb_rect.x + 4);
        assert!(cb_projection.overlay.text_y > cb_rect.y);

        let field = HudTextField {
            text: "abc".to_string(),
            cursor: 3,
            selection: 3,
            scroll_to: 3,
            max_length: 50,
            focused: true,
            hovered: false,
            cursor_visible: true,
            preedit: Some(preedit),
        };
        let overlay =
            hud_text_field_preedit_overlay(&field, 176, 166, 62, 24, 103, 10, surface, &glyphs)
                .expect("anvil/inventory text field preedit overlay");
        let (origin_x, origin_y) = hud_inventory_text_label_origin(surface, 176, 166, 62, 24);
        assert_eq!(overlay.text_x, origin_x as i32 + 19);
        assert!(overlay.text_y > origin_y as i32);

        let (idle_pos, idle_size) = hud_inventory_text_field_ime_cursor_area(
            &HudTextField {
                preedit: None,
                ..field
            },
            176,
            166,
            62,
            24,
            103,
            10,
            surface,
            &glyphs,
        );
        assert_eq!(
            idle_pos,
            PhysicalPosition::new(origin_x as i32 + 19, origin_y as i32)
        );
        assert_eq!(idle_size, PhysicalSize::new(1, 10));
    }
}
