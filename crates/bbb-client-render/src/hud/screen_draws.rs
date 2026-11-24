use super::*;

pub(super) const fn hud_stats_header_background_sprite(hovered: bool) -> HudStatsSprite {
    if hovered {
        HudStatsSprite::Slot
    } else {
        HudStatsSprite::Header
    }
}

pub(super) const fn hud_stats_sort_sprite(sort_order: i8) -> HudStatsSprite {
    if sort_order < 0 {
        HudStatsSprite::SortDown
    } else {
        HudStatsSprite::SortUp
    }
}

pub fn hud_command_block_editor_layout(
    surface_size: PhysicalSize<u32>,
) -> HudCommandBlockEditorLayout {
    let center_x = i32::try_from(surface_size.width / 2).unwrap_or(i32::MAX);
    let action_y = i32::try_from(surface_size.height / 4)
        .unwrap_or(i32::MAX)
        .saturating_add(132);
    HudCommandBlockEditorLayout {
        command_input: HudCommandBlockEditorRect {
            x: center_x.saturating_sub(150),
            y: 50,
            width: 300,
            height: 20,
        },
        previous_output: HudCommandBlockEditorRect {
            x: center_x.saturating_sub(150),
            y: 135,
            width: 276,
            height: 20,
        },
        track_output: HudCommandBlockEditorRect {
            x: center_x.saturating_add(130),
            y: 135,
            width: 20,
            height: 20,
        },
        mode: HudCommandBlockEditorRect {
            x: center_x.saturating_sub(154),
            y: 165,
            width: 100,
            height: 20,
        },
        conditional: HudCommandBlockEditorRect {
            x: center_x.saturating_sub(50),
            y: 165,
            width: 100,
            height: 20,
        },
        automatic: HudCommandBlockEditorRect {
            x: center_x.saturating_add(54),
            y: 165,
            width: 100,
            height: 20,
        },
        done: HudCommandBlockEditorRect {
            x: center_x.saturating_sub(154),
            y: action_y,
            width: 150,
            height: 20,
        },
        cancel: HudCommandBlockEditorRect {
            x: center_x.saturating_add(4),
            y: action_y,
            width: 150,
            height: 20,
        },
    }
}

pub fn hud_minecart_command_block_editor_layout(
    surface_size: PhysicalSize<u32>,
) -> HudCommandBlockEditorLayout {
    let mut layout = hud_command_block_editor_layout(surface_size);
    layout.previous_output.y = 150;
    layout.track_output.y = 150;
    layout
}

pub fn hud_structure_block_editor_layout(
    surface_size: PhysicalSize<u32>,
) -> HudStructureBlockEditorLayout {
    let center = i32::try_from(surface_size.width / 2).unwrap_or(i32::MAX);
    let field = |x: i32, y: i32, width: u32| HudCommandBlockEditorRect {
        x: center.saturating_add(x),
        y,
        width,
        height: 20,
    };
    HudStructureBlockEditorLayout {
        fields: [
            field(-152, 40, 300),
            field(-152, 80, 80),
            field(-72, 80, 80),
            field(8, 80, 80),
            field(-152, 120, 80),
            field(-72, 120, 80),
            field(8, 120, 80),
            field(-152, 120, 80),
            field(-72, 120, 80),
            field(-152, 120, 240),
        ],
        mode: field(-154, 185, 50),
        mirror: field(-20, 185, 40),
        rotations: [
            field(-102, 185, 40),
            field(-61, 185, 40),
            field(21, 185, 40),
            field(62, 185, 40),
        ],
        option: field(104, 80, 50),
        detect: field(104, 120, 50),
        strict: field(104, 120, 50),
        include_entities: field(104, 160, 50),
        action: field(104, 185, 50),
        done: field(-154, 210, 150),
        cancel: field(4, 210, 150),
    }
}

pub fn hud_jigsaw_block_editor_layout(
    surface_size: PhysicalSize<u32>,
) -> HudJigsawBlockEditorLayout {
    let center = i32::try_from(surface_size.width / 2).unwrap_or(i32::MAX);
    let widget = |x: i32, y: i32, width: u32| HudCommandBlockEditorRect {
        x: center.saturating_add(x),
        y,
        width,
        height: 20,
    };
    HudJigsawBlockEditorLayout {
        fields: [
            widget(-153, 20, 300),
            widget(-153, 55, 300),
            widget(-153, 90, 300),
            widget(-153, 125, 300),
            widget(-153, 160, 98),
            widget(-50, 160, 98),
        ],
        joint: widget(54, 160, 100),
        levels: widget(-154, 185, 100),
        keep_jigsaws: widget(-50, 185, 100),
        generate: widget(54, 185, 100),
        done: widget(-154, 210, 150),
        cancel: widget(4, 210, 150),
    }
}

pub fn hud_test_block_editor_layout(surface_size: PhysicalSize<u32>) -> HudTestBlockEditorLayout {
    let center = i32::try_from(surface_size.width / 2).unwrap_or(i32::MAX);
    HudTestBlockEditorLayout {
        message: HudCommandBlockEditorRect {
            x: center.saturating_sub(152),
            y: 80,
            width: 240,
            height: 20,
        },
        mode: HudCommandBlockEditorRect {
            x: center.saturating_sub(154),
            y: 185,
            width: 50,
            height: 20,
        },
        done: HudCommandBlockEditorRect {
            x: center.saturating_sub(154),
            y: 210,
            width: 150,
            height: 20,
        },
        cancel: HudCommandBlockEditorRect {
            x: center.saturating_add(4),
            y: 210,
            width: 150,
            height: 20,
        },
    }
}

