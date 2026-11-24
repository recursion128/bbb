use super::*;

pub(super) fn hud_mob_effect_overlay_instances(
    surface_size: PhysicalSize<u32>,
    effects: &[HudMobEffect],
) -> Vec<HudMobEffectOverlayInstance> {
    let mut ordered = effects
        .iter()
        .filter(|effect| effect.show_icon)
        .collect::<Vec<_>>();
    ordered.sort_by(|a, b| hud_mob_effect_natural_cmp(b, a));
    let mut beneficial_count = 0i32;
    let mut harmful_count = 0i32;
    ordered
        .into_iter()
        .map(|effect| {
            let (count, y) = if effect.beneficial {
                beneficial_count += 1;
                (beneficial_count, 1)
            } else {
                harmful_count += 1;
                (harmful_count, HUD_MOB_EFFECT_OVERLAY_HARMFUL_Y)
            };
            let x = i32::try_from(surface_size.width)
                .unwrap_or(i32::MAX)
                .saturating_sub(HUD_MOB_EFFECT_OVERLAY_SPACING.saturating_mul(count));
            HudMobEffectOverlayInstance {
                effect_id: effect.effect_id,
                ambient: effect.ambient,
                background: absolute_hud_rect(
                    x as f32,
                    y as f32,
                    HUD_MOB_EFFECT_BACKGROUND_SIZE,
                    HUD_MOB_EFFECT_BACKGROUND_SIZE,
                ),
                icon: absolute_hud_rect(
                    (x + 3) as f32,
                    (y + 3) as f32,
                    HUD_MOB_EFFECT_ICON_SIZE,
                    HUD_MOB_EFFECT_ICON_SIZE,
                ),
                icon_alpha: hud_mob_effect_icon_alpha(effect),
            }
        })
        .collect()
}

pub(super) fn push_hud_spyglass_scope<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    texture: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    scope: HudSpyglassScope,
) {
    let layout = hud_spyglass_scope_layout(surface_size, scope.scale);
    push_hud_draw(vertices, commands, texture, surface_size, layout.texture);
    for mask in layout.masks {
        push_hud_fill(vertices, commands, surface_size, mask, [0.0, 0.0, 0.0, 1.0]);
    }
}

