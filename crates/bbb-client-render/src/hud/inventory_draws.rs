use super::*;

pub(super) fn hud_inventory_mob_effect_panel_width(
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
) -> Option<(i32, u32)> {
    let surface_width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let screen_width = i32::try_from(screen.width).unwrap_or(i32::MAX);
    let left = (surface_width - screen_width) / 2;
    let x = left.saturating_add(screen_width).saturating_add(2);
    let available_width = surface_width.saturating_sub(x);
    (available_width >= HUD_INVENTORY_EFFECT_SQUARE_SIZE as i32).then(|| {
        let max_width = if available_width >= HUD_INVENTORY_EFFECT_WIDE_THRESHOLD {
            available_width.saturating_sub(HUD_INVENTORY_EFFECT_SPACING)
        } else {
            HUD_INVENTORY_EFFECT_SQUARE_SIZE as i32
        };
        (x, u32::try_from(max_width).unwrap_or_default())
    })
}

pub(super) fn hud_inventory_mob_effect_instances(
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    effects: &[HudMobEffect],
    glyphs: &HudFontGlyphMap,
) -> Vec<HudInventoryMobEffectInstance> {
    let Some((x, max_width)) = hud_inventory_mob_effect_panel_width(surface_size, screen) else {
        return Vec::new();
    };
    let mut ordered = effects.iter().collect::<Vec<_>>();
    ordered.sort_by(|a, b| hud_mob_effect_natural_cmp(a, b));
    let y_step = if ordered.len() > 5 {
        HUD_INVENTORY_EFFECT_TOTAL_HEIGHT
            / i32::try_from(ordered.len() - 1).unwrap_or(i32::MAX).max(1)
    } else {
        HUD_INVENTORY_EFFECT_DEFAULT_Y_STEP
    };
    let surface_height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let screen_height = i32::try_from(screen.height).unwrap_or(i32::MAX);
    let top = (surface_height - screen_height) / 2;
    ordered
        .into_iter()
        .enumerate()
        .map(|(index, effect)| {
            // `EffectsInInventory` measures the effect name and the
            // duration as Components, and breaks the name at the same
            // width, so the panel resolves each text once here.
            let name_runs = hud_coded_string_runs(&effect.display_name);
            let duration_runs = hud_coded_string_runs(&effect.duration_text);
            let name_width = HUD_INVENTORY_EFFECT_TEXT_X_OFFSET
                + hud_coded_string_width(&name_runs, glyphs)
                + HUD_INVENTORY_EFFECT_SPACING;
            let duration_width = HUD_INVENTORY_EFFECT_TEXT_X_OFFSET
                + hud_coded_string_width(&duration_runs, glyphs)
                + HUD_INVENTORY_EFFECT_SPACING;
            let background_width = max_width.min(
                u32::try_from(name_width.max(duration_width))
                    .unwrap_or(u32::MAX)
                    .max(HUD_INVENTORY_EFFECT_SQUARE_SIZE),
            );
            let y = top.saturating_add(
                i32::try_from(index)
                    .unwrap_or(i32::MAX)
                    .saturating_mul(y_step),
            );
            let max_text_width = background_width
                .saturating_sub(HUD_INVENTORY_EFFECT_TEXT_X_OFFSET as u32)
                .saturating_sub(HUD_INVENTORY_EFFECT_SPACING as u32);
            let (name, duration) = if max_text_width > 0 {
                (
                    Some(hud_clip_styled_text_runs(
                        &name_runs,
                        max_text_width,
                        glyphs,
                    )),
                    Some(duration_runs),
                )
            } else {
                (None, None)
            };
            HudInventoryMobEffectInstance {
                effect_id: effect.effect_id,
                ambient: effect.ambient,
                background_x: x,
                background_y: y,
                background_width,
                icon_x: x + HUD_INVENTORY_EFFECT_SPACING,
                icon_y: y + HUD_INVENTORY_EFFECT_SPACING,
                name,
                duration,
                text_x: x + HUD_INVENTORY_EFFECT_TEXT_X_OFFSET,
                text_y: y + HUD_INVENTORY_EFFECT_SPACING,
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_inventory_mob_effects<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    background: Option<&'a HudSpriteGpu>,
    ambient_background: Option<&'a HudSpriteGpu>,
    effect_sprites: &'a GuiSprites,
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    effects: &[HudMobEffect],
) {
    for instance in hud_inventory_mob_effect_instances(surface_size, screen, effects, glyphs) {
        let background = if instance.ambient {
            ambient_background
        } else {
            background
        };
        if let Some(background) = background {
            let rect = absolute_hud_rect(
                instance.background_x as f32,
                instance.background_y as f32,
                instance.background_width,
                HUD_INVENTORY_EFFECT_SQUARE_SIZE,
            );
            push_hud_gui_sprite(vertices, commands, background, surface_size, rect);
        }
        if let Some(icon) = effect_sprites.mob_effect(instance.effect_id) {
            push_hud_draw(
                vertices,
                commands,
                icon,
                surface_size,
                absolute_hud_rect(
                    instance.icon_x as f32,
                    instance.icon_y as f32,
                    HUD_MOB_EFFECT_ICON_SIZE,
                    HUD_MOB_EFFECT_ICON_SIZE,
                ),
            );
        }
        let Some(font_atlas) = font_atlas else {
            continue;
        };
        if let Some(name) = instance.name.as_deref() {
            push_hud_coded_string_text(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                frame_index,
                surface_size,
                name,
                (instance.text_x as f32, instance.text_y as f32),
                HUD_TINT_WHITE,
                1.0,
                false,
            );
        }
        if let Some(duration) = instance.duration.as_deref() {
            push_hud_coded_string_text(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                frame_index,
                surface_size,
                duration,
                (instance.text_x as f32, (instance.text_y + 9) as f32),
                HUD_INVENTORY_EFFECT_TEXT_GRAY,
                1.0,
                false,
            );
        }
    }
}

pub(super) fn inventory_floating_item_hud_rect(
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    item: &HudInventoryItem,
) -> HudRect {
    let (width, height) = hud_inventory_item_scaled_size(item.scale, item.scale_y);
    inventory_background_hud_rect(
        surface_size,
        screen.width,
        screen.height,
        item.x,
        item.y,
        width,
        height,
    )
}

fn hud_inventory_item_scaled_size(scale: f32, scale_y: f32) -> (u32, u32) {
    (
        ((HUD_INVENTORY_ITEM_SIZE as f32) * scale)
            .round()
            .clamp(1.0, 512.0) as u32,
        ((HUD_INVENTORY_ITEM_SIZE as f32) * scale_y)
            .round()
            .clamp(1.0, 512.0) as u32,
    )
}

pub(super) fn inventory_floating_item_scissor_hud_rect(
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    item: &HudInventoryItem,
) -> Option<HudRect> {
    item.scissor.map(|scissor| {
        inventory_background_hud_rect(
            surface_size,
            screen.width,
            screen.height,
            scissor.x,
            scissor.y,
            scissor.width,
            scissor.height,
        )
    })
}

#[allow(clippy::too_many_arguments)]
fn push_hud_item_glint<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    item_atlas: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    corners: [[f32; 2]; 4],
    uv: HudUvRect,
    alpha: f32,
    foil: HudItemFoil,
) {
    let start = vertices.len() as u32;
    let mut quad_vertices =
        hud_styled_quad_vertices(surface_size, corners, uv, [1.0, 1.0, 1.0, alpha]);
    if foil == HudItemFoil::Special {
        for vertex in &mut quad_vertices {
            vertex.local_uv = hud_item_special_foil_glint_uv(vertex.local_uv);
        }
    }
    vertices.extend_from_slice(&quad_vertices);
    let end = vertices.len() as u32;
    commands.push(HudDrawCommand::ItemGlint {
        mask: item_atlas,
        start,
        end,
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_item_icon<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    item_atlas: &'a HudSpriteGpu,
    digit_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &[HudDigitGlyph; 10],
    surface_size: PhysicalSize<u32>,
    item_rect: HudRect,
    icon: &HudItemIcon,
    // When the slot also renders a 3D block model (in the GUI item pass), its 2D sprite layers are the
    // flat block-texture stand-in that the 3D icon replaces. Decorations are deferred until after the
    // GUI item pass so the 3D model cannot cover count / durability / cooldown overlays.
    draw_layers: bool,
    draw_decorations: bool,
) {
    push_hud_item_icon_clipped(
        vertices,
        commands,
        item_atlas,
        digit_atlas,
        glyphs,
        surface_size,
        item_rect,
        None,
        icon,
        draw_layers,
        draw_decorations,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_item_icon_clipped<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    item_atlas: &'a HudSpriteGpu,
    digit_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &[HudDigitGlyph; 10],
    surface_size: PhysicalSize<u32>,
    item_rect: HudRect,
    scissor_rect: Option<HudRect>,
    icon: &HudItemIcon,
    // When the slot also renders a 3D block model (in the GUI item pass), its 2D sprite layers are the
    // flat block-texture stand-in that the 3D icon replaces. Decorations are deferred until after the
    // GUI item pass so the 3D model cannot cover count / durability / cooldown overlays.
    draw_layers: bool,
    draw_decorations: bool,
) {
    let (layer_rect, layer_uv_min, layer_uv_max) = match scissor_rect {
        Some(scissor_rect) => {
            let Some((visible, uv_min, uv_max)) =
                hud_rect_intersection_uv_span(item_rect, scissor_rect)
            else {
                return;
            };
            (visible, uv_min, uv_max)
        }
        None => (item_rect, [0.0, 0.0], [1.0, 1.0]),
    };
    let scissor_active =
        scissor_rect.is_some_and(|rect| push_hud_rect_scissor(commands, surface_size, rect));
    let layer_corners = hud_item_pop_corners(layer_rect, icon.pop);
    for_each_hud_item_icon_draw_step(icon, draw_layers, draw_decorations, |step| match step {
        HudItemIconDrawStep::Layers => {
            for layer in &icon.layers {
                push_hud_draw_with_corners_uv_and_tint(
                    vertices,
                    commands,
                    item_atlas,
                    surface_size,
                    layer_corners,
                    hud_uv_rect_subspan(layer.uv, layer_uv_min, layer_uv_max),
                    layer.tint,
                );
            }
        }
        HudItemIconDrawStep::Glint => {
            for layer in &icon.layers {
                push_hud_item_glint(
                    vertices,
                    commands,
                    item_atlas,
                    surface_size,
                    layer_corners,
                    hud_uv_rect_subspan(layer.uv, layer_uv_min, layer_uv_max),
                    layer.tint[3],
                    icon.foil,
                );
            }
        }
        HudItemIconDrawStep::DurabilityBar => push_hud_item_durability_bar(
            vertices,
            commands,
            surface_size,
            item_rect,
            icon.durability_bar.as_ref(),
        ),
        HudItemIconDrawStep::Cooldown => push_hud_item_cooldown(
            vertices,
            commands,
            surface_size,
            item_rect,
            icon.cooldown_progress,
        ),
        HudItemIconDrawStep::CountLabel => push_hud_item_count_label(
            vertices,
            commands,
            digit_atlas,
            glyphs,
            surface_size,
            item_rect,
            icon.count_label.as_ref(),
        ),
    });
    if scissor_active {
        push_hud_inventory_scissor_reset(commands, surface_size);
    }
}

pub(super) fn for_each_hud_item_icon_draw_step(
    icon: &HudItemIcon,
    draw_layers: bool,
    draw_decorations: bool,
    mut emit: impl FnMut(HudItemIconDrawStep),
) {
    if draw_layers && !icon.layers.is_empty() {
        emit(HudItemIconDrawStep::Layers);
        if icon.foil.has_foil() {
            emit(HudItemIconDrawStep::Glint);
        }
    }
    if !draw_decorations {
        return;
    }
    if icon.durability_bar.is_some() {
        emit(HudItemIconDrawStep::DurabilityBar);
    }
    if icon.cooldown_progress.is_some() {
        emit(HudItemIconDrawStep::Cooldown);
    }
    if icon.count_label.is_some() {
        emit(HudItemIconDrawStep::CountLabel);
    }
}

pub(super) fn hud_item_special_foil_glint_uv(local_uv: [f32; 2]) -> [f32; 2] {
    let scale = HUD_ITEM_SPECIAL_FOIL_TEXTURE_SCALE / HUD_ITEM_SPECIAL_FOIL_GUI_SCALE;
    [local_uv[0] * scale, -local_uv[1] * scale]
}

fn push_hud_item_durability_bar<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    surface_size: PhysicalSize<u32>,
    item_rect: HudRect,
    bar: Option<&HudItemDurabilityBar>,
) {
    let Some(bar) = bar else {
        return;
    };
    let width = bar.width.min(HUD_ITEM_BAR_BACKGROUND_WIDTH);
    push_hud_fill(
        vertices,
        commands,
        surface_size,
        hud_item_durability_bar_rect(
            item_rect,
            HUD_ITEM_BAR_BACKGROUND_WIDTH,
            HUD_ITEM_BAR_BACKGROUND_HEIGHT,
        ),
        HUD_ITEM_BAR_BACKGROUND_TINT,
    );
    if width == 0 {
        return;
    }
    push_hud_fill(
        vertices,
        commands,
        surface_size,
        hud_item_durability_bar_rect(item_rect, width, HUD_ITEM_BAR_FOREGROUND_HEIGHT),
        [bar.color[0], bar.color[1], bar.color[2], 1.0],
    );
}

fn push_hud_item_cooldown<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    surface_size: PhysicalSize<u32>,
    item_rect: HudRect,
    progress: Option<f32>,
) {
    let Some(progress) = progress else {
        return;
    };
    let Some(rect) = hud_item_cooldown_rect(item_rect, progress) else {
        return;
    };
    push_hud_fill(
        vertices,
        commands,
        surface_size,
        rect,
        HUD_ITEM_COOLDOWN_TINT,
    );
}

fn push_hud_item_count_label<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    digit_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &[HudDigitGlyph; 10],
    surface_size: PhysicalSize<u32>,
    item_rect: HudRect,
    label: Option<&HudItemCountLabel>,
) {
    let (Some(digit_atlas), Some(label)) = (digit_atlas, label) else {
        return;
    };
    let Some(text_width) = hud_digit_text_width(&label.text, glyphs) else {
        return;
    };

    for shadow in [true, false] {
        let tint = if shadow {
            HUD_TEXT_SHADOW_TINT
        } else {
            HUD_TINT_WHITE
        };
        for (pen_x, glyph) in hud_item_count_glyphs(&label.text, glyphs) {
            let quad = hud_item_count_digit_quad(item_rect, text_width, pen_x, shadow, glyph);
            let start = vertices.len() as u32;
            vertices.extend_from_slice(&hud_styled_quad_vertices(
                surface_size,
                quad.corners,
                quad.uv,
                tint,
            ));
            push_hud_sprite_batch(commands, digit_atlas, start, vertices.len() as u32);
        }
    }
}

pub(super) fn hud_item_model_scissor_rect(
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
) -> Option<crate::item_models::ItemModelScissorRect> {
    let left = rect.x.floor().max(0.0).min(surface_size.width as f32) as u32;
    let top = rect.y.floor().max(0.0).min(surface_size.height as f32) as u32;
    let right = (rect.x + rect.width as f32)
        .ceil()
        .max(0.0)
        .min(surface_size.width as f32) as u32;
    let bottom = (rect.y + rect.height as f32)
        .ceil()
        .max(0.0)
        .min(surface_size.height as f32) as u32;
    crate::item_models::ItemModelScissorRect::new(
        left,
        top,
        right.saturating_sub(left),
        bottom.saturating_sub(top),
    )
}

pub(super) fn push_hud_inventory_scissor(
    commands: &mut Vec<HudDrawCommand<'_>>,
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    scissor: HudInventoryItemScissor,
) -> bool {
    push_hud_rect_scissor(
        commands,
        surface_size,
        hud_inventory_scissor_rect(surface_size, screen, scissor),
    )
}

pub(super) fn hud_inventory_scissor_rect(
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    scissor: HudInventoryItemScissor,
) -> HudRect {
    inventory_background_hud_rect(
        surface_size,
        screen.width,
        screen.height,
        scissor.x,
        scissor.y,
        scissor.width,
        scissor.height,
    )
}

pub(super) fn hud_inventory_nested_scissor_rect(
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    scissor: HudInventoryItemScissor,
    parent: Option<HudRect>,
) -> Option<HudRect> {
    let scissor = hud_inventory_scissor_rect(surface_size, screen, scissor);
    parent.map_or(Some(scissor), |parent| {
        hud_rect_intersection_uv_span(scissor, parent).map(|(intersection, _, _)| intersection)
    })
}

pub(super) fn push_hud_inventory_scissor_reset(
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

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_inventory_fill_layers<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    layers: &[HudInventoryFillLayer],
    stage: HudInventoryFillStage,
) {
    for layer in layers.iter().filter(|layer| layer.stage == stage) {
        push_hud_fill(
            vertices,
            commands,
            surface_size,
            inventory_background_hud_rect(
                surface_size,
                screen.width,
                screen.height,
                layer.x,
                layer.y,
                layer.width,
                layer.height,
            ),
            layer.tint,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_inventory_scrollbar<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    sprites: &'a GuiSprites,
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    scrollbar: HudInventoryScrollbar,
) {
    let background = inventory_background_hud_rect(
        surface_size,
        screen.width,
        screen.height,
        scrollbar.x,
        scrollbar.y,
        scrollbar.width,
        scrollbar.height,
    );
    let thumb = inventory_background_hud_rect(
        surface_size,
        screen.width,
        screen.height,
        scrollbar.x,
        scrollbar.thumb_y,
        scrollbar.width,
        scrollbar.thumb_height,
    );
    for (kind, rect, missing_asset_tint) in [
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
                missing_asset_tint,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_inventory_preedit_overlay<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    background: Option<&'a HudSpriteGpu>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    overlay: Option<&HudInventoryPreeditOverlay>,
) {
    let (Some(font_atlas), Some(overlay)) = (font_atlas, overlay) else {
        return;
    };
    let background_rect = absolute_hud_rect(
        overlay.background_x as f32,
        overlay.background_y as f32,
        overlay.background_width,
        overlay.background_height,
    );
    if let Some(background) = background {
        push_hud_gui_sprite(
            vertices,
            commands,
            background,
            surface_size,
            background_rect,
        );
    }

    let text_len = overlay.text.chars().count();
    let focused_start = overlay.focused_start.min(text_len);
    let focused_end = overlay.focused_end.clamp(focused_start, text_len);
    let mut runs = Vec::new();
    if focused_start > 0 {
        runs.push(HudStyledTextRun::plain(hud_slice_by_chars(
            &overlay.text,
            0,
            focused_start,
        )));
    }
    if focused_start < focused_end {
        runs.push(HudStyledTextRun {
            shadow_color: None,
            text: hud_slice_by_chars(&overlay.text, focused_start, focused_end),
            style: HudTextStyle {
                underlined: true,
                ..HudTextStyle::default()
            },
            color: None,
        });
    }
    if focused_end < text_len {
        runs.push(HudStyledTextRun::plain(hud_slice_by_chars(
            &overlay.text,
            focused_end,
            text_len,
        )));
    }
    if runs.is_empty() {
        runs.push(HudStyledTextRun::plain(&overlay.text));
    }
    let geometry = hud_styled_text_geometry(
        &runs,
        glyphs,
        obfuscated_pool,
        obfuscated_seed,
        (overlay.text_x as f32, overlay.text_y as f32),
        false,
        [0.0, 0.0, 0.0, 1.0],
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

    if overlay.caret_visible {
        if let Some(caret) = overlay.caret {
            let prefix = hud_slice_by_chars(&overlay.text, 0, caret.min(text_len));
            let caret_x = overlay
                .text_x
                .saturating_add(hud_raw_string_width(&prefix, glyphs));
            push_hud_fill(
                vertices,
                commands,
                surface_size,
                absolute_hud_rect(caret_x as f32, overlay.text_y as f32 - 1.0, 1, 11),
                [0.0, 0.0, 0.0, 1.0],
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_inventory_background_layers<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    renderer: &'a Renderer,
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    layers: &[HudInventoryBackgroundLayer],
) {
    for layer in layers {
        if let HudInventoryBackgroundTexture::AdvancementLine(line) = layer.texture {
            let tint = match line {
                HudAdvancementLineTexture::Background => [0.0, 0.0, 0.0, 1.0],
                HudAdvancementLineTexture::Foreground => HUD_TINT_WHITE,
            };
            push_hud_fill(
                vertices,
                commands,
                surface_size,
                inventory_background_hud_rect(
                    surface_size,
                    screen.width,
                    screen.height,
                    layer.x,
                    layer.y,
                    layer.width,
                    layer.height,
                ),
                tint,
            );
            continue;
        }
        if let Some(background) = renderer.hud_inventory_background_sprite(layer.texture) {
            let rect = inventory_background_hud_rect(
                surface_size,
                screen.width,
                screen.height,
                layer.x,
                layer.y,
                layer.width,
                layer.height,
            );
            match layer.texture.smithing_slot_icon_alpha() {
                Some(alpha) => push_hud_draw_with_uv_and_tint(
                    vertices,
                    commands,
                    background,
                    surface_size,
                    rect,
                    layer.uv,
                    [1.0, 1.0, 1.0, alpha],
                ),
                None => push_hud_draw_with_uv(
                    vertices,
                    commands,
                    background,
                    surface_size,
                    rect,
                    layer.uv,
                ),
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_inventory_background_layers_with_dynamic_previews<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    renderer: &'a Renderer,
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    layers: &[HudInventoryBackgroundLayer],
    banner_pattern_previews: &[HudInventoryBannerPatternPreview],
    map_previews: &[HudInventoryMapPreview],
    entity_previews: &[HudEntityPreview],
) {
    for (layer_index, layer) in layers.iter().enumerate() {
        push_hud_inventory_background_layers(
            vertices,
            commands,
            renderer,
            surface_size,
            screen,
            std::slice::from_ref(layer),
        );
        for preview in banner_pattern_previews
            .iter()
            .filter(|preview| preview.after_background_layer == layer_index)
        {
            push_hud_banner_pattern_preview(
                vertices,
                commands,
                renderer,
                surface_size,
                screen,
                preview,
            );
        }
        commands.extend(
            map_previews
                .iter()
                .enumerate()
                .filter(|(_, preview)| preview.after_background_layer == layer_index)
                .map(|(preview_index, _)| HudDrawCommand::MapPreview {
                    preview_index: u8::try_from(preview_index)
                        .expect("sanitized HUD map preview count fits u8"),
                }),
        );
        for (target_index, preview) in entity_previews.iter().enumerate() {
            if preview.after_background_layer == Some(layer_index) {
                push_hud_entity_preview_blit(
                    vertices,
                    commands,
                    renderer,
                    surface_size,
                    screen,
                    target_index,
                    preview,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_inventory_text_labels<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: Option<&'a HudSpriteGpu>,
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    labels: &[HudInventoryTextLabel],
    parent_scissor: Option<HudInventoryItemScissor>,
    scrolling_time_seconds: f64,
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let parent_scissor =
        parent_scissor.map(|scissor| hud_inventory_scissor_rect(surface_size, screen, scissor));
    for label in labels {
        let glyphs = fonts.glyphs(label.font.as_deref());
        let obfuscated_pool = fonts.obfuscated_pool(label.font.as_deref());
        if let Some(background) = label.background {
            push_hud_fill(
                vertices,
                commands,
                surface_size,
                inventory_background_hud_rect(
                    surface_size,
                    screen.width,
                    screen.height,
                    background.x,
                    background.y,
                    background.width,
                    background.height,
                ),
                background.tint,
            );
        }
        let mut origin = hud_inventory_text_label_origin(
            surface_size,
            screen.width,
            screen.height,
            label.x,
            label.y,
        );
        if let Some(input) = label.input {
            push_hud_inventory_text_input_label(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                obfuscated_seed,
                surface_size,
                label,
                input,
                origin,
            );
            continue;
        }
        let adapted_runs;
        let runs = if label.component_runs.is_empty() {
            adapted_runs = label
                .runs
                .iter()
                .cloned()
                .map(|visual| HudComponentTextRun {
                    visual,
                    font: label.font.clone(),
                })
                .collect::<Vec<_>>();
            &adapted_runs
        } else {
            &label.component_runs
        };
        // Prewrapped components can have signed advances; total width is not a prefix bound.
        let mut width_limit = label.component_runs.is_empty().then_some(label.width);
        let mut scrolling_scissor = false;
        if let Some(scrolling) = label.scrolling {
            if let Some(line_width) = hud_component_runs_width(runs, fonts) {
                let layout =
                    hud_scrolling_text_layout(scrolling, line_width, scrolling_time_seconds);
                origin = hud_inventory_text_label_origin(
                    surface_size,
                    screen.width,
                    screen.height,
                    layout.x,
                    layout.y,
                );
                width_limit = None;
                if layout.clipped {
                    let effective_scissor = hud_inventory_nested_scissor_rect(
                        surface_size,
                        screen,
                        HudInventoryItemScissor {
                            x: scrolling.x,
                            y: scrolling.y,
                            width: scrolling.width,
                            height: scrolling.height,
                        },
                        parent_scissor,
                    );
                    let Some(effective_scissor) = effective_scissor else {
                        continue;
                    };
                    if !push_hud_rect_scissor(commands, surface_size, effective_scissor) {
                        continue;
                    }
                    scrolling_scissor = true;
                }
            }
        }
        let geometry = hud_component_text_geometry(
            runs,
            fonts,
            obfuscated_seed,
            origin,
            label.shadow,
            label.tint,
            width_limit,
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
        if scrolling_scissor {
            if let Some(parent_scissor) = parent_scissor {
                push_hud_rect_scissor(commands, surface_size, parent_scissor);
            } else {
                push_hud_inventory_scissor_reset(commands, surface_size);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_inventory_text_areas<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    scrollbar_sprites: &'a GuiSprites,
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    areas: &[HudInventoryTextArea],
    parent_scissor: HudInventoryItemScissor,
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let parent_rect = hud_inventory_scissor_rect(surface_size, screen, parent_scissor);
    for area in areas {
        let inner_width = area.width.saturating_sub(8).max(1);
        let (units, lines) = utf16_widget_lines(&area.text, inner_width, glyphs);
        let slice = |begin, end| utf16_widget_slice(&units, begin, end);
        let (cursor, selection, needs_cursor_position) =
            (area.cursor, area.selection, area.needs_cursor_position);
        let text_len = units.len();
        let content_height = i32::try_from(lines.len())
            .unwrap_or(i32::MAX)
            .saturating_mul(HUD_WIDGET_TEXT_LINE_HEIGHT)
            .saturating_add(8);
        let height = i32::try_from(area.height).unwrap_or(i32::MAX);
        let max_scroll = content_height.saturating_sub(height).max(0);
        let scroll_amount = area.scroll_amount.clamp(0, max_scroll);
        let inner_scissor = HudInventoryItemScissor {
            x: area.x.saturating_add(1),
            y: area.y.saturating_add(1),
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };
        let Some(inner_rect) = hud_inventory_nested_scissor_rect(
            surface_size,
            screen,
            inner_scissor,
            Some(parent_rect),
        ) else {
            continue;
        };
        if !push_hud_rect_scissor(commands, surface_size, inner_rect) {
            continue;
        }

        let text_x = area.x.saturating_add(4);
        let text_y = area.y.saturating_add(4).saturating_sub(scroll_amount);
        let viewport_top = hud_inventory_text_label_origin(
            surface_size,
            screen.width,
            screen.height,
            area.x,
            area.y,
        )
        .1;
        let viewport_y = (viewport_top, viewport_top + area.height as f32);
        let line_origin = |line_index: usize| {
            hud_inventory_text_label_origin(
                surface_size,
                screen.width,
                screen.height,
                text_x,
                text_y.saturating_add(
                    i32::try_from(line_index)
                        .unwrap_or(i32::MAX)
                        .saturating_mul(HUD_WIDGET_TEXT_LINE_HEIGHT),
                ),
            )
        };
        let insert_cursor = cursor < text_len;
        let mut has_drawn_cursor = false;
        let mut append_origin = None;
        for (line_index, line) in lines.iter().copied().enumerate() {
            let origin = line_origin(line_index);
            if !multiline_line_visible(origin.1, viewport_y) {
                continue;
            }
            if (area.cursor_visible || needs_cursor_position)
                && insert_cursor
                && !has_drawn_cursor
                && cursor >= line.begin
                && cursor <= line.end
            {
                let before = slice(line.begin, cursor);
                let after = slice(cursor, line.end);
                let cursor_x = origin.0 + hud_raw_string_width(&before, glyphs) as f32;
                for (text, position) in [(&before, origin), (&after, (cursor_x, origin.1))] {
                    push_hud_raw_string_text(
                        vertices,
                        commands,
                        white_pixel,
                        font_atlas,
                        glyphs,
                        obfuscated_pool,
                        obfuscated_seed,
                        surface_size,
                        text,
                        position,
                        area.text_tint,
                        1.0,
                        area.shadow,
                    );
                }
                if area.cursor_visible {
                    push_hud_fill(
                        vertices,
                        commands,
                        surface_size,
                        absolute_hud_rect(cursor_x, origin.1 - 1.0, 1, 11),
                        area.cursor_tint,
                    );
                }
                has_drawn_cursor = true;
            } else {
                let text = slice(line.begin, line.end);
                push_hud_raw_string_text(
                    vertices,
                    commands,
                    white_pixel,
                    font_atlas,
                    glyphs,
                    obfuscated_pool,
                    obfuscated_seed,
                    surface_size,
                    &text,
                    origin,
                    area.text_tint,
                    1.0,
                    area.shadow,
                );
                if (area.cursor_visible || needs_cursor_position) && !insert_cursor {
                    append_origin = Some((
                        origin.0 + hud_raw_string_width(&text, glyphs) as f32,
                        origin.1,
                    ));
                }
            }
        }
        if let Some(origin) = append_origin.filter(|_| area.cursor_visible) {
            push_hud_raw_string_text(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                obfuscated_seed,
                surface_size,
                "_",
                origin,
                area.cursor_tint,
                1.0,
                area.shadow,
            );
        }

        let selection = cursor.min(selection)..cursor.max(selection);
        if !selection.is_empty() {
            for (line_index, line) in lines.iter().copied().enumerate() {
                if let Some(rect) = multiline_selection_rect(
                    line.begin..line.end,
                    selection.clone(),
                    line_origin(line_index),
                    i32::try_from(area.width.saturating_sub(4)).unwrap_or(i32::MAX),
                    viewport_y,
                    |begin, end| hud_raw_string_width(&slice(begin, end), glyphs),
                ) {
                    push_hud_text_highlight(vertices, commands, surface_size, rect);
                }
            }
        }

        push_hud_rect_scissor(commands, surface_size, parent_rect);

        if max_scroll > 0 {
            let thumb_upper = height.saturating_sub(8);
            let calculated = height
                .saturating_mul(height)
                .checked_div(content_height.max(1))
                .unwrap_or(0);
            let thumb_height = if thumb_upper < 32 {
                thumb_upper.max(1)
            } else {
                calculated.clamp(32, thumb_upper)
            };
            let travel = height.saturating_sub(thumb_height);
            let thumb_y = area.y.max(
                scroll_amount
                    .saturating_mul(travel)
                    .checked_div(max_scroll)
                    .unwrap_or(0)
                    .saturating_add(area.y),
            );
            push_hud_inventory_scrollbar(
                vertices,
                commands,
                white_pixel,
                scrollbar_sprites,
                surface_size,
                screen,
                HudInventoryScrollbar {
                    x: area
                        .x
                        .saturating_add(i32::try_from(area.width).unwrap_or(i32::MAX)),
                    y: area.y,
                    width: 6,
                    height: area.height,
                    thumb_y,
                    thumb_height: u32::try_from(thumb_height).unwrap_or(1),
                },
            );
        }
        let counter = utf16_widget_counter(&area.text, area.max_length);
        let counter_x = area
            .x
            .saturating_add(i32::try_from(area.width).unwrap_or(i32::MAX))
            .saturating_sub(hud_raw_string_width(&counter, glyphs));
        let counter_origin = hud_inventory_text_label_origin(
            surface_size,
            screen.width,
            screen.height,
            counter_x,
            area.y
                .saturating_add(i32::try_from(area.height).unwrap_or(i32::MAX))
                .saturating_add(4),
        );
        push_hud_raw_string_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            obfuscated_seed,
            surface_size,
            &counter,
            counter_origin,
            area.counter_tint,
            1.0,
            false,
        );
    }
}

/// A bordered `EditBox`: its sprite, then the value 4 px in and 6 px down.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_edit_box<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    (text_field, text_field_highlighted): (Option<&'a HudSpriteGpu>, Option<&'a HudSpriteGpu>),
    rect: HudCommandBlockEditorRect,
    field: &HudTextField,
) {
    let target = absolute_hud_rect(rect.x as f32, rect.y as f32, rect.width, rect.height);
    let full_uv = HudUvRect {
        min: [0.0, 0.0],
        max: [1.0, 1.0],
    };
    match field
        .focused
        .then_some(text_field_highlighted)
        .flatten()
        .or(text_field)
    {
        Some(sprite) => push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            sprite,
            surface_size,
            target,
            full_uv,
            HUD_TINT_WHITE,
        ),
        None => push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            white_pixel,
            surface_size,
            target,
            full_uv,
            if field.focused || field.hovered {
                [0.12, 0.12, 0.12, 0.96]
            } else {
                [0.0, 0.0, 0.0, 0.72]
            },
        ),
    }
    let input = field.decoration(HUD_COMMAND_INPUT_TINT);
    let label = HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: 0,
        y: 0,
        width: rect.width.saturating_sub(8),
        text: field.text.clone(),
        tint: HUD_COMMAND_INPUT_TINT,
        background: None,
        input: Some(input),
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
        obfuscated_seed,
        surface_size,
        &label,
        input,
        (
            rect.x.saturating_add(4) as f32,
            rect.y.saturating_add(6) as f32,
        ),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_inventory_text_input_label<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    label: &HudInventoryTextLabel,
    input: HudInventoryTextInputDecoration,
    origin: (f32, f32),
) {
    let layout = utf16_widget_input_layout(label, input, glyphs);
    let parts = match &layout.split_text {
        Some((before, after)) => [
            (before.as_str(), origin),
            (after.as_str(), (origin.0 + layout.cursor_x, origin.1)),
        ],
        None => [(layout.displayed_text.as_str(), origin), ("", origin)],
    };
    for (text, position) in parts {
        if text.is_empty() {
            continue;
        }
        let runs = [label
            .runs
            .first()
            .filter(|_| label.runs.len() == 1)
            .map(|run| HudStyledTextRun {
                shadow_color: run.shadow_color,
                text: text.to_string(),
                style: run.style,
                color: run.color,
            })
            .unwrap_or_else(|| HudStyledTextRun::plain(text))];
        let geometry = hud_logical_styled_text_geometry(
            &runs,
            glyphs,
            obfuscated_pool,
            obfuscated_seed,
            position,
            label.shadow,
            label.tint,
            Some(label.width),
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

    if let Some((selection_x, selection_width)) = layout.selection_rect {
        push_hud_text_highlight(
            vertices,
            commands,
            surface_size,
            absolute_hud_rect(origin.0 + selection_x, origin.1 - 1.0, selection_width, 11),
        );
    }

    if input.cursor_visible && layout.cursor_on_screen {
        if layout.insert_cursor {
            push_hud_fill(
                vertices,
                commands,
                surface_size,
                absolute_hud_rect(origin.0 + layout.cursor_x, origin.1 - 1.0, 1, 11),
                input.cursor_tint,
            );
        } else {
            push_hud_raw_string_text(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                obfuscated_seed,
                surface_size,
                "_",
                (origin.0 + layout.cursor_x, origin.1),
                input.cursor_tint,
                1.0,
                label.shadow,
            );
        }
    }
}

pub(super) fn push_hud_inventory_tooltip_flat_background<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    target: HudRect,
) {
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        white_pixel,
        surface_size,
        target,
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        HUD_TOOLTIP_BACKGROUND_TINT,
    );
}

#[cfg(test)]
#[path = "tests/gui_editor_fill.rs"]
mod gui_editor_fill;