pub fn hud_test_instance_block_editor_layout(
    surface_size: PhysicalSize<u32>,
) -> HudTestInstanceBlockEditorLayout {
    let left = i32::try_from(surface_size.width / 2)
        .unwrap_or(i32::MAX)
        .saturating_sub(158);
    let widget = |index: i32, count: i32, y: i32| {
        let exact_width = (316.0 - f64::from(count - 1) * 8.0) / f64::from(count);
        HudCommandBlockEditorRect {
            x: left.saturating_add((f64::from(index) * (8.0 + exact_width)) as i32),
            y,
            width: exact_width as u32,
            height: 20,
        }
    };
    HudTestInstanceBlockEditorLayout {
        id: HudCommandBlockEditorRect {
            x: left,
            y: 40,
            width: 316,
            height: 20,
        },
        info: HudCommandBlockEditorRect {
            x: left,
            y: 70,
            width: 316,
            height: 72,
        },
        size: [widget(0, 5, 160), widget(1, 5, 160), widget(2, 5, 160)],
        rotation: widget(3, 5, 160),
        include_entities: widget(4, 5, 160),
        reset: widget(0, 2, 185),
        save: widget(1, 2, 185),
        run: widget(0, 3, 210),
        done: widget(1, 3, 210),
        cancel: widget(2, 3, 210),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_sign_editor_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    post_gui_item_commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    preedit_background: Option<&'a HudSpriteGpu>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudSignEditorScreen,
    sign_pip_target_ready: bool,
    hanging_sign_background: Option<&'a HudSpriteGpu>,
) {
    match screen.kind {
        HudSignEditorKind::Standing { .. }
            if screen.sign_preview.is_some() && sign_pip_target_ready =>
        {
            let start = vertices.len() as u32;
            vertices.extend_from_slice(&hud_quad_vertices(
                surface_size,
                hud_sign_editor_standing_preview_rect(surface_size),
                HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                HUD_TINT_WHITE,
            ));
            commands.push(HudDrawCommand::EntityPreviewBlit {
                target_index: 0,
                start,
                end: vertices.len() as u32,
            });
        }
        HudSignEditorKind::Hanging { .. } => {
            if let Some(background) = hanging_sign_background {
                push_hud_draw(
                    vertices,
                    commands,
                    background,
                    surface_size,
                    hud_sign_editor_hanging_background_rect(surface_size),
                );
            }
        }
        _ => {}
    }

    let Some(font_atlas) = font_atlas else {
        return;
    };
    push_hud_sign_editor_title(
        vertices,
        post_gui_item_commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        obfuscated_seed,
        surface_size,
        &screen.title,
    );
    push_hud_sign_editor_lines(
        vertices,
        post_gui_item_commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        obfuscated_seed,
        surface_size,
        screen,
    );
    let done = hud_sign_editor_done_button_rect(surface_size);
    push_hud_debug_options_button(
        vertices,
        post_gui_item_commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        obfuscated_seed,
        surface_size,
        done.x,
        done.y,
        i32::try_from(done.width).unwrap_or(i32::MAX),
        i32::try_from(done.height).unwrap_or(i32::MAX),
        &hud_coded_string_runs(&screen.done),
        true,
        screen.done_hovered,
        HUD_TINT_WHITE,
        button_sprites,
    );
    let preedit = hud_sign_editor_preedit_projection(screen, surface_size, glyphs);
    push_hud_inventory_preedit_overlay(
        vertices,
        post_gui_item_commands,
        white_pixel,
        preedit_background,
        Some(font_atlas),
        glyphs,
        obfuscated_pool,
        obfuscated_seed,
        surface_size,
        preedit.as_ref().map(|projection| &projection.overlay),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_pause_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    tooltip_background: Option<&'a HudSpriteGpu>,
    tooltip_frame: Option<&'a HudSpriteGpu>,
    now_playing_background: Option<&'a HudSpriteGpu>,
    gui_sprites: &'a GuiSprites,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudPauseScreen,
    now_playing: Option<&HudNowPlayingToast>,
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let runs = hud_coded_string_runs(screen.title.as_str());
    let draw = HudScreenTextDraw {
        runs: &runs,
        origin: hud_pause_screen_title_origin(screen, glyphs, surface_size),
        scale: 1.0,
        tint: HUD_TINT_WHITE,
        backdrop_opacity: 0.0,
    };
    push_hud_screen_text_draw(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &draw,
    );
    if screen.show_pause_menu && screen.feedback_subscreen {
        push_hud_pause_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            hud_pause_feedback_send_feedback_button_rect(surface_size),
            &hud_coded_string_runs(&screen.labels.send_feedback),
            screen.send_feedback_hovered,
            true,
            HudDebugOptionsButtonSprites::widget(gui_sprites),
        );
        push_hud_pause_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            hud_pause_feedback_report_bugs_button_rect(surface_size),
            &hud_coded_string_runs(&screen.labels.report_bugs),
            screen.report_bugs_hovered,
            screen.report_bugs_enabled,
            HudDebugOptionsButtonSprites::widget(gui_sprites),
        );
        push_hud_pause_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            hud_pause_feedback_back_button_rect(surface_size),
            &hud_coded_string_runs(&screen.labels.back),
            screen.feedback_back_hovered,
            true,
            HudDebugOptionsButtonSprites::widget(gui_sprites),
        );
    } else if screen.show_pause_menu {
        push_hud_pause_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            hud_pause_return_to_game_button_rect(surface_size),
            &hud_coded_string_runs(&screen.labels.return_to_game),
            screen.return_to_game_hovered,
            true,
            HudDebugOptionsButtonSprites::widget(gui_sprites),
        );
        push_hud_pause_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            hud_pause_advancements_button_rect(surface_size),
            &hud_coded_string_runs(&screen.labels.advancements),
            screen.advancements_hovered,
            true,
            HudDebugOptionsButtonSprites::widget(gui_sprites),
        );
        push_hud_pause_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            hud_pause_stats_button_rect(surface_size),
            &hud_coded_string_runs(&screen.labels.stats),
            screen.stats_hovered,
            true,
            HudDebugOptionsButtonSprites::widget(gui_sprites),
        );
        push_hud_pause_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            hud_pause_send_feedback_button_rect(surface_size),
            &hud_coded_string_runs(if screen.custom_dialog_label.is_some() {
                &screen.labels.feedback
            } else {
                &screen.labels.send_feedback
            }),
            screen.send_feedback_hovered,
            true,
            HudDebugOptionsButtonSprites::widget(gui_sprites),
        );
        push_hud_pause_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            hud_pause_report_bugs_button_rect(surface_size),
            &screen.custom_dialog_label.as_deref().map_or_else(
                || hud_coded_string_runs(&screen.labels.report_bugs),
                hud_coded_string_runs,
            ),
            if screen.custom_dialog_label.is_some() {
                screen.custom_dialog_hovered
            } else {
                screen.report_bugs_hovered
            },
            screen.custom_dialog_label.is_some() || screen.report_bugs_enabled,
            HudDebugOptionsButtonSprites::widget(gui_sprites),
        );
        push_hud_pause_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            hud_pause_options_button_rect(surface_size),
            &hud_coded_string_runs(&screen.labels.options),
            screen.options_hovered,
            screen.options_enabled,
            HudDebugOptionsButtonSprites::widget(gui_sprites),
        );
        push_hud_pause_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            hud_pause_disconnect_button_rect(surface_size),
            &hud_coded_string_runs(&screen.labels.disconnect),
            screen.disconnect_hovered,
            screen.disconnect_enabled,
            HudDebugOptionsButtonSprites::widget(gui_sprites),
        );
    }
    for tooltip in [
        screen.custom_dialog_tooltip.as_ref(),
        screen.options_tooltip.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        push_hud_rich_tooltip(
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
            &hud_coded_string_runs(&tooltip.text),
            tooltip.x,
            tooltip.y,
        );
    }
    if let Some(toast) = now_playing {
        push_hud_now_playing_toast(
            vertices,
            commands,
            white_pixel,
            now_playing_background,
            gui_sprites,
            Some(font_atlas),
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            toast,
        );
    }
}