pub(super) fn push_hud_sleep_overlay<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    surface_size: PhysicalSize<u32>,
    overlay: HudSleepOverlay,
) {
    push_hud_fill(
        vertices,
        commands,
        surface_size,
        absolute_hud_rect(
            0.0,
            0.0,
            surface_size.width.max(1),
            surface_size.height.max(1),
        ),
        hud_sleep_overlay_tint(overlay.alpha),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_system_toast<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    background: Option<&'a HudSpriteGpu>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    toast: &HudSystemToast,
    slot: u8,
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let (rect, x, _, _, y_offset) =
        hud_system_toast_instance_layout(surface_size, toast, slot, glyphs);
    if let Some(background) = background {
        push_hud_gui_sprite(vertices, commands, background, surface_size, rect);
    } else {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            white_pixel,
            surface_size,
            rect,
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            [0.15, 0.15, 0.15, 0.95],
        );
    }
    let title_y = if toast.message_lines.is_empty() {
        12.0
    } else {
        7.0
    };
    let mut text_lines = vec![(toast.title.as_str(), title_y, hud_argb_to_tint(0xFFFFFF00))];
    let message_lines = &toast.message_lines;
    text_lines.extend(
        message_lines
            .iter()
            .enumerate()
            .map(|(index, line)| (line.as_str(), 18.0 + index as f32 * 12.0, HUD_TINT_WHITE)),
    );
    for (text, y, tint) in text_lines {
        let runs = hud_coded_string_runs(text);
        let geometry = hud_styled_text_geometry(
            &runs,
            glyphs,
            obfuscated_pool,
            frame_index,
            (x + 18.0, y + y_offset),
            false,
            tint,
            None,
            1.0,
        );
        push_hud_styled_text_geometry(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            surface_size,
            &geometry,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_advancement_toasts<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    post_gui_item_commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    item_atlas: Option<&'a HudSpriteGpu>,
    digit_atlas: Option<&'a HudSpriteGpu>,
    digit_glyphs: &[HudDigitGlyph; 10],
    background: Option<&'a HudSpriteGpu>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    toasts: &[HudAdvancementToast],
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    for toast in toasts {
        let rect = hud_advancement_toast_rect(surface_size, toast);
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            background.unwrap_or(white_pixel),
            surface_size,
            rect,
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            if background.is_some() {
                HUD_TINT_WHITE
            } else {
                [0.15, 0.15, 0.15, 0.95]
            },
        );

        if let Some(item_atlas) = item_atlas {
            let renders_as_3d_block = toast.block_model.is_some();
            push_hud_item_icon(
                vertices,
                commands,
                item_atlas,
                digit_atlas,
                digit_glyphs,
                surface_size,
                hud_advancement_toast_icon_rect(surface_size, toast),
                &toast.icon,
                !renders_as_3d_block,
                false,
            );
        }

        let wrapped = hud_wrap_chat_runs_with_interactions(
            &toast.title_runs,
            &[],
            HUD_ADVANCEMENT_TOAST_TITLE_WIDTH,
            glyphs,
        );
        if wrapped.len() <= 1 {
            push_hud_advancement_toast_text(
                vertices,
                post_gui_item_commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                frame_index,
                surface_size,
                &[HudStyledTextRun::plain(&toast.frame_label)],
                (rect.x + 30.0, rect.y + 7.0),
                hud_argb_to_tint(0xFF00_0000 | toast.frame_label_color),
            );
            push_hud_advancement_toast_text(
                vertices,
                post_gui_item_commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                frame_index,
                surface_size,
                &toast.title_runs,
                (rect.x + 30.0, rect.y + 18.0),
                HUD_TINT_WHITE,
            );
            continue;
        }

        if toast.fully_visible_ms < 1_500 {
            let alpha = ((1_500 - toast.fully_visible_ms) as f32 / 300.0).clamp(0.0, 1.0);
            let mut tint = hud_argb_to_tint(0xFF00_0000 | toast.frame_label_color);
            tint[3] = alpha;
            push_hud_advancement_toast_text(
                vertices,
                post_gui_item_commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                frame_index,
                surface_size,
                &[HudStyledTextRun::plain(&toast.frame_label)],
                (rect.x + 30.0, rect.y + 11.0),
                tint,
            );
        } else {
            let alpha =
                ((toast.fully_visible_ms - 1_500) as f32 / 300.0).clamp(0.0, 1.0) * (252.0 / 255.0);
            let title_y =
                16i32.saturating_sub(i32::try_from(wrapped.len()).unwrap_or(i32::MAX) * 9 / 2);
            let y = rect.y + title_y as f32;
            for (index, line) in wrapped.iter().enumerate() {
                push_hud_advancement_toast_text(
                    vertices,
                    post_gui_item_commands,
                    white_pixel,
                    font_atlas,
                    glyphs,
                    obfuscated_pool,
                    frame_index,
                    surface_size,
                    &line.runs,
                    (rect.x + 30.0, y + index as f32 * 9.0),
                    [1.0, 1.0, 1.0, alpha],
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_stats_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    item_atlas: Option<&'a HudSpriteGpu>,
    digit_atlas: Option<&'a HudSpriteGpu>,
    digit_glyphs: &[HudDigitGlyph; 10],
    gui_sprites: &'a GuiSprites,
    menu_background: Option<&'a HudSpriteGpu>,
    tooltip_background: Option<&'a HudSpriteGpu>,
    tooltip_frame: Option<&'a HudSpriteGpu>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudStatsScreen,
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let screen_width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let list_x = screen_width / 2 - HUD_STATS_LIST_WIDTH / 2;
    let tab_rects = hud_stats_tab_rects(surface_size);
    for (index, (label, enabled)) in [
        (screen.general_tab.as_str(), true),
        (screen.items_tab.as_str(), screen.items_tab_enabled),
        (screen.mobs_tab.as_str(), screen.mobs_tab_enabled),
    ]
    .into_iter()
    .enumerate()
    {
        push_hud_tab_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            gui_sprites,
            menu_background,
            tab_rects[index],
            &hud_coded_string_runs(label),
            index == screen.active_tab,
            enabled,
        );
    }
    if screen.is_loading {
        let loading_y = i32::try_from(surface_size.height).unwrap_or(i32::MAX) / 2 - 4;
        push_hud_debug_options_centered_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &hud_coded_string_runs(&screen.loading_text),
            loading_y,
            HUD_TINT_WHITE,
        );
    } else if screen.active_tab == 1 {
        push_hud_stats_list_scissor(commands, surface_size);
        let header_y = HUD_STATS_LIST_TOP + HUD_STATS_LIST_ENTRY_PADDING + 1 - screen.scroll_amount;
        for column in 0..6 {
            let x = list_x + 57 + i32::try_from(column).unwrap_or(i32::MAX) * 40;
            let hovered = screen.item_hovered_column == Some(column)
                || screen.item_focused_column == Some(column);
            let background = hud_stats_header_background_sprite(hovered);
            let column_sprite = HudStatsSprite::column(column)
                .and_then(|sprite| gui_sprites.get(sprite.sprite_path()));
            if let (Some(background), Some(column_sprite)) =
                (gui_sprites.get(background.sprite_path()), column_sprite)
            {
                let rect = absolute_hud_rect(x as f32, header_y as f32, 18, 18);
                push_hud_draw(vertices, commands, background, surface_size, rect);
                push_hud_draw(vertices, commands, column_sprite, surface_size, rect);
            } else {
                let abbreviation = screen.item_column_labels[column]
                    .chars()
                    .next()
                    .map(|ch| ch.to_string())
                    .unwrap_or_else(|| "?".to_string());
                push_hud_pause_button(
                    vertices,
                    commands,
                    white_pixel,
                    font_atlas,
                    glyphs,
                    obfuscated_pool,
                    frame_index,
                    surface_size,
                    (x, header_y, 18, 18),
                    &hud_raw_string_runs(&abbreviation),
                    hovered,
                    true,
                    HudDebugOptionsButtonSprites::widget(gui_sprites),
                );
            }
            if screen.item_sort_column == Some(column) {
                let sort_sprite = hud_stats_sort_sprite(screen.item_sort_order);
                if let Some(sort_sprite) = gui_sprites.get(sort_sprite.sprite_path()) {
                    push_hud_draw(
                        vertices,
                        commands,
                        sort_sprite,
                        surface_size,
                        absolute_hud_rect((x - 18) as f32, header_y as f32, 18, 18),
                    );
                } else {
                    push_hud_debug_options_text(
                        vertices,
                        commands,
                        white_pixel,
                        font_atlas,
                        glyphs,
                        obfuscated_pool,
                        frame_index,
                        surface_size,
                        &hud_raw_string_runs(if screen.item_sort_order < 0 { "v" } else { "^" }),
                        ((x - 7) as f32, (header_y + 5) as f32),
                        HUD_TINT_WHITE,
                    );
                }
            }
        }
        for (index, row) in screen.item_rows.iter().enumerate() {
            let y = HUD_STATS_LIST_TOP
                + HUD_STATS_LIST_ENTRY_PADDING
                + 22
                + i32::try_from(screen.scroll_row.saturating_add(index))
                    .unwrap_or(i32::MAX / screen.row_height)
                    * screen.row_height
                - screen.scroll_amount;
            if let Some(slot) = gui_sprites.get(HudStatsSprite::Slot.sprite_path()) {
                push_hud_draw(
                    vertices,
                    commands,
                    slot,
                    surface_size,
                    absolute_hud_rect(list_x as f32, (y + 1) as f32, 18, 18),
                );
            } else {
                push_hud_draw_with_uv_and_tint(
                    vertices,
                    commands,
                    white_pixel,
                    surface_size,
                    absolute_hud_rect((list_x) as f32, (y + 1) as f32, 18, 18),
                    HudUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                    [0.12, 0.12, 0.12, 0.9],
                );
            }
            if let (Some(item_atlas), Some(icon)) = (item_atlas, &row.icon) {
                push_hud_item_icon(
                    vertices,
                    commands,
                    item_atlas,
                    digit_atlas,
                    digit_glyphs,
                    surface_size,
                    hud_stats_item_icon_rect(surface_size, screen, index),
                    icon,
                    row.block_model.is_none(),
                    false,
                );
            }
            let tint = if (screen.scroll_row + index).is_multiple_of(2) {
                HUD_TINT_WHITE
            } else {
                hud_argb_to_tint(0xFFBABABA)
            };
            for (column, value) in row.values.iter().enumerate() {
                let value = value.as_deref().unwrap_or(&screen.item_no_value);
                let right_x = list_x + 75 + i32::try_from(column).unwrap_or(i32::MAX) * 40;
                let runs = hud_coded_string_runs(value);
                let value_width = hud_coded_string_width(&runs, glyphs);
                push_hud_debug_options_text(
                    vertices,
                    commands,
                    white_pixel,
                    font_atlas,
                    glyphs,
                    obfuscated_pool,
                    frame_index,
                    surface_size,
                    &runs,
                    ((right_x - value_width) as f32, (y + 7) as f32),
                    tint,
                );
            }
        }
        push_hud_stats_list_scissor_reset(commands, surface_size);
        push_hud_stats_scrollbar(
            vertices,
            commands,
            white_pixel,
            gui_sprites,
            surface_size,
            screen,
        );
    } else {
        push_hud_stats_list_scissor(commands, surface_size);
        for (index, row) in screen.rows.iter().enumerate() {
            let y = HUD_STATS_LIST_TOP
                + HUD_STATS_LIST_ENTRY_PADDING
                + i32::try_from(screen.scroll_row.saturating_add(index))
                    .unwrap_or(i32::MAX / screen.row_height)
                    * screen.row_height
                - screen.scroll_amount;
            let tint = if (screen.scroll_row + index).is_multiple_of(2) {
                HUD_TINT_WHITE
            } else {
                hud_argb_to_tint(0xFFBABABA)
            };
            push_hud_debug_options_text(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                frame_index,
                surface_size,
                &hud_coded_string_runs(&row.label),
                ((list_x + 2) as f32, y as f32),
                tint,
            );
            let value_runs = hud_coded_string_runs(&row.value);
            let value_x = if row.secondary_value.is_some() {
                list_x + 12
            } else {
                list_x + HUD_STATS_LIST_WIDTH - hud_coded_string_width(&value_runs, glyphs) - 4
            };
            let value_y = y + i32::from(row.secondary_value.is_some()) * 9;
            push_hud_debug_options_text(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                frame_index,
                surface_size,
                &value_runs,
                (value_x as f32, value_y as f32),
                tint,
            );
            if let Some(secondary) = &row.secondary_value {
                push_hud_debug_options_text(
                    vertices,
                    commands,
                    white_pixel,
                    font_atlas,
                    glyphs,
                    obfuscated_pool,
                    frame_index,
                    surface_size,
                    &hud_coded_string_runs(secondary),
                    ((list_x + 12) as f32, (y + 18) as f32),
                    hud_argb_to_tint(0xFF808080),
                );
            }
        }
        push_hud_stats_list_scissor_reset(commands, surface_size);
        push_hud_stats_scrollbar(
            vertices,
            commands,
            white_pixel,
            gui_sprites,
            surface_size,
            screen,
        );
    }
    push_hud_pause_button(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        hud_stats_done_button_rect(surface_size),
        &hud_coded_string_runs(&screen.done),
        screen.done_hovered || screen.done_focused,
        true,
        HudDebugOptionsButtonSprites::widget(gui_sprites),
    );
    push_hud_debug_options_tooltip(
        vertices,
        commands,
        white_pixel,
        tooltip_background,
        tooltip_frame,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        screen.tooltip.as_ref(),
    );
}

#[allow(clippy::too_many_arguments)]
fn push_hud_tab_button<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    gui_sprites: &'a GuiSprites,
    menu_background: Option<&'a HudSpriteGpu>,
    (x, y, width, height): (i32, i32, i32, i32),
    label: &[HudStyledTextRun],
    selected: bool,
    active: bool,
) {
    let sprite = if selected {
        "widget/tab_selected"
    } else {
        "widget/tab"
    };
    if let Some(sprite) = gui_sprites.get(sprite) {
        push_hud_draw(
            vertices,
            commands,
            sprite,
            surface_size,
            absolute_hud_rect(x as f32, y as f32, width as u32, height as u32),
        );
    }
    let color = hud_argb_to_tint(if active { 0xFFFF_FFFF } else { 0xFFA0_A0A0 });
    if selected {
        if let Some(background) = menu_background {
            let (inner_width, inner_height) = ((width - 4) as u32, (height - 2) as u32);
            push_hud_draw_with_uv(
                vertices,
                commands,
                background,
                surface_size,
                absolute_hud_rect((x + 2) as f32, (y + 2) as f32, inner_width, inner_height),
                HudUvRect {
                    min: [0.0, 0.0],
                    max: [inner_width as f32 / 32.0, inner_height as f32 / 32.0],
                },
            );
        }
        let text_width = hud_font_runs_width(label, glyphs).unwrap_or_default();
        let underline = text_width.min(width - 4);
        push_hud_fill_rect(
            vertices,
            commands,
            surface_size,
            x + (width - underline) / 2,
            y + height - 2,
            underline.max(0) as u32,
            1,
            color,
        );
    }
    let top = y + if selected { 0 } else { 3 };
    push_hud_debug_options_centered_text_in_width(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        label,
        x + 1,
        width - 2,
        (top + y + height - 9) / 2 + 1,
        color,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_test_instance_block_editor_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    text_field: Option<&'a HudSpriteGpu>,
    text_field_highlighted: Option<&'a HudSpriteGpu>,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudTestInstanceBlockEditorScreen,
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let layout = hud_test_instance_block_editor_layout(surface_size);
    let label_tint = hud_argb_to_tint(0xFFA0A0A0);
    push_hud_debug_options_centered_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &hud_coded_string_runs(&screen.title),
        10,
        HUD_TINT_WHITE,
    );
    // `TestInstanceBlockEditScreen`: every label is a Component.
    for (label, x, y) in [
        (&screen.id_label, layout.id.x, 30),
        (&screen.size_label, layout.size[0].x, 150),
    ] {
        push_hud_coded_string_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &hud_coded_string_runs(label),
            (x as f32, y as f32),
            label_tint,
            1.0,
            true,
        );
    }

    let fields = std::iter::once((layout.id, &screen.id))
        .chain(layout.size.into_iter().zip(screen.size.iter()));
    for (rect, field) in fields {
        push_hud_edit_box(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            (text_field, text_field_highlighted),
            rect,
            field,
        );
    }

    let info_lines =
        hud_wrap_chat_runs_with_interactions(&screen.info_runs, &[], layout.info.width, glyphs);
    for (line_index, line) in info_lines.into_iter().take(8).enumerate() {
        let geometry = hud_styled_text_geometry(
            &line.runs,
            glyphs,
            obfuscated_pool,
            frame_index,
            (
                layout.info.x as f32,
                layout.info.y.saturating_add(line_index as i32 * 9) as f32,
            ),
            true,
            HUD_TINT_WHITE,
            Some(layout.info.width),
            1.0,
        );
        push_hud_styled_text_geometry(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            surface_size,
            &geometry,
        );
    }

    for (rect, label, hovered, enabled) in [
        (
            layout.rotation,
            screen.rotation.as_str(),
            screen.rotation_hovered,
            true,
        ),
        (
            layout.include_entities,
            screen.include_entities.as_str(),
            screen.include_entities_hovered,
            true,
        ),
        (
            layout.reset,
            screen.reset.as_str(),
            screen.reset_hovered,
            true,
        ),
        (
            layout.save,
            screen.save.as_str(),
            screen.save_hovered,
            screen.save_enabled,
        ),
        (layout.run, screen.run.as_str(), screen.run_hovered, true),
        (layout.done, screen.done.as_str(), screen.done_hovered, true),
        (
            layout.cancel,
            screen.cancel.as_str(),
            screen.cancel_hovered,
            true,
        ),
    ] {
        push_hud_debug_options_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            rect.x,
            rect.y,
            i32::try_from(rect.width).unwrap_or(i32::MAX),
            i32::try_from(rect.height).unwrap_or(i32::MAX),
            &hud_coded_string_runs(label),
            enabled,
            hovered,
            if enabled { HUD_TINT_WHITE } else { label_tint },
            button_sprites,
        );
    }
    for (label, rect) in [
        (&screen.rotation_label, layout.rotation),
        (&screen.entities_label, layout.include_entities),
    ] {
        push_hud_coded_string_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &hud_coded_string_runs(label),
            (rect.x as f32, 150.0),
            label_tint,
            1.0,
            true,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_command_suggestions<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    text_field_highlighted: Option<&'a HudSpriteGpu>,
    tooltip_background: Option<&'a HudSpriteGpu>,
    tooltip_frame: Option<&'a HudSpriteGpu>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    non_fishy_glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    state: &HudCommandSuggestions,
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let surface_height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let command_block_layout = hud_command_block_editor_layout(surface_size);
    let (input_x, input_y, input_width, input_height, text_origin, max_length) = match state.layout
    {
        HudCommandSuggestionsLayout::Chat => (
            2,
            surface_height.saturating_sub(14),
            surface_size.width.saturating_sub(4),
            12,
            chat_input::text_origin(surface_size),
            256,
        ),
        HudCommandSuggestionsLayout::CommandBlock => (
            command_block_layout.command_input.x,
            command_block_layout.command_input.y,
            command_block_layout.command_input.width,
            command_block_layout.command_input.height,
            (
                command_block_layout.command_input.x.saturating_add(4) as f32,
                command_block_layout.command_input.y.saturating_add(6) as f32,
            ),
            32_500,
        ),
    };
    if let Some(text_field_highlighted) =
        text_field_highlighted.filter(|_| state.layout == HudCommandSuggestionsLayout::CommandBlock)
    {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            text_field_highlighted,
            surface_size,
            absolute_hud_rect(input_x as f32, input_y as f32, input_width, input_height),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            HUD_TINT_WHITE,
        );
    } else if state.layout == HudCommandSuggestionsLayout::Chat {
        push_hud_fill(
            vertices,
            commands,
            surface_size,
            absolute_hud_rect(input_x as f32, input_y as f32, input_width, input_height),
            HUD_COMMAND_INPUT_BACKGROUND_TINT,
        );
    } else {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            white_pixel,
            surface_size,
            absolute_hud_rect(input_x as f32, input_y as f32, input_width, input_height),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            HUD_COMMAND_INPUT_BACKGROUND_TINT,
        );
    }
    if state.layout == HudCommandSuggestionsLayout::CommandBlock && text_field_highlighted.is_none()
    {
        for (x, y, width, height) in [
            (input_x, input_y, input_width, 1),
            (
                input_x,
                input_y.saturating_add_unsigned(input_height.saturating_sub(1)),
                input_width,
                1,
            ),
            (input_x, input_y, 1, input_height),
            (
                input_x.saturating_add_unsigned(input_width.saturating_sub(1)),
                input_y,
                1,
                input_height,
            ),
        ] {
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                white_pixel,
                surface_size,
                absolute_hud_rect((x) as f32, (y) as f32, width, height),
                HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                HUD_TINT_WHITE,
            );
        }
    }

    let input_glyphs = match state.layout {
        HudCommandSuggestionsLayout::Chat => non_fishy_glyphs,
        HudCommandSuggestionsLayout::CommandBlock => glyphs,
    };
    if let Some(view) = state
        .input_view
        .as_ref()
        .filter(|_| state.layout == HudCommandSuggestionsLayout::Chat)
    {
        chat_input::push_chat_input(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            input_glyphs,
            obfuscated_pool,
            obfuscated_seed,
            surface_size,
            view,
            &hud_command_input_runs(state),
            state.cursor_visible,
            state.suffix.as_deref(),
            text_origin,
            input_width,
        );
    } else {
        let input_runs = hud_command_input_runs(state);
        let label = HudInventoryTextLabel {
            component_runs: Vec::new(),
            x: 0,
            y: 0,
            width: if state.layout == HudCommandSuggestionsLayout::CommandBlock {
                input_width.saturating_sub(8)
            } else {
                input_width
            },
            text: state.input_text.clone(),
            tint: HUD_COMMAND_INPUT_TINT,
            background: None,
            input: Some(HudInventoryTextInputDecoration {
                cursor: state.cursor,
                selection: state.selection,
                scroll_to: state.cursor,
                max_length,
                cursor_visible: state.cursor_visible,
                cursor_tint: HUD_COMMAND_INPUT_TINT,
            }),
            scrolling: None,
            shadow: true,
            font: None,
            runs: input_runs,
        };
        let input = label.input.expect("command input decoration");
        let layout = utf16_widget_input_layout(&label, input, input_glyphs);
        push_hud_inventory_text_input_label(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            input_glyphs,
            obfuscated_pool,
            obfuscated_seed,
            surface_size,
            &label,
            input,
            text_origin,
        );
        if let Some(suffix) = state.suffix.as_deref() {
            // `EditBox` draws `this.suggestion` through the
            // `String` face, unlike the edit box value above it.
            push_hud_coded_string_text(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                input_glyphs,
                obfuscated_pool,
                obfuscated_seed,
                surface_size,
                &hud_coded_string_runs(suffix),
                (
                    text_origin.0 + (layout.cursor_x - 1.0).max(0.0),
                    text_origin.1,
                ),
                HUD_COMMAND_SUFFIX_TINT,
                1.0,
                true,
            );
        }
    }

    if !state.entries.is_empty() && state.list_width > 0 && state.list_height > 0 {
        let list_rect = absolute_hud_rect(
            state.list_x as f32,
            state.list_y as f32,
            state.list_width,
            state.list_height,
        );
        push_hud_fill(
            vertices,
            commands,
            surface_size,
            list_rect,
            HUD_COMMAND_LIST_BACKGROUND_TINT,
        );
        for (index, entry) in state.entries.iter().enumerate() {
            let y = state.list_y.saturating_add(
                i32::try_from(index)
                    .unwrap_or(i32::MAX)
                    .saturating_mul(HUD_COMMAND_LINE_HEIGHT),
            );
            push_hud_coded_string_text(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                obfuscated_seed,
                surface_size,
                &entry.runs,
                (
                    state.list_x.saturating_add(1) as f32,
                    y.saturating_add(2) as f32,
                ),
                if entry.selected {
                    HUD_COMMAND_SELECTED_TINT
                } else {
                    HUD_COMMAND_UNSELECTED_TINT
                },
                1.0,
                true,
            );
        }
        if state.has_previous || state.has_next {
            let top = state.list_y.saturating_sub(1);
            let bottom = state.list_y.saturating_add_unsigned(state.list_height);
            for y in [top, bottom] {
                push_hud_fill(
                    vertices,
                    commands,
                    surface_size,
                    absolute_hud_rect(state.list_x as f32, y as f32, state.list_width, 1),
                    HUD_COMMAND_LIST_BACKGROUND_TINT,
                );
            }
            for offset in (0..state.list_width).step_by(2) {
                for y in state
                    .has_previous
                    .then_some(top)
                    .into_iter()
                    .chain(state.has_next.then_some(bottom))
                {
                    push_hud_fill(
                        vertices,
                        commands,
                        surface_size,
                        absolute_hud_rect(
                            state.list_x.saturating_add_unsigned(offset) as f32,
                            y as f32,
                            1,
                            1,
                        ),
                        HUD_TINT_WHITE,
                    );
                }
            }
        }
    }

    if let Some(tooltip) = &state.tooltip {
        push_hud_command_suggestion_tooltip(
            vertices,
            commands,
            white_pixel,
            tooltip_background,
            tooltip_frame,
            font_atlas,
            glyphs,
            obfuscated_pool,
            obfuscated_seed,
            surface_size,
            tooltip,
        );
    }
}

