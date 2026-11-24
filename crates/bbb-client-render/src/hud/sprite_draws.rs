use super::*;
use crate::hud::gpu::FULL_UV;

pub(super) fn append_hud_block_item_model_residency(
    frame: &mut ItemModelResidencyFrame,
    model: &HudBlockItemModel,
    placement: glam::Mat4,
    target: ItemModelDrawTarget,
    scissor: Option<crate::item_models::ItemModelScissorRect>,
) {
    debug_assert_eq!(model.lighting, GuiItemLightingEntry::Items3d);
    let foil = ItemModelFoil::from_has_foil(model.foil);
    for layer in model.layers.iter() {
        let submit_order = u32::try_from(frame.instances.len() + frame.special_items.len())
            .expect("resident GUI item instance order overflow");
        match layer {
            HudResidentItemLayer::Geometry {
                atlas,
                identity,
                geometry_class,
                source,
                transform,
                tint,
            } => {
                frame.assets.push(ItemModelAssetRequest {
                    identity: identity.clone(),
                    atlas: *atlas,
                    geometry_class: *geometry_class,
                    source: source.clone(),
                });
                let base_phase = match geometry_class {
                    ItemModelGeometryClass::Solid => ItemModelDrawPhase::Solid,
                    ItemModelGeometryClass::Translucent => ItemModelDrawPhase::Translucent,
                };
                frame.instances.push(ItemModelInstanceRequest {
                    identity: identity.clone(),
                    atlas: *atlas,
                    geometry_class: *geometry_class,
                    transform: placement * *transform,
                    tint: *tint,
                    light: ITEM_MODEL_FULL_BRIGHT_LIGHT,
                    overlay: ITEM_MODEL_NO_OVERLAY,
                    foil,
                    target,
                    phase: base_phase,
                    submit_order,
                    scissor,
                });
                if foil != ItemModelFoil::None {
                    frame.instances.push(ItemModelInstanceRequest {
                        identity: identity.clone(),
                        atlas: *atlas,
                        geometry_class: *geometry_class,
                        transform: placement * *transform,
                        tint: *tint,
                        light: ITEM_MODEL_FULL_BRIGHT_LIGHT,
                        overlay: ITEM_MODEL_NO_OVERLAY,
                        foil,
                        target,
                        phase: match geometry_class {
                            ItemModelGeometryClass::Solid => ItemModelDrawPhase::Glint,
                            ItemModelGeometryClass::Translucent => {
                                ItemModelDrawPhase::GlintTranslucent
                            }
                        },
                        submit_order: submit_order.saturating_add(1),
                        scissor,
                    });
                }
            }
            HudResidentItemLayer::Special {
                model: special_model,
                transform,
            } => frame.special_items.push(SpecialItemInstanceRequest {
                model: special_model.clone(),
                transform: placement * *transform,
                light: ITEM_MODEL_FULL_BRIGHT_LIGHT,
                overlay: ITEM_MODEL_NO_OVERLAY,
                foil,
                target,
                outline_color: None,
                submit_order,
                scissor,
            }),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_mob_effect_overlay<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    background: Option<&'a HudSpriteGpu>,
    ambient_background: Option<&'a HudSpriteGpu>,
    effect_sprites: &'a GuiSprites,
    surface_size: PhysicalSize<u32>,
    effects: &[HudMobEffect],
) {
    for instance in hud_mob_effect_overlay_instances(surface_size, effects) {
        let background = if instance.ambient {
            ambient_background
        } else {
            background
        };
        if let Some(background) = background {
            push_hud_draw(
                vertices,
                commands,
                background,
                surface_size,
                instance.background,
            );
        }
        if let Some(icon) = effect_sprites.mob_effect(instance.effect_id) {
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                icon,
                surface_size,
                instance.icon,
                HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                [1.0, 1.0, 1.0, instance.icon_alpha],
            );
        }
    }
}

pub(super) fn push_hud_draw<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
) {
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        sprite,
        surface_size,
        rect,
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        HUD_TINT_WHITE,
    );
}