pub(super) fn hud_pause_screen_draws_background(screen: &HudPauseScreen) -> bool {
    screen.show_pause_menu
}

pub(super) fn push_hud_pause_background<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    surface_size: PhysicalSize<u32>,
) {
    let start = vertices.len() as u32;
    vertices.extend_from_slice(&hud_pause_background_vertices(surface_size));
    let end = vertices.len() as u32;
    commands.push(HudDrawCommand::Fill { start, end });
}

pub(super) fn hud_pause_background_vertices(surface_size: PhysicalSize<u32>) -> [HudVertex; 6] {
    let mut vertices = hud_quad_vertices(
        surface_size,
        absolute_hud_rect(
            0.0,
            0.0,
            surface_size.width.max(1),
            surface_size.height.max(1),
        ),
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        HUD_PAUSE_BACKGROUND_TOP_TINT,
    );
    vertices[2].tint = HUD_PAUSE_BACKGROUND_BOTTOM_TINT;
    vertices[4].tint = HUD_PAUSE_BACKGROUND_BOTTOM_TINT;
    vertices[5].tint = HUD_PAUSE_BACKGROUND_BOTTOM_TINT;
    vertices
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_pause_button<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    rect: (i32, i32, i32, i32),
    label: &[HudStyledTextRun],
    hovered: bool,
    enabled: bool,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
) {
    let (x, y, width, height) = rect;
    push_hud_debug_options_button_with_text_policy(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        x,
        y,
        width,
        height,
        label,
        enabled,
        hovered,
        HUD_TINT_WHITE,
        true,
        button_sprites,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_command_block_editor_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    text_field: Option<&'a HudSpriteGpu>,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudCommandBlockEditorScreen,
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let layout = match screen.kind {
        HudCommandBlockEditorKind::Block => hud_command_block_editor_layout(surface_size),
        HudCommandBlockEditorKind::Minecart => {
            hud_minecart_command_block_editor_layout(surface_size)
        }
    };
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
        20,
        HUD_TINT_WHITE,
    );
    // `AbstractCommandBlockEditScreen`: both labels are
    // `Component.translatable` drawn through the Component face.
    push_hud_coded_string_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &hud_coded_string_runs(&screen.command_label),
        (layout.command_input.x.saturating_add(1) as f32, 40.0),
        label_tint,
        1.0,
        true,
    );
    if !screen.previous_output.is_empty() {
        push_hud_coded_string_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &hud_coded_string_runs(&screen.previous_output_label),
            (
                layout.previous_output.x.saturating_add(1) as f32,
                layout.previous_output.y.saturating_sub(10) as f32,
            ),
            label_tint,
            1.0,
            true,
        );
        if let Some(text_field) = text_field {
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                text_field,
                surface_size,
                absolute_hud_rect(
                    layout.previous_output.x as f32,
                    layout.previous_output.y as f32,
                    layout.previous_output.width,
                    layout.previous_output.height,
                ),
                HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                HUD_TINT_WHITE,
            );
        } else {
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                white_pixel,
                surface_size,
                absolute_hud_rect(
                    layout.previous_output.x as f32,
                    layout.previous_output.y as f32,
                    layout.previous_output.width,
                    layout.previous_output.height,
                ),
                HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                [0.0, 0.0, 0.0, 0.5],
            );
        }
        let previous_label = HudInventoryTextLabel {
            component_runs: Vec::new(),
            x: 0,
            y: 0,
            width: layout.previous_output.width.saturating_sub(8),
            text: screen.previous_output.clone(),
            tint: HUD_COMMAND_INPUT_TINT,
            background: None,
            input: Some(HudInventoryTextInputDecoration {
                cursor: screen.previous_output.encode_utf16().count(),
                selection: screen.previous_output.encode_utf16().count(),
                scroll_to: screen.previous_output.encode_utf16().count(),
                max_length: 32_500,
                cursor_visible: false,
                cursor_tint: HUD_COMMAND_INPUT_TINT,
            }),
            scrolling: None,
            shadow: true,
            font: None,
            runs: Vec::new(),
        };
        push_hud_inventory_text_input_label(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &previous_label,
            previous_label
                .input
                .expect("previous output input decoration"),
            (
                layout.previous_output.x.saturating_add(4) as f32,
                layout.previous_output.y.saturating_add(6) as f32,
            ),
        );
    }

    let mut buttons = vec![(
        layout.track_output,
        screen.track_output.as_str(),
        screen.track_output_hovered,
        screen.controls_enabled,
    )];
    if screen.kind == HudCommandBlockEditorKind::Block {
        buttons.extend([
            (
                layout.mode,
                screen.mode.as_str(),
                screen.mode_hovered,
                screen.controls_enabled,
            ),
            (
                layout.conditional,
                screen.conditional.as_str(),
                screen.conditional_hovered,
                screen.controls_enabled,
            ),
            (
                layout.automatic,
                screen.automatic.as_str(),
                screen.automatic_hovered,
                screen.controls_enabled,
            ),
        ]);
    }
    buttons.extend([
        (
            layout.done,
            screen.done.as_str(),
            screen.done_hovered,
            screen.controls_enabled,
        ),
        (
            layout.cancel,
            screen.cancel.as_str(),
            screen.cancel_hovered,
            true,
        ),
    ]);
    for (rect, label, hovered, enabled) in buttons {
        let label = hud_coded_string_runs(label);
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
            &label,
            enabled,
            hovered,
            if enabled {
                HUD_TINT_WHITE
            } else {
                hud_argb_to_tint(0xFFA0A0A0)
            },
            button_sprites,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_structure_block_editor_screen<'a>(
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
    screen: &HudStructureBlockEditorScreen,
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let layout = hud_structure_block_editor_layout(surface_size);
    let center = i32::try_from(surface_size.width / 2).unwrap_or(i32::MAX);
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

    let mut labels = Vec::new();
    if screen.mode != HudStructureBlockMode::Data {
        labels.push((&screen.name_label, center.saturating_sub(153), 30, false));
    }
    if matches!(
        screen.mode,
        HudStructureBlockMode::Save | HudStructureBlockMode::Load
    ) {
        labels.push((
            &screen.position_label,
            center.saturating_sub(153),
            70,
            false,
        ));
        labels.push((
            &screen.include_entities_label,
            center.saturating_add(154),
            150,
            true,
        ));
    }
    match screen.mode {
        HudStructureBlockMode::Save => {
            labels.push((&screen.size_label, center.saturating_sub(153), 110, false));
            labels.push((&screen.detect_label, center.saturating_add(154), 110, true));
            labels.push((&screen.option_label, center.saturating_add(154), 70, true));
        }
        HudStructureBlockMode::Load => {
            labels.push((
                &screen.integrity_label,
                center.saturating_sub(153),
                110,
                false,
            ));
            labels.push((&screen.strict_label, center.saturating_add(154), 110, true));
            labels.push((&screen.option_label, center.saturating_add(154), 70, true));
        }
        HudStructureBlockMode::Data => {
            labels.push((&screen.data_label, center.saturating_sub(153), 110, false));
        }
        HudStructureBlockMode::Corner => {}
    }
    labels.push((
        &screen.mode_description,
        center.saturating_sub(153),
        174,
        false,
    ));
    for (label, anchor, y, right_aligned) in labels {
        // `StructureBlockEditScreen` declares every label a
        // `Component.translatable`, so `graphics.text(font, Component, …)` and
        // `font.width(Component)` read the same `FormattedText.of` run list.
        let runs = hud_coded_string_runs(label);
        let x = if right_aligned {
            anchor.saturating_sub(hud_coded_string_width(&runs, glyphs))
        } else {
            anchor
        };
        push_hud_coded_string_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &runs,
            (x as f32, y as f32),
            label_tint,
            1.0,
            true,
        );
    }

    let visible_fields: &[usize] = match screen.mode {
        HudStructureBlockMode::Save => &[0, 1, 2, 3, 4, 5, 6],
        HudStructureBlockMode::Load => &[0, 1, 2, 3, 7, 8],
        HudStructureBlockMode::Corner => &[0],
        HudStructureBlockMode::Data => &[9],
    };
    for index in visible_fields {
        let rect = layout.fields[*index];
        let field = &screen.fields[*index];
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

    let mut buttons = vec![
        (
            layout.mode,
            screen.mode_value.as_str(),
            screen.mode_hovered,
            true,
        ),
        (layout.done, screen.done.as_str(), screen.done_hovered, true),
        (
            layout.cancel,
            screen.cancel.as_str(),
            screen.cancel_hovered,
            true,
        ),
    ];
    match screen.mode {
        HudStructureBlockMode::Save => {
            buttons.extend([
                (
                    layout.option,
                    screen.option.as_str(),
                    screen.option_hovered,
                    true,
                ),
                (
                    layout.detect,
                    screen.detect.as_str(),
                    screen.detect_hovered,
                    true,
                ),
                (
                    layout.include_entities,
                    screen.include_entities.as_str(),
                    screen.include_entities_hovered,
                    true,
                ),
                (
                    layout.action,
                    screen.action.as_str(),
                    screen.action_hovered,
                    true,
                ),
            ]);
        }
        HudStructureBlockMode::Load => {
            buttons.extend([
                (
                    layout.option,
                    screen.option.as_str(),
                    screen.option_hovered,
                    true,
                ),
                (
                    layout.strict,
                    screen.strict.as_str(),
                    screen.strict_hovered,
                    true,
                ),
                (
                    layout.include_entities,
                    screen.include_entities.as_str(),
                    screen.include_entities_hovered,
                    true,
                ),
                (
                    layout.mirror,
                    screen.mirror_value.as_str(),
                    screen.mirror_hovered,
                    true,
                ),
                (
                    layout.action,
                    screen.action.as_str(),
                    screen.action_hovered,
                    true,
                ),
            ]);
            for (index, label) in ["0", "90", "180", "270"].into_iter().enumerate() {
                buttons.push((
                    layout.rotations[index],
                    label,
                    screen.rotation_hovered[index],
                    screen.rotation != label,
                ));
            }
        }
        HudStructureBlockMode::Corner | HudStructureBlockMode::Data => {}
    }
    for (rect, label, hovered, enabled) in buttons {
        let label = hud_coded_string_runs(label);
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
            &label,
            enabled,
            hovered,
            if enabled { HUD_TINT_WHITE } else { label_tint },
            button_sprites,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_jigsaw_block_editor_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    text_field: Option<&'a HudSpriteGpu>,
    text_field_highlighted: Option<&'a HudSpriteGpu>,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    slider_sprites: HudJigsawSliderSprites<'a>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    tooltip_background: Option<&'a HudSpriteGpu>,
    tooltip_frame: Option<&'a HudSpriteGpu>,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudJigsawBlockEditorScreen,
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let layout = hud_jigsaw_block_editor_layout(surface_size);
    let label_tint = hud_argb_to_tint(0xFFA0A0A0);
    // `JigsawBlockEditScreen`: every label is a `Component.translatable`.
    for (label, rect) in screen.labels.iter().zip(layout.fields) {
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
            (rect.x as f32, rect.y.saturating_sub(10) as f32),
            label_tint,
            1.0,
            true,
        );
    }
    if screen.joint_visible {
        push_hud_coded_string_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &hud_coded_string_runs(&screen.joint_label),
            (
                layout.joint.x.saturating_sub(1) as f32,
                layout.joint.y.saturating_sub(10) as f32,
            ),
            label_tint,
            1.0,
            true,
        );
    }

    for (field, rect) in screen.fields.iter().zip(layout.fields) {
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

    let slider_highlighted = screen.levels_hovered;
    let slider = if slider_highlighted {
        slider_sprites.highlighted.or(slider_sprites.normal)
    } else {
        slider_sprites.normal
    };
    if let Some(slider) = slider {
        push_hud_jigsaw_slider_track(vertices, commands, slider, surface_size, layout.levels);
    } else {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            white_pixel,
            surface_size,
            absolute_hud_rect(
                layout.levels.x as f32,
                layout.levels.y as f32,
                layout.levels.width,
                layout.levels.height,
            ),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            [0.2, 0.2, 0.2, 0.96],
        );
    }
    let slider_value = f64::from_bits(screen.slider_value_bits).clamp(0.0, 1.0);
    let handle_x = layout.levels.x.saturating_add(
        (slider_value * f64::from(layout.levels.width.saturating_sub(8))).floor() as i32,
    );
    let handle = if slider_highlighted {
        slider_sprites.handle_highlighted.or(slider_sprites.handle)
    } else {
        slider_sprites.handle
    };
    if let Some(handle) = handle {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            handle,
            surface_size,
            absolute_hud_rect(handle_x as f32, layout.levels.y as f32, 8, 20),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            HUD_TINT_WHITE,
        );
    }
    push_hud_debug_options_centered_text_in_width(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &hud_coded_string_runs(&screen.levels),
        layout.levels.x,
        i32::try_from(layout.levels.width).unwrap_or(i32::MAX),
        layout.levels.y.saturating_add(6),
        HUD_TINT_WHITE,
    );

    let mut buttons = vec![
        (
            layout.keep_jigsaws,
            screen.keep_jigsaws.as_str(),
            screen.keep_jigsaws_hovered,
            true,
        ),
        (
            layout.generate,
            screen.generate.as_str(),
            screen.generate_hovered,
            screen.controls_enabled,
        ),
        (
            layout.done,
            screen.done.as_str(),
            screen.done_hovered,
            screen.controls_enabled,
        ),
        (
            layout.cancel,
            screen.cancel.as_str(),
            screen.cancel_hovered,
            true,
        ),
    ];
    if screen.joint_visible {
        buttons.push((
            layout.joint,
            screen.joint.as_str(),
            screen.joint_hovered,
            true,
        ));
    }
    for (rect, label, hovered, enabled) in buttons {
        let label = hud_coded_string_runs(label);
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
            &label,
            enabled,
            hovered,
            if enabled {
                HUD_TINT_WHITE
            } else {
                hud_argb_to_tint(0xFFA0A0A0)
            },
            button_sprites,
        );
    }
    if let Some(tooltip) = &screen.tooltip {
        push_hud_rich_tooltip(
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
            &hud_coded_string_runs(&tooltip.text),
            tooltip.x,
            tooltip.y,
        );
    }
}