/// Vanilla `Gui.extractTitle` fade: full alpha during the
/// stay window, `(fadeIn+stay+fadeOut - t) * 255 / fadeIn` while fading in,
/// `t * 255 / fadeOut` while fading out, clamped to 0..255. `t` is
/// `titleTime - partialTick`.
pub(super) fn hud_title_alpha(state: &HudTitleText) -> i32 {
    let t = state.remaining_ticks as f32 - state.partial_tick;
    let mut alpha = 255;
    if state.remaining_ticks > state.fade_out.saturating_add(state.stay) {
        let total = state
            .fade_in
            .saturating_add(state.stay)
            .saturating_add(state.fade_out);
        alpha = ((total as f32 - t) * 255.0 / state.fade_in as f32) as i32;
    }
    if state.remaining_ticks <= state.fade_out {
        alpha = (t * 255.0 / state.fade_out as f32) as i32;
    }
    alpha.clamp(0, 255)
}

/// Resolves the action-bar overlay message into a centered draw, mirroring
/// `Gui.extractOverlayMessage`: shown only while
/// `overlayMessageTime > 0`, alpha-gated (`alpha > 0`), centered above the
/// hotbar at `(guiWidth/2 - width/2, guiHeight - 68 - 4)`, white (or the
/// jukebox rainbow colour) at the fade alpha.
pub(super) fn hud_action_bar_text_draw<'a>(
    state: &'a HudActionBarText,
    fonts: &HudFontGlyphBook,
    surface_size: PhysicalSize<u32>,
    backdrop_opacity: f32,
) -> Option<HudComponentScreenTextDraw<'a>> {
    if state.remaining_ticks <= 0 {
        return None;
    }
    let t = state.remaining_ticks as f32 - state.partial_tick;
    let alpha = hud_overlay_message_alpha(t);
    if alpha <= 0 {
        return None;
    }
    let [red, green, blue] = if state.animate_color {
        hud_overlay_message_rainbow_rgb(t)
    } else {
        [1.0, 1.0, 1.0]
    };
    let width = hud_component_runs_width(&state.runs, fonts).unwrap_or(0);
    Some(HudComponentScreenTextDraw {
        runs: &state.runs,
        origin: hud_overlay_message_text_origin(surface_size, width),
        scale: 1.0,
        tint: [red, green, blue, alpha as f32 / 255.0],
        backdrop_opacity,
    })
}

