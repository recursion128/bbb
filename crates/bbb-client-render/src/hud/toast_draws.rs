use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_now_playing_toast<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    background: Option<&'a HudSpriteGpu>,
    music_notes: &'a GuiSprites,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    toast: &HudNowPlayingToast,
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let (rect, x) = hud_now_playing_toast_layout(toast, glyphs);
    let background_rect = clip_hud_rect_to_viewport(rect, surface_size);
    if let Some(background) = background {
        push_hud_gui_sprite(
            vertices,
            commands,
            background,
            surface_size,
            background_rect,
        );
    } else {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            white_pixel,
            surface_size,
            background_rect,
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            [0.15, 0.15, 0.15, 0.95],
        );
    }
    if let Some(notes) = music_notes.get_at_tick("icon/music_notes", toast.client_tick) {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            notes,
            surface_size,
            absolute_hud_rect(x + 7.0, 7.0, 16, 16),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            hud_argb_to_tint(toast.note_color_argb),
        );
    }
    // `NowPlayingToast`: the drawn string and the toast width are
    // both the `Component.translatable` face, so they must read one run list.
    let runs = hud_coded_string_runs(&toast.text);
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
        (x + 30.0, 11.0),
        hud_argb_to_tint(0xFFD3_D3D3),
        1.0,
        true,
    );
}

pub(super) fn hud_now_playing_toast_layout(
    toast: &HudNowPlayingToast,
    glyphs: &HudFontGlyphMap,
) -> (HudRect, f32) {
    let width = 37i32
        .saturating_add(hud_coded_string_width(
            &hud_coded_string_runs(&toast.text),
            glyphs,
        ))
        .max(0) as u32;
    let x = match toast.placement {
        HudNowPlayingToastPlacement::SlidingToast => {
            width as f32 * toast.visible_portion - width as f32
        }
        HudNowPlayingToastPlacement::PauseScreen => 0.0,
    };
    (absolute_hud_rect(x, 0.0, width, 30), x)
}

pub(super) fn hud_system_toast_layout(
    surface_size: PhysicalSize<u32>,
    toast: &HudSystemToast,
    _glyphs: &HudFontGlyphMap,
) -> (HudRect, f32, u32, u32) {
    let width = toast.width;
    let height = 20u32.saturating_add(
        u32::try_from(toast.message_lines.len().max(1))
            .unwrap_or(u32::MAX)
            .saturating_mul(12),
    );
    let x = surface_size.width as f32 - width as f32 * toast.visible_portion;
    (absolute_hud_rect(x, 0.0, width, height), x, width, height)
}

pub(super) fn hud_system_toast_instance_layout(
    surface_size: PhysicalSize<u32>,
    toast: &HudSystemToast,
    slot: u8,
    glyphs: &HudFontGlyphMap,
) -> (HudRect, f32, u32, u32, f32) {
    let (mut rect, x, width, height) = hud_system_toast_layout(surface_size, toast, glyphs);
    let y_offset = f32::from(slot) * height as f32;
    rect.y += y_offset;
    (rect, x, width, height, y_offset)
}

pub(super) fn hud_advancement_toast_rect(
    surface_size: PhysicalSize<u32>,
    toast: &HudAdvancementToast,
) -> HudRect {
    let x = surface_size.width as f32 - HUD_ADVANCEMENT_TOAST_WIDTH as f32 * toast.visible_portion;
    absolute_hud_rect(
        x,
        f32::from(toast.slot) * HUD_ADVANCEMENT_TOAST_HEIGHT as f32,
        HUD_ADVANCEMENT_TOAST_WIDTH,
        HUD_ADVANCEMENT_TOAST_HEIGHT,
    )
}

pub(super) fn hud_advancement_toast_icon_rect(
    surface_size: PhysicalSize<u32>,
    toast: &HudAdvancementToast,
) -> HudRect {
    let toast_rect = hud_advancement_toast_rect(surface_size, toast);
    absolute_hud_rect(toast_rect.x + 8.0, toast_rect.y + 8.0, 16, 16)
}

pub(super) fn hud_recipe_toast_rect(
    surface_size: PhysicalSize<u32>,
    toast: &HudRecipeToast,
) -> HudRect {
    let x = surface_size.width as f32 - HUD_ADVANCEMENT_TOAST_WIDTH as f32 * toast.visible_portion;
    absolute_hud_rect(
        x,
        f32::from(toast.slot) * HUD_ADVANCEMENT_TOAST_HEIGHT as f32,
        HUD_ADVANCEMENT_TOAST_WIDTH,
        HUD_ADVANCEMENT_TOAST_HEIGHT,
    )
}

pub(super) fn hud_recipe_toast_category_icon_rect(
    surface_size: PhysicalSize<u32>,
    toast: &HudRecipeToast,
) -> HudRect {
    let toast_rect = hud_recipe_toast_rect(surface_size, toast);
    absolute_hud_rect(toast_rect.x + 2.0, toast_rect.y + 2.0, 10, 10)
}