fn push_hud_jigsaw_slider_track<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    slider: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudCommandBlockEditorRect,
) {
    if rect.width < 2 || slider.width < 2 {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            slider,
            surface_size,
            absolute_hud_rect(rect.x as f32, rect.y as f32, rect.width, rect.height),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            HUD_TINT_WHITE,
        );
        return;
    }
    let source_border = 1.0 / slider.width as f32;
    for (x, width, min_u, max_u) in [
        (rect.x, 1, 0.0, source_border),
        (
            rect.x.saturating_add(1),
            rect.width.saturating_sub(2),
            source_border,
            1.0 - source_border,
        ),
        (
            rect.x.saturating_add_unsigned(rect.width.saturating_sub(1)),
            1,
            1.0 - source_border,
            1.0,
        ),
    ] {
        if width == 0 {
            continue;
        }
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            slider,
            surface_size,
            absolute_hud_rect(x as f32, rect.y as f32, width, rect.height),
            HudUvRect {
                min: [min_u, 0.0],
                max: [max_u, 1.0],
            },
            HUD_TINT_WHITE,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_test_block_editor_screen<'a>(
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
    screen: &HudTestBlockEditorScreen,
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let layout = hud_test_block_editor_layout(surface_size);
    let center = i32::try_from(surface_size.width / 2).unwrap_or(i32::MAX);
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

    if screen.message_visible {
        // `TestBlockEditScreen`: the label and the mode message are
        // Components drawn through the `String`/`Component` face.
        push_hud_coded_string_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &hud_coded_string_runs(&screen.message_label),
            (center.saturating_sub(153) as f32, 70.0),
            label_tint,
            1.0,
            true,
        );
        let field = &screen.message;
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
            layout.message,
            field,
        );
    }

    push_hud_coded_string_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &hud_coded_string_runs(&screen.mode_info),
        (center.saturating_sub(153) as f32, 174.0),
        label_tint,
        1.0,
        true,
    );
    for (rect, label, hovered) in [
        (layout.mode, screen.mode.as_str(), screen.mode_hovered),
        (layout.done, screen.done.as_str(), screen.done_hovered),
        (layout.cancel, screen.cancel.as_str(), screen.cancel_hovered),
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
            true,
            hovered,
            HUD_TINT_WHITE,
            button_sprites,
        );
    }
}