pub(super) fn push_hud_draw_with_uv<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
    uv: HudUvRect,
) {
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        sprite,
        surface_size,
        rect,
        uv,
        HUD_TINT_WHITE,
    );
}

pub(super) fn push_hud_crosshair_draw<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
    uv: HudUvRect,
) {
    let start = vertices.len() as u32;
    vertices.extend_from_slice(&hud_quad_vertices(
        surface_size,
        rect,
        sprite.texture_uv(uv),
        HUD_TINT_WHITE,
    ));
    commands.push(HudDrawCommand::InvertingSprite {
        sprite,
        start,
        end: vertices.len() as u32,
    });
}

pub(super) fn push_hud_fill(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'_>>,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
    tint: [f32; 4],
) {
    push_hud_fill_gradient(vertices, commands, surface_size, rect, tint, tint);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_fill_rect(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'_>>,
    surface_size: PhysicalSize<u32>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    tint: [f32; 4],
) {
    push_hud_fill(
        vertices,
        commands,
        surface_size,
        absolute_hud_rect(x as f32, y as f32, width, height),
        tint,
    );
}

// GuiGraphicsExtractor submits position/color fills without a texture.
pub(super) fn push_hud_fill_gradient(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'_>>,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
    top: [f32; 4],
    bottom: [f32; 4],
) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    let start = vertices.len() as u32;
    let mut quad = hud_quad_vertices(
        surface_size,
        rect,
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        top,
    );
    for index in [2, 4, 5] {
        quad[index].tint = bottom;
    }
    vertices.extend_from_slice(&quad);
    let end = vertices.len() as u32;
    commands.push(HudDrawCommand::Fill { start, end });
}

pub(super) fn push_hud_text_highlight(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'_>>,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    let right = rect.x + rect.width as f32;
    let bottom = rect.y + rect.height as f32;
    push_hud_text_highlight_corners(
        vertices,
        commands,
        surface_size,
        [
            [rect.x, rect.y],
            [rect.x, bottom],
            [right, bottom],
            [right, rect.y],
        ],
    );
}

pub(super) fn push_hud_fill_corners(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'_>>,
    surface_size: PhysicalSize<u32>,
    corners: [[f32; 2]; 4],
    tint: [f32; 4],
) {
    let start = vertices.len() as u32;
    vertices.extend_from_slice(&hud_styled_quad_vertices(
        surface_size,
        corners,
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        tint,
    ));
    commands.push(HudDrawCommand::Fill {
        start,
        end: vertices.len() as u32,
    });
}

pub(super) fn push_hud_text_highlight_corners(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'_>>,
    surface_size: PhysicalSize<u32>,
    corners: [[f32; 2]; 4],
) {
    let uv = HudUvRect {
        min: [0.0, 0.0],
        max: [1.0, 1.0],
    };
    let invert_start = vertices.len() as u32;
    vertices.extend_from_slice(&hud_styled_quad_vertices(
        surface_size,
        corners,
        uv,
        HUD_TINT_WHITE,
    ));
    let additive_start = vertices.len() as u32;
    vertices.extend_from_slice(&hud_styled_quad_vertices(
        surface_size,
        corners,
        uv,
        HUD_COMMAND_SELECTION_TINT,
    ));
    commands.push(HudDrawCommand::TextHighlight {
        invert_start,
        additive_start,
        end: vertices.len() as u32,
    });
}

/// A whole-sprite draw is `GuiGraphicsExtractor.blitSprite`, which honours
/// the sprite's `gui.scaling`; a sub-region draw always stretches.
pub(super) fn push_hud_draw_with_uv_and_tint<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
    uv: HudUvRect,
    tint: [f32; 4],
) {
    if uv == FULL_UV && !matches!(sprite.scaling, HudGuiSpriteScaling::Stretch) {
        push_hud_gui_sprite_tinted(vertices, commands, sprite, surface_size, rect, tint);
    } else {
        push_hud_quad(vertices, commands, sprite, surface_size, rect, uv, tint);
    }
}