pub(super) fn hud_recipe_toast_unlocked_icon_rect(
    surface_size: PhysicalSize<u32>,
    toast: &HudRecipeToast,
) -> HudRect {
    let toast_rect = hud_recipe_toast_rect(surface_size, toast);
    absolute_hud_rect(toast_rect.x + 8.0, toast_rect.y + 8.0, 16, 16)
}

pub(super) fn tutorial_toast_height(line_count: usize) -> u32 {
    7u32.saturating_add(
        u32::try_from(line_count.max(2))
            .unwrap_or(u32::MAX)
            .saturating_mul(11),
    )
    .saturating_add(3)
}

pub(super) fn hud_tutorial_toast_rect(
    surface_size: PhysicalSize<u32>,
    toast: &HudTutorialToast,
) -> HudRect {
    let height = toast.lines.height();
    let x = surface_size.width as f32 - HUD_ADVANCEMENT_TOAST_WIDTH as f32 * toast.visible_portion;
    absolute_hud_rect(
        x,
        f32::from(toast.slot) * height as f32,
        HUD_ADVANCEMENT_TOAST_WIDTH,
        height,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_tutorial_toasts<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    post_gui_item_commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    background: Option<&'a HudSpriteGpu>,
    icons: &'a GuiSprites,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    toasts: &[HudTutorialToast],
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    for toast in toasts {
        let rect = hud_tutorial_toast_rect(surface_size, toast);
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

        if let Some(icon) = icons.get(toast.icon.sprite_path()) {
            push_hud_draw(
                vertices,
                commands,
                icon,
                surface_size,
                absolute_hud_rect(rect.x + 6.0, rect.y + 6.0, 20, 20),
            );
        }

        let text_height = u32::try_from(toast.lines.as_slice().len())
            .unwrap_or(u32::MAX)
            .saturating_mul(11);
        let content_height = toast.lines.height().saturating_sub(10);
        let text_top = 7u32.saturating_add(content_height.saturating_sub(text_height) / 2);
        for (index, line) in toast.lines.as_slice().iter().enumerate() {
            let geometry = hud_styled_text_geometry(
                line,
                glyphs,
                obfuscated_pool,
                frame_index,
                (
                    rect.x + 30.0,
                    rect.y + text_top as f32 + index as f32 * 11.0,
                ),
                false,
                hud_argb_to_tint(0xFF00_0000),
                Some(HUD_TUTORIAL_TOAST_TEXT_WIDTH),
                1.0,
            );
            push_hud_styled_text_geometry(
                vertices,
                post_gui_item_commands,
                white_pixel,
                font_atlas,
                surface_size,
                &geometry,
            );
        }

        if let Some(progress) = toast.progress {
            let y = rect.y + rect.height as f32 - 4.0;
            push_hud_fill(
                vertices,
                commands,
                surface_size,
                absolute_hud_rect(rect.x + 3.0, y, 154, 1),
                HUD_TINT_WHITE,
            );
            let width = (154.0 * progress).floor() as u32;
            if width > 0 {
                push_hud_fill(
                    vertices,
                    commands,
                    surface_size,
                    absolute_hud_rect(rect.x + 3.0, y, width, 1),
                    hud_argb_to_tint(if toast.progress_increasing {
                        0xFF00_5500
                    } else {
                        0xFF55_0000
                    }),
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_recipe_toasts<'a>(
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
    toasts: &[HudRecipeToast],
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    for toast in toasts {
        let rect = hud_recipe_toast_rect(surface_size, toast);
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
            if let Some(icon) = &toast.category_icon {
                push_hud_item_icon(
                    vertices,
                    commands,
                    item_atlas,
                    digit_atlas,
                    digit_glyphs,
                    surface_size,
                    hud_recipe_toast_category_icon_rect(surface_size, toast),
                    icon,
                    toast.category_block_model.is_none(),
                    false,
                );
            }
            if let Some(icon) = &toast.unlocked_icon {
                push_hud_item_icon(
                    vertices,
                    commands,
                    item_atlas,
                    digit_atlas,
                    digit_glyphs,
                    surface_size,
                    hud_recipe_toast_unlocked_icon_rect(surface_size, toast),
                    icon,
                    toast.unlocked_block_model.is_none(),
                    false,
                );
            }
        }

        push_hud_advancement_toast_text(
            vertices,
            post_gui_item_commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &hud_coded_string_runs(&toast.title),
            (rect.x + 30.0, rect.y + 7.0),
            hud_argb_to_tint(0xFF50_0050),
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
            &hud_coded_string_runs(&toast.description),
            (rect.x + 30.0, rect.y + 18.0),
            hud_argb_to_tint(0xFF00_0000),
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_advancement_toast_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    runs: &[HudStyledTextRun],
    origin: (f32, f32),
    tint: [f32; 4],
) {
    let geometry = hud_styled_text_geometry(
        runs,
        glyphs,
        obfuscated_pool,
        frame_index,
        origin,
        false,
        tint,
        Some(HUD_ADVANCEMENT_TOAST_TITLE_WIDTH),
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