pub(super) fn hud_pause_screen_title_origin(
    screen: &HudPauseScreen,
    glyphs: &HudFontGlyphMap,
    surface_size: PhysicalSize<u32>,
) -> (f32, f32) {
    let runs = hud_coded_string_runs(screen.title.as_str());
    let width = hud_font_runs_width(&runs, glyphs).unwrap_or_default() as f32;
    let y = if screen.feedback_subscreen {
        12.0
    } else if screen.show_pause_menu {
        40.0
    } else {
        10.0
    };
    (surface_size.width.max(1) as f32 * 0.5 - width * 0.5, y)
}

pub fn hud_pause_return_to_game_button_rect(
    surface_size: PhysicalSize<u32>,
) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 - HUD_PAUSE_RETURN_TO_GAME_BUTTON_WIDTH / 2,
        height / 4 + HUD_PAUSE_RETURN_TO_GAME_TOP_OFFSET,
        HUD_PAUSE_RETURN_TO_GAME_BUTTON_WIDTH,
        HUD_PAUSE_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

pub fn hud_pause_advancements_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 - HUD_PAUSE_RETURN_TO_GAME_BUTTON_WIDTH / 2,
        height / 4 + HUD_PAUSE_SECOND_ROW_TOP_OFFSET,
        HUD_PAUSE_HALF_BUTTON_WIDTH,
        HUD_PAUSE_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