fn push_hud_quad<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
    uv: HudUvRect,
    tint: [f32; 4],
) {
    let start = vertices.len() as u32;
    vertices.extend_from_slice(&hud_quad_vertices(
        surface_size,
        rect,
        sprite.texture_uv(uv),
        tint,
    ));
    let end = vertices.len() as u32;
    commands.push(HudDrawCommand::Sprite { sprite, start, end });
}

pub(super) fn push_hud_draw_with_corners_uv_and_tint<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    corners: [[f32; 2]; 4],
    uv: HudUvRect,
    tint: [f32; 4],
) {
    let start = vertices.len() as u32;
    vertices.extend_from_slice(&hud_styled_quad_vertices(
        surface_size,
        corners,
        sprite.texture_uv(uv),
        tint,
    ));
    let end = vertices.len() as u32;
    commands.push(HudDrawCommand::Sprite { sprite, start, end });
}

pub(super) fn hud_uv_rect_subspan(uv: HudUvRect, min: [f32; 2], max: [f32; 2]) -> HudUvRect {
    let width = uv.max[0] - uv.min[0];
    let height = uv.max[1] - uv.min[1];
    HudUvRect {
        min: [uv.min[0] + width * min[0], uv.min[1] + height * min[1]],
        max: [uv.min[0] + width * max[0], uv.min[1] + height * max[1]],
    }
}

pub(super) fn push_hud_rect_scissor(
    commands: &mut Vec<HudDrawCommand<'_>>,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
) -> bool {
    let Some(scissor) = hud_item_model_scissor_rect(surface_size, rect) else {
        return false;
    };
    commands.push(HudDrawCommand::Scissor {
        x: scissor.x(),
        y: scissor.y(),
        width: scissor.width(),
        height: scissor.height(),
    });
    true
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_banner_pattern_preview<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    renderer: &'a Renderer,
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    preview: &HudInventoryBannerPatternPreview,
) {
    let rect = inventory_background_hud_rect(
        surface_size,
        screen.width,
        screen.height,
        preview.x,
        preview.y,
        preview.width,
        preview.height,
    );
    push_hud_fill(
        vertices,
        commands,
        surface_size,
        rect,
        EntityDyeColor::Gray.texture_diffuse_color(),
    );

    let Some(texture) = preview.texture else {
        return;
    };
    let Some((atlas, atlas_uv)) = hud_banner_pattern_atlas_uv(renderer, texture) else {
        return;
    };
    let uv = compose_hud_uv_rect(atlas_uv, preview.uv);
    let start = vertices.len() as u32;
    vertices.extend_from_slice(&hud_quad_vertices(surface_size, rect, uv, [1.0; 4]));
    commands.push(HudDrawCommand::BannerPattern {
        atlas,
        start,
        end: vertices.len() as u32,
    });
}