/// Resolves the title (4x) and subtitle (2x) overlay lines, mirroring
/// `Gui.extractTitle`: shown only while `titleTime > 0`
/// and the fade alpha is `> 0`; both lines share the screen-center pose and
/// the same `ARGB.white(alpha)` tint, and the subtitle draws only while a
/// title is active (an empty title line still keeps the subtitle visible,
/// matching vanilla's non-null check).
pub(super) fn hud_title_text_draws<'a>(
    state: &'a HudTitleText,
    fonts: &HudFontGlyphBook,
    surface_size: PhysicalSize<u32>,
    backdrop_opacity: f32,
) -> Vec<HudComponentScreenTextDraw<'a>> {
    let mut draws = Vec::new();
    if state.remaining_ticks <= 0 {
        return draws;
    }
    let alpha = hud_title_alpha(state);
    if alpha <= 0 {
        return draws;
    }
    let tint = [1.0, 1.0, 1.0, alpha as f32 / 255.0];
    let title_width = hud_component_runs_width(&state.title_runs, fonts).unwrap_or(0);
    draws.push(HudComponentScreenTextDraw {
        runs: &state.title_runs,
        origin: hud_title_text_origin(surface_size, title_width),
        scale: HUD_TITLE_TEXT_SCALE,
        tint,
        backdrop_opacity,
    });
    if !state.subtitle_runs.is_empty() {
        let subtitle_width = hud_component_runs_width(&state.subtitle_runs, fonts).unwrap_or(0);
        draws.push(HudComponentScreenTextDraw {
            runs: &state.subtitle_runs,
            origin: hud_subtitle_text_origin(surface_size, subtitle_width),
            scale: HUD_SUBTITLE_TEXT_SCALE,
            tint,
            backdrop_opacity,
        });
    }
    draws
}