pub fn hud_pause_stats_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 + 4,
        height / 4 + HUD_PAUSE_SECOND_ROW_TOP_OFFSET,
        HUD_PAUSE_HALF_BUTTON_WIDTH,
        HUD_PAUSE_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

pub fn hud_pause_send_feedback_button_rect(
    surface_size: PhysicalSize<u32>,
) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 - HUD_PAUSE_RETURN_TO_GAME_BUTTON_WIDTH / 2,
        height / 4 + HUD_PAUSE_THIRD_ROW_TOP_OFFSET,
        HUD_PAUSE_HALF_BUTTON_WIDTH,
        HUD_PAUSE_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

pub fn hud_pause_report_bugs_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 + 4,
        height / 4 + HUD_PAUSE_THIRD_ROW_TOP_OFFSET,
        HUD_PAUSE_HALF_BUTTON_WIDTH,
        HUD_PAUSE_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

pub fn hud_pause_options_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 - HUD_PAUSE_RETURN_TO_GAME_BUTTON_WIDTH / 2,
        height / 4 + HUD_PAUSE_FOURTH_ROW_TOP_OFFSET,
        HUD_PAUSE_HALF_BUTTON_WIDTH,
        HUD_PAUSE_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

pub fn hud_pause_disconnect_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 - HUD_PAUSE_RETURN_TO_GAME_BUTTON_WIDTH / 2,
        height / 4 + HUD_PAUSE_DISCONNECT_ROW_TOP_OFFSET,
        HUD_PAUSE_RETURN_TO_GAME_BUTTON_WIDTH,
        HUD_PAUSE_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

pub fn hud_pause_feedback_send_feedback_button_rect(
    surface_size: PhysicalSize<u32>,
) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    (
        width / 2 - HUD_PAUSE_RETURN_TO_GAME_BUTTON_WIDTH / 2,
        hud_pause_feedback_button_y(surface_size),
        HUD_PAUSE_HALF_BUTTON_WIDTH,
        HUD_PAUSE_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

pub fn hud_pause_feedback_report_bugs_button_rect(
    surface_size: PhysicalSize<u32>,
) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    (
        width / 2 + 4,
        hud_pause_feedback_button_y(surface_size),
        HUD_PAUSE_HALF_BUTTON_WIDTH,
        HUD_PAUSE_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

pub fn hud_pause_feedback_back_button_rect(
    surface_size: PhysicalSize<u32>,
) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 - HUD_PAUSE_FEEDBACK_BACK_BUTTON_WIDTH / 2,
        height - HUD_PAUSE_FEEDBACK_FOOTER_HEIGHT
            + (HUD_PAUSE_FEEDBACK_FOOTER_HEIGHT - HUD_PAUSE_RETURN_TO_GAME_BUTTON_HEIGHT) / 2,
        HUD_PAUSE_FEEDBACK_BACK_BUTTON_WIDTH,
        HUD_PAUSE_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

fn hud_pause_feedback_button_y(surface_size: PhysicalSize<u32>) -> i32 {
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let grid_height = HUD_PAUSE_RETURN_TO_GAME_BUTTON_HEIGHT + HUD_PAUSE_FEEDBACK_CELL_PADDING;
    (HUD_PAUSE_FEEDBACK_HEADER_HEIGHT + HUD_PAUSE_FEEDBACK_CONTENT_MARGIN_TOP)
        .min(height - HUD_PAUSE_FEEDBACK_FOOTER_HEIGHT - grid_height)
        + HUD_PAUSE_FEEDBACK_CELL_PADDING
}

pub fn hud_stats_done_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 - HUD_STATS_DONE_BUTTON_WIDTH / 2,
        height - HUD_STATS_FOOTER_HEIGHT
            + (HUD_STATS_FOOTER_HEIGHT - HUD_STATS_DONE_BUTTON_HEIGHT) / 2,
        HUD_STATS_DONE_BUTTON_WIDTH,
        HUD_STATS_DONE_BUTTON_HEIGHT,
    )
}

pub(super) fn hud_stats_item_icon_rect(
    surface_size: PhysicalSize<u32>,
    screen: &HudStatsScreen,
    row: usize,
) -> HudRect {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let list_x = width / 2 - HUD_STATS_LIST_WIDTH / 2;
    let absolute_row = screen.scroll_row.saturating_add(row);
    let y = HUD_STATS_LIST_TOP
        + HUD_STATS_LIST_ENTRY_PADDING
        + 22
        + i32::try_from(absolute_row).unwrap_or(i32::MAX / 22) * 22
        - screen.scroll_amount;
    absolute_hud_rect((list_x + 1) as f32, (y + 2) as f32, 16, 16)
}

pub(super) fn hud_stats_list_scissor_rect(surface_size: PhysicalSize<u32>) -> HudRect {
    let top = u32::try_from(HUD_STATS_LIST_TOP).unwrap_or_default();
    let footer = u32::try_from(HUD_STATS_FOOTER_HEIGHT).unwrap_or_default();
    absolute_hud_rect(
        0.0,
        top as f32,
        surface_size.width.max(1),
        surface_size
            .height
            .saturating_sub(top.saturating_add(footer)),
    )
}

pub(super) fn push_hud_stats_list_scissor(
    commands: &mut Vec<HudDrawCommand<'_>>,
    surface_size: PhysicalSize<u32>,
) {
    let y = u32::try_from(HUD_STATS_LIST_TOP).unwrap_or_default();
    let footer = u32::try_from(HUD_STATS_FOOTER_HEIGHT).unwrap_or_default();
    let height = surface_size.height.saturating_sub(y.saturating_add(footer));
    if height > 0 {
        commands.push(HudDrawCommand::Scissor {
            x: 0,
            y,
            width: surface_size.width.max(1),
            height,
        });
    }
}

pub(super) fn push_hud_stats_list_scissor_reset(
    commands: &mut Vec<HudDrawCommand<'_>>,
    surface_size: PhysicalSize<u32>,
) {
    commands.push(HudDrawCommand::Scissor {
        x: 0,
        y: 0,
        width: surface_size.width.max(1),
        height: surface_size.height.max(1),
    });
}

pub(super) fn push_hud_stats_scrollbar<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    sprites: &'a GuiSprites,
    surface_size: PhysicalSize<u32>,
    screen: &HudStatsScreen,
) {
    let Some((background, thumb)) = hud_stats_scrollbar_rects(surface_size, screen) else {
        return;
    };
    push_hud_scrollbar(
        vertices,
        commands,
        white_pixel,
        sprites,
        surface_size,
        background,
        thumb,
    );
}