fn compose_hud_uv_rect(atlas: EntityModelUvRect, crop: HudUvRect) -> HudUvRect {
    let span = [atlas.max[0] - atlas.min[0], atlas.max[1] - atlas.min[1]];
    HudUvRect {
        min: [
            atlas.min[0] + crop.min[0] * span[0],
            atlas.min[1] + crop.min[1] * span[1],
        ],
        max: [
            atlas.min[0] + crop.max[0] * span[0],
            atlas.min[1] + crop.max[1] * span[1],
        ],
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_entity_preview_blit<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    renderer: &'a Renderer,
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    target_index: usize,
    preview: &HudEntityPreview,
) {
    if renderer
        .hud_entity_preview_pip_targets
        .get(target_index)
        .is_none()
    {
        return;
    }
    let Some(visible) = preview.visible_bounds() else {
        return;
    };
    let uv = hud_entity_preview_blit_uv(preview.rect, visible);
    let start = vertices.len() as u32;
    vertices.extend_from_slice(&hud_quad_vertices(
        surface_size,
        inventory_background_hud_rect(
            surface_size,
            screen.width,
            screen.height,
            visible.x,
            visible.y,
            visible.width,
            visible.height,
        ),
        uv,
        [1.0, 1.0, 1.0, 1.0],
    ));
    commands.push(HudDrawCommand::EntityPreviewBlit {
        target_index,
        start,
        end: vertices.len() as u32,
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_sound_subtitles<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    subtitles: &HudSoundSubtitles,
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    // `SubtitleOverlay` measures and draws the subtitle text as a
    // Component; the arrow rows are the same `String` face.
    let layout = sound_subtitles::hud_sound_subtitle_layout(
        subtitles,
        surface_size.width,
        surface_size.height,
        |text| hud_coded_string_width(&hud_coded_string_runs(text), glyphs),
    );
    for (row, row_layout) in subtitles.rows.iter().zip(layout.rows) {
        push_hud_fill_rect(
            vertices,
            commands,
            surface_size,
            row_layout.background.x,
            row_layout.background.y,
            row_layout.background.width,
            row_layout.background.height,
            HUD_SOUND_SUBTITLE_BACKGROUND_TINT,
        );
        let brightness = f32::from(row.brightness) / 255.0;
        let tint = [brightness, brightness, brightness, 1.0];
        if let Some(arrow) = row_layout.arrow {
            push_hud_coded_string_text(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                frame_index,
                surface_size,
                &hud_coded_string_runs(arrow.text),
                (arrow.origin.0 as f32, arrow.origin.1 as f32),
                tint,
                1.0,
                true,
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
            &hud_coded_string_runs(&row.text),
            (
                row_layout.text_origin.0 as f32,
                row_layout.text_origin.1 as f32,
            ),
            tint,
            1.0,
            true,
        );
    }
}

/// Extends the current GUI mesh when consecutive vertices use the same
/// texture. Vanilla 26.1's `GuiRenderer.addElementToMesh` keeps one
/// `BufferBuilder` until scissor, pipeline, or `TextureSetup` changes; glyphs
/// from the same font atlas therefore form one draw rather than one draw per
/// quad. A non-sprite command or texture transition remains an ordering
/// boundary.
pub(super) fn push_hud_sprite_batch<'a>(
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    start: u32,
    end: u32,
) {
    if let Some(HudDrawCommand::Sprite {
        sprite: previous_sprite,
        end: previous_end,
        ..
    }) = commands.last_mut()
    {
        if previous_sprite.same_texture(sprite) && *previous_end == start {
            *previous_end = end;
            return;
        }
    }
    commands.push(HudDrawCommand::Sprite { sprite, start, end });
}

pub(super) fn push_hud_gui_sprite<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    target: HudRect,
) {
    push_hud_gui_sprite_tinted(
        vertices,
        commands,
        sprite,
        surface_size,
        target,
        HUD_TINT_WHITE,
    );
}

pub(super) fn push_hud_gui_sprite_tinted<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    target: HudRect,
    tint: [f32; 4],
) {
    let target_size = (target.width, target.height);
    let segments = match sprite.scaling {
        HudGuiSpriteScaling::Tile { width, height } if target_size != (width, height) => {
            tiled_sprite_segments(target, width, height)
        }
        // GuiGraphicsExtractor blits the whole sprite at its nine-slice size.
        HudGuiSpriteScaling::NineSlice(scaling)
            if target_size != (scaling.sprite_width, scaling.sprite_height) =>
        {
            nine_slice_segments(target, scaling)
        }
        _ => {
            push_hud_quad(
                vertices,
                commands,
                sprite,
                surface_size,
                target,
                FULL_UV,
                tint,
            );
            return;
        }
    };
    for segment in segments {
        push_hud_quad(
            vertices,
            commands,
            sprite,
            surface_size,
            segment.rect,
            segment.uv,
            tint,
        );
    }
}

/// A 3D block-item icon is only worth carrying when it has geometry to draw.
pub(super) fn hud_block_item_model_is_renderable(model: &HudBlockItemModel) -> bool {
    model.lighting == GuiItemLightingEntry::Items3d && !model.layers.is_empty()
}

pub(super) fn is_flat_gui_preview(preview: &HudEntityPreview) -> bool {
    preview.lighting == GuiItemLightingEntry::ItemsFlat
        && matches!(
            preview.entity.kind,
            crate::entity_models::EntityModelKind::Sign { .. }
                | crate::entity_models::EntityModelKind::Banner {
                    flag_only: true,
                    ..
                }
        )
}