/// Picks a food icon's sprite honoring the Hunger potion swap (vanilla
/// `Gui.extractFood` sprite-id selection, Gui): under the effect
/// prefer the hunger variant, falling back to the base sprite when the variant
/// is not loaded; without the effect always use the base sprite. Generic over
/// the sprite handle so the decision is unit-testable without GPU resources.
pub(super) fn hud_food_sprite_variant<T>(
    hunger_effect: bool,
    hunger: Option<T>,
    base: Option<T>,
) -> Option<T> {
    if hunger_effect {
        hunger.or(base)
    } else {
        base
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_inventory_tooltip<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    tooltip_background: Option<&'a HudSpriteGpu>,
    tooltip_frame: Option<&'a HudSpriteGpu>,
    tooltip_styles: &'a GuiSprites,
    item_atlas: Option<&'a HudSpriteGpu>,
    font_atlas: Option<&'a HudSpriteGpu>,
    digit_glyphs: &[HudDigitGlyph; 10],
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    bundle_tooltip: Option<&HudBundleTooltip>,
) {
    let (Some(font_atlas), Some(tooltip)) = (font_atlas, screen.tooltip.as_ref()) else {
        return;
    };
    // Item name and lore lines are Components drawn through
    // `setTooltipForNextFrame`, so the
    // legacy codes resolve before the width and the glyphs are read.
    let resolved = tooltip
        .lines
        .iter()
        .map(|line| hud_resolved_tooltip_runs(&line.runs))
        .collect::<Vec<_>>();
    let Some(bundle) = bundle_tooltip else {
        push_hud_resolved_tooltip(
            vertices,
            commands,
            white_pixel,
            tooltip_background,
            tooltip_frame,
            tooltip_styles,
            font_atlas,
            fonts,
            obfuscated_seed,
            surface_size,
            screen.width,
            screen.height,
            tooltip.x,
            tooltip.y,
            tooltip.style.as_deref(),
            tooltip
                .lines
                .iter()
                .zip(&resolved)
                .map(|(line, runs)| (runs.as_slice(), line.tint)),
        );
        return;
    };
    push_hud_resolved_bundle_tooltip(
        vertices,
        commands,
        white_pixel,
        tooltip_background,
        tooltip_frame,
        tooltip_styles,
        item_atlas,
        font_atlas,
        digit_glyphs,
        fonts,
        obfuscated_seed,
        surface_size,
        screen.width,
        screen.height,
        tooltip,
        &resolved,
        bundle,
    );
}

// Vanilla `ClientBundleTooltip.java:20-33, 58-222` layout constants.
const BUNDLE_SLOT_MARGIN: f32 = 4.0;
const BUNDLE_SLOT_SIZE: u32 = 24;
const BUNDLE_GRID_WIDTH: u32 = 96;
const BUNDLE_PROGRESSBAR_HEIGHT: u32 = 13;
const BUNDLE_PROGRESSBAR_WIDTH: u32 = 96;
const BUNDLE_PROGRESSBAR_FILL_MAX: u32 = 94;
const BUNDLE_PROGRESSBAR_MARGIN_Y: u32 = 4;
const BUNDLE_EMPTY_DESC_LINE_HEIGHT: u32 = 9;
const BUNDLE_EMPTY_DESC_TINT: [f32; 4] = [170.0 / 255.0, 170.0 / 255.0, 170.0 / 255.0, 1.0];

#[allow(clippy::too_many_arguments)]
fn push_hud_resolved_bundle_tooltip<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    tooltip_background: Option<&'a HudSpriteGpu>,
    tooltip_frame: Option<&'a HudSpriteGpu>,
    tooltip_styles: &'a GuiSprites,
    item_atlas: Option<&'a HudSpriteGpu>,
    font_atlas: &'a HudSpriteGpu,
    digit_glyphs: &[HudDigitGlyph; 10],
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
    tooltip: &HudInventoryTooltip,
    resolved: &[Vec<HudComponentTextRun>],
    bundle: &HudBundleTooltip,
) {
    let empty_desc_lines = if bundle.items.is_empty() && !bundle.empty_description.is_empty() {
        let raw = [HudComponentTextRun::plain(
            bundle.empty_description.as_str(),
        )];
        hud_wrap_component_runs_with_interactions(&raw, &[], BUNDLE_GRID_WIDTH, fonts)
            .into_iter()
            .map(|wrapped| wrapped.runs)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let is_overflowing = bundle.hidden_item_count > 0;
    let shown_count = bundle.items.len().min(if is_overflowing { 11 } else { 12 });
    let slot_count = if is_overflowing { 12 } else { shown_count };
    let grid_rows = if bundle.items.is_empty() {
        0u32
    } else {
        (slot_count as u32).div_ceil(4)
    };
    let empty_desc_height =
        (empty_desc_lines.len() as u32).saturating_mul(BUNDLE_EMPTY_DESC_LINE_HEIGHT);
    let bundle_height = if bundle.items.is_empty() {
        empty_desc_height
            .saturating_add(BUNDLE_PROGRESSBAR_HEIGHT)
            .saturating_add(8)
    } else {
        grid_rows
            .saturating_mul(BUNDLE_SLOT_SIZE)
            .saturating_add(BUNDLE_PROGRESSBAR_HEIGHT)
            .saturating_add(8)
    };

    let max_text_width = resolved
        .iter()
        .filter_map(|runs| hud_component_runs_width(runs, fonts))
        .max()
        .unwrap_or(0);
    let total_width = max_text_width.max(BUNDLE_GRID_WIDTH as i32);
    // Vanilla `GuiGraphicsExtractor.java:1129, 1222-1231`: `optionalImage` is inserted after
    // line 0, so a tooltip with `n >= 1` text lines + 1 bundle component has >= 2 components
    // and total height `n * 10 + bundle_height`.
    let total_height = (tooltip.lines.len() as u32)
        .saturating_mul(10)
        .saturating_add(bundle_height);

    let background_rect = hud_inventory_tooltip_background_hud_rect(
        surface_size,
        screen_width,
        screen_height,
        tooltip.x,
        tooltip.y,
        total_width,
        total_height,
    );
    let origin_x = background_rect.x + 12.0;
    let origin_y = background_rect.y + 12.0;

    let sprites = match tooltip.style.as_deref() {
        Some(style) => tooltip_styles.tooltip_style(style),
        None => tooltip_background.zip(tooltip_frame),
    };
    if let Some((background, frame)) = sprites {
        push_hud_gui_sprite(
            vertices,
            commands,
            background,
            surface_size,
            background_rect,
        );
        push_hud_gui_sprite(vertices, commands, frame, surface_size, background_rect);
    } else {
        push_hud_inventory_tooltip_flat_background(
            vertices,
            commands,
            white_pixel,
            surface_size,
            background_rect,
        );
    }

    // Draw line 0 (item title) above the bundle component.
    if let (Some(line0), Some(runs0)) = (tooltip.lines.first(), resolved.first()) {
        let geometry = hud_component_text_geometry(
            runs0,
            fonts,
            obfuscated_seed,
            (origin_x, origin_y),
            true,
            line0.tint,
            None,
            1.0,
        );
        push_hud_styled_text_geometry(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            surface_size,
            &geometry,
        );
    }

    let bundle_y = origin_y + if tooltip.lines.is_empty() { 0.0 } else { 12.0 };
    let content_left = origin_x + ((total_width - BUNDLE_GRID_WIDTH as i32) / 2) as f32;

    if bundle.items.is_empty() {
        for (row_index, runs) in empty_desc_lines.iter().enumerate() {
            let line_y = bundle_y + (row_index as u32 * BUNDLE_EMPTY_DESC_LINE_HEIGHT) as f32;
            let geometry = hud_component_text_geometry(
                runs,
                fonts,
                obfuscated_seed,
                (content_left, line_y),
                true,
                BUNDLE_EMPTY_DESC_TINT,
                None,
                1.0,
            );
            push_hud_styled_text_geometry(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                surface_size,
                &geometry,
            );
        }
        let bar_y = bundle_y + (empty_desc_height + BUNDLE_PROGRESSBAR_MARGIN_Y) as f32;
        push_bundle_progress_bar(
            vertices,
            commands,
            white_pixel,
            tooltip_styles,
            font_atlas,
            fonts,
            obfuscated_seed,
            surface_size,
            content_left,
            bar_y,
            bundle.occupancy,
            bundle.progress_label.as_deref(),
        );
    } else {
        let x_start = content_left + BUNDLE_GRID_WIDTH as f32;
        let y_start = bundle_y + (grid_rows * BUNDLE_SLOT_SIZE) as f32;
        let slot_bg = tooltip_styles.get("container/bundle/slot_background");
        let slot_highlight_back = tooltip_styles.get("container/bundle/slot_highlight_back");
        let slot_highlight_front = tooltip_styles.get("container/bundle/slot_highlight_front");
        let mut slot_number = 1usize;
        for row_number in 1..=grid_rows {
            for col_number in 1..=4u32 {
                let draw_x = x_start - (col_number * BUNDLE_SLOT_SIZE) as f32;
                let draw_y = y_start - (row_number * BUNDLE_SLOT_SIZE) as f32;
                if is_overflowing && col_number == 1 && row_number == 1 {
                    let text = format!("+{}", bundle.hidden_item_count);
                    push_bundle_centered_text(
                        vertices,
                        commands,
                        white_pixel,
                        font_atlas,
                        fonts,
                        obfuscated_seed,
                        surface_size,
                        &text,
                        draw_x + 12.0,
                        draw_y + 10.0,
                    );
                } else if shown_count >= slot_number {
                    let item_index = shown_count - slot_number;
                    let highlighted = bundle.selected_index == Some(item_index);
                    let slot_rect = HudRect {
                        x: draw_x,
                        y: draw_y,
                        width: BUNDLE_SLOT_SIZE,
                        height: BUNDLE_SLOT_SIZE,
                    };
                    if highlighted {
                        if let Some(sprite) = slot_highlight_back {
                            push_hud_gui_sprite(
                                vertices,
                                commands,
                                sprite,
                                surface_size,
                                slot_rect,
                            );
                        } else {
                            push_hud_fill(
                                vertices,
                                commands,
                                surface_size,
                                slot_rect,
                                [0.38, 0.42, 0.72, 0.85],
                            );
                        }
                    } else if let Some(sprite) = slot_bg {
                        push_hud_gui_sprite(vertices, commands, sprite, surface_size, slot_rect);
                    } else {
                        push_hud_fill(
                            vertices,
                            commands,
                            surface_size,
                            slot_rect,
                            [0.18, 0.18, 0.22, 0.85],
                        );
                    }
                    if let Some(item_atlas) = item_atlas {
                        let item_rect = HudRect {
                            x: draw_x + BUNDLE_SLOT_MARGIN,
                            y: draw_y + BUNDLE_SLOT_MARGIN,
                            width: 16,
                            height: 16,
                        };
                        push_hud_item_icon(
                            vertices,
                            commands,
                            item_atlas,
                            Some(font_atlas),
                            digit_glyphs,
                            surface_size,
                            item_rect,
                            &bundle.items[item_index],
                            true,
                            true,
                        );
                    }
                    if highlighted {
                        if let Some(sprite) = slot_highlight_front {
                            push_hud_gui_sprite(
                                vertices,
                                commands,
                                sprite,
                                surface_size,
                                slot_rect,
                            );
                        }
                    }
                    slot_number += 1;
                }
            }
        }

        let bar_y = bundle_y + (grid_rows * BUNDLE_SLOT_SIZE + BUNDLE_PROGRESSBAR_MARGIN_Y) as f32;
        push_bundle_progress_bar(
            vertices,
            commands,
            white_pixel,
            tooltip_styles,
            font_atlas,
            fonts,
            obfuscated_seed,
            surface_size,
            content_left,
            bar_y,
            bundle.occupancy,
            bundle.progress_label.as_deref(),
        );

        // Selected item hover name tooltip above the bundle grid (`ClientBundleTooltip.java:172-189`).
        if let Some(selected_line) = bundle.selected_item_name.as_ref() {
            let selected_runs = hud_resolved_tooltip_runs(&selected_line.runs);
            let name_width = hud_component_runs_width(&selected_runs, fonts).unwrap_or(0);
            let center_x = origin_x as i32 + total_width / 2 - 12;
            push_hud_resolved_tooltip(
                vertices,
                commands,
                white_pixel,
                tooltip_background,
                tooltip_frame,
                tooltip_styles,
                font_atlas,
                fonts,
                obfuscated_seed,
                surface_size,
                surface_size.width,
                surface_size.height,
                center_x - name_width / 2,
                bundle_y as i32 - 15,
                bundle.selected_item_tooltip_style.as_deref(),
                std::iter::once((selected_runs.as_slice(), selected_line.tint)),
            );
        }
    }

    // Draw remaining tooltip text lines (lore / advanced tags) below the bundle component.
    for (extra_index, (line, runs)) in tooltip.lines.iter().zip(resolved).skip(1).enumerate() {
        let line_y = bundle_y + bundle_height as f32 + (extra_index as u32 * 10) as f32;
        let geometry = hud_component_text_geometry(
            runs,
            fonts,
            obfuscated_seed,
            (origin_x, line_y),
            true,
            line.tint,
            None,
            1.0,
        );
        push_hud_styled_text_geometry(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            surface_size,
            &geometry,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_bundle_progress_bar<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    tooltip_styles: &'a GuiSprites,
    font_atlas: &'a HudSpriteGpu,
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    x: f32,
    y: f32,
    occupancy: f32,
    label: Option<&str>,
) {
    let is_full = occupancy >= 1.0;
    let fill_width = ((occupancy.clamp(0.0, 1.0) * BUNDLE_PROGRESSBAR_FILL_MAX as f32).floor()
        as u32)
        .min(BUNDLE_PROGRESSBAR_FILL_MAX);
    if fill_width > 0 {
        let fill_rect = HudRect {
            x: x + 1.0,
            y,
            width: fill_width,
            height: BUNDLE_PROGRESSBAR_HEIGHT,
        };
        let fill_sprite = if is_full {
            tooltip_styles.get("container/bundle/bundle_progressbar_full")
        } else {
            tooltip_styles.get("container/bundle/bundle_progressbar_fill")
        };
        if let Some(sprite) = fill_sprite {
            push_hud_gui_sprite(vertices, commands, sprite, surface_size, fill_rect);
        } else {
            let tint = if is_full {
                [0.86, 0.24, 0.18, 1.0]
            } else {
                [0.29, 0.56, 0.98, 1.0]
            };
            push_hud_fill(vertices, commands, surface_size, fill_rect, tint);
        }
    }
    let border_rect = HudRect {
        x,
        y,
        width: BUNDLE_PROGRESSBAR_WIDTH,
        height: BUNDLE_PROGRESSBAR_HEIGHT,
    };
    if let Some(sprite) = tooltip_styles.get("container/bundle/bundle_progressbar_border") {
        push_hud_gui_sprite(vertices, commands, sprite, surface_size, border_rect);
    } else {
        push_hud_fill(
            vertices,
            commands,
            surface_size,
            border_rect,
            [0.7, 0.7, 0.7, 0.35],
        );
    }
    if let Some(label) = label {
        push_bundle_centered_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            fonts,
            obfuscated_seed,
            surface_size,
            label,
            x + 48.0,
            y + 3.0,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_bundle_centered_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    text: &str,
    center_x: f32,
    y: f32,
) {
    let runs = [HudComponentTextRun::plain(text)];
    let width = hud_component_runs_width(&runs, fonts).unwrap_or(0);
    let geometry = hud_component_text_geometry(
        &runs,
        fonts,
        obfuscated_seed,
        (center_x - (width / 2) as f32, y),
        true,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    push_hud_styled_text_geometry(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        surface_size,
        &geometry,
    );
}