pub(super) fn push_hud_scrollbar<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    sprites: &'a GuiSprites,
    surface_size: PhysicalSize<u32>,
    background: HudRect,
    thumb: HudRect,
) {
    for (kind, rect, fallback_tint) in [
        (
            HudStatsScrollbarSprite::Background,
            background,
            [0.0, 0.0, 0.0, 0.45],
        ),
        (
            HudStatsScrollbarSprite::Scroller,
            thumb,
            [0.55, 0.55, 0.55, 0.92],
        ),
    ] {
        if let Some(sprite) = sprites.get(kind.sprite_path()) {
            push_hud_gui_sprite(vertices, commands, sprite, surface_size, rect);
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
                fallback_tint,
            );
        }
    }
}

pub(super) fn hud_stats_scrollbar_rects(
    surface_size: PhysicalSize<u32>,
    screen: &HudStatsScreen,
) -> Option<(HudRect, HudRect)> {
    if screen.max_scroll_amount <= 0 || screen.scroller_height <= 0 {
        return None;
    }
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let viewport_height = height
        .saturating_sub(HUD_STATS_LIST_TOP + HUD_STATS_FOOTER_HEIGHT)
        .max(1);
    let x = width / 2 + HUD_STATS_LIST_WIDTH / 2 + 8;
    let scroller_height = screen
        .scroller_height
        .clamp(1, (viewport_height - 8).max(1));
    let thumb_travel = (viewport_height - scroller_height).max(0);
    let thumb_y = HUD_STATS_LIST_TOP
        + screen.scroll_amount.saturating_mul(thumb_travel) / screen.max_scroll_amount;
    Some((
        absolute_hud_rect(
            x as f32,
            HUD_STATS_LIST_TOP as f32,
            6,
            u32::try_from(viewport_height).unwrap_or(1),
        ),
        absolute_hud_rect(
            x as f32,
            thumb_y as f32,
            6,
            u32::try_from(scroller_height).unwrap_or(1),
        ),
    ))
}

fn hud_sign_editor_standing_preview_rect(surface_size: PhysicalSize<u32>) -> HudRect {
    let center_x = surface_size.width.max(1) as f32 * 0.5;
    absolute_hud_rect(center_x - 48.0, 66.0, 96, 102)
}

fn hud_sign_editor_hanging_background_rect(surface_size: PhysicalSize<u32>) -> HudRect {
    let center_x = surface_size.width.max(1) as f32 * 0.5;
    absolute_hud_rect(center_x - 36.0, 30.5, 72, 72)
}

pub(super) fn hud_sign_editor_preedit_projection(
    screen: &HudSignEditorScreen,
    surface_size: PhysicalSize<u32>,
    glyphs: &HudFontGlyphMap,
) -> Option<HudImePreeditProjection> {
    let preedit = screen
        .preedit
        .as_ref()
        .filter(|preedit| !preedit.text.is_empty())?;
    let (offset_y, scale, line_height, input_height) = match screen.kind {
        HudSignEditorKind::Standing { .. } => (90.0, 0.976_562_8, 10.0, 10),
        HudSignEditorKind::Hanging { .. } => (125.0, 1.0, 9.0, 9),
    };
    let active_line = &screen.lines[screen.line];
    // `AbstractSignEditScreen` preloads `Component::getString`, so these
    // lines carry legacy codes and `extractSignText` measures and draws them
    // through the `FormattedText.of` face.
    let line_runs = hud_coded_string_runs(active_line);
    let line_width = hud_coded_string_width(&line_runs, glyphs);
    let cursor_prefix = hud_prefix_by_chars(active_line, screen.cursor);
    let cursor_width = hud_coded_string_width(&hud_coded_string_runs(&cursor_prefix), glyphs);
    let sign_midpoint = 4.0 * line_height * 0.5;
    let cursor_y = screen.line as f32 * line_height - sign_midpoint;
    // Vanilla derives this from integer font widths before applying the sign
    // transform, so odd line widths truncate at the same point as Java `/ 2`.
    let cursor_x = cursor_width.saturating_sub(line_width / 2);
    let input_left = (surface_size.width as f32 * 0.5 + cursor_x as f32 * scale) as i32;
    let input_top = (offset_y + cursor_y * scale) as i32;
    Some(ime_preedit::project(
        preedit,
        (input_left, input_top),
        input_height,
        surface_size,
        glyphs,
    ))
}

#[allow(clippy::too_many_arguments)]
fn push_hud_sign_editor_title<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    title: &str,
) {
    // `AbstractSignEditScreen` centers the title Component through
    // `graphics.centeredText`, so the title string is the coded face.
    let title_runs = hud_coded_string_runs(title);
    let width = hud_coded_string_width(&title_runs, glyphs);
    let origin = (
        surface_size.width.max(1) as f32 * 0.5 - width as f32 * 0.5,
        40.0,
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
        &title_runs,
        origin,
        HUD_TINT_WHITE,
        1.0,
        true,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_hud_sign_editor_lines<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudSignEditorScreen,
) {
    let (offset_y, scale, line_height) = match screen.kind {
        HudSignEditorKind::Standing { .. } => (90.0, 0.976_562_8, 10.0),
        HudSignEditorKind::Hanging { .. } => (125.0, 1.0, 9.0),
    };
    let center_x = surface_size.width.max(1) as f32 * 0.5;
    let sign_midpoint = 4.0 * line_height * 0.5;

    for (line_index, line) in screen.lines.iter().enumerate() {
        // One `FormattedText.of(line)` list for the centering width and the
        // glyphs, so a code cannot shift the pen without moving the line.
        let runs = hud_coded_string_runs(line);
        let line_width = hud_coded_string_width(&runs, glyphs);
        let line_y = line_index as f32 * line_height - sign_midpoint;
        let origin = (
            center_x - (line_width / 2) as f32 * scale,
            offset_y + line_y * scale,
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
            &runs,
            origin,
            screen.text_tint,
            scale,
            false,
        );
        if line_index == screen.line
            && screen.cursor_visible
            && screen.cursor >= line.chars().count()
        {
            // `TextCursorUtils.extractAppendCursor` draws the literal `"_"`
            // through the same `String` overload.
            push_hud_coded_string_text(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                obfuscated_seed,
                surface_size,
                &hud_coded_string_runs("_"),
                (origin.0 + line_width as f32 * scale, origin.1),
                screen.text_tint,
                scale,
                false,
            );
        }
    }

    let active_line = &screen.lines[screen.line];
    let line_width = hud_coded_string_width(&hud_coded_string_runs(active_line), glyphs);
    let selection_start = screen.cursor.min(screen.selection);
    let selection_end = screen.cursor.max(screen.selection);
    let cursor_prefix = hud_prefix_by_chars(active_line, screen.cursor);
    let cursor_position =
        hud_coded_string_width(&hud_coded_string_runs(&cursor_prefix), glyphs) as f32;
    let cursor_x = cursor_position - (line_width / 2) as f32;
    let cursor_y = screen.line as f32 * line_height - sign_midpoint;

    // AbstractSignEditScreen extracts the cursor before its highlight.
    if screen.cursor_visible && screen.cursor < active_line.chars().count() {
        let x = center_x + cursor_x * scale;
        let y0 = offset_y + (cursor_y - 1.0) * scale;
        let y1 = offset_y + (cursor_y + line_height) * scale;
        let mut tint = screen.text_tint;
        tint[3] = 1.0;
        push_hud_fill_corners(
            vertices,
            commands,
            surface_size,
            [[x, y0], [x, y1], [x + scale, y1], [x + scale, y0]],
            tint,
        );
    }
    if selection_start != selection_end {
        let start_prefix = hud_prefix_by_chars(active_line, selection_start);
        let end_prefix = hud_prefix_by_chars(active_line, selection_end);
        let start_position =
            hud_coded_string_width(&hud_coded_string_runs(&start_prefix), glyphs) as f32;
        let end_position =
            hud_coded_string_width(&hud_coded_string_runs(&end_prefix), glyphs) as f32;
        let x = center_x + (start_position - (line_width / 2) as f32) * scale;
        let y = offset_y + cursor_y * scale;
        let right = center_x + (end_position - (line_width / 2) as f32) * scale;
        let bottom = offset_y + (cursor_y + line_height) * scale;
        push_hud_text_highlight_corners(
            vertices,
            commands,
            surface_size,
            [[x, y], [x, bottom], [right, bottom], [right, y]],
        );
    }
}

fn hud_chat_restrictions_done_rect(surface_size: PhysicalSize<u32>) -> HudRect {
    absolute_hud_rect(
        (surface_size.width as f32 * 0.5 - 100.0).floor(),
        i32::try_from(surface_size.height)
            .unwrap_or(i32::MAX)
            .saturating_sub(29) as f32,
        200,
        20,
    )
}

pub(super) fn hud_chat_restrictions_done_contains(
    surface_size: PhysicalSize<u32>,
    position: (f64, f64),
) -> bool {
    if !position.0.is_finite() || !position.1.is_finite() {
        return false;
    }
    let rect = hud_chat_restrictions_done_rect(surface_size);
    position.0 >= f64::from(rect.x)
        && position.1 >= f64::from(rect.y)
        && position.0 < f64::from(rect.x + rect.width as f32)
        && position.1 < f64::from(rect.y + rect.height as f32)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_chat_restrictions_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudChatRestrictionsScreen,
) {
    push_hud_pause_background(vertices, commands, surface_size);
    let Some(font_atlas) = font_atlas else { return };
    let centered = |runs: &[HudStyledTextRun]| {
        surface_size.width as f32 * 0.5
            - hud_font_runs_width(runs, glyphs).unwrap_or_default() as f32 * 0.5
    };
    let title = hud_coded_string_runs(&screen.title);
    push_hud_screen_text_draw(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &HudScreenTextDraw {
            runs: &title,
            origin: (centered(&title), 15.0),
            scale: 1.0,
            tint: HUD_TINT_WHITE,
            backdrop_opacity: 0.0,
        },
    );
    let reason = [screen.reason.clone()];
    push_hud_screen_text_draw(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &HudScreenTextDraw {
            runs: &reason,
            origin: (centered(&reason), 54.0),
            scale: 1.0,
            tint: HUD_TINT_WHITE,
            backdrop_opacity: 0.0,
        },
    );
    for (index, permission) in screen.permissions.iter().enumerate() {
        let line = [permission.clone()];
        push_hud_screen_text_draw(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &HudScreenTextDraw {
                runs: &line,
                origin: (centered(&line), 88.0 + index as f32 * 18.0),
                scale: 1.0,
                tint: HUD_TINT_WHITE,
                backdrop_opacity: 0.0,
            },
        );
    }
    let rect = hud_chat_restrictions_done_rect(surface_size);
    let hovered = screen.cursor_position.is_some_and(|(x, y)| {
        hud_chat_restrictions_done_contains(surface_size, (f64::from(x), f64::from(y)))
    });
    push_hud_debug_options_button(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        rect.x as i32,
        rect.y as i32,
        rect.width as i32,
        rect.height as i32,
        &hud_coded_string_runs(&screen.done),
        true,
        hovered,
        HUD_TINT_WHITE,
        button_sprites,
    );
}

/// Vanilla `HangingSignEditScreen` background: `textures/gui/hanging_signs/<wood>.png`.
pub(super) fn hanging_sign_texture(wood: SignModelWood) -> &'static str {
    match wood {
        SignModelWood::Oak => "textures/gui/hanging_signs/oak.png",
        SignModelWood::Spruce => "textures/gui/hanging_signs/spruce.png",
        SignModelWood::Birch => "textures/gui/hanging_signs/birch.png",
        SignModelWood::Acacia => "textures/gui/hanging_signs/acacia.png",
        SignModelWood::Cherry => "textures/gui/hanging_signs/cherry.png",
        SignModelWood::Jungle => "textures/gui/hanging_signs/jungle.png",
        SignModelWood::DarkOak => "textures/gui/hanging_signs/dark_oak.png",
        SignModelWood::PaleOak => "textures/gui/hanging_signs/pale_oak.png",
        SignModelWood::Crimson => "textures/gui/hanging_signs/crimson.png",
        SignModelWood::Warped => "textures/gui/hanging_signs/warped.png",
        SignModelWood::Mangrove => "textures/gui/hanging_signs/mangrove.png",
        SignModelWood::Bamboo => "textures/gui/hanging_signs/bamboo.png",
    }
}
